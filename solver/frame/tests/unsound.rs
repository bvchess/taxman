//! The unsound mode (`taxman::solver::unsound`): its lines replay to their scores and never beat the built-in table,
//! are the same on one thread and on four, run the rounds of a lower level first, and resume; and the wall between its
//! lines and proven ones holds both ways.  The rounds follow trajectories from the games before a game, tilt a share of
//! their neighborhoods and widen them after a stall, as the mode builds in.  The repair pulls a short line up from the
//! lines above it, and a game whose gap grew is played twice and keeps its better play.  A line's bound is the parity
//! bound, and the stop on growth ends a level's rounds from the end of level 1's on, after the quiet rounds it waits
//! for.  The best-known file takes a line that scores more than its game's, seeds the games of the runs that share it,
//! takes the repair down into its lines of the games below a run's first, and is offered the lines of a run's file when
//! the run opens it.  With `known_to` the built-in table is cut, and the games above the cut are unproven: the
//! best-known file takes and seeds from their lines, and refuses a run that knows them.
//!
//! A level of effort budgets 2^L × `round_work` work of rounds.  The tests lower `round_work` from the 65,536 of a
//! round of full neighborhoods to about half the work of a round of the small games they play, so that a level is a
//! few rounds and a test stays inside its CPU limit.  What the command line does is in `cli.rs`.

mod common;

use common::TempDir;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::slice;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use taxman::game::{replay, Optimal};
use taxman::json::Json;
use taxman::results::{verify_files, ResultsFile};
use taxman::search::finder::{NEIGHBORHOODS, NEIGHBORHOOD_WORK};
use taxman::solver::unsound::{
    self, BestKnownFile, Files, Line, Origin, OtherPlay, Plan, Previous, Raised, Repair, Source, BACK, PUSH_GROWTH,
    STOP_QUIET,
};
use taxman::solver::{solve_range, Method, RangeSummary, SolveOptions};

/// The work of a round in these tests, which level L budgets 2^L times: about half the work of a round of the small
/// games (their steps are about 80 a round, and the neighborhoods' work is 64 × 32 = 2,048), so that level 1 runs two
/// rounds and level L a few rounds more than level L − 1.
const WORK: u64 = 1_100;

/// The seed that makes the games [`WALKED`], played with no seed from the games below, fall short: with the seed 3, the
/// rounds of level 1 leave 390 7 short of the table's score, and its line is handed on to the games above it.
const SHORT_SEED: u64 = 3;

/// The plan of a run at level `effort` with the default seed and no stop on growth: the stop has tests of its own.
fn plan(effort: u32) -> Plan {
    Plan { effort, round_work: WORK, stop_growth: None, ..Plan::default() }
}

/// The plan of [`plan`] with the seed [`SHORT_SEED`], for the games that fall short.
fn walking(effort: u32) -> Plan {
    Plan { seed: SHORT_SEED, ..plan(effort) }
}

/// The options of a run on `threads` threads, with or without the tactic `seed_incumbent`.  Without it, a game's start
/// is made without the moves for n − 1, and the start of each of the games [`FIRST`] and [`SECOND`] falls short.
fn options(threads: usize, seed_incumbent: bool) -> SolveOptions {
    let mut opts = SolveOptions { threads, ..SolveOptions::default() };
    opts.tactics.seed_incumbent = seed_incumbent;
    opts
}

/// The files of a run with the built-in table, the file `out` if there is one, and no best-known file.
fn built_in(out: Option<&Path>) -> Files<'_> {
    Files::with_built_in(out, None)
}

/// Games whose rounds raise the score of a start made without the moves for n − 1: 375 and 376 in their rounds 1 and
/// 2, each to the table's score.
const FIRST: (u32, u32) = (375, 376);

/// Games 384 to 387, with no seed: three searches, whose first rounds raise their starts (385 and 387 to the table's
/// score, which is their ceiling), and 386, a repeat of 385.
const SECOND: (u32, u32) = (384, 387);

/// The lines of the games `range` played with `plan` and `opts`, appended to the file `out` if there is one, and the
/// repairs of the run; each line and each repair replays to its score, which is no more than the table's.
fn played(range: (u32, u32), opts: &SolveOptions, plan: &Plan, out: Option<&Path>) -> (Vec<Line>, Vec<Repair>) {
    let (lines, repairs, _) = played_in(range, opts, plan, &built_in(out));
    (lines, repairs)
}

