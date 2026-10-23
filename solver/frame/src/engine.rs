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
//! progress, and a [`Control`] to steer the run from outside), a [`Start`] (new units with an initial incumbent, or a
//! checkpoint to resume), an opaque `header` that identifies the search (stored in every checkpoint and compared on
//! resume), and a closure that makes one searcher per worker thread.  It returns an [`Outcome`]: whether the search
//! completed, the best value and payload, the statistics, the time and the [`Counters`].  `tests/engine.rs` runs a toy
//! problem (knapsack) through every feature.
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
//! as there are idle workers to feed: from one unit, every worker is busy after a few rounds of splits
//! in which every running worker splits, not after one split per idle worker; and splits in flight never
//! outnumber the idle workers.  A worker answers only after `MIN_POLLS` polls of its current unit: a fresh
//! unit (stolen, or the deepest one its last split left it) first goes deep enough to have something to give,
//! and no worker is made to restart its unit at every step however long the demand lasts.  Idle workers block
//! on `work_cv`; a push wakes at most as many of them as there are new units, after the lock is released.
//!
//! **Why one deque.**  A searcher's work is on its own depth-first stack, not in the deque: units exist only
//! after a yield, and running one costs the searcher a rebuild of its state (for Taxman a promotion pass and a
//! matching solved from scratch, tens to hundreds of microseconds, against about a microsecond in the critical
//! section; solver/notebook/the-engine/README.md has the numbers).  The lock is taken about twice per unit, and units
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
//! searcher runs without polling (one Taxman step takes microseconds, with a tail of milliseconds:
//! solver/notebook/the-engine/README.md) plus, for a periodic checkpoint, up to `MIN_POLLS` polls of a unit just begun;
//! then the write and its fsync; then each unit pushed in it costs a rebuild when it runs.  [`Counters`] records the
//! longest wait for the workers to park and the longest pause.
//!
//! A caller's [`CheckpointHook`] is told of the checkpoint twice in the pause, with the lock held: before the file is
//! written, with the nodes and the time it will record, and after it is written, with its path.  Every run has ended
//! by then, so whatever a searcher counts and hands over when its run ends adds up to the checkpoint's nodes.  A caller
//! that makes its own record durable when told of the first never has a record behind a checkpoint on disk (a crash
//! between the two leaves it one checkpoint ahead, which it can tell from the nodes).
//!
//! The next periodic checkpoint is due one interval after a pause ends, but never sooner than that pause
//! lasted, and its pause begins only once the released workers are all running again (`resuming` is clear:
//! no held unit is left, and the deque is empty or every live worker runs, as every one does when none is
//! live).  The coordinator waits for that without the lock, so with any interval, zero included, the workers
//! make progress and pauses take at most about half of their time.
//!
//! **Output.**  Progress lines and warnings go to stderr outside the lock, so a blocked stderr cannot stall
//! the workers, and write errors are ignored, so a closed one (a dead `tee`, EPIPE) cannot kill them either.
//!
//! **Termination.**  The search is complete when a worker that needs work finds the deque and spill empty,
//! `running == 0`, no held units and no pause; that worker sets `done` and `exit`.  (While paused, the
//! coordinator makes the same check once the pause is complete, and while no worker is live, at every look:
//! see "Workers".)  `declare_optimal` sets `exit` directly and all remaining work is discarded.  Every state
//! change that can satisfy a waiter's condition is made under the mutex and followed by a notify, so no
//! wakeup is lost.
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
//! must not depend on that race: a deadline already past (a time limit of zero), a stop signal already received or
//! a [`Control::stop`] given before the run does no search work, however the threads are scheduled.  [`run`] makes
//! that check (`Plan::stop_due`, the very test the coordinator makes, and the control's stop, applied when it is
//! attached) before it starts a worker; if it holds, it asks for the stop (`request_stop`) and starts none (`grow`
//! starts no worker once a stop is asked for), and the coordinator stops the run at its first check like any stop.
//! No unit was ever handed out, so the final checkpoint holds all of them (the starting units, or the checkpoint's),
//! the incumbent and statistics are the ones the run started with, `completed` is false and `Counters::runs` is 0; a
//! run resumed from that checkpoint while the stop still holds stops the same way and writes the same units again.
//! (A search with no units at all is complete, and says so.)  The node limit is not a stop of this kind: workers
//! enforce it when they publish their node counts, every `POLL_BATCH` polls, so a search shorter than that
//! completes whatever the limit.
//!
//! # The control
//!
//! A [`Control`] (`EngineConfig::control`) steers a run from another thread: it gives the run a ceiling, offers it
//! incumbents, stops it, changes its number of workers, and reports what it is doing.  Without one, nothing below
//! happens: a run goes exactly as it would.
//!
//! **Attaching.**  [`run`] attaches its control (a weak reference to the run's shared state, behind the trait
//! `Remote`, since the control does not know the unit type) before it starts a worker, and detaches it when the run
//! ends, a panic unwinding included, leaving the run's incumbent with the control.  The control records every request
//! it is given; a request made while a run is attached is also made of the run, and one made before is made of it when
//! it attaches.  Both happen under the control's own mutex, which attaching and detaching take too, so each request
//! reaches a run exactly once: when it is made, or when the run attaches.  Making one twice would change nothing
//! anyway (the ceiling is the least given, the incumbent the best, a stop a flag, the workers the last number given).
//! The control's mutex comes before the `state` lock and the payload's, and the run never takes it holding either.
//!
//! **Ceiling.**  A proven upper bound on the best value: an incumbent that reaches it is optimal, and the run ends as
//! if a searcher had called `declare_optimal`.  Two events can make that so, an offer that raises the incumbent and a
//! ceiling that comes down to it, and each checks for the other: an offer stores the incumbent and then loads the
//! ceiling, and `set_ceiling` stores the ceiling and then loads the incumbent, all four accesses `SeqCst`.  In their
//! single total order one side's store comes before the other side's load (the store-buffering pattern of Dekker's
//! algorithm), so at least one of the two loads sees the other side's store, and a reached ceiling always ends the
//! run, whichever came first.  The state lock is taken only then, and the payload's is released first (a checkpoint
//! takes them the other way round).  Attaching offers the control's incumbent and then gives its ceiling, so a run
//! whose starting incumbent already reaches the ceiling sets `exit` before it starts a worker and does no search work.
//!
//! **Outside incumbents and stops.**  An offer through the control is `Ctx::offer`, and one made before the run
//! raises its starting incumbent.  A stop through the control is a stop like a limit's (a final checkpoint); one given
//! before the run is a stop that holds when it starts ("Starting stopped").
//!
//! **Workers.**  The run has a *target* number of workers (`EngineConfig::threads`, or the control's), and counts the
//! *live* ones: started, and not yet retired or exited.  Every "all workers" of the protocol means the live ones: full
//! occupancy and the end of `resuming` are `running == live`, and the demand counts live workers.  Each worker has a
//! number, never reused, which indexes its `held` slot, statistics and clock; those lists only grow.
//!
//! *Growing.*  While fewer workers are live than the target, and neither `exit` nor a stop is set, the coordinator
//! starts workers (`grow`): it reserves their slots under the lock, so that they count as live at once, and spawns
//! their threads outside it.  A worker that cannot be started stops the run with the error, as one at the start does.
//! A new worker takes work like any idle one; in a pause it parks like the others (it holds nothing, so the
//! checkpoint has every unit without it).
//!
//! *Retiring.*  A worker leaves only holding no unit, so conservation is untouched.  An idle worker retires in
//! `acquire` when, outside a pause, it has found no unit, the termination test has not ended the run, and more workers
//! are live than the target; lowering the target wakes the idle workers to check.  A running worker beyond the target
//! retires in `finish_run` if it has units to give and no pause is on: it hands all of them, its deepest too, to the
//! deque in the critical section in which it stops running (a split that keeps nothing), and leaves; one with nothing
//! to give goes to `acquire` like any worker that needs work.  To bring running workers back promptly, the demand of
//! "Splitting" also counts the live workers beyond the target that are not idle (the idle ones retire by themselves),
//! and it does not count as needing work the idle workers that will retire.  Without a control the target and the live
//! workers are the configured threads from start to end, and no worker grows or retires.
//!
//! *Suspending.*  A target of 0 suspends the search (not a pause: no worker is parked, none is live): every worker
//! retires as above, the idle ones at once and the running ones at their next answer, and the units wait in the deque
//! and the spill until the target rises, when the coordinator starts workers as for any rise.  `EngineConfig::threads`
//! is at least 1, so a run starts with workers unless its control was given 0 before it started.  The rest of the run
//! goes on without workers: a pause finds none running at once, so a checkpoint, periodic or the final one of a stop,
//! writes the queued units; a stop, a stop signal, a deadline and a ceiling given or reached end the run as ever; and
//! `resuming` ends as soon as no worker is live, since then every live worker runs.
//!
//! *Conservation and termination as workers come and go.*  A worker leaves only holding no unit: a running one hands
//! its units over in the critical section in which it stops running, and an idle one holds none, since a held unit is
//! the first thing its worker takes after a pause and no worker retires in a pause.  So with no worker live every unit
//! is in the deque or the spill, from which only workers take, and no unit moves until workers start again (the
//! coordinator's checkpoints copy them).  The termination test is made by every worker that needs work and finds none,
//! before it may retire, and by the coordinator after a pause and whenever no worker is live.  When the search's last
//! run ends, its worker has nothing to hand over, needs work and makes the test (or, in a pause, the coordinator does),
//! whatever the target, so a run whose last unit ends as it is suspended completes.  A run with no worker live is
//! suspended, not complete: a retiring idle worker has just seen its test fail with nothing to take, outside a pause,
//! so another worker runs or holds a unit and is live; the last worker to leave before the run ends is therefore a
//! running one, which leaves its units in the deque.  The one exception, a run suspended from its start with no unit
//! at all, is complete, and the coordinator's test ends it at its first look.  With a target of at least 1 a worker
//! retires only while more are live than the target, so some worker is live until the run ends, and the units a
//! retiring one leaves are taken: it wakes idle workers for them, and the running ones come back for work.  So the last
//! worker to need work still makes the termination test.
//!
//! # Instrumentation
//!
//! Each worker's time is split into disjoint intervals, summed in [`Counters`]: *busy* (inside
//! [`Searcher::run`]), *idle* (waiting for work outside pauses), *parked* (waiting in a pause) and *lock wait*
//! (waiting for the `state` lock after a run).  The rest of its lifetime (from the start of the workers, or its own
//! start if it was added later, to its retirement or the end of the run) is thread start-up and the engine's own
//! bookkeeping.  The lifetimes summed over workers are the *capacity*, which is `wall × threads` when the number of
//! workers does not change; the effective parallelism is busy time over capacity.  Also counted: splits and the units
//! they gave, steals (units taken from the front by a worker that had waited), runs and tiny runs (at most one poll:
//! units with nothing left), when all workers first ran at once, the most workers alive at once and those added and
//! retired, and the pauses.  Four times show where a parallel search loses time to its shape: the idle time before
//! all workers first ran at once (the *ramp*) and after the last moment a worker finished a run while all ran (the
//! *tail*, also measured in wall time), a run's time to its first poll (the searcher's *rebuild* of its state from the
//! unit) and from its first poll to its second (its *first step*).  The clocks are read only at unit boundaries,
//! waits and a run's first two polls (which `poll` finds by testing one byte of `Ctx`), never per poll; the state
//! keeps a running total of the idle waits, so that the ramp's and the tail's idle times are snapshots of it.
//! [`Counters::summary`] is the compact form that ends every progress line.
//!
//! # Platforms
//!
//! The engine uses only `std` (plus the POSIX `signal`, `kill` and `_exit`, declared in `sys`, and on Linux
//! `sched_setscheduler`, declared in `idle`) and behaves the same on Linux and macOS, but for
//! [`EngineConfig::background`], which does nothing on macOS.  Checkpoints are durable on both: the file is
//! written, `sync_all`ed (`fsync` on Linux, `F_FULLFSYNC` on macOS) and renamed over the old one, and the directory
//! is then synced so the
//! rename itself survives a crash (needed on Linux; on platforms that cannot sync a directory this is skipped
//! and the rename is still atomic).  Worker stacks reserve [`WORKER_STACK_SIZE`] of address space each,
//! committed only as touched.  `EngineConfig::default` takes `available_parallelism`, which on Linux honors
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
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Once, PoisonError, Weak};
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
    /// Worker threads (at least 1); [`Control::set_workers`] changes their number, and can suspend the run with 0.
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
    /// Text the caller adds to the end of every progress line (what its searchers have counted, say).
    pub progress_note: Option<ProgressNote>,
    /// Emulation of many workers on few CPUs, for measuring what a larger machine would do in counts (nodes,
    /// lookups), not time: at most this many of the `threads` workers search at once, taking turns every
    /// `POLL_BATCH` polls in the order they ask (a panic in a worker gives its turn up).  `None`: no limit, and
    /// the check costs nothing but one test every `POLL_BATCH` polls.  The workers' busy time includes their
    /// waits for a turn, so the [`Counters`] times mean nothing with it.
    pub cores: Option<usize>,
    /// A handle through which another thread steers the run while it goes (a ceiling, outside incumbents, a stop, the
    /// number of workers) and watches it: see [`Control`] and "The control" in the module docs.  `None`, the default:
    /// the run goes as configured.
    pub control: Option<Arc<Control>>,
    /// Each worker puts itself under the kernel's idle scheduling policy as it starts ([`background_priority`]), so
    /// that the search takes the CPU that nothing else wants.  The thread that calls [`run`], which coordinates, keeps
    /// its policy.  Off by default.
    pub background: bool,
    /// Told of each checkpoint, with every worker parked: once before the file is written and once after
    /// ([`CheckpointEvent`]; "Checkpoint" in the module docs).  `None`, the default: nobody is told.
    pub checkpoint_hook: Option<CheckpointHook>,
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
            progress_note: None,
            cores: None,
            control: None,
            background: false,
            checkpoint_hook: None,
        }
    }
}

