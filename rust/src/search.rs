//! The branch-and-bound search for the promotion set with the largest sum, on the parallel [`engine`].
//!
//! [`prepare`] sets a game up the way the Java's `SearchManager.create` does: the frame set, the preclusion
//! table, the greedy incumbent, and (tactic `revise_freedoms`) a revised frame 0 that can lower the cap on the
//! number of promotions.  [`search`] then explores the tree of `BranchExplorer.explore`, made lazy: each child
//! is explored depth-first as soon as it is made, on an explicit stack that is reused from node to node (no
//! recursion, no allocation per node):
//!
//! ```text
//! loop while cands non-empty:
//!     if upper_bound(state, cands, max_add) + sum(promoted) <= best: break
//!     n = the candidate to branch on (tactic `branching`; the Java's is cands.max())
//!     cands.remove(n)
//!     new_state = state.promote(n) else continue
//!     new_promoted = promoted ∪ {n}
//!     max_next = max_additional(new_state, new_promoted, cands)
//!     child_cands = new_candidate_list(n, max_next, new_state, cands, promoted)
//!     if sum(promoted) + n + upper_bound(new_state, child_cands, max_next) > best:
//!         if sum(new_promoted) > best: offer new_promoted as incumbent
//!         push child (new_state, new_promoted, child_cands,
//!                     max_additional(new_state, new_promoted, child_cands))
//! ```
//!
//! The Java breaks only when the bound is below `best`; no completion can beat `best` when it is equal, so this
//! loop breaks then too.  The upper bound is `blameGame`'s, or with the `match_bound` tactic the matching bound
//! ([`crate::matchbound`]), possibly refined by conflicts (see [`Refiner`]); the children's candidate lists then
//! leave out the Java's promotability checks (see [`Bounds::with_promotability_checks`]).  When the heaviest matching
//! can itself be promoted, it is the node's best completion: it is offered as the incumbent and the node is
//! done.  With conflict branching the candidate `n` is a member of a conflict of the heaviest matching; with
//! the `fix_forced` tactic the child also promotes the members of that conflict that every completion beating
//! the incumbent must promote (see [`strongest`]).  With the `nogoods` tactic the conflicts found are learned
//! as conflicts of the root ([`Nogoods`]), which every node can use.  The search starts from the best of the
//! greedy set, the seed, and a rounding of the root's matching ([`dive`]), improved by a local search
//! ([`Neighbourhood::improve`]).
//!
//! Two exact shortcuts keep the expensive bound out of the way: the bound never exceeds the sum of the
//! `max_add` largest candidates, so when that sum already decides the comparison the bound is not computed;
//! and a child with no candidates is not pushed (its loop would do nothing).
//!
//! A unit of work is just `(promoted, cands)`: a node's state is the root with `promoted` promoted, rebuilt in
//! one pass (see [`Model::promote_all_using`]; the order of promotions does not matter), and `cands` holds the
//! candidates its loop has still to try.  A rebuilt node recomputes `max_add` from its remaining candidates,
//! which can only tighten it.

use crate::bits::Bits;
use crate::bounds::{self, max_additional, Bounds, Preclusions, RunnerUp};
use crate::engine::{self, Counters, Ctx, EngineConfig, Poll, Searcher, Start, Stats, Unit};
use crate::matchbound::{MatchBound, MatchGraph};
use crate::model::{ConflictCache, FrameSet, Lookup, Model, Nogoods, Scratch};
use crate::numtheory::Tables;
use std::io;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------------------------------------
// Tactics and units

/// The tactics of DESIGN.md that act inside the search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchTactics {
    /// Rule candidates out with the depth-2 preclusion table (sound).
    pub preclusions: bool,
    /// Use the Java's runner-up in the upper bound (unsound; reproduces the Java).
    pub legacy_bound: bool,
    /// Lower frame 0's freedoms after solving the two-frame game (unproven: "the max-sum promotion set also has
    /// the max count").
    pub revise_freedoms: bool,
    /// Bound nodes by the matching relaxation ([`crate::matchbound`]) instead of `blameGame` (sound; `blameGame`
    /// never pruned where the matching bound did not), and leave the Java's promotability checks out of the
    /// children's candidate lists (its conflicts find those candidates).
    pub match_bound: bool,
    /// Refine the matching bound by conflicts, branching this many levels deep where it does not prune
    /// (0: off; needs `match_bound`; sound, see [`Refiner`]).  At most [`Self::MAX_MATCH_REFINE`].  What it
    /// prunes can depend on the conflicts already known along the path, which work units do not record, so a
    /// single-threaded search that is interrupted may explore a slightly different tree (with the same answer).
    pub match_refine: u8,
    /// Which candidate a node branches on next (sound: any order explores every promotion set).
    pub branching: Branching,
    /// With `branching = conflict-strong`: a child also promotes the node's other *forced* candidates, the
    /// members of the conflict whose removal alone brings the matching bound below what the node needs (sound:
    /// every completion that can beat the incumbent promotes them; see [`strongest`]).
    pub fix_forced: bool,
    /// Learn the conflicts found as nogoods of the root ([`Nogoods`]) and use them at every node: for the
    /// conflict to branch on and to refine with, and to rule candidates out of children (sound: a nogood holds
    /// everywhere).  What is learned depends on the order the search went, so with it a search's tree depends on
    /// its history (threads, interruptions), not only on the node.
    pub nogoods: bool,
}

/// Which candidate a node's loop promotes next (tactic `branching`).  The child promotes it, and the node goes on
/// without it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Branching {
    /// The largest candidate left (the Java's order).
    Largest,
    /// The smallest member of a conflict of the heaviest matching (see [`Refiner`]), so that neither the child
    /// nor the node going on can repeat that matching; the largest candidate when there is none (or without
    /// `match_bound`).
    ConflictSmallest,
    /// As `ConflictSmallest`, the largest member of the conflict.  The child then follows the rounding of
    /// [`dive`] (the smaller members are the ones left out), which finds good incumbents early.
    ConflictLargest,
    /// The member of the conflict that looks best one step ahead (see [`strongest`]).
    #[default]
    ConflictStrong,
}

impl Branching {
    /// Every variant.
    pub const ALL: [Branching; 4] =
        [Branching::Largest, Branching::ConflictSmallest, Branching::ConflictLargest, Branching::ConflictStrong];

    /// The name the command line and the results use (`largest`, `conflict-smallest`, ...); [`FromStr`] reads it.
    pub fn name(self) -> &'static str {
        match self {
            Branching::Largest => "largest",
            Branching::ConflictSmallest => "conflict-smallest",
            Branching::ConflictLargest => "conflict-largest",
            Branching::ConflictStrong => "conflict-strong",
        }
    }
}

impl FromStr for Branching {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, String> {
        Branching::ALL.into_iter().find(|branching| branching.name() == text).ok_or_else(|| {
            let names: Vec<&str> = Branching::ALL.iter().map(|branching| branching.name()).collect();
            format!("expected one of {}, got {text:?}", names.join(", "))
        })
    }
}

impl Default for SearchTactics {
    fn default() -> Self {
        SearchTactics {
            preclusions: true,
            legacy_bound: false,
            revise_freedoms: true,
            match_bound: true,
            match_refine: 1,
            branching: Branching::default(),
            fix_forced: true,
            nogoods: true,
        }
    }
}

impl SearchTactics {
    /// The largest `match_refine`: the header of a checkpoint has room for no more, so a deeper setting could not
    /// be told from this one on resuming.
    pub const MAX_MATCH_REFINE: u8 = 7;

    fn rule(self) -> RunnerUp {
        if self.legacy_bound {
            RunnerUp::Legacy
        } else {
            RunnerUp::Sound
        }
    }

    /// The tactics as the header of a checkpoint records them.  Tactics added later are recorded by the bits of
    /// their non-default values (`fix_forced` by bit 9 and `nogoods` by bit 10 when they are off), so that a checkpoint made before they
    /// existed reads back with their defaults: they are sound, so a search resumed with them is still correct.
    fn bits(self) -> u16 {
        let branching = match self.branching {
            Branching::Largest => 0,
            Branching::ConflictSmallest => 1,
            Branching::ConflictLargest => 2,
            Branching::ConflictStrong => 3,
        };
        u16::from(self.preclusions)
            | u16::from(self.legacy_bound) << 1
            | u16::from(self.revise_freedoms) << 2
            | u16::from(self.match_bound) << 3
            | u16::from(self.match_refine.min(Self::MAX_MATCH_REFINE)) << 4
            | branching << 7
            | u16::from(!self.fix_forced) << 9
            | u16::from(!self.nogoods) << 10
    }

    fn from_bits(bits: u16) -> Result<Self, String> {
        if bits >> 11 != 0 {
            return Err(format!("unknown tactics {bits:#x}"));
        }
        let branching = match bits >> 7 & 3 {
            0 => Branching::Largest,
            1 => Branching::ConflictSmallest,
            2 => Branching::ConflictLargest,
            _ => Branching::ConflictStrong,
        };
        Ok(SearchTactics {
            preclusions: bits & 1 != 0,
            legacy_bound: bits & 2 != 0,
            revise_freedoms: bits & 4 != 0,
            match_bound: bits & 8 != 0,
            match_refine: (bits >> 4 & 7) as u8,
            branching,
            fix_forced: bits & 1 << 9 == 0,
            nogoods: bits & 1 << 10 == 0,
        })
    }
}

/// A unit of work: the node reached by promoting `promoted` from the root, with the candidates `cands` it has
/// still to try.  Encoded as the raw little-endian words of the two sets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaxUnit<const W: usize> {
    pub promoted: Bits<W>,
    pub cands: Bits<W>,
}

impl<const W: usize> Unit for TaxUnit<W> {
    fn encode(&self, out: &mut Vec<u8>) {
        put_bits(out, &self.promoted);
        put_bits(out, &self.cands);
    }

    fn decode(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 16 * W, "a unit of Bits<{W}> has {} bytes", 16 * W);
        TaxUnit { promoted: get_bits(&bytes[..8 * W]), cands: get_bits(&bytes[8 * W..]) }
    }
}

fn put_bits<const W: usize>(out: &mut Vec<u8>, set: &Bits<W>) {
    for word in set.as_words() {
        out.extend_from_slice(&word.to_le_bytes());
    }
}

/// The set encoded by [`put_bits`]; `bytes` must be exactly `8·W` long.
fn get_bits<const W: usize>(bytes: &[u8]) -> Bits<W> {
    let mut words = [0u64; W];
    for (word, chunk) in words.iter_mut().zip(bytes.as_chunks::<8>().0) {
        *word = u64::from_le_bytes(*chunk);
    }
    Bits::from_words(words)
}

// ---------------------------------------------------------------------------------------------------------
// The searcher

/// What every searcher of one search shares, read-only.
pub struct SearchSpace<'a, const W: usize> {
    pub model: &'a Model,
    /// The starting state (frame 0's freedoms revised if that tactic is on).
    pub root: &'a FrameSet<W>,
    /// The preclusion table, if that tactic is on.
    pub preclusions: Option<&'a Preclusions<W>>,
    /// The root's matching graph, if the `match_bound` tactic is on.
    pub match_graph: Option<&'a MatchGraph>,
    /// How deep to refine the matching bound by conflicts (tactic `match_refine`), and how to branch.
    pub match_refine: u32,
    pub conflict_branching: Branching,
    /// Tactics `fix_forced` and `nogoods`.
    pub fix_forced: bool,
    pub nogoods: bool,
    /// The largest number in the root's frames.
    pub root_max: u32,
    /// The cap on the number of promotions (`maxTotalPromotions`).
    pub max_total: u32,
    pub rule: RunnerUp,
    /// A proven ceiling on the promotion sum (n + opt(n − 1) − inTheBag): reaching it ends the search.
    pub cap: Option<u64>,
    /// Where to collect [`Diagnostics`], if anywhere.
    pub diagnostics: Option<&'a Mutex<Diagnostics>>,
    /// When the search began, and the best incumbent the searchers found (see [`Found`]).
    pub began: Instant,
    pub found: Mutex<Option<Found>>,
}

