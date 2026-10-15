//! The fields study's bound and its multipliers: one GF(2) matroid intersection plus a greedy per signed GF(3) frame
//! matroid, coupled by Lagrangian multipliers that Dantzig–Wolfe column generation fits.  Research code for
//! `examples/fields.rs` (`#[path = "common/dw.rs"] mod dw;`, beside `meet`, `frames`, `gf3`, `cores` and `lp`).
//!
//! # The bound
//!
//! Let `A`, `B` be the two GF(2) frame matroids of a state (`frames::FrameMatroids`) and `N_1, …, N_K` signed GF(3)
//! frame matroids (`gf3::Matroid3`, each of one frame `h_k` and one signing, on the candidates touching that frame,
//! `supp_k`).  Every promotable set `Q` is independent in `A` and `B` (Theorem 11) and has `Q ∩ supp_k` independent
//! in every `N_k` (Lemmas 1-2 of the study: any field, any nonzero entries).  For any multipliers `λ_k ≥ 0` on
//! `supp_k`,
//!
//! ```text
//! w(Q) = (w − Σ_k λ_k)(Q) + Σ_k λ_k(Q ∩ supp_k) ≤ MI_cap(w − Σ_k λ_k) + Σ_k G_k(λ_k) = L(λ)     (Lemma 3)
//! ```
//!
//! with `MI_cap(c)` the heaviest common independent set of `A` and `B` of at most `cap` elements (elements of weight
//! at most 0 left out, which loses nothing over a down-closed family) and `G_k(μ)` the heaviest independent set of
//! `N_k` (greedy).  A candidate that is a loop of some `N_k` is in no promotable set and is left out of the
//! intersection (it is `L` at `λ_{k,x} = w_x`, so Lemma 3 covers it).  The bound reported is `⌊L⌋` evaluated exactly
//! in integers: weights `x·S` and multipliers rounded to the grid `1/S` (`S = 2^16`, [`SCALE`]), the intersection by
//! the crate's `taxman::gf2::meet::Meet` (checked against `common/meet.rs`'s `intersect` every `check_every`-th call)
//! and the greedies over GF(3), all in `i64`.  Its soundness rests on Lemma 3 and the two exact oracles, never on
//! the floating-point LP below, which only proposes multipliers.
//!
//! # The multipliers
//!
//! `min_λ L(λ)` is the LP over `P(A) ∩ P(B) ∩ ⋂_k P(N_k)` with `Σ z ≤ cap` (Lagrangian decomposition with integral
//! subproblems; Edmonds).  Its Dantzig–Wolfe master has a column `α_j` per common independent set `Q_j` of `A`, `B`
//! (cost `w(Q_j)`) and a convexity row, as the design has it, but each `P(N_k)` enters by its rank inequalities
//! `z(S) ≤ r_k(S)` (Edmonds' description of a matroid's polytope) rather than by columns of independent sets linked
//! element by element: that master (link rows `Σ_j α_j [x ∈ Q_j] − Σ_i β_{k,i} [x ∈ I_{k,i}] ≤ ε`, the design's) was
//! built and measured first, and its simplex spent nearly all its pivots in steps of the size of the perturbation
//! (docs/EXPERIMENTS.md).  A rank inequality's dual `μ_{k,S} ≥ 0` gives `λ_{k,x} = Σ_{S ∋ x} μ_{k,S}`, and then
//! `G_k(λ_k) ≤ Σ_S μ_{k,S} r_k(S)`, so the master's dual is a Lagrangian bound's, and at the LP's optimal duals they
//! coincide.  A pricing at multipliers `λ` is one evaluation of `L(λ)`, a valid bound, and its intersection's set
//! is a column.  The inequalities are found from the master's point `z = Σ_j α_j 1_{Q_j}`: cheaply (in each matroid
//! the most violated level set of `z`, and the dependent support sets), and when that finds none, exactly (for each
//! matroid, the largest `y ≤ z` in its polytope, by a small column generation over its independent sets; `y = z`
//! exactly when `z` is in the polytope, and otherwise a level set of its final duals is violated).  When every
//! matroid's check passes, `ẑ = min(z, y_1, y_2, …)` lies in every polytope (all are down-closed), and its weight is
//! a *certified* lower bound `LB` on the LP; with the lowest `L` found it brackets the LP.
//!
//! Before the master, Polyak steps from `λ = 0` (or a given start) towards the target; then Wentges-stabilized
//! pricing at `½ λ_best + ½ λ_master`, or at `λ_master` when that prices nothing out, until nothing prices out or
//! the lowest `L` is within [`CLOSE`] of the master's value, then the cuts and the certificate, or until a budget
//! ends.  With separation, after each bracket the cores that the master's columns run into (`cores::cores_of`) are
//! merged into core-derived signings (first fit, at most `per_frame` a frame), and the master is rebuilt for the
//! changed family from its columns (but those holding a new loop) and its inequalities (ranks taken again).

use crate::cores::{cores_of, merge, Core};
use crate::frames::FrameMatroids;
use crate::gf3::{seed, Basis3, Default3, Matroid3, Signing};
use crate::lp::{Lp, RowKind, Status};
use crate::meet::intersect;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use taxman::gf2::meet::{Meet, Problem};
use taxman::model::{FrameGraph, FrameSet};

/// The grid of the multipliers: weights are `x·SCALE`, multipliers whole multiples of `1/SCALE`.
pub const SCALE: i64 = 1 << 16;
/// The default bracket `L_best − LB` at which the column generation stops (the bounds are floored to integers).
pub const CLOSE: f64 = 0.25;
/// The default share of the root's GF(2) gap at which it stops, if larger (half the study's criterion of convergence,
/// a bracket within 1% of the gap).
pub const CLOSE_GAP: f64 = 0.005;
/// A column enters the master when its reduced cost at the master's duals exceeds this.
const ENTER: f64 = 1e-7;
/// An `α` column is in the master's support when its value exceeds this.
pub const SUPPORT: f64 = 1e-6;
/// A value of the master's point below this is 0.
const ZERO: f64 = 1e-9;
/// A rank inequality is violated when its left side exceeds its rank by more than this.
const VIOLATED: f64 = 1e-6;
/// The scale of the exact check's greedy weights.
const PACK_SCALE: f64 = (1u64 << 30) as f64;
/// The cost of a master's artificial column, which takes up the violation of a rank inequality when it is added
/// (`lp::Lp::add_row_elastic`): above every dual a rank inequality can have (at most the largest weight, a number of
/// the board).
const PENALTY: f64 = 1e7;
/// The most violated rank inequalities an exact check returns.
const CUTS_PER_CHECK: usize = 5;
/// The nonbasic columns the master keeps when it purges, which it does when it has twice as many.
const KEEP: usize = 300;
/// The independent sets an extra matroid's pool keeps.
const POOL: usize = 200;
/// The largest perturbation of a rank inequality's right-hand side in the master, and of a bound `y ≤ z` in the exact
/// check, against degeneracy (uniform in `[0, PERTURB]`).
const PERTURB: f64 = 1e-6;
/// The salt of the seeds of the core-derived signings' default values.
pub const SALT_CORES: u64 = 2;
/// No row, no position.
const NONE: u32 = u32::MAX;