/// What a [`CheckpointHook`] is told, with every worker parked, so that the search stands still: `nodes` is the steps of
/// every run of the search so far, as the checkpoint records them, and `elapsed` its time.
#[derive(Clone, Copy, Debug)]
pub enum CheckpointEvent<'a> {
    /// The checkpoint is about to be written: what the caller records of the search can be made durable first, so that
    /// it is never behind a checkpoint on disk.
    Writing { nodes: u64, elapsed: Duration },
    /// The checkpoint has been written to `path`, which the caller may open and read (the next checkpoint replaces it).
    Written { path: &'a Path, nodes: u64, elapsed: Duration },
}

/// A caller's hook on the checkpoints ([`EngineConfig::checkpoint_hook`]), called by the coordinator with the engine's
/// lock held and every worker parked: it must be quick, must not wait for a worker, and cannot fail the run.
#[derive(Clone)]
pub struct CheckpointHook(pub Arc<dyn Fn(CheckpointEvent<'_>) + Send + Sync>);

impl std::fmt::Debug for CheckpointHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CheckpointHook")
    }
}

/// What a caller adds to the end of the engine's progress lines ([`EngineConfig::progress_note`]): called with the
/// engine's lock held, so it must be quick and must not wait for anything a worker holds while it waits for the engine.
#[derive(Clone)]
pub struct ProgressNote(pub Arc<dyn Fn() -> String + Send + Sync>);

