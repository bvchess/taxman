//! End-to-end tests of the per-game pipeline: whole ranges against `optimal.json`, tactic invariance, checkpoint and
//! resume through the solver, and boards beyond `optimal.json`.

mod common;

use common::{long, matching_options, options, TempDir};
use std::fs;
use std::time::Duration;
use taxman::bits::Bits;
use taxman::frames::BoardFrames;
use taxman::game::{largest_prime_at_most, replay_finished, Optimal};
use taxman::json::Json;
use taxman::numtheory::Tables;
use taxman::results::{verify_files, Proof};
use taxman::solver::{self, GameResult, Method, SolveOptions};

fn solve_all(lo: u32, hi: u32, opts: &SolveOptions) -> Vec<GameResult> {
    let results = solver::solve_range(lo, hi, opts, None).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(results.len(), (hi - lo + 1) as usize);
    results
}

#[test]
fn a_range_reproduces_optimal_json() {
    let optimal = Optimal::built_in();
    let results = solve_all(1, 300, &options(2));
    for r in &results {
        assert!(r.completed, "n={}", r.n);
        assert_eq!(Some(r.score), optimal.score(r.n), "n={}", r.n);
        assert_eq!(r.optimal_match, Some(true), "n={}", r.n);
        assert_eq!(r.prev_source().is_some(), r.n > 1, "n={}", r.n);
    }
    // A promotion is exactly a candidate that the game plays, however the game was found.
    for r in &results {
        let tables = Tables::new(r.n);
        let candidates = BoardFrames::<8>::new(&tables, Bits::EMPTY).promotion_candidates();
        let played: Bits<8> = r.moves.iter().copied().collect();
        assert_eq!(r.promotions, (candidates & played).to_vec(), "n={} ({})", r.n, r.method.as_str());
    }
}

/// Settings of the tactics that tune the GF(2) search other than the defaults, each a list of `--tactic` assignments
/// in the order of the tactics.
const TUNINGS: [&str; 14] = [
    "force_window=0",
    "force_window=20",
    "force_window=80",
    "force_window=50%",
    "branching=largest",
    "branching=split",
    "branching=forced",
    "strong_limit=1",
    "strong_limit=2",
    "strong_depth=0",
    "strong_depth=6",
    "strong_depth=6,deep_branching=largest",
    "strong_depth=6,deep_branching=exact",
    "more_forced=off",
];

/// The tactics that tune the GF(2) search find the same scores at the games `games`, on one thread, every game
/// searched.
fn every_tuning_gives_the_same_scores_at(games: std::ops::RangeInclusive<u32>) {
    let optimal = Optimal::built_in();
    for assignments in std::iter::once("default").chain(TUNINGS) {
        let mut opts = SolveOptions { force_search: true, ..options(1) };
        if assignments != "default" {
            for assignment in assignments.split(',') {
                opts.tactics.set(assignment).unwrap();
            }
        }
        for r in solve_all(*games.start(), *games.end(), &opts) {
            assert!(r.completed, "{assignments}: n={}", r.n);
            assert_eq!(Some(r.score), optimal.score(r.n), "{assignments}: n={}", r.n);
            assert_eq!(r.tactics.description(), assignments, "n={}", r.n);
        }
    }
}

/// [`every_tuning_of_the_gf2_search_gives_the_same_scores`] at the games 600..=602.
#[test]
fn every_tuning_of_the_gf2_search_gives_the_same_scores_at_a_few_games() {
    every_tuning_gives_the_same_scores_at(600..=602);
}

/// The tactics that tune the GF(2) search find the same scores at games that need longer searches than those to 300
/// (931..=940, around the hard 936, on one thread, every game searched).
#[test]
#[cfg_attr(debug_assertions, ignore = long!(every_tuning_of_the_gf2_search_gives_the_same_scores_at_a_few_games))]
fn every_tuning_of_the_gf2_search_gives_the_same_scores() {
    every_tuning_gives_the_same_scores_at(931..=940);
}

