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
//! Four settings change what the rounds search, and so the line.  Three are built in ([`BACK`], [`TILT`] and
//! [`WIDEN`]), and the plan's `above` is an option.  Every value keeps what the rounds guarantee: a line depends on the
//! game, its start, the moves it starts from, the level, the seed and `above` only, never on the threads, and a run at
//! L + 1 does the rounds of L first.
//!
//! * **`TILT`** P makes P percent of a round's neighborhoods, an equal share of each trajectory's, choose the
//!   promotions they free by their size, the small ones more often, with the same number freed; the others draw
//!   uniformly ([`Options::tilt`](crate::search::finder::Options::tilt), [`draws_tilted`](finder::draws_tilted)).
//! * **`BACK`** J gives a game a trajectory of incumbents for each distinct set that survives of the moves for the
//!   games n − 1, n − 2, ..., n − J ([`seed_survivors`](crate::search::seed_survivors)), as the finder beside a proof
//!   search starts its seed rounds, beside the first, the start's.  A set that an earlier trajectory (the start's
//!   included) began from is skipped, and so is a game whose moves nothing has.  Neighborhood k of a round works from
//!   the incumbent of trajectory k mod T, T the trajectories, and the game's set is the heaviest incumbent, the
//!   start's on a tie ([`Lns::rounds`](crate::search::finder::Lns::rounds)).  The line says which games the
//!   trajectories began from (`trajectories`).  What survives of an earlier game may be heavier than the start; the
//!   game's score is then that of the survivors from the first, which the line records as an improvement of round 0.
//!   The line records the score of what survives of each of these games, whether or not it began a trajectory
//!   (`survivor_scores`).
//! * **`above`** J (0 by default) adds, after those of `BACK`, a trajectory for each distinct set that survives of the
//!   moves for the games n + 1, ..., n + J that the run or its files have: the moves above n are dropped
//!   ([`Board::seed`](crate::board::Board::seed)).  A game played in order has no line above it in its own run, so
//!   this is for a run over a range whose files already hold lines above it.
//! * **`WIDEN`** S moves the neighborhoods of a trajectory whose incumbent has not grown heavier in S rounds down the
//!   ladder of the share kept fixed ([`LADDER`](crate::search::finder::LADDER)), and back to its first stage when it
//!   grows ([`Options::widen`](crate::search::finder::Options::widen)).  The ladder's first stage is the finder's
//!   share fixed.  The line records the rounds each trajectory spent at each stage (`stage_rounds`) and the stage of
//!   each improvement.
//!
//! # The built-in settings
//!
//! The user names a level of effort and nothing else: `BACK` is 8, `TILT` 25 and `WIDEN` 4, the setting that two races
//! chose on the 251 hard games of 3441-5060, and `STOP_QUIET` and `PUSH_GROWTH` ("The bound and the gap", below) are 4
//! and 25.  solver/docs/unsound/play.md ("The options that shape the rounds") has the races and their numbers, and
//! solver/docs/unsound/shortfalls.md ("The bound and the gap") those of the stop and the push.  The finder beside a
//! proof search never tilts, widens or goes back, and its options ([`finder::Options::default`]) are not these.
//!
//! # A range
//!
//! [`solve_range`] plays the games lo..=hi in order, one at a time, on all the threads given.  The moves for n − j that
//! a game starts from, for j = 1 and the j up to [`BACK`], are this run's line of n − j, else the file's, or the
//! best-known file's if that scores more ("The best-known file"), else the built-in table's (as a seed only, never a
//! line).  Each line is appended to the file and synced as soon as its game ends, and a restarted run skips the games
//! the file has.  A stop signal (SIGINT, SIGTERM) ends the run between rounds, and the game in play is not recorded.
//! The file holds unsound lines only: [`UnsoundFile::open`] refuses a line of a proven run, as every reader of proven
//! lines refuses an unsound one ([`refuse_unsound`](crate::game::refuse_unsound)).
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
//! replaced.  What survives is the set a trajectory of `BACK` begins from
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
//! * **The work.**  The repair runs no rounds: the work of a level is that of its rounds.  Its time is that of the
//!   survivors on the boards kept, small beside a game's, and of the boards it sets up again, a few seconds each at
//!   n ≈ 5,000: a walk past the boards kept needs them, and so does a walk that a run takes up from its file.  The
//!   run's summary has the time of its walks ([`RangeSummary::repair_time`](super::RangeSummary::repair_time)).
//! * **What a line depends on.**  The walk's steps depend on the lines alone, never on the threads, so a line still
//!   depends on nothing but the game, its start, the moves it starts from, the level, the seed and `above`.  The rounds
//!   of a game at E + 1 begin with those at E from the same moves; in a range the repair can change the moves that later
//!   games start from, as a level does.
//!
//! # Games played twice
//!
//! The repair can pull a short line up only from a line above it that does better, and in a run the lines above a
//! short game are made from it: they start from it, copy it or stop at the ceiling it sets.  A game that takes nothing
//! from the games below escapes that.  So the push `PUSH_GROWTH` T plays a game whose gap grew by more than T twice
//! ("The bound and the gap"): as any game is played, then *unseeded*, searched on the same board at the game's level
//! with no seed, no trajectory of `BACK` and no run's ceiling (the trajectories of `above` as given).  The line is the
//! better play's, the seeded one on a tie, with what the other came to (`other`), so the games above start from it and
//! the repair walks down from it as from any line.  A prime has no board, and its unseeded play takes the one set up
//! for its bound; a trivial game is played once.
//!
//! * **The work.**  The unseeded play runs the rounds of its level to the budget, as a game does, so a game played
//!   twice does up to twice a level's work; with no ceiling, its rounds stop only at the budget.  The line's `work` is
//!   its own play's and `other` has the other's.
//! * **What a line depends on.**  Which games are played twice depends on the growth of the gap, which depends on n and
//!   the moves for n − 1, and each play on what a game's line depends on, so a line still depends on nothing but the
//!   game, its start, the moves it starts from, the level, the seed and `above`, never on the threads.  Each play of a
//!   level runs the rounds of the levels below first.
//! * **Where the push comes from.**  solver/docs/unsound/shortfalls.md ("Games played twice") has the measurements.
//!
//! # The bound and the gap
//!
//! A game's *bound* is `in_the_bag` plus the GF(2) bound of the root's node ([`Job::bound`], the frame bound (B2) of
//! docs/theory/frame-bounds.md), which no play of game n exceeds: a score above it is a hard error, as one above the
//! table's is.  It is the parity bound of n, which the prover computes on the whole board (the paragraph "Against the
//! parity bound" of docs/theory/frame-bounds.md).  A board with no promotion candidates has the bound `in_the_bag`,
//! and a prime, which the prime formula plays with no board, a board set up for its bound alone.  The setup that a run
//! makes while the game before plays makes the bound too ([`Ready`]).
//!
//! A line's *gap* is its bound less its score, and its *growth* the gap less the gap of the moves for n − 1 that it
//! starts from (`growth`).  On the proven games the gap does not grow four times in five, and a short game makes it
//! jump at the game where a streak of short games begins, while a game that a short game hands its shortfall on to
//! carries the gap on, with a growth of about 0.  The bound and the growth depend on n and the moves for n − 1 alone.
//!
//! * **The stop on growth** (`stop_growth` T, [`STOP_QUIET`] K): a searched game's rounds end once its gap is within T
//!   of the gap below, its growth from −T to T, and the last K rounds raised nothing, asked after each round from the
//!   one that reaches the budget of level 1 ([`Plan::stop_floor`]) on.  Most games reach their score in level 1's
//!   rounds, and a gap that stayed as it was says that they are probably done.  A gap that shrank by more than T says
//!   that the game below may be short, or that this one has more to give, so the rounds go on.  They go on too while
//!   the game still gains, since a game that gained late went on to gain again: the stop waits until K rounds in a row
//!   have raised nothing ([`Rounds::quiet`](finder::Rounds::quiet) counts them, the start being round 0).  The floor
//!   keeps the rounds of level 1 whole, so a line at level 1 is as without the stop, and a level still runs the rounds
//!   of the levels below it first: the stop is asked of the same rounds at every level, with the same count of quiet
//!   rounds, which the rounds in their order give.  The line says whether it ended the rounds (`growth_stop`) and with
//!   how many quiet rounds (`quiet_rounds`).  `--stop-growth off` is `stop_growth` of `None`.
//! * **The push on growth** ([`PUSH_GROWTH`] T): a game whose growth after its play is above T is played twice
//!   ("Games played twice"), and keeps the better line (`pushed`).  A gap that jumps says that the game is probably
//!   short, and an unseeded play is the play that a short line below cannot hold down.  Its unseeded play counts
//!   against a budget of its own, the level's.  A game played twice at a lower level may keep its unseeded play where a
//!   higher level, whose seeded play brings the growth to T or below, plays it once and scores less: the push breaks
//!   the rule that a higher level scores at least as much, which the seeded play keeps.
//! * **What a line depends on.**  The bound is a function of n, and the growth of n and of the moves for n − 1, so a
//!   line still depends on nothing but the game, its start, the moves it starts from, the level, the seed and `above`,
//!   never on the threads.
//! * **The built-in values**, [`DEFAULT_STOP_GROWTH`], [`STOP_QUIET`] and [`PUSH_GROWTH`], and the measurements that
//!   chose them are in solver/docs/unsound/shortfalls.md ("The bound and the gap").
//!
//! # The best-known file
//!
//! A run keeps a *best-known file* ([`BestKnownFile`]): the best-known line of each game past the built-in table, the
//! best line any run found, at any level, which later runs start from and add to.  `--best-known FILE` names it, by
//! default it is `best-known.jsonl` in the directory of `--out` ([`best_known_path`]), and `--best-known off` turns it
//! off.  Its lines are unsound lines with a key `best_known` ([`BEST_KNOWN`]), and none is proven.  A game of the
//! built-in table, whose table line is its best-known line, is never in it; with `--known-to N` the table is the one
//! cut at N, so the file may hold the games above N and refuses those up to N, and a file made so is for runs with the
//! same N.  solver/docs/unsound/best-known.md is the user's guide to it.
//!
//! * **Seeding.**  Where a game takes the moves for the games below it and above it ([`moves_before`],
//!   [`moves_above`]), it takes, of the run's line (its own, else its file's) and the best-known file's, the one that
//!   scores more, the run's on a tie, and then the table.  The line says so: `prev.source` is `best_known`.  Before
//!   each game the run reads what the other holders appended since it last read the file, so a game starts from what
//!   the others found while it ran.
//! * **Offering.**  Each line the run records, a game's or a repair's, is offered to the best-known file, and appended
//!   if it scores more than the best-known file's line of its game.  A run that opens its file offers the file's last
//!   line of each game first, so a run over the range of an old file, with a copy of the file as `--out`, plays nothing
//!   and offers all it has.
//! * **The repair.**  The walk down weighs the best-known file's lines of the games below the run's first, which the
//!   run has no line of, against the run's lines within [`REPAIR_REACH`] above them, with the code that weighs the
//!   run's own lines ([`Run::weigh`]), and appends a repair of one to the best-known file alone, as a line with
//!   `best_known.was` and `repaired`.
//! * **Sharing.**  Several runs may use one best-known file at once: the lock is held to read what is new and to
//!   append, never for a run's life.  So a line depends on the moves it started from, which depend on what the other
//!   runs had found when its game began, and its `prev` (the source and the digest of the moves) says what that was.  A
//!   run that must not depend on the others, such as one of an experiment that compares settings, uses
//!   `--best-known off`, or a best-known file of its own.
//!
//! # Rehearsing
//!
//! `--known-to N` is for a rehearsal on games whose opt(n) is proven: the run takes the built-in table as holding the
//! games up to N only ([`Optimal::up_to`], once, in [`solve_range`]), so a game above N is as if unproven.  No game
//! takes the table's moves for a game above N, the best-known file may hold the games above N and refuses those up to
//! N, and the checks against the table apply up to N.  The line records N (`known_to`).  solver/docs/unsound/line.md
//! has the keys of the line ("An unsound line").

