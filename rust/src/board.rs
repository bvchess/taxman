//! A board `1..=n` analysed for the search: its number-theory tables, its frames, and the numbers the per-game
//! pipeline derives from them before it decides how to solve the game.
//!
//! This is the one path from a board size to a search that is ready to run (tables → frames → [`prepare`] → ceiling
//! and seed → [`search`]).  The per-game pipeline ([`crate::solver`]), the ladder and the benchmarks, and the tests
//! all take it, so that the facts that join the pieces are written once:
//!
//! * the score of a game of the frame model is `in_the_bag` plus the sum of its promotions, where `in_the_bag`
//!   (the Java's `inTheBag`) is what the game with no promotions scores ([`Board::in_the_bag`]);
//! * no game of `n` scores more than `n + opt(n − 1)` (deleting the move `n` from an optimal game leaves a game of
//!   `n − 1`), so no promotion set sums to more than `n + opt(n − 1) − in_the_bag`: the search's *ceiling*, at which
//!   it can stop ([`Board::promotion_cap`]);
//! * the promotions of the previous game that are candidates of this board are a good, usually near-optimal, first
//!   incumbent: the search's *seed* ([`Board::seed`]).
//!
//! A board is generic over the set width `W` (see [`crate::bits`]): pick it with `with_width!`, as the solver and
//! the ladder do.
//!
//! [`prepare`]: crate::search::prepare
//! [`search`]: crate::search::search

use crate::accelerate::solution_for_promotions;
use crate::bits::Bits;
use crate::frames::{build_extended, build_frames, ExtendedFrames, SearchFrames};
use crate::numtheory::Tables;
use crate::search::{self, Prepared, Problem, SearchTactics};
use std::path::Path;
use std::time::{Duration, Instant};

/// The board `1..=n`, analysed.
pub struct Board<const W: usize> {
    /// The maximal factors and composites of the numbers on the board.
    pub tables: Tables,
    /// The board's frames as the search takes them (numbered from the front, those with one number merged), with
    /// the promotion candidates.
    pub frames: SearchFrames<W>,
    /// The game with no promotions, in play order.
    pub plain: Vec<u32>,
    /// What [`plain`](Self::plain) scores (the Java's `inTheBag`): the part of every score that no promotion
    /// changes.
    pub in_the_bag: u64,
    /// How long forming, numbering and merging the frames took.
    pub frames_time: Duration,
}

impl<const W: usize> Board<W> {
    /// The board of `tables`: its frames are formed, numbered from the front and merged, and the game with no
    /// promotions is played.
    pub fn new(tables: Tables) -> Self {
        let start = Instant::now();
        let frames = build_frames::<W>(&tables);
        let frames_time = start.elapsed();
        let plain = solution_for_promotions::<W>(&tables, Bits::EMPTY);
        let in_the_bag = plain.iter().map(|&m| u64::from(m)).sum();
        Board { tables, frames, plain, in_the_bag, frames_time }
    }

    /// [`new`](Self::new) for the board `1..=n`.
    pub fn for_n(n: u32) -> Self {
        Board::new(Tables::new(n))
    }

    /// The board size.
    pub fn n(&self) -> u32 {
        self.tables.n
    }

    /// The numbers that could be promoted.
    pub fn candidates(&self) -> Bits<W> {
        self.frames.candidates
    }

    /// Whether nothing can be promoted, so the game with no promotions is the only game ([`plain`](Self::plain)).
    pub fn is_trivial(&self) -> bool {
        self.frames.candidates.is_empty()
    }

    /// Sets the search up: the frame set and the greedy incumbent, see [`search::prepare`].
    ///
    /// # Errors
    ///
    /// If the frames cannot be built into a model (more frames than a frame set holds).
    pub fn prepare(&self, tactics: SearchTactics) -> Result<Prepared<W>, String> {
        search::prepare(&self.tables, &self.frames.columns(), &self.frames.candidates, tactics)
    }

    /// Sets the search up for the extended problem Π⁺(∅) ([`build_extended`]): the frame model without the claims of
    /// `unzip`, and the problem it poses.  [`prepare_extended_with`](Self::prepare_extended_with) with no open source.
    ///
    /// # Errors
    ///
    /// As [`prepare_extended_with`](Self::prepare_extended_with).
    pub fn prepare_extended(&self, tactics: SearchTactics) -> Result<(ExtendedFrames<W>, Prepared<W>), String> {
        self.prepare_extended_with(&Bits::EMPTY, tactics)
    }

