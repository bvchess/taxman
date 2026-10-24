//! The Taxman searcher ([`TaxSearcher`]): what the engine runs on each work unit, a depth-first loop of steps on
//! a stack of nodes.
//!
//! The stack holds the path from the unit's node down to the node being worked on, plus one slot below it where
//! that node's children are made; slots are reused, so a step allocates nothing.  Next to the stack, and
//! mirroring it level by level, the searcher keeps the bound of every node on the path (the GF(2) bound with the
//! tactic `gf2`, the default; the matching bound without it: [`MatchBound`]; level `d` is the node at depth `d`, and
//! a child is pushed on it while it is checked and for as long as it is on the stack) and the conflicts known along
//! the path (`known`: `stack[d].known_end` of them hold for the node at depth `d`).
//!
//! A step ([`Stepper::step`]) is solver/docs/design/search.md's "The default step" in four phases (the module docs of
//! [`crate::search`] map them): [`check_node`](Stepper::check_node), [`choose_branch`](Stepper::choose_branch),
//! [`make_child`](Stepper::make_child) and [`check_child`](Stepper::check_child).  It reads the incumbent once:
//! both checks compare with that value, even after one of them has offered a better incumbent.
//!
//! A study's hook (`StepHook`) is built only with the feature `studies`: every site of it is compiled out of a
//! default build, whose steps have none.

use super::branching::{BranchSettings, Branching};
use super::diagnostics::{cut, Probe};
#[cfg(feature = "studies")]
use super::hooks::{BoundCheck, MakeHook, StepEnd, StepHook, StepRecord};
use super::lagrange::{Lagrangian, Refits};
use super::near_repeat::NearRepeat;
use super::price_memo::{PriceMemo, Reuse};
use super::profile::{Branched, Made, Phase, Phases, SearchProfile, Tags};
use super::refine::{strongest, Learning, Refiner, Seek, Verdict};
use super::setup::Prepared;
use super::symmetry::{RuleScratch, Symmetries};
use super::tactics::Gf2Branching;
use super::{Diagnostics, Forgetting, CONFLICT_BRANCHING_SLOPE, CONFLICT_LOOK_PRICE, CONFLICT_LOOK_SLACK};
use crate::bits::Bits;
use crate::bounds::max_additional;
use crate::engine::{Ctx, Poll, Searcher, Stats, Unit};
use crate::gf2::{Eval, ForceWindow, Gf2Bound, Gf2Graph, Gf2Stats};
use crate::matchbound::{MatchBound, MatchGraph};
use crate::model::{ConflictCache, FrameGraph, FrameSet, Lookup, NogoodStats, Nogoods, Scratch};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
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
    pub model: &'a FrameGraph,
    /// The starting state: the frames as built.
    pub root: &'a FrameSet<W>,
    /// The root's matching graph, which bounds every node.
    pub match_graph: &'a MatchGraph,
    /// How deep to refine the matching bound by conflicts (tactic `match_refine`).
    pub match_refine: u32,
    /// Tactics `fix_forced` and `nogoods`.
    pub fix_forced: bool,
    pub nogoods: bool,
    /// When a searcher forgets the nogoods it knows, what it keeps, and how many it holds at most (tactics
    /// `forget_nogoods`, `nogood_period`, `nogood_keep` and `nogood_limit`; sound whatever they say).
    pub forgetting: Forgetting,
    /// What the searchers' tables have done, as each publishes it (at the end of a unit, and every
    /// [`PUBLISH_EVERY`] steps): the counters of the policy.
    pub nogood_totals: Arc<Mutex<NogoodStats>>,
    /// With the Lagrangian bound over the nogoods (tactic `lagrange`, with `nogoods`), when each searcher refits
    /// its multipliers ([`Refits::SEARCH`]).
    pub lagrange: Option<Refits>,
    /// The root's candidates (where the Lagrangian bound's multipliers are fitted).
    pub candidates: Bits<W>,
    /// The largest number in the root's frames and candidates (the one-sided candidates of the extended problem are
    /// in no frame): what the nogoods hold.
    pub root_max: u32,
    /// The most promotions a promotion set can have (see [`Prepared::max_total`]).
    pub max_total: u32,
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
    /// With `gf2`: a node with at least this many promotions chooses its branch as `gf2_deep_branching` says, not
    /// as `gf2_branching` does (tactics `strong_depth` and `deep_branching`).
    pub gf2_strong_depth: u32,
    pub gf2_deep_branching: Gf2Branching,
    /// With `gf2`: which free members are searched for being forced (tactic `force_window`).
    pub gf2_force_window: ForceWindow,
    /// The branching settings, which a caller may change while the search runs; each searcher reads them when it
    /// starts a unit ([`Branching`]).  With `gf2`, their `conflict_branching` and `gate` give the gap from which a
    /// check branches on the conflict of the heaviest set whose members cost the bound most to leave out (tactic
    /// `conflict_branching`; see [`conflict_to_branch_on`]); with the tactic off, the search's own conflict everywhere.
    pub branching: Arc<Branching>,
    /// With the tactic `symmetry`: the root's symmetric pairs, whose inequalities the search keeps (`None` without
    /// the tactic, or when the root has no pair).
    pub symmetry: Option<Symmetries>,
    /// With the tactic `near_repeat`: where the search's near-repeat cut is put once it has one ([`NearRepeat`], the
    /// game proven before whose problem cuts the nodes), at its setup or while it runs (a lock is set once, so the cut
    /// is armed at most once); `None` for a search without the cut.  Each searcher picks the cut up when it starts a
    /// unit, as it reads the branching settings, and cuts by it only where the incumbent its step read reaches the
    /// cut's bound ([`NearRepeat::reached`]).
    pub near_repeat: Option<&'a OnceLock<NearRepeat<W>>>,
    /// With the tactic `price_reuse`, when the choice of the conflict takes the prices it made for a node of the
    /// searcher's path at an earlier check ([`Reuse`]); `None`: it makes them at every check.
    pub price_reuse: Option<Reuse>,
    /// The search's profile, if it keeps one ([`SearchProfile`]): each searcher counts its steps by node, and tags the
    /// nodes it makes near the root.  It observes the search and changes nothing of it.
    pub profile: Option<&'a SearchProfile>,
    /// A study's hook, which each searcher makes for itself (see [`StepHook`]; `None` in the solver's searches, and
    /// absent from a build without the feature `studies`).
    #[cfg(feature = "studies")]
    pub hooks: Option<&'a MakeHook<W>>,
}

