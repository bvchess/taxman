//! The branch-and-bound search for the promotion set with the largest sum, run on the parallel [`engine`].
//!
//! `DESIGN.md`, section 4 ("The search"), explains the algorithm: nodes, steps and units, the GF(2) bound (the
//! default) and the matching bound, conflicts, forced members and nogoods, the pseudocode of "The default step", and
//! why the search is exact.  The
//! docs here say where each piece of it is, and carry the arguments next to the code they justify.
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
//! has four phases, which take the steps of DESIGN.md's "The default step" in order:
//!
//! * `check_node` (steps 1 and 2): does the node's loop go on?  The sum of its `max_add` largest candidates, then
//!   the node's bound check, or the verdict the node's check left when it was made as a child, if `need` is the
//!   same.
//!   With the tactic `symmetry`, the rule of [`symmetry`] comes between them (`Stepper::apply_symmetry`): it cuts
//!   the node, or drops candidates from its loop, and the sum of the largest is checked again for what is left.
//! * `choose_branch` (steps 3 to 5): the member of the conflict to branch on (with strong branching,
//!   `refine::strongest`), or with `fix_forced` the forced members, which the child promotes together.
//! * `make_child` (steps 6 and 7): the child's state and candidates, without the last member of a conflict known
//!   along the path or of a nogood, and with `symmetry` without the candidates the rule drops.
//! * `check_child` (steps 8 and 9): the child's own check, which with `symmetry` begins with the rule's test of the
//!   child (cut if no completion keeps the inequalities); then it is offered as the incumbent and pushed.
//!
//! The bound check of a node or a child is `Stepper::check`.  With the tactic `gf2`, the default, it is `check_gf2`:
//! the GF(2) bound ([`crate::gf2`]), the forced members, and the conflict of the heaviest set (the smallest the
//! nogood table knows, [`crate::model::Nogoods`], else
//! [`FrameGraph::conflict_using`](crate::model::FrameGraph::conflict_using)).  Without it, on the matching path, it is
//! `refine::Refiner::top`: the matching bound ([`crate::matchbound`]), the forced members, the conflict of the
//! heaviest matching, and the refinement.  With the tactic `lagrange`, where they leave it open, the Lagrangian bound
//! over the nogoods and the forced members it finds follow (`lagrange::Lagrangian::check`).  The tactics that switch
//! these pieces are the fields of [`SearchTactics`].
//!
//! # Where the arguments are
//!
//! * The sum of the `max_add` largest candidates bounds every completion by itself, not as a shortcut for the
//!   other bounds: no completion has more promotions than the freedoms allow (see [`crate::model::FrameSet`]),
//!   nor larger ones.
//! * The bounds: [`crate::gf2`] (default) and [`crate::matchbound`].  Conflicts:
//!   [`FrameGraph::conflict_using`](crate::model::FrameGraph::conflict_using); nogoods: [`crate::model::Nogoods`];
//!   refinement and forced members: `refine::Refiner` and `refine::strongest`; the rule-outs of children:
//!   `Stepper::make_child`; the Lagrangian bound and its forced members: [`crate::matchbound::lagrange`] and
//!   [`lagrange`].
//! * The rule of the tactic `symmetry` ([`symmetry`]): every heaviest promotion set keeps the inequality
//!   `c_p ≤ c_q` of each symmetric pair `(p, q)` of the root (heuristics/THEORY.md, Theorem 13), so a node none of
//!   whose completions keeps them all holds no heaviest set, and neither does a completion that holds a candidate
//!   no such completion holds: the node is cut and the candidate dropped, whatever bound the search uses.  The
//!   other prunings hold for the completions that keep the inequalities as for all (bounds above every completion,
//!   members every completion that beats the incumbent promotes, sets that cannot be promoted).  The rule is a
//!   function of the node's promotions and candidates, so it changes which tree is searched, not the answer of a
//!   search from the root.  A search of a node below the root finds the best of its completions that keep the
//!   inequalities, which can be lighter than its best completion (THEORY.md, last paragraph of "Pruning with the
//!   inequalities").
//! * What a searcher remembers (the conflicts known along the path, the nogoods, the frames' last conflicts, the
//!   matching levels kept by strong branching, a child's verdict) are facts that hold where they are used, or
//!   caches of them: they change which tree is searched, never the answer.  So a search's tree depends on its
//!   threads and interruptions; its answer does not.
//!
//! # What is where
//!
//! * `tactics`: [`SearchTactics`], and how a checkpoint records them.
//! * `setup`: [`prepare`]: the root, the greedy incumbent, `max_total`; the [`Problem`] a setup poses.
//! * `incumbent`: the starting incumbent (seed, dive, local search).
//! * `searcher`: [`TaxSearcher`], its stack and its steps; [`SearchSpace`], [`TaxUnit`], the counters [`stat`].
//! * `refine`: the matching path's check (`Refiner`) and strong branching on it (`strongest`).
//! * [`lagrange`]: the Lagrangian bound in the search: where it is tried, its forced members, its refits.
//! * `header`: the checkpoint's header.
//! * `diagnostics`: [`Diagnostics`], for tuning.
//! * [`symmetry`]: the root's prime exchanges (symmetric pairs of primes) and the rule they give the search.
//! * here: [`search`], which starts or resumes the engine on a prepared game; [`start_incumbent`], the incumbent it
//!   starts from, which the hunt (`sacrifice::hunt`) also makes without a search; [`offer_seed`], [`best_so_far`] and
//!   [`seed_survivors_sum`], for a caller that steers searches through their [`Control`] (`EngineConfig::control`).

