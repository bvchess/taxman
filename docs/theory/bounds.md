# The matroid bounds, and prime exchanges

Part of [the theory](../THEORY.md).

## The parity matroids: a bound on the whole game

The span certificate's lemma (playable sets are independent) is about the sources, which are never anyone's factors.  It
extends to the whole board once each move's own coordinate is taken out, and the extension splits by the parity of Ω
into two matroids, so that the heaviest set independent in both is an upper bound on opt(n) that polynomial algorithms
compute.

Fix a board n and h = ⌊n/2⌋.  The coordinates are the numbers 1..h (every maximal factor of a number up to n is at most
n/2).  Each number s of 2..n has the vector v(s) over GF(2) with a 1 at each of its maximal factors, and each s ≤ h also
the unit vector e_s.

**Lemma (playable sets with their own coordinates are independent).**  If S ⊆ {2, …, n} is playable, the family

    {v(s) : s ∈ S} ∪ {e_s : s ∈ S, s ≤ h}

is linearly independent over GF(2).

*Proof.*  Play S in a legal order x₁, …, x_k (Theorem 1 gives one).  The move xᵢ surrenders a proper divisor d still in
the pot; lift it to a maximal factor fᵢ = xᵢ/p with p a prime dividing xᵢ/d, so d | fᵢ | xᵢ.  The factor fᵢ is in the
pot when xᵢ is played: whatever removed it earlier, playing it or taking it as tax for a multiple, would have removed d
with it.  Hence (a) fᵢ divides no earlier move xⱼ (j < i), which would have taken it, so it is no maximal factor of one;
and (b) fᵢ ∉ S: it is not xᵢ (a proper divisor), not an earlier move (it is still in the pot), and not a later one (xᵢ
takes it).  Let u(s) be v(s) with the coordinates in S deleted.  A nonempty vanishing sum of some u(xᵢ) is impossible:
for the largest i in it, coordinate fᵢ survives the deletion by (b), is 1 in u(xᵢ), and is 0 in every other term by (a).
So the u(s), s ∈ S, are independent.  Now let Σ_{s∈A} v(s) + Σ_{s∈B} e_s = 0 with A ⊆ S and B ⊆ S ∩ [2, h].  Deleting
the coordinates in S removes every e_s and leaves Σ_{s∈A} u(s) = 0, so A = ∅, and then Σ_{s∈B} e_s = 0 gives B = ∅.  ∎

For a set S of sources no coordinate lies in S and no e_s appears: the lemma is then the span certificate's (a
dismantling order is a play order read backwards).  It is also stronger than Theorem 1's matching: independent u(s) have
a nonzero maximal minor, and a nonzero term of its expansion matches each s ∈ S to a distinct maximal factor outside S,
which is the Franklín–Moniot relaxation; a matchable set need not be independent.  Independence sits exactly between
matchability and playability (an odd number of payment schemes onto some payment set, against at least one and exactly
one): Theorem 12, in "Relation to Franklín and Moniot", which also derives the lemma from their Theorem 1.

**Theorem 10 (the parity matroids).**  Split the coordinates by the parity of Ω (1 counts as even).  For q ∈ {0, 1} let
M_q be the linear matroid on {2, …, n} in which s is represented by

* v(s), if Ω(s) and q have different parities;
* e_s, if they have the same parity and s ≤ h;
* a coloop, if they have the same parity and s > h.

Every playable set is independent in M₀ and in M₁.  Hence opt(n) is at most the heaviest weight Σ_{s∈I} s of a set I
independent in both.  More generally, for the moves P that a node of a search fixes in and the barred numbers X that it
fixes out, every playable S with P ⊆ S and S ∩ X = ∅ weighs at most w(P) plus the heaviest common independent set of
M₀/P ∖ X and M₁/P ∖ X (and there is none unless P is independent in both).

*Proof.*  A maximal factor f of s has Ω(f) = Ω(s) − 1, so v(s) lies in the coordinates whose parity differs from Ω(s)'s
and e_s in those of Ω(s)'s own: every vector of the lemma's family lies in one of the two coordinate subspaces E₀ (Ω
even) and E₁ (Ω odd).  Since GF(2)^h is their direct sum, such a family is independent iff its members in E₀ are and its
members in E₁ are.  For a playable S, the members in E_q are the vectors representing in M_q the elements of S that are
not coloops of M_q, so they are independent, and adding coloops keeps a set independent: S is independent in M_q.
The weight of a heaviest playable set is opt(n).  At a node, a playable S ⊇ P is independent in M_q, so P is, and S ∖ P
is independent in the contraction M_q/P; it avoids X.  ∎

