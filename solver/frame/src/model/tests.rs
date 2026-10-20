use super::peel::{reduce_from, winnable, Worklist};
use super::test_support::*;
use super::*;
use crate::json::Json;

fn bits<const W: usize>(xs: &[u32]) -> Bits<W> {
    Bits::from_slice(xs)
}

// ---- the Java's algorithms, transliterated, as the reference ----

/// The Java solver's test of whether a frame is winnable, step for step.
fn ref_is_winnable<const W: usize>(moves: Bits<W>, factors: Bits<W>, math: &FactorProvider) -> bool {
    let (mut rm, mut rf) = (moves, factors);
    if rm.len() > rf.len() {
        return false;
    }
    if rm.is_empty() {
        return true;
    }
    let mut progress = true;
    while progress {
        progress = false;
        let mut cursor = rm.max();
        while let Some(m) = cursor {
            let live: Vec<u32> = math.factors(m).iter().map(|&q| u32::from(q)).filter(|&q| rf.contains(q)).collect();
            match live.len() {
                0 => return false,
                1 => {
                    rm.remove(m);
                    if rm.is_empty() {
                        return true;
                    }
                    rf.remove(live[0]);
                    progress = true;
                }
                _ => {}
            }
            cursor = rm.max_below(m);
        }
        let mut cursor = rf.max();
        while let Some(f) = cursor {
            let live: Vec<u32> = math.composites(f).iter().map(|&c| u32::from(c)).filter(|&c| rm.contains(c)).collect();
            if live.len() == 1 {
                rm.remove(live[0]);
                if rm.is_empty() {
                    return true;
                }
                rf.remove(f);
                progress = true;
            }
            cursor = rf.max_below(f);
        }
    }
    false
}

/// The Java solver's reduction of a frame, step for step (with its assertions off).
fn ref_reduce<const W: usize>(moves: Bits<W>, factors: Bits<W>, math: &FactorProvider) -> (Bits<W>, Bits<W>) {
    let (mut rm, mut rf) = (moves, factors);
    let mut progress = true;
    while progress {
        if rm.is_empty() {
            break;
        }
        progress = false;
        let mut cursor = rm.max();
        while let Some(m) = cursor {
            let live: Vec<u32> = math.factors(m).iter().map(|&q| u32::from(q)).filter(|&q| rf.contains(q)).collect();
            if live.len() <= 1 {
                rm.remove(m);
                if let Some(&q) = live.first() {
                    rf.remove(q);
                }
                progress = true;
            }
            cursor = rm.max_below(m);
        }
    }
    (rm, rf)
}

/// Most moves [`tax_winnable`] handles.
const BRUTE_MOVES: usize = 14;

/// The meaning of winnable, by exhaustion: the moves can be taken in some order so that each still has a
/// factor present when it is taken, where taking a move removes all of its factors still present (its tax).
/// The factors used up depend only on which moves have been taken, so this searches subsets of moves.
fn tax_winnable<const W: usize>(moves: Bits<W>, factors: Bits<W>, math: &FactorProvider) -> bool {
    let ms = moves.to_vec();
    let k = ms.len();
    assert!(k <= BRUTE_MOVES);
    let tax: Vec<Bits<W>> = ms
        .iter()
        .map(|&m| math.factors(m).iter().map(|&q| u32::from(q)).filter(|&q| factors.contains(q)).collect())
        .collect();
    let full = (1usize << k) - 1;
    let mut used = vec![Bits::<W>::EMPTY; full + 1];
    let mut reached = vec![false; full + 1];
    reached[0] = true;
    for taken in 0..full {
        if !reached[taken] {
            continue;
        }
        for (j, t) in tax.iter().enumerate() {
            let next = taken | 1 << j;
            if next != taken && !t.is_subset(&used[taken]) {
                reached[next] = true;
                used[next] = used[taken] | *t;
            }
        }
    }
    reached[full]
}

// ---- the Java's unit tests ----

#[test]
fn java_frame_test_cant_promote_if_its_not_there() {
    let t = Tables::new(200);
    let math = FactorProvider::new(&t, &[108, 72, 54, 36, 10]);
    let frame = Frame::<4>::new(bits(&[108, 72]), bits(&[54, 36]));
    assert!(frame.promote_out_of(10, &math).is_none());
    assert!(!frame.can_promote_out_of(10, &math));
}

#[test]
fn java_frame_test_tiny() {
    let t = Tables::new(200);
    let math = FactorProvider::new(&t, &[108, 72, 54, 36, 10]);
    let frame = Frame::<4>::new(bits(&[108, 72]), bits(&[54, 36, 10]));
    let result = frame.promote_out_of(10, &math).expect("promotable");
    assert!(result.factors.is_empty() && result.moves.is_empty());
    assert_eq!(result.freedoms, 0);
}

#[test]
fn java_frame_test_derived_from_120() {
    let t = Tables::new(200);
    let moves = [69, 74, 77, 82, 85, 86, 87, 91, 93, 94, 95, 106, 111, 115, 118, 11];
    let factors = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59];
    let mut numbers: Vec<u32> = moves.iter().chain(&factors).copied().collect();
    numbers.push(39);
    let math = FactorProvider::new(&t, &numbers);
    let frame = Frame::<4>::new(bits(&moves), bits(&factors));
    assert!(frame.promote_into(9, &math).is_none());
    assert!(!frame.can_promote_into(9, &math));
}

fn columns_85() -> Vec<Vec<u32>> {
    vec![
        vec![4, 6, 9, 10, 14, 15, 21, 25],
        vec![45, 50, 63, 75],
        vec![8, 12, 20, 28, 30, 42],
        vec![56, 60, 84],
        vec![16, 24, 36, 40],
        vec![48, 72, 80],
    ]
}

const CANDIDATES_85: [u32; 10] = [8, 12, 16, 20, 24, 28, 30, 36, 40, 42];

#[test]
fn java_frame_set_test_promote() {
    let (model, fs) = FrameGraph::build::<2>(&Tables::new(85), &columns_85()).unwrap();
    let mut state = fs;
    for n in [42, 40, 28] {
        state = model.promote(&state, n).expect("promotable");
    }
    let expect: [(&[u32], &[u32]); 3] =
        [(&[4, 6, 9, 10, 14, 15, 21, 25], &[28, 42, 45, 50, 63, 75]), (&[], &[]), (&[], &[])];
    assert_eq!(state.len(), 3);
    for (frame, (factors, moves)) in state.frames().iter().zip(expect) {
        assert_eq!((frame.factors.to_vec(), frame.moves.to_vec()), (factors.to_vec(), moves.to_vec()));
    }
}

#[test]
fn java_frame_set_test_candidates_and_max_moves() {
    let (model, fs) = FrameGraph::build::<2>(&Tables::new(85), &columns_85()).unwrap();
    let candidates = bits::<2>(&CANDIDATES_85);
    assert_eq!(fs.narrow_candidates(&candidates), candidates);
    assert_eq!(fs.max_promotions(), 3);
    let state = [42, 40, 28].iter().fold(fs, |s, &n| model.promote(&s, n).unwrap());
    assert_eq!(state.max_promotions(), 0);
}

#[test]
fn frame_set_helpers() {
    let (model, fs) = FrameGraph::build::<2>(&Tables::new(85), &columns_85()).unwrap();
    assert_eq!((model.home(42), model.home(40), model.home(9), model.home(84), model.home(999)), (1, 2, 0, 0, 0));
    assert!(model.contains_factor(&fs, 42) && !model.contains_factor(&fs, 84) && !model.contains_factor(&fs, 999));
    assert!(!model.can_promote(&fs, 9) && model.promote(&fs, 9).is_none(), "frame 0 cannot promote");
    let mut copy = model.promote(&fs, 42).expect("promotable");
    assert_ne!(copy, fs);
    copy.assign(&fs);
    assert_eq!(copy, fs);
    let mut failed = fs;
    assert!(!model.promote_in_place(&mut failed, 9));
    assert_eq!(failed, fs);
}

