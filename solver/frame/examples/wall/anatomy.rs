//! `wall anatomy`: a hard subtree taken apart.  Units of the tuning suite are solved as `tune solve` solves them (one
//! thread, fresh tables, from the known optimum, so that the subtree is a proof), with a hook on every step of the
//! searcher (`search::StepHook`), which writes what each step saw and did; a script of the notebook entry reads it.
//!
//! ```text
//! cargo run --profile test --features studies --example wall -- anatomy --suite FILE --known FILE --ids ID[,ID...]
//!     [--trace DIR] [--prefer FILE] [--best-cpu S] [--nodes K] [--tactic NAME=VALUE]... N
//! ```
//!
//! `N` is the game of the units, `--ids` the suite's ids of them (`N-INDEX`).  Each unit has a line on stdout: `id`,
//! `depth` (its promotions), `cands`, `need` (what a completion must add to beat P*), `nodes` (the engine's
//! steps), `steps` (those the hook was told of: the same), `cpu_s` (the thread's), `completed`, `best` (the best
//! promotion sum; P* unless the subtree beat it), `root_value` and `root_chosen` (the GF(2) bound of the
//! unit's first step and the size of its heaviest set), `conflicts` (the distinct conflicts the steps' checks found),
//! `residues` (the distinct residues of each frame and of the whole node, then the distinct states of each frame and
//! of all of them, below), `asked` and `overridden` (how often
//! the hook was asked which member to branch on, and how often its answer was not strong branching's), `tactics` (the
//! changes from the default tactics), `prefer` (the order's file, or null), and with
//! `--best-cpu` the best completion found (below).
//!
//! **The trace** (`--trace DIR`): for unit `ID`, `DIR/ID.steps.tsv` has a line a step, in the order of the steps,
//! under a header: `d` (the node's depth on the stack below the unit's node), `np` (its promotions), `nc` (the
//! candidates its loop had), `need`; the node's own bound check, if the step made one (`nv` the bound, `nk` the size of
//! its heaviest set, `kid` the id of its conflict, `kc` 1 if that conflict was computed and learned rather than known),
//! or `inh` 1 if the verdict was the one the node's check as a child left; the branch (`br` the member, 0 if none,
//! `lf` its live factors at the node: its maximal factors that are factors of the frame it would move into, `nadd` and
//! `sadd` the size and sum of what the child promotes, `nf` the forced members); the child's check (`cv`, `ckid`,
//! `ckc`; -1 where there is none); `end` (`P` the child pushed, `E` a child with no candidates, `F` the promotion
//! failed, `C<r>` the child cut and `L<r>` the loop ended, `r` the `search::cut` reason); `gain` and `loss`, what the
//! node's heaviest set gained and lost since the node's last check (members, comma separated, or `-`; only at a step
//! after the node's first: after it went on without a member, the substitutes for it); and the residues `r0`, `r1`,
//! ... and `rall`, and the states `s0`, `s1`, ... and `sall` (ids).  A frame's *residue* at a step is its state
//! (moves, factors, freedoms) and the node's candidates whose promotion touches it (those of home `f` and `f + 1`);
//! `rall` is the state of all frames and all the candidates, which is the node's whole residual problem but for the sum
//! it has; `s0`, ... are the frames' states alone and `sall` all of them.  Ids number the distinct values in order of
//! appearance.  `DIR/ID.conflicts.tsv` has the conflicts (`id`, `size`, `members` ascending, comma
//! separated), and `DIR/N.candidates.tsv` the game's candidates (`x`, `home`, `factors`: its maximal factors).
//!
//! **The branching order** (`--prefer FILE`): a list of numbers (whitespace separated), the first the most preferred.
//! Where strong branching found no member of a node's conflict forced, the node branches on the member of the conflict
//! that comes first in the list, and on strong branching's choice if none is in it.  Any member keeps the search exact
//! (`search::StepHook::branch`), so the best is the same and only the tree changes.  `--nodes K` stops a unit after
//! about K steps (`completed` false).
//!
//! **The best completion** (`--best-cpu S`): large neighbourhood search below the unit's node for S CPU seconds (`lns`,
//! its units restricted to the node's candidates, from the greedy completion of the node), not sound; the line has
//! `best_completion` (the sum of what it found beyond the node's promotions), its members (`completion`), the heaviest
//! set of the unit's first step (`chosen`) with each member's home and live factors at the node (`chosen_lf`, as
//! `[x, home, live]`), and that set's minimal conflicts at the node, found one after the other with each one's
//! members dropped before the next (`cores`).

