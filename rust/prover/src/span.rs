//! The span check (♦): is every rejection of descending greedy a linear dependency over GF(2)?
//!
//! Give each member its incidence vector over GF(2) (a 1 at each of its factors).  A playable set's vectors are
//! independent ([`crate::bounds`], the rank bound), so for every truncation K_{≥t} of the instance no
//! playable subset is larger than the GF(2) rank of K_{≥t}.  Now suppose
//!
//! > **(♦)** whenever descending greedy rejects x, x's vector is a sum of vectors of members greedy kept above x.
//!
//! Then greedy's kept members above any threshold t are independent (they are playable) and span every member
//! above t (the kept ones trivially, the rejected ones by (♦)), so they are a basis of K_{≥t}: greedy keeps
//! rank(K_{≥t}) members, the most any playable subset can have.  That is (★) for every component and every
//! truncation at once, and with it the upper-half optimality conjecture for every weighting consistent with
//! the order (greedy on a matroid).  (♦) is checked here exactly, one game at a time.
//!
//! The run is greedy itself, with linear algebra doing most of the playability work: a member with a factor
//! that no kept member lists is kept (the extension lemma); a member whose vector the kept ones span is rejected
//! (a dependent set is unplayable); only the rest need a peel, and a rejection among them is a violation of
//! (♦), reported for the threshold-by-threshold check of [`crate::star`].  Everything runs on the components of
//! the residual, where all cores and all linear dependencies live (a set whose vectors sum to zero lists every
//! factor an even number of times, so it is self-covering).

use crate::bounds::Gf2Basis;
use crate::instance::Instance;
use crate::peel::{components, Peeler};

#[derive(Clone, Debug, Default)]
pub struct SpanReport {
    pub members: usize,
    pub residual: usize,
    pub residual_components: usize,
    pub largest_residual_component: usize,
    pub kept: usize,
    pub kept_sum: u64,
    pub rejections: usize,
    /// Rejections whose vector the kept members above span: certified by linear algebra.
    pub spanned: usize,
    /// Rejections that are not dependencies: (♦) fails there (labels).
    pub violations: Vec<u64>,
    /// Members that needed a peel to decide.
    pub peels: usize,
    /// The largest GF(2) rank of a residual component (the kept members in it).
    pub largest_rank: usize,
}

pub fn span_check(inst: &Instance) -> SpanReport {
    let mut peeler = Peeler::new();
    let mut report = SpanReport { members: inst.len(), ..SpanReport::default() };
    let mut alive = vec![true; inst.len()];
    report.residual = peeler.residual(inst, &mut alive);
    report.kept = inst.len() - report.residual;
    report.kept_sum = (0..inst.len()).filter(|&m| !alive[m]).map(|m| inst.value[m]).sum();
    let r_components = components(inst, &alive);
    report.residual_components = r_components.len();
    report.largest_residual_component = r_components.iter().map(Vec::len).max().unwrap_or(0);
    for q in &r_components {
        let local = inst.sub(q);
        let mut basis = Gf2Basis::new(local.num_factors());
        let mut kept = vec![false; local.len()];
        let mut listed = vec![0u32; local.num_factors()];
        for x in 0..local.len() {
            let fresh = local.factors[x].iter().any(|&f| listed[f as usize] == 0);
            let accept = if fresh {
                true
            } else if basis.spans(&local, x as u32) {
                report.spanned += 1;
                false
            } else {
                report.peels += 1;
                kept[x] = true;
                let ok = peeler.playable_mask(&local, &kept);
                kept[x] = false;
                if !ok {
                    report.violations.push(local.value[x]);
                }
                ok
            };
            if accept {
                assert!(basis.insert(&local, x as u32), "a playable set is independent");
                kept[x] = true;
                local.factors[x].iter().for_each(|&f| listed[f as usize] += 1);
                report.kept += 1;
                report.kept_sum += local.value[x];
            } else {
                report.rejections += 1;
            }
        }
        report.largest_rank = report.largest_rank.max(basis.rank());
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peel::{flagged, greedy};
    use crate::primes::smallest_prime_factors;
    use crate::star::{check, Engine};

    /// The span check runs the same greedy as the plain one, and agrees with the threshold-by-threshold check.
    #[test]
    fn span_check_is_greedy_and_certifies_small_games() {
        let spf = smallest_prime_factors(3000);
        let mut peeler = Peeler::new();
        for n in (2..=600u64).chain([1155, 2002, 2310, 2873]) {
            let inst = Instance::upper_half(n, &spf);
            let report = span_check(&inst);
            let kept = flagged(&greedy(&inst, &mut peeler));
            assert_eq!(report.kept, kept.len(), "n={n}");
            assert_eq!(report.kept_sum, kept.iter().map(|&m| inst.value[m as usize]).sum::<u64>(), "n={n}");
            assert_eq!(report.rejections, inst.len() - kept.len());
            assert!(report.violations.is_empty(), "n={n}: {:?}", report.violations);
            assert_eq!(report.spanned, report.rejections);
            if n % 50 == 0 {
                assert_eq!(check(&inst, 100_000, Engine::Plus1).rejections, report.rejections);
            }
        }
    }

    /// A rejection that is not a dependency is reported: {0,1,2}, {0,1}, {0,2} is a core (factor 0 three times,
    /// 1 and 2 twice) whose vectors are independent, so greedy's rejection of the third is not spanned.
    #[test]
    fn a_core_without_a_dependency_is_a_violation() {
        let lists: Vec<Vec<u64>> = vec![vec![0, 1, 2], vec![0, 1], vec![0, 2]];
        let inst = Instance::new(vec![3, 2, 1], &lists);
        let report = span_check(&inst);
        assert_eq!((report.kept, report.rejections, report.spanned), (2, 1, 0));
        assert_eq!(report.violations, vec![1]);
    }
}
