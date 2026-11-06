//! The subcommands: `solve`, `regress`, `verify-results` and `tables`.
//!
//! The ones that solve games read their flags with [`Args`], build [`SolveOptions`] from them ([`solver_options`],
//! or [`range_options`] for those that solve a range) and call the library's pipeline ([`taxman::solver`]); what
//! they add is output, and the policies of a command-line run:
//!
//! * `solve` stops on SIGINT and SIGTERM with a final checkpoint, locks its checkpoint directory and (when it opens
//!   it, `results::ResultsFile`) its results file, and prints one summary line per game as it is concluded;
//! * `regress` solves in memory and compares every score with the table of optimal moves.
//!
//! The small ones: `verify-results` checks results files without changing them (a path that ends in `.gz` is read
//! through the gzip decoder, as every results file a command is given by path), and `tables` prints the maximal
//! factors and composites of numbers on a board.  Every command that meets a score for a board that the table of
//! optimal moves has checks it against the table built into the binary ([`Optimal::built_in`]): no flag turns that off.

use crate::cli::{
    errln, outln, parse_range, range_options, solver_options, Args, CliError, CliResult, SOLVER_FLAGS, WINDOW_FLAG,
};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use taxman::engine;
use taxman::game::Optimal;
use taxman::numtheory::Tables;
use taxman::results::verify_files;
use taxman::search::{finder, gate_trial, MAX_GATE};
use taxman::solver::{self, GameResult, Method, SolveOptions};

fn failed(message: String) -> CliError {
    CliError::Failed(message)
}

