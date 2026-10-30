//! The tactics that act inside the search ([`SearchTactics`]), and how a checkpoint records them.
//!
//! [`crate::tactics::Tactics`] holds every tactic of the solver by name; these are the ones a search and its
//! checkpoints depend on.  Each is graded in `solver/docs/design/tactics.md` (Tactics); the field docs say what each
//! does and why it is sound.

use super::NOGOOD_PERIOD;

/// The tactics of solver/docs/design/tactics.md, that act inside the search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchTactics {
    /// A node's child promotes all of the node's *forced members*, the members of the GF(2) bound's heaviest set
    /// without which the bound falls below what the node needs, and the node's loop ends after that child (sound:
    /// every completion that can beat the incumbent promotes them all, as the frame bound (B2,
    /// docs/theory/frame-bounds.md) of the node without each says; see [`crate::gf2`] and `search::searcher`'s
    /// `choose_branch`).
    pub fix_forced: bool,
    /// Learn the conflicts found as nogoods of the root ([`crate::model::Nogoods`]) and use them at every node:
    /// for the conflict to branch on, and to rule candidates out of children (sound: a nogood holds everywhere).
    /// What is learned depends on the order the search went, so with it a search's tree depends on its history
    /// (threads, interruptions), not only on the node.
    pub nogoods: bool,
    /// With `nogoods`: each searcher forgets all the nogoods it knows after each [`NOGOOD_PERIOD`] of its steps
    /// (sound: the table is a cache).  Old nogoods cost every lookup more than they help.
    pub forget_nogoods: bool,
    /// Cut a node none of whose completions keeps the inequality of every symmetric pair of the root, and drop from a
    /// node's candidates those that no such completion holds ([`crate::search::symmetry`]; proven:
    /// the prime-exchange theorem (R2) and the exchange-pruning lemma (R3) of docs/theory/search-rules.md: every
    /// heaviest promotion set keeps the inequalities, so a search for one needs only the completions that do).  The
    /// rule is a function of the node's promotions and candidates alone, so the tree depends on nothing but the node.
    pub symmetry: bool,
    /// At a check whose gap is at least [`CONFLICT_BRANCHING_GAP`](super::CONFLICT_BRANCHING_GAP), branch
    /// on the conflict of the heaviest set whose members cost the bound most to leave out, of the one the search found,
    /// the nogoods within the set and each failing frame's minimal conflict made by leaving out its cheapest members
    /// first (`search::searcher`'s `conflict_to_branch_on`; sound: every completion leaves out a member of any
    /// conflict, so the branches cover them all, and the conflicts made there are conflicts by the peel theorem
    /// (P4) and the residual theorem (P5) of docs/theory/playability.md, and the exactness theorem (X7) of
    /// docs/theory/exactness.md).
    pub conflict_branching: bool,
    /// With `conflict_branching`, where it would branch on a conflict other than the search's own, look first at the
    /// last child of the search's own conflict, the one that promotes every member but the cheapest and leaves the
    /// cheapest out, and branch on the search's own conflict if that child is cut on sight
    /// ([`CONFLICT_LOOK_PRICE`](super::CONFLICT_LOOK_PRICE), [`CONFLICT_LOOK_SLACK`](super::CONFLICT_LOOK_SLACK) and
    /// `search::searcher`'s `conflict_to_branch_on` say where; sound: it only chooses between two true conflicts, and
    /// leaves the node's state as it found it).  On by default.
    pub conflict_look: bool,
    /// Cut a node whose completions are promotion sets of a game M proven before, once the incumbent reaches a proven
    /// bound of M's best promotion sum ([`NearRepeat`](super::NearRepeat); proven: the near-repeat lemma (R7),
    /// docs/theory/search-rules.md).  It acts only where the search is given M
    /// ([`search_steered`](super::search_steered): the solver gives the game of n − 1 when it is concluded and proven
    /// before the search starts) and its incumbent or floor reaches that bound.
    pub near_repeat: bool,
    /// With `conflict_branching`, keep the prices of a node's members that the choice of the conflict makes, from one
    /// check of the node to the next and from a node to its children, and make only the prices a check lacks, all of
    /// them again after a few checks or when the node's bound has fallen (`search::price_memo`; sound: the prices only
    /// rank the conflicts, each a true conflict of the node, so any ranking keeps the search exact).  On by default.
    pub price_reuse: bool,
    /// Use the nogoods carried into the search from another search, each validated as a nogood of this root when it is
    /// loaded ([`CarriedNogoods`](crate::model::CarriedNogoods)), only to prune: to rule candidates out of a unit's node
    /// and of children, and to end a node whose forced members complete one, never to choose the conflict a node
    /// branches on (sound: a validated nogood holds at every node, as the searcher's own do).  It acts only where the
    /// search is given a store ([`SearchSpace::carried`](super::SearchSpace::carried); `solve --carried-dir DIR`).
    /// Off by default.
    pub carried_nogoods: bool,
    /// A child whose parent's heaviest set holds the members it promotes but also members it dropped finds
    /// its heaviest set from its parent's, deleting those members one at a time along the shortest cycle through each
    /// in an exchange graph that takes in every size, instead of solving from a smaller set (`crate::gf2`'s
    /// `warm_child`; sound: the bound is the same heaviest weight, found exactly, and only which heaviest set is found
    /// where several weigh the same can differ, which changes the conflicts branched on, not the answer).  On by
    /// default.
    pub warm_start: bool,
}

