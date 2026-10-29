# Fields and linear independence

Part of [the primer](../primer.md), whose [conventions](../primer.md#conventions) hold here.

## Finite fields and GF(2)

**Summary:** GF(2), the *Galois field* of two elements, is arithmetic mod 2, where 1 + 1 = 0.  Over it two equal vectors
cancel, so a sum of 0/1 vectors records only whether each coordinate is listed an odd or an even number of times; the
field decides which sets of vectors are independent.

**Fields.**  A *field* 𝔽 is a set with an addition and a multiplication that obey the usual rules, including that every
element has a negative and every nonzero element has an inverse.  The rational and real numbers are fields; the integers
are not.  For a prime p, the integers mod p form the field GF(p), the *Galois field* of p elements, named after Évariste
Galois, who founded the study of finite fields: a residue a ≠ 0 is coprime to p, so αa + βp = 1 for some integers α and
β, and α inverts a.  GF(2) has the elements 0 and 1 with 1 + 1 = 0; because −1 = 1, subtracting is adding, and *signs
vanish*.  GF(3) has 0, 1 and 2 = −1, so 1 + 1 = 2 and 2 · 2 = 1.  The theory works mostly over GF(2): its vectors have
the entries 0 and 1, which need no signs there, and a vector over GF(2) is a string of bits, cheap to test for
independence.  [The independence lemma (P10)](../playability.md#the-independence-lemma) holds over every field, and
[GF(3)](../frame-bounds.md#the-gf3-proposition) serves where a sign matters.

**Linear algebra over a field.**  A linear combination that is zero is *vanishing*, and *nontrivial* when some
coefficient is not 0.  A list of vectors over 𝔽 is *linearly independent* when it has no nontrivial vanishing
combination, and its *rank* is the dimension of its span.  The *unit vector* e_x has a 1 at the coordinate x and a 0
elsewhere.  Row reduction divides only by nonzero elements, so what it proves holds over every field: a square matrix
has independent rows exactly when its determinant is not 0, and the rank of a matrix is the size of its largest square
submatrix with a nonzero determinant.  Over GF(2) the only nonzero coefficient is 1, so a list of 0/1 vectors is
dependent exactly when some nonempty sublist has an even number of 1s at every coordinate.

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

**Summary:** A family gives each member a vector, and its independent sets are those whose vectors are linearly
independent; rank, bases, circuits and independence together with a set describe which members can stand together.

A *family of vectors* is a finite set E of *members*, each with a vector over a field 𝔽, two members possibly with the
same one.  A set I ⊆ E is *independent* when its members' vectors, one for each, are linearly independent, and
*dependent* otherwise.  Every subset of an independent set is independent, the empty set included.

**Rank, span and bases.**  The *rank* rk(Y) of Y ⊆ E is the rank of Y's vectors, and a set I *spans* a member, or a set,
when the span of I's vectors holds its vectors.  For an independent I and y outside it, I + y is dependent exactly when
I spans y.  So an independent subset of Y is maximal exactly when it spans Y, and then its vectors are a basis of the
span of Y's: all the maximal independent subsets of Y have rk(Y) members, and no independent one has more.  This is the
*exchange property*.  Equivalently, if I and I′ are independent and |I| < |I′|, some x in I′ ∖ I has I + x independent,
as I's span cannot hold all of I′'s vectors.  A *basis* is a maximal independent subset of E.

**Circuits.**  A *circuit* is a minimal dependent set.  A nontrivial vanishing combination of its vectors has no
coefficient 0, or a smaller set would be dependent, so over GF(2) a circuit is a nonempty set whose vectors sum to 0, no
smaller nonempty part of which does.  A member with the vector 0 is a circuit alone, and two with the same nonzero
vector form one.  A member x has a *private coordinate* when its vector is not 0 at a coordinate where every other
member's is 0; that coordinate gives x the coefficient 0 in every vanishing combination, so x joins every independent
set, lies in no circuit and is in every basis.

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

**Summary:** Over GF(2) the edges of a graph are vectors, and a set of edges is independent exactly when it has no
cycle.

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
game of 35, the edges 6 = 2–3 and 35 = 5–7 share no prime, so {6, 35} splits into one-member blocks on the coordinates
{2, 3} and {5, 7}, and each member has a private coordinate.
