//! Engine tests on a toy problem with an independently computable optimum: 0/1 knapsack solved by
//! depth-first branch-and-bound with a simple fractional bound, checked against dynamic programming.
//!
//! Benchmark (engine overhead per node): `cargo test --release --test engine -- --ignored --nocapture bench`
//! (sizes and thread counts: see [`bench`]).  Stress test: see [`stress`].

mod common;

use common::{Rng, TempDir};
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use taxman::engine::{self, Control, Ctx, EngineConfig, Outcome, Poll, Searcher, Start, Status, Unit};

// ------------------------------------------------------------------------------------------------------------
// The toy problem
// ------------------------------------------------------------------------------------------------------------

/// A knapsack instance with items sorted by value density, best first.
struct Instance {
    w: Vec<u64>,
    v: Vec<u64>,
    /// `rest[j]` = total value of items `j..`.
    rest: Vec<u64>,
    cap: u64,
}

impl Instance {
    /// A strongly correlated instance (value = weight + range/10), which is hard for branch-and-bound.
    fn new(n: usize, range: u64, seed: u64) -> Instance {
        let mut rng = Rng(seed | 1);
        let mut items: Vec<(u64, u64)> = (0..n)
            .map(|_| {
                let w = 1 + rng.next() % range;
                (w, w + range / 10)
            })
            .collect();
        // Density v/w descending, compared exactly by cross-multiplication.
        items.sort_by(|a, b| (b.1 * a.0).cmp(&(a.1 * b.0)));
        let cap = items.iter().map(|it| it.0).sum::<u64>() / 2;
        let v: Vec<u64> = items.iter().map(|it| it.1).collect();
        let mut rest = vec![0; n + 1];
        for j in (0..n).rev() {
            rest[j] = rest[j + 1] + v[j];
        }
        Instance { w: items.iter().map(|it| it.0).collect(), v, rest, cap }
    }

    fn n(&self) -> usize {
        self.w.len()
    }

    /// Optimum by dynamic programming over capacities.
    fn dp(&self) -> u64 {
        let mut best = vec![0u64; self.cap as usize + 1];
        for (&w, &v) in self.w.iter().zip(&self.v) {
            for c in (w as usize..best.len()).rev() {
                best[c] = best[c].max(best[c - w as usize] + v);
            }
        }
        best[self.cap as usize]
    }

    /// Value of the item set `mask`, or None if it does not fit.
    fn check(&self, mask: u64) -> Option<u64> {
        let items = (0..self.n()).filter(|i| mask >> i & 1 == 1);
        let (w, v) = items.fold((0, 0), |(w, v), i| (w + self.w[i], v + self.v[i]));
        (w <= self.cap).then_some(v)
    }

    /// Upper bound on the best completion of `node` (whose `next` is an item): all remaining room filled at
    /// the best remaining density, but no more than all remaining items.  Deliberately weaker than the
    /// Dantzig bound, so that the tree is large and its size varies smoothly with `n`.
    fn bound(&self, node: &Node) -> u64 {
        let j = node.next as usize;
        node.value + self.rest[j].min((self.cap - node.weight) * self.v[j] / self.w[j])
    }
}

/// A search node: items before `next` are decided; `mask` holds the chosen ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Node {
    next: u32,
    mask: u64,
    weight: u64,
    value: u64,
}

impl Node {
    const ROOT: Node = Node { next: 0, mask: 0, weight: 0, value: 0 };
}

impl Unit for Node {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.next.to_le_bytes());
        out.extend_from_slice(&self.mask.to_le_bytes());
        out.extend_from_slice(&self.weight.to_le_bytes());
        out.extend_from_slice(&self.value.to_le_bytes());
    }

    fn decode(b: &[u8]) -> Self {
        let u64_at = |i: usize| u64::from_le_bytes(b[i..i + 8].try_into().unwrap());
        Node {
            next: u32::from_le_bytes(b[0..4].try_into().unwrap()),
            mask: u64_at(4),
            weight: u64_at(12),
            value: u64_at(20),
        }
    }
}

/// What the depth-first search needs from its surroundings, so the same search runs inside and outside the
/// engine (the difference measures the engine's per-node overhead).
trait Env {
    fn poll(&mut self) -> Poll;
    fn best(&self) -> u64;
    fn improve(&mut self, value: u64, mask: u64);
    fn count(&mut self, extra: Option<usize>);
    fn push(&mut self, node: Node);
}

fn dfs(inst: &Instance, stack: &mut Vec<Node>, root: Node, env: &mut impl Env) {
    stack.clear();
    stack.push(root);
    while let Some(node) = stack.pop() {
        if env.poll() == Poll::Yield {
            stack.push(node);
            for unit in stack.drain(..) {
                env.push(unit);
            }
            return;
        }
        env.count(None);
        if node.value > env.best() {
            env.improve(node.value, node.mask);
        }
        let i = node.next as usize;
        if i == inst.n() {
            env.count(Some(1));
            continue;
        }
        if inst.bound(&node) <= env.best() {
            env.count(Some(0));
            continue;
        }
        stack.push(Node { next: node.next + 1, ..node });
        if node.weight + inst.w[i] <= inst.cap {
            stack.push(Node {
                next: node.next + 1,
                mask: node.mask | 1 << i,
                weight: node.weight + inst.w[i],
                value: node.value + inst.v[i],
            });
        }
    }
}

struct EngineEnv<'c, 'a> {
    ctx: &'c mut Ctx<'a, Node>,
    target: Option<u64>,
}

impl Env for EngineEnv<'_, '_> {
    #[inline]
    fn poll(&mut self) -> Poll {
        self.ctx.poll()
    }

    #[inline]
    fn best(&self) -> u64 {
        self.ctx.best()
    }

    fn improve(&mut self, value: u64, mask: u64) {
        if self.ctx.offer(value, &mask.to_le_bytes()) && Some(value) == self.target {
            self.ctx.declare_optimal();
        }
    }

    #[inline]
    fn count(&mut self, extra: Option<usize>) {
        let stats = self.ctx.stats();
        match extra {
            None => stats.nodes += 1,
            Some(k) => stats.extra[k] += 1,
        }
    }

    fn push(&mut self, node: Node) {
        self.ctx.push(node);
    }
}

struct Knapsack<'a> {
    inst: &'a Instance,
    stack: Vec<Node>,
    /// Declare optimality on reaching this value (a known upper bound).
    target: Option<u64>,
    /// A worker that starts a unit once this file exists panics (to test panic propagation after a checkpoint).
    panic_when: Option<PathBuf>,
}

impl Searcher<Node> for Knapsack<'_> {
    fn run(&mut self, unit: Node, ctx: &mut Ctx<'_, Node>) {
        if self.panic_when.as_ref().is_some_and(|path| path.exists()) {
            panic!("searcher panic requested by test");
        }
        dfs(self.inst, &mut self.stack, unit, &mut EngineEnv { ctx, target: self.target });
    }
}

/// The search without the engine.
#[derive(Default)]
struct RawEnv {
    best: u64,
    nodes: u64,
    extra: [u64; 2],
}

impl Env for RawEnv {
    #[inline]
    fn poll(&mut self) -> Poll {
        Poll::Continue
    }

    #[inline]
    fn best(&self) -> u64 {
        self.best
    }

    fn improve(&mut self, value: u64, _mask: u64) {
        self.best = value;
    }

    #[inline]
    fn count(&mut self, extra: Option<usize>) {
        match extra {
            None => self.nodes += 1,
            Some(k) => self.extra[k] += 1,
        }
    }

    fn push(&mut self, _node: Node) {
        unreachable!("the raw search never yields")
    }
}

fn raw_search(inst: &Instance) -> RawEnv {
    let mut env = RawEnv::default();
    dfs(inst, &mut Vec::new(), Node::ROOT, &mut env);
    env
}

