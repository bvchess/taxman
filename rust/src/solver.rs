//! The per-game pipeline and the range driver: what `taxman solve` runs.
//!
//! [`solve_game`] solves one board size and [`solve_range`] a range of them, each built on the one before.  They
//! produce [`GameResult`]s: the game, its score, how it was found ([`Method`]) and how its optimality is established (a
//! [`Proof`], and what it assumes, see [`crate::results`]).  Around them are the options ([`SolveOptions`]) and the
//! previous game ([`Prev`]).  The tactics are in [`crate::tactics`], the analysis of a board that every search starts
//! from in [`crate::board`], the search in [`crate::search`] and the files in [`crate::results`].
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
//! 4. Otherwise the search (method `search`): [`Board::prepare`] (or [`Board::prepare_for_resume`]), then
//!    [`search::search`] for the promotion set with the largest sum.  It starts from the better of the greedy set
//!    and the previous game's promotions that are candidates here (`seed_incumbent`, [`Board::seed`]) and stops
//!    as soon as it reaches `n + opt(n − 1) − in_the_bag` (`prev_cap`, [`Board::promotion_cap`]).  The promotions
//!    become a game through the hives ([`solution_for_promotions`]).  Between the setup and the search, the
//!    tactic `same_search` may find that an earlier game already settled this search (method `repeat`, below).
//! 5. `conclude`: the game is checked before it counts, and given its proof and assumptions.
//! 6. Recording (a range only): the result is appended to the results file and synced, and only then is the
//!    game's checkpoint deleted.
//!
//! Step 2 and the repeat of step 4 are skipped by `force_search`, and when a checkpoint of the game exists: a search
//! that was interrupted is resumed, never replaced by a derivation.
//!
//! # Repeats of earlier searches
//!
//! Consecutive boards often pose exactly the same search problem ([`search::Problem`]: the frames, the candidates,
//! `max_total` and the factor graph among their numbers), only with another cap.  With the tactic `same_search`
//! (proven, on by default), a game whose problem an earlier game `m` of the range or of the results file posed, within
//! the [`SAME_SEARCH_WINDOW`] board sizes before it, and whose proof settles that problem's best promotion sum
//! ([`Proof::settles_search`]), is `m`'s promotions played on its own board, with no search: method `repeat`, proof
//! `same_search`, resting on `m`'s proof and its assumptions.  The game is found by the problem's digest and confirmed
//! by setting `m`'s problem up again and comparing the two; it then goes through `conclude` like any game.  DESIGN.md,
//! "Same search", has why it is optimal.
//!
//! The previous game is checked before anything uses it: it must replay legally to its recorded score, be over
//! (no legal move left) and open with the largest prime, as every optimal game does.
//!
//! `conclude` checks that the game replays legally ([`replay`]); that a completed game is also over and opens with
//! the largest prime; that a game from the frame model scores `in_the_bag` plus its promotions; that no game
//! scores more than `n + score(n − 1)` (nor its promotions exceed the cap), which would prove the previous game
//! was not optimal, and that a completed game does not score less than the previous one; and, for a completed
//! game, that its score is `optimal.json`'s when that has the game (the table built into the binary,
//! [`Optimal::built_in`]).  Each failure is a hard error, and a game that fails after it was found is recorded,
//! with its promotions, in the results file's [`rejected_path`](crate::results::rejected_path).
//!
//! # A range
//!
//! [`solve_range_each`] solves the board sizes `lo..=hi` in order.  The previous game of each is the one just
//! solved; for `lo` it is the results file's game of `lo − 1`, else, with [`SolveOptions::use_prev`],
//! `optimal.json`'s (assumed optimal, which the results that rest on it record), else there is none and the game is
//! solved *standalone*, with no seed and no cap (and a note on stderr says so).  Every result is appended to
//! the results file (one JSON object per line) as soon as it is done, so that a restarted run skips what is
//! finished; the file is checked when it is opened ([`ResultsFile::open`]).  A game whose search a limit or a signal
//! stopped is recorded as incomplete, keeps its checkpoint, and ends the range.  The games a game of the range may
//! repeat the search of are the file's and the range's own (a single game solved with [`solve_game`] has none).

