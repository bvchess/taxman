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
//! 1. The tables of the board `1..=n`, and the set width `W` for it (`with_width!`; a checkpoint made at another
//!    width is refused).
//! 2. The *prime shortcut* (tactic `prime_shortcut`): a prime `n` plays the game of `n − 1` with its first move
//!    replaced by `n` (method `prime`).
//! 3. The board's frames ([`Board`]).  With no promotion candidates the game with no promotions is the only one
//!    (method `trivial`).
//! 4. Otherwise the search (method `search`): [`Board::prepare`] (or [`Board::prepare_for_resume`]), then
//!    [`search::search`] for the promotion set with the largest sum.  It starts from the better of the greedy set
//!    and the previous game's promotions that are candidates here (`seed_incumbent`, [`Board::seed`]) and stops
//!    as soon as it reaches `n + opt(n − 1) − in_the_bag` (`prev_cap`, [`Board::promotion_cap`]).  The promotions
//!    become a game through the frames ([`solution_for_promotions`]).  Between the setup and the search, the
//!    tactic `same_search` may find that an earlier game already settled this search (method `repeat`, below).
//! 5. `conclude`: the game is checked before it counts, and given its proof and assumptions.
//! 6. Recording (a range only): the result is appended to the results file and synced, and only then is the
//!    game's checkpoint deleted.
//!
//! Step 2 and the repeat of step 4 are skipped by `force_search`, and when a checkpoint of the game exists: a search
//! that was interrupted is resumed, never replaced by a derivation.
//!
//! The steps run in four phases, which the driver of a range (`window`) composes, a single game included: the
//! *setup* (`set_up`: the checks of a previous game, steps 1 to 3, and the setup of step 4), on a thread of the
//! game's own; the *repeat lookup* (the repeat of step 4, `Earlier::find`), the *search* (the rest of step 4, run
//! through a width-free `Job`), and the *conclusion* (`conclude_game`: step 5, and step 6 for a range).
//!
//! # Repeats of earlier searches
//!
//! Consecutive boards often pose exactly the same search problem ([`search::Problem`]: the frames, the candidates,
//! `max_total` and the factor graph among their numbers), only with another ceiling.  With the tactic `same_search`
//! (proven, on by default), a game whose problem an earlier game `m` of the range or of the results file posed, within
//! the [`SAME_SEARCH_WINDOW`] board sizes before it, and whose proof settles that problem's best promotion sum
//! ([`Proof::settles_search`]), is `m`'s promotions played on its own board, with no search: method `repeat`, proof
//! `same_search`, resting on `m`'s proof and its assumptions.  The game is found by the problem's digest and confirmed
//! by comparing the two problems, taking `m`'s from this run's own setup of its board when there was one and setting
//! it up again otherwise; the game then goes through `conclude` like any game.  DESIGN.md, "Same search", has why it
//! is optimal.
//!
//! The previous game is checked before anything uses it: it must replay legally to its recorded score, be over
//! (no legal move left) and open with the largest prime, as every optimal game does.  A game this run made is
//! not checked again: `conclude` made the same check before the game was recorded.
//!
//! `conclude` checks that the game replays legally ([`replay`]); that a completed game is also over and opens with
//! the largest prime; that a game from the frame model scores `in_the_bag` plus its promotions; that no game
//! scores more than `n + score(n − 1)` (nor its promotions exceed the ceiling), which would prove the previous game
//! was not optimal, and that a completed game does not score less than the previous one; and, for a completed
//! game, that its score is `optimal.json`'s when that has the game (the table built into the binary,
//! [`Optimal::built_in`]).  Each failure is a hard error, and a game that fails after it was found is recorded,
//! with its promotions, in the results file's [`rejected_path`](crate::results::rejected_path).
//!
//! # A range
//!
//! [`solve_range_each`] solves the board sizes `lo..=hi` and concludes them in order.  The previous game of each is
//! the one before it; for `lo` it is the results file's game of `lo − 1`, else, with [`SolveOptions::use_prev`],
//! `optimal.json`'s (assumed optimal, which the results that rest on it record), else there is none and the game is
//! solved *standalone*, with no seed and no ceiling (and a note on stderr says so).  Every result is appended to
//! the results file (one JSON object per line) as soon as it is concluded, so that a restarted run skips what is
//! finished; the file is checked when it is opened ([`ResultsFile::open`]).  A game whose search a limit or a signal
//! stopped is recorded as incomplete, keeps its checkpoint, and ends the range; so does one that passes the wall
//! ([`SolveOptions::game_limit`]), but with no line.  (With [`SolveOptions::keep_going`] a limit does not end the
//! range: the next game goes on from the unfinished game's best game as a seed alone, see `window`, "Past limits".)
//! The games a game of the range may
//! repeat the search of are the file's and the range's own (a single game solved with [`solve_game`] has none).
//!
//! Up to [`SolveOptions::window`] consecutive games are in flight at once: game `n + 1` is set up while `n`
//! searches, and searched from `n`'s best game so far, so that a machine of many threads is kept busy.  With a
//! window of one, and always with one thread, the range is solved strictly in order, game after game.  The lines
//! are the same either way but for what a multi-threaded search may vary (another optimal set of promotions, the
//! nodes, the times): `window` has the driver and the argument.

mod clock;
mod window;

