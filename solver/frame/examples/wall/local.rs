//! `wall local`: is the GF(2) bound's error local, and how much of a node's gap would a bound exact in a few frames
//! close?  The question, the numbers and the verdict are in
//! solver/notebook/search/a-bound-exact-in-a-few-frames-2026-10/README.md, whose `analyze.py` reads the lines.
//!
//! A set of candidates can be promoted exactly when every frame stays winnable with its part of the set applied
//! (the exactness theorem (X7), docs/theory/exactness.md), and the GF(2) bound relaxes each frame's winnability to the
//! independence of its vectors (the frame bound (B2), docs/theory/frame-bounds.md).  The *hybrid bound* of a node that
//! checks the frames `F` is the heaviest completion that is independent in every frame (over GF(2)) and leaves every
//! frame of `F` winnable.  Every promotion set is such a set, so it is an upper bound on the node's completions, never
//! above the GF(2) bound, and with every frame checked it is the node's best completion.  It is what any bound exact in
//! the frames `F`, and GF(2) elsewhere, can reach at most.  It is computed by the search itself on a relaxed model
//! (`FrameGraph::relaxing`: the frames outside `F` are not checked, so the conflicts it branches on are those of `F`)
//! with the GF(2) bound, from the node, with a known best set (worth P*) as the incumbent and without the rule of
//! symmetric pairs.  Its best is the hybrid bound if that beats P*; if it does not, the hybrid bound cuts the node.
//!
//! ```text
//! wall local --known FILE (--suite FILE --ids ID[,ID...] | --root) [--nodes K] [--checked SETS] [--per-unit K]
//!     [--hybrid-cap STEPS] [--compare] [--tactic NAME=VALUE]... N
//! wall local --compare N|A-B...
//! ```
//!
//! * For each unit, the default search of the unit (one thread, fresh tables, from the game's known best set, at most
//!   `--nodes` steps) with a hook that records every node whose subtree takes at least [`KEEP`] steps (its promotions,
//!   its candidates at its first step, its bound and need, its subtree's steps) and, at every bound check of a node,
//!   the *local situation* of each set of `--checked` frames: the node's promotions and candidates among the
//!   candidates that touch those frames (the candidates of a home `h` touch the frames `h − 1` and `h`), which is all
//!   that a computation exact in those frames, with fixed weights, depends on.  The line `{"kind":"unit",...}`: `id`;
//!   the unit's `bound`, `need`, `homes` and `opt_homes` (as a node's, below); `steps`, `completed`, `best`, `opt`
//!   (P*) and `cpu_s`; `conflicts` and `exclusions` by frame (the conflicts of the node checks, and the members the
//!   nodes went on without, by the frame of their conflict); and for each checked set `sets[i]`, `checks` (the node
//!   checks), `distinct` (the distinct local situations among them) and `repeats` (the checks whose situation an
//!   earlier check had).
//! * Then the nodes measured: up to `--per-unit` of the recorded ones (default 12), the same number from each decade
//!   of subtree size from [`KEEP`] steps up, spread over the tree, the unit's own root among them.  A line
//!   `{"kind":"node",...}` each: `id`, `node` (its order), `depth`, `subtree`, `bound` (the GF(2) bound on what its
//!   completions add), `need` (what they must add to beat P*), `homes` (the weight of its heaviest set by home, index
//!   `h` for home `h`), `opt_homes` (the known best set's completion by home, if the node holds that set, else null),
//!   and `hybrid`: for each checked set, `[frames, best, completed, steps, dual, cpu_s, seen]`, where `best` is the
//!   hybrid search's best completion sum less the node's promotions (the incumbent's, P* less them, if nothing beat
//!   it), `dual` an upper bound on the hybrid bound when the search stopped at `--hybrid-cap` (default 400,000 steps;
//!   the largest bound of a node left on its stack), and `seen` how many node checks of the unit's search had the
//!   node's local situation for that set.  The set `all` checks every frame (the search of the node's proof, without
//!   the rule of symmetric pairs: its best must be P*) and `none` none (its best is the GF(2) bound).
//! * With `--compare`, a line `{"kind":"compare",...}` for each game `n`: how its frames and candidates differ from
//!   those of `n + 1` (`frames[i]`: the moves and factors of frame `i` in one and not the other, and in both;
//!   `homes[h]`: the same for the candidates of home `h`), which says whether a local situation can recur from one
//!   game to the next.  With neither `--suite` nor `--root`, only these lines, for any games (no known best set
//!   needed).
//!
//! `--checked` lists the sets, separated by `/`, each frame numbers separated by commas, or `all` or `none`: the
//! default is `none/1/2/3/1,2/2,3/1,2,3/all`.
//!
//! With `--out FILE`, each node measured is also appended to `FILE` whole, as a line `{"kind":"instance",...}`, for
//! the study of frame 2's exact problem (solver/notebook/search/the-exact-problem-of-frame-2-2026-10/): `id`, `node` (as
//! in its `node` line, or null for a unit's own start), `depth`, `subtree`, `bound`, `need`, `max_add`, `promoted`
//! (its promotions), `frames` (each frame of its state, front first: `[moves, factors, freedoms]`), `cands` (its
//! candidates, each `[number, home]`), `chosen` (the GF(2) bound's heaviest set) and `flipped` (for each candidate,
//! the GF(2) bound without it if it is in `chosen`, with it if not, or null for a candidate whose class in some frame
//! is 0).  For a game with no known best set only the units' own starts are written, and nothing is searched.

