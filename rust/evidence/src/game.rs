//! The full game, model-free: an exact upper bound on opt(n) that uses only proven facts.
//!
//! Nothing here knows about hives, frames, greedy, `unzip` or promotions.  It rests on:
//!
//! * **Theorem 1 with peel** (THEORY.md): a set S of picks can all be played iff dismantling the reduction
//!   instance whose members are S, each listing its maximal factors outside S, removes everything.  (Peel
//!   succeeds iff S is playable, Theorems 2 and 3; with every list a set of maximal factors and no member in any
//!   list, playable implies no self-covering subset, Theorem 5 (⇒), and a full dismantling order rebuilds S by
//!   the extension lemma, whose proof needs exactly that no member is in a list.)  So opt(n) is the largest sum
//!   of a playable S ⊆ {2..n}, and every subset of a playable set is playable.
//! * **The Franklín–Moniot matching bound**: a playable S pays each pick with a distinct maximal factor outside
//!   S (Theorem 1), so S is the pick side of a matching in the graph joining each c to its maximal factors c/p,
//!   with each number in at most one edge, weighing the larger end.  The graph is bipartite by the parity of Ω.
//! * **opt(n) <= n + opt(n−1)** (deleting the move n from a game of n leaves a game of n − 1), and for a prime n,
//!   **opt(n) <= n + opt(n−1) − p** with p the largest prime below n (a game playing n plays it first, paying 1;
//!   swapping it for p, which divides nothing up to n − 1 by Bertrand, gives a game of n − 1).
//!
//! [`Prover`] decides "is there a playable S with sum > V?" by branch and bound over the numbers' roles: each
//! number is a forced pick, free, or barred from picking.  A node's bound is the maximum matching with the forced
//! picks matched downward and the barred ones only as payments; if the matching's picks are playable they are a
//! game worth the bound; otherwise they contain an unplayable subset K (shrunk to a minimal one), every
//! playable set leaves out some member of K, and child j bars the j-th free member of K and forces the ones
//! before it.  With [`Prover::enumerate`] it lists every playable set with sum > V instead of stopping at the
//! first (with V = opt(n) − 1, every optimal set).  With [`Prover::weighted`] a set's worth is the sum of other
//! positive weights of its picks: with every weight 1, "is there a playable S with more than V picks?" (the
//! longest games, [`crate::moves`]).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::parity::{Meet, Outcome, Parity, Warm};
use crate::primes::{is_prime, maximal_factors};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Free,
    /// Must be picked.
    Pick,
    /// May not be picked (may still pay).
    Barred,
}

/// The board 1..=n: maximal factors and the parity of Ω.
pub struct Board {
    pub n: u32,
    pub mf: Vec<Vec<u32>>,
    /// For each number, its maximal composites c·p <= n.
    pub mc: Vec<Vec<u32>>,
    pub even: Vec<bool>,
}

impl Board {
    pub fn new(n: u32, spf: &[u32]) -> Self {
        let size = n as usize + 1;
        let mut mf = vec![Vec::new(); size];
        let mut mc = vec![Vec::new(); size];
        let mut omega = vec![0u32; size];
        for c in 2..=n {
            let p = spf[c as usize];
            omega[c as usize] = omega[(c / p) as usize] + 1;
            mf[c as usize] = maximal_factors(u64::from(c), spf).into_iter().map(|f| f as u32).collect();
            for &f in &mf[c as usize] {
                mc[f as usize].push(c);
            }
        }
        let even = omega.iter().map(|o| o % 2 == 0).collect();
        Board { n, mf, mc, even }
    }

    /// Plays `moves` by the rules (each move needs a proper divisor in the pot and takes all of them); the score.
    #[allow(clippy::needless_range_loop)]
    pub fn replay(&self, moves: &[u32]) -> Result<u64, String> {
        let n = self.n as usize;
        let mut pot = vec![true; n + 1];
        pot[0] = false;
        let mut score = 0u64;
        for &c in moves {
            let c = c as usize;
            if c > n || !pot[c] {
                return Err(format!("{c} is not in the pot"));
            }
            let mut taxed = false;
            for d in 1..=c / 2 {
                if c.is_multiple_of(d) && pot[d] {
                    pot[d] = false;
                    taxed = true;
                }
            }
            if !taxed {
                return Err(format!("{c} has no divisor left"));
            }
            pot[c] = false;
            score += c as u64;
        }
        Ok(score)
    }

    /// Whether the picks flagged in `pick` can all be played: dismantle the instance whose members are the
    /// picks, each listing its maximal factors that are not picks.
    pub fn playable(&self, pick: &[bool]) -> bool {
        self.residual(pick).iter().all(|&x| !x)
    }

    /// The members left by dismantling the picks (empty iff playable).
    #[allow(clippy::needless_range_loop)]
    pub fn residual(&self, pick: &[bool]) -> Vec<bool> {
        let n = self.n as usize;
        let mut alive = pick.to_vec();
        let mut degree = vec![0u32; n + 1];
        for c in 2..=n {
            if alive[c] {
                for &f in &self.mf[c] {
                    if !pick[f as usize] {
                        degree[f as usize] += 1;
                    }
                }
            }
        }
        let mut queue: Vec<u32> = (2..=n as u32)
            .filter(|&c| {
                alive[c as usize] && self.mf[c as usize].iter().any(|&f| !pick[f as usize] && degree[f as usize] == 1)
            })
            .collect();
        while let Some(c) = queue.pop() {
            if !alive[c as usize] {
                continue;
            }
            alive[c as usize] = false;
            for &f in &self.mf[c as usize] {
                if pick[f as usize] {
                    continue;
                }
                let d = &mut degree[f as usize];
                *d -= 1;
                if *d == 1 {
                    let other = self.mc[f as usize].iter().find(|&&m| alive[m as usize]);
                    queue.push(*other.expect("a factor of degree 1 has a live lister"));
                }
            }
        }
        alive
    }

    /// Shrinks an unplayable set of picks to an inclusion-minimal unplayable subset (every proper subset of it is
    /// playable), trying first the members its dismantling removes.
    pub fn minimal_unplayable(&self, pick: &[bool]) -> Vec<u32> {
        let residual = self.residual(pick);
        let mut set = pick.to_vec();
        let order: Vec<u32> = (2..=self.n)
            .filter(|&c| pick[c as usize] && !residual[c as usize])
            .chain((2..=self.n).rev().filter(|&c| residual[c as usize]))
            .collect();
        for c in order {
            set[c as usize] = false;
            if self.playable(&set) {
                set[c as usize] = true;
            }
        }
        (2..=self.n).filter(|&c| set[c as usize]).collect()
    }
}

/// The matching bound: a maximum-weight matching in the bipartite graph of the board (even Ω on the left, odd
/// on the right), as a min-cost flow by successive shortest paths with potentials (Dijkstra on reduced costs),
/// stopping when no augmenting path has negative cost.  Edges (c, c/p) weigh the weight of picking c (c itself
/// unless [`set_weights`](Self::set_weights) or [`set_scale`](Self::set_scale) says otherwise); a forced pick's
/// downward edges weigh BIG more and its upward ones are dropped (it may not pay); a barred number's downward
/// edges are dropped (it may not be picked).
pub struct MatchingBound {
    /// Neighbours of each number (its maximal factors and maximal composites).
    adj: Vec<Vec<u32>>,
    dist: Vec<i64>,
    pot: Vec<i64>,
    prev: Vec<u32>,
    mate: Vec<u32>,
    /// The weight of picking each number (positive, and well below BIG in total).
    pick_weight: Vec<i64>,
    /// Subtracted from the weight of each number picked (zero but in Lagrangian experiments).
    pub penalty: Vec<i64>,
    /// The optimal dual of the last solve, and its total (the bound plus BIG per forced pick).
    dual: Vec<i64>,
    total: i64,
    /// BIG times the number of forced picks at the last solve.
    offset: i64,
}

const NONE: u32 = u32::MAX;
const BIG: i64 = 1 << 40;
const INF: i64 = i64::MAX / 4;

impl MatchingBound {
    pub fn new(board: &Board) -> Self {
        let size = board.n as usize + 1;
        let mut adj = vec![Vec::new(); size];
        for c in 2..=board.n {
            for &f in &board.mf[c as usize] {
                adj[c as usize].push(f);
                adj[f as usize].push(c);
            }
        }
        MatchingBound {
            adj,
            dist: vec![INF; size],
            pot: vec![0; size],
            prev: vec![NONE; size],
            mate: vec![NONE; size],
            pick_weight: (0..size as i64).collect(),
            penalty: vec![0; size],
            dual: vec![0; size],
            total: 0,
            offset: 0,
        }
    }

    /// Weighs picks above n/2 by `upper` times their value and the others by `lower` times theirs (the value of
    /// a solve is then in those units).
    pub fn set_scale(&mut self, upper: i64, lower: i64) {
        let n = self.pick_weight.len() as i64 - 1;
        for (c, w) in self.pick_weight.iter_mut().enumerate() {
            let c = c as i64;
            *w = c * if 2 * c > n { upper } else { lower };
        }
    }

