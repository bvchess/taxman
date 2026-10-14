//! Problem-independent parallel depth-first branch-and-bound engine.
//!
//! [`run`] drives a [`Searcher`] over a search tree that may be far too large to enumerate or to queue in
//! memory.  Work is exchanged as *units* (roots of unexplored subtrees, serialized by [`Unit`]).  The engine
//! owns the shared incumbent, distributes units over worker threads, bounds memory by spilling queued units
//! to disk, checkpoints periodically, and resumes exactly from a checkpoint.  It knows nothing about the
//! problem; the searcher explores a unit depth-first and gives work back only when asked to.
//!
//! # Using the engine
//!
//! A problem implements two traits and calls [`run`]:
//!
//! * [`Unit`]: a unit of work, the root of an unexplored subtree, that can be written to bytes and read back.  (The
//!   Taxman search's is two bitsets, `search::TaxUnit`.)
//! * [`Searcher`]: explores one unit depth-first in [`Searcher::run`], calling [`Ctx::poll`] often, at least every
//!   ~1000 nodes.  When the poll says [`Poll::Yield`] it pushes ALL its remaining unexplored work with [`Ctx::push`],
//!   shallowest first, and returns.  Through the [`Ctx`] it also reads the incumbent ([`Ctx::best`]), offers a better
//!   one ([`Ctx::offer`]), declares the incumbent optimal ([`Ctx::declare_optimal`]) and counts its nodes
//!   ([`Ctx::stats`]).
//!
//! [`run`] takes an [`EngineConfig`] (threads, memory cap and spill directory, checkpoint file and interval, limits,
//! progress), a [`Start`] (new units with an initial incumbent, or a checkpoint to resume), an opaque `header` that
//! identifies the search (stored in every checkpoint and compared on resume), and a closure that makes one searcher per
//! worker thread.  It returns an [`Outcome`]: whether the search completed, the best value and payload, the statistics,
//! the time and the [`Counters`].  `tests/engine.rs` runs a toy problem (knapsack) through every feature.
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
//! outnumber the idle workers.  A worker answers only after `MIN_POLLS` polls of its current unit: a fresh
//! unit (stolen, or the deepest one its last split left it) first goes deep enough to have something to give,
//! and no worker is made to restart its unit at every step however long the demand lasts.  Idle workers block
//! on `work_cv`; a push wakes at most as many of them as there are new units, after the lock is released.
//!
//! **Why one deque.**  A searcher's work is on its own depth-first stack, not in the deque: units exist only
//! after a yield, and running one costs the searcher a rebuild of its state (for Taxman a promotion pass and a
//! matching solved from scratch, tens to hundreds of microseconds, against about a microsecond in the critical
//! section; docs/EXPERIMENTS.md, "The engine", has the numbers).  The lock is taken about twice per unit, and units
//! are only made when workers are idle (splits) or paused (checkpoints): with every worker busy nobody takes it.
//! Per-worker deques with stealing would remove contention that is not there in steady state, at the price of an
//! ordering argument for one-thread determinism, spill and checkpoints over `threads` deques, and a distributed
//! termination test.  What did serialize the work distribution at many threads was answering one split request at a
//! time, which the demand count replaced.  `Counters::lock_wait` measures what is left: if it ever becomes a
//! noticeable share of the workers' time (many workers churning through tiny units), per-worker deques are the next
//! step.
//!
//! **Yield signal.**  `SIG_YIELD` is set exactly when `exit` is set or a stop has been asked for, and
//! `SIG_PAUSE` when a periodic checkpoint's pause is, always under the lock (`pause` is set with either).
//! While `SIG_YIELD` is set every `poll` returns `Yield`; while `SIG_PAUSE` is, a `poll` returns `Yield` once
//! the worker has made `MIN_POLLS` polls of its current unit, so that a worker released from one pause gets
//! work done before the next however short the checkpoint interval.
//!
//! **Spill.**  Whenever the in-memory deque holds more than `queue_mem_cap` units, blocks of `cap/2` units
//! move from its front to a new segment file.  Segments form a stack whose top is logically just in front of
//! the in-memory deque; only when the in-memory deque is empty is the top segment read back and its file
//! deleted.  A failed write leaves the units in memory and a failed read leaves the segment in place; either
//! stops the run with the error.  Leftover segment files are deleted when the run ends, also when a panic
//! unwinds.  Segment files (`txspill-<pid>-<run>-<seq>.seg`) left by a process that died (killed, or a panic
//! in a release build, which aborts) are never read again, because a checkpoint holds copies of every unit:
//! a run deletes those of processes that no longer exist from its spill directory when it starts.
//!
//! **Checkpoint.**  The coordinator (the calling thread) sets `pause`.  Every running worker yields, pushes
//! all but its deepest unit to the deque, keeps the deepest in its `held` slot, and parks.  When `running`
//! reaches 0, every unit is in the deque, the spill, or a `held` slot, and the coordinator writes them all in
//! deque order (oldest segment first, then the in-memory deque front to back, then the held units) together
//! with the caller's header, the incumbent, the statistics and the total elapsed time to `<path>.tmp`,
//! fsyncs, and renames it over `<path>`; a checkpoint on disk is therefore always complete, and it carries a
//! checksum.  It then clears `pause` and each worker resumes its own held unit.  A stop (time or node limit,
//! a stop signal, an I/O error) is a pause whose final checkpoint is followed by `exit` instead of a release.
//! A pause costs every worker the time until the last one parks, which is bounded by the longest stretch a
//! searcher runs without polling (one Taxman step takes microseconds, with a tail of milliseconds: docs/EXPERIMENTS.md,
//! "The engine") plus, for a periodic checkpoint, up to `MIN_POLLS` polls of a unit just begun; then the write and
//! its fsync; then each unit pushed in it costs a rebuild when it runs.  [`Counters`] records the longest wait for the
//! workers to park and the longest pause.
//!
//! The next periodic checkpoint is due one interval after a pause ends, but never sooner than that pause
//! lasted, and its pause begins only once the released workers are all running again (`resuming` is clear:
//! no held unit is left, and the deque is empty or every worker runs).  The coordinator waits for that
//! without the lock, so with any interval, zero included, the workers make progress and pauses take at most
//! about half of their time.
//!
//! **Output.**  Progress lines and warnings go to stderr outside the lock, so a blocked stderr cannot stall
//! the workers, and write errors are ignored, so a closed one (a dead `tee`, EPIPE) cannot kill them either.
//!
//! **Termination.**  The search is complete when a worker that needs work finds the deque and spill empty,
//! `running == 0`, no held units and no pause; that worker sets `done` and `exit`.  (While paused, the
//! coordinator makes the same check once the pause is complete.)  `declare_optimal` sets `exit` directly and
//! all remaining work is discarded.  Every state change that can satisfy a waiter's condition is made under
//! the mutex and followed by a notify, so no wakeup is lost.
//!
//! **Panics.**  In a build that unwinds (tests), a guard in every worker and in the coordinator sets `exit`
//! when its thread unwinds, so nobody waits forever and `std::thread::scope` re-raises the panic; no
//! checkpoint is written after a panic (the panicking worker's subtree is lost).  Release builds abort on a
//! panic (`panic = "abort"`), so the process ends at once.  Either way the previous checkpoint stays valid.
//!
//! A run that completes deletes its checkpoint file: the file is no longer needed, and resuming from it
//! would only repeat work.  A caller that must first record the result durably sets
//! `EngineConfig::keep_completed_checkpoint` and deletes the file itself afterwards ([`remove_checkpoint`]),
//! so that a crash in between loses nothing.
//!
//! **Stopping.**  Besides the time and node limits, a run stops (with its final checkpoint) when the process
//! receives SIGINT or SIGTERM after [`stop_on_signals`]: the handler only sets an atomic flag, which the
//! coordinator polls at least every `SIGNAL_POLL`.  A second such signal ends the process at once.
//! [`lock_dir`] keeps a second process out of a checkpoint directory, so that two runs never write the same
//! checkpoint file.
//!
//! **Starting stopped.**  The coordinator is the only one who checks the deadline and the stop signal, and the
//! workers run until it does, so a stop that begins while a run is under way takes effect when the coordinator
//! next looks, and a search shorter than that completes first.  A stop that already holds when the run starts
//! must not depend on that race: a deadline already past (a time limit of zero) or a stop signal already received
//! does no search work, however the threads are scheduled.  [`run`] makes that check (`Plan::stop_due`, the very
//! test the coordinator makes) before it starts a worker; if it holds, it asks for the stop (`request_stop`)
//! and starts none, and the coordinator stops the run at its first check like any stop.  No unit was ever handed
//! out, so the final checkpoint holds all of them (the starting units, or the checkpoint's), the incumbent and
//! statistics are the ones the run started with, `completed` is false and `Counters::runs` is 0; a run resumed
//! from that checkpoint while the stop still holds stops the same way and writes the same units again.  (A
//! search with no units at all is complete, and says so.)  The node limit is not a stop of this kind: workers
//! enforce it when they publish their node counts, every `POLL_BATCH` polls, so a search shorter than that
//! completes whatever the limit.
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
//! The engine uses only `std` (plus the POSIX `signal`, `kill` and `_exit`, declared in `sys`) and behaves
//! the same on Linux and macOS.  Checkpoints are durable on both: the file is written, `sync_all`ed (`fsync`
//! on Linux, `F_FULLFSYNC` on macOS) and renamed over the old one, and the directory is then synced so the
//! rename itself survives a crash (needed on Linux; on platforms that cannot sync a directory this is skipped
//! and the rename is still atomic).  Worker stacks reserve [`WORKER_STACK_SIZE`] of address space each,
//! committed only as touched.  `EngineConfig::default` takes `available_parallelism`, which on Linux honours
//! the CPU affinity mask and cgroup quotas.  A run stopped by SIGINT or SIGTERM (after [`stop_on_signals`])
//! loses nothing; one killed otherwise loses at most one checkpoint interval.  Telling whether the owner of a
//! spill segment is alive uses process ids, so processes in different PID namespaces (containers) must not
//! share a spill directory.  Threads are not pinned; on a multi-socket (NUMA) machine each searcher allocates
//! its own state on its own thread, so first-touch puts it on that thread's node.

