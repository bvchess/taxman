//! The counterexample hunt: for the sources that the prover's bounds leave open, a search of the board without each
//! of them for a game that scores more than the recorded optimum.
//!
//! Game `n` has numbers `2..=n`; a *source* is a number above `n/2`, which is nobody's factor.  `U*` is the set of
//! sources the frame solver keeps.  The open question, `U` (heuristics/THEORY.md), is whether some optimal game plays
//! every member of `U*` that has three or more prime factors.  The prover's `upper` binary writes, per game, the
//! recorded score `V` and the *open* members `u` of `U*`: those its bounds could not show that every better game
//! must hold, as `"open":[[u,omega,margin],...]`, with the game's `"gap"` and `"needs"`.  For an open `u` this
//! searches the board `[n]` without `u` (`Tables::punctured(n, &[u])`) with the frame solver for a game that scores
//! more than `V` (`taxman::sacrifice::hunt`).  A game of the punctured board never plays `u` and, `u` being nobody's
//! factor, is a legal game of `[n]` with the same moves and score, so a hit is a counterexample to `V` being optimal,
//! and it is replayed on the full board here.  The search starts from a handicapped incumbent worth `V − in_the_bag`
//! with an empty payload, so it looks only for games above `V` (the method of `tests/soundness.rs`).
//!
//! **Margins and probes.**  A pair's cost follows its *margin*, the barred parity bound's excess over `V` (`bench/hunt`),
//! and the pairs of large margin are the ones that matter most (a counterexample needs the punctured problem's own slack
//! to be below the margin, which asks the less of the relaxation the closer the margin is to the root's gap), but they do
//! not finish.  So a pair of margin up to `--probe-above` (default 900) runs to completion under the time limit
//! `--minutes` (default 10), and a pair of larger margin, or of none (`null`), is a *probe*: the solver's own *start* on
//! the punctured board (the greedy set, the survivors of the recorded game of `n` if `--results` has it, the dive and
//! the local search; a game of it above `V` ends the pair) and then a search of `--probe-minutes` (default 3).  Pairs run
//! in decreasing margin, so that a run cut short has done the informative ones.  What a verdict is worth: `found` is a real game (refereed on both boards); `none` says the search finished, which
//! is evidence only, as far as the frame model on the punctured board is complete (it is validated, not proven,
//! there); `probed` is a probe that found nothing, whether or not its search happened to finish (the line says),
//! and is never recorded as `none`; `timeout` says nothing.
//!
//! ```text
//! cargo run --release --example hunt -- --upper FILE [--games 20] [--per-game K] [--list N:U,...] [--minutes 10]
//!                                       [--probe-above 900] [--probe-minutes 3] [--results FILE]...
//!                                       [--threads 4] [--jobs 1] --out FILE
//! ```
//!
//! * `--upper FILE`: the prover's `upper` file (rust/prover/data/upper_*.jsonl), one JSON object per game.
//! * The pairs `(n, u)`.  With `--list N:U,...`, exactly those, in that order (each `n` must have a line in the file,
//!   which gives `V`; `u` must be above `n/2` and at most `n`; its omega and margin are those of the file's list of
//!   open members, `null` for a margin when it is not there).  Otherwise the `--games` (default 20) games with
//!   `"needs":1` of the largest `gap` (ties: the smaller `n` first), and of each its open members, the `--per-game`
//!   of largest margin if given (all by default); the work goes in decreasing margin over all of them (a null margin
//!   counts as the largest; ties: the smaller `n`, then `u`).
//! * `--minutes M` (default 10, a fraction is allowed): the time limit of the search of each pair that is not a probe;
//!   `--probe-above MARGIN` and `--probe-minutes M` (default 900 and 3): see above.
//! * `--results FILE` (repeatable): solver results (`results/strict.jsonl`), whose recorded game of `n` seeds the start
//!   of the probes of `n`, the first file having a game for `n` winning.  Without it the start has no seed.
//! * `--threads T` (default 4): the engine threads of each pair; `--jobs J` (default 1) runs J pairs at once, each
//!   on its own T threads.
//! * `--out FILE`: one line per pair, appended and flushed as the pair finishes.  Pairs that FILE already holds (any
//!   verdict) are skipped, so an interrupted run resumes; a last line cut short by the interruption is cut off first.
//!
//! **A line.**  `{"n":..,"u":..,"omega":..,"margin":..|null,"score":V,"in_the_bag":..,"start_score":..|null,
//! "verdict":..,"nodes":..,"secs":..,"threads":..,"minutes":..}`, with `in_the_bag` the score of the punctured board's
//! game with no promotions (`null` for an error that gave none), `start_score` the score of the game of the start of a
//! probe (what the heuristics reached, to compare with `V`; `null` for a pair that is not a probe, was settled before
//! the start, or is an error), `nodes` the
//! search's steps, `minutes` the time limit the pair's search had, and the verdict one of `found`, `none`, `probed`,
//! `timeout` and `error`.  A `probed` line adds `"finished"`, whether the probe's search ran to its end (then it says what
//! `none` would).  A `found` line adds `"game_score"` (the score of the game, above `V`) and `"moves"` (the game, which
//! replays on the full board `[n]` to `game_score`).  An `error` line adds `"error"`: the message of a hunt that failed,
//! or of a found game that does not replay on the full board to its score (its `"game_score"` and `"moves"` come too).
//!
//! **Progress** goes to stderr, a line per pair; a found game also prints a loud line to stderr at once, naming `n`,
//! `u`, `V` and the score.
//!
//! **The summary**, to stdout when every pair has run, from the lines of FILE for the pairs asked for (so a resumed
//! run summarises everything): the pairs by verdict, then for each game its `n`, `gap`, pairs, and how many are
//! `none`, `probed`, `timeout`, `found` and `error`, with the median and the largest seconds of its pairs that finished
//! (`none` or `found`), then the found games.
//!
//! **Exit status**: 2 if any pair was found (a counterexample), else 1 if any pair is an error, was left without a
//! line, or the arguments or files are wrong (the message goes to stderr), else 0.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use taxman::engine::WORKER_STACK_SIZE;
use taxman::game::replay;
use taxman::json::Json;
use taxman::sacrifice::{hunt, Hunted, PuncturedOptions};