    /// Weighs the pick of each number c by `weights[c]` (positive), instead of by c.
    pub fn set_weights(&mut self, weights: &[i64]) {
        assert_eq!(weights.len(), self.pick_weight.len(), "one weight per number 0..=n");
        self.pick_weight.copy_from_slice(weights);
    }

    /// The weight of edge {u, v} (u, v adjacent), or None if the roles forbid it.
    fn weight(&self, roles: &[Role], u: u32, v: u32) -> Option<i64> {
        let (hi, lo) = if u > v { (u, v) } else { (v, u) };
        if roles[hi as usize] == Role::Barred || roles[lo as usize] == Role::Pick {
            return None;
        }
        let bonus = if roles[hi as usize] == Role::Pick { BIG } else { 0 };
        Some(self.pick_weight[hi as usize] + bonus - self.penalty[hi as usize])
    }

    /// The bound and the matching's picks, or None if some forced pick cannot be matched.
    pub fn solve(&mut self, board: &Board, roles: &[Role]) -> Option<(u64, Vec<bool>)> {
        let n = board.n as usize;
        self.mate.iter_mut().for_each(|m| *m = NONE);
        // Potentials (s at 0): left 0; right at most −w over its edges; t at most every right's.
        let mut pot_t = 0i64;
        for v in 1..=n {
            self.pot[v] = 0;
            if !board.even[v] {
                let heaviest = self.adj[v].iter().filter_map(|&u| self.weight(roles, u, v as u32)).max();
                self.pot[v] = -heaviest.unwrap_or(0);
                pot_t = pot_t.min(self.pot[v]);
            }
        }
        let mut heap: BinaryHeap<Reverse<(i64, u32)>> = BinaryHeap::new();
        loop {
            for v in 1..=n {
                self.dist[v] = INF;
                self.prev[v] = NONE;
            }
            heap.clear();
            for v in 1..=n {
                if board.even[v] && self.mate[v] == NONE {
                    self.dist[v] = -self.pot[v];
                    heap.push(Reverse((self.dist[v], v as u32)));
                }
            }
            let mut dist_t = INF;
            let mut last = NONE;
            while let Some(Reverse((d, u))) = heap.pop() {
                let u = u as usize;
                if d > self.dist[u] {
                    continue;
                }
                if board.even[u] {
                    for i in 0..self.adj[u].len() {
                        let r = self.adj[u][i];
                        if self.mate[u] == r {
                            continue;
                        }
                        let Some(w) = self.weight(roles, u as u32, r) else { continue };
                        let nd = d + (-w + self.pot[u] - self.pot[r as usize]);
                        debug_assert!(nd >= d, "reduced costs are nonnegative");
                        if nd < self.dist[r as usize] {
                            self.dist[r as usize] = nd;
                            self.prev[r as usize] = u as u32;
                            heap.push(Reverse((nd, r)));
                        }
                    }
                } else {
                    let l = self.mate[u];
                    if l == NONE {
                        let nd = d + self.pot[u] - pot_t;
                        if nd < dist_t {
                            dist_t = nd;
                            last = u as u32;
                        }
                        continue;
                    }
                    let w = self.weight(roles, l, u as u32).expect("matched edges are allowed");
                    let nd = d + (w + self.pot[u] - self.pot[l as usize]);
                    if nd < self.dist[l as usize] {
                        self.dist[l as usize] = nd;
                        self.prev[l as usize] = u as u32;
                        heap.push(Reverse((nd, l)));
                    }
                }
            }
            // The cheapest augmenting path costs dist_t + pot_t (s has potential 0); stop unless it gains.
            if last == NONE || dist_t + pot_t >= 0 {
                // Optimal.  Shift the potentials by min(dist, D) with D = −pot_t, which keeps every residual
                // reduced cost nonnegative and brings t level with s; then y = max(p, 0) on the left and
                // max(−p, 0) on the right is an optimal dual (y_l + y_r >= w on every edge, equality on the
                // matching, zero on free vertices), as complementary slackness on the residual arcs shows.
                let shift = -pot_t;
                self.total = 0;
                for v in 1..=n {
                    let p = self.pot[v] + self.dist[v].min(shift);
                    self.dual[v] = if board.even[v] { p.max(0) } else { (-p).max(0) };
                    self.total += self.dual[v];
                }
                break;
            }
            for v in 1..=n {
                self.pot[v] += self.dist[v].min(dist_t);
            }
            pot_t += dist_t;
            let mut r = last as usize;
            loop {
                let l = self.prev[r] as usize;
                let next = self.prev[l];
                self.mate[l] = r as u32;
                self.mate[r] = l as u32;
                if next == NONE {
                    break;
                }
                r = next as usize;
            }
        }
        // `repair` expects every distance at INF.
        self.dist.iter_mut().for_each(|d| *d = INF);
        self.read(board, roles)
    }

    /// The mate of `v` in the current matching (NONE if unmatched).
    pub fn mate_of(&self, v: u32) -> u32 {
        self.mate[v as usize]
    }

    /// The neighbours of `v` in the graph (its maximal factors and composites).
    pub fn neighbours(&self, v: u32) -> &[u32] {
        &self.adj[v as usize]
    }

    /// The matching and dual, to resume from with [`update`](Self::update).
    pub fn snapshot(&self) -> (Vec<u32>, Vec<i64>) {
        (self.mate.clone(), self.dual.clone())
    }

    /// Re-optimizes after the roles changed from `old` to `new` (numbers only ever going from free to forced or
    /// barred), starting from `snapshot`, the optimal matching and dual for `old`.  Barring a number removes its
    /// downward edges, forcing one removes its upward edges and raises its downward ones by BIG (its dual rises
    /// by BIG too, keeping the dual feasible); a matched edge removed frees its ends.  Each free vertex left with
    /// a positive dual is then repaired by one Hungarian step ([`repair`](Self::repair)).  Returns the same as
    /// [`solve`](Self::solve).
    pub fn update(
        &mut self,
        board: &Board,
        snapshot: &(Vec<u32>, Vec<i64>),
        old: &[Role],
        new: &[Role],
    ) -> Option<(u64, Vec<bool>)> {
        self.update_with(board, snapshot, old, new, None)
    }

    /// [`update`](Self::update), where the penalties also change from the current ones to `penalty` (only ever
    /// downwards: a lower penalty raises the weights of the number's downward edges, so its dual rises by as
    /// much, and an upward edge it was matched by is no longer tight and is freed).
    pub fn update_with(
        &mut self,
        board: &Board,
        snapshot: &(Vec<u32>, Vec<i64>),
        old: &[Role],
        new: &[Role],
        penalty: Option<&[i64]>,
    ) -> Option<(u64, Vec<bool>)> {
        self.mate.clone_from(&snapshot.0);
        self.dual.clone_from(&snapshot.1);
        let mut deficient: Vec<u32> = Vec::new();
        if let Some(penalty) = penalty {
            for c in 2..=board.n {
                let delta = self.penalty[c as usize] - penalty[c as usize];
                assert!(delta >= 0, "penalties only fall");
                if delta == 0 {
                    continue;
                }
                self.penalty[c as usize] = penalty[c as usize];
                if old[c as usize] == Role::Barred {
                    continue;
                }
                self.dual[c as usize] += delta;
                let m = self.mate[c as usize];
                if m != NONE && m > c {
                    self.mate[c as usize] = NONE;
                    self.mate[m as usize] = NONE;
                    deficient.push(m);
                }
                deficient.push(c);
            }
        }
        for c in 2..=board.n {
            let (was, is) = (old[c as usize], new[c as usize]);
            if was == is {
                continue;
            }
            assert_eq!(was, Role::Free, "roles only go from free to forced or barred");
            let m = self.mate[c as usize];
            match is {
                Role::Barred => {
                    if m != NONE && m < c {
                        self.mate[c as usize] = NONE;
                        self.mate[m as usize] = NONE;
                        deficient.extend([c, m]);
                    }
                }
                Role::Pick => {
                    if m != NONE && m > c {
                        self.mate[c as usize] = NONE;
                        self.mate[m as usize] = NONE;
                        deficient.push(m);
                    }
                    self.dual[c as usize] += BIG;
                    deficient.push(c);
                }
                Role::Free => unreachable!(),
            }
        }
        for v in deficient {
            if self.mate[v as usize] == NONE && self.dual[v as usize] > 0 {
                self.repair(board, new, v);
            }
        }
        self.read(board, new)
    }

