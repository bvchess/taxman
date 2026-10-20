//! The matching bound: an upper bound on the sum of the promotions a search state can still make, from relaxing
//! winnability to matchability.  It is the bound of every node of the search ([`crate::search`]).
//!
//! # The relaxation
//!
//! A frame is winnable when some order takes all its moves (see [`crate::model`]).  Each move of a winning order
//! takes at least one factor that is present when it is taken, and taking it removes that factor; so giving each
//! move one such factor gives the moves *distinct* maximal factors: a winnable frame is *matchable*.  Not
//! conversely: the moves 12, 20 and 30 over the factors 4, 6 and 10 are matchable (12→4, 20→10, 30→6), but
//! whichever is taken first takes two factors and leaves one for the other two.
//!
//! Promoting a set `Q` of candidates from a state succeeds exactly when every frame is winnable with all of `Q`
//! promoted at once (see [`FrameGraph::winnable`]).  So for a promotable `Q` there is an assignment that gives every
//! move of every frame `i` a distinct maximal factor among frame `i`'s factors and every `x ∈ Q` a distinct
//! maximal factor among the factors of frame `home(x) − 1`, using no member of `Q` as a factor.  The *matching
//! bound* is the largest `sum(Q)` over the `Q ⊆ cands` that have such an assignment: at least the sum of the best
//! promotable completion.
//!
//! The frames hold distinct numbers, so the assignment is a matching in the graph whose vertices are the numbers
//! and whose edges join each possible mover `u` (a move, or a candidate) to its maximal factors `u/p` in the
//! frame it pays in.  A candidate is a single vertex whether it pays (it is promoted) or is paid with (it is
//! not), so "no member of `Q` is a factor" holds by itself.  Every edge changes the number of prime factors by
//! one, so the graph is bipartite, and the bound is a maximum-weight bipartite matching in which every move must
//! be matched and an edge whose mover is a candidate weighs that candidate.
//!
//! The bound ignores the frames' freedoms and the limit on the number of promotions (`max_total`, a node's
//! `max_add`).  Both only remove completions, so it remains an upper bound.  The Java solver's own bound does respect
//! them, and the smaller of the two would be a bound too; but it never pruned where the matching bound did not, so the
//! search does not compute it.
//!
//! # Two solvers
//!
//! [`match_bound`] is the reference: it builds the graph of a state and solves a min-cost flow from scratch,
//! simply and slowly.  [`MatchBound`] is the solver for the search: it keeps an optimal matching and an optimal
//! dual solution for every node on the search's path and repairs them after each change, which is cheap
//! because the changes along a depth-first path are small.
//!
//! Besides the bound, the search asks the solver for the candidates its heaviest matching promotes, `Q*`
//! ([`MatchBound::chosen`]: if they can all be promoted they are the best completion, else their conflicts are
//! what the search branches on), and for the bound with one candidate left out, solved exactly
//! ([`MatchBound::value_excluding`]) or bounded from the potentials alone ([`MatchBound::bound_excluding`]).
//!
//! The levels and the Hungarian repairs below are generic over how a level weighs a promoted candidate (`Weighing`):
//! by its number for the matching bound, by weights of the level's own for the Lagrangian bound over nogoods, the
//! child module [`lagrange`], which keeps a second stack of levels the same way.
//!
//! [`MatchGraph`] is the graph of the search's root, with every move and factor of the root as a vertex; its
//! edges never change.  Every state of the search is the root with some candidates promoted, and its matching
//! problem is the root's graph with each vertex in one of three roles:
//!
//! * *required*: the root's moves and the promoted numbers; each must be matched to one of its factors, with
//!   weight 0;
//! * *candidate*: the candidates left; each can be matched to one of its factors (promoted, with weight its
//!   number), or pay for a mover above it, or stay free;
//! * *factor*: every other factor, which can only pay for a mover above it.
//!
//! An edge counts when its upper end is required or a candidate and its lower end is not required.  The frames
//! of a state reached by promotions are the root's minus the pairs a reduction drops (a move left with a single
//! factor takes it, see [`FrameGraph`]); the graph keeps those pairs, but every matching that matches the required
//! vertices pairs them up just as the reduction did, so the matching's maximum weight is the same.
//!
//! # The incremental solver
//!
//! Besides a maximum-weight matching, [`MatchBound`] keeps a potential `y` per vertex that proves it optimal
//! (LP duality for bipartite matching): `y ≥ 0` except on required vertices (whose constraint is an equality),
//! `y_u + y_w ≥ weight` on every edge, with equality on matched edges, and `y = 0` on every free vertex that is
//! not required.  A change of roles breaks these conditions at a few vertices at most:
//!
//! * a candidate that is not promoted in the matching stops being a candidate: nothing breaks;
//! * a candidate that is promoted stops being one: its matched edge goes, and both ends may now be free with a
//!   positive potential;
//! * a candidate that is promoted in the matching becomes required (the search promotes it): its potential
//!   drops by its number and nothing breaks;
//! * any other number becomes required: it is free and must be matched, and if it was paying for a mover, that
//!   mover is free now.
//!
//! Each vertex where the conditions break is repaired by one Hungarian search: a Dijkstra over reduced costs
//! that grows alternating paths from it until either a free vertex is reached or the potential of a vertex in
//! the tree that is not required reaches zero (that vertex can then give up its partner, or stay free if it is
//! the start); then it shifts the potentials of the tree and flips the alternating path.  It stops early when
//! the change is local, which is the common case.

pub mod lagrange;

use crate::bits::Bits;
use crate::model::{FrameGraph, FrameSet};
use std::collections::VecDeque;

// ---------------------------------------------------------------------------------------------------------
// Reference solver

/// The matching bound of `state` with the candidates `cands`, computed from scratch (see the module docs).
/// Candidates that are not factors of a frame behind frame 0 cannot be promoted and are ignored.  Returns 0 when
/// the moves of `state` cannot all be matched, since then nothing can be promoted from it.
///
/// This is the reference for [`MatchBound`]: a min-cost flow, solved by successive shortest paths with
/// Bellman–Ford, clear rather than fast (milliseconds on large games).
pub fn match_bound<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, cands: &Bits<W>) -> u64 {
    match_bound_weighted(model, state, cands, i64::from).map_or(0, |(value, _)| value as u64)
}

/// [`match_bound`] with each candidate `x` weighed by `weight(x)` instead of `x` (a candidate whose weight is not
/// positive is never promoted): the largest total weight of the candidates a matching promotes, and those
/// candidates; `None` when the moves of `state` cannot all be matched.  The reference for the Lagrangian bound
/// ([`lagrange`]), which changes the weights.
fn match_bound_weighted<const W: usize>(
    model: &FrameGraph,
    state: &FrameSet<W>,
    cands: &Bits<W>,
    weight: impl Fn(u32) -> i64,
) -> Option<(i64, Bits<W>)> {
    let frames = state.frames();
    // The movers: (number, frame it pays in, weight), with no weight for the moves, which must be matched.
    let mut movers: Vec<(u32, usize, Option<i64>)> = Vec::new();
    for (i, frame) in frames.iter().enumerate() {
        movers.extend(frame.moves.iter().map(|m| (m, i, None)));
        if i > 0 {
            let promotable = frame.factors.intersect(cands);
            movers.extend(promotable.iter().map(|x| (x, i - 1, Some(weight(x)))).filter(|m| m.2 > Some(0)));
        }
    }
    // Weighing each move more than all the candidates together makes a heaviest matching match all the moves
    // whenever some matching does.
    let big = 1 + movers.iter().filter_map(|&(_, _, weight)| weight).sum::<i64>();

    // One node per number after the source and the sink.  Numbers with an even number of prime factors hang off
    // the source and the others off the sink; every edge joins one of each, and runs from the first to the
    // second.
    let numbers: Vec<u32> = frames.iter().flat_map(|f| f.moves.iter().chain(f.factors.iter())).collect();
    let mut node = vec![usize::MAX; numbers.iter().max().map_or(0, |&x| x as usize + 1)];
    let mut net = Network::new(2 + numbers.len());
    for (k, &x) in numbers.iter().enumerate() {
        assert_eq!(node[x as usize], usize::MAX, "{x} is in two frames");
        node[x as usize] = 2 + k;
        if even_omega(x) {
            net.add(SOURCE, 2 + k, 0);
        } else {
            net.add(2 + k, SINK, 0);
        }
    }
    let mut edges = Vec::new(); // (arc, weight, the mover if it is a candidate)
    for &(u, i, weight) in &movers {
        for w in model.graph().factors(u).iter().map(|&w| u32::from(w)) {
            if frames[i].factors.contains(w) {
                let (a, b) = (node[u as usize], node[w as usize]);
                let (from, to) = if even_omega(u) { (a, b) } else { (b, a) };
                let weight_or_big = weight.unwrap_or(big);
                edges.push((net.add(from, to, -weight_or_big), weight_or_big, weight.map(|_| u)));
            }
        }
    }
    net.min_cost_flow();
    let matched: Vec<_> = edges.iter().filter(|&&(arc, _, _)| net.carries(arc)).collect();
    let moves = movers.iter().filter(|m| m.2.is_none()).count();
    if matched.iter().filter(|e| e.2.is_none()).count() < moves {
        return None;
    }
    let chosen: Bits<W> = matched.iter().filter_map(|e| e.2).collect();
    Some((matched.iter().map(|e| e.1).sum::<i64>() - big * moves as i64, chosen))
}

/// Whether `x` has an even number of prime factors, counted with multiplicity (by trial division).
fn even_omega(mut x: u32) -> bool {
    let mut even = true;
    let mut p = 2;
    while p * p <= x {
        while x.is_multiple_of(p) {
            x /= p;
            even = !even;
        }
        p += 1;
    }
    even != (x > 1)
}

const SOURCE: usize = 0;
const SINK: usize = 1;

