//! The gate trial: a search measures, on a sample of its own units, which gate of conflict branching makes the least
//! work, and branches with that gate from then on ([`Plan`]).
//!
//! The gate is the gap from which a node branches on the conflict that `conflict_branching` chooses.  Which gate is
//! best differs from game to game far more than within a game: on the units of game 5052 a gate of 600 took 0.68 of
//! the CPU of the gate of 1,200, on those of 4984 it took 1.55, and the spread of the open units' gaps stays the same
//! through most of a game (solver/docs/design/gate-trial.md).  So the trial asks the game
//! itself, once: at the first checkpoint written after the search has taken [`Plan::after`] steps, a thread beside the
//! search reads the checkpoint's units, picks [`Plan::units`] of them spread over their gaps, and solves each under
//! each gate of [`Plan::gates`] (and the gate the search has, if the list lacks it), on one thread, from the
//! checkpoint's incumbent, to at most [`Plan::steps`] steps.  The gate whose solves took the least CPU becomes the
//! search's gate, if it took at least [`MARGIN`] less than the gate the search had.
//!
//! The trial changes nothing but the gate.  Its solves are searches of their own, with their own searchers, nogood
//! tables and incumbent, and what they find or learn is dropped: they prune nothing of the game's search and offer it
//! nothing.  The gate only chooses which true conflict a node branches on, so the search stays exhaustive under any gate
//! and any change of it between units (solver/docs/design/search.md, "The invariant of a running search";
//! solver/docs/design/gate-trial.md).

use super::near_repeat::NearRepeat;
use super::{BranchSettings, Branching, NodeBound, Prepared, SearchSpace, TaxSearcher, TaxUnit};
use crate::engine::{self, CheckpointEvent, Control, Ctx, EngineConfig, Searcher, Start, Unit};
use crate::gf2::Gf2Graph;
use crate::model::CarriedNogoods;
use crate::rng::Rng;
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// The gates a trial tries, by default, besides the search's own (the default gate, [`super::CONFLICT_BRANCHING_GAP`],
/// unless a caller sets another): the gates of the chain's re-run.
pub const GATES: [u64; 3] = [1000, 800, 600];
/// The steps a search takes before its trial, by default: a few minutes of a search of 88 threads near n = 5000,
/// enough for a checkpoint of thousands of units.
pub const AFTER: u64 = 50_000_000;
/// The units a trial solves under each gate, by default: a test of 20 to 30 units picked the best gate of a game's 60
/// within 1 to 3% of its CPU.
pub const UNITS: usize = 24;
/// The most steps of each of a trial's solves, by default: about a minute of a thread near n = 5000.
pub const STEPS: u64 = 150_000;
/// The threads a trial solves on, by default.
pub const THREADS: usize = 4;
/// The least gap of a unit a trial picks, by default: the subtree of a unit of a smaller gap near n = 5000 is too
/// small to tell the gates apart.
pub const LEAST_GAP: u64 = 300;
/// The units of the checkpoint whose gap a trial computes, a uniform sample of them if there are more.
const SAMPLE: usize = 2_000;
/// How much less CPU a gate must take than the search's to replace it: less than this is within the noise of a test.
pub const MARGIN: f64 = 0.03;

/// A gate trial: which gates, when, and on how much.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// The gates tried, each from 0 to [`super::MAX_GATE`].
    pub gates: Vec<u64>,
    /// The trial takes the first checkpoint written at this many steps of the search or more (over all its runs).
    pub after: u64,
    /// How many units it solves under each gate.
    pub units: usize,
    /// The most steps of each solve.
    pub steps: u64,
    /// The threads the solves run on.
    pub threads: usize,
    /// The least gap of a unit it picks.
    pub least_gap: u64,
}

impl Default for Plan {
    /// A trial of [`GATES`], with the defaults for the rest.
    fn default() -> Self {
        Plan::of(GATES.to_vec())
    }
}

impl Plan {
    /// A trial of `gates`, with the defaults for the rest.
    pub fn of(gates: Vec<u64>) -> Self {
        Plan { gates, after: AFTER, units: UNITS, steps: STEPS, threads: THREADS, least_gap: LEAST_GAP }
    }