use super::anatomy::suite_units;
use super::{beyond, Line, Options};
use crate::counters::cpu_seconds;
use crate::known::known_incumbent;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::bounds::max_additional;
use taxman::engine::{self, Control, EngineConfig, Start};
use taxman::game::Optimal;
use taxman::gf2::{Gf2Bound, Gf2Graph};
use taxman::matchbound::MatchGraph;
use taxman::model::{FrameGraph, FrameSet, Scratch, MAX_FRAMES};
use taxman::search::{
    payload_of, set_of_payload, MakeHook, Prepared, SearchSpace, SearchTactics, StepEnd, StepHook, StepRecord,
    TaxSearcher, TaxUnit,
};

/// The options of `local`.
#[derive(Clone, Default)]
pub struct Params {
    /// The sets of frames the hybrid bounds check (`--checked`), a bit a frame; empty: [`DEFAULT_CHECKED`].
    pub checked: Vec<u16>,
    /// The nodes measured a unit (`--per-unit`; 0: [`PER_UNIT`]).
    pub per_unit: usize,
    /// The steps at which a hybrid search stops (`--hybrid-cap`; 0: [`HYBRID_CAP`]).
    pub hybrid_cap: u64,
    /// Compare each game's frames with those of the next (`--compare`).
    pub compare: bool,
}

/// Every frame checked (`all`).
const ALL: u16 = u16::MAX;

/// The sets of `--checked` when it is not given.
const DEFAULT_CHECKED: &str = "none/1/2/3/1,2/2,3/1,2,3/all";

/// The nodes measured a unit when `--per-unit` is not given.
const PER_UNIT: usize = 12;

/// The steps of a hybrid search when `--hybrid-cap` is not given.
const HYBRID_CAP: u64 = 400_000;

/// A node is recorded if its subtree takes at least this many steps.
pub const KEEP: u64 = 100;

/// The sets of `--checked SETS` (see the module docs).
pub fn parse_checked(text: &str) -> Result<Vec<u16>, String> {
    text.split('/')
        .map(|set| match set {
            "all" => Ok(ALL),
            "none" => Ok(0),
            _ => set.split(',').try_fold(0u16, |bits, frame| match frame.parse::<usize>() {
                Ok(i) if i < MAX_FRAMES => Ok(bits | 1 << i),
                _ => Err(format!("--checked {text}: {frame:?} is not a frame (0 to {})", MAX_FRAMES - 1)),
            }),
        })
        .collect()
}

/// The name of a checked set, as `--checked` writes it.
fn set_name(set: u16) -> String {
    match set {
        ALL => "all".to_owned(),
        0 => "none".to_owned(),
        _ => (0..MAX_FRAMES).filter(|i| set & (1 << i) != 0).map(|i| i.to_string()).collect::<Vec<_>>().join(","),
    }
}

// ---------------------------------------------------------------------------------------------------------
// What the hooks share

/// A game's model, the candidates that touch each checked set, and the frames of a conflict.
struct Game<const W: usize> {
    model: Arc<FrameGraph>,
    /// The checked sets, and for each the root's candidates that touch one of its frames.
    sets: Vec<u16>,
    touch: Vec<Bits<W>>,
}

impl<const W: usize> Game<W> {
    fn new(p: &Prepared<W>, sets: &[u16]) -> Self {
        let model = Arc::clone(&p.model);
        let touch = sets
            .iter()
            .map(|&set| {
                p.candidates
                    .iter()
                    .filter(|&x| {
                        let (into, out) = model.frames_touched(x);
                        set & (1 << into) != 0 || out.is_some_and(|h| set & (1 << h) != 0)
                    })
                    .collect()
            })
            .collect();
        Game { model, sets: sets.to_vec(), touch }
    }

    /// The local situation of a node for the checked set `i`: a hash of its promotions and candidates among the
    /// candidates that touch the set's frames.
    fn key(&self, i: usize, promoted: &Bits<W>, cands: &Bits<W>) -> u64 {
        let mut h = DefaultHasher::new();
        (*promoted & self.touch[i]).as_words().hash(&mut h);
        (*cands & self.touch[i]).as_words().hash(&mut h);
        h.finish()
    }

