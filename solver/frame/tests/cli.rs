//! The command line: every subcommand's happy path and its usage errors, through the real binary.

mod common;

use common::{path, taxman, TempDir};
use std::fs;
use std::path::Path;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::search::MAX_GATE;
use taxman::solver::unsound;

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
            "--effort L",
            "--seed S",
            "--above J",
            "--stop-growth T|off",
            "--best-known FILE|off",
            "--known-to N",
            "tables N K...",
        ] {
            assert!(run.stdout.contains(text), "{args:?}: no {text:?}");
        }
        // The options that the mode no longer takes are not in the help.
        for gone in [
            "--survivors",
            "--fix LO,HI",
            "--tilt P",
            "--back J",
            "--widen S",
            "--unseeded",
            "--stop-quiet",
            "--push-growth",
            "--past-ceiling",
            "--no-repair",
            "--stop-at-known",
            "--from-known",
        ] {
            assert!(!run.stdout.contains(gone), "{args:?}: the help still names {gone}");
        }
        // The help says what the defaults are.
        let help = run.stdout.split_whitespace().collect::<Vec<_>>().join(" ");
        for text in [
            "(J from 0 to 8, default 0: none)",
            "and the last 4 rounds raised nothing, from the end of the rounds of level 1 on (default 25)",
            "(default best-known.jsonl in the directory of --out; off: none)",
        ] {
            assert!(help.contains(text), "{args:?}: no {text:?}");
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

/// `solve --unsound` appends unsound lines to its file and skips the games the file has; it needs `--effort` and
/// `--out`, and refuses every flag of the proof search, while the flags of the unsound mode are refused without it.
/// Neither `solve` without it nor `verify-results` takes its file, and it takes no file of results lines.
#[test]
fn the_unsound_mode_keeps_its_lines_apart_and_refuses_the_flags_of_the_proof_search() {
    let dir = TempDir::new("unsound");
    let file = dir.join("unsound.jsonl");
    // These small games run no round: each is not searched, or starts at its ceiling.
    let unsound = |range: &str| {
        let args = ["solve", range, "--unsound", "--effort", "1", "--threads", "1"];
        taxman(&[&args[..], &["--out", path(&file)]].concat())
    };
    let run = unsound("1-17");
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.stdout.lines().filter(|line| line.contains("  unsound")).count(), 17, "{}", run.stdout);
    let text = fs::read_to_string(&file).unwrap();
    assert_eq!(text.lines().count(), 17);
    assert!(text.lines().all(|line| Json::parse(line).unwrap().get("unsound") == Some(&Json::Bool(true))));
    let run = unsound("1-19");
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("n=19 ") && run.stdout.contains("skipped 17 games already done"), "{}", run.stdout);

    // The proven mode and the verifier refuse the file, and the unsound mode refuses a file of results lines.
    let solve = ["solve", "33", "--threads", "1", "--progress", "0", "--out", path(&file)];
    for args in [&solve[..], &["verify-results", path(&file)]] {
        let run = taxman(args);
        assert_eq!(run.code, Some(1), "{args:?}: {}", run.stdout);
        assert!(run.stderr.contains("line 1: an unsound line"), "{args:?}: {}", run.stderr);
    }
    let proven = solve_into(&dir, "proven.jsonl", "1-5");
    let run = taxman(&["solve", "6", "--unsound", "--effort", "1", "--threads", "1", "--out", &proven]);
    assert_eq!(run.code, Some(1), "{}", run.stdout);
    assert!(run.stderr.contains("line 1: not an unsound line"), "{}", run.stderr);
    let elsewhere = dir.join("past.jsonl");
    let base = ["solve", "12", "--unsound", "--effort", "1", "--out", path(&elsewhere)];
    assert_usage_error(&base[..3], "--unsound needs --effort L, the level of effort, from 1 to 6");
    assert_usage_error(&base[..5], "--unsound needs --out FILE");
    // The levels are 1 to 6: the start alone is no level, and nothing above the most is.
    for effort in ["0", "7", "33", "-1", "", "a", "1.5"] {
        let message = format!("--effort takes L, the level of effort, a whole number from 1 to 6: got {effort:?}");
        assert_usage_error(&["solve", "12", "--unsound", "--effort", effort, "--out", path(&elsewhere)], &message);
    }
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
    let unsound_only: [&[&str]; 6] = [
        &["--effort", "1"],
        &["--seed", "2"],
        &["--above", "2"],
        &["--stop-growth", "0"],
        &["--best-known", "off"],
        &["--known-to", "5"],
    ];
    for flag in unsound_only {
        let message = format!("{} is a flag of the unsound mode: give --unsound with it", flag[0]);
        assert_usage_error(&[&["solve", "12"][..], flag].concat(), &message);
    }
    // The options that the mode builds in are not flags of it, with or without `--unsound`.
    let removed: [&[&str]; 14] = [
        &["--fix", "0.5,0.7"],
        &["--tilt", "50"],
        &["--back", "2"],
        &["--widen", "3"],
        &["--unseeded-every", "8"],
        &["--unseeded-effort", "1"],
        &["--stop-quiet", "0"],
        &["--push-growth", "off"],
        &["--past-ceiling"],
        &["--no-repair"],
        &["--stop-at-known"],
        &["--from-known"],
        &["--unseeded"],
        &["--survivors"],
    ];
    for flag in removed {
        let message = format!("unknown option {}", flag[0]);
        assert_usage_error(&[&base[..], flag].concat(), &message);
        assert_usage_error(&[&["solve", "12"][..], flag].concat(), &message);
    }
}

