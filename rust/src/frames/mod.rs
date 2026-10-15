//! The frames of a board: formed, simplified, numbered from the front and merged.  This is the setup that turns the
//! board `1..=n` into the input of the search.
//!
//! Where this fits: [`BoardFrames::new`] takes the tables of a board ([`crate::numtheory`]) and a set of promoted
//! numbers and forms the board's frames: each source with the factors it can use, the sources that share a factor in
//! one frame.  It then simplifies them: it drops the impossible sources, settles the mandatory moves (`unzip`) and
//! counts the freedoms.  [`build_frames`] numbers the frames that have freedoms from the front (0 is played first) and
//! merges those with the same number, which gives the [`SearchFrames`] the search works on.  The next step is
//! [`crate::model::FrameGraph::build`], which takes [`SearchFrames::columns`], and then [`crate::search`].  At the end
//! of every game [`BoardFrames::solution`], on the frames built with the winning promotions as sources, turns them into
//! the moves of a game.
//!
//! # The types
//!
//! * [`Frame`] is a frame as setup forms it, with what the simplification found (`mandatory`, `claimed`, `impossible`).
//!   A frame that is the merger of several is a `Frame` again.
//! * [`BoardFrames`] is all the frames of a board, in playing order.
//! * [`SearchFrames`] is what the search takes of them: for each frame with freedoms, numbered from the front, its
//!   moves and factors as a [`crate::model::Frame`], and the numbers that could be promoted.
//! * [`Numbering`] is the board's frames as [`BoardFrames::new`] forms them, before any is merged, each as a
//!   [`NumberedFrame`] with its number, the frames in front of it and the pairs `unzip` settled in it; it is for the
//!   census of the structure of the frames (`examples/structure.rs`), and the solver does not use it.
//!
//! The Java has several names for the one idea (DESIGN.md, "The Java solver and the frame solver", maps them); this
//! module calls it a frame throughout.  [`Frame`] has the name of [`crate::model::Frame`], the search's copy of it (a
//! `Copy` set of moves and factors that every promotion changes), and the two do not meet: no signature has both, and
//! this module writes the search's as `model::Frame`.  A second word (`FrameDraft`) would only be one more name for
//! what the wiki calls a frame.  The names of `unzip` and `removeImpossible`, and of the fields `mandatory` and
//! `claimed`, are the Java's.
//!
//! # Sources, factors and frames
//!
//! The frame model looks at the board `1..=n` through *maximal* factors (see [`crate::numtheory`]): a move
//! `m` is paid for by one of its maximal factors, and a factor can pay for only one move.  A number with no
//! maximal composite (every number above `n/2`) divides nothing, so it is never anybody's tax and the model
//! plays it unless it is found impossible (below): a *source*.  The search may also *promote* further numbers
//! (play a factor as a move), which are then treated as sources too (`treat_as_sources`).  Everything that is not
//! a source is a *factor*.  (A *sink* is a number all of whose maximal factors are promoted, so that nothing is
//! left to pay for it: promoting 8 on the board `1..=20` makes 16 a sink.  It is neither a move nor tax, and a
//! sink that is a source is found impossible.)
//!
//! A **frame** is a connected component of the graph that joins every source to its factors: a set of sources
//! together with all the factors they compete for, so that sources that share a factor are in the same frame.
//! Frames are independent of each other, except that a factor of one frame can be *promoted* into a source of a
//! frame in front of it.  Frame `X` is *in front of* frame `Y`, and `Y` *behind* `X`, when a number of `Y` has a
//! maximal factor in `X`: `Y`'s numbers are multiples of `X`'s.  A [`BoardFrames`] holds all the frames of one board,
//! in playing order, front first, in which their games can simply be concatenated.  (A move takes all of its
//! divisors, and they lie in its own frame or in front of it; so a frame played after the frames in front of it
//! finds its factors untouched.)
//!
//! The sources of a frame can be played together exactly when each can be given a factor of its own and the
//! moves can be put in an order that works; `BoardFrames::count_freedoms` decides that with a *peel*, described there.
//!
//! # Simplifying a frame
//!
//! After the frames are formed, [`BoardFrames::new`] simplifies each one until nothing changes (`finish_setup`).
//! Three things are decided frame by frame.  All three are part of the **frame model's assumptions**: they are
//! validated (the Java's table of optimal games is reproduced to n = 1000, and small boards agree with
//! exhaustive search) but not proven; README §1 and DESIGN.md (§2, "What optimal means", and §5, "Tactics") list
//! them.
//!
//! * **unzip** finds the frame's *mandatory* moves, in two ways: a source with a single usable factor must take
//!   it, and a factor wanted by a single source is given to that source.  A pair is settled only when the factor
//!   cannot be put to better use outside the frame, as tax for a number promoted from another frame or (in the
//!   second way) as a promoted number itself, a move of a frame in front of it.  Each settled pair of a move and
//!   its *claimed* factor leaves the frame's *remaining* sources and factors, which are what the frame shows the
//!   search.  The assumption: a frame plays all of its sources, so a source with one choice makes it.  (Earlier
//!   notes call these moves "forced"; this code keeps that word out of the frames, since the search has forced
//!   members of its own that are unrelated.)
//! * **removeImpossible** drops, greedily from the largest source down, every source that cannot be played
//!   together with the sources already kept.  They are the frame's *impossible* moves, which no game of the model
//!   plays.  The assumption: that this greedy choice keeps the heaviest playable set of sources.  It would, if the
//!   playable subsets of a frame's sources formed a matroid; nobody has shown that they do.
//! * The *freedoms* (the field `freedoms`) are `|factors| − |sources|`: how many factors the frame can
//!   spare, each promotion out of or into the frame using one up.  A frame with none is *closed* and takes no part
//!   in the search; one with some goes on to it.  The assumption: that this count is the frame's capacity.  (It is
//!   only a count: the search still checks every single promotion for winnability.)
//!
//! # Numbering the frames
//!
//! A frame that is **closed** (no freedom: it has exactly as many factors as moves) can neither spare a factor
//! nor take another move, so it takes no part in the search.  The frames that have freedoms are the ones that
//! matter: each can give up a factor (to be promoted into a move of the frame in front of it) and take in a
//! promoted number from a frame behind it.  They are numbered from the front, and all the frames with the same
//! number are merged into one frame, a small game of Taxman Mini: a set of moves and a set of factors.  Frame 0 is
//! the front (in the finished game its moves are played first); promoting a factor of frame `h` makes it a move of
//! frame `h − 1`, one frame forward.
//!
//! The frames are numbered breadth first from the front:
//!
//! * number 0 goes to every frame with freedoms that has no frame with freedoms directly in front of it;
//! * number `k + 1` goes to every frame with freedoms directly behind a frame numbered `k` that is not already
//!   numbered.
//!
//! ("Directly" is through `BoardFrames::frames_in_front` and `frames_behind`, which look at the board's maximal
//! factors and composites of every number in the frame.)  Among the frames with one number, the order is that of
//! their smallest source.  The construction is the Java's and does not depend on it, but in every board up to 2000
//! the numbering comes out clean: every frame with freedoms directly in front of a frame numbered `k > 0` is
//! numbered `k − 1` (and there is at least one), and no two frames with the same number are neighbours.
//! [`numbering`] exposes the numbers of the board's frames, for the census of the structure of the frames
//! (`examples/structure.rs`).
//!
//! # Why frames are merged
//!
//! Merging happens so that a promotion always has one frame to go to.  A promoted number is paid by one of its
//! maximal factors; if those lie in two frames with the same number, the promotion would have two frames to go to,
//! and as one merged frame they are the one frame it goes to.  It happens at n = 20: the frames of 14 (with the
//! factors 2, 7) and of 15 (with 3, 5) share no factor, and both sit directly in front of the frame of 12, 18, 20
//! (with 4, 6, 9, 10), whose 6 and 10 could be paid in either.  Both get number 0 and are merged into one frame
//! (`B+C` in the golden data, `testdata/golden/n0020.json`: the move 15 and the factors 2, 3, 5, as `unzip` pairs 14
//! with 7); the frame of 12, 18, 20 is frame 1.  At n = 21 it does not happen: the frames B (14, 15, 21 with 2, 3,
//! 5, 7) and C (12, 18, 20 with 4, 6, 9, 10) are the only ones with freedoms and C is behind B, so they are frames 0
//! and 1, and A (19) and D (16) are closed (`testdata/golden/n0021.json`).
//!
//! # An example
//!
//! The board `1..=85` (`tests::frames_of_85`, from the Java's tests) has three frames, front first:
//!
//! ```text
//! frame 0   moves 45 50 63 75    factors 4 6 9 10 14 15 21 25    4 freedoms
//! frame 1   moves 56 60 84       factors 8 12 20 28 30 42        3 freedoms
//! frame 2   moves 48 72 80       factors 16 24 36 40             1 freedom
//! candidates: 8 12 16 20 24 28 30 36 40 42
//! ```
//!
//! The moves are the sources that setup found neither mandatory nor impossible; the factors are the ones not used
//! up by mandatory moves; the *freedoms* are factors minus moves.  A promotion takes a number out of the factors of
//! its frame and adds it to the moves of the frame in front of it, which costs one freedom in each.
//!
//! # The order of operations matters
//!
//! The construction mirrors the Java `FrameBuilder`/`WorkingFrame`, and the order is part of what it computes,
//! because merging changes what the frames behind can do (a merged frame has more sources to place and more
//! factors to share): the promotion candidates of the frames with one number are collected from them separately,
//! *then* they are merged, and only then are the frames with the next number looked at.  Once all are merged,
//! every frame is simplified once more (`BoardFrames::finish_setup`), since merging forgets what the parts had found.
//!
//! # Parity with the Java
//!
//! The results of the Java solver are what this module has to reproduce, so the order of every loop in it is
//! deliberate: which source is declared impossible, or which pair is claimed first, can depend on the order.  The
//! golden tests (`tests/golden_frames.rs`) compare every field of every frame as setup forms it of the 318 boards in
//! `testdata/golden` (up to n = 760), and the frames before and after numbering and merging of the eleven wide
//! boards of `testdata/golden_wide` (up to n = 2048), with what the Java produced (the data keeps the Java's names,
//! which the test maps).  Two details reproduce quirks of the Java on purpose: a frame that has not been simplified yet
//! counts as having freedoms (`Frame::new`), and the stale-frame behaviour of `Home::MergedAway`.
//!
//! # What setup costs
//!
//! Almost all of it is the peel (`BoardFrames::count_freedoms`), which setup asks of whole frames thousands of times
//! for a board of a few thousand numbers.  Three things keep that cheap, and none changes an answer.  The peel works
//! on a graph as large as the sets it is given, not as the width of a set (`Peel`).  A frame remembers which
//! numbers it can take on as a source, until its sources change (`Frame::can_add`), and which of its sources
//! `removeImpossible` has been through (`Frame::swept`).  And `unzip` asks each question about a factor once, however
//! many passes visit it (`Answers`).
//!
//! # Punctured boards
//!
//! The frames can also be formed on a board with *holes* ([`Tables::punctured`]): numbers that are never in the
//! pot, so never moves and never tax, and belong to no frame.  Only the sacrifice study ([`crate::sacrifice`])
//! does this; the cases it needs are marked `Home::OffBoard` and "through holes" in `board_frames`.

