//! Weighted matroid intersection for two linear matroids over GF(2) that are direct sums of blocks of rank at most
//! `64·R`: the solver behind [`super::Gf2Bound`].  The matroids, and why every promotable set is independent in
//! both, are in the parent module's docs; this module knows nothing of frames or promotions.
//!
//! # The algorithm
//!
//! Shortest augmenting paths (Lawler; Schrijver, *Combinatorial Optimization*, Theorem 41.7).  A common independent
//! set `I` is *extreme* when it is the heaviest of its size.  Its *exchange graph* has an arc `x → y` (`x ∈ I`,
//! `y ∉ I`) when `I − x + y` is independent in `A`, and `y → x` when it is independent in `B`; the vertex lengths are
//! `w(x)` on `I` and `−w(y)` off it.  A path from an element `y` with `I + y` independent in `A` to one with `I + y`
//! independent in `B` that is shortest, and has the fewest arcs among the shortest, gives `I` flipped along it: the
//! heaviest common independent set of size `|I| + 1`.  The heaviest weights by size are concave, so the heaviest of
//! all sizes is reached when the shortest path's length is no longer negative.
//!
//! In a linear matroid `I − x + y` is independent when `I + y` is, or when `x` is in the combination of `I`'s
//! vectors that makes `y`'s (its *circuit*).  The span of `I` in each block is kept as rows by pivot (the row's
//! lowest set bit), each with the mask of the slots (members of `I` in the block) whose vectors sum to it, so one
//! reduction of `y`'s vector gives either a non-zero residue (`I + y` independent) or its circuit.  An element that
//! is independent of `I` in a matroid has arcs to or from every member of `I`; those arcs are relaxed through one
//! minimum per Bellman–Ford round instead of one at a time.
//!
//! # Where an augmentation may start
//!
//! The augmentations may start from any set that is extreme and heavier than every smaller common independent set
//! (*dominant*): from there each one gives the heaviest set one larger, and the gains decrease, so the first gain
//! that is not positive marks the heaviest set of all sizes.  [`Meet::solve`] starts from the larger of a given
//! dominant set and the heaviest *uncontended* elements: the longest run of the heaviest elements that is common
//! independent, which is the heaviest set of its size, and dominant because weights are positive.  Every set an
//! augmentation reaches is dominant (the gains so far were positive), and [`Meet::solve`] reports each, so that a
//! caller can start a related problem from one of them (see [`super`], "Warm starts").
//!
//! # The weight split
//!
//! For the heaviest set `I*` of all sizes (if no size limit stopped the augmentations), [`Meet::split_bounds`] finds
//! `w1`, `w2` with `w1 + w2 = w` such that `I*` is a heaviest independent set of `A` for `w1` and of `B` for `w2`
//! (Frank's weight-splitting theorem; any sizes).  For any common independent `J`, `w(J) = w1(J) + w2(J)`, so the
//! heaviest independent set of `A` for `w1` plus that of `B` for `w2` bounds every common independent set (weak
//! duality), and with an element left out each of those is one exchange away from `I*`: `I* − x` plus the heaviest
//! element that can replace `x` (whose circuit holds `x`), if its weight is positive.  So the bound without each
//! member of `I*` takes one pass over the circuits.  The split comes from the conditions under which `I*` is a
//! heaviest independent set of a matroid for a weighting (members of non-negative weight; elements whose addition
//! keeps it independent of non-positive weight; no exchange that gains), which for `w1` on `A` and `w2 = w − w1` on
//! `B` are difference constraints on `w1`, solved by Bellman–Ford.

/// A block vector: bit `i` of word `i / 64` is coordinate `i` of the block's quotient space.
pub type Vector<const R: usize> = [u64; R];

#[inline]
pub(super) fn is_zero<const R: usize>(v: &Vector<R>) -> bool {
    v.iter().all(|&w| w == 0)
}

/// The lowest set bit of `v`.
#[inline]
pub(super) fn lowest<const R: usize>(v: &Vector<R>) -> Option<usize> {
    for (i, &w) in v.iter().enumerate() {
        if w != 0 {
            return Some(64 * i + w.trailing_zeros() as usize);
        }
    }
    None
}

#[inline]
pub(super) fn xor<const R: usize>(a: &mut Vector<R>, b: &Vector<R>) {
    for (x, y) in a.iter_mut().zip(b) {
        *x ^= y;
    }
}

