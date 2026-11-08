//! The per-game pipeline and the range driver: what `taxman solve` runs.
//!
//! [`solve_game`] solves one game and [`solve_range`] a range of them, each built on the one before.  They produce
//! [`GameResult`]s: the moves, their score, how they were found ([`Method`]) and how their optimality is established (a
//! [`Proof`], and what it assumes, see [`crate::results`]).  Around them are the options ([`SolveOptions`]) and the
//! previous moves ([`Prev`]).  The tactics are in [`crate::tactics`], the analysis of a board that every search starts
//! from in [`crate::board`], the search in [`crate::search`] and the files in [`crate::results`].
//!
//! # One game
//!
//! The steps, in order (solver/docs/design/engine.md, "Per-game pipeline (`solver`)", has the same steps with the Java
//! classes they replace):
//!
//! 1. The tables of the board `1..=n`, and the set width `W` for it (`with_width!`; a checkpoint made at another
//!    width is refused).
//! 2. The *prime shortcut* (tactic `prime_shortcut`): a prime `n` plays the moves for `n − 1` with its first move
//!    replaced by `n` (method `prime`).
//! 3. The board's frames ([`Board`]).  With no promotion candidates the moves with no promotions are the only ones
//!    (method `trivial`).
//! 4. Otherwise the search (method `search`, or `start` when the search's starting moves reached a ceiling before
//!    it took a step): [`Board::prepare`] (or [`Board::prepare_for_resume`]), then
//!    [`search::search`] for the promotion set with the largest sum.  It starts from the better of the greedy set
//!    and the previous moves' promotions that are candidates here (`seed_incumbent`, [`Board::seed`]) and stops
//!    as soon as it reaches `n + opt(n − 1) − in_the_bag` (`prev_cap`, [`Board::promotion_cap`]).  The promotions
//!    become moves through the frames ([`solution_for_promotions`]).  Between the setup and the search, the
//!    tactic `same_search` may find that an earlier game already settled this search (method `repeat`, below).
//! 5. `conclude`: the moves are checked before they count, and given their proof kind and assumptions.
//! 6. Recording (a range only): the result is appended to the results file and synced, and only then is the
//!    game's checkpoint deleted.
//!
//! Step 2 and the repeat of step 4 are skipped by `force_search`, and when a checkpoint of the game exists: a search
//! that was interrupted is resumed, never replaced by a derivation.
//!
//! The steps run in four phases, which the driver of a range (`window`) composes, a single game included: the
//! *setup* (`set_up`: the checks of the previous moves, steps 1 to 3, and the setup of step 4), on a thread of the
//! game's own; the *repeat lookup* (the repeat of step 4, `Earlier::find`), the *search* (the rest of step 4, run
//! through a width-free `Job`), and the *conclusion* (`conclude_game`: step 5, and step 6 for a range).
//!
//! # Repeats of earlier searches
//!
//! Consecutive boards often pose exactly the same search problem ([`search::Problem`]: the frames, the candidates,
//! `max_total` and the factor graph among their numbers), only with another ceiling.  With the tactic `same_search`
//! (proven, on by default), a game whose problem an earlier game `m` of the range or of the results file posed, within
//! the [`SAME_SEARCH_WINDOW`] games before it, and whose proof settles that problem's best promotion sum
//! ([`Proof::settles_search`]), is `m`'s promotions played on its own board, with no search: method `repeat`, proof
//! `same_search`, resting on `m`'s proof and its assumptions.  The game is found by the problem's digest and confirmed
//! by comparing the two problems, taking `m`'s from this run's own setup of its board when there was one and setting it
//! up again otherwise; the repeat then goes through `conclude` like any other result.
//! solver/docs/design/same-search.md, has why it is optimal.
//!
//! The previous moves are checked before anything uses them: they must replay legally to their recorded score, be over
//! (no legal move left) and open with the largest prime, as the moves of every optimal game do.  Moves this run made
//! are not checked again: `conclude` made the same check before they were recorded.
//!
//! `conclude` checks that the moves replay legally ([`replay`]); that completed moves are also over and open with
//! the largest prime; that moves from the frame model score `in_the_bag` plus their promotions; that no moves
//! score more than `n + score(n − 1)` (nor their promotions exceed the ceiling), which would prove the previous moves
//! were not optimal, and that completed moves do not score less than the previous ones; and, for completed
//! moves, that their score is the table's when that has the moves (the table built into the binary,
//! [`Optimal::built_in`]).  Each failure is a hard error, and moves that fail after they were found are recorded,
//! with their promotions, in the results file's [`rejected_path`](crate::results::rejected_path).
//!
//! # A range
//!
//! [`solve_range_each`] solves the games `lo..=hi` and concludes them in order.  The previous moves of each are
//! those of the game before it; for `lo` they are the results file's line for `lo − 1`, else there are none and the
//! game is solved *standalone*, with no seed and no ceiling (and a note on stderr says so).  Every result is appended to
//! the results file (one JSON object per line) as soon as it is concluded, so that a restarted run skips what is
//! finished; the file is checked when it is opened ([`ResultsFile::open`]).  A game whose search a limit or a signal
//! stopped is recorded as incomplete, keeps its checkpoint, and ends the range.  The games a game of the range may
//! repeat the search of are the file's and the range's own (a single game solved with [`solve_game`] has none).
//!
//! Up to [`SolveOptions::window`] consecutive games are in flight at once: game `n + 1` is set up while `n`
//! searches, and searched from the best moves `n`'s search has found so far, so that a machine of many threads is kept
//! busy.  With a window of one, and always with one thread, the range is solved strictly in order, game after game.
//! The lines are the same either way but for what a multi-threaded search may vary (another optimal set of
//! promotions, the nodes, the times): `window` has the driver and the argument.
//!
//! # The unsound mode
//!
//! [`unsound`] plays games with the setup, the prime shortcut and the repeats of this pipeline and the start of its
//! search, and then LNS in place of the search: its lines are lower bounds, kept apart from the proven ones.

mod clock;
pub mod unsound;
mod window;

use crate::accelerate::{prime_shortcut, solution_for_promotions};
use crate::bits::{words_for, Bits};
use crate::board::{promotion_cap, search_problem, Board};
use crate::engine::{self, Control, Counters, EngineConfig, Spent, Stats};
use crate::frames::BoardFrames;
use crate::game::{replay, replay_finished, Optimal};
use crate::gf2::Gf2Graph;
use crate::json::Json;
use crate::numtheory::Tables;
use crate::results::{
    assumptions_json, assumptions_of, build_json, moves_digest, Assumptions, Proof, ResultsFile, SCHEMA,
};
use crate::search::{
    self, finder, gate_trial, stat, BranchSettings, Branching, NearRepeat, NodeBound, Prepared, Problem, ProblemSize,
    ProfilePlan, SearchSpace, SearchTactics,
};
use crate::tactics::{on_off, Tactics};
use crate::utc;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// The largest board the set widths cover: 8191, at the widest of [`crate::bits::WIDTHS`] (128 words).
pub const MAX_N: u32 = 64 * crate::bits::WIDTHS[crate::bits::WIDTHS.len() - 1] as u32 - 1;

// ---------------------------------------------------------------------------------------------------------
// Options and results

/// How to solve.
#[derive(Clone, Debug)]
pub struct SolveOptions {
    pub tactics: Tactics,
    pub threads: usize,
    /// The search's workers run under the kernel's idle scheduling policy (`--background`,
    /// [`engine::background_priority`]), so that the search takes the CPU that nothing else on the machine wants.  It
    /// changes only the speed, never a result.
    pub background: bool,
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
    /// Search even where the previous moves or an earlier search would give the answer (skips `prime_shortcut` and
    /// `same_search`).
    pub force_search: bool,
    /// The games of a range in flight at once (`--window`, at least 1, 4 by default: the best measured at 64 threads,
    /// see `window`), which share the `threads`.  With one thread it is 1 whatever it says: there is no thread to run
    /// a second game on.
    pub window: usize,
    /// The finder (`--finder-after`, `--finder-seconds`): once a game's search has taken `finder_after` steps, over
    /// every run of it, a thread of its own looks for better moves than the search's incumbent for `finder_budget`
    /// and offers what it finds ([`search::finder`]); 0, the default here: none (`solve` takes [`finder::AFTER`] when
    /// its flag does not say).  It is no tactic and no part of what a checkpoint records: a checkpoint resumes with any
    /// finder or none, and the finder changes no answer, only how soon the incumbent is good.
    pub finder_after: u64,
    /// The finder thread's budget, from the moment it starts (it ends sooner if the game's search does); by default
    /// [`finder::BUDGET`].
    pub finder_budget: Duration,
    /// The directory of the searches' profiles (`--profile`): each game's search keeps one, `nNNNN.profile.jsonl`, at
    /// the depth `profile_depth` (`--profile-depth`; [`search::SearchProfile`]).  A profile observes a search and
    /// changes nothing of it: it is no tactic, no part of a checkpoint and no part of a result.  None by default.
    pub profile: Option<PathBuf>,
    pub profile_depth: u32,
    /// The directory of stores of carried nogoods (`--carried-dir`, with the tactic `carried_nogoods`, which the flag
    /// turns on): each game's search carries the nogoods of the stores there of its game and the games before it,
    /// checked at its root ([`crate::model::CarriedNogoods::from_dir`]).  They only prune, so the results are the same
    /// with any stores or none.  None by default.
    pub carried_dir: Option<PathBuf>,
    /// The gate each game's search starts with (`--gate`), in place of [`search::CONFLICT_BRANCHING_GAP`], from 0 to
    /// [`search::MAX_GATE`]: the gap from which conflict branching chooses the conflict.  The gate trial starts from it
    /// ([`search::gate_trial`]).  It changes no answer and is no tactic and no part of a checkpoint, which resumes with
    /// any gate.  None by default: the search's own gate.
    pub gate: Option<u64>,
    /// The gate trial (`--gate-trial`): each game's search with conflict branching on tries these gates on a sample
    /// of its own units once it has taken the plan's steps, and branches with the one that took the least CPU
    /// ([`search::gate_trial`]).  It changes only the gate, which changes no answer, and is no tactic and no part of a
    /// checkpoint.  It needs a checkpoint, so a search without one runs no trial.  None by default (`--gate-trial none`
    /// says so): a search with a fixed gate, [`search::CONFLICT_BRANCHING_GAP`] unless `gate` is set.
    pub gate_trial: Option<search::gate_trial::Plan>,
}

