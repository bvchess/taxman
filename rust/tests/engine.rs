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

use std::time::{Duration, Instant};

use taxman::engine::{self, Ctx, EngineConfig, Outcome, Poll, Searcher, Start, Unit};

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
/// inside the run, so together they take at most `wall × threads`; every pause that parked the workers ended later.
fn assert_accounting(c: &engine::Counters) {
    let capacity = c.wall * u32::try_from(c.threads).unwrap();
    assert!(c.busy + c.idle + c.parked + c.lock_wait <= capacity, "{c:?}");
    assert!((0.0..=1.0).contains(&c.efficiency()), "{c:?}");
    assert!(c.full_occupancy.is_none_or(|t| t <= c.wall), "{c:?}");
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
