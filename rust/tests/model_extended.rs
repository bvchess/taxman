//! Theorem M⁺, the extended problem Π⁺(O), against the code.
//!
//! Π⁺(O) (`Board::extended`, which calls `taxman::frames::build_extended`, whose module documentation defines it) is
//! the frame model with a set O of the sources of three or more prime factors that setup keeps made optional, and with
//! a reference predicate for its promotion sets, `ExtendedFrames::promotable`, that peels the unreduced frames.  It
//! comes in two formulations, both checked here: without the claims of `unzip` (nothing settled by it), and with the
//! claims that Lemma M5⁺ allows (heuristics/THEORY.md; with O empty that is the frame model's own problem Π_n).  The
//! tests with `with_claims` in their names have the claims, the others none.  The theorem (heuristics/THEORY.md, "The
//! extended problem"):
//!
//! > Let U\* be the sources that setup keeps (the numbers above n/2 of the game with no promotions, `board.plain`),
//! > O ⊆ U\*_{≥3} the open sources and F = U\* ∖ O.  For every S ⊆ {2, …, n} with S ⊇ F, no prime up to n/2 in S, and
//! > no source of layer 1 or 2 outside F: S is playable (some order of it is a legal game) if and only if (K, L) is a
//! > promotion set of Π⁺(O), where K is the sources of S outside F and L is S ∩ [2, n/2]; and then ΣS = `in_the_bag`
//! > + ΣK + ΣL.
//!
//! Playability is decided on the whole board by `common::playable` (the peel of THEORY.md, Theorems 1 to 3), with code
//! that shares nothing with `frames` or `model`; U\* is read off the game with no promotions and the layers off
//! `Tables::omega`, foundation code.  A mismatch between the oracle and the predicate is either a bug in the code or a
//! counterexample to the theorem.
//!
//! * On every board up to 80 (and, in ignored tests, to 100; with every source open the default test goes to 60 and
//!   an ignored one to 69) both directions are checked on every set, in either formulation, for O empty, for random
//!   subsets of U\*_{≥3} and for O = U\*_{≥3}: the sets X = K ∪ L with F ∪ X playable (family A, found depth first),
//!   all promotion sets (family B, found the same way), and the two families must be equal; the heaviest set weighs
//!   what the table of optimal games has for n, and the oracle's order for it is a legal game of that score.
//! * With O empty, Π⁺(∅) has the promotion sets of the frame model: those of Π_n (`Board::prepare`) and those of the
//!   problem the search is given for Π⁺(∅) (`Board::prepare_extended`).
//! * On larger boards (to 1300 by default, to 4095 in an ignored test) the sets are random, in either formulation:
//!   promotion sets made by adding candidates in a random order, and maximal playable sets made by adding numbers in a
//!   random order, for O a random 20 members of U\*_{≥3}, for O = U\*_{≥3} and for O empty, where random promotion
//!   sets of the frame model and of Π⁺(∅) are promotion sets of each other (and, with the claims, the search of Π⁺(∅)
//!   is given the candidates and `max_total` of the frame model).  Greedy rejects a source of layer 3 or more on few
//!   boards (374 at 741, the first), and only the boards of the test for them (741, 1100, 1200) put a rejected source
//!   into a set.
//! * The check of Lemma M5⁺ is needed.  On the boards to 400 it refuses the claims of a component, for some O that
//!   leaves the component free, only on the boards 24 to 26 and 343 to 349, and then for O = U\*_{≥3} but the
//!   component's sources, so no random subset meets it.  A mutant of the claims-on problem that keeps those claims
//!   anyway (it takes the refused pairs out of the frames) is compared with the true problem on every set of the boards
//!   to 80 for each such O (those of the boards 24 to 26) by `skipping_the_check_of_lemma_m5_plus_is_caught`: the true
//!   problem agrees with the oracle, the mutant does not.
//! * The model the search is given for Π⁺(O), `FrameGraph::build_extended` with the one-sided candidates, is checked
//!   against the reference predicate, without the claims, on every board from 2 to 300 (to 500 in an ignored test), for
//!   O empty, for O = U\*_{≥3} up to the board 120 and for five random subsets of it: random sets of candidates and
//!   random promotion sets must be promoted, all at once and one member at a time, exactly when the reference says so,
//!   and the unpromotable ones must have conflicts and nogoods that cannot be promoted, a narrowing that keeps what can
//!   be promoted, and a count that is no lower than the size of a promotion set.
//! * The search of Π⁺(O) (`Board::prepare_extended_with`, `search::search` on one thread) is held to the whole-board
//!   oracle, in either formulation, on every board from 2 to 80, for O empty (under the default tactics and those of
//!   extended mode), for random subsets of U\*_{≥3} and for O = U\*_{≥3} (to 60 by default, to 69 in an ignored test):
//!   it finds a promotion sum s with `in_the_bag` + s the heaviest weight of F ∪ X over the sets X of family A, which
//!   is also the table's score, and the set it returns is a promotion set of the reference predicate that F makes
//!   playable.

mod common;

use common::{
    config, for_each_kept_subset, kept_subsets, play_order, playable, random_open_sets, random_subset, shuffle, upper,
    upper_from_3, Rng,
};
use std::collections::BTreeSet;
use std::ops::RangeInclusive;
use std::time::Instant;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::frames::{BoardFrames, ClaimKind, ExtendedFrames};
use taxman::game::{self, Optimal};
use taxman::model::{self, FrameGraph, FrameSet, Scratch};
use taxman::search::{search, Prepared, SearchTactics};
use taxman::with_width;

// ---------------------------------------------------------------------------------------------------------
// A board and its extended problem

/// The sum of `numbers`.
fn sum(numbers: &[u32]) -> u64 {
    numbers.iter().map(|&x| u64::from(x)).sum()
}

/// What a formulation of Π⁺(O) is called in the messages and the lines a check prints: `claims=on` for the one with the
/// claims of `unzip` that Lemma M5⁺ allows, `claims=off` for the one without.
fn formulation(claims: bool) -> &'static str {
    if claims {
        "claims=on"
    } else {
        "claims=off"
    }
}

/// A board with what the theorem is about: F, O and the extended problem Π⁺(O).
struct Case<'b, const W: usize> {
    board: &'b Board<W>,
    /// The board's frames as setup forms them, which the reference predicate takes.
    components: BoardFrames<'b, W>,
    extended: ExtendedFrames<W>,
    /// F = U\* ∖ O, ascending.
    fixed: Vec<u32>,
    open: Vec<u32>,
    /// Whether the problem is the formulation with the claims of `unzip` that Lemma M5⁺ allows.
    claims: bool,
}

impl<'b, const W: usize> Case<'b, W> {
    fn new(board: &'b Board<W>, open: &[u32], claims: bool) -> Self {
        let n = board.n();
        let fixed = upper(board).into_iter().filter(|x| !open.contains(x)).collect();
        let extended = board
            .extended(&Bits::from_slice(open), claims)
            .unwrap_or_else(|e| panic!("n={n}: O={open:?}, {}: {e}", formulation(claims)));
        let components = BoardFrames::new(&board.tables, Bits::EMPTY);
        Case { board, components, extended, fixed, open: open.to_vec(), claims }
    }

    fn n(&self) -> u32 {
        self.board.n()
    }

    /// The board, the open sources and the formulation, for the message of a failed check.
    fn at(&self) -> String {
        format!("n={}, O={:?}, {}", self.n(), self.open, formulation(self.claims))
    }

    /// The numbers family A chooses its sets from, ascending: the numbers up to n/2 that are not prime, and the sources
    /// of three or more prime factors outside F.
    fn items_a(&self) -> Vec<u32> {
        let (t, n) = (&self.board.tables, self.n());
        let lower = (2..=n / 2).filter(|&x| !t.is_prime(x));
        let sources = (n / 2 + 1..=n).filter(|&x| t.omega(x) >= 3 && !self.fixed.contains(&x));
        lower.chain(sources).collect()
    }

    /// The numbers family B chooses its sets from, ascending: the two-sided candidates and the one-sided ones.
    fn items_b(&self) -> Vec<u32> {
        let extended = &self.extended;
        extended.candidates.iter().chain(extended.one_sided.iter().map(|&(source, _)| source)).collect()
    }

