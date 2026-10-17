//! A Lagrangian bound over nogoods: the matching bound with the nogoods the search has learned folded into the
//! weights, kept incrementally along the search path like the matching bound itself.
//!
//! # The bound
//!
//! A *nogood* `N` is a set of candidates that cannot all be promoted from the root ([`crate::model::Nogoods`]).
//! Take a node with promotions `P` and candidates `C`, and a nogood that is *live* there: not inside `P` (it
//! cannot be: `P` was promoted), with every member outside `P` still a candidate (`N − P ⊆ C`).  Every completion
//! `F ⊆ C` leaves out a member of `N − P`, so `|F ∩ N| ≤ |N − P| − 1`.  Dualizing these inequalities with
//! multipliers `λ_N ≥ 0`: for every completion `F`,
//!
//! ```text
//! sum(F) = Σ_{x ∈ F} (x − Σ_{N ∋ x} λ_N) + Σ_N λ_N |F ∩ N|  ≤  Σ_{x ∈ F} w(x) + Σ_N λ_N (|N − P| − 1)
//! ```
//!
//! over the live nogoods, with `w(x) = x − Σ_{live N ∋ x} λ_N`.  `F` can be matched (see the module docs of
//! [`crate::matchbound`]), so the first sum is at most the heaviest matching with the candidates weighed by `w`:
//! that matching plus the constant `Σ_N λ_N (|N − P| − 1)` bounds every completion, whatever the multipliers
//! (weak duality), and can be far below the matching bound, which knows no nogood.  A nogood that is not live
//! has a member that left the candidates without being promoted, and no completion can break it any more; it
//! contributes nothing (its multiplier is taken as 0 there, which any choice allows).  A candidate whose weight
//! is not positive gains nothing by being promoted (unpromoting it leaves a matching at least as heavy), so the
//! solver treats it as a factor.
//!
//! # Integers, and which way they round
//!
//! Weights and multipliers are integers in units of `1/`[`SCALE`] of a promoted number: a candidate `x` weighs
//! `SCALE·x − Σ λ_N`.  The arithmetic is then exact: the solver's value `L` (the heaviest matching plus the
//! constant, both integers) satisfies `SCALE · sum(F) ≤ L` for every completion `F`, so `sum(F) ≤ ⌊L / SCALE⌋`,
//! and a node that needs `need` is cut exactly when `L < SCALE · need`.  Nothing is rounded anywhere; the
//! multipliers can be any integers `≥ 0`, so fitting them ([`fit`]) affects how tight the bound is, never whether
//! it is one.
//!
//! # Incremental
//!
//! [`LagrangeBound`] keeps a stack of levels, each an optimal matching for its weights with potentials that prove
//! it (the [`MatchBound`](super::MatchBound) conditions), the weights themselves, the set of live nogoods and the
//! constant.  A level is made for a node only when the bound is asked for there, by copying the nearest level
//! below it on the path (an *ancestor*: fewer promotions, more candidates) and applying what changed between
//! them:
//!
//! * a candidate promoted: as for the matching bound (it becomes required); the constant drops by the multiplier
//!   of every live nogood it is in;
//! * a candidate gone without being promoted: as for the matching bound (it becomes a factor), and every live
//!   nogood it is in dies: the constant loses that nogood's term, and each of its other members that is still a
//!   candidate weighs more by the nogood's multiplier.
//!
//! Along a path weights only *rise* (nogoods only die).  A rise by `δ` of a candidate's weight breaks dual
//! feasibility only on its edges as a mover; if it is promoted in the optimum its potential rises by `δ` (the
//! matched edge stays tight, and the value grows by `δ`); otherwise its potential rises just as far as its edges
//! need, if at all, and if it does, the candidate is unmatched from the mover it was paying for (or, free, left
//! pending), and the Hungarian repairs of [`MatchBound`](super::MatchBound) restore optimality.  A candidate whose
//! weight rises above 0 becomes one again (it was a factor).  Weights *fall* only when the multipliers change,
//! and then the levels are dropped and the next bound is solved from scratch ([`LagrangeBound::set_relaxation`]):
//! a level never mixes two sets of multipliers.
//!
//! A level is reused for a node only if the node is reachable from it (its promotions contain the level's, the
//! extra ones and all the node's candidates among the level's candidates), and levels that are not are dropped,
//! so the bound is right whatever order the search asks in; [`LagrangeBound::leave`] drops a node's level when
//! its loop ends, which keeps the stack in step with the path.

