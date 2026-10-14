//! Adversarial soundness tests of the search: could a pruning, a shortcut or a resume lose a better promotion set?
//!
//! The key idea is the handicapped incumbent.  The engine is started with a fake incumbent worth less than the best
//! completion (`truth − k`) and an empty payload.  If any pruning step loses every best completion, the fake one
//! survives, and its payload does not match its value.  This tests every pruning at once, including all the `<`
//! and `<=` boundaries: from `truth − 1`, every node whose bound is exactly `truth` must be explored.  `truth` is
//! `optimal.json`'s promotion sum at the root of a real game, and an exhaustive search for perturbed games and
//! units deep in the tree.

mod common;
#[allow(dead_code)]
#[path = "../examples/common/fixtures.rs"]
mod fixtures;

use common::{config, random_subset, stored_games, testdata, Config, Game, Rng, TempDir};
use std::cell::OnceCell;
use std::time::Duration;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::EngineConfig;
use taxman::frames::build_frames;
use taxman::game::Optimal;
use taxman::gf2::{Gf2Bound, Gf2Graph};
use taxman::model::{FrameSet, Model, Scratch};
use taxman::numtheory::Tables;
use taxman::search::lagrange::Refits;
use taxman::search::symmetry::Symmetries;
use taxman::search::{Gf2Branching, SymmetryBound, TaxUnit};

/// Every tactic that acts inside the search, alone and in the combinations that matter: first the matching path
/// ([`SearchTactics::matching`], 0 to 18), then the default, the GF(2) bound (19 to 29), then the rule of the root's
/// symmetric pairs (tactic `symmetry`, 30 to 35).  The matching path with the
/// forgetting of nogoods at the production period, every step, every third, and never; on one thread and two; and
/// the GF(2) bound with each way of choosing the branch, with and without the matching bound as a first check,
/// forced members, nogoods and the Lagrangian bound's early refits.  The Lagrangian
/// bound (tactic `lagrange`) refits its multipliers only after 2,000 steps, which most of these searches never
/// take, so some configurations refit them after the first few steps and every few steps after that (with and
/// without forgetting at the same time, refinement, and a second thread), and one turns it off.  The rule of the
/// symmetric pairs runs over the GF(2) bound, on two threads, with the nogoods forgotten at every step, and over the
/// matching path; the Lagrangian relaxation of the pairs' inequalities on the GF(2) bound (the knob `symmetry_bound`)
/// runs for each pair alone, and for the broken pairs together on two threads.
fn configs() -> Vec<Config> {
    let (base, g) = (Config::matching(), Config::standard());
    vec![
        base,
        Config { period: Some(1), ..base },
        Config { period: Some(3), ..base },
        Config { period: None, ..base },
        Config { refine: 0, ..base },
        Config { refine: 2, ..base },
        Config { refine: 4, ..base },
        Config { fix_forced: false, ..base },
        Config { fix_forced: false, refine: 0, ..base },
        Config { nogoods: false, ..base },
        Config { nogoods: false, refine: 0, ..base },
        Config { threads: 2, ..base },
        Config { threads: 2, period: Some(2), ..base },
        Config { threads: 2, refine: 0, ..base },
        Config { refits: Some(Refits { first: 1, every: 4 }), ..base },
        Config { refits: Some(Refits { first: 3, every: 9 }), period: Some(9), ..base },
        Config { refits: Some(Refits { first: 1, every: 4 }), refine: 0, ..base },
        Config { refits: Some(Refits { first: 2, every: 5 }), threads: 2, ..base },
        Config { refits: None, ..base },
        g,
        Config { gf2_branching: Gf2Branching::Split, ..g },
        Config { gf2_branching: Gf2Branching::Largest, ..g },
        Config { gf2_branching: Gf2Branching::Forced, ..g },
        Config { gf2_prefilter: true, ..g },
        Config { fix_forced: false, ..g },
        Config { nogoods: false, ..g },
        Config { period: Some(1), ..g },
        Config { threads: 2, ..g },
        Config { refits: Some(Refits { first: 1, every: 4 }), ..g },
        Config { refits: Some(Refits { first: 2, every: 5 }), threads: 2, period: Some(3), ..g },
        Config { symmetry: true, ..g },
        Config { symmetry: true, threads: 2, ..g },
        Config { symmetry: true, period: Some(1), ..g },
        Config { symmetry: true, ..base },
        Config { symmetry: true, symmetry_bound: SymmetryBound::Pair, ..g },
        Config { symmetry: true, symmetry_bound: SymmetryBound::Joint, threads: 2, ..g },
    ]
}

