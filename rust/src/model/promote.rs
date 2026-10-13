//! Promotions: whether a number or a set can be promoted from a state, and the state after.
//!
//! Promoting `n` takes it out of the factors of its home frame `h ≥ 1` and adds it to the moves of frame `h − 1`.
//! It is legal when both frames stay winnable; both are then reduced, and each loses a freedom.
//!
//! # Sets are checked frame by frame
//!
//! Promoting a set `S` one member after the other succeeds, in any order, exactly when every member is a factor
//! of its home frame `h ≥ 1` and every frame is winnable with all of `S` applied at once: its moves plus the
//! members promoted into it, its factors minus the members promoted out of it, with no reduction in between
//! ([`Model::winnable`], [`Model::can_promote_all_using`]).  Three facts give this:
//!
//! * winnability only gets harder as moves are added and factors removed, so the frames with all of `S` applied
//!   are winnable exactly when every frame along the way is;
//! * reducing never changes whether a frame is winnable, now or after further edits (the reduction lemma in
//!   [`super::peel`]);
//! * a member that a reduction took as the factor of a forced pair fails both ways: one after the other it is no
//!   longer a factor of its home frame, and all at once the move of that pair is left with no factor.
//!
//! The state after promoting `S` is then the same in any order, since reduction is confluent on winnable frames,
//! and [`Model::promote_all_using`] makes it by checking and reducing each frame once.  Everything that reasons
//! about sets of promotions rests on this: conflicts and nogoods ([`super::conflict`], [`super::nogoods`]), the
//! matching bound's relaxation ([`crate::matchbound`]), and a search node rebuilt from its promotions alone.

use super::peel::{reduce_from, Certificate, Scratch, Worklist};
use super::{Frame, FrameSet, Model, MAX_FRAMES};
use crate::bits::Bits;

impl Model {
    /// Whether the frame `moves -> factors` is winnable, where `moves` and `factors` are those of a frame of this
    /// model with numbers promoted into and out of it (its moves plus factors of the frame above; its factors).
    /// Applied to each frame with a whole set promoted at once, this decides whether the set can be promoted
    /// (see the module docs).
    #[inline]
    pub fn winnable<const W: usize>(&self, moves: &Bits<W>, factors: &Bits<W>) -> bool {
        self.winnable_using(&mut Scratch::new(), moves, factors)
    }

    /// [`winnable`](Self::winnable) with caller-provided scratch space.
    #[inline]
    pub fn winnable_using<const W: usize>(&self, scratch: &mut Scratch<W>, moves: &Bits<W>, factors: &Bits<W>) -> bool {
        scratch.winnable(moves, factors, &self.graph)
    }

    /// Records a winning order of `frame` in `cert` (see [`Certificate`]).  False if the frame is not winnable,
    /// and then `cert` means nothing.
    pub fn certify_using<const W: usize>(
        &self,
        scratch: &mut Scratch<W>,
        frame: &Frame<W>,
        cert: &mut Certificate<W>,
    ) -> bool {
        scratch.certify(&frame.moves, &frame.factors, &self.graph, cert)
    }

    /// Whether `frame`, certified by `cert`, stays winnable with the move `x` added: from the certificate when it
    /// can tell, else by a peel of the moves from the step it gets stuck at.
    pub fn winnable_with_move_using<const W: usize>(
        &self,
        scratch: &mut Scratch<W>,
        frame: &Frame<W>,
        cert: &Certificate<W>,
        x: u32,
    ) -> bool {
        let live = || self.graph.factors(x).iter().map(|&q| u32::from(q)).filter(|&q| frame.factors.contains(q));
        let Some(first) = live().map(|q| cert.last(q)).min() else {
            return false; // a move with no factor can never be taken
        };
        if first < 0 || usize::from(cert.freed_at(first)) > live().filter(|&q| cert.last(q) == first).count() {
            return true;
        }
        scratch.winnable(&cert.moves_from_step(&frame.moves, first).with(x), &frame.factors, &self.graph)
    }

