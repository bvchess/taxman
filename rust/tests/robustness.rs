//! Robustness of the engine, the checkpoints and the command line: no unit of work lost or doubled under random
//! configurations, any checkpoint interval, absurd limits, checkpoints that belong to their game, a cap reached
//! on resuming, a closed stdout or stderr, stop signals, leftover spill segments, and a checkpoint directory that
//! two processes would share.
//!
//! `TXENGINE_CONSERVATION=<iterations>` runs more random configurations of the conservation test.

mod common;

use common::TempDir;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use std::sync::mpsc;
use std::time::Duration;

use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::{self, Ctx, EngineConfig, Poll, Searcher, Start, Unit};
use taxman::json::Json;
use taxman::search::{self, SearchTactics, TaxUnit};
use taxman::solver::{self, SolveOptions};

// ------------------------------------------------------------------------------------------------------------
// A tree of known size
// ------------------------------------------------------------------------------------------------------------

fn mix(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    x ^ (x >> 33)
}

/// A node of an irregular tree: `id` determines its children.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Node {
    depth: u32,
    id: u64,
}

impl Unit for Node {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.depth.to_le_bytes());
        out.extend_from_slice(&self.id.to_le_bytes());
    }

    fn decode(b: &[u8]) -> Self {
        assert_eq!(b.len(), 12);
        Node {
            depth: u32::from_le_bytes(b[..4].try_into().unwrap()),
            id: u64::from_le_bytes(b[4..].try_into().unwrap()),
        }
    }
}

/// A tree with no pruning: a search of it must visit every node exactly once, so with any threads, splits,
/// spills, pauses, stops and resumes the nodes counted equal its size, and one unit lost or explored twice shows.
#[derive(Clone, Copy, Debug)]
struct Tree {
    max_depth: u32,
    salt: u64,
}

impl Tree {
    /// 2 or 3 children near the root, then 0 to 3 (about 1.6 on average).
    fn children(&self, node: Node) -> u64 {
        if node.depth >= self.max_depth {
            0
        } else if node.depth < 4 {
            2 + mix(node.id ^ self.salt) % 2
        } else {
            [0, 1, 2, 2, 2, 3, 3, 0][(mix(node.id ^ self.salt) % 8) as usize]
        }
    }

    fn child(node: Node, k: u64) -> Node {
        Node { depth: node.depth + 1, id: mix(node.id.wrapping_mul(31).wrapping_add(k + 1)) }
    }

    fn value(node: Node) -> u64 {
        mix(node.id) >> 24
    }

    fn root() -> Node {
        Node { depth: 0, id: 1 }
    }

    /// The nodes and the largest value, by a plain depth-first walk.
    fn walk(&self) -> (u64, u64) {
        let mut stack = vec![Tree::root()];
        let (mut nodes, mut best) = (0, 0);
        while let Some(node) = stack.pop() {
            nodes += 1;
            best = best.max(Tree::value(node));
            stack.extend((0..self.children(node)).map(|k| Tree::child(node, k)));
        }
        (nodes, best)
    }
}

struct Walker {
    tree: Tree,
    stack: Vec<Node>,
    /// When yielding, explore the nodes without children on the spot instead of pushing them, as the Taxman
    /// searcher does with nodes that have no candidates left (so that a split can give nothing).
    drop_leaves: bool,
}

impl Walker {
    fn visit(node: Node, ctx: &mut Ctx<'_, Node>) {
        ctx.stats().nodes += 1;
        let value = Tree::value(node);
        if value > ctx.best() {
            ctx.offer(value, &node.id.to_le_bytes());
        }
    }
}

impl Searcher<Node> for Walker {
    fn run(&mut self, unit: Node, ctx: &mut Ctx<'_, Node>) {
        self.stack.clear();
        self.stack.push(unit);
        while let Some(node) = self.stack.pop() {
            if ctx.poll() == Poll::Yield {
                self.stack.push(node);
                for unit in std::mem::take(&mut self.stack) {
                    if self.drop_leaves && self.tree.children(unit) == 0 {
                        Walker::visit(unit, ctx);
                    } else {
                        ctx.push(unit);
                    }
                }
                return;
            }
            Walker::visit(node, ctx);
            let tree = self.tree;
            self.stack.extend((0..tree.children(node)).map(|k| Tree::child(node, k)));
        }
    }
}

