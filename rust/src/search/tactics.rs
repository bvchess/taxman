//! The tactics that act inside the search ([`SearchTactics`]), and how a checkpoint records them.
//!
//! [`crate::tactics::Tactics`] holds every tactic of the solver by name; these are the ones a search and its
//! checkpoints depend on.  Each is graded in `DESIGN.md`, section 5 (Tactics); the field docs say what each does
//! and why it is sound.

/// The tactics of DESIGN.md, section 5, that act inside the search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchTactics {
    /// Refine the matching bound by the branches of its conflict, this many levels deep, where it does not
    /// prune (0: off; sound, see `search::refine::Refiner`).  At most [`Self::MAX_MATCH_REFINE`].
    /// What it prunes can depend on the conflicts already known along the path, which work units do not
    /// record, so a single-threaded search that is interrupted may explore a slightly different tree (with the
    /// same answer).
    pub match_refine: u8,
    /// A node's child promotes all of the node's *forced members*, the members of the heaviest matching whose
    /// removal alone brings the matching bound below what the node needs, and the node's loop ends after that
    /// child (sound: every completion that can beat the incumbent promotes them all; see
    /// `search::refine::strongest`).
    pub fix_forced: bool,
    /// Learn the conflicts found as nogoods of the root ([`crate::model::Nogoods`]) and use them at every node:
    /// for the conflict to branch on and to refine with, and to rule candidates out of children (sound: a
    /// nogood holds everywhere).  What is learned depends on the order the search went, so with it a search's
    /// tree depends on its history (threads, interruptions), not only on the node.
    pub nogoods: bool,
    /// With `nogoods`: each searcher forgets every nogood it knows after each
    /// [`NOGOOD_PERIOD`](super::NOGOOD_PERIOD) of its steps (sound: the table is a cache).  Old nogoods cost
    /// every lookup more than they help.
    pub forget_nogoods: bool,
    /// With `nogoods`: where the matching bound and its refinement leave a node or a child open, bound it also by a
    /// Lagrangian relaxation of the nogoods the searcher has learned, kept incrementally along its path, and (with
    /// `fix_forced`) add to the forced members the members without which that bound would cut (proven: weak
    /// duality; see [`crate::matchbound::lagrange`] and `search::lagrange`).  The multipliers are refitted as the
    /// searcher learns, so the tree depends on its history, as with `nogoods`.
    pub lagrange: bool,
    /// Bound every node by the GF(2) bound ([`crate::gf2`]: the heaviest common independent set of the two GF(2)
    /// frame matroids, kept along the searcher's path by contraction), branch on the conflicts of its heaviest set,
    /// and (with `fix_forced`) force the members without which it would cut (proven: heuristics/THEORY.md, Theorem
    /// 11, and weak duality for the forced members).  Replaces the matching bound and its refinement as the node
    /// bound; the Lagrangian bound over nogoods still applies after it.
    pub gf2: bool,
    /// Cut a node none of whose completions keeps the inequality of every symmetric pair of the root, and drop from a
    /// node's candidates those that no such completion holds ([`crate::search::symmetry`]; proven:
    /// heuristics/THEORY.md, Theorem 13 and "Pruning with the inequalities": every heaviest promotion set keeps the
    /// inequalities, so a search for one needs only the completions that do).  It acts with either node bound.  The
    /// rule is a function of the node's promotions and candidates alone, so the tree depends on nothing but the node.
    pub symmetry: bool,
}

impl Default for SearchTactics {
    fn default() -> Self {
        SearchTactics {
            match_refine: 1,
            fix_forced: true,
            nogoods: true,
            forget_nogoods: true,
            lagrange: false,
            gf2: true,
            symmetry: false,
        }
    }
}

impl SearchTactics {
    /// The largest `match_refine`: the header of a checkpoint has room for no more, so a deeper setting could not
    /// be told from this one on resuming.
    pub const MAX_MATCH_REFINE: u8 = 7;

    /// The tactics of the matching path, the default before `gf2`: the matching bound with its refinement, then the
    /// Lagrangian bound over the nogoods.  Its trees are several times larger than the default's at a fraction of
    /// the cost a node, so the tests of interrupted searches use it on the small games: the engine checks a node
    /// limit only every 1,024 steps of a searcher, which the default's searches of those games do not reach.
    pub fn matching() -> Self {
        SearchTactics { gf2: false, lagrange: true, ..SearchTactics::default() }
    }

    /// The tactics as the header of a checkpoint records them: `match_refine` in bits 0 to 2, then one bit for each
    /// switch that is on, `fix_forced` (bit 3), `nogoods` (4), `forget_nogoods` (5), `lagrange` (6), `gf2` (7) and
    /// `symmetry` (8).  (A checkpoint of a build before `gf2` has bit 7 clear: it resumes with `gf2` off.  A build
    /// before `symmetry` refuses a checkpoint with the tactic on, as "unknown tactics": it would resume the search
    /// without a rule that shaped the tree it wrote.)
    pub(super) fn bits(self) -> u16 {
        u16::from(self.match_refine.min(Self::MAX_MATCH_REFINE))
            | u16::from(self.fix_forced) << 3
            | u16::from(self.nogoods) << 4
            | u16::from(self.forget_nogoods) << 5
            | u16::from(self.lagrange) << 6
            | u16::from(self.gf2) << 7
            | u16::from(self.symmetry) << 8
    }

    /// The tactics [`bits`](Self::bits) recorded, or an error if a bit that no tactic uses is set.
    pub(super) fn from_bits(bits: u16) -> Result<Self, String> {
        if bits >> 9 != 0 {
            return Err(format!("unknown tactics {bits:#x}"));
        }
        Ok(SearchTactics {
            match_refine: (bits & 7) as u8,
            fix_forced: bits & 1 << 3 != 0,
            nogoods: bits & 1 << 4 != 0,
            forget_nogoods: bits & 1 << 5 != 0,
            lagrange: bits & 1 << 6 != 0,
            gf2: bits & 1 << 7 != 0,
            symmetry: bits & 1 << 8 != 0,
        })
    }
}
