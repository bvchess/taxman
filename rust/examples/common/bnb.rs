//! The plain branch and bound of the matroid and fields studies, which proves that no promotion set beats `P*`, with
//! one bound at every node: the node is cut when the `max_add` largest candidates, or the bound, fall below what it
//! needs (`need = P* + 1 − sum(promoted)`); then, if an extra bound is given and the bound is the GF(2) one, the
//! extra bound; otherwise the bound's heaviest set is promotable (a completion better than `P*`: counted as `better`,
//! and impossible if the bound and `P*` are right) or has a conflict `k₁ < … < k_r`, and the node branches into
//! "`k₁ … k_{j−1}` promoted, `k_j` not" for each `j` (every completion leaves out a member of the conflict).  No
//! refinement, forced members, nogoods or strong branching: the same skeleton for every bound, so that their trees
//! compare.  The extra bound never changes the children, so the tree with an extra bound is a subtree of the tree
//! without it.
//!
//! A [`Search`] holds the stack and the counters; [`Search::step`] treats one node and [`Search::dive`] is one dive
//! of Knuth's estimator.  A dive from the root goes to a child chosen uniformly at each node, until a node with no
//! children; the sum over the nodes it meets of the product of the numbers of children of the nodes before them has
//! the tree's node count as its mean (Knuth, "Estimating the efficiency of backtrack programs", 1975).  The dive is
//! paired: it also sums over the nodes met before the extra bound cuts the dive's path, which has the mean of the
//! node count of the tree with the extra bound, from the same dives.
//!
//! Included with `#[path = "common/bnb.rs"] mod bnb;` next to `#[path = "common/frames.rs"] mod frames;`, whose
//! matroids and generator it uses.

use crate::frames::{frame_matroids, FrameMatroids, Rng};
use std::time::{Duration, Instant};
use taxman::bits::Bits;
use taxman::matchbound::{MatchBound, MatchGraph};
use taxman::model::{FrameSet, Scratch};
use taxman::search::Prepared;

/// The bound every node is measured with first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundKind {
    /// The heaviest common independent set of the GF(2) frame matroids.
    Gf2,
    /// The GF(2) bound, started from the parent's heaviest set where that is extreme.
    Gf2Warm,
    /// The matching bound.
    Matching,
}

/// What a node hands to its extra bound: the state, the node's candidates (narrowed), promotions, the GF(2)
/// matroids and bound already computed, and `need`.
pub struct NodeView<'a, const W: usize> {
    /// The node's state: the root with its promotions made.
    pub state: &'a FrameSet<W>,
    /// The members promoted on the node's path.
    pub promoted: Bits<W>,
    /// The node's candidates after narrowing.
    pub cands: Bits<W>,
    /// The GF(2) frame matroids of the state over the candidates.
    pub fm: &'a FrameMatroids,
    /// The GF(2) bound (capped at `max_add`).
    pub gf2: i64,
    /// What a completion must reach to beat `P*`: `P* + 1 − sum(promoted)`.
    pub need: u64,
    /// The most members a completion can add.
    pub max_add: usize,
    /// The node's depth (0 at the root).
    pub depth: usize,
    /// The members the branch from the parent promoted (`k₁ … k_{j−1}`) and the member it left out (`k_j`); both
    /// empty at the root.
    pub promoted_now: Bits<W>,
    /// The member the branch from the parent left out (`k_j`); empty at the root.
    pub removed: Bits<W>,
}

/// What an extra bound reports of a node, for the per-node logs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NodeLog {
    /// The bound at the inherited state (Tier 1) less the target `need − 1`, if it was evaluated.
    pub excess_tier1: Option<i64>,
    /// Refit pricings spent at the node.
    pub pricings: usize,
    /// Whether a refit ran.
    pub refit: bool,
}

/// A bound evaluated at the nodes the GF(2) bound leaves open, with a state that a node hands to its children.
pub trait ExtraBound<const W: usize> {
    /// What a node hands to its children.
    type State: Clone;

    /// The node's bound and its final state for the children; `parent` is the parent's final state (None at the
    /// root).  The node is cut when the bound is below `v.need`.
    fn bound(&mut self, v: &NodeView<'_, W>, parent: Option<&Self::State>) -> (i64, Self::State, NodeLog);
}

/// No extra bound.
pub struct NoExtra;

impl<const W: usize> ExtraBound<W> for NoExtra {
    type State = ();

