//! Node-count tripwires: the search tree of a fixed game under fixed tactics, counted exactly.
//!
//! Every search tactic is a pruning device: switching one off, or breaking it so that it prunes nothing, changes no
//! answer, so no assertion on answers sees it (a mutant that ignored the result of `Nogoods::ruled_out` costs
//! 27 to 67 % more nodes and passed every other test).  With one thread the tree is deterministic, so its size is
//! a fingerprint: a tactic that silently stops working makes a count larger, and one that starts pruning something
//! it should not (or a bound that gets weaker, or a changed order) moves it too.
//!
//! The counts are those of the code at the time of writing.  A change that alters the tree deliberately (a new
//! bound, another order, a different incumbent to start from, or the frame builder)
//! fails these tests; that is what they are for.  To update them, run
//!
//! ```text
//! cargo test --release --test tripwires -- --nocapture
//! ```
//!
//! which prints, on failure, the rows of the failing boards as observed, in the layout of [`EXPECTED`].  (Release,
//! because a non-release run checks only the rows of the trees of [`QUICK_NODES`] nodes or fewer, in the quick tests.)
//! Check that the changes are the intended ones (the promotion sum, which is the answer, never moves), paste the rows
//! in, and change the node counts quoted in solver/docs/design/ and in the notebook if they are affected.  The same
//! goes for [`GATED`], the trees under the gates that make conflict branching act, and for [`FORGETTING`].
//! [`NEAR_REPEATS`] counts the trees of games that the near-repeat cut shortens, solved as the solver solves them, with
//! the cut and without it; its test prints its rows as observed.

mod common;

use common::{options, prev_of, search_at_width, stored_games, Config, Game};
use std::sync::{Arc, OnceLock};
use taxman::board::Board;
use taxman::bounds::greedy;
use taxman::json::Json;
use taxman::search::{search_steered, BranchSettings, Branching, SearchTactics};
use taxman::solver::{self, Method};
use taxman::tactics::Tactics;
use taxman::with_width;

/// (board, tactics, promotion sum, nodes): the game searched from the frame builder on one thread.  The tactics are
/// `--tactic` assignments after the default.
const EXPECTED: [(u32, &str, u64, u64); 20] = [
    // The default, the GF(2) bound with the rule of the root's symmetric pairs, the reuse of prices and the warm start:
    // trees of tens to thousands of nodes; `symmetry=off` (the default before the rule) makes them larger with the same
    // answers.  Conflict branching is on by default and acts where a node's gap passes the gate, 300.  The trees of
    // 1000 are those it makes (those of [`GATED`]'s 1000 under 300, as the default's); the others have the node counts
    // that they have under the gate 1,200, so they are the search's own conflict at every node.  [`GATED`] has the
    // trees of games under other gates that make conflict branching act.
    (480, "default", 5350, 37),
    (600, "default", 8507, 365),
    (600, "fix_forced=off", 8507, 530),
    (600, "nogoods=off", 8507, 372),
    (600, "symmetry=off", 8507, 533),
    (640, "default", 9220, 15),
    (660, "default", 10242, 522),
    (660, "fix_forced=off", 10242, 908),
    (660, "nogoods=off", 10242, 615),
    (760, "default", 14170, 180),
    (936, "default", 20507, 449),
    (936, "fix_forced=off", 20507, 830),
    (936, "nogoods=off", 20507, 345),
    (936, "symmetry=off", 20507, 652),
    (1000, "default", 24168, 2841),
    (1000, "fix_forced=off", 24168, 4000),
    (1000, "nogoods=off", 24168, 5361),
    (1000, "symmetry=off", 24168, 3801),
    (1133, "default", 28690, 41),
    (1133, "nogoods=off", 28690, 34),
];

