//! Frames: the hives of a board stacked into levels, which is the shape the search works on.
//!
//! Hives that still have free factors are the ones the search has to care about.  A hive with free factors
//! can take on promoted numbers from the hives upstream of it, so the hives are grouped into *levels*:
//! level 0 is every free hive with no free hive downstream; level `k + 1` is every free hive directly
//! upstream of level `k` (and not yet placed).  All the hives of a level are merged into one **frame**.
//! A frame is described by two columns, the factors it still has and the moves (sources) it still has to
//! place; the search promotes factors of one frame into moves of the frame below it.
//!
//! The construction mirrors the Java `FrameBuilder`.  Its order of operations matters, because merging
//! changes what the hives above can do (a merged hive has more sources to place and more factors to share):
//! the promotion candidates of a level are collected from its separate hives, *then* the level is merged,
//! then the next level is looked at.  Once all levels are merged, every hive is simplified once more.

use crate::bits::Bits;
use crate::hives::Apiary;
use crate::numtheory::Tables;

/// One frame: a level of hives, merged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameColumn<const W: usize> {
    /// The merged hives' names joined with `+`; for diagnostics only.
    pub name: String,
    /// The frame's factors that are not already used up by forced moves.
    pub factors: Bits<W>,
    /// The frame's sources that are neither forced nor impossible.
    pub moves: Bits<W>,
    /// `|factors| − |sources|` of the merged hive: how many promotions the frame can absorb.
    pub free_factor_count: i32,
}

/// The frames of a board, with what the search needs to know about them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameColumns<const W: usize> {
    /// Bottom (downstream) first.
    pub frames: Vec<FrameColumn<W>>,
    /// Every number that could be promoted, in any frame.
    pub candidates: Bits<W>,
    /// An upper bound on how many promotions there can be: each frame can absorb as many as it has free
    /// factors, and a frame's own factors that are promoted use up some of the frame above's capacity.
    pub estimate_max_promotions: u32,
}

impl<const W: usize> FrameColumns<W> {
    /// The frames in the shape [`crate::model::Model::build`] takes: for each frame, bottom first, its factors
    /// and then its moves.
    pub fn columns(&self) -> Vec<Vec<u32>> {
        self.frames.iter().flat_map(|f| [f.factors.to_vec(), f.moves.to_vec()]).collect()
    }
}

/// Builds the frames of the board `1..=t.n` with no promotions.
pub fn build_frames<const W: usize>(t: &Tables) -> FrameColumns<W> {
    let mut apiary = Apiary::<W>::new(t, Bits::EMPTY);

    // A hive is identified by its smallest factor while the hives are being merged, since merging changes
    // the positions of the hives.  Hives that are free have factors.
    let levels = levels_of_free_hives(&apiary);
    let mut candidates = Bits::EMPTY;
    for level in &levels {
        let positions: Vec<usize> = level.iter().map(|&f| apiary.hive_index_of(f).expect("not yet merged")).collect();
        for &p in &positions {
            candidates |= apiary.hive_promotion_candidates(p);
        }
        if positions.len() > 1 {
            apiary.merge(&positions);
        }
    }
    // Merging forgot what the parts had found; find it again for the merged hives.
    apiary.finish_setup();

    let frames: Vec<FrameColumn<W>> = levels
        .iter()
        .map(|level| {
            let hive = &apiary.hives()[apiary.hive_index_of(level[0]).expect("frames are live hives")];
            FrameColumn {
                name: hive.name().to_owned(),
                factors: hive.remaining_factors(),
                moves: hive.remaining_sources(),
                free_factor_count: hive.free_factor_count(),
            }
        })
        .collect();
    let estimate_max_promotions = estimate_max_promotions(&frames);
    FrameColumns { frames, candidates, estimate_max_promotions }
}

/// The levels of hives that still have free factors, bottom first, each hive named by its smallest factor.
/// Within a level the hives are ordered by their smallest source.
fn levels_of_free_hives<const W: usize>(apiary: &Apiary<W>) -> Vec<Vec<u32>> {
    let hives = apiary.hives();
    let is_free = |i: usize| hives[i].has_free_factors();
    let by_smallest_source = |i: &usize| hives[*i].sources().min();

    let mut level: Vec<usize> = (0..hives.len())
        .filter(|&i| is_free(i) && apiary.downstream_hives(i).into_iter().all(|d| !is_free(d)))
        .collect();
    level.sort_by_key(by_smallest_source);

    let mut placed = vec![false; hives.len()];
    let mut levels = Vec::new();
    while !level.is_empty() {
        levels.push(level.iter().map(|&i| hives[i].factors().min().expect("a free hive has factors")).collect());
        for &i in &level {
            placed[i] = true;
        }
        let mut next: Vec<usize> = Vec::new();
        for &i in &level {
            for up in apiary.upstream_hives(i) {
                if !placed[up] && is_free(up) && !next.contains(&up) {
                    next.push(up);
                }
            }
        }
        next.sort_by_key(by_smallest_source);
        level = next;
    }
    levels
}

/// How many promotions there can be at most, working from the top frame down.  A frame can absorb as many
/// promotions (from the frame above) as it has free factors; each one uses up a free factor, and the free
/// factors that remain are what the frame can promote into the frame below.
fn estimate_max_promotions<const W: usize>(frames: &[FrameColumn<W>]) -> u32 {
    let (mut total, mut offer) = (0i32, 0i32);
    for frame in frames.iter().rev() {
        let taken = frame.free_factor_count.min(offer);
        total += taken;
        offer = frame.free_factor_count - taken;
    }
    u32::try_from(total).expect("frames have free factors")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_of_85() {
        // From FrameSetTest.java.
        let t = Tables::new(85);
        let cols = build_frames::<2>(&t);
        let shape: Vec<(Vec<u32>, Vec<u32>)> =
            cols.frames.iter().map(|f| (f.factors.to_vec(), f.moves.to_vec())).collect();
        assert_eq!(
            shape,
            vec![
                (vec![4, 6, 9, 10, 14, 15, 21, 25], vec![45, 50, 63, 75]),
                (vec![8, 12, 20, 28, 30, 42], vec![56, 60, 84]),
                (vec![16, 24, 36, 40], vec![48, 72, 80]),
            ]
        );
        assert_eq!(cols.candidates.to_vec(), vec![8, 12, 16, 20, 24, 28, 30, 36, 40, 42]);
    }

    #[test]
    fn no_frames_when_every_hive_is_closed() {
        for n in [1u32, 2, 10] {
            let cols = build_frames::<1>(&Tables::new(n));
            assert!(cols.frames.is_empty() && cols.candidates.is_empty() && cols.estimate_max_promotions == 0, "n={n}");
        }
    }

    #[test]
    fn estimate_works_down_from_the_top() {
        let frame = |free| FrameColumn::<1> {
            name: String::new(),
            factors: Bits::EMPTY,
            moves: Bits::EMPTY,
            free_factor_count: free,
        };
        // The top frame (last) absorbs nothing and can promote 2 into the middle one.  The middle one has 1
        // free factor, so absorbs only 1 and has none left to promote further; or it has 5, absorbs 2 and can
        // promote 3 into the bottom one, which has room for all of them.
        assert_eq!(estimate_max_promotions(&[frame(4), frame(1), frame(2)]), 1);
        assert_eq!(estimate_max_promotions(&[frame(4), frame(5), frame(2)]), 5);
        assert_eq!(estimate_max_promotions::<1>(&[]), 0);
    }
}
