//! The root problem Π_n of the theory, built from the board alone: what setup must build.
//!
//! docs/theory/frame-model.md defines it: the frames of the board (the components of the graph that joins each source
//! to its maximal factors), U\* (descending greedy on each frame), the numbering by layer, the candidates (tests (a)
//! and (b)), the claims of frame h in their canonical order, and the root frames.  [`build`] follows those definitions
//! with the tables of [`crate::numtheory`] and the peel of [`crate::playable`] alone, and shares nothing with `frames`
//! or `model`, so that a test (`tests/canonical_root.rs`) and the census of the frames (`examples/structure.rs`, checks
//! G and I) can check setup against a builder that is not setup.
//!
//! It is written to be read beside the theory, not for speed.  It builds the full board `1..=n` only: on a board with
//! holes the frames are formed differently (docs/theory/exactness.md, "Boards with holes").

use crate::numtheory::Tables;
use crate::playable::playable_against;
use std::collections::{BTreeMap, BTreeSet};

/// The root problem of a board, as the theory defines it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootProblem {
    /// U\*, the sources that descending greedy keeps, ascending.
    pub ustar: Vec<u32>,
    /// The candidates, ascending.
    pub candidates: Vec<u32>,
    /// The root frames, frame 0 first, each as its moves and its factors, ascending.  None when no frame has
    /// freedoms.
    pub frames: Vec<(Vec<u32>, Vec<u32>)>,
}

/// A frame of the board: its sources, the ones that descending greedy keeps, its factors, and its layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    /// The layer of the frame: Ω of its sources.
    pub layer: u32,
    /// Its sources, the numbers of the frame above n/2, ascending.
    pub sources: Vec<u32>,
    /// Its moves, the sources that descending greedy keeps, ascending.
    pub kept: Vec<u32>,
    /// Its factors, the numbers of the frame up to n/2, ascending.
    pub factors: Vec<u32>,
}

impl Frame {
    /// Whether the frame has a freedom: more factors than moves.
    pub fn has_freedoms(&self) -> bool {
        self.factors.len() > self.kept.len()
    }
}

fn with(set: &[u32], x: u32) -> Vec<u32> {
    let mut out = set.to_vec();
    out.push(x);
    out
}

fn without(set: &[u32], x: u32) -> Vec<u32> {
    set.iter().copied().filter(|&y| y != x).collect()
}

/// The root of `x` in a union-find forest, halving the path on the way.
fn find(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        parent[x as usize] = parent[parent[x as usize] as usize];
        x = parent[x as usize];
    }
    x
}

/// The frames of the board: the components of the graph that joins each source to its maximal factors, each with the
/// sources that descending greedy keeps against its factors (the frame lemma (F1), frame-model.md; descending greedy,
/// docs/theory/sources.md).
fn frames_of(t: &Tables) -> Vec<Frame> {
    let n = t.n;
    let mut parent: Vec<u32> = (0..=n).collect();
    for s in (1..=n).filter(|&x| 2 * x > n) {
        for &f in &t.max_factors[s as usize] {
            let (a, b) = (find(&mut parent, s), find(&mut parent, f));
            parent[a as usize] = b;
        }
    }
    let mut parts: BTreeMap<u32, (Vec<u32>, Vec<u32>)> = BTreeMap::new();
    for x in 1..=n {
        let (sources, factors) = parts.entry(find(&mut parent, x)).or_default();
        if 2 * x > n {
            sources.push(x);
        } else {
            factors.push(x);
        }
    }
    let mut frames = Vec::with_capacity(parts.len());
    for (sources, factors) in parts.into_values() {
        let mut kept: Vec<u32> = Vec::new();
        for &s in sources.iter().rev() {
            if playable_against(t, &with(&kept, s), &factors) {
                kept.push(s);
            }
        }
        kept.sort_unstable();
        frames.push(Frame { layer: t.omega(sources[0]), sources, kept, factors });
    }
    frames
}

/// Π_n of the board of `t`.
///
/// # Panics
///
/// If the board has holes.
pub fn build(t: &Tables) -> RootProblem {
    build_with_frames(t).0
}

/// Π_n of the board of `t` and the frames of the board it is built from, in no meaningful order: for each frame its
/// layer, all its sources, the sources it keeps and its factors (`build` is the first part alone).
///
/// # Panics
///
/// If the board has holes.
pub fn build_with_frames(t: &Tables) -> (RootProblem, Vec<Frame>) {
    assert!(t.holes.is_empty(), "the root problem of a board with holes is not built here");
    let frames = frames_of(t);
    (root_problem_of(t, &frames), frames)
}

