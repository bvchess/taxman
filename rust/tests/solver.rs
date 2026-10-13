//! End-to-end tests of the per-game pipeline: whole ranges against `optimal.json` and the Java's own counts,
//! tactic invariance, checkpoint and resume through the solver, and boards beyond `optimal.json`.

mod common;

use common::TempDir;
use std::fs;
use std::time::Duration;
use taxman::bits::Bits;
use taxman::game::Optimal;
use taxman::hives::Apiary;
use taxman::json::Json;
use taxman::numtheory::Tables;
use taxman::results::{verify_files, Proof, Taint};
use taxman::search::Revise;
use taxman::solver::{self, GameResult, Method, OptimalSource, SolveOptions};

fn options(threads: usize) -> SolveOptions {
    SolveOptions { threads, ..SolveOptions::default() }
}

/// Whether the Java derived game `n` without a search (`solve.accelerated` in the golden data).
fn java_accelerated(n: u32) -> bool {
    let json = common::golden(n);
    json.get("solve").and_then(|s| s.get("accelerated")).and_then(Json::as_bool).expect("solve.accelerated")
}

fn solve_all(lo: u32, hi: u32, opts: &SolveOptions) -> Vec<GameResult> {
    let results = solver::solve_range(lo, hi, opts, None).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(results.len(), (hi - lo + 1) as usize);
    results
}

#[test]
fn a_range_reproduces_optimal_json_and_the_javas_counts() {
    let optimal = Optimal::load_default().unwrap();
    let results = solve_all(1, 300, &options(2));
    for r in &results {
        assert!(r.completed, "n={}", r.n);
        assert_eq!(Some(r.score), optimal.score(r.n), "n={}", r.n);
        assert_eq!(r.optimal_match, Some(true), "n={}", r.n);
        assert_eq!(r.prev_source.is_some(), r.n > 1, "n={}", r.n);
    }
    // The same games are derived without search as in the Java (146 of the first 200).
    let derived = |r: &&GameResult| r.method.is_derived();
    assert_eq!(results.iter().take(200).filter(derived).count(), 146);
    for r in results.iter().take(200) {
        assert_eq!(r.method.is_derived(), java_accelerated(r.n), "n={} ({})", r.n, r.method.as_str());
    }
    // A promotion is exactly a candidate that the game plays, however the game was found.
    for r in &results {
        let tables = Tables::new(r.n);
        let candidates = Apiary::<8>::new(&tables, Bits::EMPTY).promotion_candidates();
        let played: Bits<8> = r.moves.iter().copied().collect();
        assert_eq!(r.promotions, (candidates & played).to_vec(), "n={} ({})", r.n, r.method.as_str());
    }
}

#[test]
fn every_tactic_gives_the_same_scores() {
    let optimal = Optimal::load_default().unwrap();
    // Each variant is a `--tactic` assignment; the soundness of the ones that are not on/off settings is argued in
    // DESIGN.md's "Conflicts of the matching bound".  `branching=largest` is the Java's order.
    let with = |assignment: &str| {
        let mut opts = SolveOptions { force_search: true, ..options(1) };
        opts.tactics.set(assignment).unwrap();
        (assignment.to_owned(), opts)
    };
    let default = ("default".to_owned(), SolveOptions { force_search: true, ..options(1) });
    let variants = [default].into_iter().chain(
        [
            "match_bound=off",
            "preclusions=on",
            "revise_freedoms=off",
            "seed_incumbent=off",
            "prev_cap=off",
            "branching=largest",
            "branching=conflict-smallest",
            "branching=conflict-largest",
            "match_refine=0",
            "match_refine=3",
            "fix_forced=off",
            "nogoods=off",
            "forget_nogoods=off",
        ]
        .map(with),
    );
    for (name, opts) in variants {
        for r in solve_all(1, 300, &opts) {
            assert!(r.completed, "{name}: n={}", r.n);
            assert_eq!(Some(r.score), optimal.score(r.n), "{name}: n={}", r.n);
            assert_ne!(r.method, Method::Prime, "{name}: force_search skips the shortcuts (n={})", r.n);
            assert_ne!(r.method, Method::Reuse, "{name}: force_search skips the shortcuts (n={})", r.n);
            assert_eq!(r.tactics.description(), name, "n={}", r.n);
        }
    }
}

