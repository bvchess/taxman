//! The tactics that act inside the search ([`SearchTactics`]), and how a checkpoint records them.
//!
//! [`crate::tactics::Tactics`] holds every tactic of the solver by name; these are the ones a search and its
//! checkpoints depend on.  Each is graded in `DESIGN.md`, section 5 (Tactics); the field docs say what each does
//! and why it is sound.

use super::NOGOOD_PERIOD;
use crate::model::Keep;

pub use crate::gf2::ForceWindow;
use crate::gf2::LEGACY_FORCE_WINDOW;

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
    /// With `nogoods`: each searcher forgets the nogoods it knows after each [`nogood_period`](Self::nogood_period) of
    /// its steps (sound: the table is a cache), all of them unless [`nogood_keep`](Self::nogood_keep) keeps some.  Old
    /// nogoods cost every lookup more than they help.
    pub forget_nogoods: bool,
    /// With `nogoods` and `forget_nogoods`: how many of a searcher's steps pass between its forgettings (tactic
    /// `nogood_period`).  A research knob, like the two that follow: any value is sound, and none is recorded in a
    /// checkpoint (a resumed search forgets as the run that resumes it says).
    pub nogood_period: u64,
    /// With `nogoods` and `forget_nogoods`: what a forgetting keeps (tactic `nogood_keep`; [`Keep`]).  The default
    /// keeps nothing.
    pub nogood_keep: Keep,
    /// With `nogoods`: the most nogoods a searcher's table holds (tactic `nogood_limit`), the least recently used
    /// dropped for a new one; 0: no limit.
    pub nogood_limit: u64,
    /// With `nogoods`: where the matching bound and its refinement leave a node or a child open, bound it also by a
    /// Lagrangian relaxation of the nogoods the searcher has learned, kept incrementally along its path, and (with
    /// `fix_forced`) add to the forced members the members without which that bound would cut (proven: weak
    /// duality; see [`crate::matchbound::lagrange`] and `search::lagrange`).  The multipliers are refitted as the
    /// searcher learns, so the tree depends on its history, as with `nogoods`.
    pub lagrange: bool,
    /// Bound every node by the GF(2) bound ([`crate::gf2`]: the heaviest common independent set of the two GF(2)
    /// frame matroids, kept along the searcher's path by contraction), branch on the conflicts of its heaviest set,
    /// and (with `fix_forced`) force the members without which it would cut (proven: docs/THEORY.md, Theorem
    /// 11, and weak duality for the forced members).  Replaces the matching bound and its refinement as the node
    /// bound; the Lagrangian bound over nogoods still applies after it.
    pub gf2: bool,
    /// Cut a node none of whose completions keeps the inequality of every symmetric pair of the root, and drop from a
    /// node's candidates those that no such completion holds ([`crate::search::symmetry`]; proven:
    /// docs/THEORY.md, Theorem 13 and "Pruning with the inequalities": every heaviest promotion set keeps the
    /// inequalities, so a search for one needs only the completions that do).  It acts with either node bound.  The
    /// rule is a function of the node's promotions and candidates alone, so the tree depends on nothing but the node.
    pub symmetry: bool,
    /// With `gf2` and `fix_forced`: which members of a node's heaviest set that the weight split leaves free are
    /// searched for being forced after all, by the exchange graph ([`ForceWindow`]; sound whatever it is: a member is
    /// forced only where the search shows that every completion without it falls short).
    pub force_window: ForceWindow,
    /// With `gf2`: how a node chooses the member of its conflict to branch on ([`Gf2Branching`]; sound whatever it
    /// is: every completion leaves out some member of the conflict, so the branches cover them all).
    pub branching: Gf2Branching,
    /// With `gf2`'s strong branching (`branching` exact or forced): the most members of a conflict whose bounds are
    /// found, the most promising by their split bounds first (`u32::MAX`: all; sound, as for `branching`).
    pub strong_limit: u32,
    /// With `gf2`: a node with at least this many promotions chooses its branch by `deep_branching` instead of
    /// `branching` (`u32::MAX`: none does; sound, as for `branching`).
    pub strong_depth: u32,
    pub deep_branching: Gf2Branching,
    /// With `gf2`'s strong branching and `fix_forced`: once a member is found forced, whether the bounds of the others
    /// are still found, to find more forced members (sound, as for `fix_forced`).
    pub more_forced: bool,
}

