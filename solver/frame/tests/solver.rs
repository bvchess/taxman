//! End-to-end tests of the per-game pipeline: whole ranges against the table, tactic invariance, checkpoint and
//! resume through the solver, and boards beyond the table.

mod common;

use common::{options, TempDir};
use std::fs;
use std::time::Duration;
use taxman::bits::Bits;
use taxman::frames::BoardFrames;
use taxman::game::{largest_prime_at_most, replay_finished, Optimal};
use taxman::json::Json;
use taxman::numtheory::Tables;
use taxman::results::{verify_files, Proof};
use taxman::search::MAX_GATE;
use taxman::solver::{self, GameResult, Method, SolveOptions};

fn solve_all(lo: u32, hi: u32, opts: &SolveOptions) -> Vec<GameResult> {
    let results = solver::solve_range(lo, hi, opts, None).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(results.len(), (hi - lo + 1) as usize);
    results
}

#[test]
fn a_range_reproduces_the_table() {
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

/// Every tactic, alone and in the combinations that matter, finds the scores of the table at the games `games`.
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
            "symmetry=off",
            "conflict_branching=off",
            "conflict_look=off",
            "near_repeat=off",
            "price_reuse=off",
            "conflict_look=off,price_reuse=off",
            "carried_nogoods=on",
            "conflict_branching=off,warm_start=off",
            "price_reuse=off,warm_start=off",
            "fix_forced=off,nogoods=off,symmetry=off",
        ]
        .into_iter()
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

/// Games solved with a directory of stores of carried nogoods (`--carried-dir`, with the tactic `carried_nogoods`)
/// find the scores of the table: the store there is what a search of game 499 learns in its first steps, which
/// each of the games 500..=510 checks at its own root and searches with, with the stores that the games before it in
/// the range wrote there.
#[test]
fn games_with_a_directory_of_stores_find_their_scores() {
    let optimal = Optimal::built_in();
    let dir = TempDir::new("carried-dir");
    let game = common::Game::<8>::of_board(499).expect("game 499 has candidates");
    let lines: String = game
        .learned(common::LEARNING_STEPS)
        .iter()
        .map(|n| n.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ") + "\n")
        .collect();
    assert!(!lines.is_empty(), "the search of 499 learned no nogood");
    fs::write(taxman::model::store_file(&dir, 499), lines).unwrap();
    let mut opts = SolveOptions { force_search: true, carried_dir: Some(dir.to_path_buf()), ..options(1) };
    opts.tactics.search.carried_nogoods = true;
    for r in solve_all(500, 510, &opts) {
        assert!(r.completed, "n={}", r.n);
        assert_eq!(Some(r.score), optimal.score(r.n), "n={}", r.n);
    }
    // Each game that searched wrote its own store there, of what its searchers learned, for the games after it.
    let written: Vec<u32> = (500..=510).filter(|&n| taxman::model::store_file(&dir, n).exists()).collect();
    assert!(!written.is_empty(), "no game wrote a store");
    for n in written {
        let text = fs::read_to_string(taxman::model::store_file(&dir, n)).unwrap();
        let sets = taxman::model::CarriedNogoods::<8>::parse(&text).unwrap();
        assert!(!sets.is_empty(), "n={n}: an empty store");
    }
}

/// A folder of carried nogoods that does not exist yet is made when the run starts, and the game that searches writes
/// its store there.
#[test]
fn a_missing_folder_of_stores_is_made_and_written_to() {
    let dir = TempDir::new("carried-made");
    let folder = dir.join("stores");
    let mut opts = SolveOptions { force_search: true, carried_dir: Some(folder.clone()), ..options(1) };
    opts.tactics.search.carried_nogoods = true;
    solve_all(500, 500, &opts);
    assert!(folder.is_dir(), "the run made its folder");
    assert!(taxman::model::store_file(&folder, 500).exists(), "game 500 wrote its store there");
}

/// Every tactic, alone and in the combinations that matter, finds the scores of the table at the games 1..=300.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is every_tactic_gives_the_same_scores_at_a_few_games")]
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
    assert!(results.iter().all(|r| matches!(r.method, Method::Trivial | Method::Start | Method::Search)));
    assert!(results.iter().all(|r| r.optimal_match == Some(true)));
}