/// When a search found its best incumbent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Found {
    pub value: u64,
    /// Since the search began.
    pub time: Duration,
    /// The steps (engine nodes) the worker that found it had taken in this invocation by then: all the steps of
    /// the search with one thread.
    pub steps: u64,
}

// ---------------------------------------------------------------------------------------------------------
// Conflict refinement of the matching bound

/// The matching bound of a node refined by conflicts (tactic `match_refine`), on top of a [`MatchBound`] whose
/// top level is the node, with `state` and the candidates `C` of that level.
///
/// The heaviest matching promotes some `Q* ⊆ C`.  If `Q*` can really be promoted from `state`, the bound is the
/// best completion's sum and stands.  Otherwise some subset `K ⊆ Q*` cannot be promoted, a *conflict*
/// ([`Model::conflict_using`]).  A subset of a set that can be promoted can be promoted too (moves left out and
/// factors kept only help a frame, see [`Model::winnable`]), so every completion `F ⊆ C` that can be promoted
/// leaves out some member of `K`.  With `K`'s members `k₁ < k₂ < … < k_r`, every such `F` is in exactly one
/// of the branches "`k₁ … k_{j−1}` in `F`, `k_j` not", and the best completion is at most the largest of the
/// branches' matching bounds: the matching with `k_j` no longer a candidate and `k₁ … k_{j−1}` required (as
/// if promoted), plus their sum.  Each branch is refined in the same way, one level less deep, with the members
/// required so far counted in `Q*` (so a conflict among them alone leaves the branch without completions).  By
/// induction on the depth, the refined bound is never below the best completion; `max_add` is not needed: the
/// bound only grows without it.
///
/// A conflict of a state is a conflict of every state below it (they have more promotions, which can only
/// make promoting `K` harder) and of every candidate set containing it, so the conflicts found are kept in
/// `known` for the rest of the node's loop and its subtree: one that `Q*` contains needs no peeling.
///
/// The branches are tried from the smallest member, whose exclusion costs the matching least, so that a bound
/// that stays at least `need` is usually seen at once.
struct Refiner<'r, 'g, const W: usize> {
    m: &'r mut MatchBound<'g, W>,
    model: &'r Model,
    scratch: &'r mut Scratch<W>,
    state: &'r FrameSet<W>,
    need: u64,
    /// Conflicts of `state` (or of states above it).
    known: &'r mut Vec<Bits<W>>,
    cache: &'r mut ConflictCache<W>,
    /// Matching evaluations made.
    evaluations: u64,
    /// Conflicts computed (not found in `known` or the nogood table), and found in the table.
    computed: u64,
    hits: u64,
    /// The best completion found that can be promoted (the candidates of an exact branch), or none.
    exact: Bits<W>,
    /// The nogoods (tactic `nogoods`).
    learning: Option<Learning<'r, W>>,
    /// Whether to find the heaviest matching's forced members (tactic `fix_forced`), and those found.
    forcing: bool,
    forced: Bits<W>,
}

/// What a [`Refiner`] needs to use and grow the searcher's nogood table (tactic `nogoods`).
struct Learning<'r, const W: usize> {
    table: &'r mut Nogoods<W>,
    /// The search's root, and the promotions of the node.
    root: &'r FrameSet<W>,
    promoted: Bits<W>,
}

/// What the matching bound says about a node (see `TaxSearcher::matching_check`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MatchCheck<const W: usize> {
    /// Prune, for this [`cut`] reason.
    Cut(usize),
    /// The best completion is known: these candidates.
    Exact(Bits<W>),
    /// Go on; the conflict to branch on, if one was asked for and found.
    Open(Option<Bits<W>>),
}

/// What [`Refiner::top`] found out about the top level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict<const W: usize> {
    /// The (refined) bound is below `need`.
    Below,
    /// The heaviest matching's candidates can all be promoted: they are the best completion, and the bound is
    /// at least `need`.
    Exact(Bits<W>),
    /// The bound is at least `need`; the conflict of the heaviest matching, if one was looked for.
    Open(Option<Bits<W>>),
}

impl<const W: usize> Refiner<'_, '_, W> {
    /// The top level's verdict, refining `depth` levels deep.  The heaviest matching is checked (and its conflict
    /// found) only when refining or when `conflict` asks for it.
    fn top(&mut self, depth: u32, conflict: bool) -> Verdict<W> {
        self.evaluations += 1;
        if self.m.value() < self.need {
            return Verdict::Below;
        }
        if depth == 0 && !conflict {
            return Verdict::Open(None);
        }
        let chosen = self.m.chosen();
        if self.forcing {
            // The members of the heaviest matching that every completion reaching `need` promotes, as far as the
            // potentials show (see `MatchBound::bound_excluding`).
            let need = self.need;
            self.forced = chosen.iter().filter(|&x| self.m.bound_excluding(x) < need).collect();
        }
        let k = match self.find_conflict(&chosen, self.forced) {
            Lookup::Conflict(k) => k,
            Lookup::None => return Verdict::Exact(chosen),
            Lookup::Forced => return Verdict::Below,
        };
        // A conflict all of whose members are forced leaves no completion that reaches `need`.  With some forced
        // members the node's only child promotes them, and refining is left to it (cheaper, if a few more nodes).
        let refine = depth > 0 && self.forced.is_empty();
        if k.is_subset(&self.forced) || (refine && !self.branches(k, Bits::EMPTY, depth)) {
            return Verdict::Below;
        }
        Verdict::Open(Some(k))
    }

    /// A conflict of `promoted` from `self.state`, known or found; `None` if it can all be promoted.
    fn conflict_of(&mut self, promoted: &Bits<W>) -> Option<Bits<W>> {
        match self.known.iter().rev().find(|k| k.is_subset(promoted)) {
            Some(&k) => Some(k),
            None => self.top_conflict(promoted),
        }
    }

    /// A conflict of `promoted` from `self.state` (and then known), from the nogood table if it has one (the
    /// smallest it has), else computed (and then learned); `None` if it can all be promoted.  The search branches
    /// on the top level's conflict, which is found this way, not looked up in what is known along the path: so
    /// without the `nogoods` tactic the branching depends only on the node; with it, on what the searcher has
    /// learned.  Refinements and the propagation into children use what is known too, which work units do not
    /// record.
    fn top_conflict(&mut self, promoted: &Bits<W>) -> Option<Bits<W>> {
        match self.find_conflict(promoted, Bits::EMPTY) {
            Lookup::Conflict(k) => Some(k),
            _ => None,
        }
    }

    /// [`top_conflict`](Self::top_conflict), or [`Lookup::Forced`] if the table knows a conflict all of whose
    /// members are in `forced`.
    fn find_conflict(&mut self, promoted: &Bits<W>, forced: Bits<W>) -> Lookup<W> {
        if let Some(l) = &self.learning {
            match l.table.lookup(&l.promoted, promoted, &forced) {
                Lookup::None => {}
                found => {
                    self.hits += 1;
                    if let Lookup::Conflict(k) = found {
                        self.known.push(k);
                    }
                    return found;
                }
            }
        }
        let found = self.model.conflict_cached(self.scratch, Some(self.cache), self.state, promoted, true);
        let Some(k) = found else { return Lookup::None };
        self.computed += 1;
        if let Some(l) = &mut self.learning {
            if let Some(n) = self.model.nogood(self.scratch, l.root, &l.promoted, &k) {
                l.table.insert(n, &k);
            }
        }
        self.known.push(k);
        Lookup::Conflict(k)
    }

    /// Whether the bound of the top level, with `required` (members of `C` required by the branches above)
    /// promoted, refined `depth` levels deep, is at least `need`.  Leaves the solver as it found it.
    fn reaches(&mut self, required: Bits<W>, depth: u32) -> bool {
        self.evaluations += 1;
        if self.m.value() + required.sum() < self.need {
            return false;
        }
        if depth == 0 {
            return true;
        }
        let promoted = self.m.chosen() | required;
        match self.conflict_of(&promoted) {
            None => {
                // The bound is exact, and a completion that good exists.
                if promoted.sum() > self.exact.sum() {
                    self.exact = promoted;
                }
                true
            }
            Some(k) => self.branches(k, required, depth),
        }
    }

    /// Whether some branch of the conflict `k`, with `required` promoted, refined `depth − 1` levels deep, is at
    /// least `need`.
    fn branches(&mut self, k: Bits<W>, required: Bits<W>, depth: u32) -> bool {
        let free = k - required;
        let last = free.max();
        let mut required = required;
        let mut reaches = false;
        let mut pushed = 0;
        for x in free.iter() {
            // The bound without x is often below `need` by the potentials alone (see
            // `MatchBound::bound_excluding`); else solve for it.
            reaches = self.m.bound_excluding(x) + required.sum() >= self.need && {
                self.m.push_excluding(&Bits::EMPTY.with(x));
                let reaches = self.reaches(required, depth - 1);
                self.m.pop();
                reaches
            };
            if reaches || Some(x) == last {
                break;
            }
            // The next branches have x.
            let cands = self.m.candidates();
            self.m.push(x, &cands);
            required.insert(x);
            pushed += 1;
        }
        for _ in 0..pushed {
            self.m.pop();
        }
        reaches
    }
}

/// The member of the conflict `k` to branch on by looking ahead (tactic `branching = conflict-strong`): the one
/// whose removal from the candidates lowers the matching bound most (then the largest), which makes the node
/// going on without it the likeliest to stop soon.  (The child that promotes it gains nothing to look at: the
/// heaviest matching already promotes every member.)  `m`'s top level is the node, settled; it is left so, with
/// the level without the chosen member kept for it (see [`MatchBound::keep_excluded`]).
///
/// Also returns the members whose removal brings the bound below `need`, the node's *forced* members: every
/// completion that reaches `need` promotes them all.  The chosen member is among them unless there are none.
///
/// With `fix_forced` (that tactic promotes all the forced members, so that which of them is chosen hardly
/// matters), when some member is forced the largest forced one is chosen and no level is kept (the node's loop
/// has nothing left to do after that child).  Returns whether a level is kept.
fn strongest<const W: usize>(
    m: &mut MatchBound<'_, W>,
    k: &Bits<W>,
    need: u64,
    fix_forced: bool,
) -> (u32, Bits<W>, bool) {
    let mut forced = Bits::EMPTY;
    let mut best = (u64::MAX, 0);
    let mut kept = false;
    for x in k.iter() {
        let bound = m.value_excluding(x);
        if bound < need {
            forced.insert(x);
        }
        if bound <= best.0 {
            m.keep_excluded();
            kept = true;
            best = (bound, x);
        } else {
            m.pop();
        }
    }
    match forced.max() {
        Some(n) if fix_forced => {
            m.drop_kept();
            (n, forced, false)
        }
        _ => (best.1, forced, kept),
    }
}

/// Most nogoods a searcher keeps (tactic `nogoods`); it forgets them all when it has this many.  Searches near
/// n = 1200 learn about a thousand.
const NOGOOD_CAPACITY: usize = 1 << 16;

/// One node on the searcher's stack.
#[derive(Clone, Copy)]
struct Level<const W: usize> {
    state: FrameSet<W>,
    promoted: Bits<W>,
    /// The sum of `promoted`.
    sum: u64,
    /// The candidates still to try.
    cands: Bits<W>,
    max_add: u32,
    /// How many of the searcher's known conflicts hold for this node.
    known_end: usize,
    /// What the matching bound said when the node was made, until its loop takes it: open at this `need`, with
    /// this conflict to branch on and these forced members (see [`Refiner`]).
    checked: Option<(u64, Bits<W>, Bits<W>)>,
}

/// Counts of the matching bound's work in a searcher.
#[derive(Clone, Copy, Debug, Default)]
struct RefineCounts {
    /// Matching bounds computed for nodes.
    match_bounds: u64,
    /// Refinements made (tactic `match_refine`), the matching evaluations they made, and the conflicts they
    /// had to compute (not known yet).
    refinements: u64,
    evaluations: u64,
    conflicts: u64,
    /// Conflicts found in the nogood table, and candidates its nogoods ruled out of children (tactic `nogoods`).
    hits: u64,
    removed: u64,
}

