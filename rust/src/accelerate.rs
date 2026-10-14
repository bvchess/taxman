//! The prime shortcut, and the last step that turns a promotion set into a game.
//!
//! Where this fits ([`crate::solver`] has the whole pipeline): before it searches for the game of `n`, the solver
//! tries a shortcut that needs the (optimal) game of `n − 1`, the Java's `FrameSolver.solveBasedOnPrev` for a prime
//! `n`, which `--force-search` turns off.
//!
//! * [`prime_shortcut`], for a prime `n`.  **Proven**: given an optimal game of `n − 1`, the game it returns is
//!   optimal, with no assumption from the frame model.  The proof is the documentation of the function.
//!
//! [`solution_for_promotions`] is not a shortcut: it is the last step of every path, the search's included, and
//! turns the promotion set that the frame model works with into the moves of an actual game.

use crate::bits::Bits;
use crate::hives::Apiary;
use crate::numtheory::Tables;

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

/// The game that plays the given promotions: the numbers in `promotions` become moves (they are treated as
/// sources, see [`crate::hives`]), the frames are formed again around them, and the frames' games are concatenated,
/// the frames in front first.
///
/// This is the last step of every path: the frame model, and so the search, works with promotion sets, and this
/// turns the winner into moves.  For a set the search accepted (every frame stays winnable) no source is lost, so
/// the game scores `in_the_bag + Σ promotions`, where `in_the_bag` is what [`Apiary::solution`] scores with no
/// promotions; the solver checks that identity of every game as a cross-check between the frames as setup forms
/// them ([`crate::hives`]) and the frames as the search takes them.  For a set that is not winnable the frames
/// simply drop the moves they cannot fit, and the game scores less.
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
