//! The tactic `same_search`: a game whose search problem an earlier game of the chain already settled is concluded
//! with that game's promotions, with no search (method `repeat`, proof `same_search`; DESIGN.md, "Same search").
//!
//! The games that repeat an earlier search are found here from the setups alone ([`repeats_by_problem`]), and a
//! strict chain must conclude exactly those without a search, with the scores of a chain that searches them: from
//! n = 1, from the committed `results/strict.jsonl` (whose lines, schema 3, do not record their problems' digests),
//! and from a results file that a run left (whose lines do).  A game with a checkpoint is resumed, never repeated, and
//! a strict run repeats only games that rest on nothing unproven.  The forged repeats that both verifiers refuse are
//! in `tests/results.rs`.  Small boards, one thread.

mod common;

use common::TempDir;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;
use taxman::board::Board;
use taxman::json::Json;
use taxman::results::{verify_files, Proof, Taint, OPTIMAL_JSON};
use taxman::search::{Problem, Revise};
use taxman::solver::{self, GameResult, Method, SolveOptions, SAME_SEARCH_WINDOW};

/// The strict options with `same_search` set as given.
fn strict(same_search: bool) -> SolveOptions {
    let mut opts = common::strict_options(1);
    opts.tactics.same_search = same_search;
    opts
}

/// The games of `lo..=hi` whose search problem as a strict run sets it up (frame 0 as built) one of the
/// [`SAME_SEARCH_WINDOW`] board sizes before it also posed, each with the latest of those: the games that a strict
/// chain over the range, holding every game before `lo`, concludes without a search.  Prime and trivial boards set no
/// search up.  Found from the setups alone, without a search.
fn repeats_by_problem(lo: u32, hi: u32) -> Vec<(u32, u32)> {
    let mut problems: BTreeMap<u32, Problem> = BTreeMap::new();
    let mut repeats = Vec::new();
    for n in lo.saturating_sub(SAME_SEARCH_WINDOW).max(1)..=hi {
        let problem = taxman::with_width!(n, W => {
            let board = Board::<W>::for_n(n);
            (!board.tables.is_prime(n) && !board.is_trivial()).then(|| board.problem(Revise::Off).unwrap())
        });
        let Some(problem) = problem else { continue };
        if n >= lo {
            let window = n.saturating_sub(SAME_SEARCH_WINDOW)..n;
            if let Some(m) = window.rev().find(|m| problems.get(m) == Some(&problem)) {
                repeats.push((n, m));
            }
        }
        problems.insert(n, problem);
    }
    repeats
}

/// The repeats among `games`, each with the game it repeats.
fn repeats_of(games: &[GameResult]) -> Vec<(u32, u32)> {
    let repeat = |g: &GameResult| g.same_search.as_ref().map(|s| (g.n, s.n));
    games.iter().filter_map(repeat).collect()
}

/// Checks that `on` (with `same_search`) and `off` (without) are the same games of a strict chain, with the same
/// scores, and that the repeats of `on` are `expected`, concluded with no search, on proofs that hold.
fn compare(on: &[GameResult], off: &[GameResult], expected: &[(u32, u32)]) {
    let scores = |games: &[GameResult]| games.iter().map(|g| (g.n, g.score)).collect::<Vec<_>>();
    assert_eq!(scores(on), scores(off));
    assert!(!expected.is_empty(), "no repeats to test");
    assert_eq!(repeats_of(on), expected);
    assert!(off.iter().all(|g| g.method != Method::Repeat && g.same_search.is_none()));
    for g in on {
        assert!(g.completed && g.taint.is_empty() && g.optimal_match == Some(true), "n={}: {g:?}", g.n);
        if let Some(same) = &g.same_search {
            let earlier = on.iter().chain(off).find(|e| e.n == same.n);
            let sum: u64 = g.promotions.iter().map(|&p| u64::from(p)).sum();
            assert_eq!(
                (g.method, g.proof, g.nodes, same.promotion_sum),
                (Method::Repeat, Some(Proof::SameSearch), 0, sum)
            );
            assert_eq!(g.setup.map(|s| (s.digest, s.cap)), Some((same.digest, None)), "n={}", g.n);
            if let Some(earlier) = earlier.filter(|e| e.method == Method::Search) {
                assert_eq!(g.promotions, earlier.promotions, "n={}", g.n);
            }
        }
    }
    // A repeat replaces a search: every game that was searched without the tactic is searched or repeated with it.
    for (a, b) in on.iter().zip(off) {
        assert_eq!(a.method.sets_up(), b.method.sets_up(), "n={}", a.n);
    }
}

