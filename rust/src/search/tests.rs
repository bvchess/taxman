use super::header::Header;
use super::refine::{strongest, Learning, Refiner, Verdict};
use super::*;
use crate::bounds::max_additional;
use crate::engine::Unit;
use crate::frames::build_frames;
use crate::json::Json;
use crate::matchbound::MatchBound;
use crate::model::test_support::*;
use crate::model::{ConflictCache, FrameGraph, Nogoods, Scratch};
use crate::numtheory::Tables;
use crate::test_support::TempDir;
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
            let max_add = max_additional(max_total, &state, &promoted, &cands);
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
fn refined_matching_bound_is_sound() {
    let mut rng = Rng(0x5EED_C0F1_1C75_0001);
    let mut checks = [0; 3];
    for n in golden_ns().into_iter().filter(|&n| (40..=220).contains(&n)) {
        let more = refinement_soundness::<4>(&load(n), &mut rng);
        for (a, b) in checks.iter_mut().zip(more) {
            *a += b;
        }
    }
    let [verdicts, refined, exact] = checks;
    eprintln!("refined matching bound: {verdicts} verdicts, {refined} refined prunings, {exact} exact completions");
    assert!(verdicts > 5000 && refined > 500 && exact > 500);
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
            let max_add = max_additional(max_total, &state, &promoted, &cands);
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
fn forced_members_are_in_every_good_completion() {
    let mut rng = Rng(0xF0BC_ED00_1234_5678);
    let mut checks = [0; 2];
    for n in golden_ns().into_iter().filter(|&n| (40..=220).contains(&n)) {
        let more = forced_soundness::<4>(&load(n), &mut rng);
        for (a, b) in checks.iter_mut().zip(more) {
            *a += b;
        }
    }
    let [nodes, forced] = checks;
    eprintln!("forced members: {nodes} nodes with a conflict to branch on, {forced} forced members");
    assert!(nodes > 500 && forced > 300);
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

/// So a test of each candidate at the root would find nothing to drop (see DESIGN.md, "Single-member conflicts
/// belong to the node"), and the single-member conflicts of the search are relative to its nodes.  The cost is
/// all in building the frames (10 to 20 ms a board above 500), so every board to 511 is checked (0.2 s) and
/// every sixth above (1.2 s); all of 2..=1230 takes 7 s.
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
fn tactics_do_not_change_answers() {
    let mut differences = Vec::new();
    for g in &searched_games(400) {
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
            let r = search_forgetting(&p, &config(threads), None, None, false, None, Some(period)).unwrap();
            let what = format!("n={}, forgetting every {period} steps on {threads} threads", g.n);
            assert!(r.completed && r.promotions.is_subset(&p.candidates), "{what}");
            assert_eq!(r.sum, golden_sum(g), "{what}");
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
/// thread and on two, until it finishes with game 1000's optimum: its searches of 760 are too short to interrupt (the
/// engine checks the node limit every 1024 nodes).
#[test]
fn checkpoint_and_resume_with_the_gf2_bound() {
    let n = 1000;
    let board = crate::board::Board::<16>::for_n(n);
    let opt = crate::game::Optimal::built_in().score(n).unwrap() - board.in_the_bag;
    let default = SearchTactics::default();
    for (tactics, node_limit) in [(default, 1000), (SearchTactics { symmetry: false, ..default }, 1500)] {
        let p = board.prepare(tactics).unwrap();
        for threads in [1, 2] {
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
    // Seeded with the optimum (plus a non-candidate), the search only has to prove it.
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

/// A start that reaches the ceiling is the incumbent as it stands: the search skips the dive and the local search,
/// which could not have changed it, and ends before it starts, with that start.
#[test]
fn a_start_at_the_ceiling_is_neither_dived_nor_improved() {
    let g = load(300);
    let p = prepare_golden::<8>(&g, SearchTactics::default());
    let optimum = search_checked(&p, 1, "plain");
    let graph = MatchGraph::new(&p.model, &p.root);
    let hood = Neighbourhood { model: &p.model, root: &p.root, candidates: &p.candidates };

    // The seed (the optimum, plus a non-candidate) reaches the ceiling, which the optimum is: the start is the better
    // of the greedy set and the seed's survivors, and the dive and the local search leave it as it is.
    let seed = optimum.promotions.with(299);
    let kept = surviving(&p, &seed);
    let begun = if kept.sum() > p.greedy.sum() { kept } else { p.greedy };
    assert_eq!(begun.sum(), optimum.sum);
    assert!(dive(&p.model, &p.root, &graph, &p.candidates).sum() <= begun.sum());
    assert_eq!(hood.improve(&mut Scratch::new(), &begun, true), begun);
    let r = search(&p, &config(1), Some(&seed), Some(optimum.sum), false).unwrap();
    assert!(r.completed && r.reached_cap && r.stats.nodes == 0 && r.found.is_none());
    assert_eq!((r.start, r.promotions, r.sum), (begun, begun, optimum.sum));

    // A ceiling below the optimum (which a game's proof never offers) that the greedy set reaches shows that the
    // local search is skipped: it would have raised the start, as it does without the ceiling.
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
    let hood = Neighbourhood { model: &p.model, root: &p.root, candidates: &p.candidates };
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

/// A ceiling given to the control of a search ends it when the incumbent reaches it, whether it is given before the
/// search starts, as `cap` is, or from another thread while the search runs.  Game 988 starts from an incumbent
/// 155 below its optimum, which the search finds at its step 4,614 of 6,287, a third of a second after it starts
/// (the other thread has that long to give the ceiling).
#[test]
fn a_ceiling_given_to_the_control_ends_the_search() {
    let n = 988;
    let board = crate::board::Board::<16>::for_n(n);
    let p = board.prepare(SearchTactics::default()).unwrap();
    let optimum = crate::game::Optimal::built_in().score(n).unwrap() - board.in_the_bag;
    let whole = search(&p, &config(1), None, None, false).unwrap();
    assert!(whole.completed && !whole.reached_cap && whole.sum == optimum);
    assert!(whole.start.sum() < optimum, "the search has nothing to do from its start");

    // Given before the search, in the control or as `cap`: the same search, which ends where it finds the optimum.
    let control = Control::new();
    control.set_ceiling(optimum);
    let cfg = EngineConfig { control: Some(Arc::clone(&control)), ..config(1) };
    let given = search(&p, &cfg, None, None, false).unwrap();
    let capped = search(&p, &config(1), None, Some(optimum), false).unwrap();
    assert!(given.completed && given.reached_cap && given.sum == optimum);
    assert_eq!((given.sum, given.stats.nodes), (capped.sum, capped.stats.nodes));
    assert!(given.stats.nodes < whole.stats.nodes, "{} steps, as many as without the ceiling", given.stats.nodes);

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

    let p = prepare_golden::<8>(&load(300), SearchTactics { match_refine: 3, ..SearchTactics::default() });
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
    // Every other format version, the older ones (`TXS2` was version 2) and any later one, is refused with one
    // message.
    let mut txs2 = b"TXS2".to_vec();
    txs2.extend_from_slice(&[0; 4 + 64 + 4 + 1 + 4 + 2]);
    for (version, bytes) in [("2", txs2), ("3", [&bytes[..4], &3u32.to_le_bytes(), &bytes[8..]].concat())]
        .into_iter()
        .chain([("5", [&bytes[..4], &5u32.to_le_bytes(), &bytes[8..]].concat()), ("unreadable", b"TXSH".to_vec())])
    {
        let err = Header::<8>::decode(&bytes).unwrap_err();
        assert!(err.contains(&format!("format version {version}, and this build reads version 4 only")), "{err}");
    }
    // Every field that differs is named.
    let other = |change: fn(&mut Header<8>)| {
        let mut h = header;
        change(&mut h);
        h.mismatch(&header).unwrap()
    };
    assert!(other(|h| h.n = 301).contains("n=301, not n=300"));
    assert!(other(|h| h.candidates ^= 1).contains("candidates"));
    assert!(other(|h| h.frames ^= 1).contains("frames"));
    assert!(other(|h| h.max_total += 1).contains("max_total"));
    assert!(other(|h| h.tactics.nogoods = !h.tactics.nogoods).contains("tactics"));

    // Every setting of the search tactics reads back as it was written: `match_refine` in bits 0 to 2, a bit for
    // each switch that is on (the defaults: 0x1B9, `symmetry` is bit 8), and no higher bit is used.
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
            };
            assert_eq!(SearchTactics::from_bits(t.bits()), Ok(t));
            let t = SearchTactics { symmetry: on, ..t };
            assert_eq!(SearchTactics::from_bits(t.bits()), Ok(t));
            assert_eq!(t.bits() >> 8, u16::from(on));
        }
    }
    assert_eq!(SearchTactics::default().bits(), 0x1B9);
    assert_eq!(SearchTactics::from_bits(0x1B9), Ok(SearchTactics::default()));
    assert_eq!(SearchTactics::from_bits(0xB9), Ok(SearchTactics { symmetry: false, ..SearchTactics::default() }));
    assert!(SearchTactics::from_bits(1 << 9).unwrap_err().contains("unknown tactics"));
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
