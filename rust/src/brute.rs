//! Exact Taxman on small boards, with holes, by exhaustive search over pot states: no frame model.
//!
//! A state is the pot (a `u128`, bit `i` for the number `i`), a move is any number in the pot that can pay tax,
//! the tax is every proper divisor still in the pot, and the value of a state is the best sum of moves from it,
//! memoized.  It is the Rust counterpart of the Python certifier (`heuristics/evaluation/certify.py`), extended
//! to boards with holes and to the maximal-factor rule, and it shares no code with the frame model or with
//! [`crate::numtheory`].
//!
//! A *board* is `1..=n` minus a set of *holes*: numbers that are never in the pot, so they are never moves and
//! never tax.  [`Rule`] says what a move needs:
//!
//! * [`Rule::Divisor`], the rules of the game: some proper divisor of the move is in the pot.
//! * [`Rule::MaximalFactor`]: some *maximal factor* `m/p` (`p` prime) of the move is in the pot.  On a board
//!   without holes the two rules are the same game (the lifting argument: a divisor `d` in the pot lies below a
//!   maximal factor `f`, `d | f | m`, and whatever removed `f` from the pot removed `d` with it).  With holes they
//!   differ: a move whose maximal factor `f` is a hole may still pay with a divisor of `f` under the rules, but
//!   not under this rule.  This is the game the frame model plays on a punctured board (see
//!   [`crate::sacrifice`]).

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

/// The largest board [`solve`] handles: a pot is a `u128`.
pub const MAX_N: u32 = 127;

/// What a move must find in the pot to be legal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// The rules of Taxman: some proper divisor of the move.
    Divisor,
    /// Some maximal factor of the move: `m/p` for a prime `p` dividing `m`.
    MaximalFactor,
}

/// An optimal game on a board, and how much work it took.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exact {
    pub score: u64,
    /// A game that scores `score`.
    pub moves: Vec<u32>,
    /// Distinct pot states evaluated.
    pub states: usize,
}

/// The best game on the board `1..=n` without `holes`, under `rule`.  Panics if `n > MAX_N`.
pub fn solve(n: u32, holes: &[u32], rule: Rule) -> Exact {
    let mut solver = Solver::new(n, holes, rule);
    let root = solver.normalize(solver.board);
    let score = solver.best(root);
    // Walk down the memo: at each state some move reaches the state's value.
    let mut moves = Vec::new();
    let mut pot = root;
    let mut left = score;
    while left > 0 {
        let step = members_desc(solver.pickable(pot)).find_map(|c| {
            let child = solver.after(pot, c);
            (c + solver.best(child) == left).then_some((c, child))
        });
        let (c, child) = step.expect("a state's value is reached by one of its moves");
        moves.push(c);
        pot = child;
        left -= c;
    }
    Exact { score: u64::from(score), moves, states: solver.memo.len() }
}

/// The search: the board's relations as masks, and the memo.
struct Solver {
    /// The numbers on the board.
    board: u128,
    /// For each number, what can pay for it (on the board).
    pay: Vec<u128>,
    /// For each number, the numbers it can pay for (on the board): the mirror image of `pay`.
    payers: Vec<u128>,
    /// For each number, its proper divisors: what playing it sweeps from the pot.
    sweep: Vec<u128>,
    memo: HashMap<u128, u32, BuildHasherDefault<MixHasher>>,
}

impl Solver {
    fn new(n: u32, holes: &[u32], rule: Rule) -> Self {
        assert!(n <= MAX_N, "the brute force handles boards up to {MAX_N}, not {n}");
        let size = n as usize + 1;
        let board = (1..=n).filter(|x| !holes.contains(x)).fold(0u128, |acc, x| acc | bit(x));
        let mut sweep = vec![0u128; size];
        for d in 1..=n / 2 {
            for m in (2 * d..=n).step_by(d as usize) {
                sweep[m as usize] |= bit(d);
            }
        }
        let is_prime = |p: u32| p >= 2 && (2..p).take_while(|q| q * q <= p).all(|q| !p.is_multiple_of(q));
        let mut pay = vec![0u128; size];
        let mut payers = vec![0u128; size];
        for m in members(board) {
            let can_pay = match rule {
                Rule::Divisor => sweep[m as usize],
                Rule::MaximalFactor => {
                    (2..=m).filter(|&p| m % p == 0 && is_prime(p)).fold(0u128, |acc, p| acc | bit(m / p))
                }
            } & board;
            pay[m as usize] = can_pay;
            for f in members(can_pay) {
                payers[f as usize] |= bit(m);
            }
        }
        Solver { board, pay, payers, sweep, memo: HashMap::default() }
    }

    /// The numbers in `pot` that can be played now.
    fn pickable(&self, pot: u128) -> u128 {
        members(pot).filter(|&c| self.pay[c as usize] & pot != 0).fold(0, |acc, c| acc | bit(c))
    }

    /// `pot` without the numbers that can never matter again: those that cannot be played and cannot pay for a
    /// move.  (The pot only shrinks, so neither can change; removing one can make others inert in turn.)
    fn normalize(&self, mut pot: u128) -> u128 {
        loop {
            let live = |c: u32| (self.pay[c as usize] | self.payers[c as usize]) & pot != 0;
            let keep = members(pot).filter(|&c| live(c)).fold(0, |acc, c| acc | bit(c));
            if keep == pot {
                return pot;
            }
            pot = keep;
        }
    }

    /// The pot after playing `c`.
    fn after(&self, pot: u128, c: u32) -> u128 {
        self.normalize(pot & !(self.sweep[c as usize] | bit(c)))
    }

