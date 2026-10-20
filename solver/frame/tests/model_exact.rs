//! Theorem M, the frame model's exactness, against the code.
//!
//! The theorem (docs/theory/frame-model.md, "The frame model is exact, given its upper part"):
//!
//! > Let U\* be the sources that setup keeps (the numbers above n/2 of the game with no promotions, `board.plain`), and
//! > Π_n the root problem the search is given (`Board::prepare`: the model, the root frame set, the candidates and
//! > `max_total`).  For every set L of numbers up to n/2 (the frames being numbered by layer, so that every frame with
//! > freedoms directly in front of a frame numbered h is numbered h − 1): U\* ∪ L is playable (some order of it is a legal game) if and only if L is a promotion set of
//! > Π_n (L ⊆ candidates, L can be promoted from the root, |L| ≤ `max_total`), and then the game the promotions make
//! > (`accelerate::solution_for_promotions`) scores `in_the_bag` + ΣL.
//!
//! The "if" direction (soundness: every promotion set is a game) is what every search relies on when it offers an
//! incumbent.  The "only if" direction (completeness: no game with upper part U\* is missing from the search) is the
//! one that no other test touches.  Playability is decided on the whole board, by `common::playable` (the peel of
//! THEORY.md, Theorems 1 to 3), with code that shares nothing with `frames` or `model`.
//!
//! * On every board up to 100 (and, in an ignored test, 101 to 120) both directions are checked on every set: the
//!   sets L with U\* ∪ L playable, found by depth-first search, are all promotion sets and make games of the right
//!   score, the promotion sets, found the same way, are all playable with U\*, and the two families are equal.
//! * On larger boards (to 1300 by default, to 8191 in an ignored test) the sets are random: promotion sets made by
//!   promoting candidates in a random order, and maximal playable extensions of U\* made by trying the numbers up to
//!   n/2 in a random order.
//! * The candidate filter is needed: a back claim of `unzip` hides a use of its factor as tax by a number that could be
//!   promoted into the frame, and only the filter keeps that number out of the candidates (the tests at the end).
//!
//! The oracle is itself tested first, against the rules of the game, the brute force and the table of optimal games.

mod common;

use common::{kept_subsets, play_order, playable, shuffle, Rng, QUICK};
use std::collections::BTreeSet;
use std::time::Instant;
use taxman::accelerate::solution_for_promotions;
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::brute::{self, Rule};
use taxman::frames::ClaimKind;
use taxman::game::{self, Optimal};
use taxman::model::Scratch;
use taxman::numtheory::Tables;
use taxman::search::{Prepared, SearchTactics};
use taxman::with_width;

// ---------------------------------------------------------------------------------------------------------
// The oracle against the rules

/// For every subset of `2..=n` (bit `i` of the index is the number `i + 2`), whether it is the set of moves of some
/// legal game, from the rules alone: whatever the order, the pot after a set of moves is the board less the moves and
/// all their divisors, so a set is the moves of a legal game exactly when one of its members can come last (it is in
/// the pot after the others, and so is some proper divisor of it) after the rest.  Board masks have the number `m` at
/// bit `m − 1`.
fn playable_by_the_rules(n: u32) -> Vec<bool> {
    let bit = |m: u32| 1u32 << (m - 1);
    let proper_divisors = |m: u32| (1..m).filter(|&d| m.is_multiple_of(d)).fold(0, |mask, d| mask | bit(d));
    let board = (1u32 << n) - 1;
    let size = 1usize << (n - 1);
    let mut swept = vec![0u32; size];
    let mut reachable = vec![false; size];
    reachable[0] = true;
    for moves in 1..size {
        let lowest = moves.trailing_zeros() + 2;
        swept[moves] = swept[moves & (moves - 1)] | bit(lowest) | proper_divisors(lowest);
        reachable[moves] = (0..n - 1).filter(|i| moves >> i & 1 == 1).any(|i| {
            let (last, rest) = (i + 2, moves & !(1 << i));
            let pot = board & !swept[rest];
            reachable[rest] && pot & bit(last) != 0 && pot & proper_divisors(last) != 0
        });
    }
    reachable
}

