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
//! 5. After a conclusion, the next game, if its search runs, is given its ceiling ([`Control::set_ceiling`]), the seed
//!    of the final game ([`Job::hint`]), and, if the game concluded assumes nothing, the near-repeat cut by it
//!    ([`Job::arm`]; the three in [`told`]); one not started yet gets all three with its go.  The cut is set up
//!    on the driver's thread, and each searcher picks it up when it next starts a unit, as it reads the branching
//!    settings; the nodes searched before are only not cut, which changes no answer.  The cut acts only where the
//!    incumbent reaches its bound B, the best promotion sum of the game concluded.  A search seeded with a provisional
//!    game of that game may be below B when it is armed; the seed of the final game, for a search that takes a seed,
//!    or the search's own finds raise it.
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
//! * Seeds and hints from a provisional game, offers at a conclusion, ceilings given late, near-repeat cuts armed late
//!   and deferrals change how or when a search runs, never the promotion sum it proves best: every set offered is
//!   promotable (the seed's survivors), the ceiling is proven given the game of `n − 1` optimal, which a proof by the
//!   ceiling rests on anyway, and the cut drops only nodes with nothing better than the incumbent, by a game that is
//!   proven and assumes nothing (the near-repeat lemma (R7, docs/theory/search-rules.md) and its standing).
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
//! run resumes it, and what it found is not recorded), nothing new starts, a game that waits for its go is canceled,
//! the setups that run finish, and every thread is joined before [`solve`] returns.  After a stop signal every engine
//! stops on its own, and no search starts but the head's: started, it stops at once with its starting units in its
//! checkpoint, the incomplete line a search started after the signal gives in the range solved in order, and the
//! range ends there.  Time and node limits are each search's own, per game and invocation, as always.
//!
//! # The clock
//!
//! A game's own search is the time it has been the head of the window with its search running, summed over the
//! invocations that searched it; its line records it as `seconds.own`.  The chain waits for the head and for no other
//! game; a search behind it is suspended, or shares the threads that the head leaves free, so the time it spends there
//! is not what the game costs, and the search's elapsed time in its checkpoint (which counts it) would overstate the
//! cost of a game that follows a dear one.  The driver keeps the head's clock ([`clock`](super::clock)): it adds the
//! time between its looks while the head's search runs, writes it beside the game's checkpoint every checkpoint
//! interval and when the search ends without completing, and reads it back when a search starts with a checkpoint to
//! resume, so that a spot interruption, a new machine or a rerun goes on from it.
//!
//! At each look the driver counts the time since the last look as the head's search before it takes in the event that
//! woke it, so a search whose end arrives is counted up to the moment the driver wakes, however late that is.
//!
//! # Tracing
//!
//! With the environment variable `TAXMAN_WINDOW_TRACE` set, the driver prints a line on stderr for each of its events,
//! the seconds since the range began first: a setup started and done, a repeat found, a search started (its threads,
//! whether it has a seed, its ceiling), a search's workers changed (`workers n=N a->b`: given, given back, reclaimed,
//! or suspended at 0), a hint, a deferral, a search done (with the nodes its near-repeat cut cut, if it had one), what
//! a conclusion told the next game's running search (its ceiling, the bound of the near-repeat cut if it was armed,
//! and how long that took), a game concluded.  It is how the policy's decisions were followed on the laptop, and how
//! they are to be followed on a machine of many threads.
//!
//! [`Control::set_ceiling`]: crate::engine::Control::set_ceiling
//! [`Control::stop`]: crate::engine::Control::stop

mod allot;

use super::{
    ceiling, clock, conclude_game, discard_checkpoint, search_cap, set_up, shared_spill_dir, stored_prev, Before,
    Context, Derived, Earlier, Failure, Found, GameResult, Go, Job, Outcome, Prev, RangeSummary, SearchSetUp, SetUp,
    SolveOptions,
};
use crate::accelerate::prime_shortcut;
use crate::engine::{self, Status, WORKER_STACK_SIZE};
use crate::game::Optimal;
use crate::known::Known as KnownGames;
use crate::results::ResultsFile;
use crate::search::Problem;
use allot::{allot, Grant, Load, Need, TICK};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::mem;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, Scope};
use std::time::{Duration, Instant};