use crate::accelerate::{prime_shortcut, solution_for_promotions};
use crate::bits::{width_for, words_for, Bits};
use crate::board::{search_problem, Board};
use crate::engine::{self, Counters, EngineConfig, Stats};
use crate::game::{replay, replay_finished, Optimal};
use crate::hives::Apiary;
use crate::json::Json;
use crate::numtheory::Tables;
use crate::results::{
    assumed_optimal, assumptions_json, assumptions_of, build_json, moves_digest, Assumptions, Proof, ResultsFile,
    SCHEMA,
};
use crate::search::{self, stat, Problem, ProblemSize, SearchTactics};
use crate::tactics::Tactics;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// The largest board the set widths cover (`W = 64`, the widest of [`crate::bits::WIDTHS`]).
pub const MAX_N: u32 = 64 * 64 - 1;

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
    /// Per game and invocation; an unfinished search keeps its checkpoint.  A limit that has run out before a
    /// search starts, zero above all, stops it before it does any work ([`EngineConfig::time_limit`]).
    pub time_limit: Option<Duration>,
    pub node_limit: Option<u64>,
    /// Most queued search units kept in memory; the rest spill to disk.
    pub queue_mem_cap: usize,
    /// Where queued units spill to; by default `spill` in the checkpoint directory, else a temporary directory.
    pub spill_dir: Option<PathBuf>,
    /// Search even where the previous game or an earlier search would give the answer (skips `prime_shortcut` and
    /// `same_search`).
    pub force_search: bool,
    /// Take the game of `n − 1` of a range's first game from `optimal.json` when neither the run nor the results file
    /// has it (`--use-prev`); the results whose proofs rest on it record that they assume it optimal.  Without it
    /// such a game is solved standalone.
    pub use_prev: bool,
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
            use_prev: false,
        }
    }
}

impl SolveOptions {
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
            PrevSource::Optimal => "optimal.json",
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
    /// What its proof assumes (see [`crate::results`]).
    pub assumes: Assumptions,
}

impl Prev {
    /// The game for board size `n` in `optimal`, if it has one.  The file records no proof, so the game is assumed
    /// optimal ([`assumed_optimal`]).
    pub fn from_optimal(optimal: &Optimal, n: u32) -> Option<Prev> {
        let (moves, score) = (optimal.moves(n)?, optimal.score(n)?);
        let assumes = Assumptions::from([assumed_optimal(n)]);
        Some(Prev { moves: moves.to_vec(), score, source: PrevSource::Optimal, proof: None, assumes })
    }

