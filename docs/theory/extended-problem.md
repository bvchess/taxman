# The extended problem

Part of [the theory](../THEORY.md).

The *extended problem* Π⁺(O) is the frame model with a set O of members of U\*_{≥3} left optional instead of fixed.  It
is exact as the frame model is: the extended exactness theorem (E1).  So a search of it that finds nothing better than
a recorded score proves the score optimal, once the other members of U\*_{≥3} are certified: the certificate theorem
(E4).  That is how a certificate of statement U is made, game by game.

*Words.*  Frames, their moves and factors, frame h, taking a number on, the tests (a) and (b), claims, reduction, homes,
applying a set, winnable pairs and promotion sets are those of [the frame model](frame-model.md#the-frame-model), and
the results cited by label F or X are those of that file and of [exactness.md](exactness.md).  A frame is a component of
the frame graph, which lies in one layer ([the frame lemma (F1)](frame-model.md#the-frame-lemma)), and a frame h is a
union of frames.

## The extended problem

Let O ⊆ U\*_{≥3} be the *open* sources, and Φ = U\* ∖ O the *fixed* ones, which include U\*_1 and U\*_2.  Let J be the
sources that descending greedy rejected in the frames that hold a member of O.

* The *freedoms⁺* of a frame D are |F_D| − |Φ ∩ D| ≥ 0, and D is *inert* when it has none.
* With k₀ the lowest layer of a frame with freedoms⁺, a frame with freedoms⁺ in layer k gets the number k − k₀, and
  *frame h* of Π⁺(O) is the union of the frames numbered h, with the moves Φ ∩ frame h and the factors F_h.  It
  *can take x on* when its moves with x are playable against F_h.
* A *two-sided candidate* is a factor x of a frame D numbered h ≥ 1 that passes the tests with Φ for U\*: (a) Φ ∩ D is
  playable against F_D ∖ {x}; (b) frame h − 1 can take x on.
* The *one-sided candidates* are the members of O ∪ J.  The *own frame* of one is the frame h that holds the frame of
  that source; this frame has freedoms⁺, since it holds an open source.
* The root frame h is frame h, reduced; nothing is claimed.  ΣΦ is the score with no promotions.
* *Applying* (K, L) to a frame h adds to its moves the members of K whose own frame it is and the members of L in
  F_{h+1}, and removes from its factors the members of L in F_h; on a root frame it does the same by home.  A
  *promotion set* of Π⁺(O) is a pair (K, L), K ⊆ O ∪ J and L a set of two-sided candidates each with a home, such that
  every root frame is winnable with (K, L) applied.  It is worth ΣK + ΣL, and P\*(Π⁺(O)) is the most a promotion set
  is worth.

At n = 24 with O = {16, 20, 24}, Φ is {14, 15, 18, 21, 22, 23}, with ΣΦ = 113, and J is empty.  Frame 0 has the moves
14, 15, 21 and 22 and the factors 2, 3, 5, 7 and 11; frame 1, the union of the frames of 18 and of 20, has the move 18
and the factors 4, 6, 9 and 10; frame 2 has no move and the factors 8 and 12.  The two-sided candidates are 4, 9, 8 and
12, and the one-sided ones are 20, of frame 1, and 16 and 24, of frame 2.  In the frame model of 24 the frame of 16 and
24 is closed; opening its sources gives it two freedoms⁺, and makes its factors candidates.

With O = ∅, Φ is U\*, freedoms⁺ are freedoms, J is empty, and Π⁺(∅) has the frames h and the candidates of Π_n, with
nothing claimed.  The formulation corollary (E3) shows that it has Π_n's promotion sets.

## The extended exactness theorem

**Theorem E1 (the extended exactness theorem).**  Let O ⊆ U\*_{≥3} and Φ = U\* ∖ O, and let S ⊆ {2, …, n} contain Φ.
Write K for the sources of S outside Φ and L = S ∩ {2, …, ⌊n/2⌋}.  Then S is playable iff (K, L) is a promotion set of
Π⁺(O), and then ΣS = ΣΦ + ΣK + ΣL.

*Proof.*  It is the proof of [the exactness theorem (X7)](exactness.md#the-exactness-theorem) and its lemmas, read
with Φ for U\*, freedoms⁺ for freedoms, inert frames for closed ones and the numbering of Π⁺(O).  The new piece is K,
whose members are moves of their own frames.

*Only if.*  Let S be playable, with an acyclic scheme.

1. K ⊆ O ∪ J.  A source s of K is open, or descending greedy rejected it in its frame D.  If D held no open source,
   U\* ∩ D would lie in Φ ⊆ S.  Descending greedy found s and the members of U\* ∩ D above it unplayable against F_D,
   yet in S they pay with factors of D, and restriction would make them playable.  So D holds an open source: s ∈ J.
2. As in [the closed-frames lemma (X2)](exactness.md#the-exactness-theorem), the factors of an inert frame D pay the
   members of Φ ∩ D: none is in S or pays for another member, and no member of K lies in D.
3. As in [the one-frame-forward lemma (X3)](exactness.md#the-exactness-theorem), each member of L is a factor of a frame
   numbered h ≥ 1 and pays in frame h − 1; each member of Φ or K pays in its own frame.
4. As in [the candidate lemma (X4)](exactness.md#the-exactness-theorem), each member x of L, a factor of a frame D
   numbered h, is a two-sided candidate: restrict to Φ ∩ D for (a), and to x and the moves of frame h − 1 for (b).
5. The reduction of frame h drops pairs (m, f), m ∈ Φ, with f the only maximal factor of m not taken before.  As for a
   front claim in [the claims lemma (X5)](exactness.md#the-exactness-theorem), f fails (a): the earlier drops in f's
   frame are front moves of the instance with members Φ ∩ D and pool F_D ∖ {f}, after which m would list nothing.  So
   every two-sided candidate has a home, and root frame h with (K, L) applied is frame h with (K, L) applied less front
   moves, which keep it playable or unplayable (P4).
6. As in X7, the members of frame h with (K, L) applied pay with factors of frame h outside L: by restriction and by
   step 5, root frame h with (K, L) applied is winnable.

*If.*  Let (K, L) be a promotion set.

7. Each frame h with (K, L) applied is playable (step 5).  Join acyclic schemes of these, and of Φ ∩ D against F_D for
   each inert frame D.  The joint scheme pays every member of S with distinct factors outside S.  A member of K is a
   source of its own frame and pays there, so steps 4 to 6 of X7's proof apply as they stand: a cycle through a
   source of an inert frame lies in that frame's scheme, and any other cycle in one frame h's scheme.  So S is
   playable, and ΣS = ΣΦ + ΣK + ΣL.  ∎

The extended problem has the frame model's form, with one more kind of candidate.  A one-sided member counts as one
more move of its own frame, and the rules of [search-rules.md](search-rules.md) and [the frame bound for the extended
problem (B3)](frame-bounds.md#the-frame-bound-for-the-extended-problem) apply with that reading.

## The kept-claims lemma

The claims of Π_n shrink its frames, and let consecutive games pose the same problem more often (E5), so the extended
problem keeps those it safely can.  It cannot keep them all.  A back claim (s, f) of Π_n is safe because no candidate
of Π_n has f among its maximal factors (X5), and Π⁺(O) has more candidates: opening sources gives frames more
freedoms⁺, so that a closed frame can be numbered and a frame can take more on.  At n = 24 with O = {16, 20, 24}, the
factor 12 = 2²·3 becomes a two-sided candidate, and its maximal factor 6 is the factor of Π_24's back claim (18, 6).
Kept, that claim would leave 8 and 12 both needing the factor 4 in frame 1, and lose the promotion set (∅, {8, 12}),
although Φ ∪ {8, 12} is playable.

So the claims are kept frame by frame, with a check.  Take the claims of Π_n's frames h, not its drops, and give each
the frame D of its move, which holds its factor too.  D *keeps* its claims if it holds no member of O ∪ J, and each of
its back claims (s, f) passes the *check*: f is not a two-sided candidate of Π⁺(O), and no two-sided candidate has f
among its maximal factors.  *Π⁺(O) with claims* takes the claims of the frames that keep theirs out of its frames h,
before the root is reduced; its candidates and ΣΦ are Π⁺(O)'s.

**Lemma E2 (the kept-claims lemma).**  Let a frame D keep its claims, D in frame h of Π⁺(O); let K ⊆ O ∪ J, and L a
set of two-sided candidates.  Then D's claims are, in order, front and back moves of frame h with (K, L) applied, and
none takes a member of L.

*Proof.*  The members of that instance that list factors of D are the members of Φ ∩ D, which is U\* ∩ D since D
holds no open source, and the members of L promoted into frame h.  No member of K lies in D, since D holds no member
of O ∪ J, and a member of K lists only factors of its own frame.  Claims of other frames touch none of D's moves and
factors.  Take D's claims in order, the earlier ones made.

* A front claim (s, f): s lists only f.  And f ∉ L: as in X5, the claims of D before it are front and back moves of
  the instance with members U\* ∩ D and pool F_D ∖ {f}, after which s would list nothing, so f fails (a).
* A back claim (s, f): s is the only move of D left with f as a maximal factor, by the claim's rule, since the moves
  left are U\* ∩ D less those of the earlier claims ([the kept-sources lemma
  (X1)](exactness.md#the-exactness-theorem)).  By the check no member of L lists f, and f ∉ L.  ∎

**Corollary E3 (the formulation corollary).**  Π⁺(O) and Π⁺(O) with claims have the same promotion sets: in both,
they are the pairs (K, L), K ⊆ O ∪ J and L a set of two-sided candidates, with every frame h with (K, L) applied
playable.  So E1 holds for both, and both have the same heaviest promotion set.  And Π⁺(∅) with claims is Π_n.

*Proof.*  A root frame h of Π⁺(O) with claims is frame h less the kept claims of its frames, then reduced.  The kept
claims are front and back moves of frame h with (K, L) applied, none taking a member of L (E2).  The drops that follow
are front moves taking no two-sided candidate, as in step 5 of E1's proof, with the kept claims among the earlier
moves.  So in both formulations every two-sided candidate has a home, and root frame h with (K, L) applied is winnable
iff frame h with (K, L) applied is playable (P4).  With O = ∅, Π⁺(∅) has Π_n's frames h and candidates, and every
frame keeps its claims, since a back claim's factor passes the check (X5).  So the frames h of Π⁺(∅) with claims are
those of Π_n less its claims, which are winnable and reduce to Π_n's root frames in any order ([the reduction lemma
(F5)](frame-model.md#the-reduction-lemma)).  ∎

So a certificate made in one formulation bounds the heaviest promotion set of the other.

## The certificate theorem

Let V be the score of a game of n.  A member u of U\*_{≥3} is *certified* against V when every playable set that obeys
[the bottom-layers theorem (F3)](frame-model.md#the-bottom-layers-theorem) and weighs more than V contains u.  [The
barred bound (W2)](parity-bound.md#the-barred-bound) certifies u when, with u barred, it is at most V.

**Theorem E4 (the certificate theorem).**  Let V be the score of a game of n, and Φ a set with U\*_1 ∪ U\*_2 ⊆ Φ ⊆ U\*
whose members of layer 3 or more are certified against V.  If no promotion set of Π⁺(U\* ∖ Φ) is worth more than
V − ΣΦ, then opt(n) = V.

*Proof.*  Suppose opt(n) > V.  An optimal set S obeys F3 and weighs more than V, so it contains every certified member.
By F3 it also contains U\*_2 and U\*_1 = {q(n)}, so it contains Φ.  By E1 it gives a promotion set of Π⁺(U\* ∖ Φ)
worth ΣS − ΣΦ > V − ΣΦ, a contradiction.  So opt(n) ≤ V, and V is the score of a game.  ∎

Members certified against V speak of every set that beats V, so in a search for opt(n), rather than a check of V, V
can be the search's starting incumbent.  [results/README.md](../../results/README.md#certificates) lists the games
with a certificate.

Two problems are *equal* when they have the same frames, the same two-sided candidates, and the same one-sided
candidates with the same own frames.  Equal problems have the same promotion sets, worth the same.

**Corollary E5 (the same-problem corollary).**  Let V(m) and V(n) be scores of games of m and of n, and Φ_m and Φ_n
sets as in E4, the members of Φ_n of layer 3 or more certified against V(n).  Suppose that Π⁺(U\* ∖ Φ_n) of game n
equals Π⁺(U\* ∖ Φ_m) of game m in one formulation, that no promotion set of the latter is worth more than
V(m) − ΣΦ_m, and that V(n) − ΣΦ_n ≥ V(m) − ΣΦ_m.  Then opt(n) = V(n).

*Proof.*  By E3 each problem has the same heaviest promotion set in both formulations, whichever one m's bound was
shown in.  In the formulation where the two are equal, no promotion set of game n's problem is worth more than
V(m) − ΣΦ_m ≤ V(n) − ΣΦ_n, and E4 gives opt(n) = V(n) without a search of game n.  ∎

With the claims more consecutive games pose the same problem, since a claimed pair that only one of them has is no
longer part of it.