/// The lines of the committed `results/strict.jsonl` for the board sizes below `lo`, in a file of `dir` to extend.
fn committed_below(dir: &TempDir, name: &str, lo: u32) -> std::path::PathBuf {
    let text = fs::read_to_string(common::results_path("strict.jsonl")).unwrap();
    let below = |line: &&str| Json::parse(line).unwrap().get("n").and_then(Json::as_u32).unwrap() < lo;
    let path = dir.join(name);
    fs::write(&path, text.lines().filter(below).map(|line| format!("{line}\n")).collect::<String>()).unwrap();
    path
}

fn lines(path: &Path) -> Vec<Json> {
    fs::read_to_string(path).unwrap().lines().map(|line| Json::parse(line).unwrap()).collect()
}

#[test]
fn a_strict_chain_from_1_repeats_exactly_the_searches_that_repeat() {
    let on = solver::solve_range(1, 300, &strict(true), None).unwrap();
    let off = solver::solve_range(1, 300, &strict(false), None).unwrap();
    // 107 of the 300, from 22 (which repeats 21) on.
    let expected = repeats_by_problem(1, 300);
    assert_eq!((expected.len(), expected[0]), (107, (22, 21)));
    compare(&on, &off, &expected);
    // The proofs of the games repeated: a search to exhaustion, one that reached its cap, or a repeat again.
    let proof_of = |n: u32| on.iter().find(|g| g.n == n).and_then(|g| g.proof).unwrap();
    let proofs: std::collections::BTreeSet<Proof> = expected.iter().map(|&(_, m)| proof_of(m)).collect();
    assert_eq!(proofs, [Proof::Cap, Proof::Exhausted, Proof::SameSearch].into());

    // The lines read back, by the verifier that sets both problems of every repeat up again.
    let dir = TempDir::new("same-search-chain");
    let path = dir.join("on.jsonl");
    fs::write(&path, on.iter().map(|g| format!("{}\n", g.to_json())).collect::<String>()).unwrap();
    let report = verify_files(std::slice::from_ref(&path), None).unwrap_or_else(|e| panic!("{e}"));
    let games = &report.files[0].contents.games;
    assert_eq!(games.values().filter(|g| g.proof == Proof::SameSearch).count(), 107);
}

/// The committed strict chain records no digests (schema 3): the games of the window before a range are set up
/// again to find the earlier game, which can be one of the file's (515 repeats 514).
#[test]
fn a_chain_from_the_committed_results_repeats_their_searches() {
    let (lo, hi) = (515, 560);
    let dir = TempDir::new("same-search-committed");
    let (with, without) = (committed_below(&dir, "on.jsonl", lo), committed_below(&dir, "off.jsonl", lo));
    let on = solver::solve_range(lo, hi, &strict(true), Some(&with)).unwrap();
    let off = solver::solve_range(lo, hi, &strict(false), Some(&without)).unwrap();
    let expected = repeats_by_problem(lo, hi);
    assert_eq!(expected[0], (515, 514), "the first repeats a game of the file");
    compare(&on, &off, &expected);
    let committed = common::stored_games("strict.jsonl");
    assert!(on.iter().all(|g| g.score == committed[&g.n].0), "the committed scores");
    verify_files(&[with, without], None).unwrap_or_else(|e| panic!("{e}"));
}

/// A range stopped and run again finds the earlier games of the first run in its results file, by the digests its
/// lines record (schema 4): the same repeats as a chain in one piece.
#[test]
fn a_range_run_again_repeats_the_searches_of_its_results_file() {
    let dir = TempDir::new("same-search-resumed");
    let path = dir.join("r.jsonl");
    // The first run searches everything; 65 repeats 64 of it, and 66 repeats 65 of the second run.
    let first = solver::solve_range(1, 64, &strict(false), Some(&path)).unwrap();
    assert!(first.iter().all(|g| g.method != Method::Repeat));
    assert!(lines(&path)
        .iter()
        .all(|line| line.get("setup").is_some_and(|s| s.is_null() || s.get("digest").is_some())));
    let second = solver::solve_range(1, 120, &strict(true), Some(&path)).unwrap();
    assert_eq!(second.first().map(|g| g.n), Some(65));
    let expected = repeats_by_problem(65, 120);
    assert_eq!(&expected[..2], [(65, 64), (66, 65)]);
    assert_eq!(repeats_of(&second), expected);
    let in_one_piece = solver::solve_range(1, 120, &strict(true), None).unwrap();
    assert_eq!(repeats_of(&in_one_piece[64..]), expected);
    assert!(second.iter().zip(&in_one_piece[64..]).all(|(a, b)| (a.n, a.score) == (b.n, b.score)));
    verify_files(std::slice::from_ref(&path), None).unwrap_or_else(|e| panic!("{e}"));
}

