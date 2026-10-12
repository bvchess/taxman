//! The per-game pipeline: derive the game from the previous one if possible, else search for the promotion set
//! with the largest sum, then turn it into a game, replay it, and compare it with `optimal.json`.
//!
//! [`solve_game`] solves one board size.  It picks the set width `W` from `n` and runs, in order:
//!
//! 1. the *prime shortcut* (tactic `prime_shortcut`): a prime `n` plays the game of `n − 1` with its first
//!    move replaced by `n`;
//! 2. the frames of the board; with no promotion candidates the game with no promotions is the only one
//!    (method `trivial`);
//! 3. `reuse_prev` (tactic of that name): keep the previous game's promotions that are still possible and accept
//!    the game they make if it scores what the previous game suggests;
//! 4. otherwise [`search::prepare`] and [`search::search`] for the best promotion set.  The search starts from
//!    the better of the greedy set and the previous game's promotions that are still possible
//!    (`seed_incumbent`), and stops as soon as it reaches `n + opt(n − 1) − inTheBag` (`prev_cap`), where
//!    `inTheBag` is what the game with no promotions scores (`FrameSolver.getMaxPromotionSum`).
//!
//! Steps 1 and 3 need the previous game and are skipped by `force_search`, and when a checkpoint of the game
//! exists (a search that was interrupted is resumed, not replaced by a derivation).  The promotions become a game
//! through the hives; [`game::replay`] referees it (an illegal game is a hard error); a completed game is
//! compared with `optimal.json` when that has it (a mismatch is a hard error unless the options allow it).
//!
//! [`solve_range`] solves a range of board sizes, each feeding the next, and appends every result to a results
//! file (one JSON object per line) as soon as it is done, so that a restarted run skips what is finished.

use crate::accelerate::{prime_shortcut, reuse_prev, solution_for_promotions};
use crate::bits::{words_for, Bits};
use crate::engine::{Counters, EngineConfig, Stats};
use crate::frames::{build_frames, FrameColumns};
use crate::game::{replay, Optimal};
use crate::hives::Apiary;
use crate::json::Json;
use crate::numtheory::Tables;
use crate::search::{self, prepare, prepare_for_resume, stat, Revise, SearchTactics};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// The largest board the set widths cover (`W = 64`).
pub const MAX_N: u32 = 64 * 64 - 1;

// ---------------------------------------------------------------------------------------------------------
// Tactics

/// Every switch of DESIGN.md's table of tactics.  The ones that act inside the search are [`SearchTactics`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tactics {
    pub search: SearchTactics,
    /// A prime `n` plays the game of `n − 1` with its first move replaced by `n` (proven).
    pub prime_shortcut: bool,
    /// Accept the game made of the previous game's promotions without a search, if it scores what the previous
    /// game suggests (unproven).
    pub reuse_prev: bool,
    /// Stop the search as soon as the incumbent reaches `n + opt(n − 1) − inTheBag` (proven if `opt(n − 1)` is
    /// optimal).
    pub prev_cap: bool,
    /// Start the search from the better of the greedy set and the previous game's promotions (sound).
    pub seed_incumbent: bool,
}

impl Default for Tactics {
    fn default() -> Self {
        Tactics {
            search: SearchTactics::default(),
            prime_shortcut: true,
            reuse_prev: true,
            prev_cap: true,
            seed_incumbent: true,
        }
    }
}

/// The tactics whose answers are not proven to equal the true optimum (`reuse_prev` and `revise_freedoms` are
/// conjecture-grade; `legacy_bound` is known to be unsound).
const UNPROVEN: [&str; 3] = ["reuse_prev", "revise_freedoms", "legacy_bound"];