impl<'a, const W: usize> SearchSpace<'a, W> {
    /// The space [`search`](super::search) searches `prepared` in: its tactics, with `graph` the root's matching
    /// graph (`MatchGraph::new(&prepared.model, &prepared.root)`), no diagnostics, and the searchers forgetting
    /// their nogoods as the tactic `forget_nogoods` says.
    pub fn new(prepared: &'a Prepared<W>, graph: &'a MatchGraph) -> Self {
        let tactics = prepared.tactics;
        SearchSpace {
            model: &prepared.model,
            root: &prepared.root,
            match_graph: graph,
            match_refine: u32::from(tactics.match_refine),
            fix_forced: tactics.fix_forced,
            nogoods: tactics.nogoods,
            forgetting: tactics.forgetting(),
            nogood_totals: Arc::default(),
            lagrange: (tactics.nogoods && tactics.lagrange).then_some(Refits::SEARCH),
            candidates: prepared.candidates,
            root_max: prepared
                .root
                .frames()
                .iter()
                .filter_map(|f| (f.moves | f.factors).max())
                .chain(prepared.candidates.max())
                .max()
                .unwrap_or(0),
            max_total: prepared.max_total,
            diagnostics: None,
            began: Instant::now(),
            found: Mutex::new(None),
            gf2: tactics.gf2.then(|| Gf2Graph::new(&prepared.model, &prepared.root, &prepared.candidates)),
            gf2_prefilter: false,
            gf2_branching: tactics.branching,
            gf2_strong_limit: tactics.strong_limit as usize,
            gf2_more_forced: tactics.more_forced,
            gf2_strong_depth: tactics.strong_depth,
            gf2_deep_branching: tactics.deep_branching,
            gf2_force_window: tactics.force_window,
            branching: Arc::new(Branching::new(BranchSettings::of(&tactics))),
            symmetry: tactics
                .symmetry
                .then(|| Symmetries::of(&prepared.model, &prepared.root, &prepared.candidates))
                .filter(|pairs| !pairs.is_empty()),
            near_repeat: None,
            price_reuse: tactics.price_reuse.then_some(Reuse::STANDARD),
            profile: None,
            #[cfg(feature = "studies")]
            hooks: None,
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

/// A searcher publishes what its nogood table has done to [`SearchSpace::nogood_totals`] after each this many of its
/// steps (and when a unit ends), so that a progress line shows them.
pub const PUBLISH_EVERY: u64 = 1 << 15;

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
    /// What the bound check of the node said when it was made, until its loop's first step takes it.
    pub(super) checked: Option<Checked<W>>,
    /// With the near-repeat cut: whether `promoted` is a promotion set of Π_M, the problem of the game proven before
    /// ([`NearRepeat`]), if that has been asked, and if so its state there, σ_M(P), in `near_state`.
    pub(super) near: Near,
    pub(super) near_state: FrameSet<W>,
}

/// What a node knows of its promotions in Π_M, the problem of the near-repeat cut's game ([`NearRepeat`]): found only
/// where the cut's checks get as far as asking (check 2), so that a search where they do not pays nothing for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Near {
    /// Not asked yet.
    Unknown,
    /// The promotions are no promotion set of Π_M (a member is no candidate of Π_M, or they cannot be promoted there),
    /// nor is any superset of them: the cut never holds at the node or below it.
    Off,
    /// They are, and the node's `near_state` is σ_M(P).
    On,
}

/// The verdict of a child's bound check that let it through, kept for the first step of its loop: at the
/// same `need` and with the same heaviest set, the check would say the same.
#[derive(Clone, Copy)]
pub(super) struct Checked<const W: usize> {
    pub(super) need: u64,
    /// The conflict to branch on and the forced members (see [`Open`]).
    pub(super) conflict: Bits<W>,
    pub(super) forced: Bits<W>,
    pub(super) branched: Branched,
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
    /// Whether conflict branching chose the conflict (for the profile).
    branched: Branched,
}

impl<const W: usize> Open<W> {
    /// No conflict looked for, nothing forced: the verdict of a child's check without refinement.
    const NONE: Self = Open { conflict: None, forced: Bits::EMPTY, branched: Branched::Default };

    /// The conflict to branch on, chosen as the search does without conflict branching, and the forced members.
    fn of(conflict: Option<Bits<W>>, forced: Bits<W>) -> Self {
        Open { conflict, forced, branched: Branched::Default }
    }
}

/// The candidate a node branches on, and what its child promotes.
struct Branch<const W: usize> {
    /// Promoted in the child, and no longer a candidate of the node.
    n: u32,
    /// What the child promotes: `n`, or with `fix_forced` all of the node's forced members (`n` among them).
    added: Bits<W>,
    /// The node's candidates without `n`: what the child's candidates are chosen from.
    left: Bits<W>,
    /// The child promotes the node's forced members, and the node's loop ends with it (tactic `fix_forced`).
    forced: bool,
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
    /// Bound checks of nodes and children (the GF(2) bound, or the matching bound without `gf2`).
    pub(super) match_bounds: u64,
    /// Refinements made (tactic `match_refine`), the matching evaluations they made, and the conflicts they
    /// had to compute (not known yet).
    pub(super) refinements: u64,
    pub(super) evaluations: u64,
    pub(super) conflicts: u64,
    /// Candidates the rule of the tactic `symmetry` dropped from the candidates of children and of nodes.
    pub(super) symmetry_excluded: u64,
    /// Nodes and children the near-repeat cut cut ([`NearRepeat`]).
    pub(super) near_repeat: u64,
}

/// The Taxman searcher: explores units depth-first on a stack of nodes it reuses (no recursion, and no
/// allocation once the stack is as deep as the search goes).
pub struct TaxSearcher<'a, const W: usize> {
    /// The path from the unit's node down, `stack[..=depth]`, and the slot below it where the deepest node's
    /// children are made.
    stack: Vec<Node<W>>,
    stepper: Stepper<'a, W>,
    /// With a profile, where the nodes on the stack stand in it, and the steps counted ([`Tags`]).
    tags: Option<Tags<'a>>,
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
    /// With the tactic `price_reuse`, the prices of the choice of the conflict kept for the nodes of the path, and the
    /// schedule of their reuse ([`SearchSpace::price_reuse`]).
    price_memo: Option<(PriceMemo<W>, Reuse)>,
    /// The nogoods learned (tactic `nogoods`), and the steps taken since the table was last cleared.
    nogoods: Option<Nogoods<W>>,
    steps_since_clear: u64,
    /// The Lagrangian bound over the nogoods along the path (tactic `lagrange`).
    lagrange: Option<Lagrangian<'a, W>>,
    /// The sums of the rule of the tactic `symmetry` (empty without it).
    symmetry_scratch: RuleScratch,
    counts: Counts,
    /// The incumbents offered, encoded.
    payload: Vec<u8>,
    /// The branching settings as the searcher last read them, at the start of a unit, and the generation it read them
    /// at ([`SearchSpace::branching`]): the gap from which `conflict_branching` chooses (`None`: it is off), and
    /// `conflict_look`.
    conflict_branching: Option<u64>,
    conflict_look: bool,
    branching_seen: u64,
    /// The near-repeat cut, once the searcher has picked it up at the start of a unit ([`SearchSpace::near_repeat`]).
    near_repeat: Option<&'a NearRepeat<W>>,
    probe: Option<Probe<'a>>,
    /// The GF(2) bound's counters as of the last flush.
    gf2_flushed: Gf2Stats,
    /// The bound of the last check (the GF(2) bound, or the matching bound on the matching path), and what the last
    /// step that pushed a child knew of the child: for the profile, which records them, and nothing else.
    bound: u64,
    made: Made,
    /// With a profile, the clock of the phases of one step in [`PHASE_EVERY`](super::profile::PHASE_EVERY).
    phases: Option<Phases>,
    /// A study's hook, and the record of the step in progress that it is told of ([`SearchSpace::hooks`]).
    #[cfg(feature = "studies")]
    hook: Option<(Box<dyn StepHook<W>>, StepRecord<W>)>,
    /// Whether the conflict of the last GF(2) check is its heaviest set, which can be promoted: a counting search
    /// ([`StepHook::counting`]) branches on it.
    #[cfg(feature = "studies")]
    promotable: bool,
    /// The sets a study's hook proposed to learn ([`StepHook::propose`]), reused.
    #[cfg(feature = "studies")]
    proposals: Vec<Bits<W>>,
}

