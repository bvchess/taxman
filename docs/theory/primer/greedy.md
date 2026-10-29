# The greedy algorithm and matroids

Part of [the primer](../primer.md), whose [conventions](../primer.md#conventions) hold here.

## Sums over thresholds

**Summary:** A weighted sum is a positive combination of counts above thresholds, so a set with at least as many members
above every threshold weighs at least as much.

**The identity.**  Let E be a finite set and w a positive weight function on E, and write {w ≥ ρ} for the set of members
of weight at least ρ.  For each value ρ of w, let ρ⁺ be the next smaller value, or 0 for the smallest.  For every Y ⊆ E,

    w(Y) = Σ_ρ (ρ − ρ⁺) · |Y ∩ {w ≥ ρ}|,    summed over the values ρ of w.

*Proof.*  A member of Y of weight ρ₀ is counted in the terms for ρ ≤ ρ₀, whose coefficients add up to ρ₀, since each
cancels against the next.  ∎

**Dominance.**  Let w be a positive weight function on a finite set E with distinct values on distinct elements, and let
S and S′ be subsets of E with |S ∩ {w ≥ t}| ≤ |S′ ∩ {w ≥ t}| for every t.  Then w(S) ≤ w(S′), with equality only if
S = S′.  For a set of numbers, w is any strictly increasing positive weighting, and {w ≥ w(x)} is the set of members
that are at least x.  *Proof.*  Every coefficient of the identity is positive, so w(S) ≤ w(S′) term by term.  Equality
forces equal counts at every value ρ.  The step from the next larger value to ρ adds one element, and the count rises by
1 exactly when that element is in the set, so S and S′ have the same elements.  ∎  Positivity matters: for E = {1, 2}
and w(x) = x − 10, the set {1, 2} has at least as many members above every threshold as {1}, but weighs −17 against −9.

**The converse.**  Write {≥ t} for the numbers that are at least t.  Let S and S′ be sets of numbers with |S ∩ {≥ t}| >
|S′ ∩ {≥ t}| for some t.  Then some strictly increasing positive weighting w has w(S) > w(S′).  *Proof.*  Take w(x) = εx
+ 1 for x ≥ t and w(x) = εx for x < t.  Then w(S) − w(S′) = (|S ∩ {≥ t}| − |S′ ∩ {≥ t}|) + ε(ΣS − ΣS′), which is
positive for small ε.  ∎

**Example.**  Among the numbers 10, 14, 15, 21, let S = {10, 15, 21} and S′ = {14, 15, 21}.  The counts at the
thresholds 21, 15, 14, 10 are 1, 2, 2, 3 for S and 1, 2, 3, 3 for S′, so S′ dominates S, and with w(x) = x the identity
gives 46 for S and 50 for S′.  They are two spanning trees of the graph in [Forests](linear-algebra.md#forests), and S′
is the scan's.

## The greedy algorithm

**Summary:** On a family of vectors, the greedy algorithm (the scan) takes the members from heaviest to lightest and
keeps each that is independent of those kept.  It finds a heaviest basis for any weights and, once its members of weight
at most 0 are dropped, a heaviest independent set.

*Three kinds of greedy.*  The scan is neither the Taxman strategy often called greedy, which takes the largest legal
move, nor the theory's *descending greedy* ([sources.md](../sources.md#descending-greedy)), which runs the same scan on
playable sets.

*Playable sets are not independent sets of vectors, nor of any matroid* ([Matroids](#matroids)).  Already in the game of
4, {2} and {3, 4} are playable, but neither 3 nor 4 can join 2: 3 would need the 1 that 2 needs, and 4's only maximal
factor is 2 itself.  So the maximal playable subsets of {2, 3, 4} differ in size, against [the exchange
property](linear-algebra.md#linear-independence), and the theorem below does not cover descending greedy.

**The scan.**  Let E be a family of vectors and w a weight function on E.  List E in decreasing order of weight, with
ties in any order.  Start with G empty, and for each member x in turn, put x in G if G + x is independent.

**Example.**  Take the edges 6, 10, 14, 15 and 21 of [the example](linear-algebra.md#forests), and let w be the number
itself.  The scan keeps 21 (the edge 3–7), 15 (3–5) and 14 (2–7).  It rejects 10, which would close the 4-cycle, and 6,
which would close the triangle {6, 14, 21}.  So G = {21, 15, 14}, which weighs 50; the other seven bases weigh 30, 35,
37, 39, 42, 45 and 46.

![The edges 6, 10, 14, 15 and 21 on the primes 2, 3, 5, 7, with the scan keeping 21, 15 and 14 and rejecting 10 and 6](../img/graphic-21.svg)

*The graph, each edge a number between its two primes.  Thick: the edges the scan keeps.  Dashed: the edges it rejects.*

**What the scan finds.**  Let E be a family of vectors, w a real weight function on E, and G the scan's set for any
order of ties.  Let G⁺ be the part of G with positive weight, and likewise I⁺ the part of any set I.

* (a) G is a basis.
* (b) For every real t, G ∩ {w ≥ t} is a maximal independent subset of {w ≥ t}, that is, it spans {w ≥ t}.  So every
  independent set I has |I ∩ {w ≥ t}| ≤ |G ∩ {w ≥ t}|.
* (c) G is a heaviest basis, and G⁺ is a heaviest independent set.
* (d) If the weights are distinct, G is the only heaviest basis, and if they are distinct and not 0, G⁺ is the only
  heaviest independent set.

*Proof.*  The idea is that the scan settles every member of weight at least t before it looks at a lighter one.

(b) Put Y = {w ≥ t}, and take x in Y ∖ G.  The scan rejected x, so the set kept before x, plus x, was dependent, and
that kept set lies in G ∩ Y because only members of Y came earlier.  So (G ∩ Y) + x is dependent: G ∩ Y, independent
as a subset of G, spans Y and is a maximal independent subset of Y.  It has rk(Y) members, and an independent I ∩ Y has
at most rk(Y).

(a) Take t at the least weight, so that Y = E.

(c) By the identity of [Sums over thresholds](#sums-over-thresholds), a set of positive weights has a weight that is a
positive combination of its counts |· ∩ {w ≥ t}|, and (b) bounds each count of an independent I by that of G.  Bases
have one size, so adding a constant to every weight leaves w(I) − w(G) unchanged; add one that makes all weights
positive.  For G⁺: w(I) ≤ w(I⁺), and the identity over the members of positive weight bounds w(I⁺) by w(G⁺).

(d) Make the weights positive as in (c); they stay distinct.  By (b) and Dominance, w(I) ≤ w(G) for every basis I, with
equality only if I = G.  For G⁺: a heaviest I has w(I) = w(I⁺), so no member of weight below 0, and none of weight 0, so
I = I⁺; Dominance on the members of positive weight, with (b), gives I = G⁺.  ∎

The weights must not be 0 for the last claim: with two members x and z of the vectors e₁ and e₂, w(x) = 1 and w(z) = 0,
the sets {x} and {x, z} are both heaviest.  The section on matroids says on which other families the scan works.

### The optimality criterion

**Summary:** A set can be certified heaviest by checking single changes, whatever algorithm produced it.

**The criterion.**  Let E be a family of vectors, w a real weight function on E, and I an independent set.  Then I is a
heaviest independent set exactly when

1. w(x) ≥ 0 for every x in I;
2. w(y) ≤ 0 for every y outside I with I + y independent;
3. w(y) ≤ w(x) for every y outside I with I + y dependent and every x in I that lies in the circuit of I + y.

*Proof.*  The idea is that each condition guards against one improving change, and when none is left the proof of (c)
applies to I.  If I is heaviest, the conditions hold: a violation of (1) gives the heavier set I − x, one of (2) gives
the heavier independent set I + y, and one of (3) gives I − x + y, independent by [the circuit of
I + y](linear-algebra.md#linear-independence) and heavier by w(y) − w(x).

Now suppose (1) to (3).  For t > 0 we claim that I ∩ {w ≥ t} spans {w ≥ t}.  Take y in {w ≥ t} ∖ I.  By (2), I + y is
dependent, since w(y) ≥ t > 0.  Let H be its circuit.  By (3), each x in H − y has w(x) ≥ w(y) ≥ t, so H − y lies in
I ∩ {w ≥ t}, and (I ∩ {w ≥ t}) + y holds H and is dependent.  As in (b), every independent I′ has
|I′ ∩ {w ≥ t}| ≤ |I ∩ {w ≥ t}| for t > 0.  By (1), w(I) = w(I⁺), and the argument of (c) gives
w(I′) ≤ w(I′⁺) ≤ w(I⁺) = w(I).  ∎

**Example.**  Among the forests of [the example](linear-algebra.md#forests), with w the number itself, I = {21, 15, 14}
meets the criterion.  Condition (1) holds and (2) is empty, since I is a basis.  For (3), the circuits of I + 10 and I +
6 are {10, 14, 15, 21} and {6, 14, 21}, and their members in I are larger than 10 and 6.  The basis I′ = {10, 15, 21}
fails (3): the circuit of I′ + 14 holds 10, which is less than 14, and I′ − 10 + 14 = I is heavier.

*Dummy members.*  Equivalently, add rk(E) *dummies* of weight 0, each with a private coordinate, and keep the
independent sets with at most rk(E) members.  The independent sets of E, padded with dummies, are the bases of the
result, and (1), (2) and (3) are the exchanges of a member for a dummy, a dummy for a member, and a member for a member:
the classical test that no single exchange improves a basis (Korte and Vygen, Section 13.4, Theorem 13.23).

## Matroids

**Summary:** A matroid keeps what families of vectors and forests have in common: all that the greedy algorithm needs.

**Definition.**  A *matroid* on a finite *ground set* E is a family of subsets of E, the *independent sets*, such that
(1) the empty set is independent, (2) every subset of an independent set is independent, and (3) *augmentation*: if I
and I′ are independent and |I| < |I′|, some x in I′ ∖ I has I + x independent.  Given (1) and (2), augmentation is the
exchange property: within any Y ⊆ E the maximal independent subsets have one size, the *rank* rk(Y), as augmentation
would extend a smaller one inside Y; conversely, in I ∪ I′ one holding I′ is larger than I.

**Terms.**  A set that is not independent is *dependent*; a *basis* is a maximal independent set, and a *circuit* a
minimal dependent one.  An independent subset of Y extends to a maximal one inside Y, so has at most rk(Y) members.  An
independent I *spans* a set when each of its members y is in I or makes I + y dependent.  If I is independent and I + y
is dependent, I + y holds exactly one circuit H, the *circuit of I + y* (Oxley, Section 1.1), and a member x of I lies
in H exactly when I − x + y holds no circuit, that is, is independent: H lies inside I − x + y unless x is in H, and
then a circuit inside I − x + y would lie inside I + y, so would be H, which holds x.

**Examples.**  The independent sets of a family of vectors form a *linear* matroid, and the forests of a graph a
*graphic* one (Oxley, Sections 1.1 and 5.1).  Outside Z and together with an independent Y, a matroid gives matroids,
its *deletion* and *contraction* (Oxley, Section 3.1).

**The greedy theorem.**  [The scan](#the-greedy-algorithm), run on a family F of subsets of E with (1) and (2), keeps x
when G + x is in F.  It finds a heaviest set of F for every nonnegative weight function exactly when F is a matroid
(Schrijver, Theorem 40.1; Edmonds 1971; Oxley, Section 1.8): in a matroid, the proofs of (a) to (d) and of [the
optimality criterion](#the-optimality-criterion) hold with these terms in place of those of [linear
independence](linear-algebra.md#linear-independence).  So distinct positive weights give a matroid one heaviest
independent set.
