# Relation to Franklín and Moniot

Part of [the theory](../THEORY.md).

Franklín and Moniot's central theorem is Theorem 1 in another language, and the parity bound lies exactly between their
characterization of playable sets and their bound.  A. F. Franklín and R. K. Moniot, "The difficulty of beating the
Taxman", *Discrete Applied Mathematics* 339 (2023) 166–171 (arXiv:2211.00461; theorem numbers below are the arXiv
version's), is the published foundation of the heuristics' yardsticks.  This section restates its objects in this file's
notation, shows that its central theorem is Theorem 1, and places the parity bound (Theorem 10 and its lemma, Theorem
7's lemma, Theorem 11 frame by frame) exactly between their characterization and their bound.  The paper uses no linear
algebra: everything below about GF(2) is this file's.

## What the paper proves

They play Taxman on any finite poset graded by a rank function, with weights: a move takes an element that still has
something below it in play, scores its weight, and removes it and everything below it.  Taxman is the divisibility order
on 1..n, ranked by Ω and weighted by value.  Their graph G joins two elements when one covers the other; for
divisibility, when one is a maximal factor of the other.  So G is the maximal-factor graph, with the edge f–c weighing
c.  A cycle of G is *alternating* for a matching M when its edges are alternately in M and not, and *flat* when its
vertices lie in two consecutive ranks.

* Theorem 1: optimal play is a maximum-weight matching of G with no flat alternating cycle.  The proof works set by set
  in both directions.  From a play, it matches each move with the largest number it taxes, which is a maximal factor of
  the move.  From such a matching, it builds a play of exactly its upper ends.  So what it proves is: *S is playable iff
  S is the set of upper ends of a matching of G with no flat alternating cycle.*
* Theorem 2: the game on a graded poset is NP-hard, even with unit weights.  On two ranks every cycle is flat, and the
  game becomes the maximum alternating-cycle-free matching of a bipartite graph, NP-complete by H. Müller ("Alternating
  cycle-free matchings", *Order* 7 (1990) 11–21).
* Theorem 3: the born-free matching has no flat alternating cycle.  Theorem 4: restricted to the primes up to 5, it
  takes more than half the pot for N ≥ 847.  Theorem 5: the game can be won for every N ∉ {1, 3}, by the born-free
  matching except at 7 and 13, which get games by hand.  Theorem 6: a matching with no flat alternating cycle is put in
  order in O(N log N) by removing degree-one vertices, rank pair by rank pair: peel's forced moves, on a matching
  already found.
* The bounds (unnumbered): without the flat-cycle condition, the maximum-weight matching is an upper bound
  (`evaluation/bound.py`); breaking its cycles with a feedback-arc-set heuristic gives a lower one.  The paper has no
  sharper bound: Theorem 10's is never above the matching (its lemma's minor argument).

Notation for the rest of the section.  For a set of moves S ⊆ {2, …, n}, F_S is the set of maximal factors of S's
members that are not in S.  A *payment scheme* of S pays each member with a distinct maximal factor of its own from F_S.
It is exactly a matching of G whose upper ends are S (no member can be a lower end): Theorem 1's matching without its
precedence condition.  For T ⊆ F_S with |T| = |S|, per(S, T) is the number of payment schemes whose payment set is T.
It is the permanent of A_S[S, T], where A_S has a row per member, a column per factor of F_S, and a 1 where the factor
is a maximal factor of the member.  The rows of A_S are the u(s) of Theorem 10's lemma: v(s) with the coordinates in S
deleted.

## Their Theorem 1 is Theorem 1

**Lemma (the bridge).**  Let M be a payment scheme of S with payment set T.  The following are equivalent: (a) M has a
flat alternating cycle in G; (b) the precedence of Theorem 1 is cyclic under M; (c) another payment scheme of S has the
payment set T.

*Proof.*  An alternating cycle has its matched edge at every vertex, so its vertices are members and their payments.
(a) ⇒ (b): in a flat one the members are the upper rank, matched down, and each unmatched edge joins pay(a) to a member
b of which it is a maximal factor, a precedence edge a → b.  (b) ⇒ (a) is the potential argument of Theorem 2's proof,
which needs only maximal-factor payments.  On a precedence edge a → b, pay(a) is a proper divisor of b (when a | b,
because pay(a) | a), so Ω(pay(a)) ≤ Ω(b) − 1 = Ω(pay(b)), with equality iff pay(a) ∈ mf(b).  Around a cycle equality
holds throughout.  So a simple cycle a₁ → ⋯ → a_m gives the cycle a₁, pay(a₁), a₂, …, a_m, pay(a_m) of G: alternating,
with all its members at one rank and all its payments one below, flat.  (a) ⇒ (c): swapping the matched and unmatched
edges of the cycle gives another scheme with the same payments.  (c) ⇒ (a): the two schemes differ on disjoint cycles
that alternate for M, and each is flat: an unmatched edge joins a payment pay(a) to a member b with pay(a) ∈ mf(b), so
Ω(b) = Ω(pay(a)) + 1 = Ω(a).  ∎

So Theorem 1 and their Theorem 1, read set by set, are the same statement about the same objects.  Both directions of
Theorem 1's proof are in theirs.  Their play-to-matching direction is the lifting argument: they take the largest number
a move taxes, Theorem 1 any maximal factor above the surrendered divisor.  Their matching-to-play direction repeatedly
plays a member with no other payment below it, which is a topological order of the precedence.  Their rank count is the
first half of Theorem 2's proof.  Theirs is the more general statement (every finite graded poset).  Theorem 1's form
adds that every topological order is a legal game.  Their proof also shows the precedence's second clause is redundant:
a | b implies pay(a) | b.

## One count, three conditions

**Theorem 12 (one count, three conditions).**  For S ⊆ {2, …, n}:

1. S is matchable (it has a payment scheme, the condition behind the Franklín–Moniot bound) iff per(S, T) ≥ 1 for some
   T;
2. the family of Theorem 10's lemma (v(s) for s ∈ S, e_s for s ∈ S with s ≤ n/2) is independent over GF(2) iff per(S, T)
   is odd for some T;
3. S is playable iff per(S, T) = 1 for some T.

So playable ⇒ independent ⇒ matchable.  The best score opt(n), the parity bound of Theorem 10 and the Franklín–Moniot
bound are the weights of the heaviest subsets of {2, …, n} meeting the three conditions in turn.

*Proof.*  (1) is the definition.  (2) The family is independent iff the rows of A_S are (the last step of the lemma's
proof, read both ways), iff some square A_S[S, T] is nonsingular over GF(2).  Over GF(2) the signs of a determinant's
expansion vanish, and the nonzero terms of A_S[S, T] are its perfect matchings, so its determinant is per(S, T) mod 2.
(3) By their Theorem 1 (or Theorem 1), S is playable iff some scheme has no flat alternating cycle (acyclic precedence).
By the bridge, that holds iff some scheme is the only one with its payment set.  ∎

Both implications are strict.  The triangle {6, 10, 15} (two schemes) is matchable and dependent, and no board below 15
has such a set.  The set {12, 18, 20, 30, 45} (three schemes, below) is independent and unplayable, and no board below
45 has such a set.

*The house example, n = 21, in both languages.*  The set S = {10, 14, 15, 21} is a 4-cycle on the primes: the edges 2–5,
2–7, 3–5, 3–7.  In G, S and the primes form the 8-cycle 2–10–5–15–3–21–7–14–2, all at ranks 1 and 2.  S's payment
schemes are its two perfect matchings, each the other flipped around that 8-cycle, which is flat and alternating for
both.  By their Theorem 1, S is unplayable.  Here, the scheme 10:2, 14:7, 21:3, 15:5 has the precedence cycle 10 → 14 →
21 → 15 → 10 (the other scheme the reverse).  The count per(S, {2, 3, 5, 7}) = 2 is even, and v(10) + v(14) + v(15) +
v(21) = 0.  All four have Ω = 2, so they are v(s) in M₁, and S is dependent there.  The F–M bound's 145-set {10, 12, 14,
15, 16, 18, 19, 20, 21} contains it.  It has one payment set, {1, …, 9}, with two schemes, both carrying the same
8-cycle.  So the F–M bound reads 145 at n = 21 where the parity bound reads opt(21) = 144.