use super::{Level, MatchGraph, MatchStats, Role, Search, Weighing, NONE};
use crate::bits::Bits;

/// The weights and multipliers are in units of `1/SCALE` of a promoted number.
pub const SCALE: i64 = 64;

/// Nogoods with positive multipliers, the relaxation a [`LagrangeBound`] solves, indexed for it by the vertices of a
/// [`MatchGraph`].
#[derive(Clone, Debug, Default)]
pub struct Relaxation<const W: usize> {
    /// The nogoods and their multipliers (in units of `1/SCALE`, all positive).
    sets: Vec<Bits<W>>,
    lambda: Vec<i64>,
    /// The members of nogood `j` as vertices: `members[member_start[j]..member_start[j + 1]]`.
    members: Vec<u16>,
    member_start: Vec<u32>,
    /// The nogoods vertex `v` is a member of: `of[of_start[v]..of_start[v + 1]]`.
    of: Vec<u32>,
    of_start: Vec<u32>,
}

impl<const W: usize> Relaxation<W> {
    /// The nogoods `nogoods` with the multipliers `lambda` (one each), on `graph`'s vertices.  Those whose
    /// multiplier is not positive are left out, and so is a nogood with a member that is not a vertex of the graph
    /// (no such nogood can be live, since the search's candidates are vertices).
    pub fn new(graph: &MatchGraph, nogoods: &[Bits<W>], lambda: &[i64]) -> Self {
        assert_eq!(nogoods.len(), lambda.len(), "one multiplier per nogood");
        let mut r = Relaxation { member_start: vec![0], ..Relaxation::default() };
        let mut count = vec![0u32; graph.len() + 1];
        for (&set, &l) in nogoods.iter().zip(lambda) {
            if l <= 0 || set.is_empty() || set.iter().any(|x| graph.vertex_of(x).is_none()) {
                continue;
            }
            r.sets.push(set);
            r.lambda.push(l);
            for x in set.iter() {
                let v = graph.vertex_of(x).expect("checked");
                r.members.push(v as u16);
                count[v] += 1;
            }
            r.member_start.push(r.members.len() as u32);
        }
        r.of_start = std::iter::once(0)
            .chain(count.iter().take(graph.len()).scan(0, |sum, &c| {
                *sum += c;
                Some(*sum)
            }))
            .collect();
        let mut at: Vec<u32> = r.of_start[..graph.len()].to_vec();
        let mut of = vec![0; r.members.len()];
        for j in 0..r.sets.len() {
            for &v in r.members_of(j) {
                of[at[usize::from(v)] as usize] = j as u32;
                at[usize::from(v)] += 1;
            }
        }
        r.of = of;
        r
    }

    /// How many nogoods have a positive multiplier.
    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// The nogoods kept, and their multipliers.
    pub fn nogoods(&self) -> &[Bits<W>] {
        &self.sets
    }

    pub fn multipliers(&self) -> &[i64] {
        &self.lambda
    }

    #[inline]
    fn members_of(&self, j: usize) -> &[u16] {
        &self.members[self.member_start[j] as usize..self.member_start[j + 1] as usize]
    }

    #[inline]
    fn nogoods_of(&self, v: usize) -> &[u32] {
        &self.of[self.of_start[v] as usize..self.of_start[v + 1] as usize]
    }

    /// Words of a set of nogoods.
    fn words(&self) -> usize {
        self.sets.len().div_ceil(64)
    }
}

/// A level's weighing: its weights, live nogoods and constant, and the node it is for.
#[derive(Clone, Debug, Default)]
struct Penalized<const W: usize> {
    /// The weight of each vertex as a promoted candidate: `SCALE` times its number, less the multipliers of the
    /// live nogoods it is a member of.  Kept up to date for the candidates only (`cands`); a vertex that has left
    /// them keeps the weight it had then.
    w: Vec<i64>,
    /// The live nogoods, one bit each.
    live: Vec<u64>,
    /// `Σ λ_N (|N − P| − 1)` over the live nogoods.
    constant: i64,
    /// The node's promotions and candidates (with the candidates whose weight is not positive, which the matching
    /// treats as factors).
    promoted: Bits<W>,
    cands: Bits<W>,
}

