//! Adversarial soundness tests of the search: could a pruning, a shortcut or a resume lose a better promotion set?
//!
//! The key idea is the handicapped incumbent.  The engine is started with a fake incumbent worth less than the best
//! completion (`truth − k`) and an empty payload.  If any pruning step loses every best completion, the fake one
//! survives, and its payload does not match its value.  This tests every pruning at once, including all the `<`
//! and `<=` boundaries: from `truth − 1`, every node whose bound is exactly `truth` must be explored.  `truth` is
//! `optimal.json`'s promotion sum at the root of a real game, and an exhaustive search for perturbed games and
//! units deep in the tree.

mod common;

use common::{config, random_subset, Config, Game, Rng, TempDir};
use std::time::Duration;
use taxman::bits::Bits;
use taxman::engine::EngineConfig;
use taxman::frames::build_frames;
use taxman::game::Optimal;
use taxman::hives::Apiary;
use taxman::model::{FrameSet, Model, Scratch};
use taxman::numtheory::Tables;
use taxman::search::{prepare, Branching, SearchTactics, TaxUnit};

/// Every tactic that acts inside the search, alone and in the combinations that matter, with the forgetting of
/// nogoods at the production period, every step, every third, and never; on one thread and two.
fn configs() -> Vec<Config> {
    let base = Config::standard();
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
        Config { branching: Branching::ConflictSmallest, ..base },
        Config { branching: Branching::ConflictLargest, refine: 2, ..base },
        Config { branching: Branching::Largest, ..base },
        Config { branching: Branching::Largest, refine: 0, ..base },
        Config { preclusions: true, ..base },
        Config { matching: false, branching: Branching::Largest, refine: 0, nogoods: false, ..base },
        Config { threads: 2, ..base },
        Config { threads: 2, period: Some(2), ..base },
        Config { threads: 2, refine: 0, ..base },
    ]
}

/// `optimal.json`'s best promotion sum of game `n`: its score less what the game with no promotions scores.
fn optimal_sum<const W: usize>(optimal: &Optimal, n: u32) -> Option<u64> {
    let in_the_bag: u64 = Apiary::<W>::new(&Tables::new(n), Bits::EMPTY).solution().iter().map(|&m| u64::from(m)).sum();
    optimal.score(n).map(|score| score - in_the_bag)
}

// ---------------------------------------------------------------------------------------------------------
// Real games

