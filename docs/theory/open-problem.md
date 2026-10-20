# The open problem

Part of [the theory](../THEORY.md).

The upper-half conjecture says that descending greedy finds the heaviest playable set of sources; it is open, and
certified game by game wherever (♦) holds (Theorem 7).  Run on the sources' instance of game n, descending greedy is
`optimize_mini`, and the set it keeps is U\*.

> **The upper-half conjecture.**  For every game n, no playable subset of the sources' instance outweighs the
> set descending greedy keeps.

Everything in this section locates the exact boundary between what is proven about this conjecture and what remains
open.

## Where exchange breaks

For members with at most two distinct primes the core notion turns geometric: map each factor value to a vertex (plus a
ground vertex for primes and prime powers, whose single factor makes a pendant edge) and each member to the edge joining
its two factors.  A core is exactly a subgraph of minimum degree 2, so **playable ⟺ forest**.  Forests trade well:
removing an edge and adding another never traps you, which is why greedy is bulletproof in that regime.  Members with
three or more distinct primes are hyperedges glued onto the graph, and there the trading breaks.  The smallest failure
is at **n = 2873**, on six members built from {11, 13, 17}:

    A = {1573, 2057, 2431, 2873}                (playable, size 4)
    B = {1573, 1859, 2057, 2197, 2873}          (playable, size 5)

Both members of B∖A jam: A+1859 and A+2197 each contain a core.  Two maximal playable sets of *different sizes* means
that playable sets do not support the exchange argument that justifies greedy in the textbook setting, and the
conjecture cannot be proven that way.

And yet greedy survives: in every three-prime gadget tested, descending greedy still returns the maximum-weight playable
set.  At n=2873 it takes a heavy size-5 set, {1859, 2057, 2197, 2431, 2873}, and never lands on the stuck size-4 A,
because the stuck base is also the *light* one.  The arithmetic is load-bearing: on the same incidence structure, 48 of
the 720 orderings of the six members make greedy fail; the actual member values land in the safe majority, and random
abstract weightings do not.

## The frontier argument, and exactly where it stops

The natural direct proof gets remarkably far: it proves greedy optimal wherever the jam is hyperedge-free.  Compare
greedy's set G to a supposedly heavier playable set T on the same instance, at their largest disagreement d.  The set T
cannot be the one holding d (T would contain greedy's own rejection certificate, by downward closure).  So d ∈ G, T
skips it, and if T is maximum-weight then T+d must be unplayable: every core of T+d contains d (T is playable) and dips
below d, since the part of T+d at or above d is exactly G ∩ {≥ d}, playable *because* step one put d in G, so no core
fits inside it.  Let R be the union of those cores, the 2-core of T+d in hypergraph terms.  Repairs live entirely inside
R (Theorem 6 with K = T+d); a sacrifice t repairs T+d exactly when t lies in **every** minimal core of R (the "victim
law"); and when every member of R has at most two distinct primes, R−d is a forest, R has *exactly one* minimal core (a
forest plus one edge has one cycle), so a below-d victim always exists and the sacrifice T+d−t both restores playability
and gains weight, a contradiction.  **Greedy is provably weight-optimal wherever the jam is hyperedge-free.**

Beyond that regime the single-sacrifice step is **false**.  At n = 9170, d = 6137 = 17·19², the six members {4913, 5491,
6137, 6647, 7429, 8303} (all built from 17, 19, 23; the hyperedge is 7429 = 17·19·23) form an R with **two** minimal
cores through d sharing nothing else.  Their below-d parts are {4913} and {5491}, disjoint, so no single below-d
sacrifice repairs, and every single repair costs at least d.  Greedy is still right there: every d-avoiding way to break
both cores costs more than d (any single alternative victim is heavier than d; any two members sum past n), and the
full-context optimal set does take d: skipping it costs 1,224 points net.  But proving that in general is now visibly a
*non-local, weighted* question: whenever no single lighter sacrifice repairs the jam, the would-be improver's kept
members must be charged for what they lock out elsewhere in the component, not just at the frontier.  That is the open
problem's irreducible form, with n=9170 as the first test any proof must pass.

## The cardinality reformulation

The weights can be removed from the open problem entirely, which turns the conjecture into (★), a statement about sizes.
Taxman's weights are the members themselves, so any weight assignment is consistent with the value ordering, and for
order-consistent weights the classic majorization fact applies: greedy's set outweighs a competitor for *every*
consistent weighting iff it matches or beats the competitor's cardinality within every top segment of the ordering.
Since descending greedy's decisions above any threshold never depend on anything below it (its restriction to {> t} IS
its run on the truncated instance), the conjecture becomes one unweighted statement:

> **(★)** For every game n, on every connected component of the incidence graph of its sources' instance (members the
> numbers in (n/2, n], factors their maximal factors) and every truncation {> t} of one, descending greedy returns a
> **maximum-cardinality** playable subset.

Statement (★) is open, and certified in every game to 100,000 through (♦).  It implies the conjecture by summing the
truncations (layer-cake); by majorization it is also *necessary* if optimality is to hold for every weighting consistent
with the value order, so nothing is lost, or added, in dropping the weights.

Three facts sharpen what a proof of (★) must look like.

