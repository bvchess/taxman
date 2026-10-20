# The frame model: what is proven and what remains

Part of [the theory](../THEORY.md).

The frame model is exact given one open statement about a game, U.  The frame solver (`solver/frame/`) does not search
the games of n.  It searches the *promotion sets* of the frame model, and its answer is in_the_bag plus the heaviest of
them (solver/frame/README.md, section 1, and docs/GLOSSARY.md define the words used here: sources, frames, `unzip` and
its mandatory moves, impossible sources, freedoms, closed frames, the numbering, candidates, promotion sets,
`max_total`).  The model rests on five rules that data supported and nothing proved: setup's greedy removal of
impossible sources, the mandatory moves of `unzip`, freedoms = factors − sources, closed frames taking no part, and a
promoted number paying only in the frame one forward.  This section proves all five, and leaves one statement:

* **U (the upper part)**, a statement about a game, and open: *some optimal set of n plays every source with three or
  more prime factors that setup keeps.*

The frame solver's answer for a game whose proof rests on its search (`exhausted`, `trivial`, `same_search`) is opt(n)
exactly when U holds at n (the corollary at the end).  The games proven at their ceiling or by the prime formula never
needed the model.  (Setup numbers the frames by layer, which makes the numbering *clean*, the one fact about a board
that the proof needs; the Java's numbering, breadth first from the front, is the same on every board the solver
supports.)

*Words.*  Layers, sources, U\* and the parts S_k of a set are those of [Definitions](statements.md#definitions).  The
sources that setup keeps, by descending greedy on each frame, are U\* (M1, below).

## Frames are layers

**Lemma Ω (layers).**  The frames as setup forms them, before any is merged, are the components of the graph that joins
each source to its maximal factors.  In each frame every source has one Ω, k, and every factor has Ω = k − 1: the frame
*lies in layer k*.  Every number up to n/2 is a factor of some frame.  A frame in front of a frame of layer k (one that
holds a maximal factor of one of its numbers) lies in layer k − 1.

*Proof.*  An edge joins a source s to s/p, one layer lower.  A path between two sources alternates source, factor,
source, so its layers go k, k − 1, k, …: the sources of a component share a layer and its factors lie one below.  A
number x ≤ n/2 has a prime p with n/2 < xp ≤ n (Bertrand), and xp is a source with the maximal factor x.  The maximal
factors of a source of a frame are factors of that frame itself.  A maximal factor y of a factor of a frame of layer k
lies in layer k − 2 and is at most n/2, so it is a factor of some frame, whose factors lie one layer below its sources:
that frame lies in layer k − 1.  ∎

So no number has a proper divisor in its own frame, and a promoted number never taxes the frame it came from.  The
frames of a board are the components of its layers: at n = 4000, layers 1 to 11, with one or two frames each but five in
layer 9.

## Playability splits by layer

For S ⊆ {2, …, n}, the *layer instance* of S at k is the reduction instance whose members are S_k, whose pool P_k is the
numbers of layer k − 1 up to n/2 that are not in S (P_1 = {1}), and in which each member lists its maximal factors in
P_k.

**Theorem L (layer decomposition).**  A set S is playable iff every layer instance of S is playable.

*Proof.*  By Theorem 1, S is playable iff its own reduction instance is: members S, pool the maximal factors of the
members that are not in S, each member listing its maximal factors in the pool.  A maximal factor of a member of layer k
lies in layer k − 1 and is at most n/2, so the member lists in S's instance exactly what it lists in the layer instance
at k, and a factor of layer k − 1 is listed by members of layer k only.  So S's instance is the disjoint union of its
layer instances: no factor is listed in two of them.  On a disjoint union a move of peel is a move within one part and
changes no count of another, so peel pairs every member of the union iff it pairs every member of each part; and peel
decides playability on both sides (Theorems 2 and 3).  ∎

A game is thus a chain of layer games coupled only through the numbers up to n/2 it plays: a played x of layer j is a
member of layer j and missing from the pool of layer j + 1.  The frame model is this chain with the sources fixed to
U\*, its frames the components of the layers.  The parity matroids of Theorem 10 group the layers by parity.

## The bottom layers

**Theorem B (the bottom layers).**  Let q be the largest prime ≤ n and S an optimal set of n ≥ 2.  Then S_1 = {q}; the
numbers of S_2 above n/2 are exactly U\*_2; and the numbers of S_2 up to n/2 keep U\*_2 a forest, in the graph on the
primes up to n/2 and a *ground* vertex in which pq is the edge p–q and p² the edge p–ground.  So wherever the frames of
layer 2 are closed (U\*_2 spans every prime up to n/2), S plays no number of layer 2 up to n/2.

*Proof.*  S_1 = {q} is the lemma of the prime of every optimal set ("Uniqueness of the optimal set").  So S holds no
prime up to n/2, and the pool of layer 2 is every prime up to n/2.  The layer instance of layer 2 is separated (its
members are not primes), and a member lists its one or two primes, so by Theorem 5 a set of it is playable iff it has no
self-covering subset, that is no set of edges in which every prime it touches is touched twice.  A forest has none (in a
forest, a tree with an edge has two leaves, and one of them is a prime touched once), and a cycle is one: *a set of
layer 2 is playable in S's game iff it is a forest* (the two-factor proposition of "What the parity bound keeps" is the
same fact).  So the playable sets of layer 2 are the independent sets of a graphic matroid, and setup's greedy on the
sources of layer 2 is Kruskal's algorithm: U\*_2 is the heaviest forest of sources.

