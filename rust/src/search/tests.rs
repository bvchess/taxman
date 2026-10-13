use super::header::Header;
use super::refine::{strongest, Learning, Refiner, Verdict};
use super::*;
use crate::bounds::{self, max_additional, Preclusions};
use crate::engine::Unit;
use crate::frames::build_frames;
use crate::json::Json;
use crate::matchbound::MatchBound;
use crate::model::test_support::*;
use crate::model::{ConflictCache, Model, Nogoods, Scratch};
use crate::numtheory::Tables;
use std::path::PathBuf;

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
    prepare::<W>(&Tables::new(g.n), &g.columns, &Bits::from_slice(&g.candidates), tactics, 2).unwrap()
}

/// Searches and checks that the result is complete, made of candidates, and feasible.
fn search_checked<const W: usize>(p: &Prepared<W>, threads: usize, what: &str) -> SearchResult<W> {
    let r = search(p, &config(threads), None, None, false).unwrap();
    assert!(r.completed, "{what}: incomplete");
    assert!(r.promotions.is_subset(&p.candidates), "{what}: {:?} are not all candidates", r.promotions);
    let feasible = p.model.promote_all_using(&mut Scratch::new(), &p.root, &r.promotions);
    assert!(feasible.is_some(), "{what}: {:?} cannot be promoted", r.promotions);
    r
}

fn check_against_java<const W: usize>(g: &Golden) {
    let p = prepare_golden::<W>(g, SearchTactics::default());
    let manager = g.json.get("searchManager").expect("searchManager");
    let frame0 = manager.get("revisedFrame0Freedoms").and_then(Json::as_i64);
    assert_eq!(p.root.frames().first().map(|f| i64::from(f.freedoms)), frame0, "n={} frame 0 freedoms", g.n);
    let max_total = manager.get("maxTotalPromotions").and_then(Json::as_i64).expect("maxTotalPromotions");
    assert_eq!(i64::from(p.max_total), max_total, "n={} max_total ({:?})", g.n, p.revise);
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
    let (model, root) = Model::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
    let all = Bits::<W>::from_slice(&g.candidates);
    let max_total = root.max_promotions_among(&all).max(0) as u32;
    let preclusions = Preclusions::build(&model, &root, &all, 2);
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
            let rest = cands.without(n);
            let max_next = max_additional(max_total, &next, &promoted.with(n), &rest);
            cands = bounds::new_candidate_list(&model, Some(&preclusions), n, max_next, &next, &rest, &promoted);
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
    let (model, root) = Model::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
    let all = Bits::<W>::from_slice(&g.candidates);
    let max_total = root.max_promotions_among(&all).max(0) as u32;
    let preclusions = Preclusions::build(&model, &root, &all, 2);
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
            let rest = cands.without(n);
            let max_next = max_additional(max_total, &next, &promoted.with(n), &rest);
            cands = bounds::new_candidate_list(&model, Some(&preclusions), n, max_next, &next, &rest, &promoted);
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
        let (model, root) = Model::build::<W>(&tables, &frames.columns()).unwrap();
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
        SearchTactics { preclusions: true, ..default },
        SearchTactics { legacy_bound: true, ..default },
        SearchTactics { revise_freedoms: false, ..default },
        SearchTactics { match_bound: false, ..default },
        SearchTactics { branching: Branching::Largest, match_refine: 0, ..default },
        SearchTactics { branching: Branching::ConflictSmallest, match_refine: 0, ..default },
        SearchTactics { branching: Branching::ConflictLargest, match_refine: 2, ..default },
        SearchTactics { branching: Branching::ConflictLargest, ..default },
        SearchTactics { branching: Branching::Largest, match_refine: 3, ..default },
        SearchTactics { fix_forced: false, ..default },
        SearchTactics { fix_forced: false, match_refine: 0, ..default },
        SearchTactics { nogoods: false, ..default },
        SearchTactics { nogoods: false, match_refine: 0, ..default },
        SearchTactics { forget_nogoods: false, ..default },
        SearchTactics { match_refine: 0, ..default },
        SearchTactics { branching: Branching::ConflictSmallest, match_refine: 2, ..default },
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
    assert!(differences.is_empty(), "AN UNPROVEN TACTIC CHANGES ANSWERS:\n{}", differences.join("\n"));
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

/// A fresh directory under the system temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("taxman-search-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Searches 760 in pieces of about a sixth of its nodes each, resuming from the checkpoint, and checks the
/// answer.  The engine checks the node limit every 1024 nodes, so a search of fewer than 6·1024 nodes stops
/// less often: with one thread it must still take `min_pieces` pieces.
fn resume_in_pieces(threads: usize, tactics: SearchTactics, min_pieces: usize) {
    let g = load(760);
    let tables = Tables::new(g.n);
    let cands = Bits::<16>::from_slice(&g.candidates);
    let whole = prepare::<16>(&tables, &g.columns, &cands, tactics, 2).unwrap();
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
        let resumed = prepare_for_resume::<16>(&tables, &g.columns, &cands, &ckpt, 2).unwrap();
        // A checkpoint's frame 0 counts as revised only if the tactic was on.
        assert_eq!(resumed.revise, if tactics.revise_freedoms { Revise::FromCheckpoint } else { Revise::Off });
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
    resume_in_pieces(1, SearchTactics::default(), 3);
    resume_in_pieces(1, SearchTactics { fix_forced: false, ..SearchTactics::default() }, 4);
    resume_in_pieces(1, SearchTactics { revise_freedoms: false, ..SearchTactics::default() }, 3);
}

#[test]
fn checkpoint_and_resume_four_threads() {
    // The default tactics search 760 in too few nodes for four threads to reach the node limit before the
    // end; the Java's branching takes many more.
    resume_in_pieces(4, SearchTactics { branching: Branching::Largest, ..SearchTactics::default() }, 2);
}

#[test]
fn a_checkpoint_of_another_game_is_refused() {
    let (a, b) = (load(760), load(720));
    let dir = TempDir::new("refuse");
    let ckpt = dir.0.join("ckpt.bin");
    let pa = prepare_golden::<16>(&a, SearchTactics::default());
    let cfg =
        EngineConfig { checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))), node_limit: Some(50), ..config(1) };
    assert!(!search(&pa, &cfg, None, None, false).unwrap().completed);
    let cands = Bits::<16>::from_slice(&b.candidates);
    assert!(prepare_for_resume::<16>(&Tables::new(b.n), &b.columns, &cands, &ckpt, 1).is_err());
    let cands = Bits::<32>::from_slice(&a.candidates);
    assert!(prepare_for_resume::<32>(&Tables::new(a.n), &a.columns, &cands, &ckpt, 1).is_err());
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
    // A cap the seed already reaches ends the search before it starts; a cap the search reaches ends it
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

