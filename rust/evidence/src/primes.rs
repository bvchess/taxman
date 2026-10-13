//! The sieve: smallest prime factors, and the maximal factors they give.

/// `spf[k]` is the smallest prime factor of `k` for `k >= 2` (`spf[0] = spf[1] = 0`).
pub fn smallest_prime_factors(limit: usize) -> Vec<u32> {
    let mut spf = vec![0u32; limit + 1];
    for p in 2..=limit {
        if spf[p] == 0 {
            spf[p] = p as u32;
            let mut multiple = p * p;
            while multiple <= limit {
                if spf[multiple] == 0 {
                    spf[multiple] = p as u32;
                }
                multiple += p;
            }
        }
    }
    spf
}

/// The distinct primes dividing `c`, ascending.
pub fn distinct_primes(mut c: u64, spf: &[u32]) -> Vec<u64> {
    let mut primes = Vec::new();
    while c > 1 {
        let p = u64::from(spf[c as usize]);
        primes.push(p);
        while c.is_multiple_of(p) {
            c /= p;
        }
    }
    primes
}

/// The maximal factors of `c`: `c/p` for each distinct prime `p | c` (a prime's only one is 1), ascending.
pub fn maximal_factors(c: u64, spf: &[u32]) -> Vec<u64> {
    let mut out: Vec<u64> = distinct_primes(c, spf).into_iter().map(|p| c / p).collect();
    out.sort_unstable();
    out
}

pub fn is_prime(k: u64, spf: &[u32]) -> bool {
    k >= 2 && u64::from(spf[k as usize]) == k
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maximal_factors_by_hand() {
        let spf = smallest_prime_factors(100);
        assert_eq!(maximal_factors(12, &spf), vec![4, 6]);
        assert_eq!(maximal_factors(7, &spf), vec![1]);
        assert_eq!(maximal_factors(8, &spf), vec![4]);
        assert_eq!(maximal_factors(30, &spf), vec![6, 10, 15]);
        assert!(is_prime(97, &spf) && !is_prime(91, &spf));
    }
}
