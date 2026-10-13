//! The matching check of a node ([`Refiner`]): its matching bound refined by conflicts, its forced members, the
//! conflict to branch on and the exact completions; and strong branching ([`strongest`]).
//!
//! Both work on a [`MatchBound`] whose top level is the node: its candidates `C`, and a heaviest matching that
//! promotes `Q* ⊆ C` with the bound `value = sum(Q*)` (see [`crate::matchbound`]).  `need` is how much a completion
//! of the node must add to beat the incumbent.

use crate::bits::Bits;
use crate::matchbound::MatchBound;
use crate::model::{ConflictCache, FrameSet, Lookup, Model, Nogoods, Scratch};

/// The matching bound of a node refined by conflicts (tactic `match_refine`), on top of a [`MatchBound`] whose
/// top level is the node, with `state` and the candidates `C` of that level.
///
/// If `Q*` can really be promoted from `state`, it is the best completion and the bound is exact.  Otherwise some
/// subset `K ⊆ Q*` cannot be promoted, a *conflict* ([`Model::conflict_using`]), and every completion `F ⊆ C` that
/// can be promoted leaves out some member of `K` (a subset of a promotable set is promotable).  With `K`'s
/// members `k₁ < k₂ < … < k_r`, every such `F` is in exactly one of the branches "`k₁ … k_{j−1}` in `F`, `k_j`
/// not", and the best completion is at most the largest of the branches' matching bounds: the matching with
/// `k_j` no longer a candidate and `k₁ … k_{j−1}` required (as if promoted), plus their sum.  Each branch is
/// refined in the same way, one level less deep, with the members required so far counted in `Q*` (so a
/// conflict among them alone leaves the branch without completions).  By induction on the depth, the refined
/// bound is never below the best completion; `max_add` is not needed: the bound only grows without it.
///
/// A conflict of a state is a conflict of every state below it (they have more promotions, which can only make
/// promoting `K` harder) and of every candidate set containing it, so the conflicts found are kept in `known`
/// for the rest of the node's loop and its subtree: one that `Q*` contains needs no peeling.
///
/// The branches are tried from the smallest member, whose exclusion costs the matching least, so that a bound
/// that stays at least `need` is usually seen at once.
///
/// With `forcing` (tactic `fix_forced`), the check also finds the node's *forced members*: the members `x` of
/// `Q*` whose removal alone brings the bound below `need`, by [`MatchBound::bound_excluding`] (an upper bound on
/// the matching bound without `x`, from the potentials).  Every completion that reaches `need` promotes all of
/// them, so a conflict all of whose members are forced leaves the node with no such completion.
pub(super) struct Refiner<'r, 'g, const W: usize> {
    pub(super) m: &'r mut MatchBound<'g, W>,
    pub(super) model: &'r Model,
    pub(super) scratch: &'r mut Scratch<W>,
    pub(super) state: &'r FrameSet<W>,
    pub(super) need: u64,
    /// Conflicts of `state` (or of states above it).
    pub(super) known: &'r mut Vec<Bits<W>>,
    pub(super) cache: &'r mut ConflictCache<W>,
    /// Matching evaluations made.
    pub(super) evaluations: u64,
    /// Conflicts computed (not found in `known` or the nogood table).
    pub(super) computed: u64,
    /// The best completion found that can be promoted (the candidates of an exact branch), or none.
    pub(super) exact: Bits<W>,
    /// The nogoods (tactic `nogoods`).
    pub(super) learning: Option<Learning<'r, W>>,
    /// Whether to find the heaviest matching's forced members (tactic `fix_forced`), and those found.
    pub(super) forcing: bool,
    pub(super) forced: Bits<W>,
}

/// What a [`Refiner`] needs to use and grow the searcher's nogood table (tactic `nogoods`).
pub(super) struct Learning<'r, const W: usize> {
    pub(super) table: &'r mut Nogoods<W>,
    /// The search's root, and the promotions of the node.
    pub(super) root: &'r FrameSet<W>,
    pub(super) promoted: Bits<W>,
}

/// What [`Refiner::top`] found out about the top level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Verdict<const W: usize> {
    /// The (refined) bound is below `need`: no completion reaches it.
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
    pub(super) fn top(&mut self, depth: u32, conflict: bool) -> Verdict<W> {
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
            Lookup::AllForced => {
                // With nothing forced, a nogood all of whose members are forced lies within the node's own
                // promotions: a broken table (as in `top_conflict`), which must not pass for a cut.
                debug_assert!(!self.forced.is_empty(), "a nogood lies within the promotions of a node");
                return Verdict::Below;
            }
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
            Lookup::None => None,
            // With nothing forced this would be a nogood inside the node's own promotions, which were promoted to
            // build the node: a broken table, which must not pass for a promotable set.
            Lookup::AllForced => unreachable!("a nogood lies within the promotions of a node"),
        }
    }

    /// [`top_conflict`](Self::top_conflict), or [`Lookup::AllForced`] if the table knows a conflict all of whose
    /// members are in `forced`.
    fn find_conflict(&mut self, promoted: &Bits<W>, forced: Bits<W>) -> Lookup<W> {
        if let Some(l) = &self.learning {
            match l.table.lookup(&l.promoted, promoted, &forced) {
                Lookup::None => {}
                found => {
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
                l.table.insert(n);
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
    ///
    /// The first branch with nothing required is the level without the smallest member, which strong branching
    /// solves first too: it is kept for it ([`MatchBound::keep_solved`]).
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
                if required.is_empty() {
                    self.m.keep_solved(x);
                } else {
                    self.m.pop();
                }
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

/// What strong branching chose ([`strongest`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Strong<const W: usize> {
    /// The member of the conflict to branch on.
    pub(super) member: u32,
    /// The members of the conflict whose removal brings the matching bound below `need`: the node's forced
    /// members.  `member` is among them unless there are none.
    pub(super) forced: Bits<W>,
    /// Whether the matching level without `member` is kept for the node (see [`MatchBound::keep_excluded`]).
    pub(super) kept: bool,
}

/// The member of the conflict `k` to branch on by looking ahead (tactic `branching = conflict-strong`): the one
/// whose removal from the candidates lowers the matching bound most (then the largest), which makes the node
/// going on without it the likeliest to stop soon.  (The child that promotes it gains nothing to look at: the
/// heaviest matching already promotes every member.)  `m`'s top level is the node, settled; it is left so, with
/// the level without the chosen member kept for it (see [`MatchBound::keep_excluded`]).
///
/// Each member's bound is solved exactly ([`MatchBound::value_excluding`]), so this also finds the members whose
/// removal brings the bound below `need`, the node's *forced* members: every completion that reaches `need`
/// promotes them all.
///
/// With `fix_forced` (that tactic promotes all the forced members, so that which of them is chosen hardly
/// matters), when some member is forced the largest forced one is chosen and no level is kept (the node's loop
/// has nothing left to do after that child).
pub(super) fn strongest<const W: usize>(
    m: &mut MatchBound<'_, W>,
    k: &Bits<W>,
    need: u64,
    fix_forced: bool,
) -> Strong<W> {
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
            Strong { member: n, forced, kept: false }
        }
        _ => Strong { member: best.1, forced, kept },
    }
}