const HEADER: &[u8] = b"robustness tree";

fn fresh() -> Start<Node> {
    Start::Fresh { units: vec![Tree::root()], best: 0, payload: Vec::new() }
}

/// Runs to completion through any number of resumes; returns the final outcome and the invocations.
fn run_through(cfg: &EngineConfig, tree: Tree, drop_leaves: bool) -> (engine::Outcome, usize) {
    let mut start = fresh();
    for invocation in 1..=10_000 {
        let make = |_| Walker { tree, stack: Vec::new(), drop_leaves };
        let out = engine::run(cfg, start, HEADER, make).expect("engine run");
        if out.completed {
            return (out, invocation);
        }
        let (path, _) = cfg.checkpoint.as_ref().expect("a stopped run has a checkpoint");
        start = Start::Resume(path.clone());
    }
    panic!("no progress in 10000 invocations with {cfg:?}");
}

fn dir_is_empty(dir: &Path) -> bool {
    fs::read_dir(dir).map(|mut entries| entries.next().is_none()).unwrap_or(true)
}

/// `f` on another thread, failing the test if it takes longer than `limit` (a livelock would never end).
fn within<T: Send + 'static>(limit: Duration, what: &str, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(limit).unwrap_or_else(|_| panic!("{what}: not done after {limit:?}"))
}

// ------------------------------------------------------------------------------------------------------------
// The engine
// ------------------------------------------------------------------------------------------------------------

#[test]
fn conservation_is_exact_under_random_configurations() {
    let iterations: u64 = std::env::var("TXENGINE_CONSERVATION").ok().and_then(|s| s.parse().ok()).unwrap_or(40);
    let mut rng = 0x1234_5678_9abc_def1u64;
    let mut pick = |options: &[u64]| {
        rng = mix(rng.wrapping_add(0x9e37_79b9_7f4a_7c15));
        options[(rng % options.len() as u64) as usize]
    };
    let dir = TempDir::new("conservation");
    let (ckpt, spill) = (dir.join("ckpt"), dir.join("spill"));
    for it in 0..iterations {
        let tree = Tree { max_depth: pick(&[16, 20, 24]) as u32, salt: pick(&[1, 2, 3, 4, 5, 6, 7]) };
        let (size, best) = tree.walk();
        let ms = |x: u64| Duration::from_millis(x);
        // u64::MAX stands for "none".  (An interval of zero, with a time limit shorter than a checkpoint's fsync,
        // gets each invocation little further than `MIN_POLLS` polls: it is tested on its own below.)
        let interval = pick(&[u64::MAX, 1, 3, 20]);
        let limit = pick(&[u64::MAX, u64::MAX, 1, 3]);
        let cfg = EngineConfig {
            threads: pick(&[1, 2, 3, 5, 8, 16]) as usize,
            queue_mem_cap: pick(&[1, 2, 5, 50, 1 << 20]) as usize,
            spill_dir: Some(spill.clone()),
            checkpoint: Some((
                ckpt.clone(),
                if interval == u64::MAX { Duration::from_secs(3600) } else { ms(interval) },
            )),
            keep_completed_checkpoint: pick(&[0, 1]) == 1,
            time_limit: (limit != u64::MAX).then(|| ms(limit)),
            node_limit: (pick(&[0, 0, 1]) == 1).then_some(size / 5 + 1),
            ..EngineConfig::default()
        };
        let drop_leaves = pick(&[0, 1]) == 1;
        let (out, invocations) = run_through(&cfg, tree, drop_leaves);
        let context = format!("iteration {it}: {invocations} invocations, drop_leaves {drop_leaves}, {cfg:?}");
        assert_eq!((out.stats.nodes, out.best), (size, best), "{context}");
        if cfg.keep_completed_checkpoint {
            engine::remove_checkpoint(&ckpt).unwrap();
        }
        assert!(!ckpt.exists(), "{context}: checkpoint left behind");
        assert!(dir_is_empty(&spill), "{context}: spill segments left behind");
    }
}