const USAGE: &str = "usage: hunt --upper FILE --out FILE [--games 20] [--per-game K] [--list N:U,...] [--minutes 10] \
                     [--probe-above 900] [--probe-minutes 3] [--results FILE]... [--threads 4] [--jobs 1]";

/// The verdicts, in the order of the summary.
const VERDICTS: [&str; 5] = ["none", "probed", "timeout", "found", "error"];

// ---------------------------------------------------------------------------------------------------------
// The command line

/// What the command line asks for.
struct Args {
    upper: PathBuf,
    games: usize,
    per_game: Option<usize>,
    list: Option<Vec<(u32, u32)>>,
    /// The time limit of the search of a pair that is not a probe, and of a probe's; a pair of margin above
    /// `probe_above` (or of none) is a probe.
    minutes: f64,
    probe_above: u64,
    probe_minutes: f64,
    /// The solver's results files whose recorded games seed the start.
    results: Vec<PathBuf>,
    threads: usize,
    jobs: usize,
    out: PathBuf,
}

/// The list `N:U,N:U,...`, each pair once, in the order given.
fn parse_list(text: &str) -> Result<Vec<(u32, u32)>, String> {
    let mut pairs = Vec::new();
    for item in text.split(',') {
        let (n, u) = item.split_once(':').ok_or_else(|| format!("{item:?} is not N:U\n{USAGE}"))?;
        let number = |s: &str| s.parse::<u32>().map_err(|_| format!("{item:?} is not N:U\n{USAGE}"));
        let pair = (number(n)?, number(u)?);
        if !pairs.contains(&pair) {
            pairs.push(pair);
        }
    }
    Ok(pairs)
}

