//! The known games `taxman solve --use-prev` draws on (`src/known.rs`): `optimal.json`, the committed chain and the
//! bounds store.  A board that a source has proven is recorded without a search, citing the source and carrying what it
//! assumes; the game of `n − 1` from a source gives the ceiling and the seed; the store's open boards seed their
//! searches and give them a ceiling.  Both verifiers of results files check the citations against the sources, and refuse a store line
//! that is missing, changed, or not the one cited.  The stores here are made by hand, for boards 900..960 and the like,
//! with the truth's games (`optimal.json`'s) as their lines; the committed chain is a small one made for the purpose.

mod common;

use common::{python, TempDir};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use taxman::game::{complete, replay_finished, Optimal};
use taxman::json::Json;
use taxman::known::{Cite, Known, KnownSources, LineRef, Origin};
use taxman::results::{assumed_optimal, assumed_u, verify_files_with, Assumptions, Proof};
use taxman::solver::{self, GameResult, Method, PrevSource, SolveOptions};
use taxman::store::{Rule, Side, Store};

/// The truth: `optimal.json`'s game of `n`.
fn truth(n: u32) -> (u64, Vec<u32>) {
    let table = Optimal::built_in();
    (table.score(n).expect("a game of the table"), table.moves(n).expect("a game of the table").to_vec())
}

/// A table with the one game `n` of optimal.json: the table a run confirms its scores against, so that the other
/// boards of its range are known to nothing but the sources it is given.
fn table_of(games: &[u32]) -> Optimal {
    let mut table = Optimal::from_json(&Json::Arr(Vec::new())).unwrap();
    for &n in games {
        let (score, moves) = truth(n);
        table.push(n, score, moves).unwrap();
    }
    table
}

fn line(n: u32, side: &str, value: u64, rule: &str, rest: &[(&str, Json)]) -> Json {
    let mut pairs = vec![
        ("schema", Json::from(1u32)),
        ("n", Json::from(n)),
        ("side", Json::from(side)),
        ("value", Json::from(value)),
        ("rule", Json::from(rule)),
    ];
    pairs.extend(rest.iter().map(|(key, value)| (*key, value.clone())));
    pairs.extend([("source", Json::from("test-run")), ("date", Json::from("2026-10-18"))]);
    Json::object(pairs)
}

/// The two lines of a board the store has proven with the truth's game: a lower bound by `given` with its moves, and
/// an upper bound by `given` that rests on the board itself.
fn proven_lines(n: u32) -> Vec<Json> {
    let (score, moves) = truth(n);
    vec![
        line(n, "lower", score, "given", &[("moves", Json::array(moves))]),
        line(n, "upper", score, "given", &[("rests_on", Json::array([n]))]),
    ]
}

/// `line` with the field `key` set to `value`: replaced if the line has it, else added.
fn with(line: &Json, key: &str, value: Json) -> Json {
    let Json::Obj(pairs) = line else { panic!("not an object") };
    let mut pairs: Vec<(String, Json)> = pairs.clone();
    match pairs.iter_mut().find(|(k, _)| k == key) {
        Some(pair) => pair.1 = value,
        None => pairs.push((key.to_owned(), value)),
    }
    Json::Obj(pairs)
}

fn write_lines(path: &Path, lines: &[Json]) {
    fs::write(path, lines.iter().map(|line| format!("{line}\n")).collect::<String>()).unwrap();
}

fn read_lines(path: &Path) -> Vec<Json> {
    fs::read_to_string(path).unwrap().lines().map(|text| Json::parse(text).unwrap()).collect()
}

/// A store of the boards `proven`, each proven with the truth's game.
fn store_of(dir: &TempDir, boards: &[u32]) -> PathBuf {
    let path = dir.join("bounds.jsonl");
    write_lines(&path, &boards.iter().flat_map(|&n| proven_lines(n)).collect::<Vec<_>>());
    path
}

fn sources(committed: Option<&Path>, bounds: Option<&Path>) -> KnownSources {
    KnownSources { committed: committed.map(Path::to_path_buf), bounds: bounds.map(Path::to_path_buf), boards: true }
}