/// A flow network whose arcs have capacity 1, for [`match_bound`].  Arc `e ^ 1` is the reverse of arc `e`.
struct Network {
    /// The arcs leaving each node.
    out: Vec<Vec<usize>>,
    /// The node each arc enters.
    head: Vec<usize>,
    /// Residual capacity of each arc.
    residual: Vec<i32>,
    cost: Vec<i64>,
}

impl Network {
    fn new(nodes: usize) -> Self {
        Network { out: vec![Vec::new(); nodes], head: Vec::new(), residual: Vec::new(), cost: Vec::new() }
    }

    /// Adds an arc of capacity 1 and its reverse; returns the arc.
    fn add(&mut self, from: usize, to: usize, cost: i64) -> usize {
        let e = self.head.len();
        self.head.extend([to, from]);
        self.residual.extend([1, 0]);
        self.cost.extend([cost, -cost]);
        self.out[from].push(e);
        self.out[to].push(e + 1);
        e
    }

    /// Whether the arc `e` (not a reverse arc) carries flow.
    fn carries(&self, e: usize) -> bool {
        self.residual[e] == 0
    }

    /// Successive shortest paths: while some source–sink path of negative cost is left, sends one unit along a
    /// cheapest one.  The residual network starts without cycles, and augmenting along a cheapest path never
    /// creates a negative cycle, so Bellman–Ford (here queue-based) finds cheapest paths throughout, and each
    /// flow on the way costs least among the flows of its size.  The cost per unit only rises from one path to
    /// the next, so the flow at the end costs least of all.
    fn min_cost_flow(&mut self) {
        let nodes = self.out.len();
        loop {
            let mut dist = vec![i64::MAX; nodes];
            let mut via = vec![usize::MAX; nodes];
            let mut queued = vec![false; nodes];
            let mut queue = VecDeque::from([SOURCE]);
            dist[SOURCE] = 0;
            while let Some(u) = queue.pop_front() {
                queued[u] = false;
                for &e in &self.out[u] {
                    let v = self.head[e];
                    if self.residual[e] > 0 && dist[u] + self.cost[e] < dist[v] {
                        dist[v] = dist[u] + self.cost[e];
                        via[v] = e;
                        if !queued[v] {
                            queued[v] = true;
                            queue.push_back(v);
                        }
                    }
                }
            }
            if dist[SINK] >= 0 {
                return; // also when the sink is out of reach
            }
            let mut v = SINK;
            while v != SOURCE {
                let e = via[v];
                self.residual[e] -= 1;
                self.residual[e ^ 1] += 1;
                v = self.head[e ^ 1];
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The graph of the root

/// No vertex.
const NONE: u16 = u16::MAX;

/// The matching graph of a search's root state (see the module docs), shared by every state of the search.
/// Immutable and `Sync`, so search threads can share one.
#[derive(Clone, Debug)]
pub struct MatchGraph {
    /// The number of each vertex.  Vertices are numbered in increasing order of their numbers, so a mover's
    /// vertex is larger than its factors'.
    num: Vec<u32>,
    /// The vertex of each number, `NONE` for numbers not in the root.
    vertex: Vec<u16>,
    /// The neighbours of vertex `v` are `adj[start[v]..start[v + 1]]`: first, up to `split[v]`, the factors it
    /// can pay with as a mover; then the movers it can pay for as a factor.
    start: Vec<u32>,
    split: Vec<u32>,
    adj: Vec<u16>,
    /// Whether each vertex is a move of the root.
    root_move: Vec<bool>,
}

impl MatchGraph {
    /// The graph of `root`: its moves and factors, each move joined to its maximal factors in its frame, and each
    /// factor of a frame behind frame 0 to its maximal factors in the frame in front of it.
    pub fn new<const W: usize>(model: &FrameGraph, root: &FrameSet<W>) -> Self {
        let frames = root.frames();
        let mut num: Vec<u32> = frames.iter().flat_map(|f| f.moves.iter().chain(f.factors.iter())).collect();
        num.sort_unstable();
        let count = num.len();
        assert!(count < usize::from(NONE), "{count} numbers are too many for a matching graph");
        let mut vertex = vec![NONE; num.last().map_or(0, |&x| x as usize + 1)];
        for (v, &x) in num.iter().enumerate() {
            assert_eq!(vertex[x as usize], NONE, "{x} is in two frames");
            vertex[x as usize] = v as u16;
        }

        // The moves of frame i pay in frame i; the factors of frame i ≥ 1 would pay in frame i − 1.
        let factors_in = |x: u32, i: usize| -> Vec<u16> {
            let present = model.graph().factors(x).iter().filter(|&&w| frames[i].factors.contains(u32::from(w)));
            present.map(|&w| vertex[usize::from(w)]).collect()
        };
        let mut below = vec![Vec::new(); count];
        let mut root_move = vec![false; count];
        for (i, frame) in frames.iter().enumerate() {
            for m in frame.moves.iter() {
                let v = usize::from(vertex[m as usize]);
                root_move[v] = true;
                below[v] = factors_in(m, i);
            }
            if i > 0 {
                for x in frame.factors.iter() {
                    below[usize::from(vertex[x as usize])] = factors_in(x, i - 1);
                }
            }
        }
        let mut above = vec![Vec::new(); count];
        for (v, list) in below.iter().enumerate() {
            for &w in list {
                above[usize::from(w)].push(v as u16);
            }
        }

        let mut start = Vec::with_capacity(count + 1);
        let mut split = Vec::with_capacity(count);
        let mut adj = Vec::new();
        for (down, up) in below.iter().zip(&above) {
            start.push(adj.len() as u32);
            adj.extend_from_slice(down);
            split.push(adj.len() as u32);
            adj.extend_from_slice(up);
        }
        start.push(adj.len() as u32);
        MatchGraph { num, vertex, start, split, adj, root_move }
    }

    /// Number of vertices: the moves and factors of the root.
    pub fn len(&self) -> usize {
        self.num.len()
    }

    pub fn is_empty(&self) -> bool {
        self.num.is_empty()
    }

    /// The vertex of the number `x`, if `x` is in the root.
    #[inline]
    fn vertex_of(&self, x: u32) -> Option<usize> {
        self.vertex.get(x as usize).filter(|&&v| v != NONE).map(|&v| usize::from(v))
    }

    /// The factors `v` can pay with as a mover.
    #[inline]
    fn below(&self, v: usize) -> &[u16] {
        &self.adj[self.start[v] as usize..self.split[v] as usize]
    }

    /// The movers `v` can pay for as a factor.
    #[inline]
    fn above(&self, v: usize) -> &[u16] {
        &self.adj[self.split[v] as usize..self.start[v + 1] as usize]
    }
}

// ---------------------------------------------------------------------------------------------------------
// One state: roles, matching, potentials

/// What a vertex can do in the current state (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Required,
    Candidate,
    Factor,
}

/// How the levels of a solver weigh a candidate that is promoted (an edge whose mover is that candidate): the
/// matching bound by its number ([`Numbers`]), a Lagrangian bound by weights of the level's own
/// ([`lagrange`]).  Every weight that a candidate's role allows is positive.
trait Weighing: Clone + std::fmt::Debug {
    /// The weight of the vertex `u` as a promoted candidate.
    fn of(&self, g: &MatchGraph, u: usize) -> i64;
    /// Makes `self` equal to `other` without allocating.
    fn copy_from(&mut self, other: &Self);
}

/// The matching bound's weighing: a candidate weighs its number.
#[derive(Clone, Copy, Debug, Default)]
struct Numbers;

impl Weighing for Numbers {
    #[inline(always)]
    fn of(&self, g: &MatchGraph, u: usize) -> i64 {
        i64::from(g.num[u])
    }

    #[inline(always)]
    fn copy_from(&mut self, _: &Self) {}
}

/// The solver's state at one node of the search path, with its candidates weighed by `K`.
#[derive(Clone, Debug)]
struct Level<const W: usize, K: Weighing = Numbers> {
    role: Vec<Role>,
    /// The vertex each vertex is matched to, `NONE` if free.
    mate: Vec<u16>,
    /// Potentials (the dual solution).
    y: Vec<i64>,
    /// The numbers whose role is candidate.
    cands: Bits<W>,
    /// The candidates matched to a factor (promoted in the matching), and their sum.
    chosen: Bits<W>,
    value: u64,
    /// Candidates to turn into factors before the next evaluation.
    dropped: Bits<W>,
    /// Vertices where the optimality conditions may be broken: free, and required or with a positive potential.
    pending: Vec<u16>,
    /// False once a required vertex was found that cannot be matched.
    feasible: bool,
    /// Whether anything may need repairing (drops or pending vertices) since the last settle.
    dirty: bool,
    /// How the candidates weigh.
    weighing: K,
}

impl<const W: usize, K: Weighing> Level<W, K> {
    fn new(size: usize, weighing: K) -> Self {
        Level {
            role: vec![Role::Factor; size],
            mate: vec![NONE; size],
            y: vec![0; size],
            cands: Bits::EMPTY,
            chosen: Bits::EMPTY,
            value: 0,
            dropped: Bits::EMPTY,
            pending: Vec::new(),
            feasible: true,
            dirty: true,
            weighing,
        }
    }

    /// Makes `self` equal to `other` without allocating.
    fn copy_from(&mut self, other: &Self) {
        self.role.copy_from_slice(&other.role);
        self.mate.copy_from_slice(&other.mate);
        self.y.copy_from_slice(&other.y);
        self.cands = other.cands;
        self.chosen = other.chosen;
        self.value = other.value;
        self.dropped = other.dropped;
        self.pending.clear();
        self.pending.extend_from_slice(&other.pending);
        self.feasible = other.feasible;
        self.dirty = other.dirty;
        self.weighing.copy_from(&other.weighing);
    }

    /// The weight of the edges from the mover `u` to its factors.
    #[inline]
    fn weight(&self, g: &MatchGraph, u: usize) -> i64 {
        if self.role[u] == Role::Candidate {
            self.weighing.of(g, u)
        } else {
            0
        }
    }

    /// Accounts for the matched edge `{u, w}` being added or removed: if its mover is a candidate, that
    /// candidate is promoted, or no longer is.
    #[inline]
    fn tally(&mut self, g: &MatchGraph, u: usize, w: usize, add: bool) {
        let mover = u.max(w);
        if self.role[mover] == Role::Candidate {
            let (x, weight) = (g.num[mover], self.weighing.of(g, mover) as u64);
            if add {
                self.value += weight;
                self.chosen.insert(x);
            } else {
                self.value -= weight;
                self.chosen.remove(x);
            }
        }
    }

    /// Unmatches the matched pair `{u, w}`, recording both as possibly breaking the optimality conditions.
    fn unmatch(&mut self, g: &MatchGraph, u: usize, w: usize) {
        self.tally(g, u, w, false);
        self.mate[u] = NONE;
        self.mate[w] = NONE;
        self.pending.extend([u as u16, w as u16]);
    }

    /// The root state with `promoted` promoted and the candidates `cands`, with a greedy matching and potentials
    /// that satisfy every condition except at the vertices left pending.
    fn reset(&mut self, g: &MatchGraph, promoted: &Bits<W>, cands: &Bits<W>) {
        self.cands = Bits::EMPTY;
        self.chosen = Bits::EMPTY;
        self.value = 0;
        self.dropped = Bits::EMPTY;
        self.pending.clear();
        self.feasible = true;
        self.dirty = true;
        for v in 0..g.len() {
            let x = g.num[v];
            self.role[v] = if g.root_move[v] || promoted.contains(x) {
                Role::Required
            } else if cands.contains(x) && !g.below(v).is_empty() && self.weighing.of(g, v) > 0 {
                self.cands.insert(x);
                Role::Candidate
            } else {
                Role::Factor
            };
            self.mate[v] = NONE;
            // Then the slack of every edge is its factor's potential: 0 for a factor, its weight for a candidate.
            self.y[v] = self.weight(g, v);
        }
        // Largest movers first, take an unmatched factor over a tight edge.
        for v in (0..g.len()).rev() {
            if self.role[v] == Role::Factor {
                continue;
            }
            let free = g.below(v).iter().map(|&w| usize::from(w)).find(|&w| {
                self.role[w] == Role::Factor && self.mate[w] == NONE // potential 0, so the edge is tight
            });
            match free {
                Some(w) => {
                    self.mate[v] = w as u16;
                    self.mate[w] = v as u16;
                    self.tally(g, v, w, true);
                }
                None => self.pending.push(v as u16),
            }
        }
        self.pending.reverse(); // repair the largest first
    }

    /// Makes `n` required (the search promotes it).
    fn promote(&mut self, g: &MatchGraph, n: u32) {
        let v = g.vertex_of(n).unwrap_or_else(|| panic!("{n} is not in the root"));
        self.dirty = true;
        self.dropped.remove(n);
        match self.role[v] {
            Role::Required => return,
            Role::Candidate if self.mate[v] != NONE && usize::from(self.mate[v]) < v => {
                // Promoted in the matching: it stays matched, and its edges now weigh 0 instead of its weight.
                self.tally(g, v, usize::from(self.mate[v]), false);
                self.y[v] -= self.weighing.of(g, v);
                self.role[v] = Role::Required;
                self.cands.remove(n);
                return;
            }
            _ => {}
        }
        // Free, or paying for a mover above, which has to find another factor.
        let m = self.mate[v];
        if m != NONE {
            self.unmatch(g, usize::from(m), v);
        }
        self.cands.remove(n);
        self.role[v] = Role::Required; // its potential is not negative, so its new edges (weight 0) are fine
        self.pending.push(v as u16);
    }

    /// Turns the dropped candidates into factors.
    fn apply_drops(&mut self, g: &MatchGraph) {
        let drops = self.dropped.intersect(&self.cands);
        self.dropped = Bits::EMPTY;
        if drops.is_empty() {
            return;
        }
        for x in drops.iter() {
            let v = g.vertex_of(x).expect("candidates are in the root");
            let w = self.mate[v];
            if w != NONE && usize::from(w) < v {
                self.unmatch(g, v, usize::from(w));
            }
            self.role[v] = Role::Factor;
        }
        self.cands -= drops;
    }

    /// [`MatchBound::bound_excluding`] for this level, settled and feasible: an upper bound on its value with `x`
    /// no longer a candidate, from its potentials alone (the argument is there; `x`'s weight is the level's).
    fn bound_excluding(&self, g: &MatchGraph, x: u32) -> u64 {
        let Some(v) = g.vertex_of(x).filter(|&v| self.role[v] == Role::Candidate) else {
            return self.value;
        };
        let w = self.mate[v];
        if w == NONE || usize::from(w) > v {
            return self.value; // not promoted: it pays for a mover above, or is free
        }
        let w = usize::from(w);
        // The least potential a factor `f` needs to pay for the movers above it that still count (not `x`).  It is
        // never more than `f`'s potential (every such edge `u–f` has slack: `weight(u) − y_u ≤ y_f`, and `f` is
        // not required), so the scan ends at a tight edge.  The same holds for `w`'s own factors below.
        let as_factor = |f: usize| -> i64 {
            let (most, mut least) = (self.y[f], 0);
            for u in g.above(f).iter().map(|&u| usize::from(u)) {
                if least == most {
                    break;
                }
                if u != v && self.role[u] != Role::Factor {
                    least = least.max(self.weight(g, u) - self.y[u]);
                }
            }
            least
        };
        let y_x = as_factor(v);
        let mut y_w = as_factor(w);
        if self.role[w] == Role::Candidate {
            // It can still be promoted itself, paying with its own factors.
            let own = self.weight(g, w);
            for f in g.below(w).iter().map(|&f| usize::from(f)) {
                if y_w == self.y[w] {
                    break;
                }
                if self.role[f] != Role::Required {
                    y_w = y_w.max(own - self.y[f]);
                }
            }
        }
        let bound = self.value as i64 - self.weighing.of(g, v) + y_x + y_w;
        bound.clamp(0, self.value as i64) as u64
    }

    /// The members `x` of the heaviest matching of this level, settled and feasible, whose bound without them
    /// ([`bound_excluding`](Self::bound_excluding)) plus `constant` is below `target`, leaving out those in
    /// `known` (forced already).
    fn forced(&self, g: &MatchGraph, target: i64, constant: i64, known: &Bits<W>) -> Bits<W> {
        let rest = self.chosen - *known;
        rest.iter().filter(|&x| self.bound_excluding(g, x) as i64 + constant < target).collect()
    }

    /// Restores the optimality conditions.
    fn settle(&mut self, g: &MatchGraph, search: &mut Search, stats: &mut MatchStats) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        self.apply_drops(g);
        while let Some(v) = self.pending.pop() {
            let v = usize::from(v);
            if !self.feasible {
                self.pending.clear();
                break;
            }
            if self.mate[v] != NONE || (self.role[v] != Role::Required && self.y[v] == 0) {
                continue;
            }
            stats.searches += 1;
            self.feasible = search.run(g, self, v);
            stats.scanned += search.tree.len() as u64;
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Hungarian search

/// [`Slot::tag`] values, valid for vertices stamped with the current epoch.
const REACHED: u8 = 1;
const DONE: u8 = 2;
const INNER: u8 = 3;

/// A priority queue of events `(time, code)` for a search whose times never go below the last one popped (as in
/// Dijkstra's algorithm): a radix heap.  Bucket `i > 0` holds the times whose highest bit differing from the last
/// popped time is bit `i − 1`, bucket 0 the times equal to it, as a set of codes ([`CodeSet`]).  Pops the smallest
/// `(time, code)`, like a binary heap of the pairs, so the search makes the same choices, as long as no pair is
/// pushed while it is waiting already: the search never does that (it pushes a vertex again only at a smaller
/// time, and the potential event of an inner vertex once).
#[derive(Clone, Debug)]
struct RadixHeap {
    last: u64,
    /// The codes of the events at time `last` (bucket 0), smallest first.
    now: CodeSet,
    /// Bit `i` set when bucket `i ≥ 1` is not empty.
    full: u64,
    /// Buckets 1 to 64 (index 0 unused).
    buckets: [Vec<(u64, u32)>; 65],
}

impl Default for RadixHeap {
    fn default() -> Self {
        RadixHeap { last: 0, now: CodeSet::default(), full: 0, buckets: std::array::from_fn(|_| Vec::new()) }
    }
}

impl RadixHeap {
    #[inline]
    fn bucket(&self, time: u64) -> usize {
        (u64::BITS - (time ^ self.last).leading_zeros()) as usize
    }

    fn clear(&mut self) {
        self.now.clear();
        while self.full != 0 {
            let i = self.full.trailing_zeros() as usize + 1;
            self.buckets[i].clear();
            self.full &= self.full - 1;
        }
        self.last = 0;
    }

    #[inline(always)]
    fn push(&mut self, time: u64, code: u32) {
        debug_assert!(time >= self.last, "time went back");
        match self.bucket(time) {
            0 => self.now.insert(code),
            i => {
                self.buckets[i].push((time, code));
                self.full |= 1 << (i - 1);
            }
        }
    }

    #[inline(always)]
    fn pop(&mut self) -> Option<(u64, u32)> {
        if self.now.is_empty() {
            if self.full == 0 {
                return None;
            }
            // Advance to the smallest time and spread its bucket over the lower ones (all of it moves down).
            let i = self.full.trailing_zeros() as usize + 1;
            let mut moved = std::mem::take(&mut self.buckets[i]);
            self.full &= !(1 << (i - 1));
            self.last = moved.iter().map(|e| e.0).min().expect("not empty");
            for &(time, code) in &moved {
                self.push(time, code);
            }
            moved.clear();
            self.buckets[i] = moved;
        }
        self.now.pop_min().map(|code| (self.last, code))
    }
}

/// Words of a [`CodeSet`]: the event codes of a graph are below `2 · 65536`, since it has fewer than 65535
/// vertices ([`NONE`] is not one).
const CODE_WORDS: usize = 2 * 65536 / 64;

/// A set of event codes popping its smallest member: a bitset under two summary levels, of fixed size, so that
/// an insertion needs no check.
#[derive(Clone, Debug)]
struct CodeSet {
    words: [u64; CODE_WORDS],
    mid: [u64; CODE_WORDS / 64],
    top: u64,
}

impl Default for CodeSet {
    fn default() -> Self {
        CodeSet { words: [0; CODE_WORDS], mid: [0; CODE_WORDS / 64], top: 0 }
    }
}

impl CodeSet {
    #[inline]
    fn insert(&mut self, code: u32) {
        debug_assert!((code as usize) < 64 * CODE_WORDS, "code {code} out of range");
        let w = (code >> 6) as usize % CODE_WORDS;
        debug_assert_eq!(self.words[w] >> (code & 63) & 1, 0, "code {code} pushed twice at the same time");
        self.words[w] |= 1 << (code & 63);
        self.mid[w >> 6] |= 1 << (w & 63);
        self.top |= 1 << (w >> 6);
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.top == 0
    }

    #[inline]
    fn pop_min(&mut self) -> Option<u32> {
        if self.top == 0 {
            return None;
        }
        let m = self.top.trailing_zeros() as usize;
        let w = (m << 6) | self.mid[m].trailing_zeros() as usize;
        let word = self.words[w];
        let bit = word.trailing_zeros();
        let rest = word & (word - 1);
        self.words[w] = rest;
        if rest == 0 {
            let mid = self.mid[m] & (self.mid[m] - 1);
            self.mid[m] = mid;
            if mid == 0 {
                self.top &= self.top - 1;
            }
        }
        Some(((w as u32) << 6) | bit)
    }

    fn clear(&mut self) {
        while self.top != 0 {
            let m = self.top.trailing_zeros() as usize;
            let mut mid = self.mid[m];
            while mid != 0 {
                self.words[(m << 6) | mid.trailing_zeros() as usize] = 0;
                mid &= mid - 1;
            }
            self.mid[m] = 0;
            self.top &= self.top - 1;
        }
    }
}

/// What the Hungarian search knows of a vertex, all in one place (one cache line holds four): valid while `stamp`
/// is the current epoch.
#[derive(Clone, Copy, Debug, Default)]
struct Slot {
    /// Distance from the start: when an inner vertex joined the tree; when an outer one was or can be reached.
    dist: i64,
    /// The epoch in which the vertex was last touched.
    stamp: u32,
    /// The inner vertex an outer one is reached from.
    pred: u16,
    /// Reached (an outer vertex with a tentative distance), done (an outer vertex in the tree), or inner.
    tag: u8,
}

/// Scratch space for the Hungarian search, reused from one search to the next.
#[derive(Clone, Debug, Default)]
struct Search {
    epoch: u32,
    /// Per vertex: the epoch it was last touched, tag, distance and predecessor (valid for the current epoch).
    slot: Vec<Slot>,
    /// Events by time: `2v` an outer vertex `v` reached, `2v + 1` the potential of the inner vertex `v` at zero.
    heap: RadixHeap,
    /// The vertices of the tree (inner, and done outer), whose potentials change at the end.
    tree: Vec<u16>,
}

impl Search {
    fn begin(&mut self, size: usize) {
        if self.slot.len() < size {
            self.slot.resize(size, Slot { dist: 0, stamp: 0, pred: NONE, tag: 0 });
        }
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.slot.iter_mut().for_each(|s| s.stamp = 0);
            self.epoch = 1;
        }
        self.heap.clear();
        self.tree.clear();
    }

    /// Repairs the optimality conditions at the free vertex `root` (required, or with a positive potential),
    /// keeping them everywhere else.  Inner vertices (the root, and the partners of the outer vertices in the
    /// tree) have their potentials lowered and outer vertices raised, all at the same rate, as in the Hungarian
    /// method; the search stops at the first event that settles the root:
    ///
    /// * the potential of an inner vertex that is not required reaches zero: that vertex gives up its partner
    ///   and the alternating path from the root to it flips (for the root itself, nothing flips);
    /// * a free outer vertex is reached: the alternating path from the root to it flips.
    ///
    /// Returns false when neither can happen: the root is required and no matching matches it with the others.
    fn run<const W: usize, K: Weighing>(&mut self, g: &MatchGraph, lv: &mut Level<W, K>, root: usize) -> bool {
        self.begin(g.len());
        self.add_inner(g, lv, root, 0);
        while let Some((t, code)) = self.heap.pop() {
            let t = t as i64;
            let v = (code >> 1) as usize;
            if code & 1 == 1 {
                self.shift(lv, t);
                if v != root {
                    let b = usize::from(lv.mate[v]);
                    lv.tally(g, v, b, false);
                    lv.mate[v] = NONE;
                    self.flip(g, lv, root, b);
                }
                return true;
            }
            let s = &mut self.slot[v];
            if s.tag != REACHED || s.dist != t {
                continue; // reached again sooner, or already in the tree
            }
            s.tag = DONE;
            self.tree.push(v as u16);
            let m = lv.mate[v];
            if m == NONE {
                self.shift(lv, t);
                self.flip(g, lv, root, v);
                return true;
            }
            self.add_inner(g, lv, usize::from(m), t);
        }
        false
    }

    /// Adds the inner vertex `a` to the tree at distance `d`, and reaches its neighbours over unmatched edges.
    #[inline(always)]
    fn add_inner<const W: usize, K: Weighing>(&mut self, g: &MatchGraph, lv: &Level<W, K>, a: usize, d: i64) {
        // The fields taken apart, so that the compiler knows that pushing an event changes neither the slots'
        // place nor the epoch, and keeps them in registers.
        let Search { epoch, slot, heap, tree } = self;
        let (epoch, slot) = (*epoch, slot.as_mut_slice());
        slot[a] = Slot { dist: d, stamp: epoch, pred: slot[a].pred, tag: INNER };
        tree.push(a as u16);
        let (role, ya, mate) = (lv.role[a], lv.y[a], lv.mate[a]);
        if role != Role::Required {
            heap.push((d + ya) as u64, (a as u32) << 1 | 1);
        }
        if role != Role::Factor {
            let base = d + ya - lv.weight(g, a);
            for &b in g.below(a) {
                if b != mate && lv.role[usize::from(b)] != Role::Required {
                    reach(slot, heap, epoch, usize::from(b), base + lv.y[usize::from(b)], a, d);
                }
            }
        }
        if role != Role::Required {
            let base = d + ya;
            for &b in g.above(a) {
                let b = usize::from(b);
                if b as u16 != mate && lv.role[b] != Role::Factor {
                    reach(slot, heap, epoch, b, base + lv.y[b] - lv.weight(g, b), a, d);
                }
            }
        }
    }

    /// Shifts the potentials of the tree for a search that ends at time `end`.
    fn shift<const W: usize, K: Weighing>(&self, lv: &mut Level<W, K>, end: i64) {
        for &v in &self.tree {
            let v = usize::from(v);
            let s = &self.slot[v];
            if s.tag == INNER {
                lv.y[v] -= end - s.dist;
            } else {
                lv.y[v] += end - s.dist;
            }
        }
    }

    /// Flips the alternating path from `root` to the outer vertex `b`, which is free.
    fn flip<const W: usize, K: Weighing>(&self, g: &MatchGraph, lv: &mut Level<W, K>, root: usize, mut b: usize) {
        loop {
            let a = usize::from(self.slot[b].pred);
            let next = lv.mate[a];
            if next != NONE {
                lv.tally(g, a, usize::from(next), false);
            }
            lv.mate[a] = b as u16;
            lv.mate[b] = a as u16;
            lv.tally(g, a, b, true);
            if a == root {
                return;
            }
            b = usize::from(next);
        }
    }
}

/// Reaches the outer vertex `b` from the inner vertex `a` (at distance `d`) at distance `nd`, in the search of
/// `epoch` (see [`Search::add_inner`]).
#[inline(always)]
fn reach(slot: &mut [Slot], heap: &mut RadixHeap, epoch: u32, b: usize, nd: i64, a: usize, d: i64) {
    debug_assert!(nd >= d, "negative reduced cost");
    let s = &mut slot[b];
    if s.stamp != epoch {
        s.stamp = epoch;
        s.tag = REACHED;
    } else if s.tag != REACHED || nd >= s.dist {
        debug_assert_ne!(s.tag, INNER, "the graph is not bipartite");
        return;
    }
    s.dist = nd;
    s.pred = a as u16;
    heap.push(nd as u64, (b as u32) << 1);
}

// ---------------------------------------------------------------------------------------------------------
// The solver for the search

/// Counters of the work [`MatchBound`] has done.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MatchStats {
    /// Calls of [`MatchBound::value`] and friends.
    pub evaluations: u64,
    /// Hungarian searches (repairs).
    pub searches: u64,
    /// Vertices those searches put in their trees.
    pub scanned: u64,
}

/// The matching bound kept up to date along a depth-first search path (see the module docs).  One per search
/// thread; it allocates only when the path gets deeper than it has been.
///
/// It holds a stack of levels, one per node on the path, each with an optimal matching and its proof.  Level 0
/// is the node the search starts from ([`reset`](Self::reset)); [`push`](Self::push) makes a child of the top
/// level, [`pop`](Self::pop) returns to the parent, [`remove_candidate`](Self::remove_candidate) shrinks the top
/// level's candidates, and [`value`](Self::value) gives the top level's bound.  `push` and `remove_candidate`
/// only record what changed; `value` does the work.
///
/// # Driving it from a depth-first search
///
/// As the searcher does ([`crate::search`]; `P` the node's promotions, `need = best + 1 − sum(P)`):
///
/// ```text
/// let graph = MatchGraph::new(&model, &root);      // once per search, shared by the threads
/// let mut mb = MatchBound::new(&graph);             // once per thread
/// mb.reset(&promoted, &cands);                      // level 0: the node a work unit starts from
///
/// // A step of the node on top of mb:
/// mb.take_kept();                                   // its level without its last branch, if one was kept
/// if mb.value() < need { mb.pop(); return; }        // the node's loop ends (no pop for level 0)
/// let q = mb.chosen();                              // the heaviest matching's candidates
/// mb.bound_excluding(x);                            // the bound without x ∈ q, from the potentials alone
/// // refinement, for the first branch of a conflict K ⊆ q:
/// mb.push_excluding(&{k₁});                         // the node without k₁, solved; then
/// mb.keep_solved(k₁);                               //   kept, in case strong branching asks for it
/// // strong branching, for each member x of K:
/// mb.value_excluding(x);                            // pushes a level without x and solves it (or takes the
///                                                   //   one kept for x); then
/// mb.pop();                                         //   drops it, or
/// mb.keep_excluded();                               //   keeps it for the node, which takes it next step
/// mb.remove_candidate(n);                           // the node goes on without n (unless that was kept)
/// mb.push_all(&added, &child_cands);                // the child that promotes `added` (n among them)
/// if mb.value() < need_of_child { mb.pop(); }       // pruned; else the child's steps run on top of it
/// ```
///
/// The rules: `push_all(added, child_cands)` makes the child obtained by promoting `added` from the top level,
/// where each member of `added` is one of the top level's candidates (or was removed from them since its last
/// evaluation) and `child_cands` is a subset of the top level's candidates; `pop` goes back once the child is
/// done.  Neither touches the parent, so the parent's loop continues where it was.  `remove_candidate(n)`
/// before pushing `n` is free as long as no evaluation of the parent comes between them; after one, the child
/// has to repair what the removal did.  A level kept by [`keep_excluded`](Self::keep_excluded) waits for its
/// parent to be the top level again ([`take_kept`](Self::take_kept)); children may come and go meanwhile, but
/// nothing may be done to the parent itself.  A level kept by [`keep_solved`](Self::keep_solved) is the same
/// work kept for later: the parent's level without one candidate, which `value_excluding` of that candidate
/// takes instead of solving it again, and which becomes the parent's kept level when the parent goes on without
/// it (`remove_candidate`); anything else done to the parent drops it.  The refinement of the bound by conflicts
/// pushes levels in the same way ([`push_excluding`](Self::push_excluding), [`push`](Self::push) with
/// [`candidates`](Self::candidates)) and pops them all, or keeps one solved.  The bound covers the state's own
/// moves and promotions only: the search adds `sum(P)` itself.
///
/// For a single state with no search, [`match_bound_once`] does all of this in one call.
#[derive(Clone, Debug)]
pub struct MatchBound<'g, const W: usize> {
    graph: &'g MatchGraph,
    levels: Vec<Level<W>>,
    /// `spares[d]` is a level solved ahead for level `d`, as `spare[d]` says.
    spares: Vec<Level<W>>,
    spare: Vec<Spare>,
    depth: usize,
    search: Search,
    stats: MatchStats,
}

/// What the spare level of a level of a [`MatchBound`] holds: work done ahead, which saves a repair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spare {
    None,
    /// The level without the candidate `x`, solved ([`MatchBound::keep_solved`]): `value_excluding(x)` takes it.
    Solved(u32),
    /// The level without the candidate its node goes on without ([`MatchBound::keep_excluded`]):
    /// [`MatchBound::take_kept`] makes it the level.
    Kept,
}

impl<'g, const W: usize> MatchBound<'g, W> {
    /// A solver for the states of `graph`'s search, positioned at the root with no candidates.
    pub fn new(graph: &'g MatchGraph) -> Self {
        let mut out = MatchBound {
            graph,
            levels: vec![Level::new(graph.len(), Numbers)],
            spares: Vec::new(),
            spare: Vec::new(),
            depth: 0,
            search: Search::default(),
            stats: MatchStats::default(),
        };
        out.reset(&Bits::EMPTY, &Bits::EMPTY);
        out
    }

