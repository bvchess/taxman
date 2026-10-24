//! `cycles`: do Taxman's flat-cycle cuts close the gap that the field-rank bounds leave?  A measurement of LP bounds
//! only, with no search; the question, the method and the results are in
//! solver/notebook/search/flat-cycle-cuts-on-the-matching-lp-2026-10/README.md.
//!
//! ```text
//! wall cycles [--known FILE] [--suite FILE] [--units K] [--seconds S] [--out FILE] N...
//! ```
//!
//! For each game `N`, its root and then its units in the tuning suite `--suite FILE` (the first `K` of them with
//! `--units K`; none without the file), one JSON line an instance.  `--seconds S` (default 30) is what each stage of
//! cuts may spend; `--out FILE` appends the lines to FILE as well and skips the instances already in it, so that a run
//! that was cut short goes on where it was.
//!
//! # The LP
//!
//! The *matching graph* of a node (the root is a node) has a vertex per number: the moves and factors of the node's
//! frames and its candidates.  A *mover* of frame `h` is a move of the frame or a candidate that a promotion makes a
//! move of `h` (a factor of frame `h + 1`, or a one-sided candidate of own frame `h`); it has an edge to each of its
//! maximal factors among frame `h`'s factors, its *live factors*.  A candidate is one vertex whether it pays in frame
//! `h` (promoted) or is paid with in frame `h + 1` (not).  The LP has a variable `x_e ∈ [0, 1]` per edge and a row
//! per vertex: `= 1` for a move, `≤ 1` for the others, and `Σ z ≤ max_add` with `z_x` the sum of a candidate's edges as
//! a mover, the objective `Σ x z_x`.  Without cuts it is the matching bound (`taxman::matchbound`), which `lp0`
//! reports and checks.
//!
//! Every promotable set `Q` is an integer point that satisfies every cut below: in each frame, the peel's order pays
//! each mover with a factor no mover left after it lists (the playability theorem (P2) and the peel-order lemma (P9)
//! of docs/theory/playability.md, and the bridge lemma (C1) of docs/theory/franklin-moniot.md), a matching of the frame
//! with no alternating cycle.  So each bound is an upper bound on the node's completions.
//!
//! # The cuts
//!
//! * *Cover cuts* `Σ_K z ≤ |K| − 1`, `K` a minimal conflict of the node (`FrameGraph::conflict_using`) among the
//!   candidates whose `z` is at least a threshold, as the anatomy of the gap separated them
//!   (solver/notebook/search/the-anatomy-of-the-gap-2026-10/README.md, finding 2).
//! * *Rank cuts* `Σ_T z ≤ r` for sets `T` of at most 26 candidates built from the tight cuts (the anatomy's), with `r`
//!   the largest integer below `Σ_T z`, made only when an exhaustive check proves that no promotable set holds more
//!   than `r` of `T` ([`Relax::exceeds`]).
//! * *Flat-cycle cuts* `Σ_C x ≤ |C|/2 − 1` over the edges of a cycle `C` of a frame's graph (Joswig and Pfetsch's
//!   cycle inequality): a matching with no alternating cycle has at most `|C|/2` edges of `C`, and all of them only when
//!   `C` alternates.  Every cycle of a frame is flat (its movers have one more prime factor than its factors).  The
//!   graph has no 4-cycles (two numbers with as many prime factors share at most one maximal factor), so the shortest
//!   are hexagons, which [`list_cycles`] lists with the octagons from the factor graph's triangles and 4-cycles; a
//!   hexagon is three numbers `X·pq, X·pr, X·qr` over `X·p, X·q, X·r`.  Stage (b) adds the violated ones through a mover
//!   with three live factors or more; stage (c) also separates any violated cycle exactly ([`Relax::shortest_cycles`]).
//!
//! # The stages and the bound
//!
//! (a) cover cuts until none is violated (`cover`), then cover and rank cuts (`a`); (b) adds the listed hexagons
//! (`b_hex`), then the octagons (`b`); (c) adds the separated cycles (`c`); and `c_all` separates every family
//! together.  (b) and (c) separate cycles alone, so that what they move is the cycles' doing; `c_all` shows what the
//! cover and rank cuts add at the cycles' point.  Each stage runs rounds (separate, add, re-solve) until a round adds
//! nothing or its seconds run out, and keeps the cuts of the stages before it.  The LP is solved by the A1 study's simplex
//! (`fields/lp.rs`) in floating point, and every bound reported is `⌊B⌋` for the weak-duality bound `B` of the duals
//! it ends with (the duals of `≤` rows clamped at 0, and each `x_e ≤ 1`), computed apart from the simplex: valid
//! whatever state the simplex is in, the lowest of a stage's evaluations.

use crate::fields::lp::{Lp, RowKind, Status};
use crate::{sample, set_up, Line, Options};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::{Duration, Instant};
use taxman::bits::Bits;
use taxman::bounds::max_additional;
use taxman::game::Optimal;
use taxman::gf2::{Gf2Bound, Gf2Graph};
use taxman::json::Json;
use taxman::matchbound::match_bound_once;
use taxman::model::{FrameGraph, FrameSet, Scratch};
use taxman::search::Prepared;

/// A value of `x` or `z` at most this is 0, and one at least `1 − ZERO` is 1.
const ZERO: f64 = 1e-6;
/// A cut is violated when its left side exceeds its right side by more than this.
const VIOLATED: f64 = 1e-4;
/// A cut is tight when its slack is below this.
const TIGHT: f64 = 1e-6;
/// The thresholds of the cover cuts' sets, and the conflicts taken from each set.
const THRESHOLDS: [f64; 7] = [0.999, 0.9, 0.75, 0.5, 0.3, 0.1, 0.01];
const CONFLICTS_PER_SET: usize = 12;
/// The rank cuts' sets: at most this many members, at most this many conflict computations to prove one, at most
/// this many added a round, within these seconds a round.
const RANK_MEMBERS: usize = 26;
const RANK_CONFLICTS: usize = 3000;
const RANK_PER_ROUND: usize = 60;
const RANK_ROUND_SECONDS: f64 = 3.0;
/// The most cycle cuts added a round, of each kind (listed, separated).
const CYCLES_PER_ROUND: usize = 200;
/// The cost of an artificial column that takes up a new cut's violation (`Lp::add_row_elastic`): above every dual a
/// cut can have, which is at most a few of the board's numbers.
const PENALTY: f64 = 1e6;
/// Pivots after which a solve is stopped (its duals still give a valid bound).
const PIVOTS: u64 = 1_000_000;
/// What a bound's floating-point evaluation may be off by, so that `⌊B + SLACK⌋` is at least `⌊B⌋` of the exact value.
const SLACK: f64 = 1e-4;
/// The default seconds of a stage.
const STAGE_SECONDS: f64 = 30.0;

