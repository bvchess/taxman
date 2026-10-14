//! Adversarial soundness tests of the search: could a pruning, a shortcut or a resume lose a better promotion set?
//!
//! The key idea is the handicapped incumbent.  The engine is started with a fake incumbent worth less than the best
//! completion (`truth − k`) and an empty payload.  If any pruning step loses every best completion, the fake one
//! survives, and its payload does not match its value.  This tests every pruning at once, including all the `<`
//! and `<=` boundaries: from `truth − 1`, every node whose bound is exactly `truth` must be explored.  `truth` is
//! `optimal.json`'s promotion sum at the root of a real game, and an exhaustive search for perturbed games and
//! units deep in the tree.

mod common;

use common::{config, random_subset, stored_games, Config, Game, Rng, TempDir};
use std::time::Duration;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::EngineConfig;
use taxman::frames::build_frames;
use taxman::game::Optimal;
use taxman::model::{FrameSet, Model, Scratch};
use taxman::numtheory::Tables;
use taxman::search::lagrange::Refits;
use taxman::search::Gf2Branching;
use taxman::search::TaxUnit;

/// Every tactic that acts inside the search, alone and in the combinations that matter: first the matching path
/// ([`SearchTactics::matching`], 0 to 18), then the default, the GF(2) bound (19 to 29).  The matching path with the
/// forgetting of nogoods at the production period, every step, every third, and never; on one thread and two; and
/// the GF(2) bound with each way of choosing the branch, with and without the matching bound as a first check,
/// forced members, nogoods and the Lagrangian bound's early refits.  The Lagrangian
/// bound (tactic `lagrange`) refits its multipliers only after 2,000 steps, which most of these searches never
/// take, so the last configurations refit them after the first few steps and every few steps after that (with and
/// without forgetting at the same time, refinement, and a second thread), and one turns it off.
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

/// Units of game `n` at width `W`, after up to 14 random promotions, with their candidates thinned to 6 to 23 (the
/// largest, or at random) so that the truth can be found exhaustively, searched from an incumbent worth 1, 2, 5
/// or 30 less under random `configs`: the search must come back with the truth.  Returns the number of searches.
fn handicapped_units<const W: usize>(n: u32, rng: &mut Rng, units: usize, configs: &[Config]) -> usize {
    let game = Game::<W>::of_board(n).expect("a game");
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
        let Some(rest) = game.exhaustive(&state, &thin, 3_000_000) else { continue };
        made += 1;
        let truth = promoted.sum() + rest;
        for _ in 0..if rest == 0 { 0 } else { 4 } {
            let cfg = rng.pick(configs);
            let k = rng.pick(&[1u64, 1, 2, 5, 30]).min(rest);
            let out = game.search(cfg, vec![TaxUnit { promoted, cands: thin }], truth - k, None, None);
            runs += 1;
            assert!(
                out.completed && out.best == truth && out.real,
                "n={n} W={W} {cfg:?} promoted {promoted:?} cands {thin:?} from {}: truth {truth}, got {out:?}",
                truth - k
            );
        }
    }
    runs
}

/// Units of real games at W = 20 to 64, the widths of the frontier and beyond, from a handicapped incumbent under
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
    // a searcher), and game 1000 in several thousand: the default, without forced members, forgetting every step,
    // and refitting the Lagrangian bound.
    let truth = optimal_sum::<16>(optimal, 1000).unwrap();
    let resumes = resumed_over_and_over::<16>(1000, truth, 30, 1000, &dir, &[19, 24, 26, 29]);
    assert!(resumes >= 8, "the GF(2) bound: only {resumes} resumes");
}

/// The same at W = 20, 24 and 64: on the matching path, game 1133 (its truth from `results/strict.jsonl`) from an
/// incumbent 400 below it, stopped every 60 nodes or so (the engine checks every 1,024 steps of a searcher): its
/// tactics, forgetting the nogoods every two or three steps, and refitting the Lagrangian bound every four; with the
/// GF(2) bound, whose search of 1133 is too short to stop, game 1000 at those widths.
#[test]
fn a_wide_search_resumed_over_and_over_finds_the_optimum() {
    let n = 1133;
    let truth = stored_games("strict.jsonl")[&n].0 - Board::<20>::for_n(n).in_the_bag;
    let dir = TempDir::new("churn-wide");
    let resumes = resumed_over_and_over::<20>(n, truth, 400, 60, &dir, &[0, 12, 14])
        + resumed_over_and_over::<24>(n, truth, 400, 60, &dir, &[0])
        + resumed_over_and_over::<64>(n, truth, 400, 60, &dir, &[2]);
    assert!(resumes > 10, "only {resumes} resumes");
    let truth = optimal_sum::<16>(Optimal::built_in(), 1000).unwrap();
    let resumes = resumed_over_and_over::<20>(1000, truth, 30, 60, &dir, &[19, 26])
        + resumed_over_and_over::<24>(1000, truth, 30, 60, &dir, &[19])
        + resumed_over_and_over::<64>(1000, truth, 30, 60, &dir, &[29]);
    assert!(resumes > 6, "the GF(2) bound: only {resumes} resumes");
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
        if truth.is_none_or(|truth| truth == 0) {
            continue;
        }
        games += 1;
        for (promoted, cands, state) in path(&mut rng, &game) {
            let Some(rest) = game.exhaustive(&state, &cands, 2_000_000) else { continue };
            let truth = promoted.sum() + rest;
            for _ in 0..6 {
                let cfg = rng.pick(&configs);
                let k = rng.pick(&[1u64, 1, 2, 3, 9, 30, 1000]).min(rest);
                if k == 0 {
                    continue;
                }
                let out = game.search(cfg, vec![TaxUnit { promoted, cands }], truth - k, None, None);
                runs += 1;
                assert!(
                    out.best == truth && out.real,
                    "n={n} frames {:?} {cfg:?} promoted {promoted:?} cands {cands:?} from {}: truth {truth}, \
                     got {out:?}",
                    game.root,
                    truth - k
                );
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
/// frame game of n = 833, the first board whose optimal game is not a longest one (`evidence/README.md`, section 8):
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
