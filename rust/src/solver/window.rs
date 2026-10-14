//! The window: the games of a range in flight at once, and the driver that runs them ([`solve`]).
//!
//! A game of the chain needs two things of the game before it: its moves, for the seed, and its score, for the
//! ceiling.  Everything else (its board, its frames, its search problem and whether an earlier game settled that
//! problem) is its own, and the ceiling only stops a search early.  So a range keeps a *window* of up to K consecutive
//! games in flight ([`SolveOptions::window`], `--window K`): game `n + 1` is set up while `n` searches, its search
//! starts from `n`'s best game so far, without a ceiling, and takes the ceiling `n + 1 + opt(n) − in_the_bag` and the
//! seed of `n`'s final game the moment `n` is concluded.  The threads are moved between the searches while they run
//! (`allot`).  With K = 1, and always with one thread, this is the range solved in order: a game enters the window
//! only once the one before it is concluded, so its search starts with the final seed and the ceiling, on every
//! thread, and the lines are those of a loop over the games.
//!
//! # The phases of a game
//!
//! 1. *Setup*, on a thread of the game's own, which holds one of the run's threads ([`set_up`]): the checks of a
//!    previous game not made by this run, the tables, the checkpoint; then the prime shortcut, the trivial game, or the
//!    search problem with its [`Job`].
//! 2. *Repeat lookup*, on the driver's thread, in the order of the board sizes, once the setups of all the earlier
//!    games of the window are done ([`Earlier::find`]).  A game of the window that set a search up and is not
//!    concluded counts as a candidate, as the documentation of `find` argues, so a game repeats exactly the game it
//!    repeats when the range is solved in order.  A repeat waits for the game it repeats, which is older and so is
//!    concluded first.
//! 3. *Search*, on the game's thread, when the driver says go ([`Go`]): with the seed and the ceiling known then, and
//!    the workers the driver grants it.  Its outcome goes back to the driver; the checkpoint stays.
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
//! resumed) or has no previous game; it never waits for that game to be proven.  The game before it improving its
//! best game later is not passed on until its conclusion: passing on each improvement is a possible refinement.
//!
//! # Provenance
//!
//! The lines are those of the range solved in order, but for what a multi-threaded search may vary:
//!
//! * A line is appended only after every line it rests on: that of `n − 1` (its `prev`) and, for a repeat, that of
//!   the game it repeats.  Both are earlier, and lines are appended in the order of the board sizes.
//! * `prev`, `same_search` and `assumes` are derived as the range solved in order derives them, from the concluded
//!   games; the repeat names the same game (above).
//! * The proofs are of the same kinds.  A completed search's promotion sum is its problem's best, and its proof is by
//!   the ceiling exactly when that best reaches the ceiling, however the search ran: a search started before its
//!   ceiling was known and stopped (by a limit or a signal) with its best at the ceiling is proven by it, as it would
//!   have stopped there with the ceiling given from the start.
//! * Seeds from a provisional game, offers at a conclusion and ceilings given late change how a search runs, never
//!   the promotion sum it proves best: every set offered is promotable (the seed's survivors), and the ceiling is
//!   proven given the game of `n − 1` optimal, which a proof by the ceiling rests on anyway.
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
//! [`Control::set_ceiling`]: crate::engine::Control::set_ceiling
//! [`Control::stop`]: crate::engine::Control::stop

mod allot;

use super::{
    ceiling, conclude_game, discard_checkpoint, set_up, shared_spill_dir, stored_prev, Before, Context, Derived,
    Earlier, Failure, Found, GameResult, Go, Job, Outcome, Prev, RangeSummary, SearchSetUp, SetUp, SolveOptions,
};
use crate::accelerate::prime_shortcut;
use crate::engine::{self, Status, WORKER_STACK_SIZE};
use crate::game::Optimal;
use crate::results::ResultsFile;
use crate::search::Problem;
use allot::{allot, Grant, Load, Need, TICK};
use std::collections::BTreeMap;
use std::fs;
use std::mem;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, Scope};
use std::time::{Duration, Instant};

