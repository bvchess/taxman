//! The performance probe: replays of the search's hot paths at the frontier, measured in CPU cycles and
//! instructions retired of this process (other load on the machine barely moves them, unlike wall time), each with
//! a fingerprint of everything it computes.  An optimization that keeps the search exact must leave every
//! fingerprint unchanged; one that changes the tree on purpose changes the window's.
//!
//! ```text
//! cargo run --release --example perf -- [--reps R] [--steps K] [--brief] [--fixtures DIR] [GAME...]
//! cargo run --release --example perf -- make-nodes N [--known FILE] [--every K] [--keep M] [--fixtures DIR]
//! cargo run --release --example perf -- make-window N AT [--known FILE] [--fixtures DIR]
//! ```
//!
//! The fixtures, in `testdata/perf/` unless `--fixtures` says otherwise (games 936 and 1215 by default: W = 16,
//! and the frontier):
//!
//! * `nNNNN.nodes` (F1, search nodes): nodes of the proof of game N (one thread, the strict tactics, seeded with
//!   the optimum), each its promotions and candidates.  Each kernel replays all of them: the rebuild of a node's
//!   state (`promote_all_using`), the promotion of every candidate (`promote_in_place_using`, the peel) and
//!   `can_promote_using`, the minimal conflict and the core of the heaviest matching, the matching from scratch
//!   and warm-started from the root's, the bounds without each member of the heaviest matching from the
//!   potentials, the exact ones without each member of the conflict (strong branching), children pushed and
//!   evaluated, and lookups and rule-outs in a nogood table learned from the nodes' conflicts.
//! * F2, frame checks, made from F1 on the fly: the two frames that promoting each candidate of each node changes
//!   (one gains the move, one loses the factor), each checked by `Model::winnable_using`.
//! * `nNNNN.window.ckpt` (F3, a search window): a checkpoint (TXSH, as `taxman solve` writes) of that proof after
//!   some nodes, resumed on one thread for `--steps` steps (default 50,000, one period of forgetting nogoods).  A
//!   resumed search starts with empty nogood tables, so the window is the same tree every time, and its
//!   fingerprint is the search's counters (steps, promotions, children, matching bounds, ...).  The checkpoint
//!   is read-only.
//!
//! Each kernel runs `--reps` times (default 3; the window 2) and the fastest by cycles is reported, per
//! operation and in all; the fingerprints of all runs must agree.  The incremental matching kernels (marked `+`)
//! each start from a level solved from scratch, and are reported net of that.  The last line sums the kernels'
//! cycles and instructions and combines their fingerprints.  On Linux there is no per-process cycle counter
//! without a crate, so the probe reports wall time only; run it under `perf stat` there.
//!
//! The fixtures are made with the commands `make-nodes` (the nodes handed to the searcher by a chain of runs of
//! `--every` nodes each, every run resuming the last one's checkpoint, so that each hands it the stack the last
//! one left; deterministic; `--keep` of them kept, evenly spaced) and `make-window` (the proof stopped after
//! `AT` nodes).  The window's checkpoint records the set width, so a build that picks another width for N
//! refuses it: make it again (the tree, and so the fingerprint, stays the same).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::{self, Ctx, EngineConfig, Searcher, Start};
use taxman::game::Optimal;
use taxman::matchbound::{MatchBound, MatchGraph};
use taxman::model::{Lookup, Model, Nogoods, Scratch};
use taxman::search::{search, Prepared, SearchSpace, SearchTactics, TaxSearcher, TaxUnit};
use taxman::solver::SolveOptions;

/// The games whose fixtures the probe runs by default.
const GAMES: [u32; 2] = [936, 1215];

/// The fixture of game `n` of this kind in `dir`.
fn fixture(dir: &Path, n: u32, kind: &str) -> PathBuf {
    dir.join(format!("n{n:04}.{kind}"))
}

/// The strict tactics (`revise_freedoms` off), which the frontier's results are made with.
fn tactics() -> SearchTactics {
    SearchTactics { revise_freedoms: false, ..SearchTactics::default() }
}

// ---------------------------------------------------------------------------------------------------------
// Counting cycles and instructions