use super::{beyond, Line, Options};
use crate::counters::cpu_seconds;
use crate::fixtures::from_hex;
use crate::known::known_incumbent;
use crate::lns::run_unit_with;
use crate::rng::Rng;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::Control;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::matchbound::MatchGraph;
use taxman::model::{FrameGraph, FrameSet, Scratch};
use taxman::search::{MakeHook, Prepared, SearchSpace, StepEnd, StepHook, StepRecord, TaxSearcher, TaxUnit};
use taxman::tactics::Tactics;

/// The options of `anatomy`.
#[derive(Clone, Default)]
pub struct Params {
    /// The units' ids (`--ids`), the directory of the traces (`--trace`), the branching order (`--prefer`) and the
    /// CPU seconds of the search for the best completion (`--best-cpu`; 0: none).
    pub ids: Vec<String>,
    pub trace: Option<PathBuf>,
    pub prefer: Option<PathBuf>,
    pub best_cpu: f64,
}

// ---------------------------------------------------------------------------------------------------------
// The hook

/// What every searcher's hook knows of the game: its frames, the candidates that touch each, and the order to
/// branch in.
struct Known<const W: usize> {
    /// For each frame, the candidates whose promotion touches it (of home `f`, out of it, and `f + 1`, into it).
    touching: Vec<Bits<W>>,
    /// For each number up to the largest candidate: its home frame, the frame it moves into and its maximal factors
    /// (empty for a number that is no candidate).
    home: Vec<usize>,
    into: Vec<usize>,
    factors: Vec<Vec<u32>>,
    /// For each number, its place in the preferred order (`u32::MAX`: not in it); empty without one.
    rank: Vec<u32>,
}

impl<const W: usize> Known<W> {
    fn new(p: &Prepared<W>, prefer: &[u32]) -> Known<W> {
        let model = &p.model;
        let frames = p.root.frames().len();
        let top = p.candidates.max().unwrap_or(0) as usize;
        let (mut home, mut into_frame, mut factors) = (vec![0; top + 1], vec![0; top + 1], vec![Vec::new(); top + 1]);
        let mut touching = vec![Bits::EMPTY; frames];
        for x in p.candidates.iter() {
            let (into, out) = model.frames_touched(x);
            home[x as usize] = out.unwrap_or(into);
            into_frame[x as usize] = into;
            factors[x as usize] = model.graph().factors(x).iter().map(|&y| u32::from(y)).collect();
            for f in [Some(into), out].into_iter().flatten().filter(|&f| f < frames) {
                touching[f].insert(x);
            }
        }
        let mut rank = Vec::new();
        if !prefer.is_empty() {
            rank = vec![u32::MAX; top + 1];
            for (i, &x) in prefer.iter().enumerate() {
                if let Some(slot) = rank.get_mut(x as usize).filter(|slot| **slot == u32::MAX) {
                    *slot = i as u32;
                }
            }
        }
        Known { touching, home, into: into_frame, factors, rank }
    }

    /// The maximal factors of the candidate `x` that are factors of the frame it moves into, in `state`.
    fn live_factors(&self, state: &FrameSet<W>, x: u32) -> usize {
        let factors = &state.frames()[self.into[x as usize]].factors;
        self.factors[x as usize].iter().filter(|&&y| factors.contains(y)).count()
    }
}

/// What the hooks of a unit's solve gathered, and where they write the steps.
struct Tally<const W: usize> {
    out: Option<BufWriter<File>>,
    steps: u64,
    /// The node's depth on the stack for the next step.
    depth: usize,
    conflicts: HashMap<Bits<W>, u32>,
    listed: Vec<Bits<W>>,
    /// The distinct residues of each frame and of the whole node, then the distinct states of each frame and of all,
    /// by their hash.
    residues: Vec<HashMap<u64, u32>>,
    root: Option<(u64, Bits<W>)>,
    /// The heaviest set of the node at each depth of the stack, as its last check found it.
    heaviest: Vec<Option<Bits<W>>>,
    asked: u64,
    overridden: u64,
    error: Option<String>,
}

