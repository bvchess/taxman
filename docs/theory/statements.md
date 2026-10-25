# The statements

Part of [the theory](../THEORY.md).

This file is the theory's reference: the notation that every file shares, every result with its standing and a link to
its proof, and the questions that are open.  The words of the game are the [glossary's](../GLOSSARY.md), and each of the
theory's own terms is defined in the one file that first needs it, which the glossary links.

## Notation

Each symbol below keeps one meaning across the theory, in every file and every proof.  Other letters, such as Y, Z, H,
t and ρ, are local: each file defines those it uses.

| symbol | meaning | defined in |
|---|---|---|
| n, m | games: game n, and another game m, usually one proven before | [the glossary](../GLOSSARY.md#the-game) |
| opt(n) | the best score of game n | [the glossary](../GLOSSARY.md#the-game) |
| S\*, S\*(n) | an optimal set: the set of moves of an optimal game, of game n where the game must be named.  A game can have several, so a statement about S\* says whether it holds for every optimal set or for some. | here |
| ΣS | the sum of the members of a set S | here |
| S_k, S_{≥k} | the members of a set S in layer k, and in layers k and up | here |
| ℓ(n) | the most moves of a game of n | [longest games](uniqueness-and-longest-games.md#longest-games) |
| ν(n), ν(S) | the matching number of the board's maximal-factor graph, or of a set's incidence graph | [longest games](uniqueness-and-longest-games.md#longest-games), [the primer](primer.md#matchings-permanents-and-determinants) |
| q(n) | the largest prime ≤ n; for a prime p, q(p − 1) is the largest prime below it | [neighbors.md](neighbors.md#neighboring-games) |
| p, q, r | primes, local to a statement; never a bare q where q(n) is in play | |
| Ω(x) | the number of prime factors of x, counted with multiplicity: its layer | [playability.md](playability.md#playability) |
| mf(x) | the maximal factors of x | [playability.md](playability.md#playability) |
| F, F_D, F_h | a pool of factors: an instance's pool, or the factors of a frame D or of frame h | [instances](playability.md#payment-schemes), [frames](frame-model.md#frames), [the frame model](frame-model.md#the-frame-model) |
| D | a frame, as setup forms it: a component of the frame graph | [frames](frame-model.md#frames) |
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
| the promotion limit | the most promotions that the freedoms and the candidates allow, written in words | [searching the frame model](frame-model.md#searching-the-frame-model) |
| V | a score of a game, or the value of a search's incumbent | [exact searches](search-rules.md#exact-searches), [the certificate theorem](extended-problem.md#the-certificate-theorem) |
| T_h | the candidates that touch frame h: those of home h and of home h + 1 | [signed frame matroids](frame-bounds.md#signed-frame-matroids) |
| σ | a signing of a frame | [signed frame matroids](frame-bounds.md#signed-frame-matroids) |
| M | a matroid, in the primer | [matroids](primer.md#matroids) |
| M₀, M₁ | the parity matroids | [the parity matroids](parity-bound.md#the-parity-matroids) |
| A, B | the frame matroids of the even and of the odd frames | [signed frame matroids](frame-bounds.md#signed-frame-matroids) |
| N_h^σ | the signed frame matroid of frame h under σ | [signed frame matroids](frame-bounds.md#signed-frame-matroids) |
| w, λ | weights, and multipliers | [weak duality](primer.md#weak-duality-and-lagrangian-relaxation) |
| π_{p,q}, c_r(S) | the exchange of the primes p and q, and the sum of S's cofactors of r | [the prime-exchange theorem](search-rules.md#the-prime-exchange-theorem) |
| lo(n), hi(n) | a lower and an upper bound on opt(n) | [bounds carried between games](search-rules.md#bounds-carried-between-games) |

## The statements

Each result has a row, in the reading order of the files, with its label, its name linked to its statement, its gist
and its standing.  Unlabeled rows, beside the results they concern, hold the observations, which are measured, and a few
facts shown by example or by a computation.  The standings are these, and the theory uses no others:

* *proved*: the file proves it; *explanatory* marks a proved result on which no other result or score rests;
* *proved given* an open statement: the file proves it from that statement;
* *proved by computation*: a finite statement whose proof is a computation, which its file or
  [evidence.md](evidence.md) names;
* *certified game by game*: an open statement that a computation proves in each game of a range;
* *measured*: observed on the data that [evidence.md](evidence.md) names, with no proof;
* *open*: neither proved nor refuted; *false*: refuted by a counterexample.

The theory has four open statements: [statement U](exactness.md#statement-u), [the sources conjecture
(S1)](sources.md#the-sources-conjecture), and the two that keep their symbols, (★)
([S4](sources.md#the-cardinality-conjecture)) and (♦) ([S5](sources.md#the-span-conjecture)).  evidence.md has the
computations behind every standing but *proved*, and
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
| P8 | [the exclusion theorem](playability.md#the-exclusion-theorem) | every minimal exclusion set lies in the residual, in every instance, and on the board for a separated set; {2, 4} on the board of 4 shows that separation is needed | proved |
| P9 | [the peel-order lemma](playability.md#the-peel-order-lemma) | in an order taken from a run of peel, the members' vectors are triangular with a nonzero diagonal, over every field | proved |
| P10 | [the independence lemma](playability.md#the-independence-lemma) | a playable set is independent with its own coordinates over every field | proved |
| P11 | [the two-factor proposition](playability.md#the-two-factor-proposition) | an instance whose members list at most two factors is playable iff its lists are independent over GF(2), iff its graph is a forest | proved |

### Neighboring games

| label | result | statement | standing |
|---|---|---|---|
| N1 | [the prime lemma](neighbors.md#the-prime-lemma) | every optimal game plays q(n), first, and no other prime | proved |
| N2 | [the finished-game lemma](neighbors.md#finished-games) | every optimal game opens with q(n) and is over | proved |
| N3 | [monotonicity](neighbors.md#monotonicity) | a game of n − 1 is a game of n, so opt(n − 1) ≤ opt(n) | proved |
| N4 | [the ceiling lemma](neighbors.md#the-ceiling-lemma) | a game of n without the move n is a game of n − 1, so opt(n) ≤ opt(n − 1) + n | proved |
| N5 | [the prime formula](neighbors.md#the-prime-formula) | opt(p) = opt(p − 1) + p − q(p − 1) at a prime p, by exchanging the first move | proved |
| N6 | [the step theorem](neighbors.md#the-step-theorem) | game n is at its ceiling iff some optimal set of n − 1 stays playable with n added, and its optimal sets are then exactly those; at a prime p, the optimal sets are those of p − 1 with q(p − 1) exchanged for p | proved |
| N7 | [the half lemma](neighbors.md#the-half-lemma) | at an even n, if some optimal game of n − 1 does not play n/2, game n is at its ceiling | proved |

### The heaviest playable set of sources

| label | result | statement | standing |
|---|---|---|---|
| S1 | [the sources conjecture](sources.md#the-sources-conjecture) | no playable set of sources outweighs U\* | open; [certified game by game](evidence.md#the-open-statements-about-the-sources) |
| | [where exchange breaks](sources.md#where-exchange-breaks) | the playable sets of sources can lack the exchange property: six sources of game 2873 have maximal playable sets of four and of five members | proved |
| | [the 48 trap orders](sources.md#where-exchange-breaks) | of the 720 orders of those six sources, exactly the 48 that put 1859 and 2197 last lead descending greedy to four members; the window law rules them all out | proved by computation |
| S2 | [the victim law](sources.md#the-victim-law) | in any instance, removing one member s of a set H leaves it playable iff s lies in every minimal core of H | proved; explanatory |
| S3 | [the frontier theorem](sources.md#the-frontier-theorem) | if T is a heaviest playable subset of a set H of sources, and not descending greedy's set, then at the largest member d where the two differ, (T, d) is a jam whose residual has a member with three or more maximal factors; the sources built from 17, 19 and 23, from game 8303, show such a jam | proved; explanatory |
| S4 | [the cardinality conjecture (★)](sources.md#the-cardinality-conjecture) | on every component of the sources and every truncation of one, descending greedy keeps a largest playable subset; it is equivalent to S1 for every positive, strictly increasing weighting | open, the equivalence proved; [certified game by game](evidence.md#the-open-statements-about-the-sources) |
| S5 | [the span conjecture (♦)](sources.md#the-span-conjecture) | every source that descending greedy rejects is, over GF(2), a sum of sources it kept above it | open; [certified game by game](evidence.md#the-open-statements-about-the-sources) |
| | [no rejection by an odd core](sources.md#the-span-conjecture) | in a game where (♦) holds, descending greedy rejects no source because of an odd core | certified game by game, with (♦) |
| S6 | [the span theorem](sources.md#the-span-theorem) | (♦) in game n implies (★) and S1 there, for every positive, strictly increasing weighting | proved |
| | [the component of 2873](sources.md#what-a-proof-must-use) | in the component that holds game 2873's six sources, descending greedy's six exclusions are the fewest possible | proved by computation: the check of (♦) at 2873, with S6 |
| | [the matching bound at most thresholds](sources.md#what-a-proof-must-use) | at most thresholds, descending greedy keeps as many sources as the matching bound allows, so (★) holds there with no search | measured |
| | [the monomial-impossibility criterion](sources.md#what-a-proof-must-use) | two products of primes p < q < r are ordered alike by every choice of the primes exactly when their exponents meet three inequalities, as p²q and pq² are | sufficiency proved; necessity measured |
| | [the 3-prime census](sources.md#what-a-proof-must-use) | of the 428 shapes censused, every trap order asks for an order no primes realize, was never realized, or is one of seven realizable orders | measured |
| | [the window guards](sources.md#what-a-proof-must-use) | each of the seven realizable trap orders was defeated, at every realization tested, by a source q³ or q²r that the shape forces into (n/2, n] | measured |
| | [the protection of the window law](sources.md#what-a-proof-must-use) | on every instance tested, (♦) and (★) held under every order of the real incidence structure that obeys the window law, random or adversarial, and under its reverse | measured |
| | [the window law for the primes that make the window](sources.md#what-a-proof-must-use) | (♦) failed often under the window law for a random ranking of the primes, or for primes perturbed by 20% or more, and never when the members were rebuilt from the perturbed primes | measured |

### Uniqueness and longest games

| label | result | statement | standing |
|---|---|---|---|
| Q1 | [the uniqueness theorem](uniqueness-and-longest-games.md#the-uniqueness-theorem) | in a game where (★) holds, every playable set of sources lies termwise below U\*, and for every positive, strictly increasing weighting U\* is the only heaviest | proved given (★) |
| Q2 | [the matroid proposition](uniqueness-and-longest-games.md#the-matroid-proposition) | a matroid relaxation of game n, a matroid on 2..n in which every playable set is independent, is exact when its heaviest independent set weighs opt(n); in a game with two or more optimal sets, such as 425, none is | proved |
| | [the uniqueness of the optimal set to 424](uniqueness-and-longest-games.md#the-matroid-proposition) | every game n ≤ 424 has one optimal set, and game 425 has two; in every game enumerated, the optimal sets share their size and differ by exchanges of equal sum below n/2 | measured to 424; false at 425 |
| | [the strong form of U](uniqueness-and-longest-games.md#optimal-sets-on-the-whole-board) | every optimal set contains all of U\* | open; measured on every game whose optimal sets were all enumerated |
| Q3 | [the length lemma](uniqueness-and-longest-games.md#the-length-lemma) | ℓ(n) ≤ ν(n); ℓ(n − 1) ≤ ℓ(n) ≤ ℓ(n − 1) + 1, with ℓ(n) = ℓ(n − 1) at a prime n ≥ 3; a playable set has at most \|U\*\| members above n/2 | proved; the last part given (★) |
| Q4 | [the length proposition](uniqueness-and-longest-games.md#the-length-proposition) | in game 833 every optimal game has 362 moves and ℓ(833) = 363, and a heaviest longest game scores 29 less than opt(833) | proved by computation |
| | [how far an optimal game falls short](uniqueness-and-longest-games.md#the-length-proposition) | in every game the prover has checked, the recorded optimal game is a longest game, but in games 833, 834 and 835, where it is one move short | measured |

### Frames and the frame model

| label | result | statement | standing |
|---|---|---|---|
| F1 | [the frame lemma](frame-model.md#the-frame-lemma) | every number lies in one frame, each frame lies in one layer, and the frames in front of it one layer down | proved |
| F2 | [the joining lemma](frame-model.md#the-joining-lemma) | acyclic schemes of disjoint sets against disjoint pools join into an acyclic scheme when each set lists only factors of its own pool and the pools before it | proved |
| F3 | [the layer theorem](frame-model.md#the-layer-theorem) | a set is playable iff each of its layer instances is | proved |
| F4 | [the bottom-layers theorem](frame-model.md#the-bottom-layers-theorem) | every optimal set plays q(n) alone in layer 1, exactly U\*_2 among the sources of layer 2, and a forest in layer 2; where the frames of layer 2 are closed, as they are from n = 187 on ([F10](frame-model.md#the-square-theorem)), it plays no number of layer 2 up to n/2 | proved |
| F5 | [the winnability lemma](frame-model.md#the-winnability-lemma) | a pair of Taxman Mini is winnable iff, read as an instance, it is playable, so it has at least as many factors as moves | proved |
| F6 | [clean numbering](frame-model.md#clean-numbering) | every frame with freedoms in front of frame h is numbered h − 1 | proved |
| F7 | [the take-on lemma](frame-model.md#the-take-on-lemma) | frame h can take a number on iff one of its frames can | proved |
| F8 | [the reduction lemma](frame-model.md#the-reduction-lemma) | dropping a move with one live factor keeps winnability, and a winnable pair reduces to the same pair in any order | proved |
| F9 | [the promotion-limit lemma](frame-model.md#the-promotion-limit-lemma) | filling each frame's promotions from the front reaches the promotion limit, which the freedoms and the candidates set; no completion adds more members than its node's limit, and a candidate is in some completion only if both frames it touches have freedoms | proved |
| F10 | [the square theorem](frame-model.md#the-square-theorem) | a frame of layer 2 has one freedom if none of its sources is the square of a prime, and none if one is; from n = 187 on, layer 2 is one frame with a square, so every optimal set plays exactly U\*_2 in layer 2 and no number of layer 2 up to n/2, and k₀ ≥ 3 | proved, the boards 187 to 2499 and the primes in short intervals below 396,738 by computation |

### The exactness of the frame model

| label | result | statement | standing |
|---|---|---|---|
| X1 | [the kept-sources lemma](exactness.md#the-exactness-theorem) | descending greedy over a frame's sources keeps U\* ∩ D, and after any claims it keeps every source left, so setup plays U\* when it promotes nothing | proved |
| X2 | [the closed-frames lemma](exactness.md#the-exactness-theorem) | in a playable set that contains U\*, the factors of a closed frame pay exactly its moves | proved |
| X3 | [the one-frame-forward lemma](exactness.md#the-exactness-theorem) | in a playable U\* ∪ L, each member of L, a factor of a frame numbered h ≥ 1, pays in frame h − 1, and each source in its own frame | proved |
| X4 | [the candidate lemma](exactness.md#the-exactness-theorem) | if U\* ∪ L is playable, every member of L is a candidate | proved |
| X5 | [the claims lemma](exactness.md#the-exactness-theorem) | claims take no candidate; frame h's claims are front and back moves of frame h with any set of candidates applied; and a factor that passes test (b) passes test (a) iff no claim takes it and the claims' remainder without it is playable | proved |
| X6 | [the reduction-step lemma](exactness.md#the-exactness-theorem) | in every root problem of game n, every candidate has a home, and root frame h with L applied is winnable iff frame h with L applied is playable | proved |
| X7 | [the exactness theorem](exactness.md#the-exactness-theorem) | in every root problem of game n, U\* ∪ L is playable iff L is a promotion set, and it then weighs ΣU\* + ΣL | proved |
| U | [statement U](exactness.md#statement-u) | some optimal set of n contains U\*_{≥3} | open; [certified game by game](evidence.md#statement-u) |
| X8 | [the corollary of U](exactness.md#statement-u) | ΣU\* + P\*(Π_n) ≤ opt(n), with equality iff U holds at n | proved |
| X9 | [the holes theorem](exactness.md#boards-with-holes) | on a board Y with holes, U\*(Y) ∪ L is playable iff L is a promotion set of a root problem of Y | proved |

### The parity bound

| label | result | statement | standing |
|---|---|---|---|
| W1 | [the parity bound](parity-bound.md#the-parity-bound) | a playable set is independent in both parity matroids, so the heaviest set independent in both bounds opt(n), and with picks and bars the playable sets that hold the picks and avoid the bars | proved |
| W2 | [the barred bound](parity-bound.md#the-barred-bound) | for a number b that is not a pick of [F4](frame-model.md#the-bottom-layers-theorem), if b's barred bound is at most V, every playable set that obeys F4 and weighs more than V holds b | proved |
| W3 | [the greedy-basis proposition](parity-bound.md#the-greedy-basis-proposition) | every heaviest set independent in both parity matroids holds, of the sources, the greedy basis of their GF(2) linear matroid, which is U\* iff (♦) holds in game n; then the parity bound equals the bound with the picks U\* | proved |
| | [the exactness of the parity bound to 77](parity-bound.md#what-the-parity-bound-misses) | the parity bound equals opt(n) for every n ≤ 77, and exceeds it at 78 | measured |

### Franklín and Moniot's paper

| label | result | statement | standing |
|---|---|---|---|
| C1 | [the bridge lemma](franklin-moniot.md#the-bridge-lemma) | a payment scheme has a flat alternating cycle iff its precedence is cyclic, iff another scheme has its payments | proved; explanatory |
| C2 | [the counting theorem](franklin-moniot.md#the-counting-theorem) | a set has a payment scheme, is independent with its own coordinates over GF(2), or is playable iff some set of factors receives at least one, an odd number, or exactly one of its schemes | proved; explanatory |
| C3 | [the first-odd-core proposition](franklin-moniot.md#the-first-odd-core) | every set of {2, …, 44} independent with its own coordinates over GF(2) is playable, so the parity bound is exact there; {12, 18, 20, 30, 45} is an odd core | proved; explanatory |

### The extended problem and the certificates of U

| label | result | statement | standing |
|---|---|---|---|
| E1 | [the extended exactness theorem](extended-problem.md#the-extended-exactness-theorem) | with any set O ⊆ U\* of sources left optional, a set that contains Φ is playable iff it gives a promotion set (K, L) of Π⁺(O) | proved |
| | [the first rejection in layer 3](extended-problem.md#the-extended-problem) | descending greedy rejects no source of layer 4 or more on the boards checked, and none of layer 3 before game 741, where it rejects 374 = 2·11·17 | measured |
| E2 | [the certificate theorem](extended-problem.md#the-certificate-theorem) | if the members of Φ in layers 3 and up are certified against a game's score V, and no promotion set of Π⁺(U\* ∖ Φ) is worth more than V − ΣΦ, then opt(n) = V; and U holds at n when V = ΣU\* + P\*(Π_n) | proved |
| E3 | [the ceiling-stop corollary](extended-problem.md#the-certificate-theorem) | if a promotion set of Π⁺(U\* ∖ Φ) is worth n + V(n − 1) − ΣΦ, for the score V(n − 1) of a game of n − 1 that is optimal, then opt(n) = n + V(n − 1) | proved |
| E4 | [the kept-claims lemma](extended-problem.md#the-kept-claims-lemma) | the claims that a frame keeps are front and back moves of frame h with any (K, L) applied, and take no member of L | proved |
| E5 | [the formulation corollary](extended-problem.md#the-kept-claims-lemma) | Π⁺(O) has the same promotion sets with the kept claims and without, and Π⁺(∅) with the claims of a root problem of game n is that root problem | proved |
| E6 | [the same-problem corollary](extended-problem.md#the-same-problem-corollary) | a game whose extended problem equals another game's, already bounded with a margin no larger than its own, is proven without a search | proved |

### Bounds on the promotions

| label | result | statement | standing |
|---|---|---|---|
| B1 | [the signed frame theorem](frame-bounds.md#the-signed-frame-theorem) | a promotion set is independent in every signed frame matroid, over every field; a loop lies in no promotion set; at a node, the contraction by P is the matroid of state(P), in which a candidate taken out of its home is a loop | proved |
| B2 | [the frame bound](frame-bounds.md#the-frame-bound) | the heaviest set independent in both frame matroids, within the node's promotion limit, bounds every completion of a node, the root included | proved |
| | [the frame bound against the parity bound](frame-bounds.md#the-frame-bound) | the parity bound equals ΣU\* plus the frame bound at every board 2..8191, including the 5,496 boards from 1173 on where the last part of [B6](frame-bounds.md#the-contraction-theorem) does not apply | measured |
| | [the frame bound's gap](franklin-moniot.md#the-hardness) | at the roots measured, the heaviest set of the frame bound fails only through odd cores, in the frames numbered 1 to 3 | measured |
| B3 | [the frame bound for the extended problem](frame-bounds.md#the-frame-bound-for-the-extended-problem) | B2 holds for Π⁺(O), with its one-sided candidates added to both matroids, and with the promotion limit of the freedoms that the node's promotions leave | proved |
| B4 | [the decomposition bound](frame-bounds.md#the-decomposition-bound) | the frame matroids and signed frame matroids combine into a Lagrangian bound on P\*, valid for every multiplier | proved |
| B5 | [the GF(3) proposition](frame-bounds.md#the-gf3-proposition) | a frame of a state is winnable with L applied iff L ∩ T_h is independent under every signing over GF(3) | proved; explanatory |
| B6 | [the contraction theorem](frame-bounds.md#the-contraction-theorem) | on the candidates the frame matroids are the parity matroids contracted by U\*; the promotion limit never lowers the frame bound at the root; ΣU\* plus the frame bound is the parity bound with the picks U\* and the bars outside U\* ∪ C, so at most the parity bound; given (♦), equal to it when every number up to n/2 that is a loop of neither contraction is a candidate | proved, the last part given (♦); explanatory |

### The rules of a search

| label | result | statement | standing |
|---|---|---|---|
| R1 | [the state lemma](search-rules.md#the-state-lemma) | a node's completion is a promotion set iff its new members are factors of their homes in the node's state and every frame of the state stays winnable with them applied; the same in Π⁺(O) | proved |
| R2 | [the prime-exchange theorem](search-rules.md#the-prime-exchange-theorem) | an exchange of primes p < q that maps the root onto itself maps promotion sets to promotion sets; if no candidate is divisible by pq, p² or q², every heaviest promotion set has c_p ≤ c_q | proved |
| R3 | [the exchange-pruning lemma](search-rules.md#pruning-by-prime-exchanges) | an exact search stays exact when it ends a node, or drops a candidate, that cannot keep a symmetric pair's inequality | proved |
| R4 | [the dual-bound lemma](search-rules.md#the-dual-bound-lemma) | in an exact search, every promotion set heavier than the incumbent that keeps every symmetric pair's inequality is a completion of an open unit, so P\* is at most the dual bound; a completed search proves P\* at most the incumbent, with equality when a promotion set is behind it | proved |
| R5 | [the floor lemma](search-rules.md#the-floor-lemma) | an exact search from a floor that completes having found no promotion set above it proves P\* at most the floor | proved |
| R6 | [the repeat lemma](search-rules.md#the-repeat-lemma) | for games with Π_n = Π_m, every promotion set L makes a game of n worth ΣU\*(n) + ΣL; opt(n) − ΣU\*(n) ≤ opt(m) − ΣU\*(m); and a promotion set that makes an optimal game of m makes one of n | proved; the inequality and the last part given U at n |
| R7 | [the near-repeat lemma](search-rules.md#the-near-repeat-lemma) | under three conditions against a game m, every completion of a node is a promotion set of Π_m, so worth at most a bound Z of P\*(Π_m), and an exact search may end the node | proved; an answer that uses it rests on what Z rests on |

R3 to R5 and R7 are about exact searches.  They apply to the frame solver given that it searches exactly, which is a
claim about the program ([exact searches](search-rules.md#exact-searches)).

### The arrival of n

| label | result | statement | standing |
|---|---|---|---|
| J1 | [the arrival lemma](arrival.md#the-arrival-lemma) | if n joins only closed frames, and they stay closed, game n poses the root problem of game n − 1, so opt(n) = opt(n − 1) + ΣU\*(n) − ΣU\*(n − 1) given U: n − x for odd n, given (♦), and n for even n | proved; the value of x given (♦) |
| J2 | [the layer-2 corollary](arrival.md#the-layer-2-corollary) | a number with two prime factors that joins only closed frames leaves them closed, with no conjecture; every such n ≥ 188 does | proved |
| | [the shadow law](arrival.md#the-shadow-law) | when the recorded optimal sets of n − 1 and n have the same promotion set and differ by n replacing one move x, x is the shadow of n for odd n and n/2 for even n; and for odd n, when game n poses the root problem of game n − 1, U\* gives up exactly the shadow of n | measured; proved at a prime, by [N6](neighbors.md#the-step-theorem) |
| | [the half law](arrival.md#the-half-law) | for even n ≥ 4 but 14, game n poses the root problem of game n − 1 iff n/2 ∉ U\*(n − 1) | measured; the "if" proved where J1 applies |
| | [the cofactor rule](arrival.md#the-cofactor-rule) | for n ≥ 28 whose layer 2 has no freedom in game n or in the game m before it (n − 2 when n − 1 is prime), as for every n ≥ 188 by [F10](frame-model.md#the-square-theorem): game n has the root problem of game m if n divided by its largest prime factor is a prime or twice a prime, and is at its ceiling if it is 9 | measured; the first case proved, by J2 |

## Open questions

These are the four open statements and the questions that the theory poses without a statement.  Each link leads to
where the theory states it.

* [Statement U](exactness.md#statement-u): does some optimal set of every game n contain U\*_{≥3}?  The frame solver's
  scores rest on it wherever no other proof has reached.
* [The sources conjecture (S1)](sources.md#the-sources-conjecture): is U\* the heaviest playable set of sources in every
  game?
* [The cardinality conjecture (★) (S4)](sources.md#the-cardinality-conjecture): does descending greedy keep a largest
  playable subset of every component of the sources and every truncation of one?
* [The span conjecture (♦) (S5)](sources.md#the-span-conjecture): is every source that descending greedy rejects a sum,
  over GF(2), of sources it kept above it?  Without magnitudes, the target is (♦) for every instance built from an
  increasing list of numbers standing in for the primes, under any order that obeys its window law ([what a proof must
  use](sources.md#what-a-proof-must-use)).
* Does every optimal set contain all of U\*, the strong form of U ([optimal sets on the whole
  board](uniqueness-and-longest-games.md#optimal-sets-on-the-whole-board))?
* Does exchange fail among the sources of a game smaller than 2873 ([where exchange
  breaks](sources.md#where-exchange-breaks))?
* Does a window guard always defeat a realizable trap order ([what a proof must use](sources.md#what-a-proof-must-use))?
* Is the shortfall of an optimal game from a longest one bounded ([the length
  proposition](uniqueness-and-longest-games.md#the-length-proposition))?
* Is there a condition between "odd" and "exactly one" that is computable in polynomial time and exact for Taxman ([the
  hardness](franklin-moniot.md#the-hardness))?  By Franklín and Moniot's Theorem 2, none is exact on every graded poset
  unless P = NP.
* Which odd cores weigh in, from game 78 on ([the first odd core](franklin-moniot.md#the-first-odd-core))?
* What is the complexity of Taxman ([the hardness](franklin-moniot.md#the-hardness))?