    /// A completed result of this run.
    fn from_result(result: &GameResult) -> Prev {
        Prev {
            moves: result.moves.clone(),
            score: result.score,
            source: PrevSource::Run,
            proof: result.proof,
            assumes: result.assumes.clone(),
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
            assumes: self.assumes.clone(),
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
    pub assumes: Assumptions,
}

impl PrevInfo {
    fn to_json(&self) -> Json {
        Json::object([
            ("n", Json::from(self.n)),
            ("score", Json::from(self.score)),
            ("source", Json::from(self.source.as_str())),
            ("moves_digest", Json::from(self.moves_digest.as_str())),
            ("proof", self.proof.map_or(Json::Null, |p| Json::from(p.as_str()))),
            ("assumes", assumptions_json(&self.assumes)),
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
    /// `same_search`: an earlier game's promotions, which settled the same search problem.
    Repeat,
    /// The branch-and-bound search.
    Search,
}

impl Method {
    pub const ALL: [Method; 4] = [Method::Trivial, Method::Prime, Method::Repeat, Method::Search];

    pub fn as_str(self) -> &'static str {
        match self {
            Method::Trivial => "trivial",
            Method::Prime => "prime",
            Method::Repeat => "repeat",
            Method::Search => "search",
        }
    }

    /// The method [`as_str`](Self::as_str) names.
    pub fn parse(name: &str) -> Option<Method> {
        Method::ALL.into_iter().find(|method| method.as_str() == name)
    }

    /// Whether the game set a search up (and its result line has a `setup`): a search, and a repeat, which set one
    /// up to compare it with an earlier game's.
    pub fn sets_up(self) -> bool {
        matches!(self, Method::Search | Method::Repeat)
    }
}

/// What a result records about the earlier game whose search problem it repeats (tactic `same_search`): that game's
/// board size, the digest of the problem they share, its best promotion sum, and that game's proof and assumptions,
/// which the repeat rests on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SameSearch {
    /// The board size of the earlier game.
    pub n: u32,
    /// [`Problem::digest`](crate::search::Problem::digest) of the search problem.
    pub digest: u64,
    /// The best promotion sum of the problem: the sum of the earlier game's promotions, which are this game's.
    pub promotion_sum: u64,
    pub proof: Proof,
    pub assumes: Assumptions,
}

impl SameSearch {
    fn to_json(&self) -> Json {
        Json::object([
            ("n", Json::from(self.n)),
            ("digest", Json::from(digest_text(self.digest))),
            ("promotion_sum", Json::from(self.promotion_sum)),
            ("proof", Json::from(self.proof.as_str())),
            ("assumes", assumptions_json(&self.assumes)),
        ])
    }
}

/// A search problem's digest as a result line writes it: 16 hexadecimal digits.
pub fn digest_text(digest: u64) -> String {
    format!("{digest:016x}")
}

/// The digest that [`digest_text`] wrote as `text`, if it is exactly that form of one.
pub fn parse_digest(text: &str) -> Option<u64> {
    let digest = u64::from_str_radix(text, 16).ok()?;
    (digest_text(digest) == text).then_some(digest)
}

/// Wall time of each phase of a game.
#[derive(Clone, Copy, Debug, Default)]
pub struct Phases {
    /// Hives, frames, and the search model.
    pub frames: Duration,
    pub greedy: Duration,
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
            ("greedy", secs(self.greedy)),
            ("search", secs(self.search)),
            ("search_all_runs", secs(self.search_all_runs)),
            ("total", secs(self.total)),
        ])
    }
}

/// What the search was set up with (only for games that were searched or that repeat an earlier search).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchSetup {
    pub frames: usize,
    pub candidates: u32,
    pub max_total: u32,
    pub greedy_sum: u64,
    /// The `prev_cap` ceiling on the promotion sum; `None` for a repeat, which runs no search to stop.
    pub cap: Option<u64>,
    /// [`Problem::digest`](crate::search::Problem::digest) of the search problem the setup poses.
    pub digest: u64,
}

impl SearchSetup {
    /// The size of the search problem the setup poses, as [`Problem::size`] gives it.
    pub fn size(&self) -> ProblemSize {
        ProblemSize { frames: self.frames as u32, candidates: self.candidates, max_total: self.max_total }
    }
}

