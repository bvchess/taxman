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
        for text in [
            "solve N|A-B",
            "regress",
            "bench",
            "verify-optimal",
            "extend-optimal RESULTS --out FILE",
            "tables N K...",
            "--history FILE",
        ] {
            assert!(run.stdout.contains(text), "{args:?}: no {text:?}");
        }
        for name in taxman::tactics::Tactics::names() {
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
    assert_usage_error(&["bench", "many"], "not a board size");
    assert_usage_error(&["tables"], "usage: tables N K...");
    assert_usage_error(&["tables", "30"], "usage: tables N K...");
    assert_usage_error(&["tables", "30", "31"], "not on the board");
    assert_usage_error(&["tables", "x", "5"], "N must be");
    assert_usage_error(&["verify-optimal", "a", "b"], "at most one argument");
    assert_usage_error(&["extend-optimal"], "usage: extend-optimal RESULTS --out FILE");
    assert_usage_error(&["extend-optimal", "a", "b", "--out", "c"], "usage: extend-optimal RESULTS --out FILE");
    assert_usage_error(&["extend-optimal", "a"], "extend-optimal needs --out FILE");
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
    let table = Optimal::built_in();
    let (games, top) = (table.sizes().count(), table.max_n());
    assert!(run.stdout.contains(&format!("verified {games} games (n up to {top},")), "{}", run.stdout);

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
    let optimal = Optimal::built_in();
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

// ---------------------------------------------------------------------------------------------------------
// extend-optimal

/// The first `count` games of `optimal.json` as a file in its format: its own lines, so that what a command must
/// keep as it was can be compared byte for byte.
fn table_text(count: usize) -> String {
    let lines: Vec<&str> = taxman::game::BUILT_IN.lines().skip(1).take(count).collect();
    format!("[\n{}\n]", lines.join("\n").trim_end_matches(','))
}

/// Writes the first `count` games of `optimal.json` to `name` in `dir`.
fn write_table(dir: &Path, name: &str, count: usize) -> String {
    let file = dir.join(name);
    fs::write(&file, table_text(count)).unwrap();
    path(&file).to_owned()
}

/// Solves `range` into the results file `name` in `dir` (every game chained from the one before, or standalone)
/// and returns the file's path.
fn solve_into(dir: &Path, name: &str, range: &str) -> String {
    let file = dir.join(name);
    let args = ["solve", range, "--threads", "1", "--progress", "0", "--out", path(&file)];
    let run = taxman(&args);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    path(&file).to_owned()
}

/// A results file cut after the line of `n` (lines are in order of n), with `extra` lines after it.
fn results_cut_after(file: &str, n: u32, extra: &[Json]) -> String {
    let text = fs::read_to_string(file).unwrap();
    let kept = text.lines().take(n as usize).map(|line| format!("{line}\n"));
    kept.chain(extra.iter().map(|line| format!("{line}\n"))).collect()
}

/// The line of a search of `n` that a limit stopped.
fn incomplete_line(n: u32) -> Json {
    Json::object([
        ("schema", Json::from(taxman::results::SCHEMA)),
        ("n", Json::from(n)),
        ("completed", Json::Bool(false)),
        ("invocations", Json::from(1u32)),
    ])
}

/// What `extend-optimal` writes for a table of `kept` (the text of a table) and the games `first..=last` of the
/// results file `results`: the table's lines as they were, then the results' games in the table's format.
fn extended_text(kept: &str, results: &str, first: u32, last: u32) -> String {
    let games = fs::read_to_string(results).unwrap();
    let new: Vec<String> = games
        .lines()
        .map(|line| Json::parse(line).unwrap())
        .filter(|game| game.get("n").and_then(Json::as_u32).is_some_and(|n| (first..=last).contains(&n)))
        .map(|game| {
            let moves: Vec<String> =
                game.get("moves").unwrap().as_u32_vec().unwrap().iter().map(u32::to_string).collect();
            let (n, score) = (game.get("n").unwrap(), game.get("score").unwrap());
            format!("  {{ \"n\": {n}, \"score\": {score}, \"moves\": [{}] }}", moves.join(", "))
        })
        .collect();
    assert_eq!(new.len(), (last - first + 1) as usize);
    format!("{},\n{}\n]", kept.strip_suffix("\n]").unwrap(), new.join(",\n"))
}

#[test]
fn extend_optimal_with_nothing_to_add_writes_the_table_back_byte_for_byte() {
    let dir = TempDir::new("extend-nothing");
    let results = solve_into(&dir, "r.jsonl", "1-12");
    let out = dir.join("out.json");
    let run = taxman(&["extend-optimal", &results, "--out", path(&out)]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(fs::read_to_string(&out).unwrap(), taxman::game::BUILT_IN);

    let top = Optimal::built_in().max_n();
    assert!(run.stdout.contains("added no games; the table has "), "{}", run.stdout);
    assert!(run.stdout.contains(&format!("n up to {top}; wrote ")), "{}", run.stdout);
    let end = format!("stopped at n = {}: the results end at n = 12", top + 1);
    assert!(run.stdout.contains(&end), "{}", run.stdout);
    // The temporary file of the atomic write is gone, and the results are as they were.
    let mut names: Vec<_> =
        fs::read_dir(&*dir).unwrap().map(|e| e.unwrap().file_name().into_string().unwrap()).collect();
    names.sort();
    assert_eq!(names, ["out.json", "r.jsonl"]);
}

#[test]
fn extend_optimal_appends_the_games_that_continue_the_table() {
    let dir = TempDir::new("extend-appends");
    let table = write_table(&dir, "mini.json", 12);
    let results = solve_into(&dir, "r.jsonl", "1-20");
    let out = dir.join("out.json");
    let run = taxman(&["extend-optimal", &results, "--table", &table, "--out", path(&out)]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(
        run.stdout.contains("added 8 games (n = 13..=20); the table has 20 games, n up to 20; wrote "),
        "{}",
        run.stdout
    );
    assert!(run.stdout.contains("stopped at n = 21: the results end at n = 20"), "{}", run.stdout);
    // The old games as they were, byte for byte, then the new ones; and the input table is not touched.
    assert_eq!(fs::read_to_string(&out).unwrap(), extended_text(&table_text(12), &results, 13, 20));
    assert_eq!(fs::read_to_string(&table).unwrap(), table_text(12));

    // The result is a table like any other.
    let run = taxman(&["verify-optimal", path(&out)]);
    assert!(run.code == Some(0) && run.stdout.contains("verified 20 games (n up to 20"), "{}", run.stdout);
    // And a second extension, by the same results, has nothing left to add.
    let again = dir.join("again.json");
    let run = taxman(&["extend-optimal", &results, "--table", path(&out), "--out", path(&again)]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("added no games"), "{}", run.stdout);
    assert_eq!(fs::read(&again).unwrap(), fs::read(&out).unwrap());
}

#[test]
fn extend_optimal_writes_over_the_table_only_when_asked_to_by_name() {
    let dir = TempDir::new("extend-in-place");
    let table = write_table(&dir, "table.json", 12);
    let results = solve_into(&dir, "r.jsonl", "1-14");
    let run = taxman(&["extend-optimal", &results, "--table", &table, "--out", &table]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(fs::read_to_string(&table).unwrap(), extended_text(&table_text(12), &results, 13, 14));

    // The results themselves are never overwritten, by whatever path they are named.
    let before = fs::read(&results).unwrap();
    let alias = format!("{}/./r.jsonl", path(&dir));
    for out in [&results, &alias] {
        assert_usage_error(&["extend-optimal", &results, "--table", &table, "--out", out], "would be overwritten");
    }
    assert_eq!(fs::read(&results).unwrap(), before);
}

#[test]
fn extend_optimal_stops_at_the_first_game_that_is_not_there() {
    let dir = TempDir::new("extend-stops");
    let table = write_table(&dir, "mini.json", 12);
    let out = dir.join("out.json");
    let extend = |results: &str| {
        let _ = fs::remove_file(&out);
        taxman(&["extend-optimal", results, "--table", &table, "--out", path(&out)])
    };
    let kept = table_text(12);

    // Games that assume a game of the table optimal are appended: 33 is built on optimal.json's game of 32
    // (`--use-prev`) and reaches its ceiling, which assumes nothing the table does not stand for.
    let table32 = write_table(&dir, "table32.json", 32);
    let assuming = dir.join("assuming.jsonl");
    let run = taxman(&["solve", "33-40", "--use-prev", "--threads", "1", "--progress", "0", "--out", path(&assuming)]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("assumes the game of 32 in optimal.json is optimal"), "{}", run.stdout);
    let run = taxman(&["extend-optimal", path(&assuming), "--table", &table32, "--out", path(&out)]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("added 8 games (n = 33..=40)"), "{}", run.stdout);
    assert_eq!(fs::read_to_string(&out).unwrap(), extended_text(&table_text(32), path(&assuming), 33, 40));

    // A search that a limit stopped: the file ends with one of 16 ...
    let chain = solve_into(&dir, "chain.jsonl", "1-20");
    let unfinished = dir.join("unfinished.jsonl");
    fs::write(&unfinished, results_cut_after(&chain, 15, &[incomplete_line(16)])).unwrap();
    let run = extend(path(&unfinished));
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("added 3 games (n = 13..=15)"), "{}", run.stdout);
    assert!(
        run.stdout.contains("stopped at n = 16: the results have only a search of it that a limit stopped"),
        "{}",
        run.stdout
    );
    assert_eq!(fs::read_to_string(&out).unwrap(), extended_text(&kept, &chain, 13, 15));

    // ... and the same file after the run was resumed: the completed game of 16 is the game.
    let text = fs::read_to_string(&chain).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let resumed = dir.join("resumed.jsonl");
    let resumed_text = [&lines[..15], &[&incomplete_line(16).to_string()], &lines[15..]].concat().join("\n");
    fs::write(&resumed, resumed_text + "\n").unwrap();
    let run = extend(path(&resumed));
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("added 8 games (n = 13..=20)"), "{}", run.stdout);
    assert_eq!(fs::read_to_string(&out).unwrap(), extended_text(&kept, &chain, 13, 20));

    // A gap: the file has a game of 20 solved on its own, and nothing between 12 and it.
    let gap = solve_into(&dir, "gap.jsonl", "20");
    let run = extend(&gap);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("added no games"), "{}", run.stdout);
    let reason = "stopped at n = 13: the results have no completed game of it, but go on to n = 20";
    assert!(run.stdout.contains(reason), "{}", run.stdout);
    assert_eq!(fs::read_to_string(&out).unwrap(), kept);
}

#[test]
fn extend_optimal_refuses_what_it_cannot_trust() {
    let dir = TempDir::new("extend-refuses");
    let results = solve_into(&dir, "r.jsonl", "1-20");
    let out = dir.join("out.json");

    // Results that score a board size differently from the table: not the same game, or one of them is wrong.
    let wrong = dir.join("wrong.json");
    fs::write(&wrong, mini_optimal(Some(10))).unwrap();
    let run = taxman(&["extend-optimal", &results, "--table", path(&wrong), "--out", path(&out)]);
    assert_eq!(run.code, Some(1), "{}", run.stdout);
    assert!(run.stderr.contains("line 10 (n=10): scores 40, but optimal.json has 41"), "{}", run.stderr);
    assert!(!out.exists(), "nothing is written when the extension is refused");

    // Results the reader rejects: a line before the last that is not JSON, and a line that lies about its score.
    let text = fs::read_to_string(&results).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let garbled = dir.join("garbled.jsonl");
    fs::write(&garbled, [&lines[..4], &["not json"], &lines[5..]].concat().join("\n") + "\n").unwrap();
    let lied = dir.join("lied.jsonl");
    fs::write(&lied, text.replacen("\"score\":40,", "\"score\":41,", 1)).unwrap();
    for (bad, message) in [(&garbled, "garbled.jsonl: line 5"), (&lied, "lied.jsonl: line 10 (n=10)")] {
        let run = taxman(&["extend-optimal", path(bad), "--out", path(&out)]);
        assert_eq!(run.code, Some(1), "{}", run.stdout);
        assert!(run.stderr.contains(message), "{message}: {}", run.stderr);
        assert!(!out.exists());
    }

    // A file that is not there, and a table that is not.
    let run = taxman(&["extend-optimal", path(&dir.join("missing.jsonl")), "--out", path(&out)]);
    assert!(run.code == Some(1) && run.stderr.contains("missing.jsonl"), "{}", run.stderr);
    let run = taxman(&["extend-optimal", &results, "--table", path(&dir.join("missing.json")), "--out", path(&out)]);
    assert!(run.code == Some(1) && run.stderr.contains("missing.json"), "{}", run.stderr);
    assert!(!out.exists());
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

    // A search stopped by a limit keeps its checkpoint, says so, and the same command finishes it.  (The engine
    // checks the limit every 1,024 steps of a searcher; on its own, the default searches game 1000 in about 5,000.)
    let ckpt = dir.join("ckpt");
    let limited =
        ["solve", "1000", "--force-search", "--threads", "1", "--progress", "0", "--checkpoint-dir", path(&ckpt)];
    let run = taxman(&[&limited[..], &["--node-limit", "1"]].concat());
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("stopped: the search for n=1000 hit a limit"), "{}", run.stdout);
    assert!(run.stdout.contains("run the same command to resume it"), "{}", run.stdout);
    assert!(ckpt.join("n1000.ckpt").exists());
    let run = taxman(&limited);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("optimal ok") && !run.stdout.contains("stopped"), "{}", run.stdout);
    assert!(!ckpt.join("n1000.ckpt").exists(), "a finished game removes its checkpoint");
}