// ---- golden data ----

fn check_golden_frame_set<const W: usize>(g: &Golden) {
    let (_, fs) = FrameGraph::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
    let expect = g.json.get("frameSet").and_then(Json::as_arr).expect("frameSet");
    assert_eq!(fs.len(), expect.len(), "n={}", g.n);
    for (i, (frame, want)) in fs.frames().iter().zip(expect).enumerate() {
        assert_eq!(frame.moves.to_vec(), u32s(want, "moves"), "n={} frame {i} moves", g.n);
        assert_eq!(frame.factors.to_vec(), u32s(want, "factors"), "n={} frame {i} factors", g.n);
        assert_eq!(Some(i64::from(frame.freedoms)), want.get("freedoms").and_then(Json::as_i64), "n={}", g.n);
    }
    let candidates = Bits::<W>::from_slice(&g.candidates);
    let max = g.json.get("maxPromotionsUnrevised").and_then(Json::as_i64).expect("maxPromotionsUnrevised");
    assert_eq!(i64::from(fs.max_promotions_among(&candidates)), max, "n={}", g.n);
}

#[test]
fn golden_frame_sets_and_max_promotions() {
    let ns = golden_ns();
    assert!(ns.len() >= 300, "expected the golden data set, found {} files", ns.len());
    for n in ns {
        let g = load(n);
        if n < 512 {
            check_golden_frame_set::<8>(&g);
        } else {
            check_golden_frame_set::<16>(&g);
        }
    }
}

// ---- randomized agreement with the reference ----

/// Per frame of a golden game: the numbers that can be its moves and its factors.
fn universes<const W: usize>(columns: &[Vec<u32>]) -> Vec<(Bits<W>, Bits<W>)> {
    let len = columns.len() / 2;
    (0..len)
        .map(|i| {
            let mut moves = Bits::from_slice(&columns[2 * i + 1]);
            if i + 1 < len {
                moves |= Bits::from_slice(&columns[2 * i + 2]);
            }
            (moves, Bits::from_slice(&columns[2 * i]))
        })
        .collect()
}

#[derive(Default, Debug)]
struct PeelStats {
    checks: u64,
    winnable: u64,
    /// Checks small enough to compare with the exhaustive [`tax_winnable`].
    exhaustive: u64,
}

/// Checks peeling and reduction on `(moves, factors)` against the reference, and peeling against the
/// exhaustive definition when the frame is small enough.
fn check_frame<const W: usize>(
    moves: Bits<W>,
    factors: Bits<W>,
    graph: &FactorProvider,
    math: &FactorProvider,
    stats: &mut PeelStats,
) -> bool {
    let fast = winnable(&moves, &factors, graph);
    let reference = ref_is_winnable(moves, factors, math);
    assert_eq!(fast, reference, "peel disagrees on {moves:?} -> {factors:?}");
    assert_eq!(winnable(&moves, &factors, math), reference, "peel with the union lists disagrees");
    if moves.len() as usize <= BRUTE_MOVES {
        let truth = tax_winnable(moves, factors, math);
        assert_eq!(reference, truth, "peeling is not a decision procedure on {moves:?} -> {factors:?}");
        stats.exhaustive += 1;
    }
    stats.checks += 1;
    // The residual: empty exactly on winnable frames; each live factor of a residual move is shared within the
    // residual, and the residual alone is unwinnable.
    let core = Scratch::new().stuck(&moves, &factors, graph);
    assert_eq!(core.is_empty(), fast, "core {core:?} of {moves:?} -> {factors:?}");
    for m in core.iter() {
        for q in graph.factors(m).iter().map(|&q| u32::from(q)).filter(|&q| factors.contains(q)) {
            let shared = core.iter().any(|o| o != m && graph.factors(o).contains(&(q as u16)));
            assert!(shared, "core move {m} has the private factor {q} in {moves:?} -> {factors:?}");
        }
    }
    if !core.is_empty() {
        assert!(!winnable(&core, &factors, graph), "the core {core:?} alone is winnable");
    }
    if fast {
        stats.winnable += 1;
        let (rm, rf) = ref_reduce(moves, factors, math);
        let reduced = Frame::new(moves, factors).reduce(graph);
        assert_eq!((reduced.moves, reduced.factors), (rm, rf), "reduce disagrees on {moves:?} -> {factors:?}");
    }
    fast
}

fn random_agreement<const W: usize>(g: &Golden, rng: &mut Rng, stats: &mut PeelStats) {
    let tables = Tables::new(g.n);
    let (model, root) = FrameGraph::build::<W>(&tables, &g.columns).unwrap();
    let all: Vec<u32> = g.columns.iter().flatten().copied().collect();
    let math = FactorProvider::new(&tables, &all);
    let universes = universes::<W>(&g.columns);
    let candidates = Bits::<W>::from_slice(&g.candidates);

    for _walk in 0..2 {
        let mut state = root;
        loop {
            for (i, frame) in state.frames().iter().enumerate() {
                let (move_universe, factor_universe) = universes[i];
                assert!(check_frame(frame.moves, frame.factors, &model.graph, &math, stats));
                let extra: Vec<u32> = (move_universe - frame.moves).to_vec();
                let present: Vec<u32> = frame.factors.to_vec();
                for _ in 0..4 {
                    if !extra.is_empty() {
                        let x = rng.pick(&extra);
                        let moves = frame.moves.with(x);
                        if check_frame(moves, frame.factors, &model.graph, &math, stats) {
                            // The incremental reduction a promotion does equals a full one.
                            let (mut m, mut f) = (moves, frame.factors);
                            let mut pending = Worklist::new();
                            pending.insert(x);
                            reduce_from(&mut m, &mut f, pending, &model.graph);
                            assert_eq!((m, f), ref_reduce(moves, frame.factors, &math));
                            let java = frame.promote_into(x, &math).unwrap();
                            assert_eq!((java.moves, java.factors), (m, f));
                        }
                    }
                    if !present.is_empty() {
                        let y = rng.pick(&present);
                        let factors = frame.factors.without(y);
                        if check_frame(frame.moves, factors, &model.graph, &math, stats) {
                            let java = frame.promote_out_of(y, &math).unwrap();
                            assert_eq!((java.moves, java.factors), ref_reduce(frame.moves, factors, &math));
                        }
                    }
                    // Arbitrary frames over the same numbers, reachable or not: one of any size, and one
                    // small enough for the exhaustive check.
                    let density = 20 + rng.below(70);
                    let moves = rng.subset(&move_universe, density);
                    let factors = rng.subset(&factor_universe, (density + 15).min(100));
                    check_frame(moves, factors, &model.graph, &math, stats);
                    let pool = move_universe.to_vec();
                    let count = if pool.is_empty() { 0 } else { rng.below(BRUTE_MOVES as u64 + 1) as usize };
                    let moves: Bits<W> = (0..count).map(|_| rng.pick(&pool)).collect();
                    let density = 40 + rng.below(61);
                    let factors = rng.subset(&factor_universe, density);
                    check_frame(moves, factors, &model.graph, &math, stats);
                }
            }

            // Promote a random candidate (any order), checking the model against the Java's frame operations.
            let open: Vec<u32> = state.narrow_candidates(&candidates).to_vec();
            if open.is_empty() {
                break;
            }
            let n = rng.pick(&open);
            let h = model.home(n);
            let java_into = state.frames()[h - 1].promote_into(n, &math);
            let java_out = state.frames()[h].promote_out_of(n, &math);
            let expected = java_into.zip(java_out);
            assert_eq!(model.can_promote(&state, n), expected.is_some(), "can_promote({n})");
            match model.promote(&state, n) {
                Some(next) => {
                    let (into, out) = expected.expect("the Java promotes too");
                    assert_eq!((next.frames()[h - 1], next.frames()[h]), (into, out), "promote({n})");
                    state = next;
                }
                None => {
                    assert!(expected.is_none(), "promote({n}) failed but the Java succeeds");
                    if rng.chance(30) {
                        break;
                    }
                }
            }
        }
    }
}

