//! The unsound mode, `taxman solve LO-HI --unsound --effort E --out FILE`: each game played as well as a level of
//! effort finds, with no proof that its score is opt(n).
//!
//! The proof of a hard game past 5060 takes hours, and nearly all of it goes to proving a set that the search found
//! early.  The unsound mode keeps the finding and drops the proof.  Its line holds legal moves and their score, a lower
//! bound on opt(n), and is marked unsound ([`UNSOUND`]), so that no reader of proven lines takes one.  The user names a
//! level of effort: more effort costs more time and gives a better chance that each score is opt(n).
//!
//! # One game
//!
//! * The setup is the proven mode's ([`set_up`]): the board and its frames.  A board with no promotion candidates has
//!   the moves with no promotions (method `trivial`).
//! * A prime n plays the prime shortcut on the moves for n − 1 (method `prime`).  A game whose search problem a line of
//!   the run or of the file posed, within [`SAME_SEARCH_WINDOW`](super::SAME_SEARCH_WINDOW) games before it, plays that
//!   line's promotions on its own board (method `repeat`, the proven mode's lookup).  Neither searches, and both copy
//!   what the line they start from falls short by, if it does.
//! * Any other game (method `search`) makes the start that a proven search makes
//!   ([`start_incumbent`](crate::search::start_incumbent)), seeded with the moves for n − 1, and then the rounds of its
//!   level ([`Plan::budget`]) of LNS on every thread, whose answer is the same on any number of threads
//!   ([`Lns::rounds`](crate::search::finder::Lns::rounds)).
//! * There is no ceiling.  The proven mode's, n + score(n − 1) − in_the_bag, holds only when the moves for n − 1 are
//!   optimal, which nothing here says, so nothing stops at it, is held below it or is skipped by it.  A score above
//!   n + score(n − 1) proves the line of n − 1 short: the line records by how much at least (`prev.short_by`), and the
//!   run goes on.
//! * The moves are replayed before they count ([`replay`]), and a score above the built-in table's, which is proven, is
//!   a hard error.
//!
//! # The level of effort
//!
//! Effort 0 is the start alone.  Effort E ≥ 1 runs rounds of [`NEIGHBORHOODS`] neighborhoods until the work of its
//! rounds reaches 2^(E−1) × [`ROUND_WORK`] ([`Plan::budget`]); the budget is checked after each round, and the round
//! that reaches it is the last.  The work of a round is its engine's steps and [`finder::NEIGHBORHOOD_WORK`] for each
//! of its neighborhoods, which have a cost beside their steps, so that the time a budget takes is much the same
//! whatever the options.  The steps of a neighborhood are a function of its unit, never of the threads, so the rounds
//! do not depend on the level: a run at E + 1 does the rounds of E first, and then more.  So the `improvements` of a
//! line at one level show what every lower level would have scored, and each records the `steps` and the `work` of the
//! rounds up to its own.
//!
//! # The shape of the rounds
//!
//! Four options of the plan change what the rounds search, and so the line; none changes the default.  Every value of
//! them, alone or together, keeps what the rounds guarantee: a line depends on the game, its start, the moves it starts
//! from, the level, the seed and the options only, never on the threads, and a run at E + 1 does the rounds of E first.
//!
//! * **`fix`** is the range from which each neighborhood draws the share of the incumbent's promotions it keeps fixed
//!   ([`Options::fix`](crate::search::finder::Options::fix)).  A smaller share frees more of the game to a
//!   neighborhood, which then takes more of its steps.
//! * **`tilt`** chooses the promotions a neighborhood frees by their size, the small ones more often, with the same
//!   number freed ([`Options::tilt`](crate::search::finder::Options::tilt)).
//! * **`back`** J gives a game a trajectory of incumbents for each distinct set that survives of the moves for the
//!   games n − 1, n − 2, ..., n − J ([`seed_survivors`](crate::search::seed_survivors)), as the finder beside a proof
//!   search starts its seed rounds, beside the first, the start's.  A set that an earlier trajectory (the start's
//!   included) began from is skipped, and so is a game whose moves nothing has.  Neighborhood k of a round works from
//!   the incumbent of trajectory k mod T, T the trajectories, and the game's set is the heaviest incumbent, the
//!   start's on a tie ([`Lns::rounds`](crate::search::finder::Lns::rounds)).  The line says which games the
//!   trajectories began from (`trajectories`).  What survives of an earlier game may be heavier than the start; the
//!   game's score is then that of the survivors from the first, which the line records as an improvement of round 0,
//!   and at effort 0 the score is the heaviest of the trajectories' starts.
//! * **`widen`** S moves the neighborhoods of a trajectory whose incumbent has not grown heavier in S rounds down the
//!   ladder of the share kept fixed ([`LADDER`](crate::search::finder::LADDER)), and back to its first stage when it
//!   grows ([`Options::widen`](crate::search::finder::Options::widen)).  The line records the rounds each trajectory
//!   spent at each stage (`stage_rounds`) and the stage of each improvement.  The ladder replaces `fix`, so the two are
//!   refused together.
//!
//! # A range
//!
//! [`solve_range`] plays the games lo..=hi in order, one at a time, on all the threads given.  The moves for n − j that
//! a game starts from, for j = 1 and the j up to `back`, are this run's line of n − j, else the file's, else the
//! built-in table's (as a seed only, never a line).  Each line is appended to the file and synced as soon as its game
//! ends, and a restarted run skips the games the file has.  A stop signal (SIGINT, SIGTERM) ends the run between
//! rounds, and the game in play is not recorded.  The file holds unsound lines only: [`UnsoundFile::open`] refuses a
//! line of a proven run, as every reader of proven lines refuses an unsound one
//! ([`refuse_unsound`](crate::game::refuse_unsound)).
//!
//! # Measuring
//!
//! Two options measure the mode on the games whose opt(n) is proven, and are refused for a game the built-in table
//! lacks.  `--stop-at-known` ends a game's rounds as soon as its score is the table's, and the line records the round
//! at which it was, or that it never was.  `--from-known` seeds every game with the table's moves for n − 1, not with
//! this run's line.  solver/docs/REFERENCE.md has the keys of the line ("An unsound line").

