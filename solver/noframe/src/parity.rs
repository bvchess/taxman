//! The parity vectors: an upper bound on opt(n) on the whole board, the heaviest set independent at both parities,
//! found by weighted matroid intersection over GF(2).
//!
//! The parity bound (W1, docs/theory/parity-bound.md).  Give every number s of the board 2..=n the vector v(s) over
//! GF(2) with a 1 at each of its maximal factors (coordinates 1..=n/2), and, when s <= n/2, also the unit vector e_s.
//! If S is playable, the vectors {v(s) : s ∈ S} ∪ {e_s : s ∈ S, s <= n/2} are linearly independent: in a legal order
//! each move pays with a maximal factor still in the pot (the lifting lemma, P1, docs/theory/playability.md), which no
//! earlier move lists and which is no pick, so restricted to the coordinates outside S the v(s) are independent (the
//! last-played member of a vanishing sum has a coordinate no other member of the sum has), and the e_s are exactly the
//! coordinates in S.  A maximal factor has one prime factor fewer, so v(s) lies in the coordinates whose Ω has the
//! other parity than Ω(s), and e_s in those of s's own parity.  The coordinates split into the even-Ω and the odd-Ω
//! ones, every number has one vector in each part (none in its own parity when s > n/2: it gets an extra coordinate of
//! its own there, so that it is independent of everything at that parity), and a family of vectors each in one part is
//! independent iff its vectors in each part are.  So every playable set is independent at both parities, the parity-i
//! vectors being those of [`Parity`]'s class i (class 0 for even Ω, class 1 for odd Ω), and the heaviest set
//! independent at both parities bounds opt(n).  At a node of a search with forced picks P and barred numbers X, the
//! same holds together with P and with X left out: a playable S ⊇ P has S − P independent at both parities together
//! with P.
//!
//! [`Meet`] computes, by weighted matroid intersection, the heaviest set independent in both of two families of
//! sparse vectors over GF(2) ([`Pair`]'s class 0, the side A, and class 1, the side B), with forced elements (the set
//! is independent together with them) and barred ones (left out), and *certifies* it with a weight split.
//! The value it reports rests only on the certificate, not on the algorithm that found the set:
//!
//! * **Weak duality.**  If w = w1 + w2, then every T independent in both satisfies w(T) = w1(T) + w2(T) <=
//!   max{w1(I) : I independent in A} + max{w2(J) : J independent in B}.
//! * **Local optimality** (one family of vectors, weights of any sign, sets of any size).  S independent is a heaviest
//!   independent set iff (1) w(x) >= 0 for x ∈ S, (2) w(y) <= 0 whenever S + y is independent, and (3) w(y) <= w(x)
//!   whenever x is in the circuit of S + y.  (Add r dummies of weight 0, each with a coordinate of its own, r the rank
//!   of the family, and keep the independent sets of at most r members: the independent sets become bases, padded with
//!   dummies, and (1)-(3) are the classical criterion that no single exchange improves a basis.)
//!
//! So if S is independent in both and a split satisfies (1)-(3) for S in A with w1 and in B with w2, then w(S) is
//! the maximum.  [`Meet::solve`] checks (1)-(3) explicitly, against the circuits of S, and reports a value only when
//! they hold.  The same certificate bounds the maximum with one member x of S barred: in one family the heaviest
//! independent set avoiding x is S − x plus the heaviest y with S − x + y independent if that weighs more than 0
//! (the exchange from a heaviest set, docs/theory/parity-bound.md, "Computing and checking the bound"), so
//! w1(x) − max(0, max w1(y) over the y whose circuit holds x) comes off the first maximum, and likewise in B
//! ([`Bound::without`]).
//!
//! The set itself comes from the shortest augmenting path algorithm (Lawler; Schrijver, *Combinatorial
//! Optimization*, Theorem 41.7): from a set I that is heaviest among the sets of its size, the exchange graph has
//! an arc x → y (x ∈ I, y ∉ I) when x is in the circuit of I + y in A, and y → x when it is in B's; a path from a y
//! with I + y independent in A to one with I + y independent in B, shortest for the lengths w(x) on I and −w(y)
//! off it and with the fewest arcs among those, gives a heaviest set of size |I| + 1.  The heaviest weights by size
//! are concave, so the heaviest set of all is reached when the shortest path stops being negative.  The paths are
//! found by Dijkstra on costs reduced by potentials, which start from the weight split of the starting set (the
//! longest prefix of the elements by weight that is independent in both) and are updated by the distances, as in
//! Frank's weight-splitting algorithm.  Circuits come from an echelon basis over GF(2) kept in reduced form, whose
//! rows carry the members they sum: reducing a sparse vector takes one row per coordinate of it.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use crate::game::{Board, Role};

const NONE: u32 = u32::MAX;
const INF: i64 = i64::MAX / 4;

/// Two families of vectors over GF(2) on the elements 0..len: in class q, element e is the vector with ones at
/// `coords[q][e]` (coordinates 0..dims[q]), or, when that list is empty, has a coordinate of its own and is
/// independent of everything in class q.
#[derive(Clone)]
pub struct Pair {
    pub dims: [usize; 2],
    pub coords: [Vec<Vec<u32>>; 2],
    pub weight: Vec<i64>,
}

impl Pair {
    pub fn len(&self) -> usize {
        self.weight.len()
    }

    pub fn is_empty(&self) -> bool {
        self.weight.is_empty()
    }
}

/// The parity vectors of a board: the elements are the numbers 0..=n (0 and 1 are never picked), the coordinates
/// of class q the numbers f <= n/2 whose Ω has parity q (1 among the even ones), and each number s has v(s), its
/// maximal factors, in the class of the other parity, and e_s (s <= n/2) or an extra coordinate (s > n/2) in its own.
pub struct Parity;

impl Parity {
    /// The pair for `board`, with the pick of each number c weighing `weights[c]`.
    pub fn pair(board: &Board, weights: &[i64]) -> Pair {
        let n = board.n as usize;
        let half = n / 2;
        let class = |k: usize| usize::from(!board.even[k]);
        let mut index = vec![NONE; half + 1];
        let mut dims = [0usize; 2];
        for (f, slot) in index.iter_mut().enumerate().skip(1) {
            let q = class(f);
            *slot = dims[q] as u32;
            dims[q] += 1;
        }
        let mut coords = [vec![Vec::new(); n + 1], vec![Vec::new(); n + 1]];
        for s in 2..=n {
            let own = class(s);
            coords[1 - own][s] = board.mf[s].iter().map(|&f| index[f as usize]).collect();
            if s <= half {
                coords[own][s] = vec![index[s]];
            }
        }
        Pair { dims, coords, weight: weights.to_vec() }
    }
}

