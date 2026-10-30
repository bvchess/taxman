//! The frames as setup forms them: [`Frame`] and [`BoardFrames`], with the peel that decides what a frame can play.
//!
//! The module documentation of [`crate::frames`] says what they are, what simplifying them assumes, how they are
//! numbered and merged, and the order of the claims that `unzip` makes.

use crate::bits::Bits;
use crate::numtheory::Tables;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};

/// Which of the two loops of `unzip` settled a pair of a mandatory move and the factor it claims.  The names are those
/// of the two moves of a peel (`docs/theory/playability.md`, "Peel"): a front move pairs a move with its only factor, a
/// back move a factor with its only move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClaimKind {
    /// A factor wanted by a single remaining source went to it (the first loop of `unzip`, over the factors).
    Back,
    /// A source with a single unclaimed factor took it (the second loop of `unzip`, over the sources).
    Front,
}

/// A frame as setup forms it: a connected group of sources with the factors they compete for, plus what the
/// simplification found (before frames with the same number are merged, though a merged frame is a `Frame` again, see
/// `Frame::merged`).
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
    /// search reads them; they are for the extended problem and the tests of the frames.
    claims: Vec<(u32, u32, ClaimKind)>,
    /// Sources that cannot be played together with the others.
    impossible: Bits<W>,
    /// What `BoardFrames::can_add_source` has answered for this frame, by number.  The answer depends on nothing but
    /// the frame's `sources` and `factors`, so the memo is emptied when `sources` changes (`factors` never does).
    can_add: RefCell<HashMap<u32, bool>>,
    /// Whether `BoardFrames::remove_impossible` has been through the frame's remaining sources and factors as they are
    /// now.  Every remaining source was kept, so another pass over the same ones would keep them all again and find
    /// nothing; it is skipped.  `unzip` clears the flag when it takes moves and factors out of them.
    swept: bool,
}

impl<const W: usize> Frame<W> {
    /// A frame that has only just been formed: nothing is mandatory, claimed or impossible yet.
    fn new(sources: Bits<W>, factors: Bits<W>) -> Self {
        Frame {
            name: String::new(),
            sources,
            factors,
            mandatory: Bits::EMPTY,
            claimed: Bits::EMPTY,
            claims: Vec::new(),
            impossible: Bits::EMPTY,
            can_add: RefCell::default(),
            swept: false,
        }
    }

    /// One frame made of several: the frames that get the same number are merged into one (see
    /// [`crate::frames`]).  Its sources, factors and impossible sources are the unions of the parts'.  What the parts
    /// had found mandatory is forgotten, and found again on the next `BoardFrames::finish_setup`; their impossible
    /// sources stay impossible, so that the merged frame holds every number of its parts.
    fn merged(parts: &[&Frame<W>]) -> Self {
        let sources = parts.iter().fold(Bits::EMPTY, |acc, h| acc | h.sources);
        let factors = parts.iter().fold(Bits::EMPTY, |acc, h| acc | h.factors);
        let mut frame = Frame::new(sources, factors);
        frame.impossible = parts.iter().fold(Bits::EMPTY, |acc, h| acc | h.impossible);
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
    /// the `mandatory` ones and the factors exactly the `claimed` ones, each once.  For the extended problem and the
    /// tests of the frames; nothing in the search reads them.
    pub fn claims(&self) -> &[(u32, u32, ClaimKind)] {
        &self.claims
    }

    /// The frame's freedoms, `|factors| − |sources|`: how many factors are left over once every source has one.
    /// (Both counts include the mandatory pairs, so this is also `|remaining factors| − |remaining sources|`.)  They
    /// are the theory's freedoms once `remove_impossible` has taken the impossible sources out, which
    /// `BoardFrames::finish_setup` does to every frame before it tests any claim.
    pub fn freedoms(&self) -> i32 {
        self.factors.len() as i32 - self.sources.len() as i32
    }

    /// Whether the frame is *closed*: it has no freedoms, so it can neither spare a factor nor take another move.
    pub fn is_closed(&self) -> bool {
        self.freedoms() <= 0
    }
}

/// All the frames of the board `1..=n`, for a given set of promoted numbers, simplified and in playing order (see the
/// documentation of [`crate::frames`] for what that means).  The lists `factors_of` and `composites_of` are the view of
/// the divisibility relation that setup takes, with the promoted numbers and the sinks taken into account.
///
/// A `BoardFrames` is used on one thread: `count_freedoms` keeps its working storage in it, and its frames keep what
/// they have been asked (`Frame::can_add`), both behind a `RefCell`.
pub struct BoardFrames<'t, const W: usize> {
    t: &'t Tables,
    /// In playing order: a frame is listed after every frame in front of it.
    frames: Vec<Frame<W>>,
    /// The position in `frames` of the frame of every number, indexed by number (entry 0 is unused).
    home: Vec<u32>,
    /// For each number, the factors it can use as tax, ascending: its maximal factors that are not sources; nothing
    /// for a *sink* (a number all of whose maximal factors are promoted).  This is the table setup uses wherever it
    /// asks what a move can be paid with, not `Tables::max_factors`.
    factors_of: Lists,
    /// For each number, the moves that could use it as tax, ascending: its maximal composites other than sinks;
    /// nothing for a source.  The mirror image of `factors_of`: `c` is in the list of `f` exactly when `f` is in the
    /// list of `c`.
    composites_of: Lists,
    /// The working storage of `count_freedoms`.
    peel: RefCell<Peel>,
}