/// Every game 2..=300 that has candidates, from an incumbent worth 1 or 5 less than `optimal.json`'s promotion sum,
/// under each tactic: the search must come back with exactly that sum, and with a set that is promotable.  Unlike
/// the golden tests, which start from the greedy set, this starts from a weak incumbent.
#[test]
fn a_search_from_a_handicapped_incumbent_finds_the_optimum() {
    let optimal = Optimal::load_default().unwrap();
    let configs = configs();
    let mut runs = 0;
    for n in 2..=300 {
        let Some(game) = Game::<8>::of_board(n) else { continue };
        let truth = optimal_sum::<8>(&optimal, n).unwrap();
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

/// A search interrupted every thousand nodes or so and resumed (two threads, a checkpoint every millisecond, a tiny
/// memory cap so that units spill to disk), from a weak incumbent, ends with the optimum for every tactic: the unit
/// tests of resuming use one game and a few tactics.  Returns the number of resumes.
fn resumed_over_and_over<const W: usize>(n: u32, optimal: &Optimal, dir: &TempDir, which: &[usize]) -> usize {
    let game = Game::<W>::of_board(n).unwrap();
    let truth = optimal_sum::<W>(optimal, n).unwrap();
    let mut resumes = 0;
    for (i, cfg) in configs().into_iter().enumerate().filter(|(i, _)| which.contains(i)) {
        let cfg = Config { threads: 2, ..cfg };
        let checkpoint = dir.join(format!("n{n}-{i}.ckpt"));
        let engine = EngineConfig {
            threads: 2,
            queue_mem_cap: 2,
            spill_dir: Some(dir.join("spill")),
            checkpoint: Some((checkpoint.clone(), Duration::from_millis(1))),
            node_limit: Some(1500),
            ..config(2)
        };
        let mut out = game.search(cfg, vec![game.root_unit()], truth - 30, Some(engine.clone()), None);
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
    let optimal = Optimal::load_default().unwrap();
    let dir = TempDir::new("churn");
    // Game 600 takes 7,000 to 17,000 nodes from an incumbent that far below the optimum, so a limit of 1,500 stops it
    // four to nine times: with the default tactics, and with the searchers forgetting their nogoods every two steps.
    let resumes = resumed_over_and_over::<16>(600, &optimal, &dir, &[0, 18]);
    assert!(resumes > 10, "only {resumes} resumes");
}

// ---------------------------------------------------------------------------------------------------------
// Perturbed games

/// A game that the frame builder never makes: a real game's frames with numbers dropped, made moves instead of
/// factors (or the other way round) and moved a frame up or down, or random columns; kept only if every frame of
/// the root is winnable, with at most 36 candidates (mostly those that can be promoted from the root).
fn perturbed(rng: &mut Rng, n: u32, frames: u64, density: u64) -> Option<Perturbed> {
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
    (!candidates.is_empty() && candidates.len() <= 36).then(|| Perturbed {
        tables,
        columns,
        game: Game::new(model, root, candidates),
    })
}

/// A perturbed game with what is needed to set it up again.
struct Perturbed {
    tables: Tables,
    columns: Vec<Vec<u32>>,
    game: Game<4>,
}

impl Perturbed {
    /// The game as `prepare` sets it up with the default tactics: frame 0's freedoms revised, if they change.
    /// (Lowering them to any value would not do: the exact completions of the matching bound and the like rely on
    /// the revised freedoms being ones that the best set meets anyway, which the nested search of
    /// `revise_freedoms` finds.)
    fn revised(&self) -> Game<4> {
        let p = prepare::<4>(&self.tables, &self.columns, &self.game.candidates, SearchTactics::default(), 1);
        let p = p.unwrap();
        Game::new((*p.model).clone(), p.root, p.candidates)
    }
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
/// tactics, from an incumbent worth `k` less.  Half of the games are set up as `prepare` does, with frame 0's
/// freedoms revised (where that changes them), so that the frame set binds where the frame builder's never does.
/// The search treats the revised freedoms as what they are in a real game, a fact about the best set (the
/// conjecture of `revise_freedoms`: the set with the largest sum also has the largest count out of frame 1), and
/// is exact only where that holds.  A perturbed game does not have to be a real game, so those whose revision
/// changes the best sum are left out: the search of such a game (once in about 1,300) may return a set that the
/// revised freedoms forbid and is worth more.
#[test]
fn perturbed_frame_sets_are_searched_exactly() {
    let mut rng = Rng(0x5EED_0001_C0DE_F00D);
    let configs = configs();
    let (mut games, mut runs, mut revised, mut conjecture_fails) = (0, 0, 0, 0);
    while games < 500 {
        let n = 30 + rng.below(220) as u32;
        let (frames, density) = (2 + rng.below(6), 94 + rng.below(7));
        let Some(model) = perturbed(&mut rng, n, frames, density) else { continue };
        let (unrevised, unrevised_truth) =
            (model.game.root, model.game.exhaustive(&model.game.root, &model.game.candidates, 2_000_000));
        let game = if rng.chance(50) { model.revised() } else { model.game };
        let truth = game.exhaustive(&game.root, &game.candidates, 2_000_000);
        if truth != unrevised_truth {
            conjecture_fails += 1;
            continue;
        }
        if truth.is_none_or(|truth| truth == 0) {
            continue;
        }
        revised += usize::from(game.root != unrevised);
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
    assert!(runs > 8000 && revised > 25, "{runs} runs, {revised} games with revised freedoms");
    assert!(conjecture_fails < 5, "{conjecture_fails} perturbed games break the conjecture of revise_freedoms");
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
        let Some(perturbed) = perturbed(&mut rng, n, frames, density) else { continue };
        games += 1;
        let (model, root) = (&perturbed.game.model, perturbed.game.root);
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
