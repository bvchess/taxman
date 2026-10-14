//! The GF(2) bound: an upper bound on the sum of the promotions a search state can still make, from two linear
//! matroids over GF(2) in which every promotable set is independent (tactic `gf2`).  It is never above the matching
//! bound ([`crate::matchbound`]) and usually far below it; `heuristics/THEORY.md`, Theorem 11, has the proof, and
//! `docs/EXPERIMENTS.md`, "Matroid intersection bounds" and "The GF(2) bound in the search", the measurements.
//!
//! # The matroids
//!
//! Give each frame `h` of a state the vector space over GF(2) whose coordinates are its factors, each *mover* `u`
//! paying in it (a move of `h`, or a candidate of home `h + 1`, promoted into `h`) the vector of its maximal factors
//! among them ([`FactorProvider::factors`](crate::model::FactorProvider::factors)), and each candidate `y` of home
//! `h` the unit vector `e_y`.  If a set `Q` of candidates can be promoted, then in every frame the vectors of its
//! moves, of `Q`'s members promoted into it and the unit vectors of `Q`'s members promoted out of it are linearly
//! independent (Theorem 11: a winnable frame peels completely, each mover dropped with a live factor no mover still
//! there has).  The moves of a frame are fixed, so the frame's condition is a linear matroid on the candidates that
//! touch it: their vectors reduced modulo the span of the moves (contraction), in the quotient's coordinates, whose
//! number is the frame's freedoms.  A candidate touches its home frame and the one below, one of even index and one of
//! odd, so the even frames' matroids are the blocks of one matroid `A` on the candidates and the odd frames' of
//! another, `B`.  Every promotable set is independent in both, and the heaviest common independent set (weighted
//! matroid intersection, [`meet`]) bounds the best completion, with at most `max_add` members.
//!
//! [`Gf2Graph`] builds the root's matroids once per search; [`Gf2Bound`] keeps one level per node on the search's
//! path, like [`MatchBound`](crate::matchbound::MatchBound).
//!
//! # Contraction along the path
//!
//! A state reached by promoting `x` has `x` among its moves: its matroids are the parent's contracted by `x` in both
//! of its frames (each vector of the frame reduced by `x`'s, one pivot), and a candidate that the state's reduction
//! consumed becomes a loop (a zero vector) there.  (Checked against the matroids built from the state itself at
//! every node of the probe's fixtures: `examples/matroid.rs`, `nodes`.)  Candidates dropped from a node are deleted:
//! they leave its *live* set.
//!
//! # Warm starts
//!
//! The augmentations of [`meet`] may start from any *dominant* set: the heaviest common independent set of its size,
//! and heavier than every smaller one.  Every set they pass through is dominant, and each level keeps them (its
//! *history*, by increasing size).  They stay useful below it:
//!
//! * Deleting candidates `D` keeps a dominant set that avoids `D` dominant (fewer sets compete).
//! * Contracting members of a dominant set `S` (promoting `F ⊆ S`) leaves `S − F` dominant in the contraction:
//!   a common independent set `J` of the contraction makes `J ∪ F` one of the parent's, so the contraction's
//!   heaviest set of each size `s` weighs at most the parent's of size `s + |F|` less `w(F)`, which is at most
//!   `w(S) − w(F)` for `s + |F| ≤ |S|` and less for smaller `s`.
//!
//! So a level starts from the last entry of its history that the changes since keep (an entry that holds every
//! promoted candidate and no deleted one), less what was promoted; and when that is the parent's heaviest set
//! itself, which is the heaviest of all sizes, the child's heaviest set is known with no augmentation at all.
//!
//! # Forced members
//!
//! With the heaviest set `I*` of all sizes, [`meet::Meet::split_bounds`] bounds the problem without each member of
//! `I*` from a weight split, in one pass; a member without which the bound is below what the node needs is forced:
//! every completion that beats the incumbent promotes it.

pub mod meet;

use crate::bits::Bits;
use crate::model::{FrameSet, Model};
use meet::{bit, is_zero, lowest, xor, Meet, Problem, Vector};

