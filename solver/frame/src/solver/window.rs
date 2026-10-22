//! The window: the games of a range in flight at once, and the driver that runs them ([`solve`]).
//!
//! A game of the chain needs two things of the game before it: its moves, for the seed, and its score, for the
//! ceiling.  Everything else (its board, its frames, its search problem and whether an earlier game settled that
//! problem) is its own, and the ceiling only stops a search early.  So a range keeps a *window* of up to K consecutive
//! games in flight ([`SolveOptions::window`], `--window K`): game `n + 1` is set up while `n` searches, its search
//! starts from `n`'s best game so far, without a ceiling, is given `n`'s better games as they are found, and takes the
//! ceiling `n + 1 + opt(n) − in_the_bag` and the seed of `n`'s final game the moment `n` is concluded.  The threads are
//! moved between the searches while they run, the oldest search first (`allot`).  With K = 1, and always with one
//! thread, this is the range solved in order: a game enters the window only once the one before it is concluded, so
//! its search starts with the final seed and the ceiling, on every thread, and the lines are those of a loop over the
//! games.
//!
//! # The phases of a game
//!
//! 1. *Setup*, on a thread of the game's own, which holds one of the run's threads ([`set_up`]): the checks of a
//!    previous game not made by this run, the tables, the checkpoint; then the prime shortcut, the trivial game, or the
//!    search problem with its [`Job`].
//! 2. *Repeat lookup*, on the driver's thread, in the order of n, once the setups of all the earlier games of the
//!    window are done ([`Earlier::find`]).  A game of the window that set a search up and is not concluded counts
//!    as a candidate, as the documentation of `find` argues, so a game repeats exactly the game it repeats when the
//!    range is solved in order.  A repeat waits for the game it repeats, which is older and so is concluded first.
//! 3. *Search*, on the game's thread, when the driver says go ([`Go`]): with the seed and the ceiling known then, and
//!    the workers the driver grants it, not while it is deferred (see "Deferring").  Its outcome goes back to the
//!    driver; the checkpoint stays.
//! 4. *Conclusion*, on the driver's thread, of the oldest game of the window (the *head*) only, whose previous game is
//!    concluded ([`conclude_game`]): the result is built and checked, appended to the results file and handed to the
//!    caller, the checkpoint of a completed game is deleted, and the game becomes the previous game of the next.  A
//!    conclusion can make the next head concludable at once (a repeat, a prime or trivial game, a search already
//!    done), so heads are concluded as long as they can be.
//! 5. After a conclusion, the next game, if its search runs, is given its ceiling ([`Control::set_ceiling`]) and the
//!    seed of the final game ([`Job::hint`]); one not started yet gets both with its go.
//!
//! A game's `phases.total` is its own work: its setup, its repeat lookup, its search with its start and moves, and its
//! conclusion up to the replay; not the time it waited in the window.
//!
//! # Seeds
//!
//! The *provisional game* of a game of the window is its final game once it is concluded; for a search that has
//! ended, the game of its outcome; for a search that runs, the game of its best promotions so far; for a repeat of
//! `m`, the game of `m`'s provisional promotions (`m`'s promotions once it is concluded, its search's best so far,
//! or those of the game it repeats in turn) on its own board; for a prime game, the prime formula on the provisional
//! game before it; for a trivial game, the game with no promotions; otherwise there is none yet.  A search starts only
//! once the game before it has a provisional game to seed it with, unless it takes no seed (`seed_incumbent` off, or
//! resumed) or has no previous game; it never waits for that game to be proven.
//!
//! The provisional game of the game before it improves as that game's search runs, and a search that takes a seed is
//! given each improvement: every tick, while the game before it is in flight, a running search is given that game's
//! provisional game as a hint ([`Job::hint`]: the survivors of its seed offered to its control) when its score is
//! better than that of the game the search was seeded or last hinted with.  The score of a game is the sum of its
//! moves, and the moves of a provisional game take a pass over a board, so they are made only for a game whose
//! *value* has changed, which is known without them ([`Seed::value`]): the moves' sum of a game of moves, the
//! promotions' sum of a game of promotions (its search's best value, read from its control), and the value of the game
//! before it for a prime game.  The conclusion of the game before it gives the search the final seed, as a hint, with
//! its ceiling (phase 5).
//!
//! # Deferring
//!
//! A search ready to start whose previous game is not concluded is *deferred* while the sum of its seed's survivors
//! ([`Job::survivors`]) reaches the *provisional ceiling*: the ceiling the provisional game of the game before it would
//! give if it were final (`n + score − in_the_bag`, [`ceiling`]).  If that game is final, the ceiling settles the
//! search at once when it is given, so a search started without it could only do work that is thrown away, on threads
//! the older searches could use.  The search is weighed again whenever the value of that provisional game changes, and
//! starts as usual once its seed's survivors no longer reach the provisional ceiling, or once the game before it is
//! concluded: then with its ceiling, so that it ends at once if they reach it.  A deferred search is not a claim on
//! the threads, and, not started, has no provisional game for the game after it.  Deferring changes no result: the
//! game is searched, or settled by the ceiling, exactly as it would be once the game before it is concluded; it only
//! waits.  The head is never deferred, since the game before it is concluded.
//!
//! # Provenance
//!
//! The lines are those of the range solved in order, but for what a multi-threaded search may vary:
//!
//! * A line is appended only after every line it rests on: that of `n − 1` (its `prev`) and, for a repeat, that of
//!   the game it repeats.  Both are earlier, and lines are appended in the order of n.
//! * `prev`, `same_search` and `assumes` are derived as the range solved in order derives them, from the concluded
//!   games; the repeat names the same game (above).
//! * The proofs are of the same kinds.  A completed search's promotion sum is its problem's best, and its proof is by
//!   the ceiling exactly when that best reaches the ceiling, however the search ran: a search started before its
//!   ceiling was known and stopped (by a limit or a signal) with its best at the ceiling is proven by it, as it would
//!   have stopped there with the ceiling given from the start.
//! * Seeds and hints from a provisional game, offers at a conclusion, ceilings given late and deferrals change how or
//!   when a search runs, never the promotion sum it proves best: every set offered is promotable (the seed's
//!   survivors), and the ceiling is proven given the game of `n − 1` optimal, which a proof by the ceiling rests on
//!   anyway.
//! * What may differ: the moves and promotions where a problem has several optimal sets (as between thread counts),
//!   and so the next line's digest of the previous game's moves; nodes, the searches' counters, the engines' counters
//!   and the times.
//!
//! # Stopping
//!
//! A failure (a setup that fails, a search that fails, a game that a check rejects) is reported only when its game is
//! the head, and a search that did not complete ends the range only then, after its line, so a run fails or stops at
//! the game where the range solved in order would, with the same lines before it.  When the range ends, every search
//! still in flight is stopped ([`Control::stop`]: it writes its final checkpoint, if checkpointing, so that the next
//! run resumes it, and what it found is not recorded), nothing new starts, a game that waits for its go is cancelled,
//! the setups that run finish, and every thread is joined before [`solve`] returns.  After a stop signal every engine
//! stops on its own, and no search starts but the head's: started, it stops at once with its starting units in its
//! checkpoint, the incomplete line a search started after the signal gives in the range solved in order, and the
//! range ends there.  Time and node limits are each search's own, per game and invocation, as always.
//!
//! # The wall
//!
//! `--game-hours H` ([`SolveOptions::game_limit`]) bounds how long one game's own search may take, to find the first
//! game that takes more than H hours on the machine (the *wall*) and stop there.  A game's own search is the time it
//! has been the head of the window with its search running, summed over the invocations that searched it.  The chain
//! waits for the head and for no other game; a search behind it is suspended, or shares the threads that the head
//! leaves free, so the time it spends there is not what the game costs, and the search's elapsed time in its
//! checkpoint (which counts it) would end a run at a game that follows a dear one.  The driver keeps the head's clock
//! ([`clock`](super::clock)): it adds the time between its looks while the head's search runs, writes it beside the
//! game's checkpoint every checkpoint interval and when the search ends, and reads it back when a search starts with a
//! checkpoint to resume, so that a spot interruption, a new machine or a rerun goes on from it.
//!
//! When the head's clock reaches the limit, the driver stops the head's search, and once the search has ended the
//! range ends as it does at a stop signal: every search in flight is stopped with its final checkpoint, and nothing is
//! recorded for the head, not even an incomplete line (the games before it are recorded already).
//! [`RangeSummary::walled`] names the head and its clock.  Run again with a larger limit, the head resumes from its
//! checkpoint and its clock.
//!
//! The driver looks at the clock only when it wakes, every tick or at an event, so the head's search may end before
//! the stop reaches it.  A search that completed is concluded like any other and the range goes on, as a search that
//! ends before the engine looks at its time limit completes.  One that a limit of its own stopped has its final
//! checkpoint, and is at the wall all the same.  At each look the driver counts the time since the last look as the
//! head's search before it takes in the event that woke it, so a search whose end arrives once its clock has reached
//! the limit has passed the wall, however late the driver wakes.
//!
//! # Past limits
//!
//! With [`SolveOptions::keep_going`] (`--keep-going`) a game whose search a time or node limit stops does not end the
//! range.  Its best game so far is recorded as an unfinished line, its checkpoint is kept, and the next game goes on
//! from that game as a seed: the seed is any promotable set, so it cannot make a result wrong.  What the unfinished
//! game is not is proven, so the next game takes nothing that rests on its being optimal ([`Prev::proven`]): no
//! ceiling `n + score(n − 1)` (a search needs its own exhaustion to be proven, or a ceiling from a proven game after
//! it), no prime formula (a prime game is searched), and the tripwire that a score above that ceiling proves the
//! game before it not optimal is not made.  A repeat of an earlier search rests on that earlier game, which is in the
//! file as a completed one or is not repeated, so it is as before.  The games are solved one at a time, since a game
//! that goes on from a game in flight cannot know whether to take its ceiling.
//!
//! A rerun passes over the games the file holds only unfinished lines of, seeding the game after the last of them
//! with its best game (the best of its lines: a search started again can end below an earlier one), unless `retry` is
//! set, which searches them again (each from its checkpoint, if it has one).  With `retry_improved` an unfinished game
//! is searched again only if the game before it, as it is now (the file's, or one this run has just searched again),
//! has more to give than it had when the unfinished line was made: it is proven and the line's was not, or its best game
//! scores more than the line's seed, or the line had none.  That is how a script goes over games that were made in
//! blocks apart, so that each sees the one before it: a block's first game saw none, and its first game proven gives
//! the next the ceiling.  The passing over happens before a game enters the window (it costs no setup).  A signal still
//! ends the range, but its game gets no line, so that a rerun resumes it instead of passing it over.
//!
//! # The lab
//!
//! With a lab (`SolveOptions::lab`: `--lab-dir`, `--hook`; solver/docs/design/lab.md), the driver also tells the hook
//! when the head's own search, the clock of the wall, passes `--hook-after` ([`tell_long`](Window::tell_long)) and when
//! such a game has its line, and at each progress tick takes the lab's commands from its folder and applies or refuses
//! each ([`lab_tick`](Window::lab_tick)).  A command changes only how and when the games are searched: the threads (a
//! budget, and games whose workers the lab set, which the policy leaves alone), the branching settings of a search, an
//! incumbent offered through `search::offer_found`, a search started again from nothing (its checkpoint renamed aside)
//! and the build, by ending the range as a stop signal does ([`RangeSummary::redeploy`]).  A restart of a running search
//! stops it first, and the game goes back to its setup once it has stopped, unless it completed.  Without a lab, none of
//! this runs: no clock is kept but the wall's, and the policy has every thread.
//!
//! # Tracing
//!
//! With the environment variable `TAXMAN_WINDOW_TRACE` set, the driver prints a line on stderr for each of its events,
//! the seconds since the range began first: a setup started and done, a repeat found, a search started (its threads,
//! whether it has a seed, its ceiling), a search's workers changed (`workers n=N a->b`: given, given back, reclaimed,
//! or suspended at 0), a hint, a deferral, a search done, a ceiling told, a game concluded, the wall reached.  It is
//! how the policy's decisions were followed on the laptop, and how they are to be followed on a machine of many
//! threads.
//!
//! [`Control::set_ceiling`]: crate::engine::Control::set_ceiling
//! [`Control::stop`]: crate::engine::Control::stop

