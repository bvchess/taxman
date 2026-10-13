//! Synthetic instances shaped like Taxman's upper halves, for the adversarial search: what, in a real game,
//! protects descending greedy?
//!
//! Every family keeps one thing from the real game and varies another:
//!
//! * [`Family::Shuffled`]: the real incidence structure of game n, in a uniformly random order.
//! * [`Family::Window`]: the real incidence structure, in a random order obeying the *window law* (members
//!   f·p sharing a factor f come in the order of their cofactor primes, as in the real order), and nothing
//!   else: a topological order of those constraints, chosen at random, or adversarially ([`Rule`]: always the
//!   smallest member allowed, or the three-factor members first or last).
//! * [`Family::Noisy`]: the real incidence structure, ordered by value × (1 + ε·u) with u uniform in [−1, 1]:
//!   the real order with its close calls scrambled.
//! * [`Family::Perturbed`]: *generalized primes* (Beurling): every prime p <= N replaced by p·(1 + ε·u) (and kept
//!   at least 2); the members are the products of these reals in (N/2, N], each listing the products with one
//!   prime deleted.  The multiplicative structure is exact, only the sizes change.
//! * [`Family::Cramer`]: generalized primes drawn at random with the prime density (each integer k >= 2 is a
//!   prime with probability 1/ln k, at the real value k + u/2 to keep products distinct).
//!
//! Members are in greedy order (largest first); labels are the values scaled by 1000 (for the generalized
//! families) or the real numbers (for the others).

use crate::instance::Instance;
use crate::primes::smallest_prime_factors;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Family {
    Real,
    Shuffled,
    Window(Rule),
    Noisy,
    Perturbed,
    Cramer,
}

impl Family {
    pub fn parse(s: &str) -> Family {
        match s {
            "real" => Family::Real,
            "shuffled" => Family::Shuffled,
            "window" => Family::Window(Rule::Random),
            "window-low" => Family::Window(Rule::Lowest),
            "window-hyperfirst" => Family::Window(Rule::HyperFirst),
            "window-hyperlast" => Family::Window(Rule::HyperLast),
            "antiwindow" => Family::Window(Rule::Anti),
            "rankedwindow" => Family::Window(Rule::Ranked),
            "noisy" => Family::Noisy,
            "perturbed" => Family::Perturbed,
            "cramer" => Family::Cramer,
            _ => panic!("unknown family {s}"),
        }
    }
}

/// A small deterministic generator (xorshift64*).
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn below(&mut self, k: u64) -> u64 {
        self.next_u64() % k
    }
}

/// An instance of `family` with size parameter `n`, noise `eps` and `seed`.
pub fn generate(family: Family, n: u64, eps: f64, seed: u64) -> Instance {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ seed.wrapping_mul(0xD1B5_4A32_D192_ED03) ^ n);
    rng.next_u64();
    match family {
        Family::Real | Family::Shuffled | Family::Window(_) | Family::Noisy => {
            let spf = smallest_prime_factors(n as usize);
            let real = Instance::upper_half(n, &spf);
            let order = match family {
                Family::Real => (0..real.len() as u32).collect(),
                Family::Shuffled => {
                    let mut o: Vec<u32> = (0..real.len() as u32).collect();
                    for i in (1..o.len()).rev() {
                        o.swap(i, rng.below(i as u64 + 1) as usize);
                    }
                    o
                }
                Family::Window(rule) => window_order(&real, &mut rng, rule, eps),
                Family::Noisy => {
                    let mut keyed: Vec<(f64, u32)> = (0..real.len() as u32)
                        .map(|m| (real.value[m as usize] as f64 * (1.0 + eps * (2.0 * rng.unit() - 1.0)), m))
                        .collect();
                    keyed.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                    keyed.into_iter().map(|(_, m)| m).collect()
                }
                _ => unreachable!(),
            };
            reorder(&real, &order)
        }
        Family::Perturbed => {
            let spf = smallest_prime_factors(n as usize);
            let primes: Vec<f64> = (2..=n)
                .filter(|&k| spf[k as usize] as u64 == k)
                .map(|p| (p as f64 * (1.0 + eps * (2.0 * rng.unit() - 1.0))).max(2.0))
                .collect();
            generalized(&primes, n as f64)
        }
        Family::Cramer => {
            let mut primes: Vec<f64> = Vec::new();
            for k in 2..=n {
                if k == 2 || rng.unit() < 1.0 / (k as f64).ln() {
                    primes.push(k as f64 + 0.5 * rng.unit());
                }
            }
            generalized(&primes, n as f64)
        }
    }
}