/// Every tactic, alone and in the combinations that matter, finds the scores of `optimal.json` at the games `games`.
fn every_tactic_gives_the_same_scores_at(games: std::ops::RangeInclusive<u32>) {
    let optimal = Optimal::built_in();
    // Each variant is a list of `--tactic` assignments, in the order of the tactics (that of its description).
    let with = |assignments: &str| {
        let mut opts = SolveOptions { force_search: true, ..options(1) };
        for assignment in assignments.split(',') {
            opts.tactics.set(assignment).unwrap();
        }
        (assignments.to_owned(), opts)
    };
    let default = ("default".to_owned(), SolveOptions { force_search: true, ..options(1) });
    let variants = [default].into_iter().chain(
        [
            "seed_incumbent=off",
            "prev_cap=off",
            "fix_forced=off",
            "nogoods=off",
            "forget_nogoods=off",
            "lagrange=on",
            "gf2=off",
            "symmetry=off",
            "lagrange=on,gf2=off",
            "lagrange=on,gf2=off,symmetry=off",
            "match_refine=0,lagrange=on,gf2=off",
            "match_refine=3,gf2=off",
            "fix_forced=off,lagrange=on,gf2=off",
        ]
        .into_iter()
        .chain(TUNINGS)
        .map(with),
    );
    for (name, opts) in variants {
        for r in solve_all(*games.start(), *games.end(), &opts) {
            assert!(r.completed, "{name}: n={}", r.n);
            assert_eq!(Some(r.score), optimal.score(r.n), "{name}: n={}", r.n);
            assert_ne!(r.method, Method::Prime, "{name}: force_search skips the shortcuts (n={})", r.n);
            assert_ne!(r.method, Method::Repeat, "{name}: force_search skips the shortcuts (n={})", r.n);
            assert_eq!(r.tactics.description(), name, "n={}", r.n);
        }
    }
}

/// [`every_tactic_gives_the_same_scores`] at the games 100..=120.
#[test]
fn every_tactic_gives_the_same_scores_at_a_few_games() {
    every_tactic_gives_the_same_scores_at(100..=120);
}

