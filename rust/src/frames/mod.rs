//! The frames of a board: formed, simplified, numbered by layer and merged.  This is the setup that turns the board
//! `1..=n` into the input of the search.
//!
//! Where this fits: [`BoardFrames::new`] takes the tables of a board ([`crate::numtheory`]) and a set of promoted
//! numbers and forms the board's frames: each source with the factors it can use, the sources that share a factor in
//! one frame.  It then simplifies them: it drops the impossible sources, settles the mandatory moves (`unzip`) and
//! counts the freedoms.  [`build_frames`] numbers the frames that have freedoms by layer, from the front (0 is played
//! first), and merges those with the same number, which gives the [`SearchFrames`] the search works on.  The next step is
//! [`crate::model::FrameGraph::build`], which takes [`SearchFrames::columns`], and then [`crate::search`].  At the end
//! of every game [`BoardFrames::solution`], on the frames built with the winning promotions as sources, turns them into
//! the moves of a game.
//!
//! # The types
//!
//! * [`Frame`] is a frame as setup forms it, with what the simplification found (`mandatory`, `claimed`, `impossible`).
//!   A frame that is the merger of several is a `Frame` again.
//! * [`BoardFrames`] is all the frames of a board, in playing order.
//! * [`SearchFrames`] is what the search takes of them: for each frame with freedoms, numbered by layer, its
//!   moves and factors as a [`crate::model::Frame`], and the numbers that could be promoted.
//! * [`Numbering`] is the board's frames as [`BoardFrames::new`] forms them, before any is merged, each as a
//!   [`NumberedFrame`] with its number, the frames in front of it and the pairs `unzip` settled in it; it is for the
//!   census of the structure of the frames (`examples/structure.rs`), and the solver does not use it.
//! * [`ExtendedFrames`] is the frames of the extended problem, in which some of the sources of three or more prime
//!   factors are optional and `unzip` settles nothing ([`build_extended`]); research code, which the solver does not
//!   use either.
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
//! Three things are decided frame by frame.  All three were assumptions of the frame model, validated but not proven;
//! they are theorems: heuristics/THEORY.md, "The frame model: what is proven and what remains" (Theorem M and its
//! lemmas M1 to M6, which follow this code step by step), summed up in DESIGN.md §2, "What optimal means".  What remains is a statement about the game, not about this code:
//! that some optimal game plays every source with three or more prime factors that setup keeps.
//!
//! * **unzip** finds the frame's *mandatory* moves, in two ways: a source with a single usable factor must take
//!   it, and a factor wanted by a single source is given to that source.  A pair is settled only when the factor
//!   cannot be put to better use outside the frame, as tax for a number promoted from another frame or (in the
//!   second way) as a promoted number itself, a move of a frame in front of it.  Each settled pair of a move and
//!   its *claimed* factor leaves the frame's *remaining* sources and factors, which are what the frame shows the
//!   search.  Every such pair is a move that any game playing the frame's sources must make, as the frame takes
//!   them with whatever is promoted into it (THEORY.md, M5); a pair of the second kind needs that no promotion
//!   candidate has the factor among its maximal factors, which the candidate filter makes so (`examples/structure.rs`
//!   checks it on every board).  (Earlier notes call these moves "forced"; this code keeps that word out of the
//!   frames, since the search has forced members of its own that are unrelated.)
//! * **removeImpossible** drops, greedily from the largest source down, every source that cannot be played
//!   together with the sources already kept.  They are the frame's *impossible* moves, which no game of the model
//!   plays.  What the greedy keeps is U\* (THEORY.md, M1), the heaviest playable set of sources wherever the
//!   upper-half conjecture holds (certified on every board to 100,000), although the playable subsets of a frame's
//!   sources are not a matroid; that an optimal game plays all of U\* is proven for the sources with one or two prime
//!   factors (THEORY.md, Theorem B) and is the open statement for the others.
//! * The *freedoms* (the field `freedoms`) are `|factors| − |sources|`: how many factors the frame can
//!   spare, each promotion out of or into the frame using one up.  A frame with none is *closed* and takes no part
//!   in the search; one with some goes on to it.  A closed frame's factors all pay its own sources in every game
//!   that plays them (THEORY.md, M2), and the count is only a count: the search still checks every promotion for
//!   winnability, which implies it.
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
//! The frames are numbered by *layer*.  The layer of a frame is the number of prime factors, counted with
//! multiplicity, of its sources (`layer_of`): every source of a frame has the same count, every factor one fewer, and
//! every frame in front of it (one holding a maximal factor of one of its numbers, `BoardFrames::frames_in_front`)
//! lies one layer lower (heuristics/THEORY.md, Lemma Ω; `build_frames` asserts it in debug builds).  With `ℓ₀` the
//! lowest layer of a frame with freedoms, a frame with freedoms of layer `ℓ` gets the number `ℓ − ℓ₀`; a closed frame
//! gets none.  Among the frames with one number, the order is that of their smallest source.
//!
//! So the numbering is *clean* by construction: every frame with freedoms directly in front of a frame numbered `k`
//! is numbered `k − 1`, and frames with one number are never neighbours.  A promoted number, paid for by one of its
//! maximal factors, finds them one layer lower, in the frame numbered one lower or in closed frames, which is what the
//! frame model's exactness needs (heuristics/THEORY.md, Theorem M, lemma M3).  The Java numbered the frames breadth
//! first from the front instead (number 0 to every frame with freedoms with no frame with freedoms directly in front
//! of it, `k + 1` to every one directly behind a frame numbered `k` and not yet numbered); on every board up to 8191,
//! the largest the solver supports, the two numberings are the same (the census `examples/structure.rs`,
//! `bench/structure/`, and this module's tests), so the frames, candidates and searches are those of the Java's
//! numbering.  [`numbering`] exposes the numbers of the board's frames, for the census of the structure of the frames.
//!
//! A layer between `ℓ₀` and the highest layer with freedoms that has no frame with freedoms leaves its number without
//! frames: the search then has an *empty* frame there (no moves, no factors, no freedoms), through which nothing is
//! promoted.  No board `1..=n` has one; a punctured board can (below).
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
//! does this; the cases it needs are marked `Home::OffBoard` and "through holes" in `board_frames`.  The frames of a
//! punctured board lie in layers too (a number whose maximal composites are all holes is a frame of its own, with no
//! source, one layer above that number), and they are numbered by layer like any others.  There the Java's
//! breadth-first numbering can differ and is not always clean (the board of 56 without 39 has a frame numbered 1 with
//! a frame with freedoms numbered 1 directly in front of it), and a number can be without frames (the board of 32
//! without 4: taking 4 off splits the frame of layer 3 into two closed frames, while layers 2 and 4 have freedoms).

