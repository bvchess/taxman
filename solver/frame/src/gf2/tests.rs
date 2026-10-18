use super::*;
use crate::board::Board;
use crate::frames::{build_extended, ExtendedFrames};
use crate::model::Scratch;
use crate::search::{Prepared, SearchTactics};
use crate::test_support::Rng;
use std::collections::HashSet;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Instant;

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

/// The weight of the heaviest set of at most `cap` of the elements of `graph` that is independent in both matroids,
/// by enumerating every subset of them (at most 16 elements).
fn heaviest_independent<const W: usize>(graph: &Gf2Graph, weight: impl Fn(u32) -> i64, cap: usize) -> i64 {
    let elems: Vec<u32> = graph.elems.clone();
    assert!(elems.len() <= 16);
    let mut best = 0;
    for mask in 0u32..1 << elems.len() {
        if mask.count_ones() as usize > cap {
            continue;
        }
        let set: Bits<W> = (0..elems.len()).filter(|&i| mask >> i & 1 == 1).map(|i| elems[i]).collect();
        let total: i64 = set.iter().map(&weight).sum();
        if total > best && independent(graph, &set) {
            best = total;
        }
    }
    best
}

/// The bound of `graph` for the candidates `cands`, at most `cap` promotions, and the set it chooses.
fn root_value<const W: usize>(graph: &Gf2Graph, cands: &Bits<W>, cap: u32) -> (u64, Bits<W>) {
    let mut bound = Gf2Bound::<W>::new(graph);
    bound.reset(&Bits::EMPTY, cands);
    let eval = bound.evaluate(cap, None);
    (eval.value, eval.chosen)
}

/// [`Gf2Graph::reweighted`]: with the numbers as weights it is the same bound; with any weights (some not positive,
/// none in the order of the numbers) and any limit on the size, the bound is the weight of the heaviest common
/// independent set, found by enumerating them, and the set it chooses is one, with that weight, in the labels of the
/// graph and in the numbers they stand for.
#[test]
fn a_reweighted_graph_bounds_by_its_weights() {
    const W: usize = 8;
    let mut rng = Rng(0x3C6E_F372_FE94_F82B);
    let (mut checked, mut largest) = (0, 0);
    for n in 2..=200 {
        let board = Board::<W>::for_n(n);
        if board.is_trivial() {
            continue;
        }
        let p = board.prepare(SearchTactics::default()).unwrap();
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let (plain, _) = root_value(&graph, &p.candidates, p.max_total);
        let (same, labels) = graph.reweighted(i64::from);
        let all: Bits<W> = (1..=labels.len() as u32).collect();
        assert_eq!(root_value(&same, &all, p.max_total).0, plain, "n={n}: the numbers as weights");
        if graph.len() > 16 {
            continue;
        }
        (checked, largest) = (checked + 1, largest.max(graph.len()));
        for cap in [p.max_total as usize, 1 + rng.below(4) as usize] {
            // Weights from -30 to 90: a third of them not positive.
            let weights: Vec<i64> = (0..=n).map(|_| rng.below(121) as i64 - 30).collect();
            let weight = |x: u32| weights[x as usize];
            let (re, labels) = graph.reweighted(weight);
            let positive = graph.elems.iter().filter(|&&x| weight(x) > 0).count();
            assert_eq!(re.len(), positive, "n={n}: the elements that weigh something");
            let by_weight: Vec<i64> = labels.iter().map(|&x| weight(x)).collect();
            assert!(by_weight.windows(2).all(|w| w[0] <= w[1]), "n={n}: the labels are in order of weight");
            let all: Bits<W> = (1..=labels.len() as u32).collect();
            let (value, chosen) = root_value(&re, &all, cap as u32);
            let expected = heaviest_independent::<W>(&graph, weight, cap);
            assert_eq!(value as i64, expected, "n={n}, cap {cap}: the weights {weights:?}");
            let numbers: Bits<W> = chosen.iter().map(|k| labels[k as usize - 1]).collect();
            assert!(independent(&graph, &numbers) && numbers.len() as usize <= cap, "n={n}: the chosen set");
            assert_eq!(numbers.iter().map(weight).sum::<i64>(), expected, "n={n}: the chosen set's weight");
        }
    }
    assert!(checked > 40 && largest >= 12, "{checked} boards checked, the largest of {largest} elements");
}

