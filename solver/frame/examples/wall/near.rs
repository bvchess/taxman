//! `wall near`: how many completions of a unit come near the optimum, counted exactly or estimated.  A unit of the
//! tuning suite is a node with candidates (`wall anatomy`'s units); its *completions* are the sets of its candidates
//! that the node can promote, with at most `max_total` promotions in all, and a completion's *value* is the promotion
//! sum of the unit's promotions and its own.  The command counts the completions whose value is at least the *floor*,
//! the game's optimum less `--within D`, by value.
//!
//! ```text
//! cargo run --profile test --features studies --example wall -- near --known FILE (--suite FILE --ids ID[,ID...]
//!     | --root) --within D [--nodes K] [--probes K [--frontier G] [--seed S]] [--sets DIR] [--tactic NAME=VALUE]... N
//! ```
//!
//! With `--root` the unit is the whole game: its root, with every candidate.
//!
//! **The count** is the solver's search of the unit, made to count (`search::StepHook::counting`): one thread, fresh
//! tables, the incumbent the floor less one with no set, so that every node needs what reaches the floor.  The search
//! offers nothing, so that need never moves, and where a node's heaviest set can be promoted it branches on that set's
//! members instead of stopping there, so every completion at the floor is made as a child, once; the hook counts it
//! there, whether the child is explored or not.  The rule of symmetric pairs is off (it keeps one side of each pair), and so the tree is larger than the
//! search's at the same need by that rule's share.  `--nodes K` stops the count after about K steps; the counts are
//! then lower bounds.
//!
//! **The estimate** (`--probes K`): K random descents of a plainer tree of the same completions to its *frontier*, the
//! nodes whose gap (bound less need) is at most `--frontier G` (default 300) or that are at the floor already, each
//! followed by the count of its frontier node's completions (stopped after [`FRONTIER_STEPS`] steps).  This is Knuth's
//! estimator with importance sampling and exact counts at the frontier.  A node of the plainer tree is a set `P` of
//! promotions with candidates `C`, and stands for the completions `P ∪ G`, `G ⊆ C`.  A step of its loop:
//!
//! * if `C` is empty, or the node can make no more promotions, `P` is its only completion;
//! * if the node's GF(2) bound is below what `P` needs to reach the floor, it has none at the floor;
//! * if the bound's heaviest set has members forced by that need, every completion at the floor holds them all, and the
//!   node's one child promotes them;
//! * otherwise the node branches on a member `x` of the heaviest set's conflict (of the heaviest set itself if it can
//!   be promoted): the child that promotes `x`, then the node goes on without `x`.  The member is the one whose
//!   weight-split bound without it is lowest (`Gf2Bound::split_without`).
//!
//! A descent bounds both sides of a branch, the child and the node without `x`.  A side whose bound is below its need
//! is dead, and of two live sides one is taken with a probability proportional to 10^(gap / 250) (a subtree grows about
//! a decade per 250 points of gap, as the anatomy of a hard subtree measured).  The descent's weight is the product of
//! the inverses of those probabilities.  Each descent's weight times its frontier node's count at each value is an
//! unbiased estimate of the unit's count there; the mean over the K descents (a dead one gives 0) is the estimate, and
//! their spread gives its standard error.
//!
//! **The line**, one a unit: `id`, `depth` (the unit's promotions), `cands`, `opt`, `within` (D), `floor`, `bound` (the
//! value of the unit's GF(2) bound: its promotions' sum and the bound), `steps` (the count's search steps, or the
//! descents'), `cpu_s` and `best` (the largest value counted, or null).  The count adds `children` (the children made),
//! `completed`, `count` and `by_distance` (`[d, count]` for each distance `d` below the optimum that has completions).
//! The estimate adds `probes`, `frontier`, `frontier_steps` (the steps of the frontier counts), `incomplete` (frontier
//! counts stopped: the estimate is then low), `hits` (descents whose frontier node had a completion at the floor) and
//! `estimates` (`[d, estimate, standard error]` of the completions within `d` of the optimum, for `d` = 0, 10, 20, ...
//! up to D, and D).
//!
//! **The sets** (`--sets DIR`, with the count): `DIR/ID.sets.tsv` has a header line `ref` and the members of the unit's
//! first heaviest set, then a line for each completion counted (the first [`WRITTEN`] of them): its value, and its
//! members beyond the unit's promotions that are not in that set (`plus`) and the members of that set it lacks
//! (`minus`), comma separated, `-` for none.  `DIR/N.candidates.tsv` has the game's candidates, each with its home
//! frame and its maximal factors, as `wall anatomy --trace` writes them.

