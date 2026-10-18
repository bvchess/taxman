//! The nogoods that a burst of the finder's work and of the sound search learn, and the Lagrangian bound over them
//! that `choice-prepare` and `choice` fit.  The study of the bound at the root, its data and its conclusion are in
//! docs/experiments/nogoods-from-the-finders-work-2026-10.md.
//!
//! A *nogood* is a set of candidates that cannot all be promoted from the root ([`taxman::model::Nogoods`]); it holds
//! everywhere in the search of its game.  So every completion `F` of the root has `|F ∩ N| ≤ |N| - 1` for each nogood
//! `N`, and for any multipliers `λ_N ≥ 0` the heaviest common independent set of the root's two GF(2) matroids, with
//! each candidate `x` weighing `x - Σ_{N ∋ x} λ_N`, plus `Σ λ_N (|N| - 1)`, is an upper bound on the game's promotion
//! sum (a Lagrangian bound; weak duality; the matchbound/lagrange module has the same argument for the matching).
//! With no multipliers it is the root's GF(2) bound, the baseline.  The inner problem is the crate's own GF(2) bound
//! on `Gf2Graph::reweighted` (the same matroids with the weights above).
//!
//! * The *harvest*: [`finder`] runs the finder (large neighbourhood search, `common/lns.rs`) from a start incumbent for
//!   `--cpu` CPU seconds, and [`control`] the sound search standalone from the same start for the same CPU time, and
//!   each leaves the nogoods its searchers' tables learned (a searcher's table after each unit it ran; the caller
//!   keeps the tables from being forgotten, and a finder's unit search is shorter than the forgetting period anyway).
//!   Every harvested nogood must fail to promote from the root (`promote_all_using`); [`promotable`] counts those that
//!   do not, which is a bug.
//! * The *fit*: [`fit`] fits multipliers by subgradient steps of Polyak's rule towards `SCALE · (incumbent + 1)` (at
//!   the root; `SCALE · need` at a node), in a direction with momentum ([`MOMENTUM`]), damped by a factor that halves
//!   (back at the best multipliers) after [`STALL`] steps without a lower bound.  Any multipliers make a bound, so the
//!   best of the steps is kept; a fit that reaches its target has proven the incumbent optimal (at a node, cut it).

use super::case::{budgeted, Case};
use crate::lns::{run_unit_with, Lns, Ran};
use std::collections::HashSet;
use std::sync::{Mutex, PoisonError};
use std::time::Instant;
use taxman::bits::Bits;
use taxman::engine::{Ctx, Searcher};
use taxman::gf2::{Gf2Bound, Gf2Graph};
use taxman::matchbound::lagrange::SCALE;
use taxman::model::Scratch;
use taxman::search::{SearchSpace, TaxSearcher, TaxUnit};

/// The fit halves its damping after this many steps without a lower bound, and stops when it is below `LEAST_DAMPING`.
/// (Chosen on 3267 with a pool of a few hundred or thousand nogoods: 10 steps stops the fit within about 250 steps,
/// 20 within 700, 40 goes on to 1,500 and ends lower; at 200 steps the three are within 0.03 of the root gap.)
const STALL: usize = 20;
const LEAST_DAMPING: f64 = 1e-9;
/// The direction of a step is the subgradient plus this share of the last direction.  Plain subgradient steps zigzag
/// over a pool of thousands of nogoods: on 3276, 3336 and 3381 with the nogoods of 60 CPU s of the finder (10,000 to
/// 20,000), the bound after 200 steps closed 0.26-0.31 of the root gap, less than the 0.27-0.31 that the few hundred
/// nogoods of the sound search gave, and 0.9 or 0.95 of the last direction closed 0.32-0.41 (0.98 and above is no
/// better; the search's pool, which plain steps fit as far as it goes, is unchanged).
const MOMENTUM: f64 = 0.9;

// ---------------------------------------------------------------------------------------------------------
// Harvesting

/// Where the searchers of a method leave the nogoods they learned.
pub(super) type Sink<const W: usize> = Mutex<HashSet<Bits<W>>>;

