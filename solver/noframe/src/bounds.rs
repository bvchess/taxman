//! Upper bounds on the largest playable subset of a set of members.
//!
//! * The **matching-number bound** ν: a playable set gives its members distinct factors (Theorem 1), so it is
//!   matchable, and no playable subset is larger than a maximum matching of members to factors.
//! * The **hypergraphic-matroid bound** (Lorea's matroid).  Add one *ground* factor to the list of every member
//!   with a single factor (the primes and prime powers), so that every list has at least two entries.  A
//!   playable set stays playable (the ground never needs to be private: whatever order dismantles the set
//!   without it still works), and a nonempty playable set X whose lists all have two or more entries covers at
//!   least |X| + 1 factors: dismantle it, give each member the factor that was private when it went, and the last
//!   member has a second one of its own.  Every subset of a playable set is playable, so a playable set is
//!   independent in the matroid whose independent sets are those S with |factors(X)| >= |X| + 1 for every
//!   nonempty X ⊆ S (Lorea 1975: the matroid induced by the submodular function |∪X| − 1).  Its rank is
//!   therefore an upper bound on the largest playable subset, never weaker than ν, and computable by the matroid
//!   greedy algorithm.  Independence is tested with a matching: S is independent iff it is matchable and every
//!   member can reach an unmatched factor by an alternating path (the matching survives doubling any member,
//!   which is Hall's condition with the +1).
//! * The **rank bound**.  Give each member the vector over GF(2) with a 1 at each of its factors.  A
//!   playable set's vectors are linearly independent: in a dismantling order x₁, x₂, … each x_i has a factor
//!   that no later member lists, so in any vanishing sum the earliest member's private coordinate is 1.  So no
//!   playable subset is larger than the GF(2) rank.  On members with at most two factors this is exactly the
//!   graphic-matroid rank (the forest bound, which is exact there); on three-prime members, whose factors pq, pr,
//!   qr are the edges of a triangle on the primes, a dependent set is a 2-cycle mod 2 (a closed surface of
//!   triangles), invisible to counting bounds like Lorea's.  A minimal dependent set has every factor an even
//!   number of times, so it is a core.

use crate::instance::Instance;

const NONE: u32 = u32::MAX;

/// A bipartite matching of members to factors with an undo log.
pub struct Matcher {
    mate_m: Vec<u32>,
    mate_f: Vec<u32>,
    stamp: Vec<u32>,
    epoch: u32,
    log: Vec<(bool, u32, u32)>,
    stack: Vec<(u32, u32)>,
    via: Vec<u32>,
    good: Vec<bool>,
    queue: Vec<u32>,
}