use super::{
    prime_game, set_up, Before, Derived, Earlier, Failure, Method, RangeSummary, SearchSetup, SetUp, SolveOptions,
};
use crate::bits::Bits;
use crate::engine;
use crate::game::{replay, Optimal, UNSOUND};
use crate::json::Json;
use crate::results::{append_line, build_json, each_line, mend, moves_digest, open_locked, Fields};
use crate::search::finder::{self, Improvement, NEIGHBORHOODS, STAGES};
use crate::search::ProblemSize;
use crate::tactics::Tactics;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// The most effort a run may ask for: a budget of 2^31 rounds of work, far more than a game needs, and 2^47 work,
/// which a 64-bit count holds.
pub const MAX_EFFORT: u32 = 32;

/// The games before a game that its trajectories may begin from: `back` goes up to this.
pub const MAX_BACK: u32 = 8;

/// The budget of effort 1 ([`Plan::budget`]), in work: the steps of one round whose neighborhoods all take the step
/// limit of a unit, [`finder::UNIT_STEPS`].  (The work of such a round is a little more, [`finder::NEIGHBORHOOD_WORK`]
/// for each neighborhood.)
pub const ROUND_WORK: u64 = NEIGHBORHOODS * finder::UNIT_STEPS;

/// How the unsound mode plays: the level of effort, the seed of the rounds' random choices, the shape of the
/// neighborhoods and of the rounds, and the measuring options.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    /// From 0 to [`MAX_EFFORT`].
    pub effort: u32,
    pub seed: u64,
    /// `--fix LO,HI`: the share of the incumbent's promotions that each neighborhood keeps fixed is drawn from this
    /// range, 0 ≤ LO ≤ HI ≤ 1 ([`check_fix`]).
    pub fix: (f64, f64),
    /// `--tilt`: a neighborhood frees the small promotions more often than the large ones.
    pub tilt: bool,
    /// `--back J`, J from 0 to [`MAX_BACK`]: a game follows a trajectory from each distinct set that survives of the
    /// moves for the games n − 1, ..., n − J, beside the start's.
    pub back: u32,
    /// `--widen S`, S ≥ 1: a trajectory that has not grown heavier in S rounds moves its neighborhoods down the ladder
    /// of the share fixed.
    pub widen: Option<u64>,
    /// The work that effort 1 budgets, [`ROUND_WORK`] unless a test or a measurement lowers it: no flag sets it, and a
    /// line records the budget it gave (`budget`).
    pub round_work: u64,
    /// `--stop-at-known`: a game's rounds end once its score is the built-in table's.
    pub stop_at_known: bool,
    /// `--from-known`: every game starts from the built-in table's moves for n − 1.
    pub from_known: bool,
}

/// Effort 0, the finder's seed and share fixed, the start's trajectory alone, and no measuring option.
impl Default for Plan {
    fn default() -> Plan {
        let finder::Options { fix, seed, .. } = finder::Options::default();
        Plan {
            effort: 0,
            seed,
            fix,
            tilt: false,
            back: 0,
            widen: None,
            round_work: ROUND_WORK,
            stop_at_known: false,
            from_known: false,
        }
    }
}

impl Plan {
    /// The work the rounds of the level run until: none at effort 0, 2^(E−1) × `round_work` at effort E (at most
    /// 2^31 × [`ROUND_WORK`] = 2^47 with the default, which a count of work holds; a larger `round_work` stops at the
    /// largest count).
    pub fn budget(&self) -> u64 {
        self.effort.checked_sub(1).map_or(0, |e| self.round_work.saturating_mul(1u64 << e))
    }

    /// How the rounds search: the finder's defaults, with this plan's seed, share fixed, tilt and widening.
    pub fn lns(&self) -> finder::Options {
        finder::Options {
            seed: self.seed,
            fix: self.fix,
            tilt: self.tilt,
            widen: self.widen,
            ..finder::Options::default()
        }
    }
}

/// Whether `fix` is a range of shares: 0 ≤ LO ≤ HI ≤ 1 (which no NaN is).
fn is_range((lo, hi): (f64, f64)) -> bool {
    (0.0..=1.0).contains(&lo) && (lo..=1.0).contains(&hi)
}

/// Reads the value of `--fix`, `LO,HI`: two numbers, 0 ≤ LO ≤ HI ≤ 1.
///
/// # Errors
///
/// What `text` should have been.
pub fn parse_fix(text: &str) -> Result<(f64, f64), String> {
    let numbers = text.split_once(',').and_then(|(lo, hi)| Some((lo.parse().ok()?, hi.parse().ok()?)));
    numbers.filter(|&fix| is_range(fix)).ok_or_else(|| fix_error(text))
}

/// Checks that `fix` is a range of shares: 0 ≤ LO ≤ HI ≤ 1.
///
/// # Errors
///
/// What `fix` should have been.
pub fn check_fix(fix: (f64, f64)) -> Result<(), String> {
    if is_range(fix) {
        Ok(())
    } else {
        Err(fix_error(&format!("{},{}", fix.0, fix.1)))
    }
}

/// The message of a `--fix` that is not `LO,HI` with 0 ≤ LO ≤ HI ≤ 1, quoting what it was given.
fn fix_error(got: &str) -> String {
    format!("--fix takes LO,HI, two numbers with 0 <= LO <= HI <= 1, such as 0.7,0.9: got {got:?}")
}

