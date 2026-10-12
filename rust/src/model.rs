//! The search state of the frame model: frames, frame sets, promotions, and the winnability test.
//!
//! A *frame* is a game of Taxman Mini.  Its moves are taken one at a time: each must still have one of its
//! maximal factors among the frame's factors when it is taken, and taking it removes all of its factors still
//! there (its tax).  A frame is *winnable* when some order takes every move.  A [`FrameSet`] stacks the frames of
//! a game, bottom (frame 0) first.  *Promoting* `n` takes it out of the factors of its home frame `h` and makes
//! it a move of frame `h − 1`; that is legal when both frames stay winnable.  Afterwards both frames are
//! *reduced*: a move left with a single live factor must be paid by it, so the two are dropped, repeatedly.
//! Every frame's *freedoms* (initially factors minus moves) drop by one on each promotion into or out of it.
//!
//! This is the Java solver's `Frame`, `FrameSet`, `FrameMapper` (home frames only) and `FactorProvider`
//! (package `org.taxman.h6.search`), reimplemented for speed rather than translated: frames are inline bitsets,
//! a [`FrameSet`] is `Copy` and never allocates, and promotions can be done in place.  The results are the
//! Java's (the golden tests check them); what differs is how they are computed:
//!
//! * Winnability.  A move with a *private* live factor (one no other move has) can always be taken last, and
//!   dropping it leaves a winnable frame winnable; conversely the last move of any winning order has a private
//!   factor.  So a frame is winnable exactly when repeatedly dropping moves with a private factor drops them
//!   all, in any order.  The Java peels with that rule plus "a move with a single live factor is taken first"
//!   and a pigeonhole test; those preserve winnability too, so the answers agree (the tests check this against
//!   the Java's procedure and, on small frames, against an exhaustive search of orders).  The dropping is done
//!   as in an invertible Bloom lookup table: each factor keeps the count and the XOR of its live moves, so a
//!   factor with count 1 names its move.
//! * Frames inside a [`FrameSet`] are always reduced, so after a promotion only the moves the promotion touched
//!   can start a reduction; [`Model`] starts there instead of sweeping the whole frame.  On winnable frames
//!   reduction is confluent, so the result is the one a full sweep gives.
//! * The factor and composite lists a [`Model`] uses are restricted to the frames each number can take part in
//!   (as a move of one frame, as a factor of another).  That keeps every live neighbour and drops the rest.
//!
//! Numbers are `u32` game numbers throughout; the lists store them as `u16`, which bounds boards at 65535.

use crate::bits::Bits;
use crate::numtheory::Tables;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Most frames a [`FrameSet`] can hold.  Boards up to 760 have at most 6.
pub const MAX_FRAMES: usize = 12;

// ---------------------------------------------------------------------------------------------------------
// Factor provider

/// Most maximal factors a supported number has: 2·3·5·7·11·13 = 30030 has six distinct primes, and seven need
/// 510510, beyond `u16`.
const MAX_FACTORS: usize = 6;

/// For each number, the maximal factors that can pay for it (inline: the list, then its length in the last slot)
/// and the maximal composites it can pay for (flat CSR arrays).  Numbers beyond the provider's range have empty
/// lists.
#[derive(Clone, Debug)]
pub struct FactorProvider {
    fac: Vec<[u16; MAX_FACTORS + 1]>,
    comp_start: Vec<u32>,
    comp: Vec<u16>,
}

impl FactorProvider {
    /// The Java's `FactorProvider.create(board, numbers)`: each number's maximal factors and maximal composites,
    /// restricted to `numbers`.  `tables` must cover the largest of `numbers`.
    pub fn new(tables: &Tables, numbers: &[u32]) -> Self {
        let max = numbers.iter().copied().max().unwrap_or(0);
        let mut member = vec![0u16; max as usize + 1];
        for &x in numbers {
            member[x as usize] = 1;
        }
        Self::with_roles(tables, &member, &member)
    }

    /// Lists for a graph in which `y` can pay for `x` exactly when `y` is a maximal factor of `x` and
    /// `as_move[x] & as_factor[y] != 0` (bit `i` standing for frame `i`, say).  Both slices have one entry per
    /// number `0..=max`.
    fn with_roles(tables: &Tables, as_move: &[u16], as_factor: &[u16]) -> Self {
        assert_eq!(as_move.len(), as_factor.len());
        let max = as_move.len() - 1;
        assert!(max as u32 <= tables.n, "tables for {} cannot cover {max}", tables.n);
        assert!(max <= usize::from(u16::MAX), "numbers above {} are not supported", u16::MAX);
        let mut fac = vec![[0u16; MAX_FACTORS + 1]; max + 1];
        let mut comp_start = Vec::with_capacity(max + 2);
        let mut comp = Vec::new();
        comp_start.push(0);
        for x in 0..=max {
            if as_move[x] != 0 {
                let mine = as_move[x];
                let payers = tables.max_factors[x].iter().filter(|&&y| as_factor[y as usize] & mine != 0);
                for (k, &y) in payers.enumerate() {
                    fac[x][k] = y as u16;
                    fac[x][MAX_FACTORS] = k as u16 + 1;
                }
            }
            if as_factor[x] != 0 {
                let mine = as_factor[x];
                let composites = tables.max_composites[x].iter().take_while(|&&c| c as usize <= max);
                comp.extend(composites.filter(|&&c| as_move[c as usize] & mine != 0).map(|&c| c as u16));
            }
            comp_start.push(comp.len() as u32);
        }
        FactorProvider { fac, comp_start, comp }
    }