// ------------------------------------------------------------------------------------------------------------
// Fixtures
// ------------------------------------------------------------------------------------------------------------

/// The shared test instance: about 13 million nodes, optimum known by DP.
fn instance() -> Instance {
    Instance::new(48, 10_000, 99)
}

/// Items of [`big_instance`], also the benchmark's default.
const BIG_ITEMS: usize = 56;

/// A bigger instance (about 200 million nodes, under a second on one thread), for tests that need a search to
/// last a while on many threads.
fn big_instance() -> Instance {
    Instance::new(BIG_ITEMS, 10_000, 99)
}

/// A longer one (about 0.65 G nodes), far more than a run explores before a script on another thread has acted on it.
fn long_instance() -> Instance {
    Instance::new(64, 10_000, 99)
}

const HEADER: &[u8] = b"knapsack n=48 range=10000 seed=99";

fn config(threads: usize) -> EngineConfig {
    EngineConfig { threads, ..EngineConfig::default() }
}

fn fresh() -> Start<Node> {
    Start::Fresh { units: vec![Node::ROOT], best: 0, payload: Vec::new() }
}

fn solve(cfg: &EngineConfig, start: Start<Node>, inst: &Instance) -> Outcome {
    solve_with(cfg, start, inst, None, None)
}

fn solve_with(
    cfg: &EngineConfig,
    start: Start<Node>,
    inst: &Instance,
    target: Option<u64>,
    panic_when: Option<PathBuf>,
) -> Outcome {
    let make = |_| Knapsack { inst, stack: Vec::new(), target, panic_when: panic_when.clone() };
    engine::run(cfg, start, HEADER, make).expect("engine run")
}

/// Check the incumbent against the DP optimum and its payload against the instance.
fn assert_optimal(out: &Outcome, inst: &Instance, opt: u64) {
    assert_eq!(out.best, opt, "optimum");
    let mask = u64::from_le_bytes(out.payload.as_slice().try_into().expect("payload is a mask"));
    assert_eq!(inst.check(mask), Some(opt), "payload is a feasible solution of value best");
}

fn dir_is_empty(dir: &Path) -> bool {
    fs::read_dir(dir).map(|mut entries| entries.next().is_none()).unwrap_or(true)
}

// ------------------------------------------------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------------------------------------------------

#[test]
fn instance_is_big_enough() {
    let inst = instance();
    let raw = raw_search(&inst);
    assert_eq!(raw.best, inst.dp());
    assert!(raw.nodes >= 1_000_000, "tree too small: {} nodes", raw.nodes);
}

#[test]
fn one_thread_is_correct_and_deterministic() {
    let inst = instance();
    let opt = inst.dp();
    let raw = raw_search(&inst);
    let a = solve(&config(1), fresh(), &inst);
    let b = solve(&config(1), fresh(), &inst);
    assert!(a.completed && b.completed);
    assert_optimal(&a, &inst, opt);
    assert_eq!(a.stats, b.stats, "identical statistics across runs");
    // One engine thread explores exactly the tree the plain depth-first search explores.
    assert_eq!(a.stats.nodes, raw.nodes);
    assert_eq!(a.stats.extra[..2], raw.extra);
    let c = &a.counters;
    assert_eq!((c.threads, c.splits, c.steals, c.runs), (1, 0, 0, 1), "{c:?}");
    assert_eq!((c.idle, c.parked, c.pauses), (Duration::ZERO, Duration::ZERO, 0), "a lone worker never waits");
    assert!(c.full_occupancy.is_some());
    assert_accounting(c);
}

#[test]
fn eight_threads_are_correct() {
    let inst = instance();
    let out = solve(&config(8), fresh(), &inst);
    assert!(out.completed && !out.optimal_declared);
    assert_optimal(&out, &inst, inst.dp());
    assert!(out.counters.splits > 0, "work was split");
    assert_accounting(&out.counters);
}

/// Many workers taking turns on a few CPUs (`EngineConfig::cores`, the emulation of a larger machine) give the
/// same answer, split the work among all of them, and stop at a node limit.
#[test]
fn many_workers_on_few_cores_are_correct() {
    let inst = instance();
    let cfg = EngineConfig { cores: Some(2), ..config(16) };
    let out = solve(&cfg, fresh(), &inst);
    assert!(out.completed && !out.optimal_declared);
    assert_optimal(&out, &inst, inst.dp());
    assert!(out.counters.splits > 0, "work was split");
    let limited = EngineConfig { node_limit: Some(2_000), ..cfg };
    let out = solve(&limited, fresh(), &inst);
    assert!(!out.completed, "the node limit stops it");
}

/// The instrumentation is consistent: each worker's busy, idle, parked and lock-wait intervals are disjoint and
/// inside its lifetime, so together they take at most the capacity, which is `wall × threads` when no worker was added
/// or retired (and at most that otherwise); a run's rebuild and first step are parts of its busy time, and the ramp's
/// and the tail's idle times parts of the idle time; every pause that parked the workers ended later.
fn assert_accounting(c: &engine::Counters) {
    let most = c.wall * u32::try_from(c.threads).unwrap();
    if c.spawned == 0 && c.retired == 0 {
        assert_eq!(c.capacity, most, "{c:?}");
    }
    assert!(c.capacity <= most, "{c:?}");
    assert!(c.busy + c.idle + c.parked + c.lock_wait <= c.capacity, "{c:?}");
    assert!((0.0..=1.0).contains(&c.efficiency()), "{c:?}");
    assert!(c.full_occupancy.is_none_or(|t| t <= c.wall), "{c:?}");
    assert!(c.rebuild + c.first_step <= c.busy && c.first_steps <= c.runs, "{c:?}");
    assert!(c.ramp_idle <= c.idle && c.tail_idle <= c.idle && c.tail <= c.wall, "{c:?}");
    if c.full_occupancy.is_none() {
        assert!(c.ramp_idle == c.idle && c.tail == Duration::ZERO && c.tail_idle == Duration::ZERO, "{c:?}");
    }
    assert!(c.pause_max <= c.pause_total && c.park_max <= c.pause_max, "{c:?}");
    assert!(c.runs >= c.steals, "{c:?}");
}

// Many more workers than CPUs, as on a machine with 192 CPUs: answers, splitting, checkpoints, spill and panics
// must work however the threads are scheduled.  They keep every CPU busy for seconds, so they run only when
// asked for: `cargo test --release --test engine -- --ignored many_threads`.

const MANY: [usize; 2] = [64, 192];

#[test]
#[ignore = "uses every core for seconds; run with --ignored"]
fn many_threads_are_correct() {
    let inst = instance();
    let opt = inst.dp();
    for threads in MANY {
        let out = solve(&config(threads), fresh(), &inst);
        assert!(out.completed && !out.optimal_declared, "{threads} threads");
        assert_optimal(&out, &inst, opt);
        let c = &out.counters;
        assert_eq!(c.threads, threads);
        assert!(c.splits > 0 && c.steals > 0 && c.runs > c.splits, "{threads} threads: {c:?}");
        assert!(c.busy > Duration::ZERO, "{threads} threads: {c:?}");
        assert_accounting(c);
        println!("{threads} threads: {}", c.summary());
        let out = solve_with(&config(threads), fresh(), &inst, Some(opt), None);
        assert!(out.completed && out.optimal_declared, "{threads} threads: declared optimal");
        assert_optimal(&out, &inst, opt);
    }
}