/// Π_n of the board of `t` from its frames.
fn root_problem_of(t: &Tables, frames: &[Frame]) -> RootProblem {
    let ustar: BTreeSet<u32> = frames.iter().flat_map(|frame| frame.kept.iter().copied()).collect();
    let ustar: Vec<u32> = ustar.into_iter().collect();

    // The numbering by layer: frame h is the union of the frames with freedoms of layer k₀ + h.
    let Some(k0) = frames.iter().filter(|frame| frame.has_freedoms()).map(|frame| frame.layer).min() else {
        return RootProblem { ustar, candidates: Vec::new(), frames: Vec::new() };
    };
    let number = |frame: &Frame| frame.has_freedoms().then(|| (frame.layer - k0) as usize);
    let top = frames.iter().filter_map(number).max().expect("a frame has freedoms");
    let mut frame_h: Vec<(Vec<u32>, Vec<u32>)> = vec![(Vec::new(), Vec::new()); top + 1];
    for frame in frames {
        if let Some(h) = number(frame) {
            frame_h[h].0.extend(&frame.kept);
            frame_h[h].1.extend(&frame.factors);
        }
    }
    for (moves, factors) in &mut frame_h {
        moves.sort_unstable();
        factors.sort_unstable();
    }

    // Whether frame h − 1 can take on a factor of frame h ≥ 1: test (b) of the candidates, and what decides the back
    // claims below.
    let mut taken_on = vec![false; t.n as usize + 1];
    for h in 1..=top {
        let (moves, factors) = &frame_h[h - 1];
        for &x in &frame_h[h].1 {
            taken_on[x as usize] = playable_against(t, &with(moves, x), factors);
        }
    }

    // The candidates: a factor of a frame D numbered h ≥ 1 that D can spare (test (a)) and frame h − 1 can take on
    // (test (b)).
    let mut candidates = Vec::new();
    for frame in frames {
        if number(frame).is_some_and(|h| h >= 1) {
            let spared = |x: u32| playable_against(t, &frame.kept, &without(&frame.factors, x));
            candidates.extend(frame.factors.iter().copied().filter(|&x| taken_on[x as usize] && spared(x)));
        }
    }
    candidates.sort_unstable();

    // The claims of frame h, in rounds: the back claims through the factors not yet claimed, in increasing order, then
    // the front claim of the largest move that has one.  The last round finds no front claim, so no move is left with
    // one live factor, and root frame h is frame h less its claims.
    let mut root = Vec::with_capacity(top + 1);
    for h in 0..=top {
        let (moves, factors) = &frame_h[h];
        let above: &[u32] = frame_h.get(h + 1).map_or(&[], |(_, factors)| factors);
        // A back claim's factor must have no other use: frame h − 1 cannot take it on, and frame h can take on no
        // factor of frame h + 1 that has it as a maximal factor.
        let no_other_use = |f: u32| {
            !taken_on[f as usize]
                && !above.iter().any(|&c| taken_on[c as usize] && t.max_factors[c as usize].contains(&f))
        };
        let (mut moves_claimed, mut factors_claimed) = (BTreeSet::new(), BTreeSet::new());
        loop {
            let mut made = false;
            for &f in factors {
                if factors_claimed.contains(&f) {
                    continue;
                }
                let wanting: Vec<u32> = (moves.iter().copied())
                    .filter(|s| !moves_claimed.contains(s) && t.max_factors[*s as usize].contains(&f))
                    .collect();
                if let [s] = wanting[..] {
                    if no_other_use(f) {
                        moves_claimed.insert(s);
                        factors_claimed.insert(f);
                        made = true;
                    }
                }
            }
            let live = |s: u32| -> Vec<u32> {
                (t.max_factors[s as usize].iter().copied())
                    .filter(|f| factors.binary_search(f).is_ok() && !factors_claimed.contains(f))
                    .collect()
            };
            let front = moves.iter().rev().filter(|s| !moves_claimed.contains(*s)).find_map(|&s| match live(s)[..] {
                [f] => Some((s, f)),
                _ => None,
            });
            if let Some((s, f)) = front {
                moves_claimed.insert(s);
                factors_claimed.insert(f);
                made = true;
            }
            if !made {
                break;
            }
        }
        root.push((
            moves.iter().copied().filter(|s| !moves_claimed.contains(s)).collect(),
            factors.iter().copied().filter(|f| !factors_claimed.contains(f)).collect(),
        ));
    }
    RootProblem { ustar, candidates, frames: root }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Game 21, the theory's example (frame-model.md): the back claims (15, 5) and (20, 10) leave root frame 0 with
    /// the moves 14 and 21 and the factors 2, 3 and 7, and root frame 1 with the moves 12 and 18 and the factors 4, 6
    /// and 9; the candidates are 4 and 9.
    #[test]
    fn the_root_problem_of_21() {
        let root = build(&Tables::new(21));
        assert_eq!(root.ustar, [12, 14, 15, 16, 18, 19, 20, 21]);
        assert_eq!(root.candidates, [4, 9]);
        assert_eq!(root.frames, [(vec![14, 21], vec![2, 3, 7]), (vec![12, 18], vec![4, 6, 9])]);
    }

    /// Game 8: frame 0 has the move 6 and the factors 2 and 3, the back claim through 2 comes first, and it leaves
    /// the factor 3.
    #[test]
    fn the_claims_of_8_go_in_increasing_order() {
        let root = build(&Tables::new(8));
        assert_eq!(root.ustar, [6, 7, 8]);
        assert_eq!(root.frames, [(vec![], vec![3])]);
        assert!(root.candidates.is_empty());
    }
}
