//! The two GF(2) frame matroids of a state (`FrameMatroids`, built by `frame_matroids`) and the game setup shared by
//! the studies that use them: `examples/matroid.rs`, and the fields study `examples/fields.rs` to come.  Included with
//! `#[path = "common/frames.rs"] mod frames;` next to `#[path = "common/meet.rs"] mod meet;`, whose types it uses; the
//! proof that every promotable set is independent in both matroids is in that module's comment, "The frame matroids".

use crate::meet::{intersect, intersect_fast, intersect_warm, lowest, BlockMatroid, Meet, Reduced, Span};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use taxman::bits::Bits;
use taxman::board::Board;
use taxman::game::Optimal;
use taxman::json::Json;
use taxman::model::{FrameGraph, FrameSet};
use taxman::search::{Prepared, SearchTactics};

// ---------------------------------------------------------------------------------------------------------
// The frame matroids of a state

/// The two frame matroids of a state and its candidates (see `common/meet.rs`): element `i` is the candidate
/// `elems[i]`; `a` is the direct sum of the even frames' matroids, `b` of the odd frames'.
pub struct FrameMatroids {
    pub elems: Vec<u32>,
    pub a: BlockMatroid,
    pub b: BlockMatroid,
    /// Candidates left out because they are loops of a frame matroid: no promotable set holds them.
    pub loops: Vec<u32>,
    /// Per frame, the rank of its matroid (its factors minus its moves).
    pub ranks: Vec<usize>,
}

/// One frame's coordinates and the span of its moves, reduced to echelon form.
pub struct FrameSpace {
    /// Coordinate of each number that is a factor of the frame (`u32::MAX` for the others).
    pub coord: Vec<u32>,
    pub words: usize,
    /// Rows of the moves' span, by pivot (lowest set bit).
    pub pivot_row: Vec<Option<Vec<u64>>>,
    /// The quotient coordinate of each non-pivot coordinate (`u32::MAX` for pivots).
    pub quotient: Vec<u32>,
    pub rank: usize,
}

impl FrameSpace {
    pub fn new<const W: usize>(
        model: &FrameGraph,
        factors: &Bits<W>,
        moves: &Bits<W>,
        max: u32,
    ) -> Result<Self, String> {
        let mut coord = vec![u32::MAX; max as usize + 1];
        for (k, y) in factors.iter().enumerate() {
            coord[y as usize] = k as u32;
        }
        let dim = factors.len() as usize;
        let words = dim.div_ceil(64).max(1);
        let mut s = FrameSpace { coord, words, pivot_row: vec![None; 64 * words], quotient: Vec::new(), rank: 0 };
        for m in moves.iter() {
            let v = s.vector(model, m);
            let r = s.reduce(v);
            let p = lowest(&r).ok_or_else(|| format!("the moves of a frame are dependent at {m}"))?;
            s.pivot_row[p] = Some(r);
        }
        let mut q = 0u32;
        s.quotient = (0..64 * words)
            .map(|c| {
                if c < dim && s.pivot_row[c].is_none() {
                    q += 1;
                    q - 1
                } else {
                    u32::MAX
                }
            })
            .collect();
        s.rank = q as usize;
        Ok(s)
    }

    /// The vector of the mover `u` (its maximal factors among the frame's factors).
    pub fn vector(&self, model: &FrameGraph, u: u32) -> Vec<u64> {
        let mut v = vec![0u64; self.words];
        for &q in model.graph().factors(u) {
            if let Some(&c) = self.coord.get(usize::from(q)) {
                if c != u32::MAX {
                    v[c as usize / 64] |= 1 << (c % 64);
                }
            }
        }
        v
    }

    /// The unit vector of the factor `y`.
    pub fn unit(&self, y: u32) -> Vec<u64> {
        let mut v = vec![0u64; self.words];
        let c = self.coord[y as usize];
        assert_ne!(c, u32::MAX, "{y} is not a factor of the frame");
        v[c as usize / 64] |= 1 << (c % 64);
        v
    }