/// An echelon basis over GF(2) in reduced form (each pivot column is zero in every other row), each row with the
/// set of tracked elements whose vectors (plus untracked ones) sum to it.
struct Rref {
    vw: usize,
    cw: usize,
    vecs: Vec<u64>,
    combos: Vec<u64>,
    pivot: Vec<u32>,
    row_of: Vec<u32>,
    rows: usize,
}

impl Rref {
    fn new(dim: usize, cw: usize) -> Self {
        let vw = dim.div_ceil(64).max(1);
        Rref {
            vw,
            cw,
            vecs: vec![0; dim.max(1) * vw],
            combos: vec![0; dim.max(1) * cw],
            pivot: vec![NONE; dim.max(1)],
            row_of: vec![NONE; 64 * vw],
            rows: 0,
        }
    }

    fn clear(&mut self) {
        for r in 0..self.rows {
            self.row_of[self.pivot[r] as usize] = NONE;
        }
        self.rows = 0;
    }

    /// Reduces the vector with ones at `coords`: the residue in `v` (zero at every pivot), the rows' combination
    /// in `c`.  True iff the residue is nonzero (the vector is independent of the rows).
    fn reduce(&self, coords: &[u32], v: &mut [u64], c: &mut [u64]) -> bool {
        let (vw, cw) = (self.vw, self.cw);
        v[..vw].fill(0);
        c[..cw].fill(0);
        for &k in coords {
            v[k as usize / 64] ^= 1 << (k % 64);
        }
        for &k in coords {
            let r = self.row_of[k as usize];
            if r != NONE {
                let r = r as usize;
                for (a, b) in v[..vw].iter_mut().zip(&self.vecs[r * vw..(r + 1) * vw]) {
                    *a ^= b;
                }
                for (a, b) in c[..cw].iter_mut().zip(&self.combos[r * cw..(r + 1) * cw]) {
                    *a ^= b;
                }
            }
        }
        v[..vw].iter().any(|&x| x != 0)
    }

    /// Adds a row: `v` a nonzero residue from [`reduce`](Self::reduce), `c` its combination.
    fn insert(&mut self, v: &[u64], c: &[u64]) {
        let (vw, cw) = (self.vw, self.cw);
        let w0 = v[..vw].iter().position(|&x| x != 0).expect("insert: a nonzero residue");
        let p = w0 * 64 + v[w0].trailing_zeros() as usize;
        let bit = 1u64 << (p % 64);
        for r in 0..self.rows {
            if self.vecs[r * vw + p / 64] & bit != 0 {
                for (a, b) in self.vecs[r * vw..(r + 1) * vw].iter_mut().zip(&v[..vw]) {
                    *a ^= b;
                }
                for (a, b) in self.combos[r * cw..(r + 1) * cw].iter_mut().zip(&c[..cw]) {
                    *a ^= b;
                }
            }
        }
        let r = self.rows;
        self.vecs[r * vw..(r + 1) * vw].copy_from_slice(&v[..vw]);
        self.combos[r * cw..(r + 1) * cw].copy_from_slice(&c[..cw]);
        self.pivot[r] = p as u32;
        self.row_of[p] = r as u32;
        self.rows += 1;
    }

    /// Removes tracked element `e` from the span: eliminates it from every row's combination with one row holding
    /// it, and drops that row.  False if no row holds it.
    fn remove(&mut self, e: usize) -> bool {
        let (vw, cw) = (self.vw, self.cw);
        let (word, bit) = (e / 64, 1u64 << (e % 64));
        let Some(r) = (0..self.rows).find(|&r| self.combos[r * cw + word] & bit != 0) else { return false };
        for q in 0..self.rows {
            if q != r && self.combos[q * cw + word] & bit != 0 {
                for k in 0..vw {
                    self.vecs[q * vw + k] ^= self.vecs[r * vw + k];
                }
                for k in 0..cw {
                    self.combos[q * cw + k] ^= self.combos[r * cw + k];
                }
            }
        }
        self.row_of[self.pivot[r] as usize] = NONE;
        let last = self.rows - 1;
        if r != last {
            self.vecs.copy_within(last * vw..(last + 1) * vw, r * vw);
            self.combos.copy_within(last * cw..(last + 1) * cw, r * cw);
            self.pivot[r] = self.pivot[last];
            self.row_of[self.pivot[r] as usize] = r as u32;
        }
        self.rows = last;
        true
    }
}

/// The ones of a bitset.
fn ones(words: &[u64]) -> impl Iterator<Item = usize> + '_ {
    words.iter().enumerate().flat_map(|(i, &w)| {
        let mut b = w;
        std::iter::from_fn(move || {
            if b == 0 {
                return None;
            }
            let t = b.trailing_zeros() as usize;
            b &= b - 1;
            Some(i * 64 + t)
        })
    })
}

/// What [`Meet::solve`] returns.
#[derive(Clone, Debug)]
pub enum Outcome {
    /// The forced elements are dependent in class 0 or in class 1: no set holds them all.
    Infeasible,
    /// The certificate failed (not expected; see [`MeetStats::uncertified`]): no bound is claimed.
    Uncertified,
    Bound(Bound),
}

/// A certified bound.
#[derive(Clone, Debug)]
pub struct Bound {
    /// The heaviest weight of a set independent in both classes that holds every forced element and no barred
    /// one (for the parity vectors: an upper bound on every playable set with these roles).
    pub value: i64,
    /// A heaviest such set, forced elements included, ascending.
    pub set: Vec<u32>,
    /// Free elements that depend on the forced ones in one of the classes: no set counted by `value` holds one.
    pub loops: Vec<u32>,
    /// For each free member x of `set`: an upper bound on `value` with x barred.
    pub without: Vec<(u32, i64)>,
    /// The certificate, to start a solve with more elements forced or barred from ([`Meet::solve_from`]).
    pub warm: Warm,
}

/// A certified heaviest set's free members and its weight split (w1 of each candidate, w2 = w − w1).
#[derive(Clone, Debug, Default)]
pub struct Warm {
    pub members: Vec<u32>,
    pub w1: Vec<(u32, i64)>,
}