    /// Whether `frame`, certified by `cert`, stays winnable without its factor `y`: from the certificate when it
    /// can tell, else by a peel of the moves from the step that frees `y`.
    pub fn winnable_without_factor_using<const W: usize>(
        &self,
        scratch: &mut Scratch<W>,
        frame: &Frame<W>,
        cert: &Certificate<W>,
        y: u32,
    ) -> bool {
        if !frame.factors.contains(y) {
            return true;
        }
        let step = cert.last(y);
        if step < 0 || cert.freed_at(step) >= 2 {
            return true;
        }
        scratch.winnable(&cert.moves_from_step(&frame.moves, step), &frame.factors.without(y), &self.graph)
    }

    /// Whether `n` can be promoted: it is a factor of its home frame `h ≥ 1`, and frames `h − 1` (gaining the
    /// move) and `h` (losing the factor) both stay winnable.
    #[inline]
    pub fn can_promote<const W: usize>(&self, state: &FrameSet<W>, n: u32) -> bool {
        self.can_promote_using(&mut Scratch::new(), state, n)
    }

    /// [`can_promote`](Self::can_promote) with caller-provided scratch space.
    #[inline]
    pub fn can_promote_using<const W: usize>(&self, scratch: &mut Scratch<W>, state: &FrameSet<W>, n: u32) -> bool {
        self.promotable(scratch, state, n).is_some()
    }

    /// The home frame of `n` if `n` can be promoted.
    #[inline]
    fn promotable<const W: usize>(&self, scratch: &mut Scratch<W>, state: &FrameSet<W>, n: u32) -> Option<usize> {
        let h = self.home(n);
        if h == 0 || !self.contains_factor(state, n) {
            return None;
        }
        let (into, out) = (&state.frames[h - 1], &state.frames[h]);
        let ok = (self.has_unused_factor(into, n) || scratch.winnable(&into.moves.with(n), &into.factors, &self.graph))
            && scratch.winnable(&out.moves, &out.factors.without(n), &self.graph);
        ok.then_some(h)
    }

    /// Whether the move `x` has a live factor in the winnable `frame` that none of the frame's moves has: then the
    /// frame stays winnable with `x` added, which can be taken last (nothing before it takes that factor), and no
    /// peel is needed to know it.  The answer for a sixth of the promotions near n = 1200 (the peel decides the
    /// others; the same test for several moves promoted at once was measured, and paid less than it cost).
    #[inline]
    fn has_unused_factor<const W: usize>(&self, frame: &Frame<W>, x: u32) -> bool {
        let g = &self.graph;
        let unused = |q: u32| g.composites(q).iter().all(|&m| !frame.moves.contains(u32::from(m)));
        g.factors(x).iter().map(|&q| u32::from(q)).any(|q| frame.factors.contains(q) && unused(q))
    }

    /// The state after promoting `n`, or `None` if `n` cannot be promoted.
    #[inline]
    pub fn promote<const W: usize>(&self, state: &FrameSet<W>, n: u32) -> Option<FrameSet<W>> {
        let mut out = *state;
        self.promote_in_place(&mut out, n).then_some(out)
    }

    /// Promotes `n` in place and returns true, or returns false and leaves `state` unchanged.
    #[inline]
    pub fn promote_in_place<const W: usize>(&self, state: &mut FrameSet<W>, n: u32) -> bool {
        self.promote_in_place_using(&mut Scratch::new(), state, n)
    }

    /// [`promote_in_place`](Self::promote_in_place) with caller-provided scratch space.
    pub fn promote_in_place_using<const W: usize>(
        &self,
        scratch: &mut Scratch<W>,
        state: &mut FrameSet<W>,
        n: u32,
    ) -> bool {
        let Some(h) = self.promotable(scratch, state, n) else {
            return false;
        };
        let single = Bits::EMPTY.with(n);
        self.apply(&mut state.frames[h - 1], &single, &Bits::EMPTY);
        self.apply(&mut state.frames[h], &Bits::EMPTY, &single);
        true
    }

