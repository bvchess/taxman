//! The sweep's tools (`solver/tools/cloud/sweep.sh`, run by `solver/tools/cloud/test/run.sh sweep` against a stand-in
//! for aws). The planner that sizes the blocks and cuts the excerpts of the passes (`solver/tools/sweep_plan.py`,
//! standard library only) has tests of its own, and the excerpt it writes is what `taxman solve --keep-going
//! --retry-improved` opens: the solver checks the file it opens, so the excerpt must hold whatever each of its lines
//! rests on.

mod common;

use common::{matching_options, python, TempDir};
use std::fs;
use std::path::{Path, PathBuf};
use taxman::json::Json;
use taxman::solver::{self, SolveOptions};

fn tool(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../tools").join(name)
}

#[test]
fn the_planner_passes_its_own_tests() {
    let run = python(&tool("test_sweep_plan.py"), &[]);
    assert_eq!(run.code, Some(0), "{}{}", run.stdout, run.stderr);
}

/// An excerpt of a merged file, for the games of the second block, opens as the `--out` file of a job over that block:
/// the solver's checks of it pass, it takes the game before the block as its seed, and its games it searches again
/// are the ones the planner counted.
#[test]
fn an_excerpt_opens_as_the_file_of_a_job_and_the_job_does_what_the_planner_counted() {
    let dir = TempDir::new("sweep-excerpt");
    // Two blocks made apart, as pass 1 makes them: 598..599 finished, and 600..602 stopped at the limit, the first of
    // them with no game before it; and a third block's lines far away, which the excerpt leaves out.
    let (first, second, far) = (dir.join("first.jsonl"), dir.join("second.jsonl"), dir.join("far.jsonl"));
    let finish = SolveOptions { keep_going: true, ..matching_options(1) };
    solver::solve_range(598, 599, &finish, Some(&first)).unwrap();
    let stop = SolveOptions { node_limit: Some(200), force_search: true, ..finish.clone() };
    solver::solve_range(600, 602, &stop, Some(&second)).unwrap();
    solver::solve_range(901, 902, &finish, Some(&far)).unwrap();
    let merged = dir.join("merged.jsonl");
    let text: String = [&first, &far, &second].iter().map(|path| fs::read_to_string(path).unwrap()).collect();
    fs::write(&merged, text).unwrap();

    let job = dir.join("job.jsonl");
    let run = python(&tool("sweep_plan.py"), &["excerpt", common::path(&merged), "600", "602", common::path(&job)]);
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let counts: Vec<u32> = run.stdout.split_whitespace().map(|word| word.parse().unwrap()).collect();
    let lines = |path: &Path| -> Vec<Json> {
        fs::read_to_string(path).unwrap().lines().map(|line| Json::parse(line).unwrap()).collect()
    };
    let boards: Vec<u32> = lines(&job).iter().map(|line| line.get("n").and_then(Json::as_u32).unwrap()).collect();
    assert_eq!(
        boards,
        [598, 599, 600, 601, 602],
        "the games before the block that a repeat could repeat, and the block"
    );
    // The planner counted the games the solver searches first again: 600 had no game before it, and now has 599.
    assert_eq!(counts, [5, 1]);

    let improving = SolveOptions { node_limit: None, force_search: false, retry_improved: true, ..finish };
    let mut searched = Vec::new();
    let summary = solver::solve_range_each(600, 602, &improving, Some(&job), |game| searched.push(game.n)).unwrap();
    assert_eq!(summary.solved, 3, "600 is searched again with 599, and 601 and 602 with the game just proven");
    assert_eq!(searched, [600, 601, 602]);
    assert_eq!(lines(&job).len(), 5 + 3);
}