/// `optimal.json`'s best promotion sum of game `n`: its score less what the game with no promotions scores.
fn optimal_sum<const W: usize>(optimal: &Optimal, n: u32) -> Option<u64> {
    let in_the_bag = Board::<W>::for_n(n).in_the_bag;
    optimal.score(n).map(|score| score - in_the_bag)
}

// ---------------------------------------------------------------------------------------------------------
// Real games

/// Every game 2..=300 that has candidates, from an incumbent worth 1 or 5 less than `optimal.json`'s promotion sum,
/// under each tactic: the search must come back with exactly that sum, and with a set that is promotable.  Unlike
/// the golden tests, which start from the greedy set, this starts from a weak incumbent.
#[test]
fn a_search_from_a_handicapped_incumbent_finds_the_optimum() {
    let optimal = Optimal::built_in();
    let configs = configs();
    let mut runs = 0;
    for n in 2..=300 {
        let Some(game) = Game::<8>::of_board(n) else { continue };
        let truth = optimal_sum::<8>(optimal, n).unwrap();
        // The big trees are for one configuration of each kind; the small ones for all.
        for (i, cfg) in configs.iter().enumerate() {
            for k in [1, 5] {
                if k > truth || (n > 200 && i % 3 != 0) {
                    continue;
                }
                let out = game.search(*cfg, vec![game.root_unit()], truth - k, None, None);
                runs += 1;
                assert!(out.completed, "n={n} {cfg:?} from {}: not completed", truth - k);
                assert!(out.best == truth && out.real, "n={n} {cfg:?} from {}: truth {truth}, got {out:?}", truth - k);
            }
        }
    }
    assert!(runs > 4000, "{runs} runs");
}

/// Games 301 to 600 (every tenth), whose searches take a few hundred to a few thousand steps, from an incumbent
/// worth 1 or 5 less than `optimal.json`'s promotion sum, with the Lagrangian bound refitted early and often (with
/// the nogoods forgotten at the same time, and on two threads): in the games above, few searches are long enough
/// for its multipliers to have much to fit.
#[test]
fn a_search_with_the_lagrangian_bound_from_a_handicapped_incumbent_finds_the_optimum() {
    let optimal = Optimal::built_in();
    // Over the matching bound, and over the GF(2) bound.
    let (base, g) = (Config::matching(), Config::standard());
    let configs = [
        Config { refits: Some(Refits { first: 10, every: 60 }), ..base },
        Config { refits: Some(Refits { first: 25, every: 100 }), period: Some(100), ..base },
        Config { refits: Some(Refits { first: 10, every: 60 }), threads: 2, ..base },
        Config { refits: Some(Refits { first: 10, every: 60 }), ..g },
        Config { refits: Some(Refits { first: 25, every: 100 }), period: Some(100), threads: 2, ..g },
    ];
    let runs = lagrangian_runs::<8>(optimal, 301..=511, &configs) + lagrangian_runs::<16>(optimal, 512..=600, &configs);
    assert!(runs > 250, "{runs} runs");
}

/// The searches of [`a_search_with_the_lagrangian_bound_from_a_handicapped_incumbent_finds_the_optimum`] of the
/// games `ns` (every tenth of them) at width `W`.
fn lagrangian_runs<const W: usize>(optimal: &Optimal, ns: std::ops::RangeInclusive<u32>, configs: &[Config]) -> usize {
    let mut runs = 0;
    for n in ns.filter(|n| n % 10 == 1) {
        let Some(game) = Game::<W>::of_board(n) else { continue };
        let truth = optimal_sum::<W>(optimal, n).unwrap();
        for cfg in configs {
            for k in [1, 5] {
                let out = game.search(*cfg, vec![game.root_unit()], truth - k, None, None);
                runs += 1;
                assert!(out.completed, "n={n} {cfg:?} from {}: not completed", truth - k);
                assert!(out.best == truth && out.real, "n={n} {cfg:?} from {}: truth {truth}, got {out:?}", truth - k);
            }
        }
    }
    runs
}