impl<const W: usize> Tally<W> {
    fn new(frames: usize, out: Option<BufWriter<File>>) -> Tally<W> {
        Tally {
            out,
            steps: 0,
            depth: 0,
            conflicts: HashMap::new(),
            listed: Vec::new(),
            residues: vec![HashMap::new(); 2 * (frames + 1)],
            root: None,
            heaviest: Vec::new(),
            asked: 0,
            overridden: 0,
            error: None,
        }
    }

    fn conflict(&mut self, k: Option<Bits<W>>) -> i64 {
        let Some(k) = k else { return -1 };
        let next = self.listed.len() as u32;
        let id = *self.conflicts.entry(k).or_insert(next);
        if id == next {
            self.listed.push(k);
        }
        i64::from(id)
    }

    fn residue(&mut self, slot: usize, hash: u64) -> u32 {
        let map = &mut self.residues[slot];
        let next = map.len() as u32;
        *map.entry(hash).or_insert(next)
    }
}

/// A 64-bit hash of `value` (SipHash with fixed keys: the same in every run).
fn hash_of(value: &impl Hash) -> u64 {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}

/// The hook of one searcher: everything goes to the tally of the unit being solved (one thread searches a unit, so
/// the lock is never contended).  A searcher made while no unit is (the search for the best completion) is told
/// nothing and branches as strong branching does.
struct Recorder<const W: usize> {
    known: Arc<Known<W>>,
    tally: Option<Arc<Mutex<Tally<W>>>>,
}

/// The hooks of a game's searchers ([`Recorder`]), and the tally of the unit being solved.
struct Hooks<const W: usize> {
    known: Arc<Known<W>>,
    current: Arc<Mutex<Option<Arc<Mutex<Tally<W>>>>>>,
}

impl<const W: usize> Hooks<W> {
    fn new(known: Known<W>) -> Hooks<W> {
        Hooks { known: Arc::new(known), current: Arc::default() }
    }

    /// What makes each searcher's hook ([`SearchSpace::hooks`]).
    fn maker(&self) -> impl Fn() -> Box<dyn StepHook<W>> + Sync + 'static {
        let (known, current) = (Arc::clone(&self.known), Arc::clone(&self.current));
        move || {
            let tally = current.lock().unwrap_or_else(PoisonError::into_inner).clone();
            Box::new(Recorder { known: Arc::clone(&known), tally })
        }
    }
}

const HEADER: &str = "d\tnp\tnc\tneed\tnv\tnk\tkid\tkc\tinh\tbr\tlf\tnadd\tsadd\tnf\tcv\tckid\tckc\tend\tgain\tloss";

fn end_code(end: StepEnd) -> String {
    match end {
        StepEnd::Child => "P".into(),
        StepEnd::Leaf => "E".into(),
        StepEnd::ChildFailed => "F".into(),
        StepEnd::ChildCut(r) => format!("C{r}"),
        StepEnd::LoopEnd(r) => format!("L{r}"),
    }
}