    /// Whether F ∪ `x` is playable, by the oracle.
    fn playable_with(&self, x: &[u32]) -> bool {
        playable(&self.board.tables, &[self.fixed.as_slice(), x].concat())
    }

    /// K and L of the set `x`: its sources, and its numbers up to n/2.
    fn split(&self, x: &[u32]) -> (Bits<W>, Bits<W>) {
        let (k, l): (Vec<u32>, Vec<u32>) = x.iter().copied().partition(|&number| 2 * number > self.n());
        (Bits::from_slice(&k), Bits::from_slice(&l))
    }

    /// Whether `x` is a promotion set of Π⁺(O): its numbers up to n/2 are candidates, and the pair (K, L) passes the
    /// reference predicate.
    fn is_promotion_set(&self, x: &[u32]) -> bool {
        let (k, l) = self.split(x);
        l.is_subset(&self.extended.candidates) && self.extended.promotable(&self.components, &k, &l)
    }

    /// What a failed check says of the set `x`.
    fn describe(&self, x: &[u32]) -> String {
        let (k, l) = self.split(x);
        format!("{}: S = F ∪ {x:?}, K = {:?}, L = {:?}", self.at(), k.to_vec(), l.to_vec())
    }

    /// What a failed check says of the set `x`, on which the oracle and the problem disagree as `kind` says.
    fn disagreement(&self, kind: Disagreement, x: &[u32]) -> String {
        format!("{}: {}", kind.what(), self.describe(x))
    }

    /// The mutant of this problem that skips the check of Lemma M5⁺: the claims of the components the check refused
    /// ([`ExtendedFrames::refused`]) are kept after all, each pair (s, f) taken out of its frame (s from the moves and
    /// f from the factors, the freedoms recomputed), as if the component had passed.  The problem is the same
    /// otherwise.  Only a problem with claims (`claims` set) refuses any.
    fn skipping_the_check(&self) -> Case<'b, W> {
        let mut mutant = Case::new(self.board, &self.open, self.claims);
        let refused = std::mem::take(&mut mutant.extended.refused);
        for &(s, f, _) in &refused {
            let frame = mutant.extended.frames.iter_mut().find(|frame| frame.moves.contains(s));
            let frame = frame.unwrap_or_else(|| panic!("{}: the refused {s} is a move of no frame", self.at()));
            *frame = model::Frame::new(frame.moves.without(s), frame.factors.without(f));
        }
        mutant.extended.claims.extend(refused);
        mutant
    }
}

/// How the oracle and the problem Π⁺(O) of a [`Case`] can disagree on a set X: the theorem says F ∪ X is playable
/// exactly when the pair (K, L) of X is a promotion set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Disagreement {
    /// F ∪ X is playable but (K, L) is not a promotion set.
    NotAPromotionSet,
    /// (K, L) is a promotion set but F ∪ X is not playable.
    NotPlayable,
}