/// The extended problem Π⁺(`open`) of `board` as the search is given it, built by hand since its setup comes in a later
/// step: the frames with the one-sided candidates, the model and root they make, the candidates (two-sided and
/// one-sided) and the most promotions a promotion set can have.  The extended frames come with it, for their
/// `in_the_bag`.
///
/// # Errors
///
/// If a member of `open` cannot be open on the board, or the problem has more frames than a frame set holds.
fn extended_problem<const W: usize>(
    board: &Board<W>,
    open: &Bits<W>,
) -> Result<(ExtendedFrames<W>, Prepared<W>), String> {
    let ext = build_extended::<W>(&board.tables, open, &[])?;
    let (model, root) = FrameGraph::build_extended::<W>(&board.tables, &ext.columns(), &ext.one_sided)?;
    let one_sided: Bits<W> = ext.one_sided.iter().map(|&(s, _)| s).collect();
    let candidates = ext.candidates | one_sided;
    let max_total = model.max_promotions_among(&root, &candidates).max(0) as u32;
    let prepared = Prepared {
        n: board.n(),
        model: Arc::new(model),
        root,
        candidates,
        tactics: SearchTactics::default(),
        greedy: Bits::EMPTY,
        max_total,
        timings: Default::default(),
    };
    Ok((ext, prepared))
}

/// U\*_{≥3} of `board`, the sources that can be open: the numbers above n/2 of the game with no promotions that have
/// three or more prime factors, ascending.
fn upper_from_3<const W: usize>(board: &Board<W>) -> Vec<u32> {
    let mut upper: Vec<u32> =
        board.plain.iter().copied().filter(|&x| 2 * x > board.n() && board.tables.omega(x) >= 3).collect();
    upper.sort_unstable();
    upper
}

/// What the extended problems of [`bound_and_independence_in_the_extended_problem`] met, so that the test can see that
/// it reached what it was meant to.
#[derive(Default)]
struct ExtendedTally {
    /// Problems with candidates, and those that have a one-sided candidate.
    problems: usize,
    with_one_sided: usize,
    /// Problems whose promotion sets were enumerated, those of them that hit the limit on the sets, the sets
    /// enumerated, and those that hold a one-sided candidate.
    enumerated: usize,
    capped: usize,
    sets: usize,
    sets_with_one_sided: usize,
    /// Problems on boards that the table of optimal games knows, whose bound was compared with its optimum.
    optimum_checks: usize,
}

/// On every board from 2 to 120, for the open sets U\*_{≥3} and three random subsets of it, the bound of the extended
/// problem Π⁺(O) at the root: every promotion set found by enumeration (at most 2²⁰ of them, for problems of at most
/// 22 candidates) is independent in both matroids, one-sided members included, and the bound is at least the heaviest
/// of them; and since the optimal game of these boards plays U\*_{≥3} (the table of optimal games has its score), the
/// bound is at least that score less what the fixed sources bring.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is bound_and_independence_in_the_extended_problem_of_the_small_boards"
)]
fn bound_and_independence_in_the_extended_problem() {
    bound_and_independence_in_extended_problems(2..=120, 100);
}

/// [`bound_and_independence_in_the_extended_problem`] on the boards 2 to 46.
#[test]
fn bound_and_independence_in_the_extended_problem_of_the_small_boards() {
    bound_and_independence_in_extended_problems(2..=46, 2);
}

