//! The checks of the near-repeat cut on constructed problems, each failing alone where it must, and holding where the
//! problems agree; the trap of a number that Π_M's model promotes but that is no candidate of Π_M (22 at n = 98); and
//! the lemma's conclusion at random nodes of small boards, checked by the model's own promotability.

use super::*;
use crate::board::Board;
use crate::model::test_support::{best_completion, Draws, Rng};
use crate::numtheory::Tables;
use crate::search::SearchTactics;

const W: usize = 8;

/// A problem of the frame model: its model, root and candidates.
struct Problem {
    model: Arc<FrameGraph>,
    root: FrameSet<W>,
    candidates: Bits<W>,
}

/// The problem built from `frames`, each its factors and moves, front first, with the candidates `candidates`, on the
/// tables of the board 100.  Every frame of its root must be winnable.
fn problem(frames: &[(&[u32], &[u32])], candidates: &[u32]) -> Problem {
    let columns: Vec<Vec<u32>> =
        frames.iter().flat_map(|(factors, moves)| [factors.to_vec(), moves.to_vec()]).collect();
    let (model, root) = FrameGraph::build::<W>(&Tables::new(100), &columns).unwrap();
    assert!(root.frames().iter().all(|f| model.winnable(&f.moves, &f.factors)), "{frames:?}: a frame is not winnable");
    Problem { model: Arc::new(model), root, candidates: Bits::from_slice(candidates) }
}

/// Whether the cut of a search of `n` by `m` cuts the node with the promotions `promoted` (a promotion set of `n`)
/// and the candidates `cands`.
fn cuts(n: &Problem, m: &Problem, promoted: &[u32], cands: &[u32]) -> bool {
    let near = NearRepeat::of(&n.model, &n.candidates, Arc::clone(&m.model), m.root, &m.candidates, 0).unwrap();
    let (promoted, scratch) = (Bits::from_slice(promoted), &mut Scratch::new());
    let state = n.model.promote_all_using(scratch, &n.root, &promoted).expect("a promotion set of N");
    near.cuts(scratch, &n.model, &state, &promoted, &Bits::from_slice(cands))
}

/// Whether `set` is a promotion set of `p`: made of its candidates, and promotable from its root.
fn promotes(p: &Problem, set: &[u32]) -> bool {
    let set = Bits::from_slice(set);
    set.is_subset(&p.candidates) && p.model.promote_all_using(&mut Scratch::new(), &p.root, &set).is_some()
}

/// The problem the cases change one thing of: frame 0 has the factors 6, 10 and 15 and the move 30 (paid by any of
/// them), and frame 1 the factors 18, 20 and 45, the candidates, which frame 0 can pay for with 6, 10 and 15 (their
/// maximal factors are 9 and 6, 10 and 4, 15 and 9).  Frame 0 has two freedoms: its promotion sets are the pairs of
/// candidates and the single ones, and the best is {20, 45}.
const FRAME_0: (&[u32], &[u32]) = (&[6, 10, 15], &[30]);
const FRAME_1: (&[u32], &[u32]) = (&[18, 20, 45], &[]);
const CANDIDATES: [u32; 3] = [18, 20, 45];

fn base() -> Problem {
    problem(&[FRAME_0, FRAME_1], &CANDIDATES)
}

#[test]
fn the_check_holds_where_the_problems_are_the_same() {
    let (n, m) = (base(), base());
    assert_eq!(best_completion(&n.model, &n.root, &n.candidates, 3), 65);
    assert!(cuts(&n, &m, &[], &CANDIDATES));
    assert!(cuts(&n, &m, &[20], &[18, 45]));
    assert!(cuts(&n, &m, &[20, 45], &[]));
}

/// Check 1: a candidate of N that is no candidate of M is not cut, though M's model would promote it; the node is cut
/// once its promotions and candidates leave it out.
#[test]
fn a_number_that_is_no_candidate_of_m_is_not_cut() {
    let (n, m) = (base(), problem(&[FRAME_0, FRAME_1], &[20, 45]));
    let scratch = &mut Scratch::new();
    assert!(m.model.promote_all_using(scratch, &m.root, &Bits::from_slice(&[18])).is_some(), "M's model takes 18");
    assert!(!promotes(&m, &[18]) && promotes(&n, &[18]));
    assert!(!cuts(&n, &m, &[], &CANDIDATES));
    assert!(!cuts(&n, &m, &[18], &[20]));
    assert!(cuts(&n, &m, &[], &[20, 45]));
    assert!(cuts(&n, &m, &[20], &[45]));
}

