# The theory, with proofs

The theory proves what the project's programs rest on: the frame solver, which finds optimal games; the prover, which
proves them optimal on the whole board; and the claims of the heuristics.  Each result has a name, such as [the ceiling
lemma](theory/neighbors.md#the-ceiling-lemma), and a label: the letter of its file and its number there, as in N4.
[statements.md](theory/statements.md) lists every result with its standing, with the notation and open questions.  Four
statements are open: [statement U](theory/exactness.md#statement-u), [the sources
conjecture](theory/sources.md#the-sources-conjecture), (★) and (♦).  [evidence.md](theory/evidence.md) gives the
computations behind the standings that rest on computation, and the heuristics' table of claims
([heuristics/README.md](../heuristics/README.md)) cites the theory.

## The files

In reading order.  Each file's proofs rest only on the files above it, with one exception: in
[sources.md](theory/sources.md), [the span theorem](theory/sources.md#the-span-theorem) and the equivalence of (★) with
the weighted [sources conjecture](theory/sources.md#the-sources-conjecture) use [the uniqueness
theorem](theory/uniqueness-and-longest-games.md#the-uniqueness-theorem) of the next file.  Remarks look forward in
places.

| file | letter | read it for |
|---|---|---|
| [primer.md](theory/primer.md) | | the standard mathematics: GF(2), linear independence and forests, the greedy algorithm, weight splits, permanents |
| [statements.md](theory/statements.md) | | every result with its standing and a link to it, the notation, and the open questions |
| [playability.md](theory/playability.md) | P | when a set of moves can be played: payment schemes, peel, cores, linear independence |
| [neighbors.md](theory/neighbors.md) | N | what passes from game n − 1 to game n: [the prime lemma](theory/neighbors.md#the-prime-lemma), monotonicity, the ceiling, the prime formula |
| [sources.md](theory/sources.md) | S | whether U\*, the sources that descending greedy keeps, is the heaviest playable set of sources: the sources conjecture, (★) and (♦) |
| [uniqueness-and-longest-games.md](theory/uniqueness-and-longest-games.md) | Q | whether the optimal set is unique, and whether an optimal game plays the most moves |
| [frame-model.md](theory/frame-model.md) | F | the frame model that the frame solver searches: frames, layers, the bottom layers, promotion sets, the promotion limit |
| [exactness.md](theory/exactness.md) | X | why the frame model is exact, [statement U](theory/exactness.md#statement-u), and boards with holes |
| [parity-bound.md](theory/parity-bound.md) | W | the parity bound on opt(n), which bounds the prover's searches, and the barred bound |
| [franklin-moniot.md](theory/franklin-moniot.md) | C | how the theory relates to Franklín and Moniot's paper, through a count of payment schemes |
| [extended-problem.md](theory/extended-problem.md) | E | the extended problem, and the certificates of statement U that search it |
| [frame-bounds.md](theory/frame-bounds.md) | B | the bounds on the promotions: the frame bound and the decomposition bound |
| [search-rules.md](theory/search-rules.md) | R | what a search of the frame model proves, every rule it may cut by, and the bounds carried between games |
| [arrival.md](theory/arrival.md) | J | from game n − 1 to game n: the arrival lemma, the semiprime formula, the shadow law, the cofactor rule |
| [evidence.md](theory/evidence.md) | | the computations behind the statements that are not proved |
| [writing.md](theory/writing.md) | | how the theory is written, for those who write it |

## What rests on what

One open statement, U, stands between the frame model and opt(n), and the frame solver's scores rest on it wherever no
other proof has reached.  The open statements about the sources bear on no score: the frame model fixes the sources to
U\*, and its exactness needs neither (★) nor (♦).

* **Proved outright:** playability (P1 to P11); neighboring games (N1 to N7); the frame model and its exactness (F1 to
  F10, X1 to X7, [the holes theorem X9](theory/exactness.md#boards-with-holes), and the inequality of [the corollary of
  U, X8](theory/exactness.md#statement-u)); the parity and barred bounds (W1 to W3); the extended problem and its
  certificates (E1 to E6); the bounds on the promotions (B1 to B5, and B6 but its last part); the rules of a search (R1
  to R5, [the near-repeat lemma R7](theory/search-rules.md#the-near-repeat-lemma), and the first part of [the repeat
  lemma R6](theory/search-rules.md#the-repeat-lemma)); J1 to J3; the relaxation and length propositions (Q2, Q4) and
  the first two parts of [the length lemma (Q3)](theory/uniqueness-and-longest-games.md#the-length-lemma); the
  equivalence in [the cardinality conjecture (S4)](theory/sources.md#the-cardinality-conjecture); and the explanatory
  [victim law](theory/sources.md#the-victim-law), [frontier theorem](theory/sources.md#the-frontier-theorem), [span
  theorem](theory/sources.md#the-span-theorem) and Franklín–Moniot results (S2, S3, S6, C1 to C3).
* **Given statement U at n:** the equality of the corollary of U (X8), and the inequality and last part of the repeat
  lemma (R6).
* **Given (★) in game n:** [the uniqueness theorem (Q1)](theory/uniqueness-and-longest-games.md#the-uniqueness-theorem),
  the last part of the length lemma (Q3), and through Q1 [the sources
  conjecture](theory/sources.md#the-sources-conjecture) there.
* **Given (♦) in game n:** (★) and the sources conjecture there, by the span theorem (S6), so the check of (♦) certifies
  both game by game; the last part of B6; and J1's value of x.

What the programs' answers rest on:

* **The prover's scores** rest on [the playability theorem (P2)](theory/playability.md#the-playability-theorem), [the
  peel theorem (P4)](theory/playability.md#the-peel-theorem), [the parity bound
  (W1)](theory/parity-bound.md#the-parity-bound) and the bounds between neighboring games (N3 to N5), all proved, and on
  the prover being right.
* **The frame solver's scores** are ΣU\* + P\*(Π_n).  An exact search finds P\* by the rules of search-rules.md, with
  [the frame bound (B2)](theory/frame-bounds.md#the-frame-bound) and [the promotion-limit lemma
  (F9)](theory/frame-model.md#the-promotion-limit-lemma), all proved.  [The exactness theorem
  (X7)](theory/exactness.md#the-exactness-theorem) makes the answer the best score of a game that plays U\*.  The
  corollary of U (X8), which uses [the bottom-layers theorem (F4)](theory/frame-model.md#the-bottom-layers-theorem) for
  layers 1 and 2, makes it opt(n) exactly when U holds at n.  A repeat, and every upper bound it carries between games,
  needs U at the games it passes through.
* **The certificates of U** apply [the certificate theorem (E2)](theory/extended-problem.md#the-certificate-theorem):
  [the barred bound (W2)](theory/parity-bound.md#the-barred-bound) certifies most members of U\*_{≥3}, and a search of
  the extended problem, by [the state lemma (R1)](theory/search-rules.md#the-state-lemma), the dual-bound and floor
  lemmas (R4, R5) and [the frame bound for the extended problem
  (B3)](theory/frame-bounds.md#the-frame-bound-for-the-extended-problem), finds nothing above the score.  A search
  stopped at the ceiling (E3) rests also on the recorded score of game n − 1.  A game that the prover or a certificate
  has proven does not rest on U; [results/README.md](../results/README.md#what-supports-the-scores) says which games
  those are.