    /// One Hungarian step from a free vertex with a positive dual: Dijkstra over the alternating paths from it
    /// on edge slacks y_u + y_v − w (matching edges cost 0), stopping at the cheapest of: an unmatched vertex
    /// of the other side (augment to it), or a vertex of the root's side whose dual reaches 0 first (flip the
    /// path to it, freeing it; possibly the root itself).  The duals move by δ − dist on the vertices closer than
    /// δ (down on the root's side, up on the other), which keeps them feasible and makes the path tight.
    fn repair(&mut self, board: &Board, roles: &[Role], root: u32) {
        let side = board.even[root as usize];
        let mut touched: Vec<u32> = vec![root];
        self.dist[root as usize] = 0;
        self.prev[root as usize] = NONE;
        let mut heap: BinaryHeap<Reverse<(i64, u32)>> = BinaryHeap::new();
        heap.push(Reverse((0, root)));
        let mut best = (self.dual[root as usize], root);
        while let Some(Reverse((d, u))) = heap.pop() {
            if d >= best.0 {
                break;
            }
            if d > self.dist[u as usize] {
                continue;
            }
            if board.even[u as usize] == side {
                let end = d + self.dual[u as usize];
                if end < best.0 {
                    best = (end, u);
                }
                for i in 0..self.adj[u as usize].len() {
                    let b = self.adj[u as usize][i];
                    if self.mate[u as usize] == b {
                        continue;
                    }
                    let Some(w) = self.weight(roles, u, b) else { continue };
                    let nd = d + self.dual[u as usize] + self.dual[b as usize] - w;
                    debug_assert!(nd >= d, "the dual is feasible");
                    if self.dist[b as usize] == INF {
                        touched.push(b);
                    }
                    if nd < self.dist[b as usize] {
                        self.dist[b as usize] = nd;
                        self.prev[b as usize] = u;
                        heap.push(Reverse((nd, b)));
                    }
                }
            } else {
                let x = self.mate[u as usize];
                if x == NONE {
                    if d < best.0 {
                        best = (d, u);
                    }
                    continue;
                }
                if self.dist[x as usize] == INF {
                    touched.push(x);
                }
                if d < self.dist[x as usize] {
                    self.dist[x as usize] = d;
                    self.prev[x as usize] = u;
                    heap.push(Reverse((d, x)));
                }
            }
        }
        let (delta, end) = best;
        for &v in &touched {
            let d = self.dist[v as usize];
            if d < delta {
                if board.even[v as usize] == side {
                    self.dual[v as usize] -= delta - d;
                } else {
                    self.dual[v as usize] += delta - d;
                }
            }
        }
        if board.even[end as usize] == side {
            // Flip root → … → end; end (whose dual is now 0) comes out free.
            if end != root {
                let mut x = end;
                self.mate[end as usize] = NONE;
                while x != root {
                    let b = self.prev[x as usize];
                    let px = self.prev[b as usize];
                    self.mate[px as usize] = b;
                    self.mate[b as usize] = px;
                    x = px;
                }
            }
        } else {
            let mut b = end;
            loop {
                let x = self.prev[b as usize];
                let next = self.prev[x as usize];
                self.mate[x as usize] = b;
                self.mate[b as usize] = x;
                if x == root {
                    break;
                }
                b = next;
            }
        }
        for &v in &touched {
            self.dist[v as usize] = INF;
        }
    }

    /// The matching's value and picks under `roles`, or None if a forced pick is unmatched.
    fn read(&mut self, board: &Board, roles: &[Role]) -> Option<(u64, Vec<bool>)> {
        let n = board.n as usize;
        let mut value: i64 = 0;
        let mut picks = vec![false; n + 1];
        for v in (1..=n).filter(|&v| board.even[v]) {
            let m = self.mate[v];
            if m != NONE {
                value += self.weight(roles, v as u32, m).expect("matched edges are allowed");
                picks[v.max(m as usize)] = true;
            }
        }
        self.total = self.dual[1..=n].iter().sum();
        debug_assert_eq!(value, self.total, "complementary slackness");
        if (2..=n).any(|c| roles[c] == Role::Pick && !picks[c]) {
            return None;
        }
        let forced = roles.iter().filter(|&&r| r == Role::Pick).count() as i64;
        self.offset = forced * BIG;
        Some(((value - self.offset) as u64, picks))
    }

    /// After [`solve`](Self::solve): an upper bound on the bound with the pick `c` barred, from the optimal dual
    /// alone.  Lower c's dual, and its payment's, as far as their remaining edges allow (c keeps only its upward
    /// edges; its payment loses the edge to c); the result is still dual feasible.
    pub fn bound_barring(&self, roles: &[Role], c: u32) -> i64 {
        let lowest = |v: u32, skip: u32, roles: &[Role]| -> i64 {
            self.adj[v as usize]
                .iter()
                .filter(|&&u| u != skip && !(u < v && v == c))
                .filter_map(|&u| self.weight(roles, v, u).map(|w| w - self.dual[u as usize]))
                .max()
                .unwrap_or(0)
                .max(0)
        };
        let m = self.mate[c as usize];
        let mut drop = self.dual[c as usize] - lowest(c, m, roles);
        if m != NONE && m < c {
            drop += self.dual[m as usize] - lowest(m, c, roles);
        }
        self.total - drop - self.offset
    }
}

/// What the branch and bound concluded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// No playable set beats the target: opt(n) <= target.  (Enumerating: every one that does is in
    /// [`Prover::found`].)
    Proven,
    /// A playable set with a larger sum (its picks).  (Enumerating: the one that reached the limit.)
    Beaten(Vec<u32>),
    /// The node budget ran out.
    Budget,
    /// Paused at the deadline; the open units are in [`Prover::open_units`].
    Paused,
}

/// The branch and bound.  Besides the bound and the conflict branching described at the top:
///
/// * **Forced picks.**  For each free pick of the matching, the optimal dual bounds the matching with that pick
///   barred ([`MatchingBound::bound_barring`]); if that is at most the target, every set beating the target
///   picks it, so it is forced in this node and all its children.
/// * **Nogoods.**  Every minimal unplayable set found is kept (it is unplayable at every node, since a set of
///   picks is unplayable whatever else is picked).  A nogood all of whose members are forced ends the node; one
///   with a single member not forced and free bars that member; and the matching's picks are checked against
///   the nogoods before a new conflict is computed.
/// * **Segment cuts** (with [`with_smaller_games`](Self::with_smaller_games)).  The picks of a playable S
///   that are at most m are playable on the board 1..=m (a subset of a playable set is playable, and their
///   maximal factors are all on that board), so they sum to at most opt(m).  Given opt(m) <= V(m) for every
///   m < n, a set beating the target T has sum(S ∩ (m, n]) >= T + 1 − V(m) for every m: a node whose
///   unbarred numbers above some m fall short is cut, and a number whose barring would make one fall short is
///   forced.  (For m = n − 1 this says S contains n.)  The answer then rests on those smaller games' bounds.
/// * **A Lagrangian bound** (with [`enable_lagrangian`](Self::enable_lagrangian)).  Every playable set picks
///   at most |K| − 1 members of each nogood K, so for any multipliers μ_K >= 0 the matching with each pick's
///   weight lowered by the μ of the nogoods holding it, plus Σ μ_K (|K| − 1), bounds every set the search
///   looks for.  The multipliers are fitted once (subgradient steps at the root, over the nogoods known then)
///   and fixed; the second matching runs alongside the first, incrementally the same way, and cuts nodes and
///   forces picks by the same rules.
/// * **Enumeration** (with [`enumerate`](Self::enumerate)).  Every rule above cuts only sets with sum <= target,
///   so the search can go on past a playable set P it finds: P is recorded, and the node's other sets are
///   partitioned by the first free number on which they disagree with P, taking the numbers outside P first
///   (child j forces the j-th of them and bars those before it), then P's own (all outside P barred, child j bars
///   the j-th member of P and forces those before it: subsets of P, so lighter than P).
pub struct Prover<'a> {
    board: &'a Board,
    bound: MatchingBound,
    lag: Option<Lag>,
    parity: Option<Meet>,
    /// Whether a parity solve starts from the parent node's certificate (on by default).
    parity_warm: bool,
    /// The weight of picking each number (c by default; see [`weighted`](Self::weighted)).
    weights: Vec<i64>,
    target: u64,
    /// For each m < n, an upper bound on the weight of a playable set of the board 1..=m (empty: no segment cuts).
    smaller: Vec<i64>,
    pub nogoods: Vec<Vec<u32>>,
    max_nogoods: usize,
    pub nodes: u64,
    /// Diagnostics: [new conflicts, known conflicts, forced picks, children, cut by bound, cut by the Lagrangian,
    /// cut by the parity bound, parity bounds not certified].
    pub stats: [u64; 8],
    pause_at: Option<std::time::Instant>,
    next_check: u64,
    paused: bool,
    pending: Vec<Vec<Role>>,
    budget: u64,
    /// The playable sets beating the target found so far, and how many to find before stopping (1 unless
    /// enumerating).
    pub found: Vec<Vec<u32>>,
    limit: usize,
    /// Raise the target to each set found and go on (see [`optimize`](Self::optimize)).
    optimizing: bool,
}

/// The Lagrangian matching: weights scaled by `scale`, picks penalized by the multipliers of their *active*
/// nogoods (those with no barred member: a nogood with one is satisfied by every set the node allows, so its
/// term only loosens the bound and is dropped), plus Σ μ_K (|K| − 1) over the active ones.
struct Lag {
    m: MatchingBound,
    scale: i64,
    nogoods: Vec<Vec<u32>>,
    mu: Vec<i64>,
}