const NONE: u32 = u32::MAX;

// ---------------------------------------------------------------------------------------------------------
// The root's matroids

/// The frame matroids of the root of a search, built once and shared by its searchers.
///
/// The *elements* are the root's candidates that are not loops (in both matroids their vectors are not zero),
/// indexed in increasing order: element `i` is the number `elems[i]`, which is also its weight, so the elements by
/// decreasing weight are the elements by decreasing index.
#[derive(Clone, Debug)]
pub struct Gf2Graph {
    elems: Vec<u32>,
    /// The element index of each number, `NONE` for numbers that are not elements.
    index: Vec<u32>,
    /// Words of a block vector: the largest frame rank, in 64-bit words.
    words: usize,
    /// Blocks in `A` (frames 0, 2, 4, …) and in `B` (frames 1, 3, 5, …).
    blocks: [usize; 2],
    /// Each element's block in `A` and in `B`.
    block: [Vec<u8>; 2],
    /// Each element's vector in `A` and in `B`: `words` words each.
    vecs: [Vec<u64>; 2],
    /// The elements of each block.
    block_elems: [Vec<Vec<u32>>; 2],
    weight: Vec<i64>,
    /// Each frame's rank (its freedoms, the dimension of its quotient).
    ranks: Vec<usize>,
}

/// One frame's coordinates (its factors) and the span of its moves, in echelon form by lowest set bit.
struct FrameSpace {
    /// Coordinate of each number that is a factor of the frame, `NONE` for the others.
    coord: Vec<u32>,
    words: usize,
    pivot_row: Vec<Option<Vec<u64>>>,
    /// The quotient coordinate of each coordinate that is not a pivot (`NONE` for pivots).
    quotient: Vec<u32>,
    rank: usize,
}

impl FrameSpace {
    fn new<const W: usize>(model: &Model, factors: &Bits<W>, moves: &Bits<W>, max: u32) -> Result<Self, String> {
        let mut coord = vec![NONE; max as usize + 1];
        for (k, y) in factors.iter().enumerate() {
            coord[y as usize] = k as u32;
        }
        let dim = factors.len() as usize;
        let words = dim.div_ceil(64).max(1);
        let mut s = FrameSpace { coord, words, pivot_row: vec![None; 64 * words], quotient: Vec::new(), rank: 0 };
        for m in moves.iter() {
            let r = s.reduce(s.vector(model, m));
            let p = r.iter().enumerate().find(|(_, &w)| w != 0).map(|(i, w)| 64 * i + w.trailing_zeros() as usize);
            let p = p.ok_or_else(|| format!("the moves of a frame are dependent (at {m}): the frame is unwinnable"))?;
            s.pivot_row[p] = Some(r);
        }
        let mut q = 0u32;
        s.quotient = (0..64 * words)
            .map(|c| {
                if c < dim && s.pivot_row[c].is_none() {
                    q += 1;
                    q - 1
                } else {
                    NONE
                }
            })
            .collect();
        s.rank = q as usize;
        Ok(s)
    }

    /// The vector of the mover `u`: its maximal factors among the frame's factors.
    fn vector(&self, model: &Model, u: u32) -> Vec<u64> {
        let mut v = vec![0u64; self.words];
        for &q in model.graph().factors(u) {
            if let Some(&c) = self.coord.get(usize::from(q)) {
                if c != NONE {
                    v[c as usize / 64] |= 1 << (c % 64);
                }
            }
        }
        v
    }

    /// The unit vector of the factor `y`.
    fn unit(&self, y: u32) -> Vec<u64> {
        let mut v = vec![0u64; self.words];
        let c = self.coord[y as usize];
        assert_ne!(c, NONE, "{y} is not a factor of the frame");
        v[c as usize / 64] |= 1 << (c % 64);
        v
    }

