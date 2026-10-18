//! `wall decompose`: A5's cheapest experiment (solver/docs/AGENDA.md, "A5. Decompose across the frames"): do the
//! conflicts of a node that heads a big subtree split, across some home, into two sides that do not interact, so that
//! an AND/OR search could solve the sides apart?  The question, the numbers and the verdict are in
//! solver/notebook/decomposing-across-the-frames-a5s-cheapest-experiment-2026-10/README.md, whose `analyze.py` reads
//! the lines.
//!
//! The frames form a path: a candidate of home `h` is a factor of frame `h` and, promoted, a move of frame `h - 1`,
//! so a conflict (a set of candidates that cannot all be promoted, one frame's residual: `FrameGraph::conflict_using`)
//! has members of two adjacent homes, `f` and `f + 1` for frame `f`, and never more.  Fixing the open candidates of one
//! home `h` therefore leaves the conflicts of frames `0..h-1` (members of homes `1..h`) on one side and those of frames
//! `h..` (members of homes `h..`) on the other: the *separator* `h`.  Against a separator `h` a conflict's open
//! members lie in one of seven places ([`Place`]): all below `h` (*left*), all above (*right*), some at `h` with the
//! others below (*touch-left*) or above (*touch-right*), only at `h` (*touch-only*), below and above (*cross*, which
//! no conflict does; a count that is not zero would be a bug), or none open (*decided*).
//!
//! ```text
//! wall decompose --known FILE [--fixtures DIR [--keep K]] [--pauses P --span STEPS] [--unit-cap STEPS]
//!     [--best-cap STEPS] [--and-or-cap K [--and-or-min STEPS]] N...
//! ```
//!
//! * For each game `N` with a known optimum, a line `{"kind":"game",...}`, a line `{"kind":"node",...}` for each node
//!   of the sources below, and a closing line with the counts.  The incumbent a node must beat is the known optimum,
//!   so the nodes are those of a proof, as the probe's are.  Build it with `--profile test` (opt-level 2, no LTO):
//!   the proof of 2170 takes a minute.
//! * The nodes: `--fixtures DIR` (the probe's `nNNNN.nodes`, thinned to `--keep K`) and the *pauses* of the game's
//!   proof: the proof from the optimum on one thread, stopped after every `--span` steps (the engine counts in blocks
//!   of 1,024), `--pauses P` times, each pause's open units read from its checkpoint (the stack the proof was working
//!   through: the root's remaining siblings, then the next level's, down to the deepest), and the root itself.  Units
//!   that are the same are measured once; one that today's bound cuts has a line of `"cut":true` and nothing else.
//! * A line of an open node: `id`, `source`, `depth` (its promotions) and `cands`, `need` and `today` (the GF(2)
//!   bound on its completions) and `gap` (`today - need`, at least 0), `open` (its candidates by home, index `h` for
//!   home `h`; index 0 counts the rest), `subtree` (the engine's steps of the node's subtree solved from the
//!   optimum on one thread, fresh tables, the default tactics; at most `--unit-cap` steps, then `"completed":false`)
//!   and `solve_cpu_s`; `s` (the heaviest set of today's bound: its size, and `s_homes`, `s_weights` by home) and
//!   `leaf` (it can be promoted: the node is settled); and two spreads, `cores` (the cores that [`cores_of`] finds for
//!   the heaviest set, up to six a frame) and `learned` (the nogoods that the subtree's searcher held when it ended, a
//!   sample of the conflicts the subtree met: with the default forgetting, those of its last 50,000 steps).  A
//!   [`Spread`] has `n` (its conflicts), `frames` (by the frame they fail in; one failing in several counts in each),
//!   `places` (for each separator home `h = 1, 2, ...` the counts of left, right, touch-left, touch-right, touch-only
//!   and cross), `width` (for each home the number of distinct open members in any conflict: a separator `h` has to
//!   decide `width[h]` candidates) and `span` (the widest spread of homes of a conflict: 1 or less).
//! * With `--best-cap STEPS` also the node's best completion `F*` (its own optimum, `best` its sum), if it is known:
//!   the optimum's own, for a node that holds it, else found by an optimizing solve from the greedy completion that
//!   ends within that many steps.  From it come the bound's error by home, `excess_homes` (the weight of the heaviest
//!   set at home `h` less that of `F*`; they sum to `today - best`, the gap the bound leaves to the node's own optimum:
//!   the split of the gap that the anatomy of the gap, solver/notebook/the-anatomy-of-the-gap-2026-10/README.md, made at
//!   the roots), `ideal` (the engine's steps to prove `F*` optimal from `F*`), and for each separator `h` with open
//!   candidates on both sides the *oracle separator*: the node with the candidates of home `h` fixed as `F*` has them
//!   (those of `F*` promoted, the rest left out), and each side, the open candidates below `h` and those above,
//!   solved apart from the incumbent `F*`'s own part of it (an AND/OR search whose separator costs nothing).  A `sides`
//!   entry has `h`, the sides' gaps `gl` and `gr` (each side's GF(2) bound less its part of `F*`) and their steps `tl`
//!   and `tr` (`completed` false if `--unit-cap` stopped one).
//! * With `--and-or-cap K`, for the nodes of at least `--and-or-min` steps (default 500) that a home splits, the AND/OR
//!   search itself ([`Or`]) with the home that splits the node's learned conflicts most evenly as its separator, once
//!   deciding the home's candidates largest first (`and_or`) and once by the conflict of the heaviest set
//!   (`and_or_conflict`): `or_nodes` (the OR part's evaluations of the bound, stopped at `K`: `complete` false), the
//!   `survivors` (assignments of the home that pass the bound; a lower bound if `complete` is false), `side_steps`
//!   (the steps of the two sides of a sample of eight survivors, in the cheaper order) and the `estimate` of the whole.