/// Indexes into [`Stats::extra`].
pub mod stat {
    /// Promotions tried (each candidate a node's loop gets to).
    pub const PROMOTE_ATTEMPTS: usize = 0;
    /// Promotions that succeeded.
    pub const PROMOTIONS: usize = 1;
    /// Upper bounds computed.
    pub const UPPER_BOUNDS: usize = 2;
    /// Promotability checks made while building candidate lists.
    pub const CAN_PROMOTES: usize = 3;
    /// Children pushed on the stack (explored).
    pub const CHILDREN: usize = 4;
    /// Candidates the preclusion table ruled out.
    pub const PRECLUDED: usize = 5;
    /// Frame certificates made for the promotability checks.
    pub const CERTIFICATES: usize = 6;
    /// Matching bounds computed.
    pub const MATCH_BOUNDS: usize = 7;
}

// ---------------------------------------------------------------------------------------------------------
// Diagnostics

/// Why a node's loop ended or a promoted child was not explored, for [`Diagnostics`].
pub mod cut {
    /// No candidates left (a child: none to explore).
    pub const EMPTY: usize = 0;
    /// The sum of the `max_add` largest candidates is too small.
    pub const LARGEST: usize = 1;
    /// The `blameGame` bound.
    pub const BLAME: usize = 2;
    /// The matching bound.
    pub const MATCH: usize = 3;
    /// The matching bound refined by conflicts.
    pub const REFINED: usize = 4;
    /// The heaviest matching can be promoted: the best completion is known (and offered as the incumbent).
    pub const EXACT: usize = 5;
    /// A forced candidate (tactic `fix_forced`) is no longer a candidate of the child or cannot be promoted.
    pub const FORCED: usize = 6;
    pub const COUNT: usize = 7;
    pub const NAMES: [&str; COUNT] = ["empty", "largest", "blame", "match", "refined", "exact", "forced"];
}

/// Buckets of [`Diagnostics::gaps`]: excesses `0..GAP_BUCKETS − 1`, and the last for everything above.
pub const GAP_BUCKETS: usize = 512;

/// Measurements of one search, for tuning (see [`search_with`]).  Collecting them computes extra bounds at
/// every node, so a search that collects them is slower and its times mean little.
#[derive(Clone, Debug, Default)]
pub struct Diagnostics {
    /// Steps (engine nodes: candidates a node's loop gets to, plus the step that ends the loop), by the number
    /// of promotions of the node taking them.
    pub steps: Vec<u64>,
    /// Children explored, by their number of promotions.
    pub children: Vec<u64>,
    /// How node loops ended, by [`cut`] reason.
    pub loop_ends: [u64; cut::COUNT],
    /// How promoted children were left unexplored, by [`cut`] reason.
    pub child_cuts: [u64; cut::COUNT],
    /// Candidates whose promotion failed.
    pub failed_promotions: u64,
    /// At the node-loop checks that let the loop go on: how far the bound (the smaller of `blameGame` and,
    /// with that tactic, matching) was above what the loop needed, as a histogram (see [`GAP_BUCKETS`]).
    pub gaps: Vec<u64>,
    /// The same for `blameGame` alone and for the matching bound alone (only with the `match_bound` tactic).
    pub blame_gaps: Vec<u64>,
    pub match_gaps: Vec<u64>,
    /// Matching bounds that did not prune, and how many of those had a heaviest matching whose candidates can
    /// all be promoted together (with at most the node's `max_add` of them), which makes the bound exact.
    pub match_open: u64,
    pub match_open_feasible: u64,
    /// At those matching bounds: histograms of the size of the conflict from the core, and of the minimal
    /// conflict (see [`Model::conflict_using`]).
    pub core_sizes: Vec<u64>,
    pub conflict_sizes: Vec<u64>,
    /// At those matching bounds: how far one level of refinement (with the minimal conflict) lowers the bound
    /// (a histogram), and how often that is enough to prune.
    pub refine_drops: Vec<u64>,
    pub refine_cuts: u64,
    /// With the `match_refine` tactic: refinements made, the matching evaluations they made, and the conflicts
    /// they computed (not known yet).
    pub refinements: u64,
    pub refine_evaluations: u64,
    pub refine_conflicts: u64,
    /// The matching solvers' work: evaluations, Hungarian searches, vertices scanned (see
    /// [`MatchStats`](crate::matchbound::MatchStats)).
    pub matching: [u64; 3],
}

fn bump(counts: &mut Vec<u64>, i: usize) {
    if counts.len() <= i {
        counts.resize(i + 1, 0);
    }
    counts[i] += 1;
}

fn add_all(into: &mut Vec<u64>, from: &[u64]) {
    if into.len() < from.len() {
        into.resize(from.len(), 0);
    }
    for (a, b) in into.iter_mut().zip(from) {
        *a += b;
    }
}

impl Diagnostics {
    /// Adds `other`'s counts to these.
    pub fn merge(&mut self, other: &Diagnostics) {
        add_all(&mut self.steps, &other.steps);
        add_all(&mut self.children, &other.children);
        for (a, b) in self.loop_ends.iter_mut().zip(other.loop_ends) {
            *a += b;
        }
        for (a, b) in self.child_cuts.iter_mut().zip(other.child_cuts) {
            *a += b;
        }
        self.failed_promotions += other.failed_promotions;
        add_all(&mut self.gaps, &other.gaps);
        add_all(&mut self.blame_gaps, &other.blame_gaps);
        add_all(&mut self.match_gaps, &other.match_gaps);
        self.match_open += other.match_open;
        self.match_open_feasible += other.match_open_feasible;
        add_all(&mut self.core_sizes, &other.core_sizes);
        add_all(&mut self.conflict_sizes, &other.conflict_sizes);
        add_all(&mut self.refine_drops, &other.refine_drops);
        self.refine_cuts += other.refine_cuts;
        self.refinements += other.refinements;
        self.refine_evaluations += other.refine_evaluations;
        self.refine_conflicts += other.refine_conflicts;
        for (a, b) in self.matching.iter_mut().zip(other.matching) {
            *a += b;
        }
    }

    fn gap(counts: &mut Vec<u64>, gap: u64) {
        bump(counts, (gap as usize).min(GAP_BUCKETS - 1));
    }

    /// The `q`-quantile (0 ≤ q ≤ 1) of a histogram of [`GAP_BUCKETS`] buckets.
    pub fn quantile(counts: &[u64], q: f64) -> Option<usize> {
        let total: u64 = counts.iter().sum();
        if total == 0 {
            return None;
        }
        let target = (q * total as f64).ceil().max(1.0) as u64;
        let mut seen = 0;
        counts.iter().position(|&c| {
            seen += c;
            seen >= target
        })
    }
}

/// A searcher's own diagnostics, merged into the search's at every flush.
struct Probe<'a> {
    shared: &'a Mutex<Diagnostics>,
    local: Diagnostics,
}

impl Probe<'_> {
    /// A node-loop check that let the loop go on (bound at least `need`): records how far above it was, and
    /// whether the matching bound is exact there.
    fn open<const W: usize>(
        &mut self,
        bounds: &mut Bounds<'_, W>,
        matching: &mut Option<MatchBound<'_, W>>,
        model: &Model,
        node: &Level<W>,
        need: u64,
    ) {
        let blame = bounds.upper_bound(&node.state, &node.cands, node.max_add);
        let mut bound = blame;
        Diagnostics::gap(&mut self.local.blame_gaps, blame.saturating_sub(need));
        if let Some(m) = matching {
            let value = m.value();
            Diagnostics::gap(&mut self.local.match_gaps, value.saturating_sub(need));
            bound = bound.min(value);
            self.matching_open(bounds, m, model, &node.state, node.max_add, need);
        }
        Diagnostics::gap(&mut self.local.gaps, bound.saturating_sub(need));
    }

    /// A matching bound that did not prune (it is at least `need`): is its heaviest matching a feasible
    /// completion, and what would one level of conflict refinement do?
    fn matching_open<const W: usize>(
        &mut self,
        bounds: &mut Bounds<'_, W>,
        m: &mut MatchBound<'_, W>,
        model: &Model,
        state: &FrameSet<W>,
        max_add: u32,
        need: u64,
    ) {
        let chosen = m.chosen();
        let value = m.value();
        self.local.match_open += 1;
        let scratch = bounds.scratch();
        let feasible = chosen.len() <= max_add && model.can_promote_all_using(scratch, state, &chosen);
        self.local.match_open_feasible += u64::from(feasible);
        if let Some(core) = model.conflict_using(scratch, state, &chosen, false) {
            bump(&mut self.local.core_sizes, core.len() as usize);
            let conflict = model.conflict_using(scratch, state, &chosen, true).expect("a conflict");
            bump(&mut self.local.conflict_sizes, conflict.len() as usize);
            let mut refined = 0;
            for k in conflict.iter() {
                m.push_excluding(&Bits::EMPTY.with(k));
                refined = refined.max(m.value());
                m.pop();
            }
            Diagnostics::gap(&mut self.local.refine_drops, value - refined);
            self.local.refine_cuts += u64::from(refined < need);
        }
    }

    fn flush(&mut self) {
        let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        shared.merge(&self.local);
        self.local = Diagnostics::default();
    }
}

/// The Taxman searcher: explores units depth-first on a reusable stack.
pub struct TaxSearcher<'a, const W: usize> {
    space: &'a SearchSpace<'a, W>,
    bounds: Bounds<'a, W>,
    /// The matching bound along the stack (tactic `match_bound`).  Its levels mirror the stack's: level 0 is the
    /// unit's node, and a child is pushed on it while the child is being considered and for as long as it is
    /// on the stack.
    matching: Option<MatchBound<'a, W>>,
    /// Stack slots, reused: `levels[..=depth]` is the current path, and the next slot is where a child is made.
    levels: Vec<Level<W>>,
    attempts: u64,
    promotions: u64,
    children: u64,
    counts: RefineCounts,
    /// Conflicts known for the node being worked on (see [`Refiner`]); `levels[d].known_end` of them hold for
    /// the node at depth `d`.
    known: Vec<Bits<W>>,
    /// The frames' last conflicts (see [`ConflictCache`]).
    conflict_cache: ConflictCache<W>,
    /// The nogoods learned (tactic `nogoods`).
    nogoods: Option<Nogoods<W>>,
    payload: Vec<u8>,
    probe: Option<Probe<'a>>,
    /// The matching solver's statistics already added to the diagnostics.
    match_seen: [u64; 3],
}

impl<'a, const W: usize> TaxSearcher<'a, W> {
    pub fn new(space: &'a SearchSpace<'a, W>) -> Self {
        TaxSearcher {
            space,
            bounds: Bounds::new(space.model, space.preclusions, space.rule)
                .with_promotability_checks(space.match_graph.is_none()),
            matching: space.match_graph.map(MatchBound::new),
            levels: Vec::new(),
            attempts: 0,
            promotions: 0,
            children: 0,
            counts: RefineCounts::default(),
            known: Vec::new(),
            conflict_cache: ConflictCache::default(),
            nogoods: space.nogoods.then(|| Nogoods::new(space.root_max, NOGOOD_CAPACITY)),
            payload: Vec::with_capacity(8 * W),
            probe: space.diagnostics.map(|shared| Probe { shared, local: Diagnostics::default() }),
            match_seen: [0; 3],
        }
    }