    /// `v` reduced by the moves' rows until no pivot coordinate is left.
    fn reduce(&self, mut v: Vec<u64>) -> Vec<u64> {
        for b in 0..64 * self.words {
            if v[b / 64] >> (b % 64) & 1 == 1 {
                if let Some(row) = &self.pivot_row[b] {
                    for (a, x) in v.iter_mut().zip(row) {
                        *a ^= x;
                    }
                }
            }
        }
        v
    }

    /// `v` in the quotient by the moves' span, `words` words: reduced, then the coordinates that are not pivots.
    fn project(&self, v: Vec<u64>, words: usize) -> Vec<u64> {
        let r = self.reduce(v);
        let mut out = vec![0u64; words];
        for c in 0..64 * self.words {
            if r[c / 64] >> (c % 64) & 1 == 1 {
                let q = self.quotient[c];
                debug_assert_ne!(q, NONE, "a pivot survived the reduction");
                out[q as usize / 64] |= 1 << (q % 64);
            }
        }
        out
    }
}

impl Gf2Graph {
    /// The matroids of `state` (the root of a search, or any state: its frames must all be winnable) and its
    /// candidates `cands`.  Candidates that are not factors of their home frame, or are loops of either matroid
    /// (no promotable set holds them), are left out.
    pub fn new<const W: usize>(model: &Model, state: &FrameSet<W>, cands: &Bits<W>) -> Self {
        let frames = state.frames();
        let max = frames.iter().flat_map(|f| f.factors.max().into_iter().chain(f.moves.max())).max().unwrap_or(0);
        let max = max.max(cands.max().unwrap_or(0));
        let spaces: Vec<FrameSpace> = frames
            .iter()
            .map(|f| FrameSpace::new(model, &f.factors, &f.moves, max).expect("a winnable frame"))
            .collect();
        let ranks: Vec<usize> = spaces.iter().map(|s| s.rank).collect();
        let words = ranks.iter().copied().max().unwrap_or(0).div_ceil(64).max(1);
        let blocks = [frames.len().div_ceil(2), frames.len() / 2];
        let mut g = Gf2Graph {
            elems: Vec::new(),
            index: vec![NONE; max as usize + 1],
            words,
            blocks,
            block: [Vec::new(), Vec::new()],
            vecs: [Vec::new(), Vec::new()],
            block_elems: blocks.map(|b| vec![Vec::new(); b]),
            weight: Vec::new(),
            ranks,
        };
        for x in cands.iter() {
            let h = model.home(x);
            if h == 0 || h >= frames.len() || !frames[h].factors.contains(x) {
                continue;
            }
            let out = spaces[h].project(spaces[h].unit(x), words);
            let into = spaces[h - 1].project(spaces[h - 1].vector(model, x), words);
            if out.iter().all(|&w| w == 0) || into.iter().all(|&w| w == 0) {
                continue;
            }
            // Frame f is block f / 2 of A (f even) or of B (f odd).
            let e = g.elems.len() as u32;
            for (f, v) in [(h, out), (h - 1, into)] {
                let m = f % 2;
                g.block[m].push((f / 2) as u8);
                g.vecs[m].extend_from_slice(&v);
                g.block_elems[m][f / 2].push(e);
            }
            g.index[x as usize] = e;
            g.elems.push(x);
            g.weight.push(i64::from(x));
        }
        g
    }

    /// The number of elements.
    pub fn len(&self) -> usize {
        self.elems.len()
    }

    pub fn is_empty(&self) -> bool {
        self.elems.is_empty()
    }

    /// Each frame's rank: its freedoms, the dimension of its quotient.
    pub fn ranks(&self) -> &[usize] {
        &self.ranks
    }

    /// Words of a block vector.
    pub fn words(&self) -> usize {
        self.words
    }

    /// The element of the number `x`, if it is one.
    #[inline]
    fn element(&self, x: u32) -> Option<u32> {
        self.index.get(x as usize).copied().filter(|&e| e != NONE)
    }
}

// ---------------------------------------------------------------------------------------------------------
// The bound along a search's path

