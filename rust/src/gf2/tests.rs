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
/// afresh from the node's state, whatever was inherited or restarted on the way; and the forced members are
/// forced: without each, a fresh solve is below what was asked.
#[test]
fn levels_match_fresh_matroids_along_random_paths() {
    const W: usize = 16;
    let mut rng = Rng(0xA5A5_0123_4567_89AB);
    let mut scratch = Scratch::<W>::new();
    let (mut checks, mut inherited, mut forced_checked) = (0, 0, 0);
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
                    let without = fresh.value_excluding(x, cap);
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
    }
    assert!(checks > 3000 && inherited > 300 && forced_checked > 1000, "{checks} {inherited} {forced_checked}");
}
