//! A store of nogoods carried into a search from another search (tactic `carried_nogoods`).
//!
//! A nogood of a search's root ([`Nogoods`](super::Nogoods)) depends on the root alone: it is a set of candidates that
//! cannot all be promoted from it.  The searches of neighboring games learn many of the same ones, because their frames
//! are nearly the same.  A store holds such sets, learned anywhere (by the search of the game before, say), but keeps
//! only those that [`validated`](CarriedNogoods::validated) shows to be nogoods of *this* root: a set whose members are
//! all candidates, and within which a peel at the root finds a conflict ([`FrameGraph::conflict_using`], the check the
//! search learns its own conflicts by).  What it keeps is that conflict: a subset of the set that leaves one of the
//! root's frames unwinnable, and is minimal for that frame.  So every nogood of a store is a fact of the root it was validated at, whatever its source,
//! and at every node with promotions `P`, `N − P` is a conflict of the node (see [`Nogoods`](super::Nogoods)).
//!
//! The search uses a store only to prune (solver/docs/design/carried-nogoods.md): [`start`](CarriedNogoods::start) and
//! [`promote`](CarriedNogoods::promote) drop from a unit's node and from each child the candidates that would complete
//! a nogood, and [`all_forced`](CarriedNogoods::all_forced) ends a node whose forced members complete one.  It never
//! gives the conflict a node branches on, which stays the searcher's own table's.  A carried table that also chose
//! the conflict steered the branching and lost:
//! solver/docs/design/carried-nogoods.md.
//!
//! # Watched members
//!
//! A store is read only and shared by the searchers.  Each searcher keeps [`Watches`] of it along its path, the scheme
//! of the two watched literals of SAT solvers: each nogood *watches* two of its members that the node has not promoted.
//! A nogood that watches two such members has two members left to promote, so no child can complete it by promoting
//! one, and only a promotion of a watched member needs a look at it.  Then the nogood watches another member that is
//! not promoted, if it has one; if it has none, its other watched member is the one left, and the child cannot promote
//! it.  So a child costs the nogoods that watch what it promotes, not a scan of the store, and a backtrack costs
//! nothing: members that a node has not promoted are not promoted at its ancestors either, and the watched member that
//! a nogood keeps although it is promoted is one that the child promoted, so the ancestors and the other children have
//! not promoted it (or they promote it again, and look at the nogood then).  A nogood watches the rarest of its members
//! that it can, those in the fewest nogoods of the store, which the paths that many nodes take promote least.
//!
//! A store's file is text, one nogood a line, its members in decimal separated by spaces; blank lines are skipped.  A
//! *directory of stores* holds the file of each game whose search wrote one, `nogoods-N.txt` for game N
//! ([`store_file`]), and a search reads those of its own game and the games before it
//! ([`CarriedNogoods::from_dir`]).  A search writes its own as its searchers learn ([`Harvest`]).

use super::{FrameGraph, FrameSet, Scratch};
use crate::bits::Bits;
use std::collections::{HashSet, VecDeque};
use std::io;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

/// Most sets a search reads from a directory of stores ([`CarriedNogoods::from_dir`]): checking each at the root takes a
/// fraction of a millisecond, and the search starts after it.
pub const DIR_CAP: usize = 20_000;

/// The file of game `n`'s store in the directory of stores `dir`.
pub fn store_file(dir: &Path, n: u32) -> PathBuf {
    dir.join(format!("nogoods-{n}.txt"))
}

/// The games whose stores the directory `dir` holds, and their files, the latest game first (none if it cannot be
/// read).
fn stores_in(dir: &Path) -> Vec<(u32, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut found: Vec<(u32, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let n = name.strip_prefix("nogoods-")?.strip_suffix(".txt")?.parse().ok()?;
            Some((n, entry.path()))
        })
        .collect();
    found.sort_by_key(|&(n, _)| std::cmp::Reverse(n));
    found
}

/// The nogoods a search's searchers learn, which it writes as its game's store ([`store_file`]): each searcher hands
/// the nogoods of its table on when it forgets them and when it ends a unit ([`add`](Self::add)), and the harvest keeps
/// them distinct, the latest [`DIR_CAP`] of them.  Every nogood a searcher learns is a nogood of the search's root, so
/// the store is facts of that root; a later search checks them at its own root anyway.
pub struct Harvest<const W: usize> {
    sets: Mutex<(VecDeque<Bits<W>>, HashSet<Bits<W>>)>,
}

impl<const W: usize> Harvest<W> {
    /// An empty harvest.
    pub fn new() -> Self {
        Harvest { sets: Mutex::new((VecDeque::new(), HashSet::new())) }
    }

