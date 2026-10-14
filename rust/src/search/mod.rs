//! The branch-and-bound search for the promotion set with the largest sum, run on the parallel [`engine`].
//!
//! `DESIGN.md`, section 4 ("The search"), explains the algorithm: nodes, steps and units, the matching bound,
//! conflicts, forced members and nogoods, the pseudocode of "The default step", and why the search is exact.  The
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
//!   the matching check, or the verdict the node's check left when it was made as a child, if `need` is the same.
//! * `choose_branch` (steps 3 to 5): the member of the conflict to branch on (with strong branching,
//!   `refine::strongest`), or with `fix_forced` the forced members, which the child promotes together.
//! * `make_child` (steps 6 and 7): the child's state and candidates, without the last member of a conflict known
//!   along the path or of a nogood.
//! * `check_child` (steps 8 and 9): the child's own check; then it is offered as the incumbent and pushed.
//!
//! The matching check of a node or a child is `Stepper::check` with `refine::Refiner::top`: the matching bound
//! ([`crate::matchbound`]), the forced members, the conflict of the heaviest matching (the smallest the nogood table
//! knows, [`crate::model::Nogoods`], else [`Model::conflict_using`](crate::model::Model::conflict_using)), and the
//! refinement; then, where they leave it open, the Lagrangian bound over the nogoods and the forced members it
//! finds (`lagrange::Lagrangian::check`).  The tactics that switch these pieces are the fields of
//! [`SearchTactics`].
//!
//! # Where the arguments are
//!
//! * The sum of the `max_add` largest candidates bounds every completion by itself, not as a shortcut for the
//!   matching bound: no completion has more promotions than the freedoms allow (see [`crate::model::FrameSet`]),
//!   nor larger ones.
//! * The matching bound: [`crate::matchbound`].  Conflicts:
//!   [`Model::conflict_using`](crate::model::Model::conflict_using); nogoods: [`crate::model::Nogoods`];
//!   refinement and forced members: `refine::Refiner` and `refine::strongest`; the rule-outs of children:
//!   `Stepper::make_child`; the Lagrangian bound and its forced members: [`crate::matchbound::lagrange`] and
//!   [`lagrange`].
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
//! * `refine`: the matching check (`Refiner`) and strong branching (`strongest`).
//! * [`lagrange`]: the Lagrangian bound in the search: where it is tried, its forced members, its refits.
//! * `header`: the checkpoint's header.
//! * `diagnostics`: [`Diagnostics`], for tuning.
//! * here: [`search`], which starts or resumes the engine on a prepared game.

mod diagnostics;
mod header;
mod incumbent;
pub mod lagrange;
mod refine;
mod searcher;
mod setup;
mod tactics;

pub use diagnostics::{cut, Diagnostics, GAP_BUCKETS};
pub use incumbent::Neighbourhood;
pub use searcher::{stat, Found, Gf2Branching, SearchSpace, TaxSearcher, TaxUnit};
pub use setup::{prepare, prepare_for_resume, problem_of, PrepareTimings, Prepared, Problem, ProblemSize};
pub use tactics::SearchTactics;

use crate::bits::Bits;
use crate::engine::{self, Counters, EngineConfig, Start, Stats};
use crate::matchbound::MatchGraph;
use crate::model::Scratch;
use header::Header;
use incumbent::{dive, surviving};
use searcher::{get_bits, put_bits};
use std::io;
use std::path::Path;
use std::sync::{Mutex, PoisonError};
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
    /// The search finished (or reached the cap); false when an engine limit stopped it.
    pub completed: bool,
    /// The incumbent reached the cap, which proves it optimal.
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
    /// by the local search.
    pub start: Bits<W>,
}

/// Searches for the promotion set with the largest sum.
///
/// The incumbent starts as the best of the greedy set, what survives of `seed` (its candidates that can still
/// be promoted, from the largest down) and the dive (the root's matching rounded to a set that can be
/// promoted), improved by a local search ([`Neighbourhood::improve`]).  `cap` is a proven ceiling
/// on the sum; reaching it ends the search, also before it starts.  With `resume`, the search continues from
/// the checkpoint configured in `cfg` (the `prepared` game must have been set up by [`prepare_for_resume`] or
/// identically): its header must be this game's, and its incumbent a promotable set of candidates worth what
/// the checkpoint says.  Otherwise it starts fresh.
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
    let match_graph = MatchGraph::new(&prepared.model, &prepared.root);
    let mut incumbent = prepared.greedy;
    if let Some(seed) = seed {
        let kept = surviving(prepared, seed);
        if kept.sum() > incumbent.sum() {
            incumbent = kept;
        }
    }
    let dived = dive(&prepared.model, &prepared.root, &match_graph, &prepared.candidates);
    if dived.sum() > incumbent.sum() {
        incumbent = dived;
    }
    let hood = Neighbourhood { model: &prepared.model, root: &prepared.root, candidates: &prepared.candidates };
    incumbent = hood.improve(&mut Scratch::new(), &incumbent, true);
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
    if cap.is_some_and(|cap| best.sum() >= cap) {
        // A resumed search that reaches the cap at once is finished like any other.
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

    let space = SearchSpace { nogood_period, diagnostics, began: t0, ..SearchSpace::new(prepared, &match_graph, cap) };
    let start = match checkpoint.filter(|_| resume) {
        Some(path) => Start::Resume(path.to_path_buf()),
        None => {
            let mut payload = Vec::new();
            put_bits(&mut payload, &incumbent);
            let root = TaxUnit { promoted: Bits::EMPTY, cands: prepared.candidates };
            Start::Fresh { units: vec![root], best: incumbent.sum(), payload }
        }
    };
    let header = Header::of(prepared).encode();
    let engine_start = Instant::now();
    let outcome = engine::run(cfg, start, &header, |_| TaxSearcher::new(&space))?;
    // The engine's elapsed time includes the runs before a resume; what it counts beyond this call is theirs.
    let earlier_runs = outcome.elapsed.saturating_sub(engine_start.elapsed());
    let promotions = get_bits(&outcome.payload);
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
    let kept = get_bits(&head.payload);
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
