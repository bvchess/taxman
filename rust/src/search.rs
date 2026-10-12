//! The branch-and-bound search for the promotion set with the largest sum, on the parallel [`engine`].
//!
//! [`prepare`] sets a game up the way the Java's `SearchManager.create` does: the frame set, the preclusion
//! table, the greedy incumbent, and (tactic `revise_freedoms`) a revised frame 0 that can lower the cap on the
//! number of promotions.  [`search`] then explores the tree of `BranchExplorer.explore`, made lazy: a node's
//! candidates are tried from the largest down, and each child is explored depth-first as soon as it is made,
//! on an explicit stack that is reused from node to node (no recursion, no allocation per node):
//!
//! ```text
//! loop while cands non-empty:
//!     n = cands.max()
//!     if upper_bound(state, cands, max_add) + sum(promoted) < best: break
//!     cands.remove(n)
//!     new_state = state.promote(n) else continue
//!     new_promoted = promoted ∪ {n}
//!     max_next = max_additional(new_state, new_promoted, cands)
//!     child_cands = new_candidate_list(n, max_next, new_state, cands, promoted)
//!     if sum(promoted) + n + upper_bound(new_state, child_cands, max_next) > best:
//!         if sum(new_promoted) > best: offer new_promoted as incumbent
//!         push child (new_state, new_promoted, child_cands,
//!                     max_additional(new_state, new_promoted, child_cands))
//! ```
//!
//! Two exact shortcuts keep the expensive bound out of the way: the bound never exceeds the sum of the
//! `max_add` largest candidates, so when that sum already decides the comparison the bound is not computed;
//! and a child with no candidates is not pushed (its loop would do nothing).
//!
//! A unit of work is just `(promoted, cands)`: promotions happen in decreasing order along every path, so a
//! node's state is the root with `promoted` promoted, rebuilt in one pass (see [`Model::promote_all_using`]).
//! A rebuilt node recomputes `max_add` from its remaining candidates, which can only tighten it.

use crate::bits::Bits;
use crate::bounds::{self, max_additional, Bounds, Preclusions, RunnerUp};
use crate::engine::{self, Counters, Ctx, EngineConfig, Poll, Searcher, Start, Stats, Unit};
use crate::model::{FrameSet, Model};
use crate::numtheory::Tables;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------------------------------------
// Tactics and units

/// The tactics of DESIGN.md that act inside the search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchTactics {
    /// Rule candidates out with the depth-2 preclusion table (sound).
    pub preclusions: bool,
    /// Use the Java's runner-up in the upper bound (unsound; reproduces the Java).
    pub legacy_bound: bool,
    /// Lower frame 0's freedoms after solving the two-frame game (unproven: "the max-sum promotion set also has
    /// the max count").
    pub revise_freedoms: bool,
}

impl Default for SearchTactics {
    fn default() -> Self {
        SearchTactics { preclusions: true, legacy_bound: false, revise_freedoms: true }
    }
}

impl SearchTactics {
    fn rule(self) -> RunnerUp {
        if self.legacy_bound {
            RunnerUp::Legacy
        } else {
            RunnerUp::Sound
        }
    }

    fn bits(self) -> u8 {
        u8::from(self.preclusions) | u8::from(self.legacy_bound) << 1 | u8::from(self.revise_freedoms) << 2
    }

    fn from_bits(bits: u8) -> Self {
        SearchTactics { preclusions: bits & 1 != 0, legacy_bound: bits & 2 != 0, revise_freedoms: bits & 4 != 0 }
    }
}

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

fn put_bits<const W: usize>(out: &mut Vec<u8>, set: &Bits<W>) {
    for word in set.as_words() {
        out.extend_from_slice(&word.to_le_bytes());
    }
}

/// The set encoded by [`put_bits`]; `bytes` must be exactly `8·W` long.
fn get_bits<const W: usize>(bytes: &[u8]) -> Bits<W> {
    let mut words = [0u64; W];
    for (word, chunk) in words.iter_mut().zip(bytes.as_chunks::<8>().0) {
        *word = u64::from_le_bytes(*chunk);
    }
    Bits::from_words(words)
}

// ---------------------------------------------------------------------------------------------------------
// The searcher