/// Counters of [`Meet`].
#[derive(Clone, Copy, Debug, Default)]
pub struct MeetStats {
    pub solves: u64,
    /// Augmentations of the solves from scratch, and arcs of the exchange graphs built.
    pub augmentations: u64,
    pub arcs: u64,
    /// Solves whose certificate failed (none expected; the value is then not reported).
    pub uncertified: u64,
    /// Arcs found with a negative reduced cost in a solve from scratch (none expected).
    pub negative: u64,
    /// Solves started from a parent's certificate, their improving cycles, and those that fell back to a solve from
    /// scratch.
    pub warm: u64,
    pub cycles: u64,
    pub fallbacks: u64,
    /// Why they fell back: [the start set was dependent or the split unknown, a negative reduced cost, a dependent
    /// exchange, the certificate failed].
    pub fallback_reasons: [u64; 4],
    /// Warm starts whose forced elements include a candidate of the certificate that was not in its set, and how
    /// many of them fell back.
    pub foreign: [u64; 2],
}

/// The weighted intersection of a [`Pair`] (see the module comment).
///
/// The exchange graph's vertices are the candidates (free elements that do not depend on the forced ones, in either
/// class: not the `loops`) and two hubs: the arcs x → y from every member x to every y with I + y independent in
/// class 0 run through hub `A` (x → A → y), and those from every y with I + y independent in class 1 to every member
/// x through hub `B`, each such pair of arcs counting as one.  The other arcs are the circuits': x → y for x in y's
/// circuit of class 0, y → x for x in its circuit of class 1.
///
/// A solve from a parent's certificate ([`solve_from`](Self::solve_from)) starts from the parent's free members
/// that are still candidates, with the parent's split as potentials, which stay feasible on every circuit arc
/// (forcing members of a heaviest set or barring elements only removes vertices and shortens circuits; barring a
/// member makes the elements whose circuit held it independent of the rest).  What can fail are the conditions at
/// the root: (2) for the elements made independent, (1) and (3) nowhere.  So it improves the set by shortest
/// paths in the graph whose source and sink stand for adding an element or dropping a member: from the source to
/// each y with I + y independent in class 0 (length −w(y)) and to each member (w(x)); to the sink from each y with
/// I + y independent in class 1 and from each member (length 0); an improving path is a negative cycle through the
/// root, and none left means the split from the distances certifies the set.  The set is not the heaviest of its
/// size (the parent's minus a barred member seldom is), so after an exchange the potentials can fail on new
/// circuit arcs: these searches are label-correcting ([`dijkstra`](Self::dijkstra), `tolerant`), exact unless a
/// cycle away from the root is negative, in which case they give up.  That, or an exchange that leaves the set
/// dependent, falls back to a solve from scratch (2-12% of the warm starts at n = 990-1250); the certificate is
/// checked either way.
pub struct Meet {
    pair: Pair,
    cw: usize,
    rref: [Rref; 2],
    cands: Vec<u32>,
    in_set: Vec<bool>,
    /// Potentials of the elements and the two hubs (indices len and len + 1).
    pot: Vec<i64>,
    indep: [Vec<bool>; 2],
    circ: [Vec<u64>; 2],
    dist: Vec<(i64, u32)>,
    prev: Vec<u32>,
    vbuf: Vec<u64>,
    cbuf: Vec<u64>,
    out_start: Vec<u32>,
    out_list: Vec<u32>,
    heap: BinaryHeap<Reverse<(i64, u32, u32)>>,
    /// Scratch: w1 of a warm start by element (INF where unknown), and whether an element is a candidate.
    split: Vec<i64>,
    is_cand: Vec<bool>,
    pub stats: MeetStats,
}

/// Dijkstra's relaxation of the arc `from → to` at reduced cost `reduced`, counting `hop` arcs.
#[allow(clippy::too_many_arguments)]
fn relax(
    dist: &mut [(i64, u32)],
    prev: &mut [u32],
    heap: &mut BinaryHeap<Reverse<(i64, u32, u32)>>,
    negative: &mut u64,
    from: usize,
    to: usize,
    at: (i64, u32),
    reduced: i64,
    hop: u32,
) {
    if reduced < 0 {
        *negative += 1;
    }
    let nd = (at.0 + reduced, at.1 + hop);
    if nd < dist[to] {
        dist[to] = nd;
        prev[to] = from as u32;
        heap.push(Reverse((nd.0, nd.1, to as u32)));
    }
}

/// What [`Meet::dijkstra`] found: the source vertex's potential, the number of arcs met with a negative reduced cost,
/// the best reduced distance to the sink and the last element before it, and whether a tolerant search gave up.
struct Search {
    pi_s: i64,
    negative: u64,
    best: (i64, u32),
    last: u32,
    overrun: bool,
}

/// The average number of scans of a vertex after which a tolerant search gives up.
const TOLERANT_SCANS: usize = 20;

/// How a search for an improving path ended.
enum Step {
    Improved,
    Optimal,
    /// A negative reduced cost: the potentials are not feasible (the warm start falls back).
    Negative,
    /// The exchange left the set dependent (the warm start falls back).
    Dependent,
}

impl Meet {
    pub fn new(pair: Pair) -> Self {
        let len = pair.len();
        let cw = len.div_ceil(64).max(1);
        let vw = pair.dims.iter().map(|d| d.div_ceil(64).max(1)).max().unwrap_or(1);
        let rref = [Rref::new(pair.dims[0], cw), Rref::new(pair.dims[1], cw)];
        Meet {
            pair,
            cw,
            rref,
            cands: Vec::new(),
            in_set: vec![false; len],
            pot: vec![0; len + 2],
            indep: [vec![false; len], vec![false; len]],
            circ: [vec![0; len * cw], vec![0; len * cw]],
            dist: vec![(INF, 0); len + 2],
            prev: vec![NONE; len + 2],
            vbuf: vec![0; vw],
            cbuf: vec![0; cw],
            out_start: vec![0; len + 1],
            out_list: Vec::new(),
            heap: BinaryHeap::new(),
            split: vec![INF; len],
            is_cand: vec![false; len],
            stats: MeetStats::default(),
        }
    }

    /// Reduces element `e` in class `q` against the current rows (into `vbuf`, `cbuf`); true iff independent.
    /// An element with a coordinate of its own (an empty list) is independent.
    fn reduce(&mut self, q: usize, e: usize) -> bool {
        let coords = &self.pair.coords[q][e];
        coords.is_empty() || self.rref[q].reduce(coords, &mut self.vbuf, &mut self.cbuf)
    }

    /// Adds element `e` (independent in both classes) to the rows, tracked or not.
    fn add(&mut self, e: usize, tracked: bool) {
        for q in 0..2 {
            if self.pair.coords[q][e].is_empty() {
                continue;
            }
            let free = self.reduce(q, e);
            assert!(free, "add: element {e} is dependent in class {q}");
            if tracked {
                self.cbuf[e / 64] ^= 1 << (e % 64);
            } else {
                self.cbuf.fill(0);
            }
            self.rref[q].insert(&self.vbuf, &self.cbuf);
        }
    }

