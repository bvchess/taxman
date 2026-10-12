//! The solving subcommands: `solve`, `regress` and `bench`.

use crate::cli::{errln, outln, parse_range, solver_options, Args, CliError, CliResult, SOLVER_FLAGS};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use taxman::engine;
use taxman::json::Json;
use taxman::solver::{self, GameResult, Method, Prev, SolveOptions};

fn failed(message: String) -> CliError {
    CliError::Failed(message)
}

/// Flags that take a value: the solver's and the command's own.
fn flags_with(own: &[&'static str]) -> Vec<&'static str> {
    [&SOLVER_FLAGS[..], own].concat()
}

// ---------------------------------------------------------------------------------------------------------
// solve

const SOLVE_FLAGS: [&str; 7] =
    ["--out", "--checkpoint-dir", "--checkpoint-interval", "--progress", "--time-limit", "--node-limit", "--mem-cap"];

pub fn solve(args: &[String]) -> CliResult {
    let usage = || CliError::Usage("usage: solve N|A-B [options]".into());
    let args = Args::parse(args, &flags_with(&SOLVE_FLAGS), &["--force-search", "--show"])?;
    let [range] = args.positional() else { return Err(usage()) };
    let (lo, hi) = parse_range(range)?;

    let mut opts = solver_options(&args)?;
    opts.force_search = args.has("--force-search");
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
    let out = args.value("--out").map(Path::new);
    let show = args.has("--show");

    // Two processes in one checkpoint directory would overwrite each other's checkpoints.
    let _lock = match &opts.checkpoint_dir {
        Some(dir) => {
            std::fs::create_dir_all(dir).map_err(|e| failed(format!("{}: {e}", dir.display())))?;
            Some(engine::lock_dir(dir).map_err(|e| failed(e.to_string()))?)
        }
        None => None,
    };
    // A signal stops the search with a final checkpoint and the best game so far, instead of killing it.
    if let Err(e) = engine::stop_on_signals() {
        errln!("warning: SIGINT and SIGTERM will kill the run instead of stopping it: {e}");
    }

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
        let resume =
            if opts.checkpoint_dir.is_some() { "run the same command to resume it" } else { "it was not checkpointed" };
        let why = if engine::stop_signal().is_some() { "was stopped by a signal" } else { "hit a limit" };
        outln!("stopped: the search for n={n} {why} before it finished; {resume}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// regress

pub fn regress(args: &[String]) -> CliResult {
    let args = Args::parse(args, &flags_with(&["--to", "--from"]), &[])?;
    if !args.positional().is_empty() {
        return Err(CliError::Usage("regress takes no positional arguments".into()));
    }
    let from = args.parsed::<u32>("--from")?.unwrap_or(1);
    let to = args.parsed::<u32>("--to")?.unwrap_or(400);
    let mut opts: SolveOptions = solver_options(&args)?;
    let Some(optimal) = opts.load_optimal().map_err(failed)? else {
        return Err(CliError::Usage("regress compares with optimal.json, so --optimal none makes no sense".into()));
    };
    // Report every mismatch instead of stopping at the first.
    opts.allow_optimal_mismatch = true;

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
    let counts: Vec<String> = [Method::Trivial, Method::Prime, Method::Reuse, Method::Search]
        .iter()
        .map(|&m| format!("{} {}", m.as_str(), results.iter().filter(|r| r.method == m).count()))
        .collect();
    outln!("by method: {}", counts.join(", "));
    // The Java's own test counts the games it derives without search in 1..200.
    if from <= 1 && to >= 200 {
        let derived = results.iter().filter(|r| r.n <= 200 && r.method.is_derived()).count();
        outln!("games 1..=200 derived without search: {derived}");
    }
    let unchecked = results.iter().filter(|r| r.optimal_match.is_none()).count();
    let mismatches: Vec<&GameResult> = results.iter().filter(|r| r.optimal_match == Some(false)).collect();
    for r in &mismatches {
        let expected = optimal.score(r.n).map_or_else(|| "?".to_owned(), |s| s.to_string());
        outln!(
            "MISMATCH n={}: scored {} but optimal.json says {expected} (method {})",
            r.n,
            r.score,
            r.method.as_str()
        );
    }
    outln!(
        "{} games match optimal.json, {} do not, {unchecked} are not in it",
        results.len() - mismatches.len() - unchecked,
        mismatches.len()
    );
    if mismatches.is_empty() {
        Ok(())
    } else {
        Err(CliError::Failed(format!("{} scores differ from optimal.json", mismatches.len())))
    }
}

// ---------------------------------------------------------------------------------------------------------
// bench

/// The benchmark games and the Java solver's whole-game wall times for them, with one thread and with ten
/// (including about half a second of JVM start).
const BENCH_GAMES: [(u32, f64, f64); 8] = [
    (480, 2.3, 1.5),
    (500, 2.1, 1.4),
    (600, 2.7, 1.7),
    (640, 2.0, 1.6),
    (660, 4.1, 2.1),
    (700, 18.0, 4.8),
    (720, 23.2, 5.6),
    (760, 65.6, 12.4),
];

const BENCH_HISTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/bench/history.jsonl");

pub fn bench(args: &[String]) -> CliResult {
    let args = Args::parse(args, &SOLVER_FLAGS, &[])?;
    let ns: Vec<u32> = if args.positional().is_empty() {
        BENCH_GAMES.iter().map(|g| g.0).collect()
    } else {
        let parse = |text: &String| text.parse().map_err(|_| CliError::Usage(format!("not a board size: {text:?}")));
        args.positional().iter().map(parse).collect::<Result<_, _>>()?
    };
    let mut opts = solver_options(&args)?;
    // Every game on its own: the previous game from optimal.json, and no derivation.
    opts.force_search = true;
    let Some(optimal) = opts.load_optimal().map_err(failed)? else {
        return Err(CliError::Usage(
            "bench takes the previous games from optimal.json, so --optimal none makes no sense".into(),
        ));
    };

    outln!("{} threads, tactics {}", opts.threads, opts.tactics.description());
    let columns = ["n", "nodes", "precl s", "greedy s", "revise s", "search s", "total s", "Java 1t", "Java 10t"];
    let [n, nodes, precl, greedy, revise, search, total, java1, java10] = columns;
    outln!("{n:>5} {nodes:>13} {precl:>9} {greedy:>9} {revise:>9} {search:>9} {total:>9}  | {java1:>8} {java10:>8}");
    let mut results = Vec::new();
    for &n in &ns {
        let prev = n.checked_sub(1).and_then(|before| Prev::from_optimal(&optimal, before));
        let result = solver::solve_game_with(n, prev.as_ref(), &opts, Some(&optimal)).map_err(failed)?;
        let java = BENCH_GAMES.iter().find(|g| g.0 == n);
        let reference = |pick: fn(&(u32, f64, f64)) -> f64| {
            java.map_or_else(|| format!("{:>8}", "-"), |g| format!("{:>8.1}", pick(g)))
        };
        let p = &result.phases;
        let secs = |d: Duration| d.as_secs_f64();
        outln!(
            "{:>5} {:>13} {:>9.3} {:>9.3} {:>9.3} {:>9.3} {:>9.3}  | {} {}",
            n,
            result.nodes,
            secs(p.preclusions),
            secs(p.greedy),
            secs(p.revise),
            secs(p.search),
            secs(p.total),
            reference(|g| g.1),
            reference(|g| g.2)
        );
        let _ = std::io::stdout().flush();
        results.push(result);
    }
    let total: f64 = results.iter().map(|r| r.phases.total.as_secs_f64()).sum();
    let nodes: u64 = results.iter().map(|r| r.nodes).sum();
    outln!("{:>5} {:>13} {:>59.3}", "all", nodes, total);

    let record = history_record(&opts, &results);
    append_history(&record).map_err(|e| failed(format!("{BENCH_HISTORY}: {e}")))?;
    outln!("appended to {BENCH_HISTORY}");
    Ok(())
}

/// Output of `git` in this crate's directory, if it runs and succeeds.
fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").arg("-C").arg(env!("CARGO_MANIFEST_DIR")).args(args).output().ok()?;
    output.status.success().then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// One line of the benchmark history: when, which code, how it was run, and what each game cost.
fn history_record(opts: &SolveOptions, results: &[GameResult]) -> Json {
    let games = results.iter().map(|r| {
        Json::object([
            ("n", Json::from(r.n)),
            ("score", Json::from(r.score)),
            ("nodes", Json::from(r.nodes)),
            ("optimal_match", r.optimal_match.map_or(Json::Null, Json::Bool)),
            ("seconds", r.phases.to_json()),
            ("stats", r.stats_json()),
        ])
    });
    let unix_time = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    // The history file itself changes with every run, so it does not make the code dirty.
    let dirty = git(&["status", "--porcelain", "--", ".", ":(exclude)bench"]).map(|status| !status.is_empty());
    Json::object([
        ("unix_time", Json::from(unix_time)),
        ("git", git(&["rev-parse", "--short", "HEAD"]).map_or(Json::Null, Json::from)),
        ("dirty", dirty.map_or(Json::Null, Json::Bool)),
        ("threads", Json::from(opts.threads)),
        ("tactics", Json::from(opts.tactics.description())),
        ("tactic_switches", opts.tactics.to_json()),
        ("games", Json::array(games)),
    ])
}

fn append_history(record: &Json) -> std::io::Result<()> {
    let path = Path::new(BENCH_HISTORY);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{record}")
}