impl Lag {
    /// The penalties and the constant under `roles`.
    fn terms(&self, roles: &[Role]) -> (Vec<i64>, i64) {
        let mut penalty = vec![0i64; roles.len()];
        let mut constant = 0;
        for (k, &u) in self.nogoods.iter().zip(&self.mu) {
            if u == 0 || k.iter().any(|&c| roles[c as usize] == Role::Barred) {
                continue;
            }
            k.iter().for_each(|&c| penalty[c as usize] += u);
            constant += u * (k.len() as i64 - 1);
        }
        (penalty, constant)
    }

    /// Re-optimizes from a parent's snapshot (or solves afresh), with the penalties of `roles`; returns the
    /// result and the constant.
    fn solve(
        &mut self,
        board: &Board,
        base: Option<(&[Role], &Snapshot)>,
        roles: &[Role],
    ) -> (Option<(u64, Vec<bool>)>, i64) {
        let (penalty, constant) = self.terms(roles);
        let result = match base {
            Some((old, snapshot)) => {
                self.m.penalty = self.terms(old).0;
                self.m.update_with(board, snapshot, old, roles, Some(&penalty))
            }
            None => {
                self.m.penalty = penalty;
                self.m.solve(board, roles)
            }
        };
        (result, constant)
    }
}

/// A parent's state for its children: its roles, the optimal matching and dual of each bound, and the parity
/// bound's certificate (a child starts its parity solve from it).
type Snapshot = (Vec<u32>, Vec<i64>);
struct Base<'b> {
    roles: &'b [Role],
    plain: &'b Snapshot,
    lag: Option<&'b Snapshot>,
    parity: Option<&'b Warm>,
}

impl<'a> Prover<'a> {
    pub fn new(board: &'a Board, target: u64, budget: u64) -> Self {
        Prover {
            board,
            bound: MatchingBound::new(board),
            lag: None,
            parity: None,
            parity_warm: true,
            weights: (0..=i64::from(board.n)).collect(),
            target,
            smaller: Vec::new(),
            nogoods: Vec::new(),
            max_nogoods: 20_000,
            nodes: 0,
            stats: [0; 8],
            pause_at: None,
            next_check: 256,
            paused: false,
            pending: Vec::new(),
            budget,
            found: Vec::new(),
            limit: 1,
            optimizing: false,
        }
    }

    /// Lists the playable sets beating the target instead of stopping at the first: the search stops at the
    /// `limit`-th (returning it as [`Verdict::Beaten`]) or returns [`Verdict::Proven`] with every one of them in
    /// [`found`](Self::found).
    pub fn enumerate(mut self, limit: usize) -> Self {
        assert!(limit >= 1 && !self.optimizing);
        self.limit = limit;
        self
    }

    /// Finds the heaviest playable set instead of stopping at the first beating the target: each one found raises
    /// the target to its weight, and the search goes on (every rule above cuts only sets no heavier than the
    /// target, so what was cut stays cut as the target rises).  [`Verdict::Proven`] then means that nothing beats
    /// the final target, and the last of [`found`](Self::found), if any, is a heaviest set.
    pub fn optimize(mut self) -> Self {
        assert!(self.limit == 1);
        self.optimizing = true;
        self
    }

    /// Weighs the pick of each number c by `weights[c]` (positive, one per number 0..=n) instead of by c: the
    /// search then decides whether some playable set's weight exceeds the target, and every rule above holds with
    /// "weight" for "sum" (each cuts only sets no heavier than the target).  Call it before the other builders
    /// and before running.
    pub fn weighted(mut self, weights: Vec<i64>) -> Self {
        assert!(weights.len() == self.board.n as usize + 1 && weights[2..].iter().all(|&w| w > 0));
        assert!(self.lag.is_none() && self.parity.is_none() && self.nodes == 0, "set the weights before anything else");
        self.bound.set_weights(&weights);
        self.weights = weights;
        self
    }

    /// Turns on the parity bound ([`crate::parity`], THEORY.md Theorem 10) at every node, after the matching
    /// bound: the heaviest set independent in both parity matroids with the node's forced picks contracted and
    /// its barred numbers deleted, certified by a weight split.  It cuts the node when it is at most the target;
    /// it bars every free number that depends on the forced picks in one of the matroids (no playable set of the
    /// node holds it) and forces every member whose barring alone brings it to the target (by the split); and
    /// the node branches on a minimal unplayable subset of its heaviest set instead of the matching's picks.
    pub fn with_parity(mut self) -> Self {
        self.parity = Some(Meet::new(Parity::pair(self.board, &self.weights)));
        self
    }

    /// Solves every parity bound from scratch instead of from the parent node's certificate (to measure the warm
    /// starts; the bounds are the same).
    pub fn parity_cold(mut self) -> Self {
        self.parity_warm = false;
        self
    }

    /// The parity bound's counters, if it is on.
    pub fn parity_stats(&self) -> Option<crate::parity::MeetStats> {
        self.parity.as_ref().map(|m| m.stats)
    }

    /// Turns on the segment cuts, given upper bounds `smaller[m]` on opt(m) for m = 0..n−1 (under the weights:
    /// with unit weights, on the most moves of game m).
    pub fn with_smaller_games(mut self, smaller: &[u64]) -> Self {
        let n = self.board.n as usize;
        assert!(smaller.len() >= n);
        self.smaller = smaller[..n].iter().map(|&w| w as i64).collect();
        self
    }

    /// The segment cuts: false if some segment (m, n] cannot reach its need; otherwise forces every free
    /// number whose barring would leave a segment short, setting `changed`.
    fn segments(&self, roles: &mut [Role], changed: &mut bool) -> bool {
        if self.smaller.is_empty() {
            return true;
        }
        let n = self.board.n as usize;
        // slack[m] = weight of unbarred numbers in (m, n] − need[m]; a free c is forced iff its weight > the least
        // slack over the segments containing it (m < c).
        let mut slack = vec![0i64; n];
        let mut avail = 0i64;
        for m in (1..n).rev() {
            if roles[m + 1] != Role::Barred {
                avail += self.weights[m + 1];
            }
            slack[m] = avail - (self.target as i64 + 1 - self.smaller[m]);
            if slack[m] < 0 {
                return false;
            }
        }
        let mut least = i64::MAX;
        for c in 2..=n {
            least = least.min(slack[c - 1]);
            if roles[c] == Role::Free && self.weights[c] > least {
                roles[c] = Role::Pick;
                *changed = true;
            }
        }
        true
    }

    /// Fits Lagrangian multipliers for the nogoods known now (subgradient steps at the root, keeping the best)
    /// and turns on the second bound; returns the root's Lagrangian bound.  Without nogoods it does nothing.
    pub fn enable_lagrangian(&mut self, iterations: usize) -> Option<f64> {
        const SCALE: i64 = 100;
        if self.nogoods.is_empty() {
            return None;
        }
        let roles = self.root_roles();
        let ks = self.nogoods.clone();
        let mut m = MatchingBound::new(self.board);
        m.set_weights(&self.weights.iter().map(|w| w * SCALE).collect::<Vec<_>>());
        let set = |m: &mut MatchingBound, mu: &[i64]| -> i64 {
            m.penalty.iter_mut().for_each(|p| *p = 0);
            let mut constant = 0;
            for (k, &u) in ks.iter().zip(mu) {
                k.iter().for_each(|&c| m.penalty[c as usize] += u);
                constant += u * (k.len() as i64 - 1);
            }
            constant
        };
        let mut mu = vec![0.0f64; ks.len()];
        let mut best = (i64::MAX, vec![0i64; ks.len()]);
        for _ in 0..iterations {
            let ints: Vec<i64> = mu.iter().map(|u| (u * SCALE as f64).round() as i64).collect();
            let constant = set(&mut m, &ints);
            let (value, picks) = m.solve(self.board, &roles).expect("nothing is forced");
            let bound = value as i64 + constant;
            if bound < best.0 {
                best = (bound, ints);
            }
            let g: Vec<f64> = ks
                .iter()
                .map(|k| k.iter().filter(|&&c| picks[c as usize]).count() as f64 - (k.len() as f64 - 1.0))
                .collect();
            let norm: f64 = g.iter().map(|x| x * x).sum();
            if norm == 0.0 {
                break;
            }
            let step = 0.5 * ((bound as f64 / SCALE as f64) - self.target as f64).max(1.0) / norm;
            mu.iter_mut().zip(&g).for_each(|(u, gk)| *u = (*u + step * gk).max(0.0));
        }
        // Keep only the nogoods with a multiplier: the others add nothing, and every node scans the kept ones.
        let (nogoods, mu): (Vec<Vec<u32>>, Vec<i64>) = ks.into_iter().zip(best.1).filter(|(_, u)| *u > 0).unzip();
        self.lag = Some(Lag { m, scale: SCALE, nogoods, mu });
        Some(best.0 as f64 / SCALE as f64)
    }