/// (board, tactics, promotion sum, nodes): the game searched from the frame builder on one thread, as [`EXPECTED`]'s,
/// with a gate named among the assignments, `gate=N`: the gap from which conflict branching chooses.  [`EXPECTED`]'s
/// rows have the default gate, 300, which changes the tree of only one of their games (1000), so these rows hold the
/// trees of the tactics that act only where conflict branching chooses (`conflict_look`, `price_reuse`, and the
/// conflicts themselves), and those of a tactic that changes a tree by a node (`warm_start`: the games 775 to 777 take
/// one node fewer without it).  The games 760 and 1133 under the gate 0, which chooses at every check; 775 under the
/// gate 1,200, which no node of it reaches; and 1000 under 300, the default gate, whose first row is [`EXPECTED`]'s
/// default and whose others have the reuse of prices and the warm start off in turn.
const GATED: [(u32, &str, u64, u64); 10] = [
    (760, "gate=0", 14170, 159),
    (760, "gate=0,conflict_look=off", 14170, 157),
    (760, "gate=0,price_reuse=off", 14170, 159),
    (1133, "gate=0", 28690, 29),
    (1133, "gate=0,price_reuse=off", 28690, 27),
    (775, "gate=1200", 14548, 210),
    (775, "gate=1200,warm_start=off", 14548, 209),
    (1000, "gate=300", 24168, 2841),
    (1000, "gate=300,price_reuse=off", 24168, 3319),
    (1000, "gate=300,price_reuse=off,warm_start=off", 24168, 3319),
];

/// The boards of [`GATED`].
const GATED_BOARDS: [u32; 4] = [760, 775, 1000, 1133];

/// The tactics a row names: `default` as in the tables, the others `--tactic` assignments after the default, and
/// `gate=N`, a gate of [`GATED`], none.
fn tactics(assignments: &str) -> SearchTactics {
    let mut tactics = Tactics::default();
    for assignment in assignments.split(',') {
        match assignment {
            "default" => {}
            _ if assignment.starts_with("gate=") => {}
            _ => tactics.set(assignment).unwrap(),
        }
    }
    tactics.search
}

/// The most nodes a row has for the quick tests to check it: the trees of 250 nodes or fewer take a few milliseconds
/// each.
const QUICK_NODES: u64 = 250;

/// Checks the rows of `table` ([`EXPECTED`] or [`GATED`]) for the boards `ns` whose trees have `most_nodes` nodes or
/// fewer against `tree`, the function that computes the (promotion sum, nodes) of a row's board and tactics; `what`
/// names the trees in the message of a difference, and on one the rows of those boards are printed as observed.
fn check_trees(
    table: &[(u32, &str, u64, u64)],
    ns: &[u32],
    most_nodes: u64,
    what: &str,
    tree: fn(u32, &str) -> (u64, u64),
) {
    let mut observed = Vec::new();
    let mut changed = 0;
    for &(n, assignments, sum, nodes) in table.iter().filter(|row| ns.contains(&row.0) && row.3 <= most_nodes) {
        let actual = tree(n, assignments);
        observed.push(format!("    ({n}, {assignments:?}, {}, {}),", actual.0, actual.1));
        if actual != (sum, nodes) {
            changed += 1;
            println!("n={n} {assignments}: (sum, nodes) {actual:?}, expected {:?}", (sum, nodes));
        }
    }
    assert!(
        changed == 0,
        "{changed} {what} changed (see the top of tests/tripwires.rs for what to do about it); observed:\n{}",
        observed.join("\n")
    );
}

/// The tree [`EXPECTED`] counts: (promotion sum, nodes) of the default engine's search of game `n` from the frame
/// builder under the tactics `assignments`, on one thread.
fn default_tree(n: u32, assignments: &str) -> (u64, u64) {
    with_width!(n, W => search_at_width::<W>(n, tactics(assignments)))
}

/// The tree [`GATED`] counts: [`default_tree`] with the gate `assignments` name (`gate=N`).
fn gated_tree(n: u32, assignments: &str) -> (u64, u64) {
    let gate = assignments.split(',').find_map(|a| a.strip_prefix("gate=")).expect("a row of GATED names its gate");
    let gate = gate.parse().expect("a gate is a number");
    with_width!(n, W => {
        let p = Board::<W>::for_n(n).prepare(tactics(assignments)).unwrap();
        let branching = Arc::new(Branching::new(BranchSettings::of(&p.tactics).with_gate(Some(gate))));
        let near = OnceLock::new();
        let cfg = common::config(1);
        let r = search_steered(&p, &cfg, None, None, false, None, &branching, &near, None, None, None).unwrap();
        assert!(r.completed);
        (r.sum, r.stats.nodes)
    })
}

