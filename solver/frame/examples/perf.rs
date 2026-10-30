//! The performance probe: replays of the search's hot paths on three games that were among the hardest when it was
//! made, measured in CPU cycles and instructions retired of this process (other load on the machine barely moves
//! them, unlike wall time), each with a fingerprint of everything it computes.  An optimization that keeps the search
//! exact must leave every fingerprint unchanged; one that changes the tree on purpose changes the windows'.
//!
//! ```text
//! cargo run --release --example perf -- [--reps R] [--steps K] [--brief] [--fixtures DIR] [--tactic NAME=VALUE]...
//!     [GAME...]
//! cargo run --release --example perf -- make-nodes N [--known FILE] [--every K] [--keep M] [--fixtures DIR]
//! cargo run --release --example perf -- make-window N AT [--known FILE] [--fixtures DIR] [--tactic NAME=VALUE]...
//! ```
//!
//! The fixtures, in `testdata/perf/` unless `--fixtures` says otherwise (games 936, 992 and 1215 by default, at W = 16,
//! 16 and 20):
//!
//! * `nNNNN.nodes` (fixture F1, search nodes): nodes of the proof of game N (one thread, seeded with an optimal set),
//!   each its promotions and candidates; 936 and 1215 have them, made when their proofs took 81,119 and 4,652,709 nodes
//!   (`make-nodes 936 --every 2000 --keep 300` and `make-nodes 1215 --every 100000 --keep 300`, by builds that had
//!   the matching path).  This build's proofs of them take 449 and 3,704 nodes, in one unit each, so `make-nodes` would
//!   make fixtures of one node: these stay, since a node is a node of the game whatever search found it.  Each kernel
//!   replays all of them: the rebuild of a
//!   node's state (`promote_all_using`), the promotion of every candidate (`promote_in_place_using`, the peel) and
//!   `can_promote_using`, the minimal conflict and the one from the residual (the `core` line) of the heaviest
//!   matching (as the start's dive finds it), the matching from scratch, and lookups (with no member forced) and
//!   rule-outs in a nogood table learned from the nodes' conflicts.
//! * Fixture F2, frame checks, made from F1 on the fly: the two frames that promoting each candidate of each node
//!   changes (one gains the move, one loses the factor), each checked by `FrameGraph::winnable_using`.
//! * Fixture F4, the GF(2) bound's kernels at the nodes of F1 ([`gf2_kernels`]).
//! * `nNNNN.window.ckpt` (fixture F3, a search window): a checkpoint (TXSH, as `taxman solve` writes) of that proof
//!   after some nodes, resumed on one thread for `--steps` steps (default 50,000, which the searches end before).  A
//!   resumed search starts with empty nogood tables, so the window is the same tree every time, and its fingerprint is
//!   the search's counters (steps, promotions, children, bound checks, ...), less the GF(2) bound's counts of solves
//!   and augmentations, which measure work rather than the tree.  The checkpoint is read-only.  The windows are of this
//!   build's default search, `make-window 992 1024` and `make-window 1215 1024`: the proof, seeded with an optimal
//!   set, stopped by a node limit of 1,024, which the engine reaches after 2,047 nodes, the earliest point it stops
//!   at; the proofs take 11,441 and 3,704 nodes in all, so the windows are the rest of them.  Game 936, whose proof
//!   takes 449 nodes, has none (the search ends before the engine looks at a node limit), and 992 has no nodes.
//!   `--tactic` changes the window's tactics (the search tactics only), and `make-window` takes `--tactic` as the
//!   probe does, on top of the defaults.  A game in `--fixtures` with a window and no nodes (a copy of a chain's
//!   checkpoint, renamed `nNNNN.window.ckpt`) has only the window, and one with nodes and no window only the kernels.
//!
//! Each kernel runs `--reps` times (default 3; the window 2) and the fastest by cycles is reported, per
//! operation and in all; the fingerprints of all runs must agree.  The last line sums the kernels' cycles and
//! instructions and combines their fingerprints.  On Linux there is no per-process cycle counter without a crate, so
//! the probe reports wall time only; run it under `perf stat` there.
//!
//! The fixtures are made with the commands `make-nodes` (the nodes handed to the searcher by a chain of runs of
//! `--every` nodes each, every run resuming the last one's checkpoint, so that each hands it the stack the last
//! one left; deterministic; `--keep` of them kept, evenly spaced) and `make-window` (the proof stopped after
//! `AT` nodes).  The window's checkpoint records the set width, so a build that picks another width for N
//! refuses it: make it again (the tree, and so the fingerprint, stays the same).

