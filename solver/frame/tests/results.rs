//! The committed results pass the checks that `taxman verify-results` runs (see `taxman::results`): every game
//! replays legally to its recorded score, is over and opens with the largest prime, keeps to the chain from
//! n − 1, and the arithmetic and provenance of its line hold; the files agree on every game they share.  The
//! counts asserted here are the ones `results/README.md` quotes.  And both verifiers, `verify-results` and the
//! Python referee, refuse lines that a hand edit made contradict themselves, and derive assumptions by the same
//! rules.

mod common;

use common::TempDir;
use std::fs;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::process::Command;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::known::{Known, KnownSources};
use taxman::results::{
    assumed_optimal, moves_digest, verify_files, verify_files_with, Assumptions, CeilingProof, FileReport, Proof,
    StoredGame,
};
use taxman::solver::{self, GameResult, Method, SolveOptions};

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

/// The Python referee `solver/tools/check_results.py` on the files `paths`.  Their lines cite no known source, so it is
/// given none (a path that is not there): by default it reads the committed chain and the bounds store, 0.4 s each
/// time.
fn python_referee(paths: &[PathBuf]) -> std::process::Output {
    let none = "/nonexistent/none.jsonl";
    let mut command = Command::new("python3");
    command.arg("-I").arg(crate_path("../tools/check_results.py")).args(["--committed", none, "--bounds", none]);
    command.args(paths).output().expect("python3 runs solver/tools/check_results.py")
}

/// A change to a line: the field at a path (as [`with`] takes it) and its new value.
type Edit<'a> = (&'a str, Json);

/// What both verifiers must say of a results file: accept it, or refuse it naming a line (counting from 1) and its
/// n, with a part of the Rust verifier's message.
enum Verdict<'a> {
    Accepted,
    Refused { line: usize, n: u32, expected: &'a str },
}

/// Runs both verifiers, `verify_files` (on each file alone) and the Python referee `solver/tools/check_results.py` (on
/// all of them at once), on files made of the lines of each case, and checks that each gives its verdict.
fn verifiers_agree(dir: &TempDir, cases: &[(Vec<Json>, Verdict)]) {
    let optimal = Optimal::built_in();
    let mut paths = Vec::new();
    for (k, (lines, verdict)) in cases.iter().enumerate() {
        let path = dir.join(format!("case-{k:02}.jsonl"));
        fs::write(&path, lines.iter().map(|line| format!("{line}\n")).collect::<String>()).unwrap();
        let verified = verify_files(std::slice::from_ref(&path), optimal, 1);
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

    let run = python_referee(&paths);
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
/// or the assumptions derived from a line would change, are refused by both verifiers, naming the line, and so is a
/// line of another schema.  Each file is games 1 to the tampered one.
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
        // Only schema 5 is read.
        (exhausted, vec![("schema", Json::from(4u32))], "older than the schema 5"),
        (exhausted, vec![("schema", Json::Null)], "bad \"schema\""),
        // A proof by `cap` passed off as an exhausted search, which would not inherit what n − 1 assumes.
        (capped, vec![("reached_cap", Json::Bool(false)), ("proof", s("exhausted"))], "records not reaching"),
        (prime, vec![("setup", some_setup)], "a prime game with a \"setup\""),
        (prime, vec![("in_the_bag", Json::from(0u64))], "a prime game with \"in_the_bag\""),
        (prime, vec![("reached_cap", Json::Bool(true))], "reached a cap"),
        // The prime formula proves only a prime n.
        (trivial, vec![("method", s("prime")), ("in_the_bag", Json::Null)], "is not prime"),
        (trivial, vec![("promotions", smallest_move)], "a trivial game with promotions"),
        // Only a search counts the nodes its near-repeat cut cut, and in a number.
        (prime, vec![("stats.near_repeat_cut", Json::from(0u64))], "which only a search has"),
        (exhausted, vec![("stats.near_repeat_cut", s("many"))], "bad \"stats.near_repeat_cut\""),
        (exhausted, vec![("stats.near_repeat_cut", Json::from(-1i64))], "bad \"stats.near_repeat_cut\""),
    ];
    let accepted = [(lines.len() - 1, vec![]), (exhausted, vec![("stats.near_repeat_cut", Json::from(3u64))])];
    both_verifiers(&TempDir::new("contradictions"), &lines, &accepted, &refused);
}