/// A range of board sizes for [`solve`].
pub(super) struct Range<'a> {
    pub(super) lo: u32,
    pub(super) hi: u32,
    /// The game of `lo − 1`.
    pub(super) before: Before,
    /// The table the scores are confirmed against.
    pub(super) optimal: &'a Optimal,
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

/// A game's thread: its setup, then, for a search, its search once the driver says go, unless the game is cancelled
/// first (the driver drops the sender of the go).
fn play(n: u32, before: &Before, opts: &SolveOptions, events: &Sender<Event>, go: &Receiver<Go>) {
    let _unwinding = Unwinding { n, events };
    let start = Instant::now();
    let set_up = set_up(n, before, opts).map_err(Failure::message);
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
        match mem::replace(self, Plan::Search(Search::NotStarted)) {
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
    NotStarted,
    Running(Run),
    Done(Box<Outcome>),
}

/// A search that runs, as the driver follows it.
struct Run {
    /// The workers it was last told to have.
    target: usize,
    /// When it was last given threads.
    granted: Instant,
    /// Its engine's status at the last sample, once the engine has been seen running.
    last: Option<Status>,
    load: Load,
}

impl Run {
    fn new(target: usize, now: Instant) -> Run {
        // Just given its threads: nothing to act on until the next sample.
        Run { target, granted: now, last: None, load: Load::Starting { fresh: false } }
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
                let idle_share =
                    if capacity.is_zero() { 0.0 } else { idle_time.as_secs_f64() / capacity.as_secs_f64() };
                Load::Sampled { idle_share, idle_now: now.idle, idle_time, fresh: true }
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
    /// The prime formula of the board size given, on the game before it.
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
}

/// The driver's state.
struct Window<'a> {
    opts: &'a SolveOptions,
    optimal: &'a Optimal,
    results: Option<&'a mut ResultsFile>,
    earlier: &'a mut Earlier,
    lo: u32,
    hi: u32,
    /// The game of `lo − 1`.
    first: Before,
    /// The most games in flight.
    width: usize,
    /// The games in flight, by board size: entered, and not yet concluded.
    games: BTreeMap<u32, Game>,
    /// The next board size to enter the window.
    entering: u32,
    /// The next board size to conclude: the head of the window, or a game of the results file to pass over.
    next: u32,
    /// The previous game of `next`, if it has one.
    last: Option<Prev>,
    summary: RangeSummary,
    events: Sender<Event>,
    /// When the range began, and whether `TAXMAN_WINDOW_TRACE` asks for the trace on stderr.
    began: Instant,
    trace: bool,
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
            optimal: range.optimal,
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
            if let Some(event) = event {
                self.handle(event);
            }
            let now = Instant::now();
            if now >= tick {
                self.sample();
                tick = now + TICK;
            }
        }
        Ok(())
    }

    /// Whether the results file has completed game `n`.
    fn in_file(&self, n: u32) -> bool {
        self.results.as_deref().is_some_and(|file| file.completed(n).is_some())
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
                let game = Game { before: self.before(n), stage: Stage::Waiting, own: Duration::ZERO, go: None };
                self.games.insert(n, game);
            }
        }
    }

    /// Makes the repeat lookups that are due: in the order of the board sizes, each once the setups of the games
    /// before it are done.
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
            // Not for a search that a checkpoint resumes (resumed, never replaced), nor with `force_search`.
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
            // A game of the results file is skipped, as the previous game of the one after it.
            while self.next <= self.hi {
                let Some(stored) = self.results.as_deref().and_then(|file| stored_prev(file, self.next)) else {
                    break;
                };
                self.last = Some(stored);
                self.summary.skipped += 1;
                self.next += 1;
            }
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
                self.summary.stopped_at = Some(n);
                return Ok(true);
            }
            discard_checkpoint(self.opts, n);
            self.earlier.add_result(&result);
            let prev = Prev::from_result(&result);
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
        if let Some(ceiling) = ceiling(n, set.in_the_bag, Some(prev), self.opts) {
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

    /// Whether the search of game `n`, set up and not started, can start.
    fn ready(&self, n: u32, set: &SearchSetUp) -> bool {
        // After a stop signal every engine stops on its own, and only the head's search starts (see "Stopping").
        if engine::stop_signal().is_some() && n != self.next {
            return false;
        }
        !set.job.seeds() || !matches!(self.known_before(n), Known::NotYet)
    }

    /// What the games hold and claim, oldest first, for [`allot()`].
    fn needs(&self, now: Instant) -> Vec<Need> {
        let need = |(&n, game): (&u32, &Game)| match &game.stage {
            Stage::Waiting => Need::SetUp,
            Stage::SettingUp => Need::SettingUp,
            Stage::Search(set, Plan::Search(Search::NotStarted)) if self.ready(n, set) => Need::Ready,
            Stage::Search(_, Plan::Unlooked(_) | Plan::Search(Search::NotStarted)) => Need::Pending,
            Stage::Search(_, Plan::Search(Search::Running(run))) => Need::Running(run.need(now)),
            Stage::Search(_, Plan::Repeat(_) | Plan::Search(Search::Done(_)))
            | Stage::Derived(_)
            | Stage::Failed(_) => Need::Nothing,
        };
        self.games.iter().map(need).collect()
    }

    /// Whether the driver must wake every tick: a search runs (to sample it), or a game waits for a thread or for a
    /// seed (threads free as engines retire workers, and seeds appear as searches run).  Otherwise only setups run, or
    /// nothing does, and what happens next is an event (the head is concluded before the driver waits, so a head
    /// that waits for nothing does not wait).
    fn ticking(&self) -> bool {
        self.games.values().any(|game| {
            matches!(
                game.stage,
                Stage::Waiting | Stage::Search(_, Plan::Search(Search::NotStarted | Search::Running(_)))
            )
        })
    }

    /// Shares the threads ([`allot()`]) and carries out the grants.
    fn share<'s>(&mut self, scope: &'s Scope<'s, '_>)
    where
        'a: 's,
    {
        let now = Instant::now();
        let grants = allot(self.opts.threads, &self.needs(now));
        let sizes: Vec<u32> = self.games.keys().copied().collect();
        for (i, grant) in grants {
            let n = sizes[i];
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
        let (before, opts, events) = (game.before.clone(), self.opts, self.events.clone());
        // The setup and the search's start ran on the main thread when games were solved one at a time; the stack is
        // reserved address space, committed only as touched.
        let spawned = thread::Builder::new()
            .name(format!("game-{n}"))
            .stack_size(WORKER_STACK_SIZE)
            .spawn_scoped(scope, move || play(n, &before, opts, &events, &waits));
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
        let (seed, cap) = match self.known_before(n) {
            Known::Final(prev) => {
                (seeds.then(|| prev.moves.clone()), ceiling(n, set.in_the_bag, Some(prev), self.opts))
            }
            Known::Provisional(before) => (seeds.then(|| before.moves()), None),
            Known::NotYet | Known::None => (None, None),
        };
        self.trace(format_args!("start n={n} threads={threads} seed={} cap={cap:?}", seed.is_some()));
        let Some(game) = self.games.get_mut(&n) else { return };
        if let Some(go) = game.go.take() {
            // The game's thread waits for it.
            let _ = go.send(Go { seed, cap, threads });
        }
        if let Stage::Search(_, plan) = &mut game.stage {
            *plan = Plan::Search(Search::Running(Run::new(threads, now)));
        }
    }

    /// Tells the search of game `n` to have `workers` workers from now on.
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

    /// Samples every running search's engine.
    fn sample(&mut self) {
        for game in self.games.values_mut() {
            if let Stage::Search(set, Plan::Search(Search::Running(run))) = &mut game.stage {
                run.sample(set.job.control().status());
            }
        }
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
        }
        self.games.clear();
    }
}