impl std::fmt::Debug for ProgressNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ProgressNote")
    }
}

/// Steers a run from another thread while it goes, and watches it (see "The control" in the module docs).  Attach it
/// to a run with [`EngineConfig::control`]; share it as `Arc<Control>`.  It remembers what it is told, so a request
/// made before a run is applied when the run starts, and it keeps the run's incumbent when the run ends.  A control
/// belongs to one search, a run and its resumes, since the incumbents and the ceiling it carries are that search's;
/// it is attached to one run at a time, and a stop it was given holds for every run it is attached to later.
pub struct Control {
    state: Mutex<ControlState>,
}

/// What a [`Control`] has been told, and the run it is attached to.
#[derive(Default)]
struct ControlState {
    /// The run, from before its first worker starts until it ends.
    run: Option<Weak<dyn Remote>>,
    /// The least ceiling given.
    ceiling: Option<u64>,
    /// The best incumbent offered while no run was attached, or the last run's incumbent (which is at least as good as
    /// every one offered before, since that run started from them).
    best: Option<(u64, Vec<u8>)>,
    stop: bool,
    /// The last number of workers given.
    workers: Option<usize>,
}

impl Control {
    /// A control that has been told nothing.
    #[must_use]
    pub fn new() -> Arc<Control> {
        Arc::new(Control { state: Mutex::new(ControlState::default()) })
    }

    /// A ceiling: a value the incumbent can never exceed (a proven upper bound on the best value).  Once the incumbent
    /// reaches it, the incumbent is optimal and the run ends as if a searcher had called [`Ctx::declare_optimal`]
    /// ([`Outcome::optimal_declared`]).  It can be given before the run or while it runs, any number of times; the
    /// lowest value given counts.  A run whose incumbent already reaches it when the run starts does no search work
    /// (no worker takes a unit, `Counters::runs == 0`, completed, optimal declared), and one whose incumbent reaches it
    /// when it is given ends at once.  `u64::MAX`, which says nothing, is ignored by the run.
    pub fn set_ceiling(&self, value: u64) {
        let mut state = lock(&self.state);
        state.ceiling = Some(state.ceiling.map_or(value, |ceiling| ceiling.min(value)));
        if let Some(run) = Self::attached(&state) {
            run.set_ceiling(value);
        }
    }

    /// The lowest ceiling given, if any.
    #[must_use]
    pub fn ceiling(&self) -> Option<u64> {
        lock(&self.state).ceiling
    }

    /// Offer an incumbent from outside, exactly as [`Ctx::offer`] does (the payload must be what the searchers would
    /// offer); a run's incumbent that reaches the ceiling ends it.  Before the run starts the best offered is kept, and
    /// the run starts from the better of it and its [`Start`]'s incumbent (fresh or from the checkpoint).  Returns
    /// whether it became the run's incumbent, or, with no run attached, whether it is kept as the best so far.
    pub fn offer(&self, value: u64, payload: &[u8]) -> bool {
        let mut state = lock(&self.state);
        if let Some(run) = Self::attached(&state) {
            return run.offer(value, payload);
        }
        if state.best.as_ref().is_some_and(|(best, _)| value <= *best) {
            return false;
        }
        state.best = Some((value, payload.to_vec()));
        true
    }

    /// The incumbent's value and payload: the run's while it runs (and after it ended), else the best offered, if
    /// any.
    #[must_use]
    pub fn best(&self) -> Option<(u64, Vec<u8>)> {
        let state = lock(&self.state);
        match Self::attached(&state) {
            Some(run) => Some(run.incumbent()),
            None => state.best.clone(),
        }
    }

    /// Stop the run as a limit does (with a final checkpoint if one is configured, `completed == false`).  Given
    /// before the run, the run starts stopped ("Starting stopped" in the module docs), and so does every later run
    /// this control is attached to.
    pub fn stop(&self) {
        let mut state = lock(&self.state);
        state.stop = true;
        if let Some(run) = Self::attached(&state) {
            run.stop();
        }
    }

    /// Ask the run attached for a checkpoint now, as if its interval had passed: it is written once the workers run
    /// (not while they resume from a pause), and the next periodic one is due an interval after it.  Without a
    /// checkpoint file ([`EngineConfig::checkpoint`]) it does nothing, and so it does with no run attached: a
    /// checkpoint is of a run, so the request is not kept for a later one.  Returns whether a run was attached.
    pub fn checkpoint(&self) -> bool {
        let state = lock(&self.state);
        let run = Self::attached(&state);
        if let Some(run) = &run {
            run.checkpoint();
        }
        run.is_some()
    }

    /// Whether a [`stop`](Control::stop) has been given (it holds for every run the control is attached to), for a
    /// caller that works beside a run and ends with it.
    #[must_use]
    pub fn stopped(&self) -> bool {
        lock(&self.state).stop
    }

    /// The number of workers the run should have from now on.  While the run goes, it starts workers until it has that
    /// many (each makes its own searcher with `make(i)`, `i` a new number) and retires workers that have no unit, or
    /// have just handed theirs over, until it has no more.  0 suspends the search: the units wait in the run's deque
    /// until the number is raised again, and the run is not over meanwhile ("Suspending" in the module docs).  Given
    /// before the run, it replaces [`EngineConfig::threads`], 0 included.
    pub fn set_workers(&self, workers: usize) {
        let mut state = lock(&self.state);
        state.workers = Some(workers);
        if let Some(run) = Self::attached(&state) {
            run.set_workers(workers);
        }
    }

    /// The number set by [`set_workers`](Control::set_workers), if any.
    #[must_use]
    pub fn workers(&self) -> Option<usize> {
        lock(&self.state).workers
    }

    /// What the run is doing now, if it is running (`None` before it starts and after it ends).
    #[must_use]
    pub fn status(&self) -> Option<Status> {
        Self::attached(&lock(&self.state)).map(|run| run.status())
    }

    /// The run attached, if any.
    fn attached(state: &ControlState) -> Option<Arc<dyn Remote>> {
        state.run.as_ref().and_then(Weak::upgrade)
    }