#[test]
fn derivations_can_be_switched_off() {
    let results = solve_all(
        1,
        120,
        &SolveOptions {
            tactics: {
                let mut t = taxman::tactics::Tactics::default();
                t.set("prime_shortcut=off").unwrap();
                t.set("reuse_prev=off").unwrap();
                t
            },
            ..options(2)
        },
    );
    assert!(results.iter().all(|r| matches!(r.method, Method::Trivial | Method::Search)));
    assert!(results.iter().all(|r| r.optimal_match == Some(true)));
}

#[test]
fn an_interrupted_game_resumes_from_its_checkpoint() {
    let optimal = Optimal::load_default().unwrap();
    let dir = TempDir::new("resume");
    // Without the n − 1 solution to start from, the search has real work to do.
    let opts = SolveOptions {
        checkpoint_dir: Some(dir.0.clone()),
        checkpoint_interval: Duration::from_secs(3600),
        node_limit: Some(1_000),
        queue_mem_cap: 4,
        force_search: true,
        ..options(2)
    };
    let checkpoint = dir.0.join("n0600.ckpt");
    let mut pieces = 0;
    let result = loop {
        let r = solver::solve_game_with(600, None, &opts, Some(&optimal)).unwrap();
        pieces += 1;
        if r.completed {
            break r;
        }
        assert!(checkpoint.exists(), "an unfinished game keeps its checkpoint");
        assert_eq!((r.optimal_match, r.method), (None, Method::Search));
        assert!(r.score > 0 && r.score <= optimal.score(600).unwrap(), "the best game so far, not more than the best");
        assert!(pieces < 100, "no progress");
    };
    assert!(pieces >= 3, "only {pieces} pieces");
    assert_eq!((result.score, result.optimal_match), (optimal.score(600).unwrap(), Some(true)));
    assert!(!checkpoint.exists(), "a finished game removes its checkpoint");
    let left: Vec<_> = fs::read_dir(&dir.0).unwrap().map(|e| e.unwrap().path()).collect();
    assert!(left.is_empty(), "left behind: {left:?}");
}

/// Solves 600 (a game with a search of a few thousand nodes) and 601 (the prime after it, which takes 600 as its
/// predecessor) from `base`, with no `optimal.json` to start from: a node limit stops the search of 600, and a
/// second run, without the limit, resumes it and goes on.  Returns the games of the second run and the lines of the
/// results file, which the verifier accepts.
fn run_stopped_and_resumed(name: &str, base: SolveOptions) -> (Vec<GameResult>, Vec<Json>) {
    let dir = TempDir::new(name);
    let results = dir.join("results.jsonl");
    let limited = SolveOptions {
        checkpoint_dir: Some(dir.join("ckpt")),
        node_limit: Some(300),
        optimal: OptimalSource::None,
        ..base
    };
    assert!(!limited.allow_tainted_prev);
    let summary = solver::solve_range_each(600, 601, &limited, Some(&results), |_| {}).unwrap();
    assert_eq!(summary.stopped_at, Some(600), "the node limit stops the search of 600");
    assert!(limited.checkpoint_file(600).unwrap().exists());

    let unlimited = SolveOptions { node_limit: None, ..limited };
    let games = solver::solve_range(600, 601, &unlimited, Some(&results)).unwrap_or_else(|e| panic!("{e}"));
    assert!(games.iter().all(|g| g.completed));
    // A later invocation finds both games in the file, which agrees with what they say of themselves.
    let again = solver::solve_range_each(600, 601, &unlimited, Some(&results), |_| {}).unwrap();
    assert_eq!((again.solved, again.skipped), (0, 2));
    verify_files(std::slice::from_ref(&results), None).unwrap_or_else(|e| panic!("{e}"));

    let text = fs::read_to_string(&results).unwrap();
    (games, text.lines().map(|line| Json::parse(line).unwrap()).collect())
}

/// What a results line says of the game's taint and of how frame 0's freedoms were set.
fn taint_and_revise(line: &Json) -> (Vec<&str>, Option<&str>) {
    let taint = line.get("taint").and_then(Json::as_arr).expect("taint").iter().filter_map(Json::as_str).collect();
    (taint, line.get("setup").and_then(|s| s.get("revise")).and_then(Json::as_str))
}

