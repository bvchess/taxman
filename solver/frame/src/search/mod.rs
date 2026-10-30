//! The branch-and-bound search for the promotion set with the largest sum, run on the parallel [`engine`].
//!
//! `solver/docs/design/search.md`, explains the algorithm: nodes, steps and units, the GF(2) bound, conflicts, forced
//! members and nogoods, the pseudocode of "The default step", and why the search is exact.  The docs here say where
//! each piece of it is, and carry the arguments next to the code they justify.
//!
//! The words the code uses: a *node* is a set `P` of promotions, with the state they reach (the root with `P`
//! promoted, in any order) and the candidates `C` its loop has still to try; a *completion* of the node is a set
//! `F ⊆ C` that can be promoted from its state; `need = best + 1 − sum(P)` is what a completion must add to beat
//! the incumbent (worth `best`).  A *step* is one turn of a node's loop: it makes one child, or ends the loop.  The
//! "nodes" that the engine, the results and the progress lines count are steps.  A *unit* ([`TaxUnit`]) is a node
//! as the engine passes it between threads and to checkpoints: `P` and `C` only, the state rebuilt in one pass.
//!
//! # The default step in the code
//!
//! [`TaxSearcher`] runs each unit as a loop of steps on its own stack of nodes.  A step (`searcher::Stepper::step`)
//! has four phases, which take the steps of solver/docs/design/search.md's "The default step" in order:
//!
//! * `check_node` (steps 1 and 2): does the node's loop go on?  The sum of its `max_add` largest candidates, then
//!   the node's bound check, or the verdict the node's check left when it was made as a child, if `need` is the
//!   same.
//!   With the tactic `symmetry`, the rule of [`symmetry`] comes between them (`Stepper::apply_symmetry`): it cuts
//!   the node, or drops candidates from its loop, and the sum of the largest is checked again for what is left.
//! * `choose_branch` (steps 3 to 5): the member of the conflict to branch on (with strong branching), or with
//!   `fix_forced` the forced members, which the child promotes together.
//! * `make_child` (steps 6 and 7): the child's state and candidates, without the last member of a conflict known
//!   along the path or of a nogood, and with `symmetry` without the candidates the rule drops.
//! * `check_child` (steps 8 and 9): the child's own check, which with `symmetry` begins with the rule's test of the
//!   child (cut if no completion keeps the inequalities); then it is offered as the incumbent and pushed.
//!
//! The bound check of a node or a child is `Stepper::check`: the GF(2) bound ([`crate::gf2`]), the forced members,
//! and the conflict of the heaviest set (the smallest the nogood table knows, [`crate::model::Nogoods`], else
//! [`FrameGraph::conflict_using`](crate::model::FrameGraph::conflict_using): `seek::Seek`).  The tactics that switch
//! its pieces are the fields of [`SearchTactics`].  Before the bound check of a node or a child, with the tactic
//! `near_repeat`, comes the near-repeat cut (`Stepper::near_repeat_cuts`, [`NearRepeat`]), where the search was given a
//! game proven before, at its setup or while it runs, and the incumbent reaches the cut's bound.
//!
//! # Where the arguments are
//!
//! * The sum of the `max_add` largest candidates bounds every completion by itself, not as a shortcut for the
//!   other bounds: no completion has more promotions than the freedoms allow (see [`crate::model::FrameSet`]),
//!   nor larger ones.
//! * The bound and its forced members: [`crate::gf2`].  Conflicts:
//!   [`FrameGraph::conflict_using`](crate::model::FrameGraph::conflict_using); nogoods: [`crate::model::Nogoods`];
//!   the rule-outs of children: `Stepper::make_child`.
//! * The rule of the tactic `symmetry` ([`symmetry`]): every heaviest promotion set keeps the inequality
//!   `c_p ≤ c_q` of each symmetric pair `(p, q)` of the root (the prime-exchange theorem (R2),
//!   docs/theory/search-rules.md), so a node none of whose completions keeps them all holds no heaviest set, and
//!   neither does a completion that holds a candidate no such completion holds: the node is cut and the candidate
//!   dropped, whatever bound the search uses.  The other prunings hold for the completions that keep the inequalities
//!   as for all (bounds above every completion, members every completion that beats the incumbent promotes, sets that
//!   cannot be promoted).  The rule is a function of the node's promotions and candidates, so it changes which tree is
//!   searched, not the answer of a search from the root.  A search of a node below the root finds the best of its
//!   completions that keep the inequalities, which can be lighter than its best completion (the exchange-pruning lemma
//!   (R3), docs/theory/search-rules.md, part 3).
//! * The near-repeat cut ([`NearRepeat`]; the near-repeat lemma (R7), docs/theory/search-rules.md): a node whose
//!   completions are promotion sets of a game M proven before holds none worth more than M's best promotion sum, so
//!   where the incumbent reaches a proven bound of that sum, it holds none that beats the incumbent.  The cut drops
//!   only such nodes, as a bound does, and learns nothing.
//! * What a searcher remembers (the conflicts known along the path, the nogoods, the frames' last conflicts, the
//!   levels of the bound kept by strong branching, a child's verdict) are facts that hold where they are used, or
//!   caches of them: they change which tree is searched, never the answer.  So a search's tree depends on its
//!   threads and interruptions; its answer does not.
//!
//! # What is where
//!
//! * `tactics`: [`SearchTactics`], and how a checkpoint records them.
//! * `branching`: [`Branching`], the branching settings of a search, whose gate its gate trial may change while it runs.
//! * `setup`: [`prepare`]: the root, the greedy incumbent, `max_total`; the [`Problem`] a setup poses.
//! * `incumbent`: the starting incumbent (seed, dive, local search).
//! * `searcher`: [`TaxSearcher`], its stack and its steps; [`SearchSpace`], [`TaxUnit`], the counters [`stat`].
//! * `seek`: where a node's conflicts come from (`Seek`): the nogood table, else a peel.
//! * `header`: the checkpoint's header.
//! * `dual`: [`dual_bound`], the upper bound that a stopped search's checkpoint proves, and [`NodeBound`], the bound
//!   of a unit's node as the search computes it when it starts the unit.
//! * [`symmetry`]: the root's prime exchanges (symmetric pairs of primes) and the rule they give the search.
//! * `near_repeat`: [`NearRepeat`], the setup of the near-repeat cut and its checks at a node.
//! * `price_memo`: [`Reuse`], when the tactic `price_reuse` lets a choice of the conflict take the prices it made
//!   before, and the memo that keeps them.
//! * `profile`: [`SearchProfile`], the profile of a search (`taxman solve --profile`): its tree of tagged nodes, the
//!   searchers' counts, its file.
//! * here: [`search`], which starts or resumes the engine on a prepared game, and [`search_above`], the same from a
//!   floor (an incumbent worth a given sum and no set) instead of a start incumbent; [`start_incumbent`], the incumbent
//!   [`search`] starts from; [`offer_seed`], [`best_so_far`] and [`seed_survivors_sum`], for a caller that steers
//!   searches through their [`Control`] (`EngineConfig::control`), and [`search_steered`], whose branching settings the
//!   gate trial may change as it runs.