/// With a checkpoint interval of zero the coordinator used to pause again at once after every release, without
/// letting go of the lock: the workers never ran again.  Now each pause is followed by work.
#[test]
fn a_checkpoint_interval_of_zero_makes_progress() {
    // Big enough (about 0.1 s on one thread) for several checkpoints, each with its fsync.
    let tree = Tree { max_depth: 26, salt: 3 };
    let (size, _) = tree.walk();
    // One and two threads in one go, and two in pieces (stopped by a node limit and resumed) with spilling.
    for (threads, pieces) in [(1, false), (2, false), (2, true)] {
        let dir = TempDir::new("zero");
        let ckpt = dir.join("ckpt");
        let cfg = EngineConfig {
            threads,
            checkpoint: Some((ckpt.clone(), Duration::ZERO)),
            node_limit: pieces.then_some(size / 4),
            queue_mem_cap: if pieces { 2 } else { 1 << 20 },
            spill_dir: Some(dir.join("spill")),
            ..EngineConfig::default()
        };
        let what = format!("{threads} threads, in pieces {pieces}");
        let (out, invocations) = within(Duration::from_secs(120), &what, move || run_through(&cfg, tree, false));
        assert!(out.completed, "{what}");
        assert_eq!(out.stats.nodes, size, "{what}");
        assert!(invocations >= if pieces { 3 } else { 1 }, "{what}: {invocations} invocations");
        // At least one checkpoint was taken while the search kept going.  (Asking for two was timing-dependent:
        // an optimized build can finish the tree before the second pause; the last piece can be too small for any.)
        assert!(pieces || out.counters.checkpoints >= 1, "{what}: {:?}", out.counters);
        assert!(!ckpt.exists() && dir_is_empty(&dir.join("spill")), "{what}");
    }
}

/// A run whose deadline has already passed when it starts, which is what a time limit of zero is, does no search
/// work however the threads are scheduled.  Only the coordinator looked at the deadline, so the workers, started
/// first, could finish a small search before its first look, and the run completed instead of stopping with its
/// checkpoint (with one worker about one run in a thousand on a loaded machine; with many nearly every run, since
/// the first has the start-up of the others to itself).  Now no worker is started: the run stops with every unit
/// in its checkpoint, however often it is resumed under the same deadline, and a run resumed without it finds
/// nothing lost.  (A stop signal that was received before the run starts is the same stop: `tests/stop_signal.rs`.)
#[test]
fn a_deadline_that_has_passed_stops_the_run_before_any_search() {
    // Many runs of a search small enough to finish within a start-up, with many workers and no checkpoint file.
    let small = Tree { max_depth: 5, salt: 1 };
    let expired = EngineConfig { threads: 16, time_limit: Some(Duration::ZERO), ..EngineConfig::default() };
    for run in 0..300 {
        let make = |_| Walker { tree: small, stack: Vec::new(), drop_leaves: false };
        let out = engine::run(&expired, fresh(), HEADER, make).unwrap();
        let nothing = (out.completed, out.stats.nodes, out.counters.runs, out.counters.checkpoints);
        assert_eq!(nothing, (false, 0, 0, 0), "run {run}: {:?}", out.counters);
    }

    // A search stopped part-way, with spilling, resumed under a deadline that has passed, then without it.
    let tree = Tree { max_depth: 20, salt: 2 };
    let (size, best) = tree.walk();
    let dir = TempDir::new("expired");
    let (ckpt, spill) = (dir.join("ckpt"), dir.join("spill"));
    let make = |_| Walker { tree, stack: Vec::new(), drop_leaves: false };
    let cfg = EngineConfig {
        threads: 4,
        checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
        queue_mem_cap: 2,
        spill_dir: Some(spill.clone()),
        node_limit: Some(size / 3),
        ..EngineConfig::default()
    };
    assert!(!engine::run(&cfg, fresh(), HEADER, make).unwrap().completed);
    let head = engine::read_checkpoint(&ckpt).unwrap();
    let expired = EngineConfig { node_limit: None, time_limit: Some(Duration::ZERO), ..cfg.clone() };
    for resume in 0..5 {
        let out = engine::run(&expired, Start::Resume(ckpt.clone()), HEADER, make).unwrap();
        assert!(
            !out.completed && out.counters.runs == 0 && out.counters.checkpoints == 1,
            "{resume}: {:?}",
            out.counters
        );
        assert_eq!((out.stats, out.best), (head.stats, head.best), "resume {resume}: no node explored");
        let again = engine::read_checkpoint(&ckpt).unwrap();
        assert_eq!((again.stats, again.best, &again.payload), (head.stats, head.best, &head.payload), "{resume}");
        assert!(dir_is_empty(&spill), "resume {resume}: spill segments left behind");
    }
    let rest = EngineConfig { node_limit: None, time_limit: None, ..cfg };
    let out = engine::run(&rest, Start::Resume(ckpt.clone()), HEADER, make).unwrap();
    assert!(out.completed && (out.stats.nodes, out.best) == (size, best), "every unit was kept: {:?}", out.counters);
    assert!(!ckpt.exists() && dir_is_empty(&spill));
}

