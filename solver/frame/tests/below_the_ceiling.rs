//! The theorems of docs/theory/below-the-ceiling.md against the committed chain: the 8p theorem (L2) and the 12p
//! theorem (L4).
//!
//! Each bounds the rise of the best score at a game n = kp, p a prime, with r = q(p − 1), the largest prime below p:
//! opt(n) ≤ opt(n − 1) + n − jr, with j = 4 at n = 8p and j = 6 at n = 12p.  At every game of the committed chain that
//! a theorem covers, opt(n) must be the bound, and the recorded optimal set of game n − 1 with jr exchanged for n (the
//! exchange) must play by the rules ([`replay`]) to the bound, in the order of its peel ([`play_order`], the oracle of
//! `src/playable.rs`, which uses nothing of the frame model).  A theorem is not a measurement: a game that breaks one
//! would mean an error in its proof or in the data.  Where a game fails a hypothesis, a test shows the bound failing.
//!
//! * [`the_8p_theorem_holds_on_the_committed_chain`]: the 8p theorem covers n = 8p ≥ 187 with 9r > n.  The bound at
//!   every such game, and the exchange to game [`EXCHANGES_QUICK_TO`];
//!   [`the_8p_exchange_reaches_the_bound_on_the_rest_of_the_chain`] takes the exchanges past it, and
//!   [`the_8p_bound_fails_at_184`] shows the bound failing at a game that meets neither hypothesis.
//! * [`the_12p_theorem_holds_on_the_committed_chain`]: the 12p theorem covers n = 12p ≥ 187 with 15r > n when some
//!   optimal set of game n holds 8p, 8r, 12r and 8t, for a prime t with 4p/3 < t ≤ 3p/2; the tests ask that of the
//!   recorded optimal set of game n.  Likewise [`the_12p_exchange_reaches_the_bound_on_the_rest_of_the_chain`], and
//!   [`the_12p_bound_fails_at_204`], at a game that fails only 15r > n.
//!   [`the_cycle_lemma_rules_out_the_cheaper_exchange_at_372`] checks the example of the cycle lemma (L3).

mod common;

use common::{play_order, playable};
use std::time::Instant;
use taxman::game::{replay, Optimal};
use taxman::numtheory::{sieve, Tables};

/// The quick tests replay the exchanges of the games up to this one, and the long tests those of the games above it.
/// The peel's order of a set costs about the square of its size ([`play_order`]), so the exchanges to here cost about a
/// fifteenth of them all.
const EXCHANGES_QUICK_TO: u32 = 2000;

/// A game n = kp of one of the theorems, p a prime, with r = q(p − 1), the largest prime below p.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Game {
    n: u32,
    p: u32,
    r: u32,
}

/// A theorem of docs/theory/below-the-ceiling.md: at n = `times`·p it bounds opt(n) by opt(n − 1) + n − `lost`·r.
struct Theorem {
    /// Its name and label, for the messages.
    name: &'static str,
    times: u32,
    lost: u32,
    /// Whether the theorem covers the game, with the committed chain's recorded optimal set of game n as the optimal
    /// set that the 12p theorem asks for.
    covers: fn(&Optimal, Game) -> bool,
}

/// The 8p theorem (L2).
const EIGHT_P: Theorem = Theorem { name: "the 8p theorem (L2)", times: 8, lost: 4, covers: covered_by_8p };

/// The 12p theorem (L4).
const TWELVE_P: Theorem = Theorem { name: "the 12p theorem (L4)", times: 12, lost: 6, covers: covered_by_12p };

/// Whether the 8p theorem covers the game: n ≥ 187 and 9r > n.
fn covered_by_8p(_: &Optimal, game: Game) -> bool {
    game.n >= 187 && 9 * game.r > game.n
}

/// Whether the 12p theorem covers the game with the recorded optimal set of game n: n ≥ 187 and 15r > n, and the set
/// holds 8p, 8r, 12r and 8t for a prime t with 4p/3 < t ≤ 3p/2.
fn covered_by_12p(table: &Optimal, game: Game) -> bool {
    let Game { n, p, r } = game;
    let set = table.moves(n).expect("the committed chain has game n");
    let holds = |x: u32| set.contains(&x);
    let spf = sieve(3 * p / 2).0;
    let mut eights = (4 * p / 3 + 1..=3 * p / 2).filter(|&t| spf[t as usize] == t).map(|t| 8 * t);
    n >= 187 && 15 * r > n && holds(8 * p) && holds(8 * r) && holds(12 * r) && eights.any(holds)
}

impl Theorem {
    /// The games n = times·p of the boards up to `last`, ascending, for the primes p from 3 on (2 has no prime below
    /// it).
    fn games(&self, last: u32) -> Vec<Game> {
        let primes = sieve(last / self.times).1;
        primes.windows(2).map(|pair| Game { n: self.times * pair[1], p: pair[1], r: pair[0] }).collect()
    }

