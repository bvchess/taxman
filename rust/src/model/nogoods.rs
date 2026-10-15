//! The table of nogoods a searcher learns (tactic `nogoods`).
//!
//! A *nogood* is a conflict of the search's root: a set of candidates that cannot all be promoted from the root.
//! Unlike a conflict found at a node, which is relative to the node's promotions, a nogood holds everywhere in
//! the search:
//!
//! * A node's state is the root with its promotions `P` promoted, and promoting a set `F` from it succeeds
//!   exactly when `P ∪ F` can be promoted from the root: both are decided frame by frame on the union (see
//!   [`FrameGraph::winnable`](super::FrameGraph::winnable); reducing the frames in between changes nothing).
//! * A set that cannot be promoted from the root makes every superset fail too (a subset of a promotable set is
//!   promotable).
//!
//! So for a nogood `N`, every node has `N − P` as a conflict: no completion of the node promotes all of it.
//! This depends on nothing but the root and the numbers in `N`: not on the path, the incumbent, the thread, or
//! a checkpoint (a resumed search simply starts with none).
//!
//! A conflict `K` found at a node with promotions `P` becomes a nogood by
//! [`FrameGraph::nogood`](super::FrameGraph::nogood): a minimal subset of `P ∪ K` that keeps one of the root's frames
//! unwinnable.  The search uses the table for the conflict to branch on and to refine with (the smallest it
//! knows, [`Nogoods::lookup`]) and to rule candidates out of children ([`Nogoods::ruled_out`]).

use crate::bits::Bits;

/// The nogoods a searcher has learned (see the module docs).
///
/// The table holds at most `capacity` nogoods and forgets them all when it is full, and its owner may
/// [`clear`](Self::clear) it at any time (it is a cache: losing nogoods costs only their rediscovery).  Lookups
/// scan the whole table, so a larger one makes every node slower.
///
/// For the scans, the numbers that are members of the nogoods get compact indices, in the order they first
/// appear (a few hundred at most: they are all candidates), and each nogood is also kept as a set of those, a
/// few words long: a nogood is tested against a set in a few word operations, exactly.
pub struct Nogoods<const W: usize> {
    /// The nogoods, by index.
    pub(super) sets: Vec<Bits<W>>,
    /// For each nogood, its largest member.
    maxes: Vec<u16>,
    /// For each number, the nogoods it is a member of.
    by_member: Vec<Vec<u32>>,
    capacity: usize,
    /// For each number, its compact index plus one (0: not a member of any nogood); and the members by index.
    index: Vec<u16>,
    numbers: Vec<u16>,
    /// Each nogood as a set of compact indices, `stride` words each (as many as the indices need).
    compact: Vec<u64>,
    stride: usize,
}

impl<const W: usize> Nogoods<W> {
    /// An empty table for nogoods of numbers up to `max_number`, holding at most `capacity` of them.
    pub fn new(max_number: u32, capacity: usize) -> Self {
        let size = max_number as usize + 1;
        Nogoods {
            sets: Vec::new(),
            maxes: Vec::new(),
            by_member: vec![Vec::new(); size],
            capacity,
            index: vec![0; size],
            numbers: Vec::new(),
            compact: Vec::new(),
            stride: 1,
        }
    }

    /// `set` over the compact indices, in its first [`stride`](Self::stride) words (members that are in no
    /// nogood are left out).  Compact indices are fewer than the members of the nogoods, so `W` words hold them.
    #[inline]
    fn compact_of(&self, set: &Bits<W>) -> [u64; W] {
        let mut out = [0u64; W];
        for x in set.iter() {
            let i = usize::from(self.index.get(x as usize).copied().unwrap_or(0));
            if i != 0 {
                out[(i - 1) >> 6] |= 1 << ((i - 1) & 63);
            }
        }
        out
    }

