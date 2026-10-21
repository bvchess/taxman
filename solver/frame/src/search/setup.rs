//! Setting a game up for the search ([`prepare`]): the model and the root frame set, the greedy incumbent and
//! `max_total`.  [`prepare_for_resume`] does the same for a search that a checkpoint will resume, taking the tactics
//! from it.  [`prepare_extended`] and [`prepare_extended_for_resume`] are the same for a problem with one-sided
//! candidates, the extended problem's (`crate::frames::build_extended`).
//!
//! The search problem a setup poses, in a canonical form that two games can be compared by, is a [`Problem`]
//! ([`Prepared::problem`]); [`problem_of`] rebuilds it from the frames alone, without preparing a search, and
//! [`problem_of_extended`] does the same for the extended problem's.

use super::header::Header;
use super::tactics::SearchTactics;
use crate::bits::Bits;
use crate::bounds;
use crate::engine;
use crate::gf2::{Gf2Bound, Gf2Graph, MAX_WORDS};
use crate::model::{FrameGraph, FrameSet, Scratch};
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
    /// The game's n.
    pub n: u32,
    pub model: Arc<FrameGraph>,
    /// The starting state of the search: the frames as built.
    pub root: FrameSet<W>,
    pub candidates: Bits<W>,
    pub tactics: SearchTactics,
    /// The greedy incumbent.
    pub greedy: Bits<W>,
    /// The most promotions a promotion set can have:
    /// `model.max_promotions_among(&root, &candidates)`, at least 0.  A count, not the ceiling on the promotion sum.
    pub max_total: u32,
    pub timings: PrepareTimings,
}

impl<const W: usize> Prepared<W> {
    /// The search problem this setup poses (see [`Problem`]).
    pub fn problem(&self) -> Problem {
        Problem::of(&self.model, &self.root, &self.candidates, self.max_total)
    }

    /// Whether `set` is a set of candidates that can be promoted from the root: the check of an incumbent that no
    /// step of this search made, a checkpoint's ([`resume_point`](super::resume_point)) or a finder's
    /// ([`offer_found`](super::offer_found)).
    pub fn can_promote(&self, set: &Bits<W>) -> bool {
        set.is_subset(&self.candidates) && self.model.promote_all_using(&mut Scratch::new(), &self.root, set).is_some()
    }
}

/// The search problem a game's setup poses, in a canonical form: everything the search is given that its answer
/// can depend on, so that two games with equal problems have the same promotable sets and the same best
/// promotion sum (solver/docs/design/same-search.md).
///
/// The search is given the model (the factor graph and each number's home frame), the root frame set, the
/// candidates and `max_total`, besides the tactics, the ceiling and the seed; the matching graph and the greedy set are
/// functions of those.  The promotable sets are the sets of candidates that can be promoted from the root,
/// keeping every frame's freedoms, with at most `max_total` members; which they are depends
/// on the frames and on the graph's edges among their numbers, not on `n`.  So the form records, as a list of
/// `u32`: the number of frames; for each frame, front first, its freedoms (as the bits of an `i32`) and its moves
/// and its factors (each list as its length and then its numbers, ascending); the candidates; `max_total`; and for
/// each number of the frames (their moves and factors), ascending, the number, its home frame, the maximal factors
/// that can pay for it and the maximal composites it can pay for, as the model's graph lists them.  The graph's
/// lists for a number can name numbers no longer in a frame (reducing the frames drops some), which the search never
/// sees; recording them anyway errs on the side of telling more problems apart.  The numbers are game numbers, so
/// the form does not depend on the set width.  The tactics, the ceiling and the seed are not in it: the sound tactics,
/// the ceiling and the seed change how the search runs, not its answer.
///
/// A problem with one-sided candidates (the extended problem's, [`prepare_extended`]) has them among its candidates
/// and, after all that, the tag [`Problem::ONE_SIDED`], their number, and for each, ascending, the number, its own
/// frame and the maximal factors that can pay for it.  A problem without them ends before the tag, so no such problem
/// equals one with them (the frames fix how many numbers the lists before the tag describe), and its form, and so its
/// digest, are what they were before one-sided candidates existed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem(Vec<u32>);