/// A strict run (`revise_freedoms` and `reuse_prev` off) rests on nothing unproven, whether its search ran in one
/// piece or was stopped and resumed; so the next game builds on it with no `--allow-tainted-prev`.
#[test]
fn a_strict_game_resumed_from_its_checkpoint_rests_on_nothing_unproven() {
    let (games, lines) = run_stopped_and_resumed("strict-resume", common::strict_options(1));
    let [resumed, next] = &games[..] else { panic!("{} games", games.len()) };
    assert_eq!((resumed.n, resumed.invocations), (600, 2), "600 was resumed, not started over");
    assert_eq!(resumed.setup.map(|s| s.revise), Some(Revise::Off));
    assert!(resumed.taint.is_empty(), "{:?}", resumed.taint);
    // 601 takes 600 as its predecessor, which a run with no unproven tactic on refuses if it rests on one.
    assert!(next.completed && next.proof == Some(Proof::PrimeFormula) && next.taint.is_empty(), "{next:?}");

    let [stopped, finished, prime] = &lines[..] else { panic!("{} lines", lines.len()) };
    assert_eq!(stopped.get("completed").and_then(Json::as_bool), Some(false));
    assert_eq!(finished.get("invocations").and_then(Json::as_u32), Some(2));
    assert_eq!(taint_and_revise(finished), (vec![], Some("Off")));
    assert_eq!(taint_and_revise(prime).0, Vec::<&str>::new());
}

/// With `revise_freedoms` on, the freedoms a checkpoint holds may be revised ones, and the checkpoint does not say
/// whether they are, so the resumed game rests on the tactic, and so does the prime game built on it.
#[test]
fn a_game_resumed_from_its_checkpoint_with_revise_freedoms_on_rests_on_it() {
    let (games, lines) = run_stopped_and_resumed("default-resume", options(1));
    let [resumed, next] = &games[..] else { panic!("{} games", games.len()) };
    assert_eq!((resumed.n, resumed.invocations), (600, 2), "600 was resumed, not started over");
    assert_eq!(resumed.setup.map(|s| s.revise), Some(Revise::FromCheckpoint));
    assert_eq!(resumed.taint, Taint::from(["revise_freedoms".to_owned()]));
    assert_eq!(next.taint, resumed.taint, "601 rests on 600");

    let [_, finished, prime] = &lines[..] else { panic!("{} lines", lines.len()) };
    assert_eq!(taint_and_revise(finished), (vec!["revise_freedoms"], Some("FromCheckpoint")));
    assert_eq!(taint_and_revise(prime).0, ["revise_freedoms"]);
}

#[test]
fn a_checkpoint_for_another_game_or_other_tactics_is_refused() {
    let optimal = Optimal::load_default().unwrap();
    let dir = TempDir::new("refuse");
    let opts =
        SolveOptions { checkpoint_dir: Some(dir.0.clone()), node_limit: Some(2_000), force_search: true, ..options(1) };
    assert!(!solver::solve_game_with(600, None, &opts, Some(&optimal)).unwrap().completed);
    let checkpoint = dir.0.join("n0600.ckpt");

    // Other tactics: each of the nine search tactics (a checkpoint records them all), the switches turned the other
    // way and the two settings that are not switches changed.
    let assignments = [
        "preclusions=on",
        "legacy_bound=on",
        "revise_freedoms=off",
        "match_bound=off",
        "fix_forced=off",
        "nogoods=off",
        "forget_nogoods=off",
        "branching=largest",
        "match_refine=0",
        "match_refine=2",
    ];
    for assignment in assignments {
        let mut other = opts.clone();
        other.tactics.set(assignment).unwrap();
        let err = solver::solve_game_with(600, None, &other, Some(&optimal)).unwrap_err();
        assert!(err.contains("different search tactics") && err.contains("n0600.ckpt"), "{assignment}: {err}");
    }

    // Another game, of the same width.
    fs::copy(&checkpoint, dir.0.join("n0620.ckpt")).unwrap();
    let err = solver::solve_game_with(620, None, &opts, Some(&optimal)).unwrap_err();
    assert!(err.contains("cannot resume") && err.contains("n0620.ckpt"), "{err}");

    // Another width.
    fs::copy(&checkpoint, dir.0.join("n0400.ckpt")).unwrap();
    let err = solver::solve_game_with(400, None, &opts, Some(&optimal)).unwrap_err();
    assert!(err.contains("cannot resume") && err.contains("n0400.ckpt"), "{err}");

    // The refused files are untouched, and the game itself still resumes and finishes.
    assert!(checkpoint.exists() && dir.0.join("n0620.ckpt").exists());
    let resumed = SolveOptions { node_limit: None, ..opts };
    let r = solver::solve_game_with(600, None, &resumed, Some(&optimal)).unwrap();
    assert!(r.completed && r.optimal_match == Some(true));
}

