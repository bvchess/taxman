//! The per-game pipeline and the range driver: what `taxman solve` runs.
//!
//! [`solve_game`] solves one board size and [`solve_range`] a range of them, each built on the one before.  They
//! produce [`GameResult`]s: the game, its score, how it was found ([`Method`]) and how its optimality is
//! established (a [`Proof`] and a *taint*, see [`crate::results`]).  Around them are the options ([`SolveOptions`])
//! and the previous game ([`Prev`]).  The tactics are in [`crate::tactics`], the analysis of a board that every
//! search starts from in [`crate::board`], the search in [`crate::search`] and the files in [`crate::results`].
//!
//! # One game
//!
//! The steps, in order (DESIGN.md, "Per-game pipeline", has the same steps with the Java classes they replace):
//!
//! 1. The tables of the board `1..=n`, and the set width `W` for it (`with_width!`, widened to the width of the
//!    game's checkpoint if there is one).
//! 2. The *prime shortcut* (tactic `prime_shortcut`): a prime `n` plays the game of `n − 1` with its first move
//!    replaced by `n` (method `prime`).
//! 3. The board's frames ([`Board`]).  With no promotion candidates the game with no promotions is the only one
//!    (method `trivial`).
//! 4. `reuse_prev` (an unproven tactic): keep the previous game's promotions that are still possible and accept
//!    the game they make if it scores what the previous game suggests (method `reuse`).
//! 5. Otherwise the search (method `search`): [`Board::prepare`] (or [`Board::prepare_for_resume`]), then
//!    [`search::search`] for the promotion set with the largest sum.  It starts from the better of the greedy set
//!    and the previous game's promotions that are candidates here (`seed_incumbent`, [`Board::seed`]) and stops
//!    as soon as it reaches `n + opt(n − 1) − in_the_bag` (`prev_cap`, [`Board::promotion_cap`]).  The promotions
//!    become a game through the hives ([`solution_for_promotions`]).
//! 6. `conclude`: the game is checked before it counts, and given its proof and taint.
//! 7. Recording (a range only): the result is appended to the results file and synced, and only then is the
//!    game's checkpoint deleted.
//!
//! Steps 2 and 4 need the previous game and are skipped by `force_search`, and when a checkpoint of the game
//! exists: a search that was interrupted is resumed, never replaced by a derivation.
//!
//! The previous game is checked before anything uses it: it must replay legally to its recorded score, be over
//! (no legal move left) and open with the largest prime, as every optimal game does.  A run with no unproven
//! tactic on refuses a previous game whose own proof rests on something unproven unless `allow_tainted_prev`.
//!
//! `conclude` checks that the game replays legally ([`replay`]); that a completed game is also over and opens with
//! the largest prime; that a game from the frame model scores `in_the_bag` plus its promotions; that no game
//! scores more than `n + score(n − 1)` (nor its promotions exceed the cap), which would prove the previous game
//! was not optimal, and that a completed game does not score less than the previous one; and, for a completed
//! game, the comparison with `optimal.json` when that has the game.  Each failure is a hard error (the
//! `optimal.json` comparison unless the options allow a mismatch), and a game that fails after it was found is
//! recorded, with its promotions, in the results file's [`rejected_path`](crate::results::rejected_path).
//!
//! # A range
//!
//! [`solve_range_each`] solves the board sizes `lo..=hi` in order.  The previous game of each is the one just
//! solved; for `lo` it is the results file's game of `lo − 1`, else `optimal.json`'s, else there is none and the
//! game is solved *standalone*, with no seed and no cap (and a note on stderr says so).  Every result is appended to the results file (one JSON
//! object per line) as soon as it is done, so that a restarted run skips what is finished; the file is checked
//! when it is opened ([`ResultsFile::open`]).  A game whose search a limit or a signal stopped is recorded as
//! incomplete, keeps its checkpoint, and ends the range.

use crate::accelerate::{prime_shortcut, reuse_prev, solution_for_promotions};
use crate::bits::{width_for, words_for, Bits};
use crate::board::Board;
use crate::engine::{self, Counters, EngineConfig, Stats};
use crate::game::{replay, replay_finished, Optimal};
use crate::hives::Apiary;
use crate::json::Json;
use crate::numtheory::Tables;
use crate::paths;
use crate::results::{
    build_json, moves_digest, own_taint, taint_json, taint_of, Proof, ResultsFile, Taint, OPTIMAL_JSON, SCHEMA,
};
use crate::search::{self, stat, Revise, SearchTactics};
use crate::tactics::Tactics;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// The largest board the set widths cover (`W = 64`, the widest of [`crate::bits::WIDTHS`]).
pub const MAX_N: u32 = 64 * 64 - 1;

// ---------------------------------------------------------------------------------------------------------
// Options and results