use std::collections::VecDeque;
use std::ffi::c_int;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
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
    /// Checkpoint file and interval.  A final checkpoint is also written when a limit or a stop signal stops
    /// the run (also one that holds from the start), and the file is deleted when the run completes (unless
    /// `keep_completed_checkpoint`).  Any interval works, zero included: see "Checkpoint" in the module docs.
    pub checkpoint: Option<(PathBuf, Duration)>,
    /// Leave the checkpoint file in place when the run completes, for a caller that deletes it
    /// ([`remove_checkpoint`]) only once it has recorded the result durably.  Off by default.
    pub keep_completed_checkpoint: bool,
    /// Interval between progress lines on stderr.
    pub progress: Option<Duration>,
    /// Wall-clock limit for this invocation's search, counted from when the workers start (after a resumed
    /// checkpoint has been loaded); earlier runs of a resumed search do not count.  Too large to add to the
    /// clock means none.  A limit that has run out before the first worker would start, zero above all, stops
    /// the run before any search work, with its final checkpoint ("Starting stopped" in the module docs); one
    /// that runs out later stops the run when the coordinator next looks.
    pub time_limit: Option<Duration>,
    /// Node limit for this invocation (not counting earlier runs of a resumed search).  The workers enforce it
    /// when they publish their node counts, every 1024 polls, so a search shorter than that completes.
    pub node_limit: Option<u64>,
    /// Prefix for progress lines.
    pub label: String,
    /// Emulation of many workers on few CPUs, for measuring what a larger machine would do in counts (nodes,
    /// lookups), not time: at most this many of the `threads` workers search at once, taking turns every
    /// `POLL_BATCH` polls in the order they ask (a panic in a worker gives its turn up).  `None`: no limit, and
    /// the check costs nothing but one test every `POLL_BATCH` polls.  The workers' busy time includes their
    /// waits for a turn, so the [`Counters`] times mean nothing with it.
    pub cores: Option<usize>,
}