    /// Gives `x` a compact index if it has none, making the compact sets a word longer when they need it.
    fn index_member(&mut self, x: u32) {
        if self.index[x as usize] != 0 {
            return;
        }
        self.numbers.push(x as u16);
        self.index[x as usize] = self.numbers.len() as u16;
        if self.numbers.len() > 64 * self.stride {
            let wider = self.stride + 1;
            let mut compact = Vec::with_capacity(self.sets.len() * wider);
            for set in self.compact.chunks_exact(self.stride) {
                compact.extend_from_slice(set);
                compact.push(0);
            }
            (self.compact, self.stride) = (compact, wider);
        }
    }

    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// The nogoods, in the order they were learned.
    pub fn sets(&self) -> &[Bits<W>] {
        &self.sets
    }

    /// Adds the nogood `n` (non-empty, members up to the table's `max_number`), forgetting every other one first
    /// if the table is full.
    pub fn insert(&mut self, n: Bits<W>) {
        if self.sets.len() >= self.capacity {
            self.clear();
        }
        let id = self.sets.len() as u32;
        self.sets.push(n);
        for x in n.iter() {
            self.by_member[x as usize].push(id);
            self.index_member(x);
        }
        self.maxes.push(n.max().expect("a nogood is not empty") as u16);
        let set = self.compact_of(&n);
        self.compact.extend_from_slice(&set[..self.stride]);
    }

    /// Forgets every nogood.
    pub fn clear(&mut self) {
        for n in &self.sets {
            for x in n.iter() {
                self.by_member[x as usize].clear();
            }
        }
        for &x in &self.numbers {
            self.index[usize::from(x)] = 0;
        }
        self.numbers.clear();
        self.compact.clear();
        self.stride = 1;
        self.sets.clear();
        self.maxes.clear();
    }

    /// The smallest conflict the table knows of a node with the promotions `promoted` among the candidates
    /// `set` (disjoint from `promoted`): the smallest `N − promoted` over the nogoods `N ⊆ promoted ∪ set`, the
    /// one with the largest member if there are several (then the first learned); `None` if there is none.
    pub fn conflict(&self, promoted: &Bits<W>, set: &Bits<W>) -> Option<Bits<W>> {
        match self.lookup(promoted, set, &Bits::EMPTY) {
            Lookup::Conflict(k) => Some(k),
            _ => None,
        }
    }

    /// [`conflict`](Self::conflict), unless one of the conflicts the table knows is a subset of `forced` (it then
    /// stops there and says [`Lookup::AllForced`]).  The search passes a node's forced members: every completion
    /// that can beat the incumbent promotes all of them, so if they contain a conflict there is no such
    /// completion.
    pub fn lookup(&self, promoted: &Bits<W>, set: &Bits<W>, forced: &Bits<W>) -> Lookup<W> {
        if self.sets.is_empty() {
            return Lookup::None;
        }
        // The three sets over the compact indices.
        let stride = self.stride;
        let (p, f) = (self.compact_of(promoted), self.compact_of(forced));
        let mut within = self.compact_of(set);
        for (w, p) in within.iter_mut().zip(&p).take(stride) {
            *w |= p;
        }
        // A scan whose stride the compiler knows is unrolled.
        let sets = [&within, &p, &f];
        let best = match stride {
            1 => self.scan::<1>(sets),
            2 => self.scan::<2>(sets),
            3 => self.scan::<3>(sets),
            4 => self.scan::<4>(sets),
            5 => self.scan::<5>(sets),
            6 => self.scan::<6>(sets),
            _ => self.scan::<0>(sets),
        };
        match best {
            Err(()) => Lookup::AllForced,
            Ok(Some(id)) => Lookup::Conflict(self.sets[id as usize] - *promoted),
            Ok(None) => Lookup::None,
        }
    }

