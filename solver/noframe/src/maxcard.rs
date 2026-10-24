//! Exact maximum-cardinality playable subsets, by branch and bound over cores.
//!
//! The question asked is a threshold one: does the instance have a playable subset of at least `target`
//! members?  A node is a set `I` of members every answer must keep and a set `U` still open (the rest are left
//! out).  At each node:
//!
//! 1. *Reduce.*  Let K = I ∪ U and R its residual.  Some largest playable S with I ⊆ S ⊆ K contains all of K ∖ R (the
//!    exclusion theorem, P8, docs/theory/playability.md, with exclusions E = K ∖ S: K ∖ (E ∩ R) is playable too), so
//!    only R is in question, and a subset of K is playable iff its part in R is (every core of a subset of K lies in
//!    R).
//! 2. *Bound.*  |K ∖ R| + the Lorea rank of R with I ∩ R forced (a matroid: the forced independent set extends
//!    to a basis) is an upper bound ([`crate::bounds`]); below `target`, the node is cut.
//! 3. *Exact.*  If the basis found is playable, it (with K ∖ R) is a playable set as large as the bound.
//! 4. *Branch.*  Otherwise the basis holds a core C; every playable subset leaves out some member of C, and
//!    not one of I's (I is playable).  With C ∩ U = {c₁, …, c_r}, child j leaves out c_j and keeps
//!    c₁ … c_{j−1}: the children partition the answers.
//!
//! The search is exact: a node is cut only by a valid bound, and the branching covers every playable set.

use crate::bounds::{with_ground, Lorea};
use crate::instance::Instance;
use crate::peel::{flagged, mask, minimal_core, Peeler};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// No playable subset reaches the target: proven.
    Below,
    /// A playable subset of at least the target size.
    Found(Vec<u32>),
    /// The node budget ran out.
    Budget,
}

pub struct MaxCard<'a> {
    inst: &'a Instance,
    grounded: Instance,
    lorea: Lorea,
    peeler: Peeler,
    target: usize,
    pub nodes: u64,
    budget: u64,
}

impl<'a> MaxCard<'a> {
    pub fn new(inst: &'a Instance, target: usize, budget: u64) -> Self {
        let grounded = with_ground(inst);
        let lorea = Lorea::new(&grounded);
        MaxCard { inst, grounded, lorea, peeler: Peeler::new(), target, nodes: 0, budget }
    }

    /// Does some playable subset of the whole instance have at least `target` members?
    pub fn run(&mut self) -> Outcome {
        let n = self.inst.len();
        self.node(vec![false; n], vec![true; n])
    }

    fn node(&mut self, forced: Vec<bool>, open: Vec<bool>) -> Outcome {
        self.nodes += 1;
        if self.nodes > self.budget {
            return Outcome::Budget;
        }
        let inst = self.inst;
        let whole: Vec<bool> = forced.iter().zip(&open).map(|(a, b)| *a || *b).collect();
        let mut k = whole.clone();
        let k_size = k.iter().filter(|&&x| x).count();
        let r_size = self.peeler.residual(inst, &mut k);
        let base = k_size - r_size;
        if r_size == 0 {
            return if base >= self.target { Outcome::Found(flagged(&whole)) } else { Outcome::Below };
        }
        let residual = flagged(&k);
        let forced_r: Vec<u32> = residual.iter().copied().filter(|&m| forced[m as usize]).collect();
        let open_r: Vec<u32> = residual.iter().copied().filter(|&m| !forced[m as usize]).collect();
        let basis = self.lorea.basis(&self.grounded, &forced_r, &open_r).expect("the kept set is playable");
        if base + basis.len() < self.target {
            return Outcome::Below;
        }
        let mut b = mask(inst.len(), &basis);
        if self.peeler.residual(inst, &mut b) == 0 {
            // The basis with everything outside the residual.
            let mut found = whole;
            for &m in &residual {
                found[m as usize] = false;
            }
            for &m in &basis {
                found[m as usize] = true;
            }
            return Outcome::Found(flagged(&found));
        }
        let core = minimal_core(inst, &mut self.peeler, &flagged(&b));
        let branch: Vec<u32> = core.iter().copied().filter(|&c| !forced[c as usize]).collect();
        assert!(!branch.is_empty(), "a core inside the kept set, which is playable");
        let mut kept = forced;
        let mut left = open;
        for (j, &c) in branch.iter().enumerate() {
            if j > 0 {
                // Child j keeps c₁ … c_{j−1}; if they cannot all be kept, neither can any later child's.
                kept[branch[j - 1] as usize] = true;
                if !self.peeler.playable_mask(inst, &kept) {
                    break;
                }
            }
            left[c as usize] = false;
            match self.node(kept.clone(), left.clone()) {
                Outcome::Below => {}
                other => return other,
            }
        }
        Outcome::Below
    }
}

/// The largest playable subset size, exactly (for tests and small instances): raises the target until it fails.
pub fn max_card(inst: &Instance, budget: u64) -> Option<usize> {
    let mut best = 0;
    loop {
        match MaxCard::new(inst, best + 1, budget).run() {
            Outcome::Found(s) => best = s.len().max(best + 1),
            Outcome::Below => return Some(best),
            Outcome::Budget => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn brute_max(inst: &Instance) -> usize {
        let mut p = Peeler::new();
        let m = inst.len();
        (0u32..(1 << m))
            .filter_map(|bits| {
                let set: Vec<u32> = (0..m as u32).filter(|&i| bits >> i & 1 == 1).collect();
                p.playable(inst, &set).then_some(set.len())
            })
            .max()
            .unwrap()
    }

    #[test]
    fn exact_against_brute_force_on_random_instances() {
        let mut state = 0x2545_F491_4F6C_DD1Du64;
        let mut next = |below: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % below
        };
        for _ in 0..600 {
            let members = 2 + next(12) as usize;
            let factors = 2 + next(9);
            let lists: Vec<Vec<u64>> = (0..members)
                .map(|_| {
                    let k = 1 + next(3) as usize;
                    let mut l: Vec<u64> = (0..k).map(|_| next(factors)).collect();
                    l.sort_unstable();
                    l.dedup();
                    l
                })
                .collect();
            let inst = Instance::new((0..members as u64).rev().collect(), &lists);
            let want = brute_max(&inst);
            assert_eq!(max_card(&inst, u64::MAX), Some(want), "{lists:?}");
            // Found sets are playable and as large as claimed.
            if let Outcome::Found(s) = MaxCard::new(&inst, want, u64::MAX).run() {
                assert!(s.len() >= want && Peeler::new().playable(&inst, &s));
            } else {
                panic!("the maximum is reachable");
            }
        }
    }
}