    /// `v` fully reduced by the moves' rows: no pivot coordinate left.
    pub fn reduce(&self, mut v: Vec<u64>) -> Vec<u64> {
        let mut from = 0;
        loop {
            let next = v[from / 64..].iter().enumerate().find_map(|(i, &w)| {
                let w = if i == 0 { w & (!0u64 << (from % 64)) } else { w };
                (w != 0).then(|| (from / 64 + i) * 64 + w.trailing_zeros() as usize)
            });
            let Some(b) = next else { break };
            if let Some(row) = &self.pivot_row[b] {
                for (a, x) in v.iter_mut().zip(row) {
                    *a ^= x;
                }
            }
            from = b + 1;
            if from >= 64 * self.words {
                break;
            }
        }
        v
    }

    /// `v` in the quotient by the moves' span: reduced, then the non-pivot coordinates.
    pub fn project(&self, v: Vec<u64>) -> Vec<u64> {
        let r = self.reduce(v);
        let mut out = vec![0u64; self.rank.div_ceil(64).max(1)];
        for c in meet_bits(&r) {
            let q = self.quotient[c];
            debug_assert_ne!(q, u32::MAX, "a pivot survived the reduction");
            out[q as usize / 64] |= 1 << (q % 64);
        }
        out
    }
}

pub fn meet_bits(v: &[u64]) -> Vec<usize> {
    let mut out = Vec::new();
    for (i, &w) in v.iter().enumerate() {
        let mut w = w;
        while w != 0 {
            out.push(64 * i + w.trailing_zeros() as usize);
            w &= w - 1;
        }
    }
    out
}

pub fn frame_matroids<const W: usize>(model: &FrameGraph, state: &FrameSet<W>, cands: &Bits<W>) -> FrameMatroids {
    let frames = state.frames();
    let max = frames.iter().flat_map(|f| f.factors.max().into_iter().chain(f.moves.max())).max().unwrap_or(0);
    let max = max.max(cands.max().unwrap_or(0));
    let spaces: Vec<FrameSpace> =
        frames.iter().map(|f| FrameSpace::new(model, &f.factors, &f.moves, max).expect("a winnable frame")).collect();
    let words: Vec<usize> = spaces.iter().map(|s| s.rank.div_ceil(64).max(1)).collect();
    let block_words =
        |parity: usize| -> Vec<usize> { (0..frames.len()).filter(|h| h % 2 == parity).map(|h| words[h]).collect() };
    let mut a = BlockMatroid { block: Vec::new(), vecs: Vec::new(), words: block_words(0) };
    let mut b = BlockMatroid { block: Vec::new(), vecs: Vec::new(), words: block_words(1) };
    let (mut elems, mut loops) = (Vec::new(), Vec::new());
    for x in cands.iter() {
        let h = model.home(x);
        if h == 0 || h >= frames.len() || !frames[h].factors.contains(x) {
            loops.push(x);
            continue;
        }
        let out = spaces[h].project(spaces[h].unit(x));
        let into = spaces[h - 1].project(spaces[h - 1].vector(model, x));
        if out.iter().all(|&w| w == 0) || into.iter().all(|&w| w == 0) {
            loops.push(x);
            continue;
        }
        let (even, odd) = if h % 2 == 0 { ((h, out), (h - 1, into)) } else { ((h - 1, into), (h, out)) };
        a.block.push(even.0 / 2);
        a.vecs.push(even.1);
        b.block.push(odd.0 / 2);
        b.vecs.push(odd.1);
        elems.push(x);
    }
    FrameMatroids { elems, a, b, loops, ranks: spaces.iter().map(|s| s.rank).collect() }
}