/// A sample of the golden games (every 18th board and 480, 600, 720; two walks each: 1.5 s and 32,000 checks).
/// The seeds are fixed, so more boards or walks would only be more of one sample: the whole set (every 9th
/// board and all those from 480, six walks each) is 225,000 checks and 9 s; to run it, change the filter and the
/// number of walks (`0..6`).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "long: the quick check is fast_peeling_and_reduction_agree_with_reference_on_a_sample"
)]
fn fast_peeling_and_reduction_agree_with_reference() {
    fast_peeling_and_reduction_agree_with_reference_on_every(1);
}

/// [`fast_peeling_and_reduction_agree_with_reference`] on every fifth of its golden games.
#[test]
fn fast_peeling_and_reduction_agree_with_reference_on_a_sample() {
    fast_peeling_and_reduction_agree_with_reference_on_every(5);
}

/// The fast peeling and reduction agree with the reference on every `every`th of the golden games.
fn fast_peeling_and_reduction_agree_with_reference_on_every(every: usize) {
    let mut rng = Rng(0x5EED_1234_ABCD_0001);
    let mut stats = PeelStats::default();
    for n in golden_ns().into_iter().filter(|&n| n >= 20 && (n % 18 == 0 || n % 120 == 0)).step_by(every) {
        let g = load(n);
        if n < 512 {
            random_agreement::<8>(&g, &mut rng, &mut stats);
        } else {
            random_agreement::<16>(&g, &mut rng, &mut stats);
        }
    }
    eprintln!("peel agreement: {stats:?}");
    assert!(stats.checks > 10_000 / every as u64 && stats.winnable > 1_000 / every as u64);
}

/// Promoting the same numbers in different orders reaches the same state whenever every step succeeds.
fn confluence<const W: usize>(g: &Golden, rng: &mut Rng) -> (u64, u64) {
    let (model, root) = FrameGraph::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
    let candidates = Bits::<W>::from_slice(&g.candidates);
    let (mut orders, mut failed_orders) = (0, 0);
    for _ in 0..20 {
        // A promotable set, chosen at random in descending order.
        let mut state = root;
        let mut chosen = Vec::new();
        for n in candidates.iter_desc() {
            if rng.chance(40) && model.promote_in_place(&mut state, n) {
                chosen.push(n);
            }
        }
        for _ in 0..10 {
            let mut order = chosen.clone();
            rng.shuffle(&mut order);
            let mut other = root;
            orders += 1;
            if order.iter().all(|&n| model.promote_in_place(&mut other, n)) {
                assert_eq!(other, state, "n={}: order {order:?} reaches a different state", g.n);
            } else {
                failed_orders += 1;
            }
        }
    }
    (orders, failed_orders)
}

#[test]
fn promotion_order_does_not_matter() {
    let mut rng = Rng(0xC0FF_EE00_1234_5678);
    let (mut orders, mut failed) = (0, 0);
    for n in golden_ns().into_iter().filter(|&n| n >= 20 && (n % 7 == 0 || n >= 480)) {
        let g = load(n);
        let (o, f) = if n < 512 { confluence::<8>(&g, &mut rng) } else { confluence::<16>(&g, &mut rng) };
        orders += o;
        failed += f;
    }
    eprintln!("confluence: {orders} orders tried, {failed} failed part way");
    assert_eq!(failed, 0, "a promotable set failed in some other order");
}

/// Checks [`FrameGraph::conflict_using`] on `set` in `state`: none exactly when the set can be promoted, else a
/// non-empty subset that cannot, the minimal one no larger than the one the residual gives.
fn check_conflicts<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, set: &Bits<W>, promotable: bool) {
    let mut scratch = Scratch::new();
    let core = model.conflict_using(&mut scratch, state, set, false);
    let minimal = model.conflict_using(&mut scratch, state, set, true);
    assert_eq!(core.is_none(), promotable, "{set:?}: core conflict {core:?}");
    assert_eq!(minimal.is_none(), promotable, "{set:?}: minimal conflict {minimal:?}");
    if let (Some(core), Some(minimal)) = (core, minimal) {
        for k in [core, minimal] {
            assert!(!k.is_empty() && k.is_subset(set), "conflict {k:?} of {set:?}");
            assert!(!model.can_promote_all_using(&mut scratch, state, &k), "conflict {k:?} can be promoted");
        }
        assert!(minimal.len() <= core.len(), "minimal {minimal:?} larger than {core:?}");
    }
}

/// Promoting a set one by one succeeds exactly when every frame is winnable with the whole set promoted.
fn set_check<const W: usize>(g: &Golden, rng: &mut Rng) -> (u64, u64) {
    let (model, root) = FrameGraph::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
    let candidates = Bits::<W>::from_slice(&g.candidates);
    let (mut promotable, mut tried) = (0, 0);
    for _ in 0..200 {
        let density = 3 + rng.below(30);
        let set = rng.subset(&candidates, density);
        // Conflicts in a state with a few promotions made.
        let mut start = root;
        for n in rng.subset(&candidates, 5).iter_desc() {
            model.promote_in_place(&mut start, n);
        }
        let later = set & start.narrow_candidates(&candidates);
        check_conflicts(&model, &start, &later, model.promote_all_using(&mut Scratch::new(), &start, &later).is_some());
        let mut state = root;
        let one_by_one = set.iter_desc().all(|n| model.promote_in_place(&mut state, n));
        check_conflicts(&model, &root, &set, one_by_one);
        let frames = root.frames();
        let at_once = set.iter().all(|n| model.home(n) >= 1 && model.contains_factor(&root, n))
            && frames.iter().enumerate().all(|(i, f)| {
                let gained: Bits<W> = set.iter().filter(|&n| model.home(n) == i + 1).collect();
                let lost: Bits<W> = set.iter().filter(|&n| model.home(n) == i).collect();
                model.winnable(&(f.moves | gained), &(f.factors - lost))
            });
        assert_eq!(one_by_one, at_once, "n={}: {set:?}", g.n);
        let all = model.promote_all_using(&mut Scratch::new(), &root, &set);
        assert_eq!(all, one_by_one.then_some(state), "n={}: promote_all of {set:?}", g.n);
        tried += 1;
        promotable += u64::from(one_by_one);
    }
    (tried, promotable)
}

#[test]
fn sets_can_be_checked_frame_by_frame() {
    let mut rng = Rng(0xFEED_FACE_0BAD_CAFE);
    let (mut tried, mut promotable) = (0, 0);
    for n in golden_ns().into_iter().filter(|&n| n >= 20 && (n % 5 == 0 || n >= 480)) {
        let g = load(n);
        let (t, f) = if n < 512 { set_check::<8>(&g, &mut rng) } else { set_check::<16>(&g, &mut rng) };
        tried += t;
        promotable += f;
    }
    eprintln!("frame-by-frame set check: {tried} sets, {promotable} promotable");
    assert!(promotable > tried / 10 && promotable < tried);
}

