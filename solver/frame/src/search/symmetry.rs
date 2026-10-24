//! The root's prime exchanges (symmetric pairs of primes), and the rule they give the search.
//!
//! # Prime exchanges
//!
//! For primes `p < q`, the *exchange* `π` of `p` and `q` writes a number as `p^a·q^b·r`, with `r` divisible by
//! neither prime, and maps it to `p^b·q^a·r` ([`swap_primes`]).  It is an involution and it keeps divisibility
//! (`x | y` exactly when `π(x) | π(y)`), so it maps the maximal-factor graph onto itself wherever it maps the board
//! onto itself.  It is a *symmetry of the root* when it also
//!
//! 1. maps every frame's moves onto its moves, and its factors onto its factors;
//! 2. maps the candidates onto themselves; and
//! 3. maps the factor graph among the frames' numbers onto itself: for every number `x` of the frames, the maximal
//!    factors that can pay for `π(x)` are the images of those that can pay for `x`.
//!
//! Then a frame with some promotions made and the frame with their images made are one game under the renaming `π`
//! (the promotions move the same numbers between the same frames, and the graph is the same), so a set `S` of
//! candidates can be promoted from the root exactly when `π(S)` can.  [`is_root_symmetry`] decides it for a pair, and
//! [`root_pairs`] finds the *symmetric pairs* of a game.
//!
//! # What an exchange does to a sum
//!
//! Let `c_r(S)` be the sum of `x / r` over the members `x` of `S` divisible by `r` ([`cofactors`]).  Suppose no
//! candidate is divisible by `p·q`, `p²` or `q²` (`root_pairs` reports only the pairs for which that holds).  Then a
//! candidate is `m·p`, `m·q` or fixed, with `m` divisible by neither prime, and `π(m·p) = m·q = m·p + m(q − p)`,
//! `π(m·q) = m·p = m·q − m(q − p)`.  So for a set `S` of candidates
//!
//! ```text
//! sum(π(S)) − sum(S) = (q − p)·(c_p(S) − c_q(S)).
//! ```
//!
//! # The rule
//!
//! A heaviest promotion set `S` has `c_p(S) ≤ c_q(S)` for every symmetric pair `(p, q)`: otherwise `π(S)` can be
//! promoted too and is heavier (the prime-exchange theorem (R2, `docs/theory/search-rules.md`)).  So a search for a
//! heaviest set may look only at the sets that *keep* every pair's constraint ([`Symmetries::keeps`]), and cut a node
//! when no completion of it can: [`Symmetries::hopeless`] tests that, and [`Symmetries::exclude`] also takes out of the
//! node's candidates those that no such completion holds.  Both are arithmetic on the cofactor sums of the node's
//! promotions and candidates, and allocate nothing.
//!
//! # What is where
//!
//! * [`swap_primes`], [`map_set`], [`cofactors`]: the arithmetic of an exchange, on a number and on a set, and of its
//!   effect on a sum.
//! * [`is_root_symmetry`], [`root_pairs`]: whether an exchange is a symmetry of the root, and which exchanges are.
//! * [`Symmetries`], [`RuleScratch`]: the pairs of a game as the search's rule uses them, built once per search.

use crate::bits::Bits;
use crate::model::{FrameGraph, FrameSet};
use crate::numtheory;

/// `x` (at least 1) with the exponents of the primes `p` and `q` exchanged; `u32::MAX` if that is more than a `u32`
/// holds (then it is beyond every [`Bits`] as well).
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
    match (p.checked_pow(b), q.checked_pow(a)) {
        (Some(p_b), Some(q_a)) => rest.checked_mul(p_b).and_then(|r| r.checked_mul(q_a)).unwrap_or(u32::MAX),
        _ => u32::MAX,
    }
}

/// `set` with every member mapped by `pi`, or `None` if an image is beyond what a [`Bits`] holds.
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

/// `c_r(set)`: the sum of `x / r` over the members `x` of `set` divisible by `r`.
pub fn cofactors<const W: usize>(set: &Bits<W>, r: u32) -> u64 {
    set.iter().filter(|x| x.is_multiple_of(r)).map(|x| u64::from(x / r)).sum()
}

/// The members of `set` divisible by `p` or by `q`, found by stepping through the multiples of each (so the cost
/// does not depend on how many members `set` has).
fn multiples_in<'a, const W: usize>(set: &'a Bits<W>, p: u32, q: u32) -> impl Iterator<Item = u32> + 'a {
    let top = set.max().unwrap_or(0);
    let of_p = (p..=top).step_by(p as usize);
    let of_q = (q..=top).step_by(q as usize).filter(move |x| !x.is_multiple_of(p));
    of_p.chain(of_q).filter(move |&x| set.contains(x))
}

