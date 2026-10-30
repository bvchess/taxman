//! The subcommands: `solve`, `regress`, `verify-results`, `tables`, `certify` and `verify-certificates`.
//!
//! The ones that solve games read their flags with [`Args`], build [`SolveOptions`] from them ([`solver_options`],
//! or [`range_options`] for those that solve a range) and call the library's pipeline ([`taxman::solver`]); what
//! they add is output, and the policies of a command-line run:
//!
//! * `solve` stops on SIGINT and SIGTERM with a final checkpoint, locks its checkpoint directory and (when it opens
//!   it, `results::ResultsFile`) its results file, prints one summary line per game as it is concluded, and with
//!   `--game-hours` ends at the first game whose own search passes the wall (exit status 4);
//! * `regress` solves in memory and compares every score with the table of optimal games;
//! * `certify` searches as `solve` does (the same flags for checkpoints, limits and progress, the same lock and
//!   signals), appends a certificate a game to its `--out` file (locked while it is open), takes the games of a range
//!   or of a list file, and exits with status 2 on a game that beats a recorded score.
//!
//! The small ones: `verify-results` checks results files without changing them (a path that ends in `.gz` is read
//! through the gzip decoder, as every results file a command is given by path), `verify-certificates` does the same for
//! the files `certify` wrote and says how far the proofs of the recorded scores reach, and `tables` prints the maximal
//! factors and composites of numbers on a board.  Every command that meets a score of a game the table of optimal games
//! has checks it against the table built into the binary ([`Optimal::built_in`]): no flag turns that off.

use crate::cli::{
    count, errln, outln, parse_range, range_options, read_board_list, solver_options, Args, CliError, CliResult,
    SOLVER_FLAGS, WINDOW_FLAG,
};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use taxman::certify::verify::{
    compact_ranges, frontier, read_exact, verify_certificates as verify_certificates_in, Frontier, Gap,
};
use taxman::certify::{certify_game_with, read_upper, CertificateFile, EarlierCertificates, Upper, Verdict};
use taxman::engine;
use taxman::game::Optimal;
use taxman::known::{Known, KnownSources};
use taxman::numtheory::Tables;
use taxman::paths;
use taxman::results::verify_files_with;
use taxman::search::{finder, gate_trial, MAX_GATE};
use taxman::solver::{self, GameResult, Method, SolveOptions, Wall};

fn failed(message: String) -> CliError {
    CliError::Failed(message)
}

/// The known sources of the repository (the committed chain and the bounds store in `results`) with `optimal` as
/// the table, for checking results that cite them; the table alone if there is no repository.
fn known_in_repository(optimal: &Optimal) -> Result<Known<'_>, CliError> {
    let sources = KnownSources::in_repository().unwrap_or_default();
    Known::load(optimal, &sources).map_err(failed)
}

/// The games that `certify` and `verify-certificates` take as recorded: those of the results files, and the built-in
/// table's for any they lack.  A file's game stands where it scores differently from the table's: what is certified is
/// the file's score, and a wrong score is the counterexample that `certify` is there to find, not a refused file
/// (`verify-results` is the check of a file against the table).
fn recorded_games(results: &[PathBuf]) -> Result<Optimal, CliError> {
    let table = Optimal::built_in().clone();
    results.iter().try_fold(table, |recorded, path| recorded.with_results(path)).map_err(failed)
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

/// The switch of a long search that `solve` and `certify` share, read by [`search_options`]: the workers run at the
/// lowest priority ([`SolveOptions::background`]).
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
    let bad = || CliError::Usage(format!("--gate-trial takes gates from 0 to {MAX_GATE} separated by commas: {list}"));
    let gates: Vec<u64> =
        list.split(',').map(|gate| gate.trim().parse().map_err(|_| bad())).collect::<Result<_, _>>()?;
    if gates.is_empty() || gates.iter().any(|&gate| gate > MAX_GATE) {
        return Err(bad());
    }
    Ok(gates)
}