    /// Splits `set` by frame, from its largest member down: per frame, the members promoted into it and out of
    /// it.  Stops at the first member that is not a factor of a home frame `h ≥ 1` in `state` (so cannot be
    /// promoted at all) and returns it too; the split then covers only the members above it.
    pub fn edits<const W: usize>(&self, state: &FrameSet<W>, set: &Bits<W>) -> (Edits<W>, Option<u32>) {
        let mut edits = Edits { gained: [Bits::EMPTY; MAX_FRAMES], lost: [Bits::EMPTY; MAX_FRAMES] };
        for x in set.iter_desc() {
            let h = self.home(x);
            if h == 0 || !self.contains_factor(state, x) {
                return (edits, Some(x));
            }
            edits.gained[h - 1].insert(x);
            edits.lost[h].insert(x);
        }
        (edits, None)
    }

    /// The state after promoting every member of `set` (one by one in decreasing order, or in any other order:
    /// the result is the same), or `None` if that fails.  Instead of promoting one by one, each frame is checked
    /// and reduced once with all its promotions applied (see the module docs).
    pub fn promote_all_using<const W: usize>(
        &self,
        scratch: &mut Scratch<W>,
        state: &FrameSet<W>,
        set: &Bits<W>,
    ) -> Option<FrameSet<W>> {
        let (edits, None) = self.edits(state, set) else {
            return None;
        };
        if !self.frames_winnable(scratch, state, &edits) {
            return None;
        }
        let mut out = *state;
        for (i, frame) in out.frames[..out.len].iter_mut().enumerate() {
            let (gained, lost) = (&edits.gained[i], &edits.lost[i]);
            if !gained.is_empty() || !lost.is_empty() {
                self.apply(frame, gained, lost);
            }
        }
        Some(out)
    }

    /// Whether [`promote_all_using`](Self::promote_all_using) would succeed.
    pub fn can_promote_all_using<const W: usize>(
        &self,
        scratch: &mut Scratch<W>,
        state: &FrameSet<W>,
        set: &Bits<W>,
    ) -> bool {
        let (edits, None) = self.edits(state, set) else {
            return false;
        };
        self.frames_winnable(scratch, state, &edits)
    }

    /// Whether every frame of `state` stays winnable with `edits` applied.
    fn frames_winnable<const W: usize>(&self, scratch: &mut Scratch<W>, state: &FrameSet<W>, edits: &Edits<W>) -> bool {
        state.frames().iter().zip(edits.gained.iter().zip(&edits.lost)).all(|(frame, (gained, lost))| {
            (gained.is_empty() && lost.is_empty())
                || scratch.winnable(&(frame.moves | *gained), &(frame.factors - *lost), &self.graph)
        })
    }

    /// Adds `gained` to the moves of a winnable frame of this model and removes `lost` from its factors, then
    /// reduces it.  The frame was reduced, so only the new moves and the moves that lost a factor can be in a
    /// forced pair; on a winnable frame reduction is confluent, so starting there gives what a full sweep would.
    fn apply<const W: usize>(&self, frame: &mut Frame<W>, gained: &Bits<W>, lost: &Bits<W>) {
        let g = &self.graph;
        frame.moves |= *gained;
        frame.factors -= *lost;
        let mut pending = Worklist::of(gained);
        for y in lost.iter() {
            for &c in g.composites(y) {
                if frame.moves.contains(u32::from(c)) {
                    pending.insert(u32::from(c));
                }
            }
        }
        reduce_from(&mut frame.moves, &mut frame.factors, pending, g);
        frame.freedoms -= (gained.len() + lost.len()) as i32;
    }
}

/// A set of promotions split by frame (see [`Model::edits`]).
#[derive(Clone, Copy, Debug)]
pub struct Edits<const W: usize> {
    /// Per frame, the numbers promoted into it (its new moves).
    pub gained: [Bits<W>; MAX_FRAMES],
    /// Per frame, the numbers promoted out of it (the factors it loses).
    pub lost: [Bits<W>; MAX_FRAMES],
}
