//! Problem-independent parallel depth-first branch-and-bound engine.
//!
//! [`run`] drives a [`Searcher`] over a search tree that may be far too large to enumerate or to queue in
//! memory.  Work is exchanged as *units* (roots of unexplored subtrees, serialized by [`Unit`]).  The engine
//! owns the shared incumbent, distributes units over worker threads, bounds memory by spilling queued units
//! to disk, checkpoints periodically, and resumes exactly from a checkpoint.  It knows nothing about the
//! problem; the searcher explores a unit depth-first and gives work back only when asked to.
//!
//! # Protocol
//!
//! These are the engine's invariants.  Breaking any of them loses work (wrong answers) or deadlocks.
//!
//! **Conservation.**  At every moment each unexplored subtree is in exactly one place: the in-memory deque,
//! a spill segment, a worker's `held` slot (only while paused), or inside a running searcher (counted by
//! `running`).  Every move between places, together with the matching change of `running`, happens in one
//! critical section of the `state` mutex.  Units are dropped only after `exit` is set, and no checkpoint is
//! written after `exit`.
//!
//! **Work distribution.**  The deque is ordered shallow (front) to deep (back).  A searcher gives work back
//! only by yielding: `poll` returns `Yield`, the searcher pushes its whole remaining stack, shallowest first,
//! and returns.  The engine appends all but the last pushed unit to the back of the deque and continues the
//! last (deepest) one on the same thread.  A worker whose unit is exhausted takes the next unit from the
//! back; a worker that had to wait for work takes from the front (the largest subtrees).  With one thread
//! nothing ever waits, so the deque is a pure stack: exploration is in exact depth-first order,
//! deterministic, and unchanged by checkpoints and spilling.
//!
//! **Splitting.**  Running searchers are interrupted only through the `signal` word, which `poll` reads with
//! one relaxed load.  While some worker is idle, the deque and spill are empty, nothing is paused, and some
//! worker is running, the engine raises `SIG_SPLIT`.  A running worker answers by turning that bit into
//! `SIG_INFLIGHT` with a compare-and-swap, so exactly one worker answers each request, and no new request is
//! raised while one is in flight.  The answering worker clears `SIG_INFLIGHT` in the same critical section
//! in which it hands its units over (`finish_run`); the request is then re-raised if workers are still
//! idle.  A worker whose answer had nothing to give away ignores requests for a short cooldown, so it cannot
//! spin.  Idle workers block on `work_cv`; a push wakes at most as many of them as there are new units.
//!
//! **Yield signal.**  `SIG_YIELD` is set exactly when `pause` or `exit` is set, always under the lock.  While
//! it is set every `poll` returns `Yield`.
//!
//! **Spill.**  Whenever the in-memory deque holds more than `queue_mem_cap` units, blocks of `cap/2` units
//! move from its front to a new segment file.  Segments form a stack whose top is logically just in front of
//! the in-memory deque; only when the in-memory deque is empty is the top segment read back and its file
//! deleted.  A failed write leaves the units in memory and a failed read leaves the segment in place; either
//! stops the run with the error.  Leftover segment files are deleted when the run ends, also on panic.
//! Segment files (`txspill-<pid>-<run>-<seq>.seg`) left by a killed process are never read again, because
//! a checkpoint holds copies of every unit; they can be deleted.
//!
//! **Checkpoint.**  The coordinator (the calling thread) sets `pause`.  Every running worker yields, pushes
//! all but its deepest unit to the deque, keeps the deepest in its `held` slot, and parks.  When `running`
//! reaches 0, every unit is in the deque, the spill, or a `held` slot, and the coordinator writes them all in
//! deque order (oldest segment first, then the in-memory deque front to back, then the held units) together
//! with the caller's header, the incumbent, the statistics and the total elapsed time to `<path>.tmp`,
//! fsyncs, and renames it over `<path>`; a checkpoint on disk is therefore always complete, and it carries a
//! checksum.  It then clears `pause` and each worker resumes its own held unit.  A stop (time or node limit,
//! I/O error) is a pause whose final checkpoint is followed by `exit` instead of a release.
//!
//! **Termination.**  The search is complete when a worker that needs work finds the deque and spill empty,
//! `running == 0`, no held units and no pause; that worker sets `done` and `exit`.  (While paused, the
//! coordinator makes the same check once the pause is complete.)  `declare_optimal` sets `exit` directly and
//! all remaining work is discarded.  Every state change that can satisfy a waiter's condition is made under
//! the mutex and followed by a notify, so no wakeup is lost.
//!
//! **Panics.**  A guard in every worker and in the coordinator sets `exit` when its thread unwinds, so nobody
//! waits forever and `std::thread::scope` re-raises the panic.  No checkpoint is written after a panic (the
//! panicking worker's subtree is lost); the previous checkpoint stays valid.
//!
//! A run that completes deletes its checkpoint file: the file is no longer needed, and resuming from it
//! would only repeat work.

use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

// ------------------------------------------------------------------------------------------------------------
// Public API
// ------------------------------------------------------------------------------------------------------------

/// A unit of work: the root of an unexplored subtree, from which a searcher can rebuild its state.
pub trait Unit: Send + Sized {
    /// Append an encoding of `self` to `out`.
    fn encode(&self, out: &mut Vec<u8>);
    /// Rebuild a unit from the bytes [`Unit::encode`] produced.
    fn decode(bytes: &[u8]) -> Self;
}

/// What [`Ctx::poll`] tells the searcher to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Poll {
    Continue,
    /// Push all remaining unexplored work (shallowest first, deepest last) and return.
    Yield,
}

/// A depth-first explorer of units.  Each worker thread has its own searcher (made on that thread by the
/// `make` closure given to [`run`]), so per-thread caches need no locks.
pub trait Searcher<U: Unit> {
    /// Explore `unit` depth-first.  Call `ctx.poll()` often: at least every ~1000 nodes, per node is fine.
    /// When it returns [`Poll::Yield`], push ALL remaining unexplored work with `ctx.push`, shallowest first
    /// and deepest last, and return promptly.  Pushes made without a yield are allowed and treated the same.
    fn run(&mut self, unit: U, ctx: &mut Ctx<'_, U>);
}

/// Search statistics, summed over workers and across resumed runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Nodes explored; the searcher counts them.  Progress lines and `node_limit` use this.
    pub nodes: u64,
    /// Searcher-defined counters.
    pub extra: [u64; 8],
}