/// The oracle decides playability as the rules do, on every board up to 20: for every subset of the numbers it agrees
/// with the rules' own account of which sets are the moves of a legal game, every playable set in the order it gives
/// is a legal game that scores the sum of the set, and the largest sum is the optimal score (the brute force's, and
/// the table's).  (A quick run stops at 17.)
#[test]
fn the_oracle_agrees_with_brute_force() {
    let optimal = Optimal::built_in();
    // The sets of a board are 2^(n−1): a quick run stops at 17, an eighth of the work.
    for n in 2..=if QUICK { 17 } else { 20 } {
        let t = Tables::new(n);
        let by_the_rules = playable_by_the_rules(n);
        for (moves, &legal) in by_the_rules.iter().enumerate() {
            let set: Vec<u32> = (0..n - 1).filter(|i| moves >> i & 1 == 1).map(|i| i + 2).collect();
            assert_eq!(playable(&t, &set), legal, "n={n}: the set {set:?}");
        }

        let numbers: Vec<u32> = (2..=n).collect();
        let sets = kept_subsets(&numbers, &mut |set| playable(&t, set));
        assert_eq!(sets.len(), by_the_rules.iter().filter(|&&legal| legal).count(), "n={n}: how many playable sets");
        let mut best = 0;
        for set in &sets {
            let order = play_order(&t, set).expect("a playable set has an order");
            let sum = set.iter().map(|&x| u64::from(x)).sum::<u64>();
            assert_eq!(game::replay(n, &order), Ok(sum), "n={n}: the set {set:?} in the order {order:?}");
            best = best.max(sum);
        }
        assert_eq!(
            best,
            brute::solve(n, &[], Rule::Divisor).score,
            "n={n}: the largest playable sum and the brute force"
        );
        assert_eq!(Some(best), optimal.score(n), "n={n}: the largest playable sum and the table");
    }
}

// ---------------------------------------------------------------------------------------------------------
// A board and its problem

/// A board with what the theorem is about: U\*, and the problem Π_n the search is given.
struct Case<const W: usize> {
    board: Board<W>,
    upper: Vec<u32>,
    prepared: Prepared<W>,
}

impl<const W: usize> Case<W> {
    fn new(n: u32) -> Self {
        let board = Board::<W>::for_n(n);
        let upper = board.plain.iter().copied().filter(|&x| 2 * x > n).collect();
        let prepared = board.prepare(SearchTactics::default()).unwrap_or_else(|e| panic!("n={n}: {e}"));
        Case { board, upper, prepared }
    }

    fn n(&self) -> u32 {
        self.board.n()
    }

    /// Whether U\* ∪ `lower` is playable, by the oracle.
    fn playable_with(&self, lower: &[u32]) -> bool {
        playable(&self.board.tables, &self.with_upper(lower))
    }

    /// U\* ∪ `lower`.
    fn with_upper(&self, lower: &[u32]) -> Vec<u32> {
        [self.upper.as_slice(), lower].concat()
    }

    /// Whether `lower` is a promotion set of Π_n: made of candidates, promotable from the root, at most `max_total`
    /// members.
    fn is_promotion_set(&self, scratch: &mut Scratch<W>, lower: &[u32]) -> bool {
        let (p, set) = (&self.prepared, Bits::<W>::from_slice(lower));
        set.is_subset(&p.candidates) && lower.len() as u32 <= p.max_total && self.promotable(scratch, &set)
    }

    /// Whether the model can promote all of `set` from the root.
    fn promotable(&self, scratch: &mut Scratch<W>, set: &Bits<W>) -> bool {
        self.prepared.model.can_promote_all_using(scratch, &self.prepared.root, set)
    }

    /// Asserts that `lower` is a promotion set of Π_n, one part at a time.
    fn assert_promotion_set(&self, scratch: &mut Scratch<W>, lower: &[u32], what: &str) {
        let (n, p, set) = (self.n(), &self.prepared, Bits::<W>::from_slice(lower));
        assert!(set.is_subset(&p.candidates), "n={n}: {what}: L={lower:?} is not made of candidates");
        assert!(self.promotable(scratch, &set), "n={n}: {what}: L={lower:?} cannot be promoted from the root");
        assert!(lower.len() as u32 <= p.max_total, "n={n}: {what}: |L|={} > max_total={}", lower.len(), p.max_total);
    }

