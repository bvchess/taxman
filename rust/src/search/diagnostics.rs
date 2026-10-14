//! Measurements of a search for tuning ([`Diagnostics`], collected by [`search_with`](super::search_with)): how
//! steps and loops end, how far the bound is above what was needed, conflict sizes, the matching's work.
//!
//! A searcher collects them only when its [`SearchSpace`](super::SearchSpace) has somewhere to put them; then it
//! computes extra bounds at every step, so a search that collects them is slower and its times mean little.
//! Without them each probe site of a step is a test of an `Option` that is always `None`, which costs nothing
//! measurable: a build with the sites removed runs the probe's search windows (`examples/perf.rs`) in the same
//! cycles and instructions (2026-10).

use super::searcher::{Counts, Node};
use crate::bits::Bits;
use crate::matchbound::MatchBound;
use crate::model::{FrameSet, Model, Scratch};
use std::sync::{Mutex, PoisonError};

/// Why a node's loop ended or a promoted child was not explored, for [`Diagnostics`].
pub mod cut {
    /// No candidates left (a child: none to explore).
    pub const EMPTY: usize = 0;
    /// The sum of the `max_add` largest candidates is too small.
    pub const LARGEST: usize = 1;
    /// The matching bound.
    pub const MATCH: usize = 2;
    /// The matching bound refined by conflicts, or a known conflict all of whose members are forced.
    pub const REFINED: usize = 3;
    /// The heaviest matching can be promoted: the best completion is known (and offered as the incumbent).
    pub const EXACT: usize = 4;
    /// Forced members (tactic `fix_forced`): a node's loop, because they include all of its conflict; a child,
    /// because they cannot all be promoted.
    pub const FORCED: usize = 5;
    /// The Lagrangian bound over nogoods (where the matching bound and its refinement did not cut).
    pub const LAGRANGE: usize = 6;
    /// The GF(2) bound (tactic `gf2`).
    pub const GF2: usize = 7;
    /// A symmetric pair's inequality: no completion keeps it (tactic `symmetry`, Theorem 13).
    pub const SYMMETRY: usize = 8;
    pub const COUNT: usize = 9;
    pub const NAMES: [&str; COUNT] =
        ["empty", "largest", "match", "refined", "exact", "forced", "lagrange", "gf2", "symmetry"];
}

/// Buckets of [`Diagnostics::gaps`]: excesses `0..GAP_BUCKETS − 1`, and the last for everything above.
pub const GAP_BUCKETS: usize = 512;

/// Measurements of one search, for tuning (see [`search_with`](super::search_with)).  Collecting them computes
/// extra bounds at every step, so a search that collects them is slower and its times mean little.
#[derive(Clone, Debug, Default)]
pub struct Diagnostics {
    /// Steps (engine nodes: candidates a node's loop gets to, plus the step that ends the loop), by the number
    /// of promotions of the node taking them.
    pub steps: Vec<u64>,
    /// Children explored, by their number of promotions.
    pub children: Vec<u64>,
    /// How node loops ended, by [`cut`] reason.
    pub loop_ends: [u64; cut::COUNT],
    /// How promoted children were left unexplored, by [`cut`] reason.
    pub child_cuts: [u64; cut::COUNT],
    /// Candidates whose promotion failed.
    pub failed_promotions: u64,
    /// At the node-loop checks that let the loop go on: how far the matching bound was above what the loop
    /// needed, as a histogram (see [`GAP_BUCKETS`]).
    pub gaps: Vec<u64>,
    /// Matching bounds that did not prune, and how many of those had a heaviest matching whose candidates can
    /// all be promoted together (with at most the node's `max_add` of them), which makes the bound exact.
    pub match_open: u64,
    pub match_open_feasible: u64,
    /// At those matching bounds: histograms of the size of the conflict from the core, and of the minimal
    /// conflict (see [`Model::conflict_using`]).
    pub core_sizes: Vec<u64>,
    pub conflict_sizes: Vec<u64>,
    /// At those matching bounds: how far one level of refinement (with the minimal conflict) lowers the bound
    /// (a histogram), and how often that is enough to prune.
    pub refine_drops: Vec<u64>,
    pub refine_cuts: u64,
    /// With the `match_refine` tactic: refinements made, the matching evaluations they made, and the conflicts
    /// they computed (not known yet).
    pub refinements: u64,
    pub refine_evaluations: u64,
    pub refine_conflicts: u64,
    /// With the tactic `symmetry`: the candidates the rule dropped from nodes' and children's candidates.
    pub symmetry_excluded: u64,
    /// The matching solvers' work: evaluations, Hungarian searches, vertices scanned (see
    /// [`MatchStats`](crate::matchbound::MatchStats)).
    pub matching: [u64; 3],
}

fn bump(counts: &mut Vec<u64>, i: usize) {
    if counts.len() <= i {
        counts.resize(i + 1, 0);
    }
    counts[i] += 1;
}

fn add_all(into: &mut Vec<u64>, from: &[u64]) {
    if into.len() < from.len() {
        into.resize(from.len(), 0);
    }
    for (a, b) in into.iter_mut().zip(from) {
        *a += b;
    }
}

