//! The unsound mode, `taxman solve LO-HI --unsound --effort L --out FILE`: each game played as well as a level of
//! effort finds, with no proof that its score is opt(n).  The user's guide to it is solver/docs/unsound.md.
//!
//! The proof of a hard game past 5060 takes hours, and nearly all of it goes to proving a set that the search found
//! early.  The unsound mode keeps the finding and drops the proof.  Its line holds legal moves and their score, a lower
//! bound on opt(n), and is marked unsound ([`UNSOUND`]), so that no reader of proven lines takes one.  The user names a
//! level of effort and nothing else: more effort costs more time and gives a better chance that each score is opt(n).
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
//! * The run's ceiling is n + score(n − 1) of the moves the game starts from.  A searched game stops there: its rounds
//!   end once its score reaches it, before the first round if the start does, else after the round that does (a game
//!   with no moves for n − 1 has no ceiling).  When those moves are optimal, it is the proven mode's ceiling,
//!   n + opt(n − 1), which no play of game n exceeds, and the stop loses nothing.  Nothing here says they are optimal.
//!   When they fall short, a game could score above the ceiling, which would prove the line of n − 1 short
//!   (`prev.short_by`, as before); the stop gives that chance up, for the rounds it saves in every other game.  So the
//!   stop is a heuristic and no proof: the line stays unsound, and records the ceiling and whether the game is at it
//!   (`ceiling`, `at_ceiling`).  A game is not held below the ceiling, and a round that reaches it may pass it.
//!   `--past-ceiling` takes the stop away, to measure what it costs.
//! * The moves are replayed before they count ([`replay`]), and a score above the built-in table's, which is proven, is
//!   a hard error.
//!
//! # The level of effort
//!
//! The levels are [`MIN_EFFORT`] to [`MAX_EFFORT`], 1 to 6, each twice the work of the one below.  Level L runs rounds
//! of [`NEIGHBORHOODS`] neighborhoods until the work of its rounds reaches 2^L × [`ROUND_WORK`] ([`Plan::budget`]); the
//! budget is checked after each round, and the round that reaches it is the last.  The start alone is no level, since
//! it never repairs a shortfall.  The work of a round is its engine's steps and [`finder::NEIGHBORHOOD_WORK`] for each
//! of its neighborhoods, which have a cost beside their steps, so that the time a budget takes is much the same
//! whatever the options.  The steps of a neighborhood are a function of its unit, never of the threads, so the rounds
//! do not depend on the level: a run at L + 1 does the rounds of L first, and then more.  So the `improvements` of a
//! line at one level show what every lower level would have scored, and each records the `steps` and the `work` of the
//! rounds up to its own.
//!
//! # The shape of the rounds
//!
//! Five options of the plan change what the rounds search, and so the line.  Every value of them, alone or together,
//! keeps what the rounds guarantee: a line depends on the game, its start, the moves it starts from, the level, the
//! seed and the options only, never on the threads, and a run at L + 1 does the rounds of L first.
//!
//! * **`fix`** is the range from which each neighborhood draws the share of the incumbent's promotions it keeps fixed
//!   ([`Options::fix`](crate::search::finder::Options::fix)).  A smaller share frees more of the game to a
//!   neighborhood, which then takes more of its steps.
//! * **`tilt`** P makes P percent of a round's neighborhoods, an equal share of each trajectory's, choose the
//!   promotions they free by their size, the small ones more often, with the same number freed; the others draw
//!   uniformly ([`Options::tilt`](crate::search::finder::Options::tilt), [`draws_tilted`](finder::draws_tilted)).  0
//!   tilts none, and 100 every neighborhood.
//! * **`back`** J gives a game a trajectory of incumbents for each distinct set that survives of the moves for the
//!   games n − 1, n − 2, ..., n − J ([`seed_survivors`](crate::search::seed_survivors)), as the finder beside a proof
//!   search starts its seed rounds, beside the first, the start's.  A set that an earlier trajectory (the start's
//!   included) began from is skipped, and so is a game whose moves nothing has.  Neighborhood k of a round works from
//!   the incumbent of trajectory k mod T, T the trajectories, and the game's set is the heaviest incumbent, the
//!   start's on a tie ([`Lns::rounds`](crate::search::finder::Lns::rounds)).  The line says which games the
//!   trajectories began from (`trajectories`).  What survives of an earlier game may be heavier than the start; the
//!   game's score is then that of the survivors from the first, which the line records as an improvement of round 0.
//!   0 gives the start's trajectory alone.  The line records the score of what survives of each of these games, whether
//!   or not it began a trajectory (`survivor_scores`).
//! * **`above`** J (0 by default) adds, after those of `back`, a trajectory for each distinct set that survives of the
//!   moves for the games n + 1, ..., n + J that the run or its file has, or with `--from-known` the built-in table: the
//!   moves above n are dropped ([`Board::seed`](crate::board::Board::seed)).  A game played in order has no line above
//!   it in its own run, so this is for measuring: with `--from-known` it shows what the games above could give a game
//!   if their lines were optimal.
//! * **`widen`** S moves the neighborhoods of a trajectory whose incumbent has not grown heavier in S rounds down the
//!   ladder of the share kept fixed ([`LADDER`](crate::search::finder::LADDER)), and back to its first stage when it
//!   grows ([`Options::widen`](crate::search::finder::Options::widen)).  The line records the rounds each trajectory
//!   spent at each stage (`stage_rounds`) and the stage of each improvement.  The ladder replaces `fix`, so a plan
//!   that widens has the default `fix`, and `--fix` turns widening off ([`widening`]).  `None` is no widening.
//!
//! # The defaults
//!
//! `back` is [`DEFAULT_BACK`], `tilt` [`DEFAULT_TILT`] and `widen` [`DEFAULT_WIDEN`], so that a user names a level of
//! effort and nothing else; `--back 0`, `--tilt 0` and `--widen 0` turn each off, and `fix` is the finder's share.  The
//! three are the setting that two races chose on the 251 hard games of 3441-5060: `--back 8 --tilt 25 --widen 4`
//! reaches the score that the built-in table has in 87.3, 91.2, 94.0, 95.2, 96.8 and 97.6% of them at the levels 1 to
//! 6, against 78.1, 79.7, 81.3, 82.9, 86.1 and 89.6% with no option.  The first race is the notebook's entry
//! https://github.com/bvchess/taxman-notebook/blob/main/search/tilted-back-and-wider-rounds-2026-11/README.md; the
//! second, the race R1, is
//! https://github.com/bvchess/taxman-notebook/blob/main/search/a-share-of-tilted-neighborhoods-2026-11/README.md.  The
//! entries number the levels one higher than here (their effort E is level E − 1), and the numbers above are in this
//! numbering.  At every level, `--tilt 25` and `--tilt 50` (with `--back 8 --widen 4`) are within two games of each
//! other, so 25 is chosen for its lead at the low levels, not by a significant margin.  The finder beside a proof
//! search never tilts, widens or goes back, and its options ([`finder::Options::default`]) are not these.
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
//! While a game plays, a thread of its own makes the setup of the next game to play, and its start's dive
//! ([`Boards`]): neither depends on what the game in play scores, and on many threads both would otherwise be a large
//! part of a game's time at a low level of effort, since they run on one thread.  A setup made ahead is the one the game
//! would make, so the lines are the same.  The run keeps the boards of its last [`REPAIR_REACH`] games in the same
//! place, for the repair.
//!
//! # The repair
//!
//! A game that falls short hands its shortfall on: the games above it start from its line, a prime and a repeat copy
//! it, and a searched game stops at the lower ceiling it sets.  The repair takes the shortfall back down when a line
//! above does better.  It rests on one fact: the moves of a game of N other than those above n, in their order, are a
//! game of n, scoring the moves kept, since leaving a move out only leaves more numbers on the board for the others
//! (the ceiling lemma (N4) of docs/theory/neighbors.md, applied once for each number above n).  So what survives in
//! game n of a line above it is a real game of n, and if it scores more than the line of n, that line is short, and is
//! replaced.  What survives is the set a trajectory of `back` begins from
//! ([`seed_survivors`](crate::search::seed_survivors)): the promotions of those moves that n's board can still promote,
//! from the largest down, and the moves they make, so the new line is one of the frame model, whose promotions a
//! repeat can play.
//!
//! * **When.**  After the run records a line, the walk down ([`Run::walk`]) weighs each game k below it, from the
//!   highest down: what survives on k's board of each line within [`REPAIR_REACH`] above k that the walk has written
//!   (the line just recorded, and those it has replaced), the heaviest, the nearest on a tie.  If that scores more than
//!   the line of k, it replaces it, and the walk goes on from it too; the walk ends at the first game with none of its
//!   lines within reach.  A board comes from the run, which keeps those of its last [`REPAIR_REACH`] games, else is set
//!   up again.  A prime's line is not weighed, since it follows from the line below it: once that is replaced, the
//!   prime is played again from it (the prime formula, N5).  A trivial line is optimal, and the built-in table's lines
//!   are proven: neither is weighed.
//! * **What it changes above.**  Nothing the walk does not reach: the games above the line just recorded are not played
//!   yet, and the games between it and a game repaired were weighed first.  A repeat of a game whose line is replaced
//!   plays its new promotions, and the games that follow begin their trajectories from the new lines.
//! * **The file.**  A replaced line is written again, appended: the line as its game was played, with the repair's
//!   moves, promotions and score, `at_ceiling` and `prev.short_by` of the new score, and `repaired` (the game it came
//!   from, the score it replaced, and its time) ([`repaired_line`]).  The last line of a game is the game's, and a run
//!   that opens the file takes the walk up where the file's last lines leave it, from its last line that is no repair
//!   and the repairs after it.  So a run that a stop cut short between two steps of the walk ends with the lines of one
//!   that ran on.
//! * **The work.**  The repair runs no rounds: the work of a level is that of its rounds, as without it.  Its time is
//!   that of the survivors on the boards kept, small beside a game's, and of the boards it sets up again, a few seconds
//!   each at n ≈ 5,000: a walk past the boards kept needs them, and so does a walk that a run takes up from its file.
//!   The run's summary has the time of its walks ([`RangeSummary::repair_time`](super::RangeSummary::repair_time)).
//! * **What a line depends on.**  The walk's steps depend on the lines alone, never on the threads, so a line still
//!   depends on nothing but the game, its start, the moves it starts from, the level, the seed and the options, of
//!   which the repair is one (`repair`; `--no-repair` turns it off).  The rounds of a game at E + 1 begin with those at
//!   E from the same moves, as before; in a range the repair can change the moves that later games start from, as a
//!   level does.
//!
//! # Measuring
//!
//! Two options measure the mode on the games whose opt(n) is proven, and are refused for a game the built-in table
//! lacks.  `--stop-at-known` ends a game's rounds as soon as its score is the table's (or reaches the run's ceiling, if
//! that comes first), and the line records the round at which it was the table's, or that it never was.  `--from-known`
//! seeds every game with the table's moves for n − 1, not with this run's line, so that the run's ceiling is the proven
//! one.  solver/docs/unsound.md has the keys of the line ("An unsound line").

