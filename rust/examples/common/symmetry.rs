//! Prime exchanges, and which of them are symmetries of a game's search problem at its root, for the examples
//! that study symmetry (`#[path = "common/symmetry.rs"] mod symmetry;`; `examples/dominance.rs` is the first).
//!
//! Exchanging the exponents of two primes `p < q` in every number is an automorphism of divisibility, so of the
//! maximal-factor graph wherever it maps the board onto itself.  When it also maps every frame of the root onto
//! itself (moves to moves, factors to factors, the factor graph among them onto itself) and the candidates onto
//! themselves, it maps every promotion set that can be promoted from the root to one that can, worth
//! `(q − p)·(c_p − c_q)` more, where `c_r` is the sum of `x / r` over the members `x` divisible by `r` (when no
//! candidate is divisible by both, nor by a square of either, as for every pair found to `n = 1300`).

use taxman::bits::Bits;
use taxman::search::Prepared;

/// `x` with the exponents of the primes `p` and `q` exchanged.
pub fn swap_primes(x: u32, p: u32, q: u32) -> u32 {
    let (mut rest, mut a, mut b) = (x, 0, 0);
    while rest.is_multiple_of(p) {
        rest /= p;
        a += 1;
    }
    while rest.is_multiple_of(q) {
        rest /= q;
        b += 1;
    }
    rest * p.pow(b) * q.pow(a)
}

/// `set` mapped by `pi`, or `None` if an image is beyond `Bits<W>`.
pub fn map_set<const W: usize>(set: &Bits<W>, pi: &impl Fn(u32) -> u32) -> Option<Bits<W>> {
    let mut out = Bits::EMPTY;
    for x in set.iter() {
        let y = pi(x);
        if y > Bits::<W>::MAX_VALUE {
            return None;
        }
        out.insert(y);
    }
    Some(out)
}

/// The primes up to `n`.
pub fn primes_to(n: u32) -> Vec<u32> {
    (2..=n).filter(|&k| (2..).take_while(|d| d * d <= k).all(|d| !k.is_multiple_of(d))).collect()
}

/// Whether `pi` is a symmetry of the search problem at its root: it maps every frame's moves and factors onto
/// themselves, the candidates onto themselves, and the factor graph among the frames' numbers onto itself.  Then
/// every frame with any promotions applied is the same game as the frame with their images applied, so a set
/// can be promoted from the root exactly when its image can.
pub fn root_symmetry<const W: usize>(p: &Prepared<W>, pi: &impl Fn(u32) -> u32) -> bool {
    let g = p.model.graph();
    let mut all = Bits::<W>::EMPTY;
    for f in p.root.frames() {
        if map_set(&f.moves, pi) != Some(f.moves) || map_set(&f.factors, pi) != Some(f.factors) {
            return false;
        }
        all |= f.moves | f.factors;
    }
    if map_set(&p.candidates, pi) != Some(p.candidates) {
        return false;
    }
    all.iter().all(|x| {
        let mut image: Vec<u32> = g.factors(x).iter().map(|&q| pi(u32::from(q))).collect();
        let mut theirs: Vec<u32> = g.factors(pi(x)).iter().map(|&q| u32::from(q)).collect();
        image.sort_unstable();
        theirs.sort_unstable();
        image == theirs
    })
}

/// The pairs of primes `p < q` whose exchange is a symmetry of the root (see [`root_symmetry`]) and moves some
/// candidate, with no candidate divisible by both or by a square of either (so the exchange changes a set's sum
/// by `(q − p)·(c_p − c_q)`).
pub fn root_pairs<const W: usize>(p: &Prepared<W>) -> Vec<(u32, u32)> {
    let primes = primes_to(p.n / 2);
    let simple = |a: u32, b: u32| {
        p.candidates.iter().all(|x| !x.is_multiple_of(a * b) && !x.is_multiple_of(a * a) && !x.is_multiple_of(b * b))
    };
    let mut pairs = Vec::new();
    for (i, &a) in primes.iter().enumerate() {
        if !p.candidates.iter().any(|x| x.is_multiple_of(a)) {
            continue;
        }
        for &b in &primes[i + 1..] {
            if simple(a, b) && root_symmetry(p, &|x| swap_primes(x, a, b)) {
                pairs.push((a, b));
            }
        }
    }
    pairs
}

/// `c_r(set)`: the sum of `x / r` over the members `x` of `set` divisible by `r`.
pub fn cofactors<const W: usize>(set: &Bits<W>, r: u32) -> u64 {
    set.iter().filter(|x| x.is_multiple_of(r)).map(|x| u64::from(x / r)).sum()
}