#[test]
fn a_range_stops_at_an_unfinished_game_and_picks_up_where_it_left_off() {
    let dir = TempDir::new("range");
    let results = dir.0.join("results.jsonl");
    let ckpt = dir.0.join("ckpt");
    let limited =
        SolveOptions { checkpoint_dir: Some(ckpt.clone()), node_limit: Some(200), force_search: true, ..options(1) };
    let mut seen = Vec::new();
    let summary =
        solver::solve_range_each(598, 602, &limited, Some(&results), |r| seen.push((r.n, r.completed))).unwrap();
    assert_eq!(seen.last().map(|g| g.1), Some(false), "{seen:?}");
    assert_eq!(summary.stopped_at, seen.last().map(|g| g.0));
    assert!(ckpt.join(format!("n{:04}.ckpt", summary.stopped_at.unwrap())).exists());

    // Without the limit the same command finishes the game from its checkpoint and goes on.
    let unlimited = SolveOptions { node_limit: None, ..limited };
    let mut more = Vec::new();
    let summary = solver::solve_range_each(598, 602, &unlimited, Some(&results), |r| more.push(r.n)).unwrap();
    assert_eq!(summary.stopped_at, None);
    assert_eq!(more, (seen.last().unwrap().0..=602).collect::<Vec<_>>());
    assert_eq!(summary.skipped, seen.len() - 1);
    let left: Vec<_> = fs::read_dir(&ckpt).unwrap().map(|e| e.unwrap().path()).collect();
    assert!(left.is_empty(), "left behind: {left:?}");

    // Every board size is in the file as a completed game exactly once, and the unfinished one also as such.
    let lines: Vec<Json> = fs::read_to_string(&results).unwrap().lines().map(|l| Json::parse(l).unwrap()).collect();
    let completed = |n: u32| {
        let is = |l: &&Json| {
            l.get("n").and_then(Json::as_u32) == Some(n) && l.get("completed").and_then(Json::as_bool) == Some(true)
        };
        lines.iter().filter(is).count()
    };
    assert!((598..=602).all(|n| completed(n) == 1));
    assert_eq!(lines.len(), 5 + 1);
}

/// 1500 gets `W = 24` and 2050 `W = 64` (the first board with a number above 2047 in its frames); `tests/wide.rs`
/// goes further into the latter.  The setup's cost is mostly the local search for a better first incumbent.
#[test]
fn big_boards_get_through_setup() {
    for n in [1500, 2050] {
        // Setup is the cost here, mostly the starting incumbent's local search, which runs on one thread.
        let opts = SolveOptions { node_limit: Some(1), force_search: true, ..options(2) };
        let r = solver::solve_game(n, None, &opts).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(r.n, n);
        assert!(!r.completed, "n={n}: one node cannot finish the search");
        assert_eq!((r.method, r.optimal_match), (Method::Search, None), "n={n}");
        // The engine checks its node limit only every so often.
        assert!(r.nodes < 10_000, "n={n}: {} nodes", r.nodes);
        // The best game so far is the greedy one: legal (the solver replays it) and not empty.
        let setup = r.setup.expect("a searched game has a setup");
        assert!(setup.frames > 0 && setup.candidates > 0 && setup.greedy_sum > 0, "n={n}: {setup:?}");
        assert!(r.score > 0 && !r.moves.is_empty(), "n={n}");
        let json = r.to_json();
        assert_eq!(json.get("completed").and_then(Json::as_bool), Some(false));
        assert_eq!(json.get("optimal_match"), Some(&Json::Null));
    }
}