/// Nogoods made from the conflicts of states along random promotion paths of `g`: each is a subset of the
/// state's promotions and the conflict, and cannot be promoted from the root; and
/// every nogood learned so far, minus the promotions of any later state, is a conflict of that state.
/// Returns the numbers of nogoods made and of later checks.  Eight paths per game (forty took 2 s and made 13,500
/// nogoods, checked 3.8 million times later).
fn nogood_check<const W: usize>(g: &Golden, rng: &mut Rng) -> (u64, u64) {
    let (model, root) = FrameGraph::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
    let candidates = Bits::<W>::from_slice(&g.candidates);
    let mut scratch = Scratch::new();
    let mut table = Nogoods::<W>::new(g.n, 1 << 10);
    let (mut made, mut later) = (0, 0);
    for _ in 0..8 {
        let (mut state, mut promoted) = (root, Bits::<W>::EMPTY);
        for n in rng.subset(&candidates, 20).iter() {
            if !model.promote_in_place(&mut state, n) {
                continue;
            }
            promoted.insert(n);
            // Every nogood holds here.
            for i in 0..table.len() {
                let rest = table.sets[i] - promoted;
                assert!(!rest.is_empty(), "n={}: {:?} is promoted", g.n, table.sets[i]);
                assert!(!model.can_promote_all_using(&mut scratch, &state, &rest), "n={}: {rest:?}", g.n);
                later += 1;
            }
            let set = rng.subset(&state.narrow_candidates(&candidates), 30) - promoted;
            let Some(k) = model.conflict_using(&mut scratch, &state, &set, true) else { continue };
            let nogood = model.nogood(&mut scratch, &root, &promoted, &k).expect("a conflict has a nogood");
            let what = format!("n={}: nogood {nogood:?} of {k:?} after {promoted:?}", g.n);
            assert!(nogood.is_subset(&(promoted | k)), "{what}");
            assert!(!model.can_promote_all_using(&mut scratch, &root, &nogood), "{what} can be promoted");
            // The table knows it now, as the smallest conflict there is, or one as small.
            table.insert(nogood);
            let found = table.conflict(&promoted, &set).expect("a conflict in the table");
            assert!(found.len() <= k.len() && found.is_subset(&set), "{what}: the table has {found:?}");
            made += 1;
        }
    }
    (made, later)
}

#[test]
fn nogoods_hold_everywhere() {
    let mut rng = Rng(0x0DD5_EED5_1234_5678);
    let (mut made, mut later) = (0, 0);
    for n in golden_ns().into_iter().filter(|&n| n >= 40 && (n % 7 == 0 || n >= 480)) {
        let g = load(n);
        let (m, l) = if n < 512 { nogood_check::<8>(&g, &mut rng) } else { nogood_check::<16>(&g, &mut rng) };
        made += m;
        later += l;
    }
    eprintln!("nogoods: {made} made, {later} checked at later states");
    assert!(made > 500 && later > 5000);
}

/// What [`Nogoods::lookup`] answers by definition, for the nogoods `learned` in the order they were learned: the
/// smallest `N − P` over the nogoods `N ⊆ P ∪ S` (then the one with the largest member, then the first learned), or
/// `AllForced` at the first of them within `P ∪ F`.
fn lookup_by_definition<const W: usize>(
    learned: &[Bits<W>],
    promoted: &Bits<W>,
    set: &Bits<W>,
    musts: &Bits<W>,
) -> Lookup<W> {
    let within = *promoted | *set;
    let mut best = None;
    for (id, n) in learned.iter().enumerate().filter(|(_, n)| n.is_subset(&within)) {
        let k = *n - *promoted;
        if k.is_subset(musts) {
            return Lookup::AllForced;
        }
        let key = (k.len(), std::cmp::Reverse(n.max()), id);
        if best.is_none_or(|(b, _)| key < b) {
            best = Some((key, k));
        }
    }
    best.map_or(Lookup::None, |(_, k)| Lookup::Conflict(k))
}

/// What [`Nogoods::ruled_out`] answers by definition: the last member, among `set`, of each nogood with a member in
/// `added` and all the others promoted.
fn ruled_out_by_definition<const W: usize>(
    learned: &[Bits<W>],
    added: &Bits<W>,
    promoted: &Bits<W>,
    set: &Bits<W>,
) -> Bits<W> {
    let mut out = Bits::EMPTY;
    for n in learned.iter().filter(|n| n.intersects(added)) {
        let rest = *n - *promoted;
        if rest.len() == 1 && rest.is_subset(set) {
            out |= rest;
        }
    }
    out
}

/// The table's lookups and rule-outs against their definition, on random tables over up to 250 numbers, so that
/// the compact sets grow by several words, cleared now and then.
#[test]
fn nogood_lookups_follow_their_definition() {
    let mut rng = Rng(0x0DD5_1234_ABCD_0042);
    let mut table = Nogoods::<4>::new(255, 1 << 12);
    let mut learned: Vec<Bits<4>> = Vec::new();
    let (mut conflicts, mut forced, mut ruled) = (0, 0, 0);
    for _ in 0..2000 {
        if rng.below(500) == 0 {
            table.clear();
            learned.clear();
        }
        // Most members from a crowd of 24, so that the lookups find nogoods.
        let size = 1 + rng.below(4);
        let n: Bits<4> =
            (0..size).map(|_| if rng.chance(70) { rng.below(24) } else { rng.below(250) } as u32).collect();
        table.insert(n);
        learned.push(n);
        for _ in 0..4 {
            let crowd = Bits::from_range(0, 23) | (0..20).map(|_| rng.below(250) as u32).collect();
            let promoted = rng.subset(&crowd, 10);
            let set = rng.subset(&crowd, 60) - promoted;
            let musts = rng.subset(&set, 15);
            let expected = lookup_by_definition(&learned, &promoted, &set, &musts);
            assert_eq!(table.lookup(&promoted, &set, &musts), expected, "{learned:?} {promoted:?} {set:?}");
            conflicts += u32::from(matches!(expected, Lookup::Conflict(_)));
            forced += u32::from(expected == Lookup::AllForced);
            let added = rng.subset(&promoted, 40);
            let out = ruled_out_by_definition(&learned, &added, &promoted, &set);
            assert_eq!(table.ruled_out(&added, &promoted, &set), out, "{learned:?} {added:?} {promoted:?}");
            ruled += out.len();
        }
    }
    assert!(
        conflicts > 500 && forced > 200 && ruled > 200,
        "{conflicts} conflicts, {forced} forced, {ruled} ruled out"
    );
}