Let W = S_2 ∪ U\*_2 and G the forest descending greedy keeps from W.  The sources of W come before its other members,
and among them greedy keeps exactly U\*_2: setup's greedy kept each member of U\*_2 because the sources above it do not
span it, so neither do those of W above it; and it rejected every other source because the members of U\*_2 above it
span it, and these are in W and kept.  So G = U\*_2 ∪ A, with A among S_2's numbers up to n/2.  In a matroid greedy
finds the heaviest independent set, so ΣG ≥ ΣS_2.  Replace S_2 by G in S: layer 2 stays playable; the pool of layer 3
only grows, since A ⊆ S_2, and a larger pool keeps every payment scheme; the other layers do not change.  By Theorem L
the new set is playable, and it weighs at least as much as S.  Since S is optimal, ΣG = ΣS_2: S_2 is a heaviest
independent subset of W too, and with distinct positive weights there is only one, so S_2 = G.  Where the frames of
layer 2 are closed, U\*_2 has as many edges as there are primes up to n/2, so it spans every one of them, and any
further edge closes a cycle: A is empty.  ∎

The frames of layer 2 have freedoms on 80 boards, all at most 186, and are closed on every other board to 8191
(`solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/`).  Theorem B settles the members of U\* that the
barred parity bounds leave open in layers 1 and 2: the largest prime (open at 1000 and 1300) and the rest of those (with
the prime, 13 of the 17 open members at 600, 24 of 36 at 1000, 15 of 18 at 1300 and 92 of 194 at 2175).

## Numbering by layer

Setup numbers the frames that have freedoms by layer: with ℓ₀ the lowest layer of a frame with freedoms, a frame with
freedoms of layer ℓ gets the number ℓ − ℓ₀, and closed frames get none (`frames::build_frames`).  A layer between ℓ₀ and
the highest layer with freedoms that has only closed frames leaves its number without frames: the search then has an
empty frame there, with no moves and no factors, through which nothing can be promoted.

**N (clean numbering).**  Every frame with freedoms directly in front of a frame numbered h is numbered h − 1.

Statement N holds on every board, by construction: a frame directly in front of a frame of layer ℓ lies in layer ℓ − 1
(Lemma Ω).  It is the one fact about the numbering that the proof of Theorem M uses (in M3 and M5).

