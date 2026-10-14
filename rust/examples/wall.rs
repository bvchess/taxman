//! The wall regime: how the solver's costs grow with the board size `n` and with the size of a search tree.  What a
//! game of the chain costs is its search, which grows with the tree, plus everything around the search, which grows
//! with `n` alone (the tables, the frames, the model, the GF(2) matroids of the root, the replays); the commands
//! measure the two apart.  The study, its data and its conclusions are in docs/EXPERIMENTS.md, "The wall regime
//! (2026-10)".
//!
//! ```text
//! cargo run --release --example wall -- setup [--known FILE] [--repeat R] [--width W] N|A-B...
//! cargo run --release --example wall -- nodes [--known FILE] [--nodes K] [--seconds S] [--mode start|opt] [--delta D]
//!     [--width W] [--no-seed] [--tactic NAME=VALUE]... [--ckpt-dir DIR --every SECONDS] N...
//! cargo run --release --example wall -- grow [--known FILE] [--delta D] [--nodes K] [--seconds S] [--progress S]
//!     [--tactic NAME=VALUE]... [--ckpt-dir DIR --every SECONDS] [--threads T] [--cores C] [--mem-cap UNITS] N
//! cargo run --release --example wall -- start [--known FILE] N|A-B...
//! cargo run --release --example wall -- frames N|A-B...
//! ```
//!
//! Games are set up as `taxman solve` sets them up (`taxman::board::Board` at the set width `with_width!` picks for
//! `n`, the search tactics named as for `taxman solve --tactic`).  A board with nothing to promote prints
//! `{"n":N,"trivial":true}` and is skipped.  `--known FILE` (every command but `frames`) is a results file, such as
//! `results/strict.jsonl`, whose completed games extend the known games beyond optimal.json (as the ladder's
//! `--known` does), which is how a game gets its optimum, and the game of `n - 1` for its seed and ceiling.  A range
//! `A-B` stands for every `n` in it, none filtered out (`setup`, `start` and `frames`).  Each command prints one JSON
//! object per line on stdout; a mistake in the arguments is an error on stderr and exit status 2.
//!
//! * **setup**: what a game of the chain costs besides its search, piece by piece, each timed `--repeat` times
//!   (default 1) and reported as the fastest, for every `n` (primes too, `prime` says which).  The pieces: the
//!   number-theory tables (`tables_s`), the frames (`frames_s`), the game with no promotions that `Board::new` adds
//!   to them (`plain_s`) and the board as a whole (`board_s`); `prepare_s`, the search's model and greedy incumbent
//!   (`model_s` and `greedy_s` are its two phases); `problem_s`, the canonical search problem and its digest;
//!   `search_problem_s`, the same problem built from the board size alone, which is what the tactic `same_search`
//!   does for the earlier game and what the results check does for each repeat line and the game it repeats;
//!   `match_graph_s`, the matching graph; `improve_s`, the local search from the greedy set; `zero_search_s`, the
//!   search stopped before its first node (the search's own setup: matching graph, seed, dive, local search, the
//!   GF(2) graph, the engine started and stopped), whose result gives `start`, the incumbent a search starts from,
//!   and `capped`, whether the ceiling from the game of `n - 1` already ends the search; the root's GF(2) bound
//!   (below); `moves_s` and `replay_s`, the promotions made a game and the referee's replay of it (`replay_ok`);
//!   and `prime_path_s`, the prime shortcut's own work besides its replays.  Then the sizes (below), `opt` (the
//!   known optimum promotion sum, or null), `start`, `root_bound`, the gaps `gap_start` and `gap_opt` of the root's
//!   bound over `start` and over `opt`, and `max_rss` (the process's peak resident set, bytes).  `--width W` sets the
//!   game up at that set width instead of the one `with_width!` picks, as for `nodes`, to show what the width costs the
//!   setup.
//! * **nodes**: a one-thread search of each game, stopped after `--nodes K` steps (default 5000) or `--seconds S`,
//!   for the cost per node at the root-sized part of a tree.  `--mode start` (the default) is the solver's own
//!   search from its start incumbent (greedy set, seed from the game of `n - 1`, dive, local search; `--no-seed`
//!   drops the seed and the ceiling).  `--mode opt` is the proof of `matroid prove`: the engine's incumbent is the
//!   value `opt + delta` (`--delta`, default 0) with no set behind it, no seed and no ceiling, so it needs the known
//!   optimum.  `--width W` runs the search at a set width of the list the solver is built for, instead of the one
//!   `with_width!` picks, to show that the width changes the time and nothing else: the same search at several
//!   widths has to give the same nodes and GF(2) counters.  `--ckpt-dir DIR --every SECONDS` checkpoints to
//!   `DIR/wall_n<N>_w<W>.ckpt` as often as that, and reports the size of the last checkpoint (`ckpt_bytes`; the
//!   final one, as a node limit stops the run) before deleting it.  Each line reports the sizes, the root's GF(2)
//!   bound (computed after the search, apart from it), the outcome and the search's cost: wall seconds, the
//!   engine's busy time, the cycles and instructions of the process, the promotion and GF(2) counters, the cost
//!   per node and per augmentation, and the engine's counters for pauses, checkpoints and the queue.
//! * **grow**: the emulation of a large tree.  A search whose incumbent starts `--delta D` below a known optimum
//!   (`D <= 0` usually) behaves like a search with a larger gap, so it makes a much larger tree on a known game:
//!   the proof of `nodes --mode opt` with the incumbent at `opt + D`, on `--threads T` threads (default 1; `--cores
//!   C` lets at most C of them search at once), until it completes or `--nodes K` or `--seconds S` stops it.
//!   `--tactic` sets search tactics as for `nodes`; `gf2=off,lagrange=on` (the matching path, whose nodes are ten times
//!   cheaper and whose trees a hundred times larger) makes trees of ten million nodes in minutes, for what the engine
//!   and the nogood tables do in a large tree.  The engine prints its progress lines to stderr every `--progress S`
//!   seconds (default 2), and a monitor thread prints on stdout at the same pace
//!   `{"t":SECONDS,"max_rss":BYTES,"ckpt_bytes":SIZE,"spill_files":COUNT}` for the memory, the checkpoint and the spill
//!   directory.  With `--ckpt-dir DIR` the run checkpoints to `DIR/grow_n<N>.ckpt` every `--every` seconds and spills
//!   its queue to `DIR/spill` beyond `--mem-cap` units.  One summary line closes the run, with all the engine's
//!   counters.
//! * **start**: what the search's starting incumbent costs and gains, piece by piece, for each `n` that has a known
//!   game of `n - 1` (any other has the line `{"n":N,"no_previous":true}`).  `greedy` is the sum of the greedy set;
//!   `seed_kept` the sum of the seed that survives (the promotions of the game of `n - 1` that are candidates here,
//!   promoted one by one from the largest down, skipping the ones that fail) and `seed_kept_s` the time that took;
//!   `base` the sum of the better of the two, with `members` the number of promotions in it (the solver also tries a
//!   matching dive, which is private to the library and not reproduced here).  `ceiling` is the ceiling on the
//!   promotion sum that the game of `n - 1` gives, and `at_ceiling` says whether `base` already reaches it.  Then the
//!   local search from `base`: `singles` is the sum it finds when it takes out one member at a time, `pairs` the sum
//!   when it takes out two as well (the solver's call), each with its time in `singles_s` and `pairs_s`.  `opt` is the
//!   known optimum promotion sum, or null, and `max_rss` the process's peak resident set.
//! * **frames**: for boards beyond the widest set the solver supports (up to 8191, at 128 words): the frames the
//!   board builder makes (before `Model::build` reduces them, which keeps every frame's freedoms), the time it
//!   took, and the sizes the later stages would need: per frame the factors, moves and freedoms, `max_freedoms`
//!   and the `block_words` of the GF(2) bound's vectors it needs (`max_freedoms.div_ceil(64)`).
//!
//! **The sizes** (`setup`, `nodes` and `frames`): `frames` (the root's frame count), `candidates`, `max_total`
//! (the most promotions a set can have), `members` (the distinct numbers in the moves and factors of the root's
//! frames and in the candidates), `root_max` (the largest of them), `words_n` (`words_for(n)`), `width` (the words
//! per set the game ran at), `words_members`, `words_candidates` and `words_root_max` (the words a set of that
//! many numbers, or of numbers up to `root_max`, would need), and per frame the numbers of moves, factors and
//! freedoms (`frame_moves`, `frame_factors`, `frame_freedoms`; `moves`, `factors` and `freedoms` for `frames`).
//!
//! **The root's GF(2) bound** (`setup` and `nodes`): the time to build the root's matroids (`gf2_graph_s`), their
//! `elements`, per-frame `ranks` and `block_words`; the bound with no incumbent (`root_bound_s` for the reset and
//! the evaluation, `root_bound`, `root_chosen` the size of its heaviest set, `root_augmentations`); and the same
//! evaluation asked for the members forced by an incumbent of the start value (`root_split_s`, `root_forced`).