impl Disagreement {
    fn what(self) -> &'static str {
        match self {
            Disagreement::NotAPromotionSet => "S is playable but (K, L) is not a promotion set",
            Disagreement::NotPlayable => "(K, L) is a promotion set but S is not playable",
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Every set, on small boards

/// What a walk over every set of a [`Case`] found: the sizes of the two families, and the heaviest set of family A.
struct Families {
    /// The sets X of family A: F ∪ X is playable by the oracle.
    in_a: usize,
    /// The promotion sets of Π⁺(O), family B.
    in_b: usize,
    /// The set of family A with the greatest sum.
    heaviest: Vec<u32>,
}

/// Walks both families of `case`, the sets X of family A (F ∪ X playable, found depth first) and the promotion sets of
/// family B (found the same way), and calls `disagree` with the kind and the set X for each set of A that is not a
/// promotion set and each set of B that is not playable with F.
///
/// The sets are visited as the searches find them and not kept.  Both families are closed under subsets (for the
/// promotion sets of any frames: taking a promotion out leaves every frame winnable), so each search finds every set of
/// its family.  If no set of A is not a promotion set and no set of B is unplayable, A is in B and B is in A: the
/// families are equal, and so are their sizes.
fn walk_every_set<const W: usize>(case: &Case<W>, disagree: &mut dyn FnMut(Disagreement, &[u32])) -> Families {
    let (mut in_a, mut heaviest) = (0, Vec::new());
    for_each_kept_subset(&case.items_a(), &mut |x| case.playable_with(x), &mut |x| {
        if !case.is_promotion_set(x) {
            disagree(Disagreement::NotAPromotionSet, x);
        }
        in_a += 1;
        if sum(x) > sum(&heaviest) {
            heaviest = x.to_vec();
        }
    });
    let mut in_b = 0;
    for_each_kept_subset(&case.items_b(), &mut |x| case.is_promotion_set(x), &mut |x| {
        if !case.playable_with(x) {
            disagree(Disagreement::NotPlayable, x);
        }
        in_b += 1;
    });
    Families { in_a, in_b, heaviest }
}

/// How many sets the oracle and the problem of `case` disagree on, over every set of the two families: the sets of
/// family A that are not promotion sets, and the sets of family B that are not playable.
fn count_disagreements<const W: usize>(case: &Case<W>) -> (usize, usize) {
    let (mut not_promotion_sets, mut not_playable) = (0, 0);
    walk_every_set(case, &mut |kind, _| match kind {
        Disagreement::NotAPromotionSet => not_promotion_sets += 1,
        Disagreement::NotPlayable => not_playable += 1,
    });
    (not_promotion_sets, not_playable)
}

/// Theorem M⁺ on the board `1..=n` for the open sources `open`, in the formulation `claims` says, on every set: the
/// sets X of family A (F ∪ X playable, found depth first) are promotion sets, the promotion sets of family B (found the
/// same way) make F ∪ X playable, the two families are equal, `in_the_bag` is ΣF, the heaviest ΣS is what the table of
/// optimal games has for `n`, and the oracle's order for the heaviest S replays to its sum.  Returns the number of
/// sets.
///
/// The sets are those of [`walk_every_set`], which panics here at the first disagreement: every set of A asserted a
/// promotion set puts A in B, and every set of B asserted playable with F puts B in A, so the families are equal (and
/// so are their sizes, which are compared too).
fn check_every_set(n: u32, open: &[u32], claims: bool) -> usize {
    let board = Board::<8>::for_n(n);
    let case = Case::new(&board, open, claims);
    assert_eq!(case.extended.in_the_bag, sum(&case.fixed), "{}: in the bag", case.at());
    assert!(case.playable_with(&[]), "{}: F is not playable", case.at());

    let Families { in_a, in_b, heaviest } =
        walk_every_set(&case, &mut |kind, x| panic!("{}", case.disagreement(kind, x)));
    assert_eq!(in_a, in_b, "{}: the families differ in size", case.at());

    let total = case.extended.in_the_bag + sum(&heaviest);
    assert_eq!(Some(total), Optimal::built_in().score(n), "{}: the heaviest S and the table", case.at());
    let set = [case.fixed.as_slice(), &heaviest].concat();
    let order = play_order(&board.tables, &set).expect("the heaviest set is playable");
    assert_eq!(game::replay(n, &order), Ok(total), "{}: the oracle's order {order:?}", case.describe(&heaviest));
    in_a
}

/// [`check_every_set`] in the formulation `claims` says on each board of `boards`, for the open sets `choose` gives for
/// the board `n` from its U\*_{≥3}, with the time of each.  There are sets other than the empty one on some board.
fn check_every_board(boards: RangeInclusive<u32>, claims: bool, mut choose: impl FnMut(u32, &[u32]) -> Vec<Vec<u32>>) {
    let (start, mut total, mut checks) = (Instant::now(), 0, 0);
    for n in boards.clone() {
        let upper_from_3 = upper_from_3(&Board::<8>::for_n(n));
        for open in choose(n, &upper_from_3) {
            let began = Instant::now();
            let sets = check_every_set(n, &open, claims);
            println!("n={n:>3} {} |O|={:>2}: {sets:>7} sets  {:.1?}", formulation(claims), open.len(), began.elapsed());
            total += sets;
            checks += 1;
        }
    }
    println!(
        "the boards {boards:?}, {}: {checks} open sets, {total} sets in {:.1?}",
        formulation(claims),
        start.elapsed()
    );
    assert!(total > checks, "no board has a set but the empty one");
}

/// Theorem M⁺ with no open source, on every set of every board from 2 to 80.
#[test]
fn theorem_m_plus_with_no_open_source() {
    check_every_board(2..=80, false, |_, _| vec![Vec::new()]);
}

/// [`theorem_m_plus_with_no_open_source`] in the formulation with the claims of `unzip`, which with no source open is
/// the frame model's own problem.
#[test]
fn theorem_m_plus_with_no_open_source_with_claims() {
    check_every_board(2..=80, true, |_, _| vec![Vec::new()]);
}

/// Theorem M⁺ with three random subsets of U\*_{≥3} open, on every set of every board from 2 to 80 that has such
/// sources.
#[test]
fn theorem_m_plus_with_open_sources() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    check_every_board(2..=80, false, |_, from_3| random_open_sets(&mut rng, from_3, 3, 8));
}

/// [`theorem_m_plus_with_open_sources`] with the claims of `unzip`, for other random subsets.
#[test]
fn theorem_m_plus_with_open_sources_with_claims() {
    let mut rng = Rng(0xBF58_476D_1CE4_E5B9);
    check_every_board(2..=80, true, |_, from_3| random_open_sets(&mut rng, from_3, 3, 8));
}

/// Theorem M⁺ with every source of three or more prime factors open, on every set of every board from 2 to 60.
#[test]
fn theorem_m_plus_with_every_source_open() {
    check_every_board(2..=60, false, |_, from_3| vec![from_3.to_vec()]);
}

/// [`theorem_m_plus_with_every_source_open`] with the claims of `unzip`, which only the components of layers 1 and 2
/// keep then.
#[test]
fn theorem_m_plus_with_every_source_open_with_claims() {
    check_every_board(2..=60, true, |_, from_3| vec![from_3.to_vec()]);
}

/// [`theorem_m_plus_with_no_open_source`] and [`theorem_m_plus_with_open_sources`] on the boards 81 to 100.
#[test]
#[ignore = "takes twenty seconds; the boards to 80 are in the default tests"]
fn theorem_m_plus_with_open_sources_81_to_100() {
    check_every_board(81..=100, false, |_, _| vec![Vec::new()]);
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    check_every_board(81..=100, false, |_, from_3| random_open_sets(&mut rng, from_3, 3, 8));
}

/// [`theorem_m_plus_with_open_sources_81_to_100`] with the claims of `unzip`.
#[test]
#[ignore = "takes twenty seconds; the boards to 80 are in the default tests"]
fn theorem_m_plus_with_open_sources_81_to_100_with_claims() {
    check_every_board(81..=100, true, |_, _| vec![Vec::new()]);
    let mut rng = Rng(0x94D0_49BB_1331_11EB);
    check_every_board(81..=100, true, |_, from_3| random_open_sets(&mut rng, from_3, 3, 8));
}

/// [`theorem_m_plus_with_every_source_open`] on the boards 61 to 69, which have 0.4 to 5.5 million sets each and take
/// a minute in all.  The boards 70 and 71 have 62 million sets each (3.5 minutes each), and the numbers go on growing.
#[test]
#[ignore = "takes a minute; the boards to 60 are in the default test"]
fn theorem_m_plus_with_every_source_open_61_to_69() {
    check_every_board(61..=69, false, |_, from_3| vec![from_3.to_vec()]);
}

/// [`theorem_m_plus_with_every_source_open_61_to_69`] with the claims of `unzip`.
#[test]
#[ignore = "takes a minute; the boards to 60 are in the default test"]
fn theorem_m_plus_with_every_source_open_61_to_69_with_claims() {
    check_every_board(61..=69, true, |_, from_3| vec![from_3.to_vec()]);
}

/// The check of Lemma M5⁺ is needed: a mutant of the problem with the claims that skips it is caught.
///
/// The check refuses the claims of a component that holds no open source when one of its back claims (s, f) has f a
/// two-sided candidate or listed among the maximal factors of one (`ExtendedFrames::refused`).  The mutant keeps those
/// claims anyway: it takes each refused pair out of its frame (`Case::skipping_the_check`), which on the board 24 with
/// O = {16, 20, 24} is the pair (18, 6), where the candidate 12 lists 6.  It then loses the promotion set {8, 12}: in
/// the game 18 pays with 9, 8 with 4 and 12 with 6, so F ∪ {8, 12} is playable, but without 6 in the frame 12 has
/// nothing left to be paid with once 8 takes 4.
///
/// Such an O is rare.  Opening a source adds freedoms, so the candidates only grow with O, and so does the list of
/// their maximal factors: a component's claims fail the check for some O free of it (with no member of O in it)
/// exactly when they fail for the largest such O, which is all of U\*_{≥3} but the component's sources.  So the O to
/// try on a board is U\*_{≥3} less the sources of C, for each component C of layer 3 or more; the components of layers
/// 1 and 2 are free of every O, and so of U\*_{≥3} itself, which [`theorem_m_plus_with_every_source_open_with_claims`]
/// enumerates (a scan of the boards to 400 found a refusal, for some O, only on the boards 24 to 26 and 343 to 349,
/// each for an O that leaves out the sources of a component of layer 3 or more).  A random subset of U\*_{≥3} is
/// unlikely to be such an O, which is why the other tests would hardly catch the mutant.  On each board from 2 to 80
/// and each such O that the check refuses, this test enumerates every set (as [`check_every_set`] does) for the true
/// problem, which must agree with the oracle on all of them, and for the mutant, which must not on at least one set of
/// at least one board.
/// The boards 343 to 349 are too large for every set (a board has 62 million at 70, and the number goes on growing),
/// and are left to the unit test of `build_extended` (`a_back_claim_that_fails_the_check_is_refused`) and the census
/// of the frames (`examples/structure.rs`).
#[test]
fn skipping_the_check_of_lemma_m5_plus_is_caught() {
    let (start, mut refusing, mut caught) = (Instant::now(), 0, 0);
    for n in 2..=80 {
        let board = Board::<8>::for_n(n);
        let (t, from_3) = (&board.tables, upper_from_3(&board));
        let components = BoardFrames::<8>::new(t, Bits::EMPTY);
        let sources = components.frames().iter().map(|component| component.sources());
        for own in sources.filter(|own| own.min().is_some_and(|source| t.omega(source) >= 3)) {
            let open: Vec<u32> = from_3.iter().copied().filter(|&x| !own.contains(x)).collect();
            let case = Case::new(&board, &open, true);
            if case.extended.refused.is_empty() {
                continue;
            }
            let began = Instant::now();
            let (truth, mutant) = (count_disagreements(&case), count_disagreements(&case.skipping_the_check()));
            println!(
                "n={n:>3} O={open:?} refused {:?}: true problem {truth:?}, mutant {mutant:?} (sets of A that are not \
                 promotion sets, sets of B that are not playable)  {:.1?}",
                case.extended.refused,
                began.elapsed()
            );
            assert_eq!(truth, (0, 0), "{}: the true problem disagrees with the oracle", case.at());
            refusing += 1;
            caught += usize::from(mutant != (0, 0));
        }
    }
    println!("{refusing} problems refuse a claim, {caught} of them caught the mutant, in {:.1?}", start.elapsed());
    assert!(refusing > 0 && caught > 0, "no problem refuses a claim, or the mutant is never caught");

    // The board 24 with 16, 20 and 24 open: the check refuses (18, 6), and the mutant loses {8, 12}.
    let board = Board::<8>::for_n(24);
    let case = Case::new(&board, &[16, 20, 24], true);
    assert_eq!(case.extended.refused, [(18, 6, ClaimKind::Back)], "{}", case.at());
    let mutant = case.skipping_the_check();
    assert!(case.playable_with(&[8, 12]), "{}: F ∪ {{8, 12}} is playable", case.at());
    assert!(case.is_promotion_set(&[8, 12]), "{}: {{8, 12}} is a promotion set", case.at());
    assert!(!mutant.is_promotion_set(&[8, 12]), "{}: the mutant misses {{8, 12}}", case.at());
}

// ---------------------------------------------------------------------------------------------------------
// Π⁺(∅) against Π_n

/// The promotion sets of the problem `prepared` poses the search, found depth first: the sets of candidates that can
/// be promoted from the root, with at most `max_total` members.
fn promotion_sets_of<const W: usize>(prepared: &Prepared<W>) -> BTreeSet<Vec<u32>> {
    let mut scratch = Scratch::<W>::new();
    kept_subsets(&prepared.candidates.to_vec(), &mut |set| {
        let promoted = Bits::<W>::from_slice(set);
        set.len() as u32 <= prepared.max_total
            && prepared.model.can_promote_all_using(&mut scratch, &prepared.root, &promoted)
    })
}

/// With no open source, Π⁺(∅) has the promotion sets of the frame model on every board from 2 to 100: the sets L of
/// family B are the promotion sets of Π_n (`Board::prepare`) and those of the problem the search is given for Π⁺(∅)
/// (`Board::prepare_extended`: the reduced root, the model built from `columns`), in the formulation `claims` says.
fn check_the_extended_problem_with_no_open_source_is_the_frame_model(claims: bool) {
    let (start, mut total) = (Instant::now(), 0);
    for n in 2..=100 {
        let board = Board::<8>::for_n(n);
        let case = Case::new(&board, &[], claims);
        assert!(case.extended.one_sided.is_empty(), "n={n}: a one-sided candidate");
        let extended: BTreeSet<Vec<u32>> = kept_subsets(&case.items_b(), &mut |x| case.is_promotion_set(x));

        let plain = board.prepare(SearchTactics::default()).unwrap_or_else(|e| panic!("n={n}: {e}"));
        let in_the_frame_model = promotion_sets_of(&plain);
        let (_, searched) =
            board.prepare_extended(claims, SearchTactics::default()).unwrap_or_else(|e| panic!("n={n}: {e}"));
        let in_the_search = promotion_sets_of(&searched);

        let first_difference = |a: &BTreeSet<Vec<u32>>, b: &BTreeSet<Vec<u32>>| -> Vec<Vec<u32>> {
            a.symmetric_difference(b).take(3).cloned().collect()
        };
        assert_eq!(extended, in_the_frame_model, "n={n}: {:?}", first_difference(&extended, &in_the_frame_model));
        assert_eq!(
            in_the_search,
            in_the_frame_model,
            "n={n}: {:?}",
            first_difference(&in_the_search, &in_the_frame_model)
        );
        total += extended.len();
    }
    println!("the boards 2..=100, {}: {total} promotion sets in {:.1?}", formulation(claims), start.elapsed());
    assert!(total > 99, "no board has a promotion set but the empty one");
}

#[test]
fn the_extended_problem_with_no_open_source_is_the_frame_model() {
    check_the_extended_problem_with_no_open_source_is_the_frame_model(false);
}

#[test]
fn the_extended_problem_with_no_open_source_and_the_claims_is_the_frame_model() {
    check_the_extended_problem_with_no_open_source_is_the_frame_model(true);
}

// ---------------------------------------------------------------------------------------------------------
// Random sets, on larger boards

/// A random promotion set of Π⁺(O) as the set X = K ∪ L, ascending: the one-sided and two-sided candidates in a random
/// order, each added if the set stays a promotion set, until a length drawn at random from 0 to the freedoms of the
/// frames (more than a promotion set can have) is reached.
fn random_promotion_set<const W: usize>(case: &Case<W>, rng: &mut Rng) -> Vec<u32> {
    let mut order = case.items_b();
    shuffle(rng, &mut order);
    let room: i32 = case.extended.frames.iter().map(|frame| frame.freedoms.max(0)).sum();
    let target = rng.below(room as u64 + 1) as usize;
    let mut chosen = Vec::new();
    for x in order {
        if chosen.len() == target {
            break;
        }
        chosen.push(x);
        if !case.is_promotion_set(&chosen) {
            chosen.pop();
        }
    }
    chosen.sort_unstable();
    chosen
}

/// A random maximal playable set X, ascending: the numbers of family A in a random order, each kept if F ∪ X stays
/// playable.
fn random_maximal_playable_set<const W: usize>(case: &Case<W>, rng: &mut Rng) -> Vec<u32> {
    let mut order = case.items_a();
    shuffle(rng, &mut order);
    let mut chosen = Vec::new();
    for x in order {
        chosen.push(x);
        if !case.playable_with(&chosen) {
            chosen.pop();
        }
    }
    chosen.sort_unstable();
    chosen
}

/// How many sets of each kind a check made, and how many of them had a rejected source in them.
#[derive(Clone, Copy, Debug, Default)]
struct Tally {
    promotion_sets: usize,
    promotion_members: usize,
    maximal_sets: usize,
    maximal_members: usize,
    with_rejected: usize,
}

impl Tally {
    fn mean(members: usize, sets: usize) -> f64 {
        members as f64 / sets.max(1) as f64
    }

