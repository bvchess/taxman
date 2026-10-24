//! The check of conjecture (★) on one instance (the cardinality conjecture, S4, docs/theory/sources.md).
//!
//! (★) says: on every connected component K of the instance and every truncation K_{≥t} of one, descending
//! greedy keeps a maximum-cardinality playable subset.  It is checked exactly as follows.
//!
//! * **Only the thresholds at rejections matter.**  Going down past a member x, greedy's count rises by one if
//!   it keeps x and the maximum rises by at most one; the maximum is never below greedy's count.  So equality
//!   at every threshold follows from equality at each threshold where greedy *rejects* x: the maximum of
//!   K_{≥x} must not exceed that of K_{>x}.
//! * **Only the residual matters.**  Every core of a subset of K lies in the residual R of K, greedy keeps every
//!   member outside R, and its decisions inside R depend only on R (a set is playable iff its part in R is).  So
//!   greedy is run on each component Q of R alone.
//! * **Only x's new jam matters.**  At a rejection x, the residual of Q_{≥x} is that of Q_{>x} plus the cores
//!   through x, which are connected to x.  The components of the residual not containing x were settled at
//!   earlier thresholds; the one containing x, P, must satisfy: the fewest members whose removal makes P
//!   playable is g = |P ∖ G|, greedy's own exclusions in P (G is greedy's set; G ∩ P is playable, so that many
//!   always suffice).  (At an acceptance, a merged component needs no check: it needs at least the sum of its
//!   parts' exclusions, and greedy uses exactly that many.)
//!
//! Each such P is decided by the cheapest argument that works: a single exclusion is always needed (P is a
//! nonempty residual, so it is unplayable); the matching-number bound |P| − ν(P); the Lorea bound |P| − rank(P); the
//! rank bound |P| − rank_GF(2)(P) ([`crate::bounds`]); or an
//! exact search, which either proves the bound or finds a counterexample.  The exact search is
//! [`crate::plus1`], which leans on the earlier thresholds of the same component (each needs exactly its g);
//! a threshold decided after an undecided one in the same component is marked conditional.

use crate::bounds::{gf2_rank, lorea_rank, matching_number, with_ground};
use crate::instance::Instance;
use crate::maxcard::{MaxCard, Outcome};
use crate::peel::{component_of, components, flagged, greedy, Peeler};
use crate::plus1::Plus1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Method {
    /// g = 1: P is unplayable, so at least one exclusion is needed.
    Core,
    /// |P| − ν(P) >= g.
    Matching,
    /// |P| − rank_Lorea(P) >= g.
    Lorea,
    /// |P| − rank_GF(2)(P) >= g.
    Rank,
    /// An exact search proved it.
    Exact,
    /// The exact search ran out of budget.
    Undecided,
    /// A playable subset of P larger than greedy's: (★) is false here.
    Counterexample,
}

impl Method {
    pub const ALL: [Method; 7] = [
        Method::Core,
        Method::Matching,
        Method::Lorea,
        Method::Rank,
        Method::Exact,
        Method::Undecided,
        Method::Counterexample,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Method::Core => "core",
            Method::Matching => "matching",
            Method::Lorea => "lorea",
            Method::Rank => "rank",
            Method::Exact => "exact",
            Method::Undecided => "undecided",
            Method::Counterexample => "counterexample",
        }
    }
}

/// Which exact search decides the thresholds the bounds leave open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    /// [`crate::plus1`]: the incremental search (default).
    Plus1,
    /// [`crate::maxcard`]: the from-scratch search on P alone (no reliance on earlier thresholds).
    Scratch,
}

