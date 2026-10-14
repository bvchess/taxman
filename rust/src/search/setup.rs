//! Setting a game up for the search ([`prepare`], the Java's `SearchManager.create`): the model and the root
//! frame set, the greedy incumbent and `max_total`.  [`prepare_for_resume`] does the same for a search that a
//! checkpoint will resume, taking the tactics from it.
//!
//! The search problem a setup poses, in a canonical form that two games can be compared by, is a [`Problem`]
//! ([`Prepared::problem`]); [`problem_of`] rebuilds it from the frames alone, without preparing a search.

use super::header::Header;
use super::tactics::SearchTactics;
use crate::bits::Bits;
use crate::bounds;
use crate::engine;
use crate::model::{FrameSet, Model};
use crate::numtheory::Tables;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Wall times of [`prepare`]'s phases.
#[derive(Clone, Copy, Debug, Default)]
pub struct PrepareTimings {
    pub build: Duration,
    pub greedy: Duration,
}

/// A game set up for [`search`](super::search).
pub struct Prepared<const W: usize> {
    /// The board size.
    pub n: u32,
    pub model: Arc<Model>,
    /// The starting state of the search: the frames as built.
    pub root: FrameSet<W>,
    pub candidates: Bits<W>,
    pub tactics: SearchTactics,
    /// The greedy incumbent.
    pub greedy: Bits<W>,
    /// The most promotions a promotion set can have (`maxTotalPromotions`):
    /// `root.max_promotions_among(candidates)`, at least 0.  A count, not the ceiling on the promotion sum.
    pub max_total: u32,
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
/// candidates and `max_total`, besides the tactics, the cap and the seed; the matching graph and the greedy set are
/// functions of those.  The feasible promotion sets are the sets of candidates that can be promoted from the root,
/// keeping every frame's freedoms, with at most `max_total` members; which they are depends
/// on the frames and on the graph's edges among their numbers, not on `n`.  So the form records, as a list of
/// `u32`: the number of frames; for each frame, bottom first, its freedoms (as the bits of an `i32`) and its moves
/// and its factors (each list as its length and then its numbers, ascending); the candidates; `max_total`; and for
/// each number of the frames (their moves and factors), ascending, the number, its home frame, the maximal factors
/// that can pay for it and the maximal composites it can pay for, as the model's graph lists them.  The graph's
/// lists for a number can name numbers no longer in a frame (reducing the frames drops some), which the search never
/// sees; recording them anyway errs on the side of telling more problems apart.  The numbers are game numbers, so
/// the form does not depend on the set width.  The tactics, the cap and the seed are not in it: the sound tactics,
/// the cap and the seed change how the search runs, not its answer.
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

    /// How large the problem is: what a results line's `setup` records of it besides its digest, so that a problem set
    /// up again from the line can be checked against the line as far as it goes.
    pub fn size(&self) -> ProblemSize {
        let words = &self.0;
        // The frames: freedoms, then the moves and the factors, each a length and that many numbers.
        let mut at = 1;
        for _ in 0..words[0] {
            at += 1;
            at += 1 + words[at] as usize;
            at += 1 + words[at] as usize;
        }
        let candidates = words[at];
        ProblemSize { frames: words[0], candidates, max_total: words[at + 1 + candidates as usize] }
    }
}

/// How large a search problem is ([`Problem::size`]): its number of frames, its number of candidates, and
/// `max_total`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProblemSize {
    pub frames: u32,
    pub candidates: u32,
    pub max_total: u32,
}

/// The most promotions a promotion set of the root `root` can have among `candidates` (`maxTotalPromotions`).
fn max_total<const W: usize>(root: &FrameSet<W>, candidates: &Bits<W>) -> u32 {
    root.max_promotions_among(candidates).max(0) as u32
}

/// The search problem ([`Problem`]) that [`prepare`] poses for the game whose frames are `columns`: the model and the
/// root are built again, and `max_total` follows; nothing is searched (no greedy set).
///
/// # Errors
///
/// If the frames cannot be built into a model.
pub fn problem_of<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    candidates: &Bits<W>,
) -> Result<Problem, String> {
    let (model, root, _) = build_root::<W>(tables, columns)?;
    Ok(Problem::of(&model, &root, candidates, max_total(&root, candidates)))
}

/// Sets up the search for the game whose frames are `columns` (factors and moves per frame, bottom first) and
/// whose promotion candidates are `candidates`.
pub fn prepare<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    candidates: &Bits<W>,
    tactics: SearchTactics,
) -> Result<Prepared<W>, String> {
    let (model, root, build) = build_root::<W>(tables, columns)?;
    let t0 = Instant::now();
    let greedy = bounds::greedy(&model, &root, candidates);
    let timings = PrepareTimings { build, greedy: t0.elapsed() };
    let max_total = max_total(&root, candidates);
    let n = tables.n;
    Ok(Prepared { n, model: Arc::new(model), root, candidates: *candidates, tactics, greedy, max_total, timings })
}

/// [`prepare`] for resuming the search checkpointed in `checkpoint`, with the tactics the checkpoint records.  Fails,
/// saying what differs, if the checkpoint was made for another board size, width, candidates or frames, or by a build
/// whose checkpoint headers had another format, or with a search tactic that this build no longer has, or if its
/// frame 0 has other freedoms than the ones frame 0 is built with (nothing changes them now, so it is a checkpoint
/// of another search).
pub fn prepare_for_resume<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    candidates: &Bits<W>,
    checkpoint: &Path,
) -> Result<Prepared<W>, String> {
    let bytes = engine::read_checkpoint_header(checkpoint).map_err(|e| format!("{}: {e}", checkpoint.display()))?;
    let header = Header::<W>::decode(&bytes)?;
    let prepared = prepare(tables, columns, candidates, header.tactics)?;
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
