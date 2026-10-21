//! The GF(2) bound: an upper bound on the sum of the promotions a search state can still make, from two linear
//! matroids over GF(2) in which every promotable set is independent (tactic `gf2`).  It is never above the matching
//! bound ([`crate::matchbound`]) and usually far below it; `docs/THEORY.md`, Theorem 11, has the proof, and
//! `solver/notebook/search/matroid-intersection-bounds-2026-10/README.md` and
//! `solver/notebook/search/the-gf2-bound-in-the-search-2026-10/README.md`, the measurements.
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
//! number is the frame's freedoms.  A candidate touches its home frame and the one in front of it, one of even index
//! and one of odd, so the even frames' matroids are the blocks of one matroid `A` on the candidates and the odd frames'
//! of another, `B`.  Every promotable set is independent in both, and the heaviest common independent set (weighted
//! matroid intersection, [`meet`]) bounds the best completion, with at most `max_add` members.
//!
//! [`Gf2Graph`] builds the root's matroids once per search; [`Gf2Bound`] keeps one level per node on the search's
//! path, like [`MatchBound`](crate::matchbound::MatchBound).
//!
//! # One-sided candidates
//!
//! In the extended problem ([`crate::frames::build_extended`]) a one-sided candidate `s` of frame `j` is a mover of
//! frame `j` and touches no other frame (docs/THEORY.md, Theorem 11 for the extended problem).  So its element
//! has the vector of its maximal factors in the matroid of frame `j`, and in the other matroid it is a *coloop*: it
//! constrains nothing there.  The coloops of a matroid are put in *free blocks* after its frames' blocks, each element
//! its own unit vector, at most `64·words` of them a block, so that a game with hundreds of one-sided candidates needs
//! no wider vectors than its frames do.  A free block is no frame: nothing reads a frame from a block index (the frames'
//! ranks are [`Gf2Graph::ranks`], the free blocks have none), and the intersection ([`meet`]) knows only blocks.
//! Contracting by a coloop clears its own coordinate, which no other element has.  Without one-sided candidates there
//! are no free blocks, and the matroids are those above.
//!
//! # Contraction along the path
//!
//! A state reached by promoting `x` has `x` among its moves: its matroids are the parent's contracted by `x` in both
//! of its frames (each vector of the frame reduced by `x`'s, one pivot), and a candidate that the state's reduction
//! consumed becomes a loop (a zero vector) there.  (Checked against the matroids built from the state itself at
//! every node of the probe's fixtures by `nodes` of the example `matroid`, removed from master: it builds at commit
//! 8753189.)  Candidates dropped from a node are deleted:
//! they leave its *live* set, and so do the loops, for the node and every node below it: no common independent set
//! holds a loop, and the searches, tableaux and splits of a level would only carry it.
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
//! # Forced members, and the bound without a member
//!
//! With the heaviest set `I*` of all sizes, [`meet::Meet::split_bounds`] bounds the problem without each member of
//! `I*` from a weight split, in one pass; a member without which the bound is below what the node needs is forced:
//! every completion that beats the incumbent promotes it.  For the members the split leaves free near the need,
//! `I*`'s exchange graph decides exactly (`Levels::force_by_cycles`), and strong branching reads the bound without
//! each member of a conflict off it ([`Gf2Bound::certified_excluding`]), solving only where it is not certified (see
//! [`meet`], "The heaviest set without a member").  A level's heaviest set may come from such a certificate too: the
//! node's without the member it branched on, and a child's that lost one member of its parent's heaviest set
//! (`certify_child`).

pub mod meet;