impl<'t, const W: usize> BoardFrames<'t, W> {
    // -----------------------------------------------------------------------------------------------------
    // Construction

    /// Builds the frames of the board `1..=t.n`, treating the numbers in `treat_as_sources` (the promoted
    /// numbers) as moves in addition to the numbers that are always moves.
    ///
    /// In order: find the sources and sinks, make the `factors_of` and `composites_of` tables for them, form the
    /// frames (a union-find over the source-to-factor edges), put them in playing order (front first), and simplify
    /// them (`finish_setup`: `remove_impossible` on every frame, then `unzip` and
    /// `remove_impossible` again until nothing changes).
    pub fn new(t: &'t Tables, treat_as_sources: Bits<W>) -> Self {
        let n = t.n;
        assert!(n <= Bits::<W>::MAX_VALUE, "the board 1..={n} does not fit in Bits<{W}>");
        assert!(
            !treat_as_sources.contains(0) && treat_as_sources.max().is_none_or(|m| m <= n),
            "promotions {treat_as_sources:?} are not all on the board 1..={n}"
        );

        let (sources, sinks) = Self::sources_and_sinks(t, treat_as_sources);
        let factors_of = Lists::new(n, |x, list| {
            if !sinks.contains(x) {
                let usable = |f: &u32| !sources.contains(*f);
                list.extend(t.max_factors[x as usize].iter().copied().filter(usable));
            }
        });
        let composites_of = Lists::new(n, |x, list| {
            if !sources.contains(x) {
                let usable = |c: &u32| !sinks.contains(*c);
                list.extend(t.max_composites[x as usize].iter().copied().filter(usable));
            }
        });

        let (frames, home) = Self::form_frames(t, sources, &factors_of);
        let peel = RefCell::default();
        let mut board_frames = BoardFrames { t, frames, home, factors_of, composites_of, peel };
        let order = board_frames.order_frames();
        board_frames.reorder(&order);
        board_frames.finish_setup();
        board_frames
    }

    /// The numbers that are moves (promoted ones, and those with no maximal composite: every number above `n/2`)
    /// and the sinks (numbers whose maximal factors are all promoted, which therefore have no factor to pay tax
    /// with).  1 is always a sink (it has no maximal factor at all): it can never be a move, because a move needs a
    /// divisor to pay it.
    ///
    /// Example: on the board `1..=20` with 8 promoted, 8 is a source and 16, whose only maximal factor is 8, is a
    /// sink; since 16 is also above `n/2` it is a source with nothing to pay it, which `remove_impossible` finds.
    fn sources_and_sinks(t: &Tables, promoted: Bits<W>) -> (Bits<W>, Bits<W>) {
        let (mut sources, mut sinks) = (promoted, Bits::EMPTY);
        for x in 1..=t.n {
            let xi = x as usize;
            if t.max_composites[xi].is_empty() {
                sources.insert(x);
            }
            if t.max_factors[xi].iter().all(|&f| promoted.contains(f)) {
                sinks.insert(x);
            }
        }
        (sources, sinks)
    }