impl Stats {
    pub fn add(&mut self, other: &Stats) {
        self.nodes += other.nodes;
        for (a, b) in self.extra.iter_mut().zip(other.extra) {
            *a += b;
        }
    }
}

/// How to run the engine.
#[derive(Clone, Debug)]
pub struct EngineConfig {
    /// Worker threads (at least 1).
    pub threads: usize,
    /// Maximum number of units kept in memory in the shared deque; the rest are spilled (needs `spill_dir`).
    pub queue_mem_cap: usize,
    /// Directory for spill segments; created if missing.  Without it the deque is never spilled.
    pub spill_dir: Option<PathBuf>,
    /// Checkpoint file and interval.  A final checkpoint is also written when a limit stops the run, and the
    /// file is deleted when the run completes.
    pub checkpoint: Option<(PathBuf, Duration)>,
    /// Interval between progress lines on stderr.
    pub progress: Option<Duration>,
    /// Wall-clock limit for this invocation's search, counted from when the workers start (after a resumed
    /// checkpoint has been loaded); earlier runs of a resumed search do not count.
    pub time_limit: Option<Duration>,
    /// Node limit for this invocation (not counting earlier runs of a resumed search).
    pub node_limit: Option<u64>,
    /// Prefix for progress lines.
    pub label: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            threads: thread::available_parallelism().map_or(1, std::num::NonZero::get),
            queue_mem_cap: 1 << 20,
            spill_dir: None,
            checkpoint: None,
            progress: None,
            time_limit: None,
            node_limit: None,
            label: String::new(),
        }
    }
}

/// Where a run starts.
pub enum Start<U> {
    /// A new search.  `units` go into the deque in the given order, as if pushed by a yield: one thread
    /// explores the last one first.  `best`/`payload` are the initial incumbent.
    Fresh { units: Vec<U>, best: u64, payload: Vec<u8> },
    /// Continue from a checkpoint file.  The `header` given to [`run`] must equal the checkpoint's.
    Resume(PathBuf),
}

/// Engine activity counters for one invocation (not saved in checkpoints).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    /// Split requests answered.
    pub splits: u64,
    /// Units handed to the deque by split answers.
    pub split_units: u64,
    /// Checkpoints written (periodic and final).
    pub checkpoints: u64,
    /// Units written to spill segments.
    pub spilled_units: u64,
    /// Spill segment files written.
    pub spill_segments: u64,
    /// Spill segments read back.
    pub reloaded_segments: u64,
    /// Largest in-memory deque length seen (before spilling).
    pub max_queue: u64,
}

/// The result of [`run`].
#[derive(Clone, Debug)]
pub struct Outcome {
    /// The whole tree was explored, or a searcher declared the incumbent optimal.  False when a limit
    /// stopped the run (a final checkpoint was written if configured).
    pub completed: bool,
    /// A searcher called [`Ctx::declare_optimal`].
    pub optimal_declared: bool,
    pub best: u64,
    pub payload: Vec<u8>,
    /// Totals across all runs of this search, including those before a resume.
    pub stats: Stats,
    /// Total wall time across all runs of this search, including those before a resume.
    pub elapsed: Duration,
    /// This invocation's engine activity.
    pub counters: Counters,
}

/// Stack size of worker threads; searchers may recurse deeply.
pub const WORKER_STACK_SIZE: usize = 64 << 20;

/// Run a search to completion, to a limit, or until a searcher declares the incumbent optimal.
///
/// `header` is opaque caller data stored in every checkpoint (everything the caller needs to rebuild its
/// searchers on resume); read it back with [`read_checkpoint_header`].  Worker `i` uses `make(i)`, called on
/// its own thread.  When the run completes, the checkpoint file (if configured) is deleted.
///
/// # Errors
///
/// I/O failures while spilling, checkpointing or resuming (the last checkpoint written stays valid), a
/// damaged checkpoint (`InvalidData`), and a checkpoint whose header differs from `header` (`InvalidInput`).
///
/// # Panics
///
/// A panic in a searcher (or in `make`) stops all workers and propagates to the caller.
pub fn run<U, S, F>(cfg: &EngineConfig, start: Start<U>, header: &[u8], make: F) -> io::Result<Outcome>
where
    U: Unit,
    S: Searcher<U>,
    F: Fn(usize) -> S + Sync,
{
    let t0 = Instant::now();
    let threads = cfg.threads.max(1);
    if let Some(dir) = &cfg.spill_dir {
        fs::create_dir_all(dir)?;
    }
    let (init, fresh, resume) = match start {
        Start::Fresh { units, best, payload } => {
            (Init { best, payload, base: Stats::default(), prior: Duration::ZERO }, units, None)
        }
        Start::Resume(path) => {
            // Verify the whole file before decoding anything from it.
            let head = scan_checkpoint(&path, |_| Ok(()))?;
            if head.header != header {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "checkpoint header does not match this run"));
            }
            let init = Init { best: head.best, payload: head.payload, base: head.stats, prior: head.elapsed };
            (init, Vec::new(), Some(path))
        }
    };
    let shared = Shared::new(cfg, threads, init, t0);
    shared.load(fresh, resume.as_deref())?;
    // Limits and schedules count from here, so a slow resume cannot use up the time limit by itself.
    let began = Instant::now();
    let plan = Plan {
        checkpoint: cfg.checkpoint.as_ref().map(|(path, every)| (path.as_path(), *every)),
        progress: cfg.progress,
        began,
        deadline: cfg.time_limit.map(|limit| began + limit),
        label: &cfg.label,
        header,
    };
    thread::scope(|scope| {
        for i in 0..threads {
            let (shared, make) = (&shared, &make);
            let spawned = thread::Builder::new()
                .name(format!("engine-{i}"))
                .stack_size(WORKER_STACK_SIZE)
                .spawn_scoped(scope, move || shared.worker(i, make));
            if let Err(e) = spawned {
                let mut st = shared.lock();
                st.error.get_or_insert(e);
                shared.set_exit(&mut st);
                break;
            }
        }
        shared.coordinate(&plan);
    });
    shared.outcome(plan.checkpoint.map(|(path, _)| path))
}

