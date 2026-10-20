//! `taxman solve --keep-going`: a game whose search a limit stops does not end the range.  Its best game so far is
//! recorded as an unfinished line, and the next game goes on from it as a seed and nothing more: it takes no ceiling
//! from it, no prime formula and no proof that rests on it, so that what the lines say they had from `n − 1` is what
//! they used.  A rerun passes over the games the file holds only unfinished lines of, `--retry` searches them again,
//! and the file passes both verifiers whatever its holes.  Small boards on the matching path, one thread.

mod common;

use common::{matching_options, taxman, TempDir};
use std::fs;
use std::path::Path;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::results::{verify_files, Proof};
use taxman::solver::{self, GameResult, Method, PrevSource, SolveOptions};

/// The options of a run that a node limit stops at every game: the games 598..602 need more than 200 nodes on the
/// matching path with no previous game, and `force_search` keeps the prime 601 from being derived.
fn limited() -> SolveOptions {
    SolveOptions { node_limit: Some(200), force_search: true, keep_going: true, ..matching_options(1) }
}

fn lines(path: &Path) -> Vec<Json> {
    fs::read_to_string(path).unwrap().lines().map(|line| Json::parse(line).unwrap()).collect()
}

fn solved(opts: &SolveOptions, lo: u32, hi: u32, results: &Path) -> (Vec<GameResult>, solver::RangeSummary) {
    let mut games = Vec::new();
    let summary = solver::solve_range_each(lo, hi, opts, Some(results), |game| games.push(game.clone())).unwrap();
    (games, summary)
}

