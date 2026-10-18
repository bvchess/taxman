//! The tuning benchmark's solver: work units of searches at the end of the chain, each solved to completion on one
//! thread.
//!
//! The search has settings (the tactics of `taxman solve --tactic`, and more to come) that were tuned on games near n =
//! 1000, and the hardest games (n above 3400, billions of nodes) may want other values.  A setting's effect on the tree
//! can only be measured by searches that run to completion, and one of the hardest games is far too big for that.  Such
//! a search is a deque of *units*, each the root of a subtree (a node: its promotions and its candidates,
//! `search::TaxUnit`), and a unit's subtree is a search of its own, small enough to finish.  This program takes units
//! from the checkpoint of such a search and solves them, one thread to a unit: with one thread the search is exactly
//! deterministic, so the nodes of two settings compare without noise.
//!
//! ```text
//! cargo run --release --example tune -- units GAME CKPT [--known FILE] [--sample K] [--seed S] [--out FILE]
//! cargo run --release --example tune -- solve GAME [--known FILE] [--units FILE] [--ids I,J,...]
//!     [--tactic NAME=VALUE]... [--node-limit L]
//! ```
//!
//! `units` reads the units a checkpoint of game GAME holds (the engine's deque from its front, the shallowest and
//! largest subtrees, to its back, then the units its workers held when it stopped), samples K of them at random
//! (uniformly: Algorithm R with the generator seeded by S; all of them if there are fewer; default 100 and seed 1), and
//! writes them to FILE as jsonl: a line `{"kind":"game", ...}` with what the game is (its candidates, the checkpoint's
//! incumbent and node count, the known optimum if `--known` has it), then a line `{"kind":"unit", ...}` for each unit
//! with its `id` (`GAME-INDEX`, INDEX its place in the checkpoint), its depth (the number of its promotions), the
//! number of its candidates, the sum of its promotions, the GF(2) bound of the node's completions (`bound`), what they
//! need to beat the incumbent (`need`: the known optimum's promotion sum if there is one, else the checkpoint's
//! incumbent's, less the node's own sum, plus one) and `gap` = bound − need, and both sets in hex.  The gap is what a
//! unit's size follows (a tree grows by some factor with every step of it), and costs a bound to find: `make_suite.py`
//! picks the units to solve by it.  A summary of all the checkpoint's units (the spread of depths and of numbers of
//! candidates) goes to stderr; without `--out` that is all.
//!
//! `solve` solves units of GAME to completion: each by an engine run of its own (one thread, fresh nogood tables),
//! so that a unit's result does not depend on the others solved with it.  The incumbent a subtree starts from is the
//! known optimum of the game if `--known` has it (then the subtree is *proved*: the search only has to show that
//! nothing in it beats the optimum), else the checkpoint's incumbent, recorded in the units file.  The unit ids are
//! those of the file (all the game's units if there is no `--ids`); `root` (or `GAME-root`) is the whole game, the
//! root unit with all candidates, for which no units file is needed (but the known optimum is: a whole game is a
//! proof).  `--tactic` takes the names and values of `taxman solve --tactic` (the search tactics only), so a new
//! tactic needs no change here, and `--node-limit L` stops a unit after about L nodes (the engine counts every 1,024),
//! which then reports `completed: false`.
//!
//! One jsonl line a unit: `game`, `id`, `depth`, `cands`, `nodes` (the engine's steps), `cpu_s` (the CPU time of the
//! worker thread, from the start of the unit's searcher to the end of its run: the setup of the game, the same for
//! every unit, is not in it), `wall_s`, `completed`, `best` (the best promotion sum found, the starting incumbent's if
//! nothing beat it), `incumbent` (the starting one), `proved`, `opt` (the known optimum or null), `tactics` (the
//! changes from the defaults) and `stats` (the searchers' counters, `search::stat`).
//!
//! Which `best` a subtree finds does not depend on the settings, except `symmetry` where the incumbent is not the
//! optimum: the rule keeps only the completions that keep the root's symmetric pairs' inequalities, and a subtree below
//! the root can hold a heavier completion that breaks one (heuristics/THEORY.md, "Pruning with the inequalities").
//! With the optimum as the incumbent nothing beats it whatever the settings, so a different `best` there is a bug.