/// Read the caller's header from a checkpoint file (without verifying the rest of the file).
///
/// # Errors
///
/// I/O failures, and `InvalidData` if the file is not a checkpoint of this format version.
pub fn read_checkpoint_header(path: &Path) -> io::Result<Vec<u8>> {
    let mut r = BufReader::new(File::open(path)?);
    get_prefix(&mut r).map_err(truncation)
}

/// A worker's handle to the engine, passed to [`Searcher::run`].
pub struct Ctx<'a, U: Unit> {
    shared: &'a Shared<U>,
    worker: usize,
    /// Polls left until the next slow poll (node publishing, node limit).
    countdown: u32,
    /// Slow polls during which split requests are ignored.
    cooldown: u32,
    yielding: bool,
    split_claimed: bool,
    /// `stats.nodes` already added to `shared.nodes`.
    published: u64,
    stats: Stats,
    pushed: Vec<U>,
}

impl<'a, U: Unit> Ctx<'a, U> {
    fn new(shared: &'a Shared<U>, worker: usize) -> Self {
        Ctx {
            shared,
            worker,
            countdown: POLL_BATCH,
            cooldown: 0,
            yielding: false,
            split_claimed: false,
            published: 0,
            stats: Stats::default(),
            pushed: Vec::new(),
        }
    }

    /// Whether to keep going.  Costs one decrement and one relaxed atomic load.
    #[inline]
    #[must_use]
    pub fn poll(&mut self) -> Poll {
        self.countdown -= 1;
        if self.countdown == 0 || self.shared.signal.0.load(Ordering::Relaxed) & SIG_ACT != 0 {
            return self.poll_slow();
        }
        Poll::Continue
    }

    #[cold]
    #[inline(never)]
    fn poll_slow(&mut self) -> Poll {
        if self.countdown == 0 {
            self.countdown = POLL_BATCH;
            let total = self.flush_nodes();
            if self.shared.node_limit.is_some_and(|limit| total >= limit) {
                self.shared.request_stop();
            }
        }
        if !self.yielding {
            let signal = self.shared.signal.0.load(Ordering::Acquire);
            if signal & SIG_YIELD != 0 {
                self.yielding = true;
            } else if signal & SIG_SPLIT != 0 {
                if self.cooldown > 0 {
                    self.cooldown -= 1;
                } else if self.shared.claim_split() {
                    self.split_claimed = true;
                    self.yielding = true;
                }
            }
        }
        if self.yielding {
            Poll::Yield
        } else {
            Poll::Continue
        }
    }

    /// Hand back an unexplored unit (see [`Searcher::run`] for the order).
    #[inline]
    pub fn push(&mut self, unit: U) {
        self.pushed.push(unit);
    }

    /// The incumbent value (a relaxed atomic read).
    #[inline]
    #[must_use]
    pub fn best(&self) -> u64 {
        self.shared.best.0.load(Ordering::Relaxed)
    }

    /// Offer a solution: if `value` beats the incumbent, it becomes the incumbent together with `payload`
    /// (stored atomically with the value) and `true` is returned.
    pub fn offer(&mut self, value: u64, payload: &[u8]) -> bool {
        let best = &self.shared.best.0;
        if value <= best.load(Ordering::Relaxed) {
            return false;
        }
        let mut stored = lock(&self.shared.payload);
        if value <= best.load(Ordering::Relaxed) {
            return false;
        }
        stored.clear();
        stored.extend_from_slice(payload);
        best.store(value, Ordering::Release);
        true
    }

    /// The searcher has proven the incumbent optimal: discard all remaining work and complete the run.
    /// Every later `poll` (in every worker) returns `Yield`.
    pub fn declare_optimal(&mut self) {
        let mut st = self.shared.lock();
        st.optimal = true;
        self.shared.set_exit(&mut st);
        self.yielding = true;
    }

    /// This worker's statistics; the engine sums them over workers and runs.
    #[inline]
    pub fn stats(&mut self) -> &mut Stats {
        &mut self.stats
    }

    /// This worker's index, `0..threads`.
    #[must_use]
    pub fn worker(&self) -> usize {
        self.worker
    }

    /// Publish node counts; returns this invocation's total published nodes.
    fn flush_nodes(&mut self) -> u64 {
        let delta = self.stats.nodes.wrapping_sub(self.published);
        self.published = self.stats.nodes;
        self.shared.nodes.0.fetch_add(delta, Ordering::Relaxed).wrapping_add(delta)
    }
}

// ------------------------------------------------------------------------------------------------------------
// Shared state
// ------------------------------------------------------------------------------------------------------------

/// Polls between slow polls.
const POLL_BATCH: u32 = 1024;
/// Slow polls a worker ignores split requests for after an answer that had nothing to give away.
const SPLIT_COOLDOWN: u32 = 64;

/// An idle worker wants work: one running worker should yield.
const SIG_SPLIT: u32 = 1;
/// Every running worker must yield (`pause` or `exit` is set).
const SIG_YIELD: u32 = 2;
/// A split request has been claimed and its units not yet handed over.
const SIG_INFLIGHT: u32 = 4;
/// Bits that need `poll`'s attention.
const SIG_ACT: u32 = SIG_SPLIT | SIG_YIELD;

static RUN_SEQ: AtomicU64 = AtomicU64::new(0);

/// Keeps a hot atomic on its own cache line (128 bytes covers Apple silicon and adjacent-line prefetch).
#[repr(align(128))]
struct Line<T>(T);

struct Init {
    best: u64,
    payload: Vec<u8>,
    base: Stats,
    prior: Duration,
}

struct Plan<'p> {
    checkpoint: Option<(&'p Path, Duration)>,
    progress: Option<Duration>,
    /// When the workers were started.
    began: Instant,
    deadline: Option<Instant>,
    label: &'p str,
    header: &'p [u8],
}

