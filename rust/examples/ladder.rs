//! The search "ladder": standalone searches of hard games, each with a time limit, for measuring progress.
//!
//! ```text
//! cargo run --release --example ladder -- [--threads T] [--seconds S] [--diag] [--no-cap] [--no-seed] [--seed-opt]
//!     [--tactic NAME=on|off]... N...
//! ```
//!
//! Each game is set up from the frame builder (no golden data) with the default search tactics, seeded with
//! the promotions of optimal.json's game for n − 1 that are still candidates (as the `seed_incumbent` tactic
//! does; `--seed-opt`: with the game for n itself, so that the search only has to prove it) and capped at
//! n + opt(n − 1) − inTheBag (`prev_cap`).  One line per game: whether the search
//! completed, its time and nodes, the incumbent against optimal.json, and when the final incumbent was found.
//! `--diag` collects [`Diagnostics`] (slower) and prints a summary.

use std::sync::Mutex;
use std::time::{Duration, Instant};
use taxman::bits::Bits;
use taxman::engine::EngineConfig;
use taxman::frames::build_frames;
use taxman::game::Optimal;
use taxman::hives::Apiary;
use taxman::numtheory::Tables;
use taxman::search::{cut, prepare, search_with, Branching, Diagnostics, Revise, SearchTactics};

struct Options {
    threads: usize,
    seconds: u64,
    diag: bool,
    cap: bool,
    seed: bool,
    /// Seed with the game for n itself (so that the search only has to prove it optimal).
    seed_opt: bool,
    tactics: SearchTactics,
    games: Vec<u32>,
}

fn parse() -> Result<Options, String> {
    let mut o = Options {
        threads: 10,
        seconds: 60,
        diag: false,
        cap: true,
        seed: true,
        seed_opt: false,
        tactics: SearchTactics::default(),
        games: Vec::new(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--threads" => o.threads = value("--threads")?.parse().map_err(|e| format!("--threads: {e}"))?,
            "--seconds" => o.seconds = value("--seconds")?.parse().map_err(|e| format!("--seconds: {e}"))?,
            "--diag" => o.diag = true,
            "--no-cap" => o.cap = false,
            "--no-seed" => o.seed = false,
            "--seed-opt" => o.seed_opt = true,
            "--tactic" => {
                let spec = value("--tactic")?;
                let (name, setting) = spec.split_once('=').ok_or("--tactic NAME=on|off")?;
                let t = &mut o.tactics;
                if name == "match_refine" {
                    t.match_refine = setting.parse().map_err(|e| format!("--tactic {spec}: {e}"))?;
                    continue;
                }
                if name == "branching" {
                    t.branching = match setting {
                        "largest" => Branching::Largest,
                        "conflict-smallest" => Branching::ConflictSmallest,
                        "conflict-largest" => Branching::ConflictLargest,
                        _ => return Err(format!("--tactic {spec}: largest, conflict-smallest or conflict-largest")),
                    };
                    continue;
                }
                let on = match setting {
                    "on" => true,
                    "off" => false,
                    _ => return Err(format!("--tactic {spec}: on or off")),
                };
                match name {
                    "preclusions" => t.preclusions = on,
                    "legacy_bound" => t.legacy_bound = on,
                    "revise_freedoms" => t.revise_freedoms = on,
                    "match_bound" => t.match_bound = on,
                    _ => return Err(format!("unknown tactic {name}")),
                }
            }
            n => o.games.push(n.parse().map_err(|e| format!("{n}: {e}"))?),
        }
    }
    if o.games.is_empty() {
        o.games = vec![760, 800, 840, 880, 900, 960, 992, 1000];
    }
    Ok(o)
}

fn main() {
    let o = match parse() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    let optimal = Optimal::load_default().expect("optimal.json");
    for &n in &o.games {
        match n {
            0..=1023 => run::<16>(n, &o, &optimal),
            1024..=2047 => run::<32>(n, &o, &optimal),
            _ => run::<64>(n, &o, &optimal),
        }
    }
}

fn secs(d: Duration) -> f64 {
    d.as_secs_f64()
}

