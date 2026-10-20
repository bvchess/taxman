//! The extended problem Π⁺(O): the frame model with some of the sources of three or more prime factors optional.
//!
//! The frame solver searches Π_n: the frames with freedoms, their sources fixed to U\* (the sources setup keeps), the
//! pairs `unzip` settled taken out.  Π⁺(O) makes a set O of the members of U\* in layers 3 and up optional, and keeps
//! only the pairs of `unzip` that Lemma M5⁺ shows are still moves of every instance (below), or none.  It is research
//! code, for certifying that a game's optimum plays U\*_{≥3} (docs/THEORY.md, "The extended problem"; `taxman
//! certify`, [`crate::certify`]); the solver does not use it.  The search takes it with its one-sided candidates
//! ([`Board::prepare_extended_with`](crate::board::Board::prepare_extended_with); the model's "One-sided candidates",
//! [`crate::model`]), and the reference predicate [`ExtendedFrames::promotable`] decides its promotion sets for the
//! tests.
//!
//! # The problem
//!
//! A *component* is a frame as [`BoardFrames::new`] forms it, before any merge; it lies in one layer ([`layer_of`],
//! docs/THEORY.md, Lemma Ω).  U\* is the sources of the components, U\*_{≥3} those of the components of layer 3
//! or more.  Given O ⊆ U\*_{≥3}:
//!
//! * the *fixed* sources are F = U\* ∖ O, and R, the *rejected* sources, are those greedy rejected
//!   (`Frame::impossible`) in the components that hold a member of O;
//! * the freedoms of a component C are |factors(C)| − |F ∩ C|; a component with none is *inert*;
//! * with ℓ₀ the lowest layer of a component with freedoms, frame h is the union of the components with freedoms
//!   of layer ℓ₀ + h: its moves are their fixed sources and its factors all their factors (a layer with no such
//!   component gives an empty frame), less the pairs of the claims kept (below).  The frames here are not reduced (the
//!   root the search builds from them is, as for Π_n);
//! * a *two-sided* candidate is a factor x of a frame h ≥ 1 such that the fixed sources of its component stay playable
//!   against the component's factors less x, and some component with freedoms that holds a maximal factor of x can take
//!   x on beside its fixed sources;
//! * a *one-sided* candidate is a member of O or R, with the frame of its component; keeping it adds it to that
//!   frame's moves and removes no factor;
//! * `in_the_bag` is the sum of F over all the components, the inert ones included.
//!
//! A *promotion set* is a pair (K, L) of one-sided and two-sided candidates such that every frame is winnable with
//! the members of K of that frame and the members of L that are factors of the frame behind it added to its moves, and
//! the members of L that are its own factors removed from its factors.  With O empty there are no one-sided
//! candidates and the promotion sets are those of Π_n.  In general, a playable S ⊇ F with no prime up to n/2 and no
//! source of layer 1 or 2 outside F is a promotion set (K, L), K being its sources outside F and L its numbers up to
//! n/2, and conversely, and then ΣS = `in_the_bag` + ΣK + ΣL: `tests/model_extended.rs` checks this against the whole
//! board.
//!
//! # The claims of `unzip` (Lemma M5⁺)
//!
//! [`build_extended`] is given the claims of the frame model, the pairs (s, f) that `unzip` settled in the search's
//! frames as the last `finish_setup` left them (`SearchFrames::claims`), or none.  Each pair belongs to the component
//! of its source s, which also holds f.  A component *keeps* its claims if it holds no member of O ∪ R and each of its
//! back claims (s, f) passes the check of Lemma M5⁺ ([`ClaimCheck`]): f is not a two-sided candidate, and no two-sided
//! candidate has f among its maximal factors.  Then every pair of the component is taken out of its frame, s from the
//! moves and f from the factors, before the root the search builds is reduced; the freedoms do not change.  A component
//! that holds a member of O ∪ R keeps none, nor does one whose back claim fails the check (its claims are recorded as
//! [`ExtendedFrames::refused`]).  The candidates do not change, as they are decided on the components with nothing
//! claimed, and no claimed factor is one (a back claim's by the check, a front claim's because its source has nothing
//! else left).  docs/THEORY.md, Lemma M5⁺: the claims kept are front and back moves of every instance of Π⁺(O),
//! so Theorem M⁺ holds with them taken out, and with O = ∅ every claim is kept and Π⁺(∅) is Π_n
//! (`examples/structure.rs` checks the problems equal on every board).  Without the check a back claim could hide a
//! use of f by a promoted number, as at n = 98 in Π_n (THEORY.md, "The frame model is exact, given its upper part", the
//! paragraph "Why the candidate filter matters"): a component of f's layer that holds an open source has more freedoms
//! and can take on composites of f that were no candidates in Π_n.