    /// Sets the search up for the extended problem Π⁺(O) with the open sources `open` ([`build_extended`]), its
    /// one-sided candidates (the open sources and those greedy rejected beside them) candidates of the search beside
    /// the two-sided ones ([`search::prepare_extended`]).  Research code: the certificates of `taxman certify`; the
    /// default path is [`prepare`](Self::prepare).
    ///
    /// # Errors
    ///
    /// If `open` holds a number that cannot be open, if the problem has more frames than a frame set holds, or if it
    /// has one-sided candidates and `tactics` are not those of extended mode
    /// ([`SearchTactics::refusal_with_one_sided`]).
    pub fn prepare_extended_with(
        &self,
        open: &Bits<W>,
        tactics: SearchTactics,
    ) -> Result<(ExtendedFrames<W>, Prepared<W>), String> {
        let extended = build_extended(&self.tables, open)?;
        let (columns, one_sided) = (extended.columns(), &extended.one_sided);
        let prepared = search::prepare_extended(&self.tables, &columns, one_sided, &extended.candidates, tactics)?;
        Ok((extended, prepared))
    }

    /// [`prepare_extended_with`](Self::prepare_extended_with) for resuming the checkpoint at `checkpoint`, with its
    /// tactics ([`search::prepare_extended_for_resume`]).
    ///
    /// # Errors
    ///
    /// As [`prepare_extended_with`](Self::prepare_extended_with), and if the checkpoint does not belong to this problem
    /// or cannot be read.
    pub fn prepare_extended_for_resume(
        &self,
        open: &Bits<W>,
        checkpoint: &Path,
    ) -> Result<(ExtendedFrames<W>, Prepared<W>), String> {
        let extended = build_extended(&self.tables, open)?;
        let (columns, one_sided) = (extended.columns(), &extended.one_sided);
        let candidates = &extended.candidates;
        let prepared = search::prepare_extended_for_resume(&self.tables, &columns, one_sided, candidates, checkpoint)?;
        Ok((extended, prepared))
    }

    /// Sets the search up to resume the checkpoint at `checkpoint`, with its tactics, see
    /// [`search::prepare_for_resume`].
    ///
    /// # Errors
    ///
    /// If the checkpoint does not belong to this board, or cannot be read.
    pub fn prepare_for_resume(&self, checkpoint: &Path) -> Result<Prepared<W>, String> {
        search::prepare_for_resume(&self.tables, &self.frames.columns(), &self.frames.candidates, checkpoint)
    }

    /// The ceiling on the promotion sum, given that the game of `n − 1` scores `prev_score` and is optimal; see
    /// [`promotion_cap`].
    pub fn promotion_cap(&self, prev_score: u64) -> Option<u64> {
        promotion_cap(self.n(), self.in_the_bag, prev_score)
    }

    /// The part of a game's moves that is a seed for the search: those that are promotion candidates here.
    pub fn seed(&self, moves: &[u32]) -> Bits<W> {
        moves.iter().copied().collect::<Bits<W>>() & self.frames.candidates
    }

    /// The search problem that [`prepare`](Self::prepare) poses, built again without a search, see
    /// [`search::problem_of`].
    ///
    /// # Errors
    ///
    /// As [`search::problem_of`].
    pub fn problem(&self) -> Result<Problem, String> {
        search::problem_of(&self.tables, &self.frames.columns(), &self.frames.candidates)
    }
}

/// The ceiling on the promotion sum of the board `1..=n`, whose `in_the_bag` is given, given that the game of `n − 1`
/// scores `prev_score` and is optimal: `n + prev_score − in_the_bag`.  `None` if that is negative, which proves the
/// game of `n − 1` was not optimal (no game scores less than `in_the_bag`).  A free function, for code that knows a
/// board only by its `in_the_bag`.
pub fn promotion_cap(n: u32, in_the_bag: u64, prev_score: u64) -> Option<u64> {
    (prev_score + u64::from(n)).checked_sub(in_the_bag)
}