impl<const W: usize> StepHook<W> for Recorder<W> {
    fn step(&mut self, r: &StepRecord<W>, state: &FrameSet<W>, _child: Option<&FrameSet<W>>) {
        let Some(tally) = &self.tally else { return };
        let known = &*self.known;
        let mut t = tally.lock().unwrap_or_else(PoisonError::into_inner);
        let t = &mut *t;
        t.steps += 1;
        let d = t.depth;
        t.depth = match r.end {
            StepEnd::Child => d + 1,
            StepEnd::LoopEnd(_) => d.saturating_sub(1),
            _ => d,
        };
        if t.root.is_none() {
            t.root = r.node.map(|c| (c.value, c.chosen));
        }
        let node = r.node;
        let kid = t.conflict(node.and_then(|c| c.conflict));
        let ckid = t.conflict(r.child.and_then(|c| c.conflict));
        let frames = state.frames();
        let mut residues = Vec::with_capacity(2 * (frames.len() + 1));
        for (f, frame) in frames.iter().enumerate() {
            let touching = known.touching.get(f).copied().unwrap_or_default();
            residues.push(t.residue(f, hash_of(&(frame, r.cands & touching))));
        }
        residues.push(t.residue(frames.len(), hash_of(&(frames, r.cands))));
        let states = frames.len() + 1;
        for (f, frame) in frames.iter().enumerate() {
            residues.push(t.residue(states + f, hash_of(frame)));
        }
        residues.push(t.residue(states + frames.len(), hash_of(&frames)));
        // What the node's heaviest set gained and lost since its last check (at a step after the first: the members
        // that replaced the one its last step branched on, if it went on without it).
        let first = r.inherited || t.steps == 1;
        if t.heaviest.len() < d + 2 {
            t.heaviest.resize(d + 2, None);
        }
        let now = node.map(|c| c.chosen);
        let swap = match (first, t.heaviest[d], now) {
            (false, Some(before), Some(now)) => Some((now - before, before - now)),
            _ => None,
        };
        if now.is_some() {
            t.heaviest[d] = now;
        }
        if r.end == StepEnd::Child {
            t.heaviest[d + 1] = r.child.map(|c| c.chosen);
        }
        let Some(out) = &mut t.out else { return };
        let lf = r.branch.map_or(-1, |x| known.live_factors(state, x) as i64);
        let members = |set: Option<Bits<W>>| match set.filter(|set| !set.is_empty()) {
            Some(set) => set.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","),
            None => "-".to_owned(),
        };
        let value = |c: Option<u64>| c.map_or(-1, |v| v as i64);
        let line = format!(
            "{d}\t{}\t{}\t{}\t{}\t{}\t{kid}\t{}\t{}\t{}\t{lf}\t{}\t{}\t{}\t{}\t{ckid}\t{}\t{}\t{}\t{}\t{}\n",
            r.promoted.len(),
            r.cands.len(),
            r.need,
            value(node.map(|c| c.value)),
            node.map_or(-1, |c| i64::from(c.chosen.len())),
            u8::from(node.is_some_and(|c| c.computed)),
            u8::from(r.inherited),
            r.branch.unwrap_or(0),
            r.added.len(),
            r.added.sum(),
            r.forced.len(),
            value(r.child.map(|c| c.value)),
            u8::from(r.child.is_some_and(|c| c.computed)),
            end_code(r.end),
            members(swap.map(|s| s.0)),
            members(swap.map(|s| s.1)),
            residues.iter().map(u32::to_string).collect::<Vec<_>>().join("\t"),
        );
        if let Err(e) = out.write_all(line.as_bytes()) {
            t.error.get_or_insert(format!("the trace: {e}"));
        }
    }

    fn branch(&mut self, conflict: &Bits<W>, strong: u32) -> u32 {
        let Some(tally) = &self.tally else { return strong };
        let rank = &self.known.rank;
        let mut t = tally.lock().unwrap_or_else(PoisonError::into_inner);
        t.asked += 1;
        if rank.is_empty() {
            return strong;
        }
        let place = |x: u32| rank.get(x as usize).copied().unwrap_or(u32::MAX);
        let first = conflict.iter().min_by_key(|&x| place(x)).filter(|&x| place(x) != u32::MAX);
        let chosen = first.unwrap_or(strong);
        t.overridden += u64::from(chosen != strong);
        chosen
    }
}

// ---------------------------------------------------------------------------------------------------------
// The units

/// The units `ids` of game `n` in the suite at `path`, each with its id and depth.
pub(super) fn suite_units<const W: usize>(
    path: &Path,
    n: u32,
    ids: &[String],
) -> Result<Vec<(String, TaxUnit<W>)>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut found = HashMap::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let json = Json::parse(line).map_err(|e| format!("{}: {e:?}", path.display()))?;
        let field = |key: &str| json.get(key).and_then(Json::as_str).unwrap_or_default();
        if field("kind") == "unit" && json.get("game").and_then(Json::as_u32) == Some(n) {
            let unit = TaxUnit { promoted: from_hex(field("promoted"))?, cands: from_hex(field("cands"))? };
            found.insert(field("id").to_owned(), unit);
        }
    }
    ids.iter()
        .map(|id| {
            found.get(id).map(|&unit| (id.clone(), unit)).ok_or_else(|| format!("no unit {id} of {n} in the suite"))
        })
        .collect()
}

