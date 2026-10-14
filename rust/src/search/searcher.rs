//! The Taxman searcher ([`TaxSearcher`]): what the engine runs on each work unit, a depth-first loop of steps on
//! a stack of nodes.
//!
//! The stack holds the path from the unit's node down to the node being worked on, plus one slot below it where
//! that node's children are made; slots are reused, so a step allocates nothing.  Next to the stack, and
//! mirroring it level by level, the searcher keeps the matching bound of every node on the path
//! ([`MatchBound`]: level `d` is the node at depth `d`, and a child is pushed on it while it is checked and for
//! as long as it is on the stack) and the conflicts known along the path (`known`: `stack[d].known_end` of them
//! hold for the node at depth `d`).
//!
//! A step ([`Stepper::step`]) is DESIGN.md's "The default step" in four phases (the module docs of
//! [`crate::search`] map them): [`check_node`](Stepper::check_node), [`choose_branch`](Stepper::choose_branch),
//! [`make_child`](Stepper::make_child) and [`check_child`](Stepper::check_child).  It reads the incumbent once:
//! both checks compare with that value, even after one of them has offered a better incumbent.

use super::diagnostics::{cut, Probe};
use super::lagrange::{Lagrangian, Refits};
use super::refine::{strongest, Learning, Refiner, Seek, Verdict};
use super::setup::Prepared;
use super::{Diagnostics, NOGOOD_PERIOD};
use crate::bits::Bits;
use crate::bounds::max_additional;
use crate::engine::{Ctx, Poll, Searcher, Stats, Unit};
use crate::gf2::{Gf2Bound, Gf2Graph, Gf2Stats};
use crate::matchbound::{MatchBound, MatchGraph};
use crate::model::{ConflictCache, FrameSet, Lookup, Model, Nogoods, Scratch};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------------------------------------
// Units

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

/// Appends the raw little-endian words of `set` (`8·W` bytes): how units, incumbents and digests encode sets.
pub(super) fn put_bits<const W: usize>(out: &mut Vec<u8>, set: &Bits<W>) {
    for word in set.as_words() {
        out.extend_from_slice(&word.to_le_bytes());
    }
}

/// The set encoded by [`put_bits`]; `bytes` must be exactly `8·W` long.
pub(super) fn get_bits<const W: usize>(bytes: &[u8]) -> Bits<W> {
    let mut words = [0u64; W];
    for (word, chunk) in words.iter_mut().zip(bytes.as_chunks::<8>().0) {
        *word = u64::from_le_bytes(*chunk);
    }
    Bits::from_words(words)
}

// ---------------------------------------------------------------------------------------------------------
// What the searchers share

/// What every searcher of one search shares, read-only.
pub struct SearchSpace<'a, const W: usize> {
    pub model: &'a Model,
    /// The starting state: the frames as built.
    pub root: &'a FrameSet<W>,
    /// The root's matching graph, which bounds every node.
    pub match_graph: &'a MatchGraph,
    /// How deep to refine the matching bound by conflicts (tactic `match_refine`).
    pub match_refine: u32,
    /// Tactics `fix_forced` and `nogoods`.
    pub fix_forced: bool,
    pub nogoods: bool,
    /// After how many of its steps a searcher forgets the nogoods it knows (tactic `forget_nogoods`: every
    /// [`NOGOOD_PERIOD`]), if ever.
    pub nogood_period: Option<u64>,
    /// With the Lagrangian bound over the nogoods (tactic `lagrange`, with `nogoods`), when each searcher refits
    /// its multipliers ([`Refits::SEARCH`]).
    pub lagrange: Option<Refits>,
    /// The root's candidates (where the Lagrangian bound's multipliers are fitted).
    pub candidates: Bits<W>,
    /// The largest number in the root's frames.
    pub root_max: u32,
    /// The most promotions a promotion set can have (`maxTotalPromotions`; see [`Prepared::max_total`]).
    pub max_total: u32,
    /// A proven ceiling on the promotion sum (`n + opt(n − 1) − in_the_bag`): an incumbent that reaches it ends
    /// the search.  (The ceiling, called the cap in the tactic `prev_cap`; not `max_total`, which limits the
    /// number of promotions.)
    pub cap: Option<u64>,
    /// Where to collect [`Diagnostics`], if anywhere.
    pub diagnostics: Option<&'a Mutex<Diagnostics>>,
    /// When the search began, and the best incumbent the searchers found (see [`Found`]).
    pub began: Instant,
    pub found: Mutex<Option<Found>>,
    /// With the tactic `gf2`: the root's GF(2) frame matroids, which bound every node.
    pub gf2: Option<Gf2Graph>,
    /// With `gf2`: whether the matching bound is kept along the path too, as a cheaper first check.
    pub gf2_prefilter: bool,
    /// With `gf2`: how a node chooses the member of its conflict to branch on.
    pub gf2_branching: Gf2Branching,
    /// With [`Gf2Branching::Exact`]: how many members of a conflict (the most promising by their split bounds) are
    /// solved exactly; the others are not tried.
    pub gf2_strong_limit: usize,
    /// With [`Gf2Branching::Exact`] and `fix_forced`: once a member is found forced, whether the others are still
    /// solved (as far as the need) to find more forced members.
    pub gf2_more_forced: bool,
}

