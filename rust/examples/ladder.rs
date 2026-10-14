//! The search "ladder": standalone searches of hard games, each with a time limit, for measuring progress
//! and how the search scales with threads.
//!
//! ```text
//! cargo run --release --example ladder -- [--threads T[,T...] | --scaling] [--seconds S] [--diag] [--no-cap]
//!     [--no-seed] [--seed-opt] [--known FILE] [--tactic NAME=VALUE]... [--node-limit N] [--cores C] N...
//! ```
//!
//! Each game is set up from the frame builder (no golden data) as `taxman solve` sets it up (`taxman::board::Board`,
//! at the set width `with_width!` picks for n) with the default search tactics, seeded with
//! the promotions of the known game for n − 1 that are still candidates (as the `seed_incumbent` tactic
//! does; `--seed-opt`: with the game for n itself, so that the search only has to prove it) and capped at
//! n + opt(n − 1) − inTheBag (`prev_cap`).  The known games are optimal.json's, plus the completed games of a
//! results file given with `--known` (one JSON object per line, as `taxman solve --out` writes), which is how
//! games past the end of optimal.json get their n − 1.  One line per game and thread count: whether the search
//! completed, its time and nodes, the incumbent against the known optimum, when the final incumbent was found,
//! and the engine's activity ([`Counters::summary`]: effective parallelism, idle and lock-wait time, splits,
//! steals, time to full occupancy).  `--diag` collects [`Diagnostics`] (slower) and prints a summary.
//! `--tactic` sets a search tactic with the names and values of `taxman solve --tactic` (`nogoods=off`,
//! `match_refine=0`, ...).  `--node-limit N` stops each search after N nodes (steps), for measuring a prefix of a
//! search too long to finish.
//!
//! Scaling: `--threads` takes a list of thread counts, and `--scaling` is 1, 2, 4, ... up to the available
//! parallelism (which is included).  Each game is searched with each count in turn, and a table per game (and
//! one summed over the games) compares them: wall time, speedup over the first count (times that count, so
//! with 1 first it is the usual speedup), nodes per second per thread, effective parallelism (time in the
//! searcher over wall time × threads), idle and lock-wait time, splits and the units they gave, steals, runs
//! and tiny runs (units with nothing left), and the time until all workers first ran at once.  With
//! `--seed-opt` the search only proves the optimum, so its node count barely depends on the threads and the
//! speedup measures the parallel search alone; seeded from n − 1 it also depends on when each run happens to
//! find the optimum.
//!
//! Many workers on few CPUs: `--cores C` lets at most C of the threads search at once, taking turns
//! (`EngineConfig::cores`), so that `--threads 64 --cores 4` measures in counts (nodes, lookups, splits) what 64
//! workers would do, though not in time, which the turns inflate.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use taxman::board::Board;
use taxman::engine::{Counters, EngineConfig};
use taxman::game::Optimal;
use taxman::search::{cut, search_with, Diagnostics, SearchTactics};
use taxman::tactics::Tactics;

struct Options {
    /// Thread counts to search each game with, in order.
    threads: Vec<usize>,
    seconds: u64,
    diag: bool,
    cap: bool,
    seed: bool,
    /// Seed with the game for n itself (so that the search only has to prove it optimal).
    seed_opt: bool,
    /// A results file with more known games.
    known: Option<PathBuf>,
    tactics: SearchTactics,
    games: Vec<u32>,
    /// At most this many threads search at once (`EngineConfig::cores`).
    cores: Option<usize>,
    node_limit: Option<u64>,
}

