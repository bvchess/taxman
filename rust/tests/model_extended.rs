//! Theorem M⁺, the extended problem Π⁺(O), against the code.
//!
//! Π⁺(O) (`taxman::frames::build_extended`, whose module documentation defines it) is the frame model with a set O of
//! the sources of three or more prime factors that setup keeps made optional, with nothing settled by `unzip`, and
//! with a reference predicate for its promotion sets, `ExtendedFrames::promotable`, that peels the unreduced frames.
//! The theorem (heuristics/THEORY.md, "The extended problem"):
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
//!   an ignored one to 69) both directions are checked on every set, for O empty, for random subsets of U\*_{≥3} and
//!   for O = U\*_{≥3}: the sets X = K ∪ L with F ∪ X playable (family A, found depth first), all promotion sets
//!   (family B, found the same way), and the two families must be equal; the heaviest set weighs what the table of
//!   optimal games has for n, and the oracle's order for it is a legal game of that score.
//! * With O empty, Π⁺(∅) has the promotion sets of the frame model: those of Π_n (`Board::prepare`) and those of the
//!   problem the search is given for Π⁺(∅) (`Board::prepare_extended`).
//! * On larger boards (to 1300 by default, to 4095 in an ignored test) the sets are random: promotion sets made by
//!   adding candidates in a random order, and maximal playable sets made by adding numbers in a random order, for O
//!   a random 20 members of U\*_{≥3}, for O = U\*_{≥3} and for O empty, where random promotion sets of the frame model
//!   and of Π⁺(∅) are promotion sets of each other.  Greedy rejects a source of layer 3 or more on few boards (374 at
//!   741, the first), and only the boards of the test for them (741, 1100, 1200) put a rejected source into a set.

mod common;

use common::{for_each_kept_subset, kept_subsets, play_order, playable, shuffle, Rng};
use std::collections::BTreeSet;
use std::ops::RangeInclusive;
use std::time::Instant;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::frames::{build_extended, BoardFrames, ExtendedFrames};
use taxman::game::{self, Optimal};
use taxman::model::Scratch;
use taxman::search::{Prepared, SearchTactics};
use taxman::with_width;

// ---------------------------------------------------------------------------------------------------------
// A board and its extended problem

/// The sum of `numbers`.
fn sum(numbers: &[u32]) -> u64 {
    numbers.iter().map(|&x| u64::from(x)).sum()
}

/// U\*: the numbers of the game with no promotions above n/2, ascending.
fn upper<const W: usize>(board: &Board<W>) -> Vec<u32> {
    let mut upper: Vec<u32> = board.plain.iter().copied().filter(|&x| 2 * x > board.n()).collect();
    upper.sort_unstable();
    upper
}

/// U\*_{≥3}: the members of U\* with three or more prime factors, ascending.
fn upper_from_3<const W: usize>(board: &Board<W>) -> Vec<u32> {
    upper(board).into_iter().filter(|&x| board.tables.omega(x) >= 3).collect()
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
}

impl<'b, const W: usize> Case<'b, W> {
    fn new(board: &'b Board<W>, open: &[u32]) -> Self {
        let n = board.n();
        let fixed = upper(board).into_iter().filter(|x| !open.contains(x)).collect();
        let extended = build_extended::<W>(&board.tables, &Bits::from_slice(open))
            .unwrap_or_else(|e| panic!("n={n}: O={open:?}: {e}"));
        let components = BoardFrames::new(&board.tables, Bits::EMPTY);
        Case { board, components, extended, fixed, open: open.to_vec() }
    }

    fn n(&self) -> u32 {
        self.board.n()
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
        format!("n={}, O={:?}: S = F ∪ {x:?}, K = {:?}, L = {:?}", self.n(), self.open, k.to_vec(), l.to_vec())
    }
}

// ---------------------------------------------------------------------------------------------------------
// Every set, on small boards

/// Theorem M⁺ on the board `1..=n` for the open sources `open`, on every set: the sets X of family A (F ∪ X playable,
/// found depth first) are promotion sets, the promotion sets of family B (found the same way) make F ∪ X playable,
/// the two families are equal, `in_the_bag` is ΣF, the heaviest ΣS is what the table of optimal games has for `n`, and
/// the oracle's order for the heaviest S replays to its sum.  Returns the number of sets.
///
/// The sets are visited as the searches find them and not kept.  Both families are closed under subsets, so each
/// search finds every set of its family; every set of A asserted a promotion set puts A in B, and every set of B
/// asserted playable with F puts B in A: the families are equal (and so are their sizes, which are compared too).
fn check_every_set(n: u32, open: &[u32]) -> usize {
    let board = Board::<8>::for_n(n);
    let case = Case::new(&board, open);
    assert_eq!(case.extended.in_the_bag, sum(&case.fixed), "n={n}, O={open:?}: in the bag");
    assert!(case.playable_with(&[]), "n={n}, O={open:?}: F is not playable");

    let (mut in_a, mut heaviest) = (0, Vec::new());
    for_each_kept_subset(&case.items_a(), &mut |x| case.playable_with(x), &mut |x| {
        assert!(case.is_promotion_set(x), "S is playable but (K, L) is not a promotion set: {}", case.describe(x));
        in_a += 1;
        if sum(x) > sum(&heaviest) {
            heaviest = x.to_vec();
        }
    });
    let mut in_b = 0;
    for_each_kept_subset(&case.items_b(), &mut |x| case.is_promotion_set(x), &mut |x| {
        assert!(case.playable_with(x), "(K, L) is a promotion set but S is not playable: {}", case.describe(x));
        in_b += 1;
    });
    assert_eq!(in_a, in_b, "n={n}, O={open:?}: the families differ in size");

    let total = case.extended.in_the_bag + sum(&heaviest);
    assert_eq!(Some(total), Optimal::built_in().score(n), "n={n}, O={open:?}: the heaviest S and the table");
    let set = [case.fixed.as_slice(), &heaviest].concat();
    let order = play_order(&board.tables, &set).expect("the heaviest set is playable");
    assert_eq!(game::replay(n, &order), Ok(total), "{}: the oracle's order {order:?}", case.describe(&heaviest));
    in_a
}