mod board_frames;

pub use board_frames::{BoardFrames, ClaimKind, Frame};

use crate::bits::Bits;
use crate::model;
use crate::numtheory::Tables;

/// The frames of a board as the search takes them: those with freedoms, numbered from the front and merged, and the
/// numbers that could be promoted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchFrames<const W: usize> {
    /// In playing order: frame 0, the front, first.  A frame's `moves` are its sources that setup found neither
    /// mandatory nor impossible, the moves the search still has to place; its `factors` are those not used up by
    /// mandatory moves; its `freedoms`, `|factors| − |moves|`, are how many factors it can spare, each promotion into
    /// the frame or out of it using one up.  (They are also the [`Frame::freedoms`] of the merged frame: the mandatory
    /// moves take one factor each.)
    pub frames: Vec<model::Frame<W>>,
    /// The names setup gave the merged frames (A, B, ...), joined with `+`, one for each of `frames`; for
    /// diagnostics only.
    pub names: Vec<String>,
    /// For each of `frames`, the pairs `unzip` settled in the merged frame as the final `finish_setup` left it (the
    /// [`Frame::claims`] of the frame that `frames` takes its remaining moves and factors from), in the order made.
    /// For the census of the structure of the frames (`examples/structure.rs`); nothing in the solver reads them.
    pub claims: Vec<Vec<(u32, u32, ClaimKind)>>,
    /// Every number that could be promoted, in any frame: for each frame with freedoms, the factors it could spare
    /// that a frame in front of it could take on as a source ([`BoardFrames::promotion_candidates_of`]), collected
    /// number by number before the frames with that number are merged.
    pub candidates: Bits<W>,
}