/// Reads the value of `--back`, J: a whole number from 0 to [`MAX_BACK`].
///
/// # Errors
///
/// What `text` should have been.
pub fn parse_back(text: &str) -> Result<u32, String> {
    text.parse().ok().filter(|&back| back <= MAX_BACK).ok_or_else(|| back_error(text))
}

/// The message of a `--back` that is not a whole number from 0 to [`MAX_BACK`], quoting what it was given.
fn back_error(got: &str) -> String {
    format!("--back takes J, a whole number from 0 to {MAX_BACK}: got {got:?}")
}

/// Reads the value of `--widen`, S: a whole number of rounds, at least 1.
///
/// # Errors
///
/// What `text` should have been.
pub fn parse_widen(text: &str) -> Result<u64, String> {
    text.parse().ok().filter(|&widen| widen >= 1).ok_or_else(|| widen_error(text))
}

/// The message of a `--widen` that is not a whole number of rounds from 1, quoting what it was given.
fn widen_error(got: &str) -> String {
    format!("--widen takes S, a whole number of rounds, at least 1: got {got:?}")
}

/// The message of a plan that widens the neighborhoods and also gives the share fixed: the ladder is the share fixed.
pub const WIDEN_WITH_FIX: &str = "--widen takes the share fixed from its own ladder, so --fix is refused with it";

/// Checks the plan's options: an effort of at most [`MAX_EFFORT`], `fix` ([`check_fix`]), `back` of at most
/// [`MAX_BACK`], `widen` of at least 1 and not with a `fix` other than the default, and a step budget of effort 1 above
/// 0.
///
/// # Errors
///
/// The first option that is out of range.
pub fn check_plan(plan: &Plan) -> Result<(), String> {
    if plan.effort > MAX_EFFORT {
        return Err(format!("the effort {} is more than the most, {MAX_EFFORT}", plan.effort));
    }
    check_fix(plan.fix)?;
    if plan.back > MAX_BACK {
        return Err(back_error(&plan.back.to_string()));
    }
    if let Some(widen) = plan.widen {
        if widen < 1 {
            return Err(widen_error(&widen.to_string()));
        }
        if plan.fix != Plan::default().fix {
            return Err(WIDEN_WITH_FIX.to_owned());
        }
    }
    if plan.round_work == 0 {
        return Err("the work that effort 1 budgets is 0".to_owned());
    }
    Ok(())
}

/// Where the moves for n − 1 that a game started from came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// This run's line of n − 1.
    Run,
    /// The file's line of n − 1.
    Results,
    /// The built-in table's moves for n − 1, which are proven.
    Table,
}

impl Source {
    /// The name in a line.
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Run => "run",
            Source::Results => "results",
            Source::Table => "table",
        }
    }
}

/// The moves for an earlier game that a game starts from, or that one of its trajectories begins from.
#[derive(Clone, Debug)]
struct Moves {
    /// The game they are for: n − j.
    n: u32,
    moves: Vec<u32>,
    score: u64,
    source: Source,
}

/// What a trajectory of a game's rounds began from: the start, or what survives of the moves for an earlier game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Start,
    /// The moves for this game (n − j).
    Game(u32),
}

impl Origin {
    /// The value in a line: `"start"`, or the game's number.
    fn to_json(self) -> Json {
        match self {
            Origin::Start => Json::from("start"),
            Origin::Game(n) => Json::from(n),
        }
    }
}

/// The starts of a game's trajectories, and where each began from: `start`, then each of `survivors` (the game, and
/// the set that survives of its moves, in the order of the games n − 1, n − 2, ...) that is not a set an earlier
/// trajectory began from.
pub(super) fn trajectories<const W: usize>(
    start: Bits<W>,
    survivors: impl IntoIterator<Item = (u32, Bits<W>)>,
) -> (Vec<Bits<W>>, Vec<Origin>) {
    let (mut starts, mut origins) = (vec![start], vec![Origin::Start]);
    for (game, set) in survivors {
        if !starts.contains(&set) {
            starts.push(set);
            origins.push(Origin::Game(game));
        }
    }
    (starts, origins)
}

/// What a line records of the moves for n − 1 it started from (`prev`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Previous {
    pub n: u32,
    pub score: u64,
    pub source: Source,
    /// [`moves_digest`] of the moves.
    pub moves_digest: String,
    /// How many points the moves for n − 1 fall short of opt(n − 1) at least, as this game's score proves: its score
    /// less n + score(n − 1), where that is above 0 (no moves for n score more than n + opt(n − 1)); else 0.
    pub short_by: u64,
}

/// What `--stop-at-known` recorded of a game: the built-in table's score, and the round after which the game's score
/// was that (0 for the start or the start of a trajectory, or for a game with no rounds to run), or `None` if it never
/// was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Known {
    pub score: u64,
    pub round: Option<u64>,
}

/// A round whose better set raised the game's score: the round (0 is a trajectory that begins heavier than the start,
/// before any round), the trajectory that made it (0 is the start's) and the stage of the ladder its neighborhoods
/// worked at in the round, the steps and the work of the rounds up to and including this one, the time on the game's
/// clock when it ended, and the new score.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Raised {
    pub round: u64,
    pub trajectory: usize,
    pub stage: usize,
    pub steps: u64,
    pub work: u64,
    pub seconds: f64,
    pub score: u64,
}

