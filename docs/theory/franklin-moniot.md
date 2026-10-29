# Franklín and Moniot's paper

Part of [the theory](../THEORY.md).

Franklín and Moniot's central theorem is [the playability theorem (P2)](playability.md#the-playability-theorem) in the
language of matchings, and one count of payment schemes places [the parity bound (W1)](parity-bound.md#the-parity-bound)
between their characterization of playable sets and their bound.  A set is playable when some set of factors receives
exactly one of its payment schemes, independent with its own coordinates over GF(2) when some set receives an odd
number, and admitted by their bound when some set receives at least one ([the counting theorem
(C2)](#the-counting-theorem)).  This file ties the theory to the published paper, and nothing in the theory rests on it:
its three results, [the bridge lemma (C1)](#the-bridge-lemma), the counting theorem (C2) and [the first-odd-core
proposition (C3)](#the-first-odd-core), are explanatory.

The paper is A. F. Franklín and R. K. Moniot, "The difficulty of beating the Taxman", *Discrete Applied Mathematics* 339
(2023) 166–171 (arXiv:2211.00461; the theorem numbers below are the arXiv version's).  Its upper bound is one of those
the heuristics measure their scores against ([heuristics/README.md](../../heuristics/README.md)).  The paper uses no
linear algebra: everything below about GF(2) is this file's.

The game's words are the [glossary's](../GLOSSARY.md); instances, payment schemes, precedence, peel, cores, odd cores and
the vectors u(s) are [playability.md's](playability.md#payment-schemes).  The primer explains
[GF(2)](primer.md#finite-fields-and-gf2) and [permanents and determinants](primer.md#matchings-permanents-and-determinants).
Two words differ between the paper and the theory.  The paper's *rank* of a number is its layer Ω.  And *flat* means two
things: the paper's flat alternating cycle has its vertices in two consecutive ranks, while the flat cycle of [the
flat-cycle lemma (P3)](playability.md#the-flat-cycle-lemma) is a cycle of a precedence whose members lie in one layer;
the bridge lemma shows that each gives the other.

## What the paper proves

They play Taxman on any finite poset graded by a rank function, with weights: a move takes an element that still has
something below it in play, scores its weight, and removes it and everything below it.  Taxman is the divisibility
order on 1..n, ranked by Ω and weighted by value.  Their graph joins two elements when one covers the other; for
divisibility, when one is a maximal factor of the other.  So it is the *maximal-factor graph*, with the edge f–c
weighing c.  For a matching of this graph, a cycle is *alternating* when its edges are alternately in the matching and
out of it, and *flat* when its vertices lie in two consecutive ranks.

* Theorem 1: optimal play is a maximum-weight matching of the graph with no flat alternating cycle.  The proof works set
  by set in both directions.  From a play it matches each move with the largest number the move taxes, which is a
  maximal factor of the move; from such a matching it builds a play of exactly its upper ends.  So what it proves is: *a
  set S is playable iff it is the set of upper ends of a matching with no flat alternating cycle.*
* Theorem 2: the game on a graded poset is NP-hard, even with unit weights.  On two ranks every cycle is flat, and the
  game becomes the maximum alternating-cycle-free matching of a bipartite graph, NP-complete by H. Müller
  ("Alternating cycle-free matchings", *Order* 7 (1990) 11–21).
* Theorem 3: the born-free matching has no flat alternating cycle.  Theorem 4: restricted to the primes up to 5, it
  takes more than half the pot for N ≥ 847.  Theorem 5: the game can be won for every N ∉ {1, 3}, by the born-free
  matching except at 7 and 13, which get games by hand.  Theorem 6: a matching with no flat alternating cycle is put in
  order in O(N log N) by removing degree-one vertices, rank pair by rank pair: [peel](playability.md#peel)'s forced
  moves, on a matching already found.
* Their bounds, unnumbered: without the flat-cycle condition, the maximum-weight matching is an upper bound, the
  *Franklín–Moniot bound*; breaking its cycles with a feedback-arc-set heuristic gives a lower one.  The paper has no
  sharper upper bound.

*Payment schemes are matchings.*  Identify a payment scheme with the matching made of its pairs {x, pay(x)}.  The
payment schemes of a set S ⊆ {2, …, n}, that is of its own instance, are then exactly the matchings whose upper ends are
the members of S.  A member cannot be a lower end, since it is the upper end of its own edge, so the lower ends lie in
the pool of S's own instance: maximal factors of members, outside S.  The Franklín–Moniot bound is therefore the weight
of the heaviest set that has a payment scheme.

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

1. pay has a flat alternating cycle in the maximal-factor graph;
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
3. *2 ⇒ 1.*  By [P3](playability.md#the-flat-cycle-lemma), every cycle of the precedence lies in one layer k, and on
   each of its edges a → b, b lists pay(a).  So a cycle a₁ → a₂ → ⋯ → a_t → a₁ of distinct members gives the cycle a₁,
   pay(a₁), a₂, pay(a₂), …, a_t, pay(a_t) of the graph.  Its edges a_i–pay(a_i) are in pay, and its edges
   pay(a_i)–a_{i+1} are not, since a_{i+1} is paired with its own payment.  So it alternates, and it lies in the ranks k
   − 1 and k.
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
set, and [P2](playability.md#the-playability-theorem) are the same statement about the same objects, and their proof
takes P2's steps.  Their play-to-matching direction is the argument of [the lifting lemma
(P1)](playability.md#the-lifting-lemma): they take the largest number a move taxes, P2 a maximal factor above the
divisor the move surrenders.  Their matching-to-play direction repeatedly plays a member with no other payment below it,
which is a topological order of the precedence.  Their rank count is [the flat-cycle lemma
(P3)](playability.md#the-flat-cycle-lemma).  Theirs is the more general statement, for every finite graded poset; P2
adds that every topological order of the precedence is a legal game.

## The counting theorem

**Theorem C2 (the counting theorem).**  Let S ⊆ {2, …, n}.  Then:

1. S has a payment scheme iff per(S, T) ≥ 1 for some T;
2. S is independent with its own coordinates over GF(2) iff per(S, T) is odd for some T;
3. S is playable iff per(S, T) = 1 for some T.

So a playable set is independent with its own coordinates over GF(2), and such a set has a payment scheme.  The
Franklín–Moniot bound, [the parity bound (W1)](parity-bound.md#the-parity-bound) and opt(n) are the weights of the
heaviest subsets of {2, …, n} that meet the three conditions in turn.

*Proof.*  Each condition is a demand on the count of schemes onto some T: at least one, an odd number, exactly one.

1. *Schemes.*  A payment scheme with the payments T is one of the per(S, T) schemes counted.
2. *Independence.*  S is independent with its own coordinates iff its vectors u(s), the rows of its list matrix, are
   independent ([independence](playability.md#independence)).  The rows are independent iff some |S| columns T make a
   nonsingular square matrix, since the row rank is the column rank.  Over GF(2) the determinant of a matrix of 0s and
   1s is its permanent mod 2 ([the primer](primer.md#matchings-permanents-and-determinants)), and the permanent here is
   per(S, T).
3. *Playability.*  By [P2](playability.md#the-playability-theorem), S is playable iff some payment scheme has an acyclic
   precedence.  By [C1](#the-bridge-lemma), a scheme has one iff no other scheme has its payments T, that is, iff per(S,
   T) = 1.
4. *The bounds.*  The Franklín–Moniot bound is the heaviest set with a payment scheme, as above.  The parity bound is
   the heaviest set [independent at both parities](parity-bound.md#the-parity-vectors), and those are the sets
   independent with their own coordinates ([what the parity bound
   misses](parity-bound.md#what-the-parity-bound-misses)).  And opt(n) is the heaviest playable set.  ∎

Both inclusions are strict, and the counts show why.  The triangle {6, 10, 15} has two schemes, both onto {2, 3, 5}: it
has a scheme and is dependent, and no board below 15 has such a set.  The odd core {12, 18, 20, 30, 45} has three
schemes onto its five factors ([the first odd core](#the-first-odd-core)): it is independent and not playable, and no
board below 45 has such a set.

*What the count keeps.*  By steps 4 and 5 of C1's proof, the schemes onto T are one of them flipped along the sets of
pairwise disjoint flat alternating cycles of it, the empty set included, so per(S, T) counts those sets.  Their
characterization asks that some set of factors receive one scheme only.  The parity bound asks only that the count be
odd: it keeps the part of the condition that is linear, under which the heaviest set can be found in polynomial time
([computing the bound](parity-bound.md#computing-and-checking-the-bound)), and drops uniqueness.  So the sets that the
parity bound admits and the game does not are those with an odd count onto some T but no count of 1: by C2 they are
independent and not playable, and each holds an odd core.

*Which field.*  GF(2) is stricter than the rationals: vectors of integers independent over GF(2) are independent over
the rationals ([the primer](primer.md#finite-fields-and-gf2), the paragraph "Comparing fields"), and a bound built over
the rationals would admit the triangle, whose determinant is −2.  GF(3) is neither stricter nor weaker.  The odd core of
the board of 45 has a determinant of absolute value 3, so GF(3) excludes it while GF(2) admits it; the triangle's −2 is
1 mod 3, so GF(3) admits it while GF(2) excludes it.  Taken together, the signings over GF(3) miss nothing, frame by
frame ([the GF(3) proposition (B5)](frame-bounds.md#the-gf3-proposition)).

*Game 21, in both languages.*  Game 21 is the first board on which the Franklín–Moniot bound exceeds opt(n): 145 against
144, the two being equal on every board below.  A heaviest set with a payment scheme there is {10, 12, 14, 15, 16, 18,
19, 20, 21}.  Its pool is {1, …, 9}, its only possible set of payments, onto which it has two schemes.  They differ on
{10, 14, 15, 21}, a cycle of four edges on the primes, which [playability.md's
example](playability.md#payment-schemes) shows unplayable through the precedence cycle 10 → 14 → 21 → 15 → 10.  In the
maximal-factor graph these four and the primes form the 8-cycle 2–10–5–15–3–21–7–14–2, in the ranks 1 and 2: flat,
and alternating for both schemes.  So by their Theorem 1 the set is not playable, per = 2 is even, and v(10) + v(14) +
v(15) + v(21) = 0: the parity bound excludes it.

*A second proof of the independence lemma.*  The counting theorem proves the independence lemma
(P10) over GF(2) from the paper: their Theorem 1, the bridge lemma and the
determinant mod 2.  P10's own proof, through [the peel-order lemma (P9)](playability.md#the-peel-order-lemma), holds over
every field.

## The first odd core

**Proposition C3 (the first-odd-core proposition).**  Every set S ⊆ {2, …, 44} that is independent with its own
coordinates over GF(2) is playable.  So no board n ≤ 44 has an odd core, and the parity bound equals opt(n) for every n
≤ 44.  On the board of 45, the set {12, 18, 20, 30, 45} is an odd core of its own instance.

*Proof.*  Where peel stalls on an independent set, some member must list three live factors, each wanted by another
member, and below 45 no number can.

1. *Peel keeps independence.*  Let S be independent and not playable, so a run of peel on its own instance stalls with
   members left ([the peel theorem (P4)](playability.md#the-peel-theorem)).  Its moves keep the rows of the members
   left, restricted to the live factors, independent.  A back move deletes the only row with a 1 at some factor, with
   that factor, and a dependency of the rows left, which are 0 at that factor, would be one before.  A front move
   deletes a row whose only live entry is at its factor, with that factor, and a dependency of the rows left would,
   with that row added or not, be one before.
2. *The stall.*  The instance left at the stall, of the live members and the live factors, has its lists independent by
   step 1.  Each live member lists at least two live factors, or peel would have a front move, and each live factor
   that a live member lists is listed by at least two, or peel would have a back move.  So the live members form a
   core, and the instance left is not playable ([the core theorem (P7)](playability.md#the-core-theorem)).
3. *A member with three.*  If every live member listed at most two live factors, the instance left would be playable by
   [the two-factor proposition (P11)](playability.md#the-two-factor-proposition), since its lists are independent;
   step 2 rules that out.  So some live member lists three or more live factors, each listed by another member of S.
4. *Below 45.*  The numbers up to 44 with three or more maximal factors are 30 = 2·3·5 and 42 = 2·3·7.  The maximal
   factor 15 of 30 is a maximal factor of no other number up to 44, the next being 45, and the maximal factor 21 of 42
   of none, the next being 63.  So neither can be the member of step 3, and no independent S ⊆ {2, …, 44} fails to be
   playable.  The parity bound is the weight of an independent set, which is then playable, so it is at most opt(n);
   it is at least opt(n) by [W1](parity-bound.md#the-parity-bound).
5. *The board of 45.*  The members of {12, 18, 20, 30, 45} list, among the factors 4, 6, 9, 10 and 15, none of which is
   a member: 12 lists 4 and 6, 18 lists 6 and 9, 20 lists 4 and 10, 30 lists 6, 10 and 15, and 45 lists 9 and 15.
   Every factor is listed twice or more, so the set is a core, and not playable (P7).  Its only possible set of payments
   is all five factors.  If 30 pays 6, 10 or 15, the path 10–4–6–9–15 of the edges 20, 12, 18 and 45 then pays in one
   way, so per = 3, which is odd: by [C2](#the-counting-theorem) the lists are independent, and the core is odd.  ∎

In their language, every scheme of {12, 18, 20, 30, 45} has two flat alternating cycles, both through 30 and never
disjoint, so the sets of disjoint cycles are three: none, and either one.

The parity bound stays exact past the reach of C3, [as measured to 77](parity-bound.md#what-the-parity-bound-misses):
odd cores exist from 45 on, and none weighs in before 78.

## The hardness

This section is explanatory.  Their Theorem 2 makes the game NP-hard on posets of two ranks.  There a playable set is
the upper side of a *uniquely restricted matching*: a matching with no alternating cycle, equivalently the only perfect
matching on its vertices (M. C. Golumbic, T. Hirst and M. Lewenstein, "Uniquely restricted matchings", *Algorithmica* 31
(2001)).

The parity bound extends to their game.  On a graded poset, give each element the vector of the elements it covers, and
each element that something covers its unit vector, and split the coordinates by the parity of their rank.  By their
Theorem 1, a playable set has a matching of its members to elements they cover with no flat alternating cycle, and step
5 of [C1](#the-bridge-lemma)'s proof, which uses only the ranks, makes it the only matching onto its lower ends.  So its
count is 1, its determinant odd, and its family independent over GF(2), as in Taxman.  The heaviest set independent at
both parities therefore bounds their game, and it can be computed in polynomial time, as the parity bound of Taxman is
([computing the bound](parity-bound.md#computing-and-checking-the-bound)).  On two ranks with unit weights the bound is
the GF(2) rank of the bipartite adjacency matrix, which lies between the uniquely restricted matching number and the
matching number.  Three consequences follow.

* Unless P = NP, no bound computable in polynomial time equals the best score on every graded poset.  For the parity
  bound the difference is the odd cores.
* The bound is exact on posets in which every element covers at most two others, since [the two-factor proposition
  (P11)](playability.md#the-two-factor-proposition) holds there with the same proof, and their game is then solved in
  polynomial time.  So, unless P = NP, the hardness needs elements that cover three or more others: in Taxman, the
  numbers with three or more distinct primes, which is where the odd cores live.
* The complexity of Taxman itself is open, and nothing here settles it.  But the sources' instance and the frames,
  members above the factors they list, are instances of the two-rank problem that their Theorem 2 shows hard in general.
  A proof that (♦) holds on every two-rank instance, under every order, would make descending greedy keep a largest
  playable set of each, by the argument of [the span theorem (S6)](sources.md#the-span-theorem), and so solve that
  problem in polynomial time.  So, unless P = NP, a proof of (♦) must use what sets Taxman's instances apart, such as
  the order of the integers, to which [the adversarial experiments on
  (♦)](../../solver/noframe/docs/star.md#adversarial-search-what-protects-greedy) point.  The frame bound's gap at the
  roots measured is likewise odd cores, in the frames numbered 1 to 3 ([evidence.md](evidence.md#observations)).

Three questions stay open ([open questions](statements.md#open-questions)): a condition between "odd" and "exactly one"
that is computable in polynomial time and exact for Taxman, which by their Theorem 2 cannot be exact on every graded
poset unless P = NP; a description of the odd cores that weigh in, from 78 on; and the complexity of Taxman.

## The check by computation

A program of the heuristics, [fm_relation.py](../../heuristics/evaluation/fm_relation.py), checks the results of this
file on small boards, in under two minutes on one thread.  From the folder `heuristics`, the command is

    ../.venv/bin/python -m evaluation.fm_relation

It decides playability by the rules alone, and their condition literally, as a scheme with no flat alternating cycle,
and compares both with peel, the counts per(S, T) and independence at both parities.  It runs on every set of moves of
the boards up to 22, on random sets of the boards 30 to 120 grown around members with three or more distinct primes, and
on the sets of at most six members of the boards 44 and 45, for odd cores.  Every result it checks is proved, so it
tests the programs, not the results; [the tests of the theory's
results](../../solver/docs/design/tests.md#the-theorys-results) give its counts.