/// A search whose starting game reaches the ceiling before the search takes a step has the method `start`, with the
/// proof `cap` and no nodes: game 18's greedy set already scores `n + score(n − 1)`.  Games 20 and 21 run their trees
/// and stay `search`.  Both verifiers read the file.  The line of an older build, which called game 18 a `search`
/// with no node, still reads in the Rust verifier, which guards what could change a proof and not the label; the
/// Python referee refuses it, and a `start` line that took a node or did not finish.
#[test]
fn a_search_whose_starting_game_reaches_the_ceiling_is_a_start_game() {
    let games = solver::solve_range(1, 21, &common::options(1), None).unwrap();
    let start = &games[17];
    assert_eq!((start.n, start.method, start.proof, start.nodes), (18, Method::Start, Some(Proof::Cap), 0));
    assert!(start.completed && start.reached_cap && start.setup.is_some());
    for ran in &games[19..] {
        assert_eq!((ran.method, ran.proof), (Method::Search, Some(Proof::Exhausted)), "n={}", ran.n);
        assert!(ran.nodes > 0, "n={}", ran.n);
    }
    let lines: Vec<Json> = games.iter().map(GameResult::to_json).collect();
    assert_eq!(lines[17].get("method").and_then(Json::as_str).and_then(Method::parse), Some(Method::Start));

    let s = |text: &str| Json::from(text);
    let file = |last: Json| -> Vec<Json> { lines[..17].iter().cloned().chain([last]).collect() };
    let old_build = file(with(&lines[17], "method", &s("search")));
    let stepped = file(with(&lines[17], "nodes", &Json::from(1u32)));
    let unfinished = file(stopped_at(&lines[17]));
    let dir = TempDir::new("start-game");
    let write = |name: &str, file: &[Json]| {
        let path = dir.join(name);
        fs::write(&path, file.iter().map(|line| format!("{line}\n")).collect::<String>()).unwrap();
        path
    };
    let ok = write("ok.jsonl", &lines[..18]);
    let (old, stepping) = (write("old.jsonl", &old_build), write("stepping.jsonl", &stepped));
    let unending = write("unending.jsonl", &unfinished);
    for path in [&ok, &old, &stepping] {
        verify_files(std::slice::from_ref(path), Optimal::built_in(), 1).unwrap_or_else(|e| panic!("{e}"));
    }
    let run = python_referee(&[ok, old, stepping, unending]);
    let report = String::from_utf8_lossy(&run.stdout);
    assert_eq!(run.status.code(), Some(1), "{report}");
    assert!(report.contains("3 PROBLEMS"), "{report}");
    for problem in [
        "old.jsonl line 18 (n=18): a completed search with nodes 0",
        "stepping.jsonl line 18 (n=18): a start game with nodes 1",
        "unending.jsonl line 18 (n=18): a start game that is not completed",
    ] {
        assert!(report.contains(problem), "{problem}\n{report}");
    }
}

/// A search armed with the near-repeat cut counts the nodes it cut in its line, as `stats.near_repeat_cut`, and the
/// proof stays `exhausted`: the search still explores its whole tree, with one more cut.  Game 256, searched after 255
/// in the same run, has one node cut.  Game 21, searched after 20, is armed and cuts none.  The first game of each run
/// follows no game of the run, so it is not armed and its line has no such key; nor has the line of 256 without the
/// tactic.  Both verifiers accept the files, and both reports count the search that the cut shortened.
#[test]
fn the_line_of_a_search_armed_with_the_near_repeat_cut_counts_the_nodes_it_cut() {
    let dir = TempDir::new("near-repeat-cut");
    let count = |line: &Json| line.get("stats").and_then(|stats| stats.get("near_repeat_cut")).and_then(Json::as_u64);
    let on = common::options(1);
    let mut off = on.clone();
    off.tactics.search.near_repeat = false;
    // (the file's name, its first and last game, the options, the nodes cut in the last game's search)
    let runs = [("cut", 255, 256, &on, Some(1)), ("armed", 20, 21, &on, Some(0)), ("off", 255, 256, &off, None)];
    let mut paths = Vec::new();
    for (name, first, last, opts, expected) in runs {
        let path = dir.join(format!("{name}.jsonl"));
        let games = solver::solve_range(first, last, opts, Some(&path)).unwrap();
        for game in &games {
            assert_eq!((game.method, game.proof), (Method::Search, Some(Proof::Exhausted)), "{name}: n={}", game.n);
            assert!(game.nodes > 0, "{name}: n={}", game.n);
        }
        let text = fs::read_to_string(&path).unwrap();
        let lines: Vec<Json> = text.lines().map(|line| Json::parse(line).unwrap()).collect();
        let counted: Vec<Option<u64>> = lines.iter().map(count).collect();
        assert_eq!(counted, [None, expected], "{name}: the key of each line of the file");
        assert_eq!(games.iter().map(|game| game.near_repeat_cut).collect::<Vec<_>>(), [None, expected], "{name}");
        // The key comes beside the counters, which every line has.
        for line in &lines {
            assert!(line.get("stats").and_then(|stats| stats.get("match_bounds")).is_some(), "{name}: {line}");
        }
        let report = verify_files(std::slice::from_ref(&path), Optimal::built_in(), 1).unwrap().to_string();
        let shortened = if expected > Some(0) { "exhausted 1" } else { "none" };
        let line = format!("searches shortened by the near-repeat cut, by proof: {shortened}\n");
        assert!(report.contains(&line), "{name}: {report}");
        paths.push(path);
    }
    let run = python_referee(&paths);
    let report = String::from_utf8_lossy(&run.stdout);
    assert_eq!(run.status.code(), Some(0), "{report}");
    let said = |shortened: &str| report.matches(&format!("by proof: {shortened}\n")).count();
    assert_eq!((said("exhausted 1"), said("none")), (1, 2), "{report}");
}