#[path = "common/counters.rs"]
mod counters;

use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use taxman::accelerate::solution_for_promotions;
use taxman::bits::{words_for, Bits};
use taxman::board::{search_problem, Board};
use taxman::engine::{self, Counters, EngineConfig, Start, Stats};
use taxman::frames::build_frames;
use taxman::game::{replay_finished, Optimal};
use taxman::gf2::{Gf2Bound, Gf2Graph};
use taxman::hives::Apiary;
use taxman::matchbound::MatchGraph;
use taxman::model::Scratch;
use taxman::numtheory::Tables;
use taxman::search::{search, stat, Neighbourhood, Prepared, SearchSpace, SearchTactics, TaxSearcher, TaxUnit};
use taxman::tactics::Tactics;

// ---------------------------------------------------------------------------------------------------------
// Arguments

const COMMANDS: [&str; 5] = ["setup", "nodes", "grow", "start", "frames"];

/// The options, with the commands each applies to (an option given to another command is an error).
const OPTIONS: [(&str, &[&str]); 15] = [
    ("--known", &["setup", "nodes", "grow", "start"]),
    ("--repeat", &["setup"]),
    ("--nodes", &["nodes", "grow"]),
    ("--seconds", &["nodes", "grow"]),
    ("--mode", &["nodes"]),
    ("--delta", &["nodes", "grow"]),
    ("--width", &["setup", "nodes"]),
    ("--no-seed", &["nodes"]),
    ("--tactic", &["nodes", "grow"]),
    ("--ckpt-dir", &["nodes", "grow"]),
    ("--every", &["nodes", "grow"]),
    ("--progress", &["grow"]),
    ("--threads", &["grow"]),
    ("--cores", &["grow"]),
    ("--mem-cap", &["grow"]),
];

