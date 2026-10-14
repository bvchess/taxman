//! Experiments with Lagrangian strengthenings of the matching bound at the root of game N.
//!
//! `lagrange cap N...`: cap the upper half at W(U*): for λ = k/8, opt(n) <= max over matchings of
//! [(1−λ)·upper + lower] + λ·W(U*), valid because every playable upper set weighs at most W(U*) (certified per
//! game by the span check).  Prints the bound for each λ.  (Measured: at best 4% of the matching bound's gap.)
//!
//! `lagrange conflicts N TARGET NODES [ITERATIONS]`: run the model-free search for NODES nodes to collect
//! nogoods, then fit the Lagrangian bound over them (`Prover::enable_lagrangian`) and print it.

use taxman_prover::game::{Board, MatchingBound, Prover, Role};
use taxman_prover::instance::Instance;
use taxman_prover::primes::smallest_prime_factors;
use taxman_prover::span::span_check;

fn cap(ns: &[u64]) {
    let spf = smallest_prime_factors(*ns.iter().max().unwrap() as usize);
    for &n in ns {
        let board = Board::new(n as u32, &spf);
        let span = span_check(&Instance::upper_half(n, &spf));
        assert!(span.violations.is_empty());
        let cap = span.kept_sum as i64;
        let mut roles = vec![Role::Free; n as usize + 1];
        roles[0] = Role::Barred;
        roles[1] = Role::Barred;
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

fn conflicts(n: u64, target: u64, nodes: u64, iterations: usize) {
    let spf = smallest_prime_factors(n as usize);
    let board = Board::new(n as u32, &spf);
    let mut first = Prover::new(&board, target, nodes);
    let root = first.root_bound();
    first.run();
    let mut prover = Prover::new(&board, target, u64::MAX);
    prover.nogoods = first.nogoods;
    let lag = prover.enable_lagrangian(iterations).expect("some nogoods");
    println!(
        "n={n} target={target}: matching bound {root} (gap {}); {} nogoods after {nodes} nodes; Lagrangian bound \
         {lag:.1} (gap {:.1})",
        root - target,
        prover.nogoods.len(),
        lag - target as f64
    );
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let nums: Vec<u64> = args[1..].iter().map(|a| a.parse().unwrap()).collect();
    match args[0].as_str() {
        "cap" => cap(&nums),
        "conflicts" => conflicts(nums[0], nums[1], nums[2], nums.get(3).copied().unwrap_or(200) as usize),
        _ => panic!("cap or conflicts"),
    }
}