mod branching;
mod dual;
pub mod finder;
pub mod gate_trial;
mod header;
mod incumbent;
mod near_repeat;
mod price_memo;
pub use price_memo::Reuse;
mod profile;
mod searcher;
mod seek;
mod setup;
pub mod symmetry;
mod tactics;

pub use branching::{BranchSettings, Branching};
pub use dual::{dual_bound, DualBound, NodeBound};
pub use incumbent::Neighborhood;
pub use near_repeat::NearRepeat;
pub use profile::{ProfilePlan, SearchProfile, DEPTH as PROFILE_DEPTH, MOST_IDS as PROFILE_MOST_IDS};
pub use searcher::{stat, Found, SearchSpace, TaxSearcher, TaxUnit};
pub use setup::{
    extended_words, prepare, prepare_extended, prepare_extended_for_resume, prepare_for_resume, problem_of,
    problem_of_extended, PrepareTimings, Prepared, Problem, ProblemSize,
};
pub use tactics::SearchTactics;

use crate::bits::Bits;
use crate::engine::{
    self, CheckpointEvent, CheckpointHook, Control, Counters, EngineConfig, ProgressNote, Start, Stats,
};
use crate::matchbound::MatchGraph;
use crate::model::{store_file, CarriedNogoods, Harvest, NogoodStats};
use header::Header;
use incumbent::{distinct, dive, surviving};
use profile::Begin;
use searcher::{get_bits, put_bits};
use std::io;
use std::path::Path;
use std::sync::{Arc, OnceLock, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// With the tactic `forget_nogoods`, a searcher forgets its nogoods after each this many steps.  Every interval from
/// 10,000 to 100,000 steps was about as good on one thread (22–26% less time than never forgetting on hard games near
/// n = 1200, games of fewer steps unchanged); see solver/notebook/search/forgetting-nogoods-2026-10/README.md.
pub const NOGOOD_PERIOD: u64 = 50_000;

/// With the tactic `conflict_branching`, a check chooses the conflict it branches on only where its gap, the GF(2) bound
/// less the need, is at least this (below it the search's own conflict is kept): the best of 0, 200, 300 and 400 on
/// the whole game 4050 and two units of 4548 and 4875 (solver/notebook/search/
/// where-the-search-of-4875-spends-and-which-conflict-to-branch-on-2026-10/README.md).
pub const CONFLICT_BRANCHING_GAP: u64 = 300;

/// The widest gate a caller may give conflict branching in place of [`CONFLICT_BRANCHING_GAP`]: `solve --gate` and the
/// gates of `solve --gate-trial` take whole numbers from 0 to this.  Any gate keeps the search exact: below it a check
/// branches on the search's own conflict, from it on the conflict chosen, and both are true conflicts.
pub const MAX_GATE: u64 = 4000;

/// With the tactic `conflict_branching`, a conflict weighs the sum over its members of 10^(−price/slope), with this
/// slope, a member's price being what leaving it out costs the GF(2) bound: the subtrees of its children against the
/// node's, if a subtree grows tenfold for every this many points of its gap, as the subtrees of 4875 did (the same
/// entry).
pub const CONFLICT_BRANCHING_SLOPE: f64 = 291.0;

/// With the tactic `conflict_look`, the look is made only where the cheapest member of the search's own conflict has a
/// price of at most this (a member that costs the bound almost nothing to leave out is a hub of the heaviest set).  The
/// setting of the study of when a node should use conflict branching (solver/notebook/search/
/// when-a-node-should-use-conflict-branching-2026-10/README.md).
pub const CONFLICT_LOOK_PRICE: f64 = 10.0;

/// With the tactic `conflict_look`, the look is made only where the weight of the search's own conflict less the term
/// of its cheapest member is below this many times the weight of the conflict that conflict branching chose (the same
/// entry).
pub const CONFLICT_LOOK_SLACK: f64 = 2.0;

/// The outcome of [`search`].
#[derive(Clone, Debug)]
pub struct SearchResult<const W: usize> {
    /// The best promotion set found (an optimal set when `completed`; empty if `floor_held`).
    pub promotions: Bits<W>,
    pub sum: u64,
    /// The search finished (or reached the ceiling); false when an engine limit or a stop stopped it.
    pub completed: bool,
    /// Under a floor ([`search_above`]): the search completed and found no set heavier than the floor, so no
    /// promotion set is (the floor lemma (R5), docs/theory/search-rules.md).  Then `sum` is the floor and `promotions`
    /// is empty.  Always false when the search has no floor.
    pub floor_held: bool,
    /// The incumbent reached the ceiling (`cap`, or one the control was given), which proves it optimal.
    pub reached_cap: bool,
    /// Engine statistics; see [`stat`] for `extra`.
    pub stats: Stats,
    pub counters: Counters,
    /// What the searchers' nogood tables did in this call (not in the runs before a resume), summed over the
    /// searchers: see [`NogoodStats`].
    pub nogoods: NogoodStats,
    /// Wall time of this call.
    pub time: Duration,
    /// Wall time of the search over all its runs: the engine's time in the runs before a resume (each counted up
    /// to its last checkpoint) plus this call's.
    pub time_all_runs: Duration,
    /// When this call found its best incumbent (`None` if it found none better than the starting one).
    pub found: Option<Found>,
    /// What the finder did, if it started ([`finder`]: a search is given a [`finder::Plan`] by [`search_beside`]).
    pub finder: Option<finder::Report>,
    /// What the gate trial did, if it ran to its end ([`gate_trial`]: a search is given a [`gate_trial::Plan`] by
    /// [`search_steered`]).
    pub gate_trial: Option<gate_trial::Report>,
    /// The incumbent the search started from: the best start of any seed ([`start_incumbent`]), each the best of the
    /// greedy set, the seed's survivors and the matching dive, improved by the local search (the dive and the local
    /// search are skipped when the set before them reaches the ceiling, and the local search ends when its set does).
    /// Empty under a floor, which is a value and no set.
    pub start: Bits<W>,
}

/// Searches for the promotion set with the largest sum.
///
/// The incumbent starts as the best of the greedy set, what survives of `seed` (its candidates that can still
/// be promoted, from the largest down) and the dive (the root's matching rounded to a set that can be
/// promoted), improved by a local search ([`Neighborhood::improve_on`] on the run's threads, `cfg.threads` or the
/// control's workers, with the answer of [`Neighborhood::improve`]).  The start is offered to the run's control as
/// soon as it is known and each time it improves (after the best of the greedy set and the seed's survivors, after the
/// dive and after the local search, where they improve it), so that [`best_so_far`] has it before the engine runs.
/// A search with several seeds ([`search_beside`], [`search_steered`]) makes this start from each of them and begins
/// from the best ([`start_incumbent`]).
///
/// The ceiling is a proven upper bound on the sum; reaching it ends the search, also before it starts.  It is `cap`
/// and any the run's control is given, before or while the search runs (the control is `cfg.control` if there is
/// one, else the search's own; the lowest ceiling counts, see [`Control::set_ceiling`]).  Nothing can beat a set that
/// reaches it, so the dive is skipped when the best of the greedy set and the seed's survivors does, and the local
/// search when the incumbent after the dive does: neither could change it as long as the ceiling holds, which the
/// proof by the ceiling assumes (`solver::conclude` rejects any game above it).  The dive reads the ceiling when it is
/// due and the local search before each of its passes, which it ends when its set reaches it, so one given while the
/// dive or the local search runs counts.
///
/// With `resume`, the search continues from the checkpoint configured in `cfg` (the `prepared` game must have been
/// set up by [`prepare_for_resume`] or identically): its header must be this game's, and its incumbent a promotable
/// set of candidates worth what the checkpoint says.  Otherwise it starts fresh.
///
/// # Errors
///
/// The engine's, and `InvalidData` for a checkpoint that does not belong to this game.
pub fn search<const W: usize>(
    prepared: &Prepared<W>,
    cfg: &EngineConfig,
    seed: Option<&Bits<W>>,
    cap: Option<u64>,
    resume: bool,
) -> io::Result<SearchResult<W>> {
    search_beside(prepared, cfg, seed.map(std::slice::from_ref).unwrap_or_default(), cap, resume, None)
}

/// [`search`] from a *floor* instead of a start incumbent: a search for a promotion set worth more than `floor`, a
/// promotion sum.  The incumbent it starts from is worth `floor` and is no set, so no start is made (no greedy set,
/// seed, dive or local search), and nodes are pruned against `floor` as against an incumbent of that worth.  A set
/// heavier than the floor becomes the incumbent as usual, and is a real game (so a lower bound on the game's optimum).
/// If the search completes and finds none, no promotion set is heavier than `floor`
/// ([`SearchResult::floor_held`], the floor lemma (R5), docs/theory/search-rules.md): an upper bound on the game's best
/// promotion sum, proven by the tree of a search from an incumbent worth `floor`: the proof's, when the floor is P*.
/// `cap` and `resume` are as for [`search`]; a search that a limit stopped resumes from its checkpoint under the same
/// floor.
///
/// # Errors
///
/// As [`search`]'s, and for a checkpoint that holds a floor and no set under another floor or none, or whose incumbent
/// is worth less than `floor`.
pub fn search_above<const W: usize>(
    prepared: &Prepared<W>,
    cfg: &EngineConfig,
    floor: u64,
    cap: Option<u64>,
    resume: bool,
) -> io::Result<SearchResult<W>> {
    let forget_period = prepared.tactics.forget_period();
    search_forgetting(prepared, cfg, &[], cap, resume, forget_period, Some(floor), None, None, None, None, None, None)
}

/// [`search`] with any number of `seeds` (the incumbent starts as the best start of any of them,
/// [`start_incumbent`]) and the finder of `plan` beside it, if there is one: once the search has taken `plan.after`
/// steps, a thread of its own looks for a better game than the incumbent and offers it ([`finder`]).  The finder only
/// offers sets, each after the check of [`offer_found`], so the search stays exhaustive and its answer is the same as
/// without it; it is not part of what a checkpoint is checked against, so a checkpoint resumes with any finder or none.
pub fn search_beside<const W: usize>(
    prepared: &Prepared<W>,
    cfg: &EngineConfig,
    seeds: &[Bits<W>],
    cap: Option<u64>,
    resume: bool,
    plan: Option<&finder::Plan<W>>,
) -> io::Result<SearchResult<W>> {
    let forget_period = prepared.tactics.forget_period();
    search_forgetting(prepared, cfg, seeds, cap, resume, forget_period, None, plan, None, None, None, None, None)
}

/// [`search_beside`] with the branching settings `branching`, whose gate the search's gate trial may change while the
/// search runs: each searcher reads them when it starts a unit ([`Branching`]).  They start as the caller left them,
/// whatever `prepared.tactics` says of them, and a fresh search's checkpoints record them as they were when it
/// started.  A setting there chooses only which conflict a node branches on, so the search's answer is the same
/// whatever they say and however often they change (solver/docs/design/search.md, "The invariant of a running
/// search").
///
/// With the tactic `near_repeat` on, the search cuts by the game proven before ([`NearRepeat`]) that `near_repeat` holds,
/// once it holds one: the caller may set it before the search or while it runs, once, and each searcher picks it up
/// when it next starts a unit.  A node is cut only where the incumbent reaches the cut's bound
/// ([`NearRepeat::reached`]), so a cut armed below it waits for the incumbent.
///
/// With `profile`, the search keeps the profile that it asks for in its file ([`SearchProfile`]), which observes the
/// search and changes nothing of it: a profile that cannot be written is said on stderr, and the search goes on.
///
/// With the tactic `carried_nogoods` and a directory of stores `carried`, the search carries the nogoods of the stores
/// there of its game and the games before it, validated at its root, at its start and at each resume
/// ([`CarriedNogoods::from_dir`]); they only prune (solver/docs/design/carried-nogoods.md).  It writes its own store
/// there too, of the nogoods its searchers learn ([`Harvest`]), before each checkpoint and at its end.
///
/// With `trial`, conflict branching on and a checkpoint, the search tries the gates of the plan on
/// a sample of its own units once it has taken the plan's steps, and branches from then on with the gate that took the
/// least CPU ([`gate_trial`]); the trial changes only the gate, so the search's answer is the same.
#[allow(clippy::too_many_arguments)]
pub fn search_steered<const W: usize>(
    prepared: &Prepared<W>,
    cfg: &EngineConfig,
    seeds: &[Bits<W>],
    cap: Option<u64>,
    resume: bool,
    plan: Option<&finder::Plan<W>>,
    branching: &Arc<Branching>,
    near_repeat: &OnceLock<NearRepeat<W>>,
    profile: Option<&ProfilePlan>,
    carried: Option<&Path>,
    trial: Option<&gate_trial::Plan>,
) -> io::Result<SearchResult<W>> {
    let (forget_period, near_repeat, branching) =
        (prepared.tactics.forget_period(), Some(near_repeat), Some(branching));
    search_forgetting(
        prepared,
        cfg,
        seeds,
        cap,
        resume,
        forget_period,
        None,
        plan,
        branching,
        near_repeat,
        profile,
        carried,
        trial,
    )
}

/// [`search`], the searchers forgetting their nogoods after each `forget_period` steps (never if `None`) whatever the
/// tactics say, so that tests can make them forget often, under `floor` if given (then without a seed), with the
/// branching settings `branching` if given (else those of the tactics), with the near-repeat cut that `near_repeat` holds or
/// comes to hold, if given, keeping the profile `profile` asks for, if given, carrying the nogoods of the directory
/// of stores `carried_dir`, if given, with the tactic `carried_nogoods`, and with the gate trial of `trial`, if given
/// ([`search_steered`]).
#[allow(clippy::too_many_arguments)]
fn search_forgetting<const W: usize>(
    prepared: &Prepared<W>,
    cfg: &EngineConfig,
    seeds: &[Bits<W>],
    cap: Option<u64>,
    resume: bool,
    forget_period: Option<u64>,
    floor: Option<u64>,
    plan: Option<&finder::Plan<W>>,
    branching: Option<&Arc<Branching>>,
    near_repeat: Option<&OnceLock<NearRepeat<W>>>,
    profile: Option<&ProfilePlan>,
    carried_dir: Option<&Path>,
    trial: Option<&gate_trial::Plan>,
) -> io::Result<SearchResult<W>> {
    debug_assert!(floor.is_none() || seeds.is_empty(), "a floor is the start: there are no seeds");
    let t0 = Instant::now();
    let control = cfg.control.clone().unwrap_or_else(Control::new);
    if let Some(cap) = cap {
        control.set_ceiling(cap);
    }
    let cfg = &EngineConfig { control: Some(Arc::clone(&control)), ..cfg.clone() };
    let match_graph = MatchGraph::new(&prepared.model, &prepared.root);
    // The incumbent the search starts from, as a set and its value: the start incumbent, or the floor, a value above
    // the empty set that stands for no set at all.
    let incumbent = match floor {
        None => start_incumbent(prepared, &match_graph, &control, cfg.threads, seeds),
        Some(_) => Bits::EMPTY,
    };
    let from = (incumbent, floor.unwrap_or_else(|| incumbent.sum()));
    let checkpoint = cfg.checkpoint.as_ref().map(|(path, _)| path.as_path());
    let resumed = if resume {
        let path =
            checkpoint.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "resuming needs a checkpoint"))?;
        Some(resume_point(prepared, path, floor).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?)
    } else {
        None
    };
    // What the search would start from (a set and its value), and the totals of the runs before this one.
    let (best, before, earlier) = match &resumed {
        Some((kept, head)) => (if head.best >= from.1 { (*kept, head.best) } else { from }, head.stats, head.elapsed),
        None => (from, Stats::default(), Duration::ZERO),
    };
    if reaches(best.1, control.ceiling()) {
        // A resumed search that reaches the ceiling at once is finished like any other.
        if let (true, Some(path)) = (resume && !cfg.keep_completed_checkpoint, checkpoint) {
            engine::remove_finished_checkpoint(path);
        }
        return Ok(SearchResult {
            promotions: best.0,
            sum: best.1,
            completed: true,
            floor_held: floor == Some(best.1),
            reached_cap: true,
            stats: before,
            counters: Counters::default(),
            nogoods: NogoodStats::default(),
            time: t0.elapsed(),
            time_all_runs: earlier + t0.elapsed(),
            found: None,
            finder: None,
            gate_trial: None,
            start: incumbent,
        });
    }

    // The near-repeat cut, with the tactic: a node is cut only where the incumbent reaches its bound (the setup's last
    // check), which the searchers compare at each node, since the cut may be armed when it is below.
    let near_repeat = near_repeat.filter(|_| prepared.tactics.near_repeat);
    // The carried nogoods, with the tactic: the stores of this game and the games before it, checked at this root.
    let carried = carried_dir.filter(|_| prepared.tactics.carried_nogoods).map(|dir| {
        let (store, v, files) =
            CarriedNogoods::from_dir(dir, prepared.n, &prepared.model, &prepared.root, &prepared.candidates);
        eprintln!(
            "n={}: carried nogoods: {} kept of {} sets from {} files of {} ({} with a number that is no candidate, {} \
             promotable)",
            prepared.n,
            v.kept,
            v.given,
            files.len(),
            dir.display(),
            v.not_candidates,
            v.promotable
        );
        store
    });
    // What the searchers learn, written as this game's store (the store of an earlier run of its search kept).
    let harvest = carried.as_ref().zip(carried_dir).map(|(_, dir)| {
        let path = store_file(dir, prepared.n);
        (Arc::new(Harvest::resumed(&path, &prepared.candidates)), path)
    });
    let space = SearchSpace {
        forget_period,
        began: t0,
        near_repeat,
        carried: carried.as_ref(),
        harvest: harvest.as_ref().map(|(h, _)| &**h),
        ..SearchSpace::new(prepared)
    };
    let space = match branching {
        Some(branching) => SearchSpace { branching: Arc::clone(branching), ..space },
        None => space,
    };
    let profile = profile.and_then(|plan| {
        let begin = match &resumed {
            Some((_, head)) => Begin::Resumed { nodes: head.stats.nodes, elapsed: head.elapsed },
            None => Begin::Fresh {
                bound: {
                    let root = TaxUnit { promoted: Bits::EMPTY, cands: prepared.candidates };
                    NodeBound::new(prepared, &space.gf2).completions(&root)
                },
                need: best.1 + 1,
                candidates: prepared.candidates.len(),
            },
        };
        let threads = control.workers().unwrap_or(cfg.threads);
        match SearchProfile::open(plan, prepared, &begin, threads, best.1) {
            Ok(profile) => Some(Arc::new(profile)),
            Err(e) => {
                eprintln!("warning: no profile of the search of {}: {e}", prepared.n);
                None
            }
        }
    });
    let space = SearchSpace { profile: profile.as_deref(), ..space };
    // The progress lines tell what the nogood tables have done.
    let totals = Arc::clone(&space.nogood_totals);
    let note = ProgressNote(Arc::new(move || totals.lock().unwrap_or_else(PoisonError::into_inner).summary()));
    // A profile is written at each checkpoint, before the engine writes it, and so is the game's store.
    let hook = profile.as_ref().map(|profile| {
        let profile = Arc::clone(profile);
        CheckpointHook(Arc::new(move |event| profile.checkpoint(event)))
    });
    let hook = match &harvest {
        None => hook,
        Some((harvest, path)) => {
            let (harvest, path) = (Arc::clone(harvest), path.clone());
            Some(CheckpointHook(Arc::new(move |event| {
                if matches!(event, CheckpointEvent::Writing { .. }) {
                    write_store(&harvest, &path);
                }
                if let Some(CheckpointHook(inner)) = &hook {
                    inner(event);
                }
            })))
        }
    };
    // The gate trial, which needs a checkpoint for its units, takes the first checkpoint written at its plan's steps or
    // more.
    let trial = trial.filter(|_| checkpoint.is_some()).map(|plan| (plan, gate_trial::Checkpoints::new(plan.after)));
    let hook = match &trial {
        None => hook,
        Some((_, (checkpoints, _))) => {
            let checkpoints = Arc::clone(checkpoints);
            Some(CheckpointHook(Arc::new(move |event| {
                if let Some(CheckpointHook(inner)) = &hook {
                    inner(event);
                }
                checkpoints.event(event);
            })))
        }
    };
    let cfg = &EngineConfig { progress_note: Some(note), checkpoint_hook: hook, ..cfg.clone() };
    let start = match checkpoint.filter(|_| resume) {
        Some(path) => Start::Resume(path.to_path_buf()),
        None => {
            let root = TaxUnit { promoted: Bits::EMPTY, cands: prepared.candidates };
            Start::Fresh { units: vec![root], best: from.1, payload: payload_of(&from.0) }
        }
    };
    // A resumed run goes on under the header it was written with, which `resume_point` read and found to be this
    // search's: the engine checks the stored header byte for byte, and one whose bits that bind nothing
    // ([`SearchTactics::same_recorded`]) differ from this run's is this search's without being this run's encoding of
    // it.  So the checkpoints of a run keep the header it began with.
    let header = match &resumed {
        Some((_, head)) => head.header.clone(),
        None => {
            // The branching settings as the search starts with them, which the caller may have changed.
            let mut header = Header::of(prepared);
            let settings = space.branching.settings();
            header.tactics.conflict_branching = settings.conflict_branching;
            header.tactics.conflict_look = settings.conflict_look;
            header.encode()
        }
    };
    let engine_start = Instant::now();
    let label = cfg.progress.map(|_| cfg.label.as_str());
    // The generation of the branching settings as the search starts: the trial sets its gate only if they are unchanged.
    let armed = space.branching.generation();
    let run = || {
        let trial = trial.map(|(plan, (checkpoints, receiver))| {
            let trial = gate_trial::Trial {
                plan,
                prepared,
                gf2: &space.gf2,
                branching: &space.branching,
                armed,
                near_repeat,
                carried: carried.as_ref(),
                forget_period,
                background: cfg.background,
                label: &cfg.label,
            };
            (trial, checkpoints, receiver)
        });
        let ((outcome, finder), trial) = gate_trial::run_beside(trial, || {
            finder::run_beside(prepared, &control, plan, before.nodes, label, || {
                engine::run(cfg, start, &header, |_| TaxSearcher::new(&space))
            })
        });
        (outcome, finder, trial)
    };
    let (outcome, finder, trial) = match profile.as_deref() {
        None => run(),
        // Beside the search, a thread summarizes the open units of each checkpoint; the profile ends with the run.
        Some(profile) => thread::scope(|scope| {
            let (graph, background) = (&space.gf2, cfg.background);
            let jobs = profile.jobs().expect("a profile's checkpoints are taken once");
            let summary = scope.spawn(move || profile::open_work(profile, prepared, graph, jobs, background));
            let ending = profile::Ending(profile);
            let ran = run();
            match &ran.0 {
                Ok(o) => profile.finish(Some(o.stats.nodes), o.completed, o.best, o.elapsed, None),
                Err(e) => {
                    let elapsed = earlier + engine_start.elapsed();
                    profile.finish(None, false, best.1, elapsed, Some(&e.to_string()));
                }
            }
            drop(ending);
            summary.join().expect("the profile's summary of the open work");
            ran
        }),
    };
    if let Some((harvest, path)) = &harvest {
        write_store(harvest, path);
    }
    let outcome = outcome?;
    // The engine's elapsed time includes the runs before a resume; what it counts beyond this call is theirs.
    let earlier_runs = outcome.elapsed.saturating_sub(engine_start.elapsed());
    let promotions = set_of_payload(&outcome.payload);
    // Under a floor that nothing beat, the incumbent is the floor and no set.
    let floor_stands = floor == Some(outcome.best);
    assert!(
        promotions.sum() == outcome.best || (floor_stands && promotions.is_empty()),
        "the incumbent's payload does not match its value"
    );
    let time = t0.elapsed();
    let nogoods = *space.nogood_totals.lock().unwrap_or_else(PoisonError::into_inner);
    Ok(SearchResult {
        promotions,
        sum: outcome.best,
        completed: outcome.completed,
        floor_held: outcome.completed && floor_stands,
        reached_cap: outcome.optimal_declared,
        stats: outcome.stats,
        counters: outcome.counters,
        nogoods,
        time,
        time_all_runs: time + earlier_runs,
        found: space.found.into_inner().unwrap_or_else(PoisonError::into_inner),
        finder,
        gate_trial: trial,
        start: incumbent,
    })
}