/// The same at the widths the large searches use: a `Nogoods<16>` over numbers up to 1023 whose nogoods come to
/// have more than 15 · 64 members, so that the compact sets are 1 to 16 words wide, through the strides of 1 to 6
/// (whose scans are unrolled) and on into the generic scan.  Lookups go through crowds that contain learned
/// nogoods, so that they find some at every stride.
#[test]
fn nogood_lookups_follow_their_definition_at_every_stride() {
    const MAX: u32 = 1023;
    let mut rng = Rng(0x9A4E_0001_2345_6789);
    let mut table = Nogoods::<16>::new(MAX, 1 << 14);
    let mut learned: Vec<Bits<16>> = Vec::new();
    let mut members = Bits::<16>::EMPTY;
    let (mut most, mut conflicts, mut forced, mut ruled, mut wide) = (0, 0, 0, 0, 0);
    for _ in 0..6000 {
        if rng.below(2500) == 0 {
            table.clear();
            learned.clear();
            members = Bits::EMPTY;
        }
        let size = 1 + rng.below(4);
        let n: Bits<16> = (0..size)
            .map(|_| if rng.chance(50) { 1 + rng.below(24) } else { 1 + rng.below(u64::from(MAX)) } as u32)
            .collect();
        table.insert(n);
        learned.push(n);
        members |= n;
        most = most.max(members.len());
        for _ in 0..3 {
            let mut crowd: Bits<16> = (0..20).map(|_| 1 + rng.below(u64::from(MAX)) as u32).collect();
            for _ in 0..4 {
                crowd |= learned[rng.below(learned.len() as u64) as usize];
            }
            crowd |= Bits::from_range(1, 24);
            let promoted = rng.subset(&crowd, 5);
            let set = rng.subset(&crowd, 75) - promoted;
            let musts = if rng.chance(80) { Bits::EMPTY } else { rng.subset(&set, 10) };
            let expected = lookup_by_definition(&learned, &promoted, &set, &musts);
            assert_eq!(table.lookup(&promoted, &set, &musts), expected, "{} members", members.len());
            conflicts += u32::from(matches!(expected, Lookup::Conflict(_)));
            forced += u32::from(expected == Lookup::AllForced);
            wide += u32::from(members.len() > 7 * 64 && expected != Lookup::None);
            let added = rng.subset(&promoted, 40);
            let out = ruled_out_by_definition(&learned, &added, &promoted, &set);
            assert_eq!(table.ruled_out(&added, &promoted, &set), out, "{} members", members.len());
            ruled += out.len();
        }
    }
    assert!(most > 15 * 64, "at most {most} members");
    assert!(
        conflicts > 1000 && forced > 200 && ruled > 200 && wide > 500,
        "{conflicts} conflicts, {forced} forced, {ruled} ruled out, {wide} found past a stride of 7"
    );
}

#[test]
fn nogood_table_lookups() {
    let mut table = Nogoods::<1>::new(60, 3);
    let set = |xs: &[u32]| Bits::<1>::from_slice(xs);
    table.insert(set(&[10, 20, 30]));
    table.insert(set(&[5, 40]));
    // The smallest conflict among the nogoods within the promotions and the set.
    assert_eq!(table.conflict(&set(&[]), &set(&[5, 10, 20, 30, 40])), Some(set(&[5, 40])));
    assert_eq!(table.conflict(&set(&[10, 20]), &set(&[5, 30, 40])), Some(set(&[30])));
    assert_eq!(table.conflict(&set(&[10]), &set(&[20, 40])), None);
    // Ruled out: the last member of a nogood whose others are all promoted, if a newest promotion is in it.
    let promoted = set(&[10, 20, 5]);
    assert_eq!(table.ruled_out(&set(&[20]), &promoted, &set(&[30, 40, 50])), set(&[30]));
    assert_eq!(table.ruled_out(&set(&[5, 20]), &promoted, &set(&[30, 40, 50])), set(&[30, 40]));
    assert_eq!(table.ruled_out(&set(&[5, 20]), &promoted, &set(&[50])), set(&[]));
    assert_eq!(table.ruled_out(&set(&[10]), &set(&[10]), &set(&[20, 30])), set(&[]));
    // A full table forgets everything before taking another.
    table.insert(set(&[1, 2]));
    assert_eq!(table.len(), 3);
    table.insert(set(&[3, 4]));
    assert_eq!(table.len(), 1);
    assert_eq!(table.conflict(&set(&[]), &set(&[5, 40, 1, 2])), None);
    assert_eq!(table.conflict(&set(&[3]), &set(&[4])), Some(set(&[4])));
    assert_eq!(table.ruled_out(&set(&[2]), &set(&[2]), &set(&[1])), set(&[]));
}

/// A table of four nogoods in which `b` has answered two lookups, then `c` one, then `d` ruled a candidate out; `a` has
/// not been used.
fn used_table() -> Nogoods<1> {
    let set = |xs: &[u32]| Bits::<1>::from_slice(xs);
    let mut table = Nogoods::<1>::new(60, 100);
    for n in [&[1, 2][..], &[3, 4], &[5, 6, 7], &[8, 9]] {
        table.insert(set(n));
    }
    assert_eq!(table.conflict(&set(&[3]), &set(&[4])), Some(set(&[4])));
    assert_eq!(table.conflict(&set(&[3]), &set(&[4])), Some(set(&[4])));
    assert_eq!(table.conflict(&set(&[5, 6]), &set(&[7])), Some(set(&[7])));
    assert_eq!(table.ruled_out(&set(&[8]), &set(&[8]), &set(&[9])), set(&[9]));
    table
}

#[test]
fn nogood_table_forgets_as_the_policy_says() {
    let set = |xs: &[u32]| Bits::<1>::from_slice(xs);
    let (a, b, c, d) = (set(&[1, 2]), set(&[3, 4]), set(&[5, 6, 7]), set(&[8, 9]));
    for (keep, kept) in [
        (Keep::Nothing, vec![]),
        // Everything used since the table was made: not `a`.
        (Keep::Used, vec![b, c, d]),
        // `b` is the most used; of `c` and `d`, used once each, `d` more recently.
        (Keep::Top(1), vec![b]),
        (Keep::Top(2), vec![b, d]),
        (Keep::Top(9), vec![b, c, d]),
        (Keep::Short(1), vec![]),
        (Keep::Short(2), vec![a, b, d]),
        (Keep::Short(3), vec![a, b, c, d]),
    ] {
        let mut table = used_table();
        table.forget(keep);
        assert_eq!(table.sets(), kept, "{keep}");
        // What is kept still answers, and nothing else does; the uses start again, so that nothing kept by use
        // survives a second forgetting at once.
        for n in table.sets().to_vec() {
            let first = n.max().expect("a member");
            assert_eq!(table.conflict(&(n - set(&[first])), &set(&[first])), Some(set(&[first])), "{keep}");
        }
        for n in [a, b, c, d].into_iter().filter(|n| !kept.contains(n)) {
            let first = n.max().expect("a member");
            assert_eq!(table.conflict(&(n - set(&[first])), &set(&[first])), None, "{keep}: {n:?} is forgotten");
        }
        let mut again = used_table();
        again.forget(keep);
        again.forget(Keep::Used);
        assert!(again.is_empty(), "{keep}: nothing is used between two forgettings");
        let stats = table.take_stats();
        assert_eq!((stats.forgettings, stats.size_before, stats.size_kept), (1, 4, kept.len() as u64), "{keep}");
        assert_eq!((stats.learned, stats.evicted, stats.largest), (4, 0, 4), "{keep}");
        assert_eq!(table.take_stats().largest, kept.len() as u64, "the largest table restarts from the table now");
    }
    assert_eq!(Keep::parse("top64"), Some(Keep::Top(64)));
    for keep in [Keep::Nothing, Keep::Used, Keep::Short(3), Keep::Top(256)] {
        assert_eq!(Keep::parse(&keep.to_string()), Some(keep));
    }
    for bad in ["", "all", "size", "size0", "top-1", "top 4", "hit3", "sizes3", "TOP4"] {
        assert_eq!(Keep::parse(bad), None, "{bad:?}");
    }
}

