# The exactness of the frame model

Part of [the theory](../THEORY.md).

The frame solver's answer for game n is ΣU\* + P\*: the sources U\* plus the heaviest promotion set of the root problem
Π_n.  This file proves what that answer is.  The exactness theorem (X7) shows that the promotion sets of Π_n are exactly
the sets L of numbers up to n/2 for which U\* ∪ L is playable.  With [the bottom-layers theorem
(F3)](frame-model.md#the-bottom-layers-theorem) it leaves one open statement between the model and the game, statement
U: the answer is opt(n) exactly when some optimal set contains every member of U\* with three or more prime factors
(X8).  The holes theorem (X9) carries the same result to boards with numbers taken off.

*Words.*  Frames, frame h, moves, freedoms, closed frames, taking a number on, the candidates' tests (a) and (b), claims
and claim sequences, reduction, root frames, homes, applying a set, and promotion sets are defined in [the frame
model](frame-model.md#the-frame-model), with game 21 as the example; so are the facts on instances (P2, P3, P4 and
restriction) that the proofs use, and the results cited by label F.  [The frame lemma
(F1)](frame-model.md#the-frame-lemma) and [clean numbering (F4)](frame-model.md#clean-numbering) are used throughout.

## The exactness theorem

The model's promotion sets are exactly the games that play U\*.  The idea: a payment scheme of U\* ∪ L splits into one
scheme for each frame h with L applied, and setup's claims and drops are front and back moves of these, which keep each
playable or unplayable.  Lemmas X1 to X6 prove the pieces in order, and X7 assembles them.

**Lemma X1 (the kept-sources lemma).**  Setup's first pass of descending greedy keeps U\* ∩ D in each frame D, and every
later pass, over what a frame or a frame h has left after its claims, keeps every source left.  So setup plays U\* when
it promotes nothing.

*Proof.*  1.  A set T of sources is playable iff each T ∩ D is playable against F_D.  By P2, T needs a scheme that pays
each member in its own frame; on each edge a → b of a cycle, pay(a) is a maximal factor of b (P3), so a and b pay in
one frame, and the cycle lies in that frame's scheme.  So descending greedy over all the sources, which defines U\*, and
descending greedy over each frame keep the same sources.

2.  A frame's claims are front and back moves of the instance with members U\* ∩ D and pool F_D: a front claim's move
lists only its factor once the earlier claims are out, and a back claim's factor is listed by its move alone.  By P4 the
sources left are playable against the factors left, and so is every subset of them (restriction): descending greedy
keeps them all.  For a frame h the same holds, since U\* ∩ frame h is playable against F_h (restriction).  ∎

In game 21, descending greedy over all the sources keeps 21, 20, 19 and 18, rejects 17, which needs the factor 1 that
19 takes, keeps 16, 15 and 14, rejects 13, keeps 12 and rejects 11.  Over each frame it makes the same choices, and the
frame of layer 1 keeps 19 alone.  U\* weighs 135.

**Lemma X2 (the closed-frames lemma).**  Let S ⊆ {2, …, n} be a playable set that contains U\*, and D a closed frame.
In every scheme of S, the factors of D pay exactly the members of U\* ∩ D: none is in S or pays for another member.

*Proof.*  The members of U\* ∩ D pay with distinct factors outside S, which lie in F_D, as many as D has.  ∎

**Lemma X3 (the one-frame-forward lemma).**  Let U\* ∪ L be playable, with L ⊆ {2, …, ⌊n/2⌋}, and fix a scheme.
Every member of L is a factor of a frame numbered h ≥ 1 and pays with a factor of frame h − 1.  Every member of U\*
pays with a factor of its own frame.

*Proof.*  A member x of L is a factor of a frame D (F1), not closed (X2), so numbered h.  Its payer lies in a frame D′
in front of D (F1), not closed (X2), so numbered h − 1 (F4), and h ≥ 1.  A source's maximal factors lie in its frame.  ∎

In game 21 the optimal set is U\* ∪ {9}, and 9, a factor of frame 1, pays with 3, a factor of frame 0.

**Lemma X4 (the candidate lemma).**  If U\* ∪ L is playable, with L ⊆ {2, …, ⌊n/2⌋}, every member of L is a candidate.

*Proof.*  Let x ∈ L be a factor of D, numbered h (X3).  (a) U\* ∩ D pays with factors of D other than x (X3), since x
is in the set: restrict.  (b) The moves of frame h − 1 and x pay with factors of frame h − 1 (X3): restrict.  ∎

The claims lemma says three things about a claim sequence: a candidate is never claimed; a back claim's factor is
never needed to pay for a candidate; and so, whatever set of candidates is promoted, the claims stay front and back
moves of the frame.  Its third part is about setup, which tests (a) after its first claims.

**Lemma X5 (the claims lemma).**  Let X be a frame numbered h, or frame h, with a claim sequence.

1. No candidate is the factor of one of its claims.
2. If X is frame h, no candidate has the factor of one of its back claims among its maximal factors.  So for every set
   L of candidates, the claims are, in order, front and back moves of frame h with L applied, and none of them takes a
   member of L.
3. If X is a frame D, a factor x of D that passes test (b) passes test (a) iff no claim takes x and the remainder of
   the claims, with x taken from its pool, is playable.

*Proof of part 1.*  Let (s, f) be a claim, with f a factor of the frame D of X.

* If it is a back claim, no frame of the layer below can take f on, so neither can frame h − 1, which is a union of
  such frames ([taking a number on](frame-model.md#the-frame-model)).  So f fails test (b).
* If it is a front claim, look at the claims of X whose moves lie in D, up to (s, f), and at the instance with members
  U\* ∩ D and pool F_D ∖ {f}.  None of those claims before (s, f) takes f, so they are front and back moves of that
  instance, as in X1.  If the instance were playable, it would stay playable through them (P4); but after them s lists
  nothing, since f was its only factor left and f is not in the pool.  So the instance is unplayable, and f fails test
  (a).

*Proof of part 2.*  Let c be a candidate with the factor f of a back claim among its maximal factors.  Then c is a
factor of a frame numbered at least 1 that lies behind the frame of f, so its number is h + 1 (F4): c is a factor of a
frame with freedoms that has f as a maximal factor.  The back claim's last condition says that frame h cannot take c
on, so c fails test (b), which is a contradiction.

Now let L be a set of candidates, and take the claims in order, the earlier ones made.  Frame h with L applied has as
members the moves of frame h and the members of L promoted into it, and as pool the factors of frame h less L.

* A front claim (s, f): s lists only f among the factors not yet taken, and f is in the pool, since it is not in L
  (part 1).  It is a front move.
* A back claim (s, f): f is in the pool, since it is not in L (part 1).  Of the moves of frame h left, only s lists it,
  by the claim's rule, and no member of L lists it, by the first sentence of this part.  It is a back move.

*Proof of part 3.*  In either direction no claim takes x: if x passes (a), by part 1, since x passes (b) too, and in the
other direction by hypothesis.  Then the claims are front and back moves of the instance with members U\* ∩ D and pool
F_D ∖ {x}, as in part 1, and they keep it playable or unplayable (P4).  So that instance is playable iff the remainder
of the claims, with x taken from its pool, is.  ∎

In game 21 the claims of frame 0 and frame 1 are the back claims (15, 5) and (20, 10).  With L = {9}, frame 0 has the
moves 14, 15, 21 and 9 against 2, 3, 5 and 7, and only 15 lists 5: (15, 5) is a back move.  Frame 1 has the moves 12,
18 and 20 against 4, 6 and 10, and only 20 lists 10: (20, 10) is a back move.  What is left is root frame 0 and root
frame 1 with 9 applied.

*Why test (b) is needed.*  Without it a back claim could hide a cycle.  At n = 98, the smallest board where this
happens, frame 0 has the back claim (77, 11), with 77 = 7·11; and 22 = 2·11, a factor of frame 1, passes (a).  The
moves frame 0 keeps after its claims, 62, 93, 57, 95, 65 and 91, form a path on the factors 2, 31, 3, 19, 5, 13 and 7.
With 11 gone, 22 would take the factor 2 and leave root frame 0 winnable, and root frame 1 does not need 22.  On the
board, though, 22 and 77 close the path into a [core](playability.md#cores-and-the-residual) of eight numbers on eight
factors, each listed twice, so U\* ∪ {22} is not playable.  Only test (b) keeps 22 out: frame 0 cannot take 22 on, so
22 is no candidate, and the back claim's last condition holds.

**Lemma X6 (the reduction-step lemma).**  The drops that reduce a root frame are front claims, so the claims lemma (X5)
holds for frame h's claims followed by its drops.  So every candidate of a frame numbered h has the home h, and for
every set L of candidates, root frame h with L applied is winnable iff frame h with L applied is playable.

*Proof.*  A drop pairs a source m of frame h with its one live factor, the only maximal factor of m not taken before: a
front claim.  By X5, root frame h with L applied is frame h with L applied less front and back moves, none taking a
candidate.  ∎

**Theorem X7 (the exactness theorem).**  For every L ⊆ {2, …, ⌊n/2⌋}, U\* ∪ L is playable iff L is a promotion set
of Π_n, and then Σ(U\* ∪ L) = ΣU\* + ΣL.

*Proof.*  *Only if.*  Let U\* ∪ L be playable, with an acyclic scheme.

1. Every member of L is a candidate (X4), with a home (X6).
2. The members of frame h with L applied pay with factors of frame h outside L (X3), so by restriction frame h with L
   applied is playable, and root frame h with L applied is winnable (X6).

*If.*  Let L be a promotion set.

3. Every frame h with L applied is playable (X6).  Join acyclic schemes of these, and of U\* ∩ D against F_D for each
   closed frame D.  The joint scheme pays every source in its frame, and every member of L, a candidate of a frame
   numbered h, in frame h − 1, with distinct factors outside U\* ∪ L.
4. On each edge a → b of a cycle, pay(a) is a maximal factor of b (P3).  If b is a source, pay(a) lies in b's frame,
   so a pays where b does.  If b ∈ L is a factor of a frame numbered h, b pays in frame h − 1, and a in frame h − 1
   (F4) or in a closed frame.
5. A cycle through a source of a closed frame D lies in D's scheme: going backward, each predecessor pays in D, and
   only D's sources do.
6. Any other cycle pays only in frames h, and by step 4 each member pays where its predecessor does.  So the cycle lies
   in one frame h's scheme.  Steps 5 and 6 leave no cycle, and U\* ∪ L is playable (P2), with the weight ΣU\* + ΣL.  ∎

In game 21, {9} is a promotion set and U\* ∪ {9} is the optimal set, of weight 135 + 9 = 144; {4, 9} is no promotion
set, and U\* ∪ {4, 9} is not playable, since frame 0 with {4, 9} applied has five moves for its four factors.

## Statement U

**Statement U, at n.**  Some optimal set of n contains U\*_{≥3}.

U is a statement about the game, open in general, and it is all that stands between the model and opt(n):

**Corollary X8 (the corollary of U).**  ΣU\* + P\*(Π_n) ≤ opt(n), with equality iff U holds at n.

*Proof.*  1.  Every playable set S ⊇ U\* is U\* ∪ L with L ⊆ {2, …, ⌊n/2⌋}: descending greedy rejected each other
source s because the kept sources above s with s are unplayable, and so is any set containing them.

2. By X7, ΣU\* + P\* is the largest weight of a playable set containing U\*, at most opt(n).
3. U\*_1 = {q(n)}, since descending greedy keeps the largest of the primes above n/2, which all need the factor 1.  So
   by F3 every optimal set contains U\*_1 and U\*_2.
4. If U holds, an optimal set contains all of U\*, so opt(n) ≤ ΣU\* + P\*.  If not, no optimal set contains U\*, so
   every playable set that contains U\* weighs less than opt(n).  ∎

In game 21, U\*_{≥3} is {12, 16, 18, 20}, the optimal set plays them all, and ΣU\* + P\* = 144 = opt(21).

The solver's results that rest on a search of Π_n assume U at n ([which they
are](../../results/README.md#what-supports-the-scores)).  The standing of U is in [the table of
statements](statements.md#the-statements), and its evidence in [evidence.md](evidence.md).  In brief, U holds wherever
the prover has proven opt(n) on the whole board.  Descending greedy rejects few sources of three or more prime factors:
none of four or more on the boards the solver supports, and none of layer 3 before n = 741.  [The barred bound
(W2)](parity-bound.md#the-barred-bound) certifies most members of U\*_{≥3} game by game, never all ([the
shares](../../solver/noframe/docs/proofs.md#the-upper-part-game-by-game)), and [the certificate theorem
(E4)](extended-problem.md#the-certificate-theorem) settles the rest by a search.

### Where a proof stops

This is a sketch.  Let an optimal S miss a member of U\*_{≥3}, and d be the largest source where S and U\* disagree, in
a layer k ≥ 3 (F3).  If d were in S, S would hold d with the members of U\* above d in its frame, which descending
greedy found unplayable together.  So d ∈ U\* ∖ S, S_k + d has a [core](playability.md#the-core-theorem) in its layer
instance ([the layer theorem (F2)](frame-model.md#the-layer-theorem)), and every core holds d.  If one member v < d lies
in every minimal core, S − v + d is playable and heavier, and so is S − y + d if giving up one number y of layer k − 1
breaks every core.  A counterexample thus needs d in two minimal cores with no common
[victim](sources.md#the-victim-law) below d, the phenomenon of [the frontier theorem
(S3)](sources.md#the-frontier-theorem) inside a layer.  Where the cores are graph-like the argument closes, as in layer
2; the gap is the [odd cores](playability.md#odd-cores).

## Boards with holes

The model applies unchanged to a board with numbers taken off, and a bound on opt(n) computed on such boards rests on
it.  Let H ⊆ {1, …, n} be the *holes* and B = {1, …, n} ∖ H.  On B a move needs one of its maximal factors in the pot,
a hole is never in the pot, and a move takes all its divisors in the pot.  On the full board this *maximal-factor rule*
changes nothing, by [the lifting lemma (P1)](playability.md#the-lifting-lemma); on B it can forbid a move.  On the board
1 to 9 without 3, the move 9 has the divisor 1 in the pot, but its one maximal factor, 3, is a hole.  The frames of B
join each number of B above n/2 to its maximal factors in B; a number of B up to n/2 that joins no source is a frame
alone, one layer above that number.  U\*(B) is what descending greedy keeps of the sources of B, and Π(B) is built from
these frames as Π_n is.

**Theorem X9 (the holes theorem).**  For every L ⊆ B ∩ {2, …, ⌊n/2⌋}, U\*(B) ∪ L is playable on B iff L is a promotion
set of Π(B), and then it weighs ΣU\*(B) + ΣL.  So ΣU\*(B) + P\*(Π(B)) is the best score on B exactly when some best
game of B plays all of U\*(B).

*Proof.*  On B a legal game pays each move with a maximal factor in the pot by the rule itself, and a topological order
of an acyclic scheme is a legal game, so P2 holds with the pool leaving out the holes; P3 and P4 are about instances.
F1 holds by construction where it used Bertrand's postulate, and F2, F4, F5 and X1 to X7 follow as before, with U\*(B)
for U\*.  The last sentence is X8's argument without its step 3.  ∎

The bottom-layers theorem (F3) is not claimed on B, since [the prime lemma (N1)](neighbors.md#the-prime-lemma) is about
the full board; so B needs all of U\*(B) played, not only its part in layers 3 and up.  On B a layer between k₀ and the
highest with freedoms can hold only closed frames; its frame h is then empty, and nothing is promoted through it.