use super::{columns_of, layer_of, BoardFrames, ClaimKind};
use crate::bits::Bits;
use crate::model::{self, MAX_FRAMES};
use crate::numtheory::Tables;

/// A pair that `unzip` settled: the source, the factor it claims, and which loop claimed it.
pub type Claim = (u32, u32, ClaimKind);

/// The extended problem Π⁺(O) of a board (see the module documentation).
#[derive(Clone, Debug)]
pub struct ExtendedFrames<const W: usize> {
    /// The frames, front first and unreduced: the moves are the fixed sources of the frame's components and the
    /// factors all of theirs, less the pairs of [`claims`](Self::claims), so the freedoms of a frame are the sum of
    /// its components'.
    pub frames: Vec<model::Frame<W>>,
    /// The two-sided candidates.
    pub candidates: Bits<W>,
    /// The one-sided candidates, ascending, each with the position in `frames` of its frame.
    pub one_sided: Vec<(u32, usize)>,
    /// The sum of the fixed sources: what the game with only those scores.
    pub in_the_bag: u64,
    /// The fixed sources F.
    pub fixed: Bits<W>,
    /// The open sources O.
    pub open: Bits<W>,
    /// The rejected sources R of the components that hold a member of O.
    pub rejected: Bits<W>,
    /// The lowest layer of a component with freedoms, which frame 0 lies in; `None` (and no frames) when no
    /// component has freedoms.
    pub lowest_layer: Option<u32>,
    /// The claims kept (Lemma M5⁺), in the order of the claims given: their pairs are not in `frames`.
    pub claims: Vec<Claim>,
    /// The claims of the components that hold no member of O ∪ R but have a back claim that fails the check of Lemma
    /// M5⁺, in the order of the claims given: claims of Π_n that Π⁺(O) does not keep.  (The claims of a component
    /// that holds a member of O ∪ R are kept by neither list.)
    pub refused: Vec<Claim>,
}

/// The check of Lemma M5⁺ on a factor claimed by a back claim (docs/THEORY.md): the factor is not a two-sided
/// candidate, and no two-sided candidate has it among its maximal factors.  The one place it is written, which
/// [`build_extended`] applies and the census of the frames (`examples/structure.rs`) counts.
#[derive(Clone, Debug)]
pub struct ClaimCheck<const W: usize> {
    candidates: Bits<W>,
    /// The maximal factors of the candidates.
    listed: Bits<W>,
}

impl<const W: usize> ClaimCheck<W> {
    /// The check against the two-sided candidates `candidates` of the board `1..=t.n`.
    pub fn new(t: &Tables, candidates: &Bits<W>) -> Self {
        let listed = candidates.iter().flat_map(|x| t.max_factors[x as usize].iter().copied()).collect();
        ClaimCheck { candidates: *candidates, listed }
    }

    /// Whether a back claim of the factor `f` passes.
    pub fn passes(&self, f: u32) -> bool {
        !self.candidates.contains(f) && !self.listed.contains(f)
    }
}

impl<const W: usize> ExtendedFrames<W> {
    /// The frames in the shape [`crate::model::FrameGraph::build`] takes: for each frame, front first, its factors
    /// and then its moves.
    pub fn columns(&self) -> Vec<Vec<u32>> {
        columns_of(&self.frames)
    }