    /// The matching bound's verdict on the node on top of the matching stack, which has `state`: below `need`
    /// ([`cut::MATCH`]), or brought below it by refining (tactic `match_refine`, with the conflicts `known` for
    /// `state`; [`cut::REFINED`]), or exact, or at least `need`, with the conflict to branch on if the tactic
    /// `conflict_branching` wants one.  Open without the `match_bound` tactic.
    #[allow(clippy::too_many_arguments)]
    fn matching_check(
        matching: &mut Option<MatchBound<'a, W>>,
        counts: &mut RefineCounts,
        space: &SearchSpace<'a, W>,
        scratch: &mut Scratch<W>,
        known: &mut Vec<Bits<W>>,
        cache: &mut ConflictCache<W>,
        state: &FrameSet<W>,
        need: u64,
        want_conflict: bool,
        found: &mut Bits<W>,
        nogoods: Option<(&mut Nogoods<W>, Bits<W>)>,
        forced: &mut Bits<W>,
    ) -> MatchCheck<W> {
        let Some(m) = matching.as_mut() else {
            return MatchCheck::Open(None);
        };
        counts.match_bounds += 1;
        if m.value() < need {
            return MatchCheck::Cut(cut::MATCH);
        }
        let (depth, model) = (space.match_refine, space.model);
        if depth == 0 && !want_conflict {
            return MatchCheck::Open(None);
        }
        let learning = nogoods.map(|(table, promoted)| Learning { table, root: space.root, promoted });
        let mut refiner = Refiner {
            m,
            model,
            scratch,
            state,
            need,
            known,
            cache,
            evaluations: 0,
            computed: 0,
            hits: 0,
            exact: Bits::EMPTY,
            learning,
            forcing: space.fix_forced && space.conflict_branching == Branching::ConflictStrong,
            forced: Bits::EMPTY,
        };
        let verdict = refiner.top(depth, want_conflict);
        *forced = refiner.forced;
        counts.refinements += 1;
        counts.evaluations += refiner.evaluations;
        counts.conflicts += refiner.computed;
        counts.hits += refiner.hits;
        *found = refiner.exact;
        match verdict {
            Verdict::Below => MatchCheck::Cut(cut::REFINED),
            Verdict::Exact(q) => MatchCheck::Exact(q),
            Verdict::Open(k) => MatchCheck::Open(k),
        }
    }

    /// Offers `set` as the incumbent if it beats it, recording when, and ends the search if it reaches the cap.
    fn offer(space: &SearchSpace<'a, W>, payload: &mut Vec<u8>, ctx: &mut Ctx<'_, TaxUnit<W>>, set: &Bits<W>) {
        let value = set.sum();
        if value <= ctx.best() {
            return;
        }
        payload.clear();
        put_bits(payload, set);
        if ctx.offer(value, payload) {
            let found = Found { value, time: space.began.elapsed(), steps: ctx.stats().nodes };
            let mut record = space.found.lock().unwrap_or_else(PoisonError::into_inner);
            if record.is_none_or(|r| r.value < found.value) {
                *record = Some(found);
            }
            drop(record);
            if space.cap.is_some_and(|cap| value >= cap) {
                ctx.declare_optimal();
            }
        }
    }

    /// Rebuilds the unit's node in slot 0.
    fn start(&mut self, unit: &TaxUnit<W>) {
        let space = self.space;
        let state = space
            .model
            .promote_all_using(self.bounds.scratch(), space.root, &unit.promoted)
            .unwrap_or_else(|| panic!("a unit's promotions {:?} cannot be promoted from the root", unit.promoted));
        let max_add = max_additional(space.max_total, &state, &unit.promoted, &unit.cands);
        let sum = unit.promoted.sum();
        let level =
            Level { state, promoted: unit.promoted, sum, cands: unit.cands, max_add, known_end: 0, checked: None };
        self.known.clear();
        if self.levels.is_empty() {
            self.levels.push(level);
        } else {
            self.levels[0] = level;
        }
        if let Some(matching) = &mut self.matching {
            matching.reset(&unit.promoted, &unit.cands);
        }
    }

    /// One step of the loop of the node at `depth`: tries its next candidate.  Returns the depth of the node to
    /// work on next (one deeper if a child was pushed), or `None` when the unit is finished.
    fn step(&mut self, depth: usize, ctx: &mut Ctx<'_, TaxUnit<W>>) -> Option<usize> {
        if self.levels.len() == depth + 1 {
            let copy = self.levels[depth];
            self.levels.push(copy);
        }
        let space = self.space;
        let (path, rest) = self.levels.split_at_mut(depth + 1);
        let (node, child) = (&mut path[depth], &mut rest[0]);
        self.known.truncate(node.known_end); // forget the conflicts of the child last considered
        if let Some(m) = &mut self.matching {
            m.take_kept(); // the node without the candidate it last branched on, if strong branching kept it
        }
        let bounds = &mut self.bounds;
        let matching = &mut self.matching;
        let probe = &mut self.probe;
        if let Some(p) = probe {
            bump(&mut p.local.steps, node.promoted.len() as usize);
        }

        // The node's loop ends when its candidates are used up or cannot beat the incumbent.  Its bound is
        // blameGame's or (tactic `match_bound`) the matching bound; both are at most the sum of the `max_add`
        // largest candidates, so that sum may decide first.
        let best = ctx.best();
        let need = (best + 1).saturating_sub(node.sum); // the loop goes on while the bound is at least this
        let branching = space.conflict_branching;
        let mut conflict = None;
        let mut found = Bits::EMPTY;
        let checked = node.checked.take();
        let mut forced = Bits::EMPTY; // candidates that every completion reaching `need` promotes
        let ended = if node.cands.is_empty() {
            Some(cut::EMPTY)
        } else if node.cands.sum_largest(node.max_add) < need {
            Some(cut::LARGEST)
        } else if space.match_graph.is_none() && !bounds.reaches(&node.state, &node.cands, node.max_add, need) {
            Some(cut::BLAME)
        } else if let Some((_, k, members)) = checked.filter(|&(at, _, _)| at == need) {
            // The node was checked when it was made, at the same need and with the same matching (its first
            // step): the verdict is the same, and so are the conflict and the forced members.
            (conflict, forced) = (Some(k), members);
            None
        } else {
            let (scratch, known) = (bounds.scratch(), &mut self.known);
            let check = Self::matching_check(
                matching,
                &mut self.counts,
                space,
                scratch,
                known,
                &mut self.conflict_cache,
                &node.state,
                need,
                branching != Branching::Largest,
                &mut found,
                self.nogoods.as_mut().map(|table| (table, node.promoted)),
                &mut forced,
            );
            node.known_end = known.len();
            if !found.is_empty() {
                Self::offer(space, &mut self.payload, ctx, &(node.promoted | found));
            }
            match check {
                MatchCheck::Cut(reason) => Some(reason),
                MatchCheck::Exact(q) => {
                    Self::offer(space, &mut self.payload, ctx, &(node.promoted | q));
                    Some(cut::EXACT)
                }
                MatchCheck::Open(k) => {
                    conflict = k;
                    None
                }
            }
        };
        if let Some(p) = probe {
            match ended {
                Some(reason) => p.local.loop_ends[reason] += 1,
                None => p.open(bounds, matching, space.model, node, need),
            }
        }
        if ended.is_some() {
            if depth > 0 {
                if let Some(m) = matching {
                    m.pop();
                }
            }
            return depth.checked_sub(1);
        }
        // The candidate to branch on: promoted in the child, and no longer a candidate of this node.
        let k = conflict.map(|k| k & node.cands).filter(|k| !k.is_empty());
        let mut kept = false; // whether the matching level without n is kept for the node
        forced &= node.cands;
        let n = match (branching, k) {
            (Branching::ConflictSmallest, Some(k)) => k.min(),
            (Branching::ConflictLargest, Some(k)) => k.max(),
            // With forced members known, the child promotes them all and the node's loop ends there.
            (Branching::ConflictStrong, Some(_)) if !forced.is_empty() => forced.max(),
            (Branching::ConflictStrong, Some(k)) if k.len() > 1 => {
                let m = matching.as_mut().expect("conflicts come from the matching");
                let (n, members, keeps) = strongest(m, &k, need, space.fix_forced);
                (forced, kept) = (members, keeps);
                Some(n)
            }
            (Branching::ConflictStrong, Some(k)) => k.max(),
            _ => node.cands.max(),
        }
        .expect("not empty");
        if k.is_some_and(|k| k.is_subset(&forced)) {
            // Every completion that reaches `need` promotes the whole conflict, so there is none.
            if let Some(p) = probe {
                p.local.loop_ends[cut::FORCED] += 1;
            }
            if depth > 0 {
                if let Some(m) = matching {
                    m.pop();
                }
            }
            return depth.checked_sub(1);
        }
        node.cands.remove(n);
        let left = node.cands; // what the child's candidates are chosen from
        if space.fix_forced && !forced.is_empty() {
            // Every completion that reaches `need` promotes the forced members, so all of them are the child's (or
            // none, if it fails): the node's loop has nothing left after it.
            node.cands = Bits::EMPTY;
        } else if let Some(m) = matching {
            if !kept {
                m.remove_candidate(n);
            }
        }

        self.attempts += 1;
        child.checked = None;
        // Tactic `fix_forced`: the child promotes the node's other forced members too.  Every completion of the
        // node that reaches `need` promotes them, and the child's completions are some of the node's, so the
        // child loses nothing that matters; if they cannot all be promoted (or their frames have no freedoms
        // left for them), the child has no such completion at all.  They are promoted together, each frame
        // checked once (see `Model::promote_all_using`); one by one, each would take a candidate's freedom from
        // its two frames, so together they fit exactly when no frame they touch ends with negative freedoms.
        let added = if space.fix_forced && forced.len() > 1 { forced } else { Bits::EMPTY.with(n) };
        let promoted = if added.len() == 1 {
            child.state.assign(&node.state);
            space.model.promote_in_place_using(bounds.scratch(), &mut child.state, n)
        } else {
            match space.model.promote_all_using(bounds.scratch(), &node.state, &added) {
                Some(state) => {
                    child.state.assign(&state);
                    added.iter().all(|x| {
                        let h = space.model.home(x);
                        let frames = child.state.frames();
                        frames[h].freedoms >= 0 && frames[h - 1].freedoms >= 0
                    })
                }
                None => false,
            }
        };
        if !promoted {
            if let Some(p) = probe {
                let reason = if added.len() == 1 {
                    &mut p.local.failed_promotions
                } else {
                    &mut p.local.child_cuts[cut::FORCED]
                };
                *reason += 1;
            }
            return Some(depth);
        }
        self.promotions += u64::from(added.len());
        child.promoted = node.promoted | added;
        child.sum = node.sum + added.sum();
        let rest = left - added;
        let max_next = max_additional(space.max_total, &child.state, &child.promoted, &rest);
        let mut cands = bounds.new_candidate_list_after(&added, max_next, &child.state, &rest, &node.promoted);
        // A known conflict of the node (or above) that the child promotes all but one member of rules that member
        // out: the child's state is the node's with more promotions, so promoting it would promote the whole
        // conflict there.  (The heaviest matching of the node promotes n already, so without this the child's
        // matching bound would not drop at all.)
        for k in self.known.iter().filter(|k| k.intersects(&added)) {
            let rest = *k - child.promoted;
            if rest.len() == 1 {
                cands -= rest;
            }
        }
        // So does a nogood (tactic `nogoods`), wherever it was learned.
        if let Some(table) = &self.nogoods {
            let out = table.ruled_out(&added, &child.promoted, &cands);
            self.counts.removed += u64::from(out.len());
            cands -= out;
        }
        let need = (best + 1).saturating_sub(child.sum); // the child is worth it if its bound is at least this
        let cut_by = if cands.sum_largest(max_next) < need {
            Some(cut::LARGEST)
        } else if space.match_graph.is_none() && !bounds.reaches(&child.state, &cands, max_next, need) {
            Some(cut::BLAME)
        } else if cands.is_empty() {
            // A child with no candidates has nothing to bound (both bounds are 0) and is not explored.
            None
        } else {
            if let Some(m) = matching {
                m.push_all(&added, &cands);
            }
            let (scratch, known, cache) = (bounds.scratch(), &mut self.known, &mut self.conflict_cache);
            let mut found = Bits::EMPTY;
            let mut forced = Bits::EMPTY;
            let state = &child.state;
            let check = Self::matching_check(
                matching,
                &mut self.counts,
                space,
                scratch,
                known,
                cache,
                state,
                need,
                false,
                &mut found,
                self.nogoods.as_mut().map(|table| (table, child.promoted)),
                &mut forced,
            );
            if !found.is_empty() {
                Self::offer(space, &mut self.payload, ctx, &(child.promoted | found));
            }
            match check {
                MatchCheck::Open(k) => {
                    child.checked = k.map(|k| (need, k, forced));
                    if let (Some(p), Some(m)) = (probe.as_mut(), matching.as_mut()) {
                        p.matching_open(bounds, m, space.model, &child.state, max_next, need);
                    }
                    None
                }
                cut_or_exact => {
                    if let Some(m) = matching {
                        m.pop();
                    }
                    match cut_or_exact {
                        MatchCheck::Exact(q) => {
                            Self::offer(space, &mut self.payload, ctx, &(child.promoted | q));
                            Some(cut::EXACT)
                        }
                        MatchCheck::Cut(reason) => Some(reason),
                        MatchCheck::Open(_) => unreachable!(),
                    }
                }
            }
        };
        if let Some(reason) = cut_by {
            if let Some(p) = probe {
                p.local.child_cuts[reason] += 1;
            }
            return Some(depth);
        }
        Self::offer(space, &mut self.payload, ctx, &child.promoted);
        if cands.is_empty() {
            if let Some(p) = probe {
                p.local.child_cuts[cut::EMPTY] += 1;
            }
            return Some(depth);
        }
        child.cands = cands;
        child.max_add = max_additional(space.max_total, &child.state, &child.promoted, &cands);
        child.known_end = self.known.len();
        self.children += 1;
        if let Some(p) = probe {
            bump(&mut p.local.children, child.promoted.len() as usize);
        }
        Some(depth + 1)
    }