use super::case::{Case, Clock, Played};
use super::fields::cores::{cores_of, frame_with, residual};
use super::nogoods::{harvesting, sorted, Sink};
use super::sample::fixture_nodes;
use super::Line;
use crate::known::known_incumbent;
use crate::lns::run_unit_with;
use std::collections::{BTreeSet, HashSet};
use std::time::Duration;
use taxman::bits::Bits;
use taxman::bounds::max_additional;
use taxman::engine::{self, Control, EngineConfig, Start, Unit};
use taxman::gf2::Gf2Bound;
use taxman::json::Json;
use taxman::model::{FrameGraph, FrameSet, Scratch};
use taxman::search::{payload_of, TaxSearcher, TaxUnit};

/// The cores of a node's heaviest set that are looked for in each frame.
const CORES_PER_FRAME: usize = 6;
/// The steps at which the solve of one side of a survivor of the AND/OR search is stopped (it then counts for the limit).
const SIDE_CAP: u64 = 300_000;

// ---------------------------------------------------------------------------------------------------------
// Where a conflict lies against a separator

/// Where the open members of a conflict lie against the separator home `h`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Place {
    Left,
    Right,
    TouchLeft,
    TouchRight,
    TouchOnly,
    Cross,
    Decided,
}

impl Place {
    /// The places that a spread counts, in the order of its `places` arrays.
    const COUNTED: [Place; 6] =
        [Place::Left, Place::Right, Place::TouchLeft, Place::TouchRight, Place::TouchOnly, Place::Cross];

    /// The place of a conflict whose open members have the homes `homes`, against the separator `h`.
    pub(super) fn of(homes: &[usize], h: usize) -> Place {
        let below = homes.iter().any(|&x| x < h);
        let at = homes.contains(&h);
        let above = homes.iter().any(|&x| x > h);
        match (below, at, above) {
            (true, _, true) => Place::Cross,
            (true, false, false) => Place::Left,
            (false, false, true) => Place::Right,
            (true, true, false) => Place::TouchLeft,
            (false, true, true) => Place::TouchRight,
            (false, true, false) => Place::TouchOnly,
            (false, false, false) => Place::Decided,
        }
    }
}

/// A set of conflicts of one node (its cores, or the nogoods its subtree learned) seen from every separator home.
pub(super) struct Spread {
    /// The number of frames; the homes are `1..frames`.
    frames: usize,
    total: u64,
    /// The conflicts by the frame they fail in.
    by_frame: Vec<u64>,
    /// For each separator home `h` (index `h`), the conflicts at each of the [`Place::COUNTED`].
    places: Vec<[u64; 6]>,
    /// For each home, the distinct open members of that home that are in some conflict.
    members: Vec<BTreeSet<u32>>,
    /// The widest spread of homes (largest less smallest) of a conflict's open members.
    span: usize,
}

impl Spread {
    pub(super) fn new(frames: usize) -> Spread {
        Spread {
            frames,
            total: 0,
            by_frame: vec![0; frames],
            places: vec![[0; 6]; frames],
            members: vec![BTreeSet::new(); frames],
            span: 0,
        }
    }

    /// Adds a conflict that fails in `failing` frames and has the open members `members`, each with its home.
    pub(super) fn add(&mut self, failing: &[usize], members: &[(u32, usize)]) {
        let homes: Vec<usize> = members.iter().map(|&(_, home)| home).collect();
        let (Some(&lowest), Some(&highest)) = (homes.iter().min(), homes.iter().max()) else { return };
        self.total += 1;
        self.span = self.span.max(highest - lowest);
        for &frame in failing {
            self.by_frame[frame] += 1;
        }
        for h in 1..self.frames {
            let place = Place::of(&homes, h);
            if let Some(i) = Place::COUNTED.iter().position(|&p| p == place) {
                self.places[h][i] += 1;
            }
        }
        for &(x, home) in members {
            self.members[home].insert(x);
        }
    }

    /// The conflicts that cross the separator `h`.
    pub(super) fn crossing(&self) -> u64 {
        (1..self.frames).map(|h| self.places[h][5]).sum()
    }

    /// The home that splits the conflicts most evenly, with the conflicts on each side once its candidates are fixed:
    /// the one whose smaller side is largest (the lowest home of them), if both sides of any hold a conflict.
    pub(super) fn best_split(&self) -> Option<(usize, u64, u64)> {
        let sides = |h: usize| (self.places[h][0] + self.places[h][2], self.places[h][1] + self.places[h][3]);
        (1..self.frames)
            .map(|h| (h, sides(h)))
            .filter(|&(_, (left, right))| left > 0 && right > 0)
            .max_by_key(|&(h, (left, right))| (left.min(right), std::cmp::Reverse(h)))
            .map(|(h, (left, right))| (h, left, right))
    }