fn options(sources: KnownSources) -> SolveOptions {
    SolveOptions { threads: 1, use_prev: true, known: sources, ..SolveOptions::default() }
}

/// Solves `lo..=hi` into `out` from the sources, with the one-game table of the game before `lo` (so that nothing but
/// the sources and the search knows the boards of the range).
fn run(lo: u32, hi: u32, opts: &SolveOptions, out: &Path) -> Vec<GameResult> {
    let mut games = Vec::new();
    let table = table_of(&[lo - 1]);
    solver::solve_range_each_with(lo, hi, opts, Some(out), &table, |game| games.push(game.clone()))
        .unwrap_or_else(|e| panic!("{e}"));
    games
}

fn board(games: &[GameResult], n: u32) -> &GameResult {
    games.iter().find(|game| game.n == n).unwrap_or_else(|| panic!("no game of {n}"))
}

/// The Rust verifier on `file`, given the sources, as `Err(message)` or `Ok(())`.
fn rust_verdict(file: &Path, sources: &KnownSources) -> Result<(), String> {
    let known = Known::load(Optimal::built_in(), sources)?;
    verify_files_with(&[file.to_path_buf()], &known, 1).map(|_| ())
}

/// The Python referee on `file`, given the sources: its exit status and report.
fn python_verdict(file: &Path, sources: &KnownSources) -> (Option<i32>, String) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/check_results.py");
    // A source that is not given is a path that is not there.
    let none = Path::new("/nonexistent/none.jsonl");
    let committed = sources.committed.as_deref().unwrap_or(none);
    let bounds = sources.bounds.as_deref().unwrap_or(none);
    let args = [common::path(file), "--committed", common::path(committed), "--bounds", common::path(bounds)];
    let run = python(&script, &args);
    (run.code, run.stdout)
}

/// Both verifiers accept `file` given the sources.
fn both_accept(file: &Path, sources: &KnownSources) {
    rust_verdict(file, sources).unwrap_or_else(|e| panic!("the Rust verifier: {e}"));
    let (code, report) = python_verdict(file, sources);
    assert_eq!(code, Some(0), "the Python referee:\n{report}");
}

/// Both verifiers refuse `file` given the sources, naming line `line` (counting from 1) and the board `n`; the Rust
/// verifier's message has `expected`.
fn both_refuse(file: &Path, sources: &KnownSources, line: usize, n: u32, expected: &str) {
    let err = rust_verdict(file, sources).expect_err("the Rust verifier refuses it");
    assert!(err.contains(&format!("line {line} (n={n}): ")) && err.contains(expected), "{expected}: {err}");
    let (code, report) = python_verdict(file, sources);
    let named = format!("line {line} (n={n}): ");
    assert!(code == Some(1) && report.contains(&named), "the Python referee, {named}\n{report}");
}

/// The boards of the store used below: composites and a prime, some that the chain finds at their ceiling and some it
/// searches.
const BOARDS: [u32; 6] = [906, 907, 910, 920, 930, 944];

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is proven_boards_of_the_store_are_recorded_to_913_and_both_verifiers_accept_them"
)]
fn proven_boards_of_the_store_are_recorded_without_a_search_and_both_verifiers_accept_them() {
    proven_boards_of_the_store_are_recorded_to(960, &BOARDS);
}

/// [`proven_boards_of_the_store_are_recorded_without_a_search_and_both_verifiers_accept_them`] for the games to 913,
/// the store's boards among them 906, 907 and 910.
#[test]
fn proven_boards_of_the_store_are_recorded_to_913_and_both_verifiers_accept_them() {
    proven_boards_of_the_store_are_recorded_to(913, &BOARDS[..3]);
}