impl Problem {
    /// The tag before the one-sided candidates of a problem that has some: no number of a board (they end at 65535).
    pub const ONE_SIDED: u32 = u32::MAX;

    /// The problem of a search of the model `model` from `root` over `candidates`, with at most `max_total`
    /// promotions.
    pub fn of<const W: usize>(model: &FrameGraph, root: &FrameSet<W>, candidates: &Bits<W>, max_total: u32) -> Problem {
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
        if model.has_one_sided() {
            words.extend([Self::ONE_SIDED, model.one_sided().len() as u32]);
            for &(s, frame) in model.one_sided() {
                words.extend([s, frame as u32]);
                list(&mut words, graph.factors(s).iter().map(|&y| u32::from(y)));
            }
        }
        Problem(words)
    }

    /// The form's numbers, as listed above (for the tests of the form).
    #[cfg(test)]
    pub(super) fn form(&self) -> &[u32] {
        &self.0
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

/// The most promotions a promotion set of the root `root` of `model` can have among `candidates`.
fn max_total<const W: usize>(model: &FrameGraph, root: &FrameSet<W>, candidates: &Bits<W>) -> u32 {
    model.max_promotions_among(root, candidates).max(0) as u32
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
    problem_of_extended(tables, columns, &[], candidates)
}

/// The search problem ([`Problem`]) that [`prepare_extended`] poses for the game whose frames are `columns`, with the
/// one-sided candidates `one_sided` besides the (two-sided) `candidates`: built as [`prepare_extended`] builds it, but
/// with no greedy set and without the check of the GF(2) bound's block words, so that it can be compared with another
/// problem whatever set width either would be searched at.  With no one-sided candidates it is [`problem_of`].
///
/// # Errors
///
/// If the frames cannot be built into a model.
pub fn problem_of_extended<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    one_sided: &[(u32, usize)],
    candidates: &Bits<W>,
) -> Result<Problem, String> {
    let (model, root, candidates, _) = build_extended_root::<W>(tables, columns, one_sided, candidates)?;
    Ok(Problem::of(&model, &root, &candidates, max_total(&model, &root, &candidates)))
}

/// Sets up the search for the game whose frames are `columns` (factors and moves per frame, front first) and
/// whose promotion candidates are `candidates`.
pub fn prepare<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    candidates: &Bits<W>,
    tactics: SearchTactics,
) -> Result<Prepared<W>, String> {
    prepare_extended(tables, columns, &[], candidates, tactics)
}

/// [`prepare`] with the one-sided candidates `one_sided` of the extended problem besides the (two-sided) `candidates`,
/// each with its own frame (`model`'s module docs, "One-sided candidates"): the model of
/// [`FrameGraph::build_extended`], and the search's candidates both kinds.  With none it is [`prepare`].
///
/// # Errors
///
/// If the frames cannot be built into a model, or, with one-sided candidates, if the tactics are ones that do not
/// know them ([`SearchTactics::refusal_with_one_sided`]) or a frame needs more words of the GF(2) bound than the
/// engines of the set width `W` have ([`Gf2Bound::MOST_WORDS`]: opening a source adds one to its frame's rank).
pub fn prepare_extended<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    one_sided: &[(u32, usize)],
    candidates: &Bits<W>,
    tactics: SearchTactics,
) -> Result<Prepared<W>, String> {
    if let Some(refusal) = tactics.refusal_with_one_sided().filter(|_| !one_sided.is_empty()) {
        return Err(refusal);
    }
    let (model, root, candidates, build) = build_extended_root::<W>(tables, columns, one_sided, candidates)?;
    if !one_sided.is_empty() {
        // Each open source adds one to its frame's rank, which can take a frame past the engines of this set width.
        let words = Gf2Graph::new(&model, &root, &candidates).words();
        if words > Gf2Bound::<W>::MOST_WORDS {
            return Err(format!(
                "n={}: a frame of the problem needs {words} block words of the GF(2) bound, and the set width {W} has \
                 engines of at most {} (the widest, 128 words, has {MAX_WORDS})",
                tables.n,
                Gf2Bound::<W>::MOST_WORDS
            ));
        }
    }
    let t0 = Instant::now();
    let greedy = bounds::greedy(&model, &root, &candidates);
    let timings = PrepareTimings { build, greedy: t0.elapsed() };
    let max_total = max_total(&model, &root, &candidates);
    let n = tables.n;
    Ok(Prepared { n, model: Arc::new(model), root, candidates, tactics, greedy, max_total, timings })
}