/// The node limit of `nodes` when `--nodes` is not given.
const DEFAULT_NODES: u64 = 5000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// The solver's own search, from its start incumbent.
    Start,
    /// The proof of the known optimum (the incumbent a value).
    Opt,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Start => "start",
            Mode::Opt => "opt",
        }
    }
}

struct Options {
    command: String,
    /// A results file with more known games.
    known: Option<PathBuf>,
    repeat: usize,
    nodes: Option<u64>,
    seconds: Option<Duration>,
    mode: Mode,
    delta: Option<i64>,
    width: Option<usize>,
    no_seed: bool,
    tactics: SearchTactics,
    ckpt_dir: Option<PathBuf>,
    every: Option<Duration>,
    progress: Duration,
    threads: usize,
    cores: Option<usize>,
    mem_cap: Option<usize>,
    games: Vec<u32>,
}

fn number<T: std::str::FromStr>(name: &str, text: String) -> Result<T, String>
where
    T::Err: Display,
{
    text.parse().map_err(|e| format!("{name} {text}: {e}"))
}

/// A duration given in (fractional) seconds.
fn duration(name: &str, text: String) -> Result<Duration, String> {
    let seconds: f64 = number(name, text)?;
    Duration::try_from_secs_f64(seconds).map_err(|e| format!("{name} {seconds}: {e}"))
}

/// The board sizes of the arguments: numbers, and for `setup`, `start` and `frames` also ranges `A-B`.
fn games(command: &str, texts: &[String]) -> Result<Vec<u32>, String> {
    let ranges = matches!(command, "setup" | "start" | "frames");
    let mut games = Vec::new();
    for text in texts {
        let number = |part: &str| part.parse::<u32>().map_err(|e| format!("{text}: {e}"));
        match text.split_once('-') {
            Some((a, b)) if ranges => {
                let (a, b) = (number(a)?, number(b)?);
                if a > b {
                    return Err(format!("{text}: the range is empty"));
                }
                games.extend(a..=b);
            }
            _ => games.push(number(text)?),
        }
    }
    if games.is_empty() {
        return Err("no games given".into());
    }
    match games.iter().find(|&&n| n < 2) {
        Some(n) => Err(format!("n={n}: the boards start at 2")),
        None => Ok(games),
    }
}

fn parse() -> Result<Options, String> {
    let mut args = std::env::args().skip(1);
    let command = args.next().ok_or_else(|| format!("expected a command: {}", COMMANDS.join(", ")))?;
    if !COMMANDS.contains(&command.as_str()) {
        return Err(format!("unknown command {command:?}: {}", COMMANDS.join(", ")));
    }
    let mut o = Options {
        command,
        known: None,
        repeat: 1,
        nodes: None,
        seconds: None,
        mode: Mode::Start,
        delta: None,
        width: None,
        no_seed: false,
        tactics: SearchTactics::default(),
        ckpt_dir: None,
        every: None,
        progress: Duration::from_secs(2),
        threads: 1,
        cores: None,
        mem_cap: None,
        games: Vec::new(),
    };
    let mut texts = Vec::new();
    while let Some(arg) = args.next() {
        if arg.starts_with("--") {
            match OPTIONS.iter().find(|(name, _)| *name == arg) {
                None => return Err(format!("unknown option {arg}")),
                Some((_, commands)) if !commands.contains(&o.command.as_str()) => {
                    return Err(format!("{arg} does not apply to {}", o.command));
                }
                Some(_) => {}
            }
        }
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--known" => o.known = Some(value("--known")?.into()),
            "--repeat" => o.repeat = number::<usize>("--repeat", value("--repeat")?)?.max(1),
            "--nodes" => o.nodes = Some(number("--nodes", value("--nodes")?)?),
            "--seconds" => o.seconds = Some(duration("--seconds", value("--seconds")?)?),
            "--mode" => {
                o.mode = match value("--mode")?.as_str() {
                    "start" => Mode::Start,
                    "opt" => Mode::Opt,
                    other => return Err(format!("--mode {other}: start or opt")),
                }
            }
            "--delta" => o.delta = Some(number("--delta", value("--delta")?)?),
            "--width" => o.width = Some(number("--width", value("--width")?)?),
            "--no-seed" => o.no_seed = true,
            "--tactic" => {
                // The solver's parser, so that the names and values are those of `taxman solve --tactic`.
                let spec = value("--tactic")?;
                let mut tactics = Tactics { search: o.tactics, ..Tactics::default() };
                tactics.set(&spec).map_err(|e| format!("--tactic: {e}"))?;
                if tactics != (Tactics { search: tactics.search, ..Tactics::default() }) {
                    return Err(format!("--tactic {spec}: only search tactics apply"));
                }
                o.tactics = tactics.search;
            }
            "--ckpt-dir" => o.ckpt_dir = Some(value("--ckpt-dir")?.into()),
            "--every" => o.every = Some(duration("--every", value("--every")?)?),
            "--progress" => o.progress = duration("--progress", value("--progress")?)?,
            "--threads" => o.threads = number::<usize>("--threads", value("--threads")?)?.max(1),
            "--cores" => o.cores = Some(number("--cores", value("--cores")?)?),
            "--mem-cap" => o.mem_cap = Some(number("--mem-cap", value("--mem-cap")?)?),
            _ => texts.push(arg.clone()),
        }
    }
    if o.ckpt_dir.is_some() != o.every.is_some() {
        return Err("--ckpt-dir and --every go together".into());
    }
    if o.progress.is_zero() {
        return Err("--progress must be positive".into());
    }
    if o.command == "nodes" && o.mode == Mode::Start && o.delta.is_some() {
        return Err("--delta needs --mode opt".into());
    }
    o.games = games(&o.command, &texts)?;
    if o.command == "grow" && o.games.len() != 1 {
        return Err("grow takes one game".into());
    }
    Ok(o)
}

