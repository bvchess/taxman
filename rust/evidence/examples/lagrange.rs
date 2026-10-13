//! Experiments with Lagrangian strengthenings of the matching bound at the root of game N.
//!
//! `lagrange cap N...`: cap the upper half at W(U*): for λ = k/8, opt(n) <= max over matchings of
//! [(1−λ)·upper + lower] + λ·W(U*), valid because every playable upper set weighs at most W(U*) (certified per
//! game by the span check).  Prints the bound for each λ.
//!
//! `lagrange conflicts N TARGET NODES`: run the model-free search for NODES nodes to collect conflicts (minimal
//! unplayable sets K), then relax them: for μ >= 0, max over matchings of Σ picks (c − Σ_{K∋c} μ_K) +
//! Σ μ_K (|K| − 1) bounds every set leaving out a member of each K; subgradient steps on μ.

use taxman_evidence::game::{Board, MatchingBound, Prover, Role};
use taxman_evidence::instance::Instance;
use taxman_evidence::primes::smallest_prime_factors;
use taxman_evidence::span::span_check;

fn root_roles(n: u32) -> Vec<Role> {
    let mut roles = vec![Role::Free; n as usize + 1];
    roles[0] = Role::Barred;
    roles[1] = Role::Barred;
    roles
}

fn cap(ns: &[u32]) {
    let spf = smallest_prime_factors(*ns.iter().max().unwrap() as usize);
    for &n in ns {
        let board = Board::new(n, &spf);
        let span = span_check(&Instance::upper_half(u64::from(n), &spf));
        assert!(span.violations.is_empty());
        let cap = span.kept_sum as i64;
        let roles = root_roles(n);
        let mut row = format!("n={n} W(U*)={cap}:");
        for k in 0..=8i64 {
            let mut m = MatchingBound::new(&board);
            m.set_scale(8 - k, 8);
            let (v, _) = m.solve(&board, &roles).unwrap();
            row += &format!(" {:.0}", (v as i64 + k * cap) as f64 / 8.0);
        }
        println!("{row}");
    }
}

fn conflicts(n: u32, target: u64, nodes: u64) {
    let spf = smallest_prime_factors(n as usize);
    let board = Board::new(n, &spf);
    let mut prover = Prover::new(&board, target, nodes);
    let root = prover.root_bound();
    prover.run();
    let ks = prover.nogoods.clone();
    println!("n={n} target={target} root bound {root} (gap {}), {} conflicts after {nodes} nodes", root - target, ks.len());
    const SCALE: f64 = 100.0;
    let roles = root_roles(n);
    let mut mu = vec![0.0f64; ks.len()];
    let mut best = f64::INFINITY;
    let mut m = MatchingBound::new(&board);
    for it in 0..200 {
        m.penalty.iter_mut().for_each(|p| *p = 0);
        for (k, &u) in ks.iter().zip(&mu) {
            for &c in k {
                m.penalty[c as usize] += (u * SCALE).round() as i64;
            }
        }
        m.set_scale(SCALE as i64, SCALE as i64);
        let (v, picks) = m.solve(&board, &roles).unwrap();
        let constant: f64 = ks.iter().zip(&mu).map(|(k, &u)| u * (k.len() as f64 - 1.0)).sum();
        let bound = v as f64 / SCALE + constant;
        best = best.min(bound);
        let g: Vec<f64> =
            ks.iter().map(|k| k.iter().filter(|&&c| picks[c as usize]).count() as f64 - (k.len() as f64 - 1.0)).collect();
        let norm: f64 = g.iter().map(|x| x * x).sum();
        if it % 20 == 0 || norm == 0.0 {
            println!("  iteration {it}: bound {bound:.1}, best {best:.1}, violated {}", g.iter().filter(|&&x| x > 0.0).count());
        }
        if norm == 0.0 {
            break;
        }
        let step = 0.5 * (bound - target as f64).max(1.0) / norm;
        for (u, gk) in mu.iter_mut().zip(&g) {
            *u = (*u + step * gk).max(0.0);
        }
    }
    println!("  best Lagrangian bound {best:.1}: gap {:.1} (matching alone {})", best - target as f64, root - target);
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let nums: Vec<u64> = args[1..].iter().map(|a| a.parse().unwrap()).collect();
    match args[0].as_str() {
        "cap" => cap(&nums.iter().map(|&x| x as u32).collect::<Vec<_>>()),
        "conflicts" => conflicts(nums[0] as u32, nums[1], nums[2]),
        _ => panic!("cap or conflicts"),
    }
}
