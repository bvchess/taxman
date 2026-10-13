//! The committed results pass the checks that `taxman verify-results` runs (see `taxman::results`): every game
//! replays legally to its recorded score, is over and opens with the largest prime, keeps to the chain from
//! n − 1, and the arithmetic and provenance of its line hold; the files agree on every game they share.  The
//! counts asserted here are the ones `results/README.md` quotes.  And both verifiers, `verify-results` and the
//! Python referee, refuse lines that a hand edit made contradict themselves, and derive taints by the same rules.

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

/// A change to a line: the field at a path (as [`with`] takes it) and its new value.
type Edit<'a> = (&'a str, Json);

/// Runs both verifiers, `verify_files` (on each file alone) and the Python referee `tools/check_results.py` (on all
/// of them at once), on files made of `lines` (games 1 and up) cut after line `i + 1`, with `edits` made to that
/// line.  Each of `accepted` (`i`, `edits`) must pass both; each of `refused` (`i`, `edits`, a part of the Rust
/// verifier's message) must be refused by both, naming line `i + 1`.
fn both_verifiers(
    dir: &TempDir,
    lines: &[Json],
    accepted: &[(usize, Vec<Edit>)],
    refused: &[(usize, Vec<Edit>, &str)],
) {
    let write = |name: String, i: usize, edits: &[Edit]| {
        let last = edits.iter().fold(lines[i].clone(), |line, (path, value)| with(&line, path, value));
        let text: String = lines[..i].iter().chain([&last]).map(|line| format!("{line}\n")).collect();
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        path
    };
    let optimal = Optimal::load_default().unwrap();
    let mut paths = Vec::new();
    for (k, (i, edits)) in accepted.iter().enumerate() {
        let path = write(format!("accepted-{k:02}.jsonl"), *i, edits);
        verify_files(std::slice::from_ref(&path), Some(&optimal)).unwrap_or_else(|e| panic!("{e}"));
        paths.push(path);
    }
    for (k, (i, edits, expected)) in refused.iter().enumerate() {
        let path = write(format!("refused-{k:02}.jsonl"), *i, edits);
        let err = verify_files(std::slice::from_ref(&path), Some(&optimal)).unwrap_err();
        let named = [format!("line {} (n={}): ", i + 1, i + 1), format!("line {}: ", i + 1)];
        assert!(named.iter().any(|line| err.contains(line.as_str())) && err.contains(expected), "{k}: {err}");
        paths.push(path);
    }

    let script = crate_path("tools/check_results.py");
    let run = Command::new("python3").arg("-I").arg(&script).args(&paths).output();
    let run = run.expect("python3 runs tools/check_results.py");
    let report = String::from_utf8_lossy(&run.stdout);
    assert_eq!(run.status.code(), Some(if refused.is_empty() { 0 } else { 1 }), "{report}");
    for k in 0..accepted.len() {
        assert!(!report.contains(&format!("  accepted-{k:02}.jsonl line")), "accepted {k}: {report}");
    }
    for (k, (i, _, _)) in refused.iter().enumerate() {
        let problem = format!("  refused-{k:02}.jsonl line {} (n={}): ", i + 1, i + 1);
        assert!(report.contains(&problem), "{problem}\n{report}");
    }
}

/// Lines whose fields contradict each other, which no run writes but a hand edit could, and through which the proof
/// or the taint derived from a line would change, are refused by both verifiers, naming the line; a combination
/// that older builds wrote is accepted by both, in a line of the schema they wrote.  Each file is games 1 to the
/// tampered one, solved with `reuse_prev` off so that most of them are searched.
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
    // Builds before 7af9fea wrote FromCheckpoint for a resumed search with the tactic off (and tainted it), up to
    // schema 2.
    let legacy = vec![
        ("tactic_switches.revise_freedoms", Json::Bool(false)),
        ("setup.revise", s("FromCheckpoint")),
        ("taint", Json::array(["revise_freedoms"])),
    ];
    let legacy_schema_2 = [legacy.clone(), vec![("schema", Json::from(2u32))]].concat();
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
        (unrevised, legacy, "never says"),
        (prime, vec![("setup", some_setup)], "a prime game with a \"setup\""),
        (prime, vec![("in_the_bag", Json::from(0u64))], "a prime game with \"in_the_bag\""),
        (prime, vec![("reached_cap", Json::Bool(true))], "reached a cap"),
        // The prime formula proves only a prime n.
        (trivial, vec![("method", s("prime")), ("in_the_bag", Json::Null)], "is not prime"),
        (trivial, vec![("promotions", smallest_move)], "a trivial game with promotions"),
    ];
    let accepted = [(lines.len() - 1, vec![]), (unrevised, legacy_schema_2)];
    both_verifiers(&TempDir::new("contradictions"), &lines, &accepted, &refused);
}