    /// The scan of [`lookup`](Self::lookup) over the compact sets, whose stride is `S` (0: any): with
    /// `[within, promoted, forced]` over the compact indices, the nogood with the smallest conflict (then the
    /// largest member, then the first learned), or `Err` at the first nogood within whose conflict is forced.
    #[inline]
    fn scan<const S: usize>(&self, [within, p, f]: [&[u64; W]; 3]) -> Result<Option<u32>, ()> {
        let stride = if S == 0 { self.stride } else { S };
        debug_assert_eq!(stride, self.stride);
        let (within, p, f) = (&within[..stride], &p[..stride], &f[..stride]);
        let mut best: Option<(u32, u16, u32)> = None; // (size of the conflict, largest member, nogood)
        for (id, n) in self.compact.chunks_exact(stride).enumerate() {
            if n.iter().zip(within).fold(0, |out, (n, w)| out | n & !w) != 0 {
                continue;
            }
            let (mut size, mut unforced) = (0, 0);
            for ((n, p), f) in n.iter().zip(p).zip(f) {
                let rest = n & !p;
                size += rest.count_ones();
                unforced |= rest & !f;
            }
            if unforced == 0 {
                return Err(());
            }
            let (id, top) = (id as u32, self.maxes[id]);
            if best.is_none_or(|(s, m, _)| size < s || (size == s && top > m)) {
                best = Some((size, top, id));
            }
        }
        Ok(best.map(|(_, _, id)| id))
    }

    /// The candidates among `cands` that a node with the promotions `promoted` cannot promote by the nogoods
    /// that have a member in `added` (the node's newest promotions): those `N` with `N − promoted` a single
    /// candidate.
    pub fn ruled_out(&self, added: &Bits<W>, promoted: &Bits<W>, cands: &Bits<W>) -> Bits<W> {
        let mut out = Bits::EMPTY;
        let has_member = |a: u32| self.by_member.get(a as usize).is_some_and(|ids| !ids.is_empty());
        if !added.iter().any(has_member) {
            return out;
        }
        let p = self.compact_of(promoted);
        // A loop whose stride the compiler knows is unrolled.
        match self.stride {
            1 => self.rule_out::<1>(added, &p, cands, &mut out),
            2 => self.rule_out::<2>(added, &p, cands, &mut out),
            3 => self.rule_out::<3>(added, &p, cands, &mut out),
            4 => self.rule_out::<4>(added, &p, cands, &mut out),
            5 => self.rule_out::<5>(added, &p, cands, &mut out),
            6 => self.rule_out::<6>(added, &p, cands, &mut out),
            _ => self.rule_out::<0>(added, &p, cands, &mut out),
        }
        out
    }

    /// [`ruled_out`](Self::ruled_out) with the promotions `p` over the compact indices, whose stride is `S` (0:
    /// any), adding to `out`.
    #[inline]
    fn rule_out<const S: usize>(&self, added: &Bits<W>, p: &[u64; W], cands: &Bits<W>, out: &mut Bits<W>) {
        let stride = if S == 0 { self.stride } else { S };
        debug_assert_eq!(stride, self.stride);
        let p = &p[..stride];
        for a in added.iter() {
            let Some(ids) = self.by_member.get(a as usize) else { continue };
            for &id in ids {
                let n = &self.compact[id as usize * stride..][..stride];
                // How many members are not promoted, and the last of them.
                let (mut count, mut at) = (0, 0);
                for (k, (n, p)) in n.iter().zip(p).enumerate() {
                    let rest = n & !p;
                    count += rest.count_ones();
                    if rest != 0 {
                        at = (k << 6) | rest.trailing_zeros() as usize;
                    }
                }
                if count == 1 && cands.contains(u32::from(self.numbers[at])) {
                    out.insert(u32::from(self.numbers[at]));
                }
            }
        }
    }
}

/// What [`Nogoods::lookup`] found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lookup<const W: usize> {
    /// No conflict.
    None,
    /// The smallest conflict.
    Conflict(Bits<W>),
    /// A conflict all of whose members are in the set of forced members the lookup was given.
    AllForced,
}
