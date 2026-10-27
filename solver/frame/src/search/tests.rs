use super::header::Header;
use super::refine::{strongest, Learning, Refiner, Verdict};
use super::*;
use crate::board::Board;
use crate::bounds::max_additional;
use crate::engine::Unit;
use crate::frames::build_frames;
use crate::gf2::{ForceWindow, Gf2Bound, Gf2Graph};
use crate::json::Json;
use crate::matchbound::MatchBound;
use crate::model::test_support::*;
use crate::model::{ConflictCache, FrameGraph, Keep, Nogoods, Scratch};
use crate::numtheory::Tables;
use crate::test_support::{TempDir, QUICK};
use std::thread;

fn config(threads: usize) -> EngineConfig {
    EngineConfig { threads, progress: None, ..EngineConfig::default() }
}

/// The golden games the Java had to search (not accelerated), up to `max_n`.
fn searched_games(max_n: u32) -> Vec<Golden> {
    golden_ns()
        .into_iter()
        .filter(|&n| n <= max_n)
        .map(load)
        .filter(|g| g.json.get("solve").and_then(|s| s.get("accelerated")).and_then(Json::as_bool) == Some(false))
        .collect()
}

/// The sum of the Java's optimal promotions.
fn golden_sum(g: &Golden) -> u64 {
    u32s(g.json.get("solve").expect("solve"), "promotions").iter().map(|&x| u64::from(x)).sum()
}

fn prepare_golden<const W: usize>(g: &Golden, tactics: SearchTactics) -> Prepared<W> {
    prepare::<W>(&Tables::new(g.n), &g.columns, &Bits::from_slice(&g.candidates), tactics).unwrap()
}

/// Searches and checks that the result is complete, made of candidates, and promotable.
fn search_checked<const W: usize>(p: &Prepared<W>, threads: usize, what: &str) -> SearchResult<W> {
    let r = search(p, &config(threads), None, None, false).unwrap();
    assert!(r.completed, "{what}: incomplete");
    assert!(r.promotions.is_subset(&p.candidates), "{what}: {:?} are not all candidates", r.promotions);
    let promotable = p.model.promote_all_using(&mut Scratch::new(), &p.root, &r.promotions);
    assert!(promotable.is_some(), "{what}: {:?} cannot be promoted", r.promotions);
    r
}

fn check_against_java<const W: usize>(g: &Golden) {
    let p = prepare_golden::<W>(g, SearchTactics::default());
    // The Java's frame set and `max_total` before its revision of frame 0, which this build does not have.
    let frames = g.json.get("frameSet").and_then(Json::as_arr).expect("frameSet");
    let frame0 = frames.first().and_then(|frame| frame.get("freedoms")).and_then(Json::as_i64);
    assert_eq!(p.root.frames().first().map(|f| i64::from(f.freedoms)), frame0, "n={} frame 0 freedoms", g.n);
    let max_total = g.json.get("maxPromotionsUnrevised").and_then(Json::as_i64).expect("maxPromotionsUnrevised");
    assert_eq!(i64::from(p.max_total), max_total, "n={} max_total", g.n);
    for threads in [1, 4] {
        let r = search_checked(&p, threads, &format!("n={} with {threads} threads", g.n));
        assert_eq!(r.sum, golden_sum(g), "n={} with {threads} threads: {:?}", g.n, r.promotions);
    }
}

#[test]
fn golden_games_up_to_400_match_the_java() {
    let games = searched_games(400);
    assert!(games.len() > 80, "only {} games", games.len());
    for g in &games {
        check_against_java::<8>(g);
    }
}

/// Checks the refined matching bound along random search paths of `g`: it never prunes a node whose best
/// completion (found exhaustively) reaches `need`, the completions it calls exact or finds can be promoted,
/// and the conflicts it keeps, carried down the path, stay conflicts.  Returns the numbers of verdicts, of
/// prunings the plain matching bound would not make, and of exact completions.
fn refinement_soundness<const W: usize>(g: &Golden, rng: &mut Rng) -> [u64; 3] {
    let (model, root) = FrameGraph::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
    let all = Bits::<W>::from_slice(&g.candidates);
    let max_total = root.max_promotions_among(&all).max(0) as u32;
    let graph = MatchGraph::new(&model, &root);
    let mut m = MatchBound::new(&graph);
    let mut scratch = Scratch::new();
    let mut table = Nogoods::new(g.n, 1 << 12);
    let mut checks = [0; 3];
    for _path in 0..6 {
        let (mut state, mut promoted, mut cands) = (root, Bits::<W>::EMPTY, all);
        let mut known = Vec::new();
        let mut cache = ConflictCache::default();
        loop {
            let max_add = max_additional(&model, max_total, &state, &promoted, &cands);
            let truth = best_completion(&model, &state, &cands, max_add);
            let what = format!("n={} promoted {promoted:?} cands {cands:?} best {truth}", g.n);
            for depth in [0, 1, 2, 4] {
                for need in [truth.saturating_sub(7), truth, truth + 1, truth + 9] {
                    m.reset(&promoted, &cands);
                    let plain = m.value();
                    // Half the time with the nogoods learned so far, on this path and others.
                    let learning = rng.chance(50).then_some(Learning { table: &mut table, root: &root, promoted });
                    let mut refiner = Refiner {
                        m: &mut m,
                        model: &model,
                        scratch: &mut scratch,
                        state: &state,
                        need,
                        known: &mut known,
                        cache: &mut cache,
                        evaluations: 0,
                        computed: 0,
                        exact: Bits::EMPTY,
                        learning,
                        forcing: rng.chance(50),
                        forced: Bits::EMPTY,
                    };
                    let verdict = refiner.top(depth, rng.chance(50));
                    let found = refiner.exact;
                    if need <= truth {
                        assert_ne!(verdict, Verdict::Below, "{what}: depth {depth} prunes at need {need}");
                    }
                    let exact = match verdict {
                        Verdict::Exact(q) => Some(q),
                        _ => None,
                    };
                    for q in [exact, Some(found).filter(|f| !f.is_empty())].into_iter().flatten() {
                        assert!(q.is_subset(&cands) && q.sum() >= need, "{what}: exact {q:?} at need {need}");
                        assert!(model.can_promote_all_using(&mut scratch, &state, &q), "{what}: {q:?} fails");
                    }
                    checks[0] += 1;
                    checks[1] += u64::from(verdict == Verdict::Below && plain >= need);
                    checks[2] += u64::from(exact.is_some() || !found.is_empty());
                }
            }
            for k in &known {
                assert!(!model.can_promote_all_using(&mut scratch, &state, k), "{what}: {k:?} is no conflict");
            }
            // Descend into a random child, as the search would (in any order).
            let members = cands.to_vec();
            if members.is_empty() {
                break;
            }
            let Some((n, next)) = (0..4).find_map(|_| {
                let n = rng.pick(&members);
                model.promote(&state, n).map(|s| (n, s))
            }) else {
                break;
            };
            cands = next.narrow_candidates(&cands.without(n));
            (state, promoted) = (next, promoted.with(n));
        }
    }
    checks
}

#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is refined_matching_bound_is_sound_on_a_sample")]
fn refined_matching_bound_is_sound() {
    refined_matching_bound_is_sound_on_every(1);
}

/// [`refined_matching_bound_is_sound`] on every sixth of its golden games.
#[test]
fn refined_matching_bound_is_sound_on_a_sample() {
    refined_matching_bound_is_sound_on_every(6);
}

/// The refined matching bound is sound on every `every`th of the golden games.
fn refined_matching_bound_is_sound_on_every(every: usize) {
    let mut rng = Rng(0x5EED_C0F1_1C75_0001);
    let mut checks = [0; 3];
    for n in golden_ns().into_iter().filter(|&n| (40..=220).contains(&n)).step_by(every) {
        let more = refinement_soundness::<4>(&load(n), &mut rng);
        for (a, b) in checks.iter_mut().zip(more) {
            *a += b;
        }
    }
    let [verdicts, refined, exact] = checks;
    eprintln!("refined matching bound: {verdicts} verdicts, {refined} refined prunings, {exact} exact completions");
    assert!(verdicts > 5000 / every as u64 && refined > 500 / every as u64 && exact > 500 / every as u64);
}

/// Checks the forced members of strong branching (tactic `fix_forced`) along random search paths of `g`: at a
/// node whose heaviest matching has a conflict, every forced member is in every completion that reaches
/// `need` (found exhaustively), so the child that promotes all of them keeps the best completion whenever it
/// reaches `need`.  Returns the numbers of nodes checked and of forced members.  Two paths per game (six took
/// 3.5 s and found 7,400 nodes and 18,000 forced members, 15 to 60 times what the test asks for).
fn forced_soundness<const W: usize>(g: &Golden, rng: &mut Rng) -> [u64; 2] {
    let (model, root) = FrameGraph::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
    let all = Bits::<W>::from_slice(&g.candidates);
    let max_total = root.max_promotions_among(&all).max(0) as u32;
    let graph = MatchGraph::new(&model, &root);
    let mut m = MatchBound::new(&graph);
    let mut scratch = Scratch::new();
    let mut checks = [0; 2];
    for _path in 0..2 {
        let (mut state, mut promoted, mut cands) = (root, Bits::<W>::EMPTY, all);
        loop {
            let max_add = max_additional(&model, max_total, &state, &promoted, &cands);
            let truth = best_completion(&model, &state, &cands, max_add);
            for need in [truth.saturating_sub(5), truth, truth + 1] {
                m.reset(&promoted, &cands);
                if m.value() < need {
                    continue;
                }
                let chosen = m.chosen();
                let Some(k) = model.conflict_using(&mut scratch, &state, &chosen, true) else { continue };
                let k = k & cands;
                if k.len() < 2 {
                    continue;
                }
                let what = format!("n={} promoted {promoted:?} cands {cands:?} need {need}", g.n);
                // The members of the heaviest matching forced by the potentials alone, or else those of the
                // conflict that strong branching finds.
                let quick: Bits<W> = chosen.iter().filter(|&x| m.bound_excluding(x) < need).collect();
                let forced = if quick.is_empty() || rng.chance(30) {
                    let strong = strongest(&mut m, &k, need, rng.chance(50));
                    let (n, forced) = (strong.member, strong.forced);
                    assert!(k.contains(n) && forced.is_subset(&k), "{what}: {n} {forced:?} from {k:?}");
                    assert!(forced.is_empty() || forced.contains(n), "{what}: {n} is not among {forced:?}");
                    assert!((quick & k).is_subset(&forced), "{what}: {quick:?} not all in {forced:?}");
                    forced
                } else {
                    quick
                };
                for x in forced.iter() {
                    let without = best_completion(&model, &state, &cands.without(x), max_add);
                    assert!(without < need, "{what}: {x} is forced, but leaving it out reaches {without}");
                }
                if truth >= need && !forced.is_empty() {
                    let next = model.promote_all_using(&mut scratch, &state, &forced);
                    let next = next.unwrap_or_else(|| panic!("{what}: the forced {forced:?} cannot be promoted"));
                    let rest = best_completion(&model, &next, &(cands - forced), max_add - forced.len());
                    assert_eq!(rest + forced.sum(), truth, "{what}: promoting {forced:?} loses the best");
                }
                checks[0] += 1;
                checks[1] += u64::from(forced.len());
            }
            // Descend into a random child, as the search would (in any order).
            let members = cands.to_vec();
            if members.is_empty() {
                break;
            }
            let Some((n, next)) = (0..4).find_map(|_| {
                let n = rng.pick(&members);
                model.promote(&state, n).map(|s| (n, s))
            }) else {
                break;
            };
            cands = next.narrow_candidates(&cands.without(n));
            (state, promoted) = (next, promoted.with(n));
        }
    }
    checks
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is forced_members_are_in_every_good_completion_of_a_sample"
)]
fn forced_members_are_in_every_good_completion() {
    forced_members_are_in_every_good_completion_on_every(1);
}

