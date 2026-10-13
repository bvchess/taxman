//! The span campaign: checks (♦) — every rejection of descending greedy on the upper-half instance is a GF(2)
//! dependency of the members kept above it — for every game in a range, which certifies (★) and the upper-half
//! conjecture for that game (see `span.rs`).  A game with a violation is also run through the
//! threshold-by-threshold check, whose verdicts are recorded with it.  One line per game, resumable.
//!
//!     span --from A --to B [--step S] [--list n,n,...] [--threads T] [--seconds S] [--budget NODES] --out FILE

use std::path::PathBuf;
use std::time::Instant;
use taxman_evidence::instance::Instance;
use taxman_evidence::jsonl::{campaign, parse_items, Obj};
use taxman_evidence::primes::smallest_prime_factors;
use taxman_evidence::span::span_check;
use taxman_evidence::star::{check, Engine, Method};

fn main() {
    let (games, flags) = parse_items(std::env::args().skip(1));
    let (mut threads, mut seconds, mut budget, mut out) = (1usize, f64::INFINITY, 2_000_000u64, PathBuf::new());
    for (flag, value) in flags {
        match flag.as_str() {
            "--threads" => threads = value.parse().unwrap(),
            "--seconds" => seconds = value.parse().unwrap(),
            "--budget" => budget = value.parse().unwrap(),
            "--out" => out = value.into(),
            _ => panic!("unknown flag {flag}"),
        }
    }
    assert!(!out.as_os_str().is_empty(), "--out is required");
    let spf = smallest_prime_factors(games.iter().copied().max().unwrap_or(2) as usize);
    campaign("span", &games, threads, seconds, &out, |n| {
        let started = Instant::now();
        let inst = Instance::upper_half(n, &spf);
        let r = span_check(&inst);
        let mut line = Obj::new()
            .num("n", n)
            .num("members", r.members)
            .num("residual", r.residual)
            .num("residual_components", r.residual_components)
            .num("largest_residual_component", r.largest_residual_component)
            .num("largest_rank", r.largest_rank)
            .num("kept", r.kept)
            .num("kept_sum", r.kept_sum)
            .num("rejections", r.rejections)
            .num("spanned", r.spanned)
            .num("peels", r.peels)
            .nums("violations", &r.violations);
        let mut note = None;
        if !r.violations.is_empty() {
            let star = check(&inst, budget, Engine::Plus1);
            let mut by_method = Obj::new();
            for m in Method::ALL {
                by_method = by_method.num(m.name(), star.count(m));
            }
            line = line.raw("star_thresholds_by_method", &by_method.finish());
            note = Some(format!(
                "{} violations; star: undecided={} counterexample={}",
                r.violations.len(),
                star.count(Method::Undecided),
                star.count(Method::Counterexample)
            ));
        }
        (line.float("secs", started.elapsed().as_secs_f64()).finish(), note)
    });
}
