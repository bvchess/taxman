//! Setting a game up for the search ([`prepare`], the Java's `SearchManager.create`): the model and the root
//! frame set, the preclusion table (tactic `preclusions`), the greedy incumbent, and (tactic `revise_freedoms`)
//! frame 0's revised freedoms, which can lower `max_total`.  [`prepare_for_resume`] does the same for a search
//! that a checkpoint will resume, taking frame 0's freedoms from it.

use super::header::Header;
use super::tactics::SearchTactics;
use crate::bits::Bits;
use crate::bounds::{self, Preclusions};
use crate::engine::{self, EngineConfig};
use crate::model::{FrameSet, Model};
use crate::numtheory::Tables;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// What revising frame 0's freedoms did (`UpperBoundEstimator.reviseFreedoms`).
///
/// Frame 0's freedoms only ever limit the promotions out of frame 1 (into frame 0).  The revision sets them to
/// a number of promotions the two-frame game of frames 0 and 1 is taken to allow.  The promotions out of frame 1
/// of any promotion set of the whole game are a promotion set of that two-frame game (frame 1 without the
/// promotions from frame 2 is only easier), so the revision is sound when the number bounds the *size* of every
/// promotion set of the two-frame game.  [`GreedyIsMax`](Revise::GreedyIsMax) does: it is `min(F₀, F₁)`, which
/// the freedoms imply anyway, so it changes nothing that matters.  [`Searched`](Revise::Searched) takes the size
/// of the best (max-sum) set the two-frame search returns, which bounds the sizes only by the unproven conjecture
/// of the tactic: that the max-sum set also has the max count.  Either way the number is a count of promotions
/// into frame 0, so it is at least 0 and at most the freedoms frame 0 was built with: revising only lowers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Revise {
    /// The tactic is off (also for a search resumed from the checkpoint of a game that was set up with it off).
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
    /// A limit or a stop signal stopped the two-frame game's search, so frame 0 could not be revised: the game
    /// is not ready to search, and [`search`](super::search) stops at once (as if a limit had stopped it, with no
    /// checkpoint).
    Stopped { nodes: u64 },
    /// Frame 0's freedoms were taken from a checkpoint of a game that was set up with the tactic on, and they are
    /// fewer than the ones frame 0 was built with: revised (the checkpoint does not say how).  (Builds before
    /// commit 7af9fea wrote this for every resumed search, whatever the tactics and the freedoms.)
    FromCheckpoint,
    /// The search resumed a checkpoint of a game that was set up with the tactic on, and frame 0's freedoms in it
    /// are the ones frame 0 was built with: whatever revising did then, it changed nothing the search uses.
    ResumedAsBuilt,
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

/// A game set up for [`search`](super::search).
pub struct Prepared<const W: usize> {
    /// The board size.
    pub n: u32,
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
    /// The most promotions a promotion set can have (`maxTotalPromotions`):
    /// `root.max_promotions_among(candidates)`, at least 0.  A count, not the ceiling on the promotion sum.
    pub max_total: u32,
    pub revise: Revise,
    pub timings: PrepareTimings,
}

/// Sets up the search for the game whose frames are `columns` (factors and moves per frame, bottom first) and
/// whose promotion candidates are `candidates`.  `threads` is used for the preclusion table and for the
/// two-frame search of `revise_freedoms`, which has no limits (see [`prepare_within`]).
pub fn prepare<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    candidates: &Bits<W>,
    tactics: SearchTactics,
    threads: usize,
) -> Result<Prepared<W>, String> {
    let engine = EngineConfig { threads, ..EngineConfig::default() };
    prepare_within(tables, columns, candidates, tactics, &engine)
}

/// [`prepare`] with `engine`'s threads, whose time and node limits (and stop signals) also bound the two-frame
/// search of `revise_freedoms`; nothing else of `engine` is used.  If they stop that search, `revise` is
/// [`Revise::Stopped`] and the game cannot be searched in this run.
pub fn prepare_within<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    candidates: &Bits<W>,
    tactics: SearchTactics,
    engine: &EngineConfig,
) -> Result<Prepared<W>, String> {
    let (model, root, build) = build_root::<W>(tables, columns)?;
    let mut prepared = prepare_state(tables.n, Arc::new(model), root, *candidates, tactics, engine, None)?;
    prepared.timings.build = build;
    Ok(prepared)
}