struct Shared<U: Unit> {
    signal: Line<AtomicU32>,
    /// Incumbent value; changes only while `payload` is locked.
    best: Line<AtomicU64>,
    /// Nodes published by workers in this invocation.
    nodes: Line<AtomicU64>,
    payload: Mutex<Vec<u8>>,
    state: Mutex<State<U>>,
    /// Idle and parked workers wait here.
    work_cv: Condvar,
    /// The coordinator waits here.
    coord_cv: Condvar,
    cap: usize,
    node_limit: Option<u64>,
    /// Statistics and elapsed time of the runs before a resume.
    base: Stats,
    prior: Duration,
    t0: Instant,
}

struct State<U> {
    mem: VecDeque<U>,
    spill: Spill,
    /// Each worker's deepest unit while paused.
    held: Vec<Option<U>>,
    /// Each worker's statistics as of its last finished run.
    worker_stats: Vec<Stats>,
    /// Workers inside `Searcher::run`.
    running: usize,
    /// Workers blocked on `work_cv`.
    idle: usize,
    pause: bool,
    exit: bool,
    done: bool,
    optimal: bool,
    stop_requested: bool,
    panicked: bool,
    error: Option<io::Error>,
    counters: Counters,
    scratch: Vec<u8>,
}

impl<U> State<U> {
    fn queue_empty(&self) -> bool {
        self.mem.is_empty() && self.spill.segs.is_empty()
    }

    fn has_work(&self) -> bool {
        !self.queue_empty() || self.held.iter().any(Option::is_some)
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Atomically replace the signal word by `f(current)` unless `f` returns `None`; true if it was replaced.
fn update_signal(signal: &AtomicU32, f: impl Fn(u32) -> Option<u32>) -> bool {
    let mut current = signal.load(Ordering::Acquire);
    while let Some(new) = f(current) {
        match signal.compare_exchange_weak(current, new, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return true,
            Err(actual) => current = actual,
        }
    }
    false
}

/// Sets `exit` if its thread unwinds, so that nobody waits for it.
struct PanicGuard<'a, U: Unit>(&'a Shared<U>);

impl<U: Unit> Drop for PanicGuard<'_, U> {
    fn drop(&mut self) {
        if thread::panicking() {
            let mut st = self.0.lock();
            st.panicked = true;
            self.0.set_exit(&mut st);
        }
    }
}

impl<U: Unit> Shared<U> {
    fn new(cfg: &EngineConfig, threads: usize, init: Init, t0: Instant) -> Self {
        let prefix = format!("txspill-{}-{}", std::process::id(), RUN_SEQ.fetch_add(1, Ordering::Relaxed));
        Shared {
            signal: Line(AtomicU32::new(0)),
            best: Line(AtomicU64::new(init.best)),
            nodes: Line(AtomicU64::new(0)),
            payload: Mutex::new(init.payload),
            state: Mutex::new(State {
                mem: VecDeque::new(),
                spill: Spill { dir: cfg.spill_dir.clone(), prefix, seq: 0, segs: Vec::new(), units: 0 },
                held: (0..threads).map(|_| None).collect(),
                worker_stats: vec![Stats::default(); threads],
                running: 0,
                idle: 0,
                pause: false,
                exit: false,
                done: false,
                optimal: false,
                stop_requested: false,
                panicked: false,
                error: None,
                counters: Counters::default(),
                scratch: Vec::new(),
            }),
            work_cv: Condvar::new(),
            coord_cv: Condvar::new(),
            cap: cfg.queue_mem_cap.max(1),
            node_limit: cfg.node_limit,
            base: init.base,
            prior: init.prior,
            t0,
        }
    }

