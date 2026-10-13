//! The integrity of results: what the solver checks before it builds on a previous game, the tripwires that
//! catch one that is not optimal, how taint travels down a chain, what a results file must pass when it is
//! opened, what survives a failure after a search, and the provenance every line carries.  Small boards, one or
//! two threads.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use taxman::bits::Bits;
use taxman::game::{legal_moves_left, replay, Optimal};
use taxman::hives::Apiary;
use taxman::json::Json;
use taxman::numtheory::Tables;
use taxman::results::{moves_digest, rejected_path, Proof, ResultsFile, Taint, OPTIMAL_JSON};
use taxman::solver::{self, GameResult, Prev, PrevSource, SolveOptions};

/// A fresh directory under the system temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("taxman-integrity-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        TempDir(path)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn options(threads: usize) -> SolveOptions {
    SolveOptions { threads, ..SolveOptions::default() }
}

/// No unproven tactic on.
fn strict(threads: usize) -> SolveOptions {
    let mut opts = options(threads);
    opts.tactics.set("reuse_prev=off").unwrap();
    opts.tactics.set("revise_freedoms=off").unwrap();
    opts
}

fn lines(path: &Path) -> Vec<Json> {
    fs::read_to_string(path).unwrap().lines().map(|line| Json::parse(line).unwrap()).collect()
}

fn completed_line(path: &Path, n: u32) -> Json {
    let is = |l: &&Json| l.get("n").and_then(Json::as_u32) == Some(n) && l.get("completed") == Some(&Json::Bool(true));
    lines(path).iter().find(is).cloned().unwrap_or_else(|| panic!("no completed line for n={n}"))
}

fn taint_of_line(line: &Json) -> Taint {
    let taint = line.get("taint").and_then(Json::as_arr).expect("a taint");
    taint.iter().map(|t| t.as_str().unwrap().to_owned()).collect()
}

fn prev(moves: Vec<u32>, score: u64) -> Prev {
    Prev { moves, score, source: PrevSource::Results, proof: Some(Proof::Exhausted), taint: Taint::new() }
}

/// A game of 80 that is legal, over and opens with 79, as an optimal game does, but scores less than the
/// optimum: optimal.json's game cut short and finished by always playing the smallest legal move.
fn finished_but_worse_game_of_80() -> (Vec<u32>, u64) {
    let optimal = Optimal::load_default().unwrap();
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
    let opts = SolveOptions { optimal_path: None, ..strict(1) };
    let optimal = Optimal::load_default().unwrap();
    // Not over: optimal.json's game of 80 without its last move.
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
    let opts = SolveOptions { optimal_path: None, ..strict(1) };
    let err = solver::solve_game(81, Some(&prev(moves, score)), &opts).unwrap_err();
    assert!(err.contains("exceed the cap") && err.contains("not optimal"), "{err}");
    // The error keeps the promotions that were found.
    assert!(err.contains("promotions ["), "{err}");
}

#[test]
fn a_ceiling_tripped_in_a_range_is_recorded_as_rejected() {
    let dir = TempDir::new("tripped");
    let results = dir.path("r.jsonl");
    let (moves, score) = finished_but_worse_game_of_80();
    // The bad game as optimal.json's game of 80, so that the range builds on it.
    let fake = Json::array([Json::object([
        ("n", Json::from(80u32)),
        ("score", Json::from(score)),
        ("moves", Json::array(moves.iter().copied())),
    ])]);
    let optimal = dir.path("optimal.json");
    fs::write(&optimal, fake.to_string()).unwrap();
    let opts = SolveOptions { optimal_path: Some(optimal), allow_tainted_prev: true, ..strict(1) };
    let err = solver::solve_range(81, 82, &opts, Some(&results)).unwrap_err();
    assert!(err.contains("not optimal") && err.contains("r.rejected.jsonl"), "{err}");
    assert!(lines(&results).is_empty(), "nothing is recorded as a result");
    let rejected = lines(&rejected_path(&results));
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].get("n").and_then(Json::as_u32), Some(81));
    assert!(rejected[0].get("rejected").and_then(Json::as_str).unwrap().contains("exceed the cap"));
    assert!(!rejected[0].get("promotions").and_then(Json::as_arr).unwrap().is_empty());
}

// ---------------------------------------------------------------------------------------------------------
// Taint