/// Which table of optimal games a run compares its scores with and takes previous games from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum OptimalSource {
    /// The table of optimal games in the repository, found when the run starts ([`paths::optimal_json`]).
    #[default]
    Repository,
    /// This file (`--optimal FILE`).
    File(PathBuf),
    /// No table (`--optimal none`): scores are not compared, and a range that starts after 1 has no previous game
    /// to start from unless its results file has one.
    None,
}

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
    /// Per game and invocation; an unfinished search keeps its checkpoint.  A limit that has run out before a
    /// search starts, zero above all, stops it before it does any work ([`EngineConfig::time_limit`]).
    pub time_limit: Option<Duration>,
    pub node_limit: Option<u64>,
    /// Most queued search units kept in memory; the rest spill to disk.
    pub queue_mem_cap: usize,
    /// Where queued units spill to; by default `spill` in the checkpoint directory, else a temporary directory.
    pub spill_dir: Option<PathBuf>,
    /// Search even where the previous game would give the answer (skips `prime_shortcut` and `reuse_prev`).
    pub force_search: bool,
    /// The table to compare scores with and to take previous games from.
    pub optimal: OptimalSource,
    /// Record a score that differs from `optimal.json` (as `optimal_match: false`) instead of failing.
    pub allow_optimal_mismatch: bool,
    /// Let a run with no unproven tactic on build on a previous game whose proof rests on something unproven;
    /// the results whose proofs rest on that game inherit its taint.
    pub allow_tainted_prev: bool,
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
            optimal: OptimalSource::default(),
            allow_optimal_mismatch: false,
            allow_tainted_prev: false,
        }
    }
}

impl SolveOptions {
    /// Loads the table of optimal games, if there is one.
    ///
    /// # Errors
    ///
    /// If the table is the repository's and the repository cannot be found (see [`paths`]), or the file cannot be
    /// read.
    pub fn load_optimal(&self) -> Result<Option<Optimal>, String> {
        match &self.optimal {
            OptimalSource::Repository => Optimal::load(paths::optimal_json()?).map(Some),
            OptimalSource::File(path) => Optimal::load(path).map(Some),
            OptimalSource::None => Ok(None),
        }
    }

    /// The known games: the table of optimal games with the completed games of the results files `results` added
    /// (they replace the table's moves for the same board sizes; an unfinished search's line is skipped).  This is
    /// how a search past the table's last board size gets the previous game it is seeded and capped from.
    ///
    /// # Errors
    ///
    /// As [`load_optimal`](Self::load_optimal), if a results file cannot be read, and if a game of a results file
    /// scores differently from one the table already has.
    pub fn known_games(&self, results: &[PathBuf]) -> Result<Optimal, String> {
        let mut known = match self.load_optimal()? {
            Some(optimal) => optimal,
            None => Optimal::from_json(&Json::Arr(Vec::new()))?,
        };
        for path in results {
            let before: Vec<(u32, u64)> = known.games().map(|(n, score, _)| (n, score)).collect();
            known = known.with_results(path)?;
            if let Some(&(n, score)) = before.iter().find(|&&(n, score)| known.score(n) != Some(score)) {
                let now = known.score(n).map_or_else(|| "none".to_owned(), |s| s.to_string());
                return Err(format!("{}: n={n} scores {now}, but the known games have {score}", path.display()));
            }
        }
        Ok(known)
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
    /// The name in result lines.
    pub fn as_str(self) -> &'static str {
        match self {
            PrevSource::Run => "run",
            PrevSource::Results => "results",
            PrevSource::Optimal => "optimal",
        }
    }

    /// Where it came from, for a message: "this run", "the results file" or "optimal.json".
    pub fn whence(self) -> &'static str {
        match self {
            PrevSource::Run => "this run",
            PrevSource::Results => "the results file",
            PrevSource::Optimal => OPTIMAL_JSON,
        }
    }
}

/// The solution of game `n − 1`.
#[derive(Clone, Debug)]
pub struct Prev {
    pub moves: Vec<u32>,
    pub score: u64,
    pub source: PrevSource,
    /// How its optimality is established; `None` if that is not known (`optimal.json` does not say).
    pub proof: Option<Proof>,
    /// The unproven things its optimality rests on (see [`crate::results`]).
    pub taint: Taint,
}

impl Prev {
    /// The game for board size `n` in `optimal`, if it has one.  Its taint is `optimal.json`.
    pub fn from_optimal(optimal: &Optimal, n: u32) -> Option<Prev> {
        let (moves, score) = (optimal.moves(n)?, optimal.score(n)?);
        let taint = Taint::from([OPTIMAL_JSON.to_owned()]);
        Some(Prev { moves: moves.to_vec(), score, source: PrevSource::Optimal, proof: None, taint })
    }

    /// A completed result of this run.
    fn from_result(result: &GameResult) -> Prev {
        Prev {
            moves: result.moves.clone(),
            score: result.score,
            source: PrevSource::Run,
            proof: result.proof,
            taint: result.taint.clone(),
        }
    }

    /// What a result records about the previous game it was built from.
    fn info(&self, n: u32) -> PrevInfo {
        PrevInfo {
            n: n - 1,
            score: self.score,
            source: self.source,
            moves_digest: moves_digest(&self.moves),
            proof: self.proof,
            taint: self.taint.clone(),
        }
    }