impl Diagnostics {
    /// Adds `other`'s counts to these.
    pub fn merge(&mut self, other: &Diagnostics) {
        add_all(&mut self.steps, &other.steps);
        add_all(&mut self.children, &other.children);
        for (a, b) in self.loop_ends.iter_mut().zip(other.loop_ends) {
            *a += b;
        }
        for (a, b) in self.child_cuts.iter_mut().zip(other.child_cuts) {
            *a += b;
        }
        self.failed_promotions += other.failed_promotions;
        add_all(&mut self.gaps, &other.gaps);
        self.match_open += other.match_open;
        self.match_open_feasible += other.match_open_feasible;
        add_all(&mut self.core_sizes, &other.core_sizes);
        add_all(&mut self.conflict_sizes, &other.conflict_sizes);
        add_all(&mut self.refine_drops, &other.refine_drops);
        self.refine_cuts += other.refine_cuts;
        self.refinements += other.refinements;
        self.refine_evaluations += other.refine_evaluations;
        self.refine_conflicts += other.refine_conflicts;
        self.symmetry_excluded += other.symmetry_excluded;
        for (a, b) in self.matching.iter_mut().zip(other.matching) {
            *a += b;
        }
    }

    fn gap(counts: &mut Vec<u64>, gap: u64) {
        bump(counts, (gap as usize).min(GAP_BUCKETS - 1));
    }

    /// The `q`-quantile (0 ≤ q ≤ 1) of a histogram of [`GAP_BUCKETS`] buckets.
    pub fn quantile(counts: &[u64], q: f64) -> Option<usize> {
        let total: u64 = counts.iter().sum();
        if total == 0 {
            return None;
        }
        let target = (q * total as f64).ceil().max(1.0) as u64;
        let mut seen = 0;
        counts.iter().position(|&c| {
            seen += c;
            seen >= target
        })
    }
}

/// A searcher's own diagnostics, merged into the search's at every flush.
pub(super) struct Probe<'a> {
    shared: &'a Mutex<Diagnostics>,
    local: Diagnostics,
    /// The matching solver's statistics already added.
    match_seen: [u64; 3],
}

impl<'a> Probe<'a> {
    pub(super) fn new(shared: &'a Mutex<Diagnostics>) -> Self {
        Probe { shared, local: Diagnostics::default(), match_seen: [0; 3] }
    }

    /// A step of a node with `promotions` promotions.
    pub(super) fn step(&mut self, promotions: u32) {
        bump(&mut self.local.steps, promotions as usize);
    }

    /// A node's loop ended for `reason`.
    pub(super) fn loop_end(&mut self, reason: usize) {
        self.local.loop_ends[reason] += 1;
    }

    /// The promotion that makes a child failed: of a single candidate, or of several forced members.
    pub(super) fn failed(&mut self, single: bool) {
        if single {
            self.local.failed_promotions += 1;
        } else {
            self.local.child_cuts[cut::FORCED] += 1;
        }
    }

    /// A child was made but not explored, for `reason`.
    pub(super) fn child_cut(&mut self, reason: usize) {
        self.local.child_cuts[reason] += 1;
    }

    /// A child with `promotions` promotions is explored.
    pub(super) fn child(&mut self, promotions: u32) {
        bump(&mut self.local.children, promotions as usize);
    }

    /// A node-loop check that let the loop go on (bound at least `need`): records how far above it was, and
    /// whether the matching bound is exact there.
    pub(super) fn open<const W: usize>(
        &mut self,
        scratch: &mut Scratch<W>,
        m: &mut MatchBound<'_, W>,
        model: &Model,
        node: &Node<W>,
        need: u64,
    ) {
        Diagnostics::gap(&mut self.local.gaps, m.value().saturating_sub(need));
        self.matching_open(scratch, m, model, &node.state, node.max_add, need);
    }

    /// A matching bound that did not prune (it is at least `need`): is its heaviest matching a feasible
    /// completion, and what would one level of conflict refinement do?
    pub(super) fn matching_open<const W: usize>(
        &mut self,
        scratch: &mut Scratch<W>,
        m: &mut MatchBound<'_, W>,
        model: &Model,
        state: &FrameSet<W>,
        max_add: u32,
        need: u64,
    ) {
        let chosen = m.chosen();
        let value = m.value();
        self.local.match_open += 1;
        let feasible = chosen.len() <= max_add && model.can_promote_all_using(scratch, state, &chosen);
        self.local.match_open_feasible += u64::from(feasible);
        if let Some(core) = model.conflict_using(scratch, state, &chosen, false) {
            bump(&mut self.local.core_sizes, core.len() as usize);
            let conflict = model.conflict_using(scratch, state, &chosen, true).expect("a conflict");
            bump(&mut self.local.conflict_sizes, conflict.len() as usize);
            let mut refined = 0;
            for k in conflict.iter() {
                m.push_excluding(&Bits::EMPTY.with(k));
                refined = refined.max(m.value());
                m.pop();
            }
            Diagnostics::gap(&mut self.local.refine_drops, value - refined);
            self.local.refine_cuts += u64::from(refined < need);
        }
    }

    /// Adds the searcher's counts of refinement and of the rule of the tactic `symmetry`, and the matching solver's
    /// work since the last flush, and merges everything into the search's diagnostics.
    pub(super) fn flush<const W: usize>(&mut self, counts: &Counts, matching: &MatchBound<'_, W>) {
        self.local.refinements += counts.refinements;
        self.local.refine_evaluations += counts.evaluations;
        self.local.refine_conflicts += counts.conflicts;
        self.local.symmetry_excluded += counts.symmetry_excluded;
        let s = matching.stats();
        let now = [s.evaluations, s.searches, s.scanned];
        for ((total, seen), now) in self.local.matching.iter_mut().zip(&mut self.match_seen).zip(now) {
            *total += now - *seen;
            *seen = now;
        }
        let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        shared.merge(&self.local);
        self.local = Diagnostics::default();
    }
}
