//! An exact solver for the game of Taxman, and the `taxman` command that runs it.
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
//! groups of numbers, the *hives*, stacks the ones that have numbers to spare into *frames*, and turns an optimal
//! game into the promotion set with the largest sum that keeps every frame winnable.  A branch-and-bound over
//! promotion sets, bounded by a matching relaxation and run on many threads, finds that set; a game of `n` is
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
//!       board::Board                   hives::Apiary → frames::build_frames: the frames (0 = bottom) and the
//!        │                              promotion candidates; in_the_bag = the score with no promotions
//!        ▼
//!       search::prepare                model::Model: the root state; greedy incumbent; revised frame 0
//!        │
//!        ▼
//!       search::search ───────────────► engine::run: threads, shared deque with disk spill, checkpoints
//!        │                               └ Stepper::step per node: matchbound → conflicts, forced
//!        │                                 members, nogoods → branch → child
//!        ▼
//!       promotions ──► hives::Apiary ──► the moves ──► game::replay ──► score, results line
//! ```
//!
//! [`solver`] runs this for a game, with the shortcuts that can skip the search (`accelerate`), and for a range
//! of games; [`results`] checks and keeps what it finds.
//!
//! # Module map
//!
//! In the order to read them (README, "Reading order", is the same list with advice for a first read): each layer
//! uses the ones above it.  Each module starts with a comment that says what it is for.
//!
//! | layer | module | what it is |
//! |---|---|---|
//! | foundations | [`game`] | the rules as a referee ([`game::replay`]), and the table of optimal games ([`game::Optimal`]) |
//! | | [`numtheory`] | maximal factors and composites, primes, divisors of the numbers on a board |
//! | | [`bits`] | fixed-width bitsets `Bits<W>` for sets of game numbers; the set widths, dispatched |
//! | | [`json`] | a small JSON reader and writer (the table of optimal games, golden data, results lines) |
//! | frame model | [`hives`] | sources, factors and hives; what a board's hives can play and spare |
//! | | [`frames`] | hives stacked into frames, and the promotion candidates |
//! | | [`model`] | the search state: frames, winnability, promotion, conflicts, nogoods |
//! | | [`accelerate`] | deriving the game of `n` from the game of `n − 1` without a search; promotions → moves |
//! | search | [`matchbound`] | the matching bound: the default upper bound on a node's completions |
//! | | [`search`] | the branch-and-bound searcher, its tactics, its setup and checkpoint header |
//! | | [`bounds`] | the Java solver's bound (`blameGame`), preclusions, the greedy incumbent, candidate lists |
//! | engine | [`engine`] | the problem-independent parallel depth-first driver: threads, spill, checkpoints |
//! | solving | [`tactics`] | the switchable pieces of the algorithm, by name and value |
//! | | [`board`] | a board analysed and prepared for a search: tables, frames, cap and seed |
//! | | [`solver`] | the per-game pipeline, the options, the range driver |
//! | | [`results`] | what a result line records about its proof, and the checks a results file passes |
//! | | [`paths`] | where the default files are, found at run time |
//! | studies | [`brute`] | exhaustive search without the frame model, to certify small boards |
//! | | [`sacrifice`] | the sacrifice bound and punctured games; not used by the solver |
//!
//! The command-line front end (`main.rs`, `cli.rs`, `commands.rs`) is the binary's, not the library's, and is read
//! last.  `bounds` is mostly the Java-parity layer: the default search takes from it only `max_additional`, the
//! greedy incumbent, the narrowing of the children's candidates and the scratch space of its peels (the module
//! comment lists them), and `sacrifice` and `brute` are studies; a first read can skip those two and read `bounds`
//! for that list only.  `examples/` holds the ladder (searches of hard games with time limits and thread counts), the
//! probe of the hot paths, and the sacrifice study; `tests/` the integration tests.

pub mod accelerate;
pub mod bits;
pub mod board;
pub mod bounds;
pub mod brute;
pub mod engine;
pub mod frames;
pub mod game;
pub mod hives;
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