/// The numbers of the file at `path`, in order.
fn read_order(path: &Path) -> Result<Vec<u32>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    text.split_whitespace().map(|word| word.parse().map_err(|e| format!("{}: {word}: {e}", path.display()))).collect()
}

/// What solving a unit with the hook gave.
struct Solved<const W: usize> {
    nodes: u64,
    cpu_s: f64,
    completed: bool,
    best: u64,
    tally: Tally<W>,
}

/// Solves `unit` on one thread from the incumbent `start`, with fresh tables, as `tune solve` does, each step told to
/// the [`Recorder`] of `space`'s searcher (made by `hooks`), which writes to `out` (if any).
fn solve<const W: usize>(
    space: &SearchSpace<'_, W>,
    hooks: &Hooks<W>,
    unit: TaxUnit<W>,
    start: &Bits<W>,
    node_limit: Option<u64>,
    out: Option<BufWriter<File>>,
) -> Result<Solved<W>, String> {
    let tally = Arc::new(Mutex::new(Tally::new(space.root.frames().len(), out)));
    *hooks.current.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(&tally));
    let before = cpu_seconds();
    let ran = run_unit_with(&Control::new(), unit, start, node_limit, |_| TaxSearcher::new(space));
    let cpu_s = before.zip(cpu_seconds()).map_or(0.0, |(a, b)| b - a);
    *hooks.current.lock().unwrap_or_else(PoisonError::into_inner) = None;
    let mut tally = Arc::try_unwrap(tally)
        .map_err(|_| "the hooks outlived the search".to_owned())?
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some(out) = &mut tally.out {
        out.flush().map_err(|e| format!("the trace: {e}"))?;
    }
    if let Some(e) = tally.error.take() {
        return Err(e);
    }
    Ok(Solved { nodes: ran.nodes, cpu_s, completed: ran.completed, best: ran.set.sum(), tally })
}

// ---------------------------------------------------------------------------------------------------------
// The best completion and the cores

/// The best completion of the node of `unit` that large neighbourhood search finds in `cpu` CPU seconds of this thread:
/// from the greedy completion (the node's candidates promoted from the largest down where they can be), units that fix
/// the node's promotions and a random 70 to 90% of the completion's, searching the rest of the node's candidates for at
/// most 1,024 steps.  Returns the completion (without the node's promotions).
fn best_completion<const W: usize>(space: &SearchSpace<'_, W>, unit: &TaxUnit<W>, cpu: f64, n: u32) -> Bits<W> {
    let model = space.model;
    let mut scratch = Scratch::new();
    let mut state = model.promote_all_using(&mut scratch, space.root, &unit.promoted).expect("the unit's promotions");
    let mut best = unit.promoted;
    let mut cands = unit.cands.to_vec();
    cands.reverse();
    for x in cands {
        let before = state;
        if model.promote_in_place_using(&mut scratch, &mut state, x) && state.frames().iter().all(|f| f.freedoms >= 0) {
            best.insert(x);
        } else {
            state = before;
        }
    }
    let mut rng = Rng(0xA4A7_0000 ^ u64::from(n));
    let began = cpu_seconds().unwrap_or(0.0);
    let control = Control::new();
    while cpu_seconds().unwrap_or(f64::INFINITY) - began < cpu {
        let mut members = (best - unit.promoted).to_vec();
        rng.shuffle(&mut members);
        let share = 0.7 + 0.2 * rng.unit();
        let fixed: Bits<W> = members[..(members.len() as f64 * share).round() as usize].iter().copied().collect();
        let unit = TaxUnit { promoted: unit.promoted | fixed, cands: unit.cands - fixed };
        let ran = run_unit_with(&control, unit, &best, Some(1024), |_| TaxSearcher::new(space));
        if ran.set.sum() > best.sum() {
            best = ran.set;
        }
    }
    best - unit.promoted
}