impl Default for SearchTactics {
    fn default() -> Self {
        SearchTactics {
            match_refine: 1,
            fix_forced: true,
            nogoods: true,
            forget_nogoods: true,
            nogood_period: NOGOOD_PERIOD,
            nogood_keep: Keep::Nothing,
            nogood_limit: 0,
            lagrange: false,
            gf2: true,
            symmetry: true,
            force_window: ForceWindow::default(),
            branching: Gf2Branching::Exact,
            strong_limit: u32::MAX,
            strong_depth: u32::MAX,
            deep_branching: Gf2Branching::Split,
            more_forced: true,
        }
    }
}

/// How a node chooses the member of its conflict to branch on with the tactic `gf2` (strong branching asks what each
/// member's removal does to the bound).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gf2Branching {
    /// The largest member.
    Largest,
    /// The member whose removal lowers the weight split's bound most (see [`crate::gf2::meet`]).
    Split,
    /// The member whose removal lowers the bound most, exactly: read off the heaviest set's exchange graph where it
    /// certifies the bound, solved (warm-started) where not.
    Exact,
    /// Each member solved only as far as `need` (to find the forced ones), the branch the member with the lowest
    /// weight-split bound.
    Forced,
}

impl Gf2Branching {
    /// Every choice, in the order of its code in a checkpoint's header.
    pub const ALL: [Gf2Branching; 4] =
        [Gf2Branching::Largest, Gf2Branching::Split, Gf2Branching::Exact, Gf2Branching::Forced];

    /// The name `--tactic` gives it.
    pub fn name(self) -> &'static str {
        match self {
            Gf2Branching::Largest => "largest",
            Gf2Branching::Split => "split",
            Gf2Branching::Exact => "exact",
            Gf2Branching::Forced => "forced",
        }
    }

    fn code(self) -> u8 {
        Self::ALL.iter().position(|&b| b == self).expect("a choice") as u8
    }

    fn from_code(code: u8) -> Result<Self, String> {
        Self::ALL.get(usize::from(code)).copied().ok_or_else(|| format!("unknown branching {code}"))
    }
}

/// How a searcher keeps and forgets its nogoods (the tactics `forget_nogoods`, `nogood_period`, `nogood_keep` and
/// `nogood_limit`).  Every setting is sound: the table is a cache of nogoods that hold at every node
/// ([`crate::model::Nogoods`]), so a policy changes how large the trees are, never the optimum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Forgetting {
    /// After how many of its steps a searcher forgets, if ever.
    pub period: Option<u64>,
    /// What a forgetting keeps.
    pub keep: Keep,
    /// The most nogoods a table holds, if limited.
    pub limit: Option<usize>,
}

impl Forgetting {
    /// Forget everything after each [`NOGOOD_PERIOD`] steps, with no limit: the default.
    pub const DEFAULT: Forgetting = Forgetting { period: Some(NOGOOD_PERIOD), keep: Keep::Nothing, limit: None };

    /// Forget everything after each `period` steps, with no limit.
    pub const fn every(period: u64) -> Forgetting {
        Forgetting { period: Some(period), ..Forgetting::DEFAULT }
    }
}

impl SearchTactics {
    /// How the searchers keep and forget their nogoods.
    pub fn forgetting(&self) -> Forgetting {
        Forgetting {
            period: self.forget_nogoods.then_some(self.nogood_period),
            keep: self.nogood_keep,
            limit: (self.nogood_limit > 0).then_some(self.nogood_limit as usize),
        }
    }

    /// Whether `other` is the same search as these tactics make, as a checkpoint tells ([`bits`](Self::bits) and
    /// [`tuning`](Self::tuning)): the tactics a checkpoint does not record, the policy of keeping and forgetting
    /// nogoods, may differ (a resumed search starts with no nogoods whichever policy made the first run's).
    pub fn same_recorded(&self, other: &SearchTactics) -> bool {
        self.bits() == other.bits() && self.tuning() == other.tuning()
    }

