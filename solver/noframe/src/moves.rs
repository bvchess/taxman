//! The longest games: M(n), the most moves a game of n can have, on the whole board, to set against the optimal
//! game's.
//!
//! A game's moves are its picks, so M(n) is the size of the largest playable set of picks on the board 1..=n.
//! Three proven facts bound it:
//!
//! * **The matching number.**  A playable S pays each pick with a distinct maximal factor outside S (Theorem 1),
//!   so S is the pick side of a matching of the board's maximal-factor graph (each number joined to its maximal
//!   factors): M(n) <= ν(n), that graph's matching number ([`matching_numbers`]).
//! * **M(n − 1) <= M(n) <= M(n − 1) + 1.**  A game of n − 1 is a game of n (n divides nothing smaller), and a game
//!   of n without its move n is a game of n − 1.
//! * **M(n) = M(n − 1) for a prime n >= 3.**  A playable set holds at most one prime (primes pay only with 1); in
//!   one holding n, swap n for p, the largest prime below n, which is no one's maximal factor on the board 1..=n − 1
//!   (2p > n − 1 by Bertrand) and pays with 1 as n did: a set of n − 1 as large, and playable.
//!
//! Any legal game of n is a lower bound, the recorded optimal one among them; where it meets ν(n) it is a
//! longest game.  Elsewhere [`longest`] decides M(n) with the branch and bound of [`crate::game`] under unit
//! weights, which asks whether some playable set has more picks than a target.

use crate::game::{Board, Prover, Role, Verdict};

const NONE: u32 = u32::MAX;

/// ν(m) for every m = 0..=n: the matching number of the maximal-factor graph of the board 1..=m.  Incremental:
/// adding the number m to the board raises ν by one exactly when a maximum matching of 1..=m − 1 has an
/// augmenting path from m (Berge), which one alternating search from m finds (the graph is bipartite by the
/// parity of Ω, and m's neighbours on the board 1..=m are its maximal factors).
pub fn matching_numbers(board: &Board) -> Vec<u64> {
    let n = board.n as usize;
    let mut mate = vec![NONE; n + 1];
    let mut seen = vec![0u32; n + 1];
    let mut nu = vec![0u64; 2];
    for m in 2..=board.n {
        // The alternating search from m, over the numbers present (<= m); `seen` marks the far side by stamp m.
        let grown = augment(board, m, m, &mut mate, &mut seen);
        nu.push(nu[m as usize - 1] + u64::from(grown));
    }
    nu.truncate(n + 1);
    nu
}

/// Kuhn's alternating search from `u` on the board 1..=`limit`: true (and the matching augmented) if an
/// augmenting path starts at `u`.
fn augment(board: &Board, u: u32, limit: u32, mate: &mut [u32], seen: &mut [u32]) -> bool {
    let neighbours = board.mf[u as usize].iter().chain(board.mc[u as usize].iter().take_while(|&&c| c <= limit));
    for &v in neighbours {
        if seen[v as usize] == limit {
            continue;
        }
        seen[v as usize] = limit;
        let w = mate[v as usize];
        if w == NONE || augment(board, w, limit, mate, seen) {
            mate[v as usize] = u;
            mate[u as usize] = v;
            return true;
        }
    }
    false
}

