# The heaviest playable set of sources

Part of [the theory](../THEORY.md).

The [sources](../GLOSSARY.md#the-frame-model) of game n, its numbers above n/2, are each a move or nothing, and the
frame model fixes them to U\*: the set that descending greedy keeps, taking the sources from the largest down and
keeping each one that still fits with those kept before it.  This file asks whether U\* is the heaviest playable set of
sources.  That is the sources conjecture, and it is open.

## What is open

Three statements are open, and each is checked game by game ([statements.md](statements.md#the-statements) says how
far):

* **The sources conjecture (S1):** no playable set of sources outweighs U\*.
* **The cardinality conjecture (★) (S4):** the same with sizes in place of weights, on every component of the sources
  and every truncation of one.  It is equivalent to the sources conjecture holding for every weighting that increases
  with value, and where it holds U\* is the only heaviest playable set of sources.
* **The span conjecture (♦) (S5):** every source that descending greedy rejects is a sum, over GF(2), of sources it
  kept above it.  By the span theorem (S6) it implies (★), and it is the statement that the computations check.

A proof of (★) for every game would settle the sources conjecture, and a proof of (♦) for every game would prove (★).
A refutation would be a game with a playable set of sources heavier than U\*, or, for (★), a component and a threshold
where descending greedy keeps fewer sources than fit.  A failure of (♦) alone would refute neither, since (♦) is the
stronger statement.  The uniqueness theorem ([Q1](uniqueness-and-longest-games.md#the-uniqueness-theorem)) and part of
the length lemma ([Q3](uniqueness-and-longest-games.md#the-length-lemma)) assume (★).  The scores of the frame solver
do not: they rest on [statement U](exactness.md#statement-u).

The file follows the ways a proof has been tried.  The textbook proof that a procedure like descending greedy finds a
heaviest set needs the exchange property of a matroid, which playable sets lack (where exchange breaks).  A direct
proof, which compares U\* with a heavier set at their largest disagreement, works where the obstruction there is made of
members with at most two maximal factors (the victim law and the frontier theorem), and stops at game 9170.  Dropping
the weights turns the conjecture into (★), and linear algebra over GF(2) gives (♦), which proves (★) game by game.  The
last section lists what the measurements say a proof must use.

## Definitions

### The sources' instance

The sources' instance of game n is the [reduction instance](playability.md#payment-schemes) whose members are the
sources, whose pool is the set of their maximal factors, and in which each member lists all its maximal factors.  A set
of sources is playable in game n exactly when it is playable in this instance ([the playability theorem
(P2)](playability.md#the-playability-theorem)).  So it is playable exactly when it holds no
[core](playability.md#cores-and-the-residual), a nonempty set in which every factor a member lists is listed by another
member ([the core theorem (P7)](playability.md#the-core-theorem)).  In game 21 the sources are 11 to 21, and the pool is
1 to 10:

    11: 1        14: 7, 2     17: 1        20: 10, 4
    12: 6, 4     15: 5, 3     18: 9, 6     21: 7, 3
    13: 1        16: 8        19: 1

### Descending greedy

*Descending greedy* takes the members of an instance in decreasing order, and keeps each one when it and the members
kept so far are still playable.  The set it keeps on the sources' instance of game n is U\*, and U\*_k is its part in
layer k.  In game 21 it keeps 21, 20, 19 and 18; it rejects 17, since 17 and 19 list only the factor 1, which makes {17,
19} a core; it keeps 16, 15 and 14, rejects 13, keeps 12 and rejects 11.  So U\* = {12, 14, 15, 16, 18, 19, 20, 21},
which sums to 135; the optimal game of 21 plays U\* and the factor 9, for 144.

### Components and truncations

The *incidence graph* of an instance joins each member to the factors it lists, and a *component* is the set of members
of one of its connected pieces.  Game 21's sources have four: {11, 13, 17, 19} on the factor 1, {12, 18, 20} on 4, 6, 9
and 10, {14, 15, 21} on 2, 3, 5 and 7, and {16} on 8.  The *truncation* of a set K at t is K ∩ [t, n], its members of
value t or more, where [t, n] is the integers from t to n.  Two facts follow from the core theorem.

* *Playability splits over the components.*  A set of sources is playable exactly when its part in each component is.
  A member's factors are listed only by members of its own component, so the part of a core in a component is a core
  when it is not empty.  So a set holds a core exactly when its part in some component does.
* *So does descending greedy.*  Run on a component K, descending greedy keeps U\* ∩ K, and run on a truncation K ∩ [t,
  n] it keeps U\* ∩ K ∩ [t, n].  When it reaches a member x of K, the members kept so far are playable, so by the first
  fact they stay playable with x exactly when their part in K does.  Its decision on x therefore depends only on the
  members of K kept above x, as it does in the run on K, and stopping at t changes nothing above t.

### Jams and victims

A *jam* is a set T ∪ {d} that is not playable, with T playable and d ∉ T.  A *victim* of the jam is a member t of T
whose removal repairs it: (T ∪ {d}) ∖ {t} is playable.

## The sources conjecture

**Conjecture S1 (the sources conjecture).**  For every game n, no playable set of sources of game n outweighs U\*: every
playable set S of sources has ΣS ≤ ΣU\*.

It is open, and checked game by game through (♦) ([statements.md](statements.md#the-statements)).

## Where exchange breaks

[The greedy algorithm of a matroid](primer.md#the-greedy-algorithm-of-a-matroid), which runs like descending greedy,
finds a heaviest independent set.  The independent sets of a matroid have the exchange property: all the maximal ones
have the same size.  Playable sets of sources have it as long as every member has at most two maximal factors.  Draw the
factors as vertices, with one more vertex, the ground, and each member as an edge between its two maximal factors, or
between its one maximal factor and the ground.  By [the two-factor proposition
(C3)](franklin-moniot.md#the-two-factor-proposition), a set of such members is playable exactly when its edges form a
forest.  The forests of a graph are the independent sets of its graphic matroid ([matroids](primer.md#matroids)), and
descending greedy is that matroid's greedy algorithm.

A member with three or more maximal factors is a hyperedge, and with hyperedges the exchange property fails.  It fails
in game 2873, on six sources built from the primes 11, 13 and 17; no failure in a smaller game is known:

| source | prime factors | maximal factors |
| --- | --- | --- |
| 1573 | 11²·13 | 121, 143 |
| 1859 | 11·13² | 143, 169 |
| 2057 | 11²·17 | 121, 187 |
| 2197 | 13³ | 169 |
| 2431 | 11·13·17 | 143, 187, 221 |
| 2873 | 13²·17 | 169, 221 |

The sets A = {1573, 2057, 2431, 2873} and B = {1573, 1859, 2057, 2197, 2873} are both playable.  No member of B fits
into A: A ∪ {1859} holds the core {1573, 1859, 2057, 2431, 2873}, and A ∪ {2197} is itself a core.  So among these six
sources A is a maximal playable set of size 4 and B one of size 5, and the playable sets are not the independent sets of
any matroid.  The textbook proof does not apply.

Descending greedy still gets these six right.  It keeps {1859, 2057, 2197, 2431, 2873}, of size 5 and sum 11,417, the
heaviest of the five maximal playable sets, and A, where it could be stuck, is the lightest.  The order of the values is
what saves it.  Of the 720 orders of the six sources, the 48 that put 1859 and 2197 last lead descending greedy to A;
the true order puts 1573 below both.

## The victim law

A jam is repaired by removing one member exactly when that member lies in every minimal core of the jam, a core that
holds no smaller core.

**Lemma S2 (the victim law).**  Let K be a set of members of an instance, and t a member of K, with K ∖ {t} keeping the
instance's pool and lists.  Then K ∖ {t} is playable if and only if t lies in every core in K, and if and only if t lies
in every minimal one.  In particular, a member of T is a victim of a jam T ∪ {d} exactly when it lies in every minimal
core in T ∪ {d}.

*Proof.*  By the core theorem (P7) the cores are the only obstructions to playability, and removing t clears them all
exactly when t is in each.  The lists are the instance's, so whether a set is a core depends on the set alone.

1. *If K ∖ {t} is playable, t lies in every core C in K.*  Otherwise C ⊆ K ∖ {t}, and a set that holds a core is not
   playable, by the core theorem.
2. *If t lies in every minimal core in K, then K ∖ {t} is playable.*  Otherwise K ∖ {t} holds a core C, by the core
   theorem.  A core inside C with the fewest members holds no smaller core, so it is a minimal core in K, and it misses
   t.
3. A member of every core is in every minimal one, so steps 1 and 2 close the circle.  ∎

The six sources of game 9170 built from the primes 17, 19 and 23 show the law at work:

| source | prime factors | maximal factors |
| --- | --- | --- |
| 4913 | 17³ | 289 |
| 5491 | 17²·19 | 289, 323 |
| 6137 | 17·19² | 323, 361 |
| 6647 | 17²·23 | 289, 391 |
| 7429 | 17·19·23 | 323, 391, 437 |
| 8303 | 19²·23 | 361, 437 |

The set T = {4913, 5491, 6647, 7429, 8303} is playable, and T ∪ {6137} is a jam with two minimal cores, {4913, 6137,
6647, 7429, 8303} and {5491, 6137, 6647, 7429, 8303}.  So its victims are 6647, 7429 and 8303, which lie in both, while
removing 4913 or 5491 leaves the other.

## The frontier theorem

Compare descending greedy's set with a heavier playable set at the largest member on which they disagree, the
*frontier*.  The comparison proves descending greedy right wherever the obstruction at the frontier is made of members
with at most two maximal factors.

**Theorem S3 (the frontier theorem).**  Let K be a set of sources of game n, weighted by their values, and G the set
that descending greedy keeps when run on K.  Let T be a playable subset of K other than G, and d the largest member in
exactly one of G and T.  Then:

1. d is in G and not in T;
2. if T ∪ {d} is not playable, every core in T ∪ {d} holds d and a member of T below d;
3. if moreover every member of the residual of T ∪ {d} has at most two maximal factors, some member t of T below d is
   a victim of the jam T ∪ {d}, and (T ∪ {d}) ∖ {t} outweighs T.

So a heaviest playable subset T of K other than G can exist only if T ∪ {d} is a jam whose residual holds a member with
three or more maximal factors.

*Proof.*  Above d, T agrees with G, and descending greedy's acceptance of d shows that T's part above d stays playable
with d.  So every obstruction to adding d to T reaches below d.  When the obstruction is a single cycle, a member below
d on it can be traded for d.

1. *The frontier is in G.*  The members of T above d are those of G above d.  If d were in T and not in G, descending
   greedy would have rejected d, so the members of G above d, together with d, would not be playable.  But they are
   the members of T of value d or more, a subset of the playable set T.
2. *Every obstruction reaches below d.*  Let C be a core in T ∪ {d}.  It holds d, since T is playable and so holds no
   core.  The members of T ∪ {d} of value d or more are those of G, by step 1, and they are playable, so C is not among
   them: it holds a member below d, which is a member of T.
3. *One cycle.*  Let R be the residual of T ∪ {d}, which is the union of the cores in it ([the residual theorem
   (P5)](playability.md#the-residual-theorem)).  Draw R as a multigraph, as in the two-factor proposition (C3): the
   factors and a ground vertex as vertices, and each member as an edge between its two maximal factors, or between its
   one maximal factor and the ground.  The set R ∖ {d} is part of T, so it is playable, and by C3 its edges form a
   forest.  Adding the one edge d to a forest makes at most one cycle; call it Z.
4. *Z is the only minimal core.*  A core in T ∪ {d} lies in R and is not playable, so by C3 its edges hold a cycle,
   which can only be Z.  And Z is a core: at each factor on a cycle two of its edges meet, so each factor that a member
   of Z lists is listed by another member of Z.  The set T ∪ {d} is not playable, so it holds a core, by the core
   theorem (P7); so Z exists, and every core in T ∪ {d} holds it.
5. *A trade.*  By step 2, Z holds a member t of T below d.  It lies in every minimal core in T ∪ {d}, so by the victim
   law (S2) it is a victim.  The set (T ∪ {d}) ∖ {t} outweighs T by d − t > 0.
6. *The conclusion.*  If T is a heaviest playable subset of K other than G, then T ∪ {d} is not playable, since it
   would outweigh T.  By steps 3 to 5, if every member of its residual had at most two maximal factors, a trade would
   outweigh T; so some member has three or more.  ∎

The proof uses of the weights only that they are positive and increase with the value.  It looks only at the residual
R, and rightly: by [the exclusion theorem (P8)](playability.md#the-exclusion-theorem), if removing a set E of members
repairs the jam T ∪ {d}, so does removing E's part in R.  A repair is a question about R alone.

### Where the frontier theorem stops

With a member of three maximal factors in the residual, a single trade below the frontier can fail, and the six sources
of game 9170 above show it.  Take d = 6137 and T = {4913, 5491, 6647, 7429, 8303}; the member 7429 = 17·19·23 has three
maximal factors.  On these six alone, descending greedy keeps {6137, 6647, 7429, 8303}, of sum 28,516, while T is
playable and sums to 32,783: on the six alone descending greedy is wrong, in size and in weight.  The two minimal cores
of T ∪ {6137} have the parts {4913} and {5491} below d, which share nothing, so no member below d is a victim.  The
victims 6647, 7429 and 8303 are heavier than d, and a repair from below d must remove both 4913 and 5491, which sum to
10,404, more than d.

In game 9170 the six sources lie in a larger component, which also holds 6859 = 19³, whose only maximal factor is 361.
Descending greedy keeps 6859, which is above 6137, and the five sources 5491, 6647, 6859, 7429 and 8303 form a core.
So a playable set that agrees with U\* above 6137 cannot hold 5491, and the second minimal core of T ∪ {6137}, the one
through 5491, is out of its reach.  In game 9170 the check of (♦) passes, so U\* is the heaviest playable set of sources
there (S6 and Q1).  Whether descending greedy is right thus depends on members outside the jam: a proof must charge the
members a heavier set keeps for what they lock out elsewhere in the component, not only at the frontier.  That is the
open problem in its irreducible form, and game 9170 is the first test a proof must pass.

## The cardinality conjecture

The weights can be dropped from the sources conjecture: a statement about sizes alone is equivalent to the conjecture
holding for every weighting that increases with value.

**Conjecture S4 (the cardinality conjecture (★)).**  For every game n, every component K of the sources' instance of
game n, and every t, descending greedy keeps a largest playable subset of the truncation K ∩ [t, n]: no playable subset
of K ∩ [t, n] has more members than U\* ∩ K ∩ [t, n].

The set U\* ∩ K ∩ [t, n] is what descending greedy keeps when run on the truncation itself, by the second of the two
facts under components and truncations.
Statement (★) *holds in game n* when it holds for that n.  It is open, and checked game by game through (♦).

Statement (★) in game n is equivalent to the sources conjecture holding in game n for every weighting of the sources
that is positive and strictly increasing in value.

* *Statement (★) suffices.*  The uniqueness theorem (Q1) proves it, and more: U\* is then the only heaviest playable
  set.  It sums the count inequalities of (★) over the components, then over the thresholds ([sums over
  thresholds](primer.md#sums-over-thresholds)).
* *Statement (★) is needed.*  Suppose a playable subset S of a truncation K ∩ [t, n] has more members than U\* ∩ K ∩
  [t, n].  Weigh each source x by x, plus a constant M when x ≥ t, with M greater than the sum of K.  The set S ∪ (U\*
  ∖ K) is playable, since playability splits over the components, and it outweighs U\* by at least M − ΣK > 0.

The six sources of game 9170 show why (★) is stated on whole components: on the six alone, descending greedy keeps four
where five fit.

## The span conjecture

Linear algebra over GF(2) ([finite fields](primer.md#finite-fields-and-gf2)) gives a certificate that proves (★) in a
game.  The incidence vector v(x) of a source x ([independence](playability.md#independence)) has a 1 at each of its
maximal factors.

**Conjecture S5 (the span conjecture (♦)).**  For every game n, every source x that descending greedy rejects on the
sources' instance of game n has v(x) equal, over GF(2), to a sum of the incidence vectors of sources it kept above x.

It is open, and checked game by game ([statements.md](statements.md#the-statements)).  Checking it in a game is
descending greedy with Gaussian elimination over GF(2)
([solver/noframe/docs/star.md](../../solver/noframe/docs/star.md#conjecture--at-scale)).

Statement (♦) is stronger than (★).  A rejection forced only by an [odd core](playability.md#odd-cores), a core whose
vectors are independent over GF(2), would break (♦) without breaking (★).  Odd cores occur among the sources: the core
{1573, 1859, 2057, 2431, 2873} of game 2873 is one, since its five vectors are independent over GF(2).  The failure of
exchange there needs one.  A set that is not playable but whose vectors are independent holds an odd core, so without
one the playable sets among the six would be exactly the sets with independent vectors ([the independence lemma
(P10)](playability.md#the-independence-lemma) gives the other direction), and those are a matroid's.  Descending greedy
has never been seen to reject a source because of an odd core.

## The span theorem

**Theorem S6 (the span theorem).**  If (♦) holds in game n, then (★) holds in game n, and so does the sources
conjecture, for every weighting of the sources that is positive and strictly increasing in value.

*Proof.*  Above any threshold, the members descending greedy keeps are independent and span the rest, so they are a
basis, and no playable set has more members than a basis.  Fix a component K and a threshold t, and let B = U\* ∩ K ∩
[t, n].

1. *B is independent.*  It is playable, as part of U\*, so its incidence vectors are independent over GF(2), by the
   independence lemma (P10).
2. *B spans K ∩ [t, n].*  A member of K ∩ [t, n] that descending greedy keeps is in B.  For a member x that it rejects,
   (♦) gives a set Y of kept sources above x with v(x) the sum of their vectors.  Members of other components list none
   of K's factors, so the vectors of the members of Y outside K sum to zero, and the independence of U\*'s vectors
   allows that only if there are none.  So Y lies in K, above x ≥ t, and in B.
3. *Sizes.*  So B is a basis of the space spanned by the vectors of K ∩ [t, n], and |B| is its rank.  A playable subset
   of K ∩ [t, n] has independent vectors (P10), so it has at most |B| members: that is (★) for K and t.
4. *Weights.*  The uniqueness theorem (Q1) turns (★) into the weighted statement.  ∎

## What a proof must use

These are observations from computation, with their evidence in [evidence.md](evidence.md) and
[solver/noframe/docs/star.md](../../solver/noframe/docs/star.md).

* **The order of the integers protects descending greedy, not their sizes.**  The 48 orders of game 2873's six sources
  that trap descending greedy upset sizes: it keeps four where five fit.  Each five-member set is A with one member
  traded for 1859 and 2197, so under such an order descending greedy fails in weight too whenever two members outweigh
  one, as any two values in (n/2, n] do.  No condition on the sizes of the values protects it, only their order.  What
  the true order has is the *window law*: among the sources f·p that share a maximal factor f, with p prime, the order
  of value is the order of p.  At game 2873 the window law puts 1573 = 143·11 below 1859 = 143·13, so 1859 and 2197
  cannot both be last, and none of the 48 traps obeys it.  The incidence structure alone permits the trap, so a proof
  must use the window law.
* **The component is the unit.**  Descending greedy can fail on part of a component and hold on the whole, as it does in
  game 9170.  An exact search of a component need only cover its residual, by the exclusion theorem (P8): the component
  of game 2873 that holds its six sources has 377 members, 70 of them in its residual, and an exact search there finds
  descending greedy's six exclusions the fewest possible.
* **A matching certifies most thresholds.**  A playable set's members have distinct maximal factors (P2), so a playable
  subset of K has at most ν(K) members, the matching number of K's incidence graph.  Where descending greedy meets this
  bound, (★) holds there with no search, as it does at most thresholds.  Where the bound overshoots, the surplus is
  sets whose members have distinct factors but are not playable, like the set of game 21 that keeps that game below
  its ceiling ([the step theorem (N6)](neighbors.md#the-step-theorem)).
* **Three primes.**  A census of the jams three primes can make found every trap order either impossible for primes or
  defeated in every realization tested.  A shape that is a minimal core cannot trap descending greedy:
  every one of its members is a victim (S2), so descending greedy keeps all but the smallest, as many as fit.  Traps
  need a core with other members competing for its factors, and they appear exactly when the
  shape's maximal playable subsets come in two adjacent sizes, the failure of exchange of game 2873.  Of the trap orders
  found, all but seven ask for an order no primes realize, such as p²q above pq² for primes p < q, or were never
  realized.  The seven realizable ones were defeated at every realization tested by a *window guard*: a source, q³ or
  q²r for the shape's primes p < q < r, that the shape's own members force into (n/2, n].  That the guard always
  suffices is measured, not proved.
* **The window law for the right primes.**  The span conjecture held under every order of the real incidence structure
  that obeys the window law, random or chosen adversarially, and under the window law reversed.  It failed about as
  often as a random order under the window law for a random ranking of the primes, or for the primes perturbed by more
  than a few percent, while generalized primes, perturbed by up to 50% with the members and factors rebuilt from them,
  never failed ([star.md, "Adversarial search: what protects
  greedy"](../../solver/noframe/docs/star.md#adversarial-search-what-protects-greedy)).

So a proof of (★) must show that within one component's residual the order of the integers makes descending greedy's
exclusions the fewest possible, with the window law as its engine.  Stated without magnitudes, the target is (♦) for
instances built multiplicatively from primes, under any order that obeys their window law.  Game 9170's component is the
first test.

The examples of games 21, 2873 and 9170 are computed with `heuristics/core.py`: peel decides playability, descending
greedy gives U\*, and the cores among six sources and their 720 orders are enumerated, in seconds.