    /// The matching bound of the whole board (the Franklín–Moniot bound).
    pub fn root_bound(&mut self) -> u64 {
        let roles = self.root_roles();
        self.bound.solve(self.board, &roles).expect("nothing is forced").0
    }

    /// The roles at the root: every number free (0 and 1 barred, as they cannot be picked).
    pub fn root_roles(&self) -> Vec<Role> {
        let mut roles = vec![Role::Free; self.board.n as usize + 1];
        roles[0] = Role::Barred;
        roles[1] = Role::Barred;
        roles
    }

    /// Is there a playable set of picks summing to more than the target?
    pub fn run(&mut self) -> Verdict {
        let roles = self.root_roles();
        self.run_units(vec![roles])
    }

    /// Nogood propagation: false if some nogood is entirely forced; bars the last free member of a nogood whose
    /// other members are all forced.
    fn propagate(&self, roles: &mut [Role]) -> bool {
        loop {
            let mut changed = false;
            for nogood in &self.nogoods {
                let mut free = NONE;
                let mut free_count = 0;
                let mut barred = false;
                for &c in nogood {
                    match roles[c as usize] {
                        Role::Barred => {
                            barred = true;
                            break;
                        }
                        Role::Free => {
                            free = c;
                            free_count += 1;
                        }
                        Role::Pick => {}
                    }
                }
                if barred {
                    continue;
                }
                match free_count {
                    0 => return false,
                    1 => {
                        roles[free as usize] = Role::Barred;
                        changed = true;
                    }
                    _ => {}
                }
            }
            if !self.segments(roles, &mut changed) {
                return false;
            }
            if !changed {
                return true;
            }
        }
    }

    /// The conflict-packing bound.  Every set beating the target leaves out a member of each conflict all of
    /// whose members the matching picks; leaving out c costs the matching at least drop(c) = value −
    /// [`bound_barring`](MatchingBound::bound_barring)(c).  Conflicts whose free members and their payments are
    /// pairwise apart (no shared or adjacent vertices) can have their duals lowered together, so the bound with
    /// one member of each left out is at most value − Σ min drop.  True if that cuts the node.
    fn packing_cut(&self, roles: &[Role], picks: &[bool], value: u64) -> bool {
        let gap = value as i64 - self.target as i64;
        let mut costed: Vec<(i64, usize)> = Vec::new();
        for (i, k) in self.nogoods.iter().enumerate() {
            if !k.iter().all(|&c| picks[c as usize]) {
                continue;
            }
            let cost = k
                .iter()
                .filter(|&&c| roles[c as usize] == Role::Free)
                .map(|&c| value as i64 - self.bound.bound_barring(roles, c))
                .min();
            if let Some(cost) = cost {
                costed.push((cost, i));
            }
        }
        if costed.len() < 2 {
            return false;
        }
        costed.sort_unstable_by(|a, b| b.cmp(a));
        let mut blocked = vec![false; self.board.n as usize + 1];
        let mut total = 0i64;
        for (cost, i) in costed {
            let mut vertices: Vec<u32> = Vec::new();
            for &c in self.nogoods[i].iter().filter(|&&c| roles[c as usize] == Role::Free) {
                vertices.push(c);
                let m = self.bound.mate_of(c);
                if m != NONE {
                    vertices.push(m);
                }
            }
            if vertices.iter().any(|&v| blocked[v as usize]) {
                continue;
            }
            for &v in &vertices {
                blocked[v as usize] = true;
                for &u in self.bound.neighbours(v) {
                    blocked[u as usize] = true;
                }
            }
            total += cost;
            if total >= gap {
                return true;
            }
        }
        false
    }

    /// A node with these roles; `base` is the parent's roles and optimal matchings, to re-optimize from (None:
    /// solve from scratch).
    fn node(&mut self, mut roles: Vec<Role>, base: Option<Base>) -> Verdict {
        self.nodes += 1;
        if self.nodes > self.budget {
            return Verdict::Budget;
        }
        if !self.propagate(&mut roles) {
            return Verdict::Proven;
        }
        let forced: Vec<bool> = roles.iter().map(|&r| r == Role::Pick).collect();
        if !self.board.playable(&forced) {
            return Verdict::Proven;
        }
        let board = self.board;
        let mut solved = match &base {
            Some(b) => self.bound.update(board, b.plain, b.roles, &roles),
            None => self.bound.solve(board, &roles),
        };
        let mut lag_solved =
            self.lag.as_mut().map(|l| l.solve(board, base.as_ref().and_then(|b| b.lag.map(|s| (b.roles, s))), &roles));
        let n = self.board.n;
        // (value, picks): the matching's bound and picks; (best, source): the set the node branches on, the parity
        // bound's heaviest set when that bound is on (else the matching's picks), and its weight.  The parity solves
        // start from the last certificate: the parent's, then the node's own.
        let mut warm: Option<Warm> = None;
        let parent_warm = base.as_ref().and_then(|b| b.parity).filter(|_| self.parity_warm);
        let (value, picks, best, source) = loop {
            let Some((value, picks)) = solved else { return Verdict::Proven };
            if value <= self.target {
                self.stats[4] += 1;
                return Verdict::Proven;
            }
            // Forced picks: barring one alone brings a bound down to the target.
            let mut newly: Vec<u32> = (2..=n)
                .filter(|&c| picks[c as usize] && roles[c as usize] == Role::Free)
                .filter(|&c| self.bound.bound_barring(&roles, c) <= self.target as i64)
                .collect();
            if let (Some(l), Some((lagged, constant))) = (&self.lag, &lag_solved) {
                let Some((lag_value, lag_picks)) = lagged else { return Verdict::Proven };
                let lag_target = self.target as i64 * l.scale - constant;
                if *lag_value as i64 <= lag_target {
                    self.stats[5] += 1;
                    return Verdict::Proven;
                }
                let more: Vec<u32> = (2..=n)
                    .filter(|&c| {
                        lag_picks[c as usize]
                            && roles[c as usize] == Role::Free
                            && !newly.contains(&c)
                            && l.m.bound_barring(&roles, c) <= lag_target
                    })
                    .collect();
                newly.extend(more);
            }
            // The parity bound, once the matching forces nothing more: it cuts, bars the numbers dependent on the
            // forced ones, and forces the members whose barring alone brings it down to the target.
            let mut barred: Vec<u32> = Vec::new();
            let mut parity_set = None;
            if newly.is_empty() {
                if let Some(meet) = self.parity.as_mut() {
                    match meet.solve_from(&roles, warm.as_ref().filter(|_| self.parity_warm).or(parent_warm)) {
                        Outcome::Infeasible => {
                            self.stats[6] += 1;
                            return Verdict::Proven;
                        }
                        Outcome::Uncertified => self.stats[7] += 1,
                        Outcome::Bound(b) => {
                            if b.value <= self.target as i64 {
                                self.stats[6] += 1;
                                return Verdict::Proven;
                            }
                            barred = b.loops;
                            newly.extend(b.without.iter().filter(|&&(_, v)| v <= self.target as i64).map(|&(x, _)| x));
                            if newly.is_empty() && barred.is_empty() {
                                let mut set = vec![false; n as usize + 1];
                                b.set.iter().for_each(|&c| set[c as usize] = true);
                                parity_set = Some((b.value as u64, set));
                            }
                            warm = Some(b.warm);
                        }
                    }
                }
            }
            if newly.is_empty() && barred.is_empty() {
                let (best, source) = parity_set.unwrap_or_else(|| (value, picks.clone()));
                break (value, picks, best, source);
            }
            let (old, snapshot) = (roles.clone(), self.bound.snapshot());
            let lag_snapshot = self.lag.as_ref().map(|l| l.m.snapshot());
            self.stats[2] += newly.len() as u64;
            newly.iter().for_each(|&c| roles[c as usize] = Role::Pick);
            barred.iter().for_each(|&c| roles[c as usize] = Role::Barred);
            if !self.propagate(&mut roles) {
                return Verdict::Proven;
            }
            let forced: Vec<bool> = roles.iter().map(|&r| r == Role::Pick).collect();
            if !self.board.playable(&forced) {
                return Verdict::Proven;
            }
            solved = self.bound.update(board, &snapshot, &old, &roles);
            if let (Some(l), Some(s)) = (self.lag.as_mut(), lag_snapshot.as_ref()) {
                lag_solved = Some(l.solve(board, Some((&old, s)), &roles));
            }
        };
        // Branch on the violated conflict with the fewest free members (measured: better than the one dearest to
        // break, which doubles the tree at n = 600).  A known one also shows the picks unplayable without a peel.
        // (Branching on the Lagrangian matching's picks instead was measured: 13% more nodes at n = 793.)
        //
        // The children are given by a list of decisions (number, role): child j gives the numbers of the first j
        // their roles and the j-th the other role (forced and barred swapped).  For a conflict every decision is
        // "picked", so child j forces the conflict's first j free members and bars the next.
        let source = &source;
        let free_of = |k: &[u32]| -> Vec<(u32, Role)> {
            k.iter().filter(|&&c| roles[c as usize] == Role::Free).map(|&c| (c, Role::Pick)).collect()
        };
        let known = self
            .nogoods
            .iter()
            .filter(|k| k.iter().all(|&c| source[c as usize]))
            .min_by_key(|k| k.iter().filter(|&&c| roles[c as usize] == Role::Free).count());
        let decisions = match known {
            Some(k) => {
                self.stats[1] += 1;
                free_of(k)
            }
            None if self.board.playable(source) => {
                let set: Vec<u32> = (2..=n).filter(|&c| source[c as usize]).collect();
                if self.optimizing {
                    // The node's heaviest completion (its weight is the bound): the target rises to it, which
                    // closes the node.
                    self.target = best;
                    self.found.push(set);
                    return Verdict::Proven;
                }
                self.found.push(set.clone());
                if self.found.len() >= self.limit {
                    return Verdict::Beaten(set);
                }
                // Enumerating: every other set of this node disagrees with the picks on a free number; the
                // numbers outside the picks first, largest first, then the picks, largest first.
                let free = || (2..=n).rev().filter(|&c| roles[c as usize] == Role::Free);
                free()
                    .filter(|&c| !source[c as usize])
                    .map(|c| (c, Role::Barred))
                    .chain(free().filter(|&c| source[c as usize]).map(|c| (c, Role::Pick)))
                    .collect()
            }
            None => {
                self.stats[0] += 1;
                let k = self.board.minimal_unplayable(source);
                let decisions = free_of(&k);
                if self.nogoods.len() < self.max_nogoods {
                    self.nogoods.push(k);
                }
                decisions
            }
        };
        if decisions.is_empty() {
            return Verdict::Proven;
        }
        if self.packing_cut(&roles, &picks, value) {
            self.stats[4] += 1;
            return Verdict::Proven;
        }
        self.stats[3] += decisions.len() as u64;
        let (base_roles, snapshot) = (roles.clone(), self.bound.snapshot());
        let lag_snapshot = self.lag.as_ref().map(|l| l.m.snapshot());
        let child = |j: usize, roles: &[Role]| {
            let mut child = roles.to_vec();
            decisions[..j].iter().for_each(|&(d, role)| child[d as usize] = role);
            let (d, role) = decisions[j];
            child[d as usize] = if role == Role::Pick { Role::Barred } else { Role::Pick };
            child
        };
        let branch = &decisions;
        for j in 0..branch.len() {
            if self.pause_due() {
                // Leave this child and the rest as open units.
                (j..branch.len()).for_each(|k| self.pending.push(child(k, &base_roles)));
                return Verdict::Paused;
            }
            let base = Base { roles: &base_roles, plain: &snapshot, lag: lag_snapshot.as_ref(), parity: warm.as_ref() };
            match self.node(child(j, &base_roles), Some(base)) {
                Verdict::Proven => {}
                Verdict::Paused => {
                    (j + 1..branch.len()).for_each(|k| self.pending.push(child(k, &base_roles)));
                    return Verdict::Paused;
                }
                other => return other,
            }
        }
        Verdict::Proven
    }