mod diagnostics;
mod header;
mod incumbent;
pub mod lagrange;
mod refine;
mod searcher;
mod setup;
pub mod symmetry;
mod tactics;

pub use diagnostics::{cut, Diagnostics, GAP_BUCKETS};
pub use incumbent::Neighbourhood;
pub use searcher::{stat, Found, Gf2Branching, SearchSpace, TaxSearcher, TaxUnit};
pub use setup::{
    extended_words, prepare, prepare_extended, prepare_extended_for_resume, prepare_for_resume, problem_of,
    PrepareTimings, Prepared, Problem, ProblemSize,
};
pub use tactics::SearchTactics;

use crate::bits::Bits;
use crate::engine::{self, Control, Counters, EngineConfig, Start, Stats};
use crate::matchbound::MatchGraph;
use crate::model::Scratch;
use header::Header;
use incumbent::{dive, surviving};
use searcher::{get_bits, put_bits};
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// With the tactic `forget_nogoods`, a searcher forgets its nogoods after each this many steps.  Every interval
/// from 10,000 to 100,000 steps was about as good on one thread (22–26% less time than never forgetting on hard
/// games near n = 1200, games of fewer steps unchanged); see docs/EXPERIMENTS.md, "Forgetting nogoods".
pub const NOGOOD_PERIOD: u64 = 50_000;

/// The outcome of [`search`].
#[derive(Clone, Debug)]
pub struct SearchResult<const W: usize> {
    /// The best promotion set found (the optimum when `completed`).
    pub promotions: Bits<W>,
    pub sum: u64,
    /// The search finished (or reached the ceiling); false when an engine limit or a stop stopped it.
    pub completed: bool,
    /// The incumbent reached the ceiling (`cap`, or one the control was given), which proves it optimal.
    pub reached_cap: bool,
    /// Engine statistics; see [`stat`] for `extra`.
    pub stats: Stats,
    pub counters: Counters,
    /// Wall time of this call.
    pub time: Duration,
    /// Wall time of the search over all its runs: the engine's time in the runs before a resume (each counted up
    /// to its last checkpoint) plus this call's.
    pub time_all_runs: Duration,
    /// When this call found its best incumbent (`None` if it found none better than the starting one).
    pub found: Option<Found>,
    /// The incumbent the search started from: the best of the greedy set, the seed and the matching dive, improved
    /// by the local search (the dive and the local search are skipped when the set before them reaches the
    /// ceiling, and the local search ends when its set does).
    pub start: Bits<W>,
}

