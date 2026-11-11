//! The steps of the parity bound (W4, docs/theory/parity-bound.md, "The parity steps") and where its gap stays (W5),
//! against the prover's committed parity bounds and the committed chain.
//!
//! The parity bound PB(n) of every board from 2 on is in two committed files of this crate,
//! `data/exact_parity_2_2531.jsonl` and `data/rootbounds_2532_5060.jsonl`: the field `parity_bound`, the root's bound
//! with no picks and no bars, each checked by its weight split when it was made.  The greedy basis G(n) of the
//! sources' vectors is carried here from board to board by W4's rule, and compared with a fresh greedy at sampled
//! boards; the step of each game and x_G come from it, by linear algebra over GF(2) that shares nothing with
//! `src/parity.rs`.
//!
//! * [`the_parity_steps_hold_on_the_committed_bounds`]: at every board, the bound rises as W4 says: by n − x_G at an
//!   exchange step, by n at a half-out step, by n − s to n at a half-in step (s = n/2), by at most n at an added step;
//!   the set C of an exchange step lies in n's layer, and x_G is the shadow of n (measured).
//! * [`the_parity_gap_stays_at_the_chains_exchange_and_half_out_steps`]: at every exchange step and half-out step of
//!   the committed chain, opt(n) rises as the parity bound does.  W5 proves it at the primes and at the odd numbers
//!   with two prime factors from 189 on, which must be exchange steps, and at the half-out steps given statement U and
//!   (♦) in game n − 1; at the other exchange steps it is measured.

use std::collections::BTreeMap;
use std::path::Path;
use taxman_prover::jsonl::{committed_chain, field_u64, recorded_games_to};
use taxman_prover::primes::{maximal_factors, smallest_prime_factors};
use taxman_prover::upper::omega;

/// Every this many boards, and at the last, the greedy basis carried from board to board is compared with a fresh one.
const FRESH_EVERY: u32 = 500;

/// The committed parity bounds PB(n), by n: the field `parity_bound` of the lines of `data/exact_parity_2_2531.jsonl`
/// and `data/rootbounds_2532_5060.jsonl`.
fn committed_bounds() -> BTreeMap<u32, u64> {
    let mut bounds = BTreeMap::new();
    for name in ["data/exact_parity_2_2531.jsonl", "data/rootbounds_2532_5060.jsonl"] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
        let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let n = field_u64(line, "n").expect("a line has its n") as u32;
            let bound = field_u64(line, "parity_bound").expect("a line has its parity bound");
            assert!(bounds.insert(n, bound).is_none(), "{name}: n={n} twice");
        }
    }
    bounds
}

/// A set of numbers, or a vector over GF(2) with a coordinate for each number, as bits.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Bits(Vec<u64>);

impl Bits {
    fn new(words: usize) -> Self {
        Bits(vec![0; words])
    }

    fn flip(&mut self, i: u32) {
        self.0[i as usize / 64] ^= 1 << (i % 64);
    }

    fn has(&self, i: u32) -> bool {
        self.0[i as usize / 64] >> (i % 64) & 1 == 1
    }

    fn add(&mut self, other: &Bits) {
        self.0.iter_mut().zip(&other.0).for_each(|(a, b)| *a ^= b);
    }

    fn lowest(&self) -> Option<u32> {
        let word = self.0.iter().position(|&w| w != 0)?;
        Some(word as u32 * 64 + self.0[word].trailing_zeros())
    }

    fn members(&self) -> impl Iterator<Item = u32> + '_ {
        (0..self.0.len() as u32 * 64).filter(|&i| self.has(i))
    }
}

/// A basis of the span of some sources' vectors: rows with distinct lowest coordinates, each with the set of sources
/// whose vectors sum to it.
struct Span {
    words: usize,
    rows: Vec<(Bits, Bits)>,
    /// The row whose lowest coordinate is each coordinate.
    row_at: Vec<Option<usize>>,
}

impl Span {
    fn new(words: usize) -> Self {
        Span { words, rows: Vec::new(), row_at: vec![None; words * 64] }
    }

    /// The vector `v` less a sum of rows, and the set of sources whose vectors that sum adds up: the remainder is 0
    /// exactly when the rows span `v`, which is then the sum of those sources' vectors.
    fn reduce(&self, v: &Bits) -> (Bits, Bits) {
        let (mut rest, mut sum) = (v.clone(), Bits::new(self.words));
        while let Some(low) = rest.lowest() {
            let Some(row) = self.row_at[low as usize] else { break };
            rest.add(&self.rows[row].0);
            sum.add(&self.rows[row].1);
        }
        (rest, sum)
    }