/// The checks of [`bound_and_independence_in_the_extended_problem`] on the boards `ns`, which must reach `percent`
/// percent of what it asks.
fn bound_and_independence_in_extended_problems(ns: std::ops::RangeInclusive<u32>, percent: usize) {
    const W: usize = 8;
    let optimal = crate::game::Optimal::built_in();
    let mut rng = Rng(0xE87E_ADED_0000_0001);
    let (mut tally, start) = (ExtendedTally::default(), Instant::now());
    for n in ns {
        let board = Board::<W>::for_n(n);
        let all = upper_from_3(&board);
        let mut opens: Vec<Bits<W>> = vec![all.iter().copied().collect()];
        for _ in 0..3 {
            opens.push(all.iter().copied().filter(|_| rng.chance(50)).collect());
        }
        for (k, open) in opens.iter().enumerate() {
            if opens[..k].contains(open) {
                continue;
            }
            let (ext, p) = extended_problem(&board, open).unwrap_or_else(|e| panic!("n={n}: O={open:?}: {e}"));
            if p.candidates.is_empty() {
                continue;
            }
            let one_sided: Bits<W> = ext.one_sided.iter().map(|&(s, _)| s).collect();
            let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
            let mut bound = Gf2Bound::<W>::new(&graph);
            bound.reset(&Bits::EMPTY, &p.candidates);
            let value = bound.evaluate(p.max_total, None).value;
            tally.problems += 1;
            tally.with_one_sided += usize::from(!one_sided.is_empty());
            if p.candidates.len() <= 22 {
                const LIMIT: usize = 1 << 20;
                let sets = promotable_sets(&p.model, &p.root, &p.candidates, LIMIT);
                tally.capped += usize::from(sets.len() >= LIMIT);
                for set in &sets {
                    assert!(independent(&graph, set), "n={n}, O={open:?}: the promotion set {set:?} is dependent");
                }
                let best = sets.iter().map(Bits::sum).max().unwrap_or(0);
                assert!(
                    value >= best,
                    "n={n}, O={open:?}: the bound {value} is below the heaviest promotion set {best}"
                );
                tally.enumerated += 1;
                tally.sets += sets.len();
                tally.sets_with_one_sided += sets.iter().filter(|set| set.intersects(&one_sided)).count();
            }
            if let Some(score) = optimal.score(n) {
                let opt = score.checked_sub(ext.in_the_bag).expect("the fixed sources score less than the game");
                assert!(value >= opt, "n={n}, O={open:?}: the bound {value} is below the optimum {opt}");
                tally.optimum_checks += 1;
            }
        }
    }
    let ExtendedTally { problems, with_one_sided, enumerated, capped, sets, sets_with_one_sided, optimum_checks } =
        tally;
    println!(
        "{problems} problems, {with_one_sided} with a one-sided candidate, {optimum_checks} against the optimum, \
         in {:.1?}",
        start.elapsed()
    );
    println!(
        "{enumerated} enumerated ({capped} stopped at the limit): {sets} promotion sets, {sets_with_one_sided} with a \
         one-sided member"
    );
    let at_least = |count: usize, most: usize| count * 100 >= most * percent;
    assert!(
        at_least(problems, 300) && at_least(with_one_sided, 300) && at_least(optimum_checks, 300),
        "{problems} {with_one_sided} {optimum_checks}"
    );
    assert!(
        at_least(enumerated, 150) && at_least(sets, 1_000_000) && at_least(sets_with_one_sided, 1_000_000),
        "{enumerated} {sets} {sets_with_one_sided}"
    );
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
    /// In the extended problem: the bounds compared at nodes whose candidates hold a one-sided candidate, the forced
    /// members that are one-sided, and the promotions of one-sided candidates along the paths.
    checks_with_one_sided: u64,
    forced_one_sided: u64,
    one_sided_promoted: u64,
    /// Tableaux of solved levels derived from a related one (of those from a copy), each checked against the one a
    /// solve builds ([`Gf2Bound::check_derivations`]).
    derived: u64,
    derived_from_copies: u64,
    derivations_checked: u64,
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
/// one of them (its level kept, certified or solved), and the child that promotes it pushed after that.  The
/// narrowing of a child's candidates and the count of the promotions left are the model's, so that `p` may be an
/// extended problem with one-sided candidates (for a problem without them they are the frame set's own).  Of the
/// forced members of a node, which can be hundreds when the need is close to the bound, at most `forced_cap` are
/// checked, evenly spread over them.  Every tableau the first engine derives from a related one instead of building it
/// ([`Gf2Bound::check_derivations`]) is checked against the one a solve builds.
fn walk_random_paths<const W: usize>(
    p: &Prepared<W>,
    rng: &mut Rng,
    paths: usize,
    forced_cap: usize,
    engines: &mut [Gf2Bound<'_, W>],
    walked: &mut Walked,
) {
    let n = p.n;
    let one_sided: Bits<W> = p.model.one_sided().iter().map(|&(s, _)| s).collect();
    let mut scratch = Scratch::<W>::new();
    engines[0].check_derivations();
    for _ in 0..paths {
        agree(engines, |e| e.reset(&Bits::EMPTY, &p.candidates));
        // The path: (state, promoted, candidates) per level.
        let mut path = vec![(p.root, Bits::<W>::EMPTY, p.candidates)];
        for _ in 0..40 {
            let (state, promoted, cands) = *path.last().unwrap();
            let cap = (p.max_total - promoted.len()).min(p.model.max_promotions_among(&state, &cands).max(0) as u32);
            let eval = agree(engines, |e| e.evaluate(cap, None));
            let fresh_graph = Gf2Graph::new(&p.model, &state, &cands);
            let mut fresh = Gf2Bound::<W>::new(&fresh_graph);
            fresh.reset(&Bits::EMPTY, &cands);
            let truth = fresh.evaluate(cap, None).value;
            assert_eq!(eval.value, truth, "n={n}: promoted {promoted:?}, candidates {cands:?}");
            walked.checks += 1;
            walked.checks_with_one_sided += u64::from(cands.intersects(&one_sided));
            // Forced members at a need just below the bound.
            let need = eval.value.saturating_sub(rng.below(40));
            let with_forced = agree(engines, |e| e.evaluate(cap, Some(need)));
            let step = (with_forced.forced.len() as usize).div_ceil(forced_cap).max(1);
            for x in with_forced.forced.iter().step_by(step) {
                let without = fresh.value_excluding(x, cap, u64::MAX);
                assert!(without < need, "n={n}: {x} is not forced: {without} without it, need {need}");
                walked.forced_checked += 1;
                walked.forced_one_sided += u64::from(one_sided.contains(x));
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
                    let mut child_cands = p.model.narrow(&next, &cands.without(x));
                    // Drop a few more, as the search's rule-outs do.
                    for y in child_cands.to_vec() {
                        if rng.chance(5) {
                            child_cands.remove(y);
                        }
                    }
                    let before = engines[0].stats().inherited;
                    agree(engines, |e| e.push_all(&Bits::EMPTY.with(x), &child_cands));
                    walked.inherited += engines[0].stats().inherited - before;
                    walked.one_sided_promoted += u64::from(one_sided.contains(x));
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
    walked.derived += stats.derived;
    walked.derived_from_copies += stats.derived_from_copies;
    walked.derivations_checked += engines[0].derivations_checked();
    for (k, engine) in engines.iter().enumerate() {
        assert_eq!(engine.stats(), stats, "n={n}: the work of engine {k} differs from engine 0's");
    }
}

/// Walks `paths` random paths of each of `boards` at W = 16 (`walk_random_paths`, with the one engine a board needs),
/// and checks that the walks reached at least `percent` percent of what [`levels_match_fresh_matroids_along_random_paths`]
/// asks: the bound of every node equals the bound of the matroids built afresh from its state, and the forced members,
/// the certified bounds and the solved bounds without a candidate are right.
fn walk_real_games(boards: &[u32], paths: usize, rng: &mut Rng, percent: u64) {
    const W: usize = 16;
    let mut walked = Walked::default();
    for &n in boards {
        let p = Board::<W>::for_n(n).prepare(SearchTactics::default()).unwrap();
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        walk_random_paths(&p, rng, paths, usize::MAX, &mut [Gf2Bound::<W>::new(&graph)], &mut walked);
    }
    let Walked { checks, inherited, forced_checked, certified_checked, excluded_checked, children, .. } = walked;
    println!("{checks} {inherited} {forced_checked} {certified_checked} {children} {excluded_checked}");
    let at_least = |count: u64, most: u64| count * 100 >= most * percent;
    assert!(
        at_least(checks, 3000)
            && at_least(inherited, 300)
            && at_least(forced_checked, 1000)
            && at_least(certified_checked, 300)
            && at_least(children, 100)
            && at_least(excluded_checked, 300),
        "{checks} {inherited} {forced_checked} {certified_checked} {children} {excluded_checked}"
    );
    let Walked { derived, derived_from_copies, derivations_checked, .. } = walked;
    println!("{derived} tableaux derived, {derived_from_copies} from a copy, {derivations_checked} checked");
    assert!(
        at_least(derived, 900) && at_least(derived_from_copies, 150) && derivations_checked == derived,
        "{derived} {derived_from_copies} {derivations_checked}"
    );
}

/// [`levels_match_fresh_matroids_along_random_paths`] along 3 paths of the boards 60, 120, 200 and 330.
#[test]
fn levels_match_fresh_matroids_along_a_few_random_paths() {
    walk_real_games(&[60, 120, 200, 330], 3, &mut Rng(0xA5A5_0123_4567_89AC), 5);
}

/// Along random paths of the boards from 60 to 1000 at W = 16 (`walk_random_paths`, with the one engine a board
/// needs), the bound of every node equals the bound of the matroids built afresh from its state, and the forced
/// members, the certified bounds and the solved bounds without a candidate are right.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is levels_match_fresh_matroids_along_a_few_random_paths")]
fn levels_match_fresh_matroids_along_random_paths() {
    let boards = [60u32, 90, 120, 150, 200, 260, 330, 420, 520, 640, 760, 900, 1000];
    walk_real_games(&boards, 12, &mut Rng(0xA5A5_0123_4567_89AB), 100);
}

/// Along random paths of the boards 300 and 760 at W = 16 (`walk_random_paths`), engines whose search stamps start
/// just below their 32-bit counter's wrap, so that they wrap on the way, give every result of an engine whose stamps
/// start at 0 and do the same work: the counter's wrap changes nothing.  (Without the stamps cleared before the wrap,
/// the counter came back to 0, which every stamp never written holds, and then to the stamps of 2^32 stamps before:
/// see `meet`'s test of the same.)
#[test]
fn engines_agree_across_the_wrap_of_the_stamp_counter() {
    const W: usize = 16;
    let mut rng = Rng(0x57A3_9E11_0000_0002);
    let mut walked = Walked::default();
    for n in [300u32, 760] {
        let p = Board::<W>::for_n(n).prepare(SearchTactics::default()).unwrap();
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let starts = [u32::MAX - 300, u32::MAX - 1_500, u32::MAX - 3_000];
        let mut engines: Vec<Gf2Bound<'_, W>> = (0..=starts.len()).map(|_| Gf2Bound::<W>::new(&graph)).collect();
        for (engine, &start) in engines[1..].iter_mut().zip(&starts) {
            engine.set_stamp_generation(start);
        }
        walk_random_paths(&p, &mut rng, 6, usize::MAX, &mut engines, &mut walked);
        for (engine, &start) in engines[1..].iter_mut().zip(&starts) {
            let end = engine.stamp_generation();
            assert!(end < start, "n={n}: the stamps from {start} did not wrap ({end})");
        }
    }
}

/// The most forced members per node that the walks of the extended problem check: with hundreds of one-sided
/// candidates, most members of the heaviest set are forced when the need is close to the bound, and a fresh solve for
/// each would take most of the walk's time.
const FORCED_CAP: usize = 12;

/// Walks `paths` random paths (`walk_random_paths`) of the extended problem Π⁺(O) of each of `boards` with the open
/// sources U\*_{≥3} and with a random half of them (a board whose Π⁺(U\*_{≥3}) has more frames than a frame set holds
/// is walked with the half only), with the one engine each needs.
fn walk_extended_problems<const W: usize>(boards: &[u32], paths: usize, rng: &mut Rng, walked: &mut Walked) {
    let mut walks = 0;
    for &n in boards {
        let board = Board::<W>::for_n(n);
        let all = upper_from_3(&board);
        let half: Vec<u32> = all.iter().copied().filter(|_| rng.chance(50)).collect();
        for open in [all, half] {
            let Ok((_, p)) = extended_problem(&board, &open.into_iter().collect()) else { continue };
            let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
            walk_random_paths(&p, rng, paths, FORCED_CAP, &mut [Gf2Bound::<W>::new(&graph)], walked);
            walks += 1;
        }
    }
    assert!(walks >= boards.len(), "{walks} problems walked on {} boards", boards.len());
}

/// What the walks of the extended problem reached, and the assertions that they reached enough of it (`percent` percent
/// of what the default walk of [`levels_match_fresh_matroids_along_random_paths_in_the_extended_problem`] must): nodes
/// checked, children that inherited, forced members, certified and solved bounds without a member, and, of the
/// one-sided candidates, the nodes that had one among their candidates, the forced members and the promotions.
fn assert_walks_reached(walked: &Walked, percent: u64) {
    let Walked {
        checks,
        inherited,
        forced_checked,
        certified_checked,
        excluded_checked,
        children,
        checks_with_one_sided,
        forced_one_sided,
        one_sided_promoted,
        ..
    } = *walked;
    println!(
        "{checks} nodes checked, {checks_with_one_sided} with a one-sided candidate; {inherited} inherited,          {forced_checked} forced ({forced_one_sided} one-sided), {certified_checked} certified,          {excluded_checked} solved, {children} certified children; {one_sided_promoted} one-sided promotions"
    );
    let at_least = |count: u64, most: u64| count * 100 >= most * percent;
    assert!(
        at_least(checks, 4000)
            && at_least(inherited, 300)
            && at_least(forced_checked, 30_000)
            && at_least(certified_checked, 2500)
            && at_least(children, 20)
            && at_least(excluded_checked, 1200),
        "{checks} {inherited} {forced_checked} {certified_checked} {children} {excluded_checked}"
    );
    assert!(
        at_least(checks_with_one_sided, 4000)
            && at_least(forced_one_sided, 20_000)
            && at_least(one_sided_promoted, 1000),
        "{checks_with_one_sided} {forced_one_sided} {one_sided_promoted}"
    );
    let Walked { derived, derived_from_copies, derivations_checked, .. } = *walked;
    println!("{derived} tableaux derived, {derived_from_copies} from a copy, {derivations_checked} checked");
    assert!(at_least(derived, 500) && derivations_checked == derived, "{derived} {derivations_checked}");
}

/// [`levels_match_fresh_matroids_along_random_paths_in_the_extended_problem`] along 3 paths of the boards 60, 120 and
/// 200 at W = 16.
#[test]
fn levels_match_fresh_matroids_along_a_few_random_paths_in_the_extended_problem() {
    let (mut rng, mut walked) = (Rng(0x0E87_5EED_C0FF_EE03), Walked::default());
    walk_extended_problems::<16>(&[60, 120, 200], 3, &mut rng, &mut walked);
    assert_walks_reached(&walked, 3);
}

/// Along random paths of the extended problem Π⁺(O) on the boards from 60 to 1000 at W = 16, with O = U\*_{≥3} and with
/// a random half of it (`walk_random_paths`), the bound of every node, with one-sided candidates promoted, deleted and
/// forced along the way, equals the bound of the matroids built afresh from its state, and the forced members, the
/// certified bounds and the solved bounds without a candidate are right.  Also on the board 1300 at W = 128, where the
/// frames of Π⁺(U\*_{≥3}) are too wide for the engines of three words that the narrower sets have, and need that of
/// four.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is levels_match_fresh_matroids_along_a_few_random_paths_in_the_extended_problem"
)]
fn levels_match_fresh_matroids_along_random_paths_in_the_extended_problem() {
    let mut rng = Rng(0x0E87_5EED_C0FF_EE01);
    let (mut walked, start) = (Walked::default(), Instant::now());
    let boards = [60, 90, 120, 160, 200, 260, 300, 360, 420, 520, 600, 760, 1000];
    walk_extended_problems::<16>(&boards, 12, &mut rng, &mut walked);
    walk_extended_problems::<128>(&[1300], 4, &mut rng, &mut walked);
    println!("walked in {:.1?}", start.elapsed());
    assert_walks_reached(&walked, 100);
}

/// The same walks, longer: more paths on more boards, with the boards 1300 and 1500 at W = 128 as well.
#[test]
#[ignore = "walks the extended problem on 16 boards, 40 or 10 paths a problem, about 50 s in release"]
fn levels_match_fresh_matroids_along_many_random_paths_in_the_extended_problem() {
    let mut rng = Rng(0x0E87_5EED_C0FF_EE02);
    let (mut walked, start) = (Walked::default(), Instant::now());
    let boards = [45, 60, 75, 90, 105, 120, 150, 200, 260, 330, 420, 600, 800, 1000];
    walk_extended_problems::<16>(&boards, 40, &mut rng, &mut walked);
    walk_extended_problems::<128>(&[1300, 1500], 10, &mut rng, &mut walked);
    println!("walked in {:.1?}", start.elapsed());
    assert_walks_reached(&walked, 300);
}

/// The free blocks of a graph of the extended problem are as described (`Gf2Graph`, "One-sided candidates"), on a
/// board with many one-sided candidates: the blocks of each matroid are its frames' blocks and `ceil(coloops /
/// (64·words))` free blocks (the coloops being the one-sided candidates whose own frame is in the other matroid); each
/// one-sided element is, in the matroid of the other parity, in a free block with a unit vector that no other element
/// shares, and in the matroid of its own frame in block `own / 2`, a frame's block with a vector that is not zero; and
/// no free block holds another element or is empty.  Returns the number of free blocks in each matroid.
fn check_free_blocks<const W: usize>(n: u32) -> [usize; 2] {
    let board = Board::<W>::for_n(n);
    let open: Bits<W> = upper_from_3(&board).into_iter().collect();
    let (_, p) = extended_problem(&board, &open).unwrap_or_else(|e| panic!("n={n}: {e}"));
    let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
    let frames = p.root.frames().len();
    let (words, per_block) = (graph.words, 64 * graph.words);
    let frame_blocks = [frames.div_ceil(2), frames / 2];
    assert_eq!(graph.ranks.len(), frames, "n={n}: a rank for each frame");

    // The one-sided candidates that are elements, by the matroid they are coloops of.
    let mut coloops: [Vec<usize>; 2] = Default::default();
    for &(s, j) in p.model.one_sided() {
        assert_eq!(p.model.own(s), Some(j), "n={n}: the own frame of {s}");
        if let Some(e) = graph.element(s) {
            coloops[1 - j % 2].push(e as usize);
        }
    }
    let total = coloops[0].len() + coloops[1].len();
    println!(
        "n={n}: {} one-sided candidates, {total} of them elements, coloops {}+{}, {frames} frames, {words} words, \
         blocks {:?}, ranks {:?}",
        p.model.one_sided().len(),
        coloops[0].len(),
        coloops[1].len(),
        graph.blocks,
        graph.ranks()
    );
    assert!(total > 100, "n={n}: only {total} one-sided elements");
    for m in 0..2 {
        let free = coloops[m].len().div_ceil(per_block);
        assert_eq!(graph.blocks[m], frame_blocks[m] + free, "n={n}, matroid {m}: the number of blocks");
        assert_eq!(graph.block_elems[m].len(), graph.blocks[m], "n={n}, matroid {m}: the lists of the blocks");

        // The coordinates of the free blocks: one unit vector for each coloop, each used once.
        let mut used = HashSet::new();
        for &e in &coloops[m] {
            let block = usize::from(graph.block[m][e]);
            assert!(block >= frame_blocks[m], "n={n}: the coloop {} is in the frame block {block}", graph.elems[e]);
            let vector = &graph.vecs[m][e * words..(e + 1) * words];
            let bits: u32 = vector.iter().map(|w| w.count_ones()).sum();
            assert_eq!(bits, 1, "n={n}: the coloop {} has a vector of {bits} set bits", graph.elems[e]);
            let coordinate = vector.iter().position(|&w| w != 0).map(|i| 64 * i + vector[i].trailing_zeros() as usize);
            assert!(
                used.insert((block, coordinate)),
                "n={n}: the coordinate {coordinate:?} of block {block} is shared"
            );
        }
        // The free blocks hold coloops only, at most `per_block` of them, and none is empty.
        for block in frame_blocks[m]..graph.blocks[m] {
            let members = &graph.block_elems[m][block];
            assert!(
                !members.is_empty() && members.len() <= per_block,
                "n={n}: free block {block} holds {}",
                members.len()
            );
            assert!(
                members.iter().all(|e| coloops[m].contains(&(*e as usize))),
                "n={n}: free block {block}: not coloops"
            );
        }
        let in_free_blocks: usize = (frame_blocks[m]..graph.blocks[m]).map(|b| graph.block_elems[m][b].len()).sum();
        assert_eq!(in_free_blocks, coloops[m].len(), "n={n}, matroid {m}: coloops in free blocks");
        // The frames' blocks hold no coloop.
        for block in 0..frame_blocks[m] {
            assert!(
                graph.block_elems[m][block].iter().all(|e| !coloops[m].contains(&(*e as usize))),
                "n={n}: a coloop in the frame block {block} of matroid {m}"
            );
        }
    }

    // Each one-sided element's block in the matroid of its own frame is the frame's, with a vector that is not zero.
    for &(s, j) in p.model.one_sided() {
        let Some(e) = graph.element(s) else { continue };
        let e = e as usize;
        assert_eq!(
            usize::from(graph.block[j % 2][e]),
            j / 2,
            "n={n}: the block of {s} in the matroid of its frame {j}"
        );
        assert!(
            graph.vecs[j % 2][e * words..(e + 1) * words].iter().any(|&w| w != 0),
            "n={n}: {s} is a loop of its frame"
        );
    }
    // Every list of a block holds exactly the elements that say they are in it.
    for m in 0..2 {
        for e in 0..graph.len() {
            let block = usize::from(graph.block[m][e]);
            assert!(
                graph.block_elems[m][block].contains(&(e as u32)),
                "n={n}: element {e} is not in its block {block}"
            );
        }
        let listed: usize = graph.block_elems[m].iter().map(Vec::len).sum();
        assert_eq!(listed, graph.len(), "n={n}, matroid {m}: elements listed in blocks");
    }
    [0, 1].map(|m| graph.blocks[m] - frame_blocks[m])
}

/// The free blocks of the graph of Π⁺(U\*_{≥3}) on the board 1000, where each matroid has at least one, and on the
/// board 2000, where there are more coloops than a block holds in one of them and so more than one free block (see
/// [`check_free_blocks`]).
#[test]
fn free_blocks_hold_the_coloops_of_the_one_sided_candidates() {
    let free = check_free_blocks::<16>(1000);
    assert!(free.iter().all(|&blocks| blocks >= 1), "free blocks on the board 1000: {free:?}");
    let free = check_free_blocks::<32>(2000);
    assert!(free.iter().any(|&blocks| blocks >= 2), "free blocks on the board 2000: {free:?}");
}

/// The engines of every number of block words search alike.  For boards whose frames need one or two words, the
/// natural engine and, for each larger count up to `MAX_WORDS`, the engine of that many words run on the same graph
/// ([`Gf2Bound::with_words`]) and on the graph padded with zero words ([`Gf2Graph::padded`], then [`Gf2Bound::new`])
/// are driven along the same random paths (`walk_random_paths`): every bound, heaviest set, forced set, certified
/// bound, solved bound without a candidate and weight split must be equal, and so must the work each engine did.
/// Only the sets of 128 words have the engines of four and five words, and the boards that need them (from 4740 and
/// 6392) are too large to walk, so this is where they run along paths, against the engines of one and two words.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is engines_of_every_word_count_agree_along_a_few_random_paths"
)]
fn engines_of_every_word_count_agree_along_random_paths() {
    engines_of_every_word_count_agree_along(&[120, 300, 760, 1000], 5, 100);
}

/// [`engines_of_every_word_count_agree_along_random_paths`] along 2 paths of the boards 120 and 300.
#[test]
fn engines_of_every_word_count_agree_along_a_few_random_paths() {
    engines_of_every_word_count_agree_along(&[120, 300], 2, 5);
}

/// The engines of every word count agree along `paths` random paths of each of `boards`, which reach at least
/// `percent` percent of what [`engines_of_every_word_count_agree_along_random_paths`] asks.
fn engines_of_every_word_count_agree_along(boards: &[u32], paths: usize, percent: u64) {
    const W: usize = 128;
    let mut rng = Rng(0x7E57_ED64_0000_0005);
    let mut walked = Walked::default();
    for &n in boards {
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
        walk_random_paths(&p, &mut rng, paths, usize::MAX, &mut engines, &mut walked);
    }
    let Walked { checks, excluded_checked, splits_compared, .. } = walked;
    let at_least = |count: u64, most: u64| count * 100 >= most * percent;
    assert!(
        at_least(checks, 600) && at_least(excluded_checked, 100) && at_least(splits_compared, 5000),
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