impl Tactics {
    /// The names of the switches paired with the switches themselves, in DESIGN.md's order.  The single place
    /// that knows the names.
    fn switches_mut(&mut self) -> [(&'static str, &mut bool); 8] {
        [
            ("prime_shortcut", &mut self.prime_shortcut),
            ("reuse_prev", &mut self.reuse_prev),
            ("revise_freedoms", &mut self.search.revise_freedoms),
            ("preclusions", &mut self.search.preclusions),
            ("match_bound", &mut self.search.match_bound),
            ("legacy_bound", &mut self.search.legacy_bound),
            ("prev_cap", &mut self.prev_cap),
            ("seed_incumbent", &mut self.seed_incumbent),
        ]
    }

    /// Every switch with its name and whether it is on.
    pub fn switches(&self) -> [(&'static str, bool); 8] {
        let mut copy = *self;
        copy.switches_mut().map(|(name, flag)| (name, *flag))
    }

    /// Applies `name=on` or `name=off`.
    pub fn set(&mut self, assignment: &str) -> Result<(), String> {
        let (name, value) =
            assignment.split_once('=').ok_or_else(|| format!("expected name=on or name=off, got {assignment:?}"))?;
        let value = match value {
            "on" => true,
            "off" => false,
            _ => return Err(format!("tactic {name}: expected on or off, got {value:?}")),
        };
        let Some((_, flag)) = self.switches_mut().into_iter().find(|(known, _)| *known == name) else {
            let known: Vec<&str> = Tactics::default().switches().iter().map(|&(known, _)| known).collect();
            return Err(format!("unknown tactic {name:?}; the tactics are {}", known.join(", ")));
        };
        *flag = value;
        Ok(())
    }

    /// `default`, or the switches that differ from the defaults (`match_bound=off,prev_cap=off`).
    pub fn description(&self) -> String {
        let defaults = Tactics::default().switches();
        let changed: Vec<String> = (self.switches().into_iter().zip(defaults))
            .filter(|((_, on), (_, default))| on != default)
            .map(|((name, on), _)| format!("{name}={}", if on { "on" } else { "off" }))
            .collect();
        if changed.is_empty() {
            "default".to_owned()
        } else {
            changed.join(",")
        }
    }

    /// The unproven switches that are on (see DESIGN.md); `legacy_bound` is unsound, the others conjectures.
    pub fn unproven_on(&self) -> Vec<&'static str> {
        self.switches().into_iter().filter(|(name, on)| *on && UNPROVEN.contains(name)).map(|(name, _)| name).collect()
    }

    /// All switches, by name, as a JSON object.
    pub fn to_json(&self) -> Json {
        Json::object(self.switches().map(|(name, on)| (name, Json::Bool(on))))
    }
}

// ---------------------------------------------------------------------------------------------------------
// Options and results

/// How to solve.
#[derive(Clone, Debug)]
pub struct SolveOptions {
    pub tactics: Tactics,
    pub threads: usize,
    /// Directory of the per-game checkpoints `n<NNNN>.ckpt`; a game with a checkpoint there is resumed.
    pub checkpoint_dir: Option<PathBuf>,
    pub checkpoint_interval: Duration,
    /// Interval between progress lines on stderr.
    pub progress: Option<Duration>,
    /// Per game and invocation; an unfinished search keeps its checkpoint.
    pub time_limit: Option<Duration>,
    pub node_limit: Option<u64>,
    /// Most queued search units kept in memory; the rest spill to disk.
    pub queue_mem_cap: usize,
    /// Where queued units spill to; by default `spill` in the checkpoint directory, else a temporary directory.
    pub spill_dir: Option<PathBuf>,
    /// Search even where the previous game would give the answer (skips `prime_shortcut` and `reuse_prev`).
    pub force_search: bool,
    /// The file to compare scores with and to take previous games from; `None` for no comparison.
    pub optimal_path: Option<PathBuf>,
    /// Record a score that differs from `optimal.json` (as `optimal_match: false`) instead of failing.
    pub allow_optimal_mismatch: bool,
}

impl Default for SolveOptions {
    fn default() -> Self {
        let engine = EngineConfig::default();
        SolveOptions {
            tactics: Tactics::default(),
            threads: engine.threads,
            checkpoint_dir: None,
            checkpoint_interval: Duration::from_secs(60),
            progress: None,
            time_limit: None,
            node_limit: None,
            queue_mem_cap: engine.queue_mem_cap,
            spill_dir: None,
            force_search: false,
            optimal_path: Some(Optimal::default_path()),
            allow_optimal_mismatch: false,
        }
    }
}

impl SolveOptions {
    /// Loads `optimal_path`, if there is one.
    pub fn load_optimal(&self) -> Result<Option<Optimal>, String> {
        self.optimal_path.as_ref().map(Optimal::load).transpose()
    }

    /// The checkpoint file of game `n`, if checkpointing is on.
    pub fn checkpoint_file(&self, n: u32) -> Option<PathBuf> {
        self.checkpoint_dir.as_ref().map(|dir| dir.join(format!("n{n:04}.ckpt")))
    }
}

/// Where the solution of the previous game came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrevSource {
    /// Solved earlier in this run.
    Run,
    /// Read from the results file.
    Results,
    /// Read from `optimal.json`.
    Optimal,
}

impl PrevSource {
    pub fn as_str(self) -> &'static str {
        match self {
            PrevSource::Run => "run",
            PrevSource::Results => "results",
            PrevSource::Optimal => "optimal",
        }
    }
}

/// The solution of game `n − 1`.
#[derive(Clone, Debug)]
pub struct Prev {
    pub moves: Vec<u32>,
    pub score: u64,
    pub source: PrevSource,
}

