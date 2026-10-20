//! The subcommands: `solve`, `regress`, `bench`, `verify-optimal`, `verify-results`, `extend-optimal`, `tables`,
//! `certify` and `verify-certificates`.
//!
//! The ones that solve games read their flags with [`Args`], build [`SolveOptions`] from them ([`solver_options`],
//! or [`range_options`] for those that solve a range) and call the library's pipeline ([`taxman::solver`]); what
//! they add is output, and the policies of a command-line run:
//!
//! * `solve` stops on SIGINT and SIGTERM with a final checkpoint, locks its checkpoint directory and (when it opens
//!   it, `results::ResultsFile`) its results file, prints one summary line per game as it is concluded, and with
//!   `--game-hours` ends at the first game whose own search passes the wall (exit status 4);
//! * `regress` solves in memory and compares every score with `optimal.json`;
//! * `bench` solves a fixed set of hard games standalone and appends the run to a history file, so that
//!   performance can be followed from commit to commit.  Its files are found in the repository
//!   ([`taxman::paths`]) unless `--history` and `--known` name them;
//! * `certify` searches as `solve` does (the same flags for checkpoints, limits and progress, the same lock and
//!   signals), appends a certificate a game to its `--out` file (locked while it is open), takes the games of a range
//!   or of a list file, and exits with status 2 on a game that beats a recorded score.
//!
//! The small ones: `verify-optimal` replays every game of the table of optimal games, `verify-results` checks results
//! files without changing them, `verify-certificates` does the same for the files `certify` wrote and says how far the
//! proofs of the recorded scores reach, `extend-optimal` writes the table with the games of a results file that
//! continue it appended, and `tables` prints the maximal factors and composites of numbers on a board.  Every
//! command that meets a score of a game `optimal.json` has checks it against the table built into the binary
//! ([`Optimal::built_in`]): no flag turns that off or points at another file.

use crate::cli::{
    count, errln, outln, parse_range, range_options, read_board_list, solver_options, Args, CliError, CliResult,
    SOLVER_FLAGS, WINDOW_FLAG,
};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use taxman::certify::verify::{
    compact_ranges, frontier, read_exact, verify_certificates as verify_certificates_in, Frontier, Gap,
};
use taxman::certify::{certify_game_with, read_upper, CertificateFile, EarlierCertificates, Upper, Verdict};
use taxman::engine;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::known::{Known, KnownSources};
use taxman::numtheory::Tables;
use taxman::paths;
use taxman::results::{build, verify_files_with, Build};
use taxman::solver::{self, GameResult, Method, Prev, SolveOptions, Wall};

fn failed(message: String) -> CliError {
    CliError::Failed(message)
}

/// The known sources of the repository (the committed chain and the bounds store in `results`) with `optimal` as
/// the table, for checking results that cite them; the table alone if there is no repository.
fn known_in_repository(optimal: &Optimal) -> Result<Known<'_>, CliError> {
    let sources = KnownSources::in_repository().unwrap_or_default();
    Known::load(optimal, &sources).map_err(failed)
}

