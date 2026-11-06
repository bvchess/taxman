//! The bound of a node, as the search computes it when it starts a unit.
//!
//! The search is exact (solver/docs/design/search.md: every pruning drops only sets no heavier than the incumbent of
//! its time, sets that cannot be promoted, or, with the tactic `symmetry`, sets that break an inequality every heaviest
//! set keeps, the prime-exchange theorem (R2, docs/theory/search-rules.md)), so every promotable set heavier than the
//! incumbent (keeping the inequalities, under `symmetry`) is a completion of an open unit's node: its promotions, plus
//! some of its candidates (the dual-bound lemma (R4), docs/theory/search-rules.md).  The bound of a node is the GF(2)
//! bound of the node's candidates given its promotions, with at most [`max_additional`] more promotions (the frame
//! bound (B2), docs/theory/frame-bounds.md), which holds for every completion whatever the tactics.  The search computes
//! it when it starts the unit ([`TaxSearcher`](super::TaxSearcher)`::start`, then its first check), and the profile of
//! a search reads it for the root and the open units of a checkpoint ([`super::SearchProfile`]).

use super::setup::Prepared;
use super::TaxUnit;
use crate::bounds::max_additional;
use crate::gf2::{Gf2Bound, Gf2Graph};
use crate::model::Scratch;

/// The bound a search computes at the node of a unit when it starts it: [`NodeBound::completions`].
pub struct NodeBound<'a, const W: usize> {
    prepared: &'a Prepared<W>,
    gf2: Gf2Bound<'a, W>,
    scratch: Scratch<W>,
}

impl<'a, const W: usize> NodeBound<'a, W> {
    /// The bounds of the nodes of the search of `prepared`, whose root's GF(2) graph is `graph`
    /// (`Gf2Graph::new(&prepared.model, &prepared.root, &prepared.candidates)`, as the search builds it).
    pub fn new(prepared: &'a Prepared<W>, graph: &'a Gf2Graph) -> Self {
        NodeBound { prepared, gf2: Gf2Bound::new(graph), scratch: Scratch::new() }
    }

    /// The GF(2) bound on what a completion of `unit`'s node adds to its promotions: the heaviest common independent
    /// set of the node's candidates, given its promotions, of at most [`max_additional`] members.  `None` if the
    /// unit's promotions cannot be promoted from the root (no unit of a search of this game is such).
    pub fn completions(&mut self, unit: &TaxUnit<W>) -> Option<u64> {
        let p = self.prepared;
        let state = p.model.promote_all_using(&mut self.scratch, &p.root, &unit.promoted)?;
        let max_add = max_additional(p.max_total, &state, &unit.promoted, &unit.cands);
        self.gf2.reset(&unit.promoted, &unit.cands);
        Some(self.gf2.evaluate(max_add, None).value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bits::Bits;
    use crate::board::Board;
    use crate::game::Optimal;
    use crate::search::SearchTactics;
    use crate::test_support::QUICK;

    /// The bound of the root's node is the root's GF(2) bound with no incumbent, computed on its own, and it is not
    /// below P*, the best promotion sum of game `n`.
    fn the_root_node<const W: usize>(n: u32) {
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default()).unwrap();
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let mut gf2 = Gf2Bound::<W>::new(&graph);
        gf2.reset(&Bits::EMPTY, &p.candidates);
        let root = gf2.evaluate(p.max_total, None).value;
        let unit = TaxUnit { promoted: Bits::EMPTY, cands: p.candidates };
        assert_eq!(NodeBound::new(&p, &graph).completions(&unit), Some(root), "n={n}");
        let opt = Optimal::built_in().score(n).expect("an entry of the table") - board.in_the_bag;
        assert!(opt <= root, "n={n}: the root's bound {root} is below P* {opt}");
    }

    /// Every 89th game of 60..=1300 (every 267th in a quick run).
    #[test]
    fn the_bound_of_the_root_node_is_the_roots_gf2_bound_and_not_below_p_star() {
        for n in (60..=1300).step_by(if QUICK { 267 } else { 89 }) {
            crate::with_width!(n, W => the_root_node::<W>(n));
        }
    }
}