impl Prev {
    /// The game for board size `n` in `optimal`, if it has one.
    pub fn from_optimal(optimal: &Optimal, n: u32) -> Option<Prev> {
        let (moves, score) = (optimal.moves(n)?, optimal.score(n)?);
        Some(Prev { moves: moves.to_vec(), score, source: PrevSource::Optimal })
    }

    fn from_result(result: &GameResult) -> Prev {
        Prev { moves: result.moves.clone(), score: result.score, source: PrevSource::Run }
    }
}

/// How a game was solved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// No promotion candidates: the game with no promotions.
    Trivial,
    /// The prime shortcut.
    Prime,
    /// `reuse_prev`.
    Reuse,
    /// The branch-and-bound search.
    Search,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Trivial => "trivial",
            Method::Prime => "prime",
            Method::Reuse => "reuse",
            Method::Search => "search",
        }
    }

    /// Everything but a search (what the Java calls an accelerated game).
    pub fn is_derived(self) -> bool {
        self != Method::Search
    }
}

/// Wall time of each phase of a game.
#[derive(Clone, Copy, Debug, Default)]
pub struct Phases {
    /// Hives, frames, and the search model.
    pub frames: Duration,
    pub preclusions: Duration,
    pub greedy: Duration,
    /// Revising frame 0's freedoms, including its own search.
    pub revise: Duration,
    /// The branch and bound (this invocation only).
    pub search: Duration,
    /// The whole game.
    pub total: Duration,
}

impl Phases {
    pub fn to_json(&self) -> Json {
        let secs = |d: Duration| Json::Num(d.as_secs_f64());
        Json::object([
            ("frames", secs(self.frames)),
            ("preclusions", secs(self.preclusions)),
            ("greedy", secs(self.greedy)),
            ("revise", secs(self.revise)),
            ("search", secs(self.search)),
            ("total", secs(self.total)),
        ])
    }
}

/// What the search was set up with (only for games that were searched).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchSetup {
    pub frames: usize,
    pub candidates: u32,
    pub max_total: u32,
    pub greedy_sum: u64,
    /// The `prev_cap` ceiling on the promotion sum.
    pub cap: Option<u64>,
    pub revise: Revise,
}

/// The names of [`Stats::extra`], by the index [`search::stat`] gives each.
const STAT_NAMES: [(&str, usize); 8] = [
    ("promote_attempts", stat::PROMOTE_ATTEMPTS),
    ("promotions", stat::PROMOTIONS),
    ("upper_bounds", stat::UPPER_BOUNDS),
    ("can_promotes", stat::CAN_PROMOTES),
    ("children", stat::CHILDREN),
    ("precluded", stat::PRECLUDED),
    ("certificates", stat::CERTIFICATES),
    ("match_bounds", stat::MATCH_BOUNDS),
];

/// The outcome of solving one board size.
#[derive(Clone, Debug)]
pub struct GameResult {
    pub n: u32,
    /// The game's score; for an incomplete search, that of the best game found so far.
    pub score: u64,
    pub moves: Vec<u32>,
    /// The numbers promoted, ascending.
    pub promotions: Vec<u32>,
    pub method: Method,
    /// False when a time or node limit stopped the search (its checkpoint is kept and `score` is only a lower
    /// bound).
    pub completed: bool,
    /// Search nodes, over all the runs of a resumed search.
    pub nodes: u64,
    /// The search's counters (see [`search::stat`]) and the engine's activity.
    pub stats: Stats,
    pub counters: Counters,
    pub setup: Option<SearchSetup>,
    pub phases: Phases,
    pub threads: usize,
    pub tactics: Tactics,
    /// Whether the score equals `optimal.json`'s; `None` if the game is incomplete or the file lacks `n`.
    pub optimal_match: Option<bool>,
    pub prev_source: Option<PrevSource>,
}

impl GameResult {
    fn new(n: u32, opts: &SolveOptions, prev: Option<&Prev>) -> Self {
        GameResult {
            n,
            score: 0,
            moves: Vec::new(),
            promotions: Vec::new(),
            method: Method::Search,
            completed: true,
            nodes: 0,
            stats: Stats::default(),
            counters: Counters::default(),
            setup: None,
            phases: Phases::default(),
            threads: opts.threads,
            tactics: opts.tactics,
            optimal_match: None,
            prev_source: prev.map(|p| p.source),
        }
    }

    /// The search's counters by name.
    pub fn stats_json(&self) -> Json {
        Json::object(STAT_NAMES.map(|(name, i)| (name, Json::from(self.stats.extra[i]))))
    }