    /// Attach `run`, making of it every request recorded so far (see "Attaching").
    ///
    /// # Errors
    ///
    /// `InvalidInput` if another run is attached.
    fn attach<R: Remote + 'static>(&self, run: &Arc<R>) -> io::Result<()> {
        let mut state = lock(&self.state);
        if Self::attached(&state).is_some() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "the control is attached to another run"));
        }
        // The incumbent before the ceiling, which the incumbent may reach.
        if let Some((value, payload)) = &state.best {
            run.offer(*value, payload);
        }
        if let Some(ceiling) = state.ceiling {
            run.set_ceiling(ceiling);
        }
        if state.stop {
            run.stop();
        }
        if let Some(workers) = state.workers {
            run.set_workers(workers);
        }
        let run: Weak<R> = Arc::downgrade(run);
        state.run = Some(run);
        Ok(())
    }

    /// Detach `run`, whose workers have all exited, keeping its incumbent.  The incumbent is read under the control's
    /// mutex, through which every outside offer goes, so it is the run's last.
    fn detach(&self, run: &dyn Remote) {
        let mut state = lock(&self.state);
        state.run = None;
        state.best = Some(run.incumbent());
    }
}

/// By hand, because the attached run is not `Debug`: what the control has been told, and whether a run is attached.
impl std::fmt::Debug for Control {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = lock(&self.state);
        f.debug_struct("Control")
            .field("attached", &Self::attached(&state).is_some())
            .field("ceiling", &state.ceiling)
            .field("best", &state.best.as_ref().map(|(value, _)| value))
            .field("stop", &state.stop)
            .field("workers", &state.workers)
            .finish()
    }
}

/// What a run is doing, from [`Control::status`].  The times are summed over workers, as in [`Counters`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    /// Workers alive: started, and not yet retired or exited.
    pub live: usize,
    /// Workers inside [`Searcher::run`].
    pub running: usize,
    /// Workers waiting for work outside a pause.
    pub idle: usize,
    /// Units in the deque, in memory or spilled.
    pub queued: usize,
    /// Time spent waiting for work outside pauses so far, waits still going on included.
    pub idle_time: Duration,
    /// The workers' lifetimes so far (see [`Counters::capacity`]).
    pub capacity: Duration,
    /// The incumbent's value.
    pub best: u64,
    /// Nodes published in this invocation.
    pub nodes: u64,
    /// Nodes over every run of the search: those of the runs before this one, from the checkpoint it resumed, and
    /// `nodes`.
    pub all_nodes: u64,
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
    /// The most workers alive at once: the configured threads, unless a [`Control`] changed their number (or the run
    /// started none).
    pub threads: usize,
    /// Workers started after the first ones, and workers retired (both through a [`Control`]).
    pub spawned: u64,
    pub retired: u64,
    /// The workers' lifetimes summed: each from the start of the workers (or its own start, if it was added later) to
    /// its retirement or the end of the run.  Exactly `wall × threads` when their number did not change.
    pub capacity: Duration,
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
    /// From starting the workers until every live worker was running a unit at the same time (`None`: never).
    pub full_occupancy: Option<Duration>,
    /// Idle time before full occupancy was first reached (all of it if it never was).
    pub ramp_idle: Duration,
    /// The wall time from the last moment a worker finished a run while every live worker was running to the end of
    /// the run, and the idle time after that moment (zero if that never happened).
    pub tail: Duration,
    pub tail_idle: Duration,
    /// Summed over runs of units: the time from the start of [`Searcher::run`] to its first poll (for the Taxman
    /// searcher, rebuilding its state from the unit).
    pub rebuild: Duration,
    /// Summed over runs that polled twice: the time from the first poll to the second (the unit's first node; for the
    /// Taxman searcher, its bounds solved from scratch), and the number of those runs.
    pub first_step: Duration,
    pub first_steps: u64,
    /// Pauses (each checkpoint, and the final one of a stop).
    pub pauses: u64,
    /// Total and longest time from asking the workers to yield until releasing them (or exiting).
    pub pause_total: Duration,
    pub pause_max: Duration,
    /// Longest time from asking the workers to yield until every one had parked.
    pub park_max: Duration,
}

