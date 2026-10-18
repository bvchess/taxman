//! The known game of a board as a search incumbent, for the examples that start a search from the optimum
//! (`#[path = "common/known.rs"] mod known;`; `tune` proves subtrees from it and `wall` measures what it saves), and
//! what survives of a seed in a board, from which the ledger starts its finder.

use taxman::bits::Bits;
use taxman::board::Board;
use taxman::engine::Control;
use taxman::game::Optimal;
use taxman::search::{self, Prepared};

/// The incumbent promotion set of the game's known optimum, and the optimum's promotion sum: the known game's
/// promotions (those that are candidates here) that can all be promoted, as a search seeded with them starts.
pub fn known_incumbent<const W: usize>(
    board: &Board<W>,
    prepared: &Prepared<W>,
    optimal: &Optimal,
) -> Result<Option<(Bits<W>, u64)>, String> {
    let (Some(score), Some(moves)) = (optimal.score(board.n()), optimal.moves(board.n())) else { return Ok(None) };
    let opt = score
        .checked_sub(board.in_the_bag)
        .ok_or_else(|| format!("the known game of {} scores less than the game with no promotions", board.n()))?;
    let set = survivors(prepared, &board.seed(moves))?;
    if set.sum() != opt {
        return Err(format!(
            "the known game of {} has promotions worth {} that can be promoted here, not the {opt} it scores over \
             the bag",
            board.n(),
            set.sum()
        ));
    }
    Ok(Some((set, opt)))
}

/// What survives of `seed` in `prepared`, the incumbent a search seeded with it starts from.
pub fn survivors<const W: usize>(prepared: &Prepared<W>, seed: &Bits<W>) -> Result<Bits<W>, String> {
    let control = Control::new();
    search::offer_seed(prepared, &control, seed);
    search::best_so_far(&control).ok_or_else(|| "no incumbent survives".to_owned())
}