use super::{
    prime_game, set_up, Before, Derived, Earlier, Failure, Job, Method, RangeSummary, SearchSetUp, SearchSetup, SetUp,
    SolveOptions,
};
use crate::bits::Bits;
use crate::engine;
use crate::game::{replay, Optimal, UNSOUND};
use crate::json::Json;
use crate::results::{append_line, build_json, each_line, mend, moves_digest, open_locked, Fields};
use crate::search::finder::{self, Improvement, NEIGHBORHOODS, STAGES};
use crate::search::{Problem, ProblemSize};
use crate::tactics::Tactics;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

mod best_known;

pub use best_known::{BestKnownFile, BEST_KNOWN};

/// The name of the best-known file that a run has by default, in the directory of its `--out` file
/// ([`best_known_path`]).
pub const DEFAULT_BEST_KNOWN: &str = "best-known.jsonl";

/// How many lines a run offers to the best-known file at a time, when it offers those of its file: the lock is held for
/// the lines of one offer.
const OFFER_CHUNK: usize = 32;

/// The least effort a run may ask for: a budget of 2 rounds of work.  The start alone is no level, since it never
/// repairs a shortfall.
pub const MIN_EFFORT: u32 = 1;

/// The most effort a run may ask for: a budget of 64 rounds of work, which is 2^22 with the default.
pub const MAX_EFFORT: u32 = 6;

/// The games above a game that its trajectories may begin from: `above` goes up to this.
pub const MAX_ABOVE: u32 = 8;

/// How far below a line the repair looks: what survives of a line is set against the lines of the games up to this
/// many below it (the module documentation, "The repair").
pub const REPAIR_REACH: u32 = 8;

/// The games before a game that its trajectories begin from: one from each distinct set that survives of the moves for
/// the 8 games before.  The races that chose it, with [`TILT`] and [`WIDEN`], are in the module documentation, "The
/// built-in settings".
pub const BACK: u32 = 8;

/// The percent of a round's neighborhoods that draw tilted: 25 (chosen with [`BACK`] and [`WIDEN`]; the module
/// documentation, "The built-in settings", has the races).
pub const TILT: u32 = 25;

/// The rounds without growth after which a trajectory moves down the ladder: 4 (chosen with [`BACK`] and [`TILT`]; the
/// module documentation, "The built-in settings", has the races).
pub const WIDEN: u64 = 4;

/// The default of `stop_growth`: a searched game's rounds end once its gap is within 25 of the gap below, from the end
/// of the rounds of level 1 on (the module documentation, "The bound and the gap", says how it was chosen).
pub const DEFAULT_STOP_GROWTH: Option<u64> = Some(25);

/// The quiet rounds that the stop on growth waits for: the last 4 rounds raised nothing (the module documentation,
/// "The bound and the gap", says how it was chosen).
pub const STOP_QUIET: u64 = 4;

/// The growth of the gap above which a game is played twice: 25 (the module documentation, "The bound and the gap",
/// says how it was chosen).
pub const PUSH_GROWTH: i64 = 25;

/// The work of one round, which level L budgets 2^L times ([`Plan::budget`]): the steps of a round whose neighborhoods
/// all take the step limit of a unit, [`finder::UNIT_STEPS`].  (The work of such a round is a little more,
/// [`finder::NEIGHBORHOOD_WORK`] for each neighborhood.)
pub const ROUND_WORK: u64 = NEIGHBORHOODS * finder::UNIT_STEPS;

/// How the unsound mode plays: the level of effort, the seed of the rounds' random choices, the games above that the
/// rounds may use, the cut of the built-in table, and the stop on growth.  The rest of how the rounds are shaped is
/// built in ([`BACK`], [`TILT`], [`WIDEN`], [`STOP_QUIET`], [`PUSH_GROWTH`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    /// From [`MIN_EFFORT`] to [`MAX_EFFORT`].
    pub effort: u32,
    pub seed: u64,
    /// `--above J`, J from 0 to [`MAX_ABOVE`]: a game also follows a trajectory from each distinct set that survives of
    /// the moves for the games n + 1, ..., n + J that the run or its files have.  0 takes none.
    pub above: u32,
    /// The work of one round, which level L budgets 2^L times: [`ROUND_WORK`] unless a test or a measurement lowers it.
    /// No flag sets it, and a line records the budget it gave (`budget`).
    pub round_work: u64,
    /// `--known-to N`: the run takes the table as holding the games up to N only ([`Optimal::up_to`]), and treats the
    /// games above N as unproven (the module documentation, "Rehearsing").  `None` takes the whole table.
    pub known_to: Option<u32>,
    /// `--stop-growth T` ([`DEFAULT_STOP_GROWTH`] by default): a searched game's rounds end once its growth is from −T
    /// to T and the last [`STOP_QUIET`] rounds raised nothing, asked after each round from the one that reaches the
    /// budget of level 1 on (the module documentation, "The bound and the gap").  `None` (`--stop-growth off`) ends no
    /// rounds this way.
    pub stop_growth: Option<u64>,
}