    /// The harvest that starts with the sets of the store `path` whose members are all among `candidates` (a store the
    /// game's search wrote in an earlier run of it, so that the store it writes keeps them), or an empty one.
    pub fn resumed(path: &Path, candidates: &Bits<W>) -> Self {
        let harvest = Self::new();
        let text = std::fs::read_to_string(path).unwrap_or_default();
        if let Ok(sets) = CarriedNogoods::<W>::parse(&text) {
            let kept: Vec<Bits<W>> =
                sets.iter().filter_map(|set| CarriedNogoods::candidates_of(set, candidates)).collect();
            harvest.add(&kept);
        }
        harvest
    }

    /// Adds `nogoods`, those it does not hold yet, as the latest; past [`DIR_CAP`] the earliest go.
    pub fn add(&self, nogoods: &[Bits<W>]) {
        let mut guard = self.sets.lock().unwrap_or_else(PoisonError::into_inner);
        let (order, seen) = &mut *guard;
        for n in nogoods {
            if !n.is_empty() && seen.insert(*n) {
                order.push_back(*n);
                if order.len() > DIR_CAP {
                    let earliest = order.pop_front().expect("a full harvest");
                    seen.remove(&earliest);
                }
            }
        }
    }

    /// The nogoods held, the earliest first.
    pub fn sets(&self) -> Vec<Bits<W>> {
        self.sets.lock().unwrap_or_else(PoisonError::into_inner).0.iter().copied().collect()
    }

    /// Writes the nogoods held to `path` as a store (see the module docs): to a file beside it first, then renamed onto
    /// it, so that a search that reads it meanwhile reads the old store or the new one, whole.  Nothing is written
    /// while the harvest is empty.
    pub fn write(&self, path: &Path) -> io::Result<()> {
        let sets = self.sets();
        if sets.is_empty() {
            return Ok(());
        }
        let mut text = String::new();
        for n in &sets {
            let members: Vec<String> = n.iter().map(|x| x.to_string()).collect();
            text.push_str(&members.join(" "));
            text.push('\n');
        }
        let mut name = path.as_os_str().to_owned();
        name.push(".tmp");
        let partial = PathBuf::from(name);
        std::fs::write(&partial, text)?;
        std::fs::rename(&partial, path)
    }
}

impl<const W: usize> Default for Harvest<W> {
    fn default() -> Self {
        Self::new()
    }
}

/// The nogoods carried into a search, validated at its root (see the module docs).
pub struct CarriedNogoods<const W: usize> {
    /// The members of the nogoods, nogood after nogood: nogood `i` is `members[starts[i]..starts[i + 1]]`.  Each
    /// nogood lists its members from the one in the fewest nogoods of the store (then the smallest), the order in which
    /// it chooses the members it watches (see the module docs).
    members: Vec<u32>,
    starts: Vec<usize>,
    /// One more than the largest member.
    top: usize,
    /// The members of the nogoods of one member: candidates that cannot be promoted from the root at all.
    singles: Bits<W>,
    /// What the searchers did with the store, as they add it ([`add_counts`](Self::add_counts)): candidates ruled out,
    /// and nodes ended.
    ruled_out: AtomicU64,
    cuts: AtomicU64,
}

/// The members that each nogood of a store watches, as a searcher keeps them along its path (see the module docs of
/// `model::carried`).  The watches are *valid* for a set of promotions `P` when each nogood of two or more members
/// watches two members not in `P`, or watches one member not in `P` with all its others in `P`; the store's
/// [`start`](CarriedNogoods::start) makes them valid for a unit's node, and [`promote`](CarriedNogoods::promote) for a
/// child from its parent's.
pub struct Watches {
    /// For each nogood, the members it watches (the same twice for a nogood of one member).
    watched: Vec<[u32; 2]>,
    /// For each number, the nogoods that watch it.
    watching: Vec<Vec<u32>>,
}

/// What [`CarriedNogoods::validated`] made of the sets it was given.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Validation {
    /// The sets given.
    pub given: usize,
    /// Those with a member that is not a candidate of the root (no node can promote it, so the set teaches nothing).
    pub not_candidates: usize,
    /// Those that can be promoted from the root: no nogood of it.
    pub promotable: usize,
    /// The nogoods kept, distinct; and how many of them are smaller than the set they came from.
    pub kept: usize,
    pub shrunk: usize,
}

