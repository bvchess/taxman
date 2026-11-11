//! The committed results pass the checks that `taxman verify-results` runs (see `taxman::results`): the moves of
//! every line replay legally to its recorded score, are over and open with the largest prime, keep to the chain from
//! n − 1, and the arithmetic and provenance of the line hold; the files agree on every game they share.  No count of
//! the file is asserted here: they are `results/README.md`'s to print.  And both verifiers, `verify-results` and the
//! Python referee, refuse lines that a hand edit made contradict themselves, and derive assumptions by the same
//! rules.

mod common;

use common::TempDir;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::results::{verify_files, Proof};
use taxman::solver::{self, unsound, GameResult, Method};

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

/// The first lines of the committed results that the checks of these tests meet, which the Python referee takes as its
/// table as the Rust verifier takes the built-in one.
const TABLE_GAMES: usize = 400;

/// The Python referee `solver/tools/check_results.py` on the files `paths`, with the lines of the file `table` as its
/// table.
fn python_referee_with(table: &Path, paths: &[PathBuf]) -> std::process::Output {
    let mut command = Command::new("python3");
    command.arg("-I").arg(crate_path("../tools/check_results.py"));
    command.arg("--table").arg(table);
    command.args(paths).output().expect("python3 runs solver/tools/check_results.py")
}