    /// Starts over with level 0 at the state reached from the root by promoting `promoted` (which must succeed),
    /// with the candidates `cands`.  The first [`value`](Self::value) then solves it from scratch.
    pub fn reset(&mut self, promoted: &Bits<W>, cands: &Bits<W>) {
        self.depth = 0;
        self.spare.fill(Spare::None);
        self.levels[0].reset(self.graph, promoted, cands);
    }

    /// Pushes the child of the top level made by promoting `n` (which must succeed in the search's state), with
    /// the candidates `child_cands` (a subset of the top level's).  See the type's docs.
    pub fn push(&mut self, n: u32, child_cands: &Bits<W>) {
        self.push_all(&Bits::EMPTY.with(n), child_cands);
    }

    /// [`push`](Self::push) for the child made by promoting every member of `promoted` (each of them a candidate
    /// of the top level, or removed from them since its last evaluation).
    pub fn push_all(&mut self, promoted: &Bits<W>, child_cands: &Bits<W>) {
        let graph = self.graph;
        let child = self.push_copy();
        for n in promoted.iter() {
            child.promote(graph, n);
        }
        child.dropped |= child.cands - *child_cands;
        child.dirty = true;
    }

    /// Pushes a copy of the top level with the candidates `excluded` removed: the same state, fewer
    /// candidates.
    pub fn push_excluding(&mut self, excluded: &Bits<W>) {
        let child = self.push_copy();
        child.dropped |= *excluded;
        child.dirty = true;
    }