/// How a node chooses the member of its conflict to branch on with the tactic `gf2` (strong branching asks what each
/// member's removal does to the bound).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gf2Branching {
    /// The largest member.
    Largest,
    /// The member whose removal lowers the weight split's bound most (see [`crate::gf2::meet`]).
    Split,
    /// The member whose removal lowers the bound most, solved exactly (warm-started).
    Exact,
    /// Each member solved only as far as `need` (to find the forced ones), the branch the member with the lowest
    /// weight-split bound.
    Forced,
}

impl<'a, const W: usize> SearchSpace<'a, W> {
    /// The space [`search`](super::search) searches `prepared` in: its tactics, with `graph` the root's matching
    /// graph (`MatchGraph::new(&prepared.model, &prepared.root)`), the ceiling `cap`, no diagnostics, and the
    /// searchers forgetting their nogoods as the tactic `forget_nogoods` says.
    pub fn new(prepared: &'a Prepared<W>, graph: &'a MatchGraph, cap: Option<u64>) -> Self {
        let tactics = prepared.tactics;
        SearchSpace {
            model: &prepared.model,
            root: &prepared.root,
            match_graph: graph,
            match_refine: u32::from(tactics.match_refine),
            fix_forced: tactics.fix_forced,
            nogoods: tactics.nogoods,
            nogood_period: tactics.forget_nogoods.then_some(NOGOOD_PERIOD),
            lagrange: (tactics.nogoods && tactics.lagrange).then_some(Refits::SEARCH),
            candidates: prepared.candidates,
            root_max: prepared.root.frames().iter().filter_map(|f| (f.moves | f.factors).max()).max().unwrap_or(0),
            max_total: prepared.max_total,
            cap,
            diagnostics: None,
            began: Instant::now(),
            found: Mutex::new(None),
            gf2: tactics.gf2.then(|| Gf2Graph::new(&prepared.model, &prepared.root, &prepared.candidates)),
            gf2_prefilter: false,
            gf2_branching: Gf2Branching::Exact,
            gf2_strong_limit: usize::MAX,
            gf2_more_forced: true,
        }
    }
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

/// Most nogoods a searcher keeps (tactic `nogoods`); it forgets them all when it has this many.  With
/// `forget_nogoods` a table never gets near it (a few hundred nogoods are learned in a period near n = 1200).
const NOGOOD_CAPACITY: usize = 1 << 16;

/// Indexes into [`Stats::extra`]: the searchers' counters.
pub mod stat {
    /// Promotions tried (each child a node's loop makes).
    pub const PROMOTE_ATTEMPTS: usize = 0;
    /// Candidates promoted in children that were made (several for a child that promotes forced members).
    pub const PROMOTIONS: usize = 1;
    /// Children pushed on the stack (explored).
    pub const CHILDREN: usize = 2;
    /// Matching bounds computed (checks of nodes and children).
    pub const MATCH_BOUNDS: usize = 3;
    /// With the tactic `gf2`: the GF(2) bound's solves and augmentations, and of those the ones of strong
    /// branching (the bound without a member; see [`Gf2Stats`]).  Not in the results lines.
    pub const GF2_SOLVES: usize = 4;
    pub const GF2_AUGMENTATIONS: usize = 5;
    pub const GF2_STRONG_SOLVES: usize = 6;
    pub const GF2_STRONG_AUGMENTATIONS: usize = 7;
}

// ---------------------------------------------------------------------------------------------------------
// The searcher

/// One node on the searcher's stack: a set of promotions, the state they reach, and the candidates its loop has
/// still to try.
#[derive(Clone, Copy)]
pub(super) struct Node<const W: usize> {
    pub(super) state: FrameSet<W>,
    pub(super) promoted: Bits<W>,
    /// The sum of `promoted`.
    pub(super) sum: u64,
    /// The candidates still to try.
    pub(super) cands: Bits<W>,
    /// The most promotions a completion of the node can add ([`max_additional`] of its candidates when it was
    /// pushed or rebuilt; the loop's candidates only shrink, so it stays valid).
    pub(super) max_add: u32,
    /// How many of the searcher's known conflicts hold for this node.
    pub(super) known_end: usize,
    /// What the matching check of the node said when it was made, until its loop's first step takes it.
    pub(super) checked: Option<Checked<W>>,
}

/// The verdict of a child's matching check that let it through, kept for the first step of its loop: at the
/// same `need` and with the same matching, the check would say the same.
#[derive(Clone, Copy)]
pub(super) struct Checked<const W: usize> {
    pub(super) need: u64,
    /// The conflict to branch on and the forced members (see [`Open`]).
    pub(super) conflict: Bits<W>,
    pub(super) forced: Bits<W>,
}

/// What the check of a node leaves for choosing the branch when its loop goes on.
#[derive(Clone, Copy)]
struct Open<const W: usize> {
    /// The conflict of the heaviest matching (a subset of the node's candidates), if one was looked for and the
    /// matching has one.
    conflict: Option<Bits<W>>,
    /// The node's forced members found so far (tactic `fix_forced`): every completion that reaches `need`
    /// promotes them.
    forced: Bits<W>,
}

impl<const W: usize> Open<W> {
    /// No conflict looked for, nothing forced: the verdict of a child's check without refinement.
    const NONE: Self = Open { conflict: None, forced: Bits::EMPTY };
}

/// The candidate a node branches on, and what its child promotes.
struct Branch<const W: usize> {
    /// Promoted in the child, and no longer a candidate of the node.
    n: u32,
    /// What the child promotes: `n`, or with `fix_forced` all of the node's forced members (`n` among them).
    added: Bits<W>,
    /// The node's candidates without `n`: what the child's candidates are chosen from.
    left: Bits<W>,
}

/// Where the depth-first search goes after a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Next {
    /// The node's loop has ended: on to the parent's next step (the unit is done if the node is the unit's).
    Parent,
    /// The node's next step.
    Same,
    /// A child was pushed: its first step.
    Child,
}