#[test]
fn a_strict_run_does_not_build_on_a_tainted_game_unless_told_to() {
    let dir = TempDir::new("taint");
    let results = dir.path("r.jsonl");
    // With the default tactics, 22 is derived by reuse_prev, and 23 (a prime) rests on it.
    solver::solve_range(1, 22, &options(1), Some(&results)).unwrap();
    let line = completed_line(&results, 22);
    assert_eq!(line.get("proof").and_then(Json::as_str), Some("reuse_prev"));
    assert_eq!(taint_of_line(&line), Taint::from(["reuse_prev".to_owned()]));

    let err = solver::solve_range(1, 26, &strict(1), Some(&results)).unwrap_err();
    assert!(
        err.contains("n=23") && err.contains("rests on reuse_prev") && err.contains("--allow-tainted-prev"),
        "{err}"
    );

    // Told to go on, the strict games inherit the taint exactly where their proofs rest on the game before.
    let allowed = SolveOptions { allow_tainted_prev: true, ..strict(1) };
    let games = solver::solve_range(1, 26, &allowed, Some(&results)).unwrap();
    assert_eq!(games.iter().map(|g| g.n).collect::<Vec<_>>(), [23, 24, 25, 26]);
    let mut before = taint_of_line(&line);
    for game in &games {
        let proof = game.proof.unwrap();
        let expected = if proof.rests_on_prev() { before.clone() } else { Taint::new() };
        assert_eq!(game.taint, expected, "n={} ({})", game.n, proof.as_str());
        assert!(game.allow_tainted_prev);
        before = game.taint.clone();
    }
    assert_eq!(games[0].proof, Some(Proof::PrimeFormula));
    assert_eq!(games[0].taint, Taint::from(["reuse_prev".to_owned()]));
    // The file reads back with the same taints.
    let reread = ResultsFile::open(&results, None).unwrap();
    assert!(games.iter().all(|g| reread.completed(g.n).unwrap().taint == g.taint));
}

#[test]
fn a_game_made_with_unproven_tactics_on_but_not_used_is_clean() {
    let dir = TempDir::new("clean");
    let results = dir.path("r.jsonl");
    // 30 is searched to exhaustion with the default tactics, and revise_freedoms does nothing there.
    solver::solve_range(1, 30, &options(1), Some(&results)).unwrap();
    let line = completed_line(&results, 30);
    assert_eq!(line.get("proof").and_then(Json::as_str), Some("exhausted"));
    assert!(taint_of_line(&line).is_empty());
    let games = solver::solve_range(1, 34, &strict(1), Some(&results)).unwrap();
    assert_eq!(games.len(), 4);
    assert!(games.iter().all(|g| g.taint.is_empty()), "{:?}", games.iter().map(|g| &g.taint).collect::<Vec<_>>());
}

#[test]
fn a_strict_run_from_optimal_json_needs_the_override() {
    let err = solver::solve_range(50, 52, &strict(1), None).unwrap_err();
    assert!(err.contains("n=50") && err.contains("rests on optimal.json"), "{err}");
    let allowed = SolveOptions { allow_tainted_prev: true, ..strict(1) };
    let games = solver::solve_range(50, 52, &allowed, None).unwrap();
    let first = &games[0];
    assert_eq!(first.prev.as_ref().map(|p| (p.n, p.source, p.proof)), Some((49, PrevSource::Optimal, None)));
    let expected =
        if first.proof.unwrap().rests_on_prev() { Taint::from([OPTIMAL_JSON.to_owned()]) } else { Taint::new() };
    assert_eq!(first.taint, expected);
    // With the default tactics (unproven ones on) there is nothing to refuse.
    assert!(solver::solve_range(50, 52, &options(1), None).is_ok());
}

// ---------------------------------------------------------------------------------------------------------
// The results file

/// A results file holding games 1..=12, and its lines as text.
fn twelve_games(dir: &TempDir) -> (PathBuf, Vec<String>) {
    let results = dir.path("r.jsonl");
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
        let err = taxman::results::verify_files(std::slice::from_ref(&results), None).unwrap_err();
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
    // A recorded taint or proof that is not what the chain gives.
    assert!(tamper(11, "\"taint\":[]", "\"taint\":[\"x\"]").contains("records the taint"));
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
    let bad = format!("{good}{{\"n\":13}}\n");
    fs::write(&results, &bad).unwrap();
    let err = solver::solve_range(1, 13, &options(1), Some(&results)).unwrap_err();
    assert!(err.contains("line 13: missing \"completed\""), "{err}");
    assert_eq!(fs::read_to_string(&results).unwrap(), bad);
}