impl Default for SolveOptions {
    fn default() -> Self {
        let engine = EngineConfig::default();
        SolveOptions {
            tactics: Tactics::default(),
            threads: engine.threads,
            background: false,
            checkpoint_dir: None,
            checkpoint_interval: Duration::from_secs(60),
            progress: None,
            time_limit: None,
            node_limit: None,
            queue_mem_cap: engine.queue_mem_cap,
            spill_dir: None,
            force_search: false,
            window: 4,
            finder_after: 0,
            finder_budget: finder::BUDGET,
            profile: None,
            profile_depth: search::PROFILE_DEPTH,
            carried_dir: None,
            gate: None,
            gate_trial: None,
        }
    }
}

impl SolveOptions {
    /// The checkpoint file of game `n`, if checkpointing is on.
    pub fn checkpoint_file(&self, n: u32) -> Option<PathBuf> {
        self.checkpoint_dir.as_ref().map(|dir| dir.join(format!("n{n:04}.ckpt")))
    }

    /// The file beside game `n`'s checkpoint that keeps its clock between invocations, if checkpointing is on.
    pub fn clock_file(&self, n: u32) -> Option<PathBuf> {
        self.checkpoint_dir.as_ref().map(|dir| dir.join(format!("n{n:04}.clock")))
    }
}

/// Where the previous moves came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrevSource {
    /// Solved earlier in this run: refereed by `conclude` already, so not checked again before it is used.
    Run,
    /// Read from the results file.
    Results,
}

impl PrevSource {
    pub const ALL: [PrevSource; 2] = [PrevSource::Run, PrevSource::Results];

    /// The name in result lines.
    pub fn as_str(self) -> &'static str {
        match self {
            PrevSource::Run => "run",
            PrevSource::Results => "results",
        }
    }

    /// Where it came from, for a message: "this run" or "the results file".
    pub fn whence(self) -> &'static str {
        match self {
            PrevSource::Run => "this run",
            PrevSource::Results => "the results file",
        }
    }
}

/// The moves for `n − 1`.
#[derive(Clone, Debug)]
pub struct Prev {
    pub moves: Vec<u32>,
    pub score: u64,
    pub source: PrevSource,
    /// How its optimality is established; `None` if that is not known (moves a caller gives).
    pub proof: Option<Proof>,
    /// What its proof assumes (see [`crate::results`]).
    pub assumes: Assumptions,
}

impl Prev {
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

    /// What a result records about the previous moves it was built from.
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

    /// Checks that the moves can be optimal for `n − 1`: legal, scoring what they are given as, over, and
    /// opening with the largest prime.  (Optimal itself cannot be checked; see the tripwires in [`conclude`].)
    fn check(&self, n: u32) -> Result<(), String> {
        let before = n - 1;
        let whence = self.source.whence();
        match replay(before, &self.moves) {
            Ok(score) if score == self.score => {}
            Ok(score) => {
                return Err(format!("the previous moves score {score}, not the {} they are given as", self.score))
            }
            Err(e) => return Err(format!("the previous moves are not legal for n={before}: {e}")),
        }
        replay_finished(before, &self.moves)
            .map(|_| ())
            .map_err(|e| format!("the previous moves (n={before}, from {whence}) cannot be optimal: {e}"))
    }
}

/// What a result records about the moves for `n − 1` it was built from.
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
    /// No promotion candidates: the moves with no promotions.
    Trivial,
    /// The prime shortcut.
    Prime,
    /// `same_search`: an earlier game's promotions, which settled the same search problem.
    Repeat,
    /// The search's starting moves reached a ceiling before the search took a step: the search ran no tree.
    Start,
    /// The branch-and-bound search: its tree ran.
    Search,
}

impl Method {
    pub const ALL: [Method; 5] = [Method::Trivial, Method::Prime, Method::Repeat, Method::Start, Method::Search];

    pub fn as_str(self) -> &'static str {
        match self {
            Method::Trivial => "trivial",
            Method::Prime => "prime",
            Method::Repeat => "repeat",
            Method::Start => "start",
            Method::Search => "search",
        }
    }

    /// The method [`as_str`](Self::as_str) names.
    pub fn parse(name: &str) -> Option<Method> {
        Method::ALL.into_iter().find(|method| method.as_str() == name)
    }

    /// Whether the game set a search up (and its result line has a `setup`): a search, whether its tree ran or its
    /// starting moves reached a ceiling, and a repeat, which set one up to compare it with an earlier game's.
    pub fn sets_up(self) -> bool {
        matches!(self, Method::Start | Method::Search | Method::Repeat)
    }
}

/// What a result records about the earlier game whose search problem it repeats (tactic `same_search`): that game's
/// n, the digest of the problem they share, its best promotion sum, and that game's proof and assumptions, which
/// the repeat rests on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SameSearch {
    /// The n of the earlier game.
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

/// The time of a game: its phases, and what it spent.
#[derive(Clone, Copy, Debug, Default)]
pub struct Phases {
    /// Forming, numbering and merging the frames, and the search model.
    pub frames: Duration,
    pub greedy: Duration,
    /// What the game spent in this invocation (`window`, "The clock"): the CPU time of its search's workers, the wall
    /// time it was active (with a worker: its setup and its search) and the wall time it waited (in flight with none:
    /// for a thread, for the game before it, suspended for older games).  Active and waiting together are the game's
    /// span, from the start of its setup to its line.  A game with no search to run (a prime, trivial or repeated game)
    /// used no CPU time in workers.
    pub spent: Spent,
    /// What the game spent in every invocation of a resumed search: this invocation's, and the earlier ones' as the
    /// clock they left beside the checkpoint says.
    pub spent_all_runs: Spent,
    /// The whole game (this invocation only).
    pub total: Duration,
}

