# The extended problem

Part of [the theory](../THEORY.md).

The *extended problem* Π⁺(O) is the frame model with some sources of three or more prime factors left optional instead
of fixed.  It is exact as the frame model is (Theorem M⁺), so a search of it that finds nothing better than a recorded
score certifies U at that game (the corollary at the end of this section).  Let O ⊆ U\*_{≥3} be such a set, the *open*
sources; F = U\* ∖ O the *fixed* ones (U\*_1 and U\*_2 among them); and R the sources that greedy rejected in the frames
that hold a member of O.  "Frame" means here a frame as setup forms it, before any merge: a component of Lemma Ω, in one
layer.

*The problem Π⁺(O)* (`solver/frame/src/frames/extended.rs`):

* a frame C has the *freedoms⁺* |factors(C)| − |F ∩ C|, never negative (F ∩ C ⊆ U\* ∩ C is playable); one with none is
  *inert*;
* with ℓ₀ the lowest layer of a frame with freedoms⁺, frame h of Π⁺(O) is the union of the frames with freedoms⁺ of
  layer ℓ₀ + h, with their fixed sources as moves and all their factors (empty if there is none).  Nothing is claimed
  (Lemma M5⁺, below, says which claims of `unzip` can be kept);
* the *two-sided candidates* are the factors x of the frames h ≥ 1 that pass setup's two tests with F in place of the
  remaining sources: (a) F ∩ C_x is playable against the factors of C_x less x (C_x the frame holding x), and (b) some
  frame C' with freedoms⁺ that holds a maximal factor of x can take x on: F ∩ C' and x are playable against the factors
  of C';
* the *one-sided candidates* are O ∪ R, each a move of the frame of its layer if it is kept;
* the root is reduced as Π_n's is, and in_the_bag⁺ = ΣF;
* a *promotion set* is a pair (K, L) of one-sided and two-sided candidates, each member of L a factor of a frame h ≥ 1
  (its home), such that every frame is winnable with the members of K in it added to its moves, the members of L of home
  h + 1 added to its moves, and those of home h taken from its factors.

With O = ∅, Π⁺(∅) is the frame model without `unzip`: its frames are Π_n's with the claimed pairs put back, so the
freedoms and `max_total` are the same, and so are the candidates.  A claimed factor is a candidate of neither: a back
claim's factor cannot go forward, which fails (b), and a front claim's leaves its source nothing in (a), the earlier
claims being front and back moves of that instance.  For any other factor the claims are front and back moves of the
instance of (a) with and without them, so (a) gives the same answer on the frame with its claims and on its remaining
sources and factors (Theorem 3).

**Theorem M⁺ (the extended problem is exact).**  Let S ⊆ {2, …, n} contain F, with no prime up to n/2 and no source of
layer 1 or 2 outside F.  Then S is playable iff (K, L) is a promotion set of Π⁺(O), where K is the sources of S outside
F and L = S ∩ [2, n/2]; and then ΣS = in_the_bag⁺ + ΣK + ΣL.

*Proof.*  It is Theorem M's, with F for U\* and nothing claimed (so no M5).  (Only if.)  Let S be playable, with an
acyclic payment scheme.

* The optional sources (K ⊆ O ∪ R).  A source s of K is not in U\* ∖ O = F, so it is open or rejected.  If greedy
  rejected s in a frame C that holds no member of O, then U\* ∩ C ⊆ F ⊆ S, and greedy found the members of U\* ∩ C above
  s, with s, unplayable against all of C's factors; in S they can pay only with factors of C (their maximal factors are
  C's), so S is not playable (restrict the scheme).
* Inert frames take no part (M2): the members of F ∩ C pay with distinct factors of C, as many as there are, so no
  factor of C is in S or pays for anything else, and no member of K lies in C.
* One frame forward (M3): x ∈ L is a factor of a frame A of layer Ω(x) + 1 and pays with a maximal factor y, a factor of
  a frame X of layer Ω(x).  Neither is inert (x is in S, and y pays for x), so A lies in frame h = Ω(x) + 1 − ℓ₀ of
  Π⁺(O) and X in frame h − 1: h ≥ 1, and x pays in the frame it is promoted into.  A member of F or K pays in its own
  frame.
* The tests are necessary (M4): restrict the scheme to F ∩ C_x, which pays with factors of C_x other than x, and to F ∩
  X and x, which pay with factors of X.  So L is made of two-sided candidates.
* The reduction (M6): a move of F left with one live factor in a frame of the root, once the earlier pairs are gone, is
  a front move of that frame's instance (its members F, K and the members of L promoted into it; its pool its factors
  less the members of L of that home), so no member of L is reduced away, and what is left of each frame with (K, L)
  applied is playable (Theorem 3): winnable.