    fn take_out(&mut self, e: usize) {
        for q in 0..2 {
            if !self.pair.coords[q][e].is_empty() {
                let found = self.rref[q].remove(e);
                assert!(found, "take_out: element {e} is not in the span of class {q}");
            }
        }
    }

    /// The two families of vectors.
    pub fn pair(&self) -> &Pair {
        &self.pair
    }

    /// The heaviest set independent in both classes among those holding every element with role `Pick` and none
    /// with role `Barred`, certified (module comment).
    pub fn solve(&mut self, roles: &[Role]) -> Outcome {
        self.solve_from(roles, None)
    }

    /// [`solve`](Self::solve), starting from `warm`, the certificate of a solve with fewer elements forced or
    /// barred whose forced elements were members of its set (otherwise it falls back to a solve from scratch).
    pub fn solve_from(&mut self, roles: &[Role], warm: Option<&Warm>) -> Outcome {
        self.stats.solves += 1;
        let Some((forced_weight, forced, loops)) = self.setup(roles) else { return Outcome::Infeasible };
        if let Some(warm) = warm {
            self.stats.warm += 1;
            let foreign = forced.iter().any(|&f| warm.w1.iter().any(|&(e, _)| e == f) && !warm.members.contains(&f));
            let before = self.stats.fallbacks;
            self.stats.foreign[0] += u64::from(foreign);
            match self.start_warm(warm) {
                Ok(()) => match self.finish(forced_weight, &forced, &loops) {
                    Some(bound) => return Outcome::Bound(bound),
                    None => self.stats.fallback_reasons[3] += 1,
                },
                Err(reason) => self.stats.fallback_reasons[reason] += 1,
            }
            self.stats.fallbacks += 1;
            self.stats.foreign[1] += u64::from(foreign && self.stats.fallbacks > before);
            self.setup(roles).expect("the forced elements were independent a moment ago");
        }
        self.start_prefix();
        loop {
            self.exchange_graph();
            #[cfg(test)]
            if let Some(bad) = self.violated_arc() {
                panic!(
                    "after {} augmentations: {bad}\nroles {roles:?}\ndims {:?} coords {:?} weights {:?}",
                    self.stats.augmentations, self.pair.dims, self.pair.coords, self.pair.weight
                );
            }
            if !self.augment() {
                break;
            }
            self.stats.augmentations += 1;
        }
        match self.finish(forced_weight, &forced, &loops) {
            Some(bound) => Outcome::Bound(bound),
            None => {
                self.stats.uncertified += 1;
                Outcome::Uncertified
            }
        }
    }

    /// Clears the rows, adds the forced elements (untracked), and sorts the free ones into candidates (by weight,
    /// heaviest first) and `loops` (those that depend on the forced ones).  None if the forced elements are dependent.
    fn setup(&mut self, roles: &[Role]) -> Option<(i64, Vec<u32>, Vec<u32>)> {
        let len = self.pair.len();
        for q in 0..2 {
            self.rref[q].clear();
        }
        for &e in &self.cands {
            self.is_cand[e as usize] = false;
        }
        let mut forced_weight = 0;
        let mut forced: Vec<u32> = Vec::new();
        for e in (0..len).filter(|&e| roles[e] == Role::Pick) {
            if !(0..2).all(|q| self.reduce(q, e)) {
                self.cands.clear();
                return None;
            }
            self.add(e, false);
            forced_weight += self.pair.weight[e];
            forced.push(e as u32);
        }
        let mut cands: Vec<u32> = Vec::new();
        let mut loops: Vec<u32> = Vec::new();
        for e in (0..len).filter(|&e| roles[e] == Role::Free) {
            if (0..2).all(|q| self.reduce(q, e)) {
                cands.push(e as u32);
            } else {
                loops.push(e as u32);
            }
        }
        let w = &self.pair.weight;
        cands.sort_unstable_by_key(|&e| (Reverse(w[e as usize]), Reverse(e)));
        for &e in &cands {
            self.in_set[e as usize] = false;
            self.is_cand[e as usize] = true;
        }
        self.cands = cands;
        Some((forced_weight, forced, loops))
    }

    /// The start of a solve from scratch: the longest prefix by weight that is independent in both, the heaviest
    /// set of its size, with the split w1 = w, w2 = 0 (every element left out weighs at most every one taken),
    /// that is, potentials w on the set, 0 off it, the least member's weight at hub A and 0 at hub B.
    fn start_prefix(&mut self) {
        let w = self.pair.weight.clone();
        let len = self.pair.len();
        let mut least = None;
        for i in 0..self.cands.len() {
            let e = self.cands[i] as usize;
            if !(0..2).all(|q| self.reduce(q, e)) {
                break;
            }
            self.add(e, true);
            self.in_set[e] = true;
            least = Some(w[e]);
        }
        for &e in &self.cands {
            self.pot[e as usize] = if self.in_set[e as usize] { w[e as usize] } else { 0 };
        }
        self.pot[len] = least.unwrap_or_else(|| self.cands.iter().map(|&e| w[e as usize]).max().unwrap_or(0));
        self.pot[len + 1] = 0;
    }

    /// The start of a solve from a certificate (see the type's comment): its members that are still candidates
    /// and the potentials of its split, then improving paths through the root until none is left.  An error (the
    /// index of [`MeetStats::fallback_reasons`]) if it must fall back to a solve from scratch.
    fn start_warm(&mut self, warm: &Warm) -> Result<(), usize> {
        let w = self.pair.weight.clone();
        for &(e, w1) in &warm.w1 {
            self.split[e as usize] = w1;
        }
        let known = self.cands.iter().all(|&e| self.split[e as usize] < INF);
        let mut ok = known;
        if ok {
            for &e in &warm.members {
                let e = e as usize;
                if !self.is_cand[e] {
                    continue;
                }
                if !(0..2).all(|q| self.reduce(q, e)) {
                    ok = false;
                    break;
                }
                self.add(e, true);
                self.in_set[e] = true;
            }
        }
        if ok {
            for &e in &self.cands {
                let e = e as usize;
                self.pot[e] = if self.in_set[e] { self.split[e] } else { self.split[e] - w[e] };
            }
        }
        for &(e, _) in &warm.w1 {
            self.split[e as usize] = INF;
        }
        if !ok {
            return Err(0);
        }
        loop {
            self.exchange_graph();
            match self.improve() {
                Step::Improved => self.stats.cycles += 1,
                Step::Optimal => return Ok(()),
                Step::Negative => return Err(1),
                Step::Dependent => return Err(2),
            }
        }
    }

