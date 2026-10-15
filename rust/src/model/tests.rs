use super::peel::{reduce_from, winnable, Worklist};
use super::test_support::*;
use super::*;
use crate::json::Json;

fn bits<const W: usize>(xs: &[u32]) -> Bits<W> {
    Bits::from_slice(xs)
}

// ---- the Java's algorithms, transliterated, as the reference ----

/// `Frame.isWinnable`, step for step.
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

/// `Frame.reduce(movesToKeep, factorsToKeep, math)`, step for step (with Java assertions off).
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
fn fast_peeling_and_reduction_agree_with_reference() {
    let mut rng = Rng(0x5EED_1234_ABCD_0001);
    let mut stats = PeelStats::default();
    for n in golden_ns().into_iter().filter(|&n| n >= 20 && (n % 18 == 0 || n % 120 == 0)) {
        let g = load(n);
        if n < 512 {
            random_agreement::<8>(&g, &mut rng, &mut stats);
        } else {
            random_agreement::<16>(&g, &mut rng, &mut stats);
        }
    }
    eprintln!("peel agreement: {stats:?}");
    assert!(stats.checks > 10_000 && stats.winnable > 1_000);
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