impl Matcher {
    pub fn new(inst: &Instance) -> Self {
        Matcher {
            mate_m: vec![NONE; inst.len()],
            mate_f: vec![NONE; inst.num_factors()],
            stamp: vec![0; inst.num_factors()],
            epoch: 0,
            log: Vec::new(),
            stack: Vec::new(),
            via: Vec::new(),
            good: vec![false; inst.len()],
            queue: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.mate_m.iter_mut().for_each(|x| *x = NONE);
        self.mate_f.iter_mut().for_each(|x| *x = NONE);
        self.log.clear();
    }

    fn set_member(&mut self, m: u32, f: u32) {
        self.log.push((true, m, self.mate_m[m as usize]));
        self.mate_m[m as usize] = f;
    }

    fn set_factor(&mut self, f: u32, m: u32) {
        self.log.push((false, f, self.mate_f[f as usize]));
        self.mate_f[f as usize] = m;
    }

    pub fn checkpoint(&self) -> usize {
        self.log.len()
    }

    pub fn undo(&mut self, to: usize) {
        while self.log.len() > to {
            let (is_member, i, old) = self.log.pop().unwrap();
            if is_member {
                self.mate_m[i as usize] = old;
            } else {
                self.mate_f[i as usize] = old;
            }
        }
    }

    /// Looks for an augmenting path from the unmatched member `root`; on success applies it and returns true.
    pub fn augment(&mut self, inst: &Instance, root: u32) -> bool {
        debug_assert_eq!(self.mate_m[root as usize], NONE);
        self.epoch = self.epoch.wrapping_add(1);
        if self.epoch == 0 {
            self.stamp.iter_mut().for_each(|s| *s = 0);
            self.epoch = 1;
        }
        self.stack.clear();
        self.via.clear();
        self.stack.push((root, 0));
        while let Some(&(m, i)) = self.stack.last() {
            let edges = &inst.factors[m as usize];
            if i as usize == edges.len() {
                self.stack.pop();
                self.via.pop();
                continue;
            }
            self.stack.last_mut().unwrap().1 += 1;
            let f = edges[i as usize];
            if self.stamp[f as usize] == self.epoch {
                continue;
            }
            self.stamp[f as usize] = self.epoch;
            let w = self.mate_f[f as usize];
            if w == NONE {
                // Flip the path: stack[k] takes f, and stack[j] takes via[j] for j < k.
                let k = self.stack.len() - 1;
                for j in (0..=k).rev() {
                    let member = self.stack[j].0;
                    let factor = if j == k { f } else { self.via[j] };
                    self.set_member(member, factor);
                    self.set_factor(factor, member);
                }
                return true;
            }
            self.via.push(f);
            self.stack.push((w, 0));
        }
        false
    }

    /// Whether every member flagged in `in_set` (all matched) can reach an unmatched factor by an alternating
    /// path, i.e. whether the set survives doubling any one member.
    fn all_can_reach_free(&mut self, inst: &Instance, members: &[u32], in_set: &[bool]) -> bool {
        self.queue.clear();
        for &m in members {
            self.good[m as usize] = false;
        }
        for &m in members {
            if inst.factors[m as usize].iter().any(|&f| self.mate_f[f as usize] == NONE) {
                self.good[m as usize] = true;
                self.queue.push(m);
            }
        }
        let mut count = self.queue.len();
        let mut head = 0;
        while head < self.queue.len() {
            let m = self.queue[head];
            head += 1;
            let f = self.mate_m[m as usize];
            for &l in &inst.listers[f as usize] {
                if in_set[l as usize] && !self.good[l as usize] && l != m {
                    self.good[l as usize] = true;
                    count += 1;
                    self.queue.push(l);
                }
            }
        }
        count == members.len()
    }
}

/// The matching number of `members`.
pub fn matching_number(inst: &Instance, members: &[u32]) -> usize {
    let mut matcher = Matcher::new(inst);
    members.iter().filter(|&&m| matcher.augment(inst, m)).count()
}

/// `inst` with a ground factor added to every member that lists a single factor.
pub fn with_ground(inst: &Instance) -> Instance {
    let ground = inst.num_factors() as u32;
    let factors: Vec<Vec<u32>> = inst
        .factors
        .iter()
        .map(|l| {
            let mut l = l.clone();
            if l.len() == 1 {
                l.push(ground);
            }
            l
        })
        .collect();
    let mut factor_value = inst.factor_value.clone();
    factor_value.push(0);
    Instance::from_ids(inst.value.clone(), factors, factor_value)
}

/// The matroid greedy for Lorea's hypergraphic matroid on a ground-extended instance (see [`with_ground`]):
/// starts from `forced` (which must be independent) and adds the `candidates` in order whenever independence
/// survives.  Returns the basis, which has the rank of `forced ∪ candidates` as its size.
pub struct Lorea {
    matcher: Matcher,
    in_set: Vec<bool>,
}

impl Lorea {
    pub fn new(grounded: &Instance) -> Self {
        Lorea { matcher: Matcher::new(grounded), in_set: vec![false; grounded.len()] }
    }

    pub fn basis(&mut self, grounded: &Instance, forced: &[u32], candidates: &[u32]) -> Option<Vec<u32>> {
        self.matcher.reset();
        self.in_set.iter_mut().for_each(|x| *x = false);
        let mut basis: Vec<u32> = Vec::with_capacity(forced.len() + candidates.len());
        for (i, &e) in forced.iter().chain(candidates.iter()).enumerate() {
            let is_forced = i < forced.len();
            let mark = self.matcher.checkpoint();
            if !self.matcher.augment(grounded, e) {
                if is_forced {
                    return None;
                }
                continue;
            }
            self.in_set[e as usize] = true;
            basis.push(e);
            if !self.matcher.all_can_reach_free(grounded, &basis, &self.in_set) {
                self.in_set[e as usize] = false;
                basis.pop();
                self.matcher.undo(mark);
                if is_forced {
                    return None;
                }
            }
        }
        Some(basis)
    }
}

/// The rank of `members` in Lorea's matroid (built on the ground-extended `grounded`).
pub fn lorea_rank(grounded: &Instance, members: &[u32]) -> usize {
    Lorea::new(grounded).basis(grounded, &[], members).expect("nothing forced").len()
}

/// An incremental basis of members' incidence vectors over GF(2), in echelon form: each row's pivot is its
/// lowest set bit, and no two rows share a pivot.
pub struct Gf2Basis {
    rows: Vec<Vec<u64>>,
    row_of_pivot: Vec<u32>,
    scratch: Vec<u64>,
}

impl Gf2Basis {
    pub fn new(num_factors: usize) -> Self {
        let words = num_factors.div_ceil(64).max(1);
        Gf2Basis { rows: Vec::new(), row_of_pivot: vec![NONE; words * 64], scratch: vec![0; words] }
    }

    pub fn rank(&self) -> usize {
        self.rows.len()
    }

    pub fn clear(&mut self) {
        for row in self.rows.drain(..) {
            let pivot = lowest(&row).expect("rows are nonzero");
            self.row_of_pivot[pivot] = NONE;
        }
    }

    /// Adds member `m`'s vector if it is independent of the rows; returns whether it was.
    pub fn insert(&mut self, inst: &Instance, m: u32) -> bool {
        match self.reduce(inst, m) {
            Some(b) => {
                self.row_of_pivot[b] = self.rows.len() as u32;
                self.rows.push(self.scratch.clone());
                true
            }
            None => false,
        }
    }