/// The lines and the repairs of the games `range` played with `plan` and `opts` in `files`, checked as [`played`]
/// checks them, and the summary of the run.
fn played_in(
    (lo, hi): (u32, u32),
    opts: &SolveOptions,
    plan: &Plan,
    files: &Files,
) -> (Vec<Line>, Vec<Repair>, RangeSummary) {
    let optimal = Optimal::built_in();
    let (mut lines, mut repairs) = (Vec::new(), Vec::new());
    let summary = unsound::solve_range(
        lo,
        hi,
        opts,
        plan,
        files,
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
    (lines, repairs, summary)
}

/// The lines of the games `range` played with `plan` and `opts`, appended to the file `out` if there is one
/// ([`played`]).
fn lines(range: (u32, u32), opts: &SolveOptions, plan: &Plan, out: Option<&Path>) -> Vec<Line> {
    played(range, opts, plan, out).0
}

/// The line of each of `games` played alone, so that it starts from the table's moves for the games below it.
fn from_the_table(games: &[u32], opts: &SolveOptions, plan: &Plan) -> Vec<Line> {
    games.iter().flat_map(|&n| lines((n, n), opts, plan, None)).collect()
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

/// `line` ([`timeless`]) with the source of the moves for n − 1 that it started from named as this run's, as a run in
/// one go would name the file's line of the game before a break.
fn from_run(line: &Line) -> Line {
    let source = |source| if source == Source::Results { Source::Run } else { source };
    let prev = line.prev.clone().map(|prev| Previous { source: source(prev.source), ..prev });
    Line { prev, ..timeless(line) }
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
    let lines = lines((1, 100), &options(2, true), &plan(1), None);
    assert_eq!(lines.len(), 100);
    for (i, line) in lines.iter().enumerate() {
        let n = line.n;
        let json = line.to_json();
        assert_eq!(json.get("unsound"), Some(&Json::Bool(true)), "n={n}");
        assert!(json.get("completed").is_none() && json.get("proof").is_none(), "n={n}: it claims nothing");
        assert_eq!(json.get("budget"), Some(&Json::from(plan(1).budget())), "n={n}");
        // No play scores above the game's bound, which a board with no promotion candidates is at.
        assert!(line.score <= line.bound, "n={n}: {} is above the bound {}", line.score, line.bound);
        assert!(line.method != Method::Trivial || Some(line.bound) == line.in_the_bag, "n={n}");
        assert_eq!(json.get("bound"), Some(&Json::from(line.bound)), "n={n}");
        if n > 1 {
            // The line of an unseeded play takes nothing from the games below.
            if !line.unseeded {
                let prev = line.prev.as_ref().expect("the moves for n - 1");
                assert_eq!((prev.n, prev.source, prev.short_by), (n - 1, Source::Run, 0), "n={n}");
            }
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
    assert!(
        searched.clone().all(|line| line.rounds >= 1 || line.at_ceiling),
        "it plays a round, or starts at its ceiling"
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
    // 375 and 376 are raised in their rounds 1 and 2 at level 1, to the table's score, and gain nothing after.
    assert_eq!((raised(&less[0]).len(), raised(&more[0]).len()), (2, 2));
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

/// Games whose rounds, from a start made with the table's moves for n − 1, follow trajectories from the games before
/// them, and each starts from the table's moves, so it is the same game whatever the lines before it.  The plan plays
/// each once but 539 at level 1: the gap of its seeded play grows by more than the push growth over the table's, and
/// the push plays it twice.
const SHAPED: [u32; 4] = [375, 539, 544, 680];

/// The games of [`SHAPED`] that the check of a lower level first plays, those that the push plays once at levels 1 to
/// 3.  The line of a game played twice is the better play's, which can be the other play at the next level, so a level
/// runs the rounds of the levels below first in each play, not in the line.
const PLAYED_ONCE: [u32; 2] = [375, 680];

/// The games of [`SHAPED`] but 544, which keeps the check of the threads inside its CPU limit.
const ON_THREADS: [u32; 3] = [375, 539, 680];

#[test]
fn the_rounds_of_the_built_in_settings_are_these() {
    // The lines at level 3, to see that no change to a neighborhood's draws, to a trajectory or to the budget goes
    // unseen: the neighborhoods of the finder's default are pinned beside, in `search::finder`.
    let lines = from_the_table(&SHAPED, &options(4, true), &plan(3));
    let now: Vec<_> = lines.iter().map(|line| (line.n, line.score, line.rounds, line.steps, raised(line))).collect();
    let was = [
        (375, 44328, 5, 549, vec![(1, 44328)]),
        (539, 91703, 4, 676, vec![(1, 91683), (2, 91703)]),
        (544, 93256, 4, 1142, vec![(1, 93244), (2, 93256)]),
        (680, 146826, 5, 416, vec![(1, 146826)]),
    ];
    assert_eq!(now, was);
    for line in &lines {
        assert_eq!(line.plan, plan(3), "n={}", line.n);
        assert_eq!(line.trajectories.as_ref().map(|t| t[0]), Some(Origin::Start), "n={}", line.n);
    }
}

#[test]
fn the_lines_of_the_built_in_settings_are_the_same_on_one_thread_and_on_four() {
    let one = from_the_table(&ON_THREADS, &options(1, true), &plan(3));
    let four = from_the_table(&ON_THREADS, &options(4, true), &plan(3));
    assert_eq!(timeless_all(&one), timeless_all(&four));
    assert!(one.iter().all(|line| line.threads == 1) && four.iter().all(|line| line.threads == 4));
}

#[test]
fn the_improvements_of_a_level_are_a_prefix_of_those_of_the_level_above_it() {
    let by_level: Vec<Vec<Line>> =
        (1..=3).map(|effort| from_the_table(&PLAYED_ONCE, &options(4, true), &plan(effort))).collect();
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

#[test]
fn the_line_records_the_trajectories_the_stages_and_the_work_of_the_rounds() {
    let lines = from_the_table(&SHAPED, &options(4, true), &plan(3));
    for line in &lines {
        let (n, json) = (line.n, line.to_json());
        assert_eq!(json.get("budget"), Some(&Json::from(plan(3).budget())), "n={n}");
        let (origins, stages) = (line.trajectories.as_ref().unwrap(), line.stage_rounds.as_ref().unwrap());
        assert_eq!(origins[0], Origin::Start, "n={n}");
        assert_eq!(stages.len(), origins.len(), "n={n}: the rounds of each trajectory");
        assert!(stages.iter().all(|rounds| rounds.iter().sum::<u64>() == line.rounds), "n={n}: {stages:?}");
        assert!(origins.len() <= 1 + BACK as usize, "n={n}: {origins:?}");
        let steps: Vec<u64> = line.improvements.iter().map(|raised| raised.steps).collect();
        assert!(steps.windows(2).all(|pair| pair[0] < pair[1]) && steps.iter().all(|&s| s <= line.steps), "n={n}");
        for raised in &line.improvements {
            assert!(raised.trajectory < origins.len() && raised.stage < 3, "n={n}: {raised:?}");
        }
    }
    let trajectories: Vec<usize> = lines.iter().map(|line| line.trajectories.as_ref().unwrap().len()).collect();
    assert_eq!(trajectories, [2, 3, 4, 4]);
}

#[test]
fn a_game_begins_a_trajectory_from_each_distinct_set_that_survives_of_the_games_before() {
    let origins: Vec<Vec<Origin>> = from_the_table(&SHAPED, &options(4, true), &plan(3))
        .into_iter()
        .map(|line| line.trajectories.unwrap())
        .collect();
    let game = Origin::Game;
    // What survives of the moves for 374 is the start of 375, so 375 begins no trajectory from it, and the moves for
    // 367 are the first to give another.
    assert_eq!(origins[0], [Origin::Start, game(367)]);
    assert_eq!(origins[1], [Origin::Start, game(538), game(531)]);
    assert_eq!(origins[2], [Origin::Start, game(543), game(539), game(538)]);
    assert_eq!(origins[3], [Origin::Start, game(679), game(675), game(674)]);
    // The games are in the order n − 1, n − 2, ..., none twice, and within BACK games of the game.
    for (shaped, origins) in SHAPED.iter().zip(&origins) {
        let games: Vec<u32> =
            origins[1..].iter().map(|origin| if let Origin::Game(m) = origin { *m } else { 0 }).collect();
        assert!(
            games.windows(2).all(|pair| pair[0] > pair[1])
                && games.iter().all(|m| (shaped - BACK..*shaped).contains(m))
        );
    }
}

/// Games whose start, from the table's moves for n − 1, is lighter than what survives of the moves for an earlier game:
/// their start, and the score of the heaviest start.
const SURVIVORS_AHEAD: [(u32, u64, u64); 2] = [(715, 161442, 161449), (805, 204901, 204916)];

#[test]
fn a_trajectory_that_begins_heavier_than_the_start_raises_the_score_before_any_round() {
    let opts = options(4, true);
    for (n, start, score) in SURVIVORS_AHEAD {
        let line = &lines((n, n), &opts, &plan(1), None)[0];
        assert_eq!((line.start, line.score), (Some(start), score), "n={n}");
        // The game's score is that of the trajectory, from the first: an improvement of round 0.
        let raised = &line.improvements[0];
        let origins = line.trajectories.as_ref().unwrap();
        assert!((1..origins.len()).contains(&raised.trajectory), "n={n}: {:?} of {origins:?}", raised.trajectory);
        assert_eq!((raised.round, raised.stage, raised.steps, raised.work, raised.score), (0, 0, 0, 0, score), "n={n}");
        // A level above does the same first, and its rounds go on from there.
        let more = &lines((n, n), &opts, &plan(2), None)[0];
        assert_eq!(raised_by(more).first(), Some(&(0, raised.trajectory, 0, 0, 0, score)), "n={n}");
        assert!(more.rounds >= 1 && more.score >= score, "n={n}");
    }
}

#[test]
fn a_game_has_no_trajectory_from_the_moves_nothing_has() {
    // Game n − j below 1 has no moves, and a game that takes no seed begins no trajectory from any game.
    let seeded = lines((1, 40), &options(2, true), &plan(1), None);
    let searched: Vec<&Line> = seeded.iter().filter(|line| line.trajectories.is_some()).collect();
    assert!(!searched.is_empty());
    for line in searched {
        let origins = line.trajectories.as_ref().unwrap();
        let games: Vec<u32> =
            origins.iter().filter_map(|o| if let Origin::Game(m) = o { Some(*m) } else { None }).collect();
        assert!(games.iter().all(|&m| m >= 1 && m < line.n && line.n - m <= BACK), "n={}: {games:?}", line.n);
    }
    let unseeded = lines(SECOND, &options(4, false), &plan(1), None);
    assert!(unseeded
        .iter()
        .filter(|line| line.start.is_some())
        .all(|line| line.trajectories == Some(vec![Origin::Start])));
}

#[test]
fn a_plan_with_an_option_out_of_range_is_refused() {
    let refused = |plan: Plan| {
        unsound::solve_range(12, 12, &options(1, true), &plan, &built_in(None), |_| (), |_| ()).unwrap_err()
    };
    // The levels are 1 to 6: the start alone is no level, and nothing above the most is.
    for effort in [0, unsound::MAX_EFFORT + 1] {
        let err = refused(Plan { effort, ..plan(1) });
        assert_eq!(err, format!("--effort takes L, the level of effort, a whole number from 1 to 6: got \"{effort}\""));
    }
    for above in [9, 100] {
        assert!(refused(Plan { above, ..plan(1) }).starts_with("--above takes J, a whole number from 0 to 8"));
    }
    assert!(refused(Plan { round_work: 0, ..plan(1) }).contains("is 0"));
    // The least and the most effort are accepted: the most budgets 2^22 work.
    let (least, most) = (Plan { effort: 1, ..plan(1) }, Plan { effort: 6, round_work: unsound::ROUND_WORK, ..plan(1) });
    assert_eq!((unsound::check_plan(&least), unsound::check_plan(&most)), (Ok(()), Ok(())));
    assert_eq!((least.budget(), most.budget()), (2 * WORK, 1 << 22));
}

#[test]
fn a_run_resumes_from_its_file() {
    let dir = TempDir::new("unsound-resume");
    // The games after the break begin trajectories from the moves for the games before: this run's, the file's, the
    // table's.
    let (opts, plan, (lo, hi)) = (options(4, true), plan(1), SECOND);
    let whole = lines(SECOND, &opts, &plan, Some(&dir.join("whole.jsonl")));
    let file = dir.join("parts.jsonl");
    let first = lines((lo, lo + 1), &opts, &plan, Some(&file));
    let rest = lines((lo, hi), &opts, &plan, Some(&file));
    assert_eq!((first.len(), rest.len()), (2, whole.len() - 2));
    // The game after the break starts from the file's line of the game before it: the run's line in one go.
    assert_eq!(rest[0].prev.as_ref().map(|prev| prev.source), Some(Source::Results));
    let parts: Vec<Line> = first.iter().chain(&rest).map(from_run).collect();
    assert_eq!(parts, whole.iter().map(timeless).collect::<Vec<_>>());
    let count = |name: &str| fs::read_to_string(dir.join(name)).unwrap().lines().count();
    assert_eq!(count("parts.jsonl"), count("whole.jsonl"));
}

/// A run makes the setup of each game but its first while the game before plays, and that is the setup the game makes
/// on its own: with the lines of the games before it in a file, each searched game of [`SECOND`] has the line it has in
/// the range, but for where the moves it started from came from, and 386 repeats the problem that 385's setup posed.
#[test]
fn a_setup_made_while_the_game_before_plays_is_the_one_the_game_makes() {
    let (dir, opts, plan) = (TempDir::new("unsound-setup"), options(4, true), plan(1));
    let range = lines(SECOND, &opts, &plan, None);
    let searched: Vec<u32> = range.iter().filter(|line| line.method == Method::Search).map(|line| line.n).collect();
    assert_eq!(searched, [384, 385, 387]);
    // The first game of a run has no setup made ahead.
    for (i, line) in range.iter().enumerate().filter(|(i, line)| *i > 0 && line.method == Method::Search) {
        // The file has the lines of the games before this one, so the game starts from the same moves.
        let file = dir.join(format!("before-{}.jsonl", line.n));
        let before = lines((SECOND.0, line.n - 1), &opts, &plan, Some(&file));
        assert_eq!(before.len(), i, "n={}", line.n);
        let alone = lines((line.n, line.n), &opts, &plan, Some(&file)).remove(0);
        assert_eq!(from_run(line), from_run(&alone), "n={}", line.n);
    }
    assert_eq!((range[2].method, range[2].repeats), (Method::Repeat, Some(385)));
}

/// Games 390 to 396 at level 1, with no seed and the seed [`SHORT_SEED`]: the search of 390 falls 7 short of the
/// table's score, 391 to 395 keep the shortfall, as repeats and as searches stopped at the lower ceiling it gives, and
/// 396 reaches the table's score.
const WALKED: (u32, u32) = (390, 396);

/// The proven ceiling of game `n`: n + the table's score for n − 1.
fn proven_ceiling(n: u32) -> u64 {
    Optimal::built_in().score(n - 1).unwrap() + u64::from(n)
}

/// Games whose start from the table's moves for n − 1 is at the run's ceiling: the proven mode finds each with no
/// search (method `start`, proof `cap`).
const AT_CEILING_AT_THE_START: [u32; 4] = [18, 28, 33, 42];

/// Games whose start, made without the moves for n − 1 (`options(_, false)`), falls short of the run's ceiling, and the
/// round in which the rounds reach it: 378 in its second and 385 and 387 in their first.
const AT_CEILING_IN_A_ROUND: [(u32, u64); 3] = [(378, 2), (385, 1), (387, 1)];

#[test]
fn a_game_whose_start_reaches_the_runs_ceiling_stops_before_any_round() {
    let opts = options(4, true);
    for n in AT_CEILING_AT_THE_START {
        let ceiling = proven_ceiling(n);
        // The start is at the ceiling, whatever the level: no round and no work.
        for effort in [1, 3] {
            let line = &lines((n, n), &opts, &plan(effort), None)[0];
            assert_eq!((line.method, line.start, line.score), (Method::Search, Some(ceiling), ceiling), "n={n}");
            assert_eq!((line.ceiling, line.at_ceiling), (Some(ceiling), true), "n={n} at level {effort}");
            assert_eq!((line.rounds, line.steps, line.work), (0, 0, 0), "n={n} at level {effort}");
            assert!(line.improvements.is_empty(), "n={n}");
            let json = line.to_json();
            assert_eq!(json.get("ceiling"), Some(&Json::from(ceiling)), "n={n}");
            assert_eq!(json.get("at_ceiling"), Some(&Json::Bool(true)), "n={n}");
        }
    }
}

#[test]
fn a_game_that_reaches_the_runs_ceiling_in_a_round_stops_after_that_round() {
    let opts = options(4, false);
    for (n, round) in AT_CEILING_IN_A_ROUND {
        let ceiling = proven_ceiling(n);
        let stopped = &lines((n, n), &opts, &plan(3), None)[0];
        assert_eq!((stopped.ceiling, stopped.at_ceiling, stopped.score), (Some(ceiling), true, ceiling), "n={n}");
        assert!(stopped.start.unwrap() < ceiling, "n={n}: the start falls short of it");
        // The round that reaches it is the last, well before the budget.
        assert_eq!((stopped.rounds, raised(stopped).last()), (round, Some(&(round, ceiling))), "n={n}");
        assert!(stopped.work < plan(3).budget(), "n={n}: {} work", stopped.work);
        // A higher level of effort stops at the same round, and a lower one is at the ceiling only if its budget lets it
        // do those rounds.
        for effort in [2, 4] {
            let more = &lines((n, n), &opts, &plan(effort), None)[0];
            let shown =
                |line: &Line| (line.rounds, line.steps, line.work, line.score, raised_by(line), line.at_ceiling);
            assert_eq!(shown(more), shown(stopped), "n={n} at level {effort}");
        }
        let less = &lines((n, n), &opts, &plan(1), None)[0];
        assert!(less.rounds <= stopped.rounds, "n={n}");
        assert_eq!(less.at_ceiling, less.rounds == stopped.rounds, "n={n}");
    }
}

/// The line of game 395 as a run of level 1 plays it from game 390 on, without the moves for n − 1 ([`WALKED`] has the
/// run: 390 falls 7 short of the table's score and the games after it keep the shortfall), and the file `name` of `dir`
/// that holds that line alone, so that no line of 390 to 394 repairs with it.
fn short_395_in(dir: &TempDir, name: &str, threads: usize) -> (Line, PathBuf) {
    let walked = dir.join(format!("walked-{name}"));
    let short = lines((WALKED.0, 395), &options(threads, false), &walking(1), Some(&walked)).pop().expect("a line");
    let text = fs::read_to_string(&walked).unwrap();
    let file = dir.join(name);
    fs::write(&file, format!("{}\n", text.lines().last().expect("the line of 395"))).unwrap();
    (short, file)
}

/// The lines of game 395 and then of game 396, played at level `effort` from a file that holds a short line of 395, on
/// `threads` threads: 395, played from game 390 on without the moves for n − 1 ([`short_395_in`]), falls 7 short of the
/// table's score; 396 then reads the file's line, not the table's, and stops at the lower ceiling that the line gives.
fn from_a_short_line(threads: usize, effort: u32) -> (Line, Line) {
    let (dir, opts) = (TempDir::new("unsound-short"), options(threads, false));
    let (short, file) = short_395_in(&dir, "short.jsonl", threads);
    let game = lines((396, 396), &opts, &walking(effort), Some(&file)).remove(0);
    (short, game)
}

#[test]
fn a_game_seeded_with_a_short_line_for_n_minus_1_stops_at_the_lower_ceiling_of_those_moves() {
    let (short, stopped) = from_a_short_line(4, 3);
    let table = Optimal::built_in();
    assert_eq!(short.score + 7, table.score(395).unwrap(), "395's line is 7 short of optimal");
    let prev = stopped.prev.as_ref().unwrap();
    assert_eq!((prev.n, prev.score, prev.source), (395, short.score, Source::Results));
    // The ceiling is that of the moves in the file, below the proven one, and the game stops at it.
    let ceiling = 396 + short.score;
    assert_eq!((stopped.ceiling, stopped.at_ceiling), (Some(ceiling), true));
    assert!(ceiling < proven_ceiling(396));
    assert!(stopped.start.unwrap() < ceiling, "the start falls short of it");
    assert_eq!(raised(&stopped).last().map(|&(round, _)| round), Some(stopped.rounds), "the last round reaches it");
    assert!(stopped.work < walking(3).budget(), "{} work: it stopped before its budget", stopped.work);
    // That round passes the ceiling, which proves the line of 395 short by as much at least.
    assert!(stopped.score > ceiling);
    assert_eq!(prev.short_by, stopped.score - ceiling);
    assert!(stopped.score <= table.score(396).unwrap());
}

#[test]
fn the_lines_that_stop_at_the_ceiling_are_the_same_on_one_thread_and_on_four() {
    let played = |threads| {
        let mut played = Vec::new();
        for n in AT_CEILING_AT_THE_START {
            played.extend(lines((n, n), &options(threads, true), &plan(3), None));
        }
        for (n, _) in AT_CEILING_IN_A_ROUND {
            played.extend(lines((n, n), &options(threads, false), &plan(3), None));
        }
        played.push(from_a_short_line(threads, 3).1);
        played
    };
    let (one, four) = (played(1), played(4));
    assert_eq!(timeless_all(&one), timeless_all(&four));
    assert!(one.iter().all(|line| line.at_ceiling));
    assert!(one.iter().all(|line| line.threads == 1) && four.iter().all(|line| line.threads == 4));
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
    let (opts, plan) = (options(1, true), plan(1));
    let err = unsound::solve_range(13, 13, &opts, &plan, &built_in(Some(&proven_file)), |_| (), |_| ()).unwrap_err();
    assert!(err.contains("line 1: not an unsound line"), "{err}");
    fs::write(&mixed, format!("{unsound}{}\n", proven.lines().next().unwrap())).unwrap();
    let err = unsound::solve_range(13, 13, &opts, &plan, &built_in(Some(&mixed)), |_| (), |_| ()).unwrap_err();
    assert!(err.contains("line 13: not an unsound line"), "{err}");
}

/// `json` without what varies from run to run: the times (the game's, each improvement's and a repair's), the threads
/// and the build.
fn timeless_json(json: &Json) -> Json {
    const VARIES: [&str; 4] = ["seconds", "start_seconds", "threads", "build"];
    match json {
        Json::Obj(pairs) => Json::Obj(
            pairs
                .iter()
                .filter(|(key, _)| !VARIES.contains(&key.as_str()))
                .map(|(key, value)| (key.clone(), timeless_json(value)))
                .collect(),
        ),
        Json::Arr(items) => Json::Arr(items.iter().map(timeless_json).collect()),
        other => other.clone(),
    }
}

/// The lines of the file `path`, in order, without what varies from run to run ([`timeless_json`]).
fn file_lines(path: &Path) -> Vec<Json> {
    let text = fs::read_to_string(path).unwrap();
    text.lines().map(|line| timeless_json(&Json::parse(line).unwrap())).collect()
}

/// The last line of each game of the file `path` ([`file_lines`]), the game's.
fn games_of(path: &Path) -> Vec<(u64, Json)> {
    let mut games = BTreeMap::new();
    for line in file_lines(path) {
        games.insert(line.get("n").and_then(Json::as_u64).unwrap(), line);
    }
    games.into_iter().collect()
}

#[test]
fn a_short_line_is_repaired_from_the_line_of_the_game_above_it() {
    let (dir, table) = (TempDir::new("unsound-repair"), Optimal::built_in());
    // 395 falls 7 short, played from 390 on with no seed ([`short_395_in`]); 396, seeded with nothing, passes the
    // ceiling that the short line gives, and what survives of its line in 395 scores more than the line of 395.
    let (short, file) = short_395_in(&dir, "repaired.jsonl", 4);
    let (game, repairs) = played((396, 396), &options(4, false), &walking(1), Some(&file));
    assert_eq!(short.score + 7, table.score(395).unwrap());
    assert!(game[0].prev.as_ref().unwrap().short_by > 0, "396 proves the line of 395 short");
    let [repair] = &repairs[..] else { panic!("one repair: {repairs:?}") };
    assert_eq!((repair.n, repair.from, repair.method, repair.was), (395, 396, short.method, short.score));
    assert!(repair.score > short.score && repair.score <= table.score(395).unwrap(), "{repair:?}");
    // The file has the line of 395, then the line of 396, then the line of 395 again: the line played with the
    // repair's moves, which is the game's.
    let written = file_lines(&file);
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
    let (_, repairs) = played((397, 397), &options(4, false), &walking(1), Some(&gap));
    assert_eq!(repairs.iter().map(|r| (r.n, r.from)).collect::<Vec<_>>(), [(395, 397)]);
    let next = lines((396, 396), &options(4, false), &walking(1), Some(&gap)).remove(0);
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
    let (lines, repairs) = played(WALKED, &options(4, false), &walking(1), Some(&file));
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
    for ((n, line), played) in games_of(&file).into_iter().zip(&lines) {
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
}

#[test]
fn a_run_takes_up_the_repair_from_its_file() {
    let dir = TempDir::new("unsound-repair-resume");
    let opts = options(4, false);
    let whole = dir.join("whole.jsonl");
    played((WALKED.0, WALKED.1 + 1), &opts, &walking(1), Some(&whole));
    let whole_text = fs::read_to_string(&whole).unwrap();
    // A file whose last line was never weighed against the lines below it: the seven games as played, before the walk
    // that follows the last.  The next run walks down from it first.
    let unweighed = dir.join("unweighed.jsonl");
    let games: Vec<&str> = whole_text.lines().take(7).collect();
    fs::write(&unweighed, games.join("\n") + "\n").unwrap();
    let (_, repairs) = played((WALKED.0, WALKED.1 + 1), &opts, &walking(1), Some(&unweighed));
    assert_eq!(repairs.iter().map(|r| r.n).collect::<Vec<_>>(), [395, 394, 393, 392, 391, 390]);
    let one_run =
        |games: Vec<(u64, Json)>| games.into_iter().map(|(n, line)| (n, as_one_run(line))).collect::<Vec<_>>();
    assert_eq!(one_run(games_of(&unweighed)), one_run(games_of(&whole)));
    // A walk that a stop cut short after its first step: the file ends with the line of 396 and the repair of 395.
    let cut = dir.join("cut.jsonl");
    let kept: Vec<&str> = whole_text.lines().take(8).collect();
    fs::write(&cut, kept.join("\n") + "\n").unwrap();
    let (_, repairs) = played((WALKED.0, WALKED.1 + 1), &opts, &walking(1), Some(&cut));
    assert_eq!(repairs.iter().map(|r| r.n).collect::<Vec<_>>(), [394, 393, 392, 391, 390], "it goes on from 394");
    let one_run = |path: &Path| file_lines(path).into_iter().map(as_one_run).collect::<Vec<_>>();
    assert_eq!(one_run(&cut), one_run(&whole), "line for line, the file of a run in one go");
}

#[test]
fn the_lines_of_a_run_that_repairs_are_the_same_on_one_thread_and_on_four() {
    let dir = TempDir::new("unsound-repair-threads");
    let (one, four) = (dir.join("one.jsonl"), dir.join("four.jsonl"));
    let (_, repairs) = played(WALKED, &options(1, false), &walking(1), Some(&one));
    played(WALKED, &options(4, false), &walking(1), Some(&four));
    assert_eq!(repairs.len(), 6);
    assert_eq!(file_lines(&one), file_lines(&four));
}

#[test]
fn a_file_holds_a_second_line_of_a_game_only_as_a_repair_that_scores_more() {
    let dir = TempDir::new("unsound-repair-file");
    let file = dir.join("walked.jsonl");
    played(WALKED, &options(4, false), &walking(1), Some(&file));
    let text = fs::read_to_string(&file).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let (played_395, repair_395) = (lines[5], lines[7]);
    let refused = |text: String, expected: &str| {
        let bad = dir.join("bad.jsonl");
        fs::write(&bad, text).unwrap();
        let (opts, plan) = (options(1, true), plan(1));
        let err = unsound::solve_range(400, 400, &opts, &plan, &built_in(Some(&bad)), |_| (), |_| ()).unwrap_err();
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
    let lines = lines((374, 396), &options(2, true), &plan(1), None);
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

/// The level whose rounds the tests of the stop on growth play: 17 rounds of the small games, enough for the stop to
/// wait for its quiet rounds well after the rounds of level 1.
const STOPPING: u32 = 4;

/// The plan of [`plan`] at level [`STOPPING`] with the stop on growth `stop_growth`.
fn stopping(stop_growth: Option<u64>) -> Plan {
    Plan { stop_growth, ..plan(STOPPING) }
}

/// Where the stop with [`STOP_QUIET`] rounds to wait for ends the rounds of a game, from `one` and `full`, its lines
/// with no stop at level 1 and at a higher level: the first round from the end of level 1's rounds on, and before the
/// last, that follows `STOP_QUIET` rounds in a row that raised nothing (the start being round 0).  That round, the score
/// after it, and the rounds since the score last rose; `None` if there is none.
fn quiet_stop(one: &Line, full: &Line) -> Option<(u64, u64, u64)> {
    let last = |round: u64| full.improvements.iter().rev().find(|raised| raised.round <= round);
    let since = |round: u64| round - last(round).map_or(0, |raised| raised.round);
    let score = |round: u64| last(round).map_or_else(|| full.start.expect("a searched game"), |raised| raised.score);
    (one.rounds..full.rounds).find(|&round| since(round) >= STOP_QUIET).map(|round| (round, score(round), since(round)))
}

#[test]
fn the_stop_on_growth_ends_the_rounds_from_the_end_of_the_least_levels_rounds_on_after_its_quiet_rounds() {
    let opts = options(2, false);
    let games = [FIRST.0, FIRST.1];
    let (one, full) = (from_the_table(&games, &opts, &plan(1)), from_the_table(&games, &opts, &stopping(None)));
    // Every gap is within the largest T of the gap below: from level 1's budget on, each game stops as soon as its last
    // STOP_QUIET rounds raised nothing.  375 and 376 raise their scores in their rounds 1 and 2, so that is round 6.
    let stopped = from_the_table(&games, &opts, &stopping(Some(u64::MAX)));
    for ((one, full), stopped) in one.iter().zip(&full).zip(&stopped) {
        let (n, json) = (one.n, stopped.to_json());
        let (round, score, since) = quiet_stop(one, full).expect("the rounds go on well after the last gain");
        assert_eq!((stopped.rounds, stopped.score, stopped.growth_stop, stopped.quiet), (round, score, true, since));
        assert_eq!((round, since), (raised(one).last().unwrap().0 + STOP_QUIET, STOP_QUIET), "n={n}");
        assert!(!one.growth_stop && !full.growth_stop, "n={n}");
        assert!(stopped.work >= stopped.plan.stop_floor() && stopped.work < stopped.plan.budget(), "n={n}");
        assert_eq!(json.get("quiet_rounds"), Some(&Json::from(since)), "n={n}");
        assert!(stopped.summary_line().contains(", stopped"), "{}", stopped.summary_line());
        // The rounds it ends are the same rounds, which raised what the rounds of level 1 raised.
        assert_eq!(raised_by(stopped), raised_by(one), "n={n}");
        assert_eq!(full.to_json().get("quiet_rounds"), Some(&Json::Null), "n={n}");
    }
    // Level 1 stops at its budget whatever the growth: its lines are those without the stop.
    let at_one = from_the_table(&games, &opts, &Plan { stop_growth: Some(u64::MAX), ..plan(1) });
    assert_eq!(with_plan(&at_one, plan(1)), with_plan(&one, plan(1)));
    // With a smaller T, a game stops if its gap is within T of the gap below when the quiet rounds are over, else it
    // plays its level's rounds.  The gap of 375 is 22 smaller than that of the table's 374, so it goes on with a T of 10,
    // never to come within it, and stops with 25; the gap of 376 is the gap below, and it stops with any T.
    assert_eq!(one.iter().map(|line| line.growth).collect::<Vec<_>>(), [Some(-22), Some(0)]);
    let mut ways = [0; 2];
    for most in [10, 25] {
        let lines = from_the_table(&games, &opts, &stopping(Some(most)));
        for ((full, line), growth) in full.iter().zip(&lines).zip(one.iter().map(|one| one.growth.unwrap())) {
            if growth.unsigned_abs() <= most {
                assert!(line.growth_stop && line.rounds < full.rounds, "n={}, T={most}", line.n);
                ways[0] += 1;
            } else {
                assert_eq!(
                    with_plan(slice::from_ref(line), stopping(None)),
                    with_plan(slice::from_ref(full), stopping(None)),
                    "n={}, T={most}",
                    line.n
                );
                ways[1] += 1;
            }
        }
    }
    assert_eq!(ways, [3, 1]);
    // Each line records the stop's T, and the quiet rounds it waited for are built in.
    assert_eq!(stopped[0].to_json().get("stop_growth"), Some(&Json::from(u64::MAX)));
    assert_eq!(full[0].to_json().get("stop_growth"), Some(&Json::Null));
    // The count comes from the rounds in their order, so the lines are the same on one thread.
    let on_one = from_the_table(&games, &options(1, false), &stopping(Some(u64::MAX)));
    assert_eq!(timeless_all(&on_one), timeless_all(&stopped));
}

#[test]
fn the_push_plays_a_game_twice_where_its_gap_grows_by_more_than_the_push_growth() {
    let opts = options(4, true);
    // The gap of the table's line of 468 grows by 35 over that of 467, and the line of the seeded play is the table's
    // score: the unseeded play scores no more, and the seeded play is kept on a tie.
    let tie = lines((468, 468), &opts, &plan(1), None).remove(0);
    assert_eq!((tie.pushed, tie.unseeded, tie.growth), (true, false, Some(35)));
    let other = tie.other.expect("the other play");
    assert_eq!((other.unseeded, other.score, other.method), (true, tie.score, Method::Search));
    assert!(tie.prev.is_some() && tie.ceiling.is_some(), "the line is the seeded play's");
    assert!(tie.summary_line().contains("  pushed, unseeded play"), "{}", tie.summary_line());
    // 539 at level 1 is held down by the seeds it starts from: the seeded play scores 20 less than the unseeded one,
    // which is kept, and its line has no moves for n − 1 and no ceiling.  At level 3 the seeded play gets there, and
    // the game is played once.
    let kept = lines((539, 539), &opts, &plan(1), None).remove(0);
    let other = kept.other.expect("the other play");
    assert_eq!((kept.pushed, kept.unseeded, other.unseeded), (true, true, false));
    assert_eq!(kept.score - other.score, 20);
    assert_eq!((&kept.prev, kept.ceiling, kept.trajectories.clone()), (&None, None, Some(vec![Origin::Start])));
    assert!(kept.summary_line().contains("  pushed, unseeded play kept, +20 on the seeded"));
    let more = lines((539, 539), &opts, &plan(3), None).remove(0);
    assert_eq!((more.pushed, more.other, more.score), (false, None, kept.score));
    // The line is the same on one thread, and its work is its own play's: the unseeded play ran its level's rounds.
    let on_one = lines((539, 539), &options(1, true), &plan(1), None).remove(0);
    assert_eq!(timeless(&on_one), timeless(&kept));
    assert!(kept.work >= plan(1).budget() && other.work >= plan(1).budget());
    // Over a range, a game is played twice if and only if its gap grew by more than the push growth, a trivial game
    // never, and a game with a line that is not the unseeded play's grew by more than it.
    let range = lines((446, 470), &options(4, false), &plan(1), None);
    for line in &range {
        let n = line.n;
        if line.other.is_some() {
            assert!(line.pushed, "n={n}");
        }
        if !line.unseeded {
            let grew = line.method != Method::Trivial && line.growth.is_some_and(|growth| growth > PUSH_GROWTH);
            assert_eq!(line.pushed, grew, "n={n}: growth {:?}", line.growth);
        }
    }
    assert!(range.iter().any(|line| line.pushed) && range.iter().any(|line| !line.pushed));
}

#[test]
fn the_lines_of_a_run_that_stops_and_pushes_are_the_same_on_one_thread_and_on_four() {
    let dir = TempDir::new("unsound-growth");
    let plan = Plan { stop_growth: Some(0), ..walking(2) };
    let (one, four) = (dir.join("one.jsonl"), dir.join("four.jsonl"));
    played(WALKED, &options(1, false), &plan, Some(&one));
    played(WALKED, &options(4, false), &plan, Some(&four));
    assert_eq!(file_lines(&one), file_lines(&four));
}

// ---------------------------------------------------------------------------------------------------------
// The best-known file

/// The built-in table's games below `lo`: a table that lacks the games a test plays, as the built-in table lacks the
/// games past 5060, so that the best-known file may hold them.
fn table_below(lo: u32) -> Optimal {
    Optimal::built_in().up_to(lo - 1)
}

/// The lines of the file `path` as JSON, in order.
fn json_lines(path: &Path) -> Vec<Json> {
    fs::read_to_string(path).unwrap().lines().map(|line| Json::parse(line).unwrap()).collect()
}

/// The game of a line, and its score.
fn game_and_score(line: &Json) -> (u32, u64) {
    (line.get("n").and_then(Json::as_u32).unwrap(), line.get("score").and_then(Json::as_u64).unwrap())
}

/// The key `best_known` of a line of a best-known file: the name of the file the line came from, and the score of the
/// game's line before it.
fn best_known_of(line: &Json) -> (Option<&str>, Option<u64>) {
    let key = line.get("best_known").expect("a line of a best-known file has `best_known`");
    (key.get("file").and_then(Json::as_str), key.get("was").and_then(Json::as_u64))
}

/// `line` without its key `best_known`.
fn without_best_known(line: &Json) -> Json {
    let Json::Obj(pairs) = line else { panic!("a line is an object") };
    Json::Obj(pairs.iter().filter(|(key, _)| key != "best_known").cloned().collect())
}

/// The best-known file `path` holds, the last line of each game, after the checks of a best-known file: in order, each
/// line scores more than the one before it of its game and says that score in `best_known.was`.
fn last_lines(path: &Path) -> BTreeMap<u32, Json> {
    let mut last: BTreeMap<u32, Json> = BTreeMap::new();
    for line in json_lines(path) {
        let (n, score) = game_and_score(&line);
        let before = last.get(&n).map(|line| game_and_score(line).1);
        assert!(before.is_none_or(|before| score > before), "n={n}: {score} is no more than the {before:?} before it");
        assert_eq!(best_known_of(&line).1, before, "n={n}: best_known.was is the score before");
        last.insert(n, line);
    }
    last
}

#[test]
fn a_line_that_scores_more_than_the_best_known_files_is_added_with_the_score_it_beat() {
    let (dir, table) = (TempDir::new("unsound-best-known"), table_below(390));
    let best_known = dir.join("best-known.jsonl");
    let run = |name: &str, effort: u32| {
        let out = dir.join(name);
        let files = Files { table: &table, out: Some(&out), best_known: Some(&best_known) };
        played_in((390, 392), &options(4, false), &walking(effort), &files)
    };
    // At level 1, 390 falls short of the table's score and 391 and 392 keep the shortfall; the best-known file has no
    // line yet, so it takes all three, each saying where it came from.
    let (a, _, summary) = run("a.jsonl", 1);
    assert_eq!(summary.best_known_added, 3);
    let lines = json_lines(&best_known);
    assert_eq!(lines.len(), 3);
    for (line, played) in lines.iter().zip(&a) {
        assert_eq!(game_and_score(line), (played.n, played.score));
        assert_eq!(best_known_of(line), (Some("a.jsonl"), None));
        assert_eq!(without_best_known(line), played.to_json(), "n={}: the line as the run wrote it", played.n);
    }
    // At level 3, a run reaches the table's score: its lines are added, each saying the score it beat.
    let (b, _, summary) = run("b.jsonl", 3);
    let better: Vec<(&Line, &Line)> = b.iter().zip(&a).filter(|(b, a)| b.score > a.score).collect();
    assert_eq!(better.len(), 3, "level 3 lifts the three games above their shortfall");
    assert_eq!(summary.best_known_added, better.len());
    assert_eq!(json_lines(&best_known).len(), 3 + better.len());
    let last = last_lines(&best_known);
    for (b, a) in b.iter().zip(&a) {
        let line = &last[&b.n];
        assert_eq!(game_and_score(line).1, a.score.max(b.score), "n={}", b.n);
        if b.score > a.score {
            assert_eq!(best_known_of(line), (Some("b.jsonl"), Some(a.score)), "n={}", b.n);
            assert_eq!(without_best_known(line), b.to_json(), "n={}", b.n);
        }
    }
    // The same run again scores what the best-known file has, and not more: it adds nothing, and starts game 391 from
    // its own line of 390, which the best-known file's ties.
    let before = fs::read(&best_known).unwrap();
    let (c, _, summary) = run("c.jsonl", 3);
    assert_eq!(summary.best_known_added, 0);
    assert_eq!(fs::read(&best_known).unwrap(), before);
    assert_eq!(timeless_all(&c), timeless_all(&b));
    assert_eq!(c[1].prev.as_ref().map(|prev| (prev.n, prev.source)), Some((390, Source::Run)));
    // A run at level 1 again scores less than the best-known file's in 390, and adds nothing; it starts game 391 from
    // the best-known file's line of 390, which scores more than its own.
    let (d, _, summary) = run("d.jsonl", 1);
    assert_eq!(summary.best_known_added, 0);
    assert_eq!(fs::read(&best_known).unwrap(), before);
    assert!(d[0].score < b[0].score);
    let seeded = d[1].prev.as_ref().map(|prev| (prev.n, prev.score, prev.source));
    assert_eq!(seeded, Some((390, b[0].score, Source::BestKnown)));
    // The file reads back with the checks of a best-known file.
    let read = BestKnownFile::open(&best_known, &table).unwrap();
    assert!(b.iter().zip(&a).all(|(b, a)| read.score(b.n) == Some(a.score.max(b.score))));
}

#[test]
fn a_game_starts_from_a_line_that_another_run_appended_to_the_best_known_file_while_it_ran() {
    let (dir, table) = (TempDir::new("unsound-best-known-shared"), table_below(390));
    let (best_known, a_out, b_out) = (dir.join("best-known.jsonl"), dir.join("a.jsonl"), dir.join("b.jsonl"));
    let (to_b, b_waits) = mpsc::channel::<()>();
    let (to_a, a_waits) = mpsc::channel::<()>();
    let (table, best_known, a_out, b_out) = (&table, &best_known, &a_out, &b_out);
    let patience = Duration::from_secs(120);
    // Run A plays 390 to 392 at level 1; after its game 390, run B plays 390 at level 3, in a thread of its own, and A
    // goes on once B has added its line.
    let (a, b) = thread::scope(|scope| {
        let a = scope.spawn(move || {
            let files = Files { table, out: Some(a_out), best_known: Some(best_known) };
            let mut lines = Vec::new();
            let on_game = |line: &Line| {
                lines.push(line.clone());
                if line.n == 390 {
                    to_b.send(()).unwrap();
                    a_waits.recv_timeout(patience).expect("B added its line of 390");
                }
            };
            unsound::solve_range(390, 392, &options(1, false), &walking(1), &files, on_game, |_| ()).unwrap();
            lines
        });
        let b = scope.spawn(move || {
            b_waits.recv_timeout(patience).expect("A played 390");
            let files = Files { table, out: Some(b_out), best_known: Some(best_known) };
            let (lines, ..) = played_in((390, 390), &options(1, false), &walking(3), &files);
            to_a.send(()).unwrap();
            lines
        });
        (a.join().unwrap(), b.join().unwrap())
    });
    assert!(b[0].score > a[0].score, "B's 390 at level 3 scores more than A's at level 1");
    // A's game 391 started from the best-known file's 390, which scores more than A's own, and says so.
    let prev = a[1].prev.as_ref().expect("the moves for 390");
    assert_eq!((prev.n, prev.score, prev.source), (390, b[0].score, Source::BestKnown));
    let ceiling = 391 + b[0].score;
    assert_eq!(a[1].ceiling, Some(ceiling));
    // The file has A's 390, then B's, then A's 391 and 392, and reads back in order.
    let lines = json_lines(best_known);
    assert_eq!(lines.iter().map(game_and_score).map(|(n, _)| n).collect::<Vec<_>>(), [390, 390, 391, 392]);
    assert_eq!(best_known_of(&lines[1]), (Some("b.jsonl"), Some(a[0].score)));
    assert_eq!(best_known_of(&lines[2]), (Some("a.jsonl"), None));
    let read = BestKnownFile::open(best_known, table).unwrap();
    assert_eq!((read.score(390), read.score(391)), (Some(b[0].score), Some(a[1].score)));
}

#[test]
fn a_run_adds_its_repairs_to_the_best_known_file_and_walks_down_into_its_lines_of_the_games_below_the_range() {
    let (dir, table) = (TempDir::new("unsound-best-known-repair"), table_below(390));
    // A run of 390 to 396 with the repair: each repair is offered, so the best-known file's last line of a game is the
    // run's.
    let (best_known, out) = (dir.join("best-known.jsonl"), dir.join("a.jsonl"));
    let files = Files { table: &table, out: Some(&out), best_known: Some(&best_known) };
    let (lines, repairs, summary) = played_in(WALKED, &options(4, false), &walking(1), &files);
    assert_eq!((lines.len(), repairs.len(), summary.repaired), (7, 6, 6));
    assert_eq!((summary.best_known_added, summary.best_known_repaired), (13, 0), "the 7 games and the 6 repairs");
    let (ours, theirs) = (games_of(&out), last_lines(&best_known));
    assert_eq!(theirs.len(), 7);
    for (n, line) in &ours {
        assert_eq!(timeless_json(&without_best_known(&theirs[&(*n as u32)])), *line, "n={n}");
    }
    assert_eq!(theirs[&395].get("repaired").and_then(|repaired| repaired.get("from")), Some(&Json::from(396u32)));

    // The best-known file holds A's lines of 390 to 395, 7 short each; B plays 396 alone, starting from the
    // best-known file's line of 395, and its walk goes down into the best-known file's lines of 395 to 390, which B has
    // no line of.
    let base = dir.join("base.jsonl");
    played_in((390, 395), &options(4, false), &walking(1), &Files { out: None, best_known: Some(&base), ..files });
    let b_run = |threads: usize| {
        let sub = dir.join(format!("threads-{threads}"));
        fs::create_dir(&sub).unwrap();
        let (best_known, out) = (sub.join("best-known.jsonl"), sub.join("b.jsonl"));
        fs::copy(&base, &best_known).unwrap();
        let files = Files { table: &table, out: Some(&out), best_known: Some(&best_known) };
        (played_in((396, 396), &options(threads, false), &walking(1), &files), best_known, out)
    };
    let ((lines, repairs, summary), best_known, out) = b_run(4);
    assert_eq!(lines.len(), 1);
    let walked: Vec<(u32, u32, u64, bool)> =
        repairs.iter().map(|r| (r.n, r.from, r.score - r.was, r.best_known_only)).collect();
    let down = [(395, 396), (394, 395), (393, 394), (392, 393), (391, 392), (390, 391)];
    assert_eq!(walked, down.map(|(n, from)| (n, from, 5, true)));
    assert!(repairs[0].summary_line().ends_with("best-known file only"));
    assert_eq!((summary.repaired, summary.best_known_repaired, summary.best_known_added), (0, 6, 7));
    assert_eq!(fs::read_to_string(&out).unwrap().lines().count(), 1, "the run's file has its own game alone");
    let last = last_lines(&best_known);
    let before = last_lines(&base);
    for (n, from) in down {
        let (repaired, line) = (&last[&n], &before[&n]);
        let (score, was) = (game_and_score(repaired).1, game_and_score(line).1);
        assert_eq!((score, best_known_of(repaired)), (was + 5, (Some("b.jsonl"), Some(was))), "n={n}");
        let record = repaired.get("repaired").unwrap();
        assert_eq!((record.get("from"), record.get("was")), (Some(&Json::from(from)), Some(&Json::from(was))));
    }
    // The lines are the same on one thread and on four.
    let ((one, ..), one_best_known, _) = b_run(1);
    assert_eq!(timeless_all(&one), timeless_all(&lines));
    assert_eq!(file_lines(&one_best_known), file_lines(&best_known));
}

#[test]
fn a_fresh_best_known_file_changes_no_line_of_a_run() {
    let (dir, table) = (TempDir::new("unsound-best-known-fresh"), table_below(390));
    let run = |best_known: Option<&Path>| {
        let out = dir.join(if best_known.is_some() { "with.jsonl" } else { "without.jsonl" });
        played_in(WALKED, &options(4, false), &walking(1), &Files { table: &table, out: Some(&out), best_known })
    };
    let (without, repairs_without, _) = run(None);
    let (with, repairs, _) = run(Some(&dir.join("best-known.jsonl")));
    assert_eq!(timeless_all(&with), timeless_all(&without));
    let walked = |repairs: &[Repair]| repairs.iter().map(|r| (r.n, r.from, r.was, r.score)).collect::<Vec<_>>();
    assert_eq!(walked(&repairs), walked(&repairs_without));
    assert_eq!(file_lines(&dir.join("with.jsonl")), file_lines(&dir.join("without.jsonl")));
}

#[test]
fn a_run_offers_the_last_line_of_each_game_of_its_file_to_the_best_known_file_when_it_opens_it() {
    let (dir, table) = (TempDir::new("unsound-best-known-open"), table_below(390));
    // A's file has repairs: the last line of 395 to 390 is one, and the line before it is in the file only.
    let a = dir.join("a.jsonl");
    played_in(WALKED, &options(4, false), &walking(1), &Files { table: &table, out: Some(&a), best_known: None });
    assert_eq!(json_lines(&a).len(), 13);
    // A run over the same range with a copy of the file plays nothing, and offers all it has.
    let (copy, best_known) = (dir.join("copy.jsonl"), dir.join("best-known.jsonl"));
    fs::copy(&a, &copy).unwrap();
    let files = Files { table: &table, out: Some(&copy), best_known: Some(&best_known) };
    let (lines, repairs, summary) = played_in(WALKED, &options(4, false), &walking(1), &files);
    assert_eq!((lines.len(), repairs.len(), summary.skipped, summary.best_known_added), (0, 0, 7, 7));
    let offered = json_lines(&best_known);
    assert_eq!(offered.iter().map(|line| game_and_score(line).0).collect::<Vec<_>>(), (390..=396).collect::<Vec<_>>());
    let ours = games_of(&a);
    for line in &offered {
        let n = game_and_score(line).0;
        assert_eq!(best_known_of(line), (Some("copy.jsonl"), None), "n={n}");
        let ours = &ours.iter().find(|(m, _)| *m == u64::from(n)).unwrap().1;
        assert_eq!(timeless_json(&without_best_known(line)), *ours, "n={n}: the file's last line of the game");
    }
    // The first line of a game in the best-known file may record a repair, since the line it replaced is in A's file,
    // and the best-known file reads back.
    assert!(offered[5].get("repaired").is_some_and(|repaired| !repaired.is_null()));
    let read = BestKnownFile::open(&best_known, &table).unwrap();
    assert!((390..=396).all(|n| read.score(n) == Some(game_and_score(&offered[(n - 390) as usize]).1)));
    // Offered again, the lines add nothing.
    let (_, _, summary) = played_in(WALKED, &options(4, false), &walking(1), &files);
    assert_eq!((summary.skipped, summary.best_known_added), (7, 0));
}

#[test]
fn a_best_known_file_never_holds_a_game_of_the_built_in_table_and_is_checked_as_a_best_known_file() {
    let (dir, table) = (TempDir::new("unsound-best-known-checks"), table_below(390));
    // The built-in table has every game these runs play, so nothing is added.
    let (best_known, out) = (dir.join("best-known.jsonl"), dir.join("built-in.jsonl"));
    let (_, _, summary) =
        played_in((390, 392), &options(4, true), &plan(1), &Files::with_built_in(Some(&out), Some(&best_known)));
    assert_eq!(summary.best_known_added, 0);
    assert_eq!(fs::read_to_string(&best_known).unwrap(), "");

    // A best-known file of the games 390 to 392, A's lines at level 1 and then B's at level 3.
    let (a_out, b_out) = (dir.join("a.jsonl"), dir.join("b.jsonl"));
    let (a, b) = (Some(a_out.as_path()), Some(b_out.as_path()));
    let known = Some(best_known.as_path());
    played_in((390, 392), &options(4, false), &walking(1), &Files { table: &table, out: a, best_known: known });
    played_in((390, 392), &options(4, false), &walking(3), &Files { table: &table, out: b, best_known: known });
    let text = fs::read_to_string(&best_known).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert!(lines.len() > 3 && game_and_score(&Json::parse(lines[3]).unwrap()).0 == 390, "B's line of 390 follows A's");
    let refused = |text: String, table: &Optimal, expected: &str| {
        let file = dir.join("bad.jsonl");
        fs::write(&file, text).unwrap();
        let err = BestKnownFile::open(&file, table).err().expect("a refusal");
        assert!(err.contains(expected), "{err}");
    };
    refused(text.clone(), Optimal::built_in(), "the built-in table has this game");
    refused(format!("{}\n{}\n", lines[0], lines[0]), &table, "line 2: (n=390) it scores");
    refused(format!("{}\n{}\n{}\n", lines[0], lines[3], lines[0]), &table, "line 3: (n=390) it scores");
    let plain = fs::read_to_string(dir.join("a.jsonl")).unwrap();
    refused(plain, &table, "line 1: (n=390) a best-known file's line needs a best_known");
    let (first, second) = (game_and_score(&Json::parse(lines[0]).unwrap()).1, lines[3]);
    let lost = second.replace(&format!("\"was\":{first}}}"), "\"was\":null}");
    assert_ne!(lost, second, "the second line of 390 says the score it beat");
    refused(format!("{}\n{lost}\n", lines[0]), &table, "best_known.was is None");
    refused(format!("{}\nnot json\n{}\n", lines[0], lines[1]), &table, "line 2");

    // A torn last line is cut off, and the file goes on from the line before it.
    fs::write(&best_known, format!("{text}{{\"unsound\":tr")).unwrap();
    let mut read = BestKnownFile::open(&best_known, &table).unwrap();
    assert_eq!(fs::read_to_string(&best_known).unwrap(), text);
    // A file another process replaced by a shorter one is refused at the next read.
    fs::write(&best_known, "").unwrap();
    assert!(read.refresh().unwrap_err().contains("shorter than"));
}

// ---------------------------------------------------------------------------------------------------------
// A rehearsal: the built-in table cut at `known_to`

/// The plan of a rehearsal at level `effort` on games the built-in table has: the table is cut at `known_to`.
fn rehearsal(effort: u32, known_to: u32) -> Plan {
    Plan { known_to: Some(known_to), ..walking(effort) }
}

#[test]
fn with_known_to_the_games_above_it_are_unproven_and_the_best_known_file_takes_and_seeds_from_their_lines() {
    let dir = TempDir::new("unsound-known-to");
    let best_known = dir.join("best-known.jsonl");
    let run = |name: &str, range: (u32, u32), effort: u32| {
        let out = dir.join(name);
        let files = Files::with_built_in(Some(&out), Some(&best_known));
        played_in(range, &options(4, false), &rehearsal(effort, 389), &files)
    };
    // A run of 390 to 392 plays them as a run with a table of the games up to 389 does, but for the key `known_to`, and
    // starts from the table's 389.
    let (a, _, summary) = run("a.jsonl", (390, 392), 1);
    let cut = table_below(390);
    let (alone, ..) =
        played_in((390, 392), &options(4, false), &walking(1), &Files { table: &cut, out: None, best_known: None });
    let as_rehearsed: Vec<Line> =
        timeless_all(&alone).into_iter().map(|l| Line { plan: rehearsal(1, 389), ..l }).collect();
    assert_eq!(timeless_all(&a), as_rehearsed);
    let prev = |line: &Line| line.prev.as_ref().map(|prev| (prev.n, prev.source));
    assert_eq!((prev(&a[0]), prev(&a[1])), (Some((389, Source::Table)), Some((390, Source::Run))));
    assert_eq!(a[0].to_json().get("known_to"), Some(&Json::from(389u32)));
    assert_eq!(alone[0].to_json().get("known_to"), Some(&Json::Null));
    // The built-in table has the three games, and the best-known file takes their lines.
    assert!((390..=392).all(|n| Optimal::built_in().score(n).is_some()));
    assert_eq!(summary.best_known_added, 3);
    let lines = json_lines(&best_known);
    assert_eq!(
        lines.iter().map(game_and_score).collect::<Vec<_>>(),
        a.iter().map(|l| (l.n, l.score)).collect::<Vec<_>>()
    );
    // A later run, on a copy of the file, starts game 391 from the best-known file's 390, not from the table's.
    let (copy, out) = (dir.join("copy.jsonl"), dir.join("c.jsonl"));
    fs::copy(&best_known, &copy).unwrap();
    let files = Files::with_built_in(Some(&out), Some(&copy));
    let (c, ..) = played_in((391, 391), &options(4, false), &rehearsal(1, 389), &files);
    assert_eq!(prev(&c[0]), Some((390, Source::BestKnown)));
    assert_eq!(c[0].prev.as_ref().map(|prev| prev.score), Some(a[0].score));
    // At level 3 the three lines score more, and are added, each saying the score it beat.
    let (b, _, summary) = run("b.jsonl", (390, 392), 3);
    assert_eq!(summary.best_known_added, 3);
    let last = last_lines(&best_known);
    for (b, a) in b.iter().zip(&a) {
        assert!(b.score > a.score, "n={}", b.n);
        assert_eq!(best_known_of(&last[&b.n]), (Some("b.jsonl"), Some(a.score)), "n={}", b.n);
    }
    // The games up to 389 are the table's: the best-known file takes none of their lines.
    let (known, _, summary) = run("d.jsonl", (388, 389), 1);
    assert_eq!((known.len(), summary.best_known_added), (2, 0));
    assert_eq!(known[1].to_json().get("known_to"), Some(&Json::from(389u32)));
}

#[test]
fn a_best_known_file_made_with_known_to_is_refused_by_a_run_that_knows_a_game_it_holds() {
    let dir = TempDir::new("unsound-known-to-refused");
    let (best_known, out) = (dir.join("best-known.jsonl"), dir.join("a.jsonl"));
    let files = Files::with_built_in(Some(&out), Some(&best_known));
    played_in((390, 391), &options(4, false), &rehearsal(1, 389), &files);
    // A run with a table cut at the game below the file's games reads it, and one that knows game 390 does not.
    BestKnownFile::open(&best_known, &Optimal::built_in().up_to(389)).unwrap();
    let cut = Optimal::built_in().up_to(390);
    for table in [&cut, Optimal::built_in()] {
        let err = BestKnownFile::open(&best_known, table).err().expect("a refusal");
        assert!(err.contains("line 1: (n=390) the built-in table has this game"), "{err}");
    }
    // The run without the flag is refused before it plays a game; with the flag below the file's games, it is not.
    let run = |known_to: Option<u32>| {
        let (plan, other) = (Plan { known_to, ..plan(1) }, dir.join("b.jsonl"));
        let files = Files::with_built_in(Some(&other), Some(&best_known));
        unsound::solve_range(392, 392, &options(4, false), &plan, &files, |_| (), |_| ())
    };
    let err = run(None).unwrap_err();
    assert!(err.contains("(n=390) the built-in table has this game"), "{err}");
    assert_eq!(run(Some(389)).unwrap().solved, 1);
}

#[test]
fn a_run_with_known_to_folds_an_old_file_of_the_games_into_its_best_known_file() {
    let dir = TempDir::new("unsound-known-to-fold");
    // An old file of the games 390 to 392, made by a run with the whole table.
    let old = dir.join("old.jsonl");
    let (lines, ..) = played_in((390, 392), &options(4, false), &walking(1), &built_in(Some(&old)));
    assert!(json_lines(&old).iter().all(|line| line.get("known_to") == Some(&Json::Null)));
    // A run over their range with a copy of the file as `--out` plays nothing and offers every line.
    let (copy, best_known) = (dir.join("copy.jsonl"), dir.join("best-known.jsonl"));
    fs::copy(&old, &copy).unwrap();
    let files = Files::with_built_in(Some(&copy), Some(&best_known));
    let (played, repairs, summary) = played_in((390, 392), &options(4, false), &rehearsal(1, 389), &files);
    assert_eq!((played.len(), repairs.len(), summary.skipped, summary.best_known_added), (0, 0, 3, 3));
    let offered = json_lines(&best_known);
    assert_eq!(
        offered.iter().map(game_and_score).collect::<Vec<_>>(),
        lines.iter().map(|l| (l.n, l.score)).collect::<Vec<_>>()
    );
    assert!(offered.iter().all(|line| best_known_of(line) == (Some("copy.jsonl"), None)));
    // The same run again adds nothing, and the file reads back with the table cut at 389.
    let (_, _, summary) = played_in((390, 392), &options(4, false), &rehearsal(1, 389), &files);
    assert_eq!((summary.skipped, summary.best_known_added), (3, 0));
    BestKnownFile::open(&best_known, &table_below(390)).unwrap();
}
