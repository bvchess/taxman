//! Conflicts: small sets of candidates that cannot be promoted together, and the nogoods made from them.
//!
//! A *conflict* of a set `S` in a state is a subset of `S` that cannot be promoted from that state.  A subset of a
//! promotable set is promotable (fewer moves added and fewer factors removed only help a frame), so every set
//! that can be promoted leaves out at least one member of every conflict.  The search branches and refines its
//! bound on that ([`crate::search`]): when the heaviest matching's candidates cannot all be promoted, a small
//! conflict among them says which ones to try leaving out.
//!
//! A conflict is found in a frame that fails, from its *residual* (the moves a peel cannot drop, see
//! [`super::peel`]), and made minimal ([`FrameGraph::conflict_using`]).  A conflict of a node is relative to the
//! node's promotions; [`FrameGraph::nogood`] turns it into a conflict of the root, a *nogood*, which holds at every
//! node ([`super::nogoods`]).

use super::peel::Scratch;
use super::{Frame, FrameGraph, FrameSet, MAX_FRAMES};
use crate::bits::Bits;

impl FrameGraph {
    /// A *conflict* of `set` in `state`: a subset of `set` that cannot be promoted from `state`, or `None` if
    /// all of `set` can be.  Since a subset of a set that can be promoted can be promoted too (see
    /// [`winnable`](Self::winnable)), every set that can be promoted leaves out some member of a conflict.
    ///
    /// The conflict is small: a single member if one is not a factor of a home frame `h ≥ 1`; else, for a frame
    /// that is not winnable with `set` promoted, the members that its residual (the moves a peel cannot drop)
    /// needs: the ones promoted into the frame that are residual moves, and the ones promoted out of it that are
    /// factors of residual moves.  With only those promoted, the residual is still there (its moves, and the same
    /// live factors), so the frame is still unwinnable.
    ///
    /// With `minimal`, that set is then reduced to a minimal one that keeps the frame unwinnable, by trying to
    /// leave out each member, from the smallest up (so the larger members tend to stay).  Each try peels only
    /// the moves of the current residual: with fewer members promoted the frame has fewer moves and more factors,
    /// so a residual it has is a set of moves that is stuck (each live factor shared within the set) with the
    /// current members too, and the current residual contains every such set.  Of the frames that fail, the one
    /// giving the smallest conflict is used (the first of them, if several are as small).
    pub fn conflict_using<const W: usize>(
        &self,
        scratch: &mut Scratch<W>,
        state: &FrameSet<W>,
        set: &Bits<W>,
        minimal: bool,
    ) -> Option<Bits<W>> {
        self.conflict_cached(scratch, None, state, set, minimal)
    }

    /// [`conflict_using`](Self::conflict_using), with the frames' answers kept in `cache` (if given) and reused
    /// when a frame and its edits are the same as last time: the same answer, often without peeling.
    pub fn conflict_cached<const W: usize>(
        &self,
        scratch: &mut Scratch<W>,
        mut cache: Option<&mut ConflictCache<W>>,
        state: &FrameSet<W>,
        set: &Bits<W>,
        minimal: bool,
    ) -> Option<Bits<W>> {
        let (edits, not_a_factor) = self.edits(state, set);
        if let Some(x) = not_a_factor {
            return Some(Bits::EMPTY.with(x));
        }
        let mut best: Option<Bits<W>> = None;
        for (i, frame) in state.frames().iter().enumerate() {
            let (gained, lost) = (edits.gained[i], edits.lost[i]);
            if gained.is_empty() && lost.is_empty() {
                continue;
            }
            let key = FrameEdits { moves: frame.moves, factors: frame.factors, gained, lost, minimal };
            let conflict = match cache.as_deref_mut() {
                Some(cache) => match cache.entries[i].as_ref().filter(|e| e.0 == key) {
                    Some(&(_, answer)) => answer,
                    None => {
                        let answer = self.frame_conflict(scratch, frame, &gained, &lost, minimal);
                        cache.entries[i] = Some((key, answer));
                        answer
                    }
                },
                None => self.frame_conflict(scratch, frame, &gained, &lost, minimal),
            };
            let Some(conflict) = conflict else { continue };
            if best.is_none_or(|b| conflict.len() < b.len()) {
                best = Some(conflict);
                if conflict.len() == 1 {
                    break; // none can be smaller
                }
            }
        }
        best
    }