#[path = "common/counters.rs"]
mod counters;
#[path = "common/fixtures.rs"]
mod fixtures;

use fixtures::{read_nodes, to_hex};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::{self, Ctx, EngineConfig, Searcher, Start};
use taxman::game::Optimal;
use taxman::gf2::{Gf2Bound, Gf2Graph};
use taxman::matchbound::{MatchBound, MatchGraph};
use taxman::model::{FrameGraph, Lookup, Nogoods, Scratch};
use taxman::search::{search, stat, Prepared, SearchSpace, SearchTactics, TaxSearcher, TaxUnit};
use taxman::tactics::Tactics;

/// The games whose fixtures the probe runs by default.
const GAMES: [u32; 3] = [936, 992, 1215];

/// The fixture of game `n` of this kind in `dir`.
fn fixture(dir: &Path, n: u32, kind: &str) -> PathBuf {
    dir.join(format!("n{n:04}.{kind}"))
}

// ---------------------------------------------------------------------------------------------------------
// Counting cycles and instructions

/// What one kernel cost: the fastest of its runs by cycles (by wall time without counters).
#[derive(Clone, Copy, Debug, Default)]
struct Cost {
    cycles: Option<u64>,
    instructions: Option<u64>,
    wall: Duration,
}

/// Runs `f` `reps` times and returns its cost and fingerprint (which every run must give).
fn measure(reps: u32, mut f: impl FnMut(&mut Fingerprint)) -> (Cost, u64) {
    let mut best: Option<Cost> = None;
    let mut seen = None;
    for _ in 0..reps.max(1) {
        let mut fp = Fingerprint::default();
        let before = counters::now();
        let t0 = Instant::now();
        f(&mut fp);
        let wall = t0.elapsed();
        let after = counters::now();
        let (instructions, cycles) = match (before, after) {
            (Some((i0, c0)), Some((i1, c1))) => (Some(i1 - i0), Some(c1 - c0)),
            _ => (None, None),
        };
        assert!(seen.is_none_or(|s| s == fp.0), "a kernel gave two fingerprints");
        seen = Some(fp.0);
        let cost = Cost { cycles, instructions, wall };
        let better = |b: &Cost| match (cycles, b.cycles) {
            (Some(c), Some(bc)) => c < bc,
            _ => wall < b.wall,
        };
        if best.as_ref().is_none_or(better) {
            best = Some(cost);
        }
    }
    (best.expect("one run"), seen.expect("one run"))
}

/// A running hash of a kernel's results.
#[derive(Clone, Copy, Debug)]
struct Fingerprint(u64);

impl Default for Fingerprint {
    fn default() -> Self {
        Fingerprint(0x243F_6A88_85A3_08D3)
    }
}

impl Fingerprint {
    #[inline]
    fn add(&mut self, x: u64) {
        self.0 = (self.0.rotate_left(5) ^ x).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
}

/// The report: one line per kernel, and the totals.
#[derive(Default)]
struct Report {
    brief: bool,
    cycles: u64,
    instructions: u64,
    wall: Duration,
    fingerprint: Fingerprint,
}

impl Report {
    fn line(&mut self, label: &str, ops: u64, cost: Cost, fingerprint: u64) {
        self.cycles += cost.cycles.unwrap_or(0);
        self.instructions += cost.instructions.unwrap_or(0);
        self.wall += cost.wall;
        self.fingerprint.add(fingerprint);
        if self.brief {
            return;
        }
        let per = |x: Option<u64>| x.map_or("-".into(), |x| format!("{:.0}", x as f64 / ops.max(1) as f64));
        println!(
            "  {label:<46} {ops:>9} {:>9} {:>9} {:>8.1}  {fingerprint:016x}",
            per(cost.cycles),
            per(cost.instructions),
            cost.wall.as_secs_f64() * 1e9 / ops.max(1) as f64
        );
    }

