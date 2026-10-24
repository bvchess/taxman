# The theory, with proofs

The theory proves what the project's programs rest on: the frame solver, which finds optimal games; the prover, which
proves them optimal on the whole board; and the claims of the heuristics.  Each result is stated in words, then
formally, then proved.  It has a name, such as the ceiling lemma, and a label: the letter of its file and its number
there, as in N4.  [statements.md](theory/statements.md) lists every result with its standing (proved, proved given an
open statement, certified game by game, measured, or open), the notation and the open questions.
[evidence.md](theory/evidence.md) gives the computations behind the standings that rest on computation, and the
heuristics' table of claims ([heuristics/README.md](../heuristics/README.md)) cites the theory.

## The files

In reading order, each file building on those above it:

| file | letter | read it for |
|---|---|---|
| [primer.md](theory/primer.md) | | the standard mathematics the theory uses: GF(2), matroids and their greedy algorithm, matroid intersection, Lagrangian relaxation, permanents and determinants |
| [statements.md](theory/statements.md) | | every result with its standing and a link to it, the notation, and the open questions |
| [playability.md](theory/playability.md) | P | when a set of moves can be played: payment schemes, peel, cores and the residual, linear independence |
| [neighbors.md](theory/neighbors.md) | N | what passes from game n − 1 to game n: the prime lemma, monotonicity, the ceiling, the prime formula |
| [sources.md](theory/sources.md) | S | whether U\*, the sources that descending greedy keeps, is the heaviest playable set of sources: the open statements (★) and (♦) |
| [uniqueness-and-longest-games.md](theory/uniqueness-and-longest-games.md) | Q | whether the optimal set is unique, and whether an optimal game plays the most moves |
| [parity-bound.md](theory/parity-bound.md) | W | the parity bound on opt(n), which bounds the prover's searches, and the barred bound |
| [franklin-moniot.md](theory/franklin-moniot.md) | C | how the theory relates to Franklín and Moniot's paper, through a count of payment schemes |
| [frame-model.md](theory/frame-model.md) | F | the frame model that the frame solver searches: frames, layers, the bottom layers, promotion sets, the promotion limit |
| [exactness.md](theory/exactness.md) | X | why the frame model is exact, statement U, and boards with holes |
| [extended-problem.md](theory/extended-problem.md) | E | the extended problem, which the certificates of statement U search |
| [frame-bounds.md](theory/frame-bounds.md) | B | the bounds on the promotions: signed frame matroids, the frame bound, the decomposition bound |
| [search-rules.md](theory/search-rules.md) | R | what a search of the frame model proves, every rule it may cut by, and the bounds carried between games |
| [evidence.md](theory/evidence.md) | | the computations behind the statements that are not proved |
| [writing.md](theory/writing.md) | | how the theory is written, for those who write it |

## What rests on what

Most of the theory is proved outright.  One open statement, U, stands between the frame model and opt(n), and the frame
solver's scores rest on it wherever no other proof has reached.  The open statements about the sources, (★) and (♦),
explain why the frame model may fix the sources as it does, but no score rests on them.

| rests on | results | what follows for the programs |
|---|---|---|
| nothing open | P1–P10, N1–N6, S2, S3, S6, Q2, Q4, W1, W2, C1–C4, F1–F6, X1–X7, X9, the inequality of X8, E1–E5, B1–B5, R1–R5, R6 (1), R7 | the prover's proofs of opt(n); the certificates of U; the frame solver's answer ΣU\* + P\* as the score of a real game, at most opt(n) |
| statement U at n | the equality of X8; R6 (2, 3) | the frame solver's score is opt(n); a repeat carries an optimal game from game m to game n |
| (★) in game n | Q1, Q3's last part, S1 by Q1 | U\* is the only heaviest playable set of sources, and lies termwise above every other |
| (♦) in game n | (★) and S1, by S6 | the check of (♦) certifies (★) and S1 game by game |

* **The prover's scores** rest on the playability theorem (P2), the peel theorem (P4), the parity bound (W1) and the
  bounds between neighboring games (N3 to N5), all proved, and on the prover being right.
* **The frame solver's scores** are ΣU\* + P\*(Π_n).  An exact search finds P\* by the rules of search-rules.md, with
  the frame bound (B2), all proved.  The exactness theorem (X7) and the bottom-layers theorem (F3) make the answer the
  best score of a game that plays U\*, and the corollary of U (X8) makes it opt(n) exactly when U holds at n.  A repeat
  (R6), and every upper bound it carries between games, needs U at the games it passes through.
* **The certificates of U** apply the certificate theorem (E4), with the barred bound (W2) and the frame bound for the
  extended problem (B3).  A game that the prover or a certificate has proven does not rest on U;
  [results/README.md](../results/README.md#what-supports-the-scores) says which games those are.
* **(★) and (♦)** concern the sources alone.  The frame model fixes the sources to U\* by construction, and its
  exactness is proved without them.  They give the uniqueness theorem (Q1) and part of the length lemma (Q3), and the
  check of (♦) certifies them game by game.