/// The searcher's counters, added to the worker's statistics (see [`stat`]) and diagnostics at each flush.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Counts {
    pub(super) attempts: u64,
    pub(super) promotions: u64,
    pub(super) children: u64,
    /// Matching checks of nodes and children.
    pub(super) match_bounds: u64,
    /// Refinements made (tactic `match_refine`), the matching evaluations they made, and the conflicts they
    /// had to compute (not known yet).
    pub(super) refinements: u64,
    pub(super) evaluations: u64,
    pub(super) conflicts: u64,
}

/// The Taxman searcher: explores units depth-first on a stack of nodes it reuses (no recursion, and no
/// allocation once the stack is as deep as the search goes).
pub struct TaxSearcher<'a, const W: usize> {
    /// The path from the unit's node down, `stack[..=depth]`, and the slot below it where the deepest node's
    /// children are made.
    stack: Vec<Node<W>>,
    stepper: Stepper<'a, W>,
}

/// Everything a step works with besides the stack.
struct Stepper<'a, const W: usize> {
    space: &'a SearchSpace<'a, W>,
    /// The scratch space of every peel the searcher makes.
    scratch: Scratch<W>,
    /// The matching bound along the path, level for level with the stack; kept only while `matching_on`.
    matching: MatchBound<'a, W>,
    /// Whether the matching bound is used: without the tactic `gf2`, or with it as a first check.
    matching_on: bool,
    /// The GF(2) bound along the path, level for level with the stack (tactic `gf2`).
    gf2: Option<Gf2Bound<'a, W>>,
    /// Scratch for strong branching with `gf2`: the members of a conflict by their split bounds, those tried in
    /// that order, and their bounds where the heaviest set's exchange graph certifies them.
    order: Vec<(i64, u32)>,
    tried: Vec<u32>,
    certified: Vec<Option<u64>>,
    /// Conflicts known for the node being worked on (see [`Refiner`]); `stack[d].known_end` of them hold for
    /// the node at depth `d`.
    known: Vec<Bits<W>>,
    /// The frames' last conflicts (see [`ConflictCache`]).
    conflict_cache: ConflictCache<W>,
    /// The nogoods learned (tactic `nogoods`), and the steps taken since the table was last cleared.
    nogoods: Option<Nogoods<W>>,
    steps_since_clear: u64,
    /// The Lagrangian bound over the nogoods along the path (tactic `lagrange`).
    lagrange: Option<Lagrangian<'a, W>>,
    counts: Counts,
    /// The incumbents offered, encoded.
    payload: Vec<u8>,
    probe: Option<Probe<'a>>,
    /// The GF(2) bound's counters as of the last flush.
    gf2_flushed: Gf2Stats,
}

impl<'a, const W: usize> TaxSearcher<'a, W> {
    pub fn new(space: &'a SearchSpace<'a, W>) -> Self {
        TaxSearcher {
            stack: Vec::new(),
            stepper: Stepper {
                space,
                scratch: Scratch::new(),
                matching: MatchBound::new(space.match_graph),
                matching_on: space.gf2.is_none() || space.gf2_prefilter,
                gf2: space.gf2.as_ref().map(Gf2Bound::new),
                order: Vec::new(),
                tried: Vec::new(),
                certified: Vec::new(),
                known: Vec::new(),
                conflict_cache: ConflictCache::default(),
                nogoods: space.nogoods.then(|| Nogoods::new(space.root_max, NOGOOD_CAPACITY)),
                steps_since_clear: 0,
                lagrange: space.lagrange.map(|refits| Lagrangian::new(space.match_graph, refits)),
                counts: Counts::default(),
                payload: Vec::with_capacity(8 * W),
                probe: space.diagnostics.map(Probe::new),
                gf2_flushed: Gf2Stats::default(),
            },
        }
    }

    /// Rebuilds the unit's node in slot 0.
    fn start(&mut self, unit: &TaxUnit<W>) {
        let space = self.stepper.space;
        let state = space
            .model
            .promote_all_using(&mut self.stepper.scratch, space.root, &unit.promoted)
            .unwrap_or_else(|| panic!("a unit's promotions {:?} cannot be promoted from the root", unit.promoted));
        let max_add = max_additional(space.max_total, &state, &unit.promoted, &unit.cands);
        let sum = unit.promoted.sum();
        let node =
            Node { state, promoted: unit.promoted, sum, cands: unit.cands, max_add, known_end: 0, checked: None };
        self.stepper.known.clear();
        if self.stack.is_empty() {
            self.stack.push(node);
        } else {
            self.stack[0] = node;
        }
        if self.stepper.matching_on {
            self.stepper.matching.reset(&unit.promoted, &unit.cands);
        }
        if let Some(g) = &mut self.stepper.gf2 {
            g.reset(&unit.promoted, &unit.cands);
        }
        if let Some(lagrange) = &mut self.stepper.lagrange {
            lagrange.start();
        }
    }