    /// Whether (`k`, `l`) is a promotion set: the reference predicate of the problem.  `board` must be
    /// `BoardFrames::new(t, Bits::EMPTY)` of the tables the problem was built from.
    ///
    /// False if a member of `k` is not a one-sided candidate, or a member of `l` is a factor of no frame or of frame
    /// 0 (it has no frame to be promoted into).  Otherwise every frame must be winnable, which a plain peel of its
    /// moves, those of `k` in it and those of `l` that are factors of the frame behind it, against its factors less
    /// those of `l` decides.  The peel needs no reduction first (the reduction lemma of `model::peel`: a move left
    /// with one live factor is paid by it, which changes no answer).  It does not check that `l` is made of
    /// candidates: a promotion set is both.
    pub fn promotable(&self, board: &BoardFrames<W>, k: &Bits<W>, l: &Bits<W>) -> bool {
        let mut moves: Vec<Bits<W>> = self.frames.iter().map(|frame| frame.moves).collect();
        let mut factors: Vec<Bits<W>> = self.frames.iter().map(|frame| frame.factors).collect();
        for s in k.iter() {
            match self.one_sided.binary_search_by_key(&s, |&(number, _)| number) {
                Ok(at) => moves[self.one_sided[at].1].insert(s),
                Err(_) => return false,
            }
        }
        for x in l.iter() {
            match self.frames.iter().position(|frame| frame.factors.contains(x)) {
                Some(home) if home >= 1 => {
                    moves[home - 1].insert(x);
                    factors[home].remove(x);
                }
                _ => return false,
            }
        }
        moves.iter().zip(&factors).all(|(&moves, &factors)| board.count_freedoms(moves, factors).is_some())
    }
}

/// The components of a board with what Π⁺(O) makes of them: each one's layer, whether it has freedoms⁺, and the
/// layers of the frames (the lowest and the highest layer of a component with freedoms⁺, if there is one).
struct Shape {
    layers: Vec<u32>,
    has_freedoms: Vec<bool>,
    span: Option<(u32, u32)>,
}

impl Shape {
    /// The shape of Π⁺(`open`) on the board whose components are `board`'s frames, after checking that every member
    /// of `open` is a source of three or more prime factors that setup keeps.
    fn of<const W: usize>(board: &BoardFrames<W>, t: &Tables, open: &Bits<W>) -> Result<Shape, String> {
        let components = board.frames();
        let layers: Vec<u32> = components.iter().map(|component| layer_of(component, t)).collect();
        let upper = components.iter().fold(Bits::EMPTY, |acc, component| acc | component.sources());
        let from_layer_3 = components.iter().zip(&layers).filter(|&(_, &layer)| layer >= 3);
        let may_open = from_layer_3.fold(Bits::EMPTY, |acc, (component, _)| acc | component.sources());
        if let Some(x) = open.iter().find(|&x| !may_open.contains(x)) {
            let why =
                if upper.contains(x) { "it has fewer than three prime factors" } else { "setup does not keep it" };
            return Err(format!("{x} cannot be open on the board 1..={}: {why}", t.n));
        }
        let has_freedoms: Vec<bool> =
            components.iter().map(|c| c.factors().len() > (c.sources() - *open).len()).collect();
        let with_freedoms = || layers.iter().zip(&has_freedoms).filter(|&(_, &has)| has).map(|(&layer, _)| layer);
        let span = with_freedoms().min().zip(with_freedoms().max());
        Ok(Shape { layers, has_freedoms, span })
    }

    /// How many frames Π⁺ has: one per layer from the lowest to the highest with freedoms⁺.
    fn frames(&self) -> usize {
        self.span.map_or(0, |(lowest, highest)| (highest - lowest) as usize + 1)
    }
}

/// How many frames the extended problem Π⁺(O) of the board `1..=t.n` with the open sources `open` has, whether or not
/// a frame set holds them ([`build_extended`] refuses more than [`MAX_FRAMES`]), and the layers they span: for the
/// census of the frames (`examples/structure.rs`), which counts them with every source of three or more prime factors
/// open, the most any `open` gives (an open source only adds to its component's freedoms⁺).
///
/// # Errors
///
/// As [`build_extended`] for `open`.
pub fn extended_frame_count<const W: usize>(t: &Tables, open: &Bits<W>) -> Result<(usize, Option<(u32, u32)>), String> {
    let shape = Shape::of(&BoardFrames::<W>::new(t, Bits::EMPTY), t, open)?;
    Ok((shape.frames(), shape.span))
}