/// Writes the store of `harvest` to `path`, saying on stderr if it cannot: a store is only ever a help.
fn write_store<const W: usize>(harvest: &Harvest<W>, path: &Path) {
    if let Err(e) = harvest.write(path) {
        eprintln!("warning: carried nogoods: {} not written: {e}", path.display());
    }
}

/// The incumbent a search starts from, made as [`search`] makes it (its documentation has the steps): a start from
/// each of `seeds` and the best of them.  A seed's start is the greedy set, or what survives of the seed if that is
/// heavier, then the dive if that is heavier still, then the local search on `threads` threads (the control's workers
/// if it has them).  With no seed there is one start, from the greedy set.  `graph` is the matching graph of `prepared`.
///
/// The dive is made once, and the local search runs once from each distinct set the starts have reached by then.  The
/// best result wins, the first of equals, so the order of `seeds` decides a tie and nothing else does: the thread count
/// does not.  With one seed the result is that seed's start alone.
///
/// The incumbent is offered to `control` as soon as it is known and each time it improves, so that a caller reading the
/// control ([`best_so_far`]) has it while the start is still being computed, not only once the engine runs.  The run
/// starts from exactly what it started from without the offers: the engine takes the better of its `Start`'s incumbent
/// and the best offered to the control, strictly better only, and the start's final incumbent is at least every one
/// offered, so it keeps its own, and the trees do not change.  (A resumed run's `Start` is its checkpoint, whose
/// incumbent is at least the start of the run that wrote it: the same, with the same seeds, so it keeps its own too;
/// one that a start beats is replaced by it, as `best` in [`search`] counts it.)
///
/// A start that reaches the control's ceiling cannot be beaten, so the dive, the local search and the starts not yet
/// made are skipped: they could not change it if the ceiling holds, as the proof by the ceiling assumes (and
/// `solver::conclude` rejects any game above it).  The dive is taken only when strictly heavier, and the local search
/// keeps only strictly heavier sets, after a first fill that can only add candidates; no promotable set is heavier
/// than the ceiling.  The ceiling is read when each is due, and by the local search before each pass: the control may
/// be given a lower one meanwhile.
pub fn start_incumbent<const W: usize>(
    prepared: &Prepared<W>,
    graph: &MatchGraph,
    control: &Control,
    threads: usize,
    seeds: &[Bits<W>],
) -> Bits<W> {
    // The greedy set, or the survivors of a seed if they are heavier: one start for each seed.
    let mut starts: Vec<Bits<W>> = if seeds.is_empty() {
        vec![prepared.greedy]
    } else {
        seeds.iter().map(|seed| heavier(prepared.greedy, surviving(prepared, seed))).collect()
    };
    for start in &starts {
        offer_set(control, start);
    }
    let at_ceiling = |starts: &[Bits<W>]| {
        let best = heaviest(starts.iter().copied());
        reaches(best.sum(), control.ceiling()).then_some(best)
    };
    if let Some(best) = at_ceiling(&starts) {
        return best;
    }
    let dived = dive(&prepared.model, &prepared.root, graph, &prepared.candidates);
    if dived.sum() > heaviest(starts.iter().copied()).sum() {
        offer_set(control, &dived);
    }
    for start in &mut starts {
        *start = heavier(*start, dived);
    }
    if let Some(best) = at_ceiling(&starts) {
        return best;
    }
    let hood = Neighborhood { model: &prepared.model, root: &prepared.root, candidates: &prepared.candidates };
    let mut best = Bits::EMPTY;
    for start in distinct(starts) {
        let improved = hood.improve_on(|| control.workers().unwrap_or(threads), || control.ceiling(), &start, true);
        if improved.sum() > start.sum() {
            offer_set(control, &improved);
        }
        best = heavier(best, improved);
        if reaches(best.sum(), control.ceiling()) {
            break;
        }
    }
    best
}

