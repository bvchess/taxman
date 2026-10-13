//! Games found without a search: the shortcuts from the game of `n − 1`, and the last step that turns a promotion
//! set into a game.
//!
//! Where this fits ([`crate::solver`] has the whole pipeline): before it searches for the game of `n`, the solver
//! tries two shortcuts that need the (optimal) game of `n − 1`.  Both are the Java's
//! `FrameSolver.solveBasedOnPrev`, and `--force-search` turns both off.
//!
//! * [`prime_shortcut`], for a prime `n`.  **Proven**: given an optimal game of `n − 1`, the game it returns is
//!   optimal, with no assumption from the frame model.  The proof is the documentation of the function.
//! * [`reuse_prev`], for other `n`.  **Unproven** (the tactic `reuse_prev`): it builds a game from the previous
//!   game's moves and accepts it if it scores what the previous game *suggests* the best game scores.  Its
//!   answers have always agreed with the Java's table, but nothing proves them, so a result found this way
//!   carries the taint `reuse_prev` ([`crate::results`]).
//!
//! [`solution_for_promotions`] is not a shortcut: it is the last step of every path, the search's included, and
//! turns the promotion set that the frame model works with into the moves of an actual game.

use crate::bits::Bits;
use crate::frames::FrameColumns;
use crate::hives::Apiary;
use crate::numtheory::Tables;

/// A game found without searching, and the promotions it was built from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Accelerated {
    /// The game, in playing order.
    pub moves: Vec<u32>,
    /// The promoted numbers the game was built from, ascending.
    pub promotions: Vec<u32>,
}

/// The game for a prime `n`: the game for `n − 1` with its first move replaced by `n`.  The game for 1 has no
/// moves, so for `n = 2` the result is the single move `[2]`.
///
/// The caller must know that `n` is prime and that `prev_moves` is an *optimal* game of `n − 1`; then the result
/// is an optimal game of `n` (proof below).  The function returns only the moves: the caller works out the
/// promotions (the solver takes the candidates that appear among the moves).
///
/// # Why this is optimal
///
/// Write `opt(m)` for the best score on the board `1..=m`, and let `P` be the largest prime `≤ n − 1` (for
/// `n = 2` the single move `[2]` is the only game with a move).  Call a number *inert* when it divides nothing
/// else on the board, so that it can never be anyone's tax.  Two facts are used:
///
/// * A prime has no proper divisor but 1, so it can be played only while 1 is in the pot.  Every move removes 1,
///   so a prime can only be the *first* move, and playing it removes nothing but 1 and itself.
/// * If `P` is the largest prime `≤ m`, then `2P > m` (Bertrand's postulate puts a prime between `P` and `2P`),
///   so `P` is inert on the board `1..=m`.  For a prime `n` it is inert on `1..=n` too (`2P ≠ n`), and `n` is
///   inert on `1..=n` because it is the largest number there.
///
/// *Lemma A: every optimal game of `m ≥ 2` opens with the largest prime `P ≤ m`.*  A game `S` that opens with
/// something else can be improved.  If it opens with a smaller prime `q`, open with `P` instead: it removes the
/// same (only 1), and `P` is inert, so every later move keeps its payer; the game scores `P − q` more.  If it
/// opens with a composite `c`, open with `P` and play `c` second: `c` has a proper divisor other than 1, which
/// `P` did not remove, so `c` is still legal and removes what it removed before; after it the pot differs from
/// `S`'s only by the inert `P`, so the rest of `S` is legal too, and the game scores `P` more.  (The empty game
/// is not optimal either: `P` can be played.)  [`crate::game::replay_finished`] checks this of every game it
/// referees.
///
/// *Lemma B: for a prime `n`, `opt(n) ≤ n + opt(n − 1) − P`.*  Let `S` be an optimal game of `n`.  If `n` is not
/// in `S`, `S` is a game of `n − 1` (`n` is inert), and its score is at most `opt(n − 1)`, which is less than the
/// bound because `n > P`.  Otherwise `n` is played, as the first move.  Replace it by `P`, which takes just 1 as
/// well.  A later move `m` of `S` is below `n` and is not `P` (which cannot be played once 1 is gone), and `S`
/// played it with some proper divisor `d` of `m` still in the pot; `d` is neither `n` nor `P` (both are inert), so
/// `d` is still in the new pot too.  The result is a game of `n − 1` that scores `P + score(S) − n ≤ opt(n − 1)`.
///
/// *The shortcut.*  An optimal game of `n − 1` opens with `P` (Lemma A).  Replace that move by `n`, which takes
/// just 1 as well.  Each later move was legal with a payer that is neither `P` nor `n` (inert), so it is still
/// legal, and the game is legal on `1..=n`.  It scores `opt(n − 1) − P + n`, which is the bound of Lemma B, so it
/// is optimal.  ∎
///
/// (The sacrifice bound of [`crate::sacrifice`] contains these facts as its case `f = 1`.)
pub fn prime_shortcut(n: u32, prev_moves: &[u32]) -> Vec<u32> {
    let mut moves = prev_moves.to_vec();
    if moves.is_empty() {
        moves.push(n);
    } else {
        moves[0] = n;
    }
    moves
}

