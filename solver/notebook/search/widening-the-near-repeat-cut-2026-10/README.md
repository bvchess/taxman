# Widening the near-repeat cut (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-22.

**Question.**  The near-repeat cut ([Lemma R](../../../../docs/theory/search-rules.md#the-near-repeat-lemma)) carries the
proof of a proven game M into the search of a later game N, at the nodes whose frames are no harder in M.  How much of
the chain's search does it reach, which widenings of it are sound and worth building, and which steps can no cut of
its kind ever reach?

The work ran beside the chain on its 88-thread host, at most 8 threads at `nice -n 19`, in a clone of master at e4b749c
with the study module `search::widen` and the example `widen` (`widen.patch` beside this entry, built with `--features
studies`).  A *reference* is a proven problem Π_M with a bound B ≥ P\*(Π_M) that the cut compares a node with; the
chain's reference is game n − 1.  The shares of steps are of the 49 games the chain searched from 4548 to 4917 (2.86 ×
10¹⁰ steps), and in brackets without 4875, whose search before conflict branching took 1.56 × 10¹⁰ of them.  A unit of
a snapshot is weighed by the notebook's line, log10 steps = 0.511 + 0.356 · gap/100.

## 2026-10-22: the kinds of step, and what a reference can reach in each

**Three kinds of step.**  `kinds.py` sorts the chain's searched games by the change of P\* from game n − 1:

| kind of step | games | steps | share |
|---|---|---|---|
| P\* falls: n odd, a move added | 14 | 1.97e10 | 69.1% (31.8%) |
| P\* rises: n even, n/2 freed | 20 | 5.49e9 | 19.2% (42.4%) |
| n − 1 prime: n even, a rise from n − 2 | 10 | 2.51e9 | 8.8% (19.4%) |
| P\* the same: near repeats | 4 | 6.89e8 | 2.4% (5.3%) |
| 4641, whose game n − 1 has no P\* recorded | 1 | 1.48e8 | 0.5% (1.1%) |

`widen diff M N` shows what changes.  An odd n adds its move n to the frame of its layer, and with it at times a factor
and a second move (4893 adds 3262, 4893 and 1631): N is a restriction of M.  An even n turns s = n/2, a move of frame h
in M, into a candidate of home h + 1, and adds the move n, which s pays for, to frame h + 1 (4900: s = 2450, frames 2
and 3).  A near repeat swaps a move (4901: 2783 for 4901) or a factor and its two moves (4899: 2546, 3819 and 1273 for
3266, 4899 and 1633).

**Where Lemma R holds at the units of the snapshots** (`widen units M N`, M = N − 1; the check of the node's own
promotions and candidates, as the search would make it on entering the unit):

| game, kind | units | Lemma R holds: units, share of the line | what fails |
|---|---|---|---|
| 4901, one move swapped | 6,590 | 6,296, 74.8% | the move 2783 of M left in M's frame (25.2%) |
| 4899, a factor and its moves swapped | 4,928 | 4, 0.0% | the moves 2546 and 3819 of M left in M's frame |
| 4900, P\* rises | 6,275 | 0 | s = 2450 is a candidate (96.3%) or a promotion (3.7%) |
| 4880, P\* rises | 2,359 | 0 | s = 2440 a candidate (100%) |
| 4884, P\* rises | 1,880 | 0 | s = 2442 a candidate (78.0%) or a promotion (21.9%) |
| 4875, 4893, 4895: P\* falls | 14,230, 2,270, 5,459 | all, but P\*_M > P\*_N | the bound: P\*_M is above P\*_N |
| 4893 from 4888 (P\* 543,860 ≤ 543,874) | 2,270 | 0 | 2445, freed by 4890, a candidate |

At 4901 Lemma R holds at exactly the units where the study's check of equal frames did.  The widenings below change
none of these counts.

**What no reference can reach.**

1. *Steps where P\* falls* (69.1%, or 31.8%).  N is M with moves added, so every promotion set of N is one of M, and
   Lemma R's condition holds at every unit.  But M's sets are worth up to P\*_M, above anything N can reach, so M bounds
   nothing that N's search needs.  An earlier game whose P\* is low enough lies at least two steps back, across an even
   game whose freed n/2 is a candidate of N and no candidate of the reference (4893 from 4888: no unit).
2. *Steps where P\* rises*, from n − 1 or from n − 2 across a prime (28.0%, or 61.8%).  A promotion set L of N that
   holds s is, less s, a promotion set of M: frame h of N with L applied is M's with L less s applied, and frame h + 1
   of N is M's with the move n added.  So M bounds such a completion by P\*_M + s, and P\*_N − P\*_M < s, or the ceiling
   would have settled the game.  s is no candidate of any earlier game (it is a source of every board below n), so no
   earlier game bounds a completion that holds s, by any matching of frames; and s is in the promotions or the
   candidates of every unit of the three snapshots.  Beyond that, `widen greedy` finds at many units a promotion set of
   N worth more than P\*_M, which no reference of bound P\*_M or less can cut whatever its frames: 41.8% of the line at
   4900, 50.1% at 4880, 17.8% at 4884 (lower bounds: the heaviest of four greedy completions a unit).  Where N's optimum
   holds s, as at every step measured, it is, less s, a set of M worth s − (P\*_N − P\*_M) below M's optimum (6 at 4900,
   572 at 4880, 1,554 at 4884): N's proof is about a band of M's sets that M's search pruned whole against P\*_M and
   never told apart.
