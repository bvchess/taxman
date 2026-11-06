//! The command line: every subcommand's happy path and its usage errors, through the real binary.

mod common;

use common::{path, taxman, TempDir};
use std::fs;
use std::path::Path;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::search::MAX_GATE;

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
            "verify-results [--threads K] FILE...",
            "--finder-after NODES",
            "--finder-seconds S",
            "--gate GAP",
            "--gate-trial GATES",
            "--gate-trial-plan SPEC",
            "tables N K...",
        ] {
            assert!(run.stdout.contains(text), "{args:?}: no {text:?}");
        }
        for name in taxman::tactics::Tactics::names() {
            assert!(run.stdout.contains(name), "{args:?}: the help does not name the tactic {name}");
        }
        // The results are checked by `verify-results`, which reads the committed file; no command writes the table.
        for gone in
            ["verify-optimal", "extend-optimal", "strict.jsonl", "certify", "verify-certificates", "--no-claims"]
        {
            assert!(!run.stdout.contains(gone), "{args:?}: the help still names {gone}");
        }
    }
}

#[test]
fn bad_command_lines_are_usage_errors() {
    assert_usage_error(&[], "no command given");
    assert_usage_error(&["frobnicate"], "unknown command");
    assert_usage_error(&["solve"], "usage: solve N|A-B");
    assert_usage_error(&["solve", "5-3"], "empty");
    assert_usage_error(&["solve", "x"], "expected a game N or a range A-B");
    assert_usage_error(&["solve", "12", "--tactic", "nogoods=maybe"], "expected on or off");
    assert_usage_error(&["solve", "12", "--tactic", "fix_forced"], "expected name=value");
    assert_usage_error(&["solve", "12", "--tactic", "sparkle=on"], "unknown tactic");
    assert_usage_error(&["solve", "12", "--threads", "0"], "--threads must be at least 1");
    assert_usage_error(&["solve", "12", "--window", "0"], "--window must be at least 1");
    assert_usage_error(&["solve", "12", "--window", "many"], "--window: cannot read");
    assert_usage_error(&["regress", "--window", "0"], "--window must be at least 1");
    assert_usage_error(&["solve", "12", "--nope"], "unknown option --nope");
    assert_usage_error(&["solve", "12", "--time-limit", "-1"], "not a number of seconds");
    assert_usage_error(&["solve", "12", "--node-limit", "many"], "cannot read");
    for removed in ["--keep-going", "--retry", "--retry-improved", "--pass", "--game-hours", "--use-prev"] {
        assert_usage_error(&["solve", "12", removed], &format!("unknown option {removed}"));
    }
    assert_usage_error(&["regress", "--use-prev"], "unknown option --use-prev");
    assert_usage_error(&["solve", "12", "--finder-after", "soon"], "--finder-after: cannot read");
    assert_usage_error(&["solve", "12", "--finder-after", "-1"], "--finder-after: cannot read");
    assert_usage_error(&["solve", "12", "--finder-seconds", "0"], "--finder-seconds must be more than 0 seconds");
    assert_usage_error(&["regress", "--finder-after", "1"], "unknown option --finder-after");
    assert_usage_error(&["regress", "12"], "regress takes no positional arguments");
    assert_usage_error(&["tables"], "usage: tables N K...");
    assert_usage_error(&["tables", "30"], "usage: tables N K...");
    assert_usage_error(&["tables", "30", "31"], "not on the board");
    assert_usage_error(&["tables", "x", "5"], "N must be");
    assert_usage_error(&["verify-optimal"], "unknown command");
    assert_usage_error(&["extend-optimal"], "unknown command");
    assert_usage_error(&["certify"], "unknown command");
    assert_usage_error(&["verify-certificates"], "unknown command");
    assert_usage_error(&["verify-results"], "usage: verify-results [--threads K] FILE...");
    assert_usage_error(&["verify-results", "--threads", "0", "a"], "--threads must be at least 1");
    assert_usage_error(&["verify-results", "--tactic", "nogoods=on", "a"], "unknown option --tactic");
}