    /// The result as one JSON object.
    pub fn to_json(&self) -> Json {
        let c = &self.counters;
        let engine = Json::object([
            ("splits", Json::from(c.splits)),
            ("checkpoints", Json::from(c.checkpoints)),
            ("spilled_units", Json::from(c.spilled_units)),
            ("max_queue", Json::from(c.max_queue)),
        ]);
        let setup = self.setup.map_or(Json::Null, |s| {
            Json::object([
                ("frames", Json::from(s.frames)),
                ("candidates", Json::from(s.candidates)),
                ("max_total", Json::from(s.max_total)),
                ("greedy_sum", Json::from(s.greedy_sum)),
                ("cap", s.cap.map_or(Json::Null, Json::from)),
                ("revise", Json::from(format!("{:?}", s.revise))),
            ])
        });
        Json::object([
            ("n", Json::from(self.n)),
            ("score", Json::from(self.score)),
            ("moves", Json::array(self.moves.iter().copied())),
            ("promotions", Json::array(self.promotions.iter().copied())),
            ("method", Json::from(self.method.as_str())),
            ("completed", Json::Bool(self.completed)),
            ("nodes", Json::from(self.nodes)),
            ("stats", self.stats_json()),
            ("engine", engine),
            ("setup", setup),
            ("seconds", self.phases.to_json()),
            ("threads", Json::from(self.threads)),
            ("tactics", Json::from(self.tactics.description())),
            ("tactic_switches", self.tactics.to_json()),
            ("unproven_tactics_on", Json::array(self.tactics.unproven_on())),
            ("optimal_match", self.optimal_match.map_or(Json::Null, Json::Bool)),
            ("prev_source", self.prev_source.map_or(Json::Null, |s| Json::from(s.as_str()))),
        ])
    }

    /// One line for a person: board size, score, method, nodes, time, and the comparison with `optimal.json`.
    pub fn summary_line(&self) -> String {
        let optimal = match (self.completed, self.optimal_match) {
            (false, _) => "incomplete",
            (true, Some(true)) => "optimal ok",
            (true, Some(false)) => "OPTIMAL MISMATCH",
            (true, None) => "optimal unknown",
        };
        format!(
            "n={:<5} score={:<9} {:<7} nodes={:<11} {:>9.3} s  {optimal}",
            self.n,
            self.score,
            self.method.as_str(),
            self.nodes,
            self.phases.total.as_secs_f64()
        )
    }
}

// ---------------------------------------------------------------------------------------------------------
// One game

/// Solves board size `n`.  `prev` is the solution of `n − 1` if known; `opts.optimal_path` is loaded for the
/// comparison (use [`solve_game_with`] to load it once for many games).
pub fn solve_game(n: u32, prev: Option<&Prev>, opts: &SolveOptions) -> Result<GameResult, String> {
    solve_game_with(n, prev, opts, opts.load_optimal()?.as_ref())
}

/// [`solve_game`] with `optimal.json` already loaded (`opts.optimal_path` is not used).
pub fn solve_game_with(
    n: u32,
    prev: Option<&Prev>,
    opts: &SolveOptions,
    optimal: Option<&Optimal>,
) -> Result<GameResult, String> {
    let result = match words_for(n) {
        _ if n == 0 => return Err("n must be at least 1".into()),
        1..=8 => solve_with_width::<8>(n, prev, opts, optimal),
        9..=16 => solve_with_width::<16>(n, prev, opts, optimal),
        17..=32 => solve_with_width::<32>(n, prev, opts, optimal),
        33..=64 => solve_with_width::<64>(n, prev, opts, optimal),
        _ => return Err(format!("n={n} is beyond the largest supported board, n={MAX_N}")),
    };
    result.map_err(|e| format!("n={n}: {e}"))
}

fn solve_with_width<const W: usize>(
    n: u32,
    prev: Option<&Prev>,
    opts: &SolveOptions,
    optimal: Option<&Optimal>,
) -> Result<GameResult, String> {
    let start = Instant::now();
    let tactics = opts.tactics;
    if let Some(prev) = prev {
        // The derivations and the cap trust the previous game, so make sure it is what it says it is.
        match replay(n - 1, &prev.moves) {
            Ok(score) if score == prev.score => {}
            Ok(score) => {
                return Err(format!("the previous game scores {score}, not the {} it is given as", prev.score))
            }
            Err(e) => return Err(format!("the previous game is not a legal game of n={}: {e}", n - 1)),
        }
    }
    let tables = Tables::new(n);
    let mut result = GameResult::new(n, opts, prev);
    // A search that was interrupted is resumed, whatever else could have produced the game.
    let checkpoint = opts.checkpoint_file(n).filter(|path| path.exists());
    let derive = !opts.force_search && checkpoint.is_none();

    if derive && tactics.prime_shortcut && tables.is_prime(n) {
        if let Some(prev) = prev {
            let moves = prime_shortcut(n, &prev.moves);
            let candidates = Apiary::<W>::new(&tables, Bits::EMPTY).promotion_candidates();
            let promotions = candidates & moves.iter().copied().collect::<Bits<W>>();
            result.method = Method::Prime;
            return conclude(result, moves, promotions.to_vec(), opts, optimal, start);
        }
    }

    let t0 = Instant::now();
    let frames = build_frames::<W>(&tables);
    result.phases.frames = t0.elapsed();
    if frames.candidates.is_empty() {
        result.method = Method::Trivial;
        return conclude(result, solution_for_promotions::<W>(&tables, Bits::EMPTY), Vec::new(), opts, optimal, start);
    }

    if derive && tactics.reuse_prev {
        if let Some(derived) = prev.and_then(|p| reuse_prev(&tables, &frames, &p.moves)) {
            result.method = Method::Reuse;
            return conclude(result, derived.moves, derived.promotions, opts, optimal, start);
        }
    }

    let promotions = search_game(&tables, &frames, prev, opts, checkpoint, &mut result)?;
    let moves = solution_for_promotions::<W>(&tables, promotions);
    conclude(result, moves, promotions.to_vec(), opts, optimal, start)
}