/// [`forced_members_are_in_every_good_completion`] on every sixth of its golden games.
#[test]
fn forced_members_are_in_every_good_completion_of_a_sample() {
    forced_members_are_in_every_good_completion_on_every(6);
}

/// The forced members are in every good completion on every `every`th of the golden games.
fn forced_members_are_in_every_good_completion_on_every(every: usize) {
    let mut rng = Rng(0xF0BC_ED00_1234_5678);
    let mut checks = [0; 2];
    for n in golden_ns().into_iter().filter(|&n| (40..=220).contains(&n)).step_by(every) {
        let more = forced_soundness::<4>(&load(n), &mut rng);
        for (a, b) in checks.iter_mut().zip(more) {
            *a += b;
        }
    }
    let [nodes, forced] = checks;
    eprintln!("forced members: {nodes} nodes with a conflict to branch on, {forced} forced members");
    assert!(nodes > 500 / every as u64 && forced > 300 / every as u64);
}

#[test]
fn golden_games_above_400_match_the_java() {
    let games: Vec<Golden> = golden_ns().into_iter().filter(|&n| n > 400).map(load).collect();
    assert_eq!(games.len(), 8);
    for g in &games {
        check_against_java::<16>(g);
    }
}

/// Every candidate of the frame builder, for the games `ns` (all with candidates, or none), can be promoted
/// from the root by itself.
fn candidates_can_be_promoted_from_the_root<const W: usize>(ns: impl Iterator<Item = u32>) {
    let mut scratch = Scratch::new();
    for n in ns {
        let tables = Tables::new(n);
        let frames = build_frames::<W>(&tables);
        let (model, root) = FrameGraph::build::<W>(&tables, &frames.columns()).unwrap();
        for x in frames.candidates.iter() {
            assert!(model.can_promote_using(&mut scratch, &root, x), "n={n}: candidate {x} cannot be promoted");
        }
    }
}

/// So a test of each candidate at the root would find nothing to drop (see solver/docs/design/search.md, "Forced
/// members and nogoods", the item "Single-member conflicts belong to the node"), and the single-member conflicts of the
/// search are relative to its nodes.  The cost is all in building the frames (10 to 20 ms a board above 500), so every
/// board to 511 is checked (0.2 s) and every sixth above (1.2 s); all of 2..=1230 takes 7 s.
#[test]
fn the_frame_builders_candidates_can_all_be_promoted_from_the_root() {
    candidates_can_be_promoted_from_the_root::<8>(2..=511);
    candidates_can_be_promoted_from_the_root::<16>((512..=1023).step_by(6));
    candidates_can_be_promoted_from_the_root::<32>((1024..=1230).step_by(6));
}

fn sums_by_tactics<const W: usize>(g: &Golden) -> Vec<(SearchTactics, u64)> {
    let default = SearchTactics::default();
    let variants = [
        default,
        SearchTactics { match_refine: 3, ..default },
        SearchTactics { fix_forced: false, ..default },
        SearchTactics { fix_forced: false, match_refine: 0, ..default },
        SearchTactics { fix_forced: false, match_refine: 2, ..default },
        SearchTactics { nogoods: false, ..default },
        SearchTactics { nogoods: false, match_refine: 0, ..default },
        SearchTactics { forget_nogoods: false, ..default },
        SearchTactics { lagrange: false, ..default },
        SearchTactics { match_refine: 0, ..default },
        SearchTactics { symmetry: false, ..default },
        SearchTactics::matching(),
        SearchTactics { symmetry: true, ..SearchTactics::matching() },
    ];
    variants.iter().map(|&t| (t, search_checked(&prepare_golden::<W>(g, t), 1, "tactics").sum)).collect()
}

#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is tactics_do_not_change_answers_on_a_sample")]
fn tactics_do_not_change_answers() {
    tactics_do_not_change_answers_on_every(1);
}

/// [`tactics_do_not_change_answers`] on every fifth of its golden games.
#[test]
fn tactics_do_not_change_answers_on_a_sample() {
    tactics_do_not_change_answers_on_every(5);
}

/// No tactic changes the answers on every `every`th of the golden games.
fn tactics_do_not_change_answers_on_every(every: usize) {
    let mut differences = Vec::new();
    for g in searched_games(400).iter().step_by(every) {
        let sums = sums_by_tactics::<8>(g);
        for &(tactics, sum) in &sums[1..] {
            if sum != sums[0].1 {
                differences.push(format!("n={}: {tactics:?} gives {sum}, default {}", g.n, sums[0].1));
            }
        }
    }
    assert!(differences.is_empty(), "A TACTIC CHANGES ANSWERS:\n{}", differences.join("\n"));
}

/// The searches of these games are shorter than [`NOGOOD_PERIOD`] steps, so they never forget with it: forget
/// after every step, and after a few.
#[test]
fn forgetting_nogoods_often_does_not_change_answers() {
    for g in &searched_games(400) {
        let p = prepare_golden::<8>(g, SearchTactics::default());
        for (period, threads) in [(1, 1), (3, 2), (40, 1)] {
            let every = Forgetting::every(period);
            let cfg = config(threads);
            let r = search_forgetting(&p, &cfg, &[], None, false, None, every, None, None, None, None, None).unwrap();
            let what = format!("n={}, forgetting every {period} steps on {threads} threads", g.n);
            assert!(r.completed && r.promotions.is_subset(&p.candidates), "{what}");
            assert_eq!(r.sum, golden_sum(g), "{what}");
        }
    }
}

/// Every policy of keeping and forgetting nogoods is sound: the table is a cache of nogoods that hold at every node,
/// so whatever the searchers keep, however often they forget and however small the table's limit, P* is the same.
/// The periods are short, to make the policies act in searches of a few thousand steps.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is every_policy_of_keeping_nogoods_gives_the_same_optimum_of_a_sample"
)]
fn every_policy_of_keeping_nogoods_gives_the_same_optimum() {
    every_policy_of_keeping_nogoods_gives_the_same_optimum_on_every(1);
}

/// [`every_policy_of_keeping_nogoods_gives_the_same_optimum`] on every fifth of its golden games.
#[test]
fn every_policy_of_keeping_nogoods_gives_the_same_optimum_of_a_sample() {
    every_policy_of_keeping_nogoods_gives_the_same_optimum_on_every(5);
}

/// Every policy of keeping nogoods gives the same optimum on every `every`th of the golden games.
fn every_policy_of_keeping_nogoods_gives_the_same_optimum_on_every(every: usize) {
    let policies = [
        Forgetting { period: None, keep: Keep::Nothing, limit: None },
        Forgetting { period: Some(7), keep: Keep::Used, limit: None },
        Forgetting { period: Some(7), keep: Keep::Short(2), limit: None },
        Forgetting { period: Some(7), keep: Keep::Short(3), limit: None },
        Forgetting { period: Some(7), keep: Keep::Short(4), limit: None },
        Forgetting { period: Some(7), keep: Keep::Top(1), limit: None },
        Forgetting { period: Some(7), keep: Keep::Top(64), limit: None },
        Forgetting { period: Some(300), keep: Keep::Used, limit: None },
        Forgetting { period: None, keep: Keep::Nothing, limit: Some(1) },
        Forgetting { period: None, keep: Keep::Nothing, limit: Some(4) },
        Forgetting { period: Some(50), keep: Keep::Top(3), limit: Some(5) },
    ];
    for g in searched_games(400).iter().step_by(every) {
        let p = prepare_golden::<8>(g, SearchTactics::default());
        for forgetting in policies {
            for threads in [1, 2] {
                let cfg = config(threads);
                let r = search_forgetting(&p, &cfg, &[], None, false, None, forgetting, None, None, None, None, None)
                    .unwrap();
                let what = format!("n={}, {forgetting:?} on {threads} threads", g.n);
                assert!(r.completed && r.promotions.is_subset(&p.candidates), "{what}");
                assert_eq!(r.sum, golden_sum(g), "{what}");
                if let Some(limit) = forgetting.limit {
                    assert!(r.nogoods.largest <= limit as u64, "{what}: a table of {} nogoods", r.nogoods.largest);
                }
            }
        }
    }
}

/// Searches 760 in pieces of about a sixth of its nodes each, resuming from the checkpoint, and checks the
/// answer.  The engine checks the node limit every 1024 nodes, so a search of fewer than 6·1024 nodes stops
/// less often: with one thread it must still take `min_pieces` pieces.
fn resume_in_pieces(threads: usize, tactics: SearchTactics, min_pieces: usize) {
    let g = load(760);
    let tables = Tables::new(g.n);
    let cands = Bits::<16>::from_slice(&g.candidates);
    let whole = prepare::<16>(&tables, &g.columns, &cands, tactics).unwrap();
    let nodes = search_checked(&whole, 1, "uninterrupted").stats.nodes;

    let dir = TempDir::new(&format!("resume{threads}-{}", tactics.bits()));
    let ckpt = dir.0.join("ckpt.bin");
    let cfg = EngineConfig {
        checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
        node_limit: Some(nodes / 6 + 1),
        queue_mem_cap: 4,
        spill_dir: Some(dir.0.join("spill")),
        ..config(threads)
    };
    let mut result = search(&whole, &cfg, None, None, false).unwrap();
    let mut pieces = 1;
    while !result.completed {
        assert!(ckpt.exists(), "a stopped run leaves a checkpoint");
        let resumed = prepare_for_resume::<16>(&tables, &g.columns, &cands, &ckpt).unwrap();
        assert_eq!(resumed.root, whole.root);
        result = search(&resumed, &cfg, None, None, true).unwrap();
        pieces += 1;
        assert!(pieces < 100, "no progress");
    }
    // One thread explores in depth-first order however often it is interrupted, but the conflicts a searcher
    // learns along its path are not part of a work unit, so a resumed search can prune a little differently
    // (it does not at the time of writing); several threads explore a varying number of nodes anyway.
    assert!(pieces >= if threads == 1 { min_pieces } else { 2 }, "only {pieces} pieces");
    assert!(!ckpt.exists(), "a completed run deletes its checkpoint");
    assert_eq!(result.sum, golden_sum(&g));
}

#[test]
fn checkpoint_and_resume_one_thread() {
    resume_in_pieces(1, SearchTactics::matching(), 3);
    resume_in_pieces(1, SearchTactics { fix_forced: false, ..SearchTactics::matching() }, 4);
    // With the rule of the symmetric pairs, whose units hold the candidates it has dropped from their nodes.
    resume_in_pieces(1, SearchTactics { symmetry: true, ..SearchTactics::matching() }, 3);
}

#[test]
fn checkpoint_and_resume_four_threads() {
    // The matching search of 760 takes too few nodes (about 2,300) for four threads to reach the node limit
    // before the end; without refinement, forced members and nogoods it takes about 4,700, which some thread
    // always meets the limit in.
    let weak = SearchTactics { match_refine: 0, fix_forced: false, nogoods: false, ..SearchTactics::matching() };
    resume_in_pieces(4, weak, 2);
    resume_in_pieces(4, SearchTactics { symmetry: true, ..weak }, 2);
}