use crate::accelerate::{prime_shortcut, solution_for_promotions};
use crate::bits::{words_for, Bits};
use crate::board::{promotion_cap, search_problem, Board};
use crate::engine::{self, Control, Counters, EngineConfig, Stats};
use crate::frames::BoardFrames;
use crate::game::{replay, replay_finished, Optimal};
use crate::json::Json;
use crate::known::{Bound, Cite, Known, KnownGame, KnownSources, Open, Origin};
use crate::numtheory::Tables;
use crate::results::{
    assumed_optimal, assumptions_json, assumptions_of, build_json, moves_digest, Assumptions, Proof, ResultsFile,
    SCHEMA,
};
use crate::search::{self, stat, Prepared, Problem, ProblemSize, SearchTactics};
use crate::tactics::Tactics;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// The largest board the set widths cover: 8191, at the widest of [`crate::bits::WIDTHS`] (128 words).
pub const MAX_N: u32 = 64 * crate::bits::WIDTHS[crate::bits::WIDTHS.len() - 1] as u32 - 1;

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
    /// The wall (`--game-hours`): the most a game's own search may take in all, over every invocation.  A game's own
    /// search is the time it has been the head of the window with its search running, not the time it waited behind
    /// another game ([`window`]'s "The wall"); the time is kept beside the game's checkpoint, so a resumed search goes
    /// on from it.  The game that passes it ends the range with every search in flight stopped and checkpointed and
    /// nothing recorded for it ([`RangeSummary::walled`]).
    pub game_limit: Option<Duration>,
    /// Most queued search units kept in memory; the rest spill to disk.
    pub queue_mem_cap: usize,
    /// Where queued units spill to; by default `spill` in the checkpoint directory, else a temporary directory.
    pub spill_dir: Option<PathBuf>,
    /// Search even where the previous game or an earlier search would give the answer (skips `prime_shortcut` and
    /// `same_search`).
    pub force_search: bool,
    /// Draw on the known games ([`crate::known`]; `--use-prev`): take the game of `n − 1` of a range's first game from
    /// them when neither the run nor the results file has it (`optimal.json`'s, assumed optimal, which the results whose
    /// proofs rest on it record; the committed chain's, the bounds store's, with what they assume), and let the bounds
    /// store help the search of a board it has not proven (its lower bound's game seeds, its upper bound is a ceiling).
    /// Without it such a game is solved standalone.  [`known`](Self::known) says which files, and whether the boards they prove
    /// are recorded without a search.
    pub use_prev: bool,
    /// The files [`use_prev`](Self::use_prev) draws on besides `optimal.json`, and whether it goes as far as the boards of
    /// the range (`solve --use-prev` does); none by default.
    pub known: KnownSources,
    /// A game whose search a time or node limit stops does not end the range (`--keep-going`): its best game so far
    /// is recorded as an unfinished line, and the next game goes on from it as a seed and nothing more.  The unfinished
    /// game is not known to be optimal, so the next game gets no ceiling from it, no prime formula and no repeat that
    /// rests on it ([`Prev::proven`]).  A rerun passes over the games the file holds only unfinished lines of
    /// ([`retry`](Self::retry) searches them again), and a stop by a signal still ends the range, with nothing
    /// recorded for the game it stopped, so that a rerun resumes it.  Such a range is solved one game at a time
    /// (`window` is 1): a game that goes on from a game still in flight could not tell whether it was proven.
    pub keep_going: bool,
    /// A number recorded in `options.pass` of every line this run writes (`--pass N`): which pass over the games made
    /// it, when a script (`rust/tools/cloud/sweep.sh`) goes over them more than once.  It changes nothing else.
    pub pass: Option<u32>,
    /// With [`keep_going`](Self::keep_going), search again the games the results file holds only unfinished lines of
    /// (`--retry`; each resumes from its checkpoint if it has one), instead of passing over them.  A pass with a longer
    /// limit retries the failures of an earlier one this way.
    pub retry: bool,
    /// With [`keep_going`](Self::keep_going), search again only the unfinished games whose game of `n − 1` now has more
    /// to give than it had when the game was last tried (`--retry-improved`): it is proven and the line's was not, or
    /// it has a better best game than the line's seed, or the line had no game of `n − 1`.  The others are passed over.
    /// One pass of a script (`rust/tools/cloud/sweep.sh`) that goes over blocks of games made apart, so that every
    /// game sees the one before it whichever block made it.
    pub retry_improved: bool,
    /// The games of a range in flight at once (`--window`, at least 1, 4 by default: the best measured at 64 threads,
    /// see `window`), which share the `threads`.  With one thread it is 1 whatever it says: there is no thread to run
    /// a second game on.
    pub window: usize,
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
            game_limit: None,
            queue_mem_cap: engine.queue_mem_cap,
            spill_dir: None,
            force_search: false,
            use_prev: false,
            known: KnownSources::default(),
            keep_going: false,
            pass: None,
            retry: false,
            retry_improved: false,
            window: 4,
        }
    }
}

impl SolveOptions {
    /// The checkpoint file of game `n`, if checkpointing is on.
    pub fn checkpoint_file(&self, n: u32) -> Option<PathBuf> {
        self.checkpoint_dir.as_ref().map(|dir| dir.join(format!("n{n:04}.ckpt")))
    }

    /// The file beside game `n`'s checkpoint that keeps its wall clock between invocations, if checkpointing is on.
    pub fn clock_file(&self, n: u32) -> Option<PathBuf> {
        self.checkpoint_dir.as_ref().map(|dir| dir.join(format!("n{n:04}.clock")))
    }
}

/// Where the solution of the previous game came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrevSource {
    /// Solved earlier in this run: refereed by `conclude` already, so not checked again before it is used.
    Run,
    /// Read from the results file.
    Results,
    /// Read from `optimal.json`.
    Optimal,
    /// Read from the committed chain, `results/strict.jsonl` (`--use-prev`).
    Strict,
    /// A board the bounds store has proven (`--use-prev`).
    Bounds,
}

impl PrevSource {
    pub const ALL: [PrevSource; 5] =
        [PrevSource::Run, PrevSource::Results, PrevSource::Optimal, PrevSource::Strict, PrevSource::Bounds];

    /// The name in result lines.
    pub fn as_str(self) -> &'static str {
        match self {
            PrevSource::Run => "run",
            PrevSource::Results => "results",
            PrevSource::Optimal => "optimal",
            PrevSource::Strict => "strict",
            PrevSource::Bounds => "bounds",
        }
    }

    /// Where it came from, for a message: "this run", "the results file", "optimal.json", ...
    pub fn whence(self) -> &'static str {
        match self {
            PrevSource::Run => "this run",
            PrevSource::Results => "the results file",
            PrevSource::Optimal => Origin::Optimal.whence(),
            PrevSource::Strict => Origin::Strict.whence(),
            PrevSource::Bounds => Origin::Bounds.whence(),
        }
    }
}