fn parse_args(args: &[String]) -> Result<Args, String> {
    let (mut upper, mut out, mut list, mut results) = (None, None, None, Vec::new());
    let (mut games, mut per_game, mut threads, mut jobs) = (20, None, 4, 1);
    let (mut minutes, mut probe_above, mut probe_minutes) = (10.0, 900, 3.0);
    let mut flags = args.iter();
    while let Some(flag) = flags.next() {
        let value = flags.next().ok_or_else(|| format!("{flag} needs a value\n{USAGE}"))?;
        let count = |what: &str| -> Result<usize, String> {
            value.parse().ok().filter(|&k| k >= 1).ok_or_else(|| format!("{what} must be a number, at least 1"))
        };
        let positive = |what: &str| -> Result<f64, String> {
            let ok = |m: &f64| m.is_finite() && *m > 0.0;
            value.parse::<f64>().ok().filter(ok).ok_or_else(|| format!("{what} must be a positive number"))
        };
        match flag.as_str() {
            "--upper" => upper = Some(PathBuf::from(value)),
            "--out" => out = Some(PathBuf::from(value)),
            "--results" => results.push(PathBuf::from(value)),
            "--games" => games = count("--games")?,
            "--per-game" => per_game = Some(count("--per-game")?),
            "--threads" => threads = count("--threads")?,
            "--jobs" => jobs = count("--jobs")?,
            "--list" => list = Some(parse_list(value)?),
            "--minutes" => minutes = positive("--minutes")?,
            "--probe-minutes" => probe_minutes = positive("--probe-minutes")?,
            "--probe-above" => {
                probe_above = value.parse().map_err(|_| "--probe-above must be a whole number".to_owned())?;
            }
            _ => return Err(USAGE.to_owned()),
        }
    }
    let (Some(upper), Some(out)) = (upper, out) else { return Err(USAGE.to_owned()) };
    Ok(Args { upper, games, per_game, list, minutes, probe_above, probe_minutes, results, threads, jobs, out })
}

// ---------------------------------------------------------------------------------------------------------
// The pairs

/// A game of the `upper` file, with what the prover left open.
struct Game {
    n: u32,
    /// The recorded score `V`.
    score: u64,
    /// Whether the prover's bounds left a source open (`"needs":1`).
    needs: bool,
    gap: u64,
    open: Vec<Open>,
}

/// An open member of `U*`.
struct Open {
    u: u32,
    omega: u32,
    /// How far the prover's bound was from certifying it; `None` (`null`) is the largest.
    margin: Option<u64>,
}

/// A source to take off the board of a game: one search.
#[derive(Clone)]
struct Pair {
    n: u32,
    u: u32,
    omega: u32,
    margin: Option<u64>,
    /// The game's recorded score `V` and gap.
    score: u64,
    gap: u64,
}

impl Pair {
    /// Whether this is a probe: a margin above `probe_above`, or none (`null`, counted as the largest).
    fn is_probe(&self, probe_above: u64) -> bool {
        self.margin.is_none_or(|margin| margin > probe_above)
    }
}

/// The lines of the file `path` (a missing file has none), as JSON.  A last line that lacks its newline was cut short
/// by an interruption; with `repair` it is cut off the file, and it is ignored in any case.
fn read_lines(path: &Path, repair: bool) -> Result<Vec<Json>, String> {
    let at = |e: io::Error| format!("{}: {e}", path.display());
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(at(e)),
    };
    let complete = text.rfind('\n').map_or(0, |end| end + 1);
    if repair && complete < text.len() {
        OpenOptions::new().write(true).open(path).and_then(|file| file.set_len(complete as u64)).map_err(at)?;
    }
    let lines = text[..complete].lines().enumerate().filter(|(_, line)| !line.trim().is_empty());
    lines.map(|(i, line)| Json::parse(line).map_err(|e| format!("{} line {}: {e}", path.display(), i + 1))).collect()
}

/// The recorded game of each `n` in the solver's results files `paths` (a line with an `n` and its `moves`), the first
/// file with a game for an `n` winning.
fn recorded_games(paths: &[PathBuf]) -> Result<BTreeMap<u32, Vec<u32>>, String> {
    let mut games = BTreeMap::new();
    for path in paths {
        if !path.is_file() {
            return Err(format!("{}: no such file", path.display()));
        }
        for line in read_lines(path, false)? {
            let moves = line.get("moves").and_then(Json::as_u32_vec);
            if let (Some(n), Some(moves)) = (line.get("n").and_then(Json::as_u32), moves) {
                games.entry(n).or_insert(moves);
            }
        }
    }
    Ok(games)
}