// ---------------------------------------------------------------------------------------------------------
// The setting

/// One bound computation's state: the frames, the GF(2) matroids, the cap and the target.
pub struct Ctx<'a, const W: usize> {
    pub n: u32,
    pub model: &'a FrameGraph,
    pub state: &'a FrameSet<W>,
    pub fm: &'a FrameMatroids,
    /// The most elements of a promotion set (`max_total` at the root, `max_add` at a node).
    pub cap: usize,
    /// What the bound is measured against: `P*` at the root (the Polyak steps aim at it), `P* − sum(promoted)` at a
    /// node.
    pub target: i64,
    /// Stop as soon as the bound is at most the target (a node that is cut needs nothing more).
    pub stop_at_target: bool,
    /// The GF(2) bound's gap to the target at a root (for the closing tolerance), `None` at a node.
    pub gap: Option<i64>,
    /// The GF(2) element of each number (`NONE` for the others).
    index: Vec<u32>,
    /// Words of a block vector in the intersection, and each element's vectors in `A` and `B`, that many words each.
    words: usize,
    va: Vec<u64>,
    vb: Vec<u64>,
    /// Each element's scaled weight `x·SCALE`.
    weight: Vec<i64>,
}

impl<'a, const W: usize> Ctx<'a, W> {
    pub fn new(
        n: u32,
        model: &'a FrameGraph,
        state: &'a FrameSet<W>,
        fm: &'a FrameMatroids,
        cap: usize,
        target: i64,
        stop_at_target: bool,
    ) -> Self {
        let max = fm.elems.iter().copied().max().unwrap_or(0);
        let mut index = vec![NONE; max as usize + 1];
        for (e, &x) in fm.elems.iter().enumerate() {
            index[x as usize] = e as u32;
        }
        let words = fm.a.words.iter().chain(&fm.b.words).copied().max().unwrap_or(1);
        assert!(words <= 3, "frame ranks above 192 are not supported");
        let pad = |vecs: &[Vec<u64>]| -> Vec<u64> {
            let mut out = vec![0u64; vecs.len() * words];
            for (e, v) in vecs.iter().enumerate() {
                out[e * words..e * words + v.len()].copy_from_slice(v);
            }
            out
        };
        let weight = fm.elems.iter().map(|&x| i64::from(x) * SCALE).collect();
        Ctx {
            n,
            model,
            state,
            fm,
            cap,
            target,
            stop_at_target,
            gap: None,
            index,
            words,
            va: pad(&fm.a.vecs),
            vb: pad(&fm.b.vecs),
            weight,
        }
    }

    /// The number of GF(2) elements.
    pub fn len(&self) -> usize {
        self.fm.elems.len()
    }

    /// The GF(2) element of the number `x`, if it is one.
    pub fn element(&self, x: u32) -> Option<usize> {
        self.index.get(x as usize).copied().filter(|&e| e != NONE).map(|e| e as usize)
    }
}

/// The budgets and knobs of a bound computation.
#[derive(Clone, Debug)]
pub struct Settings {
    pub seconds: f64,
    /// The most evaluations of `L` (GF(2) pricings).
    pub pricings: usize,
    /// The most separation rounds.
    pub rounds: usize,
    /// The most core-derived signings of a frame.
    pub per_frame: usize,
    /// The most cores taken from one frame of one column.
    pub cores_per_frame: usize,
    /// The default of the core-derived signings' free incidences: seeded random values, or all 1.
    pub random_defaults: bool,
    /// Polyak steps before the master.
    pub polyak: usize,
    /// Cross-check the intersection against the reference algorithm every this many evaluations (1: every one).
    pub check_every: u64,
    /// The bracket against the master's value at which the pricing of a family stops ([`CLOSE`]), or this share
    /// of the root's GF(2) gap if larger.
    pub close: f64,
    pub close_gap: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            seconds: 120.0,
            pricings: 3000,
            rounds: 40,
            per_frame: 8,
            cores_per_frame: 4,
            random_defaults: true,
            polyak: 100,
            check_every: 50,
            close: CLOSE,
            close_gap: CLOSE_GAP,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The family of extra matroids

/// One extra matroid: a signing, its matroid at the state, and where its elements sit among the GF(2) elements.
pub struct Extra {
    pub signing: Signing,
    pub m: Matroid3,
    /// The GF(2) element of each of its elements.
    pub elem: Vec<usize>,
    /// The position among its elements of each GF(2) element (`NONE` for those that are not).
    pos: Vec<u32>,
    /// Independent sets met so far (the greedy sets of the pricings and the exact checks' columns), the exact
    /// check's starting columns.
    pool: Vec<Vec<usize>>,
}

impl Extra {
    pub fn new<const W: usize>(ctx: &Ctx<'_, W>, signing: Signing) -> Self {
        let m = Matroid3::build(ctx.model, ctx.state, &ctx.fm.elems, &signing);
        let elem: Vec<usize> = m.elems.iter().map(|&x| ctx.element(x).expect("a GF(2) element")).collect();
        let mut pos = vec![NONE; ctx.len()];
        for (i, &e) in elem.iter().enumerate() {
            pos[e] = i as u32;
        }
        Extra { signing, m, elem, pos, pool: Vec::new() }
    }

    fn position(&self, e: usize) -> Option<usize> {
        Some(self.pos[e]).filter(|&p| p != NONE).map(|p| p as usize)
    }

    /// Keeps the independent set `set` (GF(2) elements) in the pool, which holds the latest [`POOL`].
    fn remember(&mut self, set: &[usize]) {
        if !set.is_empty() {
            if self.pool.len() >= POOL {
                self.pool.remove(0);
            }
            self.pool.push(set.to_vec());
        }
    }
}

/// The extra matroids and the GF(2) elements that are loops of one of them.
pub struct Family {
    pub extras: Vec<Extra>,
    pub banned: Vec<bool>,
}

impl Family {
    pub fn new<const W: usize>(ctx: &Ctx<'_, W>, signings: Vec<Signing>) -> Self {
        let extras = signings.into_iter().map(|s| Extra::new(ctx, s)).collect();
        let mut f = Family { extras, banned: Vec::new() };
        f.ban(ctx);
        f
    }

