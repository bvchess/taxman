//! The integrity of results: what the solver checks before it builds on a previous game, the tripwires that catch one
//! that is not optimal, how an assumption travels down a chain from the table, what a results file must pass when
//! it is opened, what survives a failure after a search, and the provenance every line carries.  Small boards, one or
//! two threads.

mod common;

use common::{matching_options, options, TempDir};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use taxman::bits::Bits;
use taxman::frames::BoardFrames;
use taxman::game::{legal_moves_left, replay, Optimal};
use taxman::json::Json;
use taxman::numtheory::Tables;
use taxman::results::{assumed_optimal, moves_digest, rejected_path, Assumptions, Proof, ResultsFile, SCHEMA};
use taxman::solver::{self, GameResult, Prev, PrevSource, SolveOptions};

fn lines(path: &Path) -> Vec<Json> {
    fs::read_to_string(path).unwrap().lines().map(|line| Json::parse(line).unwrap()).collect()
}

fn completed_line(path: &Path, n: u32) -> Json {
    let is = |l: &&Json| l.get("n").and_then(Json::as_u32) == Some(n) && l.get("completed") == Some(&Json::Bool(true));
    lines(path).iter().find(is).cloned().unwrap_or_else(|| panic!("no completed line for n={n}"))
}

fn assumptions_of_line(line: &Json) -> Assumptions {
    let assumes = line.get("assumes").and_then(Json::as_arr).expect("assumptions");
    assumes.iter().map(|a| a.as_str().unwrap().to_owned()).collect()
}

fn prev(moves: Vec<u32>, score: u64) -> Prev {
    Prev {
        moves,
        score,
        source: PrevSource::Results,
        proof: Some(Proof::Exhausted),
        assumes: Assumptions::new(),
        proven: true,
    }
}

/// A game of 80 that is legal, over and opens with 79, as an optimal game does, but scores less than
/// opt(n): the table's game cut short and finished by always playing the smallest legal move.
fn finished_but_worse_game_of_80() -> (Vec<u32>, u64) {
    let optimal = Optimal::built_in();
    let best = optimal.moves(80).unwrap();
    for cut in 1..best.len() {
        let mut moves = best[..best.len() - cut].to_vec();
        while let Some(&m) = legal_moves_left(80, &moves).unwrap().first() {
            moves.push(m);
        }
        let score = replay(80, &moves).unwrap();
        // Low enough that a game of 81 beats n + score(80) by a margin.
        if score + 81 + 50 < optimal.score(81).unwrap() {
            return (moves, score);
        }
    }
    panic!("no such game");
}

// ---------------------------------------------------------------------------------------------------------
// The previous game

#[test]
fn a_previous_game_that_cannot_be_optimal_is_refused() {
    let opts = options(1);
    let optimal = Optimal::built_in();
    // Not over: the table's game of 80 without its last move.
    let mut moves = optimal.moves(80).unwrap().to_vec();
    moves.pop();
    let score = replay(80, &moves).unwrap();
    let err = solver::solve_game(81, Some(&prev(moves, score)), &opts).unwrap_err();
    assert!(err.contains("cannot be optimal") && err.contains("not over"), "{err}");
    // Over, but opening with 73 rather than 79, so 79 is never played.
    let mut moves = vec![73];
    while let Some(&m) = legal_moves_left(80, &moves).unwrap().first() {
        moves.push(m);
    }
    let score = replay(80, &moves).unwrap();
    let err = solver::solve_game(81, Some(&prev(moves.clone(), score)), &opts).unwrap_err();
    assert!(err.contains("opens with 73, not with 79"), "{err}");
    // The same for the prime shortcut, which would otherwise replace the first move whatever it is.
    let mut moves = vec![73];
    while let Some(&m) = legal_moves_left(82, &moves).unwrap().first() {
        moves.push(m);
    }
    let score = replay(82, &moves).unwrap();
    assert!(solver::solve_game(83, Some(&prev(moves, score)), &opts).unwrap_err().contains("not with 79"));
}