3. *Near repeats* (2.4%, or 5.3%) are the cut's whole reach.  The swaps of one move, 4823 and 4901, hold 1.1% (2.6%);
   the swaps of a factor and its two moves, 4805 and 4899, 1.3% (2.7%).  Over every searched game of the chain with a
   step count, near repeats hold 6.0% of the steps: the factor swaps 3.4e9 (34 games, 3939 the largest with 9.7e8), the
   swaps of one move 1.1e9 (12 games).

**Whole games** (`game.sh`: proved from the root against the known optimum, one thread, at most 1,500 CPU seconds;
the cut against game M, in mode 0, Lemma R, and in modes 15 and 17, its widenings below, which made the same tree
wherever they ran; every run that ended did so at the optimum):

| N from M | kind | default: steps | Lemma R: steps | ratio | nodes cut |
|---|---|---|---|---|---|
| 4693 from 4692 | one move swapped | 572,354 | 30 | 0.0001 | 9 |
| 2821 from 2820 | one move | 32,868 | 3,946 | 0.12 | 706 |
| 2945 from 2944 | one move | 63,213 | 32,121 | 0.51 | 4,464 |
| 2527 from 2525 | one move | 4,345,210 (the other entry) | 62,819 | 0.014 | 15,879 |
| 3367 from 3366 | one move, three primes | 2,543,218 (the other entry) | 699,947 | 0.28 | 118,328 |
| 2067 from 2065 | a factor and its two moves | 11,338 | 11,122 | 0.98 | 77 |
| 2139 from 2136 | factor swap | 15,823 | 14,151 | 0.89 | 599 |
| 2301 from 2300 | factor swap | 1,251,869 | 1,247,848 | 1.00 | 530 |
| 2451 from 2450 | factor swap | 163,587 | 150,945 | 0.92 | 4,330 |
| 2613 from 2610 | factor swap | 7,892 | 8,387 | 1.06 | 48 |
| 2679 from 2676 | factor swap | 196,675 | 183,485 | 0.93 | 1,310 |
| 2847 from 2844 | factor swap | 28,467 | 28,432 | 1.00 | 27 |
| 2883 from 2880 | factor swap, 29² to 31² | 157,489 | 20,010 | 0.13 | 1,775 |
| 2967 from 2964 | factor swap | 39,317 | 29,071 | 0.74 | 2,133 |
| 3021 from 3020 | factor swap | 19,560 | 19,718 | 1.01 | 405 |
| 3081 from 3080 | factor swap | 908,521 | 875,237 | 0.96 | 2,203 |
| 3243 from 3240 | factor swap | 1,797,916 | 1,576,693 | 0.88 | 81,621 |
| 3477 from 3475 | factor swap | 1,727,981 | 1,709,546 | 0.99 | 36,503 |
| 2379, 3237 | factor swap | stopped | stopped | | |
| 4805 from 4802 | factor swap, 29² to 31² | | stopped at 1,497,807 | | 245,040 |
| 3006, 3010, 3020, 3024, 3030, 3040 | P\* rises | 17,775 to 171,219 | the same | 1 | 0 |