    /// Whether member `m`'s vector lies in the span of the rows (which are left unchanged).
    pub fn spans(&mut self, inst: &Instance, m: u32) -> bool {
        self.reduce(inst, m).is_none()
    }

    /// Reduces `m`'s vector by the rows into `scratch`; returns its pivot if it is not zero.
    fn reduce(&mut self, inst: &Instance, m: u32) -> Option<usize> {
        let v = &mut self.scratch;
        v.iter_mut().for_each(|w| *w = 0);
        for &f in &inst.factors[m as usize] {
            v[f as usize / 64] ^= 1 << (f % 64);
        }
        let mut from = 0;
        while let Some(b) = lowest_from(v, from) {
            let r = self.row_of_pivot[b];
            if r == NONE {
                return Some(b);
            }
            // Rows have no bits below their pivot, so the words before b's stay zero.
            let w0 = b / 64;
            for (a, c) in v[w0..].iter_mut().zip(&self.rows[r as usize][w0..]) {
                *a ^= c;
            }
            from = w0;
        }
        None
    }
}

fn lowest(v: &[u64]) -> Option<usize> {
    lowest_from(v, 0)
}

/// The lowest set bit of `v`, given that the words before `from` are zero.
fn lowest_from(v: &[u64], from: usize) -> Option<usize> {
    v[from..].iter().enumerate().find(|(_, &w)| w != 0).map(|(i, w)| (from + i) * 64 + w.trailing_zeros() as usize)
}

/// The GF(2) rank of `members`' incidence vectors.
pub fn gf2_rank(inst: &Instance, members: &[u32]) -> usize {
    let mut basis = Gf2Basis::new(inst.num_factors());
    members.iter().filter(|&&m| basis.insert(inst, m)).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::peel::Peeler;

    /// Brute force over all subsets: the largest playable, matchable and Lorea-independent subsets.
    fn brute(inst: &Instance) -> (usize, usize, usize) {
        let grounded = with_ground(inst);
        let m = inst.len();
        let mut p = Peeler::new();
        let (mut play, mut matchable, mut indep) = (0, 0, 0);
        for bits in 0u32..(1 << m) {
            let set: Vec<u32> = (0..m as u32).filter(|&i| bits >> i & 1 == 1).collect();
            if p.playable(inst, &set) {
                play = play.max(set.len());
            }
            if matching_number(inst, &set) == set.len() {
                matchable = matchable.max(set.len());
            }
            // Lorea independence by definition: every nonempty subset covers |X| + 1 factors.
            let ok = (1u32..(1 << set.len())).all(|sub| {
                let xs: Vec<u32> = (0..set.len()).filter(|&i| sub >> i & 1 == 1).map(|i| set[i]).collect();
                let mut cover: Vec<u32> = xs.iter().flat_map(|&x| grounded.factors[x as usize].clone()).collect();
                cover.sort_unstable();
                cover.dedup();
                cover.len() > xs.len()
            });
            if ok {
                indep = indep.max(set.len());
            }
        }
        (play, matchable, indep)
    }

    #[test]
    fn bounds_against_brute_force_on_random_instances() {
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = |below: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % below
        };
        for _ in 0..400 {
            let members = 2 + next(9) as usize;
            let factors = 2 + next(7);
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
            let all: Vec<u32> = (0..members as u32).collect();
            let (play, matchable, indep) = brute(&inst);
            assert_eq!(matching_number(&inst, &all), matchable);
            assert_eq!(lorea_rank(&with_ground(&inst), &all), indep, "{lists:?}");
            assert!(play <= indep && indep <= matchable);
            assert!(play <= gf2_rank(&inst, &all));
        }
    }

    /// The GF(2) rank against its definition: the span of r independent vectors has 2^r elements.
    #[test]
    fn gf2_rank_against_the_size_of_the_span() {
        let mut state = 0xD1B5_4A32_D192_ED03u64;
        let mut next = |below: u64| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state % below
        };
        for _ in 0..2000 {
            let members = 1 + next(12) as usize;
            let factors = 1 + next(40);
            let lists: Vec<Vec<u64>> = (0..members)
                .map(|_| {
                    let k = 1 + next(4) as usize;
                    let mut l: Vec<u64> = (0..k).map(|_| next(factors)).collect();
                    l.sort_unstable();
                    l.dedup();
                    l
                })
                .collect();
            let inst = Instance::new((0..members as u64).rev().collect(), &lists);
            let vector = |m: usize| inst.factors[m].iter().fold(0u64, |v, &f| v | 1 << f);
            let mut span: std::collections::HashSet<u64> = std::collections::HashSet::new();
            for bits in 0u32..(1 << members) {
                span.insert((0..members).filter(|&i| bits >> i & 1 == 1).fold(0, |acc, i| acc ^ vector(i)));
            }
            let all: Vec<u32> = (0..members as u32).collect();
            assert_eq!(1usize << gf2_rank(&inst, &all), span.len(), "{lists:?}");
        }
    }
}