/// A proof by the cap rests on nothing the game used itself (from schema 3 on): after a strict chain 1..81, game 82
/// searched with `revise_freedoms` on (frame 0 revised) reaches its cap, rests on nothing unproven, and a strict run
/// builds on it.  Both verifiers hold a line to the rule of its schema: schema 2 counted the game's own taint in a
/// proof by the cap too.
#[test]
fn a_proof_by_the_cap_rests_on_nothing_the_game_used_itself() {
    let dir = TempDir::new("cap-taint");
    let results = dir.join("chain.jsonl");
    let strict = common::strict_options(1);
    let mut revising = strict.clone();
    revising.tactics.set("revise_freedoms=on").unwrap();
    solver::solve_range(1, 81, &strict, Some(&results)).unwrap();
    let capped = solver::solve_range(82, 82, &revising, Some(&results)).unwrap().remove(0);
    assert_eq!(capped.proof, Some(Proof::Cap));
    assert!(capped.setup.is_some_and(|s| s.revise.sets_frame_zero()), "{:?}", capped.setup);
    assert!(capped.taint.is_empty(), "{:?}", capped.taint);
    // A strict run (without --allow-tainted-prev) builds only on a game that rests on nothing unproven.
    let next = solver::solve_range(83, 84, &strict, Some(&results)).unwrap();
    assert!(next.iter().all(|g| g.completed && g.taint.is_empty()), "{next:?}");

    let lines: Vec<Json> = fs::read_to_string(&results).unwrap().lines().map(|l| Json::parse(l).unwrap()).collect();
    let i = 81;
    assert_eq!(lines[i].get("n").and_then(Json::as_u32), Some(82));
    let schema_2 = ("schema", Json::from(2u32));
    let tainted = ("taint", Json::array(["revise_freedoms"]));
    let accepted = [(lines.len() - 1, vec![]), (i, vec![schema_2.clone(), tainted.clone()])];
    let refused = [(i, vec![schema_2], "records the taint"), (i, vec![tainted], "records the taint")];
    both_verifiers(&dir, &lines, &accepted, &refused);
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
    // strict.jsonl is a chain from n = 1 made in resumable chunks: it holds the lines of the four searches that a
    // chunk's time limit stopped, besides the completed game of each.
    for (file, lo, hi, incomplete) in [(default, 1001, 1231, 0), (strict, 1, 1300, 4), (chain, 1001, 1231, 0)] {
        let games = &file.contents.games;
        assert!(games.keys().copied().eq(lo..=hi), "{}: not the games {lo}..={hi}", file.path.display());
        assert_eq!(
            (file.contents.incomplete_lines, file.contents.torn),
            (incomplete, false),
            "{}",
            file.path.display()
        );
        assert!(file.contents.runs.is_empty(), "{}: a game left unfinished", file.path.display());
    }
    // Every game the files share has the same score in all of them: 1001-1231.
    assert_eq!(report.shared, 231);
    // The strict chain from n = 1 reproduces optimal.json: its scores to 1000 are the Java's.
    for n in 1..=1000 {
        assert_eq!(Some(strict.contents.games[&n].score), optimal.score(n), "n={n}");
    }
    // The games the table holds past 1000 were copied from it (`taxman extend-optimal`), move for move.
    for (n, score, moves) in optimal.games().filter(|&(n, _, _)| n > 1000) {
        let game =
            strict.contents.games.get(&n).unwrap_or_else(|| panic!("optimal.json has a game of {n}, strict.jsonl not"));
        assert_eq!((score, moves, game.taint.is_empty()), (game.score, &game.moves[..], true), "n={n}");
    }

    // default.jsonl: derived by reuse_prev where it could be, which most of its games rest on.
    let counts = |f: &FileReport| {
        [Proof::Trivial, Proof::PrimeFormula, Proof::Cap, Proof::Exhausted, Proof::ReusePrev].map(|p| proofs(f, p))
    };
    assert_eq!(counts(default), [0, 34, 0, 78, 119]);
    assert_eq!(clean(default), 19);
    // strict.jsonl and the chain: no unproven tactic, and nothing unproven anywhere below them.
    assert_eq!(counts(strict), [13, 211, 415, 661, 0]);
    assert_eq!(counts(chain), [0, 34, 70, 127, 0]);
    assert_eq!((clean(strict), clean(chain)), (1300, 231));
    // Scores at a proven ceiling given the game of n − 1 (what certifies a game whatever found it).
    let ceilings = |f: &FileReport| [Certificate::NPlusPrev, Certificate::PrimeFormula].map(|c| certificates(f, c));
    assert_eq!(ceilings(default), [70, 34]);
    assert_eq!(ceilings(strict), [424, 210]);
    assert_eq!(ceilings(chain), [70, 34]);
}