/// The solution of game `n − 1`.
#[derive(Clone, Debug)]
pub struct Prev {
    pub moves: Vec<u32>,
    pub score: u64,
    pub source: PrevSource,
    /// How its optimality is established; `None` if that is not known (`optimal.json` does not say), and for the best
    /// game of a search that a limit stopped ([`proven`](Self::proven) false).
    pub proof: Option<Proof>,
    /// What its proof assumes (see [`crate::results`]).
    pub assumes: Assumptions,
    /// Whether the game is optimal, given what it assumes: false for the best game found by a search that a limit
    /// stopped (`--keep-going`), which can seed the next game and do no more.  Only a proven game gives the next game
    /// its ceiling `n + score(n − 1)` and the prime formula ([`cap_score`](Self::cap_score)).
    pub proven: bool,
}

impl Prev {
    /// The game for board size `n` in `optimal`, if it has one.  The file records no proof, so the game is assumed
    /// optimal ([`assumed_optimal`]).
    pub fn from_optimal(optimal: &Optimal, n: u32) -> Option<Prev> {
        let (moves, score) = (optimal.moves(n)?, optimal.score(n)?);
        let assumes = Assumptions::from([assumed_optimal(n)]);
        Some(Prev { moves: moves.to_vec(), score, source: PrevSource::Optimal, proof: None, assumes, proven: true })
    }

    /// The game a known source holds ([`Known::game`]), with what it rests on.
    pub fn from_known(game: &KnownGame) -> Prev {
        let source = match game.origin {
            Origin::Optimal => PrevSource::Optimal,
            Origin::Strict => PrevSource::Strict,
            Origin::Bounds => PrevSource::Bounds,
        };
        Prev {
            moves: game.moves.clone(),
            score: game.score,
            source,
            proof: game.proof,
            assumes: game.assumes.clone(),
            proven: true,
        }
    }

    /// A result of this run: a completed game, or the best game of a search that a limit stopped, which is not proven.
    fn from_result(result: &GameResult) -> Prev {
        Prev {
            moves: result.moves.clone(),
            score: result.score,
            source: PrevSource::Run,
            proof: result.proof,
            assumes: result.assumes.clone(),
            proven: result.completed,
        }
    }