    pub(super) fn json(&self) -> Json {
        Json::object([
            ("n", Json::from(self.total)),
            ("frames", Json::array(self.by_frame.iter().copied())),
            ("places", Json::array((1..self.frames).map(|h| Json::array(self.places[h].iter().copied())))),
            ("width", Json::array(self.members.iter().map(|m| m.len()))),
            ("span", Json::from(self.span)),
        ])
    }
}

// ---------------------------------------------------------------------------------------------------------
// The conflicts of a node

/// The frames of `root` in which the candidates `set` cannot all be promoted (the residual of the frame with `set`
/// applied is not empty): the frames a nogood fails in.
fn failing_frames<const W: usize>(model: &FrameGraph, root: &FrameSet<W>, set: &[u32]) -> Vec<usize> {
    (0..root.len())
        .filter(|&i| {
            let (movers, pool) = frame_with(model, root, set, i);
            !residual(model, &movers, &pool).is_empty()
        })
        .collect()
}

/// The open members of a conflict with their homes.
fn with_homes(model: &FrameGraph, members: impl IntoIterator<Item = u32>) -> Vec<(u32, usize)> {
    members.into_iter().map(|x| (x, model.home(x))).collect()
}

/// The spread of the cores of the heaviest set `set` of the node whose state is `state`.
fn core_spread<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, set: &Bits<W>) -> Spread {
    let mut spread = Spread::new(state.len());
    for core in cores_of(model, state, &set.to_vec(), CORES_PER_FRAME) {
        let members = core.promoted_in.iter().chain(&core.promoted_out).copied();
        spread.add(&[core.frame], &with_homes(model, members));
    }
    spread
}

/// The spread of the nogoods `learned` by the subtree of a node with promotions `promoted`: their open members are
/// the ones outside the promotions.
fn nogood_spread<const W: usize>(
    model: &FrameGraph,
    root: &FrameSet<W>,
    promoted: &Bits<W>,
    learned: &[Bits<W>],
) -> Spread {
    let mut spread = Spread::new(root.len());
    for nogood in learned {
        let open = nogood.difference(promoted);
        if open.is_empty() {
            continue;
        }
        let failing = failing_frames(model, root, &nogood.to_vec());
        spread.add(&failing, &with_homes(model, open.iter()));
    }
    spread
}

/// The candidates of `set` by home, index `h` for home `h` (index 0 for the rest), as counts and as total weights.
fn by_home<const W: usize>(model: &FrameGraph, frames: usize, set: &Bits<W>) -> (Vec<u64>, Vec<u64>) {
    let (mut counts, mut weights) = (vec![0; frames], vec![0; frames]);
    for x in set.iter() {
        let home = model.home(x).min(frames - 1);
        counts[home] += 1;
        weights[home] += u64::from(x);
    }
    (counts, weights)
}

// ---------------------------------------------------------------------------------------------------------
// A node

/// What a node is measured against: the optimum, the incumbent that a proof is seeded with.
struct Incumbent<const W: usize> {
    set: Bits<W>,
    sum: u64,
}

/// What a completion of a node with promotions `promoted` must reach to beat the incumbent worth `sum`.
fn need_of<const W: usize>(sum: u64, promoted: &Bits<W>) -> u64 {
    (sum + 1).saturating_sub(promoted.sum())
}

/// What a solve of a unit came to.
struct Solved<const W: usize> {
    best: Bits<W>,
    steps: u64,
    completed: bool,
    cpu_s: f64,
    learned: Vec<Bits<W>>,
}

/// `unit` solved on one thread from the incumbent `start`, with fresh tables and the default forgetting, for at most
/// `cap` steps, and what the searcher held at its end.
fn solve<const W: usize>(case: &Case<'_, W>, unit: TaxUnit<W>, start: &Bits<W>, cap: Option<u64>) -> Solved<W> {
    let sink = Sink::<W>::default();
    let clock = Clock::start();
    let ran = run_unit_with(&Control::new(), unit, start, cap, |_| harvesting(case.space, &sink));
    Solved { best: ran.set, steps: ran.nodes, completed: ran.completed, cpu_s: clock.elapsed(), learned: sorted(sink) }
}

/// What a solve of a unit under a floor came to.
struct Above {
    steps: u64,
    /// The best promotion sum found: the floor if the unit holds no set above it.
    best: u64,
}

/// `unit` solved on one thread under the floor `floor` (a promotion sum: the incumbent is worth that and is no set), for
/// at most `cap` steps.
fn solve_above<const W: usize>(case: &Case<'_, W>, unit: TaxUnit<W>, floor: u64, cap: Option<u64>) -> Above {
    let cfg = EngineConfig { threads: 1, node_limit: cap, progress: None, ..EngineConfig::default() };
    let start = Start::Fresh { units: vec![unit], best: floor, payload: payload_of(&Bits::<W>::EMPTY) };
    let out = engine::run(&cfg, start, b"wall", |_| TaxSearcher::new(case.space)).expect("the engine");
    Above { steps: out.stats.nodes, best: out.best }
}

// ---------------------------------------------------------------------------------------------------------
// An AND/OR search with a home as the separator