/// The trap of the ledger's "Why each check is needed": at n = 98 the model promotes 22, a factor of frame 1, which
/// only the candidate filter keeps out of the candidates (docs/theory/frame-model.md, the paragraph "Why the candidate
/// filter matters").  A problem N that has 22 among its candidates, and is the board's otherwise, is not cut by the
/// board's problem while 22 is a candidate of the node, though the other checks hold: a check that took the model's
/// promotability for candidacy would cut it.
#[test]
fn a_number_that_the_model_of_98_promotes_but_is_no_candidate_is_not_cut() {
    let p = Board::<W>::for_n(98).prepare(SearchTactics::default()).unwrap();
    let scratch = &mut Scratch::new();
    assert!(!p.candidates.contains(22) && p.model.home(22) >= 1 && p.model.contains_factor(&p.root, 22));
    assert!(p.model.promote_all_using(scratch, &p.root, &Bits::from_slice(&[22])).is_some(), "the model takes 22");
    let with = |candidates| Problem { model: Arc::clone(&p.model), root: p.root, candidates };
    let (m, n) = (with(p.candidates), with(p.candidates.with(22)));
    assert!(!cuts(&n, &m, &[], &n.candidates.to_vec()));
    assert!(!cuts(&n, &m, &[22], &m.candidates.to_vec()));
    assert!(cuts(&n, &m, &[], &m.candidates.to_vec()));
    // Checks 2 and 3 hold where 22 is a candidate: only candidacy keeps the node.
    assert!(cuts(&n, &n, &[], &n.candidates.to_vec()));
}

/// Check 1: a candidate with another home in M is not cut.
#[test]
fn a_candidate_with_another_home_in_m_is_not_cut() {
    let n = base();
    let m = problem(&[FRAME_0, (&[20, 45], &[]), (&[18], &[])], &CANDIDATES);
    assert_eq!((n.model.home(18), m.model.home(18)), (1, 2));
    assert!(!cuts(&n, &m, &[], &CANDIDATES));
    assert!(!cuts(&n, &m, &[], &[18]));
    // N's frame 1 has the factor 18 that M's lacks, but it pays for nothing there.
    assert!(cuts(&n, &m, &[], &[20, 45]));
}

/// Check 2: promotions that M cannot promote are not cut, though the node has no candidates left.  Here frame 0 of M
/// lacks the factor 15: {18, 20} is a promotion set of N, worth 38, and not of M, whose best is 20.
#[test]
fn promotions_that_m_cannot_promote_are_not_cut() {
    let (n, m) = (base(), problem(&[(&[6, 10], &[30]), FRAME_1], &CANDIDATES));
    assert_eq!(best_completion(&m.model, &m.root, &m.candidates, 3), 20);
    assert!(promotes(&n, &[18, 20]) && !promotes(&m, &[18, 20]));
    assert!(!cuts(&n, &m, &[18, 20], &[]));
    assert!(cuts(&n, &n, &[18, 20], &[]));
}

/// Check 3, the moves: a frame of M with a move that N's lacks is not cut, and with the problems the other way round
/// (N's frame harder by a move) it is.
#[test]
fn a_frame_of_m_with_a_move_that_n_lacks_is_not_cut() {
    // 42 (maximal factors 21, 14 and 6) is paid by 21 or 6.
    let easier = problem(&[(&[6, 10, 15, 21], &[30]), FRAME_1], &CANDIDATES);
    let harder = problem(&[(&[6, 10, 15, 21], &[30, 42]), FRAME_1], &CANDIDATES);
    assert!(!cuts(&easier, &harder, &[], &CANDIDATES));
    assert!(!cuts(&easier, &harder, &[], &[20]));
    assert!(cuts(&harder, &easier, &[], &CANDIDATES));
    // A frame that no candidate touches is not compared: none is, with no candidates.
    assert!(cuts(&easier, &harder, &[], &[]));
}