/// A proof assumes what the game it rests on assumes, and nothing else.  Games 33 to 50, the first built on
/// `optimal.json`'s game of 32 (`--use-prev`): 33 reaches its ceiling and 34 repeats 33, so both assume that game
/// optimal; 35 exhausts its search, which rests on no game, so it and the games after it that rest on it assume
/// nothing.  Both verifiers refuse a line that records a proof by `cap` without the assumption, an exhausted search
/// with it, or a prime game built on a game that assumes nothing with it.
#[test]
fn a_proof_assumes_what_the_game_it_rests_on_assumes_and_nothing_else() {
    let opts = SolveOptions { use_prev: true, ..common::options(1) };
    let games = solver::solve_range(33, 50, &opts, None).unwrap();
    let lines: Vec<Json> = games.iter().map(GameResult::to_json).collect();
    let rests_on_optimal = Assumptions::from([assumed_optimal(32)]);
    let expected = [
        (33, Proof::Cap, &rests_on_optimal),
        (34, Proof::SameSearch, &rests_on_optimal),
        (35, Proof::Exhausted, &Assumptions::new()),
        (37, Proof::PrimeFormula, &Assumptions::new()),
    ];
    for (n, proof, assumes) in expected {
        let game = &games[n - 33];
        assert_eq!((game.n as usize, game.proof, &game.assumes), (n, Some(proof), assumes));
    }
    let (rests, free) = (Json::array([assumed_optimal(32)]), Json::array(Vec::<&str>::new()));
    let cut = |i: usize, assumes: &Json| -> Vec<Json> {
        lines[..i].iter().cloned().chain([with(&lines[i], "assumes", assumes)]).collect()
    };
    let refused = |i: usize, assumes: &Json| {
        (cut(i, assumes), Verdict::Refused { line: i + 1, n: 33 + i as u32, expected: "records the assumptions" })
    };
    let cases = vec![
        (lines.clone(), Verdict::Accepted),
        refused(0, &free),
        refused(1, &free),
        refused(2, &rests),
        refused(4, &rests),
    ];
    verifiers_agree(&TempDir::new("assumption-rules"), &cases);
}

