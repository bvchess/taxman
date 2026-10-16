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
//!
//! # Forgetting is sound, and a policy
//!
//! Every nogood in the table holds at every node, so the table is a *cache*: a table that holds any subset of the
//! nogoods the search has learned (or of those it might learn) is as sound as a full one, and loses only the time it
//! takes to learn what it forgot.  So which nogoods a searcher keeps, and for how long, is a matter of speed alone, a
//! policy: [`Nogoods::forget`] drops all of them or keeps some ([`Keep`]), and a table may be limited
//! ([`Nogoods::set_limit`]) so that the least recently used goes when a new one comes.  (What the tree looks like does
//! depend on the policy, because which conflict a node branches on depends on what the table knows; the optimum does
//! not.)  A nogood is *used* when a lookup answers with it ([`Nogoods::lookup`]) or it rules a candidate out
//! ([`Nogoods::ruled_out`]).

use crate::bits::Bits;
use std::fmt;

/// What [`Nogoods::forget`] keeps of the table (the others are dropped); the used count of every kept nogood starts
/// again from 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    /// Nothing: the table is cleared.
    Nothing,
    /// The nogoods used since the table was last forgotten.
    Used,
    /// The nogoods of at most this many members.
    Short(u32),
    /// At most this many nogoods, the most used since the last forgetting (only those used at all, the most recently
    /// used first among equals).
    Top(u32),
}

impl Keep {
    /// The policy `text` names: `none`, `hit`, `size3` (the nogoods of at most 3 members), `top64` (the 64 most
    /// used).
    pub fn parse(text: &str) -> Option<Keep> {
        let number = |digits: &str| {
            Some(digits)
                .filter(|d| d.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|d| d.parse::<u32>().ok())
                .filter(|&n| n >= 1)
        };
        match text {
            "none" => Some(Keep::Nothing),
            "hit" => Some(Keep::Used),
            _ => match (text.strip_prefix("size"), text.strip_prefix("top")) {
                (Some(digits), _) => number(digits).map(Keep::Short),
                (_, Some(digits)) => number(digits).map(Keep::Top),
                _ => None,
            },
        }
    }
}

/// The policy as [`Keep::parse`] reads it.
impl fmt::Display for Keep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Keep::Nothing => f.write_str("none"),
            Keep::Used => f.write_str("hit"),
            Keep::Short(size) => write!(f, "size{size}"),
            Keep::Top(count) => write!(f, "top{count}"),
        }
    }
}

/// What a table has done, for measuring policies ([`Nogoods::take_stats`] hands it over and starts again).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NogoodStats {
    /// Nogoods added.
    pub learned: u64,
    /// Lookups of a table that held a nogood, and those answered (by a conflict, or by one all of whose members are
    /// forced).
    pub lookups: u64,
    pub hits: u64,
    /// Candidates ruled out of children.
    pub ruled_out: u64,
    /// [`Nogoods::forget`]s, the sizes of the tables they began with and the numbers of nogoods they kept (summed).
    pub forgettings: u64,
    pub size_before: u64,
    pub size_kept: u64,
    /// Nogoods dropped to make room under the limit.
    pub evicted: u64,
    /// The most nogoods a table held (not a sum: [`add`](Self::add) takes the larger).
    pub largest: u64,
}

impl NogoodStats {
    /// Adds `other` to these (the larger of the two for `largest`).
    pub fn add(&mut self, other: &NogoodStats) {
        self.learned += other.learned;
        self.lookups += other.lookups;
        self.hits += other.hits;
        self.ruled_out += other.ruled_out;
        self.forgettings += other.forgettings;
        self.size_before += other.size_before;
        self.size_kept += other.size_kept;
        self.evicted += other.evicted;
        self.largest = self.largest.max(other.largest);
    }

    /// One line: nogoods learned, lookups with the share answered, candidates ruled out, forgettings with the mean
    /// table before and the mean kept, evictions, the largest table.
    pub fn summary(&self) -> String {
        let mean = |sum: u64| if self.forgettings == 0 { 0.0 } else { sum as f64 / self.forgettings as f64 };
        format!(
            "nogoods learned {} lookups {} hit {:.1}% ruled out {} forgot {} (table {:.0} kept {:.0}) evicted {} largest {}",
            self.learned,
            self.lookups,
            100.0 * self.hits as f64 / self.lookups.max(1) as f64,
            self.ruled_out,
            self.forgettings,
            mean(self.size_before),
            mean(self.size_kept),
            self.evicted,
            self.largest,
        )
    }
}

