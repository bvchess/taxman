//! Adversarial soundness tests of the search: could a pruning, a shortcut or a resume lose a better promotion set?
//!
//! The key idea is the handicapped incumbent.  The engine is started with a fake incumbent worth less than the best
//! completion (`truth − k`) and an empty payload.  If any pruning step loses every best completion, the fake one
//! survives, and its payload does not match its value.  This tests every pruning at once, including all the `<`
//! and `<=` boundaries: from `truth − 1`, every node whose bound is exactly `truth` must be explored.  `truth` is
//! the table's promotion sum at the root of a real game, and an exhaustive search for perturbed games and
//! units deep in the tree.
//!
//! The last section runs the same tests on the extended problem Π⁺(O) (a problem with one-sided candidates, searched
//! under the tactics of extended mode): every board to 300 from the root, and one large game resumed from
//! checkpoints.

mod common;
#[allow(dead_code)]
#[path = "../examples/common/fixtures.rs"]
mod fixtures;

use common::{config, random_open_sets, random_subset, testdata, upper_from_3, Config, Game, Rng, TempDir};
use std::cell::OnceCell;
use std::time::Duration;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::EngineConfig;
use taxman::frames::{build_frames, ExtendedFrames};
use taxman::game::Optimal;
use taxman::gf2::{Gf2Bound, Gf2Graph};
use taxman::model::{FrameGraph, FrameSet, Scratch};
use taxman::numtheory::Tables;
use taxman::search::symmetry::Symmetries;
use taxman::search::TaxUnit;
use taxman::search::{search, Reuse, SearchTactics};

/// Every tactic that acts inside the search, alone and in the combinations that matter: first the default, the GF(2)
/// bound with the rule of the root's symmetric pairs (tactic `symmetry`), without forced members, without nogoods, with
/// the nogoods forgotten at every step and on two threads (0 to 4), then without the rule, on one thread, on two, and
/// with the nogoods forgotten at every step (5 to 7), then conflict branching (tactic `conflict_branching`) at every
/// check, whatever its gap, without the look at the last child of the search's own conflict (tactic `conflict_look`, on
/// by default), on one thread and on two (8 and 9), the search's own conflict everywhere (10; the default chooses from
/// a gap of 1,200), and conflict branching at every check with the look, on one thread and on two (11 and 12), then the
/// search without the near-repeat cut (13; tactic `near_repeat`, which the others have, and which acts where a game
/// has the cut of a game before it, [`game_after`], and the incumbent reaches its bound), then conflict branching at
/// every check without the reuse of its prices (tactic `price_reuse`, which the standard configuration has, with
/// the standard schedule, like the others that choose at every check), and under a schedule that never makes the prices
/// again, on two threads, and one that takes none (14 to 17), then the nogoods carried into the search (tactic
/// `carried_nogoods`, a store of every nogood of two members among neighboring candidates and of some of three and
/// four, [`Game::carried`](common::Game::carried)): alone, on two threads, with conflict branching at every check and
/// the reuse of its prices, and with the search's own conflict everywhere (18 to 21), then the search without the
/// reuse of prices and without the warm start of the children's bound (tactic `warm_start`, which the standard
/// configuration has too), and with the search's own conflict everywhere on two threads without the warm start (22 and
/// 23).
fn configs() -> Vec<Config> {
    let g = Config::standard();
    // The schedules of `price_reuse`, with conflict branching at every check: the standard one; one that takes the
    // prices of an earlier check however old and however far the bound has fallen, and takes the ordered conflicts of
    // the check before; and one whose limits ask for new prices at every check but the first of a node.
    let forever = Reuse { max_steps: u32::MAX, own_drop: u64::MAX, max_seeds: u32::MAX, seed_drop: u64::MAX };
    let never_again = Reuse { max_steps: 1, own_drop: 0, max_seeds: 0, seed_drop: 0 };
    vec![
        g,
        Config { fix_forced: false, ..g },
        Config { nogoods: false, ..g },
        Config { period: Some(1), ..g },
        Config { threads: 2, ..g },
        Config { symmetry: false, ..g },
        Config { symmetry: false, threads: 2, ..g },
        Config { symmetry: false, period: Some(1), ..g },
        Config { conflict_branching: Some(0), conflict_look: false, ..g },
        Config { conflict_branching: Some(0), conflict_look: false, threads: 2, period: Some(3), ..g },
        Config { conflict_branching: None, ..g },
        Config { conflict_branching: Some(0), conflict_look: true, ..g },
        Config { conflict_branching: Some(0), conflict_look: true, threads: 2, period: Some(3), ..g },
        Config { near_repeat: false, ..g },
        Config { conflict_branching: Some(0), price_reuse: None, ..g },
        Config { conflict_branching: Some(0), conflict_look: false, price_reuse: Some(forever), ..g },
        Config { conflict_branching: Some(0), price_reuse: Some(forever), threads: 2, period: Some(3), ..g },
        Config { conflict_branching: Some(0), price_reuse: Some(never_again), ..g },
        Config { carried: true, ..g },
        Config { carried: true, threads: 2, period: Some(3), ..g },
        Config { carried: true, conflict_branching: Some(0), price_reuse: Some(Reuse::STANDARD), ..g },
        Config { carried: true, conflict_branching: None, ..g },
        Config { price_reuse: None, warm_start: false, ..g },
        Config { conflict_branching: None, warm_start: false, threads: 2, period: Some(3), ..g },
    ]
}

/// The table's best promotion sum of game `n`: its score less what the game with no promotions scores.
fn optimal_sum<const W: usize>(optimal: &Optimal, n: u32) -> Option<u64> {
    let in_the_bag = Board::<W>::for_n(n).in_the_bag;
    optimal.score(n).map(|score| score - in_the_bag)
}

/// Game `n` at width `W` ([`Game::of_board`]) with the near-repeat cut (tactic `near_repeat`,
/// [`Game::with_near_repeat`]) by the latest of the four games before it whose best promotion sum (the table's,
/// the cut's bound) is at most `reach`, else by the latest of them; without it where none of them has candidates and a
/// game in the table.  A search cuts by it where its configuration has the tactic and its incumbent reaches the
/// bound, so a search from an incumbent of `reach` cuts by the game chosen, if any.  The solver cuts by the game of
/// n − 1 alone, but the lemma holds for any game proven before, and a search from an incumbent below its own optimum
/// reaches few bounds of n − 1.
fn game_after<const W: usize>(optimal: &Optimal, n: u32, reach: u64) -> Option<Game<W>> {
    let game = Game::<W>::of_board(n)?;
    Some(match game_before::<W>(optimal, n, reach) {
        Some((m, bound)) => game.with_near_repeat(&m, bound),
        None => game,
    })
}

/// The game before `n` whose cut [`game_after`] gives game `n` for an incumbent of `reach`, and its bound, the best
/// promotion sum of the table's game.
fn game_before<const W: usize>(optimal: &Optimal, n: u32, reach: u64) -> Option<(Game<W>, u64)> {
    let before = |m: u32| Some((Game::<W>::of_board(m)?, optimal_sum::<W>(optimal, m)?));
    let mut earlier: Vec<_> = (n.saturating_sub(4).max(1)..n).rev().filter_map(before).collect();
    let chosen = earlier.iter().position(|(_, bound)| *bound <= reach).unwrap_or(0);
    (chosen < earlier.len()).then(|| earlier.swap_remove(chosen))
}

// ---------------------------------------------------------------------------------------------------------
// Real games

/// Every game of `ns` that has candidates, from an incumbent worth 1 or 5 less than the table's promotion sum,
/// under each tactic (past the board 200 one in three), with the near-repeat cut by a game before ([`game_after`])
/// whose bound both incumbents reach where there is one: the search must come back with exactly that sum, and with a
/// set that is promotable.  More than `fewest` searches are made.  Returns the nodes the cut cut.
fn searches_from_a_handicapped_incumbent(ns: std::ops::RangeInclusive<u32>, fewest: usize) -> u64 {
    let optimal = Optimal::built_in();
    let configs = configs();
    let (mut runs, mut cut) = (0, 0);
    for n in ns {
        let truth = optimal_sum::<8>(optimal, n).unwrap_or(0);
        let Some(game) = game_after::<8>(optimal, n, truth.saturating_sub(5)) else { continue };
        // The big trees are for one configuration of each kind; the small ones for all.
        for (i, cfg) in configs.iter().enumerate() {
            for k in [1, 5] {
                if k > truth || (n > 200 && i % 3 != 0) {
                    continue;
                }
                let out = game.search(*cfg, vec![game.root_unit()], truth - k, None, None);
                (runs, cut) = (runs + 1, cut + out.near_cuts);
                assert!(out.completed, "n={n} {cfg:?} from {}: not completed", truth - k);
                assert!(out.best == truth && out.real, "n={n} {cfg:?} from {}: truth {truth}, got {out:?}", truth - k);
            }
        }
    }
    assert!(runs > fewest, "{runs} runs");
    cut
}