/// The most block words supported: ranks up to 192, which every board up to 4095 stays within (169 at 4094).
pub const MAX_WORDS: usize = 3;

/// The GF(2) bound of the nodes on a search's path, level for level with the search's stack (see the module docs):
/// [`reset`](Self::reset) for a unit's node, [`push_all`](Self::push_all) for a child, [`pop`](Self::pop),
/// [`remove_candidate`](Self::remove_candidate) when a node goes on without a candidate, and
/// [`evaluate`](Self::evaluate) for the bound of the top level.  Its block vectors are as wide as the root's
/// largest frame rank needs.
pub struct Gf2Bound<'g, const W: usize>(Engine<'g, W>);

enum Engine<'g, const W: usize> {
    R1(Levels<'g, W, 1>),
    R2(Levels<'g, W, 2>),
    R3(Levels<'g, W, 3>),
}

macro_rules! dispatch {
    ($self:expr, $l:ident => $body:expr) => {
        match &mut $self.0 {
            Engine::R1($l) => $body,
            Engine::R2($l) => $body,
            Engine::R3($l) => $body,
        }
    };
}

/// What [`Gf2Bound::evaluate`] found about the top level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Eval<const W: usize> {
    /// The bound: the weight of the heaviest common independent set of at most `cap` live candidates.
    pub value: u64,
    /// That set.
    pub chosen: Bits<W>,
    /// Its members without which the bound (from the weight split) is below `need`; empty unless asked for.
    pub forced: Bits<W>,
}

/// Counters of a [`Gf2Bound`]'s work.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gf2Stats {
    /// Levels solved (augmentations run), levels whose heaviest set followed from the parent's, augmentations,
    /// Bellman–Ford rounds, weight splits.
    pub solves: u64,
    pub inherited: u64,
    pub augmentations: u64,
    pub rounds: u64,
    pub splits: u64,
    /// Blocks of the tableau rebuilt after a path whose changes were not all pivots.
    pub rebuilds: u64,
    /// The weight splits' Bellman–Ford rounds.
    pub split_rounds: u64,
    /// Of the solves and augmentations, those of [`Gf2Bound::value_excluding`] (strong branching).
    pub excluded_solves: u64,
    pub excluded_augmentations: u64,
    /// Of those, the ones a greedy lower bound settled with no solve, and the ones whose solve stopped early.
    pub greedy_stops: u64,
    pub excluded_stopped: u64,
}

impl<'g, const W: usize> Gf2Bound<'g, W> {
    /// A solver for the states of `graph`'s search, positioned at the root with no candidates.
    pub fn new(graph: &'g Gf2Graph) -> Self {
        Gf2Bound(match graph.words {
            1 => Engine::R1(Levels::new(graph)),
            2 => Engine::R2(Levels::new(graph)),
            3 => Engine::R3(Levels::new(graph)),
            w => panic!("a frame of rank above {} ({w} words) is not supported", 64 * MAX_WORDS),
        })
    }

    /// Starts over with level 0 at the state reached from the root by promoting `promoted`, with the candidates
    /// `cands`.
    pub fn reset(&mut self, promoted: &Bits<W>, cands: &Bits<W>) {
        dispatch!(self, l => l.reset(promoted, cands))
    }

    /// Pushes the child of the top level made by promoting `added` (each a candidate of the top level, or removed
    /// from them since), with the candidates `child_cands`.
    pub fn push_all(&mut self, added: &Bits<W>, child_cands: &Bits<W>) {
        dispatch!(self, l => l.push_all(added, child_cands))
    }

    /// Returns to the parent of the top level.
    pub fn pop(&mut self) {
        dispatch!(self, l => l.pop())
    }

    /// The top level goes on without the candidate `x`.
    pub fn remove_candidate(&mut self, x: u32) {
        dispatch!(self, l => l.remove_candidate(x))
    }

    /// The bound of the top level with at most `cap` promotions, its heaviest set, and with `need` (if any) its
    /// forced members.
    pub fn evaluate(&mut self, cap: u32, need: Option<u64>) -> Eval<W> {
        dispatch!(self, l => l.evaluate(cap as usize, need))
    }