    /// A nogood of `root` (see [`Nogoods`](super::Nogoods)) from the conflict `k` of the state reached from `root` by
    /// promoting `promoted`: a minimal subset of `promoted ∪ k` that keeps a frame of `root` unwinnable, found as
    /// [`conflict_using`](Self::conflict_using) finds one, in each frame whose edits include all of `k` (the
    /// conflicts it makes come from one frame each, so it is one of them); the smallest.  The state's frames are
    /// the root's with `promoted` promoted (and reduced, which does not change winnability), so such a frame is
    /// unwinnable.  The nogood minus `promoted` is a subset of `k`: as small a conflict of the state, and in
    /// practice `k` itself.  `None` if no such frame is unwinnable, which cannot happen for a conflict.
    pub fn nogood<const W: usize>(
        &self,
        scratch: &mut Scratch<W>,
        root: &FrameSet<W>,
        promoted: &Bits<W>,
        k: &Bits<W>,
    ) -> Option<Bits<W>> {
        let (edits, not_a_factor) = self.edits(root, &(*promoted | *k));
        if let Some(x) = not_a_factor {
            return Some(Bits::EMPTY.with(x));
        }
        let mut best: Option<Bits<W>> = None;
        for (i, frame) in root.frames().iter().enumerate() {
            let (gained, lost) = (&edits.gained[i], &edits.lost[i]);
            if !k.is_subset(&(*gained | *lost)) {
                continue;
            }
            if let Some(n) = self.frame_conflict(scratch, frame, gained, lost, true) {
                if best.is_none_or(|b| n.len() < b.len()) {
                    best = Some(n);
                }
            }
        }
        best
    }

    /// For each frame that `set` leaves unwinnable in `state`, the members of `set` that its residual needs (those
    /// promoted into it that are residual moves, and those promoted out of it that are factors of residual moves),
    /// from which [`conflict_using`](Self::conflict_using) cuts its minimal conflict; their union.  The tactic
    /// `conflict_branching` prices them.
    pub fn conflict_members<const W: usize>(
        &self,
        scratch: &mut Scratch<W>,
        state: &FrameSet<W>,
        set: &Bits<W>,
    ) -> Bits<W> {
        let (edits, not_a_factor) = self.edits(state, set);
        if let Some(x) = not_a_factor {
            return Bits::EMPTY.with(x);
        }
        let g = &self.graph;
        let mut out = Bits::EMPTY;
        for (i, frame) in state.frames().iter().enumerate() {
            let (gained, lost) = (edits.gained[i], edits.lost[i]);
            if gained.is_empty() && lost.is_empty() {
                continue;
            }
            let core = scratch.stuck(&(frame.moves | gained), &(frame.factors - lost), g);
            let mut factors = Bits::EMPTY;
            for m in core.iter() {
                for &q in g.factors(m) {
                    factors.insert(u32::from(q));
                }
            }
            out |= (gained & core) | (lost & factors & frame.factors);
        }
        out
    }