/// The games 2..=46, under each tactic, from an incumbent worth 1 or 5 less than the table's promotion sum.  Their
/// trees are too small for the near-repeat cut to act: [`the_near_repeat_cut_acts_and_changes_no_answer`] has games
/// where it does.  The small games are three tests, so that their thousands of searches (each one starts its threads
/// and makes its searchers' memory) run side by side.
#[test]
fn a_search_from_a_handicapped_incumbent_finds_the_optimum_of_the_games_2_to_46() {
    searches_from_a_handicapped_incumbent(2..=46, 100);
}

/// [`a_search_from_a_handicapped_incumbent_finds_the_optimum_of_the_games_2_to_46`] for the games 47..=69.
#[test]
fn a_search_from_a_handicapped_incumbent_finds_the_optimum_of_the_games_47_to_69() {
    searches_from_a_handicapped_incumbent(47..=69, 100);
}

/// [`a_search_from_a_handicapped_incumbent_finds_the_optimum_of_the_games_2_to_46`] for the games 70..=90.
#[test]
fn a_search_from_a_handicapped_incumbent_finds_the_optimum_of_the_games_70_to_90() {
    searches_from_a_handicapped_incumbent(70..=90, 100);
}

/// Every game 2..=300 that has candidates, from an incumbent worth 1 or 5 less than the table's promotion sum,
/// under each tactic: the search must come back with exactly that sum, and with a set that is promotable.  Unlike
/// the golden tests, which start from the greedy set, this starts from a weak incumbent.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is a_search_from_a_handicapped_incumbent_finds_the_optimum_of_the_games_2_to_46, \
              a_search_from_a_handicapped_incumbent_finds_the_optimum_of_the_games_47_to_69 and \
              a_search_from_a_handicapped_incumbent_finds_the_optimum_of_the_games_70_to_90"
)]
fn a_search_from_a_handicapped_incumbent_finds_the_optimum() {
    let cut = searches_from_a_handicapped_incumbent(2..=300, 4000);
    assert!(cut > 0, "the near-repeat cut cut no node");
}

/// The near-repeat cut acts, and changes no answer: the games 404, 430 and 436, whose searches from an incumbent below
/// their optimum it cuts (by the games before them that [`game_after`] chooses), under each tactic, from an incumbent
/// worth 1 or 5 less than the table's promotion sum, must come back with exactly that sum and a promotable set, and the
/// cut changes some of the trees.
#[test]
fn the_near_repeat_cut_acts_and_changes_no_answer() {
    let optimal = Optimal::built_in();
    let (mut cut, mut changed) = (0, 0);
    for n in [404, 430, 436] {
        let truth = optimal_sum::<8>(optimal, n).unwrap();
        let game = game_after::<8>(optimal, n, truth - 5).unwrap();
        for cfg in configs() {
            for k in [1, 5] {
                let [with, without] = [cfg.near_repeat, false].map(|near_repeat| {
                    game.search(Config { near_repeat, ..cfg }, vec![game.root_unit()], truth - k, None, None)
                });
                for out in [&with, &without] {
                    assert!(
                        out.completed && out.best == truth && out.real,
                        "n={n} {cfg:?} from {}: {out:?}",
                        truth - k
                    );
                }
                cut += with.near_cuts;
                changed += usize::from(with.nodes != without.nodes);
            }
        }
    }
    assert!(cut > 0 && changed > 0, "the cut cut {cut} nodes and changed {changed} trees");
}

/// The carried nogoods act, and change no answer: the games 232, 344 and 376, under each configuration that has a store
/// ([`Game::carried`](common::Game::carried)) and the same without it, from an incumbent worth 1 or 5 less than
/// the table's promotion sum, must come back with exactly that sum and a promotable set; the stores rule candidates
/// out and end nodes, and change some of the trees.
#[test]
fn the_carried_nogoods_act_and_change_no_answer() {
    let optimal = Optimal::built_in();
    let (mut changed, mut ruled_out, mut cuts) = (0, 0, 0);
    for n in [232, 344, 376] {
        let truth = optimal_sum::<8>(optimal, n).unwrap();
        let game = game_after::<8>(optimal, n, truth - 5).unwrap();
        for cfg in configs().into_iter().filter(|cfg| cfg.carried) {
            for k in [1, 5] {
                let [with, without] = [true, false].map(|carried| {
                    game.search(Config { carried, ..cfg }, vec![game.root_unit()], truth - k, None, None)
                });
                for out in [&with, &without] {
                    assert!(
                        out.completed && out.best == truth && out.real,
                        "n={n} {cfg:?} from {}: {out:?}",
                        truth - k
                    );
                }
                changed += usize::from(with.nodes != without.nodes);
            }
        }
        let (r, c) = game.carried().counts();
        assert!(!game.carried().is_empty(), "n={n}: an empty store");
        (ruled_out, cuts) = (ruled_out + r, cuts + c);
    }
    assert!(
        ruled_out > 0 && cuts > 0 && changed > 0,
        "the stores ruled out {ruled_out} candidates, ended {cuts} nodes and changed {changed} trees"
    );
}

/// The near-repeat cut waits for the incumbent to reach its bound.  On the games 404, 430 and 436, whose trees it
/// changes, by the games before them that [`game_after`] chooses: a search whose incumbent never reaches
/// the bound (one above the game's P*, which bounds the earlier game's P* as its own does) cuts nothing, and
/// searches the tree of the search without the cut, node for node, where the same search with the earlier game's own
/// bound cuts; and searches from an incumbent below the bound, just below and from nothing, armed from the start as a
/// search armed by a provisional game is, find P*, and cut once their incumbent has reached the bound.
#[test]
fn the_near_repeat_cut_cuts_nothing_while_the_incumbent_is_below_its_bound() {
    let (optimal, cfg) = (Optimal::built_in(), Config::standard());
    let (mut cut_at_the_bound, mut cut_on_the_way) = (0, 0);
    for n in [404, 430, 436] {
        let truth = optimal_sum::<8>(optimal, n).unwrap();
        let (m, bound) = game_before::<8>(optimal, n, truth - 5).unwrap();
        let cut_by = |bound: u64| Game::<8>::of_board(n).unwrap().with_near_repeat(&m, bound);
        let search = |game: &Game<8>, near_repeat: bool, from: u64| {
            let out = game.search(Config { near_repeat, ..cfg }, vec![game.root_unit()], from, None, None);
            assert!(out.completed && out.best == truth && out.real, "n={n} from {from}: {out:?}");
            out
        };
        let (at_the_bound, above) = (cut_by(bound), cut_by(bound.max(truth + 1)));
        let uncut = search(&at_the_bound, false, truth - 1);
        let never = search(&above, true, truth - 1);
        assert_eq!((never.near_cuts, never.nodes), (0, uncut.nodes), "n={n}: the cut acted below its bound");
        cut_at_the_bound += search(&at_the_bound, true, truth - 1).near_cuts;
        cut_on_the_way += [bound - 1, 0].map(|from| search(&at_the_bound, true, from).near_cuts).iter().sum::<u64>();
    }
    assert!(cut_at_the_bound > 0 && cut_on_the_way > 0, "cut {cut_at_the_bound} and {cut_on_the_way} nodes");
}