use super::anatomy::{suite_units, write_game_candidates};
use super::{beyond, Line, Options};
use crate::counters::cpu_seconds;
use crate::known::known_incumbent;
use crate::rng::Rng;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::bounds::max_additional;
use taxman::engine::{self, Control, EngineConfig, Start};
use taxman::game::Optimal;
use taxman::gf2::{Gf2Bound, Gf2Graph};
use taxman::matchbound::MatchGraph;
use taxman::model::{FrameGraph, FrameSet, Scratch};
use taxman::search::{
    payload_of, MakeHook, Prepared, SearchSpace, SearchTactics, StepEnd, StepHook, StepRecord, TaxSearcher, TaxUnit,
};

/// The options of `near`.
#[derive(Clone)]
pub struct Params {
    /// Count the completions within this much of the optimum (`--within`).
    pub within: u64,
    /// Estimate the counts from this many random descents instead of counting (`--probes`; 0: count), each to a node
    /// whose gap is at most `frontier` (`--frontier`).
    pub probes: usize,
    pub frontier: u64,
    /// The directory of the sets' files (`--sets`).
    pub sets: Option<PathBuf>,
}

impl Default for Params {
    fn default() -> Params {
        Params { within: 0, probes: 0, frontier: 300, sets: None }
    }
}

/// A decade of a subtree's size per this many points of its gap: the importance of a side of a branch.
const SCALE: f64 = 250.0;

/// The most completions a count writes to its sets' file.
const WRITTEN: u64 = 200_000;

/// The most steps of the count of a frontier node's completions; one stopped is counted as incomplete.
const FRONTIER_STEPS: u64 = 500_000;

// ---------------------------------------------------------------------------------------------------------
// What a unit's count or estimate gathers

/// The completions counted, by value, and the sets' file.
struct Sink<const W: usize> {
    by_value: BTreeMap<u64, u64>,
    count: u64,
    /// The unit's promotions and first heaviest set, which the file's lines are written against.
    promoted: Bits<W>,
    reference: Bits<W>,
    out: Option<BufWriter<File>>,
    error: Option<String>,
}

impl<const W: usize> Sink<W> {
    fn new(promoted: Bits<W>, reference: Bits<W>, out: Option<BufWriter<File>>) -> Sink<W> {
        let mut sink = Sink { by_value: BTreeMap::new(), count: 0, promoted, reference, out, error: None };
        let header = format!("ref\t{}\n", members(&reference));
        sink.write(&header);
        sink
    }

    fn write(&mut self, text: &str) {
        if let Some(out) = &mut self.out {
            if let Err(e) = out.write_all(text.as_bytes()) {
                self.error.get_or_insert(format!("the sets' file: {e}"));
            }
        }
    }

    fn record(&mut self, set: &Bits<W>) {
        let value = set.sum();
        *self.by_value.entry(value).or_insert(0) += 1;
        self.count += 1;
        if self.out.is_some() && self.count <= WRITTEN {
            let own = *set - self.promoted;
            let (plus, minus) = (members(&(own - self.reference)), members(&(self.reference - own)));
            self.write(&format!("{value}\t{plus}\t{minus}\n"));
        }
    }

