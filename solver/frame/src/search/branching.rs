//! The branching settings of a search, whose gate the search's gate trial changes while the search runs ([`Branching`]).
//!
//! A setting here chooses only how a node branches: which conflict, of the true conflicts of its heaviest set, its
//! children leave a member of out, and from which gap a node chooses (the gate).  The children of any conflict cover
//! every completion of their node, whatever the other nodes chose, so a search whose settings change between its units
//! is as exact as one that keeps them (solver/docs/design/search.md, "The invariant of a running search", argues it).
//! Every searcher reads the settings when it starts a unit, through a generation number, and never during a unit's
//! steps: what a step reads is the searcher's own copy.

use super::tactics::SearchTactics;
use super::CONFLICT_BRANCHING_GAP;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

/// What the branching settings say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BranchSettings {
    /// Whether the tactic `conflict_branching` is on.
    pub conflict_branching: bool,
    /// The gap from which a check branches on the conflict that the tactic `conflict_branching` chooses
    /// ([`CONFLICT_BRANCHING_GAP`] unless a caller sets another).  It is kept while the tactic is off, and applies when
    /// the tactic is next turned on.
    pub gate: u64,
    /// Whether the tactic `conflict_look` is on: it acts only where `conflict_branching` is choosing.
    pub conflict_look: bool,
}

impl BranchSettings {
    /// The settings that `tactics` make: `conflict_branching` and `conflict_look` as they say, and the gate
    /// [`CONFLICT_BRANCHING_GAP`].
    pub fn of(tactics: &SearchTactics) -> Self {
        BranchSettings {
            conflict_branching: tactics.conflict_branching,
            gate: CONFLICT_BRANCHING_GAP,
            conflict_look: tactics.conflict_look,
        }
    }

    /// These settings with the gate `gate`, if there is one.
    pub fn with_gate(self, gate: Option<u64>) -> Self {
        BranchSettings { gate: gate.unwrap_or(self.gate), ..self }
    }

    /// The gap from which a check branches on the conflict that `conflict_branching` chooses, if the tactic is on;
    /// `None`: the search's own conflict everywhere.
    pub fn conflict_gap(&self) -> Option<u64> {
        self.conflict_branching.then_some(self.gate)
    }
}

/// The branching settings that the searchers of one search share, and whose gate the gate trial may change while it
/// runs.  A change takes effect for the units a searcher starts after it.
#[derive(Debug)]
pub struct Branching {
    /// Raised by every change, after the settings are stored: a searcher that sees a generation it has not read the
    /// settings at reads them again.
    generation: AtomicU64,
    settings: Mutex<BranchSettings>,
}

impl Branching {
    pub fn new(settings: BranchSettings) -> Self {
        Branching { generation: AtomicU64::new(0), settings: Mutex::new(settings) }
    }

    /// The settings now.
    pub fn settings(&self) -> BranchSettings {
        *self.settings.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Sets the gate, the gap from which `conflict_branching` chooses, only if no change has come since the generation
    /// `seen` (read before the settings it was decided from): whether it did.  The gate is kept while the tactic is off.
    pub(super) fn set_gate_unchanged(&self, seen: u64, gate: u64) -> bool {
        let mut settings = self.settings.lock().unwrap_or_else(PoisonError::into_inner);
        // Every change raises the generation while it holds the lock, so under the lock it is current.
        if self.generation.load(Ordering::Acquire) != seen {
            return false;
        }
        settings.gate = gate;
        self.generation.fetch_add(1, Ordering::Release);
        true
    }

    /// The generation of the settings: it changes with every change of the gate.  A searcher reads it at the start of
    /// each unit, one atomic load, and reads the settings only when it differs from the one it read them at; a reader
    /// that loads the generation of a change in progress reads the new settings now or at its next unit.
    pub(super) fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_follow_the_tactics_and_start_at_the_default_gate() {
        let on = SearchTactics::default();
        let settings = BranchSettings::of(&on);
        assert_eq!(
            settings,
            BranchSettings { conflict_branching: true, gate: CONFLICT_BRANCHING_GAP, conflict_look: true }
        );
        assert_eq!(settings.conflict_gap(), Some(CONFLICT_BRANCHING_GAP));
        // With the tactic off there is no gap anywhere, and the gate is still the default.
        let off = BranchSettings::of(&SearchTactics { conflict_branching: false, ..on });
        assert_eq!(off, BranchSettings { conflict_branching: false, ..settings });
        assert_eq!(off.conflict_gap(), None);
        assert!(!BranchSettings::of(&SearchTactics { conflict_look: false, ..on }).conflict_look);
        // `with_gate` keeps the gate when none is given.
        assert_eq!(settings.with_gate(None), settings);
        assert_eq!(settings.with_gate(Some(500)).gate, 500);
        assert_eq!(settings.with_gate(Some(500)).conflict_gap(), Some(500));
    }

    #[test]
    fn a_new_gate_raises_the_generation_and_is_read_back() {
        let live = Branching::new(BranchSettings::of(&SearchTactics::default()));
        let seen = live.generation();
        assert!(live.set_gate_unchanged(seen, 150));
        assert_ne!(live.generation(), seen);
        assert_eq!((live.settings().gate, live.settings().conflict_gap()), (150, Some(150)));
        assert!(live.settings().conflict_branching && live.settings().conflict_look);
    }

    #[test]
    fn the_gate_is_kept_while_conflict_branching_is_off() {
        let off = SearchTactics { conflict_branching: false, ..SearchTactics::default() };
        let live = Branching::new(BranchSettings::of(&off));
        assert!(live.set_gate_unchanged(live.generation(), 0));
        assert_eq!((live.settings().gate, live.settings().conflict_gap()), (0, None));
    }

    #[test]
    fn a_gate_set_unless_changed_yields_to_a_change_made_since() {
        let live = Branching::new(BranchSettings::of(&SearchTactics::default()));
        let seen = live.generation();
        assert!(live.set_gate_unchanged(seen, 600));
        assert_eq!(live.settings().gate, 600);
        // Decided from the generation before that change, a second gate is not set.
        assert!(!live.set_gate_unchanged(seen, 1200));
        assert_eq!(live.settings().gate, 600);
    }
}