use super::{
    prime_game, set_up, Before, Derived, Earlier, Failure, Job, Method, RangeSummary, SearchSetup, SetUp, SolveOptions,
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
use std::sync::Arc;
use std::thread;
use std::time::Instant;

/// The least effort a run may ask for: a budget of 2 rounds of work.  The start alone is no level, since it never
/// repairs a shortfall.
pub const MIN_EFFORT: u32 = 1;

/// The most effort a run may ask for: a budget of 64 rounds of work, which is 2^22 with the default.
pub const MAX_EFFORT: u32 = 6;

/// The games before a game that its trajectories may begin from: `back` goes up to this.
pub const MAX_BACK: u32 = 8;

/// The games above a game that its trajectories may begin from: `above` goes up to this.
pub const MAX_ABOVE: u32 = 8;

/// How far below a line the repair looks: what survives of a line is set against the lines of the games up to this
/// many below it (the module documentation, "The repair").
pub const REPAIR_REACH: u32 = 8;

/// The most percent of a round's neighborhoods that may draw tilted: all of them.
pub const MAX_TILT: u32 = 100;

/// The default of `back`: a trajectory from each distinct set that survives of the moves for the 8 games before.  The
/// races that chose it, with [`DEFAULT_TILT`] and [`DEFAULT_WIDEN`], are in the module documentation, "The defaults".
pub const DEFAULT_BACK: u32 = 8;

/// The default of `tilt`: 25 percent of a round's neighborhoods draw tilted (chosen with [`DEFAULT_BACK`] and
/// [`DEFAULT_WIDEN`]; the module documentation, "The defaults", has the races).
pub const DEFAULT_TILT: u32 = 25;

/// The default of `widen`: a trajectory that has not grown heavier in 4 rounds moves down the ladder (chosen with
/// [`DEFAULT_BACK`] and [`DEFAULT_TILT`]; the module documentation, "The defaults", has the races).
pub const DEFAULT_WIDEN: u64 = 4;

/// The work of one round, which level L budgets 2^L times ([`Plan::budget`]): the steps of a round whose neighborhoods
/// all take the step limit of a unit, [`finder::UNIT_STEPS`].  (The work of such a round is a little more,
/// [`finder::NEIGHBORHOOD_WORK`] for each neighborhood.)
pub const ROUND_WORK: u64 = NEIGHBORHOODS * finder::UNIT_STEPS;

/// How the unsound mode plays: the level of effort, the seed of the rounds' random choices, the shape of the
/// neighborhoods and of the rounds, and the measuring options.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    /// From [`MIN_EFFORT`] to [`MAX_EFFORT`].
    pub effort: u32,
    pub seed: u64,
    /// `--fix LO,HI`: the share of the incumbent's promotions that each neighborhood keeps fixed is drawn from this
    /// range, 0 ≤ LO ≤ HI ≤ 1 ([`check_fix`]).  A plan that widens takes the shares from the ladder instead, and keeps
    /// the default here.
    pub fix: (f64, f64),
    /// `--tilt P`, P from 0 to [`MAX_TILT`] ([`DEFAULT_TILT`] by default): P percent of a round's neighborhoods free
    /// the small promotions more often than the large ones, and the rest draw uniformly.  0 tilts none.
    pub tilt: u32,
    /// `--back J`, J from 0 to [`MAX_BACK`] ([`DEFAULT_BACK`] by default): a game follows a trajectory from each
    /// distinct set that survives of the moves for the games n − 1, ..., n − J, beside the start's.  0 follows the
    /// start's alone.
    pub back: u32,
    /// `--above J`, J from 0 to [`MAX_ABOVE`]: a game also follows a trajectory from each distinct set that survives of
    /// the moves for the games n + 1, ..., n + J that the run or its file has (the built-in table's with
    /// `from_known`).  0 takes none.
    pub above: u32,
    /// `--widen S`, S ≥ 1 ([`DEFAULT_WIDEN`] by default): a trajectory that has not grown heavier in S rounds moves its
    /// neighborhoods down the ladder of the share fixed.  `None` (`--widen 0`) is no widening, and takes `fix`.
    pub widen: Option<u64>,
    /// The work of one round, which level L budgets 2^L times: [`ROUND_WORK`] unless a test or a measurement lowers it.
    /// No flag sets it, and a line records the budget it gave (`budget`).
    pub round_work: u64,
    /// `--stop-at-known`: a game's rounds end once its score is the built-in table's.
    pub stop_at_known: bool,
    /// `--from-known`: every game starts from the built-in table's moves for n − 1.
    pub from_known: bool,
    /// `--past-ceiling`: a game's rounds do not end at the run's ceiling, n + score(n − 1) of the moves it starts from.
    pub past_ceiling: bool,
    /// The repair, on unless `--no-repair`: a line that what survives of a line above it beats is replaced (the module
    /// documentation, "The repair").
    pub repair: bool,
}

/// The least effort, the finder's seed and share fixed, the defaults of `tilt`, `back` and `widen` ([`DEFAULT_TILT`],
/// [`DEFAULT_BACK`], [`DEFAULT_WIDEN`]), no trajectory from above, the stop at the run's ceiling, the repair, and no
/// measuring option.
impl Default for Plan {
    fn default() -> Plan {
        let finder::Options { fix, seed, .. } = finder::Options::default();
        Plan {
            effort: MIN_EFFORT,
            seed,
            fix,
            tilt: DEFAULT_TILT,
            back: DEFAULT_BACK,
            above: 0,
            widen: Some(DEFAULT_WIDEN),
            round_work: ROUND_WORK,
            stop_at_known: false,
            from_known: false,
            past_ceiling: false,
            repair: true,
        }
    }
}