    /// The circuits of the candidates outside the set in both classes, and the circuit arcs x → y of class 0 by x.
    fn exchange_graph(&mut self) {
        let cw = self.cw;
        for i in 0..self.cands.len() {
            let y = self.cands[i] as usize;
            if self.in_set[y] {
                continue;
            }
            for q in 0..2 {
                let free = self.reduce(q, y);
                self.indep[q][y] = free;
                if !free {
                    self.circ[q][y * cw..(y + 1) * cw].copy_from_slice(&self.cbuf);
                }
            }
        }
        let len = self.pair.len();
        self.out_start[..=len].fill(0);
        let mut arcs = 0u64;
        for &y in &self.cands {
            let y = y as usize;
            if self.in_set[y] {
                continue;
            }
            if !self.indep[0][y] {
                for x in ones(&self.circ[0][y * cw..(y + 1) * cw]) {
                    self.out_start[x + 1] += 1;
                }
            }
            if !self.indep[1][y] {
                arcs += self.circ[1][y * cw..(y + 1) * cw].iter().map(|w| u64::from(w.count_ones())).sum::<u64>();
            }
        }
        for i in 0..len {
            self.out_start[i + 1] += self.out_start[i];
        }
        let total = self.out_start[len] as usize;
        arcs += total as u64;
        self.stats.arcs += arcs;
        self.out_list.resize(total, 0);
        let mut fill: Vec<u32> = self.out_start[..len].to_vec();
        for &y in &self.cands {
            let y = y as usize;
            if self.in_set[y] || self.indep[0][y] {
                continue;
            }
            for x in ones(&self.circ[0][y * cw..(y + 1) * cw]) {
                self.out_list[fill[x] as usize] = y as u32;
                fill[x] += 1;
            }
        }
    }

    /// Shortest paths over the exchange graph on reduced costs, lexicographic in (cost, arcs), from `sources`
    /// (element, true cost of the arc from the source vertex) to every vertex, and with `to_sink` (the sink's
    /// potential) to the sink.  With `hubs`, through the hubs too; with `drops`, every member reaches the sink (at
    /// true cost 0).  Plain, it is Dijkstra, which needs the potentials feasible: it stops at the first arc with a
    /// negative reduced cost, and at the sink.  `tolerant`, it is label-correcting (a vertex is scanned again when
    /// its distance falls), which is exact whenever no cycle is negative, with no early stop and at most
    /// [`TOLERANT_SCANS`] scans a vertex on average (else it gives up: `overrun`).
    fn dijkstra(
        &mut self,
        sources: &[(u32, i64)],
        to_sink: Option<i64>,
        hubs: bool,
        drops: bool,
        tolerant: bool,
    ) -> Search {
        let Meet { pair, cw, cands, in_set, pot, indep, circ, dist, prev, out_start, out_list, heap, .. } = self;
        let (w, cw, len) = (&pair.weight, *cw, pair.len());
        let (ha, hb) = (len, len + 1);
        for &e in cands.iter() {
            dist[e as usize] = (INF, 0);
            prev[e as usize] = NONE;
        }
        for hub in [ha, hb] {
            dist[hub] = (INF, 0);
            prev[hub] = NONE;
        }
        let pi_s = sources.iter().map(|&(e, c)| pot[e as usize] - c).max().unwrap_or(0);
        heap.clear();
        for &(e, c) in sources {
            let d = (pi_s + c - pot[e as usize], 1u32);
            if d < dist[e as usize] {
                dist[e as usize] = d;
                heap.push(Reverse((d.0, d.1, e)));
            }
        }
        let mut best = (INF, u32::MAX);
        let mut last = NONE;
        let mut negative = 0u64;
        let mut scans = TOLERANT_SCANS * (cands.len() + 2);
        while let Some(Reverse((d, h, u))) = heap.pop() {
            let u = u as usize;
            let at = (d, h);
            if at > dist[u] {
                continue;
            }
            if tolerant {
                if scans == 0 {
                    return Search { pi_s, negative, best, last, overrun: true };
                }
                scans -= 1;
            } else {
                if negative > 0 {
                    // The potentials are not feasible: the distances mean nothing (and could fall forever).
                    break;
                }
                if to_sink.is_some() && at >= best {
                    break;
                }
            }
            if u == ha {
                for &y in cands.iter() {
                    let y = y as usize;
                    if !in_set[y] && indep[0][y] {
                        relax(dist, prev, heap, &mut negative, u, y, at, pot[ha] - w[y] - pot[y], 1);
                    }
                }
            } else if u == hb {
                for &x in cands.iter() {
                    let x = x as usize;
                    if in_set[x] {
                        relax(dist, prev, heap, &mut negative, u, x, at, pot[hb] + w[x] - pot[x], 1);
                    }
                }
            } else if !in_set[u] {
                if indep[1][u] {
                    if let Some(pi_t) = to_sink {
                        let reach = (d + pot[u] - pi_t, h + 1);
                        if reach < best {
                            best = reach;
                            last = u as u32;
                        }
                    }
                    if hubs {
                        relax(dist, prev, heap, &mut negative, u, hb, at, pot[u] - pot[hb], 0);
                    }
                } else {
                    for x in ones(&circ[1][u * cw..(u + 1) * cw]) {
                        relax(dist, prev, heap, &mut negative, u, x, at, pot[u] + w[x] - pot[x], 1);
                    }
                }
            } else {
                if let (Some(pi_t), true) = (to_sink, drops) {
                    let reach = (d + pot[u] - pi_t, h + 1);
                    if reach < best {
                        best = reach;
                        last = u as u32;
                    }
                }
                for i in out_start[u]..out_start[u + 1] {
                    let y = out_list[i as usize] as usize;
                    relax(dist, prev, heap, &mut negative, u, y, at, pot[u] - w[y] - pot[y], 1);
                }
                if hubs {
                    relax(dist, prev, heap, &mut negative, u, ha, at, pot[u] - pot[ha], 0);
                }
            }
        }
        Search { pi_s, negative, best, last, overrun: false }
    }