/// Flags that take a value: the solver's and the command's own.
fn flags_with(own: &[&'static str]) -> Vec<&'static str> {
    [&SOLVER_FLAGS[..], own].concat()
}

// ---------------------------------------------------------------------------------------------------------
// solve

/// The flags of a long search that `solve` and `certify` share, read by [`search_options`]: checkpoints, progress,
/// limits and memory.
const SEARCH_FLAGS: [&str; 6] =
    ["--checkpoint-dir", "--checkpoint-interval", "--progress", "--time-limit", "--node-limit", "--mem-cap"];

/// Reads [`SEARCH_FLAGS`] into `opts`.
fn search_options(args: &Args, opts: &mut SolveOptions) -> Result<(), CliError> {
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

/// The line that says why the search of `n` stopped before it finished, and how to go on.
fn stopped_line(n: u32, opts: &SolveOptions) -> String {
    let resume =
        if opts.checkpoint_dir.is_some() { "run the same command to resume it" } else { "it was not checkpointed" };
    let why = if engine::stop_signal().is_some() { "was stopped by a signal" } else { "hit a limit" };
    // A range that keeps going past limits is ended only by a signal, and records nothing for the game it stops.
    let recorded = if opts.keep_going { "; nothing is recorded for it" } else { "" };
    format!("stopped: the search for n={n} {why} before it finished{recorded}; {resume}")
}

const SOLVE_FLAGS: [&str; 4] = ["--out", WINDOW_FLAG, "--game-hours", "--pass"];

/// The line `solve` prints when a game passes the wall, `wall at N after H hours`, then why and what is kept.  A script
/// reads the words up to the colon (`chain.sh`).
fn wall_line(wall: Wall, opts: &SolveOptions) -> String {
    let hours = |time: Duration| time.as_secs_f64() / 3600.0;
    let limit = opts.game_limit.map_or(0.0, hours);
    let resume = if opts.checkpoint_dir.is_some() {
        "its checkpoint and clock are kept, and nothing is recorded for it; run the same command with a larger \
         --game-hours to go on with it"
    } else {
        "it was not checkpointed"
    };
    format!(
        "wall at {} after {:.5} hours: its own search passed the limit of {limit} hours; {resume}",
        wall.n,
        hours(wall.held)
    )
}

/// `solve N|A-B [options]`: solves game N or a range, each from the one before (`solver::solve_range_each`).
pub fn solve(args: &[String]) -> CliResult {
    let usage = || CliError::Usage("usage: solve N|A-B [options]".into());
    let flags = [&SOLVE_FLAGS[..], &SEARCH_FLAGS[..]].concat();
    let switches = ["--force-search", "--use-prev", "--show", "--keep-going", "--retry", "--retry-improved"];
    let args = Args::parse(args, &flags_with(&flags), &switches)?;
    let [range] = args.positional() else { return Err(usage()) };
    let (lo, hi) = parse_range(range)?;

    let mut opts = range_options(&args)?;
    opts.force_search = args.has("--force-search");
    opts.use_prev = args.has("--use-prev");
    if opts.use_prev {
        // The known games: optimal.json's, and the committed chain's and the bounds store's in the repository.
        opts.known = KnownSources::in_repository().unwrap_or_else(|e| {
            errln!("note: {e}; --use-prev draws on optimal.json alone");
            KnownSources { boards: true, ..KnownSources::default() }
        });
    }
    opts.keep_going = args.has("--keep-going");
    opts.retry = args.has("--retry");
    opts.retry_improved = args.has("--retry-improved");
    opts.pass = args.parsed("--pass")?;
    if (opts.retry || opts.retry_improved) && !opts.keep_going {
        return Err(CliError::Usage(
            "--retry and --retry-improved search again the games that --keep-going passed over: give --keep-going \
             too (without it a rerun resumes the game a limit stopped)"
                .into(),
        ));
    }
    if opts.retry && opts.retry_improved {
        return Err(CliError::Usage(
            "--retry searches again every unfinished game and --retry-improved only those whose game before it has \
             improved: give one"
                .into(),
        ));
    }
    opts.game_limit = args.positive_hours("--game-hours")?;
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
    if summary.passed_over > 0 {
        outln!(
            "passed over {} games that the file holds only an unfinished search of (--retry searches them again)",
            summary.passed_over
        );
    }
    if summary.unfinished > 0 {
        let kept = if opts.checkpoint_dir.is_some() { "their checkpoints are kept" } else { "not checkpointed" };
        outln!(
            "{} games ended at a limit, unfinished ({kept}); the games after each went on from its best game",
            summary.unfinished
        );
    }
    if let Some(n) = summary.stopped_at {
        outln!("{}", stopped_line(n, &opts));
    }
    if let Some(wall) = summary.walled {
        outln!("{}", wall_line(wall, &opts));
        return Err(CliError::Wall);
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// certify

const CERTIFY_FLAGS: [&str; 5] = ["--results", "--upper", "--out", "--threads", "--list"];

/// `certify (A-B | --list FILE) --results FILE... --upper FILE... --out FILE [options]`: certifies, game by game, the
/// recorded scores of A..B, or of the boards of the list FILE in its order (one n a line, `#` for comments;
/// [`read_board_list`]), that rest on the frame model, by a search of the extended problem ([`taxman::certify`]).  The
/// scores and games come from the results files (over the table of optimal games, which must agree with them), the
/// open members from the prover's barred bounds (`upper` files).  A game whose line there does not say `needs` (a
/// prime, or a score at its ceiling, which rest on n − 1) is passed over.  Each game's certificate is appended to
/// `--out` as it is made, and a rerun skips the games the file has a certificate for and resumes a stopped search
/// from its checkpoint.  The extended problem is searched with the claims of `unzip` that Lemma M5⁺ keeps, unless
/// `--no-claims` says to search it without them (the formulation of the certificates made before that lemma); each
/// certificate records its formulation.  A game whose extended problem a certified game of the last 8 games (of
/// this run, or of the file, made in either formulation) posed, with a recorded game worth what that game's
/// certificate proved the most, is certified with no search (`same_problem`; [`taxman::certify`]).  A game found to
/// beat its recorded score is printed, written to `<out>.counterexample.jsonl` too, and ends the run with status 2, as
/// does a rerun over a game the file records beaten.
pub fn certify(args: &[String]) -> CliResult {
    let usage = |why: &str| {
        CliError::Usage(format!(
            "{why}\nusage: certify (A-B | --list FILE) --results FILE... --upper FILE... --out FILE [options]"
        ))
    };
    let args = Args::parse(args, &[&CERTIFY_FLAGS[..], &SEARCH_FLAGS[..]].concat(), &["--no-claims"])?;
    let claims = !args.has("--no-claims");
    // The boards in the order to certify them, and how the tally names them.
    let lists = args.values("--list");
    let (boards, label) = match (args.positional(), lists.as_slice()) {
        ([range], []) => {
            let (lo, hi) = parse_range(range)?;
            ((lo..=hi).collect::<Vec<_>>(), format!("{lo}..={hi}"))
        }
        ([], [list]) => {
            let boards = read_board_list(Path::new(list))?;
            let label = format!("list {list} ({} boards)", boards.len());
            (boards, label)
        }
        ([_], [_]) => return Err(usage("certify takes a range A-B or --list FILE, not both")),
        ([], [_, ..]) => return Err(usage("certify takes one --list FILE")),
        _ => return Err(usage("certify takes one range A-B or --list FILE")),
    };
    let files = |flag: &str| args.values(flag).into_iter().map(PathBuf::from).collect::<Vec<_>>();
    let (results, uppers) = (files("--results"), files("--upper"));
    if results.is_empty() || uppers.is_empty() {
        return Err(usage("certify needs --results FILE and --upper FILE"));
    }
    let out = PathBuf::from(args.value("--out").ok_or_else(|| usage("certify needs --out FILE"))?);
    let mut opts = SolveOptions::default();
    if let Some(threads) = count(&args, "--threads")? {
        opts.threads = threads;
    }
    search_options(&args, &mut opts)?;

    let known = Optimal::built_in().with_results_files(&results).map_err(failed)?;
    let upper = read_upper(&uppers).map_err(failed)?;
    let mut file = CertificateFile::open(&out).map_err(failed)?;
    let mut earlier = EarlierCertificates::of_file(&file);
    let _lock = guard_checkpoints(&opts)?;

    let mut tally = BTreeMap::<&str, usize>::new();
    for n in boards {
        let line = upper.get(&n).ok_or_else(|| failed(format!("n={n}: the --upper files have no line for it")))?;
        if !line.needs {
            *tally.entry("not needing one").or_default() += 1;
            continue;
        }
        match file.verdict(n) {
            Some(Verdict::Beaten) => {
                // A counterexample stays one: a rerun over it says so again, rather than pass it over.
                let path = file.counterexample_path();
                return Err(CliError::Counterexample(format!(
                    "n={n}: --out already records a game that beats the recorded score (see {})",
                    path.display()
                )));
            }
            Some(_) => {
                *tally.entry("already in --out").or_default() += 1;
                continue;
            }
            None => {}
        }
        let certificate = certify_game_with(n, &known, line, claims, &opts, &mut earlier).map_err(failed)?;
        file.append(&certificate).map_err(failed)?;
        outln!("{}", certificate.summary_line());
        let _ = std::io::stdout().flush();
        match certificate.verdict {
            Verdict::Incomplete => {
                outln!("{}", stopped_line(n, &opts));
                return Ok(());
            }
            Verdict::Beaten => {
                // The line is safe in --out: the checkpoint can go.
                solver::discard_checkpoint(&opts, n);
                let path = file.append_counterexample(&certificate).map_err(failed)?;
                let (score, moves) = certificate.beaten_by.as_ref().expect("a game that beats the score");
                outln!("  the game of {score}: {moves:?}");
                return Err(CliError::Counterexample(format!(
                    "n={n}: a game of {score} beats the recorded score {} (written to {})",
                    certificate.score,
                    path.display()
                )));
            }
            Verdict::Certified => {
                solver::discard_checkpoint(&opts, n);
                let how = if certificate.same_problem.is_some() { "certified (same problem)" } else { "certified" };
                *tally.entry(how).or_default() += 1;
            }
        }
    }
    let counts: Vec<String> = tally.iter().map(|(what, count)| format!("{what} {count}")).collect();
    outln!("{label}: {}", counts.join(", "));
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// verify-certificates

/// `verify-certificates CERTS... --results FILE... --upper FILE... [--exact FILE...] [--threads K]`: checks the files
/// of certificates `taxman certify --out` wrote without changing them ([`taxman::certify::verify`]): every line
/// against the recorded games, the barred bounds and Π⁺(O) set up again, and the games that rest on one another; the
/// first that fails ends the command with status 1.  Then it says how far the proofs of the recorded scores reach:
/// every game to some board is proven by the prover (`--exact`, default every `exact_*.jsonl` of the repository's
/// `solver/noframe/data`), by a certificate that passed, or by a ceiling above a proven game.  K threads set Π⁺(O) up
/// again for the lines (default: all CPUs), as `verify-results` does for the repeats.
pub fn verify_certificates(args: &[String]) -> CliResult {
    let usage = |why: &str| {
        CliError::Usage(format!(
            "{why}\nusage: verify-certificates CERTS... --results FILE... --upper FILE... [--exact FILE...] \
             [--threads K]"
        ))
    };
    let args = Args::parse(args, &["--results", "--upper", "--exact", "--threads"], &[])?;
    let files = |flag: &str| args.values(flag).into_iter().map(PathBuf::from).collect::<Vec<_>>();
    let certificates: Vec<PathBuf> = args.positional().iter().map(PathBuf::from).collect();
    if certificates.is_empty() {
        return Err(usage("verify-certificates needs a file of certificates"));
    }
    let (results, uppers) = (files("--results"), files("--upper"));
    if results.is_empty() || uppers.is_empty() {
        return Err(usage("verify-certificates needs --results FILE and --upper FILE"));
    }
    let threads = solver_options(&args)?.threads;
    let start = Instant::now();
    let exact = match files("--exact") {
        given if !given.is_empty() => given,
        _ => paths::exact_files_in_repository().map_err(failed)?,
    };

    let known = Optimal::built_in().with_results_files(&results).map_err(failed)?;
    let upper = read_upper(&uppers).map_err(failed)?;
    let verified = verify_certificates_in(&certificates, &known, &upper, threads).map_err(failed)?;
    for file in verified.files.iter().filter(|file| file.torn) {
        errln!("note: {}: torn last line ignored", file.path.display());
    }
    outln!("{verified}");

    let proofs = read_exact(&exact).map_err(failed)?;
    let scores = known.games().map(|(n, score, _)| (n, score)).collect();
    let reached = frontier(&scores, &proofs, &verified.certified).map_err(failed)?;
    let names: Vec<String> = exact.iter().map(|path| path.display().to_string()).collect();
    outln!("the prover's proofs read, from {} files: {}", names.len(), names.join(", "));
    outln!("{}", frontier_text(&reached, &upper));
    outln!("checked in {:.1} s", start.elapsed().as_secs_f64());
    Ok(())
}

/// The lines that say how far the proofs reach: every game to `reached.through` is proven, the first that is not and
/// why (the barred bounds `upper` say whether it needs a certificate), and the games past it that are proven anyway.
fn frontier_text(reached: &Frontier, upper: &BTreeMap<u32, Upper>) -> String {
    let through = reached.through;
    let Some((n, gap)) = reached.first_gap else {
        return format!("proven frontier: every game to {through} is proven, the largest board of the results");
    };
    let why = match gap {
        Gap::NoGame => "no game in the results",
        Gap::Unproven if upper.get(&n).is_some_and(|line| line.needs) => {
            "needs a certificate and has none in the files given"
        }
        Gap::Unproven => "no proof by the prover, no certificate, and not at a proven ceiling",
    };
    let beyond = &reached.beyond;
    let list = if beyond.is_empty() { String::new() } else { format!(": {}", compact_ranges(beyond.iter().copied())) };
    format!(
        "proven frontier: every game to {through} is proven; the first that is not is {n} ({why})\n\
         proven past it all the same: {} games{list}",
        beyond.len()
    )
}

// ---------------------------------------------------------------------------------------------------------
// regress

/// `regress [--from A] [--to N] [--window K] [--use-prev]`: solves A..N in memory and checks every score against
/// optimal.json (the first that differs is an error, as in any run).
pub fn regress(args: &[String]) -> CliResult {
    let args = Args::parse(args, &flags_with(&["--to", "--from", WINDOW_FLAG]), &["--use-prev"])?;
    if !args.positional().is_empty() {
        return Err(CliError::Usage("regress takes no positional arguments".into()));
    }
    let from = args.parsed::<u32>("--from")?.unwrap_or(1);
    let to = args.parsed::<u32>("--to")?.unwrap_or(400);
    let mut opts: SolveOptions = range_options(&args)?;
    opts.use_prev = args.has("--use-prev");

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
    outln!("{} games match optimal.json, {unchecked} are not in it", results.len() - unchecked);
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// bench

/// The benchmark games: the last W = 16 game of the fast set and the W = 20 ones near the end of the chain, each a
/// search of a second or so on one thread (solver/docs/design/performance.md).
const BENCH_GAMES: [u32; 6] = [936, 1050, 1100, 1133, 1170, 1184];

/// Times the benchmark games (or the games given), each standalone, and appends the run to the history.
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
        let parse = |text: &String| text.parse().map_err(|_| CliError::Usage(format!("not a game n: {text:?}")));
        args.positional().iter().map(parse).collect::<Result<_, _>>()?
    };
    let mut opts = solver_options(&args)?;
    // Every game on its own: the previous game from optimal.json or, past it, from the committed results, and no
    // derivation.
    opts.force_search = true;
    let known = Optimal::built_in().with_results_files(&[results]).map_err(failed)?;

    outln!("{} threads, tactics {}", opts.threads, opts.tactics.description());
    let [n, nodes, greedy, search, total] = ["n", "nodes", "greedy s", "search s", "total s"];
    outln!("{n:>5} {nodes:>13} {greedy:>9} {search:>9} {total:>9}");
    let mut results = Vec::new();
    for &n in &ns {
        let prev = n.checked_sub(1).and_then(|before| Prev::from_optimal(&known, before));
        let result = solver::solve_game_with(n, prev.as_ref(), &opts, &known).map_err(failed)?;
        let p = &result.phases;
        let secs = |d: Duration| d.as_secs_f64();
        outln!("{:>5} {:>13} {:>9.3} {:>9.3} {:>9.3}", n, result.nodes, secs(p.greedy), secs(p.search), secs(p.total));
        let _ = std::io::stdout().flush();
        results.push(result);
    }
    let total: f64 = results.iter().map(|r| r.phases.total.as_secs_f64()).sum();
    let nodes: u64 = results.iter().map(|r| r.nodes).sum();
    outln!("{:>5} {:>13} {:>29.3}", "all", nodes, total);

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

/// `verify-optimal [FILE]`: replays every game of the table of optimal games (the built-in one if no file is given)
/// and checks its recorded score.
pub fn verify_optimal(args: &[String]) -> CliResult {
    let start = Instant::now();
    let optimal = match args {
        [] => Optimal::built_in().clone(),
        [file] => Optimal::load(file).map_err(CliError::Failed)?,
        _ => return Err(CliError::Usage("verify-optimal takes at most one argument".into())),
    };
    let load_ms = millis(start);

    let start = Instant::now();
    let checked = optimal.verify_all().map_err(CliError::Failed)?;
    let replay_ms = millis(start);

    let total_moves: usize = optimal.games().map(|(_, _, moves)| moves.len()).sum();
    outln!("verified {checked} games (n up to {}, {total_moves} moves)", optimal.max_n());
    outln!("load+parse {load_ms:.1} ms, replay {replay_ms:.1} ms");
    Ok(())
}

/// `verify-results [--threads K] FILE...`: checks results files without changing them (`results::verify_files`), their
/// scores against the built-in `optimal.json`.  The check sets up the search problems of the repeats on K threads,
/// as many as `solve` takes by default (the CPUs) unless `--threads` says.  A line that cites a known source (proof
/// `known`) is checked against it: the table, the committed chain and the bounds store of the repository's
/// `results` (as `solve --use-prev` draws on them).
pub fn verify_results(args: &[String]) -> CliResult {
    let args = Args::parse(args, &["--threads"], &[])?;
    if args.positional().is_empty() {
        return Err(CliError::Usage("usage: verify-results [--threads K] FILE...".into()));
    }
    let paths: Vec<PathBuf> = args.positional().iter().map(PathBuf::from).collect();
    let threads = solver_options(&args)?.threads;
    let start = Instant::now();
    let known = known_in_repository(Optimal::built_in())?;
    let report = verify_files_with(&paths, &known, threads).map_err(CliError::Failed)?;
    outln!("{report}");
    outln!("checked in {:.1} ms", millis(start));
    Ok(())
}

/// `extend-optimal RESULTS --out FILE [--table FILE]`: writes the table of optimal games to `--out FILE` with the
/// games of the results file `RESULTS` that continue it appended (`results::extend_optimal`).  The table is the
/// built-in one, or the file `--table` names (say a table extended before the binary was rebuilt with it); it is
/// written to `--out` only, which the command line must name, and checks the results' scores.
pub fn extend_optimal(args: &[String]) -> CliResult {
    let usage = || CliError::Usage("usage: extend-optimal RESULTS --out FILE [--table FILE]".into());
    let args = Args::parse(args, &["--out", "--table"], &[])?;
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
    let mut optimal = match args.value("--table") {
        Some(table) => Optimal::load(table).map_err(failed)?,
        None => Optimal::built_in().clone(),
    };

    let known = known_in_repository(&optimal)?;
    let contents = taxman::results::read_results_file_with(results, &known, 1).map_err(failed)?;
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