#[test]
#[ignore = "uses every core for seconds; run with --ignored"]
fn many_threads_checkpoint_resume_and_spill() {
    let inst = instance();
    let raw = raw_search(&inst);
    for threads in MANY {
        let dir = TempDir::new(&format!("many{threads}"));
        let ckpt = dir.join("ckpt.bin");
        let spill = dir.join("spill");
        // In pieces through checkpoints (each stop is a pause that parks every worker), with a tiny in-memory
        // queue: the stacks the workers push in the pause spill.
        let cfg = EngineConfig {
            checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
            node_limit: Some(raw.nodes / 4),
            queue_mem_cap: 16,
            spill_dir: Some(spill.clone()),
            ..config(threads)
        };
        let (out, runs) = run_in_pieces(&cfg, &inst, &ckpt);
        assert!(runs.len() >= 2, "{threads} threads: {} invocations", runs.len());
        assert_optimal(&out, &inst, inst.dp());
        assert!(runs.iter().any(|c| c.spilled_units > 0 && c.reloaded_segments > 0), "{threads} threads: {runs:?}");
        assert!(dir_is_empty(&spill), "{threads} threads: spill files left behind");
        // Periodic checkpoints every 2 ms, progress lines and spilling all at once.  How many pauses happen
        // depends on when the coordinator gets a CPU, so only the answer and the bookkeeping are checked.
        let big = big_instance();
        let cfg = EngineConfig {
            checkpoint: Some((ckpt.clone(), Duration::from_millis(2))),
            progress: Some(Duration::from_millis(50)),
            queue_mem_cap: 8,
            spill_dir: Some(spill.clone()),
            label: format!("many {threads}"),
            ..config(threads)
        };
        let out = solve(&cfg, fresh(), &big);
        assert!(out.completed);
        assert_optimal(&out, &big, big.dp());
        assert_accounting(&out.counters);
        println!("{threads} threads, checkpoints every 2 ms: {}", out.counters.summary());
        assert!(!ckpt.exists() && dir_is_empty(&spill), "{threads} threads: files left behind");
    }
}

#[test]
#[ignore = "uses every core for seconds; run with --ignored"]
fn many_threads_worker_panic_propagates() {
    let inst = big_instance();
    let raw_nodes = 200_000_000; // about the big instance's tree
    for threads in MANY {
        let dir = TempDir::new(&format!("panic{threads}"));
        let ckpt = dir.join("ckpt.bin");
        let spill = dir.join("spill");
        let cfg = EngineConfig {
            checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
            node_limit: Some(raw_nodes / 10),
            queue_mem_cap: 4,
            spill_dir: Some(spill.clone()),
            ..config(threads)
        };
        let first = solve(&cfg, fresh(), &inst);
        assert!(!first.completed && ckpt.exists(), "{threads} threads: stopped with a checkpoint");
        // Resumed, every worker panics as it starts a unit, while the others start theirs or run.
        let cfg = EngineConfig { node_limit: None, ..cfg };
        let resumed = || solve_with(&cfg, Start::Resume(ckpt.clone()), &inst, None, Some(ckpt.clone()));
        let result = panic::catch_unwind(AssertUnwindSafe(resumed));
        assert!(result.is_err(), "{threads} threads: the panic reached the caller");
        assert!(dir_is_empty(&spill), "{threads} threads: spill files removed while unwinding");
        assert!(ckpt.exists(), "{threads} threads: the checkpoint survives the panic");
        let out = solve(&cfg, Start::Resume(ckpt.clone()), &inst);
        assert!(out.completed);
        assert_optimal(&out, &inst, inst.dp());
    }
}

#[test]
fn tiny_queue_cap_spills_and_cleans_up() {
    let inst = instance();
    let dir = TempDir::new("spill");
    let spill = dir.join("spill");
    let cfg = EngineConfig { queue_mem_cap: 2, spill_dir: Some(spill.clone()), ..config(4) };
    let out = solve(&cfg, fresh(), &inst);
    assert!(out.completed);
    assert_optimal(&out, &inst, inst.dp());
    assert!(out.counters.spilled_units > 0, "spilled: {:?}", out.counters);
    assert!(out.counters.reloaded_segments > 0, "reloaded: {:?}", out.counters);
    assert!(dir_is_empty(&spill), "spill files left behind");
}

/// Run with a node limit, resuming from the checkpoint until complete.  Returns the final outcome and the counters
/// of every invocation.
fn run_in_pieces(cfg: &EngineConfig, inst: &Instance, ckpt: &Path) -> (Outcome, Vec<engine::Counters>) {
    let mut start = fresh();
    let mut counters = Vec::new();
    for invocation in 1.. {
        let out = solve(cfg, start, inst);
        assert_accounting(&out.counters);
        counters.push(out.counters);
        // The elapsed time is this invocation's plus that of the runs before it (from the checkpoint).  (Checked
        // with the engine's own clocks: a test thread can be descheduled for long when the tests oversubscribe
        // the CPUs.)
        if invocation == 1 {
            assert!(out.elapsed >= out.counters.wall, "elapsed includes this run: {:?}", out.counters);
        } else {
            assert!(out.elapsed > out.counters.wall, "elapsed accumulates across resumes: {:?}", out.counters);
        }
        if out.completed {
            assert!(!ckpt.exists(), "checkpoint removed on completion");
            return (out, counters);
        }
        assert!(ckpt.exists(), "final checkpoint written on stop");
        assert!(out.counters.pauses >= 1 && out.counters.checkpoints >= 1, "{:?}", out.counters);
        assert_eq!(engine::read_checkpoint_header(ckpt).unwrap(), HEADER);
        assert!(invocation < 1000, "no progress");
        start = Start::Resume(ckpt.to_path_buf());
    }
    unreachable!()
}

#[test]
fn checkpoint_resume_one_thread_is_exact() {
    let inst = instance();
    let raw = raw_search(&inst);
    let dir = TempDir::new("resume1");
    let ckpt = dir.join("ckpt.bin");
    let spill = dir.join("spill");
    let cfg = EngineConfig {
        checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
        node_limit: Some(raw.nodes / 7),
        queue_mem_cap: 3,
        spill_dir: Some(spill.clone()),
        ..config(1)
    };
    let (out, runs) = run_in_pieces(&cfg, &inst, &ckpt);
    assert!(runs.len() >= 7, "{} invocations", runs.len());
    assert_optimal(&out, &inst, inst.dp());
    // A single thread resumes in exact depth-first order, so the total work equals one uninterrupted run.
    assert_eq!(out.stats.nodes, raw.nodes);
    assert_eq!(out.stats.extra[..2], raw.extra);
    assert!(dir_is_empty(&spill));
}

#[test]
fn checkpoint_resume_many_threads() {
    let inst = instance();
    let raw = raw_search(&inst);
    let dir = TempDir::new("resume4");
    let ckpt = dir.join("ckpt.bin");
    let spill = dir.join("spill");
    let cfg = EngineConfig {
        checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
        node_limit: Some(raw.nodes / 5),
        queue_mem_cap: 4,
        spill_dir: Some(spill.clone()),
        ..config(4)
    };
    let (out, runs) = run_in_pieces(&cfg, &inst, &ckpt);
    assert!(runs.len() >= 3, "{} invocations", runs.len());
    assert_optimal(&out, &inst, inst.dp());
    let ratio = out.stats.nodes as f64 / raw.nodes as f64;
    assert!((0.5..2.0).contains(&ratio), "nodes {} vs uninterrupted {}", out.stats.nodes, raw.nodes);
    assert!(dir_is_empty(&spill));
}

#[test]
fn time_limit_stops_and_resumes() {
    let inst = instance();
    let dir = TempDir::new("time");
    let ckpt = dir.join("ckpt.bin");
    let cfg = EngineConfig {
        checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
        time_limit: Some(Duration::from_millis(5)),
        ..config(4)
    };
    let first = solve(&cfg, fresh(), &inst);
    assert!(!first.completed, "5 ms is not enough");
    assert!(ckpt.exists());
    let cfg = EngineConfig { time_limit: None, ..cfg };
    let out = solve(&cfg, Start::Resume(ckpt.clone()), &inst);
    assert!(out.completed);
    assert_optimal(&out, &inst, inst.dp());
    assert!(out.stats.nodes > first.stats.nodes);
    assert!(!ckpt.exists());
}