    /// Test aid: an arc of the exchange graph with a negative reduced cost, if any.
    #[cfg(test)]
    fn violated_arc(&self) -> Option<String> {
        let (w, cw, len) = (&self.pair.weight, self.cw, self.pair.len());
        let (ha, hb) = (len, len + 1);
        let pot = &self.pot;
        let set: Vec<usize> = self.cands.iter().map(|&e| e as usize).filter(|&e| self.in_set[e]).collect();
        for &y in &self.cands {
            let y = y as usize;
            if self.in_set[y] {
                if pot[y] - pot[ha] < 0 {
                    return Some(format!("{y} -> A: {} {}", pot[y], pot[ha]));
                }
                if pot[hb] + w[y] - pot[y] < 0 {
                    return Some(format!("B -> {y}: {} {} {}", pot[hb], w[y], pot[y]));
                }
                continue;
            }
            if self.indep[0][y] {
                if pot[ha] - w[y] - pot[y] < 0 {
                    return Some(format!("A -> {y}: {} {} {}; set {set:?}", pot[ha], w[y], pot[y]));
                }
            } else {
                for x in ones(&self.circ[0][y * cw..(y + 1) * cw]) {
                    if pot[x] - w[y] - pot[y] < 0 {
                        return Some(format!("{x} -> {y}: {} {} {}; set {set:?}", pot[x], w[y], pot[y]));
                    }
                }
            }
            if self.indep[1][y] {
                if pot[y] - pot[hb] < 0 {
                    return Some(format!("{y} -> B: {} {}", pot[y], pot[hb]));
                }
            } else {
                for x in ones(&self.circ[1][y * cw..(y + 1) * cw]) {
                    if pot[y] + w[x] - pot[x] < 0 {
                        return Some(format!("{y} -> {x}: {} {} {}; set {set:?}", pot[y], w[x], pot[x]));
                    }
                }
            }
        }
        None
    }

    /// One augmentation along a shortest path if it gains weight; false if none does.
    fn augment(&mut self) -> bool {
        let w = &self.pair.weight;
        let sources: Vec<(u32, i64)> = self
            .cands
            .iter()
            .filter(|&&y| !self.in_set[y as usize] && self.indep[0][y as usize])
            .map(|&y| (y, -w[y as usize]))
            .collect();
        let pi_t = self
            .cands
            .iter()
            .filter(|&&y| !self.in_set[y as usize] && self.indep[1][y as usize])
            .map(|&y| self.pot[y as usize])
            .min();
        let (Some(pi_t), false) = (pi_t, sources.is_empty()) else { return false };
        let Search { pi_s, negative, best, last, .. } = self.dijkstra(&sources, Some(pi_t), true, false, false);
        self.stats.negative += negative;
        if last == NONE || best.0 - pi_s + pi_t >= 0 {
            return false;
        }
        let flipped = self.flip(best.0, last, true);
        assert!(flipped, "an augmenting path keeps the set independent in both matroids");
        self.set_hubs();
        true
    }

    /// One improving cycle through the root (see the type's comment), if any.
    fn improve(&mut self) -> Step {
        let w = &self.pair.weight;
        let sources: Vec<(u32, i64)> = self
            .cands
            .iter()
            .filter(|&&e| self.in_set[e as usize] || self.indep[0][e as usize])
            .map(|&e| (e, if self.in_set[e as usize] { w[e as usize] } else { -w[e as usize] }))
            .collect();
        let pi_t = self
            .cands
            .iter()
            .filter(|&&e| self.in_set[e as usize] || self.indep[1][e as usize])
            .map(|&e| self.pot[e as usize])
            .min();
        let Some(pi_t) = pi_t else { return Step::Optimal };
        let Search { pi_s, best, last, overrun, .. } = self.dijkstra(&sources, Some(pi_t), false, true, true);
        if overrun {
            return Step::Negative;
        }
        if last == NONE || best.0 - pi_s + pi_t >= 0 {
            return Step::Optimal;
        }
        if self.flip(best.0, last, false) {
            Step::Improved
        } else {
            Step::Dependent
        }
    }

    /// Updates the potentials by the last Dijkstra's distances (capped at `best`) and exchanges the elements of the
    /// path ending at `last`: the members on it leave, the others enter.  The weight split w1 = π on the set and
    /// π + w off it is what carries over, so an element changing sides changes its potential by its weight.
    /// False if an entering element turns out dependent (then the state is left broken).
    fn flip(&mut self, best: i64, last: u32, trusted: bool) -> bool {
        let w = self.pair.weight.clone();
        let len = self.pair.len();
        for i in 0..self.cands.len() {
            let e = self.cands[i] as usize;
            self.pot[e] += self.dist[e].0.min(best);
        }
        for hub in [len, len + 1] {
            self.pot[hub] += self.dist[hub].0.min(best);
        }
        let (mut entering, mut leaving) = (Vec::new(), Vec::new());
        let mut v = last;
        while v != NONE {
            let e = v as usize;
            if e < len {
                if self.in_set[e] {
                    leaving.push(e);
                } else {
                    entering.push(e);
                }
            }
            v = self.prev[e];
        }
        for &x in &leaving {
            self.take_out(x);
            self.in_set[x] = false;
            self.pot[x] -= w[x];
        }
        for &y in &entering {
            if !trusted && !(0..2).all(|q| self.reduce(q, y)) {
                return false;
            }
            self.add(y, true);
            self.in_set[y] = true;
            self.pot[y] += w[y];
        }
        true
    }

    /// The hubs' potentials from the members': hub A at the least w1 of a member, hub B at minus the least w2.
    fn set_hubs(&mut self) {
        let len = self.pair.len();
        let w = &self.pair.weight;
        let members = self.cands.iter().map(|&e| e as usize).filter(|&e| self.in_set[e]);
        let (a, b) = members.fold((INF, -INF), |(a, b), x| (a.min(self.pot[x]), b.max(self.pot[x] - w[x])));
        if a < INF {
            self.pot[len] = a;
            self.pot[len + 1] = b;
        }
    }

