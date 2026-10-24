# The parity bound on the whole game

Part of [the theory](../THEORY.md).

Linear algebra over GF(2) bounds the best score of a game from above.  Every playable set is independent in two
matroids over GF(2), one for each parity of the number of prime factors, so the heaviest set independent in both bounds
opt(n) (the parity bound, W1).  Weighted matroid intersection computes it in polynomial time, and a weight split checks
its value without trusting the algorithm that found it.  With some numbers fixed in and others fixed out it bounds the
sets that obey those choices, and the barred bound (W2), with the bottom two layers fixed and one source left out, tells
which sources every set beating a score must play.  Both results are proved.  The prover bounds its searches by W1, and
the certificates of statement U use W2.  The same construction frame by frame bounds the frame model's promotions
([bounds on the promotions](frame-bounds.md)).

The terms of the game ([playable](../GLOSSARY.md), maximal factor, layer, source, U\*) are the glossary's; GF(2),
matroids, contraction, deletion and coloops are explained in [the primer](primer.md#matroids).

## The parity matroids

Fix a board n ≥ 2.  The coordinates are the numbers 1 to ⌊n/2⌋, since every maximal factor of a number up to n is at
most n/2.  Each number s of 2..n has the vector v(s) over GF(2) with a 1 at each of its maximal factors, and each
s ≤ n/2 also has its unit vector e_s, at its own coordinate.  For a set S ⊆ {2, …, n}, the family

    F(S) = {v(s) : s ∈ S} ∪ {e_s : s ∈ S, s ≤ n/2}

holds the vectors of S's members together with their own coordinates.  It is independent exactly when the vectors u(s),
the v(s) with the coordinates of S's members deleted, are.  Restricted to the coordinates outside S, a vanishing
combination of F(S) loses its unit vectors and leaves a vanishing combination of the u(s); and a vanishing combination
of the u(s) is, before the deletion, a combination of the v(s) that lives on S's own coordinates, which the unit vectors
of F(S) cancel.

Split the coordinates by the parity of their layer Ω (1 has Ω(1) = 0, even): E₀ holds the coordinates with Ω even, E₁
those with Ω odd.  A maximal factor f of s has Ω(f) = Ω(s) − 1, so v(s) lies in the part of the parity other than
Ω(s)'s, and e_s in the part of Ω(s)'s own parity.  Each number thus has at most one vector in each part.

**Definition (the parity matroids).**  For i = 0 and i = 1, the parity matroid M_i is the linear matroid on {2, …, n}
over GF(2) in which s is represented by its vector in E_i:

* by v(s), if Ω(s) and i have different parities;
* by e_s, if they have the same parity and s ≤ n/2;
* as a coloop, if they have the same parity and s > n/2, since s then has no vector in E_i.

*Example.*  On the board of 10 the coordinates are 1 to 5, with E₀ = {1, 4} and E₁ = {2, 3, 5}.

| number | Ω | in M₀ | in M₁ |
|---|---|---|---|
| 2, 3, 5 | 1 | e₁ | e₂, e₃, e₅ |
| 7 | 1 | e₁ | coloop |
| 4 | 2 | e₄ | e₂ |
| 6, 9, 10 | 2 | coloops | e₂ + e₃, e₃, e₂ + e₅ |
| 8 | 3 | e₄ | coloop |

In M₀ the primes are parallel, since 1 is the only maximal factor of each, so a set independent in M₀ holds at most one
prime, as a game does.  The numbers 4 and 8 are parallel too, and no game plays both, since 4 is the only maximal
factor of 8.  In M₁, {4, 6, 9} is dependent (v(4) + v(9) = v(6)), though M₀ sees nothing wrong with it, and it is
unplayable: its three members have only the two maximal factors 2 and 3 between them.  The heaviest set independent in
both is {6, 7, 8, 9, 10}, worth 40.  It is playable, in the order 7, 9, 6, 10, 8, so here the bound is opt(10) = 40.

## The parity bound

**Theorem W1 (the parity bound).**  Let n ≥ 2, and let P and X be disjoint sets of numbers of 2..n.  Every playable set
S ⊆ {2, …, n} is independent in both parity matroids M₀ and M₁.  Hence, if P is independent in both, every playable S
with P ⊆ S and S ∩ X = ∅ satisfies

    ΣS ≤ ΣP + max { ΣI : I independent in M₀/P ∖ X and in M₁/P ∖ X },

and if P is dependent in either matroid, there is no such S.  In particular (P = X = ∅), opt(n) is at most the heaviest
sum ΣI of a set I independent in M₀ and in M₁.

The right side is the *parity bound of n with the picks P and the bars X*, taken as −∞ when P is dependent in either
matroid; with no picks and no bars it is *the parity bound of n*.

*Proof.*  The idea: a playable set's vectors with its own coordinates are independent, and since each of them lies in
one parity's part, the family splits into two independent halves, which are the set's representations in M₀ and M₁.

1. *F(S) is independent.*  This is [the independence lemma (P10)](playability.md#the-independence-lemma), over GF(2).
2. *F(S) splits by parity.*  Every vector of F(S) lies in E₀ or in E₁, and GF(2)^{⌊n/2⌋} is the direct sum of the two
   parts.  A vanishing combination of F(S) therefore vanishes in each part separately, so F(S) is independent if and
   only if its vectors in E₀ are and its vectors in E₁ are.
3. *S is independent in M_i.*  The vectors of F(S) in E_i are exactly the vectors that represent in M_i the members of S
   that are not coloops of M_i: v(s) when Ω(s) and i differ in parity, and e_s when they agree and s ≤ n/2.  By steps 1
   and 2 they are independent, and adding coloops to an independent set keeps it independent.
4. *The bound.*  The moves of a game form a playable set, and a playable set can be played and the game continued, so
   opt(n) is the largest sum of a playable set; by step 3 it is at most the heaviest sum of a common independent set.
5. *Picks and bars.*  Let S be playable, with P ⊆ S and S ∩ X = ∅.  By step 3, S is independent in M_i, so its subset
   P is, and S ∖ P is independent in the contraction M_i/P, by the definition of contraction
   ([primer](primer.md#matroids)).  It avoids X, so it is independent in M_i/P ∖ X, for i = 0 and 1, and
   ΣS = ΣP + Σ(S ∖ P).  If P is dependent in either matroid, no playable set contains it.  ∎

The bound is never above the [Franklín–Moniot bound](franklin-moniot.md), the heaviest set whose members can be given
distinct maximal factors outside the set: a set whose family is independent can be given them, which is part of [the
counting theorem (C2)](franklin-moniot.md#the-counting-theorem).  And the bound joins two matroids, not one, so [the
matroid proposition (Q2)](uniqueness-and-longest-games.md#optimal-sets-on-the-whole-board), by which no single matroid
is exact for the whole game, does not apply to it.

### Computing and checking the bound

Three properties make the bound useful.

* **It takes polynomial time.**  The heaviest common independent set of two matroids is a weighted matroid
  intersection ([primer](primer.md#matroid-intersection); E. L. Lawler, "Matroid intersection algorithms",
  *Mathematical Programming* 9 (1975) 31–56; A. Frank, "A weighted matroid intersection algorithm", *Journal of
  Algorithms* 2 (1981) 328–336), and the matroids here are given by sparse vectors over GF(2).
* **Its value can be checked without trusting the algorithm.**  Split the weights as w = w₀ + w₁, and write max_{M_i}
  w_i for the heaviest weight w_i(J) of a set J independent in M_i, of any size, for weights of either sign.  By weak
  duality ([primer](primer.md#weak-duality-and-lagrangian-relaxation)), every set T independent in both matroids has
  w(T) = w₀(T) + w₁(T) ≤ max_{M₀} w₀ + max_{M₁} w₁.  So a common independent set I that is heaviest in M₀ for w₀ and in
  M₁ for w₁ has the largest weight, whatever produced it, and the optimality criterion of [the greedy algorithm of a
  matroid](primer.md#the-greedy-algorithm-of-a-matroid) checks each of those two conditions one exchange at a time.
  Every heaviest common independent set has such a split, since the intersection of the two matroids' polytopes has
  integral vertices (J. Edmonds, "Submodular functions, matroids, and certain polyhedra", in *Combinatorial Structures
  and their Applications*, Gordon and Breach, New York, 1970, 69–87), and Frank's algorithm finds one.  Nothing rests
  on that existence: a value is accepted only with its split.
* **The same split bounds the sets that avoid a member x of I.**  Such a set weighs at most the heaviest independent set
  of M₀ ∖ x for w₀ plus that of M₁ ∖ x for w₁, by the same weak duality, and each term is one exchange from I: *in a
  matroid with I heaviest for weights w of either sign, and x ∈ I, a heaviest independent set avoiding x is I − x, or
  I − x + y for the heaviest y ∉ I with I − x + y independent.*  For the proof, add as many coloops of weight 0, the
  dummies, as the matroid's rank r, and truncate to rank r.  The bases of the result are the independent sets padded
  with dummies, at the same weights, so I padded is a heaviest basis B, and the claim is about a heaviest basis B′ that
  avoids x.  By the symmetric exchange property of bases (R. A. Brualdi, "Comments on bases in dependence structures",
  *Bulletin of the Australian Mathematical Society* 1 (1969) 161–167), some y ∈ B′ ∖ B makes both B − x + y and
  B′ − y + x bases.  Since B is heaviest, w(B′ − y + x) ≤ w(B), that is w(B′) ≤ w(B − x + y).  So the basis
  B − x + y, which avoids x, is at least as heavy as B′; it is I − x + y when y is a number, and I − x when y is a
  dummy.

The prover bounds every node of its searches by W1, with the node's picks and bars, and reports a value only with its
split ([solver/noframe/docs/proofs.md, "With the parity bound (to
1300)"](../../solver/noframe/docs/proofs.md#with-the-parity-bound-to-1300)).

## The barred bound

The barred bound fixes what [the bottom-layers theorem (F3)](frame-model.md#the-bottom-layers-theorem) says every
optimal set plays in layers 1 and 2, and bars one source of a higher layer.

**Definition.**  Let q(n) be the largest prime ≤ n.  A set S ⊆ {2, …, n} *obeys F3* when it holds q(n) and no other
prime, and holds exactly U\*_2 among the sources of layer 2.  The *roles of F3* are the picks {q(n)} ∪ U\*_2 and the
bars: every prime other than q(n), every source of layer 2 outside U\*_2, and, when U\*_2 spans every prime up to n/2 in
the graph of F3, every number of layer 2 up to n/2.  For a member u of U\*_{≥3}, the *barred bound* of u is the parity
bound of n with the picks of F3 and with the bars of F3 and u.

By F3, every optimal set of n obeys F3.  A playable set obeys F3 exactly when it holds the picks and none of the bars.
The only bars that the definition of obeying does not name are the numbers of layer 2 up to n/2 when U\*_2 spans every
prime up to n/2, and a playable set that obeys F3 holds none of them.  Such a number would close a cycle with U\*_2 in
the graph of F3 (pq is the edge p–q, and p² the edge from p to a ground vertex).  A cycle's numbers list each of its
primes twice, and the primes up to n/2 are no members of the set, so the cycle is self-covering, which no playable set
contains ([the core theorem (P7)](playability.md#the-core-theorem)).

**Corollary W2 (the barred bound).**  Let n ≥ 2, V a number, and u a member of U\*_{≥3}.  If the barred bound of u is at
most V, every playable set that obeys F3 and weighs more than V holds u.  In particular, if opt(n) > V, every optimal
set of n holds u.

*Proof.*  Let S be playable, obey F3, and miss u.  Then S holds the picks of F3 and avoids its bars and u, so by W1, ΣS
is at most the barred bound of u, which is at most V.  So a playable set that obeys F3 and weighs more than V cannot
miss u.  If opt(n) > V, every optimal set weighs more than V, and it obeys F3.  ∎

The proof uses of u only that it is not a pick of F3, so W2 holds for every such number, with its barred bound defined
the same way; statement U asks about the members of U\*_{≥3}.

W2's conclusion is the condition that [the certificate theorem (E4)](extended-problem.md#the-certificate-theorem) asks
of a certified member, so a member whose barred bound is at most V is certified for V.  The barred bound does not
depend on V, so a member certified for one score is certified for every higher one.  E4 fixes the certified members in
a search of the extended problem and leaves the others open; the prover computes the barred bounds game by game
([solver/noframe/docs/proofs.md, "The upper part, game by
game"](../../solver/noframe/docs/proofs.md#the-upper-part-game-by-game)).

## What the parity bound misses

The parity bound admits exactly the sets whose family F(S) is independent, since steps 2 and 3 of the proof of W1 hold
in both directions.  So it exceeds opt(n) only through the unplayable sets whose families are independent over GF(2),
each of which holds an [odd core](playability.md#odd-cores).  By the counting theorem (C2), F(S) is independent exactly
when, for some set T of maximal factors outside S, the payment schemes of S onto T are odd in number, while S is
playable exactly when, for some T, there is only one; the bound keeps the count mod 2 and drops the rest.  An odd core
has a member with three or more maximal factors outside it ([the two-factor proposition
(C3)](franklin-moniot.md#the-two-factor-proposition)), and the first is on the board of 45 ([the first-odd-core
proposition (C4)](franklin-moniot.md#the-first-odd-core)).  As measured, the bound equals opt(n) for every n up to 77
([the table of statements](statements.md#the-statements)).

GF(2) is the field for the count mod 2, and another field would miss more.  The triangle {6, 10, 15} is unplayable on
every board that has it: its members' maximal factors are 2, 3 and 5, each listed by two of them, so whichever member is
played first takes two of the three, and the other two members share the one left.  Over GF(2) its vectors sum to 0, so
the parity bound excludes it.  Over the rationals the determinant of its three vectors is −2, and over GF(3) it is 1, so
with entries 1 the triangle is independent over either field, and a bound built there would admit it.
