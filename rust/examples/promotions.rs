//! A3, split by the number of promotions: how many nodes of a proof could be closed by bounding them with the
//! heaviest common independent set of *exactly* the size their completion needs, instead of the heaviest of all
//! sizes.  The idea and the partition argument are in docs/AGENDA.md, A3; the numbers are in docs/EXPERIMENTS.md,
//! "Split by the number of promotions: A3's test (2026-10)".
//!
//! ```text
//! cargo run --release --example promotions -- [--known FILE]... [--fixtures DIR] N...
//! ```
//!
//! For each game `N` it reads the node fixture `nNNNN.nodes` of `DIR` (`testdata/perf` by default; `perf make-nodes`
//! makes them) and prints one JSON object per node and one per game.  `P*` is the optimum's promotion sum (the known
//! score less `in_the_bag`, from the built-in table and the results files of `--known`; it must be the fixture's
//! incumbent).
//!
//! * The *window* is the set of total sizes `k <= max_total` that can still win: those whose root `best(k)`, the
//!   weight of the heaviest common independent set of the two GF(2) frame matroids of size `k`, exceeds `P*`.  A
//!   promotion set of size `k` has a sum of at most `best(k)` (it is independent in both matroids), so a set that
//!   beats `P*` has a size in the window: the sizes are a partition, and the argument needs no conjecture.
//! * A node is *open* when its plain bound (the heaviest common independent set of at most `max_add` candidates of
//!   the node's contracted matroids) exceeds `P*` less the sum of its promotions.  The total size of its heaviest
//!   completion, `promoted + size`, lies *below*, *inside* or *above* the window.  Inside, the exact-size bound is
//!   the plain bound, so the node stays open; `best` is concave, so only a node outside the window can be closed
//!   (a heaviest completion that ties in size with a larger set is counted by its smaller size here).  The command
//!   counts the nodes of each kind, and stops there: the exact-size bound itself is in the tests.
//!
//! `best(k)` for every `k` comes from the same intersection run on the weights plus a uniform amount `mu` above
//! their sum, so that every augmentation gains: it then passes through the heaviest set of each size (the weights by
//! size are concave) up to the largest common independent set, and `best(k)` is its weight less `mu * k`
//! ([`best_by_size`]; the tests check it against the enumeration of all common independent sets, and the command
//! against the plain bound and its set's size at every node).

#[allow(dead_code)]
#[path = "common/fixtures.rs"]
mod fixtures;
#[allow(dead_code)]
#[path = "common/frames.rs"]
mod frames;
#[allow(dead_code)]
#[path = "common/meet.rs"]
mod meet;

use frames::{frame_matroids, setup, FrameMatroids, Known};
use std::path::PathBuf;
use taxman::model::Scratch;

// ---------------------------------------------------------------------------------------------------------
// The weights by size and the window

/// The weight of the heaviest common independent set of each size `0, 1, ...` of `fm`, up to `limit` or the largest
/// common independent set.  Concave, as asserted.
pub fn best_by_size(fm: &FrameMatroids, limit: usize) -> Vec<i64> {
    let w: Vec<i64> = fm.elems.iter().map(|&x| i64::from(x)).collect();
    // Every augmentation gains (a set of the next size weighs at least 0 and the last one at most the sum of all
    // the weights), so the run goes on to the largest common independent set.
    let mu = w.iter().sum::<i64>() + 1;
    let shifted: Vec<i64> = w.iter().map(|&x| x + mu).collect();
    let run = fm.solve(&shifted, limit);
    // The fast algorithm starts from the `prefix` heaviest elements, a common independent set, so that its sets of
    // fewer elements are the heaviest of them and `by_size` starts at the size `prefix`.
    let mut heaviest = w;
    heaviest.sort_unstable_by(|a, b| b.cmp(a));
    let mut best: Vec<i64> = heaviest
        .iter()
        .take(run.prefix)
        .scan(0, |sum, &x| {
            *sum += x;
            Some(*sum - x)
        })
        .collect();
    best.extend(run.by_size.iter().enumerate().map(|(i, &v)| v - mu * (run.prefix + i) as i64));
    assert!(best.windows(3).all(|t| t[1] - t[0] >= t[2] - t[1]), "the weights by size are not concave: {best:?}");
    best
}