    fn ban<const W: usize>(&mut self, ctx: &Ctx<'_, W>) {
        self.banned = vec![false; ctx.len()];
        for ex in &self.extras {
            for &x in &ex.m.loops {
                self.banned[ctx.element(x).expect("a GF(2) element")] = true;
            }
        }
    }

    /// The number of GF(2) elements that are loops of some extra matroid.
    pub fn loops(&self) -> usize {
        self.banned.iter().filter(|&&b| b).count()
    }

    /// Zero multipliers.
    fn zeros(&self) -> Vec<Vec<f64>> {
        self.extras.iter().map(|ex| vec![0.0; ex.elem.len()]).collect()
    }
}

// ---------------------------------------------------------------------------------------------------------
// Evaluation

/// One evaluation of `L` at multipliers rounded to the grid.
pub struct Eval {
    /// `L` scaled by [`SCALE`], exact.
    pub value: i64,
    /// The intersection's set and each greedy's set, as GF(2) elements, ascending.
    pub q: Vec<usize>,
    pub sets: Vec<Vec<usize>>,
}

/// Counters of a bound computation.
#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub pricings_mi: u64,
    pub pricings_greedy: u64,
    pub checks: u64,
    pub pivots: u64,
    pub degenerate: u64,
    pub solves: u64,
    /// Singular bases the master's simplex met and restarted from (see `lp::LpStats::repairs`).
    pub repairs: u64,
    /// Columns the master purged.
    pub purged: u64,
    /// Rank inequalities added by the cheap separation and by the exact checks, the checks made and their pivots.
    pub cuts_cheap: u64,
    pub cuts_exact: u64,
    pub checks_exact: u64,
    pub pivots_exact: u64,
    pub masters: u64,
    pub max_rows: usize,
    pub max_s: usize,
    pub ms_mi: f64,
    pub ms_simplex: f64,
}

/// The multipliers on the grid: `max(0, round(λ·S))`.
fn grid(lam: &[Vec<f64>]) -> Vec<Vec<i64>> {
    lam.iter().map(|l| l.iter().map(|&v| (v * SCALE as f64).round().max(0.0) as i64).collect()).collect()
}

/// `L` at `lam` (one vector per extra matroid, indexed like its elements), exactly.
pub fn evaluate<const W: usize>(
    ctx: &Ctx<'_, W>,
    fam: &Family,
    lam: &[Vec<f64>],
    stats: &mut Stats,
    check: u64,
) -> Eval {
    let lhat = grid(lam);
    evaluate_grid(ctx, fam, &lhat, stats, check)
}

/// `L` at integer multipliers `lhat` (scaled by [`SCALE`], at least 0).  The intersection is checked against the
/// reference algorithm on every `check`-th call (0: never).
pub fn evaluate_grid<const W: usize>(
    ctx: &Ctx<'_, W>,
    fam: &Family,
    lhat: &[Vec<i64>],
    stats: &mut Stats,
    check: u64,
) -> Eval {
    let mut c = ctx.weight.clone();
    for (ex, l) in fam.extras.iter().zip(lhat) {
        for (&e, &v) in ex.elem.iter().zip(l) {
            assert!(v >= 0, "multipliers are at least 0");
            c[e] -= v;
        }
    }
    let mut order: Vec<usize> = (0..ctx.len()).filter(|&e| !fam.banned[e] && c[e] > 0).collect();
    order.sort_by_key(|&e| (c[e], e));
    let t = Instant::now();
    let (mi, mut q) = match ctx.words {
        1 => intersection::<W, 1>(ctx, &order, &c),
        2 => intersection::<W, 2>(ctx, &order, &c),
        _ => intersection::<W, 3>(ctx, &order, &c),
    };
    stats.ms_mi += t.elapsed().as_secs_f64() * 1e3;
    stats.pricings_mi += 1;
    if check > 0 && stats.pricings_mi.is_multiple_of(check) {
        let w: Vec<i64> = (0..ctx.len()).map(|e| if fam.banned[e] { 0 } else { c[e] }).collect();
        let reference = intersect(&ctx.fm.a, &ctx.fm.b, &w, ctx.cap, false).value;
        assert_eq!(mi, reference, "n={}: the crate's intersection differs from the reference", ctx.n);
        stats.checks += 1;
    }
    q.sort_unstable();
    let mut value = mi;
    let mut sets = Vec::with_capacity(fam.extras.len());
    for (ex, l) in fam.extras.iter().zip(lhat) {
        let (g, kept) = ex.m.greedy(l);
        value += g;
        sets.push(kept.into_iter().map(|i| ex.elem[i]).collect::<Vec<_>>());
        stats.pricings_greedy += 1;
    }
    for s in &mut sets {
        s.sort_unstable();
    }
    Eval { value, q, sets }
}

/// The heaviest common independent set of `A` and `B` of at most `cap` of the elements `order` (ascending by weight
/// `c`, all positive), by the crate's intersection on the elements renumbered in that order.
fn intersection<const W: usize, const R: usize>(ctx: &Ctx<'_, W>, order: &[usize], c: &[i64]) -> (i64, Vec<usize>) {
    if order.is_empty() || ctx.cap == 0 {
        return (0, Vec::new());
    }
    let fm = ctx.fm;
    let vector = |v: &[u64], e: usize| -> [u64; R] {
        let mut out = [0u64; R];
        out.copy_from_slice(&v[e * ctx.words..e * ctx.words + R]);
        out
    };
    let block_a: Vec<u8> = order.iter().map(|&e| fm.a.block[e] as u8).collect();
    let block_b: Vec<u8> = order.iter().map(|&e| fm.b.block[e] as u8).collect();
    let vec_a: Vec<[u64; R]> = order.iter().map(|&e| vector(&ctx.va, e)).collect();
    let vec_b: Vec<[u64; R]> = order.iter().map(|&e| vector(&ctx.vb, e)).collect();
    let weight: Vec<i64> = order.iter().map(|&e| c[e]).collect();
    let p = Problem { block: [&block_a, &block_b], vector: [&vec_a, &vec_b], weight: &weight };
    let mut meet = Meet::<R>::new(order.len(), [fm.a.words.len(), fm.b.words.len()]);
    let live: Vec<u32> = (0..order.len() as u32).collect();
    meet.solve(&p, &live, &[], ctx.cap, i64::MAX, |_| {});
    let set = (0..order.len()).filter(|&i| meet.contains(i as u32)).map(|i| order[i]).collect();
    (meet.value, set)
}

// ---------------------------------------------------------------------------------------------------------
// Ranks and the polytopes of the extra matroids

/// The rank in an extra matroid of the GF(2) elements `set` (its loops and the elements it does not have count 0).
fn rank(ex: &Extra, set: &[usize]) -> usize {
    let mut basis = Basis3::new(ex.m.rank);
    set.iter().filter_map(|&e| ex.position(e)).filter(|&i| basis.try_insert(ex.m.vecs[i].clone())).count()
}