impl<const W: usize> Weighing for Penalized<W> {
    #[inline(always)]
    fn of(&self, _: &MatchGraph, u: usize) -> i64 {
        self.w[u]
    }

    fn copy_from(&mut self, other: &Self) {
        self.w.copy_from_slice(&other.w);
        self.live.clear();
        self.live.extend_from_slice(&other.live);
        self.constant = other.constant;
        self.promoted = other.promoted;
        self.cands = other.cands;
    }
}

impl<const W: usize> Penalized<W> {
    #[inline]
    fn is_live(&self, j: u32) -> bool {
        self.live[(j >> 6) as usize] >> (j & 63) & 1 != 0
    }

    #[inline]
    fn kill(&mut self, j: u32) {
        self.live[(j >> 6) as usize] &= !(1 << (j & 63));
    }
}

/// The Lagrangian bound kept up to date along a depth-first search path (see the module docs).  One per search
/// thread.
#[derive(Clone, Debug)]
pub struct LagrangeBound<'g, const W: usize> {
    graph: &'g MatchGraph,
    relaxation: Relaxation<W>,
    /// The levels in use are `levels[..depth]`, each for a node of the path, the deepest on top.
    levels: Vec<Level<W, Penalized<W>>>,
    depth: usize,
    search: Search,
    stats: MatchStats,
    /// Scratch: vertices whose weight rose, to fix up.
    touched: Vec<u16>,
}

impl<'g, const W: usize> LagrangeBound<'g, W> {
    /// A solver for the states of `graph`'s search, with no nogoods (the bound is then the matching bound, scaled).
    pub fn new(graph: &'g MatchGraph) -> Self {
        LagrangeBound {
            graph,
            relaxation: Relaxation::new(graph, &[], &[]),
            levels: Vec::new(),
            depth: 0,
            search: Search::default(),
            stats: MatchStats::default(),
            touched: Vec::new(),
        }
    }

    /// Solves the nogoods of `relaxation` from now on, dropping every level (they were for other weights).
    pub fn set_relaxation(&mut self, relaxation: Relaxation<W>) {
        self.relaxation = relaxation;
        self.depth = 0;
    }

    pub fn relaxation(&self) -> &Relaxation<W> {
        &self.relaxation
    }

    /// Drops every level (a new work unit, say).
    pub fn clear(&mut self) {
        self.depth = 0;
    }

    /// Drops the level of the node with the promotions `promoted` if it is on top: that node's loop has ended.
    pub fn leave(&mut self, promoted: &Bits<W>) {
        if self.depth > 0 && self.levels[self.depth - 1].weighing.promoted == *promoted {
            self.depth -= 1;
        }
    }

    pub fn stats(&self) -> MatchStats {
        self.stats
    }

    /// The bound (scaled by [`SCALE`]) of the node reached from the root by promoting `promoted`, with the
    /// candidates `cands`: at least `SCALE` times the sum of every completion of it.  `None` if its moves cannot be
    /// matched (then it has no completion).  The level made for the node stays on top for its next evaluations and
    /// its children's.
    pub fn value(&mut self, promoted: &Bits<W>, cands: &Bits<W>) -> Option<i64> {
        // The deepest level the node can be reached from; the others are not on its path.
        while self.depth > 0 {
            let top = &self.levels[self.depth - 1].weighing;
            let reachable = top.promoted.is_subset(promoted)
                && (*promoted - top.promoted).is_subset(&top.cands)
                && cands.is_subset(&top.cands);
            if reachable {
                break;
            }
            self.depth -= 1;
        }
        if self.depth == 0 {
            self.push_reset(promoted, cands);
        } else {
            let top = &self.levels[self.depth - 1].weighing;
            if top.promoted != *promoted {
                self.push_copy();
                self.derive(promoted, cands);
            } else if top.cands != *cands {
                self.derive(promoted, cands);
            }
        }
        let LagrangeBound { graph, levels, depth, search, stats, .. } = self;
        let level = &mut levels[*depth - 1];
        stats.evaluations += 1;
        level.settle(graph, search, stats);
        level.feasible.then(|| level.value as i64 + level.weighing.constant)
    }

    /// The candidates the heaviest matching of the last level evaluated promotes.
    pub fn chosen(&self) -> Bits<W> {
        self.levels[self.depth - 1].chosen
    }