/// The reviewers' demonstrator: a predecessor that is not optimal no longer yields a completed result above
/// n + its score.
#[test]
fn a_previous_game_that_is_not_optimal_trips_the_ceiling() {
    let (moves, score) = finished_but_worse_game_of_80();
    let err = solver::solve_game(81, Some(&prev(moves, score)), &options(1)).unwrap_err();
    assert!(err.contains("exceed the cap") && err.contains("not optimal"), "{err}");
    // The error keeps the promotions that were found.
    assert!(err.contains("promotions ["), "{err}");
}

#[test]
fn a_ceiling_tripped_in_a_range_is_recorded_as_rejected() {
    let dir = TempDir::new("tripped");
    let results = dir.join("r.jsonl");
    let (moves, score) = finished_but_worse_game_of_80();
    // The bad game as a table's game of 80, so that a range told to use it builds on it.
    let fake = Json::array([Json::object([
        ("n", Json::from(80u32)),
        ("score", Json::from(score)),
        ("moves", Json::array(moves.iter().copied())),
    ])]);
    let table = Optimal::from_json(&fake).unwrap();
    let opts = SolveOptions { use_prev: true, ..options(1) };
    let err = solver::solve_range_each_with(81, 82, &opts, Some(&results), &table, |_| {}).unwrap_err();
    assert!(err.contains("not optimal") && err.contains("r.rejected.jsonl"), "{err}");
    assert!(lines(&results).is_empty(), "nothing is recorded as a result");
    let rejected = lines(&rejected_path(&results));
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].get("n").and_then(Json::as_u32), Some(81));
    assert!(rejected[0].get("rejected").and_then(Json::as_str).unwrap().contains("exceed the cap"));
    assert!(!rejected[0].get("promotions").and_then(Json::as_arr).unwrap().is_empty());
}

// ---------------------------------------------------------------------------------------------------------
// Assumptions

/// A run told to (`use_prev`) builds on a game of the table, which it assumes optimal, and each result records
/// that assumption exactly where its proof rests on it: a proof by `cap` or the prime formula on the game before, a
/// repeat on the game it repeats, and an exhausted search on no game.
#[test]
fn the_results_built_on_a_game_of_the_table_assume_it_as_far_as_their_proofs_rest_on_it() {
    let dir = TempDir::new("assumes");
    let results = dir.join("r.jsonl");
    // 33 reaches its ceiling and 34 repeats it, 35 is an exhausted search; 37 is a prime built on 36, an exhausted
    // search.
    let opts = SolveOptions { use_prev: true, ..options(1) };
    let games = solver::solve_range(33, 50, &opts, Some(&results)).unwrap();
    let first = &games[0];
    assert_eq!(first.prev.as_ref().map(|p| (p.n, p.source, p.proof)), Some((32, PrevSource::Optimal, None)));
    let table_assumed = Assumptions::from([assumed_optimal(32)]);
    let mut assumed: BTreeMap<u32, Assumptions> = BTreeMap::new();
    assumed.insert(32, table_assumed.clone());
    for game in &games {
        let proof = game.proof.unwrap();
        let rests_on = match &game.same_search {
            Some(same) => same.n,
            None => game.n - 1,
        };
        let expected = if proof.rests_on_another_game() { assumed[&rests_on].clone() } else { Assumptions::new() };
        assert_eq!(game.assumes, expected, "n={} ({})", game.n, proof.as_str());
        assumed.insert(game.n, game.assumes.clone());
    }
    assert_eq!((first.proof, &first.assumes), (Some(Proof::Cap), &table_assumed));
    assert_eq!((games[1].proof, &games[1].assumes), (Some(Proof::SameSearch), &table_assumed));
    assert_eq!((games[2].proof, games[2].assumes.is_empty()), (Some(Proof::Exhausted), true));
    assert_eq!((games[4].proof, games[4].assumes.is_empty()), (Some(Proof::PrimeFormula), true));
    // The file reads back with the same assumptions, and the lines carry them.
    let reread = ResultsFile::open(&results, Optimal::built_in(), 1).unwrap();
    assert!(games.iter().all(|g| reread.completed(g.n).unwrap().assumes == g.assumes));
    assert_eq!(assumptions_of_line(&completed_line(&results, 34)), table_assumed);
    assert!(assumptions_of_line(&completed_line(&results, 35)).is_empty());
}