/// A searcher that leaves its table in a sink after each unit it runs.
pub(super) struct Harvesting<'a, 's, const W: usize> {
    searcher: TaxSearcher<'a, W>,
    sink: &'s Sink<W>,
}

impl<const W: usize> Searcher<TaxUnit<W>> for Harvesting<'_, '_, W> {
    fn run(&mut self, unit: TaxUnit<W>, ctx: &mut Ctx<'_, TaxUnit<W>>) {
        self.searcher.run(unit, ctx);
        self.sink.lock().unwrap_or_else(PoisonError::into_inner).extend(self.searcher.nogoods().iter().copied());
    }
}

pub(super) fn harvesting<'a, 's, const W: usize>(
    space: &'a SearchSpace<'a, W>,
    sink: &'s Sink<W>,
) -> Harvesting<'a, 's, W> {
    Harvesting { searcher: TaxSearcher::new(space), sink }
}

/// What a method learned, and what it cost.
pub(super) struct Learned<const W: usize> {
    pub(super) cpu_s: f64,
    /// The distinct nogoods its tables learned.
    pub(super) nogoods: Vec<Bits<W>>,
}

/// The nogoods of a sink, in a fixed order.
pub(super) fn sorted<const W: usize>(sink: Sink<W>) -> Vec<Bits<W>> {
    let mut nogoods: Vec<Bits<W>> = sink.into_inner().unwrap_or_else(PoisonError::into_inner).into_iter().collect();
    nogoods.sort_by_cached_key(Bits::to_vec);
    nogoods
}

/// The nogoods of two pools together, each once, in a fixed order.
pub(super) fn union<const W: usize>(a: &[Bits<W>], b: &[Bits<W>]) -> Vec<Bits<W>> {
    let all: HashSet<Bits<W>> = a.iter().chain(b).copied().collect();
    let mut both: Vec<Bits<W>> = all.into_iter().collect();
    both.sort_by_cached_key(Bits::to_vec);
    both
}

/// The finder from `start` for `--cpu` CPU seconds.
pub(super) fn finder<const W: usize>(case: &Case<'_, W>, start: &Bits<W>) -> Learned<W> {
    let sink = Sink::default();
    let search = Lns { p: case.p, space: case.space, options: case.params.lns };
    let mut rng = search.options.rng(case.n());
    let unit_nodes = search.options.unit_nodes;
    let (_, cpu_s) = budgeted(Some(case.params.cpu), |budget| {
        let run = |unit, best: &Bits<W>| -> Ran<W> {
            run_unit_with(&budget.control, unit, best, Some(unit_nodes), |_| harvesting(case.space, &sink))
        };
        search.search_with(start, &mut rng, &|| budget.clock.elapsed(), &|| budget.expired(), run)
    });
    Learned { cpu_s, nogoods: sorted(sink) }
}

/// The sound search from `start`, standalone, for `--cpu` CPU seconds.
pub(super) fn control<const W: usize>(case: &Case<'_, W>, start: &Bits<W>) -> Learned<W> {
    let sink = Sink::default();
    let unit = TaxUnit { promoted: Bits::EMPTY, cands: case.p.candidates };
    let (_, cpu_s) = budgeted(Some(case.params.cpu), |budget| {
        run_unit_with(&budget.control, unit, start, None, |_| harvesting(case.space, &sink))
    });
    Learned { cpu_s, nogoods: sorted(sink) }
}

/// How many of `nogoods` can be promoted from the root after all: none, unless there is a bug.
pub(super) fn promotable<const W: usize>(case: &Case<'_, W>, nogoods: &[Bits<W>]) -> usize {
    let mut scratch = Scratch::new();
    nogoods.iter().filter(|set| case.p.model.promote_all_using(&mut scratch, &case.p.root, set).is_some()).count()
}

// ---------------------------------------------------------------------------------------------------------
// The Lagrangian bound

/// What a fit had found after a number of steps (a mark).
#[derive(Clone, Copy)]
pub(super) struct Mark {
    /// The lowest bound so far, scaled by `SCALE`, and the nogoods with a positive multiplier in its multipliers.
    pub(super) scaled: i64,
    pub(super) active: usize,
}