#[test]
fn declare_optimal_stops_promptly() {
    let inst = instance();
    let opt = inst.dp();
    let raw = raw_search(&inst);
    for threads in [1, 8] {
        let out = solve_with(&config(threads), fresh(), &inst, Some(opt), None);
        assert!(out.completed && out.optimal_declared);
        assert_optimal(&out, &inst, opt);
        assert!(out.stats.nodes < raw.nodes / 2, "{threads} threads: {} of {} nodes", out.stats.nodes, raw.nodes);
    }
}

#[test]
fn frequent_checkpoints_do_not_deadlock() {
    let inst = instance();
    let dir = TempDir::new("frequent");
    let ckpt = dir.join("ckpt.bin");
    let cfg = EngineConfig { checkpoint: Some((ckpt.clone(), Duration::from_millis(1))), ..config(4) };
    let out = solve(&cfg, fresh(), &inst);
    assert!(out.completed);
    assert_optimal(&out, &inst, inst.dp());
    assert!(out.counters.checkpoints > 0, "{:?}", out.counters);
    assert!(!ckpt.exists(), "checkpoint removed on completion");
}

#[test]
fn checkpoints_with_progress_and_spill_on_ten_threads() {
    let inst = instance();
    let dir = TempDir::new("busy");
    let ckpt = dir.join("ckpt.bin");
    let spill = dir.join("spill");
    let cfg = EngineConfig {
        checkpoint: Some((ckpt.clone(), Duration::from_millis(3))),
        progress: Some(Duration::from_millis(20)),
        queue_mem_cap: 8,
        spill_dir: Some(spill.clone()),
        label: "test".into(),
        ..config(10)
    };
    let out = solve(&cfg, fresh(), &inst);
    assert!(out.completed);
    assert_optimal(&out, &inst, inst.dp());
    assert!(!ckpt.exists());
    assert!(dir_is_empty(&spill));
}

#[test]
fn worker_panic_propagates() {
    worker_panic_propagates_with(None);
}

/// A worker's panic gives its turn on the emulated CPUs up, so that the workers waiting for one do not wait forever.
#[test]
fn worker_panic_propagates_on_one_core() {
    worker_panic_propagates_with(Some(1));
}

fn worker_panic_propagates_with(cores: Option<usize>) {
    let inst = instance();
    let dir = TempDir::new("panic");
    let ckpt = dir.join("ckpt.bin");
    let spill = dir.join("spill");
    let cfg = EngineConfig {
        checkpoint: Some((ckpt.clone(), Duration::from_millis(1))),
        queue_mem_cap: 2,
        spill_dir: Some(spill.clone()),
        cores,
        ..config(4)
    };
    // Workers panic once the first checkpoint is on disk, so there is one to resume from.
    let result = panic::catch_unwind(AssertUnwindSafe(|| solve_with(&cfg, fresh(), &inst, None, Some(ckpt.clone()))));
    assert!(result.is_err(), "the panic reached the caller");
    assert!(dir_is_empty(&spill), "spill files removed while unwinding");
    // The last checkpoint written before the panic is intact and resumes to the right answer.
    assert!(ckpt.exists(), "a checkpoint was written before the panic");
    let out = solve(&cfg, Start::Resume(ckpt.clone()), &inst);
    assert!(out.completed);
    assert_optimal(&out, &inst, inst.dp());
}

#[test]
fn resume_rejects_wrong_header_and_damaged_files() {
    let inst = instance();
    let dir = TempDir::new("reject");
    let ckpt = dir.join("ckpt.bin");
    let cfg = EngineConfig {
        checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
        node_limit: Some(100_000),
        ..config(2)
    };
    assert!(!solve(&cfg, fresh(), &inst).completed);
    let make = |_| Knapsack { inst: &inst, stack: Vec::new(), target: None, panic_when: None };

    let err = engine::run(&cfg, Start::Resume(ckpt.clone()), b"another problem", make).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

    let good = fs::read(&ckpt).unwrap();
    for damage in ["flip", "truncate"] {
        let mut bad = good.clone();
        match damage {
            "flip" => bad[good.len() / 2] ^= 0x10,
            _ => bad.truncate(good.len() - 5),
        }
        fs::write(&ckpt, &bad).unwrap();
        let err = engine::run(&cfg, Start::Resume(ckpt.clone()), HEADER, make).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData, "{damage}: {err}");
    }

    fs::write(&ckpt, &good).unwrap();
    let cfg = EngineConfig { node_limit: None, ..cfg };
    let out = engine::run(&cfg, Start::Resume(ckpt.clone()), HEADER, make).unwrap();
    assert!(out.completed);
    assert_optimal(&out, &inst, inst.dp());
}

/// A spill failure stops the run with the error, but the units stay in memory and go into the final
/// checkpoint, so nothing is lost.
#[cfg(unix)]
#[test]
fn spill_failure_stops_without_losing_work() {
    use std::os::unix::fs::PermissionsExt;
    let inst = instance();
    let dir = TempDir::new("spillfail");
    let ckpt = dir.join("ckpt.bin");
    let spill = dir.join("spill");
    fs::create_dir_all(&spill).unwrap();
    fs::set_permissions(&spill, fs::Permissions::from_mode(0o555)).unwrap();
    // A privileged process (root, e.g. in a container) writes to a read-only directory anyway, so the spill cannot
    // be made to fail this way: skip rather than report a false failure.
    let probe = spill.join("probe");
    if fs::write(&probe, b"").is_ok() {
        let _ = fs::remove_file(&probe);
        eprintln!("spill_failure_stops_without_losing_work: skipped (this process can write to read-only directories)");
        return;
    }
    let cfg = EngineConfig {
        checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
        queue_mem_cap: 2,
        spill_dir: Some(spill.clone()),
        ..config(4)
    };
    let make = |_| Knapsack { inst: &inst, stack: Vec::new(), target: None, panic_when: None };
    let err = engine::run(&cfg, fresh(), HEADER, make).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::PermissionDenied, "{err}");
    assert!(ckpt.exists(), "final checkpoint written after the spill failure");
    fs::set_permissions(&spill, fs::Permissions::from_mode(0o755)).unwrap();
    let out = solve(&cfg, Start::Resume(ckpt.clone()), &inst);
    assert!(out.completed);
    assert_optimal(&out, &inst, inst.dp());
    assert!(dir_is_empty(&spill));
}

#[test]
fn empty_start_completes() {
    let inst = instance();
    for threads in [1, 4] {
        let start = Start::Fresh { units: Vec::new(), best: 17, payload: b"seed".to_vec() };
        let out = solve(&config(threads), start, &inst);
        assert!(out.completed);
        assert_eq!((out.best, out.payload.as_slice(), out.stats.nodes), (17, &b"seed"[..], 0));
    }
}