    fn bound(&mut self, _: &NodeView<'_, W>, _: Option<&()>) -> (i64, (), NodeLog) {
        (i64::MAX, (), NodeLog::default())
    }
}

/// What became of a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fate {
    /// Cut by the count test.
    Count,
    /// Cut by the first bound.
    Bound,
    /// Cut by the extra bound at the inherited state (its `NodeLog::excess_tier1` at most 0).
    Tier1,
    /// Cut by the extra bound after a refit.
    Refit,
    /// The heaviest set is promotable (`better`).
    Leaf,
    /// Branched.
    Branched,
}

/// One node visited.
#[derive(Clone, Copy, Debug)]
pub struct NodeRecord<const W: usize> {
    /// Its number in the order visited (0 the root).
    pub id: u64,
    /// Its parent's number.
    pub parent: Option<u64>,
    /// Its depth (0 at the root).
    pub depth: usize,
    /// The members promoted on its path.
    pub promoted: Bits<W>,
    /// Its candidates after narrowing.
    pub cands: Bits<W>,
    /// The first bound less the target `need − 1` (None when the count test cut it).
    pub excess: Option<i64>,
    /// What became of it.
    pub fate: Fate,
    /// Children pushed (0 unless Branched).
    pub children: usize,
    /// What the extra bound reported of it (the default when there was none, or it did not run).
    pub log: NodeLog,
}

/// The counters of a run (those of the old `bnb` line, and the extra bound's cuts).
#[derive(Clone, Debug, Default)]
pub struct Tally {
    /// Nodes visited.
    pub nodes: u64,
    /// Nodes cut by the count test.
    pub cut_by_count: u64,
    /// Nodes cut by the first bound.
    pub cut_by_bound: u64,
    /// Nodes cut by the extra bound at the inherited state.
    pub cut_tier1: u64,
    /// Nodes cut by the extra bound after a refit.
    pub cut_refit: u64,
    /// Children made.
    pub branches: u64,
    /// Nodes whose heaviest set was promotable.
    pub better: u64,
    /// GF(2) bounds started from the parent's heaviest set.
    pub warm: u64,
    /// GF(2) bounds started from nothing.
    pub cold: u64,
    /// Augmentations of the GF(2) bounds.
    pub augmentations: u64,
    /// Time in the first bound (with the matroids' build), in building the GF(2) matroids, in the extra bound.
    pub bound_time: Duration,
    /// Time in building the GF(2) matroids.
    pub build_time: Duration,
    /// Time in the extra bound.
    pub extra_time: Duration,
}

/// A node waiting on the stack.
#[derive(Clone)]
pub struct Pending<const W: usize, S> {
    /// The members promoted on its path.
    pub promoted: Bits<W>,
    /// Its state: the root with the promotions made.
    pub state: FrameSet<W>,
    /// Its candidates before narrowing.
    pub cands: Bits<W>,
    /// The parent's heaviest set less the members promoted (the warm start of Gf2Warm).
    pub parent_chosen: Bits<W>,
    /// Its depth (0 at the root).
    pub depth: usize,
    /// Its parent's number.
    pub parent: Option<u64>,
    /// The members the branch from the parent promoted.
    pub promoted_now: Bits<W>,
    /// The member the branch from the parent left out.
    pub removed: Bits<W>,
    /// The promotions of its path in the order they were made, one at a time, each by `promote_all_using` of a
    /// single member (so `state` can be rebuilt from the root exactly, for a checkpoint).
    pub order: Vec<u32>,
    /// The parent's final state of the extra bound.
    pub extra: Option<S>,
}

/// A branch and bound in progress: the stack, the counters, and what a node needs.
pub struct Search<'p, const W: usize, S> {
    /// The game.
    pub p: &'p Prepared<W>,
    /// `P*`: the search proves that nothing beats it.
    pub opt: u64,
    /// The bound every node is measured with first.
    pub kind: BoundKind,
    /// The nodes waiting; the last is treated next.
    pub stack: Vec<Pending<W, S>>,
    /// The counters.
    pub tally: Tally,
    mb: MatchBound<'p, W>,
    scratch: Scratch<W>,
}

impl<'p, const W: usize, S: Clone> Search<'p, W, S> {
    /// A search from the root (`graph` is `MatchGraph::new(&p.model, &p.root)`, for the matching bound).
    pub fn new(p: &'p Prepared<W>, graph: &'p MatchGraph, opt: u64, kind: BoundKind) -> Self {
        let mut search = Search {
            p,
            opt,
            kind,
            stack: Vec::new(),
            tally: Tally::default(),
            mb: MatchBound::new(graph),
            scratch: Scratch::new(),
        };
        search.stack.push(search.root());
        search
    }

