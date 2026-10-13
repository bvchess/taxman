//! The tactics that act inside the search ([`SearchTactics`]), and how a checkpoint records them.
//!
//! `solver::Tactics` holds every tactic of the solver by name; these are the ones a search and its checkpoints
//! depend on.  Each is graded in `DESIGN.md` (Tactics); the field docs say what each does and why it is sound.

use crate::bounds::RunnerUp;
use std::str::FromStr;

/// The tactics of DESIGN.md that act inside the search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchTactics {
    /// Rule candidates out of children with the depth-2 preclusion table (sound: a promotion that fails keeps
    /// failing after more promotions; see [`crate::bounds::Preclusions`]).
    pub preclusions: bool,
    /// Use the Java's runner-up in the `blameGame` bound (unsound; reproduces the Java; only without
    /// `match_bound`).
    pub legacy_bound: bool,
    /// Lower frame 0's freedoms after solving the two-frame game of frames 0 and 1 (unproven: "the max-sum
    /// promotion set also has the max count"), which can lower `max_total`.
    pub revise_freedoms: bool,
    /// Bound nodes by the matching relaxation ([`crate::matchbound`]) instead of `blameGame` (sound; `blameGame`
    /// is then not computed at all: it never pruned where the matching bound did not), and leave the Java's
    /// promotability checks out of the children's candidate lists (conflicts find those candidates).  The other
    /// tactics that look at the heaviest matching (`branching` other than `largest`, `match_refine`,
    /// `fix_forced`, `nogoods`) do nothing without it.
    pub match_bound: bool,
    /// Refine the matching bound by the branches of its conflict, this many levels deep, where it does not
    /// prune (0: off; sound, see `search::refine::Refiner`).  At most [`Self::MAX_MATCH_REFINE`].
    /// What it prunes can depend on the conflicts already known along the path, which work units do not
    /// record, so a single-threaded search that is interrupted may explore a slightly different tree (with the
    /// same answer).
    pub match_refine: u8,
    /// Which candidate a node branches on next (sound: any order explores every promotion set).
    pub branching: Branching,
    /// With `branching = conflict-strong`: a node's child promotes all of the node's *forced members*, the
    /// members of the heaviest matching whose removal alone brings the matching bound below what the node
    /// needs, and the node's loop ends after that child (sound: every completion that can beat the incumbent
    /// promotes them all; see `search::refine::strongest`).
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
}

/// Which candidate a node's loop promotes next (tactic `branching`).  The child promotes it, and the node goes on
/// without it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Branching {
    /// The largest candidate left (the Java's order).
    Largest,
    /// The smallest member of a conflict of the heaviest matching (see `search::refine::Refiner`), so
    /// that neither the child nor the node going on can repeat that matching; the largest candidate when there
    /// is none (or without `match_bound`).
    ConflictSmallest,
    /// As `ConflictSmallest`, the largest member of the conflict.  The child then follows the rounding of the
    /// dive (the smaller members are the ones left out), which finds good incumbents early.
    ConflictLargest,
    /// The member of the conflict that looks best one step ahead (see
    /// `search::refine::strongest`), or with `fix_forced` the forced members all at once.
    #[default]
    ConflictStrong,
}

impl Branching {
    /// Every variant.
    pub const ALL: [Branching; 4] =
        [Branching::Largest, Branching::ConflictSmallest, Branching::ConflictLargest, Branching::ConflictStrong];

    /// The name the command line and the results use (`largest`, `conflict-smallest`, ...); [`FromStr`] reads it.
    pub fn name(self) -> &'static str {
        match self {
            Branching::Largest => "largest",
            Branching::ConflictSmallest => "conflict-smallest",
            Branching::ConflictLargest => "conflict-largest",
            Branching::ConflictStrong => "conflict-strong",
        }
    }
}

impl FromStr for Branching {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        Branching::ALL.into_iter().find(|branching| branching.name() == text).ok_or_else(|| {
            let names: Vec<&str> = Branching::ALL.iter().map(|branching| branching.name()).collect();
            format!("expected one of {}, got {text:?}", names.join(", "))
        })
    }
}

impl Default for SearchTactics {
    fn default() -> Self {
        SearchTactics {
            preclusions: false,
            legacy_bound: false,
            revise_freedoms: true,
            match_bound: true,
            match_refine: 1,
            branching: Branching::default(),
            fix_forced: true,
            nogoods: true,
            forget_nogoods: true,
        }
    }
}

impl SearchTactics {
    /// The largest `match_refine`: the header of a checkpoint has room for no more, so a deeper setting could not
    /// be told from this one on resuming.
    pub const MAX_MATCH_REFINE: u8 = 7;

    /// How the `blameGame` bound replaces the first member of its bid that fails (tactic `legacy_bound`).
    pub(super) fn rule(self) -> RunnerUp {
        if self.legacy_bound {
            RunnerUp::Legacy
        } else {
            RunnerUp::Sound
        }
    }

    /// The tactics as the header of a checkpoint records them.  Tactics added later are recorded by the bits of
    /// their non-default values (`fix_forced` by bit 9, `nogoods` by bit 10 and `forget_nogoods` by bit 11 when
    /// they are off), so that a checkpoint made before they existed reads back with their defaults: they are
    /// sound, so a search resumed with them is still correct.
    pub(super) fn bits(self) -> u16 {
        let branching = match self.branching {
            Branching::Largest => 0,
            Branching::ConflictSmallest => 1,
            Branching::ConflictLargest => 2,
            Branching::ConflictStrong => 3,
        };
        u16::from(self.preclusions)
            | u16::from(self.legacy_bound) << 1
            | u16::from(self.revise_freedoms) << 2
            | u16::from(self.match_bound) << 3
            | u16::from(self.match_refine.min(Self::MAX_MATCH_REFINE)) << 4
            | branching << 7
            | u16::from(!self.fix_forced) << 9
            | u16::from(!self.nogoods) << 10
            | u16::from(!self.forget_nogoods) << 11
    }

    /// The tactics [`bits`](Self::bits) recorded.
    pub(super) fn from_bits(bits: u16) -> Result<Self, String> {
        if bits >> 12 != 0 {
            return Err(format!("unknown tactics {bits:#x}"));
        }
        let branching = match bits >> 7 & 3 {
            0 => Branching::Largest,
            1 => Branching::ConflictSmallest,
            2 => Branching::ConflictLargest,
            _ => Branching::ConflictStrong,
        };
        Ok(SearchTactics {
            preclusions: bits & 1 != 0,
            legacy_bound: bits & 2 != 0,
            revise_freedoms: bits & 4 != 0,
            match_bound: bits & 8 != 0,
            match_refine: (bits >> 4 & 7) as u8,
            branching,
            fix_forced: bits & 1 << 9 == 0,
            nogoods: bits & 1 << 10 == 0,
            forget_nogoods: bits & 1 << 11 == 0,
        })
    }
}