    /// Asserts that the game the promotions `lower` make is legal and scores `in_the_bag` + ΣL.
    fn assert_game_scores(&self, lower: &[u32], what: &str) {
        let n = self.n();
        let moves = solution_for_promotions::<W>(&self.board.tables, Bits::from_slice(lower));
        let expected = self.board.in_the_bag + lower.iter().map(|&x| u64::from(x)).sum::<u64>();
        assert_eq!(game::replay(n, &moves), Ok(expected), "n={n}: {what}: the game of L={lower:?}: {moves:?}");
    }
}

// ---------------------------------------------------------------------------------------------------------
// Every set, on small boards

/// Theorem M on the board `1..=n`, on every set: the sets L with U\* ∪ L playable (family A) and the promotion sets
/// (family B), each found by depth first search, are equal; the sets of A are promotion sets that make games of the
/// right score; the sets of B are playable with U\*; and the largest ΣL is what the table of optimal games has for `n`
/// less `in_the_bag`.  Returns the number of sets.
fn check_every_set(n: u32) -> usize {
    let case = Case::<8>::new(n);
    let mut scratch = Scratch::new();

    let lower_numbers: Vec<u32> = (2..=n / 2).collect();
    let family_a = kept_subsets(&lower_numbers, &mut |lower| case.playable_with(lower));
    for lower in &family_a {
        case.assert_promotion_set(&mut scratch, lower, "L with U* ∪ L playable");
        case.assert_game_scores(lower, "L with U* ∪ L playable");
    }

    let candidates = case.prepared.candidates.to_vec();
    let family_b = kept_subsets(&candidates, &mut |lower| case.is_promotion_set(&mut scratch, lower));
    for lower in &family_b {
        assert!(case.playable_with(lower), "n={n}: the promotion set L={lower:?} makes U* ∪ L unplayable");
    }

    let difference: Vec<_> = family_a.symmetric_difference(&family_b).take(3).collect();
    assert!(
        difference.is_empty(),
        "n={n}: the families differ, {} against {}: {difference:?}",
        family_a.len(),
        family_b.len()
    );
    let best =
        family_a.iter().map(|lower| lower.iter().map(|&x| u64::from(x)).sum::<u64>()).max().expect("the empty L");
    assert_eq!(Some(best + case.board.in_the_bag), Optimal::built_in().score(n), "n={n}: the largest ΣL and the table");
    family_a.len()
}

/// [`check_every_set`] on each board of `boards`, with the time of each.  There is a set on every board (the empty
/// one), and for the check to mean something there must be others on some.
fn check_every_board(boards: std::ops::RangeInclusive<u32>) {
    let (start, mut total) = (Instant::now(), 0);
    for n in boards.clone() {
        let began = Instant::now();
        let sets = check_every_set(n);
        println!("n={n:>3}: {sets:>6} sets  {:.1?}", began.elapsed());
        total += sets;
    }
    println!("the boards {boards:?}: {total} sets in {:.1?}", start.elapsed());
    assert!(total > boards.count(), "no board has a set but the empty one");
}

/// Theorem M on every set of every board from 2 to 100.  Below 70 few numbers can be promoted (the boards to 50 have
/// 318 sets in all); the boards 98 to 100 hold the first near misses (below).
#[test]
fn theorem_m_on_every_board_to_100() {
    check_every_board(2..=100);
}

/// Theorem M on every set of every board from 101 to 120, where the sets become many (75,000 at 114).
#[test]
#[ignore = "takes ten seconds; the boards to 100 are in the default test"]
fn theorem_m_on_every_board_101_to_120() {
    check_every_board(101..=120);
}

// ---------------------------------------------------------------------------------------------------------
// Random sets, on larger boards

/// How many sets of each kind a board's check made, and how many members they had in all.
#[derive(Clone, Copy, Debug, Default)]
struct Tally {
    promotion_sets: usize,
    promotion_members: usize,
    extensions: usize,
    extension_members: usize,
}

impl Tally {
    fn mean(members: usize, sets: usize) -> f64 {
        members as f64 / sets.max(1) as f64
    }
}