impl<'a, const W: usize> TaxSearcher<'a, W> {
    pub fn new(space: &'a SearchSpace<'a, W>) -> Self {
        // The generation first: a change after it is read again at the first unit.
        let branching_seen = space.branching.generation();
        let branching = space.branching.settings();
        TaxSearcher {
            stack: Vec::new(),
            stepper: Stepper {
                space,
                scratch: Scratch::new(),
                matching: MatchBound::new(space.match_graph),
                matching_on: space.gf2.is_none() || space.gf2_prefilter,
                gf2: space.gf2.as_ref().map(|graph| {
                    let mut bound = Gf2Bound::new(graph);
                    bound.set_force_window(space.gf2_force_window);
                    bound
                }),
                order: Vec::new(),
                tried: Vec::new(),
                certified: Vec::new(),
                known: Vec::new(),
                conflict_cache: ConflictCache::default(),
                price_memo: space.price_reuse.map(|reuse| (PriceMemo::default(), reuse)),
                nogoods: space.nogoods.then(|| {
                    let mut table = Nogoods::new(space.root_max, NOGOOD_CAPACITY);
                    table.set_limit(space.forgetting.limit);
                    table
                }),
                steps_since_clear: 0,
                lagrange: space.lagrange.map(|refits| Lagrangian::new(space.match_graph, refits)),
                symmetry_scratch: space.symmetry.as_ref().map(Symmetries::scratch).unwrap_or_default(),
                counts: Counts::default(),
                payload: Vec::with_capacity(8 * W),
                branching_seen,
                conflict_branching: branching.conflict_gap(),
                conflict_look: branching.conflict_look,
                // Picked up when the searcher starts a unit (`start`), before any step.
                near_repeat: None,
                probe: space.diagnostics.map(Probe::new),
                gf2_flushed: Gf2Stats::default(),
                bound: 0,
                made: Made::default(),
                phases: space.profile.map(|_| Phases::new()),
                #[cfg(feature = "studies")]
                hook: space.hooks.map(|make| (make(), StepRecord::default())),
                #[cfg(feature = "studies")]
                promotable: false,
                #[cfg(feature = "studies")]
                proposals: Vec::new(),
            },
            tags: space.profile.map(Tags::new),
        }
    }

    /// The nogoods the searcher holds now (tactic `nogoods`; none without it), in the order it learned them, for a
    /// caller that wants what a search learned (the table is a cache: a searcher forgets as the space's
    /// [`Forgetting`] says, and a new searcher starts with none).
    pub fn nogoods(&self) -> &[Bits<W>] {
        self.stepper.nogoods.as_ref().map_or(&[], Nogoods::sets)
    }

    /// Rebuilds the unit's node in slot 0.
    fn start(&mut self, unit: &TaxUnit<W>) {
        let space = self.stepper.space;
        let state = space
            .model
            .promote_all_using(&mut self.stepper.scratch, space.root, &unit.promoted)
            .unwrap_or_else(|| panic!("a unit's promotions {:?} cannot be promoted from the root", unit.promoted));
        let max_add = max_additional(space.model, space.max_total, &state, &unit.promoted, &unit.cands);
        let sum = unit.promoted.sum();
        let node = Node {
            state,
            promoted: unit.promoted,
            sum,
            cands: unit.cands,
            max_add,
            known_end: 0,
            checked: None,
            near: Near::Unknown,
            near_state: state,
        };
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
        // The branching settings, as they are when the unit starts: changed since the searcher last read them, they are
        // read again.
        let generation = space.branching.generation();
        if generation != self.stepper.branching_seen {
            let settings = space.branching.settings();
            self.stepper.conflict_branching = settings.conflict_gap();
            self.stepper.conflict_look = settings.conflict_look;
            self.stepper.branching_seen = generation;
        }
        // The near-repeat cut, if it was armed since the searcher last looked: a lock's state is one atomic load.  The
        // unit's node finds its state in Π_M from Π_M's root, and its children from it, as any unit's does; the nodes
        // searched before are only not cut.
        if self.stepper.near_repeat.is_none() {
            self.stepper.near_repeat = space.near_repeat.and_then(OnceLock::get);
        }
    }

    /// One step of the loop of the node at `depth`.  Returns where the search goes next.
    fn step(&mut self, depth: usize, ctx: &mut Ctx<'_, TaxUnit<W>>) -> Next {
        if self.stack.len() == depth + 1 {
            let copy = self.stack[depth];
            self.stack.push(copy); // the slot for the children of a node this deep, made once
        }
        let (path, below) = self.stack.split_at_mut(depth + 1);
        let (above, node) = path.split_at_mut(depth);
        let next = self.stepper.step(&mut node[0], above.last(), &mut below[0], ctx);
        #[cfg(feature = "studies")]
        if let Some((hook, record)) = &mut self.stepper.hook {
            let made = !matches!(record.end, StepEnd::LoopEnd(_) | StepEnd::ChildFailed);
            hook.step(record, &path[depth].state, made.then_some(&below[0].state));
        }
        next
    }
}

impl<const W: usize> Searcher<TaxUnit<W>> for TaxSearcher<'_, W> {
    fn run(&mut self, unit: TaxUnit<W>, ctx: &mut Ctx<'_, TaxUnit<W>>) {
        self.start(&unit);
        if let Some(tags) = &mut self.tags {
            tags.start(&unit.promoted);
        }
        let mut depth = 0;
        loop {
            if ctx.poll() == Poll::Yield {
                // A node with no candidates left (a loop that `fix_forced` ended, say) has nothing for a unit to do:
                // its step would end the loop at once.
                for (level, node) in self.stack[..=depth].iter().enumerate() {
                    if node.cands.is_empty() {
                        continue;
                    }
                    // The profile places the unit where the node stands in it (`SearchProfile::locate`).
                    debug_assert!(
                        self.tags.as_ref().is_none_or(|tags| tags.placed(level, &node.promoted)),
                        "the profile places the unit of {:?} elsewhere than its node",
                        node.promoted
                    );
                    ctx.push(TaxUnit { promoted: node.promoted, cands: node.cands });
                }
                break;
            }
            ctx.stats().nodes += 1; // the engine's "nodes" are steps
            if let Some(tags) = &mut self.tags {
                tags.step(depth);
            }
            if let Some(phases) = &mut self.stepper.phases {
                phases.begin();
            }
            let next = self.step(depth, ctx);
            self.stepper.forget_nogoods_when_due(ctx);
            if let Some(phases) = &mut self.stepper.phases {
                phases.end();
            }
            match next {
                Next::Child => {
                    if let Some(tags) = &mut self.tags {
                        let (node, child) = (&self.stack[depth], &self.stack[depth + 1]);
                        let cands = child.cands.len();
                        tags.child(depth, &node.promoted, &child.promoted, child.sum, cands, &self.stepper.made);
                    }
                    depth += 1;
                }
                Next::Same => {}
                Next::Parent => match depth.checked_sub(1) {
                    Some(parent) => depth = parent,
                    None => break,
                },
            }
        }
        self.stepper.flush(ctx.stats());
        if let Some(tags) = &mut self.tags {
            tags.flush(self.stepper.phases.as_mut());
        }
    }
}

impl<'a, const W: usize> Stepper<'a, W> {
    /// One step of `node`'s loop (see the module docs of [`crate::search`]): whether the loop goes on, the
    /// candidate to branch on, the child (made in `child`), and whether the child is worth exploring.  `parent`: the
    /// node's parent on the stack, if `node` is not the unit's node; then its matching level was pushed for it and is
    /// popped when its loop ends.
    #[inline]
    fn step(
        &mut self,
        node: &mut Node<W>,
        parent: Option<&Node<W>>,
        child: &mut Node<W>,
        ctx: &mut Ctx<'_, TaxUnit<W>>,
    ) -> Next {
        let pushed = parent.is_some();
        self.known.truncate(node.known_end); // forget the conflicts of the child last considered
        if self.matching_on {
            // The node without the candidate it last branched on, if strong branching kept it.
            self.matching.take_kept();
        }
        if let Some(p) = &mut self.probe {
            p.step(node.promoted.len());
        }
        let best = ctx.best();
        let need = (best + 1).saturating_sub(node.sum); // what a completion must add to beat the incumbent
        #[cfg(feature = "studies")]
        if let Some((_, record)) = &mut self.hook {
            *record = StepRecord { promoted: node.promoted, cands: node.cands, need, ..StepRecord::default() };
        }

        let open = match self.check_node(node, parent, best, need, ctx) {
            Ok(open) => open,
            Err(reason) => return self.end_loop(reason, &node.promoted, pushed),
        };
        let branched = open.branched;
        if let (Some(p), None) = (&mut self.probe, &self.gf2) {
            p.open(&mut self.scratch, &mut self.matching, self.space.model, node, need);
        }
        let branch = self.choose_branch(node, open, need);
        self.mark(Phase::Branching);
        let Some(branch) = branch else {
            return self.end_loop(cut::FORCED, &node.promoted, pushed);
        };
        let made = self.make_child(node, child, &branch);
        self.mark(Phase::Children);
        let Some(max_next) = made else {
            if let Some(p) = &mut self.probe {
                p.failed(branch.added.len() == 1);
            }
            #[cfg(feature = "studies")]
            self.tell(StepEnd::ChildFailed);
            return Next::Same;
        };
        if let Err(reason) = self.check_child(child, node, &branch.added, max_next, best, ctx) {
            if let Some(p) = &mut self.probe {
                p.child_cut(reason);
            }
            #[cfg(feature = "studies")]
            self.tell(StepEnd::ChildCut(reason));
            return Next::Same;
        }
        self.offer(ctx, &child.promoted);
        if child.cands.is_empty() {
            if let Some(p) = &mut self.probe {
                p.child_cut(cut::EMPTY);
            }
            #[cfg(feature = "studies")]
            self.tell(StepEnd::Leaf);
            return Next::Same;
        }
        child.max_add =
            max_additional(self.space.model, self.space.max_total, &child.state, &child.promoted, &child.cands);
        child.known_end = self.known.len();
        self.counts.children += 1;
        let branched = if branch.forced { Branched::Forced } else { branched };
        self.made = Made { branched, bound: self.bound, need: (best + 1).saturating_sub(child.sum) };
        if let Some(p) = &mut self.probe {
            p.child(child.promoted.len());
        }
        Next::Child
    }