    /// The bound of the top level without the candidate `x` (its heaviest set restarted from the history), with at
    /// most `cap` promotions, or some value of at least `stop` if it is at least `stop` (the augmentations stop
    /// there); the top level is unchanged.
    pub fn value_excluding(&mut self, x: u32, cap: u32, stop: u64) -> u64 {
        dispatch!(self, l => l.value_excluding(x, cap as usize, stop))
    }

    /// Keeps the level that the last [`value_excluding`](Self::value_excluding) solved (to the end, not stopped), so
    /// that if the top level goes on without that candidate next ([`remove_candidate`](Self::remove_candidate)),
    /// with nothing else done to the stack meanwhile, it becomes the top level, solved.
    pub fn keep_excluded(&mut self) {
        dispatch!(self, l => l.keep_excluded())
    }

    /// The weight split's bound on the top level without its member `x`, if the last evaluation asked for forced
    /// members and the level has a split.
    pub fn split_without(&mut self, x: u32) -> Option<i64> {
        dispatch!(self, l => {
            let level = &l.levels[l.depth];
            level.has_split.then(|| level.split.iter().find(|p| p.0 == x).map(|p| p.1)).flatten()
        })
    }

    pub fn stats(&self) -> Gf2Stats {
        match &self.0 {
            Engine::R1(l) => l.stats,
            Engine::R2(l) => l.stats,
            Engine::R3(l) => l.stats,
        }
    }
}

/// One node's level: its matroids (the root's contracted by its promotions), its live candidates, and what is
/// known of its heaviest set.
#[derive(Clone)]
struct Level<const W: usize, const R: usize> {
    vec: [Vec<Vector<R>>; 2],
    /// The live candidates (elements only, loops left out).
    live: Bits<W>,
    /// Dominant sets of this level's problem by increasing size (see the module docs).  When `solved`, the last
    /// is the heaviest set under `cap`.
    history: Vec<Bits<W>>,
    solved: bool,
    /// The size limit of the last solve, and whether it ended because no augmentation gained (rather than at
    /// the limit): then the last entry of the history is the heaviest set of all sizes.
    cap: usize,
    settled: bool,
    value: u64,
    /// The bounds without each member of the heaviest set, from the weight split, once asked for.
    split: Vec<(u32, i64)>,
    has_split: bool,
    /// The workspace's solve count when this level was last solved, unchanged since (0 for none): if the workspace
    /// has made no solve since, it still holds the level's tableau.
    meet_solve: u64,
}

impl<const W: usize, const R: usize> Level<W, R> {
    fn new(n: usize) -> Self {
        Level {
            vec: [vec![[0; R]; n], vec![[0; R]; n]],
            live: Bits::EMPTY,
            history: Vec::new(),
            solved: false,
            cap: 0,
            settled: false,
            value: 0,
            split: Vec::new(),
            has_split: false,
            meet_solve: 0,
        }
    }

    fn copy_from(&mut self, other: &Self) {
        for m in 0..2 {
            self.vec[m].copy_from_slice(&other.vec[m]);
        }
        self.live = other.live;
        self.history.clear();
        self.solved = false;
        self.settled = false;
        self.has_split = false;
        self.meet_solve = 0;
    }
}

struct Levels<'g, const W: usize, const R: usize> {
    graph: &'g Gf2Graph,
    /// The root's vectors.
    root: [Vec<Vector<R>>; 2],
    levels: Vec<Level<W, R>>,
    depth: usize,
    /// A level for [`value_excluding`](Self::value_excluding), and the candidate it was solved without (to the end).
    scratch: Level<W, R>,
    scratch_for: Option<u32>,
    /// A level kept by [`keep_excluded`](Self::keep_excluded), for the top level at the depth without the candidate.
    kept: Level<W, R>,
    kept_for: Option<(usize, u32)>,
    meet: Meet<R>,
    live_idx: Vec<u32>,
    start_idx: Vec<u32>,
    stats: Gf2Stats,
}

