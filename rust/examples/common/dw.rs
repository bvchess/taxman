//! The fields study's bound and its multipliers: one GF(2) matroid intersection plus a greedy per signed GF(3) frame
//! matroid, coupled by Lagrangian multipliers that Dantzig–Wolfe column generation fits.  Research code for
//! `examples/fields.rs` (`#[path = "common/dw.rs"] mod dw;`, beside `meet`, `frames`, `gf3`, `cores` and `lp`).
//!
//! # The bound
//!
//! Let `A`, `B` be the two GF(2) frame matroids of a state (`frames::FrameMatroids`) and `N_1, …, N_K` signed GF(3)
//! frame matroids (`gf3::Matroid3`, each of one frame `h_k` and one signing, on the candidates touching that frame,
//! `supp_k`).  Every promotable set `Q` is independent in `A` and `B` (Theorem 11) and has `Q ∩ supp_k` independent
//! in every `N_k` (THEORY.md, Theorem 14: any field, any nonzero entries).  For any multipliers `λ_k ≥ 0` on
//! `supp_k`,
//!
//! ```text
//! w(Q) = (w − Σ_k λ_k)(Q) + Σ_k λ_k(Q ∩ supp_k) ≤ MI_cap(w − Σ_k λ_k) + Σ_k G_k(λ_k) = L(λ)     (Theorem 15)
//! ```
//!
//! with `MI_cap(c)` the heaviest common independent set of `A` and `B` of at most `cap` elements (elements of weight
//! at most 0 left out, which loses nothing over a down-closed family) and `G_k(μ)` the heaviest independent set of
//! `N_k` (greedy).  A candidate that is a loop of some `N_k` is in no promotable set and is left out of the
//! intersection (it is `L` at `λ_{k,x} = w_x`, so Theorem 15 covers it).  The bound reported is `⌊L⌋` evaluated exactly
//! in integers: weights `x·S` and multipliers rounded to the grid `1/S` (`S = 2^16`, [`SCALE`]), the intersection by
//! the crate's `taxman::gf2::meet::Meet` (checked against `common/meet.rs`'s `intersect` every `check_every`-th call)
//! and the greedies over GF(3), all in `i64`.  Its soundness rests on Theorem 15 and the two exact oracles, never on
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
//!
//! In [`FitMode::Search`] the exact checks are never run: a family's generation ends when nothing prices out and no
//! cheap inequality is violated, a separation round that places no core ends the computation, and `LB` stays 0.
//!
//! # Warm states and the node methods
//!
//! A fitted state ([`Warm`]: frozen signings, multipliers, the master's columns and inequalities, all by candidate
//! number) passes from a node to its children ([`Warm::child`]) and from a root fit to a file ([`Fit`]).  At a node,
//! [`tier1`] evaluates `L` at the inherited multipliers; [`refit_r1`] takes deflected Polyak steps from them, and
//! [`refit_r2`] runs the column generation of [`bound`] from the inherited master in search mode, with one separation
//! round for node-local signings.  Each returns the lowest `⌊L⌋` it evaluated, which is valid at any multipliers and
//! signings (Theorems 14 and 15, at a node by (2)); the master and the steps only propose multipliers.

use crate::cores::{cores_of, merge, Core};
use crate::frames::FrameMatroids;
use crate::gf3::{seed, Basis3, Default3, Matroid3, Signing};
use crate::lp::{Lp, RowKind, Status};
use crate::meet::intersect;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use taxman::gf2::meet::{Meet, Problem};
use taxman::json::Json;
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
const PENALTY: f64 = 1e5;
/// Pivots after which a master solve is taken to stall.
const STALL: u64 = 50_000;
/// The most columns an exact check generates (then its point and cuts are what it has).
const PACK_COLUMNS: usize = 300;
/// The most violated rank inequalities an exact check returns.
const CUTS_PER_CHECK: usize = 5;
/// The nonbasic columns the master keeps when it purges, which it does when it has twice as many.
const KEEP: usize = 300;
/// The independent sets an extra matroid's pool keeps.
const POOL: usize = 400;
/// The largest perturbation of a rank inequality's right-hand side in the master, and of a bound `y ≤ z` in the exact
/// check, against degeneracy (uniform in `[0, PERTURB]`).
const PERTURB: f64 = 1e-6;
/// The salt of the seeds of the core-derived signings' default values.
pub const SALT_CORES: u64 = 2;
/// The default of [`Settings::check_every`], and the cross-check of the evaluations of [`tier1`] and [`refit_r1`].
pub const CHECK_EVERY: u64 = 50;
/// The most master columns an [`Outcome`] (so a [`Warm`] state) keeps.
pub const WARM_COLUMNS: usize = 300;
/// The most node-local signings of a frame along a path (design S3.3).
pub const LOCAL_PER_FRAME: usize = 8;
/// The most cores a node's separation takes from one frame of one column (design S3.3).
pub const LOCAL_CORES_PER_FRAME: usize = 2;
/// Where the indices of the seeds of node-local signings' defaults start: past those of a root's core-derived
/// signings at the default cap of 8 a frame.
pub const LOCAL_SEEDS: usize = 8;
/// No row, no position.
const NONE: u32 = u32::MAX;

// ---------------------------------------------------------------------------------------------------------
// The setting

/// One bound computation's state: the frames, the GF(2) matroids, the ceiling and the target.
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

/// What a computation is for: the study's measurement, with a certified lower bound on the LP (the exact checks),
/// or a search, which needs only the bound (design S3.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitMode {
    Certified,
    Search,
}

/// The budgets and knobs of a bound computation.
#[derive(Clone, Debug)]
pub struct Settings {
    pub mode: FitMode,
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
            mode: FitMode::Certified,
            seconds: 120.0,
            pricings: 3000,
            rounds: 40,
            per_frame: 8,
            cores_per_frame: 4,
            random_defaults: true,
            polyak: 100,
            check_every: CHECK_EVERY,
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
    /// Master solves that stalled and started again from the empty set.
    pub restarts: u64,
    /// Exact checks stopped at their column limit.
    pub checks_capped: u64,
    /// Rank inequalities added by the cheap separation and by the exact checks, the checks made and their pivots.
    pub cuts_cheap: u64,
    pub cuts_exact: u64,
    pub checks_exact: u64,
    pub pivots_exact: u64,
    pub masters: u64,
    pub max_rows: usize,
    pub max_s: usize,
    pub ms_mi: f64,
    /// Time in the greedies of the evaluations.
    pub ms_greedy: f64,
    pub ms_simplex: f64,
}