/// Random engine configurations on small instances, each run to completion through any number of resumes.
/// `TXENGINE_STRESS=<iterations> cargo test --release --test engine -- --ignored stress`; another
/// `TXENGINE_STRESS_SEED=<number>` draws other configurations.
#[test]
#[ignore]
fn stress() {
    let iterations: u64 = std::env::var("TXENGINE_STRESS").ok().and_then(|s| s.parse().ok()).unwrap_or(200);
    let seed: u64 = std::env::var("TXENGINE_STRESS_SEED").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15u64 ^ seed.wrapping_mul(0xd1b5_4a32_d192_ed03));
    let mut pick = |options: &[u64]| rng.pick(options);
    let dir = TempDir::new("stress");
    let ckpt = dir.join("ckpt.bin");
    let spill = dir.join("spill");
    for it in 0..iterations {
        let inst = Instance::new(pick(&[36, 40, 44]) as usize, 10_000, pick(&[7, 99, 0x5eed_1234]));
        let opt = inst.dp();
        let raw = raw_search(&inst);
        let ms = |x: u64| (x > 0).then(|| Duration::from_millis(x));
        let cfg = EngineConfig {
            threads: pick(&[1, 2, 3, 4, 7, 10, 16, 32, 64, 128, 192]) as usize,
            queue_mem_cap: pick(&[1, 2, 5, 50, 1 << 20]) as usize,
            spill_dir: Some(spill.clone()),
            checkpoint: Some((ckpt.clone(), ms(pick(&[0, 1, 3, 20])).unwrap_or(Duration::from_secs(3600)))),
            progress: ms(pick(&[0, 0, 0, 2])),
            time_limit: ms(pick(&[0, 0, 2, 5])),
            node_limit: (pick(&[0, 0, 1]) == 1).then_some(raw.nodes / 4 + 1),
            label: format!("stress {it}"),
            ..EngineConfig::default()
        };
        let target = (pick(&[0, 0, 0, 1]) == 1).then_some(opt);
        let mut start = fresh();
        let (mut nodes, mut stalled) = (0, 0);
        loop {
            let out = solve_with(&cfg, start, &inst, target, None);
            assert_accounting(&out.counters);
            stalled = if out.stats.nodes > nodes { 0 } else { stalled + 1 };
            nodes = out.stats.nodes;
            assert!(stalled < 20, "iteration {it}: no progress with {cfg:?}");
            if out.completed {
                assert_optimal(&out, &inst, opt);
                assert_eq!(out.optimal_declared, target.is_some() && out.optimal_declared);
                if cfg.threads == 1 && target.is_none() {
                    assert_eq!(out.stats.nodes, raw.nodes, "iteration {it}: one thread is exact");
                }
                break;
            }
            start = Start::Resume(ckpt.clone());
        }
        assert!(!ckpt.exists(), "iteration {it}: checkpoint left behind");
        assert!(dir_is_empty(&spill), "iteration {it}: spill files left behind");
    }
}

// ------------------------------------------------------------------------------------------------------------
// The control
// ------------------------------------------------------------------------------------------------------------

/// `cfg` with `control` attached.
fn controlled(cfg: EngineConfig, control: &Arc<Control>) -> EngineConfig {
    EngineConfig { control: Some(Arc::clone(control)), ..cfg }
}

/// The plain search of [`big_instance`], made once for all the tests that need it.
fn big_raw() -> &'static RawEnv {
    static RAW: OnceLock<RawEnv> = OnceLock::new();
    RAW.get_or_init(|| raw_search(&big_instance()))
}

/// Runs `script` on another thread while this one runs `search`; returns what each returned.
fn alongside<T: Send>(script: impl FnOnce() -> T + Send, search: impl FnOnce() -> Outcome) -> (Outcome, T) {
    thread::scope(|scope| {
        let script = scope.spawn(script);
        let out = search();
        (out, script.join().expect("the script"))
    })
}

/// Waits until the run attached to `control` goes and `ready` holds of what it is doing; `None` if the run ended
/// first, or nothing happened for a minute.
fn wait_for(control: &Control, ready: impl Fn(&Status) -> bool) -> Option<Status> {
    let (asked, mut seen) = (Instant::now(), false);
    while asked.elapsed() < Duration::from_secs(60) {
        match control.status() {
            Some(status) if ready(&status) => return Some(status),
            Some(_) => seen = true,
            None if seen => return None,
            None => {}
        }
        thread::sleep(Duration::from_micros(200));
    }
    None
}

/// A control attached but never used changes nothing: one thread explores exactly the tree of the plain search.  It
/// keeps the run's incumbent, and says no run is going before the run and after it.
#[test]
fn an_untouched_control_changes_nothing() {
    let inst = instance();
    let raw = raw_search(&inst);
    let control = Control::new();
    assert!(control.status().is_none() && control.best().is_none() && control.ceiling().is_none());
    let out = solve(&controlled(config(1), &control), fresh(), &inst);
    assert!(out.completed && !out.optimal_declared);
    assert_optimal(&out, &inst, raw.best);
    assert_eq!((out.stats.nodes, &out.stats.extra[..2]), (raw.nodes, &raw.extra[..]));
    let c = &out.counters;
    assert_eq!((c.threads, c.spawned, c.retired, c.runs), (1, 0, 0, 1), "{c:?}");
    assert_accounting(c);
    assert_eq!(control.best(), Some((out.best, out.payload.clone())));
    assert!(control.status().is_none());
}

/// The optimum given as the ceiling before the run ends it as soon as the incumbent reaches it; the lowest ceiling
/// given counts.
#[test]
fn a_ceiling_given_before_the_run_ends_it_at_the_optimum() {
    let inst = instance();
    let raw = raw_search(&inst);
    for threads in [1, 4] {
        let control = Control::new();
        for ceiling in [raw.best + 1000, raw.best, raw.best + 5] {
            control.set_ceiling(ceiling);
        }
        assert_eq!(control.ceiling(), Some(raw.best));
        let out = solve(&controlled(config(threads), &control), fresh(), &inst);
        assert!(out.completed && out.optimal_declared, "{threads} threads");
        assert_optimal(&out, &inst, raw.best);
        assert_accounting(&out.counters);
        if threads == 1 {
            // The order of the plain search, cut short where it finds the optimum.
            assert!(out.stats.nodes < raw.nodes, "{} of {} nodes", out.stats.nodes, raw.nodes);
        }
    }
}

/// A ceiling that the starting incumbent already reaches, fresh or from a checkpoint, ends the run before any search.
#[test]
fn a_ceiling_the_starting_incumbent_reaches_needs_no_search() {
    let inst = instance();
    for threads in [1, 4] {
        for ceiling in [100, 60] {
            let control = Control::new();
            control.set_ceiling(ceiling);
            let start = Start::Fresh { units: vec![Node::ROOT], best: 100, payload: b"seed".to_vec() };
            let out = solve(&controlled(config(threads), &control), start, &inst);
            let c = &out.counters;
            assert!(out.completed && out.optimal_declared, "{c:?}");
            assert_eq!((out.best, out.payload.as_slice(), out.stats.nodes), (100, &b"seed"[..], 0));
            assert_eq!((c.runs, c.threads, c.capacity), (0, 0, Duration::ZERO), "{c:?}");
            assert_accounting(c);
        }
    }
    let dir = TempDir::new("ceiling-resume");
    let ckpt = dir.join("ckpt.bin");
    let cfg = EngineConfig {
        checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
        node_limit: Some(100_000),
        ..config(2)
    };
    assert!(!solve(&cfg, fresh(), &inst).completed);
    let head = engine::read_checkpoint(&ckpt).unwrap();
    let control = Control::new();
    control.set_ceiling(head.best);
    let cfg = EngineConfig { node_limit: None, ..controlled(cfg, &control) };
    let out = solve(&cfg, Start::Resume(ckpt.clone()), &inst);
    assert!(out.completed && out.optimal_declared && out.counters.runs == 0, "{:?}", out.counters);
    assert_eq!((out.best, out.stats), (head.best, head.stats));
    assert!(!ckpt.exists(), "a completed run deletes its checkpoint");
}

/// A ceiling given while the run goes, at the incumbent the run has then, ends the run at once.
#[test]
fn a_ceiling_the_incumbent_has_reached_ends_the_run_at_once() {
    let inst = long_instance();
    let control = Control::new();
    let cfg = controlled(config(2), &control);
    let (out, ceiling) = alongside(
        || {
            wait_for(&control, |status| status.best > 0).expect("an incumbent");
            let (best, _) = control.best().expect("the run's incumbent");
            control.set_ceiling(best);
            best
        },
        || solve(&cfg, fresh(), &inst),
    );
    assert!(out.completed && out.optimal_declared && out.best >= ceiling, "{:?}", out.counters);
    assert!(out.stats.nodes < 300_000_000, "ended at once: {} nodes", out.stats.nodes);
    let mask = u64::from_le_bytes(out.payload.as_slice().try_into().unwrap());
    assert_eq!(inst.check(mask), Some(out.best), "the payload is the incumbent's");
    assert_accounting(&out.counters);
}

