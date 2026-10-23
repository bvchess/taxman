//! The search "ladder": standalone searches of hard games, each with a time limit, for measuring progress
//! and how the search scales with threads.
//!
//! ```text
//! cargo run --release --example ladder -- [--threads T[,T...] | --scaling] [--seconds S] [--diag] [--no-cap]
//!     [--no-seed] [--seed-opt] [--known FILE] [--tactic NAME=VALUE]... [--node-limit N] [--cores C]
//!     [--checkpoint-dir DIR [--checkpoint-interval S]] [--profile DIR [--profile-depth D]] N...
//! ```
//!
//! Each game is set up from the frame builder (no golden data) as `taxman solve` sets it up (`taxman::board::Board`,
//! at the set width `with_width!` picks for n) with the default search tactics, seeded with
//! the promotions of the known game for n − 1 that are still candidates (as the `seed_incumbent` tactic
//! does; `--seed-opt`: with the game for n itself, so that the search only has to prove it) and capped at
//! n + opt(n − 1) − in_the_bag (`prev_cap`).  The known games are optimal.json's, plus the completed games of a
//! results file given with `--known` (one JSON object per line, as `taxman solve --out` writes), which is how
//! games past the end of optimal.json get their n − 1.  One line per game and thread count: whether the search
//! completed, its time and nodes, the incumbent against the known optimum, when the final incumbent was found,
//! and the engine's activity ([`Counters::summary`]: effective parallelism, idle and lock-wait time, splits,
//! steals, time to full occupancy), then the CPU seconds and the cycles of the process over the search (they move less
//! with the load on the machine than wall time does; cycles on macOS only), the tactics that differ from the defaults,
//! and what the nogood tables did ([`NogoodStats::summary`]: learned, lookups and the share answered, candidates ruled
//! out, forgettings with the table's mean size before and after, evictions, the largest table).  `--diag` collects
//! [`Diagnostics`] (slower) and prints a summary.
//! `--tactic` sets a search tactic with the names and values of `taxman solve --tactic` (`nogoods=off`,
//! `match_refine=0`, `nogood_keep=top64`, ...).  `--node-limit N` stops each search after N nodes (steps), for
//! measuring a prefix of a search too long to finish.
//!
//! `--checkpoint-dir DIR` checkpoints each search in DIR every `--checkpoint-interval` seconds (60 by default), as
//! `taxman solve` does; the checkpoint of a search that completes is deleted, and none is resumed.  `--profile DIR`
//! keeps the profile of each search (`search::SearchProfile`, as `taxman solve --profile` does) in
//! `DIR/nNNNN-tT.profile.jsonl`, T the thread count, at the depth `--profile-depth` (`search::PROFILE_DEPTH` by
//! default); it is written at each checkpoint and at the end.  With neither `--diag` nor `--profile` a search is the same
//! as with `--profile`, but for the profile, so that the two measure what the profile costs.
//!
//! Scaling: `--threads` takes a list of thread counts, and `--scaling` is 1, 2, 4, ... up to the available
//! parallelism (which is included).  Each game is searched with each count in turn, and a table per game (and
//! one summed over the games) compares them: wall time, speedup over the first count (times that count, so
//! with 1 first it is the usual speedup), nodes (steps) and nodes per second per thread (`k/s/thr`), the
//! workers' busy time per step, shares of the workers' time (`Counters::share`: of their lifetimes summed, which
//! is wall time × threads when their number does not change), splits and the units they gave, steals, runs and
//! tiny runs (units with nothing left), and the time until all workers first ran at once.  With `--seed-opt` the
//! search only proves P*, so its node count barely depends on the threads and the speedup measures the
//! parallel search alone; seeded from n − 1 it also depends on when each run happens to find an optimal set.
//!
//! Where the time of a step goes on many threads: for a search long against its own setup, `k/s/thr` is `eff %` over
//! `µs/step` (times 10), so it falls when the workers do not step all the time (they wait for work or for the lock)
//! or when a step takes longer.  The columns that tell which:
//!
//! * `µs/step`: the workers' busy time per step, in µs.
//! * `µs/tree`: the same without the units' starts, `(busy − rebuild − first_step) / (steps − first_steps)` (the
//!   `Counters` fields): the cost of a step inside a unit's tree, once its state is built and its first node is
//!   solved.
//! * `eff %`, `idle %`: the shares of the time in the searcher and of the time waiting for work.
//! * `lock %`: the share spent waiting for the engine's lock after a run (the one shared deque).
//! * `reb %`: the share spent in the units' rebuilds, from the start of a unit to its first poll, in which the
//!   searcher rebuilds its state.
//! * `1st %`: the share spent in the units' first steps, from a unit's first poll to its second: its first node,
//!   whose GF(2) bound is solved from scratch.
//! * `ramp %`: the share that was idle before all workers first ran at once.
//! * `tail %`: the share that was idle after the last moment a worker finished a unit while all were running.
//!
//! The units' starts (`reb %` and `1st %`) are paid once a unit, and part of `µs/step`: more units on more threads
//! make them dearer a step.  The lock, the ramp and the tail are time not stepping (`ramp %` and `tail %` are parts
//! of `idle %`; a search too short for its threads shows them large).  What is left of the rise of `µs/step` is the
//! step itself (`µs/tree`: caches, memory, nogoods learned by fewer steps a thread).
//!
//! Many workers on few CPUs: `--cores C` lets at most C of the threads search at once, taking turns
//! (`EngineConfig::cores`), so that `--threads 64 --cores 4` measures in counts (nodes, lookups, splits) what 64
//! workers would do, though not in time, which the turns inflate.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
use taxman::board::Board;
use taxman::engine::{Counters, EngineConfig};
use taxman::game::Optimal;
use taxman::search::{
    cut, search_steered, search_with, BranchSettings, Branching, Diagnostics, ProfilePlan, SearchTactics,
    PROFILE_DEPTH, PROFILE_MOST_IDS,
};
use taxman::tactics::Tactics;