/// An unsound line: a game as the unsound mode played it (the module documentation; solver/docs/REFERENCE.md, "An
/// unsound line", has its keys).
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub n: u32,
    pub score: u64,
    pub moves: Vec<u32>,
    /// The numbers promoted, ascending.
    pub promotions: Vec<u32>,
    /// `trivial`, `prime`, `repeat` or `search`.
    pub method: Method,
    /// What the moves with no promotions score; `None` for a prime game, which sets no board up.
    pub in_the_bag: Option<u64>,
    pub prev: Option<Previous>,
    /// For a repeat, the n of the line whose promotions it plays.
    pub repeats: Option<u32>,
    /// What a searched or repeat game was set up with (its `cap` is always `None`: there is no ceiling).
    pub setup: Option<SearchSetup>,
    pub plan: Plan,
    /// The score of a searched game's start.
    pub start: Option<u64>,
    /// What each trajectory of a searched game's rounds began from: the start, then the games `--back` took distinct
    /// sets from.
    pub trajectories: Option<Vec<Origin>>,
    /// The rounds done, the engine's steps in them, and their work ([`finder::NEIGHBORHOOD_WORK`] a neighborhood
    /// besides the steps).
    pub rounds: u64,
    pub steps: u64,
    pub work: u64,
    /// For each trajectory, the rounds it worked at each stage of the ladder of `--widen` (all at the first without).
    pub stage_rounds: Option<Vec<[u64; STAGES]>>,
    /// Each round that raised the score, in order.
    pub improvements: Vec<Raised>,
    /// The game's wall time, from the start of its setup to its line, and the part of it before the rounds.
    pub seconds: f64,
    pub start_seconds: f64,
    pub threads: usize,
    /// What `--stop-at-known` recorded, if it was given.
    pub known: Option<Known>,
    pub tactics: Tactics,
}

impl Line {
    /// The line as one JSON object.
    pub fn to_json(&self) -> Json {
        let prev = self.prev.as_ref().map_or(Json::Null, |p| {
            Json::object([
                ("n", Json::from(p.n)),
                ("score", Json::from(p.score)),
                ("source", Json::from(p.source.as_str())),
                ("moves_digest", Json::from(p.moves_digest.as_str())),
                ("short_by", Json::from(p.short_by)),
            ])
        });
        let setup = self.setup.map_or(Json::Null, |s| {
            Json::object([
                ("frames", Json::from(s.frames)),
                ("candidates", Json::from(s.candidates)),
                ("max_total", Json::from(s.max_total)),
                ("greedy_sum", Json::from(s.greedy_sum)),
                ("digest", Json::from(super::digest_text(s.digest))),
            ])
        });
        let improvements = self.improvements.iter().map(|raised| {
            Json::object([
                ("round", Json::from(raised.round)),
                ("trajectory", Json::from(raised.trajectory)),
                ("stage", Json::from(raised.stage)),
                ("neighborhoods", Json::from(raised.round * NEIGHBORHOODS)),
                ("steps", Json::from(raised.steps)),
                ("work", Json::from(raised.work)),
                ("seconds", Json::Num(raised.seconds)),
                ("score", Json::from(raised.score)),
            ])
        });
        let trajectories =
            self.trajectories.as_ref().map_or(Json::Null, |t| Json::array(t.iter().map(|o| o.to_json())));
        let stage_rounds = self
            .stage_rounds
            .as_ref()
            .map_or(Json::Null, |t| Json::array(t.iter().map(|stages| Json::array(stages.iter().copied()))));
        let known = self.known.map_or(Json::Null, |known| {
            Json::object([("known", Json::from(known.score)), ("round", known.round.map_or(Json::Null, Json::from))])
        });
        Json::object([
            (UNSOUND, Json::Bool(true)),
            ("n", Json::from(self.n)),
            ("score", Json::from(self.score)),
            ("moves", Json::array(self.moves.iter().copied())),
            ("promotions", Json::array(self.promotions.iter().copied())),
            ("method", Json::from(self.method.as_str())),
            ("in_the_bag", self.in_the_bag.map_or(Json::Null, Json::from)),
            ("prev", prev),
            ("repeats", self.repeats.map_or(Json::Null, Json::from)),
            ("setup", setup),
            ("effort", Json::from(self.plan.effort)),
            ("budget", Json::from(self.plan.budget())),
            ("seed", Json::from(self.plan.seed)),
            ("fix", Json::array([self.plan.fix.0, self.plan.fix.1])),
            ("tilt", Json::Bool(self.plan.tilt)),
            ("back", Json::from(self.plan.back)),
            ("widen", self.plan.widen.map_or(Json::Null, Json::from)),
            ("start", self.start.map_or(Json::Null, Json::from)),
            ("trajectories", trajectories),
            ("rounds", Json::from(self.rounds)),
            ("neighborhoods", Json::from(self.rounds * NEIGHBORHOODS)),
            ("steps", Json::from(self.steps)),
            ("work", Json::from(self.work)),
            ("stage_rounds", stage_rounds),
            ("improvements", Json::array(improvements)),
            ("seconds", Json::Num(self.seconds)),
            ("start_seconds", Json::Num(self.start_seconds)),
            ("threads", Json::from(self.threads)),
            ("from_known", Json::Bool(self.plan.from_known)),
            ("stop_at_known", known),
            ("tactics", Json::from(self.tactics.description())),
            ("build", build_json()),
        ])
    }

    /// One line for a person: n, score, method, the rounds and how many raised the score, the time, `unsound`, and what
    /// `--stop-at-known` recorded and a line of n − 1 that this one proves short, if anything.
    pub fn summary_line(&self) -> String {
        let known = match self.known {
            None => String::new(),
            Some(Known { round: Some(round), .. }) => format!("  known reached in round {round}"),
            Some(Known { round: None, .. }) => "  known not reached".to_owned(),
        };
        let short = self
            .prev
            .as_ref()
            .filter(|prev| prev.short_by > 0)
            .map_or_else(String::new, |prev| format!("  n-1 short by {} at least", prev.short_by));
        format!(
            "n={:<5} score={:<9} {:<7} rounds={:<6} improved={:<3} {:>9.3} s  unsound{known}{short}",
            self.n,
            self.score,
            self.method.as_str(),
            self.rounds,
            self.improvements.len(),
            self.seconds
        )
    }
}