    /// Whether the pause deadline has passed (the clock is read every 256 nodes, the first time 256 nodes into
    /// a run, so that every run makes progress).
    fn pause_due(&mut self) -> bool {
        match self.pause_at {
            Some(at) if self.nodes >= self.next_check => {
                self.next_check = self.nodes + 256;
                self.paused = self.paused || std::time::Instant::now() >= at;
                self.paused
            }
            _ => self.paused,
        }
    }

    /// Runs the open units (each the roles of an unexplored subtree; the last is explored first), from a
    /// checkpoint.  On a pause, `pending` holds every unit still open (the ones left of `units` included), in
    /// the same order: depth first, so the deepest open subtree, the one the search was in, comes back first.
    pub fn run_units(&mut self, mut units: Vec<Vec<Role>>) -> Verdict {
        self.next_check = self.nodes + 256;
        while let Some(unit) = units.pop() {
            if self.pause_due() {
                units.push(unit);
                self.pending = units;
                return Verdict::Paused;
            }
            match self.node(unit, None) {
                Verdict::Proven => {}
                Verdict::Paused => {
                    // The pause pushed each level's open children deepest level first, in sibling order; the
                    // deepest must come back first, and within a level the first sibling.
                    let mut fresh = std::mem::take(&mut self.pending);
                    fresh.reverse();
                    units.extend(fresh);
                    self.pending = units;
                    return Verdict::Paused;
                }
                other => return other,
            }
        }
        Verdict::Proven
    }

    /// Stop at `at`, leaving the open units in `pending` (see [`run_units`](Self::run_units)).
    pub fn pause_at(mut self, at: std::time::Instant) -> Self {
        self.pause_at = Some(at);
        self
    }

    /// The units left open by a pause (to checkpoint).
    pub fn open_units(&self) -> &[Vec<Role>] {
        &self.pending
    }

    /// After a pause: takes the open units, and sets a new pause deadline, to go on in the same process (for
    /// instance after [`enable_lagrangian`](Self::enable_lagrangian)) with [`run_units`](Self::run_units).
    pub fn take_units(&mut self, pause_at: std::time::Instant) -> Vec<Vec<Role>> {
        self.pause_at = Some(pause_at);
        self.paused = false;
        std::mem::take(&mut self.pending)
    }
}

/// The largest prime below `n` (n >= 3).
pub fn prime_below(n: u64, spf: &[u32]) -> u64 {
    (2..n).rev().find(|&p| is_prime(p, spf)).expect("n > 2")
}

/// How a recorded optimum V(n) meets the proven upper bounds from V(n − 1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Relation {
    /// V(n) = n + V(n − 1).
    Chain,
    /// n is prime and V(n) = n + V(n − 1) − p, p the largest prime below n.
    Prime,
    /// Neither.
    None,
}

impl Relation {
    pub fn name(self) -> &'static str {
        match self {
            Relation::Chain => "chain",
            Relation::Prime => "prime",
            Relation::None => "none",
        }
    }
}

/// Which relation V(n) = `score` meets, given V(n − 1) = `prev`.
pub fn relation(n: u64, score: u64, prev: u64, spf: &[u32]) -> Relation {
    if score == n + prev {
        Relation::Chain
    } else if n > 2 && is_prime(n, spf) && score == n + prev - prime_below(n, spf) {
        Relation::Prime
    } else {
        Relation::None
    }
}

/// Every optimal set of a game n >= 2 contains q, the largest prime <= n, and no other prime.  (Primes pay only
/// with 1, so a playable set holds at most one.  q > n/2 divides no other number of the board, so it is no one's
/// maximal factor; for a playable S without q, S minus its prime (if any) plus q is playable — the remaining
/// members list 1 nowhere, so q pays with 1 — and heavier, since every other prime is below q.)  Returns q.
pub fn the_prime_of_every_optimal_set(set: &[u32], n: u32, spf: &[u32]) -> Result<u32, String> {
    let primes: Vec<u32> = set.iter().copied().filter(|&c| is_prime(u64::from(c), spf)).collect();
    let q = (2..=n).rev().find(|&p| is_prime(u64::from(p), spf)).expect("n >= 2");
    if primes == [q] {
        Ok(q)
    } else {
        Err(format!("n={n}: the optimal set's primes are {primes:?}, not [{q}]"))
    }
}

/// The optimal sets of game n from all those of game n − 1, when V(n) meets a relation and V(n − 1) = opt(n − 1):
///
/// * **Chain** (opt(n) = n + opt(n − 1)): a playable S without n is a game of n − 1, worth at most opt(n − 1),
///   so every optimal S holds n, and S − n (playable: a subset; n is no one's factor) is worth opt(n − 1), an
///   optimal set of n − 1.  So the optimal sets of n are the T + n, T optimal for n − 1, that are playable.
/// * **Prime** (n prime, opt(n) = n + opt(n − 1) − p): every optimal S holds n (as for a chain, since n > p),
///   so no other prime; S − n + p is playable (p > (n − 1)/2 is no one's maximal factor on either board, and
///   pays with 1 as n did) and worth opt(n − 1).  Conversely every optimal T of n − 1 holds p (it holds the
///   largest prime <= n − 1, [`the_prime_of_every_optimal_set`]) and T − p + n is playable and worth opt(n).
///   So T ↦ T − p + n is a bijection: n has exactly as many optimal sets as n − 1.
///
/// `board` is game n's; `prev` must be every optimal set of n − 1 (for n = 2, the empty set).
pub fn derive_optimal_sets(board: &Board, relation: Relation, prev: &[Vec<u32>], spf: &[u32]) -> Vec<Vec<u32>> {
    let n = board.n;
    let mut sets = Vec::new();
    for t in prev {
        let set: Vec<u32> = match relation {
            Relation::Chain => t.iter().copied().chain([n]).collect(),
            Relation::Prime => {
                let p = prime_below(u64::from(n), spf) as u32;
                assert!(t.contains(&p), "n={n}: an optimal set of n − 1 without {p}");
                t.iter().copied().filter(|&c| c != p).chain([n]).collect()
            }
            Relation::None => panic!("n={n}: no relation to derive from"),
        };
        let mut pick = vec![false; n as usize + 1];
        set.iter().for_each(|&c| pick[c as usize] = true);
        let playable = board.playable(&pick);
        assert!(playable || relation == Relation::Chain, "n={n}: a prime swap is always playable");
        if playable {
            sets.push(set);
        }
    }
    sets
}

