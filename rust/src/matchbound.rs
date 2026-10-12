//! The matching bound: an upper bound on the sum of the promotions a search state can still make, from relaxing
//! winnability to matchability.
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
//! promoted at once (see [`Model::winnable`]).  So for a feasible `Q` there is an assignment that gives every
//! move of every frame `i` a distinct maximal factor among frame `i`'s factors and every `x ∈ Q` a distinct
//! maximal factor among the factors of frame `home(x) − 1`, using no member of `Q` as a factor.  The *matching
//! bound* is the largest `sum(Q)` over the `Q ⊆ cands` that have such an assignment: at least the sum of the best
//! feasible completion.
//!
//! The frames hold distinct numbers, so the assignment is a matching in the graph whose vertices are the numbers
//! and whose edges join each possible mover `u` (a move, or a candidate) to its maximal factors `u/p` in the
//! frame it pays in.  A candidate is a single vertex whether it pays (it is promoted) or is paid with (it is
//! not), so "no member of `Q` is a factor" holds by itself.  Every edge changes the number of prime factors by
//! one, so the graph is bipartite, and the bound is a maximum-weight bipartite matching in which every move must
//! be matched and an edge whose mover is a candidate weighs that candidate.
//!
//! The bound ignores frame freedoms (frame 0's can be revised to fewer than matching allows) and any cap on the
//! number of promotions.  Both only remove completions, so it remains an upper bound.  The `blameGame` bound
//! ([`crate::bounds::upper_bound`]) does respect them, and the smaller of the two is a bound too.
//!
//! # Two solvers
//!
//! [`match_bound`] is the reference: it builds the graph of a state and solves a min-cost flow from scratch,
//! simply and slowly.  [`MatchBound`] is the solver for the search: it keeps an optimal matching and an optimal
//! dual solution for every node on the search's path and repairs them after each change, which is cheap
//! because the changes along a depth-first path are small.
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
//! of a state reached by promotions are the root's minus the forced pairs (a move left with a single factor
//! takes it, see [`Model`]); the graph keeps those pairs, but every matching that matches the required vertices
//! pairs them up just as the reduction did, so the optimum is the same.
//!
//! # The incremental solver
//!
//! Besides a maximum-weight matching, [`MatchBound`] keeps a potential `y` per vertex that proves it optimal
//! (LP duality for bipartite matching): `y ≥ 0` except on required vertices (whose constraint is an equality),
//! `y_u + y_w ≥ weight` on every edge, with equality on matched edges, and `y = 0` on every free vertex that is
//! not required.  A change of roles breaks these conditions at a few vertices at most:
//!
//! * a candidate that is not promoted in the optimum stops being a candidate: nothing breaks;
//! * a candidate that is promoted stops being one: its matched edge goes, and both ends may now be free with a
//!   positive potential;
//! * a candidate that is promoted in the optimum becomes required (the search promotes it): its potential drops
//!   by its number and nothing breaks;
//! * any other number becomes required: it is free and must be matched, and if it was paying for a mover, that
//!   mover is free now.
//!
//! Each vertex where the conditions break is repaired by one Hungarian search: a Dijkstra over reduced costs
//! that grows alternating paths from it until either a free vertex is reached or the potential of a vertex in
//! the tree that is not required reaches zero (that vertex can then give up its partner, or stay free if it is
//! the start); then it shifts the potentials of the tree and flips the alternating path.  It stops early when
//! the change is local, which is the common case.

use crate::bits::Bits;
use crate::model::{FrameSet, Model};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};

// ---------------------------------------------------------------------------------------------------------
// Reference solver

