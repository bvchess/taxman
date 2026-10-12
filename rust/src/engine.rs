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
//! one relaxed load: its low bit is the yield signal, and the rest counts open *split requests*.  The engine
//! keeps `requested` (requests open, plus those claimed and not yet answered) equal to the *demand*: the idle
//! workers that the queued and held units will not serve, while nothing is paused and some worker runs
//! (`update_split`, under the lock, after every change of those numbers; it can only withdraw open requests).
//! A running worker claims one request by decrementing the count with a compare-and-swap, so each request has
//! at most one claimant, and yields; it hands its units over and retires the claim (`requested -= 1`) in one
//! critical section (`finish_run`), after which the demand is recomputed.  So as many workers split at once
//! as there are idle workers to feed: from one unit, `threads` workers are busy after a few rounds of splits
//! in which every running worker splits, not after one split per idle worker; and splits in flight never
//! outnumber the idle workers.  A worker answers only after `SPLIT_MIN_POLLS` polls of its current unit:
//! a fresh unit (stolen, or the deepest one its last split left it) first goes deep enough to have something
//! to give, and no worker is made to restart its unit at every step however long the demand lasts.  Idle
//! workers block on `work_cv`; a push wakes at most as many of them as there are new units, after the lock
//! is released.
//!
//! **Why one deque.**  A searcher's work is on its own depth-first stack, not in the deque: units exist only
//! after a yield, and running one costs the searcher a rebuild of its state (for Taxman a promotion pass and a
//! matching solved from scratch, 30–170 µs already at n = 480–760, against about a microsecond in the critical
//! section).  The lock is taken about twice per unit, and units are only made when workers are idle (splits)
//! or paused (checkpoints): with every worker busy nobody takes it.  Per-worker deques with stealing would
//! remove contention that is not there in steady state, at the price of an ordering argument for one-thread
//! determinism, spill and checkpoints over `threads` deques, and a distributed termination test.  What did
//! serialize the work distribution at many threads was answering one split request at a time, which the
//! demand count replaced.  `Counters::lock_wait` measures what is left: if it ever becomes a noticeable share
//! of the workers' time (many workers churning through tiny units), per-worker deques are the next step.
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
//! I/O error) is a pause whose final checkpoint is followed by `exit` instead of a release.  A pause costs
//! every worker the time until the last one parks, which is bounded by the longest stretch a searcher runs
//! without polling (one Taxman step: about 10 µs; measured at n = 1215–1225, about one step in 10^5 takes
//! over a millisecond and the longest 10–15 ms), plus the write and its fsync; then each unit pushed in it
//! costs a rebuild when it runs.  [`Counters`] records the longest wait for the workers to park and the
//! longest pause.  Progress lines are printed outside the lock, so a blocked stderr cannot stall the workers.
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
//! would only repeat work.  A caller that must first save the answer somewhere durable sets
//! `EngineConfig::keep_checkpoint` and deletes the file itself with [`remove_checkpoint`].
//!
//! # Instrumentation
//!
//! Each worker's time is split into disjoint intervals, summed in [`Counters`]: *busy* (inside
//! [`Searcher::run`]), *idle* (waiting for work outside pauses), *parked* (waiting in a pause) and *lock wait*
//! (waiting for the `state` lock after a run); the rest of `wall × threads` is thread start-up and the engine's
//! own bookkeeping.  The effective parallelism is busy time over `wall × threads`.  Also counted: splits and
//! the units they gave, steals (units taken from the front by a worker that had waited), runs and tiny runs
//! (at most one poll: units with nothing left), when all workers first ran at once, and the pauses.  The
//! clocks are read only at unit boundaries and waits, never per poll.  [`Counters::summary`] is the compact
//! form that ends every progress line.
//!
//! # Platforms
//!
//! The engine uses only `std` and behaves the same on Linux and macOS.  Checkpoints are durable on both: the
//! file is written, `sync_all`ed (`fsync` on Linux, `F_FULLFSYNC` on macOS) and renamed over the old one,
//! and the directory is then synced so the rename itself survives a crash (needed on Linux; on platforms
//! that cannot sync a directory this is skipped and the rename is still atomic).  Worker stacks reserve
//! [`WORKER_STACK_SIZE`] of address space each, committed only as touched.  `EngineConfig::default` takes
//! `available_parallelism`, which on Linux honours the CPU affinity mask and cgroup quotas.  No signals are
//! handled: a killed run loses at most one checkpoint interval.  Threads are not pinned; on a multi-socket
//! (NUMA) machine each searcher allocates its own state on its own thread, so first-touch puts it on that
//! thread's node.