impl Phases {
    pub fn to_json(&self) -> Json {
        let secs = |d: Duration| Json::Num(d.as_secs_f64());
        let (once, all) = (self.spent, self.spent_all_runs);
        Json::object([
            ("frames", secs(self.frames)),
            ("greedy", secs(self.greedy)),
            ("cpu", secs(once.cpu)),
            ("active", secs(once.active)),
            ("waiting", secs(once.waiting)),
            ("cpu_all_runs", secs(all.cpu)),
            ("active_all_runs", secs(all.active)),
            ("waiting_all_runs", secs(all.waiting)),
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

/// The outcome of solving one game n.
#[derive(Clone, Debug)]
pub struct GameResult {
    pub n: u32,
    /// The score of the moves; for an incomplete search, that of the best moves found so far (written as
    /// `best_so_far`, with `score` null).
    pub score: u64,
    pub moves: Vec<u32>,
    /// The numbers promoted, ascending.
    pub promotions: Vec<u32>,
    pub method: Method,
    /// False when a time or node limit stopped the search (its checkpoint is kept and `score` is only a lower
    /// bound).
    pub completed: bool,
    /// How the optimality of the moves is established (the proof kind); `None` when incomplete.
    pub proof: Option<Proof>,
    /// The search stopped because its incumbent reached the `prev_cap` ceiling.
    pub reached_cap: bool,
    /// What the proof assumes: what the moves it rests on assume, and so on down the chain (see
    /// [`crate::results`]); empty when incomplete.
    pub assumes: Assumptions,
    /// What the moves with no promotions score, for the results that come from the frame model (not prime ones).
    pub in_the_bag: Option<u64>,
    /// Search nodes, over all the runs of a resumed search.
    pub nodes: u64,
    /// The search's counters (see [`search::stat`]) and the engine's activity.
    pub stats: Stats,
    /// The nodes the near-repeat cut cut in the last invocation of the search, if the search was armed with it
    /// ([`Job::arm`]); `None` for a search that was not, and for a game that ran no search.  The line writes it as
    /// `stats.near_repeat_cut`, and only when it is there.
    pub near_repeat_cut: Option<u64>,
    pub counters: Counters,
    pub setup: Option<SearchSetup>,
    pub phases: Phases,
    /// When the game's setup started and when its line was made (this invocation's), written to the second as UTC
    /// timestamps ([`crate::utc`]).
    pub started: SystemTime,
    pub finished: SystemTime,
    /// The invocations that ran this game's search: 1, plus those of an interrupted search it resumed (as many
    /// as the results file records, at least 1; an invocation that was killed leaves no record, but its work up
    /// to its last checkpoint is in `nodes` and `phases.spent_all_runs`).
    pub invocations: u32,
    /// Of the last invocation.
    pub threads: usize,
    pub tactics: Tactics,
    /// The limits and options of the last invocation.
    pub time_limit: Option<Duration>,
    pub node_limit: Option<u64>,
    pub force_search: bool,
    pub background: bool,
    /// The finder's settings (`--finder-after`, in steps, and `--finder-seconds`): 0 for none.
    pub finder_after: u64,
    pub finder_budget: Duration,
    /// What the finder did in the last invocation, if it started.
    pub finder: Option<finder::Report>,
    /// The gate the searches started with (`--gate`).
    pub gate: Option<u64>,
    /// The gate trial's gates (`--gate-trial`), and what the trial did in the last invocation, if it ran to its end.
    pub gate_trial_gates: Option<Vec<u64>>,
    pub gate_trial: Option<gate_trial::Report>,
    /// `Some(true)` when the table has moves for `n`, whose score the result's equals (else it is rejected); `None`
    /// if the result is incomplete or the table lacks `n`.
    pub optimal_match: Option<bool>,
    /// The moves for `n − 1` this one was built from.
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
            near_repeat_cut: None,
            counters: Counters::default(),
            setup: None,
            phases: Phases::default(),
            started: UNIX_EPOCH,
            finished: UNIX_EPOCH,
            invocations: 1,
            threads: opts.threads,
            tactics: opts.tactics,
            time_limit: opts.time_limit,
            node_limit: opts.node_limit,
            force_search: opts.force_search,
            background: opts.background,
            finder_after: opts.finder_after,
            finder_budget: opts.finder_budget,
            finder: None,
            gate: opts.gate,
            gate_trial_gates: opts.gate_trial.as_ref().map(|plan| plan.gates.clone()),
            gate_trial: None,
            optimal_match: None,
            prev: prev.map(|p| p.info(n)),
            same_search: None,
        }
    }

    /// Where the moves for `n − 1` this one was built from came from, if there were any.
    pub fn prev_source(&self) -> Option<PrevSource> {
        self.prev.as_ref().map(|p| p.source)
    }

    /// The search's counters by name, and the nodes the near-repeat cut cut if the search was armed with it.
    pub fn stats_json(&self) -> Json {
        let counters = STAT_NAMES.map(|(name, i)| (name, Json::from(self.stats.extra[i])));
        let cut = self.near_repeat_cut.map(|nodes| ("near_repeat_cut", Json::from(nodes)));
        Json::object(counters.into_iter().chain(cut))
    }

    /// The result as one JSON object (schema [`SCHEMA`]; [`crate::results`] describes the provenance fields).
    pub fn to_json(&self) -> Json {
        let c = &self.counters;
        // What the finder did is there only on a line whose search started one: the lines made before it do not have it.
        let finder = self.finder.map(|f| {
            let report = Json::object([
                ("at_nodes", Json::from(f.at_nodes)),
                ("from", Json::from(f.from)),
                ("best", Json::from(f.best)),
                ("improved", Json::from(f.improved)),
                ("refused", Json::from(f.refused)),
                ("seconds", Json::Num(f.seconds)),
                ("found_seconds", Json::Num(f.found)),
            ]);
            ("finder", report)
        });
        // What the gate trial did is there only on a line whose search ran one to its end.
        let trial = self.gate_trial.as_ref().map(|t| {
            let scores = t.scores.iter().map(|s| {
                Json::object([
                    ("gate", Json::from(s.gate)),
                    ("steps", Json::from(s.steps)),
                    ("cpu_seconds", Json::Num(s.cpu)),
                    ("stopped", Json::from(s.stopped)),
                ])
            });
            let report = Json::object([
                ("at_nodes", Json::from(t.at_nodes)),
                ("units", Json::from(t.units)),
                ("from", Json::from(t.from)),
                ("scores", Json::array(scores)),
                ("chosen", Json::from(t.chosen)),
                ("seconds", Json::Num(t.seconds)),
            ]);
            ("gate_trial", report)
        });
        let engine = Json::object(
            [
                ("splits", Json::from(c.splits)),
                ("checkpoints", Json::from(c.checkpoints)),
                ("spilled_units", Json::from(c.spilled_units)),
                ("max_queue", Json::from(c.max_queue)),
            ]
            .into_iter()
            .chain(finder)
            .chain(trial),
        );
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
        // An incomplete result's score is only the best so far, so it does not go where a final score goes.
        let (score, best_so_far) = if self.completed {
            (Json::from(self.score), None)
        } else {
            (Json::Null, Some(("best_so_far", Json::from(self.score))))
        };
        let limits = Json::object([
            ("seconds", self.time_limit.map_or(Json::Null, |d| Json::Num(d.as_secs_f64()))),
            ("nodes", self.node_limit.map_or(Json::Null, Json::from)),
        ]);
        let background = self.background.then_some(("background", Json::Bool(true)));
        let finder_after = (self.finder_after > 0).then(|| ("finder_after", Json::from(self.finder_after)));
        let finder_seconds =
            (self.finder_after > 0).then_some(("finder_seconds", Json::Num(self.finder_budget.as_secs_f64())));
        let gate = self.gate.map(|gate| ("gate", Json::from(gate)));
        let trial_gates =
            self.gate_trial_gates.as_ref().map(|gates| ("gate_trial", Json::array(gates.iter().copied())));
        let options = Json::object(
            [("force_search", Json::Bool(self.force_search))]
                .into_iter()
                .chain(background)
                .chain(finder_after)
                .chain(finder_seconds)
                .chain(gate)
                .chain(trial_gates),
        );
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
            ])
            .chain([
                ("nodes", Json::from(self.nodes)),
                ("stats", self.stats_json()),
                ("engine", engine),
                ("setup", setup),
                ("started", Json::from(utc::rfc3339(self.started))),
                ("finished", Json::from(utc::rfc3339(self.finished))),
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

    /// One line for a person: n, score, method, nodes, time, the comparison with the table, the proof kind,
    /// and what it assumes, if anything; then the seconds the game was active and the CPU seconds of its workers, both
    /// over every invocation, and its best promotion sum (the score less `in_the_bag`, or `-` for a result that has no
    /// `in_the_bag`).
    pub fn summary_line(&self) -> String {
        let optimal = match (self.completed, self.optimal_match) {
            (false, _) => "incomplete",
            (true, Some(_)) => "optimal ok",
            (true, None) => "optimal unknown",
        };
        let of = self.same_search.as_ref().map_or_else(String::new, |same| format!(" of {}", same.n));
        let proof = self.proof.map_or_else(String::new, |p| format!("  proof={}{of}", p.as_str()));
        let assumes = if self.assumes.is_empty() {
            String::new()
        } else {
            format!("  assumes {}", self.assumes.iter().cloned().collect::<Vec<_>>().join("; "))
        };
        let best = self
            .in_the_bag
            .and_then(|bag| self.score.checked_sub(bag))
            .map_or_else(|| "-".to_owned(), |sum| sum.to_string());
        format!(
            "n={:<5} score={:<9} {:<7} nodes={:<11} {:>9.3} s  {optimal}{proof}{assumes}  active={:.3} s  cpu={:.3} s  \
             best={best}",
            self.n,
            self.score,
            self.method.as_str(),
            self.nodes,
            self.phases.total.as_secs_f64(),
            self.phases.spent_all_runs.active.as_secs_f64(),
            self.phases.spent_all_runs.cpu.as_secs_f64()
        )
    }
}

// ---------------------------------------------------------------------------------------------------------
// One game, in phases

/// Why a game could not be solved.
enum Failure {
    /// Before there were moves to show for it.
    Error(String),
    /// The moves were found but failed a check: the result, with the moves and promotions found, is the evidence.
    Rejected(Box<GameResult>, String),
}

impl Failure {
    /// The message; a rejected result's promotions go with it, so that they are not lost.
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

/// A check that failed after the moves were found.
fn reject(result: GameResult, reason: String) -> Result<GameResult, Failure> {
    Err(Failure::Rejected(Box::new(result), reason))
}

/// The failure of game `n`, saying which game it is.
fn at(n: u32, failure: Failure) -> Failure {
    let say = |message: String| format!("n={n}: {message}");
    match failure {
        Failure::Error(message) => Failure::Error(say(message)),
        Failure::Rejected(result, message) => Failure::Rejected(result, say(message)),
    }
}

/// Solves game `n`.  `prev` holds the moves for `n − 1` if known.  The score is confirmed against
/// the table ([`Optimal::built_in`]).
pub fn solve_game(n: u32, prev: Option<&Prev>, opts: &SolveOptions) -> Result<GameResult, String> {
    solve_game_with(n, prev, opts, Optimal::built_in())
}

/// [`solve_game`] with the score confirmed against the table `optimal` instead: for tests.  The checkpoint of a
/// completed game is deleted once the game has passed its checks.  This is the driver of a range (`window`) over the
/// one game `n`, from `prev`, with no results file and no earlier games to repeat.
pub fn solve_game_with(
    n: u32,
    prev: Option<&Prev>,
    opts: &SolveOptions,
    optimal: &Optimal,
) -> Result<GameResult, String> {
    if n == 0 {
        return Err("n must be at least 1".into());
    }
    let before = prev.cloned().map_or(Before::None, Before::Given);
    let range = window::Range { lo: n, hi: n, before, optimal, results: None };
    let mut solved = None;
    window::solve(range, opts, &mut Earlier::default(), &mut |result| solved = Some(result.clone()))?;
    Ok(solved.expect("a range of one game that does not fail concludes it"))
}

/// The moves for `n − 1` as the setup of game `n` knows them.
#[derive(Clone, Debug)]
enum Before {
    /// Not made by this run (the results file's or a caller's): known from the start, and checked by the setup.
    Given(Prev),
    /// This run's: known once it is concluded, and refereed by `conclude` before that.
    Run,
    /// None: the game is solved standalone, with no seed and no ceiling.
    None,
}

/// What the setup of a game (its first phase, [`set_up`]) made of it.
enum SetUp {
    /// A result concluded from its setup alone.
    Derived(Derived),
    /// A search problem, and the problem it poses, for the repeat lookup.
    Search(SearchSetUp, Problem),
}

/// A game that needs no search.
enum Derived {
    /// The prime shortcut: the promotion candidates of the board with no promotions, ascending.  The moves, those for
    /// `n − 1` with the first move replaced by `n`, are made when it is concluded.
    Prime { candidates: Vec<u32> },
    /// No promotion candidates: the moves with no promotions.
    Trivial { plain: Vec<u32>, in_the_bag: u64, frames: Duration },
}

/// A game set up for a search.
struct SearchSetUp {
    job: Arc<dyn Job>,
    /// What the search was set up with; its ceiling (`cap`) is filled in when the game is concluded.
    setup: SearchSetup,
    in_the_bag: u64,
    /// The setup's phases: forming the frames and the search model, and the greedy set.
    frames: Duration,
    greedy: Duration,
}

impl SearchSetUp {
    /// Fills in what a result records of the setup, for a game searched or repeated.
    fn fill(&self, result: &mut GameResult) {
        result.in_the_bag = Some(self.in_the_bag);
        result.phases.frames = self.frames;
        result.phases.greedy = self.greedy;
    }
}

/// What the driver of a range tells a game's thread when its search may start.
struct Go {
    /// The moves for `n − 1`, final or provisional, whose promotions seed the search: given when the search takes a
    /// seed ([`Job::seeds`]) and there are previous moves.
    seed: Option<Vec<u32>>,
    /// The moves for `n − 1`, final or provisional, that the finder starts from besides the search's incumbent, if
    /// the search runs one ([`SolveOptions::finder_after`]): given also when the search takes no seed (a resumed search
    /// has its incumbent in its checkpoint, and no seed).
    finder_seed: Option<Vec<u32>>,
    /// The ceiling on the promotion sum, given when game `n − 1` is concluded and `prev_cap` is on.
    cap: Option<u64>,
    /// The workers the search starts with.
    threads: usize,
    /// The moves for `n − 1`, if they are concluded when the search starts: those of the near-repeat cut
    /// ([`Job::arm`]).  A search that starts before then is armed when they are concluded, if it still runs.
    concluded: Option<Prev>,
}

/// What came of a game's search, whatever its set width.
struct Outcome {
    /// The best promotion set found (an optimal set when completed), ascending, and the moves it makes.
    promotions: Vec<u32>,
    moves: Vec<u32>,
    /// The search finished, or reached a ceiling it was given; false when a limit or a stop stopped it.
    completed: bool,
    stats: Stats,
    /// Of this invocation.
    counters: Counters,
    /// What the finder did, if it started.
    finder: Option<finder::Report>,
    /// What the gate trial did, if it ran to its end.
    gate_trial: Option<gate_trial::Report>,
    /// The nodes the near-repeat cut cut, if the search was armed with it ([`Job::arm`]).
    near_repeat_cut: Option<u64>,
}

/// A game set up for a search, whatever its set width: what the game's thread and the driver of the range need of it.
/// The thread runs the search ([`search`](Job::search)); the driver, meanwhile, reads the best promotions so far for
/// the games that follow, weighs what a seed would give the search, and gives the search its ceiling and seed through
/// its control.
trait Job: Send + Sync {
    /// The moves that `promotions` (a promotable set of the board's candidates) make on the board.
    fn moves_for(&self, promotions: &[u32]) -> Vec<u32>;
    /// The search's best promotions so far, if it has any ([`search::best_so_far`]): its engine's incumbent while it
    /// runs and after it ended, else the best offered to it.
    fn best_so_far(&self) -> Option<Vec<u32>>;
    /// Whether the game has a checkpoint, whose search is resumed, never replaced by a repeat.
    fn resuming(&self) -> bool;
    /// Whether the search starts from a seed: with `seed_incumbent`, unless it is resumed (a resumed search has its
    /// incumbent in its checkpoint).
    fn seeds(&self) -> bool;
    /// Offers the search, if it takes a seed, the promotions of the `moves` for `n − 1` that can still be promoted
    /// here, as an incumbent ([`search::offer_seed`]).
    fn hint(&self, moves: &[u32]);
    /// The promotion sum of what survives here of the `moves` for `n − 1` as a seed
    /// ([`search::seed_survivors_sum`]): a search seeded with those moves starts at least that high.
    fn survivors(&self, moves: &[u32]) -> u64;
    /// What survives here of `moves`, another game's (a move above n is dropped, [`Board::seed`]), as promotions,
    /// ascending: of the candidates among them, those that can still be promoted, from the largest down
    /// ([`search::seed_survivors`]).  The unsound mode's repair makes a line of them ([`unsound`]).
    fn surviving(&self, moves: &[u32]) -> Vec<u32>;
    /// The control through which the driver steers the search ([`engine::Control`]).
    fn control(&self) -> &Control;
    /// Arms the search's near-repeat cut by `prev`, the moves for `n − 1` once concluded ([`near_repeat`]): with
    /// the tactic `near_repeat`, where `prev` assumes nothing.  The cut is set up here, on the caller's
    /// thread, and the searchers pick it up as each starts a unit, so the search may run or not have started.  It is
    /// armed once: a later call does nothing.  Returns the cut's bound B if this call armed it.
    ///
    /// # Errors
    ///
    /// Those of [`near_repeat`], and then the cut is not armed.
    fn arm(&self, prev: &Prev) -> Result<Option<u64>, String>;
    /// The search, from the go (the game's third phase), and the moves its promotions make.  The game's checkpoint is
    /// left in place, for the caller to delete once the result is safe ([`discard_checkpoint`]).
    fn search(&self, go: Go, opts: &SolveOptions) -> Result<Outcome, String>;
    /// The unsound mode's search ([`unsound`]) on `threads` threads, with no ceiling in its control: the start that a
    /// search makes ([`search::start_incumbent`]), from the promotions of `seed`, the moves for `n − 1`, if the search
    /// takes a seed, then the rounds of LNS on many threads that `plan` asks for ([`finder::Lns::rounds`]), which end
    /// once their work reaches the plan's budget ([`unsound::Plan::budget`]) or `enough` says so of the game's
    /// promotion sum, the work of the rounds so far and the quiet rounds, those since the game's set last became
    /// heavier ([`finder::Rounds::quiet`]); the run's ceiling, the table's score and the stop on growth are the
    /// caller's.  If the search takes a seed, the rounds follow a trajectory besides the start's from what
    /// survives of the moves in `others`, games and their moves (n − 1, n − 2, ..., then n + 1, n + 2, ...: a move
    /// above n is dropped, [`Board::seed`]), for each that is not a set an earlier trajectory began from
    /// ([`unsound::trajectories`]).  `clock` is the game's seconds so far.  The start's dive is the one
    /// [`ready`](Job::ready) made, if it was called.
    fn improve(
        &self,
        seed: Option<&[u32]>,
        others: &[(u32, &[u32])],
        plan: &unsound::Plan,
        threads: usize,
        clock: &dyn Fn() -> f64,
        enough: &dyn Fn(u64, u64, u64) -> bool,
    ) -> unsound::Improved;
    /// Makes the dive of the unsound mode's start ([`search::start_dive`]) once, which depends on the board alone, so
    /// that a run may make it while the game before plays.
    fn ready(&self);
    /// The bound of the root's node, made once ([`NodeBound::root`]): the GF(2) bound of the board's candidates with at
    /// most `max_total` promotions, which bounds the promotion sum of every play of the game.  The unsound mode's bound
    /// of a game is `in_the_bag` plus this ([`unsound`], "The bound and the gap").
    fn bound(&self) -> u64;
}

/// The [`Job`] of a board of set width `W`.
struct SearchJob<const W: usize> {
    board: Board<W>,
    prepared: Prepared<W>,
    control: Arc<Control>,
    resuming: bool,
    seeds: bool,
    /// The branching settings the search starts with, which its gate trial may change ([`search::search_steered`]).
    branching: Arc<Branching>,
    /// The near-repeat cut, once it is armed ([`Job::arm`]): the search reads it from here.
    near: OnceLock<NearRepeat<W>>,
    /// The dive of the unsound mode's start, once it is made ([`Job::ready`]).
    dived: OnceLock<Bits<W>>,
    /// The bound of the root's node, once it is made ([`Job::bound`]).
    root_bound: OnceLock<u64>,
}

impl<const W: usize> SearchJob<W> {
    /// The seed of a search: the moves `prev` for `n − 1` as the set of their promotions, if there are any.
    fn seed_of(&self, prev: Option<&[u32]>) -> Option<Bits<W>> {
        prev.map(|moves| self.board.seed(moves))
    }
}

impl<const W: usize> Job for SearchJob<W> {
    fn moves_for(&self, promotions: &[u32]) -> Vec<u32> {
        solution_for_promotions::<W>(&self.board.tables, promotions.iter().copied().collect())
    }

    fn best_so_far(&self) -> Option<Vec<u32>> {
        search::best_so_far::<W>(&self.control).map(|best| best.to_vec())
    }

    fn resuming(&self) -> bool {
        self.resuming
    }

    fn seeds(&self) -> bool {
        self.seeds
    }

    fn hint(&self, moves: &[u32]) {
        if self.seeds {
            search::offer_seed(&self.prepared, &self.control, &self.board.seed(moves));
        }
    }

    fn survivors(&self, moves: &[u32]) -> u64 {
        search::seed_survivors_sum(&self.prepared, &self.board.seed(moves))
    }

    fn surviving(&self, moves: &[u32]) -> Vec<u32> {
        search::seed_survivors(&self.prepared, &self.board.seed(moves)).to_vec()
    }

    fn control(&self) -> &Control {
        &self.control
    }

    fn arm(&self, prev: &Prev) -> Result<Option<u64>, String> {
        if !self.prepared.tactics.near_repeat || self.near.get().is_some() {
            return Ok(None);
        }
        let Some(near) = near_repeat(&self.prepared, prev)? else { return Ok(None) };
        let bound = near.bound();
        Ok(self.near.set(near).is_ok().then_some(bound))
    }

    fn search(&self, go: Go, opts: &SolveOptions) -> Result<Outcome, String> {
        let n = self.board.n();
        let seed = self.seed_of(go.seed.as_deref());
        // The finder starts from the same seed, or from the moves for `n − 1` when the search takes none.
        let finder = (opts.finder_after > 0).then(|| {
            let seed = self.seed_of(go.finder_seed.as_deref());
            finder::Plan { after: opts.finder_after, budget: opts.finder_budget, seed }
        });
        let (spill_dir, own) = spill_dir(opts, n);
        let cfg = EngineConfig {
            threads: go.threads,
            queue_mem_cap: opts.queue_mem_cap,
            spill_dir: Some(spill_dir.clone()),
            checkpoint: opts.checkpoint_file(n).map(|path| (path, opts.checkpoint_interval)),
            // Deleted only once the result is safe (`discard_checkpoint`).
            keep_completed_checkpoint: true,
            progress: opts.progress,
            time_limit: opts.time_limit,
            node_limit: opts.node_limit,
            label: format!("n={n}"),
            progress_note: None,
            cores: None,
            control: Some(Arc::clone(&self.control)),
            background: opts.background,
            // The search's own, if it keeps a profile.
            checkpoint_hook: None,
        };
        if let Some(prev) = &go.concluded {
            self.arm(prev)?;
        }
        let (prepared, resuming, branching, plan) = (&self.prepared, self.resuming, &self.branching, finder.as_ref());
        let profile = opts.profile.as_deref().map(|dir| ProfilePlan::in_dir(dir, n, opts.profile_depth));
        let near = &self.near;
        let (profile, carried) = (profile.as_ref(), opts.carried_dir.as_deref());
        let found = search::search_steered(
            prepared,
            &cfg,
            seed.as_ref(),
            go.cap,
            resuming,
            plan,
            branching,
            near,
            profile,
            carried,
            opts.gate_trial.as_ref(),
        );
        if own {
            // Only removes the directory if the engine left nothing in it.
            let _ = fs::remove_dir(&spill_dir);
        }
        let found = found.map_err(|e| format!("search failed: {e}"))?;
        Ok(Outcome {
            promotions: found.promotions.to_vec(),
            moves: solution_for_promotions::<W>(&self.board.tables, found.promotions),
            completed: found.completed,
            stats: found.stats,
            counters: found.counters,
            finder: found.finder,
            gate_trial: found.gate_trial,
            near_repeat_cut: self.near.get().map(NearRepeat::nodes_cut),
        })
    }

    fn improve(
        &self,
        seed: Option<&[u32]>,
        others: &[(u32, &[u32])],
        plan: &unsound::Plan,
        threads: usize,
        clock: &dyn Fn() -> f64,
        enough: &dyn Fn(u64, u64, u64) -> bool,
    ) -> unsound::Improved {
        let prepared = &self.prepared;
        let seed = self.seed_of(seed.filter(|_| self.seeds));
        // The job's control has no ceiling, and only takes the start's offers.
        let dived = || *self.dived.get_or_init(|| search::start_dive(prepared));
        let start = search::start_incumbent_with(prepared, &self.control, threads, seed.as_ref(), dived);
        let start_seconds = clock();
        // The other trajectories, if there are any, begin from what survives of the moves for the games before, as the
        // finder's seed rounds begin from what survives of the seed.  The start is at least as heavy as the survivors
        // of the moves for n − 1, since it began from them, but those of an earlier game may be heavier than the start:
        // the game's set then begins as theirs (`finder::Lns::rounds` records it as an improvement of round 0).
        let survivors: Vec<(u32, Bits<W>)> = others
            .iter()
            .filter(|_| self.seeds)
            .map(|&(game, moves)| (game, search::seed_survivors(prepared, &self.board.seed(moves))))
            .collect();
        let sums = survivors.iter().map(|(game, set)| (*game, set.sum())).collect();
        let (starts, trajectories) = unsound::trajectories(start, survivors);
        let space = SearchSpace::new(prepared);
        let lns = finder::Lns { p: prepared, space: &space, options: plan.lns() };
        let ran = lns.rounds(&starts, plan.budget(), threads, clock, enough);
        unsound::Improved {
            start: start.sum(),
            start_seconds,
            trajectories,
            survivors: sums,
            promotions: ran.set.to_vec(),
            moves: solution_for_promotions::<W>(&self.board.tables, ran.set),
            rounds: ran.rounds,
            quiet: ran.quiet(),
            nodes: ran.nodes,
            work: ran.work,
            stage_rounds: ran.stage_rounds,
            improvements: ran.improvements,
            interrupted: ran.interrupted,
        }
    }

    fn ready(&self) {
        self.dived.get_or_init(|| search::start_dive(&self.prepared));
    }

    fn bound(&self) -> u64 {
        *self.root_bound.get_or_init(|| {
            let p = &self.prepared;
            NodeBound::new(p, &Gf2Graph::new(&p.model, &p.root, &p.candidates)).root()
        })
    }
}

/// The near-repeat cut of the search of `prepared`, game `n`, by `prev`, the moves for `n − 1` concluded
/// ([`crate::board::near_repeat`]; the near-repeat lemma (R7), docs/theory/search-rules.md).  `None` where its proof
/// assumes anything.
///
/// B, `prev`'s score less the `in_the_bag` of `n − 1`, bounds P\*(Π_M) where that score is opt(n − 1)
/// (docs/theory/search-rules.md, the paragraph "What it rests on"): moves that assume nothing are optimal within the
/// frame model, so the cut adds nothing to what the search's answer rests on, and a result that the cut helped assumes
/// what it assumed without it.  Moves that assume something would make the answer rest on that, which a result line
/// has no way to say, so they carry no cut.  Nor do provisional moves for `n − 1`, the best of a search still
/// running, which bound nothing: `window`'s driver gives a search the moves for `n − 1` only once they are concluded,
/// with the search's go or, for a search started before, at the conclusion ([`Job::arm`]).
fn near_repeat<const W: usize>(prepared: &Prepared<W>, prev: &Prev) -> Result<Option<NearRepeat<W>>, String> {
    if !prev.assumes.is_empty() {
        return Ok(None);
    }
    crate::board::near_repeat(prepared, prev.score)
}

type SetUpWithWidth = fn(u32, &Before, &SolveOptions) -> Result<SetUp, Failure>;

/// The setup of game `n` (its first phase, on the game's thread): the checks of previous moves not made by this run,
/// the tables, and then the prime shortcut, the trivial moves, or the search problem (steps 1 to 4 of the module
/// documentation, up to the repeat), at the set width of the game.
fn set_up(n: u32, before: &Before, opts: &SolveOptions) -> Result<SetUp, Failure> {
    let words = words_for(n);
    let beyond = || Failure::Error(format!("n={n} is beyond the largest supported board, n={MAX_N}"));
    let set_up: SetUpWithWidth = crate::with_words!(words, W => set_up_with_width::<W>, else return Err(beyond()));
    set_up(n, before, opts).map_err(|failure| at(n, failure))
}

fn set_up_with_width<const W: usize>(n: u32, before: &Before, opts: &SolveOptions) -> Result<SetUp, Failure> {
    if let Before::Given(prev) = before {
        // The prime shortcut and the ceiling trust the previous moves, so make sure they are what they say they are.
        // Not for moves this run made: `conclude` has refereed those already, with the same `replay_finished` and to
        // the score they are given as, before they became the previous moves.
        if prev.source != PrevSource::Run {
            prev.check(n)?;
        }
    }
    let tables = Tables::new(n);
    // A search that was interrupted is resumed, whatever else could have produced the moves.
    let checkpoint = opts.checkpoint_file(n).filter(|path| path.exists());
    let derive = !opts.force_search && checkpoint.is_none();

    if derive && opts.tactics.prime_shortcut && tables.is_prime(n) && !matches!(before, Before::None) {
        let candidates = BoardFrames::<W>::new(&tables, Bits::EMPTY).promotion_candidates();
        return Ok(SetUp::Derived(Derived::Prime { candidates: candidates.to_vec() }));
    }

    let board = Board::<W>::new(tables);
    if board.is_trivial() {
        let (in_the_bag, frames) = (board.in_the_bag, board.frames_time);
        return Ok(SetUp::Derived(Derived::Trivial { plain: board.plain, in_the_bag, frames }));
    }

    let prepared = match &checkpoint {
        Some(path) => {
            let mut p = board.prepare_for_resume(path).map_err(|e| {
                format!("cannot resume from {}: {e} (delete the file to start the game over)", path.display())
            })?;
            // A checkpoint does not bind how the searchers keep their nogoods, nor how they branch: this run says.
            p.tactics = p.tactics.with_unbound_of(&opts.tactics.search);
            if p.tactics != opts.tactics.search {
                return Err(Failure::Error(different_tactics(path, p.tactics, opts.tactics)));
            }
            p
        }
        None => board.prepare(opts.tactics.search)?,
    };
    let problem = prepared.problem();
    let setup = SearchSetup {
        frames: board.frames.frames.len(),
        candidates: board.candidates().len(),
        max_total: prepared.max_total,
        greedy_sum: prepared.greedy.sum(),
        cap: None,
        digest: problem.digest(),
    };
    let (in_the_bag, frames, greedy) =
        (board.in_the_bag, board.frames_time + prepared.timings.build, prepared.timings.greedy);
    let resuming = checkpoint.is_some();
    let seeds = opts.tactics.seed_incumbent && !resuming;
    // The branching settings the searchers start with: the tactics' (checked above to be the game's), and the gate, which
    // is no tactic, the run's (`--gate`) if it has one, else the default.
    let branching = Arc::new(Branching::new(BranchSettings::of(&prepared.tactics).with_gate(opts.gate)));
    let (control, near, dived, root_bound) = (Control::new(), OnceLock::new(), OnceLock::new(), OnceLock::new());
    let job = Arc::new(SearchJob { board, prepared, control, resuming, seeds, branching, near, dived, root_bound });
    Ok(SetUp::Search(SearchSetUp { job, setup, in_the_bag, frames, greedy }, problem))
}

/// The ceiling on the promotion sum of game `n`, whose `in_the_bag` is given, from the score of the moves for
/// `n − 1`, if there are any and `prev_cap` is on ([`promotion_cap`]).
fn ceiling(n: u32, in_the_bag: u64, prev_score: Option<u64>, opts: &SolveOptions) -> Option<u64> {
    prev_score.filter(|_| opts.tactics.prev_cap).and_then(|score| promotion_cap(n, in_the_bag, score))
}

/// How the moves were found, as the conclusion takes it.
enum Found<'a> {
    Derived(&'a Derived),
    /// A repeat of the search of the game `m`, which settled the same problem.
    Repeat(&'a SearchSetUp, u32),
    Searched(&'a SearchSetUp, &'a Outcome),
}

/// The conclusion of game `n` (its last phase): its result, as the moves were found, checked by [`conclude`], whose
/// `context` has the previous moves.  `earlier` has the game a repeat repeats, and `earlier_runs` is the number of
/// invocations a results file records for an unfinished search of `n`.
fn conclude_game(
    n: u32,
    found: Found<'_>,
    earlier: &Earlier,
    earlier_runs: u32,
    opts: &SolveOptions,
    context: Context<'_>,
) -> Result<GameResult, Failure> {
    let mut result = GameResult::new(n, opts, context.prev);
    let concluded = match found {
        Found::Derived(Derived::Prime { candidates }) => {
            let prev = context.prev.expect("the prime shortcut is taken only with previous moves");
            let (moves, promotions) = prime_game(n, candidates, &prev.moves);
            result.method = Method::Prime;
            conclude(result, moves, promotions, context)
        }
        Found::Derived(Derived::Trivial { plain, in_the_bag, frames }) => {
            result.method = Method::Trivial;
            result.in_the_bag = Some(*in_the_bag);
            result.phases.frames = *frames;
            conclude(result, plain.clone(), Vec::new(), context)
        }
        Found::Repeat(set, m) => {
            set.fill(&mut result);
            let Some((same, promotions)) = earlier.same_search(m, set.setup.digest) else {
                return Err(at(n, Failure::Error(format!("the game {m} whose search it repeats is not concluded"))));
            };
            result.method = Method::Repeat;
            result.setup = Some(set.setup);
            result.same_search = Some(same);
            let moves = set.job.moves_for(&promotions);
            conclude(result, moves, promotions, context)
        }
        Found::Searched(set, searched) => {
            set.fill(&mut result);
            let cap = ceiling(n, set.in_the_bag, context.prev.map(|prev| prev.score), opts);
            let setup = SearchSetup { cap, ..set.setup };
            result.setup = Some(setup);
            let sum: u64 = searched.promotions.iter().map(|&p| u64::from(p)).sum();
            // A search ends when its incumbent reaches its ceiling, which proves it optimal.  One that was started
            // before the ceiling was known (game `n − 1` still in flight) may have gone on to the end of its
            // tree, or been stopped by a limit or a signal, with its best set at the ceiling: it is proven by the
            // ceiling all the same, as a search given the ceiling from the start would have stopped there.
            result.reached_cap = setup.cap == Some(sum);
            result.completed = searched.completed || result.reached_cap;
            result.nodes = searched.stats.nodes;
            // A search that took no step (in any of its runs: the nodes are the total, the engine restores the stats
            // of a checkpoint) ran no tree: its starting moves reached a ceiling.
            if result.completed && result.nodes == 0 {
                result.method = Method::Start;
            }
            result.stats = searched.stats;
            result.near_repeat_cut = searched.near_repeat_cut;
            result.counters = searched.counters;
            result.finder = searched.finder;
            result.gate_trial = searched.gate_trial.clone();
            if set.job.resuming() {
                // At least one invocation made the checkpoint, even if it left no line in a results file.
                result.invocations = earlier_runs.max(1) + 1;
            }
            conclude(result, searched.moves.clone(), searched.promotions.clone(), context)
        }
    };
    concluded.map_err(|failure| at(n, failure))
}

/// The moves and the promotions of the prime shortcut on game `n`, a prime, whose board with no promotions has the
/// promotion candidates `candidates` (ascending), from `prev_moves`, the moves for `n − 1`: those moves with the first
/// replaced by `n`, and the candidates among them.
fn prime_game(n: u32, candidates: &[u32], prev_moves: &[u32]) -> (Vec<u32>, Vec<u32>) {
    let moves = prime_shortcut(n, prev_moves);
    let played: BTreeSet<u32> = moves.iter().copied().collect();
    let promotions = candidates.iter().copied().filter(|c| played.contains(c)).collect();
    (moves, promotions)
}

/// What [`conclude`] checks the moves of a game against.
#[derive(Clone, Copy)]
struct Context<'a> {
    prev: Option<&'a Prev>,
    optimal: &'a Optimal,
    /// The time of the game's own thread before its conclusion: its setup, its repeat lookup and its search (a
    /// suspended search's wait included), not the time it waited for the games before it.
    own: Duration,
    /// What the game spent in this invocation, and in the invocations before it (its clock): `Phases::spent` and the
    /// rest of `Phases::spent_all_runs`.
    spent: Spent,
    earlier: Spent,
    /// When its setup started (`GameResult::started`).
    started: SystemTime,
    /// When its conclusion began.
    since: Instant,
}

/// Checks the moves (see the module documentation), compares them with the table, and completes the result
/// with its proof kind and its assumptions.
fn conclude(
    mut result: GameResult,
    moves: Vec<u32>,
    promotions: Vec<u32>,
    context: Context,
) -> Result<GameResult, Failure> {
    let n = result.n;
    result.moves = moves;
    result.promotions = promotions;
    result.phases.total = context.own + context.since.elapsed();
    result.phases.spent = context.spent;
    result.phases.spent_all_runs = context.earlier + context.spent;
    result.started = context.started;
    result.finished = SystemTime::now();
    let method = result.method.as_str();
    // Completed moves claim to be optimal, so they must be over and open with the largest prime.
    let refereed = if result.completed { replay_finished(n, &result.moves) } else { replay(n, &result.moves) };
    result.score = match refereed {
        Ok(score) => score,
        Err(e) => return reject(result, format!("the {method} moves do not stand up: {e}")),
    };
    let score = result.score;
    let sum: u64 = result.promotions.iter().map(|&p| u64::from(p)).sum();
    if let Some(bag) = result.in_the_bag {
        if score != bag + sum {
            let why = "the moves and the frame model disagree";
            return reject(
                result,
                format!("the {method} moves score {score}, not inTheBag {bag} + their promotions' {sum}: {why}"),
            );
        }
    }
    // opt(n) ≤ n + opt(n − 1), so going past the ceiling proves the previous moves are not optimal.
    if let Some(cap) = result.setup.and_then(|s| s.cap) {
        if sum > cap {
            return reject(
                result,
                format!(
                    "the promotions' {sum} exceed the cap {cap} = n + score(n − 1) − inTheBag, which proves that the \
                     previous moves are not optimal"
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
                    "the moves score {score}, more than n + score(n − 1) = {ceiling}, which proves that the previous \
                     moves (from {whence}) are not optimal"
                ),
            );
        }
        if result.completed && score < prev.score {
            return reject(
                result,
                format!(
                    "the moves score {score}, less than the previous game's {} (from {whence}); moves for n − 1 are \
                     moves for n, so these are not optimal",
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
            return reject(result, format!("scored {score} but the table says {expected}"));
        }
        result.optimal_match = Some(true);
    }
    let proof = Proof::of(result.method, true, result.reached_cap).expect("a completed result has a proof kind");
    result.proof = Some(proof);
    // What the proof rests on besides the moves: the game a repeat repeats, else the moves for n − 1.
    let basis = match &result.same_search {
        Some(same) => Some(&same.assumes),
        None => context.prev.map(|p| &p.assumes),
    };
    result.assumes = assumptions_of(proof, basis);
    Ok(result)
}

/// The error for a checkpoint made with other search tactics than this run's: what differs, and the flags that
/// resume it.
fn different_tactics(checkpoint: &Path, made: SearchTactics, now: Tactics) -> String {
    let then = Tactics { search: made, ..now };
    let differ: Vec<_> = then.settings().into_iter().zip(now.settings()).filter(|(a, b)| a != b).collect();
    let what: Vec<String> = differ
        .iter()
        .map(|((name, there), (_, here))| format!("{name} is {}, not {}", on_off(*there), on_off(*here)))
        .collect();
    let flags: Vec<String> =
        differ.iter().map(|((name, there), _)| format!("--tactic {name}={}", on_off(*there))).collect();
    format!(
        "{} was made with different search tactics: {}; resume it with {}, or delete it to start the game over",
        checkpoint.display(),
        what.join(", "),
        flags.join(" ")
    )
}

/// Deletes game `n`'s checkpoint, if checkpointing is on.  Called only once the game's result is safe: in the
/// results file, or (without one) handed to the caller.
pub fn discard_checkpoint(opts: &SolveOptions, n: u32) {
    if let Some(path) = opts.checkpoint_file(n) {
        if let Err(e) = engine::remove_checkpoint(&path) {
            let _ = writeln!(std::io::stderr(), "warning: cannot remove the checkpoint of a finished game: {e}");
        }
    }
    clock::forget(opts, n);
}

/// Where the engine spills to, and whether the directory is the game's own, which its search removes when it is done.
/// A temporary directory is the game's own: games solved at the same time (by threads of one process) do not share
/// one, or one's removing it would fail the other's creating it.  The spill directory of the checkpoint directory is
/// shared by the games of a range, which may search side by side, so the range removes it when it ends
/// ([`shared_spill_dir`]); one the options name is the caller's.
pub(crate) fn spill_dir(opts: &SolveOptions, n: u32) -> (PathBuf, bool) {
    static TEMP_DIRS: AtomicU64 = AtomicU64::new(0);
    if let Some(dir) = opts.spill_dir.clone().or_else(|| shared_spill_dir(opts)) {
        (dir, false)
    } else {
        let name = format!("taxman-spill-{}-{}-n{n}", std::process::id(), TEMP_DIRS.fetch_add(1, Ordering::Relaxed));
        (std::env::temp_dir().join(name), true)
    }
}

/// The spill directory that the games of a range share, and that the range removes when it ends (if nothing is left
/// in it): `spill` in the checkpoint directory, unless the options name a spill directory.
fn shared_spill_dir(opts: &SolveOptions) -> Option<PathBuf> {
    match (&opts.spill_dir, &opts.checkpoint_dir) {
        (None, Some(dir)) => Some(dir.join("spill")),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------------------
// Repeats of earlier searches

/// How far back a game looks for an earlier game whose search problem it repeats (tactic `same_search`): the games
/// `n − SAME_SEARCH_WINDOW` to `n − 1`.  A board's frames change a little with each number: a game of
/// the committed chain whose problem an earlier game posed repeats the last game before it that set a search
/// up, `n − 1`, or `n − 2` when `n − 1` is prime (solver/docs/design/same-search.md).  The window
/// keeps the setups a range starts with to a few, and makes the game repeated a function of the window's games alone.
pub const SAME_SEARCH_WINDOW: u32 = 8;

/// An earlier completed game, as much of its result as a repeat of its search needs: the size and digest of its
/// problem, its promotions, and what a repeat of it rests on, its `basis` (a [`Basis`] in the proven mode, nothing in
/// the unsound mode, [`unsound`]).
#[derive(Clone, Debug)]
struct Searched<B> {
    /// The size of its problem as its setup recorded it, which the problem set up again must have.
    size: ProblemSize,
    /// The digest of its search problem, as its setup recorded it.
    digest: u64,
    promotions: Vec<u32>,
    basis: B,
}

/// What a repeat of a proven game rests on: that game's proof and assumptions.
#[derive(Clone, Debug)]
struct Basis {
    proof: Proof,
    assumes: Assumptions,
}

/// The games a game of a range may repeat the search of (tactic `same_search`): the completed games of the results
/// file and of this run that set a search up and whose proof settles it ([`Proof::settles_search`]), and the games
/// of the range in flight that set a search up and are not concluded yet (`pending`, see [`find`](Self::find)).  The
/// unsound mode keeps its own games, the lines of its file and its run that set a search up, with no basis.
struct Earlier<B = Basis> {
    games: BTreeMap<u32, Searched<B>>,
    /// The search problems of the boards this run set up ([`remember`](Self::remember)), by n, for the last
    /// [`SAME_SEARCH_WINDOW`] games: what a comparison with an earlier game needs, so that it need not set that
    /// game's board up again.
    setups: BTreeMap<u32, Problem>,
    /// The boards this run set up whose games are not concluded yet.
    pending: BTreeSet<u32>,
}

impl<B> Default for Earlier<B> {
    fn default() -> Self {
        Earlier { games: BTreeMap::new(), setups: BTreeMap::new(), pending: BTreeSet::new() }
    }
}

impl Earlier {
    /// The games of the results file.
    fn of_file(file: &ResultsFile) -> Earlier {
        let mut earlier = Earlier::default();
        for game in file.games().filter(|game| game.proof.settles_search()) {
            if let Some((size, digest)) = game.size.zip(game.digest) {
                let basis = Basis { proof: game.proof, assumes: game.assumes.clone() };
                earlier.add(game.n, size, digest, game.promotions.clone(), basis);
            }
        }
        earlier
    }

    /// Adds a game solved in this run, when it is concluded.
    fn add_result(&mut self, result: &GameResult) {
        self.pending.remove(&result.n);
        if let (true, Some(proof), Some(setup)) = (result.completed, result.proof, result.setup) {
            if proof.settles_search() {
                let basis = Basis { proof, assumes: result.assumes.clone() };
                self.add(result.n, setup.size(), setup.digest, result.promotions.clone(), basis);
            }
        }
    }

    /// The record that a repeat of the search of game `m` keeps, whose problem has the digest given, and `m`'s
    /// promotions, once `m` is among the games ([`find`](Self::find) found it).
    fn same_search(&self, m: u32, digest: u64) -> Option<(SameSearch, Vec<u32>)> {
        let game = self.games.get(&m)?;
        let promotion_sum = game.promotions.iter().map(|&p| u64::from(p)).sum();
        let Basis { proof, assumes } = game.basis.clone();
        let same = SameSearch { n: m, digest, promotion_sum, proof, assumes };
        Some((same, game.promotions.clone()))
    }
}

impl<B> Earlier<B> {
    /// Adds game `n`, whose problem has the size and the digest given, with its promotions and what a repeat of it rests
    /// on.
    fn add(&mut self, n: u32, size: ProblemSize, digest: u64, promotions: Vec<u32>, basis: B) {
        self.games.insert(n, Searched { size, digest, promotions, basis });
    }

    /// Remembers the search problem that this run set board `n` up with (at the repeat lookup of its game, in the
    /// order of n), with the game pending until it is concluded, and forgets the problems of the boards more than
    /// [`SAME_SEARCH_WINDOW`] sizes before the latest one remembered: no game that follows looks that far back.
    ///
    /// The problem that this run's own setup of board `m` poses is the problem `search_problem(m)` builds
    /// ([`Board::problem`] is the problem [`Board::prepare`] poses, which a test in `board.rs` checks), so a
    /// comparison may use either.
    fn remember(&mut self, n: u32, problem: Problem) {
        self.setups.insert(n, problem);
        self.pending.insert(n);
        let latest = self.setups.keys().next_back().copied().unwrap_or(n);
        self.setups.retain(|&m, _| latest - m <= SAME_SEARCH_WINDOW);
    }

    /// The latest game of the window before `n` ([`SAME_SEARCH_WINDOW`]) whose search problem is `problem`: the game
    /// whose search game `n` repeats.  A game whose digest is not `problem`'s is passed over; for one whose digest
    /// is, its problem (the one this run set its board up with when it did, else set up again from its n,
    /// [`search_problem`]) is compared with `problem` itself, so that the answer never rests on two digests alone.  A
    /// game whose problem is not as large as its line says (a line made by a build whose frame builder posed another
    /// problem) is dropped: a verifier would refuse it too.
    ///
    /// A pending game counts as well, compared by the problem this run set it up with.  It is the game that a range
    /// solved in order finds: there the pending game is concluded when `n` looks, and a game that set a search up
    /// and completes settles its problem (a completed search is exhausted or reaches its ceiling, a repeat repeats
    /// one that settles it), while one that does not complete ends the range, or fails it, before the line of `n`
    /// could be written.  A repeat of a pending game is concluded after it ([`same_search`](Self::same_search)).
    fn find(&mut self, n: u32, problem: &Problem) -> Option<u32> {
        let digest = problem.digest();
        for m in (n.saturating_sub(SAME_SEARCH_WINDOW)..n).rev() {
            if self.pending.contains(&m) {
                if self.setups.get(&m) == Some(problem) {
                    return Some(m);
                }
                continue;
            }
            let Some(game) = self.games.get(&m) else { continue };
            if game.digest != digest {
                continue;
            }
            let theirs = problem_of_board(&self.setups, m).filter(|theirs| theirs.size() == game.size);
            let Some(theirs) = theirs else {
                self.games.remove(&m);
                continue;
            };
            if *theirs == *problem {
                return Some(m);
            }
        }
        None
    }

    /// The promotions of game `m`, if it is among the games.
    fn promotions(&self, m: u32) -> Option<&[u32]> {
        self.games.get(&m).map(|game| game.promotions.as_slice())
    }
}

/// The search problem of board `m`: the one in `setups` (the boards this run set up), else the one set up again from
/// `m` (a game of the results file), if the board can be set up.  The two are the same problem: the one a run's own
/// setup of the board posed is what `search_problem(m)` builds, as [`Earlier::remember`] says.
fn problem_of_board(setups: &BTreeMap<u32, Problem>, m: u32) -> Option<Cow<'_, Problem>> {
    match setups.get(&m) {
        Some(problem) => Some(Cow::Borrowed(problem)),
        None => search_problem(m).ok().map(|(problem, _)| Cow::Owned(problem)),
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
    /// Lines that the unsound mode's repair replaced, and the time its walks down took, those that replaced nothing
    /// included (none in the proven mode).
    pub repaired: usize,
    pub repair_time: Duration,
    /// Lines that the unsound mode appended to its best-known file (none in the proven mode): those of its file
    /// offered when the run opened it, the games and repairs it recorded, and the repairs of the best-known file's own
    /// lines.
    pub best_known_added: usize,
    /// Of those, the repairs of the best-known file's lines of the games below the run's first, which the run has no
    /// line of.
    pub best_known_repaired: usize,
    /// The game whose search stopped at a limit, or at a signal; the range ends there.
    pub stopped_at: Option<u32>,
}

/// The completed line for `n` in the results file, as the previous moves for `n + 1`.
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

/// The moves for `n − 1` from the results file, if it has them.
fn previous_game(n: u32, results: Option<&ResultsFile>) -> Option<Prev> {
    let before = n.checked_sub(1).filter(|&before| before >= 1)?;
    results.and_then(|r| stored_prev(r, before))
}

/// Says on stderr that game `n` has no previous moves, so that it is solved standalone: correct, but without the seed
/// and the ceiling that the moves for `n − 1` give, so it costs more
/// (https://github.com/bvchess/taxman-notebook/blob/main/search/benchmarks-the-probe-pgo-and-scaling/cost-curve.md
/// measures how much).
fn note_standalone(n: u32) {
    let before = n - 1;
    let _ = writeln!(
        std::io::stderr(),
        "note: no previous game for n={n} (the game of {before} is neither in this run nor in the results file): \
         solving it standalone, with no seed and no ceiling, which costs more; to chain, give --out with the earlier \
         games"
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

/// Solves the games `lo..=hi`, each from the moves for the one before, and concludes them in order (with
/// up to [`SolveOptions::window`] games in flight, see `window`).  The moves for `lo − 1` come from the results
/// file, else there are none.  The results file is checked when it is opened ([`ResultsFile::open`]); games already
/// completed in it are skipped (and a checkpoint left for one is deleted).
/// Every other result is appended to the file (if there is one), then given to `on_game`, and only then is its
/// checkpoint deleted.  A result whose moves fail a check after they were found goes to the file's
/// [`rejected_path`](crate::results::rejected_path) instead, and the run fails.  The range stops at a game whose
/// search did not complete.  Scores are confirmed against the table ([`Optimal::built_in`]).
pub fn solve_range_each(
    lo: u32,
    hi: u32,
    opts: &SolveOptions,
    results_path: Option<&Path>,
    on_game: impl FnMut(&GameResult),
) -> Result<RangeSummary, String> {
    solve_range_each_with(lo, hi, opts, results_path, Optimal::built_in(), on_game)
}

/// [`solve_range_each`] with the table `optimal` in place of the built-in one, for tests.
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
    // The searches write their profiles and stores into these folders as they go, so they are made now: one that
    // cannot be made fails the run before its first game, as the results file that is opened next does.
    for (what, dir) in [("profile", &opts.profile), ("carried-nogoods", &opts.carried_dir)] {
        if let Some(dir) = dir {
            fs::create_dir_all(dir).map_err(|e| format!("cannot make the {what} folder {}: {e}", dir.display()))?;
        }
    }
    let mut results = results_path.map(|path| ResultsFile::open(path, optimal, opts.threads)).transpose()?;
    if let Some(file) = &results {
        // Left by a crash between saving a result and deleting its checkpoint.
        for n in (lo..=hi).filter(|&n| file.completed(n).is_some()) {
            if [opts.checkpoint_file(n), opts.clock_file(n)].iter().flatten().any(|path| path.exists()) {
                discard_checkpoint(opts, n);
            }
        }
    }
    let first = previous_game(lo, results.as_ref());
    // A first game the file settles is not solved standalone by this run.
    if lo > 1 && first.is_none() && !results.as_ref().is_some_and(|file| file.completed(lo).is_some()) {
        note_standalone(lo);
    }
    let mut earlier = results.as_ref().map(Earlier::of_file).unwrap_or_default();
    let before = first.map_or(Before::None, Before::Given);
    let range = window::Range { lo, hi, before, optimal, results: results.as_mut() };
    window::solve(range, opts, &mut earlier, &mut on_game)
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
        assert_eq!(json.get("options").unwrap().get("force_search"), Some(&Json::Bool(false)));
        let contents = crate::results::read_results(&format!("{json}\n"), Optimal::built_in(), 1).unwrap();
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

        // Extending the range uses the moves stored for 12 as the previous moves.
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
        assert!(ResultsFile::open(&path, Optimal::built_in(), 1).is_err());
    }

    #[test]
    fn a_range_that_starts_late_has_no_previous_moves_and_its_first_game_is_solved_standalone() {
        let r = solve_range(50, 52, &one_thread(), None).unwrap();
        assert_eq!((r[0].prev_source(), r[1].prev_source()), (None, Some(PrevSource::Run)));
        assert!(r.iter().all(|g| g.assumes.is_empty()), "standalone, and chained from it, nothing is assumed");
        let first = solve_range(1, 1, &one_thread(), None).unwrap();
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
        let prev = |moves: &[u32], score, source| Prev {
            moves: moves.to_vec(),
            score,
            source,
            proof: Some(Proof::Exhausted),
            assumes: Assumptions::new(),
        };
        // [5, 4, 6] are the moves for 6, scoring 15.
        let source = PrevSource::Results;
        assert!(solve_game(7, Some(&prev(&[5, 4, 6], 15, source)), &opts).is_ok());
        let err = solve_game(7, Some(&prev(&[5, 4, 6], 14, source)), &opts).unwrap_err();
        assert!(err.contains("score 15, not the 14"), "{err}");
        let err = solve_game(7, Some(&prev(&[5, 4, 4], 13, source)), &opts).unwrap_err();
        assert!(err.contains("not legal for n=6"), "{err}");
        // Moves this run made were refereed by `conclude` before they became the previous moves, so they are not
        // checked again: a wrong score goes unnoticed (the moves for 7 built from it are still refereed), and moves
        // that are not legal fail only in the moves built from them.
        assert!(solve_game(7, Some(&prev(&[5, 4, 6], 14, PrevSource::Run)), &opts).is_ok());
        let err = solve_game(7, Some(&prev(&[5, 4, 4], 13, PrevSource::Run)), &opts).unwrap_err();
        assert!(err.contains("do not stand up") && !err.contains("not legal for n=6"), "{err}");
    }

    /// Solves the games `lo..=hi` as a range does (its driver, [`window::solve`], on one thread, so one game after
    /// the other), each from the one before (the first from the table's moves for the game before it, given as a line
    /// of a results file would give them), with the `Earlier` that the run keeps; returns it and the results.
    fn solved_with_earlier(lo: u32, hi: u32) -> (Earlier, Vec<GameResult>) {
        let (opts, optimal) = (one_thread(), Optimal::built_in());
        let mut earlier = Earlier::default();
        let table_prev = |n: u32| {
            let (moves, score) = (optimal.moves(n)?.to_vec(), optimal.score(n)?);
            Some(Prev {
                moves,
                score,
                source: PrevSource::Results,
                proof: Some(Proof::Exhausted),
                assumes: Assumptions::new(),
            })
        };
        let before = table_prev(lo - 1).map_or(Before::None, Before::Given);
        let range = window::Range { lo, hi, before, optimal, results: None };
        let mut results = Vec::new();
        window::solve(range, &opts, &mut earlier, &mut |result| results.push(result.clone())).unwrap();
        (earlier, results)
    }

    /// The boards 300 and 301 pose one search problem (as the committed chain, `results/optimal.jsonl.gz`, has it): 301
    /// repeats 300's search.  The `Earlier` of a run finds that out from the problem that the run set 300 up with, as
    /// it does from the one set up again, and compares that problem, not another.
    #[test]
    fn a_problem_the_run_set_up_is_compared_as_the_one_set_up_again_is() {
        let (earlier, results) = solved_with_earlier(300, 301);
        assert_eq!((results[0].method, results[1].method), (Method::Search, Method::Repeat));
        // The run kept the problem of each board it set up, repeats included, and concluded both games.
        assert_eq!(earlier.setups.keys().copied().collect::<Vec<_>>(), [300, 301]);
        assert!(earlier.pending.is_empty());
        let ours = search_problem(301).unwrap().0;
        assert_eq!(earlier.setups[&301], ours);
        assert_eq!(earlier.setups[&300], search_problem(300).unwrap().0);

        // The same games, with nothing remembered, find the same game.
        let mut set_up_again = Earlier { games: earlier.games.clone(), ..Earlier::default() };
        let mut remembered =
            Earlier { games: earlier.games.clone(), setups: earlier.setups.clone(), ..Earlier::default() };
        let found = remembered.find(301, &ours);
        assert_eq!(found, set_up_again.find(301, &ours));
        assert_eq!(found, Some(300), "301 repeats 300");
        let (same, promotions) = remembered.same_search(300, ours.digest()).unwrap();
        assert_eq!(promotions, results[0].promotions);
        assert_eq!(Some(&same), results[1].same_search.as_ref());

        // What is remembered is what is compared: were another board's problem remembered for 300, 301 would not
        // repeat it, though the digest the game of 300 recorded is 301's.
        let setups = BTreeMap::from([(300, search_problem(310).unwrap().0)]);
        let mut wrong = Earlier { games: earlier.games.clone(), setups, ..Earlier::default() };
        assert!(wrong.find(301, &ours).is_none());
    }

    /// A game in flight that set a search up is found as it is once concluded, by the problem the run set it up with:
    /// 301 repeats 300 while 300 is pending, and the record of the repeat is the one made once 300 is concluded.
    #[test]
    fn a_game_in_flight_is_found_as_the_concluded_game_is() {
        let (concluded, results) = solved_with_earlier(300, 301);
        let ours = search_problem(301).unwrap().0;
        let mut in_flight = Earlier::default();
        in_flight.remember(300, search_problem(300).unwrap().0);
        assert_eq!(in_flight.find(301, &ours), Some(300));
        assert!(in_flight.same_search(300, ours.digest()).is_none(), "no record while 300 is in flight");
        in_flight.add_result(&results[0]);
        assert!(in_flight.pending.is_empty());
        assert_eq!(in_flight.find(301, &ours), Some(300));
        assert_eq!(in_flight.same_search(300, ours.digest()), concluded.same_search(300, ours.digest()));
        // Pending with another problem, it is passed over.
        let mut other: Earlier = Earlier::default();
        other.remember(300, search_problem(310).unwrap().0);
        assert_eq!(other.find(301, &ours), None);
    }

    #[test]
    fn only_the_problems_within_the_window_are_remembered() {
        let problem = search_problem(85).unwrap().0;
        let mut earlier: Earlier = Earlier::default();
        for n in [300, 301, 302, 309, 310] {
            earlier.remember(n, problem.clone());
        }
        // Boards up to SAME_SEARCH_WINDOW sizes before the latest, 310.
        assert_eq!(earlier.setups.keys().copied().collect::<Vec<_>>(), [302, 309, 310]);
        earlier.remember(311, problem.clone());
        assert_eq!(earlier.setups.keys().copied().collect::<Vec<_>>(), [309, 310, 311]);
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