*Theorem 10 on the published foundation.*  Theorem 12 gives Theorem 10's lemma a second proof: their Theorem 1, the
bridge, and the determinant mod 2.  The rest of Theorem 10 is linear algebra on top.  Theorem 7's lemma is the case of
the sources (no member is a factor, nothing is deleted).  Theorem 11's frame step is the same argument in each frame.  A
frame is their game on a poset of two ranks (factors below the movers that list them) in which every mover must be
taken.  By their Theorem 1, a winnable frame has a scheme that is the only one onto its payment set, so its determinant
is odd.  The frame model's own premise (promoting Q succeeds iff every frame is winnable with Q applied) is untouched.
Both proofs give more than GF(2): a scheme unique onto T makes A_S[S, T] unit triangular after reordering (the play
order of the lemma's proof does it), so a playable set's rows are independent over every field.

## What the parity bound keeps, and what it drops

Fix a scheme M with payment set T.  By the bridge, the schemes onto T are M flipped along the sets of pairwise disjoint
flat alternating cycles of M, the empty set giving M itself.  So per(S, T) counts those sets.  Their characterization
asks for a scheme with no flat alternating cycle (per = 1).  The parity bound asks only that the sets be odd in number
for some T.  It keeps the count mod 2, the part of the condition that is linear (hence a matroid intersection, hence
polynomial), and drops uniqueness.  GF(2) is the field for this.  Over the rationals independence needs a nonzero
*signed* count, which an odd count implies, since a sum of an odd number of ±1s is odd.  The triangle {6, 10, 15} has
two schemes of the same sign (determinant ±2): independent over the rationals, dependent over GF(2).

**Proposition (two factors).**  Suppose every member of S has at most two maximal factors outside S.  Let G_S be the
multigraph whose vertices are the factors and a ground vertex, and whose edges are the members, each joining its two
factors (its one factor and the ground; with none, a loop at the ground).  Then S is playable iff it is independent iff
G_S is a forest.  The flat alternating cycles of a scheme are exactly the cycles of G_S, each traversed so that every
edge pays at its forward end.

*Proof.*  A GF(2) dependency of the rows is a nonempty set of edges with even degree at every factor.  The handshake
lemma makes the ground's degree even too, so the set contains a cycle; conversely a cycle is a dependency (a loop is a
zero row).  So independent iff forest.  A forest has a scheme that is the only one onto its payment set: root each tree
at the ground, or anywhere if the tree has no ground, and pay each edge at its lower end.  A leaf can be paid only by
its own edge, and so on inward.  By Theorem 12 (3) a forest is playable, and a playable set is always independent.  An
alternating cycle s₁, f₁, s₂, f₂, … (fᵢ = pay(sᵢ), fᵢ ∈ mf(sᵢ₊₁)) is the cycle f₁ f₂ ⋯ of G_S through the edges s₂, s₃,
…, s₁, each paying at its forward end.  Conversely, take a component with a cycle and a scheme.  It has at least as many
edges as vertices (it is connected with a cycle) and at most as many as factors (the scheme), so it has no ground and is
a bijection from edges to factors with a single cycle.  Its trees pay from their leaves inward, which leaves the cycle's
edges to pay the cycle's vertices: around the cycle in one of its two directions.  ∎