impl<const W: usize> SearchFrames<W> {
    /// The frames in the shape [`crate::model::FrameGraph::build`] takes: for each frame, front first, its factors
    /// and then its moves.
    pub fn columns(&self) -> Vec<Vec<u32>> {
        self.frames.iter().flat_map(|f| [f.factors.to_vec(), f.moves.to_vec()]).collect()
    }
}

/// The board's frames as [`BoardFrames::new`] forms them with no promotions, before any merge, in playing order, with
/// the numbers [`build_frames`] gives them.  See [`numbering`].
#[derive(Clone, Debug)]
pub struct Numbering<const W: usize> {
    pub frames: Vec<NumberedFrame<W>>,
    /// [`BoardFrames::merged_away_reach`] of the board's frames once `build_frames` has merged and simplified them.
    pub merged_away_reach: (u32, u32),
}

/// One of the board's frames before any merge, as [`Numbering`] lists it.
#[derive(Clone, Debug)]
pub struct NumberedFrame<const W: usize> {
    pub name: String,
    /// The sources setup keeps (mandatory ones included), the impossible ones apart.
    pub sources: Bits<W>,
    pub impossible: Bits<W>,
    pub factors: Bits<W>,
    pub freedoms: i32,
    pub closed: bool,
    /// The pairs `unzip` settled before any merge.
    pub claims: Vec<(u32, u32, ClaimKind)>,
    /// The frame's number (0 = the front), `None` if it has none (closed).
    pub number: Option<usize>,
    /// `BoardFrames::frames_in_front` of the frame: positions in [`Numbering::frames`].
    pub in_front: Vec<usize>,
}