/// The optimum given as the ceiling while the run goes ends the run when the search finds it; one thread explores no
/// more nodes than the plain search.
#[test]
fn a_ceiling_given_while_the_run_goes_ends_it_when_reached() {
    let inst = big_instance();
    let raw = big_raw();
    let control = Control::new();
    let cfg = controlled(config(1), &control);
    let (out, ()) = alongside(
        || {
            wait_for(&control, |status| status.nodes > 0).expect("the run going");
            control.set_ceiling(raw.best);
        },
        || solve(&cfg, fresh(), &inst),
    );
    assert!(out.completed && out.optimal_declared, "{:?}", out.counters);
    assert_optimal(&out, &inst, raw.best);
    assert!(out.stats.nodes <= raw.nodes, "{} of {} nodes", out.stats.nodes, raw.nodes);
}

/// An incumbent offered from outside before the run is where the run starts from, and one offered while it goes joins
/// it: the run's incumbent is at least as good, and is the one offered if the search finds nothing better.
#[test]
fn outside_incumbents_join_the_run() {
    let inst = instance();
    let raw = raw_search(&inst);
    let solution = solve(&config(1), fresh(), &inst).payload;
    // A poor one (the first item alone) is improved on.
    let control = Control::new();
    let poor = (inst.check(1).unwrap(), 1u64.to_le_bytes().to_vec());
    assert!(control.offer(poor.0, &poor.1) && !control.offer(poor.0, b"no better"));
    assert_eq!(control.best(), Some(poor));
    let out = solve(&controlled(config(1), &control), fresh(), &inst);
    assert!(out.completed);
    assert_optimal(&out, &inst, raw.best);
    // The optimum is kept, with its payload, and one thread explores no more than from nothing.
    let control = Control::new();
    assert!(control.offer(raw.best, &solution) && !control.offer(raw.best, b"no better"));
    let out = solve(&controlled(config(1), &control), fresh(), &inst);
    assert!(out.completed && !out.optimal_declared);
    assert_eq!((out.best, &out.payload), (raw.best, &solution));
    assert!(out.stats.nodes <= raw.nodes, "{} of {} nodes", out.stats.nodes, raw.nodes);
    // With the optimum as the ceiling as well, there is nothing to search.
    control.set_ceiling(raw.best);
    let out = solve(&controlled(config(4), &control), fresh(), &inst);
    assert!(out.optimal_declared && out.counters.runs == 0, "{:?}", out.counters);
    assert_eq!((out.best, &out.payload), (raw.best, &solution));

    // While the run goes.
    let big = big_instance();
    let opt = big_raw().best;
    let solution = solve(&config(4), fresh(), &big).payload;
    let control = Control::new();
    let cfg = controlled(config(2), &control);
    let (out, accepted) = alongside(
        || {
            wait_for(&control, |status| status.nodes > 0).expect("the run going");
            control.offer(opt, &solution)
        },
        || solve(&cfg, fresh(), &big),
    );
    assert!(out.completed);
    assert_optimal(&out, &big, opt);
    if accepted {
        assert_eq!(out.payload, solution, "nothing beats the optimum");
    }
}

/// A stop from the control stops the run with a final checkpoint, from which the search resumes to the exact end; a
/// control that was told to stop starts every later run stopped.
#[test]
fn a_stop_from_the_control_stops_the_run_with_a_checkpoint() {
    let inst = big_instance();
    let raw = big_raw();
    let dir = TempDir::new("control-stop");
    let ckpt = dir.join("ckpt.bin");
    let cfg = EngineConfig { checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))), ..config(1) };
    let control = Control::new();
    let stopping = controlled(cfg.clone(), &control);
    let (out, ()) = alongside(
        || {
            wait_for(&control, |status| status.nodes > 0).expect("the run going");
            control.stop();
        },
        || solve(&stopping, fresh(), &inst),
    );
    assert!(!out.completed && ckpt.exists() && out.counters.checkpoints == 1, "{:?}", out.counters);
    let again = solve(&stopping, Start::Resume(ckpt.clone()), &inst);
    assert!(!again.completed && again.counters.runs == 0 && again.stats == out.stats, "{:?}", again.counters);
    let rest = solve(&cfg, Start::Resume(ckpt.clone()), &inst);
    assert!(rest.completed && !ckpt.exists());
    assert_optimal(&rest, &inst, raw.best);
    assert_eq!(rest.stats.nodes, raw.nodes, "one thread, stopped and resumed, explores the tree of the plain search");
}

/// A stop given before the run makes it start stopped ("Starting stopped" in the engine's docs): no unit is handed
/// out, and the final checkpoint holds them all, however many times it is resumed under that stop.
#[test]
fn a_stop_given_before_the_run_starts_it_stopped() {
    let inst = instance();
    let dir = TempDir::new("control-stopped");
    let ckpt = dir.join("ckpt.bin");
    let control = Control::new();
    control.stop();
    let cfg =
        EngineConfig { checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))), ..controlled(config(4), &control) };
    let mut start = fresh();
    for run in 0..20 {
        let out = solve(&cfg, start, &inst);
        let c = &out.counters;
        assert_eq!((out.completed, out.stats.nodes, c.runs, c.checkpoints), (false, 0, 0, 1), "{run}: {c:?}");
        start = Start::Resume(ckpt.clone());
    }
    let out = solve(&EngineConfig { control: None, ..cfg }, start, &inst);
    assert!(out.completed);
    assert_optimal(&out, &inst, inst.dp());
}

/// The number of workers given before the run replaces the configured threads; workers added while the run goes, and
/// workers retired (running or idle) and added again, leave the answer exact, and the counters say what happened.
#[test]
fn workers_added_and_retired_while_the_run_goes() {
    let control = Control::new();
    control.set_workers(2);
    let out = solve(&controlled(config(4), &control), fresh(), &instance());
    let c = &out.counters;
    assert_eq!((c.threads, c.spawned, c.retired), (2, 0, 0), "{c:?}");

    let inst = big_instance();
    let raw = big_raw();
    // From one worker to four.
    let control = Control::new();
    let cfg = controlled(config(1), &control);
    let (out, grown) = alongside(
        || {
            wait_for(&control, |status| status.running == 1)?;
            control.set_workers(4);
            wait_for(&control, |status| status.live == 4 && status.running == 4)
        },
        || solve(&cfg, fresh(), &inst),
    );
    assert!(grown.is_some(), "four workers running before the end: {:?}", out.counters);
    assert!(out.completed);
    assert_optimal(&out, &inst, raw.best);
    let c = &out.counters;
    assert_eq!((c.threads, c.spawned, c.retired), (4, 3, 0), "{c:?}");
    assert!(c.capacity < c.wall * 4, "three workers came later: {c:?}");
    assert_accounting(c);

    // From four to one, which retires running workers, and back to three.
    let control = Control::new();
    let cfg = controlled(config(4), &control);
    let (out, script) = alongside(
        || {
            wait_for(&control, |status| status.running == 4)?;
            control.set_workers(1);
            wait_for(&control, |status| status.live == 1)?;
            control.set_workers(3);
            wait_for(&control, |status| status.live == 3)
        },
        || solve(&cfg, fresh(), &inst),
    );
    assert!(script.is_some(), "the changes made before the end: {:?}", out.counters);
    assert!(out.completed);
    assert_optimal(&out, &inst, raw.best);
    let c = &out.counters;
    assert_eq!((c.threads, c.spawned, c.retired), (4, 2, 3), "{c:?}");
    assert_accounting(c);
    assert_eq!(control.workers(), Some(3));
}