    /// The score that gives the next game its ceiling: the game's, if it is proven.
    pub fn cap_score(&self) -> Option<u64> {
        self.proven.then_some(self.score)
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
    /// opening with the largest prime.  (Optimal itself cannot be checked; see the tripwires in [`conclude`].)  The
    /// best game of a stopped search is checked for the first two only, as `conclude` checks it.
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
        if !self.proven {
            return Ok(());
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
    /// Taken from a known source (`--use-prev`, [`crate::known`]): the game, with no search.
    Known,
}

impl Method {
    pub const ALL: [Method; 5] = [Method::Trivial, Method::Prime, Method::Repeat, Method::Search, Method::Known];

    pub fn as_str(self) -> &'static str {
        match self {
            Method::Trivial => "trivial",
            Method::Prime => "prime",
            Method::Repeat => "repeat",
            Method::Search => "search",
            Method::Known => "known",
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

/// The known source a result's game, or its proof, comes from (`known` of its line): the citation, and what the cited
/// lines make the game assume.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cited {
    pub cite: Cite,
    pub assumes: Assumptions,
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
    /// Forming, numbering and merging the frames, and the search model.
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
    pub keep_going: bool,
    pub retry: bool,
    pub retry_improved: bool,
    pub pass: Option<u32>,
    /// `Some(true)` when `optimal.json` has the game, whose score the game's equals (else it is rejected); `None`
    /// if the game is incomplete or the table lacks `n`.
    pub optimal_match: Option<bool>,
    /// The game of `n − 1` this one was built from.
    pub prev: Option<PrevInfo>,
    /// For a repeat (method [`Method::Repeat`], proof `same_search`), the earlier game whose search it repeats.
    pub same_search: Option<SameSearch>,
    /// For a game taken from a known source (method [`Method::Known`]), or found by a search that an upper bound of the
    /// bounds store stopped (method [`Method::Search`]; both proof `known`), the source it cites.
    pub known: Option<Cited>,
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
            keep_going: opts.keep_going,
            retry: opts.retry,
            retry_improved: opts.retry_improved,
            pass: opts.pass,
            optimal_match: None,
            prev: prev.map(|p| p.info(n)),
            same_search: None,
            known: None,
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
        // The flags of a `--keep-going` run, and its pass, are there only when given: the lines of any other run are what
        // they were.
        let keep_going = self.keep_going.then_some(("keep_going", Json::Bool(true)));
        let retry = self.retry.then_some(("retry", Json::Bool(true)));
        let improved = self.retry_improved.then_some(("retry_improved", Json::Bool(true)));
        let pass = self.pass.map(|pass| ("pass", Json::from(pass)));
        let options = Json::object(
            [("force_search", Json::Bool(self.force_search)), ("use_prev", Json::Bool(self.use_prev))]
                .into_iter()
                .chain(keep_going)
                .chain(retry)
                .chain(improved)
                .chain(pass),
        );
        // The citation of a known source is there only on a line that has one: the lines made before it do not.
        let known = self.known.as_ref().map(|cited| ("known", cited.cite.to_json()));
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
            .chain(known)
            .chain([
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
        let of = match (&self.same_search, &self.known) {
            (Some(same), _) => format!(" of {}", same.n),
            (None, Some(cited)) => format!(" from {}", cited.cite.origin().whence()),
            (None, None) => String::new(),
        };
        let proof = self.proof.map_or_else(String::new, |p| format!("  proof={}{of}", p.as_str()));
        let assumes = if self.assumes.is_empty() {
            String::new()
        } else {
            format!("  assumes {}", self.assumes.iter().cloned().collect::<Vec<_>>().join("; "))
        };
        // A game built on the best game of an unfinished search is said to be: it had a seed and no ceiling.
        let seeded = match &self.prev {
            Some(prev) if prev.proof.is_none() && prev.source != PrevSource::Optimal => {
                format!("  seeded by the unfinished game of {} (no ceiling)", prev.n)
            }
            _ => String::new(),
        };
        format!(
            "n={:<5} score={:<9} {:<7} nodes={:<11} {:>9.3} s  {optimal}{proof}{assumes}{seeded}",
            self.n,
            self.score,
            self.method.as_str(),
            self.nodes,
            self.phases.total.as_secs_f64()
        )
    }
}

// ---------------------------------------------------------------------------------------------------------
// One game, in phases

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

/// The failure of game `n`, saying which game it is.
fn at(n: u32, failure: Failure) -> Failure {
    let say = |message: String| format!("n={n}: {message}");
    match failure {
        Failure::Error(message) => Failure::Error(say(message)),
        Failure::Rejected(result, message) => Failure::Rejected(result, say(message)),
    }
}

/// Solves board size `n`.  `prev` is the solution of `n − 1` if known.  The score is confirmed against
/// `optimal.json` ([`Optimal::built_in`]).
pub fn solve_game(n: u32, prev: Option<&Prev>, opts: &SolveOptions) -> Result<GameResult, String> {
    solve_game_with(n, prev, opts, Optimal::built_in())
}

/// [`solve_game`] with the score confirmed against the table `optimal` instead: for tests, and for `taxman bench`,
/// whose known games extend the table.  The checkpoint of a completed game is deleted once the game has passed its
/// checks.  This is the driver of a range (`window`) over the one board size `n`, from `prev`, with no results file
/// and no earlier games to repeat.
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
    let known = Known::load(optimal, &opts.known)?;
    let range = window::Range { lo: n, hi: n, before, known: &known, results: None };
    let mut solved = None;
    window::solve(range, opts, &mut Earlier::default(), &mut |result| solved = Some(result.clone()))?;
    Ok(solved.expect("a range of one game that does not fail concludes it"))
}

/// The game of `n − 1` as the setup of game `n` knows it.
#[derive(Clone, Debug)]
enum Before {
    /// Not made by this run (the results file's, `optimal.json`'s or a caller's): known from the start, and checked
    /// by the setup.
    Given(Prev),
    /// This run's: known once it is concluded, and refereed by `conclude` before that.
    Run,
    /// None: the game is solved standalone, with no seed and no ceiling.
    None,
}

impl Before {
    /// Whether the game of `n − 1` is one the prime formula may build on: there is one, and it is proven.  (A game
    /// made by this run is proven, unless the range keeps going past limits, and then its game is given whole:
    /// `Given`.)
    fn proven(&self) -> bool {
        match self {
            Before::Given(prev) => prev.proven,
            Before::Run => true,
            Before::None => false,
        }
    }
}

/// What the setup of a game (its first phase, [`set_up`]) made of it.
enum SetUp {
    /// A game concluded from its setup alone.
    Derived(Derived),
    /// A search problem, and the problem it poses, for the repeat lookup.
    Search(SearchSetUp, Problem),
}

/// A game that needs no search.
enum Derived {
    /// The prime shortcut: the promotion candidates of the board with no promotions, ascending.  The game, the one of
    /// `n − 1` with its first move replaced by `n`, is made when it is concluded.
    Prime { candidates: Vec<u32> },
    /// No promotion candidates: the game with no promotions.
    Trivial { plain: Vec<u32>, in_the_bag: u64, frames: Duration },
    /// A game a known source holds (`--use-prev`), and the promotion candidates of the board with no promotions,
    /// ascending, which the game's promotions are taken from as the prime game's are.
    Known { game: Box<KnownGame>, candidates: Vec<u32> },
}

/// An upper bound of the bounds store on a board that a search is to find the game of, which is a ceiling of the search
/// besides the one its previous game gives: it stops when its incumbent reaches the bound, which proves that game optimal
/// given what the bound assumes.
struct StoreCap {
    /// The bound on the promotion sum: the store's bound less the board's `in_the_bag`.
    sum: u64,
    bound: Bound,
}

/// A game set up for a search.
struct SearchSetUp {
    job: Arc<dyn Job>,
    /// What the search was set up with; its ceiling (`cap`) is filled in when the game is concluded.
    setup: SearchSetup,
    in_the_bag: u64,
    /// The bounds store's upper bound of the board, if it has one and `--use-prev` is on.
    store_cap: Option<StoreCap>,
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
    /// The game of `n − 1`, final or provisional, whose promotions seed the search: given when the search takes a
    /// seed ([`Job::seeds`]) and the game has a previous game.
    seed: Option<Vec<u32>>,
    /// The ceiling on the promotion sum, given when the game of `n − 1` is concluded and `prev_cap` is on.
    cap: Option<u64>,
    /// The workers the search starts with.
    threads: usize,
}

/// What came of a game's search, whatever its set width.
struct Outcome {
    /// The best promotion set found (the optimum when completed), ascending, and the game it makes.
    promotions: Vec<u32>,
    moves: Vec<u32>,
    /// The search finished, or reached a ceiling it was given; false when a limit or a stop stopped it.
    completed: bool,
    stats: Stats,
    counters: Counters,
    /// Of this invocation, and over every invocation of a resumed search.
    time: Duration,
    time_all_runs: Duration,
}

/// A game set up for a search, whatever its set width: what the game's thread and the driver of the range need of it.
/// The thread runs the search ([`search`](Job::search)); the driver, meanwhile, reads the best promotions so far for
/// the games that follow, weighs what a seed would give the search, and gives the search its ceiling and seeds through
/// its control.
trait Job: Send + Sync {
    /// The game that `promotions` (a promotable set of the board's candidates) make on the board.
    fn moves_for(&self, promotions: &[u32]) -> Vec<u32>;
    /// The search's best promotions so far, if it has any ([`search::best_so_far`]): its engine's incumbent while it
    /// runs and after it ended, else the best offered to it.
    fn best_so_far(&self) -> Option<Vec<u32>>;
    /// Whether the game has a checkpoint, whose search is resumed, never replaced by a repeat.
    fn resuming(&self) -> bool;
    /// Whether the search starts from a seed: with `seed_incumbent`, unless it is resumed (a resumed search has its
    /// incumbent in its checkpoint).
    fn seeds(&self) -> bool;
    /// Offers the search, if it takes a seed, the promotions of the game `moves` of `n − 1` that can still be promoted
    /// here, as an incumbent ([`search::offer_seed`]).
    fn hint(&self, moves: &[u32]);
    /// The promotion sum of what survives here of the game `moves` of `n − 1` as a seed
    /// ([`search::seed_survivors_sum`]): a search seeded with that game starts at least that high.
    fn survivors(&self, moves: &[u32]) -> u64;
    /// The control through which the driver steers the search ([`engine::Control`]).
    fn control(&self) -> &Control;
    /// The search, from the go (the game's third phase), and the game its promotions make.  The game's checkpoint is
    /// left in place, for the caller to delete once the result is safe ([`discard_checkpoint`]).
    fn search(&self, go: Go, opts: &SolveOptions) -> Result<Outcome, String>;
}

/// The [`Job`] of a board of set width `W`.
struct SearchJob<const W: usize> {
    board: Board<W>,
    prepared: Prepared<W>,
    control: Arc<Control>,
    resuming: bool,
    seeds: bool,
    /// The game of the bounds store's best lower bound of the board, replayed by the referee, if `--use-prev` is on and
    /// the search takes a seed: a seed beside the game of `n − 1`, the better of the two used.
    store_game: Option<Vec<u32>>,
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

    fn control(&self) -> &Control {
        &self.control
    }

    fn search(&self, go: Go, opts: &SolveOptions) -> Result<Outcome, String> {
        let n = self.board.n();
        // The game of `n − 1` (what is left of it here) and the store's game of `n`: the one that keeps more is the seed
        // (of equals, the game of `n − 1`, the last).
        let seeds = [self.store_game.as_deref(), go.seed.as_deref()].into_iter().flatten();
        let seed = seeds
            .map(|moves| self.board.seed(moves))
            .max_by_key(|seed| search::seed_survivors_sum(&self.prepared, seed));
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
        };
        let found = search::search(&self.prepared, &cfg, seed.as_ref(), go.cap, self.resuming);
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
            time: found.time,
            time_all_runs: found.time_all_runs,
        })
    }
}