mod board_frames;
mod extended;

pub use board_frames::{BoardFrames, ClaimKind, Frame};
pub use extended::{build_extended, ExtendedFrames};

use crate::bits::Bits;
use crate::model;
use crate::numtheory::Tables;

/// The frames of a board as the search takes them: those with freedoms, numbered by layer and merged, and the
/// numbers that could be promoted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchFrames<const W: usize> {
    /// In playing order: frame 0, the front, first.  A frame's `moves` are its sources that setup found neither
    /// mandatory nor impossible, the moves the search still has to place; its `factors` are those not used up by
    /// mandatory moves; its `freedoms`, `|factors| − |moves|`, are how many factors it can spare, each promotion into
    /// the frame or out of it using one up.  (They are also the [`Frame::freedoms`] of the merged frame: the mandatory
    /// moves take one factor each.)  A number that no frame with freedoms has (on a punctured board, a layer between
    /// the lowest and the highest with freedoms that holds only closed frames) is an empty frame: no moves, no
    /// factors, no freedoms.
    pub frames: Vec<model::Frame<W>>,
    /// The names setup gave the merged frames (A, B, ...), joined with `+`, one for each of `frames`; `-` for an
    /// empty frame.  For diagnostics only.
    pub names: Vec<String>,
    /// For each of `frames`, the pairs `unzip` settled in the merged frame as the final `finish_setup` left it (the
    /// [`Frame::claims`] of the frame that `frames` takes its remaining moves and factors from), in the order made;
    /// none for an empty frame.  For the census of the structure of the frames (`examples/structure.rs`); nothing in
    /// the solver reads them.
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
        columns_of(&self.frames)
    }
}