    /// The bound on opt(n), opt(n − 1) + n − lost·r, with the committed score of game n − 1.
    fn bound(&self, table: &Optimal, game: Game) -> u64 {
        let before = table.score(game.n - 1).expect("the committed chain has game n - 1");
        before + u64::from(game.n) - u64::from(self.lost * game.r)
    }

    /// The exchange: the committed optimal set of game n − 1 with lost·r exchanged for n, in the order of its peel, or
    /// why there is none.
    fn exchange(&self, table: &Optimal, game: Game) -> Result<Vec<u32>, String> {
        let (n, given_up) = (game.n, self.lost * game.r);
        let before = table.moves(n - 1).expect("the committed chain has game n - 1");
        if !before.contains(&given_up) {
            return Err(format!("the optimal game of {} does not play {given_up}", n - 1));
        }
        let mut set: Vec<u32> = before.iter().copied().filter(|&x| x != given_up).collect();
        set.push(n);
        play_order(&Tables::new(n), &set).ok_or_else(|| "the exchange is not playable".to_owned())
    }

    /// The theorem at `game`, a game of the committed chain that it covers: opt(n) is the bound and, if `exchange`, the
    /// exchange plays by the rules to the bound.
    fn check(&self, table: &Optimal, game: Game, exchange: bool) -> Result<(), String> {
        let (bound, opt) = (self.bound(table, game), table.score(game.n).expect("the committed chain has game n"));
        if opt != bound {
            return Err(format!("opt(n) = {opt}, not the bound {bound}"));
        }
        if exchange {
            match replay(game.n, &self.exchange(table, game)?) {
                Ok(score) if score == bound => {}
                Ok(score) => return Err(format!("the exchange scores {score}, not the bound {bound}")),
                Err(e) => return Err(format!("the exchange is no game: {e}")),
            }
        }
        Ok(())
    }

    /// The theorem at the games of the committed chain that it covers and `games` keeps, with the exchange at those
    /// that `exchange` keeps.  Prints what it checked, and returns the games it checked, ascending, and the failures.
    fn check_games(&self, games: impl Fn(u32) -> bool, exchange: impl Fn(u32) -> bool) -> (Vec<u32>, Vec<String>) {
        let start = Instant::now();
        let table = Optimal::built_in();
        let all = self.games(table.max_n());
        let covered = all.into_iter().filter(|&game| games(game.n) && (self.covers)(table, game));
        let (mut checked, mut failures, mut exchanged) = (Vec::new(), Vec::new(), 0);
        for game in covered {
            let replayed = exchange(game.n);
            checked.push(game.n);
            exchanged += usize::from(replayed);
            if let Err(e) = self.check(table, game, replayed) {
                failures.push(format!("{}: n={} (p={}, r={}): {e}", self.name, game.n, game.p, game.r));
            }
        }
        let (first, last) = (checked.first().copied().unwrap_or(0), checked.last().copied().unwrap_or(0));
        println!(
            "{}: {} games, n from {first} to {last}, {exchanged} exchanges replayed, {} failures, {:.1?}",
            self.name,
            checked.len(),
            failures.len(),
            start.elapsed()
        );
        (checked, failures)
    }
}

// ---------------------------------------------------------------------------------------------------------
// The 8p theorem

/// The 8p theorem (L2, docs/theory/below-the-ceiling.md) on the committed chain: at every game n = 8p that it covers,
/// opt(n) is its bound opt(n − 1) + n − 4r, and to game [`EXCHANGES_QUICK_TO`] the committed optimal set of game n − 1
/// with 4r exchanged for n plays by the rules to the bound.  The first game it covers is 248 = 8·31: at 232 = 8·29,
/// r = 23 and 9r = 207 ≤ 232.
#[test]
fn the_8p_theorem_holds_on_the_committed_chain() {
    let (checked, failures) = EIGHT_P.check_games(|_| true, |n| n <= EXCHANGES_QUICK_TO);
    assert!(failures.is_empty(), "L2 fails on {} games:\n{}", failures.len(), failures.join("\n"));
    assert_eq!(checked.first(), Some(&248), "the first game that L2 covers");
    assert!(checked.last().is_some_and(|&n| n > EXCHANGES_QUICK_TO), "no game past {EXCHANGES_QUICK_TO}: {checked:?}");
}

/// [`the_8p_theorem_holds_on_the_committed_chain`] for the exchanges of the games above [`EXCHANGES_QUICK_TO`].
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is the_8p_theorem_holds_on_the_committed_chain")]
fn the_8p_exchange_reaches_the_bound_on_the_rest_of_the_chain() {
    let (checked, failures) = EIGHT_P.check_games(|n| n > EXCHANGES_QUICK_TO, |_| true);
    assert!(failures.is_empty(), "L2's exchange fails on {} games:\n{}", failures.len(), failures.join("\n"));
    assert!(!checked.is_empty(), "no game past {EXCHANGES_QUICK_TO}");
}

