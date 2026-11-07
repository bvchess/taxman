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
            "--unsound",
            "--effort E",
            "--seed S",
            "--fix LO,HI",
            "--tilt P",
            "--back J",
            "--widen S",
            "--past-ceiling",
            "--stop-at-known",
            "--from-known",
            "tables N K...",
        ] {
            assert!(run.stdout.contains(text), "{args:?}: no {text:?}");
        }
        assert!(!run.stdout.contains("--survivors"), "{args:?}: --back replaced --survivors");
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

/// `solve --unsound` appends unsound lines to its file and skips the games the file has; it needs `--effort` and
/// `--out`, and refuses every flag of the proof search, while the flags of the unsound mode are refused without it.
/// Neither `solve` without it nor `verify-results` takes its file, and it takes no file of results lines.
#[test]
fn the_unsound_mode_keeps_its_lines_apart_and_refuses_the_flags_of_the_proof_search() {
    let dir = TempDir::new("unsound");
    let file = dir.join("unsound.jsonl");
    let unsound =
        |range: &str| taxman(&["solve", range, "--unsound", "--effort", "0", "--threads", "1", "--out", path(&file)]);
    let run = unsound("1-30");
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout.lines().filter(|line| line.contains("  unsound")).count(), 30, "{}", run.stdout);
    let text = fs::read_to_string(&file).unwrap();
    assert_eq!(text.lines().count(), 30);
    assert!(text.lines().all(|line| Json::parse(line).unwrap().get("unsound") == Some(&Json::Bool(true))));
    let run = unsound("1-32");
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("n=32 ") && run.stdout.contains("skipped 30 games already done"), "{}", run.stdout);

    // The proven mode and the verifier refuse the file, and the unsound mode refuses a file of results lines.
    let solve = ["solve", "33", "--threads", "1", "--progress", "0", "--out", path(&file)];
    for args in [&solve[..], &["verify-results", path(&file)]] {
        let run = taxman(args);
        assert_eq!(run.code, Some(1), "{args:?}: {}", run.stdout);
        assert!(run.stderr.contains("line 1: an unsound line"), "{args:?}: {}", run.stderr);
    }
    let proven = solve_into(&dir, "proven.jsonl", "1-5");
    let run = taxman(&["solve", "6", "--unsound", "--effort", "0", "--threads", "1", "--out", &proven]);
    assert_eq!(run.code, Some(1), "{}", run.stdout);
    assert!(run.stderr.contains("line 1: not an unsound line"), "{}", run.stderr);
    // The measuring options take the games of the built-in table only.
    let past = (Optimal::built_in().max_n() + 1).to_string();
    let elsewhere = dir.join("past.jsonl");
    let run = taxman(&["solve", &past, "--unsound", "--effort", "0", "--stop-at-known", "--out", path(&elsewhere)]);
    assert_eq!(run.code, Some(1), "{}", run.stdout);
    assert!(run.stderr.contains(&format!("lacks game {past}")), "{}", run.stderr);

    let base = ["solve", "12", "--unsound", "--effort", "1", "--out", path(&elsewhere)];
    assert_usage_error(&base[..3], "--unsound needs --effort E");
    assert_usage_error(&base[..5], "--unsound needs --out FILE");
    assert_usage_error(
        &["solve", "12", "--unsound", "--effort", "33"],
        "--effort 33: the level of effort, from 0 to 32",
    );
    let proof_search: [&[&str]; 17] = [
        &["--window", "2"],
        &["--finder-after", "1"],
        &["--finder-seconds", "1"],
        &["--profile", "p"],
        &["--profile-depth", "3"],
        &["--carried-dir", "c"],
        &["--gate", "300"],
        &["--gate-trial", "none"],
        &["--gate-trial-plan", "after=1"],
        &["--checkpoint-dir", "c"],
        &["--checkpoint-interval", "1"],
        &["--progress", "0"],
        &["--time-limit", "1"],
        &["--node-limit", "1"],
        &["--mem-cap", "1"],
        &["--background"],
        &["--force-search"],
    ];
    for flag in proof_search {
        let message = format!("{} is a flag of the proof search, which --unsound does not run", flag[0]);
        assert_usage_error(&[&base[..], flag].concat(), &message);
    }
    let unsound_only: [&[&str]; 9] = [
        &["--effort", "1"],
        &["--seed", "2"],
        &["--fix", "0.5,0.7"],
        &["--tilt", "50"],
        &["--back", "2"],
        &["--widen", "3"],
        &["--past-ceiling"],
        &["--stop-at-known"],
        &["--from-known"],
    ];
    for flag in unsound_only {
        let message = format!("{} is a flag of the unsound mode: give --unsound with it", flag[0]);
        assert_usage_error(&[&["solve", "12"][..], flag].concat(), &message);
    }
    // `--back 1` is what `--survivors` was, and the switch is gone.
    assert_usage_error(&[&base[..], &["--survivors"]].concat(), "unknown option --survivors");
}

