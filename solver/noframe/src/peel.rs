//! Dismantling, residuals and playability (docs/theory/decision-procedure.md, "Unplayable sets: cores and the
//! residual").
//!
//! A set of members is *self-covering* when every factor of every member is listed by another member of the
//! set.  *Dismantling* repeatedly removes a member holding a factor that no other remaining member lists; its
//! fixpoint, the *residual*, is the union of all self-covering subsets (Theorem 4), and on a separated instance a
//! set is playable exactly when its residual is empty (Theorem 5, which with Theorems 1-3 makes it the same test
//! as peel).  Every instance in this crate is separated.

use crate::instance::Instance;

/// Reusable buffers for dismantling.
#[derive(Default)]
pub struct Peeler {
    degree: Vec<u32>,
    queue: Vec<u32>,
}

impl Peeler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Dismantles the members flagged in `alive`, clearing the flag of every member removed; what stays flagged
    /// is the residual.  Returns its size.
    pub fn residual(&mut self, inst: &Instance, alive: &mut [bool]) -> usize {
        self.degree.clear();
        self.degree.resize(inst.num_factors(), 0);
        let mut left = 0;
        for (m, &on) in alive.iter().enumerate() {
            if on {
                left += 1;
                for &f in &inst.factors[m] {
                    self.degree[f as usize] += 1;
                }
            }
        }
        self.queue.clear();
        for (m, &on) in alive.iter().enumerate() {
            if on && inst.factors[m].iter().any(|&f| self.degree[f as usize] == 1) {
                self.queue.push(m as u32);
            }
        }
        while let Some(m) = self.queue.pop() {
            let m = m as usize;
            if !alive[m] {
                continue;
            }
            alive[m] = false;
            left -= 1;
            for &f in &inst.factors[m] {
                let d = &mut self.degree[f as usize];
                *d -= 1;
                if *d == 1 {
                    let other = inst.listers[f as usize].iter().find(|&&l| alive[l as usize]);
                    self.queue.push(*other.expect("a factor of degree 1 has a live lister"));
                }
            }
        }
        left
    }

    /// The residual of `members` (ascending indices), ascending.
    pub fn residual_of(&mut self, inst: &Instance, members: &[u32]) -> Vec<u32> {
        let mut alive = mask(inst.len(), members);
        self.residual(inst, &mut alive);
        flagged(&alive)
    }

    /// Whether `members` is playable: its residual is empty.
    pub fn playable(&mut self, inst: &Instance, members: &[u32]) -> bool {
        let mut alive = mask(inst.len(), members);
        self.residual(inst, &mut alive) == 0
    }

    /// Whether the members flagged in `alive` are playable (`alive` is left unchanged).
    pub fn playable_mask(&mut self, inst: &Instance, alive: &[bool]) -> bool {
        let mut copy = alive.to_vec();
        self.residual(inst, &mut copy) == 0
    }
}

pub fn mask(len: usize, members: &[u32]) -> Vec<bool> {
    let mut alive = vec![false; len];
    for &m in members {
        alive[m as usize] = true;
    }
    alive
}

pub fn flagged(alive: &[bool]) -> Vec<u32> {
    alive.iter().enumerate().filter(|(_, &on)| on).map(|(m, _)| m as u32).collect()
}

/// The connected components of the flagged members, joined through shared factors; each ascending, in order of
/// their smallest member.
pub fn components(inst: &Instance, alive: &[bool]) -> Vec<Vec<u32>> {
    let mut parent: Vec<u32> = (0..inst.len() as u32).collect();
    fn find(parent: &mut [u32], mut x: u32) -> u32 {
        while parent[x as usize] != x {
            parent[x as usize] = parent[parent[x as usize] as usize];
            x = parent[x as usize];
        }
        x
    }
    for list in &inst.listers {
        let mut first = None;
        for &m in list {
            if alive[m as usize] {
                match first {
                    None => first = Some(m),
                    Some(r) => {
                        let (a, b) = (find(&mut parent, r), find(&mut parent, m));
                        if a != b {
                            parent[a.max(b) as usize] = a.min(b);
                        }
                    }
                }
            }
        }
    }
    let mut index_of_root: Vec<u32> = vec![u32::MAX; inst.len()];
    let mut out: Vec<Vec<u32>> = Vec::new();
    for m in 0..inst.len() as u32 {
        if alive[m as usize] {
            let r = find(&mut parent, m);
            if index_of_root[r as usize] == u32::MAX {
                index_of_root[r as usize] = out.len() as u32;
                out.push(Vec::new());
            }
            out[index_of_root[r as usize] as usize].push(m);
        }
    }
    out
}