/// What every searcher of one search shares, read-only.
pub struct SearchSpace<'a, const W: usize> {
    pub model: &'a Model,
    /// The starting state (frame 0's freedoms revised if that tactic is on).
    pub root: &'a FrameSet<W>,
    /// The preclusion table, if that tactic is on.
    pub preclusions: Option<&'a Preclusions<W>>,
    /// The candidates at the root.
    pub candidates: &'a Bits<W>,
    /// The cap on the number of promotions (`maxTotalPromotions`).
    pub max_total: u32,
    pub rule: RunnerUp,
    /// A proven ceiling on the promotion sum (n + opt(n − 1) − inTheBag): reaching it ends the search.
    pub cap: Option<u64>,
}

/// One node on the searcher's stack.
#[derive(Clone, Copy)]
struct Level<const W: usize> {
    state: FrameSet<W>,
    promoted: Bits<W>,
    /// The sum of `promoted`.
    sum: u64,
    /// The candidates still to try.
    cands: Bits<W>,
    max_add: u32,
}

/// Indexes into [`Stats::extra`].
pub mod stat {
    /// Promotions tried (each candidate a node's loop gets to).
    pub const PROMOTE_ATTEMPTS: usize = 0;
    /// Promotions that succeeded.
    pub const PROMOTIONS: usize = 1;
    /// Upper bounds computed.
    pub const UPPER_BOUNDS: usize = 2;
    /// Promotability checks made while building candidate lists.
    pub const CAN_PROMOTES: usize = 3;
    /// Children pushed on the stack (explored).
    pub const CHILDREN: usize = 4;
    /// Candidates the preclusion table ruled out.
    pub const PRECLUDED: usize = 5;
    /// Frame certificates made for the promotability checks.
    pub const CERTIFICATES: usize = 6;
}

/// The Taxman searcher: explores units depth-first on a reusable stack.
pub struct TaxSearcher<'a, const W: usize> {
    space: &'a SearchSpace<'a, W>,
    bounds: Bounds<'a, W>,
    /// Stack slots, reused: `levels[..=depth]` is the current path, and the next slot is where a child is made.
    levels: Vec<Level<W>>,
    attempts: u64,
    promotions: u64,
    children: u64,
    payload: Vec<u8>,
}

impl<'a, const W: usize> TaxSearcher<'a, W> {
    pub fn new(space: &'a SearchSpace<'a, W>) -> Self {
        TaxSearcher {
            space,
            bounds: Bounds::new(space.model, space.preclusions, space.rule),
            levels: Vec::new(),
            attempts: 0,
            promotions: 0,
            children: 0,
            payload: Vec::with_capacity(8 * W),
        }
    }

    /// Rebuilds the unit's node in slot 0.
    fn start(&mut self, unit: &TaxUnit<W>) {
        let space = self.space;
        let state = space
            .model
            .promote_all_using(self.bounds.scratch(), space.root, &unit.promoted)
            .unwrap_or_else(|| panic!("a unit's promotions {:?} cannot be promoted from the root", unit.promoted));
        let max_add = max_additional(space.max_total, &state, &unit.promoted, &unit.cands);
        let level = Level { state, promoted: unit.promoted, sum: unit.promoted.sum(), cands: unit.cands, max_add };
        if self.levels.is_empty() {
            self.levels.push(level);
        } else {
            self.levels[0] = level;
        }
    }

    /// One step of the loop of the node at `depth`: tries its next candidate.  Returns the depth of the node to
    /// work on next (one deeper if a child was pushed), or `None` when the unit is finished.
    fn step(&mut self, depth: usize, ctx: &mut Ctx<'_, TaxUnit<W>>) -> Option<usize> {
        if self.levels.len() == depth + 1 {
            let copy = self.levels[depth];
            self.levels.push(copy);
        }
        let space = self.space;
        let (path, rest) = self.levels.split_at_mut(depth + 1);
        let (node, child) = (&mut path[depth], &mut rest[0]);
        let bounds = &mut self.bounds;

        // The node's loop ends when its candidates are used up or cannot reach the incumbent.  The bound is at
        // most the sum of the `max_add` largest candidates, so that sum may decide without the bound.
        let best = ctx.best();
        let need = best.saturating_sub(node.sum); // the loop goes on while the bound is at least this
        if node.cands.is_empty()
            || node.cands.sum_largest(node.max_add) < need
            || !bounds.reaches(&node.state, &node.cands, node.max_add, need)
        {
            return depth.checked_sub(1);
        }
        let n = node.cands.pop_max().expect("not empty");

        self.attempts += 1;
        child.state.assign(&node.state);
        if !space.model.promote_in_place_using(bounds.scratch(), &mut child.state, n) {
            return Some(depth);
        }
        self.promotions += 1;
        child.promoted = node.promoted.with(n);
        child.sum = node.sum + u64::from(n);
        let max_next = max_additional(space.max_total, &child.state, &child.promoted, &node.cands);
        let cands = bounds.new_candidate_list(n, max_next, &child.state, &node.cands, &node.promoted);
        let need = (best + 1).saturating_sub(child.sum); // the child is worth it if its bound is at least this
        if cands.sum_largest(max_next) < need || !bounds.reaches(&child.state, &cands, max_next, need) {
            return Some(depth);
        }
        if child.sum > best {
            self.payload.clear();
            put_bits(&mut self.payload, &child.promoted);
            if ctx.offer(child.sum, &self.payload) && space.cap.is_some_and(|cap| child.sum >= cap) {
                ctx.declare_optimal();
            }
        }
        if cands.is_empty() {
            return Some(depth);
        }
        child.cands = cands;
        child.max_add = max_additional(space.max_total, &child.state, &child.promoted, &cands);
        self.children += 1;
        Some(depth + 1)
    }