fn parse() -> Result<Options, String> {
    let mut o = Options {
        threads: vec![10],
        seconds: 60,
        diag: false,
        cap: true,
        seed: true,
        seed_opt: false,
        known: None,
        tactics: SearchTactics::default(),
        games: Vec::new(),
        cores: None,
        node_limit: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--threads" => {
                let list = value("--threads")?;
                let counts: Result<Vec<usize>, _> = list.split(',').map(str::parse).collect();
                o.threads = counts.map_err(|e| format!("--threads {list}: {e}"))?;
                if o.threads.contains(&0) {
                    return Err("--threads: a thread count must be at least 1".into());
                }
            }
            "--scaling" => {
                let max = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
                o.threads = std::iter::successors(Some(1), |t| Some(t * 2)).take_while(|&t| t < max).collect();
                o.threads.push(max);
            }
            "--seconds" => o.seconds = value("--seconds")?.parse().map_err(|e| format!("--seconds: {e}"))?,
            "--diag" => o.diag = true,
            "--no-cap" => o.cap = false,
            "--no-seed" => o.seed = false,
            "--seed-opt" => o.seed_opt = true,
            "--cores" => o.cores = Some(value("--cores")?.parse().map_err(|e| format!("--cores: {e}"))?),
            "--known" => o.known = Some(value("--known")?.into()),
            "--node-limit" => {
                o.node_limit = Some(value("--node-limit")?.parse().map_err(|e| format!("--node-limit: {e}"))?)
            }
            "--tactic" => {
                // The solver's parser, so that the names and values are the same as for `taxman solve --tactic`.
                let spec = value("--tactic")?;
                let mut tactics = Tactics { search: o.tactics, ..Tactics::default() };
                tactics.set(&spec).map_err(|e| format!("--tactic: {e}"))?;
                if tactics != (Tactics { search: tactics.search, ..Tactics::default() }) {
                    return Err(format!("--tactic {spec}: the ladder only searches, so only search tactics apply"));
                }
                o.tactics = tactics.search;
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
    let optimal = match Optimal::built_in().with_results_files(o.known.as_slice()) {
        Ok(optimal) => optimal,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };
    let mut all = Vec::new();
    for &n in &o.games {
        let rows = taxman::with_width!(n, W => run::<W>(n, &o, &optimal));
        if rows.len() > 1 {
            print_scaling(&format!("n={n}"), &rows);
        }
        all.push(rows);
    }
    if all.len() > 1 && o.threads.len() > 1 {
        let total: Vec<Row> = (0..o.threads.len()).map(|t| Row::sum(all.iter().map(|rows| &rows[t]))).collect();
        print_scaling(&format!("all {} games", all.len()), &total);
    }
}

/// One search of one game with one thread count.
#[derive(Clone, Copy)]
struct Row {
    threads: usize,
    completed: bool,
    time: Duration,
    nodes: u64,
    counters: Counters,
}

impl Row {
    /// The rows of several games (at least one) with the same thread count, as one.
    fn sum<'r>(mut rows: impl Iterator<Item = &'r Row>) -> Row {
        let first = rows.next().expect("a row");
        let mut total = *first;
        for r in rows {
            total.completed &= r.completed;
            total.time += r.time;
            total.nodes += r.nodes;
            total.counters.add(&r.counters);
        }
        total
    }
}

/// A table of the searches of one game (or the sums over games) with each thread count.
fn print_scaling(title: &str, rows: &[Row]) {
    let first = rows[0].threads;
    println!("scaling, {title} (speedup over {first} thread(s), times {first}; * = stopped by the time limit):");
    println!(
        "  {:>7} {:>10} {:>7} {:>11} {:>8} {:>6} {:>6} {:>6} {:>7} {:>8} {:>7} {:>8} {:>7} {:>7}",
        "threads",
        "search s",
        "speedup",
        "nodes",
        "k/s/thr",
        "eff %",
        "idle %",
        "lock %",
        "splits",
        "units",
        "steals",
        "runs",
        "tiny",
        "full s"
    );
    let base = rows[0].time.as_secs_f64() * first as f64;
    for r in rows {
        let secs = r.time.as_secs_f64().max(1e-9);
        let c = &r.counters;
        println!(
            "  {:>7} {:>9.2}{} {:>7.2} {:>11} {:>8.1} {:>6.1} {:>6.1} {:>6.2} {:>7} {:>8} {:>7} {:>8} {:>7} {:>7}",
            r.threads,
            secs,
            if r.completed { ' ' } else { '*' },
            base / secs,
            r.nodes,
            r.nodes as f64 / secs / r.threads as f64 / 1e3,
            100.0 * c.efficiency(),
            100.0 * c.share(c.idle),
            100.0 * c.share(c.lock_wait),
            c.splits,
            c.split_units,
            c.steals,
            c.runs,
            c.tiny_runs,
            c.full_occupancy.map_or("never".into(), |d| format!("{:.3}", d.as_secs_f64())),
        );
    }
}

