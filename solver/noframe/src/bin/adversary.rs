//! The adversarial campaign: synthetic Taxman-shaped instances (`adversary.rs`), each checked for (♦) and,
//! where (♦) fails, for (★) threshold by threshold.  One line per seed, resumable ("n" is the seed):
//!
//!     adversary --family real|shuffled|window[-low|-hyperfirst|-hyperlast]|noisy|perturbed|cramer --size N [--eps E]
//!               --from A --to B [--threads T] [--seconds S] [--budget NODES] --out FILE
//!
//! With `--size 0` the items are sizes instead of seeds (seed 0), and "n" is the size.

use std::path::PathBuf;
use std::time::Instant;
use taxman_prover::adversary::{generate, Family};
use taxman_prover::jsonl::{campaign, parse_items, Obj};
use taxman_prover::span::span_check;
use taxman_prover::star::{check, Engine, Method};

fn main() {
    let (seeds, flags) = parse_items(std::env::args().skip(1));
    let (mut threads, mut seconds, mut budget, mut out) = (1usize, f64::INFINITY, 200_000u64, PathBuf::new());
    let (mut family, mut size, mut eps) = (Family::Real, 1000u64, 0.0f64);
    for (flag, value) in flags {
        match flag.as_str() {
            "--threads" => threads = value.parse().unwrap(),
            "--seconds" => seconds = value.parse().unwrap(),
            "--budget" => budget = value.parse().unwrap(),
            "--family" => family = Family::parse(&value),
            "--size" => size = value.parse().unwrap(),
            "--eps" => eps = value.parse().unwrap(),
            "--out" => out = value.into(),
            _ => panic!("unknown flag {flag}"),
        }
    }
    assert!(!out.as_os_str().is_empty(), "--out is required");
    campaign("adversary", &seeds, threads, seconds, &out, |seed| {
        let started = Instant::now();
        // With --size 0 the items are sizes (seed 0), for sweeping the deterministic families.
        let (n_size, rng_seed) = if size == 0 { (seed, 0) } else { (size, seed) };
        let inst = generate(family, n_size, eps, rng_seed);
        let span = span_check(&inst);
        let mut line = Obj::new()
            .num("n", seed)
            .num("size", n_size)
            .float("eps", eps)
            .num("members", inst.len())
            .num("residual", span.residual)
            .num("rejections", span.rejections)
            .num("violations", span.violations.len());
        let mut note = None;
        if !span.violations.is_empty() {
            let star = check(&inst, budget, Engine::Plus1);
            let mut by_method = Obj::new();
            for m in Method::ALL {
                by_method = by_method.num(m.name(), star.count(m));
            }
            let witnesses: Vec<Obj> = star
                .thresholds
                .iter()
                .filter(|t| t.method == Method::Counterexample)
                .take(3)
                .map(|t| {
                    let (p, greedy, found) = t.witness.as_ref().expect("a counterexample has a witness");
                    Obj::new()
                        .num("x", t.x)
                        .num("p", t.p_size)
                        .num("g", t.g)
                        .nums("P", p)
                        .nums("greedy", greedy)
                        .nums("found", found)
                })
                .collect();
            line = line
                .nums("violation_x", &span.violations[..span.violations.len().min(20)])
                .raw("star", &by_method.finish())
                .objs("witnesses", witnesses);
            let bad = star.count(Method::Counterexample);
            if bad > 0 || star.count(Method::Undecided) > 0 {
                note = Some(format!(
                    "{} violations of (♦); (★) counterexamples {bad}, undecided {}",
                    span.violations.len(),
                    star.count(Method::Undecided)
                ));
            }
        }
        (Some(line.float("secs", started.elapsed().as_secs_f64()).finish()), note)
    });
}
