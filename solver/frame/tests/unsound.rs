//! The unsound mode (`taxman::solver::unsound`): its lines replay to their scores and never beat the built-in table,
//! are the same on one thread and on four, do the rounds of a lower level first, resume, and end their rounds at the
//! table's score with `--stop-at-known`; and the wall between its lines and proven ones holds both ways.  The options
//! that shape the rounds (`fix`, `survivors`) change no line unless they are given, and keep all of the above.

mod common;

use common::TempDir;
use std::fs;
use std::path::Path;
use std::slice;
use taxman::game::{replay, Optimal};
use taxman::json::Json;
use taxman::results::{moves_digest, verify_files, ResultsFile};
use taxman::solver::unsound::{self, Line, Plan, Previous, Raised, Source};
use taxman::solver::{solve_range, Method, SolveOptions};

/// The plan of a run at `effort`, with the default seed and no measuring option.
fn plan(effort: u32) -> Plan {
    Plan { effort, ..Plan::default() }
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

/// The rounds that raised a line's score, and the scores they raised it to.
fn raised(line: &Line) -> Vec<(u64, u64)> {
    line.improvements.iter().map(|raised| (raised.round, raised.score)).collect()
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
    assert_eq!(one.iter().map(timeless).collect::<Vec<_>>(), four.iter().map(timeless).collect::<Vec<_>>());
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
        assert_eq!((less.rounds, more.rounds), (1, 2), "n={}", less.n);
    }
    // 375 is raised in its second round only, 376 in both.
    assert_eq!((raised(&less[0]).len(), raised(&more[0]).len()), (0, 1));
    assert_eq!((raised(&less[1]).len(), raised(&more[1]).len()), (1, 2));
}

/// Games whose rounds, from a start made with the table's moves for n − 1, find different sets under the options that
/// shape the rounds: every option reaches the table's score of 375, 539 and 544 (the default does too), and of 680, to
/// which the default's four rounds fall 42 short.  Each starts from the table's moves, so it is the same game whatever
/// the lines before it.
const SHAPED: [u32; 4] = [375, 539, 544, 680];