type SetUpWithWidth = fn(u32, &Before, &SolveOptions, &Known) -> Result<SetUp, Failure>;

/// The setup of game `n` (its first phase, on the game's thread): the checks of a previous game not made by this run,
/// the tables, and then the game a known source holds, the prime shortcut, the trivial game, or the search problem
/// (steps 1 to 4 of the module documentation, up to the repeat), at the set width of the board size.
fn set_up(n: u32, before: &Before, opts: &SolveOptions, known: &Known) -> Result<SetUp, Failure> {
    let words = words_for(n);
    let beyond = || Failure::Error(format!("n={n} is beyond the largest supported board, n={MAX_N}"));
    let set_up: SetUpWithWidth = crate::with_words!(words, W => set_up_with_width::<W>, else return Err(beyond()));
    set_up(n, before, opts, known).map_err(|failure| at(n, failure))
}

fn set_up_with_width<const W: usize>(
    n: u32,
    before: &Before,
    opts: &SolveOptions,
    known: &Known,
) -> Result<SetUp, Failure> {
    let tactics = opts.tactics;
    if let Before::Given(prev) = before {
        // The prime shortcut and the ceiling trust the previous game, so make sure it is what it says it is.  Not for
        // a game this run made: `conclude` has refereed those moves already, with the same `replay_finished` and to
        // the score the game is given as, before the game became the previous one.
        if prev.source != PrevSource::Run {
            prev.check(n)?;
        }
    }
    let tables = Tables::new(n);
    // A search that was interrupted is resumed, whatever else could have produced the game.
    let checkpoint = opts.checkpoint_file(n).filter(|path| path.exists());
    let derive = !opts.force_search && checkpoint.is_none();

    // A board a known source holds a proven game of (`--use-prev`): the game, with no search.
    if derive && opts.use_prev && opts.known.boards {
        if let Some(game) = known.game(n)? {
            let candidates = BoardFrames::<W>::new(&tables, Bits::EMPTY).promotion_candidates();
            return Ok(SetUp::Derived(Derived::Known { game: Box::new(game), candidates: candidates.to_vec() }));
        }
    }

    if derive && tactics.prime_shortcut && tables.is_prime(n) && before.proven() {
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
            // A checkpoint does not record how the searchers keep their nogoods: this run says.
            p.tactics = p.tactics.with_forgetting_of(&tactics.search);
            if p.tactics != tactics.search {
                return Err(Failure::Error(different_tactics(path, p.tactics, tactics)));
            }
            p
        }
        None => board.prepare(tactics.search)?,
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
    let seeds = tactics.seed_incumbent && !resuming;
    // What the bounds store has of the board, if it is not to be searched on its own: its lower bound's game seeds the
    // search, and its upper bound is a ceiling.  (`force_search` is a search with nothing but the chain's own help.)
    let help = if opts.use_prev && !opts.force_search { known.open(n)? } else { Open::default() };
    let store_cap = help.upper.map(|bound| StoreCap { sum: bound.value.saturating_sub(in_the_bag), bound });
    let store_game = help.game.filter(|_| seeds);
    let job = Arc::new(SearchJob { board, prepared, control: Control::new(), resuming, seeds, store_game });
    Ok(SetUp::Search(SearchSetUp { job, setup, in_the_bag, store_cap, frames, greedy }, problem))
}