#[test]
fn units_and_headers_round_trip() {
    let unit = TaxUnit::<3> { promoted: Bits::from_slice(&[1, 64, 190]), cands: Bits::from_slice(&[0, 63, 127]) };
    let mut bytes = Vec::new();
    unit.encode(&mut bytes);
    assert_eq!(bytes.len(), 48);
    assert_eq!(TaxUnit::<3>::decode(&bytes), unit);

    let p = prepare_golden::<8>(&load(300), SearchTactics { legacy_bound: true, ..SearchTactics::default() });
    let header = Header::of(&p);
    let bytes = header.encode();
    assert_eq!(bytes.len(), Header::<8>::LEN);
    let back = Header::<8>::decode(&bytes).unwrap();
    assert_eq!(back, header);
    assert_eq!(
        (back.n, back.max_total, back.frame0_freedoms, back.tactics),
        (300, p.max_total, p.root.frames().first().map(|f| f.freedoms), p.tactics)
    );
    assert_eq!(back.mismatch(&header), None);
    assert!(Header::<16>::decode(&bytes).unwrap_err().contains("Bits<8>"));
    assert!(Header::<8>::decode(b"nonsense").is_err());
    assert!(Header::<8>::decode(&bytes[..20]).is_err());
    let mut later = bytes.clone();
    later[4] = 4;
    assert!(Header::<8>::decode(&later).unwrap_err().contains("format version 4"));
    // A header of the format before version 3 is refused, saying why.
    let mut old = b"TXS2".to_vec();
    old.extend_from_slice(&[0; 4 + 64 + 4 + 1 + 4 + 2]);
    assert!(Header::<8>::decode(&old).unwrap_err().contains("older build"));
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
    assert!(other(|h| h.frame0_freedoms = None).contains("freedoms"));
    assert!(other(|h| h.tactics.nogoods = !h.tactics.nogoods).contains("tactics"));

    // Every setting of the search tactics reads back as it was written, and the default tactics of a checkpoint
    // made before `fix_forced`, `nogoods` and `forget_nogoods` existed (0x19d) read back with them on, as they
    // are now; so do those of one made before `forget_nogoods` (0x19c, the preclusions off).
    let default = SearchTactics::default();
    for refine in [0, 1, 7] {
        for branching in Branching::ALL {
            for (on, off) in [(true, false), (false, true)] {
                let t = SearchTactics {
                    preclusions: on,
                    legacy_bound: off,
                    revise_freedoms: on,
                    match_bound: off,
                    match_refine: refine,
                    branching,
                    fix_forced: on,
                    nogoods: off,
                    forget_nogoods: on,
                };
                assert_eq!(SearchTactics::from_bits(t.bits()), Ok(t));
            }
        }
    }
    assert_eq!(SearchTactics::from_bits(0x19d), Ok(SearchTactics { preclusions: true, ..default }));
    assert_eq!(SearchTactics::from_bits(0x19c), Ok(default));
    assert_eq!(SearchTactics::from_bits(0x19c | 1 << 11), Ok(SearchTactics { forget_nogoods: false, ..default }));
    assert!(SearchTactics::from_bits(1 << 12).is_err());
}