/// Some hypothesis of the 8p theorem (L2, docs/theory/below-the-ceiling.md) is needed: game 184 = 8·23, with r = 19,
/// meets neither (184 < 187, and 9r = 171 ≤ 184), and opt(184) is above the bound opt(183) + 184 − 76.
#[test]
fn the_8p_bound_fails_at_184() {
    let game = Game { n: 184, p: 23, r: 19 };
    assert!(EIGHT_P.games(184).contains(&game), "184 = 8·23, and 19 is the prime before 23");
    let table = Optimal::built_in();
    assert!(game.n < 187 && 9 * game.r <= game.n && !(EIGHT_P.covers)(table, game), "184 meets neither hypothesis");
    let (bound, opt) = (EIGHT_P.bound(table, game), table.score(game.n).expect("the committed chain has game 184"));
    assert!(opt > bound, "opt(184) = {opt}, not above the bound {bound}");
}

// ---------------------------------------------------------------------------------------------------------
// The 12p theorem

/// The 12p theorem (L4, docs/theory/below-the-ceiling.md) on the committed chain: at every game n = 12p that it covers
/// with the recorded optimal set of game n, opt(n) is its bound opt(n − 1) + n − 6r, and to game
/// [`EXCHANGES_QUICK_TO`] the committed optimal set of game n − 1 with 6r exchanged for n plays by the rules to the
/// bound.  The first game it covers so is 276 = 12·23: 228 = 12·19 has no prime t with 76/3 < t ≤ 57/2.
#[test]
fn the_12p_theorem_holds_on_the_committed_chain() {
    let (checked, failures) = TWELVE_P.check_games(|_| true, |n| n <= EXCHANGES_QUICK_TO);
    assert!(failures.is_empty(), "L4 fails on {} games:\n{}", failures.len(), failures.join("\n"));
    assert_eq!(checked.first(), Some(&276), "the first game that L4 covers with the recorded optimal sets");
    assert!(checked.last().is_some_and(|&n| n > EXCHANGES_QUICK_TO), "no game past {EXCHANGES_QUICK_TO}: {checked:?}");
}

/// [`the_12p_theorem_holds_on_the_committed_chain`] for the exchanges of the games above [`EXCHANGES_QUICK_TO`].
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is the_12p_theorem_holds_on_the_committed_chain")]
fn the_12p_exchange_reaches_the_bound_on_the_rest_of_the_chain() {
    let (checked, failures) = TWELVE_P.check_games(|n| n > EXCHANGES_QUICK_TO, |_| true);
    assert!(failures.is_empty(), "L4's exchange fails on {} games:\n{}", failures.len(), failures.join("\n"));
    assert!(!checked.is_empty(), "no game past {EXCHANGES_QUICK_TO}");
}

/// The hypothesis 15r > n of the 12p theorem (L4, docs/theory/below-the-ceiling.md) is needed: game 204 = 12·17, with
/// r = 13, fails it (15r = 195 ≤ 204) and meets the others, as n ≥ 187 and its recorded optimal set holds 8p = 136,
/// 8r = 104, 12r = 156 and 8t = 184 for the prime t = 23; and opt(204) is above the bound opt(203) + 204 − 78.
#[test]
fn the_12p_bound_fails_at_204() {
    let game = Game { n: 204, p: 17, r: 13 };
    assert!(TWELVE_P.games(204).contains(&game), "204 = 12·17, and 13 is the prime before 17");
    let table = Optimal::built_in();
    let set = table.moves(game.n).expect("the committed chain has game 204");
    assert!([136, 104, 156, 184].iter().all(|x| set.contains(x)), "the recorded optimal set of 204 holds the four");
    assert!(3 * 23 > 4 * game.p && 2 * 23 <= 3 * game.p, "23 lies between 4p/3 and 3p/2");
    assert!(game.n >= 187 && 15 * game.r <= game.n && !(TWELVE_P.covers)(table, game), "204 fails only 15r > n");
    let (bound, opt) = (TWELVE_P.bound(table, game), table.score(game.n).expect("the committed chain has game 204"));
    assert!(opt > bound, "opt(204) = {opt}, not above the bound {bound}");
}

/// The example of the cycle lemma (L3, docs/theory/below-the-ceiling.md): in game 372 = 12·31, with r = 29, giving up
/// 4t = 172, for the prime t = 43, would cost less than 6r = 174, but the recorded optimal set of game 371 with 172
/// exchanged for 372 holds 8p, 8r, 12p, 12r, 6p and 6r, so it is not playable.
#[test]
fn the_cycle_lemma_rules_out_the_cheaper_exchange_at_372() {
    let table = Optimal::built_in();
    let before = table.moves(371).expect("the committed chain has game 371");
    let mut set: Vec<u32> = before.iter().copied().filter(|&x| x != 172).collect();
    assert_eq!(set.len() + 1, before.len(), "the optimal game of 371 plays 172");
    set.push(372);
    for x in [248, 232, 372, 348, 186, 174] {
        assert!(set.contains(&x), "the exchange does not hold {x}");
    }
    assert!(!playable(&Tables::new(372), &set), "the exchange of 172 for 372 is playable");
}
