//! The branch-and-bound search for the promotion set with the largest sum, run on the parallel [`engine`].
//!
//! # What it searches
//!
//! A game's frames and promotion candidates come from the frame builder ([`crate::frames`]); the frame set they
//! make is the search's *root*.  The search finds the set of candidates with the largest sum that can be promoted
//! together from the root (see [`crate::model`]), with at most `max_total` members.  The game's score is
//! `in_the_bag` (what the game with no promotions scores) plus that sum.
//!
//! A search *node* is a set `P` of promotions, the state they reach (the root with `P` promoted, in any order),
//! and the candidates `C` the node's loop has still to try.  A *completion* of the node is a set `F ⊆ C` that can
//! be promoted from its state.  The *incumbent* is the best promotion set found so far, worth `best`; a node is
//! worth exploring only while some completion could add `need = best + 1 − sum(P)`.
//!
//! The search is depth-first, on the engine's worker threads.  Each takes a *unit* of work ([`TaxUnit`], just
//! `P` and `C`: the state is rebuilt from `P` in one pass) and runs a loop of *steps* on its own stack of nodes
//! ([`TaxSearcher`]).  A step is one turn of a node's loop: it makes one child, or ends the loop.  The "nodes"
//! that the engine, the results and the progress lines count are steps.
//!
//! # A step, with the default tactics
//!
//! ```text
//! step of the node (P, C) on top of the stack (its matching bound on top of the matching stack):
//!   need = best + 1 − sum(P)                      (`best` is read once, for the whole step)
//!
//!   1. check the node: does its loop go on?                                         (Stepper::check_node)
//!        C = ∅, or the max_add largest members of C sum to less than need         → end the loop
//!        the node's first step, at the need its check was made with               → that check's K and F
//!        else the matching check of (P, C) at need (below)                         → end the loop, or K and F
//!   2. choose the branch n                                                       (Stepper::choose_branch)
//!        forced members F found:   n = max F, and the child promotes all of F; the loop ends after it
//!        else, K of 2 or more:     strong branching: the matching bound without each member x of K, solved;
//!                                  members that bring it below need are forced (as above; all of K forced
//!                                  → end the loop); else n = the member that lowers it most
//!        else:                     n = the member of K (the largest candidate if there is no K)
//!        C ← C − n
//!   3. make the child (P ∪ A, C′), A = {n} or F                                    (Stepper::make_child)
//!        promote A from the node's state                                          → fails: next step
//!        max_next = the most promotions the child can add among C − A
//!        C′ = C − A narrowed to the frames with freedoms left, minus every candidate that would complete a
//!             conflict known along the path, or a nogood
//!   4. check the child at need′ = best + 1 − sum(P ∪ A)                          (Stepper::check_child)
//!        the max_next largest members of C′ sum to less than need′, or the matching check of the child
//!        (pushed on the matching stack) at need′ prunes it                       → next step
//!        offer P ∪ A as the incumbent; unless C′ = ∅, push the child, keeping its check's verdict for its
//!        first step
//!
//! the matching check of (P, C) at need                                    (Stepper::check, Refiner::top)
//!   the matching bound: the heaviest matching promotes Q* ⊆ C; sum(Q*) < need        → prune
//!   F = the members x of Q* whose bound without x (from the potentials) is below need
//!   K = a conflict of Q*: the smallest the nogood table knows within P ∪ Q*, else one found in a frame that
//!       fails with Q* promoted (and learned as a nogood); one the table knows inside F          → prune
//!   no conflict: Q* can be promoted, so it is the best completion          → offer P ∪ Q*, and prune
//!   K ⊆ F                                                                                        → prune
//!   F = ∅: refine one level: every branch of K (k₁ out; k₁ in and k₂ out; …) below need          → prune
//! ```
//!
//! The checks share what the searcher has learned: the conflicts found along the path (`known`), and its table
//! of nogoods (conflicts of the root), which it forgets every [`NOGOOD_PERIOD`] of its steps.  The tactics that
//! switch these pieces are the fields of [`SearchTactics`].
//!
//! # The Java's loop (`match_bound` off)
//!
//! Without the matching bound the search is the Java's `BranchExplorer.explore`, made lazy (each child is explored
//! as soon as it is made): `blameGame` ([`crate::bounds`]) bounds the node and each child, the children's
//! candidate lists keep the Java's promotability checks, and a node branches on its largest candidate.  The
//! default search does not compute `blameGame` at all: the matching bound replaces it (it never pruned where the
//! matching bound did not), it is not combined with it.
//!
//! ```text
//! loop while C ≠ ∅:
//!     if sum(P) + blameGame(S, C, max_add) < best + 1: break
//!     n = max C;  C ← C − n
//!     S′ = S with n promoted, else continue
//!     C′ = new_candidate_list(n, S′, C, P)                 narrowed, preclusions, promotability checks
//!     if sum(P) + n + blameGame(S′, C′, max_next) ≥ best + 1:
//!         offer P + n;  explore (P + n, C′) if C′ ≠ ∅
//! ```
//!
//! (The Java breaks only when the bound is below `best`; no completion can beat `best` when it is equal.)
//!
//! # Why the default search is exact
//!
//! Every pruning drops only completions that cannot beat the incumbent:
//!
//! * The sum of the `max_add` largest candidates bounds every completion: none has more than `max_add`
//!   promotions (counted from the freedoms, see [`crate::model::FrameSet`]), nor larger ones.  It is a bound of
//!   its own, tried first because it is cheap; with frame 0's freedoms revised, the count rests on
//!   `revise_freedoms`.
//! * The matching bound is a relaxation of promotability ([`crate::matchbound`]).
//! * Every completion leaves out a member of every conflict of the node ([`crate::model`], `conflict`), which
//!   conflict branching, refinement (`refine::Refiner`) and the rule-outs of children rest on; a nogood minus a
//!   node's promotions is a conflict of the node ([`crate::model::Nogoods`]).
//! * Forced members are in every completion that reaches `need` (`refine::Refiner`, `refine::strongest`), so the
//!   child that promotes all of them keeps every such completion of the node, and a conflict all of whose
//!   members are forced leaves none.
//! * A heaviest matching that can be promoted is the best completion; it is offered before the node is dropped.
//! * Every check compares with `need = best + 1 − sum(P)`, so a node is dropped only when nothing in it beats the
//!   incumbent.
//! * What a searcher remembers (the conflicts known along the path, the nogoods, the frames' last conflicts, the
//!   matching levels kept by strong branching, a child's verdict) are facts that hold where they are used, or
//!   caches of them: they change which tree is searched, never the answer.  So a search's tree depends on its
//!   threads and interruptions; its answer does not.
//!
//! # What is where
//!
//! * `tactics`: [`SearchTactics`] and [`Branching`], and how a checkpoint records them.
//! * `setup`: [`prepare`]: the root, the greedy incumbent, `revise_freedoms`, `max_total`.
//! * `incumbent`: the starting incumbent (seed, dive, local search).
//! * `searcher`: [`TaxSearcher`], its stack and its steps; [`SearchSpace`], [`TaxUnit`], the counters [`stat`].
//! * `refine`: the matching check (`Refiner`) and strong branching (`strongest`).
//! * `header`: the checkpoint's header.
//! * `diagnostics`: [`Diagnostics`], for tuning.
//! * here: [`search`], which starts or resumes the engine on a prepared game.