/// The minimal conflicts of `set` at `state`, one after the other, each one's members dropped before the next.
fn cores<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, set: &Bits<W>) -> Vec<Bits<W>> {
    let mut scratch = Scratch::new();
    let (mut left, mut found) = (*set, Vec::new());
    while let Some(k) = model.conflict_using(&mut scratch, state, &left, true) {
        if k.is_empty() {
            break;
        }
        left -= k;
        found.push(k);
    }
    found
}

// ---------------------------------------------------------------------------------------------------------
// The command

fn create(path: &Path) -> Result<BufWriter<File>, String> {
    File::create(path).map(BufWriter::new).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes the game's candidates: each with its home frame and its maximal factors.
fn write_candidates<const W: usize>(path: &Path, p: &Prepared<W>, known: &Known<W>) -> Result<(), String> {
    let mut out = create(path)?;
    let mut text = String::from("x\thome\tfactors\n");
    for x in p.candidates.iter() {
        let factors: Vec<String> = known.factors[x as usize].iter().map(u32::to_string).collect();
        text.push_str(&format!("{x}\t{}\t{}\n", known.home[x as usize], factors.join(",")));
    }
    out.write_all(text.as_bytes()).and_then(|()| out.flush()).map_err(|e| format!("{}: {e}", path.display()))
}

/// Writes the game's candidates as the trace has them, for another study (`near`).
pub(super) fn write_game_candidates<const W: usize>(path: &Path, p: &Prepared<W>) -> Result<(), String> {
    write_candidates(path, p, &Known::new(p, &[]))
}

fn write_conflicts<const W: usize>(path: &Path, listed: &[Bits<W>]) -> Result<(), String> {
    let mut out = create(path)?;
    let mut text = String::from("id\tsize\tmembers\n");
    for (id, k) in listed.iter().enumerate() {
        let members: Vec<String> = k.iter().map(|x| x.to_string()).collect();
        text.push_str(&format!("{id}\t{}\t{}\n", k.len(), members.join(",")));
    }
    out.write_all(text.as_bytes()).and_then(|()| out.flush()).map_err(|e| format!("{}: {e}", path.display()))
}

/// `anatomy` on game `n` at the set width `W`.
fn play<const W: usize>(n: u32, o: &Options, optimal: &Optimal) -> Result<(), String> {
    let params = &o.anatomy;
    let suite = o.params.suite.as_ref().ok_or("anatomy needs --suite")?;
    let units = suite_units::<W>(suite, n, &params.ids)?;
    let board = Board::<W>::for_n(n);
    let p = board.prepare(o.tactics)?;
    let (start, opt) =
        known_incumbent(&board, &p, optimal)?.ok_or(format!("n={n}: needs its known optimum (--known)"))?;
    let prefer = params.prefer.as_deref().map(read_order).transpose()?.unwrap_or_default();
    let hooks = Hooks::new(Known::new(&p, &prefer));
    let make = hooks.maker();
    let graph = MatchGraph::new(&p.model, &p.root);
    let space = SearchSpace { hooks: Some(&make as &MakeHook<W>), ..SearchSpace::new(&p, &graph) };
    if let Some(dir) = &params.trace {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        write_candidates(&dir.join(format!("{n}.candidates.tsv")), &p, &hooks.known)?;
    }
    for (id, unit) in units {
        let out = match &params.trace {
            Some(dir) => {
                let mut out = create(&dir.join(format!("{id}.steps.tsv")))?;
                let frames = |prefix: &str| -> Vec<String> {
                    (0..p.root.frames().len()).map(|f| format!("{prefix}{f}")).collect()
                };
                let (r, s) = (frames("r").join("\t"), frames("s").join("\t"));
                writeln!(out, "{HEADER}\t{r}\trall\t{s}\tsall").map_err(|e| format!("the trace: {e}"))?;
                Some(out)
            }
            None => None,
        };
        let solved = solve(&space, &hooks, unit, &start, o.nodes, out)?;
        let t = &solved.tally;
        if let Some(dir) = &params.trace {
            write_conflicts(&dir.join(format!("{id}.conflicts.tsv")), &t.listed)?;
        }
        let mut line = Line::of(n);
        line.text("id", &id)
            .field("depth", unit.promoted.len())
            .field("cands", unit.cands.len())
            .field("need", (opt + 1).saturating_sub(unit.promoted.sum()))
            .field("nodes", solved.nodes)
            .field("steps", t.steps)
            .seconds("cpu_s", solved.cpu_s)
            .field("completed", solved.completed)
            .field("best", solved.best)
            .field("opt", opt)
            .maybe("root_value", t.root.map(|r| r.0))
            .maybe("root_chosen", t.root.map(|r| r.1.len()))
            .field("conflicts", t.listed.len())
            .list("residues", t.residues.iter().map(HashMap::len))
            .field("asked", t.asked)
            .field("overridden", t.overridden)
            .text("tactics", &Tactics { search: o.tactics, ..Tactics::default() }.description())
            .maybe("prefer", params.prefer.as_ref().map(|path| format!("\"{}\"", path.display())));
        if params.best_cpu > 0.0 {
            let completion = best_completion(&space, &unit, params.best_cpu, n);
            let mut scratch = Scratch::new();
            let state = p.model.promote_all_using(&mut scratch, &p.root, &unit.promoted).expect("the unit");
            let chosen = t.root.map(|r| r.1).unwrap_or_default();
            let list = |k: &Bits<W>| format!("[{}]", k.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","));
            let known = &hooks.known;
            let lf = |x: u32| format!("[{x},{},{}]", known.home[x as usize], known.live_factors(&state, x));
            line.field("best_completion", completion.sum())
                .field("completion", list(&completion))
                .field("chosen", list(&chosen))
                .list("chosen_lf", chosen.iter().map(lf))
                .list("cores", cores(&p.model, &state, &chosen).iter().map(list));
        }
        line.print();
    }
    Ok(())
}