/// The window of winning sizes: the sizes `k <= max_total` with `best[k] > p_star`, as `(lowest, highest)`, or `None`
/// if there are none.  By concavity they are consecutive.
pub fn window(best: &[i64], max_total: usize, p_star: i64) -> Option<(usize, usize)> {
    let sizes: Vec<usize> = (0..best.len().min(max_total + 1)).filter(|&k| best[k] > p_star).collect();
    let (&lo, &hi) = (sizes.first()?, sizes.last()?);
    assert_eq!(sizes.len(), hi - lo + 1, "the winning sizes are not consecutive");
    Some((lo, hi))
}

/// How the total size of a node's heaviest completion lies against the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    Below,
    Inside,
    Above,
}

impl Place {
    fn of(size: usize, window: Option<(usize, usize)>) -> Place {
        match window {
            Some((lo, _)) if size < lo => Place::Below,
            Some((_, hi)) if size > hi => Place::Above,
            Some(_) => Place::Inside,
            // No size can win: the root is closed, and so is every node, as if above the window.
            None => Place::Above,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Place::Below => "below",
            Place::Inside => "inside",
            Place::Above => "above",
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// The command

struct Options {
    known: Vec<PathBuf>,
    fixtures: PathBuf,
    games: Vec<u32>,
}

fn parse() -> Result<Options, String> {
    let mut o = Options {
        known: Vec::new(),
        fixtures: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/perf"),
        games: Vec::new(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--known" => o.known.push(PathBuf::from(args.next().ok_or("--known needs a value")?)),
            "--fixtures" => o.fixtures = PathBuf::from(args.next().ok_or("--fixtures needs a value")?),
            _ => o.games.push(arg.parse().map_err(|e| format!("{arg}: {e}"))?),
        }
    }
    if o.games.is_empty() {
        return Err("no games (see the top of examples/promotions.rs)".into());
    }
    Ok(o)
}

fn main() {
    let o = parse().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(2);
    });
    let known = Known::load(&o.known, None).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(2);
    });
    for &n in &o.games {
        taxman::with_width!(n, W => game::<W>(n, &o, &known));
    }
}

/// The totals of a game's nodes.
#[derive(Default)]
struct Counts {
    nodes: usize,
    with_candidates: usize,
    /// Nodes of all, open or not, whose heaviest completion lies outside the window.
    outside_any: usize,
    open: usize,
    below: usize,
    inside: usize,
    above: usize,
}

