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

/// The weight split's bound on the common independent sets without a member of the heaviest set (see the module docs,
/// "The weight split"): `value`, the heaviest set's weight, less what each matroid's heaviest independent set loses
/// without the member, its weight there (`w1`, `w2`) less that of its heaviest replacement there (`repl`, 0 for none),
/// in each matroid where it weighs more than 0.
fn bound_without(value: i64, (w1, w2): (i64, i64), repl: [i64; 2]) -> i64 {
    let mut bound = value;
    if w1 > 0 {
        bound -= w1 - repl[0];
    }
    if w2 > 0 {
        bound -= w2 - repl[1];
    }
    bound
}

/// Whether the live element `i` is free in matroid `m` of the workspace `meet`'s tableau (its residue is not zero):
/// the flag kept with the residue, checked against it in debug builds where the tableau keeps its residues (a solve's;
/// see `residues` in [`Meet`]) and the element is live there (a solve writes the residues of its live elements alone).
/// A macro, so that it reads the fields alone while others are borrowed.
macro_rules! free {
    ($meet:expr, $m:expr, $i:expr) => {{
        let (m, i): (usize, usize) = ($m, $i);
        #[cfg(debug_assertions)]
        assert!(
            !$meet.residues || $meet.listed[i] != $meet.tableau || $meet.free[m][i] == !is_zero(&$meet.res[m][i]),
            "the flag of element {} in matroid {}",
            i,
            m
        );
        $meet.free[m][i]
    }};
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

    /// Adds the element `e` with the vector `v`, in the next slot; false (and nothing added) if `v` is in the span.
    #[inline]
    fn insert(&mut self, v: Vector<R>, e: u32) -> bool {
        let added = self.insert_as(v, self.elem.len());
        if added {
            self.elem.push(e);
        }
        added
    }

    /// Adds the vector `v` as slot `slot` (its bit in the masks); false (and nothing added) if `v` is in the span.
    #[inline]
    fn insert_as(&mut self, v: Vector<R>, slot: usize) -> bool {
        let (r, mut c) = self.reduce(v);
        let Some(p) = lowest(&r) else { return false };
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
/// The most tableaux a workspace's lineage holds ([`Meet::repair_from`]): a split found on an older one starts the next
/// from scratch.
const LINEAGE: usize = 64;
/// The heap entry of [`Meet::without`] that stands for the next element free in A.
const FREE_A: u32 = u32::MAX - 1;
/// The vertices of [`Meet::without_any_size`]'s search that stand for no element: one outside the set that is free in
/// both matroids, and a member that is in no circuit of either, both of weight 0.
const OUT_DUMMY: u32 = u32::MAX - 2;
const IN_DUMMY: u32 = u32::MAX - 3;
/// The heap entry of [`Meet::has_cycle_within`]'s search that stands for the next member reached through the hub (that
/// search has no vertex [`IN_DUMMY`], whose code it takes).
const HUB_MEMBER: u32 = IN_DUMMY;

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
/// others (`mask[e]`, through the member in each slot), and the lists of members and of the elements outside the
/// set.  These depend on the set alone, not on how the tableau was made, but for the numbers of the slots, which
/// only name the members: the searches order their vertices by distance, arcs and element, never by slot, and what
/// else is read off such a tableau (the split, the gap) is unique; only the split's predecessors, which choose where
/// a later split starts, can depend on that order (a solve, whose paths do, builds its tableau anew).  So a tableau
/// that reads as the one a solve would build for a set follows from the tableau of a related set, at a cost linear
/// in the elements instead of a pivot per member ([`derive`](Self::derive)):
///
/// * Exchanging a member `x` for an element `y` in the span whose circuit holds `x` keeps the span, so it changes
///   no residue and is the pivot of [`exchange`](Self::exchange).  Two independent sets of a block with the same
///   span are a pivot apart for every element of one that is not in the other (the circuit of such a `y` must hold
///   a member that is not in the other set, which would be dependent otherwise), so the pivots reach the other set.
/// * Where the span of a block changes (an element that enters is free, or the numbers that enter and leave
///   differ), an element whose circuit holds no member that leaves keeps it: the circuit lies in the new set.  The
///   others outside the new set, free ones included, are reduced by its members, which costs a reduction each
///   instead of a pivot per member over the whole block.  (A residue outside a solve is read only for being zero,
///   so a derived tableau keeps only whether each element is free, `free`, and not its residue.)
/// * Contracting by members `C` of the set `S` (promoting them): an element is in the span of `S − C` in the
///   contraction exactly when it is in the span of `S`, and its circuit there is its circuit with respect to `S`
///   less `C` (the vectors reduced modulo those of `C`).  So the circuits carry over without the slots of `C`.
/// * Deleting elements that are not members changes no other element's circuit.
/// * A member gone frees its slot, whose bit leaves the circuits, and one that enters takes a free slot; the others
///   keep theirs.
///
/// The search derives a child's tableau from its parent's (inherited, the members it promotes contracted; or
/// certified, flipped along a cycle and contracted), and a node's without the member it branched on from its own,
/// flipped along the certified cycle, from a copy ([`Snapshot`]) kept while the child's subtree uses the workspace.
pub struct Meet<const R: usize> {
    mask: [Vec<Vector<R>>; 2],
    res: [Vec<Vector<R>>; 2],
    /// Whether `res` holds the tableau's residues: after a solve, whose pivots need them, and not after a
    /// [`derive`](Self::derive), whose tableau keeps only whether each element is free (`free`), all that is read of
    /// a residue outside a solve.
    residues: bool,
    /// In debug builds, the tableau whose lists last held each element ([`lists`](Self::lists)), for the check of
    /// `free!`: the residue and flag of an element no longer live are stale.
    #[cfg(debug_assertions)]
    listed: Vec<u64>,
    /// Whether each live element's residue is not zero (it is free: `I + e` is independent there), kept with `res`
    /// wherever a solve writes a residue, and alone in a derived tableau, so that the searches read a byte instead of
    /// a vector (the macro `free!`).
    free: [Vec<bool>; 2],
    /// The slot of each member of the tableau's set, [`NO_SLOT`] for the others.
    slot: [Vec<u16>; 2],
    /// Per block: the member in each slot ([`NONE`] for a slot freed in a derived tableau), and the slots in use.
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
    /// Bellman–Ford's elements outside the set by their block of `A`, as `out_a` lists them, apart: those free in `A`
    /// and those in its span (see [`shortest_path`](Self::shortest_path)).
    free_out: Vec<Vec<u32>>,
    span_out: Vec<Vec<u32>>,
    /// In tests: every [`shortest_path`](Self::shortest_path) checked against a scan of every outside element of a
    /// block in each round ([`shortest_path_by_scan`](Self::shortest_path_by_scan)), the searches checked, and the
    /// blocks where only the elements free in `A`, and only those in its span, were visited.
    #[cfg(test)]
    pub check_paths: bool,
    #[cfg(test)]
    pub paths_checked: u64,
    #[cfg(test)]
    pub lists_visited: [u64; 2],
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
    /// [`split_bound_among`](Self::split_bound_among)'s elements of positive weight, with their weights.
    order: Vec<(i64, u32)>,
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
    /// [`flipped_independent`](Self::flipped_independent)'s rows: the circuits of the elements entering a block.
    entering_rows: Vec<Vector<R>>,
    /// The element that closed the cycle the last [`has_cycle_within`](Self::has_cycle_within) found (`NONE` for
    /// none).
    found: u32,
    /// [`has_cycle_within`](Self::has_cycle_within)'s lower bounds on the length of a path from an element outside the
    /// set to `x` (its least arc out), each found when a search first offers the element on the tableau `bounds_for`
    /// (then its entry in `bounded_at` is `bounds_stamp`); and the least `w2` of a member there.
    path_bound: Vec<i64>,
    bounded_at: Vec<u32>,
    bounds_stamp: u32,
    bounds_for: u64,
    least_w2: i64,
    /// The predecessors of [`OUT_DUMMY`] and [`IN_DUMMY`] in the last search of
    /// [`without_any_size`](Self::without_any_size).
    dummy_back: [u32; 2],
    /// Counters of [`without`](Self::without): calls, and those it certified.
    pub withouts: u64,
    pub withouts_exact: u64,
    /// Per matroid, the tableau at which each element last changed its circuit or whether it is free there, or its
    /// side of the set (0 for none since the tableau was restored from a copy), and the tableaux the workspace's
    /// derives from by [`derive`](Self::derive), oldest first and itself last, for which those marks hold: a split
    /// found on one of them starts the next from what changed since ([`repair_from`](Self::repair_from)).
    changed_at: [Vec<u64>; 2],
    /// [`repair_from`](Self::repair_from)'s elements changed in B.
    relaxed: Vec<u32>,
    lineage: Vec<u64>,
    /// [`derive`](Self::derive)'s scratch: marks by element, the elements that enter and leave a block, the live
    /// elements the tableau derived from did not have, and per matroid and block those whose circuits it does not
    /// give where the span changes.
    mark: Vec<u8>,
    entering: Vec<u32>,
    incoming: Vec<u32>,
    leaving: Vec<u32>,
    fresh: Vec<u32>,
    /// The elements of the tableau derived from that are no longer live, outside its set and in the span of A there
    /// (in some of its columns).
    dropped: Vec<u32>,
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
/// vertices [`IN_DUMMY`], [`OUT_DUMMY`] and [`FREE_A`], in that order, after every element at the same distance).
/// When the distances stay below a bound under 2^49 and the elements fit 14 bits with the three codes above them, the
/// entries are keys of a [`RadixHeap`], the distance above the vertex (`FREE_A` the largest vertex), which takes the
/// least key of the least distance: a search pushes no entry at a distance below the last it took.  Where it pushes
/// no entry below the last it took, ties included (every arc adds an arc to the packed distance, and the hubs'
/// entries are later still), the keys come out in the same order as from a binary heap, which takes the other
/// searches.  The search of [`Meet::has_cycle_within`], whose distances leave the arcs out, can push an entry at the
/// distance of the last it took and of a lower vertex, which comes out before the higher ones of that distance.
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
        self.packed = (0..1 << 49).contains(&bound) && (n as u64) < Self::FREE - 2;
        self.radix.clear();
        self.heap.clear();
    }

    #[inline]
    fn push(&mut self, d: Dist, v: u32) {
        if self.packed {
            let code = match v {
                FREE_A => Self::FREE,
                OUT_DUMMY => Self::FREE - 1,
                IN_DUMMY => Self::FREE - 2,
                _ => u64::from(v),
            };
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
                let v = match code {
                    Self::FREE => FREE_A,
                    c if c == Self::FREE - 1 => OUT_DUMMY,
                    c if c == Self::FREE - 2 => IN_DUMMY,
                    c => c as u32,
                };
                ((k >> Self::BITS) as Dist, v)
            })
        } else {
            self.heap.pop().map(|Reverse(entry)| entry)
        }
    }
}

/// A monotone priority queue of keys (Ahuja, Mehlhorn, Orlin and Tarjan's radix heap), each a distance above a code
/// of [`Queue::BITS`] bits: every key pushed must have a distance at least the last popped key's, and a pop takes the
/// least key.  A key sits in the bucket of the highest bit in which its distance differs from the last key popped's;
/// bucket 0 holds the keys of that same distance, in decreasing order, so that the keys of one distance are sorted
/// once instead of spread over the buckets of their codes' bits, and a key pushed there below the last one popped
/// comes out next.  A pop with bucket 0 empty takes the least key of the first bucket that is not, and spreads that
/// bucket over the ones below.
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

    /// The bucket of `key`.
    #[inline]
    fn bucket(&self, key: u64) -> usize {
        (64 - ((key ^ self.last) >> Queue::BITS).leading_zeros()) as usize
    }

    #[inline]
    fn push(&mut self, key: u64) {
        debug_assert!(
            key >> Queue::BITS >= self.last >> Queue::BITS,
            "a key at a distance below the last one popped's"
        );
        let b = self.bucket(key);
        if b == 0 {
            let zero = &mut self.buckets[0];
            let at = zero.partition_point(|&k| k > key);
            zero.insert(at, key);
        } else {
            self.buckets[b].push(key);
        }
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
                let b = self.bucket(k);
                self.buckets[b].push(k);
                self.full |= 1 << b;
            }
            keys.clear();
            self.buckets[i] = keys;
            self.buckets[0].sort_unstable_by(|a, b| b.cmp(a));
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
    /// By element: 0 if it was not live, 1 if outside the set, 2 if a member; and the number of members.
    role: Vec<u8>,
    members: usize,
    saved: bool,
    /// The tableau the split was found on, and the forest of its predecessors ([`Tree`]) once `built`, when first
    /// asked for.
    tableau: u64,
    tree: std::cell::RefCell<Tree>,
    built: std::cell::Cell<bool>,
}

impl SplitStart {
    /// The forest of the split's predecessors ([`Tree`]), made when first asked for.
    fn tree(&self) -> std::cell::Ref<'_, Tree> {
        if !self.built.get() {
            self.tree.borrow_mut().build(&self.pred, &self.role);
            self.built.set(true);
        }
        self.tree.borrow()
    }
}