/// A run keeps a best-known file, `best-known.jsonl` beside its file unless `--best-known FILE` names another or
/// `--best-known off` has none; and the file of the run is not its best-known file.
#[test]
fn a_run_has_a_best_known_file_beside_its_file_unless_told_otherwise() {
    let dir = TempDir::new("unsound-best-known");
    for sub in ["named", "off"] {
        fs::create_dir(dir.join(sub)).unwrap();
    }
    let solve = |name: &str, extra: &[&str]| {
        let out = dir.join(name);
        let base = ["solve", "12", "--unsound", "--effort", "1", "--threads", "1", "--out", path(&out)];
        taxman(&[&base[..], extra].concat())
    };
    // Game 12 is the built-in table's: the file is made and stays empty, and the summary says so.
    let run = solve("by-default.jsonl", &[]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(fs::read_to_string(dir.join("best-known.jsonl")).unwrap(), "");
    let summary = format!("best-known file {}: 0 lines added", dir.join("best-known.jsonl").display());
    assert!(run.stdout.contains(&summary), "{}", run.stdout);
    let elsewhere = dir.join("elsewhere.jsonl");
    let run = solve("named/o.jsonl", &["--best-known", path(&elsewhere)]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(elsewhere.exists() && !dir.join("named/best-known.jsonl").exists());
    let run = solve("off/o.jsonl", &["--best-known", "off"]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(!dir.join("off/best-known.jsonl").exists() && !run.stdout.contains("best-known file"), "{}", run.stdout);
    // The file of the run, however it is named, is not its best-known file.
    let same = dir.join("same.jsonl");
    for best_known in [path(&same), path(&dir.join("named/../same.jsonl"))] {
        let run = solve("same.jsonl", &["--best-known", best_known]);
        assert_eq!(run.code, Some(1), "{}", run.stdout);
        assert!(run.stderr.contains("--out and --best-known are the same file"), "{}", run.stderr);
    }
    let default = |out: &str| unsound::best_known_path(Some(Path::new(out)), None);
    assert_eq!(default("runs/a/o.jsonl"), Some("runs/a/best-known.jsonl".into()));
    assert_eq!(default("o.jsonl"), Some("best-known.jsonl".into()));
    assert_eq!(unsound::best_known_path(Some(Path::new("o.jsonl")), Some("off")), None);
    assert_eq!(unsound::best_known_path(Some(Path::new("o.jsonl")), Some("x/b.jsonl")), Some("x/b.jsonl".into()));
    assert_eq!(unsound::best_known_path(None, None), None);
}

/// `--known-to N` takes the built-in table as holding the games up to N only: the games above N are as if unproven, so
/// the run has a best-known file of them by default as any run of games past the table has, and its lines record N.  It
/// takes a whole number.
#[test]
fn known_to_treats_the_games_above_it_as_unproven() {
    let dir = TempDir::new("unsound-known-to");
    let out = dir.join("o.jsonl");
    let base = ["solve", "11-12", "--unsound", "--effort", "1", "--threads", "1", "--out", path(&out)];
    let run = taxman(&[&base[..], &["--known-to", "10"]].concat());
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("lines added"), "{}", run.stdout);
    for file in ["o.jsonl", "best-known.jsonl"] {
        let text = fs::read_to_string(dir.join(file)).unwrap();
        let lines: Vec<Json> = text.lines().map(|line| Json::parse(line).unwrap()).collect();
        assert!(!lines.is_empty(), "{file}: no lines");
        for line in &lines {
            assert_eq!(line.get("known_to"), Some(&Json::from(10u32)), "{file}");
            assert!(matches!(line.get("n").and_then(Json::as_u32), Some(11 | 12)), "{file}");
        }
    }
    // A run with no flag keeps the key null, and its best-known file is the table's games alone: none.
    let plain = dir.join("plain/o.jsonl");
    fs::create_dir(dir.join("plain")).unwrap();
    let base = ["solve", "11-12", "--unsound", "--effort", "1", "--threads", "1", "--out", path(&plain)];
    let run = taxman(&base);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let text = fs::read_to_string(&plain).unwrap();
    assert!(text.lines().all(|line| Json::parse(line).unwrap().get("known_to") == Some(&Json::Null)));
    assert_eq!(fs::read_to_string(dir.join("plain/best-known.jsonl")).unwrap(), "");
    assert_usage_error(&[&base[..], &["--known-to", "many"]].concat(), "--known-to: cannot read \"many\"");
}

/// `--above J` takes a whole number from 0 to 8, and `--stop-growth T` a whole number of points from 0 up or `off`.
#[test]
fn above_and_stop_growth_are_checked() {
    let dir = TempDir::new("unsound-checked");
    let file = dir.join("checked.jsonl");
    let base = ["solve", "12", "--unsound", "--effort", "1", "--threads", "1", "--out", path(&file)];
    for above in ["9", "-1", "", "a", "1.5", "99999999999"] {
        let message = format!("--above takes J, a whole number from 0 to 8: got {above:?}");
        assert_usage_error(&[&base[..], &["--above", above]].concat(), &message);
    }
    for growth in ["", "a", "1.5", "Off", "-1"] {
        let message = format!("--stop-growth takes T, a whole number of points from 0 up, or off: got {growth:?}");
        assert_usage_error(&[&base[..], &["--stop-growth", growth]].concat(), &message);
    }
    assert!(!file.exists(), "a refused run plays no game");
}

/// Games whose start, made without the moves for n - 1 (`--tactic seed_incumbent=off`), falls short of the run's
/// ceiling, from the table's moves for n - 1: the first round raises the score of 385 and 387 to the ceiling, which
/// ends the rounds, and 378 needs two.  The games are small, so that the rounds of a budget of level 1 are not played.
const REACH_THEIR_CEILING: [u32; 3] = [378, 385, 387];

/// An unsound line without what varies from run to run: the build, and the times (the game's, each improvement's and
/// those of the other play of a game played twice).
fn without_times(line: &Json) -> Json {
    let pairs = |json: &Json| json.as_obj().expect("an object").to_vec();
    let kept = |json: &Json, vary: &[&str]| -> Vec<(String, Json)> {
        pairs(json).into_iter().filter(|(key, _)| !vary.contains(&key.as_str())).collect()
    };
    let mut line = kept(line, &["build", "seconds", "start_seconds"]);
    for (key, value) in &mut line {
        if key == "improvements" {
            let raised = value.as_arr().expect("an array").iter();
            *value = Json::array(raised.map(|raised| Json::object(kept(raised, &["seconds"]))));
        }
        if key == "other" && *value != Json::Null {
            *value = Json::object(kept(value, &["seconds"]));
        }
    }
    Json::object(line)
}

/// The lines (without their times) of the games `games` played from the table's moves for n - 1 at level `effort` on
/// one thread, with no seed from them, with the flags `extra`, one run of the binary for each game, into the file
/// `TAG-N.jsonl` of `dir`.
fn unsound_lines(dir: &TempDir, tag: &str, games: &[u32], effort: u32, extra: &[&str]) -> Vec<Json> {
    let effort = effort.to_string();
    games
        .iter()
        .map(|n| {
            let (n, file) = (n.to_string(), dir.join(format!("{tag}-{n}.jsonl")));
            let args = ["solve", &n, "--unsound", "--effort", &effort, "--tactic", "seed_incumbent=off"];
            let run = taxman(&[&args[..], &["--threads", "1", "--out", path(&file)], extra].concat());
            assert_eq!(run.code, Some(0), "game {n} with {extra:?}: {}", run.stderr);
            without_times(&Json::parse(fs::read_to_string(&file).unwrap().lines().next().unwrap()).unwrap())
        })
        .collect()
}

/// With no option, `--unsound` plays the rounds of `--above 0 --seed 1 --stop-growth 25`: the lines are the same, but
/// for the build and the times.
#[test]
fn the_defaults_are_the_options_given_by_name() {
    let dir = TempDir::new("unsound-defaults");
    let games = &REACH_THEIR_CEILING;
    let by_default = unsound_lines(&dir, "default", games, 1, &[]);
    let named = ["--above", "0", "--seed", "1", "--stop-growth", "25"];
    let by_name = unsound_lines(&dir, "named", games, 1, &named);
    assert_eq!(by_default, by_name);
    for line in &by_default {
        let count = |key: &str| line.get(key).and_then(Json::as_arr).map(<[Json]>::len);
        assert!(line.get("rounds").and_then(Json::as_u64) >= Some(1), "the rounds are played: {line}");
        assert!(count("improvements") >= Some(1), "the rounds find something: {line}");
        assert_eq!(line.get("at_ceiling"), Some(&Json::Bool(true)), "the rounds end at the ceiling: {line}");
    }
}

/// The line records the flags that the mode takes, the budget of the level, the rounds with their trajectory and
/// stage, and none of the options that the mode builds in.  Game 385 from the table's moves, with no seed from them,
/// has one trajectory, the start's, and reaches its ceiling in a round or two, which ends the rounds of a budget of
/// 131,072 work.
#[test]
fn the_line_records_the_flags_of_the_mode_and_none_of_the_options_it_builds_in() {
    let dir = TempDir::new("unsound-line");
    let out = dir.join("385.jsonl");
    let args = ["solve", "385", "--unsound", "--effort", "1", "--tactic", "seed_incumbent=off", "--threads", "1"];
    let flags = ["--seed", "2", "--above", "3", "--stop-growth", "7", "--out", path(&out)];
    let run = taxman(&[&args[..], &flags].concat());
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let line = Json::parse(fs::read_to_string(&out).unwrap().lines().next().unwrap()).unwrap();
    let flag = |key: &str, value: Json| assert_eq!(line.get(key), Some(&value), "{key}");
    flag("seed", Json::from(2u64));
    flag("above", Json::from(3u32));
    flag("stop_growth", Json::from(7u64));
    flag("effort", Json::from(1u32));
    flag("budget", Json::from(131_072u64));
    flag("known_to", Json::Null);
    for built_in in [
        "fix",
        "tilt",
        "back",
        "widen",
        "unseeded_every",
        "unseeded_effort",
        "stop_quiet",
        "push_growth",
        "from_known",
        "stop_at_known",
        "repair",
        "survivors",
    ] {
        assert!(line.get(built_in).is_none(), "the line still records {built_in}");
    }
    flag("trajectories", Json::array(["start"]));
    flag("at_ceiling", Json::Bool(true));
    let rounds = line.get("rounds").and_then(Json::as_u64).expect("the rounds");
    assert!((1..=2).contains(&rounds), "{rounds} rounds");
    flag("stage_rounds", Json::array([Json::array([rounds, 0, 0])]));
    // The work of a round is its steps and 32 for each of its 64 neighborhoods.
    let (steps, work) = (line.get("steps").and_then(Json::as_u64), line.get("work").and_then(Json::as_u64));
    assert_eq!(work, steps.map(|steps| steps + rounds * 64 * 32));
    // The round that reaches the ceiling is the last, and it raises the score.
    let improvements = line.get("improvements").and_then(Json::as_arr).expect("the improvements");
    let last = improvements.last().expect("an improvement");
    assert_eq!(last.get("round"), Some(&Json::from(rounds)));
    assert_eq!((last.get("trajectory"), last.get("stage")), (Some(&Json::from(0u64)), Some(&Json::from(0u64))));
}

/// A searched game stops at the run's ceiling, and the line says what the ceiling was and whether the game is at it:
/// game 18 from the table's moves starts at its ceiling, 18 + the table's score for 17.
#[test]
fn a_searched_game_stops_at_the_runs_ceiling() {
    let dir = TempDir::new("unsound-ceiling");
    let ceiling = Optimal::built_in().score(17).unwrap() + 18;
    let file = dir.join("stop.jsonl");
    let args = ["solve", "18", "--unsound", "--effort", "1", "--threads", "1", "--out", path(&file)];
    let run = taxman(&args);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert!(run.stdout.contains("  at ceiling"), "{}", run.stdout);
    let stopped = Json::parse(fs::read_to_string(&file).unwrap().lines().next().unwrap()).unwrap();
    assert_eq!(stopped.get("ceiling"), Some(&Json::from(ceiling)));
    assert_eq!(stopped.get("at_ceiling"), Some(&Json::Bool(true)));
    assert_eq!((stopped.get("rounds"), stopped.get("work")), (Some(&Json::from(0u64)), Some(&Json::from(0u64))));
}