/// The prefixes of `order` (GF(2) elements) whose rank inequalities `z(S) ≤ r(S)` in `ex` are violated by more
/// than [`VIOLATED`], the most violated first, at most `most`: each set ascending.
fn violated_prefixes(ex: &Extra, order: &[usize], z: &[f64], most: usize) -> Vec<Vec<usize>> {
    let mut basis = Basis3::new(ex.m.rank);
    let (mut r, mut sum) = (0.0, 0.0);
    let mut violated: Vec<(f64, usize)> = Vec::new();
    for (j, &e) in order.iter().enumerate() {
        if let Some(i) = ex.position(e) {
            if basis.try_insert(ex.m.vecs[i].clone()) {
                r += 1.0;
            }
        }
        sum += z[e];
        if sum - r > VIOLATED {
            violated.push((sum - r, j));
        }
    }
    violated.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    violated
        .into_iter()
        .take(most)
        .map(|(_, j)| {
            let mut set = order[..=j].to_vec();
            set.sort_unstable();
            set
        })
        .collect()
}

/// The closure of `set` in an extra matroid among the GF(2) elements `within`: `set` and every element of `within`
/// that its vectors span, ascending.  Its rank is `set`'s, so its rank inequality is at least as violated.
fn closure(ex: &Extra, set: &[usize], within: &[usize]) -> Vec<usize> {
    let mut basis = Basis3::new(ex.m.rank);
    for &e in set {
        if let Some(i) = ex.position(e) {
            basis.try_insert(ex.m.vecs[i].clone());
        }
    }
    let mut out: Vec<usize> = set.to_vec();
    for &e in within {
        if set.binary_search(&e).is_err() {
            if let Some(i) = ex.position(e) {
                let mut v = ex.m.vecs[i].clone();
                if !basis.reduce(&mut v) {
                    out.push(e);
                }
            }
        }
    }
    out.sort_unstable();
    out
}

/// The rank inequalities of the extra matroids that the master's point `z` violates, found cheaply: in each
/// matroid, the most violated set of its elements by decreasing `z` (the level sets of `z`) and each support set,
/// each closed among the elements where `z` is positive (a closed set is at least as violated).
fn violated_cuts(fam: &Family, z: &[f64], support: &[(f64, Vec<usize>)]) -> Vec<(usize, Vec<usize>)> {
    let mut out = Vec::new();
    for (k, ex) in fam.extras.iter().enumerate() {
        let mut order: Vec<usize> = ex.elem.iter().copied().filter(|&e| z[e] > ZERO).collect();
        order.sort_by(|&a, &b| z[b].total_cmp(&z[a]).then(a.cmp(&b)));
        let mut within = order.clone();
        within.sort_unstable();
        let mut sets: Vec<Vec<usize>> = violated_prefixes(ex, &order, z, 1);
        sets.extend(support.iter().map(|(_, q)| q.iter().copied().filter(|&e| ex.position(e).is_some()).collect()));
        let mut seen = HashSet::new();
        for set in sets {
            let r = rank(ex, &set) as f64;
            let closed = closure(ex, &set, &within);
            if closed.iter().map(|&e| z[e]).sum::<f64>() - r > VIOLATED && seen.insert(closed.clone()) {
                out.push((k, closed));
            }
        }
    }
    out
}

/// What the exact check of one extra matroid found: the largest point `y ≤ z` of its polytope (on its elements,
/// by GF(2) element), and violated rank inequalities if `z` is not in the polytope.
struct Check {
    y: Vec<(usize, f64)>,
    cuts: Vec<Vec<usize>>,
    pivots: u64,
}

