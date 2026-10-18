//! Fixed-width bitsets of small non-negative integers: the type of every set of game numbers in the solver.
//!
//! `Bits<W>` is a set of `u32` values in `0..64·W`, stored inline as `[u64; W]`: it is `Copy`, never
//! allocates, and every operation is a short loop over the words that the optimizer can unroll and
//! vectorize.  The frames (as setup forms them and as the search takes them), the search state and the nogood tables
//! all hold their sets of numbers as `Bits`, which is why the whole solver is generic over the width `W`.
//!
//! # Choosing the width
//!
//! A board `1..=n` needs `W ≥ n/64 + 1` words ([`words_for`]).  The solver does not instantiate every `W`: it
//! has one copy of its code for each of the [`WIDTHS`], picks the narrowest that holds the board
//! ([`width_for`]), and dispatches to it with the macros [`with_width!`](crate::with_width) and
//! [`with_words!`](crate::with_words), which turn the run-time `n` into a compile-time constant `W`.  A set
//! operation costs a loop over all `W` words, so a narrower set is a faster search.  The boards each width serves
//! (`n ≤ 64·W − 1`):
//!
//! * `W = 8`: up to 511; `W = 16`: 512 to 1023;
//! * `W = 20`: 1024 to 1279; `W = 24`: 1280 to 1535; `W = 28`: 1536 to 1791; `W = 32`: 1792 to 2047;
//! * `W = 36`: 2048 to 2303; `W = 40`: 2304 to 2559; `W = 48`: 2560 to 3071; `W = 56`: 3072 to 3583;
//! * `W = 64`: 3584 to 4095; `W = 128`: 4096 to 8191, the largest board the solver supports.
//!
//! # Out-of-range values
//!
//! A value the set cannot hold is a logic error.  `insert` and `remove` check with `debug_assert!` and
//! otherwise rely on the array bounds check (so a bad `insert` can never corrupt memory); `from_range` checks its
//! end with `debug_assert!` and in a release build silently drops the part of the range beyond the capacity.
//! `contains`, [`max_below`](Bits::max_below) and [`min_above`](Bits::min_above) are the exceptions: asked about a
//! value beyond the capacity they just answer (`false`, or the nearest member).

use std::fmt;
use std::iter::FusedIterator;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Sub, SubAssign};

/// Number of `u64` words needed so that every value in `0..=n` fits: the smallest `W` with `64·W > n`.
pub const fn words_for(n: u32) -> usize {
    (n >> 6) as usize + 1
}

/// The set widths the solver is built for, ascending: each is a separate copy of the search's code, and a board
/// gets the narrowest that holds it ([`width_for`]), since every set operation costs a loop over all its words.
/// Widths of 8 and 16 words, then steps of 4 words from 20 to 32 (boards 1024 to 2047, where the games the
/// solver works on today are), 36, 40, 48 and 56 (boards 2048 to 3583, where the chain goes next), 64 to board 4095,
/// and 128 beyond, to board 8191 (no width between 64 and 128: past 4095 the width costs a node about 1% and the
/// board layer little, solver/notebook/boards-past-4095-2026-10/README.md).  The arms of
/// [`with_words!`](crate::with_words) repeat these numbers, because a macro arm needs literals; the test
/// `the_width_dispatch_follows_the_widths` fails if they drift apart.
pub const WIDTHS: [usize; 12] = [8, 16, 20, 24, 28, 32, 36, 40, 48, 56, 64, 128];

/// The narrowest of [`WIDTHS`] with at least `words` words, if any.
pub const fn width_for(words: usize) -> Option<usize> {
    let mut i = 0;
    while i < WIDTHS.len() {
        if WIDTHS[i] >= words {
            return Some(WIDTHS[i]);
        }
        i += 1;
    }
    None
}

/// Runs `$body` with the const `$w` set to the set width for the board `1..=$n`, `width_for(words_for($n))`, and
/// panics for a board no width holds; with `else $beyond`, evaluates that instead.
///
/// ```text
/// let words = taxman::with_width!(1215, W => W);
/// assert_eq!(words, 20);
/// ```
///
/// (Not a doc-test: rustdoc links a doc-test with the crate's fat LTO, 7 s in a release build for these three lines.
/// The unit test `the_width_dispatch_follows_the_widths` checks that the board 1215 gets 20 words.)
#[macro_export]
macro_rules! with_width {
    ($n:expr, $w:ident => $body:expr) => {{
        let n: u32 = $n;
        $crate::with_words!($crate::bits::words_for(n), $w => $body, else panic!("n={n} is beyond the widest set"))
    }};
    ($n:expr, $w:ident => $body:expr, else $beyond:expr) => {
        $crate::with_words!($crate::bits::words_for($n), $w => $body, else $beyond)
    };
}

