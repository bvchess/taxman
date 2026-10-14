//! The subcommands: `solve`, `regress`, `bench`, `verify-optimal`, `verify-results`, `extend-optimal` and `tables`.
//!
//! The ones that solve games read their flags with [`Args`], build [`SolveOptions`] from them ([`solver_options`])
//! and call the library's pipeline ([`taxman::solver`]); what they add is output, and the policies of a command-line
//! run:
//!
//! * `solve` stops on SIGINT and SIGTERM with a final checkpoint, locks its checkpoint directory and (when it opens
//!   it, `results::ResultsFile`) its results file, and prints one summary line per game as it finishes;
//! * `regress` solves in memory and compares every score with `optimal.json`;
//! * `bench` solves a fixed set of hard games standalone and appends the run to a history file, so that
//!   performance can be followed from commit to commit.  Its files are found in the repository
//!   ([`taxman::paths`]) unless `--history` and `--known` name them.
//!
//! The small ones: `verify-optimal` replays every game of the table of optimal games, `verify-results` checks results
//! files without changing them, `extend-optimal` writes the table with the proven games of a results file appended
//! to it, and `tables` prints the maximal factors and composites of numbers on a board.

use crate::cli::{errln, outln, parse_range, solver_options, Args, CliError, CliResult, SOLVER_FLAGS};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use taxman::engine;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::numtheory::Tables;
use taxman::paths;
use taxman::results::{build, verify_files, Build};
use taxman::solver::{self, GameResult, Method, OptimalSource, Prev, SolveOptions};

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

/// `solve N|A-B [options]`: solves one board size or a range, each from the one before (`solver::solve_range_each`).
pub fn solve(args: &[String]) -> CliResult {
    let usage = || CliError::Usage("usage: solve N|A-B [options]".into());
    let args = Args::parse(args, &flags_with(&SOLVE_FLAGS), &["--force-search", "--show", "--allow-tainted-prev"])?;
    let [range] = args.positional() else { return Err(usage()) };
    let (lo, hi) = parse_range(range)?;

    let mut opts = solver_options(&args)?;
    opts.force_search = args.has("--force-search");
    opts.allow_tainted_prev = args.has("--allow-tainted-prev");
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

    // Two processes in one checkpoint directory would overwrite each other's checkpoints.  (Opening the results
    // file locks it too, against a second run appending to it.)
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

/// `regress [--from A] [--to N]`: solves A..N in memory and checks every score against optimal.json.
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
    // Report every mismatch instead of stopping at the first.  The comparison is with optimal.json and nothing is
    // recorded, so a first game built on optimal.json's is fine whatever the tactics.
    opts.allow_optimal_mismatch = true;
    opts.allow_tainted_prev = true;

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

/// The benchmark games: the last W = 16 game of the fast set and the W = 20 ones near the frontier, each a
/// search of a second or so on one thread (DESIGN.md, "Performance").
const BENCH_GAMES: [u32; 6] = [936, 1050, 1100, 1133, 1170, 1184];

/// Times the benchmark games (or the board sizes given), each standalone, and appends the run to the history.
pub fn bench(args: &[String]) -> CliResult {
    let args = Args::parse(args, &flags_with(&["--history", "--known"]), &[])?;
    // Where the run is recorded, and where the previous games past optimal.json's come from: in the repository
    // unless a flag names a file.
    let file = |flag: &str, in_repository: &str| match args.value(flag) {
        Some(path) => Ok(PathBuf::from(path)),
        None => paths::in_repository(in_repository, &format!("{flag} FILE")).map_err(failed),
    };
    let history = file("--history", paths::BENCH_HISTORY_IN_REPOSITORY)?;
    let results = file("--known", paths::STRICT_RESULTS_IN_REPOSITORY)?;
    let ns: Vec<u32> = if args.positional().is_empty() {
        BENCH_GAMES.to_vec()
    } else {
        let parse = |text: &String| text.parse().map_err(|_| CliError::Usage(format!("not a board size: {text:?}")));
        args.positional().iter().map(parse).collect::<Result<_, _>>()?
    };
    let mut opts = solver_options(&args)?;
    if opts.optimal == OptimalSource::None {
        return Err(CliError::Usage(
            "bench takes the previous games from optimal.json, so --optimal none makes no sense".into(),
        ));
    }
    // Every game on its own: the previous game from optimal.json or, past it, from the committed strict results
    // (whatever the tactics: a benchmark records no result), and no derivation.
    opts.force_search = true;
    opts.allow_tainted_prev = true;
    let known = opts.known_games(&[results]).map_err(failed)?;

    outln!("{} threads, tactics {}", opts.threads, opts.tactics.description());
    let [n, nodes, greedy, revise, search, total] = ["n", "nodes", "greedy s", "revise s", "search s", "total s"];
    outln!("{n:>5} {nodes:>13} {greedy:>9} {revise:>9} {search:>9} {total:>9}");
    let mut results = Vec::new();
    for &n in &ns {
        let prev = n.checked_sub(1).and_then(|before| Prev::from_optimal(&known, before));
        let result = solver::solve_game_with(n, prev.as_ref(), &opts, Some(&known)).map_err(failed)?;
        let p = &result.phases;
        let secs = |d: Duration| d.as_secs_f64();
        outln!(
            "{:>5} {:>13} {:>9.3} {:>9.3} {:>9.3} {:>9.3}",
            n,
            result.nodes,
            secs(p.greedy),
            secs(p.revise),
            secs(p.search),
            secs(p.total)
        );
        let _ = std::io::stdout().flush();
        results.push(result);
    }
    let total: f64 = results.iter().map(|r| r.phases.total.as_secs_f64()).sum();
    let nodes: u64 = results.iter().map(|r| r.nodes).sum();
    outln!("{:>5} {:>13} {:>39.3}", "all", nodes, total);

    let record = history_record(&opts, &results);
    append_history(&history, &record).map_err(|e| failed(format!("{}: {e}", history.display())))?;
    outln!("appended to {}", history.display());
    Ok(())
}

/// One line of the benchmark history: when, which code, how it was run, and what each game cost.  The code is the
/// commit this binary was built from (`results::build`), and whether its sources differed from it.
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
    let Build { commit, dirty } = build();
    let short = commit.map(|commit| &commit[..commit.len().min(7)]);
    Json::object([
        ("unix_time", Json::from(unix_time)),
        ("git", short.map_or(Json::Null, Json::from)),
        ("dirty", dirty.map_or(Json::Null, Json::Bool)),
        ("threads", Json::from(opts.threads)),
        ("tactics", Json::from(opts.tactics.description())),
        ("tactic_switches", opts.tactics.to_json()),
        ("games", Json::array(games)),
    ])
}