/// The look at the last child of the search's own conflict (tactic `conflict_look`) acts, and changes no answer: with
/// conflict branching at every check, whatever its gap, it changes the tree of some of the games 301 to 500 (every
/// tenth) from the tree without it, and each search comes back with exactly the sum of the table, from an
/// incumbent worth 1 less.
#[test]
fn the_look_changes_the_trees_of_the_small_games_and_not_their_answers() {
    let optimal = Optimal::built_in();
    let branching = Config { conflict_branching: Some(0), conflict_look: false, ..Config::standard() };
    let looking = Config { conflict_look: true, ..branching };
    let mut changed = 0;
    for n in (301..=500).step_by(10) {
        let Some(game) = Game::<8>::of_board(n) else { continue };
        let truth = optimal_sum::<8>(optimal, n).unwrap();
        let [plain, looked] =
            [branching, looking].map(|cfg| game.search(cfg, vec![game.root_unit()], truth - 1, None, None));
        for out in [&plain, &looked] {
            assert!(out.completed && out.best == truth && out.real, "n={n}: truth {truth}, got {out:?}");
        }
        changed += usize::from(plain.nodes != looked.nodes);
    }
    assert!(changed > 0, "the look changed no tree");
}

/// The gate, the gap from which conflict branching chooses (what `solve --gate` and the gate trial set), acts and
/// changes no answer: with the gates 0, 100 and 4,000 (the most `--gate` may set), each search of the games 301 to
/// 500 (every tenth) comes back with exactly the sum of the table, from an incumbent worth 1 less, and the gates
/// change the tree of some of them.
#[test]
fn the_gate_changes_the_trees_of_the_small_games_and_not_their_answers() {
    let optimal = Optimal::built_in();
    let gated = |gate| Config { conflict_branching: Some(gate), ..Config::standard() };
    let mut changed = 0;
    for n in (301..=500).step_by(10) {
        let Some(game) = Game::<8>::of_board(n) else { continue };
        let truth = optimal_sum::<8>(optimal, n).unwrap();
        let outs = [0, 100, 4000].map(|gate| game.search(gated(gate), vec![game.root_unit()], truth - 1, None, None));
        for out in &outs {
            assert!(out.completed && out.best == truth && out.real, "n={n}: truth {truth}, got {out:?}");
        }
        changed += usize::from(outs.iter().any(|out| out.nodes != outs[0].nodes));
    }
    assert!(changed > 0, "the gate changed no tree");
}

/// A unit below the root of a game, searched from a handicapped incumbent, with what an exhaustive search found in it.
///
/// The prime-exchange theorem (R2, docs/theory/search-rules.md) says that every heaviest promotion set of the *root*
/// keeps the inequality of each symmetric pair, and the search with the tactic `symmetry` looks at the completions that
/// keep them, at every node.  A unit below the root is no root: its best completion can break an inequality (some
/// promotion of its `promoted` may even break one already, which no completion mends), and its search finds the best
/// completion that keeps them (the exchange-pruning lemma, R3, part 3).  So a search with the rule is compared with the
/// oracle for that, [`Game::exhaustive_keeping`], and the others with [`Game::exhaustive`].
struct Searched<'a, const W: usize> {
    game: &'a Game<W>,
    sym: &'a Symmetries,
    /// The unit, the state its promotions reach, and the nodes the oracles may take.
    unit: TaxUnit<W>,
    state: FrameSet<W>,
    budget: u64,
    /// The sum of the best completion (not counting the promotions).
    rest: u64,
    /// The sum of the best completion that keeps the pairs' inequalities with the promotions: `None` if finding it
    /// took more than the budget, `Some(None)` if no completion keeps them.  Found when a search with the rule asks
    /// for it ([`kept`](Self::kept)), and not at all if the best completion keeps them.
    kept: OnceCell<Option<Option<u64>>>,
}

impl<'a, const W: usize> Searched<'a, W> {
    /// The oracles for `unit`, whose state is `state`, or `None` if the best completion took more than `budget`
    /// nodes to find.
    ///
    /// The best completion that keeps the inequalities needs a search of its own, which prunes worse than the first
    /// (until it has found a completion that keeps them it has no sum to prune by).  But the best of all
    /// completions is the best of those that keep the inequalities whenever it is one of them: the latter are among
    /// the former, so none is heavier.  So if the best completion found keeps them with the promotions (which is
    /// trivially so where the root has no pair), that is the answer, and the second search is not made.
    fn of(game: &'a Game<W>, sym: &'a Symmetries, state: &FrameSet<W>, unit: TaxUnit<W>, budget: u64) -> Option<Self> {
        let (rest, best) = game.exhaustive(state, &unit.cands, budget)?;
        let kept = if sym.keeps(&(unit.promoted | best)) { OnceCell::from(Some(Some(rest))) } else { OnceCell::new() };
        Some(Searched { game, sym, unit, state: *state, budget, rest, kept })
    }

    /// The sum of the best completion that keeps the pairs' inequalities (see the field `kept`), found once.
    fn kept(&self) -> Option<Option<u64>> {
        *self.kept.get_or_init(|| {
            let (unit, state) = (&self.unit, &self.state);
            let found = self.game.exhaustive_keeping(state, &unit.promoted, &unit.cands, self.budget, self.sym);
            found.map(|found| found.map(|(sum, _)| sum))
        })
    }

    /// Searches the unit under `cfg` from an incumbent `k` (or less, where the unit has less to find) below what the
    /// search must come back with, and checks the outcome; returns whether there was a search to make.
    ///
    /// Without the tactic `symmetry` the search must find the best completion.  With it, the best completion that
    /// keeps the inequalities, or something at least as heavy (a promotable set that breaks them can be heavier);
    /// where none keeps them there is nothing to find, but whatever the search ends with is the fake incumbent or a
    /// promotable set.
    fn run(&self, cfg: Config, k: u64, at: &str) -> bool {
        let promoted = self.unit.promoted.sum();
        let truth = promoted + self.rest;
        // The best completion that the search is to find: with the rule the best one that keeps the inequalities
        // (which is not looked for unless a search needs it), else the best one.
        let kept = if cfg.symmetry { self.kept() } else { Some(Some(self.rest)) };
        // What the search must find (`None`: nothing), and the sum of the completion that the incumbent is below.
        let (target, reach) = match kept {
            Some(Some(kept)) => (Some(kept), kept),
            Some(None) => (None, self.rest),
            None => return false,
        };
        let k = k.min(reach);
        if k == 0 {
            return false;
        }
        let start = promoted + reach - k;
        let out = self.game.search(cfg, vec![self.unit], start, None, None);
        let at = format!("{at} {cfg:?} from {start}: truth {truth}, kept {kept:?}, got {out:?}");
        assert!(out.completed, "{at}: not completed");
        match target {
            Some(target) if !cfg.symmetry => assert!(out.best == promoted + target && out.real, "{at}"),
            Some(target) => assert!(out.best >= promoted + target && out.best <= truth && out.real, "{at}"),
            None => assert!(out.best <= truth && (out.best == start || out.real), "{at}"),
        }
        true
    }
}

/// Every board 2..=600 whose root has symmetric pairs, searched from the root with the rule of the pairs' inequalities
/// (tactic `symmetry`), from an incumbent 1 and 5 below
/// the table's promotion sum: the search must complete with exactly that sum and a promotable set (R2,
/// docs/theory/search-rules.md: the root's heaviest set keeps the inequalities, so a search from the root loses
/// nothing).  For the boards to 40 opt(n) is also checked against the exhaustive search without the frame model.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is the_symmetry_rule_finds_the_optimum_of_the_small_boards_with_pairs"
)]
fn the_symmetry_rule_finds_the_optimum_of_the_boards_with_pairs() {
    let optimal = Optimal::built_in();
    let configs = symmetry_configs();
    let boards =
        boards_with_pairs::<8>(optimal, 2..=511, &configs) + boards_with_pairs::<16>(optimal, 512..=600, &configs);
    assert!(boards >= 450, "{boards} boards with pairs");
}

/// [`the_symmetry_rule_finds_the_optimum_of_the_boards_with_pairs`] on the boards 2..=220.
#[test]
fn the_symmetry_rule_finds_the_optimum_of_the_small_boards_with_pairs() {
    let boards = boards_with_pairs::<8>(Optimal::built_in(), 2..=220, &symmetry_configs());
    assert!(boards >= 40, "{boards} boards with pairs");
}

/// The rule of the symmetric pairs, with forced members and without.
fn symmetry_configs() -> [Config; 2] {
    [Config::standard(), Config { fix_forced: false, ..Config::standard() }]
}

