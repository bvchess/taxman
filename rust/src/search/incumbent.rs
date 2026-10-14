//! The incumbent a search starts from: the best of the greedy set ([`crate::bounds::greedy`], made by
//! [`prepare`](super::prepare)), what [`surviving`] of the seed (the previous game's promotions), and the
//! [`dive`] (with the matching bound), improved by a local search ([`Neighbourhood::improve`]).  Every one of them
//! is a set of candidates that can be promoted, so any of them is a valid incumbent; a good one only makes the
//! search prune sooner.  The local search also runs on several threads ([`Neighbourhood::improve_on`]), with the
//! answer of the sequential one.

use super::setup::Prepared;
use crate::bits::Bits;
use crate::matchbound::{MatchBound, MatchGraph};
use crate::model::{FrameSet, Model, Scratch};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread;

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
    ///
    /// One pass tries the moves in the order of their numbers (`Move`); the first that raises the sum is kept, and
    /// the next pass starts over from the new set.  [`improve_on`](Self::improve_on) finds the same moves on more
    /// threads.
    pub fn improve(&self, scratch: &mut Scratch<W>, start: &Bits<W>, pairs: bool) -> Bits<W> {
        let mut best = self.fill(scratch, start, &Bits::EMPTY);
        while let Some(better) = self.first_improvement(scratch, &best, pairs) {
            best = better;
        }
        best
    }

    /// [`improve`](Self::improve) on `threads()` threads, with exactly its answer.  The refills of one pass are
    /// independent, so the threads take the moves of a pass in the order of their numbers, and the improvement with
    /// the lowest number is the one the sequential pass would have kept.  With one thread it is `improve`.  The
    /// number of threads is read at the start of each pass, so a caller whose number changes meanwhile (a run's
    /// workers) has the new one from the next pass; the answer does not depend on it.
    pub fn improve_on(&self, threads: impl Fn() -> usize, start: &Bits<W>, pairs: bool) -> Bits<W> {
        let mut scratch = Scratch::new();
        let mut best = self.fill(&mut scratch, start, &Bits::EMPTY);
        loop {
            let threads = threads();
            let better = if threads <= 1 {
                self.first_improvement(&mut scratch, &best, pairs)
            } else {
                self.first_improvement_on(threads, &best, pairs)
            };
            match better {
                Some(better) => best = better,
                None => return best,
            }
        }
    }

    /// `best` with the members that move number `i` of a pass over its `members` (ascending) names taken out and
    /// the rest refilled without them, if that is heavier than `best`.
    fn try_move(&self, scratch: &mut Scratch<W>, best: &Bits<W>, members: &[u32], i: usize) -> Option<Bits<W>> {
        let out = match Move::numbered(members.len(), i) {
            Move::Single(x) => Bits::EMPTY.with(members[x]),
            Move::Pair(x, y) => Bits::EMPTY.with(members[x]).with(members[y]),
        };
        let other = self.fill(scratch, &(*best - out), &out);
        (other.sum() > best.sum()).then_some(other)
    }

    /// The set of the first move of a pass over `best` that raises the sum, in the order of the moves' numbers.
    fn first_improvement(&self, scratch: &mut Scratch<W>, best: &Bits<W>, pairs: bool) -> Option<Bits<W>> {
        let members = best.to_vec();
        (0..Move::count(members.len(), pairs)).find_map(|i| self.try_move(scratch, best, &members, i))
    }

    /// [`first_improvement`](Self::first_improvement) on `threads` threads.  Each takes move numbers in increasing
    /// order from one counter and stops when its next number is above the lowest one known to improve, so every
    /// number below the lowest improving one has been tried to the end when the threads have all stopped, and
    /// that move is the first.
    fn first_improvement_on(&self, threads: usize, best: &Bits<W>, pairs: bool) -> Option<Bits<W>> {
        let members = best.to_vec();
        let moves = Move::count(members.len(), pairs);
        let next = AtomicUsize::new(0);
        let lowest = AtomicUsize::new(usize::MAX);
        let found: Mutex<Option<(usize, Bits<W>)>> = Mutex::new(None);
        thread::scope(|scope| {
            for _ in 0..threads.min(moves) {
                scope.spawn(|| {
                    // One per thread and pass: it is `512 * W` bytes, nothing against a pass's refills.
                    let mut scratch = Scratch::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        if i >= moves || i > lowest.load(Ordering::Relaxed) {
                            break;
                        }
                        if let Some(better) = self.try_move(&mut scratch, best, &members, i) {
                            lowest.fetch_min(i, Ordering::Relaxed);
                            let mut slot = found.lock().unwrap_or_else(PoisonError::into_inner);
                            if slot.as_ref().is_none_or(|(kept, _)| i < *kept) {
                                *slot = Some((i, better));
                            }
                        }
                    }
                });
            }
        });
        found.into_inner().unwrap_or_else(PoisonError::into_inner).map(|(_, better)| better)
    }
}

/// A move of the local search: members of the set taken out, to refill without them.  A pass over the `k` members
/// of the set (ascending) tries the moves in the order of their numbers, so that the order exists once and a
/// move can be named without listing the others: the `k` singles first, then (with `pairs`) the pairs.  Single
/// `x` is number `x`; the pair of members `x` and `y < x` is number `k + x(x − 1)/2 + y`, so the pairs come by `x`
/// ascending and then `y` ascending, as two nested loops would visit them.  The members are named by their index
/// in the ascending list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Move {
    Single(usize),
    Pair(usize, usize),
}