// ---------------------------------------------------------------------------------------------------------
// The matching graph

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Role {
    Move,
    Factor,
    Cand,
}

/// An edge: a mover, one of its live factors (vertices), and the frame they are in.
#[derive(Clone, Copy, Debug)]
struct Edge {
    mover: usize,
    factor: usize,
    frame: usize,
}

/// The matching graph of a node (the module comment).
struct Graph {
    nums: Vec<u32>,
    role: Vec<Role>,
    edges: Vec<Edge>,
    /// Per vertex: the edges where it is the mover (to its live factors), and those where it is the factor.
    up: Vec<Vec<usize>>,
    down: Vec<Vec<usize>>,
    /// The candidates that can be promoted, as vertices.
    cands: Vec<usize>,
    frames: usize,
}

impl Graph {
    fn new<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, cands: &Bits<W>) -> Graph {
        let frames = state.frames();
        let mut g = Graph {
            nums: Vec::new(),
            role: Vec::new(),
            edges: Vec::new(),
            up: Vec::new(),
            down: Vec::new(),
            cands: Vec::new(),
            frames: frames.len(),
        };
        let mut index: HashMap<u32, usize> = HashMap::new();
        let mut vertex = |g: &mut Graph, x: u32, role: Role| -> usize {
            *index.entry(x).or_insert_with(|| {
                g.nums.push(x);
                g.role.push(role);
                g.up.push(Vec::new());
                g.down.push(Vec::new());
                g.nums.len() - 1
            })
        };
        // The movers of each frame: its moves, then the candidates promoted into it.
        let mut movers: Vec<Vec<usize>> = vec![Vec::new(); frames.len()];
        for (h, frame) in frames.iter().enumerate() {
            for m in frame.moves.iter() {
                let v = vertex(&mut g, m, Role::Move);
                movers[h].push(v);
            }
            for y in frame.factors.iter() {
                vertex(&mut g, y, Role::Factor);
            }
        }
        for x in cands.iter() {
            let into = match model.own(x) {
                Some(j) => Some(j),
                None => {
                    let h = model.home(x);
                    (h >= 1 && h < frames.len() && model.contains_factor(state, x)).then(|| h - 1)
                }
            };
            let Some(into) = into else { continue };
            let v = vertex(&mut g, x, Role::Cand);
            g.role[v] = Role::Cand;
            g.cands.push(v);
            movers[into].push(v);
        }
        for (h, frame) in frames.iter().enumerate() {
            for &u in &movers[h] {
                for &f in model.graph().factors(g.nums[u]) {
                    let f = u32::from(f);
                    if frame.factors.contains(f) {
                        let fv = vertex(&mut g, f, Role::Factor);
                        let e = g.edges.len();
                        g.edges.push(Edge { mover: u, factor: fv, frame: h });
                        g.up[u].push(e);
                        g.down[fv].push(e);
                    }
                }
            }
        }
        g
    }

    fn vertices(&self) -> usize {
        self.nums.len()
    }

    /// The weight of an edge: its mover's number for a candidate, 0 for a move.
    fn weight(&self, e: usize) -> f64 {
        let u = self.edges[e].mover;
        if self.role[u] == Role::Cand {
            f64::from(self.nums[u])
        } else {
            0.0
        }
    }

    /// An edge for every move, to distinct factors: the peel's (a factor listed by one remaining move pays for it), as
    /// every frame of a node is winnable.  `None` if some move is left.
    fn moves_matching(&self) -> Option<Vec<usize>> {
        let nv = self.vertices();
        let (mut count, mut xor) = (vec![0u32; nv], vec![0usize; nv]);
        let moves: Vec<usize> = (0..nv).filter(|&v| self.role[v] == Role::Move).collect();
        for &m in &moves {
            for &e in &self.up[m] {
                let f = self.edges[e].factor;
                count[f] += 1;
                xor[f] ^= m;
            }
        }
        let mut stack: Vec<usize> = (0..nv).filter(|&f| count[f] == 1).collect();
        let mut paid = vec![usize::MAX; nv];
        while let Some(f) = stack.pop() {
            if count[f] != 1 {
                continue;
            }
            let m = xor[f];
            paid[m] = *self.up[m].iter().find(|&&e| self.edges[e].factor == f)?;
            for &e in &self.up[m] {
                let g = self.edges[e].factor;
                count[g] -= 1;
                xor[g] ^= m;
                if count[g] == 1 {
                    stack.push(g);
                }
            }
        }
        moves.iter().map(|&m| (paid[m] != usize::MAX).then_some(paid[m])).collect()
    }
}

// ---------------------------------------------------------------------------------------------------------
// The flat cycles listed from the arithmetic

/// A flat cycle of a frame: its edges, alternately a mover's two edges, and its movers.
#[derive(Clone, Debug)]
struct Cycle {
    edges: Vec<usize>,
    movers: Vec<usize>,
}

impl Cycle {
    /// The cycle through the factors `factors` (in order) and the movers `movers`, mover `i` joining factor `i` to
    /// factor `i + 1` (cyclically).
    fn through(g: &Graph, factors: &[usize], movers: &[usize]) -> Cycle {
        let edge = |u: usize, f: usize| -> usize {
            *g.up[u].iter().find(|&&e| g.edges[e].factor == f).expect("a mover lists the factors of its cycle")
        };
        let k = factors.len();
        let mut edges = Vec::with_capacity(2 * k);
        for i in 0..k {
            edges.push(edge(movers[i], factors[i]));
            edges.push(edge(movers[i], factors[(i + 1) % k]));
        }
        Cycle { edges, movers: movers.to_vec() }
    }

    fn key(&self) -> Vec<usize> {
        let mut key = self.edges.clone();
        key.sort_unstable();
        key
    }
}