The Java numbered the frames breadth first from the front instead: number 0 for those with no frame with freedoms in
front of them, h + 1 for those directly behind a frame numbered h and not yet numbered.  It is the numbering by layer
when it is clean and its frames numbered 0 lie in one layer, and the census of every board to 8191, the largest the
solver supports (`solver/frame/examples/structure.rs`; the data in
`solver/notebook/the-claims-of-unzip-in-the-extended-problem-2026-10/`), found the two the same on every one, with ℓ₀ =
3 on 8,100 boards and ℓ₀ = 2 on the 80 where the frames of layer 2 have freedoms (the tests of
`solver/frame/src/frames/` compare the two numberings on every board to 400 and a sample to 8191).  So on those boards
the frames, the candidates and the searches are the same under either numbering, and so is every result.  The two can
differ on boards with holes (below): on 56 without 39 the breadth-first numbering gives a frame and a frame with
freedoms directly in front of it the same number, and on 32 without 4 a layer between two with freedoms has none (taking
4 off splits the frame of layer 3 into two closed frames).

## The frame model is exact, given its upper part

What setup does, in the words of the code (`solver/frame/src/frames/`):

1. It forms the frames and runs `removeImpossible` on each: descending greedy over the frame's sources against its
   factors, peel deciding (Theorems 2 and 3).  Then, round after round until nothing changes, `unzip` and
   `removeImpossible` again on the frames' *remaining* sources and factors (those not in a mandatory pair).