    fn lock(&self) -> MutexGuard<'_, State<U>> {
        lock(&self.state)
    }

    fn wait_work<'g>(&self, st: MutexGuard<'g, State<U>>) -> MutexGuard<'g, State<U>> {
        self.work_cv.wait(st).unwrap_or_else(PoisonError::into_inner)
    }

    fn wait_coord<'g>(&self, st: MutexGuard<'g, State<U>>, timeout: Option<Duration>) -> MutexGuard<'g, State<U>> {
        match timeout {
            Some(t) => self.coord_cv.wait_timeout(st, t).unwrap_or_else(PoisonError::into_inner).0,
            None => self.coord_cv.wait(st).unwrap_or_else(PoisonError::into_inner),
        }
    }

    /// Fill the deque with the starting units, spilling as needed.
    fn load(&self, fresh: Vec<U>, resume: Option<&Path>) -> io::Result<()> {
        let mut guard = self.lock();
        let st = &mut *guard;
        for unit in fresh {
            st.mem.push_back(unit);
            self.spill_excess(st)?;
        }
        if let Some(path) = resume {
            scan_checkpoint(path, |bytes| {
                st.mem.push_back(U::decode(bytes));
                self.spill_excess(st)
            })?;
        }
        Ok(())
    }

    // --- signals -------------------------------------------------------------------------------------------

    fn set_exit(&self, st: &mut State<U>) {
        st.exit = true;
        self.signal.0.fetch_or(SIG_YIELD, Ordering::AcqRel);
        self.work_cv.notify_all();
        self.coord_cv.notify_all();
    }

    fn begin_pause(&self, st: &mut State<U>) {
        if !st.pause {
            st.pause = true;
            self.signal.0.fetch_or(SIG_YIELD, Ordering::AcqRel);
            self.update_split(st);
        }
    }

    fn release(&self, st: &mut State<U>) {
        st.pause = false;
        self.signal.0.fetch_and(!SIG_YIELD, Ordering::AcqRel);
        self.update_split(st);
        self.work_cv.notify_all();
    }

    /// Ask the coordinator to stop the run (with a final checkpoint).
    fn request_stop(&self) {
        let mut st = self.lock();
        if !st.stop_requested && !st.exit {
            st.stop_requested = true;
            self.begin_pause(&mut st);
            self.coord_cv.notify_all();
        }
    }

    /// Record an I/O error and stop the run.
    fn fail(&self, st: &mut State<U>, e: io::Error) {
        st.error.get_or_insert(e);
        st.stop_requested = true;
        self.begin_pause(st);
        self.coord_cv.notify_all();
    }

    /// Raise or clear the split request to match the state (see "Splitting").
    fn update_split(&self, st: &State<U>) {
        let wanted = st.idle > 0 && st.running > 0 && !st.pause && !st.exit && st.queue_empty();
        let signal = &self.signal.0;
        if wanted {
            update_signal(signal, |s| (s & (SIG_SPLIT | SIG_INFLIGHT) == 0).then_some(s | SIG_SPLIT));
        } else if signal.load(Ordering::Relaxed) & SIG_SPLIT != 0 {
            signal.fetch_and(!SIG_SPLIT, Ordering::AcqRel);
        }
    }

    /// Claim the pending split request; true for exactly one claimant per request.
    fn claim_split(&self) -> bool {
        update_signal(&self.signal.0, |s| (s & SIG_SPLIT != 0).then_some((s & !SIG_SPLIT) | SIG_INFLIGHT))
    }

    fn wake_idle(&self, st: &State<U>, units: usize) {
        if st.idle == 0 {
            return;
        }
        if units >= st.idle {
            self.work_cv.notify_all();
        } else {
            for _ in 0..units {
                self.work_cv.notify_one();
            }
        }
    }

    // --- the deque and its spill ---------------------------------------------------------------------------

    /// Spill blocks from the front of the in-memory deque while it is over the cap.
    fn spill_excess(&self, st: &mut State<U>) -> io::Result<()> {
        st.counters.max_queue = st.counters.max_queue.max(st.mem.len() as u64);
        if st.mem.len() <= self.cap || st.spill.dir.is_none() || st.error.is_some() {
            return Ok(());
        }
        let block = (self.cap / 2).max(1);
        while st.mem.len() > self.cap {
            st.spill.write(st.mem.range(..block), &mut st.scratch)?;
            st.mem.drain(..block);
            st.counters.spilled_units += block as u64;
            st.counters.spill_segments += 1;
        }
        Ok(())
    }

    /// Append units to the back of the deque.
    fn push_units(&self, st: &mut State<U>, units: &mut Vec<U>) {
        st.mem.extend(units.drain(..));
        if let Err(e) = self.spill_excess(st) {
            self.fail(st, e);
        }
    }

    /// Take a unit from the back (or front) of the deque, reloading the top spill segment if memory is empty.
    fn pop(&self, st: &mut State<U>, front: bool) -> Option<U> {
        if st.mem.is_empty() {
            match st.spill.read_top::<U>() {
                Ok(Some(units)) => {
                    st.mem.extend(units);
                    st.counters.reloaded_segments += 1;
                }
                Ok(None) => return None,
                Err(e) => {
                    self.fail(st, e);
                    return None;
                }
            }
        }
        if front {
            st.mem.pop_front()
        } else {
            st.mem.pop_back()
        }
    }

    // --- workers -------------------------------------------------------------------------------------------

    fn worker<S: Searcher<U>, F: Fn(usize) -> S>(&self, i: usize, make: &F) {
        let _guard = PanicGuard(self);
        let mut searcher = make(i);
        let mut ctx = Ctx::new(self, i);
        let mut next = self.acquire(self.lock(), i);
        while let Some(unit) = next {
            searcher.run(unit, &mut ctx);
            next = self.finish_run(&mut ctx);
        }
    }

    /// Wait for a unit to run (counting it in `running`), or return `None` when the worker must exit.
    fn acquire(&self, mut guard: MutexGuard<'_, State<U>>, i: usize) -> Option<U> {
        let mut starved = false;
        loop {
            let st = &mut *guard;
            if st.exit {
                return None;
            }
            if !st.pause {
                let unit = match st.held[i].take() {
                    Some(unit) => Some(unit),
                    None => self.pop(st, starved),
                };
                if let Some(unit) = unit {
                    st.running += 1;
                    self.update_split(st);
                    return Some(unit);
                }
                if st.running == 0 && !st.pause && !st.has_work() {
                    st.done = true;
                    self.set_exit(st);
                    return None;
                }
                starved = true;
            }
            st.idle += 1;
            self.update_split(st);
            guard = self.wait_work(guard);
            guard.idle -= 1;
        }
    }

    /// Take over the units a searcher pushed and decide what this worker runs next.
    fn finish_run(&self, ctx: &mut Ctx<'_, U>) -> Option<U> {
        ctx.flush_nodes();
        ctx.yielding = false;
        let claimed = std::mem::take(&mut ctx.split_claimed);
        let i = ctx.worker;
        let mut guard = self.lock();
        let st = &mut *guard;
        st.worker_stats[i] = ctx.stats;
        st.running -= 1;
        if claimed {
            self.signal.0.fetch_and(!SIG_INFLIGHT, Ordering::AcqRel);
            st.counters.splits += 1;
        }
        if st.exit {
            ctx.pushed.clear();
            return None;
        }
        let keep = ctx.pushed.pop();
        let gave = ctx.pushed.len();
        if gave > 0 {
            if claimed {
                st.counters.split_units += gave as u64;
            }
            self.push_units(st, &mut ctx.pushed);
        }
        if st.pause {
            st.held[i] = keep;
            if st.running == 0 {
                self.coord_cv.notify_all();
            }
            return self.acquire(guard, i);
        }
        if gave > 0 {
            self.wake_idle(st, gave);
        } else if claimed {
            ctx.cooldown = SPLIT_COOLDOWN;
        }
        match keep {
            Some(unit) => {
                st.running += 1;
                self.update_split(st);
                Some(unit)
            }
            None => self.acquire(guard, i),
        }
    }

    // --- coordinator ---------------------------------------------------------------------------------------

    /// Runs on the calling thread: checkpoints, limits, progress lines.  Returns once `exit` is set.
    fn coordinate(&self, plan: &Plan<'_>) {
        let _guard = PanicGuard(self);
        let mut next_checkpoint = plan.checkpoint.map(|(_, every)| plan.began + every);
        let mut next_progress = plan.progress.map(|every| plan.began + every);
        let mut last_report = (plan.began, 0);
        let mut guard = self.lock();
        loop {
            if guard.exit {
                break;
            }
            let now = Instant::now();
            let past = |t: Option<Instant>| t.is_some_and(|t| Instant::now() >= t);
            if guard.stop_requested || past(plan.deadline) || past(next_checkpoint) {
                self.begin_pause(&mut guard);
                while guard.running > 0 && !guard.exit {
                    guard = self.wait_coord(guard, None);
                }
                let st = &mut *guard;
                if st.exit {
                    continue;
                }
                if !st.has_work() {
                    st.done = true;
                    self.set_exit(st);
                    continue;
                }
                if let Some((path, _)) = plan.checkpoint {
                    match self.write_checkpoint(st, path, plan.header) {
                        Ok(()) => st.counters.checkpoints += 1,
                        Err(e) => {
                            st.error.get_or_insert(e);
                            st.stop_requested = true;
                        }
                    }
                }
                if st.stop_requested || past(plan.deadline) {
                    self.set_exit(st);
                    continue;
                }
                self.release(st);
                next_checkpoint = plan.checkpoint.map(|(_, every)| Instant::now() + every);
                continue;
            }
            if let (Some(due), Some(every)) = (next_progress, plan.progress) {
                if now >= due {
                    let nodes = self.nodes.0.load(Ordering::Relaxed);
                    let rate = (nodes - last_report.1) as f64 / now.duration_since(last_report.0).as_secs_f64();
                    eprintln!("{}", self.progress_line(&guard, plan.label, &format!("{}/s", si(rate))));
                    last_report = (now, nodes);
                    next_progress = Some(now + every);
                }
            }
            let wake = [next_checkpoint, next_progress, plan.deadline].into_iter().flatten().min();
            guard = self.wait_coord(guard, wake.map(|t| t.saturating_duration_since(now)));
        }
        if plan.progress.is_some() {
            let status = if guard.panicked {
                "panicked"
            } else if guard.error.is_some() {
                "failed"
            } else if guard.optimal {
                "optimal"
            } else if guard.done {
                "done"
            } else {
                "stopped"
            };
            eprintln!("{}", self.progress_line(&guard, plan.label, status));
        }
    }

    fn progress_line(&self, st: &State<U>, label: &str, note: &str) -> String {
        let nodes = self.base.nodes + self.nodes.0.load(Ordering::Relaxed);
        format!(
            "{label}{}{} nodes {nodes} ({note}) best {} queue {} spilled {}",
            if label.is_empty() { "" } else { " " },
            fmt_elapsed(self.prior + self.t0.elapsed()),
            self.best.0.load(Ordering::Relaxed),
            st.mem.len(),
            st.spill.units,
        )
    }

    fn total_stats(&self, st: &State<U>) -> Stats {
        let mut total = self.base;
        for s in &st.worker_stats {
            total.add(s);
        }
        total
    }

    /// Write every unit (spill, deque, held) and the run's state; see "Checkpoint".  Workers must be parked.
    fn write_checkpoint(&self, st: &mut State<U>, path: &Path, header: &[u8]) -> io::Result<()> {
        let tmp = tmp_path(path);
        let result = self.write_checkpoint_file(st, &tmp, header).and_then(|()| fs::rename(&tmp, path));
        if result.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        result?;
        // Make the rename durable; not every platform can sync a directory, so this is best effort.
        let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        if let Ok(d) = File::open(dir) {
            let _ = d.sync_all();
        }
        Ok(())
    }

    fn write_checkpoint_file(&self, st: &mut State<U>, tmp: &Path, header: &[u8]) -> io::Result<()> {
        let mut w = HashWriter { inner: BufWriter::with_capacity(1 << 20, File::create(tmp)?), hash: Hash64::new() };
        w.write_all(&MAGIC)?;
        put_u32(&mut w, VERSION)?;
        put_blob(&mut w, header)?;
        {
            let payload = lock(&self.payload);
            put_u64(&mut w, self.best.0.load(Ordering::Relaxed))?;
            put_blob(&mut w, &payload)?;
        }
        let stats = self.total_stats(st);
        put_u64(&mut w, stats.nodes)?;
        for x in stats.extra {
            put_u64(&mut w, x)?;
        }
        let elapsed = self.prior + self.t0.elapsed();
        put_u64(&mut w, u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX))?;
        let held = st.held.iter().flatten().count();
        put_u64(&mut w, st.spill.units + (st.mem.len() + held) as u64)?;
        for seg in &st.spill.segs {
            let copied = io::copy(&mut File::open(&seg.path)?, &mut w)?;
            if copied != seg.bytes {
                return Err(invalid_data(format!("spill segment {} changed size", seg.path.display())));
            }
        }
        for unit in st.mem.iter().chain(st.held.iter().flatten()) {
            put_unit(&mut w, unit, &mut st.scratch)?;
        }
        let hash = w.hash.finish();
        let mut out = w.inner;
        out.write_all(&END_MAGIC)?;
        put_u64(&mut out, hash)?;
        out.into_inner().map_err(io::IntoInnerError::into_error)?.sync_all()
    }

    /// Build the result after all workers have exited.
    fn outcome(&self, checkpoint: Option<&Path>) -> io::Result<Outcome> {
        let elapsed = self.prior + self.t0.elapsed();
        let mut st = self.lock();
        let completed = st.done || st.optimal;
        if let Some(e) = st.error.take() {
            // Errors never lose units (see "Spill"), so a completed search is still exact.
            if !completed {
                return Err(e);
            }
            eprintln!("warning: the search completed despite an I/O error: {e}");
        }
        if completed {
            if let Some(path) = checkpoint {
                for p in [path.to_path_buf(), tmp_path(path)] {
                    if let Err(e) = fs::remove_file(&p) {
                        if e.kind() != io::ErrorKind::NotFound {
                            eprintln!("warning: cannot remove finished checkpoint {}: {e}", p.display());
                        }
                    }
                }
            }
        }
        Ok(Outcome {
            completed,
            optimal_declared: st.optimal,
            best: self.best.0.load(Ordering::Acquire),
            payload: lock(&self.payload).clone(),
            stats: self.total_stats(&st),
            elapsed,
            counters: st.counters,
        })
    }
}