/// Replays the game, compares it with `optimal.json`, and completes the result.
fn conclude(
    mut result: GameResult,
    moves: Vec<u32>,
    promotions: Vec<u32>,
    opts: &SolveOptions,
    optimal: Option<&Optimal>,
    start: Instant,
) -> Result<GameResult, String> {
    let n = result.n;
    result.score = replay(n, &moves).map_err(|e| format!("the {} game is not legal: {e}", result.method.as_str()))?;
    if result.completed {
        if let Some(expected) = optimal.and_then(|o| o.score(n)) {
            result.optimal_match = Some(expected == result.score);
            if expected != result.score && !opts.allow_optimal_mismatch {
                return Err(format!("scored {} but optimal.json says {expected}", result.score));
            }
        }
    }
    result.moves = moves;
    result.promotions = promotions;
    result.phases.total = start.elapsed();
    Ok(result)
}

/// The search path: prepares the game (or the checkpoint's game), searches, and fills `result` with what the
/// search did.  Returns the best promotion set found.
fn search_game<const W: usize>(
    tables: &Tables,
    frames: &FrameColumns<W>,
    prev: Option<&Prev>,
    opts: &SolveOptions,
    checkpoint: Option<PathBuf>,
    result: &mut GameResult,
) -> Result<Bits<W>, String> {
    let n = tables.n;
    let tactics = opts.tactics;
    let columns = frames.columns();
    let resuming = checkpoint.is_some();
    let prepared = match &checkpoint {
        Some(path) => {
            let p = prepare_for_resume::<W>(tables, &columns, &frames.candidates, path, opts.threads).map_err(|e| {
                format!("cannot resume from {}: {e} (delete the file to start the game over)", path.display())
            })?;
            if p.tactics != tactics.search {
                return Err(format!(
                    "{} was made with different search tactics than the ones given (delete it to start over)",
                    path.display()
                ));
            }
            p
        }
        None => prepare::<W>(tables, &columns, &frames.candidates, tactics.search, opts.threads)?,
    };
    result.phases.frames += prepared.timings.build;
    result.phases.preclusions = prepared.timings.preclusions;
    result.phases.greedy = prepared.timings.greedy;
    result.phases.revise = prepared.timings.revise;

    let cap = match prev {
        Some(prev) if tactics.prev_cap => {
            let in_the_bag: u64 = solution_for_promotions::<W>(tables, Bits::EMPTY).iter().map(|&m| u64::from(m)).sum();
            (prev.score + u64::from(n)).checked_sub(in_the_bag)
        }
        _ => None,
    };
    // A resumed search has its incumbent in the checkpoint.
    let seed = match prev {
        Some(prev) if tactics.seed_incumbent && !resuming => {
            Some(prev.moves.iter().copied().collect::<Bits<W>>() & frames.candidates)
        }
        _ => None,
    };
    result.setup = Some(SearchSetup {
        frames: frames.frames.len(),
        candidates: frames.candidates.len(),
        max_total: prepared.max_total,
        greedy_sum: prepared.greedy.sum(),
        cap,
        revise: prepared.revise,
    });

    let (spill_dir, ours) = spill_dir(opts, n);
    let cfg = EngineConfig {
        threads: opts.threads,
        queue_mem_cap: opts.queue_mem_cap,
        spill_dir: Some(spill_dir.clone()),
        checkpoint: checkpoint.or_else(|| opts.checkpoint_file(n)).map(|path| (path, opts.checkpoint_interval)),
        progress: opts.progress,
        time_limit: opts.time_limit,
        node_limit: opts.node_limit,
        label: format!("n={n}"),
    };
    let found = search::search(&prepared, &cfg, seed.as_ref(), cap, resuming);
    if ours {
        // Only removes the directory if the engine left nothing in it.
        let _ = fs::remove_dir(&spill_dir);
    }
    let found = found.map_err(|e| format!("search failed: {e}"))?;
    result.completed = found.completed;
    result.nodes = found.stats.nodes;
    result.stats = found.stats;
    result.counters = found.counters;
    result.phases.search = found.time;
    Ok(found.promotions)
}