fn run<const W: usize>(n: u32, o: &Options, optimal: &Optimal) {
    let t0 = Instant::now();
    let tables = Tables::new(n);
    let frames = build_frames::<W>(&tables);
    let columns: Vec<Vec<u32>> = frames.frames.iter().flat_map(|f| [f.factors.to_vec(), f.moves.to_vec()]).collect();
    let p = prepare::<W>(&tables, &columns, &frames.candidates, o.tactics, o.threads).expect("prepare");
    let in_the_bag: u64 = Apiary::<W>::new(&tables, Bits::EMPTY).solution().iter().map(|&x| u64::from(x)).sum();
    let opt = optimal.score(n).map(|s| s - in_the_bag);
    let prev_cap = optimal.score(n - 1).map(|s| u64::from(n) + s - in_the_bag);
    let seed_game = if o.seed_opt { optimal.moves(n) } else { optimal.moves(n - 1).filter(|_| o.seed) };
    let seed: Option<Bits<W>> = seed_game.map(|m| m.iter().copied().collect());
    let cfg = EngineConfig {
        threads: o.threads,
        time_limit: Some(Duration::from_secs(o.seconds)),
        ..EngineConfig::default()
    };
    let diag = o.diag.then(|| Mutex::new(Diagnostics::default()));
    let cap = prev_cap.filter(|_| o.cap);
    let r = search_with(&p, &cfg, seed.as_ref(), cap, false, diag.as_ref()).expect("search");
    let t = &p.timings;
    let revise = match p.revise {
        Revise::Searched { nodes, .. } => format!("searched {nodes} nodes"),
        other => format!("{other:?}"),
    };
    let found = match r.found {
        Some(f) => format!("start {}, found at {:.2} s, {} steps", r.start.sum(), secs(f.time), f.steps),
        None => format!("start incumbent {}", r.start.sum()),
    };
    let status = if r.reached_cap {
        "CAPPED"
    } else if r.completed {
        "DONE"
    } else {
        "OPEN"
    };
    println!(
        "n={n:4} T={} {status:6} search {:7.2} s nodes {:11} ({:6.0} k/s) best {} opt {} greedy {} cap {} | {found} \
         | prep {:.2} s (prec {:.2}, revise {:.2} {revise}) max_total {}",
        o.threads,
        secs(r.time),
        r.stats.nodes,
        r.stats.nodes as f64 / secs(r.time).max(1e-9) / 1e3,
        r.sum,
        opt.map_or("?".into(), |v| v.to_string()),
        p.greedy.sum(),
        prev_cap.map_or("?".into(), |v| v.to_string()),
        secs(t0.elapsed() - r.time),
        secs(t.preclusions),
        secs(t.revise),
        p.max_total,
    );
    if let Some(diag) = diag {
        print_diagnostics(&diag.into_inner().expect("diagnostics"), r.stats.nodes, r.found.map(|f| f.steps));
    }
}

fn print_diagnostics(d: &Diagnostics, nodes: u64, found_steps: Option<u64>) {
    let total: u64 = d.steps.iter().sum();
    let line = |label: &str, counts: &[u64]| {
        let shown: Vec<String> = counts.iter().enumerate().map(|(i, c)| format!("{i}:{c}")).collect();
        println!("    {label}: {}", shown.join(" "));
    };
    line("steps by depth", &d.steps);
    line("children by depth", &d.children);
    let named = |counts: &[u64]| -> String {
        let parts: Vec<String> = cut::NAMES.iter().zip(counts).map(|(name, c)| format!("{name} {c}")).collect();
        parts.join(", ")
    };
    println!("    node loops ended: {}", named(&d.loop_ends));
    println!("    children cut: {}; promotions failed {}", named(&d.child_cuts), d.failed_promotions);
    for (label, counts) in [("bound", &d.gaps), ("blame", &d.blame_gaps), ("match", &d.match_gaps)] {
        let q = |x| Diagnostics::quantile(counts, x).map_or("-".into(), |v| v.to_string());
        let count: u64 = counts.iter().sum();
        println!(
            "    gap {label:5} over need at open node checks ({count}): p10 {} p25 {} p50 {} p75 {} p90 {} p99 {}",
            q(0.1),
            q(0.25),
            q(0.5),
            q(0.75),
            q(0.9),
            q(0.99)
        );
    }
    if d.match_open > 0 {
        println!(
            "    matching bounds that did not prune: {}, heaviest matching feasible at {:.1}%; one level of \
             refinement would prune {:.1}%",
            d.match_open,
            100.0 * d.match_open_feasible as f64 / d.match_open as f64,
            100.0 * d.refine_cuts as f64 / d.match_open as f64
        );
        let quantiles = |counts: &[u64]| -> String {
            let q = |x| Diagnostics::quantile(counts, x).map_or("-".into(), |v| v.to_string());
            format!("p10 {} p25 {} p50 {} p75 {} p90 {} p99 {}", q(0.1), q(0.25), q(0.5), q(0.75), q(0.9), q(0.99))
        };
        println!("    core conflict sizes: {}", quantiles(&d.core_sizes));
        println!("    minimal conflict sizes: {}", quantiles(&d.conflict_sizes));
        println!("    bound drop from one refinement: {}", quantiles(&d.refine_drops));
    }
    if d.refinements > 0 {
        println!(
            "    refinements {}: {:.2} evaluations and {:.3} new conflicts each",
            d.refinements,
            d.refine_evaluations as f64 / d.refinements as f64,
            d.refine_conflicts as f64 / d.refinements as f64
        );
    }
    if let Some(steps) = found_steps {
        println!(
            "    steps {total} (engine nodes {nodes}); after the final incumbent (exact with 1 thread): {:.1}%",
            100.0 * (nodes.saturating_sub(steps)) as f64 / nodes.max(1) as f64
        );
    }
}