/// The predecessors of a split ([`SplitStart`]) as a forest over its live elements and the zero node (the last vertex):
/// the children of each vertex, those whose predecessor it is, from `first[v]` to `first[v + 1]` in `children`.
///
/// They form a forest, its roots the vertices with no predecessor.  Throughout a split's Bellman–Ford, each vertex `v`
/// with a predecessor `u` has a potential of at least `u`'s plus the length of the arc `u → v`: an equality when the
/// predecessor is set (from a relaxation or a pull, or taken over with both potentials from a start, where it held with
/// equality since that split had converged), and potentials only fall.  So when a relaxation closed a cycle of
/// predecessors, it lowered its vertex strictly below its new predecessor's potential plus the arc, and adding the
/// other arcs of the cycle, each at most the difference of its ends' potentials, makes the cycle's length negative
/// (Cormen et al., *Introduction to Algorithms*, Lemma 24.16); and a split is kept only where Bellman–Ford converged,
/// which no negative cycle allows.
#[derive(Clone, Default)]
struct Tree {
    first: Vec<u32>,
    children: Vec<u32>,
    /// Scratch: where the next child of each vertex goes.
    at: Vec<u32>,
}

impl Tree {
    /// The forest of the predecessors `pred` of the vertices that `role` marks live and of the zero node, by a
    /// counting sort.
    fn build(&mut self, pred: &[u32], role: &[u8]) {
        let count = pred.len();
        let zero = count - 1;
        let parent = |v: usize| if v == zero || role[v] != 0 { pred[v] } else { NONE };
        self.first.clear();
        self.first.resize(count + 1, 0);
        for v in 0..count {
            let q = parent(v);
            if q != NONE {
                self.first[q as usize + 1] += 1;
            }
        }
        let mut sum = 0;
        for first in &mut self.first {
            sum += *first;
            *first = sum;
        }
        self.at.clear();
        self.at.extend_from_slice(&self.first[..count]);
        self.children.clear();
        self.children.resize(self.first[count] as usize, 0);
        for v in 0..count {
            let q = parent(v);
            if q != NONE {
                let k = &mut self.at[q as usize];
                self.children[*k as usize] = v as u32;
                *k += 1;
            }
        }
        #[cfg(debug_assertions)]
        {
            // Every vertex hangs from a root: no cycle of predecessors (see the docs).
            let mut stack: Vec<usize> =
                (0..count).filter(|&v| (v == zero || role[v] != 0) && pred[v] == NONE).collect();
            let mut reached = 0;
            while let Some(v) = stack.pop() {
                reached += 1;
                stack.extend(self.children_of(v).iter().map(|&c| c as usize));
            }
            let counted = (0..count).filter(|&v| v == zero || role[v] != 0).count();
            assert_eq!(reached, counted, "a cycle of a split's predecessors");
        }
    }

    /// The children of the vertex `v`.
    #[inline]
    fn children_of(&self, v: usize) -> &[u32] {
        &self.children[self.first[v] as usize..self.first[v + 1] as usize]
    }
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
/// matroid, the elements by block (the blocks' ends in `ends`), each with [`SNAP_FREE`] set where it is free, their
/// masks, and the members of each block in slot order (their ends in `slot_ends`).  The residues are not kept: a
/// derived tableau keeps only whether each element is free (see "Deriving a tableau" in the docs of [`Meet`]).
#[derive(Clone, Default)]
pub struct Snapshot<const R: usize> {
    elems: [Vec<u32>; 2],
    ends: [Vec<usize>; 2],
    masks: [Vec<Vector<R>>; 2],
    slots: [Vec<u32>; 2],
    slot_ends: [Vec<usize>; 2],
    /// The tableau saved.
    tableau: u64,
}

/// The bit of an element of a [`Snapshot`] that marks it free.
const SNAP_FREE: u32 = 1 << 31;

impl<const R: usize> Meet<R> {
    /// A workspace for problems of `n` elements and `blocks` blocks in each matroid.
    pub fn new(n: usize, blocks: [usize; 2]) -> Self {
        Meet {
            mask: [vec![[0; R]; n], vec![[0; R]; n]],
            res: [vec![[0; R]; n], vec![[0; R]; n]],
            residues: false,
            #[cfg(debug_assertions)]
            listed: vec![0; n],
            free: [vec![false; n], vec![false; n]],
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
            free_out: vec![Vec::with_capacity(n); blocks[0]],
            span_out: vec![Vec::with_capacity(n); blocks[0]],
            #[cfg(test)]
            check_paths: false,
            #[cfg(test)]
            paths_checked: 0,
            #[cfg(test)]
            lists_visited: [0; 2],
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
            order: Vec::with_capacity(n),
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
            entering_rows: Vec::new(),
            found: NONE,
            path_bound: vec![0; n],
            bounded_at: vec![0; n],
            bounds_stamp: 0,
            bounds_for: 0,
            least_w2: 0,
            dummy_back: [NONE; 2],
            withouts: 0,
            withouts_exact: 0,
            changed_at: [vec![0; n], vec![0; n]],
            relaxed: Vec::new(),
            lineage: Vec::new(),
            mark: vec![0; n],
            entering: Vec::new(),
            incoming: Vec::new(),
            leaving: Vec::new(),
            fresh: Vec::new(),
            dropped: Vec::new(),
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
        self.residues = true;
        self.lineage.clear();
        self.lineage.push(self.tableau);
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
            #[cfg(debug_assertions)]
            {
                self.listed[i] = self.tableau;
            }
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

    /// [`gap`](Self::gap), where `pot` is the weight split of the tableau's set (its `w1` by element, as
    /// [`potentials`](Self::potentials) has it): found by an A* search over the split's reduced lengths
    /// ([`gap_by_split`](Self::gap_by_split)) instead of Bellman–Ford's.  The same gap, which debug builds check.
    pub fn gap_with(&mut self, p: &Problem<'_, R>, pot: &[i64]) -> i64 {
        if self.gap_for != self.tableau {
            debug_assert!(self.settled && self.tableau == self.solves, "the gap of no settled tableau");
            self.gap = self.gap_by_split(p, pot);
            #[cfg(debug_assertions)]
            {
                let plain = self.find_gap(p);
                assert_eq!(self.gap, plain, "the gap by the split's search and by Bellman–Ford's differ");
            }
            self.gap_for = self.tableau;
        }
        self.gap
    }

    /// Whether [`gap`](Self::gap) knows the gap of the tableau the workspace holds, with no search.
    pub fn knows_gap(&self) -> bool {
        self.gap_for == self.tableau
    }

    /// The gap of [`gap_with`](Self::gap_with): the length of the shortest augmenting path, from an element outside the
    /// set free in A to one free in B, `i64::MAX` if there is none.  With the split, such a path's length is the sum
    /// of its arcs' reduced lengths (none negative) less `w1` of its first element and `w2` of its last (see the
    /// module docs), each not negative too: so a search from every element free in A at its `−w1`, which ends a path
    /// at an element free in B at its `−w2`.  A member's arcs to the elements free in A offer them no less than they
    /// start from (`w1` of a member is not negative), so they are not offered.  The search is A* over the bounds of
    /// [`smaller_loss`](Self::smaller_loss): an element outside the set in the span of B takes its least arc out, one
    /// free in B its `−w2`, a member 0.  A path is no longer than the set's weight, so no key above it is offered.
    fn gap_by_split(&mut self, p: &Problem<'_, R>, pot: &[i64]) -> i64 {
        if self.columns_for != self.tableau {
            self.build_columns();
        }
        self.sort_by_split(p, pot);
        self.bounds_for_tableau(p, pot);
        let w = p.weight;
        let w1 = |e: u32| pot[e as usize];
        let w2 = |e: u32| w[e as usize] - pot[e as usize];
        let settled = self.next_generation(2);
        let reached = settled - 1;
        let (reach, seen, stamp) = (&mut self.reach, &mut self.seen, &mut self.settled_at);
        let (bound, bounded_at, bounds_stamp) = (&mut self.path_bound, &mut self.bounded_at, self.bounds_stamp);
        let (by_w2, heap) = (&self.by_w2, &mut self.heap);
        let past = self.value.max(0) + 1;
        heap.start(past, self.pos.len());
        let mut best = past;
        // The bound of an element outside the set (not free in both: that would make the set heavier).
        macro_rules! h {
            ($i:expr, $v:expr) => {{
                let (i, v): (usize, u32) = ($i, $v);
                if self.free[1][i] {
                    -w2(v)
                } else {
                    if bounded_at[i] != bounds_stamp {
                        let (elems, mut least) = (&self.slot_elem[1][usize::from(p.block[1][i])], INF);
                        each_bit(&self.mask[1][i], |s| least = least.min(w2(elems[s]) - w2(v)));
                        bound[i] = least;
                        bounded_at[i] = bounds_stamp;
                    }
                    bound[i]
                }
            }};
        }
        // Offers the distance `d` to `v`, with the bound `h` of the rest of a path from it.
        macro_rules! offer {
            ($v:expr, $d:expr, $h:expr) => {{
                let (v, d): (u32, Dist) = ($v, $d);
                let i = v as usize;
                if stamp[i] != settled && (seen[i] != reached || d < reach[i]) {
                    let key = d.saturating_add($h);
                    if key < best {
                        seen[i] = reached;
                        reach[i] = d;
                        heap.push(key, v);
                    }
                }
            }};
        }
        // The members as reached from the elements free in B (each has an arc to every member): the hub, the least
        // distance less `w2` over those taken, and the members by increasing `w2`, of which the next one not taken is
        // queued at the hub's distance plus its `w2`.
        let mut hub = INF;
        let mut next_member = 0;
        macro_rules! queue_member {
            () => {{
                while next_member < by_w2.len() && stamp[by_w2[next_member] as usize] == settled {
                    next_member += 1;
                }
                if let (Some(&z), true) = (by_w2.get(next_member), hub != INF) {
                    if hub + w2(z) < best {
                        heap.push(hub + w2(z), HUB_MEMBER);
                    }
                }
            }};
        }
        for &y in &self.free_a {
            let i = y as usize;
            debug_assert!(!self.free[1][i], "an element free in both matroids of the heaviest set of all sizes");
            offer!(y, -w1(y), h!(i, y));
        }
        while let Some((key, code)) = heap.pop() {
            if key >= best {
                break;
            }
            let (v, d) = if code == HUB_MEMBER {
                while next_member < by_w2.len() && stamp[by_w2[next_member] as usize] == settled {
                    next_member += 1;
                }
                let Some(&z) = by_w2.get(next_member) else { continue };
                if hub + w2(z) != key {
                    continue;
                }
                let i = z as usize;
                if !(seen[i] == reached && reach[i] <= key) {
                    (seen[i], reach[i]) = (reached, key);
                }
                stamp[i] = settled;
                queue_member!();
                (z, reach[i])
            } else {
                let i = code as usize;
                let h = if self.inside[i] { 0 } else { h!(i, code) };
                if stamp[i] == settled || reach[i].saturating_add(h) != key {
                    continue;
                }
                stamp[i] = settled;
                if by_w2.get(next_member) == Some(&code) {
                    queue_member!();
                }
                (code, reach[i])
            };
            let i = v as usize;
            if self.inside[i] {
                // A member: arcs of A to the elements of its column (those free in A start lower).
                let column = usize::from(p.block[0][i]) * 64 * R + usize::from(self.slot[0][i]);
                for &y in &self.columns[column] {
                    offer!(y, d + w1(v) - w1(y), h!(y as usize, y));
                }
            } else if self.free[1][i] {
                // Free in B: a path ends here, and goes on to every member through the hub.
                best = best.min(d - w2(v));
                if d - w2(v) < hub {
                    hub = d - w2(v);
                    queue_member!();
                }
            } else {
                let elems = &self.slot_elem[1][usize::from(p.block[1][i])];
                each_bit(&self.mask[1][i], |s| {
                    let z = elems[s];
                    offer!(z, d + w2(z) - w2(v), 0);
                });
            }
        }
        if best >= past {
            i64::MAX
        } else {
            best
        }
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

    /// An upper bound on the weight of the elements of `live` in any common independent set that holds `fixed`
    /// beside them (`fixed` common independent, none of it in `live`): with `pot` the weight split of a heaviest set
    /// of all sizes of `p` (its `w1` by element, as [`potentials`](Self::potentials) has it), the heaviest
    /// independent set of `A` for `w1` plus that of `B` for `w2`, each among the sets of `live` elements that are
    /// independent with `fixed` (weak duality, see the module docs, "The weight split": `w = w1 + w2` on every
    /// element, so a common independent set `J` of `live` elements with `fixed` weighs `w1(J) + w2(J)`, each at
    /// most its matroid's heaviest).  Each heaviest set is greedy (a matroid's, contracted by `fixed`): the elements
    /// of positive weight there, by decreasing weight, each kept while independent.  It uses the spans of the
    /// uncontended elements, not the tableau, so the workspace still holds its solve.
    pub fn split_bound_among(&mut self, p: &Problem<'_, R>, pot: &[i64], live: &[u32], fixed: &[u32]) -> i64 {
        let w = p.weight;
        let mut total = 0;
        for m in 0..2 {
            for span in &mut self.spans[m] {
                span.clear();
            }
            for &e in fixed {
                let i = e as usize;
                let inserted = self.spans[m][usize::from(p.block[m][i])].insert(p.vector[m][i], e);
                debug_assert!(inserted, "the fixed elements are dependent");
            }
            let weight = |i: usize| if m == 0 { pot[i] } else { w[i] - pot[i] };
            self.order.clear();
            self.order.extend(live.iter().map(|&e| (weight(e as usize), e)).filter(|&(we, _)| we > 0));
            self.order.sort_unstable_by(|a, b| b.cmp(a));
            for &(we, e) in &self.order {
                let i = e as usize;
                if self.spans[m][usize::from(p.block[m][i])].insert(p.vector[m][i], e) {
                    total += we;
                }
            }
        }
        total
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
            self.free[m][z] = !is_zero(&p.vector[m][z]);
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
        let source = self.tableau;
        if held && self.derive_by_contraction(p, live, start, contracted) {
            // No element changed: a contraction and deletions alone.
            self.finish_derive(p, live, source, true, (self.columns_for == source).then_some(false));
            #[cfg(test)]
            self.check_derived(p, live, start);
            return;
        }
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
        self.incoming.clear();
        if ok {
            self.incoming.extend(start.iter().chain(contracted).filter(|&&e| !self.inside[e as usize]));
        }
        for m in 0..2 {
            for b in 0..self.block_live[m].len() {
                self.unsure[m][b].clear();
                if !ok {
                    continue;
                }
                self.entering.clear();
                self.leaving.clear();
                for &e in &self.incoming {
                    if usize::from(p.block[m][e as usize]) == b {
                        self.entering.push(e);
                    }
                }
                for &x in &self.slot_elem[m][b][..self.next_slot[m][b]] {
                    if x != NONE && self.mark[x as usize] & WANTED == 0 {
                        self.leaving.push(x);
                    }
                }
                let mut i = 0;
                while i < self.entering.len() {
                    let yi = self.entering[i] as usize;
                    let (mask, slot) = (&self.mask[m][yi], &self.slot[m]);
                    let k = (!free!(self, m, yi))
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
                    if outside && (free!(self, m, zi) || held()) {
                        self.unsure[m][b].push(z);
                    }
                }
            }
        }
        // The new set and its elements (an element that changes sides is marked changed); then each block's slots
        // brought to it and the circuits not known found, or the block built.
        let columns = ok && self.columns_for == source;
        self.dropped.clear();
        for list in &self.block_live[0] {
            for &e in list {
                let i = e as usize;
                if self.mark[i] & LIVE == 0 {
                    if columns && !self.inside[i] && !self.free[0][i] {
                        self.dropped.push(e);
                    }
                    self.inside[i] = false;
                }
            }
        }
        for &e in live {
            let i = e as usize;
            let inside = self.mark[i] & WANTED != 0;
            if self.mark[i] & SOURCE != 0 && self.inside[i] != inside {
                self.changed_at[0][i] = self.tableau;
                self.changed_at[1][i] = self.tableau;
            }
            self.inside[i] = inside;
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
                    self.move_slots(m, b);
                    self.reduce_unknown(p, m, b);
                } else {
                    self.rebuild(p, m, b);
                    self.built += 1;
                }
                self.dirty[m][b] = false;
            }
        }
        self.finish_derive(p, live, source, ok, columns.then_some(true));
        #[cfg(test)]
        self.check_derived(p, live, start);
    }

    /// What [`derive`](Self::derive) does last, its tableau made from the one numbered `source` (`ok`, else built):
    /// the columns of A where the workspace had the source's (`columns`: `Some(marked)`, `marked` false where no
    /// element can have been marked changed), the lineage, the lists.
    fn finish_derive(&mut self, p: &Problem<'_, R>, live: &[u32], source: u64, ok: bool, columns: Option<bool>) {
        self.derived += 1;
        self.residues = false;
        if ok && self.lineage.last() == Some(&source) {
            if self.lineage.len() == LINEAGE {
                self.lineage.remove(0);
            }
        } else {
            self.lineage.clear();
        }
        self.lineage.push(self.tableau);
        self.lists(p, live);
        // After the lists, which the columns built to check them in debug builds are made from.
        if let Some(marked) = columns {
            self.derive_columns(p, live, marked);
        }
        // `start` is the heaviest set of all sizes: the solve would end with the shortest path that gains nothing, whose
        // length is the gap, found when first asked for.
        (self.settled, self.stopped) = (true, false);
    }

    /// In tests, where asked for ([`check`](Self::check)): the tableau [`derive`](Self::derive) made of the heaviest
    /// set `start` of the elements `live` against the one a solve builds.
    #[cfg(test)]
    fn check_derived(&mut self, p: &Problem<'_, R>, live: &[u32], start: &[u32]) {
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

    /// [`derive`](Self::derive) where the new set is the one held less the members `contracted`, and the elements
    /// `live` are among those held: no element changes sides, so every circuit stays but for the members gone (a
    /// contraction keeps the others, and a deletion changes none), and each block's elements stay in their order.  So
    /// the elements no longer live leave the blocks' lists, and the members contracted free their slots
    /// ([`move_slots`](Self::move_slots)), with no marks, no exchange and no reduction.  False, with nothing done,
    /// where the sets are not so (the general derivation then makes the tableau).
    fn derive_by_contraction(&mut self, p: &Problem<'_, R>, live: &[u32], start: &[u32], contracted: &[u32]) -> bool {
        let held = |meet: &Self, e: u32| {
            let i = e as usize;
            let list = if meet.inside[i] { &meet.members } else { &meet.out_a[usize::from(p.block[0][i])] };
            list.get(meet.pos[i] as usize) == Some(&e)
        };
        let members = start.len() + contracted.len() == self.members.len();
        if !members || !start.iter().chain(contracted).all(|&e| self.inside[e as usize] && held(self, e)) {
            return false;
        }
        let g = self.next_generation(1);
        for &e in live {
            if !held(self, e) {
                return false;
            }
            self.seen[e as usize] = g;
        }
        let live_now = |meet: &Self, e: u32| meet.seen[e as usize] == g;
        if !start.iter().all(|&e| live_now(self, e)) || contracted.iter().any(|&e| live_now(self, e)) {
            return false;
        }
        // Every member is in `start` or `contracted`, and none of `contracted` is live: a live element is a member
        // exactly where it was.
        let columns = self.columns_for == self.tableau;
        self.solves += 1;
        self.tableau = self.solves;
        self.dropped.clear();
        for list in &self.block_live[0] {
            for &e in list {
                let i = e as usize;
                if self.seen[i] != g {
                    if columns && !self.inside[i] && !self.free[0][i] {
                        self.dropped.push(e);
                    }
                    self.inside[i] = false;
                }
            }
        }
        for m in 0..2 {
            for b in 0..self.block_live[m].len() {
                let seen = &self.seen;
                self.block_live[m][b].retain(|&e| seen[e as usize] == g);
                self.move_slots(m, b);
            }
        }
        true
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
                let slots = |t: &Meet<R>| {
                    let mut members: Vec<u32> =
                        t.slot_elem[m][b][..t.next_slot[m][b]].iter().copied().filter(|&z| z != NONE).collect();
                    members.sort_unstable();
                    format!("{members:?}")
                };
                same(&format!("members of block {b} of {m}"), slots(self), slots(other))?;
            }
            for &e in live {
                let i = e as usize;
                let row = |t: &Meet<R>| {
                    let free = t.free[m][i];
                    if t.inside[i] {
                        let s = usize::from(t.slot[m][i]);
                        format!("member, its slot's {}", t.slot_elem[m][usize::from(p.block[m][i])][s])
                    } else if free {
                        "free".to_string()
                    } else {
                        let mut circuit = Vec::new();
                        each_bit(&t.mask[m][i], |s| circuit.push(t.slot_elem[m][usize::from(p.block[m][i])][s]));
                        circuit.sort_unstable();
                        format!("circuit {circuit:?}")
                    }
                };
                same(&format!("element {e} of block {} of {m}", p.block[m][i]), row(self), row(other))?;
            }
        }
        Ok(())
    }