/// Intervals and limits too long to add to the clock are never reached (they used to overflow `Instant`).
#[test]
fn absurd_intervals_and_limits_are_never_reached() {
    let tree = Tree { max_depth: 16, salt: 5 };
    let (size, best) = tree.walk();
    let dir = TempDir::new("absurd");
    let cfg = EngineConfig {
        threads: 2,
        checkpoint: Some((dir.join("ckpt"), Duration::MAX)),
        progress: Some(Duration::MAX),
        time_limit: Some(Duration::MAX),
        node_limit: Some(u64::MAX),
        ..EngineConfig::default()
    };
    let out = engine::run(&cfg, fresh(), HEADER, |_| Walker { tree, stack: Vec::new(), drop_leaves: false }).unwrap();
    assert!(out.completed);
    assert_eq!((out.stats.nodes, out.best, out.counters.checkpoints), (size, best, 0));
}

#[test]
fn a_completed_run_can_keep_its_checkpoint() {
    let tree = Tree { max_depth: 20, salt: 2 };
    let (size, _) = tree.walk();
    let dir = TempDir::new("keep");
    let ckpt = dir.join("ckpt");
    let make = |_| Walker { tree, stack: Vec::new(), drop_leaves: false };
    let stop = EngineConfig {
        threads: 2,
        checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
        node_limit: Some(size / 3),
        ..EngineConfig::default()
    };
    assert!(!engine::run(&stop, fresh(), HEADER, make).unwrap().completed);
    let keep = EngineConfig { node_limit: None, keep_completed_checkpoint: true, ..stop };
    let out = engine::run(&keep, Start::Resume(ckpt.clone()), HEADER, make).unwrap();
    assert!(out.completed);
    // The file the run resumed from stays, intact, until the caller removes it.
    let head = engine::read_checkpoint(&ckpt).unwrap();
    assert!(head.stats.nodes < out.stats.nodes && head.header == HEADER);
    engine::remove_checkpoint(&ckpt).unwrap();
    assert!(!ckpt.exists());
    engine::remove_checkpoint(&ckpt).unwrap();
}