/// Theorem M on the board `1..=n` at width `W`, on `sets` random sets of each of two kinds.
///
/// *Soundness*: a random promotion set (the candidates in a random order, promoted one after the other from the root
/// where they still can be, until a length drawn at random from `0..=max_total` is reached) is at most `max_total`
/// long, can be promoted from the root, is playable with U\*, and makes a game that scores `in_the_bag` + ΣL.
///
/// *Completeness*: a random maximal playable extension of U\* (the numbers up to n/2 in a random order, each kept if
/// U\* with the ones kept and it is still playable) is made of candidates, can be promoted from the root, is at most
/// `max_total` long, and (for the first `REPLAYED` of them) makes a game that scores `in_the_bag` + ΣL.
fn check_random<const W: usize>(n: u32, sets: usize, rng: &mut Rng) -> Tally {
    /// How many of a board's maximal playable extensions are also turned into games and replayed: making the game of
    /// a promotion set forms the board's frames again, which costs far more than the checks of the model.
    const REPLAYED: usize = 10;
    let start = Instant::now();
    let case = Case::<W>::new(n);
    let (p, t) = (&case.prepared, &case.board.tables);
    let mut scratch = Scratch::new();
    let mut tally = Tally::default();
    assert!(playable(t, &case.upper), "n={n}: U* is not playable");

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
        case.assert_promotion_set(&mut scratch, &lower, "a random promotion set");
        assert!(case.playable_with(&lower), "n={n}: the promotion set L={lower:?} makes U* ∪ L unplayable");
        case.assert_game_scores(&lower, "a random promotion set");
        tally.promotion_sets += 1;
        tally.promotion_members += lower.len();
    }

    for i in 0..sets {
        let mut order: Vec<u32> = (2..=n / 2).collect();
        shuffle(rng, &mut order);
        let mut set = case.upper.clone();
        for &x in &order {
            set.push(x);
            if !playable(t, &set) {
                set.pop();
            }
        }
        let mut lower = set[case.upper.len()..].to_vec();
        lower.sort_unstable();
        case.assert_promotion_set(&mut scratch, &lower, "a maximal playable extension of U*");
        if i < REPLAYED {
            case.assert_game_scores(&lower, "a maximal playable extension of U*");
            // The oracle's own order for the whole set is a legal game of the right score too.
            let order = play_order(t, &set).expect("the extension is playable");
            let sum = set.iter().map(|&x| u64::from(x)).sum::<u64>();
            assert_eq!(game::replay(n, &order), Ok(sum), "n={n}: the oracle's order for U* ∪ L, L={lower:?}");
        }
        tally.extensions += 1;
        tally.extension_members += lower.len();
    }

    println!(
        "n={n:>5} W={W:>3}  {} candidates, max_total {:>4}  mean |L|: promotion sets {:>6.1}, extensions {:>6.1}  \
         {:.1?}",
        p.candidates.len(),
        p.max_total,
        Tally::mean(tally.promotion_members, tally.promotion_sets),
        Tally::mean(tally.extension_members, tally.extensions),
        start.elapsed(),
    );
    tally
}

/// Theorem M on `sets` random sets of each kind on each of `boards`, each at the width the solver picks, from the
/// random stream seeded with `seed`.
fn check_random_boards(boards: &[u32], sets: usize, seed: u64) {
    let mut rng = Rng(seed);
    let start = Instant::now();
    for &n in boards {
        let tally = with_width!(n, W => check_random::<W>(n, sets, &mut rng));
        // A check of tiny sets would show: the sets must not all be empty.
        assert!(tally.promotion_members > 0 && tally.extension_members > 0, "n={n}: only empty sets: {tally:?}");
    }
    println!("{} boards, {sets} sets of each kind: {:.1?}", boards.len(), start.elapsed());
}

/// Theorem M on a few random sets of the boards 60 to 500.
#[test]
fn theorem_m_on_a_few_random_sets_to_500() {
    check_random_boards(&[60, 100, 200, 300, 500], 20, 0xE220_A839_7B1D_CDAF);
}

