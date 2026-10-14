//! One frame ([`Frame`]) and the frames of a game ([`FrameSet`]), with what their freedoms say.
//!
//! A frame's *freedoms* start as its factors minus its moves and drop by one on every promotion into it (one move
//! more) or out of it (one factor less); a reduction drops a move and a factor together, so the freedoms stay
//! the factors minus the moves.  A winnable frame pays every move with a factor of its own, so its freedoms are
//! never negative.  Two counts follow from the freedoms alone, and hold in every state the search reaches:
//!
//! * [`FrameSet::narrow_candidates`]: promoting a factor of frame `i` spends a freedom of frame `i` and one of
//!   frame `i − 1`, so only the factors of frames that have freedoms left, above a frame that has too, can still
//!   be promoted.
//! * [`FrameSet::max_promotions`]: with `c_i` further promotions out of frame `i` (into frame `i − 1`), frame `i`
//!   spends `c_i + c_{i+1}` of its freedoms `F_i`.  Taking as many as fit from the bottom up maximizes `Σ c_i`
//!   under those limits (raising `c_i` at the cost of `c_{i+1}` never lowers the sum), so the count it gives
//!   bounds the promotions of any completion.  The search's `max_total` and `max_add` come from it.

use super::peel::{reduce_from, winnable, Worklist};
use super::{FactorProvider, MAX_FRAMES};
use crate::bits::Bits;
use std::fmt;
use std::hash::{Hash, Hasher};

/// One frame: its moves, its factors, and its freedoms.  Freedoms start at `|factors| − |moves|` and drop by one
/// on every promotion into or out of the frame.  Equality includes the freedoms (the Java's `Frame.equals` ignores
/// them).
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Frame<const W: usize> {
    pub moves: Bits<W>,
    pub factors: Bits<W>,
    pub freedoms: i32,
}

impl<const W: usize> Frame<W> {
    /// A frame with freedoms `|factors| − |moves|`.
    pub fn new(moves: Bits<W>, factors: Bits<W>) -> Self {
        Frame { moves, factors, freedoms: factors.len() as i32 - moves.len() as i32 }
    }

    /// Whether some order takes every move (decided by the peel, see the module docs of `model`).
    pub fn is_winnable(&self, g: &FactorProvider) -> bool {
        winnable(&self.moves, &self.factors, g)
    }

    /// The Java's `Frame.reduce`: the frame with its forced pairs removed, freedoms recomputed from the sizes.
    pub fn reduce(&self, g: &FactorProvider) -> Self {
        let (mut moves, mut factors) = (self.moves, self.factors);
        let pending = Worklist::of(&moves);
        reduce_from(&mut moves, &mut factors, pending, g);
        Frame::new(moves, factors)
    }

    /// The Java's `Frame.promoteInto`: the frame with `n` added as a move, if it stays winnable; reduced, and
    /// with one freedom less.  Works on any frame, reduced or not.
    pub fn promote_into(&self, n: u32, g: &FactorProvider) -> Option<Self> {
        let mut moves = self.moves.with(n);
        let mut factors = self.factors;
        if !winnable(&moves, &factors, g) {
            return None;
        }
        let pending = Worklist::of(&moves);
        reduce_from(&mut moves, &mut factors, pending, g);
        Some(Frame { moves, factors, freedoms: self.freedoms - 1 })
    }

    /// The Java's `Frame.promoteOutOf`: the frame without the factor `n`, if it stays winnable; reduced, and
    /// with one freedom less.  `None` also when `n` is not a factor (the Java asserts).
    pub fn promote_out_of(&self, n: u32, g: &FactorProvider) -> Option<Self> {
        if !self.factors.contains(n) {
            return None;
        }
        let mut moves = self.moves;
        let mut factors = self.factors.without(n);
        if !winnable(&moves, &factors, g) {
            return None;
        }
        let pending = Worklist::of(&moves);
        reduce_from(&mut moves, &mut factors, pending, g);
        Some(Frame { moves, factors, freedoms: self.freedoms - 1 })
    }