/// `anatomy` on each game of `o`.
pub fn run(o: &Options, optimal: &Optimal) -> Result<(), String> {
    for &n in &o.games {
        taxman::with_width!(n, W => play::<W>(n, o, optimal), else Err(beyond(n)))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use taxman::search::SearchTactics;

    /// A game whose root search takes a few hundred steps on one thread: big enough to branch, small enough to test.
    const N: u32 = 800;

    fn check<const W: usize>() {
        let optimal = Optimal::built_in();
        let board = Board::<W>::for_n(N);
        let p = board.prepare(SearchTactics::default()).expect("prepare");
        let (start, _) = known_incumbent(&board, &p, optimal).expect("known").expect("a known game");
        let graph = MatchGraph::new(&p.model, &p.root);
        let root = TaxUnit { promoted: Bits::EMPTY, cands: p.candidates };
        let space = SearchSpace::new(&p, &graph);
        let plain = run_unit_with(&Control::new(), root, &start, None, |_| TaxSearcher::new(&space));
        let hooked = |hooks: &Hooks<W>| {
            let make = hooks.maker();
            let space = SearchSpace { hooks: Some(&make as &MakeHook<W>), ..SearchSpace::new(&p, &graph) };
            solve(&space, hooks, root, &start, None, None).expect("solve")
        };
        // Told of every step, and the same tree as without the hook.
        let told = hooked(&Hooks::new(Known::new(&p, &[])));
        assert!(plain.completed && told.completed);
        assert_eq!((told.nodes, told.tally.steps, told.best), (plain.nodes, plain.nodes, plain.set.sum()));
        assert!(
            told.tally.asked > 0 && told.tally.overridden == 0,
            "asked {} of {} steps",
            told.tally.asked,
            told.nodes
        );
        // Another order changes the tree and not the best.
        let order: Vec<u32> = p.candidates.iter().collect(); // the smallest first: strong branching never is that
        let steered = hooked(&Hooks::new(Known::new(&p, &order)));
        assert!(steered.completed && steered.tally.overridden > 0);
        assert_eq!(steered.best, plain.set.sum());
        assert_ne!(steered.nodes, plain.nodes);
    }

    #[test]
    fn the_hook_sees_every_step_changes_nothing_and_an_order_keeps_the_best() {
        taxman::with_width!(N, W => check::<W>(), else panic!("{}", beyond(N)));
    }
}