/// The matching bound of `state` with the candidates `cands`, computed from scratch (see the module docs).
/// Candidates that are not factors of a frame above frame 0 cannot be promoted and are ignored.  Returns 0 when
/// the moves of `state` cannot all be matched, since then nothing can be promoted from it.
///
/// This is the reference for [`MatchBound`]: a min-cost flow, solved by successive shortest paths with
/// Bellman–Ford, clear rather than fast (milliseconds on large games).
pub fn match_bound<const W: usize>(model: &Model, state: &FrameSet<W>, cands: &Bits<W>) -> u64 {
    let frames = state.frames();
    // The movers: (number, frame it pays in, weight), with no weight for the moves, which must be matched.
    let mut movers: Vec<(u32, usize, Option<u64>)> = Vec::new();
    for (i, frame) in frames.iter().enumerate() {
        movers.extend(frame.moves.iter().map(|m| (m, i, None)));
        if i > 0 {
            movers.extend(frame.factors.intersect(cands).iter().map(|x| (x, i - 1, Some(u64::from(x)))));
        }
    }
    // Weighing each move more than all the candidates together makes a heaviest matching match all the moves
    // whenever some matching does.
    let big = 1 + movers.iter().filter_map(|&(_, _, weight)| weight).sum::<u64>();

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
    let mut edges = Vec::new(); // (arc, weight, whether the mover is a move)
    for &(u, i, weight) in &movers {
        for w in model.graph().factors(u).iter().map(|&w| u32::from(w)) {
            if frames[i].factors.contains(w) {
                let (a, b) = (node[u as usize], node[w as usize]);
                let (from, to) = if even_omega(u) { (a, b) } else { (b, a) };
                let weight_or_big = weight.unwrap_or(big);
                edges.push((net.add(from, to, -(weight_or_big as i64)), weight_or_big, weight.is_none()));
            }
        }
    }
    net.min_cost_flow();
    let matched: Vec<_> = edges.iter().filter(|&&(arc, _, _)| net.carries(arc)).collect();
    let moves = movers.iter().filter(|m| m.2.is_none()).count();
    if matched.iter().filter(|e| e.2).count() < moves {
        return 0;
    }
    matched.iter().map(|e| e.1).sum::<u64>() - big * moves as u64
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
    cap: Vec<i32>,
    cost: Vec<i64>,
}

impl Network {
    fn new(nodes: usize) -> Self {
        Network { out: vec![Vec::new(); nodes], head: Vec::new(), cap: Vec::new(), cost: Vec::new() }
    }

    /// Adds an arc of capacity 1 and its reverse; returns the arc.
    fn add(&mut self, from: usize, to: usize, cost: i64) -> usize {
        let e = self.head.len();
        self.head.extend([to, from]);
        self.cap.extend([1, 0]);
        self.cost.extend([cost, -cost]);
        self.out[from].push(e);
        self.out[to].push(e + 1);
        e
    }

