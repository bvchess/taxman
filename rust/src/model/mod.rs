//! The frame model's search state: frames, frame sets, promotions, and which sets can be promoted together.
//!
//! The frame builder ([`crate::frames`]) turns the board into *frames* numbered from the front (frame 0, played
//! first) to the back.  Each frame is a game of *Taxman Mini*: a set of *moves* and a set of *factors*.  A move
//! must still have one of its maximal factors among the frame's factors when it is taken, and taking it removes
//! all of its factors still there (its tax).  A frame is *winnable* when some order takes every move.
//! (`rust/README.md` introduces the model with an example, and its glossary defines the words used here.)
//!
//! A game chooses which candidates to *promote*: promoting `n` takes it out of the factors of its *home frame*
//! (the frame it is promoted from) `h ≥ 1` and makes it a move of frame `h − 1`, one frame forward, where it pays
//! its tax.  That is legal when both frames stay winnable.  After a promotion both frames are *reduced*: a move
//! left with a single live factor is paid by it, and the pair is dropped, repeatedly.  Every frame's *freedoms*
//! (initially factors minus moves) drop by one on each promotion into or out of it.  The search looks for the
//! promotion set with the largest sum; the score of the game is what the game with no promotions scores plus
//! that sum.
//!
//! What is where:
//!
//! * [`FactorProvider`] (`provider`): who can pay for whom, restricted to the frames each number belongs to.
//! * `peel`: winnability of one frame (the *peel*), the *residual* of an unwinnable frame, and reduction, on a
//!   reusable [`Scratch`].
//! * [`Frame`], [`FrameSet`] (`frame`): one frame, and a game's frames (`Copy`, no heap); narrowing the candidates
//!   and counting the promotions left from the freedoms.
//! * [`FrameGraph`] (here): what every state of one game's search shares, the factor graph and each number's home
//!   frame.  Its methods promote (`promote`: why a set of promotions can be checked frame by frame) and find
//!   conflicts and nogoods (`conflict`).
//! * [`Nogoods`] (`nogoods`): the table of conflicts of the root that a searcher learns.
//!
//! This is the Java solver's `Frame`, `FrameSet`, `FrameMapper` (home frames only) and `FactorProvider`
//! (package `org.taxman.h6.search`), reimplemented for speed rather than translated: frames are inline bitsets,
//! a [`FrameSet`] is `Copy` and never allocates, and promotions can be done in place.  The results are the
//! Java's (the golden tests check them); how they are computed differs: the peel's rule, the reduction started
//! only where a promotion can have left a move with a single live factor (frames in a [`FrameSet`] are always
//! reduced), and the factor lists restricted to the frames each number can take part in.
//!
//! Numbers are `u32` game numbers throughout; the lists store them as `u16`, which bounds boards at 65535.

mod conflict;
mod frame;
mod nogoods;
mod peel;
mod promote;
mod provider;

pub use conflict::ConflictCache;
pub use frame::{Frame, FrameSet};
pub use nogoods::{Lookup, Nogoods};
pub use peel::Scratch;
pub use promote::Edits;
pub use provider::FactorProvider;

use crate::bits::Bits;
use crate::numtheory::Tables;

/// Most frames a [`FrameSet`] can hold.  The frame builder gives every board up to 8191, the largest supported, at
/// most 9 frames (all of them were checked in 2026-10: 6 frames from n = 512, 7 from 1024, 8 from 2048, 9 from 4096),
/// and frame sets and per-frame arrays are copied whole, so they have no room to spare.  [`FrameGraph::build`]
/// refuses more (`tests/wide.rs` builds the boards where the count steps up, and the largest).
pub const MAX_FRAMES: usize = 9;

/// What every state of one game's search shares: the factor graph and each number's home frame.  Immutable
/// and `Sync`, so search threads can share one.  The states are [`FrameSet`]s, and this is what interprets them.
#[derive(Clone, Debug)]
pub struct FrameGraph {
    graph: FactorProvider,
    /// Home frame of each number: the frame whose factors held it when the frame set was built; 0 for numbers
    /// that were not a factor anywhere (as the Java's `FrameMapper`).
    home: Vec<u8>,
}

impl FrameGraph {
    /// The Java's `FrameSet.create`: `columns` alternate factors and moves per frame, front first; each frame
    /// is reduced.  Returns the model and the starting frame set.
    pub fn build<const W: usize>(tables: &Tables, columns: &[Vec<u32>]) -> Result<(FrameGraph, FrameSet<W>), String> {
        if !columns.len().is_multiple_of(2) {
            return Err(format!("{} columns: expected factors and moves for each frame", columns.len()));
        }
        let len = columns.len() / 2;
        if len > MAX_FRAMES {
            return Err(format!("{len} frames: at most {MAX_FRAMES} are supported"));
        }
        let max = columns.iter().flatten().copied().max().unwrap_or(0);
        if max > Bits::<W>::MAX_VALUE {
            return Err(format!("number {max} does not fit in Bits<{W}>"));
        }
        if max > tables.n {
            return Err(format!("number {max} is beyond the tables for n={}", tables.n));
        }

        // Frame i pays with its factors for its moves and for the factors of frame i + 1 promoted into it.
        let mut as_move = vec![0u16; max as usize + 1];
        let mut as_factor = vec![0u16; max as usize + 1];
        for i in 0..len {
            for &y in &columns[2 * i] {
                as_factor[y as usize] |= 1 << i;
                if i > 0 {
                    as_move[y as usize] |= 1 << (i - 1);
                }
            }
            for &x in &columns[2 * i + 1] {
                as_move[x as usize] |= 1 << i;
            }
        }
        let graph = FactorProvider::with_roles(tables, &as_move, &as_factor);

        let mut root = FrameSet { len, frames: [Frame::default(); MAX_FRAMES] };
        for (i, frame) in root.frames[..len].iter_mut().enumerate() {
            let raw = Frame::new(Bits::from_slice(&columns[2 * i + 1]), Bits::from_slice(&columns[2 * i]));
            *frame = raw.reduce(&graph);
        }
        let mut home = vec![0u8; max as usize + 1];
        for (i, frame) in root.frames().iter().enumerate() {
            for x in frame.factors.iter() {
                home[x as usize] = i as u8;
            }
        }
        Ok((FrameGraph { graph, home }, root))
    }

    /// The factor graph, restricted to the frames each number belongs to; valid for every frame of this model.
    #[inline]
    pub fn graph(&self) -> &FactorProvider {
        &self.graph
    }

    /// The home frame of `n` (0 if `n` was not a factor of any frame).
    #[inline]
    pub fn home(&self, n: u32) -> usize {
        self.home.get(n as usize).map_or(0, |&h| usize::from(h))
    }

    /// The Java's `containsFactor`: whether `n` is still a factor of its home frame.
    #[inline]
    pub fn contains_factor<const W: usize>(&self, state: &FrameSet<W>, n: u32) -> bool {
        let h = self.home(n);
        h < state.len && state.frames[h].factors.contains(n)
    }
}

/// Helpers shared by the tests of this module, `bounds`, `matchbound` and `search`.
#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod tests;
