//! The unsound mode (`taxman::solver::unsound`): its lines replay to their scores and never beat the built-in table,
//! are the same on one thread and on four, run the rounds of a lower level first, resume, and end their rounds at the
//! table's score with `--stop-at-known`; and the wall between its lines and proven ones holds both ways.  The options
//! that shape the rounds (`fix`, `tilt`, `back`, `widen`) change no line unless they are given, and each of them, and
//! the last three together, keep all of the above.
//!
//! A level of effort budgets 2^(E−1) × `round_work` work of rounds.  The tests lower `round_work` from the 65,536 of a
//! round of full neighborhoods to the work of a round or two of the small games they play, so that a level is a few
//! rounds and a test stays inside its CPU limit.

mod common;

use common::TempDir;
use std::fs;
use std::path::Path;
use std::slice;
use taxman::game::{replay, Optimal};
use taxman::json::Json;
use taxman::results::{verify_files, ResultsFile};
use taxman::search::finder::{NEIGHBORHOODS, NEIGHBORHOOD_WORK};
use taxman::solver::unsound::{self, Line, Origin, Plan, Previous, Raised, Source};
use taxman::solver::{solve_range, Method, SolveOptions};

/// The work that effort 1 budgets in these tests: a little more than the work of a round of the small games (their
/// steps are about 80 a round, and the neighborhoods' work is 64 × 32 = 2,048), so that effort E runs a few rounds more
/// than effort E − 1.
const WORK: u64 = 2_200;

/// The plan of a run at `effort`, with the default seed and no measuring option.
fn plan(effort: u32) -> Plan {
    Plan { effort, round_work: WORK, ..Plan::default() }
}

/// The options of a run on `threads` threads, with or without the tactic `seed_incumbent`.  Without it, a game's start
/// is made without the moves for n − 1, and the start of each of the games [`FIRST`] and [`SECOND`] falls short.
fn options(threads: usize, seed_incumbent: bool) -> SolveOptions {
    let mut opts = SolveOptions { threads, ..SolveOptions::default() };
    opts.tactics.seed_incumbent = seed_incumbent;
    opts
}

/// Games whose rounds raise the score of a start made without the moves for n − 1: 375 in its rounds 2 and 3, 376 in
/// its rounds 1 and 2, each to the table's score.
const FIRST: (u32, u32) = (375, 376);

/// Games 384 to 387, with no seed: three searches, whose first rounds raise their starts (384 and 387 to the table's
/// score), and 386, a repeat of 385.
const SECOND: (u32, u32) = (384, 387);

/// The lines of the games `range` played with `plan` and `opts`, appended to the file `out` if there is one; each line
/// replays to its score, which is no more than the table's.
fn lines((lo, hi): (u32, u32), opts: &SolveOptions, plan: &Plan, out: Option<&Path>) -> Vec<Line> {
    let optimal = Optimal::built_in();
    let mut lines = Vec::new();
    unsound::solve_range(lo, hi, opts, plan, out, |line| lines.push(line.clone())).expect("a run of the unsound mode");
    for line in &lines {
        let n = line.n;
        assert_eq!(replay(n, &line.moves), Ok(line.score), "n={n}: the moves do not replay to the score");
        assert!(line.score <= optimal.score(n).unwrap(), "n={n}: {} beats the table", line.score);
        let sum: u64 = line.promotions.iter().map(|&p| u64::from(p)).sum();
        assert!(line.in_the_bag.is_none_or(|bag| bag + sum == line.score), "n={n}");
    }
    lines
}

/// `line` without what varies from run to run: its times and its threads.
fn timeless(line: &Line) -> Line {
    let improvements = line.improvements.iter().map(|raised| Raised { seconds: 0.0, ..*raised }).collect();
    Line { seconds: 0.0, start_seconds: 0.0, threads: 0, improvements, ..line.clone() }
}

/// The lines without what varies from run to run.
fn timeless_all(lines: &[Line]) -> Vec<Line> {
    lines.iter().map(timeless).collect()
}

/// The rounds that raised a line's score, and the scores they raised it to.
fn raised(line: &Line) -> Vec<(u64, u64)> {
    line.improvements.iter().map(|raised| (raised.round, raised.score)).collect()
}