    /// These tactics with the policy of keeping and forgetting nogoods of `run`'s: a checkpoint does not record it,
    /// so the run that resumes the search says how its searchers keep their nogoods.
    #[must_use]
    pub fn with_forgetting_of(self, run: &SearchTactics) -> SearchTactics {
        SearchTactics {
            nogood_period: run.nogood_period,
            nogood_keep: run.nogood_keep,
            nogood_limit: run.nogood_limit,
            ..self
        }
    }

    /// The largest `match_refine`: the header of a checkpoint has room for no more, so a deeper setting could not
    /// be told from this one on resuming.
    pub const MAX_MATCH_REFINE: u8 = 7;

    /// The tactics of the matching path, the default before `gf2`: the matching bound with its refinement, then the
    /// Lagrangian bound over the nogoods, and without the rule of `symmetry`, as it was before that tactic (its trees
    /// stay those of the build before it).  Its trees are several times larger than the default's at a fraction of
    /// the cost a node, so the tests of interrupted searches use it on the small games: the engine checks a node
    /// limit only every 1,024 steps of a searcher, which the default's searches of those games do not reach.
    pub fn matching() -> Self {
        SearchTactics { gf2: false, lagrange: true, symmetry: false, ..SearchTactics::default() }
    }

    /// The tactics of *extended mode*, a search of a problem with one-sided candidates (the extended problem's,
    /// `crate::frames::build_extended`): the default without the rule of symmetric pairs, which such a search refuses
    /// ([`refusal_with_one_sided`](Self::refusal_with_one_sided)).
    pub fn extended() -> Self {
        SearchTactics { symmetry: false, ..SearchTactics::default() }
    }

    /// Why these tactics cannot search a problem with one-sided candidates, if they cannot: such a search needs the
    /// GF(2) bound (`gf2`; the matching bound, `gf2=off`, gives a one-sided candidate no vertex, so it is no bound
    /// there), and refuses the Lagrangian bound (`lagrange`, built on the matching bound) and the rule of symmetric
    /// pairs (`symmetry`: Theorem 13 is proven for the frame model, and for the extended problem would need each prime
    /// exchange to map the one-sided candidates of each frame onto themselves, which nothing checks yet).
    pub fn refusal_with_one_sided(&self) -> Option<String> {
        let mut why = Vec::new();
        if !self.gf2 {
            why.push("gf2=off (the matching bound does not know them)");
        }
        if self.lagrange {
            why.push("lagrange=on (the Lagrangian bound is the matching bound's)");
        }
        if self.symmetry {
            why.push("symmetry=on (Theorem 13 is not proven for them)");
        }
        (!why.is_empty())
            .then(|| format!("a search with one-sided candidates (extended mode) cannot run with {}", why.join(", ")))
    }

    /// The tactics as the header of a checkpoint records them: `match_refine` in bits 0 to 2, then one bit for each
    /// switch that is on, `fix_forced` (bit 3), `nogoods` (4), `forget_nogoods` (5), `lagrange` (6), `gf2` (7) and
    /// `symmetry` (8); the defaults are `0x1B9`.  The knobs of the nogood table's policy (`nogood_period`,
    /// `nogood_keep`, `nogood_limit`) are not recorded: they change the tree, never the answer, and a resumed search
    /// starts with an empty table whichever policy made the first run's.  (A checkpoint of a build before `gf2` has bit 7 clear: it resumes
    /// with `gf2` off.  One of a build before `symmetry` has bit 8 clear: it resumes with `symmetry` off, which the
    /// default no longer is, so a run resumes it only with `--tactic symmetry=off`.  A build before `symmetry`
    /// refuses a checkpoint with the tactic on, as "unknown tactics": it would resume the search without a rule that
    /// shaped the tree it wrote.)
    pub(super) fn bits(self) -> u16 {
        u16::from(self.match_refine.min(Self::MAX_MATCH_REFINE))
            | u16::from(self.fix_forced) << 3
            | u16::from(self.nogoods) << 4
            | u16::from(self.forget_nogoods) << 5
            | u16::from(self.lagrange) << 6
            | u16::from(self.gf2) << 7
            | u16::from(self.symmetry) << 8
    }

    /// The length of [`tuning`](Self::tuning).
    pub(super) const TUNING_LEN: usize = 16;