/// The test of the games 900..=`hi`, with the games of `boards` in the store.
fn proven_boards_of_the_store_are_recorded_to(hi: u32, boards: &[u32]) {
    let dir = TempDir::new("known-store");
    let store = store_of(&dir, boards);
    let sources = sources(None, Some(&store));
    let out = dir.join("out.jsonl");
    let games = run(900, hi, &options(sources.clone()), &out);

    // The control: the same range from the same game before it, with nothing but the search to find the games.
    let control = run(900, hi, &options(KnownSources::default()), &dir.join("control.jsonl"));
    assert_eq!(games.len(), control.len());

    let proven: BTreeSet<u32> = boards.iter().copied().collect();
    for game in &games {
        let (score, _) = truth(game.n);
        assert_eq!(game.score, score, "n={}", game.n);
        assert_eq!(game.score, board(&control, game.n).score, "n={}", game.n);
        if proven.contains(&game.n) {
            // The store's game, recorded with no search, citing the lines it comes from, and assuming U where the upper
            // bound does.
            let cited = LineRef { rule: Rule::Given, source: "test-run".into() };
            let cite = Cite::Bounds { lower: Some(cited.clone()), upper: cited };
            assert_eq!((game.method, game.proof, game.nodes), (Method::Known, Some(Proof::Known), 0), "n={}", game.n);
            assert_eq!(game.known.as_ref().map(|cited| &cited.cite), Some(&cite), "n={}", game.n);
            assert_eq!(game.assumes, Assumptions::from([assumed_u(game.n)]), "n={}", game.n);
            assert!(game.setup.is_none() && game.in_the_bag.is_none(), "n={}", game.n);
        } else {
            assert!(game.known.is_none() && game.method != Method::Known, "n={}", game.n);
        }
    }
    // The first game's game before it is the table's, assumed optimal; what rests on a game inherits its assumptions.
    let first = games[0].prev.as_ref().unwrap();
    assert_eq!((first.source, first.n), (PrevSource::Optimal, 899));
    for pair in games.windows(2).filter(|pair| pair[1].proof.is_some_and(Proof::rests_on_prev)) {
        assert_eq!(pair[1].assumes, pair[0].assumes, "n={}", pair[1].n);
    }
    let after_prime = board(&games, 908);
    assert!(
        after_prime.assumes.is_subset(&Assumptions::from([assumed_u(907), assumed_u(906)])),
        "{:?}",
        after_prime.assumes
    );
    // 910, 920 and 930 are games the chain searches; the store has made the search needless.
    let searched = control.iter().filter(|game| [910, 920, 930].contains(&game.n));
    assert!(searched.clone().all(|game| game.method == Method::Search && game.nodes > 0));
    assert!(games.iter().map(|game| game.nodes).sum::<u64>() < control.iter().map(|game| game.nodes).sum::<u64>());

    both_accept(&out, &sources);

    // A run that starts after a board the store has proven takes that game as the game before it: from the store, which
    // is the better source than the table that has it too (the proven game assumes U where its bound rests).
    let later = dir.join("later.jsonl");
    let games = run(907, 912, &options(sources.clone()), &later);
    let prev = games[0].prev.as_ref().unwrap();
    assert_eq!((prev.source, prev.n, prev.proof), (PrevSource::Bounds, 906, Some(Proof::Known)));
    assert_eq!(prev.assumes, Assumptions::from([assumed_u(906)]));
    assert_eq!(games[0].method, Method::Known, "907 is proven by the store itself");
    both_accept(&later, &sources);

    // The report counts them.
    let known = Known::load(Optimal::built_in(), &sources).unwrap();
    let report = verify_files_with(std::slice::from_ref(&out), &known, 1).unwrap().to_string();
    let count = boards.len();
    let statement = format!("statement U at {count} boards in all");
    assert!(report.contains(&format!("known {count}")) && report.contains(&statement), "{report}");
}