    /// An upper bound on the value of `pot`: every move takes itself and at least one other number out of the
    /// pot, so there are at most `|pot| / 2` of them, all among the numbers that can be played now.
    fn bound(&self, pot: u128) -> u32 {
        let most = pot.count_ones() / 2;
        members_desc(self.pickable(pot)).take(most as usize).sum()
    }

    /// The value of `pot`, exactly: a move is skipped only when its bound cannot beat the best move so far, so
    /// what is memoized is the true value.
    fn best(&mut self, pot: u128) -> u32 {
        if let Some(&value) = self.memo.get(&pot) {
            return value;
        }
        let mut value = 0;
        for c in members_desc(self.pickable(pot)) {
            let child = self.after(pot, c);
            if c + self.bound(child) <= value {
                continue;
            }
            value = value.max(c + self.best(child));
        }
        self.memo.insert(pot, value);
        value
    }
}

fn bit(x: u32) -> u128 {
    1u128 << x
}

/// The members of `set`, ascending.
fn members(mut set: u128) -> impl Iterator<Item = u32> {
    std::iter::from_fn(move || {
        (set != 0).then(|| {
            let x = set.trailing_zeros();
            set &= set - 1;
            x
        })
    })
}

/// The members of `set`, descending.
fn members_desc(mut set: u128) -> impl Iterator<Item = u32> {
    std::iter::from_fn(move || {
        (set != 0).then(|| {
            let x = 127 - set.leading_zeros();
            set &= !bit(x);
            x
        })
    })
}

/// A fast hasher for the memo's `u128` keys (multiply and fold; the default SipHash is several times slower).
#[derive(Default)]
struct MixHasher(u64);

impl Hasher for MixHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (self.0.rotate_left(8) ^ u64::from(b)).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
    }

    fn write_u128(&mut self, x: u128) {
        let folded = (x as u64) ^ ((x >> 64) as u64).rotate_left(29);
        let h = (self.0 ^ folded).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        self.0 = h ^ (h >> 31);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{replay, Optimal};
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn small_boards_match_optimal_json() {
        let optimal = Optimal::load_default().unwrap();
        for n in 1..=40 {
            let exact = solve(n, &[], Rule::Divisor);
            assert_eq!(Some(exact.score), optimal.score(n), "n={n}");
            assert_eq!(replay(n, &exact.moves), Ok(exact.score), "n={n}");
        }
    }

    /// A naive solver for sparse boards, sharing nothing with [`solve`]: the pot as a sorted vector, memoized.
    fn naive(pot: &[u32], rule: Rule, memo: &mut BTreeMap<Vec<u32>, u32>) -> u32 {
        if let Some(&value) = memo.get(pot) {
            return value;
        }
        let is_prime = |p: u32| p >= 2 && (2..p).all(|q| !p.is_multiple_of(q));
        let mut best = 0;
        for &c in pot {
            let pays = pot.iter().any(|&d| d < c && c % d == 0 && (rule == Rule::Divisor || is_prime(c / d)));
            if pays {
                let next: Vec<u32> = pot.iter().copied().filter(|&d| d != c && c % d != 0).collect();
                best = best.max(c + naive(&next, rule, memo));
            }
        }
        memo.insert(pot.to_vec(), best);
        best
    }

    /// The other tests stop below 65, the low half of the pot's `u128`.  Sparse boards that reach 127 (the numbers
    /// above 64 are in bits of the upper half) are cheap, and checked against a naive solver under both rules.
    #[test]
    fn sparse_boards_up_to_the_top_of_the_u128_match_a_naive_solver() {
        let mut state = 0x1234_5678_9ABC_DEF1u64;
        let mut next = move |below: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % below
        };
        for round in 0..60 {
            let n = if round % 3 == 0 { MAX_N } else { 70 + next(58) as u32 };
            let mut present: BTreeSet<u32> = BTreeSet::new();
            while present.len() < 8 + next(7) as usize {
                let x = if next(2) == 0 { 1 + next(12) as u32 } else { n - next(40) as u32 };
                if (1..=n).contains(&x) {
                    present.insert(x);
                }
            }
            let holes: Vec<u32> = (1..=n).filter(|x| !present.contains(x)).collect();
            let pot: Vec<u32> = present.into_iter().collect();
            for rule in [Rule::Divisor, Rule::MaximalFactor] {
                let expected = naive(&pot, rule, &mut BTreeMap::new());
                assert_eq!(solve(n, &holes, rule).score, u64::from(expected), "n={n} {rule:?} {pot:?}");
            }
        }
    }

    #[test]
    fn the_rules_agree_without_holes() {
        for n in 1..=36 {
            assert_eq!(solve(n, &[], Rule::Divisor).score, solve(n, &[], Rule::MaximalFactor).score, "n={n}");
        }
    }

    #[test]
    fn holes_are_never_moves_or_tax() {
        // Without 1 no prime can be played: on 1..=7 the best is 4 (paid by 2) then 6 (paid by 3).
        assert_eq!(solve(7, &[1], Rule::Divisor).score, 10);
        // Without 2, 4 can still pay with 1 under the rules (4 alone: 3 would take the 1 it needs), but not
        // under the maximal-factor rule, which leaves only 3.
        assert_eq!(solve(4, &[2], Rule::Divisor).moves, [4]);
        assert_eq!(solve(4, &[2], Rule::MaximalFactor).moves, [3]);
        // The game found is legal on the full board too, and never plays a hole.
        let exact = solve(30, &[6], Rule::MaximalFactor);
        assert!(!exact.moves.contains(&6));
        assert_eq!(replay(30, &exact.moves), Ok(exact.score));
    }
}