    /// The candidates of the heaviest matching of the last level evaluated (settled and feasible) without which
    /// the bound, from the potentials alone, is below `target` (scaled): every completion worth `target / SCALE`
    /// promotes them.  (The live nogoods' inequalities hold for every completion of the node, so the multipliers
    /// and the constant stay as they are without a candidate.)  Those in `known` (forced already) are left out.
    pub fn forced(&self, target: i64, known: &Bits<W>) -> Bits<W> {
        let level = &self.levels[self.depth - 1];
        level.forced(self.graph, target, level.weighing.constant, known)
    }

    /// Pushes a level for the node with `promoted` and `cands`, solved from scratch at the next settle.
    fn push_reset(&mut self, promoted: &Bits<W>, cands: &Bits<W>) {
        let (g, r) = (self.graph, &self.relaxation);
        if self.levels.len() == self.depth {
            let weighing = Penalized { w: vec![0; g.len()], ..Penalized::default() };
            self.levels.push(Level::new(g.len(), weighing));
        }
        let level = &mut self.levels[self.depth];
        self.depth += 1;
        let p = &mut level.weighing;
        p.live.clear();
        p.live.resize(r.words(), 0);
        p.constant = 0;
        for (v, w) in p.w.iter_mut().enumerate() {
            *w = SCALE * i64::from(g.num[v]);
        }
        for (j, (set, &l)) in r.sets.iter().zip(&r.lambda).enumerate() {
            let rest = *set - *promoted;
            if !rest.is_empty() && rest.is_subset(cands) {
                p.live[j >> 6] |= 1 << (j & 63);
                p.constant += l * (i64::from(rest.len()) - 1);
                for &v in r.members_of(j) {
                    p.w[usize::from(v)] -= l;
                }
            }
        }
        p.promoted = *promoted;
        p.cands = *cands;
        level.reset(g, promoted, cands);
    }

    /// Pushes a copy of the top level.
    fn push_copy(&mut self) {
        let d = self.depth;
        if self.levels.len() == d {
            let copy = self.levels[d - 1].clone();
            self.levels.push(copy);
        } else {
            let (lower, upper) = self.levels.split_at_mut(d);
            upper[0].copy_from(&lower[d - 1]);
        }
        self.depth = d + 1;
    }

    /// Makes the top level the node with `promoted` and `cands`, which is reachable from it (see
    /// [`value`](Self::value)): promotes the new promotions, drops the candidates gone, and kills the nogoods they
    /// were in, raising the weights of those nogoods' other members.  The Hungarian repairs wait for the settle.
    fn derive(&mut self, promoted: &Bits<W>, cands: &Bits<W>) {
        let LagrangeBound { graph: g, relaxation: r, levels, depth, touched, .. } = self;
        let level = &mut levels[*depth - 1];
        let added = *promoted - level.weighing.promoted;
        let gone = level.weighing.cands - *promoted - *cands;
        for x in added.iter() {
            level.promote(g, x);
            let v = g.vertex_of(x).expect("candidates are in the root");
            for &j in r.nogoods_of(v) {
                if level.weighing.is_live(j) {
                    let (set, l) = (r.sets[j as usize], r.lambda[j as usize]);
                    if set.is_subset(promoted) {
                        // All of it promoted, which a nogood cannot be: a set that is not one is no longer live,
                        // and its term (with the promotions of the level it was live in) goes.
                        level.weighing.kill(j);
                        let before = set - level.weighing.promoted;
                        level.weighing.constant -= l * (i64::from(before.len()) - 1);
                    } else {
                        level.weighing.constant -= l;
                    }
                }
            }
        }
        level.weighing.promoted = *promoted;
        level.weighing.cands = *cands;
        level.dropped |= gone;
        level.dirty = true;
        touched.clear();
        for x in gone.iter() {
            let v = g.vertex_of(x).expect("candidates are in the root");
            for &j in r.nogoods_of(v) {
                if !level.weighing.is_live(j) {
                    continue;
                }
                level.weighing.kill(j);
                let l = r.lambda[j as usize];
                let rest = r.sets[j as usize] - *promoted;
                level.weighing.constant -= l * (i64::from(rest.len()) - 1);
                for &u in r.members_of(j as usize) {
                    raise(g, level, usize::from(u), l, cands, touched);
                }
            }
        }
        for &u in touched.iter() {
            fix_up(g, level, usize::from(u));
        }
    }
}