mod allot;

use super::lab::{self, Command, Folder, GameLab, Restarted, Status as Answer, LAB_TICK};
use super::{
    ceiling, clock, conclude_game, discard_checkpoint, search_cap, set_up, shared_spill_dir, stored_prev,
    stored_unfinished, Before, Context, Derived, Earlier, Failure, Found, GameResult, Go, Job, Outcome, Prev,
    RangeSummary, SearchSetUp, SetUp, SolveOptions, Wall,
};
use crate::accelerate::prime_shortcut;
use crate::engine::{self, Status, WORKER_STACK_SIZE};
use crate::game::Optimal;
use crate::json::Json;
use crate::known::Known as KnownGames;
use crate::results::{BuiltOn, ResultsFile};
use crate::search::{Problem, SearchTactics};
use crate::tactics::Tactics;
use allot::{allot, Grant, Load, Need, TICK};
use std::collections::BTreeMap;
use std::fs;
use std::mem;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, Scope};
use std::time::{Duration, Instant, SystemTime};

/// A range of games for [`solve`].
pub(super) struct Range<'a> {
    pub(super) lo: u32,
    pub(super) hi: u32,
    /// The game of `lo − 1`.
    pub(super) before: Before,
    /// The known sources: the table the scores are confirmed against, and, with `--use-prev`, what the games draw on.
    pub(super) known: &'a KnownGames<'a>,
    /// The results file: its completed games are passed over, and the others appended to it.
    pub(super) results: Option<&'a mut ResultsFile>,
}

/// Solves `range` with up to [`SolveOptions::window`] games in flight (see the module documentation), handing each
/// result to `on_game` in order; `earlier` has the games whose search a game of the range may repeat.
pub(super) fn solve(
    range: Range<'_>,
    opts: &SolveOptions,
    earlier: &mut Earlier,
    on_game: &mut dyn FnMut(&GameResult),
) -> Result<RangeSummary, String> {
    let folder = opts.lab.dir.as_deref().map(Folder::open).transpose()?;
    let summary = thread::scope(|scope| {
        let (events, received) = mpsc::channel();
        // Dropped when the closure returns or unwinds, before the threads are joined: that ends the range (`Drop`).
        let mut window = Window::new(range, opts, earlier, events, folder);
        window.run(scope, &received, on_game).map(|()| window.summary.clone())
    });
    // Only removes the directory if the engines left nothing in it; not before, since the games' engines may spill
    // into it side by side.
    if let Some(dir) = shared_spill_dir(opts) {
        let _ = fs::remove_dir(dir);
    }
    summary
}

/// What a game's thread tells the driver.
enum Event {
    /// The game's setup is done, in this time.
    SetUp { n: u32, set_up: Result<SetUp, String>, took: Duration },
    /// Its search is done, in this time.
    Searched { n: u32, outcome: Result<Box<Outcome>, String>, took: Duration },
    /// Its thread unwinds from a panic (in a build that unwinds, the tests'; a release build aborts).
    Panicked { n: u32 },
}

/// A game's thread: its setup, then, for a search, its search once the driver says go, unless the game is cancelled
/// first (the driver drops the sender of the go).
fn play(
    n: u32,
    before: &Before,
    opts: &SolveOptions,
    known: &KnownGames,
    lab: Option<GameLab>,
    events: &Sender<Event>,
    go: &Receiver<Go>,
) {
    let _unwinding = Unwinding { n, events };
    let start = Instant::now();
    let set_up = set_up(n, before, opts, known, lab).map_err(Failure::message);
    let job = match &set_up {
        Ok(SetUp::Search(set, _)) => Some(Arc::clone(&set.job)),
        _ => None,
    };
    // A send fails only once the range has ended, and then nothing is left to do.
    if events.send(Event::SetUp { n, set_up, took: start.elapsed() }).is_err() {
        return;
    }
    let Some(job) = job else { return };
    let Ok(go) = go.recv() else { return };
    let start = Instant::now();
    let outcome = job.search(go, opts).map(Box::new).map_err(|e| super::at(n, Failure::Error(e)).message());
    let _ = events.send(Event::Searched { n, outcome, took: start.elapsed() });
}

/// Tells the driver when a game's thread unwinds from a panic, so that the driver, which may be waiting for the game,
/// fails the range at it instead of waiting forever; `std::thread::scope` raises the panic again when the range has
/// ended.
struct Unwinding<'e> {
    n: u32,
    events: &'e Sender<Event>,
}

impl Drop for Unwinding<'_> {
    fn drop(&mut self) {
        if thread::panicking() {
            let _ = self.events.send(Event::Panicked { n: self.n });
        }
    }
}

/// A game of the window.
struct Game {
    /// Its previous game, as its setup knows it.
    before: Before,
    stage: Stage,
    /// Its own time so far: its setup, its repeat lookup and its search.
    own: Duration,
    /// Its wall clock: the time it has been the head with its search running, over every invocation (see "The wall");
    /// kept at zero unless `--game-hours` asks for a wall or a lab is attached (its hook's `long`, its restarts).
    headed: Duration,
    /// What was last written to its clock file.
    headed_kept: Duration,
    /// Whether its search has been stopped at the wall: the range ends at it once the search has ended, unless the
    /// search completed first (see "The wall").
    stopped_at_wall: bool,
    /// The sender of its go while its thread may wait for one; dropping it cancels the game's search.
    go: Option<Sender<Go>>,
    /// What the lab did to the game, for a setup of it started again after a restart (without it, a setup reads it
    /// from beside the game's checkpoint).
    relab: Option<GameLab>,
    /// A restart the lab asked for, waiting for the game's search to stop (see `lab_restart`).
    restart: Option<Restart>,
}

/// A restart of a game's search that the lab asked for: the command, to answer once it is done, and the tactics.
struct Restart {
    name: String,
    json: Option<Json>,
    tactics: SearchTactics,
}

/// Where a game of the window is.
enum Stage {
    /// Its setup waits for a thread.
    Waiting,
    /// Its setup runs on its thread.
    SettingUp,
    /// Its setup or its search failed, as the message says: reported when the game is the head.
    Failed(String),
    /// A prime or trivial game, concluded from its setup.
    Derived(Derived),
    /// A search problem, and what has come of it.
    Search(Box<SearchSetUp>, Plan),
}

/// What becomes of a game's search problem.
enum Plan {
    /// The repeat lookup is still to be made, with this problem.
    Unlooked(Problem),
    /// The problem is that of the game `m`, which settles it: a repeat.
    Repeat(u32),
    /// Its own search.
    Search(Search),
}

impl Plan {
    /// The problem of a lookup still to be made, taken out; the plan is then a search not started, until the lookup
    /// decides.
    fn take_problem(&mut self) -> Option<Problem> {
        match mem::replace(self, Plan::Search(Search::NotStarted(Weighed::default()))) {
            Plan::Unlooked(problem) => Some(problem),
            plan => {
                *self = plan;
                None
            }
        }
    }
}

