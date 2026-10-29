# Matchings and orders

Part of [the primer](../primer.md), whose [conventions](../primer.md#conventions) hold here.

## Matchings, permanents and determinants

**Summary:** The permanent of a 0/1 matrix counts the perfect matchings of its graph, the determinant counts them with
signs, and mod 2 the signs vanish.

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

**Example.**  The rows of 6, 10 and 15 in [the first example](linear-algebra.md#finite-fields-and-gf2) form a matrix
whose graph has two perfect matchings, {(6, 2), (10, 5), (15, 3)} and {(6, 3), (10, 2), (15, 5)}, and each swaps two of
the columns, so both have sign −1.  (These are the payment schemes of the set {6, 10, 15}.)  So per = 2 and det = −2: an
even count, nonsingular over the rationals and singular modulo 2.

## Topological orders

**Summary:** The vertices of a directed graph can be listed with every edge pointing forward exactly when the graph has
no directed cycle.

**Definitions.**  A *directed graph* has vertices and edges x → y.  A *directed cycle* is a walk of one or more edges
that ends where it starts, so an edge x → x is a cycle.  A *topological order* lists the vertices so that every edge
goes from an earlier vertex to a later one.

**Cycles from out-edges.**  If every vertex of a nonempty finite directed graph has an edge leaving it, the graph has a
directed cycle.  *Proof.*  Start anywhere and keep following edges out.  Some vertex repeats after at most as many steps
as there are vertices, and the walk between its two visits is a cycle.  ∎

**The theorem.**  A finite directed graph has a topological order exactly when it has no directed cycle.  *Proof.*
Around a cycle each vertex comes before the next, so an order would put a vertex before itself.  Conversely, suppose
there is no cycle, and induct on the number of vertices; with none, the empty order serves.  Some vertex x has no edge
entering it, by the fact above applied to the graph with its edges reversed.  Order the rest, which has no cycle either,
and put x first.  ∎

**Example.**  In the set {6, 12, 18}, pay 6 with 3, 12 with 4 and 18 with 9, as a [payment
scheme](../playability.md#payment-schemes) does, and let x come before y when x's payment divides y, or x divides y.
The edges are 6 → 12 and 6 → 18 and no more, and the topological orders are 6, 12, 18 and 6, 18, 12.  Both are legal
games on the board of 18, and the four other orders are not.  In the set {6, 10, 15}, paying 6 with 2, 10 with 5 and 15
with 3 gives the cycle 6 → 10 → 15 → 6, so there is no topological order for this payment.