/// The hexagons and octagons of every frame's graph, from its *factor graph*: the factors, two of them joined by the
/// mover that lists both (at most one: the graph has no 4-cycles).  A hexagon is a triangle of the factor graph whose
/// three edges are three movers, an octagon a 4-cycle whose four are four (a triangle or a 4-cycle of the factor graph
/// with a repeated mover is not a cycle of the graph).  The third value counts the pairs of factors that two movers
/// list, which is 0.
fn list_cycles(g: &Graph) -> (Vec<Cycle>, Vec<Cycle>, usize) {
    let (mut hexagons, mut octagons, mut doubled) = (Vec::new(), Vec::new(), 0);
    let nv = g.vertices();
    for h in 0..g.frames {
        // adj[f]: (g, mover) for the factors g that a mover of frame h lists together with f.
        let mut adj: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
        let mut pair: HashMap<(usize, usize), usize> = HashMap::new();
        for u in 0..nv {
            let lives: Vec<usize> =
                g.up[u].iter().filter(|&&e| g.edges[e].frame == h).map(|&e| g.edges[e].factor).collect();
            for (i, &a) in lives.iter().enumerate() {
                for &b in &lives[i + 1..] {
                    let key = (a.min(b), a.max(b));
                    if pair.insert(key, u).is_some() {
                        doubled += 1;
                        continue;
                    }
                    adj.entry(a).or_default().push((b, u));
                    adj.entry(b).or_default().push((a, u));
                }
            }
        }
        let label = |a: usize, b: usize| pair.get(&(a.min(b), a.max(b))).copied();
        let mut firsts: Vec<usize> = adj.keys().copied().collect();
        firsts.sort_unstable();
        for &f1 in &firsts {
            let around = &adj[&f1];
            // Triangles f1 < f2 < f3.
            for &(f2, u1) in around.iter().filter(|&&(f2, _)| f2 > f1) {
                for &(f3, u3) in around.iter().filter(|&&(f3, _)| f3 > f2) {
                    if let Some(u2) = label(f2, f3) {
                        if u1 != u2 && u2 != u3 && u1 != u3 {
                            hexagons.push(Cycle::through(g, &[f1, f2, f3], &[u1, u2, u3]));
                        }
                    }
                }
            }
            // 4-cycles f1 – f2 – f3 – f4 – f1 with f1 the least, through the wedges f1 – f2 – f3.
            let mut wedges: HashMap<usize, Vec<(usize, usize, usize)>> = HashMap::new();
            for &(f2, u1) in around.iter().filter(|&&(f2, _)| f2 > f1) {
                for &(f3, u2) in &adj[&f2] {
                    if f3 > f1 && u2 != u1 {
                        wedges.entry(f3).or_default().push((f2, u1, u2));
                    }
                }
            }
            for (&f3, list) in &wedges {
                for (i, &(f2, u1, u2)) in list.iter().enumerate() {
                    for &(f4, u4, u3) in &list[i + 1..] {
                        let movers = [u1, u2, u3, u4];
                        let distinct = (0..4).all(|a| (a + 1..4).all(|b| movers[a] != movers[b]));
                        if f2 != f4 && distinct {
                            octagons.push(Cycle::through(g, &[f1, f2, f3, f4], &movers));
                        }
                    }
                }
            }
        }
    }
    (hexagons, octagons, doubled)
}

/// A distance for the heap of [`lightest_path`], ordered by `total_cmp`.
#[derive(Clone, Copy, PartialEq)]
struct Dist(f64);

impl Eq for Dist {}

impl PartialOrd for Dist {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Dist {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// The lightest path from the factor `from` to the factor `to` over the pairs `aux` (per factor: the other factor,
/// the mover, the weight), using no pair of the mover `skip`, if it weighs at most `limit`: its factors from `from` to
/// `to`, and its movers, mover `i` joining factors `i` and `i + 1`.  Dijkstra's algorithm (the weights are at least 0).
#[allow(clippy::type_complexity)]
fn lightest_path(
    aux: &HashMap<usize, Vec<(usize, usize, f64)>>,
    from: usize,
    to: usize,
    skip: usize,
    limit: f64,
) -> Option<(Vec<usize>, Vec<usize>)> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let mut dist: HashMap<usize, f64> = HashMap::new();
    let mut pred: HashMap<usize, (usize, usize)> = HashMap::new();
    let mut heap = BinaryHeap::new();
    dist.insert(from, 0.0);
    heap.push(Reverse((Dist(0.0), from)));
    while let Some(Reverse((Dist(d), f))) = heap.pop() {
        if d > dist[&f] {
            continue;
        }
        if f == to {
            let (mut factors, mut movers) = (vec![to], Vec::new());
            let mut at = to;
            while at != from {
                let (before, mover) = pred[&at];
                movers.push(mover);
                factors.push(before);
                at = before;
            }
            factors.reverse();
            movers.reverse();
            return Some((factors, movers));
        }
        for &(next, mover, w) in aux.get(&f).map(Vec::as_slice).unwrap_or(&[]) {
            let nd = d + w;
            if mover == skip || nd > limit || dist.get(&next).is_some_and(|&old| old <= nd) {
                continue;
            }
            dist.insert(next, nd);
            pred.insert(next, (f, mover));
            heap.push(Reverse((Dist(nd), next)));
        }
    }
    None
}

/// Shortcuts a path (factors, movers as [`lightest_path`] gives them) at every mover it uses twice: a mover joining
/// factors `i, i + 1` and again `j, j + 1` (`i < j`) lists factor `j + 1` too, so it joins `i` to `j + 1` directly.
fn shortcut(factors: &mut Vec<usize>, movers: &mut Vec<usize>) {
    'again: loop {
        for i in 0..movers.len() {
            if let Some(off) = movers[i + 1..].iter().position(|&m| m == movers[i]) {
                let j = i + 1 + off;
                factors.drain(i + 1..=j);
                movers.drain(i + 1..=j);
                continue 'again;
            }
        }
        break;
    }
}

// ---------------------------------------------------------------------------------------------------------
// The relaxation and its cuts

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Cover,
    Rank,
    /// A flat cycle with this many movers.
    Cycle(usize),
}

/// A row of the LP: every coefficient is 1.
struct Row {
    rhs: f64,
    eq: bool,
    entries: Vec<usize>,
}

/// A cut: its kind, its row, and for a cut on promotions its candidates (vertices).
struct Cut {
    kind: Kind,
    row: usize,
    members: Vec<usize>,
}

/// The separations a stage runs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sep {
    Cover,
    Rank,
    Hexagons,
    /// The listed hexagons and octagons.
    Octagons,
    Shortest,
}

/// What a stage did.
#[derive(Default)]
struct Stage {
    bound: i64,
    rounds: usize,
    seconds: f64,
    capped: bool,
    added: HashMap<String, usize>,
}