/// Searches for the promotion set with the largest sum.
///
/// The incumbent starts as the best of the greedy set, what survives of `seed` (its candidates that can still
/// be promoted, from the largest down) and the dive (the root's matching rounded to a set that can be
/// promoted), improved by a local search ([`Neighbourhood::improve_on`] on the run's threads, `cfg.threads` or the
/// control's workers, with the answer of [`Neighbourhood::improve`]).  The start is offered to the run's control as
/// soon as it is known and each time it improves (after the best of the greedy set and the seed's survivors, after the
/// dive and after the local search, where they improve it), so that [`best_so_far`] has it before the engine runs.
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
    let period = prepared.tactics.forget_nogoods.then_some(NOGOOD_PERIOD);
    search_forgetting(prepared, cfg, seed, cap, resume, diagnostics, period)
}

/// [`search_with`], the searchers forgetting their nogoods after each `period` steps (if any) whatever the tactic
/// `forget_nogoods` says, so that tests can make them forget often.
fn search_forgetting<const W: usize>(
    prepared: &Prepared<W>,
    cfg: &EngineConfig,
    seed: Option<&Bits<W>>,
    cap: Option<u64>,
    resume: bool,
    diagnostics: Option<&Mutex<Diagnostics>>,
    nogood_period: Option<u64>,
) -> io::Result<SearchResult<W>> {
    let t0 = Instant::now();
    let control = cfg.control.clone().unwrap_or_else(Control::new);
    if let Some(cap) = cap {
        control.set_ceiling(cap);
    }
    let cfg = &EngineConfig { control: Some(Arc::clone(&control)), ..cfg.clone() };
    let match_graph = MatchGraph::new(&prepared.model, &prepared.root);
    let incumbent = start_incumbent(prepared, &match_graph, &control, cfg.threads, seed);
    let checkpoint = cfg.checkpoint.as_ref().map(|(path, _)| path.as_path());
    let resumed = if resume {
        let path =
            checkpoint.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "resuming needs a checkpoint"))?;
        Some(resume_point(prepared, path).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?)
    } else {
        None
    };
    // What the search would start from, and the totals of the runs before this one.
    let (best, before, earlier) = match &resumed {
        Some((kept, head)) => (if kept.sum() >= incumbent.sum() { *kept } else { incumbent }, head.stats, head.elapsed),
        None => (incumbent, Stats::default(), Duration::ZERO),
    };
    if reaches(&best, control.ceiling()) {
        // A resumed search that reaches the ceiling at once is finished like any other.
        if let (true, Some(path)) = (resume && !cfg.keep_completed_checkpoint, checkpoint) {
            engine::remove_finished_checkpoint(path);
        }
        return Ok(SearchResult {
            promotions: best,
            sum: best.sum(),
            completed: true,
            reached_cap: true,
            stats: before,
            counters: Counters::default(),
            time: t0.elapsed(),
            time_all_runs: earlier + t0.elapsed(),
            found: None,
            start: incumbent,
        });
    }

    let space = SearchSpace { nogood_period, diagnostics, began: t0, ..SearchSpace::new(prepared, &match_graph) };
    let start = match checkpoint.filter(|_| resume) {
        Some(path) => Start::Resume(path.to_path_buf()),
        None => {
            let root = TaxUnit { promoted: Bits::EMPTY, cands: prepared.candidates };
            Start::Fresh { units: vec![root], best: incumbent.sum(), payload: payload_of(&incumbent) }
        }
    };
    let header = Header::of(prepared).encode();
    let engine_start = Instant::now();
    let outcome = engine::run(cfg, start, &header, |_| TaxSearcher::new(&space))?;
    // The engine's elapsed time includes the runs before a resume; what it counts beyond this call is theirs.
    let earlier_runs = outcome.elapsed.saturating_sub(engine_start.elapsed());
    let promotions = set_of_payload(&outcome.payload);
    assert_eq!(promotions.sum(), outcome.best, "the incumbent's payload does not match its value");
    let time = t0.elapsed();
    Ok(SearchResult {
        promotions,
        sum: outcome.best,
        completed: outcome.completed,
        reached_cap: outcome.optimal_declared,
        stats: outcome.stats,
        counters: outcome.counters,
        time,
        time_all_runs: time + earlier_runs,
        found: space.found.into_inner().unwrap_or_else(PoisonError::into_inner),
        start: incumbent,
    })
}