(If.)  Let (K, L) be a promotion set.  In each frame, the reductions and then a peel of what is left with (K, L) applied
pair every member of its instance by front and back moves: an acyclic scheme (Theorem 2).  Inert frames pay their fixed
sources inside.  The payments are distinct maximal factors outside S (the frames' factors are disjoint, and the members
of L are taken from their homes' factors).  A cycle of the union's precedence is flat, and a flat edge a → b has pay(a)
a maximal factor of b, which puts b in the frame where a pays: b is a source of that frame (of F or K), or a member of L
promoted into it (the numbering by layer).  If a pays in an inert frame, a is one of its sources, and every edge into a
comes from a member paying in that frame, so a cycle through it stays in it.  So every cycle lies in one frame's scheme,
which is acyclic: S is playable (Theorem 1), and ΣS = ΣF + ΣK + ΣL.  ∎

The conditions on S cost nothing: a playable S ⊇ F holds no other prime (q pays with 1, the one factor of the frame of
the primes above n/2) and no other source of layer 2 (the first point's argument).  With O = ∅, Theorem M⁺ is Theorem M
without its lemma M5: the claims of `unzip` are needed for speed, not for exactness.  The problem Π⁺(∅) has the form of
the frame model's problem (frames of Taxman Mini numbered so that a candidate of frame h is promoted into frame h − 1, a
reduced root, candidates), so the search, its bounds and its tactics apply to it unchanged: Theorems 11 and 13, the
reduction lemma, the conflicts and the nogoods use nothing else.  The search takes the one-sided candidates too, each as
a possible move of its frame (solver/docs/design/extended-mode.md, "Extended mode: one-sided candidates"), and a reference
predicate (plain peels of the unreduced frames) decides the promotion sets of Π⁺(O) for the tests.

*The claims of `unzip` in Π⁺(O).*  They cannot be reused as they are: a back claim (s, f) of Π_n is safe because no
candidate of Π_n lists f (M5), and Π⁺(O) has more candidates.  A frame of f's layer that holds an open source has more
freedoms⁺, can take on composites of f that Π_n's frame could not, and so makes them candidates: the mechanism of n = 98
above.  So the claims are kept frame by frame, with a check.  Take the claims of Π_n, the pairs (s, f) that `unzip`
settled in the merged frames of the search as the last simplification left them, in the order made, and give each the
frame C (as setup forms it, before any merge) of its source s, which holds f too (s lists only factors of its own
frame).  The frame C *keeps* its claims if it holds no member of O ∪ R and each of its back claims (s, f) passes the
*check*: f is not a two-sided candidate of Π⁺(O), and no two-sided candidate of Π⁺(O) has f among its maximal factors.
*Π⁺(O) with claims* takes the pairs of the frames that keep them out of their frames of Π⁺(O) (s from the moves, f from
the factors) before the root is reduced; everything else, the candidates and in_the_bag⁺ among it, is Π⁺(O)'s.  Let
I_h(K, L) be frame h's instance in Theorem M⁺'s proof: its members are F ∩ frame h, the members of K of frame h and the
members of L promoted into it, and its pool is frame h's factors less the members of L of home h.

**Lemma M5⁺ (the claims kept are moves of the instances).**  Let C be a frame that keeps its claims, in frame h of
Π⁺(O), and (K, L) with K ⊆ O ∪ R and L ⊆ the two-sided candidates such that either S = F ∪ K ∪ L is playable (S as in
Theorem M⁺) or (K, L) is a promotion set of Π⁺(O) with claims.  Then the claims of C, in the order made, are front and
back moves of I_h(K, L), and no claimed factor is in L.

*Proof.*  The members of I_h(K, L) that list factors of C are the sources of C, all of them fixed (C holds no member of
O, so F ∩ C = U\* ∩ C), and members of L promoted into frame h: no member of K lies in C (C holds no member of O ∪ R),
and a member of K lists only factors of its own frame.  The claims of other frames, made in between, touch none of C's
sources and factors.  Take C's claims in order, the earlier ones made (front and back moves keep a playable instance
playable, Theorem 3).

*Back claim (s, f).*  When `unzip` made it, s was the only remaining source of its merged frame that wanted f, and the
remaining sources of C were U\* ∩ C less the sources of the earlier claims (M1: the passes after the first keep every
source).  In I_h(K, L) after the earlier moves, s is the only source of C that lists f, and no member of L lists f, by
the check.  And f is in the pool: by the check f is no candidate, so f ∉ L.  It is a back move.

