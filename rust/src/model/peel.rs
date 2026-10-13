//! Winnability of one frame: the peel, the core of an unwinnable frame, certificates, and reduction.
//!
//! A frame here is a set of moves and a set of factors (see the [module docs](super)); "live" factors of a move
//! are its maximal factors among the frame's factors ([`FactorProvider::factors`]).
//!
//! # The peel
//!
//! A move with a *private* live factor (one no other move has) can always be taken last: nothing taken before it
//! takes that factor.  So a frame is winnable exactly when repeatedly dropping a move with a private live factor
//! drops every move, in any order:
//!
//! * if every move drops, taking them in the reverse order of the drops wins, since each move's private factor
//!   (private among the moves still there when it dropped) is touched by none of the moves taken before it;
//! * if the frame is winnable, the peel never gets stuck: fewer moves stay winnable (the factors the dropped moves
//!   would have taken only help the others), and the last move of a winning order has a private factor (any
//!   other move with that factor would have taken it earlier).
//!
//! The Java peels with that rule plus "a move with a single live factor is taken first" and a pigeonhole test;
//! those preserve winnability too, so the answers agree (the model's tests check this against the Java's
//! procedure and, on small frames, against an exhaustive search of orders).  The dropping is done as in an
//! invertible Bloom lookup table: each factor keeps the count and the XOR of its live moves, so a factor with
//! count 1 names its move.  (`heuristics/THEORY.md`, Theorem 4, proves the same facts for the Python side's
//! `peel`, there called dismantling, and its residual, which is the core below.)  "Peel" means this procedure
//! throughout the Rust code; `hives::count_freedoms` and the Python `peel` are other procedures.
//!
//! # The core
//!
//! The moves a peel cannot drop are the frame's *core* ([`Scratch::stuck`]): empty exactly when the frame is
//! winnable.  Every live factor of a core move is shared with another core move, so no order takes all of them
//! (the last would find its factors taken), and the frame stays unwinnable however many moves are added, factors
//! removed, or factors added that no core move has.  Conflicts ([`Model::conflict_using`](super::Model::conflict_using))
//! are built from it.
//!
//! # Reduction and forced pairs
//!
//! A move left with a single live factor must be paid by it: the two form a *forced pair* (not to be confused
//! with the search's forced members, or with a hive's mandatory moves).  *Reducing* drops forced pairs until
//! none is left ([`reduce_from`]).  It never changes whether the frame is winnable, now or after further edits
//! (moves added, other factors removed): if `(m, q)` is a forced pair, any winning order takes `m` before every
//! other move that has `q` (whichever came first would take `q`, and `m` has no other factor), so dropping `m`
//! leaves a winning order of the frame without `m` and `q`; conversely, `m` taken first wins `q` and then
//! anything that wins the frame without them.  Two moves forced onto the same `q` make the frame unwinnable, so
//! on a winnable frame a forced pair stays forced until it is dropped, and reduction is confluent: every order
//! of drops ends at the same frame.

use super::FactorProvider;
use crate::bits::Bits;

/// A set of pending numbers that pops its largest member in constant time: the words of a bitset plus a summary
/// word with bit `i` set when word `i` is non-empty.  Needs `W <= 64`, that is boards up to 4095.
pub(super) struct Worklist<const W: usize> {
    words: [u64; W],
    summary: u64,
}

impl<const W: usize> Worklist<W> {
    const FITS: () = assert!(W <= 64, "a worklist summarizes at most 64 words");

    #[inline]
    pub(super) fn new() -> Self {
        let () = Self::FITS;
        Worklist { words: [0; W], summary: 0 }
    }

    #[inline]
    pub(super) fn of(set: &Bits<W>) -> Self {
        let mut out = Self::new();
        out.words = *set.as_words();
        for (i, &w) in out.words.iter().enumerate() {
            out.summary |= u64::from(w != 0) << i;
        }
        out
    }