/// The default search (tactic `gf2`, with the rule of the symmetric pairs, tactic `symmetry`), and without the rule
/// (whose tree is larger, so its limit is), stopped every few hundred nodes and resumed from its checkpoint, on one
/// thread and on two, until it finishes with game 992's optimum: its searches of 760 are too short to interrupt (the
/// engine checks the node limit every 1024 nodes of a searcher), and so are those of 1000 on two threads since the
/// window of the forced members became `all` (2,026 nodes).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is checkpoint_and_resume_with_the_gf2_bound_of_a_small_game"
)]
fn checkpoint_and_resume_with_the_gf2_bound() {
    let default = SearchTactics::default();
    let cases = [(default, 1000), (SearchTactics { symmetry: false, ..default }, 1500)];
    resume_the_gf2_search(992, &cases, &[1, 2]);
}

/// [`checkpoint_and_resume_with_the_gf2_bound`] for game 1000 (2,026 nodes by the default) on one thread, stopped
/// every 1,000 nodes.
#[test]
fn checkpoint_and_resume_with_the_gf2_bound_of_a_small_game() {
    resume_the_gf2_search(1000, &[(SearchTactics::default(), 1000)], &[1]);
}

/// The search of game `n` under each of `cases` (tactics and the node limit that stops it), on each of `threads`,
/// stopped and resumed from its checkpoint until it finishes with P*.
fn resume_the_gf2_search(n: u32, cases: &[(SearchTactics, u64)], threads: &[usize]) {
    let board = crate::board::Board::<16>::for_n(n);
    let opt = crate::game::Optimal::built_in().score(n).unwrap() - board.in_the_bag;
    for &(tactics, node_limit) in cases {
        let p = board.prepare(tactics).unwrap();
        for &threads in threads {
            let what = format!("{threads} threads, symmetry {}", tactics.symmetry);
            let dir = TempDir::new(&format!("resume-gf2-{threads}-{}", tactics.bits()));
            let ckpt = dir.0.join("ckpt.bin");
            let cfg = EngineConfig {
                checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
                node_limit: Some(node_limit),
                queue_mem_cap: 4,
                spill_dir: Some(dir.0.join("spill")),
                ..config(threads)
            };
            let mut result = search(&p, &cfg, None, None, false).unwrap();
            let mut pieces = 1;
            while !result.completed {
                let resumed = board.prepare_for_resume(&ckpt).unwrap();
                assert_eq!(resumed.tactics, tactics, "{what}: the checkpoint records the tactics");
                result = search(&resumed, &cfg, None, None, true).unwrap();
                pieces += 1;
                assert!(pieces < 200, "no progress");
            }
            assert!(pieces >= 2, "{what}: only {pieces} pieces");
            assert_eq!(result.sum, opt, "{what}");
        }
    }
}

#[test]
fn a_checkpoint_of_another_game_is_refused() {
    let (a, b) = (load(760), load(720));
    let dir = TempDir::new("refuse");
    let ckpt = dir.0.join("ckpt.bin");
    let pa = prepare_golden::<16>(&a, SearchTactics::matching());
    let cfg =
        EngineConfig { checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))), node_limit: Some(50), ..config(1) };
    assert!(!search(&pa, &cfg, None, None, false).unwrap().completed);
    let cands = Bits::<16>::from_slice(&b.candidates);
    assert!(prepare_for_resume::<16>(&Tables::new(b.n), &b.columns, &cands, &ckpt).is_err());
    let cands = Bits::<32>::from_slice(&a.candidates);
    assert!(prepare_for_resume::<32>(&Tables::new(a.n), &a.columns, &cands, &ckpt).is_err());
}

#[test]
fn seeds_and_caps() {
    let g = load(300);
    let p = prepare_golden::<8>(&g, SearchTactics::default());
    let optimum = search_checked(&p, 1, "plain");
    // Seeded with an optimal set (plus a non-candidate), the search only has to prove it.
    let seed = optimum.promotions.with(299);
    let seeded = search(&p, &config(1), Some(&seed), None, false).unwrap();
    assert_eq!((seeded.sum, seeded.promotions), (optimum.sum, optimum.promotions));
    assert!(seeded.stats.nodes <= optimum.stats.nodes);
    // A ceiling the seed already reaches ends the search before it starts; one the search reaches ends it
    // there.
    let capped = search(&p, &config(1), Some(&seed), Some(optimum.sum), false).unwrap();
    assert!(capped.completed && capped.reached_cap && capped.stats.nodes == 0);
    let capped = search(&p, &config(1), None, Some(optimum.sum), false).unwrap();
    assert!(capped.completed && capped.reached_cap && capped.sum == optimum.sum);
    assert!(capped.stats.nodes <= optimum.stats.nodes);
    // A worse seed changes nothing.
    let worse = search(&p, &config(1), Some(&Bits::EMPTY), None, false).unwrap();
    assert_eq!(worse.sum, optimum.sum);
}

/// The root's GF(2) bound with no incumbent: the bound the search computes at its first node, as a promotion sum.
fn root_bound<const W: usize>(p: &Prepared<W>) -> u64 {
    let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
    let mut bound = Gf2Bound::<W>::new(&graph);
    bound.reset(&Bits::EMPTY, &p.candidates);
    bound.evaluate(p.max_total, None).value
}

/// The search of `p` (whose P* is `opt`) under floors around P*, on `threads` threads: a floor below it finds it, a
/// floor at it or above completes without finding anything, and a floor at the root's bound or above completes at
/// once.
fn floors_around_the_optimum<const W: usize>(p: &Prepared<W>, opt: u64, threads: usize, what: &str) {
    let cfg = config(threads);
    for floor in [opt - 1, opt - opt.min(40), opt / 2, 0] {
        let r = search_above(p, &cfg, floor, None, false).unwrap();
        assert!(r.completed && !r.floor_held, "{what}, floor {floor}: it finds a set above the floor");
        assert_eq!((r.sum, r.promotions.sum()), (opt, opt), "{what}, floor {floor}");
        assert!(r.promotions.is_subset(&p.candidates), "{what}, floor {floor}");
        assert!(p.model.promote_all_using(&mut Scratch::new(), &p.root, &r.promotions).is_some(), "{what}");
        assert!(r.start.is_empty() && r.found.is_some(), "{what}, floor {floor}: the floor is the start");
    }
    let root = root_bound(p);
    assert!(opt <= root, "{what}: the root's bound {root} is below P* {opt}");
    for floor in [opt, opt + 1, opt + (root - opt) / 2, root, root + 1] {
        let r = search_above(p, &cfg, floor, None, false).unwrap();
        assert!(r.completed && r.floor_held, "{what}, floor {floor}: nothing is above it");
        assert!(r.promotions.is_empty() && r.found.is_none() && r.start.is_empty(), "{what}, floor {floor}");
        assert_eq!(r.sum, floor, "{what}, floor {floor}: the incumbent is the floor");
        // The first check of the search is the GF(2) bound, which the floor reaches at the root (the matching path
        // checks its own bound first, which can be higher).
        if floor >= root && p.tactics.gf2 {
            assert!(r.stats.nodes <= 1, "{what}, floor {floor}: {} steps at the root's bound", r.stats.nodes);
        }
    }
}

/// With no set to find above P* the floor is the incumbent, so its tree is the proof of a search seeded with an
/// optimal set, step for step on one thread.
fn the_tree_under_the_optimum_is_the_proof(n: u32) {
    let board = crate::board::Board::<16>::for_n(n);
    let p = board.prepare(SearchTactics::default()).unwrap();
    let opt = crate::game::Optimal::built_in().score(n).unwrap() - board.in_the_bag;
    let found = search(&p, &config(1), None, None, false).unwrap();
    assert_eq!(found.sum, opt);
    let seeded = search(&p, &config(1), Some(&found.promotions), None, false).unwrap();
    let floored = search_above(&p, &config(1), opt, None, false).unwrap();
    assert!(seeded.completed && floored.completed && floored.floor_held, "n={n}");
    assert_eq!(seeded.start, found.promotions, "n={n}: the seed is the start");
    assert_eq!(floored.stats.nodes, seeded.stats.nodes, "n={n}: the floor at P* is the proof");
}

/// A floor below P* finds it, a floor at P* or above proves there is nothing heavier, and the root's bound or more ends
/// the search at once, on every game of the Java's that needs a search and with the default tactics (the rule of the
/// symmetric pairs among them), on one thread.
#[test]
fn a_floor_below_the_optimum_finds_it_and_one_at_it_proves_nothing_is_heavier() {
    for g in &searched_games(400) {
        let p = prepare_golden::<8>(g, SearchTactics::default());
        floors_around_the_optimum(&p, golden_sum(g), 1, &format!("n={}", g.n));
    }
}

/// [`a_floor_below_the_optimum_finds_it_and_one_at_it_proves_nothing_is_heavier`] on four threads, and with each of the
/// tactics that change the tree (the matching path, the rule off), for a few games.
#[test]
fn floors_hold_on_four_threads_and_under_every_search_path() {
    let tactics = [
        SearchTactics::default(),
        SearchTactics { symmetry: false, ..SearchTactics::default() },
        SearchTactics::matching(),
        SearchTactics { symmetry: true, ..SearchTactics::matching() },
    ];
    for g in searched_games(400).iter().step_by(9) {
        for tactics in tactics {
            let p = prepare_golden::<8>(g, tactics);
            let what = format!("n={}, gf2 {}, symmetry {}", g.n, tactics.gf2, tactics.symmetry);
            floors_around_the_optimum(&p, golden_sum(g), 4, &what);
        }
    }
}

/// The floor at P* and the seed of an optimal set are one search, on a few games of 300 to 1000 (300 and 480 in a quick
/// run).
#[test]
fn a_floor_at_the_optimum_searches_the_tree_of_the_search_seeded_with_it() {
    let games: &[u32] = if QUICK { &[300, 480] } else { &[300, 480, 600, 760, 930, 1000] };
    for &n in games {
        the_tree_under_the_optimum_is_the_proof(n);
    }
}

/// A floor at or above the ceiling ends the search before it starts, as a start does, and stands.
#[test]
fn a_floor_at_the_ceiling_ends_the_search_before_it_starts() {
    let p = prepare_golden::<8>(&load(300), SearchTactics::default());
    let opt = search_checked(&p, 1, "plain").sum;
    for (floor, ceiling) in [(opt, opt), (opt + 10, opt)] {
        let r = search_above(&p, &config(1), floor, Some(ceiling), false).unwrap();
        assert!(r.completed && r.reached_cap && r.floor_held && r.stats.nodes == 0, "floor {floor}, ceiling {ceiling}");
        assert_eq!((r.sum, r.promotions), (floor, Bits::EMPTY), "floor {floor}, ceiling {ceiling}");
    }
    // Below the ceiling the floor is searched: P* is the ceiling, and a floor under it finds it and stops there.
    let r = search_above(&p, &config(1), opt - 1, Some(opt), false).unwrap();
    assert!(r.completed && r.reached_cap && !r.floor_held && r.sum == opt, "floor {}", opt - 1);
}