    /// Adds the source `x`, whose vector `v` the rows do not span.
    fn insert(&mut self, x: u32, v: &Bits) {
        let (rest, mut sum) = self.reduce(v);
        let low = rest.lowest().unwrap_or_else(|| panic!("the vector of {x} is spanned"));
        sum.flip(x);
        self.row_at[low as usize] = Some(self.rows.len());
        self.rows.push((rest, sum));
    }

    /// Takes the source `x` out: of the rows whose sums hold x, the one with the highest lowest coordinate is added to
    /// the others, which keeps their lowest coordinates, and goes.
    fn remove(&mut self, x: u32) {
        let lowest = |row: &(Bits, Bits)| row.0.lowest().expect("a row is not 0");
        let holding: Vec<usize> = (0..self.rows.len()).filter(|&r| self.rows[r].1.has(x)).collect();
        let top = *holding.iter().max_by_key(|&&r| lowest(&self.rows[r])).unwrap_or_else(|| panic!("{x} is in no row"));
        let row = self.rows[top].clone();
        for &r in holding.iter().filter(|&&r| r != top) {
            self.rows[r].0.add(&row.0);
            self.rows[r].1.add(&row.1);
        }
        self.row_at[lowest(&row) as usize] = None;
        self.rows.swap_remove(top);
        if top < self.rows.len() {
            self.row_at[lowest(&self.rows[top]) as usize] = Some(top);
        }
    }
}

/// The step of game n (docs/theory/parity-bound.md, "The parity steps"), with x_G at an exchange step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Exchange(u32),
    Added,
    HalfIn,
    HalfOut,
}

/// The incidence vector v(x) over GF(2): a 1 at each maximal factor of x.
fn vector(x: u32, spf: &[u32], words: usize) -> Bits {
    let mut v = Bits::new(words);
    maximal_factors(u64::from(x), spf).into_iter().for_each(|f| v.flip(f as u32));
    v
}

/// The greedy basis of the sources' vectors of board m, fresh: the sources by decreasing value, each kept whose vector
/// the kept ones do not span.
fn fresh_greedy_basis(m: u32, spf: &[u32], words: usize) -> Bits {
    let (mut span, mut kept) = (Span::new(words), Bits::new(words));
    for s in (m / 2 + 1..=m).rev() {
        let v = vector(s, spf, words);
        if span.reduce(&v).0.lowest().is_some() {
            span.insert(s, &v);
            kept.flip(s);
        }
    }
    kept
}

/// The steps of the games 3..=`last`, by n.  The greedy basis is carried from board to board by W4's rule (n joins;
/// x_G or n/2 leaves at an exchange or a half-in step) and compared with a fresh greedy every [`FRESH_EVERY`] boards
/// and at the last.  At an exchange step the set C of sources of G(n − 1) whose vectors sum to v(n) must lie in n's
/// layer.
fn steps(last: u32, spf: &[u32]) -> BTreeMap<u32, Step> {
    let words = last as usize / 64 + 1;
    let (mut span, mut basis) = (Span::new(words), Bits::new(words));
    span.insert(2, &vector(2, spf, words));
    basis.flip(2);
    let mut steps = BTreeMap::new();
    for n in 3..=last {
        let v = vector(n, spf, words);
        let step = if n % 2 == 1 {
            let (rest, sum) = span.reduce(&v);
            match sum.lowest() {
                Some(x) if rest.lowest().is_none() => {
                    let off = sum.members().find(|&c| omega(c, spf) != omega(n, spf));
                    assert_eq!(off, None, "n={n}: a member of C lies outside n's layer");
                    Step::Exchange(x)
                }
                _ => Step::Added,
            }
        } else if basis.has(n / 2) {
            Step::HalfIn
        } else {
            Step::HalfOut
        };
        let leaves = match step {
            Step::Exchange(x) => Some(x),
            Step::HalfIn => Some(n / 2),
            Step::Added | Step::HalfOut => None,
        };
        if let Some(x) = leaves {
            span.remove(x);
            basis.flip(x);
        }
        span.insert(n, &v);
        basis.flip(n);
        if n % FRESH_EVERY == 0 || n == last {
            assert!(fresh_greedy_basis(n, spf, words) == basis, "n={n}: the greedy basis carried is not the fresh one");
        }
        steps.insert(n, step);
    }
    steps
}

/// The shadow of an odd n: n with each prime factor, counted with multiplicity, replaced by the prime just below it.
fn shadow(n: u32, spf: &[u32]) -> u32 {
    let below = |p: u32| (2..p).rev().find(|&q| spf[q as usize] == q).expect("an odd prime has a prime below it");
    let (mut rest, mut out) = (n, 1);
    while rest > 1 {
        out *= below(spf[rest as usize]);
        rest /= spf[rest as usize];
    }
    out
}

