use super::*;
use crate::board::Board;
use crate::model::Scratch;
use crate::search::SearchTactics;
use crate::test_support::Rng;

/// Whether the candidates `set` are elements and independent in both of `graph`'s matroids (at the root): Gaussian
/// elimination of their vectors, block by block.
fn independent<const W: usize>(graph: &Gf2Graph, set: &Bits<W>) -> bool {
    let words = graph.words;
    (0..2).all(|m| {
        let mut rows: Vec<(usize, Vec<u64>)> = Vec::new(); // (block, row)
        set.iter().all(|x| {
            let Some(e) = graph.element(x) else { return false };
            let e = e as usize;
            let block = usize::from(graph.block[m][e]);
            let mut v = graph.vecs[m][e * words..(e + 1) * words].to_vec();
            for (b, row) in rows.iter().filter(|(b, _)| *b == block) {
                let _ = b;
                let p =
                    row.iter().enumerate().find(|(_, &w)| w != 0).map(|(i, w)| 64 * i + w.trailing_zeros() as usize);
                let p = p.expect("rows are nonzero");
                if v[p / 64] >> (p % 64) & 1 == 1 {
                    for (a, r) in v.iter_mut().zip(row) {
                        *a ^= r;
                    }
                }
            }
            if v.iter().all(|&w| w == 0) {
                return false;
            }
            // Keep the rows reduced against each other by pivot: reduce the old rows by the new one.
            let p = v.iter().enumerate().find(|(_, &w)| w != 0).map(|(i, w)| 64 * i + w.trailing_zeros() as usize);
            let p = p.expect("nonzero");
            for (b, row) in rows.iter_mut().filter(|(b, _)| *b == block) {
                let _ = b;
                if row[p / 64] >> (p % 64) & 1 == 1 {
                    for (r, a) in row.iter_mut().zip(&v) {
                        *r ^= a;
                    }
                }
            }
            rows.push((block, v));
            true
        })
    })
}

/// Every promotable subset of `cands` from `state`, by depth-first enumeration (at most `limit` of them).
fn promotable_sets<const W: usize>(model: &Model, state: &FrameSet<W>, cands: &Bits<W>, limit: usize) -> Vec<Bits<W>> {
    let mut scratch = Scratch::new();
    let cands = cands.to_vec();
    let mut out = Vec::new();
    let mut stack = vec![(*state, Bits::<W>::EMPTY, 0usize)];
    while let Some((at, set, from)) = stack.pop() {
        out.push(set);
        if out.len() >= limit {
            break;
        }
        for (i, &x) in cands.iter().enumerate().skip(from) {
            if let Some(next) = model.promote_all_using(&mut scratch, &at, &Bits::EMPTY.with(x)) {
                stack.push((next, set.with(x), i + 1));
            }
        }
    }
    out
}

/// On every board up to 140 with at most 22 candidates: every promotable set is independent in both matroids, and
/// the bound is at least the heaviest of them.  On every board up to 400: the optimal game's promotions (from the
/// known games) are independent and the bound is at least their sum.
#[test]
fn bound_and_independence_on_small_boards() {
    const W: usize = 8;
    let optimal = crate::game::Optimal::built_in();
    let mut enumerated = 0;
    for n in 2..=400 {
        let board = Board::<W>::for_n(n);
        if board.is_trivial() {
            continue;
        }
        let p = board.prepare(SearchTactics::default()).unwrap();
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let mut bound = Gf2Bound::<W>::new(&graph);
        bound.reset(&Bits::EMPTY, &p.candidates);
        let value = bound.evaluate(p.max_total, None).value;
        if p.candidates.len() <= 22 {
            let sets = promotable_sets(&p.model, &p.root, &p.candidates, 1 << 22);
            for set in &sets {
                assert!(independent(&graph, set), "n={n}: the promotable set {set:?} is dependent");
            }
            let best = sets.iter().map(Bits::sum).max().unwrap_or(0);
            assert!(value >= best, "n={n}: the bound {value} is below the heaviest promotable set {best}");
            enumerated += 1;
        }
        let moves = optimal.moves(n).expect("a known game");
        let seed = board.seed(moves);
        let opt = optimal.score(n).unwrap() - board.in_the_bag;
        if seed.sum() == opt {
            assert!(independent(&graph, &seed), "n={n}: the optimal promotions are dependent");
        }
        assert!(value >= opt, "n={n}: the bound {value} is below the optimum {opt}");
    }
    assert!(enumerated > 80, "{enumerated} boards enumerated");
}

