//! The frames as setup forms them: [`Frame`] and [`BoardFrames`], with the peel that decides what a frame can play.
//!
//! The module documentation of [`crate::frames`] says what they are, what simplifying them assumes, how they are
//! numbered and merged, and why every loop below keeps the Java's order.

use crate::bits::Bits;
use crate::numtheory::Tables;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};

/// Which of the two loops of `unzip` settled a pair of a mandatory move and the factor it claims.  The names are those
/// of the two moves of a peel (`heuristics/THEORY.md`, Theorem 3): a front move pairs a move with its only factor, a
/// back move a factor with its only move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClaimKind {
    /// A factor wanted by a single remaining source went to it (the first loop of `unzip`, over the factors).
    Back,
    /// A source with a single unclaimed factor took it (the second loop of `unzip`, over the sources).
    Front,
}

/// A frame as setup forms it: a connected group of sources with the factors they compete for, plus what the
/// simplification found (the Java's `bombus.Hive`; before frames with the same number are merged, though a merged
/// frame is a `Frame` again, see `Frame::merged`).
///
/// The fields relate like this: `mandatory ⊆ sources`, `claimed ⊆ factors`, `|mandatory| = |claimed|` (every
/// mandatory move is paired with the factor it uses), and `impossible` is disjoint from `sources` (an impossible
/// source is taken out of `sources`, and kept only here).
#[derive(Clone, Debug)]
pub struct Frame<const W: usize> {
    name: String,
    /// The moves of the frame, without those found impossible (mandatory ones included).
    sources: Bits<W>,
    /// Every number the sources can use as tax.
    factors: Bits<W>,
    /// Sources whose move is mandatory (settled by `unzip`).
    mandatory: Bits<W>,
    /// The factors those mandatory moves use up.
    claimed: Bits<W>,
    /// The pairs `unzip` settled in this frame, as (source, factor, kind), in the order made, across all its calls on
    /// the frame.  The sources are exactly `mandatory` and the factors exactly `claimed`, each once.  Nothing in the
    /// solver reads them; they are for the census of the frames' structure (`examples/structure.rs`).
    claims: Vec<(u32, u32, ClaimKind)>,
    /// Sources that cannot be played together with the others.
    impossible: Bits<W>,
    /// The frame's freedoms, `|factors| − |sources|` once the frame has been simplified; see `Frame::new`.
    freedoms: i32,
    /// What `BoardFrames::can_add_source` has answered for this frame, by number.  The answer depends on nothing but
    /// the frame's `sources` and `factors`, so the memo is emptied when `sources` changes (`factors` never does).
    can_add: RefCell<HashMap<u32, bool>>,
    /// Whether `BoardFrames::remove_impossible` has been through the frame's remaining sources and factors as they are
    /// now.  Every remaining source was kept, so another pass over the same ones would keep them all again and find
    /// nothing; it is skipped.  `unzip` clears the flag when it takes moves and factors out of them.
    swept: bool,
}

impl<const W: usize> Frame<W> {
    /// A frame that has only just been formed: nothing is mandatory or impossible, and every factor counts as
    /// spare.  The freedoms (`freedoms`) therefore start as `|factors|`, not `|factors| − |sources|`: they
    /// are corrected when the frame is first simplified (`BoardFrames::finish_frame`), so until then the frame counts
    /// as having freedoms, and a frame visited earlier in a round sees the frames not yet visited that way.  That is
    /// what the Java does, and the results can depend on it.
    fn new(sources: Bits<W>, factors: Bits<W>) -> Self {
        Frame {
            name: String::new(),
            sources,
            factors,
            mandatory: Bits::EMPTY,
            claimed: Bits::EMPTY,
            claims: Vec::new(),
            impossible: Bits::EMPTY,
            freedoms: factors.len() as i32,
            can_add: RefCell::default(),
            swept: false,
        }
    }

    /// One frame made of several: the frames that get the same number are merged into one (see
    /// [`crate::frames`]).  It starts from scratch: the sources and factors are the unions of the current ones, and
    /// whatever the parts had found mandatory or impossible is forgotten (and found again on the next
    /// `BoardFrames::finish_setup`).  Note what that means for an impossible source: it is no longer among the parts'
    /// `sources`, so it is not carried over, and the merged frame does not know it (see `Home::MergedAway`).
    fn merged(parts: &[&Frame<W>]) -> Self {
        let sources = parts.iter().fold(Bits::EMPTY, |acc, h| acc | h.sources);
        let factors = parts.iter().fold(Bits::EMPTY, |acc, h| acc | h.factors);
        let mut frame = Frame::new(sources, factors);
        frame.name = parts.iter().map(|h| h.name.as_str()).collect::<Vec<_>>().join("+");
        frame
    }

    /// A spreadsheet-style label (A, B, ... Z, AA, ...), by position among the board's frames in playing order;
    /// merged frames join the labels of their parts with `+`.  Only for diagnostics.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The frame's moves, mandatory ones included, impossible ones excluded.
    pub fn sources(&self) -> Bits<W> {
        self.sources
    }

    /// Every factor of the frame, including those used up by mandatory moves.
    pub fn factors(&self) -> Bits<W> {
        self.factors
    }

    /// Sources that are neither mandatory nor impossible: the moves the search still has to place.
    pub fn remaining_sources(&self) -> Bits<W> {
        self.sources - (self.mandatory | self.impossible)
    }

    /// Factors not used up by mandatory moves.
    pub fn remaining_factors(&self) -> Bits<W> {
        self.factors - self.claimed
    }

    /// Sources found impossible.
    pub fn impossible(&self) -> Bits<W> {
        self.impossible
    }

    /// The pairs `unzip` settled in the frame, as (source, factor, kind), in the order made.  The sources are exactly
    /// the `mandatory` ones and the factors exactly the `claimed` ones, each once.  For the census of the frames'
    /// structure (`examples/structure.rs`); nothing in the solver reads them.
    pub fn claims(&self) -> &[(u32, u32, ClaimKind)] {
        &self.claims
    }

    /// The frame's freedoms, `|factors| − |sources|`: how many factors are left over once every source has one.
    /// (Both counts include the mandatory pairs, so this is also `|remaining factors| − |remaining sources|`.)
    pub fn freedoms(&self) -> i32 {
        self.freedoms
    }

    /// Whether the frame is *closed*: it has no freedoms, so it can neither spare a factor nor take another move.
    pub fn is_closed(&self) -> bool {
        self.freedoms <= 0
    }
}

/// Which frame a number belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Home {
    /// The frame at this position in [`BoardFrames::frames`].
    Live(u32),
    /// The number is an impossible source of a frame that has since been merged into another one.
    ///
    /// Merging builds a new frame from the parts' `sources` and `factors`, and re-homes the numbers the new frame
    /// holds ([`BoardFrames::merge`]).  An impossible source is no longer among a part's `sources`, so it is not
    /// re-homed: it keeps pointing at its old frame, which is no longer one of the board's frames but, as an
    /// object, still remembers whether it had freedoms when it was merged.  The Java behaves exactly so (`hiveMap`
    /// is updated only for `result.allNumbers()`), and `unzip`'s test for outside moves goes on consulting the
    /// stale frame for such a number (`BoardFrames::has_outside_move_using`).  It is a quirk, not something the model
    /// needs, and it is reproduced for parity.
    ///
    /// It is inert, in principle and in practice.  In principle: it can only make `unzip` claim less, and the frame
    /// model's exactness does not depend on how much `unzip` claims (heuristics/THEORY.md, lemma M5).  In practice:
    /// only [`frames::build_frames`](crate::frames::build_frames) merges frames, and on every board `2 ≤ n ≤ 8191`
    /// the test that reads this variant is reached on a single board (n = 114, twice), where it does not make the
    /// number an outside move ([`BoardFrames::merged_away_reach`], counted by the census `examples/structure.rs`);
    /// building the frames with the variant forced to answer `false` gives identical frames, candidates and
    /// freedoms on every board to 2000.  (Measured 2026-10; to repeat it, make that arm of `has_outside_move_using`
    /// return `false` and compare the frames of all the boards with and without.)
    MergedAway { had_freedoms: bool },
    /// None: the number is a hole of a punctured board ([`Tables::punctured`]).
    OffBoard,
}