// ------------------------------------------------------------------------------------------------------------
// Spill segments
// ------------------------------------------------------------------------------------------------------------

struct Segment {
    path: PathBuf,
    units: usize,
    bytes: u64,
}

/// A stack of spill segment files; each holds length-prefixed encoded units (the checkpoint's unit format).
struct Spill {
    dir: Option<PathBuf>,
    prefix: String,
    seq: u64,
    segs: Vec<Segment>,
    /// Units in all segments.
    units: u64,
}

impl Spill {
    fn write<'u, U: Unit + 'u>(
        &mut self,
        units: impl ExactSizeIterator<Item = &'u U>,
        scratch: &mut Vec<u8>,
    ) -> io::Result<()> {
        let dir = self.dir.as_ref().expect("spilling needs a spill directory");
        let path = dir.join(format!("{}-{:06}.seg", self.prefix, self.seq));
        self.seq += 1;
        let count = units.len();
        let write = || -> io::Result<u64> {
            let mut w = BufWriter::with_capacity(1 << 16, File::create(&path)?);
            let mut bytes = 0;
            for unit in units {
                bytes += put_unit(&mut w, unit, scratch)?;
            }
            w.flush()?;
            Ok(bytes)
        };
        match write() {
            Ok(bytes) => {
                self.segs.push(Segment { path, units: count, bytes });
                self.units += count as u64;
                Ok(())
            }
            Err(e) => {
                let _ = fs::remove_file(&path);
                Err(e)
            }
        }
    }