/// Raises the weight of the vertex `u` of `level` by `delta` (a nogood it is in died).  If it is a candidate
/// promoted in the optimum, its potential rises with it (its matched edge stays tight) and so does the value; any
/// other candidate is left in `touched` for [`fix_up`] once all the weights have risen.
#[inline]
fn raise<const W: usize>(
    g: &MatchGraph,
    level: &mut Level<W, Penalized<W>>,
    u: usize,
    delta: i64,
    cands: &Bits<W>,
    touched: &mut Vec<u16>,
) {
    if !cands.contains(g.num[u]) {
        // Promoted, or no longer a candidate: its weight counts no more, except that a candidate just gone and still
        // promoted in the optimum leaves the value (when the drop is applied) by the weight it entered with.
        return;
    }
    level.weighing.w[u] += delta;
    let mate = level.mate[u];
    if level.role[u] == Role::Candidate && mate != NONE && usize::from(mate) < u {
        level.y[u] += delta;
        level.value += delta as u64;
    } else {
        touched.push(u as u16);
    }
}

/// Restores dual feasibility at the candidate `u` of `level`, whose weight rose and which the optimum does not
/// promote: it becomes a candidate of the matching if its weight is positive now, and its potential rises as far
/// as its edges as a mover need.  If it rises, the candidate was free (it is left pending) or paying for a mover
/// above it, whose edge is no longer tight (the two are unmatched, and both left pending).
fn fix_up<const W: usize>(g: &MatchGraph, level: &mut Level<W, Penalized<W>>, u: usize) {
    let mate = level.mate[u];
    if level.role[u] == Role::Candidate && mate != NONE && usize::from(mate) < u {
        return; // promoted in the optimum: done by `raise`
    }
    let w = level.weighing.w[u];
    if w <= 0 || g.below(u).is_empty() {
        return; // still no gain in promoting it
    }
    if level.role[u] == Role::Factor {
        level.role[u] = Role::Candidate;
        level.cands.insert(g.num[u]);
    }
    let factors = g.below(u).iter().map(|&f| usize::from(f)).filter(|&f| level.role[f] != Role::Required);
    let need = factors.map(|f| w - level.y[f]).max().unwrap_or(0).max(0);
    if need > level.y[u] {
        level.y[u] = need;
        level.dirty = true;
        if mate != NONE {
            level.unmatch(g, usize::from(mate), u);
        } else {
            level.pending.push(u as u16);
        }
    }
}

/// Fits multipliers for `nogoods` at the node of `graph`'s search with the promotions `promoted` and the
/// candidates `cands` by the subgradient method: at most `iterations` steps of Polyak's rule towards
/// `SCALE · need` (`need`: what a completion of the node must reach to beat the incumbent), starting from `lambda`
/// and leaving in it the multipliers of the lowest bound found.  Returns that bound (scaled), `None` if the node's
/// moves cannot be matched.
///
/// The subgradient of a nogood `N` live at the node is how far the heaviest matching's candidates `Q` go over its
/// room: `|Q ∩ N| − (|N − P| − 1)`; that of the others is 0.  Each step solves the weighted matching from scratch.
pub fn fit<const W: usize>(
    graph: &MatchGraph,
    promoted: &Bits<W>,
    cands: &Bits<W>,
    nogoods: &[Bits<W>],
    lambda: &mut [i64],
    need: u64,
    iterations: usize,
) -> Option<i64> {
    assert_eq!(nogoods.len(), lambda.len(), "one multiplier per nogood");
    let target = SCALE * need as i64;
    let mut solver = LagrangeBound::new(graph);
    let (mut lowest, mut best) = (i64::MAX, lambda.to_vec());
    let mut step_of = vec![0i64; nogoods.len()];
    for _ in 0..iterations {
        solver.set_relaxation(Relaxation::new(graph, nogoods, lambda));
        let value = solver.value(promoted, cands)?;
        if value < lowest {
            lowest = value;
            best.copy_from_slice(lambda);
        }
        if lowest < target {
            break;
        }
        let q = solver.chosen();
        let mut norm = 0;
        for (g, set) in step_of.iter_mut().zip(nogoods) {
            let rest = *set - *promoted;
            let live = !rest.is_empty() && rest.is_subset(cands);
            *g = if live { i64::from((q & rest).len()) - (i64::from(rest.len()) - 1) } else { 0 };
            norm += *g * *g;
        }
        if norm == 0 {
            break; // the matching breaks no nogood: no multiplier lowers the bound
        }
        let step = ((value - target + SCALE) / norm).max(1);
        for (l, g) in lambda.iter_mut().zip(&step_of) {
            *l = (*l + step * g).max(0);
        }
    }
    lambda.copy_from_slice(&best);
    Some(lowest)
}