/// Theorem M on random sets of the boards from 60 to 1300.
#[test]
#[cfg_attr(debug_assertions, ignore = "long: the quick check is theorem_m_on_a_few_random_sets_to_500")]
fn theorem_m_on_random_sets_to_1300() {
    check_random_boards(&[60, 100, 200, 300, 500, 760, 1000, 1300], 100, 0x9E37_79B9_7F4A_7C15);
}

/// Theorem M on random sets of the boards from 2000 to 8191, 10 sets of each kind.
#[test]
#[ignore = "takes a while; the boards to 1300 are in the default test"]
fn theorem_m_on_random_sets_past_1300() {
    check_random_boards(&[2000, 3000, 4095, 6000, 8191], 10, 0xD1B5_4A32_D192_ED03);
}

// ---------------------------------------------------------------------------------------------------------
// The near miss: why the candidate filter is needed

/// The smallest board where only the candidate filter stands between the model and an unsound promotion, and the back
/// claim and the number it involves, as `(n, s, f, c)`.  [`the_scan_for_near_misses`] finds it.
const NEAR_MISS: (u32, u32, u32, u32) = (98, 77, 11, 22);

/// The numbers that make the cycle of [`NEAR_MISS`]: `c` and seven members of U\*.
const NEAR_MISS_CYCLE: [u32; 8] = [22, 57, 62, 65, 77, 91, 93, 95];

/// A back claim of `unzip` hands a factor `f` to the one source `s` that wants it, and the frame the search takes no
/// longer shows either.  A number `c` up to n/2 with `f` among its maximal factors (`c = f·p`) that could be promoted
/// into `f`'s frame would want `f` too, but the model would see `c` paid by its other factor, and the real game would
/// have a cycle through `s` and `c`.  The candidate filter (which looks at the frame with the claim in it) keeps such a
/// `c` out of the candidates.
///
/// At n = 98 the source `s` = 77 (maximal factors 7 and 11) has the back claim of `f` = 11, and `c` = 22 (maximal
/// factors 2 and 11) is a factor of the frame 1 of the root.  Frame 0 has the six sources 57, 62, 65, 91, 93 and 95 on
/// the factors 2, 3, 5, 7, 13, 19 and 31, a path with one freedom.  The model would take 22 into it with the factor 2,
/// and the frame stays winnable.  On the board, though, 22 and 77 close the path into a cycle on eight factors: the
/// eight numbers list each of the eight factors 2, 3, 5, 7, 11, 13, 19 and 31 twice, so no member has one live factor
/// and no factor one live member, and the peel of the eight stalls (THEORY.md, Theorem 3): no order of them is a game,
/// and all of them but 22 are in U\*.  So 22 is no candidate (Theorem M's "only if" needs that: U\* ∪ {22} is
/// unplayable), the model alone would accept it (so the "if" would fail without the filter), and the filter is the
/// only guard there.
#[test]
fn the_candidate_filter_keeps_out_what_a_back_claim_hides() {
    let (n, s, f, c) = NEAR_MISS;
    let case = Case::<8>::new(n);
    let (p, t) = (&case.prepared, &case.board.tables);
    let mut scratch = Scratch::new();

    assert!(case.board.frames.claims.iter().flatten().any(|&claim| claim == (s, f, ClaimKind::Back)), "no back claim");
    assert!(case.upper.contains(&s), "the source {s} is not in U*");
    assert!(t.max_composites[f as usize].contains(&c) && 2 * c <= n, "{c} is not a number up to n/2 with {f} to pay");
    assert!(p.model.home(c) >= 1 && p.model.contains_factor(&p.root, c), "{c} is not a factor of the root");
    assert!(!p.candidates.contains(c), "{c} is a candidate");
    assert!(case.promotable(&mut scratch, &Bits::from_slice(&[c])), "the model would not accept {c}");
    assert!(case.playable_with(&[]), "U* is not playable");
    assert!(!case.playable_with(&[c]), "U* ∪ {{{c}}} is playable");

    // The cycle: all of it but `c` is in U*, and no order of it is a game.
    assert!(NEAR_MISS_CYCLE.contains(&c) && NEAR_MISS_CYCLE.contains(&s), "the cycle lacks {c} or {s}");
    assert!(NEAR_MISS_CYCLE.iter().all(|x| *x == c || case.upper.contains(x)), "the cycle is not made of U* and {c}");
    assert!(!playable(t, &NEAR_MISS_CYCLE), "the cycle {NEAR_MISS_CYCLE:?} is playable");
}