/// Segments of a process that no longer exists are deleted when a run starts in their directory; those of live
/// processes are not.
#[cfg(unix)]
#[test]
fn spill_segments_of_dead_processes_are_removed() {
    let dir = TempDir::new("orphans");
    let spill = dir.join("spill");
    fs::create_dir_all(&spill).unwrap();
    // A process that has just ended: its id is free (and not reused that soon).
    let mut child = Command::new("true").spawn().unwrap();
    let dead = child.id();
    child.wait().unwrap();
    let names = [
        format!("txspill-{dead}-0-000000.seg"),
        format!("txspill-{dead}-3-000017.seg"),
        "txspill-1-0-000000.seg".to_owned(),
        format!("txspill-{}-999-000000.seg", std::process::id()),
        "unrelated.seg".to_owned(),
    ];
    for name in &names {
        fs::write(spill.join(name), b"x").unwrap();
    }
    let tree = Tree { max_depth: 12, salt: 1 };
    let cfg = EngineConfig { threads: 1, spill_dir: Some(spill.clone()), ..EngineConfig::default() };
    assert!(
        engine::run(&cfg, fresh(), HEADER, |_| Walker { tree, stack: Vec::new(), drop_leaves: false })
            .unwrap()
            .completed
    );
    let left: Vec<bool> = names.iter().map(|name| spill.join(name).exists()).collect();
    assert_eq!(left, [false, false, true, true, true], "{names:?}");
}

#[test]
fn a_locked_directory_is_refused() {
    let dir = TempDir::new("lock");
    let held = engine::lock_dir(&dir.0).unwrap();
    let err = engine::lock_dir(&dir.0).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::WouldBlock);
    let message = err.to_string();
    assert!(message.contains("in use by another process") && message.contains(engine::LOCK_FILE), "{message}");
    assert!(message.contains(&format!("process {}", std::process::id())), "{message}");
    drop(held);
    let _again = engine::lock_dir(&dir.0).unwrap();
}

// ------------------------------------------------------------------------------------------------------------
// Checkpoints of Taxman searches
// ------------------------------------------------------------------------------------------------------------

/// Game `n` from the frame builder (at width 16), prepared with the tactics of the matching path, whose search of
/// 600 is long enough to stop.
fn prepared_game(n: u32) -> (Board<16>, search::Prepared<16>) {
    let board = Board::<16>::for_n(n);
    let prepared = board.prepare(SearchTactics::matching()).unwrap();
    (board, prepared)
}

/// A search of game 600 stopped early, leaving a checkpoint at `ckpt`.
fn stopped_search(ckpt: &Path) -> (Board<16>, EngineConfig) {
    let (board, prepared) = prepared_game(600);
    let cfg = EngineConfig {
        threads: 1,
        checkpoint: Some((ckpt.to_path_buf(), Duration::from_secs(3600))),
        node_limit: Some(1500),
        ..EngineConfig::default()
    };
    assert!(!search::search(&prepared, &cfg, None, None, false).unwrap().completed);
    assert!(ckpt.exists());
    (board, EngineConfig { node_limit: None, ..cfg })
}

/// A search resumed with an incumbent that already reaches the cap is finished at once.
#[test]
fn a_resumed_search_already_at_its_cap_finishes_at_once() {
    let dir = TempDir::new("cap");
    let ckpt = dir.join("n0600.ckpt");
    for keep in [false, true] {
        let (board, cfg) = stopped_search(&ckpt);
        let head = engine::read_checkpoint(&ckpt).unwrap();
        let resumed = board.prepare_for_resume(&ckpt).unwrap();
        let cfg = EngineConfig { keep_completed_checkpoint: keep, ..cfg };
        let r = search::search(&resumed, &cfg, None, Some(head.best), true).unwrap();
        assert!(r.completed && r.reached_cap && r.sum >= head.best, "keep {keep}: {r:?}");
        assert_eq!(r.stats, head.stats, "keep {keep}: no node explored, the earlier runs' counted");
        assert_eq!(ckpt.exists(), keep);
        let _ = fs::remove_file(&ckpt);
    }
}

/// Searchers that only wait to be stopped, to write a checkpoint with any incumbent.
struct Stall;

impl Searcher<TaxUnit<16>> for Stall {
    fn run(&mut self, unit: TaxUnit<16>, ctx: &mut Ctx<'_, TaxUnit<16>>) {
        while ctx.poll() == Poll::Continue {}
        ctx.push(unit);
    }
}