fn verified(results: &Path) -> taxman::results::Report {
    verify_files(&[results.to_owned()], Optimal::built_in(), 1).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn a_limit_does_not_end_the_range_and_the_next_game_takes_the_seed_and_nothing_more() {
    let dir = TempDir::new("keep-going");
    let results = dir.join("r.jsonl");
    let (games, summary) = solved(&limited(), 598, 602, &results);
    assert_eq!((summary.solved, summary.unfinished, summary.stopped_at), (5, 5, None));
    assert!(games.iter().all(|g| !g.completed && g.method == Method::Search), "every game stops at its limit");

    // The first game had no game before it.  Each later game was built on the best game of the one before: its
    // score, unproven (no proof), and it has no ceiling to reach.
    assert!(games[0].prev.is_none());
    for pair in games.windows(2) {
        let prev = pair[1].prev.as_ref().expect("a game before it");
        assert_eq!((prev.n, prev.score, prev.source, prev.proof), (pair[0].n, pair[0].score, PrevSource::Run, None));
        assert!(prev.assumes.is_empty());
        assert_eq!(pair[1].setup.unwrap().cap, None, "n={}: no ceiling from an unfinished game", pair[1].n);
    }
    assert!(games[1..].iter().all(|g| g.summary_line().contains("seeded by the unfinished game")));

    // The file holds five unfinished lines, passes both verifiers' checks of what they record, and a rerun passes over
    // all of them, whatever its limits.
    let file = &verified(&results).files[0];
    assert_eq!((file.contents.incomplete_lines, file.contents.games.len(), file.contents.unfinished.len()), (5, 0, 5));
    let (games, again) = solved(&limited(), 598, 602, &results);
    assert_eq!((games.len(), again.solved, again.passed_over, again.skipped), (0, 0, 5, 0));
    assert_eq!(lines(&results).len(), 5);
}

#[test]
fn a_prime_game_after_an_unfinished_game_is_searched_and_after_a_proven_one_derived() {
    let dir = TempDir::new("keep-going-prime");
    let results = dir.join("r.jsonl");
    // 600 stops at the limit; 601 is prime, and goes on from 600's best game without the prime formula.
    let (stopped, _) = solved(&limited(), 600, 600, &results);
    assert!(!stopped[0].completed);
    let unlimited = SolveOptions { node_limit: None, force_search: false, ..limited() };
    let (games, summary) = solved(&unlimited, 600, 601, &results);
    assert_eq!((games.len(), summary.passed_over), (1, 1));
    let prime = &games[0];
    assert_eq!((prime.n, prime.method, prime.proof), (601, Method::Search, Some(Proof::Exhausted)));
    assert_eq!(prime.optimal_match, Some(true), "the search finds the table's score");
    let prev = prime.prev.as_ref().unwrap();
    assert_eq!((prev.score, prev.proof, prime.setup.unwrap().cap), (stopped[0].score, None, None));
    assert!(prime.assumes.is_empty());
    verified(&results);

    // The same pair of games when 600 is proven: the prime formula.
    let proven = dir.join("proven.jsonl");
    let (games, _) = solved(&SolveOptions { keep_going: true, ..matching_options(1) }, 600, 601, &proven);
    assert_eq!((games[1].method, games[1].proof), (Method::Prime, Some(Proof::PrimeFormula)));
    assert_eq!(games[1].prev.as_ref().unwrap().proof, games[0].proof);
}

/// A repeat rests on the earlier game whose search it repeats, which must be completed: 1313 poses 1312's search problem
/// (the committed chain repeats it), and is a repeat when 1312 is proven and a search when 1312 is only an unfinished
/// line, whose best game seeds it.
#[test]
fn a_repeat_rests_on_a_completed_game_and_never_on_an_unfinished_one() {
    let dir = TempDir::new("keep-going-repeat");
    let stopping = SolveOptions { node_limit: Some(1_000), keep_going: true, ..common::options(1) };
    let (games, summary) = solved(&stopping, 1312, 1313, &dir.join("unfinished.jsonl"));
    assert_eq!((summary.unfinished, summary.stopped_at), (2, None));
    assert!(games.iter().all(|g| !g.completed && g.method == Method::Search), "1313 searches, it does not repeat 1312");
    assert_eq!(games[1].prev.as_ref().map(|p| (p.n, p.proof)), Some((1312, None)));

    let proven = SolveOptions { node_limit: None, ..stopping };
    let (games, _) = solved(&proven, 1312, 1313, &dir.join("proven.jsonl"));
    assert_eq!((games[1].method, games[1].proof), (Method::Repeat, Some(Proof::SameSearch)));
    assert_eq!(games[1].same_search.as_ref().map(|s| s.n), Some(1312));
}

#[test]
fn a_retry_searches_the_unfinished_games_again_and_each_gets_what_it_lacked() {
    let dir = TempDir::new("keep-going-retry");
    let results = dir.join("r.jsonl");
    let ckpt = dir.join("ckpt");
    let stopping = SolveOptions { checkpoint_dir: Some(ckpt.clone()), ..limited() };
    solved(&stopping, 598, 602, &results);
    assert!(
        (598..=602).all(|n| ckpt.join(format!("n{n:04}.ckpt")).exists()),
        "an unfinished game keeps its checkpoint"
    );

    // No limit, no retry: the games are passed over, their checkpoints untouched.
    let unlimited = SolveOptions { node_limit: None, force_search: false, ..stopping };
    let (_, summary) = solved(&unlimited, 598, 602, &results);
    assert_eq!((summary.solved, summary.passed_over), (0, 5));

    // With a retry each resumes from its checkpoint, and from the second on the game before it is proven by then.
    let retrying = SolveOptions { retry: true, pass: Some(2), ..unlimited };
    let (games, summary) = solved(&retrying, 598, 602, &results);
    assert_eq!((summary.solved, summary.unfinished, summary.passed_over), (5, 0, 0));
    assert!(games.iter().all(|g| g.completed && g.optimal_match == Some(true) && g.invocations == 2));
    for pair in games.windows(2) {
        let prev = pair[1].prev.as_ref().unwrap();
        assert_eq!((prev.n, prev.proof), (pair[0].n, pair[0].proof), "built on the game just proven");
        assert!(prev.proof.is_some());
    }
    assert!(ckpt.read_dir().unwrap().next().is_none(), "a finished game removes its checkpoint");

    // The file: the unfinished lines first, and the lines of the retry, which name the pass.
    let all = lines(&results);
    assert_eq!(all.len(), 10);
    let options = |line: &Json| line.get("options").unwrap().clone();
    assert_eq!(options(&all[0]).get("pass"), None, "a run with no pass says none");
    assert_eq!(options(&all[0]).get("keep_going"), Some(&Json::Bool(true)));
    assert_eq!(options(&all[5]).get("pass").and_then(Json::as_u32), Some(2));
    assert_eq!(options(&all[5]).get("retry"), Some(&Json::Bool(true)));
    let report = verified(&results);
    assert_eq!(report.files[0].contents.games.len(), 5);
    // Every game is proven now, so the report says nothing is left unfinished.
    assert!(!report.to_string().contains("unfinished:"), "{report}");
}

/// Two blocks of games made apart, as a sweep makes them: 598..599 finished, and 600..602 stopped at the limit, the first
/// of them without a game before it.  `retry_improved` searches again the games of the second that have more now: 600
/// has 599, proven, and the games after it have it in turn, proven by the time they are reached.  Games with nothing new
/// are passed over.
#[test]
fn an_improved_game_before_an_unfinished_one_is_what_a_retry_improved_searches_it_again_for() {
    let dir = TempDir::new("keep-going-improved");
    let (first, second) = (dir.join("first.jsonl"), dir.join("second.jsonl"));
    let merged = dir.join("merged.jsonl");
    solved(&SolveOptions { keep_going: true, ..matching_options(1) }, 598, 599, &first);
    solved(&limited(), 600, 602, &second);
    let both = [&first, &second].map(|path| fs::read_to_string(path).unwrap()).concat();
    fs::write(&merged, &both).unwrap();
    verified(&merged);

    // On its own the second block has nothing new: its first game has none before it, and the others are as they were.
    let improving =
        SolveOptions { node_limit: None, force_search: false, retry_improved: true, pass: Some(2), ..limited() };
    let (games, summary) = solved(&improving, 600, 602, &second);
    assert_eq!((games.len(), summary.passed_over, summary.solved), (0, 3, 0));

    // Joined to the first, 600 has 599, and each game after it the game before it, proven.
    let (games, summary) = solved(&improving, 598, 602, &merged);
    assert_eq!((summary.skipped, summary.passed_over, summary.solved), (2, 0, 3));
    let optimal = Optimal::built_in();
    for game in &games {
        assert!(game.completed && game.optimal_match == Some(true), "n={}", game.n);
        assert_eq!(game.score, optimal.score(game.n).unwrap());
        let prev = game.prev.as_ref().unwrap();
        assert!(prev.proof.is_some() && prev.assumes.is_empty(), "n={}: built on a proven game", game.n);
    }
    assert!(games[0].setup.is_some_and(|setup| setup.cap.is_some()), "600 had the ceiling 599 gives");
    let report = verified(&merged);
    assert_eq!(report.files[0].contents.games.len(), 5);

    // Nothing is left to improve.
    let (games, summary) = solved(&improving, 598, 602, &merged);
    assert_eq!((games.len(), summary.skipped, summary.passed_over), (0, 5, 0));
}

/// A game that a signal stopped is the business of the command line and cannot be raised in a test; what can is that the
/// flags are read as documented.
#[test]
fn the_command_line_reads_keep_going_retry_and_pass() {
    let dir = TempDir::new("keep-going-cli");
    let results = dir.join("r.jsonl");
    let out = common::path(&results);
    let run = taxman(&["solve", "5", "--retry", "--threads", "1"]);
    assert_eq!(run.code, Some(2), "{}", run.stderr);
    assert!(run.stderr.contains("--retry") && run.stderr.contains("--keep-going"), "{}", run.stderr);

    let matching = ["--tactic", "gf2=off", "--tactic", "symmetry=off", "--threads", "1", "--out", out];
    let stop = ["solve", "598-602", "--keep-going", "--node-limit", "200", "--force-search", "--pass", "1"];
    let first = taxman(&[&stop[..], &matching[..]].concat());
    assert_eq!(first.code, Some(0), "{}{}", first.stdout, first.stderr);
    assert!(first.stdout.contains("5 games ended at a limit, unfinished"), "{}", first.stdout);
    let again = ["solve", "598-602", "--keep-going"];
    let second = taxman(&[&again[..], &matching[..]].concat());
    assert_eq!(second.code, Some(0), "{}", second.stderr);
    assert!(second.stdout.contains("passed over 5 games"), "{}", second.stdout);
    let retry = ["solve", "598-602", "--keep-going", "--retry", "--pass", "2"];
    let third = taxman(&[&retry[..], &matching[..]].concat());
    assert_eq!(third.code, Some(0), "{}", third.stderr);
    assert_eq!(third.stdout.matches("proof=").count(), 5, "{}", third.stdout);
    let pass = |line: &Json| line.get("options").and_then(|o| o.get("pass")).and_then(Json::as_u32);
    let passes: Vec<Option<u32>> = lines(&results).iter().map(pass).collect();
    assert_eq!(passes, [vec![Some(1); 5], vec![Some(2); 5]].concat());
    let help = taxman(&["help"]);
    assert!(help.stdout.contains("--keep-going") && help.stdout.contains("--retry") && help.stdout.contains("--pass"));
}