Wherever the other entry ran its check of equal frames (3021, 3081, 3243, 3477, 2527, 3367), Lemma R cut the same
nodes and made the same tree, to the step.  On the rising steps the check fails at every node on the freed n/2:
it is a promotion (at 3006, 17,839 of 17,840 checks) or a candidate, and where it is neither (6,287 checks at 3030),
M's frame still holds a move that N's lacks.

## 2026-10-22: why the cut does not carry a factor swap

The factor swaps are three times the cut's prize.  The cut takes 7% of their steps on the 13 small ones that ended
(5,874,645 against 6,326,435, from a 6% loss at 2613 to an 87% saving at 2883) and nothing at 4899's units.  What
decides it is the shape of what is swapped.

**A factor paid for by two moves alone is one move of four factors** (Lemma G below).  In 4898 the moves 2546 = 2·1273
and 3819 = 3·1273 share the factor 1273 = 19·67, which nothing else pays with, so together they act as one move that
pays with any of 38, 134, 57 and 201 (2·19, 2·67, 3·19, 3·67) and takes all four.  In 4899 the same holds of 3266 and
4899 with 46, 142, 69 and 213.  So a factor swap is a swap of one move, as 4901's is, but of a move of four factors
where 4901's 2783 had two (121 and 253).  Every factor swap from 2000 to 4900 has this shape, f = pq going to the f' of
the next primes (19·67 to 23·71), but two: at 2883 and 4805, f = 29² goes to 31², and the move has two factors, 2·29 and
3·29 at 2883, 3·29 and 5·29 at 4805 (its moves are 3f and 5f).  There the cut does better.  It proved 2883 in 20,010
steps against 157,489.  4805, proved from its root on one thread with the cut, was stopped at 3,600 CPU seconds after
1,497,807 steps (the chain took 9.8e7 on 88 threads), with the cut holding at 19% of its node checks, against 0.3% at
3081.

**That move does not leave M's frame.**  A move leaves a frame by the reduction, when all its factors but one are
gone, or as a free move (Lemma F), when another factor of it is paid with by nothing else.  2783 goes when one of its
two factors goes, and it had gone at 6,296 of 4901's 6,590 units.  The four-factor move goes only when three of its
four factors have gone, and those are hubs.  At three of 4899's units (`widen explain`: the unit of the largest gap,
one of the median gap, and the first), frame 0 holds 325 to 339 moves and 394 to 418 factors, and 560 to 578
candidates can be promoted into it; 38 is paid with by 21 movers (5 of them sources, always there), 134 by 8 (2
sources), 57 by 21 (8 sources), 201 by 8 (3 sources).  The move is in M's frame at all but 4 of the 4,928 units.  Lemma
C, which drops every mover, candidates included, that can be in no unwinnable set, keeps it there too: at those three
units it keeps every move and 98% of the candidates.  So at every unit some choice of the candidates still open makes
M's frame 0 unwinnable through it.  Taking candidates out of C does not help: `widen cover`, which takes out the
candidates paying with the factors that keep it, six times over, finds no set that lets the check hold at any unit,
since sources pay with each of the four.

**And N is not harder.**  Where M's move goes by the reduction, its last factor goes with it, and N keeps that factor
free for the candidates that pay with it.  Neither frame is harder than the other: N has M's four factors free and
needs its own, M the reverse.  The difference is real, not a weakness of the check: at 3,107 of 4899's 4,928 units
(50.9% of the line), one of a dozen greedy completions (`widen greedy`) is a promotion set of 4899 and not of 4898.
No argument that a node's completions are sets of M can cut those units, by any check and under any order of
branching; at 4901 the same test finds such a completion at 18 units (2.7% of the line).

**Steering the branching to settle the hubs first**, an idea raised during the work, is exact, as any branching is, but
it cannot make the move leave M's frame.  Leaving out a candidate leaves the hub to the sources that pay with it, 2 to 8
of them at every unit explained; promoting one takes the hub only if a reduction later leaves that candidate paying with
the hub alone.  `widen cover` measures the first: leaving out every candidate that pays with a hub never lets the check
hold.  And half the line lies in units that hold a set of N that M lacks, which no order can cut.