/// [`with_width!`](crate::with_width) for sets of at least `$words` words: `$w` is `width_for($words)`, and
/// `$beyond` is evaluated when there is none.
#[macro_export]
macro_rules! with_words {
    ($words:expr, $w:ident => $body:expr, else $beyond:expr) => {
        match $words {
            0..=8 => {
                const $w: usize = 8;
                $body
            }
            9..=16 => {
                const $w: usize = 16;
                $body
            }
            17..=20 => {
                const $w: usize = 20;
                $body
            }
            21..=24 => {
                const $w: usize = 24;
                $body
            }
            25..=28 => {
                const $w: usize = 28;
                $body
            }
            29..=32 => {
                const $w: usize = 32;
                $body
            }
            33..=36 => {
                const $w: usize = 36;
                $body
            }
            37..=40 => {
                const $w: usize = 40;
                $body
            }
            41..=48 => {
                const $w: usize = 48;
                $body
            }
            49..=56 => {
                const $w: usize = 56;
                $body
            }
            57..=64 => {
                const $w: usize = 64;
                $body
            }
            65..=128 => {
                const $w: usize = 128;
                $body
            }
            _ => $beyond,
        }
    };
}

/// A set of `u32` values in `0..64·W`: the member `x` is bit `x % 64` of word `x / 64`.  Equality and hashing are
/// by members.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bits<const W: usize>([u64; W]);

/// Sum of the members of one word, where bit `i` stands for the value `base + i`.
#[inline]
fn word_sum(base: u64, mut word: u64) -> u64 {
    let mut total = base * u64::from(word.count_ones());
    while word != 0 {
        total += u64::from(word.trailing_zeros());
        word &= word - 1;
    }
    total
}

/// The `k` highest set bits of `word`, for `k <= popcount(word)`.
#[inline]
fn top_bits(mut word: u64, k: u32) -> u64 {
    // Clear the lowest set bit until only k remain.
    for _ in k..word.count_ones() {
        word &= word - 1;
    }
    word
}

/// Smallest value in `words`, where `words[0]` holds the values `64·first_word ..`.
#[inline]
fn lowest(words: &[u64], first_word: usize) -> Option<u32> {
    let (i, w) = words.iter().enumerate().find(|&(_, &w)| w != 0)?;
    Some((((first_word + i) as u32) << 6) | w.trailing_zeros())
}

/// Largest value in `words`, where `words[0]` holds the values `0..64`.
#[inline]
fn highest(words: &[u64]) -> Option<u32> {
    let (i, w) = words.iter().enumerate().rfind(|&(_, &w)| w != 0)?;
    Some(((i as u32) << 6) | (63 - w.leading_zeros()))
}

impl<const W: usize> Bits<W> {
    /// The empty set.
    pub const EMPTY: Self = {
        assert!(W > 0, "Bits needs at least one word");
        Bits([0; W])
    };
    /// Number of distinct values the set can hold: `64·W`.
    pub const CAPACITY: u32 = 64 * W as u32;
    /// Largest value the set can hold: `64·W − 1`.
    pub const MAX_VALUE: u32 = Self::CAPACITY - 1;

    /// The empty set.
    #[inline]
    pub const fn new() -> Self {
        Self::EMPTY
    }

    /// The set of the values in `vals`.
    #[inline]
    pub fn from_slice(vals: &[u32]) -> Self {
        vals.iter().copied().collect()
    }

    /// The set `{lo, lo+1, ..., hi}`; empty when `lo > hi`.  `hi` must be within the capacity (see the module
    /// documentation).
    #[inline]
    pub fn from_range(lo: u32, hi: u32) -> Self {
        debug_assert!(lo > hi || hi < Self::CAPACITY, "range end {hi} exceeds capacity {}", Self::CAPACITY);
        let mut out = Self::EMPTY;
        if lo > hi {
            return out;
        }
        for (i, word) in out.0.iter_mut().enumerate() {
            let base = (i as u32) << 6;
            let first = lo.max(base);
            let last = hi.min(base + 63);
            if first <= last {
                let width = last - first + 1; // 1..=64
                let mask = if width == 64 { !0 } else { ((1u64 << width) - 1) << (first - base) };
                *word = mask;
            }
        }
        out
    }

    /// Wraps raw words (word `i` holds the values `64·i ..= 64·i + 63`, least significant bit first).
    #[inline]
    pub const fn from_words(words: [u64; W]) -> Self {
        Bits(words)
    }

    /// The raw words, for serialization (the search's work units are written to checkpoints and spill files as
    /// little-endian words) and for code that walks the words itself.
    #[inline]
    pub const fn as_words(&self) -> &[u64; W] {
        &self.0
    }

    /// Adds `x` to the set.
    #[inline]
    pub fn insert(&mut self, x: u32) {
        debug_assert!(x < Self::CAPACITY, "value {x} exceeds capacity {}", Self::CAPACITY);
        self.0[(x >> 6) as usize] |= 1u64 << (x & 63);
    }

    /// Removes `x` from the set (no effect if it is absent).
    #[inline]
    pub fn remove(&mut self, x: u32) {
        debug_assert!(x < Self::CAPACITY, "value {x} exceeds capacity {}", Self::CAPACITY);
        self.0[(x >> 6) as usize] &= !(1u64 << (x & 63));
    }

