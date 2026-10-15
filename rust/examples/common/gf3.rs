//! GF(3) linear algebra and the signed frame matroids of a state: the GF(3) twin of `FrameMatroids` in
//! `common/frames.rs`, for the fields study `examples/fields.rs`.  Included with `#[path = "common/gf3.rs"] mod gf3;`.
//!
//! # The signed frame matroids
//!
//! Frame `h` has the vector space over GF(3) whose coordinates are its factors.  A mover `u` of the frame (one of its
//! moves, or a candidate of home `h + 1`) has the vector `v(u) = Σ σ(u, f) e_f` over its maximal factors `f` among
//! the frame's factors, where the signing `σ` values each incidence `(u, f)` as 1 or 2.  A candidate `y` of home `h`
//! that is a factor of the frame has the unit vector `e_y`.  The moves' vectors are independent for every signing
//! (THEORY.md, Theorem 14; [`Matroid3::build`] panics if they are not), so contracting them, that is, taking
//! each candidate's vector modulo their span, gives a linear matroid on the candidates that touch the frame: `N_h^σ`,
//! the [`Matroid3`] of the frame, of rank the frame's factors minus its moves.  Its loops are the candidates whose
//! class is zero.
//!
//! With every value 1 the vectors are those of `common/frames.rs` read over GF(3) instead of GF(2).

use std::cmp::Reverse;
use std::collections::HashMap;
use taxman::model::{Frame, FrameGraph, FrameSet};

/// The marker of a number that has no coordinate, and of a coordinate that has no quotient coordinate.
const NONE: u32 = u32::MAX;

/// The words that hold `dim` coordinates of a [`V3`], at least one.
pub fn words_for(dim: usize) -> usize {
    dim.div_ceil(64).max(1)
}

// ---------------------------------------------------------------------------------------------------------
// Vectors

/// A GF(3) vector, bit-sliced: bit i of `p` set where coordinate i is 1, of `m` where it is 2.  Never both.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct V3 {
    /// The coordinates equal to 1.
    pub p: Vec<u64>,
    /// The coordinates equal to 2.
    pub m: Vec<u64>,
}

impl V3 {
    /// The zero vector of `words` words.
    pub fn zero(words: usize) -> Self {
        V3 { p: vec![0; words], m: vec![0; words] }
    }

    /// The number of words, so of coordinates 64 times that.
    pub fn words(&self) -> usize {
        self.p.len()
    }

    /// Coordinate `i`: 0, 1 or 2.
    pub fn get(&self, i: usize) -> u8 {
        let bit = 1u64 << (i % 64);
        if self.p[i / 64] & bit != 0 {
            1
        } else if self.m[i / 64] & bit != 0 {
            2
        } else {
            0
        }
    }

    /// Sets coordinate `i` to `v`, which is 0, 1 or 2 (a larger `v` panics: it is not reduced mod 3).
    pub fn set(&mut self, i: usize, v: u8) {
        assert!(v <= 2, "{v} is not an element of GF(3)");
        let bit = 1u64 << (i % 64);
        self.p[i / 64] &= !bit;
        self.m[i / 64] &= !bit;
        match v {
            1 => self.p[i / 64] |= bit,
            2 => self.m[i / 64] |= bit,
            _ => {}
        }
    }

    /// Whether every coordinate is 0.
    pub fn is_zero(&self) -> bool {
        self.p.iter().chain(&self.m).all(|&w| w == 0)
    }

    /// The lowest nonzero coordinate.
    pub fn lowest(&self) -> Option<usize> {
        self.lowest_from(0)
    }

    /// The lowest nonzero coordinate that is at least `from`.
    pub fn lowest_from(&self, from: usize) -> Option<usize> {
        let first = from / 64;
        (first..self.words()).find_map(|w| {
            let nonzero = self.p[w] | self.m[w];
            let nonzero = if w == first { nonzero & (u64::MAX << (from % 64)) } else { nonzero };
            (nonzero != 0).then(|| 64 * w + nonzero.trailing_zeros() as usize)
        })
    }