#[test]
fn a_game_that_the_store_carries_from_the_game_of_another_line_is_played_and_cited() {
    // 902 is at its ceiling: the game of 901 followed by 902.  The store has it as a line that cites the game of 901.
    let dir = TempDir::new("known-carried");
    let (score901, moves901) = truth(901);
    let score902 = score901 + 902;
    assert_eq!(truth(902).0, score902, "902 is at its ceiling");
    assert_eq!(replay_finished(902, &complete(902, &moves901).unwrap()).unwrap(), score902);
    let store = dir.join("bounds.jsonl");
    let from = [("from", Json::from(901u32)), ("from_value", Json::from(score901))];
    write_lines(
        &store,
        &[
            proven_lines(901),
            vec![
                line(902, "lower", score902, "monotone", &from),
                line(902, "upper", score902, "ceiling", &[&from[..], &[("rests_on", Json::array([901u32]))]].concat()),
            ],
        ]
        .concat(),
    );
    let sources = sources(None, Some(&store));
    let known = Known::load(Optimal::built_in(), &sources).unwrap();
    let game = known.game_from(Origin::Bounds, 902).unwrap().expect("902 is proven in the store");
    assert_eq!((game.score, game.assumes.clone()), (score902, Assumptions::from([assumed_u(901)])));
    assert_eq!(replay_finished(902, &game.moves), Ok(score902));

    let out = dir.join("out.jsonl");
    let games = run(901, 903, &options(sources.clone()), &out);
    assert_eq!((games[0].method, games[1].method), (Method::Known, Method::Known));
    assert_eq!(games[1].assumes, Assumptions::from([assumed_u(901)]));
    // 903 follows a known game, and rests on it where it can: it has the same assumption or none.
    both_accept(&out, &sources);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is a_sample_of_the_tampered_stores_makes_both_verifiers_refuse_the_results"
)]
fn a_tampered_store_or_a_store_that_was_not_given_makes_both_verifiers_refuse_the_results() {
    both_verifiers_refuse_the_cases(|_| true);
}

/// [`a_tampered_store_or_a_store_that_was_not_given_makes_both_verifiers_refuse_the_results`] for four of its thirteen
/// cases: the first, fifth, ninth and last.
#[test]
fn a_sample_of_the_tampered_stores_makes_both_verifiers_refuse_the_results() {
    both_verifiers_refuse_the_cases(|case| case % 4 == 1);
}

