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
//! `min_λ L(λ)` is the LP over `P(A) ∩ P(B) ∩ ⋂_k P(N_k)` (Lagrangian decomposition with integral subproblems).  Its
//! Dantzig–Wolfe master has a column `α_j` per common independent set `Q_j` of `A`, `B` (cost `w(Q_j)`), a column
//! `β_{k,i}` per independent set `I_{k,i}` of `N_k`, a convexity row for the `α`'s and one per `k`, and a *link row*
//! `Σ_j α_j [x ∈ Q_j] − Σ_i β_{k,i} [x ∈ I_{k,i}] ≤ ε_{k,x}` for each `k` and each `x ∈ supp_k` that some `Q_j` holds
//! (its dual is `λ_{k,x} ≥ 0`; `ε` uniform in `[0, 10⁻⁷]` against degeneracy).  The empty sets give a feasible start.
//! A pricing at multipliers `λ` is one evaluation of `L(λ)`: its intersection's set is an `α` column and each greedy
//! set a `β` column, and its value a valid bound.  So the master's value `LB` and the lowest `L` found bracket the LP.
//! Before the master, Polyak steps from `λ = 0` (or a given start) towards the target; then Wentges-stabilized
//! pricing at `½ λ_best + ½ λ_master`, or at `λ_master` when that prices nothing out, until the bracket is within
//! [`CLOSE`] or a budget ends.  With separation, after each convergence the cores that the master's `α` columns run
//! into (`cores::cores_of`) are merged into core-derived signings (first fit, at most `per_frame` a frame), and the
//! master is rebuilt for the changed family from the columns still valid in it.

use crate::cores::{cores_of, merge, Core};
use crate::frames::FrameMatroids;
use crate::gf3::{seed, Default3, Matroid3, Signing};
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
/// A column enters the master when its reduced cost at the master's duals exceeds this.
const ENTER: f64 = 1e-7;
/// An `α` column is in the master's support when its value exceeds this.
pub const SUPPORT: f64 = 1e-6;
/// The default largest perturbation of a link row's right-hand side.
pub const MAX_EPS: f64 = 1e-7;
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
    /// The bracket at which the column generation of a family stops ([`CLOSE`]).
    pub close: f64,
    /// The largest perturbation of a link row's right-hand side ([`MAX_EPS`]; 0 for none).
    pub eps: f64,
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
            eps: MAX_EPS,
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
}

impl Extra {
    pub fn new<const W: usize>(ctx: &Ctx<'_, W>, signing: Signing) -> Self {
        let m = Matroid3::build(ctx.model, ctx.state, &ctx.fm.elems, &signing);
        let elem: Vec<usize> = m.elems.iter().map(|&x| ctx.element(x).expect("a GF(2) element")).collect();
        let mut pos = vec![NONE; ctx.len()];
        for (i, &e) in elem.iter().enumerate() {
            pos[e] = i as u32;
        }
        Extra { signing, m, elem, pos }
    }