Three facts make it usable as evidence.

* **It is computable.**  The heaviest common independent set of two matroids is a polynomial problem (weighted matroid
  intersection; Lawler, Frank), and over GF(2) with these sparse vectors a solve takes milliseconds: 22 ms at n = 990
  and 43 ms at 1300 from scratch.
* **Its value can be certified without trusting the algorithm.**  For any split w = w₀ + w₁, every T independent in both
  matroids has w(T) = w₀(T) + w₁(T) ≤ max_{M₀} w₀ + max_{M₁} w₁ (weak duality).  And in one matroid an independent I is
  a heaviest independent set, over all sizes and for weights of either sign, iff (1) w(x) ≥ 0 for x ∈ I, (2) w(y) ≤ 0
  whenever I + y is independent, and (3) w(y) ≤ w(x) whenever x lies in the circuit of I + y.  (Add r dummy coloops of
  weight 0 and truncate to rank r: the independent sets, padded with dummies, become the bases, and (1)–(3) are the
  classical criterion that no single exchange improves a basis.)  So a common independent I with a split satisfying
  (1)–(3) in both matroids proves that w(I) is the maximum, whatever produced it; the same split bounds the maximum with
  one member x of I barred, since the heaviest independent set avoiding x is I − x plus the heaviest y with I − x + y
  independent, if that weighs more than 0.
* **It is never above the Franklín–Moniot bound** (by the minor argument above).  It is not one matroid, so the remark
  that no matroid relaxation is exact for the whole game (game 425) does not touch it.

What it misses are the *odd cores*: unplayable sets whose vectors, with their own coordinates taken out, are still
independent; these are the self-covering sets no subset of which lists every factor an even number of times.  Over
another field it would miss more: over GF(3) or the rationals the unsigned vectors of a triangle of moves are
independent.  The same construction, frame by frame, bounds the frame model's promotions (Theorem 11, below), and the
two agree at every root measured ([Matroid intersection
bounds](../../solver/notebook/matroid-intersection-bounds-2026-10/README.md)).  What the bound buys the prover's search
is in the evidence table below.

## The frame matroids: a bound on the promotions

Theorem 10's lemma, applied frame by frame, bounds the frame model's promotions in the same way: two matroids, one of
the even frames and one of the odd frames, in both of which every promotable set is independent.  This is the solver's
GF(2) bound (tactic `gf2`; solver/docs/design/gf2-bound.md, "The GF(2) bound").

**Theorem 11 (the frame matroids).**  Give each frame h of a state the coordinates of its factors, each mover u paying
in h (a move of h, or a number of home h + 1 promoted into it) the vector v_h(u) of its maximal factors among them, and
each factor y of h the unit vector e_y.  If a set Q of candidates can be promoted, then in every frame h the vectors of
its moves, the v_h(x) of Q's members promoted into h and the e_y of Q's members promoted out of h are linearly
independent.  So Q is independent in the matroid of each frame on the candidates that touch it (the frame's moves
contracted); a candidate of home h touches frames h and h − 1, one even and one odd, so the even frames' matroids are
the blocks of a matroid A on the candidates and the odd frames' of a matroid B, and Q is independent in both.  The
heaviest common independent set of A and B bounds the best promotion set.  At a node reached by promoting P, a
completion F makes P ∪ F promotable from the root, so F is independent in A and B contracted by P: the bound of the node
is the heaviest common independent set of the contractions.

*Proof.*  Promoting Q succeeds exactly when every frame is winnable with all of Q applied at once, its moves plus Q's
members promoted into it and its factors less Q's members promoted out of it (solver/docs/design/search.md;
`model::promote`).  A winnable frame peels completely (`model::peel`): each move dropped has a live factor that no move
still there has, so, as in the proof of Theorem 10's lemma (the peel's order read backwards is the play order there),
the movers' vectors restricted to the live factors are independent, which is independence together with the unit vectors
of the factors taken out.  The moves of a frame are fixed, so the frame's condition is independence of Q's part in the
contraction by them; frames have disjoint coordinates, and the even and odd frames' contractions are the blocks of A and
B.  At a node, F completes P exactly when P ∪ F is promotable from the root, so P ∪ F is independent in A and in B, and
F is independent in their contractions by P (which P, independent, makes well defined).  ∎

