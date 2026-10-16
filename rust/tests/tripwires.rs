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
//! [`FORGETTING`] and for [`EXTENDED`], the table of the search of the extended problem Π⁺(∅) (the frame model with
//! the claims of `unzip` put back and no source open; `Board::prepare_extended_with`), whose rows are checked the
//! same way.  Besides its table, Π⁺(∅) is held to the default engine: with no one-sided candidate its search is that
//! engine's on the same frames and candidates, node for node, which no table needs to say.

mod common;

use common::{search_at_width, Config, Game};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::bounds::greedy;
use taxman::search::{prepare, search, Prepared, SearchTactics};
use taxman::tactics::Tactics;
use taxman::with_width;

/// (board, tactics, promotion sum, nodes): the game searched from the frame builder on one thread.  The tactics are
/// `--tactic` assignments after the default or `matching`, the tactics of the matching path
/// ([`SearchTactics::matching`], the default before the GF(2) bound, and without the rule of the tactic `symmetry`
/// as it was).  The Lagrangian bound (tactic `lagrange`) first fits its multipliers after 2,000 steps, so the games
/// that take fewer (480, 640) do not see it; the matching path's rows `lagrange=off` are the trees of the build
/// before it.
const EXPECTED: [(u32, &str, u64, u64); 54] = [
    // The default, the GF(2) bound with the rule of the root's symmetric pairs: trees of tens to thousands of nodes;
    // `lagrange=on` (which refits after 2,000 steps) changes none of them, and `symmetry=off` (the default before the
    // rule) makes them larger with the same answers.
    (480, "default", 5350, 37),
    (600, "default", 8507, 365),
    (600, "fix_forced=off", 8507, 530),
    (600, "nogoods=off", 8507, 372),
    (600, "symmetry=off", 8507, 533),
    (640, "default", 9220, 15),
    (660, "default", 10242, 524),
    (660, "fix_forced=off", 10242, 908),
    (660, "nogoods=off", 10242, 618),
    (760, "default", 14170, 182),
    (936, "default", 20507, 385),
    (936, "fix_forced=off", 20507, 830),
    (936, "nogoods=off", 20507, 352),
    (936, "lagrange=on", 20507, 385),
    (936, "symmetry=off", 20507, 665),
    (1000, "default", 24168, 2172),
    (1000, "fix_forced=off", 24168, 4000),
    (1000, "nogoods=off", 24168, 3032),
    (1000, "lagrange=on", 24168, 2172),
    (1000, "symmetry=off", 24168, 2657),
    (1133, "default", 28690, 41),
    (1133, "nogoods=off", 28690, 34),
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
    (600, "matching,symmetry=on", 8507, 3856),
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
    (760, "matching,symmetry=on", 14170, 2292),
    // W = 16, and without the Lagrangian bound a search of more than 50,000 steps, which forgets its nogoods once
    // (`forget_nogoods=off` as well: 56,990).
    (936, "matching", 20507, 41653),
    (936, "matching,lagrange=off", 20507, 59055),
    // W = 20, the width of the hardest games.
    (1133, "matching", 28690, 2122),
    (1133, "matching,lagrange=off", 28690, 2386),
    (1133, "matching,nogoods=off", 28690, 5269),
    (1133, "matching,fix_forced=off", 28690, 4848),
    (1133, "matching,match_refine=0", 28690, 2156),
];

/// (board, tactics, promotion sum, nodes): the search of the extended problem Π⁺(∅) of the board, set up by
/// `Board::prepare_extended_with` with no source open, on one thread.  The tactics are `default` and `extended`
/// ([`SearchTactics::extended`]: the default without the rule of the symmetric pairs, the tactics of a search with
/// one-sided candidates, which refuses the rule).  The promotion sums are those of [`EXPECTED`]: Π⁺(∅) has the frame
/// model's promotion sets, and the sum is the answer.  The rows `extended` are the frame model's trees, node for node,
/// where [`EXPECTED`] has the same tactics (its rows `symmetry=off` for 600, 936 and 1000); the rows `default` differ
/// from those of [`EXPECTED`] (408 against 365 nodes at 600, 367 against 385 at 936, 2,258 against 2,172 at 1000), so
/// what differs is what the rule of the symmetric pairs reads from the two models, not the search.  Both are also held
/// to the default engine's search of the same frames and candidates ([`extended_searches`]), whatever the table says.
const EXTENDED: [(u32, &str, u64, u64); 16] = [
    (480, "default", 5350, 37),
    (480, "extended", 5350, 37),
    (600, "default", 8507, 408),
    (600, "extended", 8507, 533),
    (640, "default", 9220, 15),
    (640, "extended", 9220, 21),
    (660, "default", 10242, 547),
    (660, "extended", 10242, 573),
    (760, "default", 14170, 182),
    (760, "extended", 14170, 186),
    (936, "default", 20507, 367),
    (936, "extended", 20507, 665),
    (1000, "default", 24168, 2258),
    (1000, "extended", 24168, 2657),
    (1133, "default", 28690, 41),
    (1133, "extended", 28690, 41),
];

