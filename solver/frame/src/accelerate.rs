//! The prime shortcut, and the last step that turns a promotion set into moves.
//!
//! Where this fits ([`crate::solver`] has the whole pipeline): before it searches for the moves for `n`, the solver
//! tries a shortcut for a prime `n` that needs the (optimal) moves for `n − 1`; `--force-search` turns it off.
//!
//! * [`prime_shortcut`], for a prime `n`.  **Proven**: given optimal moves for `n − 1`, the moves it returns are
//!   optimal, with no assumption from the frame model.  The proof is the prime formula (N5,
//!   docs/theory/neighbors.md), cited in the documentation of the function.
//!
//! [`solution_for_promotions`] is not a shortcut: it is the last step of every path, the search's included, and
//! turns the promotion set that the frame model works with into the moves that play it.

use crate::bits::Bits;
use crate::frames::BoardFrames;
use crate::numtheory::Tables;

/// The moves for a prime `n`: the moves for `n − 1` with the first replaced by `n`.  There are no moves for 1,
/// so for `n = 2` the result is the single move `[2]`.
///
/// The caller must know that `n` is prime and that `prev_moves` are *optimal* moves for `n − 1`; then the result
/// is optimal for `n`.  The function returns only the moves: the caller works out the promotions (the
/// solver takes the candidates that appear among the moves).
///
/// # Why this is optimal
///
/// Let `P` be the largest prime below `n`.  Optimal moves for `n − 1` open with `P` (the prime lemma (N1,
/// docs/theory/neighbors.md); with the finished-game lemma (N2, same file) it is also over, which
/// [`crate::game::replay_finished`] checks of all the moves it referees).  Exchanging that move for `n` gives moves
/// for `n` that score `opt(n − 1) − P + n`, and `opt(n)` is exactly that (the prime formula (N5, same file)).  The
/// exchange changes nothing else of the moves, since neither prime divides another number of either board.  For
/// `n = 2` there are no moves for 1 and the result is the single move `[2]`.
pub fn prime_shortcut(n: u32, prev_moves: &[u32]) -> Vec<u32> {
    let mut moves = prev_moves.to_vec();
    if moves.is_empty() {
        moves.push(n);
    } else {
        moves[0] = n;
    }
    moves
}

/// The moves that play the given promotions: the numbers in `promotions` become moves (they are treated as
/// sources, see [`crate::frames`]), the frames are formed again around them, and the frames' moves are concatenated,
/// the frames in front first.
///
/// This is the last step of every path: the frame model, and so the search, works with promotion sets, and this
/// turns the winner into moves.  For a set the search accepted (every frame stays winnable) no source is lost, so
/// the moves score `in_the_bag + Σ promotions`, where `in_the_bag` is what [`BoardFrames::solution`] scores with no
/// promotions; the solver checks that identity of all moves as a cross-check between the frames as setup forms
/// them ([`crate::frames`]) and the frames as the search takes them.  For a set that is not winnable the frames
/// simply drop the moves they cannot fit, and the moves score less.
pub fn solution_for_promotions<const W: usize>(t: &Tables, promotions: Bits<W>) -> Vec<u32> {
    BoardFrames::<W>::new(t, promotions).solution()
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