2. `unzip` settles mandatory pairs (s, f), a source and a factor, each a *claim* of f by s.  A *back* claim: f is wanted
   by a single remaining source s, f cannot go forward (no frame holding a maximal factor of f could take f on as a
   source), and f has no outside move (no maximal composite c of f, in another frame with freedoms, could be taken on as
   a source by f's frame).  A *front* claim: s has a single unclaimed factor f, and f has no outside move.
3. It numbers the frames with freedoms by layer (above) and merges the frames of one number.  The candidates of number h
   are collected from its frames one by one, before they are merged and after the frames of number h − 1 have been: a
   remaining factor x of frame A is a candidate if (a) A's remaining sources stay playable against its remaining factors
   less x, and (b) some frame holding a maximal factor of x could take x on as a source.
4. After the last merge it simplifies every frame again (a merged frame starts with nothing claimed).
5. The search's frame h is the merged frame h's remaining sources and factors, *reduced*: a move left with one factor is
   paid by it, and the pair dropped.  A number's *home* is the frame whose factors hold it then.  A promotion set L is a
   set of candidates, each a factor of a home h ≥ 1, such that every frame is winnable with L applied (its moves plus
   the members of L of home h + 1, its factors less the members of home h), with at most `max_total` members.

For a set L of numbers up to n/2 and a number h, the *instance* I_h(L) has as members U\* ∩ frame h and the members of L
that are factors of frames numbered h + 1; its pool is the factors of frame h (as merged) that are not in L; each member
lists its maximal factors in the pool.  A *front move* removes from an instance a member that lists one live factor,
with that factor; a *back move* a factor listed by one live member, with that member.  Both keep a playable instance
playable (Theorem 3), and a sequence of them that pairs every member is an acyclic payment scheme (Theorem 2).  The
lemmas tie each step of setup to these instances; for the soundness half, recall from the proof of Theorem 2 that a
cycle of the precedence of any scheme that pays with maximal factors is *flat*: its members lie in one layer and on each
edge a → b, pay(a) is a maximal factor of b.

**M1 (the kept sources are U\*).**  The first `removeImpossible` is descending greedy on each frame, so it keeps U\* ∩
frame.  The later passes keep them all: the pairs of `unzip` are front and back moves of the instance (U\* ∩ frame, its
factors), by M5's argument with L empty, so the remaining sources stay playable against the remaining factors, and
greedy keeps every member of a playable set.  Merged frames share no factor, so their kept sources stay playable
together.  So the game with no promotions plays exactly U\*, and in_the_bag = ΣU\*.

**M2 (closed frames are inert).**  Let C be a closed frame, with as many factors as kept sources.  In a payment scheme
of a playable S ⊇ U\*, the sources of C pay with distinct factors of C (all their maximal factors are in C), so every
factor of C pays a source of C: none is in S, and none pays for anything else.

**M3 (one frame forward; this is where the numbering is needed).**  Let S = U\* ∪ L be playable with a payment scheme,
and x ∈ L a factor of the frame A.  By M2, A is not closed, so it has a number h.  The number x pays with one of its
maximal factors, which lies in a frame X in front of A (Lemma Ω: not in A), and X is not closed (M2).  By N, X has the
number h − 1: so h ≥ 1, and x pays in the merged frame h − 1.  In the scheme, every member of L pays in the frame the
model promotes it into, and every source in its own frame.

**M4 (the candidate filter is necessary).**  With S and x as in M3, x passes both tests of step 3.  (b) Restrict the
scheme to the sources of frame h − 1 and x: they are playable against its factors, so frame h − 1, merged by the time
x's candidacy is decided (a merged frame can take x on iff one of its parts can, as they share nothing), can take x on.
(a) Restrict the scheme to U\* ∩ A: these sources pay with factors of A other than x.  The claims made in A before the
merge are front and back moves of that instance without x: a front claim of x would leave its source nothing; a back
claim of x needs x unable to go forward, but the frame where x pays could take it on (restrict the scheme to that
frame's sources and x, as in (b)).  So the residual, A's remaining sources against its remaining factors less x, is
playable.

**M5 (the claims are moves of the instances).**  Let L ⊆ candidates, and either U\* ∪ L playable (then I_h(L) is
playable: restrict the scheme, by M3) or L a promotion set (then no claimed factor is in L: a claimed factor is in no
frame of the root, so it has no home).  The claims of a numbered frame h, in the order made, are front and back moves of
I_h(L), and no claimed factor is in L.

*Front claim (s, f).*  The factor f is s's only unclaimed factor, so after the earlier pairs s lists f alone in I_h(L),
unless f ∈ L.  In the first case f ∈ L would leave s nothing in a residual that the earlier moves kept playable.

*Back claim (s, f).*  Only s among the remaining sources wants f; what matters is that **no candidate lists f**.
Suppose a candidate c had f as a maximal factor.  The candidate c is a factor of a numbered frame Y, and f's frame lies
in front of Y, so by N, Y has the number h + 1.  When c passed test (b), the frames that could have taken it were the
frames in front of Y: a closed one cannot take a source on, and those with freedoms all have the number h and had been
merged into one frame, with the sources and factors that frame h had when the claim was made.  The claim says that frame
could not take c on (f had no outside move), so c failed (b): a contradiction.  So in I_h(L), f is listed by s alone;
and f ∉ L (in the first case f would pay in a frame in front, by M3, which could then take it on, so f could go
forward).  It is a back move.

Two quirks of the code, kept for parity with the Java, can only make `unzip` claim less, and the argument covers
whatever it claims: a frame not yet simplified counts as having freedoms, and a composite left in a frame that has been
merged away counts as an outside move.

**M6 (the reduction).**  With L as in M5, a move left with one live factor in a frame of the root is a front move of
I_h(L) after the claims, and its factor is not in L, for the reasons M5 gives for a front claim (a factor reduced away
is in no frame of the root either).  And reducing never changes whether a frame is winnable, before or after promotions
(the reduction lemma, `solver/frame/src/model/peel.rs`).

**Theorem M (exactness).**  Let Π_n be the frame solver's root problem (its frames, candidates and `max_total`).  For
every L ⊆ {2, …, n/2}: U\* ∪ L is playable iff L is a promotion set of Π_n, and then U\* ∪ L weighs in_the_bag + ΣL.

*Proof.*  (Only if.)  Let U\* ∪ L be playable.  By M2 and M3, every member of L is a factor of a numbered frame h ≥ 1
and pays in frame h − 1; by M4, L ⊆ candidates.  Each I_h(L) is playable (restrict the scheme), so by M5 and M6 its
claims and reductions are front and back moves, and no member of L is claimed or reduced away: each keeps its home.  By
Theorem 3 what is left, which is frame h of the search with L applied, is playable, that is, winnable.  So L can be
promoted from the root, and |L| ≤ `max_total`, a count that winnability implies (`solver/frame/src/model/frame.rs`).
(If.)  Let L be a promotion set.  In each numbered frame, the claims, the reductions and then a peel of what is left
with L applied pair every member of I_h(L); by M5 and M6 that is a sequence of front and back moves, an acyclic scheme
(Theorem 2).  Closed frames pay their sources inside.  So every member of U\* ∪ L is paid, with distinct maximal factors
outside the set.  A cycle of the union's precedence is flat, and a flat edge a → b stays in one frame: b moves in the
frame of pay(a), as one of its sources or, by N, as a member of L promoted into it.  The one exception is pay(a) in a
closed frame: then a is one of that frame's sources (a closed frame pays nothing else), and no edge enters a closed
frame from outside (an edge into a source of it comes from a member paying in it).  So a cycle lies in one frame's
scheme, which is acyclic: U\* ∪ L is playable (Theorem 1), and it weighs ΣU\* + ΣL = in_the_bag + ΣL (M1).  ∎

*Why the candidate filter matters.*  Without test (b) a back claim would hide a cycle.  At n = 98, the source 77 = 7·11
claims 11 (back); 22 = 2·11 is a factor of frame 1, and frame 0 holds the six sources 57, 62, 65, 91, 93 and 95, a path
on the factors 2, 31, 3, 19, 5, 13,
7.  With 11 claimed, the model would take 22 into frame 0 with the
factor 2, and the frame would stay winnable; but on the board 22 and 77 close the path into a cycle of eight numbers on
eight factors, each listed twice, so U\* ∪ {22} is not playable.  22 is no candidate, because frame 0 cannot take 22 on:
only the filter keeps it out.  The same happens on 27 boards to 2000, all in 98–120 and 182–186
(`solver/frame/tests/model_exact.rs`).

*Boards with holes.*  The sacrifice study (`solver/frame/src/sacrifice.rs`) forms frames on a board B with *holes*,
numbers taken off it, under the *maximal-factor rule*: a move needs one of its maximal factors in the pot, and a hole is
never in it.  Theorem 1 holds for that game as it stands, with the pool leaving the holes out (a legal game pays each
move with a maximal factor in the pot by the rule itself, and the construction is unchanged), and so do Theorems 2, 3
and L and Lemma Ω, with these readings: the sources are the numbers above n/2 that are not holes, a frame joins them to
their maximal factors that are not holes, and a number up to n/2 whose maximal composites above n/2 are all holes is a
frame of its own with no source, one layer above it.  Numbered by layer, B satisfies N, and the lemmas and Theorem M go
through word for word, with U\* the sources setup keeps on B and "playable" meaning playable on B.  Theorem B is not
claimed for B (its first step, the prime of every optimal set, is about the board {1, …, n}).  So the frame solver's
answer on B is the best game of B exactly when some best game of B plays every source setup keeps there: the statement
that a bound computed on punctured boards rests on.

## The upper part

**U, at n.**  Some optimal set of n contains U\*_{≥3}.

By Theorems B and M this is all that stands between the frame model and opt(n):

**Corollary (what the frame solver's answer is).**  in_the_bag + P\*(Π_n), the heaviest promotion set's weight added to
in_the_bag, is opt(n) if U holds at n, and less than opt(n) if not.

*Proof.*  By Theorem M, in_the_bag + P\*(Π_n) is the weight of the heaviest playable set containing U\* (every playable
set containing U\* is U\* ∪ L with L up to n/2, since U\* is a maximal playable set of sources: greedy kept every source
it could).  If U holds, take an optimal set S ⊇ U\*_{≥3}; by Theorem B it holds U\*_1 and U\*_2 as well, so S ⊇ U\*, and
opt(n) = ΣS ≤ in_the_bag + P\*(Π_n) ≤ opt(n).  If U fails, no optimal set contains U\*, so every playable set containing
it is lighter than opt(n).  ∎

So a result proven `exhausted` or `trivial` is opt(n) given U at n (and the code), and so is a repeat (`same_search`) of
a game m (when m was proven at its ceiling, Theorem M's soundness at m makes that ceiling m's heaviest promotion set).
A result proven at the ceiling or by the prime formula rests on the game before it, not on the model.

*What is known of U.*

* It holds wherever the prover has proven opt(n) on the whole board, where the recorded game plays U\* and is optimal
  ([results/README.md](../../results/README.md#what-supports-the-scores) says how far that reaches).  In its strong
  form, every optimal set contains U\*, it holds in every game whose optimal sets were enumerated (954 games of 2..989;
  `solver/frame/tests/bottom_layers.rs` checks Theorem B on all of them).  Past the prover's proofs, the committed games
  proven by an exhausted search or a repeat rest on it, and `taxman certify` checks it game by game
  ([results/README.md](../../results/README.md#certificates) says which games have a certificate).
* U\*_{≥3} is nearly all the sources of three or more prime factors: greedy rejects none of layer 4 or more on any board
  to 8191, and in layer 3 none before 741 and 10 of 523 at 4000 (the census).  So U says: some optimal game plays every
  source of four or more prime factors, and every source of layer 3 that greedy keeps.
* The barred parity bounds (Theorem 10, with layers 1 and 2 fixed by Theorem B and one member u of U\*_{≥3} barred)
  certify most members of U\*_{≥3} game by game: when the bound is at most the recorded score V, every set beating V
  that obeys Theorem B plays u.  Every game leaves some open: over the 490 games of 1301..2175 that rest on the frame
  model, 7.5% of the members (1 to 102 a game, 84% to 99.7% certified), and over the 635 of the EC2 chain's 2176..3314,
  14.9% (15 to 317 a game, 54% to 98% certified), the open share growing with the root's gap by 2 to 3 points per 100.
  The root's heaviest set plays all of U\*_{≥3} in every one of these games.  (`solver/noframe`, `bin/upper.rs`; the
  data in `solver/noframe/data/upper_*.jsonl` and the table in `solver/noframe/docs/proofs.md`, "The upper part, game by
  game".)

*Where a proof stops.*  Take an optimal S without U\*_{≥3}, and d the largest source where S and U\* disagree, in layer
k ≥ 3.  If d ∈ S ∖ U\*, S holds d and the members of U\*_k above it, which greedy found unplayable even with all their
maximal factors in the pool; S's pool is smaller: a contradiction.  So d ∈ U\* ∖ S, and S + d is unplayable in layer k
alone (Theorem L; d is nobody's factor).  The layer instance is separated, so Theorems 4-6 and the victim law hold in
it, and every core of S_k + d holds d.  If one member v < d of layer k lies in every minimal core, S − v + d is playable
and heavier (a v up to n/2 returns to the pool of layer k + 1, which only helps); so is S − y + d if giving up one
number y of layer k − 1 that S plays (returning y to the pool of layer k) breaks every core.  So a counterexample needs
d in two or more minimal cores of its layer instance with no common victim below d: the n = 9170 phenomenon of the
frontier argument, now inside a layer and with numbers up to n/2 among the members that could be given up.  Where the
cores are graph-like (members with at most two live factors) the argument closes, as in layer 2; the gap is the odd
cores, whose members have three distinct primes.