/// What a fit found.
pub(super) struct Fit {
    /// The steps taken (the fit ends early when the bound proves the incumbent optimal, when the chosen set breaks no
    /// nogood, or when the damping is spent), and the step at which the bound fell below what proves the incumbent
    /// optimal, if it did.
    pub(super) steps: usize,
    pub(super) closed_at: Option<usize>,
    /// What it had found at each mark it was asked for (the final state for a mark it did not reach).
    pub(super) marks: Vec<Mark>,
    /// The multipliers of the lowest bound, one for each nogood.
    pub(super) lambda: Vec<i64>,
    /// The lowest bound (scaled) after each step, and the seconds (wall) since the fit began at the end of each.
    pub(super) trace: Vec<i64>,
    pub(super) at: Vec<f64>,
}

/// What a fit is asked to lower: the heaviest common independent set of at most `cap` members of the matroids
/// `graph`, with each member weighing its number less the multipliers of the `nogoods` that hold it, plus the
/// multipliers times the `rooms` (a root's, or a node's).
pub(super) struct Fitting<'a, const W: usize> {
    pub(super) graph: &'a Gf2Graph,
    pub(super) cap: u32,
    pub(super) nogoods: &'a [Bits<W>],
    /// For each nogood, the most of its members a completion may hold: `|N| - 1` at the root, and at a node with
    /// promotions `P`, for a nogood whose members outside `P` are all candidates, `|N - P| - 1`.
    pub(super) rooms: Vec<i64>,
    /// The multipliers the fit begins from, one for each nogood.
    pub(super) start: Vec<i64>,
    /// The scaled bound the fit is to bring below, which proves what it was aimed at: `SCALE · (incumbent + 1)` at a
    /// root, `SCALE · need` at a node (a bound below it cuts the node).
    pub(super) target: i64,
}

impl<'a, const W: usize> Fitting<'a, W> {
    /// The fit at the root over `nogoods`, from no multipliers, towards proving `incumbent` optimal.
    pub(super) fn at_root(graph: &'a Gf2Graph, cap: u32, nogoods: &'a [Bits<W>], incumbent: u64) -> Self {
        let rooms = root_rooms(nogoods);
        Fitting { graph, cap, nogoods, rooms, start: vec![0; nogoods.len()], target: SCALE * (incumbent as i64 + 1) }
    }
}

/// The rooms of `nogoods` at the root: a completion holds at most `|N| - 1` members of each.
fn root_rooms<const W: usize>(nogoods: &[Bits<W>]) -> Vec<i64> {
    nogoods.iter().map(|set| i64::from(set.len()) - 1).collect()
}

/// The weight each multiplier of `lambda` takes off the members of its nogood of `nogoods`, by number, in `shift`
/// (cleared first; as long as the largest number of the nogoods plus one), and the nogoods' constant,
/// `Σ λ · room`.
fn shifts<const W: usize>(nogoods: &[Bits<W>], rooms: &[i64], lambda: &[i64], shift: &mut [i64]) -> i64 {
    shift.fill(0);
    let mut constant = 0;
    for ((set, &room), &l) in nogoods.iter().zip(rooms).zip(lambda) {
        if l > 0 {
            set.iter().for_each(|x| shift[x as usize] += l);
            constant += l * room;
        }
    }
    constant
}

/// The heaviest common independent set of the matroids `graph` (of at most `cap` members) with each element `x`
/// weighing `SCALE · x - shift[x]` (`shift` by number, 0 past its end; elements of no positive weight left out): its
/// weight (scaled) and the set, as numbers.
fn reweighted_bound<const W: usize>(graph: &Gf2Graph, cap: u32, shift: &[i64]) -> (i64, Bits<W>) {
    let (reweighted, labels) = graph.reweighted(|x| SCALE * i64::from(x) - shift.get(x as usize).copied().unwrap_or(0));
    if labels.is_empty() {
        return (0, Bits::EMPTY);
    }
    let all: Bits<W> = (1..=labels.len() as u32).collect();
    let mut bound = Gf2Bound::<W>::new(&reweighted);
    bound.reset(&Bits::EMPTY, &all);
    let eval = bound.evaluate(cap, None);
    (eval.value as i64, eval.chosen.iter().map(|k| labels[k as usize - 1]).collect())
}