/// A search under a floor that a limit stops resumes from its checkpoint under the same floor, to the same end; a
/// checkpoint that holds the floor and no set is not resumed under another floor or none, nor the other way round, and
/// the dual bound of the stopped search holds the floor as its incumbent.
#[test]
fn a_search_under_a_floor_resumes_under_the_same_floor() {
    let n = 1000;
    let board = crate::board::Board::<16>::for_n(n);
    let p = board.prepare(SearchTactics::default()).unwrap();
    let opt = crate::game::Optimal::built_in().score(n).unwrap() - board.in_the_bag;
    let dir = TempDir::new("floor-resume");
    let ckpt = dir.0.join("ckpt.bin");
    // The floors: one below P* (it is found) and one at it (nothing is), each stopped by a limit of 1,000 steps
    // (the engine checks it every 1,024).
    for floor in [opt - 100, opt] {
        let whole = search_above(&p, &config(1), floor, None, false).unwrap();
        assert!(whole.stats.nodes > 1100, "floor {floor}: {} steps, too few to stop", whole.stats.nodes);
        let cfg = EngineConfig {
            checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
            node_limit: Some(1000),
            ..config(1)
        };
        let mut result = search_above(&p, &cfg, floor, None, false).unwrap();
        assert!(
            !result.completed && ckpt.exists(),
            "floor {floor}: stopped with a checkpoint ({} of {} steps)",
            result.stats.nodes,
            whole.stats.nodes
        );
        let dual = dual_bound(&p, &ckpt).unwrap();
        assert_eq!(dual.incumbent, result.sum, "floor {floor}");
        assert!(dual.bound() >= opt && dual.bound() <= root_bound(&p), "floor {floor}: {}", dual.bound());
        if floor == opt {
            assert_eq!(dual.incumbent, floor, "nothing above the floor is found");
            for (other, what) in
                [(Some(floor - 1), "a lower floor"), (Some(floor + 1), "a higher floor"), (None, "none")]
            {
                let resumed = board.prepare_for_resume(&ckpt).unwrap();
                let refused = match other {
                    Some(other) => search_above(&resumed, &cfg, other, None, true),
                    None => search(&resumed, &cfg, None, None, true),
                };
                assert!(refused.is_err(), "the checkpoint of the floor {floor} is resumed under {what}");
            }
        }
        let mut pieces = 1;
        while !result.completed {
            let resumed = board.prepare_for_resume(&ckpt).unwrap();
            result = search_above(&resumed, &cfg, floor, None, true).unwrap();
            pieces += 1;
            assert!(pieces < 100, "no progress");
        }
        assert!(pieces >= 2 && !ckpt.exists(), "floor {floor}: {pieces} pieces");
        assert_eq!((result.floor_held, result.sum), (floor == opt, opt.max(floor)), "floor {floor}");
        assert_eq!(result.promotions.sum() == opt, floor < opt, "floor {floor}");
    }
}

/// A start that reaches the ceiling is the incumbent as it stands: the search skips the dive and the local search,
/// which could not have changed it, and ends before it starts, with that start.
#[test]
fn a_start_at_the_ceiling_is_neither_dived_nor_improved() {
    let g = load(300);
    let p = prepare_golden::<8>(&g, SearchTactics::default());
    let optimum = search_checked(&p, 1, "plain");
    let graph = MatchGraph::new(&p.model, &p.root);
    let hood = Neighborhood { model: &p.model, root: &p.root, candidates: &p.candidates };

    // The seed (an optimal set, plus a non-candidate) reaches the ceiling, which P* is: the start is the better of
    // the greedy set and the seed's survivors, and the dive and the local search leave it as it is.
    let seed = optimum.promotions.with(299);
    let kept = surviving(&p, &seed);
    let begun = if kept.sum() > p.greedy.sum() { kept } else { p.greedy };
    assert_eq!(begun.sum(), optimum.sum);
    assert!(dive(&p.model, &p.root, &graph, &p.candidates).sum() <= begun.sum());
    assert_eq!(hood.improve(&mut Scratch::new(), &begun, true), begun);
    let r = search(&p, &config(1), Some(&seed), Some(optimum.sum), false).unwrap();
    assert!(r.completed && r.reached_cap && r.stats.nodes == 0 && r.found.is_none());
    assert_eq!((r.start, r.promotions, r.sum), (begun, begun, optimum.sum));

    // A ceiling below P* (which a game's proof never offers) that the greedy set reaches shows that the local
    // search is skipped: it would have raised the start, as it does without the ceiling.
    let improved = hood.improve(&mut Scratch::new(), &p.greedy, true);
    assert!(improved.sum() > p.greedy.sum(), "the local search leaves the greedy set of 300 as it is");
    let r = search(&p, &config(1), None, Some(p.greedy.sum()), false).unwrap();
    assert!(r.completed && r.reached_cap && r.stats.nodes == 0);
    assert_eq!((r.start, r.promotions), (p.greedy, p.greedy));
}

/// The start is offered to the control as it is made, before the engine runs: a search that ends at its start, as a
/// ceiling that the start reaches ends it, leaves the control with that start, whether it is the greedy set, the
/// seed's survivors, the dive or the local search that made it.
#[test]
fn the_start_is_offered_to_the_control_as_it_is_made() {
    let n = 930;
    let board = crate::board::Board::<16>::for_n(n);
    let p = board.prepare(SearchTactics::default()).unwrap();
    let graph = MatchGraph::new(&p.model, &p.root);
    let hood = Neighborhood { model: &p.model, root: &p.root, candidates: &p.candidates };
    let dived = dive(&p.model, &p.root, &graph, &p.candidates);
    let improved = hood.improve(&mut Scratch::new(), &dived, true);
    let seed = board.seed(crate::game::Optimal::built_in().moves(n - 1).unwrap());
    let kept = surviving(&p, &seed);
    assert!(p.greedy.sum() < dived.sum() && dived.sum() < improved.sum(), "each step of the start improves it");
    assert!(p.greedy.sum() < kept.sum(), "the seed's survivors beat the greedy set");

    // Each ceiling is the sum of one step: the search ends there, and the control has that step.
    let steps = [("the greedy set", None, p.greedy), ("the dive", None, dived), ("the local search", None, improved)];
    let seeded = [("the seed", Some(&seed), kept)];
    for (what, seed, step) in steps.into_iter().chain(seeded) {
        let control = Control::new();
        let cfg = EngineConfig { control: Some(Arc::clone(&control)), ..config(1) };
        assert_eq!(best_so_far::<16>(&control), None);
        let r = search(&p, &cfg, seed, Some(step.sum()), false).unwrap();
        assert!(r.completed && r.reached_cap && r.stats.nodes == 0, "{what}");
        assert_eq!((r.start, r.promotions), (step, step), "{what}: the start");
        assert_eq!(best_so_far::<16>(&control), Some(step), "{what}: the control's best");
    }
}

/// The start of a search does not depend on its threads: the local search on several of them finds what one does,
/// with and without a seed, in the search of a game as a whole.
#[test]
fn the_start_is_the_same_on_every_number_of_threads() {
    for n in [300, 600] {
        let board = crate::board::Board::<16>::for_n(n);
        let p = board.prepare(SearchTactics::default()).unwrap();
        let optimal = crate::game::Optimal::built_in();
        let seed = board.seed(optimal.moves(n - 1).unwrap());
        let cap = board.promotion_cap(optimal.score(n - 1).unwrap());
        for (seed, cap) in [(None, None), (Some(&seed), None), (Some(&seed), cap)] {
            let one = search(&p, &config(1), seed, cap, false).unwrap();
            for threads in [2, 4] {
                let more = search(&p, &config(threads), seed, cap, false).unwrap();
                let what = format!("n={n}, seeded {}, ceiling {cap:?}, {threads} threads", seed.is_some());
                assert_eq!((more.start, more.sum, more.completed), (one.start, one.sum, one.completed), "{what}");
            }
        }
    }
}

/// A search with two seeds starts from the better of their starts, not from the seed whose survivors weigh more.  The
/// bounds store's lower-bound game of the board is a game of it, so all of it survives; here it is the start with no
/// seed, S0, a local optimum.  In game 187 S0 weighs 751 and what survives of the game of 186 only 748, so the seed with
/// more survivors is S0, but the start from the game of 186 reaches 762.  Starting from S0 alone leaves the search at
/// 751; the best of the two starts is 762.
#[test]
fn the_start_is_the_best_of_the_starts_of_the_seeds() {
    let n = 187;
    let board = Board::<8>::for_n(n);
    let p = board.prepare(SearchTactics::default()).unwrap();
    let graph = MatchGraph::new(&p.model, &p.root);
    let hood = Neighborhood { model: &p.model, root: &p.root, candidates: &p.candidates };
    let prev = board.seed(crate::game::Optimal::built_in().moves(n - 1).unwrap());
    let start = |seeds: &[Bits<8>]| start_incumbent(&p, &graph, &Control::new(), 1, seeds);
    let (s0, from_prev) = (start(&[]), start(&[prev]));

    // The trap: S0 has the survivors that weigh more, and the start from the game of n − 1 climbs above it.
    assert!(seed_survivors_sum(&p, &s0) > seed_survivors_sum(&p, &prev), "S0 survives heavier than the previous game");
    assert_eq!((start(&[s0]), s0.sum(), from_prev.sum()), (s0, 751, 762), "the start from S0 alone stays at S0");

    // Either order of the seeds starts from the better start.
    for seeds in [[s0, prev], [prev, s0]] {
        assert_eq!(start(&seeds), from_prev, "seeds in the order {:?}", seeds.map(|seed| seed.sum()));
    }
    // A search with the two seeds starts there: a ceiling at that start ends it at once, and the control has it.
    let control = Control::new();
    let cfg = EngineConfig { control: Some(Arc::clone(&control)), ..config(1) };
    let r = search_beside(&p, &cfg, &[s0, prev], Some(from_prev.sum()), false, None).unwrap();
    assert!(r.completed && r.reached_cap && r.stats.nodes == 0);
    assert_eq!((r.start, r.promotions), (from_prev, from_prev));
    assert_eq!(best_so_far::<8>(&control), Some(from_prev));

    // With one seed, or two that give the same start, the start is that of the one: the steps of the start spelled
    // out (the greedy set, the survivors, the dive, the local search), as a search with one seed has always made it.
    let steps = |seed: Option<&Bits<8>>| {
        let mut begun = p.greedy;
        if let Some(kept) = seed.map(|seed| surviving(&p, seed)).filter(|kept| kept.sum() > begun.sum()) {
            begun = kept;
        }
        let dived = dive(&p.model, &p.root, &graph, &p.candidates);
        if dived.sum() > begun.sum() {
            begun = dived;
        }
        hood.improve(&mut Scratch::new(), &begun, true)
    };
    assert_eq!((s0, from_prev), (steps(None), steps(Some(&prev))));
    assert_eq!((start(&[prev, prev]), start(&[s0, s0])), (from_prev, s0));
}

/// A seed given twice is one seed: the search has the start and the tree of the search with it once.
#[test]
fn a_seed_given_twice_makes_the_search_of_one() {
    let n = 300;
    let board = Board::<8>::for_n(n);
    let p = board.prepare(SearchTactics::default()).unwrap();
    let seed = board.seed(crate::game::Optimal::built_in().moves(n - 1).unwrap());
    let once = search(&p, &config(1), Some(&seed), None, false).unwrap();
    let twice = search_beside(&p, &config(1), &[seed, seed], None, false, None).unwrap();
    assert!(once.completed && twice.completed && once.stats.nodes > 0);
    assert_eq!((twice.start, twice.promotions, twice.sum), (once.start, once.promotions, once.sum));
    assert_eq!(twice.stats.nodes, once.stats.nodes);
}

