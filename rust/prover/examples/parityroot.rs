//! The parity bound at the root of each game (`parityroot A B [STEP]`): the heaviest common independent set of the
//! two parity matroids, against the recorded optimum (optimal.json, then `../results/strict.jsonl`), with the
//! matching bound beside it, its cost, and checks: the bound is at least the optimum and the recorded game's moves
//! are independent in both matroids (the bound with every recorded move forced is still at least the optimum).

use std::path::PathBuf;
use std::time::Instant;
use taxman_prover::game::{Board, Prover, Role};
use taxman_prover::jsonl::recorded_games;
use taxman_prover::parity::{Meet, Outcome, Parity};
use taxman_prover::primes::smallest_prime_factors;

fn main() {
    let args: Vec<u32> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let (from, to, step) = (args[0], args[1], args.get(2).copied().unwrap_or(1));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let known =
        recorded_games(&[&root.join("../../src/main/resources/optimal.json"), &root.join("../results/strict.jsonl")]);
    let spf = smallest_prime_factors(to as usize);
    for n in (from..=to).step_by(step as usize) {
        let Some((score, moves)) = known.get(&u64::from(n)) else { continue };
        let board = Board::new(n, &spf);
        let weights: Vec<i64> = (0..=i64::from(n)).collect();
        let pair = Parity::pair(&board, &weights);
        let mut meet = Meet::new(pair);
        let mut roles = vec![Role::Free; n as usize + 1];
        roles[0] = Role::Barred;
        roles[1] = Role::Barred;
        let t0 = Instant::now();
        let Outcome::Bound(bound) = meet.solve(&roles) else { panic!("n={n}: the root is not certified") };
        let secs = t0.elapsed().as_secs_f64();
        let matching = Prover::new(&board, *score, 1).root_bound();
        assert!(bound.value >= *score as i64, "n={n}: the parity bound {} is below {score}", bound.value);
        let mut with_game = roles.clone();
        moves.iter().for_each(|&c| with_game[c as usize] = Role::Pick);
        let Outcome::Bound(forced) = meet.solve(&with_game) else { panic!("n={n}: the recorded game is dependent") };
        let extra = forced.value - *score as i64;
        let forced_out = bound.without.iter().filter(|&&(_, v)| v <= *score as i64).count();
        println!(
            "{{\"n\":{n},\"opt\":{score},\"parity\":{},\"gap\":{},\"matching_gap\":{},\"size\":{},\"moves\":{},\
             \"loops\":{},\"extra\":{extra},\"forced\":{forced_out},\"augmentations\":{},\"arcs\":{},\"ms\":{:.2}}}",
            bound.value,
            bound.value - *score as i64,
            matching - score,
            bound.set.len(),
            moves.len(),
            bound.loops.len(),
            meet.stats.augmentations,
            meet.stats.arcs,
            secs * 1000.0
        );
    }
}
