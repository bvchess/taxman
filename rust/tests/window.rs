//! The window (`SolveOptions::window`, `--window`): the games of a range in flight at once.  A chain solved with any
//! window has the scores, methods, proofs, repeats, assumptions and previous games of the committed chain; with one
//! thread the window is one, and the lines are those of a window of one but for the times and the build; a repeat of
//! a game still in flight names it, on a later line; a range stopped by a limit stops at its first unfinished game with
//! the lines before it in order, keeps the checkpoints of the games searched ahead of it and completes when run again;
//! a game that passes the wall (`SolveOptions::game_limit`) ends the range with no line for it, its checkpoint and its
//! clock kept, and goes on from them when run again; and a score that is not the table's fails the range at that game,
//! with the lines before it, as a window of one does.  The policy that shares the threads is tested on its own in
//! `src/solver/window/allot.rs`.  Small boards, at most 4 threads.

#[macro_use]
mod common;

use common::{matching_options, options, results_path, TempDir};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::results::{rejected_path, verify_files};
use taxman::solver::{self, SolveOptions};

/// The lines of a results file.
fn lines(path: &Path) -> Vec<Json> {
    fs::read_to_string(path).unwrap().lines().map(|line| Json::parse(line).unwrap()).collect()
}

/// A line's board size.
fn n_of(line: &Json) -> u32 {
    line.get("n").and_then(Json::as_u32).expect("n")
}

/// What a line says that no window may change: the game's score, how it was found and proven, the ceiling its search
/// had, what it assumes, the game a repeat repeats, and the previous game's board size, score and proof.  (The moves
/// may be another optimal set, and with them the digest of the previous game's moves, as between thread counts.)
fn provenance(line: &Json) -> String {
    let field = |key: &str| line.get(key).map_or_else(String::new, ToString::to_string);
    let inner =
        |key: &str, of: &str| line.get(key).and_then(|o| o.get(of)).map_or_else(String::new, ToString::to_string);
    let fields = [
        field("n"),
        field("score"),
        field("method"),
        field("proof"),
        field("reached_cap"),
        inner("setup", "cap"),
        field("assumes"),
        inner("same_search", "n"),
        inner("prev", "n"),
        inner("prev", "score"),
        inner("prev", "proof"),
    ];
    fields.join(" ")
}

/// A line without the fields that differ from run to run whatever the window: the times and the build.
fn without_times(line: &Json) -> Json {
    let fields = line.as_obj().expect("an object").iter().filter(|(key, _)| key != "seconds" && key != "build");
    Json::object(fields.cloned())
}

/// The committed chain's games of `1..=hi`, by n, as [`provenance`] has them.
fn committed(hi: u32) -> BTreeMap<u32, String> {
    let text = fs::read_to_string(results_path("strict.jsonl")).unwrap();
    let chain = text.lines().map(|line| Json::parse(line).unwrap());
    chain.filter(|line| n_of(line) <= hi).map(|line| (n_of(&line), provenance(&line))).collect()
}

#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_short_chain_has_the_proofs_of_the_committed_chain_with_any_window))]
fn a_chain_has_the_proofs_of_the_committed_chain_with_any_window() {
    a_chain_has_the_proofs_of_the_committed_chain_with_any_window_to(600);
}

/// [`a_chain_has_the_proofs_of_the_committed_chain_with_any_window`] for the games 1..=150.
#[test]
fn a_short_chain_has_the_proofs_of_the_committed_chain_with_any_window() {
    a_chain_has_the_proofs_of_the_committed_chain_with_any_window_to(150);
}

/// Chains 1..=`hi` with windows of 1, 2, 4 and 8 on 4 threads have the committed chain's provenance and pass the
/// verifier.
fn a_chain_has_the_proofs_of_the_committed_chain_with_any_window_to(hi: u32) {
    let expected = committed(hi);
    assert_eq!(expected.len(), hi as usize);
    for window in [1, 2, 4, 8] {
        let opts = SolveOptions { window, ..options(4) };
        let games = solver::solve_range(1, hi, &opts, None).unwrap_or_else(|e| panic!("window {window}: {e}"));
        // Every game once, in order, with the committed chain's provenance.
        let chain: Vec<Json> = games.iter().map(|game| game.to_json()).collect();
        assert_eq!(chain.iter().map(n_of).collect::<Vec<_>>(), (1..=hi).collect::<Vec<_>>(), "window {window}");
        for line in &chain {
            assert_eq!(provenance(line), expected[&n_of(line)], "window {window}");
        }
        // The lines as a results file has them (written at once: a run syncs each line as it appends it, which is
        // what takes the time here), and the verifier accepts them.
        let dir = TempDir::new(&format!("window-{window}"));
        let path = dir.join("chain.jsonl");
        fs::write(&path, chain.iter().map(|line| format!("{line}\n")).collect::<String>()).unwrap();
        verify_files(std::slice::from_ref(&path), Optimal::built_in(), 4)
            .unwrap_or_else(|e| panic!("window {window}: {e}"));
    }
}