/// The committed bounds, which must cover every board from 2 on, the last board, and the smallest prime factors up to
/// it.
fn bounds_and_sieve() -> (BTreeMap<u32, u64>, u32, Vec<u32>) {
    let bounds = committed_bounds();
    let last = *bounds.keys().next_back().expect("some bounds");
    assert!(bounds.keys().copied().eq(2..=last), "the committed bounds are not those of the boards 2..={last}");
    (bounds, last, smallest_prime_factors(last as usize))
}

/// The parity steps (W4, docs/theory/parity-bound.md) on the prover's committed bounds: at every board 3..=last, the
/// bound rises by at most n, and by exactly n − x_G at an exchange step, by n at a half-out step, and by n − s to n at
/// a half-in step.  And at every exchange step, x_G is the shadow of n, as the shadow law (docs/theory/arrival.md)
/// observes (measured).
#[test]
fn the_parity_steps_hold_on_the_committed_bounds() {
    let (bounds, last, spf) = bounds_and_sieve();
    let steps = steps(last, &spf);
    let (mut failures, mut counts) = (Vec::new(), BTreeMap::new());
    for (&n, &step) in &steps {
        let rise = bounds[&n] as i64 - bounds[&(n - 1)] as i64;
        let (whole, half) = (i64::from(n), i64::from(n / 2));
        let wrong = match step {
            _ if rise > whole => Some(format!("the bound rises by {rise}, above the ceiling n")),
            Step::Exchange(x) if rise != whole - i64::from(x) => {
                Some(format!("the bound rises by {rise}, not n − x_G, x_G = {x}"))
            }
            Step::Exchange(x) if x != shadow(n, &spf) => {
                Some(format!("x_G = {x} is not the shadow {}", shadow(n, &spf)))
            }
            Step::HalfIn if rise < whole - half => Some(format!("the bound rises by {rise}, below n − n/2")),
            Step::HalfOut if rise != whole => Some(format!("the bound rises by {rise}, not n")),
            _ => None,
        };
        if let Some(wrong) = wrong {
            failures.push(format!("n={n} ({step:?}): {wrong}"));
        }
        let kind = match step {
            Step::Exchange(_) => "exchange",
            Step::Added => "added",
            Step::HalfIn => "half-in",
            Step::HalfOut => "half-out",
        };
        *counts.entry(kind).or_insert(0) += 1;
    }
    println!("the parity steps of 3..={last}: {counts:?}, {} failures", failures.len());
    assert!(failures.is_empty(), "W4 fails at {} boards:\n{}", failures.len(), failures.join("\n"));
    assert_eq!(counts.len(), 4, "a kind of step never happens: {counts:?}");
}

/// Where the parity gap stays (W5, docs/theory/parity-bound.md) on the committed chain: at every exchange step and
/// every half-out step, opt(n) − opt(n − 1) = PB(n) − PB(n − 1).  Every prime and every odd number with two prime
/// factors from 189 on is an exchange step, where W5 proves it; at the half-out steps it holds given statement U and
/// (♦) in game n − 1, and at the other exchange steps it is measured.
#[test]
fn the_parity_gap_stays_at_the_chains_exchange_and_half_out_steps() {
    let (bounds, last, spf) = bounds_and_sieve();
    let chain = recorded_games_to(&[committed_chain()], u64::from(last));
    let opt = |n: u32| chain.get(&u64::from(n)).unwrap_or_else(|| panic!("the committed chain has no game {n}")).0;
    let steps = steps(last, &spf);
    let mut failures = Vec::new();
    for (&n, &step) in &steps {
        let semiprime = n % 2 == 1 && n >= 189 && omega(n, &spf) == 2;
        if (omega(n, &spf) == 1 || semiprime) && !matches!(step, Step::Exchange(_)) {
            failures.push(format!("n={n}: a prime or an odd semiprime from 189 that is no exchange step ({step:?})"));
        }
        if matches!(step, Step::Exchange(_) | Step::HalfOut) {
            let (game, bound) = (opt(n) as i64 - opt(n - 1) as i64, bounds[&n] as i64 - bounds[&(n - 1)] as i64);
            if game != bound {
                failures.push(format!("n={n} ({step:?}): opt rises by {game}, the parity bound by {bound}"));
            }
        }
    }
    assert!(failures.is_empty(), "W5 fails at {} games:\n{}", failures.len(), failures.join("\n"));
}