impl FrameMatroids {
    /// Whether the candidates `set` (all of them elements) are independent in both matroids.
    pub fn independent<const W: usize>(&self, set: &Bits<W>) -> Result<(), String> {
        for (name, m) in [("A", &self.a), ("B", &self.b)] {
            let mut span = Span::new(m);
            for (i, &x) in self.elems.iter().enumerate() {
                if set.contains(x) {
                    if let Reduced::Circuit(_) = span.reduce(m, i) {
                        return Err(format!("{x} closes a circuit in {name}"));
                    }
                    span.insert(m, i);
                }
            }
        }
        match set.iter().find(|x| !self.elems.contains(x)) {
            Some(x) => Err(format!("{x} is not an element (a loop or not a candidate)")),
            None => Ok(()),
        }
    }

    /// The heaviest common independent set of at most `cap` candidates.
    pub fn bound(&self, cap: usize) -> Meet {
        let w: Vec<i64> = self.elems.iter().map(|&x| i64::from(x)).collect();
        self.solve(&w, cap)
    }

    /// The heaviest common independent set of at most `cap` candidates, element `i` weighing `w[i]`.
    pub fn solve(&self, w: &[i64], cap: usize) -> Meet {
        if self.a.words.iter().chain(&self.b.words).all(|&w| w == 1) {
            intersect_fast(&self.a, &self.b, w, cap)
        } else {
            intersect(&self.a, &self.b, w, cap, false)
        }
    }

    /// [`bound`](Self::bound) started from the elements `start`, if they are an extreme common independent set
    /// (see `meet::intersect_warm`).
    pub fn warm(&self, start: &[bool], cap: usize) -> Option<Meet> {
        let w: Vec<i64> = self.elems.iter().map(|&x| i64::from(x)).collect();
        intersect_warm(&self.a, &self.b, &w, cap, start)
    }

    /// [`bound`](Self::bound) by the reference algorithm.
    pub fn bound_reference(&self, cap: usize) -> Meet {
        let w: Vec<i64> = self.elems.iter().map(|&x| i64::from(x)).collect();
        intersect(&self.a, &self.b, &w, cap, false)
    }

    pub fn chosen<const W: usize>(&self, m: &Meet) -> Bits<W> {
        self.elems.iter().zip(&m.chosen).filter(|(_, &c)| c).map(|(&x, _)| x).collect()
    }
}

// ---------------------------------------------------------------------------------------------------------
// Setup

/// The board of `n` set up for a search as `taxman solve` sets it up (the default tactics); `None` for a board with
/// no candidates.
pub fn setup<const W: usize>(n: u32) -> Option<(Board<W>, Prepared<W>)> {
    let board = Board::<W>::for_n(n);
    if board.is_trivial() {
        return None;
    }
    let p = board.prepare(SearchTactics::default()).expect("prepare");
    Some((board, p))
}

pub fn is_prime(n: u32) -> bool {
    n >= 2 && (2..).take_while(|d| d * d <= n).all(|d| !n.is_multiple_of(d))
}

/// What is known of the optima: the known games, and the promotion sums `P*` of `--opts` (lines with `n` and
/// `opt`, as `bench/curve/slope_roots.jsonl` has them).
pub struct Known {
    pub optimal: Optimal,
    pub sums: HashMap<u32, u64>,
}

impl Known {
    pub fn load(known: &[PathBuf], opts: Option<&Path>) -> Result<Known, String> {
        let optimal = Optimal::built_in().with_results_files(known)?;
        let mut sums = HashMap::new();
        if let Some(path) = opts {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                let line = Json::parse(line).map_err(|e| format!("{}: {e}", path.display()))?;
                let field = |name| line.get(name).and_then(Json::as_u64);
                if let (Some(n), Some(opt)) = (field("n"), field("opt")) {
                    sums.insert(n as u32, opt);
                }
            }
        }
        Ok(Known { optimal, sums })
    }

    /// `P*` of the board `n` whose game with no promotions scores `in_the_bag`, if known.
    pub fn opt(&self, n: u32, in_the_bag: u64) -> Option<u64> {
        self.sums.get(&n).copied().or_else(|| self.optimal.score(n).map(|s| s - in_the_bag))
    }
}

/// A xorshift generator, seeded with any nonzero number.
pub struct Rng(pub u64);

impl Rng {
    pub fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}