/// [`check_every_set`] on each board of `boards`, for the open sets `choose` gives for the board `n` from its
/// U\*_{≥3}, with the time of each.  There are sets other than the empty one on some board.
fn check_every_board(boards: RangeInclusive<u32>, mut choose: impl FnMut(u32, &[u32]) -> Vec<Vec<u32>>) {
    let (start, mut total, mut checks) = (Instant::now(), 0, 0);
    for n in boards.clone() {
        let upper_from_3 = upper_from_3(&Board::<8>::for_n(n));
        for open in choose(n, &upper_from_3) {
            let began = Instant::now();
            let sets = check_every_set(n, &open);
            println!("n={n:>3} |O|={:>2}: {sets:>7} sets  {:.1?}", open.len(), began.elapsed());
            total += sets;
            checks += 1;
        }
    }
    println!("the boards {boards:?}: {checks} open sets, {total} sets in {:.1?}", start.elapsed());
    assert!(total > checks, "no board has a set but the empty one");
}

/// Three random subsets of `from_3` with 1 to 8 members (none when it is empty), the stream being `rng`.
fn random_open_sets(rng: &mut Rng, from_3: &[u32]) -> Vec<Vec<u32>> {
    let most = from_3.len().min(8) as u64;
    (0..3.min(most))
        .map(|_| {
            let mut pool = from_3.to_vec();
            shuffle(rng, &mut pool);
            pool.truncate(1 + rng.below(most) as usize);
            pool.sort_unstable();
            pool
        })
        .collect()
}

/// Theorem M⁺ with no open source, on every set of every board from 2 to 80.
#[test]
fn theorem_m_plus_with_no_open_source() {
    check_every_board(2..=80, |_, _| vec![Vec::new()]);
}

/// Theorem M⁺ with three random subsets of U\*_{≥3} open, on every set of every board from 2 to 80 that has such
/// sources.
#[test]
fn theorem_m_plus_with_open_sources() {
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    check_every_board(2..=80, |_, from_3| random_open_sets(&mut rng, from_3));
}

/// Theorem M⁺ with every source of three or more prime factors open, on every set of every board from 2 to 60.
#[test]
fn theorem_m_plus_with_every_source_open() {
    check_every_board(2..=60, |_, from_3| vec![from_3.to_vec()]);
}

/// [`theorem_m_plus_with_no_open_source`] and [`theorem_m_plus_with_open_sources`] on the boards 81 to 100.
#[test]
#[ignore = "takes twenty seconds; the boards to 80 are in the default tests"]
fn theorem_m_plus_with_open_sources_81_to_100() {
    check_every_board(81..=100, |_, _| vec![Vec::new()]);
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    check_every_board(81..=100, |_, from_3| random_open_sets(&mut rng, from_3));
}