/// How many block words the GF(2) bound of the problem of [`prepare_extended`] needs at its root
/// ([`Gf2Graph::words`]), whatever the tactics: what decides the set widths it can be searched at
/// ([`Gf2Bound::MOST_WORDS`]).
///
/// # Errors
///
/// If the frames cannot be built into a model.
pub fn extended_words<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    one_sided: &[(u32, usize)],
    candidates: &Bits<W>,
) -> Result<usize, String> {
    let (model, root, candidates, _) = build_extended_root::<W>(tables, columns, one_sided, candidates)?;
    Ok(Gf2Graph::new(&model, &root, &candidates).words())
}

/// The model and root of [`build_root`], the search's candidates (`candidates` and the one-sided ones), and the time it
/// took.
fn build_extended_root<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    one_sided: &[(u32, usize)],
    candidates: &Bits<W>,
) -> Result<(FrameGraph, FrameSet<W>, Bits<W>, Duration), String> {
    let (model, root, build) = build_root::<W>(tables, columns, one_sided)?;
    let candidates = one_sided.iter().fold(*candidates, |set, &(s, _)| set.with(s));
    Ok((model, root, candidates, build))
}

/// [`prepare`] for resuming the search checkpointed in `checkpoint`, with the tactics the checkpoint records.  Fails,
/// saying what differs, if the checkpoint was made for another n, width, candidates, frames or `max_total`, or by a
/// build whose checkpoint headers had another format.
pub fn prepare_for_resume<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    candidates: &Bits<W>,
    checkpoint: &Path,
) -> Result<Prepared<W>, String> {
    prepare_extended_for_resume(tables, columns, &[], candidates, checkpoint)
}

/// [`prepare_for_resume`] for the problem of [`prepare_extended`]; a checkpoint of another problem (other one-sided
/// candidates, say) is refused as one of another game is, and one of format version 4 (a build before this search)
/// for a problem with one-sided candidates first of all.
pub fn prepare_extended_for_resume<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    one_sided: &[(u32, usize)],
    candidates: &Bits<W>,
    checkpoint: &Path,
) -> Result<Prepared<W>, String> {
    let bytes = engine::read_checkpoint_header(checkpoint).map_err(|e| format!("{}: {e}", checkpoint.display()))?;
    let header = Header::<W>::decode(&bytes)?;
    // A checkpoint that cannot be of a search with one-sided candidates says so before its tactics are looked at.
    if let Some(refusal) = header.one_sided_refusal().filter(|_| !one_sided.is_empty()) {
        return Err(refusal);
    }
    let prepared = prepare_extended(tables, columns, one_sided, candidates, header.tactics)?;
    match header.mismatch(&Header::of(&prepared)) {
        Some(mismatch) => Err(mismatch),
        None => Ok(prepared),
    }
}

/// The model and the root frame set of the game whose frames are `columns` and whose one-sided candidates are
/// `one_sided`, and the time it took to build them.
///
/// Every frame of the root is winnable (the frame builder makes it so), and the search relies on it: promoting a
/// number that has a factor no move of the frame it is promoted into uses takes no peel (`FrameGraph::promote`), which
/// is exact only if that frame is winnable, as every frame of every state reached from a winnable root is.  Debug
/// builds check it here.
fn build_root<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    one_sided: &[(u32, usize)],
) -> Result<(FrameGraph, FrameSet<W>, Duration), String> {
    let t0 = Instant::now();
    let (model, root) = FrameGraph::build_extended::<W>(tables, columns, one_sided)?;
    let build = t0.elapsed();
    debug_assert!(
        root.frames().iter().all(|f| model.winnable(&f.moves, &f.factors)),
        "n={}: a frame of the root is not winnable",
        tables.n
    );
    Ok((model, root, build))
}