    /// The maximal factors of `n` that can pay for it, ascending.
    #[inline]
    pub fn factors(&self, n: u32) -> &[u16] {
        match self.fac.get(n as usize) {
            Some(list) => &list[..usize::from(list[MAX_FACTORS]).min(MAX_FACTORS)],
            None => &[],
        }
    }

    /// The maximal composites of `n` that it can pay for, ascending.
    #[inline]
    pub fn composites(&self, n: u32) -> &[u16] {
        let i = n as usize;
        if i + 1 < self.comp_start.len() {
            &self.comp[self.comp_start[i] as usize..self.comp_start[i + 1] as usize]
        } else {
            &[]
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Peeling and reduction

/// A set of pending numbers that pops its largest member in constant time: the words of a bitset plus a summary
/// word with bit `i` set when word `i` is non-empty.  Needs `W <= 64`, that is boards up to 4095.
struct Worklist<const W: usize> {
    words: [u64; W],
    summary: u64,
}

impl<const W: usize> Worklist<W> {
    const FITS: () = assert!(W <= 64, "a worklist summarizes at most 64 words");

    #[inline]
    fn new() -> Self {
        let () = Self::FITS;
        Worklist { words: [0; W], summary: 0 }
    }

    #[inline]
    fn of(set: &Bits<W>) -> Self {
        let mut out = Self::new();
        out.words = *set.as_words();
        for (i, &w) in out.words.iter().enumerate() {
            out.summary |= u64::from(w != 0) << i;
        }
        out
    }

    #[inline]
    fn insert(&mut self, x: u32) {
        let i = (x >> 6) as usize;
        self.words[i] |= 1 << (x & 63);
        self.summary |= 1 << i;
    }

    #[inline]
    fn pop(&mut self) -> Option<u32> {
        if self.summary == 0 {
            return None;
        }
        let i = 63 - self.summary.leading_zeros() as usize;
        let word = self.words[i];
        let bit = 63 - word.leading_zeros();
        let rest = word & !(1u64 << bit);
        self.words[i] = rest;
        if rest == 0 {
            self.summary &= !(1u64 << i);
        }
        Some(((i as u32) << 6) | bit)
    }
}

/// How many members of `list` are in `live`, counting no further than 2, and the last one found.
#[inline]
fn live_neighbours<const W: usize>(list: &[u16], live: &Bits<W>) -> (u32, u32) {
    let mut count = 0;
    let mut last = 0;
    for &x in list {
        let x = u32::from(x);
        if live.contains(x) {
            count += 1;
            last = x;
            if count == 2 {
                break;
            }
        }
    }
    (count, last)
}

/// One live move in a [`Scratch`] cell: the count lives in the high 16 bits.
const ONE_MOVE: u32 = 1 << 16;

/// Scratch space for winnability checks.  Making one zeroes `512·W` bytes, so code that checks many promotions
/// keeps one (one per thread) and passes it to the `*_using` methods of [`Model`]; every check leaves it ready
/// for the next.
///
/// Inside: one cell per number, holding for a factor the count of its live moves (high 16 bits) and the XOR of
/// their numbers (low 16 bits), all zero between checks; and a stack of factors to look at.
pub struct Scratch<const W: usize> {
    cells: [[u32; 64]; W],
    /// A factor is pushed when its count rises to 1 (adding moves) and when it falls to 1 (removing them): at
    /// most twice, so `2·64·W` entries suffice.
    stack: [[u16; 128]; W],
}

impl<const W: usize> Default for Scratch<W> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const W: usize> Scratch<W> {
    #[inline]
    pub fn new() -> Self {
        Scratch { cells: [[0; 64]; W], stack: [[0; 128]; W] }
    }

    /// Whether the frame `moves -> factors` is winnable: drops moves with a private live factor until none is
    /// left (winnable) or none can be dropped (see the module docs).
    fn winnable(&mut self, moves: &Bits<W>, factors: &Bits<W>, g: &FactorProvider) -> bool {
        let mut left = moves.len();
        if left > factors.len() {
            return false;
        }
        if left == 0 {
            return true;
        }
        let cells = self.cells.as_flattened_mut();
        let stack = self.stack.as_flattened_mut();
        let mut top = 0;
        // Count every move into the cells of its live factors, pushing each factor whose count reaches 1.
        for m in moves.iter() {
            let mut live = false;
            for &q in g.factors(m) {
                if factors.contains(u32::from(q)) {
                    live = true;
                    let cell = &mut cells[usize::from(q)];
                    *cell = (*cell + ONE_MOVE) ^ m;
                    if *cell >> 16 == 1 {
                        stack[top] = q;
                        top += 1;
                    }
                }
            }
            if !live {
                Self::clear(cells, factors);
                return false;
            }
        }
        // A factor with a single live move names it; that move can be taken last, so drop it.  While a move is
        // counted, the cells of its live factors are non-zero and those of its other factors zero.
        while top > 0 {
            top -= 1;
            let cell = cells[usize::from(stack[top])];
            if cell >> 16 != 1 {
                continue;
            }
            let m = cell & 0xFFFF;
            for &q in g.factors(m) {
                let cell = &mut cells[usize::from(q)];
                if *cell != 0 {
                    *cell = (*cell - ONE_MOVE) ^ m;
                    if *cell >> 16 == 1 {
                        stack[top] = q;
                        top += 1;
                    }
                }
            }
            left -= 1;
            if left == 0 {
                return true; // every move was counted in and out, so every cell is zero again
            }
        }
        Self::clear(cells, factors);
        false
    }

    fn clear(cells: &mut [u32], factors: &Bits<W>) {
        for q in factors.iter() {
            cells[q as usize] = 0;
        }
    }
}

/// Whether the frame `moves -> factors` is winnable.
#[inline]
fn winnable<const W: usize>(moves: &Bits<W>, factors: &Bits<W>, g: &FactorProvider) -> bool {
    Scratch::new().winnable(moves, factors, g)
}

/// Reduction: drops each move with a single live factor together with that factor, until none is left,
/// examining `pending` and whatever the drops affect.  A move with no live factor is dropped alone, as the Java
/// does with assertions off (it cannot happen on a winnable frame).
fn reduce_from<const W: usize>(
    moves: &mut Bits<W>,
    factors: &mut Bits<W>,
    mut pending: Worklist<W>,
    g: &FactorProvider,
) {
    while let Some(m) = pending.pop() {
        if !moves.contains(m) {
            continue;
        }
        let (count, q) = live_neighbours(g.factors(m), factors);
        if count < 2 {
            moves.remove(m);
            if count == 1 {
                factors.remove(q);
                for &c in g.composites(q) {
                    if moves.contains(u32::from(c)) {
                        pending.insert(u32::from(c));
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------------------
// Frame

/// One frame: its moves, its factors, and its freedoms.  Freedoms start at `|factors| − |moves|` and drop by one
/// on every promotion into or out of the frame; frame 0's can also be lowered by
/// [`FrameSet::with_frame_zero_freedoms`], so they can go negative.  Equality includes the freedoms (the Java's
/// `Frame.equals` ignores them).
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

    /// Whether some order takes every move (see the module docs).
    pub fn is_winnable(&self, g: &FactorProvider) -> bool {
        winnable(&self.moves, &self.factors, g)
    }

    /// The Java's `Frame.reduce`: the frame with forced pairs removed, freedoms recomputed from the sizes.
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

// ---------------------------------------------------------------------------------------------------------
// Frame set

/// The frames of a game, bottom (frame 0) first, inline: `Copy`, no heap.  Created by [`Model::build`]; every
/// frame in it is reduced and winnable.  Equality and hashing look at the frames in use only.
#[derive(Clone, Copy)]
pub struct FrameSet<const W: usize> {
    len: usize,
    frames: [Frame<W>; MAX_FRAMES],
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
    /// that both frame `i` and frame `i − 1` have freedoms left.
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
    /// the freedoms alone.
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

    /// The Java's `setFrameZeroFreedoms`: this frame set with frame 0's freedoms replaced.
    #[must_use]
    pub fn with_frame_zero_freedoms(&self, freedoms: i32) -> Self {
        let mut out = *self;
        if out.len > 0 {
            out.frames[0].freedoms = freedoms;
        }
        out
    }

    /// The two-frame game `{frame 0, frame 1}` (fewer if there are fewer frames), for the same [`Model`].
    #[must_use]
    pub fn front_pair(&self) -> Self {
        let mut out = FrameSet { len: self.len.min(2), frames: [Frame::default(); MAX_FRAMES] };
        out.frames[..out.len].copy_from_slice(&self.frames[..out.len]);
        out
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

// ---------------------------------------------------------------------------------------------------------
// Model

/// What every state of one game's search shares: the factor graph and each number's home frame.  Immutable
/// and `Sync`, so search threads can share one.
#[derive(Clone, Debug)]
pub struct Model {
    graph: FactorProvider,
    /// Home frame of each number: the frame whose factors held it when the frame set was built; 0 for numbers
    /// that were not a factor anywhere (as the Java's `FrameMapper`).
    home: Vec<u8>,
}

impl Model {
    /// The Java's `FrameSet.create`: `columns` alternate factors and moves per frame, bottom first; each frame
    /// is reduced.  Returns the model and the starting frame set.
    pub fn build<const W: usize>(tables: &Tables, columns: &[Vec<u32>]) -> Result<(Model, FrameSet<W>), String> {
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
        Ok((Model { graph, home }, root))
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

    /// Whether the frame `moves -> factors` is winnable, where `moves` and `factors` are those of a frame of this
    /// model with numbers promoted into and out of it (its moves plus factors of the frame above; its factors).
    ///
    /// Promoting a set of numbers, each a factor of its home frame `h ≥ 1`, one after the other succeeds exactly
    /// when every frame is winnable with all of them promoted at once (its moves plus those promoted into it, its
    /// factors minus those promoted out, unreduced): winnability only gets harder as moves are added and factors
    /// removed, and reducing never changes it.  So a set can be checked frame by frame without promoting it.
    #[inline]
    pub fn winnable<const W: usize>(&self, moves: &Bits<W>, factors: &Bits<W>) -> bool {
        self.winnable_using(&mut Scratch::new(), moves, factors)
    }

    /// [`winnable`](Self::winnable) with caller-provided scratch space.
    #[inline]
    pub fn winnable_using<const W: usize>(&self, scratch: &mut Scratch<W>, moves: &Bits<W>, factors: &Bits<W>) -> bool {
        scratch.winnable(moves, factors, &self.graph)
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
        let ok = scratch.winnable(&into.moves.with(n), &into.factors, &self.graph)
            && scratch.winnable(&out.moves, &out.factors.without(n), &self.graph);
        ok.then_some(h)
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
        let g = &self.graph;
        let (below, above) = state.frames.split_at_mut(h);
        let (into, out) = (&mut below[h - 1], &mut above[0]);

        // Both frames were reduced, so only n (a new move) and the moves that just lost n can be forced.
        into.moves.insert(n);
        let mut pending = Worklist::new();
        pending.insert(n);
        reduce_from(&mut into.moves, &mut into.factors, pending, g);
        into.freedoms -= 1;

        out.factors.remove(n);
        let mut pending = Worklist::new();
        for &c in g.composites(n) {
            if out.moves.contains(u32::from(c)) {
                pending.insert(u32::from(c));
            }
        }
        reduce_from(&mut out.moves, &mut out.factors, pending, g);
        out.freedoms -= 1;
        true
    }
}

// ---------------------------------------------------------------------------------------------------------
// Tests

/// Helpers shared by the tests of this module, `bounds` and `matchbound`.
#[cfg(test)]
pub(crate) mod test_support {
    use super::{FrameSet, Model};
    use crate::bits::Bits;
    use crate::json::Json;
    use std::path::PathBuf;
    use std::time::Instant;

    /// Tiny xorshift64 generator, so the tests need no crates and are reproducible.
    pub(crate) struct Rng(pub u64);

    impl Rng {
        pub(crate) fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }

        pub(crate) fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }

        /// True with probability `percent`/100.
        pub(crate) fn chance(&mut self, percent: u64) -> bool {
            self.next() % 100 < percent
        }

        pub(crate) fn pick<T: Copy>(&mut self, items: &[T]) -> T {
            items[self.below(items.len())]
        }

        pub(crate) fn shuffle<T>(&mut self, items: &mut [T]) {
            for i in (1..items.len()).rev() {
                items.swap(i, self.below(i + 1));
            }
        }

        /// A random subset of `set`, each member kept with probability `percent`/100.
        pub(crate) fn subset<const W: usize>(&mut self, set: &Bits<W>, percent: u64) -> Bits<W> {
            set.iter().filter(|_| self.chance(percent)).collect()
        }
    }

    /// One golden file: the Java's intermediate results for game `n`.
    pub(crate) struct Golden {
        pub n: u32,
        /// Factors and moves per frame, bottom first: the columns the Java builds its `FrameSet` from.
        pub columns: Vec<Vec<u32>>,
        pub candidates: Vec<u32>,
        pub json: Json,
    }

    pub(crate) fn golden_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata").join("golden")
    }

    /// The board sizes with a golden file, ascending.
    pub(crate) fn golden_ns() -> Vec<u32> {
        let mut ns: Vec<u32> = std::fs::read_dir(golden_dir())
            .expect("golden directory")
            .filter_map(|e| {
                let name = e.ok()?.file_name().into_string().ok()?;
                name.strip_prefix('n')?.strip_suffix(".json")?.parse().ok()
            })
            .collect();
        ns.sort_unstable();
        ns
    }

    pub(crate) fn u32s(json: &Json, key: &str) -> Vec<u32> {
        json.get(key).and_then(Json::as_u32_vec).unwrap_or_else(|| panic!("golden field {key}"))
    }

    pub(crate) fn load(n: u32) -> Golden {
        let json = Json::from_file(golden_dir().join(format!("n{n:04}.json"))).expect("golden file");
        assert!(json.get("error").is_none(), "golden n={n} recorded an error");
        let mut columns = Vec::new();
        for frame in json.get("frames").and_then(Json::as_arr).expect("golden frames") {
            columns.push(u32s(frame, "remainingFactors"));
            columns.push(u32s(frame, "remainingSources"));
        }
        let candidates = u32s(&json, "allCandidates");
        Golden { n, columns, candidates, json }
    }

    /// The largest sum of at most `max_size` members of `cands` that can be promoted from `state` (exhaustive,
    /// pruned only by sums): the truth that upper bounds are checked against.
    pub(crate) fn best_completion<const W: usize>(
        model: &Model,
        state: &FrameSet<W>,
        cands: &Bits<W>,
        max_size: u32,
    ) -> u64 {
        fn dfs<const W: usize>(model: &Model, state: &FrameSet<W>, list: &[u32], sum: u64, left: u32, best: &mut u64) {
            *best = (*best).max(sum);
            if left == 0 {
                return;
            }
            for i in 0..list.len() {
                let optimistic: u64 = sum + list[i..].iter().take(left as usize).map(|&x| u64::from(x)).sum::<u64>();
                if optimistic <= *best {
                    return;
                }
                let mut next = *state;
                if model.promote_in_place(&mut next, list[i]) {
                    dfs(model, &next, &list[i + 1..], sum + u64::from(list[i]), left - 1, best);
                }
            }
        }
        let list: Vec<u32> = cands.iter_desc().collect();
        let mut best = 0;
        dfs(model, state, &list, 0, max_size, &mut best);
        best
    }

    /// For micro-benchmarks: nanoseconds per call of `f`, which makes `calls` calls: the best of seven trials of
    /// about 0.15 s each, so that other load on the machine disturbs the figure less.
    pub(crate) fn time_per_call(calls: u64, mut f: impl FnMut()) -> f64 {
        let mut best = f64::INFINITY;
        for _ in 0..7 {
            let start = Instant::now();
            let mut rounds = 0u64;
            while start.elapsed().as_secs_f64() < 0.15 {
                f();
                rounds += 1;
            }
            best = best.min(start.elapsed().as_secs_f64() * 1e9 / (rounds * calls) as f64);
        }
        best
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use crate::json::Json;

    fn bits<const W: usize>(xs: &[u32]) -> Bits<W> {
        Bits::from_slice(xs)
    }

    // ---- the Java's algorithms, transliterated, as the reference ----

    /// `Frame.isWinnable`, step for step.
    fn ref_is_winnable<const W: usize>(moves: Bits<W>, factors: Bits<W>, math: &FactorProvider) -> bool {
        let (mut rm, mut rf) = (moves, factors);
        if rm.len() > rf.len() {
            return false;
        }
        if rm.is_empty() {
            return true;
        }
        let mut progress = true;
        while progress {
            progress = false;
            let mut cursor = rm.max();
            while let Some(m) = cursor {
                let live: Vec<u32> =
                    math.factors(m).iter().map(|&q| u32::from(q)).filter(|&q| rf.contains(q)).collect();
                match live.len() {
                    0 => return false,
                    1 => {
                        rm.remove(m);
                        if rm.is_empty() {
                            return true;
                        }
                        rf.remove(live[0]);
                        progress = true;
                    }
                    _ => {}
                }
                cursor = rm.max_below(m);
            }
            let mut cursor = rf.max();
            while let Some(f) = cursor {
                let live: Vec<u32> =
                    math.composites(f).iter().map(|&c| u32::from(c)).filter(|&c| rm.contains(c)).collect();
                if live.len() == 1 {
                    rm.remove(live[0]);
                    if rm.is_empty() {
                        return true;
                    }
                    rf.remove(f);
                    progress = true;
                }
                cursor = rf.max_below(f);
            }
        }
        false
    }

    /// `Frame.reduce(movesToKeep, factorsToKeep, math)`, step for step (with Java assertions off).
    fn ref_reduce<const W: usize>(moves: Bits<W>, factors: Bits<W>, math: &FactorProvider) -> (Bits<W>, Bits<W>) {
        let (mut rm, mut rf) = (moves, factors);
        let mut progress = true;
        while progress {
            if rm.is_empty() {
                break;
            }
            progress = false;
            let mut cursor = rm.max();
            while let Some(m) = cursor {
                let live: Vec<u32> =
                    math.factors(m).iter().map(|&q| u32::from(q)).filter(|&q| rf.contains(q)).collect();
                if live.len() <= 1 {
                    rm.remove(m);
                    if let Some(&q) = live.first() {
                        rf.remove(q);
                    }
                    progress = true;
                }
                cursor = rm.max_below(m);
            }
        }
        (rm, rf)
    }

    /// Most moves [`tax_winnable`] handles.
    const BRUTE_MOVES: usize = 14;

    /// The meaning of winnable, by exhaustion: the moves can be taken in some order so that each still has a
    /// factor present when it is taken, where taking a move removes all of its factors still present (its tax).
    /// The factors used up depend only on which moves have been taken, so this searches subsets of moves.
    fn tax_winnable<const W: usize>(moves: Bits<W>, factors: Bits<W>, math: &FactorProvider) -> bool {
        let ms = moves.to_vec();
        let k = ms.len();
        assert!(k <= BRUTE_MOVES);
        let tax: Vec<Bits<W>> = ms
            .iter()
            .map(|&m| math.factors(m).iter().map(|&q| u32::from(q)).filter(|&q| factors.contains(q)).collect())
            .collect();
        let full = (1usize << k) - 1;
        let mut used = vec![Bits::<W>::EMPTY; full + 1];
        let mut reached = vec![false; full + 1];
        reached[0] = true;
        for taken in 0..full {
            if !reached[taken] {
                continue;
            }
            for (j, t) in tax.iter().enumerate() {
                let next = taken | 1 << j;
                if next != taken && !t.is_subset(&used[taken]) {
                    reached[next] = true;
                    used[next] = used[taken] | *t;
                }
            }
        }
        reached[full]
    }

    // ---- the Java's unit tests ----

    #[test]
    fn java_frame_test_cant_promote_if_its_not_there() {
        let t = Tables::new(200);
        let math = FactorProvider::new(&t, &[108, 72, 54, 36, 10]);
        let frame = Frame::<4>::new(bits(&[108, 72]), bits(&[54, 36]));
        assert!(frame.promote_out_of(10, &math).is_none());
        assert!(!frame.can_promote_out_of(10, &math));
    }

    #[test]
    fn java_frame_test_tiny() {
        let t = Tables::new(200);
        let math = FactorProvider::new(&t, &[108, 72, 54, 36, 10]);
        let frame = Frame::<4>::new(bits(&[108, 72]), bits(&[54, 36, 10]));
        let result = frame.promote_out_of(10, &math).expect("promotable");
        assert!(result.factors.is_empty() && result.moves.is_empty());
        assert_eq!(result.freedoms, 0);
    }

    #[test]
    fn java_frame_test_derived_from_120() {
        let t = Tables::new(200);
        let moves = [69, 74, 77, 82, 85, 86, 87, 91, 93, 94, 95, 106, 111, 115, 118, 11];
        let factors = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59];
        let mut numbers: Vec<u32> = moves.iter().chain(&factors).copied().collect();
        numbers.push(39);
        let math = FactorProvider::new(&t, &numbers);
        let frame = Frame::<4>::new(bits(&moves), bits(&factors));
        assert!(frame.promote_into(9, &math).is_none());
        assert!(!frame.can_promote_into(9, &math));
    }

    fn columns_85() -> Vec<Vec<u32>> {
        vec![
            vec![4, 6, 9, 10, 14, 15, 21, 25],
            vec![45, 50, 63, 75],
            vec![8, 12, 20, 28, 30, 42],
            vec![56, 60, 84],
            vec![16, 24, 36, 40],
            vec![48, 72, 80],
        ]
    }

    const CANDIDATES_85: [u32; 10] = [8, 12, 16, 20, 24, 28, 30, 36, 40, 42];

    #[test]
    fn java_frame_set_test_promote() {
        let (model, fs) = Model::build::<2>(&Tables::new(85), &columns_85()).unwrap();
        let mut state = fs;
        for n in [42, 40, 28] {
            state = model.promote(&state, n).expect("promotable");
        }
        let expect: [(&[u32], &[u32]); 3] =
            [(&[4, 6, 9, 10, 14, 15, 21, 25], &[28, 42, 45, 50, 63, 75]), (&[], &[]), (&[], &[])];
        assert_eq!(state.len(), 3);
        for (frame, (factors, moves)) in state.frames().iter().zip(expect) {
            assert_eq!((frame.factors.to_vec(), frame.moves.to_vec()), (factors.to_vec(), moves.to_vec()));
        }
    }

    #[test]
    fn java_frame_set_test_candidates_and_max_moves() {
        let (model, fs) = Model::build::<2>(&Tables::new(85), &columns_85()).unwrap();
        let candidates = bits::<2>(&CANDIDATES_85);
        assert_eq!(fs.narrow_candidates(&candidates), candidates);
        assert_eq!(fs.max_promotions(), 3);
        let state = [42, 40, 28].iter().fold(fs, |s, &n| model.promote(&s, n).unwrap());
        assert_eq!(state.max_promotions(), 0);
    }

    #[test]
    fn frame_set_helpers() {
        let (model, fs) = Model::build::<2>(&Tables::new(85), &columns_85()).unwrap();
        assert_eq!((model.home(42), model.home(40), model.home(9), model.home(84), model.home(999)), (1, 2, 0, 0, 0));
        assert!(model.contains_factor(&fs, 42) && !model.contains_factor(&fs, 84) && !model.contains_factor(&fs, 999));
        assert!(!model.can_promote(&fs, 9) && model.promote(&fs, 9).is_none(), "frame 0 cannot promote");
        let pair = fs.front_pair();
        assert_eq!(pair.frames(), &fs.frames()[..2]);
        assert!(model.contains_factor(&pair, 42) && !model.contains_factor(&pair, 40));
        let revised = fs.with_frame_zero_freedoms(1);
        assert_eq!(revised.frames()[0].freedoms, 1);
        assert_eq!(&revised.frames()[1..], &fs.frames()[1..]);
        assert_ne!(revised, fs);
        let mut copy = pair;
        copy.assign(&fs);
        assert_eq!(copy, fs);
        let mut failed = fs;
        assert!(!model.promote_in_place(&mut failed, 9));
        assert_eq!(failed, fs);
    }

    // ---- golden data ----

    fn check_golden_frame_set<const W: usize>(g: &Golden) {
        let (_, fs) = Model::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
        let expect = g.json.get("frameSet").and_then(Json::as_arr).expect("frameSet");
        assert_eq!(fs.len(), expect.len(), "n={}", g.n);
        for (i, (frame, want)) in fs.frames().iter().zip(expect).enumerate() {
            assert_eq!(frame.moves.to_vec(), u32s(want, "moves"), "n={} frame {i} moves", g.n);
            assert_eq!(frame.factors.to_vec(), u32s(want, "factors"), "n={} frame {i} factors", g.n);
            assert_eq!(Some(i64::from(frame.freedoms)), want.get("freedoms").and_then(Json::as_i64), "n={}", g.n);
        }
        let candidates = Bits::<W>::from_slice(&g.candidates);
        let max = g.json.get("maxPromotionsUnrevised").and_then(Json::as_i64).expect("maxPromotionsUnrevised");
        assert_eq!(i64::from(fs.max_promotions_among(&candidates)), max, "n={}", g.n);
    }

    #[test]
    fn golden_frame_sets_and_max_promotions() {
        let ns = golden_ns();
        assert!(ns.len() >= 300, "expected the golden data set, found {} files", ns.len());
        for n in ns {
            let g = load(n);
            if n < 512 {
                check_golden_frame_set::<8>(&g);
            } else {
                check_golden_frame_set::<16>(&g);
            }
        }
    }

    // ---- randomized agreement with the reference ----

    /// Per frame of a golden game: the numbers that can be its moves and its factors.
    fn universes<const W: usize>(columns: &[Vec<u32>]) -> Vec<(Bits<W>, Bits<W>)> {
        let len = columns.len() / 2;
        (0..len)
            .map(|i| {
                let mut moves = Bits::from_slice(&columns[2 * i + 1]);
                if i + 1 < len {
                    moves |= Bits::from_slice(&columns[2 * i + 2]);
                }
                (moves, Bits::from_slice(&columns[2 * i]))
            })
            .collect()
    }

    #[derive(Default, Debug)]
    struct PeelStats {
        checks: u64,
        winnable: u64,
        /// Checks small enough to compare with the exhaustive [`tax_winnable`].
        exhaustive: u64,
    }

    /// Checks peeling and reduction on `(moves, factors)` against the reference, and peeling against the
    /// exhaustive definition when the frame is small enough.
    fn check_frame<const W: usize>(
        moves: Bits<W>,
        factors: Bits<W>,
        graph: &FactorProvider,
        math: &FactorProvider,
        stats: &mut PeelStats,
    ) -> bool {
        let fast = winnable(&moves, &factors, graph);
        let reference = ref_is_winnable(moves, factors, math);
        assert_eq!(fast, reference, "peel disagrees on {moves:?} -> {factors:?}");
        assert_eq!(winnable(&moves, &factors, math), reference, "peel with the union lists disagrees");
        if moves.len() as usize <= BRUTE_MOVES {
            let truth = tax_winnable(moves, factors, math);
            assert_eq!(reference, truth, "peeling is not a decision procedure on {moves:?} -> {factors:?}");
            stats.exhaustive += 1;
        }
        stats.checks += 1;
        if fast {
            stats.winnable += 1;
            let (rm, rf) = ref_reduce(moves, factors, math);
            let reduced = Frame::new(moves, factors).reduce(graph);
            assert_eq!((reduced.moves, reduced.factors), (rm, rf), "reduce disagrees on {moves:?} -> {factors:?}");
        }
        fast
    }

    fn random_agreement<const W: usize>(g: &Golden, rng: &mut Rng, stats: &mut PeelStats) {
        let tables = Tables::new(g.n);
        let (model, root) = Model::build::<W>(&tables, &g.columns).unwrap();
        let all: Vec<u32> = g.columns.iter().flatten().copied().collect();
        let math = FactorProvider::new(&tables, &all);
        let universes = universes::<W>(&g.columns);
        let candidates = Bits::<W>::from_slice(&g.candidates);

        for _walk in 0..6 {
            let mut state = root;
            loop {
                for (i, frame) in state.frames().iter().enumerate() {
                    let (move_universe, factor_universe) = universes[i];
                    assert!(check_frame(frame.moves, frame.factors, &model.graph, &math, stats));
                    let extra: Vec<u32> = (move_universe - frame.moves).to_vec();
                    let present: Vec<u32> = frame.factors.to_vec();
                    for _ in 0..4 {
                        if !extra.is_empty() {
                            let x = rng.pick(&extra);
                            let moves = frame.moves.with(x);
                            if check_frame(moves, frame.factors, &model.graph, &math, stats) {
                                // The incremental reduction a promotion does equals a full one.
                                let (mut m, mut f) = (moves, frame.factors);
                                let mut pending = Worklist::new();
                                pending.insert(x);
                                reduce_from(&mut m, &mut f, pending, &model.graph);
                                assert_eq!((m, f), ref_reduce(moves, frame.factors, &math));
                                let java = frame.promote_into(x, &math).unwrap();
                                assert_eq!((java.moves, java.factors), (m, f));
                            }
                        }
                        if !present.is_empty() {
                            let y = rng.pick(&present);
                            let factors = frame.factors.without(y);
                            if check_frame(frame.moves, factors, &model.graph, &math, stats) {
                                let java = frame.promote_out_of(y, &math).unwrap();
                                assert_eq!((java.moves, java.factors), ref_reduce(frame.moves, factors, &math));
                            }
                        }
                        // Arbitrary frames over the same numbers, reachable or not: one of any size, and one
                        // small enough for the exhaustive check.
                        let density = 20 + rng.below(70) as u64;
                        let moves = rng.subset(&move_universe, density);
                        let factors = rng.subset(&factor_universe, (density + 15).min(100));
                        check_frame(moves, factors, &model.graph, &math, stats);
                        let pool = move_universe.to_vec();
                        let count = if pool.is_empty() { 0 } else { rng.below(BRUTE_MOVES + 1) };
                        let moves: Bits<W> = (0..count).map(|_| rng.pick(&pool)).collect();
                        let density = 40 + rng.below(61) as u64;
                        let factors = rng.subset(&factor_universe, density);
                        check_frame(moves, factors, &model.graph, &math, stats);
                    }
                }

                // Promote a random candidate (any order), checking the model against the Java's frame operations.
                let open: Vec<u32> = state.narrow_candidates(&candidates).to_vec();
                if open.is_empty() {
                    break;
                }
                let n = rng.pick(&open);
                let h = model.home(n);
                let java_into = state.frames()[h - 1].promote_into(n, &math);
                let java_out = state.frames()[h].promote_out_of(n, &math);
                let expected = java_into.zip(java_out);
                assert_eq!(model.can_promote(&state, n), expected.is_some(), "can_promote({n})");
                match model.promote(&state, n) {
                    Some(next) => {
                        let (into, out) = expected.expect("the Java promotes too");
                        assert_eq!((next.frames()[h - 1], next.frames()[h]), (into, out), "promote({n})");
                        state = next;
                    }
                    None => {
                        assert!(expected.is_none(), "promote({n}) failed but the Java succeeds");
                        if rng.chance(30) {
                            break;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn fast_peeling_and_reduction_agree_with_reference() {
        let mut rng = Rng(0x5EED_1234_ABCD_0001);
        let mut stats = PeelStats::default();
        for n in golden_ns().into_iter().filter(|&n| n >= 20 && (n % 9 == 0 || n >= 480)) {
            let g = load(n);
            if n < 512 {
                random_agreement::<8>(&g, &mut rng, &mut stats);
            } else {
                random_agreement::<16>(&g, &mut rng, &mut stats);
            }
        }
        eprintln!("peel agreement: {stats:?}");
        assert!(stats.checks > 10_000 && stats.winnable > 1_000);
    }

    /// Promoting the same numbers in different orders reaches the same state whenever every step succeeds.
    fn confluence<const W: usize>(g: &Golden, rng: &mut Rng) -> (u64, u64) {
        let (model, root) = Model::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
        let candidates = Bits::<W>::from_slice(&g.candidates);
        let (mut orders, mut failed_orders) = (0, 0);
        for _ in 0..20 {
            // A feasible set, chosen at random in descending order.
            let mut state = root;
            let mut chosen = Vec::new();
            for n in candidates.iter_desc() {
                if rng.chance(40) && model.promote_in_place(&mut state, n) {
                    chosen.push(n);
                }
            }
            for _ in 0..10 {
                let mut order = chosen.clone();
                rng.shuffle(&mut order);
                let mut other = root;
                orders += 1;
                if order.iter().all(|&n| model.promote_in_place(&mut other, n)) {
                    assert_eq!(other, state, "n={}: order {order:?} reaches a different state", g.n);
                } else {
                    failed_orders += 1;
                }
            }
        }
        (orders, failed_orders)
    }

    #[test]
    fn promotion_order_does_not_matter() {
        let mut rng = Rng(0xC0FF_EE00_1234_5678);
        let (mut orders, mut failed) = (0, 0);
        for n in golden_ns().into_iter().filter(|&n| n >= 20 && (n % 7 == 0 || n >= 480)) {
            let g = load(n);
            let (o, f) = if n < 512 { confluence::<8>(&g, &mut rng) } else { confluence::<16>(&g, &mut rng) };
            orders += o;
            failed += f;
        }
        eprintln!("confluence: {orders} orders tried, {failed} failed part way");
        assert_eq!(failed, 0, "a feasible set failed in some other order");
    }

    /// Promoting a set one by one succeeds exactly when every frame is winnable with the whole set promoted.
    fn set_check<const W: usize>(g: &Golden, rng: &mut Rng) -> (u64, u64) {
        let (model, root) = Model::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
        let candidates = Bits::<W>::from_slice(&g.candidates);
        let (mut feasible, mut tried) = (0, 0);
        for _ in 0..200 {
            let density = 3 + rng.below(30) as u64;
            let set = rng.subset(&candidates, density);
            let mut state = root;
            let one_by_one = set.iter_desc().all(|n| model.promote_in_place(&mut state, n));
            let frames = root.frames();
            let at_once = set.iter().all(|n| model.home(n) >= 1 && model.contains_factor(&root, n))
                && frames.iter().enumerate().all(|(i, f)| {
                    let gained: Bits<W> = set.iter().filter(|&n| model.home(n) == i + 1).collect();
                    let lost: Bits<W> = set.iter().filter(|&n| model.home(n) == i).collect();
                    model.winnable(&(f.moves | gained), &(f.factors - lost))
                });
            assert_eq!(one_by_one, at_once, "n={}: {set:?}", g.n);
            tried += 1;
            feasible += u64::from(one_by_one);
        }
        (tried, feasible)
    }

    #[test]
    fn sets_can_be_checked_frame_by_frame() {
        let mut rng = Rng(0xFEED_FACE_0BAD_CAFE);
        let (mut tried, mut feasible) = (0, 0);
        for n in golden_ns().into_iter().filter(|&n| n >= 20 && (n % 5 == 0 || n >= 480)) {
            let g = load(n);
            let (t, f) = if n < 512 { set_check::<8>(&g, &mut rng) } else { set_check::<16>(&g, &mut rng) };
            tried += t;
            feasible += f;
        }
        eprintln!("frame-by-frame set check: {tried} sets, {feasible} feasible");
        assert!(feasible > tried / 10 && feasible < tried);
    }

    #[test]
    fn worklist_pops_in_descending_order() {
        let set = Bits::<3>::from_slice(&[0, 5, 63, 64, 100, 191]);
        let mut w = Worklist::of(&set);
        w.insert(7);
        let mut popped = Vec::new();
        while let Some(x) = w.pop() {
            popped.push(x);
        }
        assert_eq!(popped, vec![191, 100, 64, 63, 7, 5, 0]);
    }

    #[test]
    fn provider_lists_match_the_java() {
        let t = Tables::new(100);
        let p = FactorProvider::new(&t, &[2, 3, 6, 12, 18, 36, 50, 97]);
        assert_eq!(p.factors(36), &[12, 18]);
        assert_eq!(p.factors(6), &[2, 3]);
        assert_eq!(p.composites(6), &[12, 18]);
        assert_eq!(p.composites(2), &[6]);
        assert!(p.factors(97).is_empty(), "1 is not among the numbers");
        assert!(p.factors(1000).is_empty() && p.composites(1000).is_empty());
    }
}