/// A checkpoint with the right header but an incumbent that is not a promotion set of the game, or not worth
/// what is stored with it, is refused instead of pruning the search.
#[test]
fn a_checkpoint_whose_incumbent_does_not_fit_is_refused() {
    let dir = TempDir::new("payload");
    let ckpt = dir.join("n0600.ckpt");
    let (board, cfg) = stopped_search(&ckpt);
    let header = engine::read_checkpoint_header(&ckpt).unwrap();
    let real = engine::read_checkpoint(&ckpt).unwrap();
    let resumed = board.prepare_for_resume(&ckpt).unwrap();
    let words = |set: Bits<16>| set.as_words().iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<u8>>();
    let not_a_candidate = (1..600).find(|&x| !board.candidates().contains(x)).unwrap();
    let cases = [
        (real.best + 1, real.payload.clone(), "worth"),
        (u64::from(not_a_candidate), words(Bits::EMPTY.with(not_a_candidate)), "cannot be promoted"),
        (0, vec![0; 3], "bytes"),
    ];
    for (best, payload, complaint) in cases {
        let forged = EngineConfig { threads: 1, time_limit: Some(Duration::from_millis(1)), ..cfg.clone() };
        let root = TaxUnit { promoted: Bits::EMPTY, cands: board.candidates() };
        let start = Start::Fresh { units: vec![root], best, payload };
        assert!(!engine::run(&forged, start, &header, |_| Stall).unwrap().completed);
        let err = search::search(&resumed, &cfg, None, None, true).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData, "{err}");
        assert!(err.to_string().contains(complaint), "{complaint}: {err}");
        assert!(ckpt.exists(), "a refused checkpoint is left alone");
    }
}

/// One thread, checkpoints in `dir`, and the tactics of the matching path, whose search of 600 is long enough to stop.
fn options(dir: &Path) -> SolveOptions {
    SolveOptions { checkpoint_dir: Some(dir.to_path_buf()), force_search: true, ..common::matching_options(1) }
}

/// Games 140 and 141 have the same candidates and the same frame 0, but other frames: before checkpoints recorded the
/// board size and the frames, 140's was resumed as 141's.
#[test]
fn a_checkpoint_renamed_to_another_game_is_refused() {
    let dir = TempDir::new("renamed");
    let opts = SolveOptions { time_limit: Some(Duration::ZERO), ..options(&dir.0) };
    assert!(!solver::solve_game(140, None, &opts).unwrap().completed);
    fs::copy(dir.join("n0140.ckpt"), dir.join("n0141.ckpt")).unwrap();
    let err = solver::solve_game(141, None, &opts).unwrap_err();
    assert!(err.contains("n0141.ckpt") && err.contains("the game n=140, not n=141"), "{err}");
    assert!(dir.join("n0141.ckpt").exists(), "a refused checkpoint is left alone");
}

/// Changes the search's header in the checkpoint `path` by `edit`, which gets the file's bytes from the header's
/// magic on (`search::header`), and rewrites the file's checksum to match: a forgery that only the checks of the
/// header can see.
fn forge_header(path: &Path, edit: impl FnOnce(&mut [u8])) {
    let mut bytes = fs::read(path).unwrap();
    let len = bytes.len();
    // The engine's file ends with a marker and the checksum of everything before it.
    assert_eq!(&bytes[len - 16..len - 8], b"TXCKEND1");
    let checksum = |bytes: &[u8]| engine::checksum(&bytes[..len - 16]).to_le_bytes();
    assert_eq!(bytes[len - 8..], checksum(&bytes), "the checksum is of what precedes the marker");
    // The search's header follows the engine's 20 bytes.
    let at = 20;
    assert_eq!(&bytes[at..at + 4], b"TXSH");
    edit(&mut bytes[at..]);
    let sum = checksum(&bytes);
    bytes[len - 8..].copy_from_slice(&sum);
    fs::write(path, bytes).unwrap();
}