**What else was tried, and why it fails.**

* *A renaming* (Lemma S) of the swapped moves and factors: ρ must send 46 to a factor of M's frame paid with by
  exactly the movers whose images pay with 46, and the candidates paying with 46 (92, 138, …) are not those paying
  with 38.  It found no renaming at any unit.  Renaming the candidates as well would carry a completion to a set of
  other numbers; the natural map, 23 ↦ 19 and 71 ↦ 67, makes every member smaller, while the cut needs the image
  worth at least as much.
* *A bound on the swapped frame alone.*  The two games differ in frame 0 only, so P\*_N is the largest of g(S₀) over
  the sets S₀ of candidates promoted into frame 0 that N's frame 0 takes, and P\*_M the same over those M's takes, with
  one function g for both (the best of the other frames given S₀, plus ΣS₀).  M's proof bounds g on M's family; the
  sets in N's family and not in M's are the dissolved conflicts of the other entry, which no enumeration reaches, and
  the greedy test above finds them at half the line.
* *The common relaxation*, frame 0 without either gadget, is easier than N everywhere, so a proven bound V on it would
  settle N at its root.  But its search is a search of a relaxation of N against the same incumbent, no cheaper than
  N's own, and nothing in the chain proves it otherwise.
* *M's conflicts, carried where the frames compare* (Corollary N below): sound; but for the 4899 kind M is often a
  game settled by its ceiling or as a repeat, with no search and no conflicts (4898 repeats 4896, settled at its
  ceiling), and carried conflicts lost where they were measured.

## The widenings and their proofs

