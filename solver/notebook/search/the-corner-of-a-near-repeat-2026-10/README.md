# The corner of a near repeat (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-23.  Concluded 2026-10-23.

We decided not to explore it during the push to 5000, because it affects too few games.  The next steps are at the end.

**Question.**  Two consecutive games sometimes differ by one swapped move.  Could the proof of game n then be a short
proof about that move's corner of the game, which reuses the proof of game n − 1 for everything else?

**Answer.**  Open.  A sketch says that the proof of n needs to refute only the ways of playing n that use up every payer
of the old move, the factors that can pay for it, and the [near-repeat
cut](../../../../docs/theory/ledger.md#the-near-repeat-cut) does not reach those ways.  The sketch is not yet a lemma,
and nothing was built or measured on the short proof.  Near repeats hold 10% of the nodes searched from 4850 to 4955
(games 4899, 4901, 4929 and 4953), and 8.4% without 4901, which the cut already proves in 750 steps.  The cut ended 165
nodes of the 505 million of 4953 in its last run.

## The idea

Two consecutive games can pose nearly the same problem.  The recorded optimal games of 740 and 741 promote the same 44
numbers and play the same moves but one: 740 plays 374 = 2·11·17, and 741 plays 741 = 3·13·19 in its place.  Picture
the swapped move as a small corner of the game.  If that corner could be cut out, and the rest shown identical in the
two games, the proof of game n would need only a short proof about the corner, and it would reuse the proof of n − 1
for everything else.

The corner meets the rest of the game through the move's payers.  A move is paid for by one of its maximal factors,
which must still be in its frame when the move is taken ([glossary](../../../../docs/GLOSSARY.md): maximal factor,
frame, promote).  We call those factors the move's *payers*.  The move 374 has the payers 187, 34 and 22 (374 divided by
2, 11 and 17), and 741 has 247, 57 and 39.  A payer can pay for other numbers too: 22 pays for 11 numbers on the board
of 741 and for 48 on the board of 4953.  A *hub* is a payer that pays for many numbers, such as 22 and 39 (31 numbers on
the board of 4953).

## The case of 4952 to 4953

The two games pose the same problem but for one swapped move, and their optimal games promote the same 265 numbers:

| | 4952 | 4953 |
|---|---|---|
| frames, candidates, `max_total`, `greedy_sum` | 9, 1,339, 291, 538,591 | the same |
| best promotion sum P\*, as score less `in_the_bag` | 7,718,796 − 7,169,519 = 549,277 | 7,721,263 − 7,171,986 = 549,277 |
| the move of its optimal game that the other lacks | 2486 = 2·11·113 | 4953 = 3·13·127 |
| nodes of its search | 501,884,452 | 505,239,991 |

The chain started 4953 from the game of 4952, so its search held the optimum from its first progress line, after one
minute, and all 505,239,991 of its nodes were proof.  The window gives threads to the oldest game first, so 4953 waited
until 4952 concluded, with 1,453 nodes searched at its last progress line before, and only then could the near-repeat
cut be armed with the proven game of 4952.  The cut cut 311 nodes in the second run of 4953's search and 165 in the
third (a run is the stretch between a stop and a resume).  It reached almost none of the search, so 4953 repeated the
work of 4952.  4928 to 4929 is the same case, with 383,765,990 and 400,034,011 nodes.

**The payers of the two moves** (`taxman tables 4953 4953 2486 39 381 1651 22 226 1243`, saved in
[payers.txt](payers.txt)):

| move | payer | numbers it pays for on the board of 4953 |
|---|---|---|
| 2486 = 2·11·113 | 22 | 48 |
| | 226 | 8 |
| | 1243 | 2: 2486 and 3729 |
| 4953 = 3·13·127 | 39 | 31 |
| | 381 | 6 |
| | 1651 | 2: 3302 and 4953 |

The hubs are 22 and 39.  The payers 1243 and 1651 each pay for one number besides their move, and those numbers, 3729
and 3302, are moves of both optimal games.  None of the six payers is a move of either game.  The two moves meet the
rest of the game only through these six factors, so fixing what each of the six does (promoted, paying another move,
or left free) leaves the rest of the two games identical.  That is the picture.  The next section states the half of it
that a proof needs.