#[test]
#[cfg_attr(debug_assertions, ignore = long!(with_one_thread_any_window_is_a_window_of_one_for_a_short_chain))]
fn with_one_thread_any_window_is_a_window_of_one() {
    with_one_thread_any_window_is_a_window_of_one_to(600);
}

/// [`with_one_thread_any_window_is_a_window_of_one`] for the games 1..=150.
#[test]
fn with_one_thread_any_window_is_a_window_of_one_for_a_short_chain() {
    with_one_thread_any_window_is_a_window_of_one_to(150);
}

/// With one thread the lines of a chain 1..=`hi` are those of a window of one, whatever the window.
fn with_one_thread_any_window_is_a_window_of_one_to(hi: u32) {
    let chain = |window: usize| {
        let games = solver::solve_range(1, hi, &SolveOptions { window, ..options(1) }, None).unwrap();
        games.iter().map(|game| without_times(&game.to_json())).collect::<Vec<_>>()
    };
    let (one, eight) = (chain(1), chain(8));
    assert_eq!(one.len(), hi as usize);
    for (a, b) in one.iter().zip(&eight) {
        assert_eq!(a, b);
    }
}

/// 301 repeats the search of 300, as the committed chain has it (and 302 repeats 301, 303 repeats 302): with a window
/// of 2 or more, 301 is set up and looked up while 300 is in flight, and it still names 300, on the line after 300's.
#[test]
fn a_game_repeats_the_search_of_a_game_still_in_flight() {
    for window in [2, 4] {
        let dir = TempDir::new(&format!("in-flight-{window}"));
        let path = dir.join("r.jsonl");
        let opts = SolveOptions { window, use_prev: true, ..options(4) };
        solver::solve_range(300, 303, &opts, Some(&path)).unwrap();
        let chain = lines(&path);
        let repeats: Vec<(u32, Option<u32>)> = chain
            .iter()
            .map(|line| (n_of(line), line.get("same_search").and_then(|same| same.get("n")).and_then(Json::as_u32)))
            .collect();
        assert_eq!(repeats, [(300, None), (301, Some(300)), (302, Some(301)), (303, Some(302))], "window {window}");
        verify_files(std::slice::from_ref(&path), Optimal::built_in(), 1).unwrap();
    }
}

/// A node limit stops the range at its first game whose search it stops, after the lines of the games before it, in
/// order.  The games searched ahead of it keep the checkpoints their searches left, so that the next run resumes them
/// instead of starting them over; run again and again, the same command completes the range.  The searches take no
/// seed and no ceiling, and every game is searched, so that the games ahead start at once and run long enough for the
/// limit (the engine checks it every 1,024 steps of a worker).
#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_short_range_stopped_by_a_limit_ends_at_its_first_unfinished_game_and_completes_when_run_again))]
fn a_range_stopped_by_a_limit_ends_at_its_first_unfinished_game_and_completes_when_run_again() {
    a_range_stopped_by_a_limit_completes_when_run_again(598, 612);
}

/// [`a_range_stopped_by_a_limit_ends_at_its_first_unfinished_game_and_completes_when_run_again`] for the games
/// 599..=601 (600 takes more than the 1,024 steps that the limit is looked at after).
#[test]
fn a_short_range_stopped_by_a_limit_ends_at_its_first_unfinished_game_and_completes_when_run_again() {
    a_range_stopped_by_a_limit_completes_when_run_again(599, 601);
}

