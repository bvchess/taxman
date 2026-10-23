//! Winnability of one frame: the peel, the residual of an unwinnable frame, and reduction.
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
//! count 1 names its move.  (`docs/THEORY.md`, Theorem 4, proves the same facts for the Python side's
//! `peel`, there called dismantling; its residual is the residual below.)  "Peel" means this procedure
//! throughout the Rust code; `BoardFrames::count_freedoms` (setup's count of a frame's freedoms) and the Python `peel`
//! are other procedures.
//!
//! # The residual
//!
//! The moves a peel cannot drop are the frame's *residual* ([`Scratch::stuck`]): empty exactly when the frame is
//! winnable.  Every live factor of a residual move is shared with another residual move, so no order takes all of
//! them (the last would find its factors taken), and the frame stays unwinnable however many moves are added,
//! factors removed, or factors added that no residual move has.  Conflicts are built from it
//! ([`FrameGraph::conflict_using`](super::FrameGraph::conflict_using)).
//!
//! # Reduction
//!
//! A move left with a single live factor must be paid by it: the move and that factor form a pair, which is
//! dropped (not to be confused with the search's forced members, or with the mandatory moves that setup settles
//! with `unzip`).  *Reducing* drops such pairs until none is left ([`reduce_from`]).  It never changes whether the
//! frame is winnable, now or after further edits (moves added, other factors removed): if `m` is left with the
//! single live factor `q`, any winning order takes `m` before every other move that has `q` (whichever came first
//! would take `q`, and `m` has no other factor), so dropping `m` leaves a winning order of the frame without `m`
//! and `q`; conversely, `m` taken first wins `q` and then anything that wins the frame without them.  Two moves
//! left with only `q` make the frame unwinnable, so on a winnable frame such a pair stays a pair until it is
//! dropped, and reduction is confluent: every order of drops ends at the same frame.

use super::FactorProvider;
use crate::bits::Bits;

/// A set of pending numbers that pops its largest member in constant time: the words of a bitset plus summary words,
/// bit `i % 64` of summary word `i / 64` set when word `i` is non-empty.  Two summary words, so `W <= 128`, boards up
/// to 8191; at `W <= 64` only the first is ever used, which the compiler knows (`TOP`).
pub(super) struct Worklist<const W: usize> {
    words: [u64; W],
    summary: [u64; 2],
}

impl<const W: usize> Worklist<W> {
    const FITS: () = assert!(W <= 128, "a worklist summarizes at most 128 words");
    /// The last summary word in use.
    const TOP: usize = (W - 1) / 64;

    #[inline]
    pub(super) fn new() -> Self {
        let () = Self::FITS;
        Worklist { words: [0; W], summary: [0; 2] }
    }

    #[inline]
    pub(super) fn of(set: &Bits<W>) -> Self {
        let mut out = Self::new();
        out.words = *set.as_words();
        for (i, &w) in out.words.iter().enumerate() {
            out.summary[i >> 6] |= u64::from(w != 0) << (i & 63);
        }
        out
    }

    #[inline]
    pub(super) fn insert(&mut self, x: u32) {
        let i = (x >> 6) as usize;
        self.words[i] |= 1 << (x & 63);
        self.summary[i >> 6] |= 1 << (i & 63);
    }

    /// Removes and returns the largest member.
    #[inline]
    pub(super) fn pop(&mut self) -> Option<u32> {
        let mut s = Self::TOP;
        while self.summary[s] == 0 {
            if s == 0 {
                return None;
            }
            s -= 1;
        }
        let i = 64 * s + 63 - self.summary[s].leading_zeros() as usize;
        let bit = 63 - self.words[i].leading_zeros();
        self.take(i, bit);
        Some(((i as u32) << 6) | bit)
    }

    #[inline]
    fn take(&mut self, i: usize, bit: u32) {
        let rest = self.words[i] & !(1u64 << bit);
        self.words[i] = rest;
        if rest == 0 {
            self.summary[i >> 6] &= !(1u64 << (i & 63));
        }
    }
}

/// How many members of `list` are in `live`, counting no further than 2, and the last one found.
#[inline]
fn live_neighbors<const W: usize>(list: &[u16], live: &Bits<W>) -> (u32, u32) {
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
/// keeps one (one per thread) and passes it to the `*_using` methods of [`FrameGraph`](super::FrameGraph); every check
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
        peel(self.cells.as_flattened_mut(), moves, factors, g, &mut stack)
    }

    /// The moves a peel of the frame `moves -> factors` cannot drop, its *residual*: empty exactly when the frame
    /// is winnable.  Every live factor of a residual move is a factor of another residual move (else the peel could
    /// drop it), so no order takes all the residual moves: the last of them would need a factor that an earlier
    /// one has already taken.  So the residual keeps the frame unwinnable with other moves added and with factors
    /// added that no residual move has.  Unlike [`winnable`](Self::winnable), this always peels to the end.
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
}

/// Factors waiting for a peel to look at them again, last in, first out.
struct Stack<'a> {
    items: &'a mut [u16],
    top: usize,
}

impl Stack<'_> {
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

/// The peel behind [`Scratch::winnable`], on a [`Scratch`]'s cells (all zero before and after).
fn peel<const W: usize>(
    cells: &mut [u32],
    moves: &Bits<W>,
    factors: &Bits<W>,
    g: &FactorProvider,
    pending: &mut Stack<'_>,
) -> bool {
    let mut left = moves.len();
    if left > factors.len() {
        return false;
    }
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
        for &q in g.factors(m) {
            let cell = &mut cells[usize::from(q)];
            if *cell != 0 {
                *cell = (*cell - ONE_MOVE) ^ m;
                if *cell >> 16 == 1 {
                    pending.push(q);
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

/// Whether the frame `moves -> factors` is winnable, with a scratch space of its own.
#[inline]
pub(super) fn winnable<const W: usize>(moves: &Bits<W>, factors: &Bits<W>, g: &FactorProvider) -> bool {
    Scratch::new().winnable(moves, factors, g)
}

/// Reduction: drops each move with a single live factor together with that factor, until none is left,
/// examining `pending` and whatever the drops affect.  A move with no live factor is dropped alone, as the Java
/// does with assertions off (it cannot happen on a winnable frame).
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
        let (count, q) = live_neighbors(g.factors(m), factors);
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