/// The search problem of game `n` ([`Board::problem`], at the set width `with_width!` picks for `n`), and the board's
/// `in_the_bag`: what `taxman verify-results` and the solver compare when a game repeats an earlier game's search.
///
/// # Errors
///
/// As [`search::problem_of`], and if `n` is beyond the widest set.
pub fn search_problem(n: u32) -> Result<(Problem, u64), String> {
    let beyond = || Err(format!("n={n} is beyond the largest supported board"));
    crate::with_width!(n, W => {
        let board = Board::<W>::for_n(n);
        board.problem().map(|problem| (problem, board.in_the_bag))
    }, else beyond())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{replay, Optimal};
    use crate::search::ProblemSize;

    #[test]
    fn a_board_knows_its_frames_and_what_the_game_with_no_promotions_scores() {
        let board = Board::<8>::for_n(85);
        assert_eq!(board.n(), 85);
        // The frames of 85 are the Java tests' (three frames, ten candidates).
        assert_eq!(board.frames.frames.len(), 3);
        assert_eq!(board.candidates().to_vec(), [8, 12, 16, 20, 24, 28, 30, 36, 40, 42]);
        assert!(!board.is_trivial());
        assert_eq!(replay(85, &board.plain), Ok(board.in_the_bag));

        // A board without candidates has one game, which is the one with no promotions.
        let trivial = Board::<8>::for_n(10);
        assert!(trivial.is_trivial());
        assert_eq!(Some(trivial.in_the_bag), Optimal::built_in().score(10));
    }

    #[test]
    fn the_cap_and_the_seed_are_derived_from_the_previous_game() {
        let optimal = Optimal::built_in();
        let board = Board::<8>::for_n(85);
        let (prev_score, prev_moves) = (optimal.score(84).unwrap(), optimal.moves(84).unwrap());
        // No game of 85 scores above 85 + opt(84), and every game scores in_the_bag + its promotions.
        assert_eq!(board.promotion_cap(prev_score), Some(85 + prev_score - board.in_the_bag));
        assert!(optimal.score(85).unwrap() - board.in_the_bag <= board.promotion_cap(prev_score).unwrap());
        // A previous game that scores less than the game with no promotions cannot have been optimal.
        assert_eq!(board.promotion_cap(0), (85u64).checked_sub(board.in_the_bag));
        let seed = board.seed(prev_moves);
        assert!(seed.is_subset(&board.candidates()) && seed.iter().all(|x| prev_moves.contains(&x)));
    }

    /// What `taxman verify-results` and `same_search` set up again from a results line (the board size) is the
    /// problem the setup posed, at any set width.
    #[test]
    fn the_problem_set_up_again_is_the_one_the_setup_posed() {
        for n in 2..=120 {
            let board = Board::<8>::for_n(n);
            if board.is_trivial() {
                continue;
            }
            let prepared = board.prepare(SearchTactics::default()).unwrap();
            let problem = prepared.problem();
            assert_eq!(board.problem().unwrap(), problem, "n={n}");
            let frames = board.frames.frames.len() as u32;
            let size = ProblemSize { frames, candidates: board.candidates().len(), max_total: prepared.max_total };
            assert_eq!(problem.size(), size, "n={n}");
            let (narrow, wide) = (board.problem().unwrap(), Board::<16>::for_n(n).problem().unwrap());
            assert_eq!((narrow.digest(), &narrow), (wide.digest(), &wide), "n={n}");
        }
        let board = Board::<8>::for_n(85);
        assert_eq!(search_problem(85).unwrap(), (board.problem().unwrap(), board.in_the_bag));
    }

    /// The extended problem with no open source poses the search what the frame model does on every board to 300 that
    /// has something to promote: the same `max_total` and candidates, and the same `in_the_bag`.
    #[test]
    fn the_extended_problem_without_open_sources_is_posed_as_the_frame_model_is() {
        for n in 2..=300 {
            let board = Board::<8>::for_n(n);
            if board.is_trivial() {
                continue;
            }
            let plain = board.prepare(SearchTactics::default()).unwrap();
            let (extended, prepared) = board.prepare_extended(SearchTactics::default()).unwrap();
            assert_eq!((prepared.max_total, prepared.candidates), (plain.max_total, plain.candidates), "n={n}");
            assert_eq!(extended.in_the_bag, board.in_the_bag, "n={n}");
        }
    }

    #[test]
    fn prepare_sets_the_search_up_as_the_search_module_does() {
        let board = Board::<8>::for_n(300);
        let tactics = SearchTactics::default();
        let prepared = board.prepare(tactics).unwrap();
        let direct = search::prepare::<8>(&board.tables, &board.frames.columns(), &board.candidates(), tactics);
        let direct = direct.unwrap();
        assert_eq!((prepared.n, prepared.max_total, prepared.greedy), (direct.n, direct.max_total, direct.greedy));
        assert_eq!(prepared.candidates, board.candidates());
    }
}