/// The Python referee on the files `paths`.  Its table is the first [`TABLE_GAMES`] lines of the committed results, in
/// a file beside the first of `paths`.
fn python_referee(paths: &[PathBuf]) -> std::process::Output {
    python_referee_with(&common::referee_table(&paths[0], TABLE_GAMES), paths)
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

/// A search whose starting moves reach the ceiling before the search takes a step has the method `start`, with the
/// proof `cap` and no nodes: game 18's greedy set already scores `n + score(n − 1)`.  Games 20 and 21 run their trees
/// and stay `search`.  Both verifiers read the file.  A line that calls game 18 a `search` with no node reads in the
/// Rust verifier, which guards what could change a proof and not the label; the Python referee refuses it, and a
/// `start` line that took a node or did not finish.
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

/// A run that starts from game 1 builds on nothing that is assumed, so no line of it assumes anything: whatever proof
/// a game has (an exhausted search, the ceiling, the prime formula, a repeat), both verifiers refuse a line that
/// records an assumption.  Games 1 to 90.
#[test]
fn a_line_that_records_an_assumption_is_refused_by_both_verifiers() {
    let games = solver::solve_range(1, 90, &common::options(1), None).unwrap();
    let lines: Vec<Json> = games.iter().map(GameResult::to_json).collect();
    assert!(games.iter().all(|game| game.assumes.is_empty()));
    let statement = Json::array(["a statement"]);
    let refused = |proof: Proof| {
        let i = games.iter().position(|game| game.proof == Some(proof)).unwrap_or_else(|| panic!("no {proof:?} game"));
        let cut: Vec<Json> = lines[..i].iter().cloned().chain([with(&lines[i], "assumes", &statement)]).collect();
        (cut, Verdict::Refused { line: i + 1, n: i as u32 + 1, expected: "records the assumptions" })
    };
    let cases = vec![
        (lines.clone(), Verdict::Accepted),
        refused(Proof::Exhausted),
        refused(Proof::Cap),
        refused(Proof::PrimeFormula),
        refused(Proof::SameSearch),
    ];
    verifiers_agree(&TempDir::new("assumption-rules"), &cases);
}

/// A repeat (method `repeat`, proof `same_search`) stands on the game it names: both verifiers refuse one whose
/// record, digests, promotions or arithmetic do not fit that game, whose game is not a completed line of the file
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
    let statement = Json::array(["a statement"]);
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
    // The game repeated is only an unfinished search in the file (and 65 names no game it is built on, so that the
    // repeat is what is refused).
    let optimal = Optimal::built_in();
    let unfinished = Json::object([
        ("schema", Json::from(taxman::results::SCHEMA)),
        ("n", Json::from(64u32)),
        ("completed", Json::Bool(false)),
        ("invocations", Json::from(1u32)),
    ]);
    let nameless = with(&lines[i], "prev", &Json::Null);
    let mut incomplete: Vec<Json> = lines[..i - 1].to_vec();
    incomplete.extend([unfinished, nameless.clone()]);
    cases.push((
        incomplete,
        Verdict::Refused { line: 65, n: 65, expected: "not a completed game of the file below it" },
    ));
    let mut complete = lines[..i].to_vec();
    complete.push(nameless);
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
    // A repeat assumes what the game it repeats assumes, which is nothing here: an assumption in its line, or in its
    // record of the game it repeats, is not that game's.
    for (path, expected) in [
        ("assumes", "records the assumptions"),
        ("same_search.assumes", "its record of game 64 does not match line 64"),
    ] {
        let forged: Vec<Json> = lines[..i].iter().cloned().chain([with(&lines[i], path, &statement)]).collect();
        cases.push((forged, Verdict::Refused { line: 65, n: 65, expected }));
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

/// The incomplete line of a search that a limit or a signal stopped is checked by both verifiers as far as it goes: a
/// file that ends with one passes when its moves score the `best_so_far` it records, and is refused, naming the line,
/// when they do not.  A line with no moves in it (made by hand) is only counted.  Games 1 to 40, then the stop.
#[test]
fn an_incomplete_line_is_checked_as_far_as_it_goes_by_both_verifiers() {
    let games = solver::solve_range(1, 41, &common::options(1), None).unwrap();
    let lines: Vec<Json> = games.iter().map(GameResult::to_json).collect();
    let stopped: Vec<Json> = lines[..40].iter().cloned().chain([stopped_at(&lines[40])]).collect();
    let worse = Json::from(stopped[40].get("best_so_far").and_then(Json::as_u64).unwrap() + 1);
    let mut lying = stopped.clone();
    lying[40] = with(&stopped[40], "best_so_far", &worse);
    let minimal = Json::object([
        ("schema", Json::from(taxman::results::SCHEMA)),
        ("n", Json::from(700u32)),
        ("completed", Json::Bool(false)),
        ("invocations", Json::from(1u32)),
    ]);
    let cases = vec![
        (stopped.clone(), Verdict::Accepted),
        (lying, Verdict::Refused { line: 41, n: 41, expected: "best_so_far" }),
        ([stopped, vec![minimal]].concat(), Verdict::Accepted),
    ];
    verifiers_agree(&TempDir::new("incomplete-line"), &cases);
}

/// An unsound line (`taxman solve --unsound`) is refused by both verifiers, naming it: after the proven lines of a
/// file, and a file of its own, whatever the value of its mark.  Games 1 to 12, then the unsound line of 13.
#[test]
fn an_unsound_line_is_refused_by_both_verifiers() {
    let games = solver::solve_range(1, 12, &common::options(1), None).unwrap();
    let lines: Vec<Json> = games.iter().map(GameResult::to_json).collect();
    let plan = unsound::Plan::default();
    let mut unsound_lines = Vec::new();
    let played = |line: &unsound::Line| unsound_lines.push(line.to_json());
    let files = unsound::Files::with_built_in(None, None);
    unsound::solve_range(13, 13, &common::options(1), &plan, &files, played, |_| ()).unwrap();
    let unsound_line = &unsound_lines[0];
    let expected = "an unsound line";
    let cases = vec![
        (lines.iter().cloned().chain([unsound_line.clone()]).collect(), Verdict::Refused { line: 13, n: 13, expected }),
        (vec![unsound_line.clone()], Verdict::Refused { line: 1, n: 13, expected }),
        (vec![with(unsound_line, "unsound", &Json::Bool(false))], Verdict::Refused { line: 1, n: 13, expected }),
    ];
    verifiers_agree(&TempDir::new("unsound-line"), &cases);
}

/// A line's record of the moves for n − 1 it was built from is the file's own line of that game: both verifiers refuse
/// a record whose score, moves, assumptions or proof are not the line's, and a line built on a game that the file does
/// not hold.  Games 1..90.
#[test]
fn a_lines_record_of_the_game_before_is_the_files_own_line_of_it() {
    let games = solver::solve_range(1, 90, &common::options(1), None).unwrap();
    let lines: Vec<Json> = games.iter().map(GameResult::to_json).collect();
    // The index of the first game built on a game with an exhausted search (the game of n is `games[n - 1]`).
    let i = (1..games.len()).find(|&i| games[i - 1].proof == Some(Proof::Exhausted)).expect("an exhausted game");
    let before = &games[i - 1];
    let refused: Vec<(usize, Vec<Edit>, &str)> = vec![
        (i, vec![("prev.score", Json::from(before.score + 1))], "does not match line"),
        (i, vec![("prev.moves_digest", Json::from("0000000000000000"))], "does not match line"),
        (i, vec![("prev.assumes", Json::array(["a statement"]))], "does not match line"),
        (i, vec![("prev.proof", Json::from("cap"))], "does not match line"),
        (i, vec![("prev.proof", Json::Null)], "does not match line"),
    ];
    both_verifiers(&TempDir::new("prev-record"), &lines, &[(lines.len() - 1, vec![])], &refused);

    // The game named is not in the file.
    let hole: Vec<Json> = lines[..i - 1].iter().cloned().chain([lines[i].clone()]).collect();
    let cases = vec![(hole, Verdict::Refused { line: i, n: games[i].n, expected: "which the file does not hold" })];
    verifiers_agree(&TempDir::new("prev-record-hole"), &cases);
}

/// The games of the committed chain that the quick check verifies: the first of them, which cost little to set up.
const QUICK_LAST: usize = 400;

/// What holds of the committed chain whatever its length, of the first `last` games of it (a file of its first `last`
/// lines is a chain from n = 1 itself) or of all of it (`None`): the verifier accepts it, n runs from 1 with no gap,
/// every line is a completed line, none is left unfinished, and the table built into the library has the file's
/// scores and moves.  The counts of the file, by proof and by range, are `results/README.md`'s to print
/// (`taxman verify-results results/optimal.jsonl.gz`), and no test fixes them.
fn check_the_committed_results(last: Option<usize>) {
    // The verifier reads a file: all of the committed one is read through the gzip decoder, as it is by the command, and
    // a prefix is written to a file of its own.
    let dir = TempDir::new("committed-prefix");
    let path = match last {
        None => common::committed_results(),
        Some(last) => {
            let prefix = dir.join("committed-prefix.jsonl");
            fs::write(&prefix, common::committed_text(last)).unwrap();
            prefix
        }
    };
    let optimal = Optimal::built_in();
    let report = verify_files(std::slice::from_ref(&path), optimal, 1).unwrap_or_else(|e| panic!("{e}"));
    println!("{report}");
    let [file] = &report.files[..] else { panic!("one file") };
    let games = &file.contents.games;
    let top = games.keys().next_back().copied().unwrap_or(0) as usize;
    assert!(games.keys().copied().eq(1..=top as u32), "{}: the games are not 1..={top}", file.path.display());
    if let Some(last) = last {
        assert_eq!(top, last, "{}", file.path.display());
    }
    // The chain from n = 1 holds no line of a stopped search.
    assert_eq!((file.contents.incomplete_lines, file.contents.torn), (0, false), "{}", file.path.display());
    assert!(file.contents.runs.is_empty(), "{}: a game left unfinished", file.path.display());

    // The table is the file's moves, as the library reads them.
    for (&n, game) in games {
        assert_eq!((optimal.score(n), optimal.moves(n)), (Some(game.score), Some(&game.moves[..])), "n={n}");
    }
}

/// The first games of the committed chain pass the verifier and the checks of
/// [`the_committed_results_pass_the_verifier`], which hold of a prefix.
#[test]
fn the_start_of_the_committed_results_passes_the_verifier() {
    check_the_committed_results(Some(QUICK_LAST));
}

/// All the games of the committed chain pass the verifier.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is the_start_of_the_committed_results_passes_the_verifier"
)]
fn the_committed_results_pass_the_verifier() {
    check_the_committed_results(None);
}

