//! The multi-field bound and its multipliers: one weighted matroid intersection over GF(2) plus a greedy algorithm per
//! signed GF(3) frame, coupled by Lagrangian multipliers that Dantzig–Wolfe column generation fits.  It is the code of
//! the fields study (solver/notebook/search/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md) as it
//! stood at commit 8753189, with the refit R1, the refit R3 and the checkpoint formats left out and the intersection no
//! longer checked against a reference algorithm at run time (`wall/fields.rs` tests it against the crate's own bound).
//! # The bound
//!
//! Independence in every frame over GF(2) is two conditions, the even frames' side `A` and the odd frames' side `B`
//! (their vectors are `frames::FrameMatroids`).  Let there also be K *extras* `1, …, K`, each independence in one frame
//! `h_k` under one GF(3) signing, on the candidates touching that frame, `supp_k` (`gf3::Matroid3`: the test and the
//! greedy algorithm on its vectors).  Every promotable set `Q` is independent on both sides (the frame bound (B2),
//! docs/theory/frame-bounds.md) and has `Q ∩ supp_k` independent in every extra `k` (the signed frame theorem (B1): any
//! field, any nonzero entries).  For any multipliers `λ_k ≥ 0` on `supp_k`,
//!
//! ```text
//! w(Q) = (w − Σ_k λ_k)(Q) + Σ_k λ_k(Q ∩ supp_k) ≤ MI_cap(w − Σ_k λ_k) + Σ_k G_k(λ_k) = L(λ)     (B4)
//! ```
//!
//! with `MI_cap(c)` the heaviest set of at most `cap` elements independent on both sides (elements of weight at most 0
//! left out, which loses nothing over a down-closed family) and `G_k(μ)` the heaviest set independent in extra `k`
//! (greedy).  A candidate whose class is 0 in some extra is in no promotable set and is left out of the intersection
//! (it is `L` at `λ_{k,x} = w_x`, so B4 covers it).  The bound reported is `⌊L⌋` evaluated exactly
//! in integers: weights `x·S` and multipliers rounded to the grid `1/S` (`S = 2^16`, [`SCALE`]), the intersection by
//! the crate's `taxman::gf2::meet::Meet` and the greedies over GF(3), all in `i64` (sums that would leave it saturate,
//! which keeps the bound valid whatever multipliers are proposed: [`evaluate_grid`]).  Its soundness rests on the
//! decomposition bound (B4) and the two exact oracles, never on the floating-point LP below, which only proposes
//! multipliers.
//!
//! # The multipliers
//!
//! `min_λ L(λ)` is the LP over `P_A ∩ P_B ∩ ⋂_k P_k` with `Σ z ≤ cap`, where `P_A`, `P_B` and `P_k` are the polytopes
//! (convex hulls of the indicator vectors) of the sets independent on side `A`, on side `B` and in extra `k`
//! (Lagrangian decomposition with integral subproblems; Edmonds).  Its Dantzig–Wolfe master has a column `α_j` per set
//! `Q_j` independent on both sides (cost `w(Q_j)`) and a convexity row, as the design has it, but each `P_k` enters by
//! its rank inequalities `z(S) ≤ r_k(S)` (Edmonds' description of the hull of the independent sets of a family of
//! vectors, `r_k` the rank of the classes in extra `k`) rather than by columns of independent sets linked
//! element by element: that master (link rows `Σ_j α_j [x ∈ Q_j] − Σ_i β_{k,i} [x ∈ I_{k,i}] ≤ ε`, the design's) was
//! built and measured first, and its simplex spent nearly all its pivots in steps of the size of the perturbation
//! (solver/notebook/search/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md).  A rank inequality's dual
//! `μ_{k,S} ≥ 0` gives `λ_{k,x} = Σ_{S ∋ x} μ_{k,S}`, and then `G_k(λ_k) ≤ Σ_S μ_{k,S} r_k(S)`, so the master's dual is
//! a Lagrangian bound's, and at the LP's optimal duals they coincide.  A pricing at multipliers `λ` is one evaluation
//! of `L(λ)`, a valid bound, and its intersection's set is a column.  The inequalities are found from the master's
//! point `z = Σ_j α_j 1_{Q_j}`: cheaply (in each extra the most violated level set of `z`, and the dependent support
//! sets), and when that finds none, exactly (for each extra, the largest `y ≤ z` in its polytope, by a small column
//! generation over its independent sets; `y = z` exactly when `z` is in the polytope, and otherwise a level set of its
//! final duals is violated).  When every extra's check passes, `ẑ = min(z, y_1, y_2, …)` lies in every polytope (all
//! are down-closed), and its weight is a *certified* lower bound `LB` on the LP; with the lowest `L` found it brackets
//! the LP.
//!
//! Before the master, Polyak steps from `λ = 0` (or a given start) towards the target; then Wentges-stabilized
//! pricing at `½ λ_best + ½ λ_master`, or at `λ_master` when that prices nothing out, until nothing prices out or
//! the lowest `L` is within [`CLOSE`] of the master's value, then the cuts and the certificate, or until a budget
//! ends.  With separation, after each bracket the cores that the master's columns run into (`cores::cores_of`) are
//! merged into core-derived signings (first fit, at most `per_frame` a frame), and the master is rebuilt for the
//! changed family from its columns (but those holding a candidate whose class has become 0) and its inequalities (ranks
//! taken again).
//!
//! In [`FitMode::Search`] the exact checks are not run: a family's generation ends when nothing prices out and no
//! cheap inequality is violated, and the computation when the separation finds no new core ([`Stop::Fitted`]); `LB`
//! stays 0.  The result is a fit ([`Fit`]: signings and multipliers) with a valid bound and no certified bracket.
//! [`Outcome::fit`] is the fit of the lowest `L` ever evaluated, which can be lower than the final family's lowest
//! `L`: merging cores into a signing changes which sets are independent in its frame, the new ones not a subfamily of
//! the old, so a later family's LP can be the higher.  `fit.rs` writes a fit to disk.
//!
//! # Warm states and the node methods
//!
//! A fitted state ([`Warm`]: frozen signings, multipliers, the master's columns and inequalities, all by candidate
//! number) comes from a root fit read from its file ([`Warm::from_saved`]) and is restricted to a node
//! ([`Warm::child`]).  At a node, [`tier1`] evaluates `L` at the inherited multipliers, and [`refit_r2`] runs the
//! column generation of [`bound`] from the inherited master in search mode.  Each returns the lowest `⌊L⌋` it
//! evaluated, which is valid at any multipliers and signings (B1 and B4, at a node by B1 (2)); the master and
//! the steps only propose multipliers.  (The study also handed a refit's state on from a node to its children, and
//! refitted with node-local signings and with the exact checks; the state of the tree's nodes is not needed where
//! nodes are measured one by one.)
//!
//! A master solve that ends optimal at a basis that is not primal feasible, or with absurd duals, has broken down
//! (rounding in `lp.rs`'s simplex): it is solved again from the empty set's basis, and a second breakdown ends the
//! computation ([`Stop::Breakdown`]) with the lowest `L` found, which stays valid.