/// What each improvement of a line recorded, but its time: the round, the trajectory that made it, its stage, the steps
/// and the work of the rounds so far, and the score.
fn raised_by(line: &Line) -> Vec<(u64, usize, usize, u64, u64, u64)> {
    let each = |r: &Raised| (r.round, r.trajectory, r.stage, r.steps, r.work, r.score);
    line.improvements.iter().map(each).collect()
}

#[test]
fn lines_replay_to_their_scores_and_never_beat_the_table() {
    let lines = lines((1, 160), &options(2, true), &plan(0), None);
    assert_eq!(lines.len(), 160);
    for line in &lines {
        let n = line.n;
        let json = line.to_json();
        assert_eq!(json.get("unsound"), Some(&Json::Bool(true)), "n={n}");
        assert!(json.get("completed").is_none() && json.get("proof").is_none(), "n={n}: it claims nothing");
        assert_eq!(json.get("budget"), Some(&Json::from(0u64)), "n={n}: effort 0 budgets no steps");
        if n > 1 {
            let prev = line.prev.as_ref().expect("the moves for n - 1");
            assert_eq!((prev.n, prev.source, prev.short_by), (n - 1, Source::Run, 0), "n={n}");
        }
    }
    for method in [Method::Trivial, Method::Prime, Method::Repeat, Method::Search] {
        assert!(lines.iter().any(|line| line.method == method), "no {} game", method.as_str());
    }
    let searched = lines.iter().filter(|line| line.method == Method::Search);
    assert!(searched.clone().all(|line| line.start.is_some() && line.rounds == 0), "effort 0 is the start alone");
}

#[test]
fn the_lines_are_the_same_on_one_thread_and_on_four() {
    let one = lines(SECOND, &options(1, false), &plan(1), None);
    let four = lines(SECOND, &options(4, false), &plan(1), None);
    assert_eq!(timeless_all(&one), timeless_all(&four));
    let raised = one.iter().filter(|line| !line.improvements.is_empty()).count();
    assert_eq!(raised, 3, "the first round raises the start of each search");
    assert!(one.iter().all(|line| line.threads == 1) && four.iter().all(|line| line.threads == 4));
}

#[test]
fn a_higher_level_of_effort_does_the_rounds_of_a_lower_one_first() {
    let (less, more) =
        (lines(FIRST, &options(4, false), &plan(1), None), lines(FIRST, &options(4, false), &plan(2), None));
    for (less, more) in less.iter().zip(&more) {
        assert!(raised(more).starts_with(&raised(less)), "n={}: {:?}, then {:?}", less.n, raised(less), raised(more));
        assert!(more.rounds > less.rounds && more.work > less.work, "n={}", less.n);
    }
    // 375 is raised in its second round only at effort 1, and again in its third at effort 2, which 376 does not.
    assert_eq!((raised(&less[0]).len(), raised(&more[0]).len()), (1, 2));
    assert_eq!((raised(&less[1]).len(), raised(&more[1]).len()), (2, 2));
}

#[test]
fn the_rounds_of_a_level_run_until_their_work_reaches_its_budget_and_the_round_that_reaches_it_is_the_last() {
    let opts = options(4, false);
    let rounds = |plan: &Plan| lines(FIRST, &opts, plan, None);
    for effort in 1..=3 {
        let plan = plan(effort);
        assert_eq!(plan.budget(), WORK << (effort - 1));
        for line in rounds(&plan) {
            let n = line.n;
            assert!(line.work >= plan.budget(), "n={n}: {} work, a budget of {}", line.work, plan.budget());
            assert_eq!(line.to_json().get("budget"), Some(&Json::from(plan.budget())), "n={n}");
            // The work is the steps and the work of each neighborhood, and each improvement says how much of both.
            let overhead = NEIGHBORHOODS * NEIGHBORHOOD_WORK;
            assert_eq!(line.work, line.steps + line.rounds * overhead, "n={n}");
            assert_eq!(line.to_json().get("work"), Some(&Json::from(line.work)), "n={n}");
            for raised in &line.improvements {
                assert_eq!(raised.work, raised.steps + raised.round * overhead, "n={n}: {raised:?}");
                assert!(raised.work <= line.work, "n={n}");
            }
        }
    }
    // A budget of exactly the work that the rounds did is reached by the same rounds, the last of which reaches it;
    // one more is not, and takes another round.
    let n = FIRST.0;
    let line = &lines((n, n), &opts, &plan(3), None)[0];
    let again = |round_work| lines((n, n), &opts, &Plan { round_work, ..plan(1) }, None).remove(0);
    let (exact, more) = (again(line.work), again(line.work + 1));
    assert_eq!((exact.rounds, exact.work), (line.rounds, line.work), "n={n}");
    assert!(more.rounds > line.rounds && more.work > line.work, "n={n}: {} rounds", more.rounds);
}