/// The nogoods a searcher has learned (see the module docs).
///
/// The table holds at most `capacity` nogoods and forgets them all when it is full, and its owner may
/// [`clear`](Self::clear) it or [`forget`](Self::forget) some at any time (it is a cache: losing nogoods costs only
/// their rediscovery).  Lookups scan the whole table, so a larger one makes every node slower.
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
    /// For each nogood, how many times it has been used since the table was last forgotten, and when it was last used
    /// or learned (by `clock`, which counts the uses and the learnings).
    uses: Vec<u32>,
    last_used: Vec<u64>,
    clock: u64,
    /// With a limit, the table holds at most this many nogoods, and a new one takes the place of the least recently used.
    limit: Option<usize>,
    stats: NogoodStats,
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
            uses: Vec::new(),
            last_used: Vec::new(),
            clock: 0,
            limit: None,
            stats: NogoodStats::default(),
        }
    }

    /// Limits the table to `limit` nogoods (at least 1), or lifts the limit: a nogood added to a full table takes the place
    /// of the least recently used one (the first learned among equals).
    pub fn set_limit(&mut self, limit: Option<usize>) {
        self.limit = limit.map(|limit| limit.max(1));
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
    /// if the table is full, or the least recently used one if it is at its limit ([`set_limit`](Self::set_limit)).
    pub fn insert(&mut self, n: Bits<W>) {
        if self.sets.len() >= self.capacity {
            self.clear();
        }
        while self.limit.is_some_and(|limit| self.sets.len() >= limit) {
            let oldest = (0..self.sets.len()).min_by_key(|&id| self.last_used[id]).expect("a full table");
            let mut keep = vec![true; self.sets.len()];
            keep[oldest] = false;
            self.retain(&keep);
            self.stats.evicted += 1;
        }
        self.push(n);
        self.stats.learned += 1;
        self.stats.largest = self.stats.largest.max(self.sets.len() as u64);
    }

    /// Adds `n` as the newest nogood, used just now.
    fn push(&mut self, n: Bits<W>) {
        let id = self.sets.len() as u32;
        self.sets.push(n);
        for x in n.iter() {
            self.by_member[x as usize].push(id);
            self.index_member(x);
        }
        self.maxes.push(n.max().expect("a nogood is not empty") as u16);
        let set = self.compact_of(&n);
        self.compact.extend_from_slice(&set[..self.stride]);
        self.clock += 1;
        self.uses.push(0);
        self.last_used.push(self.clock);
    }

    /// Keeps the nogoods that `keep` flags (one flag for each, in the order of the table), in the order they were
    /// learned and with their uses.
    fn retain(&mut self, keep: &[bool]) {
        let kept: Vec<(Bits<W>, u32, u64)> = (0..self.sets.len())
            .filter(|&id| keep[id])
            .map(|id| (self.sets[id], self.uses[id], self.last_used[id]))
            .collect();
        self.clear();
        for (n, uses, last_used) in kept {
            self.push(n);
            let id = self.sets.len() - 1;
            (self.uses[id], self.last_used[id]) = (uses, last_used);
        }
    }

    /// Forgets the nogoods that `keep` does not keep, and starts counting the uses of the others from 0.
    pub fn forget(&mut self, keep: Keep) {
        self.stats.forgettings += 1;
        self.stats.size_before += self.sets.len() as u64;
        match keep {
            Keep::Nothing => self.clear(),
            Keep::Used => {
                let mask: Vec<bool> = self.uses.iter().map(|&uses| uses > 0).collect();
                self.retain(&mask);
            }
            Keep::Short(size) => {
                let mask: Vec<bool> = self.sets.iter().map(|n| n.len() <= size).collect();
                self.retain(&mask);
            }
            Keep::Top(count) => {
                let mut used: Vec<usize> = (0..self.sets.len()).filter(|&id| self.uses[id] > 0).collect();
                used.sort_by_key(|&id| (std::cmp::Reverse(self.uses[id]), std::cmp::Reverse(self.last_used[id])));
                let mut mask = vec![false; self.sets.len()];
                for &id in used.iter().take(count as usize) {
                    mask[id] = true;
                }
                self.retain(&mask);
            }
        }
        self.uses.fill(0);
        self.stats.size_kept += self.sets.len() as u64;
    }

    /// What the table has done since the last call (the table's size now counts as the largest it has been since).
    pub fn take_stats(&mut self) -> NogoodStats {
        let taken = self.stats;
        self.stats = NogoodStats { largest: self.sets.len() as u64, ..NogoodStats::default() };
        taken
    }

    /// Records that nogood `id` has been used.
    fn use_nogood(&mut self, id: u32) {
        self.clock += 1;
        self.uses[id as usize] += 1;
        self.last_used[id as usize] = self.clock;
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
        self.uses.clear();
        self.last_used.clear();
    }

    /// The smallest conflict the table knows of a node with the promotions `promoted` among the candidates
    /// `set` (disjoint from `promoted`): the smallest `N − promoted` over the nogoods `N ⊆ promoted ∪ set`, the
    /// one with the largest member if there are several (then the first learned); `None` if there is none.
    pub fn conflict(&mut self, promoted: &Bits<W>, set: &Bits<W>) -> Option<Bits<W>> {
        match self.lookup(promoted, set, &Bits::EMPTY) {
            Lookup::Conflict(k) => Some(k),
            _ => None,
        }
    }

    /// [`conflict`](Self::conflict), unless one of the conflicts the table knows is a subset of `forced` (it then
    /// stops there and says [`Lookup::AllForced`]).  The search passes a node's forced members: every completion
    /// that can beat the incumbent promotes all of them, so if they contain a conflict there is no such
    /// completion.  The nogood that answers is used.
    pub fn lookup(&mut self, promoted: &Bits<W>, set: &Bits<W>, forced: &Bits<W>) -> Lookup<W> {
        if self.sets.is_empty() {
            return Lookup::None;
        }
        self.stats.lookups += 1;
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
        let (answer, id) = match best {
            Err(id) => (Lookup::AllForced, id),
            Ok(Some(id)) => (Lookup::Conflict(self.sets[id as usize] - *promoted), id),
            Ok(None) => return Lookup::None,
        };
        self.stats.hits += 1;
        self.use_nogood(id);
        answer
    }

    /// The scan of [`lookup`](Self::lookup) over the compact sets, whose stride is `S` (0: any): with
    /// `[within, promoted, forced]` over the compact indices, the nogood with the smallest conflict (then the
    /// largest member, then the first learned), or `Err` of the first nogood within whose conflict is forced.
    #[inline]
    fn scan<const S: usize>(&self, [within, p, f]: [&[u64; W]; 3]) -> Result<Option<u32>, u32> {
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
                return Err(id as u32);
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
    /// candidate.  Each nogood that rules a candidate out is used.
    pub fn ruled_out(&mut self, added: &Bits<W>, promoted: &Bits<W>, cands: &Bits<W>) -> Bits<W> {
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
        self.stats.ruled_out += u64::from(out.len());
        out
    }

    /// [`ruled_out`](Self::ruled_out) with the promotions `p` over the compact indices, whose stride is `S` (0:
    /// any), adding to `out`.
    #[inline]
    fn rule_out<const S: usize>(&mut self, added: &Bits<W>, p: &[u64; W], cands: &Bits<W>, out: &mut Bits<W>) {
        let stride = if S == 0 { self.stride } else { S };
        debug_assert_eq!(stride, self.stride);
        let p = &p[..stride];
        let Nogoods { by_member, compact, numbers, uses, last_used, clock, .. } = self;
        for a in added.iter() {
            let Some(ids) = by_member.get(a as usize) else { continue };
            for &id in ids {
                let n = &compact[id as usize * stride..][..stride];
                // How many members are not promoted, and the last of them.
                let (mut count, mut at) = (0, 0);
                for (k, (n, p)) in n.iter().zip(p).enumerate() {
                    let rest = n & !p;
                    count += rest.count_ones();
                    if rest != 0 {
                        at = (k << 6) | rest.trailing_zeros() as usize;
                    }
                }
                if count == 1 && cands.contains(u32::from(numbers[at])) {
                    out.insert(u32::from(numbers[at]));
                    *clock += 1;
                    uses[id as usize] += 1;
                    last_used[id as usize] = *clock;
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
