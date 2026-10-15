//! Which members of U\*_{≥3} every set beating a score must hold, certified by the parity bound.
//!
//! A *source* of game n is a number above n/2.  U\* is descending greedy's set of sources ([`crate::peel::u_star`]),
//! and every recorded optimal game plays exactly U\* above n/2.  The *layer* of x is Ω(x), its number of prime
//! factors with multiplicity; U\*_k holds the members of U\* in layer k, and U\*_{≥3} those in layers 3 and up.
//!
//! THEORY.md's Theorem B pins down the two bottom layers of every optimal set S of n: S holds the largest prime
//! q ≤ n and no other prime; among the sources of layer 2 it holds exactly U\*_2; and wherever U\*_2 spans every
//! prime up to n/2 (every n ≥ 187) it holds no number of layer 2 up to n/2 at all.  [`low_roles`] writes this down
//! as roles for the parity bound ([`crate::parity`], Theorem 10), which is a certified upper bound on the weight of
//! every playable set whose forced members (role `Pick`) are all in it and whose barred ones (role `Barred`) are
//! none of it.  So the bound at those roles bounds every playable set that obeys Theorem B.
//!
//! Now bar one member u of U\*_{≥3} as well.  If the bound is then at most V, a score, no playable set that obeys
//! Theorem B and lacks u is worth more than V; that is, every set beating V that obeys Theorem B holds u.  Call u
//! *certified* (for V).  If V is below opt(n), every optimal set beats V and obeys Theorem B, so it holds every
//! certified member.  If with u barred the roles cannot be met at all ([`Outcome::Infeasible`]) no set lacks u, and
//! u is certified for every V.  A member that is not certified is *open*, with the margin by which its bound
//! exceeds V.  The bound at the roles of Theorem B alone, the *root*, is at least V whenever a game worth V obeys
//! them, and its excess over V is the gap that the open members share out.
//!
//! The members certified for a lower V are certified for a higher one: the bound does not depend on V.
//!
//! The same roles start the searches of `bin/exact.rs` that measure what the sources of layer 3 and up cost.  A
//! search from [`low_roles`] still proves "no playable set beats V", since every optimal set obeys them (Theorem B).
//! A search from [`upper_fixed_roles`], which fixes the whole upper part to U\* as well, is the frame model's problem
//! and proves nothing about opt(n).  [`watched`] names the numbers whose branching and barring the search counts.

use crate::game::{Board, Role};
use crate::parity::{Meet, Outcome, Parity};
use crate::peel::u_star;
use crate::primes::is_prime;

/// From this board on, U\*_2 spans every prime up to n/2 (THEORY.md, Theorem B: the frames of layer 2 are closed on
/// every board from 187 to 8191; below 187 there are boards where they are not).  [`low_roles`] checks it.
const CLOSED_FROM: u32 = 187;

/// Ω(c): the number of prime factors of c counted with multiplicity (0 for c ≤ 1).
pub fn omega(c: u32, spf: &[u32]) -> u32 {
    let (mut rest, mut count) = (c, 0);
    while rest > 1 {
        rest /= spf[rest as usize];
        count += 1;
    }
    count
}

/// The sources a recorded game of n plays (its moves above n/2), ascending; for an optimal game, U\*.
pub fn played_sources(n: u64, moves: &[u32]) -> Vec<u32> {
    let mut sources: Vec<u32> = moves.iter().copied().filter(|&c| u64::from(c) > n / 2).collect();
    sources.sort_unstable();
    sources
}

/// U\* of a recorded game of n, ascending: the game's moves above n/2 ([`played_sources`]), checked against descending
/// greedy ([`u_star`], `spf` must reach n), which every recorded game plays.  Panics, naming n, if they differ.
pub fn recorded_ustar(n: u64, moves: &[u32], spf: &[u32]) -> Vec<u32> {
    let ustar = played_sources(n, moves);
    let mut expected = u_star(n, spf);
    expected.sort_unstable();
    assert_eq!(ustar, expected, "n={n}: the recorded game's moves above n/2 are not U*");
    ustar
}