/// A repeat (method `repeat`, proof `same_search`) stands on the game it names: both verifiers refuse one whose
/// record, digests, promotions or arithmetic do not fit that game, whose game is not a completed game of the file
/// before it, settles no search or comes after it, and one that drops what the game it repeats assumes.  The Rust
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
        let found = games[..i - 1]
            .iter()
            .rev()
            .find(|g| matches!(g.method, Method::Start | Method::Search) && (g.promotions == m.promotions) == same);
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
    let optimal = Optimal::built_in();
    let unfinished = Json::object([
        ("schema", Json::from(taxman::results::SCHEMA)),
        ("n", Json::from(64u32)),
        ("completed", Json::Bool(false)),
        ("invocations", Json::from(1u32)),
    ]);
    let from_optimal = [
        ("prev.source", s("optimal")),
        ("prev.moves_digest", s(&moves_digest(optimal.moves(64).unwrap()))),
        ("prev.proof", Json::Null),
        ("prev.assumes", Json::array([assumed_optimal(64)])),
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
    // A repeat assumes what the game it repeats assumes: 34 repeats 33, which, built on optimal.json's 32, reaches
    // its ceiling and so assumes that game optimal.
    let use_prev = SolveOptions { use_prev: true, ..common::options(1) };
    let assuming: Vec<Json> =
        solver::solve_range(33, 34, &use_prev, None).unwrap().iter().map(GameResult::to_json).collect();
    assert_eq!(assuming[1].get("assumes"), Some(&Json::array([assumed_optimal(32)])));
    cases.push((assuming.clone(), Verdict::Accepted));
    for (path, value, expected) in [
        ("assumes", none.clone(), "records the assumptions"),
        ("same_search.assumes", none.clone(), "its record of game 33 does not match line 1"),
    ] {
        let forged = vec![assuming[0].clone(), with(&assuming[1], path, &value)];
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
        let err = verify_files(std::slice::from_ref(&path), optimal, 1).unwrap_err();
        assert!(err.contains(&format!("line {n} (n={n}): ")) && err.contains(&expected), "{case}: {err}");
        paths.push(path);
    }
    let run = python_referee(&paths);
    assert_eq!(run.status.code(), Some(0), "{}", String::from_utf8_lossy(&run.stdout));
}

/// `line`, a completed line, as the incomplete line of a search that a limit stopped at that game.
fn stopped_at(line: &Json) -> Json {
    let best = line.get("score").unwrap().clone();
    let edits = [
        ("completed", Json::Bool(false)),
        ("score", Json::Null),
        ("best_so_far", best),
        ("proof", Json::Null),
        ("assumes", Json::Null),
    ];
    edits.iter().fold(line.clone(), |line, (path, value)| with(&line, path, value))
}

/// A file with holes, and unfinished searches, passes both verifiers when each line stands against what it rests on in
/// the file: a game built on the best game of an unfinished search (`solve --keep-going`) has no ceiling and no proof
/// that rests on it, and names an earlier incomplete line that records that game.  The lines that claim otherwise are
/// refused by both, naming the line.
#[test]
fn games_built_on_unfinished_searches_are_checked_by_both_verifiers() {
    let dir = TempDir::new("unfinished-prev");
    // 598..602 stop at a node limit, each on the best game of the one before; 603 finishes on 602's.
    let results = dir.join("keep-going.jsonl");
    let stopping =
        SolveOptions { node_limit: Some(200), force_search: true, keep_going: true, ..common::matching_options(1) };
    solver::solve_range(598, 602, &stopping, Some(&results)).unwrap();
    let finishing = SolveOptions { node_limit: None, force_search: false, ..stopping.clone() };
    solver::solve_range(598, 603, &finishing, Some(&results)).unwrap();
    let kept: Vec<Json> =
        fs::read_to_string(&results).unwrap().lines().map(|line| Json::parse(line).unwrap()).collect();
    assert_eq!(kept.len(), 6);
    let (finished, s) = (&kept[5], |text: &str| Json::from(text));
    assert_eq!(finished.get("prev").and_then(|p| p.get("proof")), Some(&Json::Null));

    let none = Json::array(Vec::<u32>::new());
    let zeros = s("0000000000000000");
    let refused = |edits: &[(&str, Json)], expected: &'static str| {
        let line = edits.iter().fold(finished.clone(), |line, (path, value)| with(&line, path, value));
        let file: Vec<Json> = kept[..5].iter().cloned().chain([line]).collect();
        (file, Verdict::Refused { line: 6, n: 603, expected })
    };
    let mut cases = vec![(kept.clone(), Verdict::Accepted)];
    // A ceiling (the line says its search had one) needs a proven game of n − 1.
    cases.push(refused(&[("setup.cap", Json::from(1_000_000u32))], "needs a proven game"));
    // The game it names is not an earlier line's: another score, other moves.
    let other_score = Json::from(finished.get("prev").and_then(|p| p.get("score")).and_then(Json::as_u64).unwrap() + 1);
    cases.push(refused(&[("prev.score", other_score)], "no earlier line"));
    cases.push(refused(&[("prev.moves_digest", zeros)], "no earlier line"));
    // An unproven game assumes nothing.
    cases.push(refused(&[("prev.assumes", Json::array(["a statement"]))], "does not match"));
    // The incomplete line itself: its game must score what it says.
    let worse = Json::from(kept[2].get("best_so_far").and_then(Json::as_u64).unwrap() + 1);
    let mut lying = kept.clone();
    lying[2] = with(&kept[2], "best_so_far", &worse);
    cases.push((lying, Verdict::Refused { line: 3, n: 600, expected: "best_so_far" }));
    // The line the game names comes after it.
    let mut swapped = kept.clone();
    swapped.swap(4, 5);
    cases.push((swapped, Verdict::Refused { line: 5, n: 603, expected: "no earlier line" }));
    // An incomplete line with no game in it (made by hand) is only counted.
    let minimal = Json::object([
        ("schema", Json::from(taxman::results::SCHEMA)),
        ("n", Json::from(700u32)),
        ("completed", Json::Bool(false)),
        ("invocations", Json::from(1u32)),
    ]);
    cases.push(([kept.clone(), vec![minimal]].concat(), Verdict::Accepted));
    verifiers_agree(&dir, &cases);

    // A proof that rests on the game of n − 1 cannot rest on an unfinished one: a prime game of a chain from 1, with the
    // game before it made an unfinished line, and its record of that game made the unfinished game's.
    let games = solver::solve_range(1, 90, &common::options(1), None).unwrap();
    let lines: Vec<Json> = games.iter().map(GameResult::to_json).collect();
    let i = games.iter().position(|g| g.method == Method::Prime && g.n > 40).expect("a prime game in 41..=90");
    let unproven = [("prev.proof", Json::Null), ("prev.assumes", none)];
    let built = unproven.iter().fold(lines[i].clone(), |line, (path, value)| with(&line, path, value));
    let prime_on_unfinished: Vec<Json> =
        lines[..i - 1].iter().cloned().chain([stopped_at(&lines[i - 1]), built]).collect();
    let n = games[i].n;
    verifiers_agree(
        &dir,
        &[(prime_on_unfinished, Verdict::Refused { line: i + 1, n, expected: "rests on the game of n − 1, which" })],
    );
    // And a game that rests on nothing of n − 1 stands on an unfinished one: the same game with the proof its
    // search would have given needs a search, so only the file of the real run above shows it; here, the unfinished
    // line alone, in a file whose games before it are complete, is a file with a hole.
    let hole: Vec<Json> = lines[..i - 1].iter().cloned().chain([stopped_at(&lines[i - 1])]).collect();
    verifiers_agree(&dir, &[(hole, Verdict::Accepted)]);
}

