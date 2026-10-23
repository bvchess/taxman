//! Who can pay for whom: each number's maximal factors and maximal composites, restricted to the frames the
//! number takes part in.
//!
//! A move `m` is paid for by one of its *maximal factors* `m/p` (`p` a prime dividing `m`), and a number `y` can
//! pay for its *maximal composites* `y·p`.  Within a frame set, a number can only pay for a move of the frame its
//! factors are in, so [`FrameGraph::build`](super::FrameGraph::build) keeps only the pairs whose roles can meet: `y` a
//! factor of frame `i`, `x` a move of frame `i` or a factor of frame `i + 1` (promoted into frame `i`).  That keeps
//! every live neighbor a peel or a reduction can see and drops the rest.  Numbers are stored as `u16`, which
//! bounds boards at 65535 (the solver stops at [`crate::solver::MAX_N`]).

use crate::numtheory::Tables;

/// Most maximal factors a supported number has: 2·3·5·7·11·13 = 30030 has six distinct primes, and seven need
/// 510510, beyond `u16`.
const MAX_FACTORS: usize = 6;

/// For each number, the maximal factors that can pay for it (inline: the list, then its length in the last slot)
/// and the maximal composites it can pay for (flat CSR arrays).  Numbers beyond the provider's range have empty
/// lists.
#[derive(Clone, Debug)]
pub struct FactorProvider {
    fac: Vec<[u16; MAX_FACTORS + 1]>,
    comp_start: Vec<u32>,
    comp: Vec<u16>,
}

impl FactorProvider {
    /// Each number's maximal factors and maximal composites, restricted to `numbers`.  `tables` must cover the largest
    /// of `numbers`.
    pub fn new(tables: &Tables, numbers: &[u32]) -> Self {
        let max = numbers.iter().copied().max().unwrap_or(0);
        let mut member = vec![0u16; max as usize + 1];
        for &x in numbers {
            member[x as usize] = 1;
        }
        Self::with_roles(tables, &member, &member)
    }

    /// Lists for a graph in which `y` can pay for `x` exactly when `y` is a maximal factor of `x` and
    /// `as_move[x] & as_factor[y] != 0` (bit `i` standing for frame `i`, say).  Both slices have one entry per
    /// number `0..=max`.
    pub(super) fn with_roles(tables: &Tables, as_move: &[u16], as_factor: &[u16]) -> Self {
        assert_eq!(as_move.len(), as_factor.len());
        let max = as_move.len() - 1;
        assert!(max as u32 <= tables.n, "tables for {} cannot cover {max}", tables.n);
        assert!(max <= usize::from(u16::MAX), "numbers above {} are not supported", u16::MAX);
        let mut fac = vec![[0u16; MAX_FACTORS + 1]; max + 1];
        let mut comp_start = Vec::with_capacity(max + 2);
        let mut comp = Vec::new();
        comp_start.push(0);
        for x in 0..=max {
            if as_move[x] != 0 {
                let mine = as_move[x];
                let payers = tables.max_factors[x].iter().filter(|&&y| as_factor[y as usize] & mine != 0);
                for (k, &y) in payers.enumerate() {
                    fac[x][k] = y as u16;
                    fac[x][MAX_FACTORS] = k as u16 + 1;
                }
            }
            if as_factor[x] != 0 {
                let mine = as_factor[x];
                let composites = tables.max_composites[x].iter().take_while(|&&c| c as usize <= max);
                comp.extend(composites.filter(|&&c| as_move[c as usize] & mine != 0).map(|&c| c as u16));
            }
            comp_start.push(comp.len() as u32);
        }
        FactorProvider { fac, comp_start, comp }
    }

    /// The maximal factors of `n` that can pay for it, ascending.
    #[inline]
    pub fn factors(&self, n: u32) -> &[u16] {
        match self.fac.get(n as usize) {
            Some(list) => &list[..usize::from(list[MAX_FACTORS]).min(MAX_FACTORS)],
            None => &[],
        }
    }

    /// The maximal composites of `n` that it can pay for, ascending.
    #[inline]
    pub fn composites(&self, n: u32) -> &[u16] {
        let i = n as usize;
        if i + 1 < self.comp_start.len() {
            &self.comp[self.comp_start[i] as usize..self.comp_start[i + 1] as usize]
        } else {
            &[]
        }
    }
}
