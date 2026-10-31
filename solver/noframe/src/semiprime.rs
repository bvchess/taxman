//! The semiprime formula (J3, docs/theory/arrival.md, "The semiprime formula"): at an odd number n with two prime
//! factors that closes a cycle with U\*_2(n − 1) in the graph G_n of the bottom-layers theorem (F4,
//! docs/theory/frame-model.md), opt(n) = opt(n − 1) + n − x, with x the smallest member of the cycle other than n.
//!
//! G_n has a vertex for each prime up to n/2 and one more, the ground g; a number pq of layer 2 is the edge p–q, and
//! p² the edge p–g.  U\*_2(n − 1) is the part of U\*(n − 1), descending greedy on the board of n − 1
//! ([`u_star`]), that lies in layer 2: a forest in G_n.  The cycle that n closes with it is n and the *path of n*, the
//! path of that forest between the ends of n.  This module finds that path; [`crate::game::relation`] uses x to prove
//! V(n) from V(n − 1).

use crate::peel::u_star;
use crate::upper::omega;

/// The ground vertex g.  Every prime is the vertex of its own value, and no prime is 0.
const GROUND: u32 = 0;

/// The two ends of m, a number of layer 2: m = pq has the ends p and q, and m = p² the ends p and g.
fn ends(m: u32, spf: &[u32]) -> (u32, u32) {
    let (p, q) = (spf[m as usize], m / spf[m as usize]);
    if p == q {
        (p, GROUND)
    } else {
        (p, q)
    }
}

/// The path of n: the members of U\*_2(n − 1) on the path of that forest between the ends of n, ascending.  `None` if
/// n is even or does not have two prime factors, or if its ends are not joined, so that n closes no cycle with
/// U\*_2(n − 1).  `spf` must reach n.  Panics, naming n, if U\*_2(n − 1) is not a forest, which F4 proves it is.
pub fn path_of(n: u64, spf: &[u32]) -> Option<Vec<u32>> {
    let n = u32::try_from(n).expect("a board that fits 32 bits");
    if n.is_multiple_of(2) || omega(n, spf) != 2 {
        return None;
    }
    // The vertices are the primes up to n/2 and the ground, 0.  Each lists its edges as (the other end, the number).
    let mut edges_at: Vec<Vec<(u32, u32)>> = vec![Vec::new(); n as usize / 2 + 1];
    for m in u_star(u64::from(n) - 1, spf).into_iter().filter(|&m| omega(m, spf) == 2) {
        let (a, b) = ends(m, spf);
        edges_at[a as usize].push((b, m));
        edges_at[b as usize].push((a, m));
    }
    // Walk the forest from one end of n.  `came_by[v]` is the vertex before v and the edge between them (the root's
    // edge is 0, the number of no edge).  In a forest every neighbor but the one behind is new.
    let (from, to) = ends(n, spf);
    let mut came_by: Vec<Option<(u32, u32)>> = vec![None; edges_at.len()];
    came_by[from as usize] = Some((from, 0));
    let mut stack = vec![from];
    while let Some(v) = stack.pop() {
        let behind = came_by[v as usize].expect("a vertex on the stack was reached").1;
        for &(w, edge) in &edges_at[v as usize] {
            match came_by[w as usize] {
                None => {
                    came_by[w as usize] = Some((v, edge));
                    stack.push(w);
                }
                Some(_) if edge == behind => {}
                Some(_) => panic!("n={n}: U*_2({}) is not a forest: the edge {edge} closes a cycle", n - 1),
            }
        }
    }
    let mut path = Vec::new();
    let mut at = to;
    while at != from {
        let (before, edge) = came_by[at as usize]?;
        path.push(edge);
        at = before;
    }
    path.sort_unstable();
    Some(path)
}

