//! `wall nogoods`: can a burst of the finder's work produce nogoods that lower a game's upper bound?  The study, its
//! data and its conclusion are in docs/experiments/nogoods-from-the-finders-work-2026-10.md.
//!
//! A *nogood* is a set of candidates that cannot all be promoted from the root ([`taxman::model::Nogoods`]); it holds
//! everywhere in the search of its game.  So every completion `F` of the root has `|F ∩ N| ≤ |N| - 1` for each nogood
//! `N`, and for any multipliers `λ_N ≥ 0` the heaviest common independent set of the root's two GF(2) matroids, with
//! each candidate `x` weighing `x - Σ_{N ∋ x} λ_N`, plus `Σ λ_N (|N| - 1)`, is an upper bound on the game's promotion
//! sum (a Lagrangian bound; weak duality; the matchbound/lagrange module has the same argument for the matching).
//! With no multipliers it is the root's GF(2) bound, the baseline.  The inner problem is the crate's own GF(2) bound
//! on `Gf2Graph::reweighted` (the same matroids with the weights above).
//!
//! For each game the command:
//!
//! 1. makes the standalone start incumbent (as `wall finders` does);
//! 2. runs the finder (large neighbourhood search, `common/lns.rs`) for `--cpu` CPU seconds and harvests every nogood
//!    its unit searches learn (a searcher's table after each unit it ran; the tables are never forgotten, and a unit's
//!    search is shorter than the forgetting period anyway);
//! 3. as the control, runs the sound search standalone from the same start for the same CPU time, harvesting its
//!    nogoods the same way (its tables are not forgotten either, which is not what a real search does: it forgets
//!    every 50,000 steps);
//! 4. checks every harvested nogood: it must fail to promote from the root (`promote_all_using`), and any that does not
//!    is a bug, reported in `promotable` and as an error after the line;
//! 5. computes the root's GF(2) bound (the baseline) and a Lagrangian bound over each pool of nogoods (the finder's,
//!    the search's, both) with the multipliers fitted by `--steps` subgradient steps (default 200) of Polyak's rule
//!    towards `SCALE · (incumbent + 1)`, the incumbent being the best game of the start, the finder and the control,
//!    in a direction with momentum ([`MOMENTUM`]), damped by a factor that halves (back at the best multipliers) after
//!    [`STALL`] steps without a lower bound.  Any
//!    multipliers make a bound, so the best of the steps is reported; a fit that reaches the incumbent has proven it
//!    optimal (`_closed_at`).  The fit runs on to `--long` steps (default 2,000, at least `--steps`; the steps do not
//!    depend on either), to show how far the bound goes beyond the `--steps` that are reported as the answer.
//!
//! The line has the beginning `wall finders` gives (the board, `upper`, the root's GF(2) bound as a score, `opt`, the
//! known optimum with `--known`, `start`, `meets`), then `incumbent` (score) and `base_s` (the root bound's seconds);
//! for the `finder`, then for the `search` (the control): its score, `_cpu_s`, `_nodes` (the engine's steps), `_learned`
//! (nogoods learned, a repeat counting again), `_pool` (distinct ones), and for the finder `_units` (the units run),
//! for the search `_completed`; `both_pool`; `promotable` (nogoods that promote: none, or a bug); then, for each pool
//! `finder`, `search` and `both`, under its name: `_steps` (steps the fit took) and `_closed_at`; and for the fit's
//! first `--steps` steps (`_bound`) and for all of them (`_long`), the Lagrangian bound as a score and, under its name
//! with `_active` (nogoods with a positive multiplier in the best multipliers), `_best_step`, `_s` (seconds) and, with
//! a known optimum, `_share` appended: the share of the root gap it closes, `(upper - bound) / (upper - opt)`.  A bound
//! below the known optimum or the incumbent is a bug, and an error after the line.

use super::finders::{budgeted, Case, Played};
use super::root_bound;
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
type Sink<const W: usize> = Mutex<HashSet<Bits<W>>>;

/// A searcher that leaves its table in a sink after each unit it runs.
struct Harvesting<'a, 's, const W: usize> {
    searcher: TaxSearcher<'a, W>,
    sink: &'s Sink<W>,
}