    /// Pushes a copy of the top level and returns it.
    fn push_copy(&mut self) -> &mut Level<W> {
        let d = self.depth;
        if self.levels.len() == d + 1 {
            let copy = self.levels[d].clone();
            self.levels.push(copy);
        } else {
            let (lower, upper) = self.levels.split_at_mut(d + 1);
            upper[0].copy_from(&lower[d]);
        }
        self.depth = d + 1;
        self.forget_spare(d + 1);
        &mut self.levels[d + 1]
    }

    /// Returns to the parent of the top level.
    pub fn pop(&mut self) {
        assert!(self.depth > 0, "pop without push");
        self.forget_spare(self.depth);
        self.depth -= 1;
    }

    /// The matching bound of the top level with `x` no longer a candidate, on a level pushed for it: then
    /// [`pop`](Self::pop) it, or [`keep_excluded`](Self::keep_excluded) it.  If that level was solved already
    /// and kept ([`keep_solved`](Self::keep_solved)), it is pushed as it was kept, the same level.
    pub fn value_excluding(&mut self, x: u32) -> u64 {
        let d = self.depth;
        if self.spare.get(d) == Some(&Spare::Solved(x)) {
            self.spare[d] = Spare::None;
            std::mem::swap(&mut self.levels[d + 1], &mut self.spares[d]);
            self.depth = d + 1;
            self.forget_spare(d + 1);
        } else {
            self.push_excluding(&Bits::EMPTY.with(x));
        }
        self.value()
    }