    /// Adds this searcher's counts to the worker's statistics and resets them.
    fn flush(&mut self, stats: &mut Stats) {
        let b = std::mem::take(&mut self.bounds.counts);
        let counts = [
            (stat::PROMOTE_ATTEMPTS, std::mem::take(&mut self.attempts)),
            (stat::PROMOTIONS, std::mem::take(&mut self.promotions)),
            (stat::UPPER_BOUNDS, b.upper_bounds),
            (stat::CAN_PROMOTES, b.can_promotes),
            (stat::CHILDREN, std::mem::take(&mut self.children)),
            (stat::PRECLUDED, b.precluded),
            (stat::CERTIFICATES, b.certificates),
        ];
        for (i, count) in counts {
            stats.extra[i] += count;
        }
    }
}

impl<const W: usize> Searcher<TaxUnit<W>> for TaxSearcher<'_, W> {
    fn run(&mut self, unit: TaxUnit<W>, ctx: &mut Ctx<'_, TaxUnit<W>>) {
        self.start(&unit);
        let mut depth = 0;
        loop {
            if ctx.poll() == Poll::Yield {
                for level in &self.levels[..=depth] {
                    ctx.push(TaxUnit { promoted: level.promoted, cands: level.cands });
                }
                break;
            }
            ctx.stats().nodes += 1;
            match self.step(depth, ctx) {
                Some(next) => depth = next,
                None => break,
            }
        }
        self.flush(ctx.stats());
    }
}

// ---------------------------------------------------------------------------------------------------------
// Setup

/// What revising frame 0's freedoms did (`UpperBoundEstimator.reviseFreedoms`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Revise {
    /// The tactic is off.
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
    /// Frame 0's freedoms were taken from a checkpoint.
    FromCheckpoint,
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

/// A game set up for [`search`].
pub struct Prepared<const W: usize> {
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
    /// `root.max_promotions_among(candidates)`, at least 0.
    pub max_total: u32,
    pub revise: Revise,
    pub timings: PrepareTimings,
}

/// Sets up the search for the game whose frames are `columns` (factors and moves per frame, bottom first) and
/// whose promotion candidates are `candidates`.  `threads` is used for the preclusion table and for the
/// two-frame search of `revise_freedoms`.
pub fn prepare<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    candidates: &Bits<W>,
    tactics: SearchTactics,
    threads: usize,
) -> Result<Prepared<W>, String> {
    let t0 = Instant::now();
    let (model, root) = Model::build::<W>(tables, columns)?;
    let build = t0.elapsed();
    let mut prepared = prepare_state(Arc::new(model), root, *candidates, tactics, threads, None)?;
    prepared.timings.build = build;
    Ok(prepared)
}

/// [`prepare`] for resuming the search checkpointed in `checkpoint`: frame 0's freedoms come from the
/// checkpoint's header instead of being revised again (which can take a search of its own), and the tactics
/// are the checkpoint's.  Fails if the checkpoint was made for a different game or width.
pub fn prepare_for_resume<const W: usize>(
    tables: &Tables,
    columns: &[Vec<u32>],
    candidates: &Bits<W>,
    checkpoint: &Path,
    threads: usize,
) -> Result<Prepared<W>, String> {
    let bytes = engine::read_checkpoint_header(checkpoint).map_err(|e| format!("{}: {e}", checkpoint.display()))?;
    let header = Header::<W>::decode(&bytes)?;
    if header.candidates != *candidates {
        return Err("the checkpoint is for different candidates".into());
    }
    let t0 = Instant::now();
    let (model, root) = Model::build::<W>(tables, columns)?;
    let build = t0.elapsed();
    let mut prepared =
        prepare_state(Arc::new(model), root, *candidates, header.tactics, threads, Some(header.frame0_freedoms))?;
    prepared.timings.build = build;
    if prepared.max_total != header.max_total {
        return Err(format!("max_total {} differs from the checkpoint's {}", prepared.max_total, header.max_total));
    }
    Ok(prepared)
}

