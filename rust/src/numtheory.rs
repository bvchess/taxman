//! Number-theory tables for a board `1..=n` (or one with holes, [`Tables::punctured`]): smallest prime
//! factors, divisors, and the *maximal* factors and composites that the frame model is built on.
//!
//! The *maximal factors* of `c` are `c/p` for each distinct prime `p` dividing `c` (so the maximal
//! factors of a prime are `{1}`, and `1` has none).  The *maximal composites* of `c` are `c·p ≤ n` for
//! primes `p` (so those of `1` are the primes).  The two are inverse relations: `f` is a maximal
//! factor of `m` exactly when `m` is a maximal composite of `f`.  (The Java code calls these
//! "abbreviated" factors and composites.)

/// Precomputed tables for the board `1..=n`.  Every list is ascending; index 0 is unused (and empty).
pub struct Tables {
    pub n: u32,
    /// Smallest prime factor of each `k >= 2`; `spf[0] = spf[1] = 0`.
    pub spf: Vec<u32>,
    /// `c/p` for each distinct prime `p | c`.
    pub max_factors: Vec<Vec<u32>>,
    /// `c·p ≤ n` for each prime `p`.
    pub max_composites: Vec<Vec<u32>>,
    /// All divisors of `k`, including 1 and `k`.
    pub divisors: Vec<Vec<u32>>,
    /// The numbers taken off the board, ascending (see [`Tables::punctured`]); empty for the board `1..=n`.  The
    /// lists above are arithmetic and include them.
    pub holes: Vec<u32>,
    primes: Vec<u32>,
}

impl Tables {
    /// Builds all tables in `O(n log n)` time (the divisor lists dominate; the rest is near-linear).
    pub fn new(n: u32) -> Self {
        let size = n as usize + 1;

        // Sieve of Eratosthenes recording the smallest prime factor.
        let mut spf = vec![0u32; size];
        let mut primes = Vec::new();
        for p in 2..size {
            if spf[p] == 0 {
                spf[p] = p as u32;
                primes.push(p as u32);
                for multiple in (p * p..size).step_by(p) {
                    if spf[multiple] == 0 {
                        spf[multiple] = p as u32;
                    }
                }
            }
        }

        // Maximal factors: factor c with the sieve, taking c/p once per distinct prime p.  Primes come out
        // ascending, so c/p comes out descending; reverse at the end.
        let mut max_factors = vec![Vec::new(); size];
        for (c, list) in max_factors.iter_mut().enumerate().skip(2) {
            let mut rest = c;
            while rest > 1 {
                let p = spf[rest] as usize;
                list.push((c / p) as u32);
                while rest % p == 0 {
                    rest /= p;
                }
            }
            list.reverse();
        }

        // Maximal composites: for each prime in turn, append c·p to every c with c·p ≤ n.  Primes ascend, so
        // each list ends up ascending.
        let mut max_composites = vec![Vec::new(); size];
        for &p in &primes {
            for c in 1..=(n / p) {
                max_composites[c as usize].push(c * p);
            }
        }

        // Divisors: the harmonic-sum sieve.  Divisors ascend because d does.
        let mut divisors = vec![Vec::new(); size];
        for d in 1..size {
            for multiple in (d..size).step_by(d) {
                divisors[multiple].push(d as u32);
            }
        }

        Tables { n, spf, max_factors, max_composites, divisors, holes: Vec::new(), primes }
    }

    /// The tables of the board `1..=n` with the numbers in `holes` taken off it.  A hole is never in the pot, so
    /// it is never a move and never pays tax.  The lists stay arithmetic (they still mention the holes): the
    /// hives ([`crate::hives::Apiary`]) leave the holes out, and the search only ever looks at the numbers of
    /// its frames.  A number whose maximal factor is a hole does *not* pay with the hole's factors instead, so
    /// on a punctured board the frame model plays the maximal-factor rule (see [`crate::sacrifice`]).
    pub fn punctured(n: u32, holes: &[u32]) -> Self {
        let mut t = Tables::new(n);
        let mut holes = holes.to_vec();
        holes.sort_unstable();
        holes.dedup();
        assert!(holes.iter().all(|h| (1..=n).contains(h)), "holes {holes:?} are not all on the board 1..={n}");
        t.holes = holes;
        t
    }

    /// Whether `k` has been taken off the board.
    #[inline]
    pub fn is_hole(&self, k: u32) -> bool {
        !self.holes.is_empty() && self.holes.binary_search(&k).is_ok()
    }

    /// Whether `k` is prime.  Requires `k <= n`.
    #[inline]
    pub fn is_prime(&self, k: u32) -> bool {
        k >= 2 && self.spf[k as usize] == k
    }