/// Check 3, the factors: a factor of N's frame that M's lacks is not cut where it pays for a move of M's frame (15
/// for 30), or for a candidate promoted into the frame (9 for 18 and 45), or is a candidate itself (18, which M's
/// frame 1 lost to the move 90 when 45 was promoted); where it pays for none of them, the node is cut.
#[test]
fn a_factor_of_n_that_m_lacks_is_not_cut_where_it_can_pay() {
    let n = base();
    // 15 pays for the move 30 of M's frame 0.  With 18 and 20 the trap is real: {18, 20} is a promotion set of N, and
    // not of M (`promotions_that_m_cannot_promote_are_not_cut`).
    let m = problem(&[(&[6, 10], &[30]), FRAME_1], &CANDIDATES);
    assert!(!cuts(&n, &m, &[], &[20]));
    assert!(!cuts(&n, &m, &[], &[18, 20]));

    // 9 pays for the candidates 18 and 45 of frame 1, and for nothing else.
    let n9 = problem(&[(&[6, 9, 10, 15], &[30]), FRAME_1], &CANDIDATES);
    assert!(!cuts(&n9, &n, &[], &[18]));
    assert!(!cuts(&n9, &n, &[], &[20, 45]));
    assert!(cuts(&n9, &n, &[], &[20]));

    // M's frame 1 has the move 90 (maximal factors 45, 30 and 18), paid by 45 or 18: promoting 45 leaves it paid by
    // 18, and the reduction drops both, so that 18 is a factor of N's frame 1 and not of M's.
    let m90 = problem(&[FRAME_0, (&[18, 20, 45], &[90])], &CANDIDATES);
    assert!(!cuts(&n, &m90, &[], &CANDIDATES), "the move 90 of M's frame 1");
    assert!(promotes(&n, &[45, 18]) && !promotes(&m90, &[45, 18]));
    assert!(!cuts(&n, &m90, &[45], &[18, 20]));
    assert!(cuts(&n, &m90, &[45], &[20]));
}

/// The lemma's conclusion at random nodes of the boards 30 to 160, each searched with the cut by each of the three
/// boards before it: wherever the checks hold, every completion of the node that is a promotion set of Π_N, as the
/// model's own promotability and the candidates say, is one of Π_M.  The nodes promote a few candidates of both
/// problems from the root, one after another, and take up to 8 of the candidates of both that can still be promoted.
/// Some of the nodes where the checks hold have states that differ in the two problems.
#[test]
fn where_the_checks_hold_every_completion_promotable_in_n_is_promotable_in_m() {
    let mut rng = Rng(0x4EA2_5E7E_0000_0001);
    let (scratch, tactics) = (&mut Scratch::new(), SearchTactics::default());
    let (mut fired, mut apart, mut completions) = (0, 0, 0);
    for (n, m) in (30..=160).flat_map(|n| (1..=3).map(move |d| (n, n - d))) {
        let (board_n, board_m) = (Board::<W>::for_n(n), Board::<W>::for_n(m));
        if board_n.is_trivial() || board_m.is_trivial() {
            continue;
        }
        let (pn, pm) = (board_n.prepare(tactics).unwrap(), board_m.prepare(tactics).unwrap());
        let near = NearRepeat::new(&pn, &pm, 0).unwrap();
        let both = pn.candidates & pm.candidates;
        for _ in 0..30 {
            let (mut state, mut promoted) = (pn.root, Bits::EMPTY);
            let mut order = both.to_vec();
            rng.shuffle(&mut order);
            for &x in order.iter().take(rng.below(6) as usize) {
                if pn.model.promote_in_place_using(scratch, &mut state, x) {
                    promoted.insert(x);
                }
            }
            let mut left = pn.model.narrow(&state, &(both - promoted)).to_vec();
            rng.shuffle(&mut left);
            let cands: Vec<u32> = left.into_iter().take(1 + rng.below(8) as usize).collect();
            if !near.cuts(scratch, &pn.model, &state, &promoted, &Bits::from_slice(&cands)) {
                continue;
            }
            fired += 1;
            let state_m = pm.model.promote_all_using(scratch, &pm.root, &promoted).expect("check 2");
            apart += usize::from(state_m.frames() != state.frames());
            for subset in 0u32..1 << cands.len() {
                let added: Bits<W> = (0..cands.len()).filter(|i| subset >> i & 1 == 1).map(|i| cands[i]).collect();
                let set = promoted | added;
                if pn.model.promote_all_using(scratch, &pn.root, &set).is_none() {
                    continue;
                }
                completions += 1;
                let in_m =
                    set.is_subset(&pm.candidates) && pm.model.promote_all_using(scratch, &pm.root, &set).is_some();
                assert!(in_m, "n={n}, m={m}: {set:?}, a completion of {promoted:?} with {cands:?}, is not one of M");
            }
        }
    }
    let counts = format!("{fired} nodes where the checks hold, {apart} with states apart, {completions} completions");
    assert!(fired >= 3000 && apart >= 1000 && completions >= 20_000, "{counts}");
}
