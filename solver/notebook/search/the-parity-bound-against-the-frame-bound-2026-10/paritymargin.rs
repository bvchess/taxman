//! How much the parity bound loses when it must hold a number up to n/2 that is a loop of neither parity matroid
//! contracted by U\* but is no candidate (the notebook entry beside this file).  It is an example of the prover kept
//! out of the build: copy it to solver/noframe/examples/ to run it.
//!
//! ```text
//! cargo run --release --example paritymargin -- --list N,N,... --numbers FILE [--threads T] --out OUT
//! ```
//!
//! FILE has a line `n y1 y2 ...` for each board, the numbers to try (`compare.py` writes them).  For every board n of
//! the list, OUT gets one JSON line: `parity`, the parity bound of n with the picks U\* (equal to the parity bound
//! where the span conjecture (♦) holds, by W3, docs/theory/parity-bound.md), and `margins`, for each number y of the
//! board's line, `[y, loss]`, where `loss` is `parity` less the parity bound with the picks U\* and y (W1).  A loss
//! above 0 says that no heaviest set independent in both parity matroids holds y.

use std::collections::HashMap;
use std::time::Instant;
use taxman_prover::game::{Board, Role};
use taxman_prover::jsonl::{campaign, parse_items, Obj};
use taxman_prover::parity::{Meet, Outcome, Parity};
use taxman_prover::peel::u_star;
use taxman_prover::primes::smallest_prime_factors;

fn main() {
    let (games, flags) = parse_items(std::env::args().skip(1));
    let (mut threads, mut out, mut numbers) = (1usize, std::path::PathBuf::new(), std::path::PathBuf::new());
    for (flag, value) in flags {
        match flag.as_str() {
            "--threads" => threads = value.parse().unwrap(),
            "--out" => out = value.into(),
            "--numbers" => numbers = value.into(),
            _ => panic!("unknown flag {flag}"),
        }
    }
    let text = std::fs::read_to_string(&numbers).expect("read --numbers");
    let tries: HashMap<u64, Vec<u32>> = text
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace().map(|w| w.parse::<u64>().unwrap());
            let n = words.next()?;
            Some((n, words.map(|y| y as u32).collect()))
        })
        .collect();
    let spf = smallest_prime_factors(games.iter().copied().max().unwrap_or(2) as usize);
    campaign("paritymargin", &games, threads, f64::INFINITY, &out, |n| {
        let started = Instant::now();
        let board = Board::new(n as u32, &spf);
        let weights: Vec<i64> = (0..=n as i64).collect();
        let mut meet = Meet::new(Parity::pair(&board, &weights));
        let mut roles = vec![Role::Free; n as usize + 1];
        roles[0] = Role::Barred;
        roles[1] = Role::Barred;
        u_star(n, &spf).iter().for_each(|&s| roles[s as usize] = Role::Pick);
        let Outcome::Bound(fixed) = meet.solve(&roles) else { panic!("n={n}: U* is dependent or not certified") };
        let mut margins = Vec::new();
        for &y in tries.get(&n).map(Vec::as_slice).unwrap_or(&[]) {
            let mut with_y = roles.clone();
            with_y[y as usize] = Role::Pick;
            let value = match meet.solve(&with_y) {
                Outcome::Bound(b) => b.value,
                Outcome::Infeasible => i64::MIN,
                Outcome::Uncertified => panic!("n={n}, y={y}: not certified"),
            };
            margins.push(format!("[{y},{}]", fixed.value - value));
        }
        let line = Obj::new()
            .num("n", n)
            .num("parity", fixed.value)
            .raw("margins", &format!("[{}]", margins.join(",")))
            .float("secs", started.elapsed().as_secs_f64())
            .finish();
        (Some(line), None)
    });
}