/// The gate of `--gate`, a whole number from 0 to [`MAX_GATE`], if it is given: the gap from which conflict branching
/// chooses its conflict in every game of the run.  A run with conflict branching off, or without the GF(2) bound it
/// acts with, has no use for one, so the flag is refused there.
fn run_gate(args: &Args, opts: &SolveOptions) -> Result<Option<u64>, CliError> {
    let Some(gate) = args.parsed::<u64>("--gate")? else { return Ok(None) };
    if gate > MAX_GATE {
        return Err(CliError::Usage(format!("--gate takes a gap from 0 to {MAX_GATE}: {gate}")));
    }
    let tactics = &opts.tactics.search;
    let off = if !tactics.conflict_branching {
        "conflict branching is off (--tactic conflict_branching=off)"
    } else if !tactics.gf2 {
        "conflict branching acts only with the GF(2) bound, which is off (--tactic gf2=off)"
    } else {
        return Ok(Some(gate));
    };
    Err(CliError::Usage(format!(
        "--gate sets the gap from which conflict branching chooses its conflict, and {off}: a gate without conflict \
         branching does nothing"
    )))
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

const SOLVE_FLAGS: [&str; 12] = [
    "--out",
    WINDOW_FLAG,
    "--game-hours",
    "--pass",
    "--finder-after",
    "--finder-seconds",
    "--profile",
    "--profile-depth",
    "--carried-dir",
    "--gate",
    "--gate-trial",
    "--gate-trial-plan",
];

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
    let switches = [
        &["--force-search", "--use-prev", "--show", "--keep-going", "--retry", "--retry-improved"][..],
        &SEARCH_SWITCHES[..],
    ]
    .concat();
    let args = Args::parse(args, &flags_with(&flags), &switches)?;
    let [range] = args.positional() else { return Err(usage()) };
    let (lo, hi) = parse_range(range)?;

    let mut opts = range_options(&args)?;
    opts.force_search = args.has("--force-search");
    opts.use_prev = args.has("--use-prev");
    if opts.use_prev {
        // The known games: the table's, and the committed chain's and the bounds store's in the repository.
        opts.known = KnownSources::in_repository().unwrap_or_else(|e| {
            errln!("note: {e}; --use-prev draws on the built-in table alone");
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
    if let Some(gates) = args.value("--gate-trial") {
        let plan = gate_trial::Plan::of(trial_gates(gates)?);
        let spec = args.value("--gate-trial-plan").unwrap_or_default();
        opts.gate_trial = Some(plan.with(spec).map_err(|e| CliError::Usage(format!("--gate-trial-plan: {e}")))?);
    } else if args.value("--gate-trial-plan").is_some() {
        return Err(CliError::Usage("--gate-trial-plan sets the trial of --gate-trial: give it too".into()));
    }
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
/// scores and games come from the results files ([`recorded_games`]: the table of optimal games has any they lack), the
/// open members from the prover's barred bounds (`upper` files).  A game whose line there does not say `needs` (a prime, or
/// a score at its ceiling, which rest on n − 1) is passed over.  Each game's certificate is appended to `--out` as it
/// is made, and a rerun skips the games the file has a certificate for and resumes a stopped search from its
/// checkpoint.  The extended problem is searched with the claims of `unzip` that the kept-claims lemma (E4,
/// docs/theory/extended-problem.md) keeps, unless `--no-claims` says to search it without them (the formulation of the
/// certificates made before that lemma); each certificate records its formulation.  A game whose extended problem a
/// certified game of the last 8 games (of this run, or of the file, made in either formulation) posed, with a recorded
/// game worth what that game's certificate proved the most, is certified with no search (`same_problem`;
/// [`taxman::certify`]).  A game found to beat its recorded score is printed, written to `<out>.counterexample.jsonl`
/// too, and ends the run with status 2, as does a rerun over a game the file records beaten.
pub fn certify(args: &[String]) -> CliResult {
    let usage = |why: &str| {
        CliError::Usage(format!(
            "{why}\nusage: certify (A-B | --list FILE) --results FILE... --upper FILE... --out FILE [options]"
        ))
    };
    let args = Args::parse(
        args,
        &[&CERTIFY_FLAGS[..], &SEARCH_FLAGS[..]].concat(),
        &[&["--no-claims"][..], &SEARCH_SWITCHES[..]].concat(),
    )?;
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

    let known = recorded_games(&results)?;
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

    let known = recorded_games(&results)?;
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
/// the table of optimal games (the first that differs is an error, as in any run).
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