/// The exact check that the master's point `z` lies in the polytope of `ex`.  If the support `T` of `z` among its
/// elements is independent, it does (every point of `[0, 1]^T` does); if the master's support sets (`support`: the
/// columns of value above [`SUPPORT`]) are each independent in `ex`, their combination is a point `y ≤ z` of the
/// polytope, within the columns of smaller value of `z`.  Otherwise the packing LP
/// `max { y(T) : y ≤ z on T, y = Σ_i β_i 1_{I_i}, Σ β ≤ 1, β ≥ 0 }` over the independent sets `I_i` inside `T`, by
/// column generation from maximal independent subsets of the support sets and the pool (a column prices out when
/// the greedy set for the weights `1 − ν`, `ν` the duals of the rows `y ≤ z`, beats the convexity row's dual; the
/// rows' right-hand sides perturbed by at most [`PERTURB`]).
/// Its value is `min_S r(S) + z(T − S)` (the polymatroid intersection theorem), so it reaches `z(T)` exactly when
/// `z` is in the polytope; otherwise, with the final weights `μ = 1 − ν`, `z(T) − value = Σ_t δ_t (z(S_t) − r(S_t))`
/// over the level sets `S_t` of `μ`, so one of them is violated: the most violated are returned, closed.  The point `y`
/// returned is the combination's, at most `z`: in the polytope whatever the LP's accuracy (the polytope is
/// down-closed).
fn check(ex: &mut Extra, z: &[f64], support: &[(f64, Vec<usize>)], deadline: Instant, rng: &mut u64) -> Check {
    let t: Vec<usize> = ex.elem.iter().copied().filter(|&e| z[e] > ZERO).collect();
    let parts: Vec<Vec<usize>> =
        support.iter().map(|(_, q)| q.iter().copied().filter(|&e| ex.position(e).is_some()).collect()).collect();
    if rank(ex, &t) == t.len() {
        return Check { y: t.iter().map(|&e| (e, z[e])).collect(), cuts: Vec::new(), pivots: 0 };
    }
    if parts.iter().all(|p| rank(ex, p) == p.len()) {
        let mut y: HashMap<usize, f64> = HashMap::new();
        for ((a, _), part) in support.iter().zip(&parts) {
            for &e in part {
                *y.entry(e).or_default() += a;
            }
        }
        let y = t.iter().map(|&e| (e, y.get(&e).copied().unwrap_or(0.0).min(z[e]))).collect();
        return Check { y, cuts: Vec::new(), pivots: 0 };
    }
    // The coloops of the matroid restricted to T (elements in no circuit inside T) are a direct summand: the polytope
    // is the product of theirs, [0, 1] each, and the rest's, so the LP is over the rest.
    let r_t = rank(ex, &t);
    let (t, free): (Vec<usize>, Vec<usize>) = t.iter().partition(|&&x| {
        let rest: Vec<usize> = t.iter().copied().filter(|&e| e != x).collect();
        rank(ex, &rest) == r_t
    });
    let total: f64 = t.iter().map(|&e| z[e]).sum();
    let mut lp = Lp::new();
    let conv = lp.add_row(RowKind::Eq, 1.0, &[]);
    let rows: Vec<usize> = t.iter().map(|&e| lp.add_row(RowKind::Le, z[e] + PERTURB * uniform(rng), &[])).collect();
    let row_of: HashMap<usize, usize> = t.iter().copied().zip(rows.iter().copied()).collect();
    let empty = lp.add_column(0.0, &[(conv, 1.0)]);
    lp.reset(&[empty]);
    let mut cols: Vec<(usize, Vec<usize>)> = Vec::new();
    let mut seen: HashSet<Vec<usize>> = HashSet::new();
    let mut add = |lp: &mut Lp, set: Vec<usize>| {
        if !set.is_empty() && seen.insert(set.clone()) {
            let mut entries = vec![(conv, 1.0)];
            entries.extend(set.iter().map(|e| (row_of[e], 1.0)));
            let j = lp.add_column(set.len() as f64, &entries);
            cols.push((j, set));
            true
        } else {
            false
        }
    };
    for part in &parts {
        add(&mut lp, independent_part(ex, part).into_iter().filter(|e| row_of.contains_key(e)).collect());
    }
    for set in &ex.pool {
        add(&mut lp, set.iter().copied().filter(|e| row_of.contains_key(e)).collect());
    }
    let mut mu: Vec<f64>;
    loop {
        lp.solve(u64::MAX);
        let y = lp.duals();
        mu = t.iter().map(|e| 1.0 - y[row_of[e]].max(0.0)).collect();
        let mut weights = vec![0i64; ex.elem.len()];
        for (&e, &m) in t.iter().zip(&mu) {
            weights[ex.position(e).expect("an element")] = (m * PACK_SCALE).round() as i64;
        }
        let (_, kept) = ex.m.greedy(&weights);
        let set: Vec<usize> = kept.into_iter().map(|i| ex.elem[i]).collect();
        let gain: f64 = set.iter().map(|e| 1.0 - y[row_of[e]].max(0.0)).sum::<f64>() - y[conv];
        if gain <= ENTER || Instant::now() >= deadline || !add(&mut lp, set) {
            break;
        }
    }
    let mut y: HashMap<usize, f64> = HashMap::new();
    for (j, set) in &cols {
        let b = lp.primal(*j);
        if b > 0.0 {
            for &e in set {
                *y.entry(e).or_default() += b;
            }
        }
    }
    let pivots = lp.stats().pivots;
    let y: Vec<(usize, f64)> = t
        .iter()
        .map(|&e| (e, y.get(&e).copied().unwrap_or(0.0).min(z[e])))
        .chain(free.iter().map(|&e| (e, z[e])))
        .collect();
    for (_, set) in cols.into_iter().rev().take(64) {
        ex.remember(&set);
    }
    if y[..t.len()].iter().map(|&(_, v)| v).sum::<f64>() >= total - VIOLATED {
        return Check { y, cuts: Vec::new(), pivots };
    }
    let mut order: Vec<usize> = (0..t.len()).filter(|&i| mu[i] > ZERO).collect();
    order.sort_by(|&a, &b| mu[b].total_cmp(&mu[a]).then(a.cmp(&b)));
    let order: Vec<usize> = order.into_iter().map(|i| t[i]).collect();
    let mut within = t.clone();
    within.sort_unstable();
    let mut cuts: Vec<Vec<usize>> = Vec::new();
    for set in violated_prefixes(ex, &order, z, CUTS_PER_CHECK) {
        let closed = closure(ex, &set, &within);
        if !cuts.contains(&closed) {
            cuts.push(closed);
        }
    }
    Check { y, cuts, pivots }
}

/// A maximal independent subset of `set` in an extra matroid (its members in order, each kept if independent of
/// those kept), ascending.
fn independent_part(ex: &Extra, set: &[usize]) -> Vec<usize> {
    let mut basis = Basis3::new(ex.m.rank);
    set.iter().copied().filter(|&e| ex.position(e).is_some_and(|i| basis.try_insert(ex.m.vecs[i].clone()))).collect()
}

/// A uniform number in `[0, 1)` from a xorshift state.
fn uniform(rng: &mut u64) -> f64 {
    *rng ^= *rng << 13;
    *rng ^= *rng >> 7;
    *rng ^= *rng << 17;
    (*rng >> 11) as f64 / (1u64 << 53) as f64
}

// ---------------------------------------------------------------------------------------------------------
// The master

/// A column of the master: its LP column and its set (GF(2) elements, ascending).
struct Col {
    lp: usize,
    set: Vec<usize>,
}

/// A rank inequality `z(S) ≤ r_k(S)` of extra matroid `k` in the master: its set (GF(2) elements, ascending), rank
/// and row.
struct Cut {
    k: usize,
    set: Vec<usize>,
    rank: usize,
    row: usize,
}

/// The master of a family (see the module docs): the `α` columns, a convexity row and the rank inequalities found.
struct Master {
    lp: Lp,
    conv: usize,
    mi: Vec<Col>,
    seen_mi: HashSet<Vec<usize>>,
    cuts: Vec<Cut>,
    /// The lowest rank of each set among the inequalities added (a set's rows differ only in their ranks, so only
    /// one lower than those there is added: identical rows would make bases singular).
    seen_cuts: HashMap<Vec<usize>, usize>,
    /// The perturbations' generator.
    rng: u64,
}

impl Master {
    /// The master of `fam` with the columns `mi` and the rank inequalities `cuts` (their ranks in `fam`), at the
    /// basis of the empty set.
    fn new<const W: usize>(ctx: &Ctx<'_, W>, fam: &Family, mi: &[Vec<usize>], cuts: &[(usize, Vec<usize>)]) -> Self {
        let mut lp = Lp::new();
        let conv = lp.add_row(RowKind::Eq, 1.0, &[]);
        let empty = lp.add_column(0.0, &[(conv, 1.0)]);
        lp.reset(&[empty]);
        let mut m = Master {
            lp,
            conv,
            mi: Vec::new(),
            seen_mi: HashSet::from([Vec::new()]),
            cuts: Vec::new(),
            seen_cuts: HashMap::new(),
            rng: u64::from(ctx.n).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
        };
        for (k, set) in cuts {
            m.add_cut(fam, *k, set.clone());
        }
        for set in mi {
            m.add_mi(ctx, set.clone());
        }
        m
    }

    /// Adds the column of `set`; `None` if it is there.
    fn add_mi<const W: usize>(&mut self, ctx: &Ctx<'_, W>, set: Vec<usize>) -> Option<usize> {
        if !self.seen_mi.insert(set.clone()) {
            return None;
        }
        let mut entries = vec![(self.conv, 1.0)];
        for c in &self.cuts {
            let common = set.iter().filter(|e| c.set.binary_search(e).is_ok()).count();
            if common > 0 {
                entries.push((c.row, common as f64));
            }
        }
        let cost: f64 = set.iter().map(|&e| f64::from(ctx.fm.elems[e])).sum();
        let j = self.lp.add_column(cost, &entries);
        self.mi.push(Col { lp: j, set });
        Some(j)
    }