    fn add(&mut self, other: &Tally) {
        self.promotion_sets += other.promotion_sets;
        self.promotion_members += other.promotion_members;
        self.maximal_sets += other.maximal_sets;
        self.maximal_members += other.maximal_members;
        self.with_rejected += other.with_rejected;
    }
}

/// Theorem M⁺ on `sets` random sets of each of two kinds, for the open sources of `case`.
///
/// *Soundness*: a random promotion set of Π⁺(O) makes F ∪ K ∪ L playable.
///
/// *Completeness*: a random maximal playable set F ∪ X is a promotion set: its numbers up to n/2 are candidates, and
/// the pair passes the reference predicate.
fn check_random_sets<const W: usize>(case: &Case<W>, sets: usize, rng: &mut Rng) -> Tally {
    let mut tally = Tally::default();
    let rejected = case.extended.rejected;
    assert!(case.playable_with(&[]), "{}: F is not playable", case.at());
    for _ in 0..sets {
        let x = random_promotion_set(case, rng);
        assert!(case.playable_with(&x), "{}", case.disagreement(Disagreement::NotPlayable, &x));
        tally.promotion_sets += 1;
        tally.promotion_members += x.len();
        tally.with_rejected += usize::from(x.iter().any(|&s| rejected.contains(s)));
    }
    for _ in 0..sets {
        let x = random_maximal_playable_set(case, rng);
        assert!(case.is_promotion_set(&x), "{}", case.disagreement(Disagreement::NotAPromotionSet, &x));
        tally.maximal_sets += 1;
        tally.maximal_members += x.len();
        tally.with_rejected += usize::from(x.iter().any(|&s| rejected.contains(s)));
    }
    tally
}

/// With no open source, random promotion sets of the frame model (`Board::prepare`) are promotion sets of Π⁺(∅), in
/// the formulation `claims` says, and random promotion sets of Π⁺(∅) are promotion sets of the frame model.  With the
/// claims, the search is given the frame model's candidates and `max_total` (`Board::prepare_extended`).
fn check_random_sets_against_the_frame_model<const W: usize>(
    board: &Board<W>,
    claims: bool,
    sets: usize,
    rng: &mut Rng,
) {
    let (n, case) = (board.n(), Case::new(board, &[], claims));
    let p = board.prepare(SearchTactics::default()).unwrap_or_else(|e| panic!("n={n}: {e}"));
    assert_eq!(case.extended.candidates, p.candidates, "{}: the candidates", case.at());
    if claims {
        let (_, searched) =
            board.prepare_extended(true, SearchTactics::default()).unwrap_or_else(|e| panic!("n={n}: {e}"));
        assert_eq!(searched.candidates, p.candidates, "{}: the candidates the search is given", case.at());
        assert_eq!(searched.max_total, p.max_total, "{}: max_total the search is given", case.at());
    }
    let mut scratch = Scratch::<W>::new();
    for _ in 0..sets {
        let mut order = p.candidates.to_vec();
        shuffle(rng, &mut order);
        let target = rng.below(u64::from(p.max_total) + 1) as usize;
        let (mut state, mut lower) = (p.root, Vec::new());
        for &c in &order {
            if lower.len() == target {
                break;
            }
            if p.model.promote_in_place_using(&mut scratch, &mut state, c) {
                lower.push(c);
            }
        }
        lower.sort_unstable();
        assert!(case.is_promotion_set(&lower), "n={n}: a promotion set of the frame model: L = {lower:?}");

        let x = random_promotion_set(&case, rng);
        let set = Bits::<W>::from_slice(&x);
        let promotable = p.model.can_promote_all_using(&mut scratch, &p.root, &set);
        assert!(promotable && x.len() as u32 <= p.max_total, "n={n}: a promotion set of Π⁺(∅): L = {x:?}");
    }
}

/// Theorem M⁺ on the board `1..=n` at width `W`, in the formulation `claims` says, on `sets` random sets of each kind
/// for a random 20 members of U\*_{≥3} open, for all of them open, and for none open, where the frame model is checked
/// against Π⁺(∅) too.
fn check_random_board<const W: usize>(n: u32, claims: bool, sets: usize, rng: &mut Rng) -> Tally {
    let board = Board::<W>::for_n(n);
    let every = upper_from_3(&board);
    let mut random = every.clone();
    shuffle(rng, &mut random);
    random.truncate(20);
    random.sort_unstable();

    let mut tally = Tally::default();
    for open in [random, every, Vec::new()] {
        let start = Instant::now();
        let case = Case::new(&board, &open, claims);
        let found = check_random_sets(&case, sets, rng);
        if open.is_empty() {
            check_random_sets_against_the_frame_model(&board, claims, sets, rng);
        }
        println!(
            "n={n:>5} W={W:>3} {} |O|={:>3} |R|={:>2}  {} candidates, {} one-sided  mean |X|: promotion sets {:>6.1}, \
             maximal sets {:>6.1}  with a rejected source: {}  {:.1?}",
            formulation(claims),
            open.len(),
            case.extended.rejected.len(),
            case.extended.candidates.len(),
            case.extended.one_sided.len(),
            Tally::mean(found.promotion_members, found.promotion_sets),
            Tally::mean(found.maximal_members, found.maximal_sets),
            found.with_rejected,
            start.elapsed(),
        );
        tally.add(&found);
    }
    tally
}

/// Theorem M⁺ on `sets` random sets of each kind on each of `boards`, each at the width the solver picks, in the
/// formulation `claims` says, from the random stream seeded with `seed`.
fn check_random_boards(boards: &[u32], claims: bool, sets: usize, seed: u64) -> Tally {
    let mut rng = Rng(seed);
    let (start, mut tally) = (Instant::now(), Tally::default());
    for &n in boards {
        tally.add(&with_width!(n, W => check_random_board::<W>(n, claims, sets, &mut rng)));
    }
    println!("{} boards, {}, {sets} sets of each kind: {:.1?}", boards.len(), formulation(claims), start.elapsed());
    // A check of tiny sets would show: the sets must not all be empty.
    assert!(tally.promotion_members > 0 && tally.maximal_members > 0, "only empty sets: {tally:?}");
    tally
}

/// Theorem M⁺ on random sets of the boards 300, 600, 1000 and 1300.
#[test]
fn theorem_m_plus_on_random_sets() {
    check_random_boards(&[300, 600, 1000, 1300], false, 100, 0xA076_1D64_78BD_642F);
}

/// [`theorem_m_plus_on_random_sets`] with the claims of `unzip`, for other random sets.
#[test]
fn theorem_m_plus_on_random_sets_with_claims() {
    check_random_boards(&[300, 600, 1000, 1300], true, 100, 0x5851_F42D_4C95_7F2D);
}

/// Theorem M⁺ on random sets of the boards 2000, 3000 and 4095.
#[test]
#[ignore = "takes a while; the boards to 1300 are in the default test"]
fn theorem_m_plus_on_random_sets_past_1300() {
    check_random_boards(&[2000, 3000, 4095], false, 100, 0xE703_7ED1_A0B4_28DB);
}

/// [`theorem_m_plus_on_random_sets_past_1300`] with the claims of `unzip`.
#[test]
#[ignore = "takes a while; the boards to 1300 are in the default test"]
fn theorem_m_plus_on_random_sets_past_1300_with_claims() {
    check_random_boards(&[2000, 3000, 4095], true, 100, 0x1405_7B7E_F767_814F);
}

/// Theorem M⁺ on random sets of boards where greedy rejects a source of layer 3, which the opening of a source above
/// it can let in: 741 (374 is rejected), 1100 (578) and 1200 (605).  Greedy rejects no source of layer 3 or more on
/// the boards to 740, nor on 1000 or 1300, so the other tests never meet one; some of the sets here have one in them.
#[test]
fn theorem_m_plus_with_a_rejected_source() {
    let tally = check_random_boards(&[741, 1100, 1200], false, 100, 0x8EBC_6AF0_9C88_C6E3);
    assert!(tally.with_rejected > 0, "no set has a rejected source: {tally:?}");
}

/// [`theorem_m_plus_with_a_rejected_source`] with the claims of `unzip` (which the components that hold an open or a
/// rejected source do not keep).
#[test]
fn theorem_m_plus_with_a_rejected_source_with_claims() {
    let tally = check_random_boards(&[741, 1100, 1200], true, 100, 0x2F72_B4B7_F3A9_C5D1);
    assert!(tally.with_rejected > 0, "no set has a rejected source: {tally:?}");
}

// ---------------------------------------------------------------------------------------------------------
// The model with one-sided candidates

/// How much a check of the model draws for each problem Π⁺(O).
#[derive(Clone, Copy, Debug)]
struct Draws {
    /// Random subsets of the candidates, at densities from 3 to 30 percent.
    subsets: usize,
    /// Chains P ⊆ Q of promotion sets, with the states they reach.
    chains: usize,
}

/// What the checks of the model reached, so that a test can show it met what it was meant to test.
#[derive(Clone, Copy, Debug, Default)]
struct ModelTally {
    /// Problems Π⁺(O) checked, and those of them with a one-sided candidate.
    problems: usize,
    with_one_sided: usize,
    /// Sets checked against the reference, those that are promotion sets, and those of each kind with a one-sided
    /// member.
    sets: usize,
    promotable: usize,
    sets_with_one_sided: usize,
    promotable_with_one_sided: usize,
    /// Conflicts checked (both kinds), those with a one-sided member, and nogoods.
    conflicts: usize,
    conflicts_with_one_sided: usize,
    nogoods: usize,
    /// States whose narrowing and count were checked.
    states: usize,
}

/// A chain of promotion sets P ⊆ Q, and a candidate outside Q.
struct Chain<const W: usize> {
    p: Bits<W>,
    q: Bits<W>,
    /// The candidate that the reference rejected beside Q while Q was made, so that Q with it cannot be promoted; or,
    /// when none was rejected, a random candidate outside Q (none if Q holds them all).
    extra: Option<u32>,
}

/// A density, in percent, at which to draw a random subset: low enough for many of the subsets to be promotable.
fn density(rng: &mut Rng) -> u64 {
    rng.pick(&[3, 5, 10, 20, 30])
}

/// The extended problem Π⁺(O) as the search is given it: the model of `FrameGraph::build_extended` with its root,
/// beside the reference predicate of its [`Case`].
struct Problem<'b, const W: usize> {
    case: Case<'b, W>,
    model: FrameGraph,
    root: FrameSet<W>,
    /// The one-sided candidates.
    one_sided: Bits<W>,
    /// The candidates the search is given: the two-sided ones and the one-sided ones.
    candidates: Bits<W>,
}