/// `first` unless `second` is strictly heavier.
fn heavier<const W: usize>(first: Bits<W>, second: Bits<W>) -> Bits<W> {
    if second.sum() > first.sum() {
        second
    } else {
        first
    }
}

/// The first of the heaviest of `sets` (the empty set, worth nothing, if there are none).
fn heaviest<const W: usize>(sets: impl IntoIterator<Item = Bits<W>>) -> Bits<W> {
    sets.into_iter().fold(Bits::EMPTY, heavier)
}

/// Offers the candidates of `seed` that can still be promoted in `prepared`, the game searched, as an incumbent to the
/// run of `control`, as the start of [`search`] treats its `seed`.  A run not yet started starts from it if it beats
/// the run's own start; a run that goes adopts it if it beats the incumbent.  Returns whether it became the
/// incumbent ([`Control::offer`]).
pub fn offer_seed<const W: usize>(prepared: &Prepared<W>, control: &Control, seed: &Bits<W>) -> bool {
    offer_set(control, &surviving(prepared, seed))
}

/// Offers `set`, a game that a finder ([`finder`]) found and claims is worth `claimed`, to the run of `control` as an
/// incumbent, after the checks that a checkpoint's incumbent passes on resuming ([`resume_point`]): it must be a set of
/// candidates that can be promoted from the root ([`Prepared::can_promote`]) and worth `claimed`.  It then goes the way a
/// worker's improvement does ([`Control::offer`]), so the run adopts it if it beats the incumbent, and ends if it reaches
/// the ceiling.  Returns whether it became the incumbent.
///
/// # Errors
///
/// Why the set was refused, with the incumbent left as it was.
fn offer_found<const W: usize>(
    prepared: &Prepared<W>,
    control: &Control,
    set: &Bits<W>,
    claimed: u64,
) -> Result<bool, String> {
    if !prepared.can_promote(set) {
        return Err("it is not a set of candidates that can be promoted in this game".into());
    }
    if set.sum() != claimed {
        return Err(format!("it is worth {}, not the {claimed} claimed", set.sum()));
    }
    Ok(offer_set(control, set))
}