/// The searches of [`the_symmetry_rule_finds_the_optimum_of_the_boards_with_pairs`] of the boards `ns` at width `W`;
/// returns the number of boards with pairs.
fn boards_with_pairs<const W: usize>(
    optimal: &Optimal,
    ns: std::ops::RangeInclusive<u32>,
    configs: &[Config],
) -> usize {
    let mut boards = 0;
    for n in ns {
        let truth = optimal_sum::<W>(optimal, n).unwrap_or(0);
        let Some(game) = game_after::<W>(optimal, n, truth.saturating_sub(5)) else { continue };
        if game.symmetries().is_empty() {
            continue;
        }
        boards += 1;
        if n <= 40 {
            let exact = taxman::brute::solve(n, &[]).score;
            assert_eq!(Board::<W>::for_n(n).in_the_bag + truth, exact, "n={n}: the best score against the oracle");
        }
        for cfg in configs {
            for k in [1, 5] {
                if k > truth {
                    continue;
                }
                let out = game.search(*cfg, vec![game.root_unit()], truth - k, None, None);
                assert!(out.completed, "n={n} {cfg:?} from {}: not completed", truth - k);
                assert!(out.best == truth && out.real, "n={n} {cfg:?} from {}: truth {truth}, got {out:?}", truth - k);
            }
        }
    }
    boards
}

/// Units of game `n` at width `W`, after up to 14 random promotions, with their candidates thinned to 6 to 23 (the
/// largest, or at random) so that the truth can be found exhaustively, searched from an incumbent worth 1, 2, 5
/// or 30 less under random `configs`, with the near-repeat cut by a game before ([`game_after`]): the search must
/// come back with the truth (with the tactic `symmetry`, as [`Searched::run`] says).  Returns the number of searches.
fn handicapped_units<const W: usize>(n: u32, rng: &mut Rng, units: usize, configs: &[Config]) -> usize {
    let optimal = Optimal::built_in();
    let reach = optimal_sum::<W>(optimal, n).unwrap_or(0).saturating_sub(5);
    let game = game_after::<W>(optimal, n, reach).expect("a game");
    let sym = game.symmetries();
    let (mut runs, mut made) = (0, 0);
    while made < units {
        let (mut state, mut promoted, mut cands) = (game.root, Bits::EMPTY, game.candidates);
        for _ in 0..rng.below(14) {
            let members = cands.to_vec();
            if members.is_empty() {
                break;
            }
            let x = rng.pick(&members);
            cands.remove(x);
            if game.model.promote_in_place(&mut state, x) {
                promoted.insert(x);
                cands = state.narrow_candidates(&cands);
            }
        }
        let mut members = cands.to_vec();
        if members.is_empty() {
            continue;
        }
        if rng.chance(50) {
            members.reverse();
        } else {
            for i in (1..members.len()).rev() {
                members.swap(i, rng.below(i as u64 + 1) as usize);
            }
        }
        members.truncate(6 + rng.below(18) as usize);
        let thin: Bits<W> = members.iter().copied().collect();
        let unit = TaxUnit { promoted, cands: thin };
        let Some(searched) = Searched::of(&game, &sym, &state, unit, 3_000_000) else { continue };
        made += 1;
        let at = format!("n={n} W={W} promoted {promoted:?} cands {thin:?}");
        for _ in 0..if searched.rest == 0 { 0 } else { 4 } {
            let cfg = rng.pick(configs);
            let k = rng.pick(&[1u64, 1, 2, 5, 30]);
            runs += usize::from(searched.run(cfg, k, &at));
        }
    }
    runs
}

/// Units of real games at W = 20 to 64, the widths of the hardest games and beyond, from a handicapped incumbent under
/// every configuration: the game tests above stop at W = 8, the perturbed games below at W = 4.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is a_few_units_of_wide_games_from_a_handicapped_incumbent_find_the_optimum"
)]
fn units_of_wide_games_from_a_handicapped_incumbent_find_the_optimum() {
    let configs = configs();
    let mut rng = Rng(0x3D1D_E000_0000_0077);
    let runs = handicapped_units::<20>(1100, &mut rng, 40, &configs)
        + handicapped_units::<20>(1250, &mut rng, 40, &configs)
        + handicapped_units::<24>(1300, &mut rng, 40, &configs)
        + handicapped_units::<24>(1500, &mut rng, 40, &configs)
        + handicapped_units::<28>(1600, &mut rng, 40, &configs)
        + handicapped_units::<28>(1700, &mut rng, 40, &configs)
        + handicapped_units::<32>(1850, &mut rng, 40, &configs)
        + handicapped_units::<32>(2040, &mut rng, 40, &configs)
        + handicapped_units::<64>(2050, &mut rng, 40, &configs)
        + handicapped_units::<64>(3100, &mut rng, 40, &configs);
    assert!(runs > 1000, "{runs} runs");
}

/// [`units_of_wide_games_from_a_handicapped_incumbent_find_the_optimum`] with a few units of the games 1100 at W = 20
/// and 2050 at W = 64, the narrowest and the widest of them, under the configurations without carried nogoods: the
/// store of a game this wide takes a search of its own to make ([`Game::carried`](common::Game::carried)), which the
/// test above makes and the small games' tests make for every game.
#[test]
fn a_few_units_of_wide_games_from_a_handicapped_incumbent_find_the_optimum() {
    let mut rng = Rng(0x3D1D_E000_0000_0078);
    let configs: Vec<Config> = configs().into_iter().filter(|cfg| !cfg.carried).collect();
    let runs =
        handicapped_units::<20>(1100, &mut rng, 4, &configs) + handicapped_units::<64>(2050, &mut rng, 4, &configs);
    assert!(runs > 20, "{runs} runs");
}

/// Units on the path to an optimal promotion set `S*` of game `n` at width `W`: the promotions are `S*` less a random
/// 30, 50 or 70 percent of its members, the candidates what the search leaves them, searched under each of `configs`
/// from an incumbent one below P*, with the near-repeat cut by a game before ([`game_after`]).  Such a unit holds `S*`,
/// which keeps every symmetric pair's inequality (R2, docs/theory/search-rules.md), so the search with the rule of the
/// pairs (tactic `symmetry`) must find P*; a rule that cuts a node on the path to `S*`, or drops a candidate `S*`
/// needs, does not.  The nodes of such a search are deep enough that the promotions and the candidates leave the pairs'
/// inequalities open, which a game searched from its root rarely does.  Returns the number of searches.
fn optimum_units<const W: usize>(optimal: &Optimal, n: u32, rng: &mut Rng, units: usize, configs: &[Config]) -> usize {
    let truth = optimal_sum::<W>(optimal, n).expect("a known game");
    let game = game_after::<W>(optimal, n, truth - 1).expect("a game");
    let sym = game.symmetries();
    let moves = optimal.moves(n).expect("a known game");
    let best: Bits<W> = moves.iter().copied().filter(|&x| game.candidates.contains(x)).collect();
    if best.sum() != truth || sym.is_empty() {
        return 0;
    }
    let (mut runs, mut scratch) = (0, Scratch::new());
    for _ in 0..units {
        let percent = rng.pick(&[30, 50, 70]);
        let promoted: Bits<W> = best.iter().filter(|_| !rng.chance(percent)).collect();
        // A unit that already holds an optimal set has nothing to find: the search offers only what a child promotes.
        let Some(state) =
            game.model.promote_all_using(&mut scratch, &game.root, &promoted).filter(|_| promoted != best)
        else {
            continue;
        };
        let unit = TaxUnit { promoted, cands: state.narrow_candidates(&(game.candidates - promoted)) };
        for cfg in configs {
            let out = game.search(*cfg, vec![unit], truth - 1, None, None);
            runs += 1;
            assert!(out.completed, "n={n} {cfg:?} promoted {promoted:?}: not completed");
            assert!(out.best == truth && out.real, "n={n} {cfg:?} promoted {promoted:?}: truth {truth}, got {out:?}");
        }
    }
    runs
}

/// The searches of the units on the way to an optimal set ([`optimum_units`]) of the games 600 to 1000 (every 23rd)
/// under the rule of the symmetric pairs, on one thread and on two.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is a_few_units_on_the_way_to_the_optimum_find_it_with_the_symmetry_rule"
)]
fn units_on_the_way_to_the_optimum_find_it_with_the_symmetry_rule() {
    let optimal = Optimal::built_in();
    let mut rng = Rng(0x5B0D_1E57_C0DE_0003);
    let runs: usize =
        (600..=1000).step_by(23).map(|n| optimum_units::<16>(optimal, n, &mut rng, 6, &on_the_way_configs())).sum();
    assert!(runs > 200, "{runs} runs");
}