use super::cores::{cores_of, merge, Core};
use super::fit::SavedFit;
use super::frames::FrameMatroids;
use super::gf3::{seed, Basis3, Default3, Matroid3, Signing};
use super::lp::{Lp, RowKind, Status};
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
/// The most columns of the final master an outcome carries (so a fit file and a [`Warm`] state).
pub const SAVED_COLUMNS: usize = 300;
/// The independent sets an extra's pool keeps.
const POOL: usize = 400;
/// The largest perturbation of a rank inequality's right-hand side in the master, and of a bound `y ≤ z` in the exact
/// check, against degeneracy (uniform in `[0, PERTURB]`).
const PERTURB: f64 = 1e-6;
/// The salt of the seeds of the core-derived signings' default values.
pub const SALT_CORES: u64 = 2;
/// A master solve that ends optimal at a basis whose primal violation exceeds this, or with a dual of magnitude above
/// [`BROKEN_DUAL`], has broken down (design U4, Phase 0): its duals are not to be trusted.
const BROKEN_VIOLATION: f64 = 1e-6;
const BROKEN_DUAL: f64 = 1e8;
/// No row, no position.
const NONE: u32 = u32::MAX;

// ---------------------------------------------------------------------------------------------------------
// The setting

/// One bound computation's state: the frames, the GF(2) frame vectors, the ceiling and the target.
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
    /// Words of a block vector in the intersection, and each element's vectors on sides `A` and `B`, that many words
    /// each.
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

/// How far a fit goes in certifying its bracket.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitMode {
    /// The exact checks run once the separation finds no new core: the bracket `[lb, l_best]` is certified, and the
    /// computation ends when it closes (the study's instrument).
    Certified,
    /// No exact checks: a family's generation ends when no column prices out at the master's duals and no cheap
    /// inequality is violated, and the computation when the separation then finds no new core (or at a budget).  `lb`
    /// is 0, so the root is not counted as converged; `bound` is the same valid bound.  What a search can afford.
    Search,
}

/// The budgets and knobs of a bound computation.
#[derive(Clone, Debug)]
pub struct Settings {
    pub mode: FitMode,
    /// The seconds the computation has.
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
            close: CLOSE,
            close_gap: CLOSE_GAP,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The family of extras

/// One extra: a signing, the classes of the candidates in its frame at the state, and where its elements sit among
/// the GF(2) elements.
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

/// The extras and the GF(2) elements whose class is 0 in one of them.
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

    /// Zero multipliers.
    fn zeros(&self) -> Vec<Vec<f64>> {
        self.extras.iter().map(|ex| vec![0.0; ex.elem.len()]).collect()
    }

    /// The multipliers `lambda` (per signing, by candidate number; zero where missing) as vectors indexed like each
    /// extra's elements; signings past the end of `lambda` get zeros.
    pub fn multipliers_of(&self, lambda: &[HashMap<u32, f64>]) -> Vec<Vec<f64>> {
        let none = HashMap::new();
        self.extras
            .iter()
            .enumerate()
            .map(|(k, ex)| {
                let by_number = lambda.get(k).unwrap_or(&none);
                ex.m.elems.iter().map(|x| by_number.get(x).copied().unwrap_or(0.0)).collect()
            })
            .collect()
    }

    /// The multipliers `lam` (indexed like each extra's elements) by candidate number, the positive ones.
    fn by_number(&self, lam: &[Vec<f64>]) -> Vec<HashMap<u32, f64>> {
        self.extras
            .iter()
            .zip(lam)
            .map(|(ex, l)| ex.m.elems.iter().copied().zip(l.iter().copied()).filter(|&(_, v)| v > 0.0).collect())
            .collect()
    }
}

/// A fit: signings and multipliers, which together give a valid bound at the state they were fitted at (and at its
/// nodes: B1 and B4, docs/theory/frame-bounds.md).
#[derive(Clone, Debug, Default)]
pub struct Fit {
    pub signings: Vec<Signing>,
    /// Per signing, the multipliers by candidate number (the positive ones).
    pub lambda: Vec<HashMap<u32, f64>>,
}

impl Fit {
    /// The family `fam` at the multipliers `lam` (indexed like each extra's elements).
    fn of(fam: &Family, lam: &[Vec<f64>]) -> Fit {
        Fit { signings: fam.extras.iter().map(|ex| ex.signing.clone()).collect(), lambda: fam.by_number(lam) }
    }

    /// `⌊L⌋` of this fit at the state of `ctx`: the bound it gives.
    pub fn bound<const W: usize>(&self, ctx: &Ctx<'_, W>, stats: &mut Stats) -> i64 {
        let fam = Family::new(ctx, self.signings.clone());
        let lam = fam.multipliers_of(&self.lambda);
        evaluate(ctx, &fam, &lam, stats).value.div_euclid(SCALE)
    }
}

/// Multipliers by candidate number as the fit files and the checkpoints have them: per signing,
/// `[[candidate, multiplier]...]` ascending by candidate.
pub fn multipliers_to_json(lambda: &[HashMap<u32, f64>]) -> Json {
    let lists = lambda.iter().map(|l| {
        let mut pairs: Vec<(u32, f64)> = l.iter().map(|(&x, &v)| (x, v)).collect();
        pairs.sort_unstable_by_key(|&(x, _)| x);
        Json::Arr(pairs.into_iter().map(|(x, v)| Json::Arr(vec![Json::from(x), Json::Num(v)])).collect())
    });
    Json::Arr(lists.collect())
}