/// The game of a line of the `upper` file.
fn game_of(line: &Json) -> Result<Game, String> {
    let n = line.get("n").and_then(Json::as_u32).ok_or("a line of the upper file has no n")?;
    let whose = |what: &str| format!("n={n}: {what}");
    let score = line.get("score").and_then(Json::as_u64).ok_or_else(|| whose("no score"))?;
    let mut open = Vec::new();
    for member in line.get("open").and_then(Json::as_arr).unwrap_or(&[]) {
        let number = |i: usize| member.at(i).and_then(Json::as_u32);
        let margin = match member.at(2) {
            None | Some(Json::Null) => None,
            Some(m) => Some(m.as_u64().ok_or_else(|| whose("an open member with a margin that is no number"))?),
        };
        let (u, omega) = number(0).zip(number(1)).ok_or_else(|| whose("an open member is not [u, omega, margin]"))?;
        open.push(Open { u, omega, margin });
    }
    Ok(Game {
        n,
        score,
        needs: line.get("needs").and_then(Json::as_u64) == Some(1),
        gap: line.get("gap").and_then(Json::as_u64).unwrap_or(0),
        open,
    })
}

/// The number of prime factors of `x` with multiplicity.
fn omega(mut x: u32) -> u32 {
    let mut count = 0;
    let mut p = 2;
    while p * p <= x {
        while x.is_multiple_of(p) {
            x /= p;
            count += 1;
        }
        p += 1;
    }
    count + u32::from(x > 1)
}

/// The pair `(game.n, u)`: its omega and margin are the file's if it is one of the game's open members.
fn pair_of(game: &Game, u: u32) -> Pair {
    let open = game.open.iter().find(|member| member.u == u);
    Pair {
        n: game.n,
        u,
        omega: open.map_or_else(|| omega(u), |member| member.omega),
        margin: open.and_then(|member| member.margin),
        score: game.score,
        gap: game.gap,
    }
}

/// The pairs to hunt, in the order of the work.
fn select(games: &BTreeMap<u32, Game>, args: &Args) -> Result<Vec<Pair>, String> {
    if let Some(list) = &args.list {
        return list
            .iter()
            .map(|&(n, u)| {
                let game = games.get(&n).ok_or_else(|| format!("{} has no line for n={n}", args.upper.display()))?;
                if 2 * u <= n || u > n {
                    return Err(format!("n={n} u={u}: u must be above n/2 and at most n"));
                }
                Ok(pair_of(game, u))
            })
            .collect();
    }
    let mut needing: Vec<&Game> = games.values().filter(|game| game.needs).collect();
    needing.sort_by_key(|game| (Reverse(game.gap), game.n));
    let mut pairs = Vec::new();
    for game in needing.into_iter().take(args.games) {
        let mut members: Vec<&Open> = game.open.iter().collect();
        members.sort_by_key(|member| (Reverse(member.margin.unwrap_or(u64::MAX)), member.u));
        pairs.extend(members.into_iter().take(args.per_game.unwrap_or(usize::MAX)).map(|m| pair_of(game, m.u)));
    }
    // Decreasing margin over all the games, so that a run cut short has done the informative pairs.
    pairs.sort_by_key(|pair| (Reverse(pair.margin.unwrap_or(u64::MAX)), pair.n, pair.u));
    Ok(pairs)
}

/// The `(n, u)` of a line of the output.
fn key_of(line: &Json) -> Option<(u32, u32)> {
    line.get("n").and_then(Json::as_u32).zip(line.get("u").and_then(Json::as_u32))
}

// ---------------------------------------------------------------------------------------------------------
// Hunting