#[test]
fn a_stale_checkpoint_of_a_completed_game_is_removed() {
    let dir = TempDir::new("stale");
    let (results, _) = twelve_games(&dir);
    let checkpoints = dir.path("ckpt");
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
    let results = dir.path("r.jsonl");
    let checkpoints = dir.path("ckpt");
    let checkpoint = checkpoints.join("n0600.ckpt");
    let truth = Optimal::load_default().unwrap().score(600).unwrap();
    // An optimal.json that is wrong about 600 (and has no 599, so the search starts from nothing).
    let wrong = dir.path("wrong.json");
    fs::write(&wrong, format!(r#"[{{"n":600,"score":{},"moves":[]}}]"#, truth + 1)).unwrap();
    let opts = SolveOptions {
        checkpoint_dir: Some(checkpoints.clone()),
        checkpoint_interval: Duration::from_secs(3600),
        node_limit: Some(1_000),
        optimal_path: Some(wrong),
        ..options(2)
    };
    // A limit stops the search: a checkpoint and an incomplete line.
    let summary = solver::solve_range_each(600, 600, &opts, Some(&results), |_| {}).unwrap();
    assert_eq!(summary.stopped_at, Some(600));
    assert!(checkpoint.exists());

    // The search completes, then the comparison fails: the game is recorded as rejected and the checkpoint
    // survives, so the work is not lost.
    let unlimited = SolveOptions { node_limit: None, ..opts.clone() };
    let err = solver::solve_range(600, 600, &unlimited, Some(&results)).unwrap_err();
    assert!(err.contains(&format!("says {}", truth + 1)) && err.contains("r.rejected.jsonl"), "{err}");
    assert!(checkpoint.exists(), "the checkpoint is gone after a failure that followed the search");
    let rejected = lines(&rejected_path(&results));
    assert_eq!(rejected.len(), 1);
    let found = &rejected[0];
    assert_eq!(
        (found.get("score").and_then(Json::as_u64), found.get("completed")),
        (Some(truth), Some(&Json::Bool(true)))
    );
    assert!(found.get("rejected").and_then(Json::as_str).unwrap().contains("optimal.json says"));
    assert!(!found.get("promotions").and_then(Json::as_arr).unwrap().is_empty());
    assert_eq!(lines(&results).len(), 1, "only the incomplete line is a result");

    // With the right file the search resumes from the checkpoint and the result is saved before the
    // checkpoint goes.
    let right = SolveOptions { optimal_path: Some(Optimal::default_path()), ..unlimited };
    let games = solver::solve_range(600, 600, &right, Some(&results)).unwrap();
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
    let checkpoints = dir.path("ckpt");
    let opts =
        SolveOptions { checkpoint_dir: Some(checkpoints), node_limit: Some(200), force_search: true, ..options(1) };
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
    assert_eq!(done.get("schema").and_then(Json::as_u32), Some(2));
    assert!(done.get("proof").and_then(Json::as_str).is_some_and(|p| Proof::parse(p).is_some()));
    assert!(done.get("reached_cap").and_then(Json::as_bool).is_some());
    assert!(done.get("taint").and_then(Json::as_arr).is_some());
    assert!(done.get("in_the_bag").and_then(Json::as_u64).is_some());
    assert_eq!(done.get("invocations").and_then(Json::as_u32), Some(1));
    let seconds = done.get("seconds").unwrap();
    assert!(seconds.get("search").is_some() && seconds.get("search_all_runs").is_some());
    let limits = done.get("limits").unwrap();
    assert_eq!((limits.get("nodes"), limits.get("seconds")), (Some(&Json::Null), Some(&Json::Null)));
    let options = done.get("options").unwrap();
    assert_eq!(options.get("force_search"), Some(&Json::Bool(false)));
    assert_eq!(options.get("allow_tainted_prev"), Some(&Json::Bool(false)));
    assert_eq!(options.get("optimal_file").and_then(Json::as_str), Some("default"));
    // The previous game, identified by its moves.
    let before = &all[10];
    let record = done.get("prev").unwrap();
    assert_eq!(record.get("n").and_then(Json::as_u32), Some(11));
    assert_eq!((record.get("score"), record.get("proof")), (before.get("score"), before.get("proof")));
    let moves = before.get("moves").and_then(Json::as_u32_vec).unwrap();
    assert_eq!(record.get("moves_digest").and_then(Json::as_str), Some(moves_digest(&moves).as_str()));
    assert_eq!(record.get("source").and_then(Json::as_str), Some("run"));

    // An unfinished game makes no claim: no score that looks final, no proof, no taint.
    assert_eq!(unfinished.get("completed"), Some(&Json::Bool(false)));
    assert_eq!(unfinished.get("score"), Some(&Json::Null));
    assert!(unfinished.get("best_so_far").and_then(Json::as_u64).is_some_and(|s| s > 0));
    assert_eq!((unfinished.get("proof"), unfinished.get("taint")), (Some(&Json::Null), Some(&Json::Null)));
    assert_eq!(unfinished.get("invocations").and_then(Json::as_u32), Some(1));
    let limits = unfinished.get("limits").unwrap();
    assert_eq!(limits.get("nodes").and_then(Json::as_u64), Some(200));
    assert_eq!(unfinished.get("options").unwrap().get("force_search"), Some(&Json::Bool(true)));
    // The file reads back, with the unfinished search's invocation counted.
    assert_eq!(ResultsFile::open(&results, None).unwrap().earlier_runs(600), 1);
}

#[test]
fn a_resumed_game_counts_its_invocations_and_its_time() {
    let dir = TempDir::new("invocations");
    let results = dir.path("r.jsonl");
    let opts = SolveOptions {
        checkpoint_dir: Some(dir.path("ckpt")),
        checkpoint_interval: Duration::from_secs(3600),
        node_limit: Some(1_000),
        force_search: true,
        ..options(2)
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
    let opts = SolveOptions { threads: 1, optimal_path: None, force_search: true, ..SolveOptions::default() };
    for r in solver::solve_range(2, 160, &opts, None).unwrap() {
        let tables = Tables::new(r.n);
        let bag: u64 = Apiary::<8>::new(&tables, Bits::EMPTY).solution().iter().map(|&m| u64::from(m)).sum();
        let sum: u64 = r.promotions.iter().map(|&p| u64::from(p)).sum();
        assert_eq!((r.in_the_bag, r.score), (Some(bag), bag + sum), "n={}", r.n);
    }
}