impl<'g, const W: usize, const R: usize> Levels<'g, W, R> {
    fn new(graph: &'g Gf2Graph) -> Self {
        let n = graph.len();
        let root = [0, 1].map(|m| {
            (0..n)
                .map(|i| {
                    let mut v = [0u64; R];
                    v[..graph.words].copy_from_slice(&graph.vecs[m][i * graph.words..(i + 1) * graph.words]);
                    v
                })
                .collect()
        });
        Levels {
            graph,
            root,
            levels: vec![Level::new(n)],
            depth: 0,
            scratch: Level::new(n),
            scratch_for: None,
            kept: Level::new(n),
            kept_for: None,
            meet: Meet::new(n, graph.blocks),
            live_idx: Vec::with_capacity(n),
            start_idx: Vec::with_capacity(n),
            stats: Gf2Stats::default(),
        }
    }

    /// The live elements of `cands`.
    fn live_of(&self, cands: &Bits<W>) -> Bits<W> {
        cands.iter().filter(|&x| self.graph.element(x).is_some()).collect()
    }

    fn reset(&mut self, promoted: &Bits<W>, cands: &Bits<W>) {
        self.depth = 0;
        self.kept_for = None;
        let live = self.live_of(cands);
        let level = &mut self.levels[0];
        for m in 0..2 {
            level.vec[m].copy_from_slice(&self.root[m]);
        }
        level.live = live;
        level.history.clear();
        level.solved = false;
        level.settled = false;
        level.has_split = false;
        level.meet_solve = 0;
        for x in promoted.iter() {
            contract(self.graph, level, x);
        }
    }

    fn push_all(&mut self, added: &Bits<W>, child_cands: &Bits<W>) {
        let d = self.depth;
        self.kept_for = None;
        if self.levels.len() == d + 1 {
            let n = self.graph.len();
            self.levels.push(Level::new(n));
        }
        let child_live = self.live_of(child_cands);
        let (lower, upper) = self.levels.split_at_mut(d + 1);
        let (parent, child) = (&lower[d], &mut upper[0]);
        child.copy_from(parent);
        child.live = child_live;
        for x in added.iter() {
            contract(self.graph, child, x);
        }
        // The parent's dominant sets that hold every promoted candidate and no other dropped one, less the
        // promoted (see the module docs, "Warm starts").
        let kept = child.live | *added;
        let keeps = |set: &Bits<W>| added.is_subset(set) && set.is_subset(&kept);
        for set in parent.history.iter().filter(|set| keeps(set)) {
            child.history.push(*set - *added);
        }
        // The parent's heaviest set of all sizes, kept: the child's is that less the promoted.
        let whole = parent.solved && parent.settled && parent.history.last().is_some_and(keeps);
        if whole {
            child.solved = true;
            child.settled = true;
            child.cap = parent.cap.saturating_sub(added.len() as usize);
            child.value = parent.value - added.sum();
            self.stats.inherited += 1;
        }
        self.depth = d + 1;
    }

    fn pop(&mut self) {
        assert!(self.depth > 0, "pop without push");
        self.kept_for = None;
        self.depth -= 1;
    }

    fn remove_candidate(&mut self, x: u32) {
        if self.kept_for.take() == Some((self.depth, x)) {
            std::mem::swap(&mut self.levels[self.depth], &mut self.kept);
            return;
        }
        let level = &mut self.levels[self.depth];
        if !level.live.contains(x) {
            return;
        }
        level.live.remove(x);
        let held = level.history.last().is_some_and(|last| last.contains(x));
        level.history.retain(|set| !set.contains(x));
        if held || !level.solved {
            level.solved = false;
            level.settled = false;
        }
        // Deleting a candidate outside the heaviest set leaves it the heaviest: the history still ends with it.
        level.has_split = false;
        level.meet_solve = 0;
    }