/// A chain from n = 1 assumes nothing.
#[test]
fn a_chain_from_one_assumes_nothing() {
    let games = solver::solve_range(1, 120, &options(1), None).unwrap();
    assert!(games.iter().all(|g| g.assumes.is_empty()), "{:?}", games.iter().find(|g| !g.assumes.is_empty()));
}

// ---------------------------------------------------------------------------------------------------------
// The results file

/// A results file holding games 1..=12, and its lines as text.
fn twelve_games(dir: &TempDir) -> (PathBuf, Vec<String>) {
    let results = dir.join("r.jsonl");
    solver::solve_range(1, 12, &options(1), Some(&results)).unwrap();
    let text = fs::read_to_string(&results).unwrap();
    (results, text.lines().map(str::to_owned).collect())
}

#[test]
fn a_tampered_results_file_is_refused() {
    let dir = TempDir::new("tamper");
    let (results, original) = twelve_games(&dir);
    let tamper = |i: usize, from: &str, to: &str| {
        let mut changed = original.clone();
        assert!(changed[i].contains(from), "{from} not in line {}", i + 1);
        changed[i] = changed[i].replacen(from, to, 1);
        fs::write(&results, changed.join("\n") + "\n").unwrap();
        let err = solver::solve_range(1, 14, &options(1), Some(&results)).unwrap_err();
        assert!(err.contains(&format!("line {} (n={})", i + 1, i + 1)), "{err}");
        let err = taxman::results::verify_files(std::slice::from_ref(&results), Optimal::built_in(), 1).unwrap_err();
        assert!(err.contains(&format!("line {} ", i + 1)), "{err}");
        err
    };
    // The reviewers' demonstrator: a wrong score is not accepted as done.
    assert!(tamper(9, "\"score\":40", "\"score\":45").contains("not the recorded 45"));
    // Two moves swapped: 9 then 6 is legal, but 6 then 9 is not (6 takes 3, and 9 then pays no tax).
    let line = &original[10];
    let moves = Json::parse(line).unwrap().get("moves").unwrap().to_string();
    assert!(moves.contains("9,6") || moves.contains("6,9"), "{moves}");
    let swapped = if moves.contains("9,6") { moves.replace("9,6", "6,9") } else { moves.replace("6,9", "9,6") };
    assert!(tamper(10, &moves, &swapped).contains("pays no tax") || tamper(10, &moves, &swapped).contains("taken"));
    // A recorded assumption or proof that is not what the chain gives.
    assert!(tamper(11, "\"assumes\":[]", "\"assumes\":[\"x\"]").contains("records the assumptions"));
    // Nothing was appended by the refused runs.
    fs::write(&results, original.join("\n") + "\n").unwrap();
    assert_eq!(solver::solve_range(1, 14, &options(1), Some(&results)).unwrap().len(), 2);
}

#[test]
fn a_torn_last_line_is_cut_off_and_a_bad_one_is_an_error() {
    let dir = TempDir::new("torn");
    let (results, original) = twelve_games(&dir);
    let good = original.join("\n") + "\n";
    // Garbage before the newline, as power loss can leave it: torn, cut off, and game 13 solved again.
    fs::write(&results, format!("{good}\0\0\0\0\0\0\0\0\0\0,\"score\":5}}\n")).unwrap();
    let more = solver::solve_range(1, 13, &options(1), Some(&results)).unwrap();
    assert_eq!(more.iter().map(|g| g.n).collect::<Vec<_>>(), [13]);
    assert_eq!(lines(&results).len(), 13);
    // A last line that is JSON but not a result is not torn: it is an error, and the file is left alone.
    let bad = format!("{good}{{\"schema\":{SCHEMA},\"n\":13}}\n");
    fs::write(&results, &bad).unwrap();
    let err = solver::solve_range(1, 13, &options(1), Some(&results)).unwrap_err();
    assert!(err.contains("line 13: missing \"completed\""), "{err}");
    assert_eq!(fs::read_to_string(&results).unwrap(), bad);
}

