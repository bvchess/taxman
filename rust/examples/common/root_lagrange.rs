//! The Lagrangian bound of a root over nogoods generated as it is fitted, for the examples that measure roots
//! (`examples/slope.rs`, `examples/matroid.rs`: `#[path = "common/root_lagrange.rs"] mod root_lagrange;`).  A
//! search node can be measured the same way as the root of its own `MatchGraph`.

use std::collections::HashSet;
use taxman::bits::Bits;
use taxman::matchbound::lagrange::{fit, LagrangeBound, Relaxation, SCALE};
use taxman::matchbound::MatchGraph;
use taxman::model::{FrameGraph, FrameSet, Scratch};
use taxman::search::lagrange::FIT_STEPS;

/// [`root_lagrange`]'s damping halves after this many steps without a lower bound.
pub const STALL: usize = 80;
/// At each of [`root_lagrange`]'s steps, the conflicts looked for in the weighted matching's candidates `Q`: the
/// conflict of `Q`, then of `Q` without its largest member, and so on.
pub const CONFLICTS_PER_STEP: usize = 12;

/// What [`root_lagrange`] found.
pub struct RootLagrange<const W: usize> {
    /// The lowest bound found, scaled by `SCALE`.
    pub scaled: i64,
    /// The bound that `matchbound::lagrange::fit` (the search's refit: `FIT_STEPS` steps of Polyak's rule, from
    /// zero multipliers) finds with the same nogoods, scaled.
    pub refit: i64,
    /// The nogoods, and the multipliers of the lowest bound (one each).
    pub nogoods: Vec<Bits<W>>,
    pub lambda: Vec<i64>,
    pub steps: usize,
    /// The step at which the bound reached the optimum (fell below the target), if it did.
    pub closed_at: Option<usize>,
}

/// The Lagrangian bound of the root over the nogoods `nogoods` (conflicts of the root) and more generated as it
/// is fitted: at most `steps` steps of the subgradient method towards `SCALE · need` by Polyak's rule, damped by a
/// factor that halves (back at the best multipliers) after [`STALL`] steps without a lower bound, each step adding
/// the conflicts of the weighted matching's candidates ([`CONFLICTS_PER_STEP`]) that are new to the nogoods, with
/// multiplier 0.  Undamped (as `matchbound::lagrange::fit` steps), Polyak's rule towards a target hundreds below the
/// bound overshoots at every step: with one conflict a step, 1100's bound never went below the matching bound in
/// 2,000 steps.
#[allow(clippy::too_many_arguments)]
pub fn root_lagrange<const W: usize>(
    graph: &MatchGraph,
    model: &FrameGraph,
    scratch: &mut Scratch<W>,
    root: &FrameSet<W>,
    cands: &Bits<W>,
    mut nogoods: Vec<Bits<W>>,
    need: u64,
    steps: usize,
) -> RootLagrange<W> {
    let target = SCALE * need as i64;
    let mut seen = HashSet::new();
    nogoods.retain(|k| seen.insert(*k));
    let mut lambda = vec![0i64; nogoods.len()];
    let mut solver = LagrangeBound::new(graph);
    let mut out =
        RootLagrange { scaled: i64::MAX, refit: 0, nogoods: Vec::new(), lambda: Vec::new(), steps: 0, closed_at: None };
    let (mut best, mut damping, mut stalled) = (lambda.clone(), 1.0f64, 0);
    for step in 0..steps {
        out.steps = step + 1;
        solver.set_relaxation(Relaxation::new(graph, &nogoods, &lambda));
        let value = solver.value(&Bits::EMPTY, cands).expect("the root's moves can be matched");
        if value < out.scaled {
            (out.scaled, stalled) = (value, 0);
            best.clone_from(&lambda);
        } else {
            stalled += 1;
            if stalled == STALL {
                (damping, stalled) = (damping / 2.0, 0);
                if damping < 1e-3 {
                    break;
                }
                // Back to the best multipliers; the nogoods added since start at 0.
                lambda.iter_mut().zip(best.iter().chain(std::iter::repeat(&0))).for_each(|(l, b)| *l = *b);
                continue;
            }
        }
        if out.scaled < target {
            out.closed_at = Some(step + 1);
            break;
        }
        let q = solver.chosen();
        let mut rest = q;
        for _ in 0..CONFLICTS_PER_STEP {
            let Some(k) = model.conflict_using(scratch, root, &rest, true) else { break };
            if seen.insert(k) {
                nogoods.push(k);
                lambda.push(0);
            }
            rest.remove(k.max().expect("a conflict has members"));
        }
        // At the root every nogood is live: its subgradient is |Q ∩ N| − (|N| − 1).
        let g: Vec<i64> = nogoods.iter().map(|k| i64::from((q & *k).len()) - (i64::from(k.len()) - 1)).collect();
        let norm: i64 = g.iter().map(|x| x * x).sum();
        if norm == 0 {
            break;
        }
        let size = damping * (value - target + SCALE) as f64 / norm as f64;
        for (l, g) in lambda.iter_mut().zip(&g) {
            *l = (*l + (size * *g as f64).round() as i64).max(0);
        }
    }
    let mut zero = vec![0; nogoods.len()];
    out.refit = fit(graph, &Bits::EMPTY, cands, &nogoods, &mut zero, need, FIT_STEPS).expect("matchable");
    best.resize(nogoods.len(), 0);
    (out.nogoods, out.lambda) = (nogoods, best);
    out
}