impl<'b, const W: usize> Problem<'b, W> {
    /// The problem Π⁺(`open`) of `board`, without the claims of `unzip`, or `None` if it has no candidate at all,
    /// two-sided or one-sided.
    fn new(board: &'b Board<W>, open: &[u32]) -> Option<Self> {
        let case = Case::new(board, open, false);
        let extended = &case.extended;
        let (model, root) = FrameGraph::build_extended::<W>(&board.tables, &extended.columns(), &extended.one_sided)
            .unwrap_or_else(|e| panic!("n={}: O={open:?}: {e}", board.n()));
        let one_sided: Bits<W> = extended.one_sided.iter().map(|&(s, _)| s).collect();
        let candidates = extended.candidates | one_sided;
        // The model keeps the one-sided candidates it was given, each with its own frame.
        let what = format!("n={}: O={open:?}: the one-sided candidates", board.n());
        assert_eq!(model.one_sided(), extended.one_sided.as_slice(), "{what}");
        assert_eq!(model.has_one_sided(), !one_sided.is_empty(), "{what}");
        assert!(extended.one_sided.iter().all(|&(s, frame)| model.own(s) == Some(frame)), "{what}: own frames");
        (!candidates.is_empty()).then_some(Problem { case, model, root, one_sided, candidates })
    }