/// The boards of [`EXPECTED`] in three tests, so that the test threads share the work; every board is in one.
const GROUPS: [&[u32]; 3] = [&[480, 600, 640], &[660, 760, 1000], &[936, 1133]];

/// Every board, for the quick tests that check the small trees of all of them.
const ALL_BOARDS: [u32; 8] = [480, 600, 640, 660, 760, 936, 1000, 1133];

#[test]
fn the_small_search_trees_have_not_changed() {
    check_trees(&EXPECTED, &ALL_BOARDS, QUICK_NODES, "search trees", default_tree);
}

#[test]
fn the_small_search_trees_under_a_gate_that_chooses_have_not_changed() {
    check_trees(&GATED, &GATED_BOARDS, QUICK_NODES, "search trees under a gate", gated_tree);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is the_small_search_trees_under_a_gate_that_chooses_have_not_changed"
)]
fn the_search_trees_under_a_gate_that_chooses_have_not_changed() {
    check_trees(&GATED, &GATED_BOARDS, u64::MAX, "search trees under a gate", gated_tree);
}

#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is the_small_search_trees_have_not_changed")]
fn the_search_trees_of_480_600_and_640_have_not_changed() {
    check_trees(&EXPECTED, GROUPS[0], u64::MAX, "search trees", default_tree);
}

#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is the_small_search_trees_have_not_changed")]
fn the_search_trees_of_660_760_and_1000_have_not_changed() {
    check_trees(&EXPECTED, GROUPS[1], u64::MAX, "search trees", default_tree);
}

#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is the_small_search_trees_have_not_changed")]
fn the_search_trees_of_936_and_1133_have_not_changed() {
    check_trees(&EXPECTED, GROUPS[2], u64::MAX, "search trees", default_tree);
}

#[test]
fn every_board_of_the_table_is_checked() {
    let mut covered: Vec<u32> = GROUPS.concat();
    covered.sort_unstable();
    let mut boards: Vec<u32> = EXPECTED.iter().map(|row| row.0).collect();
    boards.sort_unstable();
    boards.dedup();
    assert_eq!(boards, covered, "the boards of EXPECTED and of GROUPS");
}

/// solver/docs/design/search.md: deterministic on one thread (nogoods, forgetting and all); nothing else asserts it for
/// the search.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is a_search_of_a_small_game_on_one_thread_is_deterministic"
)]
fn a_search_on_one_thread_is_deterministic() {
    let without_rule = SearchTactics { symmetry: false, ..SearchTactics::default() };
    for (n, t) in [(1000, SearchTactics::default()), (936, without_rule)] {
        assert_eq!(search_at_width::<16>(n, t), search_at_width::<16>(n, t), "n={n}");
    }
}

/// [`a_search_on_one_thread_is_deterministic`] for the games 660 (by the default) and 600 (without the rule of the
/// symmetric pairs).
#[test]
fn a_search_of_a_small_game_on_one_thread_is_deterministic() {
    let without_rule = SearchTactics { symmetry: false, ..SearchTactics::default() };
    for (n, t) in [(660, SearchTactics::default()), (600, without_rule)] {
        assert_eq!(search_at_width::<16>(n, t), search_at_width::<16>(n, t), "n={n}");
    }
}

/// The rule of the tactic `symmetry` (the default has it) is a function of a node's promotions and candidates, so a
/// search with it on one thread is as deterministic as without: the same tree, and the same set at the end.
#[test]
fn a_search_with_the_symmetry_rule_on_one_thread_is_deterministic() {
    let game = Game::<16>::of_board(1000).unwrap();
    assert!(!game.symmetries().is_empty(), "1000 has symmetric pairs, which the rule works with");
    let cfg = Config::standard();
    assert!(cfg.symmetry, "the default has the rule");
    let start = greedy(&game.model, &game.root, &game.candidates).sum();
    let run = || game.search(cfg, vec![game.root_unit()], start, None, None);
    let (first, again) = (run(), run());
    assert!(first.completed && first.real);
    assert_eq!((first.nodes, first.best, first.set), (again.nodes, again.best, again.set));
}