fn append_history(path: &Path, record: &Json) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{record}")
}

// ---------------------------------------------------------------------------------------------------------
// verify-optimal, verify-results, extend-optimal, tables

fn millis(since: Instant) -> f64 {
    since.elapsed().as_secs_f64() * 1000.0
}

/// `verify-optimal [FILE]`: replays every game of the table of optimal games (the default one if no file is given)
/// and checks its recorded score.
pub fn verify_optimal(args: &[String]) -> CliResult {
    let path = match args {
        [] => paths::optimal_json().map_err(CliError::Failed)?,
        [file] => file.into(),
        _ => return Err(CliError::Usage("verify-optimal takes at most one argument".into())),
    };
    let start = Instant::now();
    let optimal = Optimal::load(&path).map_err(CliError::Failed)?;
    let load_ms = millis(start);

    let start = Instant::now();
    let checked = optimal.verify_all().map_err(CliError::Failed)?;
    let replay_ms = millis(start);

    let total_moves: usize = optimal.games().map(|(_, _, moves)| moves.len()).sum();
    outln!("verified {checked} games (n up to {}, {total_moves} moves)", optimal.max_n());
    outln!("load+parse {load_ms:.1} ms, replay {replay_ms:.1} ms");
    Ok(())
}

/// `verify-results FILE... [--optimal FILE|none]`: checks results files without changing them
/// (`results::verify_files`).
pub fn verify_results(args: &[String]) -> CliResult {
    let args = Args::parse(args, &["--optimal"], &[])?;
    if args.positional().is_empty() {
        return Err(CliError::Usage("usage: verify-results FILE... [--optimal FILE|none]".into()));
    }
    let optimal = solver_options(&args)?.load_optimal().map_err(CliError::Failed)?;
    let paths: Vec<PathBuf> = args.positional().iter().map(PathBuf::from).collect();
    let start = Instant::now();
    let report = verify_files(&paths, optimal.as_ref()).map_err(CliError::Failed)?;
    outln!("{report}");
    outln!("checked in {:.1} ms", millis(start));
    Ok(())
}

/// `extend-optimal RESULTS --out FILE [--optimal FILE]`: writes the table of optimal games to `FILE` with the games of
/// the results file `RESULTS` that continue it appended (`results::extend_optimal`).  The table is read from
/// `--optimal` as for the other commands; it is written to `FILE` only, which the command line must name.
pub fn extend_optimal(args: &[String]) -> CliResult {
    let usage = || CliError::Usage("usage: extend-optimal RESULTS --out FILE [--optimal FILE]".into());
    let args = Args::parse(args, &["--out", "--optimal"], &[])?;
    let [results] = args.positional() else { return Err(usage()) };
    let Some(out) = args.value("--out") else {
        return Err(CliError::Usage(
            "extend-optimal needs --out FILE: it never writes over the table (or anything else) unasked".into(),
        ));
    };
    let (results, out) = (Path::new(results), Path::new(out));
    // The results are the evidence the table is made from: a mistyped --out must not replace them.
    if std::fs::canonicalize(results).ok().is_some_and(|results| std::fs::canonicalize(out).ok() == Some(results)) {
        return Err(CliError::Usage(format!(
            "--out is the results file {}, which would be overwritten",
            out.display()
        )));
    }
    let Some(mut optimal) = solver_options(&args)?.load_optimal().map_err(failed)? else {
        return Err(CliError::Usage("extend-optimal extends optimal.json, so --optimal none makes no sense".into()));
    };

    let contents = taxman::results::read_results_file(results, Some(&optimal)).map_err(failed)?;
    if contents.torn {
        errln!("note: the last line of {} is torn and was ignored", results.display());
    }
    let extension = taxman::results::extend_optimal(&mut optimal, &contents).map_err(failed)?;
    optimal.save(out).map_err(failed)?;

    let (count, total) = (extension.added.clone().count(), optimal.sizes().count());
    let added = if count == 0 {
        "added no games".to_owned()
    } else {
        let games = if count == 1 { "game" } else { "games" };
        format!("added {count} {games} (n = {}..={})", extension.added.start(), extension.added.end())
    };
    outln!("{added}; the table has {total} games, n up to {}; wrote {}", optimal.max_n(), out.display());
    outln!("stopped at n = {}: {}", extension.stopped_at, extension.reason);
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