/// [`units_on_the_way_to_the_optimum_find_it_with_the_symmetry_rule`] for the games 600, 700, 800, 900 and 1000, two
/// units each.
#[test]
fn a_few_units_on_the_way_to_the_optimum_find_it_with_the_symmetry_rule() {
    let optimal = Optimal::built_in();
    let mut rng = Rng(0x5B0D_1E57_C0DE_0004);
    let runs: usize =
        (600..=1000).step_by(100).map(|n| optimum_units::<16>(optimal, n, &mut rng, 2, &on_the_way_configs())).sum();
    assert!(runs > 10, "{runs} runs");
}

/// The rule of the symmetric pairs on one thread and on two.
fn on_the_way_configs() -> [Config; 2] {
    let g = Config::standard();
    [g, Config { threads: 2, ..g }]
}

/// A search interrupted every `node_limit` nodes or so and resumed (two threads, a checkpoint every millisecond, a
/// tiny memory cap so that units spill to disk), from an incumbent `handicap` below the promotion sum `truth` of
/// game `n`, ends with P* for the configurations `which` of [`configs`]: the unit tests of resuming use one game and
/// a few tactics.  Returns the number of resumes.
fn resumed_over_and_over<const W: usize>(
    n: u32,
    truth: u64,
    handicap: u64,
    node_limit: u64,
    dir: &TempDir,
    which: &[usize],
) -> usize {
    let game = Game::<W>::of_board(n).unwrap();
    let mut resumes = 0;
    for (i, cfg) in configs().into_iter().enumerate().filter(|(i, _)| which.contains(i)) {
        let cfg = Config { threads: 2, ..cfg };
        let checkpoint = dir.join(format!("n{n}-w{W}-{i}.ckpt"));
        let engine = EngineConfig {
            threads: 2,
            queue_mem_cap: 2,
            spill_dir: Some(dir.join("spill")),
            checkpoint: Some((checkpoint.clone(), Duration::from_millis(1))),
            node_limit: Some(node_limit),
            ..config(2)
        };
        let mut out = game.search(cfg, vec![game.root_unit()], truth - handicap, Some(engine.clone()), None);
        let mut pieces = 1;
        while !out.completed {
            out = game.search(cfg, Vec::new(), 0, Some(engine.clone()), Some(checkpoint.clone()));
            pieces += 1;
            assert!(pieces < 10_000, "n={n} {cfg:?}: no progress");
        }
        resumes += pieces - 1;
        assert!(out.best == truth && out.real, "n={n} {cfg:?}: truth {truth}, got {out:?} after {pieces} pieces");
    }
    resumes
}

/// [`a_search_resumed_over_and_over_finds_the_optimum`] of game 1000 in the first configuration, the default, from an
/// incumbent 300 below its P*.  (The search needs thousands of nodes to be stopped on two threads, and from an
/// incumbent 30 below the default search of 1000 takes fewer than two thousand; another quick check of resuming is
/// `checkpoint_and_resume_with_the_gf2_bound_of_a_small_game`, on one thread, in the unit tests, and
/// `a_small_search_of_the_extended_problem_resumed_over_and_over_finds_the_optimum`.)
#[test]
fn a_small_search_resumed_over_and_over_finds_the_optimum() {
    let dir = TempDir::new("churn-small");
    let truth = optimal_sum::<16>(Optimal::built_in(), 1000).unwrap();
    let resumes = resumed_over_and_over::<16>(1000, truth, 300, 1000, &dir, &[0]);
    assert!(resumes >= 1, "only {resumes} resumes");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is a_small_search_resumed_over_and_over_finds_the_optimum"
)]
fn a_search_resumed_over_and_over_finds_the_optimum() {
    let optimal = Optimal::built_in();
    let dir = TempDir::new("churn");
    // Game 992 takes some ten thousand nodes from an incumbent that far below P* (two threads: the count depends on
    // timing), so a limit of 1,000 stops it several times (the engine checks the limit every 1,024 steps of a searcher)
    // in each of the three configurations: the default, without forced members, and forgetting every step.  The
    // bounds below only check that the resuming really happened; they leave room for the smallest trees a loaded
    // machine produces.
    let truth = optimal_sum::<16>(optimal, 992).unwrap();
    let resumes = resumed_over_and_over::<16>(992, truth, 30, 1000, &dir, &[0, 1, 3]);
    assert!(resumes >= 6, "only {resumes} resumes");
}

/// [`a_wide_search_resumed_over_and_over_finds_the_optimum`] of game 1000 in the first configuration, the default, at
/// W = 20 and 64, from an incumbent 300 below its P*: some four thousand nodes on two threads, so that one of the two
/// workers passes the 1,024 steps at which it checks the limit, and each width resumes at least once.
#[test]
fn a_wide_search_resumed_over_and_over_finds_the_optimum_in_a_quick_run() {
    let truth = optimal_sum::<16>(Optimal::built_in(), 1000).unwrap();
    let dir = TempDir::new("churn-wide-quick");
    let resumes = [
        resumed_over_and_over::<20>(1000, truth, 300, 60, &dir, &[0]),
        resumed_over_and_over::<64>(1000, truth, 300, 60, &dir, &[0]),
    ];
    assert!(resumes.iter().all(|&r| r >= 1), "resumes at W = 20 and 64: {resumes:?}");
}

/// [`a_search_resumed_over_and_over_finds_the_optimum`] at W = 20, 24 and 64: game 992 (some ten thousand nodes), whose
/// boards W = 16 holds, from an incumbent 30 below its P*, stopped every 60 nodes or so (the engine checks every 1,024
/// steps of a searcher), in the default and forgetting every step at W = 20, the default at W = 24 and without forced
/// members at W = 64.  (The search of game 1133, whose board needs W = 20, is too short to stop.)
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is a_wide_search_resumed_over_and_over_finds_the_optimum_in_a_quick_run"
)]
fn a_wide_search_resumed_over_and_over_finds_the_optimum() {
    let truth = optimal_sum::<16>(Optimal::built_in(), 992).unwrap();
    let dir = TempDir::new("churn-wide");
    let resumes = resumed_over_and_over::<20>(992, truth, 30, 60, &dir, &[0, 3])
        + resumed_over_and_over::<24>(992, truth, 30, 60, &dir, &[0])
        + resumed_over_and_over::<64>(992, truth, 30, 60, &dir, &[1]);
    assert!(resumes > 6, "only {resumes} resumes");
}

// ---------------------------------------------------------------------------------------------------------
// The GF(2) bound at nodes of the hardest games

/// At the nodes of the probe's fixtures (`testdata/perf/`: 300 nodes each of the proofs of 936 and 1215, from the root
/// to 30 promotions deep), with an oracle that does not use the bound: every completion made by promoting the node's
/// candidates in a random order, each if it can be, is independent in every frame of the node (the bound of the
/// node with only its members as candidates is their sum) and no heavier than the bound.  (The search of the matching
/// path, started from the bound, was a second oracle, which proved the bound at all but a few of these nodes; no
/// search without the GF(2) bound is left, and an exhaustive one, given a million nodes a node, took minutes for 20 of
/// them.)
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is the_gf2_bound_holds_at_a_sample_of_the_nodes_of_the_probe"
)]
fn the_gf2_bound_holds_at_the_nodes_of_the_probe() {
    let nodes = gf2_at_probe_nodes::<16>("n0936.nodes", 1) + gf2_at_probe_nodes::<20>("n1215.nodes", 1);
    assert!(nodes >= 590, "only {nodes} nodes checked");
}

/// [`the_gf2_bound_holds_at_the_nodes_of_the_probe`] at every 30th node of each fixture.
#[test]
fn the_gf2_bound_holds_at_a_sample_of_the_nodes_of_the_probe() {
    let nodes = gf2_at_probe_nodes::<16>("n0936.nodes", 30) + gf2_at_probe_nodes::<20>("n1215.nodes", 30);
    assert!(nodes >= 20, "only {nodes} nodes checked");
}