    /// Adds this searcher's counts to the worker's statistics and resets them.
    fn flush(&mut self, stats: &mut Stats) {
        let b = std::mem::take(&mut self.bounds.counts);
        let counts = [
            (stat::PROMOTE_ATTEMPTS, std::mem::take(&mut self.attempts)),
            (stat::PROMOTIONS, std::mem::take(&mut self.promotions)),
            (stat::UPPER_BOUNDS, b.upper_bounds),
            (stat::CAN_PROMOTES, b.can_promotes),
            (stat::CHILDREN, std::mem::take(&mut self.children)),
            (stat::PRECLUDED, b.precluded),
            (stat::CERTIFICATES, b.certificates),
            (stat::MATCH_BOUNDS, std::mem::take(&mut self.counts.match_bounds)),
        ];
        for (i, count) in counts {
            stats.extra[i] += count;
        }
        let refine = std::mem::take(&mut self.counts);
        if let Some(p) = &mut self.probe {
            p.local.refinements += refine.refinements;
            p.local.refine_evaluations += refine.evaluations;
            p.local.refine_conflicts += refine.conflicts;
            if let Some(m) = &self.matching {
                let s = m.stats();
                let now = [s.evaluations, s.searches, s.scanned];
                for ((total, seen), now) in p.local.matching.iter_mut().zip(&mut self.match_seen).zip(now) {
                    *total += now - *seen;
                    *seen = now;
                }
            }
            p.flush();
        }
    }
}

impl<const W: usize> Searcher<TaxUnit<W>> for TaxSearcher<'_, W> {
    fn run(&mut self, unit: TaxUnit<W>, ctx: &mut Ctx<'_, TaxUnit<W>>) {
        self.start(&unit);
        let mut depth = 0;
        loop {
            if ctx.poll() == Poll::Yield {
                for level in &self.levels[..=depth] {
                    ctx.push(TaxUnit { promoted: level.promoted, cands: level.cands });
                }
                break;
            }
            ctx.stats().nodes += 1;
            match self.step(depth, ctx) {
                Some(next) => depth = next,
                None => break,
            }
        }
        self.flush(ctx.stats());
    }
}

// ---------------------------------------------------------------------------------------------------------
// Setup

/// What revising frame 0's freedoms did (`UpperBoundEstimator.reviseFreedoms`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Revise {
    /// The tactic is off.
    Off,
    /// Nothing to revise: the greedy incumbent already has `max_total` promotions, or there are at most 2
    /// frames.
    NotNeeded,
    /// The two-frame game's greedy set has the most promotions frames 0 and 1 allow; frame 0 got that many.
    GreedyIsMax(u32),
    /// Revising to the two-frame greedy size would not change `max_total`, so nothing was searched or changed.
    Unchanged,
    /// The two-frame game was searched; frame 0 got the size of its best set.
    Searched { size: u32, nodes: u64 },
    /// Frame 0's freedoms were taken from a checkpoint.
    FromCheckpoint,
}

/// Wall times of [`prepare`]'s phases.
#[derive(Clone, Copy, Debug, Default)]
pub struct PrepareTimings {
    pub build: Duration,
    pub preclusions: Duration,
    pub greedy: Duration,
    /// Including the two-frame game's own setup and search.
    pub revise: Duration,
}

/// A game set up for [`search`].
pub struct Prepared<const W: usize> {
    pub model: Arc<Model>,
    /// The starting state before revising frame 0.
    pub unrevised: FrameSet<W>,
    /// The starting state of the search.
    pub root: FrameSet<W>,
    pub candidates: Bits<W>,
    pub tactics: SearchTactics,
    /// Present when the `preclusions` tactic is on.
    pub preclusions: Option<Preclusions<W>>,
    /// The greedy incumbent.
    pub greedy: Bits<W>,
    /// `root.max_promotions_among(candidates)`, at least 0.
    pub max_total: u32,
    pub revise: Revise,
    pub timings: PrepareTimings,
}

/// Sets up the search for the game whose frames are `columns` (factors and moves per frame, bottom first) and
/// whose promotion candidates are `candidates`.  `threads` is used for the preclusion table and for the
/// two-frame search of `revise_freedoms`.
pub fn prepare<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    candidates: &Bits<W>,
    tactics: SearchTactics,
    threads: usize,
) -> Result<Prepared<W>, String> {
    let t0 = Instant::now();
    let (model, root) = Model::build::<W>(tables, columns)?;
    let build = t0.elapsed();
    let mut prepared = prepare_state(Arc::new(model), root, *candidates, tactics, threads, None)?;
    prepared.timings.build = build;
    Ok(prepared)
}

/// [`prepare`] for resuming the search checkpointed in `checkpoint`: frame 0's freedoms come from the
/// checkpoint's header instead of being revised again (which can take a search of its own), and the tactics
/// are the checkpoint's.  Fails if the checkpoint was made for a different game or width.
pub fn prepare_for_resume<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    candidates: &Bits<W>,
    checkpoint: &Path,
    threads: usize,
) -> Result<Prepared<W>, String> {
    let bytes = engine::read_checkpoint_header(checkpoint).map_err(|e| format!("{}: {e}", checkpoint.display()))?;
    let header = Header::<W>::decode(&bytes)?;
    if header.candidates != *candidates {
        return Err("the checkpoint is for different candidates".into());
    }
    let t0 = Instant::now();
    let (model, root) = Model::build::<W>(tables, columns)?;
    let build = t0.elapsed();
    let mut prepared =
        prepare_state(Arc::new(model), root, *candidates, header.tactics, threads, Some(header.frame0_freedoms))?;
    prepared.timings.build = build;
    if prepared.max_total != header.max_total {
        return Err(format!("max_total {} differs from the checkpoint's {}", prepared.max_total, header.max_total));
    }
    Ok(prepared)
}

/// The setup after the frame set exists.  `frame0` (from a checkpoint) replaces revising.
fn prepare_state<const W: usize>(
    model: Arc<Model>,
    root: FrameSet<W>,
    candidates: Bits<W>,
    tactics: SearchTactics,
    threads: usize,
    frame0: Option<Option<i32>>,
) -> Result<Prepared<W>, String> {
    let mut timings = PrepareTimings::default();
    let t0 = Instant::now();
    let preclusions = tactics.preclusions.then(|| Preclusions::build(&model, &root, &candidates, threads));
    timings.preclusions = t0.elapsed();
    let t0 = Instant::now();
    let greedy = bounds::greedy(&model, &root, &candidates);
    timings.greedy = t0.elapsed();
    let t0 = Instant::now();
    let (revised, revise) = match frame0 {
        Some(Some(freedoms)) => (root.with_frame_zero_freedoms(freedoms), Revise::FromCheckpoint),
        Some(None) => (root, Revise::FromCheckpoint),
        None if tactics.revise_freedoms => revise_freedoms(&model, &root, &candidates, &greedy, tactics, threads)?,
        None => (root, Revise::Off),
    };
    timings.revise = t0.elapsed();
    let max_total = revised.max_promotions_among(&candidates).max(0) as u32;
    Ok(Prepared {
        model,
        unrevised: root,
        root: revised,
        candidates,
        tactics,
        preclusions,
        greedy,
        max_total,
        revise,
        timings,
    })
}

/// The Java's `UpperBoundEstimator.reviseFreedoms`.  When the greedy incumbent has fewer promotions than the
/// freedoms allow and there are more than 2 frames, the promotions into frame 0 may be fewer than its freedoms
/// suggest.  The two-frame game {frame 0, frame 1} with frame 1's candidates settles how many: by its greedy
/// set if that reaches `min(freedoms of frames 0 and 1)`, else by searching it, but only if that could change
/// the full game's cap on promotions.  Frame 0's freedoms become that number.
fn revise_freedoms<const W: usize>(
    model: &Arc<Model>,
    root: &FrameSet<W>,
    candidates: &Bits<W>,
    greedy: &Bits<W>,
    tactics: SearchTactics,
    threads: usize,
) -> Result<(FrameSet<W>, Revise), String> {
    let full_max = root.max_promotions_among(candidates);
    if i64::from(greedy.len()) == i64::from(full_max) || root.len() <= 2 {
        return Ok((*root, Revise::NotNeeded));
    }
    let pair = root.front_pair();
    let (f0, f1) = (&pair.frames()[0], &pair.frames()[1]);
    let best_possible = f0.freedoms.min(f1.freedoms);
    let small = prepare_state(model.clone(), pair, *candidates & f1.factors, tactics, threads, None)?;
    let small_greedy = small.greedy.len();
    if i64::from(small_greedy) == i64::from(best_possible) {
        return Ok((root.with_frame_zero_freedoms(small_greedy as i32), Revise::GreedyIsMax(small_greedy)));
    }
    if root.with_frame_zero_freedoms(small_greedy as i32).max_promotions_among(candidates) == full_max {
        return Ok((*root, Revise::Unchanged));
    }
    let cfg = EngineConfig { threads, label: "revise".into(), progress: None, ..EngineConfig::default() };
    let found = search(&small, &cfg, None, None, false).map_err(|e| format!("revise search: {e}"))?;
    let size = found.promotions.len();
    Ok((root.with_frame_zero_freedoms(size as i32), Revise::Searched { size, nodes: found.stats.nodes }))
}

// ---------------------------------------------------------------------------------------------------------
// Checkpoint header

/// What a checkpoint records about the search it belongs to, so that a resumed run can check it is the same
/// search and skip revising frame 0 again.
struct Header<const W: usize> {
    candidates: Bits<W>,
    max_total: u32,
    /// Frame 0's freedoms in the search's root (`None` without frames).
    frame0_freedoms: Option<i32>,
    tactics: SearchTactics,
}

impl<const W: usize> Header<W> {
    const MAGIC: &'static [u8; 4] = b"TXS2";