    /// This plan with the settings of `spec`, `NAME=VALUE` separated by commas, each name one of `after`, `units`,
    /// `steps`, `threads` and `least_gap`, each value a whole number (`units`, `steps` and `threads` at least 1).
    ///
    /// # Errors
    ///
    /// For another name, or a value that is not such a number.
    pub fn with(mut self, spec: &str) -> Result<Self, String> {
        for setting in spec.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let (name, value) = setting.split_once('=').ok_or(format!("{setting} is not NAME=VALUE"))?;
            let number: u64 = value.trim().parse().map_err(|_| format!("{setting}: not a whole number"))?;
            let positive = || (number > 0).then_some(number).ok_or(format!("{setting}: must be at least 1"));
            match name.trim() {
                "after" => self.after = number,
                "units" => self.units = positive()? as usize,
                "steps" => self.steps = positive()?,
                "threads" => self.threads = positive()? as usize,
                "least_gap" => self.least_gap = number,
                other => {
                    return Err(format!(
                        "{other}: not a setting of the gate trial (after, units, steps, threads, least_gap)"
                    ))
                }
            }
        }
        Ok(self)
    }
}

/// What the solves under one gate took.
#[derive(Clone, Debug, PartialEq)]
pub struct Score {
    pub gate: u64,
    /// The steps of its solves.
    pub steps: u64,
    /// Their CPU time, in seconds.
    pub cpu: f64,
    /// How many of them the step limit stopped.
    pub stopped: usize,
}

/// What a gate trial did.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    /// The search's steps at the checkpoint the trial took its units from.
    pub at_nodes: u64,
    /// The units solved under each gate.
    pub units: usize,
    /// The gate the search had when the trial began.
    pub from: u64,
    /// Each gate's solves, in the order tried.
    pub scores: Vec<Score>,
    /// The gate the trial chose: `from` unless another took [`MARGIN`] less CPU.
    pub chosen: u64,
    /// How long the trial took.
    pub seconds: f64,
}

/// What the trial needs of the search: its problem, its root's GF(2) graph, its carried nogoods, near-repeat cut and
/// forgetting period (which the trial's searchers share or copy), and its branching settings (which it reads and sets).
pub(super) struct Trial<'a, const W: usize> {
    pub plan: &'a Plan,
    pub prepared: &'a Prepared<W>,
    pub gf2: &'a Gf2Graph,
    pub branching: &'a Branching,
    pub near_repeat: Option<&'a OnceLock<NearRepeat<W>>>,
    pub carried: Option<&'a CarriedNogoods<W>>,
    pub forget_period: Option<u64>,
    /// Whether the trial's threads take the idle scheduling policy, as the search's workers do.
    pub background: bool,
    /// The head of its line on stderr (`n=5054`).
    pub label: &'a str,
}

/// Where the trial takes its checkpoint: the file, open, and the search's steps at it.
pub(super) type Taken = Receiver<(File, u64)>;

/// The trial's end of the checkpoints: the search's checkpoint hook hands it the first checkpoint written at
/// [`Plan::after`] steps or more, open, as the profile's summary takes its checkpoints.
pub(super) struct Checkpoints {
    after: u64,
    sender: Mutex<Option<Sender<(File, u64)>>>,
}

impl Checkpoints {
    pub(super) fn new(after: u64) -> (Arc<Self>, Taken) {
        let (sender, receiver) = mpsc::channel();
        (Arc::new(Checkpoints { after, sender: Mutex::new(Some(sender)) }), receiver)
    }

    /// The checkpoint hook's part: the first checkpoint written at `after` steps or more goes to the trial, once.  A
    /// file that cannot be opened leaves the trial waiting for the next.
    pub(super) fn event(&self, event: CheckpointEvent<'_>) {
        if let CheckpointEvent::Written { path, nodes, .. } = event {
            if nodes >= self.after {
                self.send(path, nodes);
            }
        }
    }

    fn send(&self, path: &Path, nodes: u64) {
        let mut sender = self.sender.lock().unwrap_or_else(PoisonError::into_inner);
        if let (Some(to), Ok(file)) = (sender.as_ref(), File::open(path)) {
            let _ = to.send((file, nodes));
            *sender = None;
        }
    }

    /// No checkpoint follows: a trial still waiting for one ends.
    fn close(&self) {
        self.sender.lock().unwrap_or_else(PoisonError::into_inner).take();
    }
}