/// x of J3: the smallest member of the cycle that n closes with U\*_2(n − 1) other than n, that is, the smallest
/// member of [`path_of`] n.  `None` where there is no cycle.
pub fn exchange(n: u64, spf: &[u32]) -> Option<u32> {
    path_of(n, spf).map(|path| path[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{relation, Board, Prover, Relation, Verdict};
    use crate::jsonl::{committed_chain, recorded_games_to};
    use crate::primes::smallest_prime_factors;

    /// The examples of J3 (docs/theory/arrival.md): U\*_2(38) is {21, 22, 25, 26, 33, 34, 35, 38}, 39 = 3·13 closes
    /// the cycle 3–11–2–13 with 33, 22 and 26, and 51 = 3·17 the cycle 3–13–2–17 with 39, 26 and 34; 21 and 33 close
    /// none, and neither does an even number, a prime or a number of three prime factors.
    #[test]
    fn the_cycles_of_the_examples() {
        let spf = smallest_prime_factors(200);
        let mut layer_two: Vec<u32> = u_star(38, &spf).into_iter().filter(|&m| omega(m, &spf) == 2).collect();
        layer_two.sort_unstable();
        assert_eq!(layer_two, vec![21, 22, 25, 26, 33, 34, 35, 38]);
        assert_eq!(path_of(39, &spf), Some(vec![22, 26, 33]));
        assert_eq!(exchange(39, &spf), Some(22));
        assert_eq!(path_of(51, &spf), Some(vec![26, 34, 39]));
        assert_eq!(exchange(51, &spf), Some(26));
        for n in [21, 33, 187, 38, 41, 45, 117, 128] {
            assert_eq!(path_of(n, &spf), None, "n={n}");
        }
    }

    /// Every odd n from 189 on with two prime factors closes a cycle (J3, step 7), and the path is a path of
    /// U\*_2(n − 1): members of layer 2 above (n − 1)/2 that are at least 2 in number.
    #[test]
    fn every_odd_semiprime_from_189_closes_a_cycle() {
        let spf = smallest_prime_factors(400);
        let mut found = 0;
        for n in (189..=400u32).step_by(2).filter(|&n| omega(n, &spf) == 2) {
            let path = path_of(u64::from(n), &spf).unwrap_or_else(|| panic!("n={n}: no cycle"));
            assert!(path.len() >= 2 && path.iter().all(|&m| 2 * m > n && omega(m, &spf) == 2), "n={n}: {path:?}");
            found += 1;
        }
        assert!(found > 40, "{found}");
    }

    /// The recorded scores meet the formula: for every odd n to `LAST_RECORDED` with two prime factors that is no
    /// ceiling, the relation is `Semiprime` exactly when n closes a cycle, which is every such n from 189 on (the
    /// theory agent's Python check reached 5060).  `relation` panics where V(n) is not V(n − 1) + n − x.
    #[test]
    fn the_recorded_scores_meet_the_semiprime_formula() {
        const LAST_RECORDED: u64 = 1300;
        let known = recorded_games_to(&[committed_chain()], LAST_RECORDED);
        let spf = smallest_prime_factors(LAST_RECORDED as usize);
        let mut semiprimes = 0;
        for n in (9..=LAST_RECORDED).step_by(2).filter(|&n| omega(n as u32, &spf) == 2) {
            let (score, prev) = (known[&n].0, known[&(n - 1)].0);
            let cycle = exchange(n, &spf);
            let got = relation(n, score, prev, &spf);
            assert_eq!(got == Relation::Semiprime, cycle.is_some(), "n={n}: {got:?}, exchange {cycle:?}");
            if let Some(x) = cycle {
                assert_eq!(score, prev + n - u64::from(x), "n={n}: V(n) = V(n − 1) + n − x");
                semiprimes += 1;
            } else {
                assert!(n < 189, "n={n}: no cycle from 189 on");
            }
        }
        assert!(semiprimes > 150, "{semiprimes}");
    }

    /// V(n) above or below V(n − 1) + n − x is a contradiction of the theorem, or of the recorded scores, and stops
    /// the program, naming n.
    #[test]
    fn a_score_off_the_formula_panics() {
        let spf = smallest_prime_factors(100);
        // opt(38) = 486 and x = 22 at n = 39: 503.
        assert_eq!(relation(39, 503, 486, &spf), Relation::Semiprime);
        for score in [502, 504] {
            let caught = std::panic::catch_unwind(|| relation(39, score, 486, &smallest_prime_factors(100)))
                .expect_err("a score off the formula is refused");
            let message = caught.downcast_ref::<String>().expect("a message");
            assert!(message.contains("n=39") && message.contains("503"), "{message}");
        }
        // Without V(n − 1) (0 for a game not recorded) the formula has nothing to rest on.
        assert_eq!(relation(39, 503, 0, &spf), Relation::None);
    }

    /// opt(n) by the prover's search from nothing (exact: it is tested against brute force over all sets to n = 20 and
    /// against the recorded scores to 420), for the board n.
    fn optimum(n: u32, spf: &[u32]) -> u64 {
        let board = Board::new(n, spf);
        let mut prover = Prover::new(&board, 0, u64::MAX).optimize();
        assert_eq!(prover.run(), Verdict::Proven, "n={n}");
        prover.found.last().expect("a game").iter().map(|&c| u64::from(c)).sum()
    }

    /// The last board of the brute-force tests.
    const LAST_BRUTE: u32 = 300;

    /// An odd n with two prime factors, its optimum and its predecessor's, and the path of its cycle if it has one.
    struct Case {
        n: u32,
        opt: u64,
        before: u64,
        path: Option<Vec<u32>>,
    }

    /// The cases of the odd n to `LAST_BRUTE` with two prime factors.
    fn brute_cases() -> &'static [Case] {
        static CASES: std::sync::OnceLock<Vec<Case>> = std::sync::OnceLock::new();
        CASES.get_or_init(|| {
            let spf = smallest_prime_factors(LAST_BRUTE as usize);
            (9..=LAST_BRUTE)
                .step_by(2)
                .filter(|&n| omega(n, &spf) == 2)
                .map(|n| Case {
                    n,
                    opt: optimum(n, &spf),
                    before: optimum(n - 1, &spf),
                    path: path_of(u64::from(n), &spf),
                })
                .collect()
        })
    }

    /// The games of [`brute_cases`] with a cycle, and those among them whose optima break opt(n) − opt(n − 1) = n − x
    /// when x is the member of the path that `rule` picks.
    fn broken_by(rule: fn(&[u32]) -> u32) -> (Vec<u32>, Vec<u32>) {
        let (mut cycles, mut broken) = (Vec::new(), Vec::new());
        for Case { n, opt, before, path } in brute_cases() {
            let Some(path) = path else { continue };
            cycles.push(*n);
            if opt - before != u64::from(n - rule(path)) {
                broken.push(*n);
            }
        }
        (cycles, broken)
    }

    /// Against the search from nothing, on every odd n to `LAST_BRUTE` with two prime factors: the relation of the
    /// optimal scores is `Semiprime` exactly when n closes a cycle, and then opt(n) − opt(n − 1) = n − x, x the
    /// smallest member of the path.
    #[test]
    fn the_formula_against_the_search_from_nothing() {
        let spf = smallest_prime_factors(LAST_BRUTE as usize);
        for Case { n, opt, before, path } in brute_cases() {
            let got = relation(u64::from(*n), *opt, *before, &spf);
            assert_eq!(got == Relation::Semiprime, path.is_some(), "n={n}: {got:?}");
        }
        let (cycles, broken) = broken_by(|path| path[0]);
        assert!(cycles.len() >= 40, "{} games with a cycle", cycles.len());
        assert_eq!(broken, Vec::<u32>::new(), "the optima break the formula");
    }

    /// The test above has teeth: x taken as the largest member of the path instead of the smallest breaks the formula
    /// on every game with a cycle, since the path has at least two members.  If it broke it nowhere, the test above
    /// could not see the failure it guards against.
    #[test]
    fn a_rule_that_takes_the_largest_member_is_caught() {
        let (cycles, broken) = broken_by(|path| *path.last().expect("a path"));
        assert!(cycles.len() >= 40 && broken == cycles, "{} of {} games", broken.len(), cycles.len());
    }

    /// Only an odd number with two prime factors has a path: no prime, no even number and no number of one, three or
    /// more prime factors does.
    #[test]
    fn only_an_odd_semiprime_has_a_path() {
        let spf = smallest_prime_factors(200);
        for n in 3..200u64 {
            if n % 2 == 0 || omega(n as u32, &spf) != 2 {
                assert_eq!(path_of(n, &spf), None, "n={n}");
            }
        }
    }
}