/// All the frames of the board `1..=n` (the Java's `bombus.Apiary`), for a given set of promoted numbers,
/// simplified and in playing order (see the documentation of [`crate::frames`] for what that means).  The lists
/// `factors_of` and `composites_of` are the view of the divisibility relation that setup takes, with the promoted
/// numbers and the sinks taken into account.
///
/// On a punctured board ([`Tables::punctured`]) the holes belong to no frame and to no factor or composite list,
/// so they are never moves and never tax; a number whose maximal factor is a hole simply has one factor fewer.
/// Everything else is as on the full board: in particular the sources are still the numbers above `n/2` (a number
/// whose only maximal composites are holes is a factor nobody needs, not a source, so the search decides whether
/// to promote it), and the frames are put in an order in which every number comes after the frames of its
/// divisors even when the chain of maximal factors between them passes through a hole (playing a number sweeps
/// all of its divisors, holes or not in between).
///
/// A `BoardFrames` is used on one thread: `count_freedoms` keeps its working storage in it, and its frames keep what
/// they have been asked (`Frame::can_add`), both behind a `RefCell`.
pub struct BoardFrames<'t, const W: usize> {
    t: &'t Tables,
    /// In playing order: a frame is listed after every frame in front of it.
    frames: Vec<Frame<W>>,
    /// The frame of every number, indexed by number (entry 0 is unused).
    home: Vec<Home>,
    /// For each number, the factors it can use as tax, ascending: its maximal factors that are not sources (or
    /// holes); nothing for a *sink* (a number all of whose maximal factors are promoted or holes).  This is the
    /// table setup uses wherever it asks what a move can be paid with, not `Tables::max_factors`.
    factors_of: Lists,
    /// For each number, the moves that could use it as tax, ascending: its maximal composites other than sinks
    /// (and holes); nothing for a source.  The mirror image of `factors_of`: `c` is in the list of `f` exactly
    /// when `f` is in the list of `c`.
    composites_of: Lists,
    /// The working storage of `count_freedoms`.
    peel: RefCell<Peel>,
    /// How often `has_outside_move_using` met a composite homed in a merged-away frame ([`Home::MergedAway`]), and
    /// how often, at such a composite, it answered yes (see [`merged_away_reach`](Self::merged_away_reach)).
    merged_away: Cell<(u32, u32)>,
}

impl<'t, const W: usize> BoardFrames<'t, W> {
    // -----------------------------------------------------------------------------------------------------
    // Construction

    /// Builds the frames of the board `1..=t.n`, treating the numbers in `treat_as_sources` (the promoted
    /// numbers) as moves in addition to the numbers that are always moves.
    ///
    /// In order: find the sources and sinks, make the `factors_of` and `composites_of` tables for them, form the
    /// frames (a union-find over the source-to-factor edges), put them in playing order (front first), and simplify
    /// them (`finish_setup`: `removeImpossible` on every frame, then `unzip` and
    /// `removeImpossible` again until nothing changes).
    pub fn new(t: &'t Tables, treat_as_sources: Bits<W>) -> Self {
        let n = t.n;
        assert!(n <= Bits::<W>::MAX_VALUE, "the board 1..={n} does not fit in Bits<{W}>");
        assert!(
            !treat_as_sources.contains(0)
                && treat_as_sources.max().is_none_or(|m| m <= n)
                && t.holes.iter().all(|&h| !treat_as_sources.contains(h)),
            "promotions {treat_as_sources:?} are not all on the board 1..={n} without {:?}",
            t.holes
        );

        let (sources, sinks) = Self::sources_and_sinks(t, treat_as_sources);
        let factors_of = Lists::new(n, |x, list| {
            if !t.is_hole(x) && !sinks.contains(x) {
                let usable = |f: &u32| !sources.contains(*f) && !t.is_hole(*f);
                list.extend(t.max_factors[x as usize].iter().copied().filter(usable));
            }
        });
        let composites_of = Lists::new(n, |x, list| {
            if !t.is_hole(x) && !sources.contains(x) {
                let usable = |c: &u32| !sinks.contains(*c) && !t.is_hole(*c);
                list.extend(t.max_composites[x as usize].iter().copied().filter(usable));
            }
        });

        let (frames, home) = Self::form_frames(t, sources, &factors_of);
        let peel = RefCell::default();
        let merged_away = Cell::default();
        let mut board_frames = BoardFrames { t, frames, home, factors_of, composites_of, peel, merged_away };
        let order = board_frames.order_frames();
        board_frames.reorder(&order);
        board_frames.finish_setup();
        board_frames
    }

    /// The numbers that are moves (promoted ones, and those with no maximal composite: every number above `n/2`)
    /// and the sinks (numbers whose maximal factors are all promoted or holes, which therefore have no factor
    /// to pay tax with).  Holes are neither.  1 is always a sink (it has no maximal factor at all): it can never
    /// be a move, because a move needs a divisor to pay it.
    ///
    /// Example: on the board `1..=20` with 8 promoted, 8 is a source and 16, whose only maximal factor is 8, is a
    /// sink; since 16 is also above `n/2` it is a source with nothing to pay it, which `removeImpossible` finds.
    fn sources_and_sinks(t: &Tables, promoted: Bits<W>) -> (Bits<W>, Bits<W>) {
        let (mut sources, mut sinks) = (promoted, Bits::EMPTY);
        for x in (1..=t.n).filter(|&x| !t.is_hole(x)) {
            let xi = x as usize;
            if t.max_composites[xi].is_empty() {
                sources.insert(x);
            }
            if t.max_factors[xi].iter().all(|&f| promoted.contains(f) || t.is_hole(f)) {
                sinks.insert(x);
            }
        }
        (sources, sinks)
    }

    /// Joins every source to its factors and returns the connected components, with the frame of each number;
    /// holes are in none.  The joining is a union-find over the edges (source, factor); a component becomes a
    /// frame when its smallest member is first met, so the frames come out ordered by smallest member (which only
    /// matters for tie-breaking in [`order_frames`](Self::order_frames)).  Each number goes into its frame's sources
    /// or factors according to whether it is a source.
    ///
    /// A number that is neither a source nor a factor of one forms a frame of its own, without sources.  On the
    /// board `1..=n` that never happens, because every `x ≤ n/2` has a prime `p` with `n/2 < x·p ≤ n`, and `x·p`
    /// is a source of which `x` is a maximal factor.  On a punctured board it can, when every such `x·p` is a hole:
    /// then `x` is a factor nobody needs, a frame of that one number with a freedom, which the search may promote.
    fn form_frames(t: &Tables, sources: Bits<W>, factors_of: &Lists) -> (Vec<Frame<W>>, Vec<Home>) {
        let n = t.n;
        let size = n as usize + 1;
        let mut parent: Vec<u32> = (0..=n).collect();
        for s in sources.iter() {
            for &f in factors_of.of(s) {
                let (a, b) = (find(&mut parent, s), find(&mut parent, f));
                parent[a as usize] = b;
            }
        }

        let mut frame_of_root = vec![u32::MAX; size];
        let mut parts: Vec<(Bits<W>, Bits<W>)> = Vec::new();
        let mut home = vec![Home::OffBoard; size];
        for x in (1..=n).filter(|&x| !t.is_hole(x)) {
            let root = find(&mut parent, x) as usize;
            if frame_of_root[root] == u32::MAX {
                frame_of_root[root] = parts.len() as u32;
                parts.push((Bits::EMPTY, Bits::EMPTY));
            }
            let h = frame_of_root[root];
            let (frame_sources, frame_factors) = &mut parts[h as usize];
            if sources.contains(x) {
                frame_sources.insert(x)
            } else {
                frame_factors.insert(x)
            }
            home[x as usize] = Home::Live(h);
        }
        let frames = parts.into_iter().map(|(s, f)| Frame::new(s, f)).collect();
        (frames, home)
    }

    /// The order the frames are kept in: those in front before those behind, so that playing the frames one after
    /// the other is always a legal game.  It is a breadth-first topological sort: start from the frames with
    /// nothing in front of them, smallest source first; after taking a frame, queue the frames directly behind it
    /// whose every frame in front is already taken, again smallest source first.  (Frames without a source sort
    /// first; among those the order is that of formation.)  "In front" here passes through holes
    /// ([`Self::neighbours_through_holes`]).
    ///
    /// The tie-breaking by smallest source is what makes the order, and with it the names and the final game,
    /// the Java's.
    fn order_frames(&self) -> Vec<usize> {
        let count = self.frames.len();
        let t = self.t;
        let down: Vec<Vec<usize>> = (0..count).map(|i| self.neighbours_through_holes(i, &t.max_factors)).collect();
        let up: Vec<Vec<usize>> = (0..count).map(|i| self.neighbours_through_holes(i, &t.max_composites)).collect();
        let min_source = |i: usize| self.frames[i].sources.min();

        let mut roots: Vec<usize> = (0..count).filter(|&i| down[i].is_empty()).collect();
        roots.sort_by_key(|&i| min_source(i));
        let mut queue: VecDeque<usize> = roots.into();
        let mut done = vec![false; count];
        let mut order = Vec::with_capacity(count);
        while let Some(h) = queue.pop_front() {
            if done[h] {
                continue;
            }
            done[h] = true;
            order.push(h);
            let mut ready: Vec<usize> =
                up[h].iter().copied().filter(|&u| !done[u] && down[u].iter().all(|&d| done[d])).collect();
            ready.sort_by_key(|&i| min_source(i));
            queue.extend(ready);
        }
        assert_eq!(order.len(), count, "the frames are not a DAG");
        order
    }

