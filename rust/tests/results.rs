//! The committed results pass the checks that `taxman verify-results` runs (see `taxman::results`): every game
//! replays legally to its recorded score, is over and opens with the largest prime, keeps to the chain from
//! n − 1, and the arithmetic and provenance of its line hold; the files agree on every game they share.  The
//! counts asserted here are the ones `results/README.md` quotes.

mod common;

use common::TempDir;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::results::{verify_files, Certificate, FileReport, Proof};
use taxman::solver::{self, GameResult, Method, SolveOptions};

fn crate_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// `line` with the field at `path` (keys separated by dots, `"setup.revise"`) set to `value`.
fn with(line: &Json, path: &str, value: &Json) -> Json {
    let Json::Obj(pairs) = line else { panic!("not an object at {path}") };
    let (key, rest) = path.split_once('.').map_or((path, None), |(key, rest)| (key, Some(rest)));
    let set = |(name, old): &(String, Json)| match rest {
        _ if name != key => old.clone(),
        None => value.clone(),
        Some(rest) => with(old, rest, value),
    };
    Json::Obj(pairs.iter().map(|pair| (pair.0.clone(), set(pair))).collect())
}

/// Lines whose fields contradict each other, which no run writes but a hand edit could, and through which the proof
/// or the taint derived from a line would change, are refused by both verifiers (`verify_files` and the Python
/// referee `tools/check_results.py`), naming the line; a combination that older builds wrote is accepted by both.
/// Each file is games 1 to the tampered one, solved with `reuse_prev` off so that most of them are searched.
#[test]
fn lines_whose_fields_contradict_each_other_are_refused_by_both_verifiers() {
    let mut opts = SolveOptions { threads: 1, ..SolveOptions::default() };
    opts.tactics.set("reuse_prev=off").unwrap();
    let games = solver::solve_range(1, 90, &opts, None).unwrap();
    let lines: Vec<Json> = games.iter().map(GameResult::to_json).collect();
    let find = |what: &str, wanted: &dyn Fn(&GameResult) -> bool| {
        games.iter().position(wanted).unwrap_or_else(|| panic!("no game {what} in 1..=90"))
    };
    let exhausted = |g: &GameResult, revised: bool| {
        g.proof == Some(Proof::Exhausted) && g.setup.is_some_and(|s| s.revise.sets_frame_zero() == revised)
    };
    let revised = find("exhausted with revised freedoms", &|g| exhausted(g, true));
    let unrevised = find("exhausted with the built freedoms", &|g| exhausted(g, false));
    let capped = find("proven by the cap", &|g| g.proof == Some(Proof::Cap));
    let prime = find("prime", &|g| g.method == Method::Prime);
    let trivial = find("trivial and composite", &|g| g.method == Method::Trivial && g.n > 3);

    let s = |text: &str| Json::from(text);
    let none = Json::array(Vec::<&str>::new());
    let some_setup = Json::object([("cap", Json::Null), ("revise", s("Off"))]);
    let smallest_move = Json::array([*games[trivial].moves.iter().min().unwrap()]);
    type Edit<'a> = (&'a str, Json);
    let refused: Vec<(usize, Vec<Edit>, &str)> = vec![
        // Taints dropped: frame 0's revised freedoms said to be Off, or no setup to say anything.
        (revised, vec![("setup.revise", s("Off")), ("taint", none.clone())], "never says"),
        (revised, vec![("setup", Json::Null), ("taint", none)], "without a \"setup\""),
        (revised, vec![("in_the_bag", Json::Null)], "without \"in_the_bag\""),
        (revised, vec![("setup.revise", s("Revised"))], "bad \"setup.revise\""),
        (revised, vec![("setup.revise", s("GreedyIsMax(01)"))], "bad \"setup.revise\""),
        // A proof by the cap passed off as an exhausted search, which would not inherit the taint of n − 1.
        (capped, vec![("reached_cap", Json::Bool(false)), ("proof", s("exhausted"))], "records not reaching"),
        (unrevised, vec![("setup.revise", s("Stopped { nodes: 5 }"))], "revising was stopped"),
        (unrevised, vec![("tactic_switches.revise_freedoms", Json::Bool(false))], "never says"),
        (prime, vec![("setup", some_setup)], "a prime game with a \"setup\""),
        (prime, vec![("in_the_bag", Json::from(0u64))], "a prime game with \"in_the_bag\""),
        (prime, vec![("reached_cap", Json::Bool(true))], "reached a cap"),
        // The prime formula proves only a prime n.
        (trivial, vec![("method", s("prime")), ("in_the_bag", Json::Null)], "is not prime"),
        (trivial, vec![("promotions", smallest_move)], "a trivial game with promotions"),
    ];
    // Builds before 7af9fea wrote FromCheckpoint for a resumed search with the tactic off (and tainted it).
    let legacy = [
        ("schema", Json::from(2u32)),
        ("tactic_switches.revise_freedoms", Json::Bool(false)),
        ("setup.revise", s("FromCheckpoint")),
        ("taint", Json::array(["revise_freedoms"])),
    ];

    let dir = TempDir::new("contradictions");
    let write = |name: &str, i: usize, edits: &[Edit]| {
        let last = edits.iter().fold(lines[i].clone(), |line, (path, value)| with(&line, path, value));
        let text: String = lines[..i].iter().chain([&last]).map(|line| format!("{line}\n")).collect();
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    };
    let optimal = Optimal::load_default().unwrap();
    let mut paths = vec![write("accepted-all.jsonl", lines.len() - 1, &[])];
    paths.push(write("accepted-legacy.jsonl", unrevised, &legacy));
    for path in &paths {
        verify_files(std::slice::from_ref(path), Some(&optimal)).unwrap_or_else(|e| panic!("{e}"));
    }
    for (k, (i, edits, expected)) in refused.iter().enumerate() {
        let path = write(&format!("refused-{k:02}.jsonl"), *i, edits);
        let err = verify_files(std::slice::from_ref(&path), Some(&optimal)).unwrap_err();
        let named = [format!("line {} (n={}): ", i + 1, i + 1), format!("line {}: ", i + 1)];
        assert!(named.iter().any(|line| err.contains(line.as_str())) && err.contains(expected), "{k}: {err}");
        paths.push(path);
    }

    // The referee, on all the files at once: a problem for each refused one, at its last line, and none for the
    // others.
    let script = crate_path("tools/check_results.py");
    let run = Command::new("python3").arg("-I").arg(&script).args(&paths).output();
    let run = run.expect("python3 runs tools/check_results.py");
    let report = String::from_utf8_lossy(&run.stdout);
    assert_eq!(run.status.code(), Some(1), "{report}");
    for name in ["accepted-all.jsonl", "accepted-legacy.jsonl"] {
        assert!(!report.contains(&format!("  {name} line")), "{name}: {report}");
    }
    for (k, (i, _, _)) in refused.iter().enumerate() {
        let problem = format!("  refused-{k:02}.jsonl line {} (n={}): ", i + 1, i + 1);
        assert!(report.contains(&problem), "{problem}\n{report}");
    }
}