impl<const W: usize> CarriedNogoods<W> {
    /// The store of the nogoods of `root` (with `candidates`) that `sets` hold: of each set whose members are all
    /// candidates, the conflict a peel at the root finds within it, if any (see the module docs), each kept
    /// once.
    pub fn validated(
        model: &FrameGraph,
        root: &FrameSet<W>,
        candidates: &Bits<W>,
        sets: &[Vec<u32>],
    ) -> (Self, Validation) {
        let mut validation = Validation { given: sets.len(), ..Validation::default() };
        let mut scratch = Scratch::new();
        let mut seen = HashSet::new();
        let mut kept = Vec::new();
        for set in sets {
            let Some(bits) = Self::candidates_of(set, candidates) else {
                validation.not_candidates += 1;
                continue;
            };
            match model.conflict_using(&mut scratch, root, &bits, true) {
                None => validation.promotable += 1,
                Some(nogood) => {
                    if seen.insert(nogood) {
                        kept.push(nogood);
                        validation.shrunk += usize::from(nogood != bits);
                    }
                }
            }
        }
        validation.kept = kept.len();
        (Self::of(&kept), validation)
    }

    /// `set` as a set of candidates, if every member is one.
    fn candidates_of(set: &[u32], candidates: &Bits<W>) -> Option<Bits<W>> {
        let limit = candidates.max()?;
        let mut bits = Bits::EMPTY;
        for &x in set {
            if x > limit || !candidates.contains(x) {
                return None;
            }
            bits.insert(x);
        }
        Some(bits)
    }

    /// The store of `nogoods`, which must be nogoods of the root the search starts from, none of them empty: the
    /// caller's to know ([`validated`](Self::validated) makes such a list).
    pub(super) fn of(nogoods: &[Bits<W>]) -> Self {
        let top = nogoods.iter().filter_map(Bits::max).max().map_or(0, |x| x as usize + 1);
        let mut store = CarriedNogoods {
            members: Vec::new(),
            starts: vec![0],
            top,
            singles: Bits::EMPTY,
            ruled_out: AtomicU64::new(0),
            cuts: AtomicU64::new(0),
        };
        let mut shared = vec![0u32; top];
        for x in nogoods.iter().flat_map(Bits::iter) {
            shared[x as usize] += 1;
        }
        for nogood in nogoods {
            assert!(!nogood.is_empty(), "an empty nogood");
            let mut members: Vec<u32> = nogood.iter().collect();
            members.sort_by_key(|&x| (shared[x as usize], x));
            store.members.extend(members);
            store.starts.push(store.members.len());
            if nogood.len() == 1 {
                store.singles |= *nogood;
            }
        }
        store
    }

    /// The sets of a store's file (see the module docs), not yet validated.
    pub fn parse(text: &str) -> Result<Vec<Vec<u32>>, String> {
        let mut sets = Vec::new();
        for (i, line) in text.lines().enumerate() {
            let set: Vec<u32> = line
                .split_whitespace()
                .map(|x| x.parse::<u32>().map_err(|e| format!("line {}: {x:?}: {e}", i + 1)))
                .collect::<Result<_, _>>()?;
            if !set.is_empty() {
                sets.push(set);
            }
        }
        Ok(sets)
    }