    /// Puts the frames into the given order (a permutation of positions) and gives them their names.
    fn reorder(&mut self, order: &[usize]) {
        let mut new_pos = vec![0u32; order.len()];
        for (pos, &old) in order.iter().enumerate() {
            new_pos[old] = pos as u32;
        }
        let mut old: Vec<Option<Frame<W>>> = std::mem::take(&mut self.frames).into_iter().map(Some).collect();
        self.frames = order.iter().map(|&i| old[i].take().expect("a permutation")).collect();
        for (pos, frame) in self.frames.iter_mut().enumerate() {
            frame.name = column_name(pos);
        }
        for home in &mut self.home {
            if let Home::Live(p) = home {
                *p = new_pos[*p as usize];
            }
        }
    }

    // -----------------------------------------------------------------------------------------------------
    // Looking up frames and their neighbours

    /// The frames, in playing order.
    pub fn frames(&self) -> &[Frame<W>] {
        &self.frames
    }

    /// The tables of the board these frames are formed on.
    pub(crate) fn tables(&self) -> &'t Tables {
        self.t
    }

    /// The position in [`frames`](Self::frames) of the frame that holds `x`, unless `x` is an impossible source
    /// of a frame that has since been merged away (`Home::MergedAway`), or a hole.
    pub(crate) fn frame_index_of(&self, x: u32) -> Option<usize> {
        match self.home.get(x as usize)? {
            Home::Live(p) => Some(*p as usize),
            Home::MergedAway { .. } | Home::OffBoard => None,
        }
    }

    /// The frames in front of frame `i` (other than `i`): those that hold a maximal factor of a number in frame
    /// `i`, ascending.  Note that this looks at the board's maximal factors, not at the frames' factor tables.
    pub(crate) fn frames_in_front(&self, i: usize) -> Vec<usize> {
        self.neighbours(i, &self.t.max_factors)
    }

    /// The frames behind frame `i` (other than `i`): those that hold a maximal composite of a number in frame
    /// `i`, ascending.
    pub(crate) fn frames_behind(&self, i: usize) -> Vec<usize> {
        self.neighbours(i, &self.t.max_composites)
    }

    /// The frames (other than frame `i`) that hold a number related to one in frame `i`.  Holes are skipped.
    fn neighbours(&self, i: usize, board_relation: &[Vec<u32>]) -> Vec<usize> {
        let frame = &self.frames[i];
        let mut out = Vec::new();
        for x in (frame.sources | frame.factors).iter() {
            for &y in &board_relation[x as usize] {
                // A number homed in a merged-away frame is skipped: that frame is no longer one of the board's
                // frames, so there is nothing to reach.  (The Java's neighbour sets do contain the stale frame,
                // but every use of them filters down to the frames that are still there.)
                if let Home::Live(j) = self.home[y as usize] {
                    if j as usize != i {
                        out.push(j as usize);
                    }
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// [`neighbours`](Self::neighbours), except that a related hole is passed through: the frames holding the
    /// numbers related to the hole are reached instead.  This keeps the relation transitive across holes, as
    /// divisibility is (a number sweeps its divisors whether or not the chain of maximal factors between them
    /// is on the board).  Without holes it is `neighbours`.
    fn neighbours_through_holes(&self, i: usize, board_relation: &[Vec<u32>]) -> Vec<usize> {
        if self.t.holes.is_empty() {
            return self.neighbours(i, board_relation);
        }
        let frame = &self.frames[i];
        let mut out = Vec::new();
        let mut pending: Vec<u32> = (frame.sources | frame.factors).iter().collect();
        let mut passed = Vec::new();
        while let Some(x) = pending.pop() {
            for &y in &board_relation[x as usize] {
                match self.home[y as usize] {
                    Home::Live(j) if j as usize != i => out.push(j as usize),
                    Home::OffBoard if !passed.contains(&y) => {
                        passed.push(y);
                        pending.push(y);
                    }
                    _ => {}
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The position of the frame holding the factor `f`.  Factors are never removed from a frame, so they are
    /// always homed in one of the board's frames.
    fn frame_of_factor(&self, f: u32) -> usize {
        self.frame_index_of(f).expect("a factor always has a live frame")
    }

    /// Whether `pred` holds for any frame that holds one of the factors in `list` (each frame is tried once, which
    /// only saves repeated work: several factors of a number are often in the same frame).  `list` is the maximal
    /// factors of one number, at most five on the boards the solver supports (30030 = 2·3·5·7·11·13 is the first
    /// number with six), so the eight slots for the frames already tried always suffice.
    fn any_frame_of(&self, list: &[u32], mut pred: impl FnMut(&Frame<W>) -> bool) -> bool {
        let mut tried = [usize::MAX; 8];
        let mut count = 0;
        for &f in list {
            let h = self.frame_of_factor(f);
            if tried[..count].contains(&h) {
                continue;
            }
            if count < tried.len() {
                tried[count] = h;
                count += 1;
            }
            if pred(&self.frames[h]) {
                return true;
            }
        }
        false
    }

    // -----------------------------------------------------------------------------------------------------
    // The test for "can these moves be played together": the peel

    /// If every move in `moves` can be played together, each with a factor of its own from `factors`, how many
    /// factors are left over (the *freedoms*); `None` if they cannot all be played.  The question every decision
    /// in this file comes down to: whether a frame can keep a source, take on a promoted number, or spare a factor.
    ///
    /// It is a *peel*, the decision procedure that [`crate::model`] also uses to test a frame for winnability
    /// (there with the second step below alone; the first is the Java's addition and gives the same answers).
    /// A factor is *usable* by a move when it is in the move's list in `factors_of` and still among the `factors`;
    /// a move *wants* a factor when the factor is usable by it (so the moves a factor is wanted by are the ones in
    /// its `composites_of` list that are still among the `moves`).  While moves remain, take one of two steps, the
    /// smallest candidate first:
    ///
    /// 1. A move with exactly one usable factor: pair them and remove both.  (The move has no choice, so it can
    ///    be played first, with that factor, which is gone for the others.)
    /// 2. Failing that, a factor wanted by exactly one move: pair them and remove both.  (No other move touches
    ///    the factor, so the move can be played last.)
    ///
    /// A move left with no usable factor, or no step available while moves remain (every move has two or more
    /// usable factors and every factor two or more moves: whichever move is played last would need a factor that
    /// no other move has), means they cannot all be played, and the result is `None`.  Each step removes one move
    /// and one factor, so on success the answer is simply `|factors| − |moves|`.
    ///
    /// The peel is not a heuristic for the question it asks, whether these moves can be paid from these
    /// factors in some order: each step preserves playability, and a stall means unplayable, so it succeeds
    /// exactly when they can (`heuristics/THEORY.md`, Theorems 1 to 3; the order of the steps does not change
    /// the answer, but "smallest first" is the Java's and is kept).  What is assumed is that this is the right
    /// question to put to a frame on its own (see [`crate::frames`]).
    ///
    /// It is implemented with degree counters on a graph of the moves, the factors they can use, and the edges
    /// between them (`Peel`), so a call costs one pass over the edges instead of a scan per step, and what it costs
    /// depends on how many moves and factors it is given, not on the width of a set or the size of the board.  The
    /// edges are found from the moves' side alone (`Peel::load` reads `factors_of` only, and gives each factor its
    /// moves by turning the edges round: the list of a small factor in `composites_of` has a multiple for every
    /// prime, far more than the moves that want it here).  `Peel::run` keeps the candidates for the two kinds of
    /// step in sets of its own, so that the smallest members are the ones the plain scan would reach first.  The
    /// tests compare the result with the plain scan, the Java's recursion, on random inputs; it reads both lists, so
    /// they also show that the two are mirror images, which turning the edges round relies on.
    fn count_freedoms(&self, moves: Bits<W>, factors: Bits<W>) -> Option<u32> {
        let (move_count, factor_count) = (moves.len(), factors.len());
        if move_count > factor_count {
            return None;
        }
        let mut peel = self.peel.borrow_mut();
        let playable = peel.load(&self.factors_of, &moves, &factors) && peel.run(move_count as usize);
        playable.then_some(factor_count - move_count)
    }

    /// Whether the frame `frame` could take the number `n` as an additional source: the peel with all the frame's
    /// sources and `n` against all its factors (the mandatory pairs included).
    fn can_add_source(&self, frame: &Frame<W>, n: u32) -> bool {
        *frame
            .can_add
            .borrow_mut()
            .entry(n)
            .or_insert_with(|| self.count_freedoms(frame.sources.with(n), frame.factors).is_some())
    }

    // -----------------------------------------------------------------------------------------------------
    // Simplification of the frames

    /// Runs the simplification on every frame, then keeps re-running it on the neighbours of any frame that
    /// changed, until nothing changes.  First every frame drops its impossible sources (so that the checks in
    /// `unzip` see the real frames); then, in rounds, the frames that have been touched are visited in playing
    /// order.  A frame's findings change what its neighbours may conclude (`unzip` asks whether a neighbour could
    /// use a factor, and that depends on the neighbour's sources and freedoms), which is why they are revisited.
    ///
    /// Called by [`BoardFrames::new`], and again by the frame builder after it has merged frames
    /// ([`crate::frames`]): a merged frame starts with nothing found, and has to be simplified again.
    pub(crate) fn finish_setup(&mut self) {
        for i in 0..self.frames.len() {
            self.remove_impossible(i);
        }
        let mut todo = vec![true; self.frames.len()];
        loop {
            let round: Vec<usize> = (0..self.frames.len()).filter(|&i| todo[i]).collect();
            if round.is_empty() {
                break;
            }
            todo.fill(false);
            for i in round {
                if self.finish_frame(i) {
                    for j in self.frames_in_front(i).into_iter().chain(self.frames_behind(i)) {
                        todo[j] = true;
                    }
                }
            }
        }
    }

    /// Simplifies frame `i` once (`unzip`, then `removeImpossible`, then the freedoms are recomputed as
    /// `|factors| − |sources|`); whether it found anything new.
    fn finish_frame(&mut self, i: usize) -> bool {
        let found_mandatory = self.unzip(i);
        let found_impossible = self.remove_impossible(i);
        let frame = &mut self.frames[i];
        frame.freedoms = frame.factors.len() as i32 - frame.sources.len() as i32;
        found_mandatory | found_impossible
    }

    /// Finds the mandatory moves of frame `i` until there are no more, recording each with the factor it claims;
    /// whether it found any.  (Earlier notes call these moves "forced", which this code avoids: the search has
    /// forced members of its own that are unrelated.  The Java's field is `mandatoryMoves`.)  That no game is lost
    /// by them is heuristics/THEORY.md's lemma M5 (see [`crate::frames`]).
    ///
    /// Two kinds of move are mandatory, as long as the factor involved is of no use outside the frame, so that
    /// settling the pair now takes nothing from the search:
    ///
    /// * a remaining factor wanted by a single remaining source of the frame (a *private* factor of that source),
    ///   which cannot go forward (be promoted into a source of a frame in front of this one) and which no other
    ///   frame could use as tax, goes to that source;
    /// * a remaining source with a single unclaimed factor takes it, provided no other frame could use that
    ///   factor.  This is "take the highest prime" for the whole frame: a frame is expected to play all of its
    ///   sources, so a source with one choice makes it.  (It does not ask whether the factor could go forward,
    ///   since the source needs it either way.)
    ///
    /// Why "of no use outside the frame": the factor is a candidate for something the search may want to do,
    /// either promoting it, or using it as tax for a number promoted from another frame
    /// ([`has_outside_move_using`](Self::has_outside_move_using)); the pair would take that away.
    ///
    /// The factors are visited in ascending order against a snapshot, with the sources recomputed after each
    /// claim.  The sources are visited in descending order and the pass restarts after the first claim, since
    /// a claim can make a larger source eligible.  Both orders are the Java's.
    ///
    /// The questions about a factor's use (`factor_can_go_forward` and `has_outside_move_using`) are about
    /// other frames and about this frame's sources and factors as they were when `unzip` began, none of which a
    /// claim changes (the claims are written to the frame at the end), so each is asked once, however many passes
    /// visit the factor (`Answers`).
    fn unzip(&mut self, i: usize) -> bool {
        let (sources, factors, impossible) =
            (self.frames[i].sources, self.frames[i].factors, self.frames[i].impossible);
        let (mut mandatory, mut claimed) = (self.frames[i].mandatory, self.frames[i].claimed);
        let mut claims = Vec::new();
        let mut answers = Answers::default();
        let mut found = false;
        loop {
            let mut made_claim = false;

            let snapshot = factors - claimed;
            for f in snapshot.iter() {
                // The sources still to be settled that want `f`: its composites among the remaining sources.
                let wanting = self.composites_of.of(f).iter().copied();
                let wanting =
                    wanting.filter(|&c| sources.contains(c) && !mandatory.contains(c) && !impossible.contains(c));
                if let Some(s) = the_only(wanting) {
                    if !self.factor_can_go_forward(f, &mut answers) && !self.has_outside_move_using(f, &mut answers) {
                        mandatory.insert(s);
                        claimed.insert(f);
                        claims.push((s, f, ClaimKind::Back));
                        made_claim = true;
                    }
                }
            }

            // A source's candidates are its own factors in `factors_of` (all of them in this frame), minus the
            // factors already claimed.
            let snapshot = sources - (mandatory | impossible);
            for s in snapshot.iter_desc() {
                let open = self.factors_of.of(s).iter().copied().filter(|&f| !claimed.contains(f));
                if let Some(f) = the_only(open) {
                    if !self.has_outside_move_using(f, &mut answers) {
                        mandatory.insert(s);
                        claimed.insert(f);
                        claims.push((s, f, ClaimKind::Front));
                        made_claim = true;
                        break;
                    }
                }
            }

            found |= made_claim;
            if !made_claim {
                break;
            }
        }
        let frame = &mut self.frames[i];
        frame.mandatory = mandatory;
        frame.claimed = claimed;
        frame.claims.extend(claims);
        frame.swept &= !found;
        found
    }

    /// Whether factor `f` could be promoted into a source of some frame that holds one of its factors (a frame in
    /// front of its own): the case where `f` has a use of its own, which `unzip` must leave to the search.
    fn factor_can_go_forward(&self, f: u32, answers: &mut Answers) -> bool {
        *answers
            .downstream
            .entry(f)
            .or_insert_with(|| self.any_frame_of(self.factors_of.of(f), |frame| self.can_add_source(frame, f)))
    }

    /// Whether some move outside the frame of the factor `f` could use `f` as tax: a composite of `f` in a
    /// different frame (that has freedoms, so that it might be promoted) which the frame of `f` could take on as a
    /// source.  `unzip` leaves such a factor to the search, which may need it for that promotion.
    ///
    /// A composite that is an impossible source of a frame that has since been merged away counts as being in
    /// a different frame (see [`Home::MergedAway`]), with the freedoms that frame had when it was merged.  That
    /// is what the Java solver does, and it is inert (the variant's documentation gives the evidence).
    fn has_outside_move_using(&self, f: u32, answers: &mut Answers) -> bool {
        if let Some(&known) = answers.outside.get(&f) {
            return known;
        }
        let f_frame = self.frame_of_factor(f);
        let answer = self.composites_of.of(f).iter().any(|&c| {
            let home = self.home[c as usize];
            let c_frame_has_freedoms = match home {
                Home::Live(p) => p as usize != f_frame && !self.frames[p as usize].is_closed(),
                Home::MergedAway { had_freedoms } => had_freedoms,
                Home::OffBoard => unreachable!("a hole is nobody's composite"),
            };
            let uses_f = c_frame_has_freedoms && self.can_add_source(&self.frames[f_frame], c);
            if matches!(home, Home::MergedAway { .. }) {
                let (reached, answered_yes) = self.merged_away.get();
                self.merged_away.set((reached + 1, answered_yes + u32::from(uses_f)));
            }
            uses_f
        });
        answers.outside.insert(f, answer);
        answer
    }

    /// Marks as impossible the remaining sources of frame `i` that cannot be played together with the others,
    /// going from the largest source down and keeping each one that can be played together with the ones
    /// already kept (`count_freedoms` against the remaining factors).  Impossible sources
    /// leave the frame's `sources`.  Whether any new impossible source was found.
    ///
    /// This greedy choice, largest first, decides which sources the whole model gives up on, for good: the first pass
    /// over a frame keeps U\*, and the later passes keep everything (heuristics/THEORY.md, lemma M1; that an optimal
    /// game plays U\* is the statement the frame model rests on, see [`crate::frames`]).  (Sources that were
    /// mandatory are not reconsidered: they are not in the remaining sources.)
    ///
    /// It looks at nothing but the frame's own remaining sources and factors, and keeps every source that
    /// remains, so a second pass over the same ones would keep them all again.  The frame's `swept` flag records
    /// that it has been through them, and the pass is skipped.
    fn remove_impossible(&mut self, i: usize) -> bool {
        if self.frames[i].swept {
            return false;
        }
        let frame = &self.frames[i];
        let factors = frame.remaining_factors();
        let mut impossible = frame.impossible;
        let known = impossible.len();
        let mut kept = Bits::EMPTY;
        for s in frame.remaining_sources().iter_desc() {
            if self.count_freedoms(kept.with(s), factors).is_some() {
                kept.insert(s);
            } else {
                impossible.insert(s);
            }
        }
        let frame = &mut self.frames[i];
        frame.impossible = impossible;
        let before = frame.sources;
        frame.sources -= impossible;
        if frame.sources != before {
            frame.can_add.get_mut().clear();
        }
        frame.swept = true;
        impossible.len() > known
    }

    // -----------------------------------------------------------------------------------------------------
    // Merging

    /// Replaces the frames at the given positions (at least two) by a single new frame, which is placed where
    /// the first of them was, and returns its position.  See `Frame::merged` for what the new frame holds.  The
    /// frame builder merges the frames that get the same number into one this way.
    ///
    /// Every number is re-homed: those the merged frame holds point at it, the others at their frame's new
    /// position.  The impossible sources of the parts are held by no frame any more, and keep a stale home
    /// ([`Home::MergedAway`]); the new frame is not simplified until the next `finish_setup`.
    pub(crate) fn merge(&mut self, group: &[usize]) -> usize {
        assert!(group.len() > 1, "merging fewer than two frames");
        let at = group[0];
        let in_group = |p: usize| group.contains(&p);
        let parts: Vec<&Frame<W>> = group.iter().map(|&p| &self.frames[p]).collect();
        let merged = Frame::merged(&parts);
        let merged_numbers = merged.sources | merged.factors;
        let had_freedoms: Vec<bool> = self.frames.iter().map(|f| !f.is_closed()).collect();

        let merged_pos = (0..at).filter(|&p| !in_group(p)).count();
        let mut new_pos = vec![0u32; self.frames.len()];
        let mut frames = Vec::with_capacity(self.frames.len() + 1 - group.len());
        let mut merged = Some(merged);
        for (p, frame) in std::mem::take(&mut self.frames).into_iter().enumerate() {
            if p == at {
                frames.push(merged.take().expect("pushed once"));
            }
            if in_group(p) {
                new_pos[p] = merged_pos as u32;
            } else {
                new_pos[p] = frames.len() as u32;
                frames.push(frame);
            }
        }
        self.frames = frames;

        for x in 1..=self.t.n {
            self.home[x as usize] = match self.home[x as usize] {
                Home::Live(p) if in_group(p as usize) && merged_numbers.contains(x) => Home::Live(merged_pos as u32),
                Home::Live(p) if in_group(p as usize) => Home::MergedAway { had_freedoms: had_freedoms[p as usize] },
                Home::Live(p) => Home::Live(new_pos[p as usize]),
                stale @ (Home::MergedAway { .. } | Home::OffBoard) => stale,
            };
        }
        merged_pos
    }

    // -----------------------------------------------------------------------------------------------------
    // Results

    /// The factors of frame `i` that could be promoted: those that could be dropped from the frame without
    /// stopping its remaining sources from being played together, and that some frame holding one of their
    /// maximal factors could take on as a source.  These are two necessary conditions, each checked against one
    /// frame on its own; whether a promotion really works, with the two frames it touches both staying winnable,
    /// is for the search to decide.
    pub fn promotion_candidates_of(&self, i: usize) -> Bits<W> {
        let frame = &self.frames[i];
        let (sources, factors) = (frame.remaining_sources(), frame.remaining_factors());
        factors
            .iter()
            .filter(|&f| {
                self.count_freedoms(sources, factors.without(f)).is_some()
                    && self.any_frame_of(self.factors_of.of(f), |h| self.can_add_source(h, f))
            })
            .collect()
    }

    /// The promotion candidates of every frame.
    pub fn promotion_candidates(&self) -> Bits<W> {
        (0..self.frames.len()).fold(Bits::EMPTY, |acc, i| acc | self.promotion_candidates_of(i))
    }

    /// The impossible sources of every frame.  (After a `merge` the impossible sources of the
    /// merged frames are no longer listed by any frame.)
    pub fn impossible(&self) -> Bits<W> {
        self.frames.iter().fold(Bits::EMPTY, |acc, h| acc | h.impossible)
    }

    /// How far the quirk of [`Home::MergedAway`] reaches, as counted so far by this `BoardFrames`: how many times
    /// `has_outside_move_using` met a composite homed in a merged-away frame, and how many times, at such a
    /// composite, it found that the frame of the factor could take the composite on and the stale frame had freedoms
    /// (so that the composite counted as an outside move).  Measured by the census of the frames' structure
    /// (`examples/structure.rs`).  The quirk can only make `unzip` claim less: a composite homed in a merged-away
    /// frame is an impossible source, which no frame can take on.
    pub fn merged_away_reach(&self) -> (u32, u32) {
        self.merged_away.get()
    }

    /// A legal order in which to play the sources of frame `i`: the peel of `count_freedoms`
    /// again, this time keeping the order it finds.
    ///
    /// The game is built from both ends.  In each round, first every source that has exactly one factor left
    /// (visited in ascending order against a snapshot) is played now, with that factor; then every factor
    /// wanted by exactly one source is paired with that source, which is set aside for the end of the frame's
    /// game (the sources set aside later are played earlier).  A move with a single choice goes first and one
    /// with a private factor goes last, as in the peel; that the result is a legal game is Theorem 2 of
    /// `heuristics/THEORY.md`.  A round that makes no progress panics: it would mean the frame's sources cannot be
    /// played together, which `removeImpossible` rules out.
    pub fn solution_of(&self, i: usize) -> Vec<u32> {
        let frame = &self.frames[i];
        let (mut sources, mut factors) = (frame.sources, frame.factors);
        let mut now = Vec::with_capacity(sources.len() as usize);
        let mut at_end = Vec::new();
        while !sources.is_empty() {
            let mut progress = false;
            let snapshot = sources;
            for s in snapshot.iter() {
                if let Some(f) = the_only(self.factors_of.of(s).iter().copied().filter(|&f| factors.contains(f))) {
                    now.push(s);
                    sources.remove(s);
                    factors.remove(f);
                    progress = true;
                }
            }
            let snapshot = factors;
            for f in snapshot.iter() {
                if let Some(m) = the_only(self.composites_of.of(f).iter().copied().filter(|&c| sources.contains(c))) {
                    at_end.push(m);
                    sources.remove(m);
                    factors.remove(f);
                    progress = true;
                }
            }
            assert!(progress, "frame {} cannot be played out: {sources:?} / {factors:?}", frame.name);
        }
        now.extend(at_end.into_iter().rev());
        now
    }

    /// The whole game with no promotions beyond those these frames were built with: the frames' games, one after
    /// the other in playing order (front first).  Built on the plain board it scores `in_the_bag`, the
    /// base of every score in the frame model; built with promotions it is the game those promotions make
    /// ([`crate::accelerate::solution_for_promotions`]).
    pub fn solution(&self) -> Vec<u32> {
        let mut moves = Vec::with_capacity(self.t.n as usize);
        for i in 0..self.frames.len() {
            moves.extend(self.solution_of(i));
        }
        moves
    }
}

/// A list of numbers for each number of the board `0..=n`, all in one allocation (the tables `BoardFrames::factors_of`
/// and `composites_of`): the list of `x` is `items[starts[x]..starts[x + 1]]`.
struct Lists {
    starts: Vec<u32>,
    items: Vec<u32>,
}

impl Lists {
    /// The lists of the numbers `0..=n`: `fill(x, list)` appends the list of `x` to `list`.
    fn new(n: u32, mut fill: impl FnMut(u32, &mut Vec<u32>)) -> Self {
        let mut starts = Vec::with_capacity(n as usize + 2);
        let mut items = Vec::new();
        for x in 0..=n {
            starts.push(items.len() as u32);
            fill(x, &mut items);
        }
        starts.push(items.len() as u32);
        Lists { starts, items }
    }

    /// The list of `x`.
    fn of(&self, x: u32) -> &[u32] {
        let x = x as usize;
        &self.items[self.starts[x] as usize..self.starts[x + 1] as usize]
    }
}

/// The only item, if `items` has exactly one.
fn the_only<T>(mut items: impl Iterator<Item = T>) -> Option<T> {
    let first = items.next()?;
    items.next().is_none().then_some(first)
}

/// A set of the positions `0..size` in a [`Peel`]'s graph, in as many words as they need.
#[derive(Default)]
struct Positions {
    words: Vec<u64>,
    /// No word before this one has a member, so that finding the smallest member does not look at them again.
    first: usize,
}

impl Positions {
    /// Empties the set, making room for the positions `0..size`.
    fn reset(&mut self, size: usize) {
        self.words.clear();
        self.words.resize(size.div_ceil(64), 0);
        self.first = 0;
    }

    fn insert(&mut self, i: usize) {
        self.words[i >> 6] |= 1 << (i & 63);
        self.first = self.first.min(i >> 6);
    }

    fn remove(&mut self, i: usize) {
        self.words[i >> 6] &= !(1 << (i & 63));
    }

    /// The smallest member.
    fn min(&mut self) -> Option<usize> {
        while let Some(&word) = self.words.get(self.first) {
            if word != 0 {
                return Some(self.first * 64 + word.trailing_zeros() as usize);
            }
            self.first += 1;
        }
        None
    }
}

/// The answers one `unzip` has been given so far (see there): what its questions look at does not change while it
/// runs, so none is asked twice.
#[derive(Default)]
struct Answers {
    /// `factor_can_go_forward`, by factor.
    downstream: HashMap<u32, bool>,
    /// `has_outside_move_using`, by factor.
    outside: HashMap<u32, bool>,
}

/// The degree of a move or a factor that has been paired (removed from the peel).
const PAIRED: u32 = u32::MAX;

/// The graph that `BoardFrames::count_freedoms` peels, and the counters it peels it with: the moves and factors it is
/// given, with an edge from a move to every factor it can use.  The moves are numbered by position (move `i` is
/// the `i`th smallest of them), and so are the factors, so the arrays are as long as the sets are large, not as the
/// board is, and the smallest candidate of a step is the one with the smallest position.
///
/// `count_freedoms` is called very often, so the `Peel` lives in the [`BoardFrames`] and keeps the storage of its
/// vectors from one call to the next.
#[derive(Default)]
struct Peel {
    /// The edges of move `i`, as positions of factors, ascending, are
    /// `move_factors[move_starts[i]..move_starts[i + 1]]`.
    move_starts: Vec<u32>,
    move_factors: Vec<u32>,
    /// The same edges from the factors: those of factor `r`, as positions of moves, ascending, are
    /// `factor_moves[factor_starts[r]..factor_starts[r + 1]]`.
    factor_starts: Vec<u32>,
    factor_moves: Vec<u32>,
    /// How many unpaired factors each move can use, and how many unpaired moves can use each factor; [`PAIRED`]
    /// for those that have been paired.
    move_degree: Vec<u32>,
    factor_degree: Vec<u32>,
    /// The unpaired moves with one unpaired factor, and the unpaired factors wanted by one unpaired move: the
    /// candidates for the two kinds of step.
    lone_moves: Positions,
    lone_factors: Positions,
}

impl Peel {
    /// Builds the graph of `moves` and `factors`, with the edges the lists `factors_of` give, and the degrees of
    /// everything in it.  False if some move has no factor to use, which is as good as a peel that failed.
    fn load<const W: usize>(&mut self, factors_of: &Lists, moves: &Bits<W>, factors: &Bits<W>) -> bool {
        // The position of a factor among the factors: those in the words before its own, and those below it in its
        // own word.
        let words = factors.as_words();
        let mut before = [0u32; W];
        let mut seen = 0;
        for (count, word) in before.iter_mut().zip(words) {
            *count = seen;
            seen += word.count_ones();
        }
        let position = |f: u32| {
            let (w, bit) = ((f >> 6) as usize, f & 63);
            (before[w] + (words[w] & ((1 << bit) - 1)).count_ones()) as usize
        };
        let (move_count, factor_count) = (moves.len() as usize, seen as usize);

        // The edges of each move, and the degrees.
        self.move_starts.clear();
        self.move_factors.clear();
        self.move_degree.clear();
        self.factor_degree.clear();
        self.factor_degree.resize(factor_count, 0);
        self.lone_moves.reset(move_count);
        self.lone_factors.reset(factor_count);
        for (i, m) in moves.iter().enumerate() {
            let start = self.move_factors.len();
            self.move_starts.push(start as u32);
            for &f in factors_of.of(m).iter().filter(|&&f| factors.contains(f)) {
                let r = position(f);
                self.move_factors.push(r as u32);
                self.factor_degree[r] += 1;
            }
            let degree = self.move_factors.len() - start;
            if degree == 0 {
                return false; // can never be paired, so the peeling could never finish
            }
            self.move_degree.push(degree as u32);
            if degree == 1 {
                self.lone_moves.insert(i);
            }
        }
        let edge_count = self.move_factors.len();
        self.move_starts.push(edge_count as u32);

        // The edges of each factor: a counting sort of the edges by factor.  `factor_starts[r]` first holds the end
        // of factor `r`'s block (the factors' degrees, summed); placing the edges from the last move to the first,
        // each at the end of its block, moves it to the block's start, which is where the block of factor `r + 1`
        // begins.
        self.factor_starts.clear();
        let mut end = 0;
        for (r, &degree) in self.factor_degree.iter().enumerate() {
            end += degree;
            self.factor_starts.push(end);
            if degree == 1 {
                self.lone_factors.insert(r);
            }
        }
        self.factor_starts.push(end);
        self.factor_moves.clear();
        self.factor_moves.resize(edge_count, 0);
        for i in (0..move_count).rev() {
            for &r in &self.move_factors[self.move_starts[i] as usize..self.move_starts[i + 1] as usize] {
                self.factor_starts[r as usize] -= 1;
                self.factor_moves[self.factor_starts[r as usize] as usize] = i as u32;
            }
        }
        true
    }

    /// Peels the graph that `load` built, which has `moves` moves: whether every move gets a factor of its own.
    /// The steps and their order are those described at `BoardFrames::count_freedoms`.
    fn run(&mut self, moves: usize) -> bool {
        let Peel {
            move_starts,
            move_factors,
            factor_starts,
            factor_moves,
            move_degree,
            factor_degree,
            lone_moves,
            lone_factors,
        } = self;
        let factors_of = |i: usize| &move_factors[move_starts[i] as usize..move_starts[i + 1] as usize];
        let moves_of = |r: usize| &factor_moves[factor_starts[r] as usize..factor_starts[r + 1] as usize];
        for _ in 0..moves {
            let (m, f) = if let Some(m) = lone_moves.min() {
                let Some(&f) = factors_of(m).iter().find(|&&r| factor_degree[r as usize] != PAIRED) else {
                    return false;
                };
                (m, f as usize)
            } else {
                // None: every move has two factors, and every factor two moves.
                let Some(f) = lone_factors.min() else { return false };
                let Some(&m) = moves_of(f).iter().find(|&&i| move_degree[i as usize] != PAIRED) else { return false };
                (m as usize, f)
            };
            move_degree[m] = PAIRED;
            factor_degree[f] = PAIRED;
            lone_moves.remove(m);
            lone_factors.remove(f);
            // The other factors `m` could have used are now wanted by one move fewer ...
            for &g in factors_of(m) {
                let g = g as usize;
                if factor_degree[g] != PAIRED {
                    factor_degree[g] -= 1;
                    match factor_degree[g] {
                        0 => lone_factors.remove(g),
                        1 => lone_factors.insert(g),
                        _ => {}
                    }
                }
            }
            // ... and the other moves that could have used `f` have one factor fewer.
            for &k in moves_of(f) {
                let k = k as usize;
                if move_degree[k] != PAIRED {
                    move_degree[k] -= 1;
                    match move_degree[k] {
                        0 => return false,
                        1 => lone_moves.insert(k),
                        _ => {}
                    }
                }
            }
        }
        true
    }
}

/// Root of `x`'s set in a union-find forest (`parent[x] == x` for a root), halving the path on the way.
fn find(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        let grandparent = parent[parent[x as usize] as usize];
        parent[x as usize] = grandparent;
        x = grandparent;
    }
    x
}

/// Spreadsheet-style label for position `i`: A, B, ... Z, AA, AB, ...
fn column_name(i: usize) -> String {
    let letter = char::from(b'A' + (i % 26) as u8);
    if i >= 26 {
        format!("{}{letter}", column_name(i / 26 - 1))
    } else {
        letter.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::replay;
    use crate::test_support::Rng;

    /// The Java `countFreedoms`, verbatim: a scan per step, as the doc of `BoardFrames::count_freedoms` describes the
    /// peel.  The reference for `BoardFrames::count_freedoms`.
    fn naive_count_freedoms<const W: usize>(a: &BoardFrames<W>, moves: Bits<W>, factors: Bits<W>) -> Option<u32> {
        if moves.is_empty() {
            return Some(factors.len());
        }
        if moves.len() > factors.len() {
            return None;
        }
        for m in moves.iter() {
            let usable = Bits::from_slice(a.factors_of.of(m)) & factors;
            match usable.len() {
                0 => return None,
                1 => return naive_count_freedoms(a, moves.without(m), factors - usable),
                _ => {}
            }
        }
        for f in factors.iter() {
            let wanting = Bits::from_slice(a.composites_of.of(f)) & moves;
            if wanting.len() == 1 {
                return naive_count_freedoms(a, moves - wanting, factors.without(f));
            }
        }
        None
    }

    /// A random percentage in `lo..=hi`.
    fn percent(rng: &mut Rng, lo: u64, hi: u64) -> u64 {
        lo + rng.below(hi - lo + 1)
    }

    /// A random subset of `set` in which each member is kept with probability `percent`%.
    fn subset<const W: usize>(rng: &mut Rng, set: Bits<W>, percent: u64) -> Bits<W> {
        set.iter().filter(|_| rng.chance(percent)).collect()
    }

    /// Frames of ascending size, plus a union of several frames' sources against all factors, as the places
    /// where the matching is asked about.
    fn check_count_freedoms<const W: usize>(a: &BoardFrames<W>, rng: &mut Rng) {
        let all_factors = a.frames.iter().fold(Bits::<W>::EMPTY, |acc, h| acc | h.factors);
        let all_sources = a.frames.iter().fold(Bits::<W>::EMPTY, |acc, h| acc | h.sources | h.impossible);
        for frame in &a.frames {
            for _ in 0..40 {
                let keep = percent(rng, 20, 100);
                let moves = subset(rng, frame.sources | frame.impossible, keep);
                let factors = match rng.below(3) {
                    0 => frame.factors,
                    1 => {
                        let keep = percent(rng, 70, 100);
                        subset(rng, frame.factors, keep)
                    }
                    _ => subset(rng, all_factors, 30),
                };
                assert_eq!(
                    a.count_freedoms(moves, factors),
                    naive_count_freedoms(a, moves, factors),
                    "moves {moves:?} factors {factors:?}"
                );
            }
        }
        for _ in 0..40 {
            let keep = percent(rng, 5, 25);
            let moves = subset(rng, all_sources, keep);
            let keep = percent(rng, 50, 100);
            let factors = subset(rng, all_factors, keep);
            assert_eq!(a.count_freedoms(moves, factors), naive_count_freedoms(a, moves, factors));
        }
    }

    #[test]
    fn count_freedoms_matches_the_plain_scan() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for n in [10u32, 30, 85, 120, 200, 301] {
            let t = Tables::new(n);
            let plain = BoardFrames::<5>::new(&t, Bits::EMPTY);
            check_count_freedoms(&plain, &mut rng);
            // With promotions the sources, sinks and tables change shape.
            for _ in 0..4 {
                let promotable = plain.promotion_candidates();
                let promoted = subset(&mut rng, promotable, 50);
                check_count_freedoms(&BoardFrames::<5>::new(&t, promoted), &mut rng);
            }
        }
    }

    /// Boards with and without promotions, for the tests of what the frames remember.
    fn some_board_frames<'t>(t: &'t Tables, rng: &mut Rng) -> Vec<BoardFrames<'t, 9>> {
        let plain = BoardFrames::<9>::new(t, Bits::EMPTY);
        let promotable = plain.promotion_candidates();
        let mut board_frames = vec![plain];
        for _ in 0..3 {
            board_frames.push(BoardFrames::<9>::new(t, subset(rng, promotable, 50)));
        }
        board_frames
    }

    /// `can_add_source` is remembered by each frame, from the moment it is first asked until its sources change; no
    /// entry may be left over from before the last change.
    #[test]
    fn what_a_frame_remembers_of_taking_on_a_source_is_not_stale() {
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        for n in [85, 200, 301, 500] {
            let t = Tables::new(n);
            for a in some_board_frames(&t, &mut rng) {
                for frame in a.frames() {
                    for x in 1..=n {
                        let fresh = a.count_freedoms(frame.sources.with(x), frame.factors).is_some();
                        assert_eq!(a.can_add_source(frame, x), fresh, "n={n} frame {} number {x}", frame.name);
                    }
                }
            }
        }
    }

    /// A frame forgets what it remembers when its sources change.  Put into each frame, by hand, a source that has
    /// no factor there and so cannot be played: while it is there nothing can be added, and the frame remembers
    /// that; once `remove_impossible` has taken it out again, the frame must be able to take on what it could
    /// before.
    #[test]
    fn a_frame_forgets_what_it_remembers_when_its_sources_change() {
        let n = 200;
        let t = Tables::new(n);
        let mut a = BoardFrames::<4>::new(&t, Bits::EMPTY);
        let mut checked = 0;
        for i in 0..a.frames.len() {
            let frame = &a.frames[i];
            let Some(x) = (1..=n).find(|&x| a.can_add_source(frame, x)) else { continue };
            let in_the_frame =
                |b: u32| frame.sources.contains(b) || frame.factors.contains(b) || frame.impossible.contains(b);
            let stranger = (1..=n)
                .find(|&b| !in_the_frame(b) && a.factors_of.of(b).iter().all(|&f| !frame.factors.contains(f)))
                .expect("a number with no factor in the frame");
            let frame = &mut a.frames[i];
            frame.sources.insert(stranger);
            frame.can_add.get_mut().clear(); // the sources were changed by hand
            frame.swept = false;
            assert!(!a.can_add_source(&a.frames[i], x), "frame {i}: with {stranger}, number {x}");
            assert!(a.remove_impossible(i), "frame {i}");
            assert!(a.can_add_source(&a.frames[i], x), "frame {i}: without {stranger}, number {x}");
            checked += 1;
        }
        assert!(checked >= 3, "{checked} frames checked");
    }

    /// `remove_impossible` is not run again on remaining sources it has been through (`Frame::swept`) because it
    /// would keep every one of them again: run on the finished frames it changes nothing.
    #[test]
    fn going_through_a_finished_frame_again_removes_nothing() {
        let mut rng = Rng(0x1B87_3593_E6A8_C0DD);
        for n in [85, 200, 301, 500] {
            let t = Tables::new(n);
            for mut a in some_board_frames(&t, &mut rng) {
                for i in 0..a.frames.len() {
                    assert!(a.frames[i].swept, "n={n} frame {i}");
                    let (sources, impossible) = (a.frames[i].sources, a.frames[i].impossible);
                    a.frames[i].swept = false;
                    assert!(!a.remove_impossible(i), "n={n} frame {i}");
                    assert_eq!((a.frames[i].sources, a.frames[i].impossible), (sources, impossible), "n={n} frame {i}");
                }
            }
        }
    }

    /// The claims of every frame: distinct sources, which are the frame's mandatory moves, and distinct factors, which
    /// are the factors those moves use, each one a factor the source can use.
    fn check_claims<const W: usize>(a: &BoardFrames<W>, what: &str) {
        for frame in a.frames() {
            let claims = frame.claims();
            let sources: Bits<W> = claims.iter().map(|&(s, _, _)| s).collect();
            let factors: Bits<W> = claims.iter().map(|&(_, f, _)| f).collect();
            let name = &frame.name;
            assert_eq!(sources.len() as usize, claims.len(), "{what}, frame {name}: a source claims twice");
            assert_eq!(factors.len() as usize, claims.len(), "{what}, frame {name}: a factor is claimed twice");
            assert_eq!(sources, frame.mandatory, "{what}, frame {name}: the sources of the claims");
            assert_eq!(factors, frame.claimed, "{what}, frame {name}: the factors of the claims");
            for &(s, f, _) in claims {
                assert!(a.factors_of.of(s).contains(&f), "{what}, frame {name}: {s} cannot use {f}");
            }
        }
    }

    #[test]
    fn the_claims_of_unzip_are_the_mandatory_moves_and_the_factors_they_use() {
        let mut rng = Rng(0x5851_F42D_4C95_7F2D);
        for n in 2..=300u32 {
            let t = Tables::new(n);
            let plain = BoardFrames::<5>::new(&t, Bits::EMPTY);
            check_claims(&plain, &format!("n={n}"));
            // Nothing has been merged, so no number has a stale home.
            assert_eq!(plain.merged_away_reach(), (0, 0), "n={n}");
            let promoted = subset(&mut rng, plain.promotion_candidates(), 50);
            check_claims(&BoardFrames::<5>::new(&t, promoted), &format!("n={n} promoting {promoted:?}"));
        }
    }

    /// README's example: on the board 1..=21 the move 15 must take the factor 5, a back move (5 is wanted by 15 alone).
    #[test]
    fn fifteen_must_take_five_on_the_board_of_21() {
        let t = Tables::new(21);
        let a = BoardFrames::<1>::new(&t, Bits::EMPTY);
        let at = a.frame_index_of(15).expect("15 is on the board");
        assert!(a.frames()[at].claims().contains(&(15, 5, ClaimKind::Back)), "{:?}", a.frames()[at].claims());
    }

    #[test]
    fn positions_find_their_smallest_member_and_lists_their_numbers() {
        let mut p = Positions::default();
        p.reset(200);
        assert_eq!(p.min(), None);
        p.insert(130);
        assert_eq!(p.min(), Some(130));
        p.insert(5); // below the word that `min` has moved on to
        assert_eq!(p.min(), Some(5));
        p.remove(5);
        p.insert(64);
        assert_eq!(p.min(), Some(64));
        p.remove(64);
        p.remove(130);
        assert_eq!(p.min(), None);
        p.insert(199);
        p.reset(10);
        assert_eq!(p.min(), None);

        assert_eq!((the_only(0..0), the_only(7..8), the_only(7..9)), (None, Some(7), None));
        let lists = Lists::new(4, |x, list| list.extend(0..x));
        assert_eq!((lists.of(0), lists.of(1), lists.of(4)), (&[][..], &[0][..], &[0, 1, 2, 3][..]));
    }

    #[test]
    fn count_freedoms_small_cases() {
        let t = Tables::new(20);
        let a = BoardFrames::<1>::new(&t, Bits::EMPTY);
        let set = |v: &[u32]| Bits::<1>::from_slice(v);
        // No moves: every factor is left over.
        assert_eq!(a.count_freedoms(set(&[]), set(&[2, 3])), Some(2));
        // More moves than factors.
        assert_eq!(a.count_freedoms(set(&[12, 14]), set(&[6])), None);
        // 12 = 2^2*3 has maximal factors 4 and 6; 14 has 2 and 7; both can be paid for.
        assert_eq!(a.count_freedoms(set(&[12, 14]), set(&[4, 6, 7])), Some(1));
        // 14 has no factor in {4, 6}.
        assert_eq!(a.count_freedoms(set(&[12, 14]), set(&[4, 6])), None);
    }

    #[test]
    fn frames_of_ten() {
        let t = Tables::new(10);
        let a = BoardFrames::<1>::new(&t, Bits::EMPTY);
        let shape: Vec<(&str, Vec<u32>, Vec<u32>)> =
            a.frames().iter().map(|h| (h.name(), h.sources().to_vec(), h.factors().to_vec())).collect();
        assert_eq!(shape, vec![("A", vec![7], vec![1]), ("B", vec![6, 9, 10], vec![2, 3, 5]), ("C", vec![8], vec![4])]);
        assert_eq!(a.solution(), vec![7, 9, 6, 10, 8]);
        assert!(a.impossible().is_empty() && a.promotion_candidates().is_empty());
        assert!(a.frames().iter().all(|h| h.is_closed() && h.remaining_sources().is_empty()));
    }

    #[test]
    fn frames_of_one_and_two() {
        let t1 = Tables::new(1);
        let a = BoardFrames::<1>::new(&t1, Bits::EMPTY);
        assert_eq!(a.frames().len(), 1);
        assert!(a.frames()[0].sources().is_empty());
        assert_eq!(a.impossible().to_vec(), vec![1]);
        assert!(a.solution().is_empty());
        let t2 = Tables::new(2);
        let a = BoardFrames::<1>::new(&t2, Bits::EMPTY);
        assert_eq!(a.solution(), vec![2]);
    }

    /// The example in the documentation of `sources_and_sinks`: promoting 8 on the board `1..=20` makes 16, whose
    /// only maximal factor is 8, a sink, and a sink that is a source is impossible.
    #[test]
    fn promoting_8_leaves_16_with_nothing_to_pay_it() {
        let t = Tables::new(20);
        let (sources, sinks) = BoardFrames::<1>::sources_and_sinks(&t, Bits::from_slice(&[8]));
        assert!(sources.contains(8) && sources.contains(16));
        assert!(sinks.contains(16) && sinks.contains(1) && !sinks.contains(8));
        assert!(!BoardFrames::<1>::new(&t, Bits::EMPTY).impossible().contains(16));
        let promoted = BoardFrames::<1>::new(&t, Bits::from_slice(&[8]));
        assert!(promoted.impossible().contains(16));
        let moves = promoted.solution();
        assert!(moves.contains(&8) && !moves.contains(&16));
        assert!(replay(20, &moves).is_ok());
    }

    #[test]
    fn column_names() {
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(25), "Z");
        assert_eq!(column_name(26), "AA");
        assert_eq!(column_name(27), "AB");
        assert_eq!(column_name(51), "AZ");
        assert_eq!(column_name(52), "BA");
        assert_eq!(column_name(26 * 27 - 1), "ZZ");
        assert_eq!(column_name(26 * 27), "AAA");
    }

    /// The Java's `ApiaryTest.theMysteryOf528`: with these promotions the board's frames must give a legal game.
    #[test]
    fn the_mystery_of_528() {
        let t = Tables::new(528);
        let promoted = Bits::<9>::from_slice(&[
            128, 154, 168, 171, 174, 175, 180, 189, 195, 196, 198, 207, 208, 228, 230, 231, 234, 238, 240, 243, 244,
            245, 248, 250, 252, 255, 258, 260, 261, 264,
        ]);
        let moves = BoardFrames::new(&t, promoted).solution();
        assert!(replay(528, &moves).is_ok());
    }

    #[test]
    fn solutions_are_legal_games() {
        for n in [3u32, 17, 60, 85, 100, 211, 300] {
            let t = Tables::new(n);
            let a = BoardFrames::<5>::new(&t, Bits::EMPTY);
            let moves = a.solution();
            assert!(replay(n, &moves).is_ok(), "n={n}");
            // Whatever the frames promise as candidates can be promoted without breaking the game.
            for c in a.promotion_candidates().iter() {
                let promoted = Bits::from_slice(&[c]);
                let moves = BoardFrames::<5>::new(&t, promoted).solution();
                assert!(replay(n, &moves).is_ok(), "n={n} promoting {c}");
            }
        }
    }

    #[test]
    fn merging_places_and_remaps() {
        let t = Tables::new(85);
        let mut a = BoardFrames::<2>::new(&t, Bits::EMPTY);
        let names = |a: &BoardFrames<2>| a.frames().iter().map(|h| h.name().to_owned()).collect::<Vec<_>>();
        assert_eq!(names(&a), ["A", "B", "C", "D", "E", "F", "G", "H"]);
        // F and G hold the numbers 8..=42 and 16..=40; merge them, listing G first.
        let g = a.frame_index_of(16).unwrap();
        let f = a.frame_index_of(8).unwrap();
        assert!(g > f);
        let (sources_f, sources_g) = (a.frames()[f].sources(), a.frames()[g].sources());
        let merged = a.merge(&[g, f]);
        // The merged frame sits where the first listed frame (G) was, minus the removed F before it.
        assert_eq!(merged, g - 1);
        assert_eq!(a.frames().len(), 7);
        assert_eq!(a.frames()[merged].name(), "G+F");
        assert_eq!(a.frames()[merged].sources(), sources_f | sources_g);
        assert_eq!(a.frame_index_of(8), Some(merged));
        assert_eq!(a.frame_index_of(16), Some(merged));
        assert_eq!(a.frame_index_of(1), Some(0));
        assert_eq!(a.frame_index_of(4), Some(2));
        assert_eq!(a.frame_index_of(32), Some(merged + 1)); // H, which followed G
        assert_eq!(a.frames()[merged].freedoms(), 10);
    }

    #[test]
    fn merging_leaves_impossible_sources_behind() {
        let t = Tables::new(114);
        let mut a = BoardFrames::<2>::new(&t, Bits::EMPTY);
        let with_impossible = (0..a.frames().len()).find(|&i| !a.frames()[i].impossible().is_empty()).unwrap();
        let other =
            (0..a.frames().len()).find(|&i| i != with_impossible && !a.frames()[i].sources().is_empty()).unwrap();
        let left_behind = a.frames()[with_impossible].impossible();
        let kept = a.frames()[with_impossible].sources() | a.frames()[with_impossible].factors();
        let merged = a.merge(&[other, with_impossible]);
        for x in left_behind.iter() {
            assert_eq!(a.frame_index_of(x), None, "impossible source {x}");
        }
        for x in kept.iter() {
            assert_eq!(a.frame_index_of(x), Some(merged), "number {x}");
        }
        assert!(a.frames()[merged].impossible().is_empty());
        // The merged frame re-derives what it needs, and the board's frames stay consistent.
        a.finish_setup();
        let frame = &a.frames()[merged];
        assert_eq!(frame.freedoms(), frame.factors().len() as i32 - frame.sources().len() as i32);
        assert!(a.frame_index_of(left_behind.min().unwrap()).is_none());
    }
}
