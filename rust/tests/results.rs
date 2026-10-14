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
use taxman::results::{moves_digest, verify_files, Certificate, FileReport, Proof, Taint, OPTIMAL_JSON};
use taxman::solver::{self, GameResult, Method};

fn crate_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

/// `line` with the field at `path` (keys separated by dots, `"same_search.n"`) set to `value`; a field the line lacks
/// is added.
fn with(line: &Json, path: &str, value: &Json) -> Json {
    let Json::Obj(pairs) = line else { panic!("not an object at {path}") };
    let (key, rest) = path.split_once('.').map_or((path, None), |(key, rest)| (key, Some(rest)));
    let set = |(name, old): &(String, Json)| match rest {
        _ if name != key => old.clone(),
        None => value.clone(),
        Some(rest) => with(old, rest, value),
    };
    let mut fields: Vec<(String, Json)> = pairs.iter().map(|pair| (pair.0.clone(), set(pair))).collect();
    if rest.is_none() && !pairs.iter().any(|(name, _)| name == key) {
        fields.push((key.to_owned(), value.clone()));
    }
    Json::Obj(fields)
}

/// A change to a line: the field at a path (as [`with`] takes it) and its new value.
type Edit<'a> = (&'a str, Json);

/// What both verifiers must say of a results file: accept it, or refuse it naming a line (counting from 1) and its
/// board size, with a part of the Rust verifier's message.
enum Verdict<'a> {
    Accepted,
    Refused { line: usize, n: u32, expected: &'a str },
}

/// Runs both verifiers, `verify_files` (on each file alone) and the Python referee `tools/check_results.py` (on all
/// of them at once), on files made of the lines of each case, and checks that each gives its verdict.
fn verifiers_agree(dir: &TempDir, cases: &[(Vec<Json>, Verdict)]) {
    let optimal = Optimal::load_default().unwrap();
    let mut paths = Vec::new();
    for (k, (lines, verdict)) in cases.iter().enumerate() {
        let path = dir.join(format!("case-{k:02}.jsonl"));
        fs::write(&path, lines.iter().map(|line| format!("{line}\n")).collect::<String>()).unwrap();
        let verified = verify_files(std::slice::from_ref(&path), Some(&optimal));
        match (verified, verdict) {
            (Ok(_), Verdict::Accepted) => {}
            (Err(err), Verdict::Refused { line, n, expected }) => {
                let named = [format!("line {line} (n={n}): "), format!("line {line}: ")];
                assert!(named.iter().any(|line| err.contains(line.as_str())) && err.contains(expected), "{k}: {err}");
            }
            (Ok(_), Verdict::Refused { .. }) => panic!("case {k} is accepted"),
            (Err(err), Verdict::Accepted) => panic!("case {k}: {err}"),
        }
        paths.push(path);
    }

    let script = crate_path("tools/check_results.py");
    let run = Command::new("python3").arg("-I").arg(&script).args(&paths).output();
    let run = run.expect("python3 runs tools/check_results.py");
    let report = String::from_utf8_lossy(&run.stdout);
    let refusals = cases.iter().filter(|(_, verdict)| matches!(verdict, Verdict::Refused { .. })).count();
    assert_eq!(run.status.code(), Some(if refusals == 0 { 0 } else { 1 }), "{report}");
    for (k, (_, verdict)) in cases.iter().enumerate() {
        match verdict {
            Verdict::Accepted => assert!(!report.contains(&format!("  case-{k:02}.jsonl line")), "case {k}: {report}"),
            Verdict::Refused { line, n, .. } => {
                let problem = format!("  case-{k:02}.jsonl line {line} (n={n}): ");
                assert!(report.contains(&problem), "{problem}\n{report}");
            }
        }
    }
}

/// [`verifiers_agree`] on files made of `lines` (games 1 and up) cut after line `i + 1`, with `edits` made to that
/// line.  Each of `accepted` (`i`, `edits`) must pass both; each of `refused` (`i`, `edits`, a part of the Rust
/// verifier's message) must be refused by both, naming line `i + 1`.
fn both_verifiers(
    dir: &TempDir,
    lines: &[Json],
    accepted: &[(usize, Vec<Edit>)],
    refused: &[(usize, Vec<Edit>, &str)],
) {
    let cut = |i: usize, edits: &[Edit]| {
        let last = edits.iter().fold(lines[i].clone(), |line, (path, value)| with(&line, path, value));
        lines[..i].iter().cloned().chain([last]).collect::<Vec<Json>>()
    };
    let accepted = accepted.iter().map(|(i, edits)| (cut(*i, edits), Verdict::Accepted));
    let refused = refused
        .iter()
        .map(|(i, edits, expected)| (cut(*i, edits), Verdict::Refused { line: i + 1, n: *i as u32 + 1, expected }));
    verifiers_agree(dir, &accepted.chain(refused).collect::<Vec<_>>());
}