    /// The board, the open sources and the formulation, for the message of a failed check.
    fn at(&self) -> String {
        self.case.at()
    }

    /// Whether the set `x` is a promotion set, by the reference predicate: K is its one-sided members and L the rest.
    fn reference(&self, x: &Bits<W>) -> bool {
        let (k, extended) = (*x & self.one_sided, &self.case.extended);
        let l = *x - k;
        l.is_subset(&extended.candidates) && extended.promotable(&self.case.components, &k, &l)
    }

    /// The state after promoting the members of `x` from the root one at a time, in a random order, or `None` if one
    /// of them cannot be promoted.  Before each, `can_promote_using` must say what `promote_in_place_using` then does,
    /// and a promotion that fails must leave the state as it was.
    fn promote_one_by_one(&self, scratch: &mut Scratch<W>, x: &Bits<W>, rng: &mut Rng) -> Option<FrameSet<W>> {
        let mut order = x.to_vec();
        shuffle(rng, &mut order);
        let mut state = self.root;
        for s in order {
            let (before, can) = (state, self.model.can_promote_using(scratch, &state, s));
            let done = self.model.promote_in_place_using(scratch, &mut state, s);
            assert_eq!(can, done, "{}: can_promote_using and promote_in_place_using of {s} in X = {:?}", self.at(), x);
            if !done {
                assert_eq!(state, before, "{}: a failed promotion of {s} changed the state", self.at());
                return None;
            }
        }
        Some(state)
    }

    /// The promotion set `from` with candidates added: they are tried in a random order and kept if the reference says
    /// the set stays a promotion set, until it has `size` members or all are tried.  Also the candidates rejected on
    /// the way, each of which makes the set unpromotable (a superset of a set that cannot be promoted cannot be).
    fn extend(&self, from: Bits<W>, size: usize, rng: &mut Rng) -> (Bits<W>, Vec<u32>) {
        let mut order = (self.candidates - from).to_vec();
        shuffle(rng, &mut order);
        let (mut set, mut rejected) = (from, Vec::new());
        for x in order {
            if set.len() as usize >= size {
                break;
            }
            if self.reference(&set.with(x)) {
                set.insert(x);
            } else {
                rejected.push(x);
            }
        }
        (set, rejected)
    }

    /// A random chain: P a promotion set of a random size from 0 to the freedoms of the frames (more than a promotion
    /// set can have), and Q = P with up to five candidates more.
    fn chain(&self, rng: &mut Rng) -> Chain<W> {
        let room: i32 = self.case.extended.frames.iter().map(|frame| frame.freedoms.max(0)).sum();
        let size = rng.below(room as u64 + 1) as usize;
        let (p, _) = self.extend(Bits::EMPTY, size, rng);
        let more = rng.below(6) as usize;
        let (q, rejected) = self.extend(p, p.len() as usize + more, rng);
        let outside = (self.candidates - q).to_vec();
        let extra = if rejected.is_empty() {
            (!outside.is_empty()).then(|| rng.pick(&outside))
        } else {
            Some(rng.pick(&rejected))
        };
        Chain { p, q, extra }
    }

    /// The set `x` against the reference: `can_promote_all_using`, promoting its members one by one in a random order
    /// and `promote_all_using` must all say what the reference says, and the two states they reach must be equal
    /// (reduction is confluent).  Then, for a promotion set, its narrowing and count at the root; for another set, its
    /// conflicts at the root.  Returns whether `x` is a promotion set.
    fn check_set(&self, scratch: &mut Scratch<W>, x: &Bits<W>, rng: &mut Rng, tally: &mut ModelTally) -> bool {
        let reference = self.reference(x);
        let at_once = self.model.can_promote_all_using(scratch, &self.root, x);
        let all = self.model.promote_all_using(scratch, &self.root, x);
        let one_by_one = self.promote_one_by_one(scratch, x, rng);
        let what = || format!("{}: X = {:?}, K = {:?}", self.at(), x.to_vec(), (*x & self.one_sided).to_vec());
        assert_eq!(at_once, reference, "can_promote_all_using: {}", what());
        assert_eq!(all.is_some(), reference, "promote_all_using: {}", what());
        assert_eq!(one_by_one.is_some(), reference, "promoting one by one: {}", what());
        assert_eq!(all, one_by_one, "the states promote_all_using and promoting one by one reach: {}", what());

        let with_one_sided = x.intersects(&self.one_sided);
        tally.sets += 1;
        tally.sets_with_one_sided += usize::from(with_one_sided);
        if reference {
            tally.promotable += 1;
            tally.promotable_with_one_sided += usize::from(with_one_sided);
            self.check_narrowing_and_count(scratch, &Bits::EMPTY, &self.root, x, tally);
        } else {
            self.check_conflicts(scratch, &self.root, &Bits::EMPTY, x, tally);
        }
        reference
    }

    /// The narrowing and the count at `state`, the state after promoting the promotion set `p` from the root, for the
    /// promotion set `q ⊇ p`.  The rest of `q` must survive `narrow` (and so must every candidate that can be promoted
    /// at all), and the count `max_promotions_among` must not be below the size of the rest.
    fn check_narrowing_and_count(
        &self,
        scratch: &mut Scratch<W>,
        p: &Bits<W>,
        state: &FrameSet<W>,
        q: &Bits<W>,
        tally: &mut ModelTally,
    ) {
        let (rest, later) = (self.candidates - *p, *q - *p);
        let what = || format!("{}: P = {:?}, Q = {:?}", self.at(), p.to_vec(), q.to_vec());
        let narrowed = self.model.narrow(state, &rest);
        assert!(narrowed.is_subset(&rest), "narrow gave numbers that are not among the candidates: {}", what());
        assert!(later.is_subset(&narrowed), "narrow dropped {:?}: {}", (later - narrowed).to_vec(), what());
        for x in (rest - narrowed).iter() {
            assert!(
                !self.model.can_promote_using(scratch, state, x),
                "narrow dropped {x}, which can be promoted: {}",
                what()
            );
        }
        let count = self.model.max_promotions_among(state, &rest);
        assert!(
            count >= later.len() as i32,
            "max_promotions_among is {count} below the {} to promote: {}",
            later.len(),
            what()
        );
        tally.states += 1;
    }

    /// The conflicts of `y` at `state`, the state after promoting the promotion set `p` from the root, where `y` is
    /// disjoint from `p` and cannot be promoted from `state`: of each kind (`minimal` or not) a non-empty subset of `y`
    /// that cannot be promoted from `state` or, with `p`, from the root; and for the minimal one, its nogood, a subset
    /// of `p` and the conflict that cannot be promoted from the root.
    fn check_conflicts(
        &self,
        scratch: &mut Scratch<W>,
        state: &FrameSet<W>,
        p: &Bits<W>,
        y: &Bits<W>,
        tally: &mut ModelTally,
    ) {
        for minimal in [false, true] {
            let what = || format!("{}: P = {:?}, Y = {:?}, minimal = {minimal}", self.at(), p.to_vec(), y.to_vec());
            let k = self
                .model
                .conflict_using(scratch, state, y, minimal)
                .unwrap_or_else(|| panic!("no conflict: {}", what()));
            let how = || format!("the conflict {:?}: {}", k.to_vec(), what());
            assert!(!k.is_empty() && k.is_subset(y), "is empty or not within Y, {}", how());
            assert!(!self.model.can_promote_all_using(scratch, state, &k), "can be promoted from the state, {}", how());
            assert!(!self.reference(&(*p | k)), "can be promoted with P, by the reference, {}", how());
            tally.conflicts += 1;
            tally.conflicts_with_one_sided += usize::from(k.intersects(&self.one_sided));
            if minimal {
                let nogood =
                    self.model.nogood(scratch, &self.root, p, &k).unwrap_or_else(|| panic!("no nogood for {}", how()));
                let why = || format!("the nogood {:?} of {}", nogood.to_vec(), how());
                assert!(
                    !nogood.is_empty() && nogood.is_subset(&(*p | k)),
                    "is empty or not within P and the conflict, {}",
                    why()
                );
                assert!(
                    !self.model.can_promote_all_using(scratch, &self.root, &nogood),
                    "can be promoted from the root, {}",
                    why()
                );
                assert!(!self.reference(&nogood), "can be promoted, by the reference, {}", why());
                tally.nogoods += 1;
            }
        }
    }