    /// This set with `x` added.
    #[inline]
    #[must_use]
    pub fn with(mut self, x: u32) -> Self {
        self.insert(x);
        self
    }

    /// This set with `x` removed.
    #[inline]
    #[must_use]
    pub fn without(mut self, x: u32) -> Self {
        self.remove(x);
        self
    }

    /// Removes every member.
    #[inline]
    pub fn clear(&mut self) {
        *self = Self::EMPTY;
    }

    /// Whether `x` is a member.  Values beyond the capacity are never members.
    #[inline]
    pub fn contains(&self, x: u32) -> bool {
        match self.0.get((x >> 6) as usize) {
            Some(word) => (word >> (x & 63)) & 1 != 0,
            None => false,
        }
    }

    /// Number of members.
    #[inline]
    pub fn len(&self) -> u32 {
        self.0.iter().map(|w| w.count_ones()).sum()
    }

    /// Whether the set has no members.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|&w| w == 0)
    }

    /// Sum of the members.
    #[inline]
    pub fn sum(&self) -> u64 {
        self.0.iter().enumerate().map(|(i, &w)| word_sum((i as u64) << 6, w)).sum()
    }

    /// Smallest member.
    #[inline]
    pub fn min(&self) -> Option<u32> {
        lowest(&self.0, 0)
    }

    /// Largest member.
    #[inline]
    pub fn max(&self) -> Option<u32> {
        highest(&self.0)
    }

    /// Removes and returns the largest member.
    #[inline]
    pub fn pop_max(&mut self) -> Option<u32> {
        let x = self.max()?;
        self.remove(x);
        Some(x)
    }

    /// Removes and returns the smallest member.
    #[inline]
    pub fn pop_min(&mut self) -> Option<u32> {
        let x = self.min()?;
        self.remove(x);
        Some(x)
    }

    /// Largest member strictly below `x`.  `x` may exceed the capacity.
    #[inline]
    pub fn max_below(&self, x: u32) -> Option<u32> {
        if x == 0 {
            return None;
        }
        let top = (x - 1).min(Self::MAX_VALUE); // the largest value that may be returned
        let wi = (top >> 6) as usize;
        let masked = self.0[wi] & (!0u64 >> (63 - (top & 63)));
        if masked != 0 {
            return Some(((wi as u32) << 6) | (63 - masked.leading_zeros()));
        }
        highest(&self.0[..wi])
    }

    /// Smallest member strictly above `x`.  `x` may exceed the capacity.
    #[inline]
    pub fn min_above(&self, x: u32) -> Option<u32> {
        if x >= Self::MAX_VALUE {
            return None;
        }
        let start = x + 1;
        let wi = (start >> 6) as usize;
        let masked = self.0[wi] & (!0u64 << (start & 63));
        if masked != 0 {
            return Some(((wi as u32) << 6) | masked.trailing_zeros());
        }
        lowest(&self.0[wi + 1..], wi + 1)
    }

    /// Members of `self` or `other`.
    #[inline]
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let mut out = *self;
        out |= *other;
        out
    }

    /// Members of both `self` and `other`.
    #[inline]
    #[must_use]
    pub fn intersect(&self, other: &Self) -> Self {
        let mut out = *self;
        out &= *other;
        out
    }

    /// Members of `self` that are not in `other`.
    #[inline]
    #[must_use]
    pub fn difference(&self, other: &Self) -> Self {
        let mut out = *self;
        out -= *other;
        out
    }

    /// Whether every member of `self` is in `other`.
    #[inline]
    pub fn is_subset(&self, other: &Self) -> bool {
        self.0.iter().zip(&other.0).all(|(&a, &b)| a & !b == 0)
    }

    /// Whether `self` and `other` have a member in common.
    #[inline]
    pub fn intersects(&self, other: &Self) -> bool {
        self.0.iter().zip(&other.0).any(|(&a, &b)| a & b != 0)
    }

    /// Whether `self` and `other` have no member in common.
    #[inline]
    pub fn is_disjoint(&self, other: &Self) -> bool {
        !self.intersects(other)
    }

    /// Members in ascending order.
    #[inline]
    pub fn iter(&self) -> Iter<'_, W> {
        Iter { words: &self.0, idx: 0, cur: self.0[0] }
    }

    /// Members in descending order.
    #[inline]
    pub fn iter_desc(&self) -> IterDesc<'_, W> {
        IterDesc { words: &self.0, idx: W - 1, cur: self.0[W - 1] }
    }

    /// Calls `f(word_index, kept_bits)` for the words holding the `k` largest members, highest word first;
    /// `kept_bits` is the part of that word that belongs to the `k` largest (fewer than `k` if the set is smaller).
    #[inline]
    fn for_largest_words(&self, k: u32, mut f: impl FnMut(usize, u64)) {
        let mut need = k;
        for i in (0..W).rev() {
            if need == 0 {
                break;
            }
            let word = self.0[i];
            let pc = word.count_ones();
            if pc <= need {
                need -= pc;
                f(i, word);
            } else {
                f(i, top_bits(word, need));
                break;
            }
        }
    }

    /// The set of the `k` largest members (all of them if there are fewer than `k`).
    #[inline]
    #[must_use]
    pub fn largest(&self, k: u32) -> Self {
        let mut out = Self::EMPTY;
        self.for_largest_words(k, |i, bits| out.0[i] = bits);
        out
    }

    /// Sum of the `k` largest members (all of them if there are fewer than `k`).
    #[inline]
    pub fn sum_largest(&self, k: u32) -> u64 {
        let mut total = 0;
        self.for_largest_words(k, |i, bits| total += word_sum((i as u64) << 6, bits));
        total
    }

    /// The `k`-th largest member, counting from zero: `nth_largest(0)` is [`max`](Self::max).
    #[inline]
    pub fn nth_largest(&self, k: u32) -> Option<u32> {
        let mut skip = k;
        for i in (0..W).rev() {
            let word = self.0[i];
            let pc = word.count_ones();
            if skip < pc {
                // Drop the `skip` highest bits; the highest remaining one is the answer.
                let rest = word & !top_bits(word, skip);
                return Some(((i as u32) << 6) | (63 - rest.leading_zeros()));
            }
            skip -= pc;
        }
        None
    }

    /// Members in ascending order.
    pub fn to_vec(&self) -> Vec<u32> {
        let mut v = Vec::with_capacity(self.len() as usize);
        v.extend(self.iter());
        v
    }
}