/// Runs `search`, the engine's run, with the gate trial `trial` beside it if there is one: a thread of its own, which
/// waits for its checkpoint from `checkpoints` (whose sender is the search's checkpoint hook) and ends with the search,
/// the solves under way stopped.  Returns what `search` returns, and the trial's report if it ran to its end.
pub(super) fn run_beside<const W: usize, T>(
    trial: Option<(Trial<'_, W>, Arc<Checkpoints>, Taken)>,
    search: impl FnOnce() -> T,
) -> (T, Option<Report>) {
    let Some((trial, checkpoints, receiver)) = trial else { return (search(), None) };
    let (ending, solving) = (AtomicBool::new(false), Mutex::new(Vec::new()));
    thread::scope(|scope| {
        let (ending, solving, trial) = (&ending, &solving, &trial);
        let beside = scope.spawn(move || run(trial, &receiver, ending, solving));
        let found = search();
        // The end of the search: no checkpoint follows, and the solves under way stop.
        ending.store(true, Ordering::SeqCst);
        checkpoints.close();
        for control in lock(solving).iter() {
            control.stop();
        }
        (found, beside.join().expect("the gate trial's thread"))
    })
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// The trial: waits for its checkpoint, picks the units, solves them under each gate, and sets the gate that took the
/// least CPU.  `None` if the search ended first, or the checkpoint could not be read or held no unit to pick, or
/// conflict branching was off (the gate then says nothing).
fn run<const W: usize>(
    t: &Trial<'_, W>,
    receiver: &Taken,
    ending: &AtomicBool,
    solving: &Mutex<Vec<Arc<Control>>>,
) -> Option<Report> {
    let (file, at_nodes) = receiver.recv().ok()?;
    if t.background {
        engine::background_priority();
    }
    let began = Instant::now();
    let settings = t.branching.settings();
    if !settings.conflict_branching {
        return None;
    }
    // A uniform sample of the checkpoint's units (Algorithm R), and their gaps at its incumbent.
    let (mut sample, mut rng, mut seen) = (Vec::new(), Rng(at_nodes), 0usize);
    let head = engine::scan_checkpoint_file(file, |bytes| {
        let unit = TaxUnit::<W>::decode(bytes);
        seen += 1;
        if sample.len() < SAMPLE {
            sample.push(unit);
        } else if let Some(slot) = sample.get_mut(rng.below(seen)) {
            *slot = unit;
        }
        Ok(())
    })
    .ok()?;
    let mut bound = NodeBound::new(t.prepared, t.gf2);
    let mut gaps = Vec::new();
    for (i, unit) in sample.iter().enumerate() {
        if ending.load(Ordering::Relaxed) {
            return None;
        }
        if let Some(add) = bound.completions(unit) {
            gaps.push((i, add as i64 - (head.best + 1) as i64 + unit.promoted.sum() as i64));
        }
    }
    let picked: Vec<TaxUnit<W>> = pick(gaps, t.plan.units, t.plan.least_gap).into_iter().map(|i| sample[i]).collect();
    if picked.is_empty() {
        return None;
    }
    // The search's gate first, then the plan's others, each once.
    let mut gates = vec![settings.gate];
    for &gate in &t.plan.gates {
        if !gates.contains(&gate) {
            gates.push(gate);
        }
    }
    // The trial's own cut, a copy of the search's as it is now, so that the search's count of the nodes it cut is its own.
    let near = OnceLock::new();
    if let Some(cut) = t.near_repeat.and_then(OnceLock::get) {
        let _ = near.set(cut.copy());
    }
    let spaces: Vec<SearchSpace<'_, W>> = gates
        .iter()
        .map(|&gate| SearchSpace {
            forget_period: t.forget_period,
            near_repeat: Some(&near),
            carried: t.carried,
            branching: Arc::new(Branching::new(BranchSettings { gate, ..settings })),
            ..SearchSpace::new(t.prepared)
        })
        .collect();
    // Unit by unit, each under every gate, so that the solves of the gates see the machine alike.
    let jobs: Vec<(usize, usize)> = (0..picked.len()).flat_map(|u| (0..gates.len()).map(move |g| (u, g))).collect();
    let (next, solved) = (AtomicUsize::new(0), Mutex::new(vec![None; jobs.len()]));
    thread::scope(|scope| {
        for _ in 0..t.plan.threads.max(1) {
            scope.spawn(|| {
                if t.background {
                    engine::background_priority();
                }
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&(u, g)) = jobs.get(i) else { break };
                    let Some(done) = solve(&spaces[g], picked[u], &head, t.plan.steps, t.background, ending, solving)
                    else {
                        break;
                    };
                    lock(&solved)[i] = Some(done);
                }
            });
        }
    });
    let solved = solved.into_inner().unwrap_or_else(PoisonError::into_inner);
    if ending.load(Ordering::SeqCst) || solved.iter().any(Option::is_none) {
        return None;
    }
    let mut scores: Vec<Score> = gates.iter().map(|&gate| Score { gate, steps: 0, cpu: 0.0, stopped: 0 }).collect();
    for (&(_, g), done) in jobs.iter().zip(&solved) {
        let (steps, cpu, completed) = done.expect("every solve ran");
        scores[g].steps += steps;
        scores[g].cpu += cpu.as_secs_f64();
        scores[g].stopped += usize::from(!completed);
    }
    let chosen = choose(&scores, settings.gate);
    if chosen != settings.gate {
        t.branching.set_gate(chosen);
    }
    let report = Report {
        at_nodes,
        units: picked.len(),
        from: settings.gate,
        scores,
        chosen,
        seconds: began.elapsed().as_secs_f64(),
    };
    eprintln!("{}: {}", t.label, describe(&report));
    Some(report)
}