impl Counters {
    /// Add the counters of another run, as if it had followed this one: counts and times add up, the maxima take the
    /// larger, and full occupancy the later (never if either run never reached it).
    pub fn add(&mut self, other: &Counters) {
        self.threads = self.threads.max(other.threads);
        for (a, b) in [
            (&mut self.spawned, other.spawned),
            (&mut self.retired, other.retired),
            (&mut self.first_steps, other.first_steps),
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
            (&mut self.capacity, other.capacity),
            (&mut self.ramp_idle, other.ramp_idle),
            (&mut self.tail, other.tail),
            (&mut self.tail_idle, other.tail_idle),
            (&mut self.rebuild, other.rebuild),
            (&mut self.first_step, other.first_step),
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

    /// Effective parallelism: busy time over the capacity, from 0 to 1.
    #[must_use]
    pub fn efficiency(&self) -> f64 {
        self.share(self.busy)
    }

    /// `time` as a fraction of the capacity (of `wall × threads` for counters that have none, made by hand).
    #[must_use]
    pub fn share(&self, time: Duration) -> f64 {
        let capacity = if self.capacity > Duration::ZERO {
            self.capacity.as_secs_f64()
        } else {
            self.wall.as_secs_f64() * self.threads as f64
        };
        if capacity > 0.0 {
            time.as_secs_f64() / capacity
        } else {
            0.0
        }
    }

    /// One compact line: effective parallelism, the shares of idle and lock-wait time, splits, steals, time to
    /// full occupancy, the shares of the ramp's and the tail's idle time, of the rebuilds and of the first steps,
    /// and (if any) the pauses with the share of parked time, the longest pause and the longest wait for the
    /// workers to park.
    #[must_use]
    pub fn summary(&self) -> String {
        let full = self.full_occupancy.map_or_else(|| "never".to_owned(), fmt_short);
        let percent = |time| 100.0 * self.share(time);
        let mut line = format!(
            "eff {:.1}% idle {:.1}% lock {:.1}% splits {} steals {} full {full}",
            100.0 * self.efficiency(),
            percent(self.idle),
            percent(self.lock_wait),
            si(self.splits as f64),
            si(self.steals as f64),
        );
        line += &format!(
            " ramp {:.1}% tail {:.1}% rebuild {:.1}% first {:.1}%",
            percent(self.ramp_idle),
            percent(self.tail_idle),
            percent(self.rebuild),
            percent(self.first_step),
        );
        if self.pauses > 0 {
            line += &format!(
                " pauses {} ({:.1}%) max {} park {}",
                self.pauses,
                percent(self.parked),
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
    /// The whole tree was explored, or the incumbent was proven optimal (`optimal_declared`).  False when a limit,
    /// a stop signal or a [`Control::stop`] stopped the run (a final checkpoint was written if configured).
    pub completed: bool,
    /// A searcher called [`Ctx::declare_optimal`], or the incumbent reached the ceiling of the run's [`Control`].
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

/// Run a search to completion, to a limit, until a stop signal (see [`stop_on_signals`]) or a [`Control::stop`],
/// or until the incumbent is proven optimal (by a searcher, or by the control's ceiling).
///
/// `header` is opaque caller data stored in every checkpoint (everything the caller needs to rebuild its
/// searchers on resume); read it back with [`read_checkpoint_header`] or [`read_checkpoint`].  Worker `i` uses
/// `make(i)`, called on its own thread; the workers are numbered from 0, and those a control adds take the next
/// numbers.  When the run completes, the checkpoint file (if configured) is deleted unless
/// `cfg.keep_completed_checkpoint`.
///
/// # Errors
///
/// I/O failures while spilling, checkpointing or resuming (the last checkpoint written stays valid), a
/// damaged checkpoint (`InvalidData`), a checkpoint whose header differs from `header` (`InvalidInput`), a worker
/// thread that cannot be started, and a control attached to another run (`InvalidInput`).
///
/// # Panics
///
/// A panic in a searcher (or in `make`) stops all workers and propagates to the caller (in a build that
/// unwinds; release builds abort).
pub fn run<U, S, F>(cfg: &EngineConfig, start: Start<U>, header: &[u8], make: F) -> io::Result<Outcome>
where
    U: Unit + 'static,
    S: Searcher<U>,
    F: Fn(usize) -> S + Sync,
{
    let t0 = Instant::now();
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
    // In an `Arc`, of which an attached control holds a weak reference ("Attaching").
    let arc = Arc::new(Shared::new(cfg, init, t0));
    let shared: &Shared<U> = &arc;
    shared.load(fresh, resume.as_deref())?;
    let attached = match &cfg.control {
        Some(control) => {
            control.attach(&arc)?;
            Some(Attachment { control, shared })
        }
        None => None,
    };
    // Limits, schedules and the instrumentation count from here, so a slow resume cannot use up the time limit.
    let began = Instant::now();
    shared.lock().began = began;
    let plan = Plan {
        checkpoint: cfg.checkpoint.as_ref().map(|(path, every)| (path.as_path(), *every)),
        progress: cfg.progress,
        began,
        deadline: cfg.time_limit.and_then(|limit| began.checked_add(limit)),
        label: &cfg.label,
        note: cfg.progress_note.as_ref(),
        header,
        hook: cfg.checkpoint_hook.as_ref(),
    };
    // See "Starting stopped": a stop that already holds is asked for before any worker exists to take a unit, so that
    // none is started, and the coordinator carries it out as it does every stop.
    if plan.stop_due() {
        shared.request_stop();
    }
    let make = &make;
    let background = cfg.background;
    thread::scope(|scope| {
        let spawn = |i: usize| {
            thread::Builder::new()
                .name(format!("engine-{i}"))
                .stack_size(WORKER_STACK_SIZE)
                .spawn_scoped(scope, move || {
                    if background {
                        background_priority();
                    }
                    shared.worker(i, make);
                })
                .map(drop)
        };
        shared.grow(&spawn, true);
        shared.coordinate(&plan, &spawn);
    });
    // Detached before the outcome is taken, the incumbent is final: the control keeps the outcome's.
    drop(attached);
    let finished = plan.checkpoint.map(|(path, _)| path).filter(|_| !cfg.keep_completed_checkpoint);
    shared.outcome(finished, Instant::now())
}

/// Detaches a run from its control when the run ends, however it ends (a panic unwinding included), leaving the run's
/// incumbent with the control.
struct Attachment<'r, U: Unit> {
    control: &'r Control,
    shared: &'r Shared<U>,
}

impl<U: Unit> Drop for Attachment<'_, U> {
    fn drop(&mut self) {
        self.control.detach(self.shared);
    }
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

/// Puts the calling thread under the kernel's idle scheduling policy (Linux `SCHED_IDLE`): any other process, whatever
/// its nice value, runs before it, so the thread takes exactly the CPU that nothing else wants.  A worker calls it as
/// it starts when [`EngineConfig::background`] is set.  It is set for the thread, not the process: a thread cannot
/// leave the policy without privilege, and the threads and processes it starts inherit it, which is why the thread that
/// coordinates a run does not call it.
///
/// Where the policy cannot be set (another system than Linux, or a kernel that refuses), the search goes on at normal
/// priority and this says so once, on stderr.
pub fn background_priority() {
    static SAID: Once = Once::new();
    if let Err(e) = idle::set() {
        SAID.call_once(|| match e.kind() {
            io::ErrorKind::Unsupported => report("note: --background does nothing on this system"),
            _ => {
                report(&format!("warning: --background: cannot set the idle scheduling policy ({e}); going on without"))
            }
        });
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
    /// The polls of the current run still to time: 2 when it begins, 0 after its second poll ("Instrumentation").
    untimed: u8,
    /// When the current run began, then when it first polled.
    mark: Instant,
    /// The rebuilds and first steps timed since the last `finish_run`, and the runs whose first step was.
    rebuild: Duration,
    first_step: Duration,
    first_steps: u64,
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
            untimed: 0,
            mark: shared.t0,
            rebuild: Duration::ZERO,
            first_step: Duration::ZERO,
            first_steps: 0,
            yielding: false,
            split_claimed: false,
            published: 0,
            stats: Stats::default(),
            pushed: Vec::new(),
        }
    }

    /// Whether to keep going.  Costs a test of a byte, one decrement and one relaxed atomic load (and a reading of
    /// the clock at a run's first two polls).
    #[inline]
    #[must_use]
    pub fn poll(&mut self) -> Poll {
        if self.untimed != 0 {
            self.time_poll();
        }
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

    /// Time the rebuild (up to the run's first poll) or the first step (up to its second).
    #[cold]
    #[inline(never)]
    fn time_poll(&mut self) {
        let now = Instant::now();
        let time = now.saturating_duration_since(self.mark);
        if self.untimed == 2 {
            self.rebuild += time;
        } else {
            self.first_step += time;
            self.first_steps += 1;
        }
        self.mark = now;
        self.untimed -= 1;
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
    /// (stored atomically with the value) and `true` is returned.  If it reaches the ceiling of the run's
    /// [`Control`], the run ends as with [`declare_optimal`](Ctx::declare_optimal).
    pub fn offer(&mut self, value: u64, payload: &[u8]) -> bool {
        self.shared.offer(value, payload)
    }

    /// The searcher has proven the incumbent optimal: discard all remaining work and complete the run.
    /// Every later `poll` (in every worker) returns `Yield`.
    pub fn declare_optimal(&mut self) {
        self.shared.declare_optimal();
        self.yielding = true;
    }

    /// This worker's statistics; the engine sums them over workers and runs.
    #[inline]
    pub fn stats(&mut self) -> &mut Stats {
        &mut self.stats
    }

    /// This worker's number: `0..threads` for the first ones, the next numbers for those a [`Control`] adds.
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
/// The value of `Shared::ceiling` when there is none.
const NO_CEILING: u64 = u64::MAX;

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
    note: Option<&'p ProgressNote>,
    header: &'p [u8],
    hook: Option<&'p CheckpointHook>,
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
    /// The control's ceiling ([`NO_CEILING`]: none), read when an offer raises `best` (see "Ceiling").
    ceiling: AtomicU64,
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
    /// Each worker's deepest unit while paused.  This and the other lists by worker grow as workers are added (see
    /// "Workers"), and a worker's number is never reused.
    held: Vec<Option<U>>,
    /// Units in `held`.
    held_units: usize,
    /// Each worker's statistics as of its last finished run.
    worker_stats: Vec<Stats>,
    /// Workers alive: started (counted from the moment their slots are reserved), not yet retired or exited.
    live: usize,
    /// How many workers should be live (see "Workers"); 0 suspends the search.
    target: usize,
    /// Workers inside `Searcher::run`.
    running: usize,
    /// Workers blocked on `work_cv`.
    idle: usize,
    /// Split requests open (counted in `signal`) or claimed and not yet answered.
    requested: usize,
    pause: bool,
    /// Since the start or the last release from a pause, until the workers all run again: no held unit left,
    /// and the deque empty or every live worker running.  No periodic checkpoint begins meanwhile.
    resuming: bool,
    exit: bool,
    done: bool,
    optimal: bool,
    stop_requested: bool,
    /// A checkpoint asked for through the control ([`Control::checkpoint`]), due as soon as the workers run: taken by
    /// the next pause.
    checkpoint_asked: bool,
    panicked: bool,
    error: Option<io::Error>,
    counters: Counters,
    /// Each worker's time accounting.
    clocks: Vec<Clock>,
    /// The idle waits completed so far, summed over workers: a running total, of which the ramp and the tail take
    /// snapshots ("Instrumentation").
    idle_done: Duration,
    /// The last moment a worker finished a run while every live worker was running, and `idle_done` then.
    tail_from: Option<(Instant, Duration)>,
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

/// Where a worker's time went (see "Instrumentation"); its idle time is in `State::idle_done`.
#[derive(Clone, Copy)]
struct Clock {
    /// When the worker began to count as alive: the start of the workers for the first ones, the reservation of its
    /// slot for one added later.
    born: Instant,
    /// When it retired (`born` for one that could not be started); `None` for one alive until the end of the run.
    gone: Option<Instant>,
    busy: Duration,
    parked: Duration,
    /// Waiting for the `state` lock after a run.
    lock_wait: Duration,
    /// Since when the worker has been running its current unit.
    running_since: Option<Instant>,
    /// Since when the worker has been waiting, and whether it waits in a pause.
    waiting_since: Option<(Instant, bool)>,
}

impl Clock {
    fn new(born: Instant) -> Self {
        Clock {
            born,
            gone: None,
            busy: Duration::ZERO,
            parked: Duration::ZERO,
            lock_wait: Duration::ZERO,
            running_since: None,
            waiting_since: None,
        }
    }
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
    /// The state of a run with no worker yet, whose target is `cfg.threads` (`grow` starts the workers).
    fn new(cfg: &EngineConfig, init: Init, t0: Instant) -> Self {
        let prefix = format!("txspill-{}-{}", std::process::id(), RUN_SEQ.fetch_add(1, Ordering::Relaxed));
        Shared {
            signal: Line(AtomicU64::new(0)),
            best: Line(AtomicU64::new(init.best)),
            nodes: Line(AtomicU64::new(0)),
            ceiling: AtomicU64::new(NO_CEILING),
            payload: Mutex::new(init.payload),
            state: Mutex::new(State {
                mem: VecDeque::new(),
                spill: Spill { dir: cfg.spill_dir.clone(), prefix, seq: 0, segs: Vec::new(), units: 0 },
                held: Vec::new(),
                held_units: 0,
                worker_stats: Vec::new(),
                live: 0,
                target: cfg.threads.max(1),
                running: 0,
                idle: 0,
                requested: 0,
                pause: false,
                resuming: true,
                exit: false,
                done: false,
                optimal: false,
                stop_requested: false,
                checkpoint_asked: false,
                panicked: false,
                error: None,
                counters: Counters::default(),
                clocks: Vec::new(),
                idle_done: Duration::ZERO,
                tail_from: None,
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

    /// The incumbent is optimal (a searcher proved it, or it reached the ceiling): discard all remaining work and
    /// complete the run.
    fn declare_optimal(&self) {
        let mut st = self.lock();
        st.optimal = true;
        self.set_exit(&mut st);
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
        // Every live worker is parked and some unit is left (else the run has ended), so this ends `resuming` only when
        // no worker is live ("Suspending").
        self.note_running(st, Instant::now());
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
            // Beyond the target, idle workers retire rather than wait for work, and running ones are asked to come
            // back to retire ("Workers"); without a control there is no surplus, and the demand is the idle workers
            // that the queued and held units will not serve.
            let surplus = st.live.saturating_sub(st.target);
            let waiting = st.idle.saturating_sub(surplus);
            waiting.saturating_sub(st.queued() + st.held_units) + surplus.saturating_sub(st.idle)
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
        let mut next = self.acquire(self.lock(), &mut ctx);
        while let Some(unit) = next {
            let turn = self.gate.as_ref().map(Gate::turn);
            searcher.run(unit, &mut ctx);
            drop(turn);
            next = self.finish_run(&mut ctx);
        }
    }

    /// Count the worker of `ctx` as running a unit from now on.
    fn begin_run(&self, st: &mut State<U>, ctx: &mut Ctx<'_, U>) {
        let now = Instant::now();
        ctx.run_start = ctx.polls();
        ctx.untimed = 2;
        ctx.mark = now;
        st.running += 1;
        st.counters.runs += 1;
        st.clocks[ctx.worker].running_since = Some(now);
        self.note_running(st, now);
        self.update_split(st);
    }

    /// What follows when every live worker may have come to run (a run began, a worker retired, workers could not be
    /// started, or none is live at the start or at a release): the first full occupancy, which ends the ramp, and the
    /// end of `resuming` (see "Checkpoint"), which with no worker live is at once.
    fn note_running(&self, st: &mut State<U>, now: Instant) {
        let all = st.running == st.live;
        if all && st.live > 0 && st.counters.full_occupancy.is_none() {
            st.counters.full_occupancy = Some(now.saturating_duration_since(st.began));
            // No live worker waits, so every idle wait so far is complete.
            st.counters.ramp_idle = st.idle_done;
        }
        if st.resuming && st.held_units == 0 && (st.queue_empty() || all) {
            st.resuming = false;
            self.coord_cv.notify_all();
        }
    }

    /// Wait for a unit to run (counting it in `running`), or return `None` when the worker must leave: the run is
    /// over, or it retires (see "Workers").
    fn acquire(&self, mut guard: MutexGuard<'_, State<U>>, ctx: &mut Ctx<'_, U>) -> Option<U> {
        let i = ctx.worker;
        let mut starved = false;
        loop {
            let st = &mut *guard;
            if st.exit {
                return self.leave(st, i);
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
                    self.begin_run(st, ctx);
                    return Some(unit);
                }
                if st.running == 0 && !st.has_work() {
                    st.done = true;
                    self.set_exit(st);
                    return self.leave(st, i);
                }
                // (A spill segment that cannot be read back pauses the run, and no worker retires in a pause.)
                if !st.pause && st.live > st.target {
                    return self.leave(st, i);
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
            st.clocks[i].waiting_since = None;
            let waited = since.elapsed();
            if paused {
                st.clocks[i].parked += waited;
            } else {
                st.idle_done += waited;
            }
        }
    }

    /// Worker `i` leaves for good, holding no unit: at the end of the run (`exit` is set), or before it to retire
    /// (see "Workers"), which ends its lifetime now and changes the demand and the occupancy.
    fn leave(&self, st: &mut State<U>, i: usize) -> Option<U> {
        st.live -= 1;
        if !st.exit {
            let now = Instant::now();
            st.clocks[i].gone = Some(now);
            st.counters.retired += 1;
            self.update_split(st);
            self.note_running(st, now);
        }
        None
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
        if st.running == st.live && !st.exit {
            // A drop from full occupancy: the tail begins here, unless the workers all run at once again later.  (Not
            // once the run is over: the workers leaving one by one would each seem to drop from it.)
            st.tail_from = Some((end, st.idle_done));
        }
        st.running -= 1;
        let clock = &mut st.clocks[i];
        clock.lock_wait += end.elapsed();
        if let Some(since) = clock.running_since.take() {
            clock.busy += end.saturating_duration_since(since);
        }
        let c = &mut st.counters;
        c.rebuild += std::mem::take(&mut ctx.rebuild);
        c.first_step += std::mem::take(&mut ctx.first_step);
        c.first_steps += std::mem::take(&mut ctx.first_steps);
        if claimed {
            st.requested -= 1;
            st.counters.splits += 1;
        }
        st.counters.tiny_runs += u64::from(ctx.polls() - ctx.run_start <= 1);
        if st.exit {
            ctx.pushed.clear();
            return self.leave(st, i);
        }
        // A worker beyond the target hands all its units over, its deepest too, and retires; one with none comes back
        // for work, and `acquire` retires it after the termination test (see "Workers").
        let retire = !st.pause && st.live > st.target && !ctx.pushed.is_empty();
        let keep = if retire { None } else { ctx.pushed.pop() };
        let gave = ctx.pushed.len();
        if gave > 0 {
            if claimed {
                st.counters.split_units += gave as u64;
            }
            self.push_units(st, &mut ctx.pushed);
        }
        if retire {
            self.leave(st, i);
            let idle = st.idle;
            drop(guard);
            self.wake_idle(idle, gave);
            return None;
        }
        if st.pause {
            st.held_units += usize::from(keep.is_some());
            st.held[i] = keep;
            if st.running == 0 {
                self.coord_cv.notify_all();
            }
            return self.acquire(guard, ctx);
        }
        match keep {
            Some(unit) => {
                self.begin_run(st, ctx);
                let idle = st.idle;
                drop(guard);
                self.wake_idle(idle, gave);
                Some(unit)
            }
            // Nothing was pushed, so there is nobody to wake.
            None => self.acquire(guard, ctx),
        }
    }

    // --- the control ---------------------------------------------------------------------------------------

    /// Start workers until as many are live as the target, unless the run is ending or stopping (see "Workers").  Their
    /// slots are reserved under the lock, so that they count as live at once, and their threads spawned outside it.
    /// The first workers (`initial`) count as alive from the start of the workers, later ones from now.  A worker that
    /// cannot be started stops the run with the error.
    fn grow(&self, spawn: &dyn Fn(usize) -> io::Result<()>, initial: bool) {
        let (first, end, peak) = {
            let mut guard = self.lock();
            let st = &mut *guard;
            if st.exit || st.stop_requested || st.live >= st.target {
                return;
            }
            let born = if initial { st.began } else { Instant::now() };
            let (first, end) = (st.clocks.len(), st.clocks.len() + st.target - st.live);
            st.clocks.resize(end, Clock::new(born));
            st.held.resize_with(end, || None);
            st.worker_stats.resize(end, Stats::default());
            st.live += end - first;
            let peak = st.counters.threads;
            st.counters.threads = peak.max(st.live);
            if !initial {
                st.counters.spawned += (end - first) as u64;
            }
            (first, end, peak)
        };
        for i in first..end {
            if let Err(e) = spawn(i) {
                let mut guard = self.lock();
                let st = &mut *guard;
                // The workers from `i` on never existed: they were alive for no time.
                for clock in &mut st.clocks[i..end] {
                    clock.gone = Some(clock.born);
                }
                st.live -= end - i;
                st.counters.threads = peak.max(st.live);
                if !initial {
                    st.counters.spawned -= (end - i) as u64;
                }
                self.note_running(st, Instant::now());
                self.fail(st, e);
                return;
            }
        }
    }

    // --- coordinator ---------------------------------------------------------------------------------------

    /// Runs on the calling thread: checkpoints, limits, stop signals, progress lines, and the workers a control adds
    /// (`spawn` starts worker `i`).  Returns once `exit` is set.  Times too far ahead to represent (absurd intervals or
    /// limits) are never reached.
    fn coordinate(&self, plan: &Plan<'_>, spawn: &dyn Fn(usize) -> io::Result<()>) {
        let _guard = PanicGuard(self);
        let mut next_checkpoint = plan.checkpoint.and_then(|(_, every)| plan.began.checked_add(every));
        let mut next_progress = plan.progress.and_then(|every| plan.began.checked_add(every));
        let mut last_report = (plan.began, 0);
        let stopping = |st: &State<U>| st.stop_requested || plan.stop_due();
        let mut guard = self.lock();
        // With no worker live (a run suspended from its start), `resuming` ends at once ("Suspending").
        self.note_running(&mut guard, Instant::now());
        loop {
            if guard.exit {
                break;
            }
            let now = Instant::now();
            let stop = stopping(&guard);
            // A checkpoint the control asked for is due at once; without a checkpoint file there is none to write.
            let asked = guard.checkpoint_asked && plan.checkpoint.is_some();
            if stop || ((past(next_checkpoint) || asked) && !guard.resuming) {
                guard.checkpoint_asked = false;
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
                    let (nodes, elapsed) = (self.total_stats(st).nodes, self.prior + self.t0.elapsed());
                    let tell = |event| {
                        if let Some(hook) = plan.hook {
                            (hook.0)(event);
                        }
                    };
                    tell(CheckpointEvent::Writing { nodes, elapsed });
                    match self.write_checkpoint(st, path, plan.header) {
                        Ok(()) => {
                            st.counters.checkpoints += 1;
                            tell(CheckpointEvent::Written { path, nodes, elapsed });
                        }
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
            // With no worker live, the coordinator makes the termination test, which only a run suspended from its
            // start with no unit passes (see "Workers").
            if guard.live == 0 && !guard.has_work() {
                guard.done = true;
                self.set_exit(&mut guard);
                continue;
            }
            // The control raised the target ("Workers"); `grow` checks again under its own lock.
            if guard.live < guard.target && !guard.stop_requested {
                drop(guard);
                self.grow(spawn, false);
                guard = self.lock();
                continue;
            }
            if let (Some(due), Some(every)) = (next_progress, plan.progress) {
                if now >= due {
                    let nodes = self.nodes.0.load(Ordering::Relaxed);
                    let rate = (nodes - last_report.1) as f64 / now.duration_since(last_report.0).as_secs_f64();
                    let line = self.progress_line(&guard, plan, &format!("{}/s", si(rate)));
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
            let line = self.progress_line(&guard, plan, status);
            drop(guard);
            report(&line);
        }
    }

    fn progress_line(&self, st: &State<U>, plan: &Plan<'_>, note: &str) -> String {
        let nodes = self.base.nodes + self.nodes.0.load(Ordering::Relaxed);
        let label = plan.label;
        let line = format!(
            "{label}{}{} nodes {nodes} ({note}) best {} queue {} spilled {} | {}",
            if label.is_empty() { "" } else { " " },
            fmt_elapsed(self.prior + self.t0.elapsed()),
            self.best.0.load(Ordering::Relaxed),
            st.mem.len(),
            st.spill.units,
            Self::counters_at(st, Instant::now()).summary(),
        );
        match plan.note {
            Some(extra) => format!("{line} | {}", (extra.0)()),
            None => line,
        }
    }

    /// The counters with the workers' times summed up to `now` (also for lifetimes, runs and waits still going on).
    fn counters_at(st: &State<U>, now: Instant) -> Counters {
        let mut c = st.counters;
        c.wall = now.saturating_duration_since(st.began);
        c.idle = st.idle_done;
        for clock in &st.clocks {
            c.capacity += clock.gone.unwrap_or(now).saturating_duration_since(clock.born);
            c.busy += clock.busy + clock.running_since.map_or(Duration::ZERO, |t| now.saturating_duration_since(t));
            c.parked += clock.parked;
            c.lock_wait += clock.lock_wait;
            if let Some((t, paused)) = clock.waiting_since {
                *(if paused { &mut c.parked } else { &mut c.idle }) += now.saturating_duration_since(t);
            }
        }
        if c.full_occupancy.is_none() {
            c.ramp_idle = c.idle;
        }
        if let Some((from, idle)) = st.tail_from {
            c.tail = now.saturating_duration_since(from);
            c.tail_idle = c.idle.saturating_sub(idle);
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
        let (best, payload) = self.incumbent();
        Ok(Outcome {
            completed,
            optimal_declared: st.optimal,
            best,
            payload,
            stats: self.total_stats(&st),
            elapsed,
            counters: Self::counters_at(&st, ended),
        })
    }
}

/// What a [`Control`] asks of the run it is attached to: implemented by the run's shared state, whose unit type the
/// control does not know (see "The control").
trait Remote: Send + Sync {
    /// Offer an incumbent, as a searcher does ([`Ctx::offer`]).
    fn offer(&self, value: u64, payload: &[u8]) -> bool;
    /// Lower the ceiling to `value`, and end the run if the incumbent reaches it.
    fn set_ceiling(&self, value: u64);
    /// The incumbent's value and payload, read together.
    fn incumbent(&self) -> (u64, Vec<u8>);
    /// Stop the run with a final checkpoint.
    fn stop(&self);
    /// Write a checkpoint as soon as the workers run, as if the interval had passed.
    fn checkpoint(&self);
    /// Set the target number of workers (0 suspends the search).
    fn set_workers(&self, workers: usize);
    fn status(&self) -> Status;
}

/// Whether an incumbent worth `best` reaches `ceiling` ([`NO_CEILING`] is none).
fn reaches(best: u64, ceiling: u64) -> bool {
    ceiling != NO_CEILING && best >= ceiling
}

impl<U: Unit> Remote for Shared<U> {
    fn offer(&self, value: u64, payload: &[u8]) -> bool {
        let best = &self.best.0;
        if value <= best.load(Ordering::Relaxed) {
            return false;
        }
        {
            let mut stored = lock(&self.payload);
            if value <= best.load(Ordering::Relaxed) {
                return false;
            }
            stored.clear();
            stored.extend_from_slice(payload);
            // Stored before the ceiling is loaded, both `SeqCst`: see "Ceiling".
            best.store(value, Ordering::SeqCst);
        }
        // With the payload's lock released, since `declare_optimal` takes the state lock.
        if reaches(value, self.ceiling.load(Ordering::SeqCst)) {
            self.declare_optimal();
        }
        true
    }

    fn set_ceiling(&self, value: u64) {
        // Stored before the incumbent is loaded, both `SeqCst`: see "Ceiling".
        let ceiling = self.ceiling.fetch_min(value, Ordering::SeqCst).min(value);
        if reaches(self.best.0.load(Ordering::SeqCst), ceiling) {
            self.declare_optimal();
        }
    }

    fn incumbent(&self) -> (u64, Vec<u8>) {
        // `best` changes only while the payload is locked, so the two agree.
        let payload = lock(&self.payload);
        (self.best.0.load(Ordering::Relaxed), payload.clone())
    }

    fn stop(&self) {
        self.request_stop();
    }

    fn checkpoint(&self) {
        let mut st = self.lock();
        if !st.exit {
            st.checkpoint_asked = true;
            self.coord_cv.notify_all();
        }
    }

    fn set_workers(&self, workers: usize) {
        let mut st = self.lock();
        st.target = workers;
        // Growing is the coordinator's, retiring the workers' own: wake whoever has to act.
        if st.live < st.target {
            self.coord_cv.notify_all();
        }
        if st.live > st.target {
            self.work_cv.notify_all();
        }
        self.update_split(&mut st);
    }

    fn status(&self) -> Status {
        let st = self.lock();
        let nodes = self.nodes.0.load(Ordering::Relaxed);
        let c = Self::counters_at(&st, Instant::now());
        Status {
            live: st.live,
            running: st.running,
            idle: st.clocks.iter().filter(|clock| matches!(clock.waiting_since, Some((_, false)))).count(),
            queued: st.queued(),
            idle_time: c.idle,
            capacity: c.capacity,
            best: self.best.0.load(Ordering::Relaxed),
            nodes,
            all_nodes: self.base.nodes + nodes,
        }
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
// The idle scheduling policy: the one Linux function `std` does not wrap
// ------------------------------------------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod idle {
    use std::ffi::c_int;
    use std::io;

    /// `SCHED_IDLE` of `<sched.h>`.
    const SCHED_IDLE: c_int = 5;

    /// `struct sched_param`, whose one field the idle policy needs at 0.
    #[repr(C)]
    struct SchedParam {
        sched_priority: c_int,
    }

    extern "C" {
        fn sched_setscheduler(pid: c_int, policy: c_int, param: *const SchedParam) -> c_int;
    }

    /// Puts the calling thread under `SCHED_IDLE` (pid 0 is the calling thread on Linux, not the process).
    pub fn set() -> io::Result<()> {
        let param = SchedParam { sched_priority: 0 };
        // SAFETY: `param` is a valid `struct sched_param` for the call's duration, and the call has no other
        // preconditions.
        if unsafe { sched_setscheduler(0, SCHED_IDLE, &param) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod idle {
    use std::io;

    pub fn set() -> io::Result<()> {
        Err(io::ErrorKind::Unsupported.into())
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

/// Read and verify a whole checkpoint, passing each unit's bytes (decoded by [`Unit::decode`]) to `unit` in the order
/// the checkpoint holds them, the shared deque from its front (the shallowest, largest subtrees) to its back and then
/// the units the workers held, and return everything but the units ([`read_checkpoint`] is this with no use for them).
/// For a caller that studies a checkpoint's work (`examples/tune.rs`); the engine itself resumes from it this way.
///
/// # Errors
///
/// As [`read_checkpoint`], and the first error `unit` returns.
pub fn scan_checkpoint(path: &Path, unit: impl FnMut(&[u8]) -> io::Result<()>) -> io::Result<CheckpointHead> {
    scan_checkpoint_file(File::open(path)?, unit)
}

/// [`scan_checkpoint`] of a checkpoint already open: for a caller that opens the file as soon as it is written
/// ([`CheckpointEvent::Written`]) and reads it later, after the next checkpoint may have replaced it.
///
/// # Errors
///
/// As [`scan_checkpoint`].
pub fn scan_checkpoint_file(file: File, mut unit: impl FnMut(&[u8]) -> io::Result<()>) -> io::Result<CheckpointHead> {
    let mut r = HashReader { inner: BufReader::with_capacity(1 << 20, file), hash: Hash64::new() };
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
