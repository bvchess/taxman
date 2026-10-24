# Uniqueness of the optimal set, and longest games

Part of [the theory](../THEORY.md).

Two questions about optimal games go beyond their scores.  Can two different sets of moves both be optimal?  Many
orders play one set, so the question is about sets.  And does an optimal game play as many moves as a game of its board
can?  On the sources alone, the heaviest playable set is unique wherever the cardinality conjecture (★) holds; on the
whole board it is not, from game 425 on.  An optimal game need not be a longest one: at game 833 it plays one move
fewer.

The words of the game are the [glossary's](../GLOSSARY.md#the-game), and U\*, descending greedy, components and
truncations are defined in [sources.md](sources.md#definitions).  ΣS is the sum of a set S, and [t, n] the integers
from t to n.

## Uniqueness of the optimal set

Two statements are kept apart: one about the sources on their own, which (★) settles, and one about the whole board,
where game 425 has two optimal sets.

### The uniqueness theorem

**Theorem Q1 (the uniqueness theorem).**  Suppose [the cardinality conjecture (★)
(S4)](sources.md#the-cardinality-conjecture) holds in game n.  Then every playable set S of sources of game n has, for
every t, at most as many members of value t or more as U\* has:

    |S ∩ [t, n]| ≤ |U* ∩ [t, n]|.

So S has at most as many members as U\*, and, each listed in decreasing order, the i-th member of S is at most the i-th
member of U\*.  And for every weighting w of the sources that is positive and strictly increasing in value, the values
themselves among them, w(S) < w(U\*) unless S = U\*: U\* is the only heaviest playable set of sources.

The hypothesis holds wherever the span conjecture (♦) holds ([the span theorem (S6)](sources.md#the-span-theorem)),
and (♦) is checked game by game ([statements.md](statements.md#the-statements)).

*Proof.*  Statement (★) bounds the count above every threshold, component by component.  Summing over the components
gives the count inequality, and summing the counts over the thresholds gives the weights.

1. *Counts.*  Fix t, and a component K of the sources.  The set S ∩ K ∩ [t, n] is playable, as part of S, so by (★)
   it has at most |U\* ∩ K ∩ [t, n]| members.  Summing over the components gives |S ∩ [t, n]| ≤ |U\* ∩ [t, n]|.
2. *Member by member.*  At the smallest source, t = ⌊n/2⌋ + 1, step 1 gives |S| ≤ |U\*|.  If the i-th member s of S
   exceeded the i-th member of U\*, the threshold t = s would hold i members of S and at most i − 1 of U\*.
3. *Weights.*  List the values of the sources c₁ > c₂ > … > c_m, and put w(c_{m+1}) = 0.  By the layer-cake identity
   ([sums over thresholds](primer.md#sums-over-thresholds)),

       w(S) = Σ_j (w(c_j) − w(c_{j+1})) · |S ∩ [c_j, n]|,

   since a member c_k is counted in the terms j ≥ k, whose coefficients sum to w(c_k).  Every coefficient is positive,
   since w is positive and strictly increasing, so step 1 gives w(S) ≤ w(U\*) term by term.
4. *Only one.*  Equality needs |S ∩ [c_j, n]| = |U\* ∩ [c_j, n]| for every j.  These counts determine a set: c_j is in
   it exactly when its count at c_j exceeds its count at c_{j−1}, or is 1 when j = 1.  So then S = U\*.  ∎

This is the textbook fact that distinct weights give a matroid only one heaviest basis, transplanted.  The playable sets
of sources are not a matroid's independent sets ([where exchange breaks](sources.md#where-exchange-breaks)).  They are
always independent in the GF(2) matroid of their incidence vectors ([the independence lemma
(P10)](playability.md#the-independence-lemma)), and where (♦) holds U\* is that matroid's greedy basis; the count form
needs only (★).

### Optimal sets on the whole board

The uniqueness theorem does not say that every optimal game plays U\* above n/2.  The upper part of an optimal set is a
playable set of sources, so given (★) it is U\* or lighter.  An optimal game with a lighter upper part would have to win
the difference back below n/2, where the factors that pay for the sources compete with the moves up to n/2.  Whether
that ever happens is open, and the enumerations below never find it.  Among the sources of layers 1 and 2 it never
happens: every optimal set holds exactly U\*_1 and U\*_2 there ([the bottom-layers
theorem (F3)](frame-model.md#the-bottom-layers-theorem)).  The weaker claim that some optimal set holds the rest of U\*
is [statement U](frame-model.md#statement-u), on which the frame solver's scores rest.

On the whole board the optimal set need not be unique.  Game 425 has two optimal sets, which differ by an exchange of
three numbers up to n/2 of equal sum, {110, 132, 168} ↔ {98, 150, 162}, each summing to 410; both sets replay by the
rules and score opt(425) = 57,010.  A second optimal set rules out an exact matroid relaxation of the whole game.

**Proposition Q2 (the matroid proposition).**  Let n be a game with two or more optimal sets, such as 425.  In every
matroid on the numbers 2 to n in which every playable set of game n is independent, some independent set weighs more
than opt(n), with the values as weights.  So no matroid relaxation of the whole game is exact.

*Proof.*  A matroid whose weights are positive and distinct has only one heaviest independent set, and every optimal
set would be a heaviest one.

1. *One heaviest independent set.*  In a matroid M on 2, …, n, let G be the set that [the greedy algorithm of a
   matroid](primer.md#the-greedy-algorithm-of-a-matroid) keeps, taking the elements in decreasing order.  Its part of
   value t or more is a maximal independent subset of the elements of value t or more: an element there that it rejects
   makes a dependent set with the elements kept before it, and so also with the larger set G ∩ [t, n].  In a
   matroid all the maximal independent subsets of a set have the same size, so every independent set I has |I ∩ [t,
   n]| ≤ |G ∩ [t, n]| for every t.  Steps 3 and 4 of the proof of the uniqueness theorem then give w(I) < w(G) unless I
   = G.
2. *Every optimal set would be it.*  Every optimal set is playable, so independent in M, and the heaviest independent
   set weighs at least opt(n).  If it weighed exactly opt(n), every optimal set would be a heaviest independent set, and
   so equal to G; but there are two.  ∎

So the certificate that settles the sources, a matroid whose greedy basis is U\*, has no analogue that is exact for the
whole game.

*The uniqueness of the optimal set to 424* (measured).  The prover enumerates the optimal sets of the games it has
proved.  The optimal set is unique in every game n ≤ 424, and from 425 on some games have two or three.  In every game
enumerated, the optimal sets all have the same upper part, U\*, and the same number of members, and they differ by an
exchange of equally many numbers up to n/2, of equal sum, each a maximal factor of a member of U\*.
[solver/noframe/docs/uniqueness.md](../../solver/noframe/docs/uniqueness.md) has the method and the data, and
[evidence.md](evidence.md) the range.  How the number of optimal sets moves from one game to the next is in [the step
theorem (N6)](neighbors.md#the-step-theorem).

## Longest games

An optimal game need not play as many moves as a game of its board can: at game 833 it plays one fewer.  The wiki's
"Questions" page asks whether it must, as its third question.  Write ℓ(n) for the most moves of a game of n, the size of
the largest playable subset of 2, …, n, and ν(n) for the matching number of the board's maximal-factor graph, which
joins each number to its maximal factors.  A game that is over can still have fewer than ℓ(n) moves, since the maximal
playable sets can have different sizes ([where exchange breaks](sources.md#where-exchange-breaks)).

### The length lemma

**Lemma Q3 (the length lemma).**

* ℓ(n) ≤ ν(n).
* For n ≥ 2, ℓ(n − 1) ≤ ℓ(n) ≤ ℓ(n − 1) + 1, and ℓ(n) = ℓ(n − 1) for a prime n ≥ 3.
* If (★) holds in game n, a playable set of game n has at most |U\*| members above n/2.

*Proof.*  Each bound is a count form of a result already proved.

1. *The matching.*  [The playability theorem (P2)](playability.md#the-playability-theorem) pays the moves of a playable
   set with distinct maximal factors outside it.  Joining each move to the factor it is paid with gives as many disjoint
   edges of the maximal-factor graph as the set has members.
2. *The neighbors.*  A game of n − 1 is a game of n, by [monotonicity (N3)](neighbors.md#monotonicity), so ℓ(n − 1) ≤
   ℓ(n).  A longest game of n without the move n is a game of n − 1, by [the ceiling lemma
   (N4)](neighbors.md#the-ceiling-lemma), so ℓ(n − 1) ≥ ℓ(n) − 1.  For a prime n ≥ 3, a longest game of n that plays n
   plays it first, and the exchange of [the prime formula (N5)](neighbors.md#the-prime-formula) makes it a game of n − 1
   with as many moves; a longest game that does not play n is a game of n − 1.  So ℓ(n) ≤ ℓ(n − 1).
3. *The sources.*  The part above n/2 of a playable set is a playable set of sources, which the count inequality of the
   uniqueness theorem (Q1), at t = ⌊n/2⌋ + 1, bounds by |U\*|.  ∎

So a legal game of ν(n) moves is a longest game, a certificate that costs one matching.  The bound can overshoot, since
moves that can be given distinct maximal factors outside their set need not be playable, as with the set that keeps game
21 below its ceiling ([the step theorem (N6)](neighbors.md#the-step-theorem)).  It overshoots first at game 51, where
ν(51) = 23 and ℓ(51) = 22.

### The length proposition

**Proposition Q4 (the length proposition).**  In game 833 = 7²·17 every optimal game has 362 moves, and some legal game
has 363.  The heaviest game of 363 moves scores 218,197, which is 29 less than opt(833) = 218,226.  It plays the optimal
game's moves above n/2, which are U\*, and differs from it below n/2 alone, by five lighter moves in place of four:

    optimal game:            250, 252, 294, 400          sum 1,196
    heaviest longest game:   116, 180, 200, 320, 351     sum 1,167

*Proof (computational).*

1. The prover proves opt(833) = 218,226 and finds the optimal set unique, with 362 members
   ([solver/noframe/docs/proofs.md](../../solver/noframe/docs/proofs.md),
   [uniqueness.md](../../solver/noframe/docs/uniqueness.md)).  So every optimal game has 362 moves.
2. The optimal set with 250, 252, 294 and 400 exchanged for 116, 180, 200, 320 and 351 replays by the rules, a legal
   game of 363 moves.  Since ν(833) = 363, the length lemma (Q3) gives ℓ(833) = 363.
3. The prover's branch and bound, weighing the move c as K + c with K above every sum, so that a set's weight orders it
   by its number of moves and then by its sum, proves that no playable set of 363 members sums to more than 218,197.

The method and the data are in [solver/noframe/docs/longest-games.md](../../solver/noframe/docs/longest-games.md): the
run of the games 2 to 1300 takes seven minutes on one thread, nearly all of it the searches at 833 to 835.  Steps 1 and
3 rest on the prover's correctness, and step 2 on the rules alone.  ∎

The mechanism is a trade of weight for moves below n/2.  Every payment is a maximal factor, so at most n/2.  A move
above n/2 uses one number up to n/2, its payment, and a move up to n/2 uses two, itself and its payment, so a playable
set S has |S| + |S ∩ [1, n/2]| ≤ ⌊n/2⌋.  In game 833 the optimal game's 48 moves up to n/2 and 362 payments use 410 of
the 416 numbers up to n/2, and the longest game's 49 and 363 use 412.  The optimal game leaves two more of those numbers
idle, and gives up a move for heavier moves up to n/2: 1,196 against 1,167.

A search may not cut the branches that cannot reach the most moves: at 833 it would return 218,197.  In the terms of
[the frame model](frame-model.md#the-frame-model), the optimal game of 833 promotes 48 factors and the heaviest longest
game 49, so a search of the frame model may not prune a promotion set for having fewer promotions than the freedoms
allow.

*How far an optimal game falls short* (measured).  In every game to 1300 the recorded optimal game is a longest game,
except at 833, 834 and 835, where it is one move short.  In all but six of those games its moves meet ν(n), the
certificate above; at 51, 182 and 183, ν(n) overshoots by one, and a search shows that ℓ(n) is the optimal game's
number of moves.  So to 1300 an optimal game is at most one move short of a longest one
([solver/noframe/docs/longest-games.md](../../solver/noframe/docs/longest-games.md)).  Whether the shortfall is bounded
in general is open.

The sets of games 425 and 833 above are replayed by the rules with `heuristics/core.py`.  The values of ν(n) and ℓ(n) at
51 and 833 are from the prover's run of longest-games.md, which settles ℓ(n) for every game to 1300.