    /// Phase 1: whether `node`'s loop goes on, that is whether some completion can still add `need`.
    /// `Err(reason)` if not, or if its best completion is known (then offered): the [`cut`] reason.  Else what
    /// the bound check found for choosing the branch.
    ///
    /// Cheapest first: candidates left; the sum of the `max_add` largest (no completion has more promotions, nor
    /// larger ones: a bound of its own); then, unless the node kept its verdict from when it was made at this
    /// `need`, the rule of the tactic `symmetry` (a function of the node's promotions and candidates alone, and
    /// cheaper than the bounds), then the near-repeat cut ([`near_repeat_cuts`](Self::near_repeat_cuts); `parent`: the
    /// node's parent on the stack, if it has one; `best`: the incumbent the step read, of which `need` is made), then
    /// the bound check (the GF(2) bound; the matching bound without `gf2`).
    #[inline]
    fn check_node(
        &mut self,
        node: &mut Node<W>,
        parent: Option<&Node<W>>,
        best: u64,
        need: u64,
        ctx: &mut Ctx<'_, TaxUnit<W>>,
    ) -> Result<Open<W>, usize> {
        let checked = node.checked.take();
        if node.cands.is_empty() {
            return Err(cut::EMPTY);
        }
        if node.cands.sum_largest(node.max_add) < need {
            return Err(cut::LARGEST);
        }
        if let Some(checked) = checked.filter(|checked| checked.need == need) {
            // The node was checked when it was made, at the same need and with the same matching (this is its
            // first step): the verdict is the same, and so are the conflict and the forced members.  So is the
            // rule of `symmetry`, which `make_child` applied to the same promotions and candidates.
            #[cfg(feature = "studies")]
            if let Some((_, record)) = &mut self.hook {
                record.inherited = true;
            }
            return Ok(Open { conflict: Some(checked.conflict), forced: checked.forced, branched: checked.branched });
        }
        let space = self.space;
        if let Some(sym) = &space.symmetry {
            self.apply_symmetry(sym, node, need)?;
        }
        #[cfg(feature = "studies")]
        self.apply_hook_rule(node, need)?;
        if self.near_repeat_cuts(node, parent, best) {
            return Err(cut::NEAR_REPEAT);
        }
        self.mark(Phase::Other);
        let open = self.check(&node.state, &node.promoted, &node.cands, need, node.max_add, true, ctx);
        node.known_end = self.known.len(); // the conflicts the check found hold for the node and its subtree
        open
    }

    /// The rule of the tactic `symmetry` at `node`, for the root's symmetric pairs `sym` ([`Symmetries::exclude`]):
    /// `Err` with [`cut::SYMMETRY`] if no completion keeps every pair's inequality, else the candidates that no
    /// completion keeping them holds are dropped from the node's loop and from the bounds' levels, and the cheap
    /// checks of [`check_node`](Self::check_node) are made again for what is left.
    ///
    /// The rule has work to do at a node that has been through a loop step: the node goes on without the member
    /// it branched on, which can lower the cofactor sum of a larger prime of a pair that the candidates can still
    /// reach.  A node's first step finds the rule's fixed point already (`make_child` reached it), unless the
    /// node is a unit, rebuilt from its promotions and candidates alone.
    ///
    /// The removal from the bounds is the one the node makes of the member it branches on
    /// ([`choose_branch`](Self::choose_branch)), and is safe here for the same reasons: the node's levels are on top
    /// of both stacks (the child of the last step was popped, or never pushed), the matching bound has taken the
    /// level kept for the node at the start of the step (so it holds none for the node as it was before the
    /// removal), and the GF(2) bound keeps the level before the removal only for a child that promotes `x`, which
    /// no child of this node does, `x` being no candidate of it now.  A node cut as hopeless is left as it is: its
    /// loop ends, and its levels are popped or, for a unit, reset.
    fn apply_symmetry(&mut self, sym: &Symmetries, node: &mut Node<W>, need: u64) -> Result<(), usize> {
        let before = node.cands;
        if sym.exclude(&mut self.symmetry_scratch, &node.promoted, &mut node.cands) {
            return Err(cut::SYMMETRY);
        }
        let dropped = before - node.cands;
        if dropped.is_empty() {
            return Ok(());
        }
        self.counts.symmetry_excluded += u64::from(dropped.len());
        self.drop_candidates(node, &dropped, need)
    }

    /// Takes `dropped`, the candidates a rule has taken out of `node`'s loop, out of the bounds' levels too (safe for
    /// the reasons [`apply_symmetry`](Self::apply_symmetry) gives), and makes the cheap checks of
    /// [`check_node`](Self::check_node) again for what is left.
    #[inline]
    fn drop_candidates(&mut self, node: &Node<W>, dropped: &Bits<W>, need: u64) -> Result<(), usize> {
        for x in dropped.iter() {
            if let Some(g) = &mut self.gf2 {
                g.remove_candidate(x);
            }
            if self.matching_on {
                self.matching.remove_candidate(x);
            }
        }
        if node.cands.is_empty() {
            return Err(cut::EMPTY);
        }
        if node.cands.sum_largest(node.max_add) < need {
            return Err(cut::LARGEST);
        }
        Ok(())
    }

    /// The rule of a study's hook at `node` ([`StepHook::narrow`]), as [`apply_symmetry`](Self::apply_symmetry)
    /// applies the rule of symmetric pairs: `Err` with [`cut::SYMMETRY`] if the hook says no completion keeps its
    /// rule, else the candidates it drops leave the node's loop and its bounds.
    #[cfg(feature = "studies")]
    fn apply_hook_rule(&mut self, node: &mut Node<W>, need: u64) -> Result<(), usize> {
        let Some((hook, _)) = &mut self.hook else { return Ok(()) };
        let before = node.cands;
        if hook.narrow(&node.promoted, &mut node.cands) {
            return Err(cut::SYMMETRY);
        }
        assert!(node.cands.is_subset(&before), "a hook's rule added candidates");
        let dropped = before - node.cands;
        if dropped.is_empty() {
            return Ok(());
        }
        self.drop_candidates(node, &dropped, need)
    }