    /// Whether the arc `e` (not a reverse arc) carries flow.
    fn carries(&self, e: usize) -> bool {
        self.cap[e] == 0
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
                    if self.cap[e] > 0 && dist[u] + self.cost[e] < dist[v] {
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
                self.cap[e] -= 1;
                self.cap[e ^ 1] += 1;
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
    /// factor of a frame above frame 0 to its maximal factors in the frame below.
    pub fn new<const W: usize>(model: &Model, root: &FrameSet<W>) -> Self {
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

/// The solver's state at one node of the search path.
#[derive(Clone, Debug)]
struct Level<const W: usize> {
    role: Vec<Role>,
    /// The vertex each vertex is matched to, `NONE` if free.
    mate: Vec<u16>,
    /// Potentials (the dual solution).
    y: Vec<i64>,
    /// The numbers whose role is candidate.
    cands: Bits<W>,
    /// The candidates matched to a factor (promoted in the optimum), and their sum.
    chosen: Bits<W>,
    value: u64,
    /// Candidates to turn into factors before the next evaluation.
    dropped: Bits<W>,
    /// Vertices where the optimality conditions may be broken: free, and required or with a positive potential.
    pending: Vec<u16>,
    /// False once a required vertex was found that cannot be matched.
    feasible: bool,
}

impl<const W: usize> Level<W> {
    fn new(size: usize) -> Self {
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
    }

    /// The weight of the edges from the mover `u` to its factors.
    #[inline]
    fn weight(&self, g: &MatchGraph, u: usize) -> i64 {
        if self.role[u] == Role::Candidate {
            i64::from(g.num[u])
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
            let x = g.num[mover];
            if add {
                self.value += u64::from(x);
                self.chosen.insert(x);
            } else {
                self.value -= u64::from(x);
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
        for v in 0..g.len() {
            let x = g.num[v];
            self.role[v] = if g.root_move[v] || promoted.contains(x) {
                Role::Required
            } else if cands.contains(x) && !g.below(v).is_empty() {
                self.cands.insert(x);
                Role::Candidate
            } else {
                Role::Factor
            };
            self.mate[v] = NONE;
            // Then the slack of every edge is its factor's potential: 0 for a factor, its number for a candidate.
            self.y[v] = self.weight(g, v);
        }
        // Largest movers first, take a free factor over a tight edge.
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
        self.dropped.remove(n);
        match self.role[v] {
            Role::Required => return,
            Role::Candidate if self.mate[v] != NONE && usize::from(self.mate[v]) < v => {
                // Promoted in the optimum: it stays matched, and its edges now weigh 0 instead of n.
                self.tally(g, v, usize::from(self.mate[v]), false);
                self.y[v] -= i64::from(n);
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

    /// Restores the optimality conditions.
    fn settle(&mut self, g: &MatchGraph, search: &mut Search, stats: &mut MatchStats) {
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

/// [`Search::tag`] values, valid for vertices stamped with the current epoch.
const REACHED: u8 = 1;
const DONE: u8 = 2;
const INNER: u8 = 3;

/// Scratch space for the Hungarian search, reused from one search to the next.
#[derive(Clone, Debug, Default)]
struct Search {
    epoch: u32,
    /// The epoch in which each vertex was last touched; the other arrays are valid only for the current one.
    stamp: Vec<u32>,
    /// Reached (an outer vertex with a tentative distance), done (an outer vertex in the tree), or inner.
    tag: Vec<u8>,
    /// Distance from the start: when an inner vertex joined the tree; when an outer one was or can be reached.
    dist: Vec<i64>,
    /// The inner vertex an outer one is reached from.
    pred: Vec<u16>,
    /// Events by time: `2v` an outer vertex `v` reached, `2v + 1` the potential of the inner vertex `v` at zero.
    heap: BinaryHeap<Reverse<(i64, u32)>>,
    /// The vertices of the tree (inner, and done outer), whose potentials change at the end.
    tree: Vec<u16>,
}

impl Search {
    fn begin(&mut self, size: usize) {
        if self.stamp.len() < size {
            self.stamp.resize(size, 0);
            self.tag.resize(size, 0);
            self.dist.resize(size, 0);
            self.pred.resize(size, NONE);
        }
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.stamp.fill(0);
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
    fn run<const W: usize>(&mut self, g: &MatchGraph, lv: &mut Level<W>, root: usize) -> bool {
        self.begin(g.len());
        self.add_inner(g, lv, root, 0);
        while let Some(Reverse((t, code))) = self.heap.pop() {
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
            if self.tag[v] != REACHED || self.dist[v] != t {
                continue; // reached again sooner, or already in the tree
            }
            self.tag[v] = DONE;
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
    fn add_inner<const W: usize>(&mut self, g: &MatchGraph, lv: &Level<W>, a: usize, d: i64) {
        self.stamp[a] = self.epoch;
        self.tag[a] = INNER;
        self.dist[a] = d;
        self.tree.push(a as u16);
        let (role, ya, mate) = (lv.role[a], lv.y[a], lv.mate[a]);
        if role != Role::Required {
            self.heap.push(Reverse((d + ya, (a as u32) << 1 | 1)));
        }
        if role != Role::Factor {
            let base = d + ya - lv.weight(g, a);
            for &b in g.below(a) {
                if b != mate && lv.role[usize::from(b)] != Role::Required {
                    self.reach(usize::from(b), base + lv.y[usize::from(b)], a, d);
                }
            }
        }
        if role != Role::Required {
            let base = d + ya;
            for &b in g.above(a) {
                let b = usize::from(b);
                if b as u16 != mate && lv.role[b] != Role::Factor {
                    self.reach(b, base + lv.y[b] - lv.weight(g, b), a, d);
                }
            }
        }
    }

    /// Reaches the outer vertex `b` from the inner vertex `a` (at distance `d`) at distance `nd`.
    #[inline]
    fn reach(&mut self, b: usize, nd: i64, a: usize, d: i64) {
        debug_assert!(nd >= d, "negative reduced cost");
        if self.stamp[b] != self.epoch {
            self.stamp[b] = self.epoch;
            self.tag[b] = REACHED;
        } else if self.tag[b] != REACHED || nd >= self.dist[b] {
            debug_assert_ne!(self.tag[b], INNER, "the graph is not bipartite");
            return;
        }
        self.dist[b] = nd;
        self.pred[b] = a as u16;
        self.heap.push(Reverse((nd, (b as u32) << 1)));
    }

    /// Shifts the potentials of the tree for a search that ends at time `end`.
    fn shift<const W: usize>(&self, lv: &mut Level<W>, end: i64) {
        for &v in &self.tree {
            let v = usize::from(v);
            if self.tag[v] == INNER {
                lv.y[v] -= end - self.dist[v];
            } else {
                lv.y[v] += end - self.dist[v];
            }
        }
    }

    /// Flips the alternating path from `root` to the outer vertex `b`, which is free.
    fn flip<const W: usize>(&self, g: &MatchGraph, lv: &mut Level<W>, root: usize, mut b: usize) {
        loop {
            let a = usize::from(self.pred[b]);
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
/// ```text
/// let graph = MatchGraph::new(&model, &root);       // once per game, shared by the threads
/// let mut mb = MatchBound::new(&graph);              // once per thread
/// mb.reset(&promoted, &cands);                       // the node a work unit starts from
///
/// // The expansion loop of a node, with that node on top of mb:
/// while let Some(n) = cands.max() {
///     if sum(promoted) + blame_bound(state, cands) <= best { break }
///     if sum(promoted) + mb.value() <= best { break }                 // only when blame did not prune
///     cands.remove(n);
///     mb.remove_candidate(n);                        // recorded, not applied: free before push(n, ...)
///     let Some(new_state) = model.promote(&state, n) else { continue };
///     let child_cands = new_candidate_list(n, ..., &new_state, &cands, &promoted);
///     if sum(promoted) + n + blame_bound(new_state, child_cands) > best {
///         mb.push(n, &child_cands);
///         if sum(promoted) + n + mb.value() > best {
///             // explore the child: its expansion loop runs with the child on top of mb
///         }
///         mb.pop();
///     }
/// }
/// ```
///
/// The rules: `push(n, child_cands)` makes the child obtained by promoting `n` from the top level, where `n` is
/// one of the top level's candidates (or was removed from them since its last evaluation) and `child_cands` is
/// a subset of the top level's candidates; `pop` goes back once the child is done.  Neither touches the parent,
/// so the parent's loop continues where it was.  `remove_candidate(n)` before `push(n, ...)` is free as long as
/// no evaluation of the parent comes between them; after one, the child has to repair what the removal did.
/// The bound covers the state's own moves and promotions only: the search adds `sum(promoted)` itself.
///
/// For a single state with no search, [`match_bound_once`] does all of this in one call.
#[derive(Clone, Debug)]
pub struct MatchBound<'g, const W: usize> {
    graph: &'g MatchGraph,
    levels: Vec<Level<W>>,
    /// `spares[d]` is a level kept for level `d` (see [`keep_excluded`](Self::keep_excluded)) when `kept[d]`.
    spares: Vec<Level<W>>,
    kept: Vec<bool>,
    depth: usize,
    search: Search,
    stats: MatchStats,
}

impl<'g, const W: usize> MatchBound<'g, W> {
    /// A solver for the states of `graph`'s search, positioned at the root with no candidates.
    pub fn new(graph: &'g MatchGraph) -> Self {
        let mut out = MatchBound {
            graph,
            levels: vec![Level::new(graph.len())],
            spares: Vec::new(),
            kept: Vec::new(),
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
        self.kept.fill(false);
        self.levels[0].reset(self.graph, promoted, cands);
    }

    /// Pushes the child of the top level made by promoting `n` (which must succeed in the search's state), with
    /// the candidates `child_cands` (a subset of the top level's).  See the type's docs.
    pub fn push(&mut self, n: u32, child_cands: &Bits<W>) {
        let graph = self.graph;
        let child = self.push_copy();
        child.promote(graph, n);
        child.dropped |= child.cands - *child_cands;
    }

    /// Pushes a copy of the top level with the candidates `excluded` removed: the same state, fewer
    /// candidates.
    pub fn push_excluding(&mut self, excluded: &Bits<W>) {
        let child = self.push_copy();
        child.dropped |= *excluded;
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
        if let Some(kept) = self.kept.get_mut(d + 1) {
            *kept = false;
        }
        &mut self.levels[d + 1]
    }

    /// Returns to the parent of the top level.
    pub fn pop(&mut self) {
        assert!(self.depth > 0, "pop without push");
        if let Some(kept) = self.kept.get_mut(self.depth) {
            *kept = false;
        }
        self.depth -= 1;
    }

    /// The matching bound of the top level with `x` no longer a candidate, on a level pushed for it: then
    /// [`pop`](Self::pop) it, or [`keep_excluded`](Self::keep_excluded) it.
    pub fn value_excluding(&mut self, x: u32) -> u64 {
        self.push_excluding(&Bits::EMPTY.with(x));
        self.value()
    }

    /// Pops the level pushed by [`value_excluding`](Self::value_excluding), keeping it (settled) for its
    /// parent: once the parent is the top level again, with children pushed and popped meanwhile but nothing done
    /// to the parent itself, [`take_kept`](Self::take_kept) makes the kept level the parent, which saves
    /// repairing the parent for the same removal.  A later keep for the same parent replaces it.
    pub fn keep_excluded(&mut self) {
        let d = self.depth;
        assert!(d > 0, "nothing pushed to keep");
        if self.spares.len() < d {
            let size = self.graph.len();
            self.spares.resize_with(d, || Level::new(size));
            self.kept.resize(d, false);
        }
        std::mem::swap(&mut self.levels[d], &mut self.spares[d - 1]);
        self.kept[d - 1] = true;
        self.depth = d - 1;
    }

    /// If a level is kept for the top level, makes it the top level and returns true.
    pub fn take_kept(&mut self) -> bool {
        let d = self.depth;
        if self.kept.get(d) != Some(&true) {
            return false;
        }
        std::mem::swap(&mut self.levels[d], &mut self.spares[d]);
        self.kept[d] = false;
        true
    }

    /// Number of levels above level 0.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Removes `x` from the top level's candidates (no effect if it is not one).
    pub fn remove_candidate(&mut self, x: u32) {
        if x <= Bits::<W>::MAX_VALUE {
            self.levels[self.depth].dropped.insert(x);
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

    /// Whether the top level's moves can all be matched (true for every state the search reaches).
    pub fn is_feasible(&mut self) -> bool {
        self.settle().feasible
    }

    pub fn stats(&self) -> MatchStats {
        self.stats
    }

    fn settle(&mut self) -> &Level<W> {
        let MatchBound { graph, levels, depth, search, stats, .. } = self;
        let level = &mut levels[*depth];
        stats.evaluations += 1;
        level.settle(graph, search, stats);
        level
    }
}

/// The matching bound of one state, with [`MatchBound`] and no search around it: builds the graph with `state`
/// as the root and solves it from scratch.  Equal to [`match_bound`], and faster.
pub fn match_bound_once<const W: usize>(model: &Model, state: &FrameSet<W>, cands: &Bits<W>) -> u64 {
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
    use crate::bounds::{max_additional, new_candidate_list, upper_bound, Preclusions, RunnerUp};
    use crate::json::Json;
    use crate::model::test_support::*;
    use crate::numtheory::Tables;
    use std::collections::HashMap;
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    fn setup<const W: usize>(g: &Golden) -> (Model, FrameSet<W>, Bits<W>) {
        let (model, root) = Model::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
        (model, root, Bits::from_slice(&g.candidates))
    }

    // ---- the definition, by exhaustion ----

    /// Whether each of `movers` can be given its own maximal factor among `factors` (Kuhn's augmenting paths).
    fn covers<const W: usize>(model: &Model, movers: &[u32], factors: &Bits<W>) -> bool {
        fn augment<const W: usize>(
            model: &Model,
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
    fn brute_bound<const W: usize>(model: &Model, state: &FrameSet<W>, cands: &Bits<W>) -> u64 {
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
    fn matchable_but_not_winnable() {
        // Frame 0: factors 4, 6, 10 and moves 12 (pays with 4 or 6) and 20 (4 or 10); frame 1: the factor 30.
        // Promoting 30 adds the move 30 (6 or 10) to frame 0: matchable, 12→4, 20→10, 30→6, but not winnable.
        let columns = vec![vec![4, 6, 10], vec![12, 20], vec![30], vec![]];
        let (model, root) = Model::build::<1>(&Tables::new(30), &columns).unwrap();
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
    type Visit<'v, const W: usize> = dyn FnMut(&mut MatchBound<'_, W>, &FrameSet<W>, &Bits<W>, &Bits<W>) + 'v;

    /// Drives a [`MatchBound`] the way a depth-first search does, along random paths: at each node the expansion
    /// loop takes the largest candidate, sometimes descends into the child it makes, then goes on with the
    /// candidates shrunk.  Evaluations are random too, so changes pile up between them.
    struct Walk<'a, const W: usize> {
        model: &'a Model,
        preclusions: Option<&'a Preclusions<W>>,
        max_total: u32,
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
                assert!(mb.candidates().is_subset(&cands), "the solver's candidates are out of step");
                if self.rng.chance(80) {
                    visit(mb, state, &promoted, &cands);
                }
                cands.remove(n);
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
                    mb.value_excluding(n);
                    mb.keep_excluded();
                }
                if self.rng.chance(40) {
                    if let Some(next) = self.model.promote(state, n) {
                        let next_promoted = promoted.with(n);
                        let max_next = max_additional(self.max_total, &next, &next_promoted, &cands);
                        let child =
                            new_candidate_list(self.model, self.preclusions, n, max_next, &next, &cands, &promoted);
                        let depth = mb.depth();
                        mb.push(n, &child);
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
    fn walks<const W: usize>(g: &Golden, seed: u64, paths: usize, budget: usize, visit: &mut Visit<'_, W>) {
        let (model, root, all) = setup::<W>(g);
        let max_total = root.max_promotions_among(&all).max(0) as u32;
        let preclusions = Preclusions::build(&model, &root, &all, 4);
        let graph = MatchGraph::new(&model, &root);
        let mut mb = MatchBound::new(&graph);
        let mut walk = Walk { model: &model, preclusions: Some(&preclusions), max_total, rng: Rng(seed), budget: 0 };
        for _ in 0..paths {
            mb.reset(&Bits::EMPTY, &all);
            walk.budget = budget;
            walk.explore(&mut mb, &root, Bits::EMPTY, all, visit);
        }
    }

    /// Checks that the top level of `mb` is settled and optimal: its matching and potentials meet the conditions
    /// in the module docs, which prove the matching heaviest, and the bookkeeping agrees with them.
    fn check_certificate<const W: usize>(mb: &MatchBound<'_, W>) {
        let g = mb.graph;
        let lv = &mb.levels[mb.depth];
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
        assert_eq!(chosen.sum(), lv.value);
        assert_eq!(lv.y.iter().sum::<i64>(), matched_weight, "duality gap");
    }

    // ---- correctness ----

    #[test]
    fn reference_meets_the_definition() {
        let mut rng = Rng(0x0BAD_F00D_1234_5678);
        let mut checks = 0;
        for n in golden_ns().into_iter().filter(|&n| (30..=200).contains(&n) && n % 3 == 0) {
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
        assert!(checks > 1000);
    }

    #[test]
    fn incremental_solver_matches_the_reference() {
        let mut checks = 0;
        let mut stats = MatchStats::default();
        for n in golden_ns().into_iter().filter(|&n| n >= 30 && (n % 13 == 0 || [480, 600, 760].contains(&n))) {
            let g = load(n);
            let (paths, budget) = if n < 300 { (6, 150) } else { (2, 60) };
            if n < 512 {
                checks += agreement::<8>(&g, paths, budget, &mut stats);
            } else {
                checks += agreement::<12>(&g, paths, budget, &mut stats);
            }
        }
        eprintln!("incremental against the reference: {checks} evaluations, {stats:?}");
        assert!(checks > 2000);
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
            evaluations = mb.stats();
            checks += 1;
        });
        stats.evaluations += evaluations.evaluations;
        stats.searches += evaluations.searches;
        stats.scanned += evaluations.scanned;
        checks
    }

    #[test]
    fn bound_is_sound() {
        let (mut checks, mut tight, mut everything) = (0, 0, 0);
        for n in golden_ns().into_iter().filter(|&n| (40..=220).contains(&n)) {
            let g = load(n);
            let (model, ..) = setup::<4>(&g);
            walks::<4>(&g, u64::from(n) << 8, 6, 80, &mut |mb, state, promoted, cands| {
                let bound = mb.value();
                // No feasible completion exceeds the count the freedoms allow (they are not revised here), and
                // the cap makes the exhaustive search much faster.
                let cap = state.max_promotions_among(cands).max(0) as u32;
                let best = best_completion(&model, state, cands, cap);
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
            "matching bound soundness: {checks} states, exact at {tight}, all candidates feasible at {everything}"
        );
        assert!(checks > 3000);
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
        let max_total = root.max_promotions_among(&cands).max(0) as u32;
        let preclusions = Preclusions::build(&model, &root, &cands, 8);
        let mut left = cands;
        let mut steps: Vec<(u32, Option<Bits<W>>)> = Vec::new();
        for n in cands.iter_desc() {
            left.remove(n);
            let child = model.promote(&root, n).map(|next| {
                let max_next = max_additional(max_total, &next, &Bits::EMPTY.with(n), &left);
                new_candidate_list(&model, Some(&preclusions), n, max_next, &next, &left, &Bits::EMPTY)
            });
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
            per(
                both.searches - 2 * shrink.searches + before.searches,
                both.scanned - 2 * shrink.scanned + before.scanned,
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

    // ---- how much the matching bound prunes: cargo test --release matchbound_pruning -- --ignored --nocapture ----

    /// A depth-first branch and bound over promotions, with the loop of DESIGN.md ("Search state and order"),
    /// started with the optimum as its incumbent, so that it visits exactly the nodes its bound cannot rule out.
    /// The bound is the `blameGame` bound alone, or with the matching bound evaluated wherever the first fails
    /// to prune.  A node is pruned when its bound cannot beat the incumbent.
    struct Dfs<'a, const W: usize> {
        model: &'a Model,
        preclusions: &'a Preclusions<W>,
        max_total: u32,
        best: u64,
        use_match: bool,
        limit: u64,
        nodes: u64,
        /// Matching bounds evaluated, pruned by them, and lower than the `blameGame` bound.
        evaluations: u64,
        pruned: u64,
        lower: u64,
        /// Over the evaluations: the sums of how far each bound is above the incumbent.
        blame_excess: u64,
        match_excess: u64,
        /// Evaluations where the heaviest matching promotes more candidates than the node may add, and those of
        /// them where the matching bound is not lower than the `blameGame` bound.
        over_cap: u64,
        over_cap_not_lower: u64,
        match_time: Duration,
    }

    impl<const W: usize> Dfs<'_, W> {
        fn run(&mut self, mb: &mut MatchBound<'_, W>, state: &FrameSet<W>, promoted: Bits<W>, mut cands: Bits<W>) {
            let sum = promoted.sum();
            while let Some(n) = cands.max() {
                if self.nodes >= self.limit {
                    return;
                }
                let max_add = max_additional(self.max_total, state, &promoted, &cands);
                let blame = sum + upper_bound(self.model, state, &cands, max_add, RunnerUp::Sound);
                if blame <= self.best || (self.use_match && self.matching_prunes(mb, sum, blame, max_add)) {
                    break;
                }
                cands.remove(n);
                if self.use_match {
                    mb.remove_candidate(n);
                }
                let Some(next) = self.model.promote(state, n) else { continue };
                let next_promoted = promoted.with(n);
                let max_next = max_additional(self.max_total, &next, &next_promoted, &cands);
                let child =
                    new_candidate_list(self.model, Some(self.preclusions), n, max_next, &next, &cands, &promoted);
                let child_sum = sum + u64::from(n);
                let blame = child_sum + upper_bound(self.model, &next, &child, max_next, RunnerUp::Sound);
                if blame <= self.best {
                    continue;
                }
                if self.use_match {
                    let start = Instant::now();
                    mb.push(n, &child);
                    self.match_time += start.elapsed();
                    if !self.matching_prunes(mb, child_sum, blame, max_next) {
                        self.nodes += 1;
                        self.run(mb, &next, next_promoted, child);
                    }
                    mb.pop();
                } else {
                    self.nodes += 1;
                    self.run(mb, &next, next_promoted, child);
                }
            }
        }

        fn matching_prunes(&mut self, mb: &mut MatchBound<'_, W>, sum: u64, blame: u64, max_add: u32) -> bool {
            let start = Instant::now();
            let bound = sum + mb.value();
            self.match_time += start.elapsed();
            self.evaluations += 1;
            self.lower += u64::from(bound < blame);
            if mb.chosen().len() > max_add {
                self.over_cap += 1;
                self.over_cap_not_lower += u64::from(bound >= blame);
            }
            self.blame_excess += blame - self.best;
            self.match_excess += bound.saturating_sub(self.best);
            let prunes = bound <= self.best;
            self.pruned += u64::from(prunes);
            prunes
        }
    }

    fn pruning<const W: usize>(g: &Golden, limit: u64) {
        let (model, unrevised, all) = setup::<W>(g);
        // The search's starting point, as the Java's SearchManager makes it.
        let manager = g.json.get("searchManager").expect("a game that needs search");
        let field = |key: &str| manager.get(key).and_then(Json::as_i64).unwrap_or_else(|| panic!("{key}"));
        let root = unrevised.with_frame_zero_freedoms(field("revisedFrame0Freedoms") as i32);
        let max_total = field("maxTotalPromotions") as u32;
        let cands = root.narrow_candidates(&all);
        let best = u32s(g.json.get("solve").unwrap(), "promotions").iter().map(|&x| u64::from(x)).sum::<u64>();
        let threads = std::thread::available_parallelism().map_or(4, |p| p.get());
        let preclusions = Preclusions::build(&model, &root, &cands, threads);
        let graph = MatchGraph::new(&model, &root);
        let mut mb = MatchBound::new(&graph);
        mb.reset(&Bits::EMPTY, &cands);
        let blame = upper_bound(&model, &root, &cands, max_total, RunnerUp::Sound);
        eprintln!(
            "n={}: optimum {best}; at the root blameGame {blame} (+{}), matching {} (+{})",
            g.n,
            blame - best,
            mb.value(),
            mb.value() - best
        );
        let mut results = Vec::new();
        for use_match in [false, true] {
            let mut dfs = Dfs {
                model: &model,
                preclusions: &preclusions,
                max_total,
                best,
                use_match,
                limit,
                nodes: 0,
                evaluations: 0,
                pruned: 0,
                lower: 0,
                blame_excess: 0,
                match_excess: 0,
                over_cap: 0,
                over_cap_not_lower: 0,
                match_time: Duration::ZERO,
            };
            mb.reset(&Bits::EMPTY, &cands);
            let start = Instant::now();
            dfs.run(&mut mb, &root, Bits::EMPTY, cands);
            let secs = start.elapsed().as_secs_f64();
            let capped = if dfs.nodes >= limit { " (node limit)" } else { "" };
            let label = if use_match { "blameGame + matching" } else { "blameGame alone    " };
            eprintln!("  {label}: {:9} nodes{capped} in {secs:7.2} s", dfs.nodes);
            if use_match {
                let e = dfs.evaluations.max(1) as f64;
                eprintln!(
                    "    {} matching evaluations ({:.2} µs each, {:.2} s in all, {:.2} searches each): \
                     lower than blameGame at {:.1}%, pruning at {:.1}%; mean excess over the optimum: \
                     blameGame {:.1}, matching {:.1}",
                    dfs.evaluations,
                    dfs.match_time.as_secs_f64() * 1e6 / e,
                    dfs.match_time.as_secs_f64(),
                    mb.stats().searches as f64 / e,
                    100.0 * dfs.lower as f64 / e,
                    100.0 * dfs.pruned as f64 / e,
                    dfs.blame_excess as f64 / e,
                    dfs.match_excess as f64 / e
                );
                eprintln!(
                    "    the heaviest matching promotes more than the node may add at {:.1}% (not lower than \
                     blameGame at {:.1}% of those)",
                    100.0 * dfs.over_cap as f64 / e,
                    100.0 * dfs.over_cap_not_lower as f64 / dfs.over_cap.max(1) as f64
                );
            }
            results.push((dfs.nodes, secs));
        }
        let (plain, combined) = (results[0], results[1]);
        eprintln!(
            "  nodes {:.1}x fewer, time {:.2}x",
            plain.0 as f64 / combined.0.max(1) as f64,
            plain.1 / combined.1.max(1e-9)
        );
    }

    #[test]
    #[ignore]
    fn matchbound_pruning() {
        let limit = std::env::var("NODE_LIMIT").ok().and_then(|s| s.parse().ok()).unwrap_or(3_000_000);
        let ns: Vec<u32> = match std::env::var("GAMES") {
            Ok(list) => list.split(',').map(|s| s.trim().parse().expect("GAMES: comma-separated sizes")).collect(),
            Err(_) => vec![480, 500, 600, 640, 660, 700, 720, 760],
        };
        for n in ns {
            let g = load(n);
            if n < 512 {
                pruning::<8>(&g, limit);
            } else {
                pruning::<12>(&g, limit);
            }
        }
    }
}