    /// Adds the rank inequality of `set` in extra matroid `k` (the current point may violate it: the next solve starts
    /// with the dual simplex); false if the set is there with a rank as low.
    fn add_cut(&mut self, fam: &Family, k: usize, set: Vec<usize>) -> bool {
        let r = rank(&fam.extras[k], &set);
        if self.seen_cuts.get(&set).is_some_and(|&old| old <= r) {
            return false;
        }
        self.seen_cuts.insert(set.clone(), r);
        let entries: Vec<(usize, f64)> = self
            .mi
            .iter()
            .filter_map(|c| {
                let common = c.set.iter().filter(|e| set.binary_search(e).is_ok()).count();
                (common > 0).then_some((c.lp, common as f64))
            })
            .collect();
        let rhs = r as f64 + PERTURB * uniform(&mut self.rng);
        let (row, _) = self.lp.add_row_elastic(rhs, &entries, PENALTY);
        self.cuts.push(Cut { k, set, rank: r, row });
        true
    }

    /// The master's multipliers, one vector per extra matroid indexed like its elements: `λ_{k,x}` is the sum of
    /// the duals of matroid `k`'s inequalities whose sets hold `x`.
    fn multipliers(&self, fam: &Family) -> Vec<Vec<f64>> {
        let mut lam = fam.zeros();
        let y = self.lp.duals();
        for c in &self.cuts {
            let mu = y[c.row].max(0.0);
            if mu > 0.0 {
                for &e in &c.set {
                    if let Some(i) = fam.extras[c.k].position(e) {
                        lam[c.k][i] += mu;
                    }
                }
            }
        }
        lam
    }

    /// The reduced cost at the master's duals of the column of `set`.
    fn reduced_cost<const W: usize>(&self, ctx: &Ctx<'_, W>, set: &[usize]) -> f64 {
        let y = self.lp.duals();
        let cost: f64 = set.iter().map(|&e| f64::from(ctx.fm.elems[e])).sum();
        let used: f64 = self
            .cuts
            .iter()
            .filter(|c| y[c.row] > 0.0)
            .map(|c| y[c.row] * set.iter().filter(|e| c.set.binary_search(e).is_ok()).count() as f64)
            .sum();
        cost - y[self.conv] - used
    }

    /// Kills the nonbasic columns but the `keep` of highest reduced cost, so that pricing the master's columns
    /// stays cheap; a killed set can come back by pricing.  How many were killed.
    fn purge<const W: usize>(&mut self, ctx: &Ctx<'_, W>, keep: usize) -> usize {
        let mut nonbasic: Vec<(f64, usize)> = (0..self.mi.len())
            .filter(|&i| !self.lp.is_basic(self.mi[i].lp))
            .map(|i| (self.reduced_cost(ctx, &self.mi[i].set), i))
            .collect();
        if nonbasic.len() <= keep {
            return 0;
        }
        nonbasic.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut dead = vec![false; self.mi.len()];
        for &(_, i) in &nonbasic[keep..] {
            dead[i] = true;
            self.lp.kill(self.mi[i].lp);
            self.seen_mi.remove(&self.mi[i].set);
        }
        let mut i = 0;
        self.mi.retain(|_| {
            i += 1;
            !dead[i - 1]
        });
        nonbasic.len() - keep
    }

    /// The master's point: each GF(2) element's value `Σ_j α_j [x ∈ Q_j]`.
    fn point<const W: usize>(&self, ctx: &Ctx<'_, W>) -> Vec<f64> {
        let mut z = vec![0.0; ctx.len()];
        for c in &self.mi {
            let a = self.lp.primal(c.lp);
            if a > 0.0 {
                for &e in &c.set {
                    z[e] += a;
                }
            }
        }
        z
    }

    /// Solves the master within the deadline; whether it is optimal.
    fn solve(&mut self, deadline: Instant, stats: &mut Stats) -> bool {
        let t = Instant::now();
        let before = self.lp.stats();
        let optimal = loop {
            if self.lp.solve(500) == Status::Optimal {
                break true;
            }
            if Instant::now() >= deadline {
                break false;
            }
        };
        let after = self.lp.stats();
        stats.pivots += after.pivots - before.pivots;
        stats.degenerate += after.degenerate - before.degenerate;
        stats.repairs += after.repairs - before.repairs;
        stats.solves += 1;
        stats.ms_simplex += t.elapsed().as_secs_f64() * 1e3;
        stats.max_rows = stats.max_rows.max(self.lp.rows());
        stats.max_s = stats.max_s.max(after.max_s);
        optimal
    }

    /// The columns in the support, heaviest value first: (value, set).
    fn support(&self) -> Vec<(f64, Vec<usize>)> {
        let mut out: Vec<(f64, Vec<usize>)> = self
            .mi
            .iter()
            .map(|c| (self.lp.primal(c.lp), c.set.clone()))
            .filter(|(a, s)| *a > SUPPORT && !s.is_empty())
            .collect();
        out.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        out
    }
}

// ---------------------------------------------------------------------------------------------------------
// The column generation, with separation

/// A core found by the separation, and the index of the final signing it was assigned to (if any).
pub struct Found {
    pub core: Core,
    pub assigned: Option<usize>,
}

/// Why a computation stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// The bracket closed (certified) and separation (if on) found nothing new.
    Converged,
    /// The bound reached the target (a node, cut).
    Target,
    Seconds,
    Pricings,
    Rounds,
}

/// What a bound computation found.
pub struct Outcome {
    /// `⌊L⌋` at the lowest `L` evaluated, over every family the computation went through: a valid bound.
    pub bound: i64,
    /// That lowest `L`, scaled by [`SCALE`].
    pub best_scaled: i64,
    /// The bracket of the final family's LP: its lowest `L`, and the last certified lower bound (the weight of a
    /// point of the LP's polytope; 0 if none was certified).
    pub l_best: f64,
    pub lb: f64,
    /// The final master's value (an estimate of the LP, not certified).
    pub master: f64,
    /// The final family's signings and matroids, and the multipliers of its lowest `L` (by candidate number).
    pub family: Family,
    pub lambda: Vec<HashMap<u32, f64>>,
    /// Every distinct core the separation found, in the order found.
    pub cores: Vec<Found>,
    pub cores_assigned: usize,
    /// Distinct cores that no signing could take (the frame's signings full and none compatible).
    pub cores_unplaced: usize,
    /// Cores found again after being assigned (skipped).
    pub cores_repeated: usize,
    pub rounds: usize,
    pub stop: Stop,
    /// The master's support at the end: (value, set as candidate numbers).
    pub support: Vec<(f64, Vec<u32>)>,
    pub columns: usize,
    pub cuts: usize,
    pub stats: Stats,
    pub seconds: f64,
}