/// The LP of a node with its cuts.
struct Relax<'a, const W: usize> {
    model: &'a FrameGraph,
    state: &'a FrameSet<W>,
    scratch: Scratch<W>,
    g: Graph,
    lp: Lp,
    rows: Vec<Row>,
    cuts: Vec<Cut>,
    /// The cuts on promotions made so far, by their candidates' numbers and right-hand side, and the cycles by their
    /// sorted edges.
    seen_cuts: HashSet<Vec<usize>>,
    seen_cycles: HashSet<Vec<usize>>,
    /// The cycle cuts made, by their number of movers, and those whose movers all have at most two live factors.
    cycles_by_movers: HashMap<usize, usize>,
    cycles_of_two: usize,
    /// The listed cycles through a mover with three live factors or more, and whether each is in the LP.
    hexagons: Vec<(Cycle, bool)>,
    octagons: Vec<(Cycle, bool)>,
    x: Vec<f64>,
    /// The lowest bound evaluated.
    best: i64,
    /// The worst primal infeasibility and the pivots, over the solves.
    violation: f64,
    pivots: u64,
    pivot_limits: usize,
}

impl<'a, const W: usize> Relax<'a, W> {
    /// The LP of the node `state` with candidates `cands`, without its row on the count of promotions and its cuts.
    fn new(model: &'a FrameGraph, state: &'a FrameSet<W>, cands: &Bits<W>) -> Relax<'a, W> {
        let g = Graph::new(model, state, cands);
        let mut lp = Lp::new();
        for e in 0..g.edges.len() {
            lp.add_column(g.weight(e), &[]);
        }
        let mut rows = Vec::new();
        for v in 0..g.vertices() {
            let entries: Vec<usize> = g.up[v].iter().chain(&g.down[v]).copied().collect();
            if entries.is_empty() {
                continue;
            }
            let eq = g.role[v] == Role::Move;
            let kind = if eq { RowKind::Eq } else { RowKind::Le };
            lp.add_row(kind, 1.0, &entries.iter().map(|&e| (e, 1.0)).collect::<Vec<_>>());
            rows.push(Row { rhs: 1.0, eq, entries });
        }
        let basis = g.moves_matching().expect("the moves of a node's frames are matchable");
        lp.reset(&basis);
        let mut relax = Relax {
            model,
            state,
            scratch: Scratch::new(),
            g,
            lp,
            rows,
            cuts: Vec::new(),
            seen_cuts: HashSet::new(),
            seen_cycles: HashSet::new(),
            cycles_by_movers: HashMap::new(),
            cycles_of_two: 0,
            hexagons: Vec::new(),
            octagons: Vec::new(),
            x: Vec::new(),
            best: i64::MAX,
            violation: 0.0,
            pivots: 0,
            pivot_limits: 0,
        };
        relax.solve();
        relax
    }

    /// Adds the row `Σ z ≤ max_add` (the most promotions a completion makes) if it can bind.
    fn count_row(&mut self, max_add: u32) {
        if (max_add as usize) < self.g.cands.len() {
            let entries: Vec<usize> = self.g.cands.iter().flat_map(|&v| self.g.up[v].iter().copied()).collect();
            self.add_row(f64::from(max_add), entries);
            self.solve();
        }
    }

    fn add_row(&mut self, rhs: f64, entries: Vec<usize>) -> usize {
        let coefficients: Vec<(usize, f64)> = entries.iter().map(|&e| (e, 1.0)).collect();
        self.lp.add_row_elastic(rhs, &coefficients, PENALTY);
        self.rows.push(Row { rhs, eq: false, entries });
        self.rows.len() - 1
    }

    /// Solves the LP, reads its point and evaluates the bound of its duals.
    fn solve(&mut self) -> i64 {
        let before = self.lp.stats().pivots;
        if self.lp.solve(PIVOTS) == Status::PivotLimit {
            self.pivot_limits += 1;
        }
        self.pivots += self.lp.stats().pivots - before;
        self.violation = self.violation.max(self.lp.violation());
        self.x = (0..self.g.edges.len()).map(|e| self.lp.primal(e).clamp(0.0, 1.0)).collect();
        let bound = self.dual_bound();
        self.best = self.best.min(bound);
        bound
    }

    /// `⌊B⌋` for the weak-duality bound `B = Σ_r b_r y_r + Σ_e max(0, c_e − Σ_{r ∋ e} y_r)` of the LP's duals, those of
    /// the `≤` rows clamped at 0: for every point of the LP (so every promotable set), `c·x = Σ_e (c_e − y·a_e) x_e +
    /// y·(A x) ≤ B`, as `0 ≤ x_e ≤ 1`, `y·(A x) ≤ y·b` on the `≤` rows (`y ≥ 0`) and `=` on the others.
    fn dual_bound(&self) -> i64 {
        let duals = self.lp.duals();
        let mut reduced: Vec<f64> = (0..self.g.edges.len()).map(|e| self.g.weight(e)).collect();
        let mut value = 0.0;
        for (r, row) in self.rows.iter().enumerate() {
            let y = if row.eq { duals[r] } else { duals[r].max(0.0) };
            if y == 0.0 {
                continue;
            }
            value += row.rhs * y;
            for &e in &row.entries {
                reduced[e] -= y;
            }
        }
        value += reduced.iter().map(|&d| d.max(0.0)).sum::<f64>();
        (value + SLACK).floor() as i64
    }

    /// `z_x` of every candidate vertex (0 for the other vertices).
    fn z(&self) -> Vec<f64> {
        let mut z = vec![0.0; self.g.vertices()];
        for &v in &self.g.cands {
            z[v] = self.g.up[v].iter().map(|&e| self.x[e]).sum();
        }
        z
    }

    fn lhs(&self, row: usize) -> f64 {
        self.rows[row].entries.iter().map(|&e| self.x[e]).sum()
    }

    fn bits(&self, members: &[usize]) -> Bits<W> {
        members.iter().map(|&v| self.g.nums[v]).collect()
    }

    /// Adds the cut `Σ_members z ≤ rhs` unless it was made before; whether it was added.
    fn add_promotion_cut(&mut self, kind: Kind, mut members: Vec<usize>, rhs: usize) -> bool {
        members.sort_unstable();
        if !self.seen_cuts.insert(self.cut_key(&members, rhs)) {
            return false;
        }
        let entries: Vec<usize> = members.iter().flat_map(|&v| self.g.up[v].iter().copied()).collect();
        let row = self.add_row(rhs as f64, entries);
        self.cuts.push(Cut { kind, row, members });
        true
    }

    /// The key of the cut `Σ_members z ≤ rhs` among those made.
    fn cut_key(&self, members: &[usize], rhs: usize) -> Vec<usize> {
        let mut key: Vec<usize> = members.iter().map(|&v| self.g.nums[v] as usize).collect();
        key.sort_unstable();
        key.push(rhs);
        key
    }

    /// Adds the cycle cut of `cycle` unless it was made before; whether it was added.
    fn add_cycle_cut(&mut self, cycle: &Cycle) -> bool {
        if !self.seen_cycles.insert(cycle.key()) {
            return false;
        }
        let k = cycle.movers.len();
        let row = self.add_row((k - 1) as f64, cycle.edges.clone());
        self.cuts.push(Cut { kind: Kind::Cycle(k), row, members: Vec::new() });
        *self.cycles_by_movers.entry(k).or_default() += 1;
        if cycle.movers.iter().all(|&u| self.g.up[u].len() <= 2) {
            self.cycles_of_two += 1;
        }
        true
    }

    // -----------------------------------------------------------------------------------------------
    // Cover cuts

    fn cover_cuts(&mut self) -> usize {
        let z = self.z();
        let mut found: Vec<Vec<usize>> = Vec::new();
        let by_num: HashMap<u32, usize> = self.g.cands.iter().map(|&v| (self.g.nums[v], v)).collect();
        for t in THRESHOLDS {
            let mut set: Bits<W> = self.g.cands.iter().filter(|&&v| z[v] >= t).map(|&v| self.g.nums[v]).collect();
            for _ in 0..CONFLICTS_PER_SET {
                let Some(k) = self.model.conflict_using(&mut self.scratch, self.state, &set, true) else { break };
                let members: Vec<usize> = k.iter().map(|x| by_num[&x]).collect();
                let lhs: f64 = members.iter().map(|&v| z[v]).sum();
                if lhs > (members.len() - 1) as f64 + VIOLATED {
                    found.push(members);
                }
                set.remove(k.max().expect("a conflict has a member"));
            }
        }
        let mut added = 0;
        for members in found {
            let rhs = members.len() - 1;
            added += self.add_promotion_cut(Kind::Cover, members, rhs) as usize;
        }
        added
    }

    // -----------------------------------------------------------------------------------------------
    // Rank cuts

    /// Whether some promotable set holds more than `r` members of `t`: `Some(true)` or `Some(false)` when settled,
    /// `None` when the budget of conflict computations ran out first.  A greedy promotable subset first; then the
    /// exhaustion of the module comment: a subset of a promotable set is promotable, so such a set is `t` less at most
    /// `|t| − r − 1` members, and every unpromotable set has a member out (one of its conflict's), tried in turn, each
    /// branch keeping the members tried before it.
    fn exceeds(&mut self, t: &[usize], z: &[f64], r: usize) -> Option<bool> {
        let mut order = t.to_vec();
        order.sort_by(|&a, &b| z[b].total_cmp(&z[a]));
        let mut greedy = Bits::<W>::EMPTY;
        for &v in &order {
            let with = greedy.with(self.g.nums[v]);
            if self.model.can_promote_all_using(&mut self.scratch, self.state, &with) {
                greedy = with;
            }
        }
        if greedy.len() as usize > r {
            return Some(true);
        }
        let mut budget = RANK_CONFLICTS;
        let all = self.bits(t);
        self.branch(all, Bits::EMPTY, t.len() - r - 1, &mut budget)
    }

    fn branch(&mut self, set: Bits<W>, kept: Bits<W>, out: usize, budget: &mut usize) -> Option<bool> {
        if self.model.can_promote_all_using(&mut self.scratch, self.state, &set) {
            return Some(true);
        }
        if out == 0 {
            return Some(false);
        }
        if *budget == 0 {
            return None;
        }
        *budget -= 1;
        let Some(conflict) = self.model.conflict_using(&mut self.scratch, self.state, &set, true) else {
            return Some(true);
        };
        let mut kept = kept;
        for x in conflict.iter() {
            if kept.contains(x) {
                continue;
            }
            if self.branch(set.without(x), kept, out - 1, budget)? {
                return Some(true);
            }
            kept.insert(x);
        }
        Some(false)
    }

    /// The candidate sets of the rank cuts: the groups of fractional candidates linked by tight cuts on promotions, and
    /// their cuts' supports; the unions of two or three tight cuts that share a fractional member.
    fn rank_sets(&self, z: &[f64]) -> Vec<Vec<usize>> {
        let fractional = |v: usize| z[v] > ZERO && z[v] < 1.0 - ZERO;
        let tight: Vec<&Cut> = self
            .cuts
            .iter()
            .filter(|c| matches!(c.kind, Kind::Cover | Kind::Rank))
            .filter(|c| self.rows[c.row].rhs - self.lhs(c.row) < TIGHT)
            .collect();
        let nv = self.g.vertices();
        let mut parent: Vec<usize> = (0..nv).collect();
        fn find(parent: &mut [usize], mut a: usize) -> usize {
            while parent[a] != a {
                parent[a] = parent[parent[a]];
                a = parent[a];
            }
            a
        }
        let mut by_member: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, c) in tight.iter().enumerate() {
            let fr: Vec<usize> = c.members.iter().copied().filter(|&v| fractional(v)).collect();
            for w in fr.windows(2) {
                let (a, b) = (find(&mut parent, w[0]), find(&mut parent, w[1]));
                parent[a] = b;
            }
            for &v in &fr {
                by_member.entry(v).or_default().push(i);
            }
        }
        let mut groups: HashMap<usize, (Vec<usize>, HashSet<usize>)> = HashMap::new();
        let mut members: Vec<usize> = by_member.keys().copied().collect();
        members.sort_unstable();
        for &v in &members {
            let root = find(&mut parent, v);
            let entry = groups.entry(root).or_default();
            entry.0.push(v);
            entry.1.extend(by_member[&v].iter().copied());
        }
        let mut sets: Vec<Vec<usize>> = Vec::new();
        let union = |cuts: &[usize]| -> Vec<usize> {
            let mut all: Vec<usize> = cuts.iter().flat_map(|&i| tight[i].members.iter().copied()).collect();
            all.sort_unstable();
            all.dedup();
            all
        };
        for (fr, cut_ids) in groups.values() {
            sets.push(fr.clone());
            let ids: Vec<usize> = cut_ids.iter().copied().collect();
            sets.push(union(&ids));
        }
        for &v in &members {
            let ids = &by_member[&v];
            for (i, &a) in ids.iter().enumerate() {
                for (j, &b) in ids.iter().enumerate().skip(i + 1) {
                    sets.push(union(&[a, b]));
                    for &c in &ids[j + 1..] {
                        sets.push(union(&[a, b, c]));
                    }
                }
            }
        }
        sets.retain(|s| s.len() >= 2 && s.len() <= RANK_MEMBERS);
        sets.sort();
        sets.dedup();
        sets
    }

    fn rank_cuts(&mut self, deadline: Instant) -> usize {
        let z = self.z();
        let end = deadline.min(Instant::now() + Duration::from_secs_f64(RANK_ROUND_SECONDS));
        let mut scored: Vec<(f64, Vec<usize>, usize)> = self
            .rank_sets(&z)
            .into_iter()
            .filter_map(|t| {
                let l: f64 = t.iter().map(|&v| z[v]).sum();
                let r = ((l - VIOLATED).ceil() as usize).saturating_sub(1);
                (r < t.len() && l - r as f64 > VIOLATED).then_some((l - r as f64, t, r))
            })
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut added = 0;
        for (_, t, r) in scored {
            if added == RANK_PER_ROUND || Instant::now() >= end {
                break;
            }
            if self.seen_cuts.contains(&self.cut_key(&t, r)) {
                continue;
            }
            if self.exceeds(&t, &z, r) == Some(false) && self.add_promotion_cut(Kind::Rank, t, r) {
                added += 1;
            }
        }
        added
    }

    // -----------------------------------------------------------------------------------------------
    // Cycle cuts

    /// The violated listed cycles not yet in the LP (the hexagons, and with `octagons` the octagons too), the most
    /// violated first.
    fn listed_cuts(&mut self, octagons: bool) -> usize {
        let x = &self.x;
        let violation = |c: &Cycle| c.edges.iter().map(|&e| x[e]).sum::<f64>() - (c.movers.len() - 1) as f64;
        let mut found: Vec<(f64, bool, usize)> = Vec::new();
        for (i, (c, added)) in self.hexagons.iter().enumerate() {
            let v = violation(c);
            if !added && v > VIOLATED {
                found.push((v, false, i));
            }
        }
        if octagons {
            for (i, (c, added)) in self.octagons.iter().enumerate() {
                let v = violation(c);
                if !added && v > VIOLATED {
                    found.push((v, true, i));
                }
            }
        }
        found.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut added = 0;
        for &(_, oct, i) in found.iter().take(CYCLES_PER_ROUND) {
            let list = if oct { &mut self.octagons } else { &mut self.hexagons };
            list[i].1 = true;
            let cycle = list[i].0.clone();
            added += self.add_cycle_cut(&cycle) as usize;
        }
        added
    }

    /// The violated flat cycles of any length, separated exactly: a cycle's cut is violated when the sum over its movers
    /// of `1 − x_{u,f} − x_{u,f'}` (its two edges at `u`, each term at least 0 by `u`'s row) is below 1.  For each
    /// mover `u` and pair of its edges, the shortest path between the two factors over the other movers' pairs gives the
    /// lightest cycle through that pair; a path that uses a mover twice is shortcut there (which makes it no heavier, by
    /// the mover's row), so the cycle is simple.
    fn shortest_cycles(&mut self) -> usize {
        let g = &self.g;
        let x = &self.x;
        let mut found: Vec<(f64, Vec<usize>, Vec<usize>)> = Vec::new();
        let mut keys: HashSet<Vec<usize>> = HashSet::new();
        for h in 0..g.frames {
            // The pairs: per factor, (other factor, mover, weight), for the pairs that carry some x.
            let mut aux: HashMap<usize, Vec<(usize, usize, f64)>> = HashMap::new();
            let mut pairs: Vec<(usize, usize, usize, f64)> = Vec::new();
            for u in 0..g.vertices() {
                let ups: Vec<usize> = g.up[u].iter().copied().filter(|&e| g.edges[e].frame == h).collect();
                for (i, &e1) in ups.iter().enumerate() {
                    for &e2 in &ups[i + 1..] {
                        let w = 1.0 - x[e1] - x[e2];
                        if w < 1.0 - ZERO {
                            let (f1, f2) = (g.edges[e1].factor, g.edges[e2].factor);
                            aux.entry(f1).or_default().push((f2, u, w.max(0.0)));
                            aux.entry(f2).or_default().push((f1, u, w.max(0.0)));
                            pairs.push((u, f1, f2, w.max(0.0)));
                        }
                    }
                }
            }
            for &(u, f1, f2, w0) in &pairs {
                let limit = 1.0 - VIOLATED - w0;
                if limit < 0.0 {
                    continue;
                }
                // The path p[0] = f2, ..., p[k] = f1, its mover i joining p[i] and p[i + 1]; with u, the cycle
                // f1 –u– f2 – ... – f1.
                let Some((mut path, mut movers)) = lightest_path(&aux, f2, f1, u, limit) else { continue };
                shortcut(&mut path, &mut movers);
                path.pop();
                let mut factors = vec![f1];
                factors.extend(path);
                movers.insert(0, u);
                let cycle = Cycle::through(g, &factors, &movers);
                let key = cycle.key();
                let v = cycle.edges.iter().map(|&e| x[e]).sum::<f64>() - (cycle.movers.len() - 1) as f64;
                if v > VIOLATED && !self.seen_cycles.contains(&key) && keys.insert(key) {
                    found.push((v, factors, movers));
                }
            }
        }
        found.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut added = 0;
        for (_, factors, movers) in found.into_iter().take(CYCLES_PER_ROUND) {
            let cycle = Cycle::through(&self.g, &factors, &movers);
            added += self.add_cycle_cut(&cycle) as usize;
        }
        added
    }

    // -----------------------------------------------------------------------------------------------
    // Stages

    /// Rounds of the separations `seps` (each round runs them all on the same point, adds what they find and solves
    /// again) until one adds nothing or `seconds` run out.
    fn stage(&mut self, seps: &[Sep], seconds: f64) -> Stage {
        let start = Instant::now();
        let deadline = start + Duration::from_secs_f64(seconds);
        let mut stage = Stage::default();
        loop {
            if Instant::now() >= deadline {
                stage.capped = true;
                break;
            }
            let mut added = 0;
            for &sep in seps {
                let (name, count) = match sep {
                    Sep::Cover => ("cover", self.cover_cuts()),
                    Sep::Rank => ("rank", self.rank_cuts(deadline)),
                    Sep::Hexagons => ("listed", self.listed_cuts(false)),
                    Sep::Octagons => ("listed", self.listed_cuts(true)),
                    Sep::Shortest => ("separated", self.shortest_cycles()),
                };
                *stage.added.entry(name.to_owned()).or_default() += count;
                added += count;
            }
            if added == 0 {
                break;
            }
            stage.rounds += 1;
            self.solve();
        }
        stage.bound = self.best;
        stage.seconds = start.elapsed().as_secs_f64();
        stage
    }
}

