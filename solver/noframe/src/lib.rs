//! The prover: independent proofs and checks of the solver's results.
//!
//! The prover is the second program of the repository: a branch and bound over sets of moves on the whole board that
//! decides playability by peel (the peel theorem, P4, docs/theory/playability.md), uses only proven bounds, takes the
//! recorded score as its target and proves that no set of moves beats it, so that the frame model's answers are checked
//! without assuming the frame model.  It also lists every optimal set, finds the longest games, certifies conjecture
//! (★) at scale and searches for adversarial instances.
//!
//! This crate shares no code with the frame solver (`../frame/src`) or with the Python heuristics; it rests only on the
//! theorems of `docs/theory/` (playability = peel succeeds = no self-covering subset, P2, P4 and P7; residual
//! confinement, P8; the parity bound, W1, in docs/theory/parity-bound.md; and the bottom layers of every optimal set,
//! the bottom-layers theorem F4 of docs/theory/frame-model.md, which the proofs of `exact` start from by default) and
//! on elementary bounds.  The report is `README.md` next to this file.
//!
//! * [`primes`]: the sieve.
//! * [`instance`]: reduction instances (members, their maximal factors) and the sources' instance of a game
//!   (`Instance::upper_half`).
//! * [`peel`]: dismantling, residuals, playability, components, descending greedy.
//! * [`bounds`]: the matching-number, hypergraphic-matroid (Lorea) and rank bounds on the largest playable subset.
//! * [`maxcard`], [`plus1`]: exact maximum-cardinality playable subsets by branch and bound over cores.
//! * [`span`]: the span check (♦), which certifies (★) on a whole instance by GF(2) linear algebra.
//! * [`star`]: the per-threshold check of conjecture (★) on one instance.
//! * [`game`]: the full game on the whole board: replay, peel, and an exact branch and bound.
//! * [`moves`]: the longest games (the most moves), against the optimal game's moves.
//! * [`parity`]: the parity matroids, two GF(2) matroids every playable set is independent in, and their certified
//!   weighted intersection, a bound on opt(n).
//! * [`upper`]: which members of U\*_{≥3}, the sources of three or more prime factors that descending greedy keeps,
//!   every set beating a recorded score must hold, certified by the parity bound with the two bottom layers fixed
//!   (F4).
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
pub mod upper;