#[cfg(test)]
mod tests {
    use super::super::match_bound_weighted;
    use super::super::tests::{check_bounds_excluding, check_level, setup, walks};
    use super::*;
    use crate::model::test_support::*;
    use crate::model::{FrameGraph, FrameSet, Scratch};

    /// The bound of the node `(promoted, cands)` with state `state` from its definition, by the reference solver:
    /// the live nogoods, the weights and the constant worked out afresh.
    fn reference<const W: usize>(
        model: &FrameGraph,
        state: &FrameSet<W>,
        promoted: &Bits<W>,
        cands: &Bits<W>,
        nogoods: &[Bits<W>],
        lambda: &[i64],
    ) -> Option<i64> {
        let live: Vec<usize> = (0..nogoods.len())
            .filter(|&j| lambda[j] > 0 && !(nogoods[j] - *promoted).is_empty())
            .filter(|&j| (nogoods[j] - *promoted).is_subset(cands))
            .collect();
        let constant: i64 = live.iter().map(|&j| lambda[j] * (i64::from((nogoods[j] - *promoted).len()) - 1)).sum();
        let weight = |x: u32| {
            SCALE * i64::from(x) - live.iter().filter(|&&j| nogoods[j].contains(x)).map(|&j| lambda[j]).sum::<i64>()
        };
        match_bound_weighted(model, state, cands, weight).map(|(value, _)| value + constant)
    }