impl<const W: usize> Default for Bits<W> {
    #[inline]
    fn default() -> Self {
        Self::EMPTY
    }
}

impl<const W: usize> fmt::Debug for Bits<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<const W: usize> FromIterator<u32> for Bits<W> {
    fn from_iter<I: IntoIterator<Item = u32>>(iter: I) -> Self {
        let mut out = Self::EMPTY;
        out.extend(iter);
        out
    }
}

impl<const W: usize> Extend<u32> for Bits<W> {
    fn extend<I: IntoIterator<Item = u32>>(&mut self, iter: I) {
        for x in iter {
            self.insert(x);
        }
    }
}

impl<'a, const W: usize> IntoIterator for &'a Bits<W> {
    type Item = u32;
    type IntoIter = Iter<'a, W>;

    #[inline]
    fn into_iter(self) -> Iter<'a, W> {
        self.iter()
    }
}

/// Ascending iterator over the members of a [`Bits`].
#[derive(Clone)]
pub struct Iter<'a, const W: usize> {
    words: &'a [u64; W],
    /// Index of the word `cur` was taken from.
    idx: usize,
    /// The not-yet-visited bits of word `idx`.
    cur: u64,
}

impl<const W: usize> Iterator for Iter<'_, W> {
    type Item = u32;

    #[inline]
    fn next(&mut self) -> Option<u32> {
        while self.cur == 0 {
            self.idx += 1;
            if self.idx >= W {
                self.idx = W;
                return None;
            }
            self.cur = self.words[self.idx];
        }
        let bit = self.cur.trailing_zeros();
        self.cur &= self.cur - 1;
        Some(((self.idx as u32) << 6) | bit)
    }
}

impl<const W: usize> FusedIterator for Iter<'_, W> {}

/// Descending iterator over the members of a [`Bits`].
#[derive(Clone)]
pub struct IterDesc<'a, const W: usize> {
    words: &'a [u64; W],
    /// Index of the word `cur` was taken from.
    idx: usize,
    /// The not-yet-visited bits of word `idx`.
    cur: u64,
}

impl<const W: usize> Iterator for IterDesc<'_, W> {
    type Item = u32;

    #[inline]
    fn next(&mut self) -> Option<u32> {
        while self.cur == 0 {
            if self.idx == 0 {
                return None;
            }
            self.idx -= 1;
            self.cur = self.words[self.idx];
        }
        let bit = 63 - self.cur.leading_zeros();
        self.cur &= !(1u64 << bit);
        Some(((self.idx as u32) << 6) | bit)
    }
}

impl<const W: usize> FusedIterator for IterDesc<'_, W> {}

/// Implements a binary operator (and its `*Assign` form) as a word-wise operation.
macro_rules! word_op {
    ($trait:ident, $method:ident, $assign_trait:ident, $assign_method:ident, $op:expr) => {
        impl<const W: usize> $assign_trait for Bits<W> {
            #[inline]
            fn $assign_method(&mut self, rhs: Self) {
                for (a, b) in self.0.iter_mut().zip(rhs.0) {
                    *a = $op(*a, b);
                }
            }
        }

        impl<const W: usize> $trait for Bits<W> {
            type Output = Self;

            #[inline]
            fn $method(mut self, rhs: Self) -> Self {
                self.$assign_method(rhs);
                self
            }
        }
    };
}