    /// A random chain P ⊆ Q: both are promotion sets for the model, and Q with the extra candidate is the near miss.  At
    /// the state after P: the narrowing and the count for Q, and sets Y of candidates outside P, which are promotable
    /// from the state exactly when P ∪ Y is a promotion set, and when not have conflicts and nogoods.
    fn check_chain(&self, scratch: &mut Scratch<W>, rng: &mut Rng, tally: &mut ModelTally) {
        let Chain { p, q, extra } = self.chain(rng);
        for set in [p, q] {
            let promotable = self.check_set(scratch, &set, rng, tally);
            assert!(promotable, "{}: the promotion set {:?}", self.at(), set.to_vec());
        }
        if let Some(x) = extra {
            self.check_set(scratch, &q.with(x), rng, tally);
        }
        let state = self.model.promote_all_using(scratch, &self.root, &p).expect("P is a promotion set");
        self.check_narrowing_and_count(scratch, &p, &state, &q, tally);

        let rest = self.candidates - p;
        let mut ys = vec![q - p];
        ys.extend(extra.map(|x| (q - p).with(x)));
        for _ in 0..3 {
            let percent = density(rng);
            ys.push(random_subset(rng, &rest, percent));
        }
        for y in ys {
            let promotable = self.model.can_promote_all_using(scratch, &state, &y);
            let what = format!("{}: P = {:?}, Y = {:?}", self.at(), p.to_vec(), y.to_vec());
            assert_eq!(promotable, self.reference(&(p | y)), "Y from the state after P: {what}");
            if !promotable {
                self.check_conflicts(scratch, &state, &p, &y, tally);
            }
        }
    }

    /// All the checks on this problem: the empty set, every one-sided candidate alone, `draws.subsets` random subsets
    /// of the candidates and `draws.chains` chains.
    fn check(&self, draws: Draws, rng: &mut Rng, tally: &mut ModelTally) {
        let mut scratch = Scratch::<W>::new();
        tally.problems += 1;
        tally.with_one_sided += usize::from(!self.one_sided.is_empty());
        assert!(self.check_set(&mut scratch, &Bits::EMPTY, rng, tally), "{}: the empty set", self.at());
        for s in self.one_sided.iter() {
            self.check_set(&mut scratch, &Bits::EMPTY.with(s), rng, tally);
        }
        for _ in 0..draws.subsets {
            let percent = density(rng);
            let x = random_subset(rng, &self.candidates, percent);
            self.check_set(&mut scratch, &x, rng, tally);
        }
        for _ in 0..draws.chains {
            self.check_chain(&mut scratch, rng, tally);
        }
    }
}

/// The model with one-sided candidates against the reference predicate on each board of `boards` (at width 8), for each
/// of these open sets O: the empty set; U\*_{≥3} itself, up to the board 120; and five random subsets of U\*_{≥3} with
/// 1 to 20 members (none if U\*_{≥3} is empty, and no more than it has members).  A problem with no candidate at all is
/// skipped.
fn check_model_on_boards(boards: RangeInclusive<u32>, draws: Draws, seed: u64) -> ModelTally {
    let (mut rng, start, mut tally) = (Rng(seed), Instant::now(), ModelTally::default());
    for n in boards.clone() {
        let board = Board::<8>::for_n(n);
        let from_3 = upper_from_3(&board);
        let mut open_sets = vec![Vec::new()];
        if n <= 120 && !from_3.is_empty() {
            open_sets.push(from_3.clone());
        }
        open_sets.extend(random_open_sets(&mut rng, &from_3, 5, 20));
        for open in open_sets {
            if let Some(problem) = Problem::new(&board, &open) {
                problem.check(draws, &mut rng, &mut tally);
            }
        }
    }
    println!("the boards {boards:?}: {tally:?} in {:.1?}", start.elapsed());
    tally
}

/// That a check of the model reached what it is meant to test: every count is well above zero (those of the three
/// default tests are several times these), and both promotable and unpromotable sets are a good part of the sets.
fn assert_reached(tally: &ModelTally) {
    let ModelTally {
        problems,
        with_one_sided,
        sets,
        promotable,
        sets_with_one_sided,
        promotable_with_one_sided,
        conflicts,
        conflicts_with_one_sided,
        nogoods,
        states,
    } = *tally;
    assert!(problems >= 200 && with_one_sided >= 150, "too few problems: {tally:?}");
    assert!(sets >= 5_000 && promotable >= 2_000, "too few sets: {tally:?}");
    assert!(promotable * 10 >= sets && (sets - promotable) * 10 >= sets, "too few of one kind of set: {tally:?}");
    assert!(sets_with_one_sided >= 2_000 && promotable_with_one_sided >= 1_000, "too few one-sided sets: {tally:?}");
    assert!(conflicts >= 5_000 && conflicts_with_one_sided >= 500, "too few conflicts: {tally:?}");
    assert!(nogoods >= 2_500 && states >= 5_000, "too few nogoods or states: {tally:?}");
}

/// What the default tests draw for each problem.
const DRAWS: Draws = Draws { subsets: 100, chains: 40 };

/// The model of Π⁺(O) with one-sided candidates, against the reference predicate, on the boards 2 to 100: promoting a
/// set at once or one member at a time, its conflicts and nogoods, narrowing and count.
#[test]
fn the_model_with_one_sided_candidates_2_to_100() {
    assert_reached(&check_model_on_boards(2..=100, DRAWS, 0x2545_F491_4F6C_DD1D));
}

/// The same on the boards 101 to 200.
#[test]
fn the_model_with_one_sided_candidates_101_to_200() {
    assert_reached(&check_model_on_boards(101..=200, DRAWS, 0x9E37_79B1_85EB_CA87));
}

/// The same on the boards 201 to 300.
#[test]
fn the_model_with_one_sided_candidates_201_to_300() {
    assert_reached(&check_model_on_boards(201..=300, DRAWS, 0xC2B2_AE3D_27D4_EB4F));
}

/// The same on the boards 301 to 500 (the largest that width 8 holds is 511), which have more layers and so more frames
/// to hold a one-sided candidate.
#[test]
#[ignore = "takes half a minute; the boards to 300 are in the default tests"]
fn the_model_with_one_sided_candidates_301_to_500() {
    assert_reached(&check_model_on_boards(301..=500, DRAWS, 0x1656_67B1_9E37_79F9));
}

/// The boards 2 to 300 again with ten times the draws of the default tests.
#[test]
#[ignore = "takes a minute and a half; the draws of the default tests are a tenth of these"]
fn the_model_with_one_sided_candidates_with_more_draws() {
    let draws = Draws { subsets: 10 * DRAWS.subsets, chains: 10 * DRAWS.chains };
    assert_reached(&check_model_on_boards(2..=300, draws, 0xFF51_AFD7_ED55_8CCD));
}

// ---------------------------------------------------------------------------------------------------------
// The search against the whole-board oracle

/// The heaviest sum of a set X of family A (F ∪ X playable by the oracle, found depth first), and the number of sets of
/// the family: what the best game on the whole board makes beyond F.
fn heaviest_playable<const W: usize>(case: &Case<W>) -> (u64, usize) {
    let (mut heaviest, mut sets) = (0, 0);
    for_each_kept_subset(&case.items_a(), &mut |x| case.playable_with(x), &mut |x| {
        heaviest = heaviest.max(sum(x));
        sets += 1;
    });
    (heaviest, sets)
}

/// What a check of the search found: the sets of family A, the searches made, those of them with a source open, and
/// those whose answer plays a one-sided candidate.
#[derive(Clone, Copy, Debug, Default)]
struct Searched {
    sets: usize,
    searches: usize,
    with_open: usize,
    with_one_sided: usize,
}