/// Builds the frames of the board `1..=t.n` with no promotions.
pub fn build_frames<const W: usize>(t: &Tables) -> SearchFrames<W> {
    build(t, false).0
}

/// [`build_frames`], and with it the board's frames as they were before any was merged, with their numbers: what the
/// census of the structure of the frames (`examples/structure.rs`) looks at.  The frames the search takes are built
/// by the same function as the solver's, so the census sees exactly what the solver builds.
pub fn numbering<const W: usize>(t: &Tables) -> (Numbering<W>, SearchFrames<W>) {
    let (frames, listed) = build(t, true);
    (listed.expect("a numbering was asked for"), frames)
}

/// The work of [`build_frames`] and [`numbering`]; the numbering is made only when `record` is set, so the solver's
/// path pays nothing for it.
fn build<const W: usize>(t: &Tables, record: bool) -> (SearchFrames<W>, Option<Numbering<W>>) {
    let mut board_frames = BoardFrames::<W>::new(t, Bits::EMPTY);

    // The frames with freedoms are numbered on the board's frames as they are, before anything is merged.  A frame
    // is named by its smallest factor, because merging changes the positions of the frames but not what they hold.
    // (A frame with freedoms has factors.)
    let Numbers { groups: numbered, number, in_front } = numbered_frames(&board_frames);
    let pre_merge = record.then(|| describe_frames(&board_frames, &number, in_front));
    let mut candidates = Bits::EMPTY;
    for same_number in &numbered {
        let positions: Vec<usize> =
            same_number.iter().map(|&f| board_frames.frame_index_of(f).expect("not yet merged")).collect();
        for &p in &positions {
            candidates |= board_frames.promotion_candidates_of(p);
        }
        if positions.len() > 1 {
            board_frames.merge(&positions);
        }
    }
    // Merging forgot what the parts had found (the mandatory and impossible moves); find it again for the merged
    // frames.
    board_frames.finish_setup();

    let (mut frames, mut names) = (Vec::with_capacity(numbered.len()), Vec::with_capacity(numbered.len()));
    let mut claims = Vec::with_capacity(numbered.len());
    for same_number in &numbered {
        let at = board_frames.frame_index_of(same_number[0]).expect("a numbered frame is one of the board's frames");
        let frame = &board_frames.frames()[at];
        let search_frame = model::Frame::new(frame.remaining_sources(), frame.remaining_factors());
        debug_assert_eq!(search_frame.freedoms, frame.freedoms(), "frame {}", frame.name());
        frames.push(search_frame);
        names.push(frame.name().to_owned());
        claims.push(frame.claims().to_vec());
    }
    let listed = pre_merge.map(|frames| Numbering { frames, merged_away_reach: board_frames.merged_away_reach() });
    (SearchFrames { frames, names, claims, candidates }, listed)
}

