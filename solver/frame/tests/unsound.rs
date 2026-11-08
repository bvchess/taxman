//! The unsound mode (`taxman::solver::unsound`): its lines replay to their scores and never beat the built-in table,
//! are the same on one thread and on four, run the rounds of a lower level first, resume, and end their rounds at the
//! table's score with `--stop-at-known`; and the wall between its lines and proven ones holds both ways.  The options
//! that shape the rounds (`fix`, `tilt`, `back`, `widen`) change the line, and each of them, the last three together
//! and the defaults of the last three keep all of the above.  The repair pulls a short line up from the lines above
//! it, an unseeded game takes nothing from the games below it, and a game played twice keeps its better play.  A
//! line's bound is the parity bound, the stop on growth ends a level's rounds from the end of level 1's on, after any
//! quiet rounds it waits for, and the push plays twice a game whose gap grew.
//!
//! Most tests play the rounds with `tilt`, `back` and `widen` off (`plain`, the form of the mode before they had
//! defaults), so that each keeps testing what it tested; the options are given by name where a test is about them, and
//! the defaults are one of the shapes that `shaped` plays.  What the command line does with the defaults is in
//! `cli.rs`.
//!
//! A level of effort budgets 2^L × `round_work` work of rounds.  The tests lower `round_work` from the 65,536 of a
//! round of full neighborhoods to about half the work of a round of the small games they play, so that a level is a
//! few rounds and a test stays inside its CPU limit.

mod common;

use common::TempDir;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::slice;
use taxman::game::{replay, Optimal};
use taxman::json::Json;
use taxman::results::{verify_files, ResultsFile};
use taxman::search::finder::{NEIGHBORHOODS, NEIGHBORHOOD_WORK};
use taxman::solver::unsound::{self, Line, Origin, OtherPlay, Plan, Previous, Raised, Repair, Source};
use taxman::solver::{solve_range, Method, SolveOptions};

/// The work of a round in these tests, which level L budgets 2^L times: about half the work of a round of the small
/// games (their steps are about 80 a round, and the neighborhoods' work is 64 × 32 = 2,048), so that level 1 runs two
/// rounds and level L a few rounds more than level L − 1.
const WORK: u64 = 1_100;

/// The plan of a run at level `effort`, with the default seed and no measuring option, and with `tilt`, `back`, `widen`,
/// the repair, the games played twice and the stop and the push on growth off: the mode as it was before those options
/// had defaults.
fn plan(effort: u32) -> Plan {
    let off = Plan { tilt: 0, back: 0, widen: None, repair: false, unseeded_every: 0, ..Plan::default() };
    Plan { effort, round_work: WORK, stop_growth: None, push_growth: None, ..off }
}