/// The bound of a node made by promoting and deleting along random paths equals the bound of the matroids built
/// afresh from the node's state, whatever was inherited, certified or restarted on the way; the forced members are
/// forced: without each, a fresh solve is below what was asked; and the certified bounds without a member are the
/// fresh solves'.  Half the promotions go as strong branching makes them: the bounds without some members of the
/// heaviest set certified, the node going on without one of them (its certified level kept), and the child that
/// promotes it pushed after that.
#[test]
fn levels_match_fresh_matroids_along_random_paths() {
    const W: usize = 16;
    let mut rng = Rng(0xA5A5_0123_4567_89AB);
    let mut scratch = Scratch::<W>::new();
    let (mut checks, mut inherited, mut forced_checked, mut certified_checked, mut children) = (0, 0, 0, 0, 0);
    let mut certified = Vec::new();
    for &n in &[60u32, 90, 120, 150, 200, 260, 330, 420, 520, 640, 760, 900, 1000] {
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default()).unwrap();
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let mut bound = Gf2Bound::<W>::new(&graph);
        for _ in 0..12 {
            bound.reset(&Bits::EMPTY, &p.candidates);
            // The path: (state, promoted, candidates) per level.
            let mut path = vec![(p.root, Bits::<W>::EMPTY, p.candidates)];
            for _ in 0..40 {
                let (state, promoted, cands) = *path.last().unwrap();
                let cap = (p.max_total - promoted.len()).min(state.max_promotions_among(&cands).max(0) as u32);
                let eval = bound.evaluate(cap, None);
                let fresh_graph = Gf2Graph::new(&p.model, &state, &cands);
                let mut fresh = Gf2Bound::<W>::new(&fresh_graph);
                fresh.reset(&Bits::EMPTY, &cands);
                let truth = fresh.evaluate(cap, None).value;
                assert_eq!(eval.value, truth, "n={n}: promoted {promoted:?}, candidates {cands:?}");
                checks += 1;
                // Forced members at a need just below the bound.
                let need = eval.value.saturating_sub(rng.below(40));
                let with_forced = bound.evaluate(cap, Some(need));
                for x in with_forced.forced.iter() {
                    let without = fresh.value_excluding(x, cap, u64::MAX);
                    assert!(without < need, "n={n}: {x} is not forced: {without} without it, need {need}");
                    forced_checked += 1;
                }
                match rng.below(10) {
                    // Promote a member of the heaviest set (the child inherits it) or any candidate.
                    0..=4 => {
                        let pool = if rng.chance(60) { eval.chosen & cands } else { cands };
                        let members = pool.to_vec();
                        if members.is_empty() {
                            break;
                        }
                        let x = members[rng.below(members.len() as u64) as usize];
                        let Some(next) = p.model.promote_all_using(&mut scratch, &state, &Bits::EMPTY.with(x)) else {
                            bound.remove_candidate(x);
                            path.last_mut().unwrap().2.remove(x);
                            continue;
                        };
                        // As strong branching does: the bounds without a few members certified (checked against
                        // fresh solves), the node going on without x, from its certified level if it has one.
                        let strong = rng.chance(50) && eval.chosen.contains(x);
                        if strong {
                            let tried: Vec<u32> = std::iter::once(x)
                                .chain((eval.chosen & cands).iter().filter(|&y| y != x && rng.chance(30)))
                                .collect();
                            bound.certified_excluding(&tried, cap, &mut certified);
                            for (&y, value) in tried.iter().zip(&certified) {
                                if let Some(value) = *value {
                                    assert_eq!(value, fresh.value_excluding(y, cap, u64::MAX), "n={n}: without {y}");
                                    certified_checked += 1;
                                }
                            }
                            if certified[0].is_some() {
                                bound.keep_certified(x);
                            }
                            bound.remove_candidate(x);
                            path.last_mut().unwrap().2.remove(x);
                        }
                        let mut child_cands = next.narrow_candidates(&cands.without(x));
                        // Drop a few more, as the search's rule-outs do.
                        for y in child_cands.to_vec() {
                            if rng.chance(5) {
                                child_cands.remove(y);
                            }
                        }
                        let before = bound.stats().inherited;
                        bound.push_all(&Bits::EMPTY.with(x), &child_cands);
                        inherited += bound.stats().inherited - before;
                        path.push((next, promoted.with(x), child_cands));
                    }
                    // Delete a candidate, often one of the heaviest set.
                    5..=7 => {
                        let pool = if rng.chance(70) { eval.chosen & cands } else { cands };
                        let members = pool.to_vec();
                        if members.is_empty() {
                            break;
                        }
                        let x = members[rng.below(members.len() as u64) as usize];
                        bound.remove_candidate(x);
                        path.last_mut().unwrap().2.remove(x);
                    }
                    // Back to the parent.
                    _ => {
                        if path.len() > 1 {
                            path.pop();
                            bound.pop();
                        }
                    }
                }
            }
        }
        children += bound.stats().certified_children;
    }
    assert!(
        checks > 3000 && inherited > 300 && forced_checked > 1000 && certified_checked > 300 && children > 100,
        "{checks} {inherited} {forced_checked} {certified_checked} {children}"
    );
}

