# The statements, and the definitions

Part of [the theory](../THEORY.md).

## The statements

Each statement, its standing, and the section that proves it or states it.  *Proved* means a proof below; *given* names
the open statement a proof assumes; *certified* means checked game by game by a computation that proves it for that
game; *measured* means observed on the data named, with no proof.  A few rows name their computation instead: *proved by
computation* is a finite computation that is the proof, *verified by exhaustive search* and *checked by machine* a
program's check of the cases given, and *false* a counterexample.  The [glossary](../GLOSSARY.md) defines the project's
words, and the section "Definitions" the notation.  [results/README.md](../../results/README.md) says how far the proofs
of the committed chain reach.

| label | statement | standing | section |
|---|---|---|---|
| Theorem 1 | a set of moves can be played iff its members have distinct maximal factors outside it whose precedence is acyclic | proved | [The decision procedure](decision-procedure.md#the-decision-procedure) |
| Theorem 2 | when peel succeeds, its payments have an acyclic precedence, so a play order exists | proved | [The decision procedure](decision-procedure.md#the-decision-procedure) |
| Theorem 3 | peel succeeds on every playable set | proved | [The decision procedure](decision-procedure.md#the-decision-procedure) |
| Theorem 4 | every dismantling ends at the same residual, the union of all self-covering subsets | proved | [Unplayable sets](decision-procedure.md#unplayable-sets-cores-and-the-residual) |
| the extension lemma | a playable set stays playable with a member that has a factor no member lists | proved (separated instances) | [Unplayable sets](decision-procedure.md#unplayable-sets-cores-and-the-residual) |
| Theorem 5 | a set is playable iff its residual is empty | proved for separated instances; measured without separation | [Unplayable sets](decision-procedure.md#unplayable-sets-cores-and-the-residual) |
| Theorem 6 | a set playable after excluding E is playable after excluding E's part in the residual | proved (separated instances) | [Unplayable sets](decision-procedure.md#unplayable-sets-cores-and-the-residual) |
| the need for separation | without separation, Theorem 6 and the extension lemma fail | proved (by example); measured | [Unplayable sets](decision-procedure.md#unplayable-sets-cores-and-the-residual) |
| the upper-half conjecture | no playable set of sources outweighs U\*, the set that descending greedy keeps | open; certified in every game to 100,000 (through (♦)) | [The open problem](open-problem.md) |
| the frontier argument | greedy is weight-optimal wherever the jam is hyperedge-free | proved | [The frontier argument](open-problem.md#the-frontier-argument-and-exactly-where-it-stops) |
| the victim law | a single sacrifice repairs a jam iff it lies in every minimal core | checked by machine | [The frontier argument](open-problem.md#the-frontier-argument-and-exactly-where-it-stops) |
| (★) | on every component of the sources' instance, and every truncation of one, descending greedy keeps a playable set of the largest size | open; certified in every game to 100,000 (through (♦)) | [The cardinality reformulation](open-problem.md#the-cardinality-reformulation) |
| the component of 2873 | greedy's 6 exclusions there are minimal | verified by exhaustive search | [The cardinality reformulation](open-problem.md#the-cardinality-reformulation) |
| the monomial-impossibility criterion | a trap ordering that demands an exponent-vector inversion is never realized | measured | [The cardinality reformulation](open-problem.md#the-cardinality-reformulation) |
| the 3-prime census | every realizable trap ordering of a 3-prime shape is defeated by a window guard | measured | [The cardinality reformulation](open-problem.md#the-cardinality-reformulation) |
| the independence lemma | the incidence vectors of a playable set are linearly independent over GF(2) | proved | [The span certificate](open-problem.md#the-span-certificate-) |
| Theorem 7 | (♦) implies (★) and the upper-half conjecture | proved | [The span certificate](open-problem.md#the-span-certificate-) |
| (♦) | every source that descending greedy rejects is a GF(2) combination of sources it kept above it | open; certified in every game to 100,000 and 902 games to 2,000,000 | [The span certificate](open-problem.md#the-span-certificate-) |
| the window law's protection | (♦) and (★) hold under every order obeying the window law, adversarial or random, and its reverse | measured | [The span certificate](open-problem.md#the-span-certificate-) |
| Theorem 8 | U\* is the unique heaviest playable set of sources, and every playable set of sources lies termwise below it | proved given (★), so certified to 100,000 | [Uniqueness](uniqueness-and-longest-games.md#uniqueness-of-the-optimal-set) |
| the lemma of the prime | every optimal set holds the largest prime ≤ n and no other prime | proved | [Uniqueness](uniqueness-and-longest-games.md#uniqueness-of-the-optimal-set) |
| Theorem 9 | the optimal sets of n follow from those of n − 1 at the ceiling and at a prime | proved | [Uniqueness](uniqueness-and-longest-games.md#uniqueness-of-the-optimal-set) |
| the remark on matroids | no matroid relaxation of the whole game is exact | proved (by game 425) | [Uniqueness](uniqueness-and-longest-games.md#uniqueness-of-the-optimal-set) |
| uniqueness of the optimal set | every game has one optimal set | measured to 424; false from 425 | [Uniqueness](uniqueness-and-longest-games.md#uniqueness-of-the-optimal-set) |
| the lemma on the most moves | M(n) ≤ ν(n), M(n − 1) ≤ M(n) ≤ M(n − 1) + 1, and at most \|U\*\| moves are sources | proved (the last given (★)) | [Longest games](uniqueness-and-longest-games.md#longest-games) |
| the proposition on longest games | an optimal game need not have the most moves | proved by computation (n = 833); the gap measured at most one move to 1300, open beyond | [Longest games](uniqueness-and-longest-games.md#longest-games) |
| Theorem 10 | a playable set is independent in both parity matroids, so their heaviest common independent set bounds opt(n) | proved | [The parity matroids](bounds.md#the-parity-matroids-a-bound-on-the-whole-game) |
| Theorem 11 | a promotable set is independent in the even and the odd frames' matroids, so their heaviest common independent set bounds P\* (also for the extended problem) | proved | [The frame matroids](bounds.md#the-frame-matroids-a-bound-on-the-promotions) |
| Theorem 13 | a prime exchange that maps the root onto itself orders the two primes' cofactor sums in every heaviest promotion set | proved | [Prime exchanges](bounds.md#prime-exchanges-symmetries-of-the-promotions) |
| Lemma Ω | the frames lie in layers, one frame in front of another one layer down | proved | [Frames are layers](frame-model.md#frames-are-layers) |
| Theorem L | a set is playable iff each of its layer instances is | proved | [Playability splits by layer](frame-model.md#playability-splits-by-layer) |
| Theorem B | every optimal set plays the largest prime alone in layer 1, and exactly U\*_2 among the sources of layer 2 | proved | [The bottom layers](frame-model.md#the-bottom-layers) |
| N | every frame with freedoms directly in front of frame h is frame h − 1 | proved (by construction) | [Numbering by layer](frame-model.md#numbering-by-layer) |
| M1 to M6 | each step of setup keeps the playable sets that contain U\* | proved | [The frame model is exact](frame-model.md#the-frame-model-is-exact-given-its-upper-part) |
| Theorem M | U\* ∪ L is playable iff L is a promotion set of the root problem | proved | [The frame model is exact](frame-model.md#the-frame-model-is-exact-given-its-upper-part) |
| U | some optimal set of n contains U\*_{≥3} | open; holds wherever the prover has proven opt(n), and at the games with a certificate | [The upper part](frame-model.md#the-upper-part) |
| the strong form of U | every optimal set contains U\* | measured on every game whose optimal sets were enumerated; open | [The upper part](frame-model.md#the-upper-part) |
| the prover's proofs | the recorded score of a game is opt(n) on the whole board | proved by computation, game by game | [The upper part](frame-model.md#the-upper-part) |
| the corollary of U | the frame solver's answer is opt(n) iff U holds at n | proved | [The upper part](frame-model.md#the-upper-part) |
| Theorem M⁺ | with a set O of sources left optional, a set is playable iff it is a promotion set of the extended problem Π⁺(O) | proved | [The extended problem](extended-problem.md) |
| Lemma M5⁺ | the claims of `unzip` that a frame keeps are moves of every instance, so Π⁺(O) has the same promotion sets with them and without | proved | [The extended problem](extended-problem.md) |
| the certificate of U | a search of Π⁺(O) that finds nothing better, with the other members certified, proves opt(n) | proved | [The extended problem](extended-problem.md) |
| Lemmas L1 to L4 | bounds pass between games: monotonicity, the ceiling, the prime formula both ways, repeats | proved (L4's inequality given U) | [The ledger](ledger.md) |
| Lemma D | a stopped search bounds P\* by its incumbent and the bounds of its open units | proved | [The ledger](ledger.md) |
| Lemma F | a search under a floor that completes with nothing found proves P\* at most the floor | proved | [The ledger](ledger.md) |
| Theorem 14 | a promotable set is independent in every signed frame matroid, over any field | proved | [Signed frame matroids](bounds.md#signed-frame-matroids-the-frame-bound-over-other-fields) |
| Theorem 15 | the GF(2) and signed matroids combine into one Lagrangian bound, valid for every multiplier | proved | [Signed frame matroids](bounds.md#signed-frame-matroids-the-frame-bound-over-other-fields) |
| the bridge | a payment scheme has a flat alternating cycle iff its precedence is cyclic iff another scheme has its payment set | proved | [Their Theorem 1 is Theorem 1](franklin-moniot.md#their-theorem-1-is-theorem-1) |
| Theorem 12 | playable, independent and matchable are one count read three ways: exactly one, odd, at least one | proved | [One count, three conditions](franklin-moniot.md#one-count-three-conditions) |
| the two-factor proposition | a set whose members have at most two maximal factors outside it is playable iff independent iff a forest | proved | [What the parity bound keeps](franklin-moniot.md#what-the-parity-bound-keeps-and-what-it-drops) |
| the first odd core | no game n ≤ 44 has an odd core, and game 45 does | proved | [What the parity bound keeps](franklin-moniot.md#what-the-parity-bound-keeps-and-what-it-drops) |
| the parity bound's exactness | the parity bound equals opt(n) for every n ≤ 77, and first exceeds it at 78 | measured | [What the parity bound keeps](franklin-moniot.md#what-the-parity-bound-keeps-and-what-it-drops) |

## Definitions

*The game.*  Game n is played on the numbers 1 to n.  A *move* takes a number that still has a proper divisor in the
pot; the player scores it, and the taxman takes its divisors still in the pot.  The best score of game n is opt(n), and
an *optimal set* is the set of moves of a game that scores it.  The most moves of a game of n is M(n).  A set of numbers
is *playable* when some game plays all of them as moves, in some order.

*Maximal factors.*  The *maximal factors* of c are the divisors f with c/f prime: the divisors reached by deleting one
prime from c.  Their set is mf(c): mf(4) = {2}, mf(6) = {2, 3}, and a prime's only maximal factor is 1.

*Layers.*  The number of prime factors of x, counted with multiplicity, is Ω(x): Ω(12) = 3, and Ω(1) = 0.  The *layer*
of x is Ω(x): layer 0 is {1}, layer 1 the primes, layer 2 the products pq and the squares p², and so on.  A maximal
factor of x lies one layer below x.  For a set S, S_k is its part in layer k, and S_{≥k} its part in the layers from k
up.

*Sources.*  The *sources* of game n are the numbers above n/2.  None divides another, so a source is never tax: it is a
move or nothing.

*Reduction instances.*  A *reduction instance* is a set of members C, a factor pool F disjoint from C, and each member's
factor list mf(c) ∩ F.  The function `core.peel` reduces such an instance by forced moves only: a member with one live
factor takes it, and a factor with one live lister goes to that lister.  The sources of game n form one naturally, with
F the union of their maximal factors: the *sources' instance*.

*U\* and the frame model.*  *Descending greedy* takes the members of an instance in decreasing order and keeps each one
whenever it and the members kept so far are still playable.  The set that it keeps among the sources of game n is U\*,
and U\*_k is its part in layer k.  The frame solver searches the promotion sets of its *root problem* Π_n (its frames,
candidates and `max_total`; [The frame model](frame-model.md)).  The score of the game
with no promotions is in_the_bag = ΣU\*, and the weight of the heaviest promotion set of a problem Π is P\*(Π); P\*
alone means P\*(Π_n).