/// The test of the cases `sampled` picks (counting from 1).
fn both_verifiers_refuse_the_cases(sampled: impl Fn(usize) -> bool) {
    let dir = TempDir::new("known-tampered");
    let store = store_of(&dir, &[906, 910]);
    let sources_ok = sources(None, Some(&store));
    let out = dir.join("out.jsonl");
    let games = run(906, 911, &options(sources_ok.clone()), &out);
    both_accept(&out, &sources_ok);
    let mut case = 0;
    let mut refuse = |file: &Path, sources: &KnownSources, line: usize, n: u32, expected: &str| {
        case += 1;
        if sampled(case) {
            both_refuse(file, sources, line, n, expected);
        }
    };
    let line_of = |n: u32| (n - 906) as usize + 1;
    let (score, _) = truth(910);

    // The store as it was; a variation of it for each case.
    let lines = read_lines(&store);
    let tampered = |name: &str, edit: &dyn Fn(Vec<Json>) -> Vec<Json>| -> KnownSources {
        let path = dir.join(name);
        write_lines(&path, &edit(lines.clone()));
        self::sources(None, Some(&path))
    };
    let at = |lines: &[Json], n: u32, side: &str| {
        lines
            .iter()
            .position(|l| {
                l.get("n").and_then(Json::as_u32) == Some(n) && l.get("side").and_then(Json::as_str) == Some(side)
            })
            .unwrap()
    };

    // An upper bound worth another value, a lower bound that is gone, a bound by another rule, one from another run.
    let value_changed = tampered("value.jsonl", &|mut l| {
        let i = at(&l, 910, "upper");
        l[i] = with(&l[i], "value", Json::from(score + 1));
        l
    });
    refuse(&out, &value_changed, line_of(910), 910, "has no upper bound of board 910 worth");
    let upper_gone = tampered("upper-gone.jsonl", &|mut l| {
        let i = at(&l, 910, "upper");
        l.remove(i);
        l
    });
    refuse(&out, &upper_gone, line_of(910), 910, "has no upper bound of board 910 worth");
    let lower_gone = tampered("lower-gone.jsonl", &|mut l| {
        let i = at(&l, 906, "lower");
        l.remove(i);
        l
    });
    refuse(&out, &lower_gone, line_of(906), 906, "has no lower bound of board 906 worth");
    let other_rule = tampered("rule.jsonl", &|mut l| {
        let i = at(&l, 906, "upper");
        l[i] = with(&l[i], "rule", Json::from("exhausted"));
        l
    });
    refuse(&out, &other_rule, line_of(906), 906, "is by exhausted from test-run, not by given from test-run");
    let other_run = tampered("source.jsonl", &|mut l| {
        let i = at(&l, 906, "lower");
        l[i] = with(&l[i], "source", Json::from("another-run"));
        l
    });
    refuse(&out, &other_run, line_of(906), 906, "lower bound of board 906");
    // A store that was not given at all.
    refuse(&out, &sources(None, None), line_of(906), 906, "which was not given to the check");

    // The results lines changed: a citation of another line, an assumption dropped or added, the citation gone.
    let results = read_lines(&out);
    let edit_line = |name: &str, n: u32, key: &str, value: Json| -> PathBuf {
        let path = dir.join(name);
        let mut edited = results.clone();
        let i = (n - 906) as usize;
        edited[i] = with(&edited[i], key, value);
        write_lines(&path, &edited);
        path
    };
    let cite = |lower_source: &str| {
        Json::object([
            ("source", Json::from("bounds")),
            ("lower", Json::object([("rule", Json::from("given")), ("source", Json::from(lower_source))])),
            ("upper", Json::object([("rule", Json::from("given")), ("source", Json::from("test-run"))])),
        ])
    };
    let refused = edit_line("cite.jsonl", 906, "known", cite("elsewhere"));
    refuse(&refused, &sources_ok, 1, 906, "lower bound of board 906");
    let refused = edit_line("assumes.jsonl", 906, "assumes", Json::array(Vec::<&str>::new()));
    refuse(&refused, &sources_ok, 1, 906, "records the assumptions");
    let refused = edit_line("assumes2.jsonl", 906, "assumes", Json::array([assumed_u(906), assumed_u(907)]));
    refuse(&refused, &sources_ok, 1, 906, "records the assumptions");
    let refused = edit_line("nocite.jsonl", 906, "known", Json::Null);
    refuse(&refused, &sources_ok, 1, 906, "cites no source");
    let refused = edit_line("proof.jsonl", 906, "proof", Json::from("exhausted"));
    refuse(&refused, &sources_ok, 1, 906, "records the proof exhausted, but it is known");
    // The game of a known line is checked as any game: a score that the moves do not make is refused.
    let refused = edit_line("score.jsonl", 906, "score", Json::from(games[0].score + 1));
    refuse(&refused, &sources_ok, 1, 906, "the moves score");
    // A citation on a line that is not a known game, a prime game say, is a contradiction.
    let refused = edit_line("prime.jsonl", 907, "known", cite("test-run"));
    refuse(&refused, &sources_ok, 2, 907, "known");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is an_open_board_of_the_store_seeds_its_search_and_its_upper_bound_stops_it_to_913"
)]
fn an_open_board_of_the_store_seeds_its_search_and_its_upper_bound_stops_it() {
    an_open_board_of_the_store_seeds_its_search_to(960);
}

/// [`an_open_board_of_the_store_seeds_its_search_and_its_upper_bound_stops_it`] for the boards of 900..913, where 910
/// is the dearest to search.
#[test]
fn an_open_board_of_the_store_seeds_its_search_and_its_upper_bound_stops_it_to_913() {
    an_open_board_of_the_store_seeds_its_search_to(913);
}

