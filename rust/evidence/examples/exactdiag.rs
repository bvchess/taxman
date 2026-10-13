//! Diagnostics of the model-free search: `exactdiag N TARGET [BUDGET [WARMUP [SHOW]]]` runs it and prints its
//! counters.  With WARMUP > 0 a first search of WARMUP nodes collects nogoods, the Lagrangian bound is fitted
//! to them, and the search is run again with both bounds.

use std::time::Instant;
use taxman_evidence::game::{Board, Prover};
use taxman_evidence::primes::smallest_prime_factors;

fn main() {
    let args: Vec<u64> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let (n, target) = (args[0], args[1]);
    let budget = args.get(2).copied().unwrap_or(u64::MAX);
    let warmup = args.get(3).copied().unwrap_or(0);
    let spf = smallest_prime_factors(n as usize);
    let board = Board::new(n as u32, &spf);
    let started = Instant::now();
    let mut prover = Prover::new(&board, target, budget);
    let root = prover.root_bound();
    if warmup > 0 {
        let mut first = Prover::new(&board, target, warmup);
        first.run();
        prover.nogoods = first.nogoods;
        let lag = prover.enable_lagrangian(200);
        println!(
            "warm-up: {} nogoods from {warmup} nodes; Lagrangian root bound {lag:?} ({:.1}s)",
            prover.nogoods.len(),
            started.elapsed().as_secs_f64()
        );
    }
    let verdict = prover.run();
    let [new, known, forced, children, cut, lag_cut] = prover.stats;
    println!(
        "n={n} target={target} root={root} gap={} verdict={verdict:?} nodes={} secs={:.2}",
        root - target,
        prover.nodes,
        started.elapsed().as_secs_f64()
    );
    let expanded = new + known;
    println!(
        "  expanded {expanded} (new conflicts {new}, known {known}), mean branch {:.2}, forced picks {forced} \
         ({:.2}/node), cut by bound {cut}, by the Lagrangian {lag_cut}",
        children as f64 / expanded.max(1) as f64,
        forced as f64 / prover.nodes as f64
    );
    let mut sizes = std::collections::BTreeMap::new();
    for k in &prover.nogoods {
        *sizes.entry(k.len()).or_insert(0) += 1;
    }
    println!("  conflict sizes: {sizes:?}");
    for k in prover.nogoods.iter().take(args.get(4).copied().unwrap_or(0) as usize) {
        println!("    {k:?}");
    }
}
