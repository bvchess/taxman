//! Weighted matroid intersection for two linear matroids over GF(2) that are direct sums of blocks, and the two
//! such matroids that every promotion set of a frame-model state is independent in.  Research code for
//! `examples/matroid.rs`; the study and its numbers are in docs/EXPERIMENTS.md, "Matroid intersection bounds
//! (2026-10)".
//!
//! # The frame matroids
//!
//! Take a state of the search (the root, or a node: its frames with what it promoted) and its candidates.  Give
//! each frame `h` the vector space over GF(2) whose coordinates are its factors `F_h`, and each mover `u` paying in
//! frame `h` (a move of `h`, or a candidate of home `h + 1`) the vector `v_h(u)` with a 1 at each of its maximal
//! factors in `F_h` (the factor lists of [`taxman::model::FactorProvider`]).
//!
//! **Claim.**  If the candidates `Q` can be promoted together, then for every frame `h` the vectors
//! `{v_h(m) : m a move of h} ∪ {v_h(x) : x ∈ Q of home h + 1} ∪ {e_y : y ∈ Q of home h}` are linearly
//! independent.
//!
//! *Proof.*  `Q` can be promoted exactly when every frame is winnable with all of `Q` applied at once
//! (`model::promote`): its moves plus `Q`'s members of home `h + 1`, its factors minus `Q`'s of home `h`.  In a
//! winnable frame the peel drops every move, each with a live factor that no move still there has (`model::peel`).
//! In a vanishing sum of the movers' vectors restricted to the live factors `F_h − Q`, the first-dropped mover of
//! the sum has a coordinate that no other term has, so the sum is empty: those restricted vectors are independent.
//! Restricting to `F_h − Q` is the same as adding the unit vectors `e_y` of the factors taken away (a sum of the
//! movers that vanishes off `Q` is a sum of `e_y`'s), which gives the claim.  ∎
//!
//! The moves of `h` are independent by the claim with `Q` empty, so frame `h` makes a matroid on the candidates
//! that touch it: contract the moves, that is, reduce each candidate's vector modulo their span.  Its rank is
//! `|F_h| − |moves of h|`, the frame's freedoms.  A candidate of home `h` touches frames `h` (with `e_x`) and
//! `h − 1` (with `v_{h−1}(x)`), one of even index and one of odd, so the direct sum of the even frames' matroids,
//! `A`, and of the odd frames', `B`, are matroids on the candidates, and every promotable `Q` is independent in
//! both.  The heaviest common independent set of `A` and `B` is an upper bound on the best completion.
//!
//! It is never above the matching bound: independent vectors have a system of distinct representatives (a
//! nonzero maximal minor has a nonzero term), so every frame's movers get distinct live factors, each `y ∈ Q` of
//! home `h` taking its own coordinate, which is the matching of `matchbound` frame by frame.  On movers with two
//! maximal factors (graph edges) linear independence over GF(2) is exactly being a forest, so where a frame's
//! movers have at most two factors each the frame's condition is winnability itself; what it misses are the
//! *odd* cores of movers with three or more factors.
//!
//! # The intersection
//!
//! [`intersect`] is the shortest augmenting path algorithm (Lawler; Schrijver, *Combinatorial Optimization*,
//! Theorem 41.7): from an extreme common independent set `I` (the heaviest of its size), the exchange graph has an
//! arc `x → y` (`x ∈ I`, `y ∉ I`) when `I − x + y` is independent in `A` and `y → x` when it is in `B`; a path from
//! a `y` with `I + y` independent in `A` to one with `I + y` independent in `B` that is shortest for the lengths
//! `w(x)` on `I` and `−w(y)` off it, with the fewest arcs among those, gives the heaviest common independent set of
//! size `|I| + 1`.  The heaviest weights by size are concave, so the heaviest of all sizes is reached when the
//! shortest path's length stops being negative.  In a linear matroid `I − x + y` is independent when `I + y` is, or
//! when `x` is in the combination of `I` that makes `y`; [`Span`] keeps that combination.