#[test]
fn nogood_table_with_a_limit_drops_the_least_recently_used() {
    let set = |xs: &[u32]| Bits::<1>::from_slice(xs);
    let mut table = Nogoods::<1>::new(60, 100);
    table.set_limit(Some(2));
    table.insert(set(&[1, 2]));
    table.insert(set(&[3, 4]));
    // Using the first makes the second the least recently used.
    assert_eq!(table.conflict(&set(&[1]), &set(&[2])), Some(set(&[2])));
    table.insert(set(&[5, 6]));
    assert_eq!(table.sets(), [set(&[1, 2]), set(&[5, 6])]);
    // With no use since, the first learned goes.
    table.insert(set(&[7, 8]));
    assert_eq!(table.sets(), [set(&[5, 6]), set(&[7, 8])]);
    // What a rule-out used counts as used too.
    assert_eq!(table.ruled_out(&set(&[5]), &set(&[5]), &set(&[6])), set(&[6]));
    table.insert(set(&[9, 10]));
    assert_eq!(table.sets(), [set(&[5, 6]), set(&[9, 10])]);
    let stats = table.take_stats();
    assert_eq!((stats.learned, stats.evicted, stats.largest), (5, 3, 2));
    // The lookups still follow the index of the table that was rebuilt.
    assert_eq!(table.conflict(&set(&[9]), &set(&[10, 5, 6])), Some(set(&[10])));
    assert_eq!(table.conflict(&set(&[]), &set(&[1, 2, 3, 4, 7, 8])), None);
    // A limit of 1 holds the newest.
    table.set_limit(Some(1));
    table.insert(set(&[20, 21]));
    assert_eq!(table.sets(), [set(&[20, 21])]);
    // Without a limit the table grows again.
    table.set_limit(None);
    table.insert(set(&[22, 23]));
    assert_eq!(table.len(), 2);
}

/// Under every policy the table answers as its definition says over the nogoods it holds, however it has been grown,
/// forgotten from and limited, and a forgetting keeps a subsequence of what it held (in the order learned), at most
/// `k` of them for `Top(k)`, exactly the short ones for `Short(s)`.
#[test]
fn nogood_table_answers_by_definition_under_every_policy() {
    let mut rng = Rng(0x0DD5_77AA_0042_0001);
    let keeps = [Keep::Nothing, Keep::Used, Keep::Short(1), Keep::Short(3), Keep::Top(1), Keep::Top(5)];
    let (mut forgot, mut full, mut found) = (0, 0, 0);
    for round in 0..30 {
        let mut table = Nogoods::<4>::new(255, 1 << 12);
        let limit = (round % 2 == 0).then(|| 1 + rng.below(12) as usize);
        table.set_limit(limit);
        for _ in 0..300 {
            let size = 1 + rng.below(4);
            let n: Bits<4> =
                (0..size).map(|_| if rng.chance(70) { rng.below(24) } else { rng.below(250) } as u32).collect();
            table.insert(n);
            if let Some(limit) = limit {
                assert!(table.len() <= limit, "{} nogoods under a limit of {limit}", table.len());
                full += u32::from(table.len() == limit);
            }
            let held = table.sets().to_vec();
            assert!(held.contains(&n), "the newest nogood is held");
            for _ in 0..3 {
                let crowd = Bits::from_range(0, 23) | (0..20).map(|_| rng.below(250) as u32).collect();
                let promoted = rng.subset(&crowd, 10);
                let set = rng.subset(&crowd, 60) - promoted;
                let musts = rng.subset(&set, 15);
                let expected = lookup_by_definition(&held, &promoted, &set, &musts);
                assert_eq!(table.lookup(&promoted, &set, &musts), expected, "{held:?} {promoted:?} {set:?}");
                found += u32::from(expected != Lookup::None);
                let added = rng.subset(&promoted, 40);
                let out = ruled_out_by_definition(&held, &added, &promoted, &set);
                assert_eq!(table.ruled_out(&added, &promoted, &set), out, "{held:?} {added:?} {promoted:?}");
            }
            if rng.chance(10) {
                let keep = keeps[rng.below(keeps.len() as u64) as usize];
                table.forget(keep);
                let kept = table.sets();
                let mut rest = held.iter();
                assert!(kept.iter().all(|k| rest.any(|h| h == k)), "{keep}: {kept:?} is not a subsequence of {held:?}");
                match keep {
                    Keep::Nothing => assert!(kept.is_empty()),
                    Keep::Short(s) => {
                        let short: Vec<_> = held.iter().filter(|h| h.len() <= s).copied().collect();
                        assert_eq!(kept, short, "{keep}");
                    }
                    Keep::Top(k) => assert!(kept.len() <= k as usize, "{keep}: {} kept", kept.len()),
                    Keep::Used => {}
                }
                forgot += 1;
            }
        }
    }
    assert!(forgot > 200 && full > 500 && found > 2000, "{forgot} forgettings, {full} full, {found} found");
}

#[test]
fn worklist_pops_in_descending_order() {
    let set = Bits::<3>::from_slice(&[0, 5, 63, 64, 100, 191]);
    let mut w = Worklist::of(&set);
    w.insert(7);
    let mut popped = Vec::new();
    while let Some(x) = w.pop() {
        popped.push(x);
    }
    assert_eq!(popped, vec![191, 100, 64, 63, 7, 5, 0]);
}

/// At 128 words the summary has two words (words 0 to 63 of the set in the first, 64 to 127 in the second): members on
/// both sides of the boundary (4095 and 4096) and at both ends pop in descending order, and a worklist whose second
/// summary word has emptied still pops from the first.
#[test]
fn worklist_pops_across_both_summary_words() {
    let members = [0, 5, 4095, 4096, 4100, 6000, 8191];
    let mut w = Worklist::<128>::of(&Bits::<128>::from_slice(&members));
    w.insert(63);
    w.insert(4097);
    let mut popped = Vec::new();
    while let Some(x) = w.pop() {
        popped.push(x);
    }
    assert_eq!(popped, vec![8191, 6000, 4100, 4097, 4096, 4095, 63, 5, 0]);
    w.insert(300);
    assert_eq!((w.pop(), w.pop()), (Some(300), None));
}

#[test]
fn provider_lists_match_the_java() {
    let t = Tables::new(100);
    let p = FactorProvider::new(&t, &[2, 3, 6, 12, 18, 36, 50, 97]);
    assert_eq!(p.factors(36), &[12, 18]);
    assert_eq!(p.factors(6), &[2, 3]);
    assert_eq!(p.composites(6), &[12, 18]);
    assert_eq!(p.composites(2), &[6]);
    assert!(p.factors(97).is_empty(), "1 is not among the numbers");
    assert!(p.factors(1000).is_empty() && p.composites(1000).is_empty());
}

// ---- one-sided candidates ----

/// One-sided candidates for the frames of the board 85 (`columns_85`), each with its own frame: 18 (maximal factors 6
/// and 9, both factors of frame 0), 44 and 52 (4 alone among the factors of frame 0), all three of frame 0; 88 (8 alone)
/// of frame 1 and 176 (16 alone) of frame 2.  None is a move or a factor of those columns, and the last two are beyond
/// the board 85, so these models are built on the tables for 200.
const ONE_SIDED_85: [(u32, usize); 5] = [(18, 0), (44, 0), (52, 0), (88, 1), (176, 2)];

/// The model of the columns of 85 with [`ONE_SIDED_85`] (given in an order other than ascending), its root and its
/// tables.
fn model_with_one_sided_85() -> (FrameGraph, FrameSet<4>, Tables) {
    let tables = Tables::new(200);
    let scrambled = [(176, 2), (44, 0), (18, 0), (88, 1), (52, 0)];
    let (model, root) = FrameGraph::build_extended::<4>(&tables, &columns_85(), &scrambled).unwrap();
    (model, root, tables)
}