/// Tries to build the game for `t.n` from a game of `t.n − 1` (`prev_moves`; empty for `t.n = 1`) without a
/// search.  `None` means the shortcut does not apply and the game needs a search.  **This is the unproven tactic
/// `reuse_prev`.**
///
/// 1. If the board has no promotion candidates, the game with no promotions is the only one the frame model has,
///    and needs no previous game.  (The solver finds this case itself first, from the frames' candidates, and
///    never gets here with it; the two candidate sets, those of the unmerged hives here and those the frame
///    builder collects level by level, are equal for every board `2 ≤ n ≤ 2000`.  The branch is the Java's.)
/// 2. Otherwise promote every number that is both a candidate now and a move of the previous game, keeping at
///    most as many (the largest ones) as the frames can absorb at all (`frames.estimate_max_promotions`).
///    These are the previous game's promotions that are still candidates, and, on an even `n`, the former source
///    `n/2`, which has just become a factor.
/// 3. The *target* is the previous game's moves, plus the new number `n`, minus the previous moves that are
///    impossible on the new board.  The game built from those promotions is accepted only if it scores exactly
///    that.
///
/// # Why it is plausible, and why it is unproven
///
/// Any game of `n − 1` is also a game of `n`, and removing `n` from a game of `n` leaves a game of `n − 1`, so
/// `opt(n) ≤ n + opt(n − 1)`: the ceiling that the search stops at.  The target is that ceiling lowered by the
/// moves of the previous game that the new board has made impossible (the new source `n` competes for the factors
/// of its hive, and the greedy test of [`crate::hives`] may now find a smaller source of that hive unmatchable).
/// The tactic bets that the best game of `n` is the previous game with `n` added and those moves dropped, and so
/// accepts a game that reaches the target.
///
/// (DESIGN.md compares this rule with the "upper-delta" certificate of the Python heuristics, a similar
/// prediction from the previous game; that the two always coincide has not been shown.)
///
/// When none of the previous moves became impossible the target *is* the ceiling and an accepted game is
/// optimal, given that the previous one is.  Otherwise the target is a conjecture: nothing prevents a game of
/// `n` that is unlike the previous one from scoring more.  It has held wherever it has been checked against a
/// run that does not rely on it: a strict solve of 1 to 1000 reproduced every score of `optimal.json`, and the
/// committed default and strict chains for 1001 to 1231 have equal scores (`results/README.md`).  (`taxman
/// regress` compares with `optimal.json`, which the Java made with the same rule, so that alone is no
/// independent witness.)  A run with the tactic off (`--tactic reuse_prev=off`, the *strict* runs) never relies
/// on it.
pub fn reuse_prev<const W: usize>(t: &Tables, frames: &FrameColumns<W>, prev_moves: &[u32]) -> Option<Accelerated> {
    let plain = Apiary::<W>::new(t, Bits::EMPTY);
    let candidates = plain.promotion_candidates();
    if candidates.is_empty() {
        return Some(Accelerated { moves: plain.solution(), promotions: Vec::new() });
    }

    let prev: Bits<W> = prev_moves.iter().copied().collect();
    let promotions = (candidates & prev).largest(frames.estimate_max_promotions);
    let target = (prev.with(t.n) - (plain.impossible() & prev)).sum();

    let moves = Apiary::<W>::new(t, promotions).solution();
    let score: u64 = moves.iter().map(|&m| u64::from(m)).sum();
    (score == target).then(|| Accelerated { moves, promotions: promotions.to_vec() })
}

/// The game that plays the given promotions: the numbers in `promotions` become moves (they are treated as
/// sources, see [`crate::hives`]), the hives are formed again around them, and the hives' games are concatenated,
/// the hives downstream first.
///
/// This is the last step of every path: the frame model, and so the search, works with promotion sets, and this
/// turns the winner into moves.  For a set the search accepted (every frame stays winnable) no source is lost, so
/// the game scores `in_the_bag + Σ promotions`, where `in_the_bag` is what [`Apiary::solution`] scores with no
/// promotions; the solver checks that identity of every game as a cross-check between the hive code and the frame
/// model.  For a set that is not winnable the hives simply drop the moves they cannot fit, and the game scores
/// less (which is how [`reuse_prev`] rejects its guesses).
pub fn solution_for_promotions<const W: usize>(t: &Tables, promotions: Bits<W>) -> Vec<u32> {
    Apiary::<W>::new(t, promotions).solution()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prime_shortcut_replaces_the_first_move() {
        assert_eq!(prime_shortcut(2, &[]), vec![2]);
        assert_eq!(prime_shortcut(3, &[2]), vec![3]);
        assert_eq!(prime_shortcut(7, &[5, 4, 6]), vec![7, 4, 6]);
        assert_eq!(prime_shortcut(11, &[7, 9, 6, 10, 8]), vec![11, 9, 6, 10, 8]);
    }
}