/// Where the engine spills to, and whether this function chose the directory (and so may remove it).
fn spill_dir(opts: &SolveOptions, n: u32) -> (PathBuf, bool) {
    if let Some(dir) = &opts.spill_dir {
        (dir.clone(), false)
    } else if let Some(dir) = &opts.checkpoint_dir {
        (dir.join("spill"), true)
    } else {
        (std::env::temp_dir().join(format!("taxman-spill-{}-n{n}", std::process::id())), true)
    }
}

// ---------------------------------------------------------------------------------------------------------
// The results file

/// A completed game read back from a results file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredGame {
    pub score: u64,
    pub moves: Vec<u32>,
}

/// A results file: one JSON object per line, appended as games finish.  Only completed games count as done.
pub struct ResultsFile {
    path: PathBuf,
    file: File,
    done: BTreeMap<u32, StoredGame>,
}

impl ResultsFile {
    /// Opens (creating if need be) the file and reads the completed games in it.  A torn last line, left by a
    /// crash while writing, is dropped.
    pub fn open(path: &Path) -> Result<ResultsFile, String> {
        let io_error = |e: std::io::Error| format!("{}: {e}", path.display());
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(io_error(e)),
        };
        let mut done = BTreeMap::new();
        let mut valid_len = 0;
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        for (i, line) in lines.iter().enumerate() {
            if !line.trim().is_empty() {
                match parse_stored(line) {
                    Ok(Some((n, game))) => {
                        done.insert(n, game);
                    }
                    Ok(None) => {}
                    Err(_) if i + 1 == lines.len() && !line.ends_with('\n') => break,
                    Err(e) => return Err(format!("{}: line {}: {e}", path.display(), i + 1)),
                }
            }
            valid_len += line.len();
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path).map_err(io_error)?;
        if valid_len < text.len() {
            file.set_len(valid_len as u64).map_err(io_error)?;
        } else if !text.is_empty() && !text.ends_with('\n') {
            file.write_all(b"\n").map_err(io_error)?;
        }
        Ok(ResultsFile { path: path.to_owned(), file, done })
    }

    /// The completed game for board size `n`, if the file has one.
    pub fn completed(&self, n: u32) -> Option<&StoredGame> {
        self.done.get(&n)
    }

    /// Appends the result and makes sure it reaches the disk.
    pub fn append(&mut self, result: &GameResult) -> Result<(), String> {
        let line = format!("{}\n", result.to_json());
        let written = self.file.write_all(line.as_bytes()).and_then(|()| self.file.sync_data());
        written.map_err(|e| format!("{}: {e}", self.path.display()))?;
        if result.completed {
            self.done.insert(result.n, StoredGame { score: result.score, moves: result.moves.clone() });
        }
        Ok(())
    }
}

/// A line of a results file: `None` for a game that did not complete.
fn parse_stored(line: &str) -> Result<Option<(u32, StoredGame)>, String> {
    let json = Json::parse(line).map_err(|e| e.to_string())?;
    let field = |name: &str| json.get(name).ok_or_else(|| format!("missing \"{name}\""));
    let n = field("n")?.as_u32().ok_or("bad \"n\"")?;
    if !field("completed")?.as_bool().ok_or("bad \"completed\"")? {
        return Ok(None);
    }
    let score = field("score")?.as_u64().ok_or("bad \"score\"")?;
    let moves = field("moves")?.as_u32_vec().ok_or("bad \"moves\"")?;
    Ok(Some((n, StoredGame { score, moves })))
}

// ---------------------------------------------------------------------------------------------------------
// Ranges

/// What a range run did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RangeSummary {
    /// Games solved in this run.
    pub solved: usize,
    /// Games already in the results file.
    pub skipped: usize,
    /// The game whose search stopped at a limit; the range ends there.
    pub stopped_at: Option<u32>,
}

/// The solution of game `n − 1` from the results file, else from `optimal.json`.
fn previous_game(n: u32, results: Option<&ResultsFile>, optimal: Option<&Optimal>) -> Option<Prev> {
    let before = n.checked_sub(1).filter(|&before| before >= 1)?;
    let stored = results.and_then(|r| r.completed(before));
    match stored {
        Some(game) => Some(Prev { moves: game.moves.clone(), score: game.score, source: PrevSource::Results }),
        None => optimal.and_then(|o| Prev::from_optimal(o, before)),
    }
}

