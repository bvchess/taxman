//! A check that the parity search finds what it proves absent: `paritycheck A B [STEP]` runs, for each game in
//! the range that needs a search, the model-free search with the parity bound against the target V(n) − 1, one
//! below the recorded optimum (optimal.json, then `../results/strict.jsonl`).  It must return a playable set worth
//! at least V(n), which replays by the rules; a bound or a pruning rule that cut an optimal game would make it
//! fail.  One JSON line per game.

use std::path::PathBuf;
use std::time::Instant;
use taxman_prover::game::{relation, Board, Prover, Verdict};
use taxman_prover::jsonl::recorded_games;
use taxman_prover::moves::play_order;
use taxman_prover::primes::smallest_prime_factors;

fn main() {
    let args: Vec<u32> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let (from, to, step) = (args[0], args[1], args.get(2).copied().unwrap_or(1));
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let known =
        recorded_games(&[&root.join("../../src/main/resources/optimal.json"), &root.join("../results/strict.jsonl")]);
    let spf = smallest_prime_factors(to as usize);
    for n in (from..=to).step_by(step as usize) {
        let Some((score, _)) = known.get(&u64::from(n)) else { continue };
        let prev = known.get(&u64::from(n - 1)).map_or(0, |(s, _)| *s);
        if relation(u64::from(n), *score, prev, &spf).name() != "none" {
            continue;
        }
        let board = Board::new(n, &spf);
        let t0 = Instant::now();
        let mut prover = Prover::new(&board, score - 1, u64::MAX).with_parity();
        let verdict = prover.run();
        let Verdict::Beaten(set) = verdict else { panic!("n={n}: no set worth {score} found: {verdict:?}") };
        let order = play_order(&board, &set).unwrap_or_else(|| panic!("n={n}: the set found is not playable"));
        let sum = board.replay(&order).unwrap_or_else(|e| panic!("n={n}: the set found does not replay: {e}"));
        assert!(sum >= *score, "n={n}: the set found is worth {sum} < {score}");
        println!(
            "{{\"n\":{n},\"score\":{score},\"found\":{sum},\"nodes\":{},\"secs\":{:.3}}}",
            prover.nodes,
            t0.elapsed().as_secs_f64()
        );
    }
}
