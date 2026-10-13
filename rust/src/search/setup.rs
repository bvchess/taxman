//! Setting a game up for the search ([`prepare`], the Java's `SearchManager.create`): the model and the root
//! frame set, the preclusion table (tactic `preclusions`), the greedy incumbent, and (tactic `revise_freedoms`)
//! frame 0's revised freedoms, which can lower `max_total`.  [`prepare_for_resume`] does the same for a search
//! that a checkpoint will resume, taking frame 0's freedoms from it.
//!
//! The search problem a setup poses, in a canonical form that two games can be compared by, is a [`Problem`]
//! ([`Prepared::problem`]); [`problem_of`] rebuilds it from what a results line records, without preparing a search.

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
///
/// A results line records the value in its `Debug` form (`setup.revise`), which [`Revise::parse`] reads back.
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

impl Revise {
    /// Whether frame 0's freedoms in the search come from the tactic, so that the search rests on its conjecture:
    /// [`GreedyIsMax`](Revise::GreedyIsMax), [`Searched`](Revise::Searched) and
    /// [`FromCheckpoint`](Revise::FromCheckpoint), even when the number is the one frame 0 was built with.
    pub fn sets_frame_zero(self) -> bool {
        matches!(self, Revise::GreedyIsMax(_) | Revise::Searched { .. } | Revise::FromCheckpoint)
    }

    /// Frame 0's freedoms in the search's root, as far as this value tells: `Ok(None)` for the ones frame 0 was
    /// built with, `Ok(Some(k))` for the `k` that revising set, and an error for
    /// [`FromCheckpoint`](Revise::FromCheckpoint), which does not record the number.  (A
    /// [`Stopped`](Revise::Stopped) revision changed nothing, so its root is the one built.)
    pub fn frame_zero(self) -> Result<Option<u32>, String> {
        match self {
            Revise::Off | Revise::NotNeeded | Revise::Unchanged | Revise::Stopped { .. } | Revise::ResumedAsBuilt => {
                Ok(None)
            }
            Revise::GreedyIsMax(size) | Revise::Searched { size, .. } => Ok(Some(size)),
            Revise::FromCheckpoint => Err("frame 0's freedoms came from a checkpoint, which does not say what they \
                                           were, so the search cannot be set up again from the line"
                .into()),
        }
    }

    /// The value whose `Debug` form is `text`, which is how a results line records it (`setup.revise`, say
    /// `"Searched { size: 3, nodes: 9 }"`); `None` if `text` is not exactly that form of a value.
    pub fn parse(text: &str) -> Option<Revise> {
        let inside = |prefix: &str, suffix: &str| text.strip_prefix(prefix)?.strip_suffix(suffix);
        let parsed = match text {
            "Off" => Revise::Off,
            "NotNeeded" => Revise::NotNeeded,
            "Unchanged" => Revise::Unchanged,
            "FromCheckpoint" => Revise::FromCheckpoint,
            "ResumedAsBuilt" => Revise::ResumedAsBuilt,
            _ => {
                if let Some(size) = inside("GreedyIsMax(", ")") {
                    Revise::GreedyIsMax(size.parse().ok()?)
                } else if let Some(fields) = inside("Searched { size: ", " }") {
                    let (size, nodes) = fields.split_once(", nodes: ")?;
                    Revise::Searched { size: size.parse().ok()?, nodes: nodes.parse().ok()? }
                } else {
                    Revise::Stopped { nodes: inside("Stopped { nodes: ", " }")?.parse().ok()? }
                }
            }
        };
        // The numbers as `Debug` writes them: no sign, no leading zeros.
        (format!("{parsed:?}") == text).then_some(parsed)
    }
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

impl<const W: usize> Prepared<W> {
    /// The search problem this setup poses (see [`Problem`]).
    pub fn problem(&self) -> Problem {
        Problem::of(&self.model, &self.root, &self.candidates, self.max_total)
    }
}

/// The search problem a game's setup poses, in a canonical form: everything the search is given that its answer
/// can depend on, so that two games with equal problems have the same feasible promotion sets and the same best
/// promotion sum (DESIGN.md, "Same search").
///
/// The search is given the model (the factor graph and each number's home frame), the root frame set, the
/// candidates and `max_total`, besides the tactics, the cap and the seed; the matching graph, the greedy set and
/// the preclusion table are functions of those.  The feasible promotion sets are the sets of candidates that can be
/// promoted from the root, keeping every frame's freedoms, with at most `max_total` members; which they are depends
/// on the frames and on the graph's edges among their numbers, not on `n`.  So the form records, as a list of
/// `u32`: the number of frames; for each frame, bottom first, its freedoms (as the bits of an `i32`) and its moves
/// and its factors (each list as its length and then its numbers, ascending); the candidates; `max_total`; and for
/// each number of the frames (their moves and factors), ascending, the number, its home frame, the maximal factors
/// that can pay for it and the maximal composites it can pay for, as the model's graph lists them.  The graph's
/// lists for a number can name numbers no longer in a frame (reducing the frames drops some), which the search never
/// sees; recording them anyway errs on the side of telling more problems apart.  The numbers are game numbers, so
/// the form does not depend on the set width.  The tactics, the cap and the seed are not in it: the sound tactics,
/// the cap and the seed change how the search runs, not its answer, and the unproven ones are in a game's taint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem(Vec<u32>);

impl Problem {
    /// The problem of a search of the model `model` from `root` over `candidates`, with at most `max_total`
    /// promotions.
    pub fn of<const W: usize>(model: &Model, root: &FrameSet<W>, candidates: &Bits<W>, max_total: u32) -> Problem {
        fn list(words: &mut Vec<u32>, items: impl ExactSizeIterator<Item = u32>) {
            words.push(items.len() as u32);
            words.extend(items);
        }
        let mut words = vec![root.len() as u32];
        let mut numbers = Bits::<W>::EMPTY;
        for frame in root.frames() {
            words.push(frame.freedoms as u32);
            list(&mut words, frame.moves.to_vec().into_iter());
            list(&mut words, frame.factors.to_vec().into_iter());
            numbers |= frame.moves | frame.factors;
        }
        list(&mut words, candidates.to_vec().into_iter());
        words.push(max_total);
        let graph = model.graph();
        for x in numbers.iter() {
            words.extend([x, model.home(x) as u32]);
            list(&mut words, graph.factors(x).iter().map(|&y| u32::from(y)));
            list(&mut words, graph.composites(x).iter().map(|&y| u32::from(y)));
        }
        Problem(words)
    }