    /// Learns what a study's hook proposes ([`StepHook::propose`]) when the check of a node (its state `state`,
    /// promotions `promoted` and candidates `cands`) has learned the conflict `k` of its heaviest set `chosen`.  Of
    /// each set proposed, a minimal conflict that a peel finds within it ([`FrameGraph::conflict_using`]) is learned as
    /// `k` was ([`Seek::conflict`]): known for the node and its subtree, and as a nogood, unless the table already
    /// knows a conflict within it.  A set that can be promoted has no conflict, and nothing is learned of it: whatever
    /// the hook proposes, only conflicts of the node are learned, so the search stays exact.
    #[cfg(feature = "studies")]
    fn learn_proposed(
        &mut self,
        state: &FrameSet<W>,
        promoted: &Bits<W>,
        cands: &Bits<W>,
        chosen: &Bits<W>,
        k: &Bits<W>,
    ) {
        let Some((hook, _)) = &mut self.hook else { return };
        let mut proposals = std::mem::take(&mut self.proposals);
        proposals.clear();
        hook.propose(state, promoted, cands, chosen, k, &mut proposals);
        let space = self.space;
        for set in &proposals {
            let Some(c) = space.model.conflict_using(&mut self.scratch, state, set, true) else { continue };
            let nogood = match &mut self.nogoods {
                Some(table) => {
                    if table.conflict(promoted, &c).is_some() {
                        continue;
                    }
                    let nogood = space.model.nogood(&mut self.scratch, space.root, promoted, &c);
                    if let Some(n) = nogood {
                        table.insert(n);
                    }
                    nogood
                }
                None => None,
            };
            self.known.push(c);
            hook.learned(set, &c, nogood.as_ref());
        }
        self.proposals = proposals;
    }

    /// The near-repeat cut at `node` ([`NearRepeat`], tactic `near_repeat`): whether `best`, the incumbent the step
    /// read, reaches the cut's bound, and the cut's three checks hold, so that no completion of the node is worth more
    /// than that bound ([`SearchSpace::near_repeat`]).  First the incumbent, one comparison, so that a search armed
    /// below the bound does nothing for the cut until the incumbent reaches it; then check 1 for the candidates, a few
    /// set operations; then σ_M(P), found once for the node ([`near_state`](Self::near_state)) and kept in it for its
    /// later steps and its children; then check 3, on the frames that its candidates touch.  `parent`: the node's
    /// parent on the stack, if it has one.
    #[inline]
    fn near_repeat_cuts(&mut self, node: &mut Node<W>, parent: Option<&Node<W>>, best: u64) -> bool {
        let Some(near) = self.near_repeat else { return false };
        if !near.reached(best) || node.near == Near::Off || !near.candidates_marked(&node.cands) {
            return false;
        }
        if node.near == Near::Unknown && !self.near_state(near, node, parent) {
            return false;
        }
        let cut = near.frames_agree(self.space.model.graph(), &node.state, &node.near_state, &node.cands);
        self.counts.near_repeat += u64::from(cut);
        cut
    }

    /// Checks 1 (for the promotions) and 2 of the near-repeat cut at `node`, whose state in Π_M is not known yet:
    /// whether its promotions are marked and are a promotion set of Π_M, recorded in the node with σ_M(P).  A parent's
    /// verdict is its children's too where it is `Off` (a superset of an unmarked set is unmarked, and of a set that
    /// cannot be promoted cannot be); where it is `On`, the child's state is the parent's with the node's own
    /// promotions promoted ([`NearRepeat::promote`]), and otherwise it is promoted from the root of Π_M
    /// ([`NearRepeat::state_of`]).
    fn near_state(&mut self, near: &NearRepeat<W>, node: &mut Node<W>, parent: Option<&Node<W>>) -> bool {
        let on = match parent.map(|p| p.near) {
            Some(Near::Off) => false,
            _ if !near.marked(&node.promoted) => false,
            Some(Near::On) => {
                let parent = parent.expect("a parent");
                let added = node.promoted - parent.promoted;
                near.promote(&mut self.scratch, &parent.near_state, &added, &mut node.near_state)
            }
            _ => match near.state_of(&mut self.scratch, &node.promoted) {
                Some(state) => {
                    node.near_state.assign(&state);
                    true
                }
                None => false,
            },
        };
        node.near = if on { Near::On } else { Near::Off };
        on
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
        Some(Branch { n, added, left, forced: space.fix_forced && !forced.is_empty() })
    }

    /// Phase 3: makes in `child` the child of `node` that promotes `branch.added`: its state, promotions and
    /// candidates.  Returns how many more promotions the child can make among `branch.left` ([`max_additional`],
    /// which the child's check uses), or `None` if the promotion fails.
    ///
    /// The child's candidates are the node's others, narrowed to the frames that still have freedoms; then a
    /// candidate is ruled out if promoting it would complete a conflict known for the node (or above it), or a
    /// nogood: the child's state is the node's with more promotions, so promoting the last member would promote
    /// the whole conflict there.  (The heaviest matching of the node promotes the branch already, so without this
    /// the child's matching bound would not drop.)  With the tactic `symmetry`, the candidates that no completion
    /// keeping the symmetric pairs' inequalities holds go too ([`Symmetries::exclude`]).
    #[inline]
    fn make_child(&mut self, node: &Node<W>, child: &mut Node<W>, branch: &Branch<W>) -> Option<u32> {
        let space = self.space;
        let model = space.model;
        self.counts.attempts += 1;
        child.checked = None;
        child.near = Near::Unknown;
        let added = branch.added;
        // Several forced members are promoted together, each frame checked once (see `FrameGraph::promote_all_using`);
        // one by one, each would take a freedom from the frames it touches (two, or its own frame for a one-sided
        // candidate), so together they fit exactly when no frame they touch ends with negative freedoms.
        let promoted = if added.len() == 1 {
            child.state.assign(&node.state);
            model.promote_in_place_using(&mut self.scratch, &mut child.state, branch.n)
        } else {
            match model.promote_all_using(&mut self.scratch, &node.state, &added) {
                Some(state) => {
                    child.state.assign(&state);
                    added.iter().all(|x| {
                        let (into, out) = model.frames_touched(x);
                        let frames = child.state.frames();
                        frames[into].freedoms >= 0 && out.is_none_or(|out| frames[out].freedoms >= 0)
                    })
                }
                None => false,
            }
        };
        self.mark(Phase::Promotion);
        if !promoted {
            return None;
        }
        self.counts.promotions += u64::from(added.len());
        child.promoted = node.promoted | added;
        child.sum = node.sum + added.sum();
        let rest = branch.left - added;
        let max_next = max_additional(model, space.max_total, &child.state, &child.promoted, &rest);
        let mut cands = model.narrow(&child.state, &rest);
        // Usually the child promotes one number, and a membership test is cheaper than a set intersection.
        let touched = |k: &Bits<W>| if added.len() == 1 { k.contains(branch.n) } else { k.intersects(&added) };
        for k in self.known.iter().filter(|k| touched(k)) {
            let rest = *k - child.promoted;
            if rest.len() == 1 {
                cands -= rest;
            }
        }
        if let Some(table) = &mut self.nogoods {
            cands -= table.ruled_out(&added, &child.promoted, &cands);
        }
        if let Some(sym) = &space.symmetry {
            // The candidates the rule drops are in no completion of the child that keeps the symmetric pairs'
            // inequalities, which are the only ones the search needs (the prime-exchange theorem, R2).  Whether the
            // child has any such completion is `check_child`'s to ask, of these candidates; the count leaves out a
            // child it cuts.
            let before = cands;
            if !sym.exclude(&mut self.symmetry_scratch, &child.promoted, &mut cands) {
                self.counts.symmetry_excluded += u64::from((before - cands).len());
            }
        }
        child.cands = cands;
        Some(max_next)
    }