/// The search that A5 proposes, with the separator home `h`, run on a node of a proof: the **OR** part decides the open
/// candidates of home `h` one at a time (the largest first, or the largest in the conflict of the node's heaviest
/// set), in a child that promotes it and one that leaves it out,
/// and cuts a child when the GF(2) bound of the node (the sides' candidates still open) is below what the node needs;
/// the **AND** part, for each assignment of home `h` that survives, solves the two sides, the open candidates below
/// `h` and those above, apart.  The sides are solved by the engine: the first of them under the floor that would
/// leave the node short if the other side reached its bound, and, if it reaches that, the second under the floor its
/// optimum leaves.  The cost is the OR part's evaluations of the bound, plus the survivors times the sides' steps; the
/// sides' steps are measured for a sample of the survivors, in the cheaper order of the two sides (as if the search
/// knew it).
struct Or<'t, 'g, 'c, const W: usize> {
    case: &'t Case<'c, W>,
    tools: &'t mut Tools<'g, W>,
    incumbent: u64,
    cap: u64,
    /// Whether the next candidate to decide is chosen by the conflict of the node's heaviest set.
    by_conflict: bool,
    nodes: u64,
    survivors: u64,
    sample: usize,
    kept: Vec<Bits<W>>,
    seed: u64,
}

impl<const W: usize> Or<'_, '_, '_, W> {
    /// Decides the home's candidates `undecided` in the node of `state` and `promoted`, whose sides' candidates are
    /// `others`.  Stops when the limit on evaluations is reached.
    fn decide(&mut self, state: &FrameSet<W>, promoted: &Bits<W>, undecided: Bits<W>, others: &Bits<W>) {
        if self.nodes >= self.cap {
            return;
        }
        let (p, model) = (self.case.p, &self.case.p.model);
        let cands = model.narrow(state, &(*others | undecided));
        let max_add = max_additional(model, p.max_total, state, promoted, &cands);
        self.tools.gf2.reset(promoted, &cands);
        let eval = self.tools.gf2.evaluate(max_add, None);
        self.nodes += 1;
        if eval.value < need_of(self.incumbent, promoted) {
            return;
        }
        let undecided = undecided & cands;
        // The candidate to decide next: the largest of the home's candidates, or, by conflict, the largest of those in
        // the conflict of the heaviest set, else the largest in the heaviest set, else the largest.
        let pick = if self.by_conflict {
            let conflict = model.conflict_using(&mut self.tools.scratch, state, &eval.chosen, true);
            conflict.and_then(|k| (k & undecided).max()).or_else(|| (eval.chosen & undecided).max())
        } else {
            None
        };
        let Some(x) = pick.or_else(|| undecided.max()) else {
            self.survivors += 1;
            // A sample of the survivors (reservoir sampling with a fixed generator).
            if self.kept.len() < self.sample {
                self.kept.push(*promoted);
            } else {
                self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let slot = (self.seed >> 33) % self.survivors;
                if (slot as usize) < self.sample {
                    self.kept[slot as usize] = *promoted;
                }
            }
            return;
        };
        if let Some(next) = model.promote(state, x) {
            self.decide(&next, &promoted.with(x), undecided.without(x), others);
        }
        self.decide(state, promoted, undecided.without(x), others);
    }

    /// The steps that solving the two sides of the survivor `promoted` takes, the cheaper order of the two.
    fn sides(&mut self, promoted: &Bits<W>, h: usize, cands: &Bits<W>) -> Result<u64, String> {
        let (p, model) = (self.case.p, &self.case.p.model);
        let state = model
            .promote_all_using(&mut self.tools.scratch, &p.root, promoted)
            .ok_or("a survivor of the separator cannot be promoted")?;
        let need = need_of(self.incumbent, promoted);
        let mut side = |below: bool| {
            let cands = model.narrow(&state, &on_side(model, cands, h, below));
            let max_add = max_additional(model, p.max_total, &state, promoted, &cands);
            self.tools.gf2.reset(promoted, &cands);
            (TaxUnit { promoted: *promoted, cands }, self.tools.gf2.evaluate(max_add, None).value)
        };
        let (left, right) = (side(true), side(false));
        let base = promoted.sum();
        // Solves `first` for a completion of at least `need - other` (the bound of the other side), then `second` for
        // what the first's optimum leaves; the steps of both, the second only if the first reached its target.
        let order = |first: &(TaxUnit<W>, u64), second: &(TaxUnit<W>, u64)| {
            let target = need.saturating_sub(second.1).max(1);
            let one = solve_above(self.case, first.0, base + target - 1, Some(SIDE_CAP));
            if one.best < base + target {
                return one.steps;
            }
            let rest = need.saturating_sub(one.best - base).max(1);
            one.steps + solve_above(self.case, second.0, base + rest - 1, Some(SIDE_CAP)).steps
        };
        Ok(order(&left, &right).min(order(&right, &left)))
    }
}

/// The separator of an AND/OR search: the home, the evaluations at which the OR part stops, and whether it decides the
/// home's candidates by conflict.
#[derive(Clone, Copy)]
struct Separator {
    h: usize,
    cap: u64,
    by_conflict: bool,
}