Each widening keeps Lemma R's statement and conclusion and replaces part of its frame condition by a weaker one.  The
words are those of the [ledger](../../../../docs/theory/search-rules.md#the-near-repeat-lemma): a node of game N's search has
the promotions P and the candidates C; on frame j, C_in are the members of C of home j + 1 (moves the frame may gain)
and C_out those of home j (factors it may lose); a *mover* of frame j is a move of the frame or a member of C_in.  A
move *pays with* its maximal factors in the frame, and taking it takes them all.  Each lemma below has been checked at
random (the last section), and none has had an independent review.

**Lemma P (partial reductions).**  The state lemma holds with σ(P) replaced, frame by frame, by any frame obtained
from the root's frame with P applied by some of the reduction's drops, in an order the reduction allows (a move left
with one live factor, dropped with that factor).  So Lemma R holds when condition 3 compares, frame by frame, any
such *partial state* of N with any partial state of M.

*Proof.*  The state lemma's proof runs over the drops one at a time; run it over the drops made only.  Fix a frame, R
the frame with P applied, and (m₁, q₁), …, (m_k, q_k) the drops made, giving R_k.  If X leaves every qᵢ in place, R with
X applied is winnable iff R_k with X applied is, by the reduction lemma, drop by drop.  Otherwise let qᵢ be the first
that X takes away: after the drops before it, mᵢ has the single live factor qᵢ, which X takes away, so R with X applied
is unwinnable; and qᵢ, a member of X, is missing from its home in R_k, since applying P takes away P's members only.  So
P ∪ X is a promotion set iff every member of X is a factor of its home in the partial state and every frame of it is
winnable with X applied.  Lemma R's proof uses nothing else of σ_M(P) and σ_N(P), and a partial state of a promotion
set is winnable where X does not reach, as σ is.  ∎

**Lemma F (free moves).**  Let a be a move of a frame j at the node, and q a factor of the frame that a pays with,
such that q is not in C and no other move of the frame and no member of C_in pays with q.  Then for every X ⊆ C the
frame with X applied is winnable iff the frame without a and q, with X applied, is.  So M's frame may be compared with
its free moves dropped, one after another, each drop judged on the frame the drops before it left.

*Proof.*  If a winning order of the frame with X applied is given, leave a out.  Every other mover b paid at its turn
with a factor p ≠ q (no mover but a pays with q), and p is still there without a: leaving a out only leaves its
factors in place longer.  Conversely, a winning order of the frame without a and q, with X applied, followed by a, wins
the frame: q is there at the end, since it is not in X (q ∉ C) and no mover takes it.  The members of C stay factors
where they were, since q ∉ C.  ∎

Dropping a free move of M's frame never hurts condition 3: it takes a move out of M's frame, and its factor q pays for
nothing that the condition asks about.

**Lemma S (a renaming).**  On a frame j touched by C, let φ be an injection of the moves of M's frame into the moves of
N's frame, extended by the identity on C_in, and ρ an injection of a set D of factors of N's frame into the factors of
M's frame, such that

1. D holds every factor of N's frame that φ(a) pays with, for every mover a of M's frame;
2. for every mover a of M's frame and every q in D, a pays with ρ(q) iff φ(a) pays with q;
3. every member of C among N's factors is a factor of M's frame; ρ(q) = q for q in D ∩ C, and ρ(q) is not in C for
   the other q of D.

Then every X ⊆ C whose members of home j are factors of N's frame, and with which N's frame is winnable, leaves M's
frame winnable too, and its members of home j are factors of M's frame.  So condition 3 of Lemma R may be replaced,
frame by frame, by the existence of φ and ρ.  Condition 3 is the case of φ and ρ the identity, D the factors the two
frames share.

*Proof.*  The members of X of home j are in C and factors of N's frame, so of M's, by 3.  Take a winning order of N's
frame with X applied and keep in it the images of M's movers, the φ(a) and the members of X_in.  At its turn each kept
φ(a) paid with a factor q, not in X_out, that no move before it had taken.  By 1, q is in D; by 2, a pays with ρ(q),
a factor of M's frame; ρ(q) is not in X_out, by 3 (ρ(q) = q ∉ X when q ∈ C, and ρ(q) ∉ C otherwise).  No kept move
before it took ρ(q) in M's frame: if a mover b before a paid with ρ(q), then φ(b) pays with q by 2, and φ(b) would have
taken q at its turn in N's order, while q was still there at a's.  So the preimages, in the same order, win M's frame
with X applied.  ∎

The study looks for φ among the bijections of the moves of M's frame that N's lacks onto those of N's that M's lacks
(at most 3 onto 6), the identity elsewhere, and builds ρ factor by factor: ρ(q) = q where the movers paying with q in
both frames agree, else a factor of M's frame not in C paid with by exactly the preimages of q's payers.

**Lemma C (cores).**  On a frame j touched by C, let the *residual* be what is left of M's movers (its moves and the
members of C_in) after dropping, again and again, a mover that pays with a factor of M's frame that is not in C and
that no other mover left pays with.  Suppose every move of M's frame in the residual is a move of N's frame, and every
factor of N's frame that M's lacks is not in C and pays for no mover in the residual.  Then every X ⊆ C whose members of
home j are factors of N's frame, and with which N's frame is winnable, leaves M's frame winnable too, and its members of
home j are factors of M's frame.  Condition 3 is the case where nothing is dropped; Lemma F's drops of free moves are
drops of the residual.

