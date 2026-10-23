//! The near-repeat cut (tactic `near_repeat`): a node of game N's search is cut when its completions are promotion sets
//! of a game M proven before, whose proven bound the incumbent reaches (docs/theory/ledger.md, "The near-repeat cut":
//! the state lemma and Lemma R).  [`NearRepeat`] is the cut's setup, and its methods are the checks at a node, which
//! follow the ledger's "What a search checks" word for word; the searcher keeps σ_M(P) along its path and calls them
//! (`search::searcher`).
//!
//! The words: a node has the promotions P, the candidates C its loop has still to try, and the state σ_N(P) in Π_N, the
//! problem searched; σ_M(P) is the state of P in Π_M, the problem of game M.  B is a proven bound of P\*(Π_M).
//!
//! At setup ([`NearRepeat::new`] and [`NearRepeat::reached`]):
//!
//! * M is proven, and B is read from M's result, which the caller sees to: `crate::board::near_repeat` takes M = n − 1
//!   and B = score(M) − in_the_bag(M), and `crate::solver` gives it the game of n − 1 only once that game is concluded,
//!   is proven, and assumes nothing: with the search's go, or, for a search that runs by then, at the conclusion (it
//!   arms the search's cut, which the searchers pick up as each starts a unit).
//! * Π_M is set up as Π_N is.  The candidates of Π_N that are candidates of Π_M are marked, and among them those with
//!   the same home in both.
//! * The incumbent, or the floor, is worth at least B.  A search armed below it, from a provisional game of n − 1 that
//!   the game improved on, say, cuts nothing until the incumbent reaches B: the searcher compares the incumbent its
//!   step read with B before it checks a node.  The incumbent only rises, so this holds at every later node.
//!
//! At a node:
//!
//! 1. every member of P is marked, and every member of C is marked as having the same home ([`NearRepeat::marked`] and
//!    [`NearRepeat::candidates_marked`]);
//! 2. P, promoted from the root of Π_M, gives σ_M(P) ([`NearRepeat::state_of`]; the searcher promotes a child's from
//!    its parent's, [`NearRepeat::promote`], which gives the same state: the state after promoting a set is the same in
//!    any order, `crate::model`'s promote module);
//! 3. on every frame j that is the home h of a member of C, or h − 1: every move of σ_M(P)'s frame j is a move of
//!    σ_N(P)'s; and every factor y of σ_N(P)'s frame j that σ_M(P)'s lacks is not in C, and no number in its list
//!    `composites(y)` of Π_N's factor graph is a move of σ_M(P)'s frame j or a member of C
//!    ([`NearRepeat::frames_agree`]).
//!
//! Why each is needed: a set holding a number that is no candidate of Π_M is no promotion set of it, though Π_M's model
//! may promote it (22 at n = 98); a promoted number with another home in Π_M would change frames that the check did not
//! compare; and a provisional game of M bounds nothing.  The cut drops only nodes none of whose completions beats the
//! incumbent, as a bound does, so the search stays exact; it learns nothing.  It does not treat the one-sided
//! candidates of the extended problem, and is never set up for a problem that has them.

use super::setup::Prepared;
use crate::bits::Bits;
use crate::model::{FactorProvider, FrameGraph, FrameSet, Scratch, MAX_FRAMES};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// The setup of the near-repeat cut of a search of Π_N by a game M proven before (see the module docs): Π_M's model and
/// root, the marks of Π_N's candidates, and B.
#[derive(Debug)]
pub struct NearRepeat<const W: usize> {
    /// Π_M: the model and the root frame set of game M, set up as Π_N's.
    model: Arc<FrameGraph>,
    root: FrameSet<W>,
    /// The candidates of Π_N that are candidates of Π_M.
    marked: Bits<W>,
    /// Those of them with the same home in both problems.
    same_home: Bits<W>,
    /// For each frame h of Π_N, the members of `same_home` whose home is h: which frames a set of candidates touches.
    by_home: [Bits<W>; MAX_FRAMES],
    /// B: a proven bound of P\*(Π_M).
    bound: u64,
    /// The nodes the searchers that used this setup have cut, as each adds its count when it flushes its counters.
    cut: AtomicU64,
}