/// A unit below the root of a game, searched from a handicapped incumbent, with what an exhaustive search found in it.
///
/// Theorem 13 (heuristics/THEORY.md, "Prime exchanges") says that every heaviest promotion set of the *root* keeps
/// the inequality of each symmetric pair, and the search with the tactic `symmetry` looks at the completions that
/// keep them, at every node.  A unit below the root is no root: its best completion can break an inequality (some
/// promotion of its `promoted` may even break one already, which no completion mends), and its search finds the best
/// completion that keeps them ("Pruning with the inequalities", last paragraph).  So a search with the rule is
/// compared with the oracle for that, [`Game::exhaustive_keeping`], and the others with [`Game::exhaustive`].
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
/// (tactic `symmetry`) over the GF(2) bound and over the matching bound, from an incumbent 1 and 5 below
/// `optimal.json`'s promotion sum: the search must complete with exactly that sum and a promotable set (Theorem 13:
/// the root's heaviest set keeps the inequalities, so a search from the root loses nothing).  For the boards to 40 the
/// optimum is also checked against the exhaustive search without the frame model.
#[test]
fn the_symmetry_rule_finds_the_optimum_of_the_boards_with_pairs() {
    let optimal = Optimal::built_in();
    let (base, g) = (Config::matching(), Config::standard());
    let configs = [
        Config { symmetry: true, ..g },
        Config { symmetry: true, ..base },
        Config { symmetry: true, symmetry_bound: SymmetryBound::Pair, ..g },
        Config { symmetry: true, symmetry_bound: SymmetryBound::Joint, threads: 2, ..g },
    ];
    let boards =
        boards_with_pairs::<8>(optimal, 2..=511, &configs) + boards_with_pairs::<16>(optimal, 512..=600, &configs);
    assert!(boards >= 450, "{boards} boards with pairs");
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
        let Some(game) = Game::<W>::of_board(n) else { continue };
        if game.symmetries().is_empty() {
            continue;
        }
        boards += 1;
        let truth = optimal_sum::<W>(optimal, n).unwrap();
        if n <= 40 {
            let exact = taxman::brute::solve(n, &[], taxman::brute::Rule::Divisor).score;
            assert_eq!(Board::<W>::for_n(n).in_the_bag + truth, exact, "n={n}: the optimum against the oracle");
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

/// A random unit of `game`: the node after up to 14 random promotions (the candidates narrowed as the search narrows
/// them), with its candidates thinned to 6 to 23 (the largest, or at random) so that the truth can be found
/// exhaustively.  Returns the state the unit's promotions reach and the unit, or `None` if no candidate is left.
fn random_unit<const W: usize>(game: &Game<W>, rng: &mut Rng) -> Option<(FrameSet<W>, TaxUnit<W>)> {
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
        return None;
    }
    if rng.chance(50) {
        members.reverse();
    } else {
        for i in (1..members.len()).rev() {
            members.swap(i, rng.below(i as u64 + 1) as usize);
        }
    }
    members.truncate(6 + rng.below(18) as usize);
    Some((state, TaxUnit { promoted, cands: members.into_iter().collect() }))
}

/// Units of game `n` at width `W` ([`random_unit`]), searched from an incumbent worth 1, 2, 5 or 30 less under
/// random `configs`: the search must come back with the truth (with the tactic `symmetry`, as [`Searched::run`]
/// says).  Returns the number of searches.
fn handicapped_units<const W: usize>(n: u32, rng: &mut Rng, units: usize, configs: &[Config]) -> usize {
    let game = Game::<W>::of_board(n).expect("a game");
    let sym = game.symmetries();
    let (mut runs, mut made) = (0, 0);
    while made < units {
        let Some((state, unit)) = random_unit(&game, rng) else { continue };
        let Some(searched) = Searched::of(&game, &sym, &state, unit, 3_000_000) else { continue };
        made += 1;
        let at = format!("n={n} W={W} promoted {:?} cands {:?}", unit.promoted, unit.cands);
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

/// Units on the path to an optimal promotion set `S*` of game `n` at width `W`: the promotions are `S*` less a random
/// 30, 50 or 70 percent of its members, the candidates what the search leaves them, searched under each of `configs`
/// from an incumbent one below the optimum.  A pruning that is valid keeps `S*` (it keeps every pair's inequality,
/// Theorem 13, and is a bound above every completion), so each search must find the optimum; one that cuts a node
/// on the path to `S*` does not.  The nodes of such a search are deep enough that the promotions and the GF(2)
/// bound's heaviest set break the pairs' inequalities, which a game searched from its root rarely does.  Returns the
/// number of searches.
fn optimum_units<const W: usize>(optimal: &Optimal, n: u32, rng: &mut Rng, units: usize, configs: &[Config]) -> usize {
    let game = Game::<W>::of_board(n).expect("a game");
    let (truth, sym) = (optimal_sum::<W>(optimal, n).unwrap(), game.symmetries());
    let moves = optimal.moves(n).expect("a known game");
    let best: Bits<W> = moves.iter().copied().filter(|&x| game.candidates.contains(x)).collect();
    if best.sum() != truth || sym.is_empty() {
        return 0;
    }
    let (mut runs, mut scratch) = (0, Scratch::new());
    for _ in 0..units {
        let percent = rng.pick(&[30, 50, 70]);
        let promoted: Bits<W> = best.iter().filter(|_| !rng.chance(percent)).collect();
        // A unit that already holds the optimum has nothing to find: the search offers only what a child promotes.
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

/// The searches of the units on the way to an optimal set ([`optimum_units`]) of the games 600 to 1000 (every 23rd),
/// under the rule of the symmetric pairs alone and with the Lagrangian bound of the pairs (the knob
/// `symmetry_bound`, both ways), over the GF(2) bound: where a wrong bound of the pairs would cut the optimum (a
/// bound lowered by 30 does).
#[test]
fn units_on_the_way_to_the_optimum_find_it_with_the_symmetry_bound() {
    let optimal = Optimal::built_in();
    let g = Config::standard();
    let configs = [
        Config { symmetry: true, ..g },
        Config { symmetry: true, symmetry_bound: SymmetryBound::Pair, ..g },
        Config { symmetry: true, symmetry_bound: SymmetryBound::Joint, ..g },
    ];
    let mut rng = Rng(0x5B0D_1E57_C0DE_0003);
    let runs: usize = (600..=1000).step_by(23).map(|n| optimum_units::<16>(optimal, n, &mut rng, 6, &configs)).sum();
    assert!(runs > 200, "{runs} runs");
}

/// What [`lagrangian_bounds_on`] checked.
#[derive(Debug, Default)]
struct BoundTally {
    /// Units with a completion that keeps every inequality, and the bounds checked against it.
    units: usize,
    bounds: usize,
    /// Bounds below twice the GF(2) bound of the unit (the relaxation tightens it), and bounds equal to twice the
    /// best completion that keeps the inequalities (it is exact).
    below_plain: usize,
    exact: usize,
}

/// The units ([`random_unit`]) of game `n` at width `W`, `units` of those with a completion that keeps every pair's
/// inequality that the exhaustive search finds: the Lagrangian bound of the pairs ([`Symmetries::lagrangian`]) is at
/// least twice the sum of the best such completion, for each pair alone and for all the pairs together, with the
/// multipliers 1 and 2, at the node's GF(2) level and its limit of promotions.
fn lagrangian_bounds_on<const W: usize>(n: u32, rng: &mut Rng, units: usize, tally: &mut BoundTally) {
    let game = Game::<W>::of_board(n).expect("a game");
    let sym = game.symmetries();
    if sym.is_empty() {
        return;
    }
    let graph = Gf2Graph::new(&game.model, &game.root, &game.candidates);
    let mut bound = Gf2Bound::<W>::new(&graph);
    let mut scratch = sym.scratch();
    let pairs: Vec<usize> = (0..sym.pairs().len()).collect();
    let (mut made, mut tried) = (0, 0);
    while made < units && tried < 20 * units {
        tried += 1;
        let Some((state, unit)) = random_unit(&game, rng) else { continue };
        let Some(Some((best, _))) = game.exhaustive_keeping(&state, &unit.promoted, &unit.cands, 1_000_000, &sym)
        else {
            continue;
        };
        made += 1;
        bound.reset(&unit.promoted, &unit.cands);
        let limit = (game.max_total - unit.promoted.len()).min(state.max_promotions_among(&unit.cands).max(0) as u32);
        let plain = 2 * bound.evaluate(limit, None).value as i64;
        let kept = 2 * best as i64;
        let each_alone = pairs.iter().map(|&pair| vec![pair]);
        for links in each_alone.chain([pairs.clone()]) {
            for k in [1, 2] {
                let doubled = sym.lagrangian(&mut scratch, &mut bound, &links, k, &unit.promoted, limit, i64::MAX);
                assert!(
                    doubled >= kept,
                    "n={n} promoted {:?} cands {:?}: pairs {links:?}, k={k}: the bound {doubled} is below twice the \
                     best completion that keeps the inequalities ({kept})",
                    unit.promoted,
                    unit.cands
                );
                tally.bounds += 1;
                tally.below_plain += usize::from(doubled < plain);
                tally.exact += usize::from(doubled == kept);
            }
        }
    }
    tally.units += made;
}

/// The Lagrangian bound of the symmetric pairs is never below a completion that keeps their inequalities (the
/// completions the search with the tactic `symmetry` looks for), whatever pairs and multiplier: at several hundred
/// units of the boards 60 to 600 that have pairs, with the best such completion found exhaustively.  And it is not
/// vacuous: sometimes below the GF(2) bound it relaxes, and sometimes exact.
#[test]
fn the_lagrangian_bound_of_the_pairs_is_not_below_a_completion_that_keeps_the_inequalities() {
    let mut rng = Rng(0x1A62_A4C1_0000_0B0D);
    let mut tally = BoundTally::default();
    for n in (60..=600).step_by(11) {
        if n <= 511 {
            lagrangian_bounds_on::<8>(n, &mut rng, 8, &mut tally);
        } else {
            lagrangian_bounds_on::<16>(n, &mut rng, 8, &mut tally);
        }
    }
    assert!(tally.units >= 300 && tally.bounds > 3000 && tally.below_plain > 100 && tally.exact > 1000, "{tally:?}");
}

/// A search interrupted every `node_limit` nodes or so and resumed (two threads, a checkpoint every millisecond, a
/// tiny memory cap so that units spill to disk), from an incumbent `handicap` below the promotion sum `truth` of
/// game `n`, ends with the optimum for the configurations `which` of [`configs`]: the unit tests of resuming use one
/// game and a few tactics.  Returns the number of resumes.
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

#[test]
fn a_search_resumed_over_and_over_finds_the_optimum() {
    let optimal = Optimal::built_in();
    let dir = TempDir::new("churn");
    // On the matching path, game 600 takes 7,000 to 17,000 nodes from an incumbent that far below the optimum (two
    // threads: the count depends on timing), so a limit of 1,000 stops it at least six times in each of the three
    // configurations: the matching tactics, the searchers forgetting their nogoods every two steps, and their
    // Lagrangian bounds refitted every four.  The bounds below only check that the resuming really happened; they
    // leave room for the smallest trees a loaded machine produces.
    let truth = optimal_sum::<16>(optimal, 600).unwrap();
    let resumes = resumed_over_and_over::<16>(600, truth, 30, 1000, &dir, &[0, 12, 14]);
    assert!(resumes >= 10, "only {resumes} resumes");
    // The GF(2) bound searches 600 in too few nodes to be stopped (the engine checks the limit every 1,024 steps of
    // a searcher), and game 992 in some ten thousand: the default, without forced members, forgetting every step,
    // and refitting the Lagrangian bound.
    let truth = optimal_sum::<16>(optimal, 992).unwrap();
    let resumes = resumed_over_and_over::<16>(992, truth, 30, 1000, &dir, &[19, 24, 26, 29]);
    assert!(resumes >= 8, "the GF(2) bound: only {resumes} resumes");
}

/// The same at W = 20, 24 and 64: on the matching path, game 1133 (its truth from `results/strict.jsonl`) from an
/// incumbent 400 below it, stopped every 60 nodes or so (the engine checks every 1,024 steps of a searcher): its
/// tactics, forgetting the nogoods every two or three steps, and refitting the Lagrangian bound every four; with the
/// GF(2) bound, whose search of 1133 is too short to stop, game 992 (some ten thousand nodes) at those widths.
#[test]
fn a_wide_search_resumed_over_and_over_finds_the_optimum() {
    let n = 1133;
    let truth = stored_games("strict.jsonl")[&n].0 - Board::<20>::for_n(n).in_the_bag;
    let dir = TempDir::new("churn-wide");
    let resumes = resumed_over_and_over::<20>(n, truth, 400, 60, &dir, &[0, 12, 14])
        + resumed_over_and_over::<24>(n, truth, 400, 60, &dir, &[0])
        + resumed_over_and_over::<64>(n, truth, 400, 60, &dir, &[2]);
    assert!(resumes > 10, "only {resumes} resumes");
    let truth = optimal_sum::<16>(Optimal::built_in(), 992).unwrap();
    let resumes = resumed_over_and_over::<20>(992, truth, 30, 60, &dir, &[19, 26])
        + resumed_over_and_over::<24>(992, truth, 30, 60, &dir, &[19])
        + resumed_over_and_over::<64>(992, truth, 30, 60, &dir, &[29]);
    assert!(resumes > 6, "the GF(2) bound: only {resumes} resumes");
}

// ---------------------------------------------------------------------------------------------------------
// The GF(2) bound at nodes of the hardest games

/// At the nodes of the probe's fixtures (`testdata/perf/`: 300 nodes each of the proofs of 936 and 1215, from the root
/// to 30 promotions deep), with the oracles that do not use the bound: every completion made by promoting the node's
/// candidates in a random order, each if it can be, is independent in both of the node's matroids (the bound of the
/// node with only its members as candidates is their sum) and no heavier than the bound; and the search of the
/// matching path, started from the bound, finds no completion above it (where it ends within its time limit, it
/// proves the bound is at least the best completion).
#[test]
fn the_gf2_bound_holds_at_the_nodes_of_the_probe() {
    // All but 3 of the 600 searches end within 200 ms on an idle machine.
    let proved = gf2_at_probe_nodes::<16>("n0936.nodes") + gf2_at_probe_nodes::<20>("n1215.nodes");
    assert!(proved >= 550, "only {proved} nodes proved by a search");
}

/// [`the_gf2_bound_holds_at_the_nodes_of_the_probe`] at the nodes of `fixture`: returns the nodes at which the
/// search of the matching path ended.
fn gf2_at_probe_nodes<const W: usize>(fixture: &str) -> usize {
    let (n, _, units) = fixtures::read_nodes::<W>(&testdata(&format!("perf/{fixture}"))).unwrap();
    let game = Game::<W>::of_board(n).unwrap();
    let graph = Gf2Graph::new(&game.model, &game.root, &game.candidates);
    let mut bound = Gf2Bound::<W>::new(&graph);
    let mut scratch = Scratch::<W>::new();
    let mut rng = Rng(0x6F2B_C0DE_0000_0000 ^ u64::from(n));
    let mut proved = 0;
    for unit in units.iter().filter(|u| !u.cands.is_empty()) {
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
        let start = unit.promoted.sum() + value;
        let engine = EngineConfig { time_limit: Some(Duration::from_millis(200)), ..config(1) };
        let out = game.search(Config::matching(), vec![*unit], start, Some(engine), None);
        assert_eq!(out.best, start, "{at}: a completion above the bound {value}: {:?}", out.set);
        proved += usize::from(out.completed);
    }
    proved
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
    let (model, root) = Model::build::<4>(&tables, &columns).ok()?;
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

/// Perturbed games, their roots and some units below them, with the truth found exhaustively, under random
/// tactics, from an incumbent worth `k` less.
#[test]
fn perturbed_frame_sets_are_searched_exactly() {
    let mut rng = Rng(0x5EED_0001_C0DE_F00D);
    let configs = configs();
    let (mut games, mut runs) = (0, 0);
    while games < 500 {
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
    assert!(runs > 8000, "{runs} runs");
}

/// On perturbed games: promoting a set at once is promoting it one by one in any order (success and the state it
/// leaves), from the root and from random states; a conflict is a set that cannot be promoted; and the nogood made
/// of it is unpromotable from the root and made only of the conflict and the promotions before.  These are what
/// the search's reuse of nogoods across nodes, threads and resumes rests on.
#[test]
fn promotion_order_conflicts_and_nogoods_on_perturbed_frame_sets() {
    let mut rng = Rng(0x0DE5_0002_C0DE_F00D);
    let mut scratch = Scratch::<4>::new();
    let (mut games, mut orders, mut feasible, mut conflicts) = (0, 0, 0, 0);
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
            feasible += usize::from(at_once.is_some());
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
    assert!(orders > 1500 && feasible > 50 && conflicts > 100, "{orders} orders, {feasible} feasible, {conflicts}");
}

/// The conjecture that the promotion set with the largest sum also has the most promotions is false for the whole
/// frame game of n = 833, the first board whose optimal game is not a longest one (`prover/README.md`, section 6):
/// the optimal set promotes 48 numbers, as many as `max_total` allows less one, and a set the model accepts promotes
/// 49, four heavy promotions of the optimal set exchanged for five light ones; both games are over, the second one
/// move longer and 29 points lighter.  (The Java's `reviseFreedoms` assumed the conjecture of the two-frame game of
/// frames 0 and 1, which this does not decide.)
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
