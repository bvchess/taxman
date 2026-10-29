# Bounds from splits

Part of [the primer](../primer.md), whose [conventions](../primer.md#conventions) hold here.

## Two families at once

**Summary:** For the sets independent in two families of vectors at once, the greedy algorithm can fail, but a classical
algorithm finds a heaviest one, and a split of the weights proves its value.

**Definition.**  Let each member of a finite set E have a vector in family 0 and one in family 1; a set is *independent
in both* when it is in each.  If every set of some kind is independent in both, a heaviest set independent in both
bounds their weights.

**Matchings.**  In a bipartite graph, give the edge xy, with x on the left, the unit vector e_x in family 0 and e_y in
family 1.  Equal unit vectors are dependent and distinct ones are not, so the sets independent in both are the
[matchings](graphs.md#matchings-permanents-and-determinants).

**Example.**  A [payment scheme](../playability.md#payment-schemes) pays each of several numbers with a different
maximal factor, so it is a matching from the numbers to their factors.  For 10, 14, 15 and 21 the edges (s, f), with f a
maximal factor of s, form the 8-cycle 10–2–14–7–21–3–15–5–10, which has two perfect matchings.  Yet I = {(10, 2), (14,
7), (15, 3)} cannot be extended: each of the other five edges shares a number or a factor with an edge of I, and paying
15 with 3 strands 21.  So the matchings have maximal sets of 3 and of 4 edges, and the exchange property fails: *the
sets independent in two families are in general not the independent sets of one, nor of any matroid*
([Matroids](greedy.md#matroids)).

![The eight-cycle joining 10, 14, 15 and 21 to the primes of their maximal factors, with a matching of three edges that cannot be extended and a perfect matching of four](../img/payment-graph-21.svg)

*The 8-cycle drawn on the square of 21.  Thick: the matching I (left) and a perfect matching (right).*

**The split.**  For weights u of either sign, write max_i u for the largest weight of an independent set of family i:
u(G⁺) for the scan's set G.  For any split w = w₀ + w₁ into real weight functions, every set Y independent in both has

    w(Y) ≤ max₀ w₀ + max₁ w₁

by [weak duality](#weak-duality-and-lagrangian-relaxation), proved in the next section, each term one run of the greedy
algorithm.

**Certificates.**  Suppose that for some split, a set I independent in both meets [the optimality
criterion](greedy.md#the-optimality-criterion) in family 0 for w₀ and in family 1 for w₁.  The criterion makes I
heaviest in each family, so w(I) = w₀(I) + w₁(I) = max₀ w₀ + max₁ w₁: by the split, no set independent in both weighs
more, whatever algorithm produced I.

**Example.**  In the set {2, 10, 14, 15, 21} the move 2 is paid by 1, and a member of the set cannot pay for another, so
the others are left with the factors 3, 5 and 7: the edges (10, 5), (14, 7), (15, 3), (15, 5), (21, 3) and (21, 7).  Let
the edge (s, f) weigh s.  A heaviest matching is {(14, 7), (15, 5), (21, 3)}, which weighs 50.  Price each factor at 10:
let family 1, one edge per factor, carry the weight 10 on every edge, and let family 0, one edge per number, carry the
excess s − 10.  Then max₁ is 3 · 10 = 30 and max₀ is 0 + 4 + 5 + 11 = 20, so no matching weighs more than 50.

**What can be computed.**  Given the families by tests of independence, *weighted matroid intersection*, a classical
algorithm, finds a heaviest set independent in both, for weights of either sign, in time polynomial in |E| and the cost
of a test (Edmonds 1979; Lawler 1975; Frank 1981).  It builds a heaviest set of d members from one of d − 1 by an
*augmenting path*, a chain of exchanges that adds one member and keeps the set independent in both, so it finds the
heaviest of any size or of at most a given size (Schrijver, Section 41.3 and Theorem 41.7; Korte and Vygen, Sections
13.5 and 13.7), and the matching number of a bipartite graph.  A split that certifies its answer always exists (the
paragraph "For families of vectors" below).

## Weak duality and Lagrangian relaxation

**Summary:** To bound the weight of every set that meets several constraints, split the weight among them, and bound
each part by the best set that meets its own constraint alone.  Every split gives a bound.

**Weak duality.**  Let Γ_i be families of subsets of E, one for each constraint i, and let w = Σ_i w_i be any split of
the weights into real weight functions.  Every set Y that is in all the families has

    w(Y) ≤ Σ_i max_{S ∈ Γ_i} w_i(S).

*Proof.*  w(Y) = Σ_i w_i(Y), and w_i(Y) ≤ max over Γ_i since Y is in Γ_i.  ∎

For two families of vectors it is [the split](#two-families-at-once).  A family of vectors on a part E′ of E also fits:
take the sets Y whose part in E′ is independent, and a weight function 0 outside E′.

**Relaxing a constraint.**  Let Y range over a family Γ, subject to one more linear constraint g(Y) ≤ b, with g a weight
function.  For every multiplier λ ≥ 0, each feasible Y has

    w(Y) ≤ w(Y) + λ(b − g(Y)) = (w − λg)(Y) + λb ≤ max_{S ∈ Γ} (w − λg)(S) + λb.

This is the *Lagrangian relaxation* of the constraint: it moves into the objective at the price λ (for an equation
g(Y) = b, λ may be any real).  A split is the same idea for constraints that are families.

**Linear programming duality** (quoted; Korte and Vygen, Chapter 3).  For a matrix Θ and vectors b and w, if
{x : Θx ≤ b} is nonempty and bounded, then max {w · x : Θx ≤ b} = min {b · y : y ≥ 0, Θᵀy = w}.

**The best bound.**  The *indicator vector* of a set Y ⊆ E has a 1 at each member of Y and a 0 elsewhere.  The *convex
hull* of some vectors is the set of their averages with nonnegative weights that sum to 1, the smallest convex set that
holds them, and its *vertices* are those of the vectors that are not such an average of the others.  Let Δ_i be the
convex hull in ℝ^E of the indicator vectors of the sets in Γ_i, and suppose the Δ_i have a point in common.  Then the
least value of the weak-duality bound over all splits is the value of a linear program,

    max { w · x : x in every Δ_i }.

*Proof.*  Write Δ_i = {x : Θ_i x ≤ b_i}.  By duality, max over Δ_i of w_i · x is min
{b_i · y_i : y_i ≥ 0, Θ_iᵀ y_i = w_i}, so the least bound is min {Σ b_i · y_i : y_i ≥ 0, Σ Θ_iᵀ y_i = w}, which is the
dual of the program above.  ∎  (Guignard and Kim call the split Lagrangian decomposition.)

**For families of vectors.**  The hull of the independent sets of a family of vectors is {x ≥ 0 : x(Y) ≤ rk(Y) for every
Y ⊆ E}, where x(Y) is the sum of x over Y, and for two families, the hull of the sets independent in both is the
intersection of the two hulls, whose vertices are thus sets independent in both.  Both hold for matroids in general
(Edmonds 1970; Schrijver, Corollaries 40.2b and 41.12b).  So for two families the best split gives exactly the weight of
a heaviest set independent in both, which is then heaviest in each family for its part.  Under a limit of k members,
replace one family by its sets of at most k: they keep the exchange property ([at most k
members](linear-algebra.md#linear-independence)), so form a matroid, and [the optimality
criterion](greedy.md#the-optimality-criterion) and the exact split apply ([the section on
matroids](greedy.md#matroids)).  For three families the split can give more (Schrijver, Section 41.4).