/// A ceiling given to the control of a search ends it when the incumbent reaches it, whether it is given before the
/// search starts, as `cap` is, or from another thread while the search runs.  Game 988 starts from an incumbent
/// 155 below its optimum, which the search finds at its step 4,614 of 6,287, a third of a second after it starts
/// (the other thread has that long to give the ceiling).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is a_ceiling_given_before_the_search_to_the_control_ends_it"
)]
fn a_ceiling_given_to_the_control_ends_the_search() {
    a_ceiling_given_to_the_control_ends_the_search_of(988, true);
}

/// [`a_ceiling_given_to_the_control_ends_the_search`] for game 1000, whose search is too short to give the ceiling to
/// while it runs: given before the search, in the control or as `cap`.
#[test]
fn a_ceiling_given_before_the_search_to_the_control_ends_it() {
    a_ceiling_given_to_the_control_ends_the_search_of(1000, false);
}

/// A ceiling given to the control of the search of game `n` ends it, before it starts and, if `live`, from another
/// thread while it runs.
fn a_ceiling_given_to_the_control_ends_the_search_of(n: u32, live: bool) {
    let board = crate::board::Board::<16>::for_n(n);
    let p = board.prepare(SearchTactics::default()).unwrap();
    let optimum = crate::game::Optimal::built_in().score(n).unwrap() - board.in_the_bag;
    let whole = search(&p, &config(1), None, None, false).unwrap();
    assert!(whole.completed && !whole.reached_cap && whole.sum == optimum);
    assert!(whole.start.sum() < optimum, "the search has nothing to do from its start");

    // Given before the search, in the control or as `cap`: the same search, which ends where it finds P*.
    let control = Control::new();
    control.set_ceiling(optimum);
    let cfg = EngineConfig { control: Some(Arc::clone(&control)), ..config(1) };
    let given = search(&p, &cfg, None, None, false).unwrap();
    let capped = search(&p, &config(1), None, Some(optimum), false).unwrap();
    assert!(given.completed && given.reached_cap && given.sum == optimum);
    assert_eq!((given.sum, given.stats.nodes), (capped.sum, capped.stats.nodes));
    assert!(given.stats.nodes < whole.stats.nodes, "{} steps, as many as without the ceiling", given.stats.nodes);

    if !live {
        return;
    }
    // Given from another thread once the run goes.
    let control = Control::new();
    let cfg = EngineConfig { control: Some(Arc::clone(&control)), ..config(1) };
    let live = thread::scope(|scope| {
        let run = scope.spawn(|| search(&p, &cfg, None, None, false).unwrap());
        while control.status().is_none() && !run.is_finished() {
            thread::yield_now();
        }
        control.set_ceiling(optimum);
        run.join().unwrap()
    });
    assert!(live.completed && live.reached_cap && live.sum == optimum, "{live:?}");
    assert!(live.stats.nodes < whole.stats.nodes, "{} steps, as many as without the ceiling", live.stats.nodes);
    assert_eq!(best_so_far::<16>(&control), Some(live.promotions));
    assert_eq!(control.ceiling(), Some(optimum));
}

/// The previous game's promotions offered to the control before the run raise its start as `seed` does: game 930
/// starts from 20,469 on its own and from its optimum, 20,480, with the seed, and the search is the same either way.
#[test]
fn a_seed_offered_to_the_control_is_the_seed_of_the_search() {
    let n = 930;
    let board = crate::board::Board::<16>::for_n(n);
    let p = board.prepare(SearchTactics::default()).unwrap();
    let seed = board.seed(crate::game::Optimal::built_in().moves(n - 1).unwrap());
    let seeded = search(&p, &config(1), Some(&seed), None, false).unwrap();
    let unseeded = search(&p, &config(1), None, None, false).unwrap();
    assert!(seeded.start.sum() > unseeded.start.sum() && seeded.found.is_none() && unseeded.found.is_some());

    let control = Control::new();
    let cfg = EngineConfig { control: Some(Arc::clone(&control)), ..config(1) };
    assert_eq!(best_so_far::<16>(&control), None);
    let kept = surviving(&p, &seed);
    assert!(offer_seed(&p, &control, &seed));
    assert_eq!(best_so_far::<16>(&control), Some(kept));
    // What does not beat the incumbent is not taken.
    assert!(!offer_seed(&p, &control, &Bits::EMPTY));
    assert_eq!(best_so_far::<16>(&control), Some(kept));

    let offered = search(&p, &cfg, None, None, false).unwrap();
    assert!(offered.completed && offered.found.is_none());
    assert_eq!((offered.sum, offered.promotions), (seeded.sum, seeded.promotions));
    assert_eq!(offered.stats.nodes, seeded.stats.nodes);
    assert_eq!(best_so_far::<16>(&control), Some(offered.promotions));
}