/// A game's own search.
enum Search {
    /// Not started, as last weighed for deferring (see "Deferring").
    NotStarted(Weighed),
    Running(Run),
    Done(Box<Outcome>),
}

/// What became of a search not started when it was last weighed against the provisional game of `n − 1`: that game's
/// value ([`Seed::value`]), if it was weighed while that game is in flight, and whether the search defers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Weighed {
    value: Option<u64>,
    defers: bool,
}

/// The provisional game of `n − 1` as a running search was given it (see "Seeds"): its value when it was last looked
/// at ([`Seed::value`]), and the score of the best one given, as the seed or a hint.
#[derive(Clone, Copy)]
struct Given {
    value: u64,
    score: u64,
}

/// A search that runs, as the driver follows it.
struct Run {
    /// The workers it was last told to have (0: its engine is to be suspended).
    target: usize,
    /// When it was last given threads.
    granted: Instant,
    /// Its engine's status at the last sample, once the engine has been seen running.
    last: Option<Status>,
    load: Load,
    /// What it was given of the provisional game of `n − 1`, if it was seeded with one; not if it started with the
    /// final game, or without a seed.
    given: Option<Given>,
    /// The finders the lab started beside it: each ends when its sender is dropped, with the search's end.
    finders: Vec<Sender<()>>,
}

impl Run {
    fn new(target: usize, now: Instant, given: Option<Given>) -> Run {
        // Just given its threads: nothing to act on until the next sample.
        Run { target, granted: now, last: None, load: Load::Starting { fresh: false }, given, finders: Vec::new() }
    }

    /// Takes a sample of the engine's status (`None` before the engine starts and after it ends).
    fn sample(&mut self, now: Option<Status>) {
        self.load = match (now, self.last) {
            (None, None) => Load::Starting { fresh: true },
            // Its first sample, with nothing to compare it with, or its engine has ended.
            (Some(_), None) | (None, Some(_)) => Load::Unknown,
            (Some(now), Some(before)) => {
                let idle_time = now.idle_time.saturating_sub(before.idle_time);
                let capacity = now.capacity.saturating_sub(before.capacity);
                // A tick in which the engine had no worker (suspended) says nothing of its idle share.
                if capacity.is_zero() {
                    Load::Unknown
                } else {
                    let idle_share = idle_time.as_secs_f64() / capacity.as_secs_f64();
                    Load::Sampled { idle_share, idle_now: now.idle, idle_time, fresh: true }
                }
            }
        };
        if now.is_some() {
            self.last = now;
        }
    }

    /// The policy has run with the last sample, which is not acted on again.
    fn used(&mut self) {
        if let Load::Starting { fresh } | Load::Sampled { fresh, .. } = &mut self.load {
            *fresh = false;
        }
    }

    /// What [`allot()`] sees of it at `now`: suspended once its target is 0 and the last sample saw no worker left.
    fn need(&self, now: Instant) -> allot::Running {
        let live = self.last.map_or(0, |status| status.live);
        allot::Running { target: self.target, live, load: self.load, since_grant: now - self.granted }
    }
}