/// Whether the exchange of `p` and `q` maps every member of `set` that it moves into `set`.  The exchange is an
/// involution, so injective, and a finite set mapped into itself by an injection is mapped onto itself: then the
/// exchange maps `set` onto itself, since it fixes the members divisible by neither prime.  An image beyond what a
/// [`Bits`] holds is never a member.
fn maps_into<const W: usize>(set: &Bits<W>, p: u32, q: u32) -> bool {
    multiples_in(set, p, q).all(|x| set.contains(swap_primes(x, p, q)))
}

/// Whether the exchange of the primes `p < q` is a symmetry of the search problem at its root: it maps every frame's
/// moves and factors onto themselves, the candidates onto themselves, and the factor graph among the frames' numbers
/// onto itself (see the module docs).  It does not look at whether the exchange moves the candidates by the formula
/// of the module docs: [`root_pairs`] adds that.
///
/// Only the numbers divisible by `p` or `q` are looked at, which is all of the check.  The exchange is an
/// involution, so injective, and a finite set mapped into itself by an injection is mapped onto itself; the numbers
/// divisible by neither prime are fixed.  So are their maximal factors (a divisor of such a number is divisible by
/// neither prime either), and a fixed number's list of maximal factors is its own image.
pub fn is_root_symmetry<const W: usize>(
    model: &FrameGraph,
    root: &FrameSet<W>,
    candidates: &Bits<W>,
    p: u32,
    q: u32,
) -> bool {
    let mut numbers = Bits::<W>::EMPTY;
    for frame in root.frames() {
        if !maps_into(&frame.moves, p, q) || !maps_into(&frame.factors, p, q) {
            return false;
        }
        numbers |= frame.moves | frame.factors;
    }
    if !maps_into(candidates, p, q) {
        return false;
    }
    // The images of the numbers are numbers of the frames now, so the graph has lists for them.
    let graph = model.graph();
    let (mut image, mut theirs) = (Vec::new(), Vec::new());
    let mut moved = multiples_in(&numbers, p, q);
    moved.all(|x| {
        image.clear();
        theirs.clear();
        image.extend(graph.factors(x).iter().map(|&f| swap_primes(u32::from(f), p, q)));
        theirs.extend(graph.factors(swap_primes(x, p, q)).iter().map(|&f| u32::from(f)));
        image.sort_unstable();
        theirs.sort_unstable();
        image == theirs
    })
}

/// Whether no candidate is divisible by `p·q`, `p²` or `q²`, so that the exchange of `p` and `q` changes a set's sum
/// by `(q − p)·(c_p − c_q)`.
fn is_simple<const W: usize>(candidates: &Bits<W>, p: u32, q: u32) -> bool {
    let top = candidates.max().unwrap_or(0);
    [p * q, p * p, q * q].into_iter().all(|m| !(m..=top).step_by(m as usize).any(|x| candidates.contains(x)))
}

/// Calls `f(r, e)` for each prime `r` dividing `x`, with `e` its exponent in `x`; `spf` is a sieve that reaches `x`.
fn for_each_prime_power(spf: &[u32], mut x: u32, mut f: impl FnMut(u32, u32)) {
    while x > 1 {
        let r = spf[x as usize];
        let mut e = 0;
        while x.is_multiple_of(r) {
            x /= r;
            e += 1;
        }
        f(r, e);
    }
}