    fn of(prepared: &Prepared<W>) -> Self {
        Header {
            candidates: prepared.candidates,
            max_total: prepared.max_total,
            frame0_freedoms: prepared.root.frames().first().map(|f| f.freedoms),
            tactics: prepared.tactics,
        }
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Self::MAGIC.to_vec();
        out.extend_from_slice(&(W as u32).to_le_bytes());
        put_bits(&mut out, &self.candidates);
        out.extend_from_slice(&self.max_total.to_le_bytes());
        out.push(u8::from(self.frame0_freedoms.is_some()));
        out.extend_from_slice(&self.frame0_freedoms.unwrap_or(0).to_le_bytes());
        out.extend_from_slice(&self.tactics.bits().to_le_bytes());
        out
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        let len = 4 + 4 + 8 * W + 4 + 1 + 4 + 2;
        if bytes.len() != len || &bytes[..4] != Self::MAGIC {
            return Err("not a checkpoint of a Taxman search".into());
        }
        let u32_at = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().expect("4 bytes"));
        let width = u32_at(4) as usize;
        if width != W {
            return Err(format!("the checkpoint was made with Bits<{width}>, not Bits<{W}>"));
        }
        let mut at = 8 + 8 * W;
        let candidates = get_bits(&bytes[8..at]);
        let max_total = u32_at(at);
        let has_frame0 = bytes[at + 4] != 0;
        let frame0 = u32_at(at + 5) as i32;
        at += 9;
        Ok(Header {
            candidates,
            max_total,
            frame0_freedoms: has_frame0.then_some(frame0),
            tactics: SearchTactics::from_bits(u16::from_le_bytes([bytes[at], bytes[at + 1]]))?,
        })
    }
}

// ---------------------------------------------------------------------------------------------------------
// Search

/// The outcome of [`search`].
#[derive(Clone, Debug)]
pub struct SearchResult<const W: usize> {
    /// The best promotion set found (the optimum when `completed`).
    pub promotions: Bits<W>,
    pub sum: u64,
    /// The search finished (or reached the cap); false when an engine limit stopped it.
    pub completed: bool,
    /// The incumbent reached the cap, which proves it optimal.
    pub reached_cap: bool,
    /// Engine statistics; see [`stat`] for `extra`.
    pub stats: Stats,
    pub counters: Counters,
    /// Wall time of this call.
    pub time: Duration,
    /// When this call found its best incumbent (`None` if it found none better than the starting one).
    pub found: Option<Found>,
    /// The incumbent the search started from: the best of the greedy set, the seed and (with `match_bound`) the
    /// matching dive.
    pub start: Bits<W>,
}

/// The candidates of `seed` that can still be promoted, one by one from the largest down, skipping the ones
/// that fail.
fn surviving<const W: usize>(prepared: &Prepared<W>, seed: &Bits<W>) -> Bits<W> {
    let mut state = prepared.root;
    let mut kept = Bits::EMPTY;
    for n in (*seed & prepared.candidates).iter_desc() {
        if prepared.model.promote_in_place(&mut state, n) {
            kept.insert(n);
        }
    }
    kept
}

/// Where a local search for better promotion sets works: the search's model, root and candidates.
#[derive(Clone, Copy)]
pub struct Neighbourhood<'a, const W: usize> {
    pub model: &'a Model,
    pub root: &'a FrameSet<W>,
    pub candidates: &'a Bits<W>,
}

impl<const W: usize> Neighbourhood<'_, W> {
    /// `base` (which must be feasible) with candidates outside `excluded` added greedily, from the largest down,
    /// narrowing the candidates after each promotion as [`bounds::greedy`] does.
    fn fill(&self, scratch: &mut Scratch<W>, base: &Bits<W>, excluded: &Bits<W>) -> Bits<W> {
        let mut state = self.model.promote_all_using(scratch, self.root, base).expect("a feasible base");
        let mut out = *base;
        let mut open = state.narrow_candidates(&(*self.candidates - *base - *excluded));
        let mut next = open.max();
        while let Some(n) = next {
            if self.model.promote_in_place_using(scratch, &mut state, n) {
                out.insert(n);
                open = state.narrow_candidates(&open);
            }
            next = open.max_below(n);
        }
        out
    }

    /// A promotion set at least as good as `start` (which must be feasible), by local search: fill `start`
    /// greedily, then repeatedly take one member out (and with `pairs`, then two) and refill greedily without
    /// them ([`fill`](Self::fill)), keeping the first change that raises the sum, until none does.  A few
    /// milliseconds with `pairs` at n ≈ 1000, less without.
    pub fn improve(&self, scratch: &mut Scratch<W>, start: &Bits<W>, pairs: bool) -> Bits<W> {
        let mut best = self.fill(scratch, start, &Bits::EMPTY);
        'search: loop {
            for x in best.iter() {
                let other = self.fill(scratch, &best.without(x), &Bits::EMPTY.with(x));
                if other.sum() > best.sum() {
                    best = other;
                    continue 'search;
                }
            }
            if pairs {
                for x in best.iter() {
                    for y in best.iter().filter(|&y| y < x) {
                        let out = Bits::EMPTY.with(x).with(y);
                        let other = self.fill(scratch, &(best - out), &out);
                        if other.sum() > best.sum() {
                            best = other;
                            continue 'search;
                        }
                    }
                }
            }
            return best;
        }
    }
}

/// A promotion set found by rounding the matching relaxation at `root` (a starting incumbent): while the
/// heaviest matching's candidates cannot all be promoted, take the smallest member of one of their conflicts
/// (see [`Refiner`]) out of the candidates and solve again.  Ends with candidates that can all be promoted.
fn dive<const W: usize>(model: &Model, root: &FrameSet<W>, graph: &MatchGraph, cands: &Bits<W>) -> Bits<W> {
    let mut m = MatchBound::new(graph);
    m.reset(&Bits::EMPTY, cands);
    let mut scratch = Scratch::new();
    loop {
        let chosen = m.chosen();
        match model.conflict_using(&mut scratch, root, &chosen, true).and_then(|k| k.min()) {
            Some(x) => m.remove_candidate(x),
            None => return chosen,
        }
    }
}

/// Searches for the promotion set with the largest sum.
///
/// The incumbent starts as the greedy set, or as what survives of `seed` (see [`surviving`]) if that is
/// better.  `cap` is a proven ceiling on the sum; reaching it ends the search.  With `resume`, the search
/// continues from the checkpoint configured in `cfg` (the `prepared` game must have been set up by
/// [`prepare_for_resume`] or identically); otherwise it starts fresh.
pub fn search<const W: usize>(
    prepared: &Prepared<W>,
    cfg: &EngineConfig,
    seed: Option<&Bits<W>>,
    cap: Option<u64>,
    resume: bool,
) -> io::Result<SearchResult<W>> {
    search_with(prepared, cfg, seed, cap, resume, None)
}

/// [`search`], collecting [`Diagnostics`] into `diagnostics` if given.
pub fn search_with<const W: usize>(
    prepared: &Prepared<W>,
    cfg: &EngineConfig,
    seed: Option<&Bits<W>>,
    cap: Option<u64>,
    resume: bool,
    diagnostics: Option<&Mutex<Diagnostics>>,
) -> io::Result<SearchResult<W>> {
    let t0 = Instant::now();
    let match_graph = prepared.tactics.match_bound.then(|| MatchGraph::new(&prepared.model, &prepared.root));
    let mut incumbent = prepared.greedy;
    if let Some(seed) = seed {
        let kept = surviving(prepared, seed);
        if kept.sum() > incumbent.sum() {
            incumbent = kept;
        }
    }
    if let Some(graph) = &match_graph {
        let dived = dive(&prepared.model, &prepared.root, graph, &prepared.candidates);
        if dived.sum() > incumbent.sum() {
            incumbent = dived;
        }
    }
    let hood = Neighbourhood { model: &prepared.model, root: &prepared.root, candidates: &prepared.candidates };
    incumbent = hood.improve(&mut Scratch::new(), &incumbent, true);
    if !resume && cap.is_some_and(|cap| incumbent.sum() >= cap) {
        return Ok(SearchResult {
            promotions: incumbent,
            sum: incumbent.sum(),
            completed: true,
            reached_cap: true,
            stats: Stats::default(),
            counters: Counters::default(),
            time: t0.elapsed(),
            found: None,
            start: incumbent,
        });
    }

    let space = SearchSpace {
        model: &prepared.model,
        root: &prepared.root,
        preclusions: prepared.preclusions.as_ref(),
        match_graph: match_graph.as_ref(),
        match_refine: u32::from(prepared.tactics.match_refine),
        conflict_branching: if prepared.tactics.match_bound { prepared.tactics.branching } else { Branching::Largest },
        fix_forced: prepared.tactics.fix_forced,
        nogoods: prepared.tactics.match_bound && prepared.tactics.nogoods,
        root_max: prepared.root.frames().iter().filter_map(|f| (f.moves | f.factors).max()).max().unwrap_or(0),
        max_total: prepared.max_total,
        rule: prepared.tactics.rule(),
        cap,
        diagnostics,
        began: t0,
        found: Mutex::new(None),
    };
    let start = if resume {
        let Some((path, _)) = &cfg.checkpoint else {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "resuming needs a checkpoint file"));
        };
        Start::Resume(path.clone())
    } else {
        let mut payload = Vec::new();
        put_bits(&mut payload, &incumbent);
        let root = TaxUnit { promoted: Bits::EMPTY, cands: prepared.candidates };
        Start::Fresh { units: vec![root], best: incumbent.sum(), payload }
    };
    let header = Header::of(prepared).encode();
    let outcome = engine::run(cfg, start, &header, |_| TaxSearcher::new(&space))?;
    let promotions = get_bits(&outcome.payload);
    assert_eq!(promotions.sum(), outcome.best, "the incumbent's payload does not match its value");
    Ok(SearchResult {
        promotions,
        sum: outcome.best,
        completed: outcome.completed,
        reached_cap: outcome.optimal_declared,
        stats: outcome.stats,
        counters: outcome.counters,
        time: t0.elapsed(),
        found: space.found.into_inner().unwrap_or_else(PoisonError::into_inner),
        start: incumbent,
    })
}