fn fail(error: &str) -> ! {
    eprintln!("error: {error}");
    std::process::exit(2);
}

fn beyond(n: u32) -> String {
    format!("n={n} is beyond the widest set the solver is built for")
}

/// The arms of a match on the set width `$width`: `$run::<w>($n, $o, $optimal)` for each listed width `w`, and an
/// error naming the widths for any other.
macro_rules! match_width {
    ($width:expr, $run:ident, $n:expr, $o:expr, $optimal:expr; $($w:literal)*) => {
        match $width {
            $($w => $run::<$w>($n, $o, $optimal),)*
            width => Err(format!("--width {width}: the widths are {:?}", [$($w),*])),
        }
    };
}

/// `$run` for game `$n` at the set width `--width` (`$o.width`) if it is given, and otherwise at the width that
/// `with_width!` picks for `$n`.  The widths `--width` may name are listed here, once, for `setup` and `nodes` alike.
macro_rules! at_width {
    ($run:ident, $n:expr, $o:expr, $optimal:expr) => {{
        let (n, o, optimal) = ($n, $o, $optimal);
        match o.width {
            None => taxman::with_width!(n, W => $run::<W>(n, o, optimal), else Err(beyond(n))),
            Some(width) if width < words_for(n) => {
                Err(format!("--width {width} is below the {} words that n={n} needs", words_for(n)))
            }
            Some(width) => match_width!(width, $run, n, o, optimal; 8 16 20 24 28 32 36 40 44 48 56 64),
        }
    }};
}

fn main() {
    let o = parse().unwrap_or_else(|e| fail(&e));
    let optimal = Optimal::built_in().with_results_files(o.known.as_slice()).unwrap_or_else(|e| fail(&e));
    for &n in &o.games {
        let done = match o.command.as_str() {
            "setup" => at_width!(setup, n, &o, &optimal),
            "nodes" => at_width!(nodes, n, &o, &optimal),
            "grow" => taxman::with_width!(n, W => grow::<W>(n, &o, &optimal), else Err(beyond(n))),
            "start" => taxman::with_width!(n, W => start::<W>(n, &o, &optimal), else Err(beyond(n))),
            _ => frames_at_width(n),
        };
        done.unwrap_or_else(|e| fail(&e));
    }
}

// ---------------------------------------------------------------------------------------------------------
// What the commands share

/// One JSON object under construction (`{"n":N,...}`): each method appends a field and returns the line, to chain.
struct Line(Vec<String>);

impl Line {
    fn new() -> Line {
        Line(Vec::new())
    }

    /// A line that starts with the board size.
    fn of(n: u32) -> Line {
        let mut line = Line::new();
        line.field("n", n);
        line
    }

    fn field(&mut self, name: &str, value: impl Display) -> &mut Line {
        self.0.push(format!("\"{name}\":{value}"));
        self
    }

    fn text(&mut self, name: &str, value: &str) -> &mut Line {
        self.field(name, format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"")))
    }

    /// A time in seconds.
    fn seconds(&mut self, name: &str, value: f64) -> &mut Line {
        self.field(name, format!("{value:.6}"))
    }

    /// The value, or `null`.
    fn maybe(&mut self, name: &str, value: Option<impl Display>) -> &mut Line {
        match value {
            Some(value) => self.field(name, value),
            None => self.field(name, "null"),
        }
    }

    /// `numerator / denominator`, or `null` for a zero denominator.
    fn ratio(&mut self, name: &str, numerator: f64, denominator: u64) -> &mut Line {
        let value = (denominator > 0).then(|| format!("{:.3}", numerator / denominator as f64));
        self.maybe(name, value)
    }

    fn list<T: Display>(&mut self, name: &str, values: impl IntoIterator<Item = T>) -> &mut Line {
        let items: Vec<String> = values.into_iter().map(|v| v.to_string()).collect();
        self.field(name, format!("[{}]", items.join(",")))
    }

    fn print(&self) {
        println!("{{{}}}", self.0.join(","));
    }
}

/// A game set up as the chain sets it up, with what is known of it.
struct Game<const W: usize> {
    board: Board<W>,
    p: Prepared<W>,
    /// The known optimum promotion sum (the known score minus `in_the_bag`).
    opt: Option<u64>,
    /// The promotions of the known game of `n - 1` that are candidates here.
    seed: Option<Bits<W>>,
    /// The ceiling on the promotion sum that the known game of `n - 1` gives.
    ceiling: Option<u64>,
}

impl<const W: usize> Game<W> {
    /// The game of `n`, or `None` (after printing `{"n":N,"trivial":true}`) for a board with nothing to promote.
    /// Without `with_seed` the game has neither a seed nor a ceiling.
    fn new(n: u32, tactics: SearchTactics, optimal: &Optimal, with_seed: bool) -> Option<Game<W>> {
        let board = Board::<W>::for_n(n);
        if board.is_trivial() {
            Line::of(n).field("trivial", true).print();
            return None;
        }
        let p = board.prepare(tactics).expect("prepare");
        let opt = optimal.score(n).map(|score| score - board.in_the_bag);
        let seed = optimal.moves(n - 1).filter(|_| with_seed).map(|moves| board.seed(moves));
        let ceiling = optimal.score(n - 1).filter(|_| with_seed).and_then(|score| board.promotion_cap(score));
        Some(Game { board, p, opt, seed, ceiling })
    }
}