/// Which numbers 0..=n are in U\*, given in any order (they must all be above n/2 and at most n).
fn in_ustar(n: u32, ustar: &[u32]) -> Vec<bool> {
    assert!(ustar.iter().all(|&u| n / 2 < u && u <= n), "n={n}: U* holds only numbers above n/2 and up to n");
    let mut member = vec![false; n as usize + 1];
    ustar.iter().for_each(|&u| member[u as usize] = true);
    member
}

/// The roles of every optimal set of game n in the two bottom layers (THEORY.md, Theorem B), indexed 0..=n, where
/// `ustar` is U\* in any order.  0 and 1 are barred (they cannot be picked); the largest prime q ≤ n is forced and
/// every other prime barred; each number of layer 2 above n/2 is forced if it is in U\* and barred if not; and, from
/// n = 187 on, every number of layer 2 up to n/2 is barred.  Everything else is free.
///
/// The last rule rests on U\*_2 spanning every prime up to n/2: an independent set of that many edges on the primes
/// up to n/2 and a ground vertex (pq is the edge p–q and p² the edge p–ground) is a spanning tree, so every other
/// number of layer 2 closes a cycle with it and Theorem B excludes it.  The function checks that U\*_2 has as many
/// members as there are primes up to n/2 and panics, naming n and both counts, if not.  Below 187 the numbers of
/// layer 2 up to n/2 stay free.
pub fn low_roles(n: u32, spf: &[u32], ustar: &[u32]) -> Vec<Role> {
    assert!(n >= 2, "game n={n}: the board is 2..=n");
    let half = n / 2;
    let member = in_ustar(n, ustar);
    let largest_prime = (2..=n).rev().find(|&p| is_prime(u64::from(p), spf)).expect("a board with n >= 2 has a prime");
    let mut roles = vec![Role::Free; n as usize + 1];
    roles[0] = Role::Barred;
    roles[1] = Role::Barred;
    let mut sources_in_layer_two = 0;
    for c in 2..=n {
        let role = match omega(c, spf) {
            1 if c == largest_prime => Role::Pick,
            1 => Role::Barred,
            2 if c > half && member[c as usize] => {
                sources_in_layer_two += 1;
                Role::Pick
            }
            2 if c > half || n >= CLOSED_FROM => Role::Barred,
            _ => Role::Free,
        };
        roles[c as usize] = role;
    }
    if n >= CLOSED_FROM {
        let primes = (2..=half).filter(|&p| is_prime(u64::from(p), spf)).count();
        assert_eq!(
            sources_in_layer_two, primes,
            "n={n}: U*_2 has {sources_in_layer_two} members but there are {primes} primes up to n/2, so the \
             frames of layer 2 are not closed"
        );
    }
    roles
}

/// The roles of the frame model's problem on the whole board: [`low_roles`] (Theorem B), and above n/2 every member
/// of `ustar` forced and every other number barred, so that nothing above n/2 is free.  A search from these roles
/// proves nothing about opt(n): it looks only for sets whose upper part is U\*, and that some optimal set has that
/// upper part is the open U of THEORY.md.  It is a measurement of what the frame model's problem costs.
pub fn upper_fixed_roles(n: u32, spf: &[u32], ustar: &[u32]) -> Vec<Role> {
    let mut roles = low_roles(n, spf, ustar);
    let member = in_ustar(n, ustar);
    for c in n / 2 + 1..=n {
        roles[c as usize] = if member[c as usize] { Role::Pick } else { Role::Barred };
    }
    roles
}

/// Two sets of numbers for the counters of [`Prover::watch`](crate::game::Prover::watch), each flagged over 0..=n:
/// the sources of layer 3 and up (every number above n/2 with Ω ≥ 3), and the members of U\*_{≥3} (those of `ustar`,
/// in any order, with Ω ≥ 3).
pub fn watched(n: u32, spf: &[u32], ustar: &[u32]) -> (Vec<bool>, Vec<bool>) {
    let member = in_ustar(n, ustar);
    let high: Vec<bool> = (0..=n).map(|c| omega(c, spf) >= 3).collect();
    let sources = (0..=n).map(|c| c > n / 2 && high[c as usize]).collect();
    let members = (0..=n).map(|c| member[c as usize] && high[c as usize]).collect();
    (sources, members)
}