#[allow(dead_code)] // this reads and writes hex sets only; the probe reads the nodes fixtures too
#[path = "common/fixtures.rs"]
mod fixtures;
#[path = "common/known.rs"]
mod known;
#[path = "common/rng.rs"]
mod rng;

use fixtures::{from_hex, to_hex};
use known::{known_incumbent, survivors};
use rng::Rng;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::{self, Ctx, EngineConfig, Searcher, Start, Stats, Unit};
use taxman::game::Optimal;
use taxman::gf2::Gf2Graph;
use taxman::json::Json;
use taxman::matchbound::MatchGraph;
use taxman::search::{self, stat, NodeBound, SearchSpace, SearchTactics, TaxSearcher, TaxUnit};
use taxman::tactics::Tactics;

// ---------------------------------------------------------------------------------------------------------
// CPU time of a thread

/// The CPU time this thread has used so far, in nanoseconds: what the unit costs, whatever else the machine does.
/// (Without a clock for it, the elapsed time since the first call.)
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn thread_cpu_ns() -> u64 {
    #[repr(C)]
    struct Timespec {
        seconds: i64,
        nanoseconds: i64,
    }
    unsafe extern "C" {
        fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
    }
    // CLOCK_THREAD_CPUTIME_ID: 3 on Linux, 16 on macOS.
    const THREAD_CPUTIME: i32 = if cfg!(target_os = "linux") { 3 } else { 16 };
    let mut time = Timespec { seconds: 0, nanoseconds: 0 };
    // SAFETY: `time` is writable and has the layout of `struct timespec` (two `long`s) on 64-bit Linux and macOS.
    let rc = unsafe { clock_gettime(THREAD_CPUTIME, &mut time) };
    assert_eq!(rc, 0, "clock_gettime failed");
    (time.seconds as u64) * 1_000_000_000 + time.nanoseconds as u64
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn thread_cpu_ns() -> u64 {
    use std::sync::OnceLock;
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_nanos() as u64
}

/// A searcher that adds the CPU time of its thread to `cpu`: its making, and each unit it runs.
struct Timed<'a, 's, const W: usize> {
    inner: TaxSearcher<'s, W>,
    cpu: &'a AtomicU64,
}

impl<'a, 's, const W: usize> Timed<'a, 's, W> {
    fn new(space: &'s SearchSpace<'s, W>, cpu: &'a AtomicU64) -> Self {
        let began = thread_cpu_ns();
        let inner = TaxSearcher::new(space);
        cpu.fetch_add(thread_cpu_ns() - began, Ordering::Relaxed);
        Timed { inner, cpu }
    }
}

impl<const W: usize> Searcher<TaxUnit<W>> for Timed<'_, '_, W> {
    fn run(&mut self, unit: TaxUnit<W>, ctx: &mut Ctx<'_, TaxUnit<W>>) {
        let began = thread_cpu_ns();
        self.inner.run(unit, ctx);
        self.cpu.fetch_add(thread_cpu_ns() - began, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------------------------------------
// Solving a unit

/// What solving one unit gave.
#[derive(Clone, Debug)]
struct Solved {
    nodes: u64,
    cpu_ns: u64,
    wall_s: f64,
    completed: bool,
    best: u64,
    stats: Stats,
}

/// Solves the subtree of `unit` on one thread, from the incumbent `start` (a promotion set, worth its sum), with a
/// fresh searcher, so that the result is that of the unit alone: the same every time.  Stops after about `node_limit`
/// nodes.
fn solve_unit<const W: usize>(
    space: &SearchSpace<'_, W>,
    unit: TaxUnit<W>,
    start: &Bits<W>,
    node_limit: Option<u64>,
) -> Result<Solved, String> {
    let cfg = EngineConfig { threads: 1, node_limit, progress: None, ..EngineConfig::default() };
    let begin = Start::Fresh { units: vec![unit], best: start.sum(), payload: search::payload_of(start) };
    let cpu = AtomicU64::new(0);
    let t0 = Instant::now();
    let outcome =
        engine::run(&cfg, begin, b"tune", |_| Timed::new(space, &cpu)).map_err(|e| format!("the engine: {e}"))?;
    Ok(Solved {
        nodes: outcome.stats.nodes,
        cpu_ns: cpu.load(Ordering::Relaxed),
        wall_s: t0.elapsed().as_secs_f64(),
        completed: outcome.completed,
        best: outcome.best,
        stats: outcome.stats,
    })
}

/// The searchers' counters by name.
fn stats_json(stats: &Stats) -> Json {
    const NAMES: [(&str, usize); 8] = [
        ("promote_attempts", stat::PROMOTE_ATTEMPTS),
        ("promotions", stat::PROMOTIONS),
        ("children", stat::CHILDREN),
        ("match_bounds", stat::MATCH_BOUNDS),
        ("gf2_solves", stat::GF2_SOLVES),
        ("gf2_augmentations", stat::GF2_AUGMENTATIONS),
        ("gf2_strong_solves", stat::GF2_STRONG_SOLVES),
        ("gf2_strong_augmentations", stat::GF2_STRONG_AUGMENTATIONS),
    ];
    Json::object(NAMES.map(|(name, i)| (name, Json::from(stats.extra[i]))))
}

// ---------------------------------------------------------------------------------------------------------
// The units file

/// A unit of the file: its id (`GAME-INDEX`), its depth and the unit.
struct Row<const W: usize> {
    id: String,
    depth: u32,
    unit: TaxUnit<W>,
}

/// The `game` line of a units file: what the game is, so that units are solved only for the game they came from.
struct GameLine {
    candidates: String,
    /// The promotion set of the checkpoint's incumbent, in hex, and its sum.
    incumbent: Option<(String, u64)>,
}

fn field<'j>(line: &'j Json, name: &str) -> Result<&'j Json, String> {
    line.get(name).ok_or_else(|| format!("no \"{name}\""))
}

fn str_field<'j>(line: &'j Json, name: &str) -> Result<&'j str, String> {
    field(line, name)?.as_str().ok_or_else(|| format!("\"{name}\" is not a string"))
}