/// The fastest of `repeat` runs of `f`, in seconds, and the value of that run.
fn fastest<T>(repeat: usize, mut f: impl FnMut() -> T) -> (f64, T) {
    let mut best: Option<(Duration, T)> = None;
    for _ in 0..repeat {
        let t0 = Instant::now();
        let value = f();
        let time = t0.elapsed();
        if best.as_ref().is_none_or(|(fastest, _)| time < *fastest) {
            best = Some((time, value));
        }
    }
    let (time, value) = best.expect("at least one run");
    (time.as_secs_f64(), value)
}

/// What a call cost: its wall time, and the cycles and instructions of the process during it (0 where the system
/// does not count them).
struct Call {
    wall: Duration,
    cycles: u64,
    instructions: u64,
}

fn measure<T>(f: impl FnOnce() -> T) -> (T, Call) {
    let before = counters::now();
    let t0 = Instant::now();
    let value = f();
    let wall = t0.elapsed();
    let after = counters::now();
    let (instructions, cycles) = match (before, after) {
        (Some((i0, c0)), Some((i1, c1))) => (i1 - i0, c1 - c0),
        _ => (0, 0),
    };
    (value, Call { wall, cycles, instructions })
}

/// A search call's outcome as far as the reports go, and what it cost.
struct Run {
    completed: bool,
    best: u64,
    stats: Stats,
    counters: Counters,
    call: Call,
}

/// The proof of `matroid prove`: the engine searches `p` from the incumbent value `best`, which has no set behind
/// it, with no seed and no ceiling.  Only the engine's run is timed, not the graphs it starts from.
fn proof<const W: usize>(p: &Prepared<W>, cfg: &EngineConfig, best: u64) -> Run {
    let graph = MatchGraph::new(&p.model, &p.root);
    let space = SearchSpace::new(p, &graph, None);
    let units = vec![TaxUnit { promoted: Bits::<W>::EMPTY, cands: p.candidates }];
    // The incumbent is a value only: its payload, an empty set, is never read back here.
    let start = Start::Fresh { units, best, payload: vec![0u8; 8 * W] };
    let (out, call) = measure(|| engine::run(cfg, start, b"wall", |_| TaxSearcher::new(&space)).expect("search"));
    Run { completed: out.completed, best: out.best, stats: out.stats, counters: out.counters, call }
}

/// The sizes of a root: what the sets, the GF(2) vectors and the frame count have to hold.
struct Sizes {
    n: u32,
    /// Words per set.
    width: usize,
    /// Per frame, the numbers of moves, factors and freedoms.
    frames: Vec<(u32, u32, i32)>,
    candidates: u32,
    members: u32,
    root_max: u32,
}

impl Sizes {
    /// The sizes of the root whose frames have the given moves, factors and freedoms.
    fn new<const W: usize>(
        n: u32,
        frames: impl IntoIterator<Item = (Bits<W>, Bits<W>, i32)>,
        candidates: &Bits<W>,
    ) -> Sizes {
        let mut members = *candidates;
        let mut counts = Vec::new();
        for (moves, factors, freedoms) in frames {
            members |= moves | factors;
            counts.push((moves.len(), factors.len(), freedoms));
        }
        Sizes {
            n,
            width: W,
            frames: counts,
            candidates: candidates.len(),
            members: members.len(),
            root_max: members.max().unwrap_or(0),
        }
    }

    fn of_root<const W: usize>(n: u32, p: &Prepared<W>) -> Sizes {
        Sizes::new(n, p.root.frames().iter().map(|f| (f.moves, f.factors, f.freedoms)), &p.candidates)
    }

    fn max_freedoms(&self) -> u32 {
        self.frames.iter().map(|&(_, _, freedoms)| freedoms.max(0) as u32).max().unwrap_or(0)
    }

    /// Adds the sizes to `line`; the per-frame lists are named with `prefix` (`frame_moves`, or `moves` for none).
    fn add_to(&self, line: &mut Line, prefix: &str) {
        line.field("frames", self.frames.len())
            .field("candidates", self.candidates)
            .field("members", self.members)
            .field("root_max", self.root_max)
            .field("words_n", words_for(self.n))
            .field("width", self.width)
            .field("words_members", self.members.div_ceil(64))
            .field("words_candidates", self.candidates.div_ceil(64))
            .field("words_root_max", words_for(self.root_max))
            .list(&format!("{prefix}moves"), self.frames.iter().map(|f| f.0))
            .list(&format!("{prefix}factors"), self.frames.iter().map(|f| f.1))
            .list(&format!("{prefix}freedoms"), self.frames.iter().map(|f| f.2));
    }
}