    /// Checks that the game can be an optimal game of `n − 1`: legal, scoring what it is given as, over, and
    /// opening with the largest prime.  (Optimal itself cannot be checked; see the tripwires in [`conclude`].)
    fn check(&self, n: u32) -> Result<(), String> {
        let before = n - 1;
        let whence = self.source.whence();
        match replay(before, &self.moves) {
            Ok(score) if score == self.score => {}
            Ok(score) => {
                return Err(format!("the previous game scores {score}, not the {} it is given as", self.score))
            }
            Err(e) => return Err(format!("the previous game is not a legal game of n={before}: {e}")),
        }
        replay_finished(before, &self.moves)
            .map(|_| ())
            .map_err(|e| format!("the previous game (n={before}, from {whence}) cannot be optimal: {e}"))
    }
}

/// What a result records about the game of `n − 1` it was built from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrevInfo {
    pub n: u32,
    pub score: u64,
    pub source: PrevSource,
    /// [`moves_digest`] of its moves.
    pub moves_digest: String,
    pub proof: Option<Proof>,
    pub taint: Taint,
}

impl PrevInfo {
    fn to_json(&self) -> Json {
        Json::object([
            ("n", Json::from(self.n)),
            ("score", Json::from(self.score)),
            ("source", Json::from(self.source.as_str())),
            ("moves_digest", Json::from(self.moves_digest.as_str())),
            ("proof", self.proof.map_or(Json::Null, |p| Json::from(p.as_str()))),
            ("taint", taint_json(&self.taint)),
        ])
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
    pub const ALL: [Method; 4] = [Method::Trivial, Method::Prime, Method::Reuse, Method::Search];

    pub fn as_str(self) -> &'static str {
        match self {
            Method::Trivial => "trivial",
            Method::Prime => "prime",
            Method::Reuse => "reuse",
            Method::Search => "search",
        }
    }

    /// The method [`as_str`](Self::as_str) names.
    pub fn parse(name: &str) -> Option<Method> {
        Method::ALL.into_iter().find(|method| method.as_str() == name)
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
    /// The branch and bound over every invocation of a resumed search (see `SearchResult::time_all_runs`).
    pub search_all_runs: Duration,
    /// The whole game (this invocation only).
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
            ("search_all_runs", secs(self.search_all_runs)),
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
    /// The game's score; for an incomplete search, that of the best game found so far (written as
    /// `best_so_far`, with `score` null).
    pub score: u64,
    pub moves: Vec<u32>,
    /// The numbers promoted, ascending.
    pub promotions: Vec<u32>,
    pub method: Method,
    /// False when a time or node limit stopped the search (its checkpoint is kept and `score` is only a lower
    /// bound).
    pub completed: bool,
    /// How the game's optimality is established; `None` when incomplete.
    pub proof: Option<Proof>,
    /// The search stopped because its incumbent reached the `prev_cap` ceiling.
    pub reached_cap: bool,
    /// The unproven things the proof rests on, here or in the games of `n − 1` it rests on (see
    /// [`crate::results`]); empty when incomplete.
    pub taint: Taint,
    /// What the game with no promotions scores, for the games that come from the frame model (not prime ones).
    pub in_the_bag: Option<u64>,
    /// Search nodes, over all the runs of a resumed search.
    pub nodes: u64,
    /// The search's counters (see [`search::stat`]) and the engine's activity.
    pub stats: Stats,
    pub counters: Counters,
    pub setup: Option<SearchSetup>,
    pub phases: Phases,
    /// The invocations that ran this game's search: 1, plus those of an interrupted search it resumed (as many
    /// as the results file records, at least 1; an invocation that was killed leaves no record, but its work up
    /// to its last checkpoint is in `nodes` and `phases.search_all_runs`).
    pub invocations: u32,
    /// Of the last invocation.
    pub threads: usize,
    pub tactics: Tactics,
    /// The limits and options of the last invocation.
    pub time_limit: Option<Duration>,
    pub node_limit: Option<u64>,
    pub force_search: bool,
    pub allow_tainted_prev: bool,
    pub allow_optimal_mismatch: bool,
    /// The table of optimal games, if one was used: `default` for the repository's ([`OptimalSource::Repository`]),
    /// else the path given.
    pub optimal_file: Option<String>,
    /// Whether the score equals `optimal.json`'s; `None` if the game is incomplete or the file lacks `n`.
    pub optimal_match: Option<bool>,
    pub prev_source: Option<PrevSource>,
    /// The game of `n − 1` this one was built from.
    pub prev: Option<PrevInfo>,
}

impl GameResult {
    fn new(n: u32, opts: &SolveOptions, prev: Option<&Prev>) -> Self {
        let optimal_file = match &opts.optimal {
            OptimalSource::Repository => Some("default".to_owned()),
            OptimalSource::File(path) => Some(path.display().to_string()),
            OptimalSource::None => None,
        };
        GameResult {
            n,
            score: 0,
            moves: Vec::new(),
            promotions: Vec::new(),
            method: Method::Search,
            completed: true,
            proof: None,
            reached_cap: false,
            taint: Taint::new(),
            in_the_bag: None,
            nodes: 0,
            stats: Stats::default(),
            counters: Counters::default(),
            setup: None,
            phases: Phases::default(),
            invocations: 1,
            threads: opts.threads,
            tactics: opts.tactics,
            time_limit: opts.time_limit,
            node_limit: opts.node_limit,
            force_search: opts.force_search,
            allow_tainted_prev: opts.allow_tainted_prev,
            allow_optimal_mismatch: opts.allow_optimal_mismatch,
            optimal_file,
            optimal_match: None,
            prev_source: prev.map(|p| p.source),
            prev: prev.map(|p| p.info(n)),
        }
    }