/// The setup after the frame set exists.  `frame0` (from a checkpoint) replaces revising.
fn prepare_state<const W: usize>(
    model: Arc<Model>,
    root: FrameSet<W>,
    candidates: Bits<W>,
    tactics: SearchTactics,
    threads: usize,
    frame0: Option<Option<i32>>,
) -> Result<Prepared<W>, String> {
    let mut timings = PrepareTimings::default();
    let t0 = Instant::now();
    let preclusions = tactics.preclusions.then(|| Preclusions::build(&model, &root, &candidates, threads));
    timings.preclusions = t0.elapsed();
    let t0 = Instant::now();
    let greedy = bounds::greedy(&model, &root, &candidates);
    timings.greedy = t0.elapsed();
    let t0 = Instant::now();
    let (revised, revise) = match frame0 {
        Some(Some(freedoms)) => (root.with_frame_zero_freedoms(freedoms), Revise::FromCheckpoint),
        Some(None) => (root, Revise::FromCheckpoint),
        None if tactics.revise_freedoms => revise_freedoms(&model, &root, &candidates, &greedy, tactics, threads)?,
        None => (root, Revise::Off),
    };
    timings.revise = t0.elapsed();
    let max_total = revised.max_promotions_among(&candidates).max(0) as u32;
    Ok(Prepared {
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

/// The Java's `UpperBoundEstimator.reviseFreedoms`.  When the greedy incumbent has fewer promotions than the
/// freedoms allow and there are more than 2 frames, the promotions into frame 0 may be fewer than its freedoms
/// suggest.  The two-frame game {frame 0, frame 1} with frame 1's candidates settles how many: by its greedy
/// set if that reaches `min(freedoms of frames 0 and 1)`, else by searching it, but only if that could change
/// the full game's cap on promotions.  Frame 0's freedoms become that number.
fn revise_freedoms<const W: usize>(
    model: &Arc<Model>,
    root: &FrameSet<W>,
    candidates: &Bits<W>,
    greedy: &Bits<W>,
    tactics: SearchTactics,
    threads: usize,
) -> Result<(FrameSet<W>, Revise), String> {
    let full_max = root.max_promotions_among(candidates);
    if i64::from(greedy.len()) == i64::from(full_max) || root.len() <= 2 {
        return Ok((*root, Revise::NotNeeded));
    }
    let pair = root.front_pair();
    let (f0, f1) = (&pair.frames()[0], &pair.frames()[1]);
    let best_possible = f0.freedoms.min(f1.freedoms);
    let small = prepare_state(model.clone(), pair, *candidates & f1.factors, tactics, threads, None)?;
    let small_greedy = small.greedy.len();
    if i64::from(small_greedy) == i64::from(best_possible) {
        return Ok((root.with_frame_zero_freedoms(small_greedy as i32), Revise::GreedyIsMax(small_greedy)));
    }
    if root.with_frame_zero_freedoms(small_greedy as i32).max_promotions_among(candidates) == full_max {
        return Ok((*root, Revise::Unchanged));
    }
    let cfg = EngineConfig { threads, label: "revise".into(), progress: None, ..EngineConfig::default() };
    let found = search(&small, &cfg, None, None, false).map_err(|e| format!("revise search: {e}"))?;
    let size = found.promotions.len();
    Ok((root.with_frame_zero_freedoms(size as i32), Revise::Searched { size, nodes: found.stats.nodes }))
}

// ---------------------------------------------------------------------------------------------------------
// Checkpoint header

/// What a checkpoint records about the search it belongs to, so that a resumed run can check it is the same
/// search and skip revising frame 0 again.
struct Header<const W: usize> {
    candidates: Bits<W>,
    max_total: u32,
    /// Frame 0's freedoms in the search's root (`None` without frames).
    frame0_freedoms: Option<i32>,
    tactics: SearchTactics,
}

impl<const W: usize> Header<W> {
    const MAGIC: &'static [u8; 4] = b"TXS1";

    fn of(prepared: &Prepared<W>) -> Self {
        Header {
            candidates: prepared.candidates,
            max_total: prepared.max_total,
            frame0_freedoms: prepared.root.frames().first().map(|f| f.freedoms),
            tactics: prepared.tactics,
        }
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Self::MAGIC.to_vec();
        out.extend_from_slice(&(W as u32).to_le_bytes());
        put_bits(&mut out, &self.candidates);
        out.extend_from_slice(&self.max_total.to_le_bytes());
        out.push(u8::from(self.frame0_freedoms.is_some()));
        out.extend_from_slice(&self.frame0_freedoms.unwrap_or(0).to_le_bytes());
        out.push(self.tactics.bits());
        out
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        let len = 4 + 4 + 8 * W + 4 + 1 + 4 + 1;
        if bytes.len() != len || &bytes[..4] != Self::MAGIC {
            return Err("not a checkpoint of a Taxman search".into());
        }
        let u32_at = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().expect("4 bytes"));
        let width = u32_at(4) as usize;
        if width != W {
            return Err(format!("the checkpoint was made with Bits<{width}>, not Bits<{W}>"));
        }
        let mut at = 8 + 8 * W;
        let candidates = get_bits(&bytes[8..at]);
        let max_total = u32_at(at);
        let has_frame0 = bytes[at + 4] != 0;
        let frame0 = u32_at(at + 5) as i32;
        at += 9;
        Ok(Header {
            candidates,
            max_total,
            frame0_freedoms: has_frame0.then_some(frame0),
            tactics: SearchTactics::from_bits(bytes[at]),
        })
    }
}

// ---------------------------------------------------------------------------------------------------------
// Search

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
}

/// The candidates of `seed` that can still be promoted, one by one from the largest down, skipping the ones
/// that fail.
fn surviving<const W: usize>(prepared: &Prepared<W>, seed: &Bits<W>) -> Bits<W> {
    let mut state = prepared.root;
    let mut kept = Bits::EMPTY;
    for n in (*seed & prepared.candidates).iter_desc() {
        if prepared.model.promote_in_place(&mut state, n) {
            kept.insert(n);
        }
    }
    kept
}

/// Searches for the promotion set with the largest sum.
///
/// The incumbent starts as the greedy set, or as what survives of `seed` (see [`surviving`]) if that is
/// better.  `cap` is a proven ceiling on the sum; reaching it ends the search.  With `resume`, the search
/// continues from the checkpoint configured in `cfg` (the `prepared` game must have been set up by
/// [`prepare_for_resume`] or identically); otherwise it starts fresh.
pub fn search<const W: usize>(
    prepared: &Prepared<W>,
    cfg: &EngineConfig,
    seed: Option<&Bits<W>>,
    cap: Option<u64>,
    resume: bool,
) -> io::Result<SearchResult<W>> {
    let t0 = Instant::now();
    let mut incumbent = prepared.greedy;
    if let Some(seed) = seed {
        let kept = surviving(prepared, seed);
        if kept.sum() > incumbent.sum() {
            incumbent = kept;
        }
    }
    if !resume && cap.is_some_and(|cap| incumbent.sum() >= cap) {
        return Ok(SearchResult {
            promotions: incumbent,
            sum: incumbent.sum(),
            completed: true,
            reached_cap: true,
            stats: Stats::default(),
            counters: Counters::default(),
            time: t0.elapsed(),
        });
    }

    let space = SearchSpace {
        model: &prepared.model,
        root: &prepared.root,
        preclusions: prepared.preclusions.as_ref(),
        candidates: &prepared.candidates,
        max_total: prepared.max_total,
        rule: prepared.tactics.rule(),
        cap,
    };
    let start = if resume {
        let Some((path, _)) = &cfg.checkpoint else {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "resuming needs a checkpoint file"));
        };
        Start::Resume(path.clone())
    } else {
        let mut payload = Vec::new();
        put_bits(&mut payload, &incumbent);
        let root = TaxUnit { promoted: Bits::EMPTY, cands: prepared.candidates };
        Start::Fresh { units: vec![root], best: incumbent.sum(), payload }
    };
    let header = Header::of(prepared).encode();
    let outcome = engine::run(cfg, start, &header, |_| TaxSearcher::new(&space))?;
    let promotions = get_bits(&outcome.payload);
    assert_eq!(promotions.sum(), outcome.best, "the incumbent's payload does not match its value");
    Ok(SearchResult {
        promotions,
        sum: outcome.best,
        completed: outcome.completed,
        reached_cap: outcome.optimal_declared,
        stats: outcome.stats,
        counters: outcome.counters,
        time: t0.elapsed(),
    })
}