#[cfg(test)]
#[allow(clippy::needless_range_loop)]
mod tests {
    use super::*;
    use crate::primes::smallest_prime_factors;

    /// opt(n) by brute force over pick sets (playability by dismantling), for small n.
    fn brute(board: &Board) -> u64 {
        let n = board.n as usize;
        let mut best = 0;
        for bits in 0u64..(1 << (n - 1)) {
            let mut pick = vec![false; n + 1];
            let mut sum = 0;
            for c in 2..=n {
                if bits >> (c - 2) & 1 == 1 {
                    pick[c] = true;
                    sum += c as u64;
                }
            }
            if sum > best && board.playable(&pick) {
                best = sum;
            }
        }
        best
    }

    /// With the target one below optimal.json's score, the search (all its pruning rules: forced picks, nogoods,
    /// the conflict-packing cut, and optionally the segment cuts, the Lagrangian bound or the parity bound) must
    /// still reach a playable set worth the optimum; at the score itself it must prove that nothing beats it.
    /// Every 7th game to 420.
    #[test]
    fn the_search_finds_the_optimum_just_above_the_target() {
        let text =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../src/main/resources/optimal.json"))
                .expect("optimal.json");
        let scores: Vec<(usize, u64)> = text
            .lines()
            .filter(|l| l.contains("\"n\""))
            .map(|line| {
                let num = |key: &str| -> u64 {
                    let at = line.find(key).unwrap() + key.len();
                    line[at..]
                        .trim_start_matches([':', ' '])
                        .chars()
                        .take_while(char::is_ascii_digit)
                        .collect::<String>()
                        .parse()
                        .unwrap()
                };
                (num("\"n\"") as usize, num("\"score\""))
            })
            .collect();
        let mut opt = vec![0u64; scores.iter().map(|&(n, _)| n).max().expect("optimal.json has games") + 1];
        for (n, score) in scores {
            opt[n] = score;
        }
        let spf = smallest_prime_factors(500);
        for n in (21..=420u32).step_by(7) {
            let board = Board::new(n, &spf);
            let v = opt[n as usize];
            for variant in 0..5 {
                let segments = variant == 1 || variant == 4;
                let prover = |target| {
                    let mut p = Prover::new(&board, target, u64::MAX);
                    if variant >= 3 {
                        p = p.with_parity();
                    }
                    if segments {
                        p = p.with_smaller_games(&opt[..n as usize]);
                    }
                    if variant == 2 {
                        // The Lagrangian bound, fitted to the nogoods of a short first search.
                        let mut first = Prover::new(&board, target, 300);
                        first.run();
                        p.nogoods = first.nogoods;
                        p.enable_lagrangian(30);
                    }
                    p
                };
                match prover(v - 1).run() {
                    Verdict::Beaten(picks) => {
                        let mut pick = vec![false; n as usize + 1];
                        picks.iter().for_each(|&c| pick[c as usize] = true);
                        assert!(board.playable(&pick), "n={n}");
                        assert_eq!(picks.iter().map(|&c| u64::from(c)).sum::<u64>(), v, "n={n}");
                    }
                    other => panic!("n={n} variant={variant}: {other:?} below the optimum"),
                }
                assert_eq!(prover(v).run(), Verdict::Proven, "n={n} variant={variant}");
            }
        }
    }

    /// A search paused every few hundred nodes and resumed from its open units and nogoods reaches the verdict of
    /// an uninterrupted one, on both sides of the optimum (opt(300) = 28691, opt(432) = 59259 and, with the parity
    /// bound, opt(600) = 112623; optimal.json).
    #[test]
    fn paused_searches_resume_to_the_same_verdict() {
        let spf = smallest_prime_factors(600);
        for (n, opt, parity) in [(300u32, 28691u64, false), (432, 59259, false), (600, 112623, true)] {
            let board = Board::new(n, &spf);
            let fresh = |target| {
                let p = Prover::new(&board, target, u64::MAX);
                if parity {
                    p.with_parity()
                } else {
                    p
                }
            };
            for target in [opt, opt - 1] {
                let straight = fresh(target).run();
                let (mut units, mut nogoods, mut pauses) = (None::<Vec<Vec<Role>>>, Vec::new(), 0);
                let resumed = loop {
                    let mut p = fresh(target).pause_at(std::time::Instant::now());
                    p.nogoods = std::mem::take(&mut nogoods);
                    let verdict = match units.take() {
                        Some(u) => p.run_units(u),
                        None => p.run(),
                    };
                    if verdict != Verdict::Paused {
                        break verdict;
                    }
                    pauses += 1;
                    units = Some(p.open_units().to_vec());
                    nogoods = p.nogoods;
                };
                assert_eq!(matches!(resumed, Verdict::Beaten(_)), matches!(straight, Verdict::Beaten(_)), "n={n}");
                assert_eq!(resumed == Verdict::Proven, target == opt, "n={n} target={target}");
                assert!(target != opt || n < 400 || pauses > 1, "n={n}: the test should pause");
            }
        }
    }

    /// The known optima of 1..=20 (optimal.json), by brute force and by the prover on both sides of the value.
    #[test]
    fn small_games() {
        let spf = smallest_prime_factors(100);
        let known = [0u64, 0, 2, 3, 7, 9, 15, 17, 21, 30, 40, 44, 50, 52, 66, 81, 89, 93, 111, 113, 124];
        for n in 2..=20u32 {
            let board = Board::new(n, &spf);
            assert_eq!(brute(&board), known[n as usize], "n={n}");
            // Optimizing from nothing reaches the optimum, with and without the segment cuts and the parity bound.
            for (segments, parity) in [(false, false), (true, false), (false, true), (true, true)] {
                let mut p = Prover::new(&board, 0, u64::MAX).optimize();
                if parity {
                    p = p.with_parity();
                }
                if segments {
                    p = p.with_smaller_games(&known);
                }
                assert_eq!(p.run(), Verdict::Proven);
                let best = p.found.last().expect("a game");
                assert_eq!(best.iter().map(|&c| u64::from(c)).sum::<u64>(), known[n as usize], "n={n}");
            }
            for (segments, parity) in [(false, false), (true, false), (false, true), (true, true)] {
                let prover = |target| {
                    let mut p = Prover::new(&board, target, u64::MAX);
                    if parity {
                        p = p.with_parity();
                    }
                    if segments {
                        p.with_smaller_games(&known)
                    } else {
                        p
                    }
                };
                assert_eq!(prover(known[n as usize]).run(), Verdict::Proven, "n={n}");
                match prover(known[n as usize] - 1).run() {
                    Verdict::Beaten(picks) => {
                        let sum: u64 = picks.iter().map(|&c| u64::from(c)).sum();
                        assert!(sum >= known[n as usize], "n={n}");
                    }
                    other => panic!("n={n}: the optimum beats opt − 1, got {other:?}"),
                }
            }
        }
    }

    /// With unit weights the search decides the most moves of a game: against brute force for n <= 20, on both
    /// sides of the maximum, with and without the segment cuts (given the maxima of the smaller games).
    #[test]
    fn unit_weights_find_the_most_moves() {
        let spf = smallest_prime_factors(100);
        let mut most = vec![0u64; 2];
        for n in 2..=20u32 {
            let board = Board::new(n, &spf);
            let mut best = 0u64;
            for bits in 0u64..(1 << (n - 1)) {
                let count = u64::from(bits.count_ones());
                if count > best {
                    let mut pick = vec![false; n as usize + 1];
                    (2..=n as usize).filter(|&c| bits >> (c - 2) & 1 == 1).for_each(|c| pick[c] = true);
                    if board.playable(&pick) {
                        best = count;
                    }
                }
            }
            most.push(best);
            for (segments, parity) in [(false, false), (true, false), (false, true), (true, true)] {
                let prover = |target| {
                    let mut p = Prover::new(&board, target, u64::MAX).weighted(vec![1; n as usize + 1]);
                    if parity {
                        p = p.with_parity();
                    }
                    if segments {
                        p.with_smaller_games(&most[..n as usize])
                    } else {
                        p
                    }
                };
                assert_eq!(prover(best).run(), Verdict::Proven, "n={n}");
                match prover(best - 1).run() {
                    Verdict::Beaten(picks) => {
                        let mut pick = vec![false; n as usize + 1];
                        picks.iter().for_each(|&c| pick[c as usize] = true);
                        assert!(board.playable(&pick) && picks.len() as u64 == best, "n={n}: {picks:?}");
                    }
                    other => panic!("n={n}: the longest game beats {}, got {other:?}", best - 1),
                }
            }
        }
    }