#[path = "common/counters.rs"]
mod counters;

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
    /// Where to checkpoint each search, and how often.
    checkpoint: Option<PathBuf>,
    checkpoint_interval: Duration,
    /// Where to keep each search's profile, and its depth.
    profile: Option<PathBuf>,
    profile_depth: u32,
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
        checkpoint: None,
        checkpoint_interval: Duration::from_secs(60),
        profile: None,
        profile_depth: PROFILE_DEPTH,
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
            "--checkpoint-dir" => o.checkpoint = Some(value("--checkpoint-dir")?.into()),
            "--checkpoint-interval" => {
                let seconds: f64 =
                    value("--checkpoint-interval")?.parse().map_err(|e| format!("--checkpoint-interval: {e}"))?;
                o.checkpoint_interval =
                    Duration::try_from_secs_f64(seconds).map_err(|e| format!("--checkpoint-interval: {e}"))?;
            }
            "--profile" => o.profile = Some(value("--profile")?.into()),
            "--profile-depth" => {
                o.profile_depth = value("--profile-depth")?.parse().map_err(|e| format!("--profile-depth: {e}"))?
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
    if o.diag && o.profile.is_some() {
        return Err("--diag and --profile do not go together".into());
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

/// The columns of the scaling table: the heading and the width of each.
const COLUMNS: [(&str, usize); 20] = [
    ("threads", 7),
    ("search s", 10),
    ("speedup", 7),
    ("nodes", 11),
    ("k/s/thr", 8),
    ("µs/step", 8),
    ("µs/tree", 8),
    ("eff %", 6),
    ("idle %", 6),
    ("lock %", 6),
    ("reb %", 6),
    ("1st %", 6),
    ("ramp %", 6),
    ("tail %", 6),
    ("splits", 7),
    ("units", 8),
    ("steals", 7),
    ("runs", 8),
    ("tiny", 7),
    ("full s", 7),
];

/// One line of the scaling table: a cell for each column, aligned to the right.
fn print_cells<S: AsRef<str>>(cells: [S; COLUMNS.len()]) {
    let cells: Vec<String> =
        cells.iter().zip(COLUMNS).map(|(cell, (_, width))| format!("{:>width$}", cell.as_ref())).collect();
    println!("  {}", cells.join(" "));
}

/// `time` per step in µs, or `-` if there are no steps.
fn micros_per_step(time: Duration, steps: u64) -> String {
    if steps == 0 {
        "-".into()
    } else {
        format!("{:.2}", time.as_secs_f64() * 1e6 / steps as f64)
    }
}

/// A table of the searches of one game (or the sums over games) with each thread count.
fn print_scaling(title: &str, rows: &[Row]) {
    let first = rows[0].threads;
    println!("scaling, {title} (speedup over {first} thread(s), times {first}; * = stopped by the time limit):");
    print_cells(COLUMNS.map(|(heading, _)| heading));
    let base = rows[0].time.as_secs_f64() * first as f64;
    for r in rows {
        let secs = r.time.as_secs_f64().max(1e-9);
        let c = &r.counters;
        let share = |time| format!("{:.1}", 100.0 * c.share(time));
        // The step without its unit's start: the rebuild before it and the first node, which are paid once a unit.
        let tree_time = c.busy.saturating_sub(c.rebuild + c.first_step);
        print_cells([
            r.threads.to_string(),
            format!("{secs:>9.2}{}", if r.completed { ' ' } else { '*' }),
            format!("{:.2}", base / secs),
            r.nodes.to_string(),
            format!("{:.1}", r.nodes as f64 / secs / r.threads as f64 / 1e3),
            micros_per_step(c.busy, r.nodes),
            micros_per_step(tree_time, r.nodes.saturating_sub(c.first_steps)),
            format!("{:.1}", 100.0 * c.efficiency()),
            share(c.idle),
            format!("{:.2}", 100.0 * c.share(c.lock_wait)),
            share(c.rebuild),
            share(c.first_step),
            share(c.ramp_idle),
            share(c.tail_idle),
            c.splits.to_string(),
            c.split_units.to_string(),
            c.steals.to_string(),
            c.runs.to_string(),
            c.tiny_runs.to_string(),
            c.full_occupancy.map_or("never".into(), |d| format!("{:.3}", d.as_secs_f64())),
        ]);
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
            checkpoint: o.checkpoint.as_ref().map(|dir| (dir.join(format!("n{n:04}.ckpt")), o.checkpoint_interval)),
            ..EngineConfig::default()
        };
        if let Some(dir) = &o.checkpoint {
            std::fs::create_dir_all(dir).expect("the checkpoint directory");
        }
        let diag = o.diag.then(|| Mutex::new(Diagnostics::default()));
        let profile = o.profile.as_ref().map(|dir| ProfilePlan {
            file: dir.join(format!("n{n:04}-t{threads}.profile.jsonl")),
            depth: o.profile_depth,
            most_ids: PROFILE_MOST_IDS,
        });
        let (cpu_before, cycles_before) = (counters::cpu_seconds(), counters::now());
        let r = if o.diag {
            search_with(&p, &cfg, seed.as_ref(), cap, false, diag.as_ref())
        } else {
            // As `search_with` without diagnostics, but for the profile.
            let branching = Arc::new(Branching::new(BranchSettings::of(&p.tactics)));
            let seeds: Vec<_> = seed.into_iter().collect();
            search_steered(&p, &cfg, &seeds, cap, false, None, &branching, &OnceLock::new(), profile.as_ref())
        }
        .expect("search");
        let (cpu_after, cycles_after) = (counters::cpu_seconds(), counters::now());
        let cpu = cpu_before.zip(cpu_after).map_or("?".into(), |(a, b)| format!("{:.2}", b - a));
        let gigacycles = cycles_before
            .zip(cycles_after)
            .map_or("?".into(), |((_, a), (_, b))| format!("{:.2}", b.saturating_sub(a) as f64 / 1e9));
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
             | {found} | prep {:.2} s max_total {} | cpu {cpu} s {gigacycles} Gcycles | tactics {} | {} | {}",
            secs(r.time),
            r.stats.nodes,
            r.stats.nodes as f64 / secs(r.time).max(1e-9) / 1e3,
            r.sum,
            opt.map_or("?".into(), |v| v.to_string()),
            p.greedy.sum(),
            prev_cap.map_or("?".into(), |v| v.to_string()),
            secs(prep),
            p.max_total,
            Tactics { search: o.tactics, ..Tactics::default() }.description(),
            r.counters.summary(),
            r.nogoods.summary(),
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
    println!("    candidates excluded by the rule of symmetry: {}", d.symmetry_excluded);
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
            "    matching bounds that did not prune: {}, heaviest matching promotable at {:.1}%; one level of \
             refinement would prune {:.1}%",
            d.match_open,
            100.0 * d.match_open_promotable as f64 / d.match_open as f64,
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