    /// A digest of the problem: the engine's 64-bit checksum ([`engine::checksum`], the one checkpoints carry) of
    /// the form's numbers, each as four bytes, little-endian.  Two equal problems have the same digest, and two
    /// different ones the same with a chance of about 2^-64; the solver and `taxman verify-results` compare the
    /// problems themselves before they rely on two being the same.
    pub fn digest(&self) -> u64 {
        let bytes: Vec<u8> = self.0.iter().flat_map(|word| word.to_le_bytes()).collect();
        engine::checksum(&bytes)
    }
}

/// The most promotions a promotion set of the root `root` can have among `candidates` (`maxTotalPromotions`).
fn max_total<const W: usize>(root: &FrameSet<W>, candidates: &Bits<W>) -> u32 {
    root.max_promotions_among(candidates).max(0) as u32
}

/// The search problem ([`Problem`]) that [`prepare`] poses for the game whose frames are `columns`, when revising
/// frame 0 did what `revise` (a results line's `setup.revise`) says: the model and the root are built again, frame
/// 0's freedoms set as `revise` says ([`Revise::frame_zero`]), and `max_total` follows; nothing is searched (no
/// greedy set, no preclusions, no revising).
///
/// # Errors
///
/// If `revise` does not say what frame 0's freedoms were, or says more than frame 0 was built with (revising only
/// lowers them), or the frames cannot be built into a model.
pub fn problem_of<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    candidates: &Bits<W>,
    revise: Revise,
) -> Result<Problem, String> {
    let frame_zero = revise.frame_zero()?;
    let (model, root, _) = build_root::<W>(tables, columns)?;
    let root = match (frame_zero, root.frames().first()) {
        (None, _) => root,
        (Some(k), Some(frame)) if i64::from(k) <= i64::from(frame.freedoms) => root.with_frame_zero_freedoms(k as i32),
        (Some(k), frame) => {
            let built = frame.map_or_else(|| "no frame 0".to_owned(), |f| format!("{} freedoms", f.freedoms));
            return Err(format!("revising gave frame 0 {k} freedoms, but it was built with {built}"));
        }
    };
    Ok(Problem::of(&model, &root, candidates, max_total(&root, candidates)))
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
///
/// Every frame of the root is winnable (the frame builder makes it so), and the search relies on it: promoting a
/// move that has a factor no move of the frame below uses takes no peel (`Model::promote`), which is exact only
/// if that frame is winnable, as every frame of every state reached from a winnable root is.  Debug builds check
/// it here.
fn build_root<const W: usize>(tables: &Tables, columns: &[Vec<u32>]) -> Result<(Model, FrameSet<W>, Duration), String> {
    let t0 = Instant::now();
    let (model, root) = Model::build::<W>(tables, columns)?;
    let build = t0.elapsed();
    debug_assert!(
        root.frames().iter().all(|f| model.winnable(&f.moves, &f.factors)),
        "n={}: a frame of the root is not winnable",
        tables.n
    );
    Ok((model, root, build))
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
    let max_total = max_total(&revised, &candidates);
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