/// A payment for every pick of `set`, if it is playable: dismantling removes, one at a time, a pick with a maximal
/// factor (not a pick) that no other remaining pick lists, and that factor is its payment.  The pairs (pick,
/// payment) come in dismantling order; distinct payments, so a matching of the maximal-factor graph.
pub fn payments(board: &Board, set: &[u32]) -> Option<Vec<(u32, u32)>> {
    let n = board.n as usize;
    let mut pick = vec![false; n + 1];
    set.iter().for_each(|&c| pick[c as usize] = true);
    let mut alive = pick.clone();
    let mut degree = vec![0u32; n + 1];
    for &c in set {
        for &f in board.mf[c as usize].iter().filter(|&&f| !pick[f as usize]) {
            degree[f as usize] += 1;
        }
    }
    let private = |c: u32, degree: &[u32]| {
        board.mf[c as usize].iter().copied().find(|&f| !pick[f as usize] && degree[f as usize] == 1)
    };
    let mut queue: Vec<u32> = set.iter().copied().filter(|&c| private(c, &degree).is_some()).collect();
    let mut paid = Vec::with_capacity(set.len());
    while let Some(c) = queue.pop() {
        if !alive[c as usize] {
            continue;
        }
        let f = private(c, &degree).expect("a queued pick keeps its private factor while alive");
        alive[c as usize] = false;
        paid.push((c, f));
        for &g in board.mf[c as usize].iter().filter(|&&g| !pick[g as usize]) {
            degree[g as usize] -= 1;
            if degree[g as usize] == 1 {
                let other = board.mc[g as usize].iter().find(|&&m| alive[m as usize]);
                queue.push(*other.expect("a factor of degree 1 has a live lister"));
            }
        }
    }
    (paid.len() == set.len()).then_some(paid)
}

/// A legal order of the moves of a playable `set` (Theorem 1): a topological order of the precedence "a before b
/// when a's payment divides b, or a divides b", with the payments of [`payments`].  None if unplayable.
pub fn play_order(board: &Board, set: &[u32]) -> Option<Vec<u32>> {
    let paid = payments(board, set)?;
    let k = paid.len();
    let mut indegree = vec![0usize; k];
    let mut after: Vec<Vec<usize>> = vec![Vec::new(); k];
    for (i, &(a, pay)) in paid.iter().enumerate() {
        for (j, &(b, _)) in paid.iter().enumerate() {
            if i != j && (b % pay == 0 || b % a == 0) {
                after[i].push(j);
                indegree[j] += 1;
            }
        }
    }
    let mut ready: Vec<usize> = (0..k).filter(|&i| indegree[i] == 0).collect();
    let mut order = Vec::with_capacity(k);
    while let Some(i) = ready.pop() {
        order.push(paid[i].0);
        for &j in &after[i] {
            indegree[j] -= 1;
            if indegree[j] == 0 {
                ready.push(j);
            }
        }
    }
    assert_eq!(order.len(), k, "the dismantling payments' precedence is acyclic (Theorems 2 and 5)");
    Some(order)
}

/// What [`longest`] concluded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Longest {
    /// M(n), if the search finished within its budget.
    pub most: Option<u64>,
    /// The longest playable set found with more picks than the game given (ascending), if any.
    pub longer: Option<Vec<u32>>,
    pub nodes: u64,
}

/// The most moves of game n, given that a game of `at_least` moves exists: the unit-weight search from that
/// target, raising it to each longer set it finds ([`Prover::optimize`]) until it proves that nothing is longer
/// (or `budget` nodes run out).  `smaller`, upper bounds on M(m) for m = 0..n−1 (the matching numbers, say), turns
/// on the segment cuts: a set with more than T picks has more than T − M(m) of them above m.
pub fn longest(board: &Board, at_least: u64, smaller: Option<&[u64]>, budget: u64) -> Longest {
    let mut prover = Prover::new(board, at_least, budget).weighted(vec![1; board.n as usize + 1]).optimize();
    if let Some(smaller) = smaller {
        prover = prover.with_smaller_games(smaller);
    }
    let verdict = prover.run();
    let mut longer = prover.found.pop();
    if let Some(set) = longer.as_mut() {
        set.sort_unstable();
    }
    let most = (verdict == Verdict::Proven).then(|| longer.as_ref().map_or(at_least, |s| s.len() as u64));
    Longest { most, longer, nodes: prover.nodes }
}

/// What [`heaviest_longest`] found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heaviest {
    /// The heaviest longest game's picks (ascending), if the search finished within its budget.
    pub set: Option<Vec<u32>>,
    pub nodes: u64,
}