impl<const W: usize> NearRepeat<W> {
    /// The cut of a search of `n` by the game of `m`, both set up as [`prepare`](super::prepare) sets a game up, with
    /// `bound`, a proven bound of the best promotion sum of `m`'s problem.
    ///
    /// # Errors
    ///
    /// If either problem has one-sided candidates, which the cut does not treat.
    pub fn new(n: &Prepared<W>, m: &Prepared<W>, bound: u64) -> Result<Self, String> {
        Self::of(&n.model, &n.candidates, Arc::clone(&m.model), m.root, &m.candidates, bound)
    }

    /// [`new`](Self::new) from the parts of the two problems: Π_N's model and candidates, Π_M's model, root and
    /// candidates, and `bound`, a proven bound of P\*(Π_M).
    ///
    /// # Errors
    ///
    /// As [`new`](Self::new)'s.
    pub fn of(
        n_model: &FrameGraph,
        n_candidates: &Bits<W>,
        m_model: Arc<FrameGraph>,
        m_root: FrameSet<W>,
        m_candidates: &Bits<W>,
        bound: u64,
    ) -> Result<Self, String> {
        if n_model.has_one_sided() || m_model.has_one_sided() {
            return Err("the near-repeat cut does not treat the one-sided candidates of the extended problem".into());
        }
        let marked = *n_candidates & *m_candidates;
        let same_home: Bits<W> = marked.iter().filter(|&x| n_model.home(x) == m_model.home(x)).collect();
        let mut by_home = [Bits::EMPTY; MAX_FRAMES];
        for x in same_home.iter() {
            by_home[n_model.home(x)].insert(x);
        }
        Ok(NearRepeat { model: m_model, root: m_root, marked, same_home, by_home, bound, cut: AtomicU64::new(0) })
    }

    /// B: the proven bound of P\*(Π_M) that the incumbent must reach.
    pub fn bound(&self) -> u64 {
        self.bound
    }

    /// Whether a search whose incumbent or floor is worth `incumbent` may cut by this: the incumbent is worth at least
    /// B (the setup's third check).  Until it is, the search cuts nothing.
    #[inline]
    pub fn reached(&self, incumbent: u64) -> bool {
        incumbent >= self.bound
    }

    /// The nodes cut by this setup so far, over every searcher that has flushed its counters.
    pub fn nodes_cut(&self) -> u64 {
        self.cut.load(Ordering::Relaxed)
    }

    /// Adds `count` to [`nodes_cut`](Self::nodes_cut).
    pub(super) fn add_cut(&self, count: u64) {
        if count > 0 {
            self.cut.fetch_add(count, Ordering::Relaxed);
        }
    }

    /// The first half of check 1: every member of `promoted` is marked, a candidate of Π_M.  A superset of a set that
    /// fails it fails it too.
    #[inline]
    pub fn marked(&self, promoted: &Bits<W>) -> bool {
        promoted.is_subset(&self.marked)
    }

    /// The second half of check 1: every member of `cands` is a candidate of Π_M with the same home in both problems.
    #[inline]
    pub fn candidates_marked(&self, cands: &Bits<W>) -> bool {
        cands.is_subset(&self.same_home)
    }

    /// Check 2: σ_M(P), the state that `promoted`, promoted from the root of Π_M, gives, if `promoted` is a promotion
    /// set of Π_M (`None` if not).  The members must be marked (check 1), so that the set is one of candidates of Π_M.
    pub fn state_of(&self, scratch: &mut Scratch<W>, promoted: &Bits<W>) -> Option<FrameSet<W>> {
        debug_assert!(self.marked(promoted));
        self.model.promote_all_using(scratch, &self.root, promoted)
    }

