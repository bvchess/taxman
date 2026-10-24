# The frame model

Part of [the theory](../THEORY.md).

The frame solver does not search the games of n.  It splits the board into *frames*, fixes the sources it plays to
U\*, and searches only over which factors to *promote*, that is, to play as moves.  This file defines that model, the
*frame model*, and the words a search of it uses.  [exactness.md](exactness.md) proves that the model is exact: its
promotion sets are exactly the sets L of numbers up to n/2 with U\* ∪ L playable.  There one open statement is left
between the model and the game, statement U: the solver's answer ΣU\* + P\* is opt(n) exactly when some optimal set
contains every member of U\* with three or more prime factors.  [The extended problem](extended-problem.md) builds on
the model, [frame-bounds.md](frame-bounds.md) bounds its promotion sets, and [search-rules.md](search-rules.md) says
what a search of it proves.  Game 21 is the example throughout.

*Words.*  Sources, factors, maximal factors, layers and playable sets are the [glossary's](../GLOSSARY.md), and Ω(x)
is the layer of x.  Descending greedy, and U\*, the sources it keeps, are defined in
[sources.md](sources.md#descending-greedy).  U\*_k and S_k are the parts of U\* and of a set S in layer k
([notation](statements.md#notation)).

*Instances.*  The words for instances are [playability.md](playability.md#payment-schemes)'s: members, pool, lists
(a member's maximal factors in the pool), payment schemes, precedence, playable instances, [peel](playability.md#peel)'s
front and back moves, and the remainder they leave.  A set T is *playable against* a set F when the instance with
members T and pool F is playable.  Four facts are used throughout:

* [the playability theorem (P2)](playability.md#the-playability-theorem): a set S ⊆ {2, …, n} is playable iff its own
  instance, with pool the maximal factors of its members outside S, is playable;
* [the flat-cycle lemma (P3)](playability.md#the-flat-cycle-lemma): every cycle of a scheme's precedence is flat: its
  members lie in one layer, and on each edge a → b, pay(a) is a maximal factor of b;
* [the peel theorem (P4)](playability.md#the-peel-theorem): a front move (a member with one live factor, paired with
  it) or a back move (a live factor in one list only, paired with that member) keeps an instance playable and keeps it
  unplayable, and a run of peel that pairs every member yields an acyclic scheme;
* *restriction*: if an acyclic scheme pays the members of a subset T with factors of a set F, then T is playable
  against F, since the scheme restricted to T is a scheme of that instance and its precedence is part of the old one;
  and a scheme against a pool is one against any larger pool.

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

*Proof.*  In layer 2 descending greedy is the scan of a graphic matroid, the [greedy algorithm of a
matroid](primer.md#the-greedy-algorithm-of-a-matroid), and an exchange shows that S_2 is what the scan keeps of
S_2 ∪ U\*_2.  Parts (b) and (d) cited below are those of the primer's paragraph "What the scan finds".

1. S_1 = {q(n)} by [the prime lemma (N1)](neighbors.md#the-prime-lemma), so the pool of S's layer instance at 2 is
   every prime up to n/2.
2. A set of numbers of layer 2 is playable against those primes iff it is a forest in G_n, by [the two-factor
   proposition (C3)](franklin-moniot.md#the-two-factor-proposition): these sets are the independent sets of the
   graphic matroid of G_n ([matroids](primer.md#matroids)).
3. Adding a source s of layer 2 to the kept sources changes only their layer instance at 2 (F2), so descending greedy
   keeps s iff s closes no cycle with the kept sources of layer 2: on the sources of layer 2 it is the scan.  By part
   (b), the elements the scan keeps before an element x span all the elements above x, so it keeps x iff these do not
   span x.
4. Let G be what the scan keeps of W = S_2 ∪ U\*_2.  The members of W above a source x of W are sources of layer 2.
   If x ∈ U\*_2 they do not span x, since all the sources of layer 2 above x do not; if x ∉ U\*_2, the members of U\*_2
   above x span it.  So G = U\*_2 ∪ A, with A made of numbers of S_2 up to n/2.
5. The weights, the numbers themselves, are distinct and positive, so by part (d) G is the only heaviest independent
   subset of W.  S_2 is independent (F2 and step 2), so ΣG ≥ ΣS_2, with equality only if G = S_2.
6. S′ = (S ∖ S_2) ∪ G is playable by F2: at layer 2 it has the forest G against the same pool; at layer 3 the pool
   only grows, since S plays the numbers A too; the other layers do not change.  So ΣS′ ≤ ΣS, which forces
   S_2 = G = U\*_2 ∪ A.
7. The frames of layer 2 hold the primes up to n/2 as factors (F1).  When they are closed, U\*_2 has one edge for each
   of these primes, one fewer than G_n has vertices: it is a spanning tree, so A is empty.  ∎

In game 21, G_n has the vertices 2, 3, 5, 7 and g, and U\*_2 = {14, 15, 21} is the forest of the edges 2–7, 3–5 and
3–7.  The frame of layer 2 has a freedom, so an optimal set may add an edge, and the optimal set of 21 adds 9 = 3², the
edge 3–g; 6 = 2·3 and 10 = 2·5 would close a cycle.  The frames of layer 2 are closed on all but a few small boards
([evidence.md](evidence.md)).  [The barred bound (W2)](parity-bound.md#the-barred-bound) fixes layers 1 and 2 by this
theorem.

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

The proof of exactness uses nothing else about the numbering: a promoted number pays one layer down (F1), and F4 puts
the frames there that can pay for it in frame h − 1.

**Taking a number on.**  A frame D, or a frame h, *can take x on* when its moves together with x are playable against
its factors: x could become one more move of it without leaving the others short.  A closed frame can take nothing on,
since its moves already need all its factors.  In game 21, frame 0 can take 9 on: 9 takes 3, 15 takes 5, 21 takes 7 and
14 takes 2.  It cannot take 6 on: 15 still takes 5, and then 14, 21 and 6 list each of the factors 2, 3 and 7 twice, so
peel stalls.

Frame h can take x on iff one of its frames can.  This needs an argument, because x can have maximal factors in several
frames of frame h.

* *If frame h can take x on, so can one of its frames.*  Take an acyclic scheme of the moves of frame h with x.  The
  payer of x lies in one frame D of frame h.  The moves of D pay in D, since a move's maximal factors lie in its own
  frame, and x pays in D too: by restriction, D can take x on.
* *If a frame D of frame h can take x on, so can frame h.*  Join an acyclic scheme of the moves of D with x to acyclic
  schemes of the moves of the other frames of frame h, against their factors.  The frames share no factor, so the joint
  scheme pays each member with a different factor.  Suppose its precedence had a cycle.  On each edge a → b of it,
  pay(a) is a maximal factor of b (P3).  If b is a move of another frame D′, its maximal factors lie in D′, so a pays
  in D′, and since only the moves of D′ pay there, a is a move of D′ too.  Going backward around the cycle from a move
  of D′, every member is then a move of D′, so the cycle lies in D′'s scheme, which has none.  A cycle with no move of
  another frame lies in the scheme of D's moves with x, which has none either.

**Candidates.**  A *candidate* is a factor x of a frame D numbered h ≥ 1 that passes two tests: (a) U\* ∩ D is
playable against F_D ∖ {x}; (b) frame h − 1 can take x on.  Test (a) asks that D can spare x, and test (b) that the
frame in front can pay for it.  In game 21 the candidates are 4 and 9, factors of frame 1.  All four factors of frame 1
pass (a), and 6 and 10 fail (b); 10 = 2·5 fails as 6 does, since 14, 15, 21 and 10 list each of 2, 3, 5 and 7 twice.

**Claims.**  A *claim* (s, f) pairs a move s of a frame with a factor f of it, and takes both out.  A *claim sequence*
of a frame X, a frame D or a frame h, is a sequence of claims, each one of two kinds given those before it:

* a *front claim*: f is the only maximal factor of s that no earlier claim took;
* a *back claim*: no earlier claim took f; s is the only move of X left with f as a maximal factor; no frame of the
  layer below can take f on; and X cannot take on any factor of a frame with freedoms that has f as a maximal factor.

A front claim settles a move with one choice left, and a back claim a factor that only one move wants.  The last two
conditions of a back claim say that f has no other use: it cannot be promoted, and it cannot pay for a number promoted
into X.  In game 21 frame 0 has the back claim (15, 5): only 15 has the maximal factor 5, the frame of layer 1 is
closed, and frame 0 cannot take on 10, the one factor of frame 1 with the maximal factor 5.  Frame 1 has the back claim
(20, 10).  Each condition earns its place.  Once 15 is claimed only 21 has the maximal factor 3, but frame 0 can take
on 9, which has it; and only 18 has the maximal factor 9, but frame 0 can take 9 on.  The claim (21, 3) or (18, 9)
would lose the promotion set {9} (below).

**Reduction.**  To *reduce* a pair of Taxman Mini is to drop a move with one live factor, with that factor, until no
such move is left.

**Setup** computes the model:

1. it forms the frames, and runs descending greedy over each frame's sources against its factors;
2. it makes claims, round after round, running descending greedy again over what each frame has left, until no claim
   can be made;
3. it numbers the frames, finds the candidates, and merges the frames of each number into frame h;
4. it makes claims in each frame h, a merged frame starting with none and a frame alone in its number keeping its own;
5. it reduces each frame h less its claims.

The claims of step 2 in a frame are a claim sequence of it, and those that leave frame h are one of frame h; setup's own
tests ask at least what the rules ask.  [The kept-sources lemma (X1)](exactness.md#the-exactness-theorem) shows that
descending greedy keeps U\* at every pass, and [the claims lemma (X5)](exactness.md#the-exactness-theorem) that test
(a), which setup applies after the claims of step 2, picks the same candidates.

**The root problem.**  The *root frame h* is frame h less its claims, reduced.  The *root problem* Π_n is the root
frames and the candidates; the *root* is the root frames before any promotion.  A number's *home* is the root frame
whose factors hold it.  ΣU\* is the score with no promotions (X1).  In game 21, root frame 0 has the moves 14 and 21 and
the factors 2, 3 and 7, and root frame 1 the moves 12 and 18 and the factors 4, 6 and 9; neither reduces further.  The
candidates 4 and 9 have the home 1.

**Promotion sets.**  *Applying* a set L to frame h adds to its moves the members of L in F_{h+1} and removes from its
factors those in F_h; applying L to root frame h adds the members of L whose home is h + 1 and removes those whose home
is h.  A *promotion set* of Π_n is a set L of candidates, each with a home, such that every root frame is winnable with
L applied.  *Promoting* x thus moves it one frame forward, from the factors of its home h to the moves of frame h − 1.
P\* = P\*(Π_n) is the largest ΣL over the promotion sets.  A promotion set has at most as many members as the freedoms
allow, the promotion limit ([the promotion-limit lemma (F6)](#the-promotion-limit-lemma)).

In game 21, {9} is a promotion set.  Root frame 0 with 9 applied has the moves 14, 21 and 9 against 2, 3 and 7, won by 9
with 3, 21 with 7 and 14 with 2; root frame 1 has the moves 12 and 18 against 4 and 6, won by 18 with 6 and 12 with 4.
So is {4}, but not {4, 9}, which would leave root frame 0 four moves for three factors.  So P\* = 9, and
ΣU\* + P\* = 135 + 9 = 144 = opt(21), the score of the game 19; 9, 15, 21, 14; 18, 12, 20; 16, which plays the frames
one after another from the front.

### The reduction lemma

Reducing is safe: a dropped pair could never have gone otherwise, now or after later promotions, as long as its factor
stays.

**Lemma F5 (the reduction lemma).**  Let W be a pair of Taxman Mini, and m a move of W whose only live factor is f.

1. Dropping m and f never changes whether the pair is winnable, now or after later edits that keep f: for every set Y
   of numbers of the moves' layer and every set Z of factors with f ∉ Z, W with Y added to its moves and Z removed from
   its factors is winnable iff W less m and f, so edited, is.
2. If W is winnable, reducing it ends at the same pair whatever the order of the drops.

*Proof of part 1.*  After the edits m still has the single live factor f: added moves change no move's live factors,
and Z leaves f.  So (m, f) is a front move, which keeps the pair winnable and keeps it unwinnable (P4).

*Proof of part 2.*  The idea is the diamond argument.  Two different drops available at once commute: each leaves the
other available, and doing both in either order reaches the same pair.  An induction spreads this local agreement to
whole reductions.

1. *Every pair reached is winnable,* by part 1 with nothing added or removed.  So no two of its moves have the same
   single live factor, since a winnable pair pays its moves with different factors.
2. *The diamond.*  Let (m₁, f₁) and (m₂, f₂) be different drops available in a winnable pair.  Then m₁ ≠ m₂, since a
   move with one live factor gives one drop, and f₁ ≠ f₂ by step 1.  Dropping (m₁, f₁) removes only m₁ and f₁, so m₂
   still has the single live factor f₂, and (m₂, f₂) is still available.  Both drops, in either order, leave W less the
   two pairs.
3. *The induction.*  We show, by induction on the number of moves, that all reductions of a winnable pair W end at the
   same pair.  If no move of W has one live factor, the only reduction is the empty one.  Otherwise let two reductions
   of W begin with the drops d₁ and d₂.  If d₁ = d₂, the rest of each is a reduction of W less d₁, which has fewer
   moves, so they end together.  If d₁ ≠ d₂, take any reduction σ of W less both pairs.  By step 2, d₂ followed by σ is
   a reduction of W less d₁, as is the rest of the first reduction, so by the induction hypothesis they end together.
   Likewise d₁ followed by σ and the rest of the second reduction end together.  And d₁, d₂, σ and d₂, d₁, σ both end
   where σ does.  So the two reductions end at the same pair.  ∎

Winnability is needed in part 2: against the factor 3, the moves 6 and 9 reduce to 9 alone or to 6 alone.

## Searching the frame model

A search of Π_n branches on which candidates to promote; [search-rules.md](search-rules.md) proves the rules by which it
cuts, and [frame-bounds.md](frame-bounds.md) the bounds it cuts with.  A *node* has *promotions* P, a promotion set, and
*candidates* C, a set of candidates disjoint from P that it may still promote.  Its *completions* are the promotion sets
P ∪ X with X ⊆ C; the root node, with no promotions and every candidate, has a heaviest completion worth P\*.  The
*state* state(P) is the root frames with P applied, then reduced: each move with one live factor is dropped with that
factor until none is left ([reduction](#the-frame-model)).  The frames with P applied are winnable, so the reduced
frames are the same in any order of the drops (F5).  [The state lemma (R1)](search-rules.md#the-state-lemma) shows that
state(P) decides which completions are promotion sets.  In game 21, state({9}) has two empty frames: in root frame 0
with 9 applied, 9 drops with 3, then 21 with 7 and 14 with 2, and in root frame 1, 18 drops with 6 and then 12 with 4.

The *freedoms* of a pair of Taxman Mini are its factors less its moves.  Claims and drops keep them, and a promotion
into or out of a frame spends one.  So frame h of state(P) has the freedoms of the frames numbered h, less the members
of P promoted into or out of it.  For freedoms r_0, …, r_H of frames 0 to H, the *promotion limit* is the largest
c_1 + ⋯ + c_H over integers c_h ≥ 0 with c_h + c_{h+1} ≤ r_h for every h, where c_0 = c_{H+1} = 0: c_h counts the
promotions out of frame h, which spend a freedom of frame h and one of frame h − 1.  A node's promotion limit is that of
state(P).

### The promotion-limit lemma

**Lemma F6 (the promotion-limit lemma).**  (1) Taking each c_h as large as fits from the front, c_h = min(r_h,
r_{h−1} − c_{h−1}), reaches the promotion limit.  (2) Every completion P ∪ X of a node has |X| at most the node's
promotion limit, and a candidate of home h is in some X only if frames h and h − 1 of state(P) have freedoms.

*Proof of part 2.*  Let x_h be the number of members of X whose home is h; x_0 = 0, since no candidate has the home 0.
Root frame h with P ∪ X applied has the freedoms r_h − x_h − x_{h+1}, and it is winnable, so it has at least as many
factors as moves: x_h + x_{h+1} ≤ r_h.  So (x_h) is one of the sequences the promotion limit ranges over, and
|X| = Σx_h is at most the limit.  If x_h ≥ 1, the conditions at h and h − 1 give r_h ≥ 1 and r_{h−1} ≥ 1.

*Proof of part 1.*  The idea is an exchange: wherever a sequence falls short of the front-to-back choice, moving
promotions forward from the next frame keeps it within the freedoms and never lowers its sum.

1. *The choice (c_h) meets the conditions.*  By construction c_h ≤ r_h and c_{h−1} + c_h ≤ r_{h−1}, and c_h ≥ 0,
   since c_{h−1} ≤ r_{h−1}.
2. *A sequence that agrees with it before h falls short at h.*  Let (d_h) meet the conditions, with d_j = c_j for every
   j < h.  The condition at h gives d_h ≤ r_h, and the one at h − 1 gives d_h ≤ r_{h−1} − c_{h−1}, so d_h ≤ c_h.
3. *The exchange.*  Raise d_h to c_h, and lower d_{h+1} by as much, but not below 0.  The condition at h − 1 holds,
   since c_{h−1} + c_h ≤ r_{h−1}.  The one at h holds: d_h + d_{h+1} has not grown, unless d_{h+1} reached 0, and then
   it is c_h ≤ r_h.  The one at h + 1 only loosens.  The sum has not fallen.
4. Doing this for h = 1, 2, …, H turns (d_h) into (c_h) without lowering its sum, so the sum of (c_h) is the largest.  ∎

In game 21 the freedoms of the root are 1 and 1, so the promotion limit is 1, which {4, 9} exceeds.  In game 98 they are
1, 6, 3 and 2, and the front-to-back choice is c₁ = min(6, 1) = 1, c₂ = min(3, 6 − 1) = 3 and c₃ = min(2, 3 − 3) = 0:
the promotion limit is 4.  The sequence 0, 3, 0 meets the conditions too; the exchange raises it to 1, 2, 0 and then to
1, 3, 0.  The solver keeps state(P) as a node's frames and cuts by the promotion limit
([its search](../../solver/docs/design/search.md)).
