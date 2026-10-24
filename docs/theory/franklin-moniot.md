# Franklín and Moniot's paper

Part of [the theory](../THEORY.md).

Franklín and Moniot's central theorem is [the playability theorem (P2)](playability.md#the-playability-theorem) in the
language of matchings, and [the parity bound (W1)](parity-bound.md#the-parity-bound) lies between their
characterization of playable sets and their bound.  One count reads all three: a set is playable when some set of
factors receives exactly one of its payment schemes, independent over GF(2) when some set receives an odd number, and
within their bound when some set receives at least one ([the counting theorem (C2)](#the-counting-theorem)).

The paper is A. F. Franklín and R. K. Moniot, "The difficulty of beating the Taxman", *Discrete Applied Mathematics* 339
(2023) 166–171 (arXiv:2211.00461; the theorem numbers below are the arXiv version's).  It is the published foundation of
the heuristics' yardsticks.  It uses no linear algebra: everything below about GF(2) is this file's.

Every result here is proved, and [the table of statements](statements.md#the-statements) holds their standing.  The
game's words are the [glossary's](../GLOSSARY.md); instances, payment schemes, precedence, peel, cores, odd cores and
the vectors u(s) are [playability.md's](playability.md#payment-schemes).  The primer explains
[GF(2)](primer.md#finite-fields-and-gf2) and [permanents and determinants](primer.md#matchings-permanents-and-determinants).

## What the paper proves

They play Taxman on any finite poset graded by a rank function, with weights: a move takes an element that still has
something below it in play, scores its weight, and removes it and everything below it.  Taxman is the divisibility
order on 1..n, ranked by Ω and weighted by value.  Their graph G joins two elements when one covers the other; for
divisibility, when one is a maximal factor of the other.  So G is the *maximal-factor graph*, with the edge f–c
weighing c.  For a matching of G, a cycle of G is *alternating* when its edges are alternately in the matching and out
of it, and *flat* when its vertices lie in two consecutive ranks.

* Theorem 1: optimal play is a maximum-weight matching of G with no flat alternating cycle.  The proof works set by set
  in both directions.  From a play it matches each move with the largest number the move taxes, which is a maximal
  factor of the move; from such a matching it builds a play of exactly its upper ends.  So what it proves is: *a set S
  is playable iff it is the set of upper ends of a matching of G with no flat alternating cycle.*
* Theorem 2: the game on a graded poset is NP-hard, even with unit weights.  On two ranks every cycle is flat, and the
  game becomes the maximum alternating-cycle-free matching of a bipartite graph, NP-complete by H. Müller
  ("Alternating cycle-free matchings", *Order* 7 (1990) 11–21).
* Theorem 3: the born-free matching has no flat alternating cycle.  Theorem 4: restricted to the primes up to 5, it
  takes more than half the pot for N ≥ 847.  Theorem 5: the game can be won for every N ∉ {1, 3}, by the born-free
  matching except at 7 and 13, which get games by hand.  Theorem 6: a matching with no flat alternating cycle is put in
  order in O(N log N) by removing degree-one vertices, rank pair by rank pair: [peel](playability.md#peel)'s forced
  moves, on a matching already found.
* Their bounds, unnumbered: without the flat-cycle condition, the maximum-weight matching of G is an upper bound, the
  *Franklín–Moniot bound*; breaking its cycles with a feedback-arc-set heuristic gives a lower one.  The paper has no
  sharper upper bound.

*Payment schemes are matchings.*  Identify a payment scheme with the matching of G made of its pairs {x, pay(x)}.  The
payment schemes of a set S ⊆ {2, …, n}, that is of its own instance, are then exactly the matchings of G whose upper
ends are the members of S.  A member cannot be a lower end, since it is the upper end of its own edge, so the lower ends
lie in the pool of S's own instance: maximal factors of members, outside S.  The Franklín–Moniot bound is therefore the
weight of the heaviest set that has a payment scheme.

*The count.*  The *list matrix* of S has a row for each member, a column for each factor of the pool of S's own
instance, and a 1 where the member lists the factor; its rows are the vectors u(s) of
[independence](playability.md#independence).  For a set T of |S| factors of the pool, per(S, T) is the number of
payment schemes of S whose set of payments is T.  It is the permanent of the square matrix made of the columns T of the
list matrix, since a term of the permanent is 1 exactly when it pays each member with a distinct factor of T that the
member lists.

## The bridge lemma

A cycle of the precedence and a flat alternating cycle are one walk read in two graphs, and either one lets the scheme be
turned into another with the same payments.

**Lemma C1 (the bridge lemma).**  Let pay be a payment scheme of a set S ⊆ {2, …, n}, with T its set of payments.  The
following are equivalent:

1. pay has a flat alternating cycle in G;
2. the precedence of pay is cyclic;
3. another payment scheme of S has the set of payments T.

*Proof.*  Around a flat alternating cycle each member pays with a factor that the next member lists, which is an edge of
the precedence; and flipping the cycle hands each member the factor of the one before it.

1. *The vertices of a flat alternating cycle.*  Each vertex of an alternating cycle meets one edge of pay on it, so it is
   a member of S or a payment.  Let the cycle lie in the ranks k − 1 and k.  The edge of pay at a member x goes to
   pay(x), one rank below x, and lies on the cycle; so x is in rank k and pay(x) in rank k − 1.  A vertex in rank k − 1
   is therefore a payment and one in rank k a member.  So each edge of the cycle outside pay joins a payment pay(a) to a
   member b ≠ a of which it is a maximal factor.
2. *1 ⇒ 2.*  Read a flat alternating cycle as a, pay(a), b, pay(b), ….  By step 1, pay(a) divides b, which is an edge a
   → b of the precedence.  So the members of the cycle, in order, close a cycle of the precedence.
3. *2 ⇒ 1.*  By [the flat-cycle lemma (P3)](playability.md#the-flat-cycle-lemma), every cycle of the precedence lies in
   one layer k, and on each of its edges a → b, b lists pay(a).  So a cycle a₁ → a₂ → ⋯ → a_m → a₁ of distinct members
   gives the cycle a₁, pay(a₁), a₂, pay(a₂), …, a_m, pay(a_m) of G.  Its edges a_i–pay(a_i) are in pay, and its edges
   pay(a_i)–a_{i+1} are not, since a_{i+1} is paired with its own payment.  So it alternates, and it lies in the ranks
   k − 1 and k.
4. *1 ⇒ 3.*  Exchange the edges of pay on a flat alternating cycle for the cycle's other edges.  Each member of the cycle
   is then paired with the payment before it, a factor of the pool that it lists by step 1, and the other members keep
   theirs.  So this is a payment scheme other than pay, with the same payments T.
5. *3 ⇒ 1.*  Let pay′ ≠ pay have the payments T.  Every member and every factor of T meets one edge of pay and one of
   pay′, so the edges in exactly one of the two form disjoint cycles that alternate for pay, and there is at least one.
   On such a cycle an edge of pay′ joins pay′(b) = pay(a) to b, and pay(a) is a maximal factor of both a and b, so a and
   b lie in one rank.  Going around, the members of the cycle lie in one rank and its payments one rank below: the cycle
   is flat.  ∎

### Their Theorem 1 is the playability theorem

By C1, a scheme with no flat alternating cycle is a scheme with an acyclic precedence.  So their Theorem 1, read set by
set, and P2 are the same statement about the same objects, and their proof takes P2's steps.  Their play-to-matching
direction is the argument of [the lifting lemma (P1)](playability.md#the-lifting-lemma): they take the largest number a
move taxes, P2 a maximal factor above the divisor the move surrenders.  Their matching-to-play direction repeatedly plays
a member with no other payment below it, which is a topological order of the precedence.  Their rank count is the
flat-cycle lemma (P3).  Theirs is the more general statement, for every finite graded poset; P2 adds that every
topological order of the precedence is a legal game.

## The counting theorem

**Theorem C2 (the counting theorem).**  Let S ⊆ {2, …, n}.  Then:

1. S has a payment scheme iff per(S, T) ≥ 1 for some T;
2. S is independent with its own coordinates over GF(2) iff per(S, T) is odd for some T;
3. S is playable iff per(S, T) = 1 for some T.

So a playable set is independent over GF(2), and an independent set has a payment scheme.  The best score opt(n), the
parity bound (W1) and the Franklín–Moniot bound are the weights of the heaviest subsets of {2, …, n} that meet the three
conditions in turn.

*Proof.*  Each condition is a demand on the count of schemes onto some T: at least one, an odd number, exactly one.

1. *Schemes.*  A payment scheme with the payments T is one of the per(S, T) schemes counted.
2. *Independence.*  S is independent with its own coordinates iff its vectors u(s), the rows of its list matrix, are
   independent ([independence](playability.md#independence)).  The rows are independent iff some |S| columns T make a
   nonsingular square matrix, since the row rank is the column rank.  Over GF(2) the determinant of a matrix of 0s and
   1s is its permanent mod 2 ([the primer](primer.md#matchings-permanents-and-determinants)), and the permanent here is
   per(S, T).
3. *Playability.*  By P2, S is playable iff some payment scheme has an acyclic precedence.  By C1, a scheme has one iff
   no other scheme has its payments T, that is, iff per(S, T) = 1.
4. *The bounds.*  The best score opt(n) is the heaviest playable set, as in the proof of W1.  The parity bound is the
   heaviest set independent in both parity matroids, which are the sets independent with their own coordinates
   ([what the parity bound misses](parity-bound.md#what-the-parity-bound-misses)).  The Franklín–Moniot bound is the
   heaviest set with a payment scheme, as above.  ∎

Both implications are strict.  The triangle {6, 10, 15} has a payment scheme and is dependent: its two schemes pay 6, 10
and 15 with 2, 5 and 3 or with 3, 2 and 5, both onto {2, 3, 5}, and v(6) + v(10) + v(15) = 0.  No board below 15 has
such a set.  The set {12, 18, 20, 30, 45} is independent and not playable ([the first odd core](#the-first-odd-core)),
and no board below 45 has such a set.

*The house example, n = 21, in both languages.*  The set S = {10, 14, 15, 21} is a cycle of four edges on the primes:
2–5, 2–7, 3–5 and 3–7.  In G, S and the primes form the 8-cycle 2–10–5–15–3–21–7–14–2, all in the ranks 1 and 2.  The
payment schemes of S are its two perfect matchings, each the other flipped around that 8-cycle, which is flat and
alternating for both.  By their Theorem 1, S is not playable.  In P2's terms, the scheme 10:2, 14:7, 21:3, 15:5 has the
precedence cycle 10 → 14 → 21 → 15 → 10, and the other scheme the reverse.  The count per(S, {2, 3, 5, 7}) = 2 is even,
and v(10) + v(14) + v(15) + v(21) = 0: all four have Ω = 2, so S is dependent in the parity matroid M₁.  A heaviest set
with a payment scheme on this board, of weight 145, is {10, 12, 14, 15, 16, 18, 19, 20, 21}, which contains S.  Its pool
is {1, …, 9}, its only possible set of payments, onto which it has two schemes, both carrying the same 8-cycle.  So the
Franklín–Moniot bound reads 145 at n = 21, where the parity bound reads opt(21) = 144.

*The published foundation.*  The counting theorem gives the [independence lemma (P10)](playability.md#the-independence-lemma)
over GF(2) a second proof, from their Theorem 1, the bridge lemma and the determinant mod 2.  [The peel-order lemma
(P9)](playability.md#the-peel-order-lemma) gives more: a playable set's list matrix, in the order of a peel, is
triangular with a nonzero diagonal, so P10 holds over every field.  A frame of the frame model is their game on a poset
of two ranks, factors below the moves that list them, in which every move must be taken; the same argument, frame by
frame, is [the signed frame theorem (B1)](frame-bounds.md#the-signed-frame-theorem).

## What the parity bound keeps

Fix a scheme pay with the payments T.  By steps 4 and 5 of C1's proof, the schemes onto T are pay flipped along the sets
of pairwise disjoint flat alternating cycles of pay, the empty set giving pay itself.  So per(S, T) counts those sets.
Their characterization asks for a scheme with no flat alternating cycle: per(S, T) = 1.  The parity bound asks only that
the count be odd for some T.  It keeps the count mod 2, the part of the condition that is linear, hence a matroid
intersection, hence computable in polynomial time, and it drops uniqueness.  So the sets that the parity bound admits
and the game does not are those whose count is odd onto some T but never 1: by C2 they are independent and not
playable, and each holds an [odd core](playability.md#odd-cores).

GF(2) is the field for this.  Over the rationals, independence asks for a nonzero signed count, which an odd count
implies, since a sum of an odd number of terms ±1 is odd.  The triangle {6, 10, 15} has two schemes of the same sign,
with determinant −2 ([the primer's example](primer.md#matchings-permanents-and-determinants)): independent over the
rationals, dependent over GF(2).

## The two-factor proposition

When every member lists at most two factors, the members are the edges of a graph, and playable, independent and
acyclic are one condition.

**Proposition C3 (the two-factor proposition).**  Let an instance have members that each list at most two factors.  Its
*graph* has as vertices the factors of the pool and one more vertex, the *ground*, and as edges the members: a member
that lists two factors joins them, a member that lists one joins it to the ground, and a member that lists none is a
loop at the ground.  These are equivalent:

1. the instance is playable;
2. the members' lists, as vectors of 0s and 1s on the pool, are linearly independent over GF(2);
3. the graph is a forest.

In particular, a set S ⊆ {2, …, n} whose members each have at most two maximal factors outside S is playable iff it is
independent with its own coordinates over GF(2), and it holds no [odd core](playability.md#odd-cores).

*Proof.*  A dependency over GF(2) is a set of edges with even degree at every factor, which holds a cycle; and a forest
peels from its leaves.

1. *2 ⇔ 3.*  A vanishing sum of lists over GF(2) is a nonempty set Z of members that lists each factor an even number of
   times: in the graph, each factor has even degree in Z.  The degrees in Z sum to twice its number of edges, so the
   ground's degree is even too.  A graph with an edge and every degree even holds a cycle, since a walk that never
   reuses an edge can leave every vertex it enters until it returns to one it has visited.  Conversely, a cycle meets
   each vertex an even number of times, so its members' lists sum to 0; a loop is a member whose list is 0.  So the
   lists are independent iff the graph has no cycle.
2. *1 ⇒ 2.*  A playable instance peels completely ([the peel theorem (P4)](playability.md#the-peel-theorem)), so its
   lists are independent over every field, by the peel-order lemma (P9).
3. *3 ⇒ 1.*  Let the graph be a forest with an edge.  A tree with an edge has two leaves or more, vertices of degree 1,
   and at most one of them is the ground; so some factor is listed by one member alone, and peel has a back move.  The
   move takes out that member and that factor, which leaves a forest.  So peel pairs every member, and the instance is
   playable by P4.
4. *Sets.*  The own instance of such a set S has members that list at most two factors, so S is playable iff its
   vectors u(s) are independent, which is independence with its own coordinates.  A core of S's own instance is not
   playable in it ([the core theorem (P7)](playability.md#the-core-theorem)), so by 2 ⇒ 1 its lists are dependent: it is
   no odd core.  ∎

On such instances, too, the count of schemes is a power of two.  Take a scheme onto T.  A component of the graph with a
cycle has at least as many edges as vertices, being connected, and at most as many edges as factors, since the scheme
pays each with a distinct factor; so it has no ground and exactly one cycle.  Its trees pay from their leaves inward,
which leaves the cycle's edges to pay the cycle's vertices, one way round or the other.  A tree pays in one way onto its
part of T.  So per(S, T) is 2^c for the c components with a cycle, odd only for a forest, and on such sets the parity
bound loses nothing.  This is the graph-like case of [where exchange breaks](sources.md#where-exchange-breaks), of [the
bottom-layers theorem (F3)](frame-model.md#the-bottom-layers-theorem) and of [the frame
bound](frame-bounds.md#the-frame-bound).  What the parity bound drops are the odd cores, and by C3 each of them has a
member with three or more maximal factors outside it.

## The first odd core

**Proposition C4 (the first-odd-core proposition).**  Every set S ⊆ {2, …, 44} that is independent with its own
coordinates over GF(2) is playable.  So no board n ≤ 44 has an odd core, and the parity bound equals opt(n) for every n
≤ 44.  On the board of 45, the set {12, 18, 20, 30, 45} is an odd core.

*Proof.*  Where peel stalls on an independent set, some member must list three live factors, each wanted by another
member, and below 45 no number can.

1. *Peel keeps independence.*  Let S be independent and not playable, so a run of peel on its own instance stalls with
   members left (P4).  Its moves keep the rows of the members left, restricted to the live factors, independent.  A
   back move deletes the only row with a 1 at some factor, with that factor, and a dependency of the rows left, which
   are 0 at that factor, would be one before.  A front move deletes a row whose only live entry is at its factor, with
   that factor, and a dependency of the rows left would, with that row added or not, be one before.  So no live member
   lists nothing, since its row would be 0.
2. *The stall.*  At the stall each live member lists at least two live factors, or peel would have a front move, and
   each live factor that a live member lists is listed by at least two, or peel would have a back move.
3. *A member with three.*  Suppose every live row had exactly two entries.  The entries number two per row and at least
   two per column, while independence allows no more rows than columns; so every column holds exactly two, and the rows
   are the edges of a graph in which every vertex has degree 2.  The sum of those rows is then 0, a dependency.  So some
   live member lists three or more live factors, each listed by another member of S.
4. *Below 45.*  The numbers up to 44 with three or more maximal factors are 30 = 2·3·5 and 42 = 2·3·7.  The maximal
   factor 15 of 30 is a maximal factor of no other number up to 44, the next being 45, and the maximal factor 21 of 42
   of none, the next being 63.  So neither can be the member of step 3, and no independent S ⊆ {2, …, 44} fails to be
   playable.  The parity bound is the weight of an independent set, which is then playable, so it is at most opt(n);
   it is at least opt(n) by W1.
5. *The board of 45.*  The members of {12, 18, 20, 30, 45} list, among the factors 4, 6, 9, 10 and 15, none of which is
   a member: 12 lists 4 and 6, 18 lists 6 and 9, 20 lists 4 and 10, 30 lists 6, 10 and 15, and 45 lists 9 and 15.
   Every factor is listed twice or more, so the set is a core, and not playable (P7).  Its only possible set of payments
   is all five factors.  If 30 pays 6, 10 or 15, the path 10–4–6–9–15 of the edges 20, 12, 18 and 45 then pays in one
   way, so per = 3, which is odd: by C2 the lists are independent, and the core is odd.  ∎

In their language, every scheme of {12, 18, 20, 30, 45} has two flat alternating cycles, both through 30 and never
disjoint, so the sets of disjoint cycles are three: none, and either one.

The parity bound stays exact past the reach of C4.  As measured, it equals opt(n) for every n up to 77, and first
exceeds it at 78, with 1,992 against 1,991 ([the table of statements](statements.md#the-statements)).  So odd cores exist
from 45 on, and none weighs in before 78.

## The hardness

Their Theorem 2 makes the game NP-hard on posets of two ranks.  There a playable set is the upper side of a *uniquely
restricted matching*: a matching with no alternating cycle, equivalently the only perfect matching on its vertices (M. C.
Golumbic, T. Hirst and M. Lewenstein, "Uniquely restricted matchings", *Algorithmica* 31 (2001)).  The bridge lemma, the
counting theorem and the split by parity use nothing beyond a graded poset, with the parity of the rank for the parity of
Ω.  So the parity matroids bound their general game too, in polynomial time.  On two ranks with unit weights the bound is
the GF(2) rank of the bipartite adjacency matrix, which lies between the uniquely restricted matching number and the
matching number.  Three consequences follow.

* Unless P = NP, no bound computable in polynomial time equals the best score on every graded poset.  For the parity
  bound the difference is the odd cores.
* The bound is exact on posets in which every element covers at most two others, since C3 holds there word for word.
  Their game is then solved in polynomial time by weighted matroid intersection.  The hardness needs elements that cover
  three or more others: in Taxman, the numbers with three or more distinct primes, which is where the odd cores live.
* The paper's hardness is for the general game, and the complexity of Taxman itself is open; nothing here settles it.
  But the sources' instance, members above the factors they list, and the frames are instances of the two-rank problem
  that their Theorem 2 shows hard in general.  And the gap that the frame bound leaves is the odd cores of frames 1 to 3
  ([the anatomy of the gap](../../solver/notebook/search/the-anatomy-of-the-gap-2026-10/README.md)).  So wherever (♦)
  or the parity bound is exact in Taxman, the exactness comes from the arithmetic, not from the structure, which matches
  what the [adversarial experiments on (♦)](../../solver/noframe/docs/star.md#adversarial-search-what-protects-greedy)
  found.

Three questions stay open ([open questions](statements.md#open-questions)): a condition between "odd" and "exactly one"
that is computable in polynomial time and exact for Taxman, which by their Theorem 2 cannot be exact on every graded
poset unless P = NP; a description of the odd cores that weigh in, from 78 on; and the complexity of Taxman.

## The check by computation

A program of the heuristics, [fm_relation.py](../../heuristics/evaluation/fm_relation.py), checks the results of this
file on small boards, in under two minutes on one thread.  It decides playability by the rules alone, and their
condition literally, as a scheme with no flat alternating cycle in G, and compares both with peel, the counts per(S, T)
and independence in the parity matroids.  It runs on every set of moves of the boards up to 22, on random sets of the
boards 30 to 120 grown around members with three or more distinct primes, and on the sets of at most six members of the
boards 44 and 45, for odd cores.  Every result it checks is proved, so it tests the programs, not the results; [the
tests of the theory's results](../../solver/docs/design/tests.md#the-theorys-results) give its counts.
