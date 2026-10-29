# Primer

This primer explains the standard mathematics that the proofs of [the theory](../THEORY.md) use.  It advances nothing:
each fact is in the textbooks, and the primer proves it where the proof is short and teaches something, and otherwise
cites it ([the references](#references) come last).  The examples are Taxman's.  A section can be read with the sections
it names, so that a proof elsewhere links the one section it uses.  A reader who knows proofs, sets, graphs and real
linear algebra can follow all of it, but for one quoted fact, linear programming duality.

*Conventions.*  For a set Y and an element x, Y + x is Y ∪ {x} and Y − x is Y ∖ {x}.  A weight function w on a set E
gives Y ⊆ E the weight w(Y), the sum of w(y) over y in Y.  A set is *heavier* than another if its weight is larger, and
a *heaviest* set among some sets has the largest weight of them: a maximum-weight set, in the standard term.  Where the
theory names no other weights, a number's weight is its value, so a heaviest set is one of largest sum, never one of
most members.  A number s has a *vector* v(s) over a list of coordinates, with a 1 at each of its [maximal
factors](../GLOSSARY.md#the-game) in the list and a 0 elsewhere (6 has the maximal factors 3 and 2); the theory calls it
the incidence vector ([independence](playability.md#independence)).

## Finite fields and GF(2)

Linear algebra works over every field, and the field decides which sets of vectors are independent.  The field GF(2), in
which 1 + 1 = 0, is the one where two equal vectors cancel.

**Fields.**  A *field* 𝔽 is a set with an addition and a multiplication that obey the usual rules, including that every
element has a negative and every nonzero element has an inverse.  The rational and real numbers are fields; the integers
are not.  For a prime p, the integers mod p form the field GF(p): a residue a ≠ 0 is coprime to p, so αa + βp = 1 for
some integers α and β, and α inverts a.  GF(2) has the elements 0 and 1 with 1 + 1 = 0; because −1 = 1, subtracting is
adding, and *signs vanish*.  GF(3) has 0, 1 and 2 = −1, so 1 + 1 = 2 and 2 · 2 = 1.

**Linear algebra over a field.**  A family of vectors over 𝔽 is *linearly independent* when the only linear combination
of its members that is zero has every coefficient 0, and its *rank* is the size of its largest independent subfamily.
Row reduction divides only by nonzero elements, so what it proves holds over every field: a square matrix has
independent rows exactly when its determinant is not 0, and the rank of a matrix is the size of its largest square
submatrix with a nonzero determinant.  Over GF(2) the only nonzero coefficient is 1, so a family of 0/1 vectors is
dependent exactly when some nonempty subfamily lists every coordinate an even number of times.

**Example.**  The numbers 6, 10 and 15 have the maximal factors {3, 2}, {5, 2} and {5, 3}, so their vectors over the
coordinates 2, 3, 5 are the rows

    6    1 1 0
    10   1 0 1
    15   0 1 1

Each coordinate is listed twice, so the rows sum to zero over GF(2): they are dependent, with rank 2.  Their determinant
is −2, which is not 0 over the rationals and is 1 over GF(3), so they are independent over both.  The vectors of 10, 14,
15 and 21 over 2, 3, 5, 7 satisfy v(10) − v(14) − v(15) + v(21) = 0, so they are dependent over every field.

**Comparing fields.**  Integer vectors that are independent over GF(p), for a prime p, are independent over the
rationals; the example shows the converse failing at p = 2.  *Proof.*  Suppose d vectors are independent over GF(p).
Some d × d submatrix has a determinant that is nonzero mod p, so an integer that p does not divide, so not 0.  ∎

## Linear independence

A *family of vectors* is a finite set E of *members*, each with a vector over a field 𝔽, two members possibly with the
same one.  A set I ⊆ E is *independent* when its members' vectors, one for each, are linearly independent, and
*dependent* otherwise.  Every subset of an independent set is independent, the empty set included.

**Rank, span and bases.**  The *rank* rk(Y) of Y ⊆ E is the rank of Y's vectors, the dimension of their span, and a set
I *spans* a member, or a set, when the span of I's vectors holds its vectors.  For an independent I and y outside it,
I + y is dependent exactly when I spans y.  So an independent subset of Y is maximal exactly when it spans Y, and then
its vectors are a basis of the span of Y's: all the maximal independent subsets of Y have rk(Y) members, and no
independent one has more.  This is the *exchange property*.  Equivalently, if I and I′ are independent and |I| < |I′|,
some x in I′ ∖ I has I + x independent, as I's span cannot hold all of I′'s vectors.  A *basis* is a maximal independent
subset of E.

**Circuits.**  A *circuit* is a minimal dependent set.  A nontrivial vanishing combination of its vectors has no
coefficient 0, or a smaller set would be dependent, so over GF(2) a circuit is a set whose vectors sum to 0, no smaller
nonempty part of which does.  A member with the vector 0 is a circuit alone, and two with the same nonzero vector form
one.  A member x has a *coordinate of its own* when its vector is not 0 at a coordinate where every other member's is 0;
that coordinate gives x the coefficient 0 in every vanishing combination, so x joins every independent set, lies in no
circuit and is in every basis.

**The circuit of I + y.**  Let I be independent and y a member outside I that I spans.  Then y's vector is a combination
of I's in one way only, as two would differ by a vanishing combination of I; say it uses the members C.  Then C + y, the
*circuit of I + y*, is the one circuit inside the dependent set I + y: every circuit inside it holds y, since I is
independent, and its vanishing combination, with no coefficient 0, writes y's vector by its other members, which must be
C.  A member x of I lies in C exactly when I − x does not span y, that is, when I − x + y is independent.

**Changing the family.**

* *Outside Z.*  The independent sets that avoid a set Z are those of the family on E ∖ Z.
* *Together with Y.*  For an independent Y, a set I outside Y is *independent together with Y* when Y ∪ I is
  independent; for a dependent Y, none is.  This holds exactly when the classes of I's vectors modulo the span W of Y's
  are independent, a family of vectors in the quotient space.  *Proof.*  The classes are dependent exactly when a
  vanishing combination of the vectors of Y ∪ I has a coefficient of I that is not 0, and every nontrivial one does,
  since Y is independent.  ∎  A member with its vector in W has the class 0, and two whose vectors differ by one in W
  share a class.  If Y's vectors are unit vectors, working modulo W deletes their coordinates.
* *At most k members.*  The independent sets with at most k members keep the exchange property: if |I| < |I′| ≤ k, the x
  of I′ that can join I leaves I + x with at most k members.
* *Disjoint blocks.*  If each member's vector is 0 outside one of some disjoint blocks of coordinates, a set is
  independent exactly when its part on each block is, as a vanishing combination splits into one on each block.  The
  same holds if the vectors of E₁ lie in a subspace W₁ and those of E₂ in W₂, with only 0 in both.

### Forests

Over GF(2), with a coordinate for each vertex, the edge pq of a graph has the vector e_p + e_q.  A set of edges is
independent exactly when it is a *forest*, with no cycle.  *Proof.*  The edges of a cycle sum to zero, each of its
vertices ending two of them.  Each nonempty subset of a forest has a vertex of degree 1, where the sum of its vectors
has a 1.  ∎  So the circuits are the cycles, and a forest spans an edge outside it exactly when the edge closes a cycle
with it.  A maximal forest is a spanning tree of each component, so the rank is the number of vertices less the number
of components (Oxley, Section 5.1), and a set is a forest exactly when its part in each component, a block of
coordinates, is one.

**Example.**  A number pq with two different primes has the maximal factors p and q, so its vector is e_p + e_q: the
edge pq of a graph on the primes.  In the game of 21, the numbers 6, 10, 14, 15 and 21 are the edges 2–3, 2–5, 2–7, 3–5
and 3–7 on the primes 2, 3, 5, 7.  The circuits are the triangles {6, 10, 15} and {6, 14, 21} and the 4-cycle {10, 14,
15, 21}.  The rank is 3, and there are 8 bases.  Together with 6, the edges 10 and 15 share a class, their vectors
differing by 6's, and so do 14 and 21: the ends of 6 merge.  Every other pair stays independent together with 6.  In the
game of 35, the edges 6 = 2–3 and 35 = 5–7 share no prime, so each has a coordinate of its own in {6, 35}.

## Sums over thresholds

A weighted sum is a positive combination of counts above thresholds.  So a set with at least as many members above every
threshold weighs at least as much under every increasing positive weighting.

**The identity.**  Let E be a finite set and w a positive weight function on E.  For each value ρ of w, let ρ⁺ be the
next smaller value, or 0 for the smallest.  For every Y ⊆ E,

    w(Y) = Σ_ρ (ρ − ρ⁺) · |Y ∩ {w ≥ ρ}|,    summed over the values ρ of w.

*Proof.*  A member of Y of weight ρ₀ is counted in the terms for ρ ≤ ρ₀, whose coefficients add up to ρ₀, since each
cancels against the next.  ∎  (A layer cake: the weight is stacked in layers of thickness ρ − ρ⁺, all positive.)

**Dominance.**  Let w be a positive weight function on a finite set E with distinct values on distinct elements, and let
S and S′ be subsets of E with |S ∩ {w ≥ t}| ≤ |S′ ∩ {w ≥ t}| for every t.  Then w(S) ≤ w(S′), with equality only if
S = S′.  For a set of numbers, w is any strictly increasing positive weighting, and {w ≥ w(x)} is the set of members
that are at least x.  *Proof.*  Every coefficient of the identity is positive, so w(S) ≤ w(S′) term by term.  Equality
forces equal counts at every value ρ.  The step from the next larger value to ρ adds one element, and the count rises by
1 exactly when that element is in the set, so S and S′ have the same elements.  ∎  Positivity matters: for E = {1, 2}
and w(x) = x − 10, the set {1, 2} has at least as many members above every threshold as {1}, but weighs −17 against −9.

**The converse.**  Let S and S′ be sets of numbers with |S ∩ {≥ t}| > |S′ ∩ {≥ t}| for some t.  Then some strictly
increasing positive weighting w has w(S) > w(S′).  *Proof.*  Take w(x) = εx + 1 for x ≥ t and w(x) = εx for x < t.  Then
w(S) − w(S′) = (|S ∩ {≥ t}| − |S′ ∩ {≥ t}|) + ε(ΣS − ΣS′), which is positive for small ε.  ∎  So dominance for every
such weighting needs dominance at every threshold.

**Example.**  Among the numbers 10, 14, 15, 21, let S = {10, 15, 21} and S′ = {14, 15, 21}.  The counts at the
thresholds 21, 15, 14, 10 are 1, 2, 2, 3 for S and 1, 2, 3, 3 for S′, so S′ dominates S, and with w(x) = x the identity
gives 46 for S and 50 for S′.  They are two spanning trees of the graph in [Forests](#forests), and S′ is the scan's.

## The greedy algorithm

On a family of vectors, the greedy algorithm (the scan) goes through the members from heaviest to lightest and keeps
each one whose vector is not in the span of those kept.  It finds a heaviest basis for any weights and, once its members
of weight at most 0 are dropped, a heaviest independent set; and a set is heaviest exactly when no single change
improves it.

*Three kinds of greedy.*  The scan is not the Taxman strategy that is often called greedy, which takes the largest legal
move, and it is not the theory's *descending greedy* ([sources.md](sources.md#descending-greedy)), which runs the same
scan on playable sets.

*Playable sets are not independent sets of vectors.*  Already in the game of 4, {2} and {3, 4} are playable, but neither
3 nor 4 can join 2: 3 would need the 1 that 2 needs, and 4's only maximal factor is 2 itself.  So the maximal playable
subsets of {2, 3, 4} differ in size, against [the exchange property](#linear-independence) of vectors, and the theorem
below does not cover descending greedy.

**The scan.**  Let E be a family of vectors and w a weight function on E.  List E in decreasing order of weight, with
ties in any order.  Start with G empty, and for each member x in turn, put x in G if G + x is independent.

**Example.**  Take the edges 6, 10, 14, 15 and 21 of [the example](#forests), and let w be the number itself.  The scan
keeps 21 (the edge 3–7), 15 (3–5) and 14 (2–7).  It rejects 10, which would close the 4-cycle, and 6, which would close
the triangle {6, 14, 21}.  So G = {21, 15, 14}, which weighs 50; the other seven bases weigh 30, 35, 37, 39, 42, 45
and 46.

![The edges 6, 10, 14, 15 and 21 on the primes 2, 3, 5, 7, with the scan keeping 21, 15 and 14 and rejecting 10 and 6](img/graphic-21.svg)

*The graph, each edge a number between its two primes.  Thick: the edges the scan keeps.  Dashed: the edges it rejects.*

**What the scan finds.**  Let E be a family of vectors, w a real weight function on E, and G the scan's set for any
order of ties.  Let G⁺ be the part of G with positive weight.

* (a) G is a basis.
* (b) For every real t, G ∩ {w ≥ t} is a maximal independent subset of {w ≥ t}, that is, it spans {w ≥ t}.  So every
  independent set I has |I ∩ {w ≥ t}| ≤ |G ∩ {w ≥ t}|.
* (c) G is a heaviest basis, and G⁺ is a heaviest independent set among the independent sets of every size.
* (d) If the weights are distinct, G is the only heaviest basis, and if they are distinct and not 0, G⁺ is the only
  heaviest independent set.

*Proof.*  The idea is that the scan settles every member of weight at least t before it looks at a lighter one.

(b) Put Y = {w ≥ t}.  Every member of Y comes before every member outside it.  Take x in Y ∖ G.  The scan rejected x, so
the set kept before x, plus x, was dependent, and that kept set lies in G ∩ Y because only members of Y came earlier.
So (G ∩ Y) + x is dependent.  Nothing in Y can join G ∩ Y, which is independent as a subset of G, so G ∩ Y spans Y and
is a maximal independent subset of Y.  It has rk(Y) members, and an independent I ∩ Y has at most rk(Y).

(a) Take t at the least weight, so that Y = E.

(c) By the identity of [Sums over thresholds](#sums-over-thresholds), a set of positive weights has a weight that is a
positive combination of its counts |· ∩ {w ≥ t}|, and (b) bounds each count of an independent I by that of G.  Bases
have one size, so adding a constant to every weight leaves w(I) − w(G) unchanged; add one that makes all weights
positive.  For G⁺: w(I) ≤ w(I⁺), and the identity over the members of positive weight bounds w(I⁺) by w(G⁺).

(d) Make the weights positive as in (c); they stay distinct.  By (b) and Dominance, w(I) ≤ w(G) for every basis I, with
equality only if I = G.  For G⁺: a heaviest I has w(I) = w(I⁺), so no member of weight below 0, and none of weight 0, so
I = I⁺; Dominance on the members of positive weight, with (b), gives I = G⁺.  ∎

The weights must not be 0 for the last claim: with two members x and z of the vectors e₁ and e₂, w(x) = 1 and w(z) = 0,
the sets {x} and {x, z} are both heaviest.  [The last section](#matroids) says on which other families the scan works.

### The optimality criterion

A set can be certified heaviest by checking single changes, whatever algorithm produced it.

**The criterion.**  Let E be a family of vectors, w a real weight function on E, and I an independent set.  Then I is a
heaviest independent set, among the independent sets of every size, exactly when

1. w(x) ≥ 0 for every x in I;
2. w(y) ≤ 0 for every y outside I with I + y independent;
3. w(y) ≤ w(x) for every y outside I with I + y dependent and every x in I that lies in the circuit of I + y.

*Proof.*  The idea is that each condition guards against one improving change, and when none is left the proof of (c)
applies to I.  If I is heaviest, the conditions hold: a violation of (1) gives the heavier set I − x, one of (2) gives
the heavier independent set I + y, and one of (3) gives I − x + y, independent by [the circuit of
I + y](#linear-independence) and heavier by w(y) − w(x).

Now suppose (1) to (3).  For t > 0 we claim that I ∩ {w ≥ t} spans {w ≥ t}.  Take y in {w ≥ t} ∖ I.  By (2), I + y is
dependent, since w(y) ≥ t > 0.  Let H be its circuit.  By (3), each x in H − y has w(x) ≥ w(y) ≥ t, so H − y lies in
I ∩ {w ≥ t}, and (I ∩ {w ≥ t}) + y holds H and is dependent.  So the claim holds.  As in (b), every independent I′ has
|I′ ∩ {w ≥ t}| ≤ |I ∩ {w ≥ t}| for t > 0.  By (1), w(I) = w(I⁺), and the argument of (c) gives w(I′) ≤ w(I′⁺) ≤
w(I⁺) = w(I).  ∎

**Example.**  Among the forests of [the example](#forests), with w the number itself, I = {21, 15, 14} meets the
criterion.  Condition (1) holds and (2) is empty, since I is a basis.  For (3), the circuits of I + 10 and I + 6 are
{10, 14, 15, 21} and {6, 14, 21}, and their members in I are larger than 10 and 6.  The basis I′ = {10, 15, 21} fails
(3): the circuit of I′ + 14 holds 10, which is less than 14, and I′ − 10 + 14 = I is heavier.

*Dummy members.*  Equivalently, add rk(E) *dummies* of weight 0, each with a coordinate of its own, and keep the
independent sets with at most rk(E) members.  The independent sets of E, padded with dummies, are the bases of the
result, and (1), (2) and (3) are the exchanges of a member for a dummy, a dummy for a member, and a member for a member:
the classical test that no single exchange improves a basis (Korte and Vygen, Section 13.4, Theorem 13.23).

## Matchings, permanents and determinants

The permanent of a 0/1 matrix counts the perfect matchings of its graph, the determinant counts them with signs, and mod
2 the signs vanish.  So a square 0/1 matrix is nonsingular over GF(2) exactly when its graph has an odd number of
perfect matchings.

**Definitions.**  A graph is *bipartite* when its vertices split into two sides and every edge joins the sides.  A
*matching* is a set of edges, no two with a vertex in common; it is *perfect* when the sides have d vertices each and
the matching has d edges.  The *matching number* ν of a graph is the size of its largest matching.  The *matrix* of a
bipartite graph has a row for each vertex x on one side, a column for each vertex y on the other, and a 1 where xy is an
edge and a 0 elsewhere.

**Permanent and determinant.**  Fix an order of the rows and of the columns of a d × d matrix Ψ.  A bijection τ from the
rows to the columns has a *sign* ±1, that of the permutation it makes.  The *permanent* and the *determinant* are

    per(Ψ) = Σ_τ ∏_x Ψ[x, τ(x)]        det(Ψ) = Σ_τ sign(τ) ∏_x Ψ[x, τ(x)].

For a 0/1 matrix, the product for τ is 1 exactly when every pair (x, τ(x)) is an edge, that is, when τ is a perfect
matching of the graph, and is 0 otherwise.  So per(Ψ) is the number of perfect matchings, and det(Ψ) is the number of
perfect matchings of sign +1 less the number of sign −1.  Another order changes every sign together, so whether two
matchings have the same sign does not depend on the order.

**What the theory uses.**

* *Mod 2.*  Since −1 = 1 in GF(2), det(Ψ) mod 2 equals per(Ψ) mod 2, so the matrix is nonsingular over GF(2) exactly
  when its graph has an odd number of perfect matchings.
* *Over the rationals.*  The matrix is nonsingular exactly when the signed count is not 0.  An odd number of perfect
  matchings gives an odd signed count, so a nonzero one; an even number can give a signed count of 0 or not.
* *Rank and matching number.*  Over every field, the rank of the matrix is at most ν of its graph.  *Proof.*  A nonzero
  d × d minor has a nonzero term in its expansion, and a nonzero term is a matching with d edges.  ∎

**Example.**  The numbers 6, 10 and 15 against the factors 2, 3, 5 have the matrix with the rows 1 1 0, 1 0 1 and 0 1 1.
Its graph has two perfect matchings, {(6, 2), (10, 5), (15, 3)} and {(6, 3), (10, 2), (15, 5)}, and each swaps two of
the columns, so both have sign −1.  (These are the payment schemes of the set {6, 10, 15}.)  So per = 2 and det = −2: an
even count, nonsingular over the rationals and singular modulo 2.

## Two families at once

For the sets independent in two families of vectors at once, the greedy algorithm fails, but a classical algorithm finds
a heaviest one, and a split of the weights proves its value.

**Definition.**  Let each member of a finite set E have a vector in family 0 and one in family 1; a set is *independent
in both* when it is in each.  If every set of some kind is independent in both, a heaviest set independent in both
bounds their weights.

**Matchings.**  In a bipartite graph, give the edge xy, with x on the left, the unit vector e_x in family 0 and e_y in
family 1.  Equal unit vectors are dependent and distinct ones are not, so the sets independent in both are the
[matchings](#matchings-permanents-and-determinants), no two of whose edges share a vertex.

**Example.**  A [payment scheme](playability.md#payment-schemes) pays each of several numbers with a different maximal
factor, so it is a matching from the numbers to their factors.  For 10, 14, 15 and 21 the edges (s, f), with f a maximal
factor of s, form the 8-cycle 10–2–14–7–21–3–15–5–10, which has two perfect matchings.  Yet
I = {(10, 2), (14, 7), (15, 3)} cannot be extended: each of the other five edges shares a number or a factor with an
edge of I, and paying 15 with 3 strands 21.  So the matchings have maximal sets of 3 and of 4 edges, and the exchange
property fails: *the sets independent in two families are in general not the independent sets of one*.

![The eight-cycle joining 10, 14, 15 and 21 to the primes of their maximal factors, with a matching of three edges that cannot be extended and a perfect matching of four](img/payment-graph-21.svg)

*The 8-cycle drawn on the square of 21.  Thick: the matching I (left) and a perfect matching (right).*

**The split.**  For weights u of either sign, write max_i u for the largest weight of an independent set of family i, of
any size: u(G⁺) for the scan's set G.  For any split w = w₀ + w₁ into real weight functions, every set Y independent in
both has

    w(Y) ≤ max₀ w₀ + max₁ w₁

by [weak duality](#weak-duality-and-lagrangian-relaxation), each term one run of the greedy algorithm.

**Certificates.**  Suppose that for some split, a set I independent in both meets [the optimality
criterion](#the-optimality-criterion) in family 0 for w₀ and in family 1 for w₁.  Then w(I) = max₀ w₀ + max₁ w₁, and no
set independent in both weighs more: the split proves that I is heaviest, whatever algorithm produced I.

**Example.**  In the set {2, 10, 14, 15, 21} the move 2 is paid by 1, and a member of the set cannot pay for another, so
the others are left with the factors 3, 5 and 7: the edges (10, 5), (14, 7), (15, 3), (15, 5), (21, 3) and (21, 7).  Let
the edge (s, f) weigh s.  A heaviest matching is {(14, 7), (15, 5), (21, 3)}, which weighs 50.  Price each factor at 10:
let family 1, one edge per factor, carry the weight 10 on every edge, and let family 0, one edge per number, carry the
excess s − 10.  Then max₁ is 3 · 10 = 30 and max₀ is 0 + 4 + 5 + 11 = 20, so no matching weighs more than 50.

**What can be computed.**  Given the families by tests of independence, *weighted matroid intersection*, a classical
algorithm, finds a heaviest set independent in both, for weights of either sign, in time polynomial in |E| and the cost
of a test (Edmonds 1979; Lawler 1975; Frank 1981).  It builds a heaviest set of d members from one of d − 1 by an
augmenting path, so it finds the heaviest of any size or of at most a given size (Schrijver, Section 41.3 and Theorem
41.7; Korte and Vygen, Sections 13.5 and 13.7), and the matching number of a bipartite graph.  A split that certifies
its answer always exists (the paragraph "For families of vectors" below).

## Weak duality and Lagrangian relaxation

To bound the weight of every set that meets several constraints, split the weight among them, and bound each part by the
best set that meets its own constraint alone.  Every split gives a bound; for two families of vectors the best one is
exact.

**Weak duality.**  Let Γ_i be families of subsets of E, one for each constraint i, and let w = Σ_i w_i be any split of
the weights into real weight functions.  Every set Y that is in all the families has

    w(Y) ≤ Σ_i max_{S ∈ Γ_i} w_i(S).

*Proof.*  w(Y) = Σ_i w_i(Y), and w_i(Y) ≤ max over Γ_i since Y is in Γ_i.  ∎

For two families of vectors it is [the split](#two-families-at-once).  A family of vectors on a part E′ of E also fits:
take the sets Y whose part in E′ is independent, and a weight function 0 outside E′.

**Relaxing a constraint.**  Let Y range over a family Γ, subject to one more linear constraint g(Y) ≤ b, with g a weight
function.  For every multiplier λ ≥ 0, each feasible Y has

    w(Y) ≤ w(Y) + λ(b − g(Y)) = (w − λg)(Y) + λb ≤ max_{S ∈ Γ} (w − λg)(S) + λb.

This is the *Lagrangian relaxation* of the constraint: it moves into the objective at the price λ, and the bound holds
for every λ ≥ 0 (for an equation g(Y) = b, every real λ).  A split is the same idea for constraints that are families.

**Linear programming duality** (quoted; Korte and Vygen, Chapter 3).  For a matrix Θ and vectors b and w, if
{x : Θx ≤ b} is nonempty and bounded, then max {w · x : Θx ≤ b} = min {b · y : y ≥ 0, Θᵀy = w}.

**The best bound.**  Let Δ_i be the convex hull in ℝ^E of the indicator vectors of the sets in Γ_i, and suppose the Δ_i
have a point in common.  Then the least value of the weak-duality bound over all splits is the value of a linear
program,

    max { w · x : x in every Δ_i }.

*Proof.*  Write Δ_i = {x : Θ_i x ≤ b_i}.  By duality, max over Δ_i of w_i · x is min
{b_i · y_i : y_i ≥ 0, Θ_iᵀ y_i = w_i}, so the least bound is min {Σ b_i · y_i : y_i ≥ 0, Σ Θ_iᵀ y_i = w}, which is the
dual of the program above.  ∎  (Guignard and Kim call the split Lagrangian decomposition.)

**For families of vectors.**  The hull of the independent sets of a family of vectors is {x ≥ 0 : x(Y) ≤ rk(Y) for every
Y ⊆ E}, where x(Y) is the sum of x over Y (Edmonds 1970; Schrijver, Corollary 40.2b).  For two families, the hull of the
sets independent in both is the intersection of the two hulls, whose vertices are thus sets independent in both (Edmonds
1970; Schrijver, Corollary 41.12b).  So for two families the best split gives exactly the weight of a heaviest set
independent in both, which is then heaviest in each family for its part.  For three it can give more (Schrijver, Section
41.4).

## Topological orders

A finite directed graph can be put in order, every edge pointing forward, exactly when it has no directed cycle.

**Definitions.**  A *directed graph* has vertices and edges x → y.  A *directed cycle* is a walk of one or more edges
that ends where it starts, so an edge x → x is a cycle.  A *topological order* lists the vertices so that every edge
goes from an earlier vertex to a later one.

**Cycles from out-edges.**  If every vertex of a nonempty finite directed graph has an edge leaving it, the graph has a
directed cycle.  *Proof.*  Start anywhere and keep following edges out.  Some vertex repeats after at most as many steps
as there are vertices, and the walk between its two visits is a cycle.  ∎  The same holds for edges entering.

**The theorem.**  A finite directed graph has a topological order exactly when it has no directed cycle.  *Proof.*
Around a cycle each vertex comes before the next, so an order would put a vertex before itself.  Conversely, suppose
there is no cycle, and induct on the number of vertices; with none, the empty order serves.  Some vertex x has no edge
entering it, by the fact above applied to the graph with its edges reversed.  Order the rest, which has no cycle either,
and put x first.  ∎

**Example.**  In the set {6, 12, 18}, pay 6 with 3, 12 with 4 and 18 with 9, as a [payment
scheme](playability.md#payment-schemes) does, and let x come before y when x's payment divides y, or x divides y.  The
edges are 6 → 12 and 6 → 18 and no more, and the topological orders are 6, 12, 18 and 6, 18, 12.  Both are legal games
on the board of 18, and the four other orders are not.  In the set {6, 10, 15}, paying 6 with 2, 10 with 5 and 15 with 3
gives the cycle 6 → 10 → 15 → 6, so there is no topological order for this payment.

## Matroids

A matroid keeps what families of vectors and forests have in common: all that the greedy algorithm needs.

**Definition.**  A *matroid* on a finite *ground set* E is a family of subsets of E, the *independent sets*, such that
(1) the empty set is independent, (2) every subset of an independent set is independent, and (3) *augmentation*: if I
and I′ are independent and |I| < |I′|, some x in I′ ∖ I has I + x independent.  Given (1) and (2), augmentation is the
exchange property, that within any set the maximal independent subsets have one size: in I ∪ I′, one holding I′ is
larger than I.

**Examples.**  The independent sets of a family of vectors form a *linear* matroid, and the forests of a graph a
*graphic* one (Oxley, Sections 1.1 and 5.1), so the sections above are its main case.  Outside Z and together with Y, a
matroid gives matroids, its *deletion* and *contraction* (Oxley, Section 3.1).

**The greedy theorem.**  [The scan](#the-greedy-algorithm), run on a family F of subsets of E with (1) and (2), keeps x
when G + x is in F.  It finds a heaviest set of F for every nonnegative weight function exactly when F is a matroid
(Schrijver, Theorem 40.1; Edmonds 1971; Oxley, Section 1.8): in a matroid, the proofs of (a) to (d) and of [the
optimality criterion](#the-optimality-criterion) use only (1), (2), the exchange property and the paragraph "The circuit
of I + y", which holds in every matroid (Oxley, Section 1.1).  So distinct positive weights give a matroid one heaviest
independent set.

## References

* J. Oxley, *Matroid Theory*, 2nd edition, Oxford University Press, 2011: Sections 1.1, 1.8, 3.1 and 5.1.
* A. Schrijver, *Combinatorial Optimization: Polyhedra and Efficiency*, Springer, 2003: Theorem 40.1, Corollary 40.2b,
  Section 41.3, Theorem 41.7, Section 41.4 and Corollary 41.12b.
* B. Korte and J. Vygen, *Combinatorial Optimization: Theory and Algorithms*, 2nd edition, Springer, 2002: Chapter 3
  (linear programming duality) and Sections 13.4 (Theorem 13.23), 13.5 and 13.7.  The numbers are those of the 2nd
  edition.
* J. Edmonds, "Submodular functions, matroids, and certain polyhedra", in *Combinatorial Structures and Their
  Applications* (Calgary, 1969), Gordon and Breach, New York, 1970, pp. 69–87.
* J. Edmonds, "Matroids and the greedy algorithm", *Mathematical Programming* 1 (1971) 127–136.
* J. Edmonds, "Matroid intersection", in *Discrete Optimization I*, Annals of Discrete Mathematics 4, North-Holland,
  1979, pp. 39–49.
* E. L. Lawler, "Matroid intersection algorithms", *Mathematical Programming* 9 (1975) 31–56.
* A. Frank, "A weighted matroid intersection algorithm", *Journal of Algorithms* 2 (1981) 328–336.
* M. Guignard and S. Kim, "Lagrangean decomposition: a model yielding stronger Lagrangean bounds", *Mathematical
  Programming* 39 (1987) 215–228.