#[test]
fn a_stale_checkpoint_of_a_completed_game_is_removed() {
    let dir = TempDir::new("stale");
    let (results, _) = twelve_games(&dir);
    let checkpoints = dir.join("ckpt");
    fs::create_dir_all(&checkpoints).unwrap();
    // As a crash between saving game 10's result and deleting its checkpoint would leave it.
    for name in ["n0010.ckpt", "n0010.ckpt.tmp", "n0013.ckpt"] {
        fs::write(checkpoints.join(name), b"stale").unwrap();
    }
    let opts = SolveOptions { checkpoint_dir: Some(checkpoints.clone()), ..options(1) };
    solver::solve_range(1, 12, &opts, Some(&results)).unwrap();
    let left: Vec<String> =
        fs::read_dir(&checkpoints).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    // 13 is not completed (nor in the range), so its checkpoint is not this run's business.
    assert_eq!(left, ["n0013.ckpt"]);
}

// ---------------------------------------------------------------------------------------------------------
// A failure after the search

#[test]
fn a_failure_after_the_search_keeps_the_checkpoint_and_records_the_game() {
    let dir = TempDir::new("durable");
    let results = dir.join("r.jsonl");
    let checkpoints = dir.join("ckpt");
    let checkpoint = checkpoints.join("n0600.ckpt");
    let truth = Optimal::built_in().score(600).unwrap();
    // A table that is wrong about 600 (and the range starts from nothing).
    let wrong = format!(r#"[{{"n":600,"score":{},"moves":[]}}]"#, truth + 1);
    let wrong = Optimal::from_json(&Json::parse(&wrong).unwrap()).unwrap();
    let opts = SolveOptions {
        checkpoint_dir: Some(checkpoints.clone()),
        checkpoint_interval: Duration::from_secs(3600),
        node_limit: Some(1_000),
        ..matching_options(2)
    };
    // A limit stops the search: a checkpoint and an incomplete line.
    let summary = solver::solve_range_each_with(600, 600, &opts, Some(&results), &wrong, |_| {}).unwrap();
    assert_eq!(summary.stopped_at, Some(600));
    assert!(checkpoint.exists());

    // The search completes, then the comparison fails: the game is recorded as rejected and the checkpoint
    // survives, so the work is not lost.
    let unlimited = SolveOptions { node_limit: None, ..opts.clone() };
    let err = solver::solve_range_each_with(600, 600, &unlimited, Some(&results), &wrong, |_| {}).unwrap_err();
    assert!(err.contains(&format!("says {}", truth + 1)) && err.contains("r.rejected.jsonl"), "{err}");
    assert!(checkpoint.exists(), "the checkpoint is gone after a failure that followed the search");
    let rejected = lines(&rejected_path(&results));
    assert_eq!(rejected.len(), 1);
    let found = &rejected[0];
    assert_eq!(
        (found.get("score").and_then(Json::as_u64), found.get("completed")),
        (Some(truth), Some(&Json::Bool(true)))
    );
    assert!(found.get("rejected").and_then(Json::as_str).unwrap().contains("the table says"));
    assert!(!found.get("promotions").and_then(Json::as_arr).unwrap().is_empty());
    assert_eq!(lines(&results).len(), 1, "only the incomplete line is a result");

    // With the right table (the built-in one) the search resumes from the checkpoint and the result is saved before
    // the checkpoint goes.
    let games = solver::solve_range(600, 600, &unlimited, Some(&results)).unwrap();
    assert_eq!((games[0].score, games[0].completed, games[0].invocations), (truth, true, 2));
    assert!(games[0].phases.search_all_runs >= games[0].phases.search);
    assert!(!checkpoint.exists());
    assert_eq!(completed_line(&results, 600).get("invocations").and_then(Json::as_u32), Some(2));
}

// ---------------------------------------------------------------------------------------------------------
// Provenance

#[test]
fn result_lines_carry_their_provenance() {
    let dir = TempDir::new("provenance");
    let (results, _) = twelve_games(&dir);
    // Game 600, stopped by a limit, in the same file.
    let checkpoints = dir.join("ckpt");
    let opts = SolveOptions {
        checkpoint_dir: Some(checkpoints),
        node_limit: Some(200),
        force_search: true,
        ..matching_options(1)
    };
    let mut seen: Vec<GameResult> = Vec::new();
    solver::solve_range_each(600, 600, &opts, Some(&results), |r| seen.push(r.clone())).unwrap();
    assert!(!seen[0].completed, "600 needs more than 200 nodes");
    let all = lines(&results);
    let (done, unfinished) = (&all[11], &all[12]);

    let build = done.get("build").unwrap();
    assert!(build.get("commit").is_some() && build.get("dirty").is_some());
    if let Some(commit) = build.get("commit").and_then(Json::as_str) {
        assert_eq!(commit.len(), 40, "{commit}");
    }
    assert_eq!(done.get("schema").and_then(Json::as_u32), Some(SCHEMA));
    assert!(done.get("proof").and_then(Json::as_str).is_some_and(|p| Proof::parse(p).is_some()));
    assert!(done.get("reached_cap").and_then(Json::as_bool).is_some());
    assert!(done.get("assumes").and_then(Json::as_arr).is_some());
    assert!(done.get("in_the_bag").and_then(Json::as_u64).is_some());
    assert_eq!(done.get("invocations").and_then(Json::as_u32), Some(1));
    let seconds = done.get("seconds").unwrap();
    assert!(seconds.get("search").is_some() && seconds.get("search_all_runs").is_some());
    let limits = done.get("limits").unwrap();
    assert_eq!((limits.get("nodes"), limits.get("seconds")), (Some(&Json::Null), Some(&Json::Null)));
    let options = done.get("options").unwrap();
    assert_eq!(options.get("force_search"), Some(&Json::Bool(false)));
    assert_eq!(options.get("use_prev"), Some(&Json::Bool(false)));
    assert!(options.get("optimal_file").is_none() && done.get("prev_source").is_none());
    assert!(seconds.get("revise").is_none() && seconds.get("greedy").is_some());
    // The previous game, identified by its moves.
    let before = &all[10];
    let record = done.get("prev").unwrap();
    assert_eq!(record.get("n").and_then(Json::as_u32), Some(11));
    assert_eq!((record.get("score"), record.get("proof")), (before.get("score"), before.get("proof")));
    let moves = before.get("moves").and_then(Json::as_u32_vec).unwrap();
    assert_eq!(record.get("moves_digest").and_then(Json::as_str), Some(moves_digest(&moves).as_str()));
    assert_eq!(record.get("source").and_then(Json::as_str), Some("run"));

    // An unfinished game makes no claim: no score that looks final, no proof, no assumptions.
    assert_eq!(unfinished.get("completed"), Some(&Json::Bool(false)));
    assert_eq!(unfinished.get("score"), Some(&Json::Null));
    assert!(unfinished.get("best_so_far").and_then(Json::as_u64).is_some_and(|s| s > 0));
    assert_eq!((unfinished.get("proof"), unfinished.get("assumes")), (Some(&Json::Null), Some(&Json::Null)));
    assert_eq!(unfinished.get("invocations").and_then(Json::as_u32), Some(1));
    let limits = unfinished.get("limits").unwrap();
    assert_eq!(limits.get("nodes").and_then(Json::as_u64), Some(200));
    assert_eq!(unfinished.get("options").unwrap().get("force_search"), Some(&Json::Bool(true)));
    // The file reads back, with the unfinished search's invocation counted.
    assert_eq!(ResultsFile::open(&results, Optimal::built_in(), 1).unwrap().earlier_runs(600), 1);
}

/// A game's own search time, `seconds.own` of its line, and its best promotion sum end its summary line: a game with no
/// search to run has no own search time, and a game with no `in_the_bag` (a prime one) has `-` for its best sum.
#[test]
fn a_summary_line_ends_with_the_games_own_search_time_and_its_best_promotion_sum() {
    let games = solver::solve_range(1, 60, &options(1), None).unwrap();
    let methods: Vec<&str> = games.iter().map(|g| g.method.as_str()).collect();
    for kind in ["trivial", "prime", "repeat", "search"] {
        assert!(methods.contains(&kind), "no {kind} game in 1..=60: {methods:?}");
    }
    for game in &games {
        let line = game.to_json();
        let own = line.get("seconds").and_then(|seconds| seconds.get("own")).and_then(Json::as_f64);
        assert_eq!(own, Some(game.phases.own.as_secs_f64()), "n={}", game.n);
        match game.method.as_str() {
            "trivial" | "prime" | "repeat" => assert_eq!(game.phases.own, Duration::ZERO, "n={}", game.n),
            "search" => assert!(game.phases.own > Duration::ZERO, "n={}", game.n),
            _ => {}
        }
        let best = game.in_the_bag.map_or_else(|| "-".to_owned(), |bag| (game.score - bag).to_string());
        let ends = format!("  own={:.3} s  best={best}", game.phases.own.as_secs_f64());
        assert!(game.summary_line().ends_with(&ends), "n={}: {}", game.n, game.summary_line());
        assert_eq!(best == "-", game.method.as_str() == "prime", "n={}", game.n);
    }
}

#[test]
fn a_resumed_game_counts_its_invocations_and_its_time() {
    let dir = TempDir::new("invocations");
    let results = dir.join("r.jsonl");
    let opts = SolveOptions {
        checkpoint_dir: Some(dir.join("ckpt")),
        checkpoint_interval: Duration::from_secs(3600),
        node_limit: Some(1_000),
        force_search: true,
        ..matching_options(2)
    };
    let mut runs = 0;
    let last = loop {
        runs += 1;
        let games = solver::solve_range(600, 600, &opts, Some(&results)).unwrap();
        if games[0].completed {
            break games[0].clone();
        }
        assert!(runs < 100, "no progress");
    };
    assert!(runs >= 3, "only {runs} invocations");
    assert_eq!(last.invocations, runs);
    assert!(last.phases.search_all_runs > last.phases.search, "{:?}", last.phases);
    let lines = lines(&results);
    assert_eq!(lines.len(), runs as usize);
    let recorded: Vec<u32> = lines.iter().map(|l| l.get("invocations").and_then(Json::as_u32).unwrap()).collect();
    assert_eq!(recorded, (1..=runs).collect::<Vec<_>>());
}

/// The reviewers' invariant, now a check the solver makes on every game from the frame model.
#[test]
fn every_game_from_the_frame_model_scores_in_the_bag_plus_its_promotions() {
    let opts = SolveOptions { force_search: true, ..options(1) };
    for r in solver::solve_range(2, 160, &opts, None).unwrap() {
        let tables = Tables::new(r.n);
        let bag: u64 = BoardFrames::<8>::new(&tables, Bits::EMPTY).solution().iter().map(|&m| u64::from(m)).sum();
        let sum: u64 = r.promotions.iter().map(|&p| u64::from(p)).sum();
        assert_eq!((r.in_the_bag, r.score), (Some(bag), bag + sum), "n={}", r.n);
    }
}