// ---------------------------------------------------------------------------------------------------------
// The command

/// An instance: a game's root, or a unit of the tuning suite.
struct Instance<const W: usize> {
    id: String,
    promoted: Bits<W>,
    cands: Bits<W>,
}

/// What [`measure`] found at an instance.
struct Measured {
    line: Line,
    /// The bounds of the stages, in order: the LP with its row on the count of promotions (`lp_cap`), then [`STAGES`].
    bounds: Vec<i64>,
    lp0: i64,
    matching: u64,
    gf2: u64,
    doubled: usize,
    /// The hexagons listed, and those not of the shape `X·pq, X·pr, X·qr` ([`triangle`]).
    hexagons: usize,
    misshapen: usize,
}

/// The stages: their names and separations.  (b) and (c) add cycle cuts alone, so that what they move is the cycles'
/// doing; `c_all` then separates every family together.
const STAGES: [(&str, &[Sep]); 6] = [
    ("cover", &[Sep::Cover]),
    ("a", &[Sep::Cover, Sep::Rank]),
    ("b_hex", &[Sep::Hexagons]),
    ("b", &[Sep::Octagons]),
    ("c", &[Sep::Octagons, Sep::Shortest]),
    ("c_all", &[Sep::Octagons, Sep::Shortest, Sep::Cover, Sep::Rank]),
];