/// What the unsound mode's search of a game came to ([`Job::improve`](super::Job::improve)): the start's promotion sum,
/// when it was known on the game's clock, and the rounds after it, of one trajectory or more.
pub(super) struct Improved {
    pub(super) start: u64,
    pub(super) start_seconds: f64,
    pub(super) trajectories: Vec<Origin>,
    /// The best promotions found, ascending, and the moves they make.
    pub(super) promotions: Vec<u32>,
    pub(super) moves: Vec<u32>,
    pub(super) rounds: u64,
    pub(super) nodes: u64,
    pub(super) work: u64,
    pub(super) stage_rounds: Vec<[u64; STAGES]>,
    pub(super) improvements: Vec<Improvement>,
    /// A stop signal ended the rounds: the game is not recorded.
    pub(super) interrupted: bool,
}

// ---------------------------------------------------------------------------------------------------------
// The file

/// A line of an unsound file as the run needs it: the moves for the next game, and a repeat's.
#[derive(Clone, Debug)]
struct Stored {
    score: u64,
    moves: Vec<u32>,
    promotions: Vec<u32>,
    /// The size and the digest of the search problem of a searched or repeat game.
    problem: Option<(ProblemSize, u64)>,
}

/// The file of unsound lines that a run appends to: one JSON object a line, an unsound line each.  Opening it locks it
/// against other processes and checks it; each line is appended and synced as its game ends.  It has the torn last line
/// of a results file, and is mended as one is.
pub struct UnsoundFile {
    path: PathBuf,
    /// Locked (`flock`) for as long as it is open.
    file: File,
    games: BTreeMap<u32, Stored>,
}

impl UnsoundFile {
    /// Opens (creating if need be) the file at `path`, locks it, and checks every line: an unsound line (a line of a
    /// proven run is refused, as is a line that is not a game), whose moves replay to its score, whose promotions are
    /// moves in ascending order, which scores `in_the_bag` plus their sum where it records `in_the_bag`, no more than
    /// the score of `optimal` where that has the game, and the only line of its game.  A torn last line is cut off.
    pub fn open(path: &Path, optimal: &Optimal) -> Result<UnsoundFile, String> {
        let at = |e: String| format!("{}: {e}", path.display());
        let (mut file, text) = open_locked(path)?;
        let mut games = BTreeMap::new();
        let kept = each_line(&text, |number, json| {
            let (n, stored) = read_line(json, optimal).map_err(|e| format!("line {number}: {e}"))?;
            match games.insert(n, stored) {
                Some(_) => Err(format!("line {number}: a second line for n={n}")),
                None => Ok(()),
            }
        })
        .map_err(at)?;
        mend(&mut file, &text, kept).map_err(|e| at(e.to_string()))?;
        Ok(UnsoundFile { path: path.to_owned(), file, games })
    }

    /// The moves for `n` that the file has, and their score.
    fn moves(&self, n: u32) -> Option<(&[u32], u64)> {
        self.games.get(&n).map(|game| (game.moves.as_slice(), game.score))
    }

    /// Appends `line` and makes sure it reaches the disk.
    fn append(&mut self, line: &Line) -> Result<(), String> {
        append_line(&mut self.file, &line.to_json()).map_err(|e| format!("{}: {e}", self.path.display()))?;
        self.games.insert(line.n, stored_of(line));
        Ok(())
    }
}