/// The games that `results/beyond-the-chain.jsonl` holds past the end of the committed chain pass the verifier: every
/// line is a completed line, none is the line of a stopped search or torn, each game is there once, and the table built
/// into the library holds none of them (a game of the committed chain never goes in the file).  A line that rests on an
/// earlier game is checked against that game's line in the file.  The file's counts are `results/README.md`'s to print
/// (`taxman verify-results results/beyond-the-chain.jsonl`), and no test fixes them.
#[test]
fn the_games_beyond_the_chain_pass_the_verifier() {
    let path = common::beyond_the_chain();
    let optimal = Optimal::built_in();
    let report = verify_files(std::slice::from_ref(&path), optimal, 1).unwrap_or_else(|e| panic!("{e}"));
    println!("{report}");
    let [file] = &report.files[..] else { panic!("one file") };
    let contents = &file.contents;
    let shown = file.path.display();
    // The verifier refuses a game that two completed lines give, so each game is there once if every line is the
    // completed line of a game: none is blank or unfinished, and the file does not end in a torn line.
    assert!(!contents.games.is_empty(), "{shown}: no game");
    assert_eq!(contents.lines, contents.games.len(), "{shown}: a line that is not the line of a game");
    assert_eq!((contents.incomplete_lines, contents.torn), (0, false), "{shown}");
    assert!(contents.runs.is_empty(), "{shown}: a game left unfinished");
    for &n in contents.games.keys() {
        assert_eq!(optimal.score(n), None, "{shown}: n={n} is a game of the committed chain");
    }
}

/// The Python referee passes `results/beyond-the-chain.jsonl` too, as the other verifier does
/// ([`the_games_beyond_the_chain_pass_the_verifier`]).  It is a test of its own, so that each stays inside the quick
/// check's 3 s of CPU.
#[test]
fn the_games_beyond_the_chain_pass_the_python_referee() {
    let dir = TempDir::new("beyond-the-chain");
    // The table is a file of the temporary directory: a file written in `results/` would be a results file there.
    let table = common::referee_table(&dir.join("beyond-the-chain.jsonl"), TABLE_GAMES);
    let run = python_referee_with(&table, &[common::beyond_the_chain()]);
    assert_eq!(run.status.code(), Some(0), "{}", String::from_utf8_lossy(&run.stdout));
}
