# Primer

This primer explains the standard mathematics that the proofs of [the theory](../THEORY.md) use.  It advances nothing:
each fact is in the textbooks, and the primer proves it where the proof is short and teaches something, and otherwise
cites it ([the references](#references) come last).  The examples are Taxman's.  Each section can be read on its own, so
that a proof elsewhere links the one section it uses.  A reader who knows proofs, sets, graphs and linear algebra over
the real numbers can follow all of it.

*Conventions.*  For a set A and an element x, A + x is A ∪ {x} and A − x is A ∖ {x}.  A weight function w on a set E
gives A ⊆ E the weight w(A), the sum of w(a) over a in A.  A number m has a *vector* v(m) over a list of coordinates,
with a 1 at each of its [maximal factors](../GLOSSARY.md#the-game) in the list and a 0 elsewhere (6 has the maximal
factors 3 and 2); the theory calls it the incidence vector ([independence](playability.md#independence)).

## Finite fields and GF(2)

Linear algebra works over every field, and the field decides which sets of vectors are independent.  The field GF(2), in
which 1 + 1 = 0, is the one where two equal vectors cancel.

**Fields.**  A *field* 𝔽 is a set with an addition and a multiplication that obey the usual rules, including that every
element has a negative and every nonzero element has an inverse.  The rational and real numbers are fields; the integers
are not.  For a prime p, the integers mod p form the field GF(p): a residue a ≠ 0 is coprime to p, so ua + vp = 1 for
some integers u and v, and u inverts a.  GF(2) has the elements 0 and 1 with 1 + 1 = 0; because −1 = 1, subtracting is
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
rationals; the example shows the converse failing at p = 2.  *Proof.*  Suppose k vectors are independent over GF(p).
Some k × k submatrix has a determinant that is nonzero mod p, so an integer that p does not divide, so not 0.  ∎

## Matroids

A matroid keeps what linearly independent vectors and the forests of a graph have in common, which is what the greedy
algorithm of a matroid needs.

**Definition.**  A *matroid* M on a finite *ground set* E is a family of subsets of E, the *independent sets*, such that
(1) the empty set is independent, (2) every subset of an independent set is independent, and (3) *augmentation*: if I
and J are independent and |I| < |J|, some x in J ∖ I has I + x independent.  A set that is not independent is
*dependent*.

**Terms.**

* A *basis* is a maximal independent set.  If bases B and B′ had |B| < |B′|, augmentation would extend B, so all bases
  have one size.  The same argument inside any A ⊆ E shows that the maximal independent subsets of A have one size, the
  *rank* r(A) of A, and that every independent subset of A has at most r(A) members.
* A *circuit* is a minimal dependent set.  A *loop* is an element x with {x} dependent.  A *coloop* is an element in
  every basis; if x is a coloop and I − x is independent, so is I, since I − x extends to a basis, which holds x.

**The circuit of I + y.**  If I is independent and I + y is dependent, then I + y holds exactly one circuit, and it
contains y (Oxley, Section 1.1).  An element x of I lies in this circuit exactly when I − x + y is independent.
*Proof.*  Call the circuit C.  If x is in C, a circuit inside I − x + y would be inside I + y, so would be C, which is
impossible since C holds x; so I − x + y has no circuit.  If x is not in C, then C lies inside I − x + y, which is
dependent.  ∎

### Linear and graphic matroids

Give each element of a finite set E a vector over a field 𝔽.  The sets whose vectors are linearly independent form a
matroid, the *linear matroid* of the vectors (Oxley, Section 1.1).  The edges of a graph form a matroid whose
independent sets are the forests, the edge sets with no cycle: the *graphic matroid*.  Its circuits are the cycles, and
its rank is the number of vertices less the number of components.  It is the linear matroid over GF(2) of the vectors
e_u + e_v, one for each edge uv (Oxley, Section 5.1).  *Proof.*  A nonempty forest has a vertex of degree 1, where the
sum of its vectors has a 1; the edges of a cycle sum to zero.  ∎

**Example.**  A number pq with two different primes has the maximal factors p and q, so its vector is e_p + e_q: the
edge pq of a graph on the primes.  In the game of 21, the numbers 6, 10, 14, 15 and 21 are the edges 2–3, 2–5, 2–7, 3–5
and 3–7 on the primes 2, 3, 5, 7.  The circuits are the triangles {6, 10, 15} and {6, 14, 21} and the 4-cycle
{10, 14, 15, 21}.  The rank is 3, and there are 8 bases.  In the game of 22, the number 22 is the edge 2–11, and no
other product of two primes on the board involves 11, so adding 22 to a forest never closes a cycle: 22 is a coloop.

### Deletion, contraction and direct sums

* *Deletion.*  The matroid M ∖ X has the ground set E ∖ X and the independent sets of M that avoid X.
* *Contraction.*  For an independent set P, the matroid M/P has the ground set E ∖ P, and a set I is independent in M/P
  exactly when I ∪ P is independent in M.  It is a matroid: augment I ∪ P from J ∪ P in M, and the new element lies in
  J ∖ I.  (The general definition is in Oxley, Section 3.1.)
* *Direct sum.*  If E splits into E₁ and E₂ with a matroid on each, the sets I with I ∩ E₁ and I ∩ E₂ independent form a
  matroid, the *direct sum*, whose *blocks* are the two.  Augment inside a block where I is smaller than J.

For vectors, both are linear algebra.

* *Contraction is a quotient.*  The matroid M/P of vectors over 𝔽 is the linear matroid of the same vectors modulo the
  span of P's.  *Proof.*  The vectors of I are dependent modulo the span exactly when some vanishing combination of the
  vectors of I ∪ P has a coefficient of I that is not 0; and since P is independent, every nontrivial vanishing
  combination does.  ∎  If P's vectors are unit vectors, working modulo their span deletes those coordinates.
* *Direct sum is a direct sum of spaces.*  If the vectors of E₁ lie in a subspace V₁, those of E₂ in a subspace V₂, and
  only the zero vector lies in both, then the linear matroid of all the vectors is the direct sum of the two, since a
  vanishing combination splits into one in V₁ and one in V₂.

**Example.**  Contract the edge 6, which is 2–3, in the graph above.  Its ends merge, 10 and 15 both join the merged
vertex to 5 and become parallel, and so do 14 and 21; every other pair stays independent.  A graphic matroid is the
direct sum of those of its components.

## The greedy algorithm of a matroid

In a matroid, scanning the elements from heaviest to lightest and keeping each one that stays independent finds a
heaviest independent set, for any weights, and a set is heaviest exactly when no single change improves it.

*Not the other greedy procedures.*  This is the greedy algorithm of a matroid, which the primer also calls the scan.  It
is not the Taxman strategy that is often called greedy, taking the largest legal move, and not the theory's *descending
greedy* ([sources.md](sources.md#descending-greedy)), which runs the same scan on playable sets.  Those are not the
independent sets of a matroid: in the game of 4, {2} and {3, 4} are playable, but neither 3 nor 4 can join 2, since both
then need the 1 that 2 needs.  So the theorem below does not cover descending greedy.

**The scan.**  Let M be a matroid on E and w a weight function on E.  List E in decreasing order of weight, with ties in
any order.  Start with B empty, and for each element x in turn, put x in B if B + x is independent.

**Example.**  Take the graphic matroid above on 6, 10, 14, 15, 21, and let w be the number itself.  The scan keeps 21
(the edge 3–7), 15 (3–5) and 14 (2–7).  It rejects 10, which would close the 4-cycle, and 6, which would close the
triangle {6, 14, 21}.  So B = {21, 15, 14}, which weighs 50; the other seven bases weigh 30, 35, 37, 39, 42, 45 and 46.

**What the scan finds.**  Let M be a matroid on E, w a real weight function on E, and B the scan's set for any order of
ties.

* (a) B is a basis.
* (b) For every real t, B ∩ {w ≥ t} is a maximal independent subset of {w ≥ t}.  So every independent set J has
  |J ∩ {w ≥ t}| ≤ |B ∩ {w ≥ t}|.
* (c) B is a heaviest basis, and B⁺, the part of B with positive weight, is a heaviest independent set among the
  independent sets of every size.
* (d) If the weights are distinct, B is the only heaviest basis, and if they are distinct and not 0, B⁺ is the only
  heaviest independent set.

*Proof.*  The idea is that the scan settles every element of weight at least t before it looks at a lighter one.

(b) Put A = {w ≥ t}.  Every element of A comes before every element outside it.  Take x in A ∖ B.  The scan rejected x,
so the set kept before x, plus x, was dependent, and that kept set lies in B ∩ A because only elements of A came
earlier.  So (B ∩ A) + x is dependent.  Nothing in A can join B ∩ A, which is independent as a subset of B, so B ∩ A is
a maximal independent subset of A.  It has r(A) members, and an independent J ∩ A has at most r(A).

(a) Take t at the least weight, so that A = E.

(c) By the identity of [Sums over thresholds](#sums-over-thresholds), the weight of a set is a positive combination of
its counts |· ∩ {w ≥ t}|, and (b) bounds each count of an independent J by that of B.  A basis J has as many members as
B, so adding a constant to every weight changes w(J) − w(B) by nothing, and we add one that makes every weight positive.
For the last claim, w(J) ≤ w(J⁺) since dropping members of weight at most 0 does not lower the weight, and the identity
over the elements of positive weight bounds w(J⁺) by w(B⁺).

(d) With distinct weights, equality forces every count of J to equal that of B, since every coefficient is positive, and
passing from one threshold to the next adds one element, so J has the elements that B has.  For B⁺, weights that are not
0 also keep members of weight at most 0 out of a heaviest J.  ∎

Only matroids have this property: if the scan finds a heaviest set for every nonnegative weight function, a family
closed under subsets is a matroid (Schrijver, Theorem 40.1; Edmonds 1971; Oxley, Section 1.8).

### The optimality criterion

A set can be certified heaviest by checking single changes, whatever algorithm produced it.

**The criterion.**  Let M be a matroid on E, w a real weight function on E, and I an independent set.  Then I is a
heaviest independent set, among the independent sets of every size, exactly when

1. w(x) ≥ 0 for every x in I;
2. w(y) ≤ 0 for every y outside I with I + y independent;
3. w(y) ≤ w(x) for every y outside I with I + y dependent and every x in I that lies in the circuit of I + y.

*Proof.*  The idea is that each condition guards against one improving change, and when none is left the proof of (c)
applies to I.  If I is heaviest, the conditions hold: a violation of (1) gives the heavier set I − x, one of (2) gives
the heavier independent set I + y, and one of (3) gives I − x + y, independent by the fact on the circuit of I + y and
heavier by w(y) − w(x).

Now suppose (1) to (3).  For t > 0 we claim that I ∩ {w ≥ t} is a maximal independent subset of {w ≥ t}.  Take y in
{w ≥ t} ∖ I.  By (2), I + y is dependent, since w(y) ≥ t > 0.  Let C be its circuit.  By (3), each x in C − y has
w(x) ≥ w(y) ≥ t, so C − y lies in I ∩ {w ≥ t}, and (I ∩ {w ≥ t}) + y holds C and is dependent.  So nothing joins
I ∩ {w ≥ t}, and the claim holds.  As in (b), every independent J has |J ∩ {w ≥ t}| ≤ |I ∩ {w ≥ t}| for t > 0.  By (1),
w(I) = w(I⁺), and the argument of (c) gives w(J) ≤ w(J⁺) ≤ w(I⁺) = w(I).  ∎

**Example.**  In the graphic matroid above, with w the number itself, I = {21, 15, 14} meets the criterion.  Condition
(1) holds and (2) is empty, since I is a basis.  For (3), the circuit of I + 10 is {10, 14, 15, 21}, whose members in I
weigh at least 10, and the circuit of I + 6 is {6, 14, 21}, whose members in I weigh more than 6.  The basis
J = {10, 15, 21} fails (3): the circuit of J + 14 holds 10, which weighs less than 14, and J − 10 + 14 = I is heavier.

*Dummy coloops.*  Equivalently, add r coloops of weight 0, with r the rank of M, and cut the result to rank r.  The
independent sets of M, padded with dummies to r elements, are the bases of the result, and (1), (2) and (3) are the
exchanges of an element for a dummy, a dummy for an element, and an element for an element: the classical test that no
single exchange improves a basis (Korte and Vygen, Section 13.4, Theorem 13.23).

## Matroid intersection

Two matroids on one ground set can be combined by asking for sets that are independent in both.  These sets are not a
matroid, but a heaviest one can still be found in polynomial time.

**Definition.**  Let M and M′ be matroids on the same set E.  A *common independent set* is independent in both.

**Matchings.**  Split E into classes.  The sets with at most one element in each class form a matroid, a *partition
matroid*: if |I| < |J|, then J meets more classes than I does, and an element of J in a class that I misses can join I.
In a bipartite graph, let M allow one edge at each vertex on the left and M′ one edge at each vertex on the right.  The
common independent sets are the *matchings* of the graph.

**Example.**  A [payment scheme](playability.md#payment-schemes) pays each of several numbers with a different maximal
factor, so it is a matching from the numbers to their factors.  For 10, 14, 15 and 21 the edges (m, f), with f a maximal
factor of m, form the 8-cycle 10–2–14–7–21–3–15–5–10, which has two perfect matchings.  Yet
I = {(10, 2), (14, 7), (15, 3)} cannot be extended: each of the other five edges shares a number or a factor with an
edge of I, and paying 15 with 3 strands 21.  So the matchings have maximal sets of 3 and of 4 edges, and augmentation
fails for I and a perfect matching: *common independent sets are in general not a matroid*.

**What can be computed.**

* *The size.*  The largest common independent set has min over A ⊆ E of r(A) + r′(E ∖ A) members, where r and r′ are the
  rank functions (Edmonds' matroid intersection theorem, 1970; Schrijver, Theorem 41.1).
* *The weight.*  Give the matroids by tests of independence.  A heaviest common independent set, for weights of either
  sign and sets of any size, can be found in time polynomial in |E| and the cost of a test (Edmonds 1979; Lawler 1975;
  Frank 1981).  The algorithms build a heaviest set of k elements from one of k − 1 by an augmenting path, and the best
  over k is the heaviest of any size (Schrijver, Section 41.3 and Theorem 41.7; Korte and Vygen, Sections 13.5 and
  13.7).  The matching number of a bipartite graph is the size of the largest common independent set of its two
  partition matroids.

**Truncation.**  For a matroid M and a number k, the sets of M with at most k elements form a matroid, the *truncation*
of M to k.  *Proof.*  If |I| < |J| ≤ k, the element x of J that can join I in M gives I + x with at most |J| ≤ k
elements.  ∎  The heaviest common independent sets of M and M′ with at most k elements are those of M and the truncation
of M′, so a limit on the size costs nothing.

## Weak duality and Lagrangian relaxation

To bound the weight of every set that satisfies several constraints, split the weight among the constraints, and bound
each part by the best set that satisfies its own constraint alone.  Every split gives a valid bound, and for two
matroids the best split gives the exact answer.

For a matroid M and weights c, write max_M c for the weight of a heaviest independent set of M, of any size; the scan
computes it.

**Weak duality.**  Let 𝓕₁, …, 𝓕_K be families of subsets of E, and let w = w₁ + … + w_K be any split of the weights into
K real weight functions.  Every set T that is in all the families has

    w(T) ≤ Σ_k max_{S ∈ 𝓕_k} w_k(S).

*Proof.*  w(T) = Σ_k w_k(T), and w_k(T) ≤ max over 𝓕_k since T is in 𝓕_k.  ∎

For two matroids M and M′ and a split w = w₀ + w₁, every common independent set T has w(T) ≤ max_M w₀ + max_{M′} w₁.  A
matroid on a part E_k of E also fits: take the sets S with S ∩ E_k independent in it, and a weight function that is 0
outside E_k.

**Certificates.**  Suppose I is a common independent set, and for some split I meets the criterion of [The optimality
criterion](#the-optimality-criterion) in M for w₀ and in M′ for w₁.  Then w(I) = w₀(I) + w₁(I) = max_M w₀ + max_{M′} w₁,
and weak duality says that no common independent set weighs more.  The split proves that I is heaviest, whatever
algorithm produced I.

**Example.**  Take the numbers 10, 14, 15, 21 and the factors 3, 5, 7 (the situation when 2 is itself a move of the set,
paid by 1), and let the edge (m, f) weigh m.  A heaviest matching is {(14, 7), (15, 5), (21, 3)}, which weighs 50.
Price each factor at 10: let M′, one edge per factor, carry the weight 10 on every edge, and let M, one edge per number,
carry the excess m − 10.  Then max_{M′} is 10 for each of the three factors, 30 in all, and max_M is 0 + 4 + 5 + 11 =
20, so no matching weighs more than 50.

**Relaxing a constraint.**  Let T range over a family 𝓕, subject to one more linear constraint a(T) ≤ b.  For every
multiplier μ ≥ 0, each feasible T has

    w(T) ≤ w(T) + μ(b − a(T)) = (w − μa)(T) + μb ≤ max_{S ∈ 𝓕} (w − μa)(S) + μb.

This is the *Lagrangian relaxation* of the constraint: it moves into the objective at the price μ, and the bound holds
for every μ ≥ 0 (for an equation a(T) = b, every real μ).  A split is the same idea for constraints that are families.

**The best bound.**  Let P_k be the convex hull in ℝ^E of the indicator vectors of the sets in 𝓕_k, and suppose the P_k
have a point in common.  Then the least value of the weak-duality bound over all splits is the value of a linear
program,

    max { w · x : x in every P_k }.

*Proof.*  Write P_k = {x : B_k x ≤ d_k}.  By linear programming duality, max over P_k of w_k · x is min
{d_k · y_k : y_k ≥ 0, B_kᵀ y_k = w_k}, so the least bound is min {Σ d_k · y_k : y_k ≥ 0, Σ B_kᵀ y_k = w}, which is the
dual of the program above.  ∎  (Guignard and Kim call the split Lagrangian decomposition.)

**For matroids.**  The hull of a matroid's independent sets is {x ≥ 0 : x(A) ≤ r(A) for every A ⊆ E} (Edmonds 1970;
Schrijver, Corollary 40.2b).  For two matroids, the hull of the common independent sets is the intersection of their two
hulls, so that its vertices are common independent sets (Edmonds 1970; Schrijver, Corollary 41.12b).  So for two
matroids the best split gives the weight of a heaviest common independent set exactly.  For three matroids it can give
more (Schrijver, Section 41.4).

## Matchings, permanents and determinants

The permanent of a 0/1 matrix counts the perfect matchings of its graph, the determinant counts them with signs, and mod
2 the signs vanish.  So a square 0/1 matrix is nonsingular over GF(2) exactly when its graph has an odd number of
perfect matchings.

**Definitions.**  A graph is *bipartite* when its vertices split into two sides and every edge joins the sides.  A
*matching* is a set of edges, no two with a vertex in common; it is *perfect* when the sides have k vertices each and
the matching has k edges.  The *matching number* ν of a graph is the size of its largest matching.  The *matrix* of a
bipartite graph has a row for each vertex x on one side, a column for each vertex y on the other, and a 1 where xy is an
edge and a 0 elsewhere.

**Permanent and determinant.**  Fix an order of the rows and of the columns of a k × k matrix A.  A bijection σ from the
rows to the columns has a *sign* ±1, that of the permutation it makes.  The *permanent* and the *determinant* are

    per(A) = Σ_σ Π_x A[x, σ(x)]        det(A) = Σ_σ sign(σ) Π_x A[x, σ(x)].

For a 0/1 matrix, the product for σ is 1 exactly when every pair (x, σ(x)) is an edge, that is, when σ is a perfect
matching of the graph, and is 0 otherwise.  So per(A) is the number of perfect matchings, and det(A) is the number of
perfect matchings of sign +1 less the number of sign −1.  Another order changes every sign together, so whether two
matchings have the same sign does not depend on the order.

**What the theory uses.**

* *Mod 2.*  Since −1 = 1 in GF(2), det(A) mod 2 equals per(A) mod 2, so the matrix is nonsingular over GF(2) exactly
  when its graph has an odd number of perfect matchings.
* *Over the rationals.*  The matrix is nonsingular exactly when the signed count is not 0.  An odd number of perfect
  matchings gives an odd signed count, so a nonzero one; an even number can give a signed count of 0 or not.
* *Rank and matching number.*  Over every field, the rank of the matrix is at most ν of its graph.  *Proof.*  A nonzero
  k × k minor has a nonzero term in its expansion, and a nonzero term is a matching with k edges.  ∎

**Example.**  The numbers 6, 10 and 15 against the factors 2, 3, 5 have the matrix with the rows 1 1 0, 1 0 1 and 0 1 1.
Its graph has two perfect matchings, {(6, 2), (10, 5), (15, 3)} and {(6, 3), (10, 2), (15, 5)}, and each swaps two of
the columns, so both have sign −1.  (These are the payment schemes of the set {6, 10, 15}.)  So per = 2 and det = −2: an
even count, nonsingular over the rationals and singular modulo 2.  For the 4-cycle 10, 14, 15, 21 against 2, 3, 5, 7 the
two perfect matchings have opposite signs, so per = 2 and det = 0.

## Topological orders

A finite directed graph can be put in order, every edge pointing forward, exactly when it has no directed cycle.

**Definitions.**  A *directed graph* has vertices and edges u → v.  A *directed cycle* is a sequence of edges v₀ → v₁ →
… → v_k = v₀ with k ≥ 1, so an edge v → v is a cycle.  A *topological order* lists the vertices so that every edge goes
from an earlier vertex to a later one.

**Cycles from out-edges.**  If every vertex of a finite directed graph has an edge leaving it, the graph has a directed
cycle.  *Proof.*  Start anywhere and keep following edges out.  Some vertex repeats within |V| + 1 vertices, and the
walk between its two visits is a cycle.  ∎  The same holds for edges entering.

**The theorem.**  A finite directed graph has a topological order exactly when it has no directed cycle.  *Proof.*
Around a cycle each vertex comes before the next, so an order would put a vertex before itself.  Conversely, suppose
there is no cycle, and induct on the number of vertices; with none, the empty order serves.  Some vertex v has no edge
entering it, by the fact above applied to the graph with its edges reversed.  Order the rest, which has no cycle either,
and put v first.  ∎

**Example.**  In the set {6, 12, 18}, pay 6 with 3, 12 with 4 and 18 with 9, and let a move come before another when its
payment divides the other or it divides the other.  The edges are 6 → 12 and 6 → 18 and no more, and the topological
orders are 6, 12, 18 and 6, 18, 12.  Both are legal games on the board of 18, and the four other orders are not.  In the
set {6, 10, 15}, paying 6 with 2, 10 with 5 and 15 with 3 gives the cycle 6 → 10 → 15 → 6, so there is no topological
order for this payment.

## Sums over thresholds

A weighted sum is a positive combination of counts above thresholds.  So a set with at least as many members above every
threshold weighs at least as much under every increasing weighting.

**The identity.**  Let E be a finite set and w a weight function on E with the distinct positive values v₁ > v₂ >
… > v_m, and put v_{m+1} = 0.  For every S ⊆ E,

    w(S) = Σ_j (v_j − v_{j+1}) · |S ∩ {w ≥ v_j}|.

*Proof.*  A member of S of weight v_i is counted in the terms j ≥ i, and their coefficients add up to v_i − v_{m+1} =
v_i.  ∎  (A layer cake: the weight is stacked in layers of thickness v_j − v_{j+1}.)

Every coefficient is positive.  When E is a set K of numbers and w is strictly increasing, the sets {w ≥ v_j} are the
sets {≥ c} of the members c of K.

**Dominance.**  Let K be a finite set of numbers, and S and T subsets with |S ∩ {≥ t}| ≤ |T ∩ {≥ t}| for every t.  Then
w(S) ≤ w(T) for every strictly increasing positive weight function w on K, with equality only if S = T.  *Proof.*  Every
coefficient is positive, so w(S) ≤ w(T) term by term.  Equality forces the counts to agree at every member c of K.  The
count at c exceeds the count at the next larger member (or 0, for the largest) by 1 exactly when c is in the set, so S
and T have the same members.  ∎  Since S has at least i members at or above t exactly when its i-th largest member is at
least t, the hypothesis says that |S| ≤ |T| and the i-th largest member of S is at most the i-th largest of T, for every
i ≤ |S|.

**The converse.**  If |S ∩ {≥ t}| > |T ∩ {≥ t}| for some t, some strictly increasing positive weight function has
w(S) > w(T).  *Proof.*  Take w(x) = εx + 1 for x ≥ t and w(x) = εx for x < t.  Then w(S) − w(T) =
(|S ∩ {≥ t}| − |T ∩ {≥ t}|) + ε(ΣS − ΣT), which is positive for small ε.  ∎  So dominance for every such weight function
needs dominance at every threshold.

**Example.**  Among the numbers 10, 14, 15, 21, let S = {10, 15, 21} and T = {14, 15, 21}.  The counts at the thresholds
21, 15, 14, 10 are 1, 2, 2, 3 for S and 1, 2, 3, 3 for T, so T dominates S.  With w(x) = x the identity gives 6 · 1 +
1 · 2 + 4 · 2 + 10 · 3 = 46 for S and 6 · 1 + 1 · 2 + 4 · 3 + 10 · 3 = 50 for T.  They are two bases of [the graphic
matroid](#linear-and-graphic-matroids) above, and T is the scan's.

## References

* J. Oxley, *Matroid Theory*, 2nd edition, Oxford University Press, 2011: Sections 1.1 (independent sets, circuits and
  the circuit of I + y, linear matroids), 1.8 (the greedy algorithm), 3.1 (contraction) and 5.1 (graphic matroids).
* A. Schrijver, *Combinatorial Optimization: Polyhedra and Efficiency*, Springer, 2003: Chapter 40 (the greedy
  algorithm, Theorem 40.1; the independent set polytope, Corollary 40.2b) and Chapter 41 (matroid intersection: Theorem
  41.1, Section 41.3 and Theorem 41.7, Section 41.4 and Corollary 41.12b).
* B. Korte and J. Vygen, *Combinatorial Optimization: Theory and Algorithms*, 2nd edition, Springer, 2002: Sections 13.4
  (Theorem 13.23), 13.5 and 13.7.  The numbers are those of the 2nd edition.
* J. Edmonds, "Submodular functions, matroids, and certain polyhedra", in *Combinatorial Structures and Their
  Applications* (Calgary, 1969), Gordon and Breach, New York, 1970, pp. 69–87.
* J. Edmonds, "Matroids and the greedy algorithm", *Mathematical Programming* 1 (1971) 127–136.
* J. Edmonds, "Matroid intersection", in *Discrete Optimization I*, Annals of Discrete Mathematics 4, North-Holland,
  1979, pp. 39–49.
* E. L. Lawler, "Matroid intersection algorithms", *Mathematical Programming* 9 (1975) 31–56.
* A. Frank, "A weighted matroid intersection algorithm", *Journal of Algorithms* 2 (1981) 328–336.
* M. Guignard and S. Kim, "Lagrangean decomposition: a model yielding stronger Lagrangean bounds", *Mathematical
  Programming* 39 (1987) 215–228.