    /// The columns of A of the tableau [`derive`](Self::derive) just made of the live elements `live`, from those of
    /// the tableau it derived it from, which the workspace holds ([`build_columns`](Self::build_columns)): the same
    /// elements by slot, in another order within a column, which no search depends on (each takes its vertices by
    /// distance and element).  A column whose slot no member holds is emptied.  Where no element changed (a contraction
    /// and deletions alone), every other circuit stayed, less the members gone, so an element no longer live leaves the
    /// columns of its circuit, which nothing touched.  Where some changed (marked by `derive`), every column drops the
    /// elements no longer live and those changed, the column of a member that came in is emptied (its slot may be one
    /// that a member contracted left, whose column holds elements whose circuits lost it), and each changed element
    /// outside the set in the span of A joins the columns of its new circuit: an element whose circuit holds a member
    /// that came in changed with it.  Unless `marked`, the derivation marked no element (a contraction and deletions
    /// alone, [`derive_by_contraction`](Self::derive_by_contraction)), and the elements are not looked at for marks.
    fn derive_columns(&mut self, p: &Problem<'_, R>, live: &[u32], marked: bool) {
        let tableau = self.tableau;
        let changed = marked && live.iter().any(|&e| self.changed_at[0][e as usize] == tableau);
        let g = if changed { self.next_generation(1) } else { 0 };
        let columns = &mut self.columns;
        for (b, slots) in self.slot_elem[0].iter().enumerate() {
            for (s, &x) in slots[..self.next_slot[0][b]].iter().enumerate() {
                if x == NONE {
                    columns[b * 64 * R + s].clear();
                }
            }
        }
        if changed {
            for &e in live {
                self.seen[e as usize] = g;
            }
            let (seen, changed_at) = (&self.seen, &self.changed_at[0]);
            for (b, slots) in self.slot_elem[0].iter().enumerate() {
                for (s, &x) in slots[..self.next_slot[0][b]].iter().enumerate() {
                    let column = &mut columns[b * 64 * R + s];
                    if x != NONE && changed_at[x as usize] == tableau {
                        // A member that came in, in a slot that another left (whose column held the elements of that
                        // one's circuit): its column is the changed elements', pushed below.
                        column.clear();
                    } else if x != NONE {
                        column.retain(|&y| seen[y as usize] == g && changed_at[y as usize] != tableau);
                    }
                }
            }
            for &y in live {
                let i = y as usize;
                if changed_at[i] == tableau && !self.inside[i] && !free!(self, 0, i) {
                    let b = usize::from(p.block[0][i]);
                    each_bit(&self.mask[0][i], |s| columns[b * 64 * R + s].push(y));
                }
            }
        } else {
            for &y in &self.dropped {
                let i = y as usize;
                let b = usize::from(p.block[0][i]);
                each_bit(&self.mask[0][i], |s| {
                    let column = &mut columns[b * 64 * R + s];
                    if let Some(k) = column.iter().position(|&z| z == y) {
                        column.swap_remove(k);
                    }
                });
            }
        }
        self.free_a.clear();
        for list in &self.block_live[0] {
            for &y in list {
                let i = y as usize;
                if !self.inside[i] && free!(self, 0, i) {
                    self.free_a.push(y);
                }
            }
        }
        self.columns_for = tableau;
        #[cfg(debug_assertions)]
        {
            let sorted = |columns: &[Vec<u32>]| -> Vec<Vec<u32>> {
                columns
                    .iter()
                    .map(|c| {
                        let mut c = c.clone();
                        c.sort_unstable();
                        c
                    })
                    .collect()
            };
            // As sets: the order within a column, and of the elements free in A, decides only which of several
            // equal offers a search or Bellman–Ford takes first, which changes no distance, cycle length or split, and
            // the searches sort the elements free in A first (`sort_by_split`).
            let (derived, mut free_a) = (sorted(&self.columns), self.free_a.clone());
            self.build_columns();
            assert_eq!(derived, sorted(&self.columns), "derived columns of A differ from those built");
            free_a.sort_unstable();
            let mut built = self.free_a.clone();
            built.sort_unstable();
            assert_eq!(free_a, built, "the elements free in A of derived columns differ from those built");
        }
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
                    let (e, free) = (s.elems[m][k] & !SNAP_FREE, s.elems[m][k] & SNAP_FREE != 0);
                    let i = e as usize;
                    list.push(e);
                    self.mask[m][i] = s.masks[m][k];
                    self.free[m][i] = free;
                    self.slot[m][i] = NO_SLOT;
                }
                at = s.ends[m][b];
                let slots = &s.slots[m][slot_at..s.slot_ends[m][b]];
                for (k, &e) in slots.iter().enumerate() {
                    self.slot_elem[m][b][k] = e;
                    if e != NONE {
                        self.slot[m][e as usize] = k as u16;
                    }
                }
                self.next_slot[m][b] = slots.len();
                slot_at = s.slot_ends[m][b];
            }
        }
        for &e in &s.elems[0] {
            self.inside[(e & !SNAP_FREE) as usize] = false;
        }
        for &e in s.slots[0].iter().filter(|&&e| e != NONE) {
            self.inside[e as usize] = true;
        }
        // The workspace holds the snapshot's tableau, under a number of its own, nothing changed since the copy, with
        // no residues.
        self.solves += 1;
        self.tableau = self.solves;
        self.residues = false;
        for changed in &mut self.changed_at {
            changed.fill(0);
        }
        self.lineage.clear();
        self.lineage.extend([snapshot.tableau, self.tableau]);
        self.restored += 1;
        self.derive(p, live, start, contracted);
    }

    /// Saves the tableau the workspace holds in `snapshot`, for [`derive_from`](Self::derive_from).
    pub fn save(&self, snapshot: &mut Snapshot<R>) {
        debug_assert_eq!(self.tableau, self.solves, "a snapshot of no tableau");
        let s = snapshot;
        s.tableau = self.tableau;
        for m in 0..2 {
            for v in [&mut s.elems[m], &mut s.slots[m]] {
                v.clear();
            }
            s.ends[m].clear();
            s.masks[m].clear();
            s.slot_ends[m].clear();
            for b in 0..self.block_live[m].len() {
                for &e in &self.block_live[m][b] {
                    s.elems[m].push(if self.free[m][e as usize] { e | SNAP_FREE } else { e });
                    s.masks[m].push(self.mask[m][e as usize]);
                }
                s.ends[m].push(s.elems[m].len());
                s.slots[m].extend_from_slice(&self.slot_elem[m][b][..self.next_slot[m][b]]);
                s.slot_ends[m].push(s.slots[m].len());
            }
        }
    }

    /// The rows of the live elements of block `b` of matroid `m` whose circuits the tableau derived from does not give:
    /// those it did not have (`fresh`), and those `unsure` where the span changed.  Each is reduced by the span of
    /// the block's members, each inserted as its slot, so that its residue is zero exactly when it is in the span, and
    /// its mask is then its circuit.
    fn reduce_unknown(&mut self, p: &Problem<'_, R>, m: usize, b: usize) {
        let unsure = std::mem::take(&mut self.unsure[m][b]);
        if unsure.is_empty() && !self.fresh.iter().any(|&e| usize::from(p.block[m][e as usize]) == b) {
            self.unsure[m][b] = unsure;
            return;
        }
        let span = &mut self.spans[m][b];
        span.clear();
        for (k, &z) in self.slot_elem[m][b][..self.next_slot[m][b]].iter().enumerate() {
            if z != NONE {
                let inserted = span.insert_as(p.vector[m][z as usize], k);
                debug_assert!(inserted, "a member of a common independent set is dependent");
            }
        }
        for &e in self.fresh.iter().filter(|&&e| usize::from(p.block[m][e as usize]) == b).chain(&unsure) {
            let i = e as usize;
            let residue;
            (residue, self.mask[m][i]) = span.reduce(p.vector[m][i]);
            self.free[m][i] = !is_zero(&residue);
            self.slot[m][i] = NO_SLOT;
            self.changed_at[m][i] = self.tableau;
        }
        self.unsure[m][b] = unsure;
        self.reduced += 1;
    }

    /// Brings the slots of block `b` of matroid `m` to the new set (`inside`): each member gone (contracted, or no
    /// longer a member where the span changed, `dirty`) frees its slot, whose bit leaves the circuits of the live
    /// elements, and each member that entered where the span changed takes the first free slot.  The other members
    /// keep theirs, so that the circuits carry over.
    fn move_slots(&mut self, m: usize, b: usize) {
        let mut gone = [0u64; R];
        let mut freed = false;
        for (k, z) in self.slot_elem[m][b][..self.next_slot[m][b]].iter_mut().enumerate() {
            if *z != NONE && !self.inside[*z as usize] {
                self.slot[m][*z as usize] = NO_SLOT;
                *z = NONE;
                gone[k / 64] |= 1 << (k % 64);
                freed = true;
            }
        }
        if freed {
            for &z in &self.block_live[m][b] {
                if !self.inside[z as usize] {
                    for (c, g) in self.mask[m][z as usize].iter_mut().zip(&gone) {
                        *c &= !g;
                    }
                }
            }
        }
        if !self.dirty[m][b] {
            return;
        }
        let mut k = 0;
        for &z in &self.block_live[m][b] {
            let z = z as usize;
            if self.inside[z] && self.slot[m][z] == NO_SLOT {
                while k < self.next_slot[m][b] && self.slot_elem[m][b][k] != NONE {
                    k += 1;
                }
                if k == self.next_slot[m][b] {
                    self.next_slot[m][b] += 1;
                }
                self.slot_elem[m][b][k] = z as u32;
                self.slot[m][z] = k as u16;
                let mut own = [0; R];
                own[k / 64] |= 1 << (k % 64);
                self.mask[m][z] = own;
                self.free[m][z] = false;
            }
        }
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
        let (mask, res, free) = (&mut self.mask[m], &mut self.res[m], &mut self.free[m]);
        let (word, shift) = (p / 64, p % 64);
        for &z in &self.block_live[m][b] {
            let z = z as usize;
            let select = 0u64.wrapping_sub(res[z][word] >> shift & 1);
            for k in 0..R {
                res[z][k] ^= ry[k] & select;
                mask[z][k] ^= my[k] & select;
            }
            free[z] = !is_zero(&res[z]);
        }
        // y itself: its residue cleared by its own row, its mask now just its slot.
        self.res[m][yi] = [0; R];
        self.free[m][yi] = false;
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
        let (mask, changed, tableau) = (&mut self.mask[m], &mut self.changed_at[m], self.tableau);
        let (word, shift) = (s / 64, s % 64);
        for &z in &self.block_live[m][b] {
            let z = z as usize;
            let select = 0u64.wrapping_sub(mask[z][word] >> shift & 1);
            for k in 0..R {
                mask[z][k] ^= d[k] & select;
            }
            if select != 0 {
                changed[z] = tableau;
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
    ///
    /// Only an element free in `A` takes the hub's offer, which is new only in a round where a member improved, and
    /// only one in the span of `A` an offer from its circuit, only where a member of the block changed.  So the
    /// outside elements of each block of `A` are listed apart at the start by kind, in the order of the block's
    /// list, and where only one kind can gain, only its list is visited: the others would be left as they are, so
    /// the same elements change, in the same order, as in a scan of the whole list (tests check it against one,
    /// [`shortest_path_by_scan`](Self::shortest_path_by_scan)).
    fn shortest_path(&mut self, p: &Problem<'_, R>) -> Option<u32> {
        #[cfg(test)]
        if self.check_paths {
            return self.checked_shortest_path(p);
        }
        self.bellman_ford::<true>(p)
    }

    /// In tests: the search of [`shortest_path`](Self::shortest_path) without the lists by kind, every outside element
    /// of a block visited in each round where it is.
    #[cfg(test)]
    fn shortest_path_by_scan(&mut self, p: &Problem<'_, R>) -> Option<u32> {
        self.bellman_ford::<false>(p)
    }

    /// In tests: [`shortest_path`](Self::shortest_path) checked against
    /// [`shortest_path_by_scan`](Self::shortest_path_by_scan): the same sink, rounds, and distance and predecessor
    /// of every member and outside element.
    #[cfg(test)]
    fn checked_shortest_path(&mut self, p: &Problem<'_, R>) -> Option<u32> {
        let before = self.rounds;
        let scanned = self.shortest_path_by_scan(p);
        let (dist, pred, rounds) = (self.dist.clone(), self.pred.clone(), self.rounds - before);
        self.rounds = before;
        let found = self.bellman_ford::<true>(p);
        assert_eq!(found, scanned, "the sink differs from the scan's");
        assert_eq!(self.rounds - before, rounds, "the rounds differ from the scan's");
        for &e in self.members.iter().chain(self.out_a.iter().flatten()) {
            let i = e as usize;
            assert_eq!((self.dist[i], self.pred[i]), (dist[i], pred[i]), "element {e} differs from the scan's");
        }
        self.paths_checked += 1;
        found
    }

    /// The search of [`shortest_path`](Self::shortest_path), visiting only the outside elements of the kind that can
    /// gain where one kind can (`LISTS`), or every outside element of a block visited.
    fn bellman_ford<const LISTS: bool>(&mut self, p: &Problem<'_, R>) -> Option<u32> {
        let w = p.weight;
        for &x in &self.members {
            self.dist[x as usize] = INF;
            self.pred[x as usize] = NONE;
        }
        self.changed_out.clear();
        let mut sink = (INF, NONE);
        for (ba, list) in self.out_a.iter().enumerate() {
            let (free_out, span_out) = (&mut self.free_out[ba], &mut self.span_out[ba]);
            free_out.clear();
            span_out.clear();
            for &y in list {
                let i = y as usize;
                self.pred[i] = NONE;
                if !free!(self, 0, i) {
                    self.dist[i] = INF;
                    if LISTS {
                        span_out.push(y);
                    }
                } else {
                    let d = -w[i] * HOPS;
                    self.dist[i] = d;
                    self.changed_out.push(y);
                    if LISTS {
                        free_out.push(y);
                    }
                    if free!(self, 1, i) && d < sink.0 {
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
                if free!(self, 1, i) {
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
            for ba in 0..self.out_a.len() {
                let changed = self.changed_slots[0][ba];
                let any_changed = !is_zero(&changed);
                if !any_changed && !member_improved {
                    continue;
                }
                let elems = &self.slot_elem[0][ba];
                // The offer to the outside element `y`, free in A (`free`) or not: the hub's, or the least of the
                // changed members' of its circuit.
                macro_rules! visit {
                    ($y:expr, $free:expr) => {{
                        let y: u32 = $y;
                        let i = y as usize;
                        let ly = 1 - w[i] * HOPS;
                        let mut best = self.dist[i];
                        let mut from = NONE;
                        if $free {
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
                            if best < sink.0 && free!(self, 1, i) {
                                sink = (best, y);
                            }
                        }
                    }};
                }
                match (LISTS, member_improved, any_changed) {
                    (true, true, false) => {
                        #[cfg(test)]
                        {
                            self.lists_visited[0] += 1;
                        }
                        for k in 0..self.free_out[ba].len() {
                            visit!(self.free_out[ba][k], true);
                        }
                    }
                    (true, false, true) => {
                        #[cfg(test)]
                        {
                            self.lists_visited[1] += 1;
                        }
                        for k in 0..self.span_out[ba].len() {
                            visit!(self.span_out[ba][k], false);
                        }
                    }
                    _ => {
                        for k in 0..self.out_a[ba].len() {
                            let y = self.out_a[ba][k];
                            visit!(y, free!(self, 0, y as usize));
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
        self.split_bounds_from(p, live, None, None, f)
    }

    /// The split's arc `q → u` in the current tableau (`zero` the zero node), as [`split_bounds`](Self::split_bounds)
    /// lists them.  The answer for a vertex no longer live reads stale fields and means nothing: the walks that ask
    /// also find such a vertex's path broken.
    fn split_arc(&self, p: &Problem<'_, R>, q: usize, u: usize, zero: usize) -> bool {
        if q == zero {
            return self.inside[u] || free!(self, 0, u);
        }
        if u == zero {
            return self.inside[q] || free!(self, 1, q);
        }
        match (self.inside[q], self.inside[u]) {
            (true, false) => {
                !free!(self, 0, u)
                    && p.block[0][q] == p.block[0][u]
                    && bit(&self.mask[0][u], usize::from(self.slot[0][q]))
            }
            (false, true) => {
                !free!(self, 1, q)
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
    /// walk up the predecessors from each vertex, stopped at the first whose status is known.  Where the constraint
    /// graph on the vertices `start` had is known to be `start`'s on them (`kept`, see
    /// [`keeps_arcs`](Self::keeps_arcs)), only the vertices are checked.  Returns the stamp in `seen` of the live
    /// vertices whose path holds, and whether the zero node's does.
    fn warm_start(&mut self, p: &Problem<'_, R>, live: &[u32], start: &SplitStart, kept: bool) -> (u32, bool) {
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
                if !kept && !self.split_arc(p, q as usize, u, zero) {
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
        (valid, zero_status == valid)
    }

    /// The split's starting potentials and first round from `start`, an earlier split found on a tableau that the
    /// workspace's derives from ([`lineage`](Self::lineage)), by what changed since: in each matroid, the elements
    /// whose circuit or freedom there, or side of the set, changed (marked by [`derive`](Self::derive)), and the
    /// elements no longer live.
    ///
    /// The constraint graph's arcs into an element outside the set are of A (from the members of its circuit, or from
    /// the zero node where it is free), and those out of it of B (to the members of its circuit, or to the zero node
    /// where it is free); a member's arcs to and from the zero node are fixed by its side, and its others are those
    /// of the elements outside.  A circuit that only lost members (a member contracted or gone) only lost arcs.  So an
    /// arc between two elements on their sides of `start` is an arc of `start`'s graph, as long, unless it runs into an
    /// element changed in A or out of one changed in B.  A vertex whose path of predecessors in `start` is still a path
    /// starts from its potential there: every vertex on it live and on its side, and each arc there.  The paths that
    /// break are those through a vertex no longer live or on the other side, through an element changed in A whose arc
    /// from its predecessor is gone, or through a child of an element changed in B whose arc from it is gone: the
    /// subtrees of those vertices in the forest of `start`'s predecessors ([`Tree`]).  The others' potentials are
    /// lengths of paths, no lower than the distances, so Bellman–Ford from them reaches the same split.
    ///
    /// Where the zero node's path holds, an arc that the first round of a split from scratch would find too long runs
    /// into a vertex whose path broke or an element changed in A, or out of a vertex whose path broke or an element
    /// changed in B: between other vertices it is an arc of `start`'s graph between potentials of `start`, which keep
    /// it.  So the vertices whose paths broke and the elements changed in A pull their potentials along their arcs in,
    /// and the first round relaxes the arcs out of the vertices whose paths broke, of the elements changed in B, and of
    /// those whose pull lowered them (as [`pull_into_broken`](Self::pull_into_broken) does where nothing changed).
    /// Where the zero node's path broke, the first round relaxes every vertex.
    fn repair_from(&mut self, p: &Problem<'_, R>, live: &[u32], start: &SplitStart) {
        let w = p.weight;
        let zero = self.pot.len() - 1;
        let since = start.tableau;
        let tree = start.tree();
        let g = self.next_generation(3);
        let (live_mark, broken, listed) = (g - 2, g - 1, g);
        for &e in live {
            self.settled_at[e as usize] = live_mark;
        }
        let base = start.pot[zero];
        let on_side = |inside: &[bool], i: usize| start.role[i] == 1 + u8::from(inside[i]);
        // The roots of the subtrees that break; the elements changed in A, to pull, and in B, to relax first.
        let mut roots = std::mem::take(&mut self.walk);
        let mut pulled = std::mem::take(&mut self.changed_out);
        let mut relaxed = std::mem::take(&mut self.relaxed);
        roots.clear();
        pulled.clear();
        relaxed.clear();
        self.changed_in.clear();
        for &e in live {
            let i = e as usize;
            if !on_side(&self.inside, i) {
                roots.push(e);
                continue;
            }
            self.pot[i] = start.pot[i] + base;
            let q = start.pred[i];
            self.split_pred[i] = q;
            if self.inside[i] {
                continue;
            }
            if self.changed_at[0][i] > since {
                if q != NONE && !self.split_arc(p, q as usize, i, zero) {
                    roots.push(e);
                }
                pulled.push(e);
            }
            if self.changed_at[1][i] > since {
                for &c in tree.children_of(i) {
                    let ci = c as usize;
                    let kept = ci == zero || (self.settled_at[ci] == live_mark && on_side(&self.inside, ci));
                    if kept && !self.split_arc(p, i, ci, zero) {
                        roots.push(c);
                    }
                }
                relaxed.push(e);
            }
        }
        self.pot[zero] = base;
        self.split_pred[zero] = start.pred[zero];
        for (i, &role) in start.role.iter().enumerate() {
            if role != 0 && self.settled_at[i] != live_mark {
                roots.push(i as u32);
            }
        }
        // What hangs from the roots: the live ones start from 0 again, to be pulled and relaxed.
        let mut zero_holds = true;
        while let Some(v) = roots.pop() {
            let vi = v as usize;
            if vi == zero {
                if !zero_holds {
                    continue;
                }
                zero_holds = false;
                (self.pot[zero], self.split_pred[zero]) = (0, NONE);
            } else {
                if self.seen[vi] == broken {
                    continue;
                }
                self.seen[vi] = broken;
                if self.settled_at[vi] == live_mark {
                    (self.pot[vi], self.split_pred[vi]) = (0, NONE);
                    pulled.push(v);
                }
            }
            roots.extend_from_slice(tree.children_of(vi));
        }
        drop(tree);
        self.walk = roots;
        if !zero_holds {
            self.changed_in.extend_from_slice(live);
            self.changed_in.push(zero as u32);
            (self.changed_out, self.relaxed) = (pulled, relaxed);
            return;
        }
        for &v in &relaxed {
            if self.seen[v as usize] != broken {
                self.seen[v as usize] = listed;
                self.changed_in.push(v);
            }
        }
        // Each vertex to pull takes its potential along its arcs in: an element outside from the zero node where it is
        // free in A, else from the members of its circuit in A; a member from the zero node, and from the elements
        // outside whose circuit in B holds it, found by one pass over each block of B that has such members.  Those
        // whose paths broke are relaxed first, and so are the others that the pull lowered.
        let pz = self.pot[zero];
        for &v in &pulled {
            let vi = v as usize;
            let (mut best, mut from) = (self.pot[vi], NONE);
            if self.inside[vi] {
                if pz + w[vi] < best {
                    (best, from) = (pz + w[vi], zero as u32);
                }
                let (b, s) = (usize::from(p.block[1][vi]), usize::from(self.slot[1][vi]));
                self.changed_slots[1][b][s / 64] |= 1 << (s % 64);
            } else if free!(self, 0, vi) {
                if pz < best {
                    (best, from) = (pz, zero as u32);
                }
            } else {
                let (pot, elems) = (&self.pot, &self.slot_elem[0][usize::from(p.block[0][vi])]);
                each_bit(&self.mask[0][vi], |s| {
                    let x = elems[s];
                    if pot[x as usize] < best {
                        (best, from) = (pot[x as usize], x);
                    }
                });
            }
            if from != NONE {
                self.pot[vi] = best;
                self.split_pred[vi] = from;
            }
            if self.seen[vi] == broken || (from != NONE && self.seen[vi] != listed) {
                self.seen[vi] = listed;
                self.changed_in.push(v);
            }
        }
        for b in 0..self.block_live[1].len() {
            let slots = std::mem::replace(&mut self.changed_slots[1][b], [0; R]);
            if is_zero(&slots) {
                continue;
            }
            let elems = &self.slot_elem[1][b];
            for &y in &self.block_live[1][b] {
                let yi = y as usize;
                if self.inside[yi] || free!(self, 1, yi) {
                    continue;
                }
                let mut into = self.mask[1][yi];
                for (word, slot) in into.iter_mut().zip(&slots) {
                    *word &= slot;
                }
                let (pot, pred) = (&mut self.pot, &mut self.split_pred);
                each_bit(&into, |s| {
                    let x = elems[s] as usize;
                    let d = pot[yi] + w[x] - w[yi];
                    if d < pot[x] {
                        (pot[x], pred[x]) = (d, y);
                    }
                });
            }
        }
        (self.changed_out, self.relaxed) = (pulled, relaxed);
    }

    /// Whether, for a problem that is `start`'s contracted by `contracted`, the current set is `start`'s less
    /// `contracted`, every element live in both outside the set in both, and every element live only now outside it.
    /// Then the constraint graph on the vertices `start` had is `start`'s on them, with the same lengths: contracting
    /// members keeps the other elements' circuits less them and which elements are free, and deleting elements outside
    /// the set changes no circuit.
    fn keeps_arcs(&self, live: &[u32], start: &SplitStart, contracted: &[u32]) -> bool {
        let role = |e: u32| start.role[e as usize];
        contracted.iter().all(|&c| role(c) == 2)
            && self.members.len() + contracted.len() == start.members
            && live.iter().all(|&e| match role(e) {
                0 => !self.inside[e as usize],
                r => r == 1 + u8::from(self.inside[e as usize]),
            })
    }

    /// The first round of a split started from `start` where [`keeps_arcs`](Self::keeps_arcs) holds and the zero
    /// node's path holds: the potential of a vertex whose path holds (stamp `valid`) is its distance in `start`'s
    /// graph, so no arc between two such vertices can lower one.  So the round pulls each other vertex's potential
    /// along its arcs in, and lists those vertices to relax their own arcs, which reaches what a round over every
    /// vertex would leave to the next.
    fn pull_into_broken(&mut self, p: &Problem<'_, R>, live: &[u32], valid: u32) {
        let w = p.weight;
        let zero = self.pot.len() - 1;
        let pz = self.pot[zero];
        for &v in live {
            let vi = v as usize;
            if self.seen[vi] == valid {
                continue;
            }
            self.changed_in.push(v);
            let (mut best, mut from) = (self.pot[vi], NONE);
            if self.inside[vi] {
                // From the zero node, and from the elements whose circuit in B holds it.
                if pz + w[vi] < best {
                    (best, from) = (pz + w[vi], zero as u32);
                }
                let s = usize::from(self.slot[1][vi]);
                for &y in &self.block_live[1][usize::from(p.block[1][vi])] {
                    let yi = y as usize;
                    if !self.inside[yi] && !free!(self, 1, yi) && bit(&self.mask[1][yi], s) {
                        let d = self.pot[yi] + w[vi] - w[yi];
                        if d < best {
                            (best, from) = (d, y);
                        }
                    }
                }
            } else if free!(self, 0, vi) {
                // Free in A: from the zero node.
                if pz < best {
                    (best, from) = (pz, zero as u32);
                }
            } else {
                // From the members of its circuit in A.
                let (pot, elems) = (&self.pot, &self.slot_elem[0][usize::from(p.block[0][vi])]);
                each_bit(&self.mask[0][vi], |s| {
                    let x = elems[s];
                    if pot[x as usize] < best {
                        (best, from) = (pot[x as usize], x);
                    }
                });
            }
            if from != NONE {
                self.pot[vi] = best;
                self.split_pred[vi] = from;
            }
        }
    }

    /// The split's potentials (before they are shifted to make `w1`): Bellman–Ford from all zero, lowered first to
    /// those of `start` where they still hold (see [`warm_start`](Self::warm_start)), its first round narrowed where
    /// the problem is `start`'s contracted by `contracted` (see [`pull_into_broken`](Self::pull_into_broken)).  False
    /// if the constraints have no solution.
    fn split_potentials(
        &mut self,
        p: &Problem<'_, R>,
        live: &[u32],
        start: Option<&SplitStart>,
        contracted: Option<&[u32]>,
    ) -> bool {
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
        self.changed_in.clear();
        let mut seeded = false;
        if let Some(start) = start.filter(|start| start.saved) {
            if self.lineage.contains(&start.tableau) {
                self.repair_from(p, live, start);
                seeded = true;
            } else {
                let kept = contracted.is_some_and(|c| self.keeps_arcs(live, start, c));
                let (valid, zero_valid) = self.warm_start(p, live, start, kept);
                if kept && zero_valid {
                    self.pull_into_broken(p, live, valid);
                    seeded = true;
                }
            }
        }
        if !seeded {
            self.changed_in.extend_from_slice(live);
            self.changed_in.push(zero as u32);
        }
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
                } else if free!(self, 1, u) {
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
    /// where they still hold ([`split_from`](Self::split_from)), with the bound without each member in one pass over
    /// the circuits.
    pub fn split_bounds_from(
        &mut self,
        p: &Problem<'_, R>,
        live: &[u32],
        start: Option<&SplitStart>,
        contracted: Option<&[u32]>,
        mut f: impl FnMut(u32, i64),
    ) -> bool {
        if !self.split_from(p, live, start, contracted) {
            return false;
        }
        let w = p.weight;
        // In each matroid the members of positive weight there, B_m, are a heaviest basis of the elements of
        // positive weight; without a member x of B_m the heaviest independent set is B_m − x plus the heaviest
        // positive element whose circuit with respect to B_m holds x (one exchange), if any.  That circuit is the
        // element's circuit with respect to I*, its mask: an element's circuit holds only members at least as heavy
        // as it in the matroid (the split's arcs, `w1(y) ≤ w1(x)` in A and `w2(y) ≤ w2(x)` in B for a member x of
        // y's circuit), so a positive element's circuit holds members of B_m alone.
        for m in 0..2 {
            let (pot, repl) = (&self.pot, &mut self.repl[m]);
            let weight_in = |i: usize| if m == 0 { pot[i] } else { w[i] - pot[i] };
            for &e in live {
                repl[e as usize] = 0;
            }
            for &e in live {
                let i = e as usize;
                let wy = weight_in(i);
                if wy <= 0 || self.inside[i] {
                    continue;
                }
                debug_assert!(!free!(self, m, i), "an element of positive weight is free of I*");
                let elems = &self.slot_elem[m][usize::from(p.block[m][i])];
                each_bit(&self.mask[m][i], |s| {
                    let x = elems[s] as usize;
                    debug_assert!(weight_in(x) >= wy, "a member of a circuit lighter than its element");
                    repl[x] = repl[x].max(wy);
                });
            }
        }
        for &e in live {
            let i = e as usize;
            if self.inside[i] {
                let split = (self.pot[i], w[i] - self.pot[i]);
                f(e, bound_without(self.value, split, [self.repl[0][i], self.repl[1][i]]));
            }
        }
        true
    }

    /// After [`split_from`](Self::split_from) on the tableau the workspace holds, with `pot` its split (`w1` by
    /// element, as [`potentials`](Self::potentials) has it): the split's bound without the member `x` alone, as
    /// [`split_bounds_from`](Self::split_bounds_from) finds it for every member.  The positive elements whose
    /// circuits hold `x` are those of its column in A, and in B those of its block whose masks hold its slot.
    pub fn split_bound_without(&mut self, p: &Problem<'_, R>, pot: &[i64], x: u32) -> i64 {
        if self.columns_for != self.tableau {
            self.build_columns();
        }
        let w = p.weight;
        let i = x as usize;
        let column = usize::from(p.block[0][i]) * 64 * R + usize::from(self.slot[0][i]);
        let repl_a = self.columns[column].iter().map(|&y| pot[y as usize]).filter(|&v| v > 0).max().unwrap_or(0);
        let s = usize::from(self.slot[1][i]);
        let repl_b = self.block_live[1][usize::from(p.block[1][i])]
            .iter()
            .map(|&y| y as usize)
            .filter(|&y| !self.inside[y] && bit(&self.mask[1][y], s))
            .map(|y| w[y] - pot[y])
            .filter(|&v| v > 0)
            .max()
            .unwrap_or(0);
        bound_without(self.value, (pot[i], w[i] - pot[i]), [repl_a, repl_b])
    }

    /// The weight split of the tableau's set, the heaviest of all sizes (see the module docs, "The weight split"):
    /// `w1` by element in [`potentials`](Self::potentials), the bounds without its members left to
    /// [`split_bound_without`](Self::split_bound_without); false if the constraints have no solution.  Its
    /// Bellman–Ford starts from the potentials of an earlier split `start` where they still hold
    /// ([`warm_start`](Self::warm_start)): the same split, since the shortest distances are the same from any
    /// potentials that are lengths of paths and no lower (debug builds check it against the split from all zero).  If
    /// `p` is `start`'s problem contracted by the members `contracted`, the first round relaxes fewer arcs
    /// ([`pull_into_broken`](Self::pull_into_broken)).
    pub fn split_from(
        &mut self,
        p: &Problem<'_, R>,
        live: &[u32],
        start: Option<&SplitStart>,
        contracted: Option<&[u32]>,
    ) -> bool {
        debug_assert!(self.settled);
        if self.columns_for != self.tableau {
            self.build_columns();
        }
        let zero = self.pot.len() - 1;
        #[cfg(debug_assertions)]
        let cold = start.is_some() && {
            let solved = self.split_potentials(p, live, None, None);
            self.cold.copy_from_slice(&self.pot);
            solved
        };
        let solved = self.split_potentials(p, live, start, contracted);
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
        let (length, end) = self.shortest_cycle::<false>(p, pot, x, limit, banned)?;
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

    /// After a [`solve`](Self::solve) that settled, or a [`derive`](Self::derive), with the workspace still holding
    /// it: the weight of the heaviest common independent set without its member `x`, of any size, among the sets that
    /// hold every member of `banned` and no other element of it, and that set, the heaviest set flipped along the
    /// elements of [`cycle`](Self::cycle); `None` if the flip is not common independent, which the argument below
    /// rules out (debug builds assert it).  `pot` is the weight split of the heaviest set `I` (as for
    /// [`without`](Self::without)).  The tableau is left as it was.
    ///
    /// [`without`](Self::without)'s search, in the exchange graph of `I` with elements of weight 0 added that are
    /// independent of everything (each in a block of its own in both matroids): copies of `o` outside the set and of
    /// `i` in it.  `I` with the copies of `i` is a heaviest common independent set of all sizes there, and `w1` and
    /// `w2` both 0 on the copies extend the split.  A common independent set `J` without `x`, with as many copies of
    /// each added as make its size that set's, differs from it by cycles alone, whose lengths are not negative and add
    /// up to `w(I) − w(J)` (see the module docs, "The heaviest set without a member"), and one of them passes through
    /// `x`.  The copies of `o` have the same arcs (from and to every member), and so have those of `i` (from every
    /// element free in B, to every element free in A), so a cycle through two copies of one is no shorter than the one
    /// that goes on from the first as from the second (what it leaves out closes a cycle) and has fewer arcs: the
    /// search needs one `o` and one `i`, and its shortest cycle through `x` gives the heaviest set without `x` of every
    /// size, with no bound on what the other sizes lose.  Less `o` and `i`, the cycle is one of `I`'s exchange graph,
    /// or a path through `x` from a member to a member (through `o`: a set one smaller), from an element free in A to
    /// one free in B (through `i`: one larger), or between a member and an element free in one matroid (through both).
    /// `o` is searched as an element free in both matroids reached through `through`, `i` as a member reached through
    /// the hub, `w1(o)` and `w2(i)` being 0.  The cycle `x → o → x`, of length `w(x)` (the set `I − x`), bounds the
    /// search.
    pub fn without_any_size(&mut self, p: &Problem<'_, R>, pot: &[i64], x: u32, banned: &[u32]) -> Option<i64> {
        self.withouts += 1;
        let (length, end) = self.shortest_cycle::<true>(p, pot, x, p.weight[x as usize], banned)?;
        // The cycle's elements: from the one that closes it, its predecessors back to x, the two vertices that stand
        // for no element left out.
        self.flip.clear();
        self.flip.push(x);
        let (mut v, mut steps) = (end, 0);
        while v != x && v != NONE && steps <= self.pos.len() + 2 {
            v = match v {
                OUT_DUMMY => self.dummy_back[0],
                IN_DUMMY => self.dummy_back[1],
                _ => {
                    self.flip.push(v);
                    self.back[v as usize]
                }
            };
            steps += 1;
        }
        if v != x || !self.flipped_independent(p) {
            debug_assert!(false, "the shortest cycle through a member flips into a dependent set");
            return None;
        }
        self.withouts_exact += 1;
        Some(self.value - length)
    }

    /// Whether the exchange graph of [`without`](Self::without) has a cycle through the member `x` at most `limit`
    /// long: if it has none, the sets without `x` of the heaviest set's size lose more than `limit` against it.  If it
    /// has, a shortest is kept for [`keep_cycle`](Self::keep_cycle) (the set it flips to is not checked).  Where
    /// the search cannot be made (no settled solve's tableau holding `x`, or a limit out of range), the answer is yes,
    /// which claims nothing.  `pot` is the tableau's weight split (as for [`without`](Self::without)), and `closing`
    /// is at most the length of every arc into `x` (0 if nothing is known, see
    /// [`closing_lengths`](Self::closing_lengths)).
    ///
    /// The search is A* over the split's reduced lengths, which are never negative: a vertex is taken in the order of
    /// its distance from `x` plus a lower bound on the length of a path from it back to `x`, its least arc out where
    /// that is cheap to know, else 0.  An element outside the set in the span of A, which the members' columns offer,
    /// has arcs out to the members of its circuit in B, or to every member if it is free in B; its least is found when
    /// the element is first offered on the tableau.  Members and the elements free in A, which the search reaches
    /// through the hubs in the order of `w2` and of `w1`, take 0.  Each bound is at most every arc out of its vertex
    /// plus the bound of the arc's head, so the bounds are consistent: the keys taken never decrease, each vertex is
    /// taken at its distance, and every cycle through a vertex taken at key `k` is at least `k` long.  So the search
    /// stops at the first key at or past the shortest cycle found, or past `limit`.  A cycle through a vertex at
    /// distance `d` is also at least `d + closing` long, so no vertex is offered a distance above `limit − closing`.
    /// An element free in B has arcs to every member and no other, so it shortens no distance unless it lowers the hub
    /// (the least distance less `w2` over such elements taken): one that would not is not offered.
    pub fn has_cycle_within(&mut self, p: &Problem<'_, R>, pot: &[i64], x: u32, limit: i64, closing: i64) -> bool {
        let searchable = self.settled
            && self.tableau == self.solves
            && self.inside[x as usize]
            && (0..i64::MAX / (2 * HOPS)).contains(&limit);
        self.found = NONE;
        if !searchable {
            return true;
        }
        let Some((end, _)) = self.bounded_cycle(p, pot, x, limit, closing) else { return false };
        self.found = end;
        true
    }

    /// The length of the shortest cycle through the member `x` of the exchange graph of [`without`](Self::without), if
    /// one is at most `limit` long: the length [`without`](Self::without) finds, by the A* search of
    /// [`has_cycle_within`](Self::has_cycle_within), which costs less (its cycle may be another as short, and is not
    /// kept).  The same conditions as [`without`](Self::without)'s, with `None` where they fail.
    pub fn shortest_cycle_length(&mut self, p: &Problem<'_, R>, pot: &[i64], x: u32, limit: i64) -> Option<i64> {
        let searchable = self.settled
            && self.tableau == self.solves
            && self.inside[x as usize]
            && (0..i64::MAX / (2 * HOPS)).contains(&limit);
        debug_assert!(
            searchable,
            "the shortest cycle through a member needs the tableau of a settled solve holding it"
        );
        self.found = NONE;
        if !searchable {
            return None;
        }
        self.bounded_cycle(p, pot, x, limit, 0).map(|(_, length)| length)
    }

    /// The search of [`has_cycle_within`](Self::has_cycle_within): the element that closes a shortest cycle through `x`
    /// within `limit`, if there is one, and the cycle's length.  Any shortest cycle will do, so the distances leave the
    /// arcs out, and an entry can tie in distance with the last one taken and come out before the others of that
    /// distance ([`Queue`]).
    fn bounded_cycle(
        &mut self,
        p: &Problem<'_, R>,
        pot: &[i64],
        x: u32,
        limit: i64,
        closing: i64,
    ) -> Option<(u32, i64)> {
        if self.columns_for != self.tableau {
            self.build_columns();
        }
        self.sort_by_split(p, pot);
        self.bounds_for_tableau(p, pot);
        let w = p.weight;
        let w1 = |e: u32| pot[e as usize];
        let w2 = |e: u32| w[e as usize] - pot[e as usize];
        let settled = self.next_generation(2);
        let reached = settled - 1;
        let (reach, back, seen, stamp) = (&mut self.reach, &mut self.back, &mut self.seen, &mut self.settled_at);
        let (bound, bounded_at, bounds_stamp, least_w2) =
            (&mut self.path_bound, &mut self.bounded_at, self.bounds_stamp, self.least_w2);
        let (free_a, by_w2) = (&self.free_a, &self.by_w2);
        // Keys from `limit + 1` on cannot lead to a cycle within `limit`, and with every arc into x at least `closing`
        // long, nor can a distance from `limit + 1 − closing` on.
        let past = limit + 1;
        let offered = past - closing.clamp(0, past);
        let heap = &mut self.heap;
        heap.start(past, self.pos.len());
        // The shortest cycle so far and the element that closes it; the member through which the elements free in A
        // are reached (the least distance plus `w1` over the members taken) and the next of them by decreasing `w1`;
        // and the hub through which the members are reached (the least distance less `w2` over the elements free in B
        // taken) and the next of them by increasing `w2`.
        let mut cycle = (past, NONE);
        let (mut through, mut next_free) = ((INF, NONE), 0);
        let (mut hub, mut next_member) = ((INF, NONE), 0);
        // Offers the distance `d` to `v` from `from`, with the bound `h` of the path back (0 for a member).
        macro_rules! offer {
            ($v:expr, $d:expr, $h:expr, $from:expr) => {{
                let (v, d): (u32, Dist) = ($v, $d);
                let i = v as usize;
                if d.saturating_add($h) < cycle.0
                    && d < offered
                    && stamp[i] != settled
                    && (seen[i] != reached || d < reach[i])
                {
                    seen[i] = reached;
                    reach[i] = d;
                    back[i] = $from;
                    heap.push(d + $h, v);
                }
            }};
        }
        macro_rules! queue_free {
            () => {{
                while next_free < free_a.len() && stamp[free_a[next_free] as usize] == settled {
                    next_free += 1;
                }
                if let (Some(&y), true) = (free_a.get(next_free), through.0 != INF) {
                    let d = through.0 - w1(y);
                    if d < cycle.0 && d < offered {
                        heap.push(d, FREE_A);
                    }
                }
            }};
        }
        macro_rules! queue_member {
            () => {{
                while next_member < by_w2.len() && stamp[by_w2[next_member] as usize] == settled {
                    next_member += 1;
                }
                if let (Some(&z), true) = (by_w2.get(next_member), hub.0 != INF) {
                    let d = hub.0 + w2(z);
                    if d < cycle.0 && d < offered {
                        heap.push(d, HUB_MEMBER);
                    }
                }
            }};
        }
        // A member taken at `d`: arcs of A to the elements of its column and to every element free in A.
        macro_rules! from_member {
            ($u:expr, $d:expr) => {{
                let (u, d): (u32, Dist) = ($u, $d);
                let column = usize::from(p.block[0][u as usize]) * 64 * R + usize::from(self.slot[0][u as usize]);
                for &y in &self.columns[column] {
                    let (i, dy) = (y as usize, d + w1(u) - w1(y));
                    let free_b = self.free[1][i];
                    if free_b && dy - w2(y) >= hub.0 {
                        continue;
                    }
                    if bounded_at[i] != bounds_stamp {
                        bound[i] = if free_b {
                            least_w2 - w2(y)
                        } else {
                            let (elems, mut least) = (&self.slot_elem[1][usize::from(p.block[1][i])], INF);
                            each_bit(&self.mask[1][i], |s| least = least.min(w2(elems[s]) - w2(y)));
                            least
                        };
                        bounded_at[i] = bounds_stamp;
                    }
                    offer!(y, dy, bound[i], u);
                }
                if d + w1(u) < through.0 {
                    through = (d + w1(u), u);
                    queue_free!();
                }
            }};
        }
        stamp[x as usize] = settled;
        (seen[x as usize], reach[x as usize], back[x as usize]) = (reached, 0, NONE);
        from_member!(x, 0);
        while let Some((key, code)) = heap.pop() {
            if key >= cycle.0 {
                break;
            }
            let (v, d) = match code {
                FREE_A => {
                    while next_free < free_a.len() && stamp[free_a[next_free] as usize] == settled {
                        next_free += 1;
                    }
                    let Some(&y) = free_a.get(next_free) else { continue };
                    if through.0 - w1(y) != key {
                        continue;
                    }
                    let i = y as usize;
                    (seen[i], reach[i], back[i], stamp[i]) = (reached, key, through.1, settled);
                    queue_free!();
                    (y, key)
                }
                HUB_MEMBER => {
                    while next_member < by_w2.len() && stamp[by_w2[next_member] as usize] == settled {
                        next_member += 1;
                    }
                    let Some(&z) = by_w2.get(next_member) else { continue };
                    if hub.0 + w2(z) != key {
                        continue;
                    }
                    // Its own offers are no shorter, or it would have been taken at theirs.
                    let i = z as usize;
                    if !(seen[i] == reached && reach[i] <= key) {
                        (seen[i], reach[i], back[i]) = (reached, key, hub.1);
                    }
                    stamp[i] = settled;
                    queue_member!();
                    (z, reach[i])
                }
                v => {
                    let i = v as usize;
                    let h = if self.inside[i] { 0 } else { bound[i] };
                    if stamp[i] == settled || reach[i] + h != key {
                        continue;
                    }
                    stamp[i] = settled;
                    if by_w2.get(next_member) == Some(&v) {
                        queue_member!();
                    }
                    (v, reach[i])
                }
            };
            let i = v as usize;
            if self.inside[i] {
                from_member!(v, d);
            } else if self.free[1][i] {
                // Free in B: arcs to every member, x's closing a cycle, the others' through the hub.
                if d - w2(v) < hub.0 {
                    hub = (d - w2(v), v);
                    if hub.0 + w2(x) < cycle.0 {
                        cycle = (hub.0 + w2(x), v);
                    }
                    queue_member!();
                }
            } else {
                // The members of its circuit in B.
                let elems = &self.slot_elem[1][usize::from(p.block[1][i])];
                each_bit(&self.mask[1][i], |s| {
                    let z = elems[s];
                    let dz = d + w2(z) - w2(v);
                    if z == x {
                        if dz < cycle.0 {
                            cycle = (dz, v);
                        }
                    } else {
                        offer!(z, dz, 0, v);
                    }
                });
            }
        }
        (cycle.1 != NONE).then_some((cycle.1, cycle.0))
    }

    /// For the searches for cycles through the members of the tableau's set, with its weight split `pot` (`w1` by
    /// element): in `closing`, for each member, the least length of an arc into it, `i64::MAX` where there is none.
    /// The arcs into a member come from the elements free in B and from those whose circuit in B holds it, each `w2`
    /// of the member less that of the element, so one pass over the circuits of B finds them all.
    pub fn closing_lengths(&self, p: &Problem<'_, R>, pot: &[i64], closing: &mut [i64]) {
        let w = p.weight;
        let w2 = |i: usize| w[i] - pot[i];
        // First the heaviest element in `w2` with an arc into each member, then the arc.
        for &x in &self.members {
            closing[x as usize] = i64::MIN;
        }
        let mut free_b = i64::MIN;
        for &y in self.out_a.iter().flatten() {
            let i = y as usize;
            if free!(self, 1, i) {
                free_b = free_b.max(w2(i));
            } else {
                let elems = &self.slot_elem[1][usize::from(p.block[1][i])];
                each_bit(&self.mask[1][i], |s| {
                    let x = elems[s] as usize;
                    closing[x] = closing[x].max(w2(i));
                });
            }
        }
        for &x in &self.members {
            let i = x as usize;
            let heaviest = closing[i].max(free_b);
            closing[i] = if heaviest == i64::MIN { i64::MAX } else { w2(i) - heaviest };
        }
    }

    /// The least length of an arc out of the member `x` of the tableau's set, with its weight split `pot`:
    /// `w1` of `x` less that of the heaviest element in `w1` free in A or whose circuit in A holds `x`; `i64::MAX` if
    /// there is none.  A cycle through `x` is at least this plus its [closing length](Self::closing_lengths) long:
    /// it leaves `x` by one of these arcs and comes back by one of those (two arcs even where it has two vertices).
    pub fn opening_length(&mut self, p: &Problem<'_, R>, pot: &[i64], x: u32) -> i64 {
        if self.columns_for != self.tableau {
            self.build_columns();
        }
        self.sort_by_split(p, pot);
        let i = x as usize;
        let column = usize::from(p.block[0][i]) * 64 * R + usize::from(self.slot[0][i]);
        let free_a = self.free_a.first().map_or(i64::MIN, |&y| pot[y as usize]);
        let heaviest = self.columns[column].iter().map(|&y| pot[y as usize]).fold(free_a, i64::max);
        if heaviest == i64::MIN {
            i64::MAX
        } else {
            pot[i] - heaviest
        }
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
            if !free!(self, m, y) && !in_circuit() {
                return None;
            }
            length += if self.inside[u] { p.weight[u] } else { -p.weight[u] };
            u = v;
        }
        Some(length)
    }

    /// The search of [`without`](Self::without): the shortest cycle's length within `limit` and the element that closes
    /// it at `x`.
    fn shortest_cycle<const SIZES: bool>(
        &mut self,
        p: &Problem<'_, R>,
        pot: &[i64],
        x: u32,
        limit: i64,
        banned: &[u32],
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
        // With `SIZES`, the vertices that stand for no element ([`without_any_size`](Self::without_any_size)): their
        // distances, predecessors, and whether they are settled (`OUT_DUMMY`'s first).
        let mut dummy_reach = [INF; 2];
        let mut dummy_back = [NONE; 2];
        let mut dummy_done = [false; 2];
        // Offers the distance `d` to `v` from `from`, unless it is no shorter than the best cycle so far (the search
        // ends before it would take the entry), or `v` is free in B and would not lower the hub: such an element has
        // arcs to the members alone, all of which the hub offers no further, so it changes no distance, no
        // predecessor and no cycle (and taking it, which offers nothing, no order of the offers).
        macro_rules! offer {
            ($v:expr, $d:expr, $from:expr) => {{
                let (v, d): (u32, Dist) = ($v, $d);
                let i = v as usize;
                if d < cycle.0
                    && stamp[i] != settled
                    && (seen[i] != reached || d < reach[i])
                    && !(self.free[1][i] && d - w2(v) * HOPS + 1 >= hub.0)
                {
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
        // With `SIZES`, offers the element outside that is free in both matroids its distance through `through` (its
        // `w1` is 0), and the member in no circuit its distance through the hub (its `w2` is 0).
        macro_rules! offer_out {
            () => {{
                let d = through.0 + 1;
                if SIZES && !dummy_done[0] && d < dummy_reach[0] && d < cycle.0 {
                    (dummy_reach[0], dummy_back[0]) = (d, through.1);
                    heap.push(d, OUT_DUMMY);
                }
            }};
        }
        macro_rules! offer_in {
            () => {{
                let d = hub.0;
                if SIZES && !dummy_done[1] && d < dummy_reach[1] && d < cycle.0 {
                    (dummy_reach[1], dummy_back[1]) = (d, hub.1);
                    heap.push(d, IN_DUMMY);
                }
            }};
        }
        heap.push(0, x);
        seen[x as usize] = reached;
        reach[x as usize] = 0;
        back[x as usize] = NONE;
        while let Some((d, v)) = heap.pop() {
            if d >= cycle.0 {
                break;
            }
            if SIZES && (v == OUT_DUMMY || v == IN_DUMMY) {
                let k = usize::from(v == IN_DUMMY);
                if dummy_done[k] || d != dummy_reach[k] {
                    continue;
                }
                dummy_done[k] = true;
                tick += 1;
                if v == OUT_DUMMY {
                    // Free in B, of `w2` 0: arcs to every member, x's closing a cycle, the others' through the hub.
                    if d + w2(x) * HOPS + 1 < cycle.0 {
                        cycle = (d + w2(x) * HOPS + 1, v);
                    }
                    if d + 1 < hub.0 {
                        hub = (d + 1, v, tick);
                        queue_member!();
                        offer_in!();
                    }
                } else if d < through.0 {
                    // A member in no circuit, of `w1` 0: arcs to the elements free in A alone.
                    through = (d, v);
                    queue_free!();
                    offer_out!();
                }
                continue;
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
                    offer_out!();
                }
            } else if free!(self, 1, i) {
                // Free in B: arcs to every member, x's closing a cycle, the others' through the hub.
                if d + (w2(x) - w2(v)) * HOPS + 1 < cycle.0 {
                    cycle = (d + (w2(x) - w2(v)) * HOPS + 1, v);
                }
                if d - w2(v) * HOPS + 1 < hub.0 {
                    hub = (d - w2(v) * HOPS + 1, v, tick);
                    queue_member!();
                    offer_in!();
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
        self.dummy_back = dummy_back;
        (cycle.1 != NONE).then(|| (cycle.0.div_euclid(HOPS), cycle.1))
    }

    /// After a [`solve`](Self::solve) that settled, with the workspace still holding it, and `pot` the weight split
    /// of its heaviest set (as for [`without`](Self::without)): the least that a smaller common independent set
    /// loses against the heaviest set.  The symmetric difference of the two splits into cycles of the exchange graph
    /// and paths from member to member, at least one (see the module docs), so that loss is at least the shortest
    /// such path, a member alone included, whose length is the sum of the reduced lengths of its arcs plus `w2` of
    /// its first member and `w1` of its last: a search from every member at its `w2`, as in
    /// [`without`](Self::without).  `i64::MAX` if there is no smaller set (an empty heaviest set).
    ///
    /// The search is A* ([`has_cycle_within`](Self::has_cycle_within) has the argument): an element outside the set
    /// is taken in the order of its distance plus its least arc out, to a member of its circuit in B, found when it is
    /// first offered on the tableau and kept for the other searches there; a member, which may end a path at `w1` of
    /// no less than 0, and an element free in A, reached through the hub, take 0.  A member alone is such a path, so
    /// the search starts from the lightest member's weight and stops at the first key at or past the shortest path.
    pub fn smaller_loss(&mut self, p: &Problem<'_, R>, pot: &[i64]) -> i64 {
        if self.columns_for != self.tableau {
            self.build_columns();
        }
        self.sort_by_split(p, pot);
        self.bounds_for_tableau(p, pot);
        let w = p.weight;
        let w1 = |e: u32| pot[e as usize];
        let w2 = |e: u32| w[e as usize] - pot[e as usize];
        let Some(lightest) = self.members.iter().map(|&z| w[z as usize]).min() else { return i64::MAX };
        #[cfg(debug_assertions)]
        let plain = self.smaller_loss_by_dijkstra(p, pot);
        let settled = self.next_generation(2);
        let reached = settled - 1;
        let (reach, seen, stamp) = (&mut self.reach, &mut self.seen, &mut self.settled_at);
        let (bound, bounded_at, bounds_stamp) = (&mut self.path_bound, &mut self.bounded_at, self.bounds_stamp);
        let mut best = lightest * HOPS;
        let heap = &mut self.heap;
        heap.start(best, self.pos.len());
        // An element free in B is not offered: its arcs to the members offer each `d − w2(v) + 1 + w2(z)`, at least
        // `w2(z) + 1` (the distances are not negative, and the split makes `w2(v) ≤ 0` for an element free in B), while
        // each member started at `w2(z)`, so none is an improvement.  The key of an element outside the set is its
        // distance plus its bound (`bound`, packed as the distances are), of a member its distance.
        macro_rules! offer {
            ($v:expr, $d:expr) => {{
                let (v, d): (u32, Dist) = ($v, $d);
                let i = v as usize;
                if d < best && stamp[i] != settled && (seen[i] != reached || d < reach[i]) && !self.free[1][i] {
                    let h = if self.inside[i] {
                        0
                    } else {
                        if bounded_at[i] != bounds_stamp {
                            let (elems, mut least) = (&self.slot_elem[1][usize::from(p.block[1][i])], INF);
                            each_bit(&self.mask[1][i], |s| least = least.min(w2(elems[s]) - w2(v)));
                            bound[i] = least;
                            bounded_at[i] = bounds_stamp;
                        }
                        bound[i].saturating_mul(HOPS)
                    };
                    let key = d.saturating_add(h);
                    if key < best {
                        seen[i] = reached;
                        reach[i] = d;
                        heap.push(key, v);
                    }
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
        while let Some((key, v)) = heap.pop() {
            if key >= best {
                break;
            }
            let (v, d) = if v == FREE_A {
                while next_free < self.free_a.len() && stamp[self.free_a[next_free] as usize] == settled {
                    next_free += 1;
                }
                let Some(&y) = self.free_a.get(next_free) else { continue };
                if key != through - w1(y) * HOPS + 1 {
                    continue;
                }
                stamp[y as usize] = settled;
                queue_free!();
                (y, key)
            } else {
                let i = v as usize;
                let h = if self.inside[i] { 0 } else { bound[i].saturating_mul(HOPS) };
                if stamp[i] == settled || reach[i].saturating_add(h) != key {
                    continue;
                }
                stamp[i] = settled;
                (v, reach[i])
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
            } else {
                // In the span of B (an element free in A is in it, being free in no more than one matroid: free in both,
                // it would make the heaviest set heavier).
                debug_assert!(!free!(self, 1, i), "an element free in B in the search for the smaller sets' loss");
                let elems = &self.slot_elem[1][usize::from(p.block[1][i])];
                each_bit(&self.mask[1][i], |s| {
                    let z = elems[s];
                    offer!(z, d + (w2(z) - w2(v)) * HOPS + 1);
                });
            }
        }
        let loss = best.div_euclid(HOPS);
        #[cfg(debug_assertions)]
        assert_eq!(loss, plain, "the smaller sets' loss by A* and by Dijkstra's search differ");
        loss
    }

    /// In debug builds, [`smaller_loss`](Self::smaller_loss) by Dijkstra's search with no bounds, from no limit, to
    /// check the A* search against.
    #[cfg(debug_assertions)]
    fn smaller_loss_by_dijkstra(&mut self, p: &Problem<'_, R>, pot: &[i64]) -> i64 {
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
                if d < best && stamp[i] != settled && (seen[i] != reached || d < reach[i]) && !self.free[1][i] {
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
                best = best.min(d + w1(v) * HOPS);
                let column = usize::from(p.block[0][i]) * 64 * R + usize::from(self.slot[0][i]);
                for &y in &self.columns[column] {
                    offer!(y, d + (w1(v) - w1(y)) * HOPS + 1);
                }
                if d + w1(v) * HOPS < through {
                    through = d + w1(v) * HOPS;
                    queue_free!();
                }
            } else {
                let elems = &self.slot_elem[1][usize::from(p.block[1][i])];
                each_bit(&self.mask[1][i], |s| offer!(elems[s], d + (w2(elems[s]) - w2(v)) * HOPS + 1));
            }
        }
        if best == INF {
            i64::MAX
        } else {
            best.div_euclid(HOPS)
        }
    }

    /// The bounds of the A* searches ([`has_cycle_within`](Self::has_cycle_within), [`smaller_loss`](Self::smaller_loss))
    /// on the length of a path back from an element outside the set, made afresh where they were found on another
    /// tableau: a new stamp, cleared before the counter wraps.  After [`sort_by_split`](Self::sort_by_split).
    fn bounds_for_tableau(&mut self, p: &Problem<'_, R>, pot: &[i64]) {
        if self.bounds_for != self.tableau {
            self.bounds_stamp = self.bounds_stamp.wrapping_add(1);
            if self.bounds_stamp == 0 {
                self.bounded_at.fill(0);
                self.bounds_stamp = 1;
            }
            self.least_w2 = self.by_w2.first().map_or(INF, |&z| p.weight[z as usize] - pot[z as usize]);
            self.bounds_for = self.tableau;
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
                if !free!(self, 0, i) {
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
        start.members = self.members.len();
        start.saved = true;
        start.tableau = self.tableau;
        start.built.set(false);
    }

    /// Whether the current set flipped on `self.flip` is independent in both matroids: in each block a flipped
    /// element belongs to, the members there that stay and the outside elements that come in.  Where every element
    /// that comes in is in the span of the set, its circuit decides
    /// ([`independent_by_circuits`](Self::independent_by_circuits)); elsewhere they are inserted in a span.
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
                let independent = match self.independent_by_circuits(p, m, b) {
                    Some(independent) => {
                        debug_assert_eq!(independent, self.independent_by_span(p, m, b), "block {b} of {m}");
                        independent
                    }
                    None => self.independent_by_span(p, m, b),
                };
                if !independent {
                    self.dirty[m].fill(false);
                    return false;
                }
            }
            self.dirty[m].fill(false);
        }
        true
    }

    /// Whether the members of block `b` of matroid `m` that stay (those not on `self.flip`, which `seen` marks with
    /// the current stamp) and the elements of the flip that come in are independent, if every one coming in is in
    /// the span of the set (`None` otherwise).  An element `y` in the span is the sum of the members of its circuit,
    /// and the members are independent, so a sum of elements coming in lies in the span of the members that stay
    /// exactly when the sum of their circuits holds no slot of a member that leaves: the elements come in
    /// independently exactly when their circuits, cut to the slots of the members leaving, are independent.  A few
    /// rows of `R` words are eliminated, where a span would take every member of the block.
    fn independent_by_circuits(&mut self, p: &Problem<'_, R>, m: usize, b: usize) -> Option<bool> {
        let mut leaving = [0u64; R];
        self.entering_rows.clear();
        for &e in &self.flip {
            let i = e as usize;
            if usize::from(p.block[m][i]) != b {
                continue;
            }
            if self.inside[i] {
                let s = usize::from(self.slot[m][i]);
                leaving[s / 64] |= 1 << (s % 64);
            } else if free!(self, m, i) {
                return None;
            } else {
                self.entering_rows.push(self.mask[m][i]);
            }
        }
        let rows = &mut self.entering_rows;
        for row in rows.iter_mut() {
            for (c, l) in row.iter_mut().zip(&leaving) {
                *c &= l;
            }
        }
        // Gaussian elimination by the lowest bit of each row: a row that comes to nothing is a dependence.
        for k in 0..rows.len() {
            let row = rows[k];
            let Some(pivot) = lowest(&row) else { return Some(false) };
            for later in &mut rows[k + 1..] {
                if bit(later, pivot) {
                    xor(later, &row);
                }
            }
        }
        Some(true)
    }

    /// [`independent_by_circuits`](Self::independent_by_circuits)'s question answered from the vectors: the members
    /// that stay and the elements that come in, inserted in a span.
    fn independent_by_span(&mut self, p: &Problem<'_, R>, m: usize, b: usize) -> bool {
        let span = &mut self.spans[m][b];
        span.clear();
        let stay = self.slot_elem[m][b][..self.next_slot[m][b]]
            .iter()
            .filter(|&&z| z != NONE && self.seen[z as usize] != self.generation);
        let enter = self.flip.iter().filter(|&&y| !self.inside[y as usize] && usize::from(p.block[m][y as usize]) == b);
        for &e in stay.chain(enter) {
            if !span.insert(p.vector[m][e as usize], e) {
                return false;
            }
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
        /// Heaviest sets without a member of every size ([`Meet::without_any_size`]), with some elements banned, and of
        /// another size than the heaviest set's: smaller and larger.
        any_size: usize,
        any_size_banned: usize,
        any_size_smaller: usize,
        any_size_larger: usize,
    }

    /// Whether `set` is independent in the matroid with these blocks and vectors.
    fn independent<const R: usize>(block: &[u8], vector: &[Vector<R>], set: &[u32]) -> bool {
        let blocks = set.iter().map(|&e| usize::from(block[e as usize]) + 1).max().unwrap_or(0);
        let mut spans = vec![BlockSpan::<R>::new(); blocks];
        set.iter().all(|&e| spans[usize::from(block[e as usize])].insert(vector[e as usize], e))
    }

    /// By brute force: the heaviest common independent set of at most `cap` elements, of each subset of `live`.
    fn brute(p: &Problem<'_, 1>, live: &[u32], cap: usize) -> i64 {
        brute_holding(p, live, &[], cap)
    }

    /// [`brute`] among the sets that hold every element of `must` (elements of `live`).
    fn brute_holding(p: &Problem<'_, 1>, live: &[u32], must: &[u32], cap: usize) -> i64 {
        let mut best = 0;
        for mask in 0u32..(1 << live.len()) {
            let set: Vec<u32> = (0..live.len()).filter(|&i| mask >> i & 1 == 1).map(|i| live[i]).collect();
            if set.len() <= cap
                && must.iter().all(|e| set.contains(e))
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
        // The heaviest set without each member of every size, with nothing banned and with every third element other
        // than the member banned (members kept, the others left out), its value and the set it flips to.
        for &x in &members {
            for skip in [None, Some(trial % 3)] {
                let banned: Vec<u32> =
                    live.iter().copied().filter(|&e| e != x && Some(e as usize % 3) == skip).collect();
                let rest: Vec<u32> =
                    live.iter().copied().filter(|&e| e != x && (!banned.contains(&e) || meet.contains(e))).collect();
                let must: Vec<u32> = banned.iter().copied().filter(|&e| meet.contains(e)).collect();
                let value = meet.without_any_size(&p, &pot, x, &banned).expect("the flip is common independent");
                assert_eq!(value, brute_holding(&p, &rest, &must, n), "trial {trial}: without {x}, banned {banned:?}");
                let mut flipped: Vec<u32> = members.clone();
                for &e in meet.cycle() {
                    match flipped.iter().position(|&f| f == e) {
                        Some(i) => _ = flipped.remove(i),
                        None => flipped.push(e),
                    }
                }
                assert!(!flipped.contains(&x) && banned.iter().all(|e| flipped.contains(e) == meet.contains(*e)));
                assert!(
                    independent(p.block[0], p.vector[0], &flipped) && independent(p.block[1], p.vector[1], &flipped)
                );
                assert_eq!(flipped.iter().map(|&e| p.weight[e as usize]).sum::<i64>(), value, "trial {trial}");
                counts.any_size += 1;
                counts.any_size_banned += usize::from(!banned.is_empty());
                counts.any_size_smaller += usize::from(flipped.len() < size);
                counts.any_size_larger += usize::from(flipped.len() > size);
            }
        }
    }

    /// The search for a shortest augmenting path, which visits only the outside elements of the kind that can gain
    /// where only one kind can, against a scan of every outside element of the block: the same sink, rounds, and
    /// distance and predecessor of every element, at every augmentation and gap of solves of random problems, whose
    /// weights tie often (where the order of the offers decides the predecessors).  Both lists must be used.
    #[test]
    fn shortest_paths_by_kind_against_a_scan() {
        let mut rng = Rng(0x5D1C_E0F3_A2B4_9687);
        let mut meet_checked = (0, [0; 2]);
        for _ in 0..200 {
            let n = 10 + rng.below(60) as usize;
            let owned = random(&mut rng, n, 16);
            let p = owned.problem();
            let live: Vec<u32> = (0..n as u32).collect();
            let mut meet = Meet::<1>::new(n, owned.blocks());
            meet.check_paths = true;
            meet.solve(&p, &live, &[], usize::MAX, i64::MAX, |_| {});
            if meet.settled {
                meet.find_gap(&p);
            }
            meet_checked.0 += meet.paths_checked;
            for (total, visits) in meet_checked.1.iter_mut().zip(meet.lists_visited) {
                *total += visits;
            }
        }
        let (paths, [free, span]) = meet_checked;
        assert!(paths >= 1_000 && free >= 100 && span >= 100, "{paths} searches, {free} and {span} lists visited");
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
        let Counts { any_size, any_size_banned, any_size_smaller, any_size_larger, .. } = counts;
        println!("{any_size} without a member of any size, {any_size_banned} with some banned");
        println!("{any_size_smaller} smaller, {any_size_larger} larger");
        assert!(at_least(any_size, 10_000) && any_size_banned > any_size / 4, "{any_size} {any_size_banned}");
        assert!(any_size_smaller > any_size / 20 && any_size_larger > 0, "{any_size_smaller} {any_size_larger}");
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
    /// On random problems of one to three blocks a matroid, with tied weights, for each member `x` of the heaviest
    /// set: its least arcs out and in ([`Meet::opening_length`], [`Meet::closing_lengths`]) are those of the arcs of
    /// the exchange graph found one by one (`x → y` where `I − x + y` is independent in A, `y → x` where it is in B);
    /// a search for a cycle through `x` within a limit ([`Meet::has_cycle_within`]), bounded by its least arc in,
    /// answers as the unbounded search does and keeps the same cycle, which is a cycle of the exchange graph as long as
    /// the shortest within the limit that the search for the bound without `x` finds, and finds none where its two
    /// least arcs are longer than the limit; and the split's bound without `x` found alone
    /// ([`Meet::split_bound_without`]) is the one found for every member in one pass.
    #[test]
    fn a_members_least_arcs_bound_its_cycles_and_its_split_bound_is_found_alone() {
        let mut rng = Rng(0x1EA5_7A2C_0000_0001);
        let (mut members, mut found, mut cut) = (0, 0, 0);
        for trial in 0..300 {
            let n = 2 + rng.below(11) as usize;
            let owned = random(&mut rng, n, 4);
            let p = owned.problem();
            let w = &owned.weight;
            let live: Vec<u32> = (0..n as u32).collect();
            let mut meet = Meet::<1>::new(n, owned.blocks());
            meet.solve(&p, &live, &[], n, i64::MAX, |_| {});
            let mut bounds = Vec::new();
            if !(meet.settled && meet.split_bounds(&p, &live, |x, b| bounds.push((x, b)))) {
                continue;
            }
            let pot = meet.potentials().to_vec();
            let set: Vec<u32> = live.iter().copied().filter(|&e| meet.contains(e)).collect();
            let mut closing = vec![0; n];
            meet.closing_lengths(&p, &pot, &mut closing);
            for &(x, bound) in &bounds {
                members += 1;
                assert_eq!(meet.split_bound_without(&p, &pot, x), bound, "trial {trial}: the bound without {x}");
                let (xi, mut opening, mut closing_x) = (x as usize, i64::MAX, i64::MAX);
                for y in live.iter().copied().filter(|y| !set.contains(y)) {
                    let mut exchanged: Vec<u32> = set.iter().copied().filter(|&z| z != x).collect();
                    exchanged.push(y);
                    let yi = y as usize;
                    if independent(&owned.block[0], &owned.vector[0], &exchanged) {
                        opening = opening.min(pot[xi] - pot[yi]);
                    }
                    if independent(&owned.block[1], &owned.vector[1], &exchanged) {
                        closing_x = closing_x.min((w[xi] - pot[xi]) - (w[yi] - pot[yi]));
                    }
                }
                assert_eq!(meet.opening_length(&p, &pot, x), opening, "trial {trial}: the arcs out of {x}");
                assert_eq!(closing[xi], closing_x, "trial {trial}: the arcs into {x}");
                for limit in [0, 2, 5, 10, 20, 40] {
                    let plain = meet.has_cycle_within(&p, &pot, x, limit, 0);
                    let plain_cycle = meet.keep_cycle(x, &mut Cycles::default()).to_vec();
                    let bounded = meet.has_cycle_within(&p, &pot, x, limit, closing_x);
                    let bounded_cycle = meet.keep_cycle(x, &mut Cycles::default()).to_vec();
                    let kept = if plain { meet.cycle_length(&p, &plain_cycle) } else { None };
                    assert_eq!((bounded, bounded_cycle), (plain, plain_cycle), "trial {trial}: {x} within {limit}");
                    assert!(!plain || kept.is_some(), "trial {trial}: the cycle kept through {x} is no cycle");
                    let shortest = meet.shortest_cycle::<false>(&p, &pot, x, limit, &[]).map(|(length, _)| length);
                    assert_eq!(kept, shortest, "trial {trial}: the cycle through {x} within {limit}");
                    found += usize::from(plain);
                    if opening.saturating_add(closing_x) > limit {
                        assert!(!plain, "trial {trial}: a cycle through {x} within {limit} shorter than its arcs");
                        cut += 1;
                    }
                }
            }
        }
        println!("{members} members, {found} cycles within a limit, {cut} limits below the least arcs");
        assert!(members > 500 && found > 500 && cut > 500, "{members} {found} {cut}");
    }

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
                out.push(i64::from(meet.has_cycle_within(&p, &pot, x, limit, 0)));
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

    /// The radix heap takes its keys in a binary heap's order, on monotone runs of pushes and pops whose keys share
    /// distances and codes (the bucket of a distance sorted once, keys of the same distance pushed into it after).
    #[test]
    fn the_radix_heap_pops_as_a_binary_heap() {
        let mut rng = Rng(0x2AD1_C0DE_0000_0001);
        for trial in 0..300 {
            let (mut radix, mut heap) = (RadixHeap::new(), BinaryHeap::new());
            let mut last = 0u64;
            for _ in 0..200 {
                if rng.chance(55) {
                    // A key no lower than the last taken: often of its distance, the code above or the same.
                    let distance = (last >> Queue::BITS) + [0, 0, 0, 1, 2, 64, 1 << 20][rng.below(7) as usize];
                    let code = if distance == last >> Queue::BITS {
                        (last & Queue::FREE) + rng.below(4)
                    } else {
                        rng.below(Queue::FREE + 1)
                    };
                    let key = distance << Queue::BITS | code.min(Queue::FREE);
                    radix.push(key);
                    heap.push(Reverse(key));
                } else {
                    let (a, b) = (radix.pop(), heap.pop().map(|Reverse(k)| k));
                    assert_eq!(a, b, "trial {trial}");
                    last = a.unwrap_or(last);
                }
            }
        }
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