/// The board's frames as [`Numbering`] lists them: each with its number (`number`, by position among the frames) and
/// the frames in front of it (`in_front`, as `numbered_frames` left them: those of a closed frame are found here).
fn describe_frames<const W: usize>(
    board_frames: &BoardFrames<W>,
    number: &[Option<usize>],
    in_front: Vec<Option<Vec<usize>>>,
) -> Vec<NumberedFrame<W>> {
    board_frames
        .frames()
        .iter()
        .zip(number)
        .zip(in_front)
        .enumerate()
        .map(|(i, ((frame, &number), in_front))| NumberedFrame {
            name: frame.name().to_owned(),
            sources: frame.sources(),
            impossible: frame.impossible(),
            factors: frame.factors(),
            freedoms: frame.freedoms(),
            closed: frame.is_closed(),
            claims: frame.claims().to_vec(),
            number,
            in_front: in_front.unwrap_or_else(|| board_frames.frames_in_front(i)),
        })
        .collect()
}

/// What `numbered_frames` makes of the board's frames.
struct Numbers {
    /// One entry per number, in order, listing the frames that got it, each named by its smallest factor; the frames
    /// of one entry are the ones to merge.
    groups: Vec<Vec<u32>>,
    /// The number of each of the board's frames, by position; `None` for a closed frame, and for a frame with
    /// freedoms that no number reached (which cannot happen, the frames being a DAG).
    number: Vec<Option<usize>>,
    /// The frames in front of each frame with freedoms, which the numbering looked at (`BoardFrames::frames_in_front`),
    /// kept so that nothing is found twice; `None` for a closed frame, which it does not look at.
    in_front: Vec<Option<Vec<usize>>>,
}