/// [`the_gf2_bound_holds_at_the_nodes_of_the_probe`] at every `every`th node of `fixture`: returns the nodes checked.
fn gf2_at_probe_nodes<const W: usize>(fixture: &str, every: usize) -> usize {
    let (n, _, units) = fixtures::read_nodes::<W>(&testdata(&format!("perf/{fixture}"))).unwrap();
    let game = Game::<W>::of_board(n).unwrap();
    let graph = Gf2Graph::new(&game.model, &game.root, &game.candidates);
    let mut bound = Gf2Bound::<W>::new(&graph);
    let mut scratch = Scratch::<W>::new();
    let mut rng = Rng(0x6F2B_C0DE_0000_0000 ^ u64::from(n));
    let mut nodes = 0;
    for unit in units.iter().filter(|u| !u.cands.is_empty()).step_by(every) {
        let state = game.model.promote_all_using(&mut scratch, &game.root, &unit.promoted).expect("a node");
        let max_add = (game.max_total - unit.promoted.len()).min(state.max_promotions_among(&unit.cands).max(0) as u32);
        bound.reset(&unit.promoted, &unit.cands);
        let value = bound.evaluate(max_add, None).value;
        let at = format!("n={n}, a node of {} promotions", unit.promoted.len());
        for _ in 0..4 {
            let mut order = unit.cands.to_vec();
            for i in (1..order.len()).rev() {
                order.swap(i, rng.below(i as u64 + 1) as usize);
            }
            let (mut completion, mut s) = (Bits::<W>::EMPTY, state);
            for x in order {
                if game.model.promote_in_place(&mut s, x) {
                    completion.insert(x);
                }
            }
            assert!(completion.len() <= max_add, "{at}: a completion of {} promotions", completion.len());
            bound.reset(&unit.promoted, &completion);
            let independent = bound.evaluate(completion.len(), None).value == completion.sum();
            assert!(independent && completion.sum() <= value, "{at}: {completion:?} against the bound {value}");
        }
        nodes += 1;
    }
    nodes
}

// ---------------------------------------------------------------------------------------------------------
// Perturbed games

/// A game that the frame builder never makes: a real game's frames with numbers dropped, made moves instead of
/// factors (or the other way round) and moved a frame up or down, or random columns; kept only if every frame of
/// the root is winnable, with at most 36 candidates (mostly those that can be promoted from the root).
fn perturbed(rng: &mut Rng, n: u32, frames: u64, density: u64) -> Option<Game<4>> {
    let tables = Tables::new(n);
    let mut columns = vec![Vec::new(); 2 * frames as usize];
    if rng.chance(70) {
        let real = build_frames::<4>(&tables).columns();
        if real.is_empty() {
            return None;
        }
        columns = vec![Vec::new(); real.len()];
        let len = real.len() / 2;
        for (c, col) in real.iter().enumerate() {
            for &x in col {
                let (mut f, mut as_move) = (c / 2, c % 2 == 1);
                if rng.chance(100 - density) {
                    continue;
                }
                if rng.chance(3) {
                    as_move = !as_move;
                }
                if rng.chance(3) {
                    f = if rng.chance(50) { f.saturating_sub(1) } else { (f + 1).min(len - 1) };
                }
                columns[2 * f + usize::from(as_move)].push(x);
            }
        }
        for col in &mut columns {
            col.sort_unstable();
        }
    } else {
        for x in 2..=n {
            if rng.chance(density) {
                let f = rng.below(frames) as usize;
                columns[2 * f + usize::from(rng.chance(50))].push(x);
            }
        }
    }
    let (model, root) = FrameGraph::build::<4>(&tables, &columns).ok()?;
    if !root.frames().iter().all(|f| model.winnable(&f.moves, &f.factors)) {
        return None;
    }
    let mut candidates = root.frames()[1..].iter().fold(Bits::EMPTY, |all, f| all | f.factors);
    if rng.chance(70) {
        candidates = candidates.iter().filter(|&x| model.can_promote(&root, x)).collect();
    }
    (!candidates.is_empty() && candidates.len() <= 36).then(|| Game::new(model, root, candidates))
}

/// The units of a random path down the tree of `game`: (promoted, candidates, state) at the root and after up to
/// three random promotions.
fn path<const W: usize>(rng: &mut Rng, game: &Game<W>) -> Vec<(Bits<W>, Bits<W>, FrameSet<W>)> {
    let mut units = vec![(Bits::EMPTY, game.candidates, game.root)];
    let (mut state, mut promoted, mut cands) = (game.root, Bits::EMPTY, game.candidates);
    for _ in 0..3 {
        let members = cands.to_vec();
        if members.is_empty() {
            break;
        }
        let x = rng.pick(&members);
        cands.remove(x);
        if let Some(next) = game.model.promote(&state, x) {
            state = next;
            promoted.insert(x);
            cands = state.narrow_candidates(&cands);
            units.push((promoted, cands, state));
        }
    }
    units
}

/// `count` perturbed games, their roots and some units below them, with the truth found exhaustively, under random
/// tactics, from an incumbent worth `k` less, drawn with `seed`: more than 16 searches for each game are made.
fn perturbed_games_are_searched_exactly(count: usize, seed: u64) {
    let mut rng = Rng(seed);
    let configs = configs();
    let (mut games, mut runs) = (0, 0);
    while games < count {
        let n = 30 + rng.below(220) as u32;
        let (frames, density) = (2 + rng.below(6), 94 + rng.below(7));
        let Some(game) = perturbed(&mut rng, n, frames, density) else { continue };
        let truth = game.exhaustive(&game.root, &game.candidates, 2_000_000);
        if truth.is_none_or(|(truth, _)| truth == 0) {
            continue;
        }
        games += 1;
        let sym = game.symmetries();
        for (promoted, cands, state) in path(&mut rng, &game) {
            let unit = TaxUnit { promoted, cands };
            let Some(searched) = Searched::of(&game, &sym, &state, unit, 2_000_000) else { continue };
            let at = format!("n={n} frames {:?} promoted {promoted:?} cands {cands:?}", game.root);
            for _ in 0..6 {
                let cfg = rng.pick(&configs);
                let k = rng.pick(&[1u64, 1, 2, 3, 9, 30, 1000]);
                runs += usize::from(searched.run(cfg, k, &at));
            }
        }
    }
    assert!(runs > 16 * count, "{runs} runs");
}

/// Sixty perturbed games, their roots and some units below them, searched exactly.
#[test]
fn some_perturbed_frame_sets_are_searched_exactly() {
    perturbed_games_are_searched_exactly(60, 0x5EED_0003_C0DE_F00D);
}

/// Perturbed games, their roots and some units below them, with the truth found exhaustively, under random
/// tactics, from an incumbent worth `k` less.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is some_perturbed_frame_sets_are_searched_exactly")]
fn perturbed_frame_sets_are_searched_exactly() {
    perturbed_games_are_searched_exactly(500, 0x5EED_0001_C0DE_F00D);
}