## A reduction (a sketch, not yet a lemma)

**Sets that leave a payer of the old move are promotable in n − 1 as well.**  Let m be the move of n − 1 that n
replaces, and suppose the problems of n − 1 and n are the same but for the frame that holds the swap: the same
candidates with the same homes, the same other frames.  Let T be a promotion set of n.  A *winning order* of a frame is
an order in which every move is paid ([winnable](../../../../docs/GLOSSARY.md)), and "with T applied" is the frame
after T's promotions, as [the ledger](../../../../docs/theory/ledger.md#the-near-repeat-cut) defines it.  If some
winning order of the swapped frame, with T applied, ends with a payer of m still in the frame, then T is a promotion set
of n − 1 too, so its sum is at most P\* of n − 1, which the proof of n − 1 established.

The reason is a rearrangement of the order.  Play it without the new move.  Every other move was paid at its turn by a
factor still in the frame, and leaving a move out only leaves more factors in the frame, so the order still wins.  Then
play m last, paid by the payer that was left.  The other frames are the same in both games, so T is promotable in n − 1.

**The new obligation of n.**  The proof of n has to refute the promotion sets of n worth more than P\*.  By the
reduction, a set of n that is also a set of n − 1 is worth at most P\*, so the proof of n needs to refute only the sets
that use up every payer of m: in every winning order of the frame with T applied, each payer of m is gone by the end,
promoted out of the frame or taken by another move, while the new move is paid.  The short proof must show that none of
these sets beats P\*.

**Where it stands.**  The reduction is a sketch.  It needs a proof as careful as the one of Lemma R (promotions in and
out of the frame, freedoms, reductions), and nothing is built on it.

