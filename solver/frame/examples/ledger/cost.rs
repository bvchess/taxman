//! What the rest of a board's proof will cost, forecast from the floor steps it has had, and what to do with the board
//! next.
//!
//! A search under a floor F (`search::search_above`) that completes having found nothing proves that no promotion set
//! weighs more than F (THEORY.md, Lemma F).  Its cost grows about tenfold for every 147 points the floor is lowered
//! below the root's bound: on 14 games of 2265-3259, log10 of the nodes is linear in that distance (r² 0.996 a game), a
//! median 0.68 decades per 100 points (0.53 to 0.89 a game;
//! solver/notebook/search/proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10/README.md).  Each board
//! has its own line, `log10(nodes) = intercept + slope · distance`, fitted from the steps it has had; a board with
//! fewer than two steps of 100 nodes or more has the median slope (the shortest trees have a floor of their own, at a
//! couple of steps) and, with none, the median intercept; the fit is pulled to the median slope as a pair of points 141
//! apart on the median line would pull it, so that two steps close together do not make a slope.  The cost of the
//! board's proof, a search under a floor at its lower bound, is the line at the distance from the root's bound down to
//! the lower bound.
//!
//! The steps of a board go down by a cost, not a distance: a step costs the same factor more than the one before, the
//! factor of `floor_step` points at the median slope (a tenfold at the default 150), so that a board with a steeper
//! line takes fewer points.  The steps before the proof cost a share of it that falls geometrically (a tenth at best,
//! a half if the last step lands close to the proof), so the board goes straight to the proof when what remains of its
//! gap is within [`PROOF_STEPS`] steps; on the 14 games' measured costs, laddered that way from the root's bound, the
//! steps and the proof cost a median 1.05 times the proof alone (1.49 times when the proof comes only within one step).

use crate::book::Entry;

/// The median line of the 14 games measured under floors: `intercept` is log10 of the nodes at the root's bound and
/// `slope` the decades a point adds.  With the slope, 1,000 nodes prove down 370 points, 10,000 514, 100,000 661.
pub const PRIOR: Line = Line { intercept: 0.5, slope: 0.0068 };

/// A step of fewer nodes than this is not a point of the line.
pub const MIN_NODES: u64 = 100;

/// The slopes a fit may have, in decades a point.
const SLOPES: (f64, f64) = (0.003, 0.012);

/// The weight of the median slope in a fit, in squared points: the sum of squares about their mean of a pair of points
/// 141 apart.
const PULL: f64 = 100.0 * 100.0;

/// The steps a board's gap must exceed for a floor step to be taken, else the board goes to its proof.
pub const PROOF_STEPS: u64 = 2;

/// The steps a thread makes in a second where nothing is known, so that a limit is generous.
pub const DEFAULT_RATE: f64 = 1000.0;

/// log10 of the steps of a search under a floor, against how many points below the root's bound the floor is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub intercept: f64,
    pub slope: f64,
}

impl Line {
    pub fn log_nodes(self, distance: f64) -> f64 {
        self.intercept + self.slope * distance
    }

    /// The line through `points` (distance, log10 of nodes), its slope pulled to the median slope: none has the median
    /// line, one has the median slope through it.
    pub fn fit(points: &[(f64, f64)]) -> Line {
        if points.is_empty() {
            return PRIOR;
        }
        let n = points.len() as f64;
        let (mx, my) = (points.iter().map(|p| p.0).sum::<f64>() / n, points.iter().map(|p| p.1).sum::<f64>() / n);
        let sxx: f64 = points.iter().map(|p| (p.0 - mx).powi(2)).sum();
        let sxy: f64 = points.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum();
        let slope = ((sxy + PULL * PRIOR.slope) / (sxx + PULL)).clamp(SLOPES.0, SLOPES.1);
        Line { intercept: my - slope * mx, slope }
    }
}

/// The root's bound of `e`, as a score: its own, else its upper bound, which is never above it.
fn root(e: &Entry) -> Option<u64> {
    e.root.or(e.upper.map(|u| u.value))
}