// ---------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::build_frames;
    use crate::json::Json;
    use crate::model::test_support::*;
    use crate::model::Scratch;
    use std::path::PathBuf;

    fn config(threads: usize) -> EngineConfig {
        EngineConfig { threads, progress: None, ..EngineConfig::default() }
    }

    /// The golden games the Java had to search (not accelerated), up to `max_n`.
    fn searched_games(max_n: u32) -> Vec<Golden> {
        golden_ns()
            .into_iter()
            .filter(|&n| n <= max_n)
            .map(load)
            .filter(|g| g.json.get("solve").and_then(|s| s.get("accelerated")).and_then(Json::as_bool) == Some(false))
            .collect()
    }

    /// The sum of the Java's optimal promotions.
    fn golden_sum(g: &Golden) -> u64 {
        u32s(g.json.get("solve").expect("solve"), "promotions").iter().map(|&x| u64::from(x)).sum()
    }

    fn prepare_golden<const W: usize>(g: &Golden, tactics: SearchTactics) -> Prepared<W> {
        prepare::<W>(&Tables::new(g.n), &g.columns, &Bits::from_slice(&g.candidates), tactics, 2).unwrap()
    }

    /// Searches and checks that the result is complete, made of candidates, and feasible.
    fn search_checked<const W: usize>(p: &Prepared<W>, threads: usize, what: &str) -> SearchResult<W> {
        let r = search(p, &config(threads), None, None, false).unwrap();
        assert!(r.completed, "{what}: incomplete");
        assert!(r.promotions.is_subset(&p.candidates), "{what}: {:?} are not all candidates", r.promotions);
        let feasible = p.model.promote_all_using(&mut Scratch::new(), &p.root, &r.promotions);
        assert!(feasible.is_some(), "{what}: {:?} cannot be promoted", r.promotions);
        r
    }

    fn check_against_java<const W: usize>(g: &Golden) {
        let p = prepare_golden::<W>(g, SearchTactics::default());
        let manager = g.json.get("searchManager").expect("searchManager");
        let frame0 = manager.get("revisedFrame0Freedoms").and_then(Json::as_i64);
        assert_eq!(p.root.frames().first().map(|f| i64::from(f.freedoms)), frame0, "n={} frame 0 freedoms", g.n);
        let max_total = manager.get("maxTotalPromotions").and_then(Json::as_i64).expect("maxTotalPromotions");
        assert_eq!(i64::from(p.max_total), max_total, "n={} max_total ({:?})", g.n, p.revise);
        for threads in [1, 4] {
            let r = search_checked(&p, threads, &format!("n={} with {threads} threads", g.n));
            assert_eq!(r.sum, golden_sum(g), "n={} with {threads} threads: {:?}", g.n, r.promotions);
        }
    }

    #[test]
    fn golden_games_up_to_400_match_the_java() {
        let games = searched_games(400);
        assert!(games.len() > 80, "only {} games", games.len());
        for g in &games {
            check_against_java::<8>(g);
        }
    }

    /// Checks the refined matching bound along random search paths of `g`: it never prunes a node whose best
    /// completion (found exhaustively) reaches `need`, the completions it calls exact or finds can be promoted,
    /// and the conflicts it keeps, carried down the path, stay conflicts.  Returns the numbers of verdicts, of
    /// prunings the plain matching bound would not make, and of exact completions.
    fn refinement_soundness<const W: usize>(g: &Golden, rng: &mut Rng) -> [u64; 3] {
        let (model, root) = Model::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
        let all = Bits::<W>::from_slice(&g.candidates);
        let max_total = root.max_promotions_among(&all).max(0) as u32;
        let preclusions = Preclusions::build(&model, &root, &all, 2);
        let graph = MatchGraph::new(&model, &root);
        let mut m = MatchBound::new(&graph);
        let mut scratch = Scratch::new();
        let mut table = Nogoods::new(g.n, 1 << 12);
        let mut checks = [0; 3];
        for _path in 0..6 {
            let (mut state, mut promoted, mut cands) = (root, Bits::<W>::EMPTY, all);
            let mut known = Vec::new();
            let mut cache = ConflictCache::default();
            loop {
                let max_add = max_additional(max_total, &state, &promoted, &cands);
                let truth = best_completion(&model, &state, &cands, max_add);
                let what = format!("n={} promoted {promoted:?} cands {cands:?} best {truth}", g.n);
                for depth in [0, 1, 2, 4] {
                    for need in [truth.saturating_sub(7), truth, truth + 1, truth + 9] {
                        m.reset(&promoted, &cands);
                        let plain = m.value();
                        // Half the time with the nogoods learned so far, on this path and others.
                        let learning = rng.chance(50).then_some(Learning { table: &mut table, root: &root, promoted });
                        let mut refiner = Refiner {
                            m: &mut m,
                            model: &model,
                            scratch: &mut scratch,
                            state: &state,
                            need,
                            known: &mut known,
                            cache: &mut cache,
                            evaluations: 0,
                            computed: 0,
                            hits: 0,
                            exact: Bits::EMPTY,
                            learning,
                            forcing: rng.chance(50),
                            forced: Bits::EMPTY,
                        };
                        let verdict = refiner.top(depth, rng.chance(50));
                        let found = refiner.exact;
                        if need <= truth {
                            assert_ne!(verdict, Verdict::Below, "{what}: depth {depth} prunes at need {need}");
                        }
                        let exact = match verdict {
                            Verdict::Exact(q) => Some(q),
                            _ => None,
                        };
                        for q in [exact, Some(found).filter(|f| !f.is_empty())].into_iter().flatten() {
                            assert!(q.is_subset(&cands) && q.sum() >= need, "{what}: exact {q:?} at need {need}");
                            assert!(model.can_promote_all_using(&mut scratch, &state, &q), "{what}: {q:?} fails");
                        }
                        checks[0] += 1;
                        checks[1] += u64::from(verdict == Verdict::Below && plain >= need);
                        checks[2] += u64::from(exact.is_some() || !found.is_empty());
                    }
                }
                for k in &known {
                    assert!(!model.can_promote_all_using(&mut scratch, &state, k), "{what}: {k:?} is no conflict");
                }
                // Descend into a random child, as the search would (in any order).
                let members = cands.to_vec();
                if members.is_empty() {
                    break;
                }
                let Some((n, next)) = (0..4).find_map(|_| {
                    let n = rng.pick(&members);
                    model.promote(&state, n).map(|s| (n, s))
                }) else {
                    break;
                };
                let rest = cands.without(n);
                let max_next = max_additional(max_total, &next, &promoted.with(n), &rest);
                cands = bounds::new_candidate_list(&model, Some(&preclusions), n, max_next, &next, &rest, &promoted);
                (state, promoted) = (next, promoted.with(n));
            }
        }
        checks
    }

    #[test]
    fn refined_matching_bound_is_sound() {
        let mut rng = Rng(0x5EED_C0F1_1C75_0001);
        let mut checks = [0; 3];
        for n in golden_ns().into_iter().filter(|&n| (40..=220).contains(&n)) {
            let more = refinement_soundness::<4>(&load(n), &mut rng);
            for (a, b) in checks.iter_mut().zip(more) {
                *a += b;
            }
        }
        let [verdicts, refined, exact] = checks;
        eprintln!("refined matching bound: {verdicts} verdicts, {refined} refined prunings, {exact} exact completions");
        assert!(verdicts > 5000 && refined > 500 && exact > 500);
    }

    /// Checks the forced members of strong branching (tactic `fix_forced`) along random search paths of `g`: at a
    /// node whose heaviest matching has a conflict, every forced member is in every completion that reaches
    /// `need` (found exhaustively), so the child that promotes all of them keeps the best completion whenever it
    /// reaches `need`.  Returns the numbers of nodes checked and of forced members.
    fn forced_soundness<const W: usize>(g: &Golden, rng: &mut Rng) -> [u64; 2] {
        let (model, root) = Model::build::<W>(&Tables::new(g.n), &g.columns).unwrap();
        let all = Bits::<W>::from_slice(&g.candidates);
        let max_total = root.max_promotions_among(&all).max(0) as u32;
        let preclusions = Preclusions::build(&model, &root, &all, 2);
        let graph = MatchGraph::new(&model, &root);
        let mut m = MatchBound::new(&graph);
        let mut scratch = Scratch::new();
        let mut checks = [0; 2];
        for _path in 0..6 {
            let (mut state, mut promoted, mut cands) = (root, Bits::<W>::EMPTY, all);
            loop {
                let max_add = max_additional(max_total, &state, &promoted, &cands);
                let truth = best_completion(&model, &state, &cands, max_add);
                for need in [truth.saturating_sub(5), truth, truth + 1] {
                    m.reset(&promoted, &cands);
                    if m.value() < need {
                        continue;
                    }
                    let chosen = m.chosen();
                    let Some(k) = model.conflict_using(&mut scratch, &state, &chosen, true) else { continue };
                    let k = k & cands;
                    if k.len() < 2 {
                        continue;
                    }
                    let what = format!("n={} promoted {promoted:?} cands {cands:?} need {need}", g.n);
                    // The members of the heaviest matching forced by the potentials alone, or else those of the
                    // conflict that strong branching finds.
                    let quick: Bits<W> = chosen.iter().filter(|&x| m.bound_excluding(x) < need).collect();
                    let forced = if quick.is_empty() || rng.chance(30) {
                        let (n, forced, _) = strongest(&mut m, &k, need, rng.chance(50));
                        assert!(k.contains(n) && forced.is_subset(&k), "{what}: {n} {forced:?} from {k:?}");
                        assert!(forced.is_empty() || forced.contains(n), "{what}: {n} is not among {forced:?}");
                        assert!((quick & k).is_subset(&forced), "{what}: {quick:?} not all in {forced:?}");
                        forced
                    } else {
                        quick
                    };
                    for x in forced.iter() {
                        let without = best_completion(&model, &state, &cands.without(x), max_add);
                        assert!(without < need, "{what}: {x} is forced, but leaving it out reaches {without}");
                    }
                    if truth >= need && !forced.is_empty() {
                        let next = model.promote_all_using(&mut scratch, &state, &forced);
                        let next = next.unwrap_or_else(|| panic!("{what}: the forced {forced:?} cannot be promoted"));
                        let rest = best_completion(&model, &next, &(cands - forced), max_add - forced.len());
                        assert_eq!(rest + forced.sum(), truth, "{what}: promoting {forced:?} loses the best");
                    }
                    checks[0] += 1;
                    checks[1] += u64::from(forced.len());
                }
                // Descend into a random child, as the search would (in any order).
                let members = cands.to_vec();
                if members.is_empty() {
                    break;
                }
                let Some((n, next)) = (0..4).find_map(|_| {
                    let n = rng.pick(&members);
                    model.promote(&state, n).map(|s| (n, s))
                }) else {
                    break;
                };
                let rest = cands.without(n);
                let max_next = max_additional(max_total, &next, &promoted.with(n), &rest);
                cands = bounds::new_candidate_list(&model, Some(&preclusions), n, max_next, &next, &rest, &promoted);
                (state, promoted) = (next, promoted.with(n));
            }
        }
        checks
    }

    #[test]
    fn forced_members_are_in_every_good_completion() {
        let mut rng = Rng(0xF0BC_ED00_1234_5678);
        let mut checks = [0; 2];
        for n in golden_ns().into_iter().filter(|&n| (40..=220).contains(&n)) {
            let more = forced_soundness::<4>(&load(n), &mut rng);
            for (a, b) in checks.iter_mut().zip(more) {
                *a += b;
            }
        }
        let [nodes, forced] = checks;
        eprintln!("forced members: {nodes} nodes with a conflict to branch on, {forced} forced members");
        assert!(nodes > 500 && forced > 300);
    }

    #[test]
    fn golden_games_above_400_match_the_java() {
        let games: Vec<Golden> = golden_ns().into_iter().filter(|&n| n > 400).map(load).collect();
        assert_eq!(games.len(), 8);
        for g in &games {
            check_against_java::<16>(g);
        }
    }

    fn sums_by_tactics<const W: usize>(g: &Golden) -> Vec<(SearchTactics, u64)> {
        let default = SearchTactics::default();
        let variants = [
            default,
            SearchTactics { preclusions: false, ..default },
            SearchTactics { legacy_bound: true, ..default },
            SearchTactics { revise_freedoms: false, ..default },
            SearchTactics { match_bound: false, ..default },
            SearchTactics { branching: Branching::Largest, match_refine: 0, ..default },
            SearchTactics { branching: Branching::ConflictSmallest, match_refine: 0, ..default },
            SearchTactics { branching: Branching::ConflictLargest, match_refine: 2, ..default },
            SearchTactics { branching: Branching::ConflictLargest, ..default },
            SearchTactics { branching: Branching::Largest, match_refine: 3, ..default },
            SearchTactics { fix_forced: false, ..default },
            SearchTactics { fix_forced: false, match_refine: 0, ..default },
            SearchTactics { nogoods: false, ..default },
            SearchTactics { nogoods: false, match_refine: 0, ..default },
            SearchTactics { match_refine: 0, ..default },
            SearchTactics { branching: Branching::ConflictSmallest, match_refine: 2, ..default },
        ];
        variants.iter().map(|&t| (t, search_checked(&prepare_golden::<W>(g, t), 1, "tactics").sum)).collect()
    }

    #[test]
    fn tactics_do_not_change_answers() {
        let mut differences = Vec::new();
        for g in &searched_games(400) {
            let sums = sums_by_tactics::<8>(g);
            for &(tactics, sum) in &sums[1..] {
                if sum != sums[0].1 {
                    differences.push(format!("n={}: {tactics:?} gives {sum}, default {}", g.n, sums[0].1));
                }
            }
        }
        assert!(differences.is_empty(), "AN UNPROVEN TACTIC CHANGES ANSWERS:\n{}", differences.join("\n"));
    }

    /// A fresh directory under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("taxman-search-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("create temp dir");
            TempDir(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Searches 760 in pieces of about a sixth of its nodes each, resuming from the checkpoint, and checks the
    /// answer.  The engine checks the node limit every 1024 nodes, so a search of fewer than 6·1024 nodes stops
    /// less often: with one thread it must still take `min_pieces` pieces.
    fn resume_in_pieces(threads: usize, tactics: SearchTactics, min_pieces: usize) {
        let g = load(760);
        let tables = Tables::new(g.n);
        let cands = Bits::<16>::from_slice(&g.candidates);
        let whole = prepare::<16>(&tables, &g.columns, &cands, tactics, 2).unwrap();
        let nodes = search_checked(&whole, 1, "uninterrupted").stats.nodes;

        let dir = TempDir::new(&format!("resume{threads}-{}", tactics.bits()));
        let ckpt = dir.0.join("ckpt.bin");
        let cfg = EngineConfig {
            checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
            node_limit: Some(nodes / 6 + 1),
            queue_mem_cap: 4,
            spill_dir: Some(dir.0.join("spill")),
            ..config(threads)
        };
        let mut result = search(&whole, &cfg, None, None, false).unwrap();
        let mut pieces = 1;
        while !result.completed {
            assert!(ckpt.exists(), "a stopped run leaves a checkpoint");
            let resumed = prepare_for_resume::<16>(&tables, &g.columns, &cands, &ckpt, 2).unwrap();
            assert_eq!(resumed.revise, Revise::FromCheckpoint);
            assert_eq!(resumed.root, whole.root);
            result = search(&resumed, &cfg, None, None, true).unwrap();
            pieces += 1;
            assert!(pieces < 100, "no progress");
        }
        // One thread explores in depth-first order however often it is interrupted, but the conflicts a searcher
        // learns along its path are not part of a work unit, so a resumed search can prune a little differently
        // (it does not at the time of writing); several threads explore a varying number of nodes anyway.
        assert!(pieces >= if threads == 1 { min_pieces } else { 2 }, "only {pieces} pieces");
        assert!(!ckpt.exists(), "a completed run deletes its checkpoint");
        assert_eq!(result.sum, golden_sum(&g));
    }

    #[test]
    fn checkpoint_and_resume_one_thread() {
        resume_in_pieces(1, SearchTactics::default(), 3);
        resume_in_pieces(1, SearchTactics { fix_forced: false, ..SearchTactics::default() }, 4);
    }

    #[test]
    fn checkpoint_and_resume_four_threads() {
        // The default tactics search 760 in too few nodes for four threads to reach the node limit before the
        // end; the Java's branching takes many more.
        resume_in_pieces(4, SearchTactics { branching: Branching::Largest, ..SearchTactics::default() }, 2);
    }

    #[test]
    fn a_checkpoint_of_another_game_is_refused() {
        let (a, b) = (load(760), load(720));
        let dir = TempDir::new("refuse");
        let ckpt = dir.0.join("ckpt.bin");
        let pa = prepare_golden::<16>(&a, SearchTactics::default());
        let cfg = EngineConfig {
            checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
            node_limit: Some(50),
            ..config(1)
        };
        assert!(!search(&pa, &cfg, None, None, false).unwrap().completed);
        let cands = Bits::<16>::from_slice(&b.candidates);
        assert!(prepare_for_resume::<16>(&Tables::new(b.n), &b.columns, &cands, &ckpt, 1).is_err());
        let cands = Bits::<32>::from_slice(&a.candidates);
        assert!(prepare_for_resume::<32>(&Tables::new(a.n), &a.columns, &cands, &ckpt, 1).is_err());
    }

    #[test]
    fn seeds_and_caps() {
        let g = load(300);
        let p = prepare_golden::<8>(&g, SearchTactics::default());
        let optimum = search_checked(&p, 1, "plain");
        // Seeded with the optimum (plus a non-candidate), the search only has to prove it.
        let seed = optimum.promotions.with(299);
        let seeded = search(&p, &config(1), Some(&seed), None, false).unwrap();
        assert_eq!((seeded.sum, seeded.promotions), (optimum.sum, optimum.promotions));
        assert!(seeded.stats.nodes <= optimum.stats.nodes);
        // A cap the seed already reaches ends the search before it starts; a cap the search reaches ends it
        // there.
        let capped = search(&p, &config(1), Some(&seed), Some(optimum.sum), false).unwrap();
        assert!(capped.completed && capped.reached_cap && capped.stats.nodes == 0);
        let capped = search(&p, &config(1), None, Some(optimum.sum), false).unwrap();
        assert!(capped.completed && capped.reached_cap && capped.sum == optimum.sum);
        assert!(capped.stats.nodes <= optimum.stats.nodes);
        // A worse seed changes nothing.
        let worse = search(&p, &config(1), Some(&Bits::EMPTY), None, false).unwrap();
        assert_eq!(worse.sum, optimum.sum);
    }

    #[test]
    fn units_and_headers_round_trip() {
        let unit = TaxUnit::<3> { promoted: Bits::from_slice(&[1, 64, 190]), cands: Bits::from_slice(&[0, 63, 127]) };
        let mut bytes = Vec::new();
        unit.encode(&mut bytes);
        assert_eq!(bytes.len(), 48);
        assert_eq!(TaxUnit::<3>::decode(&bytes), unit);

        let p = prepare_golden::<8>(&load(300), SearchTactics { legacy_bound: true, ..SearchTactics::default() });
        let header = Header::of(&p);
        let back = Header::<8>::decode(&header.encode()).unwrap();
        assert_eq!(
            (back.candidates, back.max_total, back.frame0_freedoms, back.tactics),
            (p.candidates, p.max_total, p.root.frames().first().map(|f| f.freedoms), p.tactics)
        );
        assert!(Header::<16>::decode(&header.encode()).is_err());
        assert!(Header::<8>::decode(b"nonsense").is_err());

        // Every setting of the search tactics reads back as it was written, and the default tactics of a checkpoint
        // made before `fix_forced` and `nogoods` existed (0x19d) read back as today's defaults.
        let default = SearchTactics::default();
        for refine in [0, 1, 7] {
            for branching in Branching::ALL {
                for (on, off) in [(true, false), (false, true)] {
                    let t = SearchTactics {
                        preclusions: on,
                        legacy_bound: off,
                        revise_freedoms: on,
                        match_bound: off,
                        match_refine: refine,
                        branching,
                        fix_forced: on,
                        nogoods: off,
                    };
                    assert_eq!(SearchTactics::from_bits(t.bits()), Ok(t));
                }
            }
        }
        assert_eq!(SearchTactics::from_bits(0x19d), Ok(default));
        assert!(SearchTactics::from_bits(1 << 11).is_err());
    }

    // ---- benchmark: cargo test --release bench_search -- --ignored --nocapture --test-threads=1 ----

    /// Prepares and searches one game with each thread count and both settings of `match_bound` (`TAXMAN_BENCH_MATCH`
    /// = `on` or `off` restricts it to one), printing one line each.
    fn bench_game<const W: usize>(n: u32, columns: &[Vec<u32>], cands: &Bits<W>, java: (f64, f64)) {
        let tables = Tables::new(n);
        let only = std::env::var("TAXMAN_BENCH_MATCH").ok();
        let settings = [true, false].into_iter().filter(|&on| only.as_deref().is_none_or(|s| (s == "on") == on));
        for match_bound in settings {
            for (threads, java_secs) in [(1, java.0), (10, java.1)] {
                let tactics = SearchTactics { match_bound, ..SearchTactics::default() };
                let t0 = Instant::now();
                let p = prepare::<W>(&tables, columns, cands, tactics, threads).unwrap();
                let r = search(&p, &config(threads), None, None, false).unwrap();
                let total = t0.elapsed().as_secs_f64();
                let t = &p.timings;
                let revise_nodes = if let Revise::Searched { nodes, .. } = p.revise { nodes } else { 0 };
                eprintln!(
                    "n={n:4} match={:3} threads={threads:2}: total {total:7.3} s (Java {java_secs:5.1} s) | prec {:.3} \
                     revise {:.3} ({revise_nodes} nodes) search {:.3} | nodes {} | sum {} | extra {:?}",
                    if match_bound { "on" } else { "off" },
                    t.preclusions.as_secs_f64(),
                    t.revise.as_secs_f64(),
                    r.time.as_secs_f64(),
                    r.stats.nodes,
                    r.sum,
                    r.stats.extra,
                );
            }
        }
    }

    /// One game (`TAXMAN_BENCH_N`, default 700; `TAXMAN_BENCH_THREADS`, default 1; tactics changed by
    /// `TAXMAN_BENCH_MATCH=off`, `TAXMAN_BENCH_NO_PRECLUSIONS`, and `TAXMAN_BENCH_LEGACY` for the
    /// legacy bound), with progress lines, for profiling.
    #[test]
    #[ignore]
    fn bench_search_one() {
        let n = std::env::var("TAXMAN_BENCH_N").ok().and_then(|s| s.parse().ok()).unwrap_or(700);
        let threads = std::env::var("TAXMAN_BENCH_THREADS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
        let legacy_bound = std::env::var_os("TAXMAN_BENCH_LEGACY").is_some();
        let match_bound = std::env::var("TAXMAN_BENCH_MATCH").map_or(true, |s| s == "on");
        let preclusions = std::env::var_os("TAXMAN_BENCH_NO_PRECLUSIONS").is_none();
        let tactics = SearchTactics { legacy_bound, match_bound, preclusions, ..SearchTactics::default() };
        let seconds = std::env::var("TAXMAN_BENCH_SECONDS").ok().and_then(|s| s.parse().ok());
        let tables = Tables::new(n);
        // The golden columns if there are any, else the frame builder's.
        let (columns, cands) = if golden_ns().contains(&n) {
            let g = load(n);
            (g.columns, Bits::<16>::from_slice(&g.candidates))
        } else {
            let frames = build_frames::<16>(&tables);
            (frames.frames.iter().flat_map(|f| [f.factors.to_vec(), f.moves.to_vec()]).collect(), frames.candidates)
        };
        let p = prepare::<16>(&tables, &columns, &cands, tactics, threads).unwrap();
        let cfg = EngineConfig {
            progress: Some(Duration::from_secs(10)),
            time_limit: seconds.map(Duration::from_secs),
            ..config(threads)
        };
        // What optimal.json says the best promotion sum is, and the `prev_cap` ceiling.
        if let Ok(optimal) = crate::game::Optimal::load_default() {
            let in_the_bag: u64 =
                crate::hives::Apiary::<16>::new(&tables, Bits::EMPTY).solution().iter().map(|&x| u64::from(x)).sum();
            let best = optimal.score(n).map(|s| s - in_the_bag);
            let cap = optimal.score(n - 1).map(|s| u64::from(n) + s - in_the_bag);
            eprintln!("n={n}: optimal promotion sum {best:?}, prev_cap {cap:?}, greedy {}", p.greedy.sum());
        }
        let r = search(&p, &cfg, None, None, false).unwrap();
        eprintln!(
            "n={n}: {:?} search {:.3} s (completed {}), nodes {}, best {}, max_total {}, extra {:?}",
            p.timings,
            r.time.as_secs_f64(),
            r.completed,
            r.stats.nodes,
            r.sum,
            p.max_total,
            r.stats.extra
        );
    }

    #[test]
    #[ignore]
    fn bench_search() {
        // Java whole-game wall times, one thread and ten (including ~0.5 s of JVM start and setup).
        let java = [
            (480, 2.3, 1.5),
            (500, 2.1, 1.4),
            (600, 2.7, 1.7),
            (640, 2.0, 1.6),
            (660, 4.1, 2.1),
            (700, 18.0, 4.8),
            (720, 23.2, 5.6),
            (760, 65.6, 12.4),
        ];
        for (n, one, ten) in java {
            let g = load(n);
            if n < 512 {
                bench_game::<8>(n, &g.columns, &Bits::from_slice(&g.candidates), (one, ten));
            } else {
                bench_game::<16>(n, &g.columns, &Bits::from_slice(&g.candidates), (one, ten));
            }
        }
        if std::env::var_os("TAXMAN_BENCH_1000").is_some() {
            let frames = build_frames::<16>(&Tables::new(1000));
            let columns: Vec<Vec<u32>> =
                frames.frames.iter().flat_map(|f| [f.factors.to_vec(), f.moves.to_vec()]).collect();
            bench_game::<16>(1000, &columns, &frames.candidates, (f64::NAN, f64::NAN));
        }
    }
}