/// `pair` hunted: its line, and the verdict.  Nothing here fails: a hunt that does is an `error` line.
fn hunt_pair(pair: &Pair, args: &Args, seeds: &BTreeMap<u32, Vec<u32>>) -> (&'static str, Json) {
    let probe = pair.is_probe(args.probe_above);
    let minutes = if probe { args.probe_minutes } else { args.minutes };
    let opts = PuncturedOptions {
        threads: args.threads,
        time_limit: Some(Duration::from_secs_f64(minutes * 60.0)),
        // Only a probe makes the start: a search that runs to its end finds every game above V by itself.
        start: probe,
        seed: seeds.get(&pair.n).cloned(),
        ..PuncturedOptions::default()
    };
    let start = Instant::now();
    let hunted = hunt(pair.n, pair.u, pair.score, &opts);
    let secs = (start.elapsed().as_secs_f64() * 1000.0).round() / 1000.0;

    // The verdict, the punctured board's `in_the_bag`, the score of the start, the nodes, and what a line of that
    // verdict adds.  A probe that found nothing is `probed` whether or not its search finished, never `none`.
    let (verdict, in_the_bag, start_score, nodes, extra) = match hunted {
        Err(message) => ("error", Json::Null, Json::Null, 0, vec![("error", Json::from(message))]),
        Ok(h) => {
            let finished = h.outcome == Hunted::Nothing;
            let (verdict, extra) = match h.outcome {
                Hunted::Nothing | Hunted::Timeout if probe => ("probed", vec![("finished", Json::from(finished))]),
                Hunted::Nothing => ("none", Vec::new()),
                Hunted::Timeout => ("timeout", Vec::new()),
                Hunted::Found { score, moves } => {
                    let game = [("game_score", Json::from(score)), ("moves", Json::array(moves.iter().copied()))];
                    // A game of the punctured board must be a game of the full board, with the same score.
                    let problem = match replay(pair.n, &moves) {
                        Ok(full) if full == score => None,
                        Ok(full) => Some(format!("the game scores {score}, but replays on the full board to {full}")),
                        Err(e) => Some(format!("the game does not replay on the full board: {e}")),
                    };
                    match problem {
                        None => {
                            eprintln!(
                                "!!! COUNTEREXAMPLE: n={} without {}: a game scores {score}, above the recorded {}",
                                pair.n, pair.u, pair.score
                            );
                            ("found", Vec::from(game))
                        }
                        Some(message) => ("error", [("error", Json::from(message))].into_iter().chain(game).collect()),
                    }
                }
            };
            let start_score = h.start_score.map_or(Json::Null, Json::from);
            (verdict, Json::from(h.in_the_bag), start_score, h.nodes, extra)
        }
    };
    let mut line = vec![
        ("n", Json::from(pair.n)),
        ("u", Json::from(pair.u)),
        ("omega", Json::from(pair.omega)),
        ("margin", pair.margin.map_or(Json::Null, Json::from)),
        ("score", Json::from(pair.score)),
        ("in_the_bag", in_the_bag),
        ("start_score", start_score),
        ("verdict", Json::from(verdict)),
        ("nodes", Json::from(nodes)),
        ("secs", Json::from(secs)),
        ("threads", Json::from(args.threads)),
        ("minutes", Json::from(minutes)),
    ];
    line.extend(extra);
    (verdict, Json::object(line))
}

/// Hunts the pairs of `todo` on `args.jobs` threads, each pair's line appended to `out` and flushed as it finishes.
fn hunt_all(todo: &[Pair], args: &Args, seeds: &BTreeMap<u32, Vec<u32>>, out: File) -> Result<(), String> {
    let (next, finished) = (AtomicUsize::new(0), AtomicUsize::new(0));
    let out = Mutex::new(out);
    let start = Instant::now();
    let work = || -> Result<(), String> {
        while let Some(pair) = todo.get(next.fetch_add(1, Ordering::Relaxed)) {
            let (verdict, line) = hunt_pair(pair, args, seeds);
            let written = {
                let mut file = out.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
                writeln!(file, "{line}").and_then(|()| file.flush())
            };
            written.map_err(|e| {
                next.store(todo.len(), Ordering::Relaxed); // no job starts another pair
                format!("writing a line: {e}")
            })?;
            let margin = pair.margin.map_or("null".to_owned(), |m| m.to_string());
            let nodes = line.get("nodes").and_then(Json::as_u64).unwrap_or(0);
            let secs = line.get("secs").and_then(Json::as_f64).unwrap_or(0.0);
            eprintln!(
                "{} of {} pairs: n={} u={} omega={} margin={margin}: {verdict}, {nodes} nodes, {secs:.1} s \
                 ({:.0} s in all)",
                finished.fetch_add(1, Ordering::Relaxed) + 1,
                todo.len(),
                pair.n,
                pair.u,
                pair.omega,
                start.elapsed().as_secs_f64(),
            );
        }
        Ok(())
    };
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for _ in 0..args.jobs.min(todo.len()) {
            let spawned = thread::Builder::new().stack_size(WORKER_STACK_SIZE).spawn_scoped(scope, work);
            handles.push(spawned.map_err(|e| format!("cannot start a job thread: {e}"))?);
        }
        let joined = handles.into_iter().map(|handle| handle.join().unwrap_or_else(|_| Err("a job panicked".into())));
        joined.collect()
    })
}

// ---------------------------------------------------------------------------------------------------------
// The summary