    /// The frame in which the conflict `k` of a node in `state` fails: a conflict lies in one frame, with members of
    /// the home of that frame (factors it loses) and of the next (moves it gains).
    fn frame_of(&self, k: &Bits<W>, state: &FrameSet<W>) -> usize {
        let homes = || k.iter().map(|x| self.model.home(x));
        let (lo, hi) = (homes().min().unwrap_or(0), homes().max().unwrap_or(0));
        if hi > lo || lo == 0 {
            return lo;
        }
        // Members of one home `h`: moves of frame `h − 1`, or factors that frame `h` loses.
        let front = &state.frames()[lo - 1];
        if self.model.winnable(&(front.moves | *k), &front.factors) {
            lo
        } else {
            lo - 1
        }
    }
}

/// A node of a unit's tree, as its first step saw it.
#[derive(Clone, Copy)]
struct Node<const W: usize> {
    promoted: Bits<W>,
    cands: Bits<W>,
    need: u64,
    bound: Option<u64>,
    depth: usize,
    /// Its first step, and the steps of its subtree (its own and its descendants').
    first: u64,
    subtree: u64,
}

/// What the recording hook of a unit's search gathered.
#[derive(Default)]
struct Recorded<const W: usize> {
    steps: u64,
    /// The nodes whose subtree took at least [`KEEP`] steps, in the order their loops ended.
    nodes: Vec<Node<W>>,
    conflicts: [u64; MAX_FRAMES],
    exclusions: [u64; MAX_FRAMES],
    /// Per checked set: how many node checks had each local situation.
    situations: Vec<HashMap<u64, u32>>,
    checks: u64,
    repeats: Vec<u64>,
}

/// The recording hook of one searcher (see the module docs); what it gathers goes to `shared` when it is dropped.
struct Recorder<const W: usize> {
    game: Arc<Game<W>>,
    shared: Arc<Mutex<Recorded<W>>>,
    done: Recorded<W>,
    /// The nodes of the searcher's path, and the bound of the child last made, for its first step.
    stack: Vec<Node<W>>,
    child_bound: Option<u64>,
}

impl<const W: usize> Recorder<W> {
    fn new(game: Arc<Game<W>>, shared: Arc<Mutex<Recorded<W>>>) -> Self {
        let sets = game.sets.len();
        let done = Recorded { situations: vec![HashMap::new(); sets], repeats: vec![0; sets], ..Recorded::default() };
        Recorder { game, shared, done, stack: Vec::new(), child_bound: None }
    }
}

impl<const W: usize> Drop for Recorder<W> {
    fn drop(&mut self) {
        let done = std::mem::take(&mut self.done);
        *self.shared.lock().unwrap_or_else(PoisonError::into_inner) = done;
    }
}

impl<const W: usize> StepHook<W> for Recorder<W> {
    fn step(&mut self, r: &StepRecord<W>, state: &FrameSet<W>, _child: Option<&FrameSet<W>>) {
        let done = &mut self.done;
        done.steps += 1;
        if self.stack.last().is_none_or(|top| top.promoted != r.promoted) {
            let bound = r.node.map(|c| c.value).or(self.child_bound.take());
            let depth = self.stack.len();
            let (promoted, cands, need, first) = (r.promoted, r.cands, r.need, done.steps);
            self.stack.push(Node { promoted, cands, need, bound, depth, first, subtree: 0 });
        }
        if let Some(c) = r.node {
            done.checks += 1;
            for i in 0..self.game.sets.len() {
                let seen = done.situations[i].entry(self.game.key(i, &r.promoted, &r.cands)).or_insert(0);
                done.repeats[i] += u64::from(*seen > 0);
                *seen += 1;
            }
            if let Some(k) = c.conflict {
                let frame = self.game.frame_of(&k, state);
                done.conflicts[frame] += 1;
                if r.branch.is_some_and(|b| k.contains(b) && r.added == Bits::EMPTY.with(b)) {
                    done.exclusions[frame] += 1;
                }
            }
        }
        match r.end {
            StepEnd::Child => self.child_bound = r.child.map(|c| c.value),
            StepEnd::LoopEnd(_) => {
                let mut node = self.stack.pop().expect("a node on the path");
                node.subtree = done.steps - node.first + 1;
                if node.subtree >= KEEP {
                    done.nodes.push(node);
                }
            }
            _ => {}
        }
    }
}

/// What a hybrid search's hook tracked: an upper bound on what the search had left when it stopped.
#[derive(Default)]
struct Tracked {
    /// The largest bound (promotions plus the last GF(2) bound) of a node left on the path, if any was left.
    dual: Option<u64>,
}

/// The hook of a hybrid search: the last bound of each node of the path (its promotions included).  A node's last
/// bound bounds every completion its loop has still to try, since the loop's candidates only shrink.
struct Tracker {
    shared: Arc<Mutex<Tracked>>,
    path: Vec<Option<u64>>,
    depth: usize,
}