    /// Joins every source to its factors and returns the connected components, with the frame of each number.  The
    /// joining is a union-find over the edges (source, factor); a component becomes a frame when its smallest member
    /// is first met, so the frames come out ordered by smallest member (which only matters for tie-breaking in
    /// [`order_frames`](Self::order_frames)).  Each number goes into its frame's sources or factors according to
    /// whether it is a source.
    ///
    /// Every frame has a source, because every number is a source or a factor of one: every `x ≤ n/2` has a prime `p`
    /// with `n/2 < x·p ≤ n`, and `x·p` is a source of which `x` is a maximal factor.
    fn form_frames(t: &Tables, sources: Bits<W>, factors_of: &Lists) -> (Vec<Frame<W>>, Vec<u32>) {
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
        let mut home = vec![0; size];
        for x in 1..=n {
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
            home[x as usize] = h;
        }
        let frames = parts.into_iter().map(|(s, f)| Frame::new(s, f)).collect();
        (frames, home)
    }

    /// The order the frames are kept in: those in front before those behind, so that playing the frames one after
    /// the other is always a legal game.  It is a breadth-first topological sort: start from the frames with
    /// nothing in front of them, smallest source first; after taking a frame, queue the frames directly behind it
    /// whose every frame in front is already taken, again smallest source first.
    ///
    /// The tie-breaking by smallest source is what makes the order, and with it the names and the final game,
    /// the Java's.
    fn order_frames(&self) -> Vec<usize> {
        let count = self.frames.len();
        let t = self.t;
        let down: Vec<Vec<usize>> = (0..count).map(|i| self.neighbors(i, &t.max_factors)).collect();
        let up: Vec<Vec<usize>> = (0..count).map(|i| self.neighbors(i, &t.max_composites)).collect();
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
        for home in self.home.iter_mut().skip(1) {
            *home = new_pos[*home as usize];
        }
    }

    // -----------------------------------------------------------------------------------------------------
    // Looking up frames and their neighbors

    /// The frames, in playing order.
    pub fn frames(&self) -> &[Frame<W>] {
        &self.frames
    }