*The ordering is the protection, not the magnitudes.*  Every one of the 48 trap orderings on the n=2873 (equally the
n=9170) incidence structure is a pure cardinality upset: greedy is steered into excluding both members of a
factor-sharing pair, keeping 4 where 5 fit.  No magnitude-style axiom blocks that: near-uniform weights realize it, and
"all members exceed n/2" kills none of the 48.  The real numeric order simply never lands among the 48: at n=2873 the
trap needs the sharing pair ranked last, and 11²·13 sits below both sharers.  A proof must therefore use the arithmetic
of the ordering, the *window law*: among the members f·p sharing a factor f, the value order is the order of the
cofactor primes, f·q < f·p iff q < p.  It must, because the incidence structure alone provably permits the trap.

*The component is the unit of proof; sub-instances lie.*  On the bare n=9170 sextet, greedy's 4-of-6 is not maximum
(dropping only d leaves a playable 5-set), and it stays non-maximum with the first ring of attachments added.  What
restores it is 6859 = 19³, farther out in the component, whose demand for the factor 361 tightens the contest.
Cardinality dominance genuinely fails on fragments and holds on the full component.  (Exact searches stay tractable
there because Theorem 6 confines them to the component's residual: n=2873's 377-member component has a 70-member
residual.)

*Matching certifies most components.*  Playable ⊆ matchable gives the free bound |G ∩ K| ≤ max-cardinality ≤ ν(K), the
component's maximum-matching size (the *matching-number bound*); greedy meets ν on 86.8% of the large components,
certifying (★) there with no search.  Where ν overshoots, every exactly-probed case found greedy still optimal: the
surplus was sets that admit a matching but no acyclic one, the cardinality twin of the weight slack in the
Franklín–Moniot bound (first visible at n=21, where a 145-sum set is matchable but opt(21) is 144).

A census of the full 3-prime shape space (parameters in the table) closed the known jam shapes at the ordering level.
Inclusion-minimal self-covering shapes can never trap greedy: removing any member of one leaves no self-covering subset,
so every victim repairs it.  Trap potential lives only in *padded* jams (a core plus outside members competing for the
core's factors), and appears **iff the shape's maximal playable subsets come in two adjacent sizes**: trap potential is
precisely the failure of the exchange property, the n=2873 phenomenon, now a characterization rather than an anecdote.
Of the 22,909 trap orderings found, all but seven are monomially impossible (they demand an exponent-vector inversion
like p²q above pq², which no prime triple realizes) or never realized.  The seven realizable ones were defeated at every
realization tested by a *window guard*: a member (q³ or q²r, in the shape's primes p < q < r) that fixed comparisons
among the shape's own members force into (n/2, n], breaking the trap.  The general lemma this points at, that every
trap ordering requires either a monomial inversion or the absence of a window resident the shape's own realization
forces to exist, is the sharpest known form of why the integers protect greedy.  The guard's *sufficiency* in general
still rests on measurement.

The proof obligation, sharpened: *within one component's jam region (its residual, where Theorem 6 confines every
exclusion), the numeric ordering makes greedy's exclusion count minimal*, with the window law as the engine, and
n=9170's component as the first test any proof must pass.

## The span certificate (♦)

Linear algebra over GF(2) gives a far stronger bound than matching, and a certificate, (♦), that settles the conjecture
game by game.  Give each member its incidence vector: a 1 at each of its maximal factors.

**Lemma (playable sets are independent).**  The incidence vectors of a playable set are linearly independent over GF(2)
(indeed over any field).

*Proof.*  A playable set dismantles completely (Theorems 4 and 5): in some order x₁, x₂, … each xᵢ has a factor that no
later member lists.  In a vanishing sum over a nonempty set of members, the coordinate of the earliest member's private
factor is 1.  ∎

So no playable subset of a set K is larger than the GF(2) rank of K (the *rank bound*).  On members with at most two
distinct primes this is the graphic matroid's rank, exact there; on three-prime members, whose factors pq, pr, qr are
the edges of a triangle on the primes, a dependency is a closed surface mod 2 (every factor an even number of times), so
a minimal dependent set is a core.

**Theorem 7 ((♦) implies (★)).**  Suppose that whenever descending greedy rejects a member x, x's vector is a sum of
vectors of members it kept above x.  Then greedy is maximum-cardinality on every truncation {≥ t} of every component,
and its set outweighs every playable set for every weighting consistent with the order.

*Proof.*  Greedy's kept members above t are playable, hence independent, and span every member above t (the kept ones
trivially, the rejected ones by hypothesis): they are a basis of K_{≥t}, so greedy keeps rank(K_{≥t}) members, which
bounds every playable subset.  The weighted claim is greedy optimality on the linear matroid, whose bases greedy's sets
are.  ∎

Statement (♦) is open, and certified in every game checked: every game to 100,000 and 902 sampled games to 2,000,000
(Evidence, below).  It is stronger than (★): a rejection forced only by an *odd* core (self-covering, with no subset
listing every factor an even number of times) would break it without breaking (★).  Odd cores exist in Taxman (the
exchange failure at n = 2873 needs one, since GF(2)-independent sets do satisfy exchange), but descending greedy has
never rejected because of one.  Checking (♦) is greedy itself plus Gaussian elimination, a few seconds per game at n =
500,000.  The open problem becomes: **prove (♦)**.  The adversarial experiments of the prover (`solver/noframe`, a
branch and bound over sets of moves on the whole board) locate what it needs: it holds under every order of the real
incidence structure that obeys the window law, even adversarially chosen ones, and under the window law reversed.  It
fails as often as a uniformly random order under the window law for a random ranking of the primes, or for the primes
perturbed by more than a few percent, while generalized primes perturbed by 50%, with the members and factors rebuilt
from them, never fail.  The protection is the window law *for the ranking of the primes that also decides which
products lie in the window*.