/// Whether the numbers a game plays, `set`, obey `roles`: it holds every number with role `Pick` and none with role
/// `Barred`.  The error names the first breach.
pub fn obeys(roles: &[Role], set: &[u32]) -> Result<(), String> {
    let mut held = vec![false; roles.len()];
    for &c in set {
        if roles[c as usize] == Role::Barred {
            return Err(format!("{c} is barred but played"));
        }
        held[c as usize] = true;
    }
    match roles.iter().zip(&held).position(|(&role, &played)| role == Role::Pick && !played) {
        Some(c) => Err(format!("{c} is forced but not played")),
        None => Ok(()),
    }
}

/// A member of U\*_{≥3} the barred bound did not certify.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Open {
    /// The member.
    pub u: u32,
    /// Its layer, Ω(u).
    pub omega: u32,
    /// The bound with u barred minus V, or None if the bound could not be certified (not expected).
    pub margin: Option<i64>,
}

/// What the parity bound says of the members of U\*_{≥3} (module comment).
#[derive(Clone, Debug)]
pub struct Barred {
    /// The parity bound at the roles of [`low_roles`], with nothing barred beyond them.
    pub root: i64,
    /// Whether the root's heaviest set holds every member of U\*_{≥3}.
    pub root_has_upper: bool,
    /// The members of U\*_{≥3}, ascending.
    pub members: Vec<u32>,
    /// The members not certified, ascending.
    pub open: Vec<Open>,
}

impl Barred {
    /// Whether `u` is a member of U\*_{≥3} that was certified (is not open).
    pub fn certified(&self, u: u32) -> bool {
        self.members.contains(&u) && self.open.iter().all(|o| o.u != u)
    }

    /// The members that are certified, ascending.
    pub fn certified_members(&self) -> Vec<u32> {
        self.members.iter().copied().filter(|&u| self.certified(u)).collect()
    }
}