/// The test of the boards 900..=`hi`.
fn an_open_board_of_the_store_seeds_its_search_to(hi: u32) {
    let dir = TempDir::new("known-open");
    // The control: the boards of 900..=`hi` whose search is dear enough to show, searched with nothing but the table.
    let control = run(900, hi, &options(KnownSources::default()), &dir.join("control.jsonl"));
    let dear = control.iter().filter(|g| g.method == Method::Search && g.proof == Some(Proof::Exhausted));
    let target = dear.max_by_key(|g| g.nodes).expect("a searched board in the range");
    let n = target.n;
    let (score, moves) = truth(n);
    assert!(target.nodes > 300, "n={n} has {} nodes", target.nodes);

    // Only the upper bound: the search stops where it reaches the bound, which proves the game by it.
    let store = dir.join("upper.jsonl");
    write_lines(&store, &[line(n, "upper", score, "exhausted", &[("rests_on", Json::array([n]))])]);
    let sources = sources(None, Some(&store));
    let out = dir.join("upper-out.jsonl");
    let games = run(900, n + 1, &options(sources.clone()), &out);
    let game = board(&games, n);
    assert_eq!((game.method, game.proof, game.score), (Method::Search, Some(Proof::Known), score));
    let cite = Cite::Bounds { lower: None, upper: LineRef { rule: Rule::Exhausted, source: "test-run".into() } };
    assert_eq!(game.known.as_ref().map(|cited| &cited.cite), Some(&cite));
    assert_eq!(game.assumes, Assumptions::from([assumed_u(n)]));
    assert!(!game.reached_cap && game.setup.is_some(), "a search, stopped by the store's bound");
    assert!(game.nodes <= target.nodes, "{} nodes against {}", game.nodes, target.nodes);
    // The games after it are as they were (scores); the one that repeats its search rests on it and assumes what it does.
    for game in &games {
        assert_eq!(game.score, truth(game.n).0, "n={}", game.n);
    }
    both_accept(&out, &sources);

    // Only the lower bound: its game seeds the search, which starts at the optimum, and proves it as it did.  A search
    // of the board on its own (no game before it) shows the seed: the nodes fall to those of a seed at the optimum.
    let store = dir.join("lower.jsonl");
    write_lines(&store, &[line(n, "lower", score, "given", &[("moves", Json::array(moves.clone()))])]);
    let seeded = options(self::sources(None, Some(&store)));
    let alone = |opts: &SolveOptions| solver::solve_game_with(n, None, opts, &table_of(&[])).unwrap();
    let (with, without) = (alone(&seeded), alone(&options(KnownSources::default())));
    assert_eq!((with.score, with.proof, with.known.is_some()), (score, Some(Proof::Exhausted), false));
    assert!(with.nodes < without.nodes, "a seed at the optimum: {} nodes against {}", with.nodes, without.nodes);
    // ... and a forced search takes nothing from the store, but the ceiling and the seed of the game before it.
    let forced = SolveOptions { force_search: true, ..seeded };
    let forced = alone(&forced);
    assert_eq!(forced.nodes, without.nodes);
}