fn game<const W: usize>(n: u32, o: &Options, known: &Known) {
    let path = o.fixtures.join(format!("n{n:04}.nodes"));
    let (n_file, incumbent, units) = fixtures::read_nodes::<W>(&path).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(n_file, n, "{}: the fixture is of another game", path.display());
    let (board, p) = setup::<W>(n).expect("a game with candidates");
    let model = &p.model;
    let p_star = known.opt(n, board.in_the_bag).unwrap_or_else(|| panic!("n={n}: no known game (--known FILE?)"));
    assert_eq!(p_star, incumbent.sum(), "n={n}: the fixture's incumbent is not the known optimum");
    let p_star = p_star as i64;
    let max_total = p.max_total as usize;

    let root_fm = frame_matroids(model, &p.root, &p.candidates);
    let root = best_by_size(&root_fm, max_total);
    let root_bound = *root.iter().max().expect("the empty set");
    let root_size = root.iter().position(|&v| v == root_bound).expect("the heaviest set");
    // The largest common independent set at all, past `max_total`.
    let root_rank = best_by_size(&root_fm, usize::MAX).len() - 1;
    let win = window(&root, max_total, p_star);
    let margins = win.map_or(String::new(), |(lo, hi)| {
        (lo.saturating_sub(2)..(hi + 3).min(root.len()))
            .map(|k| format!("[{k},{}]", root[k] - p_star))
            .collect::<Vec<_>>()
            .join(",")
    });

    let mut scratch = Scratch::<W>::new();
    let mut c = Counts { nodes: units.len(), ..Counts::default() };
    for (i, u) in units.iter().enumerate().filter(|(_, u)| !u.cands.is_empty()) {
        c.with_candidates += 1;
        let state = model.promote_all_using(&mut scratch, &p.root, &u.promoted).expect("a node of the game");
        let promoted = u.promoted.len() as usize;
        let max_add = (p.max_total as i32 - u.promoted.len() as i32).min(state.max_promotions_among(&u.cands)).max(0);
        let max_add = max_add as usize;
        let fm = frame_matroids(model, &state, &u.cands);
        let plain = fm.bound(max_add);
        let target = p_star - u.promoted.sum() as i64;
        let open = plain.value > target;
        let size = plain.chosen.iter().filter(|&&c| c).count();
        // The weights by size agree with the plain bound and the size of its set (the smallest of the heaviest).
        let best = best_by_size(&fm, max_add);
        assert_eq!(best.iter().max(), Some(&plain.value), "n={n} node {i}: the bound from the weights by size");
        assert_eq!(best.iter().position(|&v| v == plain.value), Some(size), "n={n} node {i}: the size of the set");
        let place = Place::of(promoted + size, win);
        c.outside_any += usize::from(place != Place::Inside);
        if open {
            c.open += 1;
            match place {
                Place::Below => c.below += 1,
                Place::Inside => c.inside += 1,
                Place::Above => c.above += 1,
            }
        }
        println!(
            "{{\"kind\":\"node\",\"n\":{n},\"i\":{i},\"promoted\":{promoted},\"promoted_sum\":{},\"candidates\":{},\
             \"elements\":{},\"max_add\":{max_add},\"target\":{target},\"plain\":{},\"size\":{size},\"open\":{open},\
             \"place\":\"{}\"}}",
            u.promoted.sum(),
            u.cands.len(),
            fm.elems.len(),
            plain.value,
            place.name(),
        );
    }
    let window_json = win.map_or("null".into(), |(lo, hi)| format!("[{lo},{hi}]"));
    let outside_share = if c.open == 0 { 0.0 } else { (c.below + c.above) as f64 / c.open as f64 };
    println!(
        "{{\"kind\":\"game\",\"n\":{n},\"p_star\":{p_star},\"max_total\":{max_total},\"root_bound\":{root_bound},\
         \"root_gap\":{},\"root_size\":{root_size},\"root_rank\":{root_rank},\"window\":{window_json},\"root_margins\":[{margins}],\
         \"nodes\":{},\"with_candidates\":{},\"outside_any\":{},\"open\":{},\"below\":{},\"inside\":{},\"above\":{},\
         \"outside_share\":{outside_share:.4}}}",
        root_bound - p_star,
        c.nodes,
        c.with_candidates,
        c.outside_any,
        c.open,
        c.below,
        c.inside,
        c.above,
    );
}