    /// An upper bound on [`value_excluding(x)`](Self::value_excluding) from the top level's potentials, without
    /// repairing anything: at least the exact value, and found in time proportional to the degrees of `x` and
    /// of the factor it pays with.
    ///
    /// The top level's potentials `y` prove its value: they are a feasible dual solution whose sum is the value.
    /// Without `x` as a candidate, its edges as a mover go and nothing else changes, so they are still feasible
    /// and their sum still bounds the new value from above.  If `x` is promoted in the heaviest matching, paying
    /// with `w`, lower `y_x` and then `y_w` as far as their remaining constraints allow (never below 0, as
    /// neither is required): they share no remaining edge (the one between them was `x`'s as a mover), so each
    /// can go as low as its own edges allow.  The matched edge was tight (`y_x + y_w = x`), so the bound becomes
    /// `value − x + y'_x + y'_w`.  If `x` is not promoted, the bound is the value itself.
    pub fn bound_excluding(&mut self, x: u32) -> u64 {
        let graph = self.graph;
        let level = self.settle();
        if !level.feasible {
            return 0;
        }
        level.bound_excluding(graph, x)
    }

    /// The members of the top level's heaviest matching without which the bound, from the potentials alone
    /// ([`bound_excluding`](Self::bound_excluding)), is below `need`: every completion that adds `need` promotes
    /// them.
    pub fn forced(&mut self, need: u64) -> Bits<W> {
        let graph = self.graph;
        let level = self.settle();
        if !level.feasible {
            // No completion at all: without any member the bound is 0.
            return if need > 0 { level.chosen } else { Bits::EMPTY };
        }
        level.forced(graph, need as i64, 0, &Bits::EMPTY)
    }

    /// Pops the level pushed by [`value_excluding`](Self::value_excluding), keeping it (settled) for its
    /// parent: once the parent is the top level again, with children pushed and popped meanwhile but nothing done
    /// to the parent itself, [`take_kept`](Self::take_kept) makes the kept level the parent, which saves
    /// repairing the parent for the same removal.  A later keep for the same parent replaces it.
    pub fn keep_excluded(&mut self) {
        self.keep(Spare::Kept);
    }

    /// Pops the level pushed by [`push_excluding`](Self::push_excluding) of `{x}`, solved, and keeps it for its
    /// parent (with levels pushed and popped above it meanwhile, but nothing done to it itself): as long as
    /// nothing is done to the parent, [`value_excluding(x)`](Self::value_excluding) takes it instead of solving
    /// the same level again, and if the parent goes on without `x` ([`remove_candidate`](Self::remove_candidate))
    /// it is the parent's kept level ([`keep_excluded`](Self::keep_excluded)).  The refinement keeps the first
    /// branch of a conflict, which strong branching asks for next.  A later keep for the same parent replaces it.
    pub fn keep_solved(&mut self, x: u32) {
        self.keep(Spare::Solved(x));
    }