/// A game whose search was stopped with a checkpoint is resumed, not repeated, even when an earlier game posed its
/// problem: 265 repeats 264, but its checkpoint (made with the tactic off) is resumed; 266, 267 and 268 then repeat
/// 265, 266 and 267.
#[test]
fn a_game_with_a_checkpoint_is_resumed_not_repeated() {
    let dir = TempDir::new("same-search-checkpoint");
    let path = dir.join("r.jsonl");
    let checkpoints = dir.join("ckpt");
    solver::solve_range(1, 264, &strict(true), Some(&path)).unwrap();
    assert_eq!(repeats_by_problem(265, 268), [(265, 264), (266, 265), (267, 266), (268, 267)]);
    // A time limit that has run out stops the search before it starts, with every unit in its checkpoint.
    let stopped =
        SolveOptions { checkpoint_dir: Some(checkpoints.clone()), time_limit: Some(Duration::ZERO), ..strict(false) };
    let summary = solver::solve_range_each(265, 268, &stopped, Some(&path), |_| {}).unwrap();
    assert_eq!(summary.stopped_at, Some(265));
    assert!(checkpoints.join("n0265.ckpt").exists());

    let resumed = SolveOptions { checkpoint_dir: Some(checkpoints.clone()), ..strict(true) };
    let games = solver::solve_range(265, 268, &resumed, Some(&path)).unwrap();
    let methods: Vec<(u32, Method, Option<u32>)> =
        games.iter().map(|g| (g.n, g.method, g.same_search.as_ref().map(|s| s.n))).collect();
    let expected = [
        (265, Method::Search, None),
        (266, Method::Repeat, Some(265)),
        (267, Method::Repeat, Some(266)),
        (268, Method::Repeat, Some(267)),
    ];
    assert_eq!(methods, expected);
    assert_eq!((games[0].invocations, games[0].proof), (2, Some(Proof::Exhausted)), "265 was resumed");
    assert!(games.iter().all(|g| g.taint.is_empty()));
    assert!(!checkpoints.join("n0265.ckpt").exists());
    verify_files(std::slice::from_ref(&path), None).unwrap_or_else(|e| panic!("{e}"));
}

/// A repeat rests on the game it repeats: 34 repeats 33, which built on optimal.json's 32 reaches its cap and so rests
/// on optimal.json, and so does 34 then, though its own setup rests on nothing.  A strict run repeats only a game that
/// rests on nothing unproven, unless it is allowed to build on one (`allow_tainted_prev`): without the cap (the
/// tactic `prev_cap`, which would refuse 33 itself) and without the allowance, 34 is searched, and rests on nothing.
#[test]
fn a_repeat_rests_on_the_game_it_repeats_and_a_strict_run_repeats_only_clean_games() {
    let dir = TempDir::new("same-search-taint");
    let allowed = SolveOptions { allow_tainted_prev: true, ..strict(true) };
    let games = solver::solve_range(33, 34, &allowed, None).unwrap();
    let optimal_json = Taint::from([OPTIMAL_JSON.to_owned()]);
    assert_eq!((games[0].proof, &games[0].taint), (Some(Proof::Cap), &optimal_json));
    assert_eq!(games[1].same_search.as_ref().map(|s| (s.n, &s.taint)), Some((33, &optimal_json)));
    assert_eq!(games[1].taint, optimal_json);

    let path = dir.join("r.jsonl");
    solver::solve_range(33, 33, &allowed, Some(&path)).unwrap();
    let mut uncapped = strict(true);
    uncapped.tactics.prev_cap = false;
    let next = solver::solve_range(33, 34, &uncapped, Some(&path)).unwrap().remove(0);
    assert_eq!((next.n, next.method, next.proof), (34, Method::Search, Some(Proof::Exhausted)));
    assert!(next.taint.is_empty());
    verify_files(std::slice::from_ref(&path), None).unwrap_or_else(|e| panic!("{e}"));
}