use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
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
    /// file is deleted when the run completes (unless `keep_checkpoint`).
    pub checkpoint: Option<(PathBuf, Duration)>,
    /// Leave the checkpoint file when the run completes, for a caller that deletes it ([`remove_checkpoint`])
    /// only once the answer is saved: a crash in between then costs one checkpoint interval, not the search.
    pub keep_checkpoint: bool,
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
            keep_checkpoint: false,
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

/// Engine activity counters for one invocation (not saved in checkpoints).  The times are summed over workers
/// (see "Instrumentation" in the module docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    /// Worker threads.
    pub threads: usize,
    /// Split requests answered.
    pub splits: u64,
    /// Units handed to the deque by split answers.
    pub split_units: u64,
    /// Units taken from the front of the deque by workers that had waited for work (steals).
    pub steals: u64,
    /// Units run (calls of [`Searcher::run`]).
    pub runs: u64,
    /// Runs that polled at most once: units with (next to) nothing left to explore.
    pub tiny_runs: u64,
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
    /// From starting the workers until the last one exited.
    pub wall: Duration,
    /// Time workers spent in [`Searcher::run`].
    pub busy: Duration,
    /// Time workers spent waiting for work outside pauses.
    pub idle: Duration,
    /// Time workers spent parked in pauses.
    pub parked: Duration,
    /// Time workers spent waiting for the engine's lock after a run (contention on the shared deque).
    pub lock_wait: Duration,
    /// From starting the workers until every worker was running a unit at the same time (`None`: never).
    pub full_occupancy: Option<Duration>,
    /// Pauses (each checkpoint, and the final one of a stop).
    pub pauses: u64,
    /// Total and longest time from asking the workers to yield until releasing them (or exiting).
    pub pause_total: Duration,
    pub pause_max: Duration,
    /// Longest time from asking the workers to yield until every one had parked.
    pub park_max: Duration,
}

impl Counters {
    /// Add the counters of another run with as many threads, as if it had followed this one: counts and times
    /// add up, the maxima take the larger, and full occupancy the later (never if either run never reached it).
    pub fn add(&mut self, other: &Counters) {
        self.threads = self.threads.max(other.threads);
        for (a, b) in [
            (&mut self.splits, other.splits),
            (&mut self.split_units, other.split_units),
            (&mut self.steals, other.steals),
            (&mut self.runs, other.runs),
            (&mut self.tiny_runs, other.tiny_runs),
            (&mut self.checkpoints, other.checkpoints),
            (&mut self.spilled_units, other.spilled_units),
            (&mut self.spill_segments, other.spill_segments),
            (&mut self.reloaded_segments, other.reloaded_segments),
            (&mut self.pauses, other.pauses),
        ] {
            *a += b;
        }
        for (a, b) in [
            (&mut self.wall, other.wall),
            (&mut self.busy, other.busy),
            (&mut self.idle, other.idle),
            (&mut self.parked, other.parked),
            (&mut self.lock_wait, other.lock_wait),
            (&mut self.pause_total, other.pause_total),
        ] {
            *a += b;
        }
        self.max_queue = self.max_queue.max(other.max_queue);
        self.pause_max = self.pause_max.max(other.pause_max);
        self.park_max = self.park_max.max(other.park_max);
        self.full_occupancy = self.full_occupancy.zip(other.full_occupancy).map(|(a, b)| a.max(b));
    }

    /// Effective parallelism: busy time over `wall × threads`, from 0 to 1.
    #[must_use]
    pub fn efficiency(&self) -> f64 {
        self.share(self.busy)
    }

    /// `time` as a fraction of `wall × threads`.
    #[must_use]
    pub fn share(&self, time: Duration) -> f64 {
        let capacity = self.wall.as_secs_f64() * self.threads as f64;
        if capacity > 0.0 {
            time.as_secs_f64() / capacity
        } else {
            0.0
        }
    }

    /// One compact line: effective parallelism, the shares of idle and lock-wait time, splits, steals, time to
    /// full occupancy, and (if any) the pauses with the share of parked time, the longest pause and the longest
    /// wait for the workers to park.
    #[must_use]
    pub fn summary(&self) -> String {
        let full = self.full_occupancy.map_or_else(|| "never".to_owned(), fmt_short);
        let mut line = format!(
            "eff {:.1}% idle {:.1}% lock {:.1}% splits {} steals {} full {full}",
            100.0 * self.efficiency(),
            100.0 * self.share(self.idle),
            100.0 * self.share(self.lock_wait),
            si(self.splits as f64),
            si(self.steals as f64),
        );
        if self.pauses > 0 {
            line += &format!(
                " pauses {} ({:.1}%) max {} park {}",
                self.pauses,
                100.0 * self.share(self.parked),
                fmt_short(self.pause_max),
                fmt_short(self.park_max)
            );
        }
        line
    }
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
/// its own thread.  When the run completes, the checkpoint file (if configured) is deleted, unless
/// `cfg.keep_checkpoint`.
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
    // Limits, schedules and the instrumentation count from here, so a slow resume cannot use up the time limit.
    let began = Instant::now();
    shared.lock().began = began;
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
    let finished_checkpoint = plan.checkpoint.filter(|_| !cfg.keep_checkpoint).map(|(path, _)| path);
    shared.outcome(finished_checkpoint, Instant::now())
}

