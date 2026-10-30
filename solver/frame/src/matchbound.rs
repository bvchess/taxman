//! The matching bound: an upper bound on the sum of the promotions a search state can still make, from relaxing
//! winnability to matchability.  The search's start rounds the root's heaviest matching to a promotion set, the dive
//! ([`crate::search`]); the search bounds its nodes by the GF(2) bound ([`crate::gf2`]), which is never above this one.
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
//! simply and slowly.  [`MatchBound`] is the solver for the dive: it keeps an optimal matching and an optimal
//! dual solution for one state and repairs them after candidates are removed, which is cheap because each removal
//! changes little.  Besides the bound, the dive asks it for the candidates its heaviest matching promotes, `Q*`
//! ([`MatchBound::chosen`]: if they can all be promoted they are the dive's set, else a member of a conflict among them
//! is removed).
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
//! not required.  A state is first given a greedy matching and potentials that break these conditions only at the
//! movers left unmatched ([`MatchBound::reset`]).  Removing a candidate breaks them at two vertices at most:
//!
//! * a candidate that is not promoted in the matching stops being a candidate: nothing breaks;
//! * a candidate that is promoted stops being one: its matched edge goes, and both ends may now be free with a
//!   positive potential.
//!
//! Each vertex where the conditions break is repaired by one Hungarian search: a Dijkstra over reduced costs
//! that grows alternating paths from it until either a free vertex is reached or the potential of a vertex in
//! the tree that is not required reaches zero (that vertex can then give up its partner, or stay free if it is
//! the start); then it shifts the potentials of the tree and flips the alternating path.  It stops early when
//! the change is local, which is the common case.

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
    let frames = state.frames();
    // The movers: (number, frame it pays in, weight), with no weight for the moves, which must be matched.
    let mut movers: Vec<(u32, usize, Option<i64>)> = Vec::new();
    for (i, frame) in frames.iter().enumerate() {
        movers.extend(frame.moves.iter().map(|m| (m, i, None)));
        if i > 0 {
            let promotable = frame.factors.intersect(cands);
            movers.extend(promotable.iter().map(|x| (x, i - 1, Some(i64::from(x)))));
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
    let mut edges = Vec::new(); // (arc, weight, whether the mover is a candidate)
    for &(u, i, weight) in &movers {
        for w in model.graph().factors(u).iter().map(|&w| u32::from(w)) {
            if frames[i].factors.contains(w) {
                let (a, b) = (node[u as usize], node[w as usize]);
                let (from, to) = if even_omega(u) { (a, b) } else { (b, a) };
                let weight_or_big = weight.unwrap_or(big);
                edges.push((net.add(from, to, -weight_or_big), weight_or_big, weight.is_some()));
            }
        }
    }
    net.min_cost_flow();
    let matched: Vec<_> = edges.iter().filter(|&&(arc, _, _)| net.carries(arc)).collect();
    let moves = movers.iter().filter(|m| m.2.is_none()).count();
    if matched.iter().filter(|e| !e.2).count() < moves {
        return 0;
    }
    (matched.iter().map(|e| e.1).sum::<i64>() - big * moves as i64) as u64
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
    /// The neighbors of vertex `v` are `adj[start[v]..start[v + 1]]`: first, up to `split[v]`, the factors it
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

/// The solver's state: the roles, a heaviest matching and the potentials that prove it.
#[derive(Clone, Debug)]
struct Level<const W: usize> {
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
            dirty: true,
        }
    }

    /// The weight of the edges from the mover `u` to its factors: its number if it is a candidate (promoted, if it is
    /// matched over one), else 0.
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
        self.dirty = true;
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
    fn run<const W: usize>(&mut self, g: &MatchGraph, lv: &mut Level<W>, root: usize) -> bool {
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

    /// Adds the inner vertex `a` to the tree at distance `d`, and reaches its neighbors over unmatched edges.
    #[inline(always)]
    fn add_inner<const W: usize>(&mut self, g: &MatchGraph, lv: &Level<W>, a: usize, d: i64) {
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
    fn shift<const W: usize>(&self, lv: &mut Level<W>, end: i64) {
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
    fn flip<const W: usize>(&self, g: &MatchGraph, lv: &mut Level<W>, root: usize, mut b: usize) {
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

/// The matching bound of one state of the search, kept up to date as candidates are removed (see the module docs).
///
/// [`reset`](Self::reset) sets the state (the root with some candidates promoted, and the candidates left),
/// [`remove_candidate`](Self::remove_candidate) shrinks its candidates, and [`value`](Self::value) gives its bound and
/// [`chosen`](Self::chosen) the candidates its heaviest matching promotes.  `reset` and `remove_candidate` only record
/// what changed; `value` and `chosen` do the work.  As the dive drives it ([`crate::search`]):
///
/// ```text
/// let graph = MatchGraph::new(&model, &root);      // once per search
/// let mut mb = MatchBound::new(&graph);
/// mb.reset(&Bits::EMPTY, &cands);                   // the root and its candidates
/// let q = mb.chosen();                              // the heaviest matching's candidates
/// mb.remove_candidate(x);                           // x, of a conflict among them, left out; then `chosen` again
/// ```
///
/// The bound covers the state's own moves and promotions only: a caller adds the sum of the promotions itself.  For a
/// single state, [`match_bound_once`] does all of this in one call.
#[derive(Clone, Debug)]
pub struct MatchBound<'g, const W: usize> {
    graph: &'g MatchGraph,
    level: Level<W>,
    search: Search,
    stats: MatchStats,
}

impl<'g, const W: usize> MatchBound<'g, W> {
    /// A solver for the states of `graph`'s search, positioned at the root with no candidates.
    pub fn new(graph: &'g MatchGraph) -> Self {
        let mut out = MatchBound {
            graph,
            level: Level::new(graph.len()),
            search: Search::default(),
            stats: MatchStats::default(),
        };
        out.reset(&Bits::EMPTY, &Bits::EMPTY);
        out
    }

    /// Starts over at the state reached from the root by promoting `promoted` (which must succeed), with the
    /// candidates `cands`.  The first [`value`](Self::value) then solves it from scratch.
    pub fn reset(&mut self, promoted: &Bits<W>, cands: &Bits<W>) {
        self.level.reset(self.graph, promoted, cands);
    }

    /// Removes `x` from the state's candidates (no effect if it is not one).
    pub fn remove_candidate(&mut self, x: u32) {
        if x <= Bits::<W>::MAX_VALUE {
            self.level.dropped.insert(x);
            self.level.dirty = true;
        }
    }

    /// The matching bound of the state: an upper bound on the sum of the promotions it can still make among its
    /// candidates (not counting those already made).  0 if the state's moves cannot be matched, in which case it has
    /// no completion at all (a state the search reaches always can be).
    pub fn value(&mut self) -> u64 {
        let level = self.settle();
        if level.feasible {
            level.value
        } else {
            0
        }
    }

    /// The candidates that a heaviest matching of the state promotes (their sum is [`value`](Self::value)).
    pub fn chosen(&mut self) -> Bits<W> {
        self.settle().chosen
    }

    pub fn stats(&self) -> MatchStats {
        self.stats
    }

    fn settle(&mut self) -> &Level<W> {
        let MatchBound { graph, level, search, stats } = self;
        stats.evaluations += 1;
        if level.dirty {
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

    fn setup<const W: usize>(g: &Golden) -> (FrameGraph, FrameSet<W>, Bits<W>) {
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

    // ---- driving the solver along search paths ----

    /// What a walk shows its visitor at each evaluation: the solver (set to the node), the node's state, its
    /// promotions, and its candidates.
    type Visit<'v, const W: usize> = dyn FnMut(&mut MatchBound<'_, W>, &FrameSet<W>, &Bits<W>, &Bits<W>) + 'v;

    /// Drives a [`MatchBound`] at the nodes of random depth-first search paths, as the dive drives it at the root: at
    /// each node the solver is set to the node, and the expansion loop takes the largest candidate out of it, and
    /// sometimes another at random, so that removals pile up between evaluations (which are random too); sometimes it
    /// descends into the child that promoting the largest candidate makes, after which the solver is set to the node
    /// again.
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
            mb.reset(&promoted, &cands);
            while let Some(n) = cands.max() {
                if self.budget == 0 {
                    return;
                }
                self.budget -= 1;
                if self.rng.chance(80) {
                    visit(mb, state, &promoted, &cands);
                }
                cands.remove(n);
                mb.remove_candidate(n);
                // Sometimes another candidate goes with it, at random.
                if let Some(x) = cands.nth_largest(self.rng.below(u64::from(cands.len().max(1))) as u32) {
                    if self.rng.chance(30) {
                        cands.remove(x);
                        mb.remove_candidate(x);
                    }
                }
                if self.rng.chance(40) {
                    if let Some(next) = self.model.promote(state, n) {
                        let child = next.narrow_candidates(&cands);
                        if self.rng.chance(60) {
                            self.explore(mb, &next, promoted.with(n), child, visit);
                        } else {
                            mb.reset(&promoted.with(n), &child);
                            visit(mb, &next, &promoted.with(n), &child);
                        }
                        // The node again, from scratch, without the candidates it has gone on without.
                        mb.reset(&promoted, &cands);
                    }
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
        let graph = MatchGraph::new(&model, &root);
        let mut mb = MatchBound::new(&graph);
        let mut walk = Walk { model: &model, rng: Rng(seed), budget: 0 };
        for _ in 0..paths {
            walk.budget = budget;
            walk.explore(&mut mb, &root, Bits::EMPTY, all, visit);
        }
    }

    /// Checks that the state of `mb` is settled and optimal: its matching and potentials meet the conditions in the
    /// module docs, which prove the matching heaviest, and the bookkeeping agrees with them.
    fn check_certificate<const W: usize>(mb: &MatchBound<'_, W>) {
        let (g, lv) = (mb.graph, &mb.level);
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
            assert!(mate == NONE || usize::from(mate) > v || paid, "{} matched to a non-neighbor", g.num[v]);
        }
        assert_eq!(cands, lv.cands);
        assert_eq!(chosen, lv.chosen);
        assert_eq!(chosen.sum(), lv.value);
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
    /// certificate, and a solver started afresh at the same node, and the bound without one candidate (often one the
    /// heaviest matching promotes, as the dive removes them) against the reference; returns the number of evaluations.
    fn agreement<const W: usize>(g: &Golden, paths: usize, budget: usize, stats: &mut MatchStats) -> u64 {
        let (model, root, _) = setup::<W>(g);
        let graph = MatchGraph::new(&model, &root);
        let mut fresh = MatchBound::new(&graph);
        let mut rng = Rng(u64::from(g.n) * 7919);
        let mut checks = 0;
        let mut evaluations = MatchStats::default();
        let mut without_one = 0u64;
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
            let pool = if rng.chance(70) { mb.chosen() } else { *cands };
            if let Some(x) = pool.nth_largest(rng.below(u64::from(pool.len().max(1))) as u32) {
                let mut without = mb.clone();
                without.remove_candidate(x);
                let exact = without.value();
                check_certificate(&without);
                assert!(exact <= fast, "n={}: without {x}: {exact} > {fast}", g.n);
                assert_eq!(exact, match_bound(&model, state, &cands.without(x)), "n={}: without {x}", g.n);
                without_one += 1;
            }
            evaluations = mb.stats();
            checks += 1;
        });
        stats.evaluations += evaluations.evaluations;
        stats.searches += evaluations.searches;
        stats.scanned += evaluations.scanned;
        assert!(without_one > checks / 2);
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

    /// Times the solver at the root of `g`: from scratch, and on the root's expansion loop, where each step removes
    /// the largest candidate and evaluates the root.
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

        // The root's expansion loop, from the root solved.
        mb.reset(&Bits::EMPTY, &cands);
        mb.value();
        let solved = mb.clone();
        let copy_ns = time_per_call(1, || {
            black_box(solved.clone());
        });
        let run = || {
            let mut mb = solved.clone();
            for n in cands.iter_desc() {
                black_box(mb.value());
                mb.remove_candidate(n);
            }
            mb.stats()
        };
        let (before, after) = (solved.stats(), run());
        let steps = cands.len() as f64;
        let ns = time_per_call(1, || {
            black_box(run());
        }) - copy_ns;
        eprintln!("  solver copy:                          {:9.1} µs", copy_ns / 1e3);
        eprintln!(
            "  root, largest candidate removed:      {:9.2} µs per step ({steps} steps; {:.2} searches, {:.1} vertices \
             scanned)",
            ns / 1e3 / steps,
            (after.searches - before.searches) as f64 / steps,
            (after.scanned - before.scanned) as f64 / steps
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