/// Lines whose fields contradict each other, which no run writes but a hand edit could, and through which the proof
/// or the taint derived from a line would change, are refused by both verifiers, naming the line; a line that only
/// older builds wrote, with a `setup.revise` of "Off", is accepted by both.  Each file is games 1 to the tampered
/// one.
#[test]
fn lines_whose_fields_contradict_each_other_are_refused_by_both_verifiers() {
    let opts = common::options(1);
    let games = solver::solve_range(1, 90, &opts, None).unwrap();
    let lines: Vec<Json> = games.iter().map(GameResult::to_json).collect();
    let find = |what: &str, wanted: &dyn Fn(&GameResult) -> bool| {
        games.iter().position(wanted).unwrap_or_else(|| panic!("no game {what} in 1..=90"))
    };
    let exhausted = find("exhausted", &|g| g.proof == Some(Proof::Exhausted) && g.setup.is_some());
    let capped = find("proven by the cap", &|g| g.proof == Some(Proof::Cap));
    let prime = find("prime", &|g| g.method == Method::Prime);
    let trivial = find("trivial and composite", &|g| g.method == Method::Trivial && g.n > 3);

    let s = |text: &str| Json::from(text);
    let some_setup = Json::object([
        ("frames", Json::from(1u32)),
        ("candidates", Json::from(1u32)),
        ("max_total", Json::from(1u32)),
        ("cap", Json::Null),
        ("digest", s("0123456789abcdef")),
    ]);
    let smallest_move = Json::array([*games[trivial].moves.iter().min().unwrap()]);
    let refused: Vec<(usize, Vec<Edit>, &str)> = vec![
        (exhausted, vec![("setup", Json::Null)], "without a \"setup\""),
        (exhausted, vec![("in_the_bag", Json::Null)], "without \"in_the_bag\""),
        // Only a build that had the tactic `revise_freedoms` wrote a `setup.revise`, and with it off, "Off".
        (exhausted, vec![("setup.revise", s("Revised"))], "bad \"setup.revise\""),
        (exhausted, vec![("setup.revise", s("GreedyIsMax(0)"))], "bad \"setup.revise\""),
        (exhausted, vec![("setup.revise", Json::Null)], "bad \"setup.revise\""),
        // Only schema 4 is read.
        (exhausted, vec![("schema", Json::from(3u32))], "older than the schema 4"),
        (exhausted, vec![("schema", Json::Null)], "bad \"schema\""),
        // A proof by the cap passed off as an exhausted search, which would not inherit the taint of n − 1.
        (capped, vec![("reached_cap", Json::Bool(false)), ("proof", s("exhausted"))], "records not reaching"),
        (prime, vec![("setup", some_setup)], "a prime game with a \"setup\""),
        (prime, vec![("in_the_bag", Json::from(0u64))], "a prime game with \"in_the_bag\""),
        (prime, vec![("reached_cap", Json::Bool(true))], "reached a cap"),
        // The prime formula proves only a prime n.
        (trivial, vec![("method", s("prime")), ("in_the_bag", Json::Null)], "is not prime"),
        (trivial, vec![("promotions", smallest_move)], "a trivial game with promotions"),
    ];
    let accepted = [(lines.len() - 1, vec![]), (exhausted, vec![("setup.revise", s("Off"))])];
    both_verifiers(&TempDir::new("contradictions"), &lines, &accepted, &refused);
}

