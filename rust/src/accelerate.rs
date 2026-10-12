//! Acceleration: deriving the solution of game `n` from the solution of game `n − 1`, with no search.
//!
//! The solver tries two shortcuts before it searches (`FrameSolver.solveBasedOnPrev` in the Java code).
//! For a prime `n` the solution of `n − 1` is reused almost verbatim ([`prime_shortcut`]).  For other `n`,
//! [`reuse_prev`] keeps the promotions of the previous game that are still possible, builds the game they
//! imply, and accepts it if it scores exactly as much as the previous game can be expected to.

use crate::bits::Bits;
use crate::frames::FrameColumns;
use crate::hives::Apiary;
use crate::numtheory::Tables;

/// A game found without searching, and the promotions it was built from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Accelerated {
    pub moves: Vec<u32>,
    /// Ascending.
    pub promotions: Vec<u32>,
}

/// The game for a prime `n`: the game for `n − 1` with its first move replaced by `n`.  (The first move of an
/// optimal game is a large prime, and `n` can take its place.)  The game for 1 has no moves; for `n = 2` it
/// is padded to give the single move `[2]`.  The caller must know that `n` is prime and that `prev_moves`
/// is a solution of `n − 1`; the promotions are those of the previous game.
pub fn prime_shortcut(n: u32, prev_moves: &[u32]) -> Vec<u32> {
    let mut moves = prev_moves.to_vec();
    if moves.is_empty() {
        moves.push(n);
    } else {
        moves[0] = n;
    }
    moves
}

/// Tries to build the game for `t.n` from a solution of `t.n − 1` (`prev_moves`; empty for `t.n = 1`).
///
/// 1. If the board has no promotion candidates, the game with no promotions is the only one there is, and
///    needs no previous game.
/// 2. Otherwise promote every number that is both a candidate and a move of the previous game, keeping at
///    most as many (the largest ones) as `frames` says can be promoted.
/// 3. The previous game, plus the new number `n`, minus the moves that have become impossible, is the score
///    to beat.  The game built from those promotions is accepted only if it reaches exactly that.
///
/// `None` means the shortcut does not apply and the game needs a search.
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

/// The game that plays the given promotions: the numbers in `promotions` become moves of the hives below
/// them, and the hives' games are concatenated.
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
