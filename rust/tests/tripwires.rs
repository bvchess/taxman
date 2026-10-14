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
//! cargo test --test tripwires -- --nocapture
//! ```
//!
//! which prints, on failure, the rows of the failing boards as observed, in the layout of [`EXPECTED`].  Check that
//! the changes are the intended ones (the promotion sum, which is the answer, never moves), paste the rows in, and
//! change the node counts quoted in DESIGN.md and docs/EXPERIMENTS.md if they are affected.  The same goes for
//! [`FORGETTING`].

mod common;

use common::{search_at_width, Config, Game};
use taxman::bounds::greedy;
use taxman::search::SearchTactics;
use taxman::tactics::Tactics;
use taxman::with_width;

/// (board, tactics, promotion sum, nodes): the game searched from the frame builder on one thread.  The tactics are
/// `--tactic` assignments after the default or `matching`, the tactics of the matching path
/// ([`SearchTactics::matching`], the default before the GF(2) bound).  The Lagrangian bound (tactic `lagrange`) first
/// fits its multipliers after 2,000 steps, so the games that take fewer (480, 640) do not see it; the matching path's
/// rows `lagrange=off` are the trees of the build before it.
const EXPECTED: [(u32, &str, u64, u64); 49] = [
    // The default, the GF(2) bound: trees of tens to thousands of nodes; `lagrange=on` (which refits after 2,000
    // steps) changes none of them.
    (480, "default", 5350, 37),
    (600, "default", 8507, 533),
    (600, "fix_forced=off", 8507, 716),
    (600, "nogoods=off", 8507, 669),
    (640, "default", 9220, 21),
    (660, "default", 10242, 573),
    (660, "fix_forced=off", 10242, 1009),
    (660, "nogoods=off", 10242, 737),
    (760, "default", 14170, 186),
    (936, "default", 20507, 665),
    (936, "fix_forced=off", 20507, 1685),
    (936, "nogoods=off", 20507, 751),
    (936, "lagrange=on", 20507, 665),
    (1000, "default", 24168, 2657),
    (1000, "fix_forced=off", 24168, 4427),
    (1000, "nogoods=off", 24168, 4807),
    (1000, "lagrange=on", 24168, 2657),
    (1133, "default", 28690, 41),
    (1133, "nogoods=off", 28690, 37),
    // The matching path.  W = 8, the small end: every tactic's effect shows in a few hundred nodes.
    (480, "matching", 5350, 393),
    (480, "matching,nogoods=off", 5350, 417),
    (480, "matching,fix_forced=off", 5350, 503),
    (480, "matching,match_refine=0", 5350, 431),
    (480, "matching,match_refine=3", 5350, 254),
    // W = 16: games of a few hundred to ten thousand nodes.
    (600, "matching", 8507, 4558),
    (600, "matching,lagrange=off", 8507, 5877),
    (600, "matching,nogoods=off", 8507, 17740),
    (600, "matching,fix_forced=off", 8507, 7723),
    (600, "matching,match_refine=0", 8507, 4711),
    (600, "matching,match_refine=3", 8507, 3469),
    (640, "matching", 9220, 676),
    (640, "matching,nogoods=off", 9220, 1099),
    (640, "matching,fix_forced=off", 9220, 1569),
    (660, "matching", 10242, 8822),
    (660, "matching,lagrange=off", 10242, 10043),
    (660, "matching,nogoods=off", 10242, 14817),
    (660, "matching,fix_forced=off", 10242, 14657),
    (760, "matching", 14170, 2318),
    (760, "matching,lagrange=off", 14170, 2567),
    (760, "matching,nogoods=off", 14170, 3361),
    (760, "matching,fix_forced=off", 14170, 4317),
    (760, "matching,match_refine=3", 14170, 1643),
    // W = 16, and without the Lagrangian bound a search of more than 50,000 steps, which forgets its nogoods once
    // (`forget_nogoods=off` as well: 56,990).
    (936, "matching", 20507, 41653),
    (936, "matching,lagrange=off", 20507, 59055),
    // W = 20, the frontier's width.
    (1133, "matching", 28690, 2122),
    (1133, "matching,lagrange=off", 28690, 2386),
    (1133, "matching,nogoods=off", 28690, 5269),
    (1133, "matching,fix_forced=off", 28690, 4848),
    (1133, "matching,match_refine=0", 28690, 2156),
];