    /// Random sets of 2 to 6 of `cands` with random multipliers, some so large that they make weights negative.
    fn random_relaxation<const W: usize>(rng: &mut Rng, cands: &Bits<W>, count: usize) -> (Vec<Bits<W>>, Vec<i64>) {
        let list = cands.to_vec();
        let mut sets = Vec::new();
        let mut lambda = Vec::new();
        for _ in 0..count {
            let size = 2 + rng.below(5) as usize;
            let set: Bits<W> = (0..size).map(|_| rng.pick(&list)).collect();
            sets.push(set);
            lambda.push(match rng.below(10) {
                0 => 0,
                1 => SCALE * (1 + rng.below(400)) as i64,
                _ => 1 + rng.below(SCALE as u64 * 40) as i64,
            });
        }
        (sets, lambda)
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = long!(incremental_solver_matches_the_reference_on_a_sample))]
    fn incremental_solver_matches_the_reference() {
        incremental_solver_matches_the_reference_on_every(1);
    }

    /// [`incremental_solver_matches_the_reference`] on every third of its golden games.
    #[test]
    fn incremental_solver_matches_the_reference_on_a_sample() {
        incremental_solver_matches_the_reference_on_every(3);
    }

    /// The incremental Lagrangian solver matches the reference on every `every`th of the golden games.
    fn incremental_solver_matches_the_reference_on_every(every: usize) {
        let mut checks = 0;
        let ns = golden_ns().into_iter().filter(|&n| n >= 30 && (n % 26 == 0 || [480, 760].contains(&n)));
        for n in ns.step_by(every) {
            let g = load(n);
            let (paths, budget) = if n < 300 { (6, 150) } else { (2, 60) };
            if n < 512 {
                checks += agreement::<8>(&g, paths, budget);
            } else {
                checks += agreement::<12>(&g, paths, budget);
            }
        }
        eprintln!("the Lagrangian bound, incremental against the reference: {checks} evaluations");
        assert!(checks > 2000 / every as u64);
    }

    /// Walks the search paths of `g` with a random relaxation (changed now and then), checking every evaluation of
    /// a [`LagrangeBound`] against the reference, its level's optimality certificate, and a solver started afresh at
    /// the same node; levels are left at random, as the search leaves them when a node's loop ends.
    fn agreement<const W: usize>(g: &Golden, paths: usize, budget: usize) -> u64 {
        let (model, root, all) = setup::<W>(g);
        let graph = MatchGraph::new(&model, &root);
        let mut rng = Rng(u64::from(g.n) * 104_729);
        let count = 1 + rng.below(40) as usize;
        let (mut nogoods, mut lambda) = random_relaxation(&mut rng, &all, count);
        let mut lb = LagrangeBound::new(&graph);
        lb.set_relaxation(Relaxation::new(&graph, &nogoods, &lambda));
        let mut fresh = LagrangeBound::new(&graph);
        let mut checks = 0;
        walks::<W>(g, u64::from(g.n) << 4, paths, budget, &mut |_, state, promoted, cands| {
            if rng.chance(2) {
                let count = rng.below(40) as usize;
                (nogoods, lambda) = random_relaxation(&mut rng, &all, count);
                lb.set_relaxation(Relaxation::new(&graph, &nogoods, &lambda));
            }
            let value = lb.value(promoted, cands);
            let what = format!("n={}, promoted {promoted:?}, cands {cands:?}", g.n);
            assert_eq!(value, reference(&model, state, promoted, cands, &nogoods, &lambda), "{what}");
            check_level(&graph, &lb.levels[lb.depth - 1]);
            check_bounds_excluding(&graph, &lb.levels[lb.depth - 1]);
            if rng.chance(10) {
                fresh.set_relaxation(Relaxation::new(&graph, &nogoods, &lambda));
                assert_eq!(fresh.value(promoted, cands), value, "{what}: started afresh");
            }
            if rng.chance(20) {
                lb.leave(promoted);
            }
            checks += 1;
        });
        checks
    }

    /// Nogoods of the root of `g`: minimal conflicts of random sets of candidates.
    fn real_nogoods<const W: usize>(
        rng: &mut Rng,
        model: &FrameGraph,
        root: &FrameSet<W>,
        all: &Bits<W>,
    ) -> Vec<Bits<W>> {
        let mut scratch = Scratch::new();
        let mut nogoods = Vec::new();
        for _ in 0..200 {
            let percent = 10 + rng.below(40);
            let set = rng.subset(all, percent);
            if let Some(k) = model.conflict_using(&mut scratch, root, &set, true) {
                if !nogoods.contains(&k) {
                    nogoods.push(k);
                }
            }
        }
        nogoods
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = long!(bound_is_sound_on_a_sample))]
    fn bound_is_sound() {
        bound_is_sound_on_every(1);
    }

    /// [`bound_is_sound`] on every sixth of its golden games.
    #[test]
    fn bound_is_sound_on_a_sample() {
        bound_is_sound_on_every(6);
    }

    /// The Lagrangian bound is sound on every `every`th of the golden games.
    fn bound_is_sound_on_every(every: usize) {
        let (mut checks, mut below_plain) = (0, 0);
        for n in golden_ns().into_iter().filter(|&n| (40..=220).contains(&n)).step_by(every) {
            let g = load(n);
            let (model, root, all) = setup::<4>(&g);
            let graph = MatchGraph::new(&model, &root);
            let mut rng = Rng(u64::from(n) * 7_777);
            let nogoods = real_nogoods(&mut rng, &model, &root, &all);
            // Fitted multipliers, and random ones.
            let mut fitted = vec![0; nogoods.len()];
            let most = root.max_promotions_among(&all).max(0) as u32;
            let need = best_completion(&model, &root, &all, most) + 1;
            fit(&graph, &Bits::EMPTY, &all, &nogoods, &mut fitted, need, 50);
            let random: Vec<i64> = nogoods.iter().map(|_| rng.below(SCALE as u64 * 20) as i64).collect();
            for lambda in [fitted, random] {
                let mut lb = LagrangeBound::new(&graph);
                lb.set_relaxation(Relaxation::new(&graph, &nogoods, &lambda));
                walks::<4>(&g, u64::from(n) << 9, 4, 60, &mut |mb, state, promoted, cands| {
                    let bound = lb.value(promoted, cands).expect("a state the search reaches can be matched");
                    let most = state.max_promotions_among(cands).max(0) as u32;
                    let best = best_completion(&model, state, cands, most);
                    assert!(bound >= SCALE * best as i64, "n={n}: bound {bound} < {SCALE} × best {best}, {promoted:?}");
                    below_plain += u64::from(bound < SCALE * mb.value() as i64);
                    checks += 1;
                });
            }
        }
        eprintln!("Lagrangian bound soundness: {checks} states, below the matching bound at {below_plain}");
        assert!(checks > 3000 / every && below_plain > 0);
    }

    // ---- micro-benchmark: cargo test --release bench_lagrange -- --ignored --nocapture --test-threads=1 ----

    /// The root's expansion loop of game 1215 (each step evaluates the root without the candidates tried so far,
    /// and the child that promotes the next one), by the matching bound and by the Lagrangian bound with no
    /// nogoods and with conflicts of the root and fitted multipliers: about the same per step (3.3, 3.6 and 3.7 µs
    /// on the M1 Max), so a Lagrangian level costs what a matching level does.
    #[test]
    #[ignore]
    fn bench_lagrange_1215() {
        use crate::board::Board;
        use crate::search::SearchTactics;
        use std::hint::black_box;
        let board = Board::<20>::for_n(1215);
        let p = board.prepare(SearchTactics::default()).unwrap();
        let (model, root, cands) = (&*p.model, &p.root, p.candidates);
        let graph = MatchGraph::new(model, root);
        let mut rng = Rng(1215);
        let nogoods = real_nogoods(&mut rng, model, root, &cands);
        let mut lambda = vec![0; nogoods.len()];
        fit(&graph, &Bits::EMPTY, &cands, &nogoods, &mut lambda, p.greedy.sum() + 1, 200);
        eprintln!(
            "n=1215: {} vertices, {} candidates, {} nogoods, {} with a multiplier",
            graph.len(),
            cands.len(),
            nogoods.len(),
            lambda.iter().filter(|&&l| l > 0).count()
        );
        let mut left = cands;
        let mut steps = Vec::new();
        for n in cands.iter_desc() {
            let before = left;
            left.remove(n);
            let child = model.promote(root, n).map(|next| next.narrow_candidates(&left));
            steps.push((n, before, child));
        }
        let mut mb = super::super::MatchBound::new(&graph);
        let ns = time_per_call(steps.len() as u64, || {
            mb.reset(&Bits::EMPTY, &cands);
            for &(n, _, child) in &steps {
                black_box(mb.value());
                if let Some(child) = child {
                    mb.push(n, &child);
                    black_box(mb.value());
                    mb.pop();
                }
                mb.remove_candidate(n);
            }
        });
        eprintln!("  matching bound: {:.0} ns per step, {:?}", ns, mb.stats());
        for (label, lambda) in [("no nogoods", vec![0; nogoods.len()]), ("fitted", lambda)] {
            let mut lb = LagrangeBound::new(&graph);
            let ns = time_per_call(steps.len() as u64, || {
                lb.set_relaxation(Relaxation::new(&graph, &nogoods, &lambda));
                for &(n, before, child) in &steps {
                    black_box(lb.value(&Bits::EMPTY, &before));
                    if let Some(child) = child {
                        let promoted = Bits::EMPTY.with(n);
                        black_box(lb.value(&promoted, &child));
                        lb.leave(&promoted);
                    }
                }
            });
            eprintln!("  Lagrangian, {label}: {:.0} ns per step, {:?}", ns, lb.stats());
        }
    }

    #[test]
    fn fitting_lowers_the_root_bound() {
        for n in [120, 200, 300, 400] {
            let g = load(n);
            let (model, root, all) = setup::<8>(&g);
            let graph = MatchGraph::new(&model, &root);
            let mut rng = Rng(u64::from(n) * 31);
            let nogoods = real_nogoods(&mut rng, &model, &root, &all);
            let mut lambda = vec![0; nogoods.len()];
            let plain = SCALE * super::super::match_bound(&model, &root, &all) as i64;
            let lowest = fit(&graph, &Bits::EMPTY, &all, &nogoods, &mut lambda, 1, 100).expect("matchable");
            assert!(lowest <= plain, "n={n}: {lowest} > {plain}");
            let mut lb = LagrangeBound::new(&graph);
            lb.set_relaxation(Relaxation::new(&graph, &nogoods, &lambda));
            assert_eq!(lb.value(&Bits::EMPTY, &all), Some(lowest), "n={n}: the multipliers left are the lowest's");
            eprintln!("n={n}: {} nogoods, root bound {plain} → {lowest} (scaled)", nogoods.len());
        }
    }
}