/// The AND/OR search with a separator on the node `unit`, deciding the home's candidates largest first or by
/// conflict: see [`Or`].  The line has the evaluations of the OR part
/// (`or_nodes`; `complete` false if the limit stopped it, then a lower bound), its survivors, the steps of the sides of a
/// sample of them, and the estimate of the whole: the evaluations plus the survivors times the sample's mean.
fn and_or<const W: usize>(
    case: &Case<'_, W>,
    tools: &mut Tools<'_, W>,
    incumbent: &Incumbent<W>,
    unit: &TaxUnit<W>,
    state: &FrameSet<W>,
    separator: Separator,
) -> Result<Json, String> {
    let Separator { h, cap, by_conflict } = separator;
    let model = &case.p.model;
    let home: Bits<W> = unit.cands.iter().filter(|&x| model.home(x) == h).collect();
    let others = unit.cands.difference(&home);
    let mut or = Or {
        case,
        tools,
        incumbent: incumbent.sum,
        cap,
        by_conflict,
        nodes: 0,
        survivors: 0,
        sample: 8,
        kept: Vec::new(),
        seed: 1,
    };
    or.decide(state, &unit.promoted, home, &others);
    let complete = or.nodes < cap;
    let kept = std::mem::take(&mut or.kept);
    let mut steps = Vec::new();
    for survivor in &kept {
        steps.push(or.sides(survivor, h, &unit.cands)?);
    }
    let mean = steps.iter().sum::<u64>() as f64 / steps.len().max(1) as f64;
    let estimate = or.nodes as f64 + or.survivors as f64 * mean;
    Ok(Json::object([
        ("h", Json::from(h)),
        ("or_nodes", Json::from(or.nodes)),
        ("complete", Json::from(complete)),
        ("survivors", Json::from(or.survivors)),
        ("side_steps", Json::array(steps)),
        ("estimate", Json::from(estimate)),
    ]))
}

/// The promotions that greedy adds to the node (`promoted`, in `state`): from the largest candidate down, promote
/// every one that can be, narrowing the candidates after each.
fn greedy_completion<const W: usize>(
    model: &FrameGraph,
    scratch: &mut Scratch<W>,
    state: &FrameSet<W>,
    cands: &Bits<W>,
) -> Bits<W> {
    let (mut state, mut cands, mut added) = (*state, *cands, Bits::EMPTY);
    let mut next = cands.max();
    while let Some(x) = next {
        if model.promote_in_place_using(scratch, &mut state, x) {
            cands = model.narrow(&state, &cands);
            added.insert(x);
        }
        next = cands.max_below(x);
    }
    added
}

/// The best completion of a node, by an optimizing solve from the greedy completion (no incumbent from outside), if
/// it ends within `cap` steps: the set of its additional promotions.
fn best_completion<const W: usize>(
    case: &Case<'_, W>,
    scratch: &mut Scratch<W>,
    state: &FrameSet<W>,
    unit: &TaxUnit<W>,
    cap: u64,
) -> Option<Bits<W>> {
    let start = unit.promoted | greedy_completion(&case.p.model, scratch, state, &unit.cands);
    let solved = solve(case, *unit, &start, Some(cap));
    solved.completed.then(|| solved.best.difference(&unit.promoted))
}

/// The best completion of a node that holds the optimum: the optimum's other promotions.
fn known_completion<const W: usize>(incumbent: &Incumbent<W>, unit: &TaxUnit<W>) -> Option<Bits<W>> {
    let rest = incumbent.set.difference(&unit.promoted);
    (unit.promoted.is_subset(&incumbent.set) && rest.is_subset(&unit.cands)).then_some(rest)
}

/// The members of `set` whose home is below `h` (`below`) or above it.
fn on_side<const W: usize>(model: &FrameGraph, set: &Bits<W>, h: usize, below: bool) -> Bits<W> {
    set.iter().filter(|&x| model.home(x) != h && (model.home(x) < h) == below).collect()
}

/// The oracle separators of a node whose best completion is `best`: for each home `h` with open candidates on both
/// sides, the node with the candidates of home `h` fixed as `best` has them, and the two sides solved apart, each from
/// `best`'s part of it.
fn oracle_sides<const W: usize>(
    case: &Case<'_, W>,
    tools: &mut Tools<'_, W>,
    caps: &Caps,
    unit: &TaxUnit<W>,
    best: &Bits<W>,
) -> Result<Vec<Json>, String> {
    let (p, model) = (case.p, &case.p.model);
    let mut out = Vec::new();
    for h in 1..p.root.len() {
        let (below, above) = (on_side(model, &unit.cands, h, true), on_side(model, &unit.cands, h, false));
        if below.is_empty() || above.is_empty() {
            continue;
        }
        let at_h: Bits<W> = best.iter().filter(|&x| model.home(x) == h).collect();
        let fixed = unit.promoted | at_h;
        let state = model
            .promote_all_using(&mut tools.scratch, &p.root, &fixed)
            .ok_or_else(|| format!("n={}: the separator {h} of a best completion cannot be promoted", case.n()))?;
        let mut side = |cands: &Bits<W>, part: Bits<W>| -> Result<(i64, u64, bool), String> {
            let cands = model.narrow(&state, cands);
            if !part.is_subset(&cands) {
                return Err(format!("n={}: a best completion's part is not among its side's candidates", case.n()));
            }
            let max_add = max_additional(model, p.max_total, &state, &fixed, &cands);
            tools.gf2.reset(&fixed, &cands);
            let gap = tools.gf2.evaluate(max_add, None).value as i64 - part.sum() as i64;
            let solved = solve(case, TaxUnit { promoted: fixed, cands }, &(fixed | part), caps.unit);
            Ok((gap, solved.steps, solved.completed))
        };
        let (gl, tl, cl) = side(&below, on_side(model, best, h, true))?;
        let (gr, tr, cr) = side(&above, on_side(model, best, h, false))?;
        out.push(Json::object([
            ("h", Json::from(h)),
            ("gl", Json::from(gl)),
            ("gr", Json::from(gr)),
            ("tl", Json::from(tl)),
            ("tr", Json::from(tr)),
            ("completed", Json::from(cl && cr)),
        ]));
    }
    Ok(out)
}