impl Plan {
    /// The work the rounds of the level run until: 2^L × `round_work` at level L, at most 2^6 × [`ROUND_WORK`] = 2^22
    /// with the default (a larger `round_work` stops at the largest count of work).  Meant for a plan that
    /// [`check_plan`] accepts.
    pub fn budget(&self) -> u64 {
        self.round_work.saturating_mul(1u64 << self.effort)
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

/// Reads the value of `--effort`, L: a whole number from [`MIN_EFFORT`] to [`MAX_EFFORT`].
///
/// # Errors
///
/// What `text` should have been.
pub fn parse_effort(text: &str) -> Result<u32, String> {
    text.parse().ok().filter(|effort| (MIN_EFFORT..=MAX_EFFORT).contains(effort)).ok_or_else(|| effort_error(text))
}

/// The message of an `--effort` that is not a whole number from [`MIN_EFFORT`] to [`MAX_EFFORT`], quoting what it was
/// given.
fn effort_error(got: &str) -> String {
    format!("--effort takes L, the level of effort, a whole number from {MIN_EFFORT} to {MAX_EFFORT}: got {got:?}")
}

/// Reads the value of `--tilt`, P: a whole number of percent, from 0 to [`MAX_TILT`].
///
/// # Errors
///
/// What `text` should have been.
pub fn parse_tilt(text: &str) -> Result<u32, String> {
    text.parse().ok().filter(|&tilt| tilt <= MAX_TILT).ok_or_else(|| tilt_error(text))
}

/// The message of a `--tilt` that is not a whole number from 0 to [`MAX_TILT`], quoting what it was given.
fn tilt_error(got: &str) -> String {
    format!("--tilt takes P, a whole number of percent from 0 to {MAX_TILT}: got {got:?}")
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

/// Reads the value of `--above`, J: a whole number from 0 to [`MAX_ABOVE`].
///
/// # Errors
///
/// What `text` should have been.
pub fn parse_above(text: &str) -> Result<u32, String> {
    text.parse().ok().filter(|&above| above <= MAX_ABOVE).ok_or_else(|| above_error(text))
}

/// The message of an `--above` that is not a whole number from 0 to [`MAX_ABOVE`], quoting what it was given.
fn above_error(got: &str) -> String {
    format!("--above takes J, a whole number from 0 to {MAX_ABOVE}: got {got:?}")
}

/// Reads the value of `--widen`, S: a whole number of rounds, `None` for 0 (no widening) and `Some(S)` for S at least 1.
///
/// # Errors
///
/// What `text` should have been.
pub fn parse_widen(text: &str) -> Result<Option<u64>, String> {
    let widen: u64 = text.parse().map_err(|_| widen_error(text))?;
    Ok((widen >= 1).then_some(widen))
}

/// The message of a `--widen` that is not a whole number of rounds, quoting what it was given.
fn widen_error(got: &str) -> String {
    format!("--widen takes S, a whole number of rounds, or 0 for no widening: got {got:?}")
}

/// The message of a plan that widens the neighborhoods and also gives the share fixed: the ladder is the share fixed.
pub const WIDEN_WITH_FIX: &str =
    "--widen S of 1 or more takes the share fixed from its own ladder, so --fix is refused with it; --fix alone, or with \
     --widen 0, turns widening off";

/// The widening of a plan from the command line: `fix_given` says whether `--fix` was, and `widen` is the value of
/// `--widen` ([`parse_widen`]) if it was.  Without either, it is the default, [`DEFAULT_WIDEN`].  The ladder of the
/// widening replaces the share fixed, so `--fix` turns widening off.
///
/// # Errors
///
/// [`WIDEN_WITH_FIX`], for `--fix` with a `--widen` of 1 or more.
pub fn widening(fix_given: bool, widen: Option<Option<u64>>) -> Result<Option<u64>, String> {
    match (fix_given, widen) {
        (true, Some(Some(_))) => Err(WIDEN_WITH_FIX.to_owned()),
        (true, _) => Ok(None),
        (false, Some(widen)) => Ok(widen),
        (false, None) => Ok(Plan::default().widen),
    }
}

/// Checks the plan's options: an effort from [`MIN_EFFORT`] to [`MAX_EFFORT`], `fix` ([`check_fix`]), `tilt` of at most
/// [`MAX_TILT`], `back` of at most [`MAX_BACK`], `widen` (if any) of at least 1 and not with a `fix` other than the
/// default, and a work of a round above 0.
///
/// # Errors
///
/// The first option that is out of range.
pub fn check_plan(plan: &Plan) -> Result<(), String> {
    if !(MIN_EFFORT..=MAX_EFFORT).contains(&plan.effort) {
        return Err(effort_error(&plan.effort.to_string()));
    }
    check_fix(plan.fix)?;
    if plan.tilt > MAX_TILT {
        return Err(tilt_error(&plan.tilt.to_string()));
    }
    if plan.back > MAX_BACK {
        return Err(back_error(&plan.back.to_string()));
    }
    if plan.above > MAX_ABOVE {
        return Err(above_error(&plan.above.to_string()));
    }
    if let Some(widen) = plan.widen {
        if widen < 1 {
            return Err(format!("the plan widens after {widen} rounds: no widening is `None`, as --widen 0 gives"));
        }
        if plan.fix != Plan::default().fix {
            return Err(WIDEN_WITH_FIX.to_owned());
        }
    }
    if plan.round_work == 0 {
        return Err("the work of a round is 0".to_owned());
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

/// An unsound line: a game as the unsound mode played it (the module documentation; solver/docs/unsound.md, "An
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
    /// The run's ceiling on the game's score, n + score(n − 1) of the moves it started from ([`ceiling_over`]); `None`
    /// with no such moves, and with `--past-ceiling`.
    pub ceiling: Option<u64>,
    /// The game's score reached its `ceiling`: a searched game's rounds stopped there (and had none to run if the
    /// start was there).
    pub at_ceiling: bool,
    /// For a repeat, the n of the line whose promotions it plays.
    pub repeats: Option<u32>,
    /// What a searched or repeat game was set up with (its `cap` is always `None`: the proven mode's ceiling on the
    /// promotion sum is the line's `ceiling` here).
    pub setup: Option<SearchSetup>,
    pub plan: Plan,
    /// The score of a searched game's start.
    pub start: Option<u64>,
    /// What each trajectory of a searched game's rounds began from: the start, then the games `--back` and `--above`
    /// took distinct sets from.
    pub trajectories: Option<Vec<Origin>>,
    /// For a searched game, each game whose moves a trajectory could begin from, in the order of `trajectories`, and
    /// the score of what survives of its moves here, whether or not it began one.
    pub survivor_scores: Option<Vec<(u32, u64)>>,
    /// The rounds done, the engine's steps in them, and their work ([`finder::NEIGHBORHOOD_WORK`] a neighborhood
    /// besides the steps).
    pub rounds: u64,
    pub steps: u64,
    pub work: u64,
    /// For each trajectory, the rounds it worked at each stage of the ladder of `--widen` (all at the first without).
    pub stage_rounds: Option<Vec<[u64; STAGES]>>,
    /// Each round that raised the score, in order.
    pub improvements: Vec<Raised>,
    /// The game's wall time, from the end of the game before and of the repair's walk after it (or the start of the
    /// run) to its line, and the part of it before the rounds.  A setup made while the game before played (the module
    /// documentation, "A range") is not in it.
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
        let survivor_scores = self.survivor_scores.as_ref().map_or(Json::Null, |s| {
            Json::array(s.iter().map(|&(game, score)| Json::array([Json::from(game), Json::from(score)])))
        });
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
            ("ceiling", self.ceiling.map_or(Json::Null, Json::from)),
            ("at_ceiling", Json::Bool(self.at_ceiling)),
            ("repeats", self.repeats.map_or(Json::Null, Json::from)),
            ("setup", setup),
            ("effort", Json::from(self.plan.effort)),
            ("budget", Json::from(self.plan.budget())),
            ("seed", Json::from(self.plan.seed)),
            ("fix", Json::array([self.plan.fix.0, self.plan.fix.1])),
            ("tilt", Json::from(self.plan.tilt)),
            ("back", Json::from(self.plan.back)),
            ("above", Json::from(self.plan.above)),
            ("widen", self.plan.widen.map_or(Json::Null, Json::from)),
            ("start", self.start.map_or(Json::Null, Json::from)),
            ("trajectories", trajectories),
            ("survivor_scores", survivor_scores),
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
            ("repair", Json::Bool(self.plan.repair)),
            // A line as a game is played; the repair writes it again with what replaced its moves ([`Repair`]).
            ("repaired", Json::Null),
            ("tactics", Json::from(self.tactics.description())),
            ("build", build_json()),
        ])
    }

    /// One line for a person: n, score, method, the rounds and how many raised the score, the time, `unsound`, and what
    /// `--stop-at-known` recorded, whether the game is at the run's ceiling and a line of n − 1 that this one proves
    /// short, if anything.
    pub fn summary_line(&self) -> String {
        let known = match self.known {
            None => String::new(),
            Some(Known { round: Some(round), .. }) => format!("  known reached in round {round}"),
            Some(Known { round: None, .. }) => "  known not reached".to_owned(),
        };
        let ceiling = if self.at_ceiling { "  at ceiling" } else { "" };
        let short = self
            .prev
            .as_ref()
            .filter(|prev| prev.short_by > 0)
            .map_or_else(String::new, |prev| format!("  n-1 short by {} at least", prev.short_by));
        format!(
            "n={:<5} score={:<9} {:<7} rounds={:<6} improved={:<3} {:>9.3} s  unsound{known}{ceiling}{short}",
            self.n,
            self.score,
            self.method.as_str(),
            self.rounds,
            self.improvements.len(),
            self.seconds
        )
    }
}

/// A line that the repair replaced (the module documentation, "The repair"): game `n`'s, by moves that score more,
/// made from the line of game `from`.  Those are what survives in game `n` of the line of `from`, a game above it, or
/// for a prime, the prime formula on the repaired line of `from`, n − 1.
#[derive(Clone, Debug, PartialEq)]
pub struct Repair {
    pub n: u32,
    pub from: u32,
    /// The method of the line replaced, which its new line keeps.
    pub method: Method,
    /// The score of the line replaced, and the new score.
    pub was: u64,
    pub score: u64,
    pub moves: Vec<u32>,
    /// The numbers promoted, ascending.
    pub promotions: Vec<u32>,
    /// The time it took: the survivors of every line it weighed, its moves and their checks, and the setup of the
    /// game's board if the run did not have it.
    pub seconds: f64,
}

impl Repair {
    /// One line for a person, as [`Line::summary_line`] has for a game played: n, the new score, the method, the game
    /// the moves came from and what it raised the score by, and the time.
    pub fn summary_line(&self) -> String {
        format!(
            "n={:<5} score={:<9} {:<7} repaired from {}, +{} {:>9.3} s  unsound",
            self.n,
            self.score,
            self.method.as_str(),
            self.from,
            self.score - self.was,
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
    /// Each game whose moves a trajectory could begin from, and the promotion sum of what survives of them.
    pub(super) survivors: Vec<(u32, u64)>,
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

/// A line of an unsound file as the run needs it: the moves for the next game, and a repeat's, and what the repair
/// needs to write the line again.
#[derive(Clone, Debug)]
struct Stored {
    score: u64,
    moves: Vec<u32>,
    promotions: Vec<u32>,
    method: Method,
    /// The size and the digest of the search problem of a searched or repeat game.
    problem: Option<(ProblemSize, u64)>,
    /// The line as it was written, which a repair writes again with its own moves ([`repaired_line`]).
    text: String,
}

/// The lines of a run: those of the file of unsound lines that it appends to, one JSON object a line, an unsound line
/// each, and its own as it records them.  Opening the file locks it against other processes and checks it; each line
/// is appended and synced as its game ends or a repair replaces it.  It has the torn last line of a results file, and
/// is mended as one is.  A run without a file keeps its lines here alone.
///
/// A game has one line, or more if the repair replaced it: the last line of a game is the game's, and each line after
/// the first records a repair and scores more than the one before it.
pub struct UnsoundFile {
    /// The file and its path, if the run has one, locked (`flock`) for as long as it is open.
    file: Option<(File, PathBuf)>,
    /// The last line of each game.
    games: BTreeMap<u32, Stored>,
    /// The games of the file's last line that is no repair and of the repairs after it: where the repair's walk down
    /// was when the file was last written, which a run that opens it takes up again ([`Run::walk`]).
    walk: BTreeSet<u32>,
}

impl UnsoundFile {
    /// Opens (creating if need be) the file at `path`, locks it, and checks every line: an unsound line (a line of a
    /// proven run is refused, as is a line that is not a game), whose moves replay to its score, whose promotions are
    /// moves in ascending order, which scores `in_the_bag` plus their sum where it records `in_the_bag`, no more than
    /// the score of `optimal` where that has the game, and either the first line of its game or a repair of the line
    /// before it, which scores more.  A torn last line is cut off.
    pub fn open(path: &Path, optimal: &Optimal) -> Result<UnsoundFile, String> {
        let at = |e: String| format!("{}: {e}", path.display());
        let (mut file, text) = open_locked(path)?;
        let (mut games, mut walk) = (BTreeMap::new(), BTreeSet::new());
        let kept = each_line(&text, |number, json| {
            let (n, stored, was) = read_line(json, optimal).map_err(|e| format!("line {number}: {e}"))?;
            match (games.get(&n).map(|game: &Stored| game.score), was) {
                (None, None) => walk.clear(),
                (Some(score), Some(was)) if was == score => (),
                (None, Some(_)) => {
                    return Err(format!("line {number}: a repair of n={n}, which has no line before it"))
                }
                (Some(score), Some(was)) => {
                    return Err(format!("line {number}: a repair of a line of n={n} that scored {was}, not {score}"))
                }
                (Some(_), None) => return Err(format!("line {number}: a second line for n={n} that is no repair")),
            }
            walk.insert(n);
            games.insert(n, stored);
            Ok(())
        })
        .map_err(at)?;
        mend(&mut file, &text, kept).map_err(|e| at(e.to_string()))?;
        Ok(UnsoundFile { file: Some((file, path.to_owned())), games, walk })
    }

    /// The lines of a run that has no file.
    fn none() -> UnsoundFile {
        UnsoundFile { file: None, games: BTreeMap::new(), walk: BTreeSet::new() }
    }

    /// The moves for `n` that the file has, and their score.
    fn moves(&self, n: u32) -> Option<(&[u32], u64)> {
        self.games.get(&n).map(|game| (game.moves.as_slice(), game.score))
    }

    /// Appends the line `json` of game `n`, which the run keeps as `stored`, and makes sure it reaches the disk.
    fn append(&mut self, n: u32, json: &Json, stored: Stored) -> Result<(), String> {
        if let Some((file, path)) = &mut self.file {
            append_line(file, json).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        self.games.insert(n, stored);
        Ok(())
    }
}

/// Dropping releases the lock at once, as a results file does (`results::ResultsFile`).
impl Drop for UnsoundFile {
    fn drop(&mut self) {
        if let Some((file, _)) = &self.file {
            let _ = file.unlock();
        }
    }
}

/// What the run keeps of `line`, whose JSON is `json`.
fn stored_of(line: &Line, json: &Json) -> Stored {
    Stored {
        score: line.score,
        moves: line.moves.clone(),
        promotions: line.promotions.clone(),
        method: line.method,
        problem: line.setup.map(|setup| (setup.size(), setup.digest)),
        text: json.to_string(),
    }
}

/// The game of an unsound line of a file, checked (see [`UnsoundFile::open`]), and if it records a repair, the score of
/// the line it replaced.
fn read_line(json: &Json, optimal: &Optimal) -> Result<(u32, Stored, Option<u64>), String> {
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
    let method = f.str("method").ok().and_then(Method::parse).ok_or_else(|| Fields::bad("method"))?;
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
    let was = match f.get("repaired")? {
        Json::Null => None,
        repaired => Some(Fields(repaired).u64("was")?).filter(|&was| was < score),
    };
    if f.get("repaired")? != &Json::Null && was.is_none() {
        return Err(fail(format!("it records a repair, and scores {score}, no more than the line it replaced")));
    }
    let text = json.to_string();
    Ok((n, Stored { score, moves, promotions, method, problem, text }, was))
}

/// The line `old`, the JSON of a line of game `n`, written again for `repair`: its moves, promotions and score are the
/// repair's, the repair is recorded (`repaired`), `at_ceiling` and the short fall of the moves for n − 1 (`prev`) are
/// those of the new score, and `build` is this build.  `prev`, if given, replaces the moves for n − 1 the line started
/// from, and the run's ceiling with them: a prime is played again from the repaired line below it.  The rest is the
/// line as its game was played.
fn repaired_line(old: &str, repair: &Repair, prev: Option<&Moves>) -> Result<Json, String> {
    let old = Json::parse(old).map_err(|e| format!("n={}: the line to repair: {e}", repair.n))?;
    let f = Fields(&old);
    let ceiling = match (prev, f.opt_u64("ceiling")?) {
        (Some(prev), Some(_)) => Some(ceiling_over(repair.n, prev)),
        (_, ceiling) => ceiling,
    };
    let prev = match (prev, f.get("prev")?) {
        (Some(prev), _) => Some((prev.n, prev.score, Json::from(prev.source.as_str()), moves_digest(&prev.moves))),
        (None, Json::Null) => None,
        (None, old) => {
            let p = Fields(old);
            Some((p.u32("n")?, p.u64("score")?, p.get("source")?.clone(), p.str("moves_digest")?.to_owned()))
        }
    };
    let prev = prev.map_or(Json::Null, |(n, score, source, digest)| {
        Json::object([
            ("n", Json::from(n)),
            ("score", Json::from(score)),
            ("source", source),
            ("moves_digest", Json::from(digest)),
            ("short_by", Json::from(repair.score.saturating_sub(score + u64::from(repair.n)))),
        ])
    });
    let repaired = Json::object([
        ("from", Json::from(repair.from)),
        ("was", Json::from(repair.was)),
        ("seconds", Json::Num(repair.seconds)),
    ]);
    let new = [
        ("score", Json::from(repair.score)),
        ("moves", Json::array(repair.moves.iter().copied())),
        ("promotions", Json::array(repair.promotions.iter().copied())),
        ("prev", prev),
        ("ceiling", ceiling.map_or(Json::Null, Json::from)),
        ("at_ceiling", Json::Bool(ceiling.is_some_and(|ceiling| repair.score >= ceiling))),
        ("repaired", repaired),
        ("build", build_json()),
    ];
    let Json::Obj(pairs) = old else { return Err(format!("n={}: the line to repair is not an object", repair.n)) };
    let pairs = pairs.into_iter().map(|(key, value)| {
        let value = new.iter().find(|(name, _)| *name == key).map_or(value, |(_, value)| value.clone());
        (key, value)
    });
    Ok(Json::Obj(pairs.collect()))
}

// ---------------------------------------------------------------------------------------------------------
// A range

/// Plays the games `lo..=hi` in the unsound mode with `plan` (see the module documentation), on `opts.threads` threads
/// with `opts.tactics`, appending each line to the file `out`, if there is one, then handing it to `on_game`.  The
/// games the file has are skipped.  With the repair, each line the run records is weighed against the lines below it,
/// and each that it replaces is appended and handed to `on_repair` (the module documentation, "The repair"); first, a
/// walk down that a stop cut short in the file goes on.  A stop signal ends the range with the game in play unrecorded
/// (`stopped_at`).
///
/// # Errors
///
/// A range that is empty or starts below 1; a plan that [`check_plan`] refuses (an effort outside [`MIN_EFFORT`] to
/// [`MAX_EFFORT`], a `fix` that [`check_fix`] refuses, a `tilt` above [`MAX_TILT`], a `back` above [`MAX_BACK`], an
/// `above` above [`MAX_ABOVE`], a `widen` of `Some(0)` or with a `fix`); `--stop-at-known` or `--from-known` for a game
/// the built-in table lacks; a file that cannot be opened or written, or holds a line it may not
/// ([`UnsoundFile::open`]); and a game or a repair whose moves fail a check (the module documentation, "One game").
pub fn solve_range(
    lo: u32,
    hi: u32,
    opts: &SolveOptions,
    plan: &Plan,
    out: Option<&Path>,
    mut on_game: impl FnMut(&Line),
    mut on_repair: impl FnMut(&Repair),
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
    let lines = out.map_or_else(|| Ok(UnsoundFile::none()), |path| UnsoundFile::open(path, optimal))?;
    // The lines of the file that set a search up, whose problems a game may repeat.
    let mut earlier = Earlier::<()>::default();
    for (&n, game) in &lines.games {
        if let Some((size, digest)) = game.problem {
            earlier.add(n, size, digest, game.promotions.clone(), ());
        }
    }
    let mut run = Run {
        opts,
        optimal,
        lines,
        earlier,
        recent: BTreeMap::new(),
        mine: BTreeSet::new(),
        boards: Boards::new(opts),
        summary: RangeSummary::default(),
    };
    if plan.repair {
        let walk = run.lines.walk.clone();
        run.walk(walk, &mut on_repair)?;
    }
    for n in lo..=hi {
        if run.lines.games.contains_key(&n) {
            run.summary.skipped += 1;
            continue;
        }
        if engine::stop_signal().is_some() {
            run.summary.stopped_at = Some(n);
            break;
        }
        let past = moves_before(n, &run.recent, &run.lines, plan, optimal);
        let above = moves_above(n, &run.recent, &run.lines, plan, optimal);
        // The next game to play has the moves for its n − 1: this game's line, or the file's.
        if let Some(next) = (n + 1..=hi).find(|m| !run.lines.games.contains_key(m)) {
            run.boards.start(next);
        }
        let played = play(n, &past, &above, opts, plan, &mut run.earlier, &mut run.boards, optimal)?;
        let Some((line, board)) = played else {
            run.summary.stopped_at = Some(n);
            break;
        };
        run.record(line, board, &mut on_game)?;
        if plan.repair {
            run.walk(BTreeSet::from([n]), &mut on_repair)?;
        }
    }
    Ok(run.summary)
}

/// A board that the run set up for a searched or repeat game, which the repair weighs lines on: the game's job, and
/// what the game with no promotions scores.
#[derive(Clone)]
struct KeptBoard {
    job: Arc<dyn Job>,
    in_the_bag: u64,
}

/// A run of the unsound mode as it goes: its lines, the lines a repeat may repeat, its own lines of the games a game
/// starts from, its boards (made ahead, and kept for the repair), and what it has done.
struct Run<'a> {
    opts: &'a SolveOptions,
    optimal: &'a Optimal,
    lines: UnsoundFile,
    earlier: Earlier<()>,
    /// This run's lines of the games before, for as many as a game may begin a trajectory from.
    recent: BTreeMap<u32, Moves>,
    /// The games whose line this run wrote, played or repaired.
    mine: BTreeSet<u32>,
    boards: Boards,
    summary: RangeSummary,
}

impl Run<'_> {
    /// Records `line`, a game played on `board` (if it set one up): appends it, hands it to `on_game`, and keeps what
    /// the games after it and the repair need of it.
    fn record(&mut self, line: Line, board: Option<KeptBoard>, on_game: &mut impl FnMut(&Line)) -> Result<(), String> {
        let n = line.n;
        let json = line.to_json();
        self.lines.append(n, &json, stored_of(&line, &json))?;
        on_game(&line);
        self.summary.solved += 1;
        if let Some(setup) = line.setup {
            self.earlier.add(n, setup.size(), setup.digest, line.promotions.clone(), ());
        }
        self.recent.insert(n, Moves { n, moves: line.moves, score: line.score, source: Source::Run });
        self.recent.retain(|&m, _| m + MAX_BACK > n);
        self.mine.insert(n);
        self.boards.keep(n, board);
        Ok(())
    }

    /// The repair's walk down (the module documentation, "The repair") from the lines of the games `changed`: the line
    /// this run has just recorded, or where a walk was when a stop cut it short, as its file says.  Each game below
    /// them, from the highest down, is weighed against the lines of `changed` up to [`REPAIR_REACH`] above it
    /// ([`weigh`](Self::weigh)), and a game whose line is replaced joins them, with the prime above it played
    /// again from it ([`prime_after`](Self::prime_after)).  The walk ends at the first game that none of `changed` is
    /// that near, or at a stop signal, which leaves it to the next run that opens the file.
    fn walk(&mut self, changed: BTreeSet<u32>, on_repair: &mut impl FnMut(&Repair)) -> Result<(), String> {
        let began = Instant::now();
        let walked = self.walk_from(changed, on_repair);
        self.summary.repair_time += began.elapsed();
        walked
    }

    /// The walk of [`walk`](Self::walk), untimed.
    fn walk_from(&mut self, mut changed: BTreeSet<u32>, on_repair: &mut impl FnMut(&Repair)) -> Result<(), String> {
        let Some(&lowest) = changed.first() else { return Ok(()) };
        // A walk that a stop cut short may not have played a prime again from the line below it.
        for k in changed.clone() {
            self.prime_after(k, &mut changed, on_repair)?;
        }
        for k in (1..lowest).rev() {
            let sources: Vec<u32> = changed.range(k + 1..=k + REPAIR_REACH).copied().collect();
            if sources.is_empty() || engine::stop_signal().is_some() {
                break;
            }
            if self.weigh(k, &sources, on_repair)? {
                changed.insert(k);
                self.prime_after(k, &mut changed, on_repair)?;
            }
        }
        Ok(())
    }

    /// Replaces the line of game `k` if what survives on its board of the line of one of the games `sources`, above
    /// it, scores more: the heaviest, the nearest on a tie.  A game that neither the run nor its file has a line for
    /// (the built-in table's, which are proven, stay as they are), and a prime or trivial line, are not weighed: a
    /// prime follows from the line below it, and a trivial line is optimal.  Returns whether it replaced the line.
    fn weigh(&mut self, k: u32, sources: &[u32], on_repair: &mut impl FnMut(&Repair)) -> Result<bool, String> {
        let Some(line) = self.lines.games.get(&k) else { return Ok(false) };
        if matches!(line.method, Method::Prime | Method::Trivial) {
            return Ok(false);
        }
        let began = Instant::now();
        let Some(board) = self.boards.board(k)? else { return Ok(false) };
        let mut best: Option<(u32, Vec<u32>, u64)> = None;
        for &j in sources {
            let promotions = board.job.surviving(&self.lines.games[&j].moves);
            let sum = promotions.iter().map(|&p| u64::from(p)).sum();
            if best.as_ref().is_none_or(|&(_, _, most)| sum > most) {
                best = Some((j, promotions, sum));
            }
        }
        let line = &self.lines.games[&k];
        let Some((from, promotions, sum)) = best.filter(|&(_, _, sum)| board.in_the_bag + sum > line.score) else {
            return Ok(false);
        };
        let moves = board.job.moves_for(&promotions);
        let (method, was, score) = (line.method, line.score, board.in_the_bag + sum);
        let seconds = began.elapsed().as_secs_f64();
        self.replace(Repair { n: k, from, method, was, score, moves, promotions, seconds }, None, on_repair)?;
        Ok(true)
    }

    /// Plays the prime game k + 1 again from the line of game `k`, if the run or its file has a prime line of it and
    /// the prime formula on the line of `k` scores more, as it does once that line is repaired; it joins `changed` if
    /// so.
    fn prime_after(
        &mut self,
        k: u32,
        changed: &mut BTreeSet<u32>,
        on_repair: &mut impl FnMut(&Repair),
    ) -> Result<(), String> {
        let p = k + 1;
        let Some(line) = self.lines.games.get(&p).filter(|line| line.method == Method::Prime) else { return Ok(()) };
        let began = Instant::now();
        let SetUp::Derived(Derived::Prime { candidates }) =
            set_up(p, &Before::Run, self.opts).map_err(Failure::message)?
        else {
            return Ok(());
        };
        let below = &self.lines.games[&k];
        let (moves, promotions) = prime_game(p, &candidates, &below.moves);
        let score = replay(p, &moves).map_err(|e| format!("n={p}: the prime moves from the repaired {k}: {e}"))?;
        if score <= line.score {
            return Ok(());
        }
        let source = if self.mine.contains(&k) { Source::Run } else { Source::Results };
        let prev = Moves { n: k, moves: below.moves.clone(), score: below.score, source };
        let (was, seconds) = (line.score, began.elapsed().as_secs_f64());
        let repair = Repair { n: p, from: k, method: Method::Prime, was, score, moves, promotions, seconds };
        self.replace(repair, Some(&prev), on_repair)?;
        changed.insert(p);
        Ok(())
    }

    /// Records `repair`: checks its moves as those of a game played are checked (they replay to its score, which is no
    /// more than the built-in table's) and that they score more than the line they replace, appends the line again with
    /// them ([`repaired_line`], with `prev` for a prime played again), tells the repeats and the games that follow of
    /// them, and hands the repair to `on_repair`.
    fn replace(
        &mut self,
        repair: Repair,
        prev: Option<&Moves>,
        on_repair: &mut impl FnMut(&Repair),
    ) -> Result<(), String> {
        let n = repair.n;
        let fail = |e: String| format!("n={n}: the repair from {}: {e}", repair.from);
        let score = replay(n, &repair.moves).map_err(fail)?;
        let old = &self.lines.games[&n];
        if score != repair.score || score <= old.score {
            return Err(fail(format!("its moves score {score}, and the line it replaces {}", old.score)));
        }
        if let Some(best) = self.optimal.score(n).filter(|&best| score > best) {
            return Err(fail(format!("its moves score {score}, above the built-in table's {best}, which is proven")));
        }
        let json = repaired_line(&old.text, &repair, prev)?;
        let (method, problem) = (old.method, old.problem);
        let (moves, promotions, text) = (repair.moves.clone(), repair.promotions.clone(), json.to_string());
        self.lines.append(n, &json, Stored { score, moves, promotions, method, problem, text })?;
        if let Some((size, digest)) = problem {
            self.earlier.add(n, size, digest, repair.promotions.clone(), ());
        }
        if let Some(recent) = self.recent.get_mut(&n) {
            (recent.moves, recent.score) = (repair.moves.clone(), score);
        }
        self.mine.insert(n);
        self.summary.repaired += 1;
        on_repair(&repair);
        Ok(())
    }
}

/// The moves for the games `n − 1`, `n − 2`, ... that game `n` starts from and begins its trajectories from, as many as
/// `plan.back` and at least the first, from game `n − 1` down: each is, with `--from-known`, the built-in table's,
/// else this run's line of the game (`run`), else the file's line, else the table's; none for a game below 1, or that
/// nothing has the moves for.  A game that lacks the moves for `n − 1` has a note on stderr.
fn moves_before(
    n: u32,
    run: &BTreeMap<u32, Moves>,
    file: &UnsoundFile,
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
            let (moves, score) = file.moves(game)?;
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

/// The moves for the games `n + 1`, ..., `n + plan.above` that game `n` begins trajectories from: each is, with
/// `--from-known`, the built-in table's, else this run's line of the game (`run`), else the file's line; a game that
/// none of them has gives none.  Without `--from-known` the table gives none: a run never takes the proven game above
/// one it plays.
fn moves_above(n: u32, run: &BTreeMap<u32, Moves>, file: &UnsoundFile, plan: &Plan, optimal: &Optimal) -> Vec<Moves> {
    let moves_of = |game: u32| {
        if plan.from_known {
            let (moves, score) = (optimal.moves(game)?, optimal.score(game)?);
            return Some(Moves { n: game, moves: moves.to_vec(), score, source: Source::Table });
        }
        let file = || {
            let (moves, score) = file.moves(game)?;
            Some(Moves { n: game, moves: moves.to_vec(), score, source: Source::Results })
        };
        run.get(&game).cloned().or_else(file)
    };
    (1..=plan.above).filter_map(|j| n.checked_add(j).filter(|&game| game <= super::MAX_N).and_then(moves_of)).collect()
}

/// The boards of a run: the setups of the games after the one in play, each made on a thread of its own while that
/// game plays, and the boards of its last [`REPAIR_REACH`] games, which the repair weighs lines on.  A game's setup
/// ([`set_up`]) depends on the game, on whether it has moves for n − 1, and on the options, and not on the moves
/// themselves, so it can be made before the game before it ends; with it goes the dive of a searched game's start
/// (`Job::ready`), which depends on the board alone.  A board is set up once: the game takes the setup made ahead, and
/// the run keeps its board for the repair.
struct Boards {
    opts: SolveOptions,
    /// The games whose setups are being made, with moves for n − 1, and the threads that make them.
    making: Vec<(u32, thread::JoinHandle<Result<SetUp, Failure>>)>,
    /// The boards of the run's searched and repeat games, of the last [`REPAIR_REACH`] games it played.
    kept: BTreeMap<u32, KeptBoard>,
}

impl Boards {
    /// No setup made ahead and no board kept yet, for a run with `opts`.
    fn new(opts: &SolveOptions) -> Boards {
        Boards { opts: opts.clone(), making: Vec::new(), kept: BTreeMap::new() }
    }

    /// Starts making the setup of game `n`, which will have moves for n − 1.
    fn start(&mut self, n: u32) {
        let opts = self.opts.clone();
        let make = move || {
            let set = set_up(n, &Before::Run, &opts)?;
            if let SetUp::Search(set, _) = &set {
                set.job.ready();
            }
            Ok(set)
        };
        self.making.push((n, thread::spawn(make)));
    }

    /// The setup of game `n`, with `given` for its moves for n − 1: the one made ahead if there is one and the game has
    /// those moves, else one made now with `opts`.
    fn take(&mut self, n: u32, given: &Before, opts: &SolveOptions) -> Result<SetUp, Failure> {
        let made = self.making.iter().position(|&(m, _)| m == n).map(|i| self.making.swap_remove(i).1);
        match made {
            Some(making) if matches!(given, Before::Run) => {
                making.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            }
            Some(making) => {
                let _ = making.join();
                set_up(n, given, opts)
            }
            None => set_up(n, given, opts),
        }
    }

    /// Keeps `board`, the board of game `n`, which the run has just played, and lets go of those more than
    /// [`REPAIR_REACH`] games below it.
    fn keep(&mut self, n: u32, board: Option<KeptBoard>) {
        if let Some(board) = board {
            self.kept.insert(n, board);
        }
        self.kept.retain(|&m, _| m + REPAIR_REACH >= n);
    }

    /// The board of game `k` for the repair: the one kept, else set up again; `None` for a board with no candidates.
    fn board(&self, k: u32) -> Result<Option<KeptBoard>, String> {
        if let Some(board) = self.kept.get(&k) {
            return Ok(Some(board.clone()));
        }
        Ok(match set_up(k, &Before::None, &self.opts).map_err(Failure::message)? {
            SetUp::Search(set, _) => Some(KeptBoard { job: set.job, in_the_bag: set.in_the_bag }),
            SetUp::Derived(_) => None,
        })
    }
}

/// A run that ends waits for the setups it started, so that no thread of it outlives it.
impl Drop for Boards {
    fn drop(&mut self) {
        for (_, making) in self.making.drain(..) {
            let _ = making.join();
        }
    }
}

/// Game `n` played in the unsound mode, from the moves for the games before it, `past` ([`moves_before`]: the first is
/// the moves for `n − 1`, which the start is seeded with, and the others begin trajectories if `plan.back` asks; see
/// the module documentation, "One game"), and for the games above it that `above` has ([`moves_above`]), which begin
/// trajectories too; and its line, checked, with the board it set up if it searched or repeated, which the repair may
/// weigh lines on; `None` if a stop signal ended its rounds.  `earlier` has the lines a repeat may repeat, and is told
/// of the problem of this game's board.  The setup is the one `boards` made ahead, if it made this game's.  An error
/// names the game.
#[allow(clippy::too_many_arguments)]
fn play(
    n: u32,
    past: &[Option<Moves>],
    above: &[Moves],
    opts: &SolveOptions,
    plan: &Plan,
    earlier: &mut Earlier<()>,
    boards: &mut Boards,
    optimal: &Optimal,
) -> Result<Option<(Line, Option<KeptBoard>)>, String> {
    let began = Instant::now();
    let clock = || began.elapsed().as_secs_f64();
    let before = past.first().and_then(Option::as_ref);
    // The moves for n − 1 are a seed and the prime shortcut's moves, which nothing checks again: the run's and the
    // file's lines were replayed before they were kept, and the table's are proven.
    let given = if before.is_some() { Before::Run } else { Before::None };
    let ceiling = before.filter(|_| !plan.past_ceiling).map(|before| ceiling_over(n, before));
    let mut line = Line {
        n,
        score: 0,
        moves: Vec::new(),
        promotions: Vec::new(),
        method: Method::Search,
        in_the_bag: None,
        prev: None,
        ceiling,
        at_ceiling: false,
        repeats: None,
        setup: None,
        plan: *plan,
        start: None,
        trajectories: None,
        survivor_scores: None,
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
    let mut board = None;
    match boards.take(n, &given, opts).map_err(Failure::message)? {
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
            board = Some(KeptBoard { job: Arc::clone(&set.job), in_the_bag: set.in_the_bag });
            if let Some(m) = repeated {
                let promotions = earlier.promotions(m).expect("a game that find found").to_vec();
                (line.method, line.repeats) = (Method::Repeat, Some(m));
                line.moves = set.job.moves_for(&promotions);
                line.promotions = promotions;
            } else {
                let bag = set.in_the_bag;
                // The rounds end at the table's score (`--stop-at-known`) or the run's ceiling, whichever comes first.
                let enough = |sum: u64| {
                    let score = bag + sum;
                    known.is_some_and(|known| score >= known) || ceiling.is_some_and(|ceiling| score >= ceiling)
                };
                let seed = before.map(|before| before.moves.as_slice());
                // The games a trajectory may begin from, from n − 1 down, then from n + 1 up.
                let back = past.iter().take(plan.back as usize).flatten();
                let others: Vec<(u32, &[u32])> = back.chain(above).map(|m| (m.n, m.moves.as_slice())).collect();
                let improved = set.job.improve(seed, &others, plan, opts.threads, &clock, &enough);
                if improved.interrupted {
                    return Ok(None);
                }
                line.start = Some(bag + improved.start);
                line.start_seconds = improved.start_seconds;
                line.trajectories = Some(improved.trajectories);
                line.survivor_scores = Some(improved.survivors.iter().map(|&(game, sum)| (game, bag + sum)).collect());
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
    Ok(Some((line, board)))
}

/// The run's ceiling on the score of game `n` that starts from the moves `before` for n − 1: n + their score, which no
/// play of game n exceeds if they are optimal (the module documentation, "One game").
fn ceiling_over(n: u32, before: &Moves) -> u64 {
    before.score + u64::from(n)
}

/// The checks of the moves of `line` before they count (the module documentation, "One game"), and what they give the
/// line: its score, whether that is at its ceiling, its record of the moves for n − 1 it started from, `before`, and
/// what `--stop-at-known` recorded, if it was given (`known`, the table's score).
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
    line.at_ceiling = line.ceiling.is_some_and(|ceiling| score >= ceiling);
    line.prev = before.map(|before| Previous {
        n: n - 1,
        score: before.score,
        source: before.source,
        moves_digest: moves_digest(&before.moves),
        short_by: score.saturating_sub(ceiling_over(n, before)),
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
        let budgets: Vec<u64> = (MIN_EFFORT..=MAX_EFFORT).map(|effort| plan(effort).budget()).collect();
        assert_eq!(budgets, [131_072, 262_144, 524_288, 1_048_576, 2_097_152, 4_194_304]);
        assert_eq!(ROUND_WORK, 65_536, "one round of 64 neighborhoods of 1,024 steps");
        // The most effort budgets 64 rounds of work: 2^22, far below what a count of work holds.
        assert_eq!(plan(MAX_EFFORT).budget(), ROUND_WORK << 6);
        assert_eq!(plan(MAX_EFFORT).budget(), 1 << 22);
        // A work of a round that no flag can give still cannot overflow.
        let huge = Plan { effort: MAX_EFFORT, round_work: u64::MAX, ..Plan::default() };
        assert_eq!(huge.budget(), u64::MAX);
    }

    #[test]
    fn the_default_plan_tilts_goes_back_and_widens_as_the_races_chose() {
        let plan = Plan::default();
        assert_eq!((DEFAULT_TILT, DEFAULT_BACK, DEFAULT_WIDEN), (25, 8, 4));
        assert_eq!((plan.effort, plan.seed, plan.fix), (MIN_EFFORT, 1, (0.7, 0.9)));
        assert_eq!((plan.tilt, plan.back, plan.widen, plan.round_work), (25, 8, Some(4), ROUND_WORK));
        assert!(!plan.stop_at_known && !plan.from_known && !plan.past_ceiling, "the stop at the ceiling is on");
        assert!(plan.repair && plan.above == 0, "the repair is on, and no trajectory begins from above");
        // The rounds search with the plan's seed, tilt and widening, and the finder's other options.
        let lns = plan.lns();
        let default = finder::Options::default();
        assert_eq!((lns.fix, lns.unit_nodes, lns.seed), (default.fix, default.unit_nodes, default.seed));
        assert_eq!((lns.tilt, lns.widen), (25, Some(4)));
        assert_eq!(check_plan(&plan), Ok(()));
        // The finder beside a proof search never tilts or widens, whatever the defaults of the unsound mode are.
        assert!(default.tilt == 0 && default.widen.is_none());
        // Each is turned off with 0.
        let off = Plan { tilt: 0, back: 0, widen: None, ..plan };
        assert_eq!((off.lns().tilt, off.lns().widen, check_plan(&off)), (0, None, Ok(())));
    }

    #[test]
    fn a_fix_turns_widening_off_and_is_refused_with_a_widening() {
        assert_eq!(widening(false, None), Ok(Some(DEFAULT_WIDEN)));
        assert_eq!(widening(false, Some(Some(2))), Ok(Some(2)));
        assert_eq!(widening(false, Some(None)), Ok(None), "--widen 0 turns it off");
        assert_eq!(widening(true, None), Ok(None), "--fix alone turns it off");
        assert_eq!(widening(true, Some(None)), Ok(None), "--fix with --widen 0 is the same");
        for widen in [1, 4, 30] {
            assert_eq!(widening(true, Some(Some(widen))), Err(WIDEN_WITH_FIX.to_owned()), "--fix with --widen {widen}");
        }
    }

    #[test]
    fn an_effort_is_a_whole_number_from_the_least_level_to_the_most() {
        assert_eq!((MIN_EFFORT, MAX_EFFORT), (1, 6));
        assert_eq!((parse_effort("1"), parse_effort("3"), parse_effort("6")), (Ok(1), Ok(3), Ok(6)));
        // The start alone is no level, and nothing above the most is.
        for text in ["0", "7", "32", "-1", "", "a", "1.5", "1,2", " 1", "99999999999"] {
            let err = parse_effort(text).unwrap_err();
            assert_eq!(err, format!("--effort takes L, the level of effort, a whole number from 1 to 6: got {text:?}"));
        }
    }

    #[test]
    fn a_tilt_is_a_whole_number_of_percent() {
        assert_eq!((parse_tilt("0"), parse_tilt("50"), parse_tilt("100")), (Ok(0), Ok(50), Ok(100)));
        for text in ["101", "-1", "", "a", "50.5", "5e1", "50%", " 50", "99999999999"] {
            let err = parse_tilt(text).unwrap_err();
            assert!(err.starts_with("--tilt takes P, a whole number of percent from 0 to 100"), "{text:?}: {err}");
        }
    }

    #[test]
    fn a_number_of_games_back_or_above_and_a_widening_are_whole_numbers_in_range() {
        assert_eq!((parse_back("0"), parse_back("1"), parse_back("8")), (Ok(0), Ok(1), Ok(8)));
        for text in ["9", "-1", "", "a", "1.5", "1,2", " 1", "99999999999"] {
            let err = parse_back(text).unwrap_err();
            assert!(err.starts_with("--back takes J, a whole number from 0 to 8"), "{text:?}: {err}");
            let err = parse_above(text).unwrap_err();
            assert!(err.starts_with("--above takes J, a whole number from 0 to 8"), "{text:?}: {err}");
        }
        assert_eq!((parse_above("0"), parse_above("4"), parse_above("8")), (Ok(0), Ok(4), Ok(8)));
        assert_eq!((parse_widen("1"), parse_widen("30")), (Ok(Some(1)), Ok(Some(30))));
        assert_eq!(parse_widen("0"), Ok(None), "0 is no widening");
        for text in ["-1", "", "a", "2.5", "1,2", " 3"] {
            let err = parse_widen(text).unwrap_err();
            assert!(
                err.starts_with("--widen takes S, a whole number of rounds, or 0 for no widening"),
                "{text:?}: {err}"
            );
        }
    }

    #[test]
    fn a_plan_with_an_option_out_of_range_is_refused() {
        let plan = Plan { effort: 3, back: 8, widen: Some(1), ..Plan::default() };
        assert_eq!(check_plan(&plan), Ok(()));
        let refused = |plan: Plan| check_plan(&plan).unwrap_err();
        for effort in [MIN_EFFORT - 1, MAX_EFFORT + 1] {
            let message =
                format!("--effort takes L, the level of effort, a whole number from 1 to 6: got \"{effort}\"");
            assert_eq!(refused(Plan { effort, ..plan }), message);
        }
        assert!(refused(Plan { fix: (0.9, 0.7), ..Plan { widen: None, ..plan } }).starts_with("--fix takes LO,HI"));
        assert!(refused(Plan { tilt: MAX_TILT + 1, ..plan }).starts_with("--tilt takes P"));
        assert!(refused(Plan { back: MAX_BACK + 1, ..plan }).starts_with("--back takes J"));
        assert!(refused(Plan { above: MAX_ABOVE + 1, ..plan }).starts_with("--above takes J"));
        assert!(refused(Plan { widen: Some(0), ..plan }).starts_with("the plan widens after 0 rounds"));
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
    fn a_repaired_line_is_the_line_played_with_the_repairs_moves_and_a_record_of_the_repair() {
        // A line of game 396 that started from moves for 395 scoring 60, with its moves cut short, and what the walk
        // made of it: what survived of the line of 398 scores 500, at the run's ceiling, 396 + 60, and 44 above it.
        let old = r#"{"unsound":true,"n":396,"score":100,"moves":[2,4],"promotions":[4],"method":"search",
            "in_the_bag":96,"prev":{"n":395,"score":60,"source":"results","moves_digest":"d","short_by":0},
            "ceiling":456,"at_ceiling":false,"rounds":3,"repaired":null,"build":{"commit":"x"}}"#;
        let (moves, promotions) = (vec![2, 4, 6], vec![4, 6]);
        let repair =
            Repair { n: 396, from: 398, method: Method::Search, was: 100, score: 500, moves, promotions, seconds: 0.5 };
        let line = repaired_line(old, &repair, None).unwrap();
        let old = Json::parse(old).unwrap();
        let keys = |json: &Json| json.as_obj().unwrap().iter().map(|(key, _)| key.clone()).collect::<Vec<_>>();
        assert_eq!(keys(&line), keys(&old), "the same keys, in the same order");
        for key in ["unsound", "n", "method", "in_the_bag", "ceiling", "rounds"] {
            assert_eq!(line.get(key), old.get(key), "{key} is the line played's");
        }
        assert_eq!(line.get("score"), Some(&Json::from(500u64)));
        assert_eq!(line.get("moves"), Some(&Json::array([2u32, 4, 6])));
        assert_eq!(line.get("promotions"), Some(&Json::array([4u32, 6])));
        assert_eq!(line.get("at_ceiling"), Some(&Json::Bool(true)));
        let prev = line.get("prev").unwrap();
        assert_eq!((prev.get("source"), prev.get("score")), (Some(&Json::from("results")), Some(&Json::from(60u64))));
        assert_eq!(prev.get("short_by"), Some(&Json::from(44u64)), "500 proves the moves for 395 short by 44");
        let repaired = line.get("repaired").unwrap();
        assert_eq!(repaired.get("from"), Some(&Json::from(398u32)));
        assert_eq!((repaired.get("was"), repaired.get("seconds")), (Some(&Json::from(100u64)), Some(&Json::Num(0.5))));
        assert_ne!(line.get("build"), old.get("build"), "the build is the one that repaired it");
        // A prime played again takes the moves it started from from the repaired line below it, and its ceiling.
        let below = Moves { n: 395, moves: vec![1, 2], score: 70, source: Source::Run };
        let line = repaired_line(&old.to_string(), &repair, Some(&below)).unwrap();
        let prev = line.get("prev").unwrap();
        assert_eq!((prev.get("score"), prev.get("source")), (Some(&Json::from(70u64)), Some(&Json::from("run"))));
        assert_eq!(prev.get("moves_digest"), Some(&Json::from(moves_digest(&[1, 2]).as_str())));
        assert_eq!((line.get("ceiling"), prev.get("short_by")), (Some(&Json::from(466u64)), Some(&Json::from(34u64))));
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