/// Numbers the frames that have freedoms from the front (0 is played first) and groups them by number: one entry
/// per number, in order, listing the frames that got it, each named by its smallest factor; the frames of one entry
/// are the ones to merge.  Among the frames with one number, the order is that of their smallest source.  See the
/// module documentation for how the numbers are formed.  Besides the groups, the result has every frame's number and
/// the frames in front of the frames with freedoms ([`Numbers`]).
fn numbered_frames<const W: usize>(board_frames: &BoardFrames<W>) -> Numbers {
    let frames = board_frames.frames();
    let is_closed = |i: usize| frames[i].is_closed();
    let by_smallest_source = |i: &usize| frames[*i].sources().min();

    let mut in_front: Vec<Option<Vec<usize>>> = vec![None; frames.len()];
    let mut current: Vec<usize> = Vec::new();
    for i in (0..frames.len()).filter(|&i| !is_closed(i)) {
        let in_front_of_i = board_frames.frames_in_front(i);
        if in_front_of_i.iter().all(|&j| is_closed(j)) {
            current.push(i);
        }
        in_front[i] = Some(in_front_of_i);
    }
    current.sort_by_key(by_smallest_source);

    let mut number: Vec<Option<usize>> = vec![None; frames.len()];
    let mut groups = Vec::new();
    while !current.is_empty() {
        groups.push(
            current.iter().map(|&i| frames[i].factors().min().expect("a frame with freedoms has factors")).collect(),
        );
        for &i in &current {
            number[i] = Some(groups.len() - 1);
        }
        let mut next: Vec<usize> = Vec::new();
        for &i in &current {
            for behind in board_frames.frames_behind(i) {
                if number[behind].is_none() && !is_closed(behind) && !next.contains(&behind) {
                    next.push(behind);
                }
            }
        }
        next.sort_by_key(by_smallest_source);
        current = next;
    }
    Numbers { groups, number, in_front }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_of_85() {
        // From FrameSetTest.java.
        let t = Tables::new(85);
        let found = build_frames::<2>(&t);
        let shape: Vec<(Vec<u32>, Vec<u32>)> =
            found.frames.iter().map(|f| (f.factors.to_vec(), f.moves.to_vec())).collect();
        assert_eq!(
            shape,
            vec![
                (vec![4, 6, 9, 10, 14, 15, 21, 25], vec![45, 50, 63, 75]),
                (vec![8, 12, 20, 28, 30, 42], vec![56, 60, 84]),
                (vec![16, 24, 36, 40], vec![48, 72, 80]),
            ]
        );
        assert_eq!(found.candidates.to_vec(), vec![8, 12, 16, 20, 24, 28, 30, 36, 40, 42]);
        let freedoms: Vec<i32> = found.frames.iter().map(|f| f.freedoms).collect();
        assert_eq!(freedoms, [4, 3, 1]);
        assert_eq!(found.names.len(), found.frames.len());
    }

    #[test]
    fn no_frames_when_every_frame_is_closed() {
        for n in [1u32, 2, 10] {
            let found = build_frames::<1>(&Tables::new(n));
            assert!(found.frames.is_empty() && found.candidates.is_empty(), "n={n}");
        }
    }

    /// The numbering that the census looks at is the one `build_frames` uses, on every board up to 400: `numbering`
    /// builds the same search frames; the board's frames, grouped by number, are the groups that `build_frames`
    /// merges; only the closed frames have no number; and the frames in front are those of `frames_in_front`.
    #[test]
    fn the_numbering_is_what_build_frames_uses() {
        for n in 2..=400u32 {
            let t = Tables::new(n);
            let (numbered, found) = numbering::<8>(&t);
            assert_eq!(found, build_frames::<8>(&t), "n={n}");

            let board_frames = BoardFrames::<8>::new(&t, Bits::EMPTY);
            assert_eq!(numbered.frames.len(), board_frames.frames().len(), "n={n}");
            for (i, (listed, frame)) in numbered.frames.iter().zip(board_frames.frames()).enumerate() {
                let at = format!("n={n} frame {}", frame.name());
                assert_eq!(listed.name, frame.name(), "{at}");
                assert_eq!(listed.sources, frame.sources(), "{at}");
                assert_eq!(listed.impossible, frame.impossible(), "{at}");
                assert_eq!(listed.factors, frame.factors(), "{at}");
                assert_eq!((listed.freedoms, listed.closed), (frame.freedoms(), frame.is_closed()), "{at}");
                assert_eq!(listed.claims, frame.claims(), "{at}");
                assert_eq!(listed.number.is_none(), listed.closed, "{at}: only a closed frame has no number");
                assert_eq!(listed.in_front, board_frames.frames_in_front(i), "{at}");
            }

            let count = numbered.frames.iter().filter_map(|f| f.number).max().map_or(0, |last| last + 1);
            assert_eq!(count, found.frames.len(), "n={n}: the numbers");
            assert_eq!((found.names.len(), found.claims.len()), (count, count), "n={n}: one entry per number");
            for h in 0..count {
                // The order of the merge: by smallest source.
                let mut group: Vec<&NumberedFrame<8>> =
                    numbered.frames.iter().filter(|f| f.number == Some(h)).collect();
                assert!(!group.is_empty(), "n={n}: no frame has the number {h}");
                group.sort_by_key(|f| f.sources.min());
                let names: Vec<&str> = group.iter().map(|f| f.name.as_str()).collect();
                assert_eq!(names.join("+"), found.names[h], "n={n} number {h}");

                // The merged frame holds the parts' factors, those its mandatory moves claim among them, and no
                // source the parts did not have.
                let factors = group.iter().fold(Bits::EMPTY, |acc, f| acc | f.factors);
                let sources = group.iter().fold(Bits::EMPTY, |acc, f| acc | f.sources);
                let claimed: Bits<8> = found.claims[h].iter().map(|&(_, f, _)| f).collect();
                let claiming: Bits<8> = found.claims[h].iter().map(|&(s, _, _)| s).collect();
                assert_eq!(factors, found.frames[h].factors | claimed, "n={n} number {h}: factors");
                assert!((found.frames[h].moves | claiming).is_subset(&sources), "n={n} number {h}: sources");
            }
        }
    }

    /// The module's examples: the board of 85 has three numbers; on the board of 20 the frames of 14 and of 15 are
    /// both number 0 (and so merged), and the frame of 12, 18, 20 is number 1.
    #[test]
    fn the_numbers_of_the_examples() {
        let (numbered, found) = numbering::<2>(&Tables::new(85));
        let mut numbers: Vec<usize> = numbered.frames.iter().filter_map(|f| f.number).collect();
        numbers.sort_unstable();
        numbers.dedup();
        assert_eq!((numbers, found.frames.len()), (vec![0, 1, 2], 3));

        let (numbered, found) = numbering::<1>(&Tables::new(20));
        let frame_of = |x: u32| numbered.frames.iter().position(|f| f.sources.contains(x)).expect("a source");
        let number_of = |x: u32| numbered.frames[frame_of(x)].number;
        assert_ne!(frame_of(14), frame_of(15));
        assert_eq!((number_of(14), number_of(15), number_of(12)), (Some(0), Some(0), Some(1)));
        assert_eq!(found.names[0].matches('+').count(), 1);
    }

    /// The quirk of `Home::MergedAway` is reached on the board of 114 and on no other board up to 400, twice, and
    /// neither time does it make a factor wanted outside its frame (`BoardFrames::merged_away_reach`).
    #[test]
    fn the_reach_of_the_merged_away_quirk() {
        for n in 2..=400u32 {
            let (numbered, _) = numbering::<8>(&Tables::new(n));
            let expected = if n == 114 { (2, 0) } else { (0, 0) };
            assert_eq!(numbered.merged_away_reach, expected, "n={n}");
        }
    }

    /// The solver takes a prime game's promotions from the candidates of the board's frames before any is merged
    /// (the prime shortcut), the frame builder collects them number by number while merging; they are the same set
    /// on every board up to 2000 (checked once; this guards the small boards).
    #[test]
    fn the_candidates_of_the_frames_are_those_of_the_board_frames() {
        for n in 2..=300u32 {
            let t = Tables::new(n);
            let plain = BoardFrames::<5>::new(&t, Bits::EMPTY).promotion_candidates();
            assert_eq!(build_frames::<5>(&t).candidates, plain, "n={n}");
        }
    }
}