    /// Phase 4: whether the child made in `child` of `node` (which promoted `added` and can make `max_next` more
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
        node: &Node<W>,
        added: &Bits<W>,
        max_next: u32,
        best: u64,
        ctx: &mut Ctx<'_, TaxUnit<W>>,
    ) -> Result<(), usize> {
        let need = (best + 1).saturating_sub(child.sum);
        // A study's rule narrows the child's candidates before anything is pushed for it ([`StepHook::narrow`]).
        #[cfg(feature = "studies")]
        if let Some((hook, _)) = &mut self.hook {
            let before = child.cands;
            if hook.narrow(&child.promoted, &mut child.cands) {
                return Err(cut::SYMMETRY);
            }
            assert!(child.cands.is_subset(&before), "a hook's rule added candidates");
        }
        let cands = child.cands;
        if cands.sum_largest(max_next) < need {
            return Err(cut::LARGEST);
        }
        if cands.is_empty() {
            return Ok(());
        }
        if let Some(sym) = &self.space.symmetry {
            // No completion of the child keeps every pair's inequality, and a heaviest promotion set keeps them
            // (R2 and the exchange-pruning lemma, R3): none of its completions beats the incumbent.
            if sym.hopeless(&mut self.symmetry_scratch, &child.promoted, &cands) {
                return Err(cut::SYMMETRY);
            }
        }
        if self.near_repeat_cuts(child, Some(node), best) {
            return Err(cut::NEAR_REPEAT);
        }
        self.mark(Phase::Other);
        if self.matching_on {
            self.matching.push_all(added, &cands);
        }
        if let Some(g) = &mut self.gf2 {
            g.push_all(added, &cands);
        }
        match self.check(&child.state, &child.promoted, &cands, need, max_next, false, ctx) {
            Ok(open) => {
                child.checked = open.conflict.map(|conflict| Checked {
                    need,
                    conflict,
                    forced: open.forced,
                    branched: open.branched,
                });
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
        self.bound = m.value();
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
        self.mark(Phase::Bound);
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
                None => Ok(Open::of(conflict, forced)),
                Some(l) => {
                    let checked = l.check(&promoted, cands, need, space.fix_forced, &forced);
                    self.mark(Phase::Lagrange);
                    match checked {
                        None => Err(cut::LAGRANGE),
                        Some(more) => Ok(Open::of(conflict, forced | more)),
                    }
                }
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
        #[cfg(feature = "studies")]
        {
            self.promotable = false;
        }
        if self.matching_on && self.matching.value() < need {
            return Err(cut::MATCH);
        }
        let g = self.gf2.as_mut().expect("the tactic gf2");
        let eval = g.evaluate(cap, space.fix_forced.then_some(need));
        self.bound = eval.value;
        if let Some(phases) = &mut self.phases {
            phases.mark(Phase::Bound);
        }
        if eval.value < need {
            #[cfg(feature = "studies")]
            self.tell_check(&eval, None);
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
        if let Some(phases) = &mut self.phases {
            phases.mark(Phase::Conflict);
        }
        // Tactic `conflict_branching`: where the gap is large, the conflict within the heaviest set whose children's
        // gaps fall most.  Any conflict keeps the search exact.  With the tactic `conflict_look`, it first looks at the
        // last child of the search's own conflict.
        let mut branched = Branched::Default;
        let found = match (self.conflict_branching, found) {
            (Some(gap), Lookup::Conflict(k0)) if eval.value >= need + gap => {
                branched = Branched::Conflict;
                let look = self.conflict_look.then_some(Look {
                    cache: &mut self.conflict_cache,
                    promoted,
                    cands: *cands,
                    need,
                    max_total: space.max_total,
                });
                Lookup::Conflict(conflict_to_branch_on(
                    g,
                    space.model,
                    &mut self.scratch,
                    state,
                    &eval,
                    k0,
                    cap,
                    learning.as_mut(),
                    &mut self.known,
                    look,
                    promoted.sum(),
                    self.price_memo.as_mut().map(|(memo, reuse)| (memo, &*reuse)),
                ))
            }
            (_, found) => found,
        };
        if let Some(phases) = &mut self.phases {
            phases.mark(Phase::Pricing);
        }
        #[cfg(feature = "studies")]
        if let (true, Lookup::Conflict(k)) = (computed, &found) {
            self.learn_proposed(state, &promoted, cands, &eval.chosen, k);
        }
        #[cfg(feature = "studies")]
        if self.hook.is_some() {
            let k = match &found {
                Lookup::Conflict(k) => Some((*k, computed)),
                _ => None,
            };
            self.tell_check(&eval, k);
        }
        let conflict = match found {
            Lookup::Conflict(k) => k,
            Lookup::None => {
                // A counting search goes on, branching on the members of the heaviest set (`StepHook::counting`),
                // without the Lagrangian bound's check (leaving out a check that only cuts keeps the count).
                #[cfg(feature = "studies")]
                if self.counting() && !eval.chosen.is_empty() {
                    self.promotable = true;
                    return Ok(Open::of(Some(eval.chosen), forced));
                }
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
        let forced = match &mut self.lagrange {
            None => forced,
            Some(l) => {
                let checked = l.check(&promoted, cands, need, space.fix_forced, &forced);
                self.mark(Phase::Lagrange);
                match checked {
                    None => return Err(cut::LAGRANGE),
                    Some(more) => forced | more,
                }
            }
        };
        Ok(Open { conflict: Some(conflict), forced, branched })
    }

    /// [`choose_branch`](Self::choose_branch) with the tactic `gf2`: the forced members if there are any (the child
    /// promotes them all, with `fix_forced`), else a member of the conflict of the GF(2) bound's heaviest set chosen
    /// as [`SearchSpace::gf2_branching`] says (at a node of at least [`SearchSpace::gf2_strong_depth`] promotions, as
    /// [`SearchSpace::gf2_deep_branching`] says).  With [`Gf2Branching::Exact`], each member's bound without it is
    /// found exactly (certified or solved), which also finds forced members among them.
    fn choose_branch_gf2(&mut self, node: &mut Node<W>, open: Open<W>, need: u64) -> Option<Branch<W>> {
        let space = self.space;
        let g = self.gf2.as_mut().expect("the tactic gf2");
        let conflict = open.conflict.map(|k| k & node.cands).filter(|k| !k.is_empty());
        let mut forced = open.forced & node.cands;
        // Below the depth of strong branching (tactic `strong_depth`), the choice of `deep_branching`.
        let deep = node.promoted.len() >= space.gf2_strong_depth;
        let branching = if deep { space.gf2_deep_branching } else { space.gf2_branching };
        let n = match conflict {
            Some(_) if !forced.is_empty() => forced.max(),
            Some(k) if k.len() > 1 => match branching {
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
                    let to_need = branching == Gf2Branching::Forced;
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
                    // member's level, kept for it: solved, or certified.
                    if !to_need && !kept && !(space.fix_forced && !forced.is_empty()) {
                        g.keep_certified(best.1);
                    }
                    // A study's hook may branch on another member (any member keeps the search exact; the level kept
                    // for this one is then not used: `Gf2Bound::remove_candidate` takes it only for this member).
                    #[cfg(not(feature = "studies"))]
                    let chosen = best.1;
                    #[cfg(feature = "studies")]
                    let chosen = match &mut self.hook {
                        Some((hook, _)) if forced.is_empty() && !to_need => {
                            let x = hook.branch(&k, best.1);
                            assert!(k.contains(x), "a hook chose {x}, which is not in the conflict {k:?}");
                            x
                        }
                        _ => best.1,
                    };
                    if space.fix_forced { forced.max() } else { None }.or(Some(chosen))
                }
            },
            Some(k) => k.max(),
            None => node.cands.max(),
        }
        .expect("not empty");
        if conflict.is_some_and(|k| k.is_subset(&forced)) {
            // Every completion that reaches `need` promotes the whole conflict, so there is none.
            #[cfg(not(feature = "studies"))]
            return None;
            // Unless a counting search's "conflict" is its heaviest set, which can be promoted: then that set is the
            // one such completion, and the child that promotes the forced members makes it.
            #[cfg(feature = "studies")]
            if !self.promotable {
                return None;
            }
        }
        #[cfg(feature = "studies")]
        if let Some((hook, _)) = &mut self.hook {
            if !(space.fix_forced && !forced.is_empty()) {
                hook.excluding(&node.state, &node.promoted, &node.cands, conflict.as_ref(), n);
            }
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
        #[cfg(feature = "studies")]
        if let Some((_, record)) = &mut self.hook {
            (record.branch, record.added, record.forced) = (Some(n), added, forced);
        }
        Some(Branch { n, added, left, forced: space.fix_forced && !forced.is_empty() })
    }

    /// With a profile, the phase `phase` of a step it times ends here ([`Phases`]).
    #[inline]
    fn mark(&mut self, phase: Phase) {
        if let Some(phases) = &mut self.phases {
            phases.mark(phase);
        }
    }

    /// Ends the loop of the node with the promotions `promoted` for `reason`: the search goes back to the parent,
    /// whose matching level is then on top again (`pushed`: the node had one of its own).
    #[inline]
    fn end_loop(&mut self, reason: usize, promoted: &Bits<W>, pushed: bool) -> Next {
        if let Some(p) = &mut self.probe {
            p.loop_end(reason);
        }
        #[cfg(feature = "studies")]
        self.tell(StepEnd::LoopEnd(reason));
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

    /// Whether a study's hook makes the search count ([`StepHook::counting`]).
    #[cfg(feature = "studies")]
    fn counting(&self) -> bool {
        self.hook.as_ref().is_some_and(|(hook, _)| hook.counting())
    }

    /// Records how the step ended, for the hook, if there is one.
    #[cfg(feature = "studies")]
    #[inline]
    fn tell(&mut self, end: StepEnd) {
        if let Some((_, record)) = &mut self.hook {
            record.end = end;
        }
    }

    /// Records a GF(2) bound check, for the hook if there is one: the node's own, or once the node has chosen its
    /// branch the child's.  `conflict` is the conflict of its heaviest set, if the check got as far as one.
    #[cfg(feature = "studies")]
    #[inline]
    fn tell_check(&mut self, eval: &Eval<W>, conflict: Option<(Bits<W>, bool)>) {
        if let Some((_, record)) = &mut self.hook {
            let check = BoundCheck {
                value: eval.value,
                chosen: eval.chosen,
                forced: eval.forced,
                conflict: conflict.map(|(k, _)| k),
                computed: conflict.is_some_and(|(_, computed)| computed),
            };
            if record.branch.is_some() {
                record.child = Some(check);
            } else {
                record.node = Some(check);
            }
        }
    }

    /// Offers `set` as the incumbent if it beats it, recording when.  A proven ceiling on the sum is the run's, not
    /// the space's: the engine ends the run when the incumbent reaches it ([`crate::engine::EngineConfig::control`]).
    fn offer(&mut self, ctx: &mut Ctx<'_, TaxUnit<W>>, set: &Bits<W>) {
        #[cfg(feature = "studies")]
        if self.counting() {
            return;
        }
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
        }
    }

    /// With the tactic `forget_nogoods`, forgets the nogoods as the space's [`Forgetting`] says, after each of its
    /// periods of steps; with the tactic `lagrange`, refits the Lagrangian bound's multipliers when due, first.  A
    /// table publishes what it has done now and then ([`PUBLISH_EVERY`]).
    #[inline]
    fn forget_nogoods_when_due(&mut self, ctx: &mut Ctx<'_, TaxUnit<W>>) {
        if let (Some(l), Some(table)) = (&mut self.lagrange, &self.nogoods) {
            if let Some(phases) = &mut self.phases {
                phases.mark(Phase::Other);
            }
            l.step(table, &self.space.candidates, ctx.best());
            if let Some(phases) = &mut self.phases {
                phases.mark(Phase::Lagrange);
            }
        }
        if let (Some(table), Some(period)) = (&mut self.nogoods, self.space.forgetting.period) {
            self.steps_since_clear += 1;
            if self.steps_since_clear >= period {
                table.forget(self.space.forgetting.keep);
                self.steps_since_clear = 0;
            }
        }
        if ctx.stats().nodes.is_multiple_of(PUBLISH_EVERY) {
            self.publish_nogood_stats();
        }
    }

    /// Adds what the nogood table has done since it last published to [`SearchSpace::nogood_totals`].
    #[cold]
    fn publish_nogood_stats(&mut self) {
        if let Some(table) = &mut self.nogoods {
            let taken = table.take_stats();
            self.space.nogood_totals.lock().unwrap_or_else(PoisonError::into_inner).add(&taken);
        }
    }

    /// Adds this searcher's counts to the worker's statistics (and diagnostics) and resets them.
    fn flush(&mut self, stats: &mut Stats) {
        self.publish_nogood_stats();
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
        if let Some(near) = self.near_repeat {
            near.add_cut(counts.near_repeat);
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

/// The conflict a node branches on with the tactic `conflict_branching`: of `k0` (the one the search found), the
/// conflicts the nogood table knows within the heaviest set `eval.chosen`, and each failing frame's minimal conflict
/// made by leaving out its cheapest members first ([`FrameGraph::conflicts_ordered`]), the one of least weight.  A
/// member weighs 10^(−price/[`CONFLICT_BRANCHING_SLOPE`]), its price being what leaving it out costs the bound: exact
/// (read off the exchange graph, as strong branching reads it) for the members that some failing frame's conflict can
/// hold and those of `k0`, the weight split's estimate for the others.  The children of a conflict each leave out one
/// member, with the node's gap less that member's price, so the weight is their subtrees against the node's if a
/// subtree grows tenfold every [`CONFLICT_BRANCHING_SLOPE`] points of its gap.  A conflict whose members all cost much
/// makes every child's gap fall, and is often a set of forced members, which closes the node.  A conflict made here is
/// learned as a nogood, as the search learns its own.
///
/// Any conflict keeps the search exact: every completion leaves out some member of it, so the children cover every
/// completion.  The conflicts made here are conflicts, as [`FrameGraph::conflicts_ordered`] says.
///
/// With the tactic `conflict_look` (`look`), where another conflict than `k0` would be chosen and no member is forced,
/// the last child of `k0` is looked at first: by strong branching's order (the dearest first) it promotes every member
/// but the cheapest, `c`, and leaves `c` out.  Where `c` costs the bound at most [`CONFLICT_LOOK_PRICE`], `k0`'s weight
/// less `c`'s term is below [`CONFLICT_LOOK_SLACK`] times the best weight, and that child is cut on sight
/// ([`in_child_cut`]), the node branches on `k0`.  A child cut on sight is evidence of a hub, a member that costs the
/// bound almost nothing to leave out and that the completions holding the others of `k0` must take, and at a hub the
/// weight overrates every child of `k0`, not only the last.  `k0` and the conflict chosen are both true conflicts, so
/// the search stays exact; the look leaves the node's state as it found it.
#[allow(clippy::too_many_arguments)]
fn conflict_to_branch_on<const W: usize>(
    g: &mut Gf2Bound<'_, W>,
    model: &FrameGraph,
    scratch: &mut Scratch<W>,
    state: &FrameSet<W>,
    eval: &Eval<W>,
    k0: Bits<W>,
    cap: u32,
    mut learning: Option<&mut Learning<'_, W>>,
    known: &mut Vec<Bits<W>>,
    look: Option<Look<'_, W>>,
    promoted_sum: u64,
    mut memo: Option<(&mut PriceMemo<W>, &Reuse)>,
) -> Bits<W> {
    let members: Vec<u32> = eval.chosen.iter().collect();
    let exact: Vec<u32> = (model.conflict_members(scratch, state, &eval.chosen) | k0).iter().collect();
    // What leaving each member out costs the bound: priced now, or taken from the memo of the tactic `price_reuse`
    // for the members it has priced (the others are priced now, and kept).
    let (depth, node, parent) = g.node_ids();
    let bound = eval.value + promoted_sum;
    let mut taken = memo.as_mut().map(|(memo, reuse)| memo.take(depth, node, parent, bound, &eval.chosen, reuse));
    let mut own = std::collections::HashMap::new();
    let losses = match &mut taken {
        Some(taken) => &mut *taken.losses,
        None => &mut own,
    };
    let missing: Vec<u32> = exact.iter().copied().filter(|x| !losses.contains_key(x)).collect();
    if missing.is_empty() {
        // Nothing is priced here: the node's record of cycles for its children's pricing is its parent's, if that serves.
        g.inherit_pricing();
    } else {
        let mut bounds = Vec::new();
        g.prices_excluding(&missing, cap, &mut bounds);
        for (x, bound) in missing.into_iter().zip(bounds) {
            losses.insert(x, bound.map(|b| eval.value.saturating_sub(b)));
        }
    }
    let mut price = std::collections::HashMap::with_capacity(members.len());
    for &x in &members {
        let p = match losses.get(&x).copied().flatten() {
            Some(loss) => loss as f64,
            None => g.split_without(x).map_or(0.0, |b| (eval.value as i64 - b).max(0) as f64),
        };
        price.insert(x, p);
    }
    let kept_conflicts = taken.and_then(|taken| taken.conflicts);
    let weight = |k: &Bits<W>| {
        k.iter().map(|x| 10f64.powf(-price.get(&x).copied().unwrap_or(0.0) / CONFLICT_BRANCHING_SLOPE)).sum::<f64>()
    };
    let mut best = (weight(&k0), k0, false);
    if let Some(l) = learning.as_ref() {
        for k in l.table.all_within(&l.promoted, &eval.chosen) {
            let w = weight(&k);
            if w < best.0 {
                best = (w, k, false);
            }
        }
    }
    let mut order = members;
    order.sort_by(|a, b| price[a].total_cmp(&price[b]).then(a.cmp(b)));
    let ordered = match kept_conflicts {
        Some(kept) => kept,
        None => {
            let made = model.conflicts_ordered(scratch, state, &eval.chosen, &order);
            if let Some((memo, _)) = memo.as_mut() {
                memo.keep_conflicts(depth, &made);
            }
            made
        }
    };
    for k in ordered {
        let w = weight(&k);
        if w < best.0 {
            best = (w, k, true);
        }
    }
    if let Some(mut look) = look.filter(|_| best.1 != k0 && eval.forced.is_empty() && k0.len() >= 2) {
        let by_price = |a: &u32, b: &u32| price[a].total_cmp(&price[b]).then(a.cmp(b));
        let cheapest = k0.iter().min_by(by_price).expect("a conflict has members");
        let without_cheapest = weight(&k0) - weight(&Bits::EMPTY.with(cheapest));
        if price[&cheapest] <= CONFLICT_LOOK_PRICE
            && without_cheapest < best.0 * CONFLICT_LOOK_SLACK
            && in_child_cut(
                g,
                model,
                scratch,
                &mut look,
                state,
                k0.without(cheapest),
                cheapest,
                learning.as_deref_mut(),
            )
        {
            best = (without_cheapest, k0, false);
        }
    }
    let (_, k, computed) = best;
    if k != k0 {
        if computed {
            if let Some(l) = learning {
                if let Some(n) = model.nogood(scratch, l.root, &l.promoted, &k) {
                    l.table.insert(n);
                }
            }
        }
        known.push(k);
    }
    k
}

/// What the look of the tactic `conflict_look` needs of the node beyond what [`conflict_to_branch_on`] has: the node's
/// promotions and candidates, its need, the most promotions a completion can have, and the frames' last conflicts.
struct Look<'r, const W: usize> {
    cache: &'r mut ConflictCache<W>,
    promoted: Bits<W>,
    cands: Bits<W>,
    need: u64,
    max_total: u32,
}

/// Whether the child of the node on top of `g` (its state `state`) that promotes `added` and leaves out `m` is cut on
/// sight: its promotion fails, or its largest candidates, its GF(2) bound, or a conflict of its heaviest set all of
/// whose members it forces put it below its need (the checks of `check_child`, less the rule of `symmetry`, the
/// matching bound, the Lagrangian bound and the propagation of the node's known conflicts, so a "no" may be a child the
/// search would cut).  Its GF(2) bound is first bounded from the node's weight split ([`Gf2Bound::child_bound`]),
/// which is never below it: where that is below the need the child is cut as its solve would cut it, with nothing
/// pushed.  Otherwise the child is pushed on `g`, solved and popped.  The conflict found is learned as a nogood, as the
/// search learns its own, and is known to no node.  Nothing else a node's step reads changes: the node's level in `g`
/// resolves again at its next read, as after any child.
#[allow(clippy::too_many_arguments)]
fn in_child_cut<const W: usize>(
    g: &mut Gf2Bound<'_, W>,
    model: &FrameGraph,
    scratch: &mut Scratch<W>,
    look: &mut Look<'_, W>,
    state: &FrameSet<W>,
    added: Bits<W>,
    m: u32,
    learning: Option<&mut Learning<'_, W>>,
) -> bool {
    // As `make_child` promotes: one number in place, several together with every frame they touch left with freedoms.
    let child = if added.len() == 1 {
        let mut child = *state;
        model.promote_in_place_using(scratch, &mut child, added.max().expect("one member")).then_some(child)
    } else {
        model.promote_all_using(scratch, state, &added).filter(|child| {
            added.iter().all(|x| {
                let (into, out) = model.frames_touched(x);
                let frames = child.frames();
                frames[into].freedoms >= 0 && out.is_none_or(|out| frames[out].freedoms >= 0)
            })
        })
    };
    let Some(child) = child else { return true };
    let promoted = look.promoted | added;
    let Some(need) = look.need.checked_sub(added.sum()) else { return false };
    let rest = (look.cands - added).without(m);
    let max_next = max_additional(model, look.max_total, &child, &promoted, &rest);
    let mut cands = model.narrow(&child, &rest);
    let mut learning = learning.map(|l| Learning { table: &mut *l.table, root: l.root, promoted });
    if let Some(l) = learning.as_mut() {
        cands -= l.table.ruled_out(&added, &promoted, &cands);
    }
    if cands.sum_largest(max_next) < need {
        return true;
    }
    // An upper bound on the child's GF(2) bound from the node's weight split, with no solve: below the need, the
    // bound is too, and the child is cut as the solve would cut it.
    let split_bound = g.child_bound(&added, &cands);
    if split_bound.is_some_and(|bound| bound < need) {
        #[cfg(debug_assertions)]
        {
            g.push_all(&added, &cands);
            let value = g.evaluate(max_next, None).value;
            g.pop();
            assert!(value <= split_bound.unwrap(), "the child's bound {value} is above its split bound");
        }
        return true;
    }
    g.push_all(&added, &cands);
    let eval = g.evaluate(max_next, Some(need));
    debug_assert!(split_bound.is_none_or(|bound| eval.value <= bound), "a child's bound is above its split bound");
    let cut = eval.value < need || {
        let mut known = Vec::new();
        let mut seek =
            Seek { model, scratch, cache: &mut *look.cache, state: &child, known: &mut known, learning: &mut learning };
        match seek.conflict(&eval.chosen, eval.forced).0 {
            Lookup::AllForced => true,
            Lookup::None => false,
            Lookup::Conflict(k) if k.is_subset(&eval.forced) => true,
            // The table's conflict is not all forced, but the smallest a peel finds may be: learned as a nogood, it
            // cuts the child when the search makes it (the table's lookup finds it all forced).
            Lookup::Conflict(_) => {
                match model.conflict_cached(scratch, Some(&mut *look.cache), &child, &eval.chosen, true) {
                    Some(k) if k.is_subset(&eval.forced) => {
                        if let Some(l) = learning.as_mut() {
                            if let Some(nogood) = model.nogood(scratch, l.root, &promoted, &k) {
                                l.table.insert(nogood);
                            }
                        }
                        true
                    }
                    _ => false,
                }
            }
        }
    };
    g.pop();
    cut
}