#[test]
fn the_committed_chain_and_the_table_are_known_games_too() {
    let dir = TempDir::new("known-committed");
    // A small committed chain: the games 1..=60, as `taxman solve 1-60 --out` writes them.
    let committed = dir.join("strict.jsonl");
    let chain = solver::solve_range(1, 60, &SolveOptions { threads: 1, ..SolveOptions::default() }, Some(&committed));
    let chain = chain.unwrap();
    let store = store_of(&dir, &[50, 62]);

    // From the committed chain: 50..=60 are its games, recorded without a search and assuming what its lines do
    // (nothing); 61 is built on its game of 60, and 62 is the store's, which proves it.  50 is in the store too, but
    // the committed chain is the better source.
    let sources = sources(Some(&committed), Some(&store));
    let out = dir.join("out.jsonl");
    let table = Optimal::from_json(&Json::Arr(Vec::new())).unwrap();
    let mut games = Vec::new();
    solver::solve_range_each_with(50, 64, &options(sources.clone()), Some(&out), &table, |g| games.push(g.clone()))
        .unwrap();
    for game in games.iter().filter(|g| g.n <= 60) {
        assert_eq!((game.method, game.proof, game.nodes), (Method::Known, Some(Proof::Known), 0), "n={}", game.n);
        assert_eq!(game.known.as_ref().map(|cited| &cited.cite), Some(&Cite::Strict), "n={}", game.n);
        assert!(game.assumes.is_empty(), "n={}", game.n);
        assert_eq!(game.moves, chain[game.n as usize - 1].moves, "n={}", game.n);
    }
    let first = games[0].prev.as_ref().unwrap();
    assert_eq!((first.source, first.n, first.proof), (PrevSource::Strict, 49, chain[48].proof));
    // 61 is a prime: the prime formula on the known game of 60, which assumes nothing.
    let sixty_one = board(&games, 61);
    assert_eq!((sixty_one.method, sixty_one.proof), (Method::Prime, Some(Proof::PrimeFormula)));
    assert_eq!(board(&games, 61).prev.as_ref().map(|p| p.source), Some(PrevSource::Run));
    let sixty_two = board(&games, 62);
    assert_eq!(sixty_two.known.as_ref().map(|cited| cited.cite.origin()), Some(Origin::Bounds));
    assert_eq!(sixty_two.assumes, Assumptions::from([assumed_u(62)]));
    both_accept(&out, &sources);

    // Starting in the middle of the committed chain, on the game before the range: the first game is built on it.
    let out = dir.join("later.jsonl");
    let later = SolveOptions { known: KnownSources { boards: false, ..sources.clone() }, ..options(sources.clone()) };
    let games = run_with(&later, 58, 63, &out, &table);
    assert_eq!(games[0].prev.as_ref().map(|p| (p.source, p.n)), Some((PrevSource::Strict, 57)));
    assert!(games.iter().take(3).all(|g| g.method != Method::Known), "boards are not known without `boards`");
    both_accept(&out, &sources);

    // From the table: the whole range is its games, each assumed optimal.
    let out = dir.join("table.jsonl");
    let from_table = options(KnownSources { boards: true, ..KnownSources::default() });
    let games = run_with(&from_table, 40, 45, &out, Optimal::built_in());
    for game in &games {
        assert_eq!(game.known.as_ref().map(|cited| &cited.cite), Some(&Cite::Optimal), "n={}", game.n);
        assert_eq!(game.assumes, Assumptions::from([assumed_optimal(game.n)]), "n={}", game.n);
    }
    both_accept(&out, &KnownSources::default());
}

/// [`run`] with the options and the table given.
fn run_with(opts: &SolveOptions, lo: u32, hi: u32, out: &Path, table: &Optimal) -> Vec<GameResult> {
    let mut games = Vec::new();
    solver::solve_range_each_with(lo, hi, opts, Some(out), table, |game| games.push(game.clone())).unwrap();
    games
}

#[test]
fn force_search_leaves_the_boards_to_their_searches() {
    let dir = TempDir::new("known-forced");
    let store = store_of(&dir, &[910]);
    let forced = SolveOptions { force_search: true, ..options(sources(None, Some(&store))) };
    let games = run(910, 912, &forced, &dir.join("out.jsonl"));
    assert!(
        games.iter().all(|g| g.method == Method::Search && g.known.is_none()),
        "{:?}",
        games.iter().map(|g| g.method).collect::<Vec<_>>()
    );
}

#[test]
fn a_store_game_that_does_not_play_or_contradicts_the_chain_stops_the_run_naming_the_board() {
    let dir = TempDir::new("known-bad");
    let (score, moves) = truth(910);
    let table = table_of(&[909]);
    let attempt = |lines: Vec<Json>| {
        let store = dir.join("bad.jsonl");
        write_lines(&store, &lines);
        let opts = options(sources(None, Some(&store)));
        solver::solve_range_each_with(910, 911, &opts, None, &table, |_| {})
    };
    // Moves that are not a game of 910.
    let broken = &moves[..moves.len() - 5];
    let err = attempt(vec![
        line(910, "lower", score, "given", &[("moves", Json::array(broken.iter().copied()))]),
        line(910, "upper", score, "given", &[("rests_on", Json::array([910u32]))]),
    ])
    .unwrap_err();
    assert!(err.contains("n=910") && err.contains("the game"), "{err}");
    // A game that plays, but scores less than the game before it: the store and the chain disagree.  (A finished game of
    // 910 that opens with 907, the largest prime, and then plays the largest legal move while there is one.)
    let worse = complete(910, &[907]).unwrap();
    let worse_score = replay_finished(910, &worse).unwrap();
    assert!(worse_score < truth(909).0, "the greedy game of 910 scores {worse_score}");
    let err = attempt(vec![
        line(910, "lower", worse_score, "given", &[("moves", Json::array(worse))]),
        line(910, "upper", worse_score, "given", &[("rests_on", Json::array([910u32]))]),
    ])
    .unwrap_err();
    assert!(err.contains("n=910") && err.contains("less than the previous game"), "{err}");
}

