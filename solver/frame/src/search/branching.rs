//! The branching settings of a search, which a caller may change while the search runs ([`Branching`]).
//!
//! A setting here chooses only how a node branches: which conflict, of the true conflicts of its heaviest set, its
//! children leave a member of out.  The children of any conflict cover every completion of their node, whatever the
//! other nodes chose, so a search whose settings change between its units is as exact as one that keeps them
//! (solver/docs/design/lab.md argues it for each setting).  Every searcher reads the settings when it starts a unit,
//! through a generation number, and never during a unit's steps: what a step reads is the searcher's own copy.

use super::tactics::SearchTactics;
use super::CONFLICT_BRANCHING_GAP;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

/// What the branching settings say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BranchSettings {
    /// The gap from which a check branches on the conflict that the tactic `conflict_branching` chooses
    /// ([`CONFLICT_BRANCHING_GAP`] when the tactic is on); `None`: the search's own conflict everywhere.
    pub conflict_branching: Option<u64>,
}

impl BranchSettings {
    /// The settings that `tactics` make: `conflict_branching` from [`CONFLICT_BRANCHING_GAP`] on, if it is on.
    pub fn of(tactics: &SearchTactics) -> Self {
        BranchSettings { conflict_branching: tactics.conflict_branching.then_some(CONFLICT_BRANCHING_GAP) }
    }

    /// Whether the tactic `conflict_branching` is on.
    pub fn conflict_branching_on(&self) -> bool {
        self.conflict_branching.is_some()
    }
}

/// The branching settings that the searchers of one search share, and that a caller may change while it runs.  A
/// change takes effect for the units a searcher starts after it.
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

    /// Changes the settings, for the units the searchers start from now on.
    pub fn set(&self, settings: BranchSettings) {
        *self.settings.lock().unwrap_or_else(PoisonError::into_inner) = settings;
        self.generation.fetch_add(1, Ordering::Release);
    }

    /// Switches the tactic `conflict_branching` on (from [`CONFLICT_BRANCHING_GAP`]) or off.
    pub fn set_conflict_branching(&self, on: bool) {
        self.set(BranchSettings { conflict_branching: on.then_some(CONFLICT_BRANCHING_GAP) });
    }

    /// The generation of the settings: it changes with every [`set`](Self::set).  A searcher reads it at the start of
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
    fn a_change_raises_the_generation_and_is_read_back() {
        let tactics = SearchTactics::default();
        let live = Branching::new(BranchSettings::of(&tactics));
        assert_eq!(live.settings().conflict_branching, Some(CONFLICT_BRANCHING_GAP));
        let seen = live.generation();
        live.set_conflict_branching(false);
        assert_ne!(live.generation(), seen);
        assert_eq!(live.settings(), BranchSettings { conflict_branching: None });
        assert!(!live.settings().conflict_branching_on());
        let off = SearchTactics { conflict_branching: false, ..tactics };
        assert_eq!(BranchSettings::of(&off), live.settings());
    }
}