/// The *symmetric pairs* of the game at its root: the pairs of primes `p < q` whose exchange is a symmetry of the
/// root ([`is_root_symmetry`]) and moves some candidate, with no candidate divisible by `p·q`, `p²` or `q²` (so
/// that the exchange changes a set's sum by `(q − p)·(c_p − c_q)`), ascending by `(p, q)`.
///
/// Trying every pair of primes up to `n / 2` costs the square of their number times `n`: 20 ms a game at
/// `n = 2175`.  Instead, each prime that divides a candidate gets an *invariant*, and the full checks run only on
/// the pairs within a group of equal invariants.  The invariant of `r` is the multiset of the places of the numbers
/// divisible by `r`, each with the exponent of `r` in it, where a place is a frame's moves, a frame's factors, or the
/// candidates.  If the exchange of `p` and `q` is a symmetry of the root, `p` and `q` have equal invariants: the
/// exchange maps each place onto itself, and a number of the place with `p` to the exponent `a` onto one with `q` to
/// the exponent `a`.  A prime that divides no candidate cannot be part of a pair (if `p` divides a candidate and the
/// exchange maps the candidates onto themselves, `q` divides one too), so only the primes that divide a candidate
/// are factored and compared.
pub fn root_pairs<const W: usize>(model: &FrameGraph, root: &FrameSet<W>, candidates: &Bits<W>) -> Vec<(u32, u32)> {
    let frames = root.frames();
    let numbers = frames.iter().fold(*candidates, |all, frame| all | frame.moves | frame.factors);
    let Some(top) = numbers.max() else { return Vec::new() };
    let (spf, _) = numtheory::sieve(top);

    let mut divides_candidate = vec![false; top as usize + 1];
    for x in candidates.iter() {
        for_each_prime_power(&spf, x, |r, _| divides_candidate[r as usize] = true);
    }
    // For each such prime, one entry for each number it divides: the place of the number and the prime's exponent.
    let mut invariant: Vec<Vec<u32>> = vec![Vec::new(); top as usize + 1];
    let mut note = |place: usize, x: u32| {
        for_each_prime_power(&spf, x, |r, e| {
            if divides_candidate[r as usize] {
                invariant[r as usize].push(((place as u32) << 8) | e);
            }
        });
    };
    for (h, frame) in frames.iter().enumerate() {
        frame.moves.iter().for_each(|x| note(3 * h, x));
        frame.factors.iter().for_each(|x| note(3 * h + 1, x));
    }
    candidates.iter().for_each(|x| note(2, x));

    let mut primes: Vec<u32> = (2..=top).filter(|&r| divides_candidate[r as usize]).collect();
    for &r in &primes {
        invariant[r as usize].sort_unstable();
    }
    primes.sort_unstable_by(|&a, &b| invariant[a as usize].cmp(&invariant[b as usize]).then(a.cmp(&b)));
    let mut pairs = Vec::new();
    for group in primes.chunk_by(|&a, &b| invariant[a as usize] == invariant[b as usize]) {
        for (i, &p) in group.iter().enumerate() {
            for &q in &group[i + 1..] {
                if is_simple(candidates, p, q) && is_root_symmetry(model, root, candidates, p, q) {
                    pairs.push((p, q));
                }
            }
        }
    }
    pairs.sort_unstable();
    pairs
}

/// The symmetric pairs of a game's root, in the form the search's rule uses them (see the module docs), built once
/// per search by [`Symmetries::of`].
#[derive(Clone, Debug)]
pub struct Symmetries {
    /// [`root_pairs`], ascending.
    pairs: Vec<(u32, u32)>,
    /// The distinct primes of the pairs, ascending.
    primes: Vec<u32>,
    /// Each pair as the indices of its two primes in `primes`, in the order of `pairs`.
    links: Vec<(u16, u16)>,
    /// For each prime `r` of `primes`: the candidates `x` divisible by `r`, with `x / r`, ascending.
    family: Vec<Vec<(u32, u32)>>,
}

/// The sums a call of [`Symmetries::exclude`] or [`Symmetries::hopeless`] works in, made by [`Symmetries::scratch`]
/// and reused, so that the rule allocates nothing per call.
#[derive(Clone, Debug, Default)]
pub struct RuleScratch {
    /// For each prime of the pairs: the cofactor sum `c_r` over the promotions.
    promoted: Vec<i64>,
    /// The same over the candidates.
    cands: Vec<i64>,
    /// For each prime: the largest cofactor that a candidate divisible by it may add, see `exclude`.
    limit: Vec<i64>,
}

impl Symmetries {
    /// The symmetric pairs of the game at its root ([`root_pairs`]) and what the rule needs of them.
    pub fn of<const W: usize>(model: &FrameGraph, root: &FrameSet<W>, candidates: &Bits<W>) -> Self {
        Self::with_pairs(root_pairs(model, root, candidates), candidates)
    }

    /// The rule's data for `pairs` (ascending), over `candidates`.
    fn with_pairs<const W: usize>(pairs: Vec<(u32, u32)>, candidates: &Bits<W>) -> Self {
        let mut primes: Vec<u32> = pairs.iter().flat_map(|&(p, q)| [p, q]).collect();
        primes.sort_unstable();
        primes.dedup();
        let index = |r: u32| {
            let at = primes.binary_search(&r).expect("a prime of a pair");
            u16::try_from(at).expect("fewer than 65536 primes")
        };
        let links = pairs.iter().map(|&(p, q)| (index(p), index(q))).collect();
        let family = primes
            .iter()
            .map(|&r| candidates.iter().filter(|x| x.is_multiple_of(r)).map(|x| (x, x / r)).collect())
            .collect();
        Symmetries { pairs, primes, links, family }
    }

    /// Whether the game has no symmetric pair, so the rule does nothing.
    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    /// The symmetric pairs `(p, q)`, `p < q`, ascending.
    pub fn pairs(&self) -> &[(u32, u32)] {
        &self.pairs
    }

    /// Whether `set`, a set of candidates, keeps every pair's constraint: `c_p(set) ≤ c_q(set)` for every pair
    /// `(p, q)`.  Every heaviest promotion set does (see the module docs).
    pub fn keeps<const W: usize>(&self, set: &Bits<W>) -> bool {
        let sum = |i: u16| cofactor_sum(&self.family[usize::from(i)], set);
        self.links.iter().all(|&(p, q)| sum(p) <= sum(q))
    }