    /// The root's entry (what `new` pushes).
    pub fn root(&self) -> Pending<W, S> {
        Pending {
            promoted: Bits::EMPTY,
            state: self.p.root,
            cands: self.p.candidates,
            parent_chosen: Bits::EMPTY,
            depth: 0,
            parent: None,
            promoted_now: Bits::EMPTY,
            removed: Bits::EMPTY,
            order: Vec::new(),
            extra: None,
        }
    }

    /// Pops a node and treats it (pushing its children, the last of which is treated next); its record, or None
    /// when the stack is empty.
    pub fn step<E: ExtraBound<W, State = S>>(&mut self, extra: Option<&mut E>) -> Option<NodeRecord<W>> {
        let node = self.stack.pop()?;
        let (record, children) = self.expand(node, extra, false);
        self.stack.extend(children);
        Some(record)
    }

    /// One dive of Knuth's estimator from the root (see the module): the estimates of the node counts of the tree
    /// without the extra bound and of the tree with it, from the same dive.  With no extra bound they are equal.  The
    /// dive does not touch the stack; it adds to the tally as `step` does, except that it also makes (and counts in
    /// `branches`) the children of the nodes that the extra bound cuts, which it goes on through for the first tree.
    pub fn dive<E: ExtraBound<W, State = S>>(&mut self, mut extra: Option<&mut E>, rng: &mut Rng) -> (f64, f64) {
        let (mut gf2, mut with_extra, mut product) = (0.0, 0.0, 1.0);
        let mut alive = true;
        let mut node = self.root();
        loop {
            let (record, mut children) = self.expand(node, if alive { extra.as_deref_mut() } else { None }, true);
            gf2 += product;
            if alive {
                with_extra += product;
            }
            if matches!(record.fate, Fate::Tier1 | Fate::Refit) {
                alive = false;
            }
            if children.is_empty() {
                return (gf2, with_extra);
            }
            product *= children.len() as f64;
            node = children.swap_remove(rng.below(children.len() as u64) as usize);
        }
    }