/// A proof rests on the taint of the game it rests on and on nothing else.  Games 33 to 50, the first built on
/// `optimal.json`'s game of 32: 33 reaches its cap and 34 repeats 33, so both rest on `optimal.json`; 35 exhausts its
/// search, which rests on no game, so it and the games after it that rest on it are clean.  Both verifiers refuse a
/// line that records the taint of a proof by the cap without it, an exhausted search with it, or a prime game built
/// on a clean game with it.
#[test]
fn a_proof_rests_on_the_taint_of_the_game_it_rests_on_and_on_nothing_else() {
    let games = solver::solve_range(33, 50, &common::options(1), None).unwrap();
    let lines: Vec<Json> = games.iter().map(GameResult::to_json).collect();
    let rests_on_optimal = Taint::from([OPTIMAL_JSON.to_owned()]);
    let expected = [
        (33, Proof::Cap, &rests_on_optimal),
        (34, Proof::SameSearch, &rests_on_optimal),
        (35, Proof::Exhausted, &Taint::new()),
        (37, Proof::PrimeFormula, &Taint::new()),
    ];
    for (n, proof, taint) in expected {
        let game = &games[n - 33];
        assert_eq!((game.n as usize, game.proof, &game.taint), (n, Some(proof), taint));
    }
    let (rests, free) = (Json::array([OPTIMAL_JSON]), Json::array(Vec::<&str>::new()));
    let cut = |i: usize, taint: &Json| -> Vec<Json> {
        lines[..i].iter().cloned().chain([with(&lines[i], "taint", taint)]).collect()
    };
    let refused = |i: usize, taint: &Json| {
        (cut(i, taint), Verdict::Refused { line: i + 1, n: 33 + i as u32, expected: "records the taint" })
    };
    let cases = vec![
        (lines.clone(), Verdict::Accepted),
        refused(0, &free),
        refused(1, &free),
        refused(2, &rests),
        refused(4, &rests),
    ];
    verifiers_agree(&TempDir::new("taint-rules"), &cases);
}