fn tactics(assignments: &str) -> SearchTactics {
    let mut tactics = Tactics::default();
    for assignment in assignments.split(',') {
        match assignment {
            "default" => {}
            "matching" => tactics.search = SearchTactics::matching(),
            _ => tactics.set(assignment).unwrap(),
        }
    }
    tactics.search
}

/// Checks the rows of [`EXPECTED`] for the boards `ns`; on a difference, prints those boards' rows as observed.
fn check_trees(ns: &[u32]) {
    let mut observed = Vec::new();
    let mut changed = 0;
    for (n, assignments, sum, nodes) in EXPECTED.into_iter().filter(|row| ns.contains(&row.0)) {
        let actual = with_width!(n, W => search_at_width::<W>(n, tactics(assignments)));
        observed.push(format!("    ({n}, {assignments:?}, {}, {}),", actual.0, actual.1));
        if actual != (sum, nodes) {
            changed += 1;
            println!("n={n} {assignments}: (sum, nodes) {actual:?}, expected {:?}", (sum, nodes));
        }
    }
    assert!(
        changed == 0,
        "{changed} search trees changed (see the top of tests/tripwires.rs for what to do about it); observed:\n{}",
        observed.join("\n")
    );
}

/// The boards of [`EXPECTED`] in three tests, so that the test threads share the work; every board is in one.
const GROUPS: [&[u32]; 3] = [&[480, 600, 640], &[660, 760, 1000], &[936, 1133]];

#[test]
fn the_search_trees_of_480_600_and_640_have_not_changed() {
    check_trees(GROUPS[0]);
}

#[test]
fn the_search_trees_of_660_760_and_1000_have_not_changed() {
    check_trees(GROUPS[1]);
}

#[test]
fn the_search_trees_of_936_and_1133_have_not_changed() {
    check_trees(GROUPS[2]);
}

#[test]
fn every_board_of_the_table_is_checked() {
    let mut boards: Vec<u32> = EXPECTED.iter().map(|row| row.0).collect();
    boards.sort_unstable();
    boards.dedup();
    let mut covered: Vec<u32> = GROUPS.concat();
    covered.sort_unstable();
    assert_eq!(boards, covered, "the boards of EXPECTED and of GROUPS");
}

/// DESIGN.md (section 4): deterministic on one thread (nogoods, forgetting and all); nothing else asserts it for the
/// search.
#[test]
fn a_search_on_one_thread_is_deterministic() {
    for (n, t) in [(1000, SearchTactics::default()), (760, SearchTactics::matching())] {
        assert_eq!(search_at_width::<16>(n, t), search_at_width::<16>(n, t), "n={n}");
    }
}

/// (game, nodes forgetting the nogoods every 500 steps, nodes never forgetting) for [`forgetting_search`].
const FORGETTING: (u32, u64, u64) = (600, 5500, 4558);

/// The tree below the root of a game, searched on the matching path (the GF(2) bound's trees of the small games
/// are too small for forgetting to matter) by one searcher that forgets its nogoods every `period` steps
/// (`None`: never), from the greedy incumbent; returns (best, nodes).  A search forgets for the first time after
/// 50,000 steps (`NOGOOD_PERIOD`), so the games that finish sooner never meet what a long search meets all the
/// time: conflicts that the table no longer holds, that only the propagation of the node's known conflicts into its
/// children (`Stepper::make_child` in src/search/searcher.rs) still rules out.  Measured with a mutant that switched
/// that propagation off: no count changes below 50,000 steps (600, 760, 936, 1133, 1140, 1150), but forgetting
/// every 500 steps costs it 0.9 % at n = 600, and n = 992 (4.2M nodes) 22 %.
fn forgetting_search(game: &Game<16>, period: Option<u64>) -> (u64, u64) {
    let cfg = Config { period, ..Config::matching() };
    let start = greedy(&game.model, &game.root, &game.candidates).sum();
    let out = game.search(cfg, vec![game.root_unit()], start, None, None);
    assert!(out.completed && out.real);
    (out.best, out.nodes)
}

#[test]
fn the_tree_with_frequent_forgetting_has_not_changed() {
    let (n, expected_forgetting, expected_never) = FORGETTING;
    let game = Game::<16>::of_board(n).unwrap();
    let (best, forgetting) = forgetting_search(&game, Some(500));
    let (best_never, never) = forgetting_search(&game, None);
    assert_eq!(best, best_never);
    assert_eq!(
        (forgetting, never),
        (expected_forgetting, expected_never),
        "n={n}: nodes forgetting every 500 steps, nodes never forgetting; update FORGETTING"
    );
}
