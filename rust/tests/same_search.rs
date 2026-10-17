//! The tactic `same_search`: a game whose search problem an earlier game of the chain already settled is concluded
//! with that game's promotions, with no search (method `repeat`, proof `same_search`; DESIGN.md, "Same search").
//!
//! The games that repeat an earlier search are found here from the setups alone ([`repeats_by_problem`]), and a
//! chain must conclude exactly those without a search, with the scores of a chain that searches them: from n = 1, and
//! from a results file that a run left.  A game with a checkpoint is resumed, never repeated, and a repeat rests on
//! what the game it repeats rests on.  The forged repeats that both verifiers refuse are in `tests/results.rs`.
//! Small boards, one thread.

#[macro_use]
mod common;

use common::TempDir;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;
use taxman::board::Board;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::results::{assumed_optimal, verify_files, Assumptions, Proof};
use taxman::search::Problem;
use taxman::solver::{self, GameResult, Method, SolveOptions, SAME_SEARCH_WINDOW};

/// The default options with `same_search` set as given.
fn options(same_search: bool) -> SolveOptions {
    let mut opts = common::options(1);
    opts.tactics.same_search = same_search;
    opts
}

/// The games of `lo..=hi` whose search problem one of the [`SAME_SEARCH_WINDOW`] board sizes before it also posed,
/// each with the latest of those: the games that a chain over the range, holding every game before `lo`, concludes
/// without a search.  Prime and trivial boards set no search up.  Found from the setups alone, without a search.
fn repeats_by_problem(lo: u32, hi: u32) -> Vec<(u32, u32)> {
    let mut problems: BTreeMap<u32, Problem> = BTreeMap::new();
    let mut repeats = Vec::new();
    for n in lo.saturating_sub(SAME_SEARCH_WINDOW).max(1)..=hi {
        let problem = taxman::with_width!(n, W => {
            let board = Board::<W>::for_n(n);
            (!board.tables.is_prime(n) && !board.is_trivial()).then(|| board.problem().unwrap())
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

/// Checks that `on` (with `same_search`) and `off` (without) are the same games of a chain, with the same
/// scores, and that the repeats of `on` are `expected`, concluded with no search, on proofs that hold.
fn compare(on: &[GameResult], off: &[GameResult], expected: &[(u32, u32)]) {
    let scores = |games: &[GameResult]| games.iter().map(|g| (g.n, g.score)).collect::<Vec<_>>();
    assert_eq!(scores(on), scores(off));
    assert!(!expected.is_empty(), "no repeats to test");
    assert_eq!(repeats_of(on), expected);
    assert!(off.iter().all(|g| g.method != Method::Repeat && g.same_search.is_none()));
    for g in on {
        assert!(g.completed && g.assumes.is_empty() && g.optimal_match == Some(true), "n={}: {g:?}", g.n);
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

fn lines(path: &Path) -> Vec<Json> {
    fs::read_to_string(path).unwrap().lines().map(|line| Json::parse(line).unwrap()).collect()
}

#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_chain_from_1_to_100_repeats_exactly_the_searches_that_repeat))]
fn a_chain_from_1_repeats_exactly_the_searches_that_repeat() {
    // 107 of the 300, from 22 (which repeats 21) on.
    a_chain_from_1_repeats_exactly_the_searches_that_repeat_to(300, 107);
}

/// [`a_chain_from_1_repeats_exactly_the_searches_that_repeat`] for the games 1..=100.
#[test]
fn a_chain_from_1_to_100_repeats_exactly_the_searches_that_repeat() {
    a_chain_from_1_repeats_exactly_the_searches_that_repeat_to(100, 26);
}

/// A chain 1..=`hi` repeats `repeats` games, exactly the searches that repeat, from 22 (which repeats 21) on.
fn a_chain_from_1_repeats_exactly_the_searches_that_repeat_to(hi: u32, repeats: usize) {
    let on = solver::solve_range(1, hi, &options(true), None).unwrap();
    let off = solver::solve_range(1, hi, &options(false), None).unwrap();
    let expected = repeats_by_problem(1, hi);
    assert_eq!((expected.len(), expected[0]), (repeats, (22, 21)));
    compare(&on, &off, &expected);
    // The proofs of the games repeated: a search to exhaustion, one that reached its ceiling, or a repeat again.
    let proof_of = |n: u32| on.iter().find(|g| g.n == n).and_then(|g| g.proof).unwrap();
    let proofs: std::collections::BTreeSet<Proof> = expected.iter().map(|&(_, m)| proof_of(m)).collect();
    assert_eq!(proofs, [Proof::Cap, Proof::Exhausted, Proof::SameSearch].into());

    // The lines read back, by the verifier that sets both problems of every repeat up again.
    let dir = TempDir::new("same-search-chain");
    let path = dir.join("on.jsonl");
    fs::write(&path, on.iter().map(|g| format!("{}\n", g.to_json())).collect::<String>()).unwrap();
    let report = verify_files(std::slice::from_ref(&path), Optimal::built_in(), 1).unwrap_or_else(|e| panic!("{e}"));
    let games = &report.files[0].contents.games;
    assert_eq!(games.values().filter(|g| g.proof == Proof::SameSearch).count(), repeats);
}

/// A range stopped and run again finds the earlier games of the first run in its results file, by the digests its
/// lines record: the same repeats as a chain in one piece.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_range_run_again_repeats_the_searches_of_its_results_file_to_90))]
fn a_range_run_again_repeats_the_searches_of_its_results_file() {
    a_range_run_again_repeats_the_searches_of_its_results_file_to(120);
}

/// [`a_range_run_again_repeats_the_searches_of_its_results_file`] for the games 1..=90.
#[test]
fn a_range_run_again_repeats_the_searches_of_its_results_file_to_90() {
    a_range_run_again_repeats_the_searches_of_its_results_file_to(90);
}

/// The second run, of the games 1..=`hi`, repeats the searches of the first run's games 1..=64.
fn a_range_run_again_repeats_the_searches_of_its_results_file_to(hi: u32) {
    let dir = TempDir::new("same-search-resumed");
    let path = dir.join("r.jsonl");
    // The first run searches everything; 65 repeats 64 of it, and 66 repeats 65 of the second run.
    let first = solver::solve_range(1, 64, &options(false), Some(&path)).unwrap();
    assert!(first.iter().all(|g| g.method != Method::Repeat));
    assert!(lines(&path)
        .iter()
        .all(|line| line.get("setup").is_some_and(|s| s.is_null() || s.get("digest").is_some())));
    let second = solver::solve_range(1, hi, &options(true), Some(&path)).unwrap();
    assert_eq!(second.first().map(|g| g.n), Some(65));
    let expected = repeats_by_problem(65, hi);
    assert_eq!(&expected[..2], [(65, 64), (66, 65)]);
    assert_eq!(repeats_of(&second), expected);
    let in_one_piece = solver::solve_range(1, hi, &options(true), None).unwrap();
    assert_eq!(repeats_of(&in_one_piece[64..]), expected);
    assert!(second.iter().zip(&in_one_piece[64..]).all(|(a, b)| (a.n, a.score) == (b.n, b.score)));
    verify_files(std::slice::from_ref(&path), Optimal::built_in(), 1).unwrap_or_else(|e| panic!("{e}"));
}

/// A game whose search was stopped with a checkpoint is resumed, not repeated, even when an earlier game posed its
/// problem: 265 repeats 264, but its checkpoint (made with the tactic off) is resumed; 266, 267 and 268 then repeat
/// 265, 266 and 267.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_game_with_a_checkpoint_is_resumed_not_repeated_at_65))]
fn a_game_with_a_checkpoint_is_resumed_not_repeated() {
    a_game_with_a_checkpoint_is_resumed_not_repeated_from(265, 268);
}

/// [`a_game_with_a_checkpoint_is_resumed_not_repeated`] for 65, which repeats 64, and 66, which repeats 65.
#[test]
fn a_game_with_a_checkpoint_is_resumed_not_repeated_at_65() {
    a_game_with_a_checkpoint_is_resumed_not_repeated_from(65, 66);
}

/// The games `first..=last` each repeat the one before, and the one before `first` is solved: `first`, whose search was
/// stopped with a checkpoint (made with the tactic off), is resumed and not repeated; the others repeat it.
fn a_game_with_a_checkpoint_is_resumed_not_repeated_from(first: u32, last: u32) {
    let dir = TempDir::new("same-search-checkpoint");
    let path = dir.join("r.jsonl");
    let checkpoints = dir.join("ckpt");
    solver::solve_range(1, first - 1, &options(true), Some(&path)).unwrap();
    let repeats: Vec<(u32, u32)> = (first..=last).map(|n| (n, n - 1)).collect();
    assert_eq!(repeats_by_problem(first, last), repeats);
    // A time limit that has run out stops the search before it starts, with every unit in its checkpoint.
    let stopped =
        SolveOptions { checkpoint_dir: Some(checkpoints.clone()), time_limit: Some(Duration::ZERO), ..options(false) };
    let summary = solver::solve_range_each(first, last, &stopped, Some(&path), |_| {}).unwrap();
    assert_eq!(summary.stopped_at, Some(first));
    assert!(checkpoints.join(format!("n{first:04}.ckpt")).exists());

    let resumed = SolveOptions { checkpoint_dir: Some(checkpoints.clone()), ..options(true) };
    let games = solver::solve_range(first, last, &resumed, Some(&path)).unwrap();
    let methods: Vec<(u32, Method, Option<u32>)> =
        games.iter().map(|g| (g.n, g.method, g.same_search.as_ref().map(|s| s.n))).collect();
    let expected: Vec<(u32, Method, Option<u32>)> = (first..=last)
        .map(|n| if n == first { (n, Method::Search, None) } else { (n, Method::Repeat, Some(n - 1)) })
        .collect();
    assert_eq!(methods, expected);
    assert_eq!((games[0].invocations, games[0].proof), (2, Some(Proof::Exhausted)), "{first} was resumed");
    assert!(games.iter().all(|g| g.assumes.is_empty()));
    assert!(!checkpoints.join(format!("n{first:04}.ckpt")).exists());
    verify_files(std::slice::from_ref(&path), Optimal::built_in(), 1).unwrap_or_else(|e| panic!("{e}"));
}

/// A repeat assumes what the game it repeats assumes: 34 repeats 33, which, built on optimal.json's 32 (`--use-prev`),
/// reaches its ceiling and so assumes that game optimal, and so does 34 then, though it ran no search.  Searched
/// without the repeat and without the ceiling (a proof by `cap` would rest on 33 too), 34 is an exhausted search
/// and assumes nothing.
#[test]
fn a_repeat_assumes_what_the_game_it_repeats_assumes() {
    let dir = TempDir::new("same-search-assumes");
    let use_prev = SolveOptions { use_prev: true, ..options(true) };
    let games = solver::solve_range(33, 34, &use_prev, None).unwrap();
    let optimal_json = Assumptions::from([assumed_optimal(32)]);
    assert_eq!((games[0].proof, &games[0].assumes), (Some(Proof::Cap), &optimal_json));
    assert_eq!(games[1].same_search.as_ref().map(|s| (s.n, &s.assumes)), Some((33, &optimal_json)));
    assert_eq!(games[1].assumes, optimal_json);

    let path = dir.join("r.jsonl");
    solver::solve_range(33, 33, &use_prev, Some(&path)).unwrap();
    let mut uncapped = options(false);
    uncapped.tactics.prev_cap = false;
    let next = solver::solve_range(33, 34, &uncapped, Some(&path)).unwrap().remove(0);
    assert_eq!((next.n, next.method, next.proof), (34, Method::Search, Some(Proof::Exhausted)));
    assert!(next.assumes.is_empty());
    verify_files(std::slice::from_ref(&path), Optimal::built_in(), 1).unwrap_or_else(|e| panic!("{e}"));
}