/// A repeat (method `repeat`, proof `same_search`) stands on the game it names: both verifiers refuse one whose
/// record, digests, promotions or arithmetic do not fit that game, whose game is not a completed game of the file
/// before it, settles no search or comes after it, and one that drops the taint of the game it repeats.  The Rust
/// verifier also sets both search problems up again and compares them, which refuses two more that the Python
/// referee, which cannot, accepts.  Games 1..90, in which 65 repeats 64, a search.
#[test]
fn forged_repeats_are_refused_by_both_verifiers() {
    let games = solver::solve_range(1, 90, &common::options(1), None).unwrap();
    let lines: Vec<Json> = games.iter().map(GameResult::to_json).collect();
    let i = 64;
    let (repeat, m) = (&games[i], &games[i - 1]);
    assert_eq!((repeat.n, repeat.same_search.as_ref().map(|s| s.n), m.method), (65, Some(64), Method::Search));
    // The latest searched games before 64 with other promotions and with the same ones (but another problem), and a
    // prime game, which settles no search.
    let searched = |same: bool| {
        let found =
            games[..i - 1].iter().rev().find(|g| g.method == Method::Search && (g.promotions == m.promotions) == same);
        found.unwrap()
    };
    let (other, twin) = (searched(false), searched(true));
    assert!([other, twin].iter().all(|g| g.setup.map(|s| s.digest) != m.setup.map(|s| s.digest)));
    let prime = games[..i - 1].iter().rev().find(|g| g.method == Method::Prime).unwrap();
    let digest = |g: &GameResult| Json::from(taxman::solver::digest_text(g.setup.unwrap().digest));
    let proof = |g: &GameResult| g.proof.unwrap().as_str();
    let s = |text: &str| Json::from(text);
    let none = Json::array(Vec::<&str>::new());
    let mut promotions = repeat.promotions.clone();
    let dropped = promotions.pop().unwrap();
    let sum = repeat.same_search.as_ref().unwrap().promotion_sum;

    let refused: Vec<(usize, Vec<Edit>, &str)> = vec![
        // Another game named, with its proof: its digest is not this one's, and with the digests made its, its
        // promotions are not this one's.
        (
            i,
            vec![("same_search.n", Json::from(other.n)), ("same_search.proof", s(proof(other)))],
            "different search digests",
        ),
        (
            i,
            vec![
                ("same_search.n", Json::from(other.n)),
                ("same_search.proof", s(proof(other))),
                ("same_search.digest", digest(other)),
                ("setup.digest", digest(other)),
            ],
            "its promotions are not those of game",
        ),
        (i, vec![("same_search.digest", s("0123456789abcdef"))], "record different search digests"),
        (
            i,
            vec![("same_search.n", Json::from(prime.n)), ("same_search.proof", s("prime_formula"))],
            "settles no search",
        ),
        (i, vec![("same_search.n", Json::from(repeat.n))], "not a completed game of the file below it"),
        (i, vec![("same_search.proof", s("cap"))], "its record of game 64 does not match line 64"),
        (i, vec![("same_search.promotion_sum", Json::from(sum + 1))], "not the promotion_sum"),
        // One promotion fewer and in_the_bag raised by it: the line adds up, but the promotions are not 64's.
        (
            i,
            vec![
                ("promotions", Json::array(promotions)),
                ("in_the_bag", Json::from(repeat.in_the_bag.unwrap() + u64::from(dropped))),
                ("same_search.promotion_sum", Json::from(sum - u64::from(dropped))),
            ],
            "its promotions are not those of game 64",
        ),
        (i, vec![("setup.cap", Json::from(sum))], "has a cap"),
        (i, vec![("same_search", Json::Null)], "without a \"same_search\" record"),
        (
            i - 1,
            vec![("same_search", lines[i].get("same_search").unwrap().clone())],
            "a search game with a \"same_search\"",
        ),
    ];
    let accepted = [(lines.len() - 1, vec![])];
    let dir = TempDir::new("forged-repeats");
    both_verifiers(&dir, &lines, &accepted, &refused);

    let mut cases: Vec<(Vec<Json>, Verdict)> = Vec::new();
    // The game repeated comes after the repeat in the file.
    let mut swapped = lines[..=i].to_vec();
    swapped.swap(i - 1, i);
    cases.push((swapped, Verdict::Refused { line: i, n: 65, expected: "whose line (65) comes after it" }));
    // The game repeated is only an unfinished search in the file (and 65 is built on optimal.json's 64 instead).
    let optimal = Optimal::load_default().unwrap();
    let unfinished = Json::object([
        ("schema", Json::from(taxman::results::SCHEMA)),
        ("n", Json::from(64u32)),
        ("completed", Json::Bool(false)),
        ("invocations", Json::from(1u32)),
    ]);
    let from_optimal = [
        ("prev_source", s("optimal")),
        ("prev.source", s("optimal")),
        ("prev.moves_digest", s(&moves_digest(optimal.moves(64).unwrap()))),
        ("prev.proof", Json::Null),
        ("prev.taint", Json::array([OPTIMAL_JSON])),
    ];
    let built_on_optimal = from_optimal.iter().fold(lines[i].clone(), |line, (path, value)| with(&line, path, value));
    let mut incomplete: Vec<Json> = lines[..i - 1].to_vec();
    incomplete.extend([unfinished, built_on_optimal.clone()]);
    cases.push((
        incomplete,
        Verdict::Refused { line: 65, n: 65, expected: "not a completed game of the file below it" },
    ));
    let mut complete = lines[..i].to_vec();
    complete.push(built_on_optimal);
    cases.push((complete, Verdict::Accepted));
    // The game repeated says its problem has one more promotion to spare than it has (as a line of a build whose
    // frame builder posed another problem would): the problem set up again is not the one its line records.
    let max_total = m.setup.unwrap().max_total;
    let mut resized = lines[..=i].to_vec();
    resized[i - 1] = with(&lines[i - 1], "setup.max_total", &Json::from(max_total + 1));
    cases.push((resized, Verdict::Refused { line: 65, n: 65, expected: "not the ProblemSize" }));
    // 65 said to repeat a game that promotes what 64 does (so what 65 does), with its digests made that game's: the
    // problem set up again does not have the digest recorded (and the Python referee sees the setups' sizes differ).
    let twin_edits = [
        ("same_search.n", Json::from(twin.n)),
        ("same_search.proof", s(proof(twin))),
        ("same_search.digest", digest(twin)),
        ("setup.digest", digest(twin)),
    ];
    let twinned = twin_edits.iter().fold(lines[i].clone(), |line, (path, value)| with(&line, path, value));
    let twinned: Vec<Json> = lines[..i].iter().cloned().chain([twinned]).collect();
    cases.push((twinned, Verdict::Refused { line: 65, n: 65, expected: "its search problem's digest is" }));
    // A repeat inherits the taint of the game it repeats: 34 repeats 33, which built on optimal.json's 32 reaches its
    // cap and so rests on optimal.json.
    let tainted: Vec<Json> =
        solver::solve_range(33, 34, &common::options(1), None).unwrap().iter().map(GameResult::to_json).collect();
    assert_eq!(tainted[1].get("taint"), Some(&Json::array([OPTIMAL_JSON])));
    cases.push((tainted.clone(), Verdict::Accepted));
    for (path, value, expected) in [
        ("taint", none.clone(), "records the taint"),
        ("same_search.taint", none.clone(), "its record of game 33 does not match line 1"),
    ] {
        let forged = vec![tainted[0].clone(), with(&tainted[1], path, &value)];
        cases.push((forged, Verdict::Refused { line: 2, n: 34, expected }));
    }
    verifiers_agree(&TempDir::new("forged-repeats-files"), &cases);

    // What only the Rust verifier can check, since it sets the two search problems up again.  The setups of 64 and 65
    // both say that their problem has one more promotion to spare than it has, so the lines agree with each other
    // and the problem set up again is not the one they record.  And a repeat said to repeat an earlier game that
    // promotes what it does and whose setup is as large as its own, but poses another problem, with that game's
    // recorded digest made the repeat's: the problems differ.
    // The Python referee accepts both: it can only see that the lines agree with each other.
    let larger = Json::from(max_total + 1);
    let resized_both: Vec<Json> =
        lines[..i - 1].iter().cloned().chain([i - 1, i].map(|k| with(&lines[k], "setup.max_total", &larger))).collect();

    let (k, impostor) = games
        .iter()
        .enumerate()
        .find_map(|(k, r)| {
            let (same, setup) = (r.same_search.as_ref()?, r.setup?);
            let alike = |g: &&GameResult| {
                g.n != same.n
                    && g.proof.is_some_and(Proof::settles_search)
                    && g.promotions == r.promotions
                    && g.setup.is_some_and(|s| s.size() == setup.size() && s.digest != setup.digest)
            };
            Some((k, games[..k].iter().rev().find(alike)?))
        })
        .expect("a repeat and an earlier game alike in all but the problem");
    let mut posing = lines[..=k].to_vec();
    let (repeat_digest, at) = (digest(&games[k]), impostor.n as usize - 1);
    posing[at] = with(&lines[at], "setup.digest", &repeat_digest);
    let named = [("same_search.n", Json::from(impostor.n)), ("same_search.proof", s(proof(impostor)))];
    posing[k] = named.iter().fold(lines[k].clone(), |line, (path, value)| with(&line, path, value));
    let posed = format!("its search problem is not the one game {} (line {}) posed", impostor.n, impostor.n);

    let dir = TempDir::new("forged-repeats-rust-only");
    let mut paths = Vec::new();
    let only_rust = [(resized_both, 65, "not the ProblemSize".to_owned()), (posing, games[k].n, posed)];
    for (case, (file, n, expected)) in only_rust.into_iter().enumerate() {
        let path = dir.join(format!("rust-only-{case}.jsonl"));
        fs::write(&path, file.iter().map(|line| format!("{line}\n")).collect::<String>()).unwrap();
        let err = verify_files(std::slice::from_ref(&path), Some(&optimal)).unwrap_err();
        assert!(err.contains(&format!("line {n} (n={n}): ")) && err.contains(&expected), "{case}: {err}");
        paths.push(path);
    }
    let run = Command::new("python3").arg("-I").arg(crate_path("tools/check_results.py")).args(&paths).output();
    let run = run.expect("python3 runs tools/check_results.py");
    assert_eq!(run.status.code(), Some(0), "{}", String::from_utf8_lossy(&run.stdout));
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
    assert_eq!(names, ["strict.jsonl"], "a new results file needs its counts here and in the README");

    let optimal = Optimal::load_default().unwrap();
    let report = verify_files(&paths, Some(&optimal)).unwrap_or_else(|e| panic!("{e}"));
    println!("{report}");
    let [strict] = &report.files[..] else { panic!("one file") };
    // strict.jsonl is a chain from n = 1 made in one invocation: it holds no line of a stopped search.
    let games = &strict.contents.games;
    assert!(games.keys().copied().eq(1..=1300), "{}: not the games 1..=1300", strict.path.display());
    assert_eq!((strict.contents.incomplete_lines, strict.contents.torn), (0, false), "{}", strict.path.display());
    assert!(strict.contents.runs.is_empty(), "{}: a game left unfinished", strict.path.display());
    // The chain from n = 1 reproduces optimal.json: its scores to 1000 are the Java's.
    for n in 1..=1000 {
        assert_eq!(Some(strict.contents.games[&n].score), optimal.score(n), "n={n}");
    }
    // The games the table holds past 1000 were copied from it (`taxman extend-optimal`), move for move.
    for (n, score, moves) in optimal.games().filter(|&(n, _, _)| n > 1000) {
        let game =
            strict.contents.games.get(&n).unwrap_or_else(|| panic!("optimal.json has a game of {n}, strict.jsonl not"));
        assert_eq!((score, moves, game.taint.is_empty()), (game.score, &game.moves[..], true), "n={n}");
    }

    // No unproven tactic, and nothing unproven anywhere below the chain.
    let counts = [Proof::Trivial, Proof::PrimeFormula, Proof::Cap, Proof::Exhausted, Proof::SameSearch]
        .map(|p| proofs(strict, p));
    assert_eq!(counts, [13, 211, 115, 416, 545]);
    assert_eq!(clean(strict), 1300);
    // Scores at a proven ceiling given the game of n − 1 (what certifies a game whatever found it).
    let ceilings = [Certificate::NPlusPrev, Certificate::PrimeFormula].map(|c| certificates(strict, c));
    assert_eq!(ceilings, [424, 210]);
}