/// The lines of the units file `path` that are about `game`: its `game` line, if any, and its units, in file order.
fn read_units<const W: usize>(path: &Path, game: u32) -> Result<(Option<GameLine>, Vec<Row<W>>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (mut header, mut rows) = (None, Vec::new());
    for (i, line) in text.lines().enumerate().filter(|(_, line)| !line.trim().is_empty()) {
        let at = |e: String| format!("{}: line {}: {e}", path.display(), i + 1);
        let json = Json::parse(line).map_err(|e| at(e.to_string()))?;
        let kind = str_field(&json, "kind").map_err(at)?;
        // The suite file has other lines (its own, and the whole games') than the game's and the units'.
        if !matches!(kind, "game" | "unit") || field(&json, "game").map_err(at)?.as_u32() != Some(game) {
            continue;
        }
        match kind {
            "game" => {
                let incumbent = match (json.get("incumbent"), json.get("incumbent_sum")) {
                    (Some(Json::Str(set)), Some(sum)) => {
                        Some((set.clone(), sum.as_u64().ok_or_else(|| at("bad sum".into()))?))
                    }
                    _ => None,
                };
                header =
                    Some(GameLine { candidates: str_field(&json, "candidates").map_err(at)?.to_owned(), incumbent });
            }
            "unit" => {
                let sets = |name| from_hex(str_field(&json, name)?);
                let unit = TaxUnit { promoted: sets("promoted").map_err(at)?, cands: sets("cands").map_err(at)? };
                let depth = field(&json, "depth").map_err(at)?.as_u32().ok_or_else(|| at("bad depth".into()))?;
                rows.push(Row { id: str_field(&json, "id").map_err(at)?.to_owned(), depth, unit });
            }
            _ => {}
        }
    }
    Ok((header, rows))
}

// ---------------------------------------------------------------------------------------------------------
// Sampling the units of a checkpoint

/// A uniform sample of `capacity` of the items offered (Algorithm R), each kept with its place in the sequence.
struct Reservoir<T> {
    capacity: usize,
    seen: u64,
    kept: Vec<(u64, T)>,
    rng: Rng,
}

impl<T> Reservoir<T> {
    fn new(capacity: usize, seed: u64) -> Self {
        Reservoir { capacity, seen: 0, kept: Vec::new(), rng: Rng(seed) }
    }