/// On perturbed games: promoting a set at once is promoting it one by one in any order (success and the state it
/// leaves), from the root and from random states; a conflict is a set that cannot be promoted; and the nogood made
/// of it is unpromotable from the root and made only of the conflict and the promotions before.  These are what
/// the search's reuse of nogoods across nodes, threads and resumes rests on.
#[test]
fn promotion_order_conflicts_and_nogoods_on_perturbed_frame_sets() {
    let mut rng = Rng(0x0DE5_0002_C0DE_F00D);
    let mut scratch = Scratch::<4>::new();
    let (mut games, mut orders, mut promotable, mut conflicts) = (0, 0, 0, 0);
    while games < 40 {
        let n = 30 + rng.below(220) as u32;
        let (frames, density) = (2 + rng.below(6), 94 + rng.below(7));
        let Some(game) = perturbed(&mut rng, n, frames, density) else { continue };
        games += 1;
        let (model, root) = (&game.model, game.root);
        let all: Bits<4> = root.frames()[1..].iter().fold(Bits::EMPTY, |all, f| all | f.factors);
        for _ in 0..20 {
            // A random state to start from, and a random set to promote there.
            let mut start = root;
            let mut before = Bits::EMPTY;
            for x in all.iter().filter(|_| rng.chance(10)) {
                if model.promote_in_place(&mut start, x) {
                    before.insert(x);
                }
            }
            let set = random_subset(&mut rng, &(all - before), 25);
            let at_once = model.promote_all_using(&mut scratch, &start, &set);
            let from_root = model.promote_all_using(&mut scratch, &root, &(set | before));
            for _ in 0..3 {
                let mut order = set.to_vec();
                for i in (1..order.len()).rev() {
                    order.swap(i, rng.below(i as u64 + 1) as usize);
                }
                let mut state = start;
                let ok = order.iter().all(|&x| model.promote_in_place(&mut state, x));
                orders += 1;
                assert!(ok == at_once.is_some() && ok == from_root.is_some(), "n={n} {before:?} + {order:?}: {ok}");
                assert!(!ok || at_once == Some(state), "n={n} {before:?} + {order:?}: not the same state");
            }
            promotable += usize::from(at_once.is_some());
            if let Some(k) = model.conflict_using(&mut scratch, &start, &set, true) {
                conflicts += 1;
                assert!(model.promote_all_using(&mut scratch, &start, &k).is_none(), "n={n}: {k:?} is no conflict");
                let nogood = model.nogood(&mut scratch, &root, &before, &k);
                let sound = nogood.is_some_and(|ng| {
                    ng.is_subset(&(before | k)) && model.promote_all_using(&mut scratch, &root, &ng).is_none()
                });
                assert!(sound, "n={n}: the nogood {nogood:?} of {k:?} after {before:?}");
            }
        }
    }
    assert!(
        orders > 1500 && promotable > 50 && conflicts > 100,
        "{orders} orders, {promotable} promotable, {conflicts}"
    );
}

/// The conjecture that the promotion set with the largest sum also has the most promotions is false for the whole frame
/// game of n = 833, the first board whose optimal game is not a longest one (`solver/noframe/docs/longest-games.md`):
/// the optimal set promotes 48 numbers, as many as `max_total` allows less one, and a set the model accepts promotes
/// 49, four large promotions of the optimal set exchanged for five small ones; both games are over, the second one move
/// longer and 29 points lighter.  (The Java solver's revision of the freedoms assumed the conjecture of the two-frame
/// game of frames 0 and 1, which this does not decide.)
#[test]
fn the_heaviest_promotion_set_of_833_is_not_the_largest() {
    const W: usize = 16;
    let board = Board::<W>::for_n(833);
    let game = Game::<W>::of_board(833).expect("a game");
    let optimal: Bits<W> = [
        245, 250, 252, 258, 261, 273, 280, 288, 294, 297, 300, 306, 310, 315, 322, 330, 333, 336, 338, 342, 343, 345,
        350, 357, 363, 364, 368, 369, 372, 375, 376, 378, 380, 385, 387, 390, 392, 396, 399, 400, 402, 405, 406, 408,
        410, 412, 414, 416,
    ]
    .into_iter()
    .collect();
    let mut longer = optimal;
    [250, 252, 294, 400].into_iter().for_each(|x| longer.remove(x));
    [116, 180, 200, 320, 351].into_iter().for_each(|x| longer.insert(x));
    assert_eq!((optimal.len(), longer.len(), game.max_total), (48, 49, 49));
    assert!(game.is_real(&optimal, 16692) && game.is_real(&longer, 16663));
    for (set, moves, score) in [(optimal, 362, 218226), (longer, 363, 218197)] {
        let played = taxman::accelerate::solution_for_promotions(&board.tables, set);
        let refereed = taxman::game::replay_finished(833, &played).map(|s| (played.len(), s));
        assert_eq!(refereed, Ok((moves, score)));
    }
}

// ---------------------------------------------------------------------------------------------------------
// The extended problem: one-sided candidates, searched in extended mode

/// The configurations of extended mode, the search of a problem with one-sided candidates: the tactics of
/// [`SearchTactics::extended`] (the default without the rule of the symmetric pairs, which such a search refuses, and
/// with the GF(2) bound it needs, the reuse of prices and the warm start) on one thread; on two; with the nogoods
/// forgotten at every step; without forced members; without nogoods; and with conflict branching at every check,
/// whatever its gap, which the default gate leaves to the largest games.
fn extended_configs() -> Vec<Config> {
    let e = Config::of(&SearchTactics::extended());
    vec![
        e,
        Config { threads: 2, ..e },
        Config { period: Some(1), ..e },
        Config { fix_forced: false, ..e },
        Config { nogoods: false, ..e },
        Config { conflict_branching: Some(0), ..e },
    ]
}

/// The table's best promotion sum of the extended problem `ext` of game `n`, whatever sources are open: its score
/// less what the sources fixed in `ext` make (U holds up to 1300: some optimal game plays all of U\*, so the heaviest
/// promotion set of Π⁺(O) plays the open sources too).
fn extended_optimum<const W: usize>(optimal: &Optimal, n: u32, ext: &ExtendedFrames<W>) -> u64 {
    optimal.score(n).expect("a known game") - ext.in_the_bag
}

/// Every board 2..=300 searched as the extended problem Π⁺(O) from the root, from an incumbent worth 1 or 5 less than
/// The table's promotion sum of it and an empty payload, under each configuration of extended mode: the search
/// must complete with exactly that sum and a set that is promotable.  The sets O are the empty set, all of U\*_{≥3}
/// (the boards to 120, where the problem has dozens of one-sided candidates) and three random non-empty subsets of it.
/// Past the board 200 one configuration in three runs, as in the game tests above.  Unlike them it searches games whose
/// candidates are not all numbers of a frame: the one-sided candidates, each a move of its own frame.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is a_search_of_the_extended_problems_of_the_boards_2_to_42_from_a_handicapped_incumbent_finds_the_optimum \
              and a_search_of_the_extended_problems_of_the_boards_43_to_70_from_a_handicapped_incumbent_finds_the_optimum"
)]
fn a_search_of_the_extended_problem_from_a_handicapped_incumbent_finds_the_optimum() {
    // There were 1,258 problems, 978 of them with one-sided candidates, and 10,180 runs, 7,980 of them on those.
    extended_searches_from_a_handicapped_incumbent(2..=300, 0xE87E_4DED_5EED_0001, [1100, 850, 8_800, 6_900]);
}

/// [`a_search_of_the_extended_problem_from_a_handicapped_incumbent_finds_the_optimum`] on the boards 2..=42.  The small
/// boards are two tests, so that their thousands of searches (each one starts its threads and makes its searchers'
/// memory) run side by side.
#[test]
fn a_search_of_the_extended_problems_of_the_boards_2_to_42_from_a_handicapped_incumbent_finds_the_optimum() {
    extended_searches_from_a_handicapped_incumbent(2..=42, 0xE87E_4DED_5EED_0002, [75, 50, 500, 400]);
}

/// The same on the boards 43..=70, with a stream of random sets of its own.
#[test]
fn a_search_of_the_extended_problems_of_the_boards_43_to_70_from_a_handicapped_incumbent_finds_the_optimum() {
    extended_searches_from_a_handicapped_incumbent(43..=70, 0xE87E_4DED_5EED_0003, [75, 50, 500, 400]);
}