    /// The bound of the current set if its certificate holds: a weight split from the distances of every vertex
    /// from a source joined to each y with I + y independent in class 0 (cost −w(y)) and to each member x (cost
    /// w(x)), over the circuit arcs, checked against conditions (1)-(3) of the module comment in both classes.
    /// (The distances are the largest potentials meeting the arcs, so if any split satisfies (1)-(3) these do.)
    fn finish(&mut self, forced_weight: i64, forced: &[u32], loops: &[u32]) -> Option<Bound> {
        let w = self.pair.weight.clone();
        let cw = self.cw;
        let sources: Vec<(u32, i64)> = self
            .cands
            .iter()
            .filter(|&&e| self.in_set[e as usize] || self.indep[0][e as usize])
            .map(|&e| (e, if self.in_set[e as usize] { w[e as usize] } else { -w[e as usize] }))
            .collect();
        let Search { pi_s, overrun, .. } = self.dijkstra(&sources, None, false, false, true);
        if overrun {
            return None;
        }
        let len = self.pair.len();
        // w1 = σ on the set, σ + w off it, σ the true distance; w2 = w − w1.
        let mut w1 = vec![0i64; len];
        for &e in &self.cands {
            let e = e as usize;
            if self.dist[e].0 >= INF {
                return None;
            }
            // A reduced distance is the true one plus π_s − π(e).
            let sigma = self.dist[e].0 - pi_s + self.pot[e];
            w1[e] = if self.in_set[e] { sigma } else { sigma + w[e] };
        }
        let w2 = |e: usize, w1: &[i64]| w[e] - w1[e];
        let mut best = [vec![0i64; len], vec![0i64; len]];
        let mut ok = true;
        for &e in &self.cands {
            let e = e as usize;
            if self.in_set[e] {
                ok &= w1[e] >= 0 && w2(e, &w1) >= 0;
                continue;
            }
            for (q, best_q) in best.iter_mut().enumerate() {
                let wq = if q == 0 { w1[e] } else { w2(e, &w1) };
                if self.indep[q][e] {
                    ok &= wq <= 0;
                } else {
                    for x in ones(&self.circ[q][e * cw..(e + 1) * cw]) {
                        let wx = if q == 0 { w1[x] } else { w2(x, &w1) };
                        ok &= wq <= wx;
                        best_q[x] = best_q[x].max(wq);
                    }
                }
            }
        }
        if !ok {
            return None;
        }
        let members: Vec<u32> = self.cands.iter().copied().filter(|&e| self.in_set[e as usize]).collect();
        let value = forced_weight + members.iter().map(|&e| w[e as usize]).sum::<i64>();
        let without = members
            .iter()
            .map(|&x| {
                let x = x as usize;
                let drop = (w1[x] - best[0][x]) + (w2(x, &w1) - best[1][x]);
                (x as u32, value - drop)
            })
            .collect();
        let mut set: Vec<u32> = forced.iter().chain(&members).copied().collect();
        set.sort_unstable();
        let split = self.cands.iter().map(|&e| (e, w1[e as usize])).collect();
        Some(Bound { value, set, loops: loops.to_vec(), without, warm: Warm { members, w1: split } })
    }
}

#[cfg(test)]
#[allow(clippy::needless_range_loop)]
mod tests {
    use super::*;
    use crate::primes::smallest_prime_factors;

    struct Rng(u64);