/// The ceiling on the promotion sum of game `n`, whose `in_the_bag` is given, from the score of its previous game, if
/// there is one and `prev_cap` is on ([`promotion_cap`]).
fn ceiling(n: u32, in_the_bag: u64, prev_score: Option<u64>, opts: &SolveOptions) -> Option<u64> {
    prev_score.filter(|_| opts.tactics.prev_cap).and_then(|score| promotion_cap(n, in_the_bag, score))
}

/// The ceiling a search of game `n` stops at: the lower of the one its previous game gives ([`ceiling`]) and the bounds
/// store's upper bound of the board (`set.store_cap`), if it has either.
fn search_cap(set: &SearchSetUp, n: u32, prev_score: Option<u64>, opts: &SolveOptions) -> Option<u64> {
    let chain = ceiling(n, set.in_the_bag, prev_score, opts);
    match (chain, set.store_cap.as_ref()) {
        (Some(chain), Some(store)) => Some(chain.min(store.sum)),
        (chain, store) => chain.or(store.map(|store| store.sum)),
    }
}

/// How a game was found, as its conclusion takes it.
enum Found<'a> {
    Derived(&'a Derived),
    /// A repeat of the search of the game `m`, which settled the same problem.
    Repeat(&'a SearchSetUp, u32),
    Searched(&'a SearchSetUp, &'a Outcome),
}

/// The conclusion of game `n` (its last phase): its result, as the game was found, checked by [`conclude`], whose
/// `context` has the previous game.  `earlier` has the game a repeat repeats, and `earlier_runs` is the number of
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
            let prev = context.prev.expect("the prime shortcut is taken only with a previous game");
            let moves = prime_shortcut(n, &prev.moves);
            let played: BTreeSet<u32> = moves.iter().copied().collect();
            let promotions = candidates.iter().copied().filter(|c| played.contains(c)).collect();
            result.method = Method::Prime;
            conclude(result, moves, promotions, context)
        }
        Found::Derived(Derived::Trivial { plain, in_the_bag, frames }) => {
            result.method = Method::Trivial;
            result.in_the_bag = Some(*in_the_bag);
            result.phases.frames = *frames;
            conclude(result, plain.clone(), Vec::new(), context)
        }
        Found::Derived(Derived::Known { game, candidates }) => {
            result.method = Method::Known;
            result.known = Some(Cited { cite: game.cite.clone(), assumes: game.assumes.clone() });
            let played: BTreeSet<u32> = game.moves.iter().copied().collect();
            let promotions = candidates.iter().copied().filter(|c| played.contains(c)).collect();
            // The game must score what the source gives for it, as well as pass the referee.
            conclude(result, game.moves.clone(), promotions, context).and_then(|result| {
                match result.score == game.score {
                    true => Ok(result),
                    false => {
                        let (score, source) = (game.score, game.origin.whence());
                        let reason =
                            format!("the game replays to {}, not the {score} that {source} gives it", result.score);
                        reject(result, reason)
                    }
                }
            })
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
            let cap = ceiling(n, set.in_the_bag, context.prev.and_then(Prev::cap_score), opts);
            let setup = SearchSetup { cap, ..set.setup };
            result.setup = Some(setup);
            let sum: u64 = searched.promotions.iter().map(|&p| u64::from(p)).sum();
            // A search ends when its incumbent reaches its ceiling, which proves it optimal.  One that was started
            // before the ceiling was known (the game of `n − 1` still in flight) may have gone on to the end of its
            // tree, or been stopped by a limit or a signal, with its best set at the ceiling: it is proven by the
            // ceiling all the same, as a search given the ceiling from the start would have stopped there.
            result.reached_cap = setup.cap == Some(sum);
            // So is one that stopped at the bounds store's upper bound, when the ceiling is not what it reached: the
            // store's bound is what the line cites.
            if let Some(store) = set.store_cap.as_ref().filter(|store| !result.reached_cap && store.sum == sum) {
                let upper = store.bound.line.clone();
                let cite = Cite::Bounds { lower: None, upper };
                result.known = Some(Cited { cite, assumes: store.bound.assumes.clone() });
            }
            result.completed = searched.completed || result.reached_cap || result.known.is_some();
            result.nodes = searched.stats.nodes;
            result.stats = searched.stats;
            result.counters = searched.counters;
            result.phases.search = searched.time;
            result.phases.search_all_runs = searched.time_all_runs;
            if set.job.resuming() {
                // At least one invocation made the checkpoint, even if it left no line in a results file.
                result.invocations = earlier_runs.max(1) + 1;
            }
            let concluded = conclude(result, searched.moves.clone(), searched.promotions.clone(), context);
            // A game above the store's upper bound proves the store wrong, or statement U false at a board it rests on.
            match set.store_cap.as_ref().filter(|store| sum > store.sum) {
                Some(store) => concluded.and_then(|result| {
                    let (bound, bag) = (store.bound.value, set.in_the_bag);
                    let reason = format!(
                        "the game scores {}, more than the {bound} that the bounds store's upper bound allows \
                         (inTheBag {bag} + {}): the store is wrong, or statement U fails at a board it rests on",
                        result.score, store.sum
                    );
                    reject(result, reason)
                }),
                None => concluded,
            }
        }
    };
    concluded.map_err(|failure| at(n, failure))
}