    /// One step of the loop of the node at `depth`.  Returns where the search goes next.
    fn step(&mut self, depth: usize, ctx: &mut Ctx<'_, TaxUnit<W>>) -> Next {
        if self.stack.len() == depth + 1 {
            let copy = self.stack[depth];
            self.stack.push(copy); // the slot for the children of a node this deep, made once
        }
        let (path, below) = self.stack.split_at_mut(depth + 1);
        self.stepper.step(&mut path[depth], &mut below[0], depth > 0, ctx)
    }
}

impl<const W: usize> Searcher<TaxUnit<W>> for TaxSearcher<'_, W> {
    fn run(&mut self, unit: TaxUnit<W>, ctx: &mut Ctx<'_, TaxUnit<W>>) {
        self.start(&unit);
        let mut depth = 0;
        loop {
            if ctx.poll() == Poll::Yield {
                // A node with no candidates left (a loop that `fix_forced` ended, say) has nothing for a unit to do:
                // its step would end the loop at once.
                for node in self.stack[..=depth].iter().filter(|node| !node.cands.is_empty()) {
                    ctx.push(TaxUnit { promoted: node.promoted, cands: node.cands });
                }
                break;
            }
            ctx.stats().nodes += 1; // the engine's "nodes" are steps
            let next = self.step(depth, ctx);
            self.stepper.forget_nogoods_when_due(ctx);
            match next {
                Next::Child => depth += 1,
                Next::Same => {}
                Next::Parent => match depth.checked_sub(1) {
                    Some(parent) => depth = parent,
                    None => break,
                },
            }
        }
        self.stepper.flush(ctx.stats());
    }
}

impl<'a, const W: usize> Stepper<'a, W> {
    /// One step of `node`'s loop (see the module docs of [`crate::search`]): whether the loop goes on, the
    /// candidate to branch on, the child (made in `child`), and whether the child is worth exploring.  `pushed`:
    /// `node` is not the unit's node, so its matching level was pushed for it and is popped when its loop ends.
    #[inline]
    fn step(&mut self, node: &mut Node<W>, child: &mut Node<W>, pushed: bool, ctx: &mut Ctx<'_, TaxUnit<W>>) -> Next {
        self.known.truncate(node.known_end); // forget the conflicts of the child last considered
        if self.matching_on {
            self.matching.take_kept(); // the node without the candidate it last branched on, if strong branching kept it
        }
        if let Some(p) = &mut self.probe {
            p.step(node.promoted.len());
        }
        let best = ctx.best();
        let need = (best + 1).saturating_sub(node.sum); // what a completion must add to beat the incumbent

        let open = match self.check_node(node, need, ctx) {
            Ok(open) => open,
            Err(reason) => return self.end_loop(reason, &node.promoted, pushed),
        };
        if let (Some(p), None) = (&mut self.probe, &self.gf2) {
            p.open(&mut self.scratch, &mut self.matching, self.space.model, node, need);
        }
        let Some(branch) = self.choose_branch(node, open, need) else {
            return self.end_loop(cut::FORCED, &node.promoted, pushed);
        };
        let Some(max_next) = self.make_child(node, child, &branch) else {
            if let Some(p) = &mut self.probe {
                p.failed(branch.added.len() == 1);
            }
            return Next::Same;
        };
        if let Err(reason) = self.check_child(child, &branch.added, max_next, best, ctx) {
            if let Some(p) = &mut self.probe {
                p.child_cut(reason);
            }
            return Next::Same;
        }
        self.offer(ctx, &child.promoted);
        if child.cands.is_empty() {
            if let Some(p) = &mut self.probe {
                p.child_cut(cut::EMPTY);
            }
            return Next::Same;
        }
        child.max_add = max_additional(self.space.max_total, &child.state, &child.promoted, &child.cands);
        child.known_end = self.known.len();
        self.counts.children += 1;
        if let Some(p) = &mut self.probe {
            p.child(child.promoted.len());
        }
        Next::Child
    }

    /// Phase 1: whether `node`'s loop goes on, that is whether some completion can still add `need`.
    /// `Err(reason)` if not, or if its best completion is known (then offered): the [`cut`] reason.  Else what
    /// the matching check found for choosing the branch.
    ///
    /// Cheapest first: candidates left; the sum of the `max_add` largest (no completion has more promotions, nor
    /// larger ones: a bound of its own); then the matching check, unless the node kept its verdict from when it
    /// was made at this `need`.
    #[inline]
    fn check_node(&mut self, node: &mut Node<W>, need: u64, ctx: &mut Ctx<'_, TaxUnit<W>>) -> Result<Open<W>, usize> {
        let checked = node.checked.take();
        if node.cands.is_empty() {
            return Err(cut::EMPTY);
        }
        if node.cands.sum_largest(node.max_add) < need {
            return Err(cut::LARGEST);
        }
        if let Some(checked) = checked.filter(|checked| checked.need == need) {
            // The node was checked when it was made, at the same need and with the same matching (this is its
            // first step): the verdict is the same, and so are the conflict and the forced members.
            return Ok(Open { conflict: Some(checked.conflict), forced: checked.forced });
        }
        let open = self.check(&node.state, &node.promoted, &node.cands, need, node.max_add, true, ctx);
        node.known_end = self.known.len(); // the conflicts the check found hold for the node and its subtree
        open
    }