/// Running workers beyond a lowered target hand all their units over and retire without waiting for their units to
/// end: on a tree too deep ever to finish, whose units never end, the run goes from four workers to one, and the units
/// the others had wait in the deque.
#[test]
fn running_workers_beyond_the_target_retire_promptly() {
    let control = Control::new();
    let cfg = controlled(config(4), &control);
    let start = Start::Fresh { units: vec![Leafy(0)], best: 0, payload: Vec::new() };
    let make = |_| Walker { depth: 62, stack: Vec::new(), out: Vec::new() };
    let (out, lowered) = alongside(
        || {
            wait_for(&control, |status| status.running == 4)?;
            control.set_workers(1);
            let lowered = wait_for(&control, |status| status.live == 1);
            control.stop();
            lowered
        },
        || engine::run(&cfg, start, b"", make).unwrap(),
    );
    let lowered = lowered.unwrap_or_else(|| panic!("never down to one worker: {:?}", out.counters));
    assert!(lowered.running == 1 && lowered.queued > 0, "{lowered:?}");
    assert!(!out.completed, "stopped");
    let c = &out.counters;
    assert_eq!((c.threads, c.spawned, c.retired), (4, 0, 3), "{c:?}");
    assert_accounting(c);
}

/// A run told to have no worker while it goes is suspended: every worker retires, the units wait in the deque, and
/// nothing is searched until it is given workers again; then it ends at the exact optimum.
#[test]
fn a_run_suspended_while_it_goes_and_given_workers_again_is_exact() {
    let inst = long_instance();
    let control = Control::new();
    let cfg = controlled(config(4), &control);
    let (out, script) = alongside(
        || {
            wait_for(&control, |status| status.running == 4)?;
            control.set_workers(0);
            let suspended = wait_for(&control, |status| status.live == 0)?;
            thread::sleep(Duration::from_millis(20));
            let later = control.status()?;
            control.set_workers(3);
            wait_for(&control, |status| status.live == 3)?;
            Some((suspended, later))
        },
        || solve(&cfg, fresh(), &inst),
    );
    let (suspended, later) = script.unwrap_or_else(|| panic!("not suspended and resumed: {:?}", out.counters));
    assert!(suspended.running == 0 && suspended.idle == 0 && suspended.queued > 0, "{suspended:?}");
    let nothing_moved = (later.live, later.nodes, later.queued) == (0, suspended.nodes, suspended.queued);
    assert!(nothing_moved, "searched while suspended: {suspended:?} then {later:?}");
    assert!(out.completed && !out.optimal_declared);
    assert_optimal(&out, &inst, inst.dp());
    let c = &out.counters;
    assert_eq!((c.threads, c.spawned, c.retired), (4, 3, 4), "{c:?}");
    assert_accounting(c);
}

/// A run suspended before it starts (its control told 0) starts no worker and waits for one; with no unit at all it is
/// complete at once.
#[test]
fn a_run_suspended_before_it_starts_waits_for_workers() {
    let inst = instance();
    let control = Control::new();
    control.set_workers(0);
    let cfg = controlled(config(4), &control);
    let (out, script) = alongside(
        || {
            let waiting = wait_for(&control, |_| true)?;
            thread::sleep(Duration::from_millis(20));
            let later = control.status()?;
            control.set_workers(2);
            Some((waiting, later))
        },
        || solve(&cfg, fresh(), &inst),
    );
    let (waiting, later) = script.unwrap_or_else(|| panic!("not seen suspended: {:?}", out.counters));
    assert_eq!((waiting.live, waiting.queued, later.live, later.queued, later.nodes), (0, 1, 0, 1, 0), "{later:?}");
    assert!(out.completed);
    assert_optimal(&out, &inst, inst.dp());
    let c = &out.counters;
    assert_eq!((c.threads, c.spawned, c.retired), (2, 2, 0), "{c:?}");
    assert_accounting(c);

    let control = Control::new();
    control.set_workers(0);
    let start = Start::Fresh { units: Vec::new(), best: 17, payload: b"seed".to_vec() };
    let out = solve(&controlled(config(4), &control), start, &inst);
    assert!(out.completed && !out.optimal_declared);
    assert_eq!((out.best, out.payload.as_slice(), out.counters.threads), (17, &b"seed"[..], 0));
}

/// A suspended run checkpoints as it goes and stops with a final checkpoint, from which the search resumes to the
/// exact optimum.
#[test]
fn a_suspended_run_checkpoints_and_stops_and_resumes_exactly() {
    let inst = long_instance();
    let dir = TempDir::new("suspended-stop");
    let ckpt = dir.join("ckpt.bin");
    let control = Control::new();
    let cfg =
        EngineConfig { checkpoint: Some((ckpt.clone(), Duration::from_millis(5))), ..controlled(config(2), &control) };
    let (out, script) = alongside(
        || {
            wait_for(&control, |status| status.running == 2)?;
            control.set_workers(0);
            wait_for(&control, |status| status.live == 0)?;
            // A checkpoint written from now on is written while the run is suspended: it is later than any before.
            let elapsed = || engine::read_checkpoint(&ckpt).map(|head| head.elapsed).ok();
            let before = elapsed();
            let asked = Instant::now();
            while elapsed() <= before && asked.elapsed() < Duration::from_secs(60) {
                thread::sleep(Duration::from_millis(1));
            }
            let still = control.status()?;
            control.stop();
            Some((elapsed() > before, still))
        },
        || solve(&cfg, fresh(), &inst),
    );
    let (periodic, still) = script.unwrap_or_else(|| panic!("not suspended: {:?}", out.counters));
    assert!(periodic && still.live == 0, "no checkpoint while suspended: {still:?}, {:?}", out.counters);
    assert!(!out.completed && ckpt.exists() && out.counters.checkpoints >= 2, "{:?}", out.counters);
    assert_accounting(&out.counters);
    let rest = solve(&EngineConfig { control: None, ..cfg }, Start::Resume(ckpt.clone()), &inst);
    assert!(rest.completed && !ckpt.exists());
    assert_optimal(&rest, &inst, inst.dp());
}

/// A suspended run given a ceiling that its incumbent reaches ends at once, as optimal.
#[test]
fn a_suspended_run_given_a_ceiling_its_incumbent_reaches_ends_as_optimal() {
    let inst = long_instance();
    let control = Control::new();
    let cfg = controlled(config(2), &control);
    let (out, ceiling) = alongside(
        || {
            wait_for(&control, |status| status.running == 2 && status.best > 0)?;
            control.set_workers(0);
            wait_for(&control, |status| status.live == 0)?;
            let (best, _) = control.best()?;
            control.set_ceiling(best);
            Some(best)
        },
        || solve(&cfg, fresh(), &inst),
    );
    let ceiling = ceiling.unwrap_or_else(|| panic!("not suspended: {:?}", out.counters));
    assert!(out.completed && out.optimal_declared && out.best == ceiling, "{:?}", out.counters);
    let mask = u64::from_le_bytes(out.payload.as_slice().try_into().unwrap());
    assert_eq!(inst.check(mask), Some(out.best), "the payload is the incumbent's");
    let c = &out.counters;
    assert_eq!((c.threads, c.spawned, c.retired), (2, 0, 2), "{c:?}");
    assert_accounting(c);
}

/// A control steers one run at a time: a second run it is given is refused.
#[test]
fn a_control_is_attached_to_one_run_at_a_time() {
    let inst = big_instance();
    let control = Control::new();
    let cfg = controlled(config(1), &control);
    let (out, second) = alongside(
        || {
            wait_for(&control, |_| true).expect("the run going");
            let make = |_| Knapsack { inst: &inst, stack: Vec::new(), target: None, panic_when: None };
            let second = engine::run(&cfg, fresh(), HEADER, make).map(|out| out.completed);
            control.stop();
            second
        },
        || solve(&cfg, fresh(), &inst),
    );
    assert_eq!(second.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
    assert!(!out.completed, "stopped");
}

/// The knapsack searcher, made to take `rebuild` before its first poll of a unit and `step` between its first poll and
/// its second.
struct SlowStart<'a> {
    inner: Knapsack<'a>,
    rebuild: Duration,
    step: Duration,
}