    #[inline]
    pub(super) fn insert(&mut self, x: u32) {
        let i = (x >> 6) as usize;
        self.words[i] |= 1 << (x & 63);
        self.summary |= 1 << i;
    }

    /// Removes and returns the largest member.
    #[inline]
    pub(super) fn pop(&mut self) -> Option<u32> {
        if self.summary == 0 {
            return None;
        }
        let i = 63 - self.summary.leading_zeros() as usize;
        let bit = 63 - self.words[i].leading_zeros();
        self.take(i, bit);
        Some(((i as u32) << 6) | bit)
    }

    /// Removes and returns the smallest member.
    #[inline]
    fn pop_min(&mut self) -> Option<u32> {
        if self.summary == 0 {
            return None;
        }
        let i = self.summary.trailing_zeros() as usize;
        let bit = self.words[i].trailing_zeros();
        self.take(i, bit);
        Some(((i as u32) << 6) | bit)
    }

    #[inline]
    fn take(&mut self, i: usize, bit: u32) {
        let rest = self.words[i] & !(1u64 << bit);
        self.words[i] = rest;
        if rest == 0 {
            self.summary &= !(1u64 << i);
        }
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
/// keeps one (one per thread) and passes it to the `*_using` methods of [`Model`](super::Model); every check
/// leaves it ready for the next.
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

    /// Whether the frame `moves -> factors` is winnable: the peel, which stops as soon as every move has dropped
    /// (winnable) or none can (see the module docs).
    #[inline]
    pub(super) fn winnable(&mut self, moves: &Bits<W>, factors: &Bits<W>, g: &FactorProvider) -> bool {
        let mut stack = Stack { items: self.stack.as_flattened_mut(), top: 0 };
        peel(self.cells.as_flattened_mut(), moves, factors, g, &mut NoRecord, &mut stack)
    }

    /// The moves a peel of the frame `moves -> factors` cannot drop, its *core*: empty exactly when the frame is
    /// winnable.  Every live factor of a core move is a factor of another core move (else the peel could drop
    /// it), so no order takes all the core moves: the last of them would need a factor that an earlier one has
    /// already taken.  So the core keeps the frame unwinnable with other moves added and with factors added that
    /// no core move has.  Unlike [`winnable`](Self::winnable), this always peels to the end.
    pub(super) fn stuck(&mut self, moves: &Bits<W>, factors: &Bits<W>, g: &FactorProvider) -> Bits<W> {
        let cells = self.cells.as_flattened_mut();
        let mut pending = Stack { items: self.stack.as_flattened_mut(), top: 0 };
        let mut left = *moves;
        for m in moves.iter() {
            for &q in g.factors(m) {
                if factors.contains(u32::from(q)) {
                    let cell = &mut cells[usize::from(q)];
                    *cell = (*cell + ONE_MOVE) ^ m;
                    if *cell >> 16 == 1 {
                        pending.push(q);
                    }
                }
            }
        }
        while let Some(f) = pending.pop() {
            let cell = cells[usize::from(f)];
            if cell >> 16 != 1 {
                continue;
            }
            let m = cell & 0xFFFF;
            left.remove(m);
            for &q in g.factors(m) {
                let cell = &mut cells[usize::from(q)];
                if *cell != 0 {
                    *cell = (*cell - ONE_MOVE) ^ m;
                    if *cell >> 16 == 1 {
                        pending.push(q);
                    }
                }
            }
        }
        clear(cells, factors);
        left
    }

    /// [`winnable`](Self::winnable), recording the winning order in `cert`.  The order frees the smallest
    /// factor it can at each step, so the large factors that promoted numbers have tend to be freed late, which
    /// keeps the peels a certificate cannot avoid short (see [`Certificate`]).
    pub(super) fn certify(
        &mut self,
        moves: &Bits<W>,
        factors: &Bits<W>,
        g: &FactorProvider,
        cert: &mut Certificate<W>,
    ) -> bool {
        peel(self.cells.as_flattened_mut(), moves, factors, g, cert, &mut Worklist::<W>::new())
    }
}

/// Factors waiting for a peel to look at them again.
trait Pending {
    fn push(&mut self, factor: u16);
    fn pop(&mut self) -> Option<u16>;
}

/// Last in, first out: the fastest.
struct Stack<'a> {
    items: &'a mut [u16],
    top: usize,
}