    fn position(&self, e: usize) -> Option<usize> {
        Some(self.pos[e]).filter(|&p| p != NONE).map(|p| p as usize)
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
    pub bland: u64,
    pub solves: u64,
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
// The master

/// A column of the master: its LP column and its set (GF(2) elements, ascending).
struct Col {
    lp: usize,
    set: Vec<usize>,
}

/// The Dantzig–Wolfe master of a family (see the module docs).
struct Master {
    lp: Lp,
    /// The convexity rows: the `α`'s, then one per extra matroid.
    conv: Vec<usize>,
    /// Per extra matroid, the link row of each GF(2) element (`NONE` while no `α` column holds it).
    link: Vec<Vec<u32>>,
    mi: Vec<Col>,
    ex: Vec<Vec<Col>>,
    seen_mi: HashSet<Vec<usize>>,
    seen_ex: Vec<HashSet<Vec<usize>>>,
    /// The perturbations' generator and their largest value.
    rng: u64,
    eps: f64,
}

impl Master {
    /// The master of `fam` with the columns `mi` and `ex` (sets; the empty sets are added anyway), at the basis of
    /// the empty sets.
    fn new<const W: usize>(
        ctx: &Ctx<'_, W>,
        fam: &Family,
        mi: &[Vec<usize>],
        ex: &[Vec<Vec<usize>>],
        rng: u64,
        eps: f64,
    ) -> Self {
        let mut lp = Lp::new();
        let conv: Vec<usize> = (0..=fam.extras.len()).map(|_| lp.add_row(RowKind::Eq, 1.0, &[])).collect();
        let k = fam.extras.len();
        let mut m = Master {
            lp,
            conv,
            link: vec![vec![NONE; ctx.len()]; k],
            mi: Vec::new(),
            ex: (0..k).map(|_| Vec::new()).collect(),
            seen_mi: HashSet::new(),
            seen_ex: (0..k).map(|_| HashSet::new()).collect(),
            rng: rng | 1,
            eps,
        };
        let mut empties = vec![m.add_mi(ctx, fam, Vec::new()).expect("the empty set")];
        for j in 0..k {
            empties.push(m.add_ex(j, Vec::new()).expect("the empty set"));
        }
        m.lp.reset(&empties);
        for set in mi {
            m.add_mi(ctx, fam, set.clone());
        }
        for (j, sets) in ex.iter().enumerate() {
            for set in sets {
                m.add_ex(j, set.clone());
            }
        }
        m
    }

    fn epsilon(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.eps * (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Adds the `α` column of `set` (and the link rows of its elements that have none yet); `None` if it is there.
    fn add_mi<const W: usize>(&mut self, ctx: &Ctx<'_, W>, fam: &Family, set: Vec<usize>) -> Option<usize> {
        if self.seen_mi.contains(&set) {
            return None;
        }
        let mut entries = vec![(self.conv[0], 1.0)];
        for &e in &set {
            for (k, ex) in fam.extras.iter().enumerate() {
                if ex.position(e).is_none() {
                    continue;
                }
                if self.link[k][e] == NONE {
                    // The row's left side is −Σ β over the β columns holding e: the current point satisfies it.
                    let holders: Vec<(usize, f64)> =
                        self.ex[k].iter().filter(|c| c.set.binary_search(&e).is_ok()).map(|c| (c.lp, -1.0)).collect();
                    let eps = self.epsilon();
                    self.link[k][e] = self.lp.add_row(RowKind::Le, eps, &holders) as u32;
                }
                entries.push((self.link[k][e] as usize, 1.0));
            }
        }
        let cost: f64 = set.iter().map(|&e| f64::from(ctx.fm.elems[e])).sum();
        let j = self.lp.add_column(cost, &entries);
        self.seen_mi.insert(set.clone());
        self.mi.push(Col { lp: j, set });
        Some(j)
    }

    /// Adds the `β` column of `set` in extra matroid `k`; `None` if it is there.
    fn add_ex(&mut self, k: usize, set: Vec<usize>) -> Option<usize> {
        if self.seen_ex[k].contains(&set) {
            return None;
        }
        let mut entries = vec![(self.conv[k + 1], 1.0)];
        entries.extend(set.iter().filter(|&&e| self.link[k][e] != NONE).map(|&e| (self.link[k][e] as usize, -1.0)));
        let j = self.lp.add_column(0.0, &entries);
        self.seen_ex[k].insert(set.clone());
        self.ex[k].push(Col { lp: j, set });
        Some(j)
    }

    /// The dual of the link row of element `e` in extra matroid `k`, at least 0 (0 without a row).
    fn lambda(&self, k: usize, e: usize) -> f64 {
        match self.link[k][e] {
            NONE => 0.0,
            r => self.lp.duals()[r as usize].max(0.0),
        }
    }

    /// The master's multipliers, one vector per extra matroid indexed like its elements.
    fn multipliers(&self, fam: &Family) -> Vec<Vec<f64>> {
        fam.extras.iter().enumerate().map(|(k, ex)| ex.elem.iter().map(|&e| self.lambda(k, e)).collect()).collect()
    }

    /// The reduced costs at the master's duals of an evaluation's columns: the `α` column's, then each `β`'s.
    fn reduced_costs<const W: usize>(&self, ctx: &Ctx<'_, W>, ev: &Eval) -> (f64, Vec<f64>) {
        let y = self.lp.duals();
        let k = self.ex.len();
        let mut mi: f64 = -y[self.conv[0]];
        for &e in &ev.q {
            mi += f64::from(ctx.fm.elems[e]);
            mi -= (0..k).map(|j| self.lambda(j, e)).sum::<f64>();
        }
        let ex = (0..k).map(|j| ev.sets[j].iter().map(|&e| self.lambda(j, e)).sum::<f64>() - y[self.conv[j + 1]]);
        (mi, ex.collect())
    }

    /// Adds an evaluation's columns whose reduced costs exceed [`ENTER`]; how many were added.
    fn add_priced<const W: usize>(&mut self, ctx: &Ctx<'_, W>, fam: &Family, ev: &Eval) -> usize {
        let (mi, ex) = self.reduced_costs(ctx, ev);
        let mut added = 0;
        if mi > ENTER && self.add_mi(ctx, fam, ev.q.clone()).is_some() {
            added += 1;
        }
        for (k, rc) in ex.into_iter().enumerate() {
            if rc > ENTER && self.add_ex(k, ev.sets[k].clone()).is_some() {
                added += 1;
            }
        }
        added
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
        stats.bland += after.bland - before.bland;
        stats.solves += 1;
        stats.ms_simplex += t.elapsed().as_secs_f64() * 1e3;
        stats.max_rows = stats.max_rows.max(self.lp.rows());
        stats.max_s = stats.max_s.max(self.lp.stats().max_s);
        optimal
    }

    /// The `α` columns in the support, heaviest value first: (value, set).
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
    /// The bracket closed and separation (if on) found nothing new.
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
    /// The bracket of the final family's LP: the lowest `L` of the final family and the master's value.
    pub l_best: f64,
    pub lb: f64,
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
    pub rows: usize,
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
    let check = settings.check_every;

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

    // Polyak steps towards the target, every set they produce kept as a column.
    let mut mi_cols: Vec<Vec<usize>> = Vec::new();
    let mut ex_cols: Vec<Vec<Vec<usize>>> = fam.extras.iter().map(|_| Vec::new()).collect();
    let steps = if fam.extras.is_empty() { 1 } else { settings.polyak.max(1) };
    let (mut theta, mut stall) = (1.0f64, 0usize);
    for _ in 0..steps {
        let ev = evaluate(ctx, &fam, &lam, &mut stats, check);
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
        for (k, s) in ev.sets.iter().enumerate() {
            ex_cols[k].push(s.clone());
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

    let mut rng = u64::from(ctx.n).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut master = Master::new(ctx, &fam, &mi_cols, &ex_cols, rng, settings.eps);
    stats.masters += 1;
    let mut found: Vec<Found> = Vec::new();
    let mut known: HashMap<Core, usize> = HashMap::new();
    let (mut repeated, mut rounds) = (0, 0);
    let mut lb = 0.0;

    'rounds: while stop.is_none() {
        // Column generation for the current family.
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
                lb = master.lp.value();
                stop = Some(Stop::Seconds);
                break 'rounds;
            }
            lb = master.lp.value();
            if fam_best as f64 / SCALE as f64 - lb <= settings.close {
                break;
            }
            let lam_master = master.multipliers(&fam);
            let smoothed: Vec<Vec<f64>> = lam_best
                .iter()
                .zip(&lam_master)
                .map(|(b, m)| b.iter().zip(m).map(|(b, m)| 0.5 * b + 0.5 * m).collect())
                .collect();
            let mut added = 0;
            for at in [smoothed, lam_master] {
                let ev = evaluate(ctx, &fam, &at, &mut stats, check);
                best_scaled = best_scaled.min(ev.value);
                if ev.value < fam_best {
                    fam_best = ev.value;
                    lam_best = at;
                }
                if reached(best_scaled) {
                    stop = Some(Stop::Target);
                    break 'rounds;
                }
                added = master.add_priced(ctx, &fam, &ev);
                if added > 0 {
                    break;
                }
            }
            if added == 0 {
                // Nothing prices out at the master's own duals: the master is the LP of the family.
                break;
            }
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
            break;
        }
        // The family changed: rebuild the changed matroids, add the new ones, keep the columns still valid.
        let old_lam_best = std::mem::take(&mut lam_best);
        let mut keep_ex: Vec<Vec<Vec<usize>>> = Vec::new();
        for (k, ex) in fam.extras.iter_mut().enumerate() {
            let sets: Vec<Vec<usize>> = master.ex[k].iter().map(|c| c.set.clone()).filter(|s| !s.is_empty()).collect();
            if changed[k] {
                let by_elem: HashMap<usize, f64> =
                    ex.elem.iter().copied().zip(old_lam_best[k].iter().copied()).collect();
                let signing = std::mem::replace(&mut ex.signing, Signing::ones(0));
                *ex = Extra::new(ctx, signing);
                lam_best.push(ex.elem.iter().map(|e| by_elem.get(e).copied().unwrap_or(0.0)).collect());
                let numbers = |s: &Vec<usize>| s.iter().map(|&e| ctx.fm.elems[e]).collect::<Vec<u32>>();
                keep_ex.push(sets.into_iter().filter(|s| ex.m.independent(&numbers(s))).collect());
            } else {
                lam_best.push(old_lam_best[k].clone());
                keep_ex.push(sets);
            }
        }
        for signing in opened {
            let ex = Extra::new(ctx, signing);
            lam_best.push(vec![0.0; ex.elem.len()]);
            keep_ex.push(Vec::new());
            fam.extras.push(ex);
        }
        fam.ban(ctx);
        let keep_mi: Vec<Vec<usize>> = master
            .mi
            .iter()
            .map(|c| c.set.clone())
            .filter(|s| !s.is_empty() && s.iter().all(|&e| !fam.banned[e]))
            .collect();
        rng = rng.wrapping_add(0x9E37_79B9_7F4A_7C15);
        master = Master::new(ctx, &fam, &keep_mi, &keep_ex, rng, settings.eps);
        stats.masters += 1;
        // The lowest L of the new family starts at the old multipliers.
        let ev = evaluate(ctx, &fam, &lam_best, &mut stats, check);
        best_scaled = best_scaled.min(ev.value);
        fam_best = ev.value;
        master.add_priced(ctx, &fam, &ev);
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
        rows: master.lp.rows(),
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
