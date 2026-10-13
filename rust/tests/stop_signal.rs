//! A run that starts after the process has received a stop signal does no search work, however the threads are
//! scheduled: it stops with its checkpoint before it starts a worker ("Starting stopped" in the `engine` module
//! docs), the way a run with a time limit of zero does (`tests/robustness.rs`).
//!
//! This is a test binary of its own, with one test: the signal flag is the process's, and once set it stays set,
//! so no other run in the process could complete.

mod common;

use common::TempDir;
use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

use taxman::engine::{self, Ctx, EngineConfig, Searcher, Start, Stats, Unit};

/// A unit of work that is one node.
struct Leaf(u64);

impl Unit for Leaf {
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0.to_le_bytes());
    }

    fn decode(bytes: &[u8]) -> Self {
        Leaf(u64::from_le_bytes(bytes.try_into().expect("8 bytes")))
    }
}

struct Counting;

impl Searcher<Leaf> for Counting {
    fn run(&mut self, unit: Leaf, ctx: &mut Ctx<'_, Leaf>) {
        ctx.stats().nodes += 1;
        ctx.offer(unit.0, &unit.0.to_le_bytes());
    }
}

const HEADER: &[u8] = b"stop signal";
const UNITS: u64 = 100;

fn fresh() -> Start<Leaf> {
    Start::Fresh { units: (1..=UNITS).map(Leaf).collect(), best: 0, payload: Vec::new() }
}

#[cfg(unix)]
#[test]
fn a_run_that_starts_after_a_stop_signal_does_no_search_work() {
    let dir = TempDir::new("stop-signal");
    let ckpt = dir.join("ckpt");
    let cfg = EngineConfig {
        threads: 8,
        checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
        queue_mem_cap: 10,
        spill_dir: Some(dir.join("spill")),
        ..EngineConfig::default()
    };
    engine::stop_on_signals().unwrap();

    // Before the signal the run is an ordinary one: it explores every unit.
    let out = engine::run(&cfg, fresh(), HEADER, |_| Counting).unwrap();
    assert!(out.completed && out.stats.nodes == UNITS && out.best == UNITS, "{out:?}");

    let me = std::process::id().to_string();
    assert!(Command::new("kill").args(["-TERM", &me]).status().unwrap().success());
    // The handler runs on some thread of this process, a moment after `kill` has returned.
    let sent = Instant::now();
    while engine::stop_signal().is_none() {
        assert!(sent.elapsed() < Duration::from_secs(30), "the signal was not received");
        std::thread::yield_now();
    }

    // A fresh run stops before any search, with all its units in the checkpoint ...
    let out = engine::run(&cfg, fresh(), HEADER, |_| Counting).unwrap();
    assert!(!out.completed && out.counters.runs == 0 && out.counters.checkpoints == 1, "{:?}", out.counters);
    assert_eq!((out.stats, out.best), (Stats::default(), 0));
    let head = engine::read_checkpoint(&ckpt).unwrap();
    assert_eq!((head.stats, head.best), (Stats::default(), 0));
    let size = fs::metadata(&ckpt).unwrap().len();

    // ... and so does every run resumed from it while the signal stands, writing the same units again.
    for resume in 0..3 {
        let out = engine::run(&cfg, Start::Resume(ckpt.clone()), HEADER, |_| Counting).unwrap();
        assert!(
            !out.completed && out.counters.runs == 0 && out.counters.checkpoints == 1,
            "{resume}: {:?}",
            out.counters
        );
        let again = engine::read_checkpoint(&ckpt).unwrap();
        assert_eq!((again.stats, again.best, again.header), (Stats::default(), 0, HEADER.to_vec()), "{resume}");
        assert_eq!(fs::metadata(&ckpt).unwrap().len(), size, "resume {resume}: the same units");
    }

    // Without a checkpoint file the run just stops.
    let bare = EngineConfig { checkpoint: None, ..cfg };
    let out = engine::run(&bare, fresh(), HEADER, |_| Counting).unwrap();
    assert!(!out.completed && out.counters.runs == 0 && out.stats.nodes == 0, "{:?}", out.counters);
}
