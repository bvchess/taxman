//! The prover: independent proofs and checks of the solver's results.
//!
//! The prover is the second program of the repository: a branch and bound over sets of moves on the whole board that
//! decides playability by peel (the peel theorem, P4, docs/theory/playability.md), uses only proven bounds, takes the
//! recorded score as its target and proves that no set of moves beats it, so that the frame model's answers are checked
//! without assuming the frame model.  It also lists every optimal set, finds the longest games, certifies conjecture
//! (★) at scale and searches for adversarial instances.
//!
//! This crate shares no proof code with the frame solver (`../frame/src`) or with the Python heuristics; it rests only
//! on the theorems of `docs/theory/` (playability = peel succeeds = no self-covering subset, P2, P4 and P7; residual
//! confinement, P8; the parity bound, W1, in docs/theory/parity-bound.md; and the bottom layers of every optimal set,
//! the bottom-layers theorem F4 of docs/theory/frame-model.md, which the proofs of `exact` start from by default; and
//! the semiprime formula J3 of docs/theory/arrival.md, which proves an odd number with two prime factors from the game
//! before it) and on elementary bounds.  Both programs read the committed chain through the gzip decoder in `../gzip`,
//! whose CRC-32 check makes a decoding error fail loudly rather than give both programs the same wrong game.  The
//! report is `README.md` next to this file.
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
//! * [`parity`]: the parity vectors, two families of vectors over GF(2) in each of which every playable set is
//!   independent, and the certified heaviest set independent at both parities (weighted matroid intersection), a
//!   bound on opt(n).
//! * [`semiprime`]: the semiprime formula (J3), the exchange x with which opt(n) = opt(n − 1) + n − x at an odd number
//!   n with two prime factors.
//! * [`upper`]: which members of U\*_{≥3}, the sources of three or more prime factors that descending greedy keeps,
//!   every set beating a recorded score must hold, certified by the parity bound with the two bottom layers fixed
//!   (F4).
//! * [`adversary`]: synthetic Taxman-shaped instances for the adversarial search.
//! * [`jsonl`]: the resumable result files every campaign writes, and the reading of the recorded games.
//! * [`team`]: `exact`'s campaign, which searches a game on several threads when fewer games are left than threads.
//! * [`stop`]: the clean stop of that campaign on SIGINT and SIGTERM.

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
pub mod semiprime;
pub mod span;
pub mod star;
pub mod stop;
pub mod team;
pub mod upper;