/// One threshold: greedy rejected `x`, and P is x's component of the residual of its R-component down to x.
#[derive(Clone, Debug)]
pub struct Threshold {
    pub x: u64,
    pub p_size: usize,
    /// Greedy's exclusions in P.
    pub g: usize,
    /// The matching-number bound's lower bound on the exclusions: |P| − ν(P).
    pub matching_lb: usize,
    /// The Lorea bound's: |P| − rank(P) (computed only when the matching-number bound falls short).
    pub lorea_lb: Option<usize>,
    /// The rank bound's: |P| − rank_GF(2)(P) (computed only when the Lorea bound falls short).
    pub rank_lb: Option<usize>,
    pub method: Method,
    pub nodes: u64,
    /// Decided by the incremental search after an undecided threshold of the same component.
    pub conditional: bool,
    /// For a counterexample: P's members, greedy's subset of P, and a larger playable subset.
    pub witness: Option<(Vec<u64>, Vec<u64>, Vec<u64>)>,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub members: usize,
    pub residual: usize,
    pub residual_components: usize,
    pub largest_residual_component: usize,
    pub rejections: usize,
    /// Components of the whole instance in which greedy rejects something.
    pub rejecting_components: usize,
    /// Per method, the rejecting components whose hardest threshold needed it.
    pub components_by_method: [usize; 7],
    pub thresholds: Vec<Threshold>,
    /// Greedy's set, as labels (in member order).
    pub kept: Vec<u64>,
}

impl Report {
    pub fn count(&self, method: Method) -> usize {
        self.thresholds.iter().filter(|t| t.method == method).count()
    }
}

/// Checks (★) on every component and truncation of `inst`; `budget` caps the nodes of each exact search.
pub fn check(inst: &Instance, budget: u64, engine: Engine) -> Report {
    let mut peeler = Peeler::new();
    let mut report = Report { members: inst.len(), ..Report::default() };
    let all = vec![true; inst.len()];
    let mut component_id = vec![0u32; inst.len()];
    for (i, comp) in components(inst, &all).iter().enumerate() {
        for &m in comp {
            component_id[m as usize] = i as u32;
        }
    }
    let mut alive = all.clone();
    report.residual = peeler.residual(inst, &mut alive);
    let mut kept_all: Vec<bool> = alive.iter().map(|&in_r| !in_r).collect();
    let r_components = components(inst, &alive);
    report.residual_components = r_components.len();
    report.largest_residual_component = r_components.iter().map(Vec::len).max().unwrap_or(0);
    let mut hardest: std::collections::BTreeMap<u32, Method> = std::collections::BTreeMap::new();
    for q in &r_components {
        let local = inst.sub(q);
        let kept = greedy(&local, &mut peeler);
        for (i, &k) in kept.iter().enumerate() {
            kept_all[q[i] as usize] = k;
        }
        let g_of = |set: &[u32]| set.iter().filter(|&&m| !kept[m as usize]).count();
        // The settled jams of this component so far: components of residuals above the current threshold.
        let mut family: Vec<(Vec<u32>, usize)> = Vec::new();
        // Cores found by the exact searches (indices of `local`): still cores at every later threshold.
        let mut cores: Vec<Vec<u32>> = Vec::new();
        let mut undecided_before = false;
        for x in (0..local.len()).filter(|&x| !kept[x]) {
            report.rejections += 1;
            // The residuals just above x and down to x (cores through x may use members the first dismantles).
            let mut above = vec![false; local.len()];
            above[..x].iter_mut().for_each(|a| *a = true);
            let mut down_to = above.clone();
            down_to[x] = true;
            peeler.residual(&local, &mut above);
            peeler.residual(&local, &mut down_to);
            assert!(down_to[x], "a rejected member lies in a core of the members down to it");
            let p = component_of(&local, &down_to, x as u32);
            let mut in_p = vec![false; local.len()];
            p.iter().for_each(|&m| in_p[m as usize] = true);
            let old: Vec<Vec<u32>> = components(&local, &above).into_iter().filter(|c| in_p[c[0] as usize]).collect();
            for c in &old {
                if !family.iter().any(|(s, _)| s == c) {
                    family.push((c.clone(), g_of(c)));
                }
            }
            let inside: Vec<(Vec<u32>, usize)> = family.iter().filter(|(s, _)| in_p[s[0] as usize]).cloned().collect();
            let mut threshold = decide(&local, &p, &kept, &old, &inside, &mut cores, budget, engine, &mut peeler);
            threshold.conditional = undecided_before && threshold.method == Method::Exact && engine == Engine::Plus1;
            undecided_before |= threshold.method == Method::Undecided;
            family.push((p.clone(), threshold.g));
            let comp = component_id[q[x] as usize];
            let entry = hardest.entry(comp).or_insert(threshold.method);
            *entry = (*entry).max(threshold.method);
            report.thresholds.push(Threshold { x: local.value[x], ..threshold });
        }
    }
    report.rejecting_components = hardest.len();
    for method in hardest.values() {
        report.components_by_method[*method as usize] += 1;
    }
    report.kept = flagged(&kept_all).iter().map(|&m| inst.value[m as usize]).collect();
    report
}