/// The game of `n − 1` of a game of the window, as the driver knows it now.
enum Known<'w> {
    /// Concluded, or not made by this run: final.
    Final(&'w Prev),
    /// In flight, with a provisional game.
    Provisional(Seed<'w>),
    /// In flight, with no game yet.
    NotYet,
    /// There is none: the game is solved standalone.
    None,
}

/// A provisional game, made into moves only when it is needed (that takes a pass over a board).
enum Seed<'w> {
    /// A game's moves.
    Moves(&'w [u32]),
    /// The game that promotions make on a game's board.
    On(&'w dyn Job, Vec<u32>),
    /// The prime formula of the game given, on the game before it.
    Prime(u32, Box<Seed<'w>>),
}

impl Seed<'_> {
    fn moves(&self) -> Vec<u32> {
        match self {
            Seed::Moves(moves) => moves.to_vec(),
            Seed::On(job, promotions) => job.moves_for(promotions),
            Seed::Prime(n, before) => prime_shortcut(*n, &before.moves()),
        }
    }

    /// What the game's score is known by without its moves (see "Seeds"): the moves' sum of a game of moves (its
    /// score); the promotions' sum of a game of promotions (its score less the board's `in_the_bag`, as for every game
    /// of the frame model); for a prime game, the value of the game before it (whose score fixes the prime game's: less
    /// the prime it opens with, plus n).  So the score of a provisional game changes only when its value
    /// does: within one kind of game by the above, and from one kind to the other, from promotions to moves (its
    /// search ends, or it is concluded), the value goes from a promotions' sum to a score on the same board, which is
    /// larger.
    fn value(&self) -> u64 {
        match self {
            Seed::Moves(moves) => sum(moves),
            Seed::On(_, promotions) => sum(promotions),
            Seed::Prime(_, before) => before.value(),
        }
    }
}

/// Whether an unfinished search that had `built_on` from the game of `n − 1` is to be tried again now that the game is
/// `last` (`retry_improved`, see "Past limits"): it has more to give than it had.  With no game of `n − 1` there is
/// nothing new; a game of `n − 1` after none is new; a proven game is new after an unproven one, and an unproven game
/// is new if its score is higher than the seed's was.
fn improved(last: Option<&Prev>, built_on: Option<BuiltOn>) -> bool {
    let Some(last) = last else { return false };
    match built_on {
        None => true,
        Some(on) if on.proven => false,
        Some(on) => last.proven || last.score > on.score,
    }
}

/// The sum of `numbers`: a game's score, of its moves, or a promotion sum.
fn sum(numbers: &[u32]) -> u64 {
    numbers.iter().map(|&k| u64::from(k)).sum()
}

/// The driver's state.
struct Window<'a> {
    opts: &'a SolveOptions,
    optimal: &'a Optimal,
    known: &'a KnownGames<'a>,
    results: Option<&'a mut ResultsFile>,
    earlier: &'a mut Earlier,
    lo: u32,
    hi: u32,
    /// The game of `lo − 1`.
    first: Before,
    /// The most games in flight.
    width: usize,
    /// The games in flight, by n: entered, and not yet concluded.
    games: BTreeMap<u32, Game>,
    /// The next game to enter the window.
    entering: u32,
    /// The next game to conclude: the head of the window, or a game of the results file to pass over.
    next: u32,
    /// The previous game of `next`, if it has one.
    last: Option<Prev>,
    summary: RangeSummary,
    events: Sender<Event>,
    /// When the range began, and whether `TAXMAN_WINDOW_TRACE` asks for the trace on stderr.
    began: Instant,
    trace: bool,
    /// When the head's clock was last brought up to date.
    clock_at: Instant,
    /// The run's budget of threads: `--threads`, or what the lab last said (`threads {total}`).
    threads: usize,
    /// The lab's command folder, if the run has one, and when it is next read (at each progress tick).
    folder: Option<Folder>,
    lab_at: Instant,
    /// The workers the lab gave games, by n (`threads {n, workers}`): those games' searches keep them, outside the
    /// policy that shares the rest.
    pins: BTreeMap<u32, usize>,
    /// The finders the lab has started, for their random choices.
    finders: u64,
}

impl<'a> Window<'a> {
    fn new(
        range: Range<'a>,
        opts: &'a SolveOptions,
        earlier: &'a mut Earlier,
        events: Sender<Event>,
        folder: Option<Folder>,
    ) -> Self {
        // With one thread there is none to run a second game on, and a range that keeps going past limits solves its
        // games one after another: a game cannot go on from a game in flight until it knows whether that game is
        // proven (see "Past limits").
        let width = if opts.threads <= 1 || opts.keep_going { 1 } else { opts.window.max(1) };
        let last = match &range.before {
            Before::Given(prev) => Some(prev.clone()),
            Before::Run | Before::None => None,
        };
        Window {
            opts,
            optimal: range.known.optimal(),
            known: range.known,
            results: range.results,
            earlier,
            lo: range.lo,
            hi: range.hi,
            first: range.before,
            width,
            games: BTreeMap::new(),
            entering: range.lo,
            next: range.lo,
            last,
            summary: RangeSummary::default(),
            events,
            began: Instant::now(),
            trace: std::env::var_os("TAXMAN_WINDOW_TRACE").is_some(),
            clock_at: Instant::now(),
            threads: opts.threads,
            folder,
            lab_at: Instant::now(),
            pins: BTreeMap::new(),
            finders: 0,
        }
    }

    /// A diagnostic line on stderr, when the trace is on: the seconds since the range began, then the event.
    fn trace(&self, what: std::fmt::Arguments) {
        if self.trace {
            let line = format!("{:.3} TRACE {}\n", self.began.elapsed().as_secs_f64(), what);
            let _ = std::io::Write::write_all(&mut std::io::stderr(), line.as_bytes());
        }
    }

    /// Runs the range until it ends: every game concluded, a game that did not complete, or a failure.
    fn run<'s>(
        &mut self,
        scope: &'s Scope<'s, '_>,
        received: &Receiver<Event>,
        on_game: &mut dyn FnMut(&GameResult),
    ) -> Result<(), String>
    where
        'a: 's,
    {
        let mut tick = Instant::now() + TICK;
        while !self.settle(on_game)? {
            self.share(scope);
            // While a search runs or a game waits for a thread or a seed, the driver wakes every tick; otherwise a
            // setup runs, whose end is the next thing to happen (see `ticking`).
            let event = if self.ticking() {
                received.recv_timeout(tick.saturating_duration_since(Instant::now())).ok()
            } else {
                received.recv().ok()
            };
            // The time since the last look was spent with the head's search as it was before this event, so a clock
            // that has reached the limit by now reached it while the search ran: the search is stopped at the wall
            // before the event, which may be its end, is taken in.
            let now = Instant::now();
            self.advance_clock(now);
            self.stop_at_wall();
            self.tell_long();
            if let Some(event) = event {
                self.handle(event);
            }
            if let Some(wall) = self.walled() {
                self.summary.walled = Some(wall);
                return Ok(());
            }
            if now >= tick {
                self.sample();
                self.hint();
                tick = now + TICK;
            }
            // The lab's commands, at each progress tick (see `lab_tick`).
            if self.folder.is_some() && now >= self.lab_at {
                self.lab_tick(scope);
                self.lab_at = now + self.opts.progress.unwrap_or(LAB_TICK);
                if self.summary.redeploy.is_some() {
                    // Every search stops with its final checkpoint as the range ends (`Drop`), and the head keeps its
                    // clock.
                    self.keep_clock(self.next);
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// Whether the results file has completed game `n`.
    fn in_file(&self, n: u32) -> bool {
        self.results.as_deref().is_some_and(|file| file.completed(n).is_some())
    }

    /// The game of the results file that the range passes over at the head `n`, as the previous game of the one after
    /// it: a completed game, or, when the range keeps going past limits, the best game of an unfinished search, which is
    /// a seed and not a proven game.  Not one that `retry` searches again, nor one that `retry_improved` does because the
    /// game before it, `last`, has improved.
    fn passed_over(&self, n: u32) -> Option<Prev> {
        let file = self.results.as_deref()?;
        if let Some(done) = stored_prev(file, n) {
            return Some(done);
        }
        if !self.opts.keep_going || self.opts.retry {
            return None;
        }
        let seed = stored_unfinished(file, n)?;
        // What the game had is what its latest attempt had, not the attempt that scored best: a search started again
        // can end below an earlier one.
        let latest = file.last_attempt(n)?;
        if self.opts.retry_improved && improved(self.last.as_ref(), latest.built_on) {
            return None;
        }
        Some(seed)
    }

    /// Passes over the games of the results file at the head, each the previous game of the one after it.
    fn skip_settled(&mut self) {
        while self.next <= self.hi {
            let Some(stored) = self.passed_over(self.next) else { break };
            if stored.proven {
                self.summary.skipped += 1;
            } else {
                self.summary.passed_over += 1;
            }
            self.last = Some(stored);
            self.next += 1;
        }
    }

    /// The previous game of game `n` as its setup knows it.
    fn before(&self, n: u32) -> Before {
        if n == self.lo {
            return self.first.clone();
        }
        if self.opts.keep_going {
            // Solved one at a time, from the head: the game before it is settled, by this run or by the file, proven or
            // not, and the setup is told which (the prime formula needs a proven game).
            debug_assert_eq!(n, self.next, "a range that keeps going enters only the head");
            return self.last.clone().map_or(Before::None, Before::Given);
        }
        match self.results.as_deref().and_then(|file| stored_prev(file, n - 1)) {
            Some(prev) => Before::Given(prev),
            None => Before::Run,
        }
    }

    /// Lets the next games into the window, up to its width; the games of the results file are passed over.
    fn enter(&mut self) {
        while self.games.len() < self.width && self.entering <= self.hi {
            let n = self.entering;
            self.entering += 1;
            if !self.in_file(n) {
                let game = Game {
                    before: self.before(n),
                    stage: Stage::Waiting,
                    own: Duration::ZERO,
                    headed: Duration::ZERO,
                    headed_kept: Duration::ZERO,
                    stopped_at_wall: false,
                    go: None,
                    relab: None,
                    restart: None,
                };
                self.games.insert(n, game);
            }
        }
    }

    /// Makes the repeat lookups that are due: in the order of n, each once the setups of the games before it are
    /// done.
    fn look_up(&mut self) {
        let repeats = self.opts.tactics.same_search && !self.opts.force_search;
        let mut found = Vec::new();
        for (&n, game) in &mut self.games {
            let Stage::Search(set, plan) = &mut game.stage else {
                if matches!(game.stage, Stage::Waiting | Stage::SettingUp) {
                    break;
                }
                continue;
            };
            let Some(problem) = plan.take_problem() else { continue };
            let start = Instant::now();
            // Not for a search that a checkpoint resumes or the lab restarted (searched, never replaced), nor with
            // `force_search`.
            let repeated = if repeats && !set.searched_anyway() { self.earlier.find(n, &problem) } else { None };
            // The games that follow compare their problems with this one, whether this game repeats or searches.
            self.earlier.remember(n, problem);
            if let Some(m) = repeated {
                *plan = Plan::Repeat(m);
                // Its thread waits for no go.
                game.go = None;
                found.push((n, m));
            }
            game.own += start.elapsed();
        }
        for (n, m) in found {
            self.trace(format_args!("repeat n={n} of={m}"));
        }
    }

    /// Concludes the head as long as it can be, letting games into the window and making their lookups as places
    /// free.  Returns whether the range has ended: every game concluded, or one that did not complete.
    fn settle(&mut self, on_game: &mut dyn FnMut(&GameResult)) -> Result<bool, String> {
        loop {
            // A range that keeps going passes over first: whether a game is passed over depends on the game before it
            // (`retry_improved`), which is known only once the games before it have been.
            if self.opts.keep_going {
                self.skip_settled();
                self.entering = self.entering.max(self.next);
            }
            self.enter();
            self.look_up();
            if !self.opts.keep_going {
                self.skip_settled();
            }
            if self.next > self.hi {
                return Ok(true);
            }
            let n = self.next;
            // Whether the hook was told that the game runs long: it is told of its line too.
            let long = matches!(self.games.get(&n), Some(Game { stage: Stage::Search(set, _), .. }) if set.lab.long);
            let Some(result) = self.conclude_head(n)? else { return Ok(false) };
            self.pins.remove(&n);
            if !result.completed && self.opts.keep_going && engine::stop_signal().is_some() {
                // A signal stopped the search, not its limit: the game gets no line (as at the wall), so that a rerun
                // resumes it from its checkpoint rather than passing it over as one that spent its limit.
                self.trace(format_args!("stopped n={n} by a signal"));
                self.summary.stopped_at = Some(n);
                return Ok(true);
            }
            if let Some(file) = self.results.as_deref_mut() {
                file.append(&result)?;
            }
            on_game(&result);
            self.trace(format_args!("concluded n={n}"));
            if let (true, Some(hook)) = (long, &self.opts.lab.hook) {
                lab::spawn_hook(hook, &lab::done_event(n, result.to_json()));
            }
            self.summary.solved += 1;
            if !result.completed {
                if !self.opts.keep_going {
                    self.summary.stopped_at = Some(n);
                    return Ok(true);
                }
                // Its checkpoint stays, for a retry.  The next game goes on from its best game, as a seed.
                self.summary.unfinished += 1;
            } else {
                discard_checkpoint(self.opts, n);
            }
            self.earlier.add_result(&result);
            let mut prev = Prev::from_result(&result);
            if !result.completed {
                // A search started again can end below an earlier one the file records: the better seeds the next game.
                let file = self.results.as_deref();
                if let Some(best) = file.and_then(|file| stored_unfinished(file, n)).filter(|b| b.score > prev.score) {
                    prev = best;
                }
            }
            self.tell(n + 1, &prev);
            self.last = Some(prev);
            self.next += 1;
        }
    }

    /// The result of the head `n`, if it can be concluded now: it has been found, or has failed.  A failure is
    /// reported here, and a game that a check rejects after it was found goes to the results file's rejected games.
    fn conclude_head(&mut self, n: u32) -> Result<Option<GameResult>, String> {
        let Some(game) = self.games.get(&n) else { return Ok(None) };
        let found = match &game.stage {
            Stage::Failed(message) => return Err(message.clone()),
            Stage::Derived(derived) => Found::Derived(derived),
            Stage::Search(set, Plan::Repeat(m)) => Found::Repeat(set, *m),
            Stage::Search(set, Plan::Search(Search::Done(outcome))) => Found::Searched(set, outcome),
            _ => return Ok(None),
        };
        let earlier_runs = self.results.as_deref().map_or(0, |file| file.earlier_runs(n));
        let context = Context { prev: self.last.as_ref(), optimal: self.optimal, own: game.own, since: Instant::now() };
        let concluded = conclude_game(n, found, self.earlier, earlier_runs, self.opts, context);
        self.games.remove(&n);
        match (concluded, self.results.as_deref()) {
            (Ok(result), _) => Ok(Some(result)),
            (Err(Failure::Rejected(result, reason)), Some(file)) => {
                let path = file.reject(&result, &reason)?;
                Err(format!("{reason}; the game is recorded in {}", path.display()))
            }
            (Err(failure), _) => Err(failure.message()),
        }
    }

    /// Gives game `n`, if its search runs, what the conclusion of its previous game `prev` gives: its ceiling, and
    /// the seed of the final game.  (A search not started gets them with its go.)
    fn tell(&self, n: u32, prev: &Prev) {
        let Some(Game { stage: Stage::Search(set, Plan::Search(Search::Running(_))), .. }) = self.games.get(&n) else {
            return;
        };
        if let Some(ceiling) = search_cap(set, n, prev.cap_score(), self.opts) {
            set.job.control().set_ceiling(ceiling);
            self.trace(format_args!("told n={n}"));
        }
        set.job.hint(&prev.moves);
    }

    /// What the driver knows now of the game before game `n` of the window.
    fn known_before(&self, n: u32) -> Known<'_> {
        let Some(game) = self.games.get(&n) else { return Known::NotYet };
        match &game.before {
            Before::Given(prev) => Known::Final(prev),
            Before::None => Known::None,
            Before::Run if n == self.next => {
                Known::Final(self.last.as_ref().expect("the game before the head is concluded and completed"))
            }
            Before::Run => self.provisional(n - 1).map_or(Known::NotYet, Known::Provisional),
        }
    }

    /// The provisional game of game `g` of the window (see "Seeds"), if it has one.
    fn provisional(&self, g: u32) -> Option<Seed<'_>> {
        let game = self.games.get(&g)?;
        match &game.stage {
            Stage::Derived(Derived::Prime { .. }) => match self.known_before(g) {
                Known::Final(prev) => Some(Seed::Prime(g, Box::new(Seed::Moves(&prev.moves)))),
                Known::Provisional(before) => Some(Seed::Prime(g, Box::new(before))),
                Known::NotYet | Known::None => None,
            },
            Stage::Derived(Derived::Trivial { plain, .. }) => Some(Seed::Moves(plain)),
            Stage::Derived(Derived::Known { game, .. }) => Some(Seed::Moves(&game.moves)),
            Stage::Search(_, Plan::Search(Search::Done(outcome))) => Some(Seed::Moves(&outcome.moves)),
            Stage::Search(set, _) => self.provisional_promotions(g).map(|promotions| Seed::On(&*set.job, promotions)),
            Stage::Waiting | Stage::SettingUp | Stage::Failed(_) => None,
        }
    }

    /// The provisional promotions of game `g`, which set a search up: its promotions once it is concluded, else what
    /// its search found so far, or the provisional promotions of the game it repeats.
    fn provisional_promotions(&self, g: u32) -> Option<Vec<u32>> {
        let Some(game) = self.games.get(&g) else {
            // Concluded, so among the games that a repeat may repeat.
            return self.earlier.promotions(g).map(<[u32]>::to_vec);
        };
        match &game.stage {
            Stage::Search(_, Plan::Search(Search::Done(outcome))) => Some(outcome.promotions.clone()),
            Stage::Search(set, Plan::Search(Search::Running(_))) => set.job.best_so_far(),
            Stage::Search(_, Plan::Repeat(m)) => self.provisional_promotions(*m),
            _ => None,
        }
    }

    /// Whether the search of game `n`, set up and not started, and last weighed as `weighed` ([`weigh`](Self::weigh)),
    /// can start.
    fn ready(&self, n: u32, set: &SearchSetUp, weighed: Weighed) -> bool {
        // After a stop signal every engine stops on its own, and only the head's search starts (see "Stopping").
        if engine::stop_signal().is_some() && n != self.next {
            return false;
        }
        !weighed.defers && (!set.job.seeds() || !matches!(self.known_before(n), Known::NotYet))
    }

    /// Weighs each search set up and not started for deferring (see "Deferring"): one that takes a seed, while the game
    /// before it is in flight, defers while its seed's survivors reach the provisional ceiling, and is weighed again
    /// only when the value of that game's provisional game changes; any other does not defer.
    fn weigh(&mut self) {
        let mut changed = Vec::new();
        for (&n, game) in &self.games {
            let Stage::Search(set, Plan::Search(Search::NotStarted(last))) = &game.stage else { continue };
            let (weighed, reached) = match self.known_before(n) {
                Known::Provisional(before) if set.job.seeds() => {
                    let value = before.value();
                    if last.value == Some(value) {
                        continue;
                    }
                    let moves = before.moves();
                    let cap = ceiling(n, set.in_the_bag, Some(sum(&moves)), self.opts);
                    // The survivors and the provisional ceiling, if they reach it.
                    let reached =
                        cap.map(|cap| (set.job.survivors(&moves), cap)).filter(|(survivors, cap)| survivors >= cap);
                    (Weighed { value: Some(value), defers: reached.is_some() }, reached)
                }
                _ => (Weighed::default(), None),
            };
            if weighed != *last {
                if let (false, Some((survivors, cap))) = (last.defers, reached) {
                    self.trace(format_args!("defer n={n} survivors={survivors} ceiling={cap}"));
                }
                changed.push((n, weighed));
            }
        }
        for (n, weighed) in changed {
            if let Some(Game { stage: Stage::Search(_, Plan::Search(Search::NotStarted(last))), .. }) =
                self.games.get_mut(&n)
            {
                *last = weighed;
            }
        }
    }

    /// Gives each running search whose previous game is in flight that game's provisional game as a hint, if its score
    /// is better than that of the game the search was seeded or last hinted with (see "Seeds").  The moves are made
    /// only for a provisional game whose value has changed since it was last looked at.
    fn hint(&mut self) {
        let mut looked = Vec::new();
        for (&n, game) in &self.games {
            let Stage::Search(set, Plan::Search(Search::Running(Run { given: Some(given), .. }))) = &game.stage else {
                continue;
            };
            let Known::Provisional(before) = self.known_before(n) else { continue };
            let value = before.value();
            if value == given.value {
                continue;
            }
            let moves = before.moves();
            let score = sum(&moves);
            if score > given.score {
                set.job.hint(&moves);
                self.trace(format_args!("hint n={n} score={score}"));
            }
            looked.push((n, Given { value, score: score.max(given.score) }));
        }
        for (n, given) in looked {
            if let Some(Game { stage: Stage::Search(_, Plan::Search(Search::Running(run))), .. }) =
                self.games.get_mut(&n)
            {
                run.given = Some(given);
            }
        }
    }

    /// What the games hold and claim, oldest first, for [`allot()`]: each game's n and its need, but for the searches
    /// whose workers the lab set, which are not the policy's ([`pinned`](Self::pinned)).
    fn needs(&self, now: Instant) -> Vec<(u32, Need)> {
        let need = |(&n, game): (&u32, &Game)| match &game.stage {
            Stage::Waiting => Need::SetUp,
            Stage::SettingUp => Need::SettingUp,
            Stage::Search(set, Plan::Search(Search::NotStarted(weighed))) if self.ready(n, set, *weighed) => {
                Need::Ready
            }
            Stage::Search(_, Plan::Unlooked(_) | Plan::Search(Search::NotStarted(_))) => Need::Pending,
            Stage::Search(_, Plan::Search(Search::Running(run))) => Need::Running(run.need(now)),
            Stage::Search(_, Plan::Repeat(_) | Plan::Search(Search::Done(_)))
            | Stage::Derived(_)
            | Stage::Failed(_) => Need::Nothing,
        };
        let policy = |(n, game): &(&u32, &Game)| !self.pinned(**n, game);
        self.games.iter().filter(policy).map(|entry| (*entry.0, need(entry))).collect()
    }

    /// Whether game `n` is a search, set up or running, whose workers the lab set (`threads {n, workers}`): it keeps
    /// them, outside the policy.
    fn pinned(&self, n: u32, game: &Game) -> bool {
        self.pins.contains_key(&n)
            && matches!(game.stage, Stage::Search(_, Plan::Search(Search::NotStarted(_) | Search::Running(_))))
    }

    /// Whether the driver must wake every tick: a search runs (to sample it and give it hints), or a game waits for a
    /// thread, for a seed or while deferred (threads free as engines retire workers, and seeds appear and improve as
    /// searches run).  Otherwise only setups run, or nothing does, and what happens next is an event (the head is
    /// concluded before the driver waits, so a head that waits for nothing does not wait).
    fn ticking(&self) -> bool {
        self.games.values().any(|game| {
            matches!(
                game.stage,
                Stage::Waiting | Stage::Search(_, Plan::Search(Search::NotStarted(_) | Search::Running(_)))
            )
        })
    }

    /// Weighs the searches not started for deferring, then shares the threads ([`allot()`]) and carries out the grants.
    /// The searches whose workers the lab set have them first, and the policy shares what they leave of the run's
    /// threads.
    fn share<'s>(&mut self, scope: &'s Scope<'s, '_>)
    where
        'a: 's,
    {
        self.weigh();
        let now = Instant::now();
        let (held, pinned) = self.pinned_grants();
        for (n, grant) in pinned {
            match grant {
                Grant::Start(threads) => self.start_search(n, threads, now),
                Grant::Workers(workers) => self.set_workers(n, workers, now),
                Grant::SetUp => unreachable!("a pin is of a search"),
            }
        }
        let budget = self.threads.saturating_sub(held);
        let needs = self.needs(now);
        let grants = allot(budget, &needs.iter().map(|&(_, need)| need).collect::<Vec<_>>());
        for (i, grant) in grants {
            let n = needs[i].0;
            match grant {
                Grant::SetUp => self.start_setup(scope, n),
                Grant::Start(threads) => self.start_search(n, threads, now),
                Grant::Workers(workers) => self.set_workers(n, workers, now),
            }
        }
        if self.folder.is_some() {
            self.fit(budget, now);
        }
        for game in self.games.values_mut() {
            if let Stage::Search(_, Plan::Search(Search::Running(run))) = &mut game.stage {
                run.used();
            }
        }
    }

    /// The threads the searches whose workers the lab set hold, and what they are to be told: a running one its
    /// workers, if they are not what it was last told, and one ready to start its start, on its workers (one set to
    /// none waits, not started).
    fn pinned_grants(&self) -> (usize, Vec<(u32, Grant)>) {
        let (mut held, mut grants) = (0, Vec::new());
        for (&n, game) in &self.games {
            let Some(&pin) = self.pins.get(&n) else { continue };
            match &game.stage {
                Stage::Search(_, Plan::Search(Search::Running(run))) => {
                    held += pin.max(run.last.map_or(0, |status| status.live));
                    if run.target != pin {
                        grants.push((n, Grant::Workers(pin)));
                    }
                }
                Stage::Search(set, Plan::Search(Search::NotStarted(weighed)))
                    if pin > 0 && self.ready(n, set, *weighed) =>
                {
                    held += pin;
                    grants.push((n, Grant::Start(pin)));
                }
                _ => {}
            }
        }
        (held, grants)
    }

    /// Brings the threads the policy's games are told to hold within `budget`, when they hold more: the lab lowered
    /// the run's threads, or set a game's workers.  The youngest searches give workers back first (their engines retire
    /// them as they come idle); the oldest keeps one, and so does one in its start, which runs on its own thread.
    fn fit(&mut self, budget: usize, now: Instant) {
        let mut told = 0;
        let mut running = Vec::new();
        for (&n, game) in &self.games {
            if self.pinned(n, game) {
                continue;
            }
            match &game.stage {
                Stage::SettingUp => told += 1,
                Stage::Search(_, Plan::Search(Search::Running(run))) => {
                    told += run.target;
                    running.push((n, run.target, matches!(run.load, Load::Starting { .. })));
                }
                _ => {}
            }
        }
        let mut excess = told.saturating_sub(budget);
        let oldest = running.first().map(|&(n, ..)| n);
        for (n, target, starting) in running.into_iter().rev() {
            if excess == 0 {
                break;
            }
            let keep = usize::from(starting || Some(n) == oldest);
            let gives = target.saturating_sub(keep).min(excess);
            if gives > 0 {
                self.set_workers(n, target - gives, now);
                excess -= gives;
            }
        }
    }

    /// Starts the setup of game `n` on a thread of its own.
    fn start_setup<'s>(&mut self, scope: &'s Scope<'s, '_>, n: u32)
    where
        'a: 's,
    {
        let Some(game) = self.games.get_mut(&n) else { return };
        let (go, waits) = mpsc::channel();
        let (before, opts, known, events) = (game.before.clone(), self.opts, self.known, self.events.clone());
        let lab = game.relab.take();
        // The setup and the search's start ran on the main thread when games were solved one at a time; the stack is
        // reserved address space, committed only as touched.
        let spawned = thread::Builder::new()
            .name(format!("game-{n}"))
            .stack_size(WORKER_STACK_SIZE)
            .spawn_scoped(scope, move || play(n, &before, opts, known, lab, &events, &waits));
        match spawned {
            Ok(_) => {
                game.stage = Stage::SettingUp;
                game.go = Some(go);
                self.trace(format_args!("setup n={n}"));
            }
            Err(e) => game.stage = Stage::Failed(format!("n={n}: cannot start a thread for the game: {e}")),
        }
    }

    /// Starts the search of game `n` on `threads` workers: with the seed of the game before it, final or provisional,
    /// and the ceiling if that game is concluded.
    fn start_search(&mut self, n: u32, threads: usize, now: Instant) {
        let Some(Game { stage: Stage::Search(set, _), .. }) = self.games.get(&n) else { return };
        let seeds = set.job.seeds();
        // The ceiling is the one that a concluded game before it gives, or the bounds store's upper bound of the board.
        let (seed, cap, given) = match self.known_before(n) {
            Known::Final(prev) => {
                (seeds.then(|| prev.moves.clone()), search_cap(set, n, prev.cap_score(), self.opts), None)
            }
            Known::Provisional(before) if seeds => {
                let moves = before.moves();
                let given = Given { value: before.value(), score: sum(&moves) };
                (Some(moves), search_cap(set, n, None, self.opts), Some(given))
            }
            Known::Provisional(_) | Known::NotYet | Known::None => (None, search_cap(set, n, None, self.opts), None),
        };
        // The finder's seed is the game before it as the search takes it, and for a search that takes none (a resumed
        // one) the game before it all the same, if there is one.
        let finder_seed = if self.opts.finder_after == 0 {
            None
        } else {
            match self.known_before(n) {
                Known::Final(prev) => Some(prev.moves.clone()),
                Known::Provisional(before) => Some(before.moves()),
                Known::NotYet | Known::None => None,
            }
        };
        self.trace(format_args!("start n={n} threads={threads} seed={} cap={cap:?}", seed.is_some()));
        let clocked = self.clocked();
        let Some(game) = self.games.get_mut(&n) else { return };
        if clocked {
            // A search that resumes from its checkpoint goes on from the clock the last invocation kept.
            game.headed = clock::resume(self.opts, n);
            game.headed_kept = game.headed;
        }
        if let Some(go) = game.go.take() {
            // The game's thread waits for it.
            let _ = go.send(Go { seed, finder_seed, cap, threads });
        }
        if let Stage::Search(_, plan) = &mut game.stage {
            *plan = Plan::Search(Search::Running(Run::new(threads, now, given)));
        }
    }

    /// Tells the search of game `n` to have `workers` workers from now on (0 suspends its engine).
    fn set_workers(&mut self, n: u32, workers: usize, now: Instant) {
        let Some(Game { stage: Stage::Search(set, Plan::Search(Search::Running(run))), .. }) = self.games.get_mut(&n)
        else {
            return;
        };
        let old = run.target;
        if workers > run.target {
            run.granted = now;
        }
        run.target = workers;
        set.job.control().set_workers(workers);
        self.trace(format_args!("workers n={n} {old}->{workers}"));
    }

    /// Takes in what a game's thread says.
    fn handle(&mut self, event: Event) {
        match event {
            Event::SetUp { n, set_up, took } => {
                let Some(game) = self.games.get_mut(&n) else { return };
                game.own += took;
                game.stage = match set_up {
                    Ok(SetUp::Search(set, problem)) => Stage::Search(Box::new(set), Plan::Unlooked(problem)),
                    Ok(SetUp::Derived(derived)) => Stage::Derived(derived),
                    Err(message) => Stage::Failed(message),
                };
                if !matches!(game.stage, Stage::Search(..)) {
                    // Its thread has ended.
                    game.go = None;
                }
                let kind = match &game.stage {
                    Stage::Search(..) => "search",
                    Stage::Derived(_) => "derived",
                    _ => "failed",
                };
                self.trace(format_args!("setup-done n={n} took={took:?} kind={kind}"));
            }
            Event::Searched { n, outcome, took } => {
                // A search the lab stopped to restart it is started again, unless it completed first.
                if let Some(restart) = self.games.get_mut(&n).and_then(|game| game.restart.take()) {
                    match &outcome {
                        Ok(found) if found.completed => {
                            let why = "the search completed before it stopped: there is nothing to restart";
                            self.answer(&restart.name, restart.json.as_ref(), Err(why.into()));
                        }
                        Ok(_) if engine::stop_signal().is_some() => {
                            let why = "a stop signal ends the run: the search goes on from its checkpoint next time";
                            self.answer(&restart.name, restart.json.as_ref(), Err(why.into()));
                        }
                        Ok(stopped) => {
                            if let Some(game) = self.games.get_mut(&n) {
                                game.own += took;
                            }
                            let restarted = self.restart_now(n, restart.tactics, Some(stopped.stats.nodes));
                            self.answer(&restart.name, restart.json.as_ref(), restarted);
                            self.trace(format_args!("restarted n={n}"));
                            return;
                        }
                        Err(_) => {
                            let why = "the search failed before it stopped, and the game fails";
                            self.answer(&restart.name, restart.json.as_ref(), Err(why.into()));
                        }
                    }
                }
                // A search that stops (a signal, a limit) is resumed with its clock; a finished one's goes with its
                // checkpoint.
                if !matches!(&outcome, Ok(outcome) if outcome.completed) {
                    self.keep_clock(n);
                }
                let Some(game) = self.games.get_mut(&n) else { return };
                game.own += took;
                match (outcome, &mut game.stage) {
                    (Ok(outcome), Stage::Search(_, plan)) => *plan = Plan::Search(Search::Done(outcome)),
                    (Ok(_), _) => {}
                    (Err(message), stage) => *stage = Stage::Failed(message),
                }
                self.trace(format_args!("searched n={n} took={took:?}"));
            }
            Event::Panicked { n } => {
                let Some(game) = self.games.get_mut(&n) else { return };
                game.stage = Stage::Failed(format!("n={n}: the game's thread panicked"));
                game.go = None;
            }
        }
    }

    /// Brings the head's wall clock up to `now` if its search runs (the time since the last look is its own search),
    /// and keeps it in its file once a checkpoint interval has passed since it was last kept.  Does nothing without a
    /// wall.
    fn advance_clock(&mut self, now: Instant) {
        let since = now.saturating_duration_since(self.clock_at);
        self.clock_at = now;
        if !self.clocked() {
            return;
        }
        let n = self.next;
        let Some(game) = self.games.get_mut(&n) else { return };
        if !matches!(game.stage, Stage::Search(_, Plan::Search(Search::Running(_)))) {
            return;
        }
        game.headed += since;
        if game.headed >= game.headed_kept + self.opts.checkpoint_interval {
            self.keep_clock(n);
        }
    }

    /// Whether the driver keeps the head's clock: for the wall, and for a lab (the hook's `long`, and the record of a
    /// restart).
    fn clocked(&self) -> bool {
        self.opts.game_limit.is_some() || self.opts.lab.any()
    }

    /// Writes game `n`'s wall clock to its file, if it has run since it was last written.
    fn keep_clock(&mut self, n: u32) {
        let Some(game) = self.games.get_mut(&n) else { return };
        if game.headed > game.headed_kept {
            game.headed_kept = game.headed;
            clock::keep(self.opts, n, game.headed);
        }
    }

    /// Stops the head's search once its wall clock has reached the limit (see "The wall"): the range ends at the head
    /// when the search has ended, unless it completed first ([`walled`](Self::walled)).
    fn stop_at_wall(&mut self) {
        let Some(limit) = self.opts.game_limit else { return };
        let n = self.next;
        let Some(game) = self.games.get_mut(&n) else { return };
        let Stage::Search(set, Plan::Search(Search::Running(_))) = &game.stage else { return };
        if game.stopped_at_wall || game.headed < limit {
            return;
        }
        set.job.control().stop();
        game.stopped_at_wall = true;
        let held = game.headed;
        self.trace(format_args!("wall n={n} held={held:?}"));
    }

    /// The head, if its search was stopped at the wall and has ended without completing.
    fn walled(&self) -> Option<Wall> {
        let game = self.games.get(&self.next)?;
        let ended = matches!(&game.stage, Stage::Search(_, Plan::Search(Search::Done(outcome))) if !outcome.completed);
        (game.stopped_at_wall && ended).then_some(Wall { n: self.next, held: game.headed })
    }

    /// Samples every running search's engine.
    fn sample(&mut self) {
        for game in self.games.values_mut() {
            if let Stage::Search(set, Plan::Search(Search::Running(run))) = &mut game.stage {
                run.sample(set.job.control().status());
            }
        }
    }
}