/// The promotion sum of what survives of `seed` in `prepared`, the game searched: of the candidates of `seed` that can
/// still be promoted, from the largest down, the set that [`search`] and [`offer_seed`] make of a seed.  A search
/// seeded with `seed` starts from that set or a better one, so a ceiling that this sum reaches settles it at once.
pub fn seed_survivors_sum<const W: usize>(prepared: &Prepared<W>, seed: &Bits<W>) -> u64 {
    surviving(prepared, seed).sum()
}

/// `set` as the payload of an incumbent, which is how the searchers encode one ([`put_bits`]): a caller that starts the
/// engine itself ([`engine::run`] with a [`TaxSearcher`]) gives its starting incumbent this payload.
pub fn payload_of<const W: usize>(set: &Bits<W>) -> Vec<u8> {
    let mut payload = Vec::new();
    put_bits(&mut payload, set);
    payload
}

/// The promotion set that `payload` encodes, the inverse of [`payload_of`]: the incumbent of an engine's
/// [`Outcome`](engine::Outcome).  The payload must be exactly `8·W` bytes, as every searcher's incumbent is.
pub fn set_of_payload<const W: usize>(payload: &[u8]) -> Bits<W> {
    get_bits(payload)
}

/// Offers `set` to `control` as an incumbent, worth its sum ([`Control::offer`]).
fn offer_set<const W: usize>(control: &Control, set: &Bits<W>) -> bool {
    control.offer(set.sum(), &payload_of(set))
}

