//! What the search takes from the Java's solver: the limit on a node's further promotions ([`max_additional`]) and the
//! first incumbent ([`greedy`]).
//!
//! The Java bounds every node with a bound of its own and filters the candidates of a child with a preclusion table and
//! promotability checks.  Here a bound over the node's candidates bounds every node (the GF(2) bound, [`crate::gf2`],
//! by default; the matching bound, [`crate::matchbound`], on the matching path), the conflicts it finds take the place
//! of the checks, and a child's candidates are only narrowed to the frames that still have freedoms
//! ([`FrameGraph::narrow`]: [`FrameSet::narrow_candidates`], and the one-sided candidates of the extended problem whose
//! own frame has a freedom).

use crate::bits::Bits;
use crate::model::{FrameGraph, FrameSet, Scratch};

/// How many more promotions a node can make, given the overall limit `max_total`, the promotions made so far, and the
/// candidates left; never negative.  (A count: the bound on the number of further promotions that the freedoms give,
/// see [`FrameSet::max_promotions`] and [`FrameGraph::max_promotions_among`].)
#[inline]
pub fn max_additional<const W: usize>(
    model: &FrameGraph,
    max_total: u32,
    state: &FrameSet<W>,
    promoted: &Bits<W>,
    cands: &Bits<W>,
) -> u32 {
    let by_state = model.max_promotions_among(state, cands).max(0) as u32;
    max_total.saturating_sub(promoted.len()).min(by_state)
}

/// From the top, promote every candidate that can be; after each success, narrow the candidates in the new state and go
/// on with the next one below in the narrowed set.  Returns the promotions made: the search's first incumbent.
pub fn greedy<const W: usize>(model: &FrameGraph, root: &FrameSet<W>, cands: &Bits<W>) -> Bits<W> {
    let mut scratch = Scratch::new();
    let mut state = *root;
    let mut cands = *cands;
    let mut promoted = Bits::EMPTY;
    let mut next = cands.max();
    while let Some(n) = next {
        if model.promote_in_place_using(&mut scratch, &mut state, n) {
            cands = model.narrow(&state, &cands);
            promoted.insert(n);
        }
        next = cands.max_below(n);
    }
    promoted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::test_support::*;
    use crate::numtheory::Tables;

    fn setup<const W: usize>(g: &Golden) -> (FrameGraph, FrameSet<W>, Bits<W>) {
        let (model, root) = FrameGraph::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
        (model, root, Bits::from_slice(&g.candidates))
    }

    fn check_golden_greedy<const W: usize>(g: &Golden) {
        let (model, root, cands) = setup::<W>(g);
        let got = greedy(&model, &root, &cands).to_vec();
        assert_eq!(got, u32s(&g.json, "greedy"), "n={}", g.n);
        if let Some(manager) = g.json.get("searchManager").filter(|j| !j.is_null()) {
            assert_eq!(got, u32s(manager, "initialBest"), "n={}", g.n);
        }
    }

    #[test]
    fn golden_greedy() {
        for n in golden_ns() {
            let g = load(n);
            if n < 512 {
                check_golden_greedy::<8>(&g);
            } else {
                check_golden_greedy::<16>(&g);
            }
        }
    }

    #[test]
    fn greedy_and_max_additional_at_the_root_of_85() {
        let g = load(85);
        let (model, root, cands) = setup::<2>(&g);
        let best = greedy(&model, &root, &cands);
        assert_eq!(best.to_vec(), vec![28, 40, 42]);
        let max_total = root.max_promotions_among(&cands) as u32;
        assert_eq!(max_total, 3);
        let none = Bits::EMPTY;
        assert_eq!(max_additional(&model, max_total, &root, &none, &cands), 3);
        assert_eq!(max_additional(&model, 2, &root, &Bits::from_slice(&[42, 40, 28]), &cands), 0);
    }
}