#[test]
fn a_checkpoint_made_with_other_tactics_names_them() {
    let dir = TempDir::new("tactics");
    let opts = SolveOptions { node_limit: Some(1500), ..options(&dir.0) };
    assert!(!solver::solve_game(600, None, &opts).unwrap().completed);
    let mut other = opts.clone();
    other.tactics.set("nogoods=off").unwrap();
    other.tactics.set("match_refine=3").unwrap();
    other.tactics.set("gf2=on").unwrap();
    let err = solver::solve_game(600, None, &other).unwrap_err();
    for part in ["nogoods is on, not off", "match_refine is 1, not 3", "gf2 is off, not on", "--tactic nogoods=on"] {
        assert!(err.contains(part), "{part}: {err}");
    }
}

/// A checkpoint whose header has another format version (here 3, that of the builds before this one, whose headers
/// recorded tactics that are gone) is refused with one message, naming the file, and left alone.
#[test]
fn a_checkpoint_of_another_format_is_refused() {
    let dir = TempDir::new("format");
    let opts = SolveOptions { node_limit: Some(1500), ..options(&dir.0) };
    assert!(!solver::solve_game(600, None, &opts).unwrap().completed);
    let checkpoint = dir.join("n0600.ckpt");
    forge_header(&checkpoint, |header| header[4..8].copy_from_slice(&3u32.to_le_bytes()));
    let err = solver::solve_game(600, None, &opts).unwrap_err();
    for part in ["n0600.ckpt", "format version 3", "reads version 4 only", "delete the checkpoint"] {
        assert!(err.contains(part), "{part}: {err}");
    }
    assert!(checkpoint.exists(), "a refused checkpoint is left alone");
}

// ------------------------------------------------------------------------------------------------------------
// The command line
// ------------------------------------------------------------------------------------------------------------

fn taxman() -> Command {
    Command::new(env!("CARGO_BIN_EXE_taxman"))
}

fn run(args: &[&str]) -> (ExitStatus, String, String) {
    let out = taxman().args(args).output().unwrap();
    (out.status, String::from_utf8_lossy(&out.stdout).into(), String::from_utf8_lossy(&out.stderr).into())
}

#[test]
fn bad_times_and_limits_are_usage_errors() {
    for (flag, value) in [
        ("--checkpoint-interval", "0"),
        ("--checkpoint-interval", "nan"),
        ("--time-limit", "0"),
        ("--time-limit", "1e300"),
        ("--time-limit", "-3"),
        ("--progress", "-1"),
        ("--progress", "1e300"),
        ("--progress", "inf"),
        ("--node-limit", "0"),
    ] {
        let (status, _, stderr) = run(&["solve", "10", flag, value]);
        assert_eq!(status.code(), Some(2), "{flag} {value}: {stderr}");
        assert!(stderr.contains(flag) && !stderr.contains("panicked"), "{flag} {value}: {stderr}");
    }
    // No progress lines at all is a choice, not a mistake.
    let (status, stdout, _) = run(&["solve", "10", "--progress", "0"]);
    assert!(status.success() && stdout.contains("n=10 "), "{stdout}");
}

/// The lines of a results file as JSON.
fn results(path: &Path) -> Vec<Json> {
    fs::read_to_string(path).unwrap().lines().map(|line| Json::parse(line).unwrap()).collect()
}

fn completed(line: &Json) -> bool {
    line.get("completed").and_then(Json::as_bool).unwrap()
}

