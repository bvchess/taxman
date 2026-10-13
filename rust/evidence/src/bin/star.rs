//! The threshold-by-threshold (★) campaign: checks conjecture (★) on the upper-half instance of every game in a
//! range at every rejection threshold, recording which argument settles each, and appends one line per game to
//! a resumable JSONL file.
//!
//!     star --from A --to B [--step S] [--list n,n,...] [--threads T] [--seconds S] [--budget NODES]
//!          [--engine plus1|scratch] --out FILE
//!
//! Games already in the output file are skipped, so a stopped run is resumed by running it again.  `--seconds`
//! stops starting new games after that long (games in flight finish).  `--budget` caps the nodes of each exact
//! search; a threshold that exhausts it is recorded as undecided.

use std::path::PathBuf;
use std::time::Instant;
use taxman_evidence::instance::Instance;
use taxman_evidence::jsonl::{campaign, parse_items, Obj};
use taxman_evidence::primes::smallest_prime_factors;
use taxman_evidence::star::{check, Engine, Method, Report};

fn line(n: u64, report: &Report, secs: f64) -> String {
    let mut by_method = Obj::new();
    let mut comps = Obj::new();
    for m in Method::ALL {
        by_method = by_method.num(m.name(), report.count(m));
        comps = comps.num(m.name(), report.components_by_method[m as usize]);
    }
    let hard: Vec<Obj> = report
        .thresholds
        .iter()
        .filter(|t| t.method >= Method::Exact)
        .map(|t| {
            let mut o = Obj::new()
                .num("x", t.x)
                .num("p", t.p_size)
                .num("g", t.g)
                .num("matching_lb", t.matching_lb)
                .num("lorea_lb", t.lorea_lb.unwrap_or(0))
                .num("rank_lb", t.rank_lb.unwrap_or(0))
                .str("method", t.method.name())
                .num("nodes", t.nodes)
                .num("conditional", u8::from(t.conditional));
            if let Some((p, greedy, found)) = &t.witness {
                o = o.nums("P", p).nums("greedy", greedy).nums("found", found);
            }
            o
        })
        .collect();
    let largest_p = report.thresholds.iter().map(|t| t.p_size).max().unwrap_or(0);
    let max_g = report.thresholds.iter().map(|t| t.g).max().unwrap_or(0);
    Obj::new()
        .num("n", n)
        .num("members", report.members)
        .num("kept", report.kept.len())
        .num("kept_sum", report.kept.iter().sum::<u64>())
        .num("residual", report.residual)
        .num("residual_components", report.residual_components)
        .num("largest_residual_component", report.largest_residual_component)
        .num("rejections", report.rejections)
        .num("rejecting_components", report.rejecting_components)
        .num("largest_p", largest_p)
        .num("max_g", max_g)
        .raw("thresholds_by_method", &by_method.finish())
        .raw("components_by_method", &comps.finish())
        .objs("hard", hard)
        .float("secs", secs)
        .finish()
}

fn main() {
    let (games, flags) = parse_items(std::env::args().skip(1));
    let (mut threads, mut seconds, mut budget, mut engine, mut out) =
        (1usize, f64::INFINITY, 2_000_000u64, Engine::Plus1, PathBuf::new());
    for (flag, value) in flags {
        match flag.as_str() {
            "--threads" => threads = value.parse().unwrap(),
            "--seconds" => seconds = value.parse().unwrap(),
            "--budget" => budget = value.parse().unwrap(),
            "--engine" => engine = if value == "scratch" { Engine::Scratch } else { Engine::Plus1 },
            "--out" => out = value.into(),
            _ => panic!("unknown flag {flag}"),
        }
    }
    assert!(!out.as_os_str().is_empty(), "--out is required");
    let spf = smallest_prime_factors(games.iter().copied().max().unwrap_or(2) as usize);
    campaign("star", &games, threads, seconds, &out, |n| {
        let started = Instant::now();
        let report = check(&Instance::upper_half(n, &spf), budget, engine);
        let bad = report.count(Method::Counterexample) + report.count(Method::Undecided);
        let note = (bad > 0).then(|| {
            format!(
                "undecided={} counterexample={}",
                report.count(Method::Undecided),
                report.count(Method::Counterexample)
            )
        });
        (Some(line(n, &report, started.elapsed().as_secs_f64())), note)
    });
}
