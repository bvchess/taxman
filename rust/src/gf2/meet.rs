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
//! `B` are difference constraints on `w1`, solved by Bellman–Ford.
//!
//! # The heaviest set without a member
//!
//! Strong branching asks, for each member `x` of a conflict of `I*` (the heaviest set of all sizes), for the heaviest
//! common independent set without `x`.  [`Meet::without`] answers from `I*`'s exchange graph alone, with no
//! augmentation: it finds the shortest *structure through `x`*, one of `x` alone, a cycle through `x`, a path from
//! `x` to a member or to a sink (an element whose addition keeps `B` independent), or a path to `x` from a member or
//! from a source (the same for `A`), by two Bellman–Ford searches, one from `x` and one towards it.  `I*` flipped
//! along the structure, checked common independent, is a set without `x`, so its weight `w(I*)` less the
//! structure's length is a lower bound.  It is the exact value when the structure is no longer than any path with
//! `x` inside it could be:
//!
//! * Let `J` be the heaviest set without `x` (of at most the size limit).  In each matroid the symmetric difference
//!   of two independent sets of the same size has a perfect matching of exchange arcs (Schrijver, *Combinatorial
//!   Optimization*, Corollary 39.12a; when the sizes differ, the smaller set extended from the larger first), so
//!   `I* Δ J` splits into vertex-disjoint parts of the exchange graph: cycles only when `|J| = |I*|`, cycles and
//!   paths from member to member (a member alone included) when `J` is smaller, and cycles and paths from a source
//!   to a sink when it is larger.  The lengths of the parts add up to `w(I*) − w(J)`.
//! * The weight split makes the reduced length of every arc non-negative, `w1(z) − w1(y)` for an arc `z → y` of
//!   `A` and `w2(z) − w2(y)` for `y → z` of `B`, and a part's length is the sum of its arcs' reduced lengths plus
//!   `w2` of a first member and `w1` of a last one, or less `w1` of a first source and `w2` of a last sink: never
//!   negative.  So `w(J) ≤ w(I*) − ℓ`, where `ℓ` is the length of the part through `x`.
//! * That part is a structure the searches see whole (a cycle, `x` alone, `x` at an end of a path between members;
//!   a path from a source to `x` or from `x` to a sink is a cycle closed by an arc that every source has from every
//!   member and every sink to every member), or a path with `x` inside it, which is at least as long as the
//!   shortest path to `x` plus the shortest path from `x`, less `w(x)` (between members; or from a source to a
//!   sink, only if the limit allows a set larger than `I*`).  If the structure found is no longer than those two
//!   sums, `w(J) ≤ w(I*) − ℓ ≤` the lower bound.
//!
//! The certificate holds at every one of the 10,183 strong-branching members of the proof of 1215 (and the value
//! equals the solve's); where it does not, the caller solves (see `docs/EXPERIMENTS.md`, "The inner loop of the
//! GF(2) search").

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
const NO_SLOT: u8 = u8::MAX;
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
pub struct Meet<const R: usize> {
    mask: [Vec<Vector<R>>; 2],
    res: [Vec<Vector<R>>; 2],
    /// The slot of each member of the tableau's set, [`NO_SLOT`] for the others.
    slot: [Vec<u8>; 2],
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
    /// Bellman–Ford's frontier: the outside elements and the members whose distance changed in the last phase,
    /// the changed members' slots per block of each matroid, and the phase each vertex was last listed in.
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
    /// heaviest set is (the last shortest path's length), `i64::MAX` if there is none.  The heaviest sets of all
    /// larger sizes are lighter still (the weights by size are concave).
    pub gap: i64,
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
    /// The split's scratch: its potentials (one per element and one more), and the replacements' weights.
    pot: Vec<i64>,
    repl: [Vec<i64>; 2],
    /// [`without`](Self::without)'s scratch: its search's distances, predecessors (towards the member searched
    /// from), settled stamps and heap; the elements outside the set in the span of A by the slots of their circuits
    /// (`columns`, per block of A 64·R slots) and those free in A, both built for the tableau `columns_for` (and
    /// shared with the split), the latter by decreasing `w1` for the tableau `free_sorted_for`; and the elements of
    /// the cycle found.
    reach: Vec<Dist>,
    back: Vec<u32>,
    settled_at: Vec<u32>,
    heap: BinaryHeap<Reverse<(Dist, u32)>>,
    columns: Vec<Vec<u32>>,
    columns_for: u64,
    free_a: Vec<u32>,
    free_sorted_for: u64,
    flip: Vec<u32>,
    /// Counters of [`without`](Self::without): calls, and those it certified.
    pub withouts: u64,
    pub withouts_exact: u64,
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
            solves: 0,
            tableau: 0,
            value: 0,
            augmentations: 0,
            rounds: 0,
            rebuilds: 0,
            split_rounds: 0,
            pot: vec![0; n + 1],
            repl: [vec![0; n], vec![0; n]],
            reach: vec![INF; n],
            back: vec![NONE; n],
            settled_at: vec![0; n],
            heap: BinaryHeap::with_capacity(n),
            columns: vec![Vec::new(); blocks[0] * 64 * R],
            columns_for: 0,
            free_a: Vec::with_capacity(n),
            free_sorted_for: 0,
            flip: Vec::with_capacity(n),
            withouts: 0,
            withouts_exact: 0,
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
    /// set an augmentation reaches (all dominant), as the members of the set in ascending order.  The augmentations
    /// stop as soon as the set weighs `stop` or more ([`stopped`](Self::stopped)): the heaviest set weighs at least
    /// that much, since every augmentation gains.
    pub fn solve(
        &mut self,
        p: &Problem<'_, R>,
        live: &[u32],
        start: &[u32],
        cap: usize,
        stop: i64,
        mut report: impl FnMut(&[u32]),
    ) {
        let w = p.weight;
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
        self.value = self.members.iter().map(|&e| w[e as usize]).sum();
        report(&self.members);
        self.settled = false;
        self.stopped = false;
        loop {
            if self.value >= stop {
                self.stopped = true;
                return;
            }
            let Some(sink) = self.shortest_path(p) else {
                self.settled = true;
                self.gap = i64::MAX;
                return;
            };
            let gain = -self.dist[sink as usize].div_euclid(HOPS);
            if gain <= 0 {
                // The heaviest set of all sizes (even if it has `cap` members).
                self.settled = true;
                self.gap = -gain;
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

    /// The weight and size of a common independent set of the elements `live` (ascending) found greedily: `base` (common
    /// independent), then each other element by decreasing weight that keeps it so.  A lower bound on the heaviest
    /// common independent set, for a few reductions per element.
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
        self.slot[m][yi] = s as u8;
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
        self.slot[m][yi] = s as u8;
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
            self.generation = self.generation.wrapping_add(1);
            let generation = self.generation;
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
    pub fn split_bounds(&mut self, p: &Problem<'_, R>, live: &[u32], mut f: impl FnMut(u32, i64)) -> bool {
        debug_assert!(self.settled);
        let w = p.weight;
        // Potentials for the constraints w1(v) − w1(u) ≤ c, as arcs u → v of length c, from potentials all zero:
        // Bellman–Ford from a virtual source with an arc of length 0 to every vertex, whose distances are the same
        // whatever the order of the relaxations.  The zero node is `zero`; its arcs: to every member (w1(x) ≤ w(x))
        // and every element free in A (w1(y) ≤ 0), and from every member (w1(x) ≥ 0) and every element free in B
        // (w1(y) ≥ w(y)).  A member's other arcs go to the elements whose A-circuit holds it (w1(y) ≤ w1(x)), an
        // element's in the span of B to the members of its B-circuit (w2(y) ≤ w2(x)).  Each round relaxes the arcs
        // out of the vertices whose potential fell in the round before (every vertex in the first).
        if self.columns_for != self.tableau {
            self.build_columns();
        }
        let zero = self.pot.len() - 1;
        for &e in live {
            self.pot[e as usize] = 0;
        }
        self.pot[zero] = 0;
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
            self.generation = self.generation.wrapping_add(1);
            let generation = self.generation;
            self.changed_out.clear();
            let (pot, seen, next) = (&mut self.pot, &mut self.seen, &mut self.changed_out);
            let mut zero_fell = false;
            // Offers the potential `d` to `v`.
            macro_rules! relax {
                ($v:expr, $d:expr) => {{
                    let (v, d): (usize, i64) = ($v, $d);
                    if d < pot[v] {
                        pot[v] = d;
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
                        relax!(x as usize, pu + w[x as usize]);
                    }
                    for &y in &self.free_a {
                        relax!(y as usize, pu);
                    }
                } else if self.inside[u] {
                    relax!(zero, pu);
                    let column = usize::from(p.block[0][u]) * 64 * R + usize::from(self.slot[0][u]);
                    for &y in &self.columns[column] {
                        relax!(y as usize, pu);
                    }
                } else if !is_zero(&self.res[1][u]) {
                    relax!(zero, pu - w[u]);
                } else {
                    let elems = &self.slot_elem[1][usize::from(p.block[1][u])];
                    each_bit(&self.mask[1][u], |s| {
                        let x = elems[s] as usize;
                        relax!(x, pu + w[x] - w[u]);
                    });
                }
            }
            if zero_fell {
                next.push(zero as u32);
            }
            std::mem::swap(&mut self.changed_in, &mut self.changed_out);
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
    /// in `B` has an arc to every member, `x` included.  The search stops at the first distance that no cycle
    /// within `limit` and shorter than the best found can pass.
    pub fn without(&mut self, p: &Problem<'_, R>, pot: &[i64], x: u32, limit: i64, banned: &[u32]) -> Option<i64> {
        self.withouts += 1;
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
        if self.free_sorted_for != self.tableau {
            self.free_a.sort_unstable_by_key(|&y| (Reverse(pot[y as usize]), y));
            self.free_sorted_for = self.tableau;
        }
        let w = p.weight;
        let w1 = |e: u32| pot[e as usize];
        let w2 = |e: u32| w[e as usize] - pot[e as usize];
        // Two fresh stamps: reached (the distance in `reach` is current) and settled.
        self.generation = self.generation.wrapping_add(2);
        let (reached, settled) = (self.generation.wrapping_sub(1), self.generation);
        let (reach, back, seen, stamp) = (&mut self.reach, &mut self.back, &mut self.seen, &mut self.settled_at);
        let heap = &mut self.heap;
        heap.clear();
        // A banned element counts as settled: never offered, nor reached as an element free in A.
        for &e in banned {
            stamp[e as usize] = settled;
        }
        // Distances from `limit + 1` on cannot lead to a cycle that certifies.
        let bound = (limit + 1) * HOPS;
        let mut cycle = (bound, NONE);
        // The elements free in A, by decreasing w1: the next one unsettled, and the member through which they are
        // reached (the least distance plus w1, packed).
        let mut next_free = 0;
        let mut through = (INF, NONE);
        // Offers the distance `d` to `v` from `from`.
        macro_rules! offer {
            ($v:expr, $d:expr, $from:expr) => {{
                let (v, d): (u32, Dist) = ($v, $d);
                let i = v as usize;
                if d < bound && stamp[i] != settled && (seen[i] != reached || d < reach[i]) {
                    seen[i] = reached;
                    reach[i] = d;
                    back[i] = $from;
                    heap.push(Reverse((d, v)));
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
                        heap.push(Reverse((dy, FREE_A)));
                    }
                }
            }};
        }
        heap.push(Reverse((0, x)));
        seen[x as usize] = reached;
        reach[x as usize] = 0;
        back[x as usize] = NONE;
        while let Some(Reverse((d, v))) = heap.pop() {
            if d >= cycle.0 {
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
                if stamp[v as usize] == settled || d > reach[v as usize] {
                    continue;
                }
                stamp[v as usize] = settled;
                v
            };
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
                // Free in B: arcs to every member, x's closing a cycle.
                if d + (w2(x) - w2(v)) * HOPS + 1 < cycle.0 {
                    cycle = (d + (w2(x) - w2(v)) * HOPS + 1, v);
                }
                for &z in &self.members {
                    if z != x {
                        offer!(z, d + (w2(z) - w2(v)) * HOPS + 1, v);
                    }
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
        if cycle.1 == NONE {
            return None;
        }
        // The cycle's elements, and the set flipped along it checked common independent in the blocks it touches
        // (the others hold the same members as before).
        self.flip.clear();
        self.flip.push(x);
        let mut v = cycle.1;
        while v != x && v != NONE && self.flip.len() <= self.pos.len() {
            self.flip.push(v);
            v = self.back[v as usize];
        }
        if v != x || !self.flipped_independent(p) {
            debug_assert!(false, "the shortest cycle through a member flips into a dependent set");
            return None;
        }
        self.withouts_exact += 1;
        Some(self.value - cycle.0.div_euclid(HOPS))
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
        if self.free_sorted_for != self.tableau {
            self.free_a.sort_unstable_by_key(|&y| (Reverse(pot[y as usize]), y));
            self.free_sorted_for = self.tableau;
        }
        let w = p.weight;
        let w1 = |e: u32| pot[e as usize];
        let w2 = |e: u32| w[e as usize] - pot[e as usize];
        self.generation = self.generation.wrapping_add(2);
        let (reached, settled) = (self.generation.wrapping_sub(1), self.generation);
        let (reach, seen, stamp) = (&mut self.reach, &mut self.seen, &mut self.settled_at);
        let heap = &mut self.heap;
        heap.clear();
        let mut best = INF;
        macro_rules! offer {
            ($v:expr, $d:expr) => {{
                let (v, d): (u32, Dist) = ($v, $d);
                let i = v as usize;
                if d < best && stamp[i] != settled && (seen[i] != reached || d < reach[i]) {
                    seen[i] = reached;
                    reach[i] = d;
                    heap.push(Reverse((d, v)));
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
                    heap.push(Reverse((through - w1(y) * HOPS + 1, FREE_A)));
                }
            }};
        }
        for &z in &self.members {
            offer!(z, w2(z) * HOPS);
        }
        while let Some(Reverse((d, v))) = heap.pop() {
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
                for &z in &self.members {
                    offer!(z, d + (w2(z) - w2(v)) * HOPS + 1);
                }
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

    /// The cycle through the member that the last [`without`](Self::without) that answered found: its elements.
    pub fn cycle(&self) -> &[u32] {
        &self.flip
    }

    /// The weight split's `w1` by element after a successful [`split_bounds`](Self::split_bounds) (meaningful for
    /// the live elements).
    pub fn potentials(&self) -> &[i64] {
        &self.pot[..self.pot.len() - 1]
    }

    /// Whether the current set flipped on `self.flip` is independent in both matroids: in each block a flipped
    /// element belongs to, the members there that stay and the outside elements that come in, inserted in a span.
    fn flipped_independent(&mut self, p: &Problem<'_, R>) -> bool {
        self.generation = self.generation.wrapping_add(1);
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

    /// What a [`Problem`] of one word borrows: the blocks and vectors of both matroids, and the weights.
    struct Owned {
        block: [Vec<u8>; 2],
        vector: [Vec<Vector<1>>; 2],
        weight: Vec<i64>,
    }

    impl Owned {
        fn problem(&self) -> Problem<'_, 1> {
            Problem {
                block: [&self.block[0], &self.block[1]],
                vector: [&self.vector[0], &self.vector[1]],
                weight: &self.weight,
            }
        }
    }

    /// A random problem of `n` elements with blocks of dimension at most `dim` (one word).
    fn random(rng: &mut Rng, n: usize, dim: u64) -> Owned {
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

    fn independent(block: &[u8], vector: &[Vector<1>], set: &[u32]) -> bool {
        let mut spans = vec![BlockSpan::<1>::new(); 4];
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

    /// Against brute force: the heaviest set under every size limit, from the empty start and from every set
    /// a solve reported (dominant), with a member deleted; and the split's bound without each member.
    #[test]
    fn intersection_warm_starts_and_split_against_brute_force() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let (mut warm, mut splits) = (0, 0);
        for trial in 0..3000 {
            let n = 1 + rng.below(11) as usize;
            let owned = random(&mut rng, n, 5);
            let p = owned.problem();
            let ([ba, bb], [va, vb]) = (&owned.block, &owned.vector);
            let mut meet = Meet::<1>::new(n, [3, 3]);
            let live: Vec<u32> = (0..n as u32).collect();
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
                // The split, with no size limit.
                let value = meet.value;
                let mut bounds = Vec::new();
                if meet.settled {
                    assert!(meet.split_bounds(&p, &live, |x, b| bounds.push((x, b))), "trial {trial}: no split");
                    splits += 1;
                }
                for (x, b) in bounds {
                    let rest: Vec<u32> = live.iter().copied().filter(|&e| e != x).collect();
                    assert!(b >= brute(&p, &rest, n), "trial {trial}: the split's bound without {x}");
                    assert!(b <= value, "trial {trial}: the split's bound without {x} above the optimum");
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
                        warm += 1;
                    }
                }
            }
        }
        assert!(warm > 10_000 && splits > 2_000, "{warm} warm starts, {splits} splits");
    }

    /// The loss of the smaller sets ([`Meet::smaller_loss`], against the heaviest set one smaller, by brute force)
    /// and of the larger ones ([`Meet::gap`]), and the heaviest set without each member of the heaviest set read off
    /// its exchange graph where they certify it ([`Meet::without`]), under every size limit from the heaviest set's
    /// size up, against brute force.
    #[test]
    fn heaviest_set_without_a_member_against_brute_force() {
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        let (mut asked, mut certified, mut exact_losses) = (0, 0, 0);
        for trial in 0..4000 {
            let n = 2 + rng.below(10) as usize;
            let owned = random(&mut rng, n, 4);
            let p = owned.problem();
            let mut meet = Meet::<1>::new(n, [3, 3]);
            let live: Vec<u32> = (0..n as u32).collect();
            meet.solve(&p, &live, &[], n, i64::MAX, |_| {});
            let members: Vec<u32> = live.iter().copied().filter(|&e| meet.contains(e)).collect();
            let size = members.len();
            assert!(meet.settled && meet.split_bounds(&p, &live, |_, _| {}), "trial {trial}: no split");
            let pot = meet.potentials().to_vec();
            let loss = meet.smaller_loss(&p, &pot);
            if size == 0 {
                assert_eq!(loss, i64::MAX);
                continue;
            }
            let smaller = brute(&p, &live, size - 1);
            assert!(loss <= meet.value - smaller, "trial {trial}: the loss of the smaller sets");
            exact_losses += usize::from(loss == meet.value - smaller);
            // The heaviest common independent set of exactly one more element, if there is one.
            let larger = (0u32..1 << n)
                .filter(|mask| mask.count_ones() as usize == size + 1)
                .map(|mask| (0..n as u32).filter(|&i| mask >> i & 1 == 1).collect::<Vec<u32>>())
                .filter(|set| independent(p.block[0], p.vector[0], set) && independent(p.block[1], p.vector[1], set))
                .map(|set| set.iter().map(|&e| p.weight[e as usize]).sum::<i64>())
                .max();
            assert_eq!(meet.gap, larger.map_or(i64::MAX, |w| meet.value - w), "trial {trial}: the gap");
            for cap in size..=n {
                let limit = if cap > size { loss.min(meet.gap) } else { loss };
                for &x in &members {
                    asked += 1;
                    if let Some(value) = meet.without(&p, &pot, x, limit, &[]) {
                        certified += 1;
                        let rest: Vec<u32> = live.iter().copied().filter(|&e| e != x).collect();
                        assert_eq!(value, brute(&p, &rest, cap), "trial {trial}: without {x}, at most {cap}");
                    }
                }
            }
        }
        assert!(exact_losses > 3000, "{exact_losses} exact losses");
        assert!(asked > 10_000 && certified > asked / 4, "{certified} of {asked} certified");
    }
}