    /// Space for the rule's sums.
    pub fn scratch(&self) -> RuleScratch {
        let zeros = vec![0; self.primes.len()];
        RuleScratch { promoted: zeros.clone(), cands: zeros.clone(), limit: zeros }
    }

    /// Whether the node with promotions `promoted` and candidates `cands` is *hopeless*: no completion `F ⊆ cands`
    /// makes `promoted ∪ F` keep every pair's constraint.  The first test of [`exclude`](Self::exclude), which
    /// changes nothing.  Sound but not exact: when it says yes, no completion keeps the constraints; it can say no
    /// to a node none keeps them in (a pair's constraint can need a sum that the candidates cannot make, or the
    /// constraints of pairs that share a prime or a candidate can pull against each other).
    pub fn hopeless<const W: usize>(&self, s: &mut RuleScratch, promoted: &Bits<W>, cands: &Bits<W>) -> bool {
        self.sums(&mut s.promoted, promoted);
        self.sums(&mut s.cands, cands);
        self.some_pair_is_out_of_reach(s)
    }

    /// The rule at the node with promotions `P = promoted` and candidates `C = cands`: whether the node is
    /// [hopeless](Self::hopeless), and, when it is not, the removal from `cands` of every candidate that no
    /// completion keeping the constraints can hold.  Returns whether the node is hopeless (then `cands` is
    /// partly cut and should be dropped).
    ///
    /// A completion is a set `F ⊆ C`, and keeps the constraints when `P ∪ F` does.  Write `cP[r] = c_r(P)` and
    /// `cC[r] = c_r(C)` for the primes `r` of the pairs; the sums over `P ∪ F` are `c_r(P ∪ F) ≥ cP[r]` and, for
    /// `F ⊆ C`, `c_r(P ∪ F) ≤ cP[r] + cC[r]`.  Each round:
    ///
    /// 1. If for a pair `(p, q)` we have `cP[p] > cP[q] + cC[q]`, the node is hopeless: every completion has
    ///    `c_p(P ∪ F) ≥ cP[p] > cP[q] + cC[q] ≥ c_q(P ∪ F)`, which breaks that pair's constraint.
    /// 2. Otherwise, for each prime `p` that is the smaller prime of some pair, the *limit* of `p` is the least of
    ///    `cP[q] + cC[q] − cP[p]` over the pairs `(p, q)`.  A candidate `x = m·p` with `m` above the limit is
    ///    removed: a completion holding it has `c_p(P ∪ F) ≥ cP[p] + m > cP[q] + cC[q] ≥ c_q(P ∪ F)` for the pair
    ///    `(p, q)` that gave the limit.  (A candidate that is also in `P` changes no set `P ∪ F` and is left alone.)
    /// 3. If anything was removed, go round again: the removal lowers `cC` of `p`, which matters to the pairs whose
    ///    larger prime is `p` (steps 1 and 2), and `cC` of any other prime of pairs that divides `x`.  Otherwise
    ///    stop: the node is not hopeless as far as these sums can tell.
    ///
    /// A removal takes out only candidates that no completion keeping the constraints holds, so the completions
    /// that keep them are the same before and after, and so are the heaviest sets among them.  `P` and `C` are
    /// disjoint in the search; the sums are computed without assuming it.
    pub fn exclude<const W: usize>(&self, s: &mut RuleScratch, promoted: &Bits<W>, cands: &mut Bits<W>) -> bool {
        self.sums(&mut s.promoted, promoted);
        loop {
            self.sums(&mut s.cands, cands);
            if self.some_pair_is_out_of_reach(s) {
                return true;
            }
            s.limit.fill(i64::MAX);
            for &(p, q) in &self.links {
                let (p, q) = (usize::from(p), usize::from(q));
                s.limit[p] = s.limit[p].min(s.promoted[q] + s.cands[q] - s.promoted[p]);
            }
            let mut removed = false;
            for (family, &limit) in self.family.iter().zip(&s.limit) {
                for &(x, m) in family {
                    if i64::from(m) > limit && cands.contains(x) && !promoted.contains(x) {
                        cands.remove(x);
                        removed = true;
                    }
                }
            }
            if !removed {
                return false;
            }
        }
    }

    /// `out[i]` = the cofactor sum `c_r(set)` of the `i`-th prime `r` of the pairs.
    fn sums<const W: usize>(&self, out: &mut [i64], set: &Bits<W>) {
        for (sum, family) in out.iter_mut().zip(&self.family) {
            *sum = cofactor_sum(family, set);
        }
    }

    /// Whether for some pair `(p, q)` the sums in `s` give `cP[p] > cP[q] + cC[q]` (step 1 of `exclude`).
    fn some_pair_is_out_of_reach(&self, s: &RuleScratch) -> bool {
        self.links
            .iter()
            .any(|&(p, q)| s.promoted[usize::from(p)] > s.promoted[usize::from(q)] + s.cands[usize::from(q)])
    }
}