    /// The tables of the board these frames are formed on.
    pub(crate) fn tables(&self) -> &'t Tables {
        self.t
    }

    /// The position in [`frames`](Self::frames) of the frame that holds `x`, a number of the board `1..=n`: every
    /// number is homed in one of the board's frames, and factors are never removed from a frame.
    pub(crate) fn frame_index_of(&self, x: u32) -> usize {
        self.home[x as usize] as usize
    }

    /// The frames in front of frame `i` (other than `i`): those that hold a maximal factor of a number in frame
    /// `i`, ascending.  Note that this looks at the board's maximal factors, not at the frames' factor tables.
    pub(crate) fn frames_in_front(&self, i: usize) -> Vec<usize> {
        self.neighbors(i, &self.t.max_factors)
    }

    /// The frames behind frame `i` (other than `i`): those that hold a maximal composite of a number in frame
    /// `i`, ascending.
    pub(crate) fn frames_behind(&self, i: usize) -> Vec<usize> {
        self.neighbors(i, &self.t.max_composites)
    }

    /// The frames (other than frame `i`) that hold a number related to one in frame `i`.
    fn neighbors(&self, i: usize, board_relation: &[Vec<u32>]) -> Vec<usize> {
        let frame = &self.frames[i];
        let mut out = Vec::new();
        for x in (frame.sources | frame.factors).iter() {
            for &y in &board_relation[x as usize] {
                let j = self.frame_index_of(y);
                if j != i {
                    out.push(j);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whether `pred` holds for any frame that holds one of the factors in `list` (each frame is tried once, which
    /// only saves repeated work: several factors of a number are often in the same frame).  `list` is the maximal
    /// factors of one number, at most five on the boards the solver supports (30030 = 2·3·5·7·11·13 is the first
    /// number with six), so the eight slots for the frames already tried always suffice.
    fn any_frame_of(&self, list: &[u32], mut pred: impl FnMut(&Frame<W>) -> bool) -> bool {
        let mut tried = [usize::MAX; 8];
        let mut count = 0;
        for &f in list {
            let h = self.frame_index_of(f);
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
    /// exactly when they can (the peel theorem (P4), `docs/theory/playability.md`; the order of the steps does not
    /// change the answer, but "smallest first" is the Java's and is kept).  What is assumed is that this is the right
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
    pub(crate) fn count_freedoms(&self, moves: Bits<W>, factors: Bits<W>) -> Option<u32> {
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

    /// Runs the simplification on every frame, then keeps re-running it on the neighbors of any frame that
    /// changed, until nothing changes.  First every frame drops its impossible sources (so that the checks in
    /// `unzip` see the real frames); then, in rounds, the frames that have been touched are visited in playing
    /// order.  A frame's findings change what its neighbors may conclude (`unzip` asks whether a neighbor could
    /// use a factor, and that depends on the neighbor's sources and freedoms), which is why they are revisited.
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

    /// Simplifies frame `i` once (`unzip`, then `remove_impossible`); whether it found anything new.
    fn finish_frame(&mut self, i: usize) -> bool {
        let found_mandatory = self.unzip(i);
        let found_impossible = self.remove_impossible(i);
        found_mandatory | found_impossible
    }

    /// Finds the mandatory moves of frame `i` until there are no more, recording each with the factor it claims;
    /// whether it found any.  (Earlier notes call these moves "forced", which this code avoids: the search has
    /// forced members of its own that are unrelated.)  That no game is lost by them is the claims
    /// lemma (X5) of docs/theory/exactness.md (see [`crate::frames`]).
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
    /// a claim can make a larger source eligible.  These are the orders of the claims of frame h in
    /// docs/theory/frame-model.md, so that the frames setup builds are the root problem Π_n of the theory
    /// (`tests/canonical_root.rs`, and check I of the census `examples/structure.rs` at commit 3ca11bf).
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
    fn has_outside_move_using(&self, f: u32, answers: &mut Answers) -> bool {
        if let Some(&known) = answers.outside.get(&f) {
            return known;
        }
        let f_frame = self.frame_index_of(f);
        let answer = self.composites_of.of(f).iter().any(|&c| {
            let c_frame = self.frame_index_of(c);
            let c_frame_has_freedoms = c_frame != f_frame && !self.frames[c_frame].is_closed();
            c_frame_has_freedoms && self.can_add_source(&self.frames[f_frame], c)
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
    /// over a frame keeps U\*, and the later passes keep everything (the kept-sources lemma (X1),
    /// docs/theory/exactness.md; that an optimal game plays U\* is the statement the frame model rests on, see
    /// [`crate::frames`]).  (Sources that were mandatory are not reconsidered: they are not in the remaining sources.)
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
    /// Every number is re-homed: those of the parts, impossible sources included, point at the merged frame, the
    /// others at their frame's new position.  The new frame is not simplified until the next `finish_setup`.
    pub(crate) fn merge(&mut self, group: &[usize]) -> usize {
        assert!(group.len() > 1, "merging fewer than two frames");
        let at = group[0];
        let in_group = |p: usize| group.contains(&p);
        let parts: Vec<&Frame<W>> = group.iter().map(|&p| &self.frames[p]).collect();
        let merged = Frame::merged(&parts);

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
            self.home[x as usize] = new_pos[self.home[x as usize] as usize];
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

    /// The impossible sources of every frame.
    pub fn impossible(&self) -> Bits<W> {
        self.frames.iter().fold(Bits::EMPTY, |acc, h| acc | h.impossible)
    }

    /// A legal order in which to play the sources of frame `i`: the peel of `count_freedoms`
    /// again, this time keeping the order it finds.
    ///
    /// The game is built from both ends.  In each round, first every source that has exactly one factor left
    /// (visited in ascending order against a snapshot) is played now, with that factor; then every factor
    /// wanted by exactly one source is paired with that source, which is set aside for the end of the frame's
    /// game (the sources set aside later are played earlier).  A move with a single choice goes first and one
    /// with a private factor goes last, as in the peel; that the result is a legal game is the peel theorem
    /// (P4) of `docs/theory/playability.md`.  A round that makes no progress panics: it would mean the frame's sources
    /// cannot be played together, which `remove_impossible` rules out.
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

    /// The Java solver's way of counting freedoms, verbatim: a scan per step, as the doc of
    /// `BoardFrames::count_freedoms` describes the peel.  The reference for `BoardFrames::count_freedoms`.
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
            let promoted = subset(&mut rng, plain.promotion_candidates(), 50);
            check_claims(&BoardFrames::<5>::new(&t, promoted), &format!("n={n} promoting {promoted:?}"));
        }
    }

    /// README's example: on the board 1..=21 the move 15 must take the factor 5, a back move (5 is wanted by 15 alone).
    #[test]
    fn fifteen_must_take_five_on_the_board_of_21() {
        let t = Tables::new(21);
        let a = BoardFrames::<1>::new(&t, Bits::EMPTY);
        let at = a.frame_index_of(15);
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

    /// A case from the Java's tests: with these promotions the board's frames must give a legal game.
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
        let g = a.frame_index_of(16);
        let f = a.frame_index_of(8);
        assert!(g > f);
        let (sources_f, sources_g) = (a.frames()[f].sources(), a.frames()[g].sources());
        let (freedoms_f, freedoms_g) = (a.frames()[f].freedoms(), a.frames()[g].freedoms());
        let merged = a.merge(&[g, f]);
        // The merged frame sits where the first listed frame (G) was, minus the removed F before it.
        assert_eq!(merged, g - 1);
        assert_eq!(a.frames().len(), 7);
        assert_eq!(a.frames()[merged].name(), "G+F");
        assert_eq!(a.frames()[merged].sources(), sources_f | sources_g);
        assert_eq!(a.frame_index_of(8), merged);
        assert_eq!(a.frame_index_of(16), merged);
        assert_eq!(a.frame_index_of(1), 0);
        assert_eq!(a.frame_index_of(4), 2);
        assert_eq!(a.frame_index_of(32), merged + 1); // H, which followed G

        // The merged frame has the freedoms of its parts together, before it is simplified again.
        assert_eq!(a.frames()[merged].freedoms(), freedoms_f + freedoms_g);
        assert_eq!(a.frames()[merged].freedoms(), 4);
    }

    #[test]
    fn merging_keeps_impossible_sources() {
        let t = Tables::new(114);
        let mut a = BoardFrames::<2>::new(&t, Bits::EMPTY);
        let with_impossible = (0..a.frames().len()).find(|&i| !a.frames()[i].impossible().is_empty()).unwrap();
        let other =
            (0..a.frames().len()).find(|&i| i != with_impossible && !a.frames()[i].sources().is_empty()).unwrap();
        let impossible = a.frames()[with_impossible].impossible() | a.frames()[other].impossible();
        let numbers = a.frames()[with_impossible].sources() | a.frames()[with_impossible].factors() | impossible;
        let merged = a.merge(&[other, with_impossible]);
        for x in numbers.iter() {
            assert_eq!(a.frame_index_of(x), merged, "number {x}");
        }
        assert_eq!(a.frames()[merged].impossible(), impossible);
        // The merged frame finds its claims again, its impossible sources stay impossible, and its freedoms are its
        // factors less its sources.
        a.finish_setup();
        let frame = &a.frames()[merged];
        assert!(impossible.is_subset(&frame.impossible()));
        assert_eq!(frame.freedoms(), frame.factors().len() as i32 - frame.sources().len() as i32);
        assert_eq!(a.frame_index_of(impossible.min().unwrap()), merged);
    }
}