fn tactics(assignments: &str) -> SearchTactics {
    let mut tactics = Tactics::default();
    for assignment in assignments.split(',') {
        match assignment {
            "default" => {}
            "matching" => tactics.search = SearchTactics::matching(),
            "extended" => tactics.search = SearchTactics::extended(),
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

/// (promotion sum, nodes) of the search of `p` on one thread, which must complete.
fn searched<const W: usize>(p: &Prepared<W>) -> (u64, u64) {
    let r = search(p, &common::config(1), None, None, false).unwrap();
    assert!(r.completed);
    (r.sum, r.stats.nodes)
}

/// (promotion sum, nodes) of the search of the extended problem Π⁺(∅) of game `n` at width `W` (set up by
/// `Board::prepare_extended_with` with no source open), and of the default engine's search of the same problem: the
/// frames of Π⁺(∅) and its candidates given to `search::prepare`, which builds the model with `FrameGraph::build`.  The
/// two setups pose the same problem, and (the problem has no one-sided candidate) the two searches are one search.
fn extended_searches<const W: usize>(n: u32, tactics: SearchTactics) -> ((u64, u64), (u64, u64)) {
    let board = Board::<W>::for_n(n);
    let (ext, prepared) = board.prepare_extended_with(&Bits::EMPTY, false, tactics).unwrap();
    assert!(!prepared.model.has_one_sided(), "n={n}: Π⁺(∅) has a one-sided candidate");
    let engine = prepare::<W>(&board.tables, &ext.columns(), &ext.candidates, tactics).unwrap();
    assert_eq!(prepared.problem(), engine.problem(), "n={n}: the two setups pose different problems");
    assert_eq!((prepared.root, prepared.candidates), (engine.root, engine.candidates), "n={n}: the roots");
    assert_eq!((prepared.greedy, prepared.max_total), (engine.greedy, engine.max_total), "n={n}: the starts");
    (searched(&prepared), searched(&engine))
}

/// Checks the rows of [`EXTENDED`] for the boards `ns`, and that each of the searches is the default engine's on the
/// same problem; on a difference from the table, prints those boards' rows as observed.
fn check_extended_trees(ns: &[u32]) {
    let mut observed = Vec::new();
    let mut changed = 0;
    for (n, assignments, sum, nodes) in EXTENDED.into_iter().filter(|row| ns.contains(&row.0)) {
        let (actual, engine) = with_width!(n, W => extended_searches::<W>(n, tactics(assignments)));
        assert_eq!(actual, engine, "n={n} {assignments}: the search of Π⁺(∅) is not the default engine's");
        observed.push(format!("    ({n}, {assignments:?}, {}, {}),", actual.0, actual.1));
        if actual != (sum, nodes) {
            changed += 1;
            println!("n={n} {assignments}: (sum, nodes) {actual:?}, expected {:?}", (sum, nodes));
        }
    }
    assert!(
        changed == 0,
        "{changed} search trees of Π⁺(∅) changed (see the top of tests/tripwires.rs for what to do about it); \
         observed:\n{}",
        observed.join("\n")
    );
}

/// The boards of [`EXPECTED`] and of [`EXTENDED`] in three tests each, so that the test threads share the work; every
/// board is in one of each.
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
fn the_extended_search_trees_of_480_600_and_640_have_not_changed() {
    check_extended_trees(GROUPS[0]);
}

#[test]
fn the_extended_search_trees_of_660_760_and_1000_have_not_changed() {
    check_extended_trees(GROUPS[1]);
}

#[test]
fn the_extended_search_trees_of_936_and_1133_have_not_changed() {
    check_extended_trees(GROUPS[2]);
}

#[test]
fn every_board_of_the_tables_is_checked() {
    let mut covered: Vec<u32> = GROUPS.concat();
    covered.sort_unstable();
    let tables: [(&str, Vec<u32>); 2] = [
        ("EXPECTED", EXPECTED.iter().map(|row| row.0).collect()),
        ("EXTENDED", EXTENDED.iter().map(|row| row.0).collect()),
    ];
    for (name, mut boards) in tables {
        boards.sort_unstable();
        boards.dedup();
        assert_eq!(boards, covered, "the boards of {name} and of GROUPS");
    }
    // Each board of EXTENDED has a row for the default tactics and one for those of extended mode.
    for n in covered {
        let tactics: Vec<&str> = EXTENDED.iter().filter(|row| row.0 == n).map(|row| row.1).collect();
        assert_eq!(tactics, ["default", "extended"], "n={n}: the tactics of EXTENDED");
    }
}

/// `SearchTactics::extended` is the default without the rule of the symmetric pairs, which is how the table `EXPECTED`
/// names it (`symmetry=off`), so its rows of that name and the table `EXTENDED`'s of `extended` are searches under the
/// same tactics.
#[test]
fn the_tactics_of_extended_mode_are_the_default_without_the_symmetry_rule() {
    assert_eq!(tactics("extended"), tactics("symmetry=off"));
    assert_eq!(tactics("extended"), SearchTactics { symmetry: false, ..SearchTactics::default() });
    assert_ne!(tactics("extended"), tactics("default"));
    assert_eq!(tactics("extended").refusal_with_one_sided(), None);
    assert!(tactics("default").refusal_with_one_sided().is_some_and(|why| why.contains("symmetry=on")));
}

/// DESIGN.md (section 4): deterministic on one thread (nogoods, forgetting and all); nothing else asserts it for the
/// search.
#[test]
fn a_search_on_one_thread_is_deterministic() {
    for (n, t) in [(1000, SearchTactics::default()), (760, SearchTactics::matching())] {
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