    fn evaluate(&mut self, cap: usize, need: Option<u64>) -> Eval<W> {
        let d = self.depth;
        let level = &mut self.levels[d];
        let fits = level.solved && level.history.last().is_some_and(|last| last.len() as usize <= cap);
        // A heaviest set under a larger limit that fits a smaller one is the heaviest under it too, and one of all
        // sizes is the heaviest under any limit it fits.
        let ok = fits && (cap <= level.cap || level.settled);
        if !ok {
            let (meet, live_idx, start_idx) = (&mut self.meet, &mut self.live_idx, &mut self.start_idx);
            solve(self.graph, level, meet, live_idx, start_idx, cap, u64::MAX, &mut self.stats);
        }
        let level = &mut self.levels[d];
        let chosen = level.history.last().copied().unwrap_or(Bits::EMPTY);
        let mut forced = Bits::EMPTY;
        if let Some(need) = need {
            if !level.has_split && level.settled {
                split(self.graph, level, &mut self.meet, &mut self.live_idx, &mut self.start_idx, &mut self.stats);
            }
            if level.has_split {
                for &(x, bound) in &level.split {
                    if bound < need as i64 {
                        forced.insert(x);
                    }
                }
            }
        }
        Eval { value: level.value, chosen, forced }
    }

    fn value_excluding(&mut self, x: u32, cap: usize, stop: u64) -> u64 {
        let level = &self.levels[self.depth];
        // A lower bound first: the level's heaviest set without x, completed greedily.  If it reaches `stop`, so
        // does the bound without x, and no solve is needed.
        if stop != u64::MAX && level.solved {
            if let Some(&last) = level.history.last() {
                let graph = self.graph;
                self.live_idx.clear();
                self.live_idx.extend(level.live.iter().filter(|&y| y != x).map(|y| graph.index[y as usize]));
                self.start_idx.clear();
                self.start_idx.extend(last.iter().filter(|&y| y != x).map(|y| graph.index[y as usize]));
                let problem = Problem {
                    block: [&graph.block[0], &graph.block[1]],
                    vector: [&level.vec[0], &level.vec[1]],
                    weight: &graph.weight,
                };
                let (lower, size) = self.meet.greedy_from(&problem, &self.live_idx, &self.start_idx);
                // A greedy set larger than the limit proves nothing about the limited bound.
                if lower as u64 >= stop && size <= cap {
                    self.stats.greedy_stops += 1;
                    self.scratch_for = None;
                    return lower as u64;
                }
            }
        }
        self.scratch.copy_from(level);
        self.scratch.history.extend(level.history.iter().filter(|set| !set.contains(x)));
        self.scratch.live.remove(x);
        let (s, meet, live_idx, start_idx) =
            (&mut self.scratch, &mut self.meet, &mut self.live_idx, &mut self.start_idx);
        let before = self.stats.augmentations;
        solve(self.graph, s, meet, live_idx, start_idx, cap, stop, &mut self.stats);
        self.stats.excluded_solves += 1;
        self.stats.excluded_augmentations += self.stats.augmentations - before;
        self.stats.excluded_stopped += u64::from(!self.scratch.solved);
        self.scratch_for = self.scratch.solved.then_some(x);
        self.scratch.value
    }

    fn keep_excluded(&mut self) {
        if let Some(x) = self.scratch_for.take() {
            std::mem::swap(&mut self.scratch, &mut self.kept);
            self.kept_for = Some((self.depth, x));
        }
    }
}

/// Promotes the element `x` of `level`: contracts both matroids by its vectors and takes it out of the live set;
/// candidates whose vectors become zero are loops of the contraction and leave the live set too.
fn contract<const W: usize, const R: usize>(graph: &Gf2Graph, level: &mut Level<W, R>, x: u32) {
    let e = graph.element(x).expect("a promoted candidate is an element") as usize;
    level.live.remove(x);
    for m in 0..2 {
        let v = level.vec[m][e];
        let p = lowest(&v).expect("a promoted candidate is not a loop");
        for &y in &graph.block_elems[m][usize::from(graph.block[m][e])] {
            let y = y as usize;
            if bit(&level.vec[m][y], p) {
                xor(&mut level.vec[m][y], &v);
                if is_zero(&level.vec[m][y]) {
                    level.live.remove(graph.elems[y]);
                }
            }
        }
        level.vec[m][e] = [0; R];
    }
}

