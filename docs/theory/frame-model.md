# The frame model

Part of [the theory](../THEORY.md).

The frame solver does not search the games of n.  It splits the board into *frames*, fixes the sources it plays to
U\*, and searches only over which factors to *promote*, that is, to play as moves.  This file defines that model, the
*frame model*, and proves that its promotion sets are exactly the sets L of numbers up to n/2 with U\* ∪ L playable:
the exactness theorem (F12).  With the bottom-layers theorem (F3), one open statement is left between the model and
the game, statement U: the solver's answer ΣU\* + P\* is opt(n) exactly when some optimal set contains every member
of U\* with three or more prime factors (F13).  [The extended problem](extended-problem.md) builds on the model, and
[search-rules.md](search-rules.md) says what a search of it proves.

*Words.*  Sources, factors, maximal factors, layers and playable sets are the [glossary's](../GLOSSARY.md), and Ω(x)
is the layer of x.  Descending greedy, and U\*, the sources it keeps, are defined in
[sources.md](sources.md#the-sources-conjecture).  U\*_k and S_k are the parts of U\* and of a set S in layer k
([notation](statements.md#notation)).

*Instances.*  The words for instances are [playability.md](playability.md#payment-schemes)'s: members, pool, lists
(a member's maximal factors in the pool), payment schemes, precedence, playable instances, and peel's front and back
moves.  A set T is *playable against* a set F when the instance with members T and pool F is playable.  Four facts
are used throughout:

* [the playability theorem (P2)](playability.md#the-playability-theorem): a set S ⊆ {2, …, n} is playable iff its own
  instance, with pool the maximal factors of its members outside S, is playable;
* [the flat-cycle lemma (P3)](playability.md#the-flat-cycle-lemma): every cycle of a scheme's precedence is flat: its
  members lie in one layer, and on each edge a → b, pay(a) is a maximal factor of b;
* [the peel theorem (P4)](playability.md#the-peel-theorem): a front move (a member with one live factor, paired with
  it) or a back move (a live factor in one list only, paired with that member) keeps an instance playable and keeps it
  unplayable, and a run of peel that pairs every member yields an acyclic scheme;
* *restriction*: if an acyclic scheme pays the members of a subset T with factors of a set F, then T is playable
  against F; and a scheme against a pool is one against any larger pool.

## Frames

The *frame graph* of game n joins each source to its maximal factors.  A *frame* is a component of it: some sources
with every factor that can pay for one of them.  F_D denotes the factors of a frame D.  A frame D is *in front of* a
frame D′ ≠ D, and D′ *behind* D, when a factor of D′ has a maximal factor in D.  Game 21 has four frames, each in front
of the next (9 has the maximal factor 3, for instance):

| layer | sources | factors |
|---|---|---|
| 1 | 11, 13, 17, 19 | 1 |
| 2 | 14, 15, 21 | 2, 3, 5, 7 |
| 3 | 12, 18, 20 | 4, 6, 9, 10 |
| 4 | 16 | 8 |

A layer can hold several frames: game 98 has three in layer 3.

### The frame lemma

**Lemma F1 (the frame lemma).**  Let n ≥ 2.  Every number of the board lies in exactly one frame.  In each frame the
sources all have one layer k and the factors the layer k − 1: the frame *lies in layer k*.  A frame in front of a frame
of layer k lies in layer k − 1.

*Proof.*  An edge lowers the layer by one.

1. Every source s ≥ 2 has a maximal factor.  Every x ≤ n/2 is a maximal factor of a source: with m = ⌊n/(2x)⌋ ≥ 1,
   Bertrand's postulate gives a prime p with m < p ≤ 2m, so n/2 < xp ≤ n.  So every number is in the graph, and the
   components partition it.
2. Each edge joins a source s to a factor s/p, with Ω(s/p) = Ω(s) − 1.  Along a path the layers alternate k, k − 1,
   k, …, so in a component the sources share a layer and the factors lie one below.
3. A factor y of a frame of layer k has Ω(y) = k − 1, and a maximal factor z of y has Ω(z) = k − 2.  Since z ≤ n/2, z
   is a factor of the frame holding it (step 1), whose sources therefore lie in layer k − 1 (step 2).  ∎

So a factor has no proper divisor in its own frame, and the maximal factors of a frame's factors lie in the frames in
front of it.

### The layer theorem

For S ⊆ {2, …, n}, the *layer instance* of S at k has the members S_k and the pool of the numbers of layer k − 1 up to
n/2 that are not in S.

**Theorem F2 (the layer theorem).**  A set S ⊆ {2, …, n} is playable iff every layer instance of S is playable.

*Proof.*  The maximal factors of a member of layer k lie in layer k − 1 and are at most n/2.  So a member of S_k has
the same list in S's own instance as in the layer instance at k, and different layers draw on disjoint pools: a
scheme of S is a union of schemes of its layer instances.  By P3 a cycle of its precedence lies in one layer, so the
union is acyclic iff each part is.  ∎

A game is thus a chain of layer games, coupled only through the numbers up to n/2 that it plays, and the frame model
is this chain with the sources fixed to U\*.  The [parity bound](parity-bound.md#the-parity-bound) groups the same
layers by parity.

### The bottom-layers theorem

In layers 1 and 2 every optimal set agrees with U\*.  Let G_n be the graph whose vertices are the primes up to n/2
and one more vertex g, and whose edges are the numbers of layer 2 up to n: pq is the edge p–q, and p² the edge p–g.

**Theorem F3 (the bottom-layers theorem).**  Let n ≥ 2 and S an optimal set of n.  Then S_1 = {q(n)}, the numbers of
S_2 above n/2 are exactly U\*_2, and S_2 is a forest in G_n.  Where the frames of layer 2 are closed (defined below),
S plays no number of layer 2 up to n/2.

*Proof.*  In layer 2 descending greedy is the greedy algorithm of a graphic matroid, and an exchange shows that S_2 is
what that algorithm keeps of S_2 ∪ U\*_2.

1. S_1 = {q(n)} by [the prime lemma (N1)](neighbors.md#the-prime-lemma), so the pool of S's layer instance at 2 is
   every prime up to n/2.
2. A set of numbers of layer 2 is playable against those primes iff it is a forest in G_n, by [the two-factor
   proposition (C3)](franklin-moniot.md#the-two-factor-proposition): these sets are the independent sets of the
   graphic matroid of G_n ([matroids](primer.md#matroids)).
3. Adding a source s of layer 2 to the kept sources changes only their layer instance at 2 (F2), so descending greedy
   keeps s iff s closes no cycle with the kept sources of layer 2: it is the matroid's [greedy
   algorithm](primer.md#the-greedy-algorithm-of-a-matroid).  That algorithm keeps an element iff the elements above it
   do not span it, since the kept ones span all the elements above it.
4. Let G be what the algorithm keeps of W = S_2 ∪ U\*_2.  The members of W above a source x of W are sources of layer
   2.  If x ∈ U\*_2 they do not span x, since all the sources of layer 2 above x do not; if x ∉ U\*_2, the members of
   U\*_2 above x span it.  So G = U\*_2 ∪ A, with A made of numbers of S_2 up to n/2.
5. With distinct weights, G is the unique heaviest independent subset of W: were there another, the largest number x
   in only one of the two would close a circuit with the other, through a member y < x outside the first, and trading
   y for x would make the other heavier.  S_2 is independent (F2 and step 2), so ΣG ≥ ΣS_2, with equality only if
   G = S_2.
6. S′ = (S ∖ S_2) ∪ G is playable by F2: at layer 2 it has the forest G against the same pool; at layer 3 the pool
   only grows, since S plays the numbers A too; the other layers do not change.  So ΣS′ ≤ ΣS, which forces
   S_2 = G = U\*_2 ∪ A.
7. The frames of layer 2 hold the primes up to n/2 as factors (F1).  When they are closed, U\*_2 has one edge for each
   of these primes, one fewer than G_n has vertices: it is a spanning tree, so A is empty.  ∎

The frames of layer 2 are closed on all but a few small boards ([evidence.md](evidence.md)).  [The barred bound
(W2)](parity-bound.md#the-barred-bound) fixes layers 1 and 2 by this theorem.

## The frame model

The model keeps the sources of U\*, leaves out the frames with nothing to spare, merges the rest by layer, and asks
which factors to promote.  The code of each step is in [the design of the frame
model](../../solver/docs/design/frame-model.md#setup-in-the-code).

**The frame's game.**  *Taxman Mini* is played on a pair W of disjoint sets, its *moves* and its *factors*, the moves in
one layer and the factors in the layer below.  A move's *live* factors are its maximal factors among W's factors.  The
moves are taken one at a time; each must have a live factor when taken, and taking it removes its live factors.  W is
*winnable* when some order takes every move.  Read as the instance with members its moves and pool its factors, W is
winnable iff it is playable.  A winning order pays each move with a live factor present at its turn, and puts b after
a whenever pay(a) divides b, or b would have removed pay(a): it is a topological order of an acyclic scheme (no move
divides another).  Conversely, in a topological order of an acyclic scheme every move that lists pay(a) comes after a,
so pay(a) is there at a's turn.

**Freedoms.**  The members of U\* ∩ D pay in F_D, so U\* ∩ D is playable against F_D (restriction).  The *moves* of a
frame D are U\* ∩ D, its *freedoms* are |F_D| − |U\* ∩ D| ≥ 0, and D is *closed* when it has none.  In game 21 the
frames of layers 1 and 4 are closed (U\* keeps only 19 of layer 1), and those of layers 2 and 3 have a freedom each.

**Numbering.**  Let k₀ be the lowest layer of a frame with freedoms.  A frame with freedoms in layer k gets the number
k − k₀, a closed frame none.  *Frame h* is the union of the frames numbered h, with the moves U\* ∩ frame h and the
factors F_h; it is empty if no frame of layer k₀ + h has freedoms.  In game 21 frame 0 is the frame of layer 2, and
frame 1 that of layer 3.  In game 20 the frames of 14 (with the factors 2 and 7) and of 15 (with 3 and 5) both lie in
layer 2, and frame 0 is their union.

### Clean numbering

**Proposition F4 (clean numbering).**  Every frame with freedoms in front of a frame numbered h is numbered h − 1.

*Proof.*  It lies one layer lower (F1), and the numbers follow the layers.  ∎

The exactness theorem uses nothing else about the numbering: a promoted number pays one layer down (F1), and F4 puts
the frames there that can pay for it in frame h − 1.

**Taking a number on.**  A frame D, or a frame h, *can take x on* when its moves with x are playable against its
factors.  A closed frame can take nothing on, since its moves use all its factors.  Frame h can take x on iff one of
its frames can.  One way, restrict a scheme to x and the moves of the frame where x pays.  The other way, join D's
scheme with x to schemes of the other frames' moves.  By P3 the predecessor on a cycle of a move of another frame D′
pays in D′, so it is a move of D′, and every cycle lies in one of the schemes joined.

**Candidates.**  A *candidate* is a factor x of a frame D numbered h ≥ 1 that passes two tests: (a) U\* ∩ D is
playable against F_D ∖ {x}; (b) frame h − 1 can take x on.  In game 21 the candidates are 4 and 9.  The factors 6 and
10 fail (b): with 6, frame 0 pairs 15 with 5 by a back move, and then its moves 14, 21 and 6 list each of the factors
2, 3 and 7 twice, so peel stalls.

**Claims.**  A *claim* (s, f) pairs a move s of a frame with a factor f of it, and takes both out.  A *claim sequence*
of a frame X, a frame D or a frame h, is a sequence of claims, each one of two kinds given those before it:

* a *front claim*: f is the only maximal factor of s that no earlier claim took;
* a *back claim*: no earlier claim took f; s is the only move of X left with f as a maximal factor; no frame of the
  layer below can take f on; and X cannot take on any factor of a frame with freedoms that has f as a maximal factor.

The last two conditions say that f cannot be promoted, and cannot pay for a number promoted into X.  In game 21 frame
0 has the back claim (15, 5): only 15 has the maximal factor 5, the frame of layer 1 is closed, and frame 0 cannot take
on 10, the one factor of frame 1 with the maximal factor 5.  Frame 1 has the back claim (20, 10).  Each condition
earns its place.  Once 15 is claimed only 21 has the maximal factor 3, but frame 0 can take on 9, which has it; and
only 18 has the maximal factor 9, but frame 0 can take 9 on.  The claim (21, 3) or (18, 9) would lose the promotion
set {9} (below).

**Reduction.**  To *reduce* a pair of Taxman Mini is to drop a move with one live factor, with that factor, until no
such move is left.

**Setup** computes the model:

1. it forms the frames, and runs descending greedy over each frame's sources against its factors;
2. it makes claims, round after round, running descending greedy again over what each frame has left, until no claim can
   be made;
3. it numbers the frames, finds the candidates, and merges the frames of each number into frame h;
4. it makes claims in each frame h, a merged frame starting with none and a frame alone in its number keeping its own;
5. it reduces each frame h less its claims.

The claims of step 2 in a frame are a claim sequence of it, and those that leave frame h are one of frame h; setup's own
tests ask at least what the rules ask.  The kept-sources lemma (F6) shows that descending greedy keeps U\* at every
pass, and the claims lemma (F10) that test (a), which setup applies after the claims of step 2, picks the same
candidates.

**The root problem.**  The *root frame h* is frame h less its claims, reduced.  The *root problem* Π_n is the root
frames and the candidates; the *root* is the root frames before any promotion.  A number's *home* is the root frame
whose factors hold it.  ΣU\* is the score with no promotions (F6, below).  In game 21, root frame 0 has the moves 14 and
21 and the factors 2, 3 and 7, and root frame 1 the moves 12 and 18 and the factors 4, 6 and 9.

**Promotion sets.**  *Applying* a set L to frame h adds to its moves the members of L in F_{h+1} and removes from its
factors those in F_h; applying L to root frame h adds the members of L whose home is h + 1 and removes those whose home
is h.  A *promotion set* of Π_n is a set L of candidates, each with a home, such that every root frame is winnable with
L applied.  *Promoting* x thus moves it one frame forward, from the factors of its home h to the moves of frame h − 1.
P\* = P\*(Π_n) is the largest ΣL over the promotion sets.  A promotion set has at most as many members as the freedoms
allow, the promotion limit ([the promotion-limit lemma (F14)](#searching-the-frame-model)).  In game 21, {4} and {9}
are promotion sets, but {4, 9} would leave root frame 0 four moves for three factors.  So P\* = 9, and
ΣU\* + P\* = 135 + 9 = 144 = opt(21).

### The reduction lemma

**Lemma F5 (the reduction lemma).**  Let W be a pair of Taxman Mini, and m a move of W whose only live factor is f.

1. Dropping m and f never changes whether the pair is winnable, now or after later edits that keep f: for every set Y
   of numbers of the moves' layer and every set Z of factors with f ∉ Z, W with Y added to its moves and Z removed from
   its factors is winnable iff W less m and f, so edited, is.
2. If W is winnable, reducing it ends at the same pair whatever the order of the drops.

*Proof.*  1.  After the edits m still has the single live factor f, so (m, f) is a front move, which keeps the pair
winnable and keeps it unwinnable (P4).

2.  By part 1 every pair reached is winnable, so no two moves have the same single live factor.  Suppose part 2 holds
for every winnable pair with fewer moves.  Two reductions of W that start with the same drop end together, by
the hypothesis on W less that pair.  Let two start with the drops (m₁, f₁) ≠ (m₂, f₂).  Then m₁ ≠ m₂ and f₁ ≠ f₂, and
either drop leaves the other available.  Take a reduction ρ of W less both pairs.  By the hypothesis, (m₁, f₁),
(m₂, f₂), ρ ends where the first reduction does, and (m₂, f₂), (m₁, f₁), ρ where the second does; and these two
coincide.  ∎

Winnability is needed in part 2: against the factor 3, the moves 6 and 9 reduce to 9 alone or to 6 alone.

## The exactness theorem

The model's promotion sets are exactly the games that play U\*.  The idea: a scheme of U\* ∪ L splits into one scheme
for each frame h with L applied, and setup's claims and drops are front and back moves of these, which keep each
playable or unplayable.  Lemmas F6 to F11 prove the pieces in order, and F12 assembles them.

**Lemma F6 (the kept-sources lemma).**  Setup's first pass of descending greedy keeps U\* ∩ D in each frame D, and every
later pass, over what a frame or a frame h has left after its claims, keeps every source left.  So setup plays U\* when
it promotes nothing.

*Proof.*  1.  A set T of sources is playable iff each T ∩ D is playable against F_D.  By P2, T needs a scheme that pays
each member in its own frame; on each edge a → b of a cycle, pay(a) is a maximal factor of b (P3), so a and b pay in one
frame, and the cycle lies in that frame's scheme.  So descending greedy over all the sources, which defines U\*, and
descending greedy over each frame keep the same sources.

2.  A frame's claims are front and back moves of the instance with members U\* ∩ D and pool F_D: a front claim's move
lists only its factor once the earlier claims are out, and a back claim's factor is listed by its move alone.  By P4 the
sources left are playable against the factors left, and so is every subset of them (restriction): descending greedy
keeps them all.  For a frame h the same holds, since U\* ∩ frame h is playable against F_h (restriction).  ∎

**Lemma F7 (the closed-frames lemma).**  Let S ⊆ {2, …, n} be a playable set that contains U\*, and D a closed frame.
In every scheme of S, the factors of D pay exactly the members of U\* ∩ D: none is in S or pays for another member.

*Proof.*  The members of U\* ∩ D pay with distinct factors outside S, which lie in F_D, as many as D has.  ∎

**Lemma F8 (the one-frame-forward lemma).**  Let U\* ∪ L be playable, with L ⊆ {2, …, ⌊n/2⌋}, and fix a scheme.
Every member of L is a factor of a frame numbered h ≥ 1 and pays with a factor of frame h − 1.  Every member of U\*
pays with a factor of its own frame.

*Proof.*  A member x of L is a factor of a frame D (F1), not closed (F7), so numbered h.  Its payer lies in a frame D′
in front of D (F1), not closed (F7), so numbered h − 1 (F4), and h ≥ 1.  A source's maximal factors lie in its frame.  ∎

**Lemma F9 (the candidate lemma).**  If U\* ∪ L is playable, with L ⊆ {2, …, ⌊n/2⌋}, every member of L is a candidate.

*Proof.*  Let x ∈ L be a factor of D, numbered h (F8).  (a) U\* ∩ D pays with factors of D other than x (F8):
restrict.  (b) The moves of frame h − 1 and x pay with factors of frame h − 1 (F8): restrict.  ∎

**Lemma F10 (the claims lemma).**  Let X be a frame numbered h, or frame h, with a claim sequence.

1. No candidate is the factor of one of its claims.
2. If X is frame h, no candidate has a back claim's factor among its maximal factors; and for every set L of
   candidates the claims are, in order, front and back moves of frame h with L applied, none taking a member of L.
3. If X is a frame D, a factor x of D that passes (b) passes (a) iff no claim takes x and the
   [remainder](playability.md#peel) of the claims, with x taken from its pool, is playable.

*Proof.*  1.  Let f, a factor of the frame D of X, be claimed.  No frame of the layer below can take a back claim's f
on, so neither can frame h − 1 (taking a number on), and f fails (b).  For a front claim (s, f), the claims of X with
moves in D, up to (s, f), are front and back moves of the instance with members U\* ∩ D and pool F_D ∖ {f}, as in F6.
If that instance were playable it would stay so (P4), yet s would list nothing.  So f fails (a).

2.  A candidate c with a back claim's factor f among its maximal factors is a factor of a numbered frame behind f's,
so of a frame numbered h + 1 (F4).  The claim says that frame h cannot take c on, so c fails (b).  Now apply L.  A
front claim's move lists only its factor, which is not in L (part 1).  A back claim's factor is not in L, no member of
L lists it, and of the moves of frame h left only the claim's own does.

3.  Either way no claim takes x: if x passes (a), by part 1.  Then the claims are front and back moves of the
instance with members U\* ∩ D and pool F_D ∖ {x}, which keep it playable or unplayable (P4).  ∎

*Why test (b) is needed.*  Without it a back claim could hide a cycle.  At n = 98, the smallest board where this
happens, frame 0 has the back claim (77, 11), with 77 = 7·11; and 22 = 2·11, a factor of frame 1, passes (a).  The
moves frame 0 keeps after its claims, 62, 93, 57, 95, 65 and 91, form a path on the factors 2, 31, 3, 19, 5, 13 and 7.
With 11 gone, 22 would take the factor 2 and leave root frame 0 winnable, and root frame 1 does not need 22.  On the
board, though, 22 and 77 close the path into a [core](playability.md#cores-and-the-residual) of eight numbers on eight
factors, each listed twice, so U\* ∪ {22} is not playable.  Only test (b) keeps 22 out.

**Lemma F11 (the reduction-step lemma).**  The drops that reduce a root frame are front claims, so F10 holds for frame
h's claims followed by its drops.  So every candidate of a frame numbered h has the home h, and for every set L of
candidates, root frame h with L applied is winnable iff frame h with L applied is playable.

*Proof.*  A drop pairs a source m of frame h with its one live factor, the only maximal factor of m not taken before:
a front claim.  By F10, root frame h with L applied is frame h with L applied less front and back moves, none taking a
candidate.  ∎

**Theorem F12 (the exactness theorem).**  For every L ⊆ {2, …, ⌊n/2⌋}, U\* ∪ L is playable iff L is a promotion set
of Π_n, and then Σ(U\* ∪ L) = ΣU\* + ΣL.

*Proof.*  *Only if.*  Let U\* ∪ L be playable, with an acyclic scheme.

1. Every member of L is a candidate (F9), with a home (F11).
2. The members of frame h with L applied pay with factors of frame h outside L (F8), so by restriction frame h with L
   applied is playable, and root frame h with L applied is winnable (F11).

*If.*  Let L be a promotion set.

3. Every frame h with L applied is playable (F11).  Join acyclic schemes of these, and of U\* ∩ D against F_D for each
   closed frame D.  The joint scheme pays every source in its frame, and every member of L, a candidate of a frame
   numbered h, in frame h − 1, with distinct factors outside U\* ∪ L.
4. On each edge a → b of a cycle, pay(a) is a maximal factor of b (P3).  If b is a source, pay(a) lies in b's frame,
   so a pays where b does.  If b ∈ L is a factor of a frame numbered h, b pays in frame h − 1, and a in frame h − 1
   (F4) or in a closed frame.
5. A cycle through a source of a closed frame D lies in D's scheme: going backward, each predecessor pays in D, and
   only D's sources do.
6. Any other cycle pays only in frames h, and by step 4 each member pays where its predecessor does.  So the cycle lies
   in one frame h's scheme.  Steps 5 and 6 leave no cycle, and U\* ∪ L is playable (P2), with the weight ΣU\* + ΣL.  ∎

## Statement U

**Statement U, at n.**  Some optimal set of n contains U\*_{≥3}.

U is a statement about the game, open in general, and it is all that stands between the model and opt(n):

**Corollary F13 (the corollary of U).**  ΣU\* + P\*(Π_n) ≤ opt(n), with equality iff U holds at n.

*Proof.*  1.  Every playable set S ⊇ U\* is U\* ∪ L with L ⊆ {2, …, ⌊n/2⌋}: descending greedy rejected each other source
s because the kept sources above s with s are unplayable, and so is any set containing them.

2. By F12, ΣU\* + P\* is the largest weight of a playable set containing U\*, at most opt(n).
3. U\*_1 = {q(n)}, since descending greedy keeps the largest of the primes above n/2, which all need the factor 1.  So
   by F3 every optimal set contains U\*_1 and U\*_2.
4. If U holds, an optimal set contains all of U\*, so opt(n) ≤ ΣU\* + P\*.  If not, no optimal set contains U\*, so
   every playable set that contains U\* weighs less than opt(n).  ∎

The solver's results that rest on a search of Π_n assume U at n ([which they
are](../../results/README.md#what-supports-the-scores)).  The standing of U is in [the table of
statements](statements.md#the-statements), and its evidence in [evidence.md](evidence.md).  In brief, U holds wherever
the prover has proven opt(n) on the whole board.  Descending greedy rejects few sources of three or more prime factors:
none of four or more on the boards the solver supports, and none of layer 3 before n = 741.  [The barred bound
(W2)](parity-bound.md#the-barred-bound) certifies most members of U\*_{≥3} game by game, never all ([the
shares](../../solver/noframe/docs/proofs.md#the-upper-part-game-by-game)), and [the certificate theorem
(E4)](extended-problem.md#the-certificate-theorem) settles the rest by a search.

*Where a proof stops (a sketch).*  Let an optimal S miss a member of U\*_{≥3}, and d be the largest source where S and
U\* disagree, in a layer k ≥ 3 (F3).  If d were in S, S would hold d with the members of U\* above d in its frame, which
descending greedy found unplayable together.  So d ∈ U\* ∖ S, S_k + d has a [core](playability.md#the-core-theorem) in
its layer instance (F2), and every core holds d.  If one member v < d lies in every minimal core, S − v + d is playable
and heavier, and so is S − y + d if giving up one number y of layer k − 1 breaks every core.  A counterexample thus
needs d in two minimal cores with no common [victim](sources.md#the-victim-law) below d, the phenomenon of [the frontier
theorem (S3)](sources.md#the-frontier-theorem) inside a layer.  Where the cores are graph-like the argument closes, as
in layer 2; the gap is the [odd cores](playability.md#odd-cores).

## Searching the frame model

A search of Π_n branches on which candidates to promote; [search-rules.md](search-rules.md) proves the rules by which it
cuts, and [frame-bounds.md](frame-bounds.md) the bounds it cuts with.  A *node* has *promotions* P, a promotion set, and
*candidates* C, a set of candidates disjoint from P that it may still promote.  Its *completions* are the promotion sets
P ∪ X with X ⊆ C; the root node, with no promotions and every candidate, has a heaviest completion worth P\*.  The
*state* state(P) is the root frames with P applied, then reduced: each move with one live factor is dropped with that
factor until none is left ([reduction](#the-frame-model)).  The frames with P applied are winnable, so the reduced
frames are the same in any order of the drops (F5).  [The state lemma (R1)](search-rules.md#the-state-lemma) shows that
state(P) decides which completions are promotion sets.

The *freedoms* of a pair of Taxman Mini are its factors less its moves.  Claims and drops keep them, and a promotion
into or out of a frame spends one.  So frame h of state(P) has the freedoms of the frames numbered h, less the members
of P promoted into or out of it.  For freedoms r_0, …, r_H of frames 0 to H, the *promotion limit* is the largest
c_1 + ⋯ + c_H over integers c_h ≥ 0 with c_h + c_{h+1} ≤ r_h for every h, where c_0 = c_{H+1} = 0: c_h counts the
promotions out of frame h.  A node's promotion limit is that of state(P).

**Lemma F14 (the promotion-limit lemma).**  (1) Taking each c_h as large as fits from the front, c_h = min(r_h,
r_{h−1} − c_{h−1}), reaches the promotion limit.  (2) Every completion P ∪ X of a node has |X| at most the node's
promotion limit, and a candidate of home h is in some X only if frames h and h − 1 of state(P) have freedoms.

*Proof.*  (2) Let x_h be the number of members of X whose home is h; x_0 = 0, since no candidate has the home 0.
Root frame h with P ∪ X applied has the freedoms r_h − x_h − x_{h+1}, and it is winnable, so it has at least as many
factors as moves: x_h + x_{h+1} ≤ r_h.

(1) The sequence (c_h) meets the conditions by construction.  Let (d_h) be any sequence that meets them, and make it
equal (c_h) from the front without lowering its sum.  When d_j = c_j for j < h, the conditions at h − 1 and h give
d_h ≤ c_h.  Raise d_h to c_h and lower d_{h+1} as much, but not below 0: the condition at h − 1 holds as
c_{h−1} + c_h ≤ r_{h−1}, the one at h since d_h + d_{h+1} does not grow or else d_{h+1} = 0 and c_h ≤ r_h, and the one
at h + 1 loosens.  ∎

In game 98 the root's freedoms are 1, 6, 3 and 2, so the promotion limit is 1 + 3 + 0 = 4.  The solver keeps state(P)
as a node's frames and cuts by the promotion limit ([its search](../../solver/docs/design/search.md)).

## Boards with holes

The model applies unchanged to a board with numbers taken off, and a bound on opt(n) computed on such boards rests on
it.  Let H ⊆ {1, …, n} be the *holes* and B = {1, …, n} ∖ H.  On B a move needs one of its maximal factors in the pot,
a hole is never in the pot, and a move takes all its divisors in the pot.  On the full board this *maximal-factor rule*
changes nothing, by [the lifting lemma (P1)](playability.md#the-lifting-lemma).  The frames of B join each number of B
above n/2 to its maximal factors in B; a number of B up to n/2 that joins no source is a frame alone, one layer above
that number.  U\*(B) is what descending greedy keeps of the sources of B, and Π(B) is built from these frames as Π_n is.

**Theorem F15 (the holes theorem).**  For every L ⊆ B ∩ {2, …, ⌊n/2⌋}, U\*(B) ∪ L is playable on B iff L is a promotion
set of Π(B), and then it weighs ΣU\*(B) + ΣL.  So ΣU\*(B) + P\*(Π(B)) is the best score on B exactly when some best
game of B plays all of U\*(B).

*Proof.*  On B a legal game pays each move with a maximal factor in the pot by the rule itself, and a topological order
of an acyclic scheme is a legal game, so P2 holds with the pool leaving out the holes; P3 and P4 are about instances.
F1 holds by construction where it used Bertrand's postulate, and F2, F4 and F5 to F12 follow as before, with U\*(B)
for U\*.  The last sentence is F13's argument without its step 3.  ∎

The bottom-layers theorem (F3) is not claimed on B, since [the prime lemma (N1)](neighbors.md#the-prime-lemma) is about
the full board; so B needs all of U\*(B) played, not only its part in layers 3 and up.  On B a layer between k₀ and the
highest with freedoms can hold only closed frames; its frame h is then empty, and nothing is promoted through it.