    /// Whether [`promote_into`](Self::promote_into) would succeed.
    pub fn can_promote_into(&self, n: u32, g: &FactorProvider) -> bool {
        winnable(&self.moves.with(n), &self.factors, g)
    }

    /// Whether [`promote_out_of`](Self::promote_out_of) would succeed.
    pub fn can_promote_out_of(&self, n: u32, g: &FactorProvider) -> bool {
        self.factors.contains(n) && winnable(&self.moves, &self.factors.without(n), g)
    }
}

impl<const W: usize> fmt::Debug for Frame<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} -> {:?} ({} freedoms)", self.moves, self.factors, self.freedoms)
    }
}

/// The frames of a game, bottom (frame 0) first, inline: `Copy`, no heap.  Created by
/// [`Model::build`](super::Model::build); every
/// frame in it is reduced and winnable.  A search state is the root frame set with some candidates promoted.
/// Equality and hashing look at the frames in use only.
#[derive(Clone, Copy)]
pub struct FrameSet<const W: usize> {
    pub(super) len: usize,
    pub(super) frames: [Frame<W>; MAX_FRAMES],
}

impl<const W: usize> FrameSet<W> {
    /// Number of frames.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The frames, bottom first.
    #[inline]
    pub fn frames(&self) -> &[Frame<W>] {
        &self.frames[..self.len]
    }

    /// Makes `self` equal to `other`, copying only the frames in use (cheaper than assignment for the scratch
    /// states of a search).
    #[inline]
    pub fn assign(&mut self, other: &Self) {
        self.len = other.len;
        self.frames[..other.len].copy_from_slice(&other.frames[..other.len]);
    }

    /// The Java's `narrowCandidateSet`: the members of `candidates` that are factors of a frame `i ≥ 1` such
    /// that both frame `i` and frame `i − 1` have freedoms left (see the module docs).
    #[inline]
    pub fn narrow_candidates(&self, candidates: &Bits<W>) -> Bits<W> {
        let mut open = Bits::EMPTY;
        for pair in self.frames().windows(2) {
            if pair[1].freedoms > 0 && pair[0].freedoms > 0 {
                open |= pair[1].factors;
            }
        }
        open & *candidates
    }

    /// The Java's `computeMaxNumberOfPromotions()`: an upper bound on the number of further promotions, from
    /// the freedoms alone (see the module docs).
    #[inline]
    pub fn max_promotions(&self) -> i32 {
        self.max_promotions_with(|f| f.freedoms)
    }

    /// The Java's `computeMaxNumberOfPromotions(candidates)`: as [`max_promotions`](Self::max_promotions), but
    /// no frame promotes out more numbers than it has among `candidates`.
    #[inline]
    pub fn max_promotions_among(&self, candidates: &Bits<W>) -> i32 {
        self.max_promotions_with(|f| f.freedoms.min(f.factors.intersect(candidates).len() as i32))
    }

    /// Bottom up, each frame promotes out as many numbers as it can (`out_limit`) and the frame below can take.
    #[inline]
    fn max_promotions_with(&self, out_limit: impl Fn(&Frame<W>) -> i32) -> i32 {
        let mut total = 0;
        let mut room_below = 0;
        for f in self.frames() {
            let count = out_limit(f).min(room_below);
            total += count;
            room_below = f.freedoms - count;
        }
        total
    }
}

impl<const W: usize> PartialEq for FrameSet<W> {
    fn eq(&self, other: &Self) -> bool {
        self.frames() == other.frames()
    }
}

impl<const W: usize> Eq for FrameSet<W> {}

impl<const W: usize> Hash for FrameSet<W> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.frames().hash(state);
    }
}

impl<const W: usize> fmt::Debug for FrameSet<W> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.frames()).finish()
    }
}