/// The root's GF(2) bound: building the matroids, the bound with no incumbent, and the same evaluation asked for the
/// members forced by an incumbent worth `start`.  Adds the gaps of the bound over `start` and over `opt`.
fn root_gf2<const W: usize>(line: &mut Line, p: &Prepared<W>, start: u64, opt: Option<u64>) {
    let t0 = Instant::now();
    let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
    let graph_time = t0.elapsed();

    let mut bound = Gf2Bound::<W>::new(&graph);
    let t1 = Instant::now();
    bound.reset(&Bits::EMPTY, &p.candidates);
    let eval = bound.evaluate(p.max_total, None);
    let bound_time = t1.elapsed();

    let mut split = Gf2Bound::<W>::new(&graph);
    let t2 = Instant::now();
    split.reset(&Bits::EMPTY, &p.candidates);
    let forced = split.evaluate(p.max_total, Some(start + 1)).forced;
    let split_time = t2.elapsed();

    let gap = |floor: u64| eval.value as i64 - floor as i64;
    line.seconds("gf2_graph_s", graph_time.as_secs_f64())
        .field("elements", graph.len())
        .list("ranks", graph.ranks())
        .field("block_words", graph.words())
        .seconds("root_bound_s", bound_time.as_secs_f64())
        .field("root_bound", eval.value)
        .field("root_chosen", eval.chosen.len())
        .field("root_augmentations", bound.stats().augmentations)
        .seconds("root_split_s", split_time.as_secs_f64())
        .field("root_forced", forced.len())
        .field("gap_start", gap(start))
        .maybe("gap_opt", opt.map(gap));
}

/// The engine's counters for pausing, checkpointing and queueing.
fn engine_fields(line: &mut Line, c: &Counters) {
    line.field("pauses", c.pauses)
        .seconds("pause_total_s", c.pause_total.as_secs_f64())
        .seconds("pause_max_s", c.pause_max.as_secs_f64())
        .seconds("park_max_s", c.park_max.as_secs_f64())
        .field("checkpoints", c.checkpoints)
        .field("max_queue", c.max_queue)
        .field("spilled_units", c.spilled_units)
        .field("runs", c.runs)
        .field("tiny_runs", c.tiny_runs);
}

/// What a search came to and cost: outcome, time, the promotion and GF(2) counters, the cost per node and per
/// augmentation, and the engine's counters.
fn run_fields(line: &mut Line, run: &Run) {
    let (x, c) = (&run.stats.extra, &run.counters);
    let busy_us = c.busy.as_secs_f64() * 1e6;
    line.field("completed", run.completed)
        .field("best", run.best)
        .field("nodes", run.stats.nodes)
        .seconds("seconds", run.call.wall.as_secs_f64())
        .seconds("busy_s", c.busy.as_secs_f64())
        .field("cycles", run.call.cycles)
        .field("instructions", run.call.instructions)
        .field("promote_attempts", x[stat::PROMOTE_ATTEMPTS])
        .field("promotions", x[stat::PROMOTIONS])
        .field("children", x[stat::CHILDREN])
        .field("gf2_solves", x[stat::GF2_SOLVES])
        .field("gf2_augmentations", x[stat::GF2_AUGMENTATIONS])
        .field("gf2_strong_solves", x[stat::GF2_STRONG_SOLVES])
        .field("gf2_strong_augmentations", x[stat::GF2_STRONG_AUGMENTATIONS])
        .ratio("us_per_node", busy_us, run.stats.nodes)
        .ratio("us_per_aug", busy_us, x[stat::GF2_AUGMENTATIONS])
        .ratio("cycles_per_node", run.call.cycles as f64, run.stats.nodes);
    engine_fields(line, c);
}

/// The checkpoint a run writes, from `--ckpt-dir` and `--every`: the file `name` in the directory (which is made if
/// missing, and where a stale file of that name is deleted) and the interval.
fn checkpoint_in(o: &Options, name: &str) -> Result<Option<(PathBuf, Duration)>, String> {
    let (Some(dir), Some(every)) = (&o.ckpt_dir, o.every) else { return Ok(None) };
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let path = dir.join(name);
    engine::remove_checkpoint(&path).map_err(|e| e.to_string())?;
    Ok(Some((path, every)))
}

fn file_len(path: Option<&Path>) -> Option<u64> {
    fs::metadata(path?).ok().map(|m| m.len())
}

/// The size of the run's checkpoint file, if it wrote one; the file is deleted.
fn take_checkpoint(checkpoint: &Option<(PathBuf, Duration)>) -> Result<Option<u64>, String> {
    let Some((path, _)) = checkpoint else { return Ok(None) };
    let len = file_len(Some(path));
    engine::remove_checkpoint(path).map_err(|e| e.to_string())?;
    Ok(len)
}

/// The incumbent value `opt + delta` of a proof, if it is not below zero.
fn incumbent(n: u32, opt: u64, delta: Option<i64>) -> Result<u64, String> {
    let delta = delta.unwrap_or(0);
    opt.checked_add_signed(delta).ok_or_else(|| format!("n={n}: --delta {delta} puts the incumbent below zero"))
}

// ---------------------------------------------------------------------------------------------------------
// setup