/// `verify-results` reports the same on one thread and on several (the games of 22, 34 and 38 are repeats, whose
/// checks set up search problems).
#[test]
fn verify_results_reports_the_same_on_any_number_of_threads() {
    let dir = TempDir::new("verify-results");
    let results = solve_into(&dir, "results.jsonl", "1-40");
    // The report, without the line that says how long the check took.
    let report = |threads: &str| {
        let run = taxman(&["verify-results", "--threads", threads, &results]);
        assert_eq!(run.code, Some(0), "{}", run.stderr);
        run.stdout.lines().filter(|line| !line.starts_with("checked in")).collect::<Vec<_>>().join("\n")
    };
    let one = report("1");
    assert!(one.contains("40 completed games"), "{one}");
    assert!(one.contains("same_search"), "{one}");
    assert_eq!(report("2"), one);
    assert_eq!(report("4"), one);
    // Without the flag, it takes the CPUs.
    let run = taxman(&["verify-results", &results]);
    assert!(run.code == Some(0) && run.stdout.contains("checked in"), "{}{}", run.stdout, run.stderr);
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

/// Solves `range` into the results file `name` in `dir` (every game chained from the one before, or standalone)
/// and returns the file's path.
fn solve_into(dir: &Path, name: &str, range: &str) -> String {
    let file = dir.join(name);
    let args = ["solve", range, "--threads", "1", "--progress", "0", "--out", path(&file)];
    let run = taxman(&args);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    path(&file).to_owned()
}

/// A results file whose name ends in `.gz` is read through the gzip decoder, every member of it, as the committed results
/// are; a member that fails the decoder's check refuses the file, naming it and the check.  What a run appends to is
/// plain JSONL, so `solve` refuses a name that says gzip.
#[test]
fn verify_results_reads_gzip_files_and_solve_writes_plain_ones() {
    let dir = TempDir::new("verify-gz");
    let results = solve_into(&dir, "results.jsonl", "1-30");
    let text = fs::read_to_string(&results).unwrap();
    // Two members, as the committed file is several.
    let cut = text.match_indices('\n').nth(19).unwrap().0 + 1;
    let zipped = dir.join("results.jsonl.gz");
    fs::write(&zipped, common::gzip_stored(&[&text[..cut], &text[cut..]])).unwrap();
    let report = |file: &str| {
        let run = taxman(&["verify-results", "--threads", "1", file]);
        assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
        run.stdout.lines().filter(|line| !line.starts_with("checked in")).collect::<Vec<_>>().join("\n")
    };
    // The same report as the plain file's, but for the name of the file.
    let (plain, zipped_report) = (report(&results), report(path(&zipped)));
    assert!(plain.contains("30 completed games"), "{plain}");
    assert_eq!(zipped_report.replace("results.jsonl.gz", "results.jsonl"), plain);

    let mut bytes = fs::read(&zipped).unwrap();
    let crc = bytes.len() - 8;
    bytes[crc] ^= 1;
    let damaged = dir.join("damaged.jsonl.gz");
    fs::write(&damaged, bytes).unwrap();
    let run = taxman(&["verify-results", path(&damaged)]);
    assert_eq!(run.code, Some(1), "{}", run.stdout);
    assert!(run.stderr.contains("damaged.jsonl.gz") && run.stderr.contains("CRC-32"), "{}", run.stderr);

    let out = dir.join("out.jsonl.gz");
    let run = taxman(&["solve", "5", "--threads", "1", "--progress", "0", "--out", path(&out)]);
    assert_eq!(run.code, Some(1), "{}", run.stdout);
    assert!(run.stderr.contains("out.jsonl.gz") && run.stderr.contains("plain JSONL"), "{}", run.stderr);
    assert!(!out.exists(), "nothing is created under a gzip name");
}

/// `verify-results` refuses a file that is not what it says, naming the file and the line: a line before the last that
/// is not JSON, and a line that lies about its score.
#[test]
fn verify_results_refuses_lines_that_are_not_what_they_say() {
    let dir = TempDir::new("verify-refuses");
    let results = solve_into(&dir, "r.jsonl", "1-20");
    let text = fs::read_to_string(&results).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let garbled = dir.join("garbled.jsonl");
    fs::write(&garbled, [&lines[..4], &["not json"], &lines[5..]].concat().join("\n") + "\n").unwrap();
    let lied = dir.join("lied.jsonl");
    fs::write(&lied, text.replacen("\"score\":40,", "\"score\":41,", 1)).unwrap();
    for (bad, message) in [(&garbled, "garbled.jsonl: line 5"), (&lied, "lied.jsonl: line 10 (n=10)")] {
        let run = taxman(&["verify-results", "--threads", "1", path(bad)]);
        assert_eq!(run.code, Some(1), "{}", run.stdout);
        assert!(run.stderr.contains(message), "{message}: {}", run.stderr);
    }
    let run = taxman(&["verify-results", path(&dir.join("missing.jsonl.gz"))]);
    assert!(run.code == Some(1) && run.stderr.contains("missing.jsonl.gz"), "{}", run.stderr);
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

/// A search prints a progress line at its end (`--progress` asks for more while it runs), which starts with the UTC
/// time of day, the game, and what the game has spent over all its invocations: the time it was active, the time it
/// waited and the CPU time of its workers; the summary line on stdout ends with the active and CPU seconds.
#[test]
fn the_progress_line_has_the_time_of_day_and_what_the_game_has_spent() {
    let run = taxman(&["solve", "1000", "--force-search", "--threads", "2", "--progress", "60"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let line = run.stderr.lines().find(|line| line.contains(" nodes ")).unwrap_or_else(|| panic!("{}", run.stderr));
    let words: Vec<&str> = line.split(' ').collect();
    // 12:34:56Z n=1000 active 0.3s waited 0.0s cpu 0.5s nodes 2708 (done) best 24175 queue 0 spilled 0 | eff ...
    let time_of_day = |word: &str| {
        let digits: Vec<&str> = word.strip_suffix('Z').unwrap_or("").split(':').collect();
        digits.len() == 3 && digits.iter().all(|part| part.len() == 2 && part.bytes().all(|b| b.is_ascii_digit()))
    };
    assert!(time_of_day(words[0]), "{line}");
    let spent = |word: &str| word.ends_with('s') || word.ends_with('h');
    assert_eq!((words[1], words[2], words[4], words[6], words[8]), ("n=1000", "active", "waited", "cpu", "nodes"));
    assert!(spent(words[3]) && spent(words[5]) && spent(words[7]) && words[10] == "(done)", "{line}");
    let summary = run.stdout.lines().find(|line| line.starts_with("n=1000 ")).expect("the summary line");
    let seconds = |key: &str| -> f64 {
        let word = summary.split_whitespace().find_map(|word| word.strip_prefix(key)).expect(key);
        word.parse().unwrap_or_else(|e| panic!("{key} in {summary}: {e}"))
    };
    assert!(seconds("active=") > 0.0 && seconds("cpu=") > 0.0, "{summary}");
}

/// `--background` is a switch of `solve`, which changes no score, which a line records in its `options` where it was
/// given, and which says on stderr, once, that it does nothing where there is no idle policy.
#[test]
fn background_is_a_switch_that_is_recorded_and_changes_no_score() {
    assert_usage_error(&["solve", "12", "--background=1"], "--background takes no value");

    let dir = TempDir::new("solve-background");
    // A game of 125 nodes, searched by three workers.
    let line = |name: &str, extra: &[&str]| {
        let file = dir.join(name);
        let args = ["solve", "900", "--force-search", "--threads", "3", "--progress", "0", "--out", path(&file)];
        let run = taxman(&[&args[..], extra].concat());
        assert_eq!(run.code, Some(0), "{}", run.stderr);
        let text = fs::read_to_string(&file).unwrap();
        (Json::parse(text.trim()).unwrap(), run.stderr)
    };
    let (plain, said) = line("plain.jsonl", &[]);
    let (background, note) = line("background.jsonl", &["--background"]);
    assert_eq!(plain.get("score"), background.get("score"));
    assert_eq!(plain.get("options").unwrap().get("background"), None, "a line without the flag has no record of it");
    assert_eq!(background.get("options").unwrap().get("background"), Some(&Json::Bool(true)));
    assert!(!said.contains("--background"), "{said}");
    // Where there is no idle policy, each of the three workers would say so; the run says it once.
    if cfg!(not(target_os = "linux")) {
        assert_eq!(note.matches("--background does nothing on this system").count(), 1, "{note}");
    }
}

/// `--gate GAP` is a flag of `solve`: a whole number from 0 to `MAX_GATE`, refused with conflict branching off, where a
/// gate does nothing.  A line records it in `options.gate` where it was given, beside the gates of the trial
/// (`options.gate_trial`: none by default, as with `--gate-trial none`; the gates `--gate-trial` lists, and 1000, 800
/// and 600 where only `--gate-trial-plan` asks for a trial), and the game ends with the score of the table.
#[test]
fn gate_is_a_flag_of_solve_that_is_recorded_and_changes_no_score() {
    let over = (MAX_GATE + 1).to_string();
    assert_usage_error(&["solve", "12", "--gate", &over], &format!("--gate takes a gap from 0 to {MAX_GATE}: {over}"));
    assert_usage_error(&["solve", "12", "--gate", "-1"], "--gate: cannot read");
    assert_usage_error(&["solve", "12", "--gate", "wide"], "--gate: cannot read");
    let off = |tactic: &'static str| ["solve", "12", "--gate", "1200", "--tactic", tactic];
    assert_usage_error(&off("conflict_branching=off"), "conflict branching is off");
    assert_usage_error(&off("conflict_branching=off"), "a gate without conflict branching does nothing");
    assert_usage_error(&["regress", "--gate", "1200"], "unknown option --gate");
    // `none` is no trial, so it has no plan to change; the plan of a trial takes only its own settings; the gates are
    // numbers.
    assert_usage_error(&["solve", "12", "--gate-trial", "none", "--gate-trial-plan", "after=1"], "no trial");
    assert_usage_error(&["solve", "12", "--gate-trial-plan", "gate=600"], "--gate-trial-plan: gate: not a setting");
    assert_usage_error(&["solve", "12", "--gate-trial", "600,none"], "--gate-trial takes gates from 0 to");

    let dir = TempDir::new("solve-gate");
    // A game of 125 nodes, searched by three workers.
    let line = |name: &str, extra: &[&str]| {
        let file = dir.join(name);
        let args = ["solve", "900", "--force-search", "--threads", "3", "--progress", "0", "--out", path(&file)];
        let run = taxman(&[&args[..], extra].concat());
        assert_eq!(run.code, Some(0), "{}", run.stderr);
        let text = fs::read_to_string(&file).unwrap();
        Json::parse(text.trim()).unwrap()
    };
    let plain = line("plain.jsonl", &[]);
    let options = |line: &Json| line.get("options").unwrap().clone();
    assert_eq!(options(&plain).get("gate"), None, "a line without the flag has no record of it");
    let trial = |line: &Json| options(line).get("gate_trial").map(Json::to_string);
    assert_eq!(trial(&plain), None, "no trial by default");
    assert_eq!(plain.get("score").and_then(Json::as_u64), Optimal::built_in().score(900));
    // The gate at its least with no trial, and at its most with one: the trial's gates are recorded beside it.  A plan
    // alone asks for the trial of the default gates.
    let zero = line("zero.jsonl", &["--gate", "0", "--gate-trial", "none"]);
    let widest = line("widest.jsonl", &["--gate", &MAX_GATE.to_string(), "--gate-trial", "0,600"]);
    let planned = line("planned.jsonl", &["--gate-trial-plan", "units=3"]);
    assert_eq!(trial(&planned).as_deref(), Some("[1000,800,600]"), "a plan alone asks for the default trial");
    for (name, gated) in [("zero", &zero), ("widest", &widest), ("planned", &planned)] {
        assert_eq!(gated.get("score"), plain.get("score"), "{name}");
        assert_eq!(gated.get("optimal_match"), Some(&Json::Bool(true)), "{name}");
    }
    assert_eq!(options(&zero).get("gate").and_then(Json::as_u64), Some(0));
    assert_eq!(trial(&zero), None, "--gate-trial none records no trial");
    assert_eq!(options(&widest).get("gate").and_then(Json::as_u64), Some(MAX_GATE));
    assert_eq!(trial(&widest).as_deref(), Some("[0,600]"));
}

/// A results file that another process has open (here this one, appending to it as a run does) is refused by
/// `solve`, which appends nothing; once it is closed, `solve` goes on with it.
#[test]
fn solve_refuses_a_results_file_another_process_has_open() {
    let dir = TempDir::new("solve-lock");
    let results = dir.join("results.jsonl");
    let args = ["solve", "5-9", "--threads", "1", "--progress", "0", "--out", path(&results)];
    let held = taxman::results::ResultsFile::open(&results, Optimal::built_in(), 1).unwrap();
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
fn regress_checks_scores_against_the_table() {
    let run = taxman(&["regress", "--to", "40", "--threads", "1"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("solved 1..=40 (1 threads, tactics default)"), "{}", run.stdout);
    assert!(run.stdout.contains("40 games match the table, 0 are not in it"), "{}", run.stdout);

    // Games in flight, sharing two threads.
    let run = taxman(&["regress", "--to", "40", "--threads", "2", "--window", "4"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("40 games match the table, 0 are not in it"), "{}", run.stdout);

    let run = taxman(&["regress", "--from", "30", "--to", "34", "--threads", "1", "--tactic", "nogoods=off"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("solved 30..=34 (1 threads, tactics nogoods=off)"), "{}", run.stdout);
    assert!(run.stdout.contains("by method: trivial "), "{}", run.stdout);
}

#[test]
fn a_game_with_no_previous_game_says_it_is_solved_standalone() {
    let standalone = |args: &[&str]| taxman(&[&["solve", "--threads", "1", "--progress", "0"][..], args].concat());
    let run = standalone(&["40"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    for text in ["note: no previous game for n=40", "standalone", "give --out with the earlier games"] {
        assert!(run.stderr.contains(text), "{text}: {}", run.stderr);
    }
    assert!(run.stdout.contains("n=40") && !run.stdout.contains("assumes"), "{}", run.stdout);
    // For n = 1, which has no previous game to have, there is nothing to say.
    for args in [&["1"][..], &["1-5"]] {
        let run = standalone(args);
        assert_eq!(run.code, Some(0), "{args:?}: {}", run.stderr);
        assert_eq!(run.stderr, "", "{args:?}");
    }
}