word_op!(BitOr, bitor, BitOrAssign, bitor_assign, |a: u64, b: u64| a | b);
word_op!(BitAnd, bitand, BitAndAssign, bitand_assign, |a: u64, b: u64| a & b);
word_op!(Sub, sub, SubAssign, sub_assign, |a: u64, b: u64| a & !b);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Rng;
    use std::collections::{BTreeSet, HashSet};

    /// A random number in `0..n`.
    fn below(rng: &mut Rng, n: u32) -> u32 {
        rng.below(u64::from(n)) as u32
    }

    /// A random set with a randomly chosen density, as both a model and a `Bits`.
    fn random_set<const W: usize>(rng: &mut Rng) -> (BTreeSet<u32>, Bits<W>) {
        let cap = Bits::<W>::CAPACITY;
        let target = match below(rng, 4) {
            0 => below(rng, 4),
            1 => below(rng, cap / 8 + 1),
            2 => below(rng, cap / 2 + 1),
            _ => below(rng, cap + 1),
        };
        let mut model = BTreeSet::new();
        let mut bits = Bits::<W>::EMPTY;
        for _ in 0..target {
            let x = below(rng, cap);
            model.insert(x);
            bits.insert(x);
        }
        (model, bits)
    }

    fn model_vec(model: &BTreeSet<u32>) -> Vec<u32> {
        model.iter().copied().collect()
    }

    fn check_unary<const W: usize>(model: &BTreeSet<u32>, bits: &Bits<W>) {
        let asc = model_vec(model);
        let desc: Vec<u32> = asc.iter().rev().copied().collect();
        assert_eq!(bits.len() as usize, asc.len());
        assert_eq!(bits.is_empty(), asc.is_empty());
        assert_eq!(bits.sum(), asc.iter().map(|&x| u64::from(x)).sum::<u64>());
        assert_eq!(bits.min(), asc.first().copied());
        assert_eq!(bits.max(), asc.last().copied());
        assert_eq!(bits.to_vec(), asc);
        assert_eq!(bits.iter().collect::<Vec<_>>(), asc);
        assert_eq!(bits.iter_desc().collect::<Vec<_>>(), desc);
        assert_eq!(bits.into_iter().count(), asc.len());

        // largest / sum_largest / nth_largest for every k, including k beyond the size.
        for k in 0..=asc.len() as u32 + 2 {
            let take: Vec<u32> = desc.iter().take(k as usize).copied().collect();
            let expect: BTreeSet<u32> = take.iter().copied().collect();
            assert_eq!(model_vec(&expect), bits.largest(k).to_vec(), "largest({k})");
            assert_eq!(take.iter().map(|&x| u64::from(x)).sum::<u64>(), bits.sum_largest(k), "sum_largest({k})");
            assert_eq!(desc.get(k as usize).copied(), bits.nth_largest(k), "nth_largest({k})");
        }

        // Neighbour queries, including values beyond the capacity.
        for x in 0..=Bits::<W>::CAPACITY + 2 {
            assert_eq!(model.contains(&x), bits.contains(x), "contains({x})");
            assert_eq!(model.range(..x).next_back().copied(), bits.max_below(x), "max_below({x})");
            assert_eq!(model.range(x + 1..).next().copied(), bits.min_above(x), "min_above({x})");
        }
        assert_eq!(bits.max_below(u32::MAX), asc.last().copied());
        assert_eq!(bits.min_above(u32::MAX), None);
    }

    fn check_binary<const W: usize>(am: &BTreeSet<u32>, a: &Bits<W>, bm: &BTreeSet<u32>, b: &Bits<W>) {
        let union: BTreeSet<u32> = am.union(bm).copied().collect();
        let inter: BTreeSet<u32> = am.intersection(bm).copied().collect();
        let diff: BTreeSet<u32> = am.difference(bm).copied().collect();
        assert_eq!(a.union(b).to_vec(), model_vec(&union));
        assert_eq!(a.intersect(b).to_vec(), model_vec(&inter));
        assert_eq!(a.difference(b).to_vec(), model_vec(&diff));
        assert_eq!((*a | *b).to_vec(), model_vec(&union));
        assert_eq!((*a & *b).to_vec(), model_vec(&inter));
        assert_eq!((*a - *b).to_vec(), model_vec(&diff));
        let (mut x, mut y, mut z) = (*a, *a, *a);
        x |= *b;
        y &= *b;
        z -= *b;
        assert_eq!((x, y, z), (a.union(b), a.intersect(b), a.difference(b)));
        assert_eq!(a.is_subset(b), am.is_subset(bm));
        assert_eq!(b.is_subset(a), bm.is_subset(am));
        assert_eq!(a.intersects(b), !inter.is_empty());
        assert_eq!(a.is_disjoint(b), inter.is_empty());
        assert!(inter.iter().all(|&x| a.contains(x) && b.contains(x)));
        assert!(a.intersect(b).is_subset(a) && a.is_subset(&a.union(b)));
        assert_eq!(*a == *b, am == bm);
    }

    fn random_agreement<const W: usize>(seed: u64) {
        random_agreement_rounds::<W>(seed, 150);
    }

    fn random_agreement_rounds<const W: usize>(seed: u64, rounds: usize) {
        let mut rng = Rng(seed);
        for _ in 0..rounds {
            let (am, a) = random_set::<W>(&mut rng);
            let (bm, b) = random_set::<W>(&mut rng);
            check_unary(&am, &a);
            check_binary(&am, &a, &bm, &b);
        }
    }

    #[test]
    fn random_agreement_with_model_w1() {
        random_agreement::<1>(0x9E37_79B9_7F4A_7C15);
    }

    #[test]
    fn random_agreement_with_model_w2() {
        random_agreement::<2>(0xD1B5_4A32_D192_ED03);
    }

    #[test]
    fn random_agreement_with_model_w3() {
        random_agreement::<3>(0x1234_5678_9ABC_DEF1);
    }

    #[test]
    fn random_agreement_with_model_w8() {
        random_agreement::<8>(0xDEAD_BEEF_CAFE_F00D);
    }

    /// The widths the solver picks for boards from 512 up (16, 32, 36, 40, 48, 56, 64 and 128 words, a sample of its
    /// widths), which no other unit test reaches: a loop that stops at a fixed word (a `len` that sums only the first
    /// 32, say) passes everything below.
    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is a_few_random_agreements_with_model_at_the_widths_of_large_boards"
    )]
    fn random_agreement_with_model_at_the_widths_of_large_boards() {
        random_agreements_at_the_widths_of_large_boards(1);
    }

    /// [`random_agreement_with_model_at_the_widths_of_large_boards`] with a sixth of the rounds.
    #[test]
    fn a_few_random_agreements_with_model_at_the_widths_of_large_boards() {
        random_agreements_at_the_widths_of_large_boards(6);
    }

    /// The rounds of [`random_agreement_with_model_at_the_widths_of_large_boards`], each divided by `divisor`.
    fn random_agreements_at_the_widths_of_large_boards(divisor: usize) {
        random_agreement_rounds::<16>(0x0123_4567_89AB_CDEF, 60 / divisor);
        random_agreement_rounds::<32>(0xFEDC_BA98_7654_3210, 30 / divisor);
        random_agreement_rounds::<36>(0x1357_9BDF_0246_8ACE, 25 / divisor);
        random_agreement_rounds::<40>(0x2468_ACE0_1357_9BDF, 20 / divisor);
        random_agreement_rounds::<48>(0x0F1E_2D3C_4B5A_6978, 20 / divisor);
        random_agreement_rounds::<56>(0x8796_A5B4_C3D2_E1F0, 15 / divisor);
        random_agreement_rounds::<64>(0xA5A5_5A5A_C3C3_3C3C, 15 / divisor);
        random_agreement_rounds::<128>(0x3C3C_A5A5_5A5A_C3C3, 8 / divisor);
    }

    /// Every word holds numbers, including its first and last bit: the sums, counts, extremes and neighbours across
    /// each word boundary, at the large widths.
    fn every_word_is_used<const W: usize>() {
        let edge = [0u32, 1, 62, 63];
        let all: Vec<u32> = (0..W as u32).flat_map(|word| edge.map(|bit| 64 * word + bit)).collect();
        let bits: Bits<W> = all.iter().copied().collect();
        assert_eq!(bits.to_vec(), all);
        assert_eq!(bits.len() as usize, all.len());
        assert_eq!(bits.sum(), all.iter().map(|&x| u64::from(x)).sum::<u64>());
        assert_eq!((bits.min(), bits.max()), (Some(0), Some(64 * W as u32 - 1)));
        assert_eq!(bits.largest(5).to_vec(), all[all.len() - 5..]);
        assert_eq!(bits.sum_largest(7), all[all.len() - 7..].iter().map(|&x| u64::from(x)).sum::<u64>());
        assert_eq!(bits.nth_largest(all.len() as u32 - 1), Some(0));
        for (i, &x) in all.iter().enumerate() {
            assert!(bits.contains(x));
            assert_eq!(bits.max_below(x), i.checked_sub(1).map(|j| all[j]), "max_below({x})");
            assert_eq!(bits.min_above(x), all.get(i + 1).copied(), "min_above({x})");
        }
        // Words that are all ones, and the intersection and difference of the odd and even words.
        let (odd, even): (Vec<u32>, Vec<u32>) =
            ((0..W as u32).filter(|w| w % 2 == 1).flat_map(|w| 64 * w..64 * w + 64)).partition(|x| x % 2 == 1);
        let (a, b) = (Bits::<W>::from_slice(&odd), Bits::<W>::from_slice(&even));
        assert_eq!(a.len() as usize, odd.len());
        assert!(a.is_disjoint(&b) && (a | b).len() as usize == odd.len() + even.len());
        let mut drained = bits;
        let mut seen = 0;
        while let Some(x) = drained.pop_max() {
            assert_eq!(Some(x), all.get(all.len() - 1 - seen).copied());
            seen += 1;
        }
        assert_eq!(seen, all.len());
    }

    #[test]
    fn every_word_of_the_large_widths_is_used() {
        every_word_is_used::<16>();
        every_word_is_used::<32>();
        every_word_is_used::<36>();
        every_word_is_used::<40>();
        every_word_is_used::<48>();
        every_word_is_used::<56>();
        every_word_is_used::<64>();
        every_word_is_used::<128>();
    }

    #[test]
    fn empty_set() {
        let e = Bits::<2>::new();
        assert_eq!(e, Bits::<2>::EMPTY);
        assert_eq!(e, Bits::<2>::default());
        assert!(e.is_empty());
        assert_eq!(e.len(), 0);
        assert_eq!(e.sum(), 0);
        assert_eq!((e.min(), e.max()), (None, None));
        assert_eq!((e.max_below(100), e.min_above(0)), (None, None));
        assert_eq!(e.iter().next(), None);
        assert_eq!(e.iter_desc().next(), None);
        assert_eq!(e.largest(5), e);
        assert_eq!(e.sum_largest(5), 0);
        assert_eq!(e.nth_largest(0), None);
        assert_eq!(e.to_vec(), Vec::<u32>::new());
        assert_eq!(format!("{e:?}"), "{}");
    }

    #[test]
    fn capacity_constants_and_words_for() {
        assert_eq!(Bits::<1>::CAPACITY, 64);
        assert_eq!(Bits::<1>::MAX_VALUE, 63);
        assert_eq!(Bits::<8>::CAPACITY, 512);
        assert_eq!(Bits::<8>::MAX_VALUE, 511);
        assert_eq!(words_for(0), 1);
        assert_eq!(words_for(63), 1);
        assert_eq!(words_for(64), 2);
        assert_eq!(words_for(511), 8);
        assert_eq!(words_for(512), 9);
        assert_eq!(words_for(1000), 16);
        // The largest value n must itself fit.
        for n in [0u32, 1, 62, 63, 64, 65, 127, 128, 511, 512, 1000, 4095, 4096, 8191] {
            assert!(n < 64 * words_for(n) as u32);
        }
    }

    #[test]
    fn the_width_dispatch_follows_the_widths() {
        assert!(WIDTHS.is_sorted() && WIDTHS[WIDTHS.len() - 1] == 128);
        for words in 0..=140 {
            let picked = crate::with_words!(words, W => Some(W), else None);
            assert_eq!(picked, width_for(words), "{words} words");
            assert!(picked.is_none_or(|w| w >= words && WIDTHS.contains(&w)), "{words} words");
        }
        // The last board of each width, and the first of the next.
        let edges =
            [(1, 8), (511, 8), (512, 16), (1023, 16), (1024, 20), (1215, 20), (1279, 20), (1280, 24), (1535, 24)];
        let more = [(1536, 28), (1791, 28), (1792, 32), (2047, 32), (2048, 36), (2303, 36), (2304, 40), (2559, 40)];
        let most = [(2560, 48), (3071, 48), (3072, 56), (3583, 56), (3584, 64), (4095, 64), (4096, 128), (8191, 128)];
        for (n, width) in edges.into_iter().chain(more).chain(most) {
            assert_eq!(crate::with_width!(n, W => W), width, "n={n}");
            assert_eq!(width_for(words_for(n)), Some(width), "n={n}");
        }
        assert_eq!(crate::with_width!(8192, W => Some(W), else None), None);
    }

    #[test]
    fn w1_boundaries() {
        let mut b = Bits::<1>::new();
        b.insert(0);
        b.insert(63);
        assert_eq!(b.to_vec(), vec![0, 63]);
        assert_eq!(b.as_words(), &[1 | (1 << 63)]);
        assert_eq!(b.sum(), 63);
        assert_eq!((b.min(), b.max()), (Some(0), Some(63)));
        assert_eq!(b.max_below(63), Some(0));
        assert_eq!(b.max_below(64), Some(63));
        assert_eq!(b.max_below(1000), Some(63));
        assert_eq!(b.max_below(0), None);
        assert_eq!(b.max_below(1), Some(0));
        assert_eq!(b.min_above(0), Some(63));
        assert_eq!(b.min_above(62), Some(63));
        assert_eq!(b.min_above(63), None);
        assert_eq!(b.largest(1).to_vec(), vec![63]);
        assert_eq!(b.largest(2), b);
        assert_eq!(b.nth_largest(1), Some(0));
        assert_eq!(b.nth_largest(2), None);
        assert!(b.contains(0) && b.contains(63));
        assert!(!b.contains(64) && !b.contains(u32::MAX));
        b.remove(63);
        b.remove(0);
        assert!(b.is_empty());
    }

    #[test]
    fn w2_boundaries() {
        let mut b = Bits::<2>::new();
        for x in [0, 63, 64, 127] {
            b.insert(x);
        }
        assert_eq!(b.to_vec(), vec![0, 63, 64, 127]);
        assert_eq!(b.iter_desc().collect::<Vec<_>>(), vec![127, 64, 63, 0]);
        assert_eq!(b.max_below(64), Some(63));
        assert_eq!(b.max_below(65), Some(64));
        assert_eq!(b.max_below(127), Some(64));
        assert_eq!(b.max_below(128), Some(127));
        assert_eq!(b.min_above(63), Some(64));
        assert_eq!(b.min_above(64), Some(127));
        assert_eq!(b.min_above(126), Some(127));
        assert_eq!(b.min_above(127), None);
        assert_eq!(b.largest(3).to_vec(), vec![63, 64, 127]);
        assert_eq!(b.sum_largest(3), 63 + 64 + 127);
        assert_eq!(b.nth_largest(2), Some(63));
        assert_eq!(b.sum(), 63 + 64 + 127);
    }

    #[test]
    fn insert_remove_are_idempotent() {
        let mut b = Bits::<2>::new();
        b.insert(70);
        b.insert(70);
        assert_eq!(b.len(), 1);
        b.remove(5); // absent
        assert_eq!(b.len(), 1);
        b.remove(70);
        b.remove(70);
        assert!(b.is_empty());
        let c = Bits::<2>::new().with(3).with(100).with(3);
        assert_eq!(c.to_vec(), vec![3, 100]);
        assert_eq!(c.without(3).to_vec(), vec![100]);
        let mut d = c;
        d.clear();
        assert!(d.is_empty());
    }

    #[test]
    fn from_slice_collect_extend_and_words() {
        let a = Bits::<2>::from_slice(&[9, 3, 5, 9, 127]);
        assert_eq!(a.to_vec(), vec![3, 5, 9, 127]);
        let b: Bits<2> = [127, 5, 3, 9].into_iter().collect();
        assert_eq!(a, b);
        let mut c = Bits::<2>::new();
        c.extend([3, 5]);
        c.extend(vec![9, 127]);
        assert_eq!(a, c);
        assert_eq!(Bits::<2>::from_words(*a.as_words()), a);
        let mut seen = Vec::new();
        for x in &a {
            seen.push(x);
        }
        assert_eq!(seen, vec![3, 5, 9, 127]);
    }

    #[test]
    fn from_range_matches_model() {
        for lo in 0..128u32 {
            for hi in 0..128u32 {
                let expect: Vec<u32> = (lo..=hi).collect();
                assert_eq!(Bits::<2>::from_range(lo, hi).to_vec(), expect, "{lo}..={hi}");
            }
        }
        assert_eq!(Bits::<2>::from_range(1, 100).len(), 100);
        assert!(Bits::<2>::from_range(5, 4).is_empty());
        assert!(Bits::<1>::from_range(70, 3).is_empty());
        assert_eq!(Bits::<1>::from_range(0, 63).len(), 64);
        assert_eq!(Bits::<1>::from_range(0, 63).sum(), 63 * 64 / 2);
    }

    #[test]
    fn pop_min_and_max_drain_in_order() {
        let mut rng = Rng(42);
        let (model, bits) = random_set::<3>(&mut rng);
        let mut asc = bits;
        let mut desc = bits;
        let mut got_asc = Vec::new();
        while let Some(x) = asc.pop_min() {
            got_asc.push(x);
        }
        let mut got_desc = Vec::new();
        while let Some(x) = desc.pop_max() {
            got_desc.push(x);
        }
        assert_eq!(got_asc, model_vec(&model));
        got_desc.reverse();
        assert_eq!(got_desc, model_vec(&model));
        assert!(asc.is_empty() && desc.is_empty());
    }

    #[test]
    fn debug_format() {
        assert_eq!(format!("{:?}", Bits::<1>::from_slice(&[9, 3, 5])), "{3, 5, 9}");
        assert_eq!(format!("{:?}", Bits::<2>::from_slice(&[100])), "{100}");
    }

    #[test]
    fn hash_and_equality() {
        let a = Bits::<2>::from_slice(&[1, 2, 100]);
        let b = Bits::<2>::from_slice(&[100, 2, 1]);
        let c = Bits::<2>::from_slice(&[1, 2]);
        let set: HashSet<Bits<2>> = [a, b, c].into_iter().collect();
        assert_eq!(set.len(), 2);
        assert!(set.contains(&a) && set.contains(&c));
    }

    #[test]
    fn iterators_are_fused() {
        let b = Bits::<2>::from_slice(&[1, 70]);
        let mut it = b.iter();
        assert_eq!((it.next(), it.next(), it.next(), it.next()), (Some(1), Some(70), None, None));
        let mut it = b.iter_desc();
        assert_eq!((it.next(), it.next(), it.next(), it.next()), (Some(70), Some(1), None, None));
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "exceeds capacity")]
    fn insert_out_of_range_is_caught_in_debug() {
        Bits::<1>::new().insert(64);
    }
}