impl<const W: usize> Searcher<TaxUnit<W>> for Harvesting<'_, '_, W> {
    fn run(&mut self, unit: TaxUnit<W>, ctx: &mut Ctx<'_, TaxUnit<W>>) {
        self.searcher.run(unit, ctx);
        self.sink.lock().unwrap_or_else(PoisonError::into_inner).extend(self.searcher.nogoods().iter().copied());
    }
}

fn harvesting<'a, 's, const W: usize>(space: &'a SearchSpace<'a, W>, sink: &'s Sink<W>) -> Harvesting<'a, 's, W> {
    Harvesting { searcher: TaxSearcher::new(space), sink }
}

/// What a method found and learned.
pub(super) struct Learned<const W: usize> {
    /// Its best game.
    best: Bits<W>,
    pub(super) cpu_s: f64,
    /// The engine's steps.
    nodes: u64,
    /// The units searched (the finder's), and whether the search ran to its end (the control's).
    units: u64,
    completed: bool,
    /// The nogoods its tables learned, a repeat counted again (the space's counter), and the distinct ones.
    learned: u64,
    pub(super) nogoods: Vec<Bits<W>>,
}

/// The nogoods of a sink, in a fixed order.
fn sorted<const W: usize>(sink: Sink<W>) -> Vec<Bits<W>> {
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

/// The nogoods the tables of the space's searchers have learned so far.
fn learned_so_far<const W: usize>(space: &SearchSpace<'_, W>) -> u64 {
    space.nogood_totals.lock().unwrap_or_else(PoisonError::into_inner).learned
}

/// The finder from `start` for `--cpu` CPU seconds.
pub(super) fn finder<const W: usize>(case: &Case<'_, W>, start: &Bits<W>) -> Learned<W> {
    let (sink, before) = (Sink::default(), learned_so_far(case.space));
    let search = Lns { p: case.p, space: case.space, options: case.params.lns };
    let mut rng = search.options.rng(case.n());
    let unit_nodes = search.options.unit_nodes;
    let (out, cpu_s) = budgeted(Some(case.params.cpu), |budget| {
        let run = |unit, best: &Bits<W>| -> Ran<W> {
            run_unit_with(&budget.control, unit, best, Some(unit_nodes), |_| harvesting(case.space, &sink))
        };
        search.search_with(start, &mut rng, &|| budget.clock.elapsed(), &|| budget.expired(), run)
    });
    Learned {
        best: out.set,
        cpu_s,
        nodes: out.nodes,
        units: out.units,
        completed: false,
        learned: learned_so_far(case.space) - before,
        nogoods: sorted(sink),
    }
}

/// The sound search from `start`, standalone, for `--cpu` CPU seconds.
pub(super) fn control<const W: usize>(case: &Case<'_, W>, start: &Bits<W>) -> Learned<W> {
    let (sink, before) = (Sink::default(), learned_so_far(case.space));
    let unit = TaxUnit { promoted: Bits::EMPTY, cands: case.p.candidates };
    let (ran, cpu_s) = budgeted(Some(case.params.cpu), |budget| {
        run_unit_with(&budget.control, unit, start, None, |_| harvesting(case.space, &sink))
    });
    Learned {
        best: ran.set,
        cpu_s,
        nodes: ran.nodes,
        units: 1,
        completed: ran.completed,
        learned: learned_so_far(case.space) - before,
        nogoods: sorted(sink),
    }
}

/// How many of `nogoods` can be promoted from the root after all: none, unless there is a bug.
fn promotable<const W: usize>(case: &Case<'_, W>, nogoods: &[Bits<W>]) -> usize {
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
    /// The step the lowest bound came at.
    best_step: usize,
    /// The seconds the steps took.
    seconds: f64,
}

/// What a fit found.
pub(super) struct Fit {
    /// The bound of the first step, with no multipliers (the root's GF(2) bound, scaled).
    first: i64,
    /// The steps taken (the fit ends early when the bound proves the incumbent optimal, when the chosen set breaks no
    /// nogood, or when the damping is spent), and the step at which the bound fell below what proves the incumbent
    /// optimal, if it did.
    pub(super) steps: usize,
    closed_at: Option<usize>,
    /// What it had found at each mark it was asked for (the final state for a mark it did not reach).
    pub(super) marks: Vec<Mark>,
    /// The multipliers of the lowest bound, one for each nogood.
    pub(super) lambda: Vec<i64>,
}

/// The weight each multiplier of `lambda` takes off the members of its nogood of `nogoods`, by number, in `shift`
/// (cleared first; as long as the largest number of the nogoods plus one), and the nogoods' constant at the root,
/// `Σ λ (|N| - 1)`.
pub(super) fn shifts<const W: usize>(nogoods: &[Bits<W>], lambda: &[i64], shift: &mut [i64]) -> i64 {
    shift.fill(0);
    let mut constant = 0;
    for (set, &l) in nogoods.iter().zip(lambda).filter(|&(_, &l)| l > 0) {
        set.iter().for_each(|x| shift[x as usize] += l);
        constant += l * (i64::from(set.len()) - 1);
    }
    constant
}

/// The heaviest common independent set of the matroids `graph` (of at most `cap` members) with each element `x`
/// weighing `SCALE · x - shift[x]` (`shift` by number, 0 past its end; elements of no positive weight left out): its
/// weight (scaled) and the set, as numbers.
pub(super) fn reweighted_bound<const W: usize>(graph: &Gf2Graph, cap: u32, shift: &[i64]) -> (i64, Bits<W>) {
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

/// The heaviest common independent set of the root's matroids (of at most `cap` members) with the weights the
/// multipliers `lambda` leave (`SCALE · x` less the multipliers of the nogoods holding `x`), as a scaled bound with the
/// nogoods' constant `Σ λ (|N| - 1)` added, and the set it chooses.  `shift` is scratch, as long as the largest number
/// of the nogoods plus one.
fn dual<const W: usize>(
    graph: &Gf2Graph,
    cap: u32,
    nogoods: &[Bits<W>],
    lambda: &[i64],
    shift: &mut [i64],
) -> (i64, Bits<W>) {
    let constant = shifts(nogoods, lambda, shift);
    let (value, chosen) = reweighted_bound(graph, cap, shift);
    (value + constant, chosen)
}

/// Fits multipliers for `nogoods` (sets that cannot be promoted from the root) by at most `steps` steps of the
/// subgradient method: Polyak's rule towards `SCALE · (incumbent + 1)`, damped by a factor that halves, back at the
/// best multipliers, after [`STALL`] steps without a lower bound.  The subgradient of a nogood is how far the chosen
/// set goes over its room, `|Q ∩ N| - (|N| - 1)`, held at zero for a multiplier at zero that it would lower; a step
/// goes in the direction of the subgradient plus [`MOMENTUM`] of the last direction (none after a return to the best
/// multipliers).  What it has found after each number of steps in `marks` is in the result (the steps do not depend
/// on the marks).
pub(super) fn fit<const W: usize>(
    graph: &Gf2Graph,
    cap: u32,
    nogoods: &[Bits<W>],
    steps: usize,
    marks: &[usize],
    incumbent: u64,
) -> Fit {
    let began = Instant::now();
    let target = SCALE * (incumbent as i64 + 1);
    let top = nogoods.iter().filter_map(Bits::max).max().unwrap_or(0) as usize;
    let mut shift = vec![0i64; top + 1];
    let mut lambda = vec![0i64; nogoods.len()];
    let mut direction = vec![0.0f64; nogoods.len()];
    let mut out = Fit { first: 0, steps: 0, closed_at: None, marks: Vec::new(), lambda: Vec::new() };
    let (mut best, mut best_chosen) = (lambda.clone(), Bits::<W>::EMPTY);
    let (mut lowest, mut best_step) = (i64::MAX, 0);
    let (mut damping, mut stalled) = (1.0f64, 0);
    let mark = |best: &[i64], lowest, best_step| Mark {
        scaled: lowest,
        active: best.iter().filter(|&&l| l > 0).count(),
        best_step,
        seconds: began.elapsed().as_secs_f64(),
    };
    for step in 0..steps {
        let (mut value, mut chosen) = dual(graph, cap, nogoods, &lambda, &mut shift);
        out.steps = step + 1;
        if step == 0 {
            out.first = value;
        }
        if value < lowest {
            (lowest, best_step, stalled) = (value, step, 0);
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
        if marks.contains(&out.steps) {
            out.marks.push(mark(&best, lowest, best_step));
        }
        if lowest < target {
            out.closed_at = Some(out.steps);
            break;
        }
        let (mut slack, mut norm) = (false, 0.0);
        for ((set, &l), d) in nogoods.iter().zip(&lambda).zip(&mut direction) {
            let over = i64::from((chosen & *set).len()) - (i64::from(set.len()) - 1);
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
    let ended = mark(&best, lowest, best_step);
    out.marks.resize(marks.len(), ended);
    out.lambda = best;
    out
}

// ---------------------------------------------------------------------------------------------------------
// The command

/// The line of game `n` (see the module comment).
pub fn of<const W: usize>(case: &Case<'_, W>) -> Played {
    let (start, start_cpu_s) = case.start();
    let (mut line, meets) = case.line(&start, start_cpu_s);
    let mut faults = Vec::new();
    if meets {
        return Played { line, faults };
    }
    let n = case.n();
    let graph = case.space.gf2.as_ref().expect("the GF(2) bound is a default tactic");
    let (eval, _, base_time) = root_bound(case.p, graph);
    let base = eval.value;
    let (found, searched) = (finder(case, &start), control(case, &start));
    let incumbent = [start, found.best, searched.best].into_iter().max_by_key(Bits::sum).expect("three games");
    faults.extend(case.referee(&incumbent).err().map(|e| format!("n={n}: the best game: {e}")));
    let opt = case.optimal.score(n).map(|score| score - case.board.in_the_bag);
    let both = union(&found.nogoods, &searched.nogoods);
    let wrong = promotable(case, &both);
    if wrong > 0 {
        faults.push(format!("n={n}: {wrong} harvested nogoods can be promoted from the root"));
    }
    line.field("incumbent", case.score(&incumbent)).seconds("base_s", base_time.as_secs_f64());
    line.field("finder", case.score(&found.best))
        .seconds("finder_cpu_s", found.cpu_s)
        .field("finder_nodes", found.nodes)
        .field("finder_units", found.units)
        .field("finder_learned", found.learned)
        .field("finder_pool", found.nogoods.len());
    line.field("search", case.score(&searched.best))
        .seconds("search_cpu_s", searched.cpu_s)
        .field("search_nodes", searched.nodes)
        .field("search_completed", searched.completed)
        .field("search_learned", searched.learned)
        .field("search_pool", searched.nogoods.len());
    line.field("both_pool", both.len()).field("promotable", wrong);
    let (steps, long) = (case.params.steps, case.params.steps.max(case.params.long));
    for (name, pool) in [("finder", &found.nogoods), ("search", &searched.nogoods), ("both", &both)] {
        let fitted = fit(graph, case.p.max_total, pool, long, &[steps, long], incumbent.sum());
        if fitted.first != SCALE * base as i64 {
            faults
                .push(format!("n={n}: {name}: the first step, {}, is not the root bound {base} scaled", fitted.first));
        }
        line.field(&format!("{name}_steps"), fitted.steps).maybe(&format!("{name}_closed_at"), fitted.closed_at);
        // The bound after `--steps` steps (`_bound`) and after `--long` (`_long`), as scores, with the nogoods that
        // have a positive multiplier in it, the step it came at, the seconds, and the share of the root gap it closes.
        for (suffix, mark) in [("bound", fitted.marks[0]), ("long", fitted.marks[1])] {
            let bound = mark.scaled.div_euclid(SCALE) as u64;
            line.field(&format!("{name}_{suffix}"), case.board.in_the_bag + bound)
                .field(&format!("{name}_{suffix}_active"), mark.active)
                .field(&format!("{name}_{suffix}_best_step"), mark.best_step)
                .seconds(&format!("{name}_{suffix}_s"), mark.seconds)
                .maybe(
                    &format!("{name}_{suffix}_share"),
                    opt.filter(|&opt| base > opt)
                        .map(|opt| format!("{:.4}", (base as f64 - bound as f64) / (base - opt) as f64)),
                );
            if bound < incumbent.sum() || opt.is_some_and(|opt| bound < opt) {
                faults.push(format!(
                    "n={n}: {name}: the {suffix} bound {bound} is below the incumbent {} or the known optimum {opt:?}",
                    incumbent.sum()
                ));
            }
        }
    }
    Played { line, faults }
}