*Proof.*  A frame is an instance of the [decision procedure](../../../../docs/theory/playability.md#the-core-theorem): its movers
are the members, its factors the pool, and each mover lists the maximal factors it pays with.  The instance is
separated, since the movers of a frame lie in one layer and their factors one below.  So by Theorem 5 a frame is
winnable iff no nonempty set of its movers is *self-covering*: every factor that a member of the set pays with is
paid with by another member of it.  The members of X of home j are in C and factors of N's frame, so of M's.  Suppose
M's frame with X applied is not winnable, and let T be a self-covering set of its movers, against the factors of M's
frame less X_out.  T lies in the residual: of the movers dropped, take the first that is in T; when it was dropped,
all of T was left, and it paid with a factor q not in C, so not in X_out, that no other mover left paid with; so no
other member of T pays with q, against T being self-covering.  So T's moves are moves of N's frame, and its other
members are in X_in.  T is self-covering in N's frame with X applied too: a factor of N's frame less X_out that a
member of T pays with is a factor of M's frame, since the factors that M's lacks pay for no mover in the residual; so
another member of T pays with it.  By Theorem 5 (the direction that needs no separation), N's frame with X applied is
not winnable.  ∎

**Lemma G (a factor paid for by two moves alone).**  Let a and b be moves of a frame j at the node whose only common
factor f is not in C and is paid with by no other mover.  Then for every X ⊆ C the frame with X applied is winnable iff
it is with a, b and f replaced by one move w that may pay with any factor of a or b other than f and takes them all.

*Proof.*  Given a winning order with a and b, say a first: a takes f, so b pays with a factor of its own other than f.
Put w where b was, paying with that factor, and leave a out: a's other factors stay until then, which harms no move,
and after w's turn the factors left are the same as before.  Given a winning order with w, paying with a factor of b,
say: put a, paying with f, and then b, paying with that factor, where w was; a does not take that factor, since a and b
share no factor but f, and afterwards the same factors are gone.  ∎

Lemma G explains the factor swaps and is not a cut.  The study's mode 16 checks Lemma C; it found nothing that Lemma
R's condition does not.

**Lemma K (a cover).**  Let K ⊆ C be such that Lemma R, or a widening, holds at the node with the candidates C less K.
Then every completion of the node worth more than the incumbent holds a member of K, and the node needs only the
children that promote a member of K, made as the node's loop makes them.  *Proof.*  A completion P ∪ X with X ⊆ C
less K is a completion of the node with those candidates, worth at most B.  ∎  `widen cover` looks for a small K at
4899's units and finds none.

**Corollary O (one-sided candidates).**  Lemma R and the lemmas above hold for the extended problem Π⁺(O), with each
one-sided member of C read as a member of C_in of its own frame that takes no factor away, and condition 1 asking that
it be a one-sided candidate of Π_M with the same own frame.  *Proof.*  The state lemma holds in Π⁺ word for word:
applying a one-sided member adds a move, and the reduction lemma covers edits that add moves.  In Lemma R's proof a
one-sided member of X is a kept mover of its frame, paid as a member of C_in is.  ∎

**Corollary N (the previous game's conflicts).**  Where the frame j of a node compares (condition 3, or Lemma S), a set
K ⊆ C that leaves M's frame j unwinnable leaves N's unwinnable too: no completion holding K is a promotion set of N.
*Proof.*  The proof of Lemma R on frame j, with X = K, read backwards.  ∎  It is sound, and buys nothing a search lacks.
Where condition 3 holds on every frame and the incumbent reaches B, the cut ends the node, which no conflict improves
on.  Where only the bound fails, the step is a restriction (P\* falls): N's frames compare with M's at every node, so
the matching region is the whole tree, and the share of N's conflicts that M had learned is the one the other entry
measured over whole games, 19% to 73%.  There every nogood of M is a nogood of N already, checked by one peel at N's
root (Lemma N of [carrying a search's learning](../carrying-a-searchs-learning-to-the-next-game-2026-10/README.md)), and
carrying them was measured at ×0.90 to ×1.74 the CPU on its three restriction steps (×1.31 in geometric mean over its
eight pairs).  In a near repeat the twin units learn the same conflicts, step for step, but those units are the ones the
cut ends.

## 2026-10-22: the lemmas checked at random

`widen fuzz 60 700 --nodes 300` takes, for every n from 60 to 700 with a problem and M = n − 1 and n − 2 (1,282
pairs), 300 random nodes of N: P promoted greedily from a random order of the candidates and cut short at random, C a
random part of the candidates left.  At each node it checks every mode, and where a mode holds it makes 16 completions
greedily in N, each tested in M.  None of the completions tested was a promotion set of N and not of M:

| mode | nodes where it holds (of 384,600) | completions tested | promotable in N, not in M |
|---|---|---|---|
| 0, Lemma R | 222,183 | 3,554,928 | 0 |
| 1, with Lemma F | 235,350 | 3,765,600 | 0 |
| 15, with Lemmas F, P and S | 245,741 | 3,931,856 | 0 |
| 16, with Lemma C | 235,675 | 3,770,800 | 0 |
| 31, all | 246,029 | 3,936,464 | 0 |

So on random nodes of small games the widenings hold at up to 11% more nodes than Lemma R, while on the nodes the
chain's searches reach they add none.  `widen verify 4900 4901` (mode 15) tested 5,040 completions at 630 of 4901's
units where the check holds: 0 promotable in 4901 and not in 4900.

## 2026-10-22: where it stands

* **The reach.**  The near-repeat cut can reach the near repeats only: 2.4% of the chain's steps from 4548 to 4917, or
  5.3% without 4875's search, and 6.0% over the chain's whole record.  Lemma R as proven cut exactly what the study's
  check of equal frames cut, wherever both ran (the units of 4899 and 4901, six whole games), and its widenings cut
  no more anywhere.  It takes nearly all of a swap of one move (4693 in 30 steps, 2821 in 0.12, 2527 in 0.014 of the
  default's) and of a factor swap whose factor is a square (2883 in 0.13), and 7% of the other factor swaps at small
  n.
* **What it cannot reach, and why.**  Where P\* falls, the previous game's P\* is above the incumbent, and the earlier
  games below it lack a candidate of N (the freed n/2 of the even game between).  Where P\* rises, the freed n/2 is a
  promotion or a candidate at every unit and node measured, and a completion that holds it is a set of no earlier
  game.  At the factor swaps of the 4899 kind, half of 4899's line lies in units holding a set of N that M lacks.
* **The widenings** (Lemmas P, F, S, C and K, Corollaries O and N) are proved here and checked at random (no
  counterexample in 3.9 million completions); they widen the cut on random nodes of small games, by up to 11%, and
  on none of the nodes the chain's searches reach.  None is worth building.  None has had an independent review.

Recommendations, best first:

1. **Keep the tactic `near_repeat` (8e2b4d7) as it is, with no widening.**  Expected gain: the swaps of one move, 1.4%
   of the chain's steps over its record (4901: 750 steps on one thread against 2.3e8 on 88), and part of the two factor
   swaps whose factor is a square (4805: 9.8e7 steps; the cut held at 19% of the checks of its first 1.5e6 steps on one
   thread).  Evidence: the tables above and the other entry's.  Cost: none more.  The test that decides it: the chain's
   next swap of one move, and 4805 proved whole on the host with the tactic, at 8 threads.
2. **Stop looking for a cut of this kind for the other steps.**  The steps where P\* falls or rises, 97% of the
   chain's steps from 4548 to 4917 (94% without 4875), are out of its reach for the reasons above, which hold for any
   earlier game as the reference; and the factor swaps of the 4899 kind hold sets that M lacks in half their units.
   What could carry them is a different kind of argument: a proof about the band of M's sets between N's optimum less
   n/2 and M's optimum, or about the sets that use M's hubs, which M's search never told apart.  No such argument is
   known.
3. **Read a factor swap's gadget before its search** (`widen diff`): where f is a square the cut took 2883 nearly
   whole; elsewhere it took 7% of the steps at small n.

## Files

The runs were made on the chain's host in a clone of master at e4b749c with `widen.patch` applied, built with
`--features studies` (`build.sh`); the scripts name that host's folders.

* `widen.patch`: the study module `solver/frame/src/search/widen.rs`, the check of Lemma R and of its widenings by the
  bits of a mode (1 Lemma F, 2 and 4 Lemma P for N and for M, 8 Lemma S, 16 Lemma C; 0 Lemma R alone); its call at the
  start of each GF(2) check; the references of `tune solve`, from `TAXMAN_WIDEN_REF` (games joined by commas) and
  `TAXMAN_WIDEN_MODE`; and the example `widen`: `diff M N` (what differs between the problems), `units M N --units
  FILE` (the check of each mode at each unit of a snapshot), `greedy` (a dozen greedy completions a unit: worth more
  than P\*_M, or a set of N that M lacks), `cover` (Lemma K's set), `explain` (why the check fails at one unit),
  `verify` and `fuzz FROM TO` (the checks at random).
* `kinds.py STRICT FROM TO`: the chain's searched games by the change of P\*.
* `units.sh M N SNAP`: `widen units` and `widen greedy` on a snapshot, to `out/`; `reach.py` and `above.py`: their
  tables, collected in `snapshots.txt`.
* `game.sh N SETTING [CPU]`, `pool.sh JOBS`, `jobs-units.txt`, `jobs-games.txt`, `jobs-swaps.txt`: the snapshot
  checks and the whole games, one thread each, at most 8 at a time (4 while a lab study runs); `games.py`: their table,
  and `games.jsonl`, every run that ended or was stopped by its limit.
* `checks.txt`: the output of `widen fuzz 60 700 --nodes 300` and of `widen verify 4900 4901` (mode 15).