/// Every tactic, alone and in the combinations that matter, finds the scores of `optimal.json` at the games 1..=300.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(every_tactic_gives_the_same_scores_at_a_few_games))]
fn every_tactic_gives_the_same_scores() {
    every_tactic_gives_the_same_scores_at(1..=300);
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
                t.set("same_search=off").unwrap();
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
    let optimal = Optimal::built_in();
    let dir = TempDir::new("resume");
    // Without the n − 1 solution to start from, the search has real work to do.
    let opts = SolveOptions {
        checkpoint_dir: Some(dir.0.clone()),
        checkpoint_interval: Duration::from_secs(3600),
        node_limit: Some(1_000),
        queue_mem_cap: 4,
        force_search: true,
        ..matching_options(2)
    };
    let checkpoint = dir.0.join("n0600.ckpt");
    let mut pieces = 0;
    let result = loop {
        let r = solver::solve_game_with(600, None, &opts, optimal).unwrap();
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

/// Solves `n` (600 or 760, games with a search of a few thousand nodes on the matching path) and `n + 1` (the prime
/// after it, which takes `n` as its predecessor) from `base`, with no previous game to start from: a node limit stops
/// the search of `n`, and a second run, without the limit, resumes it and goes on.  Returns the games of the second run
/// and the lines of the results file, which the verifier accepts.
fn run_stopped_and_resumed(name: &str, base: SolveOptions, n: u32) -> (Vec<GameResult>, Vec<Json>) {
    let dir = TempDir::new(name);
    let results = dir.join("results.jsonl");
    let limited = SolveOptions { checkpoint_dir: Some(dir.join("ckpt")), node_limit: Some(300), ..base };
    let summary = solver::solve_range_each(n, n + 1, &limited, Some(&results), |_| {}).unwrap();
    assert_eq!(summary.stopped_at, Some(n), "the node limit stops the search of {n}");
    assert!(limited.checkpoint_file(n).unwrap().exists());

    let unlimited = SolveOptions { node_limit: None, ..limited };
    let games = solver::solve_range(n, n + 1, &unlimited, Some(&results)).unwrap_or_else(|e| panic!("{e}"));
    assert!(games.iter().all(|g| g.completed));
    // A later invocation finds both games in the file, which agrees with what they say of themselves.
    let again = solver::solve_range_each(n, n + 1, &unlimited, Some(&results), |_| {}).unwrap();
    assert_eq!((again.solved, again.skipped), (0, 2));
    verify_files(std::slice::from_ref(&results), Optimal::built_in(), 1).unwrap_or_else(|e| panic!("{e}"));

    let text = fs::read_to_string(&results).unwrap();
    (games, text.lines().map(|line| Json::parse(line).unwrap()).collect())
}

/// What a results line says the game assumes.
fn assumptions(line: &Json) -> Vec<&str> {
    line.get("assumes").and_then(Json::as_arr).expect("assumes").iter().filter_map(Json::as_str).collect()
}

/// A game assumes nothing whether its search ran in one piece or was stopped and resumed, and so does the
/// next game, built on it.
#[test]
fn a_game_resumed_from_its_checkpoint_assumes_nothing() {
    let (games, lines) = run_stopped_and_resumed("resume", matching_options(1), 600);
    let [resumed, next] = &games[..] else { panic!("{} games", games.len()) };
    assert_eq!((resumed.n, resumed.invocations), (600, 2), "600 was resumed, not started over");
    assert!(resumed.assumes.is_empty(), "{:?}", resumed.assumes);
    // 601 takes 600 as its predecessor, and its proof rests on it.
    assert!(next.completed && next.proof == Some(Proof::PrimeFormula) && next.assumes.is_empty(), "{next:?}");

    let [stopped, finished, prime] = &lines[..] else { panic!("{} lines", lines.len()) };
    assert_eq!(stopped.get("completed").and_then(Json::as_bool), Some(false));
    assert_eq!(finished.get("invocations").and_then(Json::as_u32), Some(2));
    assert!(assumptions(finished).is_empty() && assumptions(prime).is_empty());
}

#[test]
fn a_checkpoint_for_another_game_or_other_tactics_is_refused() {
    let optimal = Optimal::built_in();
    let dir = TempDir::new("refuse");
    let opts = SolveOptions {
        checkpoint_dir: Some(dir.0.clone()),
        node_limit: Some(2_000),
        force_search: true,
        ..matching_options(1)
    };
    assert!(!solver::solve_game_with(600, None, &opts, optimal).unwrap().completed);
    let checkpoint = dir.0.join("n0600.ckpt");

    // Other tactics: each of the seven search tactics (a checkpoint records them all), the switches turned the other
    // way and `match_refine` changed.
    let assignments = [
        "fix_forced=off",
        "nogoods=off",
        "forget_nogoods=off",
        "lagrange=off",
        "gf2=on",
        "symmetry=on",
        "match_refine=0",
        "match_refine=2",
    ];
    for assignment in assignments {
        let mut other = opts.clone();
        other.tactics.set(assignment).unwrap();
        let err = solver::solve_game_with(600, None, &other, optimal).unwrap_err();
        assert!(err.contains("different search tactics") && err.contains("n0600.ckpt"), "{assignment}: {err}");
    }

    // Another game, of the same width.
    fs::copy(&checkpoint, dir.0.join("n0620.ckpt")).unwrap();
    let err = solver::solve_game_with(620, None, &opts, optimal).unwrap_err();
    assert!(err.contains("cannot resume") && err.contains("n0620.ckpt"), "{err}");

    // Another width.
    fs::copy(&checkpoint, dir.0.join("n0400.ckpt")).unwrap();
    let err = solver::solve_game_with(400, None, &opts, optimal).unwrap_err();
    assert!(err.contains("cannot resume") && err.contains("n0400.ckpt"), "{err}");

    // The refused files are untouched, and the game itself still resumes and finishes.
    assert!(checkpoint.exists() && dir.0.join("n0620.ckpt").exists());
    let resumed = SolveOptions { node_limit: None, ..opts };
    let r = solver::solve_game_with(600, None, &resumed, optimal).unwrap();
    assert!(r.completed && r.optimal_match == Some(true));
}

#[test]
fn a_range_stops_at_an_unfinished_game_and_picks_up_where_it_left_off() {
    let dir = TempDir::new("range");
    let results = dir.0.join("results.jsonl");
    let ckpt = dir.0.join("ckpt");
    let limited = SolveOptions {
        checkpoint_dir: Some(ckpt.clone()),
        node_limit: Some(200),
        force_search: true,
        ..matching_options(1)
    };
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

/// 1500 gets `W = 24`, 2050 `W = 36` (the first board with a number above 2047 in its frames), 3600 `W = 64` (the
/// widest set up to board 4095, which the boards from 3584 up get) and 4100 `W = 128`, the widest, which the boards
/// from 4096 up get; `tests/wide.rs` goes further into the second and compares the widths from 36 to 56 with 64, and
/// the boards past 4095 with the board layer's invariants.  The setup's cost is mostly the local search for a better
/// first incumbent.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_big_board_gets_through_setup))]
fn big_boards_get_through_setup() {
    for n in [1500, 2050, 3600, 4100] {
        a_big_board_gets_through_setup_at(n);
    }
}

/// [`big_boards_get_through_setup`] for the board 1500, `W = 24`.
#[test]
fn a_big_board_gets_through_setup() {
    a_big_board_gets_through_setup_at(1500);
}

/// The pipeline takes the board `n` through its setup and one node of its search.
fn a_big_board_gets_through_setup_at(n: u32) {
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
    assert_eq!(r.moves.first().copied(), largest_prime_at_most(n), "n={n}: the game opens with the largest prime");
    assert_eq!(replay_finished(n, &r.moves), Ok(r.score), "n={n}: the game is legal, over, and scores as claimed");
    let json = r.to_json();
    assert_eq!(json.get("completed").and_then(Json::as_bool), Some(false));
    assert_eq!(json.get("optimal_match"), Some(&Json::Null));
}