/// The extended problem Π⁺(O) of the board `1..=t.n`, with the open sources `open`, keeping the pairs of `claims` that
/// Lemma M5⁺ allows (see the module documentation).  `claims` is the frame model's (`SearchFrames::claims` of
/// [`build_frames`](super::build_frames) on the same tables: for each of its frames, the pairs in the order made), or
/// empty for the problem with nothing claimed.
///
/// # Errors
///
/// If a member of `open` is not a source of three or more prime factors that setup keeps, or if the problem has more
/// frames than a frame set holds ([`MAX_FRAMES`]; a board past 4095 can have ten, with every source open).
///
/// # Panics
///
/// If a claim is not a source and a factor of one component with freedoms (`claims` is not the frame model's).
pub fn build_extended<const W: usize>(
    t: &Tables,
    open: &Bits<W>,
    claims: &[Vec<Claim>],
) -> Result<ExtendedFrames<W>, String> {
    let board = BoardFrames::<W>::new(t, Bits::EMPTY);
    let components = board.frames();
    let shape = Shape::of(&board, t, open)?;
    let count = shape.frames();
    let Shape { layers, has_freedoms, span } = shape;
    if count > MAX_FRAMES {
        let (lowest, highest) = span.expect("frames");
        return Err(format!(
            "the board 1..={}: Π⁺(O) with {} open sources needs {count} frames, for the layers {lowest} to {highest}, \
             and a frame set holds at most {MAX_FRAMES}",
            t.n,
            open.len()
        ));
    }
    let upper = components.iter().fold(Bits::EMPTY, |acc, component| acc | component.sources());
    let fixed = upper - *open;
    let fixed_in = |i: usize| components[i].sources() - *open;
    let mut extended = ExtendedFrames {
        frames: Vec::new(),
        candidates: Bits::EMPTY,
        one_sided: Vec::new(),
        in_the_bag: fixed.sum(),
        fixed,
        open: *open,
        rejected: Bits::EMPTY,
        lowest_layer: None,
        claims: Vec::new(),
        refused: Vec::new(),
    };
    let Some((lowest, highest)) = span else {
        return Ok(extended);
    };
    extended.lowest_layer = Some(lowest);

    // The two-sided candidates: the factors of the frames behind frame 0 that their component can spare and that a
    // component in front of them can take on.
    let can_take_on = |x: u32| {
        let mut homes: Vec<usize> = t.max_factors[x as usize]
            .iter()
            .filter_map(|&m| board.frame_index_of(m))
            .filter(|&j| has_freedoms[j])
            .collect();
        homes.sort_unstable();
        homes.dedup();
        homes.into_iter().any(|j| board.count_freedoms(fixed_in(j).with(x), components[j].factors()).is_some())
    };
    for i in (0..components.len()).filter(|&i| has_freedoms[i] && layers[i] > lowest) {
        let (moves, factors) = (fixed_in(i), components[i].factors());
        for x in factors.iter() {
            if board.count_freedoms(moves, factors.without(x)).is_some() && can_take_on(x) {
                extended.candidates.insert(x);
            }
        }
    }

    // The one-sided candidates: the open sources, and the sources greedy rejected beside them, which may be played
    // once a source above them is open.
    for (i, component) in components.iter().enumerate().filter(|(_, component)| component.sources().intersects(open)) {
        assert!(has_freedoms[i], "the frame of {} holds a source of {open:?} and has no freedoms", component.name());
        extended.rejected |= component.impossible();
        let frame = (layers[i] - lowest) as usize;
        let sources = (component.sources() & *open) | component.impossible();
        extended.one_sided.extend(sources.iter().map(|s| (s, frame)));
    }
    extended.one_sided.sort_unstable();

    // The claims, each with the component of its source; a component keeps them if it holds no member of O ∪ R and
    // its back claims pass the check of Lemma M5⁺.
    let component_of = |&(s, f, _): &Claim| {
        let i = board.frame_index_of(s).filter(|&i| components[i].sources().contains(s) && has_freedoms[i]);
        let i = i.unwrap_or_else(|| panic!("n={}: the claiming {s} is no source of a component with freedoms", t.n));
        assert!(components[i].factors().contains(f), "n={}: {s} claims {f}, which is no factor of its component", t.n);
        i
    };
    let held = *open | extended.rejected;
    let free: Vec<bool> = components.iter().map(|c| !(c.sources() | c.impossible()).intersects(&held)).collect();
    let check = ClaimCheck::new(t, &extended.candidates);
    let mut passes = vec![true; components.len()];
    for &(s, f, kind) in claims.iter().flatten() {
        passes[component_of(&(s, f, kind))] &= kind == ClaimKind::Front || check.passes(f);
    }
    let (mut claiming, mut claimed) = (Bits::EMPTY, Bits::EMPTY);
    for &claim in claims.iter().flatten() {
        let i = component_of(&claim);
        if free[i] && passes[i] {
            extended.claims.push(claim);
            claiming.insert(claim.0);
            claimed.insert(claim.1);
        } else if free[i] {
            extended.refused.push(claim);
        }
    }
    debug_assert!(!claimed.intersects(&extended.candidates), "n={}: a factor of a claim kept is a candidate", t.n);

    // The frames the components with freedoms make, without the pairs of the claims kept.
    let mut parts = vec![(Bits::EMPTY, Bits::EMPTY); (highest - lowest) as usize + 1];
    for i in (0..components.len()).filter(|&i| has_freedoms[i]) {
        let (moves, factors) = &mut parts[(layers[i] - lowest) as usize];
        *moves |= fixed_in(i) - claiming;
        *factors |= components[i].factors() - claimed;
    }
    extended.frames = parts.into_iter().map(|(moves, factors)| model::Frame::new(moves, factors)).collect();
    Ok(extended)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Board;
    use crate::frames::Frame;

    /// Π⁺(∅) of the board `1..=n` is the frame model of `build_frames` with the claims of `unzip` put back: the same
    /// number of frames, each with the moves of the search's frame and the sources that claim a factor, all its
    /// factors, and the same freedoms; the same candidates, and the sum of U\* for `in_the_bag`.  Nothing is open,
    /// rejected or one-sided.
    fn check_no_open_source_is_the_frame_model<const W: usize>(n: u32) {
        let board = Board::<W>::for_n(n);
        let (found, extended) = (&board.frames, build_extended::<W>(&board.tables, &Bits::EMPTY, &[]).unwrap());
        assert_eq!(extended.frames.len(), found.frames.len(), "n={n}: the number of frames");
        for (h, (ours, theirs)) in extended.frames.iter().zip(&found.frames).enumerate() {
            let claiming: Bits<W> = found.claims[h].iter().map(|&(source, _, _)| source).collect();
            let claimed: Bits<W> = found.claims[h].iter().map(|&(_, factor, _)| factor).collect();
            assert_eq!(ours.moves, theirs.moves | claiming, "n={n} frame {h}: moves");
            assert_eq!(ours.factors, theirs.factors | claimed, "n={n} frame {h}: factors");
            assert_eq!(ours.freedoms, theirs.freedoms, "n={n} frame {h}: freedoms");
        }
        assert_eq!(extended.candidates, found.candidates, "n={n}: candidates");
        assert_eq!(extended.in_the_bag, board.in_the_bag, "n={n}: in the bag");
        let upper: Bits<W> = board.plain.iter().copied().filter(|&x| 2 * x > n).collect();
        assert_eq!(extended.fixed, upper, "n={n}: the fixed sources");
        let nothing = (extended.open, extended.rejected, extended.one_sided.len());
        assert_eq!(nothing, (Bits::EMPTY, Bits::EMPTY, 0), "n={n}: open, rejected, one-sided");
        assert_eq!(extended.lowest_layer.is_some(), !extended.frames.is_empty(), "n={n}: the lowest layer");
        assert!(extended.claims.is_empty() && extended.refused.is_empty(), "n={n}: claims given none");

        // With the frame model's claims (Lemma M5⁺), every one is kept and the frames are the search's.
        let with_claims = build_extended::<W>(&board.tables, &Bits::EMPTY, &found.claims).unwrap();
        assert_eq!(with_claims.frames, found.frames, "n={n}: the frames with claims");
        assert_eq!(with_claims.candidates, found.candidates, "n={n}: the candidates with claims");
        assert_eq!(with_claims.in_the_bag, board.in_the_bag, "n={n}: in the bag with claims");
        let every: Vec<Claim> = found.claims.iter().flatten().copied().collect();
        assert_eq!((with_claims.claims, with_claims.refused), (every, Vec::new()), "n={n}: the claims kept");
    }

    /// The board `1..=85` of the module `frames`: the frames hold the pairs `unzip` settles there, seven of them in
    /// frame 0 (the sources 44, 52, 66, 68, 70, 76 and 78 with the factors 22, 26, 33, 34, 35, 38 and 39), which the
    /// search does not see.
    #[test]
    fn frames_of_85() {
        let extended = build_extended::<2>(&Tables::new(85), &Bits::EMPTY, &[]).unwrap();
        let shape: Vec<(Vec<u32>, Vec<u32>)> =
            extended.frames.iter().map(|f| (f.factors.to_vec(), f.moves.to_vec())).collect();
        assert_eq!(
            shape,
            vec![
                (
                    vec![4, 6, 9, 10, 14, 15, 21, 22, 25, 26, 33, 34, 35, 38, 39],
                    vec![44, 45, 50, 52, 63, 66, 68, 70, 75, 76, 78]
                ),
                (vec![8, 12, 20, 28, 30, 42], vec![56, 60, 84]),
                (vec![16, 24, 36, 40], vec![48, 72, 80]),
            ]
        );
        let freedoms: Vec<i32> = extended.frames.iter().map(|f| f.freedoms).collect();
        assert_eq!((freedoms, extended.lowest_layer), (vec![4, 3, 1], Some(3)));
        assert_eq!(extended.candidates.to_vec(), vec![8, 12, 16, 20, 24, 28, 30, 36, 40, 42]);
        assert_eq!(extended.columns().len(), 6);
        check_no_open_source_is_the_frame_model::<2>(85);
    }

    /// On the boards where claims, merges and layers of two meet, and on one with a rejected source of layer 3.
    #[test]
    fn no_open_source_is_the_frame_model_on_the_examples() {
        for n in [20, 21, 85, 98, 114, 120, 182, 186, 300, 600, 741, 1000, 1300] {
            crate::with_width!(n, W => check_no_open_source_is_the_frame_model::<W>(n));
        }
    }

    /// The board of 24 with 16, 20 and 24 open, all of U\*_{≥3} but 18: the smallest board where a back claim fails the
    /// check of Lemma M5⁺ for some O (the candidates only grow with O, so a frame's claims fail for some O free of it
    /// exactly when they fail for all of U\*_{≥3} but its sources; a scan of the boards to 400 found 24-26 and 343-349,
    /// none with every source open).  The frame model claims 6 for 18 (back: 6 is wanted by 18 alone, and 12, its
    /// other composite, is a factor of the closed frame of 16 and 24).  With 16 and 24 open that frame has freedoms, 12
    /// becomes a candidate (the frame of 20, 20 open, can take it on with 4) and lists 6: the frame of 18 keeps
    /// nothing, and the frame of layer 3 holds 18 and 6.  Taking the pair out anyway would lose the promotion set
    /// {8, 12}, in which 12 pays with 6 (`tests/model_extended.rs` catches it).
    #[test]
    fn a_back_claim_that_fails_the_check_is_refused() {
        let board = Board::<1>::for_n(24);
        let claims = &board.frames.claims;
        assert!(claims.iter().flatten().any(|&claim| claim == (18, 6, ClaimKind::Back)), "{claims:?}");
        let open = Bits::from_slice(&[16, 20, 24]);
        let extended = build_extended::<1>(&board.tables, &open, claims).unwrap();
        assert!(extended.candidates.contains(12) && board.tables.max_factors[12].contains(&6));
        assert_eq!(extended.refused, [(18, 6, ClaimKind::Back)]);
        assert!(extended.claims.iter().all(|&(s, f, _)| s != 18 && f != 6), "{:?}", extended.claims);
        let frame = &extended.frames[(board.tables.omega(18) - extended.lowest_layer.unwrap()) as usize];
        assert!(frame.moves.contains(18) && frame.factors.contains(6));
        // The check itself: 6 is listed by the candidate 12; 10, neither a candidate nor listed by one, passes.
        let check = ClaimCheck::new(&board.tables, &extended.candidates);
        assert!(!check.passes(6) && check.passes(10));
        // With 18 open too the frame holds a member of O and keeps nothing either, but no claim of it is refused.
        let all = build_extended::<1>(&board.tables, &open.with(18), claims).unwrap();
        assert!(all.refused.is_empty() && all.claims.iter().all(|&(s, _, _)| s != 18));
    }

    /// The same on every board to 400: the frames, candidates and freedoms of Π⁺(∅) are those of the frame model.
    #[test]
    fn no_open_source_is_the_frame_model_on_every_board_to_400() {
        for n in 2..=400 {
            check_no_open_source_is_the_frame_model::<8>(n);
        }
    }

    /// A source cannot be open unless setup keeps it and it has three or more prime factors: not a factor (4), not a
    /// source of two prime factors (85 = 5·17), not a source greedy rejected (374 = 2·11·17 on the board of 741, the
    /// first with a rejected source of layer 3 or more).  The refusal names the number.
    #[test]
    fn an_open_source_must_be_a_kept_source_of_three_prime_factors() {
        let t = Tables::new(741);
        let rejected = BoardFrames::<16>::new(&t, Bits::EMPTY).impossible();
        assert!(rejected.contains(374) && t.omega(374) == 3, "374 is not a rejected source of layer 3");
        for (n, x) in [(85, 4), (85, 85), (741, 374)] {
            let refused = build_extended::<16>(&Tables::new(n), &Bits::from_slice(&[x]), &[]).unwrap_err();
            assert!(refused.starts_with(&format!("{x} cannot be open on the board 1..={n}")), "{refused}");
        }
        // The kept sources of the frame of 374 can be open, and 374 is then one-sided with them.
        let board = BoardFrames::<16>::new(&t, Bits::EMPTY);
        let component = board.frames().iter().find(|c| c.impossible().contains(374)).expect("the frame of 374");
        let open = Bits::from_slice(&[component.sources().max().expect("a source")]);
        let extended = build_extended::<16>(&t, &open, &[]).unwrap();
        assert_eq!(extended.rejected, component.impossible());
        assert!(extended.one_sided.iter().any(|&(s, _)| s == 374));
    }

    /// The components of layer 3 or more of `board`, the ones whose sources can be open.
    fn upper_components<'a, const W: usize>(board: &'a BoardFrames<W>, t: &Tables) -> Vec<&'a Frame<W>> {
        board.frames().iter().filter(|c| layer_of(c, t) >= 3).collect()
    }

    /// Every source of three or more prime factors open: the open set that gives Π⁺(O) the most frames (an open source
    /// only adds to the freedoms of its component).
    fn every_source_open<const W: usize>(board: &BoardFrames<W>, t: &Tables) -> Bits<W> {
        upper_components(board, t).iter().fold(Bits::EMPTY, |acc, c| acc | c.sources())
    }

    /// With every source of layer 3 or more open, the open and rejected sources are exactly the one-sided candidates,
    /// each with the frame of its layer (the frame numbered by its layer less the lowest layer of a component with
    /// freedoms), and the fixed sources are those of layers 1 and 2.
    fn check_every_open_source_is_one_sided<const W: usize>(n: u32) {
        let t = Tables::new(n);
        let board = BoardFrames::<W>::new(&t, Bits::EMPTY);
        let open = every_source_open(&board, &t);
        let rejected = upper_components(&board, &t).iter().fold(Bits::<W>::EMPTY, |acc, c| acc | c.impossible());
        let extended = build_extended::<W>(&t, &open, &[]).unwrap();
        assert_eq!((extended.open, extended.rejected), (open, rejected), "n={n}");
        assert!(extended.fixed.iter().all(|x| t.omega(x) <= 2), "n={n}: a fixed source of layer 3 or more");
        let lowest = extended.lowest_layer.expect("a component has freedoms");
        let expected: Vec<(u32, usize)> =
            (open | rejected).iter().map(|x| (x, (t.omega(x) - lowest) as usize)).collect();
        assert_eq!(extended.one_sided, expected, "n={n}");
        assert!(extended.one_sided.iter().all(|&(_, frame)| frame < extended.frames.len()), "n={n}");
    }

    #[test]
    fn every_open_source_is_one_sided_with_the_frame_of_its_layer() {
        for n in [85, 120, 400, 741, 1000] {
            crate::with_width!(n, W => check_every_open_source_is_one_sided::<W>(n));
        }
    }

    /// The most frames Π⁺(O) has on the board `1..=n`, with every source of three or more prime factors open: how many
    /// (`extended_frame_count`, and the frames `build_extended` makes when it can), the lowest and the highest layer
    /// they span, and `build_extended`'s answer, the problem if it has no more frames than [`MAX_FRAMES`] and the
    /// refusal if it has.
    fn frames_with_every_source_open<const W: usize>(n: u32) -> (usize, Option<(u32, u32)>, Result<usize, String>) {
        let t = Tables::new(n);
        let open = every_source_open(&BoardFrames::<W>::new(&t, Bits::EMPTY), &t);
        let (count, span) = extended_frame_count::<W>(&t, &open).unwrap();
        (count, span, build_extended::<W>(&t, &open, &[]).map(|extended| extended.frames.len()))
    }

    /// `build_extended` refuses a problem of more frames than a frame set holds, and says which board and how many:
    /// the board 4096 with every source of three or more prime factors open needs ten frames, one for each layer from
    /// 3 to 12 (the layer of 4096 = 2¹²), where [`MAX_FRAMES`] is nine.  One board less, at 4095, nine are enough.
    #[test]
    fn a_problem_of_more_frames_than_a_frame_set_holds_is_refused() {
        assert_eq!(MAX_FRAMES, 9, "the numbers of this test are for nine frames");
        let (count, span, built) = crate::with_width!(4096, W => frames_with_every_source_open::<W>(4096));
        assert_eq!((count, span), (10, Some((3, 12))));
        let refused = built.unwrap_err();
        assert!(refused.starts_with("the board 1..=4096"), "{refused}");
        assert!(refused.contains("needs 10 frames"), "{refused}");
        assert!(refused.contains("the layers 3 to 12"), "{refused}");

        let (count, span, built) = crate::with_width!(4095, W => frames_with_every_source_open::<W>(4095));
        assert_eq!((count, span, built), (9, Some((3, 11)), Ok(9)));
    }

    /// What has no frame to be promoted into is refused: a factor of frame 0 (4) or a number that is no factor (45)
    /// as a member of L, a source that is not one-sided (44, fixed here) as a member of K.
    #[test]
    fn promotable_refuses_what_cannot_be_promoted() {
        let t = Tables::new(85);
        let board = BoardFrames::<2>::new(&t, Bits::EMPTY);
        let extended = build_extended::<2>(&t, &Bits::EMPTY, &[]).unwrap();
        let (none, set) = (Bits::EMPTY, |x: u32| Bits::from_slice(&[x]));
        assert!(extended.promotable(&board, &none, &none));
        assert!(extended.promotable(&board, &none, &set(8)));
        for x in [4, 45] {
            assert!(!extended.promotable(&board, &none, &set(x)), "L = {{{x}}}");
        }
        assert!(!extended.promotable(&board, &set(44), &none));

        // With 44 open it is one-sided, in frame 0, and playing it or not are both promotion sets.
        let open = build_extended::<2>(&t, &set(44), &[]).unwrap();
        assert_eq!(open.one_sided, [(44, 0)]);
        assert_eq!(open.in_the_bag, extended.in_the_bag - 44);
        assert_eq!(open.frames[0].freedoms, extended.frames[0].freedoms + 1);
        assert!(open.promotable(&board, &none, &none) && open.promotable(&board, &set(44), &none));
    }
}