/// The prime factors of `n`, ascending, with multiplicity.
fn prime_factors(mut n: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut p = 2;
    while p * p <= n {
        while n.is_multiple_of(p) {
            out.push(p);
            n /= p;
        }
        p += 1;
    }
    if n > 1 {
        out.push(n);
    }
    out
}

pub(super) fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// Whether the three movers of a hexagon are `X·pq, X·pr, X·qr` for distinct primes `p, q, r` (with `X` their gcd).
fn triangle(movers: &[u32]) -> bool {
    let x = movers.iter().fold(0, |a, &b| gcd(a, b));
    let pairs: Vec<Vec<u32>> = movers.iter().map(|&u| prime_factors(u / x)).collect();
    let mut all: Vec<u32> = pairs.concat();
    all.sort_unstable();
    all.dedup();
    movers.len() == 3 && pairs.iter().all(|q| q.len() == 2 && q[0] != q[1]) && all.len() == 3
}

/// The LP bounds of the instance `inst` of game `n` (board `p`), each stage within `seconds`.  `target` is what the
/// gap is measured to: P* at a root, `need` (P* plus 1 less the promotions' sum) at a unit, as the
/// A1 measurements measured it.
fn measure<const W: usize>(p: &Prepared<W>, inst: &Instance<W>, target: Option<i64>, seconds: f64) -> Measured {
    let mut scratch = Scratch::new();
    let state = p.model.promote_all_using(&mut scratch, &p.root, &inst.promoted).expect("a unit can be promoted");
    let cands = inst.cands;
    let max_add = max_additional(&p.model, p.max_total, &state, &inst.promoted, &cands);
    let clock = Instant::now();
    let graph = Gf2Graph::new(&p.model, &state, &cands);
    let mut bound = Gf2Bound::<W>::new(&graph);
    bound.reset(&Bits::EMPTY, &cands);
    let gf2 = bound.evaluate(max_add, None).value;
    let gf2_s = clock.elapsed().as_secs_f64();
    let matching = match_bound_once(&p.model, &state, &cands);

    let clock = Instant::now();
    let mut relax = Relax::new(&p.model, &state, &cands);
    let lp0 = relax.best;
    relax.count_row(max_add);
    let lp_cap = relax.best;
    let build_s = clock.elapsed().as_secs_f64();
    let (hexagons, octagons, doubled) = list_cycles(&relax.g);
    let g = &relax.g;
    let through_three = |c: &Cycle| c.movers.iter().any(|&u| g.up[u].len() >= 3);
    let numbers = |c: &Cycle| -> Vec<u32> { c.movers.iter().map(|&u| g.nums[u]).collect() };
    let misshapen = hexagons.iter().filter(|c| !triangle(&numbers(c))).count();
    let (all_hex, all_oct) = (hexagons.len(), octagons.len());
    relax.hexagons = hexagons.into_iter().filter(through_three).map(|c| (c, false)).collect();
    let g = &relax.g;
    let through_three = |c: &Cycle| c.movers.iter().any(|&u| g.up[u].len() >= 3);
    relax.octagons = octagons.into_iter().filter(through_three).map(|c| (c, false)).collect();
    // The movers by their number of live factors (1, 2, 3, 4 or more): all of them, and the candidates.
    let mut by_live = [[0usize; 4]; 2];
    for (v, ups) in relax.g.up.iter().enumerate() {
        if !ups.is_empty() {
            let d = ups.len().clamp(1, 4) - 1;
            by_live[0][d] += 1;
            if relax.g.role[v] == Role::Cand {
                by_live[1][d] += 1;
            }
        }
    }

    let mut line = Line::of(p.n);
    line.text("id", &inst.id);
    line.field("promoted", inst.promoted.len()).field("promoted_sum", inst.promoted.sum());
    line.field("cands", relax.g.cands.len()).field("max_add", max_add);
    line.maybe("target", target).field("gf2", gf2).maybe("gap", target.map(|t| gf2 as i64 - t));
    line.field("matching", matching).field("lp0", lp0).field("lp_cap", lp_cap);
    let mut bounds = vec![lp_cap];
    let mut stages = Vec::new();
    for (name, seps) in STAGES {
        let stage = relax.stage(seps, seconds);
        line.field(name, stage.bound);
        bounds.push(stage.bound);
        stages.push((name, stage));
    }
    for (name, stage) in &stages {
        let mut added: Vec<(&String, &usize)> = stage.added.iter().collect();
        added.sort();
        let added: Vec<String> = added.iter().map(|(k, v)| format!("\"{k}\":{v}")).collect();
        line.seconds(&format!("{name}_s"), stage.seconds);
        line.field(&format!("{name}_rounds"), stage.rounds).field(&format!("{name}_capped"), stage.capped);
        line.field(&format!("{name}_added"), format!("{{{}}}", added.join(",")));
    }
    let mut by_movers: Vec<(usize, usize)> = relax.cycles_by_movers.iter().map(|(&k, &v)| (k, v)).collect();
    by_movers.sort_unstable();
    let by_movers: Vec<String> = by_movers.iter().map(|(k, v)| format!("\"{k}\":{v}")).collect();
    let array = |a: &[usize]| format!("[{}]", a.iter().map(usize::to_string).collect::<Vec<_>>().join(","));
    line.field("cycle_cuts_by_movers", format!("{{{}}}", by_movers.join(",")));
    line.field("cycle_cuts_of_two", relax.cycles_of_two);
    line.field("hexagons", all_hex).field("hexagons_3", relax.hexagons.len()).field("misshapen", misshapen);
    line.field("octagons", all_oct).field("octagons_3", relax.octagons.len()).field("doubled", doubled);
    line.field("movers_by_live", array(&by_live[0])).field("cands_by_live", array(&by_live[1]));
    line.field("vertices", relax.g.vertices()).field("edges", relax.g.edges.len()).field("rows", relax.rows.len());
    line.field("pivots", relax.pivots).field("pivot_limits", relax.pivot_limits);
    line.field("violation", format!("{:.2e}", relax.violation));
    line.seconds("gf2_s", gf2_s).seconds("build_s", build_s);
    Measured { line, bounds, lp0, matching, gf2, doubled, hexagons: all_hex, misshapen }
}