/// A linear matroid over GF(2) that is a direct sum of blocks: element `e` lies in block `block[e]` with the
/// vector `vecs[e]` (as many words as that block's `words`).  A zero vector is a loop.
#[derive(Clone, Debug)]
pub struct BlockMatroid {
    pub block: Vec<usize>,
    pub vecs: Vec<Vec<u64>>,
    /// Words per block.
    pub words: Vec<usize>,
}

impl BlockMatroid {
    pub fn len(&self) -> usize {
        self.block.len()
    }
}

/// An echelon basis, per block, of the vectors of a set of elements, each row with the elements whose vectors
/// sum to it.  A row's pivot is its lowest set bit.
pub struct Span {
    /// Per block: the rows (vector, combination over the elements) and the row of each pivot.
    rows: Vec<Vec<(Vec<u64>, Vec<u64>)>>,
    pivot: Vec<Vec<u32>>,
    combo_words: usize,
}

const NONE: u32 = u32::MAX;

/// What [`Span::reduce`] finds: the element is independent of the span, or the elements whose vectors make it.
pub enum Reduced {
    Free,
    Circuit(Vec<u64>),
}

impl Span {
    pub fn new(m: &BlockMatroid) -> Self {
        let combo_words = m.len().div_ceil(64).max(1);
        Span {
            rows: vec![Vec::new(); m.words.len()],
            pivot: m.words.iter().map(|&w| vec![NONE; 64 * w]).collect(),
            combo_words,
        }
    }

    pub fn clear(&mut self) {
        for (rows, pivot) in self.rows.iter_mut().zip(&mut self.pivot) {
            for (v, _) in rows.drain(..) {
                pivot[lowest(&v).expect("rows are nonzero")] = NONE;
            }
        }
    }

    /// Reduces element `e`'s vector by the rows of its block: `(residue, combination)`.
    fn reduce_raw(&self, m: &BlockMatroid, e: usize) -> (Vec<u64>, Vec<u64>) {
        let b = m.block[e];
        let mut v = m.vecs[e].clone();
        let mut c = vec![0u64; self.combo_words];
        let mut from = 0;
        while let Some(bit) = lowest_from(&v, from) {
            let r = self.pivot[b][bit];
            if r == NONE {
                break;
            }
            let (rv, rc) = &self.rows[b][r as usize];
            let w0 = bit / 64;
            for (a, x) in v[w0..].iter_mut().zip(&rv[w0..]) {
                *a ^= x;
            }
            for (a, x) in c.iter_mut().zip(rc) {
                *a ^= x;
            }
            from = w0;
        }
        (v, c)
    }

    /// Whether `e` is independent of the span, or else the elements of the span whose vectors sum to `e`'s.
    pub fn reduce(&self, m: &BlockMatroid, e: usize) -> Reduced {
        let (v, c) = self.reduce_raw(m, e);
        if v.iter().all(|&w| w == 0) {
            Reduced::Circuit(c)
        } else {
            Reduced::Free
        }
    }

    /// Adds `e`, which must be independent of the span.
    pub fn insert(&mut self, m: &BlockMatroid, e: usize) {
        let (v, mut c) = self.reduce_raw(m, e);
        let p = lowest(&v).expect("insert: dependent element");
        c[e / 64] ^= 1 << (e % 64);
        let b = m.block[e];
        self.pivot[b][p] = self.rows[b].len() as u32;
        self.rows[b].push((v, c));
    }
}

pub fn lowest(v: &[u64]) -> Option<usize> {
    lowest_from(v, 0)
}

/// The lowest set bit of `v`, given that the words before `from` are zero.
fn lowest_from(v: &[u64], from: usize) -> Option<usize> {
    v[from..].iter().enumerate().find(|(_, &w)| w != 0).map(|(i, w)| (from + i) * 64 + w.trailing_zeros() as usize)
}

fn bits(c: &[u64]) -> impl Iterator<Item = usize> + '_ {
    c.iter().enumerate().flat_map(|(i, &w)| {
        let mut w = w;
        std::iter::from_fn(move || {
            (w != 0).then(|| {
                let b = w.trailing_zeros() as usize;
                w &= w - 1;
                64 * i + b
            })
        })
    })
}