/// Dropping releases the lock at once, as a results file does (`results::ResultsFile`).
impl Drop for UnsoundFile {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// What the run keeps of `line`.
fn stored_of(line: &Line) -> Stored {
    Stored {
        score: line.score,
        moves: line.moves.clone(),
        promotions: line.promotions.clone(),
        problem: line.setup.map(|setup| (setup.size(), setup.digest)),
    }
}

/// The game of an unsound line of a file, checked (see [`UnsoundFile::open`]).
fn read_line(json: &Json, optimal: &Optimal) -> Result<(u32, Stored), String> {
    const NOT_UNSOUND: &str =
        "not an unsound line (a line of a proven run, or no game): a file of unsound lines holds no other kind";
    if json.get(UNSOUND) != Some(&Json::Bool(true)) {
        return Err(NOT_UNSOUND.to_owned());
    }
    let f = Fields(json);
    let n = f.u32("n")?;
    if !(1..=super::MAX_N).contains(&n) {
        return Err(format!("n={n} is not a game from 1 to {}", super::MAX_N));
    }
    let (score, moves, promotions) = (f.u64("score")?, f.u32s("moves")?, f.u32s("promotions")?);
    let fail = |message: String| format!("(n={n}) {message}");
    let replayed = replay(n, &moves).map_err(fail)?;
    if replayed != score {
        return Err(fail(format!("the moves score {replayed}, not the recorded {score}")));
    }
    let played: BTreeSet<u32> = moves.iter().copied().collect();
    if !promotions.windows(2).all(|pair| pair[0] < pair[1]) || !promotions.iter().all(|p| played.contains(p)) {
        return Err(fail("the promotions are not moves in ascending order".into()));
    }
    let sum: u64 = promotions.iter().map(|&p| u64::from(p)).sum();
    if f.opt_u64("in_the_bag")?.is_some_and(|bag| bag + sum != score) {
        return Err(fail(format!("it scores {score}, not in_the_bag plus the promotions' {sum}")));
    }
    if let Some(best) = optimal.score(n).filter(|&best| score > best) {
        return Err(fail(format!("it scores {score}, above the built-in table's {best}, which is proven")));
    }
    let problem = match f.get("setup")? {
        Json::Null => None,
        setup => {
            let s = Fields(setup);
            let size = ProblemSize {
                frames: s.u32("frames")?,
                candidates: s.u32("candidates")?,
                max_total: s.u32("max_total")?,
            };
            Some((size, s.digest("digest")?))
        }
    };
    Ok((n, Stored { score, moves, promotions, problem }))
}

// ---------------------------------------------------------------------------------------------------------
// A range

/// Plays the games `lo..=hi` in the unsound mode with `plan` (see the module documentation), on `opts.threads` threads
/// with `opts.tactics`, appending each line to the file `out`, if there is one, then handing it to `on_game`.  The
/// games the file has are skipped.  A stop signal ends the range with the game in play unrecorded (`stopped_at`).
///
/// # Errors
///
/// A range that is empty or starts below 1; a plan that [`check_plan`] refuses (an effort above [`MAX_EFFORT`], a `fix`
/// that [`check_fix`] refuses, a `back` above [`MAX_BACK`], a `widen` of 0 or with a `fix`);
/// `--stop-at-known` or `--from-known` for a game the built-in table lacks; a file that cannot be opened or written, or
/// holds a line it may not ([`UnsoundFile::open`]); and a game whose moves fail a check (the module documentation, "One
/// game").
pub fn solve_range(
    lo: u32,
    hi: u32,
    opts: &SolveOptions,
    plan: &Plan,
    out: Option<&Path>,
    mut on_game: impl FnMut(&Line),
) -> Result<RangeSummary, String> {
    if lo < 1 || lo > hi {
        return Err(format!("the range {lo}..={hi} is empty or starts below 1"));
    }
    check_plan(plan)?;
    let optimal = Optimal::built_in();
    if plan.stop_at_known || plan.from_known {
        if let Some(n) = (lo..=hi).find(|&n| optimal.score(n).is_none()) {
            return Err(format!(
                "--stop-at-known and --from-known measure the games whose opt(n) is proven, and the built-in table \
                 lacks game {n}"
            ));
        }
    }
    let mut file = out.map(|path| UnsoundFile::open(path, optimal)).transpose()?;
    // The lines of the file that set a search up, whose problems a game may repeat.
    let mut earlier = Earlier::<()>::default();
    for (&n, game) in file.iter().flat_map(|file| &file.games) {
        if let Some((size, digest)) = game.problem {
            earlier.add(n, size, digest, game.promotions.clone(), ());
        }
    }
    let mut summary = RangeSummary::default();
    // This run's lines of the games before, for as many as a game may begin a trajectory from.
    let mut run: BTreeMap<u32, Moves> = BTreeMap::new();
    for n in lo..=hi {
        if file.as_ref().is_some_and(|file| file.games.contains_key(&n)) {
            summary.skipped += 1;
            continue;
        }
        if engine::stop_signal().is_some() {
            summary.stopped_at = Some(n);
            break;
        }
        let past = moves_before(n, &run, file.as_ref(), plan, optimal);
        let Some(line) = play(n, &past, opts, plan, &mut earlier, optimal)? else {
            summary.stopped_at = Some(n);
            break;
        };
        if let Some(file) = file.as_mut() {
            file.append(&line)?;
        }
        on_game(&line);
        summary.solved += 1;
        if let Some(setup) = line.setup {
            earlier.add(n, setup.size(), setup.digest, line.promotions.clone(), ());
        }
        run.insert(n, Moves { n, moves: line.moves, score: line.score, source: Source::Run });
        run.retain(|&m, _| m + MAX_BACK > n);
    }
    Ok(summary)
}

/// The moves for the games `n − 1`, `n − 2`, ... that game `n` starts from and begins its trajectories from, as many as
/// `plan.back` and at least the first, from game `n − 1` down: each is, with `--from-known`, the built-in table's,
/// else this run's line of the game (`run`), else the file's line, else the table's; none for a game below 1, or that
/// nothing has the moves for.  A game that lacks the moves for `n − 1` has a note on stderr.
fn moves_before(
    n: u32,
    run: &BTreeMap<u32, Moves>,
    file: Option<&UnsoundFile>,
    plan: &Plan,
    optimal: &Optimal,
) -> Vec<Option<Moves>> {
    let moves_of = |game: u32| {
        let table = || {
            let (moves, score) = (optimal.moves(game)?, optimal.score(game)?);
            Some(Moves { n: game, moves: moves.to_vec(), score, source: Source::Table })
        };
        if plan.from_known {
            return table();
        }
        let file = || {
            let (moves, score) = file?.moves(game)?;
            Some(Moves { n: game, moves: moves.to_vec(), score, source: Source::Results })
        };
        run.get(&game).cloned().or_else(file).or_else(table)
    };
    (1..=plan.back.max(1))
        .map(|j| {
            let found = n.checked_sub(j).filter(|&game| game >= 1).and_then(moves_of);
            if found.is_none() && j == 1 && n > 1 {
                let _ = writeln!(
                    std::io::stderr(),
                    "note: no moves for {} (in this run, the file or the built-in table): game {n} starts with no \
                     seed",
                    n - 1
                );
            }
            found
        })
        .collect()
}

/// Game `n` played in the unsound mode, from the moves for the games before it, `past` ([`moves_before`]: the first is
/// the moves for `n − 1`, which the start is seeded with, and the others begin trajectories if `plan.back` asks; see
/// the module documentation, "One game"), and its line, checked; `None` if a stop signal ended its rounds.  `earlier`
/// has the lines a repeat may repeat, and is told of the problem of this game's board.  An error names the game.
fn play(
    n: u32,
    past: &[Option<Moves>],
    opts: &SolveOptions,
    plan: &Plan,
    earlier: &mut Earlier<()>,
    optimal: &Optimal,
) -> Result<Option<Line>, String> {
    let began = Instant::now();
    let clock = || began.elapsed().as_secs_f64();
    let before = past.first().and_then(Option::as_ref);
    // The moves for n − 1 are a seed and the prime shortcut's moves, which nothing checks again: the run's and the
    // file's lines were replayed before they were kept, and the table's are proven.
    let given = if before.is_some() { Before::Run } else { Before::None };
    let mut line = Line {
        n,
        score: 0,
        moves: Vec::new(),
        promotions: Vec::new(),
        method: Method::Search,
        in_the_bag: None,
        prev: None,
        repeats: None,
        setup: None,
        plan: *plan,
        start: None,
        trajectories: None,
        rounds: 0,
        steps: 0,
        work: 0,
        stage_rounds: None,
        improvements: Vec::new(),
        seconds: 0.0,
        start_seconds: 0.0,
        threads: opts.threads,
        known: None,
        tactics: opts.tactics,
    };
    let known = plan.stop_at_known.then(|| optimal.score(n).expect("the range was checked against the table"));
    match set_up(n, &given, opts).map_err(Failure::message)? {
        SetUp::Derived(Derived::Trivial { plain, in_the_bag, .. }) => {
            (line.method, line.moves, line.in_the_bag) = (Method::Trivial, plain, Some(in_the_bag));
        }
        SetUp::Derived(Derived::Prime { candidates }) => {
            let before = before.expect("the prime shortcut is taken only with the moves for n − 1");
            line.method = Method::Prime;
            (line.moves, line.promotions) = prime_game(n, &candidates, &before.moves);
        }
        SetUp::Search(set, problem) => {
            let repeated = opts.tactics.same_search.then(|| earlier.find(n, &problem)).flatten();
            earlier.remember(n, problem);
            line.setup = Some(set.setup);
            line.in_the_bag = Some(set.in_the_bag);
            if let Some(m) = repeated {
                let promotions = earlier.promotions(m).expect("a game that find found").to_vec();
                (line.method, line.repeats) = (Method::Repeat, Some(m));
                line.moves = set.job.moves_for(&promotions);
                line.promotions = promotions;
            } else {
                let bag = set.in_the_bag;
                let enough = |sum: u64| known.is_some_and(|known| bag + sum >= known);
                let seed = before.map(|before| before.moves.as_slice());
                // The games a trajectory may begin from, from n − 1 down.
                let back: Vec<(u32, &[u32])> =
                    past.iter().take(plan.back as usize).flatten().map(|m| (m.n, m.moves.as_slice())).collect();
                let improved = set.job.improve(seed, &back, plan, opts.threads, &clock, &enough);
                if improved.interrupted {
                    return Ok(None);
                }
                line.start = Some(bag + improved.start);
                line.start_seconds = improved.start_seconds;
                line.trajectories = Some(improved.trajectories);
                (line.rounds, line.steps, line.work) = (improved.rounds, improved.nodes, improved.work);
                line.stage_rounds = Some(improved.stage_rounds);
                line.improvements = improved
                    .improvements
                    .iter()
                    .map(|i| Raised {
                        round: i.round,
                        trajectory: i.trajectory,
                        stage: i.stage,
                        steps: i.steps,
                        work: i.work,
                        seconds: i.seconds,
                        score: bag + i.sum,
                    })
                    .collect();
                (line.moves, line.promotions) = (improved.moves, improved.promotions);
            }
        }
    }
    conclude(&mut line, before, known, optimal).map_err(|e| format!("n={n}: {e}"))?;
    line.seconds = clock();
    Ok(Some(line))
}

/// The checks of the moves of `line` before they count (the module documentation, "One game"), and what they give the
/// line: its score, its record of the moves for n − 1 it started from, `before`, and what `--stop-at-known` recorded,
/// if it was given (`known`, the table's score).
fn conclude(line: &mut Line, before: Option<&Moves>, known: Option<u64>, optimal: &Optimal) -> Result<(), String> {
    let n = line.n;
    let method = line.method.as_str();
    line.score = replay(n, &line.moves).map_err(|e| format!("the {method} moves do not stand up: {e}"))?;
    let (score, sum): (u64, u64) = (line.score, line.promotions.iter().map(|&p| u64::from(p)).sum());
    if let Some(bag) = line.in_the_bag.filter(|&bag| bag + sum != score) {
        return Err(format!(
            "the {method} moves score {score}, not inTheBag {bag} + their promotions' {sum}: the moves and the frame \
             model disagree (promotions {:?})",
            line.promotions
        ));
    }
    if let Some(best) = optimal.score(n).filter(|&best| score > best) {
        return Err(format!(
            "the {method} moves score {score}, above the built-in table's {best}, which is proven (promotions {:?})",
            line.promotions
        ));
    }
    line.prev = before.map(|before| Previous {
        n: n - 1,
        score: before.score,
        source: before.source,
        moves_digest: moves_digest(&before.moves),
        short_by: score.saturating_sub(before.score + u64::from(n)),
    });
    line.known = known.map(|known| {
        // The round after which the score was the table's: the first improvement's that reached it, else the start's.
        let reached = line.improvements.iter().find(|raised| raised.score >= known).map(|raised| raised.round);
        Known { score: known, round: reached.or_else(|| (line.start.unwrap_or(score) >= known).then_some(0)) }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_budget_of_a_level_doubles_with_each_level_and_cannot_overflow() {
        let plan = |effort| Plan { effort, ..Plan::default() };
        let budgets: Vec<u64> = (0..=4).map(|effort| plan(effort).budget()).collect();
        assert_eq!(budgets, [0, 65_536, 131_072, 262_144, 524_288]);
        assert_eq!(ROUND_WORK, 65_536, "one round of 64 neighborhoods of 1,024 steps");
        // The most effort budgets 2^31 rounds of work: 2^47, far below what a count of work holds.
        assert_eq!(plan(MAX_EFFORT).budget(), ROUND_WORK << 31);
        assert_eq!(plan(MAX_EFFORT).budget(), 1 << 47);
        // A step budget of effort 1 that no flag can give still cannot overflow.
        let huge = Plan { effort: MAX_EFFORT, round_work: u64::MAX, ..Plan::default() };
        assert_eq!(huge.budget(), u64::MAX);
    }

    #[test]
    fn the_default_plan_searches_as_the_first_form_did() {
        let plan = Plan::default();
        assert_eq!((plan.effort, plan.seed, plan.fix, plan.tilt), (0, 1, (0.7, 0.9), false));
        assert_eq!((plan.back, plan.widen, plan.round_work), (0, None, ROUND_WORK));
        assert!(!plan.stop_at_known && !plan.from_known);
        let lns = plan.lns();
        let default = finder::Options::default();
        assert_eq!((lns.fix, lns.unit_nodes, lns.seed), (default.fix, default.unit_nodes, default.seed));
        assert!(!lns.tilt && lns.widen.is_none());
        assert_eq!(check_plan(&plan), Ok(()));
    }

    #[test]
    fn a_number_of_games_back_and_a_widening_are_whole_numbers_in_range() {
        assert_eq!((parse_back("0"), parse_back("1"), parse_back("8")), (Ok(0), Ok(1), Ok(8)));
        for text in ["9", "-1", "", "a", "1.5", "1,2", " 1", "99999999999"] {
            let err = parse_back(text).unwrap_err();
            assert!(err.starts_with("--back takes J, a whole number from 0 to 8"), "{text:?}: {err}");
        }
        assert_eq!((parse_widen("1"), parse_widen("30")), (Ok(1), Ok(30)));
        for text in ["0", "-1", "", "a", "2.5", "1,2", " 3"] {
            let err = parse_widen(text).unwrap_err();
            assert!(err.starts_with("--widen takes S, a whole number of rounds, at least 1"), "{text:?}: {err}");
        }
    }

    #[test]
    fn a_plan_with_an_option_out_of_range_is_refused() {
        let plan = Plan { effort: 3, back: 8, widen: Some(1), ..Plan::default() };
        assert_eq!(check_plan(&plan), Ok(()));
        let refused = |plan: Plan| check_plan(&plan).unwrap_err();
        assert!(refused(Plan { effort: MAX_EFFORT + 1, ..plan }).contains("more than the most"));
        assert!(refused(Plan { fix: (0.9, 0.7), ..Plan { widen: None, ..plan } }).starts_with("--fix takes LO,HI"));
        assert!(refused(Plan { back: MAX_BACK + 1, ..plan }).starts_with("--back takes J"));
        assert!(refused(Plan { widen: Some(0), ..plan }).starts_with("--widen takes S"));
        assert_eq!(refused(Plan { fix: (0.5, 0.7), ..plan }), WIDEN_WITH_FIX);
        assert!(refused(Plan { round_work: 0, ..plan }).contains("is 0"));
    }

    #[test]
    fn a_trajectory_begins_from_a_set_that_no_trajectory_before_it_began_from() {
        const W: usize = 16;
        let set = |numbers: &[u32]| numbers.iter().copied().collect::<Bits<W>>();
        let (start, a, b) = (set(&[10, 20]), set(&[10, 21]), set(&[5]));
        // The start's trajectory is first, the games follow in order, and each is a trajectory if its set is new.
        let (starts, origins) = trajectories(start, [(7, a), (6, b)]);
        assert_eq!((starts, origins), (vec![start, a, b], vec![Origin::Start, Origin::Game(7), Origin::Game(6)]));
        // A set equal to the start's, or to an earlier game's, adds none.
        let (starts, origins) = trajectories(start, [(7, start), (6, a), (5, start), (4, a), (3, b), (2, a)]);
        assert_eq!((starts, origins), (vec![start, a, b], vec![Origin::Start, Origin::Game(6), Origin::Game(3)]));
        // No game back is the start alone.
        let (starts, origins) = trajectories(start, []);
        assert_eq!((starts, origins), (vec![start], vec![Origin::Start]));
        assert_eq!(
            Json::array([Origin::Start, Origin::Game(4224)].map(Origin::to_json)).to_string(),
            r#"["start",4224]"#
        );
    }

    #[test]
    fn a_share_fixed_is_a_range_inside_0_to_1() {
        for (text, fix) in
            [("0.7,0.9", (0.7, 0.9)), ("0,1", (0.0, 1.0)), ("0.5,0.5", (0.5, 0.5)), ("1e-1,.2", (0.1, 0.2))]
        {
            assert_eq!(parse_fix(text), Ok(fix), "{text}");
        }
        for text in [
            "",
            "0.5",
            "0.5,",
            ",0.5",
            "0.5,0.7,0.9",
            "a,b",
            "0.7;0.9",
            "0.9,0.7",
            "-0.1,0.5",
            "0.5,1.1",
            "nan,0.5",
            "0.5,inf",
        ] {
            let err = parse_fix(text).unwrap_err();
            assert!(err.starts_with("--fix takes LO,HI") && err.contains("0 <= LO <= HI <= 1"), "{text:?}: {err}");
        }
        assert_eq!(check_fix((0.3, 0.5)), Ok(()));
        assert!(check_fix((f64::NAN, 0.5)).is_err() && check_fix((0.5, f64::NAN)).is_err());
    }
}