use crate::bits::Bits;
use crate::model::{FrameGraph, FrameSet};
use meet::{bit, is_zero, lowest, xor, Cycles, Meet, Problem, Snapshot, SplitStart, Vector};

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
    /// Blocks in `A` (frames 0, 2, 4, …) and in `B` (frames 1, 3, 5, …), each followed by its free blocks (the
    /// coloops of the one-sided candidates; none without them).
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
    fn new<const W: usize>(model: &FrameGraph, factors: &Bits<W>, moves: &Bits<W>, max: u32) -> Result<Self, String> {
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
    fn vector(&self, model: &FrameGraph, u: u32) -> Vec<u64> {
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
    /// (no promotable set holds them), are left out.  A one-sided candidate of the model (the extended problem's) is
    /// an element whose vector in the matroid of its own frame is its maximal factors there, as for a number promoted
    /// into that frame, and which is a coloop of the other matroid: its own coordinate in a *free block* of that
    /// matroid, after the frames' blocks, each free block holding `64·words` of them (see the module docs).
    pub fn new<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, cands: &Bits<W>) -> Self {
        let frames = state.frames();
        let max = frames.iter().flat_map(|f| f.factors.max().into_iter().chain(f.moves.max())).max().unwrap_or(0);
        let max = max.max(cands.max().unwrap_or(0));
        let spaces: Vec<FrameSpace> = frames
            .iter()
            .map(|f| FrameSpace::new(model, &f.factors, &f.moves, max).expect("a winnable frame"))
            .collect();
        let ranks: Vec<usize> = spaces.iter().map(|s| s.rank).collect();
        let words = ranks.iter().copied().max().unwrap_or(0).div_ceil(64).max(1);
        // Frame f is block f / 2 of A (f even) or of B (f odd); the free blocks come after the frames' in each.
        let frame_blocks = [frames.len().div_ceil(2), frames.len() / 2];
        let mut g = Gf2Graph {
            elems: Vec::new(),
            index: vec![NONE; max as usize + 1],
            words,
            blocks: frame_blocks,
            block: [Vec::new(), Vec::new()],
            vecs: [Vec::new(), Vec::new()],
            block_elems: frame_blocks.map(|b| vec![Vec::new(); b]),
            weight: Vec::new(),
            ranks,
        };
        let zero = |v: &[u64]| v.iter().all(|&w| w == 0);
        let mut coloops = [0usize; 2];
        for x in cands.iter() {
            // The element's block and vector in each matroid, by matroid.
            let parts = if let Some(j) = model.own(x) {
                let into = spaces[j].project(spaces[j].vector(model, x), words);
                if zero(&into) {
                    continue;
                }
                let m = 1 - j % 2;
                let (k, per_block) = (coloops[m], 64 * words);
                coloops[m] += 1;
                let mut unit = vec![0u64; words];
                unit[k % per_block / 64] |= 1 << (k % 64);
                let free = frame_blocks[m] + k / per_block;
                if free == g.block_elems[m].len() {
                    g.block_elems[m].push(Vec::new());
                }
                [(j % 2, j / 2, into), (m, free, unit)]
            } else {
                let h = model.home(x);
                if h == 0 || h >= frames.len() || !frames[h].factors.contains(x) {
                    continue;
                }
                let out = spaces[h].project(spaces[h].unit(x), words);
                let into = spaces[h - 1].project(spaces[h - 1].vector(model, x), words);
                if zero(&out) || zero(&into) {
                    continue;
                }
                [(h % 2, h / 2, out), ((h - 1) % 2, (h - 1) / 2, into)]
            };
            let e = g.elems.len() as u32;
            for (m, block, v) in parts {
                g.block[m].push(u8::try_from(block).expect("at most 255 blocks"));
                g.vecs[m].extend_from_slice(&v);
                g.block_elems[m][block].push(e);
            }
            g.index[x as usize] = e;
            g.elems.push(x);
            g.weight.push(i64::from(x));
        }
        g.blocks = g.block_elems.each_ref().map(Vec::len);
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

    /// The same matroids with block vectors of `words` words (at least [`words`](Self::words)), the words above the
    /// graph's zero: every bound, and so every search, is the same, but [`Gf2Bound::new`] runs the engine of
    /// `words` words on it.  For tests, to run the engines of four and five words on boards that need fewer.
    pub fn padded(&self, words: usize) -> Gf2Graph {
        assert!(words >= self.words, "{words} words are fewer than the graph's {}", self.words);
        let vecs = self.vecs.each_ref().map(|vecs| {
            vecs.chunks_exact(self.words)
                .flat_map(|v| v.iter().copied().chain(std::iter::repeat_n(0, words - self.words)))
                .collect()
        });
        Gf2Graph { vecs, words, ..self.clone() }
    }

    /// The same matroids with each element `x` weighing `weight(x)` instead of its number, for bounds that are not the
    /// search's (a Lagrangian bound, whose multipliers lower the weights of the members of nogoods); with the labels,
    /// because the solver wants its elements by increasing weight and the numbers it is asked about to be those
    /// elements.  An element whose weight is not positive is left out (a heaviest common independent set never needs
    /// one), and the others are relabelled `1, 2, …` by increasing weight (ties by number): the graph is of those
    /// labels, and `labels[k - 1]` is the number of label `k`.  Run [`Gf2Bound`] on it with the candidates
    /// `1..=labels.len()`, and map the sets it chooses back through `labels`.
    pub fn reweighted(&self, weight: impl Fn(u32) -> i64) -> (Gf2Graph, Vec<u32>) {
        let mut kept: Vec<(i64, u32, usize)> = (0..self.elems.len())
            .filter_map(|e| Some((weight(self.elems[e]), self.elems[e], e)).filter(|&(w, _, _)| w > 0))
            .collect();
        kept.sort_unstable();
        let count = kept.len() as u32;
        let mut g = Gf2Graph {
            elems: (1..=count).collect(),
            index: vec![NONE; count as usize + 1],
            words: self.words,
            blocks: self.blocks,
            block: [Vec::new(), Vec::new()],
            vecs: [Vec::new(), Vec::new()],
            block_elems: self.block_elems.each_ref().map(|blocks| vec![Vec::new(); blocks.len()]),
            weight: kept.iter().map(|&(w, _, _)| w).collect(),
            ranks: self.ranks.clone(),
        };
        for (k, &(_, _, e)) in kept.iter().enumerate() {
            for m in 0..2 {
                let block = self.block[m][e];
                g.block[m].push(block);
                g.vecs[m].extend_from_slice(&self.vecs[m][e * self.words..(e + 1) * self.words]);
                g.block_elems[m][usize::from(block)].push(k as u32);
            }
            g.index[k + 1] = k as u32;
        }
        (g, kept.iter().map(|&(_, x, _)| x).collect())
    }

    /// The element of the number `x`, if it is one.
    #[inline]
    fn element(&self, x: u32) -> Option<u32> {
        self.index.get(x as usize).copied().filter(|&e| e != NONE)
    }
}

// ---------------------------------------------------------------------------------------------------------
// The bound along a search's path

/// The window of the tactic `force_window` ([`ForceWindow::Within`]) that was its default until 2026-10-16, when the
/// default became [`ForceWindow::All`].  The split's bound is often far above the exact one (half the members it
/// leaves free that the search finds forced are within 15 of `need`, nine in ten within 53, at 1215), and the searches
/// of the members it leaves free cost more than they save while the trees are small: over the quick set of one-thread
/// proofs (936-1350) a window of 25 costs 20.6 G cycles, 40 18.2, 60 17.3, 80 16.9, 120 17.6, no window 18.0 (without
/// the search 26.4; nodes 217,374 then 132,049, 108,856, 96,283, 89,751, 86,481, 85,349).  At the end of the chain
/// the trees are dear enough that every search pays
/// (solver/notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md).  A checkpoint's header of
/// version 5 implies this window.
pub const LEGACY_FORCE_WINDOW: u32 = 80;

/// Which members of the heaviest set that the weight split leaves free [`Gf2Bound::evaluate`] searches for being
/// forced after all (`Levels::force_by_cycles`; the tactic `force_window`): any choice is sound, since a member is
/// called forced only where the search shows it.  The default is [`All`](ForceWindow::All).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ForceWindow {
    /// Those whose split bound is less than this far above `need` (0: none).
    Within(u32),
    /// Those whose split bound is above `need` by less than this percentage of the node's slack, the bound less
    /// `need`.
    Share(u8),
    /// All of them.
    #[default]
    All,
}