/// The searches of the extended problems of the boards `ns`, drawn with `seed` (see
/// [`a_search_of_the_extended_problem_from_a_handicapped_incumbent_finds_the_optimum`]); more than `fewest` problems,
/// of them with one-sided candidates, searches, and searches on those.
fn extended_searches_from_a_handicapped_incumbent(ns: std::ops::RangeInclusive<u32>, seed: u64, fewest: [usize; 4]) {
    let optimal = Optimal::built_in();
    let configs = extended_configs();
    let mut rng = Rng(seed);
    let (mut problems, mut with_one_sided, mut runs, mut one_sided_runs) = (0, 0, 0, 0);
    for n in ns {
        let board = Board::<8>::for_n(n);
        let from_3 = upper_from_3(&board);
        let mut opens = vec![Vec::new()];
        if n <= 120 && !from_3.is_empty() {
            opens.push(from_3.clone());
        }
        opens.extend(random_open_sets(&mut rng, &from_3, 3, 20));
        for open in opens {
            let (ext, prepared) = board
                .prepare_extended_with(&Bits::from_slice(&open), false, SearchTactics::extended())
                .unwrap_or_else(|e| panic!("n={n}: O={open:?}: {e}"));
            if prepared.candidates.is_empty() {
                continue;
            }
            let game = Game::of_prepared(&prepared);
            let truth = extended_optimum(optimal, n, &ext);
            let one_sided = prepared.model.has_one_sided();
            problems += 1;
            with_one_sided += usize::from(one_sided);
            for (i, cfg) in configs.iter().enumerate() {
                for k in [1, 5] {
                    if k > truth || (n > 200 && i % 3 != 0) {
                        continue;
                    }
                    let out = game.search(*cfg, vec![game.root_unit()], truth - k, None, None);
                    runs += 1;
                    one_sided_runs += usize::from(one_sided);
                    let at = format!("n={n} O={open:?} {cfg:?} from {}: truth {truth}", truth - k);
                    assert!(out.completed, "{at}: not completed");
                    assert!(out.best == truth && out.real, "{at}, got {out:?}");
                }
            }
        }
    }
    println!("{problems} problems, {with_one_sided} with one-sided candidates, {runs} runs, {one_sided_runs} of them");
    let [few_problems, few_one_sided, few_runs, few_one_sided_runs] = fewest;
    assert!(
        problems > few_problems && with_one_sided > few_one_sided,
        "{problems} problems, {with_one_sided} one-sided"
    );
    assert!(runs > few_runs && one_sided_runs > few_one_sided_runs, "{runs} runs, {one_sided_runs} on one-sided ones");
}

/// The harness's own exhaustive oracle on the extended problem: [`Game::exhaustive`] on the game of Π⁺(O), whose counts
/// of promotions and narrowing of candidates come from the model that knows the one-sided candidates, finds
/// The table's promotion sum and a set that is real, on the boards to 60 for O empty, all of U\*_{≥3} and two
/// random subsets of it.  (The perturbed games above are held to this oracle, so it must be right where the search is
/// run.)
#[test]
fn the_exhaustive_oracle_finds_the_optimum_of_the_extended_problem() {
    let optimal = Optimal::built_in();
    let mut rng = Rng(0x0AC1_E0F5_0E27_0001);
    let (mut finished, mut with_one_sided, mut skipped) = (0, 0, 0);
    for n in 2..=60 {
        let board = Board::<8>::for_n(n);
        let from_3 = upper_from_3(&board);
        let mut opens = vec![Vec::new(), from_3.clone()];
        opens.extend(random_open_sets(&mut rng, &from_3, 2, 8));
        for open in opens {
            let (ext, prepared) =
                board.prepare_extended_with(&Bits::from_slice(&open), false, SearchTactics::extended()).unwrap();
            if prepared.candidates.is_empty() {
                continue;
            }
            let game = Game::of_prepared(&prepared);
            let Some((best, set)) = game.exhaustive(&game.root, &game.candidates, 3_000_000) else {
                skipped += 1;
                continue;
            };
            let truth = extended_optimum(optimal, n, &ext);
            assert!(
                best == truth && game.is_real(&set, best),
                "n={n} O={open:?}: truth {truth}, got {best} with {set:?}"
            );
            finished += 1;
            with_one_sided += usize::from(prepared.model.has_one_sided());
        }
    }
    println!("{finished} problems, {with_one_sided} with one-sided candidates, {skipped} past the budget");
    assert!(finished > 150 && with_one_sided > 100, "{finished} problems, {with_one_sided} with one-sided candidates");
}

/// The game and the seed for the open sources of
/// [`a_search_of_the_extended_problem_resumed_over_and_over_finds_the_optimum`]: a random 20 of the game's U\*_{≥3},
/// drawn with the seed.  The extended search of 992 with them takes 16,717 steps on one thread (of the games from 600
/// to 1000 only 990 to 998 take more than 10,000; the search of 1000 takes 2,293 steps, that of 988 6,273).
const EXTENDED_RESUME: (u32, u64) = (992, 0x5EED_0E27_E4DE_D001);

/// The same for [`a_small_search_of_the_extended_problem_resumed_over_and_over_finds_the_optimum`].
const QUICK_RESUME: (u32, u64) = (800, 0x5EED_0E27_E4DE_D001);

/// The extended problem of game `EXTENDED_RESUME.0`, with a random 20 of its U\*_{≥3} open, searched on one thread in
/// pieces of 1,000 nodes, each resumed from the checkpoint the one before it left, ends with the sum of an
/// uninterrupted search, which is the table's.  The sets are the search's own (`Board::prepare_extended_with`,
/// `Board::prepare_extended_for_resume`, `search::search`), so the checkpoints are the ones the real search writes:
/// their header records the one-sided candidates, and the resume checks them.  The engine checks a node limit every
/// 1,024 steps of a searcher, so a piece is at least that long.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is a_small_search_of_the_extended_problem_resumed_over_and_over_finds_the_optimum"
)]
fn a_search_of_the_extended_problem_resumed_over_and_over_finds_the_optimum() {
    let (n, seed) = EXTENDED_RESUME;
    an_extended_search_resumed_over_and_over(n, seed, 10_000);
}

/// [`a_search_of_the_extended_problem_resumed_over_and_over_finds_the_optimum`] of a smaller game, `QUICK_RESUME`.
#[test]
fn a_small_search_of_the_extended_problem_resumed_over_and_over_finds_the_optimum() {
    let (n, seed) = QUICK_RESUME;
    an_extended_search_resumed_over_and_over(n, seed, 1_200);
}

/// The extended problem of game `n`, with a random 20 of its U\*_{≥3} open drawn with `seed`, searched in pieces of
/// 1,000 nodes (see [`a_search_of_the_extended_problem_resumed_over_and_over_finds_the_optimum`]); its search takes
/// more than `most_nodes` nodes uninterrupted, so that the limit stops it.
fn an_extended_search_resumed_over_and_over(n: u32, seed: u64, most_nodes: u64) {
    let board = Board::<16>::for_n(n);
    let mut open = upper_from_3(&board);
    common::shuffle(&mut Rng(seed), &mut open);
    open.truncate(20);
    open.sort_unstable();
    let open = Bits::<16>::from_slice(&open);
    let (ext, prepared) = board.prepare_extended_with(&open, false, SearchTactics::extended()).unwrap();
    assert!(prepared.model.has_one_sided() && ext.open == open, "n={n}: the open sources");
    let truth = extended_optimum(Optimal::built_in(), n, &ext);

    let whole = search(&prepared, &config(1), None, None, false).unwrap();
    assert!(whole.completed && whole.sum == truth, "n={n}: uninterrupted: {} not {truth}", whole.sum);
    assert!(
        whole.stats.nodes > most_nodes,
        "n={n}: only {} nodes, too few for the limit to stop it",
        whole.stats.nodes
    );

    let dir = TempDir::new("extended-resume");
    let checkpoint = dir.join("n.ckpt");
    let cfg = EngineConfig {
        checkpoint: Some((checkpoint.clone(), Duration::from_secs(3600))),
        node_limit: Some(1000),
        queue_mem_cap: 4,
        spill_dir: Some(dir.join("spill")),
        ..config(1)
    };
    let mut result = search(&prepared, &cfg, None, None, false).unwrap();
    let mut pieces = 1;
    while !result.completed {
        assert!(checkpoint.exists(), "n={n}: a stopped run leaves a checkpoint");
        let (resumed_ext, resumed) = board.prepare_extended_for_resume(&open, false, &checkpoint).unwrap();
        assert_eq!((resumed_ext.one_sided, resumed.tactics), (ext.one_sided.clone(), prepared.tactics));
        assert_eq!(resumed.problem(), prepared.problem(), "n={n}: the problem set up from the checkpoint");
        result = search(&resumed, &cfg, None, None, true).unwrap();
        pieces += 1;
        assert!(pieces < 100, "n={n}: no progress");
    }
    assert!(pieces >= 2, "n={n}: only {pieces} pieces");
    assert!(!checkpoint.exists(), "a completed run deletes its checkpoint");
    assert_eq!(result.sum, whole.sum, "n={n}: {pieces} pieces against the uninterrupted search");
    assert_eq!(result.sum, truth);
    let game = Game::of_prepared(&prepared);
    assert!(game.is_real(&result.promotions, result.sum), "n={n}: the set {:?}", result.promotions);
    println!("n={n}: {} nodes uninterrupted, {pieces} pieces of at least 1,000 nodes", whole.stats.nodes);
}