/// A record of the proof `known` stands on the file's own line of the game before, whatever proof that line has.  A run
/// with `--use-prev` records the committed chain's games as `known`, so when its lines are appended to the committed
/// file, its first line names a game of n − 1 as `known` while the file's line of that game has the proof it was made
/// with.  Both verifiers accept such a record when the score, the moves and the assumptions are the line's, and refuse
/// it when any of them differs, when the game is not in the file, when it names another source, and a record of any
/// other proof that is not the line's.  Games 1..90.
#[test]
fn a_known_record_of_the_game_before_stands_on_the_files_own_line() {
    let games = solver::solve_range(1, 90, &common::options(1), None).unwrap();
    let lines: Vec<Json> = games.iter().map(GameResult::to_json).collect();
    // The index of the first game built on a game of each proof (the game of n is `games[n - 1]`).
    let after = |proof: Proof| {
        (1..games.len())
            .find(|&i| games[i - 1].proof == Some(proof))
            .unwrap_or_else(|| panic!("no game after a game of proof {} in 1..=90", proof.as_str()))
    };
    let s = |text: &str| Json::from(text);
    let known = || ("prev.proof", s("known"));
    let proofs = [Proof::Exhausted, Proof::Cap, Proof::SameSearch, Proof::PrimeFormula];
    let accepted: Vec<(usize, Vec<Edit>)> = proofs.iter().map(|&proof| (after(proof), vec![known()])).collect();

    let i = after(Proof::Exhausted);
    let before = &games[i - 1];
    let optimal = Optimal::built_in();
    let digest = moves_digest(optimal.moves(before.n).unwrap());
    let from_optimal: Vec<Edit> = vec![
        ("prev.source", s("optimal")),
        ("prev.moves_digest", s(&digest)),
        known(),
        ("prev.assumes", Json::array([assumed_optimal(before.n)])),
    ];
    let refused: Vec<(usize, Vec<Edit>, &str)> = vec![
        (i, vec![known(), ("prev.score", Json::from(before.score + 1))], "does not match line"),
        (i, vec![known(), ("prev.moves_digest", s("0000000000000000"))], "does not match line"),
        (i, vec![known(), ("prev.assumes", Json::array(["a statement"]))], "does not match line"),
        // Another proof than the line's, and than `known`.
        (i, vec![("prev.proof", s("cap"))], "does not match line"),
        // `known` is the record of the file's own game: the game of optimal.json has no proof to leave out.
        (i, from_optimal, "does not match optimal.json"),
    ];
    both_verifiers(&TempDir::new("known-record"), &lines, &accepted, &refused);

    // The game named is not in the file.
    let hole: Vec<Json> = lines[..i - 1].iter().cloned().chain([with(&lines[i], "prev.proof", &s("known"))]).collect();
    let cases = vec![(hole, Verdict::Refused { line: i, n: games[i].n, expected: "which the file does not hold" })];
    verifiers_agree(&TempDir::new("known-record-hole"), &cases);
}