**How it relates to Theorem D and to Lemma R.**  Theorem D, in [carrying a search's
learning](../carrying-a-searchs-learning-to-the-next-game-2026-10/README.md#3-why-the-work-does-not-repeat-the-two-proofs-have-disjoint-obligations),
says that the proof of a game whose incumbent is worth P\* of the game before has to refute only sets promotable in the
game and not in the game before.  Its proof needs no relaxation: a set promotable in both is worth at most P\*.  The
sketch adds a description of the sets that remain, as the ones that use up the payers of the old move.  Lemma R uses the
same fact node by node, and the corner would use it once at the root, in one statement about the payers.

## Why the near-repeat cut does not reach it

**The cut needs the frames of the two games to be the same at the node, apart from factors that pay for nothing.**  For
each frame that the node's candidates touch, Lemma R (condition 3) asks that the moves of n − 1's frame be moves of n's,
and it allows n's frame extra factors only if they pay for no move of n − 1's.  In a corner the old move m is a move of
n − 1's frame and not of n's, so the condition holds at a node only after m has left the frame of n − 1.  A move leaves
a frame when all its payers but one are gone ([the widening
entry](../widening-the-near-repeat-cut-2026-10/README.md#2026-10-22-why-the-cut-does-not-carry-a-factor-swap) shows this
for the factor swaps).  The payers of the moves of 4953 include the hubs 22 and 39, which pay for dozens of numbers, so
this happens only deep in the tree.  The cut fired 165 times in the last run of a search of 505 million nodes.

## How often it happens

**Swaps of one move are 1,729 of the 4,954 consecutive pairs of games 1 to 4955, and 79 of them were searched.**
`scan_swaps.py` reads the committed chain `results/strict.jsonl` (games 1 to 4949) and, for games 4950 to 4955, the
chain's own `~/taxman_work/chain-data/results/strict.jsonl`, whose lines are not in the committed file as of
2026-10-23.  It keeps the first completed line of each game.  A pair is a *swap* when the recorded optimal games of n − 1 and n promote the
same numbers and the moves of n are those of n − 1 with one older move replaced by n.  Game n of a swap was settled
as follows:

| settled by | swaps |
|---|---|
| the prime shortcut | 618 |
| a repeat of an earlier game | 905 |
| a known game | 124 |
| the trivial case | 3 |
| a search | 79 |

In 31 of the 79 searched swaps, game n − 1 was searched too.  The scan compares the recorded games, not the problems,
so a swap is not always a near repeat: 4921 swaps 2635 for 4921 after 4920, but its problem has 1,350 candidates against
1,357.  The output, with the old and new move, their factorizations and both node counts, is in [swaps.txt](swaps.txt).

**One kind recurs.**  21 of the 79 searched swaps are of the kind of 4953: a move 2·11·p gives way to n = 3·13·q, and q
is the prime after p every time (p = 17 and q = 19 at 741, p = 113 and q = 127 at 4953).  Where both games of a swap were
searched, the second repeats the first's work, within 11% of its nodes in these six:

| n − 1 → n | old move → new move | nodes of n − 1 | nodes of n |
|---|---|---|---|
| 740 → 741 | 374 = 2·11·17 → 741 = 3·13·19 | 108 | 108 |
| 1208 → 1209 | 638 = 2·11·29 → 1209 = 3·13·31 | 5,780 | 5,524 |
| 1832 → 1833 | 946 = 2·11·43 → 1833 = 3·13·47 | 436,882 | 443,352 |
| 2300 → 2301 | 1166 = 2·11·53 → 2301 = 3·13·59 | 3,059,845 | 2,732,880 |
| 4928 → 4929 | 2726 = 2·29·47 → 4929 = 3·31·53 | 383,765,990 | 400,034,011 |
| 4952 → 4953 | 2486 = 2·11·113 → 4953 = 3·13·127 | 501,884,452 | 505,239,991 |

The node counts are those committed, each from the build of its own run.

**The move given up is always n's shadow.**  Call the *shadow* of n the number made from n by replacing each of its
prime factors with the prime just below it, with multiplicity: the shadow of 4953 = 3·13·127 is 2·11·113 = 2486, and
the shadow of a prime is the prime before it.  In every one of the 1,729 swaps, the move that n replaces is n's shadow
when n is odd, and n/2 when n is even (2 has no prime below it):

| n | swaps | the move given up |
|---|---|---|
| prime | 662 | the prime before n (the prime shortcut's formula) |
| odd composite | 1,053, 67 of them searched | the shadow of n, in every one |
| even | 15, 12 of them searched | n/2, at n = 8, 12, 16, 24, 32, 48, 64, 96, 256, 384, 512, 768, 1024, 2048, 3072 |

There is no exception (`shadow.py`, its output [shadow.txt](shadow.txt)).  The 21 searched swaps of the 4953 kind are
the shadows of 3·13·q, and the odd semiprimes are the case that [the wiki's open questions, section
7](../../the-wikis-open-questions-2026-10/README.md#7-which-n) found from the other side: δ(pq) = p′q′, the loss of an
odd semiprime is its shadow, at 417 of the 427 up to 2175.  The even swaps are the powers of two and three times them,
where δ(n) = n/2 (the same entry, section 8).  So when two consecutive games differ by one move and promote the same
numbers, the move given up is fixed by the factorization of n alone.  The law describes which move a swap gives up, not
when a swap happens, and it is measured, not proven.  A proof would say why the shadow is the one move of the game of
n − 1 that n displaces.

## 740 to 741 in detail

The smallest of the kind has the same shape as 4953.  Both games have 6 frames, 174 candidates and `max_total`
44, the same 44 promotions, and P\* = 13,565.  The payers (`taxman tables 741 741 374 39 57 247 22 34 187`, in
[payers.txt](payers.txt)): 374 needs one of 22, 34 or 187, and 741 one of 39, 57 or 247.  The payer 247 pays only for 494
and 741, as 1651 pays only for 3302 and 4953; and 187 pays only for 374 and 561, as 1243 pays only for 2486 and 3729.

The boundary is the other moves of both optimal games that share a payer with the corner:

* 22: 44, 418, 506, 638, 682
* 34: 238, 442, 578, 646
* 187: 561
* 39: 273, 429, 507, 663
* 57: 285, 399, 627
* 247: 494

## The existing cut on these pairs

**The cut gains nothing on 741, 5% on 1209 and 15% on 1833.**  `cut_runs.sh` solves the second game of three pairs from
the committed chain up to the game before, on one thread, with the tactic `near_repeat` on and off.  The runs were made
on 2026-10-23 with the build of d10e27a; every one ended at the optimum (the committed score).  Their output is in
[cut-runs.txt](cut-runs.txt):

| game | nodes with the cut | nodes without | nodes cut |
|---|---|---|---|
| 741 | 58 | 58 | 0 |
| 1209 | 1,662 | 1,740 | 36 |
| 1833 | 95,681 | 112,176 | 1,553 |

The counts differ from the committed ones (741: 108) because those depend on the thread count, the seed and the build.

## How much it could give

**At most a tenth of the nodes searched from 4850 to 4955.**  Near repeats, the games that pose the problem of the game
before but for a swap (the same candidates, `max_total` and P\*), were 1.38e9 of the 13.70e9 nodes searched from 4850 to
4955, 10%: games 4899, 4901, 4929 and 4953.  The existing cut proves 4901 in 750 steps on one thread (the [register's
row](../../register/tactics-and-shortcuts.md)), so without its 231 million nodes the corner could add 8.4%, and only if
its proof were free.  Over the whole record, the games n of the 79 searched swaps hold 7.3% of the nodes searched
(6.35e9 of 86.79e9).  The scan prints both shares, of nodes as committed from builds of different eras.

The kind of 4953 would put the next near repeat past 5000 at 5109 (2794 = 2·11·127 gives way to 5109 = 3·13·131), if
the prime after p keeps being q.  That is an extrapolation from 21 swaps, not a check.

## Decision and next steps

**Decision.**  Not explored during the push to 5000.  The corner reaches four of the 40 games searched from 4850 to
4955, one of which the existing cut already settles.  It would reopen if the chain's later games of the kind are as dear
as 4953, and it is an item of [what to try once the chain passes 5000](../../../docs/agenda-after-5000.md).

**Next steps, not taken.**

1. Prove the reduction as a lemma, with the care of Lemma R, and relate its obligation to Theorem D.
2. Measure the short proof.  Search n only over the ways of playing that use up all of the old move's payers, on 1209,
   1833 and 2301, and compare its size with the full search.  The first and cheapest number is one bound at the root of
   n with the old move's payers forced out of reach.  How to force them so that the bound still covers every set of the
   new obligation is the first question of step 1.
3. If the short proof is small, build a *payer cut* that setup arms for each near repeat, finding the swapped pair and
   its payers automatically.

## Files

* `scan_swaps.py [STRICT.jsonl ...]`: the scan of swaps.  With no argument it reads the two files above; `./scan_swaps.py
  > swaps.txt` made [swaps.txt](swaps.txt).  It prints the counts, the shares of nodes and one line for each searched
  swap, and marks the kind of 4953 with an asterisk.
* `shadow.py`: compares the move each swap gives up with the shadow of n (odd n) or n/2 (even n); `./shadow.py >
  shadow.txt` made [shadow.txt](shadow.txt), from the top of the repository or anywhere.
* `payers.sh [TAXMAN]`: runs `taxman tables` on the two corners and counts what each payer pays for; `./payers.sh >
  payers.txt` made [payers.txt](payers.txt).  The first two lines of each corner are the moves, which pay for nothing.
* `cut_runs.sh [TAXMAN]`: the runs of the cut on 741, 1209 and 1833.  Run it from the top of the repository, with its
  output going to [cut-runs.txt](cut-runs.txt).  Each run copies the committed chain up to n − 1 to a scratch directory
  and runs `taxman solve N --use-prev --force-search --threads 1 --progress 0 --tactic near_repeat=on|off --out COPY` on
  it at `nice -n 19`.
* The binary of `payers.sh` and `cut_runs.sh` is the chain's release build of d10e27a
  (`~/taxman_work/chain/solver/frame/target/release/taxman`).  The commits since then change only the docs.