    fn header(&self, title: &str) {
        if !self.brief {
            println!("{title}");
            println!("  {:<46} {:>9} {:>9} {:>9} {:>8}  fingerprint", "kernel", "ops", "cyc/op", "ins/op", "ns/op");
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Games and fixtures

/// A game set up as the searches of the hardest games are: from the frame builder, with the default tactics.
fn setup<const W: usize>(n: u32) -> Prepared<W> {
    Board::<W>::for_n(n).prepare(SearchTactics::default()).expect("prepare")
}

/// The promotions of the optimal game that can be promoted from the root, one by one from the largest down.
fn seed<const W: usize>(p: &Prepared<W>, optimal: &Optimal) -> Bits<W> {
    let moves = optimal.moves(p.n).unwrap_or_else(|| panic!("no known game for n={} (--known FILE?)", p.n));
    let mut state = p.root;
    moves.iter().copied().filter(|&x| p.candidates.contains(x)).fold(Bits::EMPTY, |set, x| {
        if p.model.promote_in_place(&mut state, x) {
            set.with(x)
        } else {
            set
        }
    })
}

// ---------------------------------------------------------------------------------------------------------
// Fixtures F1 and F2: the kernels on the captured nodes

/// One captured node, rebuilt: its state, the heaviest matching's candidates and their minimal conflict.
struct Node<const W: usize> {
    promoted: Bits<W>,
    cands: Bits<W>,
    state: taxman::model::FrameSet<W>,
    chosen: Bits<W>,
    conflict: Option<Bits<W>>,
}

fn kernels<const W: usize>(path: &Path, reps: u32, report: &mut Report) -> Result<(), String> {
    let (n, incumbent, units) = read_nodes::<W>(path)?;
    let p = setup::<W>(n);
    let model: &FrameGraph = &p.model;
    let graph = MatchGraph::new(model, &p.root);
    let mut scratch = Scratch::<W>::new();
    if !incumbent.is_subset(&p.candidates) || model.promote_all_using(&mut scratch, &p.root, &incumbent).is_none() {
        return Err(format!("{}: the incumbent is not a promotion set of game {n}", path.display()));
    }
    let mut mb = MatchBound::new(&graph);
    let mut nodes = Vec::new();
    for u in units.iter().filter(|u| !u.cands.is_empty()) {
        let fits = u.promoted.is_subset(&p.candidates) && u.cands.is_subset(&p.candidates);
        let state = model.promote_all_using(&mut scratch, &p.root, &u.promoted).filter(|_| fits);
        let state = state.ok_or_else(|| format!("{}: a node that is not one of game {n}", path.display()))?;
        mb.reset(&u.promoted, &u.cands);
        let chosen = mb.chosen();
        let conflict = model.conflict_using(&mut scratch, &state, &chosen, true);
        nodes.push(Node { promoted: u.promoted, cands: u.cands, state, chosen, conflict });
    }
    let count = |f: fn(&Node<W>) -> u32| nodes.iter().map(|x| u64::from(f(x))).sum::<u64>();
    let (cands, chosen, conflicts) =
        (count(|x| x.cands.len()), count(|x| x.chosen.len()), count(|x| x.conflict.map_or(0, |k| k.len())));
    let nodes_len = nodes.len() as u64;
    report.header(&format!(
        "n={n} W={W}: {} nodes; per node {:.1} candidates, {:.1} in the heaviest matching, {:.2} in its conflict",
        nodes.len(),
        cands as f64 / nodes_len as f64,
        chosen as f64 / nodes_len as f64,
        conflicts as f64 / nodes_len as f64
    ));

    let (cost, fp) = measure(reps, |fp| {
        for x in &nodes {
            let state = model.promote_all_using(&mut scratch, &p.root, &x.promoted).expect("promotable");
            for f in state.frames() {
                fp.add(u64::from(f.moves.len()) << 32 | u64::from(f.factors.len()));
            }
        }
    });
    report.line("rebuild: promote_all_using(root, P)", nodes_len, cost, fp);
    let (cost, fp) = measure(reps, |fp| {
        for x in &nodes {
            for c in x.cands.iter() {
                let mut s = x.state;
                fp.add(u64::from(model.promote_in_place_using(&mut scratch, &mut s, c)));
            }
        }
    });
    report.line("promote_in_place_using(state, x), x in C", cands, cost, fp);
    let (cost, fp) = measure(reps, |fp| {
        for x in &nodes {
            for c in x.cands.iter() {
                fp.add(u64::from(model.can_promote_using(&mut scratch, &x.state, c)));
            }
        }
    });
    report.line("can_promote_using(state, x), x in C", cands, cost, fp);
    for (label, minimal) in [("conflict_using(state, Q*), minimal", true), ("conflict_using(state, Q*), core", false)] {
        let (cost, fp) = measure(reps, |fp| {
            for x in &nodes {
                fp.add(model.conflict_using(&mut scratch, &x.state, &x.chosen, minimal).map_or(0, |k| k.sum()));
            }
        });
        report.line(label, nodes_len, cost, fp);
    }
    let (cost, fp) = measure(reps, |fp| {
        for x in &nodes {
            mb.reset(&x.promoted, &x.cands);
            fp.add(mb.value());
        }
    });
    report.line("matching from scratch: reset(P, C) + value", nodes_len, cost, fp);

    let root_max = p.root.frames().iter().filter_map(|f| (f.moves | f.factors).max()).max().unwrap_or(0);
    let mut table = Nogoods::<W>::new(root_max, 1 << 16);
    for x in &nodes {
        if let Some(k) = x.conflict {
            if let Some(ng) = model.nogood(&mut scratch, &p.root, &x.promoted, &k) {
                table.insert(ng);
            }
        }
    }
    let (cost, fp) = measure(reps, |fp| {
        for x in &nodes {
            fp.add(match table.lookup(&x.promoted, &x.chosen, &Bits::EMPTY) {
                Lookup::None => 0,
                Lookup::AllForced => 1,
                Lookup::Conflict(k) => k.sum(),
            });
        }
    });
    report.line(&format!("Nogoods::lookup(P, Q*), {} nogoods", table.len()), nodes_len, cost, fp);
    let pushes = nodes.iter().map(|x| u64::from(x.chosen.len().min(8))).sum();
    let (cost, fp) = measure(reps, |fp| {
        for x in &nodes {
            for c in x.chosen.iter_desc().take(8) {
                let added = Bits::EMPTY.with(c);
                fp.add(table.ruled_out(&added, &(x.promoted | added), &x.cands).sum());
            }
        }
    });
    report.line("Nogoods::ruled_out(x, P + x, C), 8 largest of Q*", pushes, cost, fp);

    // Fixture F2: the frame checks of promoting each candidate.
    let mut checks: Vec<(Bits<W>, Bits<W>)> = Vec::new();
    for x in &nodes {
        for c in x.cands.iter() {
            let h = model.home(c);
            let frames = x.state.frames();
            if h == 0 || !frames[h].factors.contains(c) {
                continue;
            }
            checks.push((frames[h - 1].moves.with(c), frames[h - 1].factors));
            checks.push((frames[h].moves, frames[h].factors.without(c)));
        }
    }
    let (cost, fp) = measure(reps, |fp| {
        for (moves, factors) in &checks {
            fp.add(u64::from(model.winnable_using(&mut scratch, moves, factors)));
        }
    });
    report.line("F2 frame checks: winnable_using(moves, factors)", checks.len() as u64, cost, fp);
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// Fixture F4: the GF(2) bound's kernels

/// The GF(2) bound's kernels at the nodes of the F1 fixture `path` ([`taxman::gf2::Gf2Bound`]), each over all the
/// nodes: a node's level from the root and its bound from scratch; the same with its forced members (the weight
/// split); and the bound without each member of the conflict of its heaviest set (strong branching's solves,
/// warm-started from the level's history, to the end).  Fingerprinted by the values.
fn gf2_kernels<const W: usize>(path: &Path, reps: u32, report: &mut Report) -> Result<(), String> {
    let (n, incumbent, units) = read_nodes::<W>(path)?;
    let p = setup::<W>(n);
    let model: &FrameGraph = &p.model;
    let graph = Gf2Graph::new(model, &p.root, &p.candidates);
    let mut scratch = Scratch::<W>::new();
    let best = incumbent.sum();
    // (promoted, candidates, max_add, need, the conflict of the heaviest set)
    let mut nodes = Vec::new();
    let mut bound = Gf2Bound::<W>::new(&graph);
    for u in units.iter().filter(|u| !u.cands.is_empty()) {
        let state = model.promote_all_using(&mut scratch, &p.root, &u.promoted).ok_or("a node of the game")?;
        let cap = (p.max_total - u.promoted.len()).min(state.max_promotions_among(&u.cands).max(0) as u32);
        bound.reset(&u.promoted, &u.cands);
        let chosen = bound.evaluate(cap, None).chosen;
        let conflict = model.conflict_using(&mut scratch, &state, &chosen, true).unwrap_or(Bits::EMPTY);
        let need = (best + 1).saturating_sub(u.promoted.sum());
        nodes.push((u.promoted, u.cands, cap, need, conflict));
    }
    let count = nodes.len() as u64;
    let members: u64 = nodes.iter().map(|x| u64::from(x.4.len())).sum();
    report.header(&format!(
        "n={n} W={W}, the GF(2) bound: {} nodes, rank {:?} per frame, {:.1} members a conflict",
        nodes.len(),
        graph.ranks(),
        members as f64 / count.max(1) as f64
    ));
    let mut b = Gf2Bound::<W>::new(&graph);
    let (cost, fp) = measure(reps, |fp| {
        for &(promoted, cands, cap, _, _) in &nodes {
            b.reset(&promoted, &cands);
            fp.add(b.evaluate(cap, None).value);
        }
    });
    report.line("GF(2): reset + bound from scratch", count, cost, fp);
    let (cost, fp) = measure(reps, |fp| {
        for &(promoted, cands, cap, need, _) in &nodes {
            b.reset(&promoted, &cands);
            let e = b.evaluate(cap, Some(need));
            fp.add(e.value);
            fp.add(e.forced.sum());
        }
    });
    report.line("GF(2): reset + bound + forced (split)", count, cost, fp);
    let before = b.stats();
    let (cost, fp) = measure(reps, |fp| {
        for &(promoted, cands, cap, _, conflict) in &nodes {
            b.reset(&promoted, &cands);
            b.evaluate(cap, None);
            for x in conflict.iter() {
                fp.add(b.value_excluding(x, cap, u64::MAX));
            }
        }
    });
    report.line("GF(2): bound without each conflict member", members, cost, fp);
    if !report.brief {
        let after = b.stats();
        let (solves, augmentations) = (after.solves - before.solves, after.augmentations - before.augmentations);
        let rounds = after.rounds - before.rounds;
        println!(
            "    (in this kernel {:.1} augmentations a solve, {:.2} Bellman-Ford rounds an augmentation)",
            augmentations as f64 / solves.max(1) as f64,
            rounds as f64 / augmentations.max(1) as f64
        );
    }
    // Strong branching as the search does it: the split, then the bounds the heaviest set's exchange graph
    // certifies, and solves for the others.  The values, and so the fingerprint, are the kernel's above.
    let (mut tried, mut certified) = (Vec::new(), Vec::new());
    let before = b.stats();
    let (cost, fp) = measure(reps, |fp| {
        for &(promoted, cands, cap, need, conflict) in &nodes {
            b.reset(&promoted, &cands);
            b.evaluate(cap, Some(need));
            tried.clear();
            tried.extend(conflict.iter());
            b.certified_excluding(&tried, cap, &mut certified);
            for (&x, value) in tried.iter().zip(&certified) {
                fp.add(value.unwrap_or_else(|| b.value_excluding(x, cap, u64::MAX)));
            }
        }
    });
    report.line("GF(2): split + bound without each, certified or solved", members, cost, fp);
    if !report.brief {
        let after = b.stats();
        let runs = u64::from(reps.max(1));
        println!(
            "    (in this kernel {:.1}% of the bounds certified)",
            100.0 * (after.certified - before.certified) as f64 / (members * runs).max(1) as f64
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// Fixture F3: the search window

/// The window of the checkpoint `path` of game `n`: `steps` steps, with the tactics the checkpoint records changed
/// by `changes` (`name=value`, search tactics only).
fn window<const W: usize>(
    n: u32,
    path: &Path,
    steps: u64,
    changes: &[String],
    reps: u32,
    report: &mut Report,
) -> Result<(), String> {
    let mut p = Board::<W>::for_n(n)
        .prepare_for_resume(path)
        .map_err(|e| format!("{}: {e} (make it again with `perf make-window`)", path.display()))?;
    let mut tactics = Tactics { search: p.tactics, ..Tactics::default() };
    for change in changes {
        tactics.set(change).map_err(|e| format!("--tactic: {e}"))?;
    }
    if tactics != (Tactics { search: tactics.search, ..Tactics::default() }) {
        return Err("--tactic: the window only searches, so only search tactics apply".into());
    }
    p.tactics = tactics.search;
    let space = SearchSpace::new(&p);
    let header = engine::read_checkpoint_header(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let before = engine::read_checkpoint(path).map_err(|e| format!("{}: {e}", path.display()))?.stats.nodes;
    let cfg = EngineConfig { threads: 1, node_limit: Some(steps), progress: None, ..EngineConfig::default() };
    let mut failure = None;
    let (mut done, mut completed) = (0, false);
    let (cost, fp) = measure(reps.min(2), |fp| {
        // Without a checkpoint of its own the run leaves the file as it is.
        match engine::run(&cfg, Start::Resume(path.to_path_buf()), &header, |_| TaxSearcher::new(&space)) {
            Ok(o) => {
                (done, completed) = (o.stats.nodes, o.completed);
                // The GF(2) bound's counters of solves and augmentations measure work, not the tree (a faster
                // bound makes fewer for the same tree), so they count as zero.
                let mut extra = o.stats.extra;
                for k in
                    [stat::GF2_SOLVES, stat::GF2_AUGMENTATIONS, stat::GF2_STRONG_SOLVES, stat::GF2_STRONG_AUGMENTATIONS]
                {
                    extra[k] = 0;
                }
                for x in [o.stats.nodes, o.best].into_iter().chain(extra) {
                    fp.add(x);
                }
            }
            Err(e) => failure = Some(e.to_string()),
        }
    });
    if let Some(e) = failure {
        return Err(format!("{}: {e}", path.display()));
    }
    let name = path.file_name().map_or("?".into(), |s| s.to_string_lossy().into_owned());
    // A search that ends inside the window (other tactics than the checkpoint's, say) is costed by the steps it took.
    let ops = if completed { done - before } else { steps };
    report.line(&format!("F3 window: {steps} steps from {name}"), ops, cost, fp);
    if !report.brief {
        let end = if completed { ", finished" } else { "" };
        println!("    (the checkpoint's search had {done} nodes after the window{end})");
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// Making the fixtures

/// Records every unit it is given, then searches it.
struct Recorder<'a, const W: usize> {
    inner: TaxSearcher<'a, W>,
    log: &'a Mutex<Vec<TaxUnit<W>>>,
}

impl<const W: usize> Searcher<TaxUnit<W>> for Recorder<'_, W> {
    fn run(&mut self, unit: TaxUnit<W>, ctx: &mut Ctx<'_, TaxUnit<W>>) {
        self.log.lock().expect("log").push(unit);
        self.inner.run(unit, ctx);
    }
}

fn make_nodes<const W: usize>(dir: &Path, n: u32, optimal: &Optimal, every: u64, keep: usize) -> Result<(), String> {
    let p = setup::<W>(n);
    let incumbent = seed(&p, optimal);
    let space = SearchSpace::new(&p);
    let log = Mutex::new(Vec::new());
    let tmp = std::env::temp_dir().join(format!("taxman-perf-{n}-{}.ckpt", std::process::id()));
    let payload = incumbent.as_words().iter().flat_map(|w| w.to_le_bytes()).collect();
    let mut start = Start::Fresh {
        units: vec![TaxUnit { promoted: Bits::EMPTY, cands: p.candidates }],
        best: incumbent.sum(),
        payload,
    };
    let cfg = EngineConfig {
        threads: 1,
        checkpoint: Some((tmp.clone(), Duration::from_secs(1 << 20))),
        node_limit: Some(every),
        progress: None,
        ..EngineConfig::default()
    };
    let mut runs = 0;
    let total = loop {
        let o = engine::run(&cfg, start, b"perf", |_| Recorder { inner: TaxSearcher::new(&space), log: &log })
            .map_err(|e| e.to_string())?;
        runs += 1;
        if o.completed {
            break o.stats.nodes; // a resumed run counts the earlier runs' nodes too
        }
        start = Start::Resume(tmp.clone());
    };
    let mut seen = HashSet::new();
    let units: Vec<TaxUnit<W>> = log
        .into_inner()
        .expect("log")
        .into_iter()
        .filter(|u| seen.insert((*u.promoted.as_words(), *u.cands.as_words())))
        .collect();
    let kept: Vec<&TaxUnit<W>> =
        (0..keep.min(units.len())).map(|i| &units[i * units.len() / keep.min(units.len())]).collect();
    let path = fixture(dir, n, "nodes");
    let mut text = format!(
        "# Search nodes of game {n} for examples/perf.rs (F1), made by\n\
         # `perf make-nodes {n} --every {every} --keep {keep}`: its proof (one thread, default tactics, seeded with\n\
         # an optimal set) in {runs} runs of {every} nodes, each resuming the last one's checkpoint ({total} nodes in\n\
         # all), handed the searcher {} distinct units; these {} are evenly spaced among them.  Sets in hex, digit k\n\
         # holding the numbers 4k..4k+3, lowest bit first.\n",
        units.len(),
        kept.len()
    );
    text.push_str(&format!("n {n}\nincumbent {}\n", to_hex(&incumbent)));
    for u in kept {
        text.push_str(&format!("node {} {}\n", to_hex(&u.promoted), to_hex(&u.cands)));
    }
    std::fs::create_dir_all(path.parent().expect("a directory")).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("wrote {} ({runs} runs, {total} nodes, {} distinct units)", path.display(), units.len());
    Ok(())
}

fn make_window<const W: usize>(
    dir: &Path,
    n: u32,
    optimal: &Optimal,
    at: u64,
    changes: &[String],
) -> Result<(), String> {
    let mut tactics = Tactics::default();
    for change in changes {
        tactics.set(change).map_err(|e| format!("--tactic: {e}"))?;
    }
    let p = Board::<W>::for_n(n).prepare(tactics.search).map_err(|e| format!("n={n}: {e}"))?;
    let incumbent = seed(&p, optimal);
    let path = fixture(dir, n, "window.ckpt");
    std::fs::create_dir_all(path.parent().expect("a directory")).map_err(|e| e.to_string())?;
    let cfg = EngineConfig {
        threads: 1,
        checkpoint: Some((path.clone(), Duration::from_secs(1 << 20))),
        node_limit: Some(at),
        progress: None,
        ..EngineConfig::default()
    };
    let r = search(&p, &cfg, Some(&incumbent), None, false).map_err(|e| e.to_string())?;
    if r.completed {
        return Err(format!("the search of {n} completed within {at} nodes: no window to keep"));
    }
    println!("wrote {} after {} nodes ({:.1} s)", path.display(), r.stats.nodes, r.time.as_secs_f64());
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1).peekable();
    let command = match args.peek().map(String::as_str) {
        Some("make-nodes" | "make-window") => args.next(),
        _ => None,
    };
    let (mut reps, mut steps, mut brief, mut known, mut every, mut keep) = (3, 50_000, false, None, 100_000, 300);
    let mut changes = Vec::new();
    let mut dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/perf");
    let mut numbers = Vec::new();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        let parse = |text: String| text.parse::<u64>().map_err(|e| format!("{text}: {e}"));
        match arg.as_str() {
            "--reps" => reps = parse(value("--reps")?)? as u32,
            "--steps" => steps = parse(value("--steps")?)?,
            "--brief" => brief = true,
            "--known" => known = Some(PathBuf::from(value("--known")?)),
            "--every" => every = parse(value("--every")?)?,
            "--keep" => keep = parse(value("--keep")?)? as usize,
            "--fixtures" => dir = PathBuf::from(value("--fixtures")?),
            "--tactic" => changes.push(value("--tactic")?),
            _ => numbers.push(parse(arg)?),
        }
    }
    let optimal = || Optimal::built_in().with_results_files(known.as_slice());
    match (command.as_deref(), &numbers[..]) {
        (Some("make-nodes"), &[n]) => {
            let n = n as u32;
            taxman::with_width!(n, W => make_nodes::<W>(&dir, n, &optimal()?, every, keep))
        }
        (Some("make-window"), &[n, at]) => {
            let n = n as u32;
            taxman::with_width!(n, W => make_window::<W>(&dir, n, &optimal()?, at, &changes))
        }
        (Some(command), _) => Err(format!("{command}: wrong arguments (see the top of examples/perf.rs)")),
        (None, games) => {
            let games: Vec<u32> =
                if games.is_empty() { GAMES.to_vec() } else { games.iter().map(|&n| n as u32).collect() };
            let mut report = Report { brief, ..Report::default() };
            // A tactic the probe does not know is refused before anything runs.
            let mut tactics = Tactics::default();
            for change in &changes {
                tactics.set(change).map_err(|e| format!("--tactic: {e}"))?;
            }
            for n in games {
                // A game with a window and no nodes (a chain's checkpoint, say) has only the window, and one with nodes
                // and no window only the kernels.
                let (nodes, checkpoint) = (fixture(&dir, n, "nodes"), fixture(&dir, n, "window.ckpt"));
                if !nodes.exists() && !checkpoint.exists() {
                    return Err(format!("no fixture of game {n} in {}", dir.display()));
                }
                taxman::with_width!(n, W => {
                    if nodes.exists() {
                        kernels::<W>(&nodes, reps, &mut report)?;
                        gf2_kernels::<W>(&nodes, reps, &mut report)?;
                    } else {
                        report.header(&format!("n={n} W={W}, a search window"));
                    }
                    if checkpoint.exists() {
                        window::<W>(n, &checkpoint, steps, &changes, reps, &mut report)?;
                    }
                });
            }
            let g = |x: u64| x as f64 / 1e9;
            println!(
                "probe total: cycles {:.3} G, instructions {:.3} G, wall {:.2} s; fingerprint {:016x}",
                g(report.cycles),
                g(report.instructions),
                report.wall.as_secs_f64(),
                report.fingerprint.0
            );
            Ok(())
        }
    }
}