impl Pending for Stack<'_> {
    #[inline]
    fn push(&mut self, factor: u16) {
        self.items[self.top] = factor;
        self.top += 1;
    }

    #[inline]
    fn pop(&mut self) -> Option<u16> {
        self.top = self.top.checked_sub(1)?;
        Some(self.items[self.top])
    }
}

/// Smallest factor first.
impl<const W: usize> Pending for Worklist<W> {
    #[inline]
    fn push(&mut self, factor: u16) {
        self.insert(u32::from(factor));
    }

    #[inline]
    fn pop(&mut self) -> Option<u16> {
        self.pop_min().map(|f| f as u16)
    }
}

/// The peel behind [`Scratch::winnable`] and [`Scratch::certify`], on a [`Scratch`]'s cells (all zero before
/// and after).
fn peel<const W: usize, R: Recorder<W>, P: Pending>(
    cells: &mut [u32],
    moves: &Bits<W>,
    factors: &Bits<W>,
    g: &FactorProvider,
    record: &mut R,
    pending: &mut P,
) -> bool {
    let moves_total = moves.len();
    let mut left = moves_total;
    if left > factors.len() {
        return false;
    }
    record.begin(factors, moves_total);
    if left == 0 {
        return true;
    }
    // Count every move into the cells of its live factors, queueing each factor whose count reaches 1.
    for m in moves.iter() {
        let mut live = false;
        for &q in g.factors(m) {
            if factors.contains(u32::from(q)) {
                live = true;
                let cell = &mut cells[usize::from(q)];
                *cell = (*cell + ONE_MOVE) ^ m;
                if *cell >> 16 == 1 {
                    pending.push(q);
                }
            }
        }
        if !live {
            clear(cells, factors);
            return false;
        }
    }
    // A factor with a single live move names it; that move can be taken last, so drop it.  While a move is
    // counted, the cells of its live factors are non-zero and those of its other factors zero.
    while let Some(f) = pending.pop() {
        let cell = cells[usize::from(f)];
        if cell >> 16 != 1 {
            continue;
        }
        let m = cell & 0xFFFF;
        let step = moves_total - left;
        record.dropped(step, m);
        for &q in g.factors(m) {
            let cell = &mut cells[usize::from(q)];
            if *cell != 0 {
                *cell = (*cell - ONE_MOVE) ^ m;
                if *cell >> 16 == 1 {
                    pending.push(q);
                } else if *cell == 0 {
                    record.freed(q, step);
                }
            }
        }
        left -= 1;
        if left == 0 {
            return true; // every move was counted in and out, so every cell is zero again
        }
    }
    clear(cells, factors);
    false
}

fn clear<const W: usize>(cells: &mut [u32], factors: &Bits<W>) {
    for q in factors.iter() {
        cells[q as usize] = 0;
    }
}

/// What a peel reports as it goes: the move it drops at each step and the factors that lose their last live move
/// then (one move per step, from step 0).
trait Recorder<const W: usize> {
    /// The peel of a frame with these factors and this many moves begins.
    fn begin(&mut self, factors: &Bits<W>, moves: u32);
    fn dropped(&mut self, step: u32, m: u32);
    fn freed(&mut self, factor: u16, step: u32);
}

/// The recorder of a plain winnability check.
struct NoRecord;

impl<const W: usize> Recorder<W> for NoRecord {
    #[inline(always)]
    fn begin(&mut self, _: &Bits<W>, _: u32) {}

    #[inline(always)]
    fn dropped(&mut self, _: u32, _: u32) {}