    /// The search's counters by name.
    pub fn stats_json(&self) -> Json {
        Json::object(STAT_NAMES.map(|(name, i)| (name, Json::from(self.stats.extra[i]))))
    }

    /// The result as one JSON object (schema [`SCHEMA`]; [`crate::results`] describes the provenance fields).
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
        // An incomplete game's score is only the best so far, so it does not go where a final score goes.
        let (score, best_so_far) = if self.completed {
            (Json::from(self.score), None)
        } else {
            (Json::Null, Some(("best_so_far", Json::from(self.score))))
        };
        let limits = Json::object([
            ("seconds", self.time_limit.map_or(Json::Null, |d| Json::Num(d.as_secs_f64()))),
            ("nodes", self.node_limit.map_or(Json::Null, Json::from)),
        ]);
        let options = Json::object([
            ("force_search", Json::Bool(self.force_search)),
            ("allow_tainted_prev", Json::Bool(self.allow_tainted_prev)),
            ("allow_optimal_mismatch", Json::Bool(self.allow_optimal_mismatch)),
            ("optimal_file", self.optimal_file.as_deref().map_or(Json::Null, Json::from)),
        ]);
        let fields = [("schema", Json::from(SCHEMA)), ("n", Json::from(self.n)), ("score", score)]
            .into_iter()
            .chain(best_so_far)
            .chain([
                ("moves", Json::array(self.moves.iter().copied())),
                ("promotions", Json::array(self.promotions.iter().copied())),
                ("method", Json::from(self.method.as_str())),
                ("completed", Json::Bool(self.completed)),
                ("proof", self.proof.map_or(Json::Null, |p| Json::from(p.as_str()))),
                ("reached_cap", Json::Bool(self.reached_cap)),
                ("taint", if self.completed { taint_json(&self.taint) } else { Json::Null }),
                ("in_the_bag", self.in_the_bag.map_or(Json::Null, Json::from)),
                ("prev", self.prev.as_ref().map_or(Json::Null, PrevInfo::to_json)),
                ("nodes", Json::from(self.nodes)),
                ("stats", self.stats_json()),
                ("engine", engine),
                ("setup", setup),
                ("seconds", self.phases.to_json()),
                ("invocations", Json::from(self.invocations)),
                ("threads", Json::from(self.threads)),
                ("tactics", Json::from(self.tactics.description())),
                ("tactic_switches", self.tactics.to_json()),
                ("unproven_tactics_on", Json::array(self.tactics.unproven_on())),
                ("limits", limits),
                ("options", options),
                ("optimal_match", self.optimal_match.map_or(Json::Null, Json::Bool)),
                ("prev_source", self.prev_source.map_or(Json::Null, |s| Json::from(s.as_str()))),
                ("build", build_json()),
            ]);
        Json::object(fields)
    }