    /// The tactics that tune the GF(2) search (`force_window` to `more_forced`) as every search of the builds before
    /// them had them, which a checkpoint's header of version 5 (or 4) implies, with the other tactics at their
    /// defaults.  Pinned here, apart from the defaults, because the defaults move: `force_window` was `80` then and is
    /// `all` now.
    pub(super) fn legacy_tuning() -> Self {
        SearchTactics {
            force_window: ForceWindow::Within(LEGACY_FORCE_WINDOW),
            branching: Gf2Branching::Exact,
            strong_limit: u32::MAX,
            strong_depth: u32::MAX,
            deep_branching: Gf2Branching::Split,
            more_forced: true,
            ..SearchTactics::default()
        }
    }

    /// Whether the tactics that tune the GF(2) search (`force_window` to `more_forced`) are those of
    /// [`legacy_tuning`](Self::legacy_tuning), which a checkpoint's header of version 5 implies and writes.  (The
    /// policy of keeping and forgetting nogoods is not recorded, so it does not count.)
    pub(super) fn tuning_is_legacy(&self) -> bool {
        Self::legacy_tuning().with_switches_of(self) == *self
    }

    /// `self` with the switches and `match_refine` of `other`, and its policy of keeping and forgetting nogoods.
    fn with_switches_of(self, other: &Self) -> Self {
        SearchTactics {
            match_refine: other.match_refine,
            fix_forced: other.fix_forced,
            nogoods: other.nogoods,
            forget_nogoods: other.forget_nogoods,
            lagrange: other.lagrange,
            gf2: other.gf2,
            symmetry: other.symmetry,
            ..self.with_forgetting_of(other)
        }
    }

    /// The tactics that tune the GF(2) search as a header of version 6 records them after the switches' bits, little
    /// endian: `force_window` (its kind, 0 within, 1 share and 2 all, then its value, u32), `branching` and
    /// `deep_branching` (u8 each: largest, split, exact, forced), `more_forced` (u8), `strong_limit` and
    /// `strong_depth` (u32 each).
    pub(super) fn tuning(&self) -> [u8; Self::TUNING_LEN] {
        let (kind, value) = match self.force_window {
            ForceWindow::Within(window) => (0u8, window),
            ForceWindow::Share(percent) => (1, u32::from(percent)),
            ForceWindow::All => (2, 0),
        };
        let mut out = [0u8; Self::TUNING_LEN];
        out[0] = kind;
        out[1..5].copy_from_slice(&value.to_le_bytes());
        out[5] = self.branching.code();
        out[6] = self.deep_branching.code();
        out[7] = u8::from(self.more_forced);
        out[8..12].copy_from_slice(&self.strong_limit.to_le_bytes());
        out[12..16].copy_from_slice(&self.strong_depth.to_le_bytes());
        out
    }

    /// `self` with the tuning that [`tuning`](Self::tuning) recorded in `bytes`, or an error if it is not one.
    pub(super) fn with_tuning(self, bytes: &[u8]) -> Result<Self, String> {
        let u32_at = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().expect("4 bytes"));
        if bytes.len() != Self::TUNING_LEN || bytes[7] > 1 {
            return Err("unknown tuning of the GF(2) search".into());
        }
        let force_window = match (bytes[0], u32_at(1)) {
            (0, window) => ForceWindow::Within(window),
            (1, percent) => ForceWindow::Share(u8::try_from(percent).map_err(|_| "a share above 255%")?),
            (2, 0) => ForceWindow::All,
            _ => return Err("unknown force_window".into()),
        };
        Ok(SearchTactics {
            force_window,
            branching: Gf2Branching::from_code(bytes[5])?,
            deep_branching: Gf2Branching::from_code(bytes[6])?,
            more_forced: bytes[7] == 1,
            strong_limit: u32_at(8),
            strong_depth: u32_at(12),
            ..self
        })
    }

    /// The tactics [`bits`](Self::bits) recorded, with the tuning a header of version 5 implies
    /// ([`legacy_tuning`](Self::legacy_tuning)), or an error if a bit that no tactic uses is set.
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
            ..SearchTactics::legacy_tuning()
        })
    }
}
