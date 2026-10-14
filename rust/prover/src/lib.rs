//! The prover: independent proofs and checks of the solver's results.
//!
//! The prover is the second program of the repository: a branch and bound over sets of moves on the whole board that
//! decides playability by peel (Theorem 1), uses only proven bounds, takes the recorded score as its target and proves
//! that no set of moves beats it, so that the frame model's answers are checked without assuming the frame model.  It
//! also lists every optimal set, finds the longest games, certifies conjecture (★) at scale and searches for
//! adversarial instances.
//!
//! This crate shares no code with the solver (`../src`) or with the Python heuristics; it rests only on the
//! theorems of `heuristics/THEORY.md` (playability = peel succeeds = no self-covering subset; residual
//! confinement) and on elementary bounds.  The report is `README.md` next to this file.
//!
//! * [`primes`]: the sieve.
//! * [`instance`]: reduction instances (members, their maximal factors) and the upper-half instance of a game.
//! * [`peel`]: dismantling, residuals, playability, components, descending greedy.
//! * [`bounds`]: the matching-number, hypergraphic-matroid (Lorea) and rank bounds on the largest playable subset.
//! * [`maxcard`], [`plus1`]: exact maximum-cardinality playable subsets by branch and bound over cores.
//! * [`span`]: the span check (♦), which certifies (★) on a whole instance by GF(2) linear algebra.
//! * [`star`]: the per-threshold check of conjecture (★) on one instance.
//! * [`game`]: the full game on the whole board: replay, peel, and an exact branch and bound.
//! * [`moves`]: the longest games (the most moves), against the optimal game's moves.
//! * [`parity`]: the parity matroids, two GF(2) matroids every playable set is independent in, and their certified
//!   weighted intersection, a bound on opt(n).
//! * [`adversary`]: synthetic Taxman-shaped instances for the adversarial search.
//! * [`jsonl`]: the resumable result files every campaign writes.

pub mod adversary;
pub mod bounds;
pub mod game;
pub mod instance;
pub mod jsonl;
pub mod maxcard;
pub mod moves;
pub mod parity;
pub mod peel;
pub mod plus1;
pub mod primes;
pub mod span;
pub mod star;