/// The lab channel (solver/docs/design/lab.md): the hook's events, and the commands of the folder, each checked against
/// the window and applied or refused.  A command changes how and when the games are searched, never what proves them:
/// the order and share of the threads, how nodes branch, the incumbent through `offer_found`, a search started again
/// from nothing, and which build runs.
impl<'a> Window<'a> {
    /// Tells the hook that the head runs long, once its own search has passed `--hook-after`: once a game, over every
    /// invocation (its record beside the checkpoint keeps that it was told).
    fn tell_long(&mut self) {
        let Some(hook) = &self.opts.lab.hook else { return };
        let (n, opts) = (self.next, self.opts);
        let Some(game) = self.games.get_mut(&n) else { return };
        let Stage::Search(set, Plan::Search(Search::Running(_))) = &mut game.stage else { return };
        if set.lab.long || game.headed < opts.lab.hook_after {
            return;
        }
        set.lab.long = true;
        set.lab.keep(opts, n);
        let status = set.job.control().status();
        let (nodes, best) = (status.map(|s| s.nodes), status.map(|s| s.best));
        lab::spawn_hook(hook, &lab::long_event(n, game.headed, nodes, best, opts));
        self.trace(format_args!("long n={n}"));
    }

    /// Takes the commands in the folder and applies or refuses each, writing its outcome: at each progress tick, on the
    /// driver's thread, never by a worker.
    fn lab_tick<'s>(&mut self, scope: &'s Scope<'s, '_>)
    where
        'a: 's,
    {
        let Some(folder) = self.folder.clone() else { return };
        for taken in folder.take() {
            let outcome = match taken.command {
                Err(why) => Some(Err(why)),
                Ok(command) => self.apply(scope, &folder, &taken.name, taken.json.as_ref(), command),
            };
            // A restart is answered once the search has stopped.
            if let Some(outcome) = outcome {
                self.answer(&taken.name, taken.json.as_ref(), outcome);
            }
        }
    }

    /// Writes the outcome of the command `name`, if the run has a folder.
    fn answer(&self, name: &str, json: Option<&Json>, outcome: Result<String, String>) {
        if let Some(folder) = &self.folder {
            match outcome {
                Ok(reason) => folder.answer(name, json, Answer::Applied, &reason),
                Err(reason) => folder.answer(name, json, Answer::Refused, &reason),
            }
        }
    }

    /// Applies `command`, or says why it is refused; `None` for a restart that waits for its search to stop.
    fn apply<'s>(
        &mut self,
        scope: &'s Scope<'s, '_>,
        folder: &Folder,
        name: &str,
        json: Option<&Json>,
        command: Command,
    ) -> Option<Result<String, String>>
    where
        'a: 's,
    {
        if self.summary.redeploy.is_some() {
            return Some(Err("the run is stopping to be redeployed".into()));
        }
        Some(match command {
            Command::Checkpoint => self.lab_checkpoint(),
            Command::Offer { n, set, value } => self.lab_offer(n, &set, value),
            Command::Branching { n, conflict_branching } => self.lab_branching(n, conflict_branching),
            Command::Restart { n, tactics } => return self.lab_restart(n, &tactics, name, json),
            Command::Threads { total } => self.lab_threads(total),
            Command::Workers { n, workers } => self.lab_workers(n, workers),
            Command::Finder { n, budget } => self.lab_finder(scope, folder, name, n, budget),
            Command::Redeploy { commit } => self.lab_redeploy(commit),
        })
    }

    /// Game `n`'s search, set up and not ended (running or not), or why there is none to act on.
    fn searching(&self, n: u32) -> Result<&SearchSetUp, String> {
        self.in_range(n)?;
        let Some(game) = self.games.get(&n) else {
            let flight: Vec<u32> = self.games.keys().copied().collect();
            return Err(format!("game {n} is not in flight (the window holds {flight:?})"));
        };
        match &game.stage {
            Stage::Search(set, Plan::Unlooked(_) | Plan::Search(Search::NotStarted(_) | Search::Running(_))) => Ok(set),
            Stage::Search(_, Plan::Search(Search::Done(_))) => Err(format!("game {n}'s search has ended")),
            Stage::Search(_, Plan::Repeat(m)) => {
                Err(format!("game {n} repeats the search of {m}, and has none of its own"))
            }
            Stage::Derived(_) => Err(format!("game {n} needs no search")),
            Stage::Waiting | Stage::SettingUp => Err(format!("game {n} is being set up: send it again once it is")),
            Stage::Failed(_) => Err(format!("game {n} has failed")),
        }
    }

    /// Whether `n` is a game of the range not concluded yet, or why not.
    fn in_range(&self, n: u32) -> Result<(), String> {
        if !(self.lo..=self.hi).contains(&n) {
            Err(format!("game {n} is not in the range {}..={}", self.lo, self.hi))
        } else if n < self.next {
            Err(format!("game {n} is concluded"))
        } else {
            Ok(())
        }
    }

    /// `checkpoint`: every running search writes its checkpoint now.
    fn lab_checkpoint(&self) -> Result<String, String> {
        if self.opts.checkpoint_dir.is_none() {
            return Err("the run keeps no checkpoints (no --checkpoint-dir)".into());
        }
        let asked: Vec<u32> = (self.games.iter())
            .filter_map(|(&n, game)| match &game.stage {
                Stage::Search(set, Plan::Search(Search::Running(_))) => set.job.control().checkpoint().then_some(n),
                _ => None,
            })
            .collect();
        Ok(match asked.is_empty() {
            true => "no search is running".into(),
            false => format!("the searches of {asked:?} write their checkpoints"),
        })
    }

    /// `offer`: the set goes to game `n`'s search through `offer_found`, which refuses a set that cannot be promoted
    /// or is not worth what is claimed.
    fn lab_offer(&self, n: u32, set: &[u32], value: Option<u64>) -> Result<String, String> {
        let job = &self.searching(n)?.job;
        match job.offer(set, value) {
            Ok(true) => Ok(format!("it is game {n}'s incumbent")),
            Ok(false) => {
                let best = job.control().best().map_or(0, |(best, _)| best);
                Ok(format!("it is not better than game {n}'s incumbent, worth {best}: nothing changes"))
            }
            Err(why) => Err(format!("offer_found refused it: {why}")),
        }
    }

    /// `branching`: game `n`'s searchers branch with `conflict_branching` on or off from the next unit each starts.
    fn lab_branching(&mut self, n: u32, on: bool) -> Result<String, String> {
        self.searching(n)?;
        let opts = self.opts;
        let Some(Game { stage: Stage::Search(set, _), .. }) = self.games.get_mut(&n) else { unreachable!("searching") };
        set.job.branching().set_conflict_branching(on);
        set.lab.conflict_branching = Some(on);
        set.lab.branching_changes += 1;
        set.lab.keep(opts, n);
        let unused = if set.job.tactics().gf2 { "" } else { " (it acts only with gf2, which this search has off)" };
        let setting = if on { "on" } else { "off" };
        Ok(format!(
            "game {n}'s searchers branch with conflict_branching {setting} from the next unit each starts{unused}"
        ))
    }

    /// `restart`: game `n`'s search is abandoned and started again from nothing, with the run's search tactics changed
    /// by `assignments`.  A running search is stopped first, with a final checkpoint, and restarted when it has
    /// stopped (`None`: answered then); one not started is restarted at once.
    fn lab_restart(
        &mut self,
        n: u32,
        assignments: &[String],
        name: &str,
        json: Option<&Json>,
    ) -> Option<Result<String, String>> {
        if engine::stop_signal().is_some() {
            return Some(Err("a stop signal ends the run: nothing new starts".into()));
        }
        let tactics = match lab::restart_tactics(&self.opts.tactics, assignments) {
            Ok(tactics) => tactics,
            Err(why) => return Some(Err(why)),
        };
        if let Err(why) = self.searching(n) {
            return Some(Err(why));
        }
        let game = self.games.get_mut(&n)?;
        if game.restart.is_some() {
            return Some(Err(format!("game {n} is being restarted already")));
        }
        if game.stopped_at_wall {
            return Some(Err(format!("game {n} has passed the wall")));
        }
        let Stage::Search(set, Plan::Search(Search::Running(_))) = &game.stage else {
            return Some(self.restart_now(n, tactics, None));
        };
        set.job.control().stop();
        game.restart = Some(Restart { name: name.to_owned(), json: json.cloned(), tactics });
        self.trace(format_args!("restart n={n} stopping"));
        None
    }

    /// Starts game `n` again from nothing with the search tactics `tactics`: its checkpoint and clock are put aside in
    /// the checkpoint directory (never deleted), the restart is recorded beside it, and the game goes back to its setup,
    /// which searches it whatever an earlier game settled.  `nodes`: the steps of the search abandoned, if known.  If
    /// the checkpoint cannot be put aside, the game goes on from it, not restarted.
    fn restart_now(&mut self, n: u32, tactics: SearchTactics, nodes: Option<u64>) -> Result<String, String> {
        let opts = self.opts;
        let game = self.games.get_mut(&n).expect("a game in flight");
        let Stage::Search(set, _) = &game.stage else { unreachable!("a restart is of a search") };
        let mut record = GameLab::clone(&set.lab);
        let k = record.restarts.len() + 1;
        let aside = |path: Option<PathBuf>, ext: &str| -> Result<Option<String>, String> {
            let Some(path) = path.filter(|path| path.exists()) else { return Ok(None) };
            let name = format!("n{n:04}.restart{k}.{ext}");
            fs::rename(&path, path.with_file_name(&name)).map_err(|e| format!("{}: {e}", path.display()))?;
            Ok(Some(name))
        };
        let back = |game: &mut Game, record: GameLab| {
            game.stage = Stage::Waiting;
            game.go = None;
            game.relab = Some(record);
        };
        let put = match aside(opts.checkpoint_file(n), "ckpt") {
            Ok(put) => put,
            Err(why) => {
                // Set up again as it was: a search the lab stopped resumes from its checkpoint.
                back(game, record);
                return Err(format!("cannot put game {n}'s checkpoint aside ({why}): it goes on from its checkpoint"));
            }
        };
        if let Err(why) = aside(opts.clock_file(n), "clock") {
            lab::say(format_args!("the clock of n={n} stays where it was: {why}"));
        }
        let described = Tactics { search: tactics, ..opts.tactics }.description();
        record.restarts.push(Restarted {
            at: lab::utc(SystemTime::now()),
            own_seconds: game.headed.as_secs_f64(),
            nodes,
            tactics: described.clone(),
            aside: put.clone(),
        });
        record.tactics = Some(tactics);
        // The restart's tactics say how it branches, until the lab says otherwise.
        record.conflict_branching = None;
        record.keep(opts, n);
        // A search started again starts its clock again ("The wall").
        (game.headed, game.headed_kept, game.stopped_at_wall) = (Duration::ZERO, Duration::ZERO, false);
        back(game, record);
        let kept = put.map_or_else(|| "it had no checkpoint".to_owned(), |name| format!("its checkpoint is {name}"));
        Ok(format!("game {n} starts again from nothing with the tactics {described}; {kept}"))
    }

    /// `threads {total}`: the run's budget of threads.
    fn lab_threads(&mut self, total: usize) -> Result<String, String> {
        let pinned: usize = self.pins.values().sum();
        if total < pinned {
            return Err(format!("the games whose workers the lab set hold {pinned} of them"));
        }
        let old = mem::replace(&mut self.threads, total);
        Ok(format!("the run's threads go from {old} to {total}"))
    }

    /// `threads {n, workers}`: game `n`'s search keeps `workers` workers, outside the policy, until it is concluded or
    /// the lab gives it back to the policy (`None`).  0 suspends a running search, and keeps one not started waiting.
    fn lab_workers(&mut self, n: u32, workers: Option<usize>) -> Result<String, String> {
        self.in_range(n)?;
        let Some(workers) = workers else {
            return Ok(match self.pins.remove(&n) {
                Some(_) => format!("game {n}'s workers are the policy's again"),
                None => format!("game {n}'s workers were the policy's"),
            });
        };
        let others: usize = self.pins.iter().filter(|&(&m, _)| m != n).map(|(_, &w)| w).sum();
        if others + workers > self.threads {
            return Err(format!("the run has {} threads, and the other games the lab set hold {others}", self.threads));
        }
        self.pins.insert(n, workers);
        Ok(format!("game {n}'s search keeps {workers} of the run's threads, outside the policy"))
    }

    /// `finder`: a finder beside game `n`'s running search for `budget`, which offers what it finds through
    /// `offer_found` and ends with the search; its report goes to `out/` as `report-NAME`.
    fn lab_finder<'s>(
        &mut self,
        scope: &'s Scope<'s, '_>,
        folder: &Folder,
        name: &str,
        n: u32,
        budget: Duration,
    ) -> Result<String, String>
    where
        'a: 's,
    {
        self.searching(n)?;
        let seed = match self.known_before(n) {
            Known::Final(prev) => Some(prev.moves.clone()),
            Known::Provisional(before) => Some(before.moves()),
            Known::NotYet | Known::None => None,
        };
        self.finders += 1;
        let random = self.finders;
        let Some(Game { stage: Stage::Search(set, Plan::Search(Search::Running(run))), .. }) = self.games.get_mut(&n)
        else {
            return Err(format!("game {n}'s search is not running"));
        };
        let (job, (stop, ended)) = (Arc::clone(&set.job), mpsc::channel::<()>());
        let (folder, report) = (folder.clone(), format!("report-{name}"));
        let out = report.clone();
        let spawned = thread::Builder::new().name(format!("finder-{n}")).spawn_scoped(scope, move || {
            let found = job.find(budget, seed.as_deref(), random, &ended);
            let json = Json::object([
                ("n", Json::from(n)),
                ("from", Json::from(found.from)),
                ("best", Json::from(found.best)),
                ("improved", Json::from(found.improved)),
                ("refused", Json::from(found.refused)),
                ("seconds", Json::Num(found.seconds)),
                ("found_seconds", Json::Num(found.found)),
            ]);
            folder.write(&out, &json);
        });
        spawned.map_err(|e| format!("cannot start a thread for the finder: {e}"))?;
        run.finders.push(stop);
        let seconds = budget.as_secs_f64();
        Ok(format!("a finder runs beside game {n}'s search for {seconds:.1} s; its report will be out/{report}"))
    }

    /// `redeploy`: the range ends here, every search stopped with a final checkpoint, and `solve` exits with the status
    /// that tells the chain's script to build `commit` and run it.
    fn lab_redeploy(&mut self, commit: String) -> Result<String, String> {
        if self.opts.checkpoint_dir.is_none() {
            return Err("the run keeps no checkpoints (no --checkpoint-dir): its searches would be lost".into());
        }
        let said = format!("every search stops with a final checkpoint, and the run ends to be redeployed at {commit}");
        self.summary.redeploy = Some(commit);
        Ok(said)
    }
}