impl Default for EngineConfig {
    fn default() -> Self {
        EngineConfig {
            threads: thread::available_parallelism().map_or(1, std::num::NonZero::get),
            queue_mem_cap: 1 << 20,
            spill_dir: None,
            checkpoint: None,
            keep_completed_checkpoint: false,
            progress: None,
            time_limit: None,
            node_limit: None,
            label: String::new(),
            cores: None,
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
    /// The whole tree was explored, or a searcher declared the incumbent optimal.  False when a limit or a
    /// stop signal stopped the run (a final checkpoint was written if configured).
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

/// Stack size of worker threads: room for searchers that recurse (the Taxman searcher does not).  It is only
/// reserved address space, committed as touched.
pub const WORKER_STACK_SIZE: usize = 64 << 20;

/// Run a search to completion, to a limit, until a stop signal (see [`stop_on_signals`]), or until a searcher
/// declares the incumbent optimal.
///
/// `header` is opaque caller data stored in every checkpoint (everything the caller needs to rebuild its
/// searchers on resume); read it back with [`read_checkpoint_header`] or [`read_checkpoint`].  Worker `i` uses
/// `make(i)`, called on its own thread.  When the run completes, the checkpoint file (if configured) is
/// deleted unless `cfg.keep_completed_checkpoint`.
///
/// # Errors
///
/// I/O failures while spilling, checkpointing or resuming (the last checkpoint written stays valid), a
/// damaged checkpoint (`InvalidData`), and a checkpoint whose header differs from `header` (`InvalidInput`).
///
/// # Panics
///
/// A panic in a searcher (or in `make`) stops all workers and propagates to the caller (in a build that
/// unwinds; release builds abort).
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
        remove_orphaned_segments(dir);
    }
    let (init, fresh, resume) = match start {
        Start::Fresh { units, best, payload } => {
            (Init { best, payload, base: Stats::default(), prior: Duration::ZERO }, units, None)
        }
        Start::Resume(path) => {
            // Verify the whole file before decoding anything from it.
            let head = read_checkpoint(&path)?;
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
        deadline: cfg.time_limit.and_then(|limit| began.checked_add(limit)),
        label: &cfg.label,
        header,
    };
    // See "Starting stopped": a stop that already holds is made before any worker exists to take a unit, and the
    // coordinator carries it out as it does every stop.
    let workers = if plan.stop_due() {
        shared.request_stop();
        0
    } else {
        threads
    };
    thread::scope(|scope| {
        for i in 0..workers {
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
    let finished = plan.checkpoint.map(|(path, _)| path).filter(|_| !cfg.keep_completed_checkpoint);
    shared.outcome(finished, Instant::now())
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

/// Everything a checkpoint holds but its units.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointHead {
    /// The caller's header.
    pub header: Vec<u8>,
    /// The incumbent's value and payload.
    pub best: u64,
    pub payload: Vec<u8>,
    /// Totals of the runs so far.
    pub stats: Stats,
    pub elapsed: Duration,
}

/// Read a checkpoint file, verifying all of it (its checksum included), and return everything but its units.
///
/// # Errors
///
/// I/O failures, and `InvalidData` if the file is not a checkpoint of this format version or is damaged.
pub fn read_checkpoint(path: &Path) -> io::Result<CheckpointHead> {
    scan_checkpoint(path, |_| Ok(()))
}

/// Delete a checkpoint file and any temporary file of it left by a failed write; files that do not exist are
/// fine.  For a caller that ran with `keep_completed_checkpoint`.
///
/// # Errors
///
/// The first failure to delete one of them.
pub fn remove_checkpoint(path: &Path) -> io::Result<()> {
    let mut result = Ok(());
    for p in [path.to_path_buf(), tmp_path(path)] {
        if let Err(e) = fs::remove_file(&p) {
            if e.kind() != io::ErrorKind::NotFound && result.is_ok() {
                result = Err(io::Error::new(e.kind(), format!("cannot remove {}: {e}", p.display())));
            }
        }
    }
    result
}

/// [`remove_checkpoint`] for a search that completed, with a warning on stderr if a file stays (resuming it
/// would only repeat work).
pub fn remove_finished_checkpoint(path: &Path) {
    if let Err(e) = remove_checkpoint(path) {
        report(&format!("warning: the search completed, but its checkpoint stays: {e}"));
    }
}

/// A checksum of `bytes`, the one checkpoints carry (a 64-bit hash, `Hash64`): for callers that want to record a digest
/// of something in their header.
#[must_use]
pub fn checksum(bytes: &[u8]) -> u64 {
    let mut hash = Hash64::new();
    hash.update(bytes);
    hash.finish()
}

/// An exclusive lock on a directory, held until dropped (see [`lock_dir`]).
#[derive(Debug)]
pub struct DirLock {
    file: File,
}

/// Dropping releases the lock at once, by unlocking rather than only closing.  An `flock` belongs to the open
/// file description, which a process spawned meanwhile by another thread shares until its `exec` closes the
/// copy of the descriptor it inherited; closing ours would leave the directory locked for that long (a
/// millisecond or two), and a [`lock_dir`] right after the drop would be refused.  `unlock` frees every copy.
impl Drop for DirLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

/// The name of the lock file [`lock_dir`] uses.
pub const LOCK_FILE: &str = "taxman.lock";

/// Lock `dir` (which must exist) against every other process, by an advisory lock (`flock`) on the file
/// [`LOCK_FILE`] in it, which records the holder's process id.  The operating system releases the lock when
/// the process ends, however it ends, so a lock is never left behind.  Two handles in one process exclude
/// each other too.
///
/// # Errors
///
/// `WouldBlock`, naming the holder, if another process holds the lock; I/O failures.
pub fn lock_dir(dir: &Path) -> io::Result<DirLock> {
    let path = dir.join(LOCK_FILE);
    let mut file = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(&path)?;
    match file.try_lock() {
        Ok(()) => {
            file.set_len(0)?;
            writeln!(file, "{}", std::process::id())?;
            Ok(DirLock { file })
        }
        Err(TryLockError::WouldBlock) => {
            let mut holder = String::new();
            let _ = file.read_to_string(&mut holder);
            let holder = holder.trim();
            let who = if holder.is_empty() { String::new() } else { format!(" (process {holder})") };
            let message = format!("{} is in use by another process{who}: {} is locked", dir.display(), path.display());
            Err(io::Error::new(io::ErrorKind::WouldBlock, message))
        }
        Err(TryLockError::Error(e)) => Err(e),
    }
}

/// Make SIGINT and SIGTERM stop every run of this process the way a time limit does (with a final checkpoint,
/// `completed == false`), now and later: the first signal sets a flag that every run's coordinator polls at
/// least every `SIGNAL_POLL`, and a run that starts after it stops at once, before any search work ("Starting
/// stopped" in the module docs).  A second signal ends the process at once with status 128 + the signal (the
/// last checkpoint stays valid).
///
/// # Errors
///
/// The operating system's, if it refuses a handler; `Unsupported` where there are no POSIX signals.
pub fn stop_on_signals() -> io::Result<()> {
    for signal in [sys::SIGINT, sys::SIGTERM] {
        sys::install(signal, on_stop_signal)?;
    }
    STOP_HANDLERS.store(true, Ordering::Relaxed);
    Ok(())
}

/// The signal that asked this process to stop (see [`stop_on_signals`]), if one has.
#[must_use]
pub fn stop_signal() -> Option<i32> {
    match STOP_SIGNAL.load(Ordering::Relaxed) {
        0 => None,
        signal => Some(signal),
    }
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
            if let Some(gate) = &self.shared.gate {
                gate.pass();
            }
            let total = self.flush_nodes();
            if self.shared.node_limit.is_some_and(|limit| total >= limit) {
                self.shared.request_stop();
            }
        }
        if !self.yielding {
            let signal = self.shared.signal.0.load(Ordering::Acquire);
            let settled = self.polls() - self.run_start >= MIN_POLLS;
            if signal & SIG_YIELD != 0 || (signal & SIG_PAUSE != 0 && settled) {
                self.yielding = true;
            } else if signal >= SIG_REQUEST && settled && self.shared.claim_split() {
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
/// Polls of its current unit a worker makes before it answers a split request or a periodic checkpoint's pause
/// (see "Splitting" and "Yield signal").
const MIN_POLLS: u64 = 64;

/// Every running worker must yield now (`exit` is set, or a stop is under way).
const SIG_YIELD: u64 = 1;
/// Every running worker must yield once it has made `MIN_POLLS` polls of its unit (a periodic checkpoint).
const SIG_PAUSE: u64 = 2;
/// One open split request: the signal word counts them above the two flags.
const SIG_REQUEST: u64 = 4;
/// How long a coordinator waits at most before it looks for a stop signal again (after [`stop_on_signals`]).
const SIGNAL_POLL: Duration = Duration::from_millis(50);

static RUN_SEQ: AtomicU64 = AtomicU64::new(0);
/// The first stop signal received, 0 if none (see [`stop_on_signals`]).
static STOP_SIGNAL: AtomicI32 = AtomicI32::new(0);
/// Whether [`stop_on_signals`] has installed its handlers, so that coordinators must poll `STOP_SIGNAL`.
static STOP_HANDLERS: AtomicBool = AtomicBool::new(false);

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

impl Plan<'_> {
    /// Whether the clock or the process asks the run to stop now: the deadline has passed, or a stop signal
    /// has been received (see [`stop_on_signals`]).  The one test of both: the coordinator makes it at every
    /// look, and [`run`] before it starts a worker ("Starting stopped").  Once true it stays true.
    fn stop_due(&self) -> bool {
        stop_signal().is_some() || past(self.deadline)
    }
}

/// Whether `t`, if there is one, is not in the future.
fn past(t: Option<Instant>) -> bool {
    t.is_some_and(|t| Instant::now() >= t)
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
    /// With [`EngineConfig::cores`], the turns of the workers.
    gate: Option<Gate>,
}

/// Lets at most `slots` workers search at once, in the order they ask: a ticket may go once fewer than `slots`
/// tickets issued before it are still inside (for [`EngineConfig::cores`]).
struct Gate {
    slots: u64,
    /// Tickets issued, and tickets that have left.
    turns: Mutex<(u64, u64)>,
    cv: Condvar,
}

impl Gate {
    fn new(slots: usize) -> Self {
        Gate { slots: slots.max(1) as u64, turns: Mutex::new((0, 0)), cv: Condvar::new() }
    }

    /// Wait for a turn, which lasts until the returned guard is dropped.
    fn turn(&self) -> Turn<'_> {
        self.wait_for_ticket(lock(&self.turns));
        Turn(self)
    }

    /// The worker in a turn gives it up and takes its place at the back of the line, in one step.
    fn pass(&self) {
        let mut turns = lock(&self.turns);
        turns.1 += 1;
        self.cv.notify_all();
        self.wait_for_ticket(turns);
    }

    /// Take the next ticket and wait until fewer than `slots` earlier ones are inside.
    fn wait_for_ticket(&self, mut turns: MutexGuard<'_, (u64, u64)>) {
        let ticket = turns.0;
        turns.0 += 1;
        while ticket >= turns.1 + self.slots {
            turns = self.cv.wait(turns).unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// A turn of a [`Gate`], given back when dropped, a panic unwinding included, so that nobody waits for it forever.
struct Turn<'a>(&'a Gate);

impl Drop for Turn<'_> {
    fn drop(&mut self) {
        lock(&self.0.turns).1 += 1;
        self.0.cv.notify_all();
    }
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
    /// Since the start or the last release from a pause, until the workers all run again: no held unit left,
    /// and the deque empty or every worker running.  No periodic checkpoint begins meanwhile.
    resuming: bool,
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
                resuming: true,
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
            gate: cfg.cores.map(Gate::new),
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

    /// Pause the workers: for a stop (`urgent`) at their next poll, for a periodic checkpoint after `MIN_POLLS`
    /// polls of their units.  A stop asked for during a checkpoint's pause makes it urgent.
    fn begin_pause(&self, st: &mut State<U>, urgent: bool) {
        if !st.pause {
            st.pause = true;
            st.pause_began = Some(Instant::now());
            self.update_split(st);
        }
        self.signal.0.fetch_or(if urgent { SIG_YIELD } else { SIG_PAUSE }, Ordering::AcqRel);
    }

    /// Count the pause that is ending (the coordinator calls this before releasing the workers or exiting);
    /// returns its length.
    fn end_pause(st: &mut State<U>) -> Duration {
        let Some(asked) = st.pause_began.take() else { return Duration::ZERO };
        let length = asked.elapsed();
        let c = &mut st.counters;
        c.pauses += 1;
        c.pause_total += length;
        c.pause_max = c.pause_max.max(length);
        length
    }

    fn release(&self, st: &mut State<U>) {
        st.pause = false;
        st.resuming = true;
        self.signal.0.fetch_and(!(SIG_YIELD | SIG_PAUSE), Ordering::AcqRel);
        self.update_split(st);
        self.work_cv.notify_all();
    }

    /// Ask the coordinator to stop the run (with a final checkpoint).
    fn request_stop(&self) {
        let mut st = self.lock();
        if !st.stop_requested && !st.exit {
            st.stop_requested = true;
            self.begin_pause(&mut st, true);
            self.coord_cv.notify_all();
        }
    }

    /// Record an I/O error and stop the run.
    fn fail(&self, st: &mut State<U>, e: io::Error) {
        st.error.get_or_insert(e);
        st.stop_requested = true;
        self.begin_pause(st, true);
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
            let new = (current & (SIG_YIELD | SIG_PAUSE)) | (wanted as u64 * SIG_REQUEST);
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

    /// Spill blocks from the front of the in-memory deque while it is over `queue_mem_cap`.
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
            let turn = self.gate.as_ref().map(Gate::turn);
            searcher.run(unit, &mut ctx);
            drop(turn);
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
        if st.resuming && st.held_units == 0 && (st.queue_empty() || st.running == st.clocks.len()) {
            st.resuming = false;
            self.coord_cv.notify_all();
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

    /// Runs on the calling thread: checkpoints, limits, stop signals, progress lines.  Returns once `exit` is
    /// set.  Times too far ahead to represent (absurd intervals or limits) are never reached.
    fn coordinate(&self, plan: &Plan<'_>) {
        let _guard = PanicGuard(self);
        let mut next_checkpoint = plan.checkpoint.and_then(|(_, every)| plan.began.checked_add(every));
        let mut next_progress = plan.progress.and_then(|every| plan.began.checked_add(every));
        let mut last_report = (plan.began, 0);
        let stopping = |st: &State<U>| st.stop_requested || plan.stop_due();
        let mut guard = self.lock();
        loop {
            if guard.exit {
                break;
            }
            let now = Instant::now();
            let stop = stopping(&guard);
            if stop || (past(next_checkpoint) && !guard.resuming) {
                self.begin_pause(&mut guard, stop);
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
                let pause = Self::end_pause(st);
                if stopping(st) {
                    self.set_exit(st);
                    continue;
                }
                self.release(st);
                // See "Checkpoint": never sooner than the pause took, and not before the workers run again.
                next_checkpoint = plan.checkpoint.and_then(|(_, every)| Instant::now().checked_add(every.max(pause)));
                continue;
            }
            if let (Some(due), Some(every)) = (next_progress, plan.progress) {
                if now >= due {
                    let nodes = self.nodes.0.load(Ordering::Relaxed);
                    let rate = (nodes - last_report.1) as f64 / now.duration_since(last_report.0).as_secs_f64();
                    let line = self.progress_line(&guard, plan.label, &format!("{}/s", si(rate)));
                    // Not under the lock: a blocked stderr (a full pipe) must not stall the workers.
                    drop(guard);
                    report(&line);
                    last_report = (now, nodes);
                    next_progress = now.checked_add(every);
                    guard = self.lock();
                    continue;
                }
            }
            // While the workers resume, the checkpoint that is due waits for `begin_run` to say they have.
            let checkpoint = next_checkpoint.filter(|_| !guard.resuming);
            let signals = STOP_HANDLERS.load(Ordering::Relaxed).then(|| now + SIGNAL_POLL);
            let wake = [checkpoint, next_progress, plan.deadline, signals].into_iter().flatten().min();
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
            report(&line);
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

    /// Build the result after all workers have exited (at `ended`); a completed run deletes `finished`.
    fn outcome(&self, finished: Option<&Path>, ended: Instant) -> io::Result<Outcome> {
        let elapsed = self.prior + ended.saturating_duration_since(self.t0);
        let mut st = self.lock();
        let completed = st.done || st.optimal;
        if let Some(e) = st.error.take() {
            // Errors never lose units (see "Spill"), so a completed search is still exact.
            if !completed {
                return Err(e);
            }
            report(&format!("warning: the search completed despite an I/O error: {e}"));
        }
        if let (true, Some(path)) = (completed, finished) {
            remove_finished_checkpoint(path);
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

/// Delete the spill segments in `dir` of processes that no longer exist (see "Spill"), as best it can.
fn remove_orphaned_segments(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let me = std::process::id();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let owner = name.to_str().and_then(|name| name.strip_prefix("txspill-")?.split('-').next()?.parse().ok());
        if owner.is_some_and(|pid| pid != me && !sys::process_exists(pid)) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

// ------------------------------------------------------------------------------------------------------------
// Signals and processes: the few POSIX functions `std` does not wrap
// ------------------------------------------------------------------------------------------------------------

/// Sets [`STOP_SIGNAL`]; at a second signal, ends the process (see [`stop_on_signals`]).  Async-signal-safe:
/// a lock-free atomic and `_exit`.
extern "C" fn on_stop_signal(signal: c_int) {
    if STOP_SIGNAL.swap(signal, Ordering::SeqCst) != 0 {
        sys::exit_now(128 + signal);
    }
}

#[cfg(unix)]
mod sys {
    use std::ffi::c_int;
    use std::io;

    /// The same numbers on Linux and macOS.
    pub const SIGINT: c_int = 2;
    pub const SIGTERM: c_int = 15;
    /// "No such process", the same number on Linux and macOS.
    const ESRCH: i32 = 3;
    /// What `signal` returns when it fails (`SIG_ERR`, that is `(void (*)(int)) -1`).
    const SIG_ERR: usize = usize::MAX;

    extern "C" {
        /// The previous handler is returned as an address, since it can be `SIG_DFL` (0) or `SIG_IGN` (1).
        fn signal(signum: c_int, handler: extern "C" fn(c_int)) -> usize;
        fn kill(pid: c_int, signum: c_int) -> c_int;
        fn _exit(status: c_int) -> !;
    }

    /// Install `handler` for `signum` (BSD semantics on Linux and macOS: it stays installed, and interrupted
    /// system calls restart).
    pub fn install(signum: c_int, handler: extern "C" fn(c_int)) -> io::Result<()> {
        // SAFETY: `signal` has no preconditions; the handlers given are async-signal-safe.
        if unsafe { signal(signum, handler) } == SIG_ERR {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Whether process `pid` exists (when in doubt, it does).
    pub fn process_exists(pid: u32) -> bool {
        let Ok(pid) = c_int::try_from(pid) else { return true };
        // SAFETY: signal 0 sends nothing; it only checks that the process exists.
        let found = unsafe { kill(pid, 0) } == 0;
        found || io::Error::last_os_error().raw_os_error() != Some(ESRCH)
    }

    /// End the process at once, without unwinding or running destructors (async-signal-safe).
    pub fn exit_now(status: c_int) -> ! {
        // SAFETY: `_exit` has no preconditions.
        unsafe { _exit(status) }
    }
}

#[cfg(not(unix))]
mod sys {
    use std::ffi::c_int;
    use std::io;

    pub const SIGINT: c_int = 2;
    pub const SIGTERM: c_int = 15;

    pub fn install(_signum: c_int, _handler: extern "C" fn(c_int)) -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
    }

    pub fn process_exists(_pid: u32) -> bool {
        true
    }

    pub fn exit_now(status: c_int) -> ! {
        std::process::exit(status)
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

/// Read and verify a whole checkpoint, passing each unit's bytes to `unit` in order.
fn scan_checkpoint(path: &Path, mut unit: impl FnMut(&[u8]) -> io::Result<()>) -> io::Result<CheckpointHead> {
    let mut r = HashReader { inner: BufReader::with_capacity(1 << 20, File::open(path)?), hash: Hash64::new() };
    let mut scan = || -> io::Result<CheckpointHead> {
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
        Ok(CheckpointHead { header, best, payload, stats, elapsed })
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

/// Write a line to stderr in one write, ignoring errors (see "Output"; `eprintln!` would panic on a closed
/// stderr, and release builds abort on a panic).
fn report(line: &str) {
    let _ = io::stderr().write_all(format!("{line}\n").as_bytes());
}

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

    /// A dropped lock is free at once, though a copy of its descriptor lives on, as in a child process between
    /// `fork` and `exec` (found as a rare failure of `a_locked_directory_is_refused` while other tests spawned
    /// processes).
    #[test]
    fn a_dropped_dir_lock_is_free_while_a_copy_of_its_descriptor_lives() {
        let dir = crate::test_support::TempDir::new("dir-lock-copy");
        let held = lock_dir(&dir).unwrap();
        let copy = held.file.try_clone().unwrap();
        drop(held);
        let again = lock_dir(&dir);
        assert!(again.is_ok(), "{again:?}");
        drop(copy);
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