/// `inst` with its members in the given order (a permutation of its indices).
fn reorder(inst: &Instance, order: &[u32]) -> Instance {
    let value = order.iter().map(|&m| inst.value[m as usize]).collect();
    let factors = order.iter().map(|&m| inst.factors[m as usize].clone()).collect();
    Instance::from_ids(value, factors, inst.factor_value.clone())
}

/// How a window-law order chooses among the members whose predecessors are all placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// Uniformly at random.
    Random,
    /// The smallest: the order least like the real one that the window law allows.
    Lowest,
    /// Members with three or more factors (the hyperedges, where exchange fails) first, at random.
    HyperFirst,
    /// Members with three or more factors last, at random.
    HyperLast,
    /// At random, but obeying the reversed window law (members sharing a factor in increasing order of their
    /// cofactors): a control, as consistent as the window law but backwards.
    Anti,
    /// At random, obeying the window law for another ranking of the primes (cofactors compared by rank): a
    /// random permutation (eps = 0), or the primes by value × (1 + eps·u) (local disorder).
    Ranked,
}

/// An order of the upper half obeying the window law: for every factor f, the members f·p listing it come in
/// decreasing order of p.  (A topological sort of those constraints, choosing by `rule`.)
fn window_order(inst: &Instance, rng: &mut Rng, rule: Rule, eps: f64) -> Vec<u32> {
    if !matches!(rule, Rule::Random | Rule::Anti | Rule::Ranked) {
        return window_order_by(inst, rng, rule);
    }
    // For Ranked: another ranking of the primes, by which the cofactors are compared instead.
    let mut rank: Vec<u64> = (0..=inst.value.iter().copied().max().unwrap_or(1)).collect();
    if rule == Rule::Ranked && eps == 0.0 {
        for i in (1..rank.len()).rev() {
            rank.swap(i, rng.below(i as u64 + 1) as usize);
        }
    } else if rule == Rule::Ranked {
        let keys: Vec<f64> = (0..rank.len()).map(|p| p as f64 * (1.0 + eps * (2.0 * rng.unit() - 1.0))).collect();
        let mut by: Vec<usize> = (0..rank.len()).collect();
        by.sort_by(|&a, &b| keys[a].partial_cmp(&keys[b]).unwrap());
        by.iter().enumerate().for_each(|(r, &p)| rank[p] = r as u64);
    }
    let m = inst.len();
    let mut succ: Vec<Vec<u32>> = vec![Vec::new(); m];
    let mut indegree = vec![0u32; m];
    for (f, list) in inst.listers.iter().enumerate() {
        // Members listing f, by value descending: that is cofactor-prime descending (all are f·p).
        let mut ms = list.clone();
        ms.sort_by_key(|&x| std::cmp::Reverse(inst.value[x as usize]));
        if rule == Rule::Anti {
            ms.reverse();
        }
        if rule == Rule::Ranked {
            let cofactor = |x: u32| inst.value[x as usize] / inst.factor_value[f];
            ms.sort_by_key(|&x| std::cmp::Reverse(rank[cofactor(x) as usize]));
        }
        for w in ms.windows(2) {
            succ[w[0] as usize].push(w[1]);
            indegree[w[1] as usize] += 1;
        }
    }
    let mut ready: Vec<u32> = (0..m as u32).filter(|&x| indegree[x as usize] == 0).collect();
    let mut order = Vec::with_capacity(m);
    while !ready.is_empty() {
        let i = rng.below(ready.len() as u64) as usize;
        let x = ready.swap_remove(i);
        order.push(x);
        for &y in &succ[x as usize] {
            indegree[y as usize] -= 1;
            if indegree[y as usize] == 0 {
                ready.push(y);
            }
        }
    }
    assert_eq!(order.len(), m, "the window law is acyclic");
    order
}