/// The tools a node is measured with.
struct Tools<'a, const W: usize> {
    gf2: Gf2Bound<'a, W>,
    scratch: Scratch<W>,
}

/// The limits of a run.
struct Caps {
    /// The steps of a node's subtree solve (none: to the end).
    unit: Option<u64>,
    /// The steps of the solve that finds a node's best completion (0: it is not looked for).
    best: u64,
    /// The evaluations of the OR part of the AND/OR search (0: it is not run), for the nodes whose subtree is at
    /// least `and_or_min` steps.
    and_or: u64,
    and_or_min: u64,
}

/// The line of one node: see the module comment.
fn measure_unit<const W: usize>(
    case: &Case<'_, W>,
    tools: &mut Tools<'_, W>,
    incumbent: &Incumbent<W>,
    caps: &Caps,
    id: &str,
    source: &str,
    unit: &TaxUnit<W>,
) -> Result<Json, String> {
    let (p, model) = (case.p, &case.p.model);
    let at = |e: &str| format!("n={}: {id}: {e}", case.n());
    let state =
        model.promote_all_using(&mut tools.scratch, &p.root, &unit.promoted).ok_or_else(|| at("cannot be promoted"))?;
    let max_add = max_additional(model, p.max_total, &state, &unit.promoted, &unit.cands);
    tools.gf2.reset(&unit.promoted, &unit.cands);
    let eval = tools.gf2.evaluate(max_add, None);
    let need = need_of(incumbent.sum, &unit.promoted);
    let gap = eval.value as i64 - need as i64;
    let mut fields: Vec<(&str, Json)> = vec![
        ("kind", Json::from("node")),
        ("id", Json::from(id)),
        ("source", Json::from(source)),
        ("depth", Json::from(unit.promoted.len())),
        ("cands", Json::from(unit.cands.len())),
        ("need", Json::from(need)),
        ("today", Json::from(eval.value)),
        ("gap", Json::from(gap)),
    ];
    if gap < 0 {
        fields.push(("cut", Json::from(true)));
        return Ok(Json::object(fields));
    }

    let frames = state.len();
    let (open, _) = by_home(model, frames, &unit.cands);
    let (s_homes, s_weights) = by_home(model, frames, &eval.chosen);
    let leaf = model.conflict_using(&mut tools.scratch, &state, &eval.chosen, true).is_none();
    let cores = core_spread(model, &state, &eval.chosen);
    if cores.crossing() > 0 || cores.span > 1 {
        return Err(at("a core has members in homes that are not adjacent"));
    }
    let solved = solve(case, *unit, &incumbent.set, caps.unit);
    let learned = nogood_spread(model, &p.root, &unit.promoted, &solved.learned);
    if learned.crossing() > 0 || learned.span > 1 {
        return Err(at("a nogood has members in homes that are not adjacent"));
    }
    fields.extend([
        ("open", Json::array(open)),
        ("s", Json::from(eval.chosen.len())),
        ("s_homes", Json::array(s_homes.clone())),
        ("s_weights", Json::array(s_weights.clone())),
        ("leaf", Json::from(leaf)),
        ("cores", cores.json()),
        ("subtree", Json::from(solved.steps)),
        ("completed", Json::from(solved.completed)),
        ("solve_cpu_s", Json::from(solved.cpu_s)),
        ("learned", learned.json()),
    ]);
    if caps.and_or > 0 && solved.steps >= caps.and_or_min {
        if let Some((h, _, _)) = learned.best_split().or_else(|| cores.best_split()) {
            for (name, by_conflict) in [("and_or", false), ("and_or_conflict", true)] {
                let run = and_or(case, tools, incumbent, unit, &state, Separator { h, cap: caps.and_or, by_conflict });
                fields.push((name, run.map_err(|e| at(&e))?));
            }
        }
    }
    let known = known_completion(incumbent, unit);
    let best = match known {
        Some(_) => known,
        None if caps.best > 0 && !leaf => best_completion(case, &mut tools.scratch, &state, unit, caps.best),
        None => None,
    };
    if let Some(completion) = best {
        let (_, best_weights) = by_home(model, frames, &completion);
        let excess: Vec<i64> = s_weights.iter().zip(&best_weights).map(|(&s, &b)| s as i64 - b as i64).collect();
        let ideal = match known {
            Some(_) => (solved.steps, solved.completed),
            None => {
                let ideal = solve(case, *unit, &(unit.promoted | completion), caps.unit);
                (ideal.steps, ideal.completed)
            }
        };
        fields.extend([
            ("best", Json::from(completion.sum())),
            ("excess_homes", Json::array(excess)),
            ("ideal", Json::object([("steps", Json::from(ideal.0)), ("completed", Json::from(ideal.1))])),
            ("sides", Json::array(oracle_sides(case, tools, caps, unit, &completion)?)),
        ]);
    }
    Ok(Json::object(fields))
}

