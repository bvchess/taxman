//! What survives of a seed in a board, from which the ledger starts its finder
//! (`#[path = "../common/known.rs"] mod known;`).

use taxman::bits::Bits;
use taxman::engine::Control;
use taxman::search::{self, Prepared};

/// What survives of `seed` in `prepared`, the incumbent a search seeded with it starts from.
pub fn survivors<const W: usize>(prepared: &Prepared<W>, seed: &Bits<W>) -> Result<Bits<W>, String> {
    let control = Control::new();
    search::offer_seed(prepared, &control, seed);
    search::best_so_far(&control).ok_or_else(|| "no incumbent survives".to_owned())
}