    /// The conflicts of `set` in `state` that the tactic `conflict_branching` weighs: for each frame that `set` leaves
    /// unwinnable, the minimal conflict that [`conflict_using`](Self::conflict_using) makes, but trying to leave out
    /// the members in the order `order` (which must list every member of `set`) instead of from the smallest up.  A
    /// member tried early tends to go, so an order from the cheapest member keeps the members that cost most to leave
    /// out.
    ///
    /// Each is a conflict, so no completion holds all of it.  With its members alone applied, its frame keeps a stuck
    /// set of moves (each live factor listed by two of them or more), so the frame is not winnable
    /// (the peel theorem (P4), docs/theory/playability.md).  A stuck set survives any larger set of promotions, which
    /// only adds moves and takes away factors (it stays a core: the residual theorem, P5), so no set holding the whole
    /// conflict leaves the frame winnable, and none can be promoted (the exactness theorem (X7),
    /// docs/theory/exactness.md).  Each is checked by the whole peel, apart from the search that made it.
    pub fn conflicts_ordered<const W: usize>(
        &self,
        scratch: &mut Scratch<W>,
        state: &FrameSet<W>,
        set: &Bits<W>,
        order: &[u32],
    ) -> Vec<Bits<W>> {
        let (edits, not_a_factor) = self.edits(state, set);
        if let Some(x) = not_a_factor {
            return vec![Bits::EMPTY.with(x)];
        }
        let g = &self.graph;
        let mut out = Vec::new();
        for (i, frame) in state.frames().iter().enumerate() {
            let (gained, lost) = (edits.gained[i], edits.lost[i]);
            if gained.is_empty() && lost.is_empty() {
                continue;
            }
            let near = |core: &Bits<W>| -> Bits<W> {
                let mut out = Bits::EMPTY;
                for m in core.iter() {
                    for &q in g.factors(m) {
                        out.insert(u32::from(q));
                    }
                }
                out & frame.factors
            };
            let mut core = scratch.stuck(&(frame.moves | gained), &(frame.factors - lost), g);
            if core.is_empty() {
                continue;
            }
            let mut factors = near(&core);
            let mut conflict = (gained & core) | (lost & factors);
            for &x in order {
                if !conflict.contains(x) {
                    continue;
                }
                let fewer = conflict.without(x);
                let moves = (core & frame.moves) | (fewer & gained);
                let left = scratch.stuck(&moves, &(factors - (fewer & lost)), g);
                if !left.is_empty() {
                    core = left;
                    factors = near(&core);
                    conflict = fewer & ((gained & core) | (lost & factors));
                }
            }
            // A conflict (see above): the whole peel finds the frame unwinnable with its members alone applied.
            assert!(
                !self.winnable_using(
                    scratch,
                    &(frame.moves | (conflict & gained)),
                    &(frame.factors - (conflict & lost))
                ),
                "an ordered conflict {conflict:?} of frame {i} leaves the frame winnable"
            );
            out.push(conflict);
        }
        out
    }

    /// The conflict of [`conflict_using`](Self::conflict_using) for one frame with `gained` promoted into it and
    /// `lost` promoted out of it: `None` if it stays winnable.
    fn frame_conflict<const W: usize>(
        &self,
        scratch: &mut Scratch<W>,
        frame: &Frame<W>,
        gained: &Bits<W>,
        lost: &Bits<W>,
        minimal: bool,
    ) -> Option<Bits<W>> {
        let g = &self.graph;
        // The factors of the frame that the moves of the residual (`core`) can pay with, live or promoted out.
        let near = |core: &Bits<W>| -> Bits<W> {
            let mut out = Bits::EMPTY;
            for m in core.iter() {
                for &q in g.factors(m) {
                    out.insert(u32::from(q));
                }
            }
            out & frame.factors
        };
        let mut core = scratch.stuck(&(frame.moves | *gained), &(frame.factors - *lost), g);
        if core.is_empty() {
            return None;
        }
        let mut factors = near(&core);
        let mut conflict = (*gained & core) | (*lost & factors);
        if minimal {
            let members = conflict;
            for x in members.iter() {
                if !conflict.contains(x) {
                    continue;
                }
                let fewer = conflict.without(x);
                let moves = (core & frame.moves) | (fewer & *gained);
                let left = scratch.stuck(&moves, &(factors - (fewer & *lost)), g);
                if !left.is_empty() {
                    core = left;
                    factors = near(&core);
                    conflict = fewer & ((*gained & core) | (*lost & factors));
                }
            }
        }
        debug_assert!(!conflict.is_empty(), "the frames of a state are winnable");
        Some(conflict)
    }
}

/// One frame with promotions into and out of it, and the kind of conflict asked for: the key of a
/// [`ConflictCache`] entry.
#[derive(Clone, Copy, PartialEq, Eq)]
struct FrameEdits<const W: usize> {
    moves: Bits<W>,
    factors: Bits<W>,
    gained: Bits<W>,
    lost: Bits<W>,
    minimal: bool,
}

/// The last answer [`FrameGraph::conflict_cached`] found for each frame, to reuse when the frame and its edits are the
/// same: along a search, consecutive heaviest matchings mostly promote the same numbers in most frames.
pub struct ConflictCache<const W: usize> {
    entries: [Option<(FrameEdits<W>, Option<Bits<W>>)>; MAX_FRAMES],
}

impl<const W: usize> Default for ConflictCache<W> {
    fn default() -> Self {
        ConflictCache { entries: [None; MAX_FRAMES] }
    }
}