/// [`solve_range_each`], returning the results of the games solved in this run.
pub fn solve_range(
    lo: u32,
    hi: u32,
    opts: &SolveOptions,
    results_path: Option<&Path>,
) -> Result<Vec<GameResult>, String> {
    let mut results = Vec::new();
    solve_range_each(lo, hi, opts, results_path, |r| results.push(r.clone()))?;
    Ok(results)
}

/// Solves the board sizes `lo..=hi` in order, each from the solution of the one before.  The solution of
/// `lo − 1` comes from the results file, else from `optimal.json`.  Games already completed in the results
/// file are skipped; every other result is appended to the file (if there is one) and then given to `on_game`.
/// The range stops at a game whose search did not complete.
pub fn solve_range_each(
    lo: u32,
    hi: u32,
    opts: &SolveOptions,
    results_path: Option<&Path>,
    mut on_game: impl FnMut(&GameResult),
) -> Result<RangeSummary, String> {
    if lo < 1 || lo > hi {
        return Err(format!("the range {lo}..={hi} is empty or starts below 1"));
    }
    let optimal = opts.load_optimal()?;
    let mut results = results_path.map(ResultsFile::open).transpose()?;
    let mut summary = RangeSummary::default();
    let mut prev: Option<Prev> = None;
    for n in lo..=hi {
        if let Some(game) = results.as_ref().and_then(|r| r.completed(n)) {
            prev = Some(Prev { moves: game.moves.clone(), score: game.score, source: PrevSource::Results });
            summary.skipped += 1;
            continue;
        }
        let before = if n == lo { previous_game(n, results.as_ref(), optimal.as_ref()) } else { prev.take() };
        let result = solve_game_with(n, before.as_ref(), opts, optimal.as_ref())?;
        if let Some(results) = &mut results {
            results.append(&result)?;
        }
        on_game(&result);
        summary.solved += 1;
        if !result.completed {
            summary.stopped_at = Some(n);
            break;
        }
        prev = Some(Prev::from_result(&result));
    }
    Ok(summary)
}