/// The bound at a state with the extra matroids of `signings` (and, with `separate`, the core-derived signings the
/// separation adds), the multipliers started at `start` (per signing, by candidate number; zero elsewhere).
pub fn bound<const W: usize>(
    ctx: &Ctx<'_, W>,
    signings: Vec<Signing>,
    start: Option<&[HashMap<u32, f64>]>,
    separate: bool,
    settings: &Settings,
) -> Outcome {
    let t0 = Instant::now();
    let deadline = t0 + std::time::Duration::from_secs_f64(settings.seconds);
    let mut stats = Stats::default();
    let mut fam = Family::new(ctx, signings);
    let check_every = settings.check_every;

    // The multipliers: the start, by candidate number, or zero.
    let mut lam = fam.zeros();
    if let Some(start) = start {
        for ((l, ex), st) in lam.iter_mut().zip(&fam.extras).zip(start) {
            for (v, &x) in l.iter_mut().zip(&ex.m.elems) {
                *v = st.get(&x).copied().unwrap_or(0.0);
            }
        }
    }

    // The lowest L of all, and of the current family with its multipliers.
    let mut best_scaled = i64::MAX;
    let mut fam_best = i64::MAX;
    let mut lam_best = lam.clone();
    let mut stop = None;
    let reached = |v: i64| ctx.stop_at_target && v.div_euclid(SCALE) <= ctx.target;

    // Polyak steps towards the target, every intersection they produce kept as a column.
    let mut mi_cols: Vec<Vec<usize>> = Vec::new();
    let steps = if fam.extras.is_empty() { 1 } else { settings.polyak.max(1) };
    let (mut theta, mut stall) = (1.0f64, 0usize);
    for _ in 0..steps {
        let ev = evaluate(ctx, &fam, &lam, &mut stats, check_every);
        best_scaled = best_scaled.min(ev.value);
        if ev.value < fam_best {
            fam_best = ev.value;
            lam_best.clone_from(&lam);
            stall = 0;
        } else {
            stall += 1;
            if stall >= 10 {
                theta /= 2.0;
                stall = 0;
            }
        }
        mi_cols.push(ev.q.clone());
        for (ex, set) in fam.extras.iter_mut().zip(&ev.sets) {
            ex.remember(set);
        }
        if reached(best_scaled) {
            stop = Some(Stop::Target);
            break;
        }
        // The subgradient g_{k,x} = [x ∈ I_k] − [x ∈ Q]; a step lowers λ where g is positive.
        let mut g: Vec<Vec<f64>> = fam.zeros();
        let mut norm = 0.0;
        for (k, ex) in fam.extras.iter().enumerate() {
            for &e in &ev.sets[k] {
                g[k][ex.position(e).expect("an element")] += 1.0;
            }
            for &e in &ev.q {
                if let Some(i) = ex.position(e) {
                    g[k][i] -= 1.0;
                }
            }
            norm += g[k].iter().map(|v| v * v).sum::<f64>();
        }
        let excess = ev.value as f64 / SCALE as f64 - ctx.target as f64;
        if norm == 0.0 || excess <= 0.0 {
            break;
        }
        let step = theta * excess / norm;
        for (l, g) in lam.iter_mut().zip(&g) {
            for (v, g) in l.iter_mut().zip(g) {
                *v = (*v - step * g).max(0.0);
            }
        }
    }

    let mut master = Master::new(ctx, &fam, &mi_cols, &[]);
    stats.masters += 1;
    let mut found: Vec<Found> = Vec::new();
    let mut known: HashMap<Core, usize> = HashMap::new();
    let (mut repeated, mut rounds) = (0, 0);
    let mut lb = 0.0;
    let mut rng = u64::from(ctx.n).wrapping_mul(0xD1B5_4A32_D192_ED03) | 1;

    // The bracket aimed at: `close`, or a share of the root's GF(2) gap if larger (the study counts a root whose
    // bracket is within 1% of its gap as converged).
    let close = ctx.gap.map_or(settings.close, |g| settings.close.max(settings.close_gap * g as f64));
    // Whether the current family's point is to be certified: from the start without separation, else once a
    // separation round has found no new core (and a certified family whose separation finds none is the end).
    let mut certify = !separate;
    let mut certified = false;
    'rounds: while stop.is_none() {
        // Column and cut generation for the current family, until no column prices out and no cut is found (with
        // the exact checks when certifying).
        loop {
            if Instant::now() >= deadline {
                stop = Some(Stop::Seconds);
                break 'rounds;
            }
            if stats.pricings_mi as usize >= settings.pricings {
                stop = Some(Stop::Pricings);
                break 'rounds;
            }
            if !master.solve(deadline, &mut stats) {
                stop = Some(Stop::Seconds);
                break 'rounds;
            }
            if master.mi.len() > 2 * KEEP {
                stats.purged += master.purge(ctx, KEEP) as u64;
            }
            let value = master.lp.value();
            let mut added = 0;
            if fam_best as f64 / SCALE as f64 - value > close {
                let lam_master = master.multipliers(&fam);
                let smoothed: Vec<Vec<f64>> = lam_best
                    .iter()
                    .zip(&lam_master)
                    .map(|(b, m)| b.iter().zip(m).map(|(b, m)| 0.5 * b + 0.5 * m).collect())
                    .collect();
                for at in [smoothed, lam_master] {
                    let ev = evaluate(ctx, &fam, &at, &mut stats, check_every);
                    for (ex, set) in fam.extras.iter_mut().zip(&ev.sets) {
                        ex.remember(set);
                    }
                    best_scaled = best_scaled.min(ev.value);
                    if ev.value < fam_best {
                        fam_best = ev.value;
                        lam_best = at;
                    }
                    if reached(best_scaled) {
                        stop = Some(Stop::Target);
                        break 'rounds;
                    }
                    if master.reduced_cost(ctx, &ev.q) > ENTER && master.add_mi(ctx, ev.q).is_some() {
                        added += 1;
                        break;
                    }
                }
            }
            if added > 0 {
                continue;
            }
            // No column prices out (or the bracket against the master is closed): the master's point must lie in
            // every extra matroid's polytope.  Cheap cuts first, then the exact check.
            let z = master.point(ctx);
            let support = master.support();
            let mut new_cuts = 0;
            for (k, set) in violated_cuts(&fam, &z, &support) {
                new_cuts += usize::from(master.add_cut(&fam, k, set));
            }
            stats.cuts_cheap += new_cuts as u64;
            if new_cuts > 0 {
                continue;
            }
            if !certify {
                break;
            }
            let mut zhat = z.clone();
            let mut found_cuts = Vec::new();
            for (k, ex) in fam.extras.iter_mut().enumerate() {
                let c = check(ex, &z, &support, deadline, &mut rng);
                stats.checks_exact += 1;
                stats.pivots_exact += c.pivots;
                for (e, y) in c.y {
                    zhat[e] = zhat[e].min(y);
                }
                found_cuts.extend(c.cuts.into_iter().map(|set| (k, set)));
                if Instant::now() >= deadline {
                    stop = Some(Stop::Seconds);
                    break 'rounds;
                }
            }
            for (k, set) in found_cuts {
                new_cuts += usize::from(master.add_cut(&fam, k, set));
            }
            stats.cuts_exact += new_cuts as u64;
            if new_cuts > 0 {
                continue;
            }
            // The point less what the checks could not certify lies in every polytope: a certified lower bound.
            lb = zhat.iter().zip(&ctx.fm.elems).map(|(z, &x)| z * f64::from(x)).sum();
            certified = true;
            break;
        }
        if !separate {
            break;
        }
        if rounds >= settings.rounds {
            stop = Some(Stop::Rounds);
            break;
        }
        // Separation: the cores of the support's sets, merged into the core-derived signings.
        rounds += 1;
        let mut changed = vec![false; fam.extras.len()];
        let mut opened: Vec<Signing> = Vec::new();
        let mut new = 0;
        for (_, set) in master.support() {
            let q: Vec<u32> = set.iter().map(|&e| ctx.fm.elems[e]).collect();
            for core in cores_of(ctx.model, ctx.state, &q, settings.cores_per_frame) {
                if let Some(&id) = known.get(&core) {
                    if found[id].assigned.is_some() {
                        repeated += 1;
                        continue;
                    }
                }
                let id = *known.entry(core.clone()).or_insert_with(|| {
                    found.push(Found { core: core.clone(), assigned: None });
                    found.len() - 1
                });
                if let Some(k) = place(ctx, &mut fam, &mut opened, &found, id, settings) {
                    if k < changed.len() {
                        changed[k] = true;
                    }
                    found[id].assigned = Some(k);
                    new += 1;
                }
            }
        }
        if new == 0 {
            if certified {
                break;
            }
            certify = true;
            continue;
        }
        // The family changed: rebuild the changed matroids and add the new ones; keep the columns (but those with a
        // new loop) and the rank inequalities (their ranks taken again in the changed matroids).
        let old_lam_best = std::mem::take(&mut lam_best);
        for (k, ex) in fam.extras.iter_mut().enumerate() {
            if changed[k] {
                let by_elem: HashMap<usize, f64> =
                    ex.elem.iter().copied().zip(old_lam_best[k].iter().copied()).collect();
                let signing = std::mem::replace(&mut ex.signing, Signing::ones(0));
                *ex = Extra::new(ctx, signing);
                lam_best.push(ex.elem.iter().map(|e| by_elem.get(e).copied().unwrap_or(0.0)).collect());
            } else {
                lam_best.push(old_lam_best[k].clone());
            }
        }
        for signing in opened {
            let ex = Extra::new(ctx, signing);
            lam_best.push(vec![0.0; ex.elem.len()]);
            fam.extras.push(ex);
        }
        fam.ban(ctx);
        let keep_mi: Vec<Vec<usize>> =
            master.mi.iter().map(|c| c.set.clone()).filter(|s| s.iter().all(|&e| !fam.banned[e])).collect();
        let keep_cuts: Vec<(usize, Vec<usize>)> = master.cuts.iter().map(|c| (c.k, c.set.clone())).collect();
        master = Master::new(ctx, &fam, &keep_mi, &keep_cuts);
        stats.masters += 1;
        // The lowest L of the new family starts at the old multipliers.
        let ev = evaluate(ctx, &fam, &lam_best, &mut stats, check_every);
        best_scaled = best_scaled.min(ev.value);
        fam_best = ev.value;
        master.add_mi(ctx, ev.q);
        lb = 0.0;
        (certify, certified) = (false, false);
        if reached(best_scaled) {
            stop = Some(Stop::Target);
        }
    }
    let stop = stop.unwrap_or(Stop::Converged);
    let lambda = fam
        .extras
        .iter()
        .zip(&lam_best)
        .map(|(ex, l)| ex.m.elems.iter().copied().zip(l.iter().copied()).filter(|&(_, v)| v > 0.0).collect())
        .collect();
    let support =
        master.support().into_iter().map(|(a, s)| (a, s.iter().map(|&e| ctx.fm.elems[e]).collect())).collect();
    Outcome {
        bound: best_scaled.div_euclid(SCALE),
        best_scaled,
        l_best: fam_best as f64 / SCALE as f64,
        lb,
        master: master.lp.value(),
        family: fam,
        lambda,
        cores_assigned: found.iter().filter(|f| f.assigned.is_some()).count(),
        cores_unplaced: found.iter().filter(|f| f.assigned.is_none()).count(),
        cores: found,
        cores_repeated: repeated,
        rounds,
        stop,
        support,
        columns: master.lp.columns(),
        cuts: master.cuts.len(),
        stats,
        seconds: t0.elapsed().as_secs_f64(),
    }
}

