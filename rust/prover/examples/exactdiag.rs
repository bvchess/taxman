//! Diagnostics of the model-free search: `exactdiag N TARGET [--budget NODES] [--warmup NODES] [--show K]
//! [--parity 1] [--warm 0] [--iterations I]` runs it and prints its counters.  With a warm-up a first search of
//! that many nodes collects nogoods, the Lagrangian bound is fitted to them (I subgradient steps, default 200), and
//! the search is run again with both bounds.  `--parity 1` turns on the parity bound (`Prover::with_parity`), `--warm 0`
//! solves it from scratch at every node (`Prover::parity_cold`).

use std::time::Instant;
use taxman_prover::game::{Board, Prover};
use taxman_prover::primes::smallest_prime_factors;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (n, target): (u64, u64) = (args[0].parse().unwrap(), args[1].parse().unwrap());
    let (mut budget, mut warmup, mut show, mut parity, mut iterations) = (u64::MAX, 0u64, 0usize, false, 200usize);
    let mut warm = true;
    for pair in args[2..].chunks(2) {
        let value = &pair[1];
        match pair[0].as_str() {
            "--budget" => budget = value.parse().unwrap(),
            "--warmup" => warmup = value.parse().unwrap(),
            "--show" => show = value.parse().unwrap(),
            "--parity" => parity = value == "1",
            "--warm" => warm = value == "1",
            "--iterations" => iterations = value.parse().unwrap(),
            flag => panic!("unknown flag {flag}"),
        }
    }
    let spf = smallest_prime_factors(n as usize);
    let board = Board::new(n as u32, &spf);
    let started = Instant::now();
    let fresh = || {
        let p = Prover::new(&board, target, budget);
        if parity {
            let p = p.with_parity();
            if warm {
                p
            } else {
                p.parity_cold()
            }
        } else {
            p
        }
    };
    let mut prover = fresh();
    let root = prover.root_bound();
    if warmup > 0 {
        let mut first = Prover::new(&board, target, warmup);
        first.run();
        prover.nogoods = first.nogoods;
        let lag = prover.enable_lagrangian(iterations);
        println!(
            "warm-up: {} nogoods from {warmup} nodes; Lagrangian root bound {lag:?} ({:.1}s)",
            prover.nogoods.len(),
            started.elapsed().as_secs_f64()
        );
    }
    let verdict = prover.run();
    let [new, known, forced, children, cut, lag_cut, parity_cut, uncertified] = prover.stats;
    println!(
        "n={n} target={target} root={root} gap={} verdict={verdict:?} nodes={} secs={:.2}",
        root as i64 - target as i64,
        prover.nodes,
        started.elapsed().as_secs_f64()
    );
    let expanded = new + known;
    println!(
        "  expanded {expanded} (new conflicts {new}, known {known}), mean branch {:.2}, forced picks {forced} \
         ({:.2}/node), cut by bound {cut}, by the Lagrangian {lag_cut}, by the parity bound {parity_cut}",
        children as f64 / expanded.max(1) as f64,
        forced as f64 / prover.nodes as f64
    );
    if let Some(s) = prover.parity_stats() {
        println!(
            "  parity: {} solves, {:.1} augmentations and {:.0} arcs a solve, {uncertified} not certified, {} negative arcs",
            s.solves,
            s.augmentations as f64 / s.solves.max(1) as f64,
            s.arcs as f64 / s.solves.max(1) as f64,
            s.negative
        );
        println!(
            "  warm starts: {} ({:.2} improving cycles each), {} fell back {:?}; with foreign forced {:?}",
            s.warm,
            s.cycles as f64 / s.warm.max(1) as f64,
            s.fallbacks,
            s.fallback_reasons,
            s.foreign
        );
    }
    let mut sizes = std::collections::BTreeMap::new();
    for k in &prover.nogoods {
        *sizes.entry(k.len()).or_insert(0) += 1;
    }
    println!("  conflict sizes: {sizes:?}");
    for k in prover.nogoods.iter().take(show) {
        println!("    {k:?}");
    }
}