#[test]
fn an_interrupted_game_resumes_from_its_checkpoint() {
    let optimal = Optimal::built_in();
    let dir = TempDir::new("resume");
    // Without the n − 1 solution to start from, the search of 800 has real work to do: some 1,400 steps, more than the
    // 1,024 of a searcher after which a node limit is looked at.
    const N: u32 = 800;
    let opts = SolveOptions {
        checkpoint_dir: Some(dir.0.clone()),
        checkpoint_interval: Duration::from_secs(3600),
        node_limit: Some(1_000),
        queue_mem_cap: 4,
        force_search: true,
        ..options(1)
    };
    let checkpoint = dir.0.join(format!("n{N:04}.ckpt"));
    let mut pieces = 0;
    let result = loop {
        let r = solver::solve_game_with(N, None, &opts, optimal).unwrap();
        pieces += 1;
        if r.completed {
            break r;
        }
        assert!(checkpoint.exists(), "an unfinished game keeps its checkpoint");
        assert_eq!((r.optimal_match, r.method), (None, Method::Search));
        assert!(r.score > 0 && r.score <= optimal.score(N).unwrap(), "the best game so far, not more than the best");
        assert!(pieces < 100, "no progress");
    };
    assert!(pieces >= 2, "only {pieces} pieces");
    assert_eq!((result.score, result.optimal_match), (optimal.score(N).unwrap(), Some(true)));
    assert!(!checkpoint.exists(), "a finished game removes its checkpoint");
    let left: Vec<_> = fs::read_dir(&dir.0).unwrap().map(|e| e.unwrap().path()).collect();
    assert!(left.is_empty(), "left behind: {left:?}");
}