/// Puts the core `found[id]` into a core-derived signing of its frame: the first of the family's (then of those
/// opened this round) that `merge` accepts, else a new one if the frame has fewer than `per_frame`.  Returns the
/// signing's index in the family (an index past the family's for one opened this round), or `None`.
fn place<const W: usize>(
    ctx: &Ctx<'_, W>,
    fam: &mut Family,
    opened: &mut Vec<Signing>,
    found: &[Found],
    id: usize,
    settings: &Settings,
) -> Option<usize> {
    let core = &found[id].core;
    let h = core.frame;
    let existing = fam.extras.len();
    let signings = fam.extras.iter_mut().map(|ex| &mut ex.signing).chain(opened.iter_mut());
    let mut count = 0;
    for (k, s) in signings.enumerate() {
        if !s.mergeable || s.frame != h {
            continue;
        }
        count += 1;
        let others: Vec<&Core> = s.cores.iter().map(|&c| &found[c].core).collect();
        if merge(s, core, &others) {
            s.cores.push(id);
            return Some(k);
        }
    }
    if count >= settings.per_frame {
        return None;
    }
    let default =
        if settings.random_defaults { Default3::Random(seed(ctx.n, h, count, SALT_CORES)) } else { Default3::Ones };
    let mut s = Signing::for_cores(h, default);
    assert!(merge(&mut s, core, &[]), "a core always merges into a signing of its own");
    s.cores.push(id);
    opened.push(s);
    Some(existing + opened.len() - 1)
}
