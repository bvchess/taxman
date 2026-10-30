//! The command line: every subcommand's happy path and its usage errors, through the real binary.

mod common;

use common::{committed_results, path, taxman, TempDir};
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
            "certify (A-B | --list FILE) --results FILE... --upper FILE... --out FILE",
            "verify-certificates CERTS... --results FILE... --upper FILE...",
            "--no-claims",
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
        for gone in ["verify-optimal", "extend-optimal", "strict.jsonl"] {
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
    for removed in ["--keep-going", "--retry", "--retry-improved", "--pass", "--game-hours"] {
        assert_usage_error(&["solve", "12", removed], &format!("unknown option {removed}"));
    }
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
    assert_usage_error(&["verify-results"], "usage: verify-results [--threads K] FILE...");
    assert_usage_error(&["verify-results", "--threads", "0", "a"], "--threads must be at least 1");
    assert_usage_error(&["verify-results", "--tactic", "nogoods=on", "a"], "unknown option --tactic");
}

/// `certify` finds a bad command line before it opens a file or starts a search (the files named here do not exist):
/// no range or more than one, a range that is none, a missing `--results`, `--upper` or `--out`, a flag it does not
/// have (the search tactics are `solve`'s, not `certify`'s: its search is always the extended mode) and values that
/// are no counts or limits.
#[test]
fn certify_command_lines_are_usage_errors() {
    const USAGE: &str = "usage: certify (A-B | --list FILE) --results FILE... --upper FILE... --out FILE [options]";
    let files = ["--results", "r.jsonl", "--upper", "u.jsonl", "--out", "o.jsonl"];
    let with_files = |head: &[&'static str]| [head, &files[..]].concat();

    // The range.
    assert_usage_error(&["certify"], "certify takes one range A-B");
    assert_usage_error(&with_files(&["certify"]), "certify takes one range A-B");
    assert_usage_error(&with_files(&["certify", "1302", "1303"]), "certify takes one range A-B");
    assert_usage_error(&with_files(&["certify", "x"]), "expected a game N or a range A-B");
    assert_usage_error(&with_files(&["certify", "1302-"]), "expected a game N or a range A-B");
    assert_usage_error(&with_files(&["certify", "1303-1302"]), "empty or starts below 1");
    assert_usage_error(&with_files(&["certify", "0-5"]), "empty or starts below 1");

    // The three files it cannot do without, each missing in turn; these also print the command's own usage line.
    let missing = [
        (
            &["certify", "1302", "--upper", "u.jsonl", "--out", "o.jsonl"][..],
            "certify needs --results FILE and --upper FILE",
        ),
        (
            &["certify", "1302", "--results", "r.jsonl", "--out", "o.jsonl"],
            "certify needs --results FILE and --upper FILE",
        ),
        (&["certify", "1302", "--results", "r.jsonl", "--upper", "u.jsonl"], "certify needs --out FILE"),
    ];
    for (args, message) in missing {
        assert_usage_error(args, message);
        assert_usage_error(args, USAGE);
    }
    assert_usage_error(
        &["certify", "1302", "--results", "r.jsonl", "--upper", "u.jsonl", "--out"],
        "--out needs a value",
    );

    // The flags.
    assert_usage_error(&with_files(&["certify", "1302", "--tactic", "nogoods=off"]), "unknown option --tactic");
    assert_usage_error(&with_files(&["certify", "1302", "--show"]), "unknown option --show");
    assert_usage_error(&with_files(&["certify", "1302", "--window", "2"]), "unknown option --window");
    assert_usage_error(&with_files(&["certify", "1302", "--threads", "0"]), "--threads must be at least 1");
    assert_usage_error(&with_files(&["certify", "1302", "--threads", "many"]), "--threads: cannot read");
    assert_usage_error(&with_files(&["certify", "1302", "--node-limit", "0"]), "--node-limit must be at least 1");
    assert_usage_error(&with_files(&["certify", "1302", "--time-limit", "-1"]), "not a number of seconds");
    assert_usage_error(&with_files(&["certify", "1302", "--checkpoint-interval", "0"]), "must be more than 0 seconds");
    // `--no-claims` is a switch of `certify` alone: it takes no value, and `verify-certificates` has none (each line
    // says its formulation).
    assert_usage_error(&with_files(&["certify", "1302", "--no-claims=yes"]), "--no-claims takes no value");
    assert_usage_error(&["verify-certificates", "c.jsonl", "--no-claims"], "unknown option --no-claims");
}

/// `certify` takes a range or a list file of boards, exactly one of them, and reads the list before it opens any other
/// file or starts a search: both, neither, two lists, a missing list file, a line that is no board, a board out of
/// range, a board twice and a list with no board are usage errors that name the file and the line.
#[test]
fn certify_takes_a_range_or_a_list_of_boards_and_not_both() {
    let dir = TempDir::new("certify-list-usage");
    let files = ["--results", "r.jsonl", "--upper", "u.jsonl", "--out", "o.jsonl"];
    let list = |name: &str, text: &str| {
        let file = dir.join(name);
        fs::write(&file, text).unwrap();
        file
    };
    // The command line `head` followed by the files it cannot do without, refused with `message`.
    let refused = |head: &[&str], message: &str| {
        let args: Vec<&str> = head.iter().chain(&files).copied().collect();
        assert_usage_error(&args, message);
    };

    let good = list("good.txt", "# the boards\n1304\n\n1302\n");
    let (good, missing) = (path(&good), path(&dir.join("missing.txt")).to_owned());
    refused(&["certify", "1302", "--list", good], "certify takes a range A-B or --list FILE, not both");
    refused(&["certify", "--list", good, "--list", good], "certify takes one --list FILE");
    refused(&["certify"], "certify takes one range A-B or --list FILE");
    refused(&["certify", "--list", &missing], "missing.txt: ");
    assert_usage_error(&["certify", "--list"], "--list needs a value");

    let bad = |name: &str, text: &str, why: &str| {
        let file = list(name, text);
        refused(&["certify", "--list", path(&file)], &format!("{name}: line {why}"));
    };
    bad("word.txt", "1302\nboard\n", "2: expected a game n, got \"board\"");
    bad("note.txt", "1302 # a note\n", "1: expected a game n, got \"1302 # a note\"");
    bad("zero.txt", "0\n", "1: 0 is not a game from 1 to 8191");
    bad("large.txt", "# c\n\n8192\n", "3: 8192 is not a game from 1 to 8191");
    bad("twice.txt", "1302\n1304\n\n1302\n", "4: the board 1302 is listed again (first on line 1)");
    let empty = list("empty.txt", "# nothing\n\n");
    refused(&["certify", "--list", path(&empty)], "empty.txt: no board is listed");
}

/// `verify-certificates` needs a file of certificates, `--results` and `--upper`, takes `--exact` and `--threads` and
/// no other flag, and a file that is not there is a failure (status 1) that names it.
#[test]
fn verify_certificates_command_lines() {
    const USAGE: &str = "usage: verify-certificates CERTS... --results FILE... --upper FILE...";
    assert_usage_error(
        &["verify-certificates", "--results", "r.jsonl", "--upper", "u.jsonl"],
        "needs a file of certificates",
    );
    assert_usage_error(
        &["verify-certificates", "c.jsonl", "--upper", "u.jsonl"],
        "needs --results FILE and --upper FILE",
    );
    assert_usage_error(&["verify-certificates", "c.jsonl", "--results", "r.jsonl"], USAGE);
    let files = ["c.jsonl", "--results", "r.jsonl", "--upper", "u.jsonl"];
    assert_usage_error(
        &[&["verify-certificates"][..], &files, &["--threads", "0"]].concat(),
        "--threads must be at least 1",
    );
    assert_usage_error(&[&["verify-certificates"][..], &files, &["--out", "o.jsonl"]].concat(), "unknown option --out");
    assert_usage_error(&[&["verify-certificates"][..], &files, &["--exact"]].concat(), "--exact needs a value");

    let dir = TempDir::new("verify-certificates-missing");
    let missing = dir.join("missing.jsonl");
    let run = taxman(&["verify-certificates", path(&missing), "--results", path(&missing), "--upper", path(&missing)]);
    assert_eq!(run.code, Some(1), "{}{}", run.stdout, run.stderr);
    assert!(run.stderr.contains("missing.jsonl"), "{}", run.stderr);
    assert_eq!(run.stdout, "");
}

/// A command that fails on its inputs, not on its command line, exits with status 1 and names what is wrong, without
/// searching: a results file that is not there, a board the barred bounds have no line for (it is named, also when
/// the boards before it are fine), and a board whose game the results lack.
#[test]
fn certify_names_the_input_that_is_missing() {
    let dir = TempDir::new("certify-missing");
    let (committed, out) = (committed_results(), dir.join("out.jsonl"));
    // The barred bounds of a prime and, on the board after it, nothing.
    let upper = dir.join("upper.jsonl");
    fs::write(&upper, "{\"n\":1301,\"score\":532272,\"relation\":\"prime\",\"needs\":0,\"secs\":0.002}\n").unwrap();
    let certify = |range: &str, results: &Path| {
        let args = ["certify", range, "--results", path(results), "--upper", path(&upper), "--out", path(&out)];
        taxman(&[&args[..], &["--threads", "1", "--progress", "0"]].concat())
    };

    let run = certify("1301-1302", &committed);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert!(run.stderr.contains("error: n=1302: the --upper files have no line for it"), "{}", run.stderr);
    assert_eq!(run.stdout, "");
    let run = certify("2176", &committed);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert!(run.stderr.contains("n=2176: the --upper files have no line for it"), "{}", run.stderr);

    let missing = dir.join("missing.jsonl");
    let run = certify("1301", &missing);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert!(run.stderr.contains("missing.jsonl"), "{}", run.stderr);
    assert_eq!(run.stdout, "");

    // The barred bounds say that the first board past the end of the table built into the binary needs a certificate,
    // and the results given have no game of it.  Certify says so before it sets the board up (a board of 5001 numbers
    // takes seconds), so the line needs no open member; an open member is a source that setup keeps on the board it
    // is for (682 is one on the board 1302, not on 5001).
    let past = Optimal::built_in().max_n() + 1;
    let needs = dir.join("needs.jsonl");
    fs::write(&needs, format!("{{\"n\":{past},\"score\":533559,\"needs\":1,\"open\":[]}}\n")).unwrap();
    let alone = dir.join("alone.jsonl");
    fs::write(&alone, "").unwrap();
    let board = past.to_string();
    let args = ["certify", &board, "--results", path(&alone), "--upper", path(&needs), "--out", path(&out)];
    let run = taxman(&args);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert!(run.stderr.contains(&format!("n={past}: the results have no game of n")), "{}", run.stderr);
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

/// `--background` is a switch that `solve` and `certify` take, which changes no score, which a line records in its
/// `options` where it was given, and which says on stderr, once, that it does nothing where there is no idle policy.
#[test]
fn background_is_a_switch_that_is_recorded_and_changes_no_score() {
    assert_usage_error(&["solve", "12", "--background=1"], "--background takes no value");
    assert_usage_error(&["certify", "12", "--background"], "certify needs --results FILE and --upper FILE");

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
/// gate does nothing.  A line records it in `options.gate` where it was
/// given, beside the gates of `--gate-trial`, and the game ends with the score of the table.
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
    assert_eq!(plain.get("score").and_then(Json::as_u64), Optimal::built_in().score(900));
    // The gate at its least, and at its most with a trial: the trial's gates are recorded beside it.
    let zero = line("zero.jsonl", &["--gate", "0"]);
    let widest = line("widest.jsonl", &["--gate", &MAX_GATE.to_string(), "--gate-trial", "0,600"]);
    for (name, gated) in [("zero", &zero), ("widest", &widest)] {
        assert_eq!(gated.get("score"), plain.get("score"), "{name}");
        assert_eq!(gated.get("optimal_match"), Some(&Json::Bool(true)), "{name}");
    }
    assert_eq!(options(&zero).get("gate").and_then(Json::as_u64), Some(0));
    assert_eq!(options(&zero).get("gate_trial"), None);
    assert_eq!(options(&widest).get("gate").and_then(Json::as_u64), Some(MAX_GATE));
    assert_eq!(options(&widest).get("gate_trial").map(Json::to_string).as_deref(), Some("[0,600]"));
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
    for text in ["note: no previous game for n=40", "standalone", "--use-prev to take the known game of 39"] {
        assert!(run.stderr.contains(text), "{text}: {}", run.stderr);
    }
    assert!(run.stdout.contains("n=40") && !run.stdout.contains("assumes"), "{}", run.stdout);
    // With a previous game (the table's 39, which the game then assumes optimal), or for n = 1, which has none to
    // have, there is nothing to say.
    for args in [&["40", "--use-prev"][..], &["1"], &["1-5"]] {
        let run = standalone(args);
        assert_eq!(run.code, Some(0), "{args:?}: {}", run.stderr);
        assert_eq!(run.stderr, "", "{args:?}");
    }
}