    /// Check 2 from a parent: `added`, marked, promoted in Π_M from `from`, the state σ_M of the parent's promotions,
    /// into `into`; whether it can be.  The state is σ_M of the union, as [`state_of`](Self::state_of) gives it.  If it
    /// cannot be, `into` holds nothing of use.
    pub fn promote(
        &self,
        scratch: &mut Scratch<W>,
        from: &FrameSet<W>,
        added: &Bits<W>,
        into: &mut FrameSet<W>,
    ) -> bool {
        debug_assert!(self.marked(added));
        if added.len() == 1 {
            into.assign(from);
            return self.model.promote_in_place_using(scratch, into, added.min().expect("one member"));
        }
        match self.model.promote_all_using(scratch, from, added) {
            Some(state) => {
                into.assign(&state);
                true
            }
            None => false,
        }
    }

    /// Check 3: whether, on every frame j that a member of `cands` touches (its home h, and h − 1), `state_m`'s frame
    /// j, σ_M(P)'s, has no move that `state_n`'s, σ_N(P)'s, lacks, and every factor y of `state_n`'s frame j that
    /// `state_m`'s lacks is no member of `cands`, and no number of `graph`'s `composites(y)`, Π_N's factor graph, is a
    /// move of `state_m`'s frame j or a member of `cands`.  The members of `cands` must be marked as having the same
    /// home (check 1).
    ///
    /// The list `composites(y)` holds y's maximal composites among the moves of frame j and the factors of frame j + 1
    /// as setup forms them, so it holds every move of σ_N(P)'s frame j and every member of C of home j + 1 that y pays
    /// for, and no other member of C: the check is the lemma's condition 3, that y is a maximal factor of no move of
    /// σ_M(P)'s frame j and of no member of C of home j + 1, given that the moves of σ_M(P)'s frame j are moves of
    /// σ_N(P)'s.
    pub fn frames_agree(
        &self,
        graph: &FactorProvider,
        state_n: &FrameSet<W>,
        state_m: &FrameSet<W>,
        cands: &Bits<W>,
    ) -> bool {
        debug_assert!(self.candidates_marked(cands));
        let (frames_n, frames_m) = (state_n.frames(), state_m.frames());
        let mut touched = 0u32;
        for (h, homes) in self.by_home.iter().enumerate().skip(1) {
            if homes.intersects(cands) {
                touched |= 0b11 << (h - 1);
            }
        }
        while touched != 0 {
            let j = touched.trailing_zeros() as usize;
            touched &= touched - 1;
            // Every member of C has its home in both problems' frames, so this holds; a frame missing from either is
            // no frame the lemma compares, and the node is not cut.
            let (Some(a), Some(b)) = (frames_m.get(j), frames_n.get(j)) else { return false };
            if !a.moves.is_subset(&b.moves) {
                return false;
            }
            for y in (b.factors - a.factors).iter() {
                if cands.contains(y) {
                    return false;
                }
                let pays = |&z: &u16| {
                    let z = u32::from(z);
                    a.moves.contains(z) || cands.contains(z)
                };
                if graph.composites(y).iter().any(pays) {
                    return false;
                }
            }
        }
        true
    }

    /// The three checks at a node of the search of Π_N, whose model is `n_model`, with the promotions `promoted`, the
    /// candidates `cands` and the state `state_n`, σ_M(P) found from the root of Π_M: whether the node may be cut
    /// once the incumbent reaches B.  The searcher makes the same checks, with σ_M(P) kept along its path.
    pub fn cuts(
        &self,
        scratch: &mut Scratch<W>,
        n_model: &FrameGraph,
        state_n: &FrameSet<W>,
        promoted: &Bits<W>,
        cands: &Bits<W>,
    ) -> bool {
        if !self.marked(promoted) || !self.candidates_marked(cands) {
            return false;
        }
        match self.state_of(scratch, promoted) {
            Some(state_m) => self.frames_agree(n_model.graph(), state_n, &state_m, cands),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests;