/// The trial's line on stderr.
fn describe(r: &Report) -> String {
    let scores: Vec<String> = r
        .scores
        .iter()
        .map(|s| format!("{} {:.1} s ({} steps, {} stopped)", s.gate, s.cpu, s.steps, s.stopped))
        .collect();
    let outcome = if r.chosen == r.from {
        format!("the gate stays {}", r.from)
    } else {
        format!("the gate is {} from now on, was {}", r.chosen, r.from)
    };
    format!(
        "gate trial at {} steps, {} units: CPU by gate {}; {} ({:.0} s)",
        r.at_nodes,
        r.units,
        scores.join(", "),
        outcome,
        r.seconds
    )
}

/// Solves `unit` with the searchers of `space` on one thread (under the idle scheduling policy with `background`), from
/// the incumbent of the checkpoint `head`, to at most `steps` steps: its steps, the CPU time of its thread, and whether
/// it completed.  `None` if the search ended.
fn solve<const W: usize>(
    space: &SearchSpace<'_, W>,
    unit: TaxUnit<W>,
    head: &engine::CheckpointHead,
    steps: u64,
    background: bool,
    ending: &AtomicBool,
    solving: &Mutex<Vec<Arc<Control>>>,
) -> Option<(u64, Duration, bool)> {
    let control = Control::new();
    lock(solving).push(Arc::clone(&control));
    // After the control is in the list: the end of the search either sees it there or is seen here.
    if ending.load(Ordering::SeqCst) {
        return None;
    }
    let cfg = EngineConfig {
        threads: 1,
        node_limit: Some(steps),
        control: Some(Arc::clone(&control)),
        background,
        ..EngineConfig::default()
    };
    let start = Start::Fresh { units: vec![unit], best: head.best, payload: head.payload.clone() };
    let cpu = AtomicU64::new(0);
    let outcome = engine::run(&cfg, start, b"gate trial", |_| Timed { inner: TaxSearcher::new(space), cpu: &cpu });
    lock(solving).retain(|c| !Arc::ptr_eq(c, &control));
    let outcome = outcome.ok()?;
    if ending.load(Ordering::SeqCst) {
        return None;
    }
    Some((outcome.stats.nodes, Duration::from_nanos(cpu.load(Ordering::Relaxed)), outcome.completed))
}

/// A searcher that adds the CPU time of its thread in each unit to `cpu`.
struct Timed<'a, 's, const W: usize> {
    inner: TaxSearcher<'s, W>,
    cpu: &'a AtomicU64,
}

impl<const W: usize> Searcher<TaxUnit<W>> for Timed<'_, '_, W> {
    fn run(&mut self, unit: TaxUnit<W>, ctx: &mut Ctx<'_, TaxUnit<W>>) {
        let began = thread_cpu_ns();
        self.inner.run(unit, ctx);
        self.cpu.fetch_add(thread_cpu_ns().saturating_sub(began), Ordering::Relaxed);
    }
}