/// What [`conclude`] checks a game against.
#[derive(Clone, Copy)]
struct Context<'a> {
    prev: Option<&'a Prev>,
    optimal: &'a Optimal,
    /// The game's own time before its conclusion: its setup, its repeat lookup and its search, not the time it
    /// waited in the window.
    own: Duration,
    /// When its conclusion began.
    since: Instant,
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
    result.phases.total = context.own + context.since.elapsed();
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
        // Only an optimal game of n − 1 bounds the game of n; the best game of a stopped search proves nothing.
        if prev.proven && score > ceiling {
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
    // A game that cites a known source (taken from it, or found by a search that its upper bound stopped) is proven by
    // it, and assumes what the cited lines make it assume.
    if let Some(cited) = &result.known {
        result.proof = Some(Proof::Known);
        result.assumes = cited.assumes.clone();
        return Ok(result);
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
/// results file (or `taxman certify`'s file of certificates), or (without one) handed to the caller.
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

/// How far back a game looks for an earlier game whose search problem it repeats (tactic `same_search`): the board
/// sizes `n − SAME_SEARCH_WINDOW` to `n − 1`.  A board's frames change a little with each number: each of the 1650
/// games of the committed chain 1..3914 whose problem an earlier game posed repeats the last board size before it that
/// set a search up, `n − 1`, or `n − 2` when `n − 1` is prime (docs/experiments/same-search-2026-10.md).  The window
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
/// file and of this run that set a search up and whose proof settles it ([`Proof::settles_search`]), and the games
/// of the range in flight that set a search up and are not concluded yet (`pending`, see [`find`](Self::find)).
#[derive(Default)]
struct Earlier {
    games: BTreeMap<u32, Searched>,
    /// The search problems of the boards this run set up ([`remember`](Self::remember)), by board size, for the last
    /// [`SAME_SEARCH_WINDOW`] board sizes: what a comparison with an earlier game needs, so that it need not set
    /// that game's board up again.
    setups: BTreeMap<u32, Problem>,
    /// The boards this run set up whose games are not concluded yet.
    pending: BTreeSet<u32>,
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

    /// Adds a game solved in this run, when it is concluded.
    fn add_result(&mut self, result: &GameResult) {
        self.pending.remove(&result.n);
        if let (true, Some(proof), Some(setup)) = (result.completed, result.proof, result.setup) {
            if proof.settles_search() {
                let (promotions, assumes) = (result.promotions.clone(), result.assumes.clone());
                let (size, digest) = (setup.size(), setup.digest);
                self.games.insert(result.n, Searched { size, digest, promotions, proof, assumes });
            }
        }
    }

    /// Remembers the search problem that this run set board `n` up with (at the repeat lookup of its game, in the
    /// order of the board sizes), with the game pending until it is concluded, and forgets the problems of the boards
    /// more than [`SAME_SEARCH_WINDOW`] sizes before the latest one remembered: no game that follows looks that far
    /// back.
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
    /// is, its problem (the one this run set its board up with when it did, else set up again from the board size,
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

    /// The record that a repeat of the search of game `m` keeps, whose problem has the digest given, and `m`'s
    /// promotions, once `m` is among the games ([`find`](Self::find) found it).
    fn same_search(&self, m: u32, digest: u64) -> Option<(SameSearch, Vec<u32>)> {
        let game = self.games.get(&m)?;
        let promotion_sum = game.promotions.iter().map(|&p| u64::from(p)).sum();
        let same = SameSearch { n: m, digest, promotion_sum, proof: game.proof, assumes: game.assumes.clone() };
        Some((same, game.promotions.clone()))
    }

    /// The promotions of game `m`, if it is among the games.
    fn promotions(&self, m: u32) -> Option<&[u32]> {
        self.games.get(&m).map(|game| game.promotions.as_slice())
    }
}

/// The search problem of board `m`: the one in `setups` (the boards this run set up), else the one set up again from
/// the board size (a game of the results file), if the board can be set up.  The two are the same problem: the one a
/// run's own setup of the board posed is what `search_problem(m)` builds, as [`Earlier::remember`] says.
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
    /// With [`SolveOptions::keep_going`], the games of the results file that only an unfinished search is recorded
    /// for, which the range passed over (not counted in `skipped`).
    pub passed_over: usize,
    /// With [`SolveOptions::keep_going`], the games solved in this run whose search a limit stopped (counted in
    /// `solved`): unfinished lines, with their checkpoints kept.
    pub unfinished: usize,
    /// The game whose search stopped at a limit, or at a signal; the range ends there.  (With
    /// [`SolveOptions::keep_going`] a limit does not end the range, only a signal does, and nothing is recorded for
    /// that game.)
    pub stopped_at: Option<u32>,
    /// The game that passed the wall ([`SolveOptions::game_limit`]); the range ends there, with nothing recorded for it.
    pub walled: Option<Wall>,
}

/// The game that passed the wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wall {
    pub n: u32,
    /// What its clock said when the range stopped: its own search so far, over every invocation.
    pub held: Duration,
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
        proven: true,
    })
}

/// The best game of the last unfinished search of `n` in the results file (a board size it has no completed game of),
/// as the previous game of `n + 1`: a seed, not a proven game.
fn stored_unfinished(results: &ResultsFile, n: u32) -> Option<Prev> {
    let game = results.unfinished(n)?;
    Some(Prev {
        moves: game.moves.clone(),
        score: game.score,
        source: PrevSource::Results,
        proof: None,
        assumes: Assumptions::new(),
        proven: false,
    })
}

/// The solution of game `n − 1` from the results file, else (when the range keeps going past limits) the best game of
/// an unfinished search of it, as a seed, else, with `use_prev`, the game the known sources hold ([`Known::game`]).
fn previous_game(
    n: u32,
    results: Option<&ResultsFile>,
    known: &Known,
    opts: &SolveOptions,
) -> Result<Option<Prev>, String> {
    let Some(before) = n.checked_sub(1).filter(|&before| before >= 1) else { return Ok(None) };
    let unfinished = |r: &ResultsFile| opts.keep_going.then(|| stored_unfinished(r, before)).flatten();
    let in_results = results.and_then(|r| stored_prev(r, before).or_else(|| unfinished(r)));
    if in_results.is_some() || !opts.use_prev {
        return Ok(in_results);
    }
    Ok(known.game(before)?.map(|game| Prev::from_known(&game)))
}

