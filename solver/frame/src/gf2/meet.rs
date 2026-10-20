//! Weighted matroid intersection for two linear matroids over GF(2) that are direct sums of blocks of rank at most
//! `64·R`: the solver behind [`super::Gf2Bound`].  The matroids, and why every promotable set is independent in
//! both, are in the parent module's docs; this module knows nothing of frames or promotions.
//!
//! # The algorithm
//!
//! Shortest augmenting paths (Lawler; Schrijver, *Combinatorial Optimization*, Theorem 41.7).  A common independent
//! set `I` is *extreme* when it is the heaviest of its size.  Its *exchange graph* has an arc `x → y` (`x ∈ I`,
//! `y ∉ I`) when `I − x + y` is independent in `A`, and `y → x` when it is independent in `B`; the vertex lengths are
//! `w(x)` on `I` and `−w(y)` off it.  A path from an element `y` with `I + y` independent in `A` to one with `I + y`
//! independent in `B` that is shortest, and has the fewest arcs among the shortest, gives `I` flipped along it: the
//! heaviest common independent set of size `|I| + 1`.  The heaviest weights by size are concave, so the heaviest of
//! all sizes is reached when the shortest path's length is no longer negative.
//!
//! In a linear matroid `I − x + y` is independent when `I + y` is, or when `x` is in the combination of `I`'s
//! vectors that makes `y`'s (its *circuit*).  The span of `I` in each block is kept as rows by pivot (the row's
//! lowest set bit), each with the mask of the slots (members of `I` in the block) whose vectors sum to it, so one
//! reduction of `y`'s vector gives either a non-zero residue (`I + y` independent) or its circuit.  An element that
//! is independent of `I` in a matroid has arcs to or from every member of `I`; those arcs are relaxed through one
//! minimum per Bellman–Ford round instead of one at a time.
//!
//! # Where an augmentation may start
//!
//! The augmentations may start from any set that is extreme and heavier than every smaller common independent set
//! (*dominant*): from there each one gives the heaviest set one larger, and the gains decrease, so the first gain
//! that is not positive marks the heaviest set of all sizes.  [`Meet::solve`] starts from the larger of a given
//! dominant set and the heaviest *uncontended* elements: the longest run of the heaviest elements that is common
//! independent, which is the heaviest set of its size, and dominant because weights are positive.  Every set an
//! augmentation reaches is dominant (the gains so far were positive), and [`Meet::solve`] reports each, so that a
//! caller can start a related problem from one of them (see [`super`], "Warm starts").
//!
//! # The weight split
//!
//! For the heaviest set `I*` of all sizes (if no size limit stopped the augmentations), [`Meet::split_bounds`] finds
//! `w1`, `w2` with `w1 + w2 = w` such that `I*` is a heaviest independent set of `A` for `w1` and of `B` for `w2`
//! (Frank's weight-splitting theorem; any sizes).  For any common independent `J`, `w(J) = w1(J) + w2(J)`, so the
//! heaviest independent set of `A` for `w1` plus that of `B` for `w2` bounds every common independent set (weak
//! duality), and with an element left out each of those is one exchange away from `I*`: `I* − x` plus the heaviest
//! element that can replace `x` (whose circuit holds `x`), if its weight is positive.  So the bound without each
//! member of `I*` takes one pass over the circuits.  The split comes from the conditions under which `I*` is a
//! heaviest independent set of a matroid for a weighting (members of non-negative weight; elements whose addition
//! keeps it independent of non-positive weight; no exchange that gains), which for `w1` on `A` and `w2 = w − w1` on
//! `B` are difference constraints on `w1`, solved by Bellman–Ford.  The split is the shortest distances from a
//! virtual source, which Bellman–Ford reaches from any potentials that are lengths of paths, so a split may start from
//! an earlier one's potentials wherever the path of predecessors that gave each is still a path
//! ([`Meet::split_bounds_from`]): a child's problem is its parent's with a few vertices gone, a node's after it goes
//! on without a member its own with a few circuits changed.
//!
//! # The heaviest set without a member
//!
//! Strong branching asks, for each member `x` of a conflict of `I*` (the heaviest set of all sizes), for the heaviest
//! common independent set without `x`; forced members are those without which it is below what the node needs.
//! [`Meet::without`] answers from `I*`'s exchange graph alone, with no augmentation: the shortest cycle through `x`
//! (Dijkstra's search over the weight split's reduced lengths, below), along which `I*` flips to a set without `x`
//! of the same size, checked common independent: its weight `w(I*) − ℓ`, for a cycle of length `ℓ`, is a lower
//! bound.  It is the heaviest set without `x` when the sets of other sizes than `I*`'s lose at least `ℓ` against
//! `I*`:
//!
//! * Let `J` be any common independent set without `x`.  In each matroid the symmetric difference of two
//!   independent sets of the same size has a perfect matching of exchange arcs (Schrijver, *Combinatorial
//!   Optimization*, Corollary 39.12a; when the sizes differ, the smaller set extended from the larger first), so
//!   `I* Δ J` splits into vertex-disjoint parts of the exchange graph: cycles only when `|J| = |I*|`, cycles and
//!   paths from member to member (a member alone included) when `J` is smaller, and cycles and paths from a source
//!   (an element whose addition keeps `A` independent) to a sink (the same for `B`) when it is larger.  The lengths
//!   of the parts add up to `w(I*) − w(J)`.
//! * The weight split makes the reduced length of every arc non-negative, `w1(z) − w1(y)` for an arc `z → y` of
//!   `A` and `w2(z) − w2(y)` for `y → z` of `B`; a cycle's length is the sum of its arcs' reduced lengths, a path
//!   between members' that plus `w2` of its first member and `w1` of its last, a path from a source to a sink's that
//!   less `w1` of the source and `w2` of the sink: none is negative.
//! * So if `|J| = |I*|`, `w(J) ≤ w(I*) − ℓ'`, `ℓ'` the length of the cycle through `x`, at least `ℓ`.  A smaller `J`
//!   loses at least the shortest path between members ([`Meet::smaller_loss`]; exactly what `I*` gains over a dominant
//!   set one smaller, where one is known), a larger one at least the gap to the heaviest set one larger
//!   ([`Meet::gap`], the solve's last shortest path).  If both are at least `ℓ`, no set without `x` beats the
//!   flipped one.
//!
//! The same argument restricted to the sets that hold some elements and avoid others certifies a child's heaviest
//! set from its parent's exchange graph (a cycle that avoids them), and with no cycle within `value − need` and the
//! other sizes losing more, shows a member forced ([`Meet::has_cycle_within`]).  In the proof of 1215, every bound
//! without a member that strong branching asked for (10,183) was such a cycle's, as the solves found; where the
//! certificate does not hold the caller solves (see
//! `solver/notebook/search/the-inner-loop-of-the-gf2-search-2026-10/README.md`).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// A block vector: bit `i` of word `i / 64` is coordinate `i` of the block's quotient space.
pub type Vector<const R: usize> = [u64; R];

#[inline]
pub(super) fn is_zero<const R: usize>(v: &Vector<R>) -> bool {
    v.iter().all(|&w| w == 0)
}

/// The lowest set bit of `v`.
#[inline]
pub(super) fn lowest<const R: usize>(v: &Vector<R>) -> Option<usize> {
    for (i, &w) in v.iter().enumerate() {
        if w != 0 {
            return Some(64 * i + w.trailing_zeros() as usize);
        }
    }
    None
}

#[inline]
pub(super) fn xor<const R: usize>(a: &mut Vector<R>, b: &Vector<R>) {
    for (x, y) in a.iter_mut().zip(b) {
        *x ^= y;
    }
}

#[inline]
pub(super) fn bit<const R: usize>(v: &Vector<R>, i: usize) -> bool {
    v[i / 64] >> (i % 64) & 1 == 1
}

/// Calls `f` with each set bit of `mask`.
#[inline]
fn each_bit<const R: usize>(mask: &Vector<R>, mut f: impl FnMut(usize)) {
    for (i, &w) in mask.iter().enumerate() {
        let mut w = w;
        while w != 0 {
            f(64 * i + w.trailing_zeros() as usize);
            w &= w - 1;
        }
    }
}