/// The least effort, the finder's seed, no trajectory from above, the whole built-in table, and the stop on growth
/// ([`DEFAULT_STOP_GROWTH`]).
impl Default for Plan {
    fn default() -> Plan {
        Plan {
            effort: MIN_EFFORT,
            seed: finder::Options::default().seed,
            above: 0,
            round_work: ROUND_WORK,
            known_to: None,
            stop_growth: DEFAULT_STOP_GROWTH,
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

    /// The work from which on `stop_growth` ends a game's rounds: the budget of level [`MIN_EFFORT`], so that a game at
    /// any level plays the rounds of the least level first and its line at the least level is as without the rule.
    pub fn stop_floor(&self) -> u64 {
        Plan { effort: MIN_EFFORT, ..*self }.budget()
    }

    /// How the rounds search: the finder's defaults, with this plan's seed, and the built-in tilt and widening
    /// ([`TILT`], [`WIDEN`]).
    pub fn lns(&self) -> finder::Options {
        finder::Options { seed: self.seed, tilt: TILT, widen: Some(WIDEN), ..finder::Options::default() }
    }
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

/// Reads the value of `--stop-growth`: T, a whole number of points from 0 up, as `Some(T)`, or `off`, `None`.
///
/// # Errors
///
/// What `text` should have been.
pub fn parse_stop_growth(text: &str) -> Result<Option<u64>, String> {
    if text == "off" {
        return Ok(None);
    }
    text.parse()
        .map(Some)
        .map_err(|_| format!("--stop-growth takes T, a whole number of points from 0 up, or off: got {text:?}"))
}

/// Checks the plan's options: an effort from [`MIN_EFFORT`] to [`MAX_EFFORT`], `above` of at most [`MAX_ABOVE`], and a
/// work of a round above 0.
///
/// # Errors
///
/// The first option that is out of range.
pub fn check_plan(plan: &Plan) -> Result<(), String> {
    if !(MIN_EFFORT..=MAX_EFFORT).contains(&plan.effort) {
        return Err(effort_error(&plan.effort.to_string()));
    }
    if plan.above > MAX_ABOVE {
        return Err(above_error(&plan.above.to_string()));
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
    /// The best-known file's line of n − 1, which scored more than this run's and its file's.
    BestKnown,
}

impl Source {
    /// The name in a line.
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Run => "run",
            Source::Results => "results",
            Source::Table => "table",
            Source::BestKnown => "best_known",
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

/// An unsound line: a game as the unsound mode played it (the module documentation; solver/docs/unsound/line.md, "An
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
    /// with no such moves.
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
    /// What each trajectory of a searched game's rounds began from: the start, then the games [`BACK`] and `--above`
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
    /// The rounds since the score last rose, when a searched game's rounds ended ([`finder::Rounds::quiet`]; 0 on any
    /// other line).  The line records it as `quiet_rounds` where the stop on growth ended the rounds.
    pub quiet: u64,
    /// For each trajectory, the rounds it worked at each stage of the ladder ([`WIDEN`]).
    pub stage_rounds: Option<Vec<[u64; STAGES]>>,
    /// Each round that raised the score, in order.
    pub improvements: Vec<Raised>,
    /// The game's wall time, from the end of the game before and of the repair's walk after it (or the start of the
    /// run) to its line, and the part of it before the rounds.  A setup made while the game before played (the module
    /// documentation, "A range") is not in it.
    pub seconds: f64,
    pub start_seconds: f64,
    pub threads: usize,
    pub tactics: Tactics,
    /// The line is of the unseeded play of a game played twice, which took nothing from the games below.
    pub unseeded: bool,
    /// For a game played twice ([`PUSH_GROWTH`]), the play that this line is not: the line is the better of the two,
    /// the seeded play on a tie.
    pub other: Option<OtherPlay>,
    /// The game's bound: `in_the_bag` plus the GF(2) bound of the root's node, which no play of the game exceeds (the
    /// module documentation, "The bound and the gap").
    pub bound: u64,
    /// The growth of the gap: the line's gap (`bound` less `score`) less the gap of the moves for n − 1 that the game
    /// started from, the run's line when it played the game (or the file's, or the table's); `None` if the run had no
    /// moves for n − 1.
    pub growth: Option<i64>,
    /// A searched game's rounds ended before the budget with its growth from −T to T, T the plan's `stop_growth`, and
    /// the last [`STOP_QUIET`] rounds raised nothing.
    pub growth_stop: bool,
    /// The game was played twice because its growth after the seeded play was above [`PUSH_GROWTH`].
    pub pushed: bool,
}

/// What the play that a line of a game played twice is not came to ([`Line::other`]): whether it was the unseeded one,
/// its method, its score, and its rounds, steps, work and seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OtherPlay {
    pub unseeded: bool,
    pub method: Method,
    pub score: u64,
    pub rounds: u64,
    pub steps: u64,
    pub work: u64,
    pub seconds: f64,
}

impl OtherPlay {
    /// What `line` came to.
    fn of(line: &Line) -> OtherPlay {
        let Line { unseeded, method, score, rounds, steps, work, seconds, .. } = *line;
        OtherPlay { unseeded, method, score, rounds, steps, work, seconds }
    }
}

impl Line {
    /// The line of game `n` before it is played, with `plan`, the threads and tactics of `opts`, `ceiling`, the run's
    /// ceiling, and `bound`, the game's bound.
    fn blank(n: u32, plan: &Plan, opts: &SolveOptions, ceiling: Option<u64>, bound: u64) -> Line {
        Line {
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
            quiet: 0,
            stage_rounds: None,
            improvements: Vec::new(),
            seconds: 0.0,
            start_seconds: 0.0,
            threads: opts.threads,
            tactics: opts.tactics,
            unseeded: false,
            other: None,
            bound,
            growth: None,
            growth_stop: false,
            pushed: false,
        }
    }

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
        let other = self.other.map_or(Json::Null, |other| {
            Json::object([
                ("unseeded", Json::Bool(other.unseeded)),
                ("method", Json::from(other.method.as_str())),
                ("score", Json::from(other.score)),
                ("rounds", Json::from(other.rounds)),
                ("steps", Json::from(other.steps)),
                ("work", Json::from(other.work)),
                ("seconds", Json::Num(other.seconds)),
            ])
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
            ("bound", Json::from(self.bound)),
            ("growth", self.growth.map_or(Json::Null, Json::from)),
            ("growth_stop", Json::Bool(self.growth_stop)),
            ("quiet_rounds", if self.growth_stop { Json::from(self.quiet) } else { Json::Null }),
            ("pushed", Json::Bool(self.pushed)),
            ("repeats", self.repeats.map_or(Json::Null, Json::from)),
            ("setup", setup),
            ("effort", Json::from(self.plan.effort)),
            ("budget", Json::from(self.plan.budget())),
            ("seed", Json::from(self.plan.seed)),
            ("above", Json::from(self.plan.above)),
            ("stop_growth", self.plan.stop_growth.map_or(Json::Null, Json::from)),
            ("unseeded", Json::Bool(self.unseeded)),
            ("start", self.start.map_or(Json::Null, Json::from)),
            ("trajectories", trajectories),
            ("survivor_scores", survivor_scores),
            ("rounds", Json::from(self.rounds)),
            ("neighborhoods", Json::from(self.rounds * NEIGHBORHOODS)),
            ("steps", Json::from(self.steps)),
            ("work", Json::from(self.work)),
            ("stage_rounds", stage_rounds),
            ("improvements", Json::array(improvements)),
            ("other", other),
            ("seconds", Json::Num(self.seconds)),
            ("start_seconds", Json::Num(self.start_seconds)),
            ("threads", Json::from(self.threads)),
            ("known_to", self.plan.known_to.map_or(Json::Null, Json::from)),
            // A line as a game is played; the repair writes it again with what replaced its moves ([`Repair`]).
            ("repaired", Json::Null),
            ("tactics", Json::from(self.tactics.description())),
            ("build", build_json()),
        ])
    }

    /// One line for a person: n, score, method, the rounds and how many raised the score, the time, `unsound`, and
    /// whether the game is at the run's ceiling and a line of n − 1 that this one proves short, if anything, the growth
    /// of the gap and whether it stopped the rounds, and for a game the push played twice what its unseeded play
    /// scored.
    pub fn summary_line(&self) -> String {
        let ceiling = if self.at_ceiling { "  at ceiling" } else { "" };
        let short = self
            .prev
            .as_ref()
            .filter(|prev| prev.short_by > 0)
            .map_or_else(String::new, |prev| format!("  n-1 short by {} at least", prev.short_by));
        let growth = match self.growth {
            None => String::new(),
            Some(growth) if self.growth_stop => format!("  growth {growth}, stopped"),
            Some(growth) => format!("  growth {growth}"),
        };
        let twice = match self.other {
            None => String::new(),
            Some(other) if self.unseeded => {
                format!("  pushed, unseeded play kept, +{} on the seeded", self.score - other.score)
            }
            Some(other) => format!("  pushed, unseeded play {}", other.score),
        };
        let notes = format!("{ceiling}{short}{growth}{twice}");
        format!(
            "n={:<5} score={:<9} {:<7} rounds={:<6} improved={:<3} {:>9.3} s  unsound{notes}",
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
    /// The line replaced is the best-known file's, and the run has none of the game: only the best-known file gets the
    /// repair.
    pub best_known_only: bool,
}

impl Repair {
    /// One line for a person, as [`Line::summary_line`] has for a game played: n, the new score, the method, the game
    /// the moves came from and what it raised the score by, the time, and "best-known file only" for a repair of the
    /// best-known file's line alone.
    pub fn summary_line(&self) -> String {
        let only = if self.best_known_only { "  best-known file only" } else { "" };
        format!(
            "n={:<5} score={:<9} {:<7} repaired from {}, +{} {:>9.3} s  unsound{only}",
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
    /// The rounds since the game's set last became heavier ([`finder::Rounds::quiet`]).
    pub(super) quiet: u64,
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
        mend(&mut file, &text, kept, 0).map_err(|e| at(e.to_string()))?;
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
/// repair's, the repair is recorded (`repaired`), `at_ceiling`, the short fall of the moves for n − 1 (`prev`) and the
/// growth of the gap are those of the new score, and `build` is this build.  `prev`, if given, replaces the moves for
/// n − 1 the line started from, and the run's ceiling with them: a prime is played again from the repaired line below
/// it.  The rest is the line as its game was played.
fn repaired_line(old: &str, repair: &Repair, prev: Option<&Moves>) -> Result<Json, String> {
    let old = Json::parse(old).map_err(|e| format!("n={}: the line to repair: {e}", repair.n))?;
    let f = Fields(&old);
    let ceiling = match (prev, f.opt_u64("ceiling")?) {
        (Some(prev), Some(_)) => Some(ceiling_over(repair.n, prev)),
        (_, ceiling) => ceiling,
    };
    // The growth falls by what the repair raised the score by, and for a prime played again, rises by what the moves
    // below gained, whose gap falls by as much.
    let gained = match (prev, f.get("prev")?) {
        (Some(prev), old @ Json::Obj(_)) => prev.score as i64 - Fields(old).u64("score")? as i64,
        _ => 0,
    };
    let raised = (repair.score - repair.was) as i64;
    let growth = old.get("growth").and_then(Json::as_i64).map(|growth| growth - raised + gained);
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
        ("growth", growth.map_or(Json::Null, Json::from)),
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

/// What a run reads and writes besides its games: the table of proven games, the file it appends its lines to (`--out`)
/// and its best-known file (`--best-known`, the module documentation, "The best-known file").
#[derive(Clone, Copy)]
pub struct Files<'a> {
    /// The table that the run checks its lines against, takes the moves for the games below its range from, and that
    /// says which games the best-known file never holds: [`Optimal::built_in`] for the command, and a table of fewer
    /// games for a test of a game the built-in table has.  The plan's `known_to` cuts it ([`Optimal::up_to`]).
    pub table: &'a Optimal,
    /// The file the run appends its lines to, if it has one.
    pub out: Option<&'a Path>,
    /// The best-known file the run seeds from and offers its lines to, if it has one.
    pub best_known: Option<&'a Path>,
}

impl<'a> Files<'a> {
    /// The built-in table with the files `out` and `best_known`.
    pub fn with_built_in(out: Option<&'a Path>, best_known: Option<&'a Path>) -> Files<'a> {
        Files { table: Optimal::built_in(), out, best_known }
    }
}

/// The best-known file of a run whose file is `out`, and whose `--best-known` is `given`: `off` has none, any other
/// value is the path, and without one it is [`DEFAULT_BEST_KNOWN`] in the directory of `out` (a run with no file has
/// none).
pub fn best_known_path(out: Option<&Path>, given: Option<&str>) -> Option<PathBuf> {
    match given {
        Some("off") => None,
        Some(path) => Some(PathBuf::from(path)),
        None => out.map(|out| out.with_file_name(DEFAULT_BEST_KNOWN)),
    }
}

/// Whether the paths `a` and `b` name the same file, which is the case where their directories and names are the same
/// (the files need not be there yet).
fn same_file(a: &Path, b: &Path) -> Result<bool, String> {
    let resolved = |path: &Path| {
        let dir = path.parent().filter(|dir| !dir.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        fs::canonicalize(dir)
            .map(|dir| dir.join(path.file_name().unwrap_or_default()))
            .map_err(|e| format!("{}: {e}", dir.display()))
    };
    Ok(resolved(a)? == resolved(b)?)
}

/// Plays the games `lo..=hi` in the unsound mode with `plan` (see the module documentation), on `opts.threads` threads
/// with `opts.tactics`, appending each line to the file `files.out`, if there is one, then handing it to `on_game`.
/// The games the file has are skipped.  Each line the run records is weighed against the lines below it, and each that
/// it replaces is appended and handed to `on_repair` (the module documentation, "The repair"); first, a walk down that
/// a stop cut short in the file goes on.  A run with a best-known file `files.best_known` seeds from it, offers it every
/// line of the file and every line it records, and repairs its lines of the games below the range (the module
/// documentation, "The best-known file").  A stop signal ends the range with the game in play unrecorded
/// (`stopped_at`).
///
/// # Errors
///
/// A range that is empty or starts below 1; a plan that [`check_plan`] refuses (an effort outside [`MIN_EFFORT`] to
/// [`MAX_EFFORT`], an `above` above [`MAX_ABOVE`], a work of a round of 0); the same file for `out` and `best_known`; a
/// file that cannot be opened or written, or holds a line it may not ([`UnsoundFile::open`], [`BestKnownFile::open`]);
/// and a game or a repair whose moves fail a check (the module documentation, "One game").
pub fn solve_range(
    lo: u32,
    hi: u32,
    opts: &SolveOptions,
    plan: &Plan,
    files: &Files,
    mut on_game: impl FnMut(&Line),
    mut on_repair: impl FnMut(&Repair),
) -> Result<RangeSummary, String> {
    if lo < 1 || lo > hi {
        return Err(format!("the range {lo}..={hi} is empty or starts below 1"));
    }
    check_plan(plan)?;
    // With `--known-to N` the run knows the table's games up to N only: every use of the table below is of the cut one.
    let cut = plan.known_to.map(|n| files.table.up_to(n));
    let optimal = cut.as_ref().unwrap_or(files.table);
    if let (Some(out), Some(best_known)) = (files.out, files.best_known) {
        if same_file(out, best_known)? {
            return Err(format!("{}: --out and --best-known are the same file", out.display()));
        }
    }
    let lines = files.out.map_or_else(|| Ok(UnsoundFile::none()), |path| UnsoundFile::open(path, optimal))?;
    let best_known = files.best_known.map(|path| BestKnownFile::open(path, optimal)).transpose()?;
    // The lines of the file that set a search up, whose problems a game may repeat.
    let mut earlier = Earlier::<()>::default();
    for (&n, game) in &lines.games {
        if let Some((size, digest)) = game.problem {
            earlier.add(n, size, digest, game.promotions.clone(), ());
        }
    }
    let name = files.out.and_then(Path::file_name).map(|name| name.to_string_lossy().into_owned());
    let mut run = Run {
        opts,
        optimal,
        lines,
        best_known,
        name,
        earlier,
        recent: BTreeMap::new(),
        bounds: BTreeMap::new(),
        mine: BTreeSet::new(),
        boards: Boards::new(opts),
        summary: RangeSummary::default(),
    };
    run.offer_file()?;
    let walk = run.lines.walk.clone();
    run.walk(walk, &mut on_repair)?;
    for n in lo..=hi {
        if run.lines.games.contains_key(&n) {
            run.summary.skipped += 1;
            continue;
        }
        if engine::stop_signal().is_some() {
            run.summary.stopped_at = Some(n);
            break;
        }
        if let Some(best_known) = &mut run.best_known {
            best_known.refresh()?;
        }
        let past = moves_before(n, &run.recent, &run.lines, run.best_known.as_ref(), optimal);
        let above = moves_above(n, &run.recent, &run.lines, run.best_known.as_ref(), plan);
        // The gap of the moves for n − 1 that the game starts from, unless it takes none.
        let gap_below = match past.first().and_then(Option::as_ref) {
            Some(below) => Some(gap(below.n, run.bound_of(below.n)?, below.score)?),
            None => None,
        };
        // The next game to play has the moves for its n − 1, this game's line or the file's.
        if let Some(next) = (n + 1..=hi).find(|m| !run.lines.games.contains_key(m)) {
            run.boards.start(next);
        }
        let played = play(n, &past, &above, gap_below, opts, plan, &mut run.earlier, &mut run.boards, optimal)?;
        let Some((line, board)) = played else {
            run.summary.stopped_at = Some(n);
            break;
        };
        run.record(line, board, &mut on_game)?;
        run.walk(BTreeSet::from([n]), &mut on_repair)?;
    }
    Ok(run.summary)
}

/// A board that the run set up for a searched or repeat game, which the repair weighs lines on and the unseeded play of
/// a game played twice searches: the game's job, what the game with no promotions scores, and what the board was set
/// up with.
#[derive(Clone)]
struct KeptBoard {
    job: Arc<dyn Job>,
    in_the_bag: u64,
    setup: SearchSetup,
}

impl KeptBoard {
    /// The board of `set`.
    fn of(set: &SearchSetUp) -> KeptBoard {
        KeptBoard { job: Arc::clone(&set.job), in_the_bag: set.in_the_bag, setup: set.setup }
    }
}

/// A run of the unsound mode as it goes: its lines and the best-known file, the lines a repeat may repeat, its own
/// lines of the games a game starts from, its boards (made ahead, and kept for the repair), and what it has done.
struct Run<'a> {
    opts: &'a SolveOptions,
    optimal: &'a Optimal,
    lines: UnsoundFile,
    /// The best-known file, if the run has one (the module documentation, "The best-known file").
    best_known: Option<BestKnownFile<'a>>,
    /// The name, with no directory, of the file the run appends to, which the lines it offers to the best-known file
    /// say they came from.
    name: Option<String>,
    earlier: Earlier<()>,
    /// This run's lines of the games before, for as many as a game may begin a trajectory from.
    recent: BTreeMap<u32, Moves>,
    /// The bounds of the games before ([`Line::bound`]), of the games this run played and those it set up for one.
    bounds: BTreeMap<u32, u64>,
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
        self.offer(n)?;
        on_game(&line);
        self.summary.solved += 1;
        if let Some(setup) = line.setup {
            self.earlier.add(n, setup.size(), setup.digest, line.promotions.clone(), ());
        }
        self.recent.insert(n, Moves { n, moves: line.moves, score: line.score, source: Source::Run });
        self.recent.retain(|&m, _| m + BACK > n);
        self.bounds.insert(n, line.bound);
        self.bounds.retain(|&m, _| m >= n);
        self.mine.insert(n);
        self.boards.keep(n, board);
        Ok(())
    }

    /// The bound of game `k`: the run's, if it played the game, else that of its board, set up now
    /// ([`Ready::of`]).
    fn bound_of(&mut self, k: u32) -> Result<u64, String> {
        if let Some(&bound) = self.bounds.get(&k) {
            return Ok(bound);
        }
        let bound = Ready::of(k, &Before::None, self.opts, false).map_err(Failure::message)?.bound;
        self.bounds.insert(k, bound);
        Ok(bound)
    }

    /// The line of game `k` that the repair works on: the run's own, else the best-known file's (of a game below the
    /// run's first, which the run has no line of).
    fn line(&self, k: u32) -> Option<&Stored> {
        self.lines.games.get(&k).or_else(|| self.best_known.as_ref()?.line(k))
    }

    /// Whether the line of game `k` is the best-known file's alone: the run has none of the game.
    fn best_known_only(&self, k: u32) -> bool {
        !self.lines.games.contains_key(&k)
    }

    /// Offers the run's line of game `n` to the best-known file, if the run has one; whether it was appended.
    fn offer(&mut self, n: u32) -> Result<bool, String> {
        let (Some(best_known), Some(line)) = (&mut self.best_known, self.lines.games.get(&n)) else {
            return Ok(false);
        };
        let added = best_known.offer(self.name.as_deref(), [(n, line)])?;
        self.summary.best_known_added += added;
        Ok(added > 0)
    }

    /// Offers the best-known file, if the run has one, the last line of each game of the run's file, in order of n, a
    /// few at a time (the module documentation, "The best-known file").
    fn offer_file(&mut self) -> Result<(), String> {
        let Some(best_known) = &mut self.best_known else { return Ok(()) };
        let games: Vec<(u32, &Stored)> = self.lines.games.iter().map(|(&n, line)| (n, line)).collect();
        for chunk in games.chunks(OFFER_CHUNK) {
            self.summary.best_known_added += best_known.offer(self.name.as_deref(), chunk.iter().copied())?;
        }
        Ok(())
    }

    /// The repair's walk down (the module documentation, "The repair") from the lines of the games `changed`: the line
    /// this run has just recorded, or where a walk was when a stop cut it short, as its file says.  Each game below
    /// them, from the highest down, is weighed against the lines of `changed` up to [`REPAIR_REACH`] above it
    /// ([`weigh`](Self::weigh)), and a game whose line is replaced joins them, with the prime above it played
    /// again from it ([`prime_after`](Self::prime_after)).  A game below the run's first, which the run has no line of,
    /// is weighed on the best-known file's line, and a repair of it goes to the best-known file alone.  The walk ends
    /// at the first game that none of `changed` is that near, or at a stop signal, which leaves it to the next run that
    /// opens the file.
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
    /// it, scores more: the heaviest, the nearest on a tie.  The line is the run's, or the best-known file's where the
    /// run has none of the game.  A game that neither has a line for (the built-in table's, which are proven, stay as
    /// they are), and a prime or trivial line, are not weighed: a prime follows from the line below it, and a trivial
    /// line is optimal.  Returns whether it replaced the line.
    fn weigh(&mut self, k: u32, sources: &[u32], on_repair: &mut impl FnMut(&Repair)) -> Result<bool, String> {
        let Some(line) = self.line(k) else { return Ok(false) };
        if matches!(line.method, Method::Prime | Method::Trivial) {
            return Ok(false);
        }
        let began = Instant::now();
        let Some(board) = self.boards.board(k)? else { return Ok(false) };
        let mut best: Option<(u32, Vec<u32>, u64)> = None;
        for &j in sources {
            let promotions = board.job.surviving(&self.line(j).expect("a game whose line changed has one").moves);
            let sum = promotions.iter().map(|&p| u64::from(p)).sum();
            if best.as_ref().is_none_or(|&(_, _, most)| sum > most) {
                best = Some((j, promotions, sum));
            }
        }
        let line = self.line(k).expect("the line weighed");
        let Some((from, promotions, sum)) = best.filter(|&(_, _, sum)| board.in_the_bag + sum > line.score) else {
            return Ok(false);
        };
        let moves = board.job.moves_for(&promotions);
        let (method, was, score) = (line.method, line.score, board.in_the_bag + sum);
        let seconds = began.elapsed().as_secs_f64();
        let best_known_only = self.best_known_only(k);
        let repair = Repair { n: k, from, method, was, score, moves, promotions, seconds, best_known_only };
        self.replace(repair, None, on_repair)?;
        Ok(true)
    }

    /// Plays the prime game k + 1 again from the line of game `k`, if the run or the best-known file has a prime line
    /// of it and the prime formula on the line of `k` scores more, as it does once that line is repaired; it joins
    /// `changed` if so.
    fn prime_after(
        &mut self,
        k: u32,
        changed: &mut BTreeSet<u32>,
        on_repair: &mut impl FnMut(&Repair),
    ) -> Result<(), String> {
        let p = k + 1;
        let Some(line) = self.line(p).filter(|line| line.method == Method::Prime) else { return Ok(()) };
        let began = Instant::now();
        let SetUp::Derived(Derived::Prime { candidates }) =
            set_up(p, &Before::Run, self.opts).map_err(Failure::message)?
        else {
            return Ok(());
        };
        let Some(below) = self.line(k) else { return Ok(()) };
        let (moves, promotions) = prime_game(p, &candidates, &below.moves);
        let score = replay(p, &moves).map_err(|e| format!("n={p}: the prime moves from the repaired {k}: {e}"))?;
        if score <= line.score {
            return Ok(());
        }
        let source = match (self.mine.contains(&k), self.best_known_only(k)) {
            (true, _) => Source::Run,
            (false, false) => Source::Results,
            (false, true) => Source::BestKnown,
        };
        let prev = Moves { n: k, moves: below.moves.clone(), score: below.score, source };
        let (was, seconds) = (line.score, began.elapsed().as_secs_f64());
        let best_known_only = self.best_known_only(p);
        let repair =
            Repair { n: p, from: k, method: Method::Prime, was, score, moves, promotions, seconds, best_known_only };
        self.replace(repair, Some(&prev), on_repair)?;
        changed.insert(p);
        Ok(())
    }

    /// Records `repair`: checks its moves as those of a game played are checked (they replay to its score, which is no
    /// more than the table's) and that they score more than the line they replace, appends the line again with them
    /// ([`repaired_line`], with `prev` for a prime played again) to the run's file, tells the repeats and the games
    /// that follow of them, and hands the repair to `on_repair`.  A repair of the best-known file's line alone, of a
    /// game the run has no line of, goes to the best-known file only.  Either way the best-known file is offered it.
    fn replace(
        &mut self,
        repair: Repair,
        prev: Option<&Moves>,
        on_repair: &mut impl FnMut(&Repair),
    ) -> Result<(), String> {
        let n = repair.n;
        let fail = |e: String| format!("n={n}: the repair from {}: {e}", repair.from);
        let score = replay(n, &repair.moves).map_err(fail)?;
        let old = self.line(n).expect("a line to replace");
        if score != repair.score || score <= old.score {
            return Err(fail(format!("its moves score {score}, and the line it replaces {}", old.score)));
        }
        if let Some(best) = self.optimal.score(n).filter(|&best| score > best) {
            return Err(fail(format!("its moves score {score}, above the table's {best}, which is proven")));
        }
        let json = repaired_line(&old.text, &repair, prev)?;
        let (method, problem) = (old.method, old.problem);
        let (moves, promotions, text) = (repair.moves.clone(), repair.promotions.clone(), json.to_string());
        let stored = Stored { score, moves, promotions, method, problem, text };
        if repair.best_known_only {
            let added = match &mut self.best_known {
                Some(best_known) => best_known.offer(self.name.as_deref(), [(n, &stored)])?,
                None => 0,
            };
            self.summary.best_known_added += added;
            self.summary.best_known_repaired += added;
            if added > 0 {
                self.mine.insert(n);
                on_repair(&repair);
            }
            return Ok(());
        }
        self.mine.insert(n);
        self.lines.append(n, &json, stored)?;
        if let Some((size, digest)) = problem {
            self.earlier.add(n, size, digest, repair.promotions.clone(), ());
        }
        if let Some(recent) = self.recent.get_mut(&n) {
            (recent.moves, recent.score) = (repair.moves.clone(), score);
        }
        self.summary.repaired += 1;
        self.offer(n)?;
        on_repair(&repair);
        Ok(())
    }
}

/// The built-in table's moves for `game`, which are proven.
fn table_moves(game: u32, optimal: &Optimal) -> Option<Moves> {
    let (moves, score) = (optimal.moves(game)?, optimal.score(game)?);
    Some(Moves { n: game, moves: moves.to_vec(), score, source: Source::Table })
}

/// The moves for `game` that the run has, from its own line (`run`), else its file's, or from the best-known file's
/// line if that scores more (the module documentation, "The best-known file"): none if none of them has the game.
fn kept_moves(
    game: u32,
    run: &BTreeMap<u32, Moves>,
    file: &UnsoundFile,
    best_known: Option<&BestKnownFile>,
) -> Option<Moves> {
    let own = run.get(&game).cloned().or_else(|| {
        let (moves, score) = file.moves(game)?;
        Some(Moves { n: game, moves: moves.to_vec(), score, source: Source::Results })
    });
    let better = best_known
        .and_then(|best_known| best_known.moves(game))
        .filter(|&(_, score)| own.as_ref().is_none_or(|own| score > own.score));
    match better {
        Some((moves, score)) => Some(Moves { n: game, moves: moves.to_vec(), score, source: Source::BestKnown }),
        None => own,
    }
}

/// The moves for the games `n − 1`, `n − 2`, ..., `n − BACK` that game `n` starts from and begins its trajectories
/// from, from game `n − 1` down: each is this run's line of the game (`run`), else the file's line, or the best-known
/// file's if that scores more, else the table's; none for a game below 1, or that nothing has the moves for.  A game
/// that lacks the moves for `n − 1` has a note on stderr.
fn moves_before(
    n: u32,
    run: &BTreeMap<u32, Moves>,
    file: &UnsoundFile,
    best_known: Option<&BestKnownFile>,
    optimal: &Optimal,
) -> Vec<Option<Moves>> {
    let moves_of = |game: u32| kept_moves(game, run, file, best_known).or_else(|| table_moves(game, optimal));
    (1..=BACK)
        .map(|j| {
            let found = n.checked_sub(j).filter(|&game| game >= 1).and_then(moves_of);
            if found.is_none() && j == 1 && n > 1 {
                let _ = writeln!(
                    std::io::stderr(),
                    "note: no moves for {} (in this run, the file, the best-known file or the built-in table): game \
                     {n} starts with no seed",
                    n - 1
                );
            }
            found
        })
        .collect()
}

/// The moves for the games `n + 1`, ..., `n + plan.above` that game `n` begins trajectories from: each is this run's
/// line of the game (`run`), else the file's line, or the best-known file's if that scores more; a game that none of
/// them has gives none.  The table gives none: a run never takes the proven game above one it plays.
fn moves_above(
    n: u32,
    run: &BTreeMap<u32, Moves>,
    file: &UnsoundFile,
    best_known: Option<&BestKnownFile>,
    plan: &Plan,
) -> Vec<Moves> {
    let moves_of = |game: u32| kept_moves(game, run, file, best_known);
    (1..=plan.above).filter_map(|j| n.checked_add(j).filter(|&game| game <= super::MAX_N).and_then(moves_of)).collect()
}

/// A game's setup ([`set_up`]), its bound ([`Line::bound`]), and for a prime, the board its bound was made on.
struct Ready {
    set: SetUp,
    bound: u64,
    /// A prime's board, which the prime formula sets none up for, set up as for a game with no moves for n − 1, and the
    /// problem it poses: the board of the prime's bound, and of its unseeded play if it is played twice.
    prime: Option<(KeptBoard, Problem)>,
}

impl Ready {
    /// The setup of game `n` with `given` for its moves for n − 1, with the dive of its start if `dive` and it is
    /// searched, and its bound: `in_the_bag` plus the bound of the root's node (`Job::bound`) of its board, or of a
    /// prime's board set up for it; `in_the_bag` alone for a board with no promotion candidates, whose moves with no
    /// promotions are optimal.
    fn of(n: u32, given: &Before, opts: &SolveOptions, dive: bool) -> Result<Ready, Failure> {
        let set = set_up(n, given, opts)?;
        let (bound, prime) = match &set {
            SetUp::Search(set, _) => {
                if dive {
                    set.job.ready();
                }
                (set.in_the_bag + set.job.bound(), None)
            }
            SetUp::Derived(Derived::Trivial { in_the_bag, .. }) => (*in_the_bag, None),
            SetUp::Derived(Derived::Prime { .. }) => {
                let board = Ready::of(n, &Before::None, opts, false)?;
                let prime = match board.set {
                    SetUp::Search(set, problem) => Some((KeptBoard::of(&set), problem)),
                    SetUp::Derived(_) => None,
                };
                (board.bound, prime)
            }
        };
        Ok(Ready { set, bound, prime })
    }
}

/// The boards of a run: the setups of the games after the one in play, each made on a thread of its own while that
/// game plays, and the boards of its last [`REPAIR_REACH`] games, which the repair weighs lines on.  A game's setup
/// ([`set_up`]) depends on the game, on whether it has moves for n − 1, and on the options, and not on the moves
/// themselves, so it can be made before the game before it ends; with it go the dive of a searched game's start
/// (`Job::ready`) and the game's bound ([`Ready`]), which depend on the board alone.  A board is set up once: the game
/// takes the setup made ahead, and the run keeps its board for the repair.
struct Boards {
    opts: SolveOptions,
    /// The setups being made.
    making: Vec<Making>,
    /// The boards of the run's searched and repeat games, of the last [`REPAIR_REACH`] games it played, and those that
    /// the repair set up for a game that far below the last ([`board`](Self::board)).
    kept: BTreeMap<u32, KeptBoard>,
    /// The last game the run played (0 before the first).
    latest: u32,
}

/// A setup being made ahead: the game's, with moves for n − 1, and the thread that makes it.
struct Making {
    n: u32,
    thread: thread::JoinHandle<Result<Ready, Failure>>,
}

impl Boards {
    /// No setup made ahead and no board kept yet, for a run with `opts`.
    fn new(opts: &SolveOptions) -> Boards {
        Boards { opts: opts.clone(), making: Vec::new(), kept: BTreeMap::new(), latest: 0 }
    }

    /// Starts making the setup of game `n`, which will have moves for n − 1.
    fn start(&mut self, n: u32) {
        let opts = self.opts.clone();
        let make = move || Ready::of(n, &Before::Run, &opts, true);
        self.making.push(Making { n, thread: thread::spawn(make) });
    }

    /// The setup of game `n`, with `given` for its moves for n − 1, and its bound: the one made ahead if there is one
    /// and `given` has moves for n − 1, else one made now with `opts`.
    fn take(&mut self, n: u32, given: &Before, opts: &SolveOptions) -> Result<Ready, Failure> {
        let made = self.making.iter().position(|making| making.n == n).map(|i| self.making.swap_remove(i));
        match made {
            Some(making) if matches!(given, Before::Run) => {
                making.thread.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            }
            Some(making) => {
                let _ = making.thread.join();
                Ready::of(n, given, opts, false)
            }
            None => Ready::of(n, given, opts, false),
        }
    }

    /// Keeps `board`, the board of game `n`, which the run has just played, and lets go of those more than
    /// [`REPAIR_REACH`] games below it.
    fn keep(&mut self, n: u32, board: Option<KeptBoard>) {
        if let Some(board) = board {
            self.kept.insert(n, board);
        }
        self.latest = n;
        self.kept.retain(|&m, _| m + REPAIR_REACH >= n);
    }

    /// The board of game `k` for the repair: the one kept, else set up again; `None` for a board with no candidates.
    /// One set up again for a game within [`REPAIR_REACH`] below the last the run played is kept too, as the walk after
    /// each of the next games weighs the same line against it again (the best-known file's lines of the games below
    /// the run's first are such games).
    fn board(&mut self, k: u32) -> Result<Option<KeptBoard>, String> {
        if let Some(board) = self.kept.get(&k) {
            return Ok(Some(board.clone()));
        }
        let board = match set_up(k, &Before::None, &self.opts).map_err(Failure::message)? {
            SetUp::Search(set, _) => Some(KeptBoard::of(&set)),
            SetUp::Derived(_) => None,
        };
        if let Some(board) = board.as_ref().filter(|_| self.latest > 0 && k + REPAIR_REACH >= self.latest) {
            self.kept.insert(k, board.clone());
        }
        Ok(board)
    }
}

/// A run that ends waits for the setups it started, so that no thread of it outlives it.
impl Drop for Boards {
    fn drop(&mut self) {
        for making in self.making.drain(..) {
            let _ = making.thread.join();
        }
    }
}

/// Game `n` played in the unsound mode, from the moves for the games before it, `past` ([`moves_before`]: the first is
/// the moves for `n − 1`, which the start is seeded with, and the others begin trajectories; see the module
/// documentation, "One game"), and for the games above it that `above` has ([`moves_above`]), which begin
/// trajectories too; and its line, checked, with the board it set up if it searched or repeated, which the repair may
/// weigh lines on; `None` if a stop signal ended its rounds.  `gap_below` is the gap of the moves for `n − 1` that the
/// game starts from, over which the gap grows ([`Line::growth`]).  `earlier` has the lines a repeat may repeat, and is
/// told of the problem of this game's board.  The setup is the one `boards` made ahead, if it made this game's.  An
/// error names the game.
#[allow(clippy::too_many_arguments)]
fn play(
    n: u32,
    past: &[Option<Moves>],
    above: &[Moves],
    gap_below: Option<u64>,
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
    let ceiling = before.map(|before| ceiling_over(n, before));
    let Ready { set, bound, prime } = boards.take(n, &given, opts).map_err(Failure::message)?;
    let mut line = Line::blank(n, plan, opts, ceiling, bound);
    // The stop on growth: from the end of the rounds of the least level on, a score whose gap is within the plan's T of
    // the gap below, after the last `STOP_QUIET` rounds that raised nothing.
    let floor = plan.stop_floor();
    let stops = |score: u64, work: u64, quiet: u64| {
        let within = |(most, below)| growth(bound, score, below).unsigned_abs() <= most;
        work >= floor && quiet >= STOP_QUIET && plan.stop_growth.zip(gap_below).is_some_and(within)
    };
    let mut board = None;
    match set {
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
            let kept = KeptBoard::of(&set);
            if let Some(m) = repeated {
                let promotions = earlier.promotions(m).expect("a game that find found").to_vec();
                (line.method, line.repeats) = (Method::Repeat, Some(m));
                line.moves = set.job.moves_for(&promotions);
                line.promotions = promotions;
            } else {
                let bag = set.in_the_bag;
                // The rounds end at the run's ceiling or the stop on growth, whichever comes first.
                let enough = |sum: u64, work: u64, quiet: u64| {
                    let score = bag + sum;
                    let at_ceiling = ceiling.is_some_and(|ceiling| score >= ceiling);
                    at_ceiling || stops(score, work, quiet)
                };
                let seed = before.map(|before| before.moves.as_slice());
                // The games a trajectory may begin from, from n − 1 down, then from n + 1 up.
                let others: Vec<(u32, &[u32])> =
                    past.iter().flatten().chain(above).map(|m| (m.n, m.moves.as_slice())).collect();
                if !searched(&mut line, &kept, seed, &others, &clock, &enough) {
                    return Ok(None);
                }
            }
            board = Some(kept);
        }
    }
    conclude(&mut line, before, optimal).map_err(|e| format!("n={n}: {e}"))?;
    line.seconds = clock();
    line.growth = gap_below.map(|below| growth(bound, line.score, below));
    line.growth_stop =
        line.method == Method::Search && line.work < plan.budget() && stops(line.score, line.work, line.quiet);
    line.pushed = line.method != Method::Trivial && line.growth.is_some_and(|growth| growth > PUSH_GROWTH);
    if !line.pushed {
        return Ok(Some((line, board)));
    }
    // The unseeded play of a game played twice, on the game's board; a prime has none, and takes the one set up for its
    // bound, with no moves for n − 1.
    let (board, prime_problem) = match (board, prime) {
        (Some(board), _) => (board, None),
        (None, Some((board, problem))) => (board, Some(problem)),
        (None, None) => return Ok(Some((line, None))),
    };
    let began = Instant::now();
    let clock = || began.elapsed().as_secs_f64();
    let mut unseeded = Line { unseeded: true, ..Line::blank(n, plan, opts, None, bound) };
    (unseeded.setup, unseeded.in_the_bag) = (Some(board.setup), Some(board.in_the_bag));
    let others: Vec<(u32, &[u32])> = above.iter().map(|m| (m.n, m.moves.as_slice())).collect();
    if !searched(&mut unseeded, &board, None, &others, &clock, &|_, _, _| false) {
        return Ok(None);
    }
    conclude(&mut unseeded, None, optimal).map_err(|e| format!("n={n}, unseeded: {e}"))?;
    unseeded.seconds = clock();
    unseeded.growth = gap_below.map(|below| growth(bound, unseeded.score, below));
    let (seconds, pushed) = (line.seconds + unseeded.seconds, line.pushed);
    let (kept, other) = if unseeded.score > line.score { (unseeded, line) } else { (line, unseeded) };
    // A prime's board poses a problem that a later game may repeat only if the line kept searched it: the prime
    // formula's line records no setup, and so no promotions that a repeat could play.
    if let Some(problem) = prime_problem.filter(|_| kept.unseeded) {
        earlier.remember(n, problem);
    }
    Ok(Some((Line { other: Some(OtherPlay::of(&other)), seconds, pushed, ..kept }, Some(board))))
}

/// The gap of the moves for game `n` that score `score`, on the game's bound `bound`: `bound` less `score`.
///
/// # Errors
///
/// Moves that score above the bound, which no play of the game exceeds.
fn gap(n: u32, bound: u64, score: u64) -> Result<u64, String> {
    bound.checked_sub(score).ok_or_else(|| format!("n={n}: the moves score {score}, above the game's bound {bound}"))
}

/// The growth of the gap of a play of a game whose bound is `bound` that scores `score`, over the moves for n − 1 it
/// starts from, whose gap is `below` ([`Line::growth`]): its gap, `bound` less `score`, less `below`.  The three are
/// below the largest score, far below 2^63.
fn growth(bound: u64, score: u64, below: u64) -> i64 {
    bound as i64 - score as i64 - below as i64
}

/// The search of a game on `board` into `line`: the start, seeded with the moves `seed` for n − 1 if any, and the rounds
/// of `line.plan` on `line.threads` threads, with trajectories from what survives of the moves of `others`, ended once
/// `enough` says so of the promotion sum and the work of the rounds so far ([`Job::improve`]); `clock` is the play's
/// seconds so far.  Fills in what the line records of the search, and its moves and promotions; false if a stop signal
/// ended the rounds.
fn searched(
    line: &mut Line,
    board: &KeptBoard,
    seed: Option<&[u32]>,
    others: &[(u32, &[u32])],
    clock: &dyn Fn() -> f64,
    enough: &dyn Fn(u64, u64, u64) -> bool,
) -> bool {
    let bag = board.in_the_bag;
    let improved = board.job.improve(seed, others, &line.plan, line.threads, clock, enough);
    if improved.interrupted {
        return false;
    }
    line.start = Some(bag + improved.start);
    line.start_seconds = improved.start_seconds;
    line.trajectories = Some(improved.trajectories);
    line.survivor_scores = Some(improved.survivors.iter().map(|&(game, sum)| (game, bag + sum)).collect());
    (line.rounds, line.quiet, line.steps, line.work) = (improved.rounds, improved.quiet, improved.nodes, improved.work);
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
    true
}

/// The run's ceiling on the score of game `n` that starts from the moves `before` for n − 1: n + their score, which no
/// play of game n exceeds if they are optimal (the module documentation, "One game").
fn ceiling_over(n: u32, before: &Moves) -> u64 {
    before.score + u64::from(n)
}

/// The checks of the moves of `line` before they count (the module documentation, "One game"), and what they give the
/// line: its score, whether that is at its ceiling, and its record of the moves for n − 1 it started from, `before`.
fn conclude(line: &mut Line, before: Option<&Moves>, optimal: &Optimal) -> Result<(), String> {
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
    if score > line.bound {
        return Err(format!(
            "the {method} moves score {score}, above the game's bound {}, which no play exceeds (promotions {:?})",
            line.bound, line.promotions
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
    fn the_plan_tilts_goes_back_and_widens_as_the_races_chose() {
        let plan = Plan::default();
        assert_eq!((TILT, BACK, WIDEN), (25, 8, 4));
        assert_eq!((plan.effort, plan.seed, plan.round_work), (MIN_EFFORT, 1, ROUND_WORK));
        assert_eq!(
            (plan.above, plan.known_to),
            (0, None),
            "no trajectory begins from above, and the whole table is known"
        );
        let growth = (DEFAULT_STOP_GROWTH, plan.stop_growth, STOP_QUIET, PUSH_GROWTH);
        assert_eq!(growth, (Some(25), Some(25), 4, 25), "the stop and the push on growth are on");
        // The rounds search with the plan's seed, the built-in tilt and widening, and the finder's other options.
        let lns = plan.lns();
        let default = finder::Options::default();
        assert_eq!((lns.fix, lns.unit_nodes, lns.seed), (default.fix, default.unit_nodes, default.seed));
        assert_eq!((lns.tilt, lns.widen), (25, Some(4)));
        assert_eq!(check_plan(&plan), Ok(()));
        // The finder beside a proof search never tilts or widens, whatever the built-in settings of the unsound mode are.
        assert!(default.tilt == 0 && default.widen.is_none());
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
    fn a_number_of_games_above_is_a_whole_number_in_range() {
        for text in ["9", "-1", "", "a", "1.5", "1,2", " 1", "99999999999"] {
            let err = parse_above(text).unwrap_err();
            assert!(err.starts_with("--above takes J, a whole number from 0 to 8"), "{text:?}: {err}");
        }
        assert_eq!((parse_above("0"), parse_above("4"), parse_above("8")), (Ok(0), Ok(4), Ok(8)));
    }

    #[test]
    fn a_growth_is_a_whole_number_of_points_or_off() {
        // The stop's T is a distance from the gap below, 0 or more.
        assert_eq!(["0", "25", "off"].map(parse_stop_growth), [Ok(Some(0)), Ok(Some(25)), Ok(None)]);
        for text in ["", "a", "1.5", "Off", " 1", "none", "-1", "99999999999999999999999"] {
            let err = parse_stop_growth(text).unwrap_err();
            assert_eq!(err, format!("--stop-growth takes T, a whole number of points from 0 up, or off: got {text:?}"));
        }
        // The floor of the stop on growth is the budget of level 1, at every level.
        let plan = Plan { effort: 3, ..Plan::default() };
        assert_eq!((plan.stop_floor(), Plan { effort: 6, ..plan }.stop_floor()), (2 * ROUND_WORK, 2 * ROUND_WORK));
    }

    #[test]
    fn a_plan_with_an_option_out_of_range_is_refused() {
        let plan = Plan { effort: 3, above: 8, ..Plan::default() };
        assert_eq!(check_plan(&plan), Ok(()));
        let refused = |plan: Plan| check_plan(&plan).unwrap_err();
        for effort in [MIN_EFFORT - 1, MAX_EFFORT + 1] {
            let message =
                format!("--effort takes L, the level of effort, a whole number from 1 to 6: got \"{effort}\"");
            assert_eq!(refused(Plan { effort, ..plan }), message);
        }
        assert!(refused(Plan { above: MAX_ABOVE + 1, ..plan }).starts_with("--above takes J"));
        assert!(refused(Plan { round_work: 0, ..plan }).contains("is 0"));
        // Any cut of the table, and any stop on growth, is a plan.
        let cut = Plan { known_to: Some(0), stop_growth: Some(9), ..plan };
        assert_eq!(check_plan(&cut), Ok(()));
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
            "ceiling":456,"at_ceiling":false,"growth":30,"rounds":3,"repaired":null,"build":{"commit":"x"}}"#;
        let (moves, promotions) = (vec![2, 4, 6], vec![4, 6]);
        let (n, from, was, score, seconds) = (396, 398, 100, 500, 0.5);
        let repair =
            Repair { n, from, method: Method::Search, was, score, moves, promotions, seconds, best_known_only: false };
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
        assert_eq!(
            line.get("growth"),
            Some(&Json::from(30 - 400i64)),
            "the gap is 400 smaller, over the same gap below"
        );
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
        // The moves below gained 10, and their gap is 10 smaller.
        assert_eq!(line.get("growth"), Some(&Json::from(30 - 400 + 10i64)));
    }
}