impl Move {
    /// How many moves a pass over `k` members has.
    fn count(k: usize, pairs: bool) -> usize {
        k + if pairs { k * k.saturating_sub(1) / 2 } else { 0 }
    }

    /// Move number `i` of a pass over `k` members (`i` below [`count`](Self::count) with `pairs`).
    fn numbered(k: usize, i: usize) -> Move {
        match i.checked_sub(k) {
            None => Move::Single(i),
            Some(j) => {
                // The `x` with x(x − 1)/2 <= j < x(x + 1)/2: the largest with (2x − 1)² <= 8j + 1.
                let x = (8 * j + 1).isqrt().div_ceil(2);
                Move::Pair(x, j - x * (x - 1) / 2)
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::game::Optimal;
    use crate::search::SearchTactics;
    use std::cell::Cell;

    /// The local search as the nested loops spell it, which `Move` numbers: what `improve` must reproduce.
    fn improve_by_loops<const W: usize>(hood: &Neighbourhood<W>, start: &Bits<W>, pairs: bool) -> Bits<W> {
        let mut scratch = Scratch::new();
        let mut best = hood.fill(&mut scratch, start, &Bits::EMPTY);
        'search: loop {
            for x in best.iter() {
                let other = hood.fill(&mut scratch, &best.without(x), &Bits::EMPTY.with(x));
                if other.sum() > best.sum() {
                    best = other;
                    continue 'search;
                }
            }
            if pairs {
                for x in best.iter() {
                    for y in best.iter().filter(|&y| y < x) {
                        let out = Bits::EMPTY.with(x).with(y);
                        let other = hood.fill(&mut scratch, &(best - out), &out);
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

    #[test]
    fn the_moves_of_a_pass_are_numbered_in_the_order_of_the_nested_loops() {
        for k in 0..=30 {
            let mut loops: Vec<Move> = (0..k).map(Move::Single).collect();
            loops.extend((0..k).flat_map(|x| (0..x).map(move |y| Move::Pair(x, y))));
            assert_eq!(Move::count(k, true), loops.len(), "k={k}");
            assert_eq!(Move::count(k, false), k, "k={k}");
            let numbered: Vec<Move> = (0..loops.len()).map(|i| Move::numbered(k, i)).collect();
            assert_eq!(numbered, loops, "k={k}");
        }
        // The largest passes (4000 members, 8 million moves), and every pair number up to 300,000: the pair
        // numbered j is (x, y) with y < x and x(x − 1)/2 + y = j.
        let k = 4000;
        assert_eq!(Move::count(k, true), k + k * (k - 1) / 2);
        assert_eq!(Move::numbered(k, k - 1), Move::Single(k - 1));
        assert_eq!(Move::numbered(k, k), Move::Pair(1, 0));
        assert_eq!(Move::numbered(k, Move::count(k, true) - 1), Move::Pair(k - 1, k - 2));
        for j in 0..300_000 {
            let Move::Pair(x, y) = Move::numbered(7, 7 + j) else { panic!("number {} is a pair", 7 + j) };
            assert!(y < x && x * (x - 1) / 2 + y == j, "pair number {j} came out as ({x}, {y})");
        }
    }

    /// `improve_on` on 0 to 4 threads gives the set of `improve`, which is the nested loops', from the greedy set and
    /// from the survivors of the previous game, with and without pairs.  Returns how many of the local searches
    /// improved their start.
    fn same_answers<const W: usize>(n: u32) -> usize {
        let board = Board::<W>::for_n(n);
        let prepared = board.prepare(SearchTactics::default()).unwrap();
        let hood = Neighbourhood { model: &prepared.model, root: &prepared.root, candidates: &prepared.candidates };
        let seed = board.seed(Optimal::built_in().moves(n - 1).expect("optimal.json has the game"));
        let starts = [("greedy", prepared.greedy), ("seed", surviving(&prepared, &seed))];
        let mut improved = 0;
        for (what, start) in starts {
            for pairs in [false, true] {
                let expected = hood.improve(&mut Scratch::new(), &start, pairs);
                let by_loops = improve_by_loops(&hood, &start, pairs);
                assert_eq!(expected, by_loops, "n={n} from the {what} set, pairs={pairs}: improve and the loops");
                assert!(expected.sum() >= start.sum());
                improved += usize::from(expected.sum() > start.sum());
                for threads in 0..=4 {
                    let got = hood.improve_on(|| threads, &start, pairs);
                    assert_eq!(got, expected, "n={n} from the {what} set, pairs={pairs}, {threads} threads");
                }
                // A number that changes from pass to pass (read once per pass) changes nothing either.
                let reads = Cell::new(0);
                let got = hood.improve_on(
                    || {
                        reads.set(reads.get() + 1);
                        reads.get() % 5
                    },
                    &start,
                    pairs,
                );
                assert_eq!(got, expected, "n={n} from the {what} set, pairs={pairs}, a changing number of threads");
                assert!(reads.get() > 0, "the number of threads is read at the start of each pass");
            }
        }
        improved
    }

    #[test]
    fn improve_on_threads_gives_the_sequential_answer() {
        let mut improved = 0;
        for n in [300, 600, 960, 1215] {
            improved += crate::with_width!(n, W => same_answers::<W>(n));
        }
        assert!(improved > 0, "no local search improved its start, so the comparison showed nothing");
    }
}