    impl Rng {
        fn below(&mut self, n: u64) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 % n
        }
    }

    /// Whether the elements `set` are independent in class `q`, by plain elimination (an element with an empty list
    /// gets a coordinate of its own).
    fn independent(pair: &Pair, q: usize, set: &[usize]) -> bool {
        let dim = pair.dims[q] + set.len();
        let mut basis: Vec<Vec<bool>> = Vec::new();
        for (i, &e) in set.iter().enumerate() {
            let mut v = vec![false; dim];
            if pair.coords[q][e].is_empty() {
                v[pair.dims[q] + i] = true;
            }
            for &k in &pair.coords[q][e] {
                v[k as usize] ^= true;
            }
            for b in &basis {
                let p = b.iter().position(|&x| x).unwrap();
                if v[p] {
                    v.iter_mut().zip(b).for_each(|(a, &x)| *a ^= x);
                }
            }
            if !v.iter().any(|&x| x) {
                return false;
            }
            basis.push(v);
        }
        true
    }

    /// The heaviest weight of a set independent in both classes holding every `Pick` and no `Barred` element, by
    /// enumeration (None if there is none).
    fn brute(pair: &Pair, roles: &[Role]) -> Option<i64> {
        let len = pair.len();
        let mut best = None;
        for bits in 0u64..(1 << len) {
            let set: Vec<usize> = (0..len).filter(|&e| bits >> e & 1 == 1).collect();
            if set.iter().any(|&e| roles[e] == Role::Barred)
                || (0..len).any(|e| roles[e] == Role::Pick && bits >> e & 1 == 0)
            {
                continue;
            }
            if independent(pair, 0, &set) && independent(pair, 1, &set) {
                let value: i64 = set.iter().map(|&e| pair.weight[e]).sum();
                best = Some(best.map_or(value, |b: i64| b.max(value)));
            }
        }
        best
    }

    fn check(pair: &Pair, roles: &[Role], meet: &mut Meet) {
        let want = brute(pair, roles);
        let got = match meet.solve(roles) {
            Outcome::Bound(b) => Some(b),
            Outcome::Infeasible => None,
            Outcome::Uncertified => {
                let set: Vec<usize> = (0..pair.len()).filter(|&e| meet.in_set[e]).collect();
                panic!(
                    "uncertified: want {want:?}, the set {set:?}; negative arcs {}; roles {roles:?}\n\
                     dims {:?} coords {:?} weights {:?}",
                    meet.stats.negative, pair.dims, pair.coords, pair.weight
                );
            }
        };
        assert_eq!(meet.stats.negative, 0);
        assert_eq!(got.as_ref().map(|b| b.value), want, "roles {roles:?}");
        let Some(b) = got else { return };
        let set: Vec<usize> = b.set.iter().map(|&e| e as usize).collect();
        assert!(independent(pair, 0, &set) && independent(pair, 1, &set));
        assert_eq!(set.iter().map(|&e| pair.weight[e]).sum::<i64>(), b.value);
        for &l in &b.loops {
            let mut with = roles.to_vec();
            with[l as usize] = Role::Pick;
            assert_eq!(brute(pair, &with), None, "loop {l}");
        }
        for &(x, bound) in &b.without {
            let mut barred = roles.to_vec();
            assert_eq!(barred[x as usize], Role::Free);
            barred[x as usize] = Role::Barred;
            let truth = brute(pair, &barred).unwrap_or(i64::MIN);
            assert!(bound >= truth, "barring {x}: {bound} < {truth}");
        }
    }

    /// The certified intersection against enumeration on random pairs of small families of vectors (sparse vectors,
    /// some elements with a coordinate of their own, some repeated vectors), with random weights and roles.
    #[test]
    fn intersection_against_brute_force() {
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        for round in 0..3000 {
            let len = 2 + rng.below(10) as usize;
            let dims = [1 + rng.below(6) as usize, 1 + rng.below(6) as usize];
            let mut coords = [vec![Vec::new(); len], vec![Vec::new(); len]];
            for q in 0..2 {
                for e in 0..len {
                    if rng.below(8) == 0 {
                        continue; // an element with a coordinate of its own
                    }
                    let k = 1 + rng.below(3);
                    let mut c: Vec<u32> = (0..k).map(|_| rng.below(dims[q] as u64) as u32).collect();
                    c.sort_unstable();
                    c.dedup();
                    coords[q][e] = c;
                }
            }
            let weight: Vec<i64> = (0..len)
                .map(|_| if round % 3 == 0 { 1 + rng.below(4) as i64 } else { 1 + rng.below(100) as i64 })
                .collect();
            let pair = Pair { dims, coords, weight };
            let mut meet = Meet::new(pair.clone());
            for _ in 0..4 {
                let roles: Vec<Role> = (0..len)
                    .map(|_| match rng.below(7) {
                        0 => Role::Pick,
                        1 => Role::Barred,
                        _ => Role::Free,
                    })
                    .collect();
                check(&pair, &roles, &mut meet);
            }
            check(&pair, &vec![Role::Free; len], &mut meet);
        }
    }

    /// A solve started from a parent's certificate equals enumeration: children of random parents with members of
    /// the parent's set forced or barred, other elements barred, and now and then a non-member forced (which the
    /// warm start does not expect, so it may fall back).  Most children must not fall back.
    #[test]
    fn warm_starts_against_brute_force() {
        let mut rng = Rng(0xD1B5_4A32_D192_ED03);
        let (mut warm, mut fallbacks, mut expected_fallbacks) = (0u64, 0u64, 0u64);
        let mut reasons = [0u64; 4];
        for round in 0..2000 {
            let len = 3 + rng.below(9) as usize;
            let dims = [1 + rng.below(6) as usize, 1 + rng.below(6) as usize];
            let mut coords = [vec![Vec::new(); len], vec![Vec::new(); len]];
            for q in 0..2 {
                for e in 0..len {
                    if rng.below(8) == 0 {
                        continue;
                    }
                    let k = 1 + rng.below(3);
                    let mut c: Vec<u32> = (0..k).map(|_| rng.below(dims[q] as u64) as u32).collect();
                    c.sort_unstable();
                    c.dedup();
                    coords[q][e] = c;
                }
            }
            let weight: Vec<i64> = (0..len)
                .map(|_| if round % 3 == 0 { 1 + rng.below(4) as i64 } else { 1 + rng.below(100) as i64 })
                .collect();
            let pair = Pair { dims, coords, weight };
            let mut meet = Meet::new(pair.clone());
            let roles: Vec<Role> =
                (0..len).map(|_| if rng.below(6) == 0 { Role::Barred } else { Role::Free }).collect();
            let Outcome::Bound(parent) = meet.solve(&roles) else { continue };
            for _ in 0..4 {
                let mut child = roles.clone();
                for &x in &parent.warm.members {
                    match rng.below(5) {
                        0 => child[x as usize] = Role::Pick,
                        1 => child[x as usize] = Role::Barred,
                        _ => {}
                    }
                }
                for e in 0..len {
                    if child[e] == Role::Free && !parent.set.contains(&(e as u32)) {
                        match rng.below(12) {
                            0 => child[e] = Role::Pick,
                            1 | 2 => child[e] = Role::Barred,
                            _ => {}
                        }
                    }
                }
                let before = meet.stats;
                let want = brute(&pair, &child);
                let got = match meet.solve_from(&child, Some(&parent.warm)) {
                    Outcome::Bound(b) => Some(b),
                    Outcome::Infeasible => None,
                    Outcome::Uncertified => panic!("uncertified child {child:?} of {roles:?}"),
                };
                assert_eq!(got.as_ref().map(|b| b.value), want, "child {child:?} of {roles:?}, pair {:?}", pair.coords);
                if let Some(b) = got {
                    let set: Vec<usize> = b.set.iter().map(|&e| e as usize).collect();
                    assert!(independent(&pair, 0, &set) && independent(&pair, 1, &set));
                    for &(x, bound) in &b.without {
                        let mut barred = child.clone();
                        barred[x as usize] = Role::Barred;
                        assert!(bound >= brute(&pair, &barred).unwrap_or(i64::MIN), "barring {x}");
                    }
                }
                warm += meet.stats.warm - before.warm;
                fallbacks += meet.stats.fallbacks - before.fallbacks;
                let expected = (0..len).all(|e| child[e] != Role::Pick || parent.set.contains(&(e as u32)));
                if expected {
                    expected_fallbacks += meet.stats.fallbacks - before.fallbacks;
                    for (r, (a, b)) in
                        reasons.iter_mut().zip(meet.stats.fallback_reasons.iter().zip(before.fallback_reasons))
                    {
                        *r += a - b;
                    }
                }
            }
        }
        assert!(warm > 3000 && fallbacks * 10 < warm, "{fallbacks} fallbacks in {warm} warm starts");
        eprintln!(
            "warm starts: {warm}, fallbacks {fallbacks}, of them with only members forced {expected_fallbacks}; \
             reasons {:?}",
            reasons
        );
    }

    /// The parity bound (W1, docs/theory/parity-bound.md) on small boards: every playable set of 2..=n is independent
    /// at both parities, so the heaviest set independent at both is at least opt(n); and with random roles the bound
    /// is at least the heaviest playable set with those roles, and the intersection is exact against enumeration.
    #[test]
    fn playable_sets_are_independent_in_both_parity_matroids() {
        let spf = smallest_prime_factors(64);
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for n in 2..=18u32 {
            let board = Board::new(n, &spf);
            let weights: Vec<i64> = (0..=i64::from(n)).collect();
            let pair = Parity::pair(&board, &weights);
            let mut meet = Meet::new(pair.clone());
            let mut playable_sets: Vec<Vec<usize>> = Vec::new();
            for bits in 0u64..(1 << (n - 1)) {
                let set: Vec<usize> = (2..=n as usize).filter(|&c| bits >> (c - 2) & 1 == 1).collect();
                let mut pick = vec![false; n as usize + 1];
                set.iter().for_each(|&c| pick[c] = true);
                if board.playable(&pick) {
                    assert!(independent(&pair, 0, &set) && independent(&pair, 1, &set), "n={n} {set:?}");
                    playable_sets.push(set);
                }
            }
            let mut roles = vec![Role::Free; n as usize + 1];
            roles[0] = Role::Barred;
            roles[1] = Role::Barred;
            for trial in 0..6 {
                if trial > 0 {
                    for c in 2..=n as usize {
                        roles[c] = match rng.below(6) {
                            0 => Role::Pick,
                            1 => Role::Barred,
                            _ => Role::Free,
                        };
                    }
                }
                let best_playable = playable_sets
                    .iter()
                    .filter(|s| {
                        s.iter().all(|&c| roles[c] != Role::Barred)
                            && (2..=n as usize).all(|c| roles[c] != Role::Pick || s.contains(&c))
                    })
                    .map(|s| s.iter().map(|&c| c as i64).sum::<i64>())
                    .max();
                let got = meet.solve(&roles);
                if let Some(best) = best_playable {
                    assert!(matches!(&got, Outcome::Bound(b) if b.value >= best), "n={n} roles={roles:?}");
                }
                if n <= 13 {
                    check(&pair, &roles, &mut meet);
                }
            }
        }
    }
}