/// The heaviest longest game: the largest sum of a playable set of M(n) picks, given `witness`, a playable set of
/// M(n) picks.  The search weighs the pick of c by K + c, K above every sum: as no playable set has more than M(n)
/// picks, one outweighs K·M(n) + x exactly when it has M(n) picks and sums to more than x.  It starts from the
/// witness's weight and raises the target to each heavier set it finds ([`Prover::optimize`]).  A search from a
/// light witness is slow, so with `upper_first`, a set of numbers above n/2 (the optimal game's upper part, say),
/// it first searches the sets whose upper part that is (cheap: the upper half is fixed) and starts the whole
/// search from the heaviest of those.  (At n = 833 the whole search takes 2.6 million nodes from that start, and
/// had not finished 10 minutes from the witness of [`longest`]; segment cuts on K·M(m) + opt(m) pruned nothing
/// there.)  Panics if it finds a set with more picks than the witness (which is then not a longest game).
pub fn heaviest_longest(board: &Board, witness: &[u32], upper_first: Option<&[u32]>, budget: u64) -> Heaviest {
    let n = u64::from(board.n);
    let k = n * (n + 1) / 2 + 1;
    let weights: Vec<i64> = (0..=n as i64).map(|c| k as i64 + c).collect();
    let most = witness.len() as u64;
    let weight = |set: &[u32]| k * most + set.iter().map(|&c| u64::from(c)).sum::<u64>();
    let search = |target: u64, budget: u64| Prover::new(board, target, budget).weighted(weights.clone()).optimize();
    let mut best = witness.to_vec();
    let mut nodes = 0;
    if let Some(upper) = upper_first {
        let mut roles = search(0, 0).root_roles();
        for c in (n / 2 + 1)..=n {
            roles[c as usize] = if upper.contains(&(c as u32)) { Role::Pick } else { Role::Barred };
        }
        let mut prover = search(weight(&best), budget);
        let verdict = prover.run_units(vec![roles]);
        nodes += prover.nodes;
        if verdict != Verdict::Proven {
            return Heaviest { set: None, nodes };
        }
        if let Some(set) = prover.found.pop() {
            best = set;
        }
    }
    let mut prover = search(weight(&best), budget.saturating_sub(nodes));
    let verdict = prover.run();
    nodes += prover.nodes;
    if let Some(set) = prover.found.pop() {
        best = set;
    }
    assert_eq!(best.len() as u64, most, "the witness is not a longest game");
    best.sort_unstable();
    Heaviest { set: (verdict == Verdict::Proven).then_some(best), nodes }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::MatchingBound;
    use crate::primes::smallest_prime_factors;

    /// The first game whose optimal game is not a longest one (README, section 6): the recorded game of 833 has
    /// 362 moves, and exchanging four of its moves below n/2 for five lighter ones gives a legal game of 363, the
    /// most any game of 833 can have (ν(833) = 363), worth 29 less.  (That no game of 363 moves is worth more is a
    /// search of 2.6 million nodes, `bin/moves`.)
    #[test]
    fn game_833_has_a_longer_game_than_its_optimal_one() {
        let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../results/optimal.json"));
        let (score, moves) = crate::jsonl::recorded_games(&[path])[&833].clone();
        let spf = smallest_prime_factors(833);
        let board = Board::new(833, &spf);
        let mut longer: Vec<u32> = moves
            .iter()
            .copied()
            .filter(|c| ![250, 252, 294, 400].contains(c))
            .chain([116, 180, 200, 320, 351])
            .collect();
        longer.sort_unstable();
        assert_eq!((moves.len(), score, matching_numbers(&board)[833]), (362, 218_226, 363));
        assert_eq!(board.replay(&play_order(&board, &longer).unwrap()).map(|s| (longer.len(), s)), Ok((363, 218_197)));
    }

    /// The moves of optimal.json's games to 400, in the order `play_order` gives them, replay by the rules to the
    /// recorded scores; a set with an unplayable subset has no payments.
    #[test]
    fn play_order_replays_the_recorded_games() {
        let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../results/optimal.json"));
        let known = crate::jsonl::recorded_games(&[path]);
        let spf = smallest_prime_factors(400);
        for n in 2..=400u32 {
            let (score, moves) = &known[&u64::from(n)];
            let board = Board::new(n, &spf);
            let mut set = moves.clone();
            set.sort_unstable();
            assert_eq!(board.replay(&play_order(&board, &set).unwrap()), Ok(*score), "n={n}");
        }
        let board = Board::new(39, &spf);
        assert_eq!(payments(&board, &[22, 26, 33, 39]), None, "the self-covering core of THEORY.md");
    }

    /// The incremental matching numbers equal the unit-weight Franklín–Moniot bound solved afresh on each board.
    #[test]
    fn matching_numbers_are_the_unit_matching_bound() {
        let spf = smallest_prime_factors(400);
        let nu = matching_numbers(&Board::new(400, &spf));
        for m in 2..=400u32 {
            let board = Board::new(m, &spf);
            let mut bound = MatchingBound::new(&board);
            bound.set_weights(&vec![1; m as usize + 1]);
            let mut roles = vec![Role::Free; m as usize + 1];
            roles[0] = Role::Barred;
            roles[1] = Role::Barred;
            assert_eq!(bound.solve(&board, &roles).map(|s| s.0), Some(nu[m as usize]), "m={m}");
        }
    }

    /// M(n) against brute force for n <= 22, from below (a smaller game given) and with the segment cuts; and the
    /// proven relations between neighbouring games.
    #[test]
    fn longest_against_brute_force() {
        let spf = smallest_prime_factors(100);
        let mut most = vec![0u64; 2];
        for n in 2..=22u32 {
            let board = Board::new(n, &spf);
            let brute = (0u64..1 << (n - 1))
                .filter(|bits| {
                    let mut pick = vec![false; n as usize + 1];
                    (2..=n as usize).filter(|&c| bits >> (c - 2) & 1 == 1).for_each(|c| pick[c] = true);
                    board.playable(&pick)
                })
                .map(|bits| u64::from(bits.count_ones()))
                .max()
                .unwrap();
            let nu = matching_numbers(&board);
            assert!(brute <= nu[n as usize]);
            for smaller in [None, Some(&nu[..n as usize])] {
                let got = longest(&board, 0, smaller, u64::MAX);
                assert_eq!(got.most, Some(brute), "n={n}");
                let longer = got.longer.expect("a game longer than none");
                assert_eq!(longer.len() as u64, brute);
            }
            // The heaviest longest game, against brute force; and its moves replay by the rules in the order
            // `play_order` gives.
            let heaviest_brute = (0u64..1 << (n - 1))
                .filter(|bits| u64::from(bits.count_ones()) == brute)
                .map(|bits| (2..=n).filter(|&c| bits >> (c - 2) & 1 == 1).collect::<Vec<u32>>())
                .filter(|set| {
                    let mut pick = vec![false; n as usize + 1];
                    set.iter().for_each(|&c| pick[c as usize] = true);
                    board.playable(&pick)
                })
                .map(|set| set.iter().map(|&c| u64::from(c)).sum::<u64>())
                .max()
                .unwrap();
            let witness = longest(&board, 0, None, u64::MAX).longer.unwrap();
            let heaviest = heaviest_longest(&board, &witness, None, u64::MAX).set.unwrap();
            let sum: u64 = heaviest.iter().map(|&c| u64::from(c)).sum();
            assert_eq!((heaviest.len() as u64, sum), (brute, heaviest_brute), "n={n}");
            let upper: Vec<u32> = heaviest.iter().copied().filter(|&c| 2 * c > n).collect();
            let seeded = heaviest_longest(&board, &witness, Some(&upper), u64::MAX).set.unwrap();
            assert_eq!(seeded.iter().map(|&c| u64::from(c)).sum::<u64>(), heaviest_brute, "n={n}");
            assert_eq!(board.replay(&play_order(&board, &heaviest).unwrap()), Ok(sum), "n={n}");
            let prev = most[n as usize - 1];
            assert!(prev <= brute && brute <= prev + 1);
            assert!(n < 3 || !crate::primes::is_prime(u64::from(n), &spf) || brute == prev, "n={n}");
            most.push(brute);
        }
    }
}