#[cfg(target_os = "macos")]
mod counters {
    unsafe extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut u64) -> i32;
    }

    /// (instructions retired, cycles) of this process so far: `rusage_info_v4` from `proc_pid_rusage`, whose
    /// fields after the 16-byte uuid are u64s, `ri_instructions` and `ri_cycles` the 30th and 31st.
    pub fn now() -> Option<(u64, u64)> {
        let mut info = [0u64; 64];
        // SAFETY: `info` is writable and larger than `rusage_info_v4` (37 words), which flavor 4 fills in.
        let rc = unsafe { proc_pid_rusage(std::process::id() as i32, 4, info.as_mut_ptr()) };
        (rc == 0).then_some((info[2 + 29], info[2 + 30]))
    }
}

#[cfg(not(target_os = "macos"))]
mod counters {
    pub fn now() -> Option<(u64, u64)> {
        None
    }
}

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

/// A game set up as the frontier's searches are: from the frame builder, with the strict tactics.
fn setup<const W: usize>(n: u32) -> Prepared<W> {
    Board::<W>::for_n(n).prepare(tactics(), 1).expect("prepare")
}

/// The optimum's promotions that can be promoted from the root, one by one from the largest down.
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

/// A set as hex digits, digit `k` holding the members `4k..4k + 3` (lowest bit first), trailing zeros left out
/// (the empty set is "0").
fn to_hex<const W: usize>(set: &Bits<W>) -> String {
    let len = set.max().map_or(1, |x| x / 4 + 1);
    (0..len)
        .map(|k| {
            let nibble = (0..4).filter(|b| set.contains(4 * k + b)).fold(0, |acc, b| acc | 1 << b);
            char::from_digit(nibble, 16).expect("a nibble")
        })
        .collect()
}

fn from_hex<const W: usize>(text: &str) -> Result<Bits<W>, String> {
    let mut set = Bits::EMPTY;
    for (k, c) in text.chars().enumerate() {
        let nibble = c.to_digit(16).ok_or_else(|| format!("{c:?} is not a hex digit"))?;
        for b in (0..4).filter(|b| nibble >> b & 1 == 1) {
            let x = 4 * k as u32 + b;
            if x > Bits::<W>::MAX_VALUE {
                return Err(format!("{x} does not fit in Bits<{W}>"));
            }
            set.insert(x);
        }
    }
    Ok(set)
}

/// The nodes of a `.nodes` fixture: (board size, incumbent, units).
type Nodes<const W: usize> = (u32, Bits<W>, Vec<TaxUnit<W>>);

fn read_nodes<const W: usize>(path: &Path) -> Result<Nodes<W>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (mut n, mut incumbent, mut units) = (None, Bits::EMPTY, Vec::new());
    for (i, line) in text.lines().enumerate().filter(|(_, line)| !line.starts_with('#') && !line.is_empty()) {
        let at = |e: String| format!("{}: line {}: {e}", path.display(), i + 1);
        let fields: Vec<&str> = line.split(' ').collect();
        match fields[..] {
            ["n", value] => n = Some(value.parse().map_err(|e| at(format!("{e}")))?),
            ["incumbent", set] => incumbent = from_hex(set).map_err(at)?,
            ["node", promoted, cands] => {
                units.push(TaxUnit { promoted: from_hex(promoted).map_err(at)?, cands: from_hex(cands).map_err(at)? })
            }
            _ => return Err(at(format!("unexpected line {line:?}"))),
        }
    }
    Ok((n.ok_or_else(|| format!("{}: no board size", path.display()))?, incumbent, units))
}

// ---------------------------------------------------------------------------------------------------------
// F1 and F2: the kernels on the captured nodes

/// One captured node, rebuilt: its state, the heaviest matching's candidates and their minimal conflict.
struct Node<const W: usize> {
    promoted: Bits<W>,
    cands: Bits<W>,
    state: taxman::model::FrameSet<W>,
    need: u64,
    chosen: Bits<W>,
    conflict: Option<Bits<W>>,
}