/// Says on stderr that game `n` has no previous game, so that it is solved standalone: correct, but without the
/// seed and the ceiling that the game of `n − 1` gives, so it costs more (`bench/curve/README.md` measures how
/// much).  `in_table`: whether `optimal.json` has the game of `n − 1` (which `--use-prev` would take).
fn note_standalone(n: u32, in_table: bool) {
    let before = n - 1;
    let how = if in_table {
        format!("give --out with the earlier games, or --use-prev to take the known game of {before}")
    } else {
        "give --out with the earlier games, or --use-prev if the committed chain or the bounds store has it".to_owned()
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

/// Solves the board sizes `lo..=hi`, each from the solution of the one before, and concludes them in order (with
/// up to [`SolveOptions::window`] games in flight, see `window`).  The solution of `lo − 1` comes from the results
/// file, else, with [`SolveOptions::use_prev`], from `optimal.json`.  The results file is checked when it is opened
/// ([`ResultsFile::open`]); games already completed in it are skipped (and a checkpoint left for one is deleted).
/// Every other result is appended to the file (if there is one), then given to `on_game`, and only then is its
/// checkpoint deleted.  A game that fails a check after it was found goes to the file's
/// [`rejected_path`](crate::results::rejected_path) instead, and the run fails.  The range stops at a game whose
/// search did not complete.  Scores are confirmed against `optimal.json` ([`Optimal::built_in`]).
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
    let known = Known::load(optimal, &opts.known)?;
    let open = |path: &Path| {
        ResultsFile::open_with(path, &known, opts.threads).map_err(|e| match e.contains("not given to the check") {
            // A file made with `--use-prev` has lines that cite the sources it drew on.
            true => format!("{e} (a file with lines that cite a known source is opened with --use-prev)"),
            false => e,
        })
    };
    let mut results = results_path.map(open).transpose()?;
    if let Some(file) = &results {
        // Left by a crash between saving a result and deleting its checkpoint.
        for n in (lo..=hi).filter(|&n| file.completed(n).is_some()) {
            if [opts.checkpoint_file(n), opts.clock_file(n)].iter().flatten().any(|path| path.exists()) {
                discard_checkpoint(opts, n);
            }
        }
    }
    let first = previous_game(lo, results.as_ref(), &known, opts)?;
    // A first game the file settles, or a retry does not search, is not solved standalone by this run.
    let passes_unfinished = opts.keep_going && !opts.retry;
    let settled =
        |file: &ResultsFile| file.completed(lo).is_some() || (passes_unfinished && file.unfinished(lo).is_some());
    if lo > 1 && first.is_none() && !results.as_ref().is_some_and(settled) {
        note_standalone(lo, optimal.score(lo - 1).is_some());
    }
    let mut earlier = results.as_ref().map(Earlier::of_file).unwrap_or_default();
    let before = first.map_or(Before::None, Before::Given);
    let range = window::Range { lo, hi, before, known: &known, results: results.as_mut() };
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
        let options = json.get("options").unwrap();
        assert_eq!(
            (options.get("force_search"), options.get("use_prev")),
            (Some(&Json::Bool(false)), Some(&Json::Bool(false)))
        );
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
        assert!(ResultsFile::open(&path, Optimal::built_in(), 1).is_err());
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
        let prev = |moves: &[u32], score, source| Prev {
            moves: moves.to_vec(),
            score,
            source,
            proof: Some(Proof::Exhausted),
            assumes: Assumptions::new(),
            proven: true,
        };
        // [5, 4, 6] is the game for 6, scoring 15.
        for source in [PrevSource::Results, PrevSource::Optimal] {
            assert!(solve_game(7, Some(&prev(&[5, 4, 6], 15, source)), &opts).is_ok());
            let err = solve_game(7, Some(&prev(&[5, 4, 6], 14, source)), &opts).unwrap_err();
            assert!(err.contains("scores 15, not the 14"), "{err}");
            let err = solve_game(7, Some(&prev(&[5, 4, 4], 13, source)), &opts).unwrap_err();
            assert!(err.contains("not a legal game of n=6"), "{err}");
        }
        // A game this run made was refereed by `conclude` before it became the previous game, so it is not checked
        // again: a wrong score goes unnoticed (the game of 7 built from it is still refereed), and moves that are
        // not a game fail only in the game built from them.
        assert!(solve_game(7, Some(&prev(&[5, 4, 6], 14, PrevSource::Run)), &opts).is_ok());
        let err = solve_game(7, Some(&prev(&[5, 4, 4], 13, PrevSource::Run)), &opts).unwrap_err();
        assert!(err.contains("does not stand up") && !err.contains("not a legal game of n=6"), "{err}");
    }

    /// Solves the board sizes `lo..=hi` as a range does (its driver, [`window::solve`], on one thread, so one game
    /// after the other), each from the one before (the first from `optimal.json`'s game before it), with the
    /// `Earlier` that the run keeps; returns it and the results.
    fn solved_with_earlier(lo: u32, hi: u32) -> (Earlier, Vec<GameResult>) {
        let (opts, optimal) = (one_thread(), Optimal::built_in());
        let mut earlier = Earlier::default();
        let before = Prev::from_optimal(optimal, lo - 1).map_or(Before::None, Before::Given);
        let known = Known::table(optimal);
        let range = window::Range { lo, hi, before, known: &known, results: None };
        let mut results = Vec::new();
        window::solve(range, &opts, &mut earlier, &mut |result| results.push(result.clone())).unwrap();
        (earlier, results)
    }

    /// The boards 300 and 301 pose one search problem (as the committed chain, `results/strict.jsonl`, has it): 301
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
        let mut other = Earlier::default();
        other.remember(300, search_problem(310).unwrap().0);
        assert_eq!(other.find(301, &ours), None);
    }

    #[test]
    fn only_the_problems_within_the_window_are_remembered() {
        let problem = search_problem(85).unwrap().0;
        let mut earlier = Earlier::default();
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