/// [`prepare`] for resuming the search checkpointed in `checkpoint`: frame 0's freedoms come from the
/// checkpoint's header instead of being revised again (which can take a search of its own), and the tactics
/// are the checkpoint's.  Fails, saying what differs, if the checkpoint was made for another board size,
/// width, candidates or frames, or by a build whose checkpoint headers had another format, or if its frame 0
/// has freedoms its search cannot have had (see `resumed_root`).
pub fn prepare_for_resume<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    candidates: &Bits<W>,
    checkpoint: &Path,
    threads: usize,
) -> Result<Prepared<W>, String> {
    let bytes = engine::read_checkpoint_header(checkpoint).map_err(|e| format!("{}: {e}", checkpoint.display()))?;
    let header = Header::<W>::decode(&bytes)?;
    let (model, root, build) = build_root::<W>(tables, columns)?;
    // The game itself must match before it is set up with what the checkpoint says of the search.
    let game = Header {
        n: tables.n,
        candidates: Header::digest_candidates(candidates),
        frames: Header::digest_frames(&root),
        ..header
    };
    if let Some(mismatch) = header.mismatch(&game) {
        return Err(mismatch);
    }
    let resumed = resumed_root(&header, &root)?;
    let engine = EngineConfig { threads, ..EngineConfig::default() };
    let mut prepared =
        prepare_state(tables.n, Arc::new(model), root, *candidates, header.tactics, &engine, Some(resumed))?;
    prepared.timings.build = build;
    // What is left to compare is max_total, which follows from frame 0's freedoms.
    match header.mismatch(&Header::of(&prepared)) {
        Some(mismatch) => Err(mismatch),
        None => Ok(prepared),
    }
}

/// The model and the root frame set of the game whose frames are `columns`, and the time it took to build them.
fn build_root<const W: usize>(tables: &Tables, columns: &[Vec<u32>]) -> Result<(Model, FrameSet<W>, Duration), String> {
    let t0 = Instant::now();
    let (model, root) = Model::build::<W>(tables, columns)?;
    Ok((model, root, t0.elapsed()))
}

/// The root of the search that `header`'s checkpoint holds, for the game whose root as built is `root`, and what
/// revising frame 0 did as far as the checkpoint tells.  Frame 0's freedoms are the checkpoint's: the ones frame
/// 0 was built with (and [`Revise::Off`] or [`Revise::ResumedAsBuilt`]), or with `revise_freedoms` on, fewer
/// ([`Revise::FromCheckpoint`]).  Any other number its search cannot have had: without the tactic nothing
/// changes them, and revising sets them to a number of promotions into frame 0, at least 0 and at most the
/// built freedoms; so it is refused, as a checkpoint of another search.
fn resumed_root<const W: usize>(header: &Header<W>, root: &FrameSet<W>) -> Result<(FrameSet<W>, Revise), String> {
    let built = root.frames().first().map(|f| f.freedoms);
    let on = header.tactics.revise_freedoms;
    match (header.frame0_freedoms, built) {
        (stored, built) if stored == built => Ok((*root, if on { Revise::ResumedAsBuilt } else { Revise::Off })),
        (Some(stored), Some(built)) if on && (0..built).contains(&stored) => {
            Ok((root.with_frame_zero_freedoms(stored), Revise::FromCheckpoint))
        }
        (stored, built) => {
            let why = if on {
                "revising them sets them to a number of promotions, from 0 to the built ones"
            } else {
                "revise_freedoms is off, so nothing changes them"
            };
            let number = |freedoms: Option<i32>| freedoms.map_or_else(|| "none".to_owned(), |f| f.to_string());
            Err(format!(
                "the checkpoint is of a search whose frame 0's freedoms are {}, not the {} frame 0 was built with \
                 ({why})",
                number(stored),
                number(built)
            ))
        }
    }
}