    /// All primes `<= n`, ascending.
    #[inline]
    pub fn primes(&self) -> &[u32] {
        &self.primes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brute force: divisors of `k` by trial division.
    fn brute_divisors(k: u32) -> Vec<u32> {
        (1..=k).filter(|&d| k.is_multiple_of(d)).collect()
    }

    /// Brute force, straight from the definition: proper divisors of `c` that are not themselves a proper
    /// divisor of another proper divisor of `c`.
    fn brute_max_factors(c: u32) -> Vec<u32> {
        let proper: Vec<u32> = brute_divisors(c).into_iter().filter(|&d| d < c).collect();
        proper.iter().copied().filter(|&f| !proper.iter().any(|&g| g != f && g % f == 0)).collect()
    }

    /// Brute force: multiples `m <= n` of `c` such that no number strictly between divides `m` and is divisible
    /// by `c`.
    fn brute_max_composites(c: u32, n: u32) -> Vec<u32> {
        let multiples: Vec<u32> = (2 * c..=n).step_by(c as usize).collect();
        multiples.iter().copied().filter(|&m| !multiples.iter().any(|&g| g != m && m % g == 0)).collect()
    }

    fn is_ascending(v: &[u32]) -> bool {
        v.windows(2).all(|w| w[0] < w[1])
    }

    #[test]
    fn java_factor_table_cases() {
        // From FactorTableTest.java.
        let t100 = Tables::new(100);
        assert_eq!(t100.max_factors[66], vec![6, 22, 33]);
        assert_eq!(t100.max_factors[50], vec![10, 25]);
        assert_eq!(t100.max_composites[49], vec![98]);
        assert_eq!(t100.max_composites[20], vec![40, 60, 100]);
        assert_eq!(t100.max_composites[50], vec![100]);
        assert!(Tables::new(20).max_factors[1].is_empty());
        assert_eq!(t100.divisors[83], vec![1, 83]);
        assert_eq!(Tables::new(20).divisors[20], vec![1, 2, 4, 5, 10, 20]);
        assert_eq!(
            t100.primes(),
            &[2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97]
        );
    }

    #[test]
    fn special_values() {
        let t = Tables::new(100);
        // 1 has no maximal factors; the maximal factors of a prime are {1}.
        assert!(t.max_factors[1].is_empty());
        assert_eq!(t.max_factors[2], vec![1]);
        assert_eq!(t.max_factors[97], vec![1]);
        // The maximal composites of 1 are the primes; primes above n/2 have none.
        assert_eq!(t.max_composites[1], t.primes());
        assert!(t.max_composites[53].is_empty());
        assert!(t.max_composites[100].is_empty());
        // Prime powers have one maximal factor.
        assert_eq!(t.max_factors[64], vec![32]);
        assert_eq!(t.max_factors[81], vec![27]);
        // Index 0 is unused.
        assert!(t.max_factors[0].is_empty() && t.max_composites[0].is_empty() && t.divisors[0].is_empty());
        assert_eq!((t.spf[0], t.spf[1], t.spf[2], t.spf[91], t.spf[97]), (0, 0, 2, 7, 97));
    }

    #[test]
    fn matches_brute_force_up_to_300() {
        for n in [1u32, 2, 3, 10, 64, 100, 211, 300] {
            let t = Tables::new(n);
            assert_eq!(t.n, n);
            assert_eq!(t.spf.len(), n as usize + 1);
            for k in 1..=n {
                let ku = k as usize;
                assert_eq!(t.divisors[ku], brute_divisors(k), "divisors of {k}");
                assert_eq!(t.max_factors[ku], brute_max_factors(k), "max_factors of {k}");
                assert_eq!(t.max_composites[ku], brute_max_composites(k, n), "max_composites of {k}, n={n}");
                assert_eq!(t.is_prime(k), brute_divisors(k).len() == 2, "is_prime({k})");
                if k >= 2 {
                    assert_eq!(t.spf[ku], (2..=k).find(|d| k % d == 0).unwrap(), "spf of {k}");
                }
                assert!(is_ascending(&t.divisors[ku]));
                assert!(is_ascending(&t.max_factors[ku]));
                assert!(is_ascending(&t.max_composites[ku]));
            }
            let primes: Vec<u32> = (1..=n).filter(|&k| brute_divisors(k).len() == 2).collect();
            assert_eq!(t.primes(), primes.as_slice());
        }
    }

    #[test]
    fn factors_and_composites_are_inverse_relations() {
        let t = Tables::new(500);
        for m in 2..=500u32 {
            for &f in &t.max_factors[m as usize] {
                assert!(t.max_composites[f as usize].contains(&m), "{f} -> {m}");
            }
        }
        for c in 1..=500u32 {
            for &m in &t.max_composites[c as usize] {
                assert!(t.max_factors[m as usize].contains(&c), "{m} <- {c}");
            }
        }
    }

    #[test]
    fn tiny_boards() {
        let t0 = Tables::new(0);
        assert_eq!(t0.n, 0);
        assert!(t0.primes().is_empty());
        assert_eq!(t0.divisors.len(), 1);
        let t1 = Tables::new(1);
        assert!(t1.primes().is_empty());
        assert_eq!(t1.divisors[1], vec![1]);
        assert!(t1.max_factors[1].is_empty() && t1.max_composites[1].is_empty());
        let t2 = Tables::new(2);
        assert_eq!(t2.primes(), &[2]);
        assert_eq!(t2.max_composites[1], vec![2]);
    }

    #[test]
    fn large_board_builds() {
        let t = Tables::new(10_000);
        assert_eq!(t.primes().len(), 1229);
        assert_eq!(t.max_factors[9_999], vec![99, 909, 3333]);
        assert_eq!(t.divisors[9_240].len(), 64);
    }
}
