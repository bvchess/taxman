//! Prints the jam P at a rejection threshold: `inspect N X` lists P's members with their factorizations, greedy's
//! decisions, the bounds, and how P splits into the components just above x and the new members.

use taxman_evidence::bounds::{lorea_rank, matching_number, with_ground};
use taxman_evidence::instance::Instance;
use taxman_evidence::peel::{component_of, components, greedy, Peeler};
use taxman_evidence::primes::{distinct_primes, smallest_prime_factors};

fn factorize(mut c: u64, spf: &[u32]) -> String {
    let mut parts = Vec::new();
    for p in distinct_primes(c, spf) {
        let mut e = 0;
        while c.is_multiple_of(p) {
            c /= p;
            e += 1;
        }
        parts.push(if e == 1 { format!("{p}") } else { format!("{p}^{e}") });
    }
    parts.join("·")
}

fn main() {
    let args: Vec<u64> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let (n, x) = (args[0], args[1]);
    let verbose = args.get(2).copied().unwrap_or(1) == 1;
    let spf = smallest_prime_factors(n as usize);
    let inst = Instance::upper_half(n, &spf);
    let mut peeler = Peeler::new();
    let mut alive = vec![true; inst.len()];
    peeler.residual(&inst, &mut alive);
    for q in components(&inst, &alive) {
        let local = inst.sub(&q);
        let Some(xi) = local.value.iter().position(|&v| v == x) else { continue };
        let kept = greedy(&local, &mut peeler);
        let mut above = vec![false; local.len()];
        above[..xi].iter_mut().for_each(|a| *a = true);
        let mut down_to = above.clone();
        down_to[xi] = true;
        peeler.residual(&local, &mut above);
        peeler.residual(&local, &mut down_to);
        let p = component_of(&local, &down_to, xi as u32);
        let sub = local.sub(&p);
        let all: Vec<u32> = (0..sub.len() as u32).collect();
        println!(
            "n={n} x={x}: R-component {} members, P {} members, greedy excludes {}, nu {}, lorea rank {}",
            q.len(),
            p.len(),
            p.iter().filter(|&&m| !kept[m as usize]).count(),
            matching_number(&sub, &all),
            lorea_rank(&with_ground(&sub), &all)
        );
        let mut in_p = vec![false; local.len()];
        p.iter().for_each(|&m| in_p[m as usize] = true);
        let mut covered = vec![false; local.len()];
        for c in components(&local, &above).into_iter().filter(|c| in_p[c[0] as usize]) {
            let g = c.iter().filter(|&&m| !kept[m as usize]).count();
            println!("  old component: {} members, g {}", c.len(), g);
            c.iter().for_each(|&m| covered[m as usize] = true);
        }
        for &m in &p {
            let v = local.value[m as usize];
            let fs: Vec<u64> = local.factors[m as usize].iter().map(|&f| local.factor_value[f as usize]).collect();
            let tag = if !covered[m as usize] { "NEW" } else { "" };
            if verbose || !covered[m as usize] {
                let k = if kept[m as usize] { "keep" } else { "REJECT" };
                println!("  {v:>6} = {:<14} {k:<6} {tag:<3} factors {fs:?}", factorize(v, &spf));
            }
        }
    }
}