/// Decides one threshold's P (members of `local`, ascending) against greedy's `kept` flags.  `old` are the
/// components of the residual just above x inside P; `family` every settled jam inside P (`old` included);
/// `cores` the cores known in the component, to which the exact search adds what it finds.
#[allow(clippy::too_many_arguments)]
fn decide(
    local: &Instance,
    p: &[u32],
    kept: &[bool],
    old: &[Vec<u32>],
    family: &[(Vec<u32>, usize)],
    cores: &mut Vec<Vec<u32>>,
    budget: u64,
    engine: Engine,
    peeler: &mut Peeler,
) -> Threshold {
    let g = p.iter().filter(|&&m| !kept[m as usize]).count();
    let sub = local.sub(p);
    let all: Vec<u32> = (0..sub.len() as u32).collect();
    let matching_lb = sub.len() - matching_number(&sub, &all);
    let mut t = Threshold {
        x: 0,
        p_size: p.len(),
        g,
        matching_lb,
        lorea_lb: None,
        rank_lb: None,
        method: Method::Core,
        nodes: 0,
        conditional: false,
        witness: None,
    };
    if g == 1 {
        return t;
    }
    if matching_lb >= g {
        t.method = Method::Matching;
        return t;
    }
    let lorea_lb = sub.len() - lorea_rank(&with_ground(&sub), &all);
    t.lorea_lb = Some(lorea_lb);
    if lorea_lb >= g {
        t.method = Method::Lorea;
        return t;
    }
    let rank_lb = sub.len() - gf2_rank(&sub, &all);
    t.rank_lb = Some(rank_lb);
    if rank_lb >= g {
        t.method = Method::Rank;
        return t;
    }
    let outcome = match engine {
        Engine::Scratch => {
            let mut search = MaxCard::new(&sub, sub.len() - g + 1, budget);
            let outcome = search.run();
            t.nodes = search.nodes;
            outcome
        }
        Engine::Plus1 => {
            // Re-index the family into P's local indices.
            let mut pos = vec![u32::MAX; local.len()];
            p.iter().enumerate().for_each(|(i, &m)| pos[m as usize] = i as u32);
            let sets: Vec<(Vec<u32>, usize)> =
                family.iter().map(|(s, g)| (s.iter().map(|&m| pos[m as usize]).collect(), *g)).collect();
            let b: usize = old.iter().map(|c| c.iter().filter(|&&m| !kept[m as usize]).count()).sum();
            assert_eq!(b + 1, g, "the new members hold exactly one rejection, x");
            // Known cores inside P (a core is connected, so it lies inside P or misses it).
            let known: Vec<Vec<u32>> = cores
                .iter()
                .filter(|c| pos[c[0] as usize] != u32::MAX)
                .map(|c| c.iter().map(|&m| pos[m as usize]).collect())
                .collect();
            let reused = known.len();
            let mut search = Plus1::new(&sub, &sets, b, budget, known);
            let outcome = search.run();
            t.nodes = search.nodes;
            cores.extend(search.cores[reused..].iter().map(|c| c.iter().map(|&i| p[i as usize]).collect()));
            outcome
        }
    };
    t.method = match outcome {
        Outcome::Below => Method::Exact,
        Outcome::Budget => Method::Undecided,
        Outcome::Found(set) => {
            assert!(peeler.playable(&sub, &set) && set.len() > sub.len() - g, "a found set must beat greedy");
            let greedy_set: Vec<u64> =
                p.iter().filter(|&&m| kept[m as usize]).map(|&m| local.value[m as usize]).collect();
            let found: Vec<u64> = set.iter().map(|&m| sub.value[m as usize]).collect();
            t.witness = Some((sub.value.clone(), greedy_set, found));
            Method::Counterexample
        }
    };
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maxcard::max_card;
    use crate::primes::smallest_prime_factors;

    /// (★) by definition on small games: for every threshold, greedy's count equals the exact maximum of the
    /// truncation, found by the from-scratch search on the whole truncation (no components, residual or
    /// thresholds; that search is itself checked against brute force on random instances).
    #[test]
    fn check_agrees_with_the_definition_on_small_games() {
        let spf = smallest_prime_factors(400);
        let mut peeler = Peeler::new();
        for n in 2..=160u64 {
            let inst = Instance::upper_half(n, &spf);
            let report = check(&inst, u64::MAX, Engine::Plus1);
            assert!(report.thresholds.iter().all(|t| t.method <= Method::Exact), "n={n}");
            let kept = greedy(&inst, &mut peeler);
            assert_eq!(report.kept, flagged(&kept).iter().map(|&m| inst.value[m as usize]).collect::<Vec<_>>());
            for t in 1..=inst.len() {
                let top: Vec<u32> = (0..t as u32).collect();
                let truncated = inst.sub(&top);
                let greedy_count = kept[..t].iter().filter(|&&k| k).count();
                assert_eq!(max_card(&truncated, u64::MAX), Some(greedy_count), "n={n} t={t}");
            }
        }
    }

    /// On random instances, where greedy is often not maximum, the incremental engine finds exactly the
    /// counterexamples the from-scratch engine finds (which is checked against brute force in `maxcard`).
    #[test]
    fn the_engines_agree_on_random_instances() {
        let mut state = 0x5DEE_CE66_D1CE_4E5Bu64;
        let mut next = |below: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % below
        };
        let (mut counterexamples, mut exact) = (0, 0);
        for _ in 0..3000 {
            let members = 6 + next(30) as usize;
            let factors = 4 + next(members as u64);
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
            let a = check(&inst, u64::MAX, Engine::Plus1);
            let b = check(&inst, u64::MAX, Engine::Scratch);
            // Up to the first counterexample of a component the engines answer the same question; after it the
            // incremental engine's premises (each earlier jam needs exactly its g) are false, so stop there.
            for (s, t) in a.thresholds.iter().zip(&b.thresholds) {
                assert_eq!(s.x, t.x);
                assert_eq!(s.method, t.method, "{lists:?} x={}", s.x);
                counterexamples += usize::from(t.method == Method::Counterexample);
                exact += usize::from(t.method == Method::Exact);
                if t.method == Method::Counterexample {
                    break;
                }
            }
        }
        assert!(counterexamples > 100 && exact > 100, "{counterexamples} counterexamples, {exact} exact");
    }

    /// The two exact engines agree on every threshold they both decide, over games with real jams.
    #[test]
    fn the_engines_agree() {
        let spf = smallest_prime_factors(3000);
        for n in [1155u64, 1500, 2002, 2310, 2873] {
            let inst = Instance::upper_half(n, &spf);
            let a = check(&inst, 300_000, Engine::Plus1);
            let b = check(&inst, 300_000, Engine::Scratch);
            assert_eq!(a.thresholds.len(), b.thresholds.len());
            for (s, t) in a.thresholds.iter().zip(&b.thresholds) {
                assert_eq!(s.x, t.x);
                if s.method != Method::Undecided && t.method != Method::Undecided {
                    assert_eq!(s.method, t.method, "n={n} x={}", s.x);
                }
            }
        }
    }
}