/// The plans of the options at `effort`: a share fixed other than the default's, a second trajectory, and both (with
/// another share fixed, under which the rounds of both trajectories raise the score of 375 and 544 twice), for the
/// games [`SHAPED`].
fn shaped(effort: u32) -> [(&'static str, Plan); 3] {
    let known = Plan { from_known: true, ..plan(effort) };
    [
        ("fix", Plan { fix: (0.4, 0.6), ..known }),
        ("survivors", Plan { survivors: true, ..known }),
        ("fix and survivors", Plan { fix: (0.6, 0.8), survivors: true, ..known }),
    ]
}

/// The lines of the games [`SHAPED`] played with `plan`, which starts them from the table's moves.
fn shaped_lines(opts: &SolveOptions, plan: &Plan) -> Vec<Line> {
    assert!(plan.from_known);
    SHAPED.iter().flat_map(|&n| lines((n, n), opts, plan, None)).collect()
}

/// The rounds that raised a line's score, the trajectories that made them, and the scores.
fn raised_by(line: &Line) -> Vec<(u64, usize, u64)> {
    line.improvements.iter().map(|raised| (raised.round, raised.trajectory, raised.score)).collect()
}

/// What a line of the first form of the mode (commit e37d6a1, before `fix` and `survivors`) held of a game.
struct Was {
    n: u32,
    start: Option<u64>,
    score: u64,
    steps: u64,
    /// The rounds that raised the score, and the scores.
    raised: &'static [(u64, u64)],
    /// [`moves_digest`] of the moves.
    moves: &'static str,
}

/// The games [`SHAPED`] from the table's moves for n − 1 at effort 3, as the first form played them.
const WAS_SHAPED: [Was; 4] = [
    Was {
        n: 375,
        start: Some(44303),
        score: 44328,
        steps: 324,
        raised: &[(2, 44327), (3, 44328)],
        moves: "c60134b0257ae6c6",
    },
    Was { n: 539, start: Some(91632), score: 91703, steps: 328, raised: &[(2, 91703)], moves: "2951cc1d96d9b139" },
    Was { n: 544, start: Some(93214), score: 93256, steps: 339, raised: &[(1, 93256)], moves: "b0d3b082dc846aac" },
    Was {
        n: 680,
        start: Some(146758),
        score: 146784,
        steps: 260,
        raised: &[(1, 146773), (4, 146784)],
        moves: "702a5d8476976273",
    },
];

/// The games [`SECOND`], with no seed, at effort 2, as the first form played them (386 repeats 385).
const WAS_SECOND: [Was; 4] = [
    Was { n: 384, start: Some(45975), score: 45999, steps: 153, raised: &[(1, 45999)], moves: "1ef59033cec8af1d" },
    Was {
        n: 385,
        start: Some(46360),
        score: 46384,
        steps: 157,
        raised: &[(1, 46368), (2, 46384)],
        moves: "ad6f91f2a7f43303",
    },
    Was { n: 386, start: None, score: 46770, steps: 0, raised: &[], moves: "093617dce8a0500c" },
    Was { n: 387, start: Some(47133), score: 47157, steps: 151, raised: &[(1, 47157)], moves: "d121c33eea93cf8c" },
];

/// `lines` are what the first form held of these games, and say they followed one trajectory with the default share.
fn assert_were(lines: &[Line], was: &[Was]) {
    assert_eq!(lines.len(), was.len());
    for (line, was) in lines.iter().zip(was) {
        let n = was.n;
        let now = (line.n, line.start, line.score, line.steps, raised(line), moves_digest(&line.moves));
        assert_eq!(now, (n, was.start, was.score, was.steps, was.raised.to_vec(), was.moves.to_owned()), "n={n}");
        assert_eq!((line.plan.fix, line.plan.survivors), ((0.7, 0.9), false), "n={n}");
        assert_eq!(line.trajectories, line.start.map(|_| 1), "n={n}: one trajectory for a searched game");
        assert!(line.improvements.iter().all(|raised| raised.trajectory == 0), "n={n}");
    }
}

#[test]
fn without_the_options_the_lines_are_those_of_the_first_form() {
    assert_were(&shaped_lines(&options(4, true), &Plan { from_known: true, ..plan(3) }), &WAS_SHAPED);
    assert_were(&lines(SECOND, &options(4, false), &plan(2), None), &WAS_SECOND);
}

/// The module `$module` of three tests, `fix`, `survivors` and `fix_and_survivors`, each calling `$check` with the
/// index of that option in [`shaped`]: one test of all three would use more CPU than a test may.
macro_rules! for_each_option {
    ($module:ident, $check:ident) => {
        mod $module {
            #[test]
            fn fix() {
                super::$check(0);
            }
            #[test]
            fn survivors() {
                super::$check(1);
            }
            #[test]
            fn fix_and_survivors() {
                super::$check(2);
            }
        }
    };
}

/// The lines of the games [`SHAPED`] under option `shape` of [`shaped`] are the same on one thread and on four.
fn check_same_on_one_thread_and_on_four(shape: usize) {
    let shape = shaped(3)[shape].1;
    let one = shaped_lines(&options(1, true), &shape);
    let four = shaped_lines(&options(4, true), &shape);
    assert_eq!(one.iter().map(timeless).collect::<Vec<_>>(), four.iter().map(timeless).collect::<Vec<_>>());
    assert!(one.iter().all(|line| line.threads == 1) && four.iter().all(|line| line.threads == 4));
}
for_each_option!(same_on_one_thread_and_on_four, check_same_on_one_thread_and_on_four);

/// Under option `shape` of [`shaped`], the improvements of the games [`SHAPED`] at effort 1 are a prefix of those at
/// effort 2, and those at effort 2 of those at effort 3: the rounds, the trajectories and the scores.
fn check_lower_level_first(shape: usize) {
    let by_level: Vec<Vec<Line>> =
        (1..=3).map(|effort| shaped_lines(&options(4, true), &shaped(effort)[shape].1)).collect();
    for (less, more) in by_level.iter().zip(&by_level[1..]) {
        for (less, more) in less.iter().zip(more) {
            let (n, raised) = (less.n, (raised_by(less), raised_by(more)));
            assert!(raised.1.starts_with(&raised.0), "n={n}: {:?}, then {:?}", raised.0, raised.1);
            assert_eq!(more.rounds, 2 * less.rounds, "n={n}");
        }
    }
    // The longer run finds more, so the comparison shows something.
    let found: Vec<usize> = by_level.iter().map(|lines| lines.iter().map(|l| l.improvements.len()).sum()).collect();
    assert!(found[0] < found[2], "improvements at efforts 1 to 3: {found:?}");
}
for_each_option!(lower_level_first, check_lower_level_first);

/// Option `shape` of [`shaped`] reaches the table's score of every game [`SHAPED`] in four rounds, 680 included, and
/// the line records the option and the trajectories the game followed.
fn check_what_the_option_finds(shape: usize) {
    let (what, shape) = shaped(3)[shape];
    let lines = shaped_lines(&options(4, true), &shape);
    let scores: Vec<u64> = lines.iter().map(|line| line.score).collect();
    assert_eq!(scores, [44328, 91703, 93256, 146826], "the default's rounds leave game 680 at 146,784");
    let trajectories: Vec<Option<usize>> = lines.iter().map(|line| line.trajectories).collect();
    assert_eq!(trajectories, [Some(if shape.survivors { 2 } else { 1 }); 4]);
    assert!(lines.iter().all(|line| (line.plan.fix, line.plan.survivors) == (shape.fix, shape.survivors)));
    let json = lines[0].to_json();
    assert_eq!(json.get("fix"), Some(&Json::array([shape.fix.0, shape.fix.1])));
    assert_eq!(json.get("survivors"), Some(&Json::Bool(shape.survivors)));
    if what == "survivors" {
        // The second trajectory raises the score first in all four games, the start's trajectory passes it in 375 and
        // 539 (680 is raised twice by the second), and the line says which made each improvement.
        let by = |line: &Line| line.improvements.iter().map(|raised| raised.trajectory).collect::<Vec<_>>();
        assert_eq!(lines.iter().map(by).collect::<Vec<_>>(), [vec![1, 0], vec![1, 0], vec![1], vec![1, 1]]);
        let in_round = |line: &Line| line.improvements.iter().map(|raised| raised.round).collect::<Vec<_>>();
        assert_eq!(lines.iter().map(in_round).collect::<Vec<_>>(), [vec![2, 3], vec![1, 2], vec![2], vec![1, 2]]);
        let record = json.get("improvements").and_then(Json::as_arr).expect("the improvements");
        assert_eq!(record[0].get("trajectory"), Some(&Json::from(1u64)));
        assert_eq!(record[1].get("trajectory"), Some(&Json::from(0u64)));
    }
}
for_each_option!(what_the_option_finds, check_what_the_option_finds);

#[test]
fn a_game_with_no_seed_follows_one_trajectory_whatever_the_switch() {
    let (with, without) = (Plan { survivors: true, ..plan(2) }, plan(2));
    let (with, without) =
        (lines(SECOND, &options(4, false), &with, None), lines(SECOND, &options(4, false), &without, None));
    // But for the switch the plan records, the lines are the same, and say that the searched games followed one.
    let unswitched = |line: &Line| Line { plan: Plan { survivors: false, ..line.plan }, ..timeless(line) };
    assert_eq!(with.iter().map(unswitched).collect::<Vec<_>>(), without.iter().map(unswitched).collect::<Vec<_>>());
    assert!(with.iter().all(|line| line.plan.survivors && line.trajectories == line.start.map(|_| 1)));
    assert!(with.iter().any(|line| line.start.is_some()), "a searched game");
}

#[test]
fn a_share_fixed_that_is_not_a_range_inside_0_to_1_is_refused() {
    for fix in [(0.9, 0.7), (-0.1, 0.5), (0.5, 1.1), (f64::NAN, 0.5), (0.5, f64::INFINITY)] {
        let plan = Plan { fix, ..plan(1) };
        let err = unsound::solve_range(12, 12, &options(1, true), &plan, None, |_| ()).unwrap_err();
        assert!(err.starts_with("--fix takes LO,HI") && err.contains("0 <= LO <= HI <= 1"), "{fix:?}: {err}");
    }
}

#[test]
fn a_run_resumes_from_its_file() {
    let dir = TempDir::new("unsound-resume");
    let (opts, plan, (lo, hi)) = (options(4, false), plan(1), SECOND);
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
