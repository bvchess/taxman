# Evidence

Part of [the theory](../THEORY.md).

These are the computations behind the statements that are not proved: statement U, the three open statements about the
sources, and the observations.  Each row names the statement, what was checked, by which program and on which input,
and what one must trust to accept it.  [The table of statements](statements.md#the-statements) gives each one's
standing.  A result that is proved needs no evidence; the tests that check the programs against the proved results are
in [the solver's design](../../solver/docs/design/tests.md#the-theorys-results).

## Statement U

Statement U holds at n when the frame solver's answer ΣU\* + P\*(Π_n) equals opt(n), by [the corollary of U
(X8)](exactness.md#statement-u).  Two computations prove that equality game by game, and how far each reaches drifts as
the work goes on, so the ranges are behind the links.

| statement | evidence | what one must trust |
|---|---|---|
| [statement U](exactness.md#statement-u), at the games the prover has proven | The prover proves opt(n) on the whole board, game by game, by a branch and bound with peel, [the parity bound (W1)](parity-bound.md#the-parity-bound) and the bounds between games ([solver/noframe/docs/proofs.md](../../solver/noframe/docs/proofs.md)).  Where its opt(n) is the frame solver's recorded score, U holds.  [results/README.md](../../results/README.md#what-supports-the-scores) gives the range. | the prover, and the frame solver's search for the score it matches |
| statement U, at the games with a certificate | `taxman certify` applies [the certificate theorem (E4)](extended-problem.md#the-certificate-theorem): the prover's barred bounds ([W2](parity-bound.md#the-barred-bound), `solver/noframe/data/upper_*.jsonl`) certify most members of U\*_{≥3}, and a search of the extended problem with the others open finds nothing above the recorded score.  `taxman verify-certificates` checks the file.  [results/README.md](../../results/README.md#certificates) gives the games and the time. | the frame solver's search of the extended problem, and the prover's barred bounds |
| the share that barred bounds certify | The barred bounds alone certify most members of U\*_{≥3} in every game, and all of them in none ([proofs.md, "The upper part, game by game"](../../solver/noframe/docs/proofs.md#the-upper-part-game-by-game)). | the prover |

Whether every optimal set holds all of U\*, the strong form, is measured with [the uniqueness of the optimal
set](#observations).

## The open statements about the sources

The check of (♦) certifies the other two, by [the span theorem (S6)](sources.md#the-span-theorem).  [solver/noframe/docs/
star.md](../../solver/noframe/docs/star.md#conjecture--at-scale) has the tables, the data and the programs.

| statement | evidence | what one must trust |
|---|---|---|
| [the span conjecture (♦)](sources.md#the-span-conjecture) | The prover's check of (♦) (`src/span.rs`) is descending greedy with Gaussian elimination over GF(2), on the components of the residual.  It found no violation in every game n = 2..100,000, in 61 minutes, nor in 902 sampled games from 101,000 to 2,000,000, in 3.7 hours (`solver/noframe/data/span_2_100000.jsonl.gz`, `span_samples_101000_2000000.jsonl.gz`). | the check's program, whose descending greedy equals the heuristics' on every game to 1000 and 62 samples to 6000 |
| [the cardinality conjecture (★)](sources.md#the-cardinality-conjecture) | Certified wherever (♦) is, by S6.  Separately, the prover's check threshold by threshold (`src/star.rs`) settles every threshold of every game n = 2..20,000 by a bound alone, with no search: one exclusion is always needed, the matching-number bound, Lorea's hypergraphic-matroid bound, or the rank bound (`solver/noframe/data/star_2_20000.jsonl.gz`). | the check of (♦), or the check by thresholds |
| [the sources conjecture (S1)](sources.md#the-sources-conjecture) | Certified wherever (♦) is, by S6. | the check of (♦) |

## Observations

| statement | evidence | what one must trust |
|---|---|---|
| [the protection of the window law](sources.md#what-a-proof-must-use) | The prover's `src/adversary.rs` keeps the incidence structure of real games and changes the order of the sources.  On 1,981 instances of games 2000 to 20,000, under orders that obey the window law, random (900) or adversarial (781), or under its reverse (300), (♦) and (★) never failed.  Uniformly random orders fail (♦) on 16% to 50% of the instances, and the window law for a random ranking of the primes on 26% to 57% (`tools/adversary_campaign.sh`, `data/adversary/`; [star.md](../../solver/noframe/docs/star.md#adversarial-search-what-protects-greedy)). | the adversary's program |
| [the component of 2873](sources.md#what-a-proof-must-use) | An exact search over the 70 members of the component's residual, 12.4 million checks of playability, found no playable subset with fewer exclusions than descending greedy's six.  It also follows from (★) at 2873, which the check of (♦) certifies. | for the search, its record alone: the program was not committed (commit e04f7c4 records the run); for the second proof, the check of (♦) |
| [the monomial-impossibility criterion](sources.md#what-a-proof-must-use) | The exponent vectors' majorization agreed with 300 sampled constellations of primes, with no disagreement. | the record alone: the program was not committed (commit fc60228) |
| [the 3-prime census](sources.md#what-a-proof-must-use) | The census padded 77 minimal cores of three primes with up to two members, to 428 shapes of up to eight members, exhaustively to seven members and by sampling at eight.  Of their 22,909 trap orders, 22,891 ask for an order no primes realize, 11 were never realized, and 7 are realizable.  On all 428 shapes, a shape can trap descending greedy exactly when its maximal playable subsets come in two adjacent sizes. | the record alone: the program was not committed (commit 20822ea) |
| [the window guards](sources.md#what-a-proof-must-use) | Each of the 18 realizations tested of the seven realizable trap orders, in components of up to 6,186 members, was defeated by its guard: q³ for five of the orders and q²r for two. | the record alone (commit 20822ea) |
| [the uniqueness of the optimal set to 424](uniqueness-and-longest-games.md#optimal-sets-on-the-whole-board) | The prover lists every optimal set of a game (`src/bin/optsets.rs`, its branch and bound in a mode that keeps every set at the recorded score).  It covered every game 2..989, 954 of them completely (`solver/noframe/data/optsets_2_989.jsonl`): one optimal set in every game to 424, and two or three in 30 games of 425 to 779.  In all 954, the optimal sets have the upper part U\* and one size, and differ by exchanges of equal sum below n/2 ([uniqueness.md](../../solver/noframe/docs/uniqueness.md)). | the prover's enumeration, which agrees with brute force to 19 and with a search over pots that uses no theory to 57 |
| [the shortfall of longest games to 1300](uniqueness-and-longest-games.md#the-length-proposition) | The prover's `src/bin/moves.rs` settles ℓ(n) for every game 2..1300 (`solver/noframe/data/moves_2_1300.jsonl`), in seven minutes on one thread.  The recorded optimal game is a longest game by the matching certificate in 1,293 games and by a search in 3 (51, 182 and 183), and one move short in 3 (833, 834 and 835) ([longest-games.md](../../solver/noframe/docs/longest-games.md)). | the prover's branch and bound; every longer game is replayed by the rules |
| [the exactness of the parity bound to 77](parity-bound.md#what-the-parity-bound-misses) | The prover computed the parity bound of every game 2..1300, each value with the weight split that checks it, beside its proof of opt(n) (`solver/noframe/data/exact_parity_2_1300.jsonl`).  The two are equal at every n ≤ 77, and differ first at 78. | the prover's proofs of opt(n); the bounds are checked by their splits |
| [the closed frames of layer 2](frame-model.md#the-bottom-layers-theorem) | The census of the frames (`solver/frame/examples/structure.rs`) ran over every board 2..8191, in 1,271 s summed over the boards: a frame of layer 2 has freedoms on 80 boards, all at most 186 ([its notebook entry](../../solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/README.md)). | the census, which forms the frames with the frame solver's own setup |
| [the frame bound against the parity bound](frame-bounds.md#the-frame-bound) | At the 361 roots of the games 880 to 1300 that are not prime, the parity bound equals ΣU\* plus the frame bound ([matroid intersection bounds](../../solver/notebook/search/matroid-intersection-bounds-2026-10/README.md)). | the study's program, `examples/matroid.rs`, removed after the study and built at commit 8753189; its intersection agrees with brute force on 3,000 random pairs of matroids |