Three facts about it.  It is never above the matching bound of solver/docs/design/search.md, by Theorem 10's minor
argument frame by frame: an assignment of the movers to distinct live factors in which each unit vector e_y takes y
itself, so that no promoted number pays as a factor, is the matching bound's relaxation.  On movers with at most two
live factors (graph edges, with a ground vertex for one) independence over GF(2) is being a forest, which is winnability
itself, so what it misses are, as for Theorem 10, the odd cores of movers with three or more factors (the frames'
residuals at the roots of 936-1300 hold 18-65 movers, and Lorea's matroid misses them too).  And Theorem 10's bound
equals in_the_bag plus Theorem 11's at every root of 880-1300; its gap to opt(n) is a tenth of the Franklín–Moniot
bound's to 1300.  The measurements are in solver/notebook/matroid-intersection-bounds-2026-10/README.md and
solver/notebook/the-gf2-bound-in-the-search-2026-10/README.md; the search's forced members rest on weak duality for a
weight split, as Theorem 10's certificate does (`gf2::meet`).

## Prime exchanges: symmetries of the promotions

Exchanging two primes in every number is an automorphism of divisibility.  Where it maps the frame model's root onto
itself, it maps promotion sets to promotion sets, and no optimal set is improved by it, which orders the two primes'
shares in every optimal set at once.  This is the solver's tactic `symmetry` (solver/docs/design/gf2-bound.md, "Prime
exchanges"; solver/notebook/dominance-and-symmetry-2026-10/README.md found the exchanges and measured them).

For primes p < q let π = π_{p,q} send p^a q^b m (m prime to pq) to p^b q^a m.  For a set S of numbers and a prime r let
c_r(S) = Σ { x/r : x ∈ S, r | x }, the sum of S's cofactors of r.

**Theorem 13 (prime exchanges).**  Suppose that π maps the moves of every frame of the root onto themselves, the factors
of every frame onto themselves, and the candidates onto themselves.  Then:

1. a set S of candidates is promotable exactly when π(S) is;
2. Σ π(S) − Σ S = Σ_{x∈S} (π(x) − x), which is (q − p)(c_p(S) − c_q(S)) when no candidate is divisible by pq, p² or q²;
3. every heaviest promotion set S\* has c_p(S\*) ≤ c_q(S\*), for all the pairs (p, q) that meet these conditions at
   once.

*Proof.*  (1) The exchange π is multiplicative and a bijection of the positive integers that sends primes to primes, so
y = x/r with r prime exactly when π(y) = π(x)/π(r): π preserves the maximal-factor relation.  Promoting S succeeds
exactly when every member of S is a factor of its home frame h ≥ 1 and every frame is winnable with all of S applied at
once, its moves plus S's members promoted into it and its factors less S's members promoted out of it
(solver/frame/src/model/promote.rs, from the peel); a promotion set also leaves no frame with negative freedoms and has
at most max_total members.  The exchange π maps each frame's factors onto themselves, so π(x) has the home frame of x,
and π maps frame h with S applied onto frame h with π(S) applied: moves to moves, factors to factors, and each mover's
maximal factors in the frame to its image's.  The two are the same game of Taxman Mini, winnable together; the freedoms
a frame loses are the members promoted into and out of it, counts that π keeps, as it keeps the size of S.  The exchange
π is an involution, so the converse is the same argument for π(S).  (2) The first form is the definition of the sums.
Under the condition, a member x of S is divisible by at most one of p and q, and by it once: π(x) = xq/p, a gain of (q −
p)·x/p, if p | x; π(x) = xp/q, a loss of (q − p)·x/q, if q | x; and π(x) = x otherwise.  (3) By (1), π(S\*) is a
promotion set, so Σ π(S\*) ≤ Σ S\*, which by (2) is c_p(S\*) ≤ c_q(S\*).  The argument for one pair uses no other, so
every heaviest set keeps every pair's inequality.  ∎

Within a class of primes any two of which are such a pair (the exchanges compose: π_{p,r} = π_{p,q} π_{q,r} π_{p,q}),
(3) says that every optimal set gives the larger primes the larger cofactor sums, the order in which the rearrangement
inequality puts the class's arrangements at their heaviest.  No sorting argument is needed, though, nor any condition
across classes: a set that breaks one pair's inequality is beaten by its image under that pair's exchange alone,
whatever its members' other primes.

*Pruning with the inequalities.*  A branch and bound for a heaviest promotion set may discard every node whose
completions F all leave P ∪ F (P the node's promotions) breaking some pair's inequality.  If the incumbent is not
optimal, every heaviest set beats it and keeps every inequality, so such a node holds none of them; and the other
prunings (bounds above every completion, members that every completion beating the incumbent promotes, sets that cannot
be promoted) hold for the completions that keep the inequalities as for all completions.  With C the node's candidates,
every completion has c_p(P ∪ F) ≥ c_p(P) and c_q(P ∪ F) ≤ c_q(P) + c_q(C).  So a node with c_p(P) > c_q(P) + c_q(C) has
no completion that keeps the pair's inequality (it is *hopeless*), and a candidate x = mp with c_p(P) + m > c_q(P) +
c_q(C) is in none.  And for any μ ≥ 0, a completion that keeps the inequality has

    Σ F ≤ Σ F + μ (c_q(P ∪ F) − c_p(P ∪ F))
        = Σ_{x∈F} w_μ(x) + μ (c_q(P) − c_p(P)),

with w_μ(mp) = m(p − μ), w_μ(mq) = m(q + μ) and w_μ(x) = x for the other candidates.  By Theorem 11, F is common
independent in the node's contracted matroids, so their heaviest common independent set under w_μ (of members of
positive weight, and no more than the node can still promote), plus μ (c_q(P) − c_p(P)), bounds every such completion: a
Lagrangian relaxation of the inequality (weak duality, as for Theorem 11's weight split).  The solver uses the first
two; the third was measured and not kept (solver/notebook/prime-exchanges-in-the-search-2026-10/README.md).

These hold for the search of the root's problem, whose subtrees all are; a search of one node's completions for their
own best (as some tests run) finds the best of those that keep the inequalities.

## Signed frame matroids: the frame bound over other fields

Theorem 11's argument never uses that its entries are 1 or that the field is GF(2): a winnable frame's movers have a
triangular matrix with a nonzero diagonal under every choice of nonzero entries, over every field.  Each such choice
gives a matroid per frame in which every promotable set is independent, and the GF(2) bound and any number of them
combine by Lagrangian multipliers into one bound (the study of the multi-field bound,
solver/notebook/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md, which measured it).

Fix a state and a frame h of it, with factors F_h and moves M_h; its *movers* are its moves and the candidates of home
h + 1, and the candidates *touching* it, supp_h, are those of homes h and h + 1.  A *signing* of h over a field K gives
each incidence (u, f), u a mover and f one of u's maximal factors among F_h (the factor lists of
`model::FactorProvider`), a value σ(u, f) in K − {0}.  Mover u has the vector v_σ(u) = Σ_f σ(u, f) e_f of K^{F_h}, and a
factor y the unit vector e_y.

**Theorem 14 (signed frame matroids).**  If a set Q of candidates can be promoted, then for every frame h and every
signing σ of it, the vectors v_σ(m) of h's moves, v_σ(x) of Q's members of home h + 1 and e_y of Q's members of home h
are linearly independent over K.  So, with the moves contracted (each vector taken modulo their span), Q ∩ supp_h is
independent in the linear matroid N_h^σ that represents a candidate x of home h + 1 by the class of v_σ(x) and a
candidate y of home h by the class of e_y.  Moreover:

1. the signings of different frames, and their fields, are chosen independently: every promotable set is independent in
   every such matroid at once;
2. at a node reached by promoting P, every completion F (P ∪ F promotable from the root) has F ∩ supp_h independent in
   N_h^σ / P, and this contraction is the matroid built with the same σ from the node's own frame h (its moves M_h plus
   P's members of home h + 1, its factors F_h less P's members of home h), whether or not the node's frames were
   reduced;