impl Stats {
    /// Adds `other`'s counts and times (the largest of the maxima).
    pub fn add(&mut self, other: &Stats) {
        let Stats {
            pricings_mi,
            pricings_greedy,
            checks,
            pivots,
            degenerate,
            solves,
            repairs,
            purged,
            restarts,
            checks_capped,
            cuts_cheap,
            cuts_exact,
            checks_exact,
            pivots_exact,
            masters,
            max_rows,
            max_s,
            ms_mi,
            ms_greedy,
            ms_simplex,
        } = *other;
        self.pricings_mi += pricings_mi;
        self.pricings_greedy += pricings_greedy;
        self.checks += checks;
        self.pivots += pivots;
        self.degenerate += degenerate;
        self.solves += solves;
        self.repairs += repairs;
        self.purged += purged;
        self.restarts += restarts;
        self.checks_capped += checks_capped;
        self.cuts_cheap += cuts_cheap;
        self.cuts_exact += cuts_exact;
        self.checks_exact += checks_exact;
        self.pivots_exact += pivots_exact;
        self.masters += masters;
        self.max_rows = self.max_rows.max(max_rows);
        self.max_s = self.max_s.max(max_s);
        self.ms_mi += ms_mi;
        self.ms_greedy += ms_greedy;
        self.ms_simplex += ms_simplex;
    }
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
    let t = Instant::now();
    for (ex, l) in fam.extras.iter().zip(lhat) {
        let (g, kept) = ex.m.greedy(l);
        value += g;
        sets.push(kept.into_iter().map(|i| ex.elem[i]).collect::<Vec<_>>());
        stats.pricings_greedy += 1;
    }
    stats.ms_greedy += t.elapsed().as_secs_f64() * 1e3;
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

/// The rank in an extra matroid of its elements at the positions `positions`.
fn rank_at(ex: &Extra, positions: impl Iterator<Item = usize>) -> usize {
    let mut basis = Basis3::new(ex.m.rank);
    positions.filter(|&i| basis.try_insert(ex.m.vecs[i].clone())).count()
}

/// The rank in an extra matroid of the GF(2) elements `set` (its loops and the elements it does not have count 0).
fn rank(ex: &Extra, set: &[usize]) -> usize {
    rank_at(ex, set.iter().filter_map(|&e| ex.position(e)))
}

/// The rank in `fam`'s extra matroid `k` of the candidates `set` (members that are not its elements count 0).
pub fn cut_rank(fam: &Family, k: usize, set: &[u32]) -> usize {
    let ex = &fam.extras[k];
    rank_at(ex, set.iter().filter_map(|&x| ex.m.index(x)))
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
    /// Whether the column generation stopped at [`PACK_COLUMNS`].
    capped: bool,
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
        return Check { y: t.iter().map(|&e| (e, z[e])).collect(), cuts: Vec::new(), pivots: 0, capped: false };
    }
    if parts.iter().all(|p| rank(ex, p) == p.len()) {
        let mut y: HashMap<usize, f64> = HashMap::new();
        for ((a, _), part) in support.iter().zip(&parts) {
            for &e in part {
                *y.entry(e).or_default() += a;
            }
        }
        let y = t.iter().map(|&e| (e, y.get(&e).copied().unwrap_or(0.0).min(z[e]))).collect();
        return Check { y, cuts: Vec::new(), pivots: 0, capped: false };
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
    let (mut generated, mut capped) = (0, false);
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
        generated += 1;
        if generated >= PACK_COLUMNS {
            capped = true;
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
    // The pool keeps the decomposition found and the latest columns: a check of a similar point starts from them.
    let used: Vec<bool> = cols.iter().map(|(j, _)| lp.primal(*j) > 0.0).collect();
    let latest = cols.len().saturating_sub(32);
    for (i, (_, set)) in cols.into_iter().enumerate() {
        if used[i] || i >= latest {
            ex.remember(&set);
        }
    }
    if y[..t.len()].iter().map(|&(_, v)| v).sum::<f64>() >= total - VIOLATED {
        return Check { y, cuts: Vec::new(), pivots, capped };
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
    Check { y, cuts, pivots, capped }
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
    /// The empty set's column: the master's starting basis, returned to when a solve stalls.
    empty: usize,
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
            empty,
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
        let mut since = 0;
        let optimal = loop {
            if self.lp.solve(500) == Status::Optimal {
                break true;
            }
            if Instant::now() >= deadline {
                break false;
            }
            // A solve that runs on is cycling in rounding (the objective does not move): start again from the empty
            // set, which every inequality admits.
            since += 500;
            if since >= STALL {
                self.lp.reset(&[self.empty]);
                stats.restarts += 1;
                since = 0;
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
    /// The bracket closed (certified; in search mode, nothing priced out and no cheap cut was violated) and
    /// separation (if on) found nothing new.
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
    /// The final master's columns as candidate numbers (ascending each): the support first, by decreasing value,
    /// then its other nonempty columns in its order; at most [`WARM_COLUMNS`].
    pub master_columns: Vec<Vec<u32>>,
    /// The final master's rank inequalities: (extra matroid, set as candidate numbers ascending, rank).
    pub master_cuts: Vec<(usize, Vec<u32>, usize)>,
    pub columns: usize,
    pub cuts: usize,
    pub stats: Stats,
    pub seconds: f64,
}

/// The multipliers of `maps` (per extra matroid, by candidate number; zero where absent, and for the extras past
/// them), one vector per extra matroid of `fam`, indexed like its elements.
fn by_number(fam: &Family, maps: &[HashMap<u32, f64>]) -> Vec<Vec<f64>> {
    fam.extras
        .iter()
        .enumerate()
        .map(|(k, ex)| ex.m.elems.iter().map(|x| maps.get(k).and_then(|m| m.get(x)).copied().unwrap_or(0.0)).collect())
        .collect()
}

/// The positive multipliers of `lam` (one vector per extra matroid of `fam`) by candidate number.
fn to_numbers(fam: &Family, lam: &[Vec<f64>]) -> Vec<HashMap<u32, f64>> {
    fam.extras
        .iter()
        .zip(lam)
        .map(|(ex, l)| ex.m.elems.iter().copied().zip(l.iter().copied()).filter(|&(_, v)| v > 0.0).collect())
        .collect()
}

/// The subgradient of `L` at the evaluation `ev`, `g_{k,x} = [x ∈ I_k] − [x ∈ Q]` (one vector per extra matroid,
/// indexed like its elements), and its squared norm.
fn subgradient(fam: &Family, ev: &Eval) -> (Vec<Vec<f64>>, f64) {
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
    (g, norm)
}

/// `λ ← max(0, λ − step · d)`.
fn descend(lam: &mut [Vec<f64>], step: f64, d: &[Vec<f64>]) {
    for (l, d) in lam.iter_mut().zip(d) {
        for (v, d) in l.iter_mut().zip(d) {
            *v = (*v - step * d).max(0.0);
        }
    }
}

/// The inner product of two multiplier-shaped vectors.
fn dot(a: &[Vec<f64>], b: &[Vec<f64>]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a.iter().zip(b).map(|(a, b)| a * b).sum::<f64>()).sum()
}

/// A signing as a search keeps it: nothing merges into it (`mergeable` false, no cores), so its matroid stays.
pub fn frozen(s: &Signing) -> Signing {
    Signing { mergeable: false, cores: Vec::new(), ..s.clone() }
}

/// One computation's state, which [`bound`] and [`refit_r2`] drive through the same phases: the family and its
/// master, the lowest `L` of all and of the family with its multipliers, the cores found, the budgets, the stop.
struct Fitter<'r, 'a, const W: usize> {
    ctx: &'r Ctx<'a, W>,
    settings: &'r Settings,
    stats: &'r mut Stats,
    fam: Family,
    master: Master,
    /// The lowest `L` of all (scaled), of the current family, and the family's multipliers there.
    best_scaled: i64,
    fam_best: i64,
    lam_best: Vec<Vec<f64>>,
    found: Vec<Found>,
    known: HashMap<Core, usize>,
    repeated: usize,
    rounds: usize,
    /// The current family's last certified lower bound (0 if none).
    lb: f64,
    /// The exact checks' perturbations' generator.
    rng: u64,
    /// The bracket aimed at: `close`, or a share of the root's GF(2) gap if larger (the study counts a root whose
    /// bracket is within 1% of its gap as converged).
    close: f64,
    t0: Instant,
    deadline: Instant,
    /// The pricings allowed, counted from `stats.pricings_mi` at `from`.
    budget: usize,
    from: u64,
    stop: Option<Stop>,
    /// The lowest `⌊L⌋` after each evaluation, if kept (a refit's trace).
    trace: Option<Vec<i64>>,
}

impl<'r, 'a, const W: usize> Fitter<'r, 'a, W> {
    /// A computation over `fam` from the multipliers `start` (indexed like each extra's elements), with a master of
    /// the columns `columns` and the rank inequalities `cuts` (GF(2) elements; ranks taken in `fam`), started at
    /// `t0`.  Its pricing budget, `settings.pricings`, counts from here.
    #[allow(clippy::too_many_arguments)]
    fn new(
        ctx: &'r Ctx<'a, W>,
        fam: Family,
        start: Vec<Vec<f64>>,
        columns: &[Vec<usize>],
        cuts: &[(usize, Vec<usize>)],
        settings: &'r Settings,
        stats: &'r mut Stats,
        t0: Instant,
    ) -> Self {
        let master = Master::new(ctx, &fam, columns, cuts);
        stats.masters += 1;
        Fitter {
            ctx,
            settings,
            fam,
            master,
            best_scaled: i64::MAX,
            fam_best: i64::MAX,
            lam_best: start,
            found: Vec::new(),
            known: HashMap::new(),
            repeated: 0,
            rounds: 0,
            lb: 0.0,
            rng: u64::from(ctx.n).wrapping_mul(0xD1B5_4A32_D192_ED03) | 1,
            close: ctx.gap.map_or(settings.close, |g| settings.close.max(settings.close_gap * g as f64)),
            t0,
            deadline: t0 + Duration::from_secs_f64(settings.seconds),
            budget: settings.pricings,
            from: stats.pricings_mi,
            stop: None,
            trace: None,
            stats,
        }
    }

    /// The pricings made since the budget started.
    fn spent(&self) -> usize {
        (self.stats.pricings_mi - self.from) as usize
    }

    /// Starts a budget of `budget` pricings from here.
    fn restart_budget(&mut self, budget: usize) {
        self.from = self.stats.pricings_mi;
        self.budget = budget;
    }

    /// `L` at `at` for the current family (the cross-check every `check_every`-th evaluation by the cumulative count).
    fn eval_at(&mut self, at: &[Vec<f64>]) -> Eval {
        evaluate(self.ctx, &self.fam, at, self.stats, self.settings.check_every)
    }

    /// Records the value of an evaluation at `at`: the lowest `L` of all, the trace, the target (a stop when the
    /// context stops there), and the family's lowest with its multipliers.  Whether it is the family's lowest.
    fn note(&mut self, value: i64, at: &[Vec<f64>]) -> bool {
        self.best_scaled = self.best_scaled.min(value);
        if let Some(trace) = &mut self.trace {
            trace.push(self.best_scaled.div_euclid(SCALE));
        }
        if self.ctx.stop_at_target && self.best_scaled.div_euclid(SCALE) <= self.ctx.target {
            self.stop = Some(Stop::Target);
        }
        let lower = value < self.fam_best;
        if lower {
            self.fam_best = value;
            self.lam_best = at.to_vec();
        }
        lower
    }

    /// Records an evaluation at `at` ([`note`](Self::note)), its greedy sets kept in the extras' pools.
    fn record(&mut self, ev: &Eval, at: &[Vec<f64>]) -> bool {
        for (ex, set) in self.fam.extras.iter_mut().zip(&ev.sets) {
            ex.remember(set);
        }
        self.note(ev.value, at)
    }

    /// Polyak steps from the start towards the target (`settings.polyak`; one without extra matroids), every
    /// intersection they produce a column of the master.
    fn polyak(&mut self) {
        let steps = if self.fam.extras.is_empty() { 1 } else { self.settings.polyak.max(1) };
        let mut lam = self.lam_best.clone();
        let (mut theta, mut stall) = (1.0f64, 0usize);
        for _ in 0..steps {
            let ev = self.eval_at(&lam);
            if self.record(&ev, &lam) {
                stall = 0;
            } else {
                stall += 1;
                if stall >= 10 {
                    theta /= 2.0;
                    stall = 0;
                }
            }
            self.master.add_mi(self.ctx, ev.q.clone());
            if self.stop.is_some() {
                break;
            }
            // A step lowers λ where the subgradient is positive.
            let (g, norm) = subgradient(&self.fam, &ev);
            let excess = ev.value as f64 / SCALE as f64 - self.ctx.target as f64;
            if norm == 0.0 || excess <= 0.0 {
                break;
            }
            descend(&mut lam, theta * excess / norm, &g);
        }
    }

    /// The current family's column and cut generation, until no column prices out and no cut is found (with the
    /// exact checks if `certify`), or a stop (seconds, the pricing budget, the target), which is left in `stop`.
    /// Whether the family's point was certified.
    fn generate(&mut self, certify: bool) -> bool {
        let ctx = self.ctx;
        loop {
            if Instant::now() >= self.deadline {
                self.stop = Some(Stop::Seconds);
                return false;
            }
            if self.spent() >= self.budget {
                self.stop = Some(Stop::Pricings);
                return false;
            }
            if !self.master.solve(self.deadline, self.stats) {
                self.stop = Some(Stop::Seconds);
                return false;
            }
            if self.master.mi.len() > 2 * KEEP {
                self.stats.purged += self.master.purge(ctx, KEEP) as u64;
            }
            let value = self.master.lp.value();
            let mut added = false;
            if self.fam_best as f64 / SCALE as f64 - value > self.close {
                let lam_master = self.master.multipliers(&self.fam);
                let smoothed: Vec<Vec<f64>> = self
                    .lam_best
                    .iter()
                    .zip(&lam_master)
                    .map(|(b, m)| b.iter().zip(m).map(|(b, m)| 0.5 * b + 0.5 * m).collect())
                    .collect();
                for at in [smoothed, lam_master] {
                    if self.spent() >= self.budget {
                        self.stop = Some(Stop::Pricings);
                        return false;
                    }
                    let ev = self.eval_at(&at);
                    self.record(&ev, &at);
                    if self.stop.is_some() {
                        return false;
                    }
                    if self.master.reduced_cost(ctx, &ev.q) > ENTER && self.master.add_mi(ctx, ev.q).is_some() {
                        added = true;
                        break;
                    }
                }
            }
            if added {
                continue;
            }
            // No column prices out (or the bracket against the master is closed): the master's point must lie in
            // every extra matroid's polytope.  Cheap cuts first, then the exact check.
            let z = self.master.point(ctx);
            let support = self.master.support();
            let mut new_cuts = 0;
            for (k, set) in violated_cuts(&self.fam, &z, &support) {
                new_cuts += usize::from(self.master.add_cut(&self.fam, k, set));
            }
            self.stats.cuts_cheap += new_cuts as u64;
            if new_cuts > 0 {
                continue;
            }
            if !certify {
                return false;
            }
            let mut zhat = z.clone();
            let mut found_cuts = Vec::new();
            let mut capped = false;
            for (k, ex) in self.fam.extras.iter_mut().enumerate() {
                let c = check(ex, &z, &support, self.deadline, &mut self.rng);
                self.stats.checks_exact += 1;
                self.stats.pivots_exact += c.pivots;
                self.stats.checks_capped += u64::from(c.capped);
                capped |= c.capped;
                for (e, y) in c.y {
                    zhat[e] = zhat[e].min(y);
                }
                found_cuts.extend(c.cuts.into_iter().map(|set| (k, set)));
                if Instant::now() >= self.deadline {
                    self.stop = Some(Stop::Seconds);
                    return false;
                }
            }
            for (k, set) in found_cuts {
                new_cuts += usize::from(self.master.add_cut(&self.fam, k, set));
            }
            self.stats.cuts_exact += new_cuts as u64;
            if new_cuts > 0 {
                continue;
            }
            // The point less what the checks could not certify lies in every polytope: a certified lower bound.
            self.lb = zhat.iter().zip(&ctx.fm.elems).map(|(z, &x)| z * f64::from(x)).sum();
            if capped && self.fam_best as f64 / SCALE as f64 - self.lb > self.close {
                // A check stopped at its column limit left the certificate short: check again, from the pools.
                continue;
            }
            return true;
        }
    }

    /// One separation round: the cores of the master's support columns, placed in core-derived signings (at most
    /// `per_frame(h)` mergeable signings of frame `h`, those opened seeded from index `seed_offset`); if any was
    /// placed, the family and the master rebuilt.  The cores placed and the signings opened.
    fn separate(&mut self, per_frame: &dyn Fn(usize) -> usize, seed_offset: usize) -> (usize, usize) {
        let ctx = self.ctx;
        self.rounds += 1;
        let mut changed = vec![false; self.fam.extras.len()];
        let mut opened: Vec<Signing> = Vec::new();
        let mut placed = 0;
        for (_, set) in self.master.support() {
            let q: Vec<u32> = set.iter().map(|&e| ctx.fm.elems[e]).collect();
            for core in cores_of(ctx.model, ctx.state, &q, self.settings.cores_per_frame) {
                if let Some(&id) = self.known.get(&core) {
                    if self.found[id].assigned.is_some() {
                        self.repeated += 1;
                        continue;
                    }
                }
                let found = &mut self.found;
                let id = *self.known.entry(core.clone()).or_insert_with(|| {
                    found.push(Found { core: core.clone(), assigned: None });
                    found.len() - 1
                });
                if let Some(k) = self.place(&mut opened, id, per_frame, seed_offset) {
                    if k < changed.len() {
                        changed[k] = true;
                    }
                    self.found[id].assigned = Some(k);
                    placed += 1;
                }
            }
        }
        let count = opened.len();
        if placed > 0 {
            self.rebuild(&changed, opened);
        }
        (placed, count)
    }

    /// Puts the core `found[id]` into a core-derived signing of its frame: the first of the family's (then of those
    /// opened this round) that `merge` accepts, else a new one if the frame has fewer than `per_frame(h)` (its
    /// default seeded by index `seed_offset` plus that count).  Returns the signing's index in the family (an index
    /// past the family's for one opened this round), or `None`.
    fn place(
        &mut self,
        opened: &mut Vec<Signing>,
        id: usize,
        per_frame: &dyn Fn(usize) -> usize,
        seed_offset: usize,
    ) -> Option<usize> {
        let core = &self.found[id].core;
        let h = core.frame;
        let existing = self.fam.extras.len();
        let signings = self.fam.extras.iter_mut().map(|ex| &mut ex.signing).chain(opened.iter_mut());
        let mut count = 0;
        for (k, s) in signings.enumerate() {
            if !s.mergeable || s.frame != h {
                continue;
            }
            count += 1;
            let others: Vec<&Core> = s.cores.iter().map(|&c| &self.found[c].core).collect();
            if merge(s, core, &others) {
                s.cores.push(id);
                return Some(k);
            }
        }
        if count >= per_frame(h) {
            return None;
        }
        let default = if self.settings.random_defaults {
            Default3::Random(seed(self.ctx.n, h, seed_offset + count, SALT_CORES))
        } else {
            Default3::Ones
        };
        let mut s = Signing::for_cores(h, default);
        assert!(merge(&mut s, core, &[]), "a core always merges into a signing of its own");
        s.cores.push(id);
        opened.push(s);
        Some(existing + opened.len() - 1)
    }

    /// The family changed (the signings `changed` took cores, `opened` are new): rebuild the changed matroids and
    /// add the new ones; keep the master's columns (but those with a new loop) and its rank inequalities (their
    /// ranks taken again in the changed matroids); the lowest `L` of the new family starts at the old multipliers.
    fn rebuild(&mut self, changed: &[bool], opened: Vec<Signing>) {
        let ctx = self.ctx;
        let old_lam_best = std::mem::take(&mut self.lam_best);
        for (k, ex) in self.fam.extras.iter_mut().enumerate() {
            if changed[k] {
                let by_elem: HashMap<usize, f64> =
                    ex.elem.iter().copied().zip(old_lam_best[k].iter().copied()).collect();
                let signing = std::mem::replace(&mut ex.signing, Signing::ones(0));
                *ex = Extra::new(ctx, signing);
                self.lam_best.push(ex.elem.iter().map(|e| by_elem.get(e).copied().unwrap_or(0.0)).collect());
            } else {
                self.lam_best.push(old_lam_best[k].clone());
            }
        }
        for signing in opened {
            let ex = Extra::new(ctx, signing);
            self.lam_best.push(vec![0.0; ex.elem.len()]);
            self.fam.extras.push(ex);
        }
        self.fam.ban(ctx);
        let keep_mi: Vec<Vec<usize>> =
            self.master.mi.iter().map(|c| c.set.clone()).filter(|s| s.iter().all(|&e| !self.fam.banned[e])).collect();
        let keep_cuts: Vec<(usize, Vec<usize>)> = self.master.cuts.iter().map(|c| (c.k, c.set.clone())).collect();
        self.master = Master::new(ctx, &self.fam, &keep_mi, &keep_cuts);
        self.stats.masters += 1;
        let at = self.lam_best.clone();
        let ev = self.eval_at(&at);
        self.fam_best = i64::MAX;
        self.note(ev.value, &at);
        self.master.add_mi(ctx, ev.q);
        self.lb = 0.0;
    }

    /// The outcome.
    fn finish(self) -> Outcome {
        let ctx = self.ctx;
        // The GF(2) elements ascend with their candidate numbers, so the sets stay ascending.
        let numbers = |set: &[usize]| -> Vec<u32> { set.iter().map(|&e| ctx.fm.elems[e]).collect() };
        let support = self.master.support();
        let others = self
            .master
            .mi
            .iter()
            .filter(|c| !c.set.is_empty() && self.master.lp.primal(c.lp) <= SUPPORT)
            .map(|c| &c.set);
        let master_columns =
            support.iter().map(|(_, s)| s).chain(others).take(WARM_COLUMNS).map(|s| numbers(s)).collect();
        let master_cuts = self.master.cuts.iter().map(|c| (c.k, numbers(&c.set), c.rank)).collect();
        Outcome {
            bound: self.best_scaled.div_euclid(SCALE),
            best_scaled: self.best_scaled,
            l_best: self.fam_best as f64 / SCALE as f64,
            lb: self.lb,
            master: self.master.lp.value(),
            lambda: to_numbers(&self.fam, &self.lam_best),
            cores_assigned: self.found.iter().filter(|f| f.assigned.is_some()).count(),
            cores_unplaced: self.found.iter().filter(|f| f.assigned.is_none()).count(),
            cores_repeated: self.repeated,
            rounds: self.rounds,
            stop: self.stop.unwrap_or(Stop::Converged),
            support: support.into_iter().map(|(a, s)| (a, numbers(&s))).collect(),
            master_columns,
            master_cuts,
            columns: self.master.lp.columns(),
            cuts: self.master.cuts.len(),
            stats: *self.stats,
            seconds: self.t0.elapsed().as_secs_f64(),
            cores: self.found,
            family: self.fam,
        }
    }
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
    let mut stats = Stats::default();
    let fam = Family::new(ctx, signings);
    let lam = by_number(&fam, start.unwrap_or_default());
    let mut f = Fitter::new(ctx, fam, lam, &[], &[], settings, &mut stats, t0);
    f.polyak();
    let search = settings.mode == FitMode::Search;
    // Whether the current family's point is to be certified: from the start without separation, else once a
    // separation round has found no new core (and a certified family whose separation finds none is the end).  In
    // search mode never: a round that finds nothing new is the end.
    let mut certify = !search && !separate;
    while f.stop.is_none() {
        let certified = f.generate(certify);
        if f.stop.is_some() || !separate {
            break;
        }
        if f.rounds >= settings.rounds {
            f.stop = Some(Stop::Rounds);
            break;
        }
        let (placed, _) = f.separate(&|_| settings.per_frame, 0);
        if placed > 0 {
            certify = false;
        } else if certified || search {
            break;
        } else {
            certify = true;
        }
    }
    f.finish()
}

// ---------------------------------------------------------------------------------------------------------
// Warm states and fits

/// A fitted state at a node: the frozen signings (the root's, then the node-local ones), the multipliers, the
/// master's columns and cuts.  All by candidate number, so that a child can take them over.
#[derive(Clone, Debug, PartialEq)]
pub struct Warm {
    pub signings: Vec<Signing>,
    /// How many of `signings`, the last ones, are node-local (made below the root).
    pub local: usize,
    /// Per signing, its positive multipliers by candidate number.
    pub lambda: Vec<HashMap<u32, f64>>,
    /// Common independent sets of the GF(2) matroids (candidate numbers, ascending), the master's columns.
    pub columns: Vec<Vec<u32>>,
    /// Rank inequalities (signing index, candidate numbers ascending); ranks are taken again where they are used.
    pub cuts: Vec<(usize, Vec<u32>)>,
}

impl Warm {
    /// The state of a computation's outcome: its family's signings frozen (`mergeable: false`, `cores` emptied, so
    /// that nothing merges into them during a search), its multipliers (`Outcome::lambda`), master columns and cuts.
    /// None of its signings is node-local.
    pub fn from_outcome(r: &Outcome) -> Warm {
        Warm {
            signings: r.family.extras.iter().map(|ex| frozen(&ex.signing)).collect(),
            local: 0,
            lambda: r.lambda.clone(),
            columns: r.master_columns.clone(),
            cuts: r.master_cuts.iter().map(|(k, set, _)| (*k, set.clone())).collect(),
        }
    }

    /// The state a child starts from: columns holding every candidate in `promoted_now` and none in `removed`, those
    /// taken out, members not in `live` (the child's GF(2) elements, ascending) dropped, columns with more than
    /// `max_add` members or none dropped, duplicates dropped; cuts with `promoted_now`, `removed` and members not in
    /// `live` taken out, empty ones and duplicates dropped; the multipliers restricted to `live`; signings and
    /// `local` unchanged.
    pub fn child(&self, promoted_now: &[u32], removed: &[u32], live: &[u32], max_add: usize) -> Warm {
        let kept = |x: &u32| !promoted_now.contains(x) && !removed.contains(x) && live.binary_search(x).is_ok();
        let mut seen = HashSet::new();
        let columns = self
            .columns
            .iter()
            .filter(|c| promoted_now.iter().all(|x| c.contains(x)) && !removed.iter().any(|x| c.contains(x)))
            .map(|c| c.iter().copied().filter(kept).collect::<Vec<u32>>())
            .filter(|c| !c.is_empty() && c.len() <= max_add && seen.insert(c.clone()))
            .collect();
        let mut seen = HashSet::new();
        let cuts = self
            .cuts
            .iter()
            .map(|(k, set)| (*k, set.iter().copied().filter(kept).collect::<Vec<u32>>()))
            .filter(|cut| !cut.1.is_empty() && seen.insert(cut.clone()))
            .collect();
        let lambda = self
            .lambda
            .iter()
            .map(|m| m.iter().filter(|(x, _)| live.binary_search(x).is_ok()).map(|(&x, &v)| (x, v)).collect())
            .collect();
        Warm { signings: self.signings.clone(), local: self.local, lambda, columns, cuts }
    }

    /// The local signings of frame `h`.
    pub fn local_in(&self, h: usize) -> usize {
        self.signings[self.signings.len() - self.local..].iter().filter(|s| s.frame == h).count()
    }

    /// The state as a JSON object (the fields of the fit file, cuts without ranks).
    pub fn to_json(&self) -> Json {
        Json::object(self.pairs(None))
    }

    /// The state's fields of the fit file (design S5.1): `signings`, `local`, `lambda`, `columns`, `cuts` (each
    /// `[signing, [candidates]]`, with its rank appended when `ranks` are given).
    fn pairs(&self, ranks: Option<&[usize]>) -> Vec<(&'static str, Json)> {
        let lambda = self.lambda.iter().map(|m| {
            let mut entries: Vec<(u32, f64)> = m.iter().map(|(&x, &v)| (x, v)).collect();
            entries.sort_by_key(|&(x, _)| x);
            Json::Arr(entries.into_iter().map(|(x, v)| Json::Arr(vec![x.into(), Json::Num(v)])).collect())
        });
        let cuts = self.cuts.iter().enumerate().map(|(i, (k, set))| {
            let mut cut = vec![Json::from(*k), Json::array(set.iter().copied())];
            cut.extend(ranks.map(|r| Json::from(r[i])));
            Json::Arr(cut)
        });
        vec![
            ("signings", Json::Arr(self.signings.iter().map(signing_to_json).collect())),
            ("local", self.local.into()),
            ("lambda", Json::Arr(lambda.collect())),
            ("columns", Json::Arr(self.columns.iter().map(|c| Json::array(c.iter().copied())).collect())),
            ("cuts", Json::Arr(cuts.collect())),
        ]
    }

    /// The state of a JSON object with the fields of [`to_json`](Self::to_json) (a cut's rank, if there, ignored).
    pub fn from_json(j: &Json) -> Result<Warm, String> {
        let signings = field(j, "signings")?
            .as_arr()
            .ok_or("signings: not an array")?
            .iter()
            .map(signing_from_json)
            .collect::<Result<Vec<Signing>, String>>()?;
        let local = field(j, "local")?.as_u64().ok_or("local: not a count")? as usize;
        if local > signings.len() {
            return Err(format!("local: {local} of {} signings", signings.len()));
        }
        let lambda = field(j, "lambda")?
            .as_arr()
            .ok_or("lambda: not an array")?
            .iter()
            .map(|m| {
                m.as_arr()
                    .ok_or("lambda: a signing's multipliers are not an array")?
                    .iter()
                    .map(|p| match p.as_arr() {
                        Some([x, v]) => {
                            Ok((x.as_u32().ok_or("lambda: a bad candidate")?, v.as_f64().ok_or("lambda: a bad value")?))
                        }
                        _ => Err("lambda: not an [x, value] pair"),
                    })
                    .collect::<Result<HashMap<u32, f64>, &str>>()
            })
            .collect::<Result<Vec<_>, &str>>()?;
        if lambda.len() != signings.len() {
            return Err(format!("lambda: {} lists for {} signings", lambda.len(), signings.len()));
        }
        let columns = field(j, "columns")?
            .as_arr()
            .ok_or("columns: not an array")?
            .iter()
            .map(|c| c.as_u32_vec().ok_or("columns: not a list of candidates"))
            .collect::<Result<Vec<_>, &str>>()?;
        let cuts = cuts_of(j)?
            .into_iter()
            .map(|(k, set, _)| if k < signings.len() { Ok((k, set)) } else { Err(format!("cuts: signing {k}")) })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Warm { signings, local, lambda, columns, cuts })
    }
}

/// The field `name` of a JSON object.
fn field<'j>(j: &'j Json, name: &str) -> Result<&'j Json, String> {
    j.get(name).ok_or(format!("no field {name:?}"))
}

/// A cut as a fit file has it: signing, candidates, rank if given.
type FileCut = (usize, Vec<u32>, Option<usize>);

/// The cuts of a fit file's object.
fn cuts_of(j: &Json) -> Result<Vec<FileCut>, String> {
    let cuts = field(j, "cuts")?.as_arr().ok_or("cuts: not an array")?;
    cuts.iter()
        .map(|c| {
            let parts = c.as_arr().filter(|p| p.len() == 2 || p.len() == 3).ok_or("cuts: not [signing, set, rank]")?;
            let k = parts[0].as_u64().ok_or("cuts: a bad signing")? as usize;
            let set = parts[1].as_u32_vec().ok_or("cuts: a bad set")?;
            let rank = parts.get(2).map(|r| r.as_u64().map(|r| r as usize).ok_or("cuts: a bad rank")).transpose()?;
            Ok((k, set, rank))
        })
        .collect()
}

/// A signing as the fit file has it: `frame`, `default` (`ones`, or `random` with its `seed` as a decimal string, a
/// u64 not fitting JSON's numbers), `fixed` as `[u, f, v]` sorted by `(u, f)`, `mergeable`.  Its cores are not kept.
fn signing_to_json(s: &Signing) -> Json {
    let mut pairs = vec![("frame", Json::from(s.frame))];
    match s.default {
        Default3::Ones => pairs.push(("default", "ones".into())),
        Default3::Random(seed) => {
            pairs.push(("default", "random".into()));
            pairs.push(("seed", seed.to_string().into()));
        }
    }
    let mut fixed: Vec<(u32, u32, u8)> = s.fixed.iter().map(|(&(u, f), &v)| (u, f, v)).collect();
    fixed.sort_unstable();
    pairs.push(("fixed", Json::Arr(fixed.into_iter().map(|(u, f, v)| Json::array([u, f, u32::from(v)])).collect())));
    pairs.push(("mergeable", s.mergeable.into()));
    Json::object(pairs)
}

/// The signing of [`signing_to_json`]'s object (no cores).
fn signing_from_json(j: &Json) -> Result<Signing, String> {
    let frame = field(j, "frame")?.as_u64().ok_or("a signing's frame is not a number")? as usize;
    let default = match field(j, "default")?.as_str() {
        Some("ones") => Default3::Ones,
        Some("random") => {
            let seed = field(j, "seed")?.as_str().ok_or("a signing's seed is not a string")?;
            Default3::Random(seed.parse().map_err(|e| format!("a signing's seed {seed:?}: {e}"))?)
        }
        _ => return Err("a signing's default is neither \"ones\" nor \"random\"".into()),
    };
    let mut fixed = HashMap::new();
    for t in field(j, "fixed")?.as_arr().ok_or("a signing's fixed values are not an array")? {
        match t.as_u32_vec().as_deref() {
            Some(&[u, f, v]) if v == 1 || v == 2 => fixed.insert((u, f), v as u8),
            _ => return Err(format!("a signing's fixed value {t} is not [u, f, 1 or 2]")),
        };
    }
    let mergeable = field(j, "mergeable")?.as_bool().ok_or("a signing's mergeable is not a boolean")?;
    Ok(Signing { frame, default, fixed, cores: Vec::new(), mergeable })
}

/// A root fit, as a file (design S5.1): the game, the bounds, the state.
#[derive(Clone, Debug, PartialEq)]
pub struct Fit {
    pub n: u32,
    pub opt: i64,
    pub gf2: i64,
    pub bound: i64,
    pub l_best: f64,
    pub warm: Warm,
    /// The ranks of `warm.cuts` at the root (written for reference; never trusted when read).
    pub ranks: Vec<usize>,
}

impl Fit {
    pub fn from_outcome(n: u32, opt: i64, gf2: i64, r: &Outcome) -> Fit {
        Fit {
            n,
            opt,
            gf2,
            bound: r.bound,
            l_best: r.l_best,
            warm: Warm::from_outcome(r),
            ranks: r.master_cuts.iter().map(|&(_, _, rank)| rank).collect(),
        }
    }