/// The multipliers of [`multipliers_to_json`]'s array, which must hold a list for each of `signings` signings.
pub fn multipliers_from_json(json: &Json, signings: usize) -> Result<Vec<HashMap<u32, f64>>, String> {
    let lists = json.as_arr().ok_or("the multipliers are not an array")?;
    if lists.len() != signings {
        return Err(format!("{signings} signings but {} multiplier lists", lists.len()));
    }
    lists
        .iter()
        .map(|l| {
            let pairs = l.as_arr().ok_or("a signing's multipliers are not an array")?;
            pairs
                .iter()
                .map(|p| match (p.at(0).and_then(Json::as_u32), p.at(1).and_then(Json::as_f64)) {
                    (Some(x), Some(v)) => Ok((x, v)),
                    _ => Err("a multiplier is not [candidate, value]".to_string()),
                })
                .collect::<Result<HashMap<u32, f64>, String>>()
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------------------
// Evaluation

/// One evaluation of `L` at multipliers rounded to the grid.
#[derive(Clone)]
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
    /// Master solves that broke down: they ended optimal at a basis whose primal violation exceeds `1e-6`, or with a
    /// dual above `1e8` in magnitude, so that the duals they report are not to be trusted (each is restarted from the
    /// empty set once: [`Master::solve`]).
    pub breakdowns: u64,
    /// Evaluations of `L` whose value saturated at `i64::MAX` (multipliers far beyond every weight; see
    /// [`evaluate_grid`]).
    pub saturated: u64,
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
    /// Time in the exact checks, in the separation rounds (finding and merging cores), and in building masters and
    /// the changed extras.
    pub ms_exact: f64,
    pub ms_separate: f64,
    pub ms_build: f64,
}

/// The multipliers on the grid: `max(0, round(λ·S))`, a `NaN` taken as 0 and a value of `2^63` or more (`λ` from
/// about `1.4·10^14`, or infinite) as `i64::MAX`, which is still a multiplier, being at least 0.
fn grid(lam: &[Vec<f64>]) -> Vec<Vec<i64>> {
    lam.iter().map(|l| l.iter().map(|&v| (v * SCALE as f64).round().max(0.0) as i64).collect()).collect()
}

/// `L` at `lam` (one vector per extra, indexed like its elements), exactly.
pub fn evaluate<const W: usize>(ctx: &Ctx<'_, W>, fam: &Family, lam: &[Vec<f64>], stats: &mut Stats) -> Eval {
    let lhat = grid(lam);
    evaluate_grid(ctx, fam, &lhat, stats)
}

/// `L` at integer multipliers `lhat` (scaled by [`SCALE`], at least 0).
///
/// The multipliers proposed can lie far beyond every weight (a master of the R2 refit whose basis broke down proposed
/// some above `10^50`, which [`grid`] takes as `i64::MAX`), so the sums saturate instead of wrapping around: each is
/// exact unless its true value lies beyond `i64`.  A reweighted weight `x·S − Σ_k λ̂_{k,x}` is then `i64::MIN`, not
/// positive either, so the intersection gets exactly the elements of positive reweighted weight, with their weights.
/// Those are at most `x·S < 2^29` (`n < 2^13`, the solver's largest board), as the crate needs: it packs a path's
/// length, at most the sum of the weights (below `2^42`), times `2^16` with the path's number of arcs into one `i64`.
/// A greedy's value and `L` are then `i64::MAX`, still a bound, since every promotion set weighs less than
/// `n²·S < 2^42`.  Wrapping around instead gave an element with two or more such multipliers a positive weight (near
/// `2^63`, or a wrong one just above `x·S`), on which the crate's distances overflowed, and could make `L` wrong.
pub fn evaluate_grid<const W: usize>(ctx: &Ctx<'_, W>, fam: &Family, lhat: &[Vec<i64>], stats: &mut Stats) -> Eval {
    let mut c = ctx.weight.clone();
    for (ex, l) in fam.extras.iter().zip(lhat) {
        for (&e, &v) in ex.elem.iter().zip(l) {
            assert!(v >= 0, "multipliers are at least 0");
            c[e] = c[e].saturating_sub(v);
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
    q.sort_unstable();
    let mut value = mi;
    let mut sets = Vec::with_capacity(fam.extras.len());
    let t = Instant::now();
    for (ex, l) in fam.extras.iter().zip(lhat) {
        let (g, kept) = ex.m.greedy(l);
        value = value.saturating_add(g);
        sets.push(kept.into_iter().map(|i| ex.elem[i]).collect::<Vec<_>>());
        stats.pricings_greedy += 1;
    }
    stats.ms_greedy += t.elapsed().as_secs_f64() * 1e3;
    for s in &mut sets {
        s.sort_unstable();
    }
    stats.saturated += u64::from(value == i64::MAX);
    Eval { value, q, sets }
}

/// The heaviest set independent on both sides `A` and `B` of at most `cap` of the elements `order` (ascending by weight
/// `c`, all positive), by the crate's intersection on the elements renumbered in that order: its weight and its
/// elements (indices into the GF(2) elements).  `R` is the words of a block vector in the context.
fn intersection<const W: usize, const R: usize>(ctx: &Ctx<'_, W>, order: &[usize], c: &[i64]) -> (i64, Vec<usize>) {
    if order.is_empty() || ctx.cap == 0 {
        return (0, Vec::new());
    }
    assert_eq!(ctx.words, R, "a context of {} words solved with {R}", ctx.words);
    let fm = ctx.fm;
    let vectors = |v: &[u64]| -> Vec<u64> {
        order.iter().flat_map(|&e| v[e * ctx.words..e * ctx.words + R].iter().copied()).collect()
    };
    let block = |m: &super::frames::BlockMatroid| -> Vec<u8> { order.iter().map(|&e| m.block[e] as u8).collect() };
    let (block_a, block_b) = (block(&fm.a), block(&fm.b));
    let (vector_a, vector_b) = (vectors(&ctx.va), vectors(&ctx.vb));
    let weight: Vec<i64> = order.iter().map(|&e| c[e]).collect();
    let n = order.len();
    let p = Problem {
        block: [&block_a, &block_b],
        vector: [vector_a.as_chunks::<R>().0, vector_b.as_chunks::<R>().0],
        weight: &weight,
    };
    let mut meet = Meet::<R>::new(n, [fm.a.words.len(), fm.b.words.len()]);
    let live: Vec<u32> = (0..n as u32).collect();
    meet.solve(&p, &live, &[], ctx.cap, i64::MAX, |_| {});
    (meet.value, (0..n).filter(|&i| meet.contains(i as u32)).map(|i| order[i]).collect())
}

// ---------------------------------------------------------------------------------------------------------
// Ranks and the polytopes of the extras

/// The rank in an extra (of the span of the classes) of its elements at the positions `positions`.
fn rank_at(ex: &Extra, positions: impl Iterator<Item = usize>) -> usize {
    let mut basis = Basis3::new(ex.m.rank);
    positions.filter(|&i| basis.try_insert(ex.m.vecs[i].clone())).count()
}

/// The rank in an extra of the GF(2) elements `set` (those of class 0 and those it does not have count 0).
fn rank(ex: &Extra, set: &[usize]) -> usize {
    rank_at(ex, set.iter().filter_map(|&e| ex.position(e)))
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

/// `set` closed in an extra among the GF(2) elements `within`: `set` and every element of `within` whose class lies in
/// the span of `set`'s classes, ascending.  Its rank is `set`'s, so its rank inequality is at least as violated.
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

/// The rank inequalities of the extras that the master's point `z` violates, found cheaply: in each extra, the most
/// violated set of its elements by decreasing `z` (the level sets of `z`) and each support set, each closed among the
/// elements where `z` is positive (a closed set is at least as violated).
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

/// What the exact check of one extra found: the largest point `y ≤ z` of its polytope (on its elements,
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
    // The elements of T in no dependent subset of T (those whose removal lowers the rank of T) are free of the rest:
    // the polytope is the product of theirs, [0, 1] each, and the rest's, so the LP is over the rest.
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

/// A maximal independent subset of `set` in an extra (its members in order, each kept if independent of
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

/// A rank inequality `z(S) ≤ r_k(S)` of extra `k` in the master: its set (GF(2) elements, ascending), rank
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
    /// For the tests: breakdowns to inject, one into each of the next optimal solves (`Lp::corrupt`).
    #[cfg(test)]
    inject: usize,
}

/// How a master solve ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Solved {
    Optimal,
    /// The deadline came first.
    Deadline,
    /// It broke down twice running (see [`Stats::breakdowns`]): the second time after a restart from the empty set.
    Broken,
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
            #[cfg(test)]
            inject: 0,
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

    /// Adds the rank inequality of `set` in extra `k` (the current point may violate it: the next solve starts
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

    /// The master's multipliers, one vector per extra indexed like its elements: `λ_{k,x}` is the sum of the duals of
    /// extra `k`'s inequalities whose sets hold `x`.
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

    /// Solves the master within the deadline.  A solve that ends optimal at a basis that has broken down (a primal
    /// violation above [`BROKEN_VIOLATION`] or a dual above [`BROKEN_DUAL`]; rounding in the simplex, which `lp.rs`
    /// does not guard against) is counted and solved again from the empty set's basis, with the same columns and
    /// inequalities; if that breaks down too, the solve is [`Solved::Broken`].
    fn solve(&mut self, deadline: Instant, stats: &mut Stats) -> Solved {
        let t = Instant::now();
        let before = self.lp.stats();
        let mut restarted = false;
        let solved = loop {
            if !self.optimize(deadline, stats) {
                break Solved::Deadline;
            }
            #[cfg(test)]
            if self.inject > 0 {
                self.inject -= 1;
                self.lp.corrupt();
            }
            if self.lp.violation() <= BROKEN_VIOLATION && self.lp.largest_dual() <= BROKEN_DUAL {
                break Solved::Optimal;
            }
            stats.breakdowns += 1;
            if restarted {
                break Solved::Broken;
            }
            self.lp.reset(&[self.empty]);
            restarted = true;
        };
        let after = self.lp.stats();
        stats.pivots += after.pivots - before.pivots;
        stats.degenerate += after.degenerate - before.degenerate;
        stats.repairs += after.repairs - before.repairs;
        stats.solves += 1;
        stats.ms_simplex += t.elapsed().as_secs_f64() * 1e3;
        stats.max_rows = stats.max_rows.max(self.lp.rows());
        stats.max_s = stats.max_s.max(after.max_s);
        solved
    }

    /// Runs the simplex within the deadline; whether it ended optimal.
    fn optimize(&mut self, deadline: Instant, stats: &mut Stats) -> bool {
        let mut since = 0;
        loop {
            if self.lp.solve(500) == Status::Optimal {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            // A solve that runs on is cycling in rounding (the objective does not move): start again from the empty
            // set, which every inequality admits.
            since += 500;
            if since >= STALL {
                self.lp.reset(&[self.empty]);
                stats.restarts += 1;
                since = 0;
            }
        }
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
    /// The bracket closed (certified; in search mode without separation, nothing priced out and no cheap cut was
    /// violated) and separation (if on) found nothing new.
    Converged,
    /// A search-mode fit ended: nothing prices out, no cheap inequality is violated, separation found no new core.
    Fitted,
    /// The bound reached the target (a node, cut).
    Target,
    Seconds,
    Pricings,
    Rounds,
    /// The master broke down again after its restart ([`Master::solve`]); the lowest `L` so far is kept.
    Breakdown,
}

/// What a bound computation found.
pub struct Outcome {
    /// `⌊L⌋` at the lowest `L` evaluated, over every family the computation went through: a valid bound.
    pub bound: i64,
    /// The bracket of the final family's LP: its lowest `L`, and the last certified lower bound (the weight of a
    /// point of the LP's polytope; 0 if none was certified).
    pub l_best: f64,
    pub lb: f64,
    pub rounds: usize,
    pub stop: Stop,
    /// The lowest `L` ever evaluated, over every family: its signings and multipliers (`fit.bound` gives `bound`).
    pub fit: Fit,
    /// The master's support at the end: (value, set as candidate numbers).
    pub support: Vec<(f64, Vec<u32>)>,
    /// The final master's columns as candidate numbers (ascending each): the support first, by decreasing value,
    /// then its other nonempty columns in its order; at most [`SAVED_COLUMNS`].
    pub master_columns: Vec<Vec<u32>>,
    /// The final master's rank inequalities: (index of the extra in the final family, candidates ascending, rank).
    pub master_cuts: Vec<(usize, Vec<u32>, usize)>,
    pub stats: Stats,
}

/// The lowest `L` evaluated so far, scaled by [`SCALE`], and the fit that gave it.
struct Best {
    scaled: i64,
    fit: Fit,
}

impl Best {
    /// Keeps `fam` at the multipliers `lam` if the value `L = value` (scaled) of that evaluation is the lowest yet.
    fn offer(&mut self, value: i64, fam: &Family, lam: &[Vec<f64>]) {
        if value < self.scaled {
            self.scaled = value;
            self.fit = Fit::of(fam, lam);
        }
    }
}

/// The subgradient of `L` at the evaluation `ev`, `g_{k,x} = [x ∈ I_k] − [x ∈ Q]` (one vector per extra, indexed like
/// its elements), and its squared norm.
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

/// A signing as a search keeps it: nothing merges into it (`mergeable` false, no cores), so its frame's independence
/// stays.
pub fn frozen(s: &Signing) -> Signing {
    Signing { mergeable: false, cores: Vec::new(), ..s.clone() }
}

/// One computation's state, which [`bound`] and [`refit_r2`] drive through the same phases: the family and its
/// master, the lowest `L` of all (with its fit) and of the family with its multipliers, the cores found, the
/// budgets, the stop.
struct Fitter<'r, 'a, const W: usize> {
    ctx: &'r Ctx<'a, W>,
    settings: &'r Settings,
    stats: &'r mut Stats,
    fam: Family,
    master: Master,
    /// The lowest `L` of all (scaled, with the fit that gave it), of the current family, and the family's multipliers
    /// there.
    best: Best,
    fam_best: i64,
    lam_best: Vec<Vec<f64>>,
    found: Vec<Found>,
    known: HashMap<Core, usize>,
    rounds: usize,
    /// The current family's last certified lower bound (0 if none).
    lb: f64,
    /// The exact checks' perturbations' generator.
    rng: u64,
    /// The bracket aimed at: `close`, or a share of the root's GF(2) gap if larger (the study counts a root whose
    /// bracket is within 1% of its gap as converged).
    close: f64,
    deadline: Instant,
    /// The pricings allowed, counted from `stats.pricings_mi` at `from`.
    budget: usize,
    from: u64,
    stop: Option<Stop>,
}

/// How [`Fitter::generate`] ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Generated {
    /// At a stop, left in `Fitter::stop`.
    Stopped,
    /// No column prices out and no cut is violated (without the exact checks: no cheap one).
    Settled,
    /// The same with the exact checks: the point is certified.
    Certified,
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
        let t = Instant::now();
        let master = Master::new(ctx, &fam, columns, cuts);
        stats.masters += 1;
        stats.ms_build += t.elapsed().as_secs_f64() * 1e3;
        Fitter {
            ctx,
            settings,
            fam,
            master,
            best: Best { scaled: i64::MAX, fit: Fit::default() },
            fam_best: i64::MAX,
            lam_best: start,
            found: Vec::new(),
            known: HashMap::new(),
            rounds: 0,
            lb: 0.0,
            rng: u64::from(ctx.n).wrapping_mul(0xD1B5_4A32_D192_ED03) | 1,
            close: ctx.gap.map_or(settings.close, |g| settings.close.max(settings.close_gap * g as f64)),
            deadline: t0 + Duration::from_secs_f64(settings.seconds),
            budget: settings.pricings,
            from: stats.pricings_mi,
            stop: None,
            stats,
        }
    }

    /// The pricings made since the budget started.
    fn spent(&self) -> usize {
        (self.stats.pricings_mi - self.from) as usize
    }

    /// `L` at `at` for the current family.
    fn eval_at(&mut self, at: &[Vec<f64>]) -> Eval {
        evaluate(self.ctx, &self.fam, at, self.stats)
    }

    /// Records the value of an evaluation at `at`: the lowest `L` of all with its fit, the trace, the target (a stop
    /// when the context stops there), and the family's lowest with its multipliers.  Whether it is the family's lowest.
    fn note(&mut self, value: i64, at: &[Vec<f64>]) -> bool {
        self.best.offer(value, &self.fam, at);
        let best = self.best.scaled.div_euclid(SCALE);
        if self.ctx.stop_at_target && best <= self.ctx.target {
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

    /// Polyak steps from the start towards the target (`settings.polyak`; one without extras), every
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
    /// exact checks if `certify`), or a stop (seconds, the pricing budget, the target, a breakdown), which is left in
    /// `stop`.
    fn generate(&mut self, certify: bool) -> Generated {
        let ctx = self.ctx;
        loop {
            if Instant::now() >= self.deadline {
                self.stop = Some(Stop::Seconds);
                return Generated::Stopped;
            }
            if self.spent() >= self.budget {
                self.stop = Some(Stop::Pricings);
                return Generated::Stopped;
            }
            match self.master.solve(self.deadline, self.stats) {
                Solved::Optimal => {}
                Solved::Deadline => {
                    self.stop = Some(Stop::Seconds);
                    return Generated::Stopped;
                }
                Solved::Broken => {
                    self.stop = Some(Stop::Breakdown);
                    return Generated::Stopped;
                }
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
                        return Generated::Stopped;
                    }
                    let ev = self.eval_at(&at);
                    self.record(&ev, &at);
                    if self.stop.is_some() {
                        return Generated::Stopped;
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
            // every extra's polytope.  Cheap cuts first, then the exact check.
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
                return Generated::Settled;
            }
            let mut zhat = z.clone();
            let mut found_cuts = Vec::new();
            let mut checks_capped = false;
            let t = Instant::now();
            for (k, ex) in self.fam.extras.iter_mut().enumerate() {
                let c = check(ex, &z, &support, self.deadline, &mut self.rng);
                self.stats.checks_exact += 1;
                self.stats.pivots_exact += c.pivots;
                self.stats.checks_capped += u64::from(c.capped);
                checks_capped |= c.capped;
                for (e, y) in c.y {
                    zhat[e] = zhat[e].min(y);
                }
                found_cuts.extend(c.cuts.into_iter().map(|set| (k, set)));
                if Instant::now() >= self.deadline {
                    self.stats.ms_exact += t.elapsed().as_secs_f64() * 1e3;
                    self.stop = Some(Stop::Seconds);
                    return Generated::Stopped;
                }
            }
            self.stats.ms_exact += t.elapsed().as_secs_f64() * 1e3;
            for (k, set) in found_cuts {
                new_cuts += usize::from(self.master.add_cut(&self.fam, k, set));
            }
            self.stats.cuts_exact += new_cuts as u64;
            if new_cuts > 0 {
                continue;
            }
            // The point less what the checks could not certify lies in every polytope: a certified lower bound.
            self.lb = zhat.iter().zip(&ctx.fm.elems).map(|(z, &x)| z * f64::from(x)).sum();
            if checks_capped && self.fam_best as f64 / SCALE as f64 - self.lb > self.close {
                // A check stopped at its column limit left the certificate short: check again, from the pools.
                continue;
            }
            return Generated::Certified;
        }
    }

    /// One separation round: the cores of the master's support columns, placed in core-derived signings (at most
    /// `per_frame(h)` mergeable signings of frame `h`, those opened seeded from index `seed_offset`); if any was
    /// placed, the family and the master rebuilt.  The cores placed and the signings opened.
    fn separate(&mut self, per_frame: &dyn Fn(usize) -> usize, seed_offset: usize) -> (usize, usize) {
        let ctx = self.ctx;
        let t = Instant::now();
        self.rounds += 1;
        let mut cores = Vec::new();
        for (_, set) in self.master.support() {
            let q: Vec<u32> = set.iter().map(|&e| ctx.fm.elems[e]).collect();
            cores.extend(cores_of(ctx.model, ctx.state, &q, self.settings.cores_per_frame));
        }
        self.stats.ms_separate += t.elapsed().as_secs_f64() * 1e3;
        self.take(cores, per_frame, seed_offset)
    }

    /// Places `cores` in core-derived signings in turn, as [`separate`](Self::separate) places the cores it finds (a
    /// core placed before is skipped), and rebuilds the family and the master if any was placed: the cores placed and
    /// the signings opened.
    fn take(&mut self, cores: Vec<Core>, per_frame: &dyn Fn(usize) -> usize, seed_offset: usize) -> (usize, usize) {
        let t = Instant::now();
        let mut changed = vec![false; self.fam.extras.len()];
        let mut opened: Vec<Signing> = Vec::new();
        let mut placed = 0;
        for core in cores {
            if let Some(&id) = self.known.get(&core) {
                if self.found[id].assigned.is_some() {
                    continue;
                }
            }
            let found = &mut self.found;
            let id = *self.known.entry(core.clone()).or_insert_with(|| {
                found.push(Found { core, assigned: None });
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
        let count = opened.len();
        self.stats.ms_separate += t.elapsed().as_secs_f64() * 1e3;
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

    /// The family changed (the signings `changed` took cores, `opened` are new): rebuild the changed extras and add
    /// the new ones; keep the master's columns (but those holding a candidate whose class has become 0) and its rank
    /// inequalities (their ranks taken again in the changed extras); the lowest `L` of the new family starts at the old
    /// multipliers.
    fn rebuild(&mut self, changed: &[bool], opened: Vec<Signing>) {
        let ctx = self.ctx;
        let t = Instant::now();
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
        self.stats.ms_build += t.elapsed().as_secs_f64() * 1e3;
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
            support.iter().map(|(_, s)| s).chain(others).take(SAVED_COLUMNS).map(|s| numbers(s)).collect();
        let master_cuts = self.master.cuts.iter().map(|c| (c.k, numbers(&c.set), c.rank)).collect();
        Outcome {
            bound: self.best.scaled.div_euclid(SCALE),
            l_best: self.fam_best as f64 / SCALE as f64,
            lb: self.lb,
            fit: self.best.fit,
            rounds: self.rounds,
            stop: self.stop.unwrap_or(Stop::Converged),
            support: support.into_iter().map(|(a, s)| (a, numbers(&s))).collect(),
            master_columns,
            master_cuts,
            stats: *self.stats,
        }
    }
}

/// The bound at a state with the extras of `signings` (and, with `separate`, the core-derived signings the
/// separation adds), the multipliers started at `start` (per signing, by candidate number; zero elsewhere).
pub fn bound<const W: usize>(
    ctx: &Ctx<'_, W>,
    signings: Vec<Signing>,
    start: Option<&[HashMap<u32, f64>]>,
    separate: bool,
    settings: &Settings,
) -> Outcome {
    bound_offered(ctx, signings, start, Vec::new(), separate, settings)
}

/// [`bound`] with the cores `offer` placed first, before the Polyak steps, as the separation places the cores it finds
/// (into the core-derived signings opened for them, at most `settings.per_frame` a frame; the signings given are
/// frozen or not as they come): the cores that a previous game's search met at its nodes, offered to the next game's
/// fit (design U4, Phase 0).  Any signing is valid, so a core that is not one at this state does no harm; one of a
/// frame the state lacks is left out.  The cores offered are the first of `Outcome::cores`.
pub fn bound_offered<const W: usize>(
    ctx: &Ctx<'_, W>,
    signings: Vec<Signing>,
    start: Option<&[HashMap<u32, f64>]>,
    offer: Vec<Core>,
    separate: bool,
    settings: &Settings,
) -> Outcome {
    let t0 = Instant::now();
    let mut stats = Stats::default();
    let fam = Family::new(ctx, signings);
    let lam = fam.multipliers_of(start.unwrap_or_default());
    let mut f = Fitter::new(ctx, fam, lam, &[], &[], settings, &mut stats, t0);
    let offer: Vec<Core> = offer.into_iter().filter(|core| core.frame < ctx.state.len()).collect();
    if !offer.is_empty() {
        f.take(offer, &|_| settings.per_frame, 0);
    }
    f.polyak();
    let search = settings.mode == FitMode::Search;
    // Whether the current family's point is to be certified: from the start without separation, else once a
    // separation round has found no new core (and a certified family whose separation finds none is the end).  In
    // search mode never: a round that finds nothing new is the end.
    let mut certify = !search && !separate;
    while f.stop.is_none() {
        let certified = f.generate(certify) == Generated::Certified;
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
        } else if search {
            f.stop = Some(Stop::Fitted);
            break;
        } else if certified {
            break;
        } else {
            certify = true;
        }
    }
    f.finish()
}

// ---------------------------------------------------------------------------------------------------------
// Warm states

/// A fitted state at a node: the frozen signings, the multipliers, the master's columns and cuts.  All by candidate number, so that a child can take them over.
#[derive(Clone, Debug, PartialEq)]
pub struct Warm {
    pub signings: Vec<Signing>,
    /// Per signing, its positive multipliers by candidate number.
    pub lambda: Vec<HashMap<u32, f64>>,
    /// Sets independent in every frame over GF(2) (candidate numbers, ascending), the master's columns.
    pub columns: Vec<Vec<u32>>,
    /// Rank inequalities (signing index, candidate numbers ascending); ranks are taken again where they are used.
    pub cuts: Vec<(usize, Vec<u32>)>,
}

impl Warm {
    /// The state of a root fit read from its file (or about to be written to it): the fit's signings frozen, its
    /// multipliers, the final master's columns and rank inequalities (their ranks left out).
    pub fn from_saved(saved: &SavedFit) -> Warm {
        Warm {
            signings: saved.fit.signings.iter().map(frozen).collect(),
            lambda: saved.fit.lambda.clone(),
            columns: saved.columns.clone(),
            cuts: saved.cuts.iter().map(|(k, set, _)| (*k, set.clone())).collect(),
        }
    }

    /// The state a child starts from: columns holding every candidate in `promoted_now` and none in `removed`, those
    /// taken out, members not in `live` (the child's GF(2) elements, ascending) dropped, columns with more than
    /// `max_add` members or none dropped, duplicates dropped; cuts with `promoted_now`, `removed` and members not in
    /// `live` taken out, empty ones and duplicates dropped; the multipliers restricted to `live`; signings
    /// unchanged.
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
        Warm { signings: self.signings.clone(), lambda, columns, cuts }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The node methods

/// How a refit ended: its pricings (evaluations of L after Tier 1) and why it stopped.
#[derive(Clone, Copy, Debug)]
pub struct Trace {
    pub pricings: usize,
    pub stop: Stop,
}

/// Tier 1: one evaluation of L at `warm`'s multipliers, `fam` being built from `warm.signings` in their order at the
/// node's state.  Returns ⌊L⌋ and the evaluation (its sets start a refit).
pub fn tier1<const W: usize>(ctx: &Ctx<'_, W>, fam: &Family, warm: &Warm, stats: &mut Stats) -> (i64, Eval) {
    assert_eq!(fam.extras.len(), warm.signings.len(), "a family of the warm state's signings");
    let ev = evaluate(ctx, fam, &fam.multipliers_of(&warm.lambda), stats);
    (ev.value.div_euclid(SCALE), ev)
}

/// The computation a refit at a node runs: over `fam` (built from `warm.signings` at the node) from
/// `warm`'s multipliers, whose evaluation `first` is recorded first (not a refit pricing), with a master of `warm`'s
/// columns and `first.q` and of `warm`'s rank inequalities (ranks taken in `fam`), with the budgets of `settings`.
fn warm_fitter<'r, 'a, const W: usize>(
    ctx: &'r Ctx<'a, W>,
    fam: Family,
    warm: &Warm,
    first: &Eval,
    settings: &'r Settings,
    stats: &'r mut Stats,
) -> Fitter<'r, 'a, W> {
    assert_eq!(fam.extras.len(), warm.signings.len(), "a family of the warm state's signings");
    let t0 = Instant::now();
    let elements = |set: &[u32]| -> Vec<usize> { set.iter().filter_map(|&x| ctx.element(x)).collect() };
    // Columns with a candidate of class 0 in the node's family are no set independent on both sides.
    let mut columns: Vec<Vec<usize>> =
        warm.columns.iter().map(|c| elements(c)).filter(|c| c.iter().all(|&e| !fam.banned[e])).collect();
    columns.push(first.q.clone());
    // A rank inequality is over its extra's elements; one whose rank is its size holds at every master point.
    let cuts: Vec<(usize, Vec<usize>)> = warm
        .cuts
        .iter()
        .map(|(j, set)| (*j, elements(set).into_iter().filter(|&e| fam.extras[*j].position(e).is_some()).collect()))
        .filter(|(j, set): &(usize, Vec<usize>)| rank(&fam.extras[*j], set) < set.len())
        .collect();
    let start = fam.multipliers_of(&warm.lambda);
    let mut f = Fitter::new(ctx, fam, start.clone(), &columns, &cuts, settings, stats, t0);
    f.record(first, &start);
    f
}

/// A refit's results from its computation `f`: the lowest ⌊L⌋ (with Tier 1's) and how it ended, its pricings being
/// those made since `stats.pricings_mi` was `before`.
fn refit_end<const W: usize>(f: Fitter<'_, '_, W>, before: u64) -> (i64, Trace) {
    let r = f.finish();
    (r.bound, Trace { pricings: (r.stats.pricings_mi - before) as usize, stop: r.stop })
}

/// The R2 refit, the warm master (design step S3.2): the column generation of [`bound`] in search mode from `warm` at
/// the node ([`warm_fitter`]), at most `k` pricings after Tier 1, stopping at the target (if the context stops there),
/// the budget, `settings.seconds`, a breakdown, or when nothing prices out and no cheap cut is violated.  Returns the
/// lowest ⌊L⌋ (with Tier 1's) and how it ended.
pub fn refit_r2<const W: usize>(
    ctx: &Ctx<'_, W>,
    fam: Family,
    warm: &Warm,
    first: Eval,
    k: usize,
    settings: &Settings,
    stats: &mut Stats,
) -> (i64, Trace) {
    let before = stats.pricings_mi;
    let settings = Settings { mode: FitMode::Search, pricings: k, ..settings.clone() };
    let mut f = warm_fitter(ctx, fam, warm, &first, &settings, stats);
    if f.stop.is_none() {
        f.generate(false);
    }
    refit_end(f, before)
}

#[cfg(test)]
mod tests {
    use super::super::frames::frame_matroids;
    use super::*;
    use taxman::bits::Bits;
    use taxman::board::Board;
    use taxman::game::Optimal;
    use taxman::gf2::{Gf2Bound, Gf2Graph};
    use taxman::search::{Prepared, SearchTactics};

    /// Board `n` prepared as a search prepares it.
    fn prepared<const W: usize>(n: u32) -> Prepared<W> {
        Board::<W>::for_n(n).prepare(SearchTactics::default()).expect("prepare")
    }

    /// Board `n` prepared, and the known optimum promotion sum (the table of optimal games to 1300).
    fn game<const W: usize>(n: u32) -> (Prepared<W>, u64) {
        let opt = Optimal::built_in().score(n).expect("a built-in optimum") - Board::<W>::for_n(n).in_the_bag;
        (prepared(n), opt)
    }

    /// U4's test of the master's breakdowns, at the root of 600: a breakdown injected into a solve (`Lp::corrupt`, a
    /// basic value set to −1 after an optimal solve, as rounding drift leaves one) is detected and counted, and the
    /// solve from the empty set's basis reaches the master's optimum again; two running make the solve broken, and a
    /// family's generation stops there ([`Stop::Breakdown`]) with a valid bound kept.
    #[test]
    fn breakdowns_are_detected_and_recovered() {
        taxman::with_width!(600, W => breakdowns::<W>(600));
    }

    fn breakdowns<const W: usize>(n: u32) {
        let (p, opt) = game::<W>(n);
        let fm = frame_matroids(&p.model, &p.root, &p.candidates);
        let ctx = Ctx::new(n, &p.model, &p.root, &fm, p.max_total as usize, opt as i64, false);
        let signings = (1..p.root.len().min(4)).map(Signing::ones).collect();
        let settings = Settings { mode: FitMode::Search, seconds: 10.0, ..Settings::default() };
        let fitted = bound(&ctx, signings, None, true, &settings);
        let warm = Warm::from_saved(&SavedFit::of(n, opt, 0, "search", &fitted));
        let elements = |set: &[u32]| set.iter().filter_map(|&x| ctx.element(x)).collect::<Vec<usize>>();
        let columns: Vec<Vec<usize>> = warm.columns.iter().map(|c| elements(c)).collect();
        let cuts: Vec<(usize, Vec<usize>)> = warm.cuts.iter().map(|(k, set)| (*k, elements(set))).collect();
        assert!(!columns.is_empty() && !cuts.is_empty(), "n={n}: a master to break");

        let fam = Family::new(&ctx, warm.signings.clone());
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut stats = Stats::default();
        let mut master = Master::new(&ctx, &fam, &columns, &cuts);
        assert_eq!(master.solve(deadline, &mut stats), Solved::Optimal);
        let value = master.lp.value();
        master.inject = 1;
        assert_eq!(master.solve(deadline, &mut stats), Solved::Optimal, "a breakdown recovered from");
        assert_eq!(stats.breakdowns, 1, "the breakdown counted");
        assert!(master.lp.violation() <= BROKEN_VIOLATION && master.lp.largest_dual() <= BROKEN_DUAL);
        assert!((master.lp.value() - value).abs() <= 1e-6 * value.abs().max(1.0), "the optimal value again");
        master.inject = 2;
        assert_eq!(master.solve(deadline, &mut stats), Solved::Broken, "two breakdowns running");
        assert_eq!(stats.breakdowns, 3);

        let mut stats = Stats::default();
        let start = fam.multipliers_of(&warm.lambda);
        let mut f = Fitter::new(&ctx, fam, start.clone(), &columns, &cuts, &settings, &mut stats, Instant::now());
        let ev = f.eval_at(&start);
        f.record(&ev, &start);
        f.master.inject = 2;
        assert_eq!(f.generate(false), Generated::Stopped);
        assert_eq!(f.stop, Some(Stop::Breakdown));
        let r = f.finish();
        assert_eq!((r.stop, r.stats.breakdowns), (Stop::Breakdown, 2));
        assert!(r.bound >= opt as i64 && r.bound == ev.value.div_euclid(SCALE), "the bound kept");
    }

    /// A xorshift generator.
    struct Rng(u64);

    impl Rng {
        fn below(&mut self, n: u64) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 % n
        }
    }

    /// The intersection `evaluate_grid` makes, on the frame vectors of this module (`frame_matroids`), equals the
    /// crate's own bound on `Gf2Graph::reweighted` (the frame vectors the search builds) at random reweightings, in
    /// value and in the weight of the set it chooses, which has at most `cap` elements.  This is what the run-time
    /// check of the study (against a reference algorithm) was.
    fn intersection_matches_the_crates_bound<const W: usize>(n: u32, rounds: usize) -> usize {
        let p = prepared::<W>(n);
        let fm = frame_matroids(&p.model, &p.root, &p.candidates);
        let cap = p.max_total as usize;
        let ctx = Ctx::new(n, &p.model, &p.root, &fm, cap, 0, false);
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ u64::from(n));
        for round in 0..rounds {
            // Most elements keep their weight, some lose a random part of it, some all of it (and are left out).
            let c: Vec<i64> = ctx
                .weight
                .iter()
                .map(|&w| match rng.below(4) {
                    0 => w - rng.below(w as u64 + 1) as i64,
                    1 if round % 2 == 1 => w - rng.below(2 * w as u64) as i64,
                    _ => w,
                })
                .collect();
            let mut order: Vec<usize> = (0..ctx.len()).filter(|&e| c[e] > 0).collect();
            order.sort_by_key(|&e| (c[e], e));
            let (value, chosen) = match ctx.words {
                1 => intersection::<W, 1>(&ctx, &order, &c),
                2 => intersection::<W, 2>(&ctx, &order, &c),
                _ => intersection::<W, 3>(&ctx, &order, &c),
            };
            let (reweighted, labels) = graph.reweighted(|x| ctx.element(x).map_or(0, |e| c[e]));
            let all: Bits<W> = (1..=labels.len() as u32).collect();
            let mut bound = Gf2Bound::<W>::new(&reweighted);
            bound.reset(&Bits::EMPTY, &all);
            let expected = bound.evaluate(cap as u32, None).value as i64;
            assert_eq!(value, expected, "n={n}, round {round}: the intersection differs from the crate's bound");
            assert!(chosen.len() <= cap && chosen.iter().map(|&e| c[e]).sum::<i64>() == value, "n={n}: the set");
        }
        ctx.words
    }

    #[test]
    fn the_intersection_equals_the_crates_bound_at_random_weights() {
        let mut words = Vec::new();
        for n in [120, 936, 2700, 3300] {
            words.push(taxman::with_width!(n, W => intersection_matches_the_crates_bound::<W>(n, 12)));
        }
        eprintln!("the intersection checked at block vectors of {words:?} words");
    }

    /// The fits of `testdata/fits` give their bounds again at the roots they were fitted at: the study's own numbers
    /// (studies E1 and E2, the pilot), computed by its code with the intersection checked against a reference.  Among
    /// them 2700 and 3300, with frame vectors of more than 64 and more than 128 coordinates.
    #[test]
    fn the_study_fits_give_their_bounds_again() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/fits");
        for n in [987, 1215, 2700, 3300] {
            let saved = SavedFit::load(&dir.join(format!("n{n}.json"))).expect("a fit");
            taxman::with_width!(n, W => {
                let p = prepared::<W>(n);
                let fm = frame_matroids(&p.model, &p.root, &p.candidates);
                let ctx = Ctx::new(n, &p.model, &p.root, &fm, p.max_total as usize, saved.opt as i64, false);
                assert_eq!(saved.bound(&ctx), saved.bound, "n={n}: the fit's bound");
                eprintln!("n={n}: the fit's bound {} again, with blocks of {} words", saved.bound, ctx.words);
            });
        }
    }
}