/// `frames` in the shape [`crate::model::FrameGraph::build`] takes: for each frame, front first, its factors and then
/// its moves.
fn columns_of<const W: usize>(frames: &[model::Frame<W>]) -> Vec<Vec<u32>> {
    frames.iter().flat_map(|f| [f.factors.to_vec(), f.moves.to_vec()]).collect()
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

/// Builds the frames of the board `1..=t.n` with no promotions: the frames that have freedoms, numbered by layer (see
/// "Numbering the frames" above) and merged by number.
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
    let Numbers { groups: numbered, number, layers } = numbered_frames(&board_frames);
    debug_assert_eq!(lemma_omega_violation(&board_frames, &layers), None, "the frames do not lie in layers");
    let pre_merge = record.then(|| describe_frames(&board_frames, &number));
    let mut candidates = Bits::EMPTY;
    // A number that no frame has is an empty group: it adds no candidates and merges nothing.
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
        let Some(&first) = same_number.first() else {
            frames.push(model::Frame::new(Bits::EMPTY, Bits::EMPTY));
            names.push("-".to_owned());
            claims.push(Vec::new());
            continue;
        };
        let at = board_frames.frame_index_of(first).expect("a numbered frame is one of the board's frames");
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
/// the frames in front of it (`BoardFrames::frames_in_front`).
fn describe_frames<const W: usize>(board_frames: &BoardFrames<W>, number: &[Option<usize>]) -> Vec<NumberedFrame<W>> {
    board_frames
        .frames()
        .iter()
        .zip(number)
        .enumerate()
        .map(|(i, (frame, &number))| NumberedFrame {
            name: frame.name().to_owned(),
            sources: frame.sources(),
            impossible: frame.impossible(),
            factors: frame.factors(),
            freedoms: frame.freedoms(),
            closed: frame.is_closed(),
            claims: frame.claims().to_vec(),
            number,
            in_front: board_frames.frames_in_front(i),
        })
        .collect()
}

/// What `numbered_frames` makes of the board's frames.
struct Numbers {
    /// One entry per number `h` = 0, 1, ..., up to the largest number given, in order, listing the frames with
    /// freedoms of layer `l0 + h`, each named by its smallest factor, in the order of their smallest source; the
    /// frames of one entry are the ones to merge.  An entry is empty when no frame with freedoms lies in that layer.
    groups: Vec<Vec<u32>>,
    /// The number of each of the board's frames, by position: `layer − l0` for a frame with freedoms, `l0` being the
    /// lowest layer of one; `None` for a closed frame.
    number: Vec<Option<usize>>,
    /// The layer of each of the board's frames, by position, closed ones included ([`layer_of`]).
    layers: Vec<u32>,
}

/// The layer of a frame (heuristics/THEORY.md, Lemma Ω): Ω of its sources, the number of their prime factors counted
/// with multiplicity.  The frame's sources and impossible sources all have the same Ω, so this is Ω of the smallest of
/// them.  A frame that has neither (only a punctured board has one: a factor whose maximal composites are all holes)
/// is a single factor, one layer above that factor's.
pub(crate) fn layer_of<const W: usize>(frame: &Frame<W>, t: &Tables) -> u32 {
    match (frame.sources() | frame.impossible()).min() {
        Some(source) => t.omega(source),
        None => t.omega(frame.factors().min().expect("a frame holds a number")) + 1,
    }
}

/// Numbers the frames that have freedoms by layer and groups them by number: the number of a frame is its layer minus
/// `l0`, the lowest layer of a frame with freedoms, so 0 is played first.  The result has one entry per number, in
/// order, listing the frames that got it, each named by its smallest factor; the frames of one entry are the ones to
/// merge.  Among the frames with one number, the order is that of their smallest source.  See the module
/// documentation for how the numbers are formed.  Besides the groups, the result has every frame's number and layer
/// ([`Numbers`]).
fn numbered_frames<const W: usize>(board_frames: &BoardFrames<W>) -> Numbers {
    let (frames, t) = (board_frames.frames(), board_frames.tables());
    let layers: Vec<u32> = frames.iter().map(|frame| layer_of(frame, t)).collect();
    let open: Vec<usize> = (0..frames.len()).filter(|&i| !frames[i].is_closed()).collect();

    let mut number: Vec<Option<usize>> = vec![None; frames.len()];
    let mut same_number: Vec<Vec<usize>> = Vec::new();
    if let Some(lowest) = open.iter().map(|&i| layers[i]).min() {
        for &i in &open {
            let h = (layers[i] - lowest) as usize;
            number[i] = Some(h);
            if same_number.len() <= h {
                same_number.resize_with(h + 1, Vec::new);
            }
            same_number[h].push(i);
        }
    }
    let groups = same_number
        .into_iter()
        .map(|mut group| {
            group.sort_by_key(|&i| frames[i].sources().min());
            group.iter().map(|&i| frames[i].factors().min().expect("a frame with freedoms has factors")).collect()
        })
        .collect();
    Numbers { groups, number, layers }
}

/// Where the frames of the board do not lie in layers (heuristics/THEORY.md, Lemma Ω), given the `layers` of the
/// frames ([`layer_of`]): a message naming the board, the first frame that fails and what fails, or `None`.  In a
/// frame of layer `k`, every source and impossible source has Ω equal to `k`, every factor has Ω equal to `k − 1`, and
/// every frame in front of it ([`BoardFrames::frames_in_front`]) is of layer `k − 1`.
fn lemma_omega_violation<const W: usize>(board_frames: &BoardFrames<W>, layers: &[u32]) -> Option<String> {
    let t = board_frames.tables();
    let board = if t.holes.is_empty() { format!("n={}", t.n) } else { format!("n={} without {:?}", t.n, t.holes) };
    for (i, frame) in board_frames.frames().iter().enumerate() {
        let layer = layers[i];
        let held = frame.sources() | frame.impossible();
        let failed = if let Some(s) = held.iter().find(|&s| t.omega(s) != layer) {
            format!("its source {s} has {} prime factors", t.omega(s))
        } else if let Some(f) = frame.factors().iter().find(|&f| t.omega(f) + 1 != layer) {
            format!("its factor {f} has {} prime factors", t.omega(f))
        } else if let Some(j) = board_frames.frames_in_front(i).into_iter().find(|&j| layers[j] + 1 != layer) {
            format!("frame {}, in front of it, is of layer {}", board_frames.frames()[j].name(), layers[j])
        } else {
            continue;
        };
        return Some(format!("{board}: frame {} is of layer {layer}, and {failed}", frame.name()));
    }
    None
}

/// The numbering of the Java, kept to show that the numbering by layer is the same on the boards `1..=n` (the tests of
/// this module): the frames with freedoms numbered breadth first from the front.  Number 0 goes to every frame with
/// freedoms that has no frame with freedoms directly in front of it, and number `k + 1` to every frame with freedoms
/// directly behind a frame numbered `k` that is not numbered yet.  Returns the number of each of the board's frames,
/// by position (`None` for a closed frame), and the frames in front of each frame with freedoms (`None` for a closed
/// frame).
#[cfg(test)]
fn breadth_first_numbers<const W: usize>(
    board_frames: &BoardFrames<W>,
) -> (Vec<Option<usize>>, Vec<Option<Vec<usize>>>) {
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
    let mut count = 0;
    while !current.is_empty() {
        for &i in &current {
            number[i] = Some(count);
        }
        count += 1;
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
    (number, in_front)
}

/// Where a numbering is not clean (heuristics/THEORY.md, N): the first frame `y`, by position, that is numbered `h`
/// and has a frame with freedoms `x` directly in front of it not numbered `h − 1`, as `(y, Some(x))`, or that has
/// freedoms and no number, as `(y, None)`; `None` when the numbering is clean.  `number` is the number of each frame
/// and `in_front` the frames in front of each frame with freedoms (`None` for a closed frame), as
/// [`breadth_first_numbers`] returns them; `closed` says whether a frame is closed.  It looks at the frames with
/// freedoms only and at their lists of frames in front once, and computes nothing new.
#[cfg(test)]
fn unclean(
    number: &[Option<usize>],
    in_front: &[Option<Vec<usize>>],
    closed: impl Fn(usize) -> bool,
) -> Option<(usize, Option<usize>)> {
    for (y, in_front_of_y) in in_front.iter().enumerate() {
        let Some(in_front_of_y) = in_front_of_y else { continue };
        let Some(h) = number[y] else { return Some((y, None)) };
        // A frame numbered 0 can have no frame with freedoms in front of it.
        let expected = h.checked_sub(1);
        for &x in in_front_of_y {
            if !closed(x) && expected.is_none_or(|k| number[x] != Some(k)) {
                return Some((y, Some(x)));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accelerate::solution_for_promotions;
    use crate::board::Board;
    use crate::brute::{self, Rule};
    use crate::engine::EngineConfig;
    use crate::game::replay_punctured;
    use crate::model::FrameGraph;
    use crate::search::{self, SearchTactics};

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

    /// `unclean` on made-up numberings: a frame is a position, `number` its number and `in_front` its frames in front
    /// (`None` for a closed frame), as `breadth_first_numbers` leaves them.
    #[test]
    fn unclean_finds_where_the_numbering_is_not_clean() {
        let some = |lists: &[&[usize]]| -> Vec<Option<Vec<usize>>> { lists.iter().map(|l| Some(l.to_vec())).collect() };
        let never = |_: usize| false;

        // A chain 0, 1, 2, each in front of the next, is clean.
        let chain = some(&[&[], &[0], &[1]]);
        assert_eq!(unclean(&[Some(0), Some(1), Some(2)], &chain, never), None);

        // A frame numbered 2 with a frame with freedoms numbered 0 in front of it is reported with both, though the
        // frame numbered 1 in front of it is as it should be.
        let skipping = some(&[&[], &[0], &[1, 0]]);
        assert_eq!(unclean(&[Some(0), Some(1), Some(2)], &skipping, never), Some((2, Some(0))));
        // The first frame by position is the one reported.
        let two = some(&[&[], &[0], &[0], &[1, 2]]);
        assert_eq!(unclean(&[Some(0), Some(2), Some(1), Some(2)], &two, never), Some((1, Some(0))));

        // A frame numbered 0 has no frame with freedoms in front of it.
        let in_front_of_the_front = some(&[&[], &[0]]);
        assert_eq!(unclean(&[Some(0), Some(0)], &in_front_of_the_front, never), Some((1, Some(0))));

        // A closed frame in front is ignored, whatever its number.
        let with_closed = vec![Some(vec![]), None, Some(vec![1]), Some(vec![0, 1])];
        let closed = |i: usize| i == 1;
        assert_eq!(unclean(&[Some(0), None, Some(0), Some(1)], &with_closed, closed), None);

        // A frame with freedoms and no number.
        let unreached = some(&[&[], &[0]]);
        assert_eq!(unclean(&[Some(0), None], &unreached, never), Some((1, None)));
    }

    /// The frames in front of each frame with freedoms of the board (`None` for a closed frame): the lists `unclean`
    /// takes with the numbers.
    fn in_front_of_the_open<const W: usize>(board_frames: &BoardFrames<W>) -> Vec<Option<Vec<usize>>> {
        let frames = board_frames.frames();
        (0..frames.len()).map(|i| (!frames[i].is_closed()).then(|| board_frames.frames_in_front(i))).collect()
    }

    /// The numbering by layer of the board `1..=t.n` gives every frame the number that the breadth-first numbering
    /// gives it, which is clean, and no number is without a frame.
    fn check_the_numbering_by_layer_is_the_breadth_first_one<const W: usize>(t: &Tables) {
        let board_frames = BoardFrames::<W>::new(t, Bits::EMPTY);
        let frames = board_frames.frames();
        let (breadth_first, in_front) = breadth_first_numbers(&board_frames);
        let Numbers { groups, number, .. } = numbered_frames(&board_frames);
        assert_eq!(number, breadth_first, "n={}", t.n);
        assert!(groups.iter().all(|group| !group.is_empty()), "n={}: a number without a frame", t.n);
        assert_eq!(unclean(&breadth_first, &in_front, |i| frames[i].is_closed()), None, "n={}", t.n);
    }

    /// On the boards `1..=n` the frames are numbered by layer as they are numbered breadth first from the front (the
    /// Java's numbering, [`breadth_first_numbers`]), frame by frame, and that numbering is clean.  Every board to 400,
    /// and a sample of the boards to 8191 at the width each dispatches to (`tests/wide.rs` builds the frames of
    /// those): every 199th board from 401, the boards where the count of frames steps up (512, 1024, 2048, 4096) and
    /// the largest.
    #[test]
    fn the_numbering_by_layer_is_the_breadth_first_one() {
        let sample = (401..=8191).step_by(199).chain([512, 1024, 2048, 4096, 8191]);
        for n in (2..=400).chain(sample) {
            crate::with_width!(n, W => check_the_numbering_by_layer_is_the_breadth_first_one::<W>(&Tables::new(n)));
        }
    }

    /// The frames lie in layers (heuristics/THEORY.md, Lemma Ω) on every board to 400 and on every board of up to 60
    /// numbers with one number taken off it, and on all of them the numbering by layer is clean.
    #[test]
    fn lemma_omega_holds_on_every_board() {
        let check = |t: &Tables| {
            let board_frames = BoardFrames::<8>::new(t, Bits::EMPTY);
            let frames = board_frames.frames();
            let Numbers { number, layers, .. } = numbered_frames(&board_frames);
            let board = format!("n={} without {:?}", t.n, t.holes);
            assert_eq!(lemma_omega_violation(&board_frames, &layers), None, "{board}");
            let in_front = in_front_of_the_open(&board_frames);
            assert_eq!(unclean(&number, &in_front, |i| frames[i].is_closed()), None, "{board}");
        };
        for n in 2..=400 {
            check(&Tables::new(n));
        }
        for n in 2..=60 {
            for hole in 1..=n {
                check(&Tables::punctured(n, &[hole]));
            }
        }
    }

    /// `lemma_omega_violation` names the board and the frame where a layer is not Ω of what the frame holds.
    #[test]
    fn lemma_omega_violation_names_the_frame_that_fails() {
        let t = Tables::new(30);
        let board_frames = BoardFrames::<8>::new(&t, Bits::EMPTY);
        let Numbers { mut layers, .. } = numbered_frames(&board_frames);
        assert_eq!(lemma_omega_violation(&board_frames, &layers), None);
        let last = layers.len() - 1;
        layers[last] += 1;
        let message = lemma_omega_violation(&board_frames, &layers).expect("a violation");
        assert!(
            message.starts_with("n=30: frame ") && message.contains(board_frames.frames()[last].name()),
            "{message}"
        );
    }

    /// A punctured board is numbered by layer too, which is clean where the breadth-first numbering is not: on the
    /// board of 56 without 39 the breadth-first numbering gives a frame (F) and a frame with freedoms directly in
    /// front of it (E) the number 1, and the numbering by layer gives F the number of E plus one (the sacrifice
    /// study's boards; `Tables::punctured`).
    #[test]
    fn a_punctured_board_is_numbered_by_layer() {
        let t = Tables::punctured(56, &[39]);
        let board_frames = BoardFrames::<2>::new(&t, Bits::EMPTY);
        let frames = board_frames.frames();
        let closed = |i: usize| frames[i].is_closed();

        let (breadth_first, in_front) = breadth_first_numbers(&board_frames);
        let (y, x) = unclean(&breadth_first, &in_front, closed).expect("an unclean numbering");
        let x = x.expect("a frame in front");
        assert_eq!((frames[y].name(), frames[x].name()), ("F", "E"));
        assert_eq!((breadth_first[y], breadth_first[x]), (Some(1), Some(1)));

        let Numbers { number, layers, .. } = numbered_frames(&board_frames);
        assert_eq!(number[y], number[x].map(|k| k + 1));
        assert_eq!(unclean(&number, &in_front_of_the_open(&board_frames), closed), None);
        let lowest = (0..frames.len()).filter(|&i| !closed(i)).map(|i| layers[i]).min().expect("frames with freedoms");
        for (i, &number) in number.iter().enumerate() {
            assert_eq!(number, (!closed(i)).then(|| (layers[i] - lowest) as usize), "frame {}", frames[i].name());
        }
        assert!(!build_frames::<2>(&t).frames.is_empty());
    }

    /// A punctured board can have a number with no frame: the board of 32 without 4 is the smallest, by size and then
    /// by the hole, of those with one hole (a scan of every board to 127).  Taking 4 off splits the frame of 18, 20,
    /// 27, 28 and 30 (layer 3) into two closed frames, so layer 3 has no frame with freedoms, while layer 2 (the
    /// frames of 21 and of 22, 26) and layer 4 (the frame of 24) have; layer 2 is number 0, layer 3 number 1 and
    /// layer 4 number 2.  The empty frame has no moves, no factors and no freedoms, the model and the search take
    /// it, and both bounds (the GF(2) bound and the matching bound) meet it: the search finds the best game that the
    /// maximal-factor rule allows on the board, which the brute force finds too, and the game of its promotions is a
    /// game of the punctured board.
    #[test]
    fn an_empty_frame_where_a_layer_has_no_freedoms() {
        const W: usize = 8;
        let (n, holes) = (32, [4]);
        let board = Board::<W>::new(Tables::punctured(n, &holes));
        let board_frames = BoardFrames::<W>::new(&board.tables, Bits::EMPTY);
        let Numbers { groups, number, layers } = numbered_frames(&board_frames);
        let mut with_freedoms: Vec<u32> =
            (0..layers.len()).filter(|&i| number[i].is_some()).map(|i| layers[i]).collect();
        with_freedoms.sort_unstable();
        with_freedoms.dedup();
        assert_eq!(with_freedoms, [2, 4]);
        assert_eq!(groups.iter().map(Vec::len).collect::<Vec<_>>(), [2, 0, 1]);

        let found = &board.frames;
        assert_eq!((found.frames.len(), found.names.as_slice()), (3, ["B+C", "-", "G"].map(String::from).as_slice()));
        let empty = &found.frames[1];
        assert!(empty.moves.is_empty() && empty.factors.is_empty() && empty.freedoms == 0);
        assert!(found.claims[1].is_empty());
        FrameGraph::build::<W>(&board.tables, &found.columns()).expect("the model takes an empty frame");

        let exact = brute::solve(n, &holes, Rule::MaximalFactor).score;
        let cfg = EngineConfig { threads: 1, progress: None, ..EngineConfig::default() };
        for tactics in [SearchTactics::default(), SearchTactics { gf2: false, ..SearchTactics::default() }] {
            let prepared = board.prepare(tactics).expect("a prepared search");
            let result = search::search(&prepared, &cfg, None, None, false).expect("a search");
            assert!(result.completed, "{tactics:?}");
            let score = board.in_the_bag + result.sum;
            assert_eq!(score, exact, "{tactics:?}");
            let moves = solution_for_promotions::<W>(&board.tables, result.promotions);
            assert_eq!(replay_punctured(n, &holes, &moves), Ok(score), "{tactics:?}");
        }
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