impl Drop for Tracker {
    fn drop(&mut self) {
        let dual = self.path.iter().take(self.depth + 1).flatten().copied().max();
        self.shared.lock().unwrap_or_else(PoisonError::into_inner).dual = dual;
    }
}

impl<const W: usize> StepHook<W> for Tracker {
    fn step(&mut self, r: &StepRecord<W>, _: &FrameSet<W>, _: Option<&FrameSet<W>>) {
        let d = self.depth;
        if self.path.len() < d + 2 {
            self.path.resize(d + 2, None);
        }
        if let Some(c) = r.node {
            self.path[d] = Some(r.promoted.sum() + c.value);
        }
        match r.end {
            StepEnd::Child => {
                self.path[d + 1] = r.child.map(|c| (r.promoted | r.added).sum() + c.value);
                self.depth = d + 1;
            }
            StepEnd::LoopEnd(_) => {
                self.path[d] = None;
                self.depth = d.saturating_sub(1);
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The searches

/// What one search of a unit came to.
struct Solved {
    steps: u64,
    completed: bool,
    best: u64,
    cpu_s: f64,
}

/// Solves `unit` on one thread from the incumbent `start` in `space`, with at most `node_limit` steps.
fn solve<const W: usize>(
    space: &SearchSpace<'_, W>,
    unit: TaxUnit<W>,
    start: &Bits<W>,
    node_limit: Option<u64>,
) -> Solved {
    let cfg = EngineConfig { threads: 1, node_limit, control: Some(Control::new()), ..EngineConfig::default() };
    let begin = Start::Fresh { units: vec![unit], best: start.sum(), payload: payload_of(start) };
    let before = cpu_seconds();
    let out = engine::run(&cfg, begin, b"local", |_| TaxSearcher::new(space)).expect("the engine");
    let cpu_s = before.zip(cpu_seconds()).map_or(0.0, |(a, b)| b - a);
    Solved { steps: out.stats.nodes, completed: out.completed, best: set_of_payload::<W>(&out.payload).sum(), cpu_s }
}

/// The default search of `unit`, recorded.
fn record<const W: usize>(
    p: &Prepared<W>,
    graph: &MatchGraph,
    game: &Arc<Game<W>>,
    unit: TaxUnit<W>,
    start: &Bits<W>,
    node_limit: Option<u64>,
) -> (Solved, Recorded<W>) {
    let shared = Arc::new(Mutex::new(Recorded::default()));
    let make = {
        let (game, shared) = (Arc::clone(game), Arc::clone(&shared));
        move || Box::new(Recorder::<W>::new(Arc::clone(&game), Arc::clone(&shared))) as Box<dyn StepHook<W>>
    };
    let hooks: &MakeHook<W> = &make;
    let space = SearchSpace { hooks: Some(hooks), ..SearchSpace::new(p, graph) };
    let solved = solve(&space, unit, start, node_limit);
    drop(space);
    let recorded = std::mem::take(&mut *shared.lock().unwrap_or_else(PoisonError::into_inner));
    (solved, recorded)
}

/// The hybrid search of `unit` that checks the frames `checked` (see the module docs), and its dual bound if it
/// stopped.
fn hybrid<const W: usize>(
    p: &Prepared<W>,
    graph: &MatchGraph,
    checked: u16,
    unit: TaxUnit<W>,
    start: &Bits<W>,
    cap: u64,
) -> (Solved, Option<u64>) {
    let frames = p.root.frames().len();
    let relaxed = p.model.relaxing(!checked & ((1u16 << frames) - 1));
    let shared = Arc::new(Mutex::new(Tracked::default()));
    let make = {
        let shared = Arc::clone(&shared);
        move || {
            let tracker = Tracker { shared: Arc::clone(&shared), path: Vec::new(), depth: 0 };
            Box::new(tracker) as Box<dyn StepHook<W>>
        }
    };
    let hooks: &MakeHook<W> = &make;
    let space = SearchSpace { model: &relaxed, symmetry: None, hooks: Some(hooks), ..SearchSpace::new(p, graph) };
    let solved = solve(&space, unit, start, Some(cap));
    drop(space);
    let dual = shared.lock().unwrap_or_else(PoisonError::into_inner).dual.filter(|_| !solved.completed);
    (solved, dual)
}

// ---------------------------------------------------------------------------------------------------------
// The nodes measured

/// Up to `k` of `nodes`: the same number from each decade of subtree size from [`KEEP`] up (what a decade lacks goes to
/// the others), spread evenly over each decade's nodes in the order their loops ended; the root (the largest) always.
fn sample<const W: usize>(nodes: &[Node<W>], k: usize) -> Vec<Node<W>> {
    let mut decades: Vec<Vec<Node<W>>> = Vec::new();
    for node in nodes.iter().filter(|n| n.bound.is_some_and(|b| b >= n.need)) {
        let d = (node.subtree as f64 / KEEP as f64).log10().floor() as usize;
        if decades.len() <= d {
            decades.resize_with(d + 1, Vec::new);
        }
        decades[d].push(*node);
    }
    let mut chosen: Vec<Node<W>> = Vec::new();
    if let Some(root) = nodes.iter().max_by_key(|n| n.subtree) {
        chosen.push(*root);
    }
    let mut left = k.saturating_sub(chosen.len());
    let mut open: Vec<usize> = (0..decades.len()).filter(|&d| !decades[d].is_empty()).collect();
    let mut taken = vec![0usize; decades.len()];
    while left > 0 && !open.is_empty() {
        let share = (left / open.len()).max(1);
        for &d in &open {
            let more = share.min(decades[d].len() - taken[d]).min(left);
            taken[d] += more;
            left -= more;
        }
        open.retain(|&d| taken[d] < decades[d].len());
    }
    // A node whose subtree is nearly that of a node chosen above it (a child that promotes forced members) adds
    // nothing: the next node of its decade stands in for it.
    let near = |c: &Node<W>, node: &Node<W>| c.promoted.is_subset(&node.promoted) && 10 * node.subtree > 9 * c.subtree;
    for (d, nodes) in decades.iter().enumerate() {
        let t = taken[d];
        for j in 0..t {
            let start = (2 * j + 1) * nodes.len() / (2 * t);
            let end = (2 * j + 3) * nodes.len() / (2 * t);
            let pick = nodes[start..end.min(nodes.len())].iter().find(|node| chosen.iter().all(|c| !near(c, node)));
            if let Some(&node) = pick {
                chosen.push(node);
            }
        }
    }
    chosen.sort_by_key(|n| n.first);
    chosen
}

/// The weight of `set` by home, index `h` for home `h`.
fn by_home<const W: usize>(model: &FrameGraph, set: &Bits<W>) -> Vec<u64> {
    let mut out = vec![0; MAX_FRAMES];
    for x in set.iter() {
        out[model.home(x)] += u64::from(x);
    }
    while out.len() > 1 && out.last() == Some(&0) {
        out.pop();
    }
    out
}

/// A node's GF(2) bound on what its completions add, its heaviest set's weight by home, and, if the node holds the
/// known best set `opt_set` (its promotions are in that set, and the rest of the set is among its candidates), the
/// weight of that set's completion by home.
fn bound_by_home<const W: usize>(
    p: &Prepared<W>,
    bound: &mut Gf2Bound<'_, W>,
    opt_set: &Bits<W>,
    unit: &TaxUnit<W>,
) -> (u64, Vec<u64>, Option<String>) {
    let state = p.model.promote_all_using(&mut Scratch::new(), &p.root, &unit.promoted).expect("a node's promotions");
    let max_add = max_additional(&p.model, p.max_total, &state, &unit.promoted, &unit.cands);
    bound.reset(&unit.promoted, &unit.cands);
    let eval = bound.evaluate(max_add, None);
    let rest = *opt_set - unit.promoted;
    let holds = unit.promoted.is_subset(opt_set) && rest.is_subset(&unit.cands);
    let opt_homes = holds.then(|| format!("{:?}", by_home(&p.model, &rest)).replace(' ', ""));
    (eval.value, by_home(&p.model, &eval.chosen), opt_homes)
}

/// Where a node of a unit's tree sits, for its line of `--out`.
struct Placed<'a> {
    id: &'a str,
    node: Option<usize>,
    depth: usize,
    subtree: u64,
    need: Option<u64>,
}

/// Appends the line `{"kind":"instance",...}` of the node `unit` (see the module docs) to `out`.
fn dump_instance<const W: usize>(
    p: &Prepared<W>,
    bound: &mut Gf2Bound<'_, W>,
    unit: &TaxUnit<W>,
    at: &Placed<'_>,
    out: &Path,
) -> Result<(), String> {
    let state = p.model.promote_all_using(&mut Scratch::new(), &p.root, &unit.promoted).expect("a node's promotions");
    let max_add = max_additional(&p.model, p.max_total, &state, &unit.promoted, &unit.cands);
    bound.reset(&unit.promoted, &unit.cands);
    let eval = bound.evaluate(max_add, None);
    let list = |set: &Bits<W>| format!("{:?}", set.iter().collect::<Vec<u32>>()).replace(' ', "");
    let frames: Vec<String> =
        state.frames().iter().map(|f| format!("[{},{},{}]", list(&f.moves), list(&f.factors), f.freedoms)).collect();
    // Each candidate's GF(2) bound with its membership flipped against the heaviest set: without it for a member,
    // with it for the others (null for a candidate whose class in some frame is 0, which no completion holds).
    let live = bound.live();
    let mut value: HashMap<u32, u64> = HashMap::new();
    for x in eval.chosen.iter() {
        value.insert(x, bound.value_excluding(x, max_add, u64::MAX));
    }
    for x in (live - eval.chosen).iter() {
        bound.reset(&unit.promoted.with(x), &unit.cands.without(x));
        value.insert(x, bound.evaluate(max_add.saturating_sub(1), None).value + u64::from(x));
    }
    let flipped: Vec<String> =
        unit.cands.iter().map(|x| value.get(&x).map_or("null".to_owned(), u64::to_string)).collect();
    let cands: Vec<String> = unit.cands.iter().map(|x| format!("[{x},{}]", p.model.home(x))).collect();
    let mut line = Line::of(p.n);
    line.text("kind", "instance")
        .text("id", at.id)
        .maybe("node", at.node)
        .field("depth", at.depth)
        .field("subtree", at.subtree)
        .field("bound", eval.value)
        .maybe("need", at.need)
        .field("max_add", max_add)
        .field("promoted", list(&unit.promoted))
        .list("frames", frames)
        .list("cands", cands)
        .field("chosen", list(&eval.chosen))
        .list("flipped", flipped);
    let mut file =
        OpenOptions::new().create(true).append(true).open(out).map_err(|e| format!("{}: {e}", out.display()))?;
    writeln!(file, "{}", line.render()).map_err(|e| format!("{}: {e}", out.display()))
}

/// `local` on game `n` at the set width `W`.
fn play<const W: usize>(n: u32, o: &Options, optimal: &Optimal) -> Result<(), String> {
    let board = Board::<W>::for_n(n);
    if board.is_trivial() {
        return Ok(());
    }
    let p = board.prepare(o.tactics)?;
    let units = if o.exchanges.root {
        vec![(format!("{n}-root"), TaxUnit { promoted: Bits::EMPTY, cands: p.candidates })]
    } else {
        let suite = o.params.suite.as_ref().ok_or("local needs --suite or --root")?;
        suite_units::<W>(suite, n, &o.anatomy.ids)?
    };
    let known = known_incumbent(&board, &p, optimal)?;
    let gf2 = Gf2Graph::new(&p.model, &p.root, &p.candidates);
    let mut bound = Gf2Bound::<W>::new(&gf2);
    if let (None, Some(out)) = (&known, &o.out) {
        for (id, unit) in &units {
            let at = Placed { id, node: None, depth: 0, subtree: 0, need: None };
            dump_instance(&p, &mut bound, unit, &at, out)?;
        }
        return Ok(());
    }
    let (opt_set, opt) = known.ok_or_else(|| format!("n={n}: needs a known optimum"))?;
    let l = &o.local;
    let parsed;
    let sets = if l.checked.is_empty() {
        parsed = parse_checked(DEFAULT_CHECKED)?;
        &parsed
    } else {
        &l.checked
    };
    let per_unit = if l.per_unit == 0 { PER_UNIT } else { l.per_unit };
    let cap = if l.hybrid_cap == 0 { HYBRID_CAP } else { l.hybrid_cap };
    let graph = MatchGraph::new(&p.model, &p.root);
    let game = Arc::new(Game::new(&p, sets));
    let names: Vec<String> = sets.iter().map(|&s| set_name(s)).collect();
    for (id, unit) in units {
        let (value, homes, opt_homes) = bound_by_home(&p, &mut bound, &opt_set, &unit);
        let (solved, rec) = record(&p, &graph, &game, unit, &opt_set, o.nodes);
        let mut line = Line::of(n);
        line.text("kind", "unit")
            .text("id", &id)
            .field("bound", value)
            .field("need", opt - unit.promoted.sum() + 1)
            .list("homes", homes)
            .maybe("opt_homes", opt_homes)
            .field("steps", solved.steps)
            .field("completed", solved.completed)
            .field("best", solved.best)
            .field("opt", opt)
            .seconds("cpu_s", solved.cpu_s)
            .field("recorded", rec.nodes.len())
            .list("conflicts", rec.conflicts)
            .list("exclusions", rec.exclusions)
            .field("checks", rec.checks)
            .list("sets", names.iter().map(|s| format!("\"{s}\"")))
            .list("distinct", rec.situations.iter().map(HashMap::len))
            .list("repeats", rec.repeats.iter());
        line.print();
        if solved.best != opt {
            return Err(format!("n={n} {id}: the default search found {} against the optimum {opt}", solved.best));
        }
        for (i, node) in sample(&rec.nodes, per_unit).iter().enumerate() {
            let unit = TaxUnit { promoted: node.promoted, cands: node.cands };
            if let Some(out) = &o.out {
                let at =
                    Placed { id: &id, node: Some(i), depth: node.depth, subtree: node.subtree, need: Some(node.need) };
                dump_instance(&p, &mut bound, &unit, &at, out)?;
            }
            let (value, homes, opt_homes) = bound_by_home(&p, &mut bound, &opt_set, &unit);
            let base = node.promoted.sum();
            let mut hybrids = Vec::new();
            for (s, &set) in sets.iter().enumerate() {
                let (h, dual) = hybrid(&p, &graph, set, unit, &opt_set, cap);
                if set == ALL && h.completed && h.best != opt {
                    return Err(format!("n={n} {id} node {i}: the proof found {} against the optimum {opt}", h.best));
                }
                let seen = rec.situations[s].get(&game.key(s, &node.promoted, &node.cands)).copied().unwrap_or(0);
                let dual = dual.map(|d| d.saturating_sub(base));
                hybrids.push(format!(
                    "[\"{}\",{},{},{},{},{:.3},{seen}]",
                    names[s],
                    h.best - base.min(h.best),
                    h.completed,
                    h.steps,
                    dual.map_or("null".to_owned(), |d| d.to_string()),
                    h.cpu_s
                ));
            }
            let mut line = Line::of(n);
            line.text("kind", "node")
                .text("id", &id)
                .field("node", i)
                .field("depth", node.depth)
                .field("subtree", node.subtree)
                .field("bound", value)
                .maybe("recorded_bound", node.bound)
                .field("need", node.need)
                .list("homes", homes)
                .maybe("opt_homes", opt_homes)
                .list("hybrid", hybrids);
            line.print();
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// From one game to the next

/// A game's frames, each its moves and its factors, and its candidates, each with its home.
type FramesOf = (Vec<(Vec<u32>, Vec<u32>)>, Vec<(u32, usize)>);

/// The frames and candidates of game `n`.
fn frames_of(n: u32) -> Result<FramesOf, String> {
    fn at<const W: usize>(n: u32) -> Result<FramesOf, String> {
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default())?;
        let frames = p.root.frames().iter().map(|f| (f.moves.to_vec(), f.factors.to_vec())).collect();
        let cands = p.candidates.iter().map(|x| (x, p.model.home(x))).collect();
        Ok((frames, cands))
    }
    taxman::with_width!(n, W => at::<W>(n), else Err(beyond(n)))
}

/// `[only in a, only in b, in both]` of two ascending lists.
fn differ(a: &[u32], b: &[u32]) -> String {
    let both = a.iter().filter(|x| b.binary_search(x).is_ok()).count();
    format!("[{},{},{both}]", a.len() - both, b.len() - both)
}

/// The line `{"kind":"compare",...}` of game `n` against `n + 1`.
fn compare(n: u32) -> Result<(), String> {
    let (fa, ca) = frames_of(n)?;
    let (fb, cb) = frames_of(n + 1)?;
    let frames = (0..fa.len().max(fb.len())).map(|i| {
        let empty = (Vec::new(), Vec::new());
        let (a, b) = (fa.get(i).unwrap_or(&empty), fb.get(i).unwrap_or(&empty));
        format!("{{\"moves\":{},\"factors\":{}}}", differ(&a.0, &b.0), differ(&a.1, &b.1))
    });
    let homes = (0..MAX_FRAMES).map(|h| {
        let of = |c: &[(u32, usize)]| c.iter().filter(|&&(_, home)| home == h).map(|&(x, _)| x).collect::<Vec<_>>();
        differ(&of(&ca), &of(&cb))
    });
    let mut line = Line::of(n);
    line.text("kind", "compare").field("next", n + 1).list("frames", frames).list("homes", homes);
    line.print();
    Ok(())
}

/// `local` on each game of `o`: with `--compare` and no units (neither `--suite` nor `--root`), only the comparisons.
pub fn run(o: &Options, optimal: &Optimal) -> Result<(), String> {
    let search = o.exchanges.root || o.params.suite.is_some();
    if !search && !o.local.compare {
        return Err("local needs --suite or --root, or --compare".to_owned());
    }
    for &n in &o.games {
        if o.local.compare {
            compare(n)?;
        }
        if search {
            taxman::with_width!(n, W => play::<W>(n, o, optimal), else Err(beyond(n)))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checked_sets_parse_and_print() {
        let sets = parse_checked(DEFAULT_CHECKED).expect("the default");
        assert_eq!(sets, vec![0, 0b10, 0b100, 0b1000, 0b110, 0b1100, 0b1110, ALL]);
        assert_eq!(sets.iter().map(|&s| set_name(s)).collect::<Vec<_>>().join("/"), DEFAULT_CHECKED);
        assert!(parse_checked("1,x").is_err() && parse_checked("9").is_err());
    }

    /// Game 1215 from its known best set: the hybrid bound checking no frame is the root's GF(2) bound, checking all of
    /// them is P*, and checking more frames never gives more; the recording hook changes no tree, and its nodes'
    /// subtrees fit in the tree.
    fn hybrids_1215<const W: usize>() {
        let n = 1215;
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default()).expect("prepare");
        let (set, opt) = known_incumbent(&board, &p, Optimal::built_in()).expect("known").expect("a known game");
        let graph = MatchGraph::new(&p.model, &p.root);
        let root = TaxUnit { promoted: Bits::EMPTY, cands: p.candidates };
        let plain = solve(&SearchSpace::new(&p, &graph), root, &set, None);
        let sets = parse_checked(DEFAULT_CHECKED).expect("the default");
        let game = Arc::new(Game::new(&p, &sets));
        let (solved, rec) = record(&p, &graph, &game, root, &set, None);
        assert_eq!((solved.steps, solved.best, rec.steps), (plain.steps, opt, plain.steps));
        assert!(rec.nodes.iter().all(|node| node.first + node.subtree <= rec.steps + 1));
        assert_eq!(rec.nodes.iter().map(|node| node.subtree).max(), Some(plain.steps));
        let gf2 = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let mut bound = Gf2Bound::<W>::new(&gf2);
        bound.reset(&Bits::EMPTY, &p.candidates);
        let max_add = max_additional(&p.model, p.max_total, &p.root, &Bits::EMPTY, &p.candidates);
        let value = bound.evaluate(max_add, None).value;
        let best = |checked: u16| {
            let (h, dual) = hybrid(&p, &graph, checked, root, &set, HYBRID_CAP);
            assert!(h.completed && dual.is_none());
            h.best
        };
        let (none, two, one_two, all) = (best(0), best(0b100), best(0b110), best(ALL));
        assert_eq!((none, all), (value, opt));
        assert!(none >= two && two >= one_two && one_two >= all, "{none} {two} {one_two} {all}");
        assert!(two < none && one_two < two, "frames 1 and 2 hold the root's gap at {n}");
        // Stopped early, the hybrid search's dual bound lies between what it found and the GF(2) bound.
        let (h, dual) = hybrid(&p, &graph, 0b110, root, &set, 1);
        let dual = dual.expect("a stopped search's dual bound");
        assert!(!h.completed && h.best <= dual && one_two <= dual && dual <= value, "{} {dual}", h.best);
    }

    #[test]
    fn the_hybrid_bounds_lie_between_the_gf2_bound_and_the_optimum() {
        taxman::with_width!(1215, W => hybrids_1215::<W>(), else panic!("{}", beyond(1215)));
    }

    /// The root of 600 written whole: its frames and candidates as the model has them, and each candidate's flipped
    /// bound at most the root's bound, null exactly for the candidates whose class in some frame is 0.
    fn instance_600<const W: usize>() {
        use taxman::json::Json;
        let board = Board::<W>::for_n(600);
        let p = board.prepare(SearchTactics::default()).expect("prepare");
        let gf2 = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let mut bound = Gf2Bound::<W>::new(&gf2);
        let unit = TaxUnit { promoted: Bits::EMPTY, cands: p.candidates };
        let out = std::env::temp_dir().join(format!("wall-local-instance-{}.jsonl", std::process::id()));
        let _ = std::fs::remove_file(&out);
        let at = Placed { id: "600-root", node: None, depth: 0, subtree: 0, need: None };
        dump_instance(&p, &mut bound, &unit, &at, &out).expect("the line");
        let text = std::fs::read_to_string(&out).expect("the file");
        std::fs::remove_file(&out).expect("removed");
        let line = Json::parse(text.trim()).expect("one JSON line");
        let numbers = |key: &str| -> Vec<Json> { line.get(key).and_then(Json::as_arr).expect(key).to_vec() };
        let frames = numbers("frames");
        assert_eq!(frames.len(), p.root.frames().len());
        for (frame, f) in frames.iter().zip(p.root.frames()) {
            let moves: Vec<u32> =
                frame.at(0).and_then(Json::as_arr).expect("moves").iter().filter_map(Json::as_u32).collect();
            assert_eq!(moves, f.moves.to_vec());
        }
        let value = line.get("bound").and_then(Json::as_u64).expect("bound");
        let (cands, flipped) = (numbers("cands"), numbers("flipped"));
        let count = p.candidates.len() as usize;
        assert_eq!((cands.len(), flipped.len()), (count, count));
        bound.reset(&Bits::EMPTY, &p.candidates);
        let live = bound.live();
        for (c, v) in cands.iter().zip(&flipped) {
            let x = c.at(0).and_then(Json::as_u32).expect("a candidate");
            assert_eq!(c.at(1).and_then(Json::as_u64), Some(p.model.home(x) as u64));
            assert_eq!(v.is_null(), !live.contains(x), "{x}");
            assert!(v.as_u64().is_none_or(|v| v <= value), "{x}: {v:?} against {value}");
        }
    }

    #[test]
    fn an_instance_line_holds_the_node_whole() {
        taxman::with_width!(600, W => instance_600::<W>(), else panic!("{}", beyond(600)));
    }
}
