//! The two GF(2) frame matroids of a state, in the form the A1 bound reads them (`dw::Ctx` hands their vectors to the
//! crate's intersection, `taxman::gf2::meet`): the blocks of the even frames' matroid `A` and of the odd frames'
//! matroid `B`, on the candidates of a state.  They are the matroids of `taxman::gf2::Gf2Graph` (src/gf2.rs has the
//! claim that every promotable set is independent in both, and its proof is THEORY.md, Theorem 11), built here as
//! plain vectors so that the study can read them; the crate's graph keeps its own.

use taxman::bits::Bits;
use taxman::model::{FrameGraph, FrameSet};

/// A linear matroid over GF(2) that is a direct sum of blocks: element `e` lies in block `block[e]` with the vector
/// `vecs[e]` (as many words as that block's `words`).  A zero vector is a loop.
#[derive(Clone, Debug)]
pub struct BlockMatroid {
    pub block: Vec<usize>,
    pub vecs: Vec<Vec<u64>>,
    /// Words per block.
    pub words: Vec<usize>,
}

/// The two frame matroids of a state and its candidates: element `i` is the candidate `elems[i]`; `a` is the direct
/// sum of the even frames' matroids, `b` of the odd frames'.
pub struct FrameMatroids {
    pub elems: Vec<u32>,
    pub a: BlockMatroid,
    pub b: BlockMatroid,
}

/// One frame's coordinates and the span of its moves, reduced to echelon form.
struct FrameSpace {
    /// Coordinate of each number that is a factor of the frame (`u32::MAX` for the others).
    coord: Vec<u32>,
    words: usize,
    /// Rows of the moves' span, by pivot (lowest set bit).
    pivot_row: Vec<Option<Vec<u64>>>,
    /// The quotient coordinate of each non-pivot coordinate (`u32::MAX` for pivots).
    quotient: Vec<u32>,
    rank: usize,
}

impl FrameSpace {
    fn new<const W: usize>(model: &FrameGraph, factors: &Bits<W>, moves: &Bits<W>, max: u32) -> Result<Self, String> {
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
    fn vector(&self, model: &FrameGraph, u: u32) -> Vec<u64> {
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
    fn unit(&self, y: u32) -> Vec<u64> {
        let mut v = vec![0u64; self.words];
        let c = self.coord[y as usize];
        assert_ne!(c, u32::MAX, "{y} is not a factor of the frame");
        v[c as usize / 64] |= 1 << (c % 64);
        v
    }

    /// `v` fully reduced by the moves' rows: no pivot coordinate left.
    fn reduce(&self, mut v: Vec<u64>) -> Vec<u64> {
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
    fn project(&self, v: Vec<u64>) -> Vec<u64> {
        let r = self.reduce(v);
        let mut out = vec![0u64; self.rank.div_ceil(64).max(1)];
        for c in set_bits(&r) {
            let q = self.quotient[c];
            debug_assert_ne!(q, u32::MAX, "a pivot survived the reduction");
            out[q as usize / 64] |= 1 << (q % 64);
        }
        out
    }
}

/// The lowest set bit of `v`.
fn lowest(v: &[u64]) -> Option<usize> {
    v.iter().enumerate().find(|(_, &w)| w != 0).map(|(i, w)| i * 64 + w.trailing_zeros() as usize)
}

/// The set bits of `v`, ascending.
fn set_bits(v: &[u64]) -> Vec<usize> {
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

/// The frame matroids of `state` on the candidates `cands`: those that are loops of a frame matroid are left out (no
/// promotable set holds one).
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
    let mut elems = Vec::new();
    for x in cands.iter() {
        let h = model.home(x);
        if h == 0 || h >= frames.len() || !frames[h].factors.contains(x) {
            continue;
        }
        let out = spaces[h].project(spaces[h].unit(x));
        let into = spaces[h - 1].project(spaces[h - 1].vector(model, x));
        if out.iter().all(|&w| w == 0) || into.iter().all(|&w| w == 0) {
            continue;
        }
        let (even, odd) = if h % 2 == 0 { ((h, out), (h - 1, into)) } else { ((h - 1, into), (h, out)) };
        a.block.push(even.0 / 2);
        a.vecs.push(even.1);
        b.block.push(odd.0 / 2);
        b.vecs.push(odd.1);
        elems.push(x);
    }
    FrameMatroids { elems, a, b }
}
