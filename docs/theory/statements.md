# The statements

Part of [the theory](../THEORY.md).

This file is the theory's reference: the notation that every file shares, every result with its standing and a link to
its proof, and the questions that are open.  The words of the game are the [glossary's](../GLOSSARY.md), and each of the
theory's own terms is defined in the one file that first needs it, which the glossary links.

## Notation

Each symbol below keeps one meaning across the theory.  A letter that serves one proof only is defined there, and is
none of these.

| symbol | meaning | defined in |
|---|---|---|
| n, m | games: game n, and another game m, usually one proven before | [the glossary](../GLOSSARY.md#the-game) |
| opt(n) | the best score of game n | [the glossary](../GLOSSARY.md#the-game) |
| S\*, S\*(n) | an optimal set: the moves of an optimal game, of game n where the game must be named.  A game can have several, so a statement about S\* says whether it holds for every optimal set or for some. | [the glossary](../GLOSSARY.md#the-game) |
| ℓ(n) | the most moves of a game of n | [longest games](uniqueness-and-longest-games.md#longest-games) |
| ν(n), ν(S) | the matching number of the board's maximal-factor graph, or of a set's incidence graph | [longest games](uniqueness-and-longest-games.md#longest-games), [the primer](primer.md#matchings-permanents-and-determinants) |
| q(n) | the largest prime ≤ n; for a prime n, q(n − 1) is the largest prime below it | [neighbors.md](neighbors.md#neighboring-games) |
| p, q, r | primes, local to a statement; never a bare q where q(n) is in play | |
| Ω(x) | the number of prime factors of x, counted with multiplicity: its layer | [playability.md](playability.md#playability) |
| mf(x) | the maximal factors of x | [playability.md](playability.md#playability) |
| S_k, S_{≥k} | the members of a set S in layer k, and in layers k and up | here |
| ΣS | the sum of the members of a set S | here |
| F, F_h | a pool of factors: an instance's pool, or the factors of frame h | [instances](playability.md#payment-schemes), [the frame model](frame-model.md#the-frame-model) |
| R | the residual of an instance | [the residual](playability.md#cores-and-the-residual) |
| pay(x) | the factor that a payment scheme pays x with | [payment schemes](playability.md#payment-schemes) |
| v(s), u(s), e_s | the incidence vector of s; the same with the coordinates of a set's own members deleted; the unit vector of s | [independence](playability.md#independence) |
| 𝔽 | a field; GF(2) and GF(3) are the fields of two and three elements | [the primer](primer.md#finite-fields-and-gf2) |
| per(S, T) | the number of payment schemes of S whose payments are T | [Franklín and Moniot](franklin-moniot.md#what-the-paper-proves) |
| U\*, U\*_k | the sources that descending greedy keeps, and their part in layer k | [descending greedy](sources.md#descending-greedy) |
| U\*(n) | U\* of game n, where two games are in play | [the repeat lemma](search-rules.md#the-repeat-lemma) |
| U, (★), (♦) | statement U, the cardinality conjecture, the span conjecture | [statement U](exactness.md#statement-u), [(★)](sources.md#the-cardinality-conjecture), [(♦)](sources.md#the-span-conjecture) |
| k, k₀ | a layer, and the lowest layer of a frame with freedoms | [the frame model](frame-model.md#the-frame-model) |
| h | the number of a frame | [the frame model](frame-model.md#the-frame-model) |
| Π_n, Π⁺(O) | the root problem of game n, and the extended problem with the open sources O | [the frame model](frame-model.md#the-frame-model), [the extended problem](extended-problem.md#the-extended-problem) |
| Φ, J | the fixed sources U\* ∖ O of the extended problem, and the sources that descending greedy rejected in the frames holding a member of O | [the extended problem](extended-problem.md#the-extended-problem) |
| P\*(Π), P\* | the weight of the heaviest promotion set of Π; P\* alone is P\*(Π_n) | [the frame model](frame-model.md#the-frame-model) |
| ΣU\*, ΣΦ | the score with no promotions, in the frame model and in the extended problem | [the frame model](frame-model.md#the-frame-model), [the extended problem](extended-problem.md#the-extended-problem) |
| L | a promotion set; (K, L) in the extended problem, with K its sources | [the frame model](frame-model.md#the-frame-model), [the extended problem](extended-problem.md#the-extended-problem) |
| P, C, X | a node's promotions, its candidates, and what a completion adds to P | [searching the frame model](frame-model.md#searching-the-frame-model) |
| state(P) | the root frames with the promotions P applied, then reduced | [searching the frame model](frame-model.md#searching-the-frame-model) |
| the promotion limit | the most promotions the freedoms allow, written in words | [searching the frame model](frame-model.md#searching-the-frame-model) |
| T_h | the candidates that touch frame h: those of home h and of home h + 1 | [signed frame matroids](frame-bounds.md#signed-frame-matroids) |
| σ | a signing of a frame | [signed frame matroids](frame-bounds.md#signed-frame-matroids) |
| M | a matroid, in the primer | [matroids](primer.md#matroids) |
| M₀, M₁ | the parity matroids | [the parity matroids](parity-bound.md#the-parity-matroids) |
| A, B | the frame matroids of the even and of the odd frames | [signed frame matroids](frame-bounds.md#signed-frame-matroids) |
| N_h^σ | the signed frame matroid of frame h under σ | [signed frame matroids](frame-bounds.md#signed-frame-matroids) |
| w, λ | weights, and Lagrange multipliers | [weak duality](primer.md#weak-duality-and-lagrangian-relaxation) |
| π_{p,q}, c_r(S) | the exchange of the primes p and q, and the sum of S's cofactors of r | [the prime-exchange theorem](search-rules.md#the-prime-exchange-theorem) |
| lo(n), hi(n) | a lower and an upper bound on opt(n) | [bounds carried between games](search-rules.md#bounds-carried-between-games) |

"Greedy" never stands alone.  *Descending greedy* is the procedure that builds U\*, and *the greedy algorithm of a
matroid* is [the primer's](primer.md#the-greedy-algorithm-of-a-matroid).  In Taxman "greedy" usually means the strategy
that takes the largest legal move, which the theory does not use.

## The statements

Each result has a row, in the order of the files, with its label, its name linked to its statement, its gist and its
standing.  Observations, which are measured and have no label, have rows beside the results they concern.  The standings
are these:

* *proved*: the file proves it;
* *proved given* an open statement: the file proves it from that statement;
* *proved by computation*: a finite statement whose proof is a computation, named in its file or in evidence.md;
* *certified game by game*: an open statement that a computation proves in each game of a range;
* *measured*: observed on the data named in [evidence.md](evidence.md), with no proof;
* *open*: neither proved nor refuted; *false*: refuted by a counterexample.

[evidence.md](evidence.md) has the computations behind every standing but *proved*, and
[results/README.md](../../results/README.md#what-supports-the-scores) says how far the proofs of the committed chain
reach.

### Playability

| label | result | statement | standing |
|---|---|---|---|
| P1 | [the lifting lemma](playability.md#the-lifting-lemma) | a proper divisor of x in the pot lies below a maximal factor of x in the pot, so a move needs only a maximal factor | proved |
| P2 | [the playability theorem](playability.md#the-playability-theorem) | a set is playable iff it can pay each member with a distinct maximal factor outside it under an acyclic precedence; every topological order of the precedence is a game | proved |
| P3 | [the flat-cycle lemma](playability.md#the-flat-cycle-lemma) | when each member pays with a maximal factor outside the set, every cycle of the precedence is flat | proved |
| P4 | [the peel theorem](playability.md#the-peel-theorem) | an instance is playable iff a run of peel pairs every member, whatever the order of its moves | proved |
| P5 | [the residual theorem](playability.md#the-residual-theorem) | every dismantling ends at the same residual, the union of all the cores | proved |
| P6 | [the extension lemma](playability.md#the-extension-lemma) | a playable set of members stays playable with a member that lists a factor none of them lists | proved |
| P7 | [the core theorem](playability.md#the-core-theorem) | an instance is playable iff it has no core, iff its residual is empty; a set is playable iff its own instance's residual is empty | proved |
| P8 | [the exclusion theorem](playability.md#the-exclusion-theorem) | every minimal exclusion set lies in the residual, and on the board too for a separated set; {2, 4, 8} shows that separation is needed | proved |
| P9 | [the peel-order lemma](playability.md#the-peel-order-lemma) | in an order taken from a run of peel, the members' vectors are triangular with a nonzero diagonal, over every field | proved |
| P10 | [the independence lemma](playability.md#the-independence-lemma) | a playable set is independent with its own coordinates over every field | proved |

### Neighboring games

| label | result | statement | standing |
|---|---|---|---|
| N1 | [the prime lemma](neighbors.md#the-prime-lemma) | every optimal game plays q(n), first, and no other prime | proved |
| N2 | [the finished-game lemma](neighbors.md#finished-games) | every optimal game opens with q(n) and is over | proved |
| N3 | [monotonicity](neighbors.md#monotonicity) | a game of n − 1 is a game of n, so opt(n − 1) ≤ opt(n) | proved |
| N4 | [the ceiling lemma](neighbors.md#the-ceiling-lemma) | a game of n without the move n is a game of n − 1, so opt(n) ≤ opt(n − 1) + n | proved |
| N5 | [the prime formula](neighbors.md#the-prime-formula) | opt(p) = opt(p − 1) + p − q(p − 1) at a prime p, by exchanging the first move | proved |
| N6 | [the step theorem](neighbors.md#the-step-theorem) | at its ceiling, a game's optimal sets are those of n − 1 that stay playable with n added; at a prime, those of p − 1 with the first move exchanged | proved |

### The heaviest playable set of sources

| label | result | statement | standing |
|---|---|---|---|
| S1 | [the sources conjecture](sources.md#the-sources-conjecture) | no playable set of sources outweighs U\* | open; [certified game by game](evidence.md#the-open-statements-about-the-sources) |
| | [where exchange breaks](sources.md#where-exchange-breaks) | the playable sets of sources lack the exchange property: six sources of game 2873 have maximal playable sets of four and of five members; no failure in a smaller game is known | proved (by example) |
| S2 | [the victim law](sources.md#the-victim-law) | removing one member repairs a jam iff that member lies in every minimal core | proved |
| S3 | [the frontier theorem](sources.md#the-frontier-theorem) | a heaviest playable set of sources other than descending greedy's needs, at the largest member where the two differ, a jam whose residual has a member with three or more maximal factors; game 9170 shows that a single trade can then fail | proved |
| S4 | [the cardinality conjecture (★)](sources.md#the-cardinality-conjecture) | on every component of the sources and every truncation of one, descending greedy keeps a largest playable subset; it is equivalent to S1 for every weighting that increases with value | open, the equivalence proved; [certified game by game](evidence.md#the-open-statements-about-the-sources) |
| S5 | [the span conjecture (♦)](sources.md#the-span-conjecture) | every source that descending greedy rejects is, over GF(2), a sum of sources it kept above it | open; [certified game by game](evidence.md#the-open-statements-about-the-sources) |
| S6 | [the span theorem](sources.md#the-span-theorem) | (♦) in game n implies (★) and S1 there, for every weighting that increases with value | proved |
| | [the protection of the window law](sources.md#what-a-proof-must-use) | (♦) and (★) hold under every order of the real incidence structure that obeys the window law, random or adversarial, and under its reverse | measured |
| | [the component of 2873](sources.md#what-a-proof-must-use) | in the component that holds game 2873's six sources, descending greedy's six exclusions are the fewest possible | proved by computation |
| | [the monomial-impossibility criterion](sources.md#what-a-proof-must-use) | the exponent vectors of two products of primes p < q < r decide whether every choice of the primes orders them alike, as it puts p²q below pq² | measured |
| | [the 3-prime census](sources.md#what-a-proof-must-use) | every trap order of a shape of three primes, up to eight members, asks for an order no primes realize, was never realized, or is one of seven realizable orders | measured |
| | [the window guards](sources.md#what-a-proof-must-use) | each of the seven realizable trap orders was defeated, at every realization tested, by a source q³ or q²r that the shape forces into (n/2, n] | measured |

### Uniqueness and longest games

| label | result | statement | standing |
|---|---|---|---|
| Q1 | [the uniqueness theorem](uniqueness-and-longest-games.md#the-uniqueness-theorem) | every playable set of sources lies termwise below U\*, and U\* is the only heaviest | proved given (★) |
| Q2 | [the matroid proposition](uniqueness-and-longest-games.md#optimal-sets-on-the-whole-board) | in a game with two or more optimal sets, such as 425, every matroid in which the playable sets are independent has an independent set heavier than opt(n) | proved |
| | [the uniqueness of the optimal set to 424](uniqueness-and-longest-games.md#optimal-sets-on-the-whole-board) | every game n ≤ 424 has one optimal set, and game 425 has two; in every game enumerated, the optimal sets share the upper part U\* and their size, and differ by exchanges of equal sum below n/2 | measured to 424; false at 425 |
| Q3 | [the length lemma](uniqueness-and-longest-games.md#the-length-lemma) | ℓ(n) ≤ ν(n); ℓ(n − 1) ≤ ℓ(n) ≤ ℓ(n − 1) + 1, with ℓ(n) = ℓ(n − 1) at a prime; a playable set has at most \|U\*\| members above n/2 | proved; the last part given (★) |
| Q4 | [the length proposition](uniqueness-and-longest-games.md#the-length-proposition) | in game 833 every optimal game has 362 moves, and some legal game has 363 | proved by computation |
| | [the shortfall of longest games to 1300](uniqueness-and-longest-games.md#the-length-proposition) | in every game to 1300 the recorded optimal game is a longest game, but at 833, 834 and 835, where it is one move short | measured |

### The parity bound

| label | result | statement | standing |
|---|---|---|---|
| W1 | [the parity bound](parity-bound.md#the-parity-bound) | a playable set is independent in both parity matroids, so their heaviest common independent set bounds opt(n), with any picks and bars | proved |
| W2 | [the barred bound](parity-bound.md#the-barred-bound) | if the barred bound of u is at most V, every playable set that obeys F3 and weighs more than V holds u | proved |
| | [the exactness of the parity bound to 77](parity-bound.md#what-the-parity-bound-misses) | the parity bound equals opt(n) for every n ≤ 77, and first exceeds it at 78, with 1,992 against 1,991 | measured |

### Franklín and Moniot's paper

| label | result | statement | standing |
|---|---|---|---|
| C1 | [the bridge lemma](franklin-moniot.md#the-bridge-lemma) | a payment scheme has a flat alternating cycle iff its precedence is cyclic, iff another scheme has its payments | proved |
| C2 | [the counting theorem](franklin-moniot.md#the-counting-theorem) | a set has a payment scheme, is independent over GF(2), or is playable iff some set of factors receives at least one, an odd number, or exactly one of its schemes | proved |
| C3 | [the two-factor proposition](franklin-moniot.md#the-two-factor-proposition) | an instance whose members list at most two factors is playable iff its lists are independent over GF(2), iff its graph is a forest | proved |
| C4 | [the first-odd-core proposition](franklin-moniot.md#the-first-odd-core) | every set of {2, …, 44} independent over GF(2) is playable, so the parity bound is exact there; {12, 18, 20, 30, 45} is an odd core | proved |

### The frame model

| label | result | statement | standing |
|---|---|---|---|
| F1 | [the frame lemma](frame-model.md#the-frame-lemma) | every number lies in one frame, each frame lies in one layer, and the frames in front of it one layer down | proved |
| F2 | [the layer theorem](frame-model.md#the-layer-theorem) | a set is playable iff each of its layer instances is | proved |
| F3 | [the bottom-layers theorem](frame-model.md#the-bottom-layers-theorem) | every optimal set plays q(n) alone in layer 1 and exactly U\*_2 among the sources of layer 2, and its part in layer 2 is a forest | proved |
| | [the closed frames of layer 2](frame-model.md#the-bottom-layers-theorem) | of the boards 2 to 8191, only 80, all at most 186, have a frame of layer 2 with freedoms | measured |
| F4 | [clean numbering](frame-model.md#clean-numbering) | every frame with freedoms in front of frame h is numbered h − 1 | proved |
| F5 | [the reduction lemma](frame-model.md#the-reduction-lemma) | dropping a move with one live factor keeps winnability, and a winnable pair reduces to the same pair in any order | proved |
| F6 | [the promotion-limit lemma](frame-model.md#the-promotion-limit-lemma) | no completion of a node adds more members than the node's promotion limit | proved |

### The exactness of the frame model

| label | result | statement | standing |
|---|---|---|---|
| X1 | [the kept-sources lemma](exactness.md#the-exactness-theorem) | every pass of descending greedy in setup keeps U\* | proved |
| X2 | [the closed-frames lemma](exactness.md#the-exactness-theorem) | in a playable set that contains U\*, the factors of a closed frame pay exactly its moves | proved |
| X3 | [the one-frame-forward lemma](exactness.md#the-exactness-theorem) | in a playable U\* ∪ L, each member of L, a factor of a frame numbered h ≥ 1, pays in frame h − 1, and each source in its own frame | proved |
| X4 | [the candidate lemma](exactness.md#the-exactness-theorem) | if U\* ∪ L is playable, every member of L is a candidate | proved |
| X5 | [the claims lemma](exactness.md#the-exactness-theorem) | claims take no candidate, and are front and back moves of frame h with any set of candidates applied | proved |
| X6 | [the reduction-step lemma](exactness.md#the-exactness-theorem) | every candidate has a home, and root frame h with L applied is winnable iff frame h with L applied is playable | proved |
| X7 | [the exactness theorem](exactness.md#the-exactness-theorem) | U\* ∪ L is playable iff L is a promotion set of Π_n, and it then weighs ΣU\* + ΣL | proved |
| U | [statement U](exactness.md#statement-u) | some optimal set of n contains U\*_{≥3} | open; [certified game by game](evidence.md#statement-u) |
| X8 | [the corollary of U](exactness.md#statement-u) | ΣU\* + P\*(Π_n) ≤ opt(n), with equality iff U holds at n | proved |
| X9 | [the holes theorem](exactness.md#boards-with-holes) | on a board B with holes, U\*(B) ∪ L is playable iff L is a promotion set of Π(B) | proved |

### The extended problem

| label | result | statement | standing |
|---|---|---|---|
| E1 | [the extended exactness theorem](extended-problem.md#the-extended-exactness-theorem) | with the sources O ⊆ U\*_{≥3} left optional, a set that contains Φ is playable iff it gives a promotion set (K, L) of Π⁺(O) | proved |
| E2 | [the kept-claims lemma](extended-problem.md#the-kept-claims-lemma) | the claims that a frame keeps are front and back moves of frame h with any (K, L) applied, and take no member of L | proved |
| E3 | [the formulation corollary](extended-problem.md#the-kept-claims-lemma) | Π⁺(O) has the same promotion sets with the kept claims and without, and Π⁺(∅) with them is Π_n | proved |
| E4 | [the certificate theorem](extended-problem.md#the-certificate-theorem) | if the members of Φ in layers 3 and up are certified against a game's score V, and no promotion set of Π⁺(U\* ∖ Φ) is worth more than V − ΣΦ, then opt(n) = V | proved |
| E5 | [the same-problem corollary](extended-problem.md#the-certificate-theorem) | a game whose extended problem equals one already bounded, with a margin no smaller, is proven without a search | proved |

### Bounds on the promotions

| label | result | statement | standing |
|---|---|---|---|
| B1 | [the signed frame theorem](frame-bounds.md#the-signed-frame-theorem) | a promotion set is independent in every signed frame matroid, over every field | proved |
| B2 | [the frame bound](frame-bounds.md#the-frame-bound) | the heaviest set independent in both frame matroids, within the promotion limit, bounds P\* at the root and at every node | proved |
| | [the frame bound against the parity bound](frame-bounds.md#the-frame-bound) | the parity bound equals ΣU\* plus the frame bound at every root where both were measured | measured |
| B3 | [the frame bound for the extended problem](frame-bounds.md#the-frame-bound-for-the-extended-problem) | B2 holds for Π⁺(O), with its one-sided candidates added to both matroids | proved |
| B4 | [the GF(3) proposition](frame-bounds.md#the-gf3-proposition) | a frame winnable with nothing applied stays winnable with L applied iff L is independent under every signing over GF(3) | proved |
| B5 | [the decomposition bound](frame-bounds.md#the-decomposition-bound) | the frame matroids and signed frame matroids combine into a Lagrangian bound on P\*, valid for every multiplier | proved |

### The rules of a search

| label | result | statement | standing |
|---|---|---|---|
| R1 | [the state lemma](search-rules.md#the-state-lemma) | a node's completion is a promotion set iff its new members can be promoted from the node's state | proved |
| R2 | [the prime-exchange theorem](search-rules.md#the-prime-exchange-theorem) | an exchange of two primes that maps the root onto itself maps promotion sets to promotion sets, which orders the two primes' cofactor sums in every heaviest one | proved |
| R3 | [the exchange-pruning lemma](search-rules.md#pruning-by-prime-exchanges) | a search may end the nodes, and drop the candidates, that cannot keep a symmetric pair's inequality, and stay exact | proved |
| R4 | [the dual-bound lemma](search-rules.md#the-dual-bound-lemma) | a stopped search bounds P\* by its incumbent and the bounds of its open units | proved |
| R5 | [the floor lemma](search-rules.md#the-floor-lemma) | a search from a floor that completes with nothing found proves P\* at most the floor | proved |
| R6 | [the repeat lemma](search-rules.md#the-repeat-lemma) | games with equal root problems have the same promotion sets, and a promotion set optimal at m makes an optimal game of n | proved; parts 2 and 3 given U at n |
| R7 | [the near-repeat lemma](search-rules.md#the-near-repeat-lemma) | a node whose promotions leave, in a game m, frames no harder wherever its candidates reach has no completion worth more than P\*(Π_m) | proved |

## Open questions

Each question below is open, and its link leads to where the theory states it.

* [Statement U](exactness.md#statement-u): does some optimal set of every game n contain U\*_{≥3}?  The frame solver's
  scores rest on it wherever the prover has not reached.
* Does every optimal set contain all of U\*, as in every game whose optimal sets were enumerated ([optimal sets on the
  whole board](uniqueness-and-longest-games.md#optimal-sets-on-the-whole-board))?
* [The sources conjecture (S1)](sources.md#the-sources-conjecture): is U\* the heaviest playable set of sources in every
  game?
* [The cardinality conjecture (★) (S4)](sources.md#the-cardinality-conjecture): does descending greedy keep a largest
  playable subset of every component of the sources and every truncation of one?
* [The span conjecture (♦) (S5)](sources.md#the-span-conjecture): is every source that descending greedy rejects a sum,
  over GF(2), of sources it kept above it?  Without magnitudes, the target is (♦) for instances built multiplicatively
  from primes, under any order that obeys their window law ([what a proof must use](sources.md#what-a-proof-must-use)).
* Does exchange fail in a game smaller than 2873 ([where exchange breaks](sources.md#where-exchange-breaks))?
* Does a window guard always suffice to defeat a realizable trap order ([what a proof must
  use](sources.md#what-a-proof-must-use))?
* Is the gap between an optimal game and a longest one bounded ([the length
  proposition](uniqueness-and-longest-games.md#the-length-proposition))?  To 1300 it is at most one move.
* Is there a condition between "odd" and "exactly one" that is computable in polynomial time and exact for Taxman ([the
  hardness](franklin-moniot.md#the-hardness))?  By Franklín and Moniot's Theorem 2, none is exact on every graded poset
  unless P = NP.
* Which odd cores weigh in, from game 78 on ([the first odd core](franklin-moniot.md#the-first-odd-core))?
* What is the complexity of Taxman ([the hardness](franklin-moniot.md#the-hardness))?