*Front claim (s, f).*  When `unzip` made it, f was the only factor of s not claimed before, so after the earlier moves s
lists f alone in I_h(K, L), or nothing if f ∈ L.  If S is playable, f ∈ L would leave s nothing in a residual that the
earlier moves kept playable; if (K, L) is a promotion set, f is in no frame of the root, so it has no home, and every
member of L has one.  So f ∉ L, and the claim is a front move.  ∎

So Theorem M⁺ holds for Π⁺(O) with claims, with the same proof: (only if) by the lemma the claims kept are moves of each
playable I_h(K, L) and take nothing from L, so what is left, the frame with claims with (K, L) applied, is playable
(Theorem 3), and the reduction is M6's; (if) in each frame the claims kept, the reductions and a peel of what is left
pair every member of I_h(K, L) by front and back moves (the lemma, for the claims), an acyclic scheme (Theorem 2), and
the rest of the proof is unchanged.

**Corollary (the formulation does not matter).**  The two formulations, Π⁺(O) and Π⁺(O) with claims, have the same
promotion sets, and so the same heaviest promotion set, worth the same: by Theorem M⁺ for each, the promotion sets of
both are the (K, L) with F ∪ K ∪ L playable, and in_the_bag⁺ = ΣF in both.  So a certificate made in one formulation
bounds the heaviest promotion set of the other.

With O = ∅ every frame keeps its claims, and Π⁺(∅) with claims is Π_n: every back claim of Π_n passes the check (its
factor cannot go forward, so it fails test (b) and is no candidate, and M5 shows that no candidate lists it), Π⁺(∅) has
Π_n's candidates, and each frame of Π⁺(∅), the union of its parts' sources and factors less the pairs, is the merged
frame's remaining sources and factors.  The census of the frames checks that the two problems the search is given are
equal on every board to 8191 (`solver/frame/examples/structure.rs`, check H).

*Theorem 11 for Π⁺.*  Give each one-sided candidate s, of frame h, the vector v_h(s) of its maximal factors among frame
h's factors in the matroid of frame h (A or B, by the parity of h), and make it a coloop of the other matroid: a
coordinate of its own, which no other element has.  Then every promotion set (K, L) of Π⁺(O) is independent in both
matroids, and at a node reached by promoting P a completion is independent in their contractions by P.  *Proof.*
Theorem 11's, with the movers of frame h being its moves, the members of L promoted into it and the members of K of
frame h: a winnable frame peels completely, so their vectors, with the unit vectors of the members of L promoted out of
h, are independent.  A member of K touches no other frame, so in the other matroid it is constrained by nothing, which
is what a coloop is: a set is independent with it exactly when it is without it.  Contraction by a member of K is
contraction by its vector in its frame and by its own coordinate in the other matroid.  ∎  (In the code the coloops of
each matroid are blocks of their own after the frames' blocks, `gf2`'s *free blocks*; nothing else of the bound
changes.)

**Corollary (a certificate of U).**  Let V be the score of a game of n and F a set with U\*_1 ∪ U\*_2 ⊆ F ⊆ U\*, each of
whose members u of layer 3 or more is *certified*: no playable set that holds q alone in layer 1 and U\*_2 among the
sources of layer 2, but not u, weighs more than V (the parity bound of Theorem 10 with those roles and u barred is at
most V, say).  If no promotion set of Π⁺(U\* ∖ F) is worth more than V − in_the_bag⁺, then opt(n) = V.

*Proof.*  If opt(n) > V, an optimal set S satisfies Theorem B, so by the certificates it holds every member of F, and by
Theorem M⁺ it is a promotion set of Π⁺(U\* ∖ F) worth ΣS − in_the_bag⁺ > V − in_the_bag⁺.  ∎

In a search for opt(n) rather than a check of V, V can be the search's starting incumbent: members certified
against it cover every set that beats it.

Equal problems have equal heaviest promotion sets, so if Π⁺ of game n is Π⁺ of an earlier game m whose certificate
showed its heaviest set worth V(m) − in_the_bag⁺(m), and V(n) − in_the_bag⁺(n) ≥ V(m) − in_the_bag⁺(m), then no
promotion set of Π⁺ of n is worth more than V(n) − in_the_bag⁺(n), and with n's own certified members the corollary
gives opt(n) = V(n) without a search (`taxman certify`'s `same_problem`).  The two problems are compared in one
formulation, with the claims of Lemma M5⁺ or without, whatever formulation m's certificate was made in: the heaviest
promotion set of Π⁺ of m is the same in both (the corollary of Lemma M5⁺).  With the claims more consecutive games pose
the same problem, since a claimed pair that only one of them has is no longer part of it.
