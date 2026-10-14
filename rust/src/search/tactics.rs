//! The tactics that act inside the search ([`SearchTactics`]), and how a checkpoint records them.
//!
//! [`crate::tactics::Tactics`] holds every tactic of the solver by name; these are the ones a search and its
//! checkpoints depend on.  Each is graded in `DESIGN.md`, section 5 (Tactics); the field docs say what each does
//! and why it is sound.

/// The tactics of DESIGN.md, section 5, that act inside the search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchTactics {
    /// Lower frame 0's freedoms after solving the two-frame game of frames 0 and 1 (unproven: "the max-sum
    /// promotion set also has the max count"), which can lower `max_total`.
    pub revise_freedoms: bool,
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
}

impl Default for SearchTactics {
    fn default() -> Self {
        SearchTactics {
            revise_freedoms: true,
            match_refine: 1,
            fix_forced: true,
            nogoods: true,
            forget_nogoods: true,
            lagrange: true,
        }
    }
}

impl SearchTactics {
    /// The largest `match_refine`: the header of a checkpoint has room for no more, so a deeper setting could not
    /// be told from this one on resuming.
    pub const MAX_MATCH_REFINE: u8 = 7;

    /// The bits of a checkpoint's header that earlier builds used for tactics this one no longer has: bit 0
    /// `preclusions`, bit 1 `legacy_bound`, bit 3 `match_bound`, bits 7 and 8 `branching`.
    const REMOVED_BITS: u16 = 0b11 | 1 << 3 | 3 << 7;

    /// What those bits hold when a search runs as every search now does: `match_bound` on (bit 3), `branching` as
    /// `conflict-strong` (3 in bits 7 and 8), `preclusions` and `legacy_bound` off (bits 0 and 1 clear).
    const REMOVED_BITS_AS_RUN: u16 = 1 << 3 | 3 << 7;

    /// The tactics as the header of a checkpoint records them.  Tactics added later are recorded by the bits of
    /// their non-default values (`fix_forced` by bit 9, `nogoods` by bit 10, `forget_nogoods` by bit 11 and
    /// `lagrange` by bit 12 when they are off), so that a checkpoint made before they existed reads back with their
    /// defaults: they are sound, so a search resumed with them is still correct.  The bits of the tactics this
    /// build no longer has are written as the search now runs.
    pub(super) fn bits(self) -> u16 {
        Self::REMOVED_BITS_AS_RUN
            | u16::from(self.revise_freedoms) << 2
            | u16::from(self.match_refine.min(Self::MAX_MATCH_REFINE)) << 4
            | u16::from(!self.fix_forced) << 9
            | u16::from(!self.nogoods) << 10
            | u16::from(!self.forget_nogoods) << 11
            | u16::from(!self.lagrange) << 12
    }

    /// The tactics [`bits`](Self::bits) recorded, or an error if the checkpoint was made with a tactic that this
    /// build no longer has (`preclusions`, `legacy_bound`, `match_bound` off, or a `branching` other than
    /// `conflict-strong`), or with one it does not know.
    pub(super) fn from_bits(bits: u16) -> Result<Self, String> {
        if bits >> 13 != 0 {
            return Err(format!("unknown tactics {bits:#x}"));
        }
        if bits & Self::REMOVED_BITS != Self::REMOVED_BITS_AS_RUN {
            return Err(format!(
                "the checkpoint was made with a search tactic that this build no longer has ({}); finish it with the \
                 build that made it",
                Self::removed_settings(bits).join(", ")
            ));
        }
        Ok(SearchTactics {
            revise_freedoms: bits & 4 != 0,
            match_refine: (bits >> 4 & 7) as u8,
            fix_forced: bits & 1 << 9 == 0,
            nogoods: bits & 1 << 10 == 0,
            forget_nogoods: bits & 1 << 11 == 0,
            lagrange: bits & 1 << 12 == 0,
        })
    }

    /// The settings of the removed tactics that `bits` records and the search no longer runs, spelled as
    /// `--tactic` once spelled them.
    fn removed_settings(bits: u16) -> Vec<String> {
        let mut settings = Vec::new();
        if bits & 1 != 0 {
            settings.push("preclusions=on".to_owned());
        }
        if bits & 2 != 0 {
            settings.push("legacy_bound=on".to_owned());
        }
        if bits & 1 << 3 == 0 {
            settings.push("match_bound=off".to_owned());
        }
        match bits >> 7 & 3 {
            0 => settings.push("branching=largest".to_owned()),
            1 => settings.push("branching=conflict-smallest".to_owned()),
            2 => settings.push("branching=conflict-largest".to_owned()),
            _ => {}
        }
        settings
    }
}