/// A range of games for [`solve`].
pub(super) struct Range<'a> {
    pub(super) lo: u32,
    pub(super) hi: u32,
    /// The game of `lo − 1`.
    pub(super) before: Before,
    /// The known sources: the table the scores are confirmed against, and, with `--use-prev`, what the games draw on.
    pub(super) known: &'a KnownGames<'a>,
    /// The results file: its completed games are skipped, and the others appended to it.
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
    let summary = thread::scope(|scope| {
        let (events, received) = mpsc::channel();
        // Dropped when the closure returns or unwinds, before the threads are joined: that ends the range (`Drop`).
        let mut window = Window::new(range, opts, earlier, events);
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

/// A game's thread: its setup, then, for a search, its search once the driver says go, unless the game is canceled
/// first (the driver drops the sender of the go).
fn play(n: u32, before: &Before, opts: &SolveOptions, known: &KnownGames, events: &Sender<Event>, go: &Receiver<Go>) {
    let _unwinding = Unwinding { n, events };
    let start = Instant::now();
    let set_up = set_up(n, before, opts, known).map_err(Failure::message);
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
    /// Its clock, its own search time: the time it has been the head with its search running, over every invocation
    /// (see "The clock").  The game's line (`seconds.own`) reads it.
    headed: Duration,
    /// What was last written to its clock file.
    headed_kept: Duration,
    /// The sender of its go while its thread may wait for one; dropping it cancels the game's search.
    go: Option<Sender<Go>>,
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
}

impl Run {
    fn new(target: usize, now: Instant, given: Option<Given>) -> Run {
        // Just given its threads: nothing to act on until the next sample.
        Run { target, granted: now, last: None, load: Load::Starting { fresh: false }, given }
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
}

impl<'a> Window<'a> {
    fn new(range: Range<'a>, opts: &'a SolveOptions, earlier: &'a mut Earlier, events: Sender<Event>) -> Self {
        // With one thread there is none to run a second game on.
        let width = if opts.threads <= 1 { 1 } else { opts.window.max(1) };
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
            // The time since the last look was spent with the head's search as it was before this event, which may be
            // its end: the clock is brought up to date before the event is taken in.
            let now = Instant::now();
            self.advance_clock(now);
            if let Some(event) = event {
                self.handle(event);
            }
            if now >= tick {
                self.sample();
                self.hint();
                tick = now + TICK;
            }
        }
        Ok(())
    }

    /// Whether the results file has completed game `n`.
    fn in_file(&self, n: u32) -> bool {
        self.results.as_deref().is_some_and(|file| file.completed(n).is_some())
    }

    /// Skips the games of the results file at the head, each the previous game of the one after it.
    fn skip_settled(&mut self) {
        while self.next <= self.hi {
            let Some(stored) = self.results.as_deref().and_then(|file| stored_prev(file, self.next)) else { break };
            self.summary.skipped += 1;
            self.last = Some(stored);
            self.next += 1;
        }
    }

    /// The previous game of game `n` as its setup knows it.
    fn before(&self, n: u32) -> Before {
        if n == self.lo {
            return self.first.clone();
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
                    go: None,
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
            // Not for a search that a checkpoint resumes (searched, never replaced), nor with `force_search`.
            let repeated = if repeats && !set.job.resuming() { self.earlier.find(n, &problem) } else { None };
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
            self.enter();
            self.look_up();
            self.skip_settled();
            if self.next > self.hi {
                return Ok(true);
            }
            let n = self.next;
            let Some(result) = self.conclude_head(n)? else { return Ok(false) };
            if let Some(file) = self.results.as_deref_mut() {
                file.append(&result)?;
            }
            on_game(&result);
            self.trace(format_args!("concluded n={n}"));
            self.summary.solved += 1;
            if !result.completed {
                // Its checkpoint stays, for the next run.
                self.summary.stopped_at = Some(n);
                return Ok(true);
            }
            discard_checkpoint(self.opts, n);
            self.earlier.add_result(&result);
            let prev = Prev::from_result(&result);
            self.tell(n + 1, &prev);
            self.last = Some(prev);
            // The conclusion was the old head's: the next head's clock counts from here.
            self.advance_clock(Instant::now());
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
        let context = Context {
            prev: self.last.as_ref(),
            optimal: self.optimal,
            own: game.own,
            own_search: game.headed,
            since: Instant::now(),
        };
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

    /// Gives game `n`, if its search runs, what the conclusion of its previous game `prev` gives ([`told`]).  (A search
    /// not started gets it with its go.)  A cut that cannot be set up is said on stderr, and the search goes on
    /// without it.
    fn tell(&self, n: u32, prev: &Prev) {
        let Some(Game { stage: Stage::Search(set, Plan::Search(Search::Running(_))), .. }) = self.games.get(&n) else {
            return;
        };
        let start = Instant::now();
        let (cap, armed) = told(set, n, prev, self.opts);
        let bound = armed.unwrap_or_else(|e| {
            let _ = writeln!(std::io::stderr(), "warning: n={n}: the near-repeat cut is not armed: {e}");
            None
        });
        self.trace(format_args!("told n={n} cap={cap:?} near_repeat={bound:?} took={:?}", start.elapsed()));
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

    /// What the games hold and claim, oldest first, for [`allot()`]: each game's n and its need.
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
        self.games.iter().map(|entry| (*entry.0, need(entry))).collect()
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

    /// Weighs the searches not started for deferring, then shares the run's threads ([`allot()`]) and carries out the
    /// grants.
    fn share<'s>(&mut self, scope: &'s Scope<'s, '_>)
    where
        'a: 's,
    {
        self.weigh();
        let now = Instant::now();
        let needs = self.needs(now);
        let grants = allot(self.opts.threads, &needs.iter().map(|&(_, need)| need).collect::<Vec<_>>());
        for (i, grant) in grants {
            let n = needs[i].0;
            match grant {
                Grant::SetUp => self.start_setup(scope, n),
                Grant::Start(threads) => self.start_search(n, threads, now),
                Grant::Workers(workers) => self.set_workers(n, workers, now),
            }
        }
        for game in self.games.values_mut() {
            if let Stage::Search(_, Plan::Search(Search::Running(run))) = &mut game.stage {
                run.used();
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
        // The setup and the search's start ran on the main thread when games were solved one at a time; the stack is
        // reserved address space, committed only as touched.
        let spawned = thread::Builder::new()
            .name(format!("game-{n}"))
            .stack_size(WORKER_STACK_SIZE)
            .spawn_scoped(scope, move || play(n, &before, opts, known, &events, &waits));
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
                (seeds.then(|| prev.moves.clone()), search_cap(set, n, Some(prev.score), self.opts), None)
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
        // The head's own search begins here: the clock counts from now, not from its last look.
        if n == self.next {
            self.advance_clock(now);
        }
        // The game before it, if it is concluded: the game of the near-repeat cut (`near_repeat`).  A search started
        // before then does not cut by it.
        let concluded = match self.known_before(n) {
            Known::Final(prev) => Some(prev.clone()),
            _ => None,
        };
        let Some(game) = self.games.get_mut(&n) else { return };
        // A search that resumes from its checkpoint goes on from the clock the last invocation kept.
        game.headed = clock::resume(self.opts, n);
        game.headed_kept = game.headed;
        if let Some(go) = game.go.take() {
            // The game's thread waits for it.
            let _ = go.send(Go { seed, finder_seed, cap, threads, concluded });
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
                // A search that stops (a signal, a limit) is resumed with its clock; a finished one's goes with its
                // checkpoint.
                if !matches!(&outcome, Ok(outcome) if outcome.completed) {
                    self.keep_clock(n);
                }
                let Some(game) = self.games.get_mut(&n) else { return };
                game.own += took;
                let cut = outcome.as_ref().ok().and_then(|outcome| outcome.near_repeat_cut);
                match (outcome, &mut game.stage) {
                    (Ok(outcome), Stage::Search(_, plan)) => *plan = Plan::Search(Search::Done(outcome)),
                    (Ok(_), _) => {}
                    (Err(message), stage) => *stage = Stage::Failed(message),
                }
                self.trace(format_args!("searched n={n} took={took:?} near_repeat_cut={cut:?}"));
            }
            Event::Panicked { n } => {
                let Some(game) = self.games.get_mut(&n) else { return };
                game.stage = Stage::Failed(format!("n={n}: the game's thread panicked"));
                game.go = None;
            }
        }
    }

    /// Brings the head's clock up to `now` if its search runs (the time since the last look is its own search), and
    /// keeps it in its file once a checkpoint interval has passed since it was last kept.
    fn advance_clock(&mut self, now: Instant) {
        let since = now.saturating_duration_since(self.clock_at);
        self.clock_at = self.clock_at.max(now);
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

    /// Writes game `n`'s clock to its file, if it has run since it was last written.
    fn keep_clock(&mut self, n: u32) {
        let Some(game) = self.games.get_mut(&n) else { return };
        if game.headed > game.headed_kept {
            game.headed_kept = game.headed;
            clock::keep(self.opts, n, game.headed);
        }
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

/// What the conclusion of game `n − 1`, as `prev`, gives the search of game `n`, set up as `set`, whether it runs or
/// waits for its go (step 5 of the module docs): its ceiling ([`search_cap`]), the seed of the final game as a hint
/// ([`Job::hint`]), and the near-repeat cut by `prev`, where the search has the tactic and `prev` assumes
/// nothing ([`Job::arm`]).  Returns the ceiling given, if any, and the cut's bound if this armed the cut, or why it could
/// not be set up.
fn told(set: &SearchSetUp, n: u32, prev: &Prev, opts: &SolveOptions) -> (Option<u64>, Result<Option<u64>, String>) {
    let cap = search_cap(set, n, Some(prev.score), opts);
    if let Some(cap) = cap {
        set.job.control().set_ceiling(cap);
    }
    set.job.hint(&prev.moves);
    (cap, set.job.arm(prev))
}

/// Ends the range, however it ends (with a panic of the driver unwinding too): stops every search in flight and
/// cancels the games that wait for their go (see "Stopping"), so that every game's thread ends and can be joined.
impl Drop for Window<'_> {
    fn drop(&mut self) {
        for game in self.games.values() {
            if let Stage::Search(set, Plan::Search(Search::Running(_))) = &game.stage {
                set.job.control().stop();
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
        Status { live, running: live - idle, idle, queued: 0, idle_time, capacity, best: 0, nodes: 0, all_nodes: 0 }
    }

    /// A near repeat: game 1083 poses nearly the problem of 1082, and the near-repeat cut by 1082 shortens its search
    /// (`tests/tripwires.rs`, `NEAR_REPEATS`).
    const NEAR: u32 = 1083;

    /// Game `n` as the run concludes a search of it: the table's game, assuming nothing.
    fn concluded(n: u32) -> Prev {
        let optimal = Optimal::built_in();
        Prev {
            moves: optimal.moves(n).expect("a game of the table").to_vec(),
            score: optimal.score(n).expect("a game of the table"),
            source: super::super::PrevSource::Run,
            proof: Some(crate::results::Proof::Exhausted),
            assumes: Default::default(),
        }
    }

    /// Game `n` set up for its search as the window sets it up, with the game before it `prev`, under `opts`.
    fn search_set_up(n: u32, prev: &Prev, opts: &SolveOptions) -> SearchSetUp {
        let known = KnownGames::table(Optimal::built_in());
        match set_up(n, &Before::Given(prev.clone()), opts, &known) {
            Ok(SetUp::Search(set, _)) => set,
            Ok(SetUp::Derived(_)) => panic!("n={n} needs no search"),
            Err(failure) => panic!("n={n}: {}", failure.message()),
        }
    }

    /// The search of game `n`, set up as `set`, as the window runs it beside the game before it: started from `prev` as
    /// a provisional game, without a ceiling and without the near-repeat cut, and held at no worker once its engine runs
    /// until it has been told of the conclusion of that game, as `prev` ([`told`]); then let go on one worker.  What it
    /// was told, and its outcome.
    fn searched_beside(set: &SearchSetUp, n: u32, prev: &Prev, opts: &SolveOptions) -> (Told, Outcome) {
        let control = set.job.control();
        control.set_workers(0);
        thread::scope(|scope| {
            let go = Go { seed: Some(prev.moves.clone()), finder_seed: None, cap: None, threads: 1, concluded: None };
            let search = scope.spawn(|| set.job.search(go, opts));
            // Its engine runs, with no worker: nothing of the tree has been searched.
            let since = Instant::now();
            while control.status().is_none() {
                assert!(since.elapsed() < Duration::from_secs(30), "n={n}: the search's engine never runs");
                thread::sleep(ms(1));
            }
            let told = told(set, n, prev, opts);
            control.set_workers(1);
            (told, search.join().expect("the search").expect("an outcome"))
        })
    }

    /// What [`told`] says it gave a search: the ceiling, and the near-repeat cut's bound if it armed the cut.
    type Told = (Option<u64>, Result<Option<u64>, String>);

    /// A search that runs when the game before it is concluded is armed with the near-repeat cut by that game, then
    /// (step 5 of the module docs): game 1083, searched beside 1082, held before its first step until the conclusion of
    /// 1082 has armed it, cuts, and comes back with the table's score, as it does without the tactic, which arms
    /// nothing, in more steps.  B is the score of 1082 less its `in_the_bag`, and the cut is armed once.
    #[test]
    fn a_running_search_is_armed_with_the_near_repeat_cut_when_the_game_before_it_is_concluded() {
        let (prev, optimal) = (concluded(NEAR - 1), Optimal::built_in());
        let bound = prev.score - crate::board::search_problem(NEAR - 1).unwrap().1;
        let on = SolveOptions { threads: 1, ..SolveOptions::default() };
        let mut off = on.clone();
        off.tactics.search.near_repeat = false;
        let mut outcomes = Vec::new();
        for (opts, armed) in [(&on, Some(bound)), (&off, None)] {
            let set = search_set_up(NEAR, &prev, opts);
            let ((cap, told), outcome) = searched_beside(&set, NEAR, &prev, opts);
            assert!(cap.is_some(), "the ceiling is told");
            assert_eq!(told, Ok(armed), "near_repeat {}", opts.tactics.search.near_repeat);
            // Armed once: the go of a search, or a second conclusion, arms nothing more.
            assert_eq!(set.job.arm(&prev), Ok(None));
            assert!(outcome.completed);
            assert_eq!(Some(outcome.moves.iter().map(|&x| u64::from(x)).sum()), optimal.score(NEAR));
            outcomes.push(outcome);
        }
        let (cut, uncut) = (&outcomes[0], &outcomes[1]);
        assert!(cut.near_repeat_cut.is_some_and(|nodes| nodes > 0), "the cut cut nothing: {:?}", cut.near_repeat_cut);
        assert_eq!(uncut.near_repeat_cut, None, "a search without the tactic is never armed");
        assert!(cut.stats.nodes < uncut.stats.nodes, "{} steps with the cut, {}", cut.stats.nodes, uncut.stats.nodes);
    }

    /// The near-repeat cut is armed only by a game that assumes nothing, only with the tactic, and only once: not by a
    /// game assumed optimal; a search armed with its go is not armed again by a conclusion.
    #[test]
    fn the_near_repeat_cut_is_armed_once_by_a_game_that_assumes_nothing() {
        let prev = concluded(NEAR - 1);
        let bound = prev.score - crate::board::search_problem(NEAR - 1).unwrap().1;
        let opts = SolveOptions { threads: 1, ..SolveOptions::default() };
        let set = search_set_up(NEAR, &prev, &opts);
        let assumed = Prev { assumes: [crate::results::assumed_optimal(NEAR - 1)].into(), ..prev.clone() };
        assert_eq!(set.job.arm(&assumed), Ok(None));
        assert_eq!(set.job.arm(&prev), Ok(Some(bound)));
        assert_eq!(set.job.arm(&prev), Ok(None));
        // Armed with its go, as a search that starts once the game before it is concluded is, then told of that game.
        let set = search_set_up(NEAR, &prev, &opts);
        let go = Go {
            seed: Some(prev.moves.clone()),
            finder_seed: None,
            cap: None,
            threads: 1,
            concluded: Some(prev.clone()),
        };
        let outcome = set.job.search(go, &opts).unwrap();
        assert!(
            outcome.completed && outcome.near_repeat_cut.is_some_and(|nodes| nodes > 0),
            "{:?}",
            outcome.near_repeat_cut
        );
        assert_eq!(told(&set, NEAR, &prev, &opts).1, Ok(None));
        // Without the tactic, neither the go nor the conclusion arms it.
        let mut off = opts.clone();
        off.tactics.search.near_repeat = false;
        let set = search_set_up(NEAR, &prev, &off);
        assert_eq!(set.job.arm(&prev), Ok(None));
        let go = Go {
            seed: Some(prev.moves.clone()),
            finder_seed: None,
            cap: None,
            threads: 1,
            concluded: Some(prev.clone()),
        };
        assert_eq!(set.job.search(go, &off).unwrap().near_repeat_cut, None);
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