/// What the run found, for the exit status.
struct Totals {
    found: usize,
    errors: usize,
    /// Pairs asked for that have no line (a run that failed to write one).
    missing: usize,
}

/// The median of `values`, ascending, if any.
fn median(values: &[f64]) -> Option<f64> {
    let mid = values.len() / 2;
    match values.len() {
        0 => None,
        len if len % 2 == 1 => Some(values[mid]),
        _ => Some((values[mid - 1] + values[mid]) / 2.0),
    }
}

/// The summary of `pairs` from the lines of the output, and what it found.
fn summary(pairs: &[Pair], lines: &[Json]) -> (String, Totals) {
    let by_pair: BTreeMap<(u32, u32), &Json> = lines.iter().filter_map(|line| Some((key_of(line)?, line))).collect();
    let verdict_of = |pair: &Pair| by_pair.get(&(pair.n, pair.u)).and_then(|line| line.get("verdict")?.as_str());
    let missing = pairs.iter().filter(|pair| verdict_of(pair).is_none()).count();
    let count = |verdict: &str| pairs.iter().filter(|pair| verdict_of(pair) == Some(verdict)).count();

    let mut rows = Vec::new();
    let tally: Vec<String> = VERDICTS.iter().map(|verdict| format!("{verdict} {}", count(verdict))).collect();
    let left = if missing > 0 { format!(", no line {missing}") } else { String::new() };
    rows.push(format!("{} pairs: {}{left}", pairs.len(), tally.join(", ")));
    rows.push(format!(
        "{:>6} {:>6} {:>6} {:>6} {:>7} {:>8} {:>6} {:>6} {:>10} {:>10}",
        "n", "gap", "pairs", "none", "probed", "timeout", "found", "error", "median s", "max s"
    ));
    // The games by gap, the largest first, however the pairs are ordered.
    let mut order: Vec<(Reverse<u64>, u32)> = pairs.iter().map(|pair| (Reverse(pair.gap), pair.n)).collect();
    order.sort_unstable();
    order.dedup();
    for (_, n) in order {
        let ours: Vec<&Pair> = pairs.iter().filter(|pair| pair.n == n).collect();
        let tally = |verdict: &str| ours.iter().filter(|pair| verdict_of(pair) == Some(verdict)).count();
        let mut seconds: Vec<f64> = (ours.iter())
            .filter(|pair| matches!(verdict_of(pair), Some("none" | "found")))
            .filter_map(|pair| by_pair.get(&(pair.n, pair.u))?.get("secs")?.as_f64())
            .collect();
        seconds.sort_by(f64::total_cmp);
        let seconds_text = |value: Option<f64>| value.map_or("-".to_owned(), |s| format!("{s:.1}"));
        rows.push(format!(
            "{n:>6} {:>6} {:>6} {:>6} {:>7} {:>8} {:>6} {:>6} {:>10} {:>10}",
            ours[0].gap,
            ours.len(),
            tally("none"),
            tally("probed"),
            tally("timeout"),
            tally("found"),
            tally("error"),
            seconds_text(median(&seconds)),
            seconds_text(seconds.last().copied()),
        ));
    }
    for pair in pairs.iter().filter(|pair| verdict_of(pair) == Some("found")) {
        let score = by_pair[&(pair.n, pair.u)].get("game_score").map_or("?".to_owned(), Json::to_string);
        let (n, u, recorded) = (pair.n, pair.u, pair.score);
        rows.push(format!("FOUND: n={n} without {u}: a game scores {score}, above the recorded {recorded}"));
    }
    for pair in pairs.iter().filter(|pair| verdict_of(pair) == Some("error")) {
        let message = by_pair[&(pair.n, pair.u)].get("error").and_then(Json::as_str).unwrap_or("?");
        rows.push(format!("ERROR: n={} u={}: {message}", pair.n, pair.u));
    }
    (rows.join("\n"), Totals { found: count("found"), errors: count("error"), missing })
}

// ---------------------------------------------------------------------------------------------------------
// Running