/// The two matroids: for each element, its block in `A` and in `B` and its vectors there.  An element whose vector
/// is zero in either is a loop: no common independent set holds it.
pub struct Problem<'a, const R: usize> {
    pub block: [&'a [u8]; 2],
    pub vector: [&'a [Vector<R>]; 2],
    /// The weight of each element: positive, and not decreasing with the element's index (the heaviest elements come
    /// last).  In the search an element's weight is its number and the elements are indexed by increasing number.
    pub weight: &'a [i64],
}

/// The span of the members of a set in one block of one matroid: the rows by pivot, each with the mask of the
/// slots whose vectors make it, and the element in each slot.
#[derive(Clone)]
struct BlockSpan<const R: usize> {
    row: Vec<(Vector<R>, Vector<R>)>,
    has: Vector<R>,
    elem: Vec<u32>,
}

impl<const R: usize> BlockSpan<R> {
    fn new() -> Self {
        BlockSpan { row: vec![([0; R], [0; R]); 64 * R], has: [0; R], elem: Vec::with_capacity(64 * R) }
    }

    #[inline]
    fn clear(&mut self) {
        self.has = [0; R];
        self.elem.clear();
    }

    /// `v` reduced by the rows: `(residue, slot mask)`.  The residue is zero when `v` is in the span (and the mask
    /// makes it); otherwise its lowest set bit is not a pivot.
    #[inline]
    fn reduce(&self, mut v: Vector<R>) -> (Vector<R>, Vector<R>) {
        let mut c = [0; R];
        while let Some(p) = lowest(&v) {
            if !bit(&self.has, p) {
                break;
            }
            // A row has no bits below its pivot.
            let (rv, rc) = &self.row[p];
            xor(&mut v, rv);
            xor(&mut c, rc);
        }
        (v, c)
    }

    /// Adds the element `e` with the vector `v`; false (and nothing added) if `v` is in the span.
    #[inline]
    fn insert(&mut self, v: Vector<R>, e: u32) -> bool {
        let (r, mut c) = self.reduce(v);
        let Some(p) = lowest(&r) else { return false };
        let slot = self.elem.len();
        self.elem.push(e);
        c[slot / 64] |= 1 << (slot % 64);
        self.row[p] = (r, c);
        self.has[p / 64] |= 1 << (p % 64);
        true
    }
}

/// A distance in the exchange graph, packed: the length times [`HOPS`] plus the number of arcs, so that one
/// comparison orders by length, then by arcs.  Lengths stay far below `i64::MAX / HOPS` (a path's length is at most
/// the sum of the weights) and paths have fewer than `HOPS` arcs (at most twice the elements).
type Dist = i64;
const HOPS: i64 = 1 << 16;
const INF: Dist = i64::MAX;
const NONE: u32 = u32::MAX;
const NO_SLOT: u16 = u16::MAX;
/// The heap entry of [`Meet::without`] that stands for the next element free in A.
const FREE_A: u32 = u32::MAX - 1;

/// The workspace of the intersection: everything a solve needs, kept between solves so that a solve allocates
/// nothing once the vectors have grown.
///
/// # The tableau
///
/// For the current set `I`, each matroid keeps, per element `e` of a block, its coordinates on a basis of the block's
/// space made of `I`'s members there (one *slot* each) and the coordinates no member has eliminated: `mask[e]` (the
/// slots) and `res[e]` (the rest).  `res[e] = 0` exactly when `e` is in the span of `I` (then `mask[e]` is its circuit
/// less `e`), and otherwise `I + e` is independent there.  A member's mask is its own slot.  An augmentation changes
/// `I` by one addition and a few exchanges in each matroid, and each is a pivot over the block's live elements
/// ([`Meet::add`], [`Meet::exchange`]); when a path's changes cannot be applied as pivots in some order (an exchange
/// across blocks, or with an element independent of `I`), the blocks they touch are rebuilt from their members.
///
/// # Deriving a tableau
///
/// Outside a solve only part of the tableau is ever read: which elements are free (`res[e] ≠ 0`), the circuits of the
/// others (`mask[e]`), the member in each slot, and the lists of members and of the elements outside the set.  Of
/// these the free elements and the circuits depend on the set alone, not on how the tableau was made, and a rebuild
/// numbers each block's slots by increasing member.  So a tableau equal to the one a solve would build for a set
/// follows from the tableau of a related set, at a cost linear in the elements instead of a pivot per member
/// ([`derive`](Self::derive)):
///
/// * Exchanging a member `x` for an element `y` in the span whose circuit holds `x` keeps the span, so it changes
///   no residue and is the pivot of [`exchange`](Self::exchange).  Two independent sets of a block with the same
///   span are a pivot apart for every element of one that is not in the other (the circuit of such a `y` must hold
///   a member that is not in the other set, which would be dependent otherwise), so the pivots reach the other set.
/// * Where the span of a block changes (an element that enters is free, or the numbers that enter and leave
///   differ), an element whose circuit holds no member that leaves keeps it: the circuit lies in the new set.  The
///   others outside the new set, free ones included, are reduced by its members, which costs a reduction each
///   instead of a pivot per member over the whole block.  (The residues of a derived tableau are read only for
///   being zero, so a reduction's will do.)
/// * Contracting by members `C` of the set `S` (promoting them): an element is in the span of `S − C` in the
///   contraction exactly when it is in the span of `S`, and its circuit there is its circuit with respect to `S`
///   less `C` (the vectors reduced modulo those of `C`).  So the circuits carry over without the slots of `C`.
/// * Deleting elements that are not members changes no other element's circuit.
/// * Numbering the slots by increasing member renames the bits of the circuits.
///
/// The search derives a child's tableau from its parent's (inherited, the members it promotes contracted; or
/// certified, flipped along a cycle and contracted), and a node's without the member it branched on from its own,
/// flipped along the certified cycle, from a copy ([`Snapshot`]) kept while the child's subtree uses the workspace.
pub struct Meet<const R: usize> {
    mask: [Vec<Vector<R>>; 2],
    res: [Vec<Vector<R>>; 2],
    /// The slot of each member of the tableau's set, [`NO_SLOT`] for the others.
    slot: [Vec<u16>; 2],
    /// Per block: the member in each slot, and the slots in use.
    slot_elem: [Vec<Vec<u32>>; 2],
    next_slot: [Vec<usize>; 2],
    /// Per block: the live elements of the solve.
    block_live: [Vec<Vec<u32>>; 2],
    /// Per matroid, per block: spans for the heaviest uncontended elements and the split's replacements.
    spans: [Vec<BlockSpan<R>>; 2],
    /// Element `e` is in the current set.
    inside: Vec<bool>,
    /// The members of the current set, and the elements outside it by their block of `A` (in no order), each
    /// element's index in its list in `pos`.
    members: Vec<u32>,
    out_a: Vec<Vec<u32>>,
    pos: Vec<u32>,
    dist: Vec<Dist>,
    pred: Vec<u32>,
    /// The last augmenting path, from its source to its sink.
    path: Vec<u32>,
    /// Bellman–Ford's worklist: the outside elements and the members whose distance changed in the last phase,
    /// the changed members' slots per block of each matroid, and the phase each vertex was last listed in (a
    /// stamp, also of the other searches, `generation` the last one given; see
    /// [`next_generation`](Self::next_generation)).
    changed_out: Vec<u32>,
    changed_in: Vec<u32>,
    changed_slots: [Vec<Vector<R>>; 2],
    seen: Vec<u32>,
    generation: u32,
    /// The blocks a path's changes left to rebuild.
    dirty: [Vec<bool>; 2],
    /// Whether the last solve ended because no augmentation gains (rather than at the size limit): then the set is
    /// the heaviest of all sizes and the tableau describes it, which the split needs.
    pub settled: bool,
    /// Whether the last solve stopped at its `stop` weight, before the heaviest set.
    pub stopped: bool,
    /// After a solve that settled: how much lighter the heaviest common independent set one larger than the
    /// heaviest set is (the last shortest path's length), `i64::MAX` if there is none ([`gap`](Self::gap)), for the
    /// tableau `gap_for` (a derived tableau's is found when first asked for).  The heaviest sets of all larger sizes
    /// are lighter still (the weights by size are concave).
    gap: i64,
    gap_for: u64,
    /// Solves made: a caller can tell whether the workspace still holds the tableau of a solve it made.
    pub solves: u64,
    /// The solve whose tableau the workspace holds (a greedy set counts as a solve and spoils it).
    tableau: u64,
    /// The heaviest set found and its weight.
    pub value: i64,
    /// Counters: augmentations made, Bellman–Ford rounds run, blocks rebuilt after a path.
    pub augmentations: u64,
    pub rounds: u64,
    pub rebuilds: u64,
    /// The weight splits' Bellman–Ford rounds.
    pub split_rounds: u64,
    /// The split's scratch: its potentials (one per element and one more), the predecessor that gave each its
    /// potential (`NONE` for 0, the virtual source's), the replacements' weights, a walk's vertices, and in debug
    /// builds the potentials found from all zero to check those found from a start.
    pot: Vec<i64>,
    split_pred: Vec<u32>,
    repl: [Vec<i64>; 2],
    walk: Vec<u32>,
    #[cfg(debug_assertions)]
    cold: Vec<i64>,
    /// [`without`](Self::without)'s scratch: its search's distances, predecessors (towards the member searched
    /// from), settled stamps, when each was offered its distance, and heap; the elements outside the set in the span
    /// of A by the slots of their circuits (`columns`, per block of A 64·R slots) and those free in A, both built for
    /// the tableau `columns_for` (and shared with the split), the latter by decreasing `w1`, and the members by
    /// increasing `w2`, for the tableau `sorted_for` and its split; and the elements of the cycle found.
    reach: Vec<Dist>,
    back: Vec<u32>,
    settled_at: Vec<u32>,
    offered_at: Vec<u32>,
    heap: Queue,
    columns: Vec<Vec<u32>>,
    columns_for: u64,
    free_a: Vec<u32>,
    by_w2: Vec<u32>,
    sorted_for: u64,
    flip: Vec<u32>,
    /// The element that closed the cycle the last [`has_cycle_within`](Self::has_cycle_within) found (`NONE` for
    /// none).
    found: u32,
    /// Counters of [`without`](Self::without): calls, and those it certified.
    pub withouts: u64,
    pub withouts_exact: u64,
    /// [`derive`](Self::derive)'s scratch: marks by element, the new slot of each old one, the elements that enter
    /// and leave a block, the live elements the tableau derived from did not have, and per matroid and block those
    /// whose circuits it does not give where the span changes.
    mark: Vec<u8>,
    slot_map: Vec<u16>,
    entering: Vec<u32>,
    leaving: Vec<u32>,
    fresh: Vec<u32>,
    unsure: [Vec<Vec<u32>>; 2],
    /// Counters: tableaux derived (of which from a snapshot), blocks built in them, and blocks where elements were
    /// reduced.
    pub derived: u64,
    pub restored: u64,
    pub built: u64,
    pub reduced: u64,
    /// In tests: a workspace that solves every derived tableau's problem again, to check the derived one against it,
    /// and the tableaux it checked.
    #[cfg(test)]
    pub check: Option<Box<Meet<R>>>,
    #[cfg(test)]
    pub checked: u64,
}

/// The queue of the searches of the exchange graph, which take the entries `(distance, vertex)` least first (the
/// vertex [`FREE_A`] after every element at the same distance).  When the distances stay below a bound under 2^49 and
/// the elements fit 14 bits, the entries are keys of a [`RadixHeap`], the distance above the vertex (`FREE_A` the
/// largest vertex): a search pushes no entry below the last it took (every arc adds an arc to the packed distance,
/// and the hubs' entries are later still), so the keys are monotone and come out in the same order as from a binary
/// heap, which takes the other searches.
struct Queue {
    radix: RadixHeap,
    heap: BinaryHeap<Reverse<(Dist, u32)>>,
    packed: bool,
}

impl Queue {
    const BITS: u32 = 14;
    const FREE: u64 = (1 << Self::BITS) - 1;

    fn new(n: usize) -> Self {
        Queue { radix: RadixHeap::new(), heap: BinaryHeap::with_capacity(n), packed: false }
    }

    /// Empties the queue for a search whose distances stay below `bound`, of elements numbered below `n`.
    fn start(&mut self, bound: Dist, n: usize) {
        self.packed = (0..1 << 49).contains(&bound) && (n as u64) < Self::FREE;
        self.radix.clear();
        self.heap.clear();
    }

    #[inline]
    fn push(&mut self, d: Dist, v: u32) {
        if self.packed {
            let code = if v == FREE_A { Self::FREE } else { u64::from(v) };
            self.radix.push((d as u64) << Self::BITS | code);
        } else {
            self.heap.push(Reverse((d, v)));
        }
    }

    #[inline]
    fn pop(&mut self) -> Option<(Dist, u32)> {
        if self.packed {
            self.radix.pop().map(|k| {
                let code = k & Self::FREE;
                ((k >> Self::BITS) as Dist, if code == Self::FREE { FREE_A } else { code as u32 })
            })
        } else {
            self.heap.pop().map(|Reverse(entry)| entry)
        }
    }
}

/// A monotone priority queue of keys (Ahuja, Mehlhorn, Orlin and Tarjan's radix heap): every key pushed must be at
/// least the last one popped, and a pop takes the least key.  A key sits in the bucket of the highest bit in which it
/// differs from the last one popped (bucket 0 if it equals it); a pop with bucket 0 empty takes the least key of the
/// first bucket that is not, and spreads that bucket over the ones below.
struct RadixHeap {
    buckets: Vec<Vec<u64>>,
    /// Bit `i` set when bucket `i` is not empty.
    full: u128,
    last: u64,
}

impl RadixHeap {
    fn new() -> Self {
        RadixHeap { buckets: (0..65).map(|_| Vec::new()).collect(), full: 0, last: 0 }
    }

    fn clear(&mut self) {
        let mut full = self.full;
        while full != 0 {
            self.buckets[full.trailing_zeros() as usize].clear();
            full &= full - 1;
        }
        self.full = 0;
        self.last = 0;
    }

    #[inline]
    fn push(&mut self, key: u64) {
        debug_assert!(key >= self.last, "a key below the last one popped");
        let b = (64 - (key ^ self.last).leading_zeros()) as usize;
        self.buckets[b].push(key);
        self.full |= 1 << b;
    }

    #[inline]
    fn pop(&mut self) -> Option<u64> {
        if self.full & 1 == 0 {
            if self.full == 0 {
                return None;
            }
            let i = self.full.trailing_zeros() as usize;
            let mut keys = std::mem::take(&mut self.buckets[i]);
            self.full &= !(1 << i);
            self.last = *keys.iter().min().expect("a bucket marked full");
            for &k in &keys {
                let b = (64 - (k ^ self.last).leading_zeros()) as usize;
                self.buckets[b].push(k);
                self.full |= 1 << b;
            }
            keys.clear();
            self.buckets[i] = keys;
        }
        let k = self.buckets[0].pop();
        if self.buckets[0].is_empty() {
            self.full &= !1;
        }
        k
    }
}

/// A weight split's potentials as its Bellman–Ford found them, the predecessor that gave each its potential, and the
/// live elements and members of the problem it was for ([`Meet::save_split`]): a later split of a related problem
/// starts from the potentials whose paths of predecessors are still paths of its constraint graph
/// ([`Meet::split_bounds_from`]).
#[derive(Clone, Default)]
pub struct SplitStart {
    pot: Vec<i64>,
    pred: Vec<u32>,
    /// By element: 0 if it was not live, 1 if outside the set, 2 if a member.
    role: Vec<u8>,
    saved: bool,
}

/// Cycles of exchange graphs, each its elements in the order of its arcs (from each to the next, and from the last to
/// the first), kept from the searches for forced members ([`Meet::keep_cycle`]) to check against a later exchange
/// graph ([`Meet::cycle_length`]).
#[derive(Clone, Default)]
pub struct Cycles {
    elems: Vec<u32>,
    ends: Vec<usize>,
}

impl Cycles {
    pub fn clear(&mut self) {
        self.elems.clear();
        self.ends.clear();
    }

    pub fn push(&mut self, cycle: &[u32]) {
        self.elems.extend_from_slice(cycle);
        self.ends.push(self.elems.len());
    }

    pub fn iter(&self) -> impl Iterator<Item = &[u32]> {
        let mut start = 0;
        self.ends.iter().map(move |&end| {
            let cycle = &self.elems[start..end];
            start = end;
            cycle
        })
    }
}

/// A copy of a workspace's tableau ([`Meet::save`]) to derive another from later ([`Meet::derive_from`]): per
/// matroid, the elements by block (the blocks' ends in `ends`) with their masks and residues, and the members of each
/// block in slot order (their ends in `slot_ends`).
#[derive(Clone, Default)]
pub struct Snapshot<const R: usize> {
    elems: [Vec<u32>; 2],
    ends: [Vec<usize>; 2],
    rows: [Vec<(Vector<R>, Vector<R>)>; 2],
    slots: [Vec<u32>; 2],
    slot_ends: [Vec<usize>; 2],
}

impl<const R: usize> Meet<R> {
    /// A workspace for problems of `n` elements and `blocks` blocks in each matroid.
    pub fn new(n: usize, blocks: [usize; 2]) -> Self {
        Meet {
            mask: [vec![[0; R]; n], vec![[0; R]; n]],
            res: [vec![[0; R]; n], vec![[0; R]; n]],
            slot: [vec![NO_SLOT; n], vec![NO_SLOT; n]],
            slot_elem: blocks.map(|b| vec![vec![NONE; 64 * R]; b]),
            next_slot: blocks.map(|b| vec![0; b]),
            block_live: blocks.map(|b| vec![Vec::with_capacity(n); b]),
            spans: blocks.map(|b| vec![BlockSpan::new(); b]),
            inside: vec![false; n],
            members: Vec::with_capacity(n),
            out_a: vec![Vec::with_capacity(n); blocks[0]],
            pos: vec![0; n],
            dist: vec![INF; n],
            pred: vec![NONE; n],
            path: Vec::with_capacity(n),
            changed_out: Vec::with_capacity(n),
            changed_in: Vec::with_capacity(n),
            changed_slots: blocks.map(|b| vec![[0; R]; b]),
            seen: vec![0; n],
            generation: 0,
            dirty: blocks.map(|b| vec![false; b]),
            settled: false,
            stopped: false,
            gap: 0,
            gap_for: 0,
            solves: 0,
            tableau: 0,
            value: 0,
            augmentations: 0,
            rounds: 0,
            rebuilds: 0,
            split_rounds: 0,
            pot: vec![0; n + 1],
            split_pred: vec![NONE; n + 1],
            repl: [vec![0; n], vec![0; n]],
            walk: Vec::with_capacity(n + 1),
            #[cfg(debug_assertions)]
            cold: vec![0; n + 1],
            reach: vec![INF; n],
            back: vec![NONE; n],
            settled_at: vec![0; n],
            offered_at: vec![0; n],
            heap: Queue::new(n),
            columns: vec![Vec::new(); blocks[0] * 64 * R],
            columns_for: 0,
            free_a: Vec::with_capacity(n),
            by_w2: Vec::with_capacity(n),
            sorted_for: 0,
            flip: Vec::with_capacity(n),
            found: NONE,
            withouts: 0,
            withouts_exact: 0,
            mark: vec![0; n],
            slot_map: vec![NO_SLOT; 64 * R],
            entering: Vec::new(),
            leaving: Vec::new(),
            fresh: Vec::new(),
            unsure: blocks.map(|b| vec![Vec::new(); b]),
            derived: 0,
            restored: 0,
            built: 0,
            reduced: 0,
            #[cfg(test)]
            check: None,
            #[cfg(test)]
            checked: 0,
        }
    }

    /// Whether `e` is in the heaviest set found by the last [`solve`](Self::solve).
    #[inline]
    pub fn contains(&self, e: u32) -> bool {
        self.inside[e as usize]
    }

    /// The heaviest common independent set of at most `cap` of the elements `live` (ascending), its weight in
    /// `self.value` and its members marked ([`contains`](Self::contains)).  The augmentations start from the larger
    /// of `start` (which must be a dominant common independent set of live elements, see the module docs; it may
    /// be empty) and the heaviest uncontended elements.  `report` is called with the start set and then with each
    /// set an augmentation reaches (all dominant), as the members of the set in no particular order.  The
    /// augmentations stop as soon as the set weighs `stop` or more ([`stopped`](Self::stopped)): the heaviest set
    /// weighs at least that much, since every augmentation gains.
    pub fn solve(
        &mut self,
        p: &Problem<'_, R>,
        live: &[u32],
        start: &[u32],
        cap: usize,
        stop: i64,
        mut report: impl FnMut(&[u32]),
    ) {
        self.solves += 1;
        self.tableau = self.solves;
        for &e in live {
            self.inside[e as usize] = false;
        }
        // The heaviest uncontended elements: the live elements by decreasing weight (index), while common
        // independent.
        for spans in &mut self.spans {
            for s in spans {
                s.clear();
            }
        }
        let mut prefix = 0;
        for &e in live.iter().rev() {
            if prefix == cap || !self.insert_both(p, e) {
                break;
            }
            prefix += 1;
        }
        if start.len() > prefix {
            for &e in start {
                self.inside[e as usize] = true;
            }
        } else {
            for &e in live.iter().rev().take(prefix) {
                self.inside[e as usize] = true;
            }
        }
        for m in 0..2 {
            for list in &mut self.block_live[m] {
                list.clear();
            }
            for &e in live {
                self.block_live[m][usize::from(p.block[m][e as usize])].push(e);
            }
            for b in 0..self.block_live[m].len() {
                self.rebuild(p, m, b);
            }
        }
        self.lists(p, live);
        report(&self.members);
        self.augment(p, cap, stop, report);
    }

    /// The members of the set (`inside`) among the elements `live` (ascending), and the others by their block of A,
    /// each in increasing order; the set's weight.
    fn lists(&mut self, p: &Problem<'_, R>, live: &[u32]) {
        self.members.clear();
        for list in &mut self.out_a {
            list.clear();
        }
        for &e in live {
            let i = e as usize;
            let list = if self.inside[i] { &mut self.members } else { &mut self.out_a[usize::from(p.block[0][i])] };
            self.pos[i] = list.len() as u32;
            list.push(e);
        }
        self.value = self.members.iter().map(|&e| p.weight[e as usize]).sum();
    }

    /// The augmentations of [`solve`](Self::solve) from the set the tableau describes, its weight in `self.value`.
    fn augment(&mut self, p: &Problem<'_, R>, cap: usize, stop: i64, mut report: impl FnMut(&[u32])) {
        self.settled = false;
        self.stopped = false;
        loop {
            if self.value >= stop {
                self.stopped = true;
                return;
            }
            let Some(sink) = self.shortest_path(p) else {
                self.settled = true;
                (self.gap, self.gap_for) = (i64::MAX, self.tableau);
                return;
            };
            let gain = -self.dist[sink as usize].div_euclid(HOPS);
            if gain <= 0 {
                // The heaviest set of all sizes (even if it has `cap` members).
                self.settled = true;
                (self.gap, self.gap_for) = (-gain, self.tableau);
                return;
            }
            if self.members.len() >= cap {
                // A larger set would be heavier, but the limit forbids it.
                return;
            }
            self.path.clear();
            let mut v = sink;
            loop {
                self.path.push(v);
                let i = v as usize;
                self.inside[i] = !self.inside[i];
                self.move_list(p, v);
                if self.pred[i] == NONE {
                    break;
                }
                v = self.pred[i];
            }
            self.path.reverse();
            self.apply_path(p);
            self.value += gain;
            self.augmentations += 1;
            report(&self.members);
        }
    }

    /// The next `k` stamps (`k` at most 2) for `seen` and `settled_at`, the last of them returned: a search compares a
    /// stamp only for equality with its own, so none it finds may be left from an earlier one.  Before the counter
    /// would wrap, every stamp is cleared (the stamps start over at 1); otherwise, after 2^32 stamps, one left from
    /// that long ago would pass for a new one.
    fn next_generation(&mut self, k: u32) -> u32 {
        if self.generation > u32::MAX - k {
            self.seen.fill(0);
            self.settled_at.fill(0);
            self.generation = 0;
        }
        self.generation += k;
        self.generation
    }

    /// In tests: the last stamp given ([`next_generation`](Self::next_generation)), and setting it, to start a
    /// workspace's stamps just below the counter's wrap.
    #[cfg(test)]
    pub(crate) fn generation(&self) -> u32 {
        self.generation
    }

    #[cfg(test)]
    pub(crate) fn set_generation(&mut self, generation: u32) {
        self.generation = generation;
    }

    /// After a solve that settled, or a [`derive`](Self::derive), with the workspace still holding it: how much
    /// lighter the heaviest common independent set one larger than the heaviest set is, `i64::MAX` if there is none.
    /// The heaviest sets of all larger sizes are lighter still (the weights by size are concave).
    pub fn gap(&mut self, p: &Problem<'_, R>) -> i64 {
        if self.gap_for != self.tableau {
            debug_assert!(self.settled && self.tableau == self.solves, "the gap of no settled tableau");
            self.gap = self.find_gap(p);
            self.gap_for = self.tableau;
        }
        self.gap
    }

    /// The gap of the tableau's set, the heaviest of all sizes: the length of the shortest augmenting path, which
    /// gains nothing.
    fn find_gap(&mut self, p: &Problem<'_, R>) -> i64 {
        match self.shortest_path(p) {
            None => i64::MAX,
            Some(sink) => {
                let gain = -self.dist[sink as usize].div_euclid(HOPS);
                debug_assert!(gain <= 0, "a path gains on the heaviest set of all sizes");
                -gain
            }
        }
    }

    /// The weight and size of a common independent set of the elements `live` (ascending) found greedily: `base`
    /// (common independent), then each other element by decreasing weight that keeps it so.  A lower bound on the
    /// heaviest common independent set, for a few reductions per element.
    pub fn greedy_from(&mut self, p: &Problem<'_, R>, live: &[u32], base: &[u32]) -> (i64, usize) {
        // The workspace no longer holds the last solve.
        self.solves += 1;
        for &e in live {
            self.inside[e as usize] = false;
        }
        for spans in &mut self.spans {
            for s in spans {
                s.clear();
            }
        }
        let (mut total, mut size) = (0, base.len());
        for &e in base {
            let added = self.insert_both(p, e);
            debug_assert!(added, "the base of a greedy set is common independent");
            self.inside[e as usize] = true;
            total += p.weight[e as usize];
        }
        for &e in live.iter().rev() {
            let i = e as usize;
            if !self.inside[i] && self.insert_both(p, e) {
                total += p.weight[i];
                size += 1;
            }
        }
        for &e in base {
            self.inside[e as usize] = false;
        }
        (total, size)
    }

    /// Moves `e`, just flipped, to the list of its new side.
    #[inline]
    fn move_list(&mut self, p: &Problem<'_, R>, e: u32) {
        let i = e as usize;
        let block = usize::from(p.block[0][i]);
        let (from, to) = if self.inside[i] {
            (&mut self.out_a[block], &mut self.members)
        } else {
            let (members, out_a) = (&mut self.members, &mut self.out_a);
            (members, &mut out_a[block])
        };
        let at = self.pos[i] as usize;
        from.swap_remove(at);
        if let Some(&moved) = from.get(at) {
            self.pos[moved as usize] = at as u32;
        }
        self.pos[i] = to.len() as u32;
        to.push(e);
    }

    /// Adds `e` to the spans of both matroids if it is independent in both; false (nothing added) otherwise.
    #[inline]
    fn insert_both(&mut self, p: &Problem<'_, R>, e: u32) -> bool {
        let i = e as usize;
        let (ba, bb) = (usize::from(p.block[0][i]), usize::from(p.block[1][i]));
        let (va, vb) = (p.vector[0][i], p.vector[1][i]);
        if is_zero(&self.spans[0][ba].reduce(va).0) || is_zero(&self.spans[1][bb].reduce(vb).0) {
            return false;
        }
        self.spans[0][ba].insert(va, e);
        self.spans[1][bb].insert(vb, e);
        true
    }

    /// Rebuilds block `b` of matroid `m` from its members (the live elements of the block that are `inside`).
    fn rebuild(&mut self, p: &Problem<'_, R>, m: usize, b: usize) {
        self.next_slot[m][b] = 0;
        for &z in &self.block_live[m][b] {
            let z = z as usize;
            self.mask[m][z] = [0; R];
            self.res[m][z] = p.vector[m][z];
            self.slot[m][z] = NO_SLOT;
        }
        for k in 0..self.block_live[m][b].len() {
            let z = self.block_live[m][b][k];
            if self.inside[z as usize] {
                let added = self.add(m, b, z);
                debug_assert!(added, "a member of a common independent set is dependent");
            }
        }
    }

    /// The tableau that [`solve`](Self::solve) makes of the set `start` of the elements `live` (both ascending) of
    /// `p`, from `start` with no size limit nor stop, where `start` is the heaviest set of all sizes (so that the solve
    /// starts from it and augments nothing): its value and `settled` with it, and its [`gap`](Self::gap) when asked
    /// for.  Where it can, it derives the tableau from the one the workspace holds, of a set `S` of elements `L` of a
    /// problem `q`, instead of building it (see "Deriving a tableau" in the docs of [`Meet`]); for that `p` must be
    /// `q` contracted by `contracted` (members of `start ∪ contracted`, which must be common independent in `q`), and
    /// `L` must hold `start` and `contracted` (an element of `live` outside `L`, a loop of `q` in one matroid, say,
    /// has its circuits found by reduction).  If the workspace holds no tableau, or `L` does not hold those, every
    /// block is built.
    pub fn derive(&mut self, p: &Problem<'_, R>, live: &[u32], start: &[u32], contracted: &[u32]) {
        const SOURCE: u8 = 1;
        const LIVE: u8 = 2;
        const WANTED: u8 = 4;
        let held = self.tableau == self.solves;
        self.solves += 1;
        self.tableau = self.solves;
        // The elements of the tableau held are the live elements of its blocks of A.
        for list in &self.block_live[0] {
            for &e in list {
                self.mark[e as usize] = SOURCE;
            }
        }
        let mut ok = held;
        self.fresh.clear();
        for &e in live {
            if self.mark[e as usize] & SOURCE == 0 {
                self.fresh.push(e);
            }
            self.mark[e as usize] |= LIVE;
        }
        for &e in start.iter().chain(contracted) {
            ok &= self.mark[e as usize] & SOURCE != 0;
            self.mark[e as usize] |= WANTED;
        }
        for &e in contracted {
            ok &= self.mark[e as usize] & LIVE == 0;
        }
        // In each block, `start ∪ contracted` from `S` by exchanges: each element that enters, in the span of the
        // set, takes the slot of a member that leaves and is in its circuit, while one does (if the two sets are
        // independent and span the same in the block, every element that enters has one, until none enters).  Where
        // elements are left to enter or leave, the span changes (`dirty`): a circuit that holds no member left to
        // leave lies in what stays, and is the element's circuit in the new set; the other elements outside the new
        // set, the free ones included, are reduced by its members (`unsure`), as are those the tableau did not have.
        for m in 0..2 {
            for b in 0..self.block_live[m].len() {
                self.unsure[m][b].clear();
                if !ok {
                    continue;
                }
                self.entering.clear();
                self.leaving.clear();
                for &e in start.iter().chain(contracted) {
                    if usize::from(p.block[m][e as usize]) == b && !self.inside[e as usize] {
                        self.entering.push(e);
                    }
                }
                for &x in &self.slot_elem[m][b][..self.next_slot[m][b]] {
                    if self.mark[x as usize] & WANTED == 0 {
                        self.leaving.push(x);
                    }
                }
                let mut i = 0;
                while i < self.entering.len() {
                    let yi = self.entering[i] as usize;
                    let (mask, slot) = (&self.mask[m][yi], &self.slot[m]);
                    let k = is_zero(&self.res[m][yi])
                        .then(|| self.leaving.iter().position(|&x| bit(mask, usize::from(slot[x as usize]))))
                        .flatten();
                    if let Some(k) = k {
                        let x = self.leaving.swap_remove(k);
                        let y = self.entering.swap_remove(i);
                        self.exchange(m, b, x, y);
                        // The exchange changed the circuits of the others: look again from the first.
                        i = 0;
                    } else {
                        i += 1;
                    }
                }
                if self.entering.is_empty() && self.leaving.is_empty() {
                    continue;
                }
                self.dirty[m][b] = true;
                let mut gone = [0u64; R];
                for &x in &self.leaving {
                    let s = usize::from(self.slot[m][x as usize]);
                    gone[s / 64] |= 1 << (s % 64);
                }
                for &z in &self.block_live[m][b] {
                    let zi = z as usize;
                    let outside = self.mark[zi] & (LIVE | WANTED) == LIVE;
                    let held = || self.mask[m][zi].iter().zip(&gone).any(|(c, g)| c & g != 0);
                    if outside && (!is_zero(&self.res[m][zi]) || held()) {
                        self.unsure[m][b].push(z);
                    }
                }
            }
        }
        // The new set and its elements; then each block's slots numbered by increasing member and the circuits not
        // known found, or the block built.
        for list in &self.block_live[0] {
            for &e in list {
                self.inside[e as usize] = false;
            }
        }
        for &e in live {
            self.inside[e as usize] = self.mark[e as usize] & WANTED != 0;
        }
        for list in &self.block_live[0] {
            for &e in list {
                self.mark[e as usize] = 0;
            }
        }
        for &e in start.iter().chain(contracted).chain(&self.fresh) {
            self.mark[e as usize] = 0;
        }
        for m in 0..2 {
            for list in &mut self.block_live[m] {
                list.clear();
            }
            for &e in live {
                self.block_live[m][usize::from(p.block[m][e as usize])].push(e);
            }
            for b in 0..self.block_live[m].len() {
                if ok {
                    self.renumber(m, b);
                    self.reduce_unknown(p, m, b);
                } else {
                    self.rebuild(p, m, b);
                    self.built += 1;
                }
                self.dirty[m][b] = false;
            }
        }
        self.derived += 1;
        self.lists(p, live);
        // `start` is the heaviest set of all sizes: the solve would end with the shortest path that gains nothing, whose
        // length is the gap, found when first asked for.
        (self.settled, self.stopped) = (true, false);
        #[cfg(test)]
        if let Some(mut check) = self.check.take() {
            check.solve(p, live, start, usize::MAX, i64::MAX, |_| {});
            // The gap found as `gap` would, without the count of rounds, which the tests compare across engines.
            let rounds = self.rounds;
            let gap = self.find_gap(p);
            self.rounds = rounds;
            assert_eq!(gap, check.gap, "a derived tableau's gap differs from the solve's");
            if let Err(e) = check.same_tableau(self, p, live) {
                panic!("a derived tableau differs from the one a solve builds: {e}");
            }
            self.check = Some(check);
            self.checked += 1;
        }
    }

    /// Whether `other`'s tableau reads the same as this one's outside a solve (see "Deriving a tableau" in the docs
    /// of [`Meet`]) for the live elements `live` of `p`: the same set, lists, free elements, circuits and slots, and
    /// the same value and `settled`.
    #[cfg(test)]
    fn same_tableau(&self, other: &Meet<R>, p: &Problem<'_, R>, live: &[u32]) -> Result<(), String> {
        let same = |what: &str, a: String, b: String| if a == b { Ok(()) } else { Err(format!("{what}: {a} {b}")) };
        same("value", format!("{}", self.value), format!("{}", other.value))?;
        same("settled", format!("{}", self.settled), format!("{}", other.settled))?;
        same("members", format!("{:?}", self.members), format!("{:?}", other.members))?;
        same("outside", format!("{:?}", self.out_a), format!("{:?}", other.out_a))?;
        for m in 0..2 {
            for b in 0..self.block_live[m].len() {
                let slots = |t: &Meet<R>| format!("{:?}", &t.slot_elem[m][b][..t.next_slot[m][b]]);
                same(&format!("slots of block {b} of {m}"), slots(self), slots(other))?;
            }
            for &e in live {
                let i = e as usize;
                let row = |t: &Meet<R>| {
                    let free = !is_zero(&t.res[m][i]);
                    if t.inside[i] {
                        format!("member, slot {}", t.slot[m][i])
                    } else if free {
                        "free".to_string()
                    } else {
                        format!("circuit {:?}", t.mask[m][i])
                    }
                };
                same(&format!("element {e} of block {} of {m}", p.block[m][i]), row(self), row(other))?;
            }
        }
        Ok(())
    }

    /// [`derive`](Self::derive) from the tableau `snapshot` holds (saved by [`save`](Self::save)) instead of the one
    /// the workspace holds.
    pub fn derive_from(
        &mut self,
        snapshot: &Snapshot<R>,
        p: &Problem<'_, R>,
        live: &[u32],
        start: &[u32],
        contracted: &[u32],
    ) {
        let s = snapshot;
        for m in 0..2 {
            let (mut at, mut slot_at) = (0, 0);
            for b in 0..self.block_live[m].len() {
                let list = &mut self.block_live[m][b];
                list.clear();
                for k in at..s.ends[m][b] {
                    let e = s.elems[m][k];
                    list.push(e);
                    (self.mask[m][e as usize], self.res[m][e as usize]) = s.rows[m][k];
                    self.slot[m][e as usize] = NO_SLOT;
                }
                at = s.ends[m][b];
                let slots = &s.slots[m][slot_at..s.slot_ends[m][b]];
                for (k, &e) in slots.iter().enumerate() {
                    self.slot_elem[m][b][k] = e;
                    self.slot[m][e as usize] = k as u16;
                }
                self.next_slot[m][b] = slots.len();
                slot_at = s.slot_ends[m][b];
            }
        }
        for &e in &s.elems[0] {
            self.inside[e as usize] = false;
        }
        for &e in &s.slots[0] {
            self.inside[e as usize] = true;
        }
        // The workspace holds the snapshot's tableau.
        self.tableau = self.solves;
        self.restored += 1;
        self.derive(p, live, start, contracted);
    }

    /// Saves the tableau the workspace holds in `snapshot`, for [`derive_from`](Self::derive_from).
    pub fn save(&self, snapshot: &mut Snapshot<R>) {
        debug_assert_eq!(self.tableau, self.solves, "a snapshot of no tableau");
        let s = snapshot;
        for m in 0..2 {
            for v in [&mut s.elems[m], &mut s.slots[m]] {
                v.clear();
            }
            s.ends[m].clear();
            s.rows[m].clear();
            s.slot_ends[m].clear();
            for b in 0..self.block_live[m].len() {
                for &e in &self.block_live[m][b] {
                    s.elems[m].push(e);
                    s.rows[m].push((self.mask[m][e as usize], self.res[m][e as usize]));
                }
                s.ends[m].push(s.elems[m].len());
                s.slots[m].extend_from_slice(&self.slot_elem[m][b][..self.next_slot[m][b]]);
                s.slot_ends[m].push(s.slots[m].len());
            }
        }
    }

    /// The rows of the live elements of block `b` of matroid `m` whose circuits the tableau derived from does not give:
    /// those it did not have (`fresh`), and those `unsure` where the span changed.  Each is reduced by the span of
    /// the block's members, inserted in slot order, so that its residue is zero exactly when it is in the span, and
    /// its mask is then its circuit.
    fn reduce_unknown(&mut self, p: &Problem<'_, R>, m: usize, b: usize) {
        let unsure = std::mem::take(&mut self.unsure[m][b]);
        if unsure.is_empty() && !self.fresh.iter().any(|&e| usize::from(p.block[m][e as usize]) == b) {
            self.unsure[m][b] = unsure;
            return;
        }
        let span = &mut self.spans[m][b];
        span.clear();
        for &z in &self.block_live[m][b] {
            if self.inside[z as usize] {
                let inserted = span.insert(p.vector[m][z as usize], z);
                debug_assert!(inserted, "a member of a common independent set is dependent");
            }
        }
        for &e in self.fresh.iter().filter(|&&e| usize::from(p.block[m][e as usize]) == b).chain(&unsure) {
            let i = e as usize;
            (self.res[m][i], self.mask[m][i]) = span.reduce(p.vector[m][i]);
            self.slot[m][i] = NO_SLOT;
        }
        self.unsure[m][b] = unsure;
        self.reduced += 1;
    }

    /// Numbers the slots of block `b` of matroid `m` by increasing member among its live elements, as a rebuild does:
    /// each member there has a slot, and the circuit of each other live element in the span is carried over to the
    /// new numbers, less the slots of members that are no longer live elements (contracted), or no longer members
    /// where the span changed (`dirty`; there the members that enter have no slot yet).
    fn renumber(&mut self, m: usize, b: usize) {
        // Members in increasing order in their slots, none of them gone and none new: the numbering is already a
        // rebuild's, and so are the circuits.
        let slots = &self.slot_elem[m][b][..self.next_slot[m][b]];
        let kept = slots.iter().all(|&z| self.inside[z as usize]) && slots.windows(2).all(|pair| pair[0] < pair[1]);
        if kept && !self.dirty[m][b] {
            return;
        }
        self.slot_map.fill(NO_SLOT);
        let mut k = 0u16;
        for &z in &self.block_live[m][b] {
            if self.inside[z as usize] {
                let s = self.slot[m][z as usize];
                if s != NO_SLOT {
                    self.slot_map[usize::from(s)] = k;
                } else {
                    debug_assert!(self.dirty[m][b], "a member without a slot");
                }
                k += 1;
            }
        }
        let mut k = 0;
        for &z in &self.block_live[m][b] {
            let z = z as usize;
            if self.inside[z] {
                self.slot_elem[m][b][k] = z as u32;
                self.slot[m][z] = k as u16;
                let mut own = [0; R];
                own[k / 64] |= 1 << (k % 64);
                self.mask[m][z] = own;
                self.res[m][z] = [0; R];
                k += 1;
            } else {
                self.slot[m][z] = NO_SLOT;
                if is_zero(&self.res[m][z]) {
                    let mut circuit = [0; R];
                    let map = &self.slot_map;
                    each_bit(&self.mask[m][z], |s| {
                        let t = map[s];
                        if t != NO_SLOT {
                            circuit[usize::from(t) / 64] |= 1 << (t % 64);
                        }
                    });
                    self.mask[m][z] = circuit;
                }
            }
        }
        self.next_slot[m][b] = k;
    }

    /// Makes `y` (independent of the tableau's set in block `b` of matroid `m`) a member: a new slot, and the
    /// coordinate of its residue's lowest bit eliminated from every live element of the block.  False if `y` is in
    /// the span.
    #[inline]
    fn add(&mut self, m: usize, b: usize, y: u32) -> bool {
        let yi = y as usize;
        let ry = self.res[m][yi];
        let Some(p) = lowest(&ry) else { return false };
        let s = self.next_slot[m][b];
        self.next_slot[m][b] += 1;
        let mut my = self.mask[m][yi];
        my[s / 64] |= 1 << (s % 64);
        // Branchless: the bit selects all ones or all zeros, which the row and the mask are anded with.
        let (mask, res) = (&mut self.mask[m], &mut self.res[m]);
        let (word, shift) = (p / 64, p % 64);
        for &z in &self.block_live[m][b] {
            let z = z as usize;
            let select = 0u64.wrapping_sub(res[z][word] >> shift & 1);
            for k in 0..R {
                res[z][k] ^= ry[k] & select;
                mask[z][k] ^= my[k] & select;
            }
        }
        // y itself: its residue cleared by its own row, its mask now just its slot.
        self.res[m][yi] = [0; R];
        let mut own = [0; R];
        own[s / 64] |= 1 << (s % 64);
        self.mask[m][yi] = own;
        self.slot[m][yi] = s as u16;
        self.slot_elem[m][b][s] = y;
        true
    }

    /// Exchanges the member `x` for `y` (in the span, its circuit holding `x`) in block `b` of matroid `m`: `y` takes
    /// `x`'s slot, and every live element whose mask holds that slot is re-expressed.
    #[inline]
    fn exchange(&mut self, m: usize, b: usize, x: u32, y: u32) {
        let (xi, yi) = (x as usize, y as usize);
        let s = usize::from(self.slot[m][xi]);
        let mut d = self.mask[m][yi];
        d[s / 64] ^= 1 << (s % 64);
        let mask = &mut self.mask[m];
        let (word, shift) = (s / 64, s % 64);
        for &z in &self.block_live[m][b] {
            let z = z as usize;
            let select = 0u64.wrapping_sub(mask[z][word] >> shift & 1);
            for k in 0..R {
                mask[z][k] ^= d[k] & select;
            }
        }
        // y was in the loop (its mask holds s): it is now just its slot, and x the circuit y had.
        let mut own = [0; R];
        own[s / 64] |= 1 << (s % 64);
        self.mask[m][yi] = own;
        self.slot[m][yi] = s as u16;
        self.slot[m][xi] = NO_SLOT;
        self.slot_elem[m][b][s] = y;
    }

    /// Brings the tableau to the set `self.inside` after the augmentation along `self.path` (`y₀ x₁ y₁ … x_k y_k`):
    /// in `A` it adds `y₀` and exchanges each `x_i` for `y_i`; in `B` it exchanges each `x_i` for `y_{i−1}` and adds
    /// `y_k`.  The changes are applied as pivots as long as one applies; the blocks of any left are rebuilt.
    fn apply_path(&mut self, p: &Problem<'_, R>) {
        let k = self.path.len() / 2;
        for m in 0..2 {
            // (incoming, outgoing or NONE) for this matroid.
            let mut ops: [(u32, u32); 64] = [(NONE, NONE); 64];
            let count = k + 1;
            if count > ops.len() {
                // A very long path: rebuild every block it touches.
                for &v in &self.path {
                    self.dirty[m][usize::from(p.block[m][v as usize])] = true;
                }
            } else {
                for (i, op) in ops.iter_mut().enumerate().take(count) {
                    *op = if m == 0 {
                        (self.path[2 * i], if i == 0 { NONE } else { self.path[2 * i - 1] })
                    } else {
                        (self.path[2 * i], if i == k { NONE } else { self.path[2 * i + 1] })
                    };
                }
                let mut left = count;
                let mut done = [false; 64];
                loop {
                    let mut progressed = false;
                    for (i, &(y, x)) in ops.iter().enumerate().take(count) {
                        if done[i] {
                            continue;
                        }
                        let b = usize::from(p.block[m][y as usize]);
                        let applied = if x == NONE {
                            self.add(m, b, y)
                        } else {
                            let s = self.slot[m][x as usize];
                            let same = usize::from(p.block[m][x as usize]) == b;
                            if same && s != NO_SLOT && bit(&self.mask[m][y as usize], usize::from(s)) {
                                self.exchange(m, b, x, y);
                                true
                            } else {
                                false
                            }
                        };
                        if applied {
                            done[i] = true;
                            left -= 1;
                            progressed = true;
                        }
                    }
                    if left == 0 || !progressed {
                        break;
                    }
                }
                for (i, &(y, x)) in ops.iter().enumerate().take(count) {
                    if !done[i] {
                        self.dirty[m][usize::from(p.block[m][y as usize])] = true;
                        if x != NONE {
                            self.dirty[m][usize::from(p.block[m][x as usize])] = true;
                        }
                    }
                }
            }
            for b in 0..self.dirty[m].len() {
                if self.dirty[m][b] {
                    self.dirty[m][b] = false;
                    self.rebuild(p, m, b);
                    self.rebuilds += 1;
                }
            }
        }
    }

    /// Bellman–Ford from the sources (elements independent of `I` in `A`), lexicographic in length and arcs.
    /// Returns the sink (an element independent of `I` in `B`) at the least distance, if any is reached.
    ///
    /// The graph is bipartite, so a round has two phases: arcs from outside to members (`B`'s exchanges), then from
    /// members to outside (`A`'s).  Each phase relaxes only the arcs out of vertices whose distance changed in the
    /// phase before, which reaches the distances a full round would: an unchanged vertex has already offered its
    /// distance along all its arcs.  An element independent of `I` has arcs to or from every member; those go
    /// through one minimum, offered again only when it improves.  A pull into an outside element reads only the
    /// circuit members that changed: its mask and the mask of the block's changed slots (a block of `A` with none
    /// is skipped).  The best sink is kept as distances fall.
    fn shortest_path(&mut self, p: &Problem<'_, R>) -> Option<u32> {
        let w = p.weight;
        for &x in &self.members {
            self.dist[x as usize] = INF;
            self.pred[x as usize] = NONE;
        }
        self.changed_out.clear();
        let mut sink = (INF, NONE);
        for list in &self.out_a {
            for &y in list {
                let i = y as usize;
                self.pred[i] = NONE;
                if is_zero(&self.res[0][i]) {
                    self.dist[i] = INF;
                } else {
                    let d = -w[i] * HOPS;
                    self.dist[i] = d;
                    self.changed_out.push(y);
                    if !is_zero(&self.res[1][i]) && d < sink.0 {
                        sink = (d, y);
                    }
                }
            }
        }
        let limit = self.pos.len() + 2;
        let (mut best_free, mut best_member) = ((INF, NONE), (INF, NONE));
        let mut rounds = 0;
        while !self.changed_out.is_empty() {
            rounds += 1;
            assert!(rounds <= limit, "a negative cycle in the exchange graph of an extreme set");
            let generation = self.next_generation(1);
            // Outside to members.
            self.changed_in.clear();
            let mut free_improved = false;
            for k in 0..self.changed_out.len() {
                let y = self.changed_out[k];
                let i = y as usize;
                let d = self.dist[i];
                if !is_zero(&self.res[1][i]) {
                    if d < best_free.0 {
                        best_free = (d, y);
                        free_improved = true;
                    }
                    continue;
                }
                let elems = &self.slot_elem[1][usize::from(p.block[1][i])];
                let (dist, pred, seen, changed) =
                    (&mut self.dist, &mut self.pred, &mut self.seen, &mut self.changed_in);
                each_bit(&self.mask[1][i], |s| {
                    let x = elems[s];
                    let xi = x as usize;
                    let cand = d + w[xi] * HOPS + 1;
                    if cand < dist[xi] {
                        dist[xi] = cand;
                        pred[xi] = y;
                        if seen[xi] != generation {
                            seen[xi] = generation;
                            changed.push(x);
                        }
                    }
                });
            }
            if free_improved {
                let (d, y) = best_free;
                for &x in &self.members {
                    let xi = x as usize;
                    let cand = d + w[xi] * HOPS + 1;
                    if cand < self.dist[xi] {
                        self.dist[xi] = cand;
                        self.pred[xi] = y;
                        if self.seen[xi] != generation {
                            self.seen[xi] = generation;
                            self.changed_in.push(x);
                        }
                    }
                }
            }
            // Members to outside.
            self.changed_slots[0].fill([0; R]);
            let mut member_improved = false;
            for &x in &self.changed_in {
                let xi = x as usize;
                let s = usize::from(self.slot[0][xi]);
                let slots = &mut self.changed_slots[0][usize::from(p.block[0][xi])];
                slots[s / 64] |= 1 << (s % 64);
                if self.dist[xi] < best_member.0 {
                    best_member = (self.dist[xi], x);
                    member_improved = true;
                }
            }
            self.changed_out.clear();
            for (ba, list) in self.out_a.iter().enumerate() {
                let changed = self.changed_slots[0][ba];
                let any_changed = !is_zero(&changed);
                if !any_changed && !member_improved {
                    continue;
                }
                let elems = &self.slot_elem[0][ba];
                for &y in list {
                    let i = y as usize;
                    let ly = 1 - w[i] * HOPS;
                    let mut best = self.dist[i];
                    let mut from = NONE;
                    if !is_zero(&self.res[0][i]) {
                        if member_improved && best_member.0 + ly < best {
                            (best, from) = (best_member.0 + ly, best_member.1);
                        }
                    } else if any_changed {
                        let mut m = self.mask[0][i];
                        let mut any = false;
                        for (a, c) in m.iter_mut().zip(&changed) {
                            *a &= c;
                            any |= *a != 0;
                        }
                        if any {
                            let dist = &self.dist;
                            each_bit(&m, |s| {
                                let x = elems[s];
                                let d = dist[x as usize];
                                if d + ly < best {
                                    (best, from) = (d + ly, x);
                                }
                            });
                        }
                    }
                    if from != NONE {
                        self.dist[i] = best;
                        self.pred[i] = from;
                        self.changed_out.push(y);
                        if best < sink.0 && !is_zero(&self.res[1][i]) {
                            sink = (best, y);
                        }
                    }
                }
            }
        }
        self.rounds += rounds as u64;
        (sink.1 != NONE).then_some(sink.1)
    }

    /// After a [`solve`](Self::solve) that settled (see [`settled`](Self::settled)): the bound on the heaviest
    /// common independent set without each member `x` of the heaviest set, from a weight split (see the module
    /// docs), passed to `f(x, bound)`.  False (and `f` never called) if the split's constraints have no solution,
    /// which Frank's theorem rules out for a settled solve (a test checks it).
    pub fn split_bounds(&mut self, p: &Problem<'_, R>, live: &[u32], f: impl FnMut(u32, i64)) -> bool {
        self.split_bounds_from(p, live, None, f)
    }

    /// The split's arc `q → u` in the current tableau (`zero` the zero node), as [`split_bounds`](Self::split_bounds)
    /// lists them.
    fn split_arc(&self, p: &Problem<'_, R>, q: usize, u: usize, zero: usize) -> bool {
        if q == zero {
            return self.inside[u] || !is_zero(&self.res[0][u]);
        }
        if u == zero {
            return self.inside[q] || !is_zero(&self.res[1][q]);
        }
        match (self.inside[q], self.inside[u]) {
            (true, false) => {
                is_zero(&self.res[0][u])
                    && p.block[0][q] == p.block[0][u]
                    && bit(&self.mask[0][u], usize::from(self.slot[0][q]))
            }
            (false, true) => {
                is_zero(&self.res[1][q])
                    && p.block[1][q] == p.block[1][u]
                    && bit(&self.mask[1][q], usize::from(self.slot[1][u]))
            }
            _ => false,
        }
    }

    /// Lowers the split's starting potentials to those of `start` (an earlier split) at the vertices whose path of
    /// predecessors there is still a path of the constraint graph, as long: every vertex on it live then and now, a
    /// member then exactly when a member now (which fixes the lengths of its arcs), and every arc still an arc.  Each
    /// such potential is the length of a path from the virtual source, so no lower than the shortest distance.  A
    /// walk up the predecessors from each vertex, stopped at the first whose status is known.
    fn warm_start(&mut self, p: &Problem<'_, R>, live: &[u32], start: &SplitStart) {
        let zero = self.pot.len() - 1;
        let g = self.next_generation(4);
        let (live_mark, on_path, valid, invalid) = (g - 3, g - 2, g - 1, g);
        for &e in live {
            self.settled_at[e as usize] = live_mark;
        }
        let base = start.pot[zero];
        let mut walk = std::mem::take(&mut self.walk);
        // The zero node's status (0 unknown, then valid or invalid), and whether it is on the walk.
        let mut zero_status = 0;
        for v in live.iter().map(|&e| e as usize).chain(std::iter::once(zero)) {
            walk.clear();
            let mut zero_on_walk = false;
            let mut u = v;
            let ok = loop {
                let status = if u == zero {
                    if zero_on_walk {
                        invalid
                    } else {
                        zero_status
                    }
                } else {
                    let stamp = self.seen[u];
                    if stamp == on_path {
                        invalid
                    } else if stamp == valid || stamp == invalid {
                        stamp
                    } else {
                        0
                    }
                };
                if status != 0 {
                    break status == valid;
                }
                walk.push(u as u32);
                if u == zero {
                    zero_on_walk = true;
                } else {
                    if self.settled_at[u] != live_mark || start.role[u] != 1 + u8::from(self.inside[u]) {
                        break false;
                    }
                    self.seen[u] = on_path;
                }
                let q = start.pred[u];
                if q == NONE {
                    break true;
                }
                if !self.split_arc(p, q as usize, u, zero) {
                    break false;
                }
                u = q as usize;
            };
            let status = if ok { valid } else { invalid };
            for &w in &walk {
                let w = w as usize;
                if w == zero {
                    zero_status = status;
                } else {
                    self.seen[w] = status;
                }
                if ok {
                    let potential = if w == zero { base } else { start.pot[w] + base };
                    if potential < self.pot[w] {
                        self.pot[w] = potential;
                        self.split_pred[w] = start.pred[w];
                    }
                }
            }
        }
        self.walk = walk;
    }

    /// The split's potentials (before they are shifted to make `w1`): Bellman–Ford from all zero, lowered first to
    /// those of `start` where they still hold (see [`warm_start`](Self::warm_start)).  False if the constraints have
    /// no solution.
    fn split_potentials(&mut self, p: &Problem<'_, R>, live: &[u32], start: Option<&SplitStart>) -> bool {
        let w = p.weight;
        // Potentials for the constraints w1(v) − w1(u) ≤ c, as arcs u → v of length c, from potentials all zero:
        // Bellman–Ford from a virtual source with an arc of length 0 to every vertex, whose distances are the same
        // whatever the order of the relaxations.  The zero node is `zero`; its arcs: to every member (w1(x) ≤ w(x))
        // and every element free in A (w1(y) ≤ 0), and from every member (w1(x) ≥ 0) and every element free in B
        // (w1(y) ≥ w(y)).  A member's other arcs go to the elements whose A-circuit holds it (w1(y) ≤ w1(x)), an
        // element's in the span of B to the members of its B-circuit (w2(y) ≤ w2(x)).  Each round relaxes the arcs
        // out of the vertices whose potential fell in the round before (every vertex in the first).
        let zero = self.pot.len() - 1;
        for &e in live {
            self.pot[e as usize] = 0;
            self.split_pred[e as usize] = NONE;
        }
        self.pot[zero] = 0;
        self.split_pred[zero] = NONE;
        if let Some(start) = start.filter(|start| start.saved) {
            self.warm_start(p, live, start);
        }
        self.changed_in.clear();
        self.changed_in.extend_from_slice(live);
        self.changed_in.push(zero as u32);
        let limit = live.len() + 3;
        let mut rounds = 0;
        while !self.changed_in.is_empty() {
            rounds += 1;
            self.split_rounds += 1;
            if rounds > limit {
                return false;
            }
            let generation = self.next_generation(1);
            self.changed_out.clear();
            let (pot, seen, next) = (&mut self.pot, &mut self.seen, &mut self.changed_out);
            let pred = &mut self.split_pred;
            let mut zero_fell = false;
            // Offers the potential `d` to `v` from `u`.
            macro_rules! relax {
                ($v:expr, $d:expr, $u:expr) => {{
                    let (v, d): (usize, i64) = ($v, $d);
                    if d < pot[v] {
                        pot[v] = d;
                        pred[v] = $u as u32;
                        if v == zero {
                            zero_fell = true;
                        } else if seen[v] != generation {
                            seen[v] = generation;
                            next.push(v as u32);
                        }
                    }
                }};
            }
            for &u in &self.changed_in {
                let u = u as usize;
                let pu = pot[u];
                if u == zero {
                    for &x in &self.members {
                        relax!(x as usize, pu + w[x as usize], u);
                    }
                    for &y in &self.free_a {
                        relax!(y as usize, pu, u);
                    }
                } else if self.inside[u] {
                    relax!(zero, pu, u);
                    let column = usize::from(p.block[0][u]) * 64 * R + usize::from(self.slot[0][u]);
                    for &y in &self.columns[column] {
                        relax!(y as usize, pu, u);
                    }
                } else if !is_zero(&self.res[1][u]) {
                    relax!(zero, pu - w[u], u);
                } else {
                    let elems = &self.slot_elem[1][usize::from(p.block[1][u])];
                    each_bit(&self.mask[1][u], |s| {
                        let x = elems[s] as usize;
                        relax!(x, pu + w[x] - w[u], u);
                    });
                }
            }
            if zero_fell {
                next.push(zero as u32);
            }
            std::mem::swap(&mut self.changed_in, &mut self.changed_out);
        }
        true
    }

    /// [`split_bounds`](Self::split_bounds), its Bellman–Ford started from the potentials of an earlier split `start`
    /// where they still hold ([`warm_start`](Self::warm_start)): the same split, since the shortest distances are
    /// the same from any potentials that are lengths of paths and no lower (debug builds check it against the split
    /// from all zero).
    pub fn split_bounds_from(
        &mut self,
        p: &Problem<'_, R>,
        live: &[u32],
        start: Option<&SplitStart>,
        mut f: impl FnMut(u32, i64),
    ) -> bool {
        debug_assert!(self.settled);
        let w = p.weight;
        if self.columns_for != self.tableau {
            self.build_columns();
        }
        let zero = self.pot.len() - 1;
        #[cfg(debug_assertions)]
        let cold = start.is_some() && {
            let solved = self.split_potentials(p, live, None);
            self.cold.copy_from_slice(&self.pot);
            solved
        };
        let solved = self.split_potentials(p, live, start);
        #[cfg(debug_assertions)]
        if start.is_some() {
            assert_eq!(solved, cold, "a split from a start and the split from all zero disagree on a solution");
            for e in live.iter().map(|&e| e as usize).chain(std::iter::once(zero)) {
                assert!(!solved || self.pot[e] == self.cold[e], "the split from a start differs at {e}");
            }
        }
        if !solved {
            return false;
        }
        // The split: w1 = pot − pot[zero], w2 = w − w1.  Members have 0 ≤ w1, w2 ≤ w.
        let base = self.pot[zero];
        for &e in live {
            self.pot[e as usize] -= base;
        }
        // In each matroid the members of positive weight there, B_m, are a heaviest basis of the elements of
        // positive weight; without a member x of B_m the heaviest independent set is B_m − x plus the heaviest
        // positive element whose circuit with respect to B_m holds x (one exchange), if any.  In a block where
        // every member has positive weight, B_m is I* there and the circuits are the tableau's masks; in a block
        // with a member of weight 0 the circuits with respect to I* would miss the replacements through it, so
        // B_m's span there is built and the elements reduced by it.
        for m in 0..2 {
            let weight_in = |i: usize, pot: &[i64]| if m == 0 { pot[i] } else { w[i] - pot[i] };
            let mut has_zero = [false; 8];
            for &e in live {
                let i = e as usize;
                self.repl[m][i] = 0;
                if self.inside[i] && weight_in(i, &self.pot) <= 0 {
                    has_zero[usize::from(p.block[m][i])] = true;
                }
            }
            for (b, span) in self.spans[m].iter_mut().enumerate() {
                if has_zero[b] {
                    span.clear();
                }
            }
            for &e in live {
                let i = e as usize;
                let b = usize::from(p.block[m][i]);
                if has_zero[b] && self.inside[i] && weight_in(i, &self.pot) > 0 {
                    let inserted = self.spans[m][b].insert(p.vector[m][i], e);
                    debug_assert!(inserted, "a member of a common independent set is dependent");
                }
            }
            for &e in live {
                let i = e as usize;
                let wy = weight_in(i, &self.pot);
                if wy <= 0 || self.inside[i] {
                    continue;
                }
                let b = usize::from(p.block[m][i]);
                let repl = &mut self.repl[m];
                if has_zero[b] {
                    let span = &self.spans[m][b];
                    let (r, c) = span.reduce(p.vector[m][i]);
                    // A positive element independent of B_m would make B_m + y heavier: the split rules it out.
                    debug_assert!(is_zero(&r), "an element of positive weight is free of a heaviest basis");
                    each_bit(&c, |slot| {
                        let x = span.elem[slot] as usize;
                        repl[x] = repl[x].max(wy);
                    });
                } else {
                    debug_assert!(is_zero(&self.res[m][i]), "an element of positive weight is free of I*");
                    let elems = &self.slot_elem[m][b];
                    each_bit(&self.mask[m][i], |s| {
                        let x = elems[s] as usize;
                        repl[x] = repl[x].max(wy);
                    });
                }
            }
        }
        for &e in live {
            let i = e as usize;
            if !self.inside[i] {
                continue;
            }
            let (w1, w2) = (self.pot[i], w[i] - self.pot[i]);
            let mut bound = self.value;
            if w1 > 0 {
                bound -= w1 - self.repl[0][i];
            }
            if w2 > 0 {
                bound -= w2 - self.repl[1][i];
            }
            f(e, bound);
        }
        true
    }

    /// After a [`solve`](Self::solve) that settled, with the workspace still holding it (no solve or greedy set
    /// since): the heaviest set flipped along the shortest cycle through its member `x` in the exchange graph, if
    /// one is at most `limit` long: its weight (the elements of the cycle in [`cycle`](Self::cycle)), a common
    /// independent set without `x`.  It is the heaviest without `x` if the sets of other sizes than the heaviest
    /// set's lose at least as much (see the module docs, "The heaviest set without a member"), which the caller
    /// judges.  `pot` is the weight split of the solve's heaviest set (its `w1` by element, as
    /// [`potentials`](Self::potentials) has it after [`split_bounds`](Self::split_bounds)).
    /// The cycle avoids the elements of `banned` (and the value is then the heaviest set's without `x` among the sets
    /// that hold every member of `banned` and no other element of it: the decomposition of the module docs keeps
    /// such sets' parts within the elements not banned).  The tableau is left as it was.
    ///
    /// Dijkstra's search from `x` over the split's reduced lengths, `w1(z) − w1(y)` on an arc `z → y` of `A` and
    /// `w2(z) − w2(y)` on an arc `y → z` of `B`, which are never negative and add up along a cycle to its length;
    /// distances packed with the number of arcs as for the augmenting paths, so that of the shortest cycles one with
    /// the fewest arcs is found.  An element free in `A` has an arc from every member, so all of them are reached
    /// through the member with the least distance plus `w1`, in decreasing order of their own `w1`; an element free
    /// in `B` has an arc to every member, `x` included, so the members are reached through the element free in `B`
    /// with the least distance less `w2`, in increasing order of their own `w2`: the same distances, the same order
    /// of the vertices settled, and each member's predecessor the element whose offer reached its distance first, as
    /// if each arc were offered.  The search stops at the first distance that no cycle within `limit` and shorter
    /// than the best found can pass.
    pub fn without(&mut self, p: &Problem<'_, R>, pot: &[i64], x: u32, limit: i64, banned: &[u32]) -> Option<i64> {
        self.withouts += 1;
        let (length, end) = self.shortest_cycle(p, pot, x, limit, banned, false)?;
        // The cycle's elements, and the set flipped along it checked common independent in the blocks it touches
        // (the others hold the same members as before).
        self.flip.clear();
        self.flip.push(x);
        let mut v = end;
        while v != x && v != NONE && self.flip.len() <= self.pos.len() {
            self.flip.push(v);
            v = self.back[v as usize];
        }
        if v != x || !self.flipped_independent(p) {
            debug_assert!(false, "the shortest cycle through a member flips into a dependent set");
            return None;
        }
        self.withouts_exact += 1;
        Some(self.value - length)
    }

    /// Whether the exchange graph of [`without`](Self::without) has a cycle through the member `x` at most `limit`
    /// long (the search stops at the first it finds, and the set it flips to is not checked): if it has none, the
    /// sets without `x` of the heaviest set's size lose more than `limit` against it.
    /// Where the search cannot be made (no settled solve's tableau holding `x`, or a limit out of range), the answer
    /// is yes, which claims nothing.
    pub fn has_cycle_within(&mut self, p: &Problem<'_, R>, pot: &[i64], x: u32, limit: i64) -> bool {
        let searchable = self.settled
            && self.tableau == self.solves
            && self.inside[x as usize]
            && (0..i64::MAX / (2 * HOPS)).contains(&limit);
        self.found = NONE;
        if !searchable {
            return true;
        }
        let Some((_, end)) = self.shortest_cycle(p, pot, x, limit, &[], true) else { return false };
        self.found = end;
        true
    }

    /// After a [`has_cycle_within`](Self::has_cycle_within) through `x` that found a cycle: that cycle, added to
    /// `cycles` from `x` on, and returned (empty if there is none).  Each of its members has it through it too, as
    /// long.
    pub fn keep_cycle<'c>(&self, x: u32, cycles: &'c mut Cycles) -> &'c [u32] {
        // The search's predecessors from the element that closed the cycle lead back to x, against the arcs.
        let start = cycles.elems.len();
        let (mut v, mut steps) = (self.found, 0);
        while v != x && v != NONE && steps <= self.pos.len() {
            cycles.elems.push(v);
            v = self.back[v as usize];
            steps += 1;
        }
        if v != x {
            cycles.elems.truncate(start);
            return &[];
        }
        cycles.elems.push(x);
        cycles.elems[start..].reverse();
        cycles.ends.push(cycles.elems.len());
        &cycles.elems[start..]
    }

    /// Whether `cycle`, live elements in the order of its arcs, is a cycle of the exchange graph of the set whose
    /// tableau the workspace holds: if so, its length, the weights of its members less those of the others.  It
    /// costs a bit test an arc, where a search for a cycle settles hundreds of vertices.
    pub fn cycle_length(&self, p: &Problem<'_, R>, cycle: &[u32]) -> Option<i64> {
        let mut length = 0;
        let mut u = *cycle.last()? as usize;
        for &v in cycle {
            let v = v as usize;
            if self.inside[u] == self.inside[v] {
                return None;
            }
            // An arc of A from a member to an element outside free in A or whose circuit there holds it, or one of B
            // to a member from an element outside free in B or whose circuit there holds it.
            let (m, x, y) = if self.inside[u] { (0, u, v) } else { (1, v, u) };
            let in_circuit = || p.block[m][x] == p.block[m][y] && bit(&self.mask[m][y], usize::from(self.slot[m][x]));
            if is_zero(&self.res[m][y]) && !in_circuit() {
                return None;
            }
            length += if self.inside[u] { p.weight[u] } else { -p.weight[u] };
            u = v;
        }
        Some(length)
    }

    /// The search of [`without`](Self::without): the shortest cycle's length and the element that closes it at `x`,
    /// or with `first` the first cycle found within `limit`.
    fn shortest_cycle(
        &mut self,
        p: &Problem<'_, R>,
        pot: &[i64],
        x: u32,
        limit: i64,
        banned: &[u32],
        first: bool,
    ) -> Option<(i64, u32)> {
        if !self.settled || self.tableau != self.solves || !self.inside[x as usize] {
            debug_assert!(false, "the bound without a member needs the tableau of a settled solve holding it");
            return None;
        }
        if !(0..i64::MAX / (2 * HOPS)).contains(&limit) {
            return None;
        }
        if self.columns_for != self.tableau {
            self.build_columns();
        }
        self.sort_by_split(p, pot);
        let w = p.weight;
        let w1 = |e: u32| pot[e as usize];
        let w2 = |e: u32| w[e as usize] - pot[e as usize];
        // Two fresh stamps: reached (the distance in `reach` is current) and settled.
        let settled = self.next_generation(2);
        let reached = settled - 1;
        let (reach, back, seen, stamp) = (&mut self.reach, &mut self.back, &mut self.seen, &mut self.settled_at);
        // Distances from `limit + 1` on cannot lead to a cycle that certifies.
        let bound = (limit + 1) * HOPS;
        let (heap, offered_at, by_w2) = (&mut self.heap, &mut self.offered_at, &self.by_w2);
        heap.start(bound, self.pos.len());
        // A banned element counts as settled: never offered, nor reached as an element free in A.
        for &e in banned {
            stamp[e as usize] = settled;
        }
        let mut cycle = (bound, NONE);
        // The elements free in A, by decreasing w1: the next one unsettled, and the member through which they are
        // reached (the least distance plus w1, packed).
        let mut next_free = 0;
        let mut through = (INF, NONE);
        // The members as reached from the elements free in B (each has an arc to every member): the hub, the least
        // distance less w2 (packed, with the arc) over those settled, the first that reached it and the vertex
        // count when it did; and the members by increasing w2, of which the next one unsettled is queued at the
        // hub's distance plus its w2.  A member's distance is the least of the hub's and its own offers', and its
        // predecessor the one that offered that first, as if each element free in B offered every member.
        let mut hub = (INF, NONE, 0u32);
        let mut next_member = 0;
        // The vertices settled so far, which orders the offers.
        let mut tick = 0u32;
        // Offers the distance `d` to `v` from `from`.
        macro_rules! offer {
            ($v:expr, $d:expr, $from:expr) => {{
                let (v, d): (u32, Dist) = ($v, $d);
                let i = v as usize;
                if d < bound && stamp[i] != settled && (seen[i] != reached || d < reach[i]) {
                    seen[i] = reached;
                    reach[i] = d;
                    back[i] = $from;
                    offered_at[i] = tick;
                    heap.push(d, v);
                }
            }};
        }
        // Queues the next member still unsettled, through the hub.
        macro_rules! queue_member {
            () => {{
                while next_member < by_w2.len() && stamp[by_w2[next_member] as usize] == settled {
                    next_member += 1;
                }
                if let (Some(&z), true) = (by_w2.get(next_member), hub.0 != INF) {
                    let dz = hub.0 + w2(z) * HOPS;
                    if dz < bound {
                        heap.push(dz, z);
                    }
                }
            }};
        }
        // Queues the next element free in A still unsettled, through `through`.
        macro_rules! queue_free {
            () => {{
                while next_free < self.free_a.len() && stamp[self.free_a[next_free] as usize] == settled {
                    next_free += 1;
                }
                if let Some(&y) = self.free_a.get(next_free) {
                    let dy = through.0 - w1(y) * HOPS + 1;
                    if dy < bound {
                        heap.push(dy, FREE_A);
                    }
                }
            }};
        }
        heap.push(0, x);
        seen[x as usize] = reached;
        reach[x as usize] = 0;
        back[x as usize] = NONE;
        while let Some((d, v)) = heap.pop() {
            if d >= cycle.0 || (first && cycle.1 != NONE) {
                break;
            }
            let v = if v == FREE_A {
                // The element free in A with the largest w1 still unsettled, unless the entry is stale.
                while next_free < self.free_a.len() && stamp[self.free_a[next_free] as usize] == settled {
                    next_free += 1;
                }
                let Some(&y) = self.free_a.get(next_free) else { continue };
                if d != through.0 - w1(y) * HOPS + 1 {
                    continue;
                }
                seen[y as usize] = reached;
                reach[y as usize] = d;
                back[y as usize] = through.1;
                stamp[y as usize] = settled;
                queue_free!();
                y
            } else {
                let i = v as usize;
                if stamp[i] == settled {
                    continue;
                }
                if self.inside[i] {
                    let own = if seen[i] == reached { reach[i] } else { INF };
                    let via_hub = if hub.0 == INF { INF } else { hub.0 + w2(v) * HOPS };
                    let via_hub = if via_hub < bound { via_hub } else { INF };
                    if d > own.min(via_hub) {
                        continue;
                    }
                    if via_hub < own || (via_hub == own && hub.2 < offered_at[i]) {
                        seen[i] = reached;
                        reach[i] = via_hub;
                        back[i] = hub.1;
                    }
                    stamp[i] = settled;
                    if by_w2.get(next_member) == Some(&v) {
                        queue_member!();
                    }
                } else {
                    if d > reach[i] {
                        continue;
                    }
                    stamp[i] = settled;
                }
                v
            };
            tick += 1;
            let i = v as usize;
            if self.inside[i] {
                // A member: arcs of A to the elements whose circuit holds it, and to every element free in A.
                let column = usize::from(p.block[0][i]) * 64 * R + usize::from(self.slot[0][i]);
                for &y in &self.columns[column] {
                    offer!(y, d + (w1(v) - w1(y)) * HOPS + 1, v);
                }
                if d + w1(v) * HOPS < through.0 {
                    through = (d + w1(v) * HOPS, v);
                    queue_free!();
                }
            } else if !is_zero(&self.res[1][i]) {
                // Free in B: arcs to every member, x's closing a cycle, the others' through the hub.
                if d + (w2(x) - w2(v)) * HOPS + 1 < cycle.0 {
                    cycle = (d + (w2(x) - w2(v)) * HOPS + 1, v);
                }
                if d - w2(v) * HOPS + 1 < hub.0 {
                    hub = (d - w2(v) * HOPS + 1, v, tick);
                    queue_member!();
                }
            } else {
                // The members of its circuit in B.
                let elems = &self.slot_elem[1][usize::from(p.block[1][i])];
                each_bit(&self.mask[1][i], |s| {
                    let z = elems[s];
                    let dz = d + (w2(z) - w2(v)) * HOPS + 1;
                    if z == x {
                        if dz < cycle.0 {
                            cycle = (dz, v);
                        }
                    } else {
                        offer!(z, dz, v);
                    }
                });
            }
        }
        (cycle.1 != NONE).then(|| (cycle.0.div_euclid(HOPS), cycle.1))
    }

    /// After a [`solve`](Self::solve) that settled, with the workspace still holding it, and `pot` the weight split
    /// of its heaviest set (as for [`without`](Self::without)): the least that a smaller common independent set
    /// loses against the heaviest set.  The symmetric difference of the two splits into cycles of the exchange graph
    /// and paths from member to member, at least one (see the module docs), so that loss is at least the shortest
    /// such path, a member alone included, whose length is the sum of the reduced lengths of its arcs plus `w2` of
    /// its first member and `w1` of its last: Dijkstra's search from every member at its `w2`, as in
    /// [`without`](Self::without).  `i64::MAX` if there is no smaller set (an empty heaviest set).
    pub fn smaller_loss(&mut self, p: &Problem<'_, R>, pot: &[i64]) -> i64 {
        if self.columns_for != self.tableau {
            self.build_columns();
        }
        self.sort_by_split(p, pot);
        let w = p.weight;
        let w1 = |e: u32| pot[e as usize];
        let w2 = |e: u32| w[e as usize] - pot[e as usize];
        let settled = self.next_generation(2);
        let reached = settled - 1;
        let (reach, seen, stamp) = (&mut self.reach, &mut self.seen, &mut self.settled_at);
        let heap = &mut self.heap;
        heap.start(INF, self.pos.len());
        let mut best = INF;
        macro_rules! offer {
            ($v:expr, $d:expr) => {{
                let (v, d): (u32, Dist) = ($v, $d);
                let i = v as usize;
                if d < best && stamp[i] != settled && (seen[i] != reached || d < reach[i]) {
                    seen[i] = reached;
                    reach[i] = d;
                    heap.push(d, v);
                }
            }};
        }
        let mut next_free = 0;
        let mut through = INF;
        macro_rules! queue_free {
            () => {{
                while next_free < self.free_a.len() && stamp[self.free_a[next_free] as usize] == settled {
                    next_free += 1;
                }
                if let Some(&y) = self.free_a.get(next_free) {
                    heap.push(through - w1(y) * HOPS + 1, FREE_A);
                }
            }};
        }
        for &z in &self.members {
            offer!(z, w2(z) * HOPS);
        }
        while let Some((d, v)) = heap.pop() {
            if d >= best {
                break;
            }
            let v = if v == FREE_A {
                while next_free < self.free_a.len() && stamp[self.free_a[next_free] as usize] == settled {
                    next_free += 1;
                }
                let Some(&y) = self.free_a.get(next_free) else { continue };
                if d != through - w1(y) * HOPS + 1 {
                    continue;
                }
                stamp[y as usize] = settled;
                queue_free!();
                y
            } else {
                if stamp[v as usize] == settled || d > reach[v as usize] {
                    continue;
                }
                stamp[v as usize] = settled;
                v
            };
            let i = v as usize;
            if self.inside[i] {
                // A path may end here, at w1, or go on along the arcs of A.
                best = best.min(d + w1(v) * HOPS);
                let column = usize::from(p.block[0][i]) * 64 * R + usize::from(self.slot[0][i]);
                for &y in &self.columns[column] {
                    offer!(y, d + (w1(v) - w1(y)) * HOPS + 1);
                }
                if d + w1(v) * HOPS < through {
                    through = d + w1(v) * HOPS;
                    queue_free!();
                }
            } else if !is_zero(&self.res[1][i]) {
                // Free in B: its arcs to the members offer each `d − w2(v) + 1 + w2(z)`, at least `w2(z) + 1` (the
                // distances are not negative, and the split makes `w2(v) ≤ 0` for an element free in B), while each
                // member started at `w2(z)`: none is an improvement.
            } else {
                let elems = &self.slot_elem[1][usize::from(p.block[1][i])];
                each_bit(&self.mask[1][i], |s| {
                    let z = elems[s];
                    offer!(z, d + (w2(z) - w2(v)) * HOPS + 1);
                });
            }
        }
        if best == INF {
            i64::MAX
        } else {
            best.div_euclid(HOPS)
        }
    }

    /// For [`split_bounds`](Self::split_bounds) and [`without`](Self::without) on the current tableau: the elements
    /// outside the set in the span of A by the slots of their circuits (each has an arc from those members), and
    /// those free in A.
    fn build_columns(&mut self) {
        for column in &mut self.columns {
            column.clear();
        }
        self.free_a.clear();
        let columns = &mut self.columns;
        for (b, list) in self.out_a.iter().enumerate() {
            for &y in list {
                let i = y as usize;
                if is_zero(&self.res[0][i]) {
                    each_bit(&self.mask[0][i], |s| columns[b * 64 * R + s].push(y));
                } else {
                    self.free_a.push(y);
                }
            }
        }
        self.columns_for = self.tableau;
    }

    /// For the searches of the exchange graph on the current tableau (its columns built) with the weight split `pot`:
    /// the elements free in A by decreasing `w1` and the members by increasing `w2`, each then by element.
    fn sort_by_split(&mut self, p: &Problem<'_, R>, pot: &[i64]) {
        if self.sorted_for != self.tableau {
            let w = p.weight;
            self.free_a.sort_unstable_by_key(|&y| (Reverse(pot[y as usize]), y));
            self.by_w2.clear();
            self.by_w2.extend_from_slice(&self.members);
            self.by_w2.sort_unstable_by_key(|&z| (w[z as usize] - pot[z as usize], z));
            self.sorted_for = self.tableau;
        }
    }

    /// The cycle through the member that the last [`without`](Self::without) that answered found: its elements.
    pub fn cycle(&self) -> &[u32] {
        &self.flip
    }

    /// The weight split's `w1` by element after a successful [`split_bounds`](Self::split_bounds) (meaningful for
    /// the live elements).
    pub fn potentials(&self) -> &[i64] {
        &self.pot[..self.pot.len() - 1]
    }

    /// Keeps the last split's potentials and predecessors in `start`, for a later split to start from
    /// ([`split_bounds_from`](Self::split_bounds_from)).
    pub fn save_split(&self, start: &mut SplitStart) {
        start.pot.clone_from(&self.pot);
        start.pred.clone_from(&self.split_pred);
        start.role.clear();
        start.role.resize(self.inside.len(), 0);
        for &x in &self.members {
            start.role[x as usize] = 2;
        }
        for &y in self.out_a.iter().flatten() {
            start.role[y as usize] = 1;
        }
        start.saved = true;
    }

    /// Whether the current set flipped on `self.flip` is independent in both matroids: in each block a flipped
    /// element belongs to, the members there that stay and the outside elements that come in, inserted in a span.
    fn flipped_independent(&mut self, p: &Problem<'_, R>) -> bool {
        self.next_generation(1);
        for &e in &self.flip {
            self.seen[e as usize] = self.generation;
        }
        for m in 0..2 {
            for k in 0..self.flip.len() {
                let b = usize::from(p.block[m][self.flip[k] as usize]);
                if self.dirty[m][b] {
                    continue;
                }
                self.dirty[m][b] = true;
                let span = &mut self.spans[m][b];
                span.clear();
                let stay = self.slot_elem[m][b][..self.next_slot[m][b]]
                    .iter()
                    .filter(|&&z| self.seen[z as usize] != self.generation);
                let enter =
                    self.flip.iter().filter(|&&y| !self.inside[y as usize] && usize::from(p.block[m][y as usize]) == b);
                for &e in stay.chain(enter) {
                    if !span.insert(p.vector[m][e as usize], e) {
                        self.dirty[m].fill(false);
                        return false;
                    }
                }
            }
            self.dirty[m].fill(false);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::Rng;

    /// What a [`Problem`] of `R` words borrows: the blocks and vectors of both matroids, and the weights.
    struct Owned<const R: usize> {
        block: [Vec<u8>; 2],
        vector: [Vec<Vector<R>>; 2],
        weight: Vec<i64>,
    }

    impl<const R: usize> Owned<R> {
        fn problem(&self) -> Problem<'_, R> {
            Problem {
                block: [&self.block[0], &self.block[1]],
                vector: [&self.vector[0], &self.vector[1]],
                weight: &self.weight,
            }
        }

        /// The number of blocks in each matroid.
        fn blocks(&self) -> [usize; 2] {
            self.block.each_ref().map(|block| block.iter().map(|&b| usize::from(b) + 1).max().unwrap_or(0))
        }
    }

    /// A random problem of `n` elements with blocks of dimension at most `dim` (one word).
    fn random(rng: &mut Rng, n: usize, dim: u64) -> Owned<1> {
        let blocks = [1 + rng.below(3), 1 + rng.below(3)];
        let dims = [
            (0..blocks[0]).map(|_| 1 + rng.below(dim)).collect::<Vec<_>>(),
            (0..blocks[1]).map(|_| 1 + rng.below(dim)).collect(),
        ];
        let mut block = [Vec::new(), Vec::new()];
        let mut vector = [Vec::new(), Vec::new()];
        for _ in 0..n {
            for m in 0..2 {
                let b = rng.below(blocks[m]) as usize;
                block[m].push(b as u8);
                vector[m].push([rng.below(1 << dims[m][b])]);
            }
        }
        let mut weight: Vec<i64> = (0..n).map(|_| 1 + rng.below(30) as i64).collect();
        weight.sort_unstable();
        Owned { block, vector, weight }
    }

    /// A random problem like [`random`], some of whose elements are one-sided as the extended problem's candidates are:
    /// in one matroid such an element has a random vector in a random block, and in the other it is a coloop, its own
    /// unit vector in one of one or two *free blocks* appended after that matroid's random blocks, where no other
    /// element has that coordinate.  There are between one and `n` of them; they are the heaviest elements if `last`
    /// (as the search orders them) and otherwise at random places of the weight order, which the solver must not care
    /// about.  Returns the problem and which elements are one-sided.
    fn random_one_sided(rng: &mut Rng, n: usize, dim: u64, last: bool) -> (Owned<1>, Vec<bool>) {
        let blocks = [1 + rng.below(3) as usize, 1 + rng.below(3) as usize];
        let free = [1 + rng.below(2) as usize, 1 + rng.below(2) as usize];
        let dims: [Vec<u64>; 2] = blocks.map(|b| (0..b).map(|_| 1 + rng.below(dim)).collect());
        let count = 1 + rng.below(n as u64) as usize;
        let mut places: Vec<usize> = (0..n).collect();
        if !last {
            for i in (1..n).rev() {
                places.swap(i, rng.below(i as u64 + 1) as usize);
            }
        }
        let mut one_sided = vec![false; n];
        for &e in &places[n - count..] {
            one_sided[e] = true;
        }
        let mut block = [Vec::new(), Vec::new()];
        let mut vector = [Vec::new(), Vec::new()];
        let mut coloops = [[0u32; 2]; 2];
        for &is_one_sided in &one_sided {
            // The matroid of the element's own frame, the other being the one it is a coloop of.
            let own = rng.below(2) as usize;
            for m in 0..2 {
                if is_one_sided && m != own {
                    let at = rng.below(free[m] as u64) as usize;
                    block[m].push((blocks[m] + at) as u8);
                    vector[m].push([1 << coloops[m][at]]);
                    coloops[m][at] += 1;
                } else {
                    let b = rng.below(blocks[m] as u64) as usize;
                    block[m].push(b as u8);
                    vector[m].push([rng.below(1 << dims[m][b])]);
                }
            }
        }
        let mut weight: Vec<i64> = (0..n).map(|_| 1 + rng.below(30) as i64).collect();
        weight.sort_unstable();
        (Owned { block, vector, weight }, one_sided)
    }

    /// What the checks of a problem count, so that the tests can see that they met what they were meant to.
    #[derive(Default)]
    struct Counts {
        /// Warm starts, those whose start holds a one-sided element or whose deleted element is one, weight splits, and
        /// the loss of the smaller sets that was exactly the brute-force loss.
        warm: usize,
        warm_with_one_sided: usize,
        splits: usize,
        exact_losses: usize,
        /// Heaviest sets of all sizes, and those that hold a one-sided element (a coloop of one matroid).
        heaviest: usize,
        heaviest_with_one_sided: usize,
        /// Members of the heaviest set that the bound without a member was asked for, and those it certified; of
        /// each, the ones that were one-sided.
        asked: usize,
        certified: usize,
        asked_one_sided: usize,
        certified_one_sided: usize,
    }

    /// Whether `set` is independent in the matroid with these blocks and vectors.
    fn independent<const R: usize>(block: &[u8], vector: &[Vector<R>], set: &[u32]) -> bool {
        let blocks = set.iter().map(|&e| usize::from(block[e as usize]) + 1).max().unwrap_or(0);
        let mut spans = vec![BlockSpan::<R>::new(); blocks];
        set.iter().all(|&e| spans[usize::from(block[e as usize])].insert(vector[e as usize], e))
    }

    /// By brute force: the heaviest common independent set of at most `cap` elements, of each subset of `live`.
    fn brute(p: &Problem<'_, 1>, live: &[u32], cap: usize) -> i64 {
        let mut best = 0;
        for mask in 0u32..(1 << live.len()) {
            let set: Vec<u32> = (0..live.len()).filter(|&i| mask >> i & 1 == 1).map(|i| live[i]).collect();
            if set.len() <= cap
                && independent(p.block[0], p.vector[0], &set)
                && independent(p.block[1], p.vector[1], &set)
            {
                best = best.max(set.iter().map(|&e| p.weight[e as usize]).sum());
            }
        }
        best
    }

    /// The checks of the intersection on one problem, against brute force: the heaviest set under every size limit,
    /// from the empty start and from every set a solve reported (dominant), with a member deleted; and the split's
    /// bound without each member.  `one_sided` says which elements are one-sided (none, for [`random`]'s problems); it
    /// only decides what `counts` counts, the checks being the same for all.
    fn check_warm_starts_and_split(
        rng: &mut Rng,
        owned: &Owned<1>,
        one_sided: &[bool],
        trial: usize,
        counts: &mut Counts,
    ) {
        let n = owned.weight.len();
        let p = owned.problem();
        let ([ba, bb], [va, vb]) = (&owned.block, &owned.vector);
        let mut meet = Meet::<1>::new(n, owned.blocks());
        let live: Vec<u32> = (0..n as u32).collect();
        let holds_one_sided = |set: &[u32]| set.iter().any(|&e| one_sided[e as usize]);
        for cap in 0..=n {
            let mut reported = Vec::new();
            meet.solve(&p, &live, &[], cap, i64::MAX, |set| reported.push(set.to_vec()));
            assert_eq!(meet.value, brute(&p, &live, cap), "trial {trial} cap {cap}");
            let chosen: Vec<u32> = live.iter().copied().filter(|&e| meet.contains(e)).collect();
            assert!(chosen.len() <= cap && independent(ba, va, &chosen) && independent(bb, vb, &chosen));
            assert_eq!(chosen.iter().map(|&e| p.weight[e as usize]).sum::<i64>(), meet.value);
            let mut last = reported.last().cloned().unwrap_or_default();
            last.sort_unstable();
            assert_eq!(last, chosen, "trial {trial}: the last set reported is the heaviest");
            if cap < n {
                continue;
            }
            counts.heaviest += 1;
            counts.heaviest_with_one_sided += usize::from(holds_one_sided(&chosen));
            // The split, with no size limit.
            let value = meet.value;
            let mut bounds = Vec::new();
            if meet.settled {
                assert!(meet.split_bounds(&p, &live, |x, b| bounds.push((x, b))), "trial {trial}: no split");
                counts.splits += 1;
            }
            for (x, b) in bounds {
                let rest: Vec<u32> = live.iter().copied().filter(|&e| e != x).collect();
                assert!(b >= brute(&p, &rest, n), "trial {trial}: the split's bound without {x}");
                assert!(b <= value, "trial {trial}: the split's bound without {x} above the heaviest set's weight");
            }
            // Warm starts: a reported set without a deleted element that it does not hold.
            for set in &reported {
                for d in 0..n as u32 {
                    if set.contains(&d) {
                        continue;
                    }
                    let rest: Vec<u32> = live.iter().copied().filter(|&e| e != d).collect();
                    meet.solve(&p, &rest, set, n, i64::MAX, |_| {});
                    let truth = brute(&p, &rest, n);
                    assert_eq!(meet.value, truth, "trial {trial}: from {set:?} without {d}");
                    // Stopped early: at least the stop, and the heaviest set is at least as heavy.
                    let stop = 1 + rng.below(truth as u64 + 5) as i64;
                    meet.solve(&p, &rest, set, n, stop, |_| {});
                    assert!(meet.value <= truth && (meet.stopped == (meet.value >= stop)), "trial {trial}");
                    assert!(meet.stopped || meet.value == truth, "trial {trial}: stop {stop}");
                    counts.warm += 1;
                    counts.warm_with_one_sided += usize::from(holds_one_sided(set) || one_sided[d as usize]);
                }
            }
        }
    }

    /// The checks of the loss of the smaller sets ([`Meet::smaller_loss`], against the heaviest set one smaller, by
    /// brute force) and of the larger ones ([`Meet::gap`]), and of the heaviest set without each member of the
    /// heaviest set read off its exchange graph where they certify it ([`Meet::without`]), under every size limit from
    /// the heaviest set's size up, on one problem, against brute force.  `one_sided` is as for
    /// [`check_warm_starts_and_split`].
    fn check_heaviest_set_without_a_member(owned: &Owned<1>, one_sided: &[bool], trial: usize, counts: &mut Counts) {
        let n = owned.weight.len();
        let p = owned.problem();
        let mut meet = Meet::<1>::new(n, owned.blocks());
        let live: Vec<u32> = (0..n as u32).collect();
        meet.solve(&p, &live, &[], n, i64::MAX, |_| {});
        let members: Vec<u32> = live.iter().copied().filter(|&e| meet.contains(e)).collect();
        let size = members.len();
        assert!(meet.settled && meet.split_bounds(&p, &live, |_, _| {}), "trial {trial}: no split");
        counts.heaviest += 1;
        counts.heaviest_with_one_sided += usize::from(members.iter().any(|&e| one_sided[e as usize]));
        let pot = meet.potentials().to_vec();
        let loss = meet.smaller_loss(&p, &pot);
        if size == 0 {
            assert_eq!(loss, i64::MAX);
            return;
        }
        let smaller = brute(&p, &live, size - 1);
        assert!(loss <= meet.value - smaller, "trial {trial}: the loss of the smaller sets");
        counts.exact_losses += usize::from(loss == meet.value - smaller);
        // The heaviest common independent set of exactly one more element, if there is one.
        let larger = (0u32..1 << n)
            .filter(|mask| mask.count_ones() as usize == size + 1)
            .map(|mask| (0..n as u32).filter(|&i| mask >> i & 1 == 1).collect::<Vec<u32>>())
            .filter(|set| independent(p.block[0], p.vector[0], set) && independent(p.block[1], p.vector[1], set))
            .map(|set| set.iter().map(|&e| p.weight[e as usize]).sum::<i64>())
            .max();
        let gap = meet.gap(&p);
        assert_eq!(gap, larger.map_or(i64::MAX, |w| meet.value - w), "trial {trial}: the gap");
        for cap in size..=n {
            let limit = if cap > size { loss.min(gap) } else { loss };
            for &x in &members {
                counts.asked += 1;
                counts.asked_one_sided += usize::from(one_sided[x as usize]);
                if let Some(value) = meet.without(&p, &pot, x, limit, &[]) {
                    counts.certified += 1;
                    counts.certified_one_sided += usize::from(one_sided[x as usize]);
                    let rest: Vec<u32> = live.iter().copied().filter(|&e| e != x).collect();
                    assert_eq!(value, brute(&p, &rest, cap), "trial {trial}: without {x}, at most {cap}");
                }
            }
        }
    }

    /// Against brute force: the heaviest set under every size limit, from the empty start and from every set
    /// a solve reported (dominant), with a member deleted; and the split's bound without each member.
    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is some_intersection_warm_starts_and_splits_against_brute_force"
    )]
    fn intersection_warm_starts_and_split_against_brute_force() {
        warm_starts_and_split(3000, 100);
    }

    /// [`intersection_warm_starts_and_split_against_brute_force`] for the first 100 problems.
    #[test]
    fn some_intersection_warm_starts_and_splits_against_brute_force() {
        warm_starts_and_split(100, 2);
    }

    /// The first `trials` problems of [`intersection_warm_starts_and_split_against_brute_force`], which must reach
    /// `percent` percent of the counts that it asks.
    fn warm_starts_and_split(trials: usize, percent: usize) {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let mut counts = Counts::default();
        for trial in 0..trials {
            let n = 1 + rng.below(11) as usize;
            let owned = random(&mut rng, n, 5);
            check_warm_starts_and_split(&mut rng, &owned, &vec![false; n], trial, &mut counts);
        }
        let Counts { warm, splits, .. } = counts;
        let at_least = |count: usize, most: usize| count * 100 >= most * percent;
        assert!(at_least(warm, 10_000) && at_least(splits, 2_000), "{warm} warm starts, {splits} splits");
    }

    /// The same checks on problems with one-sided elements, whose coloops are in free blocks after the random blocks
    /// of the other matroid (see [`random_one_sided`]), at the end of the weight order and at random places in it:
    /// the solver, which knows blocks and not frames, must find the heaviest sets, split and warm-start as it does
    /// without them.  Many of the heaviest sets must hold coloops, and many of the warm starts start from, or delete,
    /// a one-sided element.
    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is some_intersection_warm_starts_and_splits_with_free_blocks_against_brute_force"
    )]
    fn intersection_warm_starts_and_split_with_free_blocks_against_brute_force() {
        warm_starts_and_split_with_free_blocks(3000, 100);
    }

    /// [`intersection_warm_starts_and_split_with_free_blocks_against_brute_force`] for the first 100 problems.
    #[test]
    fn some_intersection_warm_starts_and_splits_with_free_blocks_against_brute_force() {
        warm_starts_and_split_with_free_blocks(100, 2);
    }

    /// The first `trials` problems of [`intersection_warm_starts_and_split_with_free_blocks_against_brute_force`],
    /// which must reach `percent` percent of the counts that it asks.
    fn warm_starts_and_split_with_free_blocks(trials: usize, percent: usize) {
        let mut rng = Rng(0x51DE_D00D_0F5E_ED01);
        let mut counts = Counts::default();
        for trial in 0..trials {
            let n = 1 + rng.below(11) as usize;
            let (owned, one_sided) = random_one_sided(&mut rng, n, 5, trial % 2 == 0);
            check_warm_starts_and_split(&mut rng, &owned, &one_sided, trial, &mut counts);
        }
        let Counts { warm, splits, heaviest, heaviest_with_one_sided, warm_with_one_sided, .. } = counts;
        println!("{heaviest} problems, {heaviest_with_one_sided} with a one-sided element in the heaviest set");
        println!("{warm} warm starts, {warm_with_one_sided} from or without a one-sided element, {splits} splits");
        let at_least = |count: usize, most: usize| count * 100 >= most * percent;
        assert!(at_least(warm, 10_000) && at_least(splits, 2_000), "{warm} warm starts, {splits} splits");
        assert!(heaviest_with_one_sided > heaviest / 2, "{heaviest_with_one_sided} of {heaviest} heaviest sets");
        assert!(warm_with_one_sided > warm / 2, "{warm_with_one_sided} of {warm} warm starts");
    }

    /// The loss of the smaller sets ([`Meet::smaller_loss`], against the heaviest set one smaller, by brute force)
    /// and of the larger ones ([`Meet::gap`]), and the heaviest set without each member of the heaviest set read off
    /// its exchange graph where they certify it ([`Meet::without`]), under every size limit from the heaviest set's
    /// size up, against brute force.
    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is the_heaviest_set_without_a_member_of_some_problems_against_brute_force"
    )]
    fn heaviest_set_without_a_member_against_brute_force() {
        heaviest_set_without_a_member(4000, 100);
    }

    /// [`heaviest_set_without_a_member_against_brute_force`] for the first 120 problems.
    #[test]
    fn the_heaviest_set_without_a_member_of_some_problems_against_brute_force() {
        heaviest_set_without_a_member(120, 2);
    }

    /// The first `trials` problems of [`heaviest_set_without_a_member_against_brute_force`], which must reach
    /// `percent` percent of the counts that it asks.
    fn heaviest_set_without_a_member(trials: usize, percent: usize) {
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        let mut counts = Counts::default();
        for trial in 0..trials {
            let n = 2 + rng.below(10) as usize;
            let owned = random(&mut rng, n, 4);
            check_heaviest_set_without_a_member(&owned, &vec![false; n], trial, &mut counts);
        }
        let Counts { exact_losses, asked, certified, .. } = counts;
        let at_least = |count: usize, most: usize| count * 100 >= most * percent;
        assert!(at_least(exact_losses, 3000), "{exact_losses} exact losses");
        assert!(at_least(asked, 10_000) && certified > asked / 4, "{certified} of {asked} certified");
    }

    /// The same checks on problems with one-sided elements, as for the warm starts above.  The heaviest sets must hold
    /// coloops, and the bounds without a member must be asked for, and certified for, one-sided members.
    #[test]
    #[cfg_attr(
        debug_assertions,
        ignore = "long: the quick check is the_heaviest_set_without_a_member_of_some_problems_with_free_blocks_against_brute_force"
    )]
    fn heaviest_set_without_a_member_with_free_blocks_against_brute_force() {
        heaviest_set_without_a_member_with_free_blocks(4000, 100);
    }

    /// [`heaviest_set_without_a_member_with_free_blocks_against_brute_force`] for the first 120 problems.
    #[test]
    fn the_heaviest_set_without_a_member_of_some_problems_with_free_blocks_against_brute_force() {
        heaviest_set_without_a_member_with_free_blocks(120, 2);
    }

    /// The first `trials` problems of [`heaviest_set_without_a_member_with_free_blocks_against_brute_force`], which must
    /// reach `percent` percent of the counts that it asks.
    fn heaviest_set_without_a_member_with_free_blocks(trials: usize, percent: usize) {
        let mut rng = Rng(0x7A3C_91E5_B0D2_4F68);
        let mut counts = Counts::default();
        for trial in 0..trials {
            let n = 2 + rng.below(10) as usize;
            let (owned, one_sided) = random_one_sided(&mut rng, n, 4, trial % 2 == 0);
            check_heaviest_set_without_a_member(&owned, &one_sided, trial, &mut counts);
        }
        let Counts { heaviest, heaviest_with_one_sided, exact_losses, asked, certified, .. } = counts;
        let (asked_one_sided, certified_one_sided) = (counts.asked_one_sided, counts.certified_one_sided);
        println!("{heaviest} problems, {heaviest_with_one_sided} with a one-sided element in the heaviest set");
        println!("{certified} of {asked} certified, {certified_one_sided} of {asked_one_sided} for one-sided members");
        let at_least = |count: usize, most: usize| count * 100 >= most * percent;
        assert!(heaviest_with_one_sided > heaviest / 2, "{heaviest_with_one_sided} of {heaviest} heaviest sets");
        assert!(at_least(exact_losses, 3000), "{exact_losses} exact losses");
        assert!(at_least(asked, 10_000) && certified > asked / 6, "{certified} of {asked} certified");
        assert!(asked_one_sided > asked / 4, "{asked_one_sided} of {asked} asked about are one-sided");
        assert!(certified_one_sided > asked_one_sided / 8, "{certified_one_sided} of {asked_one_sided} certified");
    }

    /// What a workspace computes from one problem, as numbers: the solve's value, whether it split, the split's bounds
    /// and potentials, the loss of the smaller sets, and for each member of the heaviest set whether a cycle through it
    /// is within each of a few limits and the bound without it read off the exchange graph.
    fn everything(meet: &mut Meet<1>, owned: &Owned<1>) -> Vec<i64> {
        let p = owned.problem();
        let n = owned.weight.len();
        let live: Vec<u32> = (0..n as u32).collect();
        meet.solve(&p, &live, &[], n, i64::MAX, |_| {});
        let mut bounds = Vec::new();
        let split = meet.settled && meet.split_bounds(&p, &live, |x, b| bounds.push((x, b)));
        let mut out = vec![meet.value, i64::from(split)];
        if !split {
            return out;
        }
        out.extend(bounds.iter().flat_map(|&(x, b)| [i64::from(x), b]));
        let pot = meet.potentials().to_vec();
        out.extend_from_slice(&pot);
        let loss = meet.smaller_loss(&p, &pot);
        out.push(loss);
        let members: Vec<u32> = live.iter().copied().filter(|&e| meet.contains(e)).collect();
        for &x in &members {
            for limit in [0, 3, 10, 40] {
                out.push(i64::from(meet.has_cycle_within(&p, &pot, x, limit)));
            }
            out.push(meet.without(&p, &pot, x, loss.min(1000), &[]).unwrap_or(-1));
        }
        out
    }

    /// The searches compare their stamps for equality only, with a 32-bit counter that a long search wraps (tens of
    /// stamps a step: past some 90 M steps of one worker at 3441).  A workspace whose stamps start just below the wrap,
    /// so that it wraps in a solve, a split, or a search of the exchange graph, computes exactly what one whose stamps
    /// start at 0 does ([`Meet::next_generation`] clears the stamps before the wrap).  Before that, the counter came
    /// back to 0, which every stamp never written holds, and then to stamps of 2^32 stamps before.
    #[test]
    fn the_stamps_hold_across_the_wrap_of_their_counter() {
        let mut rng = Rng(0x57A3_9E11_0000_0001);
        let mut wrapped = 0;
        for trial in 0..400 {
            let n = 2 + rng.below(10) as usize;
            let owned = random(&mut rng, n, 4);
            let fresh = everything(&mut Meet::<1>::new(n, owned.blocks()), &owned);
            for below in [1u32, 2, 3, 4, 7, 20, 60] {
                let mut meet = Meet::<1>::new(n, owned.blocks());
                meet.set_generation(u32::MAX - below + 1);
                assert_eq!(everything(&mut meet, &owned), fresh, "trial {trial}: stamps from {below} below the wrap");
                wrapped += usize::from(meet.generation() < u32::MAX / 2);
            }
        }
        assert!(wrapped > 1000, "{wrapped} workspaces wrapped");
    }

    /// A random word that is not a linear function of the generator's state over GF(2).  The generator is xorshift, so
    /// every vector made of its outputs lies in a space of at most 64 dimensions (all are linear in the 64 bits of its
    /// state): random vectors of a larger dimension would never have a rank above 64.  The finalizer of splitmix64,
    /// whose multiplications are not linear, breaks that.
    fn scrambled(rng: &mut Rng) -> u64 {
        let mut z = rng.next().wrapping_mul(0x9E37_79B9_7F4A_7C15);
        z = (z ^ z >> 30).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ z >> 27).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ z >> 31
    }

    /// A problem of `n` elements in which the matroid `free` is free (every element has a unit vector of its own, the
    /// elements spread over two or more blocks of dimension at most `64·R`) and the other is one block of random
    /// vectors of dimension `dim`, at most `64·R`.  The weights are positive and sorted, with ties.
    fn one_matroid_free<const R: usize>(rng: &mut Rng, n: usize, dim: usize, free: usize) -> Owned<R> {
        let blocks = n.div_ceil(64 * R).max(2);
        let mut block = [Vec::new(), Vec::new()];
        let mut vector = [Vec::new(), Vec::new()];
        for e in 0..n {
            let mut unit = [0; R];
            unit[e / blocks / 64] |= 1 << (e / blocks % 64);
            block[free].push((e % blocks) as u8);
            vector[free].push(unit);
            let mut random = [0; R];
            for (i, word) in random.iter_mut().enumerate() {
                let bits = dim.saturating_sub(64 * i).min(64) as u32;
                *word = scrambled(rng).checked_shr(64 - bits).unwrap_or(0);
            }
            block[1 - free].push(0);
            vector[1 - free].push(random);
        }
        let mut weight: Vec<i64> = (0..n).map(|_| 1 + rng.below(40) as i64).collect();
        weight.sort_unstable();
        Owned { block, vector, weight }
    }

    /// The matroid greedy algorithm on matroid `m` of `owned`: the elements of `live` (ascending) by decreasing weight,
    /// each kept when independent of those kept so far, `skip` left out.  Its first `cap` picks are the heaviest
    /// independent set of at most `cap` elements, whatever `cap` is, since the weights are positive.
    fn greedy<const R: usize>(owned: &Owned<R>, m: usize, live: &[u32], skip: Option<u32>) -> Vec<u32> {
        let mut spans = vec![BlockSpan::<R>::new(); owned.blocks()[m]];
        let keep = |&e: &u32| {
            Some(e) != skip && spans[usize::from(owned.block[m][e as usize])].insert(owned.vector[m][e as usize], e)
        };
        live.iter().rev().copied().filter(keep).collect()
    }

    /// The weight of the first `cap` of `picks`.
    fn weigh<const R: usize>(owned: &Owned<R>, picks: &[u32], cap: usize) -> i64 {
        picks.iter().take(cap).map(|&e| owned.weight[e as usize]).sum()
    }

    /// Blocks of more than 255 members, which the `u8` slots of the tableau could not number (255 was the mark of no
    /// slot, and slot 256 would have wrapped to 0), at `R = 5`, against the greedy algorithm.  One matroid is free
    /// (every element has a unit vector of its own, over a few blocks) and the other is one block of random vectors of
    /// a dimension from 260 to 320, so every set independent in the second is common independent, and the heaviest
    /// common independent set of at most `cap` members is the heaviest independent set of the second, which the
    /// greedy algorithm finds.  Under four limits, the solve's value and its set (independent in both, within the
    /// limit, of the weight asked) must match; with no limit, the weight split's bound without a member, and the
    /// heaviest set without a member read off the exchange graph where it certifies one, must match the greedy
    /// algorithm's set without that member.  Then with the roles of the matroids swapped.
    #[test]
    fn blocks_of_more_than_255_members_against_the_greedy_algorithm() {
        const R: usize = 5;
        let mut rng = Rng(0x1F2E_3D4C_5B6A_7988);
        let mut widest = [0; 2];
        let (mut split_checked, mut asked, mut certified) = (0, 0, 0);
        for (random, largest) in widest.iter_mut().enumerate() {
            for _ in 0..8 {
                let dim = 260 + rng.below(61) as usize;
                let n = dim + 20 + rng.below(41) as usize;
                let owned = one_matroid_free::<R>(&mut rng, n, dim, 1 - random);
                let p = owned.problem();
                let live: Vec<u32> = (0..n as u32).collect();
                assert!(independent(p.block[1 - random], p.vector[1 - random], &live), "the free matroid is not free");
                let picks = greedy(&owned, random, &live, None);
                assert!(picks.len() > 255, "the random matroid has rank {}", picks.len());
                let mut meet = Meet::<R>::new(n, owned.blocks());
                for cap in [n, dim, dim / 2, 7] {
                    meet.solve(&p, &live, &[], cap, i64::MAX, |_| {});
                    assert_eq!(meet.value, weigh(&owned, &picks, cap), "n={n} dim={dim} cap={cap}");
                    let chosen: Vec<u32> = live.iter().copied().filter(|&e| meet.contains(e)).collect();
                    assert_eq!(chosen.len(), picks.len().min(cap), "n={n} dim={dim} cap={cap}");
                    assert!(independent(p.block[0], p.vector[0], &chosen), "cap={cap}: dependent in A");
                    assert!(independent(p.block[1], p.vector[1], &chosen), "cap={cap}: dependent in B");
                    assert_eq!(weigh(&owned, &chosen, cap), meet.value, "cap={cap}");
                    *largest = (*largest).max(chosen.len());
                    if cap < n {
                        continue;
                    }

                    // No limit: the heaviest set of all sizes, its split, and the sets without a few of its members.
                    let (value, size) = (meet.value, chosen.len());
                    assert!(meet.settled, "n={n} dim={dim}: the solve without a limit did not settle");
                    let mut bounds = Vec::new();
                    assert!(meet.split_bounds(&p, &live, |x, b| bounds.push((x, b))), "n={n}: no split");
                    assert_eq!(bounds.len(), size);
                    let pot = meet.potentials().to_vec();
                    let (loss, gap) = (meet.smaller_loss(&p, &pot), meet.gap(&p));
                    let sample: Vec<u32> = chosen.iter().copied().filter(|_| rng.chance(15)).collect();
                    let rests: Vec<Vec<u32>> = sample.iter().map(|&x| greedy(&owned, random, &live, Some(x))).collect();
                    for &(x, bound) in &bounds {
                        assert!(bound <= value, "the split's bound without {x} is above the heaviest set's weight");
                        if let Some(i) = sample.iter().position(|&y| y == x) {
                            assert!(bound >= weigh(&owned, &rests[i], n), "the split's bound without {x}");
                            split_checked += 1;
                        }
                    }
                    for (&x, rest) in sample.iter().zip(&rests) {
                        for at_most in [size, n] {
                            let limit = if at_most > size { loss.min(gap) } else { loss };
                            asked += 1;
                            if let Some(v) = meet.without(&p, &pot, x, limit, &[]) {
                                certified += 1;
                                assert_eq!(v, weigh(&owned, rest, at_most), "without {x}, at most {at_most}");
                            }
                        }
                    }
                }
            }
        }
        assert!(widest.iter().all(|&w| w > 255), "the largest sets: {widest:?}");
        assert!(split_checked > 20 && asked > 40 && certified > asked / 10, "{split_checked} {certified} of {asked}");
    }
}