/// A results file that another process has open (here this one, appending to it as a run does) is refused by
/// `solve`, which appends nothing; once it is closed, `solve` goes on with it.
#[test]
fn solve_refuses_a_results_file_another_process_has_open() {
    let dir = TempDir::new("solve-lock");
    let results = dir.join("results.jsonl");
    let args = ["solve", "5-9", "--threads", "1", "--progress", "0", "--out", path(&results)];
    let held = taxman::results::ResultsFile::open(&results, Optimal::built_in()).unwrap();
    let run = taxman(&args);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert!(run.stderr.contains("results.jsonl is in use by another process"), "{}", run.stderr);
    assert_eq!(fs::read_to_string(&results).unwrap(), "");
    drop(held);
    let run = taxman(&args);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(fs::read_to_string(&results).unwrap().lines().count(), 5);
}

#[test]
fn regress_checks_scores_against_optimal_json() {
    let run = taxman(&["regress", "--to", "40", "--threads", "1"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("solved 1..=40 (1 threads, tactics default)"), "{}", run.stdout);
    assert!(run.stdout.contains("40 games match optimal.json, 0 are not in it"), "{}", run.stdout);

    let run = taxman(&["regress", "--from", "30", "--to", "34", "--threads", "1", "--tactic", "match_refine=0"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("solved 30..=34 (1 threads, tactics match_refine=0)"), "{}", run.stdout);
    assert!(run.stdout.contains("by method: trivial "), "{}", run.stdout);
}

#[test]
fn a_game_with_no_previous_game_says_it_is_solved_standalone() {
    let standalone = |args: &[&str]| taxman(&[&["solve", "--threads", "1", "--progress", "0"][..], args].concat());
    let run = standalone(&["40"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    for text in ["note: no previous game for n=40", "standalone", "--use-prev to take optimal.json's game of 39"] {
        assert!(run.stderr.contains(text), "{text}: {}", run.stderr);
    }
    assert!(run.stdout.contains("n=40") && !run.stdout.contains("assumes"), "{}", run.stdout);
    // With a previous game (optimal.json's 39, which the game then assumes optimal), or for n = 1, which has none to
    // have, there is nothing to say.
    for args in [&["40", "--use-prev"][..], &["1"], &["1-5"]] {
        let run = standalone(args);
        assert_eq!(run.code, Some(0), "{args:?}: {}", run.stderr);
        assert_eq!(run.stderr, "", "{args:?}");
    }
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
    assert_eq!(games[0].get("score").and_then(Json::as_u64), Optimal::built_in().score(480));
    assert_eq!(games[0].get("optimal_match").and_then(Json::as_bool), Some(true));
    assert_eq!(record.get("threads").and_then(Json::as_u32), Some(1));
    assert!(games[0].get("nodes").and_then(Json::as_u64).is_some_and(|nodes| nodes > 0));
    assert_eq!(fs::read(&tracked).ok(), before, "a test must not touch the tracked benchmark history");
}