3. a candidate whose class is zero in some N_h^σ (a *loop*) is in no promotable set.

*Proof.*  Promoting Q succeeds exactly when every frame is winnable with all of Q applied at once: movers M_h plus Q's
members of home h + 1, pool F_h less Q's members of home h (solver/frame/src/model/promote.rs; the proof of Theorem 11).
A winnable frame peels completely (`model::peel`): list the movers u₁, …, u_k in the order the peel drops them, u_i with
a pool factor f_i that no mover still there lists, so that f_i is listed by no u_j with j > i.  In the matrix of the
movers' vectors restricted to the pool, rows in that order and columns f₁, …, f_k first, entry (u_i, f_i) is σ(u_i, f_i)
≠ 0 and entry (u_j, f_i) is 0 for j > i: the matrix is triangular with a nonzero diagonal, so the restricted rows are
independent.  Now let a combination of the moves' vectors, of the v_σ(x) of Q's members of home h + 1 and of the e_y of
Q's members of home h vanish.  Restricted to the pool it loses the e_y and leaves a vanishing combination of the movers'
restricted vectors, whose coefficients are therefore 0; what is left is a vanishing combination of distinct unit
vectors, whose coefficients are 0 too.  The moves alone are independent (Q empty), so the independence of the whole
family is that of Q's part in the quotient by the moves' span, which is the claim.  (1): each frame's condition is about
that frame's vectors alone.  (2): P ∪ F is promotable from the root, so it is independent in the root's N_h^σ, and F is
independent in the contraction by P (P is independent).  Contracting the e_y of P's members of home h is quotienting
K^{F_h} by their coordinates, which leaves K^{F_h − P} with each vector restricted, and contracting the v_σ(x) of P's
members of home h + 1 adds them to the moves: the node's frame with the same values.  A reduction drops a move m whose
only live factor is q together with q; m's vector is then σ(m, q) e_q on the frame's factors, and the quotient by it is
the deletion of coordinate q, so the matroid is unchanged.  (3): a promotable set holding x would have the class of x,
zero, in an independent family.  ∎