impl ForceWindow {
    /// Whether a member whose split bound is `above` more than `need`, at a node whose bound is `slack` more than
    /// `need`, is searched.
    #[inline]
    fn admits(self, above: i64, slack: i64) -> bool {
        match self {
            ForceWindow::Within(window) => above < i64::from(window),
            ForceWindow::Share(percent) => above * 100 < i64::from(percent) * slack,
            ForceWindow::All => true,
        }
    }
}

/// The most block words supported: ranks up to 320, which every board up to 8191 stays within (all checked in
/// 2026-10: 314 at 8174; four words from n = 4740, five from 6392).
pub const MAX_WORDS: usize = 5;

/// The most block words at set widths of at most 64 words, the boards up to 4095 (ranks up to 169, at 4095).  Only the
/// wider sets have the engines of four and five words ([`Gf2Bound::WIDE`]), so that the narrower widths are built as
/// before.
const NARROW_WORDS: usize = 3;

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
    R4(Levels<'g, W, 4>),
    R5(Levels<'g, W, 5>),
}

/// Runs `$body` on the engine's levels `$l`.  The arms of four and five words test the constant [`Gf2Bound::WIDE`],
/// so that the code for them is generated only for the sets wider than 64 words.
macro_rules! dispatch {
    ($self:expr, $l:ident => $body:expr) => {
        match &mut $self.0 {
            Engine::R1($l) => $body,
            Engine::R2($l) => $body,
            Engine::R3($l) => $body,
            Engine::R4($l) => {
                if Self::WIDE {
                    $body
                } else {
                    unreachable!("an engine of four words at {W} words")
                }
            }
            Engine::R5($l) => {
                if Self::WIDE {
                    $body
                } else {
                    unreachable!("an engine of five words at {W} words")
                }
            }
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
    /// Bounds without a member that the heaviest set's exchange graph certified, with no solve
    /// ([`Gf2Bound::certified_excluding`]), and children whose heaviest set their parent's certified.
    pub certified: u64,
    pub certified_children: u64,
    /// Members found forced by `Levels::force_by_cycles` where the split left them free.
    pub forced_by_cycles: u64,
    /// Tableaux of solved levels made again ([`resolve`]): derived from a related one rather than built, of those
    /// from a copy, and the blocks built in them all.
    pub derived: u64,
    pub derived_from_copies: u64,
    pub blocks_built: u64,
}

impl<'g, const W: usize> Gf2Bound<'g, W> {
    /// Whether the sets are wider than 64 words: only then are the engines of four and five words compiled (a branch
    /// on this constant is dropped before code generation where it is false).
    const WIDE: bool = W > 64;

    /// The most block words an engine has at this set width: [`MAX_WORDS`] for the sets wider than 64 words, three for
    /// the others.  A graph that needs more ([`Gf2Graph::words`]) cannot be searched at this width.
    pub const MOST_WORDS: usize = if Self::WIDE { MAX_WORDS } else { NARROW_WORDS };

    /// A solver for the states of `graph`'s search, positioned at the root with no candidates.
    pub fn new(graph: &'g Gf2Graph) -> Self {
        Self::with_words(graph, graph.words)
    }

    /// The bound with block vectors of `words` words, at least the graph's ([`Gf2Graph::words`]): the same bound
    /// with its vectors padded with zeros, which tests use to run the engines of more words than a board needs.
    pub fn with_words(graph: &'g Gf2Graph, words: usize) -> Self {
        assert!(words >= graph.words, "{words} words are fewer than the graph's {}", graph.words);
        let most = Self::MOST_WORDS;
        if words > most {
            panic!("a frame of rank above {} ({words} words) is not supported at a set width of {W} words", 64 * most);
        }
        Gf2Bound(match words {
            1 => Engine::R1(Levels::new(graph)),
            2 => Engine::R2(Levels::new(graph)),
            3 => Engine::R3(Levels::new(graph)),
            _ => Self::wide(graph, words),
        })
    }

    /// The engine of four or five words, for the sets wider than 64 words.
    fn wide(graph: &'g Gf2Graph, words: usize) -> Engine<'g, W> {
        if Self::WIDE {
            match words {
                4 => Engine::R4(Levels::new(graph)),
                _ => Engine::R5(Levels::new(graph)),
            }
        } else {
            unreachable!("an engine of {words} words at {W} words")
        }
    }

    /// Which members the weight split leaves free [`evaluate`](Self::evaluate) searches for being forced (the tactic
    /// `force_window`; [`ForceWindow::default`] until set).
    pub fn set_force_window(&mut self, window: ForceWindow) {
        dispatch!(self, l => l.force_window = window)
    }

    /// Starts over with level 0 at the state reached from the root by promoting `promoted`, with the candidates
    /// `cands`.
    pub fn reset(&mut self, promoted: &Bits<W>, cands: &Bits<W>) {
        dispatch!(self, l => l.reset(promoted, cands))
    }

    /// For a study: the top level's live candidates (its candidates less the loops of either matroid).
    #[cfg(feature = "studies")]
    pub fn live(&mut self) -> Bits<W> {
        dispatch!(self, l => l.levels[l.depth].live)
    }

    /// Pushes the child of the top level made by promoting `added` (each a candidate of the top level, or removed
    /// from them since), with the candidates `child_cands` (among the top level's, less `added`).
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

    /// The bounds of the top level without each candidate of `xs` (members of its heaviest set), with at most `cap`
    /// promotions, read off the exchange graph of its heaviest set where it certifies them, with no solve
    /// ([`meet::Meet::without`]): `out[i]` is the bound without `xs[i]`, or `None` where it is not certified (and
    /// for every one, unless the level is solved and its heaviest set is of all sizes, within `cap`).  Each is what
    /// [`value_excluding`](Self::value_excluding) with no `stop` would return.  The top level is unchanged.
    pub fn certified_excluding(&mut self, xs: &[u32], cap: u32, out: &mut Vec<Option<u64>>) {
        dispatch!(self, l => l.certified_excluding(xs, cap as usize, out))
    }

    /// Keeps the level of the top level without `x` that the last [`certified_excluding`](Self::certified_excluding)
    /// certified (its heaviest set the one it found, of all sizes), as [`keep_excluded`](Self::keep_excluded) keeps
    /// a solved one.
    pub fn keep_certified(&mut self, x: u32) {
        dispatch!(self, l => l.keep_certified(x))
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

    /// In tests: from here on every tableau derived instead of built ([`meet::Meet::derive`]) is checked against the
    /// one a solve builds, and a difference panics.
    #[cfg(test)]
    pub(crate) fn check_derivations(&mut self) {
        dispatch!(self, l => l.meet.check = Some(Box::new(Meet::new(l.graph.len(), l.graph.blocks))))
    }

    /// In tests: the workspace's last search stamp, and setting it (to start just below the counter's wrap).
    #[cfg(test)]
    pub(crate) fn stamp_generation(&mut self) -> u32 {
        dispatch!(self, l => l.meet.generation())
    }

    #[cfg(test)]
    pub(crate) fn set_stamp_generation(&mut self, generation: u32) {
        dispatch!(self, l => l.meet.set_generation(generation))
    }

    /// In tests: the derived tableaux checked so far ([`check_derivations`](Self::check_derivations)).
    #[cfg(test)]
    pub(crate) fn derivations_checked(&mut self) -> u64 {
        dispatch!(self, l => l.meet.checked)
    }

    pub fn stats(&self) -> Gf2Stats {
        match &self.0 {
            Engine::R1(l) => l.stats(),
            Engine::R2(l) => l.stats(),
            Engine::R3(l) => l.stats(),
            Engine::R4(l) => l.stats(),
            Engine::R5(l) => l.stats(),
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
    /// The bounds without each member of the heaviest set, from the weight split, once asked for, and the split
    /// itself (`w1` by element) if one was found.
    split: Vec<(u32, i64)>,
    has_split: bool,
    pot: Vec<i64>,
    has_pot: bool,
    /// A lower bound on what the common independent sets smaller than the heaviest set (of all sizes) lose against
    /// it (0 if nothing more is known; see [`Losses`]).
    loss_floor: i64,
    /// The workspace's solve count when this level was last solved, unchanged since (0 for none): if the workspace
    /// has made no solve since, it still holds the level's tableau.
    meet_solve: u64,
    /// What the level's tableau can be derived from when it is made again ([`resolve`]).
    source: Source<W, R>,
    /// What the level's node promoted from its parent's (nothing at a unit's root).
    added: Bits<W>,
}

/// Where a solved level's tableau can be derived from instead of built ([`meet::Meet::derive`]): the workspace's solve
/// count when it held a related tableau (0 for none), whose problem is the level's before the members `contracted`
/// were promoted; and a copy of the level's own tableau, or of one it was certified from (with nothing contracted
/// since), if `saved`.  Either serves while the level's set is the same and its live elements only lose members.
#[derive(Clone, Default)]
struct Source<const W: usize, const R: usize> {
    held: u64,
    contracted: Bits<W>,
    snapshot: Snapshot<R>,
    saved: bool,
}

impl<const W: usize, const R: usize> Source<W, R> {
    fn clear(&mut self) {
        self.held = 0;
        self.contracted = Bits::EMPTY;
        self.saved = false;
    }
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
            pot: vec![0; n],
            has_pot: false,
            loss_floor: 0,
            meet_solve: 0,
            source: Source::default(),
            added: Bits::EMPTY,
        }
    }

    /// What `other` knows of its heaviest set (its live elements, dominant sets and split), without its vectors:
    /// enough for a child to start from (see [`Levels::push_all`] and [`certify_child`]).
    fn copy_sets_from(&mut self, other: &Self) {
        self.live = other.live;
        self.has_pot = other.has_pot;
        if other.has_pot {
            self.pot.copy_from_slice(&other.pot);
        }
        self.meet_solve = other.meet_solve;
        self.loss_floor = other.loss_floor;
        self.history.clone_from(&other.history);
        self.solved = other.solved;
        self.cap = other.cap;
        self.settled = other.settled;
        self.value = other.value;
        self.source.clear();
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
        self.has_pot = false;
        self.meet_solve = 0;
        self.source.clear();
        self.added = other.added;
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
    /// The top level as it was before it went on without a candidate ([`remove_candidate`](Self::remove_candidate)),
    /// at the depth and for the candidate of `prior_for`: the child pushed next, which promotes that candidate,
    /// starts from its dominant sets (the level without the candidate has none that hold it).
    prior: Level<W, R>,
    prior_for: Option<(usize, u32)>,
    /// The heaviest sets without the members that the last [`certified_excluding`](Self::certified_excluding)
    /// certified, their weights, and the lengths of their cycles.
    certified: Vec<(u32, Bits<W>, u64, i64)>,
    meet: Meet<R>,
    live_idx: Vec<u32>,
    start_idx: Vec<u32>,
    contracted_idx: Vec<u32>,
    stats: Gf2Stats,
    /// Which free members are searched for being forced ([`ForceWindow`]).
    force_window: ForceWindow,
    /// The cycles within the slack that the last search for forced members at each depth found or kept
    /// ([`force_by_cycles`](Self::force_by_cycles)), and scratch for the next search's.
    cycles: Vec<Cycles>,
    next_cycles: Cycles,
    /// The node at each depth (a number given to each unit's root and each child), and the last weight split at each
    /// depth with the node it was for: a node's split starts from its own last one, or else from its parent's.
    nodes: Vec<u64>,
    next_node: u64,
    starts: Vec<(SplitStart, u64)>,
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
            prior: Level::new(n),
            prior_for: None,
            certified: Vec::new(),
            meet: Meet::new(n, graph.blocks),
            live_idx: Vec::with_capacity(n),
            start_idx: Vec::with_capacity(n),
            contracted_idx: Vec::with_capacity(n),
            stats: Gf2Stats::default(),
            force_window: ForceWindow::default(),
            cycles: Vec::new(),
            next_cycles: Cycles::default(),
            nodes: vec![0],
            next_node: 0,
            starts: Vec::new(),
        }
    }

    /// The counters, with the workspace's of the tableaux made again.
    fn stats(&self) -> Gf2Stats {
        Gf2Stats {
            derived: self.meet.derived,
            derived_from_copies: self.meet.restored,
            blocks_built: self.meet.built,
            ..self.stats
        }
    }

    /// The live elements of `cands`.
    fn live_of(&self, cands: &Bits<W>) -> Bits<W> {
        cands.iter().filter(|&x| self.graph.element(x).is_some()).collect()
    }

    fn reset(&mut self, promoted: &Bits<W>, cands: &Bits<W>) {
        self.depth = 0;
        self.next_node += 1;
        self.nodes[0] = self.next_node;
        self.kept_for = None;
        self.prior_for = None;
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
        level.has_pot = false;
        level.meet_solve = 0;
        level.source.clear();
        level.added = Bits::EMPTY;
        for x in promoted.iter() {
            contract(self.graph, level, x);
        }
    }

    fn push_all(&mut self, added: &Bits<W>, child_cands: &Bits<W>) {
        let d = self.depth;
        self.kept_for = None;
        let from_prior = self.prior_for.take().is_some_and(|(depth, x)| depth == d && added.contains(x));
        if self.levels.len() == d + 1 {
            let n = self.graph.len();
            self.levels.push(Level::new(n));
        }
        let (lower, upper) = self.levels.split_at_mut(d + 1);
        let child = &mut upper[0];
        child.copy_from(&lower[d]);
        // The child's candidates are among the top level's, so its live ones are the top level's that it keeps: the
        // loops of the contractions above stay out of it (`contract` leaves those of its own).
        child.live = child.live.intersect(child_cands);
        child.added = *added;
        // The child's problem is the top level's before it went on without the candidate the child promotes (the
        // same vectors), so it starts from that level's dominant sets.
        let parent = if from_prior { &self.prior } else { &lower[d] };
        for x in added.iter() {
            contract(self.graph, child, x);
        }
        #[cfg(debug_assertions)]
        {
            // The candidates that are elements and not loops of the child's matroids (a promoted candidate's vectors
            // are zero too, and it is no candidate of the child).
            let graph = self.graph;
            let elements: Bits<W> = child_cands
                .iter()
                .filter(|&x| {
                    graph
                        .element(x)
                        .is_some_and(|e| !is_zero(&child.vec[0][e as usize]) && !is_zero(&child.vec[1][e as usize]))
                })
                .collect();
            assert_eq!(child.live, elements, "a child's live candidates are not the elements of its candidates");
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
        // The workspace's tableau, if it is the parent's, is the child's to derive its own from (the parent's
        // problem with the promoted contracted, its set flipped along a cycle if the child is certified).
        let held = if parent.meet_solve == self.meet.solves { parent.meet_solve } else { 0 };
        if whole {
            child.solved = true;
            child.settled = true;
            child.cap = parent.cap.saturating_sub(added.len() as usize);
            child.value = parent.value - added.sum();
            // The child's smaller sets, with the promoted, are smaller sets of the parent's.
            child.loss_floor = parent.loss_floor;
            (child.source.held, child.source.contracted) = (held, *added);
            self.stats.inherited += 1;
        } else if certify_child(self.graph, &lower[d], parent, child, added, &mut self.meet, &mut self.live_idx) {
            (child.source.held, child.source.contracted) = (held, *added);
            self.stats.certified_children += 1;
        }
        self.depth = d + 1;
        self.next_node += 1;
        if self.nodes.len() == d + 1 {
            self.nodes.push(0);
        }
        self.nodes[d + 1] = self.next_node;
    }

    fn pop(&mut self) {
        assert!(self.depth > 0, "pop without push");
        self.kept_for = None;
        self.prior_for = None;
        self.depth -= 1;
    }

    fn remove_candidate(&mut self, x: u32) {
        self.prior_for = Some((self.depth, x));
        if self.kept_for.take() == Some((self.depth, x)) {
            std::mem::swap(&mut self.levels[self.depth], &mut self.kept);
            std::mem::swap(&mut self.kept, &mut self.prior);
            return;
        }
        let level = &mut self.levels[self.depth];
        self.prior.copy_sets_from(level);
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
        // Deleting a candidate outside the heaviest set leaves it the heaviest: the history still ends with it.  Its
        // tableau, if the workspace holds it, is then the one to derive the level's from (a deletion).
        level.has_split = false;
        level.has_pot = false;
        if level.solved && level.meet_solve == self.meet.solves && level.meet_solve != 0 {
            (level.source.held, level.source.contracted) = (level.meet_solve, Bits::EMPTY);
        }
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
                if self.starts.len() <= d {
                    self.starts.resize_with(d + 1, Default::default);
                }
                let node = self.nodes[d];
                let from = if self.starts[d].1 == node { Some(d) } else { d.checked_sub(1) };
                let start = from.map(|i| &self.starts[i].0);
                // The level's problem is its node's last split's, or its parent node's contracted by what it promoted.
                let graph = self.graph;
                let contracted = &mut self.contracted_idx;
                contracted.clear();
                let same_problem = match from {
                    Some(i) if i == d => true,
                    Some(i) if self.starts[i].1 == self.nodes[i] => {
                        contracted.extend(level.added.iter().map(|x| graph.index[x as usize]));
                        true
                    }
                    _ => false,
                };
                let contracted = same_problem.then_some(&contracted[..]);
                let (meet, live_idx, start_idx) = (&mut self.meet, &mut self.live_idx, &mut self.start_idx);
                split(graph, level, meet, live_idx, start_idx, &mut self.stats, start, contracted);
                if level.has_pot {
                    self.meet.save_split(&mut self.starts[d].0);
                    self.starts[d].1 = node;
                }
            }
            if level.has_split {
                for &(x, bound) in &level.split {
                    if bound < need as i64 {
                        forced.insert(x);
                    }
                }
                self.force_by_cycles(cap, need, &mut forced);
            }
        }
        let level = &self.levels[d];
        Eval { value: level.value, chosen, forced }
    }

    /// More forced members of the top level's heaviest set (solved, of all sizes, its split found and its tableau
    /// the workspace's), at `need`, added to `forced`: a member is forced when every set without it weighs less than
    /// `need`, which holds when the sets of other sizes than the heaviest set's lose more than the slack `value −
    /// need` and its exchange graph has no cycle through the member that short (see `meet`'s module docs).  Only the
    /// members whose split bound the window admits are searched (the tactic `force_window`, [`ForceWindow`]).
    ///
    /// A cycle within the slack shows every member on it free, so the cycles that the last searches at the parent's
    /// depth, at this one and at the child's found are tried first: each that is still a cycle of this exchange graph
    /// and within the slack ([`Meet::cycle_length`]) frees its members with no search.  A child's exchange graph is
    /// its parent's less the elements it promoted or dropped (contracting a member keeps the circuits of the others),
    /// and no cycle within the slack passes through a forced member, so a child that inherits its parent's heaviest
    /// set finds nearly all its members free on its parent's cycles.  The forced members are those a search for each
    /// would find: a member is free exactly when some cycle within the slack passes through it.
    fn force_by_cycles(&mut self, cap: usize, need: u64, forced: &mut Bits<W>) {
        let graph = self.graph;
        let level = &mut self.levels[self.depth];
        let size = level.history.last().map_or(0, |last| last.len() as usize);
        let held = level.meet_solve == self.meet.solves;
        if !(level.has_pot && level.settled && held && size <= cap && level.value >= need) {
            return;
        }
        let slack = (level.value - need) as i64;
        let problem = Problem {
            block: [&graph.block[0], &graph.block[1]],
            vector: [&level.vec[0], &level.vec[1]],
            weight: &graph.weight,
        };
        let mut losses = Losses::of(level, &mut self.meet, &problem, cap > size);
        if !losses.certify(slack + 1, &mut self.meet, &problem, &level.pot) {
            return;
        }
        level.loss_floor = losses.smaller;
        // The members of the cycles within the slack, each of which passes through them: the cycles kept first, then
        // those that the searches find.  `within` becomes this depth's cycles.
        let d = self.depth;
        if self.cycles.len() <= d {
            self.cycles.resize_with(d + 1, Cycles::default);
        }
        let mut within = std::mem::take(&mut self.next_cycles);
        within.clear();
        let mut free = Bits::<W>::EMPTY;
        let meet = &self.meet;
        for cycles in &self.cycles[d.saturating_sub(1)..self.cycles.len().min(d + 2)] {
            for cycle in cycles.iter() {
                let live = cycle.iter().all(|&e| level.live.contains(graph.elems[e as usize]));
                let frees = || cycle.iter().any(|&e| meet.contains(e) && !free.contains(graph.elems[e as usize]));
                if live && frees() && meet.cycle_length(&problem, cycle).is_some_and(|length| length <= slack) {
                    free_members(graph, meet, cycle, &mut free);
                    within.push(cycle);
                }
            }
        }
        #[cfg(debug_assertions)]
        for x in free.iter() {
            let found = self.meet.has_cycle_within(&problem, &level.pot, graph.index[x as usize], slack);
            assert!(found, "{x} is on a cycle kept within the slack {slack}, which the search does not find");
        }
        for &(x, bound) in &level.split {
            let near = self.force_window.admits(bound - need as i64, slack);
            if !near || forced.contains(x) || free.contains(x) {
                continue;
            }
            let e = graph.index[x as usize];
            if self.meet.has_cycle_within(&problem, &level.pot, e, slack) {
                free_members(graph, &self.meet, self.meet.keep_cycle(e, &mut within), &mut free);
            } else {
                forced.insert(x);
                self.stats.forced_by_cycles += 1;
            }
        }
        self.next_cycles = std::mem::replace(&mut self.cycles[d], within);
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

    fn certified_excluding(&mut self, xs: &[u32], cap: usize, out: &mut Vec<Option<u64>>) {
        out.clear();
        let graph = self.graph;
        let level = &mut self.levels[self.depth];
        let size = level.history.last().map_or(0, |last| last.len() as usize);
        if !(level.solved && level.settled && level.has_pot && size <= cap) {
            out.extend(xs.iter().map(|_| None));
            return;
        }
        if level.meet_solve != self.meet.solves {
            // The workspace holds another solve: the level's heaviest set solved again from itself (its tableau
            // built and one shortest path, no augmentation).
            resolve(graph, level, &mut self.meet, &mut self.live_idx, &mut self.start_idx);
        }
        let problem = Problem {
            block: [&graph.block[0], &graph.block[1]],
            vector: [&level.vec[0], &level.vec[1]],
            weight: &graph.weight,
        };
        let last = level.history.last().copied().unwrap_or(Bits::EMPTY);
        let mut losses = Losses::of(level, &mut self.meet, &problem, cap > size);
        let reach = losses.reach(&last);
        self.certified.clear();
        for &x in xs {
            let mut bound = None;
            if let Some(value) = self.meet.without(&problem, &level.pot, graph.index[x as usize], reach, &[]) {
                // The heaviest set flipped along the cycle, which is the heaviest set without x if the sizes
                // certify it.
                let flip: Bits<W> = self.meet.cycle().iter().map(|&e| graph.elems[e as usize]).collect();
                let delta = level.value as i64 - value;
                if losses.certify(delta, &mut self.meet, &problem, &level.pot) {
                    self.certified.push((x, (last - flip) | (flip - last), value as u64, delta));
                    bound = Some(value as u64);
                }
            }
            out.push(bound);
        }
        level.loss_floor = losses.smaller;
        self.stats.certified += out.iter().filter(|b| b.is_some()).count() as u64;
    }

    fn keep_certified(&mut self, x: u32) {
        let &(_, set, value, delta) = self.certified.iter().find(|c| c.0 == x).expect("a certified bound without x");
        let level = &self.levels[self.depth];
        let kept = &mut self.kept;
        kept.copy_from(level);
        kept.live.remove(x);
        // The level's dominant sets without x stay dominant without it; the heaviest set without x ends them.
        kept.history.extend(level.history.iter().filter(|h| !h.contains(x) && h.len() < set.len()));
        kept.history.push(set);
        kept.solved = true;
        kept.settled = true;
        kept.cap = level.cap;
        kept.value = value;
        // Without x the smaller sets weigh no more than with it.
        kept.loss_floor = (level.loss_floor - delta).max(0);
        // The level's tableau, which the workspace holds (it certified the set), is the one to derive the kept
        // level's from (the set flipped along the cycle, x deleted), copied because the child's subtree will use
        // the workspace before the level goes on without x.
        if level.meet_solve == self.meet.solves && level.meet_solve != 0 {
            kept.source.held = level.meet_solve;
            self.meet.save(&mut kept.source.snapshot);
            kept.source.saved = true;
        }
        self.kept_for = Some((self.depth, x));
        self.scratch_for = None;
    }

    fn keep_excluded(&mut self) {
        if let Some(x) = self.scratch_for.take() {
            std::mem::swap(&mut self.scratch, &mut self.kept);
            self.kept_for = Some((self.depth, x));
        }
    }
}

/// Adds the members of the workspace's set on `cycle` (elements) to `free` (candidates).
fn free_members<const W: usize, const R: usize>(graph: &Gf2Graph, meet: &Meet<R>, cycle: &[u32], free: &mut Bits<W>) {
    for &e in cycle.iter().filter(|&&e| meet.contains(e)) {
        free.insert(graph.elems[e as usize]);
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
    level.source.clear();
    level.cap = cap;
    level.value = meet.value as u64;
    level.loss_floor = Losses::from_history(level).unwrap_or(0);
    level.has_split = false;
    level.has_pot = false;
}

/// A child that keeps every member of its parent's heaviest set but one, `x`, besides those it promotes: its heaviest
/// set read off the parent's exchange graph with no solve, if the workspace still holds the parent's tableau and the
/// sizes certify it ([`meet::Meet::without`], the cycle through `x` avoiding the promoted members and every element
/// the child dropped: the child's sets with the promoted are the parent's sets that hold them and avoid the dropped).
/// Then the child is solved, its heaviest set (of all sizes) the parent's flipped along the cycle, less the
/// promoted.  `vectors` is a level with the parent's vectors (`parent` may be a copy of its sets only, see
/// [`Levels::push_all`]); `banned` is scratch.
fn certify_child<const W: usize, const R: usize>(
    graph: &Gf2Graph,
    vectors: &Level<W, R>,
    parent: &Level<W, R>,
    child: &mut Level<W, R>,
    added: &Bits<W>,
    meet: &mut Meet<R>,
    banned: &mut Vec<u32>,
) -> bool {
    if !(parent.solved && parent.settled && parent.has_pot && parent.meet_solve == meet.solves) {
        return false;
    }
    let Some(&last) = parent.history.last() else { return false };
    let missing = last - *added - child.live;
    if !added.is_subset(&last) || missing.len() != 1 {
        return false;
    }
    let x = missing.max().expect("one member");
    let problem = Problem {
        block: [&graph.block[0], &graph.block[1]],
        vector: [&vectors.vec[0], &vectors.vec[1]],
        weight: &graph.weight,
    };
    // The child's heaviest set of all sizes: the parent's sets of every other size count.
    let mut losses = Losses::of(parent, meet, &problem, true);
    banned.clear();
    banned.extend((*added | (parent.live - last - child.live)).iter().map(|y| graph.index[y as usize]));
    let reach = losses.reach(&last);
    let Some(value) = meet.without(&problem, &parent.pot, graph.index[x as usize], reach, banned) else {
        return false;
    };
    let flip: Bits<W> = meet.cycle().iter().map(|&e| graph.elems[e as usize]).collect();
    let delta = parent.value as i64 - value;
    if !losses.certify(delta, meet, &problem, &parent.pot) {
        return false;
    }
    let set = ((last - flip) | (flip - last)) - *added;
    while child.history.last().is_some_and(|entry| entry.len() >= set.len()) {
        child.history.pop();
    }
    child.history.push(set);
    child.solved = true;
    child.settled = true;
    child.cap = parent.cap.saturating_sub(added.len() as usize);
    child.value = value as u64 - added.sum();
    // The child's smaller sets, with the promoted, are smaller sets of the parent's without x.
    child.loss_floor = (losses.smaller - delta).max(0);
    true
}

/// What the common independent sets of other sizes than a level's heaviest set (of all sizes, its tableau the
/// workspace's) lose against it at least, which certifies a set read off its exchange graph as the heaviest without
/// a member if the set loses no more (see `meet`'s module docs): the smaller ones `smaller` (exact once `exact`),
/// the larger ones `larger` (`i64::MAX` where they do not count).
struct Losses {
    smaller: i64,
    exact: bool,
    larger: i64,
}

impl Losses {
    /// What the smaller sets lose, exactly, from a dominant set one smaller in `level`'s history, if it has one.
    fn from_history<const W: usize, const R: usize>(level: &Level<W, R>) -> Option<i64> {
        let size = level.history.last()?.len();
        let smaller = level.history.iter().rev().find(|set| set.len() + 1 == size)?;
        Some(level.value as i64 - smaller.sum() as i64)
    }

    /// For `level`, whose tableau the workspace `meet` holds; the larger sets count if `larger`.
    fn of<const W: usize, const R: usize>(
        level: &Level<W, R>,
        meet: &mut Meet<R>,
        problem: &Problem<'_, R>,
        larger: bool,
    ) -> Self {
        let exact = Self::from_history(level);
        Losses {
            smaller: exact.unwrap_or(level.loss_floor).max(level.loss_floor),
            exact: exact.is_some(),
            larger: if larger { meet.gap(problem) } else { i64::MAX },
        }
    }

    /// The longest cycle that can certify: no longer than what the larger sets lose, nor than the lightest member
    /// of the heaviest set `last`, which is all that the heaviest set less it loses.
    fn reach<const W: usize>(&self, last: &Bits<W>) -> i64 {
        self.larger.min(last.min().map_or(0, i64::from))
    }

    /// Whether a set that loses `delta` against the heaviest set is the heaviest of its kind; if the bound on the
    /// smaller sets does not decide, it is made exact first ([`Meet::smaller_loss`] on the workspace's tableau and
    /// the level's split `pot`).
    fn certify<const R: usize>(
        &mut self,
        delta: i64,
        meet: &mut Meet<R>,
        problem: &Problem<'_, R>,
        pot: &[i64],
    ) -> bool {
        if delta > self.larger {
            return false;
        }
        if delta > self.smaller && !self.exact {
            self.smaller = self.smaller.max(meet.smaller_loss(problem, pot));
            self.exact = true;
        }
        delta <= self.smaller
    }
}

/// Solves `level` (solved and settled) again from its heaviest set, so that the workspace holds its tableau: the
/// tableau built and one shortest path, no augmentation.  `live_idx` is left holding the level's live elements.
fn resolve<const W: usize, const R: usize>(
    graph: &Gf2Graph,
    level: &mut Level<W, R>,
    meet: &mut Meet<R>,
    live_idx: &mut Vec<u32>,
    start_idx: &mut Vec<u32>,
) {
    let chosen = level.history.last().copied().unwrap_or(Bits::EMPTY);
    live_idx.clear();
    live_idx.extend(level.live.iter().map(|x| graph.index[x as usize]));
    start_idx.clear();
    start_idx.extend(chosen.iter().map(|x| graph.index[x as usize]));
    let size = start_idx.len();
    let source = &level.source;
    start_idx.extend(source.contracted.iter().map(|x| graph.index[x as usize]));
    let (start, contracted) = start_idx.split_at(size);
    let problem = Problem {
        block: [&graph.block[0], &graph.block[1]],
        vector: [&level.vec[0], &level.vec[1]],
        weight: &graph.weight,
    };
    // The same tableau as the solve from the heaviest set would build (see `meet`, "Deriving a tableau"), derived
    // from a related one where the level has one.
    if source.held != 0 && source.held == meet.solves {
        meet.derive(&problem, live_idx, start, contracted);
    } else if source.saved {
        meet.derive_from(&source.snapshot, &problem, live_idx, start, &[]);
    } else {
        meet.solve(&problem, live_idx, start, usize::MAX, i64::MAX, |_| {});
    }
    debug_assert_eq!(meet.value as u64, level.value, "the heaviest set solved again changed");
    level.meet_solve = meet.solves;
}

/// The bounds without each member of `level`'s heaviest set (settled), from a weight split, its Bellman–Ford started
/// from `start`, an earlier split, whose problem the level's is contracted by `contracted` if that is known
/// ([`Meet::split_bounds_from`]).  Unless the workspace still holds the level's last solve, the level's heaviest set
/// is solved again from itself first ([`resolve`]).
#[allow(clippy::too_many_arguments)]
fn split<const W: usize, const R: usize>(
    graph: &Gf2Graph,
    level: &mut Level<W, R>,
    meet: &mut Meet<R>,
    live_idx: &mut Vec<u32>,
    start_idx: &mut Vec<u32>,
    stats: &mut Gf2Stats,
    start: Option<&SplitStart>,
    contracted: Option<&[u32]>,
) {
    if level.meet_solve != meet.solves {
        resolve(graph, level, meet, live_idx, start_idx);
    } else {
        live_idx.clear();
        live_idx.extend(level.live.iter().map(|x| graph.index[x as usize]));
    }
    let problem = Problem {
        block: [&graph.block[0], &graph.block[1]],
        vector: [&level.vec[0], &level.vec[1]],
        weight: &graph.weight,
    };
    stats.splits += 1;
    let out = &mut level.split;
    out.clear();
    level.has_pot = meet.settled
        && meet.split_bounds_from(&problem, live_idx, start, contracted, |e, bound| {
            out.push((graph.elems[e as usize], bound))
        });
    if level.has_pot {
        level.pot.copy_from_slice(meet.potentials());
    }
    stats.split_rounds = meet.split_rounds;
    level.has_split = true;
}

#[cfg(test)]
mod tests;