/// The names of [`Stats::extra`], by the index [`search::stat`] gives each.
const STAT_NAMES: [(&str, usize); 4] = [
    ("promote_attempts", stat::PROMOTE_ATTEMPTS),
    ("promotions", stat::PROMOTIONS),
    ("children", stat::CHILDREN),
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
    /// What the proof assumes: what the game it rests on assumes, and so on down the chain (see
    /// [`crate::results`]); empty when incomplete.
    pub assumes: Assumptions,
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
    pub use_prev: bool,
    /// `Some(true)` when `optimal.json` has the game, whose score the game's equals (else it is rejected); `None`
    /// if the game is incomplete or the table lacks `n`.
    pub optimal_match: Option<bool>,
    /// The game of `n − 1` this one was built from.
    pub prev: Option<PrevInfo>,
    /// For a repeat (method [`Method::Repeat`], proof `same_search`), the earlier game whose search it repeats.
    pub same_search: Option<SameSearch>,
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
            proof: None,
            reached_cap: false,
            assumes: Assumptions::new(),
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
            use_prev: opts.use_prev,
            optimal_match: None,
            prev: prev.map(|p| p.info(n)),
            same_search: None,
        }
    }

    /// Where the game of `n − 1` this one was built from came from, if there was one.
    pub fn prev_source(&self) -> Option<PrevSource> {
        self.prev.as_ref().map(|p| p.source)
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
                ("digest", Json::from(digest_text(s.digest))),
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
        let options =
            Json::object([("force_search", Json::Bool(self.force_search)), ("use_prev", Json::Bool(self.use_prev))]);
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
                ("assumes", if self.completed { assumptions_json(&self.assumes) } else { Json::Null }),
                ("in_the_bag", self.in_the_bag.map_or(Json::Null, Json::from)),
                ("prev", self.prev.as_ref().map_or(Json::Null, PrevInfo::to_json)),
                ("same_search", self.same_search.as_ref().map_or(Json::Null, SameSearch::to_json)),
                ("nodes", Json::from(self.nodes)),
                ("stats", self.stats_json()),
                ("engine", engine),
                ("setup", setup),
                ("seconds", self.phases.to_json()),
                ("invocations", Json::from(self.invocations)),
                ("threads", Json::from(self.threads)),
                ("tactics", Json::from(self.tactics.description())),
                ("tactic_switches", self.tactics.to_json()),
                ("limits", limits),
                ("options", options),
                ("optimal_match", self.optimal_match.map_or(Json::Null, Json::Bool)),
                ("build", build_json()),
            ]);
        Json::object(fields)
    }

    /// One line for a person: board size, score, method, nodes, time, the comparison with `optimal.json`, the
    /// proof, and what it assumes, if anything.
    pub fn summary_line(&self) -> String {
        let optimal = match (self.completed, self.optimal_match) {
            (false, _) => "incomplete",
            (true, Some(_)) => "optimal ok",
            (true, None) => "optimal unknown",
        };
        let of = self.same_search.as_ref().map_or_else(String::new, |s| format!(" of {}", s.n));
        let proof = self.proof.map_or_else(String::new, |p| format!("  proof={}{of}", p.as_str()));
        let assumes = if self.assumes.is_empty() {
            String::new()
        } else {
            format!("  assumes {}", self.assumes.iter().cloned().collect::<Vec<_>>().join("; "))
        };
        format!(
            "n={:<5} score={:<9} {:<7} nodes={:<11} {:>9.3} s  {optimal}{proof}{assumes}",
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

/// Solves board size `n`.  `prev` is the solution of `n − 1` if known.  The score is confirmed against
/// `optimal.json` ([`Optimal::built_in`]).
pub fn solve_game(n: u32, prev: Option<&Prev>, opts: &SolveOptions) -> Result<GameResult, String> {
    solve_game_with(n, prev, opts, Optimal::built_in())
}

/// [`solve_game`] with the score confirmed against the table `optimal` instead: for tests, and for `taxman bench`,
/// whose known games extend the table.  The checkpoint of a completed game is deleted once the game has passed its
/// checks.
pub fn solve_game_with(
    n: u32,
    prev: Option<&Prev>,
    opts: &SolveOptions,
    optimal: &Optimal,
) -> Result<GameResult, String> {
    let result = solve_one(n, prev, opts, optimal, 0, None).map_err(Failure::message)?;
    if result.completed {
        discard_checkpoint(opts, n);
    }
    Ok(result)
}

type SolveWithWidth =
    fn(u32, Option<&Prev>, &SolveOptions, &Optimal, u32, Option<&mut Earlier>) -> Result<GameResult, Failure>;

/// Solves board size `n` and leaves its checkpoint, if any, to the caller.  `earlier_runs` is the number of
/// invocations a results file records for an unfinished search of `n`; `earlier` the games whose search `n` may
/// repeat (tactic `same_search`).
fn solve_one(
    n: u32,
    prev: Option<&Prev>,
    opts: &SolveOptions,
    optimal: &Optimal,
    earlier_runs: u32,
    earlier: Option<&mut Earlier>,
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
    solve(n, prev, opts, optimal, earlier_runs, earlier).map_err(|failure| match failure {
        Failure::Error(e) => Failure::Error(format!("n={n}: {e}")),
        Failure::Rejected(result, e) => Failure::Rejected(result, format!("n={n}: {e}")),
    })
}

fn solve_with_width<const W: usize>(
    n: u32,
    prev: Option<&Prev>,
    opts: &SolveOptions,
    optimal: &Optimal,
    earlier_runs: u32,
    earlier: Option<&mut Earlier>,
) -> Result<GameResult, Failure> {
    let start = Instant::now();
    let tactics = opts.tactics;
    if let Some(prev) = prev {
        // The prime shortcut and the cap trust the previous game, so make sure it is what it says it is.
        prev.check(n)?;
    }
    let tables = Tables::new(n);
    let mut result = GameResult::new(n, opts, prev);
    let context = Context { prev, optimal, start };
    // A search that was interrupted is resumed, whatever else could have produced the game.
    let checkpoint = opts.checkpoint_file(n).filter(|path| path.exists());
    let derive = !opts.force_search && checkpoint.is_none();

    if derive && tactics.prime_shortcut && tables.is_prime(n) {
        if let Some(prev) = prev {
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

    let promotions = search_game(&board, prev, opts, checkpoint, earlier_runs, earlier, &mut result)?;
    let moves = solution_for_promotions::<W>(&board.tables, promotions);
    conclude(result, moves, promotions.to_vec(), context)
}

/// What [`conclude`] checks a game against.
#[derive(Clone, Copy)]
struct Context<'a> {
    prev: Option<&'a Prev>,
    optimal: &'a Optimal,
    start: Instant,
}

/// Checks the game (see the module documentation), compares it with `optimal.json`, and completes the result
/// with its proof and its assumptions.
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
    if let Some(expected) = context.optimal.score(n) {
        if expected != score {
            return reject(result, format!("scored {score} but optimal.json says {expected}"));
        }
        result.optimal_match = Some(true);
    }
    let proof = Proof::of(result.method, true, result.reached_cap).expect("a completed game has a proof");
    result.proof = Some(proof);
    // What the proof rests on besides the game: the game a repeat repeats, else the game of n − 1.
    let basis = match &result.same_search {
        Some(same) => Some(&same.assumes),
        None => context.prev.map(|p| &p.assumes),
    };
    result.assumes = assumptions_of(proof, basis);
    Ok(result)
}

/// The search path: prepares the game (or the checkpoint's game), searches, and fills `result` with what the
/// search did.  Returns the best promotion set found.  The checkpoint is left in place (see [`solve_one`]).
///
/// Before it searches, the tactic `same_search` looks in `earlier` for a game whose search problem is this one's
/// and whose proof settles it: then the game is that game's promotions (method `repeat`), with no search.  Not for a
/// search that a checkpoint resumes (resumed, never replaced), or with `force_search`.
fn search_game<const W: usize>(
    board: &Board<W>,
    prev: Option<&Prev>,
    opts: &SolveOptions,
    checkpoint: Option<PathBuf>,
    earlier_runs: u32,
    earlier: Option<&mut Earlier>,
    result: &mut GameResult,
) -> Result<Bits<W>, Failure> {
    let n = board.n();
    let tactics = opts.tactics;
    let resuming = checkpoint.is_some();
    let prepared = match &checkpoint {
        Some(path) => {
            let p = board.prepare_for_resume(path).map_err(|e| {
                format!("cannot resume from {}: {e} (delete the file to start the game over)", path.display())
            })?;
            if p.tactics != tactics.search {
                return Err(Failure::Error(different_tactics(path, p.tactics, tactics)));
            }
            p
        }
        None => board.prepare(tactics.search)?,
    };
    result.phases.frames += prepared.timings.build;
    result.phases.greedy = prepared.timings.greedy;

    let problem = prepared.problem();
    let mut setup = SearchSetup {
        frames: board.frames.frames.len(),
        candidates: board.candidates().len(),
        max_total: prepared.max_total,
        greedy_sum: prepared.greedy.sum(),
        cap: None,
        digest: problem.digest(),
    };
    let repeats = tactics.same_search && !resuming && !opts.force_search;
    if let Some((same, promotions)) = earlier.filter(|_| repeats).and_then(|e| e.find(n, &problem)) {
        result.method = Method::Repeat;
        result.setup = Some(setup);
        result.same_search = Some(same);
        return Ok(promotions.into_iter().collect());
    }

    setup.cap = prev.filter(|_| tactics.prev_cap).and_then(|prev| board.promotion_cap(prev.score));
    let cap = setup.cap;
    // A resumed search has its incumbent in the checkpoint.
    let seed = prev.filter(|_| tactics.seed_incumbent && !resuming).map(|prev| board.seed(&prev.moves));
    result.setup = Some(setup);

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
        cores: None,
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
// Repeats of earlier searches

/// How far back a game looks for an earlier game whose search problem it repeats (tactic `same_search`): the board
/// sizes `n − SAME_SEARCH_WINDOW` to `n − 1`.  A board's frames change a little with each number: each of the 545
/// games of the strict chain 1..1300 whose problem an earlier game posed repeats the last board size before it that
/// set a search up, `n − 1`, or `n − 2` when `n − 1` is prime (docs/EXPERIMENTS.md, "Same search").  The window
/// keeps the setups a range starts with to a few, and makes the game repeated a function of the window's games alone.
pub const SAME_SEARCH_WINDOW: u32 = 8;

/// An earlier completed game, as much of it as a repeat of its search needs.
#[derive(Clone, Debug)]
struct Searched {
    /// The size of its problem as its setup recorded it, which the problem set up again must have.
    size: ProblemSize,
    /// The digest of its search problem, as its setup recorded it.
    digest: u64,
    promotions: Vec<u32>,
    proof: Proof,
    assumes: Assumptions,
}

/// The games a game of a range may repeat the search of (tactic `same_search`): the completed games of the results
/// file and of this run that set a search up and whose proof settles it ([`Proof::settles_search`]).
#[derive(Default)]
struct Earlier {
    games: BTreeMap<u32, Searched>,
}

impl Earlier {
    /// The games of the results file.
    fn of_file(file: &ResultsFile) -> Earlier {
        let mut earlier = Earlier::default();
        for game in file.games().filter(|game| game.proof.settles_search()) {
            if let Some((size, digest)) = game.size.zip(game.digest) {
                let (promotions, proof, assumes) = (game.promotions.clone(), game.proof, game.assumes.clone());
                let searched = Searched { size, digest, promotions, proof, assumes };
                earlier.games.insert(game.n, searched);
            }
        }
        earlier
    }

    /// Adds a game solved in this run.
    fn add_result(&mut self, result: &GameResult) {
        if let (true, Some(proof), Some(setup)) = (result.completed, result.proof, result.setup) {
            if proof.settles_search() {
                let (promotions, assumes) = (result.promotions.clone(), result.assumes.clone());
                let (size, digest) = (setup.size(), setup.digest);
                self.games.insert(result.n, Searched { size, digest, promotions, proof, assumes });
            }
        }
    }

    /// The latest game of the window before `n` ([`SAME_SEARCH_WINDOW`]) whose search problem is `problem`, as the
    /// record a repeat of it keeps, and its promotions.  A game whose digest is not `problem`'s is passed over; for
    /// one whose digest is, the problem is set up again from its board size ([`search_problem`]) and compared with
    /// `problem` itself, so that the answer never rests on two digests alone.  A game whose problem is not as large
    /// as its line says (a line made by a build whose frame builder posed another problem) is dropped: a verifier
    /// would refuse it too.
    fn find(&mut self, n: u32, problem: &Problem) -> Option<(SameSearch, Vec<u32>)> {
        let digest = problem.digest();
        for m in (n.saturating_sub(SAME_SEARCH_WINDOW)..n).rev() {
            let Some(game) = self.games.get(&m) else { continue };
            if game.digest != digest {
                continue;
            }
            let theirs = search_problem(m).ok().filter(|(theirs, _)| theirs.size() == game.size);
            let Some((theirs, _)) = theirs else {
                self.games.remove(&m);
                continue;
            };
            if theirs == *problem {
                let promotion_sum = game.promotions.iter().map(|&p| u64::from(p)).sum();
                let same = SameSearch { n: m, digest, promotion_sum, proof: game.proof, assumes: game.assumes.clone() };
                return Some((same, game.promotions.clone()));
            }
        }
        None
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
        assumes: game.assumes.clone(),
    })
}

/// The solution of game `n − 1` from the results file, else, with `use_prev`, from `optimal.json`.
fn previous_game(n: u32, results: Option<&ResultsFile>, optimal: &Optimal, use_prev: bool) -> Option<Prev> {
    let before = n.checked_sub(1).filter(|&before| before >= 1)?;
    let from_table = || use_prev.then(|| Prev::from_optimal(optimal, before)).flatten();
    results.and_then(|r| stored_prev(r, before)).or_else(from_table)
}

/// Says on stderr that game `n` has no previous game, so that it is solved standalone: correct, but without the
/// seed and the ceiling that the game of `n − 1` gives, so it costs more (`bench/curve/README.md` measures how
/// much).  `in_table`: whether `optimal.json` has the game of `n − 1` (which `--use-prev` would take).
fn note_standalone(n: u32, in_table: bool) {
    let before = n - 1;
    let how = if in_table {
        format!("give --out with the earlier games, or --use-prev to take optimal.json's game of {before}")
    } else {
        "give --out with the earlier games (optimal.json does not have it either)".to_owned()
    };
    let _ = writeln!(
        std::io::stderr(),
        "note: no previous game for n={n} (the game of {before} is neither in this run nor in the results file): \
         solving it standalone, with no seed and no ceiling, which costs more; to chain, {how}"
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
/// `lo − 1` comes from the results file, else, with [`SolveOptions::use_prev`], from `optimal.json`.  The results
/// file is checked when it is opened ([`ResultsFile::open`]); games already completed in it are skipped (and a
/// checkpoint left for one is deleted).  Every other result is appended to the file (if there is one), then given
/// to `on_game`, and only then is its checkpoint deleted.  A game that fails a check after it was found goes to the
/// file's [`rejected_path`](crate::results::rejected_path) instead, and the run fails.  The range stops at a game
/// whose search did not complete.  Scores are confirmed against `optimal.json` ([`Optimal::built_in`]).
pub fn solve_range_each(
    lo: u32,
    hi: u32,
    opts: &SolveOptions,
    results_path: Option<&Path>,
    on_game: impl FnMut(&GameResult),
) -> Result<RangeSummary, String> {
    solve_range_each_with(lo, hi, opts, results_path, Optimal::built_in(), on_game)
}

/// [`solve_range_each`] with the table `optimal` in place of `optimal.json`, for tests.
pub fn solve_range_each_with(
    lo: u32,
    hi: u32,
    opts: &SolveOptions,
    results_path: Option<&Path>,
    optimal: &Optimal,
    mut on_game: impl FnMut(&GameResult),
) -> Result<RangeSummary, String> {
    if lo < 1 || lo > hi {
        return Err(format!("the range {lo}..={hi} is empty or starts below 1"));
    }
    let mut results = results_path.map(|path| ResultsFile::open(path, optimal)).transpose()?;
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
    let mut earlier = results.as_ref().map(Earlier::of_file).unwrap_or_default();
    for n in lo..=hi {
        if let Some(stored) = results.as_ref().and_then(|r| stored_prev(r, n)) {
            prev = Some(stored);
            summary.skipped += 1;
            continue;
        }
        let before = if n == lo { previous_game(n, results.as_ref(), optimal, opts.use_prev) } else { prev.take() };
        if n == lo && n > 1 && before.is_none() {
            note_standalone(n, optimal.score(n - 1).is_some());
        }
        let earlier_runs = results.as_ref().map_or(0, |r| r.earlier_runs(n));
        let solved = solve_one(n, before.as_ref(), opts, optimal, earlier_runs, Some(&mut earlier));
        let result = match (solved, &results) {
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
        earlier.add_result(&result);
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

    /// The options of a test: one thread.
    fn one_thread() -> SolveOptions {
        SolveOptions { threads: 1, ..SolveOptions::default() }
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
        assert_eq!((json.get("prev"), json.get("assumes")), (Some(&Json::Null), Some(&Json::array(Vec::<u32>::new()))));
        let options = json.get("options").unwrap();
        assert_eq!(
            (options.get("force_search"), options.get("use_prev")),
            (Some(&Json::Bool(false)), Some(&Json::Bool(false)))
        );
        let contents = crate::results::read_results(&format!("{json}\n"), Optimal::built_in()).unwrap();
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
        assert_eq!(more[0].prev_source(), Some(PrevSource::Results));
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
        assert!(ResultsFile::open(&path, Optimal::built_in()).is_err());
    }

    #[test]
    fn the_previous_game_comes_from_optimal_json_only_when_asked() {
        // By default a range that starts late has no previous game: its first game is solved standalone.
        let r = solve_range(50, 52, &one_thread(), None).unwrap();
        assert_eq!((r[0].prev_source(), r[1].prev_source()), (None, Some(PrevSource::Run)));
        assert!(r.iter().all(|g| g.assumes.is_empty()), "standalone, and chained from it, nothing is assumed");
        // With `use_prev`, optimal.json's game of 49, assumed optimal as far as a proof rests on it.
        let opts = SolveOptions { use_prev: true, ..one_thread() };
        let r = solve_range(50, 52, &opts, None).unwrap();
        assert_eq!((r[0].prev_source(), r[1].prev_source()), (Some(PrevSource::Optimal), Some(PrevSource::Run)));
        let record = r[0].prev.as_ref().unwrap();
        assert_eq!((record.n, record.proof, &record.assumes), (49, None, &Assumptions::from([assumed_optimal(49)])));
        assert!(r[0].use_prev);
        let first = solve_range(1, 1, &opts, None).unwrap();
        assert_eq!((first[0].prev_source(), first[0].score, first[0].moves.len()), (None, 0, 0));
    }

    #[test]
    fn a_score_that_is_not_the_tables_is_a_hard_error() {
        let table = |text: &str| Optimal::from_json(&Json::parse(text).unwrap()).unwrap();
        let wrong = table(r#"[{"n":4,"score":8,"moves":[4,3]}]"#);
        let err = solve_game_with(4, None, &one_thread(), &wrong).unwrap_err();
        assert!(err.contains("scored 7") && err.contains("says 8"), "{err}");
        let r = solve_game_with(4, None, &one_thread(), &table("[]")).unwrap();
        assert_eq!((r.score, r.optimal_match), (7, None));
        let r = solve_game(4, None, &one_thread()).unwrap();
        assert_eq!((r.score, r.optimal_match), (7, Some(true)));
    }

    #[test]
    fn a_previous_game_that_does_not_check_out_is_refused() {
        let opts = one_thread();
        let prev = |moves: &[u32], score| Prev {
            moves: moves.to_vec(),
            score,
            source: PrevSource::Run,
            proof: Some(Proof::Exhausted),
            assumes: Assumptions::new(),
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
        let opts = one_thread();
        assert!(solve_game(0, None, &opts).is_err());
        assert!(solve_game(MAX_N + 1, None, &opts).unwrap_err().contains("beyond"));
        assert!(solve_range(5, 4, &opts, None).is_err());
        assert!(solve_range(0, 4, &opts, None).is_err());
    }
}