/// Solves `level` under the size limit `cap`, from the last entry of its history that fits it, or until its value
/// reaches `stop` (then it is left unsolved, its value what was reached and its history the sets passed through).
#[allow(clippy::too_many_arguments)]
fn solve<const W: usize, const R: usize>(
    graph: &Gf2Graph,
    level: &mut Level<W, R>,
    meet: &mut Meet<R>,
    live_idx: &mut Vec<u32>,
    start_idx: &mut Vec<u32>,
    cap: usize,
    stop: u64,
    stats: &mut Gf2Stats,
) {
    live_idx.clear();
    live_idx.extend(level.live.iter().map(|x| graph.index[x as usize]));
    while level.history.last().is_some_and(|last| last.len() as usize > cap) {
        level.history.pop();
    }
    start_idx.clear();
    if let Some(last) = level.history.last() {
        start_idx.extend(last.iter().map(|x| graph.index[x as usize]));
    }
    let problem = Problem {
        block: [&graph.block[0], &graph.block[1]],
        vector: [&level.vec[0], &level.vec[1]],
        weight: &graph.weight,
    };
    let before = meet.augmentations;
    let history = &mut level.history;
    let mut first = true;
    let stop = i64::try_from(stop).unwrap_or(i64::MAX);
    meet.solve(&problem, live_idx, start_idx, cap, stop, |set| {
        let bits: Bits<W> = set.iter().map(|&e| graph.elems[e as usize]).collect();
        if first {
            // The start: entries at least as large are off its chain.
            first = false;
            let size = bits.len();
            while history.last().is_some_and(|last| last.len() >= size) {
                history.pop();
            }
        }
        history.push(bits);
    });
    stats.solves += 1;
    stats.augmentations += meet.augmentations - before;
    stats.rounds = meet.rounds;
    stats.rebuilds = meet.rebuilds;
    stats.split_rounds = meet.split_rounds;
    level.solved = !meet.stopped;
    level.settled = meet.settled;
    level.meet_solve = meet.solves;
    level.cap = cap;
    level.value = meet.value as u64;
    level.has_split = false;
}

/// The bounds without each member of `level`'s heaviest set (settled), from a weight split.  Unless the workspace
/// still holds the level's last solve, the level's heaviest set is solved again from itself first (the tableau built
/// and one shortest path, no augmentation).
fn split<const W: usize, const R: usize>(
    graph: &Gf2Graph,
    level: &mut Level<W, R>,
    meet: &mut Meet<R>,
    live_idx: &mut Vec<u32>,
    start_idx: &mut Vec<u32>,
    stats: &mut Gf2Stats,
) {
    let chosen = level.history.last().copied().unwrap_or(Bits::EMPTY);
    live_idx.clear();
    live_idx.extend(level.live.iter().map(|x| graph.index[x as usize]));
    start_idx.clear();
    start_idx.extend(chosen.iter().map(|x| graph.index[x as usize]));
    let problem = Problem {
        block: [&graph.block[0], &graph.block[1]],
        vector: [&level.vec[0], &level.vec[1]],
        weight: &graph.weight,
    };
    if level.meet_solve != meet.solves {
        meet.solve(&problem, live_idx, start_idx, usize::MAX, i64::MAX, |_| {});
        debug_assert_eq!(meet.value as u64, level.value, "the heaviest set solved again changed");
    }
    stats.splits += 1;
    let out = &mut level.split;
    out.clear();
    if meet.settled {
        meet.split_bounds(&problem, live_idx, |e, bound| out.push((graph.elems[e as usize], bound)));
    }
    stats.split_rounds = meet.split_rounds;
    level.has_split = true;
}

#[cfg(test)]
mod tests;
