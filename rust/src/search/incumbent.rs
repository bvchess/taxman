//! The incumbent a search starts from: the best of the greedy set ([`crate::bounds::greedy`], made by
//! [`prepare`](super::prepare)), what [`surviving`] of the seed (the previous game's promotions), and the
//! [`dive`] (with the matching bound), improved by a local search ([`Neighbourhood::improve`]).  Every one of them
//! is a set of candidates that can be promoted, so any of them is a valid incumbent; a good one only makes the
//! search prune sooner.

use super::setup::Prepared;
use crate::bits::Bits;
use crate::matchbound::{MatchBound, MatchGraph};
use crate::model::{FrameSet, Model, Scratch};

/// The candidates of `seed` that can still be promoted, one by one from the largest down, skipping the ones
/// that fail.
pub(super) fn surviving<const W: usize>(prepared: &Prepared<W>, seed: &Bits<W>) -> Bits<W> {
    let mut state = prepared.root;
    let mut kept = Bits::EMPTY;
    for n in (*seed & prepared.candidates).iter_desc() {
        if prepared.model.promote_in_place(&mut state, n) {
            kept.insert(n);
        }
    }
    kept
}

/// Where a local search for better promotion sets works: the search's model, root and candidates.
#[derive(Clone, Copy)]
pub struct Neighbourhood<'a, const W: usize> {
    pub model: &'a Model,
    pub root: &'a FrameSet<W>,
    pub candidates: &'a Bits<W>,
}

impl<const W: usize> Neighbourhood<'_, W> {
    /// `base` (which must be feasible) with candidates outside `excluded` added greedily, from the largest down,
    /// narrowing the candidates after each promotion as [`crate::bounds::greedy`] does.
    fn fill(&self, scratch: &mut Scratch<W>, base: &Bits<W>, excluded: &Bits<W>) -> Bits<W> {
        let mut state = self.model.promote_all_using(scratch, self.root, base).expect("a feasible base");
        let mut out = *base;
        let mut open = state.narrow_candidates(&(*self.candidates - *base - *excluded));
        let mut next = open.max();
        while let Some(n) = next {
            if self.model.promote_in_place_using(scratch, &mut state, n) {
                out.insert(n);
                open = state.narrow_candidates(&open);
            }
            next = open.max_below(n);
        }
        out
    }

    /// A promotion set at least as good as `start` (which must be feasible), by local search: fill `start`
    /// greedily, then repeatedly take one member out (and with `pairs`, then two) and refill greedily without
    /// them, keeping the first change that raises the sum, until none does.  A few milliseconds with `pairs` at
    /// n ≈ 1000, less without.
    pub fn improve(&self, scratch: &mut Scratch<W>, start: &Bits<W>, pairs: bool) -> Bits<W> {
        let mut best = self.fill(scratch, start, &Bits::EMPTY);
        'search: loop {
            for x in best.iter() {
                let other = self.fill(scratch, &best.without(x), &Bits::EMPTY.with(x));
                if other.sum() > best.sum() {
                    best = other;
                    continue 'search;
                }
            }
            if pairs {
                for x in best.iter() {
                    for y in best.iter().filter(|&y| y < x) {
                        let out = Bits::EMPTY.with(x).with(y);
                        let other = self.fill(scratch, &(best - out), &out);
                        if other.sum() > best.sum() {
                            best = other;
                            continue 'search;
                        }
                    }
                }
            }
            return best;
        }
    }
}

/// A promotion set found by rounding the matching relaxation at `root` (a starting incumbent): while the
/// heaviest matching's candidates cannot all be promoted, take the smallest member of one of their conflicts
/// (see [`Model::conflict_using`]) out of the candidates and solve again.  Ends with candidates that can all be
/// promoted.
pub(super) fn dive<const W: usize>(model: &Model, root: &FrameSet<W>, graph: &MatchGraph, cands: &Bits<W>) -> Bits<W> {
    let mut m = MatchBound::new(graph);
    m.reset(&Bits::EMPTY, cands);
    let mut scratch = Scratch::new();
    loop {
        let chosen = m.chosen();
        match model.conflict_using(&mut scratch, root, &chosen, true).and_then(|k| k.min()) {
            Some(x) => m.remove_candidate(x),
            None => return chosen,
        }
    }
}