    /// Offers the next item; `make` builds it, and is called only if the item is kept.
    fn offer(&mut self, make: impl FnOnce() -> T) {
        let index = self.seen;
        self.seen += 1;
        if self.kept.len() < self.capacity {
            self.kept.push((index, make()));
        } else if self.capacity > 0 {
            let slot = self.rng.next() % (index + 1);
            if (slot as usize) < self.capacity {
                self.kept[slot as usize] = (index, make());
            }
        }
    }

    /// The sample, in the order the items came.
    fn into_sorted(mut self) -> Vec<(u64, T)> {
        self.kept.sort_by_key(|(index, _)| *index);
        self.kept
    }
}

/// The spread of a list of counts, as a line: smallest, quartiles and largest.
fn spread(counts: &mut [u32]) -> String {
    if counts.is_empty() {
        return "none".into();
    }
    counts.sort_unstable();
    let at = |q: usize| counts[(counts.len() - 1) * q / 100];
    format!("min {}, p25 {}, median {}, p75 {}, max {}", at(0), at(25), at(50), at(75), at(100))
}

/// The units of the checkpoint `ckpt` of game `game`, sampled, as the lines of a units file.
fn sample_units<const W: usize>(
    game: u32,
    ckpt: &Path,
    optimal: &Optimal,
    sample: usize,
    seed: u64,
) -> Result<Vec<Json>, String> {
    let board = Board::<W>::for_n(game);
    let prepared = board.prepare_for_resume(ckpt)?;
    let mut reservoir = Reservoir::new(sample, seed);
    let (mut depths, mut cands) = (Vec::new(), Vec::new());
    let head = engine::scan_checkpoint(ckpt, |bytes| {
        let unit = TaxUnit::<W>::decode(bytes);
        depths.push(unit.promoted.len());
        cands.push(unit.cands.len());
        reservoir.offer(|| unit);
        Ok(())
    })
    .map_err(|e| format!("{}: {e}", ckpt.display()))?;
    let total = reservoir.seen;
    eprintln!("{} units in {}: {} nodes searched, incumbent {}", total, ckpt.display(), head.stats.nodes, head.best);
    eprintln!("  depth (promotions): {}", spread(&mut depths));
    eprintln!("  candidates: {}", spread(&mut cands));
    let opt = optimal.score(game).and_then(|score| score.checked_sub(board.in_the_bag));
    let incumbent = search::set_of_payload::<W>(&head.payload);
    let mut lines = vec![Json::object([
        ("kind", Json::from("game")),
        ("game", Json::from(game)),
        ("width", Json::from(W)),
        ("candidates", Json::from(to_hex(&prepared.candidates))),
        ("in_the_bag", Json::from(board.in_the_bag)),
        ("max_total", Json::from(prepared.max_total)),
        ("opt", opt.map_or(Json::Null, Json::from)),
        ("incumbent", Json::from(to_hex(&incumbent))),
        ("incumbent_sum", Json::from(head.best)),
        ("checkpoint", Json::from(ckpt.file_name().map_or(String::new(), |s| s.to_string_lossy().into_owned()))),
        ("checkpoint_nodes", Json::from(head.stats.nodes)),
        ("units_in_checkpoint", Json::from(total)),
        ("sample", Json::from(sample)),
        ("seed", Json::from(seed)),
    ])];
    // The bound of each unit's node against what it needs, the best predictor of its size that costs nothing: the
    // trees of nodes with large gaps are the large ones.
    let graph = Gf2Graph::new(&prepared.model, &prepared.root, &prepared.candidates);
    let mut bound = NodeBound::new(&prepared, &graph);
    let best = opt.unwrap_or(head.best);
    for (index, unit) in reservoir.into_sorted() {
        let gap = bound.completions(&unit).map(|value| {
            let need = (best + 1).saturating_sub(unit.promoted.sum());
            (value, need)
        });
        lines.push(Json::object([
            ("kind", Json::from("unit")),
            ("game", Json::from(game)),
            ("id", Json::from(format!("{game}-{index}"))),
            ("index", Json::from(index)),
            ("depth", Json::from(unit.promoted.len())),
            ("n_cands", Json::from(unit.cands.len())),
            ("sum", Json::from(unit.promoted.sum())),
            ("bound", gap.map_or(Json::Null, |(value, _)| Json::from(value))),
            ("need", gap.map_or(Json::Null, |(_, need)| Json::from(need))),
            ("gap", gap.map_or(Json::Null, |(value, need)| Json::from(value as i64 - need as i64))),
            ("promoted", Json::from(to_hex(&unit.promoted))),
            ("cands", Json::from(to_hex(&unit.cands))),
        ]));
    }
    Ok(lines)
}