So on such sets their condition is acyclicity, and the parity bound loses nothing: a nonzero per(S, T) is 2^c for c
cyclic components, odd only without cycles.  This is the graph-like regime of "Where exchange breaks" and of Theorem
11's remark.  What the bound drops are the *odd cores*, now exactly: unplayable sets with an odd number of schemes onto
some payment set.  By the proposition, each has a member with three or more maximal factors outside it.

**Proposition (the first odd core).**  No board n ≤ 44 has an odd core, so there the parity bound equals opt(n).  On the
board 45, {12, 18, 20, 30, 45} is one.

*Proof.*  Let S be independent and unplayable, so peel fails on it (Theorem 3).  Peel's moves keep the rows, restricted
to the live factors, independent.  A back move deletes the only row with a 1 at some factor, together with that factor.
A front move deletes a row e_f and the column f, and a dependency of what is left would be one of the rows before, with
or without e_f.  So peel never strands a member (a zero row): it stalls with members left.  Each has at least two live
factors, and every live factor that one of them lists is listed by at least two.  Suppose every row had exactly two
entries.  The entries number at least two per column and exactly two per row, and independence allows no more rows than
columns.  So every column would hold exactly two, and the rows would be the edges of a 2-regular graph, a union of
cycles, which is dependent.  So some member of the stall has three or more live factors, each listed by a second member
of S.  Below 45, the numbers with three or more maximal factors are 30 and 42.  Their factors 15 and 21 have no other
multiple on the board (the next are 45 and 63), so neither can be that member.

