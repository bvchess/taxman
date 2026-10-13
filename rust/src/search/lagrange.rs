//! The Lagrangian bound in the search (tactic `lagrange`): where a searcher tries it, what it does with it, and
//! where its multipliers come from.  The bound and its incremental solver are [`crate::matchbound::lagrange`],
//! whose module docs have the argument that it is a bound.
//!
//! # Where, and what for
//!
//! At the check of a node or a child ([`Lagrangian::check`]), where the matching bound and its refinement leave it
//! open, the searcher asks for the Lagrangian bound of the node (its promotions, its candidates).  If it is below
//! what the node needs, the node is cut.  Otherwise the members of the bound's heaviest matching without which the
//! bound, read from its potentials, would be below what the node needs are *forced*: every completion that beats
//! the incumbent promotes them.  (The live nogoods' inequalities hold for every completion of the node, with or
//! without a given candidate, so the multipliers and the constant stay and the bound without the candidate is
//! still a bound.)  With the tactic `fix_forced` they join the node's forced members, which its child promotes
//! together; that is where most of the gain is (docs/EXPERIMENTS.md, "A Lagrangian bound over the nogoods").
//!
//! # The multipliers
//!
//! Each searcher fits its own, at the root of the search, to the nogoods it has learned ([`crate::model::Nogoods`]):
//! first after [`Refits::first`] of its steps, then after every [`Refits::every`] steps, which is when it forgets
//! its nogoods ([`NOGOOD_PERIOD`]): the refit comes just before the table is cleared and folds what the table
//! learned into the multipliers.  A refit fits the table's nogoods together with those of the relaxation it
//! replaces (their multipliers as they were, the new ones from 0), with [`FIT_STEPS`] subgradient steps towards
//! what the root needs to beat the incumbent ([`fit`]), and keeps the nogoods whose multipliers are positive.  Any
//! nogoods and any multipliers make a bound, so the refits decide how tight it is, never whether it is one.  Like
//! the nogood table, all of this is a cache that work units and checkpoints do not record: a resumed search fits
//! afresh.

use super::NOGOOD_PERIOD;
use crate::bits::Bits;
use crate::matchbound::lagrange::{fit, LagrangeBound, Relaxation, SCALE};
use crate::matchbound::MatchGraph;
use crate::model::Nogoods;
use std::collections::HashSet;

/// When a searcher refits its multipliers: after its first `first` steps, then after every `every` steps (both
/// counted from its start).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refits {
    pub first: u64,
    pub every: u64,
}

impl Refits {
    /// The schedule of the tactic `lagrange`: after 2,000 steps, by when a searcher has learned a few dozen
    /// nogoods, then with every forgetting of its nogoods.  (On the proofs of 1215 and 1254, a first refit after
    /// 500 steps, refits every 10,000 steps, and 20 or 100 subgradient steps instead of [`FIT_STEPS`] each changed
    /// the cycles by less than ±12%: docs/EXPERIMENTS.md.)
    pub const SEARCH: Refits = Refits { first: 2_000, every: NOGOOD_PERIOD };
}

/// Subgradient steps of a refit.
pub const FIT_STEPS: usize = 50;

/// A searcher's Lagrangian bound: the solver along its path, its multipliers, and when to refit them.
pub(super) struct Lagrangian<'a, const W: usize> {
    bound: LagrangeBound<'a, W>,
    graph: &'a MatchGraph,
    refits: Refits,
    /// The searcher's steps so far.
    steps: u64,
}

impl<'a, const W: usize> Lagrangian<'a, W> {
    pub(super) fn new(graph: &'a MatchGraph, refits: Refits) -> Self {
        Lagrangian { bound: LagrangeBound::new(graph), graph, refits, steps: 0 }
    }

    /// A new work unit: the levels of the last one's path are of no more use (the multipliers stay).
    pub(super) fn start(&mut self) {
        self.bound.clear();
    }

    /// The Lagrangian check of the node with the promotions `promoted` and the candidates `cands`, which needs
    /// `need` (see the module docs): `None` if no completion of it reaches `need`, else the forced members found
    /// (with `forcing`; none without).  Before the first refit there are no multipliers, and nothing is found.
    #[inline]
    pub(super) fn check(&mut self, promoted: &Bits<W>, cands: &Bits<W>, need: u64, forcing: bool) -> Option<Bits<W>> {
        if self.bound.relaxation().is_empty() {
            return Some(Bits::EMPTY);
        }
        let target = SCALE * need as i64;
        match self.bound.value(promoted, cands) {
            Some(value) if value >= target => Some(if forcing { self.bound.forced(target) } else { Bits::EMPTY }),
            _ => None,
        }
    }

    /// The loop of the node with the promotions `promoted` has ended.
    #[inline]
    pub(super) fn leave(&mut self, promoted: &Bits<W>) {
        self.bound.leave(promoted);
    }

    /// Counts a step of the searcher and refits the multipliers when the schedule says: to the nogoods of `table`
    /// and of the relaxation, at the root of the search (whose candidates are `candidates`), with the incumbent
    /// worth `best`.  The searcher calls it after each of its steps, before it forgets its nogoods.
    #[inline]
    pub(super) fn step(&mut self, table: &Nogoods<W>, candidates: &Bits<W>, best: u64) {
        self.steps += 1;
        if self.steps == self.refits.first || self.steps.is_multiple_of(self.refits.every) {
            self.refit(table, candidates, best);
        }
    }

    #[cold]
    fn refit(&mut self, table: &Nogoods<W>, candidates: &Bits<W>, best: u64) {
        let relaxation = self.bound.relaxation();
        let mut nogoods = relaxation.nogoods().to_vec();
        let mut lambda = relaxation.multipliers().to_vec();
        let mut seen: HashSet<Bits<W>> = nogoods.iter().copied().collect();
        for &n in table.sets() {
            if seen.insert(n) {
                nogoods.push(n);
                lambda.push(0);
            }
        }
        fit(self.graph, &Bits::EMPTY, candidates, &nogoods, &mut lambda, best + 1, FIT_STEPS);
        self.bound.set_relaxation(Relaxation::new(self.graph, &nogoods, &lambda));
    }
}