/// The command line finds the sources in `rust/results` of the repository it runs in (a directory above the working
/// directory with `src/main/resources/optimal.json`), and names no option for them: a repository made for the purpose,
/// with a store of two boards and no committed chain.
#[test]
fn solve_use_prev_and_verify_results_find_the_sources_in_the_repository() {
    let dir = TempDir::new("known-cli");
    fs::create_dir_all(dir.join("src/main/resources")).unwrap();
    fs::write(dir.join("src/main/resources/optimal.json"), "[]").unwrap();
    fs::create_dir_all(dir.join("rust/results")).unwrap();
    let store = dir.join("rust/results/bounds.jsonl");
    write_lines(&store, &[proven_lines(905), proven_lines(906)].concat());
    let in_repo = |args: &[&str]| {
        let run =
            std::process::Command::new(env!("CARGO_BIN_EXE_taxman")).args(args).current_dir(&*dir).output().unwrap();
        (
            run.status.code(),
            String::from_utf8_lossy(&run.stdout).into_owned(),
            String::from_utf8_lossy(&run.stderr).into_owned(),
        )
    };

    let out = dir.join("out.jsonl");
    let (code, stdout, stderr) =
        in_repo(&["solve", "903-908", "--use-prev", "--threads", "1", "--out", common::path(&out)]);
    assert_eq!(code, Some(0), "{stderr}");
    // The store's boards come from the store, the others from the table; none is searched.
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 6, "{stdout}");
    for (line, n) in lines.iter().zip(903..) {
        let store_board = [905, 906].contains(&n);
        let source = if store_board { "the bounds store" } else { "optimal.json" };
        assert!(line.contains("known") && line.contains(&format!("from {source}")), "{line}");
        assert!(line.contains(&format!("n={n} ")), "{line}");
    }
    assert!(stdout.contains(&assumed_u(905)) && stdout.contains(&assumed_optimal(903)), "{stdout}");

    // The verifier finds the same sources; without the store it refuses the lines that cite it, saying so.
    let (code, stdout, stderr) = in_repo(&["verify-results", common::path(&out)]);
    assert_eq!(code, Some(0), "{stdout}{stderr}");
    assert!(stdout.contains("proof: known 6"), "{stdout}");
    let moved = dir.join("rust/results/bounds.jsonl.moved");
    fs::rename(&store, &moved).unwrap();
    let (code, _, stderr) = in_repo(&["verify-results", common::path(&out)]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("line 3 (n=905)") && stderr.contains("which was not given to the check"), "{stderr}");
    // A run that resumes the file without the flag is told what to do.
    let (code, _, stderr) = in_repo(&["solve", "903-910", "--threads", "1", "--out", common::path(&out)]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("opened with --use-prev"), "{stderr}");
}

#[test]
fn the_store_is_read_by_the_library_that_the_ledger_writes_it_with() {
    // The one copy of the format: what the test wrote is a store, with its best bounds.
    let dir = TempDir::new("known-format");
    let store = store_of(&dir, &BOARDS);
    let read = Store::read(&store).unwrap();
    let best = read.best();
    assert_eq!(best.proven(), BOARDS.to_vec());
    assert!(best.lower.values().all(|line| line.side == Side::Lower));
}