/// `build_extended` refuses a one-sided candidate that cannot be one, and names it: a frame that does not exist, a
/// number that is a move or a factor of a frame, a number listed twice, and a number beyond the tables or the set
/// width (as it does any number of the columns).
#[test]
fn build_extended_refuses_what_cannot_be_a_one_sided_candidate() {
    let tables = Tables::new(200);
    let columns = columns_85();
    type Listed<'a> = &'a [(u32, usize)];
    let refused = |one_sided: Listed| FrameGraph::build_extended::<4>(&tables, &columns, one_sided).unwrap_err();
    let cases: [(Listed, u32, &str); 11] = [
        (&[(52, 3)], 52, "a frame past the last of three"),
        (&[(18, 0), (52, 99)], 52, "a frame far beyond the last"),
        (&[(45, 0)], 45, "a move of frame 0"),
        (&[(84, 1)], 84, "a move of frame 1"),
        (&[(80, 0)], 80, "a move of frame 2, listed with the frame 0"),
        (&[(25, 0)], 25, "a factor of frame 0"),
        (&[(42, 2)], 42, "a factor of frame 1, listed with the frame 2"),
        (&[(36, 2)], 36, "a factor of frame 2"),
        (&[(44, 0), (44, 0)], 44, "listed twice with its frame"),
        (&[(44, 0), (88, 1), (44, 2)], 44, "listed twice with two frames"),
        (&[(18, 0), (44, 0), (88, 1), (44, 0)], 44, "listed twice, not next to each other"),
    ];
    for (one_sided, number, why) in cases {
        let message = refused(one_sided);
        assert!(message.contains(&number.to_string()), "{why}: the message {message:?} does not name {number}");
    }
    // A number past the tables, and one past the width of the sets: Bits<2> holds up to 127.
    let message = FrameGraph::build_extended::<4>(&Tables::new(85), &columns, &[(86, 0)]).unwrap_err();
    assert!(message.contains("86"), "{message}");
    let message = FrameGraph::build_extended::<2>(&tables, &columns, &[(150, 0)]).unwrap_err();
    assert!(message.contains("150"), "{message}");
    // The same candidates, once they are ones, are accepted.
    assert!(FrameGraph::build_extended::<4>(&tables, &columns, &[(44, 0), (88, 1), (176, 2)]).is_ok());
}

/// With no one-sided candidate `build_extended` is `build`: the same root, home frames, and factor lists.
#[test]
fn build_extended_with_no_one_sided_candidate_is_build() {
    let tables = Tables::new(85);
    let (plain, plain_root) = FrameGraph::build::<2>(&tables, &columns_85()).unwrap();
    let (model, root) = FrameGraph::build_extended::<2>(&tables, &columns_85(), &[]).unwrap();
    assert_eq!(root, plain_root);
    assert!(!model.has_one_sided() && model.one_sided().is_empty());
    for x in 0..=85 {
        assert_eq!(model.home(x), plain.home(x), "home of {x}");
        assert_eq!(model.own(x), None, "own frame of {x}");
        assert_eq!(model.graph().factors(x), plain.graph().factors(x), "factors of {x}");
        assert_eq!(model.graph().composites(x), plain.graph().composites(x), "composites of {x}");
    }
}

/// A one-sided candidate has its own frame and no home, touches that frame only, and is paid for by its maximal
/// factors that are factors of that frame; a two-sided candidate touches its home and the frame in front.  The root
/// and the home frames are those of the model without them, and the factor lists add the one-sided candidates as
/// composites of those factors.
#[test]
fn one_sided_candidates_have_their_own_frames() {
    let (model, root, tables) = model_with_one_sided_85();
    let columns = columns_85();
    let (plain, plain_root) = FrameGraph::build::<4>(&tables, &columns).unwrap();
    assert_eq!(root, plain_root);
    assert!(model.has_one_sided() && !plain.has_one_sided());
    assert_eq!(model.one_sided(), ONE_SIDED_85, "ascending");
    for (s, j) in ONE_SIDED_85 {
        assert_eq!((model.own(s), model.home(s), model.frames_touched(s)), (Some(j), 0, (j, None)), "{s}");
        let payers: Vec<u16> = tables.max_factors[s as usize]
            .iter()
            .filter(|&&f| columns[2 * j].contains(&f))
            .map(|&f| f as u16)
            .collect();
        assert!(!payers.is_empty(), "{s} has no maximal factor in the frame {j}");
        assert_eq!(model.graph().factors(s), payers, "factors of {s}");
        assert!(payers.iter().all(|&f| model.graph().composites(u32::from(f)).contains(&(s as u16))), "{s}");
        assert!(!model.contains_factor(&root, s), "{s} is no factor of its home");
    }
    for c in CANDIDATES_85 {
        let h = model.home(c);
        assert_eq!(h, plain.home(c), "home of {c}");
        assert_eq!((model.own(c), model.frames_touched(c)), (None, (h - 1, Some(h))), "{c}");
    }
    for x in (0..=85).filter(|x| !ONE_SIDED_85.iter().any(|&(s, _)| s == *x)) {
        assert_eq!((model.own(x), model.home(x)), (None, plain.home(x)), "{x}");
    }
    assert_eq!(model.own(999), None);
    // The numbers each one-sided candidate is a composite of, and no others: 4 pays for 44 and 52, 6 and 9 for 18.
    let composites = |f: u32| model.graph().composites(f).to_vec();
    assert!(composites(4).contains(&44) && composites(4).contains(&52) && !composites(4).contains(&18));
    assert!(composites(6).contains(&18) && composites(9).contains(&18) && !composites(10).contains(&18));
}

/// Promoting a one-sided candidate adds it to the moves of its own frame, removes no factor, changes no other frame
/// and spends a freedom of that frame, however it is done: `promote`, `promote_in_place_using`, `promote_all_using`.
/// What the reduction after it then drops, a move with the factor that pays for it, leaves the freedoms the factors
/// less the moves.  `edits` puts it in its own frame's gains and nowhere else.
#[test]
fn promoting_a_one_sided_candidate_changes_only_its_own_frame() {
    let (model, root, _) = model_with_one_sided_85();
    let mut scratch = Scratch::<4>::new();
    for (s, j) in ONE_SIDED_85 {
        assert!(model.can_promote(&root, s), "{s}");
        let state = model.promote(&root, s).expect("promotable");
        let mut in_place = root;
        assert!(model.promote_in_place_using(&mut scratch, &mut in_place, s), "{s}");
        assert_eq!(in_place, state, "{s}: in place");
        assert_eq!(model.promote_all_using(&mut scratch, &root, &bits(&[s])), Some(state), "{s}: all at once");
        for (i, (before, after)) in root.frames().iter().zip(state.frames()).enumerate() {
            assert!(i == j || before == after, "{s}: frame {i} changed");
        }
        let (before, after) = (root.frames()[j], state.frames()[j]);
        let dropped = before.factors.len() - after.factors.len();
        assert!(after.factors.is_subset(&before.factors), "{s}: factors");
        assert!(after.moves.is_subset(&before.moves.with(s)), "{s}: moves");
        assert_eq!(after.moves.len() + dropped, before.moves.len() + 1, "{s}: a move is dropped with each factor");
        assert_eq!(after.freedoms, before.freedoms - 1, "{s}: freedoms");
        assert_eq!(after.freedoms, after.factors.len() as i32 - after.moves.len() as i32, "{s}: freedoms and sizes");
    }
    // 18 has two live factors in frame 0 and stays a move; 44 has only 4, and is dropped with it.
    let frame0 = root.frames()[0];
    let after_18 = model.promote(&root, 18).unwrap().frames()[0];
    assert_eq!((after_18.moves, after_18.factors), (frame0.moves.with(18), frame0.factors));
    let after_44 = model.promote(&root, 44).unwrap().frames()[0];
    assert_eq!((after_44.moves, after_44.factors), (frame0.moves, frame0.factors.without(4)));

    // 88 into frame 1, 18 into frame 0 and 42 (a factor of frame 1) into frame 0 out of frame 1.
    let (edits, none) = model.edits(&root, &bits(&[88, 18, 42]));
    assert_eq!(none, None);
    let gained: Vec<Vec<u32>> = edits.gained.iter().map(Bits::to_vec).collect();
    let lost: Vec<Vec<u32>> = edits.lost.iter().map(Bits::to_vec).collect();
    assert_eq!(gained[..3], [vec![18, 42], vec![88], vec![]]);
    assert_eq!(lost[..3], [vec![], vec![42], vec![]]);
}

