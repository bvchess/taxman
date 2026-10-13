//! Frames: the hives of a board stacked into levels, which is the shape the search works on.
//!
//! Where this fits: [`build_frames`] takes the [`Apiary`] of a board (see [`crate::hives`] for the vocabulary:
//! sources, factors, hives, free and closed hives) and turns it into the input of the search, a list of
//! [`FrameColumn`]s plus the numbers that could be promoted.  The next step is [`crate::model::Model::build`],
//! which takes [`FrameColumns::columns`], and then [`crate::search`].
//!
//! # What a frame is
//!
//! A hive that is **closed** (no free factor: it has exactly as many factors as moves) can neither spare a factor
//! nor take another move, so it takes no part in the search.  The **free** hives are the ones that matter: each
//! can give up a factor (to be promoted into a move of the hive below) and take in a promoted number from the hive
//! above.  They are grouped into *levels*, and all the hives of a level are merged into one **frame**, a small
//! game of Taxman Mini: a set of moves and a set of factors.  Frame 0 is the bottom, the downstream end (in the
//! finished game its hives are played first); promoting a factor of frame `h` makes it a move of frame `h − 1`.
//!
//! The levels are built breadth first from the bottom:
//!
//! * level 0 is every free hive that has no free hive directly downstream of it;
//! * level `k + 1` is every free hive directly upstream of level `k` that is not already placed.
//!
//! ("Directly" is through `Apiary::downstream_hives` and `upstream_hives`, which look at the board's maximal
//! factors and composites of every number in the hive.)  Within a level the hives are ordered by their smallest
//! source.  The construction is the Java's and does not depend on it, but in every board up to 2000 the layering
//! comes out clean: every free hive directly downstream of a hive of level `k > 0` is in level `k − 1` (and there
//! is at least one), and no two hives of a level are neighbours.
//!
//! # An example
//!
//! The board `1..=85` (`tests::frames_of_85`, from the Java's tests) has three frames, bottom first:
//!
//! ```text
//! frame 0   moves 45 50 63 75    factors 4 6 9 10 14 15 21 25    4 freedoms
//! frame 1   moves 56 60 84       factors 8 12 20 28 30 42        3 freedoms
//! frame 2   moves 48 72 80       factors 16 24 36 40             1 freedom
//! candidates: 8 12 16 20 24 28 30 36 40 42
//! ```
//!
//! The moves are the sources the hives found neither mandatory nor impossible; the factors are the ones not used
//! up by mandatory moves; the *freedoms* are factors minus moves.  A promotion takes a number out of the factors of
//! its frame and adds it to the moves of the frame below, which costs one freedom in each.
//!
//! # The order of operations matters
//!
//! The construction mirrors the Java `FrameBuilder`/`WorkingFrame`, and the order is part of what it computes,
//! because merging changes what the hives above can do (a merged hive has more sources to place and more factors
//! to share): the promotion candidates of a level are collected from its separate hives, *then* the level is
//! merged, and only then is the next level looked at.  Once all levels are merged, every hive is simplified once
//! more (`Apiary::finish_setup`), since merging forgets what the parts had found.

use crate::bits::Bits;
use crate::hives::Apiary;
use crate::numtheory::Tables;

/// One frame: a level of hives, merged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameColumn<const W: usize> {
    /// The merged hives' names joined with `+`; for diagnostics only.
    pub name: String,
    /// The frame's factors that are not already used up by mandatory moves.
    pub factors: Bits<W>,
    /// The frame's sources that are neither mandatory nor impossible: the moves the search still has to place.
    pub moves: Bits<W>,
    /// `|factors| − |sources|` of the merged hive, the frame's *freedoms*: how many spare factors it has.  Each
    /// promotion into the frame or out of it uses one up.  (It equals `factors.len() − moves.len()`: the mandatory
    /// moves take one factor each.)
    pub free_factor_count: i32,
}

/// The frames of a board, with what the search needs to know about them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameColumns<const W: usize> {
    /// Bottom (downstream) first.
    pub frames: Vec<FrameColumn<W>>,
    /// Every number that could be promoted, in any frame: for each free hive, the factors it could spare that the
    /// hive below could take on as a source ([`Apiary::hive_promotion_candidates`]), collected level by level
    /// before the level is merged.
    pub candidates: Bits<W>,
    /// An upper bound on how many promotions there can be, from the frames' freedoms alone (the Java's
    /// `estimateMaxPromotions`; `reuse_prev` is its only user here).
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

    // The levels are found on the apiary as it is, before anything is merged.  A hive is named by its smallest
    // factor, because merging changes the positions of the hives but not what they hold.  (A free hive has
    // factors.)
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
    // Merging forgot what the parts had found (the mandatory and impossible moves); find it again for the merged
    // hives.
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
    let estimate_max_promotions = max_promotions_by_freedoms(&frames);
    FrameColumns { frames, candidates, estimate_max_promotions }
}

/// The levels of hives that still have free factors, bottom first, each hive named by its smallest factor.
/// Within a level the hives are ordered by their smallest source.  See the module documentation for how the
/// levels are formed.
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

/// How many promotions there can be at most, given only the frames' freedoms (the Java's `estimateMaxPromotions`),
/// working from the top frame down.
///
/// A promotion from frame `h` to frame `h − 1` uses one freedom of each.  The top frame has nothing above it to
/// absorb, so all its freedoms are on offer to the frame below.  Each lower frame absorbs as many of the offered
/// promotions as it has freedoms (each absorbed one counts once, in the total) and offers what is left of its
/// freedoms to the frame below it; frame 0 offers to no one.  Taking promotions as early as possible is the best
/// use of a freedom, since an absorbed promotion costs the frame above only a freedom it could not use otherwise.
///
/// [`crate::model::FrameSet::max_promotions`] computes the same number from the bottom up, from the search state.
fn max_promotions_by_freedoms<const W: usize>(frames: &[FrameColumn<W>]) -> u32 {
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
        let freedoms: Vec<i32> = cols.frames.iter().map(|f| f.free_factor_count).collect();
        assert_eq!(freedoms, [4, 3, 1]);
    }

    #[test]
    fn no_frames_when_every_hive_is_closed() {
        for n in [1u32, 2, 10] {
            let cols = build_frames::<1>(&Tables::new(n));
            assert!(cols.frames.is_empty() && cols.candidates.is_empty() && cols.estimate_max_promotions == 0, "n={n}");
        }
    }

    /// `reuse_prev` takes its candidates from the unmerged apiary, the frame builder collects them level by level
    /// while merging; the documentation of `reuse_prev` says they are the same set on every board up to 2000
    /// (checked once; this guards the small boards).
    #[test]
    fn the_frames_candidates_are_the_plain_apiarys() {
        for n in 2..=300u32 {
            let t = Tables::new(n);
            let plain = Apiary::<5>::new(&t, Bits::EMPTY).promotion_candidates();
            assert_eq!(build_frames::<5>(&t).candidates, plain, "n={n}");
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
        assert_eq!(max_promotions_by_freedoms(&[frame(4), frame(1), frame(2)]), 1);
        assert_eq!(max_promotions_by_freedoms(&[frame(4), frame(5), frame(2)]), 5);
        assert_eq!(max_promotions_by_freedoms::<1>(&[]), 0);
    }
}