/// A number `c` up to n/2 with the factor `f` of a back claim `(s, f)` among its maximal factors that is a factor of a
/// frame of the root, with what the model and the oracle say of it.
#[derive(Clone, Copy, Debug)]
struct Sighting {
    n: u32,
    s: u32,
    f: u32,
    c: u32,
    candidate: bool,
    /// Whether the model would accept `c` alone: `{c}` can be promoted from the root.
    model_accepts: bool,
    /// Whether U\* ∪ {c} is playable.
    playable: bool,
}

/// The sightings on the board `1..=n` at width `W`: for every back claim `(s, f)` of the search's frames and every
/// number `c` up to n/2 that has `f` among its maximal factors and is a factor of a frame of the root.
fn sightings<const W: usize>(n: u32) -> Vec<Sighting> {
    let case = Case::<W>::new(n);
    let (p, t) = (&case.prepared, &case.board.tables);
    let mut scratch = Scratch::new();
    let mut found = Vec::new();
    for claims in &case.board.frames.claims {
        for &(s, f, _) in claims.iter().filter(|claim| claim.2 == ClaimKind::Back) {
            for &c in t.max_composites[f as usize].iter().filter(|&&c| 2 * c <= n) {
                if p.model.home(c) >= 1 && p.model.contains_factor(&p.root, c) {
                    let model_accepts = case.promotable(&mut scratch, &Bits::from_slice(&[c]));
                    let candidate = p.candidates.contains(c);
                    found.push(Sighting { n, s, f, c, candidate, model_accepts, playable: case.playable_with(&[c]) });
                }
            }
        }
    }
    found
}

/// Every board from 2 to 2000, for the back claims that a number up to n/2 could hide a use of the factor behind.
/// No such number is a candidate (check D of `examples/structure.rs`), and none makes U\* ∪ {c} playable (Theorem M);
/// where the model alone would accept it, only the candidate filter keeps it out.  Prints what it finds; the first of
/// them is [`NEAR_MISS`].
#[test]
#[ignore = "takes ten seconds; it found the board of the near-miss test"]
fn the_scan_for_near_misses() {
    let start = Instant::now();
    let mut seen = Vec::new();
    for n in 2..=2000 {
        seen.extend(with_width!(n, W => sightings::<W>(n)));
    }
    let boards = |list: &[Sighting]| list.iter().map(|s| s.n).collect::<BTreeSet<u32>>();
    let (candidates, playable): (Vec<Sighting>, Vec<Sighting>) =
        (seen.iter().filter(|s| s.candidate).copied().collect(), seen.iter().filter(|s| s.playable).copied().collect());
    let near_misses: Vec<Sighting> =
        seen.iter().filter(|s| s.model_accepts && !s.candidate && !s.playable).copied().collect();
    let rejected: Vec<Sighting> =
        seen.iter().filter(|s| !s.model_accepts && !s.candidate && !s.playable).copied().collect();
    println!("{} sightings on the boards 2..=2000 in {:.1?}", seen.len(), start.elapsed());
    println!("  candidates: {}; playable with U*: {}", candidates.len(), playable.len());
    println!("  rejected by the model as well: {} on {} boards", rejected.len(), boards(&rejected).len());
    println!(
        "  near misses: {} on {} boards: {:?}",
        near_misses.len(),
        boards(&near_misses).len(),
        boards(&near_misses)
    );
    let distinct: BTreeSet<(u32, u32, u32)> = near_misses.iter().map(|s| (s.s, s.f, s.c)).collect();
    println!("  near misses by (s, f, c): {distinct:?}");

    assert!(candidates.is_empty(), "candidates that a back claim hides: {candidates:?}");
    assert!(playable.is_empty(), "numbers that a back claim hides and U* plays: {playable:?}");
    let first = near_misses.first().expect("a near miss");
    assert_eq!((first.n, first.s, first.f, first.c), NEAR_MISS, "the smallest near miss");
}