/// Ends the range, however it ends (with a panic of the driver unwinding too): stops every search in flight and
/// cancels the games that wait for their go (see "Stopping"), so that every game's thread ends and can be joined.
impl Drop for Window<'_> {
    fn drop(&mut self) {
        for game in self.games.values() {
            if let Stage::Search(set, Plan::Search(Search::Running(_))) = &game.stage {
                set.job.control().stop();
            }
            // A restart that waits for its search to stop is not made: the search resumes from its checkpoint.
            if let Some(restart) = &game.restart {
                let why = "the run ended before the search stopped: it goes on from its checkpoint, not restarted";
                self.answer(&restart.name, restart.json.as_ref(), Err(why.into()));
            }
        }
        self.games.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(millis: u64) -> Duration {
        Duration::from_millis(millis)
    }

    /// An engine's status: `live` workers, `idle` of them waiting for work, over lifetimes of `capacity` ms in all of
    /// which they waited `idle_time` ms.
    fn status(live: usize, idle: usize, capacity: u64, idle_time: u64) -> Status {
        let (capacity, idle_time) = (ms(capacity), ms(idle_time));
        Status { live, running: live - idle, idle, queued: 0, idle_time, capacity, best: 0, nodes: 0 }
    }

    fn game(score: u64, proven: bool) -> Prev {
        Prev {
            moves: Vec::new(),
            score,
            source: super::super::PrevSource::Run,
            proof: None,
            assumes: Default::default(),
            proven,
        }
    }

    #[test]
    fn an_unfinished_game_is_tried_again_when_the_game_before_it_has_more_to_give() {
        let on = |score, proven| Some(BuiltOn { score, proven });
        // Nothing before it now: nothing new, whatever it had.
        assert!(!improved(None, None) && !improved(None, on(5, false)));
        // It had no game before it, and now there is one, proven or not.
        assert!(improved(Some(&game(5, false)), None) && improved(Some(&game(5, true)), None));
        // It had a proven game: nothing can be more.
        assert!(!improved(Some(&game(5, true)), on(5, true)) && !improved(Some(&game(9, true)), on(5, true)));
        // It had an unproven game: a proven one is new, and an unproven one if it scores more.
        assert!(improved(Some(&game(5, true)), on(5, false)));
        assert!(improved(Some(&game(6, false)), on(5, false)));
        assert!(!improved(Some(&game(5, false)), on(5, false)) && !improved(Some(&game(4, false)), on(5, false)));
    }

    #[test]
    fn a_tick_in_which_the_engine_had_no_worker_has_no_idle_share() {
        let now = Instant::now();
        let mut run = Run::new(1, now, None);
        run.sample(Some(status(1, 0, 10, 0)));
        run.target = 0;
        run.sample(Some(status(0, 0, 10, 0)));
        // Suspended, and not saturated: it claims one thread, not as many again as it has.
        let need = run.need(now);
        assert_eq!((need.load, need.target, need.live), (Load::Unknown, 0, 0));
        assert_eq!(allot(4, &[Need::Running(need)]), [(0, Grant::Workers(1))]);
        // Started again on one worker, busy over its first tick with it: saturated, it asks one more.
        run.target = 1;
        run.sample(Some(status(1, 0, 15, 0)));
        assert_eq!(allot(4, &[Need::Running(run.need(now))]), [(0, Grant::Workers(2))]);
    }
}
