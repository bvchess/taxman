//! Where the conflicts of a node come from ([`Seek`]): the nogood table, else a peel, whose conflict is then learned.
//!
//! A *conflict* of a state is a set `K` of candidates that cannot be promoted together from it
//! ([`FrameGraph::conflict_using`]): every completion that can be promoted leaves out some member of `K` (a subset of a
//! promotable set is promotable).  A conflict of a state is a conflict of every state below it (they have more
//! promotions, which can only make promoting `K` harder) and of every candidate set containing it, so the conflicts
//! found are kept in the searcher's `known` for the rest of the node's loop and its subtree.

use crate::bits::Bits;
use crate::model::{ConflictCache, FrameGraph, FrameSet, Lookup, Nogoods, Scratch};

/// What a [`Seek`] needs to use and grow the searcher's nogood table (tactic `nogoods`).
pub(super) struct Learning<'r, const W: usize> {
    pub(super) table: &'r mut Nogoods<W>,
    /// The search's root, and the promotions of the node.
    pub(super) root: &'r FrameSet<W>,
    pub(super) promoted: Bits<W>,
}

/// Where a node's conflicts come from (the check of a node, and the look of conflict branching at a child): the
/// smallest the nogood table knows within a set (tactic `nogoods`), else a minimal conflict computed by a peel
/// ([`FrameGraph::conflict_cached`]), which is then learned as a nogood; either way kept in `known` for the node and
/// its subtree.
pub(super) struct Seek<'r, 'l, const W: usize> {
    pub(super) model: &'r FrameGraph,
    pub(super) scratch: &'r mut Scratch<W>,
    pub(super) cache: &'r mut ConflictCache<W>,
    pub(super) state: &'r FrameSet<W>,
    pub(super) known: &'r mut Vec<Bits<W>>,
    pub(super) learning: &'r mut Option<Learning<'l, W>>,
}

impl<const W: usize> Seek<'_, '_, W> {
    /// A conflict of `set` from the state, or [`Lookup::AllForced`] if the table knows one all of whose members are
    /// in `forced`, or [`Lookup::None`] if `set` can be promoted.
    pub(super) fn conflict(&mut self, set: &Bits<W>, forced: Bits<W>) -> Lookup<W> {
        if let Some(l) = self.learning.as_mut() {
            match l.table.lookup(&l.promoted, set, &forced) {
                Lookup::None => {}
                found => {
                    if let Lookup::Conflict(k) = found {
                        self.known.push(k);
                    }
                    return found;
                }
            }
        }
        let found = self.model.conflict_cached(self.scratch, Some(self.cache), self.state, set, true);
        let Some(k) = found else { return Lookup::None };
        if let Some(l) = self.learning.as_mut() {
            if let Some(n) = self.model.nogood(self.scratch, l.root, &l.promoted, &k) {
                l.table.insert(n);
            }
        }
        self.known.push(k);
        Lookup::Conflict(k)
    }
}
