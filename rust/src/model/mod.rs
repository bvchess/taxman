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
//! # One-sided candidates
//!
//! The extended problem Π⁺(O) ([`crate::frames::build_extended`], heuristics/THEORY.md, "The extended problem")
//! has, besides the candidates above (*two-sided*: out of one frame, into the one in front), *one-sided* candidates:
//! sources that a game may play or leave out.  A one-sided candidate `s` has an *own* frame `j`
//! ([`FrameGraph::own`]); promoting it adds `s` to the moves of frame `j` if that frame stays winnable, removes no
//! factor and changes no other frame, and spends one freedom of frame `j`.  [`FrameGraph::build_extended`] builds a
//! model with them.  Everything that reasons about sets of promotions (frame-by-frame checks, conflicts, nogoods)
//! holds for them unchanged: a one-sided member is one more move of its frame, like a number promoted into it.  The
//! state does not record which one-sided candidates were promoted (a two-sided one is no longer a factor of its home
//! once promoted), so each is promoted at most once along a path, as the search's candidates ensure.  A model with
//! none ([`FrameGraph::build`]) is the frame model as it was, and every method below reduces to what it did.
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

/// What every state of one game's search shares: the factor graph, each number's home frame and the one-sided
/// candidates' own frames.  Immutable and `Sync`, so search threads can share one.  The states are [`FrameSet`]s,
/// and this is what interprets them.
#[derive(Clone, Debug)]
pub struct FrameGraph {
    graph: FactorProvider,
    /// Home frame of each number: the frame whose factors held it when the frame set was built; 0 for numbers
    /// that were not a factor anywhere (as the Java's `FrameMapper`).
    home: Vec<u8>,
    /// The own frame plus one of each one-sided candidate, 0 for every other number; empty when there are none.
    own: Vec<u8>,
    /// The one-sided candidates, ascending, each with its own frame.
    one_sided: Vec<(u32, usize)>,
    /// Per frame, the one-sided candidates whose own frame it is, and all of them: the words of a set of numbers
    /// (empty when there are none).
    one_sided_by_frame: Vec<Vec<u64>>,
    one_sided_all: Vec<u64>,
}

impl FrameGraph {
    /// The Java's `FrameSet.create`: `columns` alternate factors and moves per frame, front first; each frame
    /// is reduced.  Returns the model and the starting frame set.
    pub fn build<const W: usize>(tables: &Tables, columns: &[Vec<u32>]) -> Result<(FrameGraph, FrameSet<W>), String> {
        Self::build_extended(tables, columns, &[])
    }