/// Games whose rounds, from a start made with the table's moves for n − 1, find different sets under the options that
/// shape the rounds, and each starts from the table's moves, so it is the same game whatever the lines before it.
const SHAPED: [u32; 4] = [375, 539, 544, 680];

/// The plans of the options at `effort`, for the games [`SHAPED`] from the table's moves: a share fixed other than the
/// default's, the tilted share, trajectories from the 8 games before, the widening after a round without growth, and
/// the last three together.
fn shaped(effort: u32) -> [(&'static str, Plan); 5] {
    let known = Plan { from_known: true, ..plan(effort) };
    [
        ("fix", Plan { fix: (0.4, 0.6), ..known }),
        ("tilt", Plan { tilt: true, ..known }),
        ("back", Plan { back: 8, ..known }),
        ("widen", Plan { widen: Some(1), ..known }),
        ("tilt, back and widen", Plan { tilt: true, back: 8, widen: Some(1), ..known }),
    ]
}

/// The lines of the games [`SHAPED`] played with `plan`, which starts them from the table's moves.
fn shaped_lines(opts: &SolveOptions, plan: &Plan) -> Vec<Line> {
    assert!(plan.from_known);
    SHAPED.iter().flat_map(|&n| lines((n, n), opts, plan, None)).collect()
}

#[test]
fn without_the_options_the_rounds_are_these() {
    // The lines of this form of the mode at effort 3, to see that no change to a neighborhood's draws or to the budget
    // goes unseen: the neighborhoods of the default are pinned beside, in `search::finder`.
    let plan = Plan { from_known: true, ..plan(3) };
    let lines = shaped_lines(&options(4, true), &plan);
    let now: Vec<_> = lines.iter().map(|line| (line.n, line.score, line.rounds, line.steps, raised(line))).collect();
    let was = [
        (375, 44328, 5, 394, vec![(2, 44327), (3, 44328)]),
        (539, 91703, 5, 394, vec![(2, 91703)]),
        (544, 93256, 5, 429, vec![(1, 93256)]),
        (680, 146798, 5, 326, vec![(1, 146773), (4, 146784), (5, 146798)]),
    ];
    assert_eq!(now, was);
    for line in &lines {
        assert_eq!((line.plan.fix, line.plan.tilt, line.plan.back, line.plan.widen), ((0.7, 0.9), false, 0, None));
        assert_eq!(line.trajectories, Some(vec![Origin::Start]), "n={}: the start's trajectory alone", line.n);
        assert_eq!(line.stage_rounds, Some(vec![[line.rounds, 0, 0]]), "n={}: every round at stage 0", line.n);
        assert!(line.improvements.iter().all(|raised| (raised.trajectory, raised.stage) == (0, 0)), "n={}", line.n);
    }
}

/// The module `$module` of five tests, `fix`, `tilt`, `back`, `widen` and `all_three`, each calling `$check` with the
/// index of that option in [`shaped`]: one test of all five would use more CPU than a test may.
macro_rules! for_each_option {
    ($module:ident, $check:ident) => {
        mod $module {
            #[test]
            fn fix() {
                super::$check(0);
            }
            #[test]
            fn tilt() {
                super::$check(1);
            }
            #[test]
            fn back() {
                super::$check(2);
            }
            #[test]
            fn widen() {
                super::$check(3);
            }
            #[test]
            fn all_three() {
                super::$check(4);
            }
        }
    };
}

/// The lines of the games [`SHAPED`] under option `shape` of [`shaped`] are the same on one thread and on four.
fn check_same_on_one_thread_and_on_four(shape: usize) {
    let shape = shaped(3)[shape].1;
    let one = shaped_lines(&options(1, true), &shape);
    let four = shaped_lines(&options(4, true), &shape);
    assert_eq!(timeless_all(&one), timeless_all(&four));
    assert!(one.iter().all(|line| line.threads == 1) && four.iter().all(|line| line.threads == 4));
}
for_each_option!(same_on_one_thread_and_on_four, check_same_on_one_thread_and_on_four);

/// Under option `shape` of [`shaped`], the improvements of the games [`SHAPED`] at effort 1 are a prefix of those at
/// effort 2, and those at effort 2 of those at effort 3: the rounds, the trajectories, the stages, the steps and the
/// scores.
fn check_lower_level_first(shape: usize) {
    let by_level: Vec<Vec<Line>> =
        (1..=3).map(|effort| shaped_lines(&options(4, true), &shaped(effort)[shape].1)).collect();
    for (less, more) in by_level.iter().zip(&by_level[1..]) {
        for (less, more) in less.iter().zip(more) {
            let (n, raised) = (less.n, (raised_by(less), raised_by(more)));
            assert!(raised.1.starts_with(&raised.0), "n={n}: {:?}, then {:?}", raised.0, raised.1);
            assert!(more.rounds >= less.rounds && more.work >= less.work, "n={n}");
        }
    }
    // The longer run does more, so the comparison shows something.
    let rounds: Vec<u64> = by_level.iter().map(|lines| lines.iter().map(|line| line.rounds).sum()).collect();
    assert!(rounds[0] < rounds[2], "rounds at efforts 1 to 3: {rounds:?}");
}
for_each_option!(lower_level_first, check_lower_level_first);

/// Option `shape` of [`shaped`] is in the line: the plan, the trajectories the game followed, the rounds of each at
/// each stage, and what each improvement says of both.
fn check_what_the_line_records(shape: usize) {
    let (what, shape) = shaped(3)[shape];
    let lines = shaped_lines(&options(4, true), &shape);
    for line in &lines {
        let (n, json) = (line.n, line.to_json());
        assert_eq!(line.plan, shape, "{what}");
        assert_eq!(json.get("fix"), Some(&Json::array([shape.fix.0, shape.fix.1])), "n={n}");
        assert_eq!(json.get("tilt"), Some(&Json::Bool(shape.tilt)), "n={n}");
        assert_eq!(json.get("back"), Some(&Json::from(shape.back)), "n={n}");
        assert_eq!(json.get("widen"), Some(&shape.widen.map_or(Json::Null, Json::from)), "n={n}");
        assert_eq!(json.get("budget"), Some(&Json::from(shape.budget())), "n={n}");
        let (origins, stages) = (line.trajectories.as_ref().unwrap(), line.stage_rounds.as_ref().unwrap());
        assert_eq!(origins[0], Origin::Start, "n={n}");
        assert_eq!(stages.len(), origins.len(), "n={n}: the rounds of each trajectory");
        assert!(stages.iter().all(|rounds| rounds.iter().sum::<u64>() == line.rounds), "n={n}: {stages:?}");
        assert!(shape.widen.is_some() || stages.iter().all(|rounds| rounds[1..] == [0, 0]), "n={n}: {stages:?}");
        assert!(origins.len() <= 1 + shape.back as usize, "n={n}: {origins:?}");
        let steps: Vec<u64> = line.improvements.iter().map(|raised| raised.steps).collect();
        assert!(steps.windows(2).all(|pair| pair[0] < pair[1]) && steps.iter().all(|&s| s <= line.steps), "n={n}");
        for raised in &line.improvements {
            assert!(raised.trajectory < origins.len() && raised.stage < 3, "n={n}: {raised:?}");
            assert!(shape.widen.is_some() || raised.stage == 0, "n={n}: {raised:?}");
        }
    }
    let trajectories: Vec<usize> = lines.iter().map(|line| line.trajectories.as_ref().unwrap().len()).collect();
    assert_eq!(trajectories, if shape.back > 0 { vec![2, 3, 4, 4] } else { vec![1; 4] }, "{what}");
    if shape.widen.is_some() {
        // A trajectory that stalls widens its neighborhoods: some game spends rounds at the second stage.
        let widened: u64 = lines.iter().flat_map(|line| line.stage_rounds.as_ref().unwrap()).map(|r| r[1] + r[2]).sum();
        assert!(widened > 0, "{what}: no round at a stage below the first");
    }
}
for_each_option!(what_the_line_records, check_what_the_line_records);

#[test]
fn back_adds_a_trajectory_for_each_distinct_set_that_survives_of_the_games_before() {
    let known = Plan { from_known: true, ..plan(1) };
    let origins = |back| -> Vec<Vec<Origin>> {
        let shape = Plan { back, ..known };
        shaped_lines(&options(4, true), &shape).into_iter().map(|line| line.trajectories.unwrap()).collect()
    };
    let (none, one, eight) = (origins(0), origins(1), origins(8));
    assert!(none.iter().all(|origins| origins == &[Origin::Start]));
    let game = Origin::Game;
    // What survives of the moves for 374 is the start of 375, so 375 begins no trajectory from it, and the moves for
    // 367 are the first to give another; the other games begin one from the moves for the game before.
    assert_eq!(
        one,
        [
            vec![Origin::Start],
            vec![Origin::Start, game(538)],
            vec![Origin::Start, game(543)],
            vec![Origin::Start, game(679)]
        ]
    );
    assert_eq!(eight[0], [Origin::Start, game(367)]);
    assert_eq!(eight[1], [Origin::Start, game(538), game(531)]);
    assert_eq!(eight[2], [Origin::Start, game(543), game(539), game(538)]);
    assert_eq!(eight[3], [Origin::Start, game(679), game(675), game(674)]);
    // The games are in the order n − 1, n − 2, ..., none twice, and a smaller J keeps the first trajectories.
    for (shaped, (one, eight)) in SHAPED.iter().zip(one.iter().zip(&eight)) {
        let games: Vec<u32> =
            eight[1..].iter().map(|origin| if let Origin::Game(m) = origin { *m } else { 0 }).collect();
        assert!(
            games.windows(2).all(|pair| pair[0] > pair[1]) && games.iter().all(|m| (shaped - 8..*shaped).contains(m))
        );
        assert!(eight.starts_with(one), "n={shaped}");
    }
}

/// Games whose start, from the table's moves for n − 1, is lighter than what survives of the moves for an earlier game:
/// their start, and the score of the heaviest start.
const SURVIVORS_AHEAD: [(u32, u64, u64); 2] = [(715, 161442, 161449), (805, 204901, 204916)];

#[test]
fn a_trajectory_that_begins_heavier_than_the_start_raises_the_score_before_any_round() {
    let back = Plan { back: 8, from_known: true, stop_at_known: true, ..plan(0) };
    let opts = options(4, true);
    for (n, start, score) in SURVIVORS_AHEAD {
        let line = &lines((n, n), &opts, &back, None)[0];
        assert_eq!(
            (line.start, line.score, line.rounds, line.steps, line.work),
            (Some(start), score, 0, 0, 0),
            "n={n}"
        );
        // The game's score is that of the trajectory, from the first: an improvement of round 0.
        let [raised] = &line.improvements[..] else { panic!("n={n}: {:?}", line.improvements) };
        let origins = line.trajectories.as_ref().unwrap();
        assert!((1..origins.len()).contains(&raised.trajectory), "n={n}: {:?} of {origins:?}", raised.trajectory);
        assert_eq!((raised.round, raised.stage, raised.steps, raised.work, raised.score), (0, 0, 0, 0, score), "n={n}");
        // What --stop-at-known recorded is that round, if the score is the table's.
        let known = line.known.unwrap();
        assert_eq!(known.round, (score >= known.score).then_some(0), "n={n}");
        // A level above does the same first, and its rounds go on from there.
        let more = &lines((n, n), &opts, &Plan { effort: 1, stop_at_known: false, ..back }, None)[0];
        assert_eq!(raised_by(more).first(), Some(&(0, raised.trajectory, 0, 0, 0, score)), "n={n}");
        assert!(more.rounds >= 1 && more.score >= score, "n={n}");
    }
}

#[test]
fn a_game_has_no_trajectory_from_the_moves_nothing_has() {
    // Game n − j below 1 has no moves, and a game that takes no seed begins no trajectory from any game.
    let back = Plan { back: 8, ..plan(1) };
    let seeded = lines((1, 40), &options(2, true), &back, None);
    let searched: Vec<&Line> = seeded.iter().filter(|line| line.trajectories.is_some()).collect();
    assert!(!searched.is_empty());
    for line in searched {
        let origins = line.trajectories.as_ref().unwrap();
        let games: Vec<u32> =
            origins.iter().filter_map(|o| if let Origin::Game(m) = o { Some(*m) } else { None }).collect();
        assert!(games.iter().all(|&m| m >= 1 && m < line.n && line.n - m <= 8), "n={}: {games:?}", line.n);
    }
    let unseeded = lines(SECOND, &options(4, false), &back, None);
    let plain = lines(SECOND, &options(4, false), &plan(1), None);
    let unbacked = |line: &Line| Line { plan: Plan { back: 0, ..line.plan }, ..timeless(line) };
    assert_eq!(unseeded.iter().map(unbacked).collect::<Vec<_>>(), plain.iter().map(unbacked).collect::<Vec<_>>());
    assert!(unseeded
        .iter()
        .filter(|line| line.start.is_some())
        .all(|line| line.trajectories == Some(vec![Origin::Start])));
}

#[test]
fn a_plan_with_an_option_out_of_range_is_refused() {
    let refused = |plan: Plan| unsound::solve_range(12, 12, &options(1, true), &plan, None, |_| ()).unwrap_err();
    for fix in [(0.9, 0.7), (-0.1, 0.5), (0.5, 1.1), (f64::NAN, 0.5), (0.5, f64::INFINITY)] {
        let err = refused(Plan { fix, ..plan(1) });
        assert!(err.starts_with("--fix takes LO,HI") && err.contains("0 <= LO <= HI <= 1"), "{fix:?}: {err}");
    }
    for back in [9, 100] {
        assert!(refused(Plan { back, ..plan(1) }).starts_with("--back takes J, a whole number from 0 to 8"), "{back}");
    }
    assert!(refused(Plan { widen: Some(0), ..plan(1) }).starts_with("--widen takes S"));
    // The ladder of the widening replaces the share fixed.
    for fix in [(0.5, 0.7), (0.4, 0.6)] {
        assert_eq!(refused(Plan { widen: Some(2), fix, ..plan(1) }), unsound::WIDEN_WITH_FIX, "{fix:?}");
    }
    assert!(refused(Plan { effort: unsound::MAX_EFFORT + 1, ..plan(1) }).contains("more than the most"));
    // The most effort budgets 2^47 work, which no run reaches and no count overflows: the plan is accepted.
    let most = Plan { effort: unsound::MAX_EFFORT, round_work: unsound::ROUND_WORK, ..plan(1) };
    assert_eq!((unsound::check_plan(&most), most.budget()), (Ok(()), 1 << 47));
}

#[test]
fn a_run_resumes_from_its_file() {
    let dir = TempDir::new("unsound-resume");
    // The games after the break begin trajectories from the moves for the 3 games before: this run's, the file's, the
    // table's.
    let (opts, plan, (lo, hi)) = (options(4, true), Plan { back: 3, ..plan(1) }, SECOND);
    let whole = lines(SECOND, &opts, &plan, Some(&dir.join("whole.jsonl")));
    let file = dir.join("parts.jsonl");
    let first = lines((lo, lo + 1), &opts, &plan, Some(&file));
    let rest = lines((lo, hi), &opts, &plan, Some(&file));
    assert_eq!((first.len(), rest.len()), (2, whole.len() - 2));
    // The game after the break starts from the file's line of the game before it: the run's line in one go.
    assert_eq!(rest[0].prev.as_ref().map(|prev| prev.source), Some(Source::Results));
    let from_run = |line: &Line| {
        let source = |source| if source == Source::Results { Source::Run } else { source };
        let prev = line.prev.clone().map(|prev| Previous { source: source(prev.source), ..prev });
        Line { prev, ..timeless(line) }
    };
    let parts: Vec<Line> = first.iter().chain(&rest).map(from_run).collect();
    assert_eq!(parts, whole.iter().map(timeless).collect::<Vec<_>>());
    assert_eq!(fs::read_to_string(&file).unwrap().lines().count(), whole.len());
}

#[test]
fn stop_at_known_ends_the_rounds_at_the_tables_score() {
    let optimal = Optimal::built_in();
    let measured = Plan { stop_at_known: true, ..plan(3) };
    let lines = lines(FIRST, &options(4, false), &measured, None);
    let reached: Vec<Option<u64>> = lines.iter().map(|line| line.known.and_then(|known| known.round)).collect();
    assert_eq!(reached, [Some(3), Some(2)], "the rounds in which 375 and 376 reach the table's score");
    for line in &lines {
        let known = line.known.expect("what --stop-at-known recorded");
        assert_eq!((known.score, line.score), (optimal.score(line.n).unwrap(), known.score), "n={}", line.n);
        assert_eq!(Some(line.rounds), known.round, "n={}: the rounds end with the one that reached it", line.n);
        assert!(line.work < measured.budget(), "n={}: the rounds ended before their budget", line.n);
    }
    // With the seed, 376 starts from the moves for 375, at the table's score, and runs no round.
    let seeded = self::lines(FIRST, &options(4, true), &measured, None);
    assert_eq!(
        (seeded[1].start, seeded[1].rounds, seeded[1].known.unwrap().round),
        (Some(seeded[1].score), 0, Some(0))
    );
}

#[test]
fn the_measuring_options_take_the_tables_games_only() {
    let past = Optimal::built_in().max_n() + 1;
    for measured in [Plan { stop_at_known: true, ..plan(0) }, Plan { from_known: true, ..plan(0) }] {
        let err = unsound::solve_range(past - 1, past, &options(1, true), &measured, None, |_| ()).unwrap_err();
        assert!(err.contains(&format!("lacks game {past}")), "{err}");
    }
    // With --from-known every game starts from the table's moves for n − 1.
    let known = lines(FIRST, &options(1, true), &Plan { from_known: true, ..plan(0) }, None);
    assert!(known.iter().all(|line| line.prev.as_ref().unwrap().source == Source::Table && line.plan.from_known));
}

/// A file of unsound lines is refused by every reader of proven lines, and a file of proven lines by the unsound mode.
#[test]
fn the_wall_between_unsound_and_proven_lines_holds_both_ways() {
    let dir = TempDir::new("unsound-wall");
    let unsound_file = dir.join("unsound.jsonl");
    lines((1, 12), &options(1, true), &plan(1), Some(&unsound_file));
    let proven_file = dir.join("proven.jsonl");
    solve_range(1, 12, &options(1, true), Some(&proven_file)).unwrap();

    let refused = |err: String| assert!(err.contains("line 1: an unsound line"), "{err}");
    refused(ResultsFile::open(&unsound_file, Optimal::built_in(), 1).err().expect("solve refuses the file"));
    refused(solve_range(1, 13, &options(1, true), Some(&unsound_file)).unwrap_err());
    refused(verify_files(slice::from_ref(&unsound_file), Optimal::built_in(), 1).unwrap_err());
    let table = Optimal::built_in().clone();
    refused(table.clone().with_results(&unsound_file).err().expect("the table refuses it"));
    refused(table.with_results_files(slice::from_ref(&unsound_file)).err().expect("the table refuses it"));

    // An unsound line after proven ones, and a proven line after unsound ones.
    let proven = fs::read_to_string(&proven_file).unwrap();
    let unsound = fs::read_to_string(&unsound_file).unwrap();
    let mixed = dir.join("mixed.jsonl");
    fs::write(&mixed, format!("{proven}{}\n", unsound.lines().next().unwrap())).unwrap();
    let err = verify_files(slice::from_ref(&mixed), Optimal::built_in(), 1).unwrap_err();
    assert!(err.contains("line 13: an unsound line"), "{err}");
    let err = unsound::solve_range(13, 13, &options(1, true), &plan(0), Some(&proven_file), |_| ()).unwrap_err();
    assert!(err.contains("line 1: not an unsound line"), "{err}");
    fs::write(&mixed, format!("{unsound}{}\n", proven.lines().next().unwrap())).unwrap();
    let err = unsound::solve_range(13, 13, &options(1, true), &plan(0), Some(&mixed), |_| ()).unwrap_err();
    assert!(err.contains("line 13: not an unsound line"), "{err}");
}
