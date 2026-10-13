//! Node-count tripwires: the search tree of a fixed game under fixed tactics, counted exactly.
//!
//! Every search tactic is a pruning device: switching one off, or breaking it so that it prunes nothing, changes no
//! answer, so no assertion on answers sees it (a mutant that ignored the result of `Nogoods::ruled_out` costs
//! 27 to 67 % more nodes and passed every other test).  With one thread the tree is deterministic, so its size is
//! a fingerprint: a tactic that silently stops working makes a count larger, and one that starts pruning something
//! it should not (or a bound that gets weaker, or a changed order) moves it too.
//!
//! The counts are those of the code at the time of writing.  A change that alters the tree deliberately (a new
//! bound, another order, a different incumbent to start from, the frame builder or the revision of the freedoms)
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

/// (board, tactics as `--tactic` assignments, promotion sum, nodes): the game searched from the frame builder on one
/// thread, with the revision of frame 0's freedoms where the tactics have it on.  The 1133 rows have it off, like
/// the committed strict results.
const EXPECTED: [(u32, &str, u64, u64); 28] = [
    // W = 8, the small end: every tactic's effect shows in a few hundred nodes.
    (480, "default", 5350, 393),
    (480, "nogoods=off", 5350, 417),
    (480, "fix_forced=off", 5350, 503),
    (480, "match_refine=0", 5350, 431),
    (480, "match_refine=3", 5350, 254),
    (480, "preclusions=on", 5350, 391),
    // W = 16: games of a few hundred to ten thousand nodes.
    (600, "default", 8507, 5877),
    (600, "nogoods=off", 8507, 17740),
    (600, "fix_forced=off", 8507, 8180),
    (600, "match_refine=0", 8507, 6100),
    (600, "match_refine=3", 8507, 4070),
    (600, "branching=conflict-smallest", 8507, 24443),
    (600, "branching=conflict-largest", 8507, 10099),
    (640, "default", 9220, 676),
    (640, "nogoods=off", 9220, 1099),
    (640, "fix_forced=off", 9220, 1569),
    (660, "default", 10242, 10043),
    (660, "nogoods=off", 10242, 14817),
    (660, "fix_forced=off", 10242, 15685),
    (760, "default", 14170, 2567),
    (760, "nogoods=off", 14170, 3361),
    (760, "fix_forced=off", 14170, 5332),
    (760, "match_refine=3", 14170, 1643),
    // W = 16, and a search of more than 50,000 steps, which forgets its nogoods once (`forget_nogoods=off`: 56,990).
    (936, "default", 20507, 59055),
    // W = 20, the frontier's width.
    (1133, "revise_freedoms=off", 28690, 2386),
    (1133, "revise_freedoms=off,nogoods=off", 28690, 5269),
    (1133, "revise_freedoms=off,fix_forced=off", 28690, 6621),
    (1133, "revise_freedoms=off,match_refine=0", 28690, 2472),
];

fn tactics(assignments: &str) -> SearchTactics {
    let mut tactics = Tactics::default();
    for assignment in assignments.split(',').filter(|a| *a != "default") {
        tactics.set(assignment).unwrap();
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
const GROUPS: [&[u32]; 3] = [&[480, 600, 640], &[660, 760], &[936, 1133]];

#[test]
fn the_search_trees_of_480_600_and_640_have_not_changed() {
    check_trees(GROUPS[0]);
}

#[test]
fn the_search_trees_of_660_and_760_have_not_changed() {
    check_trees(GROUPS[1]);
}

#[test]
fn the_search_trees_of_936_and_1133_have_not_changed() {
    check_trees(GROUPS[2]);
}

#[test]
fn every_board_of_the_table_is_checked() {
    let mut boards: Vec<u32> = EXPECTED.iter().map(|row| row.0).collect();
    boards.dedup();
    let mut covered: Vec<u32> = GROUPS.concat();
    covered.sort_unstable();
    assert_eq!(boards, covered, "the boards of EXPECTED and of GROUPS");
}

/// DESIGN.md (section 4): deterministic on one thread (nogoods, forgetting and all); nothing else asserts it for the
/// search.
#[test]
fn a_search_on_one_thread_is_deterministic() {
    let d = SearchTactics::default();
    assert_eq!(search_at_width::<16>(760, d), search_at_width::<16>(760, d));
}

/// (game, nodes forgetting the nogoods every 500 steps, nodes never forgetting) for [`forgetting_search`].
const FORGETTING: (u32, u64, u64) = (600, 8270, 5877);

/// The tree below the root of a game, searched by one searcher that forgets its nogoods every `period` steps
/// (`None`: never), from the greedy incumbent; returns (best, nodes).  A search forgets for the first time after
/// 50,000 steps (`NOGOOD_PERIOD`), so the games that finish sooner never meet what a long search meets all the
/// time: conflicts that the table no longer holds, that only the propagation of the node's known conflicts into its
/// children (search.rs, "A known conflict of the node ...") still rules out.  Measured with a mutant that switched
/// that propagation off: no count changes below 50,000 steps (600, 760, 936, 1133, 1140, 1150), but forgetting
/// every 500 steps costs it 0.9 % at n = 600, and n = 992 (4.2M nodes) 22 %.
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
    let (best, forgetting) = forgetting_search(&game, Some(500));
    let (best_never, never) = forgetting_search(&game, None);
    assert_eq!(best, best_never);
    assert_eq!(
        (forgetting, never),
        (expected_forgetting, expected_never),
        "n={n}: nodes forgetting every 500 steps, nodes never forgetting; update FORGETTING"
    );
}