/// The sum of `x / r` over the members `x` of `set` among the candidates `family` divisible by `r` (as a
/// [`Symmetries`] lists them): `c_r(set)` for a set of candidates.
fn cofactor_sum<const W: usize>(family: &[(u32, u32)], set: &Bits<W>) -> i64 {
    family.iter().filter(|&&(x, _)| set.contains(x)).map(|&(_, m)| i64::from(m)).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::game::Optimal;
    use crate::model::Scratch;
    use crate::search::{Prepared, SearchTactics};
    use crate::test_support::Rng;

    /// The board `1..=n` set up as `taxman solve` sets it up, or `None` if nothing on it can be promoted.
    fn setup<const W: usize>(n: u32) -> Option<Prepared<W>> {
        let board = Board::<W>::for_n(n);
        (!board.is_trivial()).then(|| board.prepare(SearchTactics::default()).expect("prepare"))
    }

    /// The definition of a symmetry of the root, with every number looked at: `pi` maps every frame's moves and
    /// factors onto themselves, the candidates onto themselves, and the factor graph among the frames' numbers onto
    /// itself.
    fn brute_symmetry<const W: usize>(p: &Prepared<W>, pi: &impl Fn(u32) -> u32) -> bool {
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

    /// The definition of the symmetric pairs, with every pair of primes up to `n / 2` tried.
    fn brute_pairs<const W: usize>(p: &Prepared<W>) -> Vec<(u32, u32)> {
        let primes = numtheory::sieve(p.n / 2).1;
        let simple = |a: u32, b: u32| {
            p.candidates
                .iter()
                .all(|x| !x.is_multiple_of(a * b) && !x.is_multiple_of(a * a) && !x.is_multiple_of(b * b))
        };
        let mut pairs = Vec::new();
        for (i, &a) in primes.iter().enumerate() {
            if !p.candidates.iter().any(|x| x.is_multiple_of(a)) {
                continue;
            }
            for &b in &primes[i + 1..] {
                if simple(a, b) && brute_symmetry(p, &|x| swap_primes(x, a, b)) {
                    pairs.push((a, b));
                }
            }
        }
        pairs
    }

    /// Checks `root_pairs` of the board `n` against the definition, and returns the number of pairs.
    fn pairs_agree<const W: usize>(n: u32) -> usize {
        let Some(p) = setup::<W>(n) else { return 0 };
        let pairs = root_pairs(&p.model, &p.root, &p.candidates);
        assert_eq!(pairs, brute_pairs(&p), "n={n}");
        pairs.len()
    }

    #[test]
    #[cfg_attr(debug_assertions, ignore = "long: the quick check is root_pairs_is_the_definition_on_the_small_boards")]
    fn root_pairs_is_the_definition() {
        let (mut games, mut pairs) = (0, 0);
        for n in (2..=300).chain((301..=2200).step_by(53)).chain([1300, 2175]) {
            let found = crate::with_width!(n, W => pairs_agree::<W>(n));
            games += usize::from(found > 0);
            pairs += found;
        }
        assert!(games > 150 && pairs > 500, "{pairs} pairs in {games} games");
    }

    /// [`root_pairs_is_the_definition`] on the boards 2 to 150 and 1300.
    #[test]
    fn root_pairs_is_the_definition_on_the_small_boards() {
        let (mut games, mut pairs) = (0, 0);
        for n in (2..=150).chain([1300]) {
            let found = crate::with_width!(n, W => pairs_agree::<W>(n));
            games += usize::from(found > 0);
            pairs += found;
        }
        assert!(games > 25 && pairs > 50, "{pairs} pairs in {games} games");
    }

    #[test]
    fn swap_primes_exchanges_exponents_and_never_overflows() {
        assert_eq!(swap_primes(2 * 2 * 7 * 11, 2, 7), 7 * 7 * 2 * 11);
        assert_eq!(swap_primes(13, 2, 7), 13);
        assert_eq!(swap_primes(swap_primes(2 * 2 * 3 * 7, 3, 7), 3, 7), 2 * 2 * 3 * 7);
        // 2^6 · 61 would become 2 · 61^6, more than a u32 holds.
        assert_eq!(swap_primes(64 * 61, 2, 61), u32::MAX);
    }

    fn pairs_of<const W: usize>(n: u32) -> Vec<(u32, u32)> {
        let p = setup::<W>(n).expect("not trivial");
        root_pairs(&p.model, &p.root, &p.candidates)
    }

    #[test]
    fn the_pairs_of_1300() {
        let classes = [
            vec![41, 43],
            vec![59, 61],
            vec![67, 71],
            vec![73, 79],
            vec![89, 97, 101, 103, 107],
            vec![109, 113, 127, 131, 137, 139],
            vec![149, 151, 157],
        ];
        let mut expected = Vec::new();
        for class in &classes {
            for (i, &p) in class.iter().enumerate() {
                expected.extend(class[i + 1..].iter().map(|&q| (p, q)));
            }
        }
        expected.sort_unstable();
        assert_eq!(expected.len(), 32);
        assert_eq!(crate::with_width!(1300, W => pairs_of::<W>(1300)), expected);
    }

    /// A maximal set of candidates that can be promoted from the root: the candidates in a random order, each kept
    /// if it can be promoted after those kept.
    fn random_maximal<const W: usize>(p: &Prepared<W>, scratch: &mut Scratch<W>, rng: &mut Rng) -> Bits<W> {
        let mut order = p.candidates.to_vec();
        for i in (1..order.len()).rev() {
            order.swap(i, rng.below(i as u64 + 1) as usize);
        }
        let mut now = p.root;
        order.into_iter().filter(|&x| p.model.promote_in_place_using(scratch, &mut now, x)).collect()
    }

    /// Checks that the image of `set` under every pair's exchange can be promoted and is worth the formula more.
    fn check_images<const W: usize>(p: &Prepared<W>, pairs: &[(u32, u32)], scratch: &mut Scratch<W>, set: &Bits<W>) {
        let n = p.n;
        for &(a, b) in pairs {
            let image = map_set(set, &|x| swap_primes(x, a, b)).expect("in range");
            assert!(p.model.promote_all_using(scratch, &p.root, &image).is_some(), "n={n}: ({a}, {b}) of {set:?}");
            let gain = i64::from(b - a) * (cofactors(set, a) as i64 - cofactors(set, b) as i64);
            assert_eq!(image.sum() as i64 - set.sum() as i64, gain, "n={n}: the sum of the image of {set:?}");
        }
    }

    /// How many games `images_on` checked, and in how many of them it enumerated every promotable set.
    #[derive(Default)]
    struct Covered {
        games: usize,
        enumerated: usize,
    }

    fn images_on<const W: usize>(n: u32, optimal: &Optimal, covered: &mut Covered) {
        let Some(p) = setup::<W>(n) else { return };
        let symmetries = Symmetries::of(&p.model, &p.root, &p.candidates);
        if symmetries.is_empty() {
            return;
        }
        covered.games += 1;
        let pairs = symmetries.pairs();
        let mut scratch = Scratch::new();
        let moves = optimal.moves(n).expect("a known game");
        let best: Bits<W> = moves.iter().copied().filter(|&x| p.candidates.contains(x)).collect();
        assert!(p.model.promote_all_using(&mut scratch, &p.root, &best).is_some(), "n={n}: an optimal set");
        check_images(&p, pairs, &mut scratch, &best);
        assert!(symmetries.keeps(&best), "n={n}: the optimal set breaks a constraint");
        let mut rng = Rng(0x5eed ^ u64::from(n));
        for _ in 0..20 {
            let set = random_maximal(&p, &mut scratch, &mut rng);
            check_images(&p, pairs, &mut scratch, &set);
        }
        if p.candidates.len() > 22 {
            return;
        }
        // Every promotable set, by depth-first search over the candidates in order.
        covered.enumerated += 1;
        let cands = p.candidates.to_vec();
        let mut best_kept = None;
        let mut stack = vec![(0usize, Bits::<W>::EMPTY, p.root)];
        while let Some((i, set, state)) = stack.pop() {
            if i == cands.len() {
                check_images(&p, pairs, &mut scratch, &set);
                if symmetries.keeps(&set) {
                    best_kept = best_kept.max(Some(set.sum()));
                }
                continue;
            }
            stack.push((i + 1, set, state));
            if let Some(next) = p.model.promote(&state, cands[i]) {
                stack.push((i + 1, set.with(cands[i]), next));
            }
        }
        assert_eq!(best_kept, Some(best.sum()), "n={n}: the sets that keep the constraints miss P*");
    }

    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is images_are_promotable_and_worth_the_formula_on_the_small_boards"
    )]
    fn images_are_promotable_and_worth_the_formula() {
        let (optimal, mut covered) = (Optimal::built_in(), Covered::default());
        for n in (2..=400).chain((401..=1300).step_by(25)) {
            crate::with_width!(n, W => images_on::<W>(n, optimal, &mut covered));
        }
        assert!(
            covered.games > 150 && covered.enumerated > 10,
            "{} games, {} enumerated",
            covered.games,
            covered.enumerated
        );
    }

    /// [`images_are_promotable_and_worth_the_formula`] on the boards 2 to 150, and every 100th to 1300.
    #[test]
    fn images_are_promotable_and_worth_the_formula_on_the_small_boards() {
        let (optimal, mut covered) = (Optimal::built_in(), Covered::default());
        for n in (2..=150).chain((400..=1300).step_by(100)) {
            crate::with_width!(n, W => images_on::<W>(n, optimal, &mut covered));
        }
        assert!(
            covered.games > 30 && covered.enumerated > 10,
            "{} games, {} enumerated",
            covered.games,
            covered.enumerated
        );
    }

    /// How the cases of `the_rule_is_its_definition` came out.
    #[derive(Debug, Default)]
    struct Tally {
        cases: usize,
        /// `hopeless` said yes.
        hopeless: usize,
        /// `exclude` said yes, though `hopeless` said no: removals made the node hopeless.
        cut_after_removals: usize,
        /// `exclude` removed candidates and the node was not hopeless.
        removals: usize,
        /// Neither said or did anything.
        nothing: usize,
        /// The cases on games whose pairs are independent, where the rule is exact.
        exact: usize,
    }

    /// Whether the pairs of `symmetries` share no prime, and no candidate is divisible by primes of two of them
    /// (or of both primes of one): then each pair's constraint can be met on its own multiples, and the rule is
    /// exact.
    fn independent<const W: usize>(symmetries: &Symmetries, candidates: &Bits<W>) -> bool {
        let primes: Vec<u32> = symmetries.pairs().iter().flat_map(|&(p, q)| [p, q]).collect();
        let mut distinct = primes.clone();
        distinct.sort_unstable();
        distinct.dedup();
        distinct.len() == primes.len()
            && candidates.iter().all(|x| primes.iter().filter(|&&r| x.is_multiple_of(r)).count() <= 1)
    }

    /// Checks the rule at one node against its definition: enumerates the completions `F ⊆ cands` that make
    /// `promoted ∪ F` keep every pair's constraint, over the candidates divisible by a prime of a pair (the others
    /// change no cofactor sum).
    fn check_rule<const W: usize>(
        symmetries: &Symmetries,
        independent: bool,
        promoted: &Bits<W>,
        cands: &Bits<W>,
        tally: &mut Tally,
    ) {
        let pairs = symmetries.pairs();
        let related: Vec<u32> =
            cands.iter().filter(|x| pairs.iter().any(|&(p, q)| x.is_multiple_of(p) || x.is_multiple_of(q))).collect();
        assert!(related.len() <= 14);
        assert!(promoted.is_disjoint(cands));
        // c_r(P ∪ F) = c_r(P) + c_r(F), as P and F are disjoint.
        let base: Vec<(i64, i64)> =
            pairs.iter().map(|&(p, q)| (cofactors(promoted, p) as i64, cofactors(promoted, q) as i64)).collect();
        let keeping: Vec<u32> = (0u32..1 << related.len())
            .filter(|&mask| {
                let f: Bits<W> =
                    related.iter().enumerate().filter(|&(i, _)| mask >> i & 1 == 1).map(|(_, &x)| x).collect();
                pairs
                    .iter()
                    .zip(&base)
                    .all(|(&(p, q), &(c_p, c_q))| c_p + cofactors(&f, p) as i64 <= c_q + cofactors(&f, q) as i64)
            })
            .collect();
        let mut scratch = symmetries.scratch();
        let hopeless = symmetries.hopeless(&mut scratch, promoted, cands);
        let mut after = *cands;
        let cut = symmetries.exclude(&mut scratch, promoted, &mut after);
        let removed = *cands - after;
        let what = format!("P={promoted:?} C={cands:?} -> {after:?}");

        // Sound: a node the rule gives up has no completion that keeps the constraints, and a candidate it removes
        // is in none that does.
        assert!(!hopeless || cut, "exclude finds the node hopeless that hopeless does: {what}");
        if hopeless || cut {
            assert!(keeping.is_empty(), "a hopeless node with a completion that keeps the constraints: {what}");
        } else {
            assert_eq!(after | removed, *cands, "{what}");
            for (i, x) in related.iter().enumerate() {
                if removed.contains(*x) {
                    assert!(
                        keeping.iter().all(|&mask| mask >> i & 1 == 0),
                        "{x} is in a completion that keeps: {what}"
                    );
                }
            }
            assert!(removed.iter().all(|x| related.contains(&x)), "{what}");
        }
        tally.cases += 1;
        match (hopeless, cut, removed.is_empty()) {
            (true, ..) => tally.hopeless += 1,
            (false, true, _) => tally.cut_after_removals += 1,
            (false, false, false) => tally.removals += 1,
            (false, false, true) => tally.nothing += 1,
        }
        if !independent {
            return;
        }
        // Exact where the pairs are independent: the node is hopeless exactly when no completion keeps the
        // constraints, and a candidate is removed exactly when none that does holds it.
        tally.exact += 1;
        assert_eq!(cut, keeping.is_empty(), "{what}");
        assert_eq!(hopeless, keeping.is_empty(), "{what}");
        if !cut {
            for (i, x) in related.iter().enumerate() {
                let held = keeping.iter().any(|&mask| mask >> i & 1 == 1);
                assert_eq!(!removed.contains(*x), held, "{x}: {what}");
            }
        }
    }

    fn rule_on<const W: usize>(n: u32, cases: usize, tally: &mut Tally) {
        let Some(p) = setup::<W>(n) else { return };
        let symmetries = Symmetries::of(&p.model, &p.root, &p.candidates);
        if symmetries.is_empty() {
            return;
        }
        let independent = independent(&symmetries, &p.candidates);
        let pairs = symmetries.pairs();
        let mut rng = Rng(0xbeef ^ u64::from(n));
        for _ in 0..cases {
            // P holds many of the multiples of the smaller primes of pairs, or few, and some of the others, so
            // that the pairs' constraints are near what the candidates can reach.
            let (small, others) = (rng.pick(&[5, 20, 50]), rng.pick(&[0, 5, 20]));
            let promoted: Bits<W> = p
                .candidates
                .iter()
                .filter(|&x| rng.chance(if pairs.iter().any(|&(a, _)| x.is_multiple_of(a)) { small } else { others }))
                .collect();
            let rest = p.candidates - promoted;
            let (mut related, other): (Vec<u32>, Vec<u32>) =
                rest.iter().partition(|x| pairs.iter().any(|&(a, b)| x.is_multiple_of(a) || x.is_multiple_of(b)));
            for i in (1..related.len()).rev() {
                related.swap(i, rng.below(i as u64 + 1) as usize);
            }
            related.truncate(rng.below(15) as usize);
            let mut cands: Bits<W> = related.into_iter().collect();
            cands.extend(other.into_iter().filter(|_| rng.chance(50)));
            check_rule(&symmetries, independent, &promoted, &cands, tally);
        }
    }

    #[test]
    fn the_rule_is_its_definition() {
        let mut tally = Tally::default();
        for n in (2..=300).chain([1300, 2175]) {
            let cases = if n > 300 { 100 } else { 8 };
            crate::with_width!(n, W => rule_on::<W>(n, cases, &mut tally));
        }
        // The cases cover the ways the rule answers (but for removals that make a node hopeless, which random nodes
        // rarely reach: the next test builds one), and the games where it is exact.
        assert!(tally.cases >= 300, "{tally:?}");
        assert!(tally.hopeless > 0, "no hopeless node: {tally:?}");
        assert!(tally.removals > 0 && tally.nothing > 0, "no node left open: {tally:?}");
        assert!(tally.exact > 0, "no game with independent pairs: {tally:?}");
    }

    #[test]
    fn removals_can_make_a_node_hopeless() {
        // The pairs (5, 7), (5, 11) and (7, 11), the candidates 20 = 5·4, 42 = 7·6 and 44 = 11·4, and 20 promoted.
        // No pair is out of reach at first (the cofactor of 5 is 4, and 7 and 11 can reach 6 and 4), but 42 would
        // lift 7 above everything 11 can reach, so it goes; then 7 cannot reach the 4 that 5 has.
        let candidates = Bits::<8>::from_slice(&[20, 42, 44]);
        let symmetries = Symmetries::with_pairs(vec![(5, 7), (5, 11), (7, 11)], &candidates);
        let (promoted, cands) = (Bits::<8>::from_slice(&[20]), Bits::<8>::from_slice(&[42, 44]));
        assert!(!symmetries.hopeless(&mut symmetries.scratch(), &promoted, &cands));
        let mut tally = Tally::default();
        check_rule(&symmetries, false, &promoted, &cands, &mut tally);
        assert_eq!(tally.cut_after_removals, 1, "{tally:?}");
    }

    #[test]
    fn the_rule_is_sound_but_not_exact_where_pairs_pull_against_each_other() {
        // The same chain of pairs, 60 = 5·12 promoted, and the candidates 42 = 7·6, 56 = 7·8 and 143 = 11·13.  The
        // constraint of (5, 7) needs the cofactor of 7 to reach 12, which only 6 + 8 does, and then (7, 11) needs
        // 11 to reach 14, which it cannot (13).  No completion keeps the constraints, but each sum alone is
        // within reach, so the rule leaves the node open and takes nothing out.
        let candidates = Bits::<8>::from_slice(&[60, 42, 56, 143]);
        let symmetries = Symmetries::with_pairs(vec![(5, 7), (5, 11), (7, 11)], &candidates);
        let (promoted, cands) = (Bits::<8>::from_slice(&[60]), Bits::<8>::from_slice(&[42, 56, 143]));
        let mut after = cands;
        assert!(!symmetries.exclude(&mut symmetries.scratch(), &promoted, &mut after));
        assert_eq!(after, cands);
        for mask in 0u32..8 {
            let chosen = cands.iter().enumerate().filter(|&(i, _)| mask >> i & 1 == 1).map(|(_, x)| x);
            assert!(!symmetries.keeps(&(promoted | chosen.collect::<Bits<8>>())), "completion {mask:03b}");
        }
    }
}