    /// Treats a node: its record, and its children in the order `step` pushes them.  The extra bound's cut nodes get
    /// no children unless `children_when_extra_cuts`.
    fn expand<E: ExtraBound<W, State = S>>(
        &mut self,
        node: Pending<W, S>,
        extra: Option<&mut E>,
        children_when_extra_cuts: bool,
    ) -> (NodeRecord<W>, Vec<Pending<W, S>>) {
        let p = self.p;
        let model = &p.model;
        let Pending {
            promoted,
            state,
            cands,
            parent_chosen,
            depth,
            parent,
            promoted_now,
            removed,
            order,
            extra: inherited,
        } = node;
        assert!(
            extra.is_none() || self.kind != BoundKind::Matching,
            "an extra bound needs the GF(2) matroids: use the Gf2 or Gf2Warm bound"
        );
        let id = self.tally.nodes;
        self.tally.nodes += 1;
        let cands = state.narrow_candidates(&cands);
        let need = (self.opt + 1).saturating_sub(promoted.sum());
        let max_add = (p.max_total as i32 - promoted.len() as i32).min(state.max_promotions_among(&cands)).max(0);
        let mut record = NodeRecord {
            id,
            parent,
            depth,
            promoted,
            cands,
            excess: None,
            fate: Fate::Count,
            children: 0,
            log: NodeLog::default(),
        };
        if cands.sum_largest(max_add as u32) < need {
            self.tally.cut_by_count += 1;
            return (record, Vec::new());
        }

        // The first bound.
        let tb = Instant::now();
        let (value, chosen, fm) = match self.kind {
            BoundKind::Gf2 | BoundKind::Gf2Warm => {
                let fm = frame_matroids(model, &state, &cands);
                self.tally.build_time += tb.elapsed();
                let start: Vec<bool> = fm.elems.iter().map(|&x| parent_chosen.contains(x)).collect();
                let size = start.iter().filter(|&&s| s).count();
                let warmed = (self.kind == BoundKind::Gf2Warm && size > 0 && size <= max_add as usize)
                    .then(|| fm.warm(&start, max_add as usize))
                    .flatten();
                let m = match warmed {
                    Some(m) => {
                        self.tally.warm += 1;
                        m
                    }
                    None => {
                        self.tally.cold += 1;
                        fm.bound(max_add as usize)
                    }
                };
                self.tally.augmentations += m.augmentations as u64;
                (m.value as u64, fm.chosen::<W>(&m), Some(fm))
            }
            BoundKind::Matching => {
                self.mb.reset(&promoted, &cands);
                (self.mb.value(), self.mb.chosen(), None)
            }
        };
        self.tally.bound_time += tb.elapsed();
        record.excess = Some(value as i64 - (need as i64 - 1));
        if value < need {
            self.tally.cut_by_bound += 1;
            record.fate = Fate::Bound;
            return (record, Vec::new());
        }

        // The extra bound.
        let mut next_extra = None;
        let mut extra_cut = false;
        if let Some(e) = extra {
            let fm = fm.as_ref().expect("the GF(2) matroids of a Gf2 or Gf2Warm node");
            let view = NodeView {
                state: &state,
                promoted,
                cands,
                fm,
                gf2: value as i64,
                need,
                max_add: max_add as usize,
                depth,
                promoted_now,
                removed,
            };
            let te = Instant::now();
            let (bound, final_state, log) = e.bound(&view, inherited.as_ref());
            self.tally.extra_time += te.elapsed();
            record.log = log;
            extra_cut = bound < need as i64;
            next_extra = Some(final_state);
        }

        // The branching.
        let conflict = model.conflict_using(&mut self.scratch, &state, &chosen, true);
        if extra_cut {
            assert!(conflict.is_some(), "the extra bound cut a node whose heaviest set is promotable");
            if record.log.excess_tier1.is_some_and(|x| x <= 0) {
                self.tally.cut_tier1 += 1;
                record.fate = Fate::Tier1;
            } else {
                self.tally.cut_refit += 1;
                record.fate = Fate::Refit;
            }
            if !children_when_extra_cuts {
                return (record, Vec::new());
            }
        }
        let Some(k) = conflict else {
            self.tally.better += 1;
            record.fate = Fate::Leaf;
            return (record, Vec::new());
        };
        let mut children = Vec::new();
        let (mut required, mut rest, mut at) = (Bits::<W>::EMPTY, cands, state);
        let mut path = order;
        for x in k.iter() {
            rest.remove(x);
            children.push(Pending {
                promoted: promoted | required,
                state: at,
                cands: rest,
                parent_chosen: chosen - required,
                depth: depth + 1,
                parent: Some(id),
                promoted_now: required,
                removed: Bits::EMPTY.with(x),
                order: path.clone(),
                extra: next_extra.clone(),
            });
            self.tally.branches += 1;
            match model.promote_all_using(&mut self.scratch, &at, &Bits::EMPTY.with(x)) {
                Some(next) => {
                    at = next;
                    required.insert(x);
                    path.push(x);
                }
                None => break,
            }
        }
        if !extra_cut {
            record.fate = Fate::Branched;
            record.children = children.len();
        }
        (record, children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::{setup, Known};

    /// An extra bound that cuts every node of depth `cut_depth` or more, and checks what the nodes hand it: its
    /// state is the node's depth, so the parent's state it is given must be the depth less one.
    struct CutAtDepth {
        cut_depth: usize,
    }

    impl<const W: usize> ExtraBound<W> for CutAtDepth {
        type State = usize;

        fn bound(&mut self, v: &NodeView<'_, W>, parent: Option<&usize>) -> (i64, usize, NodeLog) {
            assert_eq!(parent.copied(), v.depth.checked_sub(1), "the parent's state");
            assert_eq!(v.depth == 0, v.promoted_now.is_empty() && v.removed.is_empty());
            assert!(v.gf2 >= v.need as i64, "a node the extra bound sees is open for the GF(2) bound");
            let bound = if v.depth >= self.cut_depth { 0 } else { i64::MAX };
            (bound, v.depth, NodeLog { excess_tier1: Some(0), ..NodeLog::default() })
        }
    }

    /// The tree of `n` under the GF(2) bound: the records of its nodes in the order visited, and the tally.
    fn tree<const W: usize, E: ExtraBound<W, State = usize>>(
        n: u32,
        mut extra: Option<&mut E>,
    ) -> (Vec<NodeRecord<W>>, Tally) {
        let known = Known::load(&[], None).expect("the built-in optima");
        let (board, p) = setup::<W>(n).expect("a game with candidates");
        let opt = known.opt(n, board.in_the_bag).expect("a known optimum");
        let graph = MatchGraph::new(&p.model, &p.root);
        let mut s = Search::<W, usize>::new(&p, &graph, opt, BoundKind::Gf2);
        let mut records = Vec::new();
        while let Some(r) = s.step(extra.as_deref_mut()) {
            records.push(r);
        }
        (records, s.tally)
    }

    /// The nodes of the tree of `n` with no extra bound, checked for the shape of a tree.
    fn plain_nodes<const W: usize>(n: u32) -> u64 {
        let (records, tally) = tree::<W, CutAtDepth>(n, None);
        assert_eq!(records.len() as u64, tally.nodes, "n={n}");
        assert_eq!(tally.better, 0, "n={n}: the bound and P* are right");
        assert_eq!(records.iter().map(|r| r.children as u64).sum::<u64>(), tally.nodes - 1, "n={n}");
        assert_eq!(tally.branches, tally.nodes - 1, "n={n}");
        for (i, r) in records.iter().enumerate() {
            assert_eq!(r.id, i as u64, "n={n}: ids run in the order visited");
            match r.parent {
                None => assert_eq!((i, r.depth), (0, 0), "n={n}: only the root has no parent"),
                Some(q) => {
                    assert!(q < r.id, "n={n}: node {} has parent {q}", r.id);
                    assert_eq!(records[q as usize].depth + 1, r.depth, "n={n}");
                    assert_eq!(records[q as usize].fate, Fate::Branched, "n={n}");
                }
            }
            assert_eq!(r.children > 0, r.fate == Fate::Branched, "n={n}");
            assert_eq!(r.excess.is_none(), r.fate == Fate::Count, "n={n}");
        }
        tally.nodes
    }

    #[test]
    fn steps_make_a_tree() {
        for n in [300u32, 360, 420, 480] {
            taxman::with_width!(n, W => plain_nodes::<W>(n));
        }
        taxman::with_width!(600, W => plain_nodes::<W>(600));
    }

    /// The nodes of the tree of 600 by exact count, and the dives' mean, with and without an extra bound that
    /// changes no children.
    fn dives<const W: usize>() {
        let n = 600;
        let nodes = plain_nodes::<W>(n) as f64;
        let known = Known::load(&[], None).expect("the built-in optima");
        let (board, p) = setup::<W>(n).expect("a game with candidates");
        let opt = known.opt(n, board.in_the_bag).expect("a known optimum");
        let graph = MatchGraph::new(&p.model, &p.root);
        let mut s = Search::<W, ()>::new(&p, &graph, opt, BoundKind::Gf2);
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let mut mean = 0.0;
        for _ in 0..200 {
            let (gf2, with_extra) = s.dive(Some(&mut NoExtra), &mut rng);
            assert_eq!(gf2, with_extra, "an extra bound that never cuts leaves the trees equal");
            mean += gf2 / 200.0;
        }
        assert!(mean > nodes / 3.0 && mean < nodes * 3.0, "the estimate {mean} of {nodes} nodes");
        assert_eq!(s.stack.len(), 1, "dives leave the stack alone");

        // An extra bound that cuts at depth 2: the same dives (the children are the same), the extra tree no larger.
        let mut cut = Search::<W, usize>::new(&p, &graph, opt, BoundKind::Gf2);
        let mut plain = Search::<W, usize>::new(&p, &graph, opt, BoundKind::Gf2);
        let (mut rng_cut, mut rng_plain) = (Rng(0x2545_F491_4F6C_DD1D), Rng(0x2545_F491_4F6C_DD1D));
        let mut shrunk = false;
        for _ in 0..50 {
            let (gf2, with_extra) = cut.dive(Some(&mut CutAtDepth { cut_depth: 2 }), &mut rng_cut);
            let (gf2_plain, _) = plain.dive(None::<&mut CutAtDepth>, &mut rng_plain);
            assert_eq!(gf2, gf2_plain, "the extra bound does not change the children");
            assert!(with_extra <= gf2);
            shrunk |= with_extra < gf2;
        }
        assert!(shrunk, "an extra bound that cuts at depth 2 shrinks some dive's tree");
    }

    #[test]
    fn dives_estimate_the_tree() {
        taxman::with_width!(600, W => dives::<W>());
    }

    /// The tree of `n` with an extra bound that cuts at depth 3 is a subtree of the tree with none.
    fn subtree<const W: usize>(n: u32) {
        let (whole, _) = tree::<W, CutAtDepth>(n, None);
        let (cut, tally) = tree(n, Some(&mut CutAtDepth { cut_depth: 3 }));
        assert!(cut.len() < whole.len(), "n={n}");
        assert_eq!(tally.cut_tier1 as usize, cut.iter().filter(|r| r.fate == Fate::Tier1).count(), "n={n}");
        assert_eq!(tally.cut_refit, 0, "n={n}");
        assert!(cut.iter().all(|r| r.depth <= 3), "n={n}: no node below the cut depth");
        assert!(cut.iter().any(|r| r.fate == Fate::Tier1 && r.depth == 3), "n={n}");
        let promotions: std::collections::HashSet<_> = whole.iter().map(|r| r.promoted).collect();
        assert!(cut.iter().all(|r| promotions.contains(&r.promoted)), "n={n}");
    }

    #[test]
    fn extra_bound_cuts_a_subtree() {
        taxman::with_width!(600, W => subtree::<W>(600));
    }
}