/// Solves `n` and `n + 1` (the prime after it, which takes `n` as its predecessor) from `base`, with no previous game to
/// start from: a time limit of nothing stops the search of `n` before it starts, and a second run, without the limit,
/// resumes it and goes on.  Returns the games of the second run and the lines of the results file, which the verifier
/// accepts.
fn run_stopped_and_resumed(name: &str, base: SolveOptions, n: u32) -> (Vec<GameResult>, Vec<Json>) {
    let dir = TempDir::new(name);
    let results = dir.join("results.jsonl");
    let limited = SolveOptions { checkpoint_dir: Some(dir.join("ckpt")), time_limit: Some(Duration::ZERO), ..base };
    let summary = solver::solve_range_each(n, n + 1, &limited, Some(&results), |_| {}).unwrap();
    assert_eq!(summary.stopped_at, Some(n), "the time limit stops the search of {n}");
    assert!(limited.checkpoint_file(n).unwrap().exists());

    let unlimited = SolveOptions { time_limit: None, ..limited };
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
    let (games, lines) = run_stopped_and_resumed("resume", options(1), 600);
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

/// The finder, started as early as it can be, changes no score: games 999..=1001 on two threads, whose searches take
/// thousands of steps (no ceiling from the game before, `prev_cap` off, so that they are searched to the end), have
/// the table's scores with it as without it, and the finder's record is in the lines of the games it started for,
/// which the verifier accepts.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is a_finder_started_early_changes_no_score_of_one_game")]
fn a_finder_started_early_changes_no_score() {
    a_finder_started_early_changes_no_score_of(999, 1001, 2);
}

/// [`a_finder_started_early_changes_no_score`] for game 800 alone on one thread, whose search from nothing takes some
/// 1,400 steps, more than the 1,024 of a searcher after which the finder can look.
#[test]
fn a_finder_started_early_changes_no_score_of_one_game() {
    a_finder_started_early_changes_no_score_of(800, 800, 1);
}

/// The games `lo..=hi` on `threads` threads, with the finder as early as it can be and without it.
fn a_finder_started_early_changes_no_score_of(lo: u32, hi: u32, threads: usize) {
    let optimal = Optimal::built_in();
    let dir = TempDir::new("finder-scores");
    let mut base = SolveOptions { force_search: true, ..options(threads) };
    base.tactics.prev_cap = false;
    let without = solve_all(lo, hi, &base);
    let finder = SolveOptions { finder_after: 1, finder_budget: Duration::from_secs(1), ..base };
    let results = dir.join("results.jsonl");
    let with = solve_all(lo, hi, &SolveOptions { checkpoint_dir: None, ..finder.clone() });
    let written = solver::solve_range(lo, hi, &finder, Some(&results)).unwrap_or_else(|e| panic!("{e}"));
    for ((a, b), c) in without.iter().zip(&with).zip(&written) {
        assert!(a.completed && b.completed && c.completed, "n={}", a.n);
        assert_eq!(Some(a.score), optimal.score(a.n), "n={}", a.n);
        assert_eq!((b.score, c.score), (a.score, a.score), "n={}: the finder changed the score", a.n);
        assert!(a.finder.is_none(), "n={}: no finder was asked for", a.n);
    }
    // A search that ends before the finder looks reports nothing, but of thousands of steps some do start it.
    let started: Vec<_> = written.iter().filter_map(|r| r.finder.map(|f| (r.n, f))).collect();
    assert!(!started.is_empty(), "no game started its finder");
    for (n, f) in &started {
        assert!(f.best >= f.from && f.refused == 0 && f.seconds <= 1.5, "n={n}: {f:?}");
    }
    // The lines say what the finder did, where it did something, and what it was asked for.
    let lines: Vec<Json> =
        fs::read_to_string(&results).unwrap().lines().map(|line| Json::parse(line).unwrap()).collect();
    for (line, game) in lines.iter().zip(&written) {
        let options = line.get("options").unwrap();
        assert_eq!(options.get("finder_after").and_then(Json::as_u64), Some(1), "n={}", game.n);
        assert_eq!(options.get("finder_seconds").and_then(Json::as_f64), Some(1.0), "n={}", game.n);
        let record = line.get("engine").unwrap().get("finder");
        assert_eq!(record.is_some(), game.finder.is_some(), "n={}", game.n);
        if let (Some(record), Some(f)) = (record, game.finder) {
            assert_eq!(record.get("at_nodes").and_then(Json::as_u64), Some(f.at_nodes), "n={}", game.n);
            assert_eq!(record.get("best").and_then(Json::as_u64), Some(f.best), "n={}", game.n);
            assert_eq!(record.get("improved").and_then(Json::as_u64), Some(f.improved), "n={}", game.n);
            assert_eq!(record.get("refused").and_then(Json::as_u64), Some(0), "n={}", game.n);
        }
    }
    verify_files(std::slice::from_ref(&results), Optimal::built_in(), 1).unwrap_or_else(|e| panic!("{e}"));
}

/// A checkpoint is the same search whatever the finder: a search stopped by a node limit is resumed, piece after piece,
/// with the finder off, on early, off, and on later and with another budget, and finishes with the right score.  Game
/// 1000 from nothing takes more than three times the 1,024 steps after which a node limit stops a search on one thread.
#[test]
fn a_checkpoint_resumes_with_any_finder_settings() {
    const N: u32 = 1000;
    let optimal = Optimal::built_in();
    let dir = TempDir::new("finder-resume");
    let base = SolveOptions {
        checkpoint_dir: Some(dir.0.clone()),
        checkpoint_interval: Duration::from_secs(3600),
        node_limit: Some(1_000),
        queue_mem_cap: 4,
        force_search: true,
        ..options(1)
    };
    let settings = [(0, 60), (1, 1), (0, 60), (2_000, 5), (1, 60)];
    let mut pieces = 0;
    let result = loop {
        let (finder_after, seconds) = settings[pieces % settings.len()];
        let opts = SolveOptions { finder_after, finder_budget: Duration::from_secs(seconds), ..base.clone() };
        let r = solver::solve_game_with(N, None, &opts, optimal).unwrap_or_else(|e| panic!("piece {pieces}: {e}"));
        pieces += 1;
        if r.completed {
            break r;
        }
        assert!(base.checkpoint_file(N).unwrap().exists(), "an unfinished game keeps its checkpoint");
        assert!(pieces < 100, "no progress");
    };
    assert!(pieces >= 3, "only {pieces} pieces");
    assert_eq!((result.score, result.optimal_match), (optimal.score(N).unwrap(), Some(true)));
}

/// A checkpoint resumes under either setting of `conflict_branching` and of `conflict_look` (bits 9 and 10 of its
/// tactics bind nothing) and under any gate (no part of it), as a game does that resumes at the run's `--gate` after a
/// gate trial chose another.  Game 1000, whose default search takes more than the 1,024 steps after which a node limit
/// stops it, is stopped again and again and resumed, each piece under settings and a gate that differ from those of the
/// piece before.  It finishes with its score in the table.
#[test]
fn a_checkpoint_resumes_under_other_branching_settings_and_gates() {
    let optimal = Optimal::built_in();
    let dir = TempDir::new("branching-resume");
    let base =
        SolveOptions { checkpoint_dir: Some(dir.0.clone()), node_limit: Some(1), force_search: true, ..options(1) };
    let settings = [(true, false), (false, true), (true, true), (false, false)];
    let gates = [300, 150, MAX_GATE, 600];
    let mut pieces = 0;
    let result = loop {
        let (conflict_branching, conflict_look) = settings[pieces % settings.len()];
        let mut opts = SolveOptions { gate: Some(gates[pieces % gates.len()]), ..base.clone() };
        opts.tactics.search.conflict_branching = conflict_branching;
        opts.tactics.search.conflict_look = conflict_look;
        let r = solver::solve_game_with(1000, None, &opts, optimal).unwrap_or_else(|e| panic!("piece {pieces}: {e}"));
        let ran = &r.tactics.search;
        assert_eq!((ran.conflict_branching, ran.conflict_look), (conflict_branching, conflict_look), "piece {pieces}");
        assert_eq!(r.gate, opts.gate, "piece {pieces}");
        pieces += 1;
        if r.completed {
            break r;
        }
        assert!(base.checkpoint_file(1000).unwrap().exists(), "an unfinished game keeps its checkpoint");
        assert!(pieces < 50, "no progress");
    };
    assert!(pieces >= 2, "no piece resumed a search");
    assert_eq!((result.score, result.optimal_match), (optimal.score(1000).unwrap(), Some(true)));
}

#[test]
fn a_checkpoint_for_another_game_or_other_tactics_is_refused() {
    let optimal = Optimal::built_in();
    let dir = TempDir::new("refuse");
    // A time limit of nothing stops the search before it starts, with its checkpoint.
    let opts = SolveOptions {
        checkpoint_dir: Some(dir.0.clone()),
        time_limit: Some(Duration::ZERO),
        force_search: true,
        ..options(1)
    };
    assert!(!solver::solve_game_with(600, None, &opts, optimal).unwrap().completed);
    let checkpoint = dir.0.join("n0600.ckpt");

    // Other tactics: each of the search switches that a checkpoint records and binds, turned the other way.
    let assignments = ["fix_forced=off", "nogoods=off", "forget_nogoods=off", "symmetry=off"];
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
    let resumed = SolveOptions { time_limit: None, ..opts };
    let r = solver::solve_game_with(600, None, &resumed, optimal).unwrap();
    assert!(r.completed && r.optimal_match == Some(true));
}

#[test]
fn a_range_stops_at_an_unfinished_game_and_picks_up_where_it_left_off() {
    let dir = TempDir::new("range");
    let results = dir.0.join("results.jsonl");
    let ckpt = dir.0.join("ckpt");
    // A time limit of nothing stops the first search before it starts, with its checkpoint.
    let limited = SolveOptions {
        checkpoint_dir: Some(ckpt.clone()),
        time_limit: Some(Duration::ZERO),
        force_search: true,
        ..options(1)
    };
    let mut seen = Vec::new();
    let summary =
        solver::solve_range_each(598, 602, &limited, Some(&results), |r| seen.push((r.n, r.completed))).unwrap();
    assert_eq!(seen.last().map(|g| g.1), Some(false), "{seen:?}");
    assert_eq!(summary.stopped_at, seen.last().map(|g| g.0));
    assert!(ckpt.join(format!("n{:04}.ckpt", summary.stopped_at.unwrap())).exists());

    // Without the limit the same command finishes the game from its checkpoint and goes on.
    let unlimited = SolveOptions { time_limit: None, ..limited };
    let mut more = Vec::new();
    let summary = solver::solve_range_each(598, 602, &unlimited, Some(&results), |r| more.push(r.n)).unwrap();
    assert_eq!(summary.stopped_at, None);
    assert_eq!(more, (seen.last().unwrap().0..=602).collect::<Vec<_>>());
    assert_eq!(summary.skipped, seen.len() - 1);
    let left: Vec<_> = fs::read_dir(&ckpt).unwrap().map(|e| e.unwrap().path()).collect();
    assert!(left.is_empty(), "left behind: {left:?}");

    // Every n is in the file as a completed game exactly once, and the unfinished one also as such.
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

/// The gate of the run (`--gate`) reaches the searchers of a game and changes its tree, not its answer.  Game 780,
/// searched from nothing on one thread, is solved with conflict branching off; under a run gate of `MAX_GATE`, wider
/// than any gap of the game, which gives the tree of conflict branching off; and under a run gate of 0, so that
/// conflict branching chooses at every check, which gives another.  Each ends with its score in the table, and its line
/// records the run's gate.
#[test]
fn the_runs_gate_reaches_the_searchers() {
    const N: u32 = 780;
    let optimal = Optimal::built_in();
    let solved = |tag: &str, branching: bool, gate: Option<u64>| {
        let mut opts = SolveOptions { force_search: true, gate, ..options(1) };
        opts.tactics.search.conflict_branching = branching;
        let r = solver::solve_game_with(N, None, &opts, optimal).unwrap_or_else(|e| panic!("{tag}: {e}"));
        let answer = (r.score, r.completed, r.optimal_match);
        assert_eq!(answer, (optimal.score(N).unwrap(), true, Some(true)), "{tag}");
        let recorded = r.to_json().get("options").and_then(|options| options.get("gate")).and_then(Json::as_u64);
        assert_eq!(recorded, gate, "{tag}: the line's record of the run's gate");
        r.nodes
    };
    let off = solved("run-gate-off", false, None);
    assert_eq!(solved("run-gate-wide", true, Some(MAX_GATE)), off, "a gate wider than every gap is no gate");
    let at_every_check = solved("run-gate-0", true, Some(0));
    assert_ne!(at_every_check, off, "a run gate of 0 took the steps of conflict branching off");
}

/// 1500 gets `W = 24`, 2050 `W = 36` (the first board with a number above 2047 in its frames), 3600 `W = 64` (the
/// widest set up to board 4095, which the boards from 3584 up get) and 4100 `W = 128`, the widest, which the boards
/// from 4096 up get; `tests/wide.rs` goes further into the second and compares the widths from 36 to 56 with 64, and
/// the boards past 4095 with the board layer's invariants.  The setup's cost is mostly the local search for a better
/// first incumbent.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is a_big_board_gets_through_setup")]
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