fn window_order_by(inst: &Instance, rng: &mut Rng, rule: Rule) -> Vec<u32> {
    use std::cmp::Reverse;
    use std::collections::BinaryHeap;
    let m = inst.len();
    let mut succ: Vec<Vec<u32>> = vec![Vec::new(); m];
    let mut indegree = vec![0u32; m];
    for list in &inst.listers {
        let mut ms = list.clone();
        ms.sort_by_key(|&x| Reverse(inst.value[x as usize]));
        for w in ms.windows(2) {
            succ[w[0] as usize].push(w[1]);
            indegree[w[1] as usize] += 1;
        }
    }
    // Priority: smaller key first; ties broken at random.
    let key = |x: u32, rng: &mut Rng| -> (u64, u64) {
        let hyper = inst.factors[x as usize].len() >= 3;
        match rule {
            Rule::Lowest => (inst.value[x as usize], 0),
            Rule::HyperFirst => (u64::from(!hyper), rng.next_u64()),
            Rule::HyperLast => (u64::from(hyper), rng.next_u64()),
            Rule::Random | Rule::Anti | Rule::Ranked => unreachable!(),
        }
    };
    let mut ready: BinaryHeap<Reverse<((u64, u64), u32)>> = BinaryHeap::new();
    for x in (0..m as u32).filter(|&x| indegree[x as usize] == 0) {
        ready.push(Reverse((key(x, rng), x)));
    }
    let mut order = Vec::with_capacity(m);
    while let Some(Reverse((_, x))) = ready.pop() {
        order.push(x);
        for &y in &succ[x as usize] {
            indegree[y as usize] -= 1;
            if indegree[y as usize] == 0 {
                ready.push(Reverse((key(y, rng), y)));
            }
        }
    }
    assert_eq!(order.len(), m, "the window law is acyclic");
    order
}

/// The upper-half instance of the generalized integers built on `primes` (reals >= 2, ascending): members are
/// the products in (N/2, N], largest first, each listing the products with one prime deleted.
fn generalized(primes: &[f64], n: f64) -> Instance {
    let mut primes = primes.to_vec();
    primes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // Every product <= n as (value, exponent vector), by depth-first search over nondecreasing prime indices.
    let mut products: Vec<(f64, Vec<(u32, u8)>)> = Vec::new();
    type Vector = Vec<(u32, u8)>;
    let mut stack: Vec<(f64, usize, Vector)> = vec![(1.0, 0, Vec::new())];
    while let Some((value, from, vector)) = stack.pop() {
        if value > n / 2.0 {
            products.push((value, vector.clone()));
        }
        for (i, &p) in primes.iter().enumerate().skip(from) {
            let next = value * p;
            if next > n {
                break;
            }
            let mut v = vector.clone();
            match v.last_mut() {
                Some((j, e)) if *j == i as u32 => *e += 1,
                _ => v.push((i as u32, 1)),
            }
            stack.push((next, i, v));
        }
    }
    products.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let mut ids: HashMap<Vec<(u32, u8)>, u32> = HashMap::new();
    let mut factor_value: Vec<u64> = Vec::new();
    let mut factors = Vec::with_capacity(products.len());
    for (value, vector) in &products {
        let mut list = Vec::new();
        for k in 0..vector.len() {
            let mut f = vector.clone();
            f[k].1 -= 1;
            if f[k].1 == 0 {
                f.remove(k);
            }
            let fv = value / primes[vector[k].0 as usize];
            let id = *ids.entry(f).or_insert_with(|| {
                factor_value.push((fv * 1000.0).round() as u64);
                (factor_value.len() - 1) as u32
            });
            list.push(id);
        }
        list.sort_unstable();
        factors.push(list);
    }
    let value = products.iter().map(|(v, _)| (v * 1000.0).round() as u64).collect();
    Instance::from_ids(value, factors, factor_value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primes::smallest_prime_factors;

    #[test]
    fn generalized_primes_with_no_noise_give_the_real_instance() {
        let n = 500u64;
        let spf = smallest_prime_factors(n as usize);
        let real = Instance::upper_half(n, &spf);
        let fake = generate(Family::Perturbed, n, 0.0, 7);
        assert_eq!(fake.len(), real.len());
        assert_eq!(fake.value.iter().map(|v| v / 1000).collect::<Vec<_>>(), real.value);
        for m in 0..real.len() {
            let mut a: Vec<u64> = real.factors[m].iter().map(|&f| real.factor_value[f as usize]).collect();
            let mut b: Vec<u64> = fake.factors[m].iter().map(|&f| fake.factor_value[f as usize] / 1000).collect();
            a.sort_unstable();
            b.sort_unstable();
            assert_eq!(a, b);
        }
    }

    #[test]
    fn window_orders_obey_the_window_law() {
        let inst = generate(Family::Window(Rule::Lowest), 800, 0.0, 3);
        let mut pos = vec![0usize; 801];
        for (i, &v) in inst.value.iter().enumerate() {
            pos[v as usize] = i;
        }
        for list in &inst.listers {
            let mut vals: Vec<u64> = list.iter().map(|&m| inst.value[m as usize]).collect();
            vals.sort_unstable_by(|a, b| b.cmp(a));
            assert!(vals.windows(2).all(|w| pos[w[0] as usize] < pos[w[1] as usize]));
        }
    }
}