// ---------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("taxman-solver-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    #[test]
    fn tactics_defaults_and_switches() {
        let t = Tactics::default();
        assert_eq!(t.description(), "default");
        assert!(t.unproven_on().eq(&["reuse_prev", "revise_freedoms"]));
        let names: Vec<&str> = t.switches().iter().map(|s| s.0).collect();
        assert_eq!(
            names,
            [
                "prime_shortcut",
                "reuse_prev",
                "revise_freedoms",
                "preclusions",
                "match_bound",
                "legacy_bound",
                "prev_cap",
                "seed_incumbent"
            ]
        );
        let on: Vec<bool> = t.switches().iter().map(|s| s.1).collect();
        assert_eq!(on, [true, true, true, true, true, false, true, true]);
    }

    #[test]
    fn tactics_set_by_name() {
        let mut t = Tactics::default();
        t.set("match_bound=off").unwrap();
        t.set("legacy_bound=on").unwrap();
        t.set("prev_cap=off").unwrap();
        assert!(!t.search.match_bound && t.search.legacy_bound && !t.prev_cap);
        assert_eq!(t.description(), "match_bound=off,legacy_bound=on,prev_cap=off");
        assert!(t.unproven_on().contains(&"legacy_bound"));
        t.set("reuse_prev=off").unwrap();
        t.set("revise_freedoms=off").unwrap();
        t.set("legacy_bound=off").unwrap();
        assert!(t.unproven_on().is_empty());
        assert!(t.set("matchbound=off").unwrap_err().contains("unknown tactic"));
        assert!(t.set("match_bound=maybe").unwrap_err().contains("on or off"));
        assert!(t.set("match_bound").unwrap_err().contains("name=on"));
        let json = t.to_json();
        assert_eq!(json.get("match_bound").and_then(Json::as_bool), Some(false));
        assert_eq!(json.get("seed_incumbent").and_then(Json::as_bool), Some(true));
    }

    #[test]
    fn results_serialize_and_read_back() {
        let opts = SolveOptions { threads: 1, ..SolveOptions::default() };
        let r = solve_game(10, None, &opts).unwrap();
        assert_eq!((r.score, r.method, r.completed, r.optimal_match), (40, Method::Trivial, true, Some(true)));
        let json = Json::parse(&r.to_json().to_string()).unwrap();
        assert_eq!(json.get("n").and_then(Json::as_u32), Some(10));
        assert_eq!(json.get("method").and_then(Json::as_str), Some("trivial"));
        assert_eq!(json.get("optimal_match").and_then(Json::as_bool), Some(true));
        assert_eq!(json.get("tactics").and_then(Json::as_str), Some("default"));
        assert_eq!(json.get("prev_source"), Some(&Json::Null));
        assert_eq!(json.get("unproven_tactics_on").and_then(Json::as_arr).map(<[_]>::len), Some(2));
        let (n, stored) = parse_stored(&json.to_string()).unwrap().unwrap();
        assert_eq!((n, stored.score, stored.moves), (10, r.score, r.moves));
    }

    #[test]
    fn the_results_file_skips_what_is_done_and_survives_a_torn_line() {
        let dir = temp_dir("results");
        let path = dir.join("results.jsonl");
        let opts = SolveOptions { threads: 1, ..SolveOptions::default() };
        let first = solve_range(1, 12, &opts, Some(&path)).unwrap();
        assert_eq!(first.len(), 12);
        let again = solve_range(1, 12, &opts, Some(&path)).unwrap();
        assert!(again.is_empty(), "everything is already done");

        // Extending the range uses the stored game 12 as the previous game.
        let more = solve_range(1, 14, &opts, Some(&path)).unwrap();
        assert_eq!(more.iter().map(|r| r.n).collect::<Vec<_>>(), [13, 14]);
        assert_eq!(more[0].prev_source, Some(PrevSource::Results));
        assert_eq!(more[0].method, Method::Prime);

        // A crash in the middle of writing game 15 leaves half a line, which is dropped.
        let mut text = fs::read_to_string(&path).unwrap();
        text.push_str("{\"n\":15,\"sco");
        fs::write(&path, &text).unwrap();
        let rest = solve_range(1, 16, &opts, Some(&path)).unwrap();
        assert_eq!(rest.iter().map(|r| r.n).collect::<Vec<_>>(), [15, 16]);
        let lines = fs::read_to_string(&path).unwrap();
        assert_eq!(lines.lines().count(), 16);
        assert!(lines.lines().all(|l| Json::parse(l).is_ok()));

        // A malformed line that is not the last is an error, not something to skip.
        fs::write(&path, format!("garbage\n{}", lines.lines().next().unwrap())).unwrap();
        assert!(ResultsFile::open(&path).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_previous_game_comes_from_the_optimal_file_when_the_range_starts_late() {
        let opts = SolveOptions { threads: 1, ..SolveOptions::default() };
        let r = solve_range(50, 52, &opts, None).unwrap();
        assert_eq!(r[0].prev_source, Some(PrevSource::Optimal));
        assert_eq!(r[1].prev_source, Some(PrevSource::Run));
        let first = solve_range(1, 1, &opts, None).unwrap();
        assert_eq!((first[0].prev_source, first[0].score, first[0].moves.len()), (None, 0, 0));
    }

    #[test]
    fn a_wrong_optimal_file_is_a_hard_error_unless_allowed() {
        let dir = temp_dir("mismatch");
        let path = dir.join("optimal.json");
        fs::write(&path, r#"[{"n":4,"score":8,"moves":[4,3]}]"#).unwrap();
        let mut opts = SolveOptions { threads: 1, optimal_path: Some(path), ..SolveOptions::default() };
        let err = solve_game(4, None, &opts).unwrap_err();
        assert!(err.contains("scored 7") && err.contains("says 8"), "{err}");
        opts.allow_optimal_mismatch = true;
        let r = solve_game(4, None, &opts).unwrap();
        assert_eq!((r.score, r.optimal_match), (7, Some(false)));
        opts.optimal_path = None;
        assert_eq!(solve_game(4, None, &opts).unwrap().optimal_match, None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_previous_game_that_does_not_check_out_is_refused() {
        let opts = SolveOptions { optimal_path: None, ..SolveOptions::default() };
        let prev = |moves: &[u32], score| Prev { moves: moves.to_vec(), score, source: PrevSource::Run };
        // [5, 4, 6] is the game for 6, scoring 15.
        assert!(solve_game(7, Some(&prev(&[5, 4, 6], 15)), &opts).is_ok());
        let err = solve_game(7, Some(&prev(&[5, 4, 6], 14)), &opts).unwrap_err();
        assert!(err.contains("scores 15, not the 14"), "{err}");
        let err = solve_game(7, Some(&prev(&[5, 4, 4], 13)), &opts).unwrap_err();
        assert!(err.contains("not a legal game of n=6"), "{err}");
    }

    #[test]
    fn bad_board_sizes_are_errors() {
        let opts = SolveOptions { optimal_path: None, ..SolveOptions::default() };
        assert!(solve_game(0, None, &opts).is_err());
        assert!(solve_game(MAX_N + 1, None, &opts).unwrap_err().contains("beyond"));
        assert!(solve_range(5, 4, &opts, None).is_err());
        assert!(solve_range(0, 4, &opts, None).is_err());
    }
}