// ---------------------------------------------------------------------------------------------------------
// The nodes

/// The open units of the proof of the game from the optimum on one thread, stopped after every `span` steps `pauses`
/// times: the units of each pause's checkpoint.
fn paused_units<const W: usize>(
    case: &Case<'_, W>,
    incumbent: &Incumbent<W>,
    span: u64,
    pauses: usize,
) -> Result<Vec<Vec<TaxUnit<W>>>, String> {
    let n = case.n();
    let path = std::env::temp_dir().join(format!("taxman-decompose-{n}-{}.ckpt", std::process::id()));
    let cfg = EngineConfig {
        threads: 1,
        checkpoint: Some((path.clone(), Duration::from_secs(1 << 20))),
        node_limit: Some(span),
        progress: None,
        ..EngineConfig::default()
    };
    let root = TaxUnit { promoted: Bits::EMPTY, cands: case.p.candidates };
    let mut start = Start::Fresh { units: vec![root], best: incumbent.sum, payload: payload_of(&incumbent.set) };
    let mut taken = Vec::new();
    for _ in 0..pauses {
        let out = engine::run(&cfg, start, b"wall", |_| TaxSearcher::new(case.space)).map_err(|e| e.to_string())?;
        if out.completed {
            break;
        }
        let mut units = Vec::new();
        engine::scan_checkpoint(&path, |bytes| {
            units.push(TaxUnit::<W>::decode(bytes));
            Ok(())
        })
        .map_err(|e| format!("{}: {e}", path.display()))?;
        taken.push(units);
        start = Start::Resume(path.clone());
    }
    engine::remove_checkpoint(&path).map_err(|e| e.to_string())?;
    Ok(taken)
}

/// Every `k`th of `items`, at most `count` of them.
fn thinned<T: Copy>(items: &[T], count: usize) -> Vec<T> {
    items.iter().step_by(items.len().div_ceil(count.max(1)).max(1)).copied().collect()
}

/// The line of each node of game `n`: see the module comment.
pub fn measure<const W: usize>(case: &Case<'_, W>) -> Result<Played, String> {
    let clock = Clock::start();
    let (measured, cut) = measure_with(case, &mut |line| println!("{line}"))?;
    let mut line = Line::of(case.n());
    line.text("kind", "done").field("measured", measured).field("cut", cut).seconds("cpu_s", clock.elapsed());
    Ok(Played { line, faults: Vec::new() })
}

/// [`measure`], giving each line to `emit` as it is made; the open nodes measured and the nodes cut.
fn measure_with<const W: usize>(case: &Case<'_, W>, emit: &mut dyn FnMut(&Json)) -> Result<(u64, u64), String> {
    let (n, p, params) = (case.n(), case.p, case.params);
    let (set, sum) = known_incumbent(case.board, p, case.optimal)?
        .ok_or_else(|| format!("n={n}: decompose needs the game's known optimum (--known FILE)"))?;
    let incumbent = Incumbent { set, sum };
    let graph = case.space.gf2.as_ref().ok_or_else(|| format!("n={n}: decompose needs the GF(2) bound"))?;
    let mut tools = Tools { gf2: Gf2Bound::new(graph), scratch: Scratch::new() };
    let caps =
        Caps { unit: params.unit_cap, best: params.best_cap, and_or: params.and_or_cap, and_or_min: params.and_or_min };

    let mut sources: Vec<(String, Vec<TaxUnit<W>>)> = Vec::new();
    sources.push(("root".into(), vec![TaxUnit { promoted: Bits::EMPTY, cands: p.candidates }]));
    for (k, units) in paused_units(case, &incumbent, params.span, params.pauses)?.into_iter().enumerate() {
        sources.push((format!("pause{}", k + 1), units));
    }
    let fixture: Vec<TaxUnit<W>> = fixture_nodes::<W>(&params.fixtures, n, sum)?.into_iter().map(|s| s.unit).collect();
    sources.push(("fixture".into(), thinned(&fixture, params.keep)));

    tools.gf2.reset(&Bits::EMPTY, &p.candidates);
    let root_bound = tools.gf2.evaluate(p.max_total, None).value;
    emit(&Json::object([
        ("kind", Json::from("game")),
        ("n", Json::from(n)),
        ("frames", Json::from(p.root.len())),
        ("candidates", Json::from(p.candidates.len())),
        ("open", Json::array(by_home(&p.model, p.root.len(), &p.candidates).0)),
        ("optimum", Json::from(sum)),
        ("root_bound", Json::from(root_bound)),
    ]));

    let (mut seen, mut measured, mut cut) = (HashSet::new(), 0, 0);
    for (source, units) in &sources {
        for (i, unit) in units.iter().enumerate() {
            if !seen.insert((unit.promoted, unit.cands)) {
                continue;
            }
            let line = measure_unit(case, &mut tools, &incumbent, &caps, &format!("{n}-{source}-{i}"), source, unit)?;
            if line.get("cut").is_some() {
                cut += 1;
            } else {
                measured += 1;
            }
            emit(&line);
        }
    }
    Ok((measured, cut))
}

