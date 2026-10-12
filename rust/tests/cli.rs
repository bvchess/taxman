//! The command line: every subcommand's happy path and its usage errors, through the real binary.

mod common;

use common::TempDir;
use std::fs;
use std::path::Path;
use std::process::Command;
use taxman::game::Optimal;
use taxman::json::Json;

/// What a run of the binary did.
struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn taxman(args: &[&str]) -> Run {
    let out = Command::new(env!("CARGO_BIN_EXE_taxman")).args(args).output().expect("run taxman");
    Run {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

fn path(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// A usage error: exit code 2, the message and then the usage on stderr, nothing on stdout.
fn assert_usage_error(args: &[&str], message: &str) {
    let run = taxman(args);
    assert_eq!(run.code, Some(2), "{args:?}: {}", run.stderr);
    assert!(run.stderr.contains(message), "{args:?}: expected {message:?} in {:?}", run.stderr);
    assert!(run.stderr.contains("usage: taxman <command>"), "{args:?}: the usage follows the message");
    assert_eq!(run.stdout, "", "{args:?}");
}

#[test]
fn help_lists_the_commands_and_every_tactic() {
    for args in [&["help"][..], &["--help"], &["-h"]] {
        let run = taxman(args);
        assert_eq!(run.code, Some(0));
        for text in ["solve N|A-B", "regress", "bench", "verify-optimal", "tables N K...", "--history FILE"] {
            assert!(run.stdout.contains(text), "{args:?}: no {text:?}");
        }
        for name in taxman::solver::Tactics::names() {
            assert!(run.stdout.contains(name), "{args:?}: the help does not name the tactic {name}");
        }
    }
}

#[test]
fn bad_command_lines_are_usage_errors() {
    assert_usage_error(&[], "no command given");
    assert_usage_error(&["frobnicate"], "unknown command");
    assert_usage_error(&["solve"], "usage: solve N|A-B");
    assert_usage_error(&["solve", "5-3"], "empty");
    assert_usage_error(&["solve", "x"], "expected a board size N or a range A-B");
    assert_usage_error(&["solve", "12", "--tactic", "nogoods=maybe"], "expected on or off");
    assert_usage_error(&["solve", "12", "--tactic", "fix_forced"], "expected name=value");
    assert_usage_error(&["solve", "12", "--tactic", "sparkle=on"], "unknown tactic");
    assert_usage_error(&["solve", "12", "--threads", "0"], "--threads must be at least 1");
    assert_usage_error(&["solve", "12", "--nope"], "unknown option --nope");
    assert_usage_error(&["solve", "12", "--time-limit", "-1"], "not a number of seconds");
    assert_usage_error(&["solve", "12", "--node-limit", "many"], "cannot read");
    assert_usage_error(&["regress", "12"], "regress takes no positional arguments");
    assert_usage_error(&["regress", "--optimal", "none"], "--optimal none makes no sense");
    assert_usage_error(&["bench", "--optimal", "none"], "--optimal none makes no sense");
    assert_usage_error(&["bench", "many"], "not a board size");
    assert_usage_error(&["tables"], "usage: tables N K...");
    assert_usage_error(&["tables", "30"], "usage: tables N K...");
    assert_usage_error(&["tables", "30", "31"], "not on the board");
    assert_usage_error(&["tables", "x", "5"], "N must be");
    assert_usage_error(&["verify-optimal", "a", "b"], "at most one argument");
}

#[test]
fn tables_prints_the_maximal_factors_and_composites() {
    let run = taxman(&["tables", "30", "12", "30"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert_eq!(lines[0], "12: maximal factors [4, 6]; maximal composites [24]");
    assert_eq!(lines[1], "30: maximal factors [6, 10, 15]; maximal composites []");
}

#[test]
fn verify_optimal_replays_a_file_and_names_the_game_that_fails() {
    let run = taxman(&["verify-optimal"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("verified 1000 games (n up to 1000"), "{}", run.stdout);

    let dir = TempDir::new("verify");
    let good = dir.join("optimal.json");
    fs::write(&good, mini_optimal(None)).unwrap();
    let run = taxman(&["verify-optimal", path(&good)]);
    assert!(run.code == Some(0) && run.stdout.contains("verified 12 games (n up to 12"), "{}", run.stdout);

    let bad = dir.join("bad.json");
    fs::write(&bad, mini_optimal(Some(10))).unwrap();
    let run = taxman(&["verify-optimal", path(&bad)]);
    assert_eq!(run.code, Some(1));
    assert!(run.stderr.contains("n=10: recorded score 41 but the moves score 40"), "{}", run.stderr);

    let run = taxman(&["verify-optimal", path(&dir.join("missing.json"))]);
    assert_eq!(run.code, Some(1));
    assert!(run.stderr.contains("missing.json"), "{}", run.stderr);
}

/// Games 1 to 12 of `optimal.json` as a file of their own, with the score of game `wrong` raised by one.
fn mini_optimal(wrong: Option<u32>) -> String {
    let optimal = Optimal::load_default().unwrap();
    let games = optimal.games().take(12).map(|(n, score, moves)| {
        let score = score + u64::from(wrong == Some(n));
        Json::object([
            ("n", Json::from(n)),
            ("score", Json::from(score)),
            ("moves", Json::array(moves.iter().copied())),
        ])
    });
    Json::array(games).to_string()
}

#[test]
fn solve_writes_its_results_skips_what_is_done_and_resumes_what_was_stopped() {
    let dir = TempDir::new("solve");
    let results = dir.join("results.jsonl");
    let args = |range: &'static str| ["solve", range, "--threads", "1", "--progress", "0", "--out", path(&results)];
    let first = args("5-9");
    let run = taxman(&first);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout.lines().count(), 5, "{}", run.stdout);
    assert!(run.stdout.contains("n=9 ") && run.stdout.contains("optimal ok"), "{}", run.stdout);

    // The same file again, a little further: what is done is skipped.
    let run = taxman(&args("5-11"));
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("skipped 5 games already done"), "{}", run.stdout);
    assert_eq!(fs::read_to_string(&results).unwrap().lines().count(), 7);

    // --show prints the promotions and the moves.
    let run = taxman(&["solve", "10", "--threads", "1", "--progress", "0", "--show"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("promotions: ") && run.stdout.contains("moves: [7, 9, 6, 10, 8]"), "{}", run.stdout);

    // A search stopped by a limit keeps its checkpoint, says so, and the same command finishes it.
    let ckpt = dir.join("ckpt");
    let limited =
        ["solve", "600", "--force-search", "--threads", "1", "--progress", "0", "--checkpoint-dir", path(&ckpt)];
    let run = taxman(&[&limited[..], &["--node-limit", "1"]].concat());
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("stopped: the search for n=600 hit a limit"), "{}", run.stdout);
    assert!(run.stdout.contains("run the same command to resume it"), "{}", run.stdout);
    assert!(ckpt.join("n0600.ckpt").exists());
    let run = taxman(&limited);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("optimal ok") && !run.stdout.contains("stopped"), "{}", run.stdout);
    assert!(!ckpt.join("n0600.ckpt").exists(), "a finished game removes its checkpoint");
}

#[test]
fn regress_checks_scores_against_optimal_json() {
    let run = taxman(&["regress", "--to", "40", "--threads", "1"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("solved 1..=40 (1 threads, tactics default)"), "{}", run.stdout);
    assert!(run.stdout.contains("40 games match optimal.json, 0 do not, 0 are not in it"), "{}", run.stdout);

    let run = taxman(&["regress", "--from", "30", "--to", "34", "--threads", "1", "--tactic", "reuse_prev=off"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("solved 30..=34 (1 threads, tactics reuse_prev=off)"), "{}", run.stdout);

    // A score that differs is reported with its board size, and the exit code says so.
    let dir = TempDir::new("regress");
    let wrong = dir.join("wrong.json");
    fs::write(&wrong, mini_optimal(Some(10))).unwrap();
    let run = taxman(&["regress", "--to", "12", "--threads", "1", "--optimal", path(&wrong)]);
    assert_eq!(run.code, Some(1));
    assert!(run.stdout.contains("MISMATCH n=10: scored 40 but optimal.json says 41"), "{}", run.stdout);
    assert!(run.stdout.contains("11 games match optimal.json, 1 do not"), "{}", run.stdout);
    assert!(run.stderr.contains("1 scores differ from optimal.json"), "{}", run.stderr);
}

#[test]
fn bench_appends_to_the_history_it_is_given_and_not_to_the_tracked_one() {
    let tracked = Path::new(env!("CARGO_MANIFEST_DIR")).join("bench/history.jsonl");
    let before = fs::read(&tracked).ok();
    let dir = TempDir::new("bench");
    let history = dir.join("nested").join("history.jsonl");
    for runs in 1..=2 {
        let run = taxman(&["bench", "480", "--threads", "1", "--history", path(&history)]);
        assert_eq!(run.code, Some(0), "{}", run.stderr);
        assert!(run.stdout.contains("1 threads, tactics default"), "{}", run.stdout);
        assert!(run.stdout.contains("appended to ") && run.stdout.contains("history.jsonl"), "{}", run.stdout);
        let text = fs::read_to_string(&history).unwrap();
        assert_eq!(text.lines().count(), runs, "{text}");
    }
    let record = Json::parse(fs::read_to_string(&history).unwrap().lines().last().unwrap()).unwrap();
    let games = record.get("games").and_then(Json::as_arr).unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].get("n").and_then(Json::as_u32), Some(480));
    assert_eq!(games[0].get("score").and_then(Json::as_u64), Optimal::load_default().unwrap().score(480));
    assert_eq!(games[0].get("optimal_match").and_then(Json::as_bool), Some(true));
    assert_eq!(record.get("threads").and_then(Json::as_u32), Some(1));
    assert!(games[0].get("nodes").and_then(Json::as_u64).is_some_and(|nodes| nodes > 0));
    assert_eq!(fs::read(&tracked).ok(), before, "a test must not touch the tracked benchmark history");
}