    /// Phase 2: the candidate `node` branches on, taken out of its loop, and what the child promotes; `None` if
    /// every member of the conflict is forced (no completion reaches `need`, and the loop ends).
    ///
    /// With a conflict, the branch is one of its members, the one that looks best one step ahead (see
    /// `refine::strongest`): every completion leaves out one of them, so neither the child nor the node going on
    /// can find the same heaviest matching again.  With forced members (tactic `fix_forced`), the child promotes
    /// all of them and the node's loop ends after it: every completion of the node that reaches `need` promotes
    /// them all, and so is a completion of that child.  Without a conflict, the branch is the largest candidate.
    #[inline]
    fn choose_branch(&mut self, node: &mut Node<W>, open: Open<W>, need: u64) -> Option<Branch<W>> {
        let space = self.space;
        if self.gf2.is_some() {
            return self.choose_branch_gf2(node, open, need);
        }
        let conflict = open.conflict.map(|k| k & node.cands).filter(|k| !k.is_empty());
        let mut forced = open.forced & node.cands;
        let mut kept = false; // whether the matching level without n is kept for the node
        let n = match conflict {
            // With forced members known, the child promotes them all and the node's loop ends there.
            Some(_) if !forced.is_empty() => forced.max(),
            Some(k) if k.len() > 1 => {
                let strong = strongest(&mut self.matching, &k, need, space.fix_forced);
                (forced, kept) = (strong.forced, strong.kept);
                Some(strong.member)
            }
            Some(k) => k.max(),
            None => node.cands.max(),
        }
        .expect("not empty");
        if conflict.is_some_and(|k| k.is_subset(&forced)) {
            // Every completion that reaches `need` promotes the whole conflict, so there is none.
            return None;
        }
        node.cands.remove(n);
        let left = node.cands;
        if space.fix_forced && !forced.is_empty() {
            // Every completion that reaches `need` promotes the forced members, so all of them are the child's (or
            // none, if it fails): the node's loop has nothing left after it.
            node.cands = Bits::EMPTY;
        } else if !kept {
            self.matching.remove_candidate(n);
        }
        let added = if space.fix_forced && forced.len() > 1 { forced } else { Bits::EMPTY.with(n) };
        Some(Branch { n, added, left })
    }

    /// Phase 3: makes in `child` the child of `node` that promotes `branch.added`: its state, promotions and
    /// candidates.  Returns how many more promotions the child can make among `branch.left` ([`max_additional`],
    /// which the child's check uses), or `None` if the promotion fails.
    ///
    /// The child's candidates are the node's others, narrowed to the frames that still have freedoms; then a
    /// candidate is ruled out if promoting it would complete a conflict known for the node (or above it), or a
    /// nogood: the child's state is the node's with more promotions, so promoting the last member would promote
    /// the whole conflict there.  (The heaviest matching of the node promotes the branch already, so without this
    /// the child's matching bound would not drop.)
    #[inline]
    fn make_child(&mut self, node: &Node<W>, child: &mut Node<W>, branch: &Branch<W>) -> Option<u32> {
        let space = self.space;
        let model = space.model;
        self.counts.attempts += 1;
        child.checked = None;
        let added = branch.added;
        // Several forced members are promoted together, each frame checked once (see `Model::promote_all_using`);
        // one by one, each would take a freedom from its two frames, so together they fit exactly when no frame
        // they touch ends with negative freedoms.
        let promoted = if added.len() == 1 {
            child.state.assign(&node.state);
            model.promote_in_place_using(&mut self.scratch, &mut child.state, branch.n)
        } else {
            match model.promote_all_using(&mut self.scratch, &node.state, &added) {
                Some(state) => {
                    child.state.assign(&state);
                    added.iter().all(|x| {
                        let h = model.home(x);
                        let frames = child.state.frames();
                        frames[h].freedoms >= 0 && frames[h - 1].freedoms >= 0
                    })
                }
                None => false,
            }
        };
        if !promoted {
            return None;
        }
        self.counts.promotions += u64::from(added.len());
        child.promoted = node.promoted | added;
        child.sum = node.sum + added.sum();
        let rest = branch.left - added;
        let max_next = max_additional(space.max_total, &child.state, &child.promoted, &rest);
        let mut cands = child.state.narrow_candidates(&rest);
        // Usually the child promotes one number, and a membership test is cheaper than a set intersection.
        let touched = |k: &Bits<W>| if added.len() == 1 { k.contains(branch.n) } else { k.intersects(&added) };
        for k in self.known.iter().filter(|k| touched(k)) {
            let rest = *k - child.promoted;
            if rest.len() == 1 {
                cands -= rest;
            }
        }
        if let Some(table) = &self.nogoods {
            cands -= table.ruled_out(&added, &child.promoted, &cands);
        }
        child.cands = cands;
        Some(max_next)
    }