    /// One line for a person: board size, score, method, nodes, time, the comparison with `optimal.json`, the
    /// proof, and what it rests on that is unproven.
    pub fn summary_line(&self) -> String {
        let optimal = match (self.completed, self.optimal_match) {
            (false, _) => "incomplete",
            (true, Some(true)) => "optimal ok",
            (true, Some(false)) => "OPTIMAL MISMATCH",
            (true, None) => "optimal unknown",
        };
        let proof = self.proof.map_or_else(String::new, |p| format!("  proof={}", p.as_str()));
        let taint = if self.taint.is_empty() {
            String::new()
        } else {
            format!(" (rests on {})", self.taint.iter().cloned().collect::<Vec<_>>().join(", "))
        };
        format!(
            "n={:<5} score={:<9} {:<7} nodes={:<11} {:>9.3} s  {optimal}{proof}{taint}",
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

/// Why a game could not be solved.
enum Failure {
    /// Before there was a game to show for it.
    Error(String),
    /// The game was found but failed a check: the result, with the moves and promotions found, is the evidence.
    Rejected(Box<GameResult>, String),
}

impl Failure {
    /// The message; a rejected game's promotions go with it, so that they are not lost.
    fn message(self) -> String {
        match self {
            Failure::Error(message) => message,
            Failure::Rejected(result, message) => format!("{message} (promotions {:?})", result.promotions),
        }
    }
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Failure::Error(message)
    }
}

/// A check that failed after the game was found.
fn reject(result: GameResult, reason: String) -> Result<GameResult, Failure> {
    Err(Failure::Rejected(Box::new(result), reason))
}

/// Solves board size `n`.  `prev` is the solution of `n − 1` if known; `opts.optimal` is loaded for the
/// comparison (use [`solve_game_with`] to load it once for many games).
pub fn solve_game(n: u32, prev: Option<&Prev>, opts: &SolveOptions) -> Result<GameResult, String> {
    solve_game_with(n, prev, opts, opts.load_optimal()?.as_ref())
}

/// [`solve_game`] with `optimal.json` already loaded (`opts.optimal` is not used).  The checkpoint of a
/// completed game is deleted once the game has passed its checks.
pub fn solve_game_with(
    n: u32,
    prev: Option<&Prev>,
    opts: &SolveOptions,
    optimal: Option<&Optimal>,
) -> Result<GameResult, String> {
    let result = solve_one(n, prev, opts, optimal, 0).map_err(Failure::message)?;
    if result.completed {
        discard_checkpoint(opts, n);
    }
    Ok(result)
}

type SolveWithWidth = fn(u32, Option<&Prev>, &SolveOptions, Option<&Optimal>, u32) -> Result<GameResult, Failure>;

/// Solves board size `n` and leaves its checkpoint, if any, to the caller.  `earlier_runs` is the number of
/// invocations a results file records for an unfinished search of `n`.
fn solve_one(
    n: u32,
    prev: Option<&Prev>,
    opts: &SolveOptions,
    optimal: Option<&Optimal>,
    earlier_runs: u32,
) -> Result<GameResult, Failure> {
    if n == 0 {
        return Err(Failure::Error("n must be at least 1".into()));
    }
    // A search that was interrupted resumes at the set width it was made with, which an older build may have
    // chosen otherwise for this game (if this build has that width and it holds the board; if not, resuming
    // says why it cannot).
    let checkpoint = opts.checkpoint_file(n).filter(|path| path.exists());
    let made_with = checkpoint.and_then(|path| search::checkpoint_width(&path)).filter(|&w| width_for(w) == Some(w));
    let words = words_for(n).max(made_with.unwrap_or(0));
    let beyond = || Failure::Error(format!("n={n} is beyond the largest supported board, n={MAX_N}"));
    let solve: SolveWithWidth = crate::with_words!(words, W => solve_with_width::<W>, else return Err(beyond()));
    solve(n, prev, opts, optimal, earlier_runs).map_err(|failure| match failure {
        Failure::Error(e) => Failure::Error(format!("n={n}: {e}")),
        Failure::Rejected(result, e) => Failure::Rejected(result, format!("n={n}: {e}")),
    })
}

fn solve_with_width<const W: usize>(
    n: u32,
    prev: Option<&Prev>,
    opts: &SolveOptions,
    optimal: Option<&Optimal>,
    earlier_runs: u32,
) -> Result<GameResult, Failure> {
    let start = Instant::now();
    let tactics = opts.tactics;
    if let Some(prev) = prev {
        // The derivations and the cap trust the previous game, so make sure it is what it says it is.
        prev.check(n)?;
    }
    let tables = Tables::new(n);
    let mut result = GameResult::new(n, opts, prev);
    if optimal.is_none() {
        result.optimal_file = None;
    }
    let context = Context { prev, opts, optimal, start };
    // A search that was interrupted is resumed, whatever else could have produced the game.
    let checkpoint = opts.checkpoint_file(n).filter(|path| path.exists());
    let derive = !opts.force_search && checkpoint.is_none();

    if derive && tactics.prime_shortcut && tables.is_prime(n) {
        if let Some(prev) = prev {
            trust(prev, opts)?;
            let moves = prime_shortcut(n, &prev.moves);
            let candidates = Apiary::<W>::new(&tables, Bits::EMPTY).promotion_candidates();
            let promotions = candidates & moves.iter().copied().collect::<Bits<W>>();
            result.method = Method::Prime;
            return conclude(result, moves, promotions.to_vec(), context);
        }
    }

    let board = Board::<W>::new(tables);
    result.phases.frames = board.frames_time;
    result.in_the_bag = Some(board.in_the_bag);
    if board.is_trivial() {
        result.method = Method::Trivial;
        return conclude(result, board.plain, Vec::new(), context);
    }

    if let Some(prev) = prev.filter(|_| tactics.prev_cap) {
        trust(prev, opts)?;
    }
    if derive && tactics.reuse_prev {
        if let Some(derived) = prev.and_then(|p| reuse_prev(&board.tables, &board.frames, &p.moves)) {
            result.method = Method::Reuse;
            return conclude(result, derived.moves, derived.promotions, context);
        }
    }

    let promotions = search_game(&board, prev, opts, checkpoint, earlier_runs, &mut result)?;
    let moves = solution_for_promotions::<W>(&board.tables, promotions);
    conclude(result, moves, promotions.to_vec(), context)
}

/// What [`conclude`] checks a game against.
#[derive(Clone, Copy)]
struct Context<'a> {
    prev: Option<&'a Prev>,
    opts: &'a SolveOptions,
    optimal: Option<&'a Optimal>,
    start: Instant,
}

/// Refuses, in a run with no unproven tactic on, a previous game whose proof rests on something unproven, as
/// the game about to be solved could rest on it; `allow_tainted_prev` lets it through (the results whose proofs
/// rest on it then inherit its taint).
fn trust(prev: &Prev, opts: &SolveOptions) -> Result<(), Failure> {
    if prev.taint.is_empty() || !opts.tactics.unproven_on().is_empty() || opts.allow_tainted_prev {
        return Ok(());
    }
    let taint: Vec<&str> = prev.taint.iter().map(String::as_str).collect();
    Err(Failure::Error(format!(
        "the previous game (from {}) rests on {}, which is not proven, and a run with no unproven tactic on does \
         not build on it.  Start from a game that rests on nothing unproven (a run from n = 1 with these tactics \
         does), or pass --allow-tainted-prev to go on and mark the results that rest on it",
        prev.source.whence(),
        taint.join(", ")
    )))
}

/// Checks the game (see the module documentation), compares it with `optimal.json`, and completes the result
/// with its proof and taint.
fn conclude(
    mut result: GameResult,
    moves: Vec<u32>,
    promotions: Vec<u32>,
    context: Context,
) -> Result<GameResult, Failure> {
    let n = result.n;
    result.moves = moves;
    result.promotions = promotions;
    result.phases.total = context.start.elapsed();
    let method = result.method.as_str();
    // A completed game claims to be optimal, so it must be over and open with the largest prime.
    let refereed = if result.completed { replay_finished(n, &result.moves) } else { replay(n, &result.moves) };
    result.score = match refereed {
        Ok(score) => score,
        Err(e) => return reject(result, format!("the {method} game does not stand up: {e}")),
    };
    let score = result.score;
    let sum: u64 = result.promotions.iter().map(|&p| u64::from(p)).sum();
    if let Some(bag) = result.in_the_bag {
        if score != bag + sum {
            let why = "the game and the frame model disagree";
            return reject(
                result,
                format!("the {method} game scores {score}, not inTheBag {bag} + its promotions' {sum}: {why}"),
            );
        }
    }
    // opt(n) ≤ n + opt(n − 1), so going past the ceiling proves the previous game is not optimal.
    if let Some(cap) = result.setup.and_then(|s| s.cap) {
        if sum > cap {
            return reject(
                result,
                format!(
                    "the promotions' {sum} exceed the cap {cap} = n + score(n − 1) − inTheBag, which proves that the \
                     previous game is not optimal"
                ),
            );
        }
    }
    if let Some(prev) = context.prev {
        let ceiling = prev.score + u64::from(n);
        let whence = prev.source.whence();
        if score > ceiling {
            return reject(
                result,
                format!(
                    "the game scores {score}, more than n + score(n − 1) = {ceiling}, which proves that the previous \
                     game (from {whence}) is not optimal"
                ),
            );
        }
        if result.completed && score < prev.score {
            return reject(
                result,
                format!(
                    "the game scores {score}, less than the previous game's {} (from {whence}); a game of n − 1 is a \
                     game of n, so this one is not optimal",
                    prev.score
                ),
            );
        }
    }
    if !result.completed {
        return Ok(result);
    }
    if let Some(expected) = context.optimal.and_then(|o| o.score(n)) {
        result.optimal_match = Some(expected == score);
        if expected != score && !context.opts.allow_optimal_mismatch {
            return reject(result, format!("scored {score} but optimal.json says {expected}"));
        }
    }
    let own = own_taint(result.method, result.setup.map(|s| s.revise), result.tactics.search.legacy_bound);
    let proof = Proof::of(result.method, true, result.reached_cap).expect("a completed game has a proof");
    result.proof = Some(proof);
    result.taint = taint_of(own, proof, context.prev.map(|p| &p.taint));
    Ok(result)
}

/// The search path: prepares the game (or the checkpoint's game), searches, and fills `result` with what the
/// search did.  Returns the best promotion set found.  The checkpoint is left in place (see [`solve_one`]).
fn search_game<const W: usize>(
    board: &Board<W>,
    prev: Option<&Prev>,
    opts: &SolveOptions,
    checkpoint: Option<PathBuf>,
    earlier_runs: u32,
    result: &mut GameResult,
) -> Result<Bits<W>, Failure> {
    let n = board.n();
    let tactics = opts.tactics;
    let resuming = checkpoint.is_some();
    let prepared = match &checkpoint {
        Some(path) => {
            let p = board.prepare_for_resume(path, opts.threads).map_err(|e| {
                format!("cannot resume from {}: {e} (delete the file to start the game over)", path.display())
            })?;
            if p.tactics != tactics.search {
                return Err(Failure::Error(different_tactics(path, p.tactics, tactics)));
            }
            p
        }
        None => {
            // The two-frame search of `revise_freedoms` stays within this invocation's limits too.
            let limits = EngineConfig {
                threads: opts.threads,
                time_limit: opts.time_limit,
                node_limit: opts.node_limit,
                ..EngineConfig::default()
            };
            board.prepare_within(tactics.search, &limits)?
        }
    };
    result.phases.frames += prepared.timings.build;
    result.phases.preclusions = prepared.timings.preclusions;
    result.phases.greedy = prepared.timings.greedy;
    result.phases.revise = prepared.timings.revise;

    let cap = prev.filter(|_| tactics.prev_cap).and_then(|prev| board.promotion_cap(prev.score));
    // A resumed search has its incumbent in the checkpoint.
    let seed = prev.filter(|_| tactics.seed_incumbent && !resuming).map(|prev| board.seed(&prev.moves));
    result.setup = Some(SearchSetup {
        frames: board.frames.frames.len(),
        candidates: board.candidates().len(),
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
        // Deleted only once the result is safe (`solve_game_with`, `solve_range_each`).
        keep_completed_checkpoint: true,
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
    result.reached_cap = found.reached_cap;
    result.nodes = found.stats.nodes;
    result.stats = found.stats;
    result.counters = found.counters;
    result.phases.search = found.time;
    result.phases.search_all_runs = found.time_all_runs;
    if resuming {
        // At least one invocation made the checkpoint, even if it left no line in a results file.
        result.invocations = earlier_runs.max(1) + 1;
    }
    Ok(found.promotions)
}

/// The error for a checkpoint made with other search tactics than this run's: what differs, and the flags that
/// resume it.
fn different_tactics(checkpoint: &Path, made: SearchTactics, now: Tactics) -> String {
    let then = Tactics { search: made, ..now };
    let differ: Vec<_> = then.settings().into_iter().zip(now.settings()).filter(|(a, b)| a != b).collect();
    let what: Vec<String> =
        differ.iter().map(|((name, there), (_, here))| format!("{name} is {there}, not {here}")).collect();
    let flags: Vec<String> = differ.iter().map(|((name, there), _)| format!("--tactic {name}={there}")).collect();
    format!(
        "{} was made with different search tactics: {}; resume it with {}, or delete it to start the game over",
        checkpoint.display(),
        what.join(", "),
        flags.join(" ")
    )
}

/// Deletes game `n`'s checkpoint, if checkpointing is on.  Called only once the game's result is safe: in the
/// results file, or (without one) handed to the caller.
fn discard_checkpoint(opts: &SolveOptions, n: u32) {
    if let Some(path) = opts.checkpoint_file(n) {
        if let Err(e) = engine::remove_checkpoint(&path) {
            let _ = writeln!(std::io::stderr(), "warning: cannot remove the checkpoint of a finished game: {e}");
        }
    }
}

/// Where the engine spills to, and whether this function chose the directory (and so may remove it).  A
/// temporary directory is the game's own: games solved at the same time (by threads of one process) do not share
/// one, or one's removing it would fail the other's creating it.
fn spill_dir(opts: &SolveOptions, n: u32) -> (PathBuf, bool) {
    static TEMP_DIRS: AtomicU64 = AtomicU64::new(0);
    if let Some(dir) = &opts.spill_dir {
        (dir.clone(), false)
    } else if let Some(dir) = &opts.checkpoint_dir {
        (dir.join("spill"), true)
    } else {
        let name = format!("taxman-spill-{}-{}-n{n}", std::process::id(), TEMP_DIRS.fetch_add(1, Ordering::Relaxed));
        (std::env::temp_dir().join(name), true)
    }
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

/// The completed game of `n` in the results file, as the previous game of `n + 1`.
fn stored_prev(results: &ResultsFile, n: u32) -> Option<Prev> {
    let game = results.completed(n)?;
    Some(Prev {
        moves: game.moves.clone(),
        score: game.score,
        source: PrevSource::Results,
        proof: Some(game.proof),
        taint: game.taint.clone(),
    })
}

/// The solution of game `n − 1` from the results file, else from `optimal.json`.
fn previous_game(n: u32, results: Option<&ResultsFile>, optimal: Option<&Optimal>) -> Option<Prev> {
    let before = n.checked_sub(1).filter(|&before| before >= 1)?;
    results.and_then(|r| stored_prev(r, before)).or_else(|| optimal.and_then(|o| Prev::from_optimal(o, before)))
}

/// Says on stderr that game `n` has no previous game, so that it is solved standalone: correct, but without the
/// seed and the ceiling that the game of `n − 1` gives, so it costs more (`bench/curve/README.md` measures how
/// much).
fn note_standalone(n: u32) {
    let _ = writeln!(
        std::io::stderr(),
        "note: no previous game for n={n} (the game of {} is in neither the results file nor optimal.json): solving \
         it standalone, with no seed and no ceiling, which costs more; to chain, give --out with the earlier games",
        n - 1
    );
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
/// `lo − 1` comes from the results file, else from `optimal.json`.  The results file is checked when it is
/// opened ([`ResultsFile::open`]); games already completed in it are skipped (and a checkpoint left for one is
/// deleted).  Every other result is appended to the file (if there is one), then given to `on_game`, and only
/// then is its checkpoint deleted.  A game that fails a check after it was found goes to the file's
/// [`rejected_path`](crate::results::rejected_path) instead, and the run fails.  The range stops at a game whose
/// search did not complete.
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
    let mut results = results_path.map(|path| ResultsFile::open(path, optimal.as_ref())).transpose()?;
    if let Some(file) = &results {
        // Left by a crash between saving a result and deleting its checkpoint.
        for n in (lo..=hi).filter(|&n| file.completed(n).is_some()) {
            if opts.checkpoint_file(n).is_some_and(|path| path.exists()) {
                discard_checkpoint(opts, n);
            }
        }
    }
    let mut summary = RangeSummary::default();
    let mut prev: Option<Prev> = None;
    for n in lo..=hi {
        if let Some(stored) = results.as_ref().and_then(|r| stored_prev(r, n)) {
            prev = Some(stored);
            summary.skipped += 1;
            continue;
        }
        let before = if n == lo { previous_game(n, results.as_ref(), optimal.as_ref()) } else { prev.take() };
        if n == lo && n > 1 && before.is_none() {
            note_standalone(n);
        }
        let earlier_runs = results.as_ref().map_or(0, |r| r.earlier_runs(n));
        let result = match (solve_one(n, before.as_ref(), opts, optimal.as_ref(), earlier_runs), &results) {
            (Ok(result), _) => result,
            (Err(Failure::Rejected(result, reason)), Some(file)) => {
                let path = file.reject(&result, &reason)?;
                return Err(format!("{reason}; the game is recorded in {}", path.display()));
            }
            (Err(failure), _) => return Err(failure.message()),
        };
        if let Some(results) = &mut results {
            results.append(&result)?;
        }
        on_game(&result);
        summary.solved += 1;
        if !result.completed {
            summary.stopped_at = Some(n);
            break;
        }
        discard_checkpoint(opts, n);
        prev = Some(Prev::from_result(&result));
    }
    Ok(summary)
}

// ---------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    /// The options of a test: one thread, and no table of optimal games.
    fn without_optimal() -> SolveOptions {
        SolveOptions { optimal: OptimalSource::None, ..SolveOptions::default() }
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
        let contents = crate::results::read_results(&format!("{json}\n"), None).unwrap();
        let stored = &contents.games[&10];
        assert_eq!((stored.score, &stored.moves, stored.proof), (r.score, &r.moves, Proof::Trivial));
    }

    #[test]
    fn the_results_file_skips_what_is_done_and_survives_a_torn_line() {
        let dir = TempDir::new("results");
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
        assert!(ResultsFile::open(&path, None).is_err());
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
        let dir = TempDir::new("mismatch");
        let path = dir.join("optimal.json");
        fs::write(&path, r#"[{"n":4,"score":8,"moves":[4,3]}]"#).unwrap();
        let mut opts =
            SolveOptions { threads: 1, optimal: OptimalSource::File(path.clone()), ..SolveOptions::default() };
        let err = solve_game(4, None, &opts).unwrap_err();
        assert!(err.contains("scored 7") && err.contains("says 8"), "{err}");
        opts.allow_optimal_mismatch = true;
        let r = solve_game(4, None, &opts).unwrap();
        assert_eq!((r.score, r.optimal_match), (7, Some(false)));
        assert_eq!(r.optimal_file, Some(path.display().to_string()));
        opts.optimal = OptimalSource::None;
        let r = solve_game(4, None, &opts).unwrap();
        assert_eq!((r.optimal_match, r.optimal_file), (None, None));
    }

    #[test]
    fn the_known_games_are_the_table_with_the_results_files_added() {
        let dir = TempDir::new("known");
        let table = dir.join("optimal.json");
        fs::write(&table, r#"[{"n":4,"score":7,"moves":[3,4]}]"#).unwrap();
        let line = |n: u32, score: u64, moves: &str| {
            format!("{{\"n\":{n},\"score\":{score},\"moves\":{moves},\"completed\":true}}\n")
        };
        let results = dir.join("results.jsonl");
        fs::write(&results, line(4, 7, "[2,3,4]") + &line(5, 12, "[5,4,3]")).unwrap();
        let opts = SolveOptions { optimal: OptimalSource::File(table.clone()), ..SolveOptions::default() };

        let known = opts.known_games(std::slice::from_ref(&results)).unwrap();
        // The results file's game replaces the table's, and adds a board size the table lacks.
        assert_eq!((known.score(4), known.moves(4)), (Some(7), Some(&[2, 3, 4][..])));
        assert_eq!(known.score(5), Some(12));
        assert_eq!(opts.known_games(&[]).unwrap().score(5), None);

        // A score that disagrees with the table's is an error, which names the file.
        fs::write(&results, line(4, 9, "[2,3,4]")).unwrap();
        let err = opts.known_games(std::slice::from_ref(&results)).err().expect("a disagreement");
        assert!(err.contains("results.jsonl") && err.contains("n=4 scores 9"), "{err}");

        // With no table, the results files are all there is.
        fs::write(&results, line(5, 12, "[5,4,3]")).unwrap();
        let known = without_optimal().known_games(&[results]).unwrap();
        assert_eq!((known.score(4), known.score(5)), (None, Some(12)));
        assert!(without_optimal().load_optimal().unwrap().is_none());
    }

    #[test]
    fn a_previous_game_that_does_not_check_out_is_refused() {
        let opts = without_optimal();
        let prev = |moves: &[u32], score| Prev {
            moves: moves.to_vec(),
            score,
            source: PrevSource::Run,
            proof: Some(Proof::Exhausted),
            taint: Taint::new(),
        };
        // [5, 4, 6] is the game for 6, scoring 15.
        assert!(solve_game(7, Some(&prev(&[5, 4, 6], 15)), &opts).is_ok());
        let err = solve_game(7, Some(&prev(&[5, 4, 6], 14)), &opts).unwrap_err();
        assert!(err.contains("scores 15, not the 14"), "{err}");
        let err = solve_game(7, Some(&prev(&[5, 4, 4], 13)), &opts).unwrap_err();
        assert!(err.contains("not a legal game of n=6"), "{err}");
    }

    #[test]
    fn bad_board_sizes_are_errors() {
        let opts = without_optimal();
        assert!(solve_game(0, None, &opts).is_err());
        assert!(solve_game(MAX_N + 1, None, &opts).unwrap_err().contains("beyond"));
        assert!(solve_range(5, 4, &opts, None).is_err());
        assert!(solve_range(0, 4, &opts, None).is_err());
    }
}