#[test]
fn units_and_headers_round_trip() {
    let unit = TaxUnit::<3> { promoted: Bits::from_slice(&[1, 64, 190]), cands: Bits::from_slice(&[0, 63, 127]) };
    let mut bytes = Vec::new();
    unit.encode(&mut bytes);
    assert_eq!(bytes.len(), 48);
    assert_eq!(TaxUnit::<3>::decode(&bytes), unit);

    // The header of version 5 is the one of a search tuned as the builds before the tactics that tune the GF(2) search
    // were (`force_window` 80), which it implies; the default's `force_window` is `all`.
    let p = prepare_golden::<8>(&load(300), SearchTactics { match_refine: 3, ..SearchTactics::legacy_tuning() });
    let header = Header::of(&p);
    let bytes = header.encode();
    assert_eq!(bytes.len(), Header::<8>::LEN);
    let back = Header::<8>::decode(&bytes).unwrap();
    assert_eq!(back, header);
    assert_eq!((back.n, back.max_total, back.tactics), (300, p.max_total, p.tactics));
    assert_eq!(back.mismatch(&header), None);
    assert!(Header::<16>::decode(&bytes).unwrap_err().contains("Bits<8>"));
    assert!(Header::<8>::decode(b"nonsense").unwrap_err().contains("not a checkpoint"));
    assert!(Header::<8>::decode(&bytes[..20]).is_err());
    // Version 4, the format of the builds before the extended problem's search, is version 5 without the digest of the
    // one-sided candidates (`max_total` and the tactics follow the digest of the frames): it reads as this header with
    // the digest of none, which this problem (no one-sided candidates) has, so nothing differs but the version.
    let v4 = [&bytes[..4], &4u32.to_le_bytes(), &bytes[8..32], &bytes[40..]].concat();
    assert_eq!(v4.len(), Header::<8>::LEN_4);
    let read = Header::<8>::decode(&v4).unwrap();
    assert_eq!(read, Header { version: 4, ..header });
    assert_eq!(read.one_sided, Header::<8>::digest_one_sided(&[]));
    assert_eq!(read.mismatch(&header), None);
    // A version-4 header of a search with one-sided candidates cannot exist, and one is refused for such a search,
    // saying so before anything else.
    let extended = Header { one_sided: Header::<8>::digest_one_sided(&[(150, 0)]), ..header };
    let refused = read.mismatch(&extended).unwrap();
    assert!(refused.contains("format version 4") && refused.contains("one-sided candidates"), "{refused}");
    // A version-4 number with a header of version 5's length is refused for its length, as any short or long header.
    let versioned = |version: u32| [&bytes[..4], &version.to_le_bytes(), &bytes[8..]].concat();
    let err = Header::<8>::decode(&versioned(4)).unwrap_err();
    assert!(err.contains(&format!("{} bytes, not {}", Header::<8>::LEN, Header::<8>::LEN_4)), "{err}");
    // Every other format version, the older ones (`TXS2` was version 2; version 3 recorded tactics that are gone) and
    // any later one, is refused with one message.
    let mut txs2 = b"TXS2".to_vec();
    txs2.extend_from_slice(&[0; 4 + 64 + 4 + 1 + 4 + 2]);
    assert_eq!(Header::<8>::VERSION, 5);
    assert_eq!(Header::<8>::decode(&versioned(5)), Ok(header));
    for (version, bytes) in
        [("2", txs2), ("3", versioned(3))].into_iter().chain([("7", versioned(7)), ("unreadable", b"TXSH".to_vec())])
    {
        let err = Header::<8>::decode(&bytes).unwrap_err();
        let message = format!("format version {version}, and this build reads versions 4 to 6 only");
        assert!(err.contains(&message), "{err}");
    }
    // A search whose tactics that tune the GF(2) search are not those of the builds before them writes version 6, which
    // records them after the switches' bits; one whose are writes version 5, which implies them.  Version 6's number
    // with version 5's length is refused for its length.
    assert_eq!(header.version, 5);
    assert_eq!(header.tactics.force_window, ForceWindow::Within(80));
    let tuned = Header {
        tactics: SearchTactics {
            force_window: ForceWindow::Share(40),
            branching: Gf2Branching::Forced,
            strong_limit: 4,
            strong_depth: 30,
            deep_branching: Gf2Branching::Largest,
            more_forced: false,
            ..header.tactics
        },
        version: 6,
        ..header
    };
    let bytes6 = tuned.encode();
    assert_eq!(bytes6.len(), Header::<8>::LEN_6);
    assert_eq!(bytes6[..bytes.len()], [&bytes[..4], &6u32.to_le_bytes(), &bytes[8..]].concat()[..]);
    assert_eq!(Header::<8>::decode(&bytes6), Ok(tuned));
    assert!(tuned.mismatch(&header).unwrap().contains("tactics"));
    // The default's tuning is not the builds before's (`force_window` is `all`, it was 80), so its header is version 6;
    // a checkpoint of version 5 is of a search with the window of 80 and is not resumed by one with the default's.
    let current =
        Header::of(&prepare_golden::<8>(&load(300), SearchTactics { match_refine: 3, ..SearchTactics::default() }));
    assert_eq!(current.version, 6);
    assert_eq!(current.tactics.force_window, ForceWindow::All);
    assert_eq!(Header::<8>::decode(&current.encode()), Ok(current));
    assert_eq!(current.encode().len(), Header::<8>::LEN_6);
    assert!(header.mismatch(&current).unwrap().contains("tactics"));
    assert_eq!(Header::<8>::decode(&bytes).unwrap().tactics.force_window, ForceWindow::Within(80));
    let err = Header::<8>::decode(&versioned(6)).unwrap_err();
    assert!(err.contains(&format!("{} bytes, not {}", Header::<8>::LEN, Header::<8>::LEN_6)), "{err}");
    for window in [ForceWindow::Within(0), ForceWindow::Within(200), ForceWindow::All] {
        for branching in Gf2Branching::ALL {
            let t = SearchTactics { force_window: window, branching, ..tuned.tactics };
            let h = Header { tactics: t, ..tuned };
            assert_eq!(Header::<8>::decode(&h.encode()), Ok(h));
        }
    }
    // An unknown tuning is refused.
    let mut bad = bytes6.clone();
    bad[Header::<8>::LEN + 5] = 9;
    assert!(Header::<8>::decode(&bad).unwrap_err().contains("unknown branching"));
    // Every field that differs is named.
    let other = |change: fn(&mut Header<8>)| {
        let mut h = header;
        change(&mut h);
        h.mismatch(&header).unwrap()
    };
    assert!(other(|h| h.n = 301).contains("n=301, not n=300"));
    assert!(other(|h| h.candidates ^= 1).contains("candidates"));
    assert!(other(|h| h.frames ^= 1).contains("frames"));
    assert!(other(|h| h.one_sided ^= 1).contains("one-sided candidates"));
    assert!(other(|h| h.max_total += 1).contains("max_total"));
    assert!(other(|h| h.tactics.nogoods = !h.tactics.nogoods).contains("tactics"));
    // The policy of keeping and forgetting nogoods is not recorded, so a header made under another is this search's,
    // and its bytes are those of the default's.
    let mut policy = header;
    policy.tactics = SearchTactics { nogood_period: 7, nogood_keep: Keep::Top(3), nogood_limit: 9, ..policy.tactics };
    assert_eq!(policy.mismatch(&header), None);
    assert_eq!(policy.encode(), header.encode());
    // The branching switch is recorded, as the search began with it, but binds nothing: a header with it the other way
    // is this search's too.
    let mut branching = header;
    branching.tactics.conflict_branching = !header.tactics.conflict_branching;
    assert_eq!(branching.mismatch(&header), None);
    assert_ne!(branching.encode(), header.encode());
    // So is the look, which is on by default: a header with it off, as the builds before it wrote, is this search's.
    let mut unlooking = header;
    unlooking.tactics.conflict_look = false;
    assert_eq!(unlooking.mismatch(&header), None);
    assert_ne!(unlooking.encode(), header.encode());
    assert_eq!(Header::<8>::decode(&unlooking.encode()), Ok(unlooking));
    assert!(header.tactics.conflict_look);

    // So is the near-repeat cut, which is on by default.
    let mut cutting = header;
    cutting.tactics.near_repeat = !header.tactics.near_repeat;
    assert_eq!(cutting.mismatch(&header), None);
    assert_ne!(cutting.encode(), header.encode());
    assert_eq!(Header::<8>::decode(&cutting.encode()), Ok(cutting));
    assert!(header.tactics.near_repeat);

    // So is the reuse of prices, which is off by default: a header with it on is this search's, and the builds before
    // it wrote none.
    let mut reusing = header;
    reusing.tactics.price_reuse = true;
    assert_eq!(reusing.mismatch(&header), None);
    assert_eq!(header.mismatch(&reusing), None);
    assert_ne!(reusing.encode(), header.encode());
    assert_eq!(Header::<8>::decode(&reusing.encode()), Ok(reusing));
    assert!(!header.tactics.price_reuse);

    // So is the carrying of nogoods, which is off by default.
    let mut carrying = header;
    carrying.tactics.carried_nogoods = true;
    assert_eq!(carrying.mismatch(&header), None);
    assert_eq!(header.mismatch(&carrying), None);
    assert_ne!(carrying.encode(), header.encode());
    assert_eq!(Header::<8>::decode(&carrying.encode()), Ok(carrying));
    assert!(!header.tactics.carried_nogoods);

    // Every setting of the search tactics reads back as it was written: `match_refine` in bits 0 to 2, a bit for
    // each switch that is on (the defaults: 0xFB9, `symmetry` is bit 8, `conflict_branching` bit 9, `conflict_look`
    // bit 10, `near_repeat` bit 11, and `price_reuse` bit 12 and `carried_nogoods` bit 13, off by default), and no
    // higher bit is used.
    for refine in [0, 1, 7] {
        for (on, off) in [(true, false), (false, true)] {
            let t = SearchTactics {
                match_refine: refine,
                fix_forced: on,
                nogoods: off,
                forget_nogoods: on,
                lagrange: off,
                gf2: on,
                symmetry: off,
                conflict_branching: off,
                near_repeat: off,
                ..SearchTactics::legacy_tuning()
            };
            assert_eq!(SearchTactics::from_bits(t.bits()), Ok(t));
            let t = SearchTactics { symmetry: on, ..t };
            assert_eq!(SearchTactics::from_bits(t.bits()), Ok(t));
            assert_eq!(t.bits() >> 8 & 1, u16::from(on));
            let t = SearchTactics { conflict_branching: on, ..t };
            assert_eq!(SearchTactics::from_bits(t.bits()), Ok(t));
            assert_eq!(t.bits() >> 9 & 1, u16::from(on));
            let t = SearchTactics { near_repeat: on, ..t };
            assert_eq!(SearchTactics::from_bits(t.bits()), Ok(t));
            assert_eq!(t.bits() >> 11, u16::from(on));
            let t = SearchTactics { price_reuse: on, ..t };
            assert_eq!(SearchTactics::from_bits(t.bits()), Ok(t));
            assert_eq!(t.bits() >> 12, u16::from(on));
            let t = SearchTactics { carried_nogoods: on, ..t };
            assert_eq!(SearchTactics::from_bits(t.bits()), Ok(t));
            assert_eq!(t.bits() >> 13, u16::from(on));
        }
    }
    assert_eq!(SearchTactics::default().bits(), 0xFB9);
    assert_eq!(SearchTactics::from_bits(0xFB9), Ok(SearchTactics::legacy_tuning()));
    // A checkpoint written before `near_repeat` has bit 11 clear, so it reads with the tactic off; the bit binds
    // nothing, and a run that resumes a search says it.
    let uncut = SearchTactics { near_repeat: false, ..SearchTactics::legacy_tuning() };
    assert_eq!(SearchTactics::from_bits(0x7B9), Ok(uncut));
    assert_eq!(SearchTactics::NEAR_REPEAT_BIT, 1 << 11);
    assert!(uncut.same_recorded(&SearchTactics::legacy_tuning()));
    assert!(SearchTactics::legacy_tuning().same_recorded(&uncut));
    assert_eq!(uncut.with_unbound_of(&SearchTactics::default()), SearchTactics::legacy_tuning());
    assert!(!uncut.same_recorded(&SearchTactics { symmetry: false, ..uncut }));
    // A checkpoint written before `conflict_branching` has bit 9 clear, so it reads with the tactic off.  The bit only
    // chooses how nodes branch, and binds nothing: each search resumes the other's, under its own setting.  The other
    // switches bind.
    let before = SearchTactics { conflict_branching: false, ..SearchTactics::legacy_tuning() };
    assert_eq!(SearchTactics::from_bits(0xDB9), Ok(before));
    assert_eq!(SearchTactics::from_bits(0xCB9), Ok(SearchTactics { symmetry: false, ..before }));
    assert!(before.same_recorded(&SearchTactics::legacy_tuning()));
    assert!(SearchTactics::legacy_tuning().same_recorded(&before));
    assert_eq!(SearchTactics::BRANCHING_BITS, 1 << 9 | 1 << 10);
    assert!(!before.same_recorded(&SearchTactics { symmetry: false, ..before }));
    let resumed = before.with_unbound_of(&SearchTactics::default());
    assert_eq!(resumed, SearchTactics { conflict_branching: true, ..before });
    // `conflict_look` is bit 10, on by default, so a checkpoint whose bit is clear (written before it, or by a search
    // with it off) reads with it off.  It binds nothing either, and a run that resumes a search says it.
    let unlooking = SearchTactics { conflict_look: false, ..SearchTactics::legacy_tuning() };
    assert_eq!(unlooking.bits(), 0xFB9 & !(1 << 10));
    assert_eq!(SearchTactics::from_bits(0xBB9), Ok(unlooking));
    assert!(unlooking.tuning_is_legacy());
    assert!(unlooking.same_recorded(&SearchTactics::legacy_tuning()));
    assert!(SearchTactics::legacy_tuning().same_recorded(&unlooking));
    assert_eq!(SearchTactics::legacy_tuning().with_unbound_of(&unlooking), unlooking);
    assert_eq!(unlooking.with_unbound_of(&SearchTactics::default()), SearchTactics::legacy_tuning());
    assert!(!unlooking.same_recorded(&SearchTactics { symmetry: false, ..unlooking }));
    // `price_reuse` is bit 12, off by default, so the bit is clear in the defaults' 0xFB9; a checkpoint with it set reads
    // with the tactic on, binds nothing, and a run that resumes a search says it.
    let reusing = SearchTactics { price_reuse: true, ..SearchTactics::legacy_tuning() };
    assert_eq!(reusing.bits(), 0xFB9 | 1 << 12);
    assert_eq!(SearchTactics::from_bits(0x1FB9), Ok(reusing));
    assert_eq!(SearchTactics::PRICE_REUSE_BIT, 1 << 12);
    assert!(reusing.same_recorded(&SearchTactics::legacy_tuning()));
    assert!(SearchTactics::legacy_tuning().same_recorded(&reusing));
    assert_eq!(SearchTactics::legacy_tuning().with_unbound_of(&reusing), reusing);
    assert_eq!(reusing.with_unbound_of(&SearchTactics::default()), SearchTactics::legacy_tuning());
    assert!(!reusing.same_recorded(&SearchTactics { symmetry: false, ..reusing }));
    // `carried_nogoods` is bit 13, off by default, and binds nothing either.
    let carrying = SearchTactics { carried_nogoods: true, ..SearchTactics::legacy_tuning() };
    assert_eq!(carrying.bits(), 0xFB9 | 1 << 13);
    assert_eq!(SearchTactics::from_bits(0x2FB9), Ok(carrying));
    assert_eq!(SearchTactics::CARRIED_NOGOODS_BIT, 1 << 13);
    assert!(carrying.same_recorded(&SearchTactics::legacy_tuning()));
    assert!(SearchTactics::legacy_tuning().same_recorded(&carrying));
    assert_eq!(SearchTactics::legacy_tuning().with_unbound_of(&carrying), carrying);
    assert_eq!(carrying.with_unbound_of(&SearchTactics::default()), SearchTactics::legacy_tuning());
    assert!(!carrying.same_recorded(&SearchTactics { symmetry: false, ..carrying }));
    assert!(SearchTactics::from_bits(1 << 14).unwrap_err().contains("unknown tactics"));
}

// ---- the extended problem: one-sided candidates ----

/// U\*_{≥3} of `board`: the sources that setup keeps (the numbers above n/2 of the game with no promotions) with three
/// or more prime factors, which can be open in the extended problem Π⁺(O).
fn open_sources<const W: usize>(board: &Board<W>) -> Bits<W> {
    board.plain.iter().copied().filter(|&x| 2 * x > board.n() && board.tables.omega(x) >= 3).collect()
}

/// A checkpoint of the search of `prepared` that holds the root unit, made by a deadline that has passed before the
/// search starts, in `path`.
fn root_checkpoint<const W: usize>(prepared: &Prepared<W>, path: &std::path::Path) {
    let cfg = EngineConfig {
        checkpoint: Some((path.to_path_buf(), Duration::from_secs(3600))),
        time_limit: Some(Duration::ZERO),
        ..config(1)
    };
    assert!(!search(prepared, &cfg, None, None, false).unwrap().completed);
    assert!(path.exists(), "a search stopped by its limit leaves a checkpoint");
}