fn kernels<const W: usize>(path: &Path, reps: u32, report: &mut Report) -> Result<(), String> {
    let (n, incumbent, units) = read_nodes::<W>(path)?;
    let p = setup::<W>(n);
    let model: &Model = &p.model;
    let graph = MatchGraph::new(model, &p.root);
    let mut scratch = Scratch::<W>::new();
    if !incumbent.is_subset(&p.candidates) || model.promote_all_using(&mut scratch, &p.root, &incumbent).is_none() {
        return Err(format!("{}: the incumbent is not a promotion set of game {n}", path.display()));
    }
    let best = incumbent.sum();
    let mut mb = MatchBound::new(&graph);
    let mut nodes = Vec::new();
    for u in units.iter().filter(|u| !u.cands.is_empty()) {
        let fits = u.promoted.is_subset(&p.candidates) && u.cands.is_subset(&p.candidates);
        let state = model.promote_all_using(&mut scratch, &p.root, &u.promoted).filter(|_| fits);
        let state = state.ok_or_else(|| format!("{}: a node that is not one of game {n}", path.display()))?;
        mb.reset(&u.promoted, &u.cands);
        let chosen = mb.chosen();
        let conflict = model.conflict_using(&mut scratch, &state, &chosen, true);
        let need = (best + 1).saturating_sub(u.promoted.sum());
        nodes.push(Node { promoted: u.promoted, cands: u.cands, state, need, chosen, conflict });
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
    let (reset, fp) = measure(reps, |fp| {
        for x in &nodes {
            mb.reset(&x.promoted, &x.cands);
            fp.add(mb.value());
        }
    });
    report.line("matching from scratch: reset(P, C) + value", nodes_len, reset, fp);
    let mut warm = MatchBound::new(&graph);
    warm.reset(&Bits::EMPTY, &p.candidates);
    warm.value();
    let (cost, fp) = measure(reps, |fp| {
        for x in &nodes {
            warm.push_all(&x.promoted, &x.cands);
            fp.add(warm.value());
            warm.pop();
        }
    });
    report.line("warm start: root level + push_all(P, C) + value", nodes_len, cost, fp);
    // The incremental kernels each start from a level solved from scratch: reported net of that.
    let net = |cost: Cost| Cost {
        cycles: cost.cycles.zip(reset.cycles).map(|(a, b)| a.saturating_sub(b)),
        instructions: cost.instructions.zip(reset.instructions).map(|(a, b)| a.saturating_sub(b)),
        wall: cost.wall.saturating_sub(reset.wall),
    };
    let (cost, fp) = measure(reps, |fp| {
        for x in &nodes {
            mb.reset(&x.promoted, &x.cands);
            mb.value();
            for c in x.chosen.iter() {
                fp.add(mb.bound_excluding(c));
            }
        }
    });
    report.line("+ bound_excluding(x), x in Q*", chosen, net(cost), fp);
    let (cost, fp) = measure(reps, |fp| {
        for x in &nodes {
            mb.reset(&x.promoted, &x.cands);
            mb.value();
            for c in x.conflict.unwrap_or_default().iter() {
                fp.add(mb.value_excluding(c));
                mb.pop();
            }
        }
    });
    report.line("+ value_excluding(x) + pop, x in the conflict", conflicts, net(cost), fp);
    let pushes = nodes.iter().map(|x| u64::from(x.chosen.len().min(8))).sum();
    let (cost, fp) = measure(reps, |fp| {
        for x in &nodes {
            mb.reset(&x.promoted, &x.cands);
            mb.value();
            for c in x.chosen.iter_desc().take(8) {
                mb.push(c, &x.cands.without(c));
                fp.add(mb.value());
                mb.pop();
            }
        }
    });
    report.line("+ push(x, C - x) + value + pop, 8 largest of Q*", pushes, net(cost), fp);

    let root_max = p.root.frames().iter().filter_map(|f| (f.moves | f.factors).max()).max().unwrap_or(0);
    let mut table = Nogoods::<W>::new(root_max, 1 << 16);
    for x in &nodes {
        if let Some(k) = x.conflict {
            if let Some(ng) = model.nogood(&mut scratch, &p.root, &x.promoted, &k) {
                table.insert(ng);
            }
        }
    }
    let forced: Vec<Bits<W>> = nodes
        .iter()
        .map(|x| {
            mb.reset(&x.promoted, &x.cands);
            mb.value();
            x.chosen.iter().filter(|&c| mb.bound_excluding(c) < x.need).collect()
        })
        .collect();
    let (cost, fp) = measure(reps, |fp| {
        for (x, forced) in nodes.iter().zip(&forced) {
            fp.add(match table.lookup(&x.promoted, &x.chosen, forced) {
                Lookup::None => 0,
                Lookup::AllForced => 1,
                Lookup::Conflict(k) => k.sum(),
            });
        }
    });
    report.line(&format!("Nogoods::lookup(P, Q*, forced), {} nogoods", table.len()), nodes_len, cost, fp);
    let (cost, fp) = measure(reps, |fp| {
        for x in &nodes {
            for c in x.chosen.iter_desc().take(8) {
                let added = Bits::EMPTY.with(c);
                fp.add(table.ruled_out(&added, &(x.promoted | added), &x.cands).sum());
            }
        }
    });
    report.line("Nogoods::ruled_out(x, P + x, C), 8 largest of Q*", pushes, cost, fp);

    // F2: the frame checks of promoting each candidate.
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
// F3: the search window

fn window<const W: usize>(n: u32, path: &Path, steps: u64, reps: u32, report: &mut Report) -> Result<(), String> {
    let p = Board::<W>::for_n(n)
        .prepare_for_resume(path, 1)
        .map_err(|e| format!("{}: {e} (make it again with `perf make-window`)", path.display()))?;
    let graph = MatchGraph::new(&p.model, &p.root);
    let space = SearchSpace::new(&p, Some(&graph), None);
    let header = engine::read_checkpoint_header(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let cfg = EngineConfig { threads: 1, node_limit: Some(steps), progress: None, ..EngineConfig::default() };
    let mut failure = None;
    let mut done = 0;
    let (cost, fp) = measure(reps.min(2), |fp| {
        // Without a checkpoint of its own the run leaves the file as it is.
        match engine::run(&cfg, Start::Resume(path.to_path_buf()), &header, |_| TaxSearcher::new(&space)) {
            Ok(o) => {
                done = o.stats.nodes;
                for x in [o.stats.nodes, o.best].into_iter().chain(o.stats.extra) {
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
    report.line(&format!("F3 window: {steps} steps from {name}"), steps, cost, fp);
    if !report.brief {
        println!("    (the checkpoint's search had {done} nodes after the window)");
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
    let graph = MatchGraph::new(&p.model, &p.root);
    let space = SearchSpace::new(&p, Some(&graph), None);
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
         # `perf make-nodes {n} --every {every} --keep {keep}`: its proof (one thread, strict tactics, seeded with\n\
         # the optimum) in {runs} runs of {every} nodes, each resuming the last one's checkpoint ({total} nodes in\n\
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

fn make_window<const W: usize>(dir: &Path, n: u32, optimal: &Optimal, at: u64) -> Result<(), String> {
    let p = setup::<W>(n);
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
            _ => numbers.push(parse(arg)?),
        }
    }
    let optimal = || SolveOptions::default().known_games(known.as_slice());
    match (command.as_deref(), &numbers[..]) {
        (Some("make-nodes"), &[n]) => {
            let n = n as u32;
            taxman::with_width!(n, W => make_nodes::<W>(&dir, n, &optimal()?, every, keep))
        }
        (Some("make-window"), &[n, at]) => {
            let n = n as u32;
            taxman::with_width!(n, W => make_window::<W>(&dir, n, &optimal()?, at))
        }
        (Some(command), _) => Err(format!("{command}: wrong arguments (see the top of examples/perf.rs)")),
        (None, games) => {
            let games: Vec<u32> =
                if games.is_empty() { GAMES.to_vec() } else { games.iter().map(|&n| n as u32).collect() };
            let mut report = Report { brief, ..Report::default() };
            for n in games {
                taxman::with_width!(n, W => {
                    kernels::<W>(&fixture(&dir, n, "nodes"), reps, &mut report)?;
                    window::<W>(n, &fixture(&dir, n, "window.ckpt"), steps, reps, &mut report)?;
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