    /// Read the top segment back and delete it; the segment stays if reading fails.
    fn read_top<U: Unit>(&mut self) -> io::Result<Option<Vec<U>>> {
        let Some(seg) = self.segs.last() else { return Ok(None) };
        let data = fs::read(&seg.path)?;
        let mut units = Vec::with_capacity(seg.units);
        let mut rest = &data[..];
        while !rest.is_empty() {
            let (unit, tail) = split_unit(rest)?;
            units.push(U::decode(unit));
            rest = tail;
        }
        if units.len() != seg.units {
            return Err(invalid_data(format!("spill segment {} is damaged", seg.path.display())));
        }
        let seg = self.segs.pop().expect("segment present");
        self.units -= seg.units as u64;
        let _ = fs::remove_file(&seg.path);
        Ok(Some(units))
    }
}

impl Drop for Spill {
    fn drop(&mut self) {
        for seg in &self.segs {
            let _ = fs::remove_file(&seg.path);
        }
    }
}

// ------------------------------------------------------------------------------------------------------------
// Checkpoint file format (little-endian):
//   "TXCKPT01", version u32, header (u64 len + bytes), best u64, payload (u64 len + bytes),
//   nodes u64, extra 8 x u64, elapsed nanoseconds u64, unit count u64, units (u32 len + bytes each),
//   "TXCKEND1", checksum u64 of everything before "TXCKEND1".
// ------------------------------------------------------------------------------------------------------------

const MAGIC: [u8; 8] = *b"TXCKPT01";
const END_MAGIC: [u8; 8] = *b"TXCKEND1";
const VERSION: u32 = 1;
/// Largest header or payload accepted when reading (guards against allocating for garbage lengths).
const MAX_BLOB: u64 = 1 << 32;

struct Head {
    header: Vec<u8>,
    best: u64,
    payload: Vec<u8>,
    stats: Stats,
    elapsed: Duration,
}

/// Read and verify a whole checkpoint, passing each unit's bytes to `unit` in order.
fn scan_checkpoint(path: &Path, mut unit: impl FnMut(&[u8]) -> io::Result<()>) -> io::Result<Head> {
    let mut r = HashReader { inner: BufReader::with_capacity(1 << 20, File::open(path)?), hash: Hash64::new() };
    let mut scan = || -> io::Result<Head> {
        let header = get_prefix(&mut r)?;
        let best = get_u64(&mut r)?;
        let payload = get_blob(&mut r, "payload")?;
        let mut stats = Stats { nodes: get_u64(&mut r)?, extra: [0; 8] };
        for x in &mut stats.extra {
            *x = get_u64(&mut r)?;
        }
        let elapsed = Duration::from_nanos(get_u64(&mut r)?);
        let count = get_u64(&mut r)?;
        let mut buf = Vec::new();
        for _ in 0..count {
            let len = get_u32(&mut r)?;
            // Grow the buffer as bytes arrive, so a damaged length cannot cause a huge allocation.
            buf.clear();
            (&mut r).take(u64::from(len)).read_to_end(&mut buf)?;
            if buf.len() != len as usize {
                return Err(io::ErrorKind::UnexpectedEof.into());
            }
            unit(&buf)?;
        }
        let expected = r.hash.finish();
        let mut end = [0; 8];
        r.inner.read_exact(&mut end)?;
        let checksum = get_u64(&mut r.inner)?;
        if end != END_MAGIC || checksum != expected || r.inner.read(&mut [0])? != 0 {
            return Err(invalid_data("checkpoint is corrupt (checksum mismatch)"));
        }
        Ok(Head { header, best, payload, stats, elapsed })
    };
    scan().map_err(truncation)
}

fn get_prefix(r: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut magic = [0; 8];
    r.read_exact(&mut magic)?;
    if magic != MAGIC {
        return Err(invalid_data("not an engine checkpoint"));
    }
    let version = get_u32(r)?;
    if version != VERSION {
        return Err(invalid_data(format!("unsupported checkpoint version {version}")));
    }
    get_blob(r, "header")
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".tmp");
    PathBuf::from(s)
}

fn invalid_data(msg: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.into())
}

fn truncation(e: io::Error) -> io::Error {
    if e.kind() == io::ErrorKind::UnexpectedEof {
        invalid_data("checkpoint is truncated")
    } else {
        e
    }
}

fn put_u32(w: &mut impl Write, x: u32) -> io::Result<()> {
    w.write_all(&x.to_le_bytes())
}

fn put_u64(w: &mut impl Write, x: u64) -> io::Result<()> {
    w.write_all(&x.to_le_bytes())
}