    /// Phase 4: whether the child made in `child` (which promoted `added` and can make `max_next` more
    /// promotions) is worth exploring at the incumbent `best` the step began with.  `Err(reason)` if no
    /// completion of it can beat the incumbent, or if its best completion is known (then offered).
    ///
    /// The checks are the node's (see [`check_node`](Self::check_node)), without asking for a conflict to
    /// branch on.  The child is pushed on the matching stack for its check and stays there if it passes, with
    /// its verdict kept for its first step.  A child with no candidates passes without a check: it has nothing
    /// to bound, and is offered but not explored.
    #[inline]
    fn check_child(
        &mut self,
        child: &mut Node<W>,
        added: &Bits<W>,
        max_next: u32,
        best: u64,
        ctx: &mut Ctx<'_, TaxUnit<W>>,
    ) -> Result<(), usize> {
        let need = (best + 1).saturating_sub(child.sum);
        let cands = child.cands;
        if cands.sum_largest(max_next) < need {
            return Err(cut::LARGEST);
        }
        if cands.is_empty() {
            return Ok(());
        }
        if self.matching_on {
            self.matching.push_all(added, &cands);
        }
        if let Some(g) = &mut self.gf2 {
            g.push_all(added, &cands);
        }
        match self.check(&child.state, &child.promoted, &cands, need, max_next, false, ctx) {
            Ok(open) => {
                child.checked = open.conflict.map(|conflict| Checked { need, conflict, forced: open.forced });
                if let (Some(p), None) = (&mut self.probe, &self.gf2) {
                    p.matching_open(
                        &mut self.scratch,
                        &mut self.matching,
                        self.space.model,
                        &child.state,
                        max_next,
                        need,
                    );
                }
                Ok(())
            }
            Err(reason) => {
                if self.matching_on {
                    self.matching.pop();
                }
                if let Some(g) = &mut self.gf2 {
                    g.pop();
                }
                if let Some(l) = &mut self.lagrange {
                    l.leave(&child.promoted);
                }
                Err(reason)
            }
        }
    }

    /// The matching check of the node on top of the matching stack, whose state is `state`, promotions `promoted`
    /// and candidates `cands`, at `need` (see [`Refiner`]).  `Err(reason)`: its matching bound is below `need`
    /// ([`cut::MATCH`]), or brought below it by refining or by forced members ([`cut::REFINED`]), or its heaviest
    /// matching can be promoted ([`cut::EXACT`], the best completion, offered), or with the tactic `lagrange` the
    /// Lagrangian bound is below `need` ([`cut::LAGRANGE`]).  Else the conflict to branch on (when
    /// `wants_conflict`, or found by refining) and the forced members (of the matching and of the Lagrangian
    /// bound).  A promotable completion that refining finds is offered too.
    #[allow(clippy::too_many_arguments)]
    fn check(
        &mut self,
        state: &FrameSet<W>,
        promoted: &Bits<W>,
        cands: &Bits<W>,
        need: u64,
        cap: u32,
        wants_conflict: bool,
        ctx: &mut Ctx<'_, TaxUnit<W>>,
    ) -> Result<Open<W>, usize> {
        if self.gf2.is_some() {
            return self.check_gf2(state, promoted, cands, need, cap, ctx);
        }
        let space = self.space;
        let m = &mut self.matching;
        self.counts.match_bounds += 1;
        if m.value() < need {
            return Err(cut::MATCH);
        }
        let depth = space.match_refine;
        if depth == 0 && !wants_conflict {
            // A child's check without refinement (`match_refine` 0) goes no further, the Lagrangian bound included:
            // the child's first step checks it again, looking for its conflict.
            return Ok(Open::NONE);
        }
        let promoted = *promoted;
        let learning = self.nogoods.as_mut().map(|table| Learning { table, root: space.root, promoted });
        let mut refiner = Refiner {
            m,
            model: space.model,
            scratch: &mut self.scratch,
            state,
            need,
            known: &mut self.known,
            cache: &mut self.conflict_cache,
            evaluations: 0,
            computed: 0,
            exact: Bits::EMPTY,
            learning,
            forcing: space.fix_forced,
            forced: Bits::EMPTY,
        };
        let verdict = refiner.top(depth, wants_conflict);
        let (forced, exact) = (refiner.forced, refiner.exact);
        self.counts.refinements += 1;
        self.counts.evaluations += refiner.evaluations;
        self.counts.conflicts += refiner.computed;
        if !exact.is_empty() {
            self.offer(ctx, &(promoted | exact));
        }
        match verdict {
            Verdict::Below => Err(cut::REFINED),
            Verdict::Exact(q) => {
                self.offer(ctx, &(promoted | q));
                Err(cut::EXACT)
            }
            Verdict::Open(conflict) => match &mut self.lagrange {
                None => Ok(Open { conflict, forced }),
                Some(l) => match l.check(&promoted, cands, need, space.fix_forced, &forced) {
                    None => Err(cut::LAGRANGE),
                    Some(more) => Ok(Open { conflict, forced: forced | more }),
                },
            },
        }
    }