/// How many games of the report in `range` have the proof, the ceiling proof, or no assumption.
fn proofs(file: &FileReport, range: RangeInclusive<u32>, proof: Proof) -> usize {
    file.contents.games.range(range).filter(|(_, g)| g.proof == proof).count()
}

/// How many games of the report in `range` have the proof `exhausted` and were shortened by the near-repeat cut: their
/// lines count nodes it cut (`stats.near_repeat_cut` above 0).
fn shortened(file: &FileReport, range: RangeInclusive<u32>) -> usize {
    let cut = |game: &StoredGame| game.near_repeat_cut.is_some_and(|nodes| nodes > 0);
    file.contents.games.range(range).filter(|(_, game)| game.proof == Proof::Exhausted && cut(game)).count()
}

fn ceiling_proofs(file: &FileReport, ceiling_proof: CeilingProof) -> usize {
    file.contents.games.values().filter(|g| g.ceiling_proof == Some(ceiling_proof)).count()
}

fn clean(file: &FileReport) -> usize {
    file.contents.games.values().filter(|g| g.assumes.is_empty()).count()
}

/// The last game of the committed results; of the run on a laptop that made their first part, which the prover
/// proved on the whole board; of the second run, which made the games past the first, on EC2; and of `optimal.json`,
/// which they reproduce.  The third run, also on EC2, made the games past the second, the fourth the games past the
/// third, the fifth the games past the fourth, and the sixth, on another machine, the games past the fifth.
const LAST: u32 = 4900;
const LAST_OF_FIFTH_RUN: u32 = 4874;
const LAST_OF_FOURTH_RUN: u32 = 4547;
const LAST_OF_THIRD_RUN: u32 = 3914;
const LAST_OF_SECOND_RUN: u32 = 3440;
const LAST_OF_FIRST_RUN: u32 = 2175;
const LAST_OF_TABLE: u32 = 1300;
/// The builds that made the six runs (a prefix of the commit each line records).  The sixth run had three: its stops
/// and restarts took in newer ones, so each stretch of its games, up to the game given, has a build of its own.
const FIRST_BUILD: &str = "feba6d0";
const SECOND_BUILD: &str = "55287ff";
const THIRD_BUILD: &str = "eb84a6f";
const FOURTH_BUILD: &str = "ca357bf";
const FIFTH_BUILD: &str = "4974a71";
const SIXTH_BUILDS: [(u32, &str); 3] = [(4879, "a88dc7f"), (4898, "bcfde26"), (LAST, "551f883")];
/// The games that several invocations solved, and how many: they were in flight when an instance of the second or the
/// fourth run, or the solver of the sixth, was stopped, and the next took them up.
const RESUMED: [(u32, u32); 14] = [
    (3311, 2),
    (3312, 2),
    (3315, 2),
    (3940, 2),
    (3942, 2),
    (4340, 2),
    (4431, 2),
    (4432, 2),
    (4433, 2),
    (4879, 2),
    (4880, 3),
    (4895, 2),
    (4899, 2),
    (4900, 2),
];

/// The games of the committed chain that the quick check verifies: the first of them, which cost little to set up.
const QUICK_LAST: u32 = 400;