/// Flags that take a value: the solver's and the command's own.
fn flags_with(own: &[&'static str]) -> Vec<&'static str> {
    [&SOLVER_FLAGS[..], own].concat()
}

// ---------------------------------------------------------------------------------------------------------
// solve

/// The flags of a long search, read by [`search_options`]: checkpoints, progress, limits and memory.
const SEARCH_FLAGS: [&str; 6] =
    ["--checkpoint-dir", "--checkpoint-interval", "--progress", "--time-limit", "--node-limit", "--mem-cap"];

/// The switch of a long search, read by [`search_options`]: the workers run at the lowest priority
/// ([`SolveOptions::background`]).
const SEARCH_SWITCHES: [&str; 1] = ["--background"];

/// Reads [`SEARCH_FLAGS`] and [`SEARCH_SWITCHES`] into `opts`.
fn search_options(args: &Args, opts: &mut SolveOptions) -> Result<(), CliError> {
    opts.background = args.has("--background");
    opts.checkpoint_dir = args.value("--checkpoint-dir").map(PathBuf::from);
    if let Some(interval) = args.positive_seconds("--checkpoint-interval")? {
        opts.checkpoint_interval = interval;
    }
    // A progress line a minute unless asked otherwise; 0 turns them off.
    let progress = args.seconds("--progress")?.unwrap_or(Duration::from_secs(60));
    opts.progress = (!progress.is_zero()).then_some(progress);
    opts.time_limit = args.positive_seconds("--time-limit")?;
    opts.node_limit = args.parsed("--node-limit")?;
    if opts.node_limit == Some(0) {
        return Err(CliError::Usage("--node-limit must be at least 1".into()));
    }
    if let Some(cap) = args.parsed::<usize>("--mem-cap")? {
        opts.queue_mem_cap = cap;
    }
    Ok(())
}

/// What a run of long searches holds while it goes: its checkpoint directory locked, since two processes in one would
/// overwrite each other's checkpoints (the file a run appends to is locked when it is opened), and SIGINT and SIGTERM
/// made to stop a search with a final checkpoint and the best found so far, instead of killing it.
fn guard_checkpoints(opts: &SolveOptions) -> Result<Option<engine::DirLock>, CliError> {
    let lock = match &opts.checkpoint_dir {
        Some(dir) => {
            std::fs::create_dir_all(dir).map_err(|e| failed(format!("{}: {e}", dir.display())))?;
            Some(engine::lock_dir(dir).map_err(|e| failed(e.to_string()))?)
        }
        None => None,
    };
    if let Err(e) = engine::stop_on_signals() {
        errln!("warning: SIGINT and SIGTERM will kill the run instead of stopping it: {e}");
    }
    Ok(lock)
}

/// The gates of `--gate-trial`: whole numbers from 0 to [`MAX_GATE`], separated by commas.
fn trial_gates(list: &str) -> Result<Vec<u64>, CliError> {
    let bad = || {
        CliError::Usage(format!("--gate-trial takes gates from 0 to {MAX_GATE} separated by commas, or none: {list}"))
    };
    let gates: Vec<u64> =
        list.split(',').map(|gate| gate.trim().parse().map_err(|_| bad())).collect::<Result<_, _>>()?;
    if gates.is_empty() || gates.iter().any(|&gate| gate > MAX_GATE) {
        return Err(bad());
    }
    Ok(gates)
}

/// The gate trial that `--gate-trial` and `--gate-trial-plan` ask for: none where neither is given or `--gate-trial`
/// says `none` (a search with the one gate it starts with), the trial of the gates `--gate-trial` lists, and the trial
/// of the gates [`gate_trial::GATES`] where only `--gate-trial-plan` is given.
fn run_gate_trial(args: &Args) -> Result<Option<gate_trial::Plan>, CliError> {
    let spec = args.value("--gate-trial-plan");
    let gates = match (args.value("--gate-trial"), spec) {
        (Some("none"), Some(_)) => {
            return Err(CliError::Usage("--gate-trial-plan sets the trial, and --gate-trial none is no trial".into()))
        }
        (Some("none") | None, None) => return Ok(None),
        (Some(gates), _) => trial_gates(gates)?,
        (None, Some(_)) => gate_trial::GATES.to_vec(),
    };
    let plan = gate_trial::Plan::of(gates).with(spec.unwrap_or_default());
    plan.map(Some).map_err(|e| CliError::Usage(format!("--gate-trial-plan: {e}")))
}

/// The gate of `--gate`, a whole number from 0 to [`MAX_GATE`], if it is given: the gap from which conflict branching
/// chooses its conflict in every game of the run.  A run with conflict branching off has no use for one, so the flag is
/// refused there.
fn run_gate(args: &Args, opts: &SolveOptions) -> Result<Option<u64>, CliError> {
    let Some(gate) = args.parsed::<u64>("--gate")? else { return Ok(None) };
    if gate > MAX_GATE {
        return Err(CliError::Usage(format!("--gate takes a gap from 0 to {MAX_GATE}: {gate}")));
    }
    if opts.tactics.search.conflict_branching {
        return Ok(Some(gate));
    }
    Err(CliError::Usage(
        "--gate sets the gap from which conflict branching chooses its conflict, and conflict branching is off \
         (--tactic conflict_branching=off): a gate without conflict branching does nothing"
            .into(),
    ))
}

/// The line that says why the search of `n` stopped before it finished, and how to go on.
fn stopped_line(n: u32, opts: &SolveOptions) -> String {
    let resume =
        if opts.checkpoint_dir.is_some() { "run the same command to resume it" } else { "it was not checkpointed" };
    let why = if engine::stop_signal().is_some() { "was stopped by a signal" } else { "hit a limit" };
    format!("stopped: the search for n={n} {why} before it finished; {resume}")
}

const SOLVE_FLAGS: [&str; 10] = [
    "--out",
    WINDOW_FLAG,
    "--finder-after",
    "--finder-seconds",
    "--profile",
    "--profile-depth",
    "--carried-dir",
    "--gate",
    "--gate-trial",
    "--gate-trial-plan",
];

/// `solve N|A-B [options]`: solves game N or a range, each from the one before (`solver::solve_range_each`).
pub fn solve(args: &[String]) -> CliResult {
    let usage = || CliError::Usage("usage: solve N|A-B [options]".into());
    let flags = [&SOLVE_FLAGS[..], &SEARCH_FLAGS[..]].concat();
    let switches = [&["--force-search", "--show"][..], &SEARCH_SWITCHES[..]].concat();
    let args = Args::parse(args, &flags_with(&flags), &switches)?;
    let [range] = args.positional() else { return Err(usage()) };
    let (lo, hi) = parse_range(range)?;

    let mut opts = range_options(&args)?;
    opts.force_search = args.has("--force-search");
    opts.finder_after = args.parsed("--finder-after")?.unwrap_or(finder::AFTER);
    if let Some(budget) = args.positive_seconds("--finder-seconds")? {
        opts.finder_budget = budget;
    }
    opts.profile = args.value("--profile").map(PathBuf::from);
    if let Some(depth) = args.parsed("--profile-depth")? {
        if opts.profile.is_none() {
            return Err(CliError::Usage(
                "--profile-depth is the depth of the profiles of --profile: give it too".into(),
            ));
        }
        opts.profile_depth = depth;
    }
    opts.carried_dir = args.value("--carried-dir").map(PathBuf::from);
    if opts.carried_dir.is_some() {
        opts.tactics.search.carried_nogoods = true;
    }
    opts.gate = run_gate(&args, &opts)?;
    opts.gate_trial = run_gate_trial(&args)?;
    search_options(&args, &mut opts)?;
    let out = args.value("--out").map(Path::new);
    let show = args.has("--show");
    let _lock = guard_checkpoints(&opts)?;

    let summary = solver::solve_range_each(lo, hi, &opts, out, |result| {
        outln!("{}", result.summary_line());
        if show {
            outln!("  promotions: {:?}", result.promotions);
            outln!("  moves: {:?}", result.moves);
        }
        // Games print as they finish, also when the output is a pipe or a file.
        let _ = std::io::stdout().flush();
    })
    .map_err(failed)?;
    if summary.skipped > 0 {
        let file = out.map_or_else(String::new, |p| format!(" in {}", p.display()));
        outln!("skipped {} games already done{file}", summary.skipped);
    }
    if let Some(n) = summary.stopped_at {
        outln!("{}", stopped_line(n, &opts));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// regress

/// `regress [--from A] [--to N] [--window K]`: solves A..N in memory and checks every score against the table of
/// optimal moves (the first that differs is an error, as in any run).
pub fn regress(args: &[String]) -> CliResult {
    let args = Args::parse(args, &flags_with(&["--to", "--from", WINDOW_FLAG]), &[])?;
    if !args.positional().is_empty() {
        return Err(CliError::Usage("regress takes no positional arguments".into()));
    }
    let from = args.parsed::<u32>("--from")?.unwrap_or(1);
    let to = args.parsed::<u32>("--to")?.unwrap_or(400);
    let opts: SolveOptions = range_options(&args)?;

    let start = Instant::now();
    let mut results: Vec<GameResult> = Vec::new();
    solver::solve_range_each(from, to, &opts, None, |result| {
        if result.n % 100 == 0 {
            errln!("  through n={} after {:.1} s", result.n, start.elapsed().as_secs_f64());
        }
        results.push(result.clone());
    })
    .map_err(failed)?;
    let elapsed = start.elapsed();

    outln!(
        "solved {from}..={to} ({} threads, tactics {}) in {:.1} s",
        opts.threads,
        opts.tactics.description(),
        elapsed.as_secs_f64()
    );
    let counts: Vec<String> = Method::ALL
        .iter()
        .map(|&m| format!("{} {}", m.as_str(), results.iter().filter(|r| r.method == m).count()))
        .collect();
    outln!("by method: {}", counts.join(", "));
    let unchecked = results.iter().filter(|r| r.optimal_match.is_none()).count();
    outln!("{} games match the table, {unchecked} are not in it", results.len() - unchecked);
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// verify-results, tables

fn millis(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

/// `verify-results [--threads K] FILE...`: checks results files without changing them (`results::verify_files`), their
/// scores against the built-in table; a file whose name ends in `.gz`, the committed results `results/optimal.jsonl.gz`
/// among them, is read through the gzip decoder.  The check sets up the search problems of the repeats on K threads,
/// as many as `solve` takes by default (the CPUs) unless `--threads` says.
pub fn verify_results(args: &[String]) -> CliResult {
    let args = Args::parse(args, &["--threads"], &[])?;
    if args.positional().is_empty() {
        return Err(CliError::Usage("usage: verify-results [--threads K] FILE...".into()));
    }
    let paths: Vec<PathBuf> = args.positional().iter().map(PathBuf::from).collect();
    let threads = solver_options(&args)?.threads;
    let start = Instant::now();
    let report = verify_files(&paths, Optimal::built_in(), threads).map_err(CliError::Failed)?;
    outln!("{report}");
    outln!("checked in {:.1} ms", millis(start));
    Ok(())
}

/// `tables N K...`: the maximal factors and composites of each K on the board `1..=N`.
pub fn tables(args: &[String]) -> CliResult {
    let usage = || CliError::Usage("usage: tables N K...".into());
    let (n, ks) = args.split_first().ok_or_else(usage)?;
    if ks.is_empty() {
        return Err(usage());
    }
    let n: u32 = n.parse().map_err(|_| CliError::Usage(format!("N must be a non-negative integer, got {n:?}")))?;
    let tables = Tables::new(n);
    for k in ks {
        let k: u32 = k.parse().map_err(|_| CliError::Usage(format!("K must be a non-negative integer, got {k:?}")))?;
        if k < 1 || k > n {
            return Err(CliError::Usage(format!("K = {k} is not on the board 1..={n}")));
        }
        outln!(
            "{k}: maximal factors {:?}; maximal composites {:?}",
            tables.max_factors[k as usize],
            tables.max_composites[k as usize]
        );
    }
    Ok(())
}