/// The incumbent a search starts from, made as [`search`] makes it (its documentation has the steps): the best of the
/// greedy set and what survives of `seed`, then the dive, then the local search on `threads` threads (the control's
/// workers if it has them).  `graph` is the matching graph of `prepared`.
///
/// The incumbent is offered to `control` as soon as it is known and each time it improves, so that a caller reading the
/// control ([`best_so_far`]) has it while the start is still being computed, not only once the engine runs.  The run
/// starts from exactly what it started from without the offers: the engine takes the better of its `Start`'s incumbent
/// and the best offered to the control, strictly better only, and the start's final incumbent is at least every one
/// offered, so it keeps its own, and the trees do not change.  (A resumed run's `Start` is its checkpoint, whose
/// incumbent is at least the start of the run that wrote it: the same, with the same seed, so it keeps its own too;
/// one that a start beats is replaced by it, as `best` in [`search`] counts it.)
///
/// A start that reaches the control's ceiling cannot be beaten, so the dive and the local search are skipped: they could
/// not change it if the ceiling holds, as the proof by the ceiling assumes (and `solver::conclude` rejects any game
/// above it).  The dive is taken only when strictly heavier, and the local search keeps only strictly heavier sets,
/// after a first fill that can only add candidates; no promotable set is heavier than the ceiling.  The ceiling is
/// read when each is due, and by the local search before each pass: the control may be given a lower one meanwhile.
pub fn start_incumbent<const W: usize>(
    prepared: &Prepared<W>,
    graph: &MatchGraph,
    control: &Control,
    threads: usize,
    seed: Option<&Bits<W>>,
) -> Bits<W> {
    let mut incumbent = prepared.greedy;
    if let Some(seed) = seed {
        let kept = surviving(prepared, seed);
        if kept.sum() > incumbent.sum() {
            incumbent = kept;
        }
    }
    offer_set(control, &incumbent);
    if !reaches(&incumbent, control.ceiling()) {
        let dived = dive(&prepared.model, &prepared.root, graph, &prepared.candidates);
        if dived.sum() > incumbent.sum() {
            incumbent = dived;
            offer_set(control, &incumbent);
        }
    }
    if !reaches(&incumbent, control.ceiling()) {
        let hood = Neighbourhood { model: &prepared.model, root: &prepared.root, candidates: &prepared.candidates };
        let improved = hood.improve_on(|| control.workers().unwrap_or(threads), || control.ceiling(), &incumbent, true);
        if improved.sum() > incumbent.sum() {
            offer_set(control, &improved);
        }
        incumbent = improved;
    }
    incumbent
}

/// Offers the candidates of `seed` that can still be promoted in `prepared`, the game searched, as an incumbent to the
/// run of `control`, as the start of [`search`] treats its `seed`.  A run not yet started starts from it if it beats
/// the run's own start; a run that goes adopts it if it beats the incumbent.  Returns whether it became the
/// incumbent ([`Control::offer`]).
pub fn offer_seed<const W: usize>(prepared: &Prepared<W>, control: &Control, seed: &Bits<W>) -> bool {
    offer_set(control, &surviving(prepared, seed))
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

/// Whether `set` is worth `ceiling`, the ceiling on the promotion sum, or more, which proves it optimal.
fn reaches<const W: usize>(set: &Bits<W>, ceiling: Option<u64>) -> bool {
    ceiling.is_some_and(|ceiling| set.sum() >= ceiling)
}

/// The checkpoint `path` read and checked for resuming the search of `prepared`: its whole file (checksum
/// included), its header, and its incumbent, which must be a set of candidates that can be promoted from the
/// root and is worth the value stored with it.  Returns that set and what else the checkpoint holds.
fn resume_point<const W: usize>(
    prepared: &Prepared<W>,
    path: &Path,
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
    let fits = kept.is_subset(&prepared.candidates)
        && prepared.model.promote_all_using(&mut Scratch::new(), &prepared.root, &kept).is_some();
    if !fits {
        return Err(at("its incumbent cannot be promoted in this game".into()));
    }
    if kept.sum() != head.best {
        return Err(at(format!("its incumbent is worth {}, not the {} stored with it", kept.sum(), head.best)));
    }
    Ok((kept, head))
}

#[cfg(test)]
mod tests;