/// The heaviest common independent set of the matroids of `fitting` (of at most `cap` members) with the weights the
/// multipliers `lambda` leave (`SCALE · x` less the multipliers of the nogoods holding `x`), as a scaled bound with the
/// nogoods' constant `Σ λ · room` added, and the set it chooses.  `shift` is scratch, as long as the largest number of
/// the nogoods plus one.
fn dual<const W: usize>(fitting: &Fitting<'_, W>, lambda: &[i64], shift: &mut [i64]) -> (i64, Bits<W>) {
    let constant = shifts(fitting.nogoods, &fitting.rooms, lambda, shift);
    let (value, chosen) = reweighted_bound(fitting.graph, fitting.cap, shift);
    (value + constant, chosen)
}

/// Fits multipliers for the nogoods of `fitting` (sets that cannot be promoted from the root) by at most `steps` steps
/// of the subgradient method: Polyak's rule towards its target, damped by a factor that halves, back at the best
/// multipliers, after [`STALL`] steps without a lower bound.  The subgradient of a nogood is how far the chosen set
/// goes over its room, `|Q ∩ N| - room`, held at zero for a multiplier at zero that it would lower; a step goes in the
/// direction of the subgradient plus [`MOMENTUM`] of the last direction (none after a return to the best multipliers).
/// What it has found after each number of steps in `marks` is in the result (the steps do not depend on the marks).
pub(super) fn fit<const W: usize>(fitting: &Fitting<'_, W>, steps: usize, marks: &[usize]) -> Fit {
    let began = Instant::now();
    let (nogoods, target) = (fitting.nogoods, fitting.target);
    let top = nogoods.iter().filter_map(Bits::max).max().unwrap_or(0) as usize;
    let mut shift = vec![0i64; top + 1];
    let mut lambda = fitting.start.clone();
    let mut direction = vec![0.0f64; nogoods.len()];
    let mut out =
        Fit { steps: 0, closed_at: None, marks: Vec::new(), lambda: Vec::new(), trace: Vec::new(), at: Vec::new() };
    let (mut best, mut best_chosen) = (lambda.clone(), Bits::<W>::EMPTY);
    let mut lowest = i64::MAX;
    let (mut damping, mut stalled) = (1.0f64, 0);
    let mark = |best: &[i64], lowest| Mark { scaled: lowest, active: best.iter().filter(|&&l| l > 0).count() };
    for step in 0..steps {
        let (mut value, mut chosen) = dual(fitting, &lambda, &mut shift);
        out.steps = step + 1;
        if value < lowest {
            (lowest, stalled) = (value, 0);
            best.copy_from_slice(&lambda);
            best_chosen = chosen;
        } else {
            stalled += 1;
            if stalled >= STALL {
                // Back to the best multipliers, to step from them with a smaller step.
                (damping, stalled) = (damping / 2.0, 0);
                lambda.copy_from_slice(&best);
                (value, chosen) = (lowest, best_chosen);
                direction.fill(0.0);
            }
        }
        out.trace.push(lowest);
        out.at.push(began.elapsed().as_secs_f64());
        if marks.contains(&out.steps) {
            out.marks.push(mark(&best, lowest));
        }
        if lowest < target {
            out.closed_at = Some(out.steps);
            break;
        }
        let (mut slack, mut norm) = (false, 0.0);
        for (((set, &room), &l), d) in nogoods.iter().zip(&fitting.rooms).zip(&lambda).zip(&mut direction) {
            let over = i64::from((chosen & *set).len()) - room;
            let slope = if l == 0 && over < 0 { 0 } else { over };
            slack |= slope != 0;
            *d = slope as f64 + MOMENTUM * *d;
            norm += *d * *d;
        }
        if !slack || norm == 0.0 || damping < LEAST_DAMPING {
            break;
        }
        let size = damping * (value - target + SCALE) as f64 / norm;
        for (l, &d) in lambda.iter_mut().zip(&direction) {
            *l = (*l + (size * d).round() as i64).max(0);
        }
    }
    // A mark the fit did not reach has the state it ended in.
    let ended = mark(&best, lowest);
    out.marks.resize(marks.len(), ended);
    out.lambda = best;
    out
}