/// The incumbent of the run of `control` as a promotion set, if it has one: the run's, while it goes and after it
/// ended, else the best offered ([`Control::best`]), which includes the start of the search that has the control, as
/// soon as it is known ([`search`]).  The control must belong to a search of width `W`, whose payloads are promotion
/// sets (as [`offer_seed`] makes them).
pub fn best_so_far<const W: usize>(control: &Control) -> Option<Bits<W>> {
    control.best().map(|(_, payload)| set_of_payload(&payload))
}

/// Whether an incumbent worth `sum` reaches `ceiling`, the ceiling on the promotion sum, which proves it optimal.
fn reaches(sum: u64, ceiling: Option<u64>) -> bool {
    ceiling.is_some_and(|ceiling| sum >= ceiling)
}

/// The checkpoint `path` read and checked for resuming the search of `prepared`: its whole file (checksum
/// included), its header, and its incumbent, which must be a set of candidates that can be promoted from the
/// root and is worth the value stored with it.  Returns that set and what else the checkpoint holds.
///
/// Under a `floor` the incumbent is worth the floor or more.  A search that has found nothing above its floor holds
/// the floor itself and no set (an empty incumbent worth `floor`), which only a resume under that floor accepts.
fn resume_point<const W: usize>(
    prepared: &Prepared<W>,
    path: &Path,
    floor: Option<u64>,
) -> Result<(Bits<W>, engine::CheckpointHead), String> {
    let at = |e: String| format!("{}: {e}", path.display());
    let head = engine::read_checkpoint(path).map_err(|e| at(e.to_string()))?;
    let header = Header::<W>::decode(&head.header).map_err(at)?;
    if let Some(mismatch) = header.mismatch(&Header::of(prepared)) {
        return Err(at(mismatch));
    }
    if head.payload.len() != 8 * W {
        return Err(at(format!("its incumbent has {} bytes, not {}", head.payload.len(), 8 * W)));
    }
    let kept = set_of_payload(&head.payload);
    if !prepared.can_promote(&kept) {
        return Err(at("its incumbent cannot be promoted in this game".into()));
    }
    if let Some(floor) = floor.filter(|&floor| head.best < floor) {
        return Err(at(format!("its incumbent is worth {}, below the floor {floor}", head.best)));
    }
    if kept.sum() != head.best && !(floor == Some(head.best) && kept.is_empty()) {
        return Err(at(format!("its incumbent is worth {}, not the {} stored with it", kept.sum(), head.best)));
    }
    Ok((kept, head))
}

#[cfg(test)]
mod tests;