/// The header of a problem with one-sided candidates carries a digest of them, each with its frame, after the digest
/// of the frames: it round-trips, sits where the layout says, differs from that of a problem with none and from that of
/// other one-sided candidates, and the difference is named.
#[test]
fn the_header_of_an_extended_problem_records_its_one_sided_candidates() {
    let board = Board::<8>::for_n(300);
    let open = open_sources(&board);
    assert!(open.len() > 2, "{open:?}");
    let (ext, prepared) = board.prepare_extended_with(&open, false, SearchTactics::extended()).unwrap();
    let (_, empty) = board.prepare_extended(false, SearchTactics::extended()).unwrap();
    assert!(prepared.model.has_one_sided() && !empty.model.has_one_sided());

    let header = Header::of(&prepared);
    let bytes = header.encode();
    // The default's tuning of the GF(2) search is recorded after the tactics' bits (version 6): the layout up to them
    // is version 5's.
    assert_eq!(bytes.len(), Header::<8>::LEN_6);
    assert_eq!(Header::<8>::decode(&bytes), Ok(header));
    // Magic, version, width, n, then the digests of the candidates, of the frames and of the one-sided candidates.
    let digest_at = |at: usize| u64::from_le_bytes(bytes[at..at + 8].try_into().unwrap());
    assert_eq!(digest_at(16), header.candidates);
    assert_eq!(digest_at(24), header.frames);
    assert_eq!(digest_at(32), header.one_sided);
    assert_eq!(Header::<8>::LEN, 46);

    // The digest is of the model's one-sided candidates, which are the open sources and those greedy rejected.
    let digest = |one_sided: &[(u32, usize)]| Header::<8>::digest_one_sided(one_sided);
    assert_eq!(header.one_sided, digest(&ext.one_sided));
    assert_eq!(header.one_sided, digest(prepared.model.one_sided()));
    assert!(ext.one_sided.len() >= open.len() as usize);
    assert_eq!(Header::of(&empty).one_sided, digest(&[]));
    assert_ne!(header.one_sided, digest(&[]));
    // A candidate and its frame are both in the digest, and so is every candidate.
    let mut other_frame = ext.one_sided.clone();
    other_frame[0].1 += 1;
    assert_ne!(digest(&other_frame), header.one_sided);
    assert_ne!(digest(&ext.one_sided[1..]), header.one_sided);
    assert_ne!(digest(&ext.one_sided[..ext.one_sided.len() - 1]), header.one_sided);

    // Another problem's header is refused, saying what differs.
    let message = Header::of(&empty).mismatch(&header).unwrap();
    assert!(message.starts_with("the checkpoint is of other "), "{message}");
    let mut forged = header;
    forged.one_sided = digest(&other_frame);
    let message = forged.mismatch(&header).unwrap();
    assert!(message.contains("other one-sided candidates"), "{message}");
    assert_eq!(header.mismatch(&header), None);
}

/// A checkpoint written for Π⁺(O) is resumed for Π⁺(O) and for nothing else: not for the same board with other open
/// sources, with none, or as the frame model's problem.  The problems differ in their candidates (every open source is
/// one) and in their frames (an open source is no move), well before the digest of the one-sided candidates, so the
/// message names the candidates.  (Of the open sets tried on every board to 400, the empty set, all of U\*_{≥3}, each
/// source alone and six random subsets, no two pose problems with the same candidates, frames and `max_total` and other
/// one-sided candidates, so the refusal for that is tested on a header above and on a checkpoint forged to carry
/// another digest, in `tests/robustness.rs`.)
#[test]
fn a_checkpoint_of_one_extended_problem_is_refused_for_another() {
    let board = Board::<8>::for_n(300);
    let open = open_sources(&board);
    let fewer: Bits<8> = open.iter().take(open.len() as usize / 2).collect();
    assert!(!fewer.is_empty() && fewer != open);
    let dir = TempDir::new("extended-other");
    let checkpoint = dir.0.join("ckpt.bin");
    let (ext, prepared) = board.prepare_extended_with(&open, false, SearchTactics::extended()).unwrap();
    root_checkpoint(&prepared, &checkpoint);

    // The same problem resumes, and finishes with P*.
    let (_, resumed) = board.prepare_extended_for_resume(&open, false, &checkpoint).unwrap();
    assert_eq!((resumed.problem(), resumed.tactics), (prepared.problem(), prepared.tactics));
    let cfg = EngineConfig { checkpoint: Some((checkpoint.clone(), Duration::from_secs(3600))), ..config(1) };
    let result = search(&resumed, &cfg, None, None, true).unwrap();
    let truth = crate::game::Optimal::built_in().score(300).unwrap() - ext.in_the_bag;
    assert!(result.completed && result.sum == truth, "{} not {truth}", result.sum);

    // Another problem is refused, naming what differs, and the checkpoint is left alone.
    root_checkpoint(&prepared, &checkpoint);
    for (what, other) in [("fewer open sources", fewer), ("no open source", Bits::EMPTY)] {
        let err = board.prepare_extended_for_resume(&other, false, &checkpoint).err().expect(what);
        assert!(err.starts_with("the checkpoint is of other promotion candidates"), "{what}: {err}");
    }
    let err = board.prepare_for_resume(&checkpoint).err().expect("the frame model");
    assert!(err.starts_with("the checkpoint is of other "), "the frame model: {err}");
    let next = Board::<8>::for_n(301);
    let err = next.prepare_extended_for_resume(&open_sources(&next), false, &checkpoint).err().expect("another board");
    assert!(err.contains("the game n=300, not n=301"), "another board: {err}");
    assert!(checkpoint.exists(), "a refused checkpoint is left alone");
    // The tactics of a checkpoint that the problem refuses are refused when it is resumed, by the setup: a checkpoint
    // of Π⁺(∅) under the default tactics, with the rule of the symmetric pairs, resumed for Π⁺(O).
    root_checkpoint(&board.prepare_extended(false, SearchTactics::default()).unwrap().1, &checkpoint);
    let err =
        board.prepare_extended_for_resume(&open, false, &checkpoint).err().expect("tactics that cannot search it");
    assert!(err.contains("one-sided") && err.contains("symmetry=on"), "{err}");
}

/// A search with one-sided candidates needs the GF(2) bound and refuses the Lagrangian bound and the rule of the
/// symmetric pairs, and the setup says so, naming each tactic that is wrong; the tactics of extended mode are accepted.
/// With no open source (and no source greedy rejected) there is no one-sided candidate and every tactics are accepted,
/// the matching path's included.
#[test]
fn a_problem_with_one_sided_candidates_is_set_up_only_for_the_tactics_that_know_them() {
    let board = Board::<8>::for_n(200);
    let open = open_sources(&board);
    assert!(!open.is_empty());
    let extended = SearchTactics::extended();
    assert_eq!(extended, SearchTactics { symmetry: false, ..SearchTactics::default() });
    assert_eq!(extended.refusal_with_one_sided(), None);

    let (ext, prepared) = board.prepare_extended_with(&open, false, extended).unwrap();
    assert!(prepared.model.has_one_sided() && prepared.tactics == extended);
    assert_eq!(prepared.candidates, ext.one_sided.iter().fold(ext.candidates, |set, &(s, _)| set.with(s)));
    assert_eq!(prepared.model.one_sided(), ext.one_sided.as_slice());

    let (gf2, lagrange, symmetry) = ("gf2=off", "lagrange=on", "symmetry=on");
    let cases = [
        ("the default", SearchTactics::default(), vec![symmetry]),
        ("the matching path", SearchTactics::matching(), vec![gf2, lagrange]),
        ("no GF(2) bound", SearchTactics { gf2: false, ..extended }, vec![gf2]),
        ("the Lagrangian bound", SearchTactics { lagrange: true, ..extended }, vec![lagrange]),
        ("the rule, no bound", SearchTactics { gf2: false, symmetry: true, ..extended }, vec![gf2, symmetry]),
        (
            "all three",
            SearchTactics { gf2: false, lagrange: true, symmetry: true, ..extended },
            vec![gf2, lagrange, symmetry],
        ),
    ];
    for &(what, tactics, ref wrong) in &cases {
        let err =
            board.prepare_extended_with(&open, false, tactics).err().unwrap_or_else(|| panic!("{what}: accepted"));
        assert!(err.contains("one-sided candidates (extended mode) cannot run with"), "{what}: {err}");
        assert_eq!(Some(err.clone()), tactics.refusal_with_one_sided(), "{what}");
        for tactic in [gf2, lagrange, symmetry] {
            assert_eq!(err.contains(tactic), wrong.contains(&tactic), "{what}: {tactic} in {err}");
        }
        // The setup of a resumed search takes the tactics of the checkpoint through the same check, and the setup that
        // takes no one-sided candidate (the default path's) is not asked.
        let columns = ext.columns();
        let refused = prepare_extended(&board.tables, &columns, &ext.one_sided, &ext.candidates, tactics);
        assert_eq!(refused.err().as_ref(), Some(&err), "{what}: prepare_extended");
        assert!(prepare::<8>(&board.tables, &columns, &ext.candidates, tactics).is_ok(), "{what}: prepare");
    }

    // With nothing open, any tactics: Π⁺(∅) is the frame model's problem.
    for &(what, tactics, _) in &cases {
        let (empty, prepared) = board.prepare_extended(false, tactics).unwrap_or_else(|e| panic!("{what}: {e}"));
        assert!(!prepared.model.has_one_sided() && empty.one_sided.is_empty(), "{what}");
        let (_, same) = board.prepare_extended_with(&Bits::EMPTY, false, tactics).unwrap();
        assert_eq!((same.problem(), same.tactics), (prepared.problem(), tactics), "{what}");
    }
}

/// An open source adds one to the rank of its frame, so a problem with many open sources can need more block words of
/// the GF(2) bound than the engines of a narrow set width have (three, rank 192; five, rank 320, for the widest): the
/// setup refuses it, naming the words, instead of failing in the search; at 128 words it is accepted.  With every
/// source of three or more prime factors open, frame 0 of the board 1265 has rank 192 or less and that of 1266 more.
#[test]
fn a_problem_whose_frame_needs_more_block_words_than_the_set_width_has_is_refused() {
    fn words<const W: usize>(prepared: &Prepared<W>) -> usize {
        Gf2Graph::new(&prepared.model, &prepared.root, &prepared.candidates).words()
    }
    assert_eq!((Gf2Bound::<20>::MOST_WORDS, Gf2Bound::<64>::MOST_WORDS, Gf2Bound::<128>::MOST_WORDS), (3, 3, 5));

    // Below the boundary the problem fits the narrow engines (the rank is high, near 192).
    let board = Board::<20>::for_n(1265);
    let (_, fits) = board.prepare_extended_with(&open_sources(&board), false, SearchTactics::extended()).unwrap();
    assert_eq!(words(&fits), 3);

    for n in [1266, 1300] {
        let narrow = crate::with_width!(n, W => {
            let board = Board::<W>::for_n(n);
            let err = board.prepare_extended_with(&open_sources(&board), false, SearchTactics::extended());
            (W, err.err().unwrap_or_else(|| panic!("n={n}: accepted at W={W}")))
        });
        let (width, err) = narrow;
        assert!(width < 64, "n={n}: the natural width {width}");
        let wanted = format!(
            "n={n}: a frame of the problem needs 4 block words of the GF(2) bound, and the set width {width} has \
             engines of at most 3 "
        );
        assert!(err.starts_with(&wanted), "n={n}: {err}");

        // The widest set holds engines of five words, and the engine of four is made for the problem it accepts.
        let wide = Board::<128>::for_n(n);
        let (_, accepted) = wide.prepare_extended_with(&open_sources(&wide), false, SearchTactics::extended()).unwrap();
        assert_eq!(words(&accepted), 4, "n={n}: the words the problem needs");
        let graph = Gf2Graph::new(&accepted.model, &accepted.root, &accepted.candidates);
        let mut bound = Gf2Bound::<128>::new(&graph);
        bound.reset(&Bits::EMPTY, &accepted.candidates);
        let root = bound.evaluate(accepted.max_total, None).value;
        assert!(
            root >= accepted.greedy.sum(),
            "n={n}: the root's bound {root}, the greedy set {}",
            accepted.greedy.sum()
        );
    }
}