// ---------------------------------------------------------------------------------------------------------
// Tests: `cargo test --features studies` runs them (the example is `test = true` in Cargo.toml)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::case::Params;
    use std::path::Path;
    use taxman::board::Board;
    use taxman::game::Optimal;
    use taxman::matchbound::MatchGraph;
    use taxman::search::{SearchSpace, SearchTactics};

    fn number(line: &Json, name: &str) -> i64 {
        line.get(name).and_then(Json::as_i64).unwrap_or_else(|| panic!("{name} in {line}"))
    }

    /// The root of game 936 and some of the probe's nodes, measured: no conflict crosses a separator (the program
    /// refuses a line if one does), the bound's error by home sums to the root's gap to the optimum, the root's
    /// subtree is its ideal proof, the oracle separators' sides have gaps of the bound over their parts of the
    /// optimum, and the AND/OR search stops at its limit.
    #[test]
    fn the_root_of_936_and_some_probe_nodes_are_measured() {
        let board = Board::<16>::for_n(936);
        let p = board.prepare(SearchTactics::default()).expect("prepare");
        let graph = MatchGraph::new(&p.model, &p.root);
        let space = SearchSpace::new(&p, &graph);
        let params = Params {
            fixtures: Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/perf")),
            keep: 6,
            best_cap: 20_000,
            and_or_cap: 1_500,
            and_or_min: 100,
            ..Params::default()
        };
        let case = Case { board: &board, p: &p, space: &space, params: &params, optimal: Optimal::built_in() };
        let mut lines = Vec::new();
        let (measured, cut) = measure_with(&case, &mut |line| lines.push(line.clone())).expect("measured");
        assert_eq!(lines.len() as u64, 1 + measured + cut);
        let game = &lines[0];
        assert_eq!(game.get("kind").and_then(Json::as_str), Some("game"));
        let (optimum, root_bound) = (number(game, "optimum"), number(game, "root_bound"));
        assert!(root_bound > optimum && measured >= 2, "{game}");

        let root = &lines[1];
        assert_eq!(root.get("id").and_then(Json::as_str), Some("936-root-0"));
        assert_eq!(number(root, "best"), optimum);
        let excess: i64 =
            root.get("excess_homes").and_then(Json::as_arr).unwrap().iter().filter_map(Json::as_i64).sum();
        assert_eq!(excess, root_bound - optimum, "{root}");
        assert_eq!(number(root, "subtree"), number(root.get("ideal").unwrap(), "steps"));
        let sides = root.get("sides").and_then(Json::as_arr).unwrap();
        assert!(!sides.is_empty());
        for side in sides {
            assert!(number(side, "gl") >= 0 && number(side, "gr") >= 0, "{side}");
            assert!(number(side, "tl") >= 1 && number(side, "tr") >= 1, "{side}");
        }
        for name in ["and_or", "and_or_conflict"] {
            let run = root.get(name).unwrap_or_else(|| panic!("{name} in {root}"));
            assert_eq!(number(run, "or_nodes"), 1_500, "{run}");
            assert_eq!(run.get("complete").and_then(Json::as_bool), Some(false));
        }
        for node in lines.iter().filter(|line| line.get("kind").and_then(Json::as_str) == Some("node")) {
            if node.get("cut").is_none() {
                assert!(number(node, "subtree") >= 1 && number(node, "need") >= 1, "{node}");
            }
        }
    }

    #[test]
    fn a_conflict_lies_in_one_place_against_a_separator() {
        assert_eq!(Place::of(&[1, 2], 3), Place::Left);
        assert_eq!(Place::of(&[1, 2], 2), Place::TouchLeft);
        assert_eq!(Place::of(&[2, 3], 2), Place::TouchRight);
        assert_eq!(Place::of(&[2, 2], 2), Place::TouchOnly);
        assert_eq!(Place::of(&[3, 4], 2), Place::Right);
        assert_eq!(Place::of(&[1, 3], 2), Place::Cross);
        assert_eq!(Place::of(&[], 2), Place::Decided);
    }

    #[test]
    fn a_spread_counts_each_conflict_at_every_separator() {
        let mut spread = Spread::new(5);
        spread.add(&[1], &[(10, 1), (20, 2)]);
        spread.add(&[2], &[(21, 2), (30, 3)]);
        spread.add(&[3], &[(31, 3), (40, 4)]);
        spread.add(&[3], &[(32, 3)]);
        assert_eq!((spread.total, spread.span, spread.crossing()), (4, 1, 0));
        assert_eq!(spread.by_frame, vec![0, 1, 1, 2, 0]);
        // Against home 2: the first touches it from the left, the second from the right, the third and fourth lie on
        // the right; against home 3 the first lies on the left.
        assert_eq!(spread.places[2], [0, 2, 1, 1, 0, 0]);
        assert_eq!(spread.places[3], [1, 0, 1, 1, 1, 0]);
        assert_eq!(spread.members[2].len(), 2);
        assert_eq!(spread.members[3].iter().copied().collect::<Vec<_>>(), vec![30, 31, 32]);
        spread.add(&[1], &[(10, 1), (40, 4)]);
        assert_eq!(spread.crossing(), 2);
    }

    #[test]
    fn thinning_keeps_every_kth() {
        assert_eq!(thinned(&[1, 2, 3, 4, 5, 6], 3), vec![1, 3, 5]);
        assert_eq!(thinned(&[1, 2, 3], 10), vec![1, 2, 3]);
        assert_eq!(thinned::<u32>(&[], 3), Vec::<u32>::new());
    }
}