impl Default for SearchTactics {
    fn default() -> Self {
        SearchTactics {
            fix_forced: true,
            nogoods: true,
            forget_nogoods: true,
            symmetry: true,
            conflict_branching: true,
            conflict_look: true,
            near_repeat: true,
            price_reuse: true,
            carried_nogoods: false,
            warm_start: true,
        }
    }
}

impl SearchTactics {
    /// After how many of its steps a searcher forgets its nogoods, if ever: [`NOGOOD_PERIOD`] with `forget_nogoods`.
    pub fn forget_period(&self) -> Option<u64> {
        self.forget_nogoods.then_some(NOGOOD_PERIOD)
    }

    /// The bits of [`bits`](Self::bits) that only choose how a node branches: `conflict_branching` (bit 4) and
    /// `conflict_look` (bit 5).  A checkpoint records them, as the search began with them, but they do not bind its
    /// resumption: the children of any true conflict cover every completion of their node, so a search whose branching
    /// changes from one unit to the next is as exact as one that keeps it (solver/docs/design/search.md, "The invariant
    /// of a running search").  So a checkpoint resumes under either setting, and a run may change the gate while it goes
    /// ([`Branching`](super::Branching)).
    pub const BRANCHING_BITS: u16 = 1 << 4 | 1 << 5;

    /// The bit of [`bits`](Self::bits) of `near_repeat` (bit 6).  A checkpoint records it, as the search began with
    /// it, but it binds nothing: the cut drops only nodes none of whose completions beats the incumbent, by a bound
    /// that each run of the search sets up for itself, as it takes its ceiling, so a search whose units are searched
    /// with the cut and without it is as exact as one that keeps it (the near-repeat lemma (R7),
    /// docs/theory/search-rules.md, the paragraph "What it rests on").  And the bit cannot say whether the cut acted:
    /// that depends on whether the game before was proven when the run set the search up.  So a checkpoint resumes with
    /// the tactic on or off.
    pub const NEAR_REPEAT_BIT: u16 = 1 << 6;

    /// The bit of [`bits`](Self::bits) of `price_reuse` (bit 7).  A checkpoint records it, as the search began with it,
    /// but it binds nothing: the prices only rank true conflicts, so a search whose units are searched with the reuse and
    /// without it is as exact as one that keeps it.  So a checkpoint resumes with the tactic on or off.
    pub const PRICE_REUSE_BIT: u16 = 1 << 7;

    /// The bit of [`bits`](Self::bits) of `carried_nogoods` (bit 8).  A checkpoint records it, as the search began
    /// with it, but it binds nothing: a carried nogood is a fact of the root, validated there, so a search whose units
    /// are searched with the store and without it is as exact as one that keeps it.  So a checkpoint resumes with the
    /// tactic on or off.
    pub const CARRIED_NOGOODS_BIT: u16 = 1 << 8;

    /// The bit of [`bits`](Self::bits) of `warm_start` (bit 9).  A checkpoint records it, as the search began with it,
    /// but it binds nothing: the bound of every node is the same with it and without it, and only the heaviest set
    /// found among several of the same weight, whose conflicts are branched on, can differ.  So a checkpoint resumes
    /// with the tactic on or off.
    pub const WARM_START_BIT: u16 = 1 << 9;

    /// The bits of [`bits`](Self::bits) that a checkpoint records and that bind nothing:
    /// [`BRANCHING_BITS`](Self::BRANCHING_BITS), [`NEAR_REPEAT_BIT`](Self::NEAR_REPEAT_BIT),
    /// [`PRICE_REUSE_BIT`](Self::PRICE_REUSE_BIT), [`CARRIED_NOGOODS_BIT`](Self::CARRIED_NOGOODS_BIT) and
    /// [`WARM_START_BIT`](Self::WARM_START_BIT).
    const UNBOUND_BITS: u16 = Self::BRANCHING_BITS
        | Self::NEAR_REPEAT_BIT
        | Self::PRICE_REUSE_BIT
        | Self::CARRIED_NOGOODS_BIT
        | Self::WARM_START_BIT;