/// The ids of the lines of `out`, if it exists.
fn done_in(out: &Option<std::path::PathBuf>) -> Result<HashSet<String>, String> {
    let Some(path) = out.as_ref().filter(|path| path.exists()) else { return Ok(HashSet::new()) };
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text
        .lines()
        .filter_map(|line| Json::parse(line).ok())
        .filter_map(|json| json.get("id").and_then(Json::as_str).map(str::to_owned))
        .collect())
}

/// `wall cycles`: every instance of every game of `o`, a line each (the module comment).
pub fn run(o: &Options, optimal: &Optimal) -> Result<(), String> {
    let seconds = o.seconds.map_or(STAGE_SECONDS, |d| d.as_secs_f64());
    let done = done_in(&o.out)?;
    for &n in &o.games {
        taxman::with_width!(n, W => game::<W>(n, o, optimal, seconds, &done), else Err(crate::beyond(n)))?;
    }
    Ok(())
}

fn game<const W: usize>(
    n: u32,
    o: &Options,
    optimal: &Optimal,
    seconds: f64,
    done: &HashSet<String>,
) -> Result<(), String> {
    let (board, prepared) = set_up::<W>(n);
    let Some(p) = prepared else {
        let mut line = Line::of(n);
        line.field("trivial", true).print();
        return Ok(());
    };
    let opt = optimal.score(n).map(|score| (score - board.in_the_bag) as i64);
    let mut instances = vec![(Instance { id: format!("{n}-root"), promoted: Bits::EMPTY, cands: p.candidates }, opt)];
    // The suite's units in its order, every `step`th of them for `--units K` (at most K, spread over the list).
    let units = sample::suite_nodes::<W>(&o.params.suite, n)?;
    let step = o.units.map_or(1, |k| units.len().div_ceil(k.max(1)).max(1));
    for (k, s) in units.into_iter().enumerate().step_by(step) {
        let need = opt.map(|opt| opt + 1 - s.unit.promoted.sum() as i64);
        let unit = Instance { id: format!("{n}-suite-{k}"), promoted: s.unit.promoted, cands: s.unit.cands };
        instances.push((unit, need));
    }
    for (inst, target) in instances.iter().filter(|(inst, _)| !done.contains(&inst.id)) {
        let mut measured = measure(&p, inst, *target, seconds);
        let root = inst.promoted.is_empty();
        let mut faults = Vec::new();
        if measured.lp0 != measured.matching as i64 {
            faults.push(format!("the LP without cuts is {}, the matching bound {}", measured.lp0, measured.matching));
        }
        if let (true, Some(opt)) = (root, opt) {
            if let Some(&low) = measured.bounds.iter().chain([&(measured.gf2 as i64)]).filter(|&&b| b < opt).min() {
                faults.push(format!("a bound of {low} below P* {opt}"));
            }
        }
        if measured.doubled > 0 {
            faults.push(format!("{} pairs of factors listed by two movers", measured.doubled));
        }
        if measured.misshapen > 0 {
            faults.push(format!("{} of {} hexagons not X·pq, X·pr, X·qr", measured.misshapen, measured.hexagons));
        }
        let faults: Vec<String> = faults.iter().map(|f| format!("\"{f}\"")).collect();
        measured.line.field("faults", format!("[{}]", faults.join(",")));
        let text = measured.line.render();
        println!("{text}");
        if let Some(path) = &o.out {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            writeln!(file, "{text}").map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// Tests: `cargo test --features studies` runs them

#[cfg(test)]
mod tests {
    use super::*;
    use taxman::board::Board;
    use taxman::search::SearchTactics;

    /// At the root of a game: the LP without cuts is the matching bound, every stage's bound is at most the one before
    /// and at least P*, no two movers share two factors, and every hexagon is `X·pq, X·pr, X·qr`.
    fn the_bounds_hold_at_the_root<const W: usize>(n: u32) {
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default()).expect("prepare");
        let opt = (Optimal::built_in().score(n).expect("a built-in optimum") - board.in_the_bag) as i64;
        let inst = Instance { id: "root".into(), promoted: Bits::EMPTY, cands: p.candidates };
        let m = measure(&p, &inst, Some(opt), 0.3);
        assert_eq!(m.lp0, m.matching as i64);
        assert!(m.bounds.windows(2).all(|w| w[1] <= w[0]), "{:?}", m.bounds);
        assert!(m.bounds.iter().all(|&b| b >= opt), "{:?} against {opt}", m.bounds);
        assert!(m.gf2 as i64 >= opt);
        assert_eq!((m.doubled, m.misshapen), (0, 0));
        assert!(m.hexagons > 0, "no hexagon at {n}");
    }

    #[test]
    fn a_hexagon_is_a_triangle_of_primes_over_a_base() {
        assert!(triangle(&[30, 42, 70]) && triangle(&[12, 20, 30]) && triangle(&[60, 84, 140]));
        assert!(!triangle(&[30, 42, 105 * 2 * 2]) && !triangle(&[12, 18, 30]));
    }

    #[test]
    fn the_bounds_hold_at_the_root_of_600() {
        taxman::with_width!(600, W => the_bounds_hold_at_the_root::<W>(600));
    }
}
