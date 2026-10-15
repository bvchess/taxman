use super::*;
use crate::board::Board;
use crate::model::Scratch;
use crate::search::{Prepared, SearchTactics};
use crate::test_support::Rng;
use std::fmt::Debug;

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
fn promotable_sets<const W: usize>(
    model: &FrameGraph,
    state: &FrameSet<W>,
    cands: &Bits<W>,
    limit: usize,
) -> Vec<Bits<W>> {
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

/// What [`walk_random_paths`] checked, so that the tests can see that their walks reached what they were meant to.
#[derive(Default)]
struct Walked {
    /// Bounds compared with those of matroids built afresh from the node's state.
    checks: u64,
    /// Children that inherited their parent's heaviest set outright.
    inherited: u64,
    /// Forced members, certified bounds without a member and solved ones, each checked against a fresh solve.
    forced_checked: u64,
    certified_checked: u64,
    excluded_checked: u64,
    /// Children whose heaviest set their parent's exchange graph certified.
    children: u64,
    /// Weight-split bounds without a candidate (of those a level had one for) compared across the engines.
    splits_compared: u64,
}

/// Runs `f` on every engine, checks that all give the same result and returns it.
fn agree<const W: usize, T: PartialEq + Debug>(
    engines: &mut [Gf2Bound<'_, W>],
    mut f: impl FnMut(&mut Gf2Bound<'_, W>) -> T,
) -> T {
    let (first, rest) = engines.split_first_mut().expect("an engine");
    let result = f(first);
    for (k, engine) in rest.iter_mut().enumerate() {
        assert_eq!(f(engine), result, "engine {} differs from engine 0", k + 1);
    }
    result
}

/// Walks `paths` random paths of the search of `p`, applying every operation to each of `engines` (all of them the
/// bound of `p`'s game), and checks that every result is the first engine's and that the engines end with the same
/// stats.  The first engine's bound of a node made by promoting and deleting along the path equals the bound of the
/// matroids built afresh from the node's state, whatever was inherited, certified or restarted on the way; the forced
/// members are forced: without each, a fresh solve is below what was asked; the certified bounds without a member
/// are the fresh solves', and so are the bounds without a candidate that are solved (to the end, where they must be
/// equal, or stopped early, where they are between the stop and the truth).  Half the promotions go as strong
/// branching makes them: the bounds without some members of the heaviest set certified, the node going on without
/// one of them (its level kept, certified or solved), and the child that promotes it pushed after that.
fn walk_random_paths<const W: usize>(
    p: &Prepared<W>,
    rng: &mut Rng,
    paths: usize,
    engines: &mut [Gf2Bound<'_, W>],
    walked: &mut Walked,
) {
    let n = p.n;
    let mut scratch = Scratch::<W>::new();
    for _ in 0..paths {
        agree(engines, |e| e.reset(&Bits::EMPTY, &p.candidates));
        // The path: (state, promoted, candidates) per level.
        let mut path = vec![(p.root, Bits::<W>::EMPTY, p.candidates)];
        for _ in 0..40 {
            let (state, promoted, cands) = *path.last().unwrap();
            let cap = (p.max_total - promoted.len()).min(state.max_promotions_among(&cands).max(0) as u32);
            let eval = agree(engines, |e| e.evaluate(cap, None));
            let fresh_graph = Gf2Graph::new(&p.model, &state, &cands);
            let mut fresh = Gf2Bound::<W>::new(&fresh_graph);
            fresh.reset(&Bits::EMPTY, &cands);
            let truth = fresh.evaluate(cap, None).value;
            assert_eq!(eval.value, truth, "n={n}: promoted {promoted:?}, candidates {cands:?}");
            walked.checks += 1;
            // Forced members at a need just below the bound.
            let need = eval.value.saturating_sub(rng.below(40));
            let with_forced = agree(engines, |e| e.evaluate(cap, Some(need)));
            for x in with_forced.forced.iter() {
                let without = fresh.value_excluding(x, cap, u64::MAX);
                assert!(without < need, "n={n}: {x} is not forced: {without} without it, need {need}");
                walked.forced_checked += 1;
            }
            // The weight split's bound without each candidate: for the members of the heaviest set, if the level
            // has a split.
            for x in cands.iter() {
                let split = agree(engines, |e| e.split_without(x));
                walked.splits_compared += u64::from(split.is_some());
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
                        agree(engines, |e| e.remove_candidate(x));
                        path.last_mut().unwrap().2.remove(x);
                        continue;
                    };
                    // As strong branching does: the bounds without a few members certified (checked against fresh
                    // solves), the node going on without x, from its certified level if it has one, else from the
                    // level solved without x.
                    let strong = rng.chance(50) && eval.chosen.contains(x);
                    if strong {
                        let tried: Vec<u32> = std::iter::once(x)
                            .chain((eval.chosen & cands).iter().filter(|&y| y != x && rng.chance(30)))
                            .collect();
                        let certified = agree(engines, |e| {
                            let mut out = Vec::new();
                            e.certified_excluding(&tried, cap, &mut out);
                            out
                        });
                        for (&y, value) in tried.iter().zip(&certified) {
                            if let Some(value) = *value {
                                assert_eq!(value, fresh.value_excluding(y, cap, u64::MAX), "n={n}: without {y}");
                                walked.certified_checked += 1;
                            }
                        }
                        if certified[0].is_some() {
                            agree(engines, |e| e.keep_certified(x));
                        } else {
                            let value = agree(engines, |e| e.value_excluding(x, cap, u64::MAX));
                            assert_eq!(value, fresh.value_excluding(x, cap, u64::MAX), "n={n}: without {x}, solved");
                            agree(engines, |e| e.keep_excluded());
                            walked.excluded_checked += 1;
                        }
                        agree(engines, |e| e.remove_candidate(x));
                        path.last_mut().unwrap().2.remove(x);
                    }
                    let mut child_cands = next.narrow_candidates(&cands.without(x));
                    // Drop a few more, as the search's rule-outs do.
                    for y in child_cands.to_vec() {
                        if rng.chance(5) {
                            child_cands.remove(y);
                        }
                    }
                    let before = engines[0].stats().inherited;
                    agree(engines, |e| e.push_all(&Bits::EMPTY.with(x), &child_cands));
                    walked.inherited += engines[0].stats().inherited - before;
                    path.push((next, promoted.with(x), child_cands));
                }
                // Delete a candidate, often one of the heaviest set; half the time after its bound without it is
                // solved, to the end or stopped early, and kept for the node.
                5..=7 => {
                    let pool = if rng.chance(70) { eval.chosen & cands } else { cands };
                    let members = pool.to_vec();
                    if members.is_empty() {
                        break;
                    }
                    let x = members[rng.below(members.len() as u64) as usize];
                    if rng.chance(50) {
                        let stop = if rng.chance(50) { u64::MAX } else { eval.value.saturating_sub(rng.below(30)) };
                        let truth = fresh.value_excluding(x, cap, u64::MAX);
                        let value = agree(engines, |e| e.value_excluding(x, cap, stop));
                        if value < stop {
                            assert_eq!(value, truth, "n={n}: without {x}");
                        } else {
                            assert!(
                                (stop..=truth).contains(&value),
                                "n={n}: without {x}: {value}, stop {stop}, {truth}"
                            );
                        }
                        agree(engines, |e| e.keep_excluded());
                        walked.excluded_checked += 1;
                    }
                    agree(engines, |e| e.remove_candidate(x));
                    path.last_mut().unwrap().2.remove(x);
                }
                // Back to the parent.
                _ => {
                    if path.len() > 1 {
                        path.pop();
                        agree(engines, |e| e.pop());
                    }
                }
            }
        }
    }
    walked.children += engines[0].stats().certified_children;
    let stats = engines[0].stats();
    for (k, engine) in engines.iter().enumerate() {
        assert_eq!(engine.stats(), stats, "n={n}: the work of engine {k} differs from engine 0's");
    }
}

/// Along random paths of the boards from 60 to 1000 at W = 16 (`walk_random_paths`, with the one engine a board
/// needs), the bound of every node equals the bound of the matroids built afresh from its state, and the forced
/// members, the certified bounds and the solved bounds without a candidate are right.
#[test]
fn levels_match_fresh_matroids_along_random_paths() {
    const W: usize = 16;
    let mut rng = Rng(0xA5A5_0123_4567_89AB);
    let mut walked = Walked::default();
    for &n in &[60u32, 90, 120, 150, 200, 260, 330, 420, 520, 640, 760, 900, 1000] {
        let p = Board::<W>::for_n(n).prepare(SearchTactics::default()).unwrap();
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        walk_random_paths(&p, &mut rng, 12, &mut [Gf2Bound::<W>::new(&graph)], &mut walked);
    }
    let Walked { checks, inherited, forced_checked, certified_checked, excluded_checked, children, .. } = walked;
    assert!(
        checks > 3000
            && inherited > 300
            && forced_checked > 1000
            && certified_checked > 300
            && children > 100
            && excluded_checked > 300,
        "{checks} {inherited} {forced_checked} {certified_checked} {children} {excluded_checked}"
    );
}

/// The engines of every number of block words search alike.  For boards whose frames need one or two words, the
/// natural engine and, for each larger count up to `MAX_WORDS`, the engine of that many words run on the same graph
/// ([`Gf2Bound::with_words`]) and on the graph padded with zero words ([`Gf2Graph::padded`], then [`Gf2Bound::new`])
/// are driven along the same random paths (`walk_random_paths`): every bound, heaviest set, forced set, certified
/// bound, solved bound without a candidate and weight split must be equal, and so must the work each engine did.
/// Only the sets of 128 words have the engines of four and five words, and the boards that need them (from 4740 and
/// 6392) are too large to walk, so this is where they run along paths, against the engines of one and two words.
#[test]
fn engines_of_every_word_count_agree_along_random_paths() {
    const W: usize = 128;
    let mut rng = Rng(0x7E57_ED64_0000_0005);
    let mut walked = Walked::default();
    for n in [120u32, 300, 760, 1000] {
        let p = Board::<W>::for_n(n).prepare(SearchTactics::default()).unwrap();
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let natural = graph.words();
        assert!((1..=2).contains(&natural), "n={n}: {natural} words");
        let padded: Vec<Gf2Graph> = (natural + 1..=MAX_WORDS).map(|words| graph.padded(words)).collect();
        let mut engines = vec![Gf2Bound::<W>::new(&graph)];
        for (words, padded) in (natural + 1..=MAX_WORDS).zip(&padded) {
            assert_eq!(padded.words(), words);
            engines.push(Gf2Bound::<W>::with_words(&graph, words));
            engines.push(Gf2Bound::<W>::new(padded));
        }
        assert_eq!(engines.len(), 1 + 2 * (MAX_WORDS - natural), "n={n}");
        walk_random_paths(&p, &mut rng, 5, &mut engines, &mut walked);
    }
    let Walked { checks, excluded_checked, splits_compared, .. } = walked;
    assert!(
        checks > 600 && excluded_checked > 100 && splits_compared > 5000,
        "{checks} {excluded_checked} {splits_compared}"
    );
}

/// The narrow set widths (at most 64 words) have the engines of at most three words: asking one of four is refused,
/// whatever the graph, since the engines of four and five words are compiled only for the sets wider than 64 words.
#[test]
#[should_panic(expected = "not supported at a set width of 16 words")]
fn a_narrow_width_refuses_an_engine_of_four_words() {
    let p = Board::<16>::for_n(300).prepare(SearchTactics::default()).unwrap();
    let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
    let _ = Gf2Bound::<16>::with_words(&graph, 4);
}

/// Even the widest sets refuse an engine of more than `MAX_WORDS` words (ranks above 320).
#[test]
#[should_panic(expected = "not supported at a set width of 128 words")]
fn no_width_has_an_engine_of_six_words() {
    let p = Board::<128>::for_n(300).prepare(SearchTactics::default()).unwrap();
    let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates).padded(MAX_WORDS + 1);
    let _ = Gf2Bound::<128>::new(&graph);
}