/// [`theorem_m_plus_with_every_source_open`] on the boards 61 to 69, which have 0.4 to 5.5 million sets each and take
/// a minute in all.  The boards 70 and 71 have 62 million sets each (3.5 minutes each), and the numbers go on growing.
#[test]
#[ignore = "takes a minute; the boards to 60 are in the default test"]
fn theorem_m_plus_with_every_source_open_61_to_69() {
    check_every_board(61..=69, |_, from_3| vec![from_3.to_vec()]);
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
/// (`Board::prepare_extended`: the reduced root, the model built from `columns`).
#[test]
fn the_extended_problem_with_no_open_source_is_the_frame_model() {
    let (start, mut total) = (Instant::now(), 0);
    for n in 2..=100 {
        let board = Board::<8>::for_n(n);
        let case = Case::new(&board, &[]);
        assert!(case.extended.one_sided.is_empty(), "n={n}: a one-sided candidate");
        let extended: BTreeSet<Vec<u32>> = kept_subsets(&case.items_b(), &mut |x| case.is_promotion_set(x));

        let plain = board.prepare(SearchTactics::default()).unwrap_or_else(|e| panic!("n={n}: {e}"));
        let in_the_frame_model = promotion_sets_of(&plain);
        let (_, searched) = board.prepare_extended(SearchTactics::default()).unwrap_or_else(|e| panic!("n={n}: {e}"));
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
    println!("the boards 2..=100: {total} promotion sets in {:.1?}", start.elapsed());
    assert!(total > 99, "no board has a promotion set but the empty one");
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
    assert!(case.playable_with(&[]), "n={}, O={:?}: F is not playable", case.n(), case.open);
    for _ in 0..sets {
        let x = random_promotion_set(case, rng);
        assert!(case.playable_with(&x), "(K, L) is a promotion set but S is not playable: {}", case.describe(&x));
        tally.promotion_sets += 1;
        tally.promotion_members += x.len();
        tally.with_rejected += usize::from(x.iter().any(|&s| rejected.contains(s)));
    }
    for _ in 0..sets {
        let x = random_maximal_playable_set(case, rng);
        assert!(case.is_promotion_set(&x), "S is playable but (K, L) is not a promotion set: {}", case.describe(&x));
        tally.maximal_sets += 1;
        tally.maximal_members += x.len();
        tally.with_rejected += usize::from(x.iter().any(|&s| rejected.contains(s)));
    }
    tally
}

/// With no open source, random promotion sets of the frame model (`Board::prepare`) are promotion sets of Π⁺(∅), and
/// random promotion sets of Π⁺(∅) are promotion sets of the frame model.
fn check_random_sets_against_the_frame_model<const W: usize>(board: &Board<W>, sets: usize, rng: &mut Rng) {
    let (n, case) = (board.n(), Case::new(board, &[]));
    let p = board.prepare(SearchTactics::default()).unwrap_or_else(|e| panic!("n={n}: {e}"));
    assert_eq!(case.extended.candidates, p.candidates, "n={n}: the candidates");
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

/// Theorem M⁺ on the board `1..=n` at width `W`, on `sets` random sets of each kind for a random 20 members of
/// U\*_{≥3} open, for all of them open, and for none open, where the frame model is checked against Π⁺(∅) too.
fn check_random_board<const W: usize>(n: u32, sets: usize, rng: &mut Rng) -> Tally {
    let board = Board::<W>::for_n(n);
    let every = upper_from_3(&board);
    let mut random = every.clone();
    shuffle(rng, &mut random);
    random.truncate(20);
    random.sort_unstable();

    let mut tally = Tally::default();
    for open in [random, every, Vec::new()] {
        let start = Instant::now();
        let case = Case::new(&board, &open);
        let found = check_random_sets(&case, sets, rng);
        if open.is_empty() {
            check_random_sets_against_the_frame_model(&board, sets, rng);
        }
        println!(
            "n={n:>5} W={W:>3} |O|={:>3} |R|={:>2}  {} candidates, {} one-sided  mean |X|: promotion sets {:>6.1}, \
             maximal sets {:>6.1}  with a rejected source: {}  {:.1?}",
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

/// Theorem M⁺ on `sets` random sets of each kind on each of `boards`, each at the width the solver picks, from the
/// random stream seeded with `seed`.
fn check_random_boards(boards: &[u32], sets: usize, seed: u64) -> Tally {
    let mut rng = Rng(seed);
    let (start, mut tally) = (Instant::now(), Tally::default());
    for &n in boards {
        tally.add(&with_width!(n, W => check_random_board::<W>(n, sets, &mut rng)));
    }
    println!("{} boards, {sets} sets of each kind: {:.1?}", boards.len(), start.elapsed());
    // A check of tiny sets would show: the sets must not all be empty.
    assert!(tally.promotion_members > 0 && tally.maximal_members > 0, "only empty sets: {tally:?}");
    tally
}

/// Theorem M⁺ on random sets of the boards 300, 600, 1000 and 1300.
#[test]
fn theorem_m_plus_on_random_sets() {
    check_random_boards(&[300, 600, 1000, 1300], 100, 0xA076_1D64_78BD_642F);
}

/// Theorem M⁺ on random sets of the boards 2000, 3000 and 4095.
#[test]
#[ignore = "takes a while; the boards to 1300 are in the default test"]
fn theorem_m_plus_on_random_sets_past_1300() {
    check_random_boards(&[2000, 3000, 4095], 100, 0xE703_7ED1_A0B4_28DB);
}

/// Theorem M⁺ on random sets of boards where greedy rejects a source of layer 3, which the opening of a source above
/// it can let in: 741 (374 is rejected), 1100 (578) and 1200 (605).  Greedy rejects no source of layer 3 or more on
/// the boards to 740, nor on 1000 or 1300, so the other tests never meet one; some of the sets here have one in them.
#[test]
fn theorem_m_plus_with_a_rejected_source() {
    let tally = check_random_boards(&[741, 1100, 1200], 100, 0x8EBC_6AF0_9C88_C6E3);
    assert!(tally.with_rejected > 0, "no set has a rejected source: {tally:?}");
}
