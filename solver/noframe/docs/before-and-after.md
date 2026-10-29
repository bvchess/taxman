# What existed before, and what remains open

Part of [the prover's README](../README.md).

## What existed before

Before the prover, this was the state of the evidence.

* **Proven** ([the theory](../../../docs/THEORY.md)):
  * playability ⟺ a maximal-factor matching with acyclic precedence ([the playability theorem
    (P2)](../../../docs/theory/playability.md#the-playability-theorem)), and peel decides it ([the peel theorem
    (P4)](../../../docs/theory/playability.md#the-peel-theorem));
  * for separated instances, playable ⟺ no self-covering subset, and the minimal exclusion sets lie in the residual
    ([P5](../../../docs/theory/playability.md#the-residual-theorem),
    [P7](../../../docs/theory/playability.md#the-core-theorem) and
    [P8](../../../docs/theory/playability.md#the-exclusion-theorem));
  * greedy is weight-optimal wherever the frontier jam is hyperedge-free ([the frontier theorem
    (S3)](../../../docs/theory/sources.md#the-frontier-theorem));
  * the bounds opt(n) ≤ n + opt(n − 1) ([the ceiling lemma (N4)](../../../docs/theory/neighbors.md#the-ceiling-lemma)),
    the prime formula ([N5](../../../docs/theory/neighbors.md#the-prime-formula)), and the Franklín–Moniot bound.
* **(★), the cardinality conjecture ([S4](../../../docs/theory/sources.md#the-cardinality-conjecture)).**
  docs/THEORY.md had, to commit ddbaf30, 3,231 of 3,236 rejection-bearing components (all n = 6..1500, forty samples to
  4000, n = 2873 and 9170) verified by exact search over the residual or certified by the matching-number bound.  Five
  were undecided (n = 3103, 3872, 3936, 4000, 9170, with search budgets of 2.7×10⁹ to 1.1×10⁴⁷), the last "to be settled
  only by a theorem"; the uncertified 0.2% were exactly those five.  Besides:
  * an exhaustive check that U\* is the max-playable set of sources for n = 2..99, and a brute force over pot states
    (`heuristics/evaluation/certify.py`) that the heaviest set of sources is unique to n = 62, with one optimal game
    recorded per n;
  * the census of 3-prime padded jams: 428 shapes, 22,909 trap orderings, none unprotected;
  * `heuristics/evaluation/verify.py`: U\* equals the sources of the optimal game for n ≤ 1000.

  The scripts and data of the (★) campaign and of the census were never committed, only their numbers, in THEORY.md
  and the commit messages of e04f7c4, 20822ea and 43936d3.  So that evidence could not be reproduced; this crate's
  checks can.
* **The frame model** ([solver/docs/design/frame-model.md](../../docs/design/frame-model.md)).  Its assumptions (the
  greedy removal of impossible sources, `unzip`'s mandatory moves, freedoms = factors − sources, closed frames inert,
  promotions one frame forward) were validated end to end:
  * the committed chain reproduces the Java solver's table of optimal games to 1000, score for score; the same model
    made that table, so the agreement is not independent;
  * `src/brute.rs` matches it to 40 in its tests, and once to 52, by the example `sacrifice`, removed from master;
  * `heuristics/evaluation/certify.py`'s brute force over pot states confirms it to n = 62.

  Beyond 1000 (`results`), nothing was independent of the frame model.

## What remains open

* **Conjectures (★) and (♦)** are certified game by game, not proven.  The sharpest form found is to prove (♦): that
  every rejection of descending greedy on the sources' instance is a GF(2) dependency of what it kept above.  That is a
  statement about cores and closed surfaces mod 2, which the adversarial results tie to the window law with the real
  ranking of the primes.
* **The prover's proofs reach 2175**, where the committed chain ended when they were made; the chain reaches further
  within the frame model.  1301–2175 took 226 thread-hours ([proofs.md](proofs.md)), and a longer chain needs more.
  What the parity bound still misses are the [odd
  cores](../../../docs/theory/parity-bound.md#what-the-parity-bound-misses).  Its root gap is 130 points on average at
  1000–1300 and 384 over the searched games of 1301–2175 (125 to 767, with no steady rise), and the cost follows it,
  tenfold the nodes per 210 points.  The conflicts of its heaviest sets are large: 20–45 members at 990.
* **The optimal sets of 990–1300 are not enumerated.**  The enumeration mode works with the parity bound (tested against
  brute force), so `optsets` with it would extend [uniqueness.md](uniqueness.md) at about the cost of the proofs.
* **Statement U past 2175.**  The frame model's individual assumptions are no longer open: they are theorems given a
  per-board fact that holds on every board to 8191 ([the bottom-layers theorem
  (F4)](../../../docs/theory/frame-model.md#the-bottom-layers-theorem) and [the exactness theorem
  (X7)](../../../docs/theory/exactness.md#the-exactness-theorem)).  What the model's answers rest on is one statement,
  U: some optimal game plays every source of three or more prime factors that setup keeps.  The prover's proofs
  establish U at every n ≤ 2175.  Beyond, the committed games past 2175 that rest on U, by an exhausted search or a
  repeat, have it uncertified.  The barred parity bounds certify about 85% of those sources per game on 2176–3314 and
  80% on 3315–3440, but close no game alone ([proofs.md](proofs.md#the-upper-part-game-by-game)).  The rest needs a
  search that leaves the open sources optional: the certificates of U
  ([solver/docs/design/certificates.md](../../docs/design/certificates.md)), which superseded the measurement E6 of
  [reproducing.md](reproducing.md).
* **Whether every optimal game's upper part is U\*.**  [The uniqueness theorem
  (Q1)](../../../docs/theory/uniqueness-and-longest-games.md#the-uniqueness-theorem) settles the sources' instance
  alone.  For the whole game, a frontier argument, as in [S3](../../../docs/theory/sources.md#the-frontier-theorem),
  would start from an optimal set S whose upper part S_U is not U\*.  Termwise domination puts the largest disagreement
  d in U\* ∖ S_U, and S + d is unplayable, so the question is whether some repair costs less than d.  The picks below
  n/2 are all below d, and every exchange observed between optimal sets turns on numbers below n/2 that are maximal
  factors of members of U\*, which is where to look.  Also observed but unexplained: a game's optimal sets all have the
  same size.  The cheapest next evidence is the other half for the 34 games enumerated only with the sources fixed.
  Their enumeration costs about what their proofs did (some 6 CPU-hours in all, 5 to 100 minutes for each of the 18 that
  took a search), so it wants `exact`'s pausing and Lagrangian bound with the target lowered by one and the enumeration
  mode on, which would let `exact` return every optimal set with each proof.