/// The form of `problem` ([`Problem`]'s list of numbers).
fn form_of(problem: &Problem) -> Vec<u32> {
    problem.form().to_vec()
}

/// Where the part of `form` that a problem without one-sided candidates ends: after the frames (their freedoms, moves
/// and factors), the candidates, `max_total`, and for each number of the frames, ascending, its record.  Checked as it
/// is walked: the frames of `prepared` are what it lists.
fn plain_part_of<const W: usize>(form: &[u32], prepared: &Prepared<W>) -> usize {
    let list = |at: &mut usize| -> Vec<u32> {
        let len = form[*at] as usize;
        *at += 1 + len;
        form[*at - len..*at].to_vec()
    };
    let frames = prepared.root.frames();
    assert_eq!(form[0] as usize, frames.len());
    let (mut at, mut numbers) = (1, Bits::<W>::EMPTY);
    for frame in frames {
        assert_eq!(form[at], frame.freedoms as u32);
        at += 1;
        assert_eq!(list(&mut at), frame.moves.to_vec());
        assert_eq!(list(&mut at), frame.factors.to_vec());
        numbers |= frame.moves | frame.factors;
    }
    assert_eq!(list(&mut at), prepared.candidates.to_vec());
    assert_eq!(form[at], prepared.max_total);
    at += 1;
    let graph = prepared.model.graph();
    for x in numbers.iter() {
        assert_eq!(form[at..at + 2], [x, prepared.model.home(x) as u32]);
        at += 2;
        let factors = graph.factors(x).iter().map(|&y| u32::from(y)).collect::<Vec<_>>();
        let composites = graph.composites(x).iter().map(|&y| u32::from(y)).collect::<Vec<_>>();
        assert_eq!((list(&mut at), list(&mut at)), (factors, composites), "the record of {x}");
    }
    at
}

/// The extended problem Π⁺(∅) poses the search problem that [`prepare`] poses for the same frames and candidates: the
/// same form and digest, with no tag of one-sided candidates, on every board to 300 (at width 8) and at larger boards
/// at their own widths.
#[test]
fn the_problem_of_the_extended_problem_with_no_open_source_is_the_plain_problem() {
    fn check<const W: usize>(n: u32) {
        let board = Board::<W>::for_n(n);
        let (ext, extended) = board.prepare_extended(false, SearchTactics::default()).unwrap();
        let plain = prepare::<W>(&board.tables, &ext.columns(), &ext.candidates, SearchTactics::default()).unwrap();
        let (problem, other) = (extended.problem(), plain.problem());
        assert_eq!(problem, other, "n={n}");
        assert_eq!(problem.digest(), other.digest(), "n={n}");
        assert_eq!(problem, problem_of::<W>(&board.tables, &ext.columns(), &ext.candidates).unwrap(), "n={n}");
        let form = form_of(&problem);
        assert_eq!(plain_part_of(&form, &extended), form.len(), "n={n}: a form with nothing after the numbers' lists");
        assert!(!form.contains(&Problem::ONE_SIDED), "n={n}: a tag of one-sided candidates");
        assert_eq!(problem.size().candidates, extended.candidates.len(), "n={n}");
    }
    (2..=300).for_each(check::<8>);
    for n in [480, 600, 760, 1000, 1133, 1300] {
        crate::with_width!(n, W => check::<W>(n));
    }
}

/// The form of a problem with one-sided candidates ends, after the lists of the numbers of the frames, with the tag
/// [`Problem::ONE_SIDED`] once, their number, and for each, ascending, the number, its own frame and the maximal
/// factors that can pay for it; the digest is that of the whole form, so it tells the problem from every one with other
/// one-sided candidates, and from the same frames and candidates without them.
#[test]
fn the_form_of_a_problem_with_one_sided_candidates_ends_with_them() {
    fn check<const W: usize>(n: u32) {
        let board = Board::<W>::for_n(n);
        let open = open_sources(&board);
        let few: Bits<W> = open.iter().take(5).collect();
        let (_, empty) = board.prepare_extended(false, SearchTactics::extended()).unwrap();
        for (what, set) in [("all of U*>=3", open), ("five sources", few)] {
            let (_, prepared) = board.prepare_extended_with(&set, false, SearchTactics::extended()).unwrap();
            let one_sided = prepared.model.one_sided();
            assert!(one_sided.len() >= set.len() as usize, "n={n} {what}");
            let (problem, form) = (prepared.problem(), form_of(&prepared.problem()));
            let at = plain_part_of(&form, &prepared);
            assert_eq!(form.iter().filter(|&&word| word == Problem::ONE_SIDED).count(), 1, "n={n} {what}: the tag");
            assert_eq!(form[at], Problem::ONE_SIDED, "n={n} {what}: the tag follows the numbers' lists");
            assert_eq!(form[at + 1] as usize, one_sided.len(), "n={n} {what}: their number");
            let mut rest = at + 2;
            for &(s, frame) in one_sided {
                let factors: Vec<u32> = prepared.model.graph().factors(s).iter().map(|&y| u32::from(y)).collect();
                assert!(!factors.is_empty(), "n={n} {what}: {s} has no factor to pay for it");
                assert_eq!(form[rest..rest + 3], [s, frame as u32, factors.len() as u32], "n={n} {what}: {s}");
                assert_eq!(form[rest + 3..rest + 3 + factors.len()], factors[..], "n={n} {what}: the factors of {s}");
                rest += 3 + factors.len();
            }
            assert_eq!(rest, form.len(), "n={n} {what}: nothing after the one-sided candidates");
            assert_eq!(problem.size().candidates, prepared.candidates.len(), "n={n} {what}: the candidates");
            assert_eq!(problem.size().frames as usize, prepared.root.len(), "n={n} {what}: the frames");

            // The digest covers the whole form, the one-sided candidates and all, and differs from Π⁺(∅)'s.
            let bytes = |words: &[u32]| -> Vec<u8> { words.iter().flat_map(|word| word.to_le_bytes()).collect() };
            assert_eq!(problem.digest(), engine::checksum(&bytes(&form)), "n={n} {what}");
            assert_ne!(problem.digest(), engine::checksum(&bytes(&form[..at])), "n={n} {what}: without the tag");
            assert_ne!(problem, empty.problem(), "n={n} {what}");
            assert_ne!(problem.digest(), empty.problem().digest(), "n={n} {what}");
            // The same problem set up again is the same.
            let (_, again) = board.prepare_extended_with(&set, false, SearchTactics::extended()).unwrap();
            assert_eq!((again.problem(), again.problem().digest()), (problem.clone(), problem.digest()));
        }
    }
    for n in [100, 200, 300] {
        check::<8>(n);
    }
    for n in [600, 1000] {
        check::<16>(n);
    }
}

// ---- benchmark: cargo test --release bench_search -- --ignored --nocapture --test-threads=1 ----

/// Prepares and searches one game with each thread count, printing one line each.
fn bench_game<const W: usize>(n: u32, columns: &[Vec<u32>], cands: &Bits<W>, java: (f64, f64)) {
    let tables = Tables::new(n);
    for (threads, java_secs) in [(1, java.0), (10, java.1)] {
        let t0 = Instant::now();
        let p = prepare::<W>(&tables, columns, cands, SearchTactics::default()).unwrap();
        let r = search(&p, &config(threads), None, None, false).unwrap();
        let total = t0.elapsed().as_secs_f64();
        let t = &p.timings;
        eprintln!(
            "n={n:4} threads={threads:2}: total {total:7.3} s (Java {java_secs:5.1} s) | greedy {:.3} search {:.3} \
             | nodes {} | sum {} | extra {:?}",
            t.greedy.as_secs_f64(),
            r.time.as_secs_f64(),
            r.stats.nodes,
            r.sum,
            r.stats.extra,
        );
    }
}

/// One game (`TAXMAN_BENCH_N`, default 700; `TAXMAN_BENCH_THREADS`, default 1), with progress lines, for
/// profiling.
#[test]
#[ignore]
fn bench_search_one() {
    let n = std::env::var("TAXMAN_BENCH_N").ok().and_then(|s| s.parse().ok()).unwrap_or(700);
    let threads = std::env::var("TAXMAN_BENCH_THREADS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let tactics = SearchTactics::default();
    let seconds = std::env::var("TAXMAN_BENCH_SECONDS").ok().and_then(|s| s.parse().ok());
    let tables = Tables::new(n);
    // The golden columns if there are any, else the frame builder's.
    let (columns, cands) = if golden_ns().contains(&n) {
        let g = load(n);
        (g.columns, Bits::<16>::from_slice(&g.candidates))
    } else {
        let frames = build_frames::<16>(&tables);
        (frames.frames.iter().flat_map(|f| [f.factors.to_vec(), f.moves.to_vec()]).collect(), frames.candidates)
    };
    let p = prepare::<16>(&tables, &columns, &cands, tactics).unwrap();
    let cfg = EngineConfig {
        progress: Some(Duration::from_secs(10)),
        time_limit: seconds.map(Duration::from_secs),
        ..config(threads)
    };
    // What optimal.json says the best promotion sum is, and the `prev_cap` ceiling.
    {
        let optimal = crate::game::Optimal::built_in();
        let in_the_bag: u64 =
            crate::frames::BoardFrames::<16>::new(&tables, Bits::EMPTY).solution().iter().map(|&x| u64::from(x)).sum();
        let best = optimal.score(n).map(|s| s - in_the_bag);
        let cap = optimal.score(n - 1).map(|s| u64::from(n) + s - in_the_bag);
        eprintln!("n={n}: optimal promotion sum {best:?}, prev_cap {cap:?}, greedy {}", p.greedy.sum());
    }
    let r = search(&p, &cfg, None, None, false).unwrap();
    eprintln!(
        "n={n}: {:?} search {:.3} s (completed {}), nodes {}, best {}, max_total {}, extra {:?}",
        p.timings,
        r.time.as_secs_f64(),
        r.completed,
        r.stats.nodes,
        r.sum,
        p.max_total,
        r.stats.extra
    );
}

#[test]
#[ignore]
fn bench_search() {
    // Java whole-game wall times, one thread and ten (including ~0.5 s of JVM start and setup).
    let java = [
        (480, 2.3, 1.5),
        (500, 2.1, 1.4),
        (600, 2.7, 1.7),
        (640, 2.0, 1.6),
        (660, 4.1, 2.1),
        (700, 18.0, 4.8),
        (720, 23.2, 5.6),
        (760, 65.6, 12.4),
    ];
    for (n, one, ten) in java {
        let g = load(n);
        if n < 512 {
            bench_game::<8>(n, &g.columns, &Bits::from_slice(&g.candidates), (one, ten));
        } else {
            bench_game::<16>(n, &g.columns, &Bits::from_slice(&g.candidates), (one, ten));
        }
    }
    if std::env::var_os("TAXMAN_BENCH_1000").is_some() {
        let frames = build_frames::<16>(&Tables::new(1000));
        let columns: Vec<Vec<u32>> =
            frames.frames.iter().flat_map(|f| [f.factors.to_vec(), f.moves.to_vec()]).collect();
        bench_game::<16>(1000, &columns, &frames.candidates, (f64::NAN, f64::NAN));
    }
}