/// How many solves the main workspace of `bound` has made.
fn main_solves<const W: usize>(bound: &Gf2Bound<W>) -> u64 {
    match &bound.0 {
        Engine::R1(l) => l.meet.solves,
        Engine::R2(l) => l.meet.solves,
        Engine::R3(l) => l.meet.solves,
    }
}

/// The heaviest common independent set of the elements of `graph` (a graph built from a node, so that its
/// elements are the node's live candidates) of positive weight, each element `x` weighing `weight(x)`, with at most
/// `cap` members: [`Meet::solve`] on the graph's vectors in a workspace of its own, the elements listed by (weight,
/// reverse element), which is not the order of the solve under test among elements of equal weight.
fn heaviest_weighted<const R: usize>(graph: &Gf2Graph, weight: &dyn Fn(u32) -> i64, cap: usize) -> i64 {
    let n = graph.len();
    let vector = [0, 1].map(|m| {
        (0..n)
            .map(|i| {
                let mut v = [0u64; R];
                v[..graph.words].copy_from_slice(&graph.vecs[m][i * graph.words..(i + 1) * graph.words]);
                v
            })
            .collect::<Vec<_>>()
    });
    let weights: Vec<i64> = graph.elems.iter().map(|&x| weight(x)).collect();
    let mut live: Vec<u32> = (0..n as u32).filter(|&e| weights[e as usize] > 0).collect();
    live.sort_unstable_by_key(|&e| (weights[e as usize], std::cmp::Reverse(e)));
    let problem =
        Problem { block: [&graph.block[0], &graph.block[1]], vector: [&vector[0], &vector[1]], weight: &weights };
    let mut meet = Meet::<R>::new(n, graph.blocks);
    meet.solve(&problem, &live, &[], cap, i64::MAX, |_| {});
    meet.value
}