fn secs(d: Duration) -> f64 {
    d.as_secs_f64()
}

/// Sets game `n` up once and searches it with each of the thread counts; one line per search.
fn run<const W: usize>(n: u32, o: &Options, optimal: &Optimal) -> Vec<Row> {
    let t0 = Instant::now();
    let board = Board::<W>::for_n(n);
    let p = board.prepare(o.tactics).expect("prepare");
    let opt = optimal.score(n).map(|s| s - board.in_the_bag);
    let prev_cap = optimal.score(n - 1).and_then(|s| board.promotion_cap(s));
    let seed_game = if o.seed_opt { optimal.moves(n) } else { optimal.moves(n - 1).filter(|_| o.seed) };
    let seed = seed_game.map(|moves| board.seed(moves));
    let cap = prev_cap.filter(|_| o.cap);
    let prep = t0.elapsed();
    let mut rows = Vec::new();
    for &threads in &o.threads {
        let cfg = EngineConfig {
            threads,
            time_limit: Some(Duration::from_secs(o.seconds)),
            cores: o.cores,
            node_limit: o.node_limit,
            ..EngineConfig::default()
        };
        let diag = o.diag.then(|| Mutex::new(Diagnostics::default()));
        let r = search_with(&p, &cfg, seed.as_ref(), cap, false, diag.as_ref()).expect("search");
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
            "n={n:4} T={threads} {status:6} search {:7.2} s nodes {:11} ({:6.0} k/s) best {} opt {} greedy {} cap {} \
             | {found} | prep {:.2} s max_total {} | {}",
            secs(r.time),
            r.stats.nodes,
            r.stats.nodes as f64 / secs(r.time).max(1e-9) / 1e3,
            r.sum,
            opt.map_or("?".into(), |v| v.to_string()),
            p.greedy.sum(),
            prev_cap.map_or("?".into(), |v| v.to_string()),
            secs(prep),
            p.max_total,
            r.counters.summary(),
        );
        if let Some(diag) = diag {
            print_diagnostics(&diag.into_inner().expect("diagnostics"), r.stats.nodes, r.found.map(|f| f.steps));
        }
        rows.push(Row { threads, completed: r.completed, time: r.time, nodes: r.stats.nodes, counters: r.counters });
    }
    rows
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
    println!(
        "    candidates excluded by the rule of symmetry: {}; Lagrangian bounds of its pairs solved: {}",
        d.symmetry_excluded, d.symmetry_solves
    );
    let q = |x| Diagnostics::quantile(&d.gaps, x).map_or("-".into(), |v| v.to_string());
    let count: u64 = d.gaps.iter().sum();
    println!(
        "    gap of the bound over need at open node checks ({count}): p10 {} p25 {} p50 {} p75 {} p90 {} p99 {}",
        q(0.1),
        q(0.25),
        q(0.5),
        q(0.75),
        q(0.9),
        q(0.99)
    );
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
    let [evaluations, searches, scanned] = d.matching;
    if evaluations > 0 {
        println!(
            "    matching: {:.2} evaluations per step, {:.2} repairs per evaluation, {:.1} vertices per repair",
            evaluations as f64 / total.max(1) as f64,
            searches as f64 / evaluations as f64,
            scanned as f64 / searches.max(1) as f64
        );
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