/// Of the units `gaps` (an index and a gap each), `want` of a gap of at least `least` at evenly spaced ranks of the
/// gap, largest first: the middle unit of each of `want` equal slices of them in the order of their gaps, so that the
/// picks follow the units' own mix of gaps (most where most units are) and still reach the largest.  All of them if
/// there are no more than `want`.
fn pick(mut gaps: Vec<(usize, i64)>, want: usize, least: u64) -> Vec<usize> {
    gaps.retain(|&(_, gap)| gap >= least as i64);
    gaps.sort_unstable_by_key(|&(i, gap)| (gap, i));
    let n = gaps.len();
    if n <= want {
        return gaps.iter().rev().map(|&(i, _)| i).collect();
    }
    (0..want).rev().map(|k| gaps[(2 * k + 1) * n / (2 * want)].0).collect()
}

/// The gate whose solves took the least CPU, if it took at least [`MARGIN`] less than `from`'s; else `from`.
fn choose(scores: &[Score], from: u64) -> u64 {
    let Some(base) = scores.iter().find(|s| s.gate == from) else { return from };
    match scores.iter().min_by(|a, b| a.cpu.total_cmp(&b.cpu)) {
        Some(best) if best.cpu <= (1.0 - MARGIN) * base.cpu => best.gate,
        _ => from,
    }
}

/// The CPU time this thread has used so far, in nanoseconds (`clock_gettime` of `CLOCK_THREAD_CPUTIME_ID`).
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn thread_cpu_ns() -> u64 {
    #[repr(C)]
    struct Timespec {
        seconds: i64,
        nanoseconds: i64,
    }
    unsafe extern "C" {
        fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
    }
    // CLOCK_THREAD_CPUTIME_ID: 3 on Linux, 16 on macOS.
    let clock = if cfg!(target_os = "linux") { 3 } else { 16 };
    let mut time = Timespec { seconds: 0, nanoseconds: 0 };
    // SAFETY: `time` is writable and has the layout of `struct timespec` (two `long`s) on 64-bit Linux and macOS.
    let rc = unsafe { clock_gettime(clock, &mut time) };
    assert_eq!(rc, 0, "clock_gettime failed");
    (time.seconds as u64) * 1_000_000_000 + time.nanoseconds as u64
}

