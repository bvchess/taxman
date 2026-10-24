//! Reduction instances: a set of members, each listing the factors it can pay with.
//!
//! The instances here are *separated* (no member is a factor of another member), as the exclusion theorem (P8,
//! docs/theory/playability.md) requires: the sources of a game, its truncations, their components and
//! residuals, and synthetic instances built the same way.  Members are indexed in *greedy order*: member 0 is
//! considered first (for a game, the largest number), so descending greedy is a scan of the indices in order.

use crate::primes::maximal_factors;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Instance {
    /// A label per member (for a game, the number itself); greedy order is index order.
    pub value: Vec<u64>,
    /// For each member, the ids of its factors, ascending.
    pub factors: Vec<Vec<u32>>,
    /// For each factor id, the members listing it, ascending.
    pub listers: Vec<Vec<u32>>,
    /// A label per factor (for a game, the factor itself).
    pub factor_value: Vec<u64>,
}

impl Instance {
    /// An instance from members in greedy order and their factors (by label).
    pub fn new(value: Vec<u64>, factor_lists: &[Vec<u64>]) -> Self {
        assert_eq!(value.len(), factor_lists.len());
        let mut id: HashMap<u64, u32> = HashMap::new();
        let mut factor_value = Vec::new();
        let mut factors = Vec::with_capacity(value.len());
        for list in factor_lists {
            let mut ids: Vec<u32> = list
                .iter()
                .map(|&f| {
                    *id.entry(f).or_insert_with(|| {
                        factor_value.push(f);
                        (factor_value.len() - 1) as u32
                    })
                })
                .collect();
            ids.sort_unstable();
            ids.dedup();
            factors.push(ids);
        }
        Self::from_ids(value, factors, factor_value)
    }

    /// An instance from members in greedy order and their factor ids `0..factor_value.len()`.
    pub fn from_ids(value: Vec<u64>, factors: Vec<Vec<u32>>, factor_value: Vec<u64>) -> Self {
        let mut listers = vec![Vec::new(); factor_value.len()];
        for (m, list) in factors.iter().enumerate() {
            for &f in list {
                listers[f as usize].push(m as u32);
            }
        }
        Instance { value, factors, listers, factor_value }
    }

    /// The sources' instance of game `n` (docs/theory/sources.md): the members are the numbers in `(n/2, n]`, largest
    /// first, each listing its maximal factors.  `spf` must reach `n`.
    pub fn upper_half(n: u64, spf: &[u32]) -> Self {
        let value: Vec<u64> = (n / 2 + 1..=n).rev().collect();
        let lists: Vec<Vec<u64>> = value.iter().map(|&c| maximal_factors(c, spf)).collect();
        Self::new(value, &lists)
    }

    pub fn len(&self) -> usize {
        self.value.len()
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn num_factors(&self) -> usize {
        self.factor_value.len()
    }

    /// The instance restricted to `members` (indices into `self`, ascending, so greedy order is kept), with
    /// only the factors they list, re-indexed.
    pub fn sub(&self, members: &[u32]) -> Instance {
        debug_assert!(members.windows(2).all(|w| w[0] < w[1]));
        let mut id: HashMap<u32, u32> = HashMap::new();
        let mut factor_value = Vec::new();
        let mut factors = Vec::with_capacity(members.len());
        for &m in members {
            let list: Vec<u32> = self.factors[m as usize]
                .iter()
                .map(|&f| {
                    *id.entry(f).or_insert_with(|| {
                        factor_value.push(self.factor_value[f as usize]);
                        (factor_value.len() - 1) as u32
                    })
                })
                .collect();
            factors.push(list);
        }
        for list in factors.iter_mut() {
            list.sort_unstable();
        }
        let value = members.iter().map(|&m| self.value[m as usize]).collect();
        Self::from_ids(value, factors, factor_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primes::smallest_prime_factors;

    #[test]
    fn upper_half_of_21() {
        let spf = smallest_prime_factors(100);
        let inst = Instance::upper_half(21, &spf);
        assert_eq!(inst.value, (11..=21).rev().collect::<Vec<u64>>());
        // 20 lists 4 and 10.
        let mut f20: Vec<u64> = inst.factors[1].iter().map(|&f| inst.factor_value[f as usize]).collect();
        f20.sort_unstable();
        assert_eq!(f20, vec![4, 10]);
        let sub = inst.sub(&[1, 3]);
        assert_eq!(sub.value, vec![20, 18]);
        assert_eq!(sub.num_factors(), 4);
    }
}