// ---------------------------------------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::build_frames;
    use crate::json::Json;
    use crate::model::test_support::*;
    use crate::model::Scratch;
    use std::path::PathBuf;

    fn config(threads: usize) -> EngineConfig {
        EngineConfig { threads, progress: None, ..EngineConfig::default() }
    }

    /// The golden games the Java had to search (not accelerated), up to `max_n`.
    fn searched_games(max_n: u32) -> Vec<Golden> {
        golden_ns()
            .into_iter()
            .filter(|&n| n <= max_n)
            .map(load)
            .filter(|g| g.json.get("solve").and_then(|s| s.get("accelerated")).and_then(Json::as_bool) == Some(false))
            .collect()
    }

    /// The sum of the Java's optimal promotions.
    fn golden_sum(g: &Golden) -> u64 {
        u32s(g.json.get("solve").expect("solve"), "promotions").iter().map(|&x| u64::from(x)).sum()
    }

    fn prepare_golden<const W: usize>(g: &Golden, tactics: SearchTactics) -> Prepared<W> {
        prepare::<W>(&Tables::new(g.n), &g.columns, &Bits::from_slice(&g.candidates), tactics, 2).unwrap()
    }

    /// Searches and checks that the result is complete, made of candidates, and feasible.
    fn search_checked<const W: usize>(p: &Prepared<W>, threads: usize, what: &str) -> SearchResult<W> {
        let r = search(p, &config(threads), None, None, false).unwrap();
        assert!(r.completed, "{what}: incomplete");
        assert!(r.promotions.is_subset(&p.candidates), "{what}: {:?} are not all candidates", r.promotions);
        let feasible = p.model.promote_all_using(&mut Scratch::new(), &p.root, &r.promotions);
        assert!(feasible.is_some(), "{what}: {:?} cannot be promoted", r.promotions);
        r
    }

    fn check_against_java<const W: usize>(g: &Golden) {
        let p = prepare_golden::<W>(g, SearchTactics::default());
        let manager = g.json.get("searchManager").expect("searchManager");
        let frame0 = manager.get("revisedFrame0Freedoms").and_then(Json::as_i64);
        assert_eq!(p.root.frames().first().map(|f| i64::from(f.freedoms)), frame0, "n={} frame 0 freedoms", g.n);
        let max_total = manager.get("maxTotalPromotions").and_then(Json::as_i64).expect("maxTotalPromotions");
        assert_eq!(i64::from(p.max_total), max_total, "n={} max_total ({:?})", g.n, p.revise);
        for threads in [1, 4] {
            let r = search_checked(&p, threads, &format!("n={} with {threads} threads", g.n));
            assert_eq!(r.sum, golden_sum(g), "n={} with {threads} threads: {:?}", g.n, r.promotions);
        }
    }

    #[test]
    fn golden_games_up_to_400_match_the_java() {
        let games = searched_games(400);
        assert!(games.len() > 80, "only {} games", games.len());
        for g in &games {
            check_against_java::<8>(g);
        }
    }

    fn sums_by_tactics<const W: usize>(g: &Golden) -> Vec<(SearchTactics, u64)> {
        let default = SearchTactics::default();
        let variants = [
            default,
            SearchTactics { preclusions: false, ..default },
            SearchTactics { legacy_bound: true, ..default },
            SearchTactics { revise_freedoms: false, ..default },
        ];
        variants.iter().map(|&t| (t, search_checked(&prepare_golden::<W>(g, t), 1, "tactics").sum)).collect()
    }

    #[test]
    fn tactics_do_not_change_answers() {
        let mut differences = Vec::new();
        for g in &searched_games(400) {
            let sums = sums_by_tactics::<8>(g);
            for &(tactics, sum) in &sums[1..] {
                if sum != sums[0].1 {
                    differences.push(format!("n={}: {tactics:?} gives {sum}, default {}", g.n, sums[0].1));
                }
            }
        }
        assert!(differences.is_empty(), "AN UNPROVEN TACTIC CHANGES ANSWERS:\n{}", differences.join("\n"));
    }

    /// A fresh directory under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!("taxman-search-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("create temp dir");
            TempDir(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn resume_in_pieces(threads: usize) {
        let g = load(480);
        let tables = Tables::new(g.n);
        let cands = Bits::<8>::from_slice(&g.candidates);
        let whole = prepare::<8>(&tables, &g.columns, &cands, SearchTactics::default(), 2).unwrap();
        let nodes = search_checked(&whole, 1, "uninterrupted").stats.nodes;

        let dir = TempDir::new(&format!("resume{threads}"));
        let ckpt = dir.0.join("ckpt.bin");
        let cfg = EngineConfig {
            checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
            node_limit: Some(nodes / 6 + 1),
            queue_mem_cap: 4,
            spill_dir: Some(dir.0.join("spill")),
            ..config(threads)
        };
        let mut result = search(&whole, &cfg, None, None, false).unwrap();
        let mut pieces = 1;
        while !result.completed {
            assert!(ckpt.exists(), "a stopped run leaves a checkpoint");
            let resumed = prepare_for_resume::<8>(&tables, &g.columns, &cands, &ckpt, 2).unwrap();
            assert_eq!(resumed.revise, Revise::FromCheckpoint);
            assert_eq!(resumed.root, whole.root);
            result = search(&resumed, &cfg, None, None, true).unwrap();
            pieces += 1;
            assert!(pieces < 100, "no progress");
        }
        assert!(pieces >= 4, "only {pieces} pieces");
        assert!(!ckpt.exists(), "a completed run deletes its checkpoint");
        assert_eq!(result.sum, golden_sum(&g));
        if threads == 1 {
            // One thread explores in exact depth-first order however often it is interrupted.
            assert_eq!(result.stats.nodes, nodes);
        }
    }

    #[test]
    fn checkpoint_and_resume_one_thread() {
        resume_in_pieces(1);
    }

    #[test]
    fn checkpoint_and_resume_four_threads() {
        resume_in_pieces(4);
    }

    #[test]
    fn a_checkpoint_of_another_game_is_refused() {
        let (a, b) = (load(480), load(400));
        let dir = TempDir::new("refuse");
        let ckpt = dir.0.join("ckpt.bin");
        let pa = prepare_golden::<8>(&a, SearchTactics::default());
        let cfg = EngineConfig {
            checkpoint: Some((ckpt.clone(), Duration::from_secs(3600))),
            node_limit: Some(50),
            ..config(1)
        };
        assert!(!search(&pa, &cfg, None, None, false).unwrap().completed);
        let cands = Bits::<8>::from_slice(&b.candidates);
        assert!(prepare_for_resume::<8>(&Tables::new(b.n), &b.columns, &cands, &ckpt, 1).is_err());
        let cands = Bits::<16>::from_slice(&a.candidates);
        assert!(prepare_for_resume::<16>(&Tables::new(a.n), &a.columns, &cands, &ckpt, 1).is_err());
    }

    #[test]
    fn seeds_and_caps() {
        let g = load(300);
        let p = prepare_golden::<8>(&g, SearchTactics::default());
        let optimum = search_checked(&p, 1, "plain");
        // Seeded with the optimum (plus a non-candidate), the search only has to prove it.
        let seed = optimum.promotions.with(299);
        let seeded = search(&p, &config(1), Some(&seed), None, false).unwrap();
        assert_eq!((seeded.sum, seeded.promotions), (optimum.sum, optimum.promotions));
        assert!(seeded.stats.nodes <= optimum.stats.nodes);
        // A cap the seed already reaches ends the search before it starts; a cap the search reaches ends it
        // there.
        let capped = search(&p, &config(1), Some(&seed), Some(optimum.sum), false).unwrap();
        assert!(capped.completed && capped.reached_cap && capped.stats.nodes == 0);
        let capped = search(&p, &config(1), None, Some(optimum.sum), false).unwrap();
        assert!(capped.completed && capped.reached_cap && capped.sum == optimum.sum);
        assert!(capped.stats.nodes <= optimum.stats.nodes);
        // A worse seed changes nothing.
        let worse = search(&p, &config(1), Some(&Bits::EMPTY), None, false).unwrap();
        assert_eq!(worse.sum, optimum.sum);
    }

    #[test]
    fn units_and_headers_round_trip() {
        let unit = TaxUnit::<3> { promoted: Bits::from_slice(&[1, 64, 190]), cands: Bits::from_slice(&[0, 63, 127]) };
        let mut bytes = Vec::new();
        unit.encode(&mut bytes);
        assert_eq!(bytes.len(), 48);
        assert_eq!(TaxUnit::<3>::decode(&bytes), unit);

        let p = prepare_golden::<8>(&load(300), SearchTactics { legacy_bound: true, ..SearchTactics::default() });
        let header = Header::of(&p);
        let back = Header::<8>::decode(&header.encode()).unwrap();
        assert_eq!(
            (back.candidates, back.max_total, back.frame0_freedoms, back.tactics),
            (p.candidates, p.max_total, p.root.frames().first().map(|f| f.freedoms), p.tactics)
        );
        assert!(Header::<16>::decode(&header.encode()).is_err());
        assert!(Header::<8>::decode(b"nonsense").is_err());
    }

    // ---- benchmark: cargo test --release bench_search -- --ignored --nocapture --test-threads=1 ----

    fn bench_game<const W: usize>(n: u32, columns: &[Vec<u32>], cands: &Bits<W>, java: (f64, f64)) {
        let tables = Tables::new(n);
        for (threads, java_secs) in [(1, java.0), (10, java.1)] {
            let t0 = Instant::now();
            let p = prepare::<W>(&tables, columns, cands, SearchTactics::default(), threads).unwrap();
            let r = search(&p, &config(threads), None, None, false).unwrap();
            let total = t0.elapsed().as_secs_f64();
            let t = &p.timings;
            let revise_nodes = if let Revise::Searched { nodes, .. } = p.revise { nodes } else { 0 };
            eprintln!(
                "n={n:4} threads={threads:2}: total {total:7.3} s (Java {java_secs:5.1} s) | build {:.3} prec {:.3} \
                 greedy {:.4} revise {:.3} search {:.3} | nodes {} revise nodes {revise_nodes} | sum {} max_total {} \
                 {:?} | extra {:?}",
                t.build.as_secs_f64(),
                t.preclusions.as_secs_f64(),
                t.greedy.as_secs_f64(),
                t.revise.as_secs_f64(),
                r.time.as_secs_f64(),
                r.stats.nodes,
                r.sum,
                p.max_total,
                p.revise,
                &r.stats.extra[..6],
            );
        }
    }

    /// One game (`TAXMAN_BENCH_N`, default 700; `TAXMAN_BENCH_THREADS`, default 1; `TAXMAN_BENCH_LEGACY` for the
    /// legacy bound), with progress lines, for profiling.
    #[test]
    #[ignore]
    fn bench_search_one() {
        let n = std::env::var("TAXMAN_BENCH_N").ok().and_then(|s| s.parse().ok()).unwrap_or(700);
        let threads = std::env::var("TAXMAN_BENCH_THREADS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
        let legacy_bound = std::env::var_os("TAXMAN_BENCH_LEGACY").is_some();
        let tactics = SearchTactics { legacy_bound, ..SearchTactics::default() };
        let g = load(n);
        let cands = Bits::<16>::from_slice(&g.candidates);
        let p = prepare::<16>(&Tables::new(n), &g.columns, &cands, tactics, threads).unwrap();
        let cfg = EngineConfig { progress: Some(Duration::from_secs(10)), ..config(threads) };
        let r = search(&p, &cfg, None, None, false).unwrap();
        eprintln!(
            "n={n}: {:?} search {:.3} s, nodes {}, extra {:?}",
            p.timings,
            r.time.as_secs_f64(),
            r.stats.nodes,
            r.stats.extra
        );
    }

    #[test]
    #[ignore]
    fn bench_search() {
        // Java whole-game wall times, one thread and ten (including ~0.5 s of JVM start and setup).
        let java = [
            (480, 2.3, 1.5),
            (500, 2.1, 1.4),
            (600, 2.7, 1.7),
            (640, 2.0, 1.6),
            (660, 4.1, 2.1),
            (700, 18.0, 4.8),
            (720, 23.2, 5.6),
            (760, 65.6, 12.4),
        ];
        for (n, one, ten) in java {
            let g = load(n);
            if n < 512 {
                bench_game::<8>(n, &g.columns, &Bits::from_slice(&g.candidates), (one, ten));
            } else {
                bench_game::<16>(n, &g.columns, &Bits::from_slice(&g.candidates), (one, ten));
            }
        }
        if std::env::var_os("TAXMAN_BENCH_1000").is_some() {
            let frames = build_frames::<16>(&Tables::new(1000));
            let columns: Vec<Vec<u32>> =
                frames.frames.iter().flat_map(|f| [f.factors.to_vec(), f.moves.to_vec()]).collect();
            bench_game::<16>(1000, &columns, &frames.candidates, (f64::NAN, f64::NAN));
        }
    }
}
