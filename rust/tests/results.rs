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
use taxman::results::{moves_digest, verify_files, Certificate, FileReport, Proof, OPTIMAL_JSON};
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
    let some_setup = Json::object([
        ("frames", Json::from(1u32)),
        ("candidates", Json::from(1u32)),
        ("max_total", Json::from(1u32)),
        ("cap", Json::Null),
        ("revise", s("Off")),
        ("digest", s("0123456789abcdef")),
    ]);
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
/// builds on it.  (Searched: with `same_search` on, 82 would repeat 81, whose problem is 82's, frame 0 and all.)
/// Both verifiers hold a line to the rule of its schema: schema 2 counted the game's own taint in a proof by the cap
/// too.
#[test]
fn a_proof_by_the_cap_rests_on_nothing_the_game_used_itself() {
    let dir = TempDir::new("cap-taint");
    let results = dir.join("chain.jsonl");
    let strict = common::strict_options(1);
    let mut revising = strict.clone();
    revising.tactics.set("revise_freedoms=on").unwrap();
    revising.tactics.set("same_search=off").unwrap();
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

/// A repeat (method `repeat`, proof `same_search`) stands on the game it names: both verifiers refuse one whose
/// record, digests, promotions or arithmetic do not fit that game, whose game is not a completed game of the file
/// before it, settles no search or comes after it, and one that drops the taint of the game it repeats.  The Rust
/// verifier also sets both search problems up again and compares them, which refuses two more that the Python
/// referee, which cannot, accepts.  Games 1..90 of a strict chain, in which 65 repeats 64, a search.
#[test]
fn forged_repeats_are_refused_by_both_verifiers() {
    let games = solver::solve_range(1, 90, &common::strict_options(1), None).unwrap();
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
        (i, vec![("setup.revise", s("ResumedAsBuilt"))], "a repeat game whose \"setup.revise\" is ResumedAsBuilt"),
        (i, vec![("same_search", Json::Null)], "without a \"same_search\" record"),
        (i, vec![("schema", Json::from(3u32))], "a repeat game in a line of schema 3"),
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
    let allowed = SolveOptions { allow_tainted_prev: true, ..common::strict_options(1) };
    let tainted: Vec<Json> =
        solver::solve_range(33, 34, &allowed, None).unwrap().iter().map(GameResult::to_json).collect();
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

    // What only the Rust verifier can check, since it sets the two search problems up again.  64's frame 0 said to
    // be revised, with 64's taint and 65's records of it made to fit: 64's problem set up again is smaller than its
    // line says.  And a repeat said to repeat an earlier game that promotes what it does and whose setup is as large
    // as its own, but poses another problem, with that game's recorded digest made the repeat's: the problems differ.
    // The Python referee accepts both: it can only see that the lines agree with each other.
    let taint = if m.proof == Some(Proof::Cap) { none } else { Json::array(["revise_freedoms"]) };
    let revised = [
        ("tactic_switches.revise_freedoms", Json::Bool(true)),
        ("setup.revise", s("GreedyIsMax(0)")),
        ("taint", taint.clone()),
    ];
    let revised = revised.iter().fold(lines[i - 1].clone(), |line, (path, value)| with(&line, path, value));
    let fitted = ["same_search.taint", "taint", "prev.taint"].iter().fold(lines[i].clone(), |l, p| with(&l, p, &taint));
    let revised: Vec<Json> = lines[..i - 1].iter().cloned().chain([revised, fitted]).collect();

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
    let only_rust = [(revised, 65, "not the ProblemSize".to_owned()), (posing, games[k].n, posed)];
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