/// How many games of the report have the proof, the certificate, or an empty taint.
fn proofs(file: &FileReport, proof: Proof) -> usize {
    file.contents.games.values().filter(|g| g.proof == proof).count()
}

fn certificates(file: &FileReport, certificate: Certificate) -> usize {
    file.contents.games.values().filter(|g| g.certificate == Some(certificate)).count()
}

fn clean(file: &FileReport) -> usize {
    file.contents.games.values().filter(|g| g.taint.is_empty()).count()
}

#[test]
fn the_committed_results_pass_the_verifier() {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(crate_path("results"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .collect();
    paths.sort();
    let names: Vec<_> = paths.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(names, ["default.jsonl", "strict.jsonl"], "a new results file needs its counts here and in the README");
    // The chain of the cost-curve study is the strict chain 1001-1231 built by a later commit.
    paths.push(crate_path("bench/curve/chain.jsonl"));

    let optimal = Optimal::load_default().unwrap();
    let report = verify_files(&paths, Some(&optimal)).unwrap_or_else(|e| panic!("{e}"));
    println!("{report}");
    let [default, strict, chain] = &report.files[..] else { panic!("three files") };
    for (file, lo, hi) in [(default, 1001, 1231), (strict, 1001, 1219), (chain, 1001, 1231)] {
        let games = &file.contents.games;
        assert!(games.keys().copied().eq(lo..=hi), "{}: not the games {lo}..={hi}", file.path.display());
        assert_eq!((file.contents.incomplete_lines, file.contents.torn), (0, false), "{}", file.path.display());
    }
    // Every game the files share has the same score in all of them.
    assert_eq!(report.shared, 231);

    // default.jsonl: derived by reuse_prev where it could be, which most of its games rest on.
    let counts =
        |f: &FileReport| [Proof::PrimeFormula, Proof::Cap, Proof::Exhausted, Proof::ReusePrev].map(|p| proofs(f, p));
    assert_eq!(counts(default), [34, 0, 78, 119]);
    assert_eq!(clean(default), 19);
    // strict.jsonl and the chain: no unproven tactic, and nothing unproven anywhere below them.
    assert_eq!(counts(strict), [31, 66, 122, 0]);
    assert_eq!(counts(chain), [34, 70, 127, 0]);
    assert_eq!((clean(strict), clean(chain)), (219, 231));
    // Scores at a proven ceiling given the game of n − 1 (what certifies a game whatever found it).
    let ceilings = |f: &FileReport| [Certificate::NPlusPrev, Certificate::PrimeFormula].map(|c| certificates(f, c));
    assert_eq!(ceilings(default), [70, 34]);
    assert_eq!(ceilings(strict), [66, 31]);
    assert_eq!(ceilings(chain), [70, 34]);
}