/// The range `lo..=hi` under a node limit stops at its first unfinished game, and completes when run again.
fn a_range_stopped_by_a_limit_completes_when_run_again(lo: u32, hi: u32) {
    let dir = TempDir::new("window-limit");
    let results = dir.join("r.jsonl");
    let checkpoints = dir.join("ckpt");
    let mut limited = SolveOptions {
        checkpoint_dir: Some(checkpoints.clone()),
        node_limit: Some(200),
        force_search: true,
        use_prev: true,
        window: 4,
        ..matching_options(4)
    };
    limited.tactics.seed_incumbent = false;
    limited.tactics.prev_cap = false;
    let mut seen = Vec::new();
    let summary =
        solver::solve_range_each(lo, hi, &limited, Some(&results), |game| seen.push((game.n, game.completed))).unwrap();
    let stopped = summary.stopped_at.expect("the node limit stops a search of the range");
    let expected: Vec<(u32, bool)> = (lo..stopped).map(|n| (n, true)).chain([(stopped, false)]).collect();
    assert_eq!(seen, expected);
    assert_eq!(lines(&results).iter().map(n_of).collect::<Vec<_>>(), (lo..=stopped).collect::<Vec<_>>());
    let left = |dir: &Path| -> Vec<u32> {
        let names = fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned());
        let mut ns: Vec<u32> =
            names.filter_map(|name| name.strip_prefix('n')?.strip_suffix(".ckpt")?.parse().ok()).collect();
        ns.sort_unstable();
        ns
    };
    let ahead = left(&checkpoints);
    assert_eq!(ahead.first(), Some(&stopped), "{ahead:?}");
    assert!(ahead.iter().all(|&n| n < stopped + 4), "only games of the window searched: {ahead:?}");

    let mut runs = 1;
    while solver::solve_range_each(lo, hi, &limited, Some(&results), |_| {}).unwrap().stopped_at.is_some() {
        runs += 1;
        assert!(runs < 100, "no progress");
    }
    let chain = lines(&results);
    verify_files(std::slice::from_ref(&results), Optimal::built_in(), 1).unwrap_or_else(|e| panic!("{e}"));
    // Every game is completed once; the games whose checkpoints were left were resumed, not started over.
    let completed: Vec<&Json> =
        chain.iter().filter(|line| line.get("completed").and_then(Json::as_bool) == Some(true)).collect();
    assert_eq!(completed.iter().map(|line| n_of(line)).collect::<Vec<_>>(), (lo..=hi).collect::<Vec<_>>());
    for line in completed.iter().filter(|line| ahead.contains(&n_of(line))) {
        let invocations = line.get("invocations").and_then(Json::as_u32);
        assert!(invocations >= Some(2), "n={}: {invocations:?}", n_of(line));
    }
    assert!(left(&checkpoints).is_empty() && !checkpoints.join("spill").exists());
}

/// The seconds in the clock file of game `n`.
fn clock_of(checkpoints: &Path, n: u32) -> f64 {
    let text = fs::read_to_string(checkpoints.join(format!("n{n:04}.clock"))).unwrap_or_else(|e| panic!("n={n}: {e}"));
    text.trim().parse().unwrap_or_else(|e| panic!("n={n}: {text:?}: {e}"))
}

/// The wall: the head of the window, whose search runs about half a second on 4 threads (no seed, no ceiling, the
/// matching tactics), passes a limit of 3 ms.  The range ends there with no line for it, nothing in the results file, a
/// checkpoint and a clock for it; the games searched ahead of it keep their checkpoints; a run with the same limit
/// walls again at the same game at once, from the clock it was left with (which is never smaller); and a run with no
/// limit completes every game and leaves no clock behind.
#[test]
#[cfg_attr(debug_assertions, ignore = long!(a_lone_game_that_passes_the_wall_ends_the_range_with_no_line_and_goes_on_from_its_clock))]
fn a_game_that_passes_the_wall_ends_the_range_with_no_line_and_goes_on_from_its_clock() {
    a_game_that_passes_the_wall_goes_on_from_its_clock(800, 801);
}

/// [`a_game_that_passes_the_wall_ends_the_range_with_no_line_and_goes_on_from_its_clock`] for the game 800 alone.
#[test]
fn a_lone_game_that_passes_the_wall_ends_the_range_with_no_line_and_goes_on_from_its_clock() {
    a_game_that_passes_the_wall_goes_on_from_its_clock(800, 800);
}