/// One-sided candidates of one frame compete for its factors as the moves of that frame do: 44 and 52 are paid for by
/// 4 alone, so only one of them can be promoted, in either order, and the conflict and the nogood say so.
#[test]
fn one_sided_candidates_compete_for_the_factors_of_their_frame() {
    let (model, root, _) = model_with_one_sided_85();
    let mut scratch = Scratch::<4>::new();
    let mut can = |from: &FrameSet<4>, xs: &[u32]| model.can_promote_all_using(&mut scratch, from, &bits(xs));
    assert!(can(&root, &[44]) && can(&root, &[52]) && !can(&root, &[44, 52]));
    assert!(can(&root, &[18, 44]) && can(&root, &[18, 52]) && !can(&root, &[18, 44, 52]));
    assert!(can(&root, &[18, 44, 88, 176]), "one in each frame, and 18 and 44 pay with different factors");
    for (first, second) in [(44, 52), (52, 44)] {
        let state = model.promote(&root, first).unwrap();
        assert!(!model.can_promote(&state, second), "{second} after {first}");
    }

    let both = bits(&[44, 52]);
    for minimal in [false, true] {
        let conflict = model.conflict_using(&mut scratch, &root, &bits(&[18, 44, 52, 88]), minimal);
        assert_eq!(conflict, Some(both), "minimal = {minimal}");
    }
    let after_18 = model.promote(&root, 18).unwrap();
    assert_eq!(model.conflict_using(&mut scratch, &after_18, &bits(&[44, 52, 176]), true), Some(both));
    assert_eq!(model.nogood(&mut scratch, &root, &bits(&[18]), &both), Some(both));
    assert!(model.conflict_using(&mut scratch, &root, &bits(&[18, 44, 88, 176]), true).is_none());
}

/// Whether every frame of `columns` is winnable by exhaustion with the members of `set` promoted at once (`math` pairs
/// every number with its maximal factors, and the exhaustion keeps those among the frame's factors), where the
/// one-sided members of `one_sided` are added to the moves of their own frames and any other member is moved from the
/// factors of its home frame to the moves of the frame in front (none if it has no home, or the home is frame 0).
fn winnable_by_exhaustion(
    math: &FactorProvider,
    columns: &[Vec<u32>],
    one_sided: &[(u32, usize)],
    set: &[u32],
) -> bool {
    let frames = columns.len() / 2;
    let mut moves: Vec<Bits<4>> = (0..frames).map(|i| bits(&columns[2 * i + 1])).collect();
    let mut factors: Vec<Bits<4>> = (0..frames).map(|i| bits(&columns[2 * i])).collect();
    for &x in set {
        if let Some(&(_, own)) = one_sided.iter().find(|&&(s, _)| s == x) {
            moves[own].insert(x);
        } else {
            match (0..frames).find(|&h| columns[2 * h].contains(&x)) {
                Some(home) if home >= 1 => {
                    moves[home - 1].insert(x);
                    factors[home].remove(x);
                }
                _ => return false,
            }
        }
    }
    (0..frames).all(|i| tax_winnable(moves[i], factors[i], math))
}

/// A set of the candidates of 85 and the one-sided ones can be promoted exactly when every frame is winnable with it
/// applied, as the game defines winnable (some order of the moves in which each still has a factor present when it is
/// taken), found by exhaustion for every set of up to six of the fifteen numbers, and that is the same one at a time.
#[test]
fn one_sided_sets_are_promotable_when_every_frame_is_winnable_by_exhaustion() {
    let (model, root, tables) = model_with_one_sided_85();
    let columns = columns_85();
    let numbers: Vec<u32> = CANDIDATES_85.iter().copied().chain(ONE_SIDED_85.iter().map(|&(s, _)| s)).collect();
    let math = FactorProvider::new(&tables, &(1..=tables.n).collect::<Vec<_>>());
    let mut scratch = Scratch::<4>::new();
    let (mut sets, mut promotable, mut mixed) = (0, 0, 0);
    for mask in 0u32..1 << numbers.len() {
        if mask.count_ones() > 6 {
            continue;
        }
        let set: Vec<u32> = (0..numbers.len()).filter(|&i| mask >> i & 1 == 1).map(|i| numbers[i]).collect();
        let expected = winnable_by_exhaustion(&math, &columns, &ONE_SIDED_85, &set);
        let at_once = model.can_promote_all_using(&mut scratch, &root, &bits(&set));
        let all = model.promote_all_using(&mut scratch, &root, &bits(&set));
        let mut state = root;
        let one_by_one = set.iter().rev().all(|&x| model.promote_in_place_using(&mut scratch, &mut state, x));
        assert_eq!(at_once, expected, "all at once: {set:?}");
        assert_eq!(all.is_some(), expected, "promote_all_using: {set:?}");
        assert_eq!(one_by_one, expected, "one by one: {set:?}");
        if expected {
            // The two ways reach the same state, whose freedoms are the factors less the moves.
            assert_eq!(all, Some(state), "the state after {set:?}");
            for frame in state.frames() {
                let sizes = frame.factors.len() as i32 - frame.moves.len() as i32;
                assert_eq!(frame.freedoms, sizes, "freedoms after {set:?}");
            }
        }
        sets += 1;
        promotable += usize::from(expected);
        let one_sided_members = set.iter().filter(|&&x| model.own(x).is_some()).count();
        mixed += usize::from(expected && 0 < one_sided_members && one_sided_members < set.len());
    }
    eprintln!("one-sided sets of 85: {sets} sets, {promotable} promotable, {mixed} of them with both kinds of member");
    assert!(sets > 9_000 && promotable > 400 && promotable < sets - 400, "{sets} sets, {promotable} promotable");
    assert!(mixed > 200, "only {mixed} promotable sets have members of both kinds");
}

/// Each one-sided candidate in the cands is one more promotion for the count beside what the freedoms allow the
/// two-sided ones, and `narrow` keeps a one-sided candidate while its own frame has a freedom left: 176 spends the one
/// freedom of frame 2, which takes the factors of frame 2 and 176 itself out of the narrowing.
#[test]
fn one_sided_candidates_count_in_the_narrowing_and_the_bound() {
    let (model, root, _) = model_with_one_sided_85();
    let cands = bits::<4>(&CANDIDATES_85) | bits(&[18, 44, 52, 88, 176]);
    assert_eq!(model.narrow(&root, &cands), cands);
    assert_eq!(model.max_promotions_among(&root, &bits(&CANDIDATES_85)), 3);
    assert_eq!(model.max_promotions_among(&root, &cands), 3 + 5);
    assert_eq!(model.max_promotions_among(&root, &bits(&[18, 44])), 2, "no two-sided candidate among them");

    let state = model.promote(&root, 176).unwrap();
    assert_eq!(state.frames().iter().map(|frame| frame.freedoms).collect::<Vec<_>>(), [4, 3, 0]);
    assert_eq!(model.narrow(&state, &cands).to_vec(), [8, 12, 18, 20, 28, 30, 42, 44, 52, 88]);
    assert_eq!(model.max_promotions_among(&state, &cands.without(176)), 3 + 4);
}