    /// The fit file's one line: `n`, `opt`, `gf2`, `bound`, `l_best`, then the state's fields, cuts with ranks.
    pub fn to_json(&self) -> String {
        let head =
            [("n", self.n.into()), ("opt", self.opt.into()), ("gf2", self.gf2.into()), ("bound", self.bound.into())];
        let pairs =
            head.into_iter().chain([("l_best", Json::Num(self.l_best))]).chain(self.warm.pairs(Some(&self.ranks)));
        Json::object(pairs).to_string()
    }
}

/// The fit of a fit file's line.
pub fn fit_from_json(text: &str) -> Result<Fit, String> {
    let j = Json::parse(text).map_err(|e| e.to_string())?;
    let int = |name: &str| field(&j, name)?.as_i64().ok_or(format!("{name}: not an integer"));
    let warm = Warm::from_json(&j)?;
    let ranks = cuts_of(&j)?
        .into_iter()
        .map(|(_, _, rank)| rank.ok_or("cuts: a fit's cut without its rank"))
        .collect::<Result<Vec<_>, &str>>()?;
    Ok(Fit {
        n: u32::try_from(int("n")?).map_err(|e| format!("n: {e}"))?,
        opt: int("opt")?,
        gf2: int("gf2")?,
        bound: int("bound")?,
        l_best: field(&j, "l_best")?.as_f64().ok_or("l_best: not a number")?,
        warm,
        ranks,
    })
}

// ---------------------------------------------------------------------------------------------------------
// The node methods

/// The bound after each pricing of a refit, for the audit and the logs.
#[derive(Clone, Debug, Default)]
pub struct Trace {
    /// `after[0]`: ⌊L⌋ at the inherited multipliers (Tier 1); `after[i]`: the lowest ⌊L⌋ after `i` refit pricings.
    pub after: Vec<i64>,
    /// Refit pricings (evaluations of L after Tier 1).
    pub pricings: usize,
    /// Node-local signings opened.
    pub new_signings: usize,
    pub stop: Option<Stop>,
}

/// Tier 1: one evaluation of L at `warm`'s multipliers, `fam` being built from `warm.signings` in their order at the
/// node's state.  Returns ⌊L⌋ and the evaluation (its sets start a refit).
pub fn tier1<const W: usize>(ctx: &Ctx<'_, W>, fam: &Family, warm: &Warm, stats: &mut Stats) -> (i64, Eval) {
    assert_eq!(fam.extras.len(), warm.signings.len(), "a family of the warm state's signings");
    let ev = evaluate(ctx, fam, &by_number(fam, &warm.lambda), stats, CHECK_EVERY);
    (ev.value.div_euclid(SCALE), ev)
}

/// R1, deflected Polyak toward the target (design S3.2), from `warm`'s multipliers whose evaluation is `first`, at
/// most `k` steps: subgradient `g_{k,x} = [x ∈ I_k] − [x ∈ Q]`, direction `d = g + β d_prev` with `β = 1.5 · max(0,
/// −g·d_prev) / ‖d_prev‖²` (Camerini, Fratta and Maffioli; `d = g` the first time), step `θ (L − t + ½) / ‖d‖²` toward
/// the target `t`, `λ` clamped at 0, `θ` from 1 halved after 3 steps without a new lowest `L`; it stops at the target,
/// at `k` steps, at `‖d‖ = 0` or at `θ < 1/64`.  No LP.  Returns the lowest ⌊L⌋ (with Tier 1's), the state with the
/// multipliers of the lowest L (columns and cuts passed through unchanged) and the trace.
pub fn refit_r1<const W: usize>(
    ctx: &Ctx<'_, W>,
    fam: &Family,
    warm: &Warm,
    first: &Eval,
    k: usize,
    stats: &mut Stats,
) -> (i64, Warm, Trace) {
    let pricings = stats.pricings_mi;
    let mut lam = by_number(fam, &warm.lambda);
    let mut lam_best = lam.clone();
    let mut best = first.value;
    let mut after = vec![best.div_euclid(SCALE)];
    let (mut theta, mut since, mut steps) = (1.0f64, 0usize, 0usize);
    let mut last: Option<Eval> = None;
    let mut previous: Option<Vec<Vec<f64>>> = None;
    let stop = loop {
        if best.div_euclid(SCALE) <= ctx.target {
            break Stop::Target;
        }
        if theta < 1.0 / 64.0 {
            break Stop::Converged;
        }
        if steps == k {
            break Stop::Pricings;
        }
        let ev = last.as_ref().unwrap_or(first);
        let (mut d, _) = subgradient(fam, ev);
        if let Some(p) = &previous {
            let beta = 1.5 * (-dot(&d, p)).max(0.0) / dot(p, p);
            for (d, p) in d.iter_mut().zip(p) {
                for (d, p) in d.iter_mut().zip(p) {
                    *d += beta * p;
                }
            }
        }
        let norm = dot(&d, &d);
        if norm == 0.0 {
            break Stop::Converged;
        }
        let excess = ev.value as f64 / SCALE as f64 - ctx.target as f64 + 0.5;
        descend(&mut lam, theta * excess / norm, &d);
        let next = evaluate(ctx, fam, &lam, stats, CHECK_EVERY);
        steps += 1;
        if next.value < best {
            best = next.value;
            lam_best.clone_from(&lam);
            since = 0;
        } else {
            since += 1;
            if since == 3 {
                theta /= 2.0;
                since = 0;
            }
        }
        after.push(best.div_euclid(SCALE));
        last = Some(next);
        previous = Some(d);
    };
    let state = Warm { lambda: to_numbers(fam, &lam_best), ..warm.clone() };
    let pricings = (stats.pricings_mi - pricings) as usize;
    (best.div_euclid(SCALE), state, Trace { after, pricings, new_signings: 0, stop: Some(stop) })
}

/// R2, the warm master (design S3.2): the column generation of [`bound`] over `fam` (built from `warm.signings` at
/// the node) from `warm`'s multipliers, whose evaluation `first` is recorded first (not a refit pricing), with a
/// master of `warm`'s columns and `first.q` and of `warm`'s rank inequalities (ranks taken in `fam`), in search mode,
/// at most `k` pricings after Tier 1, stopping at the target (if the context stops there), the budget,
/// `settings.seconds`, or when nothing prices out and no cheap cut is violated.  With `local > 0`, after it stops
/// without reaching the target, one separation round at the node ([`LOCAL_CORES_PER_FRAME`] cores a frame and
/// column; at most `min(local, LOCAL_PER_FRAME − warm.local_in(h))` new signings of frame `h`, their defaults
/// seeded from index `LOCAL_SEEDS + warm.local`) and, if it opened any, at most `k / 2` more pricings for the new
/// family.  Returns the lowest ⌊L⌋ (with Tier 1's), the final state (the new signings last and node-local) and the
/// trace.
#[allow(clippy::too_many_arguments)]
pub fn refit_r2<const W: usize>(
    ctx: &Ctx<'_, W>,
    fam: Family,
    warm: &Warm,
    first: Eval,
    k: usize,
    local: usize,
    settings: &Settings,
    stats: &mut Stats,
) -> (i64, Warm, Trace) {
    assert_eq!(fam.extras.len(), warm.signings.len(), "a family of the warm state's signings");
    let t0 = Instant::now();
    let pricings = stats.pricings_mi;
    let elements = |set: &[u32]| -> Vec<usize> { set.iter().filter_map(|&x| ctx.element(x)).collect() };
    // Columns with a loop of the node's family are no common independent set of the intersection's.
    let mut columns: Vec<Vec<usize>> =
        warm.columns.iter().map(|c| elements(c)).filter(|c| c.iter().all(|&e| !fam.banned[e])).collect();
    columns.push(first.q.clone());
    // A rank inequality is over its matroid's elements; one whose rank is its size holds at every master point.
    let cuts: Vec<(usize, Vec<usize>)> = warm
        .cuts
        .iter()
        .map(|(j, set)| (*j, elements(set).into_iter().filter(|&e| fam.extras[*j].position(e).is_some()).collect()))
        .filter(|(j, set): &(usize, Vec<usize>)| rank(&fam.extras[*j], set) < set.len())
        .collect();
    let start = by_number(&fam, &warm.lambda);
    let settings =
        Settings { mode: FitMode::Search, pricings: k, cores_per_frame: LOCAL_CORES_PER_FRAME, ..settings.clone() };
    let mut f = Fitter::new(ctx, fam, start.clone(), &columns, &cuts, &settings, stats, t0);
    f.trace = Some(Vec::new());
    f.record(&first, &start);
    if f.stop.is_none() {
        f.generate(false);
    }
    let mut new = 0;
    if local > 0 && k / 2 > 0 && !matches!(f.stop, Some(Stop::Target | Stop::Seconds)) {
        let stopped = f.stop.take();
        f.restart_budget(k / 2);
        let per_frame = |h: usize| local.min(LOCAL_PER_FRAME.saturating_sub(warm.local_in(h)));
        (_, new) = f.separate(&per_frame, LOCAL_SEEDS + warm.local);
        if new == 0 {
            f.stop = stopped;
        } else if f.stop.is_none() {
            f.generate(false);
        }
    }
    let after = f.trace.take().unwrap_or_default();
    let r = f.finish();
    let state = Warm { local: warm.local + new, ..Warm::from_outcome(&r) };
    let pricings = (stats.pricings_mi - pricings) as usize;
    (r.bound, state, Trace { after, pricings, new_signings: new, stop: Some(r.stop) })
}