    /// [`build`](Self::build) with the one-sided candidates `one_sided` (see the module docs), each a number with its
    /// own frame: a frame of `columns`, of which the number is neither a move nor a factor.  The factor graph lists a
    /// one-sided candidate's maximal factors among its own frame's factors, as it does a move's.  With none, it is
    /// [`build`](Self::build).
    pub fn build_extended<const W: usize>(
        tables: &Tables,
        columns: &[Vec<u32>],
        one_sided: &[(u32, usize)],
    ) -> Result<(FrameGraph, FrameSet<W>), String> {
        if !columns.len().is_multiple_of(2) {
            return Err(format!("{} columns: expected factors and moves for each frame", columns.len()));
        }
        let len = columns.len() / 2;
        if len > MAX_FRAMES {
            return Err(format!("{len} frames: at most {MAX_FRAMES} are supported"));
        }
        let in_frames = columns.iter().flatten().copied().max().unwrap_or(0);
        let max = one_sided.iter().map(|&(s, _)| s).fold(in_frames, u32::max);
        if max > Bits::<W>::MAX_VALUE {
            return Err(format!("number {max} does not fit in Bits<{W}>"));
        }
        if max > tables.n {
            return Err(format!("number {max} is beyond the tables for n={}", tables.n));
        }

        // Frame i pays with its factors for its moves, for the factors of frame i + 1 promoted into it, and for the
        // one-sided candidates of frame i.
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
        let mut sorted = one_sided.to_vec();
        sorted.sort_unstable();
        let mut own = if sorted.is_empty() { Vec::new() } else { vec![0u8; max as usize + 1] };
        let words = max as usize / 64 + 1;
        let (mut by_frame, mut all) = (vec![vec![0u64; words]; len], vec![0u64; words]);
        for &(s, j) in &sorted {
            if j >= len {
                return Err(format!("the one-sided candidate {s} has the frame {j} of {len}"));
            }
            if as_move[s as usize] != 0 || as_factor[s as usize] != 0 || own[s as usize] != 0 {
                return Err(format!("the one-sided candidate {s} is already in a frame, or listed twice"));
            }
            as_move[s as usize] = 1 << j;
            own[s as usize] = j as u8 + 1;
            by_frame[j][s as usize / 64] |= 1 << (s % 64);
            all[s as usize / 64] |= 1 << (s % 64);
        }
        let graph = FactorProvider::with_roles(tables, &as_move, &as_factor);

        let mut root = FrameSet { len, frames: [Frame::default(); MAX_FRAMES] };
        for (i, frame) in root.frames[..len].iter_mut().enumerate() {
            let raw = Frame::new(Bits::from_slice(&columns[2 * i + 1]), Bits::from_slice(&columns[2 * i]));
            *frame = raw.reduce(&graph);
        }
        let mut home = vec![0u8; in_frames as usize + 1];
        for (i, frame) in root.frames().iter().enumerate() {
            for x in frame.factors.iter() {
                home[x as usize] = i as u8;
            }
        }
        if sorted.is_empty() {
            (by_frame, all) = (Vec::new(), Vec::new());
        }
        let model =
            FrameGraph { graph, home, own, one_sided: sorted, one_sided_by_frame: by_frame, one_sided_all: all };
        Ok((model, root))
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

    /// The own frame of `n` if it is a one-sided candidate (see the module docs).
    #[inline]
    pub fn own(&self, n: u32) -> Option<usize> {
        self.own.get(n as usize).filter(|&&j| j != 0).map(|&j| usize::from(j) - 1)
    }

    /// Whether the model has one-sided candidates.
    #[inline]
    pub fn has_one_sided(&self) -> bool {
        !self.one_sided.is_empty()
    }

    /// The one-sided candidates, ascending, each with its own frame.
    pub fn one_sided(&self) -> &[(u32, usize)] {
        &self.one_sided
    }

    /// The frames that promoting the candidate `x` touches: the one it becomes a move of, and the one whose factor it
    /// stops being (none for a one-sided candidate).  `x` must be a candidate: one-sided, or of home `h ≥ 1`.
    #[inline]
    pub fn frames_touched(&self, x: u32) -> (usize, Option<usize>) {
        match self.own(x) {
            Some(j) => (j, None),
            None => {
                let h = self.home(x);
                (h - 1, Some(h))
            }
        }
    }

    /// The members of `cands` that `state` can still promote by the count of freedoms alone: the two-sided ones of
    /// [`FrameSet::narrow_candidates`], and the one-sided ones whose own frame has a freedom left (promoting one
    /// spends a freedom of that frame and of no other, and a winnable frame has no negative freedoms).
    #[inline]
    pub fn narrow<const W: usize>(&self, state: &FrameSet<W>, cands: &Bits<W>) -> Bits<W> {
        let two_sided = state.narrow_candidates(cands);
        if !self.has_one_sided() {
            return two_sided;
        }
        let mut words = *two_sided.as_words();
        for (frame, own) in state.frames().iter().zip(&self.one_sided_by_frame) {
            if frame.freedoms > 0 {
                for ((word, own), cand) in words.iter_mut().zip(own).zip(cands.as_words()) {
                    *word |= own & cand;
                }
            }
        }
        Bits::from_words(words)
    }

    /// An upper bound on the number of members of `cands` that a completion of `state` promotes: the count of
    /// [`FrameSet::max_promotions_among`] for the two-sided ones (one-sided ones are no frame's factors, so it does
    /// not see them), plus the number of one-sided ones.  Sound: a one-sided promotion only spends a freedom, which
    /// lowers what the two-sided ones can have.
    #[inline]
    pub fn max_promotions_among<const W: usize>(&self, state: &FrameSet<W>, cands: &Bits<W>) -> i32 {
        let two_sided = state.max_promotions_among(cands);
        if !self.has_one_sided() {
            return two_sided;
        }
        let one_sided = self.one_sided_all.iter().zip(cands.as_words()).map(|(own, cand)| (own & cand).count_ones());
        two_sided + one_sided.sum::<u32>() as i32
    }
}

/// Helpers shared by the tests of this module, `bounds`, `matchbound` and `search`.
#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod tests;