impl Searched {
    fn add(&mut self, other: &Searched) {
        self.sets += other.sets;
        self.searches += other.searches;
        self.with_open += other.with_open;
        self.with_one_sided += other.with_one_sided;
    }
}

/// The search of Π⁺(`open`) of the board `1..=n`, in the formulation `claims` says, against the whole-board oracle: for
/// each of `tactics` (all of which must take the problem), the search on one thread, with no seed and no ceiling,
/// completes with a promotion sum s such that `in_the_bag` + s is the heaviest weight of F ∪ X over the sets X of
/// family A (F ∪ X playable), and is `optimal.json`'s score; the set it returns is a promotion set by the reference
/// predicate, and F together with it is playable.  The sets of family A are enumerated once for all the tactics.
fn check_the_search(n: u32, open: &[u32], claims: bool, tactics: &[SearchTactics]) -> Searched {
    let board = Board::<8>::for_n(n);
    let case = Case::new(&board, open, claims);
    let (heaviest, sets) = heaviest_playable(&case);
    let heaviest_total = sum(&case.fixed) + heaviest;
    let mut found = Searched { sets, ..Searched::default() };
    for &tactics in tactics {
        let what = format!("{}, {tactics:?}", case.at());
        let (ext, prepared) = board
            .prepare_extended_with(&Bits::from_slice(open), claims, tactics)
            .unwrap_or_else(|e| panic!("{what}: {e}"));
        let result = search(&prepared, &config(1), None, None, false).unwrap_or_else(|e| panic!("{what}: {e}"));
        assert!(result.completed, "{what}: the search did not complete");
        let score = ext.in_the_bag + result.sum;
        assert_eq!(score, heaviest_total, "{what}: the search's score and the heaviest set of the oracle");
        assert_eq!(Some(score), Optimal::built_in().score(n), "{what}: the search's score and the table's");
        let set = result.promotions.to_vec();
        assert_eq!(sum(&set), result.sum, "{what}: the set {set:?}");
        assert!(case.is_promotion_set(&set), "{what}: the set {set:?} is not a promotion set");
        assert!(case.playable_with(&set), "{what}: F with the set {set:?} is not playable");
        found.searches += 1;
        found.with_open += usize::from(!open.is_empty());
        found.with_one_sided += usize::from(set.iter().any(|&x| ext.one_sided.iter().any(|&(s, _)| s == x)));
    }
    found
}

/// [`check_the_search`] in the formulation `claims` says on each board of `boards`, for the open sets `choose` gives
/// for the board `n` from its U\*_{≥3}, with the time of each.  Returns what the boards made.
fn check_the_search_on_boards(
    boards: RangeInclusive<u32>,
    claims: bool,
    tactics: &[SearchTactics],
    mut choose: impl FnMut(u32, &[u32]) -> Vec<Vec<u32>>,
) -> Searched {
    let (start, mut total) = (Instant::now(), Searched::default());
    for n in boards.clone() {
        let upper_from_3 = upper_from_3(&Board::<8>::for_n(n));
        for open in choose(n, &upper_from_3) {
            let began = Instant::now();
            let found = check_the_search(n, &open, claims, tactics);
            println!(
                "n={n:>3} {} |O|={:>2}: {:>7} sets  {:.1?}",
                formulation(claims),
                open.len(),
                found.sets,
                began.elapsed()
            );
            total.add(&found);
        }
    }
    println!("the boards {boards:?}, {}: {total:?} in {:.1?}", formulation(claims), start.elapsed());
    total
}

/// The search of Π⁺(∅) on every board from 2 to 80, under the default tactics and those of extended mode (the search
/// is the frame model's, but for the pairs of `unzip` left in the frames, and takes either).
#[test]
fn the_search_with_no_open_source_finds_the_heaviest_set() {
    let both = [SearchTactics::default(), SearchTactics::extended()];
    let found = check_the_search_on_boards(2..=80, false, &both, |_, _| vec![Vec::new()]);
    assert_eq!(found.searches, 2 * 79);
}

/// [`the_search_with_no_open_source_finds_the_heaviest_set`] with the claims of `unzip`: the search is then the frame
/// model's own.
#[test]
fn the_search_with_no_open_source_finds_the_heaviest_set_with_claims() {
    let both = [SearchTactics::default(), SearchTactics::extended()];
    let found = check_the_search_on_boards(2..=80, true, &both, |_, _| vec![Vec::new()]);
    assert_eq!(found.searches, 2 * 79);
}

/// The search of Π⁺(O) on every board from 2 to 80 that has sources of three or more prime factors, for three random
/// subsets O of them with 1 to 8 members each, under the tactics of extended mode.
#[test]
fn the_search_with_open_sources_finds_the_heaviest_set() {
    let mut rng = Rng(0x2C1B_3C6D_9E37_79B9);
    let found = check_the_search_on_boards(2..=80, false, &[SearchTactics::extended()], |_, from_3| {
        random_open_sets(&mut rng, from_3, 3, 8)
    });
    assert!(found.with_open > 100, "{found:?}");
    assert!(found.with_one_sided * 2 > found.with_open, "most answers play an open source: {found:?}");
}

/// [`the_search_with_open_sources_finds_the_heaviest_set`] with the claims of `unzip`, for other random subsets.
#[test]
fn the_search_with_open_sources_finds_the_heaviest_set_with_claims() {
    let mut rng = Rng(0x7A6D_5E2F_3C4B_1A09);
    let found = check_the_search_on_boards(2..=80, true, &[SearchTactics::extended()], |_, from_3| {
        random_open_sets(&mut rng, from_3, 3, 8)
    });
    assert!(found.with_open > 100, "{found:?}");
    assert!(found.with_one_sided * 2 > found.with_open, "most answers play an open source: {found:?}");
}

/// The search of Π⁺(U\*_{≥3}), every source of three or more prime factors open, on every board from 2 to 60.
#[test]
fn the_search_with_every_source_open_finds_the_heaviest_set() {
    let found =
        check_the_search_on_boards(2..=60, false, &[SearchTactics::extended()], |_, from_3| vec![from_3.to_vec()]);
    assert!(found.with_open > 20, "{found:?}");
    assert!(found.with_one_sided * 2 > found.with_open, "most answers play an open source: {found:?}");
}

/// [`the_search_with_every_source_open_finds_the_heaviest_set`] with the claims of `unzip`.
#[test]
fn the_search_with_every_source_open_finds_the_heaviest_set_with_claims() {
    let found =
        check_the_search_on_boards(2..=60, true, &[SearchTactics::extended()], |_, from_3| vec![from_3.to_vec()]);
    assert!(found.with_open > 20, "{found:?}");
    assert!(found.with_one_sided * 2 > found.with_open, "most answers play an open source: {found:?}");
}

/// [`the_search_with_every_source_open_finds_the_heaviest_set`] on the boards 61 to 69, whose sets (0.4 to 5.5 million
/// each, 21 million in all) are enumerated for the oracle's answer, as in
/// [`theorem_m_plus_with_every_source_open_61_to_69`], in eighteen seconds.  The boards 70 and 71 have 62 million each.
#[test]
#[ignore = "takes twenty seconds; the boards to 60 are in the default test"]
fn the_search_with_every_source_open_finds_the_heaviest_set_61_to_69() {
    let found =
        check_the_search_on_boards(61..=69, false, &[SearchTactics::extended()], |_, from_3| vec![from_3.to_vec()]);
    assert!(found.with_open > 0 && found.with_one_sided * 2 > found.with_open, "{found:?}");
}

/// [`the_search_with_every_source_open_finds_the_heaviest_set_61_to_69`] with the claims of `unzip`.
#[test]
#[ignore = "takes twenty seconds; the boards to 60 are in the default test"]
fn the_search_with_every_source_open_finds_the_heaviest_set_61_to_69_with_claims() {
    let found =
        check_the_search_on_boards(61..=69, true, &[SearchTactics::extended()], |_, from_3| vec![from_3.to_vec()]);
    assert!(found.with_open > 0 && found.with_one_sided * 2 > found.with_open, "{found:?}");
}