    /// The check of [`check`](Self::check) with the tactic `gf2`, on the node on top of the GF(2) stack (and of
    /// the matching stack, as a first check, with [`SearchSpace::gf2_prefilter`]), at `need`, with at most `cap`
    /// promotions.  `Err(reason)`: the matching bound is below `need` ([`cut::MATCH`]), or the GF(2) bound is
    /// ([`cut::GF2`]), or its heaviest set can be promoted ([`cut::EXACT`], the best completion, offered), or
    /// every member of its conflict is forced, or a nogood is ([`cut::REFINED`]), or with the tactic `lagrange` the
    /// Lagrangian bound is below `need` ([`cut::LAGRANGE`]).  Else the conflict of the heaviest set (from the nogood
    /// table or computed, as [`Refiner`] finds the matching's) and, with `fix_forced`, the forced members: those of
    /// the heaviest set without which the weight split's bound is below `need`, and those of the Lagrangian bound.
    fn check_gf2(
        &mut self,
        state: &FrameSet<W>,
        promoted: &Bits<W>,
        cands: &Bits<W>,
        need: u64,
        cap: u32,
        ctx: &mut Ctx<'_, TaxUnit<W>>,
    ) -> Result<Open<W>, usize> {
        let space = self.space;
        self.counts.match_bounds += 1;
        if self.matching_on && self.matching.value() < need {
            return Err(cut::MATCH);
        }
        let g = self.gf2.as_mut().expect("the tactic gf2");
        let eval = g.evaluate(cap, space.fix_forced.then_some(need));
        if eval.value < need {
            return Err(cut::GF2);
        }
        let forced = eval.forced;
        let promoted = *promoted;
        let mut learning = self.nogoods.as_mut().map(|table| Learning { table, root: space.root, promoted });
        let mut seek = Seek {
            model: space.model,
            scratch: &mut self.scratch,
            cache: &mut self.conflict_cache,
            state,
            known: &mut self.known,
            learning: &mut learning,
        };
        let (found, computed) = seek.conflict(&eval.chosen, forced);
        self.counts.conflicts += u64::from(computed);
        let conflict = match found {
            Lookup::Conflict(k) => k,
            Lookup::None => {
                // The heaviest set can be promoted: the best completion.
                self.offer(ctx, &(promoted | eval.chosen));
                return Err(cut::EXACT);
            }
            // A nogood all of whose members are forced or promoted: no completion reaches `need`.
            Lookup::AllForced => return Err(cut::REFINED),
        };
        if conflict.is_subset(&forced) {
            return Err(cut::REFINED);
        }
        match &mut self.lagrange {
            None => Ok(Open { conflict: Some(conflict), forced }),
            Some(l) => match l.check(&promoted, cands, need, space.fix_forced, &forced) {
                None => Err(cut::LAGRANGE),
                Some(more) => Ok(Open { conflict: Some(conflict), forced: forced | more }),
            },
        }
    }

    /// [`choose_branch`](Self::choose_branch) with the tactic `gf2`: the forced members if there are any (the child
    /// promotes them all, with `fix_forced`), else a member of the conflict of the GF(2) bound's heaviest set chosen
    /// as [`SearchSpace::gf2_branching`] says.  With [`Gf2Branching::Exact`], each member's bound is solved without
    /// it, which also finds forced members among them.
    fn choose_branch_gf2(&mut self, node: &mut Node<W>, open: Open<W>, need: u64) -> Option<Branch<W>> {
        let space = self.space;
        let g = self.gf2.as_mut().expect("the tactic gf2");
        let conflict = open.conflict.map(|k| k & node.cands).filter(|k| !k.is_empty());
        let mut forced = open.forced & node.cands;
        let n = match conflict {
            Some(_) if !forced.is_empty() => forced.max(),
            Some(k) if k.len() > 1 => match space.gf2_branching {
                Gf2Branching::Largest => k.max(),
                Gf2Branching::Split => {
                    let mut best = (i64::MAX, 0);
                    for x in k.iter() {
                        let bound = g.split_without(x).unwrap_or(i64::MAX);
                        if bound <= best.0 {
                            best = (bound, x);
                        }
                    }
                    Some(best.1)
                }
                Gf2Branching::Exact | Gf2Branching::Forced => {
                    // The members by their split bounds, the likeliest to lower the bound most first.  The bound
                    // without each is read off the exchange graph of the heaviest set where that certifies it (no
                    // solve, before any solve spoils the workspace's tableau), and solved where not: each solve
                    // stops as soon as its value reaches both the best so far and `need` (then the member can be
                    // neither chosen nor forced), or `need` once a member is forced (with `fix_forced` only which
                    // are forced still matters).  The exact value of a certified bound leads to the same choices as
                    // a solve stopped at any `stop` would.
                    let order = &mut self.order;
                    order.clear();
                    order.extend(k.iter().map(|x| (g.split_without(x).unwrap_or(i64::MAX), u32::MAX - x)));
                    order.sort_unstable();
                    let tried = order.len().min(space.gf2_strong_limit.max(1));
                    self.tried.clear();
                    self.tried.extend(order[..tried].iter().map(|&(_, key)| u32::MAX - key));
                    g.certified_excluding(&self.tried, node.max_add, &mut self.certified);
                    let mut best = (u64::MAX, 0);
                    let to_need = space.gf2_branching == Gf2Branching::Forced;
                    if to_need {
                        best.1 = self.tried[0];
                    }
                    // Whether the level without the best member so far was kept from its solve.
                    let mut kept = false;
                    for (&x, &certified) in self.tried.iter().zip(&self.certified) {
                        if !space.gf2_more_forced && space.fix_forced && !forced.is_empty() {
                            break;
                        }
                        // A solve that stops has a value of at least `stop`, which is never below `need`: only a
                        // solve that ran to the end can show a member forced.
                        let stop =
                            if to_need || (space.fix_forced && !forced.is_empty()) { need } else { best.0.max(need) };
                        let bound = certified.unwrap_or_else(|| g.value_excluding(x, node.max_add, stop));
                        if bound < need {
                            forced.insert(x);
                        }
                        if bound < best.0 && !to_need {
                            kept = certified.is_none();
                            if kept {
                                g.keep_excluded();
                            }
                            best = (bound, x);
                        }
                    }
                    // Unless forced members end the node's loop, it goes on without the chosen member, from that
                    // member's level solved to the end (kept for it, the same level whichever way its bound was
                    // found).
                    if !to_need && !kept && !(space.fix_forced && !forced.is_empty()) {
                        g.value_excluding(best.1, node.max_add, u64::MAX);
                        g.keep_excluded();
                    }
                    if space.fix_forced { forced.max() } else { None }.or(Some(best.1))
                }
            },
            Some(k) => k.max(),
            None => node.cands.max(),
        }
        .expect("not empty");
        if conflict.is_some_and(|k| k.is_subset(&forced)) {
            // Every completion that reaches `need` promotes the whole conflict, so there is none.
            return None;
        }
        node.cands.remove(n);
        let left = node.cands;
        if space.fix_forced && !forced.is_empty() {
            // Every completion that reaches `need` promotes the forced members, so all of them are the child's.
            node.cands = Bits::EMPTY;
        } else {
            g.remove_candidate(n);
            if self.matching_on {
                self.matching.remove_candidate(n);
            }
        }
        let added = if space.fix_forced && forced.len() > 1 { forced } else { Bits::EMPTY.with(n) };
        Some(Branch { n, added, left })
    }