#[inline]
pub(super) fn bit<const R: usize>(v: &Vector<R>, i: usize) -> bool {
    v[i / 64] >> (i % 64) & 1 == 1
}

/// Calls `f` with each set bit of `mask`.
#[inline]
fn each_bit<const R: usize>(mask: &Vector<R>, mut f: impl FnMut(usize)) {
    for (i, &w) in mask.iter().enumerate() {
        let mut w = w;
        while w != 0 {
            f(64 * i + w.trailing_zeros() as usize);
            w &= w - 1;
        }
    }
}

/// The two matroids: for each element, its block in `A` and in `B` and its vectors there.  An element whose vector
/// is zero in either is a loop: no common independent set holds it.
pub struct Problem<'a, const R: usize> {
    pub block: [&'a [u8]; 2],
    pub vector: [&'a [Vector<R>]; 2],
    /// The weight of each element: positive, and not decreasing with the element's index (the heaviest elements come
    /// last).  In the search an element's weight is its number and the elements are indexed by increasing number.
    pub weight: &'a [i64],
}

/// The span of the members of a set in one block of one matroid: the rows by pivot, each with the mask of the
/// slots whose vectors make it, and the element in each slot.
#[derive(Clone)]
struct BlockSpan<const R: usize> {
    row: Vec<(Vector<R>, Vector<R>)>,
    has: Vector<R>,
    elem: Vec<u32>,
}

impl<const R: usize> BlockSpan<R> {
    fn new() -> Self {
        BlockSpan { row: vec![([0; R], [0; R]); 64 * R], has: [0; R], elem: Vec::with_capacity(64 * R) }
    }

    #[inline]
    fn clear(&mut self) {
        self.has = [0; R];
        self.elem.clear();
    }

    /// `v` reduced by the rows: `(residue, slot mask)`.  The residue is zero when `v` is in the span (and the mask
    /// makes it); otherwise its lowest set bit is not a pivot.
    #[inline]
    fn reduce(&self, mut v: Vector<R>) -> (Vector<R>, Vector<R>) {
        let mut c = [0; R];
        while let Some(p) = lowest(&v) {
            if !bit(&self.has, p) {
                break;
            }
            // A row has no bits below its pivot.
            let (rv, rc) = &self.row[p];
            xor(&mut v, rv);
            xor(&mut c, rc);
        }
        (v, c)
    }

    /// Adds the element `e` with the vector `v`; false (and nothing added) if `v` is in the span.
    #[inline]
    fn insert(&mut self, v: Vector<R>, e: u32) -> bool {
        let (r, mut c) = self.reduce(v);
        let Some(p) = lowest(&r) else { return false };
        let slot = self.elem.len();
        self.elem.push(e);
        c[slot / 64] |= 1 << (slot % 64);
        self.row[p] = (r, c);
        self.has[p / 64] |= 1 << (p % 64);
        true
    }
}

/// Lengths in the exchange graph are compared by length, then by the number of arcs.
type Dist = (i64, u32);
const INF: Dist = (i64::MAX, u32::MAX);
const NONE: u32 = u32::MAX;

/// The workspace of the intersection: everything a solve needs, kept between solves so that a solve allocates
/// nothing once the vectors have grown.
pub struct Meet<const R: usize> {
    /// Per matroid, per block: the span of the current set.
    spans: [Vec<BlockSpan<R>>; 2],
    /// Element `e` is in the current set.
    inside: Vec<bool>,
    members: Vec<u32>,
    outside: Vec<u32>,
    /// Per matroid, for each element outside: independent of the set there (`free`), else its circuit
    /// `circ[at[e].0..at[e].1]`.
    free: [Vec<bool>; 2],
    circ: [Vec<u32>; 2],
    at: [Vec<(u32, u32)>; 2],
    dist: Vec<Dist>,
    pred: Vec<u32>,
    /// Whether the exchange data describe the final set (the augmentations ended because no path gains, not
    /// because of the size limit), which the split needs.
    pub settled: bool,
    /// The heaviest set found and its weight.
    pub value: i64,
    /// Counters: augmentations made, Bellman–Ford rounds run.
    pub augmentations: u64,
    pub rounds: u64,
    /// The split's scratch: its potentials (one per element and one more), and the replacements' weights.
    pot: Vec<i64>,
    repl: [Vec<i64>; 2],
}

impl<const R: usize> Meet<R> {
    /// A workspace for problems of `n` elements and `blocks` blocks in each matroid.
    pub fn new(n: usize, blocks: [usize; 2]) -> Self {
        Meet {
            spans: blocks.map(|b| vec![BlockSpan::new(); b]),
            inside: vec![false; n],
            members: Vec::with_capacity(n),
            outside: Vec::with_capacity(n),
            free: [vec![false; n], vec![false; n]],
            circ: [Vec::new(), Vec::new()],
            at: [vec![(0, 0); n], vec![(0, 0); n]],
            dist: vec![INF; n],
            pred: vec![NONE; n],
            settled: false,
            value: 0,
            augmentations: 0,
            rounds: 0,
            pot: vec![0; n + 1],
            repl: [vec![0; n], vec![0; n]],
        }
    }

    /// Whether `e` is in the heaviest set found by the last [`solve`](Self::solve).
    #[inline]
    pub fn contains(&self, e: u32) -> bool {
        self.inside[e as usize]
    }

    /// The heaviest common independent set of at most `cap` of the elements `live` (ascending), its weight in
    /// `self.value` and its members marked ([`contains`](Self::contains)).  The augmentations start from the larger
    /// of `start` (which must be a dominant common independent set of live elements, see the module docs; it may
    /// be empty) and the heaviest uncontended elements.  `report` is called with the start set and then with each
    /// set an augmentation reaches (all dominant), as the members of the set in ascending order.
    pub fn solve(
        &mut self,
        p: &Problem<'_, R>,
        live: &[u32],
        start: &[u32],
        cap: usize,
        mut report: impl FnMut(&[u32]),
    ) {
        let w = p.weight;
        for &e in live {
            self.inside[e as usize] = false;
        }
        // The heaviest uncontended elements: the live elements by decreasing weight (index), while common
        // independent.
        self.clear_spans();
        let mut prefix = 0;
        for &e in live.iter().rev() {
            if prefix == cap || !self.insert_both(p, e) {
                break;
            }
            prefix += 1;
        }
        if start.len() > prefix {
            for &e in live.iter().rev().take(prefix) {
                self.inside[e as usize] = false;
            }
            for &e in start {
                self.inside[e as usize] = true;
            }
        } else {
            for &e in live.iter().rev().take(prefix) {
                self.inside[e as usize] = true;
            }
        }
        self.members.clear();
        self.members.extend(live.iter().copied().filter(|&e| self.inside[e as usize]));
        self.value = self.members.iter().map(|&e| w[e as usize]).sum();
        report(&self.members);
        self.settled = false;
        loop {
            self.members.clear();
            self.outside.clear();
            for &e in live {
                if self.inside[e as usize] {
                    self.members.push(e);
                } else {
                    self.outside.push(e);
                }
            }
            if self.members.len() >= cap {
                return;
            }
            self.exchange_data(p);
            let Some(sink) = self.shortest_path(w) else {
                self.settled = true;
                return;
            };
            let gain = -self.dist[sink as usize].0;
            if gain <= 0 {
                self.settled = true;
                return;
            }
            let mut v = sink;
            loop {
                let i = v as usize;
                self.inside[i] = !self.inside[i];
                if self.pred[i] == NONE {
                    break;
                }
                v = self.pred[i];
            }
            self.value += gain;
            self.augmentations += 1;
            self.members.clear();
            self.members.extend(live.iter().copied().filter(|&e| self.inside[e as usize]));
            report(&self.members);
        }
    }

    fn clear_spans(&mut self) {
        for spans in &mut self.spans {
            for s in spans {
                s.clear();
            }
        }
    }

    /// Adds `e` to the spans of both matroids if it is independent in both; false (nothing added) otherwise.
    #[inline]
    fn insert_both(&mut self, p: &Problem<'_, R>, e: u32) -> bool {
        let i = e as usize;
        let (ba, bb) = (usize::from(p.block[0][i]), usize::from(p.block[1][i]));
        let (va, vb) = (p.vector[0][i], p.vector[1][i]);
        if is_zero(&self.spans[0][ba].reduce(va).0) || is_zero(&self.spans[1][bb].reduce(vb).0) {
            return false;
        }
        self.spans[0][ba].insert(va, e);
        self.spans[1][bb].insert(vb, e);
        true
    }

    /// The spans of the members and, for each element outside, whether it is free in each matroid or its circuit.
    fn exchange_data(&mut self, p: &Problem<'_, R>) {
        self.clear_spans();
        for &e in &self.members {
            let i = e as usize;
            for m in 0..2 {
                let inserted = self.spans[m][usize::from(p.block[m][i])].insert(p.vector[m][i], e);
                debug_assert!(inserted, "a member of a common independent set is dependent");
            }
        }
        for m in 0..2 {
            self.circ[m].clear();
            for &y in &self.outside {
                let i = y as usize;
                let span = &self.spans[m][usize::from(p.block[m][i])];
                let (r, c) = span.reduce(p.vector[m][i]);
                let start = self.circ[m].len() as u32;
                // A loop (zero vector) reduces to zero with an empty circuit: no arcs, never a source or sink.
                let free = !is_zero(&r);
                if !free {
                    let circ = &mut self.circ[m];
                    each_bit(&c, |slot| circ.push(span.elem[slot]));
                }
                self.free[m][i] = free && !is_zero(&p.vector[m][i]);
                self.at[m][i] = (start, self.circ[m].len() as u32);
            }
        }
    }

    /// Bellman–Ford from the sources (elements free in `A`), lexicographic in length and arcs.  Returns the sink
    /// (an element free in `B`) at the least distance, if any is reached.
    fn shortest_path(&mut self, w: &[i64]) -> Option<u32> {
        for &e in self.members.iter().chain(&self.outside) {
            self.dist[e as usize] = INF;
            self.pred[e as usize] = NONE;
        }
        for &y in &self.outside {
            let i = y as usize;
            if self.free[0][i] && self.free[1][i] {
                // A source that is also a sink: the path of one element.  (Still relaxed below: a shorter path
                // to it is a longer path, never a shorter one.)
            }
            if self.free[0][i] {
                self.dist[i] = (-w[i], 0);
            }
        }
        let limit = self.members.len() + self.outside.len() + 2;
        let mut rounds = 0;
        loop {
            rounds += 1;
            assert!(rounds <= limit, "a negative cycle in the exchange graph of an extreme set");
            let mut changed = false;
            // y → x when I − x + y is independent in B: through one minimum for the free y's.
            let mut best_free = (INF, NONE);
            for &y in &self.outside {
                let i = y as usize;
                let d = self.dist[i];
                if d == INF {
                    continue;
                }
                if self.free[1][i] {
                    if d < best_free.0 {
                        best_free = (d, y);
                    }
                } else {
                    let (s, t) = self.at[1][i];
                    for &x in &self.circ[1][s as usize..t as usize] {
                        let x = x as usize;
                        let cand = (d.0 + w[x], d.1 + 1);
                        if cand < self.dist[x] {
                            self.dist[x] = cand;
                            self.pred[x] = y;
                            changed = true;
                        }
                    }
                }
            }
            if best_free.0 != INF {
                let (d, y) = best_free;
                for &x in &self.members {
                    let x = x as usize;
                    let cand = (d.0 + w[x], d.1 + 1);
                    if cand < self.dist[x] {
                        self.dist[x] = cand;
                        self.pred[x] = y;
                        changed = true;
                    }
                }
            }
            // x → y when I − x + y is independent in A: through one minimum for the free y's.
            let mut best_member = (INF, NONE);
            for &x in &self.members {
                let d = self.dist[x as usize];
                if d < best_member.0 {
                    best_member = (d, x);
                }
            }
            for &y in &self.outside {
                let i = y as usize;
                let mut best = self.dist[i];
                let mut from = NONE;
                if self.free[0][i] {
                    if best_member.0 != INF {
                        let cand = (best_member.0 .0 - w[i], best_member.0 .1 + 1);
                        if cand < best {
                            (best, from) = (cand, best_member.1);
                        }
                    }
                } else {
                    let (s, t) = self.at[0][i];
                    for &x in &self.circ[0][s as usize..t as usize] {
                        let d = self.dist[x as usize];
                        if d != INF {
                            let cand = (d.0 - w[i], d.1 + 1);
                            if cand < best {
                                (best, from) = (cand, x);
                            }
                        }
                    }
                }
                if from != NONE {
                    self.dist[i] = best;
                    self.pred[i] = from;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        self.rounds += rounds as u64;
        let mut sink = None;
        let mut best = INF;
        for &y in &self.outside {
            let i = y as usize;
            if self.free[1][i] && self.dist[i] < best {
                (sink, best) = (Some(y), self.dist[i]);
            }
        }
        sink
    }

    /// After a [`solve`](Self::solve) that settled (see [`settled`](Self::settled)): the bound on the heaviest
    /// common independent set without each member `x` of the heaviest set, from a weight split (see the module
    /// docs), passed to `f(x, bound)`.  False (and `f` never called) if the split's constraints have no solution,
    /// which Frank's theorem rules out for a settled solve (a test checks it).
    pub fn split_bounds(&mut self, p: &Problem<'_, R>, live: &[u32], mut f: impl FnMut(u32, i64)) -> bool {
        debug_assert!(self.settled);
        let w = p.weight;
        // Potentials for the constraints w1(v) − w1(u) ≤ c, as arcs u → v of length c; the zero node is `zero`.
        let zero = self.pot.len() - 1;
        for &e in live {
            self.pot[e as usize] = 0;
        }
        self.pot[zero] = 0;
        let limit = live.len() + 3;
        let mut settled = false;
        for _ in 0..limit {
            let mut changed = false;
            let pot = &mut self.pot;
            let mut relax = |u: usize, v: usize, c: i64| {
                if pot[u] + c < pot[v] {
                    pot[v] = pot[u] + c;
                    changed = true;
                }
            };
            for &e in live {
                let i = e as usize;
                if self.inside[i] {
                    relax(i, zero, 0); // w1(x) ≥ 0
                    relax(zero, i, w[i]); // w1(x) ≤ w(x)
                } else {
                    if self.free[0][i] {
                        relax(zero, i, 0); // w1(y) ≤ 0
                    } else {
                        let (s, t) = self.at[0][i];
                        for &x in &self.circ[0][s as usize..t as usize] {
                            relax(x as usize, i, 0); // w1(y) ≤ w1(x)
                        }
                    }
                    if self.free[1][i] {
                        relax(i, zero, -w[i]); // w1(y) ≥ w(y)
                    } else {
                        let (s, t) = self.at[1][i];
                        for &x in &self.circ[1][s as usize..t as usize] {
                            let x = x as usize;
                            relax(i, x, w[x] - w[i]); // w2(y) ≤ w2(x)
                        }
                    }
                }
            }
            if !changed {
                settled = true;
                break;
            }
        }
        if !settled {
            return false;
        }
        // The split: w1 = pot − pot[zero], w2 = w − w1.  Members have 0 ≤ w1, w2 ≤ w.
        let base = self.pot[zero];
        for &e in live {
            self.pot[e as usize] -= base;
        }
        // In each matroid the members of positive weight there, B_m, are a heaviest basis of the elements of
        // positive weight; without a member x of B_m the heaviest independent set is B_m − x plus the heaviest
        // positive element whose circuit with respect to B_m holds x (one exchange), if any.  (Circuits with
        // respect to all of I* would miss the replacements through members of weight 0.)
        for m in 0..2 {
            let weight_in = |i: usize, pot: &[i64]| if m == 0 { pot[i] } else { w[i] - pot[i] };
            for span in &mut self.spans[m] {
                span.clear();
            }
            for &e in live {
                let i = e as usize;
                self.repl[m][i] = 0;
                if self.inside[i] && weight_in(i, &self.pot) > 0 {
                    let inserted = self.spans[m][usize::from(p.block[m][i])].insert(p.vector[m][i], e);
                    debug_assert!(inserted, "a member of a common independent set is dependent");
                }
            }
            for &e in live {
                let i = e as usize;
                let wy = weight_in(i, &self.pot);
                if wy <= 0 || (self.inside[i] && wy > 0) {
                    continue;
                }
                let span = &self.spans[m][usize::from(p.block[m][i])];
                let (r, c) = span.reduce(p.vector[m][i]);
                // A positive element independent of B_m would make B_m + y heavier: the split rules it out.
                debug_assert!(is_zero(&r), "an element of positive weight is free of a heaviest basis");
                let repl = &mut self.repl[m];
                each_bit(&c, |slot| {
                    let x = span.elem[slot] as usize;
                    repl[x] = repl[x].max(wy);
                });
            }
        }
        for &e in live {
            let i = e as usize;
            if !self.inside[i] {
                continue;
            }
            let (w1, w2) = (self.pot[i], w[i] - self.pot[i]);
            let mut bound = self.value;
            if w1 > 0 {
                bound -= w1 - self.repl[0][i];
            }
            if w2 > 0 {
                bound -= w2 - self.repl[1][i];
            }
            f(e, bound);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Rng;

    /// A random problem of `n` elements with blocks of dimension at most `dim` (one word).
    fn random(rng: &mut Rng, n: usize, dim: u64) -> (Vec<u8>, Vec<u8>, Vec<Vector<1>>, Vec<Vector<1>>, Vec<i64>) {
        let blocks = [1 + rng.below(3), 1 + rng.below(3)];
        let dims = [
            (0..blocks[0]).map(|_| 1 + rng.below(dim)).collect::<Vec<_>>(),
            (0..blocks[1]).map(|_| 1 + rng.below(dim)).collect(),
        ];
        let mut block = [Vec::new(), Vec::new()];
        let mut vector = [Vec::new(), Vec::new()];
        for _ in 0..n {
            for m in 0..2 {
                let b = rng.below(blocks[m]) as usize;
                block[m].push(b as u8);
                vector[m].push([rng.below(1 << dims[m][b])]);
            }
        }
        let mut weight: Vec<i64> = (0..n).map(|_| 1 + rng.below(30) as i64).collect();
        weight.sort_unstable();
        let [ba, bb] = block;
        let [va, vb] = vector;
        (ba, bb, va, vb, weight)
    }

    fn independent(block: &[u8], vector: &[Vector<1>], set: &[u32]) -> bool {
        let mut spans = vec![BlockSpan::<1>::new(); 4];
        set.iter().all(|&e| spans[usize::from(block[e as usize])].insert(vector[e as usize], e))
    }

    /// By brute force: the heaviest common independent set of at most `cap` elements, of each subset of `live`.
    fn brute(p: &Problem<'_, 1>, live: &[u32], cap: usize) -> i64 {
        let mut best = 0;
        for mask in 0u32..(1 << live.len()) {
            let set: Vec<u32> = (0..live.len()).filter(|&i| mask >> i & 1 == 1).map(|i| live[i]).collect();
            if set.len() <= cap
                && independent(p.block[0], p.vector[0], &set)
                && independent(p.block[1], p.vector[1], &set)
            {
                best = best.max(set.iter().map(|&e| p.weight[e as usize]).sum());
            }
        }
        best
    }

    /// Against brute force: the heaviest set under every size limit, from the empty start and from every set
    /// a solve reported (dominant), with a member deleted; and the split's bound without each member.
    #[test]
    fn intersection_warm_starts_and_split_against_brute_force() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let (mut warm, mut splits) = (0, 0);
        for trial in 0..3000 {
            let n = 1 + rng.below(11) as usize;
            let (ba, bb, va, vb, weight) = random(&mut rng, n, 5);
            let p = Problem { block: [&ba, &bb], vector: [&va, &vb], weight: &weight };
            let mut meet = Meet::<1>::new(n, [3, 3]);
            let live: Vec<u32> = (0..n as u32).collect();
            for cap in 0..=n {
                let mut reported = Vec::new();
                meet.solve(&p, &live, &[], cap, |set| reported.push(set.to_vec()));
                assert_eq!(meet.value, brute(&p, &live, cap), "trial {trial} cap {cap}");
                let chosen: Vec<u32> = live.iter().copied().filter(|&e| meet.contains(e)).collect();
                assert!(chosen.len() <= cap && independent(&ba, &va, &chosen) && independent(&bb, &vb, &chosen));
                assert_eq!(chosen.iter().map(|&e| weight[e as usize]).sum::<i64>(), meet.value);
                assert_eq!(reported.last(), Some(&chosen), "trial {trial}: the last set reported is the heaviest");
                if cap < n {
                    continue;
                }
                // The split, with no size limit.
                let value = meet.value;
                let mut bounds = Vec::new();
                if meet.settled {
                    assert!(meet.split_bounds(&p, &live, |x, b| bounds.push((x, b))), "trial {trial}: no split");
                    splits += 1;
                }
                for (x, b) in bounds {
                    let rest: Vec<u32> = live.iter().copied().filter(|&e| e != x).collect();
                    assert!(b >= brute(&p, &rest, n), "trial {trial}: the split's bound without {x}");
                    assert!(b <= value, "trial {trial}: the split's bound without {x} above the optimum");
                }
                // Warm starts: a reported set without a deleted element that it does not hold.
                for set in &reported {
                    for d in 0..n as u32 {
                        if set.contains(&d) {
                            continue;
                        }
                        let rest: Vec<u32> = live.iter().copied().filter(|&e| e != d).collect();
                        meet.solve(&p, &rest, set, n, |_| {});
                        assert_eq!(meet.value, brute(&p, &rest, n), "trial {trial}: from {set:?} without {d}");
                        warm += 1;
                    }
                }
            }
        }
        assert!(warm > 10_000 && splits > 2_000, "{warm} warm starts, {splits} splits");
    }
}