/// The plan of a run at level `effort` with the defaults.
fn by_default(effort: u32) -> Plan {
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

/// The lines of the games `range` played with `plan` and `opts`, appended to the file `out` if there is one, and the
/// repairs of the run; each line and each repair replays to its score, which is no more than the table's.
fn played((lo, hi): (u32, u32), opts: &SolveOptions, plan: &Plan, out: Option<&Path>) -> (Vec<Line>, Vec<Repair>) {
    let optimal = Optimal::built_in();
    let (mut lines, mut repairs) = (Vec::new(), Vec::new());
    unsound::solve_range(
        lo,
        hi,
        opts,
        plan,
        out,
        |line| lines.push(line.clone()),
        |repair| repairs.push(repair.clone()),
    )
    .expect("a run of the unsound mode");
    for line in &lines {
        let n = line.n;
        assert_eq!(replay(n, &line.moves), Ok(line.score), "n={n}: the moves do not replay to the score");
        assert!(line.score <= optimal.score(n).unwrap(), "n={n}: {} beats the table", line.score);
        let sum: u64 = line.promotions.iter().map(|&p| u64::from(p)).sum();
        assert!(line.in_the_bag.is_none_or(|bag| bag + sum == line.score), "n={n}");
    }
    for repair in &repairs {
        let n = repair.n;
        assert_eq!(replay(n, &repair.moves), Ok(repair.score), "n={n}: the repair's moves do not replay to its score");
        assert!(repair.was < repair.score && repair.score <= optimal.score(n).unwrap(), "n={n}: {repair:?}");
    }
    (lines, repairs)
}

/// The lines of the games `range` played with `plan` and `opts`, appended to the file `out` if there is one
/// ([`played`]).
fn lines(range: (u32, u32), opts: &SolveOptions, plan: &Plan, out: Option<&Path>) -> Vec<Line> {
    played(range, opts, plan, out).0
}

/// `line` without what varies from run to run: its times (and those of its other play) and its threads.
fn timeless(line: &Line) -> Line {
    let improvements = line.improvements.iter().map(|raised| Raised { seconds: 0.0, ..*raised }).collect();
    let other = line.other.map(|other| OtherPlay { seconds: 0.0, ..other });
    Line { seconds: 0.0, start_seconds: 0.0, threads: 0, improvements, other, ..line.clone() }
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
    // The rounds end at the table's score (`stop_at_known`), which most of these small games have at their start.
    let measured = Plan { stop_at_known: true, ..plan(1) };
    let lines = lines((1, 160), &options(2, true), &measured, None);
    assert_eq!(lines.len(), 160);
    for (i, line) in lines.iter().enumerate() {
        let n = line.n;
        let json = line.to_json();
        assert_eq!(json.get("unsound"), Some(&Json::Bool(true)), "n={n}");
        assert!(json.get("completed").is_none() && json.get("proof").is_none(), "n={n}: it claims nothing");
        assert_eq!(json.get("budget"), Some(&Json::from(measured.budget())), "n={n}");
        // No play scores above the game's bound, which a board with no promotion candidates is at.
        assert!(line.score <= line.bound, "n={n}: {} is above the bound {}", line.score, line.bound);
        assert!(line.method != Method::Trivial || Some(line.bound) == line.in_the_bag, "n={n}");
        assert_eq!(json.get("bound"), Some(&Json::from(line.bound)), "n={n}");
        if n > 1 {
            let prev = line.prev.as_ref().expect("the moves for n - 1");
            assert_eq!((prev.n, prev.source, prev.short_by), (n - 1, Source::Run, 0), "n={n}");
            // The growth is the game's gap less the gap of the line below.
            let below = &lines[i - 1];
            let growth = (line.bound - line.score) as i64 - (below.bound - below.score) as i64;
            assert_eq!(line.growth, Some(growth), "n={n}");
        } else {
            assert_eq!(line.growth, None, "game 1 has no moves below it");
        }
    }
    for method in [Method::Trivial, Method::Prime, Method::Repeat, Method::Search] {
        assert!(lines.iter().any(|line| line.method == method), "no {} game", method.as_str());
    }
    let searched = lines.iter().filter(|line| line.method == Method::Search);
    assert!(searched.clone().all(|line| line.start.is_some()), "a searched game has a start");
    let at_known = |line: &Line| line.known.is_some_and(|known| known.round == Some(0));
    assert!(
        searched.clone().all(|line| line.rounds >= 1 || at_known(line)),
        "it plays a round, or starts at the table's"
    );
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
    // 375 is raised in its second round only at level 1, and again in its third at level 2, which 376 does not.
    assert_eq!((raised(&less[0]).len(), raised(&more[0]).len()), (1, 2));
    assert_eq!((raised(&less[1]).len(), raised(&more[1]).len()), (2, 2));
}

#[test]
fn the_rounds_of_a_level_run_until_their_work_reaches_its_budget_and_the_round_that_reaches_it_is_the_last() {
    let opts = options(4, false);
    let rounds = |plan: &Plan| lines(FIRST, &opts, plan, None);
    for effort in 1..=3 {
        let plan = plan(effort);
        assert_eq!(plan.budget(), WORK << effort);
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
    // A budget that the work the rounds did reaches is reached by the same rounds, the last of which reaches it, and
    // the next budget is not, and takes another round.  The budget of level 1 is twice the work of a round, so it is
    // even: the budgets are the work, if that is even, and the even numbers next to it.
    let n = FIRST.0;
    let line = &lines((n, n), &opts, &plan(3), None)[0];
    let again = |budget: u64| lines((n, n), &opts, &Plan { round_work: budget / 2, ..plan(1) }, None).remove(0);
    let reached = line.work - line.work % 2;
    let (exact, more) = (again(reached), again(reached + 2));
    assert_eq!((exact.rounds, exact.work), (line.rounds, line.work), "n={n}");
    assert!(more.rounds > line.rounds && more.work > line.work, "n={n}: {} rounds", more.rounds);
}

/// Games whose rounds, from a start made with the table's moves for n − 1, find different sets under the options that
/// shape the rounds, and each starts from the table's moves, so it is the same game whatever the lines before it.  The
/// defaults play each once but 539 at level 1: the gap of its seeded play grows by 39 over the table's, and the push
/// (more than 25) plays it twice.
const SHAPED: [u32; 4] = [375, 539, 544, 680];

/// The games of [`SHAPED`] that the check of a lower level first plays, those that the defaults play once at levels 1
/// to 3.  The line of a game played twice is the better play's, which can be the other play at the next level, so a
/// level runs the rounds of the levels below first in each play, not in the line: the seeded play is the game played
/// once, and the unseeded play is the game played unseeded alone
/// (`a_game_played_twice_has_the_line_of_the_better_play_...`).
const PLAYED_ONCE: [u32; 2] = [375, 680];

/// The games of [`SHAPED`] but 544, which keeps the check of the threads inside its CPU limit.
const ON_THREADS: [u32; 3] = [375, 539, 680];

/// The plans of the options at level `effort`, for the games [`SHAPED`] from the table's moves: a share fixed other than
/// the default's, the tilted share, trajectories from the 8 games before, the widening after a round without growth,
/// the last three together, half the neighborhoods tilted, and the defaults, but for the stop on growth, which ends the
/// rounds of a level above the first where the first does and has a test of its own.
fn shaped(effort: u32) -> [(&'static str, Plan); 7] {
    let known = Plan { from_known: true, ..plan(effort) };
    [
        ("fix", Plan { fix: (0.4, 0.6), ..known }),
        ("tilt", Plan { tilt: 100, ..known }),
        ("back", Plan { back: 8, ..known }),
        ("widen", Plan { widen: Some(1), ..known }),
        ("tilt, back and widen", Plan { tilt: 100, back: 8, widen: Some(1), ..known }),
        ("half tilt", Plan { tilt: 50, ..known }),
        ("the defaults", Plan { from_known: true, stop_growth: None, ..by_default(effort) }),
    ]
}

/// The lines of `games` (of [`SHAPED`]) played with `plan`, which starts them from the table's moves.
fn shaped_lines(games: &[u32], opts: &SolveOptions, plan: &Plan) -> Vec<Line> {
    assert!(plan.from_known);
    games.iter().flat_map(|&n| lines((n, n), opts, plan, None)).collect()
}

#[test]
fn with_the_options_off_the_rounds_are_these() {
    // The lines of this form of the mode at level 3 (`--back 0 --tilt 0 --widen 0`), to see that no change to a
    // neighborhood's draws or to the budget goes unseen: the neighborhoods of the finder's default are pinned beside, in
    // `search::finder`.
    let plan = Plan { from_known: true, ..plan(3) };
    let lines = shaped_lines(&SHAPED, &options(4, true), &plan);
    let now: Vec<_> = lines.iter().map(|line| (line.n, line.score, line.rounds, line.steps, raised(line))).collect();
    let was = [
        (375, 44328, 5, 394, vec![(2, 44327), (3, 44328)]),
        (539, 91703, 5, 394, vec![(2, 91703)]),
        (544, 93256, 5, 429, vec![(1, 93256)]),
        (680, 146798, 5, 326, vec![(1, 146773), (4, 146784), (5, 146798)]),
    ];
    assert_eq!(now, was);
    for line in &lines {
        assert_eq!((line.plan.fix, line.plan.tilt, line.plan.back, line.plan.widen), ((0.7, 0.9), 0, 0, None));
        assert_eq!(line.trajectories, Some(vec![Origin::Start]), "n={}: the start's trajectory alone", line.n);
        assert_eq!(line.stage_rounds, Some(vec![[line.rounds, 0, 0]]), "n={}: every round at stage 0", line.n);
        assert!(line.improvements.iter().all(|raised| (raised.trajectory, raised.stage) == (0, 0)), "n={}", line.n);
    }
}

/// The module `$module` of seven tests, `fix`, `tilt`, `back`, `widen`, `all_three`, `half_tilt` and `defaults`, each
/// calling `$check` with the index of that shape in [`shaped`]: one test of all seven would use more CPU than a test
/// may.
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
            #[test]
            fn half_tilt() {
                super::$check(5);
            }
            #[test]
            fn defaults() {
                super::$check(6);
            }
        }
    };
}

/// The lines of the games [`ON_THREADS`] under option `shape` of [`shaped`] are the same on one thread and on four.
fn check_same_on_one_thread_and_on_four(shape: usize) {
    let shape = shaped(3)[shape].1;
    let one = shaped_lines(&ON_THREADS, &options(1, true), &shape);
    let four = shaped_lines(&ON_THREADS, &options(4, true), &shape);
    assert_eq!(timeless_all(&one), timeless_all(&four));
    assert!(one.iter().all(|line| line.threads == 1) && four.iter().all(|line| line.threads == 4));
}
for_each_option!(same_on_one_thread_and_on_four, check_same_on_one_thread_and_on_four);

/// Under option `shape` of [`shaped`], the improvements of the games [`PLAYED_ONCE`] at level 1 are a prefix of those
/// at level 2, and those at level 2 of those at level 3: the rounds, the trajectories, the stages, the steps and the
/// scores.
fn check_lower_level_first(shape: usize) {
    let by_level: Vec<Vec<Line>> =
        (1..=3).map(|effort| shaped_lines(&PLAYED_ONCE, &options(4, true), &shaped(effort)[shape].1)).collect();
    let twice: Vec<_> = by_level.iter().flatten().filter(|line| line.other.is_some()).map(|line| line.n).collect();
    assert!(twice.is_empty(), "each game is played once: {twice:?}");
    for (less, more) in by_level.iter().zip(&by_level[1..]) {
        for (less, more) in less.iter().zip(more) {
            let (n, raised) = (less.n, (raised_by(less), raised_by(more)));
            assert!(raised.1.starts_with(&raised.0), "n={n}: {:?}, then {:?}", raised.0, raised.1);
            assert!(more.rounds >= less.rounds && more.work >= less.work, "n={n}");
        }
    }
    // The longer run does more, so the comparison shows something.
    let rounds: Vec<u64> = by_level.iter().map(|lines| lines.iter().map(|line| line.rounds).sum()).collect();
    assert!(rounds[0] < rounds[2], "rounds at levels 1 to 3: {rounds:?}");
}
for_each_option!(lower_level_first, check_lower_level_first);

