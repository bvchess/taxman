//! The parity bound at the root, split by the sources (the notebook entry beside this file).  It is an example of the
//! prover kept out of the build: copy it to solver/noframe/examples/ to run it.
//!
//! ```text
//! cargo run --release --example paritysplit -- --from A --to B [--threads T] [--lists yes] --out FILE
//! ```
//!
//! For every board n of A..=B it writes one JSON line to FILE (resumable: a board already there is skipped):
//!
//! * `ustar_sum`: the sum of U\*, descending greedy's sources (`peel::u_star`);
//! * `parity`: the parity bound of n (W1, docs/theory/parity-bound.md) with no picks and no bars, the heaviest set
//!   independent in both parity matroids, certified by its weight split; `size` its members, and `sources_ustar`
//!   whether its members above n/2 are exactly U\*;
//! * `parity_ustar`: the parity bound with the members of U\* picked and nothing barred;
//! * `free`, `free_sum`, `free_digest`: the numbers up to n/2 that are not loops of either parity matroid contracted by
//!   U\* (the *GF(2) candidates*), their count, sum and FNV-1a digest (64 bits, over the sorted list as 32-bit
//!   little-endian words), to compare with the frame solver's candidates (`framebound.rs` beside this file);
//! * `diamond`: the violations of the span conjecture (♦) on the sources' instance (`span::span_check`), 0 where it
//!   holds;
//! * `secs`: the time of the line, and `secs_root`, `secs_fixed` and `secs_span` that of its three parts.
//!
//! With `--lists yes` the line also has `free_list`, the GF(2) candidates, and `chosen`, the members up to n/2 of the
//! heaviest set found for `parity`.

use std::time::Instant;
use taxman_prover::game::{Board, Role};
use taxman_prover::instance::Instance;
use taxman_prover::jsonl::{campaign, parse_items, Obj};
use taxman_prover::parity::{Meet, Outcome, Parity};
use taxman_prover::peel::u_star;
use taxman_prover::primes::smallest_prime_factors;
use taxman_prover::span::span_check;

/// FNV-1a over the 32-bit little-endian words of `xs`.
fn digest(xs: &[u32]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for x in xs {
        for b in x.to_le_bytes() {
            h ^= u64::from(b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    }
    h
}

fn main() {
    let (games, flags) = parse_items(std::env::args().skip(1));
    let (mut threads, mut out, mut lists) = (1usize, std::path::PathBuf::new(), false);
    for (flag, value) in flags {
        match flag.as_str() {
            "--threads" => threads = value.parse().unwrap(),
            "--out" => out = value.into(),
            "--lists" => lists = value == "yes",
            _ => panic!("unknown flag {flag}"),
        }
    }
    assert!(!out.as_os_str().is_empty(), "--out is required");
    let spf = smallest_prime_factors(games.iter().copied().max().unwrap_or(2) as usize);
    campaign("paritysplit", &games, threads, f64::INFINITY, &out, |n| {
        let started = Instant::now();
        let board = Board::new(n as u32, &spf);
        let half = n as usize / 2;
        let weights: Vec<i64> = (0..=n as i64).collect();
        let ustar = u_star(n, &spf);
        let ustar_sum: u64 = ustar.iter().map(|&s| u64::from(s)).sum();
        let mut meet = Meet::new(Parity::pair(&board, &weights));

        let mut roles = vec![Role::Free; n as usize + 1];
        roles[0] = Role::Barred;
        roles[1] = Role::Barred;
        let t0 = Instant::now();
        let Outcome::Bound(root) = meet.solve(&roles) else { panic!("n={n}: the root is not certified") };
        let secs_root = t0.elapsed().as_secs_f64();
        let mut upper: Vec<u32> = root.set.iter().copied().filter(|&s| s as usize > half).collect();
        let mut sorted_ustar = ustar.clone();
        upper.sort_unstable();
        sorted_ustar.sort_unstable();

        let mut picked = roles.clone();
        ustar.iter().for_each(|&s| picked[s as usize] = Role::Pick);
        let t1 = Instant::now();
        let Outcome::Bound(fixed) = meet.solve_from(&picked, Some(&root.warm)) else {
            panic!("n={n}: U* is dependent or not certified")
        };
        let secs_fixed = t1.elapsed().as_secs_f64();
        let mut loop_flag = vec![false; n as usize + 1];
        fixed.loops.iter().for_each(|&x| loop_flag[x as usize] = true);
        let free: Vec<u32> = (2..=half as u32).filter(|&x| !loop_flag[x as usize]).collect();

        let t2 = Instant::now();
        let span = span_check(&Instance::upper_half(n, &spf));
        let secs_span = t2.elapsed().as_secs_f64();
        let mut line = Obj::new()
            .num("n", n)
            .num("ustar_sum", ustar_sum)
            .num("parity", root.value)
            .num("size", root.set.len())
            .str("sources_ustar", if upper == sorted_ustar { "yes" } else { "no" })
            .num("parity_ustar", fixed.value)
            .num("free", free.len())
            .num("free_sum", free.iter().map(|&x| u64::from(x)).sum::<u64>())
            .str("free_digest", &format!("{:016x}", digest(&free)))
            .num("diamond", span.violations.len())
            .float("secs_root", secs_root)
            .float("secs_fixed", secs_fixed)
            .float("secs_span", secs_span)
            .float("secs", started.elapsed().as_secs_f64());
        if lists {
            let chosen: Vec<u32> = root.set.iter().copied().filter(|&x| x as usize <= half).collect();
            line = line.nums("free_list", &free).nums("chosen", &chosen);
        }
        (Some(line.finish()), None)
    });
}