fn put_blob(w: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    put_u64(w, bytes.len() as u64)?;
    w.write_all(bytes)
}

/// Write one length-prefixed unit; returns the bytes written.
fn put_unit<U: Unit>(w: &mut impl Write, unit: &U, scratch: &mut Vec<u8>) -> io::Result<u64> {
    scratch.clear();
    unit.encode(scratch);
    let len = u32::try_from(scratch.len()).map_err(|_| invalid_data("encoded unit exceeds 4 GiB"))?;
    put_u32(w, len)?;
    w.write_all(scratch)?;
    Ok(4 + u64::from(len))
}

fn split_unit(bytes: &[u8]) -> io::Result<(&[u8], &[u8])> {
    let damaged = || invalid_data("damaged unit framing");
    let (len, rest) = bytes.split_first_chunk::<4>().ok_or_else(damaged)?;
    let len = u32::from_le_bytes(*len) as usize;
    if rest.len() < len {
        return Err(damaged());
    }
    Ok(rest.split_at(len))
}

fn get_u32(r: &mut impl Read) -> io::Result<u32> {
    let mut b = [0; 4];
    r.read_exact(&mut b)?;
    Ok(u32::from_le_bytes(b))
}

fn get_u64(r: &mut impl Read) -> io::Result<u64> {
    let mut b = [0; 8];
    r.read_exact(&mut b)?;
    Ok(u64::from_le_bytes(b))
}

fn get_blob(r: &mut impl Read, what: &str) -> io::Result<Vec<u8>> {
    let len = get_u64(r)?;
    if len > MAX_BLOB {
        return Err(invalid_data(format!("checkpoint {what} length {len} is implausible")));
    }
    let mut v = Vec::new();
    r.take(len).read_to_end(&mut v)?;
    if v.len() as u64 != len {
        return Err(io::ErrorKind::UnexpectedEof.into());
    }
    Ok(v)
}

/// Streaming 64-bit checksum over 8-byte words.  Each step is a bijection of the running state, so any error
/// confined to one word is always detected; others are missed with probability about 2^-64.
#[derive(Clone, Copy)]
struct Hash64 {
    h: u64,
    len: u64,
    tail: [u8; 8],
    ntail: usize,
}

impl Hash64 {
    const K: u64 = 0x9E37_79B9_7F4A_7C15;

    fn new() -> Self {
        Hash64 { h: 0x243F_6A88_85A3_08D3, len: 0, tail: [0; 8], ntail: 0 }
    }

    fn mix(&mut self, word: u64) {
        self.h = (self.h.rotate_left(23) ^ word).wrapping_mul(Self::K);
    }

    fn update(&mut self, mut data: &[u8]) {
        self.len += data.len() as u64;
        if self.ntail > 0 {
            let take = (8 - self.ntail).min(data.len());
            self.tail[self.ntail..self.ntail + take].copy_from_slice(&data[..take]);
            self.ntail += take;
            data = &data[take..];
            if self.ntail < 8 {
                return;
            }
            self.mix(u64::from_le_bytes(self.tail));
            self.ntail = 0;
        }
        let (words, rest) = data.as_chunks::<8>();
        for word in words {
            self.mix(u64::from_le_bytes(*word));
        }
        self.tail[..rest.len()].copy_from_slice(rest);
        self.ntail = rest.len();
    }

    fn finish(&self) -> u64 {
        let mut s = *self;
        if s.ntail > 0 {
            let mut last = [0; 8];
            last[..s.ntail].copy_from_slice(&s.tail[..s.ntail]);
            s.mix(u64::from_le_bytes(last));
        }
        s.mix(s.len);
        let h = s.h ^ (s.h >> 32);
        let h = h.wrapping_mul(Self::K);
        h ^ (h >> 29)
    }
}

struct HashWriter<W> {
    inner: W,
    hash: Hash64,
}

impl<W: Write> Write for HashWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hash.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

struct HashReader<R> {
    inner: R,
    hash: Hash64,
}

impl<R: Read> Read for HashReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.hash.update(&buf[..n]);
        Ok(n)
    }
}

// ------------------------------------------------------------------------------------------------------------
// Formatting
// ------------------------------------------------------------------------------------------------------------

fn fmt_elapsed(d: Duration) -> String {
    let s = d.as_secs();
    if s < 60 {
        format!("{:.1}s", d.as_secs_f64())
    } else if s < 3600 {
        format!("{}m{:02}s", s / 60, s % 60)
    } else {
        format!("{}h{:02}m{:02}s", s / 3600, s / 60 % 60, s % 60)
    }
}

fn si(x: f64) -> String {
    if x >= 1e9 {
        format!("{:.2}G", x / 1e9)
    } else if x >= 1e6 {
        format!("{:.2}M", x / 1e6)
    } else if x >= 1e3 {
        format!("{:.1}k", x / 1e3)
    } else {
        format!("{x:.0}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_independent_of_chunking() {
        let data: Vec<u8> = (0..1000u32).map(|i| (i * 7 + 3) as u8).collect();
        let mut whole = Hash64::new();
        whole.update(&data);
        for step in [1, 3, 7, 8, 13, 64] {
            let mut parts = Hash64::new();
            for chunk in data.chunks(step) {
                parts.update(chunk);
            }
            assert_eq!(parts.finish(), whole.finish(), "step {step}");
        }
        let mut flipped = data.clone();
        flipped[500] ^= 1;
        let mut other = Hash64::new();
        other.update(&flipped);
        assert_ne!(other.finish(), whole.finish());
        let mut shorter = Hash64::new();
        shorter.update(&data[..999]);
        assert_ne!(shorter.finish(), whole.finish());
    }

    #[test]
    fn elapsed_and_rate_formatting() {
        assert_eq!(fmt_elapsed(Duration::from_millis(12_340)), "12.3s");
        assert_eq!(fmt_elapsed(Duration::from_secs(125)), "2m05s");
        assert_eq!(fmt_elapsed(Duration::from_secs(3 * 3600 + 7 * 60 + 9)), "3h07m09s");
        assert_eq!(si(12_345_678.0), "12.35M");
        assert_eq!(si(999.0), "999");
    }
}