/// The games 1..=`last` of the committed chain pass the verifier and the checks that follow, which hold of any prefix
/// of the chain (a file of its first `last` lines is a chain from n = 1 itself); the counts that `results/README.md`
/// quotes, and the last scores, are checked where the games reach them.
fn check_the_committed_results_to(last: u32) {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(crate_path("../../results"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        .collect();
    paths.sort();
    let names: Vec<_> = paths.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    // bounds.jsonl is not a results file (its lines are bounds, not games): the ledger's own tests check it, in
    // examples/ledger/store_tests.rs.
    assert_eq!(names, ["bounds.jsonl", "strict.jsonl"], "a new results file needs its counts here and in the README");
    let strict = paths.into_iter().find(|path| path.ends_with("strict.jsonl")).unwrap();
    let whole = fs::read_to_string(&strict).unwrap();
    let text: String = whole.split_inclusive('\n').take(last as usize).collect();

    // The verifier reads a file, so a prefix is written to one.
    let dir = TempDir::new("committed-prefix");
    paths = vec![strict];
    if last < LAST {
        let prefix = dir.join("strict.jsonl");
        fs::write(&prefix, &text).unwrap();
        paths = vec![prefix];
    }
    let optimal = Optimal::built_in();
    // The games taken from the bounds store cite it and the committed chain, which is the file itself: the sources are
    // the repository's.  (The start of the chain cites none.)
    let sources = if last == LAST { KnownSources::in_repository().unwrap() } else { KnownSources::default() };
    let known = Known::load(optimal, &sources).unwrap_or_else(|e| panic!("{e}"));
    let report = verify_files_with(&paths, &known, 1).unwrap_or_else(|e| panic!("{e}"));
    println!("{report}");
    let [strict] = &report.files[..] else { panic!("one file") };
    // strict.jsonl is a chain from n = 1: it holds no line of a stopped search.
    let games = &strict.contents.games;
    assert!(games.keys().copied().eq(1..=last), "{}: not the games 1..={last}", strict.path.display());
    assert_eq!((strict.contents.incomplete_lines, strict.contents.torn), (0, false), "{}", strict.path.display());
    assert!(strict.contents.runs.is_empty(), "{}: a game left unfinished", strict.path.display());
    let last_scores = [
        (LAST_OF_FIRST_RUN, 1_492_209),
        (LAST_OF_SECOND_RUN, 3_727_726),
        (LAST_OF_THIRD_RUN, 4_820_492),
        (LAST_OF_FOURTH_RUN, 6_498_966),
        (LAST_OF_FIFTH_RUN, 7_473_314),
        (LAST, 7_557_145),
    ];
    for (n, score) in last_scores {
        assert!(n > last || games[&n].score == score, "n={n}");
    }
    // Every line records the one clean build that made its part of the file (the first run's to 2175, the second's to
    // 3440, the third's to 3914, the fourth's to 4547, the fifth's to 4874, the sixth's three after), a single
    // invocation but at the games that the stop of an instance or of the solver interrupted, and the verdict on
    // optimal.json, which has the games to 1300 and none beyond: `optimal_match` is true where it has the game and
    // null where it has not.
    for (k, line) in text.lines().enumerate() {
        let (n, line) = (k as u32 + 1, Json::parse(line).unwrap_or_else(|e| panic!("line {}: {e:?}", k + 1)));
        let build = line.get("build").unwrap_or_else(|| panic!("n={n}: no build"));
        assert_eq!(build.get("dirty"), Some(&Json::Bool(false)), "n={n}");
        let commit = build.get("commit").and_then(Json::as_str).unwrap();
        let expected = if n <= LAST_OF_FIRST_RUN {
            FIRST_BUILD
        } else if n <= LAST_OF_SECOND_RUN {
            SECOND_BUILD
        } else if n <= LAST_OF_THIRD_RUN {
            THIRD_BUILD
        } else if n <= LAST_OF_FOURTH_RUN {
            FOURTH_BUILD
        } else if n <= LAST_OF_FIFTH_RUN {
            FIFTH_BUILD
        } else {
            SIXTH_BUILDS.iter().find(|&&(until, _)| n <= until).expect("a build of the sixth run").1
        };
        assert!(commit.starts_with(expected), "n={n}: built by {commit}, not {expected}");
        let invocations = RESUMED.iter().find(|&&(game, _)| game == n).map_or(1, |&(_, count)| count);
        assert_eq!(line.get("invocations").and_then(Json::as_u32), Some(invocations), "n={n}");
        let matched = if n <= LAST_OF_TABLE { Json::Bool(true) } else { Json::Null };
        assert_eq!(line.get("optimal_match"), Some(&matched), "n={n}");
    }
    // The chain from n = 1 reproduces optimal.json: its scores are the table's, which are the Java's to 1000 and
    // those of an earlier chain of this solver from 1001 to 1300.
    let in_table = 1..=LAST_OF_TABLE.min(last);
    for n in in_table.clone() {
        assert_eq!(Some(strict.contents.games[&n].score), optimal.score(n), "n={n}");
    }
    // And its sets of moves are the table's but at six games, 539, 770 to 773 and 1215, where the chain found another
    // of the several optimal sets (solver/noframe/docs/uniqueness.md lists the exchanges of the
    // first five; it enumerates the sets only to 989).  The previous chain differed from the table at 684 to 687
    // instead.
    let sorted = |moves: &[u32]| {
        let mut moves = moves.to_vec();
        moves.sort_unstable();
        moves
    };
    let other_set: Vec<u32> =
        in_table.filter(|&n| sorted(&strict.contents.games[&n].moves) != sorted(optimal.moves(n).unwrap())).collect();
    let expected: Vec<u32> = [539, 770, 771, 772, 773, 1215].into_iter().filter(|&n| n <= last).collect();
    assert_eq!(other_set, expected);
    // The games the table holds past 1000 were copied from that earlier chain (`taxman extend-optimal`), and this
    // one has them move for move but at 1215.
    for (n, score, moves) in optimal.games().filter(|&(n, _, _)| n > 1000 && n != 1215 && n <= last) {
        let game =
            strict.contents.games.get(&n).unwrap_or_else(|| panic!("optimal.json has a game of {n}, strict.jsonl not"));
        assert_eq!((score, moves, game.assumes.is_empty()), (game.score, &game.moves[..], true), "n={n}");
    }

    // Its proofs, in the order trivial, prime formula, `cap`, exhausted, same search, known (taken from the bounds
    // store, with no search): in all, in the games of the table (where the previous chain, of commit 271c817, had
    // every one of them), and past it, in the first run's, in the second's, in the third's, in the fourth's, in the
    // fifth's and in the sixth's.  The known games, which only the fourth and fifth runs made, assume statement U;
    // every other game assumes nothing.
    let kinds = [Proof::Trivial, Proof::PrimeFormula, Proof::Cap, Proof::Exhausted, Proof::SameSearch, Proof::Known];
    let by = |range: RangeInclusive<u32>| kinds.map(|p| proofs(strict, range.clone(), p));
    assert_eq!(by(1..=last).iter().sum::<usize>(), last as usize, "every game has one of the proofs");
    let from_store = by(1..=last)[5];
    assert_eq!(clean(strict), last as usize - from_store);
    // The chain was made before the near-repeat cut, so the cut shortened none of its exhausted searches: the README's
    // row of them is 0 in every range.
    assert_eq!(shortened(strict, 1..=last), 0);
    if last == LAST {
        assert_eq!(by(1..=LAST), [13, 611, 429, 1551, 1914, 382]);
        assert_eq!(by(1..=LAST_OF_TABLE), [13, 211, 115, 416, 545, 0]);
        assert_eq!(by(LAST_OF_TABLE + 1..=LAST_OF_FIRST_RUN), [0, 115, 77, 314, 369, 0]);
        assert_eq!(by(LAST_OF_FIRST_RUN + 1..=LAST_OF_SECOND_RUN), [0, 155, 119, 448, 543, 0]);
        assert_eq!(by(LAST_OF_SECOND_RUN + 1..=LAST_OF_THIRD_RUN), [0, 60, 43, 178, 193, 0]);
        assert_eq!(by(LAST_OF_THIRD_RUN + 1..=LAST_OF_FOURTH_RUN), [0, 55, 57, 150, 204, 167]);
        assert_eq!(by(LAST_OF_FOURTH_RUN + 1..=LAST_OF_FIFTH_RUN), [0, 13, 14, 35, 50, 215]);
        assert_eq!(by(LAST_OF_FIFTH_RUN + 1..=LAST), [0, 2, 4, 10, 10, 0]);
        // Scores at a proven ceiling given the game of n − 1 (what certifies a game whatever found it).
        let ceilings = [CeilingProof::NPlusPrev, CeilingProof::PrimeFormula].map(|c| ceiling_proofs(strict, c));
        assert_eq!(ceilings, [1586, 653]);
    }
}

/// The first games of the committed chain pass the verifier and the checks of
/// [`the_committed_results_pass_the_verifier`] that hold of a prefix; the counts of the whole file are for that test.
#[test]
fn the_start_of_the_committed_results_passes_the_verifier() {
    check_the_committed_results_to(QUICK_LAST);
}

/// All 4900 games of the committed chain pass the verifier, with the counts `results/README.md` quotes.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is the_start_of_the_committed_results_passes_the_verifier"
)]
fn the_committed_results_pass_the_verifier() {
    check_the_committed_results_to(LAST);
}