    /// Ends the loop of the node with the promotions `promoted` for `reason`: the search goes back to the parent,
    /// whose matching level is then on top again (`pushed`: the node had one of its own).
    #[inline]
    fn end_loop(&mut self, reason: usize, promoted: &Bits<W>, pushed: bool) -> Next {
        if let Some(p) = &mut self.probe {
            p.loop_end(reason);
        }
        if pushed {
            if self.matching_on {
                self.matching.pop();
            }
            if let Some(g) = &mut self.gf2 {
                g.pop();
            }
        }
        if let Some(l) = &mut self.lagrange {
            l.leave(promoted);
        }
        Next::Parent
    }

    /// Offers `set` as the incumbent if it beats it, recording when, and ends the search if it reaches the cap.
    fn offer(&mut self, ctx: &mut Ctx<'_, TaxUnit<W>>, set: &Bits<W>) {
        let value = set.sum();
        if value <= ctx.best() {
            return;
        }
        let space = self.space;
        self.payload.clear();
        put_bits(&mut self.payload, set);
        if ctx.offer(value, &self.payload) {
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

    /// With the tactic `forget_nogoods`, forgets the nogoods every [`SearchSpace::nogood_period`] steps; with the
    /// tactic `lagrange`, refits the Lagrangian bound's multipliers when due, first.
    #[inline]
    fn forget_nogoods_when_due(&mut self, ctx: &Ctx<'_, TaxUnit<W>>) {
        if let (Some(l), Some(table)) = (&mut self.lagrange, &self.nogoods) {
            l.step(table, &self.space.candidates, ctx.best());
        }
        if let (Some(table), Some(period)) = (&mut self.nogoods, self.space.nogood_period) {
            self.steps_since_clear += 1;
            if self.steps_since_clear >= period {
                table.clear();
                self.steps_since_clear = 0;
            }
        }
    }

    /// Adds this searcher's counts to the worker's statistics (and diagnostics) and resets them.
    fn flush(&mut self, stats: &mut Stats) {
        let counts = std::mem::take(&mut self.counts);
        let extra = [
            (stat::PROMOTE_ATTEMPTS, counts.attempts),
            (stat::PROMOTIONS, counts.promotions),
            (stat::CHILDREN, counts.children),
            (stat::MATCH_BOUNDS, counts.match_bounds),
        ];
        for (i, count) in extra {
            stats.extra[i] += count;
        }
        if let Some(g) = &self.gf2 {
            let (now, before) = (g.stats(), self.gf2_flushed);
            stats.extra[stat::GF2_SOLVES] += now.solves - before.solves;
            stats.extra[stat::GF2_AUGMENTATIONS] += now.augmentations - before.augmentations;
            stats.extra[stat::GF2_STRONG_SOLVES] += now.excluded_solves - before.excluded_solves;
            stats.extra[stat::GF2_STRONG_AUGMENTATIONS] += now.excluded_augmentations - before.excluded_augmentations;
            self.gf2_flushed = now;
        }
        if let Some(p) = &mut self.probe {
            p.flush(&counts, &self.matching);
        }
    }
}