With σ ≡ 1 over GF(2), N_h^σ is Theorem 11's frame matroid, and the even frames' and odd frames' make A and B.  Over
GF(3) the converse holds too, which the study of the multi-field bound uses to choose its signings and nothing below
needs.  If frame h with Q applied is not winnable, its peel's residual C is self-covering on the pool (every pool factor
a member of C lists is listed by two or more).  Give the listers of each such factor values that sum to 0 (pairs 1, 2
and, for an odd count, one triple 1, 1, 1).  Then Σ_{u∈C} v_σ(u) vanishes on the pool, so it is a combination of the e_y
of Q's members of home h, and Q ∩ supp_h is dependent in N_h^σ: the relation has the coefficient 1 on each member of Q
among C's movers, and if there were none and every e_y's coefficient were 0, C's moves would have vectors summing to 0,
against their independence.  So the GF(3) signed frame matroids of all signings are together exact for winnability
(Hershkowitz and Schneider's theorem, in this setting).

**Theorem 15 (the decomposition bound).**  Let A, B be Theorem 11's matroids of a state, N_1, …, N_K signed frame
matroids of it (each of one frame h_k and one signing, on supp_{h_k}), cap the most members a completion can have, and
w(x) = x.  For any real vectors λ_k on supp_{h_k}, every promotable set (at a node: every completion) Q with at most cap
members satisfies

    w(Q) ≤ MI_cap(w − Σ_k λ_k) + Σ_k G_k(λ_k) = L(λ),

where MI_cap(c) is the heaviest common independent set of A and B of at most cap elements for the weights c, its
elements of weight at most 0 left out, and G_k(μ) the heaviest independent set of N_k for the weights μ.  The same holds
with the loops of the N_k (Theorem 14, (3)) left out of the intersection.

*Proof.*  Split the weight: w(Q) = (w − Σ_k λ_k)(Q) + Σ_k λ_k(Q ∩ supp_{h_k}).  The set Q is common independent in A and
B with at most cap members (Theorem 11), and so is its subset Q⁺ of positive reweighted weight, which weighs at least as
much: the first term is at most MI_cap.  Its part Q ∩ supp_{h_k} is independent in N_k (Theorem 14), so the k-th term is
at most G_k.  The set Q holds no loop (Theorem 14, (3)), so leaving the loops out changes no term.  At a node, the same
with the node's matroids (Theorem 11 and Theorem 14, (2)) and its completions.  ∎

The bound is evaluated exactly in integers: weights x·S and multipliers that are multiples of 1/S (S = 2^16), the
intersection by the crate's `gf2::meet` and the greedy algorithm over GF(3), so ⌊L⌋ bounds every promotion sum whatever
produced the multipliers.  Its least value over λ is the linear program over the polytopes of A, B (with the cap) and
every N_k (Lagrangian decomposition with integral subproblems, by Edmonds' theorems); the study fits λ to it by column
generation, which is interpretation, not part of the proof.
