//! How the search problems of two boards differ: what changes from one game of the chain to the next.  The study that
//! used it, with its findings, is in docs/EXPERIMENTS.md, "Carrying a search's learning to the next game (2026-10)".
//!
//! ```text
//! cargo run --release --example diff -- [--known FILE]... A:B ...
//! ```
//!
//! Both boards of a pair `A:B` are set up as `taxman solve` sets them up (`Board::prepare` with the default search
//! tactics), and one block is printed:
//!
//! * a header: the frames, the candidates, `max_total` and `in_the_bag` of A and B;
//! * per frame number, `same` with its moves, factors and freedoms, or the moves and factors that B has and A lacks
//!   (`+[..]`) and the other way round (`-[..]`), with the freedoms of A and of B (more than 12 numbers are cut to the
//!   first 12 and a count);
//! * the candidates B gained and lost, and the candidates of both whose home frame changed;
//! * for each of the two games, the promotions of its optimal game (its moves up to n/2): their number and sum `P*`,
//!   whether they are all candidates of their own board, and whether they are all candidates of the other board and
//!   can all be promoted from its root.  Promotable one way and not the other is the mark of a problem that relaxes
//!   the other; it is a check on two sets, not a proof (the sufficient condition is in the entry "Going higher and
//!   coming back down").
//!
//! The optimal games are those of optimal.json and of the completed games of each `--known` results file (as
//! `results/strict.jsonl` for the games of 2..3440, or a copy of a longer chain); a game that none of them has gets no
//! line.

use std::path::PathBuf;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::game::Optimal;
use taxman::model::Scratch;
use taxman::search::{Prepared, SearchTactics};

/// `set` for printing: its numbers, or the first 12 and how many there are.
fn list<const W: usize>(set: &Bits<W>) -> String {
    let numbers: Vec<String> = set.iter().map(|x| x.to_string()).collect();
    if numbers.len() > 12 {
        format!("[{} ... {} numbers]", numbers[..12].join(" "), numbers.len())
    } else {
        format!("[{}]", numbers.join(" "))
    }
}

/// The promotions of the optimal game of `n`: its moves up to n/2 (the numbers above n/2 are sources, which are
/// nobody's factor and so are not promotions).  `None` if no known game has `n`.
fn promotions<const W: usize>(opt: &Optimal, n: u32) -> Option<Bits<W>> {
    let mut set = Bits::<W>::EMPTY;
    for &m in opt.moves(n)? {
        if 2 * m <= n {
            set.insert(m);
        }
    }
    Some(set)
}

/// The block for the boards `a` and `b`.
fn pair<const W: usize>(a: u32, b: u32, opt: &Optimal) {
    let (board_a, board_b) = (Board::<W>::for_n(a), Board::<W>::for_n(b));
    let tactics = SearchTactics::default();
    let (pa, pb): (Prepared<W>, Prepared<W>) = (board_a.prepare(tactics).unwrap(), board_b.prepare(tactics).unwrap());
    println!(
        "== {a} -> {b}: frames {} -> {}, candidates {} -> {}, max_total {} -> {}, in_the_bag {} -> {}",
        pa.root.len(),
        pb.root.len(),
        pa.candidates.len(),
        pb.candidates.len(),
        pa.max_total,
        pb.max_total,
        board_a.in_the_bag,
        board_b.in_the_bag
    );
    for h in 0..pa.root.len().max(pb.root.len()) {
        let frame = |p: &Prepared<W>| {
            p.root.frames().get(h).map_or((Bits::EMPTY, Bits::EMPTY, 0), |f| (f.moves, f.factors, f.freedoms))
        };
        let ((moves_a, factors_a, freedoms_a), (moves_b, factors_b, freedoms_b)) = (frame(&pa), frame(&pb));
        if (moves_a, factors_a, freedoms_a) == (moves_b, factors_b, freedoms_b) {
            println!("  frame {h}: same ({} moves, {} factors, freedoms {freedoms_a})", moves_a.len(), factors_a.len());
        } else {
            println!(
                "  frame {h}: moves {}->{} (+{} -{}), factors {}->{} (+{} -{}), freedoms {freedoms_a}->{freedoms_b}",
                moves_a.len(),
                moves_b.len(),
                list(&(moves_b - moves_a)),
                list(&(moves_a - moves_b)),
                factors_a.len(),
                factors_b.len(),
                list(&(factors_b - factors_a)),
                list(&(factors_a - factors_b))
            );
        }
    }
    let (candidates_a, candidates_b) = (pa.candidates, pb.candidates);
    let mut moved = Bits::<W>::EMPTY;
    for x in (candidates_a & candidates_b).iter() {
        if pa.model.home(x) != pb.model.home(x) {
            moved.insert(x);
        }
    }
    println!(
        "  candidates gained {} lost {} home changed {}",
        list(&(candidates_b - candidates_a)),
        list(&(candidates_a - candidates_b)),
        list(&moved)
    );
    let mut scratch = Scratch::<W>::new();
    let mut show = |n: u32, own: &Prepared<W>, other: &Prepared<W>| {
        if let Some(set) = promotions::<W>(opt, n) {
            let here = set.is_subset(&own.candidates);
            let candidates = set.is_subset(&other.candidates);
            let promotable = candidates && other.model.promote_all_using(&mut scratch, &other.root, &set).is_some();
            println!(
                "  P* {n}'s optimal promotions: {} members, sum {} (candidates here: {here}); on the other board: all \
                 candidates {candidates}, promotable {promotable}",
                set.len(),
                set.sum()
            );
        }
    };
    show(a, &pa, &pb);
    show(b, &pb, &pa);
}

fn main() {
    let mut known: Vec<PathBuf> = Vec::new();
    let mut pairs = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--known" {
            known.push(args.next().expect("--known takes a results file").into());
        } else {
            let (a, b) = arg.split_once(':').expect("a pair is written A:B");
            pairs.push((a.parse::<u32>().expect("A is a board size"), b.parse::<u32>().expect("B is a board size")));
        }
    }
    let opt = Optimal::built_in().with_results_files(&known).unwrap_or_else(|e| panic!("{e}"));
    for (a, b) in pairs {
        taxman::with_width!(a.max(b), W => pair::<W>(a, b, &opt));
    }
}