    /// Whether `other` is the same search as these tactics make, as a checkpoint tells ([`bits`](Self::bits)): the
    /// bits it records but that only choose how a node branches ([`BRANCHING_BITS`](Self::BRANCHING_BITS)), the
    /// near-repeat cut ([`NEAR_REPEAT_BIT`](Self::NEAR_REPEAT_BIT)), the reuse of prices
    /// ([`PRICE_REUSE_BIT`](Self::PRICE_REUSE_BIT)), the carried nogoods
    /// ([`CARRIED_NOGOODS_BIT`](Self::CARRIED_NOGOODS_BIT)) and the warm start of the GF(2) bound
    /// ([`WARM_START_BIT`](Self::WARM_START_BIT)) may differ.
    pub fn same_recorded(&self, other: &SearchTactics) -> bool {
        self.bits() & !Self::UNBOUND_BITS == other.bits() & !Self::UNBOUND_BITS
    }

    /// These tactics with what of `run`'s a checkpoint does not bind ([`same_recorded`](Self::same_recorded)): its
    /// branching switches `conflict_branching` and `conflict_look`, `near_repeat`, `price_reuse`, `carried_nogoods` and
    /// `warm_start`.  The run that resumes a search says them.
    #[must_use]
    pub fn with_unbound_of(self, run: &SearchTactics) -> SearchTactics {
        SearchTactics {
            conflict_branching: run.conflict_branching,
            conflict_look: run.conflict_look,
            near_repeat: run.near_repeat,
            price_reuse: run.price_reuse,
            carried_nogoods: run.carried_nogoods,
            warm_start: run.warm_start,
            ..self
        }
    }

    /// The tactics of *extended mode*, a search of a problem with one-sided candidates (the extended problem's,
    /// `crate::frames::build_extended`): the default without the rule of symmetric pairs, which such a search refuses
    /// ([`refusal_with_one_sided`](Self::refusal_with_one_sided)).
    pub fn extended() -> Self {
        SearchTactics { symmetry: false, ..SearchTactics::default() }
    }

    /// Why these tactics cannot search a problem with one-sided candidates, if they cannot: such a search refuses the
    /// rule of symmetric pairs (`symmetry`: the prime-exchange theorem (R2, docs/theory/search-rules.md) is proven for
    /// the frame model, and for the extended problem would need each prime exchange to map the one-sided candidates of
    /// each frame onto themselves, which nothing checks yet).
    pub fn refusal_with_one_sided(&self) -> Option<String> {
        self.symmetry.then(|| {
            "a search with one-sided candidates (extended mode) cannot run with symmetry=on (the prime-exchange \
             theorem, R2, is not proven for them)"
                .to_owned()
        })
    }

    /// The tactics as the header of a checkpoint records them: one bit for each switch that is on, `fix_forced` (bit
    /// 0), `nogoods` (1), `forget_nogoods` (2), `symmetry` (3), `conflict_branching` (4), `conflict_look` (5),
    /// `near_repeat` (6), `price_reuse` (7), `carried_nogoods` (8) and `warm_start` (9); the defaults are `0x2FF`.
    /// Bits 4 to 9 record the settings the search began with and bind nothing ([`same_recorded`](Self::same_recorded)).
    pub(super) fn bits(self) -> u16 {
        u16::from(self.fix_forced)
            | u16::from(self.nogoods) << 1
            | u16::from(self.forget_nogoods) << 2
            | u16::from(self.symmetry) << 3
            | u16::from(self.conflict_branching) << 4
            | u16::from(self.conflict_look) << 5
            | u16::from(self.near_repeat) << 6
            | u16::from(self.price_reuse) << 7
            | u16::from(self.carried_nogoods) << 8
            | u16::from(self.warm_start) << 9
    }

    /// The tactics [`bits`](Self::bits) recorded, or an error if a bit that no tactic uses is set.
    pub(super) fn from_bits(bits: u16) -> Result<Self, String> {
        if bits >> 10 != 0 {
            return Err(format!("unknown tactics {bits:#x}"));
        }
        Ok(SearchTactics {
            fix_forced: bits & 1 != 0,
            nogoods: bits & 1 << 1 != 0,
            forget_nogoods: bits & 1 << 2 != 0,
            symmetry: bits & 1 << 3 != 0,
            conflict_branching: bits & 1 << 4 != 0,
            conflict_look: bits & 1 << 5 != 0,
            near_repeat: bits & 1 << 6 != 0,
            price_reuse: bits & 1 << 7 != 0,
            carried_nogoods: bits & 1 << 8 != 0,
            warm_start: bits & 1 << 9 != 0,
        })
    }
}