/// How many points below the root's bound the floor `floor` (a score) is.
fn distance(e: &Entry, floor: u64) -> f64 {
    root(e).map_or(0, |root| root.saturating_sub(floor)) as f64
}

/// The points of the board's line: its completed steps of `MIN_NODES` nodes or more, as (distance, log10 of nodes).
fn points(e: &Entry) -> Vec<(f64, f64)> {
    let steps = e.steps.iter().filter(|s| s.nodes >= MIN_NODES);
    steps.map(|s| (distance(e, s.floor), (s.nodes as f64).log10())).collect()
}

/// The board's line, from its points.  A stalled step is a lower bound of what the floor it works under costs: the line
/// is lifted to pass no lower than it.
pub fn line(e: &Entry) -> Line {
    let mut line = Line::fit(&points(e));
    if let Some(stalled) = e.stalled.filter(|s| s.nodes >= MIN_NODES) {
        let cost = (stalled.nodes as f64).log10();
        line.intercept += (cost - line.log_nodes(distance(e, stalled.floor))).max(0.0);
    }
    line
}

/// The points a step goes down: `floor_step` at the median slope, fewer on a steeper line.
pub fn step_points(line: Line, floor_step: u64) -> u64 {
    ((floor_step as f64 * PRIOR.slope / line.slope).round() as u64).max(1)
}

/// The steps and the seconds of the floor steps of the board, the stalled one included.
pub fn spent(e: &Entry) -> (u64, f64) {
    let steps = e.steps.iter().map(|s| (s.nodes, s.seconds)).chain(e.stalled.map(|s| (s.nodes, s.seconds)));
    steps.fold((0, 0.0), |(nodes, seconds), (more, longer)| (nodes + more, seconds + longer))
}

/// Whether the board has yet to give its line two points: it has a step to take (not its proof), none stalled, and
/// fewer than two completed steps of `MIN_NODES` nodes or more.  Such steps are cheap (the first four cost under a
/// second at n = 4500, where a step of 150 points is a few tens of nodes), and a board with no line is forecast by
/// the median line, which is far off at some n (at 4500 the first points have a slope of 0.0035, not 0.0068, and the
/// median line forecasts a proof 5 decades too dear); so the scheduler gives every board its two points before it
/// deepens any, and no board looks dearer than it is because it has not been tried.
pub fn exploring(e: &Entry, floor_step: u64) -> bool {
    floor_step > 0
        && e.stalled.is_none()
        && points(e).len() < 2
        && matches!(plan(e, floor_step, 0.0), Plan::Step { .. })
}

/// The steps a thread of the board makes in a second: its own steps' if they took a second or more, else `pooled`
/// (the run's), else [`DEFAULT_RATE`].
pub fn rate(e: &Entry, pooled: f64) -> f64 {
    let (nodes, seconds) = spent(e);
    if seconds >= 1.0 && nodes > 0 {
        nodes as f64 / seconds
    } else if pooled > 0.0 {
        pooled
    } else {
        DEFAULT_RATE
    }
}

/// What the rest of a board's proof is forecast to cost: the nodes and the seconds of one thread, as logarithms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Forecast {
    pub log_nodes: f64,
    pub log_seconds: f64,
}

impl Forecast {
    pub fn seconds(self) -> f64 {
        10f64.powf(self.log_seconds)
    }
}

/// The forecast of the proof of `e`: its line at the distance from the root's bound down to its lower bound, but not
/// less than twice the steps that its ordinary searches and climbs have spent without finishing (the tree is more than
/// they explored, and the doubling limits make the next stop twice as far).  So a board whose proof keeps being
/// stopped is forecast dearer each time, as the order of the searches without floor steps charged it by `level`.  None
/// if it has no lower bound.
pub fn forecast(e: &Entry, pooled: f64) -> Option<Forecast> {
    let lower = e.lower.as_ref()?.value;
    let spent = (2.0 * e.nodes as f64).max(1.0).log10();
    let log_nodes = line(e).log_nodes(distance(e, lower)).max(spent);
    Some(Forecast { log_nodes, log_seconds: log_nodes - rate(e, pooled).log10() })
}