    /// The store of the nogoods in the file `path`, validated at `root` ([`validated`](Self::validated)).
    pub fn read(
        path: &Path,
        model: &FrameGraph,
        root: &FrameSet<W>,
        candidates: &Bits<W>,
    ) -> Result<(Self, Validation), String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let sets = Self::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self::validated(model, root, candidates, &sets))
    }

    /// The store of game `n` from the directory of stores `dir`: the sets of the files of game `n` (written by an
    /// earlier run of its search) and of the games before it, the latest game's first, at most [`DIR_CAP`] of them,
    /// validated at `root` ([`validated`](Self::validated)); and the files it read.  A file that cannot be read or
    /// parsed is passed over, with a warning on stderr: a store is only ever a help.
    pub fn from_dir(
        dir: &Path,
        n: u32,
        model: &FrameGraph,
        root: &FrameSet<W>,
        candidates: &Bits<W>,
    ) -> (Self, Validation, Vec<PathBuf>) {
        let (mut sets, mut read) = (Vec::new(), Vec::new());
        for (_, path) in stores_in(dir).into_iter().filter(|&(m, _)| m <= n) {
            if sets.len() >= DIR_CAP {
                break;
            }
            let parsed = std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|text| Self::parse(&text));
            match parsed {
                Ok(more) => {
                    sets.extend(more.into_iter().take(DIR_CAP - sets.len()));
                    read.push(path);
                }
                Err(e) => eprintln!("warning: carried nogoods: {} passed over: {e}", path.display()),
            }
        }
        let (store, validation) = Self::validated(model, root, candidates, &sets);
        (store, validation, read)
    }

    /// The number of nogoods.
    pub fn len(&self) -> usize {
        self.starts.len() - 1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The nogoods, in the order they were kept.
    pub fn sets(&self) -> impl Iterator<Item = Bits<W>> + '_ {
        self.starts.windows(2).map(|w| self.members[w[0]..w[1]].iter().copied().collect())
    }

    /// The members of nogood `id`.
    #[inline]
    fn nogood(&self, id: u32) -> &[u32] {
        let id = id as usize;
        &self.members[self.starts[id]..self.starts[id + 1]]
    }

    /// Watches of the store, to be made valid by [`start`](Self::start) before any other use.
    pub fn watches(&self) -> Watches {
        Watches { watched: vec![[0, 0]; self.len()], watching: vec![Vec::new(); self.top] }
    }

    /// Makes `watches` valid for a unit's node with the promotions `promoted` (see [`Watches`]), and returns the
    /// candidates among `cands` that it cannot promote by the store: the members of the nogoods of one member, and the
    /// member left of each nogood all of whose other members are promoted.
    pub fn start(&self, watches: &mut Watches, promoted: &Bits<W>, cands: &Bits<W>) -> Bits<W> {
        for list in &mut watches.watching {
            list.clear();
        }
        let mut out = self.singles & *cands;
        for id in 0..self.len() as u32 {
            let nogood = self.nogood(id);
            let mut open = nogood.iter().copied().filter(|&x| !promoted.contains(x));
            let pair = match (open.next(), open.next()) {
                (Some(a), Some(b)) => [a, b],
                (Some(a), None) => {
                    // One member left: the node cannot promote it.  The nogood watches it and a promoted member.
                    if cands.contains(a) {
                        out.insert(a);
                    }
                    [a, nogood.iter().copied().find(|&x| x != a).unwrap_or(a)]
                }
                // Every member promoted: a unit's promotions cannot be, as they are promotable.
                _ => [nogood[0], nogood[nogood.len().min(2) - 1]],
            };
            watches.watched[id as usize] = pair;
            watches.watching[pair[0] as usize].push(id);
            if pair[1] != pair[0] {
                watches.watching[pair[1] as usize].push(id);
            }
        }
        out
    }

    /// Makes `watches`, valid for a node, valid for its child with the promotions `promoted`, the node's and `added`,
    /// and returns the candidates among `cands` that the child cannot promote by the nogoods that watched a member of
    /// `added`: each such nogood whose other members are all promoted but the one it also watches.  At a child of a node
    /// whose own candidates the store has narrowed (from its unit's node down), no other nogood rules a candidate out:
    /// one that watches two members not promoted has two left.
    pub fn promote(&self, watches: &mut Watches, added: &Bits<W>, promoted: &Bits<W>, cands: &Bits<W>) -> Bits<W> {
        let mut out = Bits::EMPTY;
        let Watches { watched, watching } = watches;
        for x in added.iter() {
            let Some(list) = watching.get_mut(x as usize) else { continue };
            let mut list = mem::take(list);
            list.retain(|&id| {
                let [a, b] = watched[id as usize];
                let other = if a == x { b } else { a };
                match self.nogood(id).iter().copied().find(|&y| y != other && !promoted.contains(y)) {
                    Some(y) => {
                        watched[id as usize] = [other, y];
                        watching[y as usize].push(id);
                        false
                    }
                    None => {
                        // All promoted but `other`: the child cannot promote it.  The nogood keeps watching `x`.
                        if !promoted.contains(other) && cands.contains(other) {
                            out.insert(other);
                        }
                        true
                    }
                }
            });
            watching[x as usize] = list;
        }
        out
    }

    /// Whether some nogood lies within `promoted ∪ forced` with a member in `forced`, at a node with the promotions
    /// `promoted` for which `watches` are valid: then the node has no completion that promotes all of `forced`.  Such a
    /// nogood watches two members of `forced` (one member left would have been ruled out of the node's candidates, and
    /// `forced` is among them), so only the nogoods that watch a member of `forced` are looked at.
    pub fn all_forced(&self, watches: &Watches, promoted: &Bits<W>, forced: &Bits<W>) -> bool {
        forced.iter().any(|f| {
            watches.watching.get(f as usize).is_some_and(|list| {
                list.iter().any(|&id| {
                    let [a, b] = watches.watched[id as usize];
                    let other = if a == f { b } else { a };
                    forced.contains(other)
                        && self.nogood(id).iter().all(|&y| promoted.contains(y) || forced.contains(y))
                })
            })
        })
    }

    /// Adds what a searcher did with the store: candidates it ruled out, and nodes it ended.
    pub fn add_counts(&self, ruled_out: u64, cuts: u64) {
        if ruled_out > 0 {
            self.ruled_out.fetch_add(ruled_out, Ordering::Relaxed);
        }
        if cuts > 0 {
            self.cuts.fetch_add(cuts, Ordering::Relaxed);
        }
    }

    /// What the searchers have done with the store so far (as they add it at each flush): candidates ruled out, and
    /// nodes ended.
    pub fn counts(&self) -> (u64, u64) {
        (self.ruled_out.load(Ordering::Relaxed), self.cuts.load(Ordering::Relaxed))
    }
}