/// Hunts what `args` asks for and returns what it found.
fn run(args: &Args) -> Result<Totals, String> {
    if !args.upper.is_file() {
        return Err(format!("{}: no such file", args.upper.display()));
    }
    let mut games = BTreeMap::new();
    for line in read_lines(&args.upper, false)? {
        let game = game_of(&line)?;
        games.insert(game.n, game);
    }
    if games.is_empty() {
        return Err(format!("{} holds no games", args.upper.display()));
    }
    let pairs = select(&games, args)?;
    let done: BTreeSet<(u32, u32)> = read_lines(&args.out, true)?.iter().filter_map(key_of).collect();
    let todo: Vec<Pair> = pairs.iter().filter(|pair| !done.contains(&(pair.n, pair.u))).cloned().collect();
    let seeds = recorded_games(&args.results)?;
    let probes = todo.iter().filter(|pair| pair.is_probe(args.probe_above)).count();
    eprintln!(
        "{} pairs of {} games; {} are in {} already; {} to do ({probes} probes of {} minute(s), the rest {} minute(s) \
         each), {} thread(s) each on {} job(s); {} recorded game(s) to seed the start",
        pairs.len(),
        pairs.iter().map(|pair| pair.n).collect::<BTreeSet<_>>().len(),
        pairs.len() - todo.len(),
        args.out.display(),
        todo.len(),
        args.probe_minutes,
        args.minutes,
        args.threads,
        args.jobs,
        seeds.len(),
    );
    let out = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&args.out)
        .map_err(|e| format!("{}: {e}", args.out.display()))?;
    hunt_all(&todo, args, &seeds, out)?;

    let (text, totals) = summary(&pairs, &read_lines(&args.out, false)?);
    println!("{text}");
    Ok(totals)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&args) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("{message}");
            return ExitCode::from(1);
        }
    };
    match run(&args) {
        Ok(totals) if totals.found > 0 => ExitCode::from(2),
        Ok(totals) if totals.errors > 0 || totals.missing > 0 => ExitCode::from(1),
        Ok(_) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(1)
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Tests (`cargo t --release --example hunt`)

#[cfg(test)]
mod tests {
    use super::*;
    use taxman::brute::{self, Rule};

    /// The arguments of a command line that names the two required files and then `more`.
    fn args_with(more: &[&str]) -> Result<Args, String> {
        let words = ["--upper", "upper.jsonl", "--out", "out.jsonl"].iter().chain(more);
        parse_args(&words.map(|word| (*word).to_owned()).collect::<Vec<_>>())
    }

    fn open(u: u32, margin: Option<u64>) -> Open {
        Open { u, omega: 3, margin }
    }

    fn game(n: u32, gap: u64, open: Vec<Open>) -> Game {
        Game { n, score: 1000 + u64::from(n), needs: true, gap, open }
    }

    fn pair(n: u32, u: u32, margin: Option<u64>) -> Pair {
        Pair { n, u, omega: 3, margin, score: 1000, gap: 0 }
    }

    #[test]
    fn the_defaults_are_the_policy_of_the_cloud_run() {
        let args = args_with(&[]).unwrap();
        assert_eq!((args.minutes, args.probe_above, args.probe_minutes), (10.0, 900, 3.0));
        let args = args_with(&["--minutes", "0.5", "--probe-above", "500", "--probe-minutes", "1.5"]).unwrap();
        assert_eq!((args.minutes, args.probe_above, args.probe_minutes), (0.5, 500, 1.5));
        assert!(args_with(&["--probe-minutes", "0"]).is_err() && args_with(&["--probe-above", "-1"]).is_err());
    }

    /// A pair is a probe when its margin is above the limit, or unknown (counted as the largest).
    #[test]
    fn a_probe_is_a_margin_above_the_limit_or_none() {
        assert!(!pair(1, 1, Some(0)).is_probe(900) && !pair(1, 1, Some(900)).is_probe(900));
        assert!(pair(1, 1, Some(901)).is_probe(900) && pair(1, 1, None).is_probe(900));
        assert!(!pair(1, 1, Some(901)).is_probe(1000));
    }

    /// The work goes in decreasing margin over all the games, a null margin first, ties by n and then u; `--per-game`
    /// keeps a game's largest margins; `--list` keeps the order given.
    #[test]
    fn the_pairs_run_in_decreasing_margin() {
        let games: BTreeMap<u32, Game> = [
            game(10, 300, vec![open(7, Some(100)), open(8, Some(950)), open(9, Some(400))]),
            game(20, 200, vec![open(15, Some(950)), open(16, None), open(17, Some(100))]),
            game(30, 100, vec![open(25, Some(5000))]),
        ]
        .into_iter()
        .map(|game| (game.n, game))
        .collect();
        let order = |args: &Args| select(&games, args).unwrap().iter().map(|p| (p.n, p.u)).collect::<Vec<_>>();
        let all = args_with(&[]).unwrap();
        assert_eq!(order(&all), [(20, 16), (30, 25), (10, 8), (20, 15), (10, 9), (10, 7), (20, 17)]);
        let two_games = args_with(&["--games", "2"]).unwrap();
        assert_eq!(order(&two_games), [(20, 16), (10, 8), (20, 15), (10, 9), (10, 7), (20, 17)]);
        let one_each = args_with(&["--per-game", "1"]).unwrap();
        assert_eq!(order(&one_each), [(20, 16), (30, 25), (10, 8)]);
        let list = args_with(&["--list", "10:7,30:25,20:16"]).unwrap();
        assert_eq!(order(&list), [(10, 7), (30, 25), (20, 16)]);
    }

    /// A probe that finds nothing is `probed`, never `none`, whether or not its search finished, and a pair that is not
    /// a probe is `none` when its search finishes; either finds the best game one below the claim.  Only a probe makes
    /// the start, which never reaches more than the best game.  The board 30 without 17 has its best game by the brute
    /// force.
    #[test]
    fn a_probe_that_finds_nothing_is_probed_never_none() {
        let (n, u) = (30, 17);
        let best = brute::solve(n, &[u], Rule::MaximalFactor).score;
        let args = args_with(&["--threads", "1"]).unwrap();
        let hunted = |claim: u64, margin: u64| {
            let claimed = Pair { score: claim, ..pair(n, u, Some(margin)) };
            hunt_pair(&claimed, &args, &BTreeMap::new())
        };
        let (verdict, line) = hunted(best, 10);
        assert_eq!((verdict, line.get("minutes").and_then(Json::as_f64)), ("none", Some(10.0)));
        assert!(line.get("start_score").is_some_and(Json::is_null) && line.get("finished").is_none(), "{line}");

        let (verdict, line) = hunted(best, 5000);
        assert_eq!((verdict, line.get("minutes").and_then(Json::as_f64)), ("probed", Some(3.0)));
        assert_eq!(line.get("finished").and_then(Json::as_bool), Some(true), "{line}");
        assert!(line.get("start_score").and_then(Json::as_u64).is_some_and(|score| score <= best), "{line}");

        for margin in [10, 5000] {
            let (verdict, line) = hunted(best - 1, margin);
            assert_eq!(verdict, "found", "margin {margin}");
            assert_eq!(line.get("game_score").and_then(Json::as_u64), Some(best), "margin {margin}");
        }
    }

    /// The recorded games of the results files seed the start, the first file with a game for an `n` winning, and a
    /// file that is not there is an error.
    #[test]
    fn recorded_games_come_from_the_results_files_first_file_first() {
        let dir = std::env::temp_dir().join(format!("hunt_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let (first, second) = (dir.join("first.jsonl"), dir.join("second.jsonl"));
        fs::write(&first, "{\"n\":5,\"score\":9,\"moves\":[5,4]}\n{\"n\":6,\"score\":9,\"nothing\":1}\n").unwrap();
        fs::write(&second, "{\"n\":5,\"score\":8,\"moves\":[3]}\n{\"n\":7,\"score\":7,\"moves\":[7]}\n").unwrap();
        let games = recorded_games(&[first.clone(), second.clone()]).unwrap();
        assert_eq!(games, BTreeMap::from([(5, vec![5, 4]), (7, vec![7])]));
        assert!(recorded_games(&[dir.join("missing.jsonl")]).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    /// The summary counts the probes in a column of their own.
    #[test]
    fn the_summary_counts_probed_pairs() {
        let pairs = [pair(10, 7, Some(100)), pair(10, 8, Some(950))];
        let lines: Vec<Json> = [
            r#"{"n":10,"u":7,"verdict":"none","secs":1.5}"#,
            r#"{"n":10,"u":8,"verdict":"probed","secs":180.0,"finished":false}"#,
        ]
        .iter()
        .map(|line| Json::parse(line).unwrap())
        .collect();
        let (text, totals) = summary(&pairs, &lines);
        assert!(text.starts_with("2 pairs: none 1, probed 1, timeout 0, found 0, error 0"), "{text}");
        assert_eq!((totals.found, totals.errors, totals.missing), (0, 0, 0));
    }
}