/// What to do next with an open board.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Plan {
    /// The proof: the ordinary search, seeded with the best game and capped by the upper bound.
    Proof,
    /// A step under the floor `score`: `resume` it from its checkpoint (it was stalled, `tries` times); its forecast is
    /// `seconds` of one thread.
    Step { score: u64, resume: bool, seconds: f64, tries: u32 },
}

/// The plan for `e` with floor steps of `floor_step` points (0: none, the board goes to its proof).  A stalled step is
/// resumed while its floor is between the bounds; else the next step is the floor one step below the upper bound, if
/// the gap is more than [`PROOF_STEPS`] steps.
pub fn plan(e: &Entry, floor_step: u64, pooled: f64) -> Plan {
    let (Some(lower), Some(upper)) = (&e.lower, e.upper) else { return Plan::Proof };
    if floor_step == 0 {
        return Plan::Proof;
    }
    let line = line(e);
    let seconds = |floor: u64| 10f64.powf(line.log_nodes(distance(e, floor))) / rate(e, pooled);
    if let Some(stalled) = e.stalled.filter(|s| lower.value < s.floor && s.floor < upper.value) {
        return Plan::Step {
            score: stalled.floor,
            resume: true,
            seconds: seconds(stalled.floor),
            tries: stalled.tries,
        };
    }
    let step = step_points(line, floor_step);
    if upper.value - lower.value <= PROOF_STEPS * step {
        return Plan::Proof;
    }
    let score = upper.value - step;
    Plan::Step { score, resume: false, seconds: seconds(score), tries: 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::{Lower, Rule, Stalled, Step, Upper};

    /// A board whose root bound is `root`, with the bounds `lower` and `upper` and the given steps (floor, nodes,
    /// seconds).
    fn board(root: u64, lower: u64, upper: u64, steps: &[(u64, u64, f64)]) -> Entry {
        Entry {
            n: 5000,
            root: Some(root),
            lower: Some(Lower { value: lower, moves: Vec::new(), promotions: None, rule: Rule::Search, from: None }),
            upper: Some(Upper { value: upper, rule: Rule::Root, from: None }),
            steps: steps.iter().map(|&(floor, nodes, seconds)| Step { floor, nodes, seconds }).collect(),
            ..Entry::default()
        }
    }

    /// The steps a line makes: the floors at the distances, the nodes the line gives, a thousand a second.
    fn made(root: u64, line: Line, distances: &[u64]) -> Vec<(u64, u64, f64)> {
        distances
            .iter()
            .map(|&d| {
                let nodes = 10f64.powf(line.log_nodes(d as f64));
                (root - d, nodes as u64, nodes / 1000.0)
            })
            .collect()
    }

    #[test]
    fn a_fit_recovers_the_line_of_its_points_and_is_pulled_to_the_median_slope_by_few() {
        let truth = Line { intercept: -0.3, slope: 0.0085 };
        let points: Vec<(f64, f64)> = [200.0, 400.0, 600.0, 800.0].iter().map(|&d| (d, truth.log_nodes(d))).collect();
        let fit = Line::fit(&points);
        assert!(
            (fit.slope - truth.slope).abs() < 0.0003 && (fit.log_nodes(900.0) - truth.log_nodes(900.0)).abs() < 0.1
        );
        assert_eq!(Line::fit(&[]), PRIOR);
        let one = Line::fit(&[(500.0, 4.0)]);
        assert_eq!(one.slope, PRIOR.slope);
        assert!((one.log_nodes(500.0) - 4.0).abs() < 1e-9);
        // Two points a few apart do not make a slope: the median one stays.
        let close = Line::fit(&[(500.0, 4.0), (505.0, 4.5)]);
        assert!((close.slope - PRIOR.slope).abs() < 0.0005, "{close:?}");
        // The slope stays in its range.
        let steep = Line::fit(&[(100.0, 1.0), (1000.0, 30.0)]);
        assert_eq!(steep.slope, SLOPES.1);
    }

    #[test]
    fn a_boards_line_and_forecast_come_from_its_steps_of_enough_nodes() {
        let own = Line { intercept: 0.2, slope: 0.0075 };
        let mut e = board(1_000_000, 999_000, 1_000_000, &made(1_000_000, own, &[150, 300, 450, 600]));
        // The proof is a floor at the lower bound: 1,000 points down.
        let fore = forecast(&e, 0.0).unwrap();
        assert!((fore.log_nodes - own.log_nodes(1000.0)).abs() < 0.15, "{fore:?} against {}", own.log_nodes(1000.0));
        // Steps of fewer than 100 nodes are not points: a board with only those has the median line.
        e.steps.retain(|s| s.nodes < MIN_NODES);
        assert!(!e.steps.is_empty());
        assert_eq!(line(&e), PRIOR);
        // The median line forecasts 0.5 + 0.0068 * 1000 decades.
        assert!((forecast(&e, 0.0).unwrap().log_nodes - 7.3).abs() < 1e-9);
        // A stalled step lifts the line to pass no lower than it.
        let before = forecast(&e, 0.0).unwrap().log_nodes;
        e.stalled = Some(Stalled { floor: 999_500, nodes: 10u64.pow(9), seconds: 1e6, tries: 3 });
        assert!(forecast(&e, 0.0).unwrap().log_nodes > before + 1.0);
        assert!((line(&e).log_nodes(500.0) - 9.0).abs() < 1e-9);
        // Ordinary searches that spent more than the line forecasts raise the forecast to twice what they spent.
        e.stalled = None;
        assert!((forecast(&e, 0.0).unwrap().log_nodes - 7.3).abs() < 1e-9);
        e.nodes = 10u64.pow(8);
        assert!((forecast(&e, 0.0).unwrap().log_nodes - (2e8f64).log10()).abs() < 1e-9);
        e.nodes = 10u64.pow(6);
        assert!((forecast(&e, 0.0).unwrap().log_nodes - 7.3).abs() < 1e-9, "less than the line gives: the line stays");
        // No lower bound, no forecast.
        e.lower = None;
        assert!(forecast(&e, 0.0).is_none());
    }

    #[test]
    fn the_seconds_come_from_the_boards_rate_else_the_runs_else_a_default() {
        let slow = board(1_000_000, 990_000, 1_000_000, &[(999_000, 5000, 10.0), (998_000, 20_000, 40.0)]);
        assert_eq!(rate(&slow, 7777.0), 500.0, "its own steps took more than a second");
        let quick = board(1_000_000, 990_000, 1_000_000, &[(999_000, 5000, 0.01)]);
        assert_eq!(rate(&quick, 7777.0), 7777.0, "too little time to measure: the run's");
        assert_eq!(rate(&quick, 0.0), DEFAULT_RATE);
        let (slow, quick) = (forecast(&slow, 7777.0).unwrap(), forecast(&quick, 7777.0).unwrap());
        assert!((slow.log_seconds - (slow.log_nodes - 500f64.log10())).abs() < 1e-9);
        assert!((quick.log_seconds - (quick.log_nodes - 7777f64.log10())).abs() < 1e-9);
    }

    #[test]
    fn a_step_goes_down_by_the_same_factor_of_cost_and_the_proof_comes_within_two_steps() {
        assert_eq!(step_points(PRIOR, 150), 150);
        // A steeper line takes fewer points for the same factor, a flatter one more.
        assert_eq!(step_points(Line { slope: 0.0136, ..PRIOR }, 150), 75);
        assert_eq!(step_points(Line { slope: 0.0034, ..PRIOR }, 150), 300);
        // A board with no steps and a gap of 1,000: the step is the floor 150 below its upper bound, not resumed.
        let fresh = board(1_000_000, 999_000, 1_000_000, &[]);
        match plan(&fresh, 150, 0.0) {
            Plan::Step { score, resume, tries, seconds } => {
                assert_eq!((score, resume, tries), (999_850, false, 0));
                // The median line at 150 points down: 0.5 + 1.02 decades, a thousand nodes a second by default.
                assert!((seconds - 10f64.powf(1.52) / DEFAULT_RATE).abs() < 1e-9, "{seconds}");
            }
            plan => panic!("{plan:?}"),
        }
        // The next step is below the upper bound, not the root's bound: U fell to 999,600.
        let lowered = board(1_000_000, 999_000, 999_600, &[]);
        assert!(matches!(plan(&lowered, 150, 0.0), Plan::Step { score: 999_450, .. }));
        // The proof when the gap is within two steps, a step when it is a point more.
        assert_eq!(plan(&board(1_000_000, 999_700, 1_000_000, &[]), 150, 0.0), Plan::Proof);
        assert!(matches!(plan(&board(1_000_000, 999_699, 1_000_000, &[]), 150, 0.0), Plan::Step { .. }));
        // Floor steps off: the proof, whatever the gap.
        assert_eq!(plan(&fresh, 0, 0.0), Plan::Proof);
        // No bounds: nothing to plan.
        assert_eq!(plan(&Entry::default(), 150, 0.0), Plan::Proof);
    }

    #[test]
    fn a_board_explores_until_its_line_has_two_points() {
        let mut e = board(1_000_000, 999_000, 1_000_000, &[]);
        assert!(exploring(&e, 150));
        assert!(!exploring(&e, 0), "no floor steps, nothing to explore");
        // Steps of fewer than 100 nodes are no points; one point is not a line.
        e.steps =
            vec![Step { floor: 999_850, nodes: 33, seconds: 0.01 }, Step { floor: 999_700, nodes: 99, seconds: 0.01 }];
        assert!(exploring(&e, 150));
        e.steps.push(Step { floor: 999_500, nodes: 400, seconds: 0.1 });
        assert!(exploring(&e, 150));
        e.steps.push(Step { floor: 999_400, nodes: 1500, seconds: 0.4 });
        assert!(!exploring(&e, 150), "two points");
        // A board with a stalled step has a lifted line already; one whose gap is within two steps goes to its proof.
        e.steps.truncate(3);
        e.stalled = Some(Stalled { floor: 999_300, nodes: 10_000, seconds: 3.0, tries: 1 });
        assert!(!exploring(&e, 150));
        let near = board(1_000_000, 999_800, 1_000_000, &[]);
        assert!(!exploring(&near, 150) && plan(&near, 150, 0.0) == Plan::Proof);
    }

    #[test]
    fn a_stalled_step_is_resumed_under_its_floor_while_the_floor_is_between_the_bounds() {
        let mut e = board(1_000_000, 999_000, 1_000_000, &[]);
        e.stalled = Some(Stalled { floor: 999_700, nodes: 50_000, seconds: 20.0, tries: 2 });
        assert!(matches!(plan(&e, 150, 0.0), Plan::Step { score: 999_700, resume: true, tries: 2, .. }));
        // The upper bound fell to the floor or below it, or the lower bound rose to it: the step is moot.
        e.upper.as_mut().unwrap().value = 999_700;
        assert!(matches!(plan(&e, 150, 0.0), Plan::Step { resume: false, .. }));
        e.upper.as_mut().unwrap().value = 1_000_000;
        e.lower.as_mut().unwrap().value = 999_700;
        assert_eq!(plan(&e, 150, 0.0), Plan::Proof);
    }

    /// The forecast orders boards by their lines: of two boards with the same bounds, the one whose steps cost less is
    /// forecast cheaper, and a board with a flatter or lower line than the median is cheaper than one with none.
    #[test]
    fn the_forecast_orders_boards_by_their_lines() {
        let root = 2_000_000;
        let (cheap, dear) = (Line { intercept: -1.0, slope: 0.006 }, Line { intercept: 0.5, slope: 0.0085 });
        let a = board(root, root - 1000, root, &made(root, cheap, &[500, 650, 800]));
        let b = board(root, root - 1000, root, &made(root, dear, &[500, 650, 800]));
        let none = board(root, root - 1000, root, &[]);
        let log = |e: &Entry| forecast(e, 0.0).unwrap().log_nodes;
        assert!(log(&a) < log(&none) && log(&none) < log(&b), "{} {} {}", log(&a), log(&none), log(&b));
        // The same lines, a board whose lower bound is further down has the dearer proof.
        let deeper = board(root, root - 1200, root, &made(root, cheap, &[500, 650, 800]));
        assert!(log(&deeper) > log(&a));
    }
}