/// Delete a checkpoint file and any temporary file a checkpoint write left next to it.  A file that is not
/// there is fine.
///
/// # Errors
///
/// Any other failure to delete either file.
pub fn remove_checkpoint(path: &Path) -> io::Result<()> {
    for p in [path.to_path_buf(), tmp_path(path)] {
        match fs::remove_file(&p) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => {
                return Err(io::Error::new(e.kind(), format!("{}: {e}", p.display())))
            }
            _ => {}
        }
    }
    Ok(())
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
    /// Times `countdown` ran out; with it, `polls` counts every poll.
    batches: u64,
    /// `polls()` when the current run began.
    run_start: u64,
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
            batches: 0,
            run_start: 0,
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
        if self.countdown == 0 || self.shared.signal.0.load(Ordering::Relaxed) != 0 {
            return self.poll_slow();
        }
        Poll::Continue
    }

    /// Polls made by this worker so far.
    fn polls(&self) -> u64 {
        self.batches * u64::from(POLL_BATCH) + u64::from(POLL_BATCH - self.countdown)
    }

    #[cold]
    #[inline(never)]
    fn poll_slow(&mut self) -> Poll {
        if self.countdown == 0 {
            self.countdown = POLL_BATCH;
            self.batches += 1;
            let total = self.flush_nodes();
            if self.shared.node_limit.is_some_and(|limit| total >= limit) {
                self.shared.request_stop();
            }
        }
        if !self.yielding {
            let signal = self.shared.signal.0.load(Ordering::Acquire);
            if signal & SIG_YIELD != 0 {
                self.yielding = true;
            } else if signal >= SIG_REQUEST
                && self.polls() - self.run_start >= SPLIT_MIN_POLLS
                && self.shared.claim_split()
            {
                self.split_claimed = true;
                self.yielding = true;
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
/// Polls of its current unit a worker makes before it answers split requests (see "Splitting").
const SPLIT_MIN_POLLS: u64 = 64;

/// Every running worker must yield (`pause` or `exit` is set).
const SIG_YIELD: u64 = 1;
/// One open split request: the signal word counts them above the yield bit.
const SIG_REQUEST: u64 = 2;

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
    /// [`SIG_YIELD`] plus the number of open split requests times [`SIG_REQUEST`].
    signal: Line<AtomicU64>,
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
    /// Units in `held`.
    held_units: usize,
    /// Each worker's statistics as of its last finished run.
    worker_stats: Vec<Stats>,
    /// Workers inside `Searcher::run`.
    running: usize,
    /// Workers blocked on `work_cv`.
    idle: usize,
    /// Split requests open (counted in `signal`) or claimed and not yet answered.
    requested: usize,
    pause: bool,
    exit: bool,
    done: bool,
    optimal: bool,
    stop_requested: bool,
    panicked: bool,
    error: Option<io::Error>,
    counters: Counters,
    /// Each worker's time accounting.
    clocks: Vec<Clock>,
    /// When the workers were started.
    began: Instant,
    /// When the current pause was asked for.
    pause_began: Option<Instant>,
    scratch: Vec<u8>,
}

impl<U> State<U> {
    /// Units in the deque, in memory or spilled.
    fn queued(&self) -> usize {
        self.mem.len() + usize::try_from(self.spill.units).unwrap_or(usize::MAX)
    }

    fn queue_empty(&self) -> bool {
        self.mem.is_empty() && self.spill.segs.is_empty()
    }

    fn has_work(&self) -> bool {
        !self.queue_empty() || self.held_units > 0
    }
}

/// Where a worker's time went (see "Instrumentation").
#[derive(Clone, Copy, Default)]
struct Clock {
    busy: Duration,
    idle: Duration,
    parked: Duration,
    /// Waiting for the `state` lock after a run.
    lock_wait: Duration,
    /// Since when the worker has been running its current unit.
    running_since: Option<Instant>,
    /// Since when the worker has been waiting, and whether it waits in a pause.
    waiting_since: Option<(Instant, bool)>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Atomically replace the signal word by `f(current)` unless `f` returns `None`; true if it was replaced.
fn update_signal(signal: &AtomicU64, f: impl Fn(u64) -> Option<u64>) -> bool {
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
            signal: Line(AtomicU64::new(0)),
            best: Line(AtomicU64::new(init.best)),
            nodes: Line(AtomicU64::new(0)),
            payload: Mutex::new(init.payload),
            state: Mutex::new(State {
                mem: VecDeque::new(),
                spill: Spill { dir: cfg.spill_dir.clone(), prefix, seq: 0, segs: Vec::new(), units: 0 },
                held: (0..threads).map(|_| None).collect(),
                held_units: 0,
                worker_stats: vec![Stats::default(); threads],
                running: 0,
                idle: 0,
                requested: 0,
                pause: false,
                exit: false,
                done: false,
                optimal: false,
                stop_requested: false,
                panicked: false,
                error: None,
                counters: Counters { threads, ..Counters::default() },
                clocks: vec![Clock::default(); threads],
                began: t0,
                pause_began: None,
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
            st.pause_began = Some(Instant::now());
            self.signal.0.fetch_or(SIG_YIELD, Ordering::AcqRel);
            self.update_split(st);
        }
    }

    /// Count the pause that is ending (the coordinator calls this before releasing the workers or exiting).
    fn end_pause(st: &mut State<U>) {
        if let Some(asked) = st.pause_began.take() {
            let length = asked.elapsed();
            let c = &mut st.counters;
            c.pauses += 1;
            c.pause_total += length;
            c.pause_max = c.pause_max.max(length);
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

    /// Open or withdraw split requests so that `requested` matches the demand (see "Splitting").  Claims are made
    /// outside the lock, but a claim only moves a request from open to claimed, so `requested` stays exact.
    fn update_split(&self, st: &mut State<U>) {
        let demand = if st.pause || st.exit || st.running == 0 {
            0
        } else {
            st.idle.saturating_sub(st.queued() + st.held_units)
        };
        let signal = &self.signal.0;
        let mut current = signal.load(Ordering::Acquire);
        loop {
            let open = usize::try_from(current / SIG_REQUEST).expect("request count fits");
            let claimed = st.requested - open;
            let wanted = demand.saturating_sub(claimed);
            if wanted == open {
                return;
            }
            let new = (current & SIG_YIELD) | (wanted as u64 * SIG_REQUEST);
            match signal.compare_exchange_weak(current, new, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => {
                    st.requested = claimed + wanted;
                    return;
                }
                Err(actual) => current = actual,
            }
        }
    }

    /// Claim an open split request; each request has at most one claimant.
    fn claim_split(&self) -> bool {
        update_signal(&self.signal.0, |s| (s >= SIG_REQUEST).then(|| s - SIG_REQUEST))
    }

    /// Wake at most as many idle workers as `units` new units can serve.  Called after the lock is released: the
    /// units were queued under it and a waiter checks the queue under it before waiting, so no wakeup is lost,
    /// and the woken workers do not have to wait for the lock.
    fn wake_idle(&self, idle: usize, units: usize) {
        if units >= idle {
            if idle > 0 {
                self.work_cv.notify_all();
            }
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
        let unit = if front { st.mem.pop_front() } else { st.mem.pop_back() };
        if front && unit.is_some() {
            st.counters.steals += 1;
        }
        unit
    }

    // --- workers -------------------------------------------------------------------------------------------

    fn worker<S: Searcher<U>, F: Fn(usize) -> S>(&self, i: usize, make: &F) {
        let _guard = PanicGuard(self);
        let mut searcher = make(i);
        let mut ctx = Ctx::new(self, i);
        let mut next = self.acquire(self.lock(), i);
        while let Some(unit) = next {
            ctx.run_start = ctx.polls();
            searcher.run(unit, &mut ctx);
            next = self.finish_run(&mut ctx);
        }
    }

    /// Count worker `i` as running a unit from now on.
    fn begin_run(&self, st: &mut State<U>, i: usize) {
        let now = Instant::now();
        st.running += 1;
        st.counters.runs += 1;
        st.clocks[i].running_since = Some(now);
        if st.running == st.clocks.len() && st.counters.full_occupancy.is_none() {
            st.counters.full_occupancy = Some(now.saturating_duration_since(st.began));
        }
        self.update_split(st);
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
                    Some(unit) => {
                        st.held_units -= 1;
                        Some(unit)
                    }
                    None => self.pop(st, starved),
                };
                if let Some(unit) = unit {
                    self.begin_run(st, i);
                    return Some(unit);
                }
                if st.running == 0 && !st.has_work() {
                    st.done = true;
                    self.set_exit(st);
                    return None;
                }
                starved = true;
            }
            st.idle += 1;
            self.update_split(st);
            let (since, paused) = (Instant::now(), st.pause);
            st.clocks[i].waiting_since = Some((since, paused));
            guard = self.wait_work(guard);
            let st = &mut *guard;
            st.idle -= 1;
            let clock = &mut st.clocks[i];
            clock.waiting_since = None;
            *(if paused { &mut clock.parked } else { &mut clock.idle }) += since.elapsed();
        }
    }

    /// Take over the units a searcher pushed and decide what this worker runs next.
    fn finish_run(&self, ctx: &mut Ctx<'_, U>) -> Option<U> {
        ctx.flush_nodes();
        ctx.yielding = false;
        let claimed = std::mem::take(&mut ctx.split_claimed);
        let i = ctx.worker;
        let end = Instant::now();
        let mut guard = self.lock();
        let st = &mut *guard;
        st.worker_stats[i] = ctx.stats;
        st.running -= 1;
        let clock = &mut st.clocks[i];
        clock.lock_wait += end.elapsed();
        if let Some(since) = clock.running_since.take() {
            clock.busy += end.saturating_duration_since(since);
        }
        if claimed {
            st.requested -= 1;
            st.counters.splits += 1;
        }
        st.counters.tiny_runs += u64::from(ctx.polls() - ctx.run_start <= 1);
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
            st.held_units += usize::from(keep.is_some());
            st.held[i] = keep;
            if st.running == 0 {
                self.coord_cv.notify_all();
            }
            return self.acquire(guard, i);
        }
        match keep {
            Some(unit) => {
                self.begin_run(st, i);
                let idle = st.idle;
                drop(guard);
                self.wake_idle(idle, gave);
                Some(unit)
            }
            // Nothing was pushed, so there is nobody to wake.
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
                if let Some(asked) = st.pause_began {
                    st.counters.park_max = st.counters.park_max.max(asked.elapsed());
                }
                if !st.has_work() {
                    Self::end_pause(st);
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
                Self::end_pause(st);
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
                    let line = self.progress_line(&guard, plan.label, &format!("{}/s", si(rate)));
                    // Not under the lock: a blocked stderr (a full pipe) must not stall the workers.
                    drop(guard);
                    eprintln!("{line}");
                    last_report = (now, nodes);
                    next_progress = Some(now + every);
                    guard = self.lock();
                    continue;
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
            let line = self.progress_line(&guard, plan.label, status);
            drop(guard);
            eprintln!("{line}");
        }
    }

    fn progress_line(&self, st: &State<U>, label: &str, note: &str) -> String {
        let nodes = self.base.nodes + self.nodes.0.load(Ordering::Relaxed);
        format!(
            "{label}{}{} nodes {nodes} ({note}) best {} queue {} spilled {} | {}",
            if label.is_empty() { "" } else { " " },
            fmt_elapsed(self.prior + self.t0.elapsed()),
            self.best.0.load(Ordering::Relaxed),
            st.mem.len(),
            st.spill.units,
            Self::counters_at(st, Instant::now()).summary(),
        )
    }

    /// The counters with the workers' times summed up to `now` (also for runs and waits still going on).
    fn counters_at(st: &State<U>, now: Instant) -> Counters {
        let mut c = st.counters;
        c.wall = now.saturating_duration_since(st.began);
        for clock in &st.clocks {
            c.busy += clock.busy + clock.running_since.map_or(Duration::ZERO, |t| now.saturating_duration_since(t));
            c.idle += clock.idle;
            c.parked += clock.parked;
            c.lock_wait += clock.lock_wait;
            if let Some((t, paused)) = clock.waiting_since {
                *(if paused { &mut c.parked } else { &mut c.idle }) += now.saturating_duration_since(t);
            }
        }
        c
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

    /// Build the result after all workers have exited (at `ended`).
    fn outcome(&self, checkpoint: Option<&Path>, ended: Instant) -> io::Result<Outcome> {
        let elapsed = self.prior + ended.saturating_duration_since(self.t0);
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
                if let Err(e) = remove_checkpoint(path) {
                    eprintln!("warning: cannot remove finished checkpoint: {e}");
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
            counters: Self::counters_at(&st, ended),
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

/// A short duration: milliseconds below a second.
fn fmt_short(d: Duration) -> String {
    if d < Duration::from_secs(1) {
        format!("{:.1}ms", d.as_secs_f64() * 1e3)
    } else {
        format!("{:.2}s", d.as_secs_f64())
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