/// The setup after the frame set exists.  `resumed` (the root of a checkpoint's search, and what revising did as
/// far as it tells) replaces revising; `engine` as for [`prepare_within`].
fn prepare_state<const W: usize>(
    n: u32,
    model: Arc<Model>,
    root: FrameSet<W>,
    candidates: Bits<W>,
    tactics: SearchTactics,
    engine: &EngineConfig,
    resumed: Option<(FrameSet<W>, Revise)>,
) -> Result<Prepared<W>, String> {
    let mut timings = PrepareTimings::default();
    let t0 = Instant::now();
    let threads = engine.threads;
    let preclusions = tactics.preclusions.then(|| Preclusions::build(&model, &root, &candidates, threads));
    timings.preclusions = t0.elapsed();
    let t0 = Instant::now();
    let greedy = bounds::greedy(&model, &root, &candidates);
    timings.greedy = t0.elapsed();
    let t0 = Instant::now();
    let (revised, revise) = match resumed {
        Some(resumed) => resumed,
        None if tactics.revise_freedoms => revise_freedoms(n, &model, &root, &candidates, &greedy, tactics, engine)?,
        None => (root, Revise::Off),
    };
    timings.revise = t0.elapsed();
    let max_total = revised.max_promotions_among(&candidates).max(0) as u32;
    Ok(Prepared {
        n,
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

/// The Java's `UpperBoundEstimator.reviseFreedoms` (see [`Revise`] for what it rests on).  When the greedy
/// incumbent has fewer promotions than the freedoms allow and there are more than 2 frames, the promotions into
/// frame 0 may be fewer than its freedoms suggest.  The two-frame game {frame 0, frame 1} with frame 1's
/// candidates settles how many: by its greedy set if that reaches `min(freedoms of frames 0 and 1)`, else by
/// searching it, but only if that could change the full game's limit on the number of promotions.  Frame 0's
/// freedoms become that number.  The search runs within `engine`'s limits; if they stop it, nothing is revised
/// and the result says [`Revise::Stopped`].
fn revise_freedoms<const W: usize>(
    n: u32,
    model: &Arc<Model>,
    root: &FrameSet<W>,
    candidates: &Bits<W>,
    greedy: &Bits<W>,
    tactics: SearchTactics,
    engine: &EngineConfig,
) -> Result<(FrameSet<W>, Revise), String> {
    let full_max = root.max_promotions_among(candidates);
    if i64::from(greedy.len()) == i64::from(full_max) || root.len() <= 2 {
        return Ok((*root, Revise::NotNeeded));
    }
    let pair = root.front_pair();
    let (f0, f1) = (&pair.frames()[0], &pair.frames()[1]);
    let best_possible = f0.freedoms.min(f1.freedoms);
    let small = prepare_state(n, model.clone(), pair, *candidates & f1.factors, tactics, engine, None)?;
    let small_greedy = small.greedy.len();
    if i64::from(small_greedy) == i64::from(best_possible) {
        return Ok((root.with_frame_zero_freedoms(small_greedy as i32), Revise::GreedyIsMax(small_greedy)));
    }
    if root.with_frame_zero_freedoms(small_greedy as i32).max_promotions_among(candidates) == full_max {
        return Ok((*root, Revise::Unchanged));
    }
    let cfg = EngineConfig {
        threads: engine.threads,
        time_limit: engine.time_limit,
        node_limit: engine.node_limit,
        label: "revise".into(),
        ..EngineConfig::default()
    };
    let found = super::search(&small, &cfg, None, None, false).map_err(|e| format!("revise search: {e}"))?;
    let nodes = found.stats.nodes;
    if !found.completed {
        return Ok((*root, Revise::Stopped { nodes }));
    }
    let size = found.promotions.len();
    Ok((root.with_frame_zero_freedoms(size as i32), Revise::Searched { size, nodes }))
}