/// For game n (`board`) with U\* = `ustar` (any order) and the score `v`: the root bound at the roles of
/// [`low_roles`], and for each member u of U\*_{≥3} the bound with u barred as well, which certifies u if it is at
/// most `v` or the roles cannot be met, and otherwise leaves u open (module comment).  Each barred solve starts from
/// the root's certificate.  Panics if the root's bound is not certified: a game obeying the roles exists, so they
/// can be met.
pub fn barred_bounds(board: &Board, spf: &[u32], v: u64, ustar: &[u32]) -> Barred {
    let n = board.n;
    let roles = low_roles(n, spf, ustar);
    let weights: Vec<i64> = (0..=i64::from(n)).collect();
    let mut meet = Meet::new(Parity::pair(board, &weights));
    let Outcome::Bound(root) = meet.solve(&roles) else {
        panic!("n={n}: the parity bound at the roles of Theorem B is not certified")
    };
    let mut members: Vec<u32> = ustar.iter().copied().filter(|&u| omega(u, spf) >= 3).collect();
    members.sort_unstable();
    let root_has_upper = members.iter().all(|u| root.set.binary_search(u).is_ok());
    let mut open = Vec::new();
    for &u in &members {
        let mut barred = roles.clone();
        barred[u as usize] = Role::Barred;
        let margin = match meet.solve_from(&barred, Some(&root.warm)) {
            Outcome::Infeasible => continue,
            Outcome::Bound(bound) if bound.value <= v as i64 => continue,
            Outcome::Bound(bound) => Some(bound.value - v as i64),
            Outcome::Uncertified => None,
        };
        open.push(Open { u, omega: omega(u, spf), margin });
    }
    Barred { root: root.value, root_has_upper, members, open }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{relation, Prover, Relation, Verdict};
    use crate::jsonl::recorded_games;
    use crate::primes::smallest_prime_factors;
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::time::Instant;

    /// The games of optimal.json: n -> (score, moves).
    fn optimal() -> BTreeMap<u64, (u64, Vec<u32>)> {
        recorded_games(&[Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../src/main/resources/optimal.json"))])
    }

    /// The boards the enumerating tests cover.
    const SMALL: std::ops::RangeInclusive<u64> = 2..=60;

    /// Every claim V' below the recorded score V: the members certified at V' are certified at V, and every
    /// playable set that obeys the roles of Theorem B and beats V' holds each of them.  The sets are enumerated by
    /// the matching bound alone, so the check does not rest on the parity bound under test.  Boards 2..=60.
    #[test]
    fn certified_members_are_held_by_every_set_beating_the_claim() {
        let started = Instant::now();
        let (known, spf) = (optimal(), smallest_prime_factors(*SMALL.end() as usize));
        let (mut checks, mut certified, mut open) = (0usize, 0usize, 0usize);
        for n in SMALL {
            let (v, moves) = &known[&n];
            let board = Board::new(n as u32, &spf);
            let ustar = played_sources(n, moves);
            let roles = low_roles(n as u32, &spf, &ustar);
            let at_v = barred_bounds(&board, &spf, *v, &ustar);
            for claim in [v - 1, v.saturating_sub(2), v.saturating_sub(5)].into_iter().filter(|&c| c >= 1) {
                let barred = barred_bounds(&board, &spf, claim, &ustar);
                let cert = barred.certified_members();
                assert!(cert.iter().all(|&u| at_v.certified(u)), "n={n} claim={claim}: a lower claim certifies more");
                let mut prover = Prover::new(&board, claim, u64::MAX).enumerate(usize::MAX);
                assert_eq!(prover.run_units(vec![roles.clone()]), Verdict::Proven, "n={n} claim={claim}");
                for set in &prover.found {
                    for &u in &cert {
                        assert!(set.contains(&u), "n={n} claim={claim}: the set {set:?} lacks the certified {u}");
                        checks += 1;
                    }
                }
                certified += cert.len();
                open += barred.open.len();
            }
        }
        eprintln!(
            "{checks} (set, member) checks, {certified} members certified and {open} open, {:.2?}",
            started.elapsed()
        );
        assert!(checks > 0, "the test checked nothing");
    }

    /// Every optimal set of a game obeys the roles of Theorem B, found by enumeration from the plain root (the
    /// sets worth more than V − 1).  Boards 2..=60.
    #[test]
    fn every_optimal_set_obeys_the_roles() {
        let (known, spf) = (optimal(), smallest_prime_factors(*SMALL.end() as usize));
        for n in SMALL {
            let (v, moves) = &known[&n];
            let board = Board::new(n as u32, &spf);
            let roles = low_roles(n as u32, &spf, &played_sources(n, moves));
            let mut prover = Prover::new(&board, v - 1, u64::MAX).enumerate(usize::MAX);
            assert_eq!(prover.run(), Verdict::Proven, "n={n}");
            assert!(!prover.found.is_empty(), "n={n}: no optimal set");
            for set in &prover.found {
                obeys(&roles, set).unwrap_or_else(|e| panic!("n={n}: the optimal set {set:?}: {e}"));
            }
        }
    }

    /// From 187 on U\*_2 spans every prime up to n/2 (the function checks it), and every recorded game of
    /// optimal.json obeys the roles.  Every board, 187..=1300.
    #[test]
    fn the_layer_two_closure_holds_to_1300() {
        let (known, spf) = (optimal(), smallest_prime_factors(1300));
        let mut boards = 0;
        for (&n, (_, moves)) in known.range(CLOSED_FROM as u64..) {
            let roles = low_roles(n as u32, &spf, &played_sources(n, moves));
            obeys(&roles, moves).unwrap_or_else(|e| panic!("n={n}: the recorded game: {e}"));
            boards += 1;
        }
        assert_eq!(boards, 1300 - CLOSED_FROM + 1);
    }

    /// The figures of the barred bounds at 600 (they appear in THEORY.md's "What is known of U": 4 of the 161
    /// members are open).
    #[test]
    fn the_barred_bounds_at_600() {
        let (known, spf) = (optimal(), smallest_prime_factors(600));
        let (v, moves) = &known[&600];
        let ustar = played_sources(600, moves);
        let barred = barred_bounds(&Board::new(600, &spf), &spf, *v, &ustar);
        assert!(barred.root >= *v as i64);
        assert!(barred.root_has_upper);
        assert_eq!(barred.members.len(), 161);
        assert_eq!(barred.open.len(), 4, "{:?}", barred.open);
        assert!(barred.open.iter().all(|o| o.margin.is_some_and(|m| m > 0)));
    }

    /// A game that needs a search: its board size, recorded score, moves and U\*.
    struct Searched {
        n: u64,
        v: u64,
        moves: Vec<u32>,
        ustar: Vec<u32>,
    }

    /// The games of optimal.json in 187..=800 that need a search, as `exact` decides it: the relation to the game
    /// before is none and the parity bound at the root is above the score.  Found once for all the tests.
    fn searched_games() -> &'static [Searched] {
        static GAMES: std::sync::OnceLock<Vec<Searched>> = std::sync::OnceLock::new();
        GAMES.get_or_init(|| {
            let (known, spf) = (optimal(), smallest_prime_factors(800));
            let mut games = Vec::new();
            for n in 187..=800u64 {
                let (v, moves) = known[&n].clone();
                if relation(n, v, known[&(n - 1)].0, &spf) != Relation::None {
                    continue;
                }
                let board = Board::new(n as u32, &spf);
                let weights: Vec<i64> = (0..=n as i64).collect();
                let roles = Prover::new(&board, v, u64::MAX).root_roles();
                let Outcome::Bound(root) = Meet::new(Parity::pair(&board, &weights)).solve(&roles) else {
                    panic!("n={n}: the root's parity bound is not certified")
                };
                if root.value > v as i64 {
                    let ustar = recorded_ustar(n, &moves, &spf);
                    games.push(Searched { n, v, moves, ustar });
                }
            }
            games
        })
    }

    /// The games of [`searched_games`] with n in `range`.
    fn searched_in(range: std::ops::RangeInclusive<u64>) -> Vec<&'static Searched> {
        searched_games().iter().filter(|g| range.contains(&g.n)).collect()
    }

    /// The flags of a set of picks over 0..=n.
    fn flags(n: u64, set: &[u32]) -> Vec<bool> {
        let mut pick = vec![false; n as usize + 1];
        set.iter().for_each(|&c| pick[c as usize] = true);
        pick
    }

    /// Every recorded game of optimal.json plays exactly U\* above n/2 (`recorded_ustar` checks it).  Boards 2..=1300.
    #[test]
    fn every_recorded_game_plays_ustar() {
        let (known, spf) = (optimal(), smallest_prime_factors(1300));
        for (&n, (_, moves)) in &known {
            if n >= 2 {
                recorded_ustar(n, moves, &spf);
            }
        }
    }

    /// The counters of `Prover::watch` change nothing in the search: the verdict and the node count are those of the
    /// search without them.  Every 30th game that needs a search in 500..=800 (every 10th takes three times the time).
    #[test]
    fn watching_changes_nothing() {
        let started = Instant::now();
        let spf = smallest_prime_factors(800);
        let games = searched_in(500..=800);
        let mut checked = 0;
        for game in games.iter().step_by(30) {
            let board = Board::new(game.n as u32, &spf);
            let mut bare = Prover::new(&board, game.v, u64::MAX).with_parity();
            let (branch, barred) = watched(game.n as u32, &spf, &game.ustar);
            let mut watching = Prover::new(&board, game.v, u64::MAX).with_parity().watch(branch, barred);
            assert_eq!(watching.run(), bare.run(), "n={}", game.n);
            assert_eq!(watching.nodes, bare.nodes, "n={}", game.n);
            checked += 1;
        }
        eprintln!("{checked} of {} games compared, {:.2?}", games.len(), started.elapsed());
        assert!(checked > 0, "the test checked nothing");
    }

    /// Fixing the two bottom layers by Theorem B keeps the proofs and the optima: from `low_roles`, with the target
    /// the score nothing beats it, and with the target one below a playable set obeying the roles is found, worth
    /// the score.  Every 3rd game that needs a search in 187..=700 (all of them take three times the time).
    #[test]
    fn fixing_the_bottom_layers_keeps_the_proofs() {
        let started = Instant::now();
        let spf = smallest_prime_factors(700);
        let games = searched_in(187..=700);
        for Searched { n, v, ustar, .. } in games.iter().step_by(3).copied() {
            let board = Board::new(*n as u32, &spf);
            let roles = low_roles(*n as u32, &spf, ustar);
            let mut proof = Prover::new(&board, *v, u64::MAX).with_parity();
            assert_eq!(proof.run_units(vec![roles.clone()]), Verdict::Proven, "n={n}");
            let mut beaten = Prover::new(&board, v - 1, u64::MAX).with_parity();
            let Verdict::Beaten(set) = beaten.run_units(vec![roles.clone()]) else {
                panic!("n={n}: no set beats {}", v - 1)
            };
            assert!(set.iter().map(|&c| u64::from(c)).sum::<u64>() >= *v, "n={n}: the set {set:?} is worth less");
            assert!(board.playable(&flags(*n, &set)), "n={n}: the set {set:?} is not playable");
            obeys(&roles, &set).unwrap_or_else(|e| panic!("n={n}: the set {set:?}: {e}"));
        }
        eprintln!("{} of {} games, {:.2?}", games.len().div_ceil(3), games.len(), started.elapsed());
        assert!(!games.is_empty(), "the test checked nothing");
    }

    /// The counters count what they say.  From `upper_fixed_roles` nothing above n/2 is free and no member of U\*_{≥3}
    /// is barred, so both are 0, and the search proves the score (the recorded game has upper part U\*, so nothing
    /// with that upper part beats it); from `low_roles` neither exceeds the nodes.  Every 4th game that needs a
    /// search in 187..=700 (all of them take four times the time); the totals are printed.
    #[test]
    fn the_counters_count_the_sources() {
        let started = Instant::now();
        let spf = smallest_prime_factors(700);
        let games = searched_in(187..=700);
        let (mut low_nodes, mut fixed_nodes, mut branching, mut barring) = (0u64, 0u64, 0u64, 0u64);
        for Searched { n, v, moves, ustar } in games.iter().step_by(4).copied() {
            let (n32, board) = (*n as u32, Board::new(*n as u32, &spf));
            let (branch, barred) = watched(n32, &spf, ustar);
            assert!((0..=*n as usize).all(|c| !barred[c] || branch[c]), "n={n}: U*_(>=3) is not in the sources");
            let fixed_roles = upper_fixed_roles(n32, &spf, ustar);
            assert!(fixed_roles[(n32 / 2 + 1) as usize..].iter().all(|&r| r != Role::Free), "n={n}: a source is free");
            obeys(&fixed_roles, moves).unwrap_or_else(|e| panic!("n={n}: the recorded game: {e}"));
            let mut fixed = Prover::new(&board, *v, u64::MAX).with_parity().watch(branch.clone(), barred.clone());
            assert_eq!(fixed.run_units(vec![fixed_roles]), Verdict::Proven, "n={n}");
            let watch = fixed.watch.as_ref().expect("watching");
            assert_eq!((watch.branching, watch.barring), (0, 0), "n={n}: the fixed upper part is never free or barred");
            fixed_nodes += fixed.nodes;

            let mut low = Prover::new(&board, *v, u64::MAX).with_parity().watch(branch, barred);
            assert_eq!(low.run_units(vec![low_roles(n32, &spf, ustar)]), Verdict::Proven, "n={n}");
            let watch = low.watch.as_ref().expect("watching");
            assert!(watch.branching <= low.nodes && watch.barring <= low.nodes, "n={n}: more than one per node");
            low_nodes += low.nodes;
            branching += watch.branching;
            barring += watch.barring;
        }
        eprintln!(
            "{} of {} games: B {low_nodes} nodes ({branching} branching on a source of layer 3 or up, {barring} \
             barring a member of U*_>=3), D {fixed_nodes} nodes, {:.2?}",
            games.len().div_ceil(4),
            games.len(),
            started.elapsed()
        );
        assert!(!games.is_empty(), "the test checked nothing");
    }
}