/// The heaviest common independent sets of `a` and `b` (on the same elements, with weights `w`).
#[derive(Clone, Debug)]
pub struct Meet {
    /// `by_size[k]`: the largest weight of a common independent set of `k` elements, for `k` up to where the
    /// algorithm stopped.
    pub by_size: Vec<i64>,
    /// The heaviest common independent set found of at most `max_size` elements.
    pub chosen: Vec<bool>,
    /// Its weight.
    pub value: i64,
    /// Augmentations made and Bellman–Ford rounds run (cost counters).
    pub augmentations: usize,
    pub rounds: usize,
    /// How many of the heaviest elements the fast algorithm started from (0 for the others).
    pub prefix: usize,
}

/// The heaviest common independent set of `a` and `b` of at most `max_size` elements, weights `w`.  Elements of
/// weight 0 or less are left out (they never make a heavier set).  With `all_sizes` the augmentations go on past
/// the heaviest set, to fill `by_size` up to the largest common independent set (or `max_size`); otherwise they
/// stop at the first augmentation that gains nothing.
pub fn intersect(a: &BlockMatroid, b: &BlockMatroid, w: &[i64], max_size: usize, all_sizes: bool) -> Meet {
    let n = a.len();
    assert_eq!(b.len(), n);
    assert_eq!(w.len(), n);
    let mut inside = vec![false; n];
    let mut span_a = Span::new(a);
    let mut span_b = Span::new(b);
    let mut meet = Meet { by_size: vec![0], chosen: vec![false; n], value: 0, augmentations: 0, rounds: 0, prefix: 0 };
    let mut weight = 0i64;
    let usable: Vec<bool> = (0..n).map(|e| w[e] > 0).collect();
    // Exchange data for the elements outside I: whether I + y is independent, else the circuit's part in I.
    let mut free_a = vec![false; n];
    let mut free_b = vec![false; n];
    let mut circ_a: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut circ_b: Vec<Vec<usize>> = vec![Vec::new(); n];
    const INF: (i64, u32) = (i64::MAX, u32::MAX);
    let mut dist = vec![INF; n];
    let mut pred = vec![usize::MAX; n];
    let mut count = 0usize;
    while count < max_size {
        span_a.clear();
        span_b.clear();
        let members: Vec<usize> = (0..n).filter(|&e| inside[e]).collect();
        for &e in &members {
            span_a.insert(a, e);
            span_b.insert(b, e);
        }
        for y in (0..n).filter(|&y| !inside[y] && usable[y]) {
            match span_a.reduce(a, y) {
                Reduced::Free => (free_a[y], circ_a[y]) = (true, Vec::new()),
                Reduced::Circuit(c) => (free_a[y], circ_a[y]) = (false, bits(&c).collect()),
            }
            match span_b.reduce(b, y) {
                Reduced::Free => (free_b[y], circ_b[y]) = (true, Vec::new()),
                Reduced::Circuit(c) => (free_b[y], circ_b[y]) = (false, bits(&c).collect()),
            }
        }
        // Bellman–Ford from the sources, lexicographic in (length, arcs).
        dist.iter_mut().for_each(|d| *d = INF);
        pred.iter_mut().for_each(|p| *p = usize::MAX);
        let outside: Vec<usize> = (0..n).filter(|&y| !inside[y] && usable[y]).collect();
        for &y in &outside {
            if free_a[y] {
                dist[y] = (-w[y], 0);
            }
        }
        let mut rounds = 0;
        loop {
            rounds += 1;
            assert!(rounds <= 2 * n + 2, "a negative cycle in the exchange graph");
            let mut changed = false;
            // y → x when I − x + y is independent in B.
            for &y in &outside {
                let d = dist[y];
                if d == INF {
                    continue;
                }
                let mut relax = |x: usize| {
                    let cand = (d.0 + w[x], d.1 + 1);
                    if cand < dist[x] {
                        dist[x] = cand;
                        pred[x] = y;
                        changed = true;
                    }
                };
                if free_b[y] {
                    members.iter().for_each(|&x| relax(x));
                } else {
                    circ_b[y].iter().for_each(|&x| relax(x));
                }
            }
            // x → y when I − x + y is independent in A.
            for &y in &outside {
                let mut best = dist[y];
                let mut from = usize::MAX;
                let mut consider = |x: usize| {
                    let d = dist[x];
                    if d != INF {
                        let cand = (d.0 - w[y], d.1 + 1);
                        if cand < best {
                            best = cand;
                            from = x;
                        }
                    }
                };
                if free_a[y] {
                    members.iter().for_each(|&x| consider(x));
                } else {
                    circ_a[y].iter().for_each(|&x| consider(x));
                }
                if from != usize::MAX {
                    dist[y] = best;
                    pred[y] = from;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        meet.rounds += rounds;
        let sink = outside.iter().copied().filter(|&y| free_b[y] && dist[y] != INF).min_by_key(|&y| dist[y]);
        let Some(sink) = sink else { break };
        let gain = -dist[sink].0;
        if gain <= 0 && !all_sizes {
            break;
        }
        // Flip the path.
        let mut v = sink;
        let mut steps = 0;
        loop {
            inside[v] = !inside[v];
            steps += 1;
            assert!(steps <= n + 1, "a cycle in the predecessors");
            if pred[v] == usize::MAX {
                break;
            }
            v = pred[v];
        }
        count += 1;
        weight += gain;
        meet.augmentations += 1;
        meet.by_size.push(weight);
        if weight > meet.value {
            meet.value = weight;
            meet.chosen.clone_from(&inside);
        }
    }
    meet
}

/// [`intersect`] for matroids whose blocks all have rank at most 64 (one word per vector), faster: each block's
/// span keeps its rows by pivot with the combination as a mask of the block's slots, and the exchange arcs from or
/// to an element that is independent of `I` (arcs to or from every member of `I`) are relaxed through one minimum
/// per round instead of one arc each.  Same answers as [`intersect`] (a test checks it).
pub fn intersect_fast(a: &BlockMatroid, b: &BlockMatroid, w: &[i64], max_size: usize) -> Meet {
    assert!(a.words.iter().chain(&b.words).all(|&w| w == 1), "intersect_fast needs blocks of rank at most 64");
    let mut fast = Fast::new(a, b, w);
    // Start from the heaviest elements, as many as are common independent before the first that is not: they
    // are the heaviest set of their size, and heavier than any smaller set (the weights are positive), so the
    // augmentations may start there.
    let mut order: Vec<usize> = (0..a.len()).filter(|&e| w[e] > 0).collect();
    order.sort_by_key(|&e| std::cmp::Reverse(w[e]));
    for &e in order.iter().take(max_size) {
        let (ra, _) = fast.spans[0].reduce(a.block[e], fast.va[e]);
        let (rb, _) = fast.spans[1].reduce(b.block[e], fast.vb[e]);
        if ra == 0 || rb == 0 {
            break;
        }
        fast.spans[0].insert(a.block[e], fast.va[e], e as u32);
        fast.spans[1].insert(b.block[e], fast.vb[e], e as u32);
        fast.inside[e] = true;
        fast.meet.prefix += 1;
    }
    fast.run(max_size, false);
    fast.meet
}

/// [`intersect_fast`] started from the common independent set `start` instead of the empty set, if `start` is
/// extreme (the heaviest common independent set of its size, which its exchange graph having no negative cycle
/// decides); `None` if it is not (or is not common independent).  The augmentations from an extreme set give the
/// same value as from the empty set.  `by_size` then starts at `start`'s size.
pub fn intersect_warm(a: &BlockMatroid, b: &BlockMatroid, w: &[i64], max_size: usize, start: &[bool]) -> Option<Meet> {
    assert!(a.words.iter().chain(&b.words).all(|&w| w == 1), "intersect_warm needs blocks of rank at most 64");
    let mut fast = Fast::new(a, b, w);
    fast.inside.copy_from_slice(start);
    fast.run(max_size, true).then_some(fast.meet)
}

/// One matroid's span of the current set, per block: rows by pivot (vector, mask of slots), and the element in
/// each slot.
struct Spans1 {
    row_v: Vec<[u64; 64]>,
    row_c: Vec<[u64; 64]>,
    has: Vec<u64>,
    slot_elem: Vec<[u32; 64]>,
    slots: Vec<u32>,
}

impl Spans1 {
    fn new(blocks: usize) -> Self {
        Spans1 {
            row_v: vec![[0; 64]; blocks],
            row_c: vec![[0; 64]; blocks],
            has: vec![0; blocks],
            slot_elem: vec![[0; 64]; blocks],
            slots: vec![0; blocks],
        }
    }

    fn clear(&mut self) {
        self.has.iter_mut().for_each(|h| *h = 0);
        self.slots.iter_mut().for_each(|s| *s = 0);
    }

    /// `(residue, slot mask)` of `v` in block `blk`: the residue is 0 when `v` is in the span (and the mask
    /// makes it), else its lowest bit is not a pivot.
    #[inline]
    fn reduce(&self, blk: usize, mut v: u64) -> (u64, u64) {
        let (rv, rc, has) = (&self.row_v[blk], &self.row_c[blk], self.has[blk]);
        let mut c = 0u64;
        while v != 0 {
            let p = v.trailing_zeros() as usize;
            if has & (1u64 << p) == 0 {
                break;
            }
            // Rows have no bits below their pivot.
            v ^= rv[p];
            c ^= rc[p];
        }
        (v, c)
    }

    fn insert(&mut self, blk: usize, v: u64, e: u32) {
        let (r, mut c) = self.reduce(blk, v);
        let p = r.trailing_zeros() as usize;
        assert!(r != 0 && self.has[blk] & (1u64 << p) == 0, "insert: dependent element");
        let slot = self.slots[blk];
        self.slots[blk] += 1;
        self.slot_elem[blk][slot as usize] = e;
        c ^= 1u64 << slot;
        self.row_v[blk][p] = r;
        self.row_c[blk][p] = c;
        self.has[blk] |= 1u64 << p;
    }
}

struct Fast<'a> {
    a: &'a BlockMatroid,
    b: &'a BlockMatroid,
    w: &'a [i64],
    va: Vec<u64>,
    vb: Vec<u64>,
    inside: Vec<bool>,
    spans: [Spans1; 2],
    meet: Meet,
}

const INF: (i64, u32) = (i64::MAX, u32::MAX);

impl<'a> Fast<'a> {
    fn new(a: &'a BlockMatroid, b: &'a BlockMatroid, w: &'a [i64]) -> Self {
        let n = a.len();
        Fast {
            a,
            b,
            w,
            va: a.vecs.iter().map(|v| v[0]).collect(),
            vb: b.vecs.iter().map(|v| v[0]).collect(),
            inside: vec![false; n],
            spans: [Spans1::new(a.words.len()), Spans1::new(b.words.len())],
            meet: Meet { by_size: vec![0], chosen: vec![false; n], value: 0, augmentations: 0, rounds: 0, prefix: 0 },
        }
    }

    /// Augments from `self.inside` (empty, or a warm start) up to `max_size` elements.  With `check`, first makes
    /// sure the start is common independent and extreme (no negative cycle in its exchange graph, Schrijver
    /// Theorem 41.5), and returns false if it is not.
    fn run(&mut self, max_size: usize, check: bool) -> bool {
        let n = self.a.len();
        let w = self.w;
        let usable: Vec<usize> = (0..n).filter(|&e| w[e] > 0).collect();
        for e in (0..n).filter(|&e| w[e] <= 0) {
            self.inside[e] = false;
        }
        let mut members: Vec<usize> = Vec::new();
        let mut outside: Vec<usize> = Vec::new();
        let mut free_a = vec![false; n];
        let mut free_b = vec![false; n];
        // Circuits, flat: for element y, ca[sa[y].0..sa[y].1].
        let (mut ca, mut cb): (Vec<u32>, Vec<u32>) = (Vec::new(), Vec::new());
        let (mut sa, mut sb) = (vec![(0u32, 0u32); n], vec![(0u32, 0u32); n]);
        let mut dist = vec![INF; n];
        let mut pred = vec![u32::MAX; n];
        let mut weight: i64 = usable.iter().filter(|&&e| self.inside[e]).map(|&e| w[e]).sum();
        let mut count = usable.iter().filter(|&&e| self.inside[e]).count();
        if weight > self.meet.value {
            self.meet.value = weight;
            self.meet.chosen.clone_from(&self.inside);
        }
        self.meet.by_size = vec![weight];
        let mut checking = check;
        while checking || count < max_size {
            members.clear();
            outside.clear();
            for &e in &usable {
                if self.inside[e] {
                    members.push(e);
                } else {
                    outside.push(e);
                }
            }
            for s in &mut self.spans {
                s.clear();
            }
            for &e in &members {
                for (s, m, v) in [(0, self.a, &self.va), (1, self.b, &self.vb)] {
                    let blk = m.block[e];
                    if checking && self.spans[s].reduce(blk, v[e]).0 == 0 {
                        return false;
                    }
                    self.spans[s].insert(blk, v[e], e as u32);
                }
            }
            ca.clear();
            cb.clear();
            for &y in &outside {
                let (r, c) = self.spans[0].reduce(self.a.block[y], self.va[y]);
                free_a[y] = r != 0;
                let start = ca.len() as u32;
                if r == 0 {
                    let blk = self.a.block[y];
                    ca.extend(bits(&[c]).map(|s| self.spans[0].slot_elem[blk][s]));
                }
                sa[y] = (start, ca.len() as u32);
                let (r, c) = self.spans[1].reduce(self.b.block[y], self.vb[y]);
                free_b[y] = r != 0;
                let start = cb.len() as u32;
                if r == 0 {
                    let blk = self.b.block[y];
                    cb.extend(bits(&[c]).map(|s| self.spans[1].slot_elem[blk][s]));
                }
                sb[y] = (start, cb.len() as u32);
            }
            for &e in members.iter().chain(&outside) {
                dist[e] = INF;
                pred[e] = u32::MAX;
            }
            if checking {
                // From every vertex: the distances settle within one round per vertex unless a cycle is negative.
                for &x in &members {
                    dist[x] = (w[x], 0);
                }
                for &y in &outside {
                    dist[y] = (-w[y], 0);
                }
            } else {
                for &y in &outside {
                    if free_a[y] {
                        dist[y] = (-w[y], 0);
                    }
                }
            }
            let limit = members.len() + outside.len() + 1;
            let mut rounds = 0;
            loop {
                rounds += 1;
                if rounds > limit + 1 {
                    assert!(checking, "a negative cycle in the exchange graph of an extreme set");
                    return false;
                }
                let mut changed = false;
                // y → x when I − x + y is independent in B.
                let mut best_free = (INF, u32::MAX);
                for &y in &outside {
                    let d = dist[y];
                    if d == INF {
                        continue;
                    }
                    if free_b[y] {
                        if d < best_free.0 {
                            best_free = (d, y as u32);
                        }
                    } else {
                        for &x in &cb[sb[y].0 as usize..sb[y].1 as usize] {
                            let cand = (d.0 + w[x as usize], d.1 + 1);
                            if cand < dist[x as usize] {
                                dist[x as usize] = cand;
                                pred[x as usize] = y as u32;
                                changed = true;
                            }
                        }
                    }
                }
                if best_free.0 != INF {
                    let (d, y) = best_free;
                    for &x in &members {
                        let cand = (d.0 + w[x], d.1 + 1);
                        if cand < dist[x] {
                            dist[x] = cand;
                            pred[x] = y;
                            changed = true;
                        }
                    }
                }
                // x → y when I − x + y is independent in A.
                let mut best_member = (INF, u32::MAX);
                for &x in &members {
                    if dist[x] < best_member.0 {
                        best_member = (dist[x], x as u32);
                    }
                }
                for &y in &outside {
                    let mut best = dist[y];
                    let mut from = u32::MAX;
                    if free_a[y] {
                        if best_member.0 != INF {
                            let cand = (best_member.0 .0 - w[y], best_member.0 .1 + 1);
                            if cand < best {
                                (best, from) = (cand, best_member.1);
                            }
                        }
                    } else {
                        for &x in &ca[sa[y].0 as usize..sa[y].1 as usize] {
                            let d = dist[x as usize];
                            if d != INF {
                                let cand = (d.0 - w[y], d.1 + 1);
                                if cand < best {
                                    (best, from) = (cand, x);
                                }
                            }
                        }
                    }
                    if from != u32::MAX {
                        dist[y] = best;
                        pred[y] = from;
                        changed = true;
                    }
                }
                if !changed {
                    break;
                }
            }
            self.meet.rounds += rounds;
            if checking {
                // No negative cycle: the start is extreme.  It must also be at least as heavy as every smaller
                // common independent set (the weights by size are concave but can fall): a path from a member to
                // a member, x₀ → y₁ → x₁ → … → y_k → x_k, exchanges the y's for the x's, one fewer, and weighs
                // its length less; none may be negative.
                for &e in members.iter().chain(&outside) {
                    dist[e] = INF;
                }
                for &x in &members {
                    dist[x] = (w[x], 0);
                }
                let mut rounds = 0;
                loop {
                    rounds += 1;
                    assert!(rounds <= limit + 1, "a negative cycle after the check");
                    let mut changed = false;
                    for &y in &outside {
                        let d = dist[y];
                        if d == INF {
                            continue;
                        }
                        let targets: &[usize] = if free_b[y] { &members } else { &[] };
                        let circuit = &cb[sb[y].0 as usize..sb[y].1 as usize];
                        for x in targets.iter().copied().chain(circuit.iter().map(|&x| x as usize)) {
                            let cand = (d.0 + w[x], d.1 + 1);
                            if cand < dist[x] {
                                dist[x] = cand;
                                changed = true;
                            }
                        }
                    }
                    for &y in &outside {
                        let sources: &[usize] = if free_a[y] { &members } else { &[] };
                        let circuit = &ca[sa[y].0 as usize..sa[y].1 as usize];
                        for x in sources.iter().copied().chain(circuit.iter().map(|&x| x as usize)) {
                            let d = dist[x];
                            if d != INF {
                                let cand = (d.0 - w[y], d.1 + 1);
                                if cand < dist[y] {
                                    dist[y] = cand;
                                    changed = true;
                                }
                            }
                        }
                    }
                    if !changed {
                        break;
                    }
                }
                self.meet.rounds += rounds;
                if members.iter().any(|&x| dist[x].0 < 0) {
                    return false;
                }
                checking = false;
                continue;
            }
            let sink = outside.iter().copied().filter(|&y| free_b[y] && dist[y] != INF).min_by_key(|&y| dist[y]);
            let Some(sink) = sink else { break };
            let gain = -dist[sink].0;
            if gain <= 0 {
                break;
            }
            let mut v = sink;
            let mut steps = 0;
            loop {
                self.inside[v] = !self.inside[v];
                steps += 1;
                assert!(steps <= n + 1, "a cycle in the predecessors");
                if pred[v] == u32::MAX {
                    break;
                }
                v = pred[v] as usize;
            }
            count += 1;
            weight += gain;
            self.meet.augmentations += 1;
            self.meet.by_size.push(weight);
            if weight > self.meet.value {
                self.meet.value = weight;
                self.meet.chosen.clone_from(&self.inside);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Rng(u64);

    impl Rng {
        fn below(&mut self, n: u64) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 % n
        }
    }

    fn random_matroid(rng: &mut Rng, n: usize) -> BlockMatroid {
        let blocks = 1 + rng.below(3) as usize;
        let dims: Vec<u64> = (0..blocks).map(|_| 1 + rng.below(5)).collect();
        let block: Vec<usize> = (0..n).map(|_| rng.below(blocks as u64) as usize).collect();
        let vecs = block.iter().map(|&b| vec![rng.below(1 << dims[b])]).collect();
        BlockMatroid { block, vecs, words: vec![1; blocks] }
    }

    fn independent(m: &BlockMatroid, set: &[usize]) -> bool {
        let mut span = Span::new(m);
        for &e in set {
            if matches!(span.reduce(m, e), Reduced::Circuit(_)) {
                return false;
            }
            span.insert(m, e);
        }
        true
    }

    /// The heaviest weight of a common independent set of each size, by brute force.
    fn brute(a: &BlockMatroid, b: &BlockMatroid, w: &[i64]) -> Vec<Option<i64>> {
        let n = a.len();
        let mut best = vec![None; n + 1];
        for mask in 0u32..(1 << n) {
            let set: Vec<usize> = (0..n).filter(|&e| mask >> e & 1 == 1).collect();
            if independent(a, &set) && independent(b, &set) {
                let sum: i64 = set.iter().map(|&e| w[e]).sum();
                let slot = &mut best[set.len()];
                *slot = Some(slot.map_or(sum, |s: i64| s.max(sum)));
            }
        }
        best
    }

    #[test]
    fn intersection_against_brute_force() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let (mut warm_ok, mut warm_refused) = (0, 0);
        for trial in 0..3000 {
            let n = 1 + rng.below(11) as usize;
            let a = random_matroid(&mut rng, n);
            let b = random_matroid(&mut rng, n);
            let w: Vec<i64> = (0..n).map(|_| 1 + rng.below(20) as i64).collect();
            let want = brute(&a, &b, &w);
            let got = intersect(&a, &b, &w, n, true);
            let largest = want.iter().rposition(Option::is_some).unwrap();
            assert_eq!(got.by_size.len() - 1, largest, "trial {trial}: the largest common independent set");
            for (k, &v) in got.by_size.iter().enumerate() {
                assert_eq!(Some(v), want[k], "trial {trial}: size {k}");
            }
            let heaviest = want.iter().flatten().copied().max().unwrap();
            assert_eq!(got.value, heaviest, "trial {trial}");
            // Warm starts: the optimum without one member, and random common independent sets.  When the start
            // passes the check, the result is the optimum.
            let opt: Vec<usize> = (0..n).filter(|&e| got.chosen[e]).collect();
            let mut starts: Vec<Vec<bool>> = Vec::new();
            for &z in &opt {
                let mut s = got.chosen.clone();
                s[z] = false;
                starts.push(s);
            }
            for _ in 0..4 {
                let mut s = vec![false; n];
                let mut set = Vec::new();
                for _ in 0..n {
                    let e = rng.below(n as u64) as usize;
                    if !s[e] {
                        set.push(e);
                        if independent(&a, &set) && independent(&b, &set) {
                            s[e] = true;
                        } else {
                            set.pop();
                        }
                    }
                }
                starts.push(s);
            }
            for s in &starts {
                let size = s.iter().filter(|&&x| x).count();
                for cap in size..=n {
                    if let Some(m) = intersect_warm(&a, &b, &w, cap, s) {
                        let limited = want[..=cap].iter().flatten().copied().max().unwrap();
                        assert_eq!(m.value, limited, "trial {trial}: warm start {s:?}, cap {cap}");
                        warm_ok += 1;
                    } else {
                        warm_refused += 1;
                    }
                }
            }
            let chosen: Vec<usize> = (0..n).filter(|&e| got.chosen[e]).collect();
            assert!(independent(&a, &chosen) && independent(&b, &chosen));
            assert_eq!(chosen.iter().map(|&e| w[e]).sum::<i64>(), got.value);
            // Stopping at the first augmentation that gains nothing finds the same value (concavity).
            assert_eq!(intersect(&a, &b, &w, n, false).value, heaviest, "trial {trial}");
            // A size limit.
            for cap in 0..=n {
                let limited = want[..=cap].iter().flatten().copied().max().unwrap();
                assert_eq!(intersect(&a, &b, &w, cap, false).value, limited, "trial {trial}, cap {cap}");
                let fast = intersect_fast(&a, &b, &w, cap);
                assert_eq!(fast.value, limited, "trial {trial}, cap {cap}: intersect_fast");
                let chosen: Vec<usize> = (0..n).filter(|&e| fast.chosen[e]).collect();
                assert!(independent(&a, &chosen) && independent(&b, &chosen) && chosen.len() <= cap);
                assert_eq!(chosen.iter().map(|&e| w[e]).sum::<i64>(), fast.value);
            }
        }
        // The check must let most starts through, or the warm start tests nothing.
        assert!(warm_ok > warm_refused / 4, "warm starts: {warm_ok} accepted, {warm_refused} refused");
        eprintln!("warm starts: {warm_ok} accepted, {warm_refused} refused");
    }
}