// ---------------------------------------------------------------------------------------------------------
// Tests (`cargo test --release --example promotions`)

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::Rng;
    use crate::meet::{intersect, Reduced, Span};
    use taxman::bits::Bits;

    /// Whether the elements `members` (indices into `fm.elems`, common independent) and `extra` are independent in
    /// both matroids.
    fn extends(fm: &FrameMatroids, members: &[usize], extra: usize) -> bool {
        [&fm.a, &fm.b].into_iter().all(|m| {
            let mut span = Span::new(m);
            members.iter().for_each(|&e| span.insert(m, e));
            matches!(span.reduce(m, extra), Reduced::Free)
        })
    }

    /// The exact-size bound of a node that has promoted `promoted` candidates: the most that a completion of a size
    /// that the window allows can add, `best` being the node's weights by size (computed at least up to the window's
    /// top less `promoted`, and `max_add`), or `None` if no size of the window can be reached.  The node is closed
    /// when `promoted`'s sum plus this is at most `P*` (or when there is none).
    fn exact_size_bound(window: Option<(usize, usize)>, promoted: usize, best: &[i64], max_add: usize) -> Option<i64> {
        let (lo, hi) = window?;
        (lo..=hi)
            .filter_map(|k| k.checked_sub(promoted))
            .filter(|&j| j <= max_add && j < best.len())
            .map(|j| best[j])
            .max()
    }

    /// The heaviest common independent set of each size by the enumeration of all of them.
    fn brute_by_size(fm: &FrameMatroids) -> Vec<i64> {
        fn go(fm: &FrameMatroids, from: usize, members: &mut Vec<usize>, weight: i64, best: &mut Vec<i64>) {
            if best.len() <= members.len() {
                best.push(i64::MIN);
            }
            best[members.len()] = best[members.len()].max(weight);
            for e in from..fm.elems.len() {
                if extends(fm, members, e) {
                    members.push(e);
                    go(fm, e + 1, members, weight + i64::from(fm.elems[e]), best);
                    members.pop();
                }
            }
        }
        let mut best = Vec::new();
        go(fm, 0, &mut Vec::new(), 0, &mut best);
        best
    }

    /// The boards of the tests: all of 20 to 60 that have candidates (at most 11 of them, so that the enumeration of
    /// every common independent set is small).
    fn boards() -> Vec<u32> {
        (20..=60).filter(|&n| taxman::with_width!(n, W => setup::<W>(n).is_some())).collect()
    }

    /// Both algorithms give the weights by size of the enumeration, at the root and at nodes (a random promotable
    /// set promoted), whatever the limit.
    fn by_size_matches_enumeration<const W: usize>(n: u32) {
        let (_, p) = setup::<W>(n).expect("a game with candidates");
        let model = &p.model;
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15 ^ u64::from(n));
        let mut scratch = Scratch::<W>::new();
        for round in 0..4 {
            let mut state = p.root;
            let mut cands = p.candidates;
            if round > 0 {
                let mut order: Vec<u32> = p.candidates.iter().collect();
                for i in (1..order.len()).rev() {
                    order.swap(i, rng.below(i as u64 + 1) as usize);
                }
                for &x in order.iter().take(round) {
                    if model.promote_in_place_using(&mut scratch, &mut state, x) {
                        cands.remove(x);
                    }
                }
            }
            let fm = frame_matroids(model, &state, &cands);
            let brute = brute_by_size(&fm);
            let all = best_by_size(&fm, usize::MAX);
            assert_eq!(all, brute, "n={n} round {round}: the weights by size (fast or reference)");
            // The reference algorithm, which starts from nothing and runs through every size.
            let w: Vec<i64> = fm.elems.iter().map(|&x| i64::from(x)).collect();
            assert_eq!(intersect(&fm.a, &fm.b, &w, usize::MAX, true).by_size, brute, "n={n}: the reference");
            for limit in 0..brute.len() + 1 {
                let limited = best_by_size(&fm, limit);
                assert_eq!(limited, brute[..brute.len().min(limit + 1)], "n={n} limit {limit}");
            }
            // The plain bound is the largest of them.
            let max_add = (p.max_total as i32 - round as i32).max(0) as usize;
            let bound = fm.bound(max_add).value;
            assert_eq!(bound, brute[..brute.len().min(max_add + 1)].iter().copied().max().unwrap(), "n={n}");
        }
    }

    #[test]
    fn the_weights_by_size_are_those_of_the_enumeration() {
        let boards = boards();
        assert!(boards.len() > 20, "{boards:?}");
        for n in boards {
            taxman::with_width!(n, W => by_size_matches_enumeration::<W>(n));
        }
    }

    /// A node made for the soundness test: what it has promoted, the best sum of a completion within its candidates
    /// (by the enumeration), its plain bound, its limit on the completion's size and its weights by size.
    struct Probe {
        promoted: usize,
        sum: i64,
        completion: i64,
        plain: i64,
        max_add: usize,
        best: Vec<i64>,
    }

    /// Closing a node by the exact-size bound loses no completion that beats the incumbent: on a board with every
    /// promotable set enumerated, for every promotable set as a node's promotions with its candidates thinned at
    /// random, and for every incumbent from 0 to one above the optimum (so every window there is), a closed node has
    /// no completion with a sum above the incumbent, and the plain bound is never below the best completion.
    /// Returns the pairs of a node and an incumbent tried.
    fn closing_is_sound<const W: usize>(n: u32) -> usize {
        let (_, p) = setup::<W>(n).expect("a game with candidates");
        let model = &p.model;
        let mut scratch = Scratch::<W>::new();
        // Every promotable set.
        let cands: Vec<u32> = p.candidates.iter().collect();
        let mut sets: Vec<Bits<W>> = Vec::new();
        let mut stack = vec![(p.root, Bits::<W>::EMPTY, 0usize)];
        while let Some((state, set, from)) = stack.pop() {
            sets.push(set);
            for (i, &x) in cands.iter().enumerate().skip(from) {
                if let Some(next) = model.promote_all_using(&mut scratch, &state, &Bits::EMPTY.with(x)) {
                    stack.push((next, set.with(x), i + 1));
                }
            }
        }
        let truth = sets.iter().map(|s| s.sum()).max().expect("the empty set") as i64;
        let root_fm = frame_matroids(model, &p.root, &p.candidates);
        let max_total = p.max_total as usize;
        let root = best_by_size(&root_fm, max_total);
        let mut rng = Rng(0x2545_F491_4F6C_DD1D ^ u64::from(n));
        let mut probes = Vec::new();
        for set in &sets {
            let state = model.promote_all_using(&mut scratch, &p.root, set).expect("a promotable set");
            let cands: Bits<W> = p.candidates.iter().filter(|&x| !set.contains(x) && rng.below(4) > 0).collect();
            let within = |t: &&Bits<W>| {
                set.iter().all(|x| t.contains(x)) && t.iter().all(|x| set.contains(x) || cands.contains(x))
            };
            let completion = sets.iter().filter(within).map(|t| t.sum() as i64 - set.sum() as i64).max().unwrap();
            let promoted = set.len() as usize;
            let room = (p.max_total as i32 - promoted as i32).min(state.max_promotions_among(&cands));
            let max_add = room.max(0) as usize;
            let fm = frame_matroids(model, &state, &cands);
            let plain = fm.bound(max_add).value;
            assert!(plain >= completion, "n={n}: the plain bound {plain} is below the completion {completion}");
            let best = best_by_size(&fm, max_add);
            probes.push(Probe { promoted, sum: set.sum() as i64, completion, plain, max_add, best });
        }
        let mut tried = 0;
        for p_star in 0..=truth + 1 {
            let win = window(&root, max_total, p_star);
            for probe in &probes {
                let target = p_star - probe.sum;
                let bound = exact_size_bound(win, probe.promoted, &probe.best, probe.max_add);
                tried += 1;
                if bound.is_none_or(|b| b <= target) {
                    let completion = probe.completion;
                    assert!(completion <= target, "n={n}: closed a node whose completion {completion} beats {target}");
                } else {
                    assert!(bound.unwrap() <= probe.plain, "n={n}: the exact-size bound is above the plain bound");
                }
            }
        }
        tried
    }

    /// (On these boards, whose games promote at most 3 numbers, the exact-size bound is never below the plain bound
    /// where it does not close the node: the test checks that closing loses nothing, not that it gains.)
    #[test]
    fn closing_by_the_exact_size_bound_is_sound() {
        let tried: usize = boards().into_iter().map(|n| taxman::with_width!(n, W => closing_is_sound::<W>(n))).sum();
        assert!(tried > 10_000, "{tried}");
    }

    /// The window and the exact-size bound on weights by size made by hand.
    #[test]
    fn the_window_and_the_bound_by_hand() {
        let best = [0, 10, 18, 24, 28, 30, 30, 28];
        // Above 29 only sizes 5 and 6 (30 and 30); above 24 sizes 4 to 7.
        assert_eq!(window(&best, 7, 29), Some((5, 6)));
        assert_eq!(window(&best, 7, 24), Some((4, 7)));
        assert_eq!(window(&best, 5, 29), Some((5, 5)), "max_total cuts the window");
        assert_eq!(window(&best, 7, 30), None);
        assert_eq!(window(&best, 7, -1), Some((0, 7)));
        // A node with two promotions reaches the totals 5 and 6 by completions of 3 and 4.
        let node = [0, 9, 17, 20, 21];
        assert_eq!(exact_size_bound(Some((5, 6)), 2, &node, 4), Some(21));
        assert_eq!(exact_size_bound(Some((5, 6)), 2, &node, 3), Some(20), "max_add cuts the sizes");
        assert_eq!(exact_size_bound(Some((5, 6)), 2, &node[..4], 4), Some(20), "no set of 4 more");
        assert_eq!(exact_size_bound(Some((5, 6)), 2, &node, 2), None, "the window is out of reach");
        assert_eq!(exact_size_bound(Some((5, 6)), 7, &node, 4), None, "more promotions than the window allows");
        assert_eq!(exact_size_bound(None, 0, &node, 4), None);
    }
}