impl Searcher<Node> for SlowStart<'_> {
    fn run(&mut self, unit: Node, ctx: &mut Ctx<'_, Node>) {
        thread::sleep(self.rebuild);
        if ctx.poll() == Poll::Yield {
            ctx.push(unit);
            return;
        }
        thread::sleep(self.step);
        self.inner.run(unit, ctx);
    }
}

/// The time from a run's start to its first poll (the rebuild) and from its first poll to its second (the first step)
/// is measured, and the ramp's and the tail's idle times are parts of the idle time.
#[test]
fn the_start_of_each_run_is_timed() {
    let inst = Instance::new(40, 10_000, 7);
    let (rebuild, step) = (Duration::from_millis(2), Duration::from_millis(3));
    let make = |_| SlowStart {
        inner: Knapsack { inst: &inst, stack: Vec::new(), target: None, panic_when: None },
        rebuild,
        step,
    };
    // One thread runs the root's two children, one after the other, and never waits.
    let skip = Node { next: 1, ..Node::ROOT };
    let take = Node { next: 1, mask: 1, weight: inst.w[0], value: inst.v[0] };
    let start = Start::Fresh { units: vec![skip, take], best: 0, payload: Vec::new() };
    let out = engine::run(&config(1), start, HEADER, make).unwrap();
    assert!(out.completed);
    assert_optimal(&out, &inst, inst.dp());
    let c = &out.counters;
    assert_eq!((c.runs, c.first_steps), (2, 2), "{c:?}");
    assert!(c.rebuild >= rebuild * 2 && c.first_step >= step * 2, "{c:?}");
    assert_eq!((c.idle, c.ramp_idle, c.tail_idle), (Duration::ZERO, Duration::ZERO, Duration::ZERO), "{c:?}");
    assert_accounting(c);
    // Four threads split the work: every run is timed.
    let out = engine::run(&config(4), fresh(), HEADER, make).unwrap();
    assert!(out.completed);
    assert_optimal(&out, &inst, inst.dp());
    let c = &out.counters;
    let runs = u32::try_from(c.runs).unwrap();
    // A run polls once and stops (a tiny run) only if its first poll says to yield.
    assert!(c.runs > 1 && c.first_steps == c.runs - c.tiny_runs, "{c:?}");
    assert!(c.rebuild >= rebuild * runs && c.first_step >= step * u32::try_from(c.first_steps).unwrap(), "{c:?}");
    assert_accounting(c);
    let summary = c.summary();
    assert!(["ramp ", "tail ", "rebuild ", "first "].iter().all(|field| summary.contains(field)), "{summary}");
}

// ------------------------------------------------------------------------------------------------------------
// Benchmark: cargo test --release --test engine -- --ignored --nocapture bench
// ------------------------------------------------------------------------------------------------------------

/// A complete binary tree with no work per node, to isolate the engine's per-node cost.
#[derive(Clone, Copy)]
struct Leafy(u32);

impl Unit for Leafy {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0.to_le_bytes());
    }

    fn decode(b: &[u8]) -> Self {
        Leafy(u32::from_le_bytes(b.try_into().unwrap()))
    }
}

/// Walk the tree below `root` (whose children have depth `root.0 + 1`, down to `depth`).
fn walk(stack: &mut Vec<Leafy>, root: Leafy, depth: u32, mut poll: impl FnMut() -> Poll, out: &mut Vec<Leafy>) -> u64 {
    let mut nodes = 0;
    stack.clear();
    stack.push(root);
    while let Some(node) = stack.pop() {
        if poll() == Poll::Yield {
            stack.push(node);
            out.append(stack);
            break;
        }
        nodes += 1;
        if node.0 < depth {
            stack.push(Leafy(node.0 + 1));
            stack.push(Leafy(node.0 + 1));
        }
    }
    nodes
}

struct Walker {
    depth: u32,
    stack: Vec<Leafy>,
    out: Vec<Leafy>,
}

impl Searcher<Leafy> for Walker {
    fn run(&mut self, unit: Leafy, ctx: &mut Ctx<'_, Leafy>) {
        let nodes = walk(&mut self.stack, unit, self.depth, || ctx.poll(), &mut self.out);
        ctx.stats().nodes += nodes;
        for u in self.out.drain(..) {
            ctx.push(u);
        }
    }
}

#[test]
#[ignore]
///
/// The environment can change the thread counts (`TXENGINE_BENCH_THREADS`, default `1,2,4,8`), the knapsack's
/// items (`TXENGINE_BENCH_ITEMS`, default [`BIG_ITEMS`]: 0.2 G nodes; 64: 0.65 G, 72: 4.2 G, 80: 60 G) and the
/// binary tree's depth (`TXENGINE_BENCH_DEPTH`, default 27: 0.27 G nodes, each level more doubles it).  The
/// unthreaded runs take about 2.6 ns per knapsack node and 1.2 ns per tree node on an M1 CPU, so on many CPUs
/// the defaults finish in milliseconds; there `TXENGINE_BENCH_ITEMS=80 TXENGINE_BENCH_DEPTH=35` measure
/// something (and take about four minutes for the unthreaded runs).
fn bench() {
    let threads: Vec<usize> = bench_env("TXENGINE_BENCH_THREADS", "1,2,4,8")
        .split(',')
        .map(|t| t.parse().expect("TXENGINE_BENCH_THREADS: a list of thread counts"))
        .collect();
    let items: usize =
        bench_env("TXENGINE_BENCH_ITEMS", &BIG_ITEMS.to_string()).parse().expect("TXENGINE_BENCH_ITEMS: a number");
    let depth: u32 = bench_env("TXENGINE_BENCH_DEPTH", "27").parse().expect("TXENGINE_BENCH_DEPTH: a number");

    let inst = Instance::new(items, 10_000, 99);
    let t = Instant::now();
    let raw = raw_search(&inst);
    let raw_secs = t.elapsed().as_secs_f64();
    println!("knapsack, {items} items: {} nodes", raw.nodes);
    println!("  raw dfs           {:7.3} s  {:6.1} M nodes/s", raw_secs, raw.nodes as f64 / raw_secs / 1e6);
    for &threads in &threads {
        let out = solve(&config(threads), fresh(), &inst);
        let secs = out.elapsed.as_secs_f64();
        println!(
            "  engine {threads:3} threads {secs:7.3} s  {:6.1} M nodes/s  ({} nodes) {}",
            out.stats.nodes as f64 / secs / 1e6,
            out.stats.nodes,
            out.counters.summary()
        );
    }

    let t = Instant::now();
    let nodes = walk(&mut Vec::new(), Leafy(0), depth, || Poll::Continue, &mut Vec::new());
    let raw_secs = t.elapsed().as_secs_f64();
    println!("empty binary tree, depth {depth}: {nodes} nodes");
    println!("  raw walk          {:7.3} s  {:6.2} ns/node", raw_secs, raw_secs * 1e9 / nodes as f64);
    for &threads in &threads {
        let start = Start::Fresh { units: vec![Leafy(0)], best: 0, payload: Vec::new() };
        let make = |_| Walker { depth, stack: Vec::new(), out: Vec::new() };
        let out = engine::run(&config(threads), start, b"", make).unwrap();
        assert_eq!(out.stats.nodes, nodes);
        let secs = out.elapsed.as_secs_f64();
        println!(
            "  engine {threads:3} threads {secs:7.3} s  {:6.2} ns/node  ({:.2} ns/node/thread) {}",
            secs * 1e9 / nodes as f64,
            secs * 1e9 * threads as f64 / nodes as f64,
            out.counters.summary()
        );
    }
}

/// The benchmark setting `name` from the environment, or `default`.
fn bench_env(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}
