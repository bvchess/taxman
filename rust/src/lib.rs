//! A solver for the game of Taxman, and the `taxman` command that runs it.
//!
//! # The game
//!
//! The pot holds the numbers `1..=n`.  The player takes a number that still has a proper divisor in the pot and
//! scores it; the taxman takes that number and every divisor of it still in the pot.  The game is over when no
//! number in the pot has a divisor left in it.  An *optimal game* scores the most; the best scores for
//! `n = 1, 2, 3, …` are the Taxman sequence.  [`game::replay`] is the rules in code, and the referee that every
//! game the solver finds must satisfy.
//!
//! # What the crate does
//!
//! It finds optimal games.  Every number above `n/2` is never tax and is played (unless it turns out to be
//! impossible), so a game is chosen by which of the other numbers to *promote* to a move as well.  The *frame
//! model* (a restatement of the algorithm of the Java solver in `src/main/java`) splits the board into independent
//! *frames*, each a group of sources with the factors they compete for, numbers the ones that have freedoms from
//! the front (0 is played first) and merges those with the same number, and turns an optimal game into the
//! promotion set with the largest sum that keeps every frame winnable.  A branch-and-bound over
//! promotion sets, bounded by the heaviest set independent in two GF(2) matroids of the frames (or by a matching
//! relaxation) and run on many threads, finds that set; a game of `n` is
//! built from the game of `n − 1`, so a chain of games from `n = 1` proves each from the one before.  `README.md`
//! (in the crate's directory) is the user guide and has the glossary; `DESIGN.md` explains the design;
//! `docs/EXPERIMENTS.md` is the lab notebook of how it got there.
//!
//! # From a board size to a game
//!
//! ```text
//! n ──► numtheory::Tables               maximal factors and composites of 1..=n
//!        │
//!        ▼
//!       board::Board                   frames::BoardFrames → frames::build_frames: the frames (0 = front) and the
//!        │                              promotion candidates; in_the_bag = the score with no promotions
//!        ▼
//!       search::prepare                model::FrameGraph and its root FrameSet; greedy incumbent
//!        │
//!        ▼
//!       search::search ───────────────► engine::run: threads, shared deque with disk spill, checkpoints
//!        │                               └ Stepper::step per node: gf2 (or matchbound) → conflicts,
//!        │                                 forced members, nogoods → branch → child
//!        ▼
//!       promotions ──► frames::BoardFrames ──► the moves ──► game::replay ──► score, results line
//! ```
//!
//! [`solver`] runs this for a game, with the shortcuts that can skip the search (`accelerate`, and the repeat of an
//! earlier game's search, which the setup's [`search::Problem`] finds), and for a range of games; [`results`] checks
//! and keeps what it finds.
//!
//! # Module map
//!
//! In the order to read them (README, "Reading order", is the same list with advice for a first read): each layer
//! uses the ones above it.  Each module starts with a comment that says what it is for.
//!
//! | layer | module | what it is |
//! |---|---|---|
//! | foundations | [`game`] | the rules as a referee ([`game::replay`]), and the table of optimal games |
//! | | [`numtheory`] | maximal factors and composites, primes, divisors of the numbers on a board |
//! | | [`bits`] | fixed-width bitsets `Bits<W>` for sets of game numbers; the set widths, dispatched |
//! | | [`json`] | a small JSON reader and writer (the table of optimal games, golden data, results lines) |
//! | frame model | [`frames`] | the frames of a board: formed, simplified, numbered from the front and merged |
//! | | [`model`] | the search state: frames, winnability, promotion, conflicts, nogoods |
//! | | [`accelerate`] | the prime shortcut (the game of a prime `n` from the game of `n − 1`); promotions → moves |
//! | search | [`matchbound`] | the matching bound, an upper bound on a node's completions; the Lagrangian bound |
//! | | [`gf2`] | the GF(2) bound, the default node bound: two frame matroids and their weighted intersection |
//! | | [`search`] | the branch-and-bound searcher, its tactics, its setup and checkpoint header |
//! | | [`bounds`] | the greedy incumbent and the limit on a node's further promotions |
//! | engine | [`engine`] | the problem-independent parallel depth-first driver: threads, spill, checkpoints |
//! | solving | [`tactics`] | the switchable pieces of the algorithm, by name and value |
//! | | [`board`] | a board analysed and prepared for a search: tables, frames, ceiling and seed |
//! | | [`solver`] | the per-game pipeline, the options, the range driver |
//! | | [`results`] | what a result line records about its proof, and the checks a results file passes |
//! | | [`paths`] | where the default files are, found at run time |
//! | studies | [`brute`] | exhaustive search without the frame model, to certify small boards |
//! | | [`sacrifice`] | the sacrifice bound and punctured games; not used by the solver |
//!
//! The command-line front end (`main.rs`, `cli.rs`, `commands.rs`) is the binary's, not the library's, and is read
//! last.  `bounds` is two small functions that the Java solver has too (`max_additional` and the greedy
//! incumbent), and `sacrifice` and `brute` are studies that a first read can skip.  `examples/` holds the ladder
//! (searches of hard games with time limits and thread counts), the probe of the hot paths, the sacrifice study,
//! the list of games whose search repeats an earlier game's, and the slope study (the gaps of a game's root, and
//! proofs with every bound lowered by a margin); `tests/` the integration tests.

pub mod accelerate;
pub mod bits;
pub mod board;
pub mod bounds;
pub mod brute;
pub mod engine;
pub mod frames;
pub mod game;
pub mod gf2;
pub mod json;
pub mod matchbound;
pub mod model;
pub mod numtheory;
pub mod paths;
pub mod results;
pub mod sacrifice;
pub mod search;
pub mod solver;
pub mod tactics;

/// Helpers the unit tests share with the integration tests (`tests/common/support.rs`: temporary directories and a
/// random number generator).
#[cfg(test)]
#[path = "../tests/common/support.rs"]
pub(crate) mod test_support;