/// (game, nodes forgetting the nogoods every 200 steps, nodes never forgetting) for [`forgetting_search`].
const FORGETTING: (u32, u64, u64) = (800, 1640, 1424);

/// The tree below the root of a game of more than a thousand nodes (the trees of the smaller games are too small for
/// forgetting to matter), searched by one searcher that forgets its nogoods every `period` steps (`None`: never), from
/// the greedy incumbent; returns (best, nodes).  A search forgets for the first time after 50,000 steps
/// (`NOGOOD_PERIOD`), so the games that finish sooner never meet what a long search meets all the time: conflicts that
/// the table no longer holds, that only the propagation of the node's known conflicts into its children
/// (`Stepper::make_child` in src/search/searcher.rs) still rules out.  Measured on the matching path, the node bound
/// before the GF(2) bound, with a mutant that switched that propagation off: no count changed below 50,000 steps (600,
/// 760, 936, 1133, 1140, 1150), but forgetting every 500 steps cost it 0.9 % at n = 600, and n = 992 (4.2M nodes) 22 %.
fn forgetting_search(game: &Game<16>, period: Option<u64>) -> (u64, u64) {
    let cfg = Config { period, ..Config::standard() };
    let start = greedy(&game.model, &game.root, &game.candidates).sum();
    let out = game.search(cfg, vec![game.root_unit()], start, None, None);
    assert!(out.completed && out.real);
    (out.best, out.nodes)
}

#[test]
fn the_tree_with_frequent_forgetting_has_not_changed() {
    let (n, expected_forgetting, expected_never) = FORGETTING;
    let game = Game::<16>::of_board(n).unwrap();
    let (best, forgetting) = forgetting_search(&game, Some(200));
    let (best_never, never) = forgetting_search(&game, None);
    assert_eq!(best, best_never);
    assert_eq!(
        (forgetting, never),
        (expected_forgetting, expected_never),
        "n={n}: nodes forgetting every 200 steps, nodes never forgetting; update FORGETTING"
    );
}

/// (game, nodes with the near-repeat cut, nodes without it): the game solved by the solver on one thread from the
/// committed chain's moves for n − 1, which is the cut's game proven before (tactic `near_repeat`).  The cut ends the
/// search of 1101 at its first step, and shortens those of 768 and 1083.  The line of each game counts the nodes the
/// cut cut.
const NEAR_REPEATS: [(u32, u64, u64); 3] = [(768, 62, 85), (1083, 18, 70), (1101, 1, 548)];

#[test]
fn the_trees_that_the_near_repeat_cut_shortens_have_not_changed() {
    let games = stored_games();
    let mut observed = Vec::new();
    for (n, ..) in NEAR_REPEATS {
        let prev = prev_of(&games, n);
        let [with, without] = [true, false].map(|near_repeat| {
            let mut opts = options(1);
            opts.tactics.search.near_repeat = near_repeat;
            let r = solver::solve_game(n, Some(&prev), &opts).unwrap();
            assert_eq!((r.method, r.score), (Method::Search, games[&n].0), "n={n}, near_repeat {near_repeat}");
            // Its line counts the nodes the cut cut (`stats.near_repeat_cut`) if it was armed, and has no such key if
            // it was not.
            let line = r.to_json();
            let counted = line.get("stats").and_then(|stats| stats.get("near_repeat_cut")).and_then(Json::as_u64);
            assert_eq!(counted, r.near_repeat_cut, "n={n}, near_repeat {near_repeat}");
            assert_eq!(counted.is_some_and(|nodes| nodes > 0), near_repeat, "n={n}, near_repeat {near_repeat}");
            r.nodes
        });
        observed.push((n, with, without));
    }
    assert_eq!(observed, NEAR_REPEATS, "the trees changed (see the top of tests/tripwires.rs); observed: {observed:?}");
}