/// Option `shape` of [`shaped`] is in the line: the plan, the trajectories the game followed, the rounds of each at
/// each stage, and what each improvement says of both.
fn check_what_the_line_records(shape: usize) {
    let (what, shape) = shaped(3)[shape];
    let lines = shaped_lines(&SHAPED, &options(4, true), &shape);
    for line in &lines {
        let (n, json) = (line.n, line.to_json());
        assert_eq!(line.plan, shape, "{what}");
        assert_eq!(json.get("fix"), Some(&Json::array([shape.fix.0, shape.fix.1])), "n={n}");
        assert_eq!(json.get("tilt"), Some(&Json::from(shape.tilt)), "n={n}");
        assert_eq!(json.get("back"), Some(&Json::from(shape.back)), "n={n}");
        assert_eq!(json.get("widen"), Some(&shape.widen.map_or(Json::Null, Json::from)), "n={n}");
        assert_eq!(json.get("budget"), Some(&Json::from(shape.budget())), "n={n}");
        assert_eq!(json.get("unseeded_every"), Some(&Json::from(shape.unseeded_every)), "n={n}");
        let twice = shape.unseeded_every > 0 && n.is_multiple_of(shape.unseeded_every);
        assert_eq!(line.other.is_some(), twice || line.pushed, "n={n}: played twice if a multiple of K, or pushed");
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
    // With a widening of 4 rounds, the few rounds of this level may not reach a stage below the first, so only a quicker
    // widening is held to it.
    if shape.widen.is_some_and(|rounds| rounds <= 2) {
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
        shaped_lines(&SHAPED, &options(4, true), &shape).into_iter().map(|line| line.trajectories.unwrap()).collect()
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
    let back = Plan { back: 8, from_known: true, stop_at_known: true, ..plan(1) };
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
        let more = &lines((n, n), &opts, &Plan { effort: 2, stop_at_known: false, ..back }, None)[0];
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
    let refused =
        |plan: Plan| unsound::solve_range(12, 12, &options(1, true), &plan, None, |_| (), |_| ()).unwrap_err();
    for fix in [(0.9, 0.7), (-0.1, 0.5), (0.5, 1.1), (f64::NAN, 0.5), (0.5, f64::INFINITY)] {
        let err = refused(Plan { fix, ..plan(1) });
        assert!(err.starts_with("--fix takes LO,HI") && err.contains("0 <= LO <= HI <= 1"), "{fix:?}: {err}");
    }
    for tilt in [101, 1000] {
        let err = refused(Plan { tilt, ..plan(1) });
        assert!(err.starts_with("--tilt takes P, a whole number of percent from 0 to 100"), "{tilt}: {err}");
    }
    for back in [9, 100] {
        assert!(refused(Plan { back, ..plan(1) }).starts_with("--back takes J, a whole number from 0 to 8"), "{back}");
    }
    // No widening is `None` (`--widen 0`), and a plan that widens after no rounds is refused.
    assert!(refused(Plan { widen: Some(0), ..plan(1) }).starts_with("the plan widens after 0 rounds"));
    // The ladder of the widening replaces the share fixed.
    for fix in [(0.5, 0.7), (0.4, 0.6)] {
        assert_eq!(refused(Plan { widen: Some(2), fix, ..plan(1) }), unsound::WIDEN_WITH_FIX, "{fix:?}");
    }
    // The levels are 1 to 6: the start alone is no level, and nothing above the most is.
    for effort in [0, unsound::MAX_EFFORT + 1] {
        let err = refused(Plan { effort, ..plan(1) });
        assert_eq!(err, format!("--effort takes L, the level of effort, a whole number from 1 to 6: got \"{effort}\""));
    }
    // The least and the most effort are accepted: the most budgets 2^22 work.
    let (least, most) = (Plan { effort: 1, ..plan(1) }, Plan { effort: 6, round_work: unsound::ROUND_WORK, ..plan(1) });
    assert_eq!((unsound::check_plan(&least), unsound::check_plan(&most)), (Ok(()), Ok(())));
    assert_eq!((least.budget(), most.budget()), (2 * WORK, 1 << 22));
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

/// A run makes the setup of each game but its first while the game before plays, and that is the setup the game makes
/// on its own: from the table's moves, each searched game of [`SECOND`] has the line it has alone, as the first game
/// of a run, and 386 repeats the problem that 385's setup posed.
#[test]
fn a_setup_made_while_the_game_before_plays_is_the_one_the_game_makes() {
    let (opts, plan) = (options(4, true), Plan { back: 3, ..from_table(1) });
    let range = lines(SECOND, &opts, &plan, None);
    let alone: Vec<Line> = range.iter().flat_map(|line| lines((line.n, line.n), &opts, &plan, None)).collect();
    let searched: Vec<u32> = range.iter().filter(|line| line.method == Method::Search).map(|line| line.n).collect();
    assert_eq!(searched, [384, 385, 387]);
    for (in_range, alone) in range.iter().zip(&alone).filter(|(line, _)| line.method == Method::Search) {
        assert_eq!(timeless(in_range), timeless(alone), "n={}", in_range.n);
    }
    assert_eq!((range[2].method, range[2].repeats), (Method::Repeat, Some(385)));
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

/// The plan of a run at level `effort` from the table's moves for n − 1, so that the run's ceiling is the proven one.
fn from_table(effort: u32) -> Plan {
    Plan { from_known: true, ..plan(effort) }
}

/// The proven ceiling of game `n`: n + the table's score for n − 1.
fn proven_ceiling(n: u32) -> u64 {
    Optimal::built_in().score(n - 1).unwrap() + u64::from(n)
}

/// Games whose start from the table's moves for n − 1 is at the run's ceiling: the proven mode finds each with no
/// search (method `start`, proof `cap`).
const AT_CEILING_AT_THE_START: [u32; 4] = [18, 28, 33, 42];

/// Games whose start, made without the moves for n − 1 (`options(_, false)`), falls short of the run's ceiling, and the
/// round in which the rounds reach it: 378 in its third, 385 in its second and 387 in its first.
const AT_CEILING_IN_A_ROUND: [(u32, u64); 3] = [(378, 3), (385, 2), (387, 1)];

#[test]
fn a_game_whose_start_reaches_the_runs_ceiling_stops_before_any_round() {
    let opts = options(4, true);
    for n in AT_CEILING_AT_THE_START {
        let ceiling = proven_ceiling(n);
        // The start is at the ceiling, whatever the level: no round and no work.
        for effort in [1, 3] {
            let line = &lines((n, n), &opts, &from_table(effort), None)[0];
            assert_eq!((line.method, line.start, line.score), (Method::Search, Some(ceiling), ceiling), "n={n}");
            assert_eq!((line.ceiling, line.at_ceiling), (Some(ceiling), true), "n={n} at level {effort}");
            assert_eq!((line.rounds, line.steps, line.work), (0, 0, 0), "n={n} at level {effort}");
            assert!(line.improvements.is_empty(), "n={n}");
            let json = line.to_json();
            assert_eq!(json.get("ceiling"), Some(&Json::from(ceiling)), "n={n}");
            assert_eq!(json.get("at_ceiling"), Some(&Json::Bool(true)), "n={n}");
        }
        // With --past-ceiling the game searches its budget, and finds nothing above the proven ceiling.
        let past = Plan { past_ceiling: true, ..from_table(3) };
        let line = &lines((n, n), &opts, &past, None)[0];
        assert_eq!((line.ceiling, line.at_ceiling, line.score), (None, false, ceiling), "n={n}");
        assert!(line.rounds >= 1 && line.work >= past.budget(), "n={n}: {} rounds, {} work", line.rounds, line.work);
        assert!(line.improvements.is_empty(), "n={n}");
        let json = line.to_json();
        assert_eq!((json.get("ceiling"), json.get("at_ceiling")), (Some(&Json::Null), Some(&Json::Bool(false))));
    }
}

#[test]
fn a_game_that_reaches_the_runs_ceiling_in_a_round_stops_after_that_round() {
    let opts = options(4, false);
    for (n, round) in AT_CEILING_IN_A_ROUND {
        let ceiling = proven_ceiling(n);
        let stopped = &lines((n, n), &opts, &from_table(3), None)[0];
        assert_eq!((stopped.ceiling, stopped.at_ceiling, stopped.score), (Some(ceiling), true, ceiling), "n={n}");
        assert!(stopped.start.unwrap() < ceiling, "n={n}: the start falls short of it");
        // The round that reaches it is the last, well before the budget.
        assert_eq!((stopped.rounds, raised(stopped).last()), (round, Some(&(round, ceiling))), "n={n}");
        assert!(stopped.work < from_table(3).budget(), "n={n}: {} work", stopped.work);
        // A higher level of effort stops at the same round, and a lower one is at the ceiling only if its budget lets it
        // do those rounds.
        for effort in [2, 4] {
            let more = &lines((n, n), &opts, &from_table(effort), None)[0];
            let shown =
                |line: &Line| (line.rounds, line.steps, line.work, line.score, raised_by(line), line.at_ceiling);
            assert_eq!(shown(more), shown(stopped), "n={n} at level {effort}");
        }
        let less = &lines((n, n), &opts, &from_table(1), None)[0];
        assert!(less.rounds <= stopped.rounds, "n={n}");
        assert_eq!(less.at_ceiling, less.rounds == stopped.rounds, "n={n}");
        // Without the stop the game goes on to its budget, and the rounds before the stop are the same.
        let past = &lines((n, n), &opts, &Plan { past_ceiling: true, ..from_table(3) }, None)[0];
        assert_eq!((past.ceiling, past.at_ceiling, past.score), (None, false, ceiling), "n={n}");
        assert!(past.rounds > stopped.rounds && past.work >= from_table(3).budget(), "n={n}: {} rounds", past.rounds);
        assert_eq!(raised_by(past), raised_by(stopped), "n={n}");
    }
}

/// The line of game 395 as a run of level 1 plays it from game 390 on, without the moves for n − 1 ([`WALKED`] has the
/// run: 390 falls 7 short of the table's score and the games after it keep the shortfall), and the file `name` of `dir`
/// that holds that line alone, so that no line of 390 to 394 repairs with it.
fn short_395_in(dir: &TempDir, name: &str, threads: usize) -> (Line, PathBuf) {
    let walked = dir.join(format!("walked-{name}"));
    let short = lines((WALKED.0, 395), &options(threads, false), &plan(1), Some(&walked)).pop().expect("a line");
    let text = fs::read_to_string(&walked).unwrap();
    let file = dir.join(name);
    fs::write(&file, format!("{}\n", text.lines().last().expect("the line of 395"))).unwrap();
    (short, file)
}

/// The lines of game 395 and then of game 396, played from a file that holds a short line of 395, on `threads` threads:
/// 395, played from game 390 on without the moves for n − 1 ([`short_395_in`]), falls 7 short of the table's score; 396
/// then reads the file's line, not the table's, and stops at the lower ceiling that the line gives.
fn from_a_short_line(threads: usize, past_ceiling: bool) -> (Line, Line) {
    let (dir, opts) = (TempDir::new("unsound-short"), options(threads, false));
    let (short, file) = short_395_in(&dir, "short.jsonl", threads);
    let game = lines((396, 396), &opts, &Plan { past_ceiling, ..plan(1) }, Some(&file)).remove(0);
    (short, game)
}

#[test]
fn a_game_seeded_with_a_short_line_for_n_minus_1_stops_at_the_lower_ceiling_of_those_moves() {
    let (short, stopped) = from_a_short_line(4, false);
    let table = Optimal::built_in();
    assert_eq!(short.score + 7, table.score(395).unwrap(), "395's line is 7 short of optimal");
    let prev = stopped.prev.as_ref().unwrap();
    assert_eq!((prev.n, prev.score, prev.source), (395, short.score, Source::Results));
    // The ceiling is that of the moves in the file, below the proven one, and the game stops at it.
    let ceiling = 396 + short.score;
    assert_eq!((stopped.ceiling, stopped.at_ceiling), (Some(ceiling), true));
    assert!(ceiling < proven_ceiling(396));
    assert!(stopped.start.unwrap() < ceiling, "the start falls short of it");
    assert_eq!((stopped.rounds, raised(&stopped).len()), (1, 1), "the first round reaches it");
    assert!(stopped.work < plan(1).budget(), "{} work: it stopped before its budget", stopped.work);
    // That round passes the ceiling, which proves the line of 395 short by as much at least.
    assert!(stopped.score > ceiling);
    assert_eq!(prev.short_by, stopped.score - ceiling);
    // Without the stop the game goes on to its budget.
    let (_, past) = from_a_short_line(4, true);
    assert_eq!((past.ceiling, past.at_ceiling), (None, false));
    assert!(past.rounds > stopped.rounds && past.work >= plan(1).budget());
    assert!(past.score >= stopped.score && past.score <= table.score(396).unwrap());
}

#[test]
fn the_lines_that_stop_at_the_ceiling_are_the_same_on_one_thread_and_on_four() {
    let played = |threads| {
        let mut played = Vec::new();
        for n in AT_CEILING_AT_THE_START {
            played.extend(lines((n, n), &options(threads, true), &from_table(3), None));
        }
        for (n, _) in AT_CEILING_IN_A_ROUND {
            played.extend(lines((n, n), &options(threads, false), &from_table(3), None));
        }
        played.push(from_a_short_line(threads, false).1);
        played
    };
    let (one, four) = (played(1), played(4));
    assert_eq!(timeless_all(&one), timeless_all(&four));
    assert!(one.iter().all(|line| line.at_ceiling));
    assert!(one.iter().all(|line| line.threads == 1) && four.iter().all(|line| line.threads == 4));
}

#[test]
fn the_measuring_options_take_the_tables_games_only() {
    let past = Optimal::built_in().max_n() + 1;
    for measured in [Plan { stop_at_known: true, ..plan(1) }, Plan { from_known: true, ..plan(1) }] {
        let err = unsound::solve_range(past - 1, past, &options(1, true), &measured, None, |_| (), |_| ()).unwrap_err();
        assert!(err.contains(&format!("lacks game {past}")), "{err}");
    }
    // With --from-known every game starts from the table's moves for n − 1.
    let known = lines(FIRST, &options(1, true), &Plan { from_known: true, ..plan(1) }, None);
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
    let err =
        unsound::solve_range(13, 13, &options(1, true), &plan(1), Some(&proven_file), |_| (), |_| ()).unwrap_err();
    assert!(err.contains("line 1: not an unsound line"), "{err}");
    fs::write(&mixed, format!("{unsound}{}\n", proven.lines().next().unwrap())).unwrap();
    let err = unsound::solve_range(13, 13, &options(1, true), &plan(1), Some(&mixed), |_| (), |_| ()).unwrap_err();
    assert!(err.contains("line 13: not an unsound line"), "{err}");
}

/// The plan of [`plan`] at level `effort` with the repair.
fn repairing(effort: u32) -> Plan {
    Plan { repair: true, ..plan(effort) }
}

/// `json` without what varies from run to run: the times (the game's, each improvement's and a repair's), the threads
/// and the build; and without `repair`, the option, where `optionless`.
fn timeless_json(json: &Json, optionless: bool) -> Json {
    const VARIES: [&str; 4] = ["seconds", "start_seconds", "threads", "build"];
    match json {
        Json::Obj(pairs) => Json::Obj(
            pairs
                .iter()
                .filter(|(key, _)| !VARIES.contains(&key.as_str()) && !(optionless && key == "repair"))
                .map(|(key, value)| (key.clone(), timeless_json(value, false)))
                .collect(),
        ),
        Json::Arr(items) => Json::Arr(items.iter().map(|item| timeless_json(item, false)).collect()),
        other => other.clone(),
    }
}

/// The lines of the file `path`, in order, without what varies from run to run ([`timeless_json`]).
fn file_lines(path: &Path, optionless: bool) -> Vec<Json> {
    let text = fs::read_to_string(path).unwrap();
    text.lines().map(|line| timeless_json(&Json::parse(line).unwrap(), optionless)).collect()
}

/// The last line of each game of the file `path` ([`file_lines`]), the game's.
fn games_of(path: &Path, optionless: bool) -> Vec<(u64, Json)> {
    let mut games = BTreeMap::new();
    for line in file_lines(path, optionless) {
        games.insert(line.get("n").and_then(Json::as_u64).unwrap(), line);
    }
    games.into_iter().collect()
}

/// Games 390 to 396 at level 1, with no seed: the search of 390 falls 7 short of the table's score, 391 to 395 keep
/// the shortfall, as repeats and as searches stopped at the lower ceiling it gives, and 396 reaches the table's score.
const WALKED: (u32, u32) = (390, 396);

#[test]
fn a_short_line_is_repaired_from_the_line_of_the_game_above_it() {
    let (dir, table) = (TempDir::new("unsound-repair"), Optimal::built_in());
    // 395 falls 7 short, played from 390 on with no seed ([`short_395_in`]); 396, seeded with nothing, passes the
    // ceiling that the short line gives, and what survives of its line in 395 scores more than the line of 395.
    let (short, file) = short_395_in(&dir, "repaired.jsonl", 4);
    let (game, repairs) = played((396, 396), &options(4, false), &repairing(1), Some(&file));
    assert_eq!(short.score + 7, table.score(395).unwrap());
    assert!(game[0].prev.as_ref().unwrap().short_by > 0, "396 proves the line of 395 short");
    let [repair] = &repairs[..] else { panic!("one repair: {repairs:?}") };
    assert_eq!((repair.n, repair.from, repair.method, repair.was), (395, 396, short.method, short.score));
    assert!(repair.score > short.score && repair.score <= table.score(395).unwrap(), "{repair:?}");
    // The file has the line of 395, then the line of 396, then the line of 395 again: the line played with the
    // repair's moves, which is the game's.
    let written = file_lines(&file, false);
    assert_eq!(
        written.iter().map(|line| line.get("n").and_then(Json::as_u64).unwrap()).collect::<Vec<_>>(),
        [395, 396, 395]
    );
    let (first, again) = (&written[0], &written[2]);
    assert_eq!(again.get("score"), Some(&Json::from(repair.score)));
    assert_eq!(again.get("moves"), Some(&Json::array(repair.moves.iter().copied())));
    let repaired = again.get("repaired").unwrap();
    assert_eq!(
        (repaired.get("from"), repaired.get("was")),
        (Some(&Json::from(396u32)), Some(&Json::from(short.score)))
    );
    assert_eq!(first.get("repaired"), Some(&Json::Null));
    for key in ["method", "start", "rounds", "work", "improvements", "setup", "ceiling"] {
        assert_eq!(again.get(key), first.get(key), "{key} is the line played's");
    }
    // A game started from a repaired line starts from the line the file ends with.  In a file with the short line of
    // 395 alone, the prime 397, played from the table's moves for 396, repairs it two games below, and a run of 396
    // then starts from the repaired line.
    let (_, gap) = short_395_in(&dir, "gap.jsonl", 4);
    let (_, repairs) = played((397, 397), &options(4, false), &repairing(1), Some(&gap));
    assert_eq!(repairs.iter().map(|r| (r.n, r.from)).collect::<Vec<_>>(), [(395, 397)]);
    let next = lines((396, 396), &options(4, false), &repairing(1), Some(&gap)).remove(0);
    let prev = next.prev.unwrap();
    assert_eq!((prev.n, prev.score, prev.source), (395, repairs[0].score, Source::Results));
}

/// `line` with the source of the moves for n − 1 that it started from named as this run's, as a run in one go would
/// name the file's line of the game before the break.
fn as_one_run(line: Json) -> Json {
    let Json::Obj(pairs) = line else { return line };
    let pairs = pairs.into_iter().map(|(key, value)| match (key.as_str(), value) {
        ("prev", Json::Obj(prev)) => {
            let prev = prev.into_iter().map(|(key, value)| match (key.as_str(), value) {
                ("source", Json::Str(source)) if source == "results" => (key, Json::from("run")),
                (_, value) => (key, value),
            });
            (key, Json::Obj(prev.collect()))
        }
        (_, value) => (key, value),
    });
    Json::Obj(pairs.collect())
}

#[test]
fn the_repair_walks_down_from_a_line_through_each_game_it_raises() {
    let (dir, table) = (TempDir::new("unsound-walk"), Optimal::built_in());
    let file = dir.join("walked.jsonl");
    let (lines, repairs) = played(WALKED, &options(4, false), &repairing(1), Some(&file));
    let short: Vec<u64> = lines.iter().map(|line| table.score(line.n).unwrap() - line.score).collect();
    assert_eq!(short, [7, 7, 7, 7, 7, 7, 0]);
    // After 396, the walk raises 395 from it, then 394 from 395, and so on down to 390, each by 5, from the top down.
    let walked: Vec<(u32, u32, u64)> = repairs.iter().map(|r| (r.n, r.from, r.score - r.was)).collect();
    assert_eq!(walked, [(395, 396, 5), (394, 395, 5), (393, 394, 5), (392, 393, 5), (391, 392, 5), (390, 391, 5)]);
    let methods: Vec<Method> = repairs.iter().map(|r| r.method).collect();
    assert_eq!(
        methods,
        [Method::Repeat, Method::Repeat, Method::Repeat, Method::Search, Method::Repeat, Method::Search]
    );
    // The last line of each game is the game's: the repair's, which records where it came from, or the line played.
    for ((n, line), played) in games_of(&file, false).into_iter().zip(&lines) {
        let (from, score) = match repairs.iter().find(|r| r.n == played.n) {
            Some(repair) => (Json::from(repair.from), repair.score),
            None => (Json::Null, played.score),
        };
        let repaired = line.get("repaired").unwrap();
        assert_eq!(
            (repaired.get("from").unwrap_or(repaired), line.get("score")),
            (&from, Some(&Json::from(score))),
            "n={n}"
        );
    }
    // Without the repair the run writes the lines played alone, the same but for the option.
    let unrepaired = dir.join("unrepaired.jsonl");
    let (again, none) = played(WALKED, &options(4, false), &plan(1), Some(&unrepaired));
    assert!(none.is_empty());
    let without =
        |lines: &[Line]| lines.iter().map(|line| Line { plan: plan(1), ..timeless(line) }).collect::<Vec<_>>();
    assert_eq!(without(&again), without(&lines));
}

#[test]
fn a_run_takes_up_the_repair_from_its_file() {
    let dir = TempDir::new("unsound-repair-resume");
    let opts = options(4, false);
    let whole = dir.join("whole.jsonl");
    played((WALKED.0, WALKED.1 + 1), &opts, &repairing(1), Some(&whole));
    // A file whose last line was never weighed against the lines below it: the next run walks down from it first.
    let unweighed = dir.join("unweighed.jsonl");
    played(WALKED, &opts, &plan(1), Some(&unweighed));
    let (_, repairs) = played((WALKED.0, WALKED.1 + 1), &opts, &repairing(1), Some(&unweighed));
    assert_eq!(repairs.iter().map(|r| r.n).collect::<Vec<_>>(), [395, 394, 393, 392, 391, 390]);
    let one_run =
        |games: Vec<(u64, Json)>| games.into_iter().map(|(n, line)| (n, as_one_run(line))).collect::<Vec<_>>();
    assert_eq!(one_run(games_of(&unweighed, true)), one_run(games_of(&whole, true)));
    // A walk that a stop cut short after its first step: the file ends with the line of 396 and the repair of 395.
    let cut = dir.join("cut.jsonl");
    played(WALKED, &opts, &repairing(1), Some(&cut));
    let text = fs::read_to_string(&cut).unwrap();
    let kept: Vec<&str> = text.lines().take(8).collect();
    fs::write(&cut, kept.join("\n") + "\n").unwrap();
    let (_, repairs) = played((WALKED.0, WALKED.1 + 1), &opts, &repairing(1), Some(&cut));
    assert_eq!(repairs.iter().map(|r| r.n).collect::<Vec<_>>(), [394, 393, 392, 391, 390], "it goes on from 394");
    let one_run = |path: &Path| file_lines(path, false).into_iter().map(as_one_run).collect::<Vec<_>>();
    assert_eq!(one_run(&cut), one_run(&whole), "line for line, the file of a run in one go");
}

#[test]
fn the_lines_of_a_run_that_repairs_are_the_same_on_one_thread_and_on_four() {
    let dir = TempDir::new("unsound-repair-threads");
    let (one, four) = (dir.join("one.jsonl"), dir.join("four.jsonl"));
    let (_, repairs) = played(WALKED, &options(1, false), &repairing(1), Some(&one));
    played(WALKED, &options(4, false), &repairing(1), Some(&four));
    assert_eq!(repairs.len(), 6);
    assert_eq!(file_lines(&one, false), file_lines(&four, false));
}

#[test]
fn a_file_holds_a_second_line_of_a_game_only_as_a_repair_that_scores_more() {
    let dir = TempDir::new("unsound-repair-file");
    let file = dir.join("walked.jsonl");
    played(WALKED, &options(4, false), &repairing(1), Some(&file));
    let text = fs::read_to_string(&file).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let (played_395, repair_395) = (lines[5], lines[7]);
    let refused = |text: String, expected: &str| {
        let bad = dir.join("bad.jsonl");
        fs::write(&bad, text).unwrap();
        let err = unsound::solve_range(400, 400, &options(1, true), &plan(1), Some(&bad), |_| (), |_| ()).unwrap_err();
        assert!(err.contains(expected), "{err}");
    };
    refused(format!("{played_395}\n{played_395}\n"), "line 2: a second line for n=395 that is no repair");
    refused(format!("{repair_395}\n"), "line 1: a repair of n=395, which has no line before it");
    refused(format!("{played_395}\n{repair_395}\n{played_395}\n"), "line 3: a second line for n=395 that is no repair");
    // A repair of a line other than the one before it, and one that scores no more than the line it replaced.
    let was = |was: u64| {
        let Json::Obj(pairs) = Json::parse(repair_395).unwrap() else { panic!("an object") };
        let repaired =
            Json::object([("from", Json::from(396u32)), ("was", Json::from(was)), ("seconds", Json::Num(0.0))]);
        let line = pairs
            .into_iter()
            .map(|(key, value)| if key == "repaired" { (key, repaired.clone()) } else { (key, value) });
        Json::Obj(line.collect()).to_string()
    };
    refused(format!("{played_395}\n{}\n", was(1)), "line 2: a repair of a line of n=395 that scored 1, not");
    let score = Json::parse(repair_395).unwrap().get("score").and_then(Json::as_u64).unwrap();
    refused(format!("{played_395}\n{}\n", was(score)), "line 2: (n=395) it records a repair, and scores");
}

/// Games 383 to 387, which a run that takes the moves below plays as a prime (383), searches and a repeat of 385
/// (386).
const UNSEEDED: (u32, u32) = (383, 387);

#[test]
fn an_unseeded_game_takes_nothing_from_the_games_below() {
    let opts = options(4, true);
    let seeded = lines(UNSEEDED, &opts, &Plan { back: 8, ..plan(1) }, None);
    let methods: Vec<Method> = seeded.iter().map(|line| line.method).collect();
    assert_eq!(methods, [Method::Prime, Method::Search, Method::Search, Method::Repeat, Method::Search]);
    // Unseeded, every game is searched, from its start alone, with no moves for n − 1 and no ceiling.
    let unseeded = Plan { unseeded: true, back: 8, ..plan(1) };
    let range = lines(UNSEEDED, &opts, &unseeded, None);
    for line in &range {
        let n = line.n;
        assert_eq!((line.method, line.unseeded, line.other), (Method::Search, true, None), "n={n}");
        assert_eq!((&line.prev, line.ceiling), (&None, None), "n={n}");
        assert_eq!(line.trajectories, Some(vec![Origin::Start]), "n={n}");
        assert_eq!(line.to_json().get("unseeded"), Some(&Json::Bool(true)), "n={n}");
    }
    // Its line is the one it has alone, as the first game of a run, and the one it has after the table's games.
    let alone: Vec<Line> = range.iter().flat_map(|line| lines((line.n, line.n), &opts, &unseeded, None)).collect();
    let known = lines(UNSEEDED, &opts, &Plan { from_known: true, ..unseeded }, None);
    let planless =
        |lines: &[Line]| lines.iter().map(|line| Line { plan: unseeded, ..timeless(line) }).collect::<Vec<_>>();
    assert_eq!(planless(&range), planless(&alone));
    assert_eq!(planless(&range), planless(&known));
}

/// The plan of a run at effort 1 that repairs and plays every game whose n is a multiple of `every` twice.
fn twice(every: u32) -> Plan {
    Plan { unseeded_every: every, ..repairing(1) }
}

/// The score of each game of the file `path` (its last line), less the table's.
fn short_in(path: &Path) -> Vec<u64> {
    let table = Optimal::built_in();
    let score =
        |(n, line): (u64, Json)| table.score(n as u32).unwrap() - line.get("score").and_then(Json::as_u64).unwrap();
    games_of(path, false).into_iter().map(score).collect()
}

#[test]
fn a_game_played_twice_has_the_line_of_the_better_play_and_the_repair_walks_down_from_it() {
    let (dir, opts) = (TempDir::new("unsound-twice"), options(4, false));
    // Played once, the games of `WALKED` hand 390's shortfall of 7 on to 395; from 396, the repair takes 5 of it back.
    let once = dir.join("once.jsonl");
    played(WALKED, &opts, &twice(0), Some(&once));
    assert_eq!(short_in(&once), [2, 2, 2, 2, 2, 2, 0]);
    // Played twice, the even games: the unseeded play of 392 is not held to 391's ceiling, and reaches the table's
    // score, 7 above the seeded play, which stopped there.  It is kept, the repair walks down from it to 390, and the
    // games above start from it.
    let file = dir.join("twice.jsonl");
    let (lines, repairs) = played(WALKED, &opts, &twice(2), Some(&file));
    assert_eq!(short_in(&file), [0; 7]);
    for line in &lines {
        assert_eq!(line.other.is_some(), line.n % 2 == 0, "n={}", line.n);
    }
    let game = &lines[2];
    let other = game.other.expect("392 is played twice");
    assert_eq!(
        (game.n, game.unseeded, other.unseeded, other.method, game.score - other.score),
        (392, true, false, Method::Search, 7)
    );
    assert_eq!((&game.prev, game.ceiling, game.trajectories.clone()), (&None, None, Some(vec![Origin::Start])));
    assert_eq!(repairs.iter().map(|r| (r.n, r.from)).collect::<Vec<_>>(), [(391, 392), (390, 391)]);
    // Elsewhere the seeded play is kept, on a tie too (390 and 396), and the line says what the unseeded play scored.
    for line in lines.iter().filter(|line| line.n != 392 && line.other.is_some()) {
        let other = line.other.unwrap();
        assert!(!line.unseeded && other.unseeded && other.score <= line.score, "n={}: {other:?}", line.n);
    }
    // The unseeded play is the game played unseeded alone.
    let alone = self::lines((392, 392), &opts, &Plan { unseeded: true, ..repairing(1) }, None).remove(0);
    let play = |line: &Line| (line.score, line.moves.clone(), line.rounds, line.work, raised_by(line));
    assert_eq!(play(game), play(&alone));
    // On one thread, the same file.
    let one = dir.join("one.jsonl");
    played(WALKED, &options(1, false), &twice(2), Some(&one));
    assert_eq!(file_lines(&one, false), file_lines(&file, false));
}

/// The parity bound of the games 373 to 396, the prover's bound on the whole board, as the notebook's entry "The parity
/// bound against the frame bound (2026-10)" computed it for every board from 2 to 8191 (`parity_2_8191.jsonl.gz`).
const PARITY: [u64; 24] = [
    43737, 44111, 44339, 44543, 44667, 45045, 45051, 45278, 45433, 45815, 45819, 46011, 46396, 46782, 47169, 47557,
    47563, 47799, 47943, 48335, 48474, 48868, 49044, 49438,
];

/// `lines` with `plan` in place of each line's plan, and without what varies from run to run.
fn with_plan(lines: &[Line], plan: Plan) -> Vec<Line> {
    lines.iter().map(|line| Line { plan, ..timeless(line) }).collect()
}

#[test]
fn a_games_bound_is_the_parity_bound_and_the_first_games_gap_grows_over_the_tables_line() {
    let table = Optimal::built_in();
    let lines = lines((374, 396), &options(2, true), &Plan { stop_at_known: true, ..plan(1) }, None);
    // Searched games, repeats and the primes 379, 383 and 389, whose bound is that of the board set up for it.
    let bounds: Vec<u64> = lines.iter().map(|line| line.bound).collect();
    assert_eq!(bounds, PARITY[1..]);
    assert!(
        lines.iter().any(|line| line.method == Method::Prime) && lines.iter().any(|line| line.method == Method::Repeat)
    );
    // The first game starts from the table's line of 373, whose gap the run takes from the bound of 373's board.
    let gap_below = PARITY[0] - table.score(373).unwrap();
    assert_eq!(lines[0].growth, Some((lines[0].bound - lines[0].score) as i64 - gap_below as i64));
}

/// The plan of [`from_table`] at level `effort` with the stop on growth `stop_growth`, which waits for no quiet rounds:
/// the stop of the band alone.
fn stopping(effort: u32, stop_growth: Option<u64>) -> Plan {
    Plan { stop_growth, stop_quiet: 0, ..from_table(effort) }
}

/// The plan of [`from_table`] at level `effort` whose stop on growth has no limit on the growth, and waits for
/// `stop_quiet` rounds in a row that raised nothing: the stop of the quiet rounds alone.
fn waiting(effort: u32, stop_quiet: u64) -> Plan {
    Plan { stop_quiet, ..stopping(effort, Some(u64::MAX)) }
}

/// With no quiet rounds to wait for (`stopping`), the stop is the band alone, as it was before the stop waited for any.
#[test]
fn the_stop_on_growth_ends_the_rounds_from_the_end_of_the_least_levels_rounds_on() {
    let opts = options(4, false);
    let (one, three) = (lines(FIRST, &opts, &from_table(1), None), lines(FIRST, &opts, &from_table(3), None));
    let rounds = |line: &Line| (line.rounds, line.work, line.score, raised_by(line));
    // Every gap is within the largest T of the gap below: from level 1's budget on, each game stops at once, with
    // level 1's rounds.
    let stopped = lines(FIRST, &opts, &stopping(3, Some(u64::MAX)), None);
    for ((one, three), stopped) in one.iter().zip(&three).zip(&stopped) {
        let n = one.n;
        assert!(three.rounds > one.rounds, "n={n}: level 3 plays more rounds than level 1");
        assert_eq!(rounds(stopped), rounds(one), "n={n}");
        assert!(stopped.growth_stop && !one.growth_stop && !three.growth_stop, "n={n}");
        assert!(stopped.work >= stopped.plan.stop_floor() && stopped.work < stopped.plan.budget(), "n={n}");
        assert!(stopped.summary_line().contains(", stopped"), "{}", stopped.summary_line());
    }
    // Level 1 stops at its budget whatever the growth: its lines are those without the stop.
    let at_one = lines(FIRST, &opts, &stopping(1, Some(u64::MAX)), None);
    assert_eq!(with_plan(&at_one, from_table(1)), with_plan(&one, from_table(1)));
    // With a smaller T, a game stops at the end of level 1's rounds if its gap is within T of the gap below there, else
    // at the first round after them that brings it within T, else never, and its line is then level 3's.
    let mut ways = [0; 3];
    for most in [0, 10, 25] {
        let lines = lines(FIRST, &opts, &stopping(3, Some(most)), None);
        for ((one, three), line) in one.iter().zip(&three).zip(&lines) {
            let (n, last) = (line.n, three.growth.expect("moves below"));
            let within = |growth: i64| growth.unsigned_abs() <= most;
            let later = three.improvements.iter().filter(|raised| raised.round > one.rounds);
            let at = later.clone().find(|raised| within(last + (three.score - raised.score) as i64));
            if within(one.growth.expect("moves below")) {
                assert_eq!(rounds(line), rounds(one), "n={n}, T={most}");
                ways[0] += 1;
            } else if let Some(at) = at {
                assert_eq!((line.rounds, line.score, line.growth_stop), (at.round, at.score, true), "n={n}, T={most}");
                ways[1] += 1;
            } else {
                assert_eq!(
                    with_plan(slice::from_ref(line), from_table(3)),
                    with_plan(slice::from_ref(three), from_table(3))
                );
                ways[2] += 1;
            }
        }
    }
    // At the end of level 1's rounds the gap of 375 is 21 smaller than that of the table's 374, so it goes on with a T
    // of 0 or 10, never to come within it, and stops with 25; the gap of 376 is the gap below, and it stops with any T.
    assert_eq!(one.iter().map(|line| line.growth).collect::<Vec<_>>(), [Some(-21), Some(0)]);
    assert_eq!(ways, [4, 0, 2]);
    // Each line records the stop's T.
    assert_eq!(stopped[0].to_json().get("stop_growth"), Some(&Json::from(u64::MAX)));
    assert_eq!(three[0].to_json().get("stop_growth"), Some(&Json::Null));
    assert_eq!(three[0].to_json().get("stop_quiet"), Some(&Json::from(4u64)), "the default");
    assert_eq!(stopped[0].to_json().get("stop_quiet"), Some(&Json::from(0u64)));
}

/// Where the stop of [`waiting`] with `quiet` rounds to wait for ends the rounds of a game, from `one` and `three`, its
/// lines with no stop at level 1 and at a higher level: the first round from the end of level 1's rounds on, and before
/// the last, that follows `quiet` rounds in a row that raised nothing (the start being round 0).  That round, the score
/// after it, and the rounds since the score last rose; `None` if there is none.
fn quiet_stop(one: &Line, three: &Line, quiet: u64) -> Option<(u64, u64, u64)> {
    let last = |round: u64| three.improvements.iter().rev().find(|raised| raised.round <= round);
    let since = |round: u64| round - last(round).map_or(0, |raised| raised.round);
    let score = |round: u64| last(round).map_or_else(|| three.start.expect("a searched game"), |raised| raised.score);
    (one.rounds..three.rounds).find(|&round| since(round) >= quiet).map(|round| (round, score(round), since(round)))
}

#[test]
fn the_stop_on_growth_waits_for_k_rounds_that_raised_nothing() {
    let opts = options(4, false);
    let (one, three) = (lines(FIRST, &opts, &from_table(1), None), lines(FIRST, &opts, &from_table(3), None));
    // At level 3, 375 raises the score in rounds 2 and 3 and 376 in rounds 1 and 2, and the quiet rounds follow.
    let gains: Vec<Vec<u64>> =
        three.iter().map(|line| raised(line).iter().map(|&(round, _)| round).collect()).collect();
    assert_eq!(gains, [[2, 3], [1, 2]]);
    // The round at which each game's rounds end, for K from 0 to 3 (none: the game plays its level's rounds).
    let (mut ends, mut waited) = (Vec::new(), Vec::new());
    for quiet in 0..=3 {
        let lines = lines(FIRST, &opts, &waiting(3, quiet), None);
        let mut end = Vec::new();
        for ((one, three), line) in one.iter().zip(&three).zip(&lines) {
            let (n, json, stop) = (line.n, line.to_json(), quiet_stop(one, three, quiet));
            assert_eq!(json.get("stop_quiet"), Some(&Json::from(quiet)), "n={n}, K={quiet}");
            if let Some((round, score, since)) = stop {
                // The rounds end at the round the line with no stop gives, with the quiet rounds it had.
                let ended = (line.rounds, line.score, line.growth_stop, line.quiet);
                assert_eq!(ended, (round, score, true, since), "n={n}, K={quiet}");
                assert!(since >= quiet && line.work < line.plan.budget(), "n={n}, K={quiet}");
                assert_eq!(json.get("quiet_rounds"), Some(&Json::from(since)), "n={n}, K={quiet}");
                assert!(line.summary_line().contains(", stopped"), "{}", line.summary_line());
            } else {
                // Never quiet enough before the last round: the line of level 3 with no stop.
                let (line, three) = (slice::from_ref(line), slice::from_ref(three));
                assert_eq!(with_plan(line, from_table(3)), with_plan(three, from_table(3)), "n={n}, K={quiet}");
                assert_eq!(json.get("quiet_rounds"), Some(&Json::Null), "n={n}, K={quiet}");
            }
            end.push(stop.map(|(round, ..)| round));
        }
        ends.push(end);
        waited.push(lines);
    }
    // K = 0 asks the gap alone, so the rounds end with level 1's, round 2 in both games.  Each game then waits for K
    // rounds after its last gain, in round 3 and round 2.  Level 3 has 5 rounds, and a game that would wait for its
    // last round plays them all.
    assert_eq!(ends, [[Some(2), Some(2)], [Some(4), Some(3)], [None, Some(4)], [None, None]]);
    // The count comes from the rounds in their order, so the lines are the same on one thread.
    let on_one = lines(FIRST, &options(1, false), &waiting(3, 1), None);
    assert_eq!(timeless_all(&on_one), timeless_all(&waited[1]));
}

/// The plan of a run at level 1 that repairs and plays twice a game whose growth is above `push_growth`.
fn pushing(push_growth: Option<i64>) -> Plan {
    Plan { push_growth, ..twice(0) }
}

#[test]
fn the_push_plays_a_game_twice_where_its_gap_grows_by_more_than_t() {
    let opts = options(4, false);
    // Every growth is above the least, and a game with moves below it is played twice, as by `--unseeded-every 1`.
    let first = (WALKED.0, WALKED.0 + 3);
    let every = lines(first, &opts, &twice(1), None);
    let all = lines(first, &opts, &pushing(Some(i64::MIN)), None);
    assert!(all.iter().all(|line| line.pushed && line.other.is_some()));
    let unpushed = |lines: &[Line]| lines.iter().map(|line| Line { pushed: false, ..line.clone() }).collect::<Vec<_>>();
    assert_eq!(with_plan(&unpushed(&all), twice(1)), with_plan(&every, twice(1)));
    // No growth is above the largest: the lines without the push.
    let once = lines(WALKED, &opts, &twice(0), None);
    assert_eq!(with_plan(&lines(WALKED, &opts, &pushing(Some(i64::MAX)), None), twice(0)), with_plan(&once, twice(0)));
    // Played once, 390 falls 7 short of the table's score, and its gap grows by 24: the 17 of the table's 390 and the
    // 7.  The games above carry the gap on, with a growth of 0, and 396 reaches the table's score, 7 below that gap.
    let table = Optimal::built_in();
    let proven_gap = |n: u32| PARITY[(n - 373) as usize] - table.score(n).unwrap();
    assert_eq!(proven_gap(390) as i64 - proven_gap(389) as i64, 17);
    let growths: Vec<Option<i64>> = once.iter().map(|line| line.growth).collect();
    assert_eq!(growths, [24, 0, 0, 0, 0, 0, -7].map(Some));
    // So a T of 20 plays 390 twice, and no other game.
    let pushed = lines(WALKED, &opts, &pushing(Some(20)), None);
    let twice: Vec<u32> = pushed.iter().filter(|line| line.other.is_some()).map(|line| line.n).collect();
    assert_eq!(twice, [390]);
    assert!(pushed[0].pushed && pushed[0].summary_line().contains("  pushed, unseeded play"));
    assert_eq!(pushed[0].to_json().get("push_growth"), Some(&Json::from(20i64)));
}

#[test]
fn the_unseeded_play_of_a_game_played_twice_is_at_the_level_of_unseeded_effort() {
    let opts = options(4, false);
    let at_one = Plan { unseeded_every: 1, unseeded_effort: Some(1), ..from_table(3) };
    let lines = lines(FIRST, &opts, &at_one, None);
    let alone = self::lines(FIRST, &opts, &Plan { unseeded: true, ..from_table(1) }, None);
    let seeded = self::lines(FIRST, &opts, &from_table(3), None);
    for ((line, alone), seeded) in lines.iter().zip(&alone).zip(&seeded) {
        let other = line.other.expect("every game is played twice");
        let plays = [(line.unseeded, line.score, line.work), (other.unseeded, other.score, other.work)];
        let (unseeded, played) = if line.unseeded { (plays[0], plays[1]) } else { (plays[1], plays[0]) };
        assert_eq!(unseeded, (true, alone.score, alone.work), "n={}: the unseeded play is at level 1", line.n);
        assert_eq!(played, (false, seeded.score, seeded.work), "n={}: the seeded play is at level 3", line.n);
        assert_eq!(line.plan.effort, if line.unseeded { 1 } else { 3 }, "n={}: the line's level is its play's", line.n);
    }
}

/// A prime played twice whose line is the prime formula's sets no search up, so no game repeats its board's problem:
/// every game of 37 to 40 played twice, 37 a prime whose unseeded play scores no more than the formula, and whose
/// board poses the problem of 38's (a build that remembered the problem of every prime's board for the repeats failed
/// at 38, finding no promotions to repeat).
#[test]
fn a_repeat_plays_the_promotions_of_a_line_that_searched_its_problem() {
    let lines = lines((37, 40), &options(4, true), &Plan { unseeded_every: 1, ..plan(1) }, None);
    let prime = &lines[0];
    assert_eq!(
        (prime.n, prime.method, prime.setup, prime.other.map(|o| o.unseeded)),
        (37, Method::Prime, None, Some(true))
    );
    for line in lines.iter().filter(|line| line.method == Method::Repeat) {
        let m = line.repeats.expect("the game it repeats");
        let repeated = lines.iter().find(|other| other.n == m).expect("a game of the run");
        assert!(repeated.setup.is_some(), "n={}: repeats {m}, whose line set no search up", line.n);
    }
}

#[test]
fn the_lines_of_a_run_that_stops_and_pushes_are_the_same_on_one_thread_and_on_four() {
    let dir = TempDir::new("unsound-growth");
    let plan = Plan { stop_growth: Some(0), stop_quiet: 1, push_growth: Some(0), ..repairing(2) };
    let (one, four) = (dir.join("one.jsonl"), dir.join("four.jsonl"));
    played(WALKED, &options(1, false), &plan, Some(&one));
    played(WALKED, &options(4, false), &plan, Some(&four));
    assert_eq!(file_lines(&one, false), file_lines(&four, false));
}