    /// The nonzero coordinates, ascending.
    pub fn support(&self) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(self.lowest(), |&c| self.lowest_from(c + 1))
    }

    /// `self += b`.
    pub fn add_assign(&mut self, b: &V3) {
        self.add_slices(&b.p, &b.m);
    }

    /// `self -= b`, that is, `self += (−b)`, and negation swaps the slices.
    pub fn sub_assign(&mut self, b: &V3) {
        self.add_slices(&b.m, &b.p);
    }

    /// `self = −self`.
    pub fn neg(&mut self) {
        std::mem::swap(&mut self.p, &mut self.m);
    }

    /// `self += c·b` for `c` in {0, 1, 2}.
    pub fn add_scaled(&mut self, b: &V3, c: u8) {
        match c {
            0 => {}
            1 => self.add_assign(b),
            2 => self.sub_assign(b),
            _ => panic!("{c} is not an element of GF(3)"),
        }
    }

    /// `self += b` for `b` given as the slices of its 1s and of its 2s.  A coordinate of the sum is 1 where one term
    /// is 1 and the other 0, or both are 2, and is 2 where one is 2 and the other 0, or both are 1.
    fn add_slices(&mut self, ones: &[u64], twos: &[u64]) {
        assert_eq!(self.words(), ones.len(), "vectors of different lengths");
        for ((ap, am), (&bp, &bm)) in self.p.iter_mut().zip(&mut self.m).zip(ones.iter().zip(twos)) {
            let (an, bn) = (*ap | *am, bp | bm);
            let (cp, cm) = ((*ap & !bn) | (bp & !an) | (*am & bm), (*am & !bn) | (bm & !an) | (*ap & bp));
            (*ap, *am) = (cp, cm);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Bases, determinants and ranks

/// An echelon basis over GF(3): rows by pivot, a row's pivot its lowest nonzero coordinate, normalized to 1.
#[derive(Clone, Debug)]
pub struct Basis3 {
    rows: Vec<Option<V3>>,
    words: usize,
    rank: usize,
}

impl Basis3 {
    /// The empty basis of the vectors with coordinates `0..dim`.
    pub fn new(dim: usize) -> Self {
        Basis3 { rows: vec![None; dim], words: words_for(dim), rank: 0 }
    }

    /// Reduces `v` in place until no pivot coordinate of the basis is nonzero in it: scan its nonzero coordinates
    /// from the lowest up; at a coordinate c that is a pivot, subtract v[c]·row_c (row_c has nothing below c, so
    /// only coordinates above c change), then continue above c.  Returns true iff the residue is nonzero (v is
    /// independent of the rows).  The residue is the unique representative of v modulo the rows' span that is zero
    /// on every pivot coordinate.
    pub fn reduce(&self, v: &mut V3) -> bool {
        assert_eq!(v.words(), self.words, "a vector of another length than the basis's");
        let mut from = 0;
        while let Some(c) = v.lowest_from(from) {
            if let Some(row) = &self.rows[c] {
                v.add_scaled(row, 3 - v.get(c));
            }
            from = c + 1;
        }
        !v.is_zero()
    }

    /// Inserts a nonzero residue from [`reduce`](Self::reduce) (its lowest coordinate is not a pivot): negate it if
    /// that entry is 2, store it as the row of that pivot.  Panics on a zero vector or an occupied pivot.
    pub fn insert(&mut self, mut v: V3) {
        assert_eq!(v.words(), self.words, "a vector of another length than the basis's");
        let c = v.lowest().expect("a zero vector has no pivot");
        assert!(self.rows[c].is_none(), "coordinate {c} is already a pivot");
        if v.get(c) == 2 {
            v.neg();
        }
        self.rows[c] = Some(v);
        self.rank += 1;
    }

    /// Reduces `v` and inserts the residue if it is nonzero.  Returns whether `v` was independent of the rows.
    pub fn try_insert(&mut self, mut v: V3) -> bool {
        let independent = self.reduce(&mut v);
        if independent {
            self.insert(v);
        }
        independent
    }

    /// Whether `c` is the pivot of a row.
    pub fn is_pivot(&self, c: usize) -> bool {
        self.rows[c].is_some()
    }

    /// The number of rows.
    pub fn rank(&self) -> usize {
        self.rank
    }

    /// The number of coordinates.
    pub fn dim(&self) -> usize {
        self.rows.len()
    }

    /// Removes every row.
    pub fn clear(&mut self) {
        self.rows.fill(None);
        self.rank = 0;
    }
}

/// The determinant mod 3 of the square matrix whose rows are `rows` (each with at least `rows.len()` coordinates;
/// coordinates 0..rows.len() are the columns), by Gaussian elimination over GF(3) tracking pivots and row swaps.
pub fn det3(rows: &[V3]) -> u8 {
    let n = rows.len();
    assert!(rows.iter().all(|r| 64 * r.words() >= n), "a row has fewer coordinates than there are rows");
    let mut m = rows.to_vec();
    let mut det = 1;
    for c in 0..n {
        let Some(r) = (c..n).find(|&r| m[r].get(c) != 0) else { return 0 };
        if r != c {
            m.swap(r, c);
            det = det * 2 % 3;
        }
        // 1 and 2 are their own inverses in GF(3), so a row with a in column c loses `a·pivot` times the pivot row.
        let (done, rest) = m.split_at_mut(c + 1);
        let pivot = done[c].get(c);
        det = det * pivot % 3;
        for row in rest {
            let a = row.get(c);
            if a != 0 {
                row.add_scaled(&done[c], 3 - a * pivot % 3);
            }
        }
    }
    det
}

/// The rank over GF(3) of a family of vectors with coordinates `0..dim`.
pub fn rank3(vecs: &[V3], dim: usize) -> usize {
    let mut basis = Basis3::new(dim);
    for v in vecs {
        basis.try_insert(v.clone());
    }
    basis.rank()
}

// ---------------------------------------------------------------------------------------------------------
// Signings

/// How a signing values the incidences no core fixed: all 1, or a seeded hash of (seed, u, f) into {1, 2}.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Default3 {
    /// Every value is 1.
    Ones,
    /// Every value is 1 or 2, by a hash of the seed and the incidence.
    Random(u64),
}

/// The output of SplitMix64 that steps from the state `x`, which makes it a good hash of `x`.
fn splitmix64(x: u64) -> u64 {
    let z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    let z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A seed for a random signing, from the board, the frame, an index and a salt (distinct arms use distinct salts).
pub fn seed(n: u32, frame: usize, index: usize, salt: u64) -> u64 {
    [u64::from(n), frame as u64, index as u64, salt].into_iter().fold(0, |h, x| splitmix64(h ^ x))
}

/// The identifier of a core, assigned by the caller (an index into its list of cores).
pub type CoreId = usize;

/// A signing of one frame: values on incidences (mover, factor), a default for the others.
#[derive(Clone, Debug, PartialEq)]
pub struct Signing {
    /// The frame it signs.
    pub frame: usize,
    /// The values of the incidences that `fixed` does not hold.
    pub default: Default3,
    /// Values set by cores (1 or 2), keyed by (mover, factor).
    pub fixed: HashMap<(u32, u32), u8>,
    /// Cores assigned to it (empty for the plain and the random signings).
    pub cores: Vec<CoreId>,
    /// Whether cores may be merged into it (false for the plain and the random signings).
    pub mergeable: bool,
}

impl Signing {
    /// The signing of `frame` with every value 1.
    pub fn ones(frame: usize) -> Self {
        Signing { frame, default: Default3::Ones, fixed: HashMap::new(), cores: Vec::new(), mergeable: false }
    }

    /// The signing of `frame` with every value a hash of `seed`, the incidence, in {1, 2}.
    pub fn random(frame: usize, seed: u64) -> Self {
        Signing { default: Default3::Random(seed), ..Signing::ones(frame) }
    }

    /// A signing of `frame` without values yet and with a `default`, for cores to be merged into.
    pub fn for_cores(frame: usize, default: Default3) -> Self {
        Signing { default, mergeable: true, ..Signing::ones(frame) }
    }

    /// σ(u, f): the fixed value if any, else the default's.  A random default is 1 plus one bit of
    /// `splitmix64(seed ^ g(u, f))`, with `g(u, f) = splitmix64((u << 32) | f)`.
    pub fn value(&self, u: u32, f: u32) -> u8 {
        if let Some(&v) = self.fixed.get(&(u, f)) {
            return v;
        }
        match self.default {
            Default3::Ones => 1,
            Default3::Random(seed) => {
                let g = splitmix64((u64::from(u) << 32) | u64::from(f));
                1 + (splitmix64(seed ^ g) & 1) as u8
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The signed frame matroid of a frame

/// One frame's coordinates and the quotient of its space by the span of its moves.
struct Space {
    /// Coordinate of each number that is a factor of the frame (`NONE` for the others, and past the end).
    coord: Vec<u32>,
    words: usize,
    /// The moves' vectors, in echelon form.
    basis: Basis3,
    /// The quotient coordinate of each coordinate that is not a pivot (`NONE` for the pivots).
    quotient: Vec<u32>,
    rank: usize,
}

impl Space {
    /// The space of `frame`, the signing `s` valuing its moves' vectors.
    fn new<const W: usize>(model: &FrameGraph, frame: &Frame<W>, s: &Signing) -> Self {
        let dim = frame.factors.len() as usize;
        let mut coord = vec![NONE; frame.factors.max().map_or(0, |max| max as usize + 1)];
        for (k, y) in frame.factors.iter().enumerate() {
            coord[y as usize] = k as u32;
        }
        let basis = Basis3::new(dim);
        let mut space = Space { coord, words: words_for(dim), basis, quotient: Vec::new(), rank: 0 };
        for m in frame.moves.iter() {
            let v = space.vector(model, s, m);
            assert!(space.basis.try_insert(v), "the moves of frame {} are dependent over GF(3)", s.frame);
        }
        let mut rank = 0;
        space.quotient = (0..dim)
            .map(|c| {
                if space.basis.is_pivot(c) {
                    NONE
                } else {
                    rank += 1;
                    rank - 1
                }
            })
            .collect();
        space.rank = rank as usize;
        space
    }

    /// The vector of the mover `u`: the signing's values at its maximal factors among the frame's factors.
    fn vector(&self, model: &FrameGraph, s: &Signing, u: u32) -> V3 {
        let mut v = V3::zero(self.words);
        for &f in model.graph().factors(u) {
            if let Some(&c) = self.coord.get(usize::from(f)).filter(|&&c| c != NONE) {
                v.set(c as usize, s.value(u, u32::from(f)));
            }
        }
        v
    }

    /// The unit vector of the factor `y` of the frame.
    fn unit(&self, y: u32) -> V3 {
        let c = self.coord[y as usize];
        debug_assert_ne!(c, NONE, "{y} is not a factor of the frame");
        let mut v = V3::zero(self.words);
        v.set(c as usize, 1);
        v
    }

    /// `v` in the quotient: reduced by the moves' span, then the non-pivot coordinates, renumbered.
    fn project(&self, mut v: V3) -> V3 {
        self.basis.reduce(&mut v);
        let mut class = V3::zero(words_for(self.rank));
        for c in v.support() {
            debug_assert_ne!(self.quotient[c], NONE, "a pivot survived the reduction");
            class.set(self.quotient[c] as usize, v.get(c));
        }
        class
    }
}

/// The signed frame matroid N_h^σ of a state.
#[derive(Clone, Debug)]
pub struct Matroid3 {
    /// The frame it belongs to.
    pub frame: usize,
    /// The elements: candidate numbers, ascending (the candidates given that touch the frame and are not loops).
    pub elems: Vec<u32>,
    /// Their classes in the quotient (`rank` coordinates).
    pub vecs: Vec<V3>,
    /// The quotient's dimension: factors minus moves.
    pub rank: usize,
    /// The candidates given that touch the frame and whose class is zero, ascending.
    pub loops: Vec<u32>,
}

impl Matroid3 {
    /// Built from frame `s.frame` of `state` and the candidates `cands` (ascending numbers).  A candidate touches
    /// the frame `h` when its home is `h + 1`, or its home is `h` and it is a factor of the frame; the others are
    /// ignored.  The coordinates are the frame's factors, ascending.  Each move's vector is reduced and inserted in
    /// a basis, which panics if the moves are dependent; the quotient's coordinates are the coordinates that are not
    /// pivots, renumbered.  A candidate of home `h + 1` gets the class of its vector, a candidate of home `h` the
    /// class of its unit vector.
    pub fn build<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, cands: &[u32], s: &Signing) -> Self {
        let h = s.frame;
        let frame = &state.frames()[h];
        let space = Space::new(model, frame, s);
        let (mut elems, mut vecs, mut loops) = (Vec::new(), Vec::new(), Vec::new());
        for &x in cands {
            let home = model.home(x);
            let v = if home == h + 1 {
                space.vector(model, s, x)
            } else if home == h && frame.factors.contains(x) {
                space.unit(x)
            } else {
                continue;
            };
            let class = space.project(v);
            if class.is_zero() {
                loops.push(x);
            } else {
                elems.push(x);
                vecs.push(class);
            }
        }
        Matroid3 { frame: h, elems, vecs, rank: space.rank, loops }
    }

    /// A matroid from given vectors (for tests): elements `elems`, classes `vecs` of `rank` coordinates, no loops.
    /// A zero vector is allowed and is never kept by [`greedy`](Self::greedy), as a loop would not be.
    pub fn from_vectors(frame: usize, elems: Vec<u32>, vecs: Vec<V3>, rank: usize) -> Self {
        assert_eq!(elems.len(), vecs.len(), "an element without a vector");
        Matroid3 { frame, elems, vecs, rank, loops: Vec::new() }
    }

    /// The index in `elems` of the candidate `x`, if it is an element.
    pub fn index(&self, x: u32) -> Option<usize> {
        self.elems.binary_search(&x).ok()
    }

    /// max { μ(I) : I independent }, μ indexed like `elems`: the greedy algorithm over the elements of positive
    /// weight, by decreasing weight (ties: lower index first), keeping each element independent of those kept.
    /// Returns the value and the kept indices, ascending.
    pub fn greedy(&self, mu: &[i64]) -> (i64, Vec<usize>) {
        assert_eq!(mu.len(), self.vecs.len(), "a weight for every element");
        let mut order: Vec<usize> = (0..mu.len()).filter(|&i| mu[i] > 0).collect();
        order.sort_by_key(|&i| (Reverse(mu[i]), i));
        let mut basis = Basis3::new(self.rank);
        let (mut value, mut kept) = (0, Vec::new());
        for i in order {
            if basis.rank() == self.rank {
                break;
            }
            if basis.try_insert(self.vecs[i].clone()) {
                value += mu[i];
                kept.push(i);
            }
        }
        kept.sort_unstable();
        (value, kept)
    }

    /// Whether the members of `set` (candidate numbers) that touch the frame are independent: false if one is a
    /// loop; members that are neither elements nor loops are ignored.
    pub fn independent(&self, set: &[u32]) -> bool {
        let mut basis = Basis3::new(self.rank);
        set.iter().all(|&x| match self.index(x) {
            Some(i) => basis.try_insert(self.vecs[i].clone()),
            None => self.loops.binary_search(&x).is_err(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taxman::board::Board;
    use taxman::search::SearchTactics;

    /// A xorshift generator, seeded with any nonzero number.
    struct Rng(u64);

    impl Rng {
        fn below(&mut self, n: u64) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 % n
        }

        /// A percentage of zero coordinates for [`random_entries`]: none, some, half, most or all.
        fn zero_percent(&mut self) -> u64 {
            [0, 33, 50, 90, 100][self.below(5) as usize]
        }
    }

    /// `len` random coordinates, each 0 with probability `zero_percent` / 100 and else 1 or 2.
    fn random_entries(rng: &mut Rng, len: usize, zero_percent: u64) -> Vec<u8> {
        (0..len).map(|_| if rng.below(100) < zero_percent { 0 } else { 1 + rng.below(2) as u8 }).collect()
    }

    /// The vector with coordinates `entries` (in 0..3), in `words` words.
    fn to_v3(entries: &[u8], words: usize) -> V3 {
        let mut v = V3::zero(words);
        for (i, &e) in entries.iter().enumerate() {
            v.set(i, e);
        }
        v
    }

    /// The first `len` coordinates of `v`.
    fn to_entries(v: &V3, len: usize) -> Vec<u8> {
        (0..len).map(|i| v.get(i)).collect()
    }

    /// The coordinates `f(a[i], b[i])`.
    fn zip_with(a: &[u8], b: &[u8], f: impl Fn(u8, u8) -> u8) -> Vec<u8> {
        a.iter().zip(b).map(|(&x, &y)| f(x, y)).collect()
    }

    /// The rank over GF(3) of rows of equal lengths, by dense elimination.
    fn dense_rank(rows: &[Vec<u8>]) -> usize {
        let mut m = rows.to_vec();
        let width = m.first().map_or(0, Vec::len);
        let mut rank = 0;
        for c in 0..width {
            let Some(r) = (rank..m.len()).find(|&r| m[r][c] != 0) else { continue };
            m.swap(rank, r);
            let pivot = m[rank].clone();
            for row in &mut m[rank + 1..] {
                let f = row[c] * pivot[c] % 3;
                for (x, p) in row.iter_mut().zip(&pivot) {
                    *x = (*x + 3 - f * p % 3) % 3;
                }
            }
            rank += 1;
        }
        rank
    }

    /// The rank of `rows` with `extra` appended.
    fn dense_rank_with(rows: &[Vec<u8>], extra: &[u8]) -> usize {
        dense_rank(&[rows, &[extra.to_vec()]].concat())
    }

    #[test]
    fn v3_matches_scalar_arithmetic() {
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        for _ in 0..2_000 {
            let len = 1 + rng.below(200) as usize;
            let words = words_for(len);
            let (zeros_a, zeros_b) = (rng.zero_percent(), rng.zero_percent());
            let ea = random_entries(&mut rng, len, zeros_a);
            let eb = random_entries(&mut rng, len, zeros_b);
            let (a, b) = (to_v3(&ea, words), to_v3(&eb, words));
            assert_eq!(to_entries(&a, len), ea, "get after set");
            assert!(a.p.iter().zip(&a.m).all(|(p, m)| p & m == 0), "a coordinate is both 1 and 2");
            assert_eq!(a.is_zero(), ea.iter().all(|&x| x == 0));
            assert_eq!(a.lowest(), ea.iter().position(|&x| x != 0));
            assert_eq!(a.support().collect::<Vec<_>>(), (0..len).filter(|&i| ea[i] != 0).collect::<Vec<_>>());
            let from = rng.below(64 * words as u64 + 70) as usize;
            assert_eq!(a.lowest_from(from), (from..len).find(|&i| ea[i] != 0));

            let mut sum = a.clone();
            sum.add_assign(&b);
            assert_eq!(sum, to_v3(&zip_with(&ea, &eb, |x, y| (x + y) % 3), words), "add");
            let mut difference = a.clone();
            difference.sub_assign(&b);
            assert_eq!(difference, to_v3(&zip_with(&ea, &eb, |x, y| (x + 3 - y) % 3), words), "sub");
            let mut negated = a.clone();
            negated.neg();
            assert_eq!(negated, to_v3(&ea.iter().map(|&x| (3 - x) % 3).collect::<Vec<_>>(), words), "neg");
            for c in 0..3 {
                let mut scaled = a.clone();
                scaled.add_scaled(&b, c);
                assert_eq!(scaled, to_v3(&zip_with(&ea, &eb, |x, y| (x + c * y) % 3), words), "add_scaled by {c}");
            }
        }
    }

    #[test]
    #[should_panic(expected = "not an element of GF(3)")]
    fn v3_set_rejects_values_above_two() {
        V3::zero(1).set(0, 3);
    }

    #[test]
    #[should_panic(expected = "different lengths")]
    fn v3_add_rejects_vectors_of_different_lengths() {
        V3::zero(1).add_assign(&V3::zero(2));
    }

    #[test]
    fn basis3_matches_dense_elimination() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for _ in 0..1_000 {
            let dim = 1 + rng.below(130) as usize;
            let words = words_for(dim);
            let rows: Vec<Vec<u8>> = (0..1 + rng.below(40)).map(|_| random_entries(&mut rng, dim, 50)).collect();
            let mut basis = Basis3::new(dim);
            for row in &rows {
                basis.try_insert(to_v3(row, words));
            }
            assert_eq!(basis.dim(), dim);
            assert_eq!(basis.rank(), dense_rank(&rows), "rank");
            assert_eq!((0..dim).filter(|&c| basis.is_pivot(c)).count(), basis.rank(), "pivots");

            // A further vector: random, or a combination of the rows (in the span).
            let mut entries = random_entries(&mut rng, dim, 50);
            if rng.below(3) == 0 {
                entries = vec![0; dim];
                for row in &rows {
                    let c = rng.below(3) as u8;
                    entries = zip_with(&entries, row, |x, y| (x + c * y) % 3);
                }
            }
            let mut residue = to_v3(&entries, words);
            let outside = basis.reduce(&mut residue);
            assert_eq!(outside, dense_rank_with(&rows, &entries) > basis.rank(), "outside the span");
            assert_eq!(outside, !residue.is_zero());
            assert!((0..dim).all(|c| !basis.is_pivot(c) || residue.get(c) == 0), "a pivot survived");
            let difference = zip_with(&entries, &to_entries(&residue, dim), |x, y| (x + 3 - y) % 3);
            assert_eq!(dense_rank_with(&rows, &difference), basis.rank(), "v minus its residue is in the span");

            basis.clear();
            assert_eq!(basis.rank(), 0);
            assert!((0..dim).all(|c| !basis.is_pivot(c)));
            let mut again = to_v3(&entries, words);
            assert_eq!(basis.reduce(&mut again), entries.iter().any(|&x| x != 0), "reduce after clear");
        }
    }

    #[test]
    fn greedy_matches_brute_force() {
        let mut rng = Rng(0xD1B5_4A32_D192_ED03);
        for _ in 0..3_000 {
            let (count, dim) = (1 + rng.below(10) as usize, 1 + rng.below(6) as usize);
            let zeros = rng.zero_percent();
            let entries: Vec<Vec<u8>> = (0..count).map(|_| random_entries(&mut rng, dim, zeros)).collect();
            let vecs = entries.iter().map(|e| to_v3(e, words_for(dim))).collect();
            let m = Matroid3::from_vectors(0, (0..count as u32).collect(), vecs, dim);
            let mu: Vec<i64> = (0..count).map(|_| rng.below(26) as i64 - 5).collect();

            let independent = |set: &[usize]| {
                let rows: Vec<Vec<u8>> = set.iter().map(|&i| entries[i].clone()).collect();
                dense_rank(&rows) == set.len()
            };
            let weight = |set: &[usize]| set.iter().map(|&i| mu[i]).sum::<i64>();
            let best = (0..1u32 << count)
                .map(|mask| (0..count).filter(|&i| (mask >> i) & 1 == 1).collect::<Vec<_>>())
                .filter(|set| independent(set))
                .map(|set| weight(&set))
                .max()
                .expect("the empty set is independent");

            let (value, kept) = m.greedy(&mu);
            assert_eq!(value, best, "greedy value");
            assert!(independent(&kept), "greedy's set is independent");
            assert_eq!(weight(&kept), value, "greedy's set weighs its value");
            assert!(kept.windows(2).all(|w| w[0] < w[1]), "kept indices ascend");
            let numbers: Vec<u32> = kept.iter().map(|&i| m.elems[i]).collect();
            assert!(m.independent(&numbers));
        }
    }

    #[test]
    fn matroid3_independent_and_index() {
        // Elements 5, 7, 9 with classes e0, e1, e0 + e1 in dimension 2; 3 is a loop, 11 is not a candidate.
        let words = words_for(2);
        let vecs = vec![to_v3(&[1, 0], words), to_v3(&[0, 1], words), to_v3(&[1, 1], words)];
        let mut m = Matroid3::from_vectors(0, vec![5, 7, 9], vecs, 2);
        m.loops = vec![3];
        assert_eq!((m.index(7), m.index(8)), (Some(1), None));
        assert!(m.independent(&[]) && m.independent(&[5, 9]) && m.independent(&[7, 9, 11]));
        assert!(!m.independent(&[5, 7, 9]), "three vectors in dimension 2");
        assert!(!m.independent(&[3]) && !m.independent(&[5, 3]), "a loop");
        assert!(!m.independent(&[5, 5]), "a repeated element");
        assert_eq!(m.greedy(&[1, 5, 3]), (8, vec![1, 2]));
    }

    /// The integer determinant by the permutation expansion.
    fn integer_det(m: &[Vec<u8>]) -> i64 {
        fn expand(m: &[Vec<u8>], used: &mut Vec<usize>) -> i64 {
            let row = used.len();
            if row == m.len() {
                let inversions =
                    (0..row).flat_map(|i| (i + 1..row).map(move |j| (i, j))).filter(|&(i, j)| used[i] > used[j]);
                return if inversions.count() % 2 == 0 { 1 } else { -1 };
            }
            let mut total = 0;
            for c in 0..m.len() {
                if m[row][c] != 0 && !used.contains(&c) {
                    used.push(c);
                    total += i64::from(m[row][c]) * expand(m, used);
                    used.pop();
                }
            }
            total
        }
        expand(m, &mut Vec::new())
    }

    #[test]
    fn det3_matches_the_integer_determinant() {
        let mut rng = Rng(0x8CB9_2BA7_2F3D_8DD7);
        for _ in 0..2_000 {
            let n = 1 + rng.below(6) as usize;
            // Rows may be longer than the matrix is wide: the extra coordinates are not columns.
            let width = n + rng.below(70) as usize;
            let entries: Vec<Vec<u8>> = (0..n).map(|_| random_entries(&mut rng, width, 33)).collect();
            let rows: Vec<V3> = entries.iter().map(|e| to_v3(e, words_for(width))).collect();
            let square: Vec<Vec<u8>> = entries.iter().map(|e| e[..n].to_vec()).collect();
            assert_eq!(i64::from(det3(&rows)), integer_det(&square).rem_euclid(3), "{square:?}");
        }
    }

    #[test]
    fn det3_sees_the_core_that_gf2_misses() {
        // The rows 12, 18, 20, 30, 45 over the columns 4, 6, 9, 10, 15, each row listing its columns.
        let columns = [4, 6, 9, 10, 15];
        let rows = [(12, vec![4, 6]), (18, vec![6, 9]), (20, vec![4, 10]), (30, vec![6, 10, 15]), (45, vec![9, 15])];
        let matrix = |sign: &dyn Fn(usize, u32) -> u8| -> Vec<Vec<u8>> {
            rows.iter()
                .enumerate()
                .map(|(r, (_, listed))| {
                    columns.iter().map(|&f| if listed.contains(&f) { sign(r, f) } else { 0 }).collect()
                })
                .collect()
        };
        let vectors = |entries: &[Vec<u8>]| entries.iter().map(|e| to_v3(e, 1)).collect::<Vec<_>>();

        let ones = matrix(&|_, _| 1);
        assert_eq!(integer_det(&ones).abs(), 3);
        assert_eq!(det3(&vectors(&ones)), 0, "all ones");

        // Column 6 is listed three times, every other column twice: the lower-numbered lister gets 1, the other 2.
        let column_sum = matrix(&|r, f| {
            let listers: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].1.contains(&f)).collect();
            if listers.len() == 3 || listers[0] == r {
                1
            } else {
                2
            }
        });
        for c in 0..columns.len() {
            assert_eq!(column_sum.iter().map(|row| u32::from(row[c])).sum::<u32>() % 3, 0, "column {}", columns[c]);
        }
        assert!(rank3(&vectors(&column_sum), 5) < 5);
        assert_eq!(det3(&vectors(&column_sum)), 0);
    }

    #[test]
    fn splitmix64_is_the_published_generator() {
        // The first two outputs of SplitMix64 seeded with 0 (the state steps by the golden gamma before mixing).
        assert_eq!(splitmix64(0), 0xE220_A839_7B1D_CDAF);
        assert_eq!(splitmix64(0x9E37_79B9_7F4A_7C15), 0x6E78_9E6A_A1B9_65F4);
    }

    #[test]
    fn seeds_differ_with_every_argument() {
        let base = seed(120, 1, 2, 3);
        assert_eq!(base, seed(120, 1, 2, 3));
        for other in [seed(121, 1, 2, 3), seed(120, 2, 2, 3), seed(120, 1, 3, 3), seed(120, 1, 2, 4)] {
            assert_ne!(base, other);
        }
    }

    #[test]
    fn signing_values() {
        let incidences = (0..100u32).flat_map(|u| (0..100u32).map(move |f| (u, f)));
        let ones = Signing::ones(0);
        assert!(incidences.clone().all(|(u, f)| ones.value(u, f) == 1));

        let random = Signing::random(0, 42);
        let values: Vec<u8> = incidences.clone().map(|(u, f)| random.value(u, f)).collect();
        assert!(values.iter().all(|&v| v == 1 || v == 2));
        let twos = values.iter().filter(|&&v| v == 2).count();
        assert!((4_500..5_500).contains(&twos), "{twos} of 10000 values are 2");
        let again: Vec<u8> = incidences.clone().map(|(u, f)| Signing::random(0, 42).value(u, f)).collect();
        assert_eq!(values, again, "deterministic");
        let other: Vec<u8> = incidences.map(|(u, f)| Signing::random(0, 43).value(u, f)).collect();
        assert_ne!(values, other, "another seed, another signing");

        let (mut fixed_ones, mut fixed_random) = (Signing::ones(0), Signing::random(0, 42));
        for s in [&mut fixed_ones, &mut fixed_random] {
            for (u, f, v) in [(3, 4, 2), (5, 6, 1)] {
                s.fixed.insert((u, f), v);
            }
            assert_eq!((s.value(3, 4), s.value(5, 6)), (2, 1), "fixed overrides the default");
        }
        assert_eq!(fixed_ones.value(4, 3), 1);
        assert_eq!(fixed_random.value(4, 3), random.value(4, 3));

        assert!(!Signing::ones(0).mergeable && !Signing::random(0, 1).mergeable);
        let for_cores = Signing::for_cores(2, Default3::Random(7));
        assert!(for_cores.mergeable && for_cores.cores.is_empty() && for_cores.frame == 2);
        assert_eq!(for_cores.default, Default3::Random(7));
    }

    #[test]
    fn matroids_of_a_real_board_have_the_frames_freedoms() {
        for n in [120, 500] {
            let board = Board::<8>::for_n(n);
            let p = board.prepare(SearchTactics::default()).expect("prepare");
            let cands = p.candidates.to_vec();
            let promoted = p.greedy.to_vec();
            for (h, frame) in p.root.frames().iter().enumerate() {
                let freedoms = (frame.factors.len() - frame.moves.len()) as usize;
                let signings = (0..4).map(|i| Signing::random(h, seed(n, h, i, 0xF3))).chain([Signing::ones(h)]);
                for s in signings {
                    let m = Matroid3::build(&p.model, &p.root, &cands, &s);
                    assert_eq!(m.rank, freedoms, "n = {n}, frame {h} under {:?}", s.default);
                    assert_eq!(m.frame, h);
                    assert!(m.elems.windows(2).all(|w| w[0] < w[1]) && m.loops.windows(2).all(|w| w[0] < w[1]));
                    assert!(m.vecs.iter().all(|v| !v.is_zero() && v.words() == words_for(m.rank)), "classes");
                    let touching = cands.iter().filter(|&&x| {
                        let home = p.model.home(x);
                        home == h + 1 || (home == h && frame.factors.contains(x))
                    });
                    assert_eq!(touching.count(), m.elems.len() + m.loops.len(), "n = {n}, frame {h}: elements, loops");
                    assert!(m.independent(&promoted), "n = {n}, frame {h}: the greedy incumbent is promotable");
                }
            }
        }
    }
}