/// The component of `x` among the flagged members (ascending).
pub fn component_of(inst: &Instance, alive: &[bool], x: u32) -> Vec<u32> {
    let mut seen = vec![false; inst.len()];
    let mut seen_factor = vec![false; inst.num_factors()];
    let mut stack = vec![x];
    seen[x as usize] = true;
    while let Some(m) = stack.pop() {
        for &f in &inst.factors[m as usize] {
            if !seen_factor[f as usize] {
                seen_factor[f as usize] = true;
                for &l in &inst.listers[f as usize] {
                    if alive[l as usize] && !seen[l as usize] {
                        seen[l as usize] = true;
                        stack.push(l);
                    }
                }
            }
        }
    }
    flagged(&seen)
}

/// Descending greedy: scan the members in index order and keep each one whose addition leaves the kept set
/// playable.  Returns the kept flags.  (On the upper-half instance this is `optimize_mini`, and its set is U*.)
pub fn greedy(inst: &Instance, peeler: &mut Peeler) -> Vec<bool> {
    let mut kept = vec![false; inst.len()];
    let mut listed = vec![0u32; inst.num_factors()];
    for x in 0..inst.len() {
        // A factor no kept member lists makes x safe (the extension lemma); otherwise dismantle.
        let safe = inst.factors[x].iter().any(|&f| listed[f as usize] == 0);
        let accept = safe || {
            kept[x] = true;
            let ok = peeler.playable_mask(inst, &kept);
            kept[x] = false;
            ok
        };
        if accept {
            kept[x] = true;
            for &f in &inst.factors[x] {
                listed[f as usize] += 1;
            }
        }
    }
    kept
}

/// U\* of game n: descending greedy on the upper-half instance (the numbers above n/2), returned in descending order.
/// `spf` must reach `n`.
pub fn u_star(n: u64, spf: &[u32]) -> Vec<u32> {
    let inst = Instance::upper_half(n, spf);
    let kept = greedy(&inst, &mut Peeler::new());
    (0..inst.len()).filter(|&m| kept[m]).map(|m| inst.value[m] as u32).collect()
}

/// Shrinks a self-covering set (a nonempty residual) to an inclusion-minimal one (a *minimal core*) by the
/// deletion filter, trying members in the given order.
pub fn minimal_core(inst: &Instance, peeler: &mut Peeler, core: &[u32]) -> Vec<u32> {
    let mut alive = mask(inst.len(), core);
    for &c in core {
        if !alive[c as usize] {
            continue;
        }
        let mut trial = alive.clone();
        trial[c as usize] = false;
        if peeler.residual(inst, &mut trial) > 0 {
            alive = trial;
        }
    }
    flagged(&alive)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primes::smallest_prime_factors;

    fn values(inst: &Instance, ms: &[u32]) -> Vec<u64> {
        ms.iter().map(|&m| inst.value[m as usize]).collect()
    }

    #[test]
    fn the_n39_core_is_self_covering_and_minimal() {
        let spf = smallest_prime_factors(100);
        let lists: Vec<Vec<u64>> =
            [39u64, 33, 26, 22].iter().map(|&c| crate::primes::maximal_factors(c, &spf)).collect();
        let inst = Instance::new(vec![39, 33, 26, 22], &lists);
        let mut p = Peeler::new();
        assert_eq!(p.residual_of(&inst, &[0, 1, 2, 3]), vec![0, 1, 2, 3]);
        for drop in 0..4u32 {
            let rest: Vec<u32> = (0..4).filter(|&m| m != drop).collect();
            assert!(p.playable(&inst, &rest));
        }
        assert_eq!(minimal_core(&inst, &mut p, &[0, 1, 2, 3]).len(), 4);
    }

    #[test]
    fn greedy_on_21_keeps_the_known_upper_half() {
        // U* of 21, as heuristics/core.py's optimize_mini computes it: one prime (the largest, 19; all primes
        // pay with 1) and the composites that fit.
        let spf = smallest_prime_factors(100);
        let inst = Instance::upper_half(21, &spf);
        let mut p = Peeler::new();
        let kept = greedy(&inst, &mut p);
        let got = values(&inst, &flagged(&kept));
        assert_eq!(got, vec![21, 20, 19, 18, 16, 15, 14, 12]);
    }
}