On the board 45, the rows of {12, 18, 20, 30, 45} over the factors 4, 6, 9, 10, 15 (none of them a member) are 12: {4,
6}, 18: {6, 9}, 20: {4, 10}, 30: {6, 10, 15} and 45: {9, 15}.  Every factor is listed twice or more, so the set is
self-covering, hence unplayable (Theorem 5's first half, which needs no separation).  The payment set is all five
factors.  30 pays 6, 10 or 15, and the path 10–4–6–9–15 (edges 20, 12, 18, 45) then pays uniquely, so per = 3 is odd.
In their language, every scheme has two flat alternating cycles, both through 30, so never disjoint: three sets (none,
either one).  ∎

The bound is exact longer than the proof reaches.  It equals opt(n) for every n ≤ 77 and first exceeds it at 78, with
1,992 against 1,991 (`solver/noframe/data/exact_parity_2_1300.jsonl`).  So odd cores exist from 45 on, and none weighs
in before 78.

## The hardness

Their Theorem 2 makes the game NP-hard on posets of two ranks.  There a playable set is the upper side of a uniquely
restricted matching (M. C. Golumbic, T. Hirst and M. Lewenstein, "Uniquely restricted matchings", *Algorithmica* 31
(2001): a matching with no alternating cycle, equivalently the only perfect matching on its vertices).  The bridge,
Theorem 12 and the parity split use nothing beyond a graded poset (rank parity for Ω parity).  So the parity matroids
bound their general game too, in polynomial time.  On two ranks with unit weights the bound is the GF(2) rank of the
bipartite adjacency matrix, which lies between the uniquely restricted matching number and the matching number.  Three
consequences:

* Unless P = NP, no bound computable in polynomial time equals the best score on every graded poset.  For the parity
  bound the difference is the odd cores.
* The bound *is* exact on posets in which every element covers at most two others, since the two-factor proposition
  holds verbatim.  There their game is solved in polynomial time by weighted matroid intersection.  The hardness needs
  elements that cover three or more others.  In Taxman these are the numbers with three or more distinct primes, which
  is exactly where the odd cores live.
* The paper's hardness is for the general game.  The complexity of Taxman itself is open, and nothing here settles it.
  But the sources' instance (members above the factors they list) and the frames are instances of the two-rank problem
  that their Theorem 2 shows hard in general, and the gap left by Theorem 11 is the odd cores of frames 1–3
  (solver/notebook/the-anatomy-of-the-gap-2026-10/README.md).  So wherever (♦) or the parity bound is exact in Taxman,
  the exactness comes from the arithmetic, not the structure.  This matches what the adversarial experiments on (♦)
  found.

## Status

*Proven:* the bridge; Theorem 12; Theorem 10's lemma (with Theorem 7's, and Theorem 11 frame by frame) from their
Theorem 1; the two-factor proposition; no odd core below the board 45, so the parity bound is exact for n ≤ 44; the odd
core at 45.

*Checked by computation* (`python3 -m evaluation.fm_relation` from heuristics/, two minutes).  Playability is decided by
the rule alone.  Their condition is checked literally, as a scheme with no flat alternating cycle in G.

* All 2,097,152 sets of moves from {2, …, 22}.  Playability, their condition, per = 1 for some T, and peel agree on
  every set.  Independence of the lemma's family, independence in both parity matroids, and an odd per for some T also
  agree on every set.  The hierarchy holds.  1,112 sets are matchable and dependent, the smallest {6, 10, 15}.  There
  are no odd cores.  The heaviest playable, independent and matchable sets give opt(n), the parity bound and the F–M
  bound, which equal the prover's values at every n ≤ 22.
* 18,200 random matchable sets of up to 12 members on the boards 30–120.  Each was grown around a member with three or
  more distinct primes, the regime in which the hierarchy is strict.  The same equivalences hold on every set, including
  500 odd cores and 570 sets that are matchable and dependent.
* Board 44 has no odd core of at most six members (the proposition says none at all).  On board 45, {12, 18, 20, 30, 45}
  is the only minimal odd core of at most six members.

*Measured:* the parity bound's exactness to 77, above.

*Open:* a condition between "odd" and "exactly one" that is computable in polynomial time and exact for Taxman (by their
Theorem 2 none is exact on every graded poset unless P = NP); a description of the odd cores that weigh in, from 78 on;
the complexity of Taxman itself.