/// [`Gf2Bound::value_weighted`] is the heaviest common independent set of the node's live candidates of positive
/// weight, for weights in no relation to the numbers' order (some of zero or less, many equal), at every node of
/// random paths down boards 60 to 400 (promotions, deletions, pops; the node's matroids built afresh and solved in
/// a workspace of their own for the truth), under the node's limit and a smaller one, with and without a stop; and
/// it leaves the main workspace alone (no solve made in it, the level's solve and the stats of the other solves as
/// they were), and the top level's evaluation as it was.
#[test]
fn weighted_values_match_fresh_solves_along_random_paths() {
    const W: usize = 8;
    let mut rng = Rng(0x7E16_4ED5_0C0F_FEE1);
    let mut scratch = Scratch::<W>::new();
    let (mut nodes, mut stopped, mut dropped) = (0, 0, 0);
    for n in (60u32..=400).step_by(7) {
        let board = Board::<W>::for_n(n);
        let p = board.prepare(SearchTactics::default()).unwrap();
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let mut bound = Gf2Bound::<W>::new(&graph);
        for _ in 0..4 {
            bound.reset(&Bits::EMPTY, &p.candidates);
            let mut path = vec![(p.root, Bits::<W>::EMPTY, p.candidates)];
            for _ in 0..25 {
                let (state, promoted, cands) = *path.last().unwrap();
                let cap = (p.max_total - promoted.len()).min(state.max_promotions_among(&cands).max(0) as u32);
                let eval = bound.evaluate(cap, None);
                let fresh = Gf2Graph::new(&p.model, &state, &cands);
                // Weights by number: small, so that many are equal, some below 1, and not in the order of the numbers.
                let by_number: Vec<i64> = (0..=n).map(|_| rng.below(40) as i64 - 8).collect();
                let weight = |x: u32| by_number[x as usize];
                for limit in [cap, rng.below(u64::from(cap) + 1) as u32] {
                    let truth = match fresh.words {
                        1 => heaviest_weighted::<1>(&fresh, &weight, limit as usize),
                        2 => heaviest_weighted::<2>(&fresh, &weight, limit as usize),
                        _ => heaviest_weighted::<3>(&fresh, &weight, limit as usize),
                    };
                    let (solves, stats) = (main_solves(&bound), bound.stats());
                    let at = format!("n={n}: promoted {promoted:?}, candidates {cands:?}, limit {limit}");
                    assert_eq!(bound.value_weighted(weight, limit, i64::MAX), truth, "{at}");
                    // A stop: the value reached it if the truth does, else it is the truth.
                    let stop = 1 + rng.below(truth as u64 + 10) as i64;
                    let value = bound.value_weighted(weight, limit, stop);
                    assert!(if truth >= stop { value >= stop && value <= truth } else { value == truth }, "{at}");
                    stopped += usize::from(truth >= stop);
                    // Nothing else of the bound moved.
                    let after = bound.stats();
                    assert_eq!(main_solves(&bound), solves, "{at}: the main workspace made a solve");
                    assert_eq!(
                        (after.solves, after.augmentations, after.excluded_solves, after.rounds),
                        (stats.solves, stats.augmentations, stats.excluded_solves, stats.rounds),
                        "{at}"
                    );
                    assert_eq!(after.weighted_solves, stats.weighted_solves + 2, "{at}");
                    assert_eq!(bound.evaluate(cap, None), eval, "{at}: the evaluation changed");
                    dropped += fresh.elems.iter().filter(|&&x| weight(x) <= 0).count();
                    nodes += 1;
                }
                match rng.below(10) {
                    // Promote a candidate, preferably a member of the heaviest set.
                    0..=5 => {
                        let pool = if rng.chance(60) { eval.chosen & cands } else { cands };
                        let members = pool.to_vec();
                        if members.is_empty() {
                            break;
                        }
                        let x = members[rng.below(members.len() as u64) as usize];
                        let Some(next) = p.model.promote_all_using(&mut scratch, &state, &Bits::EMPTY.with(x)) else {
                            bound.remove_candidate(x);
                            path.last_mut().unwrap().2.remove(x);
                            continue;
                        };
                        let child_cands = next.narrow_candidates(&cands.without(x));
                        bound.push_all(&Bits::EMPTY.with(x), &child_cands);
                        path.push((next, promoted.with(x), child_cands));
                    }
                    // Delete a candidate.
                    6..=7 => {
                        let members = cands.to_vec();
                        if members.is_empty() {
                            break;
                        }
                        let x = members[rng.below(members.len() as u64) as usize];
                        bound.remove_candidate(x);
                        path.last_mut().unwrap().2.remove(x);
                    }
                    // Back to the parent.
                    _ => {
                        if path.len() > 1 {
                            path.pop();
                            bound.pop();
                        }
                    }
                }
            }
        }
    }
    assert!(nodes > 6000 && stopped > 4000 && dropped > 30_000, "{nodes} checks, {stopped} stops, {dropped} left out");
}