// ---- benchmark: cargo test --release bench_search -- --ignored --nocapture --test-threads=1 ----

/// Prepares and searches one game with each thread count and both settings of `match_bound` (`TAXMAN_BENCH_MATCH`
/// = `on` or `off` restricts it to one), printing one line each.
fn bench_game<const W: usize>(n: u32, columns: &[Vec<u32>], cands: &Bits<W>, java: (f64, f64)) {
    let tables = Tables::new(n);
    let only = std::env::var("TAXMAN_BENCH_MATCH").ok();
    let settings = [true, false].into_iter().filter(|&on| only.as_deref().is_none_or(|s| (s == "on") == on));
    for match_bound in settings {
        for (threads, java_secs) in [(1, java.0), (10, java.1)] {
            let tactics = SearchTactics { match_bound, ..SearchTactics::default() };
            let t0 = Instant::now();
            let p = prepare::<W>(&tables, columns, cands, tactics, threads).unwrap();
            let r = search(&p, &config(threads), None, None, false).unwrap();
            let total = t0.elapsed().as_secs_f64();
            let t = &p.timings;
            let revise_nodes = if let Revise::Searched { nodes, .. } = p.revise { nodes } else { 0 };
            eprintln!(
                "n={n:4} match={:3} threads={threads:2}: total {total:7.3} s (Java {java_secs:5.1} s) | prec {:.3} \
                 revise {:.3} ({revise_nodes} nodes) search {:.3} | nodes {} | sum {} | extra {:?}",
                if match_bound { "on" } else { "off" },
                t.preclusions.as_secs_f64(),
                t.revise.as_secs_f64(),
                r.time.as_secs_f64(),
                r.stats.nodes,
                r.sum,
                r.stats.extra,
            );
        }
    }
}

/// One game (`TAXMAN_BENCH_N`, default 700; `TAXMAN_BENCH_THREADS`, default 1; tactics changed by
/// `TAXMAN_BENCH_MATCH=off`, `TAXMAN_BENCH_NO_PRECLUSIONS`, and `TAXMAN_BENCH_LEGACY` for the
/// legacy bound), with progress lines, for profiling.
#[test]
#[ignore]
fn bench_search_one() {
    let n = std::env::var("TAXMAN_BENCH_N").ok().and_then(|s| s.parse().ok()).unwrap_or(700);
    let threads = std::env::var("TAXMAN_BENCH_THREADS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let legacy_bound = std::env::var_os("TAXMAN_BENCH_LEGACY").is_some();
    let match_bound = std::env::var("TAXMAN_BENCH_MATCH").map_or(true, |s| s == "on");
    let preclusions = std::env::var_os("TAXMAN_BENCH_NO_PRECLUSIONS").is_none();
    let tactics = SearchTactics { legacy_bound, match_bound, preclusions, ..SearchTactics::default() };
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
    let p = prepare::<16>(&tables, &columns, &cands, tactics, threads).unwrap();
    let cfg = EngineConfig {
        progress: Some(Duration::from_secs(10)),
        time_limit: seconds.map(Duration::from_secs),
        ..config(threads)
    };
    // What optimal.json says the best promotion sum is, and the `prev_cap` ceiling.
    if let Ok(optimal) = crate::game::Optimal::load_default() {
        let in_the_bag: u64 =
            crate::hives::Apiary::<16>::new(&tables, Bits::EMPTY).solution().iter().map(|&x| u64::from(x)).sum();
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