/// `--fix LO,HI` takes two numbers with 0 <= LO <= HI <= 1, `--tilt P` a whole number from 0 to 100, `--back J` a whole
/// number from 0 to 8 and `--widen S` a whole number from 1, `--widen` is refused with `--fix`, and the line records the
/// options.
#[test]
fn the_options_that_shape_the_rounds_are_checked_and_the_line_records_them() {
    let dir = TempDir::new("unsound-fix");
    let file = dir.join("fix.jsonl");
    let base = ["solve", "12", "--unsound", "--effort", "0", "--threads", "1", "--out", path(&file)];
    for fix in ["0.9,0.7", "-0.1,0.5", "0.5,1.1", "0.5", "0.5,", "a,b", "0.5,0.7,0.9", "nan,0.5", "0.5,inf", ""] {
        let message = format!("--fix takes LO,HI, two numbers with 0 <= LO <= HI <= 1, such as 0.7,0.9: got {fix:?}");
        assert_usage_error(&[&base[..], &["--fix", fix]].concat(), &message);
    }
    for tilt in ["101", "-1", "", "a", "50.5", "99999999999"] {
        let message = format!("--tilt takes P, a whole number of percent from 0 to 100: got {tilt:?}");
        assert_usage_error(&[&base[..], &["--tilt", tilt]].concat(), &message);
    }
    // The switch it replaced is gone: a `--tilt` with no value is a missing value, like any other.
    assert_usage_error(&[&base[..], &["--tilt"]].concat(), "--tilt needs a value");
    for back in ["9", "-1", "", "a", "1.5", "99999999999"] {
        let message = format!("--back takes J, a whole number from 0 to 8: got {back:?}");
        assert_usage_error(&[&base[..], &["--back", back]].concat(), &message);
    }
    for widen in ["0", "-1", "", "a", "2.5"] {
        let message = format!("--widen takes S, a whole number of rounds, at least 1: got {widen:?}");
        assert_usage_error(&[&base[..], &["--widen", widen]].concat(), &message);
    }
    // The widening has a ladder of its own for the share fixed, whatever the value of the share.
    for fix in ["0.7,0.9", "0.4,0.6"] {
        let message = "--widen takes the share fixed from its own ladder, so --fix is refused with it";
        assert_usage_error(&[&base[..], &["--widen", "2", "--fix", fix]].concat(), message);
    }
    assert!(!file.exists(), "a refused run plays no game");
    let options = ["--fix", "0.4,0.6", "--tilt", "30", "--back", "1"];
    let run = taxman(&[&base[..], &options].concat());
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let line = Json::parse(fs::read_to_string(&file).unwrap().lines().next().unwrap()).unwrap();
    assert_eq!(line.get("fix"), Some(&Json::array([0.4, 0.6])));
    let (tilt, back, widen) = (line.get("tilt"), line.get("back"), line.get("widen"));
    assert_eq!((tilt, back, widen), (Some(&Json::from(30u64)), Some(&Json::from(1u64)), Some(&Json::Null)));
    assert!(line.get("survivors").is_none(), "--back replaced --survivors");
    // The widening alone, and without the options, the line records the defaults.
    let widened = dir.join("widen.jsonl");
    let run = taxman(&["solve", "12", "--unsound", "--effort", "0", "--out", path(&widened), "--widen", "3"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let line = Json::parse(fs::read_to_string(&widened).unwrap().lines().next().unwrap()).unwrap();
    assert_eq!((line.get("widen"), line.get("fix")), (Some(&Json::from(3u64)), Some(&Json::array([0.7, 0.9]))));
    let other = dir.join("default.jsonl");
    let run = taxman(&["solve", "12", "--unsound", "--effort", "0", "--threads", "1", "--out", path(&other)]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let line = Json::parse(fs::read_to_string(&other).unwrap().lines().next().unwrap()).unwrap();
    assert_eq!(line.get("fix"), Some(&Json::array([0.7, 0.9])));
    assert_eq!(
        (line.get("tilt"), line.get("back"), line.get("widen")),
        (Some(&Json::from(0u64)), Some(&Json::from(0u64)), Some(&Json::Null))
    );
    assert_eq!(line.get("budget"), Some(&Json::from(0u64)), "effort 0 budgets no steps");

    // The three options together, with rounds: game 375 from the table's moves has two trajectories, the start's and
    // one from the moves for 367, and reaches the table's score in the first round, which ends the rounds of a budget
    // of 65,536 steps.
    let rounds = dir.join("rounds.jsonl");
    let args = ["solve", "375", "--unsound", "--effort", "1", "--from-known", "--stop-at-known", "--threads", "1"];
    let options = ["--tilt", "100", "--back", "8", "--widen", "2", "--out", path(&rounds)];
    let run = taxman(&[&args[..], &options].concat());
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let line = Json::parse(fs::read_to_string(&rounds).unwrap().lines().next().unwrap()).unwrap();
    assert_eq!(line.get("budget"), Some(&Json::from(65_536u64)));
    assert_eq!(line.get("trajectories"), Some(&Json::array([Json::from("start"), Json::from(367u64)])));
    assert_eq!(line.get("stage_rounds"), Some(&Json::array([Json::array([1u64, 0, 0]), Json::array([1u64, 0, 0])])));
    assert_eq!(line.get("rounds"), Some(&Json::from(1u64)));
    // The work of a round is its steps and 32 for each of its 64 neighborhoods.
    let (steps, work) = (line.get("steps").and_then(Json::as_u64), line.get("work").and_then(Json::as_u64));
    assert_eq!(work, steps.map(|steps| steps + 64 * 32));
    let improvements = line.get("improvements").and_then(Json::as_arr).expect("the improvements");
    assert_eq!(improvements.len(), 1);
    assert_eq!(improvements[0].get("round"), Some(&Json::from(1u64)));
    assert_eq!(
        (improvements[0].get("trajectory"), improvements[0].get("stage")),
        (Some(&Json::from(0u64)), Some(&Json::from(0u64)))
    );
    let known = line.get("stop_at_known").expect("what --stop-at-known recorded");
    assert_eq!(known.get("round"), Some(&Json::from(1u64)));
}

/// A searched game stops at the run's ceiling unless `--past-ceiling` is given, and the line says what the ceiling was
/// and whether the game is at it: game 18 from the table's moves starts at its ceiling, 18 + the table's score for 17.
#[test]
fn past_ceiling_takes_the_stop_at_the_runs_ceiling_away() {
    let dir = TempDir::new("unsound-ceiling");
    let ceiling = Optimal::built_in().score(17).unwrap() + 18;
    let play = |name: &str, extra: &[&str]| {
        let file = dir.join(name);
        let args =
            ["solve", "18", "--unsound", "--effort", "1", "--from-known", "--threads", "1", "--out", path(&file)];
        let run = taxman(&[&args[..], extra].concat());
        assert_eq!(run.code, Some(0), "{}", run.stderr);
        let line = fs::read_to_string(&file).unwrap();
        (run.stdout, Json::parse(line.lines().next().unwrap()).unwrap())
    };
    let (summary, stopped) = play("stop.jsonl", &[]);
    assert!(summary.contains("  at ceiling"), "{summary}");
    assert_eq!(stopped.get("ceiling"), Some(&Json::from(ceiling)));
    assert_eq!(stopped.get("at_ceiling"), Some(&Json::Bool(true)));
    assert_eq!((stopped.get("rounds"), stopped.get("work")), (Some(&Json::from(0u64)), Some(&Json::from(0u64))));
    let (summary, past) = play("past.jsonl", &["--past-ceiling"]);
    assert!(!summary.contains("at ceiling"), "{summary}");
    assert_eq!((past.get("ceiling"), past.get("at_ceiling")), (Some(&Json::Null), Some(&Json::Bool(false))));
    assert!(past.get("rounds").and_then(Json::as_u64) >= Some(1), "the game searches its budget: {past:?}");
    assert_eq!(past.get("score"), stopped.get("score"));
}