    fn finish(mut self) -> Result<Sink<W>, String> {
        if let Some(out) = &mut self.out {
            out.flush().map_err(|e| format!("the sets' file: {e}"))?;
        }
        match self.error.take() {
            Some(e) => Err(e),
            None => Ok(self),
        }
    }
}

/// The members of `set`, comma separated, or `-`.
fn members<const W: usize>(set: &Bits<W>) -> String {
    if set.is_empty() {
        return "-".into();
    }
    set.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")
}

/// The estimate of the completions within each distance of `distances` of `opt`, with its standard error, from the
/// descents that reached the frontier (each its weight and its frontier node's count by value) of `probes` in all.
fn estimates(ends: &[(f64, BTreeMap<u64, u64>)], probes: usize, opt: u64, distances: &[u64]) -> Vec<(u64, f64, f64)> {
    let k = probes.max(1) as f64;
    distances
        .iter()
        .map(|&d| {
            let floor = opt.saturating_sub(d);
            let (sum, squares) = ends.iter().fold((0.0, 0.0), |(s, q), (weight, by_value)| {
                let x = weight * by_value.range(floor..).map(|(_, &c)| c as f64).sum::<f64>();
                (s + x, q + x * x)
            });
            let mean = sum / k;
            let variance = (squares / k - mean * mean).max(0.0);
            (d, mean, (variance / k).sqrt())
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------------------
// The count

/// The hook of the counting search: every child made at the floor is a completion, counted once, whether it is
/// explored or not (a child with nothing left to promote is cut as exact, and is a completion all the same).
struct Counter<const W: usize> {
    floor: u64,
    shared: Arc<Mutex<Counted<W>>>,
}

/// What the counting search's hook gathers: the completions, and the children made.
struct Counted<const W: usize> {
    sink: Sink<W>,
    children: u64,
}

impl<const W: usize> StepHook<W> for Counter<W> {
    fn step(&mut self, r: &StepRecord<W>, _: &FrameSet<W>, _: Option<&FrameSet<W>>) {
        if matches!(r.end, StepEnd::Child | StepEnd::Leaf | StepEnd::ChildCut(_)) {
            let mut counted = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
            counted.children += 1;
            let set = r.promoted | r.added;
            if set.sum() >= self.floor {
                counted.sink.record(&set);
            }
        }
    }

    fn counting(&self) -> bool {
        true
    }
}

/// What a count gave.
struct Count<const W: usize> {
    steps: u64,
    completed: bool,
    counted: Counted<W>,
}

/// Counts the completions of `unit` at `floor` (at least 1) by the search of `p` made to count, for at most `limit`
/// steps, into `sink`.
fn count<const W: usize>(
    p: &Prepared<W>,
    graph: &MatchGraph,
    unit: TaxUnit<W>,
    floor: u64,
    limit: Option<u64>,
    mut sink: Sink<W>,
) -> Result<Count<W>, String> {
    if unit.promoted.sum() >= floor {
        sink.record(&unit.promoted); // the unit's node itself: no child makes it
    }
    let shared = Arc::new(Mutex::new(Counted { sink, children: 0 }));
    let make = {
        let shared = Arc::clone(&shared);
        move || Box::new(Counter { floor, shared: Arc::clone(&shared) }) as Box<dyn StepHook<W>>
    };
    let space = SearchSpace { hooks: Some(&make as &MakeHook<W>), ..SearchSpace::new(p, graph) };
    let cfg = EngineConfig { threads: 1, node_limit: limit, control: Some(Control::new()), ..EngineConfig::default() };
    let begin = Start::Fresh { units: vec![unit], best: floor - 1, payload: payload_of(&Bits::<W>::EMPTY) };
    let out = engine::run(&cfg, begin, b"near", |_| TaxSearcher::new(&space)).map_err(|e| format!("{e:?}"))?;
    drop(space);
    drop(make);
    let counted = Arc::try_unwrap(shared)
        .map_err(|_| "the hooks outlived the search".to_owned())?
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner);
    Ok(Count { steps: out.stats.nodes, completed: out.completed, counted })
}

// ---------------------------------------------------------------------------------------------------------
// The estimate

/// A node of the estimate's tree: its promotions, their state and sum, its candidates, and the most promotions it can
/// add.
#[derive(Clone, Copy)]
struct Node<const W: usize> {
    state: FrameSet<W>,
    promoted: Bits<W>,
    sum: u64,
    cands: Bits<W>,
    max_add: u32,
}

/// What a step of a node's loop finds before it makes a child.
enum Open<const W: usize> {
    /// The loop ends; true if the node's promotions are a completion at the floor (its last one).
    End(bool),
    /// The node is on the frontier: its completions are counted exactly.
    Frontier,
    /// Every completion at the floor holds these members.
    Forced(Bits<W>),
    /// The member to branch on.
    Branch(u32),
}

/// The descents of the estimate's tree of a unit's completions at the floor, to its frontier: the GF(2) bound along
/// the path, and the steps taken.
struct Walk<'a, const W: usize> {
    model: &'a FrameGraph,
    max_total: u32,
    floor: u64,
    frontier: u64,
    g: Gf2Bound<'a, W>,
    scratch: Scratch<W>,
    steps: u64,
}

impl<'a, const W: usize> Walk<'a, W> {
    fn new(p: &'a Prepared<W>, graph: &'a Gf2Graph, floor: u64, frontier: u64) -> Walk<'a, W> {
        let mut g = Gf2Bound::new(graph);
        g.set_force_window(p.tactics.force_window);
        Walk { model: &p.model, max_total: p.max_total, floor, frontier, g, scratch: Scratch::new(), steps: 0 }
    }

    /// The unit's node, with the bound's levels started there.
    fn start(&mut self, root: &FrameSet<W>, unit: &TaxUnit<W>) -> Node<W> {
        let state =
            self.model.promote_all_using(&mut self.scratch, root, &unit.promoted).expect("the unit's promotions");
        let max_add = max_additional(self.model, self.max_total, &state, &unit.promoted, &unit.cands);
        self.g.reset(&unit.promoted, &unit.cands);
        Node { state, promoted: unit.promoted, sum: unit.promoted.sum(), cands: unit.cands, max_add }
    }

    /// What `node`'s loop does next, at the floor (see the module docs); its bound's level is on top.
    fn open(&mut self, node: &Node<W>) -> Open<W> {
        self.steps += 1;
        let need = self.floor.saturating_sub(node.sum);
        if node.cands.is_empty() || node.max_add == 0 {
            return Open::End(need == 0);
        }
        if node.cands.sum_largest(node.max_add) < need {
            return Open::End(false);
        }
        let eval = self.g.evaluate(node.max_add, (need > 0).then_some(need));
        if eval.value < need {
            return Open::End(false);
        }
        if need == 0 || eval.value - need <= self.frontier {
            return Open::Frontier;
        }
        if eval.chosen.is_empty() {
            // No candidate can be added: a common independent set of one member would weigh more than nothing.
            return Open::End(need == 0);
        }
        let forced = eval.forced & node.cands;
        if !forced.is_empty() {
            return Open::Forced(forced);
        }
        let pool = self
            .model
            .conflict_using(&mut self.scratch, &node.state, &eval.chosen, true)
            .filter(|k| !k.is_empty())
            .unwrap_or(eval.chosen);
        let g = &mut self.g;
        let x = if need > 0 {
            pool.iter().min_by_key(|&x| (g.split_without(x).unwrap_or(i64::MAX), u32::MAX - x))
        } else {
            pool.max()
        };
        Open::Branch(x.expect("a conflict has members"))
    }

    /// The child of `node` that promotes `added`, with the candidates `rest` narrowed to it; `None` if the promotion
    /// fails.  (Several are promoted together, each frame checked once, as the search's `make_child` does.)
    fn child(&mut self, node: &Node<W>, added: &Bits<W>, rest: &Bits<W>) -> Option<Node<W>> {
        let model = self.model;
        let state = if added.len() == 1 {
            let mut state = node.state;
            let x = added.max().expect("one member");
            model.promote_in_place_using(&mut self.scratch, &mut state, x).then_some(state)?
        } else {
            let state = model.promote_all_using(&mut self.scratch, &node.state, added)?;
            let fits = added.iter().all(|x| {
                let (into, out) = model.frames_touched(x);
                let frames = state.frames();
                frames[into].freedoms >= 0 && out.is_none_or(|out| frames[out].freedoms >= 0)
            });
            fits.then_some(state)?
        };
        let promoted = node.promoted | *added;
        let cands = model.narrow(&state, rest);
        let max_add = max_additional(model, self.max_total, &state, &promoted, &cands);
        Some(Node { state, promoted, sum: node.sum + added.sum(), cands, max_add })
    }

    /// One random descent of the tree from `unit`'s node (see the module docs) to a node of the frontier, with the
    /// descent's weight, or `None` if the descent ends with no completion at the floor.
    fn descend(&mut self, root: &FrameSet<W>, unit: &TaxUnit<W>, rng: &mut Rng) -> Option<(Node<W>, f64)> {
        let mut node = self.start(root, unit);
        let mut weight = 1.0;
        loop {
            match self.open(&node) {
                Open::End(true) | Open::Frontier => return Some((node, weight)),
                Open::End(false) => return None,
                Open::Forced(forced) => {
                    let rest = node.cands - forced;
                    node = self.child(&node, &forced, &rest)?;
                    self.g.push_all(&forced, &node.cands);
                }
                Open::Branch(x) => {
                    let need = self.floor.saturating_sub(node.sum);
                    let rest = node.cands.without(x);
                    // The node without `x` holds `P` itself when it is at the floor already.
                    let without = if need == 0 {
                        Some(0)
                    } else {
                        let value = self.g.value_excluding(x, node.max_add, u64::MAX);
                        (value >= need).then(|| value - need)
                    };
                    let added = Bits::EMPTY.with(x);
                    let child = self.child(&node, &added, &rest);
                    let with = child.as_ref().and_then(|c| {
                        self.g.push_all(&added, &c.cands);
                        let need = self.floor.saturating_sub(c.sum);
                        let value = if c.cands.is_empty() || c.max_add == 0 {
                            0
                        } else {
                            self.g.evaluate(c.max_add, None).value
                        };
                        (value >= need).then(|| value - need)
                    });
                    let p = match (with, without) {
                        (None, None) => return None,
                        (Some(_), None) => 1.0,
                        (None, Some(_)) => 0.0,
                        (Some(a), Some(b)) => 1.0 / (1.0 + 10f64.powf((b as f64 - a as f64) / SCALE)),
                    };
                    if rng.unit() < p {
                        weight /= p;
                        node = child.expect("a live side");
                    } else {
                        weight /= 1.0 - p;
                        if child.is_some() {
                            self.g.pop();
                        }
                        node.cands = rest;
                        self.g.remove_candidate(x);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The command

/// The tactics of a count: the search's, without the rule of symmetric pairs; the GF(2) bound is needed.
fn counting_tactics(tactics: SearchTactics) -> Result<SearchTactics, String> {
    if !tactics.gf2 {
        return Err("near counts with the GF(2) bound (the tactic gf2)".into());
    }
    Ok(SearchTactics { symmetry: false, ..tactics })
}

/// `near` on game `n` at the set width `W`.
fn play<const W: usize>(n: u32, o: &Options, optimal: &Optimal) -> Result<(), String> {
    let params = &o.near;
    let board = Board::<W>::for_n(n);
    let p = board.prepare(counting_tactics(o.tactics)?)?;
    let units = if o.exchanges.root {
        vec![(n.to_string(), TaxUnit { promoted: Bits::EMPTY, cands: p.candidates })]
    } else {
        let suite = o.params.suite.as_ref().ok_or("near needs --suite or --root")?;
        suite_units::<W>(suite, n, &o.anatomy.ids)?
    };
    let (_, opt) = known_incumbent(&board, &p, optimal)?.ok_or(format!("n={n}: needs its known optimum (--known)"))?;
    let floor = opt.saturating_sub(params.within).max(1);
    let graph = MatchGraph::new(&p.model, &p.root);
    let gf2 = Gf2Graph::new(&p.model, &p.root, &p.candidates);
    if let Some(dir) = &params.sets {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        write_game_candidates(&dir.join(format!("{n}.candidates.tsv")), &p)?;
    }
    let mut rng = o.params.lns.rng(n);
    for (id, unit) in units {
        // The unit's bound and first heaviest set, which the sets' file writes against.
        let mut walk = Walk::new(&p, &gf2, floor, params.frontier);
        let node = walk.start(&p.root, &unit);
        let first = walk.g.evaluate(node.max_add, None);
        let mut line = Line::of(n);
        line.text("id", &id)
            .field("depth", unit.promoted.len())
            .field("cands", unit.cands.len())
            .field("opt", opt)
            .field("within", params.within)
            .field("floor", floor)
            .field("bound", unit.promoted.sum() + first.value);
        let before = cpu_seconds();
        let cpu = || before.zip(cpu_seconds()).map_or(0.0, |(a, b)| b - a);
        if params.probes == 0 {
            let out = match &params.sets {
                Some(dir) => {
                    let path = dir.join(format!("{id}.sets.tsv"));
                    Some(BufWriter::new(File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?))
                }
                None => None,
            };
            let done = count(&p, &graph, unit, floor, o.nodes, Sink::new(unit.promoted, first.chosen, out))?;
            let sink = done.counted.sink.finish()?;
            let pairs = sink.by_value.iter().rev().map(|(&v, &c)| format!("[{},{c}]", opt.saturating_sub(v)));
            line.field("steps", done.steps)
                .seconds("cpu_s", cpu())
                .maybe("best", sink.by_value.keys().next_back())
                .field("children", done.counted.children)
                .field("completed", done.completed)
                .field("count", sink.count)
                .list("by_distance", pairs);
        } else {
            let (mut ends, mut frontier_steps, mut incomplete) = (Vec::new(), 0, 0);
            for _ in 0..params.probes {
                let Some((node, weight)) = walk.descend(&p.root, &unit, &mut rng) else { continue };
                let frontier = TaxUnit { promoted: node.promoted, cands: node.cands };
                let sink = Sink::new(unit.promoted, Bits::EMPTY, None);
                let done = count(&p, &graph, frontier, floor, Some(FRONTIER_STEPS), sink)?;
                frontier_steps += done.steps;
                incomplete += u64::from(!done.completed);
                if done.counted.sink.count > 0 {
                    ends.push((weight, done.counted.sink.by_value));
                }
            }
            let mut distances: Vec<u64> = (0..=params.within).step_by(10).collect();
            if distances.last() != Some(&params.within) {
                distances.push(params.within);
            }
            let rows = estimates(&ends, params.probes, opt, &distances);
            let best = ends.iter().filter_map(|(_, by_value)| by_value.keys().next_back()).max();
            line.field("steps", walk.steps)
                .seconds("cpu_s", cpu())
                .maybe("best", best)
                .field("probes", params.probes)
                .field("frontier", params.frontier)
                .field("frontier_steps", frontier_steps)
                .field("incomplete", incomplete)
                .field("hits", ends.len())
                .list("estimates", rows.iter().map(|(d, e, se)| format!("[{d},{e:.1},{se:.1}]")));
        }
        line.print();
    }
    Ok(())
}

/// `near` on each game of `o`.
pub fn run(o: &Options, optimal: &Optimal) -> Result<(), String> {
    for &n in &o.games {
        taxman::with_width!(n, W => play::<W>(n, o, optimal), else Err(beyond(n)))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A game small enough to list every promotable set (25 candidates, 30,435 sets), with a search that branches.
    const N: u32 = 150;

    /// Whether `set` can be promoted from the root of `p`: with at most `max_total` members, every frame winnable and
    /// none with negative freedoms.
    fn promotable<const W: usize>(p: &Prepared<W>, scratch: &mut Scratch<W>, set: &Bits<W>) -> bool {
        set.len() <= p.max_total
            && p.model
                .promote_all_using(scratch, &p.root, set)
                .is_some_and(|state| state.frames().iter().all(|f| f.freedoms >= 0))
    }

    /// Every promotable set of `p`'s candidates, by value: each promotable set extended by larger candidates, each
    /// set checked from the root (a subset of a promotable set is promotable, so none is missed).
    fn every_set<const W: usize>(p: &Prepared<W>) -> BTreeMap<u64, u64> {
        let mut scratch = Scratch::new();
        let cands = p.candidates.to_vec();
        let mut found = BTreeMap::new();
        let mut stack = vec![(Bits::<W>::EMPTY, 0)];
        while let Some((set, next)) = stack.pop() {
            *found.entry(set.sum()).or_insert(0) += 1;
            for (i, &x) in cands.iter().enumerate().skip(next) {
                if promotable(p, &mut scratch, &set.with(x)) {
                    stack.push((set.with(x), i + 1));
                }
            }
        }
        found
    }

    fn check<const W: usize>() {
        let board = Board::<W>::for_n(N);
        let p = board.prepare(counting_tactics(SearchTactics::default()).expect("gf2")).expect("prepare");
        let (_, opt) = known_incumbent(&board, &p, Optimal::built_in()).expect("known").expect("a known game");
        let all = every_set(&p);
        assert_eq!(all.keys().next_back(), Some(&opt), "the best set is the optimum");
        let graph = MatchGraph::new(&p.model, &p.root);
        let root = TaxUnit { promoted: Bits::EMPTY, cands: p.candidates };
        // The count is every set at the floor, by value: near the optimum, and every set but the empty one.
        for floor in [opt, opt - 10, opt - 50, 1] {
            let done = count(&p, &graph, root, floor, None, Sink::new(Bits::EMPTY, Bits::EMPTY, None)).expect("count");
            assert!(done.completed);
            let expected: BTreeMap<u64, u64> = all.range(floor..).map(|(&v, &c)| (v, c)).collect();
            assert_eq!(done.counted.sink.by_value, expected, "{} below {opt}", opt - floor);
        }
        // The descents reach promotable sets, and the estimate is near the count: with the frontier at a gap of 0 (the
        // descents go down to the sets themselves, but for the nodes at the floor) and a little above.
        let within = 50;
        let exact: f64 = all.range(opt - within..).map(|(_, &c)| c as f64).sum();
        let gf2 = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let mut scratch = Scratch::new();
        for frontier in [0, 20] {
            let mut walk = Walk::new(&p, &gf2, opt - within, frontier);
            let (mut rng, probes, mut ends) = (Rng(1), 400, Vec::new());
            for _ in 0..probes {
                let Some((node, weight)) = walk.descend(&p.root, &root, &mut rng) else { continue };
                assert!(promotable(&p, &mut scratch, &node.promoted));
                let unit = TaxUnit { promoted: node.promoted, cands: node.cands };
                let done = count(&p, &graph, unit, opt - within, None, Sink::new(Bits::EMPTY, Bits::EMPTY, None));
                ends.push((weight, done.expect("count").counted.sink.by_value));
            }
            let (_, estimate, error) = estimates(&ends, probes, opt, &[within])[0];
            let close = (estimate - exact).abs() <= 3.0 * error.max(1.0);
            assert!(close, "frontier {frontier}: {estimate} ± {error} against {exact}");
        }
    }

    #[test]
    fn the_count_is_every_promotable_set_at_the_floor_and_the_estimate_is_near_it() {
        taxman::with_width!(N, W => check::<W>(), else panic!("{}", beyond(N)));
    }
}