    #[inline(always)]
    fn freed(&mut self, _: u16, _: u32) {}
}

/// One winning order of a winnable frame, kept to answer "is the frame still winnable with one more move, or
/// with one factor less?" mostly without peeling (see [`Model::certify_using`](super::Model::certify_using)).
/// Used by the Java's promotability checks of children's candidates (`bounds`, off the default path).
///
/// The order drops one move per step, each with a factor no remaining move but it has.  Record, for each
/// factor, the step at which its last live move is dropped (−1 if it has none): the factors freed at a step
/// are exactly the ones that let its move go.  Replaying the order:
///
/// * with a new move `x` whose live factors are `Q`: nothing changes until step `i0 = min last(Q)`; if the
///   move of that step has a freed factor outside `Q` it still goes, and then `x` has a factor of its own and
///   can go too, after which the order finishes as before.  So the frame stays winnable if `i0 < 0` or step
///   `i0` frees a factor outside `Q`.
/// * without a factor `y`: only the step that frees `y` is affected, and its move still goes if that step
///   frees another factor.  So the frame stays winnable if `y` has no live move or its step frees two factors.
///
/// When neither applies, a peel decides, but only of the moves from the affected step on: the steps before it
/// are still valid, and peeling can always be continued from any valid partial order.
#[derive(Clone)]
pub struct Certificate<const W: usize> {
    /// Per factor, the step at which it is freed (−1 if it has no live move).
    last: [[i16; 64]; W],
    /// Per step, how many factors it frees.
    freed: [[u8; 64]; W],
    /// Per step, the move it drops.
    order: [[u16; 64]; W],
}

impl<const W: usize> Default for Certificate<W> {
    fn default() -> Self {
        Certificate { last: [[0; 64]; W], freed: [[0; 64]; W], order: [[0; 64]; W] }
    }
}

impl<const W: usize> Certificate<W> {
    /// The step at which `factor` is freed (−1 if it has no live move).
    #[inline]
    pub(super) fn last(&self, factor: u32) -> i16 {
        self.last.as_flattened()[factor as usize]
    }

    /// How many factors `step` frees.
    #[inline]
    pub(super) fn freed_at(&self, step: i16) -> u8 {
        self.freed.as_flattened()[step as usize]
    }

    /// `moves` without the moves the order drops before `step`.
    #[inline]
    pub(super) fn moves_from_step(&self, moves: &Bits<W>, step: i16) -> Bits<W> {
        let mut rest = *moves;
        for &m in &self.order.as_flattened()[..step.max(0) as usize] {
            rest.remove(u32::from(m));
        }
        rest
    }
}

impl<const W: usize> Recorder<W> for Certificate<W> {
    fn begin(&mut self, factors: &Bits<W>, moves: u32) {
        let last = self.last.as_flattened_mut();
        for f in factors.iter() {
            last[f as usize] = -1;
        }
        self.freed.as_flattened_mut()[..moves as usize].fill(0);
    }

    #[inline]
    fn dropped(&mut self, step: u32, m: u32) {
        self.order.as_flattened_mut()[step as usize] = m as u16;
    }

    #[inline]
    fn freed(&mut self, factor: u16, step: u32) {
        self.last.as_flattened_mut()[usize::from(factor)] = step as i16;
        self.freed.as_flattened_mut()[step as usize] += 1;
    }
}

/// Whether the frame `moves -> factors` is winnable, with a scratch space of its own.
#[inline]
pub(super) fn winnable<const W: usize>(moves: &Bits<W>, factors: &Bits<W>, g: &FactorProvider) -> bool {
    Scratch::new().winnable(moves, factors, g)
}

/// Reduction: drops each move with a single live factor together with that factor (a forced pair), until none
/// is left, examining `pending` and whatever the drops affect.  A move with no live factor is dropped alone, as
/// the Java does with assertions off (it cannot happen on a winnable frame).
pub(super) fn reduce_from<const W: usize>(
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
