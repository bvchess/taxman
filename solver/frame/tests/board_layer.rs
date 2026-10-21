//! The board layer ([`taxman::frames`], [`taxman::board`]) against the committed chain.
//!
//! Every game of `results/strict.jsonl` that was searched (or repeats a search) records the moves of the game its
//! promotions make and the digest of the search problem its board poses.  Both come out of the board layer, which
//! nothing else compares with the chain at the boards that matter (the golden data stops at 2049): a change of how
//! setup forms and simplifies the frames, however it is made, must leave every one of them as it was.  A sample of
//! the chain is enough to catch a change of any decision (the boards differ in what their frames hold), and costs a
//! few seconds.

mod common;

use common::results_path;
use taxman::accelerate::solution_for_promotions;
use taxman::bits::Bits;
use taxman::board::search_problem;
use taxman::json::Json;
use taxman::numtheory::Tables;
use taxman::solver::digest_text;
use taxman::with_width;

/// Every this many of the chain's searched games (those with method `search` or `repeat`) are checked, counting
/// from the first.
const EVERY: usize = 25;

/// Every this many in the quick test, which stops at the board 1300: a board costs about as much as n² to set up.
const QUICK_EVERY: usize = 20;

/// A searched game of the chain, as its results line records it.
struct Line {
    n: u32,
    promotions: Vec<u32>,
    moves: Vec<u32>,
    digest: String,
}

/// The searched games of `results/strict.jsonl`: every `every`th one, and the last (if the board is at most
/// `up_to`).
fn sampled_lines(every: usize, up_to: u32) -> Vec<Line> {
    let text = std::fs::read_to_string(results_path("strict.jsonl")).expect("results file");
    // Line k is the game k, so the lines up to the board `up_to` are the first `up_to`.
    let searched: Vec<Json> = text
        .lines()
        .take(up_to as usize)
        .filter(|l| !l.trim().is_empty())
        .map(|l| Json::parse(l).expect("a results line"))
        .filter(|j| matches!(j.get("method").and_then(Json::as_str), Some("start" | "search" | "repeat")))
        .collect();
    let last = searched.len() - 1;
    let line = |j: &Json| {
        let field = |name: &str| j.get(name).unwrap_or_else(|| panic!("a results line without {name}"));
        let n = field("n").as_u32().expect("n");
        let digest = field("setup").get("digest").and_then(Json::as_str).unwrap_or_else(|| panic!("n={n}: no digest"));
        Line {
            n,
            promotions: field("promotions").as_u32_vec().expect("promotions"),
            moves: field("moves").as_u32_vec().expect("moves"),
            digest: digest.to_owned(),
        }
    };
    let sampled = (0..=last).filter(|i| i % every == 0 || *i == last).map(|i| line(&searched[i]));
    sampled.filter(|line| line.n <= up_to).collect()
}

/// The game the line's promotions make, as the board layer plays it now, at the set width the solver picks.
fn moves_of(line: &Line) -> Vec<u32> {
    with_width!(line.n, W => {
        let promotions = Bits::<W>::from_slice(&line.promotions);
        solution_for_promotions::<W>(&Tables::new(line.n), promotions)
    })
}

/// The moves of the games a promotion set makes, and the digest of the search problem of a board, are those of the
/// committed chain: at a sample of boards from 18 to 4546, the sets of 8 to 128 words.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is the_board_layer_reproduces_the_chains_moves_and_problems_to_1300"
)]
fn the_board_layer_reproduces_the_chains_moves_and_problems() {
    let lines = sampled_lines(EVERY, u32::MAX);
    assert!(lines.len() > 100 && lines.last().is_some_and(|l| l.n == 4546), "{} lines", lines.len());
    the_board_layer_reproduces(&lines);
}

/// [`the_board_layer_reproduces_the_chains_moves_and_problems`] for every twentieth of the searched games to the board
/// 1300.
#[test]
fn the_board_layer_reproduces_the_chains_moves_and_problems_to_1300() {
    let lines = sampled_lines(QUICK_EVERY, 1300);
    assert!(lines.len() >= 10, "{} lines", lines.len());
    the_board_layer_reproduces(&lines);
}

/// The moves of the games the promotions of each of `lines` make, and the digest of the search problem of each board,
/// are those of the committed chain.
fn the_board_layer_reproduces(lines: &[Line]) {
    for line in lines {
        assert_eq!(moves_of(line), line.moves, "n={}: the game the promotions make", line.n);
        let (problem, _in_the_bag) = search_problem(line.n).expect("a problem");
        assert_eq!(digest_text(problem.digest()), line.digest, "n={}: the search problem's digest", line.n);
    }
}
