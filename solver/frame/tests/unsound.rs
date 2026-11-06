//! The unsound mode (`taxman::solver::unsound`): its lines replay to their scores and never beat the built-in table,
//! are the same on one thread and on four, do the rounds of a lower level first, resume, and end their rounds at the
//! table's score with `--stop-at-known`; and the wall between its lines and proven ones holds both ways.

mod common;

use common::TempDir;
use std::fs;
use std::path::Path;
use std::slice;
use taxman::game::{replay, Optimal};
use taxman::json::Json;
use taxman::results::{verify_files, ResultsFile};
use taxman::solver::unsound::{self, Line, Plan, Previous, Raised, Source};
use taxman::solver::{solve_range, Method, SolveOptions};

/// The plan of a run at `effort`, with the default seed and no measuring option.
fn plan(effort: u32) -> Plan {
    Plan { effort, seed: 1, stop_at_known: false, from_known: false }
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