    /// Pops the top level into its parent's spare, which then holds `what`.  (A spare belongs to the level in
    /// its place, so the top level's own is forgotten, as is the incoming level's when a spare is taken.)
    fn keep(&mut self, what: Spare) {
        let d = self.depth;
        assert!(d > 0, "nothing pushed to keep");
        self.forget_spare(d);
        if self.spares.len() < d {
            let size = self.graph.len();
            self.spares.resize_with(d, || Level::new(size, Numbers));
            self.spare.resize(d, Spare::None);
        }
        std::mem::swap(&mut self.levels[d], &mut self.spares[d - 1]);
        self.spare[d - 1] = what;
        self.depth = d - 1;
    }

    /// If a level is kept for the top level, makes it the top level and returns true.
    pub fn take_kept(&mut self) -> bool {
        let d = self.depth;
        if self.spare.get(d) != Some(&Spare::Kept) {
            return false;
        }
        std::mem::swap(&mut self.levels[d], &mut self.spares[d]);
        self.spare[d] = Spare::None;
        true
    }

    /// Forgets the level kept for the top level, if any.
    pub fn drop_kept(&mut self) {
        self.forget_spare(self.depth);
    }

    /// Forgets what the spare of level `d` holds.
    #[inline]
    fn forget_spare(&mut self, d: usize) {
        if let Some(spare) = self.spare.get_mut(d) {
            *spare = Spare::None;
        }
    }

    /// Number of levels above level 0.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Removes `x` from the top level's candidates (no effect if it is not one).
    pub fn remove_candidate(&mut self, x: u32) {
        let d = self.depth;
        if x <= Bits::<W>::MAX_VALUE {
            self.levels[d].dropped.insert(x);
            self.levels[d].dirty = true;
        }
        if let Some(spare) = self.spare.get_mut(d) {
            // A level solved without x is the level now, and one solved without another candidate is not.
            *spare = if *spare == Spare::Solved(x) { Spare::Kept } else { Spare::None };
        }
    }

    /// The matching bound of the top level: an upper bound on the sum of the promotions its state can still make
    /// among its candidates (not counting those already made).  0 if the state's moves cannot be matched, in
    /// which case it has no completion at all (a state the search reaches always can be).
    pub fn value(&mut self) -> u64 {
        let level = self.settle();
        if level.feasible {
            level.value
        } else {
            0
        }
    }

    /// The candidates that a heaviest matching promotes in the top level (their sum is [`value`](Self::value)).
    pub fn chosen(&mut self) -> Bits<W> {
        self.settle().chosen
    }

    /// The top level's candidates.
    pub fn candidates(&self) -> Bits<W> {
        let level = &self.levels[self.depth];
        level.cands - level.dropped
    }

    pub fn stats(&self) -> MatchStats {
        self.stats
    }

    fn settle(&mut self) -> &Level<W> {
        let MatchBound { graph, levels, spare, depth, search, stats, .. } = self;
        let level = &mut levels[*depth];
        stats.evaluations += 1;
        if level.dirty {
            // A level solved without one of its candidates is not that level once it changes.
            if let Some(s @ Spare::Solved(_)) = spare.get_mut(*depth) {
                *s = Spare::None;
            }
            level.settle(graph, search, stats);
        }
        level
    }
}

/// The matching bound of one state, with [`MatchBound`] and no search around it: builds the graph with `state`
/// as the root and solves it from scratch.  Equal to [`match_bound`], and faster.
pub fn match_bound_once<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, cands: &Bits<W>) -> u64 {
    let graph = MatchGraph::new(model, state);
    let mut solver = MatchBound::new(&graph);
    solver.reset(&Bits::EMPTY, cands);
    solver.value()
}