mod diagnostics;
mod header;
mod incumbent;
mod refine;
mod searcher;
mod setup;
mod tactics;

pub use diagnostics::{cut, Diagnostics, GAP_BUCKETS};
pub use header::checkpoint_width;
pub use incumbent::Neighbourhood;
pub use searcher::{stat, Found, SearchSpace, TaxSearcher, TaxUnit};
pub use setup::{prepare, prepare_for_resume, prepare_within, PrepareTimings, Prepared, Revise};
pub use tactics::{Branching, SearchTactics};

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
/// games near n = 1200, games of fewer steps unchanged); see DESIGN.md, "Forgetting nogoods".
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
    /// The incumbent the search started from: the best of the greedy set, the seed and (with `match_bound`) the
    /// matching dive, improved by the local search.
    pub start: Bits<W>,
}

/// Searches for the promotion set with the largest sum.
///
/// The incumbent starts as the best of the greedy set, what survives of `seed` (its candidates that can still
/// be promoted, from the largest down) and, with `match_bound`, the dive (the root's matching rounded to a set
/// that can be promoted), improved by a local search ([`Neighbourhood::improve`]).  `cap` is a proven ceiling
/// on the sum; reaching it ends the search, also before it starts.  With `resume`, the search continues from
/// the checkpoint configured in `cfg` (the `prepared` game must have been set up by [`prepare_for_resume`] or
/// identically): its header must be this game's, and its incumbent a promotable set of candidates worth what
/// the checkpoint says.  Otherwise it starts fresh, unless `prepared` is [`Revise::Stopped`]: then it stops at
/// once, incomplete, without a checkpoint.
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
    let reached_cap = cap.is_some_and(|cap| best.sum() >= cap);
    let stopped = !resume && matches!(prepared.revise, Revise::Stopped { .. });
    if reached_cap || stopped {
        // A resumed search that reaches the cap at once is finished like any other.
        if let (true, Some(path)) = (reached_cap && resume && !cfg.keep_completed_checkpoint, checkpoint) {
            engine::remove_finished_checkpoint(path);
        }
        return Ok(SearchResult {
            promotions: best,
            sum: best.sum(),
            completed: reached_cap,
            reached_cap,
            stats: before,
            counters: Counters::default(),
            time: t0.elapsed(),
            time_all_runs: earlier + t0.elapsed(),
            found: None,
            start: incumbent,
        });
    }

    let space =
        SearchSpace { nogood_period, diagnostics, began: t0, ..SearchSpace::new(prepared, match_graph.as_ref(), cap) };
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
