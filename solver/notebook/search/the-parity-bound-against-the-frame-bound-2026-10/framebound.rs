//! The frame bound at the root (the notebook entry beside this file).  It is an example of the frame solver kept out
//! of the build: copy it to solver/frame/examples/ to run it.
//!
//! ```text
//! cargo run --release --example framebound -- FROM TO OUT [lists]
//! ```
//!
//! For every board n of FROM..=TO it appends one JSON line to the file OUT, skipping the boards OUT already has (a
//! last line cut short is cut off first), so an interrupted run resumes.  Each board is set up as `taxman solve` sets
//! it up (`Board::for_n` at the width `with_width!` picks, `prepare` with the default tactics), and the line has:
//!
//! * `in_the_bag`: the score of the game with no promotions, ΣU\* (the kept-sources lemma, X1,
//!   docs/theory/exactness.md);
//! * `candidates`, `cand_sum`, `cand_digest`: the root's candidates, their sum and FNV-1a digest (64 bits, over the
//!   sorted list as 32-bit little-endian words), to compare with the prover's GF(2) candidates (`paritysplit.rs`
//!   beside this file);
//! * `max_total`: the promotion limit of the root;
//! * `frame`: the root's GF(2) bound with at most `max_total` promotions, the bound the search computes at its first
//!   node (the frame bound, B2, docs/theory/frame-bounds.md), and `frame_free` the same with no limit on the number
//!   of promotions;
//! * `secs`: the time of the line.
//!
//! With `lists` the line also has `cand_list`, the candidates, and `chosen`, the heaviest set found for `frame`.
//!
//! A board with nothing to promote has no candidates and the frame bound 0.

use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::Instant;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::gf2::{Gf2Bound, Gf2Graph};
use taxman::search::SearchTactics;

/// FNV-1a over the 32-bit little-endian words of `xs`.
fn digest(xs: &[u32]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for x in xs {
        for b in x.to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

fn line_of<const W: usize>(n: u32, lists: bool) -> String {
    let t0 = Instant::now();
    let board = Board::<W>::for_n(n);
    let cands = board.candidates().to_vec();
    let (mut max_total, mut frame, mut frame_free, mut chosen) = (0, 0, 0, Vec::new());
    if !board.is_trivial() {
        let p = board.prepare(SearchTactics::default()).expect("prepare");
        assert_eq!(p.candidates.to_vec(), cands, "n={n}: the search's candidates are the board's");
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let mut bound = Gf2Bound::<W>::new(&graph);
        bound.reset(&Bits::EMPTY, &p.candidates);
        let eval = bound.evaluate(p.max_total, None);
        (max_total, frame, chosen) = (p.max_total, eval.value, eval.chosen.to_vec());
        let mut free = Gf2Bound::<W>::new(&graph);
        free.reset(&Bits::EMPTY, &p.candidates);
        frame_free = free.evaluate(u32::MAX, None).value;
    }
    let list = |xs: &[u32]| xs.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    let extra = if lists {
        format!(",\"cand_list\":[{}],\"chosen\":[{}]", list(&cands), list(&chosen))
    } else {
        String::new()
    };
    format!(
        "{{\"n\":{n},\"in_the_bag\":{},\"candidates\":{},\"cand_sum\":{},\"cand_digest\":\"{:016x}\",\
         \"max_total\":{max_total},\"frame\":{frame},\"frame_free\":{frame_free},\"secs\":{:.3}{extra}}}",
        board.in_the_bag,
        cands.len(),
        cands.iter().map(|&x| u64::from(x)).sum::<u64>(),
        digest(&cands),
        t0.elapsed().as_secs_f64()
    )
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (from, to): (u32, u32) = (args[0].parse().unwrap(), args[1].parse().unwrap());
    let out = &args[2];
    let lists = args.get(3).is_some_and(|a| a == "lists");
    let text = fs::read_to_string(out).unwrap_or_default();
    let whole = text.rfind('\n').map_or(0, |at| at + 1);
    let done: HashSet<u32> = text[..whole]
        .lines()
        .filter_map(|l| l.strip_prefix("{\"n\":")?.split(',').next()?.parse().ok())
        .collect();
    let mut file = OpenOptions::new().create(true).append(true).open(out).expect("open the output file");
    if whole < text.len() {
        file.set_len(whole as u64).expect("cut off a line cut short");
    }
    for n in (from..=to).filter(|n| !done.contains(n)) {
        let line = taxman::with_width!(n, W => line_of::<W>(n, lists));
        writeln!(file, "{line}").expect("append a line");
        file.flush().expect("flush");
    }
}
