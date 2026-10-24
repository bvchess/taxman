# Uniqueness of the optimal set, and longest games

Part of [the theory](../THEORY.md).

Two questions about optimal games go beyond their scores.  Can two different sets of moves both be optimal?  Many
orders play one set, so the question is about sets.  And does an optimal game play as many moves as a game of its board
can?  On the sources alone, the heaviest playable set is unique wherever [the cardinality conjecture (★)
(S4)](sources.md#the-cardinality-conjecture) holds.  On the whole board it need not be: game 425 is the first with two
optimal sets.  And an optimal game need not be a longest one: in game 833 it plays one move fewer.

The words of the game are the [glossary's](../GLOSSARY.md#the-game).  U\*, [descending
greedy](sources.md#descending-greedy), components and truncations are defined in [sources.md](sources.md#definitions),
with [t, n] the integers from t to n.

## Uniqueness of the optimal set

### The uniqueness theorem

**Theorem Q1 (the uniqueness theorem).**  Suppose (★) holds in game n.  Then every playable set S of sources of game n
has, for every t, at most as many members of value t or more as U\* has:

    |S ∩ [t, n]| ≤ |U* ∩ [t, n]|.

So S has at most as many members as U\*, and, each listed in decreasing order, the i-th member of S is at most the i-th
member of U\*.  And for every weighting w of the sources that is positive and strictly increasing in value, the values
themselves among them, w(S) < w(U\*) unless S = U\*: U\* is the only heaviest playable set of sources.

The hypothesis holds wherever [the span conjecture](sources.md#the-span-conjecture) (♦) holds ([the span theorem
(S6)](sources.md#the-span-theorem)), which is checked game by game ([statements.md](statements.md#the-statements)).  In
game 21, U\* = {12, 14, 15, 16, 18, 19, 20, 21}, which sums to 135.  The set {12, 14, 15, 16, 17, 18, 20, 21}, U\* with
17 for 19, is playable; listed in decreasing order, its members are at or below those of U\*, and it sums to 133.

*Proof.*  Statement (★) bounds the count above every threshold, component by component, and the counts give the
weights.

1. *Counts.*  Fix t, and a component H of the sources.  The set S ∩ H ∩ [t, n] is playable, as part of S (a set of
   members of a playable instance is playable: [induced instances](playability.md#payment-schemes)).  So by (★) it has
   at most |U\* ∩ H ∩ [t, n]| members.  Summing over the components gives |S ∩ [t, n]| ≤ |U\* ∩ [t, n]|.
2. *The rest.*  The paragraph "Dominance" of [sums over thresholds](primer.md#sums-over-thresholds) turns these counts
   into the order of the members, and into w(S) ≤ w(U\*) for every positive, strictly increasing weighting w, with
   equality only if S = U\*.  ∎

This is the textbook fact that distinct weights give a matroid only one heaviest basis, transplanted.  The playable sets
of sources need not be a matroid's independent sets, as in game 2873 ([where exchange
breaks](sources.md#where-exchange-breaks)); in game 21 they are the forests of a graph ([the two-factor proposition
(P11)](playability.md#the-two-factor-proposition)).  They are always independent in the GF(2) matroid of their incidence
vectors ([the independence lemma (P10)](playability.md#the-independence-lemma)), and where (♦) holds U\* is that
matroid's greedy basis; the count form needs only (★).

### Optimal sets on the whole board

[The uniqueness theorem](#the-uniqueness-theorem) does not say that every optimal game plays U\* above n/2.  The upper
part of an optimal set is a playable set of sources, so given (★) it is U\* or lighter.  An optimal game with a lighter
upper part would have to win the difference back below n/2, where the factors that pay for the sources compete with the
moves up to n/2.  Whether that ever happens is open, and the enumerations below never find it.  Among the sources of
layers 1 and 2 it never happens: U\*_1 is {q(n)}, since descending greedy keeps the largest source prime, which lists
the factor 1 that no member kept before it lists, and rejects the other primes, which list only 1 too; and every optimal
set holds exactly U\*_1 and U\*_2 there ([the bottom-layers theorem (F4)](frame-model.md#the-bottom-layers-theorem)).
The weaker claim that some optimal set holds the rest of U\* is [statement U](exactness.md#statement-u), on which the
frame solver's scores rest.

On the whole board the optimal set need not be unique.  Game 425 has two optimal sets, which differ by an exchange of
three numbers up to n/2 of equal sum, {110, 132, 168} ↔ {98, 150, 162}, each summing to 410; both sets replay by the
rules and score opt(425) = 57,010.

#### The matroid proposition

A second optimal set rules out a matroid that is exact for the whole game.  A *matroid relaxation* of game n is a
matroid on the numbers 2 to n in which every playable set of game n is independent.  Its heaviest independent set, with
the values as weights, weighs at least opt(n), since the optimal sets are independent; the relaxation is *exact* when it
weighs exactly opt(n).

**Proposition Q2 (the matroid proposition).**  Let n be a game with two or more optimal sets, such as 425.  Then no
matroid relaxation of game n is exact: in every matroid on the numbers 2 to n in which every playable set of game n is
independent, some independent set weighs more than opt(n), with the values as weights.

*Proof.*  A matroid whose weights are positive and distinct has only one heaviest independent set, and every optimal
set would be a heaviest one.

1. *One heaviest independent set.*  In a matroid on 2, …, n, let G be the set that [the greedy algorithm of a
   matroid](primer.md#the-greedy-algorithm-of-a-matroid) keeps, taking the elements in decreasing order.  The values
   are positive and distinct, so by part (d) of the primer's paragraph "What the scan finds", G is the only heaviest
   independent set: every other independent set I has w(I) < w(G).
2. *Every optimal set would be it.*  Every optimal set is playable, so independent, and the heaviest independent set
   weighs at least opt(n).  If it weighed exactly opt(n), every optimal set would be a heaviest independent set, and so
   equal to G; but there are two.  ∎

So the certificate that settles the sources, a matroid whose greedy basis is U\*, has no analogue that is exact for the
whole game.

*The uniqueness of the optimal set to 424* (measured).  The prover enumerates the optimal sets of the games it has
proved.  The optimal set is unique in every game n ≤ 424; from 425 on a few games have two or three, and most have one.
In every game enumerated, the optimal sets all have the same upper part, U\*, and the same number of members, and they
differ by an exchange of equally many numbers up to n/2, of equal sum, each a maximal factor of a member of U\*.  At 685
the set recorded in [results/optimal.json](../../results/optimal.json) is the one of its three optimal sets that does
not stay playable with 686 ([the step theorem (N6)](neighbors.md#the-step-theorem)).
[solver/noframe/docs/uniqueness.md](../../solver/noframe/docs/uniqueness.md) has the method and the data, and
[evidence.md](evidence.md) the range.

## Longest games

Write ℓ(n) for the most moves of a game of n, the size of the largest playable subset of 2, …, n; a *longest game* of n
is one with ℓ(n) moves.  Write ν(n) for the matching number of the board's [maximal factor
graph](../GLOSSARY.md#the-game), which joins each number to its maximal factors.  A game can be over with fewer than
ℓ(n) moves for two reasons.  Its set need not be a maximal playable set: the game 9, 15, 21, 14, 18, 12, 20, 16 of 21 is
over, yet its set with 19 is playable, if 19 is played first ([the prime lemma (N1)](neighbors.md#the-prime-lemma)).
And maximal playable sets can have different sizes: in game 4, {2} and {3, 4} are both maximal ([the greedy algorithm of
a matroid](primer.md#the-greedy-algorithm-of-a-matroid)).

### The length lemma

**Lemma Q3 (the length lemma).**

* The matching number bounds the length: ℓ(n) ≤ ν(n).
* For n ≥ 2, ℓ(n − 1) ≤ ℓ(n) ≤ ℓ(n − 1) + 1, and ℓ(n) = ℓ(n − 1) for a prime n ≥ 3.
* If (★) holds in game n, a playable set of game n has at most |U\*| members above n/2.

The prime 2 is excluded since ℓ(2) = 1 and ℓ(1) = 0.  In game 21, |U\*| = 8: the four primes among the eleven sources
list only the factor 1, so a playable set holds at most one of them, and at most 8 sources in all.

*Proof.*  Each bound is a count form of a result already proved.

1. *The matching.*  [The playability theorem (P2)](playability.md#the-playability-theorem) pays the moves of a playable
   set with distinct maximal factors outside it.  Joining each move to the factor it is paid with gives as many disjoint
   edges of the maximal factor graph as the set has members.
2. *The neighbors.*  A game of n − 1 is a game of n, by [monotonicity (N3)](neighbors.md#monotonicity), so ℓ(n − 1) ≤
   ℓ(n).  A longest game of n with its move n deleted is a game of n − 1, by [the ceiling lemma
   (N4)](neighbors.md#the-ceiling-lemma), so ℓ(n − 1) ≥ ℓ(n) − 1.  For a prime n ≥ 3, a longest game of n that plays n
   plays it first, and the exchange of [the prime formula (N5)](neighbors.md#the-prime-formula) makes it a game of n − 1
   with as many moves; a longest game that does not play n is a game of n − 1.  So ℓ(n) ≤ ℓ(n − 1).
3. *The sources.*  The part above n/2 of a playable set is a playable set of sources, which the count inequality of [the
   uniqueness theorem (Q1)](#the-uniqueness-theorem), at t = ⌊n/2⌋ + 1, bounds by |U\*|.  ∎

So a legal game of ν(n) moves is a longest game, a certificate that costs one matching.  The bound can overshoot, since
moves that can be given distinct maximal factors outside their set need not be playable: {10, 14, 15, 21} of game 21
cannot be played together.  There the bound still holds with equality, ν(21) = ℓ(21) = 9.  It first overshoots in game
51, where ν(51) = 23 and ℓ(51) = 22: every maximum matching of the board of 51 gives a cyclic precedence
([solver/noframe/docs/longest-games.md](../../solver/noframe/docs/longest-games.md)).

### The length proposition

**Proposition Q4 (the length proposition).**  In game 833 = 7²·17 every optimal game has 362 moves, and ℓ(833) = 363.
A heaviest longest game scores 218,197, which is 29 less than opt(833) = 218,226.  One of them plays the optimal game's
moves above n/2, which are U\*, and differs from it below n/2 alone, by five lighter moves in place of four:

    optimal game:             250, 252, 294, 400          sum 1,196
    a heaviest longest game:  116, 180, 200, 320, 351     sum 1,167

*Proof (computational).*

1. The prover proves opt(833) = 218,226 and finds the optimal set unique, with 362 members, and with upper part U\*
   ([solver/noframe/docs/proofs.md](../../solver/noframe/docs/proofs.md),
   [uniqueness.md](../../solver/noframe/docs/uniqueness.md)).  So every optimal game has 362 moves.
2. The optimal set of [results/optimal.json](../../results/optimal.json), with 250, 252, 294 and 400 exchanged for 116,
   180, 200, 320 and 351, replays by the rules: a legal game of 363 moves, scoring 218,197.  Since ν(833) = 363, [the
   length lemma (Q3)](#the-length-lemma) gives ℓ(833) = 363.
3. [The prover](../GLOSSARY.md#programs-files-and-documents)'s branch and bound, weighing each move by its value plus a
   constant above every sum, so that a set's weight orders it by its number of moves and then by its sum, proves that
   no playable set of 363 members sums to more than 218,197.

Steps 1 and 3 rest on the prover's correctness, and the replay of step 2 on the rules alone.  The value ν(833) = 363 is
a maximum matching, which the prover computes by augmenting paths and anyone can recompute: the maximal factor graph is
bipartite, since a number and its maximal factor differ by one prime factor.  The method and the data are in
[solver/noframe/docs/longest-games.md](../../solver/noframe/docs/longest-games.md).  ∎

The mechanism is a trade below n/2.  Each of the nine exchanged numbers is a maximal factor of a member of U\*: 250 of
500 and 750, for instance.  A number played as a move is gone from the pot, so it cannot pay for a source.  The optimal
game plays the four heavy numbers and pays for those sources otherwise; a heaviest longest game leaves them to pay (in
one of its payment schemes, 250, 252, 294 and 400 pay for 500, 756, 588 and 800) and plays five lighter numbers instead,
one more move for 29 points less.

A search cannot cut the branches that cannot reach the most moves: in game 833 it would return 218,197.  In the terms
of [the frame model](frame-model.md#the-frame-model), the optimal game of 833 promotes 48 factors and a heaviest longest
game 49, so a search of the frame model cannot prune a promotion set for having fewer promotions than the freedoms
allow.

*How far an optimal game falls short* (measured).  In every game the prover has checked, the recorded optimal game of
[results/optimal.json](../../results/optimal.json) is a longest game, except in games 833, 834 and 835, where it is one
move short; [evidence.md](evidence.md) gives the range.  Whether the shortfall is bounded in general is open.

The sets of games 21, 425 and 833 above are replayed by the rules with `heuristics/core.py`, which also gives the
payments at 833.