/// Writing to a closed stdout or stderr fails (EPIPE) instead of killing the process: the run goes on, and
/// its results file is complete.
#[test]
fn a_closed_stdout_and_stderr_do_not_stop_a_run() {
    let dir = TempDir::new("epipe");
    let out = dir.join("results.jsonl");
    let ckpt = dir.join("ckpt");
    let mut child = taxman()
        .args(["solve", "936", "--force-search", "--threads", "2", "--progress", "0.002", "--show"])
        .args(["--out".as_ref(), out.as_os_str(), "--checkpoint-dir".as_ref(), ckpt.as_os_str()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    drop(child.stderr.take());
    let status = child.wait().unwrap();
    assert!(status.success(), "{status}");
    let lines = results(&out);
    assert_eq!(lines.len(), 1);
    assert!(completed(&lines[0]) && lines[0].get("optimal_match") == Some(&Json::Bool(true)));
}

#[test]
fn a_checkpoint_directory_in_use_is_refused() {
    let dir = TempDir::new("busy");
    let _held = engine::lock_dir(&dir.0).unwrap();
    let (status, _, stderr) = run(&["solve", "600", "--checkpoint-dir", dir.0.to_str().unwrap()]);
    assert_eq!(status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("in use by another process"), "{stderr}");
}

/// Runs `solve 944` (on optimal.json's game of 943) with progress lines, and once the search is under way sends
/// `signal` (if any); returns the exit status and stdout.  The tactics are the matching path's, whose search of 944
/// lasts long enough to be stopped (the GF(2) search's takes under a tenth of a second).
#[cfg(unix)]
fn solve_944(dir: &TempDir, signal: Option<&str>) -> (ExitStatus, String) {
    let out = dir.join("results.jsonl");
    let ckpt = dir.join("ckpt");
    let mut child = taxman()
        .args(["solve", "944", "--use-prev", "--force-search", "--threads", "2", "--progress", "0.02"])
        .args(["--tactic", "gf2=off", "--tactic", "lagrange=on"])
        .args(["--out".as_ref(), out.as_os_str(), "--checkpoint-dir".as_ref(), ckpt.as_os_str()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stderr = BufReader::new(child.stderr.take().unwrap());
    let (tx, rx) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in stderr.lines().map_while(Result::ok) {
            let _ = tx.send(line);
        }
    });
    if let Some(signal) = signal {
        // The first progress line says the search is running.
        let line = rx.recv_timeout(Duration::from_secs(60)).expect("a progress line");
        assert!(line.starts_with("n=944 ") && line.contains(" nodes "), "{line}");
        let pid = child.id().to_string();
        assert!(Command::new("kill").args([signal, &pid]).status().unwrap().success());
    }
    let mut stdout = String::new();
    std::io::Read::read_to_string(&mut child.stdout.take().unwrap(), &mut stdout).unwrap();
    let status = child.wait().unwrap();
    reader.join().unwrap();
    let last = rx.try_iter().last().unwrap_or_default();
    if signal.is_some() {
        assert!(last.contains("(stopped)"), "the final progress line: {last}");
    }
    (status, stdout)
}

/// SIGINT and SIGTERM stop a search with a final checkpoint, and the next run resumes it to the end.
#[cfg(unix)]
#[test]
fn a_signal_stops_the_search_with_a_checkpoint_to_resume() {
    let dir = TempDir::new("signal");
    let ckpt = dir.join("ckpt").join("n0944.ckpt");
    let mut nodes = 0;
    for (signal, code) in [("-INT", 130), ("-TERM", 143)] {
        let (status, stdout) = solve_944(&dir, Some(signal));
        assert_eq!(status.code(), Some(code), "{signal}: {stdout}");
        assert!(stdout.contains("stopped by a signal"), "{signal}: {stdout}");
        assert!(ckpt.exists(), "{signal}: the final checkpoint");
        let lines = results(&dir.join("results.jsonl"));
        let last = lines.last().unwrap();
        assert!(!completed(last), "{signal}");
        let so_far = last.get("nodes").and_then(Json::as_u64).unwrap();
        assert!(so_far >= nodes, "{signal}: nodes {so_far} after {nodes}");
        nodes = so_far;
        let head = engine::read_checkpoint(&ckpt).unwrap();
        assert_eq!(head.stats.nodes, so_far, "{signal}: the checkpoint holds the work done");
    }
    let (status, stdout) = solve_944(&dir, None);
    assert!(status.success(), "{stdout}");
    let lines = results(&dir.join("results.jsonl"));
    assert_eq!(lines.len(), 3);
    let last = lines.last().unwrap();
    assert!(completed(last) && last.get("optimal_match") == Some(&Json::Bool(true)), "{stdout}");
    assert!(last.get("nodes").and_then(Json::as_u64).unwrap() > nodes);
    assert!(!ckpt.exists());
}