/// Without a clock for a thread's CPU time, the time since the first call.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn thread_cpu_ns() -> u64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::search::{search, search_steered, SearchTactics};
    use crate::test_support::TempDir;

    const W: usize = 16;

    fn score(gate: u64, cpu: f64) -> Score {
        Score { gate, steps: 0, cpu, stopped: 0 }
    }

    /// Game `n` with the default tactics (conflict branching on, at the gate of `CONFLICT_BRANCHING_GAP`).
    fn game(n: u32) -> Prepared<W> {
        Board::<W>::for_n(n).prepare(SearchTactics::default()).expect("the setup")
    }

    #[test]
    fn a_trial_solves_its_units_under_each_gate_and_sets_the_cheapest() {
        // Game 800's search, about 1,400 steps, stopped at its first check of the node limit (1,024 steps), leaves its
        // units in its checkpoint; the two that the trial picks take under 100 steps each under either gate, within the
        // plan's 300.
        let p = game(800);
        let dir = TempDir::new("gate-trial");
        let path = dir.join("n800.ckpt");
        let every = Duration::from_secs(3600);
        let cfg = EngineConfig {
            threads: 1,
            node_limit: Some(1),
            checkpoint: Some((path.clone(), every)),
            ..Default::default()
        };
        let stopped = search(&p, &cfg, None, None, false).expect("the search");
        assert!(!stopped.completed);
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        // The search starts with the gate of the run (`--gate`), which the trial compares the others with.
        let branching = Branching::new(BranchSettings::of(&p.tactics).with_gate(Some(1200)));
        let from = branching.settings().gate;
        assert_eq!(from, 1200);
        let plan = Plan { gates: vec![0, from], after: 0, units: 2, steps: 300, threads: 2, least_gap: 0 };
        let trial = Trial {
            plan: &plan,
            prepared: &p,
            gf2: &graph,
            branching: &branching,
            near_repeat: None,
            carried: None,
            forget_period: p.tactics.forget_period(),
            background: false,
            label: "n=800",
        };
        let (checkpoints, taken) = Checkpoints::new(plan.after);
        checkpoints.event(CheckpointEvent::Written {
            path: &path,
            nodes: stopped.stats.nodes,
            elapsed: Duration::ZERO,
        });
        let report = run(&trial, &taken, &AtomicBool::new(false), &Mutex::new(Vec::new())).expect("the trial's report");
        // The search's gate first, each gate once; the steps of each gate are the same every time (one thread each).
        assert_eq!(report.scores.iter().map(|s| s.gate).collect::<Vec<_>>(), vec![from, 0]);
        assert_eq!((report.at_nodes, report.units, report.from), (stopped.stats.nodes, 2, from));
        assert!(report.scores.iter().all(|s| s.steps > 0 && s.cpu > 0.0), "{report:?}");
        assert_eq!(branching.settings().gate, report.chosen);
        assert_eq!(report.chosen, choose(&report.scores, from));
    }

    #[test]
    fn a_search_with_a_trial_finds_what_it_finds_without_and_ends_with_it() {
        // Game 800's search, about 1,400 steps, with a checkpoint every 50 ms: the trial takes the first, and the end
        // of the search stops it wherever it is.
        let p = game(800);
        let dir = TempDir::new("gate-trial-search");
        let cfg = EngineConfig {
            threads: 2,
            checkpoint: Some((dir.join("n800.ckpt"), Duration::from_millis(50))),
            ..EngineConfig::default()
        };
        let plan = Plan { gates: vec![0], after: 0, units: 4, steps: 2000, threads: 1, least_gap: 0 };
        let branching = Arc::new(Branching::new(BranchSettings::of(&p.tactics)));
        let near = OnceLock::new();
        let with = search_steered(&p, &cfg, &[], None, false, None, &branching, &near, None, None, Some(&plan));
        let with = with.expect("the search with a trial");
        let without = search(&p, &EngineConfig { threads: 1, ..EngineConfig::default() }, None, None, false);
        assert!(with.completed);
        assert_eq!(with.sum, without.expect("the search").sum);
        if let Some(report) = &with.gate_trial {
            assert_eq!(branching.settings().gate, report.chosen);
        }
    }

    #[test]
    fn the_picks_follow_the_gaps_and_reach_the_largest() {
        // Units 0..100 of gaps 0, 10, ..., 990: those of gap 300 or more are 30..100, seventy of them.
        let gaps: Vec<(usize, i64)> = (0..100).map(|i| (i, 10 * i as i64)).collect();
        assert_eq!(pick(gaps.clone(), 7, 300), vec![95, 85, 75, 65, 55, 45, 35]);
        assert_eq!(pick(gaps.clone(), 70, 300).len(), 70);
        assert_eq!(pick(gaps.clone(), 1000, 300), (30..100).rev().collect::<Vec<_>>());
        assert_eq!(pick(gaps, 2, 0), vec![75, 25]);
        assert!(pick(vec![(0, 299), (1, -5)], 4, 300).is_empty());
    }

    #[test]
    fn a_plan_takes_its_settings_by_name() {
        let plan = Plan::of(vec![1200, 600]).with("after=0, units=3,steps=500,threads=2,least_gap=0").unwrap();
        let want = Plan { gates: vec![1200, 600], after: 0, units: 3, steps: 500, threads: 2, least_gap: 0 };
        assert_eq!(plan, want);
        assert_eq!(Plan::of(vec![600]).with("").unwrap(), Plan::of(vec![600]));
        for bad in ["units=0", "steps=x", "after", "gate=600"] {
            assert!(Plan::of(vec![600]).with(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_gate_must_save_the_margin_to_be_chosen() {
        let scores = [score(1200, 100.0), score(800, 98.0), score(600, 96.0)];
        assert_eq!(choose(&scores, 1200), 600);
        let scores = [score(1200, 100.0), score(800, 98.0), score(600, 97.5)];
        assert_eq!(choose(&scores, 1200), 1200);
        let scores = [score(1200, 100.0), score(600, 140.0)];
        assert_eq!(choose(&scores, 1200), 1200);
        assert_eq!(choose(&scores, 600), 1200);
    }
}
