use super::*;
use crate::board::Board;
use crate::model::Scratch;
use crate::search::{Prepared, SearchTactics};
use crate::test_support::Rng;
use std::fmt::Debug;

/// Whether the candidates `set` are elements and independent in every frame (at the root), on both of `graph`'s sides,
/// the even frames' and the odd frames': Gaussian elimination of their vectors, block by block.
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

/// On every board up to 400 with at most 22 candidates: every promotable set is independent in every frame, and
/// the bound is at least the heaviest of them.  On every board up to 400: the promotions of the optimal moves (from the
/// known moves) are independent and the bound is at least their sum.
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
        let moves = optimal.moves(n).expect("a known entry");
        let seed = board.seed(moves);
        let opt = optimal.score(n).unwrap() - board.in_the_bag;
        if seed.sum() == opt {
            assert!(independent(&graph, &seed), "n={n}: the optimal promotions are dependent");
        }
        assert!(value >= opt, "n={n}: the bound {value} is below P* {opt}");
    }
    assert!(enumerated > 80, "{enumerated} boards enumerated");
}

/// The weight of the heaviest set of at most `cap` of the elements of `graph` that is independent in every frame,
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

impl Gf2Graph {
    /// The same graph with each element `x` weighing `weight(x)` instead of its number, to test the bound with
    /// weights that the search never gives it; with the labels, because the solver wants its elements by increasing
    /// weight and the numbers it is asked about to be those elements.  An element whose weight is not positive is left
    /// out (a heaviest set independent on both sides never needs one), and the others are relabeled `1, 2, …` by
    /// increasing weight (ties by number): the graph is of those labels, and `labels[k - 1]` is the number of label
    /// `k`.  Run [`Gf2Bound`] on it with the candidates `1..=labels.len()`, and map the sets it chooses back through
    /// `labels`.
    fn reweighted(&self, weight: impl Fn(u32) -> i64) -> (Gf2Graph, Vec<u32>) {
        let mut kept: Vec<(i64, u32, usize)> = (0..self.elems.len())
            .filter_map(|e| Some((weight(self.elems[e]), self.elems[e], e)).filter(|&(w, _, _)| w > 0))
            .collect();
        kept.sort_unstable();
        let count = kept.len() as u32;
        let mut g = Gf2Graph {
            elems: (1..=count).collect(),
            index: vec![NONE; count as usize + 1],
            words: self.words,
            blocks: self.blocks,
            block: [Vec::new(), Vec::new()],
            vecs: [Vec::new(), Vec::new()],
            block_elems: self.block_elems.each_ref().map(|blocks| vec![Vec::new(); blocks.len()]),
            weight: kept.iter().map(|&(w, _, _)| w).collect(),
            ranks: self.ranks.clone(),
        };
        for (k, &(_, _, e)) in kept.iter().enumerate() {
            for m in 0..2 {
                let block = self.block[m][e];
                g.block[m].push(block);
                g.vecs[m].extend_from_slice(&self.vecs[m][e * self.words..(e + 1) * self.words]);
                g.block_elems[m][usize::from(block)].push(k as u32);
            }
            g.index[k + 1] = k as u32;
        }
        (g, kept.iter().map(|&(_, x, _)| x).collect())
    }
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

/// What [`walk_random_paths`] checked, so that the tests can see that their walks reached what they were meant to.
#[derive(Default)]
struct Walked {
    /// Bounds compared with those of a [`Gf2Graph`] built afresh from the node's state.
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
    /// Tableaux of solved levels derived from a related one (of those from a copy), each checked against the one a
    /// solve builds ([`Gf2Bound::check_derivations`]).
    derived: u64,
    derived_from_copies: u64,
    derivations_checked: u64,
    /// Members priced (`Gf2Bound::prices_excluding`) whose shortest cycle came from the parent's pricing.
    carried: u64,
    /// Children bounded by their parent's weight split (`Gf2Bound::child_bound`), each checked against the bound of
    /// a [`Gf2Graph`] built afresh from the child's state, and of those the bounds equal to it.
    child_bounds: u64,
    child_bounds_equal: u64,
    /// Children pushed at a need whose warm start stopped below it (`Gf2Bound::push_all_at`), each checked against the
    /// bound of a [`Gf2Graph`] built afresh from the child's state.
    below: u64,
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
/// stats (but for the members found forced by the searches, of which a window's split bounds take some).  The first
/// engine's bound of a node made by promoting and deleting along the path equals the bound of a [`Gf2Graph`] built
/// afresh from the node's state, whatever was inherited, certified or restarted on the way; the forced members are
/// forced: without each, a fresh solve is below what was asked; the certified bounds without a member are the fresh
/// solves', and so are the bounds without a candidate that are solved (to the end, where they must be equal, or stopped
/// early, where they are between the stop and the truth).  Half the promotions go as strong branching makes them: the
/// bounds without some members of the heaviest set certified, the node going on without one of them (its level kept,
/// certified or solved), and the child that promotes it pushed after that.  With `at_need`, half the children are
/// pushed as the search checks them, at a need near what the node's bound leaves them ([`Gf2Bound::push_all_at`]): one
/// whose warm start shows it below the need is cut, its bound no lower than that of a [`Gf2Graph`] built afresh from
/// its state, and the node goes on.  Every node's members are priced as conflict branching prices them, carrying its
/// parent's cycles, and the prices are the
/// bounds found without the carry.  Of the forced members of a node, which can be hundreds when the need is close to
/// the bound, at most `forced_cap` are checked, evenly spread over them.  Every tableau the first engine derives from a related
/// one instead of building it ([`Gf2Bound::check_derivations`]) is checked against the one a solve builds.  At each
/// node a child is looked at as the tactic `conflict_look` looks: it promotes some members of the heaviest set and
/// leaves out another, and the bound from the node's weight split ([`Gf2Bound::child_bound`]) does no other work and
/// is no lower than the child's bound.
fn walk_random_paths<const W: usize>(
    p: &Prepared<W>,
    rng: &mut Rng,
    paths: usize,
    forced_cap: usize,
    at_need: bool,
    engines: &mut [Gf2Bound<'_, W>],
    walked: &mut Walked,
) {
    let n = p.n;
    let mut scratch = Scratch::<W>::new();
    // The looks draw from their own generator, so that the paths are those a walk without them takes.
    let mut look_rng = Rng(rng.0 ^ 0x100C_A7C4_11D0_0001);
    engines[0].check_derivations();
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
            let members = (with_forced.chosen & cands).to_vec();
            let before = engines[0].stats().carried;
            let prices = agree(engines, |e| {
                let mut out = Vec::new();
                e.prices_excluding(&members, cap, &mut out);
                out
            });
            walked.carried += engines[0].stats().carried - before;
            let certified = agree(engines, |e| {
                let mut out = Vec::new();
                e.certified_excluding(&members, cap, &mut out);
                out
            });
            assert_eq!(prices, certified, "n={n}: the prices of {members:?}, promoted {promoted:?}");
            let step = (with_forced.forced.len() as usize).div_ceil(forced_cap).max(1);
            for x in with_forced.forced.iter().step_by(step) {
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
            look_at_a_child(
                p,
                &mut look_rng,
                engines,
                &mut scratch,
                (&state, &promoted, &cands),
                &with_forced.chosen,
                walked,
            );
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
                    if at_need && rng.chance(50) {
                        // As the search checks a child: pushed at a need near what the node's bound leaves it, which
                        // a warm start may show the child below before its heaviest set is known.  A child below it is
                        // cut, and the node goes on; another is checked against a fresh graph as every level is.
                        let room = p.max_total.saturating_sub(promoted.len() + 1);
                        let child_cap = room.min(next.max_promotions_among(&child_cands).max(0) as u32);
                        let need = (eval.value.saturating_sub(u64::from(x)) + 10).saturating_sub(rng.below(40));
                        let below = engines[0].stats().warm_below;
                        agree(engines, |e| e.push_all_at(&Bits::EMPTY.with(x), &child_cands, need));
                        let value = agree(engines, |e| e.evaluate(child_cap, Some(need))).value;
                        walked.below += engines[0].stats().warm_below - below;
                        if value < need {
                            let child_graph = Gf2Graph::new(&p.model, &next, &child_cands);
                            let mut fresh_child = Gf2Bound::<W>::new(&child_graph);
                            fresh_child.reset(&Bits::EMPTY, &child_cands);
                            let truth = fresh_child.evaluate(child_cap, None).value;
                            assert!(truth <= value, "n={n}: promoting {x} after {promoted:?}: {truth} above {value}");
                            agree(engines, |e| e.pop());
                            continue;
                        }
                    } else {
                        agree(engines, |e| e.push_all(&Bits::EMPTY.with(x), &child_cands));
                    }
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
    walked.derived += stats.derived;
    walked.derived_from_copies += stats.derived_from_copies;
    walked.derivations_checked += engines[0].derivations_checked();
    for (k, engine) in engines.iter().enumerate() {
        let work = Gf2Stats { forced_by_cycles: stats.forced_by_cycles, ..engine.stats() };
        assert_eq!(work, stats, "n={n}: the work of engine {k} differs from engine 0's");
    }
}

/// The look of the tactic `conflict_look` at a child of the node `(state, promoted, cands)` on top of `engines`,
/// whose heaviest set is `chosen`: the child promotes a random part of it and leaves out one other member, and its
/// candidates are narrowed as the search narrows them, with a few more dropped.  The bound from the node's weight
/// split ([`Gf2Bound::child_bound`]) does no work but its count, and is no lower than the bound of a
/// [`Gf2Graph`] built afresh from the child's state.
fn look_at_a_child<const W: usize>(
    p: &Prepared<W>,
    rng: &mut Rng,
    engines: &mut [Gf2Bound<'_, W>],
    scratch: &mut Scratch<W>,
    (state, promoted, cands): (&FrameSet<W>, &Bits<W>, &Bits<W>),
    chosen: &Bits<W>,
    walked: &mut Walked,
) {
    let members = (*chosen & *cands).to_vec();
    if members.len() < 2 {
        return;
    }
    let left_out = members[rng.below(members.len() as u64) as usize];
    let added: Bits<W> = members.iter().copied().filter(|&y| y != left_out && rng.chance(40)).collect();
    if added.is_empty() {
        return;
    }
    let Some(next) = p.model.promote_all_using(scratch, state, &added) else { return };
    let mut child_cands = next.narrow_candidates(&(*cands - added).without(left_out));
    for y in child_cands.to_vec() {
        if rng.chance(5) {
            child_cands.remove(y);
        }
    }
    let room = p.max_total.saturating_sub(promoted.len() + added.len());
    let cap = room.min(next.max_promotions_among(&child_cands).max(0) as u32);
    let before = engines[0].stats();
    let Some(bound) = agree(engines, |e| e.child_bound(&added, &child_cands)) else { return };
    let after = engines[0].stats();
    assert_eq!(
        Gf2Stats { child_bounds: before.child_bounds, ..after },
        before,
        "n={}: a child's split bound worked",
        p.n
    );
    // The child's bound from a graph built afresh from its state, which leaves the engines' workspaces as they were.
    let fresh_graph = Gf2Graph::new(&p.model, &next, &child_cands);
    let mut fresh = Gf2Bound::<W>::new(&fresh_graph);
    fresh.reset(&Bits::EMPTY, &child_cands);
    let value = fresh.evaluate(cap, None).value;
    assert!(value <= bound, "n={}: promoting {added:?} from {promoted:?}, the bound {value} is above {bound}", p.n);
    walked.child_bounds += 1;
    walked.child_bounds_equal += u64::from(value == bound);
}

/// Walks `paths` random paths of each of `boards` at W = 16 (`walk_random_paths`, with the one engine a board needs,
/// with the tactic `warm_start` if `warm`), and checks that the walks reached at least `percent` percent of what
/// [`levels_match_fresh_matroids_along_random_paths`] asks: the bound of every node equals the bound of a
/// [`Gf2Graph`] built afresh from its state, and the forced members, the certified bounds and the solved bounds without
/// a candidate are right.  With `warm`, the children found from their parent's heaviest set count as the certified ones
/// do, some must have left out more than one member of it, and some nodes must have gone on without a member of their
/// own.
fn walk_real_games(boards: &[u32], paths: usize, rng: &mut Rng, percent: u64, warm: bool) {
    const W: usize = 16;
    let mut walked = Walked::default();
    let (mut warm_children, mut warm_levels, mut warm_deletions) = (0, 0, 0);
    for &n in boards {
        let p = Board::<W>::for_n(n).prepare(SearchTactics::default()).unwrap();
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let mut engine = Gf2Bound::<W>::new(&graph);
        engine.set_warm_start(warm);
        walk_random_paths(&p, rng, paths, usize::MAX, true, std::slice::from_mut(&mut engine), &mut walked);
        warm_children += engine.stats().warm_children;
        warm_levels += engine.stats().warm_levels;
        warm_deletions += engine.stats().warm_deletions;
    }
    walked.children += warm_children;
    if warm {
        let below = walked.below;
        println!("{warm_children} children and {warm_levels} nodes warm-started, {warm_deletions} deletions");
        println!("{below} children pushed at a need cut by their warm starts");
        let more = warm_deletions > warm_children + warm_levels;
        assert!(warm_children > 0 && warm_levels > 0 && more, "{warm_children} {warm_levels} {warm_deletions}");
        assert!(below > 0, "no child pushed at a need was cut by its warm start");
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
    let Walked { derived, derived_from_copies, derivations_checked, carried, .. } = walked;
    println!("{derived} tableaux derived, {derived_from_copies} from a copy, {derivations_checked} checked");
    assert!(
        at_least(derived, 900) && at_least(derived_from_copies, 150) && derivations_checked == derived,
        "{derived} {derived_from_copies} {derivations_checked}"
    );
    println!("{carried} prices carried");
    assert!(at_least(carried, 4000), "{carried} prices carried");
    let Walked { child_bounds, child_bounds_equal, .. } = walked;
    println!("{child_bounds} children bounded by their parent's split, {child_bounds_equal} equal to the solve's");
    assert!(at_least(child_bounds, 3000) && child_bounds_equal > 0, "{child_bounds} {child_bounds_equal}");
}

/// [`levels_match_fresh_matroids_along_random_paths`] along 3 paths of the boards 60, 120, 200 and 330.
#[test]
fn levels_match_fresh_matroids_along_a_few_random_paths() {
    walk_real_games(&[60, 120, 200, 330], 3, &mut Rng(0xA5A5_0123_4567_89AC), 5, false);
}

/// [`levels_match_fresh_matroids_along_a_few_random_paths`] with the tactic `warm_start`: a child that drops members
/// of its parent's heaviest set finds its own from it (debug builds also check each such child against a solve).
#[test]
fn levels_match_fresh_matroids_along_a_few_random_paths_with_warm_starts() {
    walk_real_games(&[60, 120, 200, 330], 3, &mut Rng(0xA5A5_0123_4567_89AD), 5, true);
}

/// Along random paths of the boards from 60 to 1000 at W = 16 (`walk_random_paths`, with the one engine a board
/// needs), the bound of every node equals the bound of a [`Gf2Graph`] built afresh from its state, and the forced
/// members, the certified bounds and the solved bounds without a candidate are right.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is levels_match_fresh_matroids_along_a_few_random_paths")]
fn levels_match_fresh_matroids_along_random_paths() {
    let boards = [60u32, 90, 120, 150, 200, 260, 330, 420, 520, 640, 760, 900, 1000];
    walk_real_games(&boards, 12, &mut Rng(0xA5A5_0123_4567_89AB), 100, false);
}

/// [`levels_match_fresh_matroids_along_random_paths`] with the tactic `warm_start`.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is levels_match_fresh_matroids_along_a_few_random_paths_with_warm_starts"
)]
fn levels_match_fresh_matroids_along_random_paths_with_warm_starts() {
    let boards = [60u32, 90, 120, 150, 200, 260, 330, 420, 520, 640, 760, 900, 1000];
    walk_real_games(&boards, 12, &mut Rng(0xA5A5_0123_4567_89AE), 100, true);
}

/// Along random paths of the boards 300 and 760 at W = 16 (`walk_random_paths`, 6 paths and 2), engines whose search
/// stamps start just below their 32-bit counter's wrap, so that they wrap on the way, give every result of an engine
/// whose stamps start at 0 and do the same work: the counter's wrap changes nothing.  (Without the stamps cleared
/// before the wrap, the counter came back to 0, which every stamp never written holds, and then to the stamps of 2^32
/// stamps before: see `meet`'s test of the same.)  The engines have no warm start, so the walk pushes no child at a
/// need, which would change nothing for them.
#[test]
fn engines_agree_across_the_wrap_of_the_stamp_counter() {
    const W: usize = 16;
    let mut rng = Rng(0x57A3_9E11_0000_0002);
    let mut walked = Walked::default();
    for (n, paths) in [(300u32, 6), (760, 2)] {
        let p = Board::<W>::for_n(n).prepare(SearchTactics::default()).unwrap();
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let starts = [u32::MAX - 300, u32::MAX - 1_500, u32::MAX - 3_000];
        let mut engines: Vec<Gf2Bound<'_, W>> = (0..=starts.len()).map(|_| Gf2Bound::<W>::new(&graph)).collect();
        for (engine, &start) in engines[1..].iter_mut().zip(&starts) {
            engine.set_stamp_generation(start);
        }
        walk_random_paths(&p, &mut rng, paths, usize::MAX, false, &mut engines, &mut walked);
        for (engine, &start) in engines[1..].iter_mut().zip(&starts) {
            let end = engine.stamp_generation();
            assert!(end < start, "n={n}: the stamps from {start} did not wrap ({end})");
        }
    }
}

/// The engines of every number of block words search alike.  For boards whose frames need one or two words, the
/// natural engine and, for each larger count up to the most of the set width ([`Gf2Bound::MOST`]), the engine of that
/// many words run on the same graph ([`Gf2Bound::with_words`]) and on the graph padded with zero words
/// ([`Gf2Graph::padded`], then [`Gf2Bound::new`]) are driven along the same random paths (`walk_random_paths`): every
/// bound, heaviest set, forced set, certified bound, solved bound without a candidate and weight split must be equal,
/// and so must the work each engine did.  Only the sets wider than 64 words have the engines of four and five words,
/// and only those wider than 128 the engines of six and seven, and the boards that need them (from 4740, 6392 and
/// past 8191) are too large to walk, so this is where they run along paths, against the engines of one and two words:
/// at 128 words up to five, and at 160 up to seven.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is engines_of_every_word_count_agree_along_a_few_random_paths"
)]
fn engines_of_every_word_count_agree_along_random_paths() {
    let mut walked = Walked::default();
    engines_of_every_word_count_agree_along::<128>(&[120, 300, 760, 1000], 5, &mut walked);
    engines_of_every_word_count_agree_along::<160>(&[120, 300, 760, 1000], 5, &mut walked);
    walked_at_least(&walked, 200);
}

/// [`engines_of_every_word_count_agree_along_random_paths`] along 2 paths of the boards 120 and 300 at 160 words, the
/// width that has every engine.
#[test]
fn engines_of_every_word_count_agree_along_a_few_random_paths() {
    let mut walked = Walked::default();
    engines_of_every_word_count_agree_along::<160>(&[120, 300], 2, &mut walked);
    walked_at_least(&walked, 5);
}

/// The engines of every word count at width `W` agree along `paths` random paths of each of `boards`, which add to
/// `walked`.
fn engines_of_every_word_count_agree_along<const W: usize>(boards: &[u32], paths: usize, walked: &mut Walked) {
    let most = Gf2Bound::<W>::MOST;
    let mut rng = Rng(0x7E57_ED64_0000_0005);
    for &n in boards {
        let p = Board::<W>::for_n(n).prepare(SearchTactics::default()).unwrap();
        let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
        let natural = graph.words();
        assert!((1..=2).contains(&natural), "n={n}: {natural} words");
        let padded: Vec<Gf2Graph> = (natural + 1..=most).map(|words| graph.padded(words)).collect();
        let mut engines = vec![Gf2Bound::<W>::new(&graph)];
        for (words, padded) in (natural + 1..=most).zip(&padded) {
            assert_eq!(padded.words(), words);
            engines.push(Gf2Bound::<W>::with_words(&graph, words));
            engines.push(Gf2Bound::<W>::new(padded));
        }
        assert_eq!(engines.len(), 1 + 2 * (most - natural), "n={n}");
        walk_random_paths(&p, &mut rng, paths, usize::MAX, true, &mut engines, walked);
    }
}

/// The paths `walked` reach at least `percent` percent of what one width of
/// [`engines_of_every_word_count_agree_along_random_paths`] asked when it walked the sets of 128 words alone.
fn walked_at_least(walked: &Walked, percent: u64) {
    let &Walked { checks, excluded_checked, splits_compared, .. } = walked;
    let at_least = |count: u64, most: u64| count * 100 >= most * percent;
    assert!(
        at_least(checks, 600) && at_least(excluded_checked, 100) && at_least(splits_compared, 5000),
        "{checks} {excluded_checked} {splits_compared}"
    );
}

/// The narrow set widths (at most 64 words) have the engines of at most three words: asking one of four is refused,
/// whatever the graph, since the engines of four words and more are compiled only for the sets wider than 64 words.
#[test]
#[should_panic(expected = "not supported at a set width of 16 words")]
fn a_narrow_width_refuses_an_engine_of_four_words() {
    let p = Board::<16>::for_n(300).prepare(SearchTactics::default()).unwrap();
    let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates);
    let _ = Gf2Bound::<16>::with_words(&graph, 4);
}

/// The sets of 65 to 128 words have the engines of at most five words (ranks up to 320): one of six, which only the
/// sets wider than 128 words have, is refused.
#[test]
#[should_panic(expected = "not supported at a set width of 128 words")]
fn the_sets_of_128_words_refuse_an_engine_of_six_words() {
    let p = Board::<128>::for_n(300).prepare(SearchTactics::default()).unwrap();
    let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates).padded(WIDE_WORDS + 1);
    let _ = Gf2Bound::<128>::new(&graph);
}

/// Even the widest sets refuse an engine of more than `MAX_WORDS` words.
#[test]
#[should_panic(expected = "not supported at a set width of 160 words")]
fn no_width_has_an_engine_of_more_than_max_words() {
    let p = Board::<160>::for_n(300).prepare(SearchTactics::default()).unwrap();
    let graph = Gf2Graph::new(&p.model, &p.root, &p.candidates).padded(MAX_WORDS + 1);
    let _ = Gf2Bound::<160>::new(&graph);
}