fn setup<const W: usize>(n: u32, o: &Options, optimal: &Optimal) -> Result<(), String> {
    let Some(game) = Game::<W>::new(n, SearchTactics::default(), optimal, true) else { return Ok(()) };
    let Game { board, p, opt, seed, ceiling } = &game;
    let (r, tables) = (o.repeat, &board.tables);
    let mut line = Line::of(n);
    line.field("prime", tables.is_prime(n));

    // Building the board, piece by piece and whole.
    line.seconds("tables_s", fastest(r, || Tables::new(n)).0)
        .seconds("frames_s", fastest(r, || build_frames::<W>(tables)).0)
        .seconds("plain_s", fastest(r, || solution_for_promotions::<W>(tables, Bits::EMPTY)).0)
        .seconds("board_s", fastest(r, || Board::<W>::new(Tables::new(n))).0);

    // What the search is given.
    let (prepare_s, prepared) = fastest(r, || board.prepare(SearchTactics::default()).expect("prepare"));
    line.seconds("prepare_s", prepare_s)
        .seconds("model_s", prepared.timings.build.as_secs_f64())
        .seconds("greedy_s", prepared.timings.greedy.as_secs_f64())
        .seconds("problem_s", fastest(r, || p.problem().digest()).0)
        .seconds("search_problem_s", fastest(r, || search_problem(n).expect("problem")).0)
        .seconds("match_graph_s", fastest(r, || MatchGraph::new(&p.model, &p.root)).0);
    let hood = Neighbourhood { model: &p.model, root: &p.root, candidates: &p.candidates };
    line.seconds("improve_s", fastest(r, || hood.improve(&mut Scratch::new(), &p.greedy, true)).0);

    // The search's own setup: it is stopped before its first node.
    let cfg = EngineConfig { threads: 1, time_limit: Some(Duration::ZERO), ..EngineConfig::default() };
    let (zero_s, zero) = fastest(r, || search(p, &cfg, seed.as_ref(), *ceiling, false).expect("search"));
    let start = zero.start.sum();
    line.seconds("zero_search_s", zero_s).field("capped", zero.reached_cap);
    root_gf2(&mut line, p, start, *opt);

    // Turning the promotions into a game, and checking it.
    let (moves_s, moves) = fastest(r, || solution_for_promotions::<W>(tables, zero.start));
    let (replay_s, replay) = fastest(r, || replay_finished(n, &moves));
    line.seconds("moves_s", moves_s)
        .seconds("replay_s", replay_s)
        .field("replay_ok", replay.is_ok())
        .seconds("prime_path_s", fastest(r, || Apiary::<W>::new(tables, Bits::EMPTY).promotion_candidates()).0);

    line.field("max_total", p.max_total);
    Sizes::of_root(n, p).add_to(&mut line, "frame_");
    line.maybe("opt", *opt).field("start", start).maybe("max_rss", counters::max_rss());
    line.print();
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// nodes

fn nodes<const W: usize>(n: u32, o: &Options, optimal: &Optimal) -> Result<(), String> {
    let Some(game) = Game::<W>::new(n, o.tactics, optimal, !o.no_seed) else { return Ok(()) };
    let Game { p, opt, seed, ceiling, .. } = &game;
    if o.mode == Mode::Opt && opt.is_none() {
        Line::of(n).text("error", "--mode opt needs the known optimum (--known FILE)").print();
        return Ok(());
    }
    let node_limit = o.nodes.unwrap_or(DEFAULT_NODES);
    let checkpoint = checkpoint_in(o, &format!("wall_n{n}_w{W}.ckpt"))?;
    let cfg = EngineConfig {
        threads: 1,
        node_limit: Some(node_limit),
        time_limit: o.seconds,
        checkpoint: checkpoint.clone(),
        keep_completed_checkpoint: true,
        ..EngineConfig::default()
    };
    let (run, start, delta) = match (o.mode, opt) {
        (Mode::Opt, Some(opt)) => {
            let start = incumbent(n, *opt, o.delta)?;
            (proof(p, &cfg, start), start, o.delta.or(Some(0)))
        }
        _ => {
            let (result, call) = measure(|| search(p, &cfg, seed.as_ref(), *ceiling, false).expect("search"));
            let run = Run {
                completed: result.completed,
                best: result.sum,
                stats: result.stats,
                counters: result.counters,
                call,
            };
            (run, result.start.sum(), None)
        }
    };
    let ckpt_bytes = take_checkpoint(&checkpoint)?;

    let mut line = Line::of(n);
    line.text("mode", o.mode.name())
        .maybe("delta", delta)
        .text("tactics", &Tactics { search: o.tactics, ..Tactics::default() }.description())
        .field("node_limit", node_limit)
        .field("max_total", p.max_total);
    Sizes::of_root(n, p).add_to(&mut line, "frame_");
    root_gf2(&mut line, p, start, *opt);
    line.field("start", start).maybe("opt", *opt);
    run_fields(&mut line, &run);
    line.maybe("ckpt_bytes", ckpt_bytes).maybe("max_rss", counters::max_rss());
    line.print();
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// grow

/// Prints a line of the process's memory, the checkpoint's size and the spill directory's files every `period`
/// until `done`; it looks every 50 ms, so it ends soon after.
fn monitor(done: &AtomicBool, period: Duration, checkpoint: Option<&Path>, spill: Option<&Path>) {
    let started = Instant::now();
    let mut next = period;
    while !done.load(Ordering::Acquire) {
        std::thread::sleep(Duration::from_millis(50));
        if started.elapsed() < next {
            continue;
        }
        next += period;
        let spill_files =
            spill.and_then(|dir| fs::read_dir(dir).ok()).map_or(0, |dir| dir.filter_map(Result::ok).count());
        let mut line = Line::new();
        line.seconds("t", started.elapsed().as_secs_f64())
            .maybe("max_rss", counters::max_rss())
            .maybe("ckpt_bytes", file_len(checkpoint))
            .field("spill_files", spill_files);
        line.print();
    }
}

fn grow<const W: usize>(n: u32, o: &Options, optimal: &Optimal) -> Result<(), String> {
    let Some(game) = Game::<W>::new(n, o.tactics, optimal, false) else { return Ok(()) };
    let p = &game.p;
    let opt = game.opt.ok_or_else(|| format!("n={n}: grow needs the known optimum (--known FILE)"))?;
    let start = incumbent(n, opt, o.delta)?;
    let checkpoint = checkpoint_in(o, &format!("grow_n{n}.ckpt"))?;
    let mut cfg = EngineConfig {
        threads: o.threads,
        cores: o.cores,
        node_limit: o.nodes,
        time_limit: o.seconds,
        progress: Some(o.progress),
        label: format!("n={n}"),
        checkpoint: checkpoint.clone(),
        keep_completed_checkpoint: true,
        spill_dir: o.ckpt_dir.as_ref().map(|dir| dir.join("spill")),
        ..EngineConfig::default()
    };
    if let Some(units) = o.mem_cap {
        cfg.queue_mem_cap = units;
    }

    let done = AtomicBool::new(false);
    let checkpoint_path = checkpoint.as_ref().map(|(path, _)| path.as_path());
    let run = std::thread::scope(|scope| {
        scope.spawn(|| monitor(&done, o.progress, checkpoint_path, cfg.spill_dir.as_deref()));
        let run = proof(p, &cfg, start);
        done.store(true, Ordering::Release);
        run
    });
    let ckpt_bytes = take_checkpoint(&checkpoint)?;

    let c = &run.counters;
    let mut line = Line::of(n);
    line.field("opt", opt).field("delta", o.delta.unwrap_or(0)).field("threads", o.threads).maybe("cores", o.cores);
    run_fields(&mut line, &run);
    line.field("splits", c.splits)
        .field("split_units", c.split_units)
        .field("steals", c.steals)
        .field("spill_segments", c.spill_segments)
        .field("reloaded_segments", c.reloaded_segments)
        .maybe("ckpt_bytes", ckpt_bytes)
        .maybe("max_rss", counters::max_rss());
    line.print();
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// start

/// The promotions of `seed` that are candidates and still promote one by one from the largest down, skipping the
/// ones that fail: what `search::incumbent::surviving` (private to the library) keeps of the solver's seed.
fn surviving_seed<const W: usize>(p: &Prepared<W>, seed: &Bits<W>) -> Bits<W> {
    let mut state = p.root;
    let mut kept = Bits::EMPTY;
    for x in (*seed & p.candidates).iter_desc() {
        if p.model.promote_in_place(&mut state, x) {
            kept.insert(x);
        }
    }
    kept
}

fn start<const W: usize>(n: u32, _: &Options, optimal: &Optimal) -> Result<(), String> {
    let Some(game) = Game::<W>::new(n, SearchTactics::default(), optimal, true) else { return Ok(()) };
    let Game { p, opt, seed, ceiling, .. } = &game;
    let Some(seed) = seed else {
        Line::of(n).field("no_previous", true).print();
        return Ok(());
    };

    // The two incumbents the solver starts from, and the better of them (it also tries a matching dive, not here).
    let greedy = p.greedy.sum();
    let (seed_kept_s, kept) = fastest(1, || surviving_seed(p, seed));
    let base = if kept.sum() > greedy { kept } else { p.greedy };
    let base_sum = base.sum();

    // The local search from it, as the solver calls it (with pairs), and without them.
    let hood = Neighbourhood { model: &p.model, root: &p.root, candidates: &p.candidates };
    let (singles_s, singles) = fastest(1, || hood.improve(&mut Scratch::new(), &base, false).sum());
    let (pairs_s, pairs) = fastest(1, || hood.improve(&mut Scratch::new(), &base, true).sum());

    let mut line = Line::of(n);
    line.field("greedy", greedy)
        .field("seed_kept", kept.sum())
        .seconds("seed_kept_s", seed_kept_s)
        .field("base", base_sum)
        .field("members", base.len())
        .maybe("ceiling", *ceiling)
        .maybe("at_ceiling", ceiling.map(|ceiling| base_sum >= ceiling))
        .seconds("singles_s", singles_s)
        .field("singles", singles)
        .seconds("pairs_s", pairs_s)
        .field("pairs", pairs)
        .maybe("opt", *opt)
        .maybe("max_rss", counters::max_rss());
    line.print();
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// frames

/// `frames` for board `n`, at the set width the solver picks for it, or at 128 words beyond the widest.
fn frames_at_width(n: u32) -> Result<(), String> {
    if n > Bits::<128>::MAX_VALUE {
        return Err(format!("n={n}: frames goes up to {}", Bits::<128>::MAX_VALUE));
    }
    taxman::with_width!(n, W => frames::<W>(n), else frames::<128>(n));
    Ok(())
}

fn frames<const W: usize>(n: u32) {
    let tables = Tables::new(n);
    let t0 = Instant::now();
    let built = build_frames::<W>(&tables);
    let frames_s = t0.elapsed().as_secs_f64();
    let shapes = built.frames.iter().map(|f| (f.moves, f.factors, f.free_factor_count));
    let sizes = Sizes::new(n, shapes, &built.candidates);
    let mut line = Line::of(n);
    line.seconds("frames_s", frames_s);
    sizes.add_to(&mut line, "");
    line.field("max_freedoms", sizes.max_freedoms()).field("block_words", sizes.max_freedoms().div_ceil(64));
    line.print();
}