// ---------------------------------------------------------------------------------------------------------
// Solving the units of a game

/// How `solve` is asked to run.
struct SolveOptions {
    known: Option<PathBuf>,
    units: Option<PathBuf>,
    ids: Option<Vec<String>>,
    tactics: SearchTactics,
    node_limit: Option<u64>,
}

/// Solves the units of `game` that `o` asks for, printing a line each.
fn solve<const W: usize>(game: u32, o: &SolveOptions, optimal: &Optimal) -> Result<(), String> {
    let (header, rows) = match &o.units {
        Some(path) => read_units::<W>(path, game)?,
        None => (None, Vec::new()),
    };
    let board = Board::<W>::for_n(game);
    let prepared = board.prepare(o.tactics)?;
    if let Some(header) = &header {
        if header.candidates != to_hex(&prepared.candidates) {
            return Err(format!("the units of game {game} are for other candidates than this build makes"));
        }
    }
    let known = known_incumbent(&board, &prepared, optimal)?;
    let (start, opt) = match (&known, header.as_ref().and_then(|h| h.incumbent.as_ref())) {
        (Some((set, opt)), _) => (*set, Some(*opt)),
        (None, Some((hex, sum))) => {
            let set = survivors(&prepared, &from_hex(hex)?)?;
            if set.sum() != *sum {
                return Err(format!(
                    "the checkpoint's incumbent is worth {sum}, but {} of it can be promoted",
                    set.sum()
                ));
            }
            (set, None)
        }
        (None, None) => return Err(format!("no known game of {game}, and no checkpoint incumbent in the units")),
    };
    let wanted: Vec<Row<W>> = match &o.ids {
        None => rows,
        Some(ids) => {
            let mut chosen = Vec::new();
            for id in ids {
                if id == "root" || id.ends_with("-root") {
                    let unit = TaxUnit { promoted: Bits::EMPTY, cands: prepared.candidates };
                    chosen.push(Row { id: format!("{game}-root"), depth: 0, unit });
                } else {
                    let row =
                        rows.iter().find(|row| row.id == *id).ok_or_else(|| format!("no unit {id} in the file"))?;
                    chosen.push(Row { id: row.id.clone(), depth: row.depth, unit: row.unit });
                }
            }
            chosen
        }
    };
    let graph = MatchGraph::new(&prepared.model, &prepared.root);
    let space = SearchSpace::new(&prepared, &graph);
    let description = Tactics { search: o.tactics, ..Tactics::default() }.description();
    for Row { id, depth, unit } in wanted {
        let solved = solve_unit(&space, unit, &start, o.node_limit)?;
        let line = Json::object([
            ("game", Json::from(game)),
            ("id", Json::from(id)),
            ("depth", Json::from(depth)),
            ("cands", Json::from(unit.cands.len())),
            ("nodes", Json::from(solved.nodes)),
            ("cpu_s", Json::from(solved.cpu_ns as f64 / 1e9)),
            ("wall_s", Json::from(solved.wall_s)),
            ("completed", Json::from(solved.completed)),
            ("best", Json::from(solved.best)),
            ("incumbent", Json::from(start.sum())),
            ("proved", Json::from(known.is_some())),
            ("opt", opt.map_or(Json::Null, Json::from)),
            ("tactics", Json::from(description.clone())),
            ("node_limit", o.node_limit.map_or(Json::Null, Json::from)),
            ("stats", stats_json(&solved.stats)),
        ]);
        println!("{line}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// The command line

enum Command {
    Units { game: u32, ckpt: PathBuf, sample: usize, seed: u64, out: Option<PathBuf>, known: Option<PathBuf> },
    Solve { game: u32, options: SolveOptions },
}

fn parse() -> Result<Command, String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or("usage: tune units|solve GAME ... (see the top of examples/tune.rs)")?;
    let mut positional = Vec::new();
    let (mut known, mut units, mut ids, mut out, mut node_limit) = (None, None, None, None, None);
    let (mut sample, mut seed) = (100usize, 1u64);
    let mut tactics = Tactics::default();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        let number = |name: &str, text: String| text.parse::<u64>().map_err(|e| format!("{name} {text}: {e}"));
        match arg.as_str() {
            "--known" => known = Some(PathBuf::from(value("--known")?)),
            "--units" => units = Some(PathBuf::from(value("--units")?)),
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--ids" => ids = Some(value("--ids")?.split(',').map(str::to_owned).collect()),
            "--sample" => sample = number("--sample", value("--sample")?)? as usize,
            "--seed" => seed = number("--seed", value("--seed")?)?,
            "--node-limit" => node_limit = Some(number("--node-limit", value("--node-limit")?)?),
            "--tactic" => {
                // The solver's parser, so that the names and values are the same as for `taxman solve --tactic`.
                let spec = value("--tactic")?;
                tactics.set(&spec).map_err(|e| format!("--tactic: {e}"))?;
                if tactics != (Tactics { search: tactics.search, ..Tactics::default() }) {
                    return Err(format!("--tactic {spec}: a unit is only searched, so only search tactics apply"));
                }
            }
            other if other.starts_with("--") => return Err(format!("unknown option {other}")),
            _ => positional.push(arg),
        }
    }
    let game = |text: Option<&String>| -> Result<u32, String> {
        let text = text.ok_or("GAME is missing")?;
        text.parse().map_err(|e| format!("GAME {text}: {e}"))
    };
    match (command.as_str(), &positional[..]) {
        ("units", [g, ckpt]) => {
            Ok(Command::Units { game: game(Some(g))?, ckpt: ckpt.into(), sample, seed, out, known })
        }
        ("solve", [g]) => {
            let options = SolveOptions { known, units, ids, tactics: tactics.search, node_limit };
            Ok(Command::Solve { game: game(Some(g))?, options })
        }
        ("units", _) => Err("usage: tune units GAME CKPT [--known FILE] [--sample K] [--seed S] [--out FILE]".into()),
        ("solve", _) => {
            Err("usage: tune solve GAME [--known FILE] [--units FILE] [--ids I,J,...] [--tactic N=V]...".into())
        }
        (other, _) => Err(format!("unknown command {other:?}: units or solve")),
    }
}

/// The known games: the built-in table, and the completed games of `known`.
fn known_games(known: &Option<PathBuf>) -> Result<Optimal, String> {
    Optimal::built_in().with_results_files(known.as_slice())
}

fn run() -> Result<(), String> {
    match parse()? {
        Command::Units { game, ckpt, sample, seed, out, known } => {
            let optimal = known_games(&known)?;
            let lines = taxman::with_width!(game, W => sample_units::<W>(game, &ckpt, &optimal, sample, seed))?;
            if let Some(out) = out {
                let text: String = lines.iter().map(|line| format!("{line}\n")).collect();
                std::fs::write(&out, text).map_err(|e| format!("{}: {e}", out.display()))?;
                eprintln!("wrote {} units to {}", lines.len() - 1, out.display());
            }
            Ok(())
        }
        Command::Solve { game, options } => {
            let optimal = known_games(&options.known)?;
            taxman::with_width!(game, W => solve::<W>(game, &options, &optimal))
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use taxman::search::Prepared;

    /// A game whose proof takes several thousand steps with the default tactics (3,647 from the optimum): more than
    /// the 1,024 the engine counts by, so that a node limit can stop it and a checkpoint of it holds units.
    const GAME: u32 = 1215;

    /// The game set up as `solve` sets it up, with the optimum known (the built-in table has it).
    struct World<const W: usize> {
        prepared: Prepared<W>,
        board: Board<W>,
        /// The known optimum as the starting incumbent, and its promotion sum.
        start: Bits<W>,
        opt: u64,
    }

    fn world<const W: usize>() -> World<W> {
        let board = Board::<W>::for_n(GAME);
        let prepared = board.prepare(SearchTactics::default()).expect("prepare");
        let (start, opt) = known_incumbent(&board, &prepared, Optimal::built_in()).expect("known").expect("a game");
        World { prepared, board, start, opt }
    }

    fn root<const W: usize>(w: &World<W>) -> TaxUnit<W> {
        TaxUnit { promoted: Bits::EMPTY, cands: w.prepared.candidates }
    }

    /// A scratch directory of this test run.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tune-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    #[test]
    fn a_unit_is_solved_the_same_every_time() {
        fn check<const W: usize>() {
            let w = world::<W>();
            let graph = MatchGraph::new(&w.prepared.model, &w.prepared.root);
            let space = SearchSpace::new(&w.prepared, &graph);
            let a = solve_unit(&space, root(&w), &w.start, None).unwrap();
            let b = solve_unit(&space, root(&w), &w.start, None).unwrap();
            assert!(a.completed && a.nodes > 1_024, "{} nodes", a.nodes);
            assert_eq!((a.nodes, a.best, a.stats), (b.nodes, b.best, b.stats));
            assert_eq!(a.best, w.opt, "a proof from the optimum finds nothing better");
            assert!(a.cpu_ns > 0 && b.cpu_ns > 0);
        }
        taxman::with_width!(GAME, W => check::<W>());
    }

    /// The root unit is the whole search: the nodes of the one-thread proof that `search` makes, seeded with the
    /// optimum, are the root unit's.  (The checkable case: the subtree of a node below the root is not a part of the
    /// search that one can check in this way, because the searcher's nogoods, learned in the units before it, are not
    /// the fresh searcher's.)
    #[test]
    fn the_root_unit_is_the_whole_search() {
        fn check<const W: usize>() {
            let w = world::<W>();
            let graph = MatchGraph::new(&w.prepared.model, &w.prepared.root);
            let space = SearchSpace::new(&w.prepared, &graph);
            let unit = solve_unit(&space, root(&w), &w.start, None).unwrap();
            let seed = w.board.seed(Optimal::built_in().moves(GAME).expect("a game"));
            let cfg = EngineConfig { threads: 1, progress: None, ..EngineConfig::default() };
            let whole = search::search(&w.prepared, &cfg, Some(&seed), None, false).unwrap();
            assert!(whole.completed);
            assert_eq!((unit.nodes, unit.best), (whole.stats.nodes, whole.sum));
            assert_eq!(unit.stats, whole.stats);
        }
        taxman::with_width!(GAME, W => check::<W>());
    }

    #[test]
    fn a_node_limit_stops_a_unit_and_the_tactics_change_its_tree() {
        fn check<const W: usize>() {
            let w = world::<W>();
            let graph = MatchGraph::new(&w.prepared.model, &w.prepared.root);
            let space = SearchSpace::new(&w.prepared, &graph);
            let full = solve_unit(&space, root(&w), &w.start, None).unwrap();
            let limit = full.nodes / 2;
            let cut = solve_unit(&space, root(&w), &w.start, Some(limit)).unwrap();
            assert!(!cut.completed && cut.nodes >= limit && cut.nodes < full.nodes, "{} of {}", cut.nodes, full.nodes);
            // Another tactic is another tree; the answer is the same.
            let tactics = SearchTactics { symmetry: false, ..SearchTactics::default() };
            let other = w.board.prepare(tactics).unwrap();
            let other_graph = MatchGraph::new(&other.model, &other.root);
            let other_space = SearchSpace::new(&other, &other_graph);
            let off = solve_unit(&other_space, root(&w), &w.start, None).unwrap();
            assert!(off.completed && off.nodes != full.nodes);
            assert_eq!(off.best, full.best);
        }
        taxman::with_width!(GAME, W => check::<W>());
    }

    #[test]
    fn the_sample_is_uniform_ordered_and_seeded() {
        let sample = |capacity, seed, items: u64| {
            let mut r = Reservoir::new(capacity, seed);
            (0..items).for_each(|i| r.offer(|| i * 10));
            r.into_sorted()
        };
        let a = sample(5, 7, 1_000);
        assert_eq!(a.len(), 5);
        assert!(a.windows(2).all(|w| w[0].0 < w[1].0), "in order, distinct");
        assert!(a.iter().all(|&(index, item)| item == index * 10));
        assert_eq!(a, sample(5, 7, 1_000), "the same seed gives the same sample");
        assert_ne!(a, sample(5, 8, 1_000));
        assert_eq!(sample(5, 7, 3).len(), 3, "fewer items than the capacity: all");
        assert!(sample(0, 7, 3).is_empty());
        // Each of 10 items is in a sample of 3 about 30% of the time, over seeds.
        let mut counts = [0u32; 10];
        for seed in 0..2_000 {
            sample(3, seed, 10).iter().for_each(|&(index, _)| counts[index as usize] += 1);
        }
        assert!(counts.iter().all(|&c| (500..700).contains(&c)), "{counts:?}");
    }

    /// Units of a checkpoint: sampled, written, read back, and solved from the checkpoint's incumbent and from the
    /// optimum.
    #[test]
    fn units_of_a_checkpoint_are_sampled_written_read_and_solved() {
        fn check<const W: usize>() {
            let w = world::<W>();
            let dir = scratch("units");
            let ckpt = dir.join("game.ckpt");
            // A one-thread search of the game stopped after some steps leaves its deque in a checkpoint.
            let cfg = EngineConfig {
                threads: 1,
                node_limit: Some(2_048),
                progress: None,
                checkpoint: Some((ckpt.clone(), Duration::from_secs(1 << 20))),
                ..EngineConfig::default()
            };
            let stopped = search::search(&w.prepared, &cfg, None, None, false).unwrap();
            assert!(!stopped.completed, "the game must take more than 2,048 steps");
            let mut all = Vec::new();
            let head = engine::scan_checkpoint(&ckpt, |bytes| {
                all.push(TaxUnit::<W>::decode(bytes));
                Ok(())
            })
            .unwrap();
            assert!(all.len() > 3, "{} units", all.len());

            let lines = sample_units::<W>(GAME, &ckpt, Optimal::built_in(), 3, 5).unwrap();
            assert_eq!(lines.len(), 1 + 3);
            assert_eq!(lines[0].get("units_in_checkpoint").and_then(Json::as_u64), Some(all.len() as u64));
            assert_eq!(lines[0].get("incumbent_sum").and_then(Json::as_u64), Some(head.best));
            assert_eq!(lines[0].get("opt").and_then(Json::as_u64), Some(w.opt));
            let again = sample_units::<W>(GAME, &ckpt, Optimal::built_in(), 3, 5).unwrap();
            assert_eq!(lines, again, "the same seed gives the same units");
            let path = dir.join("units.jsonl");
            std::fs::write(&path, lines.iter().map(|line| format!("{line}\n")).collect::<String>()).unwrap();

            let (header, rows) = read_units::<W>(&path, GAME).unwrap();
            let header = header.expect("a game line");
            assert_eq!(header.candidates, to_hex(&w.prepared.candidates));
            assert_eq!(header.incumbent.as_ref().map(|(_, sum)| *sum), Some(head.best));
            assert_eq!(rows.len(), 3);
            assert!(read_units::<W>(&path, GAME + 1).unwrap().1.is_empty(), "other games' lines are not read");
            let graph = MatchGraph::new(&w.prepared.model, &w.prepared.root);
            let space = SearchSpace::new(&w.prepared, &graph);
            let checkpoint_start = survivors(&w.prepared, &from_hex(&header.incumbent.unwrap().0).unwrap()).unwrap();
            for row in &rows {
                let index: usize = row.id.rsplit('-').next().unwrap().parse().unwrap();
                assert_eq!(row.unit, all[index], "{} is that unit of the checkpoint", row.id);
                assert_eq!(row.depth, row.unit.promoted.len());
                // From the optimum the subtree is proved, and solved the same every time.
                let proof = solve_unit(&space, row.unit, &w.start, None).unwrap();
                let again = solve_unit(&space, row.unit, &w.start, None).unwrap();
                assert!(proof.completed && proof.best == w.opt);
                assert_eq!((proof.nodes, proof.stats), (again.nodes, again.stats));
                // From the checkpoint's incumbent, which is no better than the optimum, it finds at most the optimum.
                let search = solve_unit(&space, row.unit, &checkpoint_start, None).unwrap();
                assert!(search.completed && (checkpoint_start.sum()..=w.opt).contains(&search.best));
            }
            std::fs::remove_dir_all(&dir).unwrap();
        }
        taxman::with_width!(GAME, W => check::<W>());
    }
}
