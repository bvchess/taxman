//! Independent empirical checks of the assumptions behind the solver's "optimal" results.
//!
//! This crate shares no code with the solver (`../src`) or with the Python heuristics; it rests only on the
//! theorems of `heuristics/THEORY.md` (playability = peel succeeds = no self-covering subset; residual
//! confinement) and on elementary bounds.  The report is `README.md` next to this file.
//!
//! * [`primes`]: the sieve.
//! * [`instance`]: reduction instances (members, their maximal factors) and the upper-half instance of a game.
//! * [`peel`]: dismantling, residuals, playability, components, descending greedy.
//! * [`bounds`]: the matching, hypergraphic-matroid (Lorea) and GF(2) incidence-rank bounds on the largest
//!   playable subset.
//! * [`maxcard`], [`plus1`]: exact maximum-cardinality playable subsets by branch and bound over cores.
//! * [`span`]: the span check (♦), which certifies (★) on a whole instance by GF(2) linear algebra.
//! * [`star`]: the per-threshold check of conjecture (★) on one instance.
//! * [`game`]: the full game, model-free: replay, peel over the whole board, and an exact branch and bound.
//! * [`jsonl`]: the resumable result files every campaign writes.

pub mod bounds;
pub mod game;
pub mod instance;
pub mod jsonl;
pub mod maxcard;
pub mod peel;
pub mod plus1;
pub mod primes;
pub mod span;
pub mod star;