    /// Enumeration lists exactly the playable sets beating the target, each once, against brute force: at
    /// opt − 1 (the optimal sets) and well below (where sets containing a found one also beat the target).
    #[test]
    fn enumeration_lists_every_set_beating_the_target() {
        let spf = smallest_prime_factors(100);
        for n in 2..=19u32 {
            let board = Board::new(n, &spf);
            let opt = brute(&board);
            for below in [1u64, 4, 12] {
                let Some(target) = opt.checked_sub(below) else { continue };
                let mut want: Vec<Vec<u32>> = Vec::new();
                for bits in 0u64..(1 << (n - 1)) {
                    let set: Vec<u32> = (2..=n).filter(|&c| bits >> (c - 2) & 1 == 1).collect();
                    if set.iter().map(|&c| u64::from(c)).sum::<u64>() > target {
                        let mut pick = vec![false; n as usize + 1];
                        set.iter().for_each(|&c| pick[c as usize] = true);
                        if board.playable(&pick) {
                            want.push(set);
                        }
                    }
                }
                want.sort();
                for parity in [false, true] {
                    let mut prover = Prover::new(&board, target, u64::MAX).enumerate(usize::MAX);
                    if parity {
                        prover = prover.with_parity();
                    }
                    assert_eq!(prover.run(), Verdict::Proven, "n={n}");
                    let mut got = prover.found.clone();
                    got.sort();
                    assert_eq!(got, want, "n={n} target={target} parity={parity}");
                }
                // With a limit the search stops at the limit-th set.
                if want.len() >= 2 {
                    let mut limited = Prover::new(&board, target, u64::MAX).enumerate(2);
                    assert!(matches!(limited.run(), Verdict::Beaten(_)), "n={n}");
                    assert_eq!(limited.found.len(), 2);
                }
            }
        }
    }

    /// Where V(n) meets the chain or prime relation, the optimal sets derived from those of n − 1 are the ones
    /// enumerated directly; and every optimal set holds the largest prime <= n and no other.  Every game to 260
    /// (optimal.json's values; the optimal set is unique in all of them but 2..260 still exercises both rules).
    #[test]
    fn derived_optimal_sets_are_the_enumerated_ones() {
        let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../src/main/resources/optimal.json"));
        let known = crate::jsonl::recorded_games(&[path]);
        let spf = smallest_prime_factors(300);
        let all = |n: u64| -> Vec<Vec<u32>> {
            if n < 2 {
                return vec![Vec::new()];
            }
            let board = Board::new(n as u32, &spf);
            let mut prover = Prover::new(&board, known[&n].0 - 1, u64::MAX).enumerate(usize::MAX);
            assert_eq!(prover.run(), Verdict::Proven);
            let mut sets = prover.found;
            sets.iter_mut().for_each(|s| s.sort_unstable());
            sets.sort();
            sets
        };
        let (mut derived, mut by_prime) = (0, 0);
        let mut prev = all(1);
        for n in 2..=260u64 {
            let sets = all(n);
            for s in &sets {
                the_prime_of_every_optimal_set(s, n as u32, &spf).unwrap();
            }
            let rel = relation(n, known[&n].0, known.get(&(n - 1)).map_or(0, |g| g.0), &spf);
            if rel != Relation::None {
                let board = Board::new(n as u32, &spf);
                let mut got = derive_optimal_sets(&board, rel, &prev, &spf);
                got.iter_mut().for_each(|s| s.sort_unstable());
                got.sort();
                assert_eq!(got, sets, "n={n} {rel:?}");
                derived += 1;
                by_prime += usize::from(rel == Relation::Prime);
            }
            prev = sets;
        }
        assert!(derived > 100 && by_prime > 40, "{derived} {by_prime}");
    }

    /// The first game with two optimal sets: 424 has one, 425 has two (optimal.json's, and the one with
    /// {110, 132, 168} exchanged for {98, 150, 162}), with the same upper part.
    #[test]
    fn game_425_has_two_optimal_sets() {
        let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../src/main/resources/optimal.json"));
        let known = crate::jsonl::recorded_games(&[path]);
        let spf = smallest_prime_factors(500);
        let sets = |n: u64| {
            let board = Board::new(n as u32, &spf);
            let mut prover = Prover::new(&board, known[&n].0 - 1, u64::MAX).enumerate(usize::MAX);
            assert_eq!(prover.run(), Verdict::Proven);
            prover.found
        };
        assert_eq!(sets(424).len(), 1);
        let found = sets(425);
        assert_eq!(found.len(), 2);
        let recorded: Vec<u32> = {
            let mut m = known[&425].1.clone();
            m.sort_unstable();
            m
        };
        let other: Vec<u32> = found.into_iter().find(|s| *s != recorded).expect("a second set");
        let gone: Vec<u32> = recorded.iter().copied().filter(|c| !other.contains(c)).collect();
        let new: Vec<u32> = other.iter().copied().filter(|c| !recorded.contains(c)).collect();
        assert_eq!((gone, new), (vec![110, 132, 168], vec![98, 150, 162]));
    }

    /// Re-optimizing from a snapshot gives the same value as solving from scratch, along random chains of role
    /// changes on boards up to 300.
    #[test]
    fn update_agrees_with_solving_from_scratch() {
        let spf = smallest_prime_factors(400);
        let mut state = 0x2545_F491_4F6C_DD1Du64;
        let mut next = |below: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % below
        };
        for _ in 0..60 {
            let n = 10 + next(290) as u32;
            let board = Board::new(n, &spf);
            let mut roles = vec![Role::Free; n as usize + 1];
            roles[0] = Role::Barred;
            roles[1] = Role::Barred;
            let mut inc = MatchingBound::new(&board);
            let mut got = inc.solve(&board, &roles);
            for _ in 0..12 {
                let Some((_, picks)) = got.clone() else { break };
                let snapshot = inc.snapshot();
                let old = roles.clone();
                for _ in 0..1 + next(3) {
                    let c = 2 + next(u64::from(n) - 1) as usize;
                    if roles[c] == Role::Free {
                        // Mostly bar or force picks of the matching, as the search does.
                        roles[c] = if picks[c] && next(2) == 0 { Role::Barred } else { Role::Pick };
                    }
                }
                got = inc.update(&board, &snapshot, &old, &roles);
                let fresh = MatchingBound::new(&board).solve(&board, &roles);
                assert_eq!(got.as_ref().map(|g| g.0), fresh.as_ref().map(|f| f.0), "n={n}");
            }
        }
    }

    /// The matching bound against brute force over matchings on tiny boards, with random roles.
    #[test]
    fn matching_bound_against_brute_force() {
        let spf = smallest_prime_factors(100);
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = |below: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % below
        };
        for _ in 0..300 {
            let n = 4 + next(11) as u32;
            let board = Board::new(n, &spf);
            let mut roles = vec![Role::Free; n as usize + 1];
            roles[0] = Role::Barred;
            roles[1] = Role::Barred;
            for c in 2..=n as usize {
                roles[c] = match next(6) {
                    0 => Role::Pick,
                    1 => Role::Barred,
                    _ => Role::Free,
                };
            }
            // Brute force: every set of edges forming a matching.
            let edges: Vec<(u32, u32)> =
                (2..=n).flat_map(|c| board.mf[c as usize].iter().map(move |&f| (c, f))).collect();
            let mut best: Option<u64> = None;
            for bits in 0u64..(1 << edges.len()) {
                let mut used = vec![false; n as usize + 1];
                let mut ok = true;
                let mut value = 0;
                let mut picked = vec![false; n as usize + 1];
                for (i, &(c, f)) in edges.iter().enumerate() {
                    if bits >> i & 1 == 1 {
                        if used[c as usize] || used[f as usize] {
                            ok = false;
                            break;
                        }
                        used[c as usize] = true;
                        used[f as usize] = true;
                        if roles[c as usize] == Role::Barred || roles[f as usize] == Role::Pick {
                            ok = false;
                            break;
                        }
                        picked[c as usize] = true;
                        value += u64::from(c);
                    }
                }
                if ok && (2..=n as usize).all(|c| roles[c] != Role::Pick || picked[c]) {
                    best = Some(best.map_or(value, |b| b.max(value)));
                }
            }
            let mut bound = MatchingBound::new(&board);
            let got = bound.solve(&board, &roles);
            assert_eq!(got.as_ref().map(|(v, _)| *v), best, "n={n} roles={roles:?}");
            // The dual bound with a pick barred is never below the true bound with it barred.
            if let Some((_, picks)) = got {
                for c in (2..=n).filter(|&c| picks[c as usize] && roles[c as usize] == Role::Free) {
                    let estimate = bound.bound_barring(&roles, c);
                    let mut barred = roles.clone();
                    barred[c as usize] = Role::Barred;
                    let truth = MatchingBound::new(&board).solve(&board, &barred).map_or(i64::MIN, |(v, _)| v as i64);
                    assert!(estimate >= truth, "n={n} c={c}: {estimate} < {truth}");
                }
            }
        }
    }
}