/// The head `lo` of the range `lo..=hi` passes a wall of 3 ms, and the range goes on from its clock.
fn a_game_that_passes_the_wall_goes_on_from_its_clock(lo: u32, hi: u32) {
    let dir = TempDir::new("window-wall");
    let results = dir.join("r.jsonl");
    let checkpoints = dir.join("ckpt");
    let mut walled = SolveOptions {
        checkpoint_dir: Some(checkpoints.clone()),
        game_limit: Some(Duration::from_millis(3)),
        force_search: true,
        use_prev: true,
        window: 4,
        ..matching_options(4)
    };
    walled.tactics.seed_incumbent = false;
    walled.tactics.prev_cap = false;
    let mut seen = Vec::new();
    let summary = solver::solve_range_each(lo, hi, &walled, Some(&results), |game| seen.push(game.n)).unwrap();
    let wall = summary.walled.expect("the head's search takes longer than the limit");
    assert_eq!(wall.n, lo, "the first game of the range is the head");
    assert!(wall.held >= Duration::from_millis(3), "{wall:?}");
    assert!(summary.stopped_at.is_none() && summary.solved == 0 && seen.is_empty(), "{summary:?} {seen:?}");
    assert_eq!(fs::read_to_string(&results).unwrap(), "", "nothing is recorded for the game, or any other");
    assert!(checkpoints.join(format!("n{lo:04}.ckpt")).exists());
    let kept = clock_of(&checkpoints, lo);
    assert!(kept >= 0.0029 && kept <= wall.held.as_secs_f64() + 0.001, "{kept} {wall:?}");

    // The same limit again: the clock is where it was left, so the game walls at once, with a clock no smaller.
    let again = solver::solve_range_each(lo, hi, &walled, Some(&results), |_| {}).unwrap();
    let wall_again = again.walled.expect("a clock at the limit walls the game again");
    assert_eq!(wall_again.n, lo);
    assert!(wall_again.held >= wall.held && clock_of(&checkpoints, lo) >= kept);

    // No limit: the game resumes from its checkpoint and the range completes; no clock is left.
    let free = SolveOptions { game_limit: None, ..walled };
    let done = solver::solve_range_each(lo, hi, &free, Some(&results), |game| seen.push(game.n)).unwrap();
    assert!(done.walled.is_none() && done.stopped_at.is_none(), "{done:?}");
    assert_eq!(seen, (lo..=hi).collect::<Vec<_>>());
    assert_eq!(lines(&results).iter().map(n_of).collect::<Vec<_>>(), (lo..=hi).collect::<Vec<_>>());
    verify_files(std::slice::from_ref(&results), Optimal::built_in(), 1).unwrap_or_else(|e| panic!("{e}"));
    let left: Vec<String> = fs::read_dir(&checkpoints)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name != "taxman.lock")
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

/// A table that is wrong about one game fails a range at that game, with the lines of the games before it and the game
/// in the rejected file, whatever the window.
#[test]
fn a_score_that_is_not_the_tables_fails_the_range_at_that_game_with_any_window() {
    let wrong_at = 300;
    let games = Optimal::built_in().games().map(|(n, score, moves)| {
        let score = if n == wrong_at { score + 1 } else { score };
        Json::object([
            ("n", Json::from(n)),
            ("score", Json::from(score)),
            ("moves", Json::array(moves.iter().copied())),
        ])
    });
    let wrong = Optimal::from_json(&Json::array(games)).unwrap();
    let mut outcomes = Vec::new();
    for window in [1, 4] {
        let dir = TempDir::new(&format!("window-wrong-{window}"));
        let path = dir.join("r.jsonl");
        let opts = SolveOptions { window, use_prev: true, ..options(4) };
        let err = solver::solve_range_each_with(280, 320, &opts, Some(&path), &wrong, |_| {}).unwrap_err();
        assert!(err.starts_with(&format!("n={wrong_at}: scored ")) && err.contains("r.rejected.jsonl"), "{err}");
        let recorded: Vec<u32> = lines(&path).iter().map(n_of).collect();
        assert_eq!(recorded, (280..wrong_at).collect::<Vec<_>>(), "window {window}");
        let rejected: Vec<u32> = lines(&rejected_path(&path)).iter().map(n_of).collect();
        assert_eq!(rejected, [wrong_at], "window {window}");
        outcomes.push(err.replace(dir.to_str().unwrap(), "DIR"));
    }
    assert_eq!(outcomes[0], outcomes[1]);
}