// ---------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::test_support::*;
    use crate::numtheory::Tables;
    use std::cmp::Reverse;
    use std::collections::{BinaryHeap, HashMap, HashSet};
    use std::hint::black_box;
    use std::time::Instant;

    pub(super) fn setup<const W: usize>(g: &Golden) -> (FrameGraph, FrameSet<W>, Bits<W>) {
        let (model, root) = FrameGraph::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
        (model, root, Bits::from_slice(&g.candidates))
    }

    // ---- the definition, by exhaustion ----

    /// Whether each of `movers` can be given its own maximal factor among `factors` (Kuhn's augmenting paths).
    fn covers<const W: usize>(model: &FrameGraph, movers: &[u32], factors: &Bits<W>) -> bool {
        fn augment<const W: usize>(
            model: &FrameGraph,
            movers: &[u32],
            factors: &Bits<W>,
            k: usize,
            seen: &mut Vec<u32>,
            owner: &mut HashMap<u32, usize>,
        ) -> bool {
            for w in model.graph().factors(movers[k]).iter().map(|&w| u32::from(w)) {
                if factors.contains(w) && !seen.contains(&w) {
                    seen.push(w);
                    let free = match owner.get(&w) {
                        None => true,
                        Some(&other) => augment(model, movers, factors, other, seen, owner),
                    };
                    if free {
                        owner.insert(w, k);
                        return true;
                    }
                }
            }
            false
        }
        let mut owner = HashMap::new();
        (0..movers.len()).all(|k| augment(model, movers, factors, k, &mut Vec::new(), &mut owner))
    }

    /// The matching bound straight from its definition: the best of all subsets `Q` of the candidates that can
    /// be promoted (at most 14 of them) such that every frame's moves plus the members of `Q` promoted into it
    /// can be given distinct factors among its factors minus the members of `Q` promoted out of it.
    fn brute_bound<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, cands: &Bits<W>) -> u64 {
        let frames = state.frames();
        let movable: Vec<(u32, usize)> = (1..frames.len())
            .flat_map(|i| frames[i].factors.intersect(cands).to_vec().into_iter().map(move |x| (x, i)))
            .collect();
        assert!(movable.len() <= 14, "too many candidates to try every subset");
        let mut best = None;
        for mask in 0u32..1 << movable.len() {
            let q: Vec<(u32, usize)> =
                movable.iter().enumerate().filter(|&(k, _)| mask >> k & 1 == 1).map(|(_, &m)| m).collect();
            let fits = frames.iter().enumerate().all(|(i, f)| {
                let movers: Vec<u32> =
                    f.moves.iter().chain(q.iter().filter(|&&(_, h)| h == i + 1).map(|&(x, _)| x)).collect();
                let out: Bits<W> = q.iter().filter(|&&(_, h)| h == i).map(|&(x, _)| x).collect();
                covers(model, &movers, &(f.factors - out))
            });
            if fits {
                best = best.max(Some(q.iter().map(|&(x, _)| u64::from(x)).sum::<u64>()));
            }
        }
        best.unwrap_or(0)
    }

    #[test]
    fn radix_heap_pops_like_a_binary_heap() {
        let mut rng = Rng(0x4EA9_0000_1234_5678);
        let mut radix = RadixHeap::default();
        for round in 0..200 {
            radix.clear();
            let mut reference = BinaryHeap::new();
            let mut now = 0u64;
            // The pairs waiting: as in the search, none is pushed again while it waits (a code can wait at several
            // times, as a vertex reached again sooner does).
            let mut waiting = HashSet::new();
            for _ in 0..rng.below(300) {
                // Pushes at or after the last time popped; sometimes pops.
                for _ in 0..rng.below(4) {
                    let time = now + [0, 0, 1, 2, 7, 100, 1 << 20][rng.below(7) as usize] + rng.below(3);
                    let code = rng.below(1000) as u32;
                    if waiting.insert((time, code)) {
                        radix.push(time, code);
                        reference.push(Reverse((time, code)));
                    }
                }
                if rng.chance(60) {
                    let expected = reference.pop().map(|Reverse(e)| e);
                    assert_eq!(radix.pop(), expected, "round {round}");
                    if let Some((time, code)) = expected {
                        now = time;
                        waiting.remove(&(time, code));
                    }
                }
            }
            // A search stops part way and clears the rest, or drains it.
            if rng.chance(50) {
                while let Some(Reverse(e)) = reference.pop() {
                    assert_eq!(radix.pop(), Some(e), "round {round}, draining");
                }
                assert_eq!(radix.pop(), None);
            }
        }
    }

    #[test]
    fn matchable_but_not_winnable() {
        // Frame 0: factors 4, 6, 10 and moves 12 (pays with 4 or 6) and 20 (4 or 10); frame 1: the factor 30.
        // Promoting 30 adds the move 30 (6 or 10) to frame 0: matchable, 12→4, 20→10, 30→6, but not winnable.
        let columns = vec![vec![4, 6, 10], vec![12, 20], vec![30], vec![]];
        let (model, root) = FrameGraph::build::<1>(&Tables::new(30), &columns).unwrap();
        let cands = Bits::from_slice(&[30]);
        assert!(!model.can_promote(&root, 30));
        assert_eq!(best_completion(&model, &root, &cands, u32::MAX), 0);
        assert_eq!(brute_bound(&model, &root, &cands), 30);
        assert_eq!(match_bound(&model, &root, &cands), 30);
        assert_eq!(match_bound_once(&model, &root, &cands), 30);
        assert_eq!(match_bound_once(&model, &root, &Bits::EMPTY), 0);
    }

    // ---- driving the solver as a search does ----

    /// What a walk shows its visitor at each evaluation: the solver (with the node on top), the node's state,
    /// its promotions, and its candidates.
    pub(super) type Visit<'v, const W: usize> =
        dyn FnMut(&mut MatchBound<'_, W>, &FrameSet<W>, &Bits<W>, &Bits<W>) + 'v;

    /// Drives a [`MatchBound`] the way a depth-first search does, along random paths: at each node the expansion
    /// loop takes the largest candidate, sometimes descends into the child it makes, then goes on with the
    /// candidates shrunk.  Evaluations are random too, so changes pile up between them.
    struct Walk<'a, const W: usize> {
        model: &'a FrameGraph,
        rng: Rng,
        /// Loop iterations left.
        budget: usize,
    }

    impl<const W: usize> Walk<'_, W> {
        fn explore(
            &mut self,
            mb: &mut MatchBound<'_, W>,
            state: &FrameSet<W>,
            promoted: Bits<W>,
            mut cands: Bits<W>,
            visit: &mut Visit<'_, W>,
        ) {
            while let Some(n) = cands.max() {
                if self.budget == 0 {
                    return;
                }
                self.budget -= 1;
                // A level kept for the node becomes its level, as at the searcher's steps: the same level as the
                // node's own, repaired.
                if self.rng.chance(20) {
                    let mut twin = mb.clone();
                    twin.drop_kept();
                    twin.value();
                    if mb.take_kept() {
                        mb.value();
                        assert_same_level(&mb.levels[mb.depth], &twin.levels[twin.depth], "a kept level");
                    }
                } else {
                    mb.take_kept();
                }
                assert!(mb.candidates().is_subset(&cands), "the solver's candidates are out of step");
                if self.rng.chance(80) {
                    visit(mb, state, &promoted, &cands);
                }
                cands.remove(n);
                // The level without n solved ahead and kept, as the refinement keeps its first branch: taken by
                // value_excluding(n) below, or the node's kept level once the node goes on without n.
                if self.rng.chance(30) {
                    mb.push_excluding(&Bits::EMPTY.with(n));
                    mb.value();
                    mb.keep_solved(n);
                    if self.rng.chance(30) {
                        // Evaluating the node meanwhile drops it if the node had changes to repair.
                        mb.value();
                    }
                }
                let remove_first = self.rng.chance(30);
                if remove_first {
                    mb.remove_candidate(n);
                    if self.rng.chance(30) {
                        // Applies the removal, so that a push has to bring n back as a required vertex.
                        mb.value();
                    }
                }
                // Or the level without n is solved beforehand and kept for after the child.
                let keep = !remove_first && self.rng.chance(40);
                if keep {
                    let mut twin = mb.clone();
                    twin.push_excluding(&Bits::EMPTY.with(n));
                    twin.value();
                    mb.value_excluding(n);
                    assert_same_level(&mb.levels[mb.depth], &twin.levels[twin.depth], "a level solved ahead");
                    mb.keep_excluded();
                }
                if self.rng.chance(40) {
                    if let Some(mut next) = self.model.promote(state, n) {
                        let mut next_promoted = promoted.with(n);
                        let mut child = next.narrow_candidates(&cands);
                        // Sometimes the child promotes another candidate too, as the search's forced ones.
                        let mut added = Bits::EMPTY.with(n);
                        if let Some(m) = child.max().filter(|_| self.rng.chance(30)) {
                            if self.model.promote_in_place(&mut next, m) {
                                child.remove(m);
                                next_promoted.insert(m);
                                added.insert(m);
                                child = next.narrow_candidates(&child);
                            }
                        }
                        let depth = mb.depth();
                        mb.push_all(&added, &child);
                        if self.rng.chance(60) {
                            self.explore(mb, &next, next_promoted, child, visit);
                        } else {
                            visit(mb, &next, &next_promoted, &child);
                        }
                        mb.pop();
                        assert_eq!(mb.depth(), depth);
                    }
                }
                if keep {
                    assert!(mb.take_kept(), "the kept level is gone");
                } else if !remove_first {
                    mb.remove_candidate(n);
                }
                if self.rng.chance(5) {
                    break;
                }
            }
        }
    }

    /// Runs `paths` walks of `budget` loop iterations each from the root of `g`.
    pub(super) fn walks<const W: usize>(g: &Golden, seed: u64, paths: usize, budget: usize, visit: &mut Visit<'_, W>) {
        let (model, root, all) = setup::<W>(g);
        let graph = MatchGraph::new(&model, &root);
        let mut mb = MatchBound::new(&graph);
        let mut walk = Walk { model: &model, rng: Rng(seed), budget: 0 };
        for _ in 0..paths {
            mb.reset(&Bits::EMPTY, &all);
            walk.budget = budget;
            walk.explore(&mut mb, &root, Bits::EMPTY, all, visit);
        }
    }

    /// Checks [`Level::bound_excluding`] for every member of a settled level's heaviest matching against the
    /// formula it computes, every edge scanned (it stops at a tight one): the same value, or the forced members,
    /// and so the trees, would change.
    pub(super) fn check_bounds_excluding<const W: usize, K: Weighing>(g: &MatchGraph, lv: &Level<W, K>) {
        for x in lv.chosen.iter() {
            let v = g.vertex_of(x).expect("a vertex");
            let w = usize::from(lv.mate[v]);
            let as_factor = |f: usize| {
                let counted = g.above(f).iter().map(|&u| usize::from(u)).filter(|&u| u != v);
                let counted = counted.filter(|&u| lv.role[u] != Role::Factor);
                counted.map(|u| lv.weight(g, u) - lv.y[u]).max().unwrap_or(0).max(0)
            };
            let mut y_w = as_factor(w);
            if lv.role[w] == Role::Candidate {
                let factors = g.below(w).iter().map(|&f| usize::from(f)).filter(|&f| lv.role[f] != Role::Required);
                y_w = factors.map(|f| lv.weight(g, w) - lv.y[f]).fold(y_w, i64::max);
            }
            let bound = (lv.value as i64 - lv.weighing.of(g, v) + as_factor(v) + y_w).clamp(0, lv.value as i64);
            assert_eq!(lv.bound_excluding(g, x), bound as u64, "the bound without {x} from the potentials");
        }
    }

    /// Asserts that two levels hold the same solution (roles, matching, potentials, and what follows from them):
    /// work kept for later must be the work it saves, or the search's trees would change.
    fn assert_same_level<const W: usize>(a: &Level<W>, b: &Level<W>, what: &str) {
        assert_eq!(a.role, b.role, "{what}: roles");
        assert_eq!(a.mate, b.mate, "{what}: matching");
        assert_eq!(a.y, b.y, "{what}: potentials");
        assert_eq!((a.cands, a.chosen, a.value, a.feasible), (b.cands, b.chosen, b.value, b.feasible), "{what}");
    }

    /// Checks that the top level of `mb` is settled and optimal: its matching and potentials meet the conditions
    /// in the module docs, which prove the matching heaviest, and the bookkeeping agrees with them.
    fn check_certificate<const W: usize>(mb: &MatchBound<'_, W>) {
        check_level(mb.graph, &mb.levels[mb.depth]);
    }

    /// [`check_certificate`] for a level weighed in any way.
    pub(super) fn check_level<const W: usize, K: Weighing>(g: &MatchGraph, lv: &Level<W, K>) {
        assert!(lv.pending.is_empty() && lv.dropped.is_empty(), "not settled");
        assert!(lv.feasible, "the moves of a search state can always be matched");
        let (mut cands, mut chosen, mut matched_weight) = (Bits::<W>::EMPTY, Bits::<W>::EMPTY, 0);
        for v in 0..g.len() {
            let (role, y, mate) = (lv.role[v], lv.y[v], lv.mate[v]);
            if role == Role::Candidate {
                cands.insert(g.num[v]);
            }
            if role == Role::Required {
                assert!(mate != NONE && usize::from(mate) < v, "required {} is not paid for", g.num[v]);
            } else {
                assert!(y >= 0, "negative potential at {}", g.num[v]);
                assert!(mate != NONE || y == 0, "free {} has a potential", g.num[v]);
            }
            if mate != NONE {
                assert_eq!(usize::from(lv.mate[usize::from(mate)]), v, "matching not symmetric");
            }
            let mut paid = false;
            for w in g.below(v).iter().map(|&w| usize::from(w)) {
                let matched = usize::from(mate) == w;
                if role == Role::Factor || lv.role[w] == Role::Required {
                    assert!(!matched, "matched over an edge that does not count");
                    continue;
                }
                let slack = y + lv.y[w] - lv.weight(g, v);
                assert!(slack >= 0, "edge {}-{} has negative slack", g.num[v], g.num[w]);
                if matched {
                    assert_eq!(slack, 0, "matched edge {}-{} is not tight", g.num[v], g.num[w]);
                    paid = true;
                    matched_weight += lv.weight(g, v);
                    if role == Role::Candidate {
                        chosen.insert(g.num[v]);
                    }
                }
            }
            assert!(mate == NONE || usize::from(mate) > v || paid, "{} matched to a non-neighbour", g.num[v]);
        }
        assert_eq!(cands, lv.cands);
        assert_eq!(chosen, lv.chosen);
        let weights = chosen.iter().map(|x| lv.weighing.of(g, g.vertex_of(x).expect("a vertex"))).sum::<i64>();
        assert_eq!(weights, lv.value as i64);
        assert_eq!(lv.y.iter().sum::<i64>(), matched_weight, "duality gap");
    }

    // ---- correctness ----

    #[test]
    #[cfg_attr(debug_assertions, ignore = "long: the quick check is reference_meets_the_definition_on_a_sample")]
    fn reference_meets_the_definition() {
        reference_meets_the_definition_on_every(1);
    }

    /// [`reference_meets_the_definition`] on every sixth of its golden games.
    #[test]
    fn reference_meets_the_definition_on_a_sample() {
        reference_meets_the_definition_on_every(6);
    }

    /// The reference bound meets the definition on every `every`th of the golden games.
    fn reference_meets_the_definition_on_every(every: usize) {
        let mut rng = Rng(0x0BAD_F00D_1234_5678);
        let mut checks = 0;
        let ns = golden_ns().into_iter().filter(|&n| (30..=200).contains(&n) && n % 6 == 0);
        for n in ns.step_by(every) {
            let g = load(n);
            let (model, ..) = setup::<4>(&g);
            walks::<4>(&g, n.into(), 3, 60, &mut |_, state, _, cands| {
                // The whole candidate set when it is small, and random small subsets of it.
                let all = (1..state.len()).map(|i| state.frames()[i].factors).fold(Bits::EMPTY, |a, f| a | f);
                let mut sets = vec![rng.subset(cands, 30)];
                if cands.intersect(&all).len() <= 14 {
                    sets.push(*cands);
                }
                for set in sets {
                    let set = set.intersect(&all).largest(1 + rng.below(12) as u32);
                    let truth = brute_bound(&model, state, &set);
                    assert_eq!(match_bound(&model, state, &set), truth, "n={n}, {set:?}");
                    assert_eq!(match_bound_once(&model, state, &set), truth, "n={n}, {set:?}");
                    checks += 1;
                }
            });
        }
        eprintln!("reference against the definition: {checks} candidate sets");
        assert!(checks > 1000 / every);
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is incremental_solver_matches_the_reference_on_a_sample"
    )]
    fn incremental_solver_matches_the_reference() {
        incremental_solver_matches_the_reference_on_every(1);
    }

    /// [`incremental_solver_matches_the_reference`] on every sixth of its golden games.
    #[test]
    fn incremental_solver_matches_the_reference_on_a_sample() {
        incremental_solver_matches_the_reference_on_every(6);
    }

    /// The incremental solver matches the reference on every `every`th of the golden games.
    fn incremental_solver_matches_the_reference_on_every(every: usize) {
        let mut checks = 0;
        let mut stats = MatchStats::default();
        let ns = golden_ns().into_iter().filter(|&n| n >= 30 && (n % 26 == 0 || [480, 760].contains(&n)));
        for n in ns.step_by(every) {
            let g = load(n);
            let (paths, budget) = if n < 300 { (6, 150) } else { (2, 60) };
            if n < 512 {
                checks += agreement::<8>(&g, paths, budget, &mut stats);
            } else {
                checks += agreement::<12>(&g, paths, budget, &mut stats);
            }
        }
        eprintln!("incremental against the reference: {checks} evaluations, {stats:?}");
        assert!(checks > 2000 / every as u64);
    }

    /// Walks the search paths of `g`, checking every evaluation against the reference, the optimality
    /// certificate, and a solver started afresh at the same node; returns the number of evaluations.
    fn agreement<const W: usize>(g: &Golden, paths: usize, budget: usize, stats: &mut MatchStats) -> u64 {
        let (model, root, _) = setup::<W>(g);
        let graph = MatchGraph::new(&model, &root);
        let mut fresh = MatchBound::new(&graph);
        let mut rng = Rng(u64::from(g.n) * 7919);
        let mut checks = 0;
        let mut evaluations = MatchStats::default();
        let (mut quick_checks, mut quick_tight) = (0u64, 0u64);
        walks::<W>(g, g.n.into(), paths, budget, &mut |mb, state, promoted, cands| {
            let fast = mb.value();
            check_certificate(mb);
            assert_eq!(fast, match_bound(&model, state, cands), "n={}, promoted {promoted:?}", g.n);
            assert_eq!(mb.chosen().sum(), fast);
            assert!(mb.chosen().is_subset(cands));
            if rng.chance(10) {
                fresh.reset(promoted, cands);
                assert_eq!(fresh.value(), fast, "n={}: started afresh at {promoted:?}", g.n);
                check_certificate(&fresh);
            }
            // The quick bound without a candidate (often a promoted one) is at least the exact value.
            let pool = if rng.chance(70) { mb.chosen() } else { *cands };
            check_bounds_excluding(&graph, &mb.levels[mb.depth]);
            if let Some(x) = pool.nth_largest(rng.below(u64::from(pool.len().max(1))) as u32) {
                let quick = mb.bound_excluding(x);
                let exact = mb.value_excluding(x);
                mb.pop();
                assert!(exact <= quick && quick <= fast, "n={}: without {x}: {exact} > {quick}", g.n);
                assert_eq!(exact, match_bound(&model, state, &cands.without(x)), "n={}: without {x}", g.n);
                quick_tight += u64::from(quick == exact);
                quick_checks += 1;
            }
            evaluations = mb.stats();
            checks += 1;
        });
        stats.evaluations += evaluations.evaluations;
        stats.searches += evaluations.searches;
        stats.scanned += evaluations.scanned;
        eprintln!("  n={}: quick bounds without a candidate exact at {quick_tight} of {quick_checks}", g.n);
        assert!(quick_checks > checks / 2);
        checks
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = "long: the quick check is bound_is_sound_on_a_sample")]
    fn bound_is_sound() {
        bound_is_sound_on_every(1);
    }

    /// [`bound_is_sound`] on every sixth of its golden games.
    #[test]
    fn bound_is_sound_on_a_sample() {
        bound_is_sound_on_every(6);
    }

    /// The matching bound is sound on every `every`th of the golden games.
    fn bound_is_sound_on_every(every: usize) {
        let (mut checks, mut tight, mut everything) = (0, 0, 0);
        for n in golden_ns().into_iter().filter(|&n| (40..=220).contains(&n)).step_by(every) {
            let g = load(n);
            let (model, ..) = setup::<4>(&g);
            walks::<4>(&g, u64::from(n) << 8, 6, 80, &mut |mb, state, promoted, cands| {
                let bound = mb.value();
                // No promotable completion exceeds the count the freedoms allow, and
                // the limit makes the exhaustive search much faster.
                let most = state.max_promotions_among(cands).max(0) as u32;
                let best = best_completion(&model, state, cands, most);
                assert!(bound >= best, "n={n}: bound {bound} < best {best} after {promoted:?} with {cands:?}");
                assert!(bound <= cands.sum());
                if best == cands.sum() {
                    // Every candidate can be promoted together: the bound is exact.
                    assert_eq!(bound, best);
                    everything += 1;
                }
                tight += u64::from(bound == best);
                checks += 1;
            });
        }
        eprintln!(
            "matching bound soundness: {checks} states, exact at {tight}, all candidates promotable at {everything}"
        );
        assert!(checks > 3000 / every);
    }

    // ---- micro-benchmarks: cargo test --release bench_matchbound -- --ignored --nocapture --test-threads=1 ----

    fn millis(since: Instant) -> f64 {
        since.elapsed().as_secs_f64() * 1e3
    }

    /// Times the solver at the root of `g`: from scratch, and on the root's expansion loop, where each step
    /// removes the largest candidate and evaluates the root, and also pushes, evaluates and pops the child that
    /// promoting it makes.
    fn bench<const W: usize>(g: &Golden) {
        let (model, root, cands) = setup::<W>(g);
        let graph = MatchGraph::new(&model, &root);
        let mut mb = MatchBound::new(&graph);
        let edges = (0..graph.len()).map(|v| graph.below(v).len()).sum::<usize>();
        eprintln!("n={} W={W}: {} vertices, {edges} edges, {} candidates", g.n, graph.len(), cands.len());

        let start = Instant::now();
        let reference = match_bound(&model, &root, &cands);
        eprintln!("  reference (min-cost flow):            {:9.3} ms", millis(start));
        let ns = time_per_call(1, || {
            mb.reset(&Bits::EMPTY, &cands);
            black_box(mb.value());
        });
        assert_eq!(mb.value(), reference);
        eprintln!("  reset + value (from scratch):         {:9.1} µs", ns / 1e3);
        let ns = time_per_call(1, || {
            black_box(match_bound_once(&model, &root, &cands));
        });
        eprintln!("  match_bound_once (graph + solve):     {:9.1} µs", ns / 1e3);

        // The root's expansion loop: the children's candidate lists, worked out beforehand.
        let mut left = cands;
        let mut steps: Vec<(u32, Option<Bits<W>>)> = Vec::new();
        for n in cands.iter_desc() {
            left.remove(n);
            let child = model.promote(&root, n).map(|next| next.narrow_candidates(&left));
            steps.push((n, child));
        }
        let children = steps.iter().filter(|s| s.1.is_some()).count();
        mb.reset(&Bits::EMPTY, &cands);
        mb.value();
        let solved = mb.levels[0].clone();
        let copy_ns = time_per_call(1, || mb.levels[0].copy_from(black_box(&solved)));
        let run = |with_children: bool, mb: &mut MatchBound<'_, W>| {
            mb.levels[0].copy_from(&solved);
            for &(n, child) in &steps {
                black_box(mb.value());
                if let (true, Some(child)) = (with_children, child) {
                    mb.push(n, &child);
                    black_box(mb.value());
                    mb.pop();
                }
                mb.remove_candidate(n);
            }
        };
        let before = mb.stats();
        run(false, &mut mb);
        let shrink = mb.stats();
        run(true, &mut mb);
        let both = mb.stats();
        let shrink_ns = time_per_call(1, || run(false, &mut mb)) - copy_ns;
        let both_ns = time_per_call(1, || run(true, &mut mb)) - copy_ns;
        let per = |searches: u64, scanned: u64, count: usize| {
            format!(
                "{:.2} searches, {:.1} vertices scanned",
                searches as f64 / count as f64,
                scanned as f64 / count as f64
            )
        };
        eprintln!("  level copy:                           {:9.1} µs", copy_ns / 1e3);
        eprintln!(
            "  root, largest candidate removed:      {:9.2} µs per step ({} steps; {})",
            shrink_ns / 1e3 / steps.len() as f64,
            steps.len(),
            per(shrink.searches - before.searches, shrink.scanned - before.scanned, steps.len())
        );
        eprintln!(
            "  child (push + value + pop):           {:9.2} µs per child ({children} children; {})",
            (both_ns - shrink_ns) / 1e3 / children as f64,
            // Added before subtracting: the difference of the first two alone can be negative.
            per(
                both.searches + before.searches - 2 * shrink.searches,
                both.scanned + before.scanned - 2 * shrink.scanned,
                children
            )
        );
    }

    #[test]
    #[ignore]
    fn bench_matchbound_480() {
        bench::<8>(&load(480));
    }

    #[test]
    #[ignore]
    fn bench_matchbound_760() {
        bench::<12>(&load(760));
    }
}
