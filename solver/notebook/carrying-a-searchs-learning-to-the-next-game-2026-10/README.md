# Carrying a search's learning to the next game (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  In a bad patch of the chain, a run of games whose root gap is high (3388-3424 on the EC2 chain), is the
solver doing 99% of the same search over and over, game after game?  The owner asked it, with a second question
behind it: can the frame solver carry the most important things one game's search learns into the next game's?  Taken
apart: what changes from one searched game to the next; how much of the next game's search is the previous game's,
and why; what does repeat; how much could any reuse of work save; what can be carried soundly; and what does carrying
it give?  This entry records the diagnosis and the plan that followed it.

*What was kept.*  Nothing in the solver, and one example: `examples/diff.rs` (how the search problems of two boards
differ, below).  Everything else was scratch: a copy of master 2e8b851 with recording hooks and a few commands added to
two examples, run on the laptop at `nice -n 10`, one thread a run, about 35 minutes of CPU in all, nothing on EC2.  The
copy and its logs were not kept, so the tables below are the record, and the method is written out to be redone.  A
*searched game* below is a game that poses a search problem of its own (not a prime, not a repeat that `same_search`
settles); its search ends by exhausting the tree or at its ceiling.

*The answer in brief.*  **The difficulty repeats, the work does not.**  Consecutive searched games pose problems that
differ by one local edit, keep the same root gap (the same hard core) and relearn many of the same nogoods, but their
search trees share almost no node, even when both are proven at the same level; a two-line theorem (Theorem D, below)
says why: what one game's proof establishes about values says nothing about what the next game's proof must refute.
What carries soundly (nogoods, the branching order, a value-free bound state) is measured to give nothing in the
present solver: re-validated nogoods make the next search 1.31 times slower in geometric mean.  And the arithmetic of
the runs limits any reuse of work to 1.2 to 1.4 times between games whose cores match.  What is worth carrying is what a
better bound learns about the core: the fields study's core-derived signings and multipliers, which carry with no check
at all.  A pilot at five pairs of roots says they do: a family fitted at one game's root closes 0.75 (the median) of
what the next root's own fit closes, with no refit, and 1.08 of it after a tenth of the budget spent refitting, more
than its own in four of five pairs, because the two games' cores add up.  So carrying is not a lever in the present
solver; it is the way avenue A1's bound, once it is in the search (A1's E3), can learn along the chain.

*Method.*  The games are the chain's: the recorded games of `results/strict.jsonl` (to 2175) and, past it, the EC2
chain's results of 2026-10-15 (2176-3423, 64 threads; that snapshot is not committed, but
`solver/notebook/the-gap-past-2175-2026-10/gaps_chain.jsonl` holds its games to 3312 with their root gaps and nodes).  The instruments:

* `examples/diff.rs` (new, in the repository): `diff [--known FILE] A:B ...` sets up the boards A and B as the solver
  does and compares their search problems frame by frame (the moves and factors each has that the other lacks, the
  freedoms), the candidates gained and lost and those whose home frame changed, `max_total`, `in_the_bag`, and, for the
  optimal promotions of each game (read from the chain), whether they are all candidates on the other board and can all
  be promoted from its root.
* A step log (scratch, the hooks inert unless switched on): per step of a searcher, that is per pass of its loop at
  a node, the 64-bit FNV-1a hash of the node's promotions (their numbers ascending, four bytes each, little endian),
  the hash of the child pushed (0 if none), the outcome (loop ended, child not pushed, child pushed), the node's depth,
  its candidates after the step and its promotions; and every nogood the search learned (in `refine::Seek::conflict`,
  next to the insert into the table), as its numbers.  Written to a file of 24-byte records by the ladder
  (`examples/ladder.rs --threads 1 --known results/strict.jsonl N`, which seeds a game from the recorded game of n − 1
  and stops it at its ceiling, as `taxman solve` in order does).  A game searched twice has the same steps (the base run
  of 1832 in the nogood table below has the 114,150 steps of its recorded run).  A script reads two such files and
  counts: the distinct nodes, the nodes both took steps at, the steps of each at the other's nodes, the identical steps
  (same node, child and outcome), the common prefix of the two step sequences, the children both pushed, the nogoods
  both learned.
* The proof at a fixed value (scratch, in the ladder): the search with the incumbent a value V and no set behind it,
  as `wall nodes --mode opt` runs one, so that two games are proven at the same level.
* A preloaded table (scratch): game n's learned nogoods, each kept only if every member is a candidate of n + 1 and
  `promote_all_using` from n + 1's root fails, made minimal there (`FrameGraph::nogood` with nothing promoted), inserted
  into every searcher's table when it is made (*carry*: forgotten with the rest after the first 50,000 steps of that
  searcher) or put back after each forgetting as well (*pin*); the table counts the lookups answered by a carried one.
* For the pilot of section 7, a `carry` command of `examples/fields.rs` (below).

**1. What changes from one searched game to the next.**  The end of the EC2 chain, from 3372 to 3423 in its snapshot:
each searched game against the searched game before it (the repeats and primes between them pose the problem of one of
them), from `diff`.

| step | the edit | shape | P\* change | nodes of the later game (64 threads) |
|---|---|---|---|---|
| 3372→3375 | frame 3 gains the move 3375 | restriction | −162 | 25.7M |
| 3375→3376 | 1688 leaves frame 1's moves; frame 2 gains 3376 and the factor 1688; candidate 1688 | relaxation | +124 | 26.2M |
| 3376→3380 | the same with f = 1690 (frames 1→2) | relaxation | +630 | 45.1M |
| 3380→3381 | frame 1 gains 3381 | restriction | −16 | 27.9M |
| 3381→3384 | f = 1692 (frames 2→3) | relaxation | +1,571 | 36.0M |
| 3384→3388 | f = 1694 (frames 1→2) | relaxation | +569 | 46.6M |
| 3388→3390 | f = 1695 (frames 0→1) | relaxation | +169 | 47.4M |
| 3390→3392 | f = 1696 (frames 3→4) | relaxation | +360 | 26.5M |
| 3392→3393 | frame 1 gains 3393 | restriction | −57 | 103.0M |
| 3393→3395 | frame 0 gains 3395 | restriction | −291 | 68.0M |
| 3395→3396 | f = 1698 (frames 0→1) | relaxation | +12 | 63.0M |
| 3396→3399 | frame 0 gains 2266, 3399 and the factor 1133 | restriction | −150 | 420.6M |
| 3399→3400 | f = 1700 (frames 2→3) | relaxation | +1,556 | 522.1M |
| 3400→3402 | f = 1701 (frames 3→4) | relaxation | +752 | 93.8M |
| 3402→3404 | frames 0 and 1: six candidates gained (1178 … 1702) | relaxation (a larger edit) | +1,664 | 280.9M |
| 3404→3405 | frame 0 gains 2270, 3405 and 1135 | restriction | 0 | 0 (the ceiling) |
| 3405→3408 | f = 1704 (frames 2→3) | relaxation | +1,274 | 158.7M |
| 3408→3411 | frame 0 gains 2274, 3411 and 1137 | restriction | 0 | 0 (the ceiling) |
| 3411→3416 | f = 1708 (frames 1→2) | relaxation | +650 | 93.2M |
| 3416→3417 | frame 0 ±3 moves; frame 1 loses 7 moves and 7 factors; 7 candidates lost | restriction (a larger edit) | −1,584 | 93.5M |
| 3417→3420 | f = 1710 (frames 2→3) | relaxation | +1,409 | 206.6M |
| 3420→3423 | frame 0 gains 2282, 3423 and 1141 | restriction | −336 | 118.7M |

*Relaxation* is the shape of ["Going higher"](../going-higher-and-coming-back-down-2026-10/README.md), section 2 (from an odd n to
n + 1 = 2f: f leaves the moves of frame h, becomes a candidate of home frame h + 1, and the new move 2f has f as a
private factor).  In every such step the earlier game's optimal promotions are promotable on the later root and not the
reverse, and the later optimum contains the new candidate.  *Restriction* is a new move, or two new moves over a new
factor (2266 and 3399 over 1133, 2270 and 3405 over 1135); in every such step the later game's optimal promotions are
promotable on the earlier root, and the earlier game's are not promotable on the later root unless the optimum stays
(3405 and 3411, which the ceiling settles).  The single-f relaxations meet "Going higher"'s sufficient condition for
one problem to relax another, and the one-move restrictions meet it with the roles exchanged (the problem without the
move relaxes the problem with it); the two-move additions and the two larger edits (3402→3404, 3416→3417) are classed
by the optimal-set check alone.  The analogues below 2175 are the same: 1460→1463 and 1472→1475 (frame 0 gains a move),
1536→1539 (frame 2 gains a move), 2064→2065, and 1463→1464, 1475→1476 and 1539→1540 (f = 732, 738, 770).  Two
near repeats are different problems with the same optimum, which `same_search` cannot see: 1832→1833 (frame 0 swaps the
two moves 946 and 1419 over the factor 473 for 1222 and 1833 over 611, nothing else changes) and 2065→2067 (1034 and
1551 over 517 for 1378 and 2067 over 689), both a swap of one factor's pair {2pq, 3pq} over pq in frame 0 for
another's.

So every step is one local edit, as the owner says, and **all but the steps the ceiling settles move the optimum** (by
12 to 1,664 in the table above).  Of the 443 exhausted searches of 2176-3423 on the EC2 chain, those whose P\* differs
from the previous searched game's by more than 500 hold 62.8% of the nodes (243 games), by 101-500 19.3% (98), by
21-100 11.5% (58), by 1-20 4.3% (23), by 0 2.1% (21).

**2. How much of game n + 1's search is game n's.**  Each pair searched on one thread as the ladder does it, with the
step log on.  The 14 pairs are consecutive searched games of 1460-2172 that one thread records in minutes (29 thousand
to 553 thousand steps), restrictions and relaxations, with shifts of the optimum from 0 to 714.  *Steps at shared
nodes*: the share of n + 1's steps taken at a node at which n also took a step; *identical steps*: the same node,
child and outcome; *nogoods known*: the share of the distinct nogoods n + 1 learned that n had learned too.

| step | edit | P\* change | steps (n, n + 1) | steps at shared nodes | identical steps | first step that differs | nogoods known |
|---|---|---|---|---|---|---|---|
| 1832→1833 | near repeat | 0 | 114,150 / 114,150 | 100% | 100% | none | 100% |
| 2065→2067 | near repeat | 0 | 73,037 / 73,037 | 100% | 100% | none | 100% |
| 1830→1832 | relaxation (916) | +8 | 112,861 / 114,150 | 38.9% | 35.8% | 29 | 63% |
| 1463→1464 | relaxation (732) | +296 | 87,630 / 88,367 | 12.8% | 11.0% | 17 | 73% |
| 1540→1544 | relaxation (772) | +8 | 250,502 / 174,658 | 5.1% | 4.1% | 16 | 40% |
| 2170→2172 | relaxation (1086) | +12 | 440,988 / 510,340 | 12.6% | 11.7% | 788 | 46% |
| 1470→1472 | relaxation (736) | +563 | 136,217 / 352,483 | 0.2% | 0.1% | 18 | 52% |
| 2064→2065 | restriction | −647 | 59,772 / 73,037 | 0.1% | 0.0% | 5 | 47% |
| 1460→1463 | restriction (frame 0) | −197 | 28,897 / 87,630 | 0.0% | 0.0% | 2 | 26% |
| 1472→1475 | restriction (frame 0) | −179 | 352,483 / 553,347 | 0.0% | 0.0% | 0 | 32% |
| 1475→1476 | relaxation (738) | +640 | 553,347 / 353,508 | 0.0% | 0.0% | 1 | 41% |
| 1536→1539 | restriction (frame 2) | −459 | 337,476 / 170,013 | 0.0% | 0.0% | 4 | 54% |
| 1539→1540 | relaxation (770) | +342 | 170,013 / 250,502 | 0.0% | 0.0% | 4 | 19% |
| 1464→1470 | three steps | +714 | 88,367 / 136,217 | 0.0% | 0.0% | 3 | 9% |

The trees are shared only where the optimum barely moves: a near repeat is the same tree step for step; a shift of 8
shares between a twentieth and two fifths of the steps, 12 an eighth (296 an eighth once); a shift of 179 to 647 shares
a handful of nodes near the root and nothing else.  The median of the 14 pairs is 0.15% of the steps at shared nodes.
Past 2175 the games whose optimum moved by 20 or less hold 6.4% of the nodes (section 1).

*Is it the level or the edit?*  The restriction 1472→1475 proven at fixed levels, the incumbent a value with no set
behind it, one thread:

| proof | steps |
|---|---|
| 1472 at its optimum, 52,565 | 309,591 |
| 1475 at 52,565 (1472's optimum, which is 1475's ceiling) | 9,196 |
| 1475 at its optimum, 52,386 | 489,761 |

At the same level the two trees share the root and nothing else (the first child differs), and 1475 at its two levels
shares 117 of 489,761 steps.  So the edit alone changes the tree from its first branch (one move of frame 0 changes
the root's heaviest set, its conflict and its forced members), and so does the level alone.  The table also shows where
a restriction step's cost is: nothing above the ceiling is proven in 9 thousand steps, nothing above the optimum 179
below it in 490 thousand (53 times, the margin scans' 0.01 decades per unit of value, see ["What drives the
slope"](../what-drives-the-slope-2026-10/README.md)).

**3. Why the work does not repeat: the two proofs have disjoint obligations.**  For a search problem Π (a game's root:
its frames, candidates and `max_total`) let Prom(Π) be its promotion sets and P\*(Π) the largest sum ΣT over
T ∈ Prom(Π).  For a value V, the *obligation* O(Π, V) = {T ∈ Prom(Π) : ΣT > V} is what a search whose incumbent is
worth V must show empty.  The search of game n shows O(Π_n, P\*_n) = ∅.  Say Π′ *relaxes* Π when Prom(Π) ⊆ Prom(Π′):
every set Π can promote from its root Π′ can promote from its own.  ("Going higher" proves a sufficient condition on
the two roots, which the single-f relaxation steps of section 1 meet, and the one-move restriction steps with the roles
exchanged.)

**Theorem D.**  (a) If Π_{n+1} relaxes Π_n, then O(Π_{n+1}, V) ∩ Prom(Π_n) = ∅ for every V ≥ P\*_n, in particular for
V = P\*_{n+1}: every set the proof of n + 1 must refute is unpromotable in Π_n.  (b) If Π_n relaxes Π_{n+1} (a
restriction step), then O(Π_{n+1}, V) = {T ∈ Prom(Π_{n+1}) : V < ΣT ≤ P\*_n}: the obligation of n + 1 lies in the band
of values (P\*_{n+1}, P\*_n], about which the proof of n claimed nothing.

*Proof.*  (a) T ∈ Prom(Π_n) gives ΣT ≤ P\*_n ≤ V, so T ∉ O(Π_{n+1}, V).  (b) T ∈ Prom(Π_{n+1}) ⊆ Prom(Π_n) gives
ΣT ≤ P\*_n.  ∎

(The theorem is about what the searches must refute, and nothing in the solver or the prover rests on it, so it stays
here rather than in THEORY.md, which holds the theorems the two programs rest on; the sufficient condition for a
relaxation, which it uses to classify the edits, is proved in "Going higher".)  What it means for each kind of fact a
search establishes:

* *Value facts*, which are what a branch and bound tree is made of (a subtree is cut because its bound is below its
  need: every completion of the node (P, C) is worth less than need − ΣP at the incumbent of the time).  Across a
  relaxation they bound sets of Π_n, none of which is in the next obligation: no use.  Across a restriction they stay
  true (the restricted problem's GF(2) matroids are contained in the other's, "Going higher" section 3, so its bound at
  a node is no larger), but they settle a subtree of n + 1 only if they were cut with a margin of at least
  P\*_n − P\*_{n+1}, and the trees do not share their nodes anyway (section 2).  The one value fact that carries in
  full is the one the chain already carries, the ceiling, which in a restriction step that adds the new source to the
  bag is exactly P\*_n; and the part of n + 1's obligation it settles, the sets above P\*_n, is the cheap part (at 1475,
  9 thousand of 490 thousand steps, section 2).
* *Facts of promotability* (nogoods: N cannot be promoted from the root) do not depend on values: they carry wherever
  they stay true (section 6).
* *Bound state that is not a value* (signings and multipliers of a Lagrangian bound): carries unconditionally, since
  any signing and any multipliers give a valid bound (section 6).

So "the same search over and over" is not what happens: the next proof refutes sets the previous proof never had to
look at, in a tree of its own.

**4. What does repeat: the hard core.**

* *The gap.*  Over the 509 searched games of 2176-3312 (`solver/notebook/the-gap-past-2175-2026-10/gaps_chain.jsonl`) the root GF(2) gaps of
  consecutive searched games have lag-1 autocorrelation 0.95; of consecutive searched games at most 6 apart, half
  (51%) have exactly the same gap and three quarters gaps within 50 (median change 0, mean 33, against an sd of 211).
  The edit moves the bound and the optimum together ("Going higher" section 5 saw it on 37 of 37 chain steps), so the
  odd cores of frames 1-3 that make the gap are the same game after game.  That is why the cost residuals run (lag-1
  0.84, [the gap past 2175](../the-gap-past-2175-2026-10/README.md)) and the bad patches come in runs: the same core priced again,
  at 10^(G/250), game after game.
* *The conflicts.*  19% to 73% of the nogoods the next game learns were learned by the previous one in the 11 pairs of
  one edit that move the optimum (median 46%; 9% across three edits, 100% for the near repeats; the last column of the
  table of section 2).  Of the previous game's nogoods, across a restriction step every one is still a nogood (1,102
  of 1,102 and 2,240 of 2,240 in the pairs carried in section 6, as Lemma N (ii) says; 326 of 326 at 2064→2065);
  across a relaxation step most are (651 of 651, 967 of 968, 1,628 of 1,628, 1,942 of 2,186, 612 of 1,035: the freedom
  that f's leaving gives frame h releases the nogoods that leaned on it).

Same difficulty, different work: each game refutes the same core along its own path, because the bound cannot hold what
the previous game learned about the core.  The owner's expectation that the wall will be met in such a patch is right:
the wall is a crest of the gap's waves, where every game costs about 10^(G/250) on its own; reuse cannot lower a crest,
and a bound that closes a share of G can.

*What the bound learns.*  Nothing persistent, in the default search.  The GF(2) bound is a function of the problem; its
per-node state (the heaviest set, the weight split, the forced members, the certified levels) is tied to the node's
need, is recomputed in micro- to milliseconds, and changes with every edit (section 2: the root's choices differ at
once).  The only state a searcher accumulates is its nogood table, a cache forgotten every 50,000 of its steps; the
Lagrangian multipliers over nogoods exist only with `lagrange`, off by default and changing no tree measured with
`gf2`.  The one expensive, value-free bound state in the project is the fields study's: core-derived GF(3) signings and
their multipliers, tens of seconds to fit at a root and valid on any problem.

**5. The limit of any reuse of work.**  A search that proves a run of games together, or reuses every piece of work from
game to game, still contains a proof of the run's dearest game: with perfect reuse a run costs its dearest member.  Over
the EC2 chain's 400 exhausted searches of 2176-3312 (the nodes of `solver/notebook/the-gap-past-2175-2026-10/gaps_chain.jsonl`), runs cut where
consecutive searched games are more than 6 apart or their gaps differ by more than a tolerance:

| runs (gap tolerance) | total over the sum of the runs' dearest | saving if each run cost its dearest |
|---|---|---|
| equal gaps (252 runs) | 1.22 | 18% |
| within 10 (224) | 1.29 | 22% |
| within 50 (134) | 1.41 | 29% |
| within 100 (63) | 1.84 | 46% |
| any gap (15 runs) | 2.95 | 66% |

On the patch 3396-3423 (10 exhausted searches, 2.05 billion nodes, the dearest 3400 with 0.52 billion) the ratio is 3.9
times; 2.6 with 3424 at 1.3 billion nodes (a figure the snapshot lacks, which ends at 3423).  Over the committed chain's
1301-2175, solved again by the current build on one thread (`solver/notebook/the-gap-past-2175-2026-10/resolved_1301_2175.jsonl`, 314 exhausted
searches), it is 1.28 with equal gaps and 1.47 within 10.  So perfect reuse between games whose cores match would give
1.2 to 1.4 times, and 10 times is out of reach of any reuse of work: it needs the dearest games themselves to get
cheaper, which only a tighter bound gives.  And the measured reuse is far from perfect (section 2).

**6. What can be carried soundly.**  Every row says when it is valid in game n + 1, the lemma that proves it, what it
can buy, and what was measured.

| what | valid in game n + 1 when | proof | what it can buy | measured | verdict |
|---|---|---|---|---|---|
| the incumbent (the seed) | always, its survivors | any promotable set is an incumbent | the start | in use | done |
| the ceiling | always (n + opt(n − 1)) | `prev_cap` | in a restriction step, all of n's proof that bears on n + 1 (Theorem D) | in use | done |
| nogoods | re-checked: every member a candidate, and not promotable from n + 1's root (exact, a peel per frame); no check needed in a restriction step | Lemma N | conflicts known sooner | ×0.90 to ×2.74 the time, ×1.31 in geometric mean | killed |
| subtree certificates (value facts) | restriction steps only, as upper bounds, cutting only with a margin ≥ P\*_n − P\*_{n+1} | Lemma R | nothing: no shared nodes | 0% of steps shared in restriction steps | killed |
| the branching order | always (any branching is a partition) | the search's exactness argument | strong branching's time at shared nodes | the trees part at the first branches | killed |
| the gap, as an aspiration value | always, if a failed aspiration is followed by a normal search | Lemma A | the cost of finding the optimum late: 12% at 1472, 11% at 1475, on the half of the games whose gap repeats | not built | not worth building |
| forced members, weight splits, certified levels | not as such: tied to the need and to the matroids | none | none | none | not carried |
| the root's matroids, symmetric pairs, problem setup | recomputed per game | none | milliseconds a game | none | not carried |
| a joint search of a run | sound if a node is cut only where it is cut for every member | Lemma J | the union of the trees instead of their sum | the trees share no node even at one level | killed |
| replay of a near repeat | each replayed verdict re-certified in n + 1 | Lemma P | most of a near repeat's search | near repeats hold 2.1% of the nodes past 2175 | not worth building |
| signings and multipliers of the fields bound (A1) | always: the signing by numbers, any nonzero value on a new incidence, multipliers by candidate number | Lemma S (Theorems 14, 15) | the next root's family and fit; a family that grows along the chain | pilot at 5 roots: no refit 0.59-0.94 of the root's own share (median 0.75), refit with separation for 12 s 0.82-1.17 (median 1.08) | the one worth building, inside A1's E3 |

*Nogoods carried on the GF(2) path (measured).*  Game n's learned nogoods, carried and pinned as in the method above,
into game n + 1 searched as in section 2; user CPU seconds from `/usr/bin/time` (so a stopped process does not count),
one thread:

| step | kept of n's | base: steps, s | carry: steps, s | pin: steps, s | lookups answered by a carried nogood (pin) |
|---|---|---|---|---|---|
| 1830→1832 (relaxation, +8) | 651 of 651 | 114,150, 10.1 | 192,425, 16.6 (×1.64) | 187,546, 16.6 (×1.64) | 95% |
| 1463→1464 (relaxation, +296) | 967 of 968 | 88,367, 10.5 | 82,025, 9.9 (×0.94) | 76,760, 9.5 (×0.91) | 99.8% |
| 1472→1475 (restriction, −179) | 1,102 of 1,102 | 553,347, 55.6 | 874,743, 94.3 (×1.69) | 871,796, 94.8 (×1.70) | 94% |
| 1475→1476 (relaxation, +640) | 1,942 of 2,186 | 353,508, 26.4 | 646,395, 55.0 (×2.09) | 829,676, 72.2 (×2.74) | 93% |
| 1536→1539 (restriction, −459) | 2,240 of 2,240 | 170,013, 16.9 | 148,402, 15.2 (×0.90) | 136,874, 15.4 (×0.91) | 100% |
| 1539→1540 (relaxation, +342) | 612 of 1,035 | 250,502, 23.6 | 266,100, 24.8 (×1.05) | 227,617, 21.9 (×0.93) | 54% |
| 2064→2065 (restriction, −647) | 326 of 326 | 73,037, 10.5 | 83,548, 12.8 (×1.21) | 120,722, 18.3 (×1.74) | 99.8% |
| 2170→2172 (relaxation, +12) | 1,628 of 1,628 | 510,340, 67.7 | 720,115, 95.3 (×1.41) | 465,767, 67.2 (×0.99) | 100% |
| geometric mean of the eight | | | ×1.31 (steps ×1.27); 2 of 8 faster | ×1.34 (steps ×1.25); 4 of 8 faster, by 1-9% | |

Mostly valid (every one across a restriction; across a relaxation 59% to 100%, the rest made promotable by the freedom
frame h gains), re-checked in microseconds, answering most lookups, and worse on balance.  As on the matching path
in ["Carrying nogoods over"](../carrying-nogoods-over-2026-10-measured-not-kept/README.md) (1227, the best a carried table can be:
−10% nodes, +3% time; the chain 1201-1231 617 s to 941 s), a carried table changes which conflict a node branches on
(the smallest the table knows, rather than one computed from the frame where the heaviest set fails), so it changes the
tree, and the change is noise around a loss.  A larger table also costs every node (["Forgetting
nogoods"](../forgetting-nogoods-2026-10/README.md)).

*The lemmas.*

**Lemma N (nogoods across games).**  Let N be a set of candidates of Π_n that cannot be promoted from its root.  (i) N
is a nogood of Π_{n+1} exactly when N ⊆ C_{n+1} and `promote_all_using` from Π_{n+1}'s root fails; every use the search
makes of a nogood (the conflicts of nodes, the rule-outs of children) rests on that alone (`model::Nogoods`), so a
table loaded with such sets is as sound as one learned.  (ii) If Π_n relaxes Π_{n+1}, every such N with N ⊆ C_{n+1} is
a nogood of Π_{n+1} with no check (a set promotable in n + 1 is promotable in n).  (iii) In a relaxation step of the
shape of section 1 (n + 1 = 2f), at every node that has promoted f, every nogood of Π_n (less the node's promotions) is
a conflict with no check: if T ∋ f can be promoted in Π_{n+1}, then frame h with T applied is n's frame h with T − f
applied, and frame h + 1 is n's frame h + 1 with T − f applied plus the move 2f (f itself being promoted out), so
T − f can be promoted in Π_n.  The check of (i) costs a peel per frame and is what an implementation would do anyway.

**Lemma R (value facts in a restriction step).**  If Π_n relaxes Π_{n+1}, then at every node (P, C) of n + 1, every
upper bound on the completions of (P, C′) in Π_n with C ⊆ C′ bounds the completions of (P, C) in Π_{n+1}, and the
GF(2) bound of n + 1 at (P, C) is at most n's (the contracted frame matroids of the restriction are contained in the
relaxation's).  It cuts the node in n + 1 only if it is below P\*_{n+1} + 1 − ΣP.  The rule of symmetric pairs does not
carry: a pair of n need not be a pair of n + 1 (the move 3417 = 3·17·67 breaks the pairs of 17 and of 67).

**Lemma A (the gap as an aspiration value).**  Let V be a guess of P\*_{n+1}, say n + 1's root bound less n's gap.  A
search started with the incumbent value V − 1 and no set behind it either finds a set worth V or more, after which it
is the ordinary search, exact; or completes without one, which proves P\*_{n+1} ≤ V − 1, a ceiling for the ordinary
search that must then follow.  Exact either way; it saves the time spent finding the optimum late when the guess is
right (half the games keep their gap), and costs the cheap proof above the guess when it is too high.  What it could
save is the difference between a game's search and its proof at its optimum: 352,483 steps against 309,591 at 1472
(12%), 553,347 against 489,761 at 1475 (11%).

**Lemma J (joint search).**  Let a tree's branching be any partition, and cut a node only when, for every member k of
the run, it is cut for k (its bound for k below its need for k, or none of its completions is promotable in k).  The
part of the tree each member sees is then a branch and bound of that member, so the search is exact for each.  Its
size is the union of what each member needs under the common branching, at least the dearest member's; with the trees
measured disjoint (section 2), the union is the sum.

**Lemma P (replay of a near repeat).**  Replay n's tree in n + 1, re-certifying each verdict there (a cut: the bound
below the need, or a conflict by a peel; forced members by their bounds; the symmetric pairs of n + 1) and searching
afresh from any node whose verdict fails.  It is a branch and bound of n + 1 with a given branching and re-checked
prunings, so it is exact.  Where the trees differ it saves strong branching and nothing else; where they agree (the
near repeats) it saves most of a search, and those hold 2.1% of the nodes past 2175.

**Lemma S (signings and multipliers across games).**  Let σ be a signing of frame h of Π_n over GF(3) (a nonzero value
on each incidence of a mover with a maximal factor in the frame), given by numbers: `Signing { fixed: (mover, factor)
→ value, default }` as `examples/common/gf3.rs` holds it.  Read on Π_{n+1}'s frame h, it gives σ(u, y) where (u, y) is
an incidence of both and the default (1, or a hashed 1 or 2) elsewhere: a signing of Π_{n+1}'s frame h.  So every
promotion set of Π_{n+1} is independent in its matroid (Theorem 14), and with any multipliers (n's, by candidate number,
0 for a new candidate) L bounds every promotion set of Π_{n+1} (Theorem 15; [THEORY.md, signed frame
matroids](../../../docs/theory/bounds.md#signed-frame-matroids-the-frame-bound-over-other-fields)).  A core of n whose
movers, their incidences and its factors are unchanged in n + 1 has the same column sums, so the carried signing still
makes it dependent.  Frame h is the same layer in both games (numbering by layer, ℓ₀ = 3 on every board past 186;
[THEORY.md, numbering by layer](../../../docs/theory/frame-model.md#numbering-by-layer)).  Nothing is re-checked: validity is
unconditional, only the strength depends on the fit.

*Ideas from other fields, and why they fit or not.*

* *Incremental SAT* (learned clauses kept across calls, re-validated by their derivations when clauses go; the heuristic
  state kept): fits the nogoods, which here re-validate more simply, by an exact test.  What it buys in SAT, clause
  reuse and the branching state, measured here as a loss: a carried nogood steers the branching, not the bound.
* *Reoptimization in MIP* (SCIP's, Gamrath, Hiller and Witzig 2015; SYMPHONY's warm start, Ralphs and Güzelsoy 2006:
  keep the previous tree's leaves and re-solve each in the changed problem): assumes the leaves mostly stay pruned.
  Theorem D says ours cannot (the obligation moves into the band, or into sets that were unpromotable), and the trees
  share nothing.  Does not fit.
* *Parametric and postoptimality analysis in IP* (Geoffrion and Nauss 1977): a tree valid over a range of a scalar
  parameter.  Our edits change the constraints, not a parameter of the objective.  Does not fit.
* *Proof reuse in verification* (IC3/PDR lemmas re-validated after a change; proof repair): lemmas that stay true are
  kept.  The nogoods are such lemmas; the replay of a near repeat is proof repair.  Fits, worth little here.
* *Nogood recording across restarts in constraint programming* (Lecoutre and others, 2007): as the nogoods; large
  tables cost every node ("Forgetting nogoods").
* *Lagrangian warm starts across related instances* (rolling horizons; column generation restarted from a previous
  instance's duals): fits A1's multipliers, since any multipliers give a valid bound.
* *Benders decomposition with cut reuse*: optimality cuts of an unchanged subproblem stay valid when only the master
  changes, so value knowledge of the core frames would carry across edits elsewhere (most restriction steps edit frame
  0, where the GF(2) bound has no excess).  It needs a search decomposed at a frame boundary, which is avenue A5's
  AND/OR search with caching, ranked high-risk because the cores sit in adjacent frames.  Noted, not proposed.
* *Solution-guided search, phase saving*: the seed, in use.  *Pseudocosts carried across instances*: exact strong
  branching's cycles are about a ninth of a step (DESIGN.md, the proof of 1521), so at most that.  *Incremental
  heuristic search* (LPA\*, D\* Lite): needs states shared between searches; the search has no transpositions, and the
  trees share no state.

**7. The A1 pilot: do the fields bound's signings and multipliers carry?**  The one carry with a reason to pay: in a bad
patch the cores persist (section 4), so the core-derived signings fitted at one root should see most of the next root's
gap.  `examples/fields.rs` already holds what is needed (`dw::bound` takes signings and starting multipliers by
candidate number and returns the final family and multipliers; its `nodes` command carries a root's to its nodes the
same way), and the pilot added a `carry` command to a scratch copy: `fields carry --known results/strict.jsonl
--seconds 120 --refit 0.1 A:B ...`.  At A's root, the main arm (`arm("f1cores", …)`, `bound(ctx, signings, None, true,
settings)`) with a budget of 120 s, keeping the family's signings and the multipliers.  At B's root: *own*, B's main arm
with the budget; *own short*, with a tenth of it (12 s); *carried*, one `evaluate` of A's signings, frozen (no cores
added), at A's multipliers re-indexed by number (0 for a candidate A did not have); *refit*, `bound(ctx_B, frozen,
Some(&lambda_A), false, short)`; *refit with separation*, the same with `true`.  Each is reported as the share of B's
root GF(2) gap it closes (by min(L, GF(2) bound)), and every bound is asserted at least P\*_B.  The share of B's root
GF(2) gap closed, and in brackets its ratio to B's own fit; one thread, about 8 minutes in all:

| A→B (P\* change) | gaps of A, B | A's own fit (s) | B's own fit (s) | B's own, 12 s | carried, no refit | refit, 12 s | refit with separation, 12 s | signings then |
|---|---|---|---|---|---|---|---|---|
| 2075→2076 (+36) | 327, 327 | 0.948 (15) | 0.847 (23) | 0.844 | 0.636 (0.75) | 0.948 (1.12) | 0.994 (1.17) | 47 |
| 2100→2104 (+24) | 526, 526 | 0.633 (34) | 0.671 (15) | 0.669 | 0.443 (0.66) | 0.631 (0.94) | 0.684 (1.02) | 51 |
| 1925→1926 (+895) | 215, 243 | 0.744 (11) | 0.593 (10) | 0.593 | 0.490 (0.83) | 0.638 (1.08) | 0.638 (1.08) | 43 |
| 1800→1805 (−834) | 507, 474 | 0.611 (12) | 0.587 (6) | 0.587 | 0.549 (0.94) | 0.610 (1.04) | 0.673 (1.15) | 63 |
| 2170→2172 (+12) | 767, 767 | 0.433 (120, open) | 0.505 (82) | 0.473 | 0.297 (0.59) | 0.408 (0.81) | 0.416 (0.82) | 51 |

(The refit columns are budgets of 12 s; the fits that converged sooner stopped sooner: 1800→1805's refits took 2.2 s
and 6.6 s.)  A family carried from one root and refitted briefly closes more of the next root's gap than the next root's
own converged fit in four of five pairs (by 2, 8, 15 and 17%): the carried signings and the ones the next root's
separation adds make a larger family (43-63 signings against the 26-33 one root builds), each seeing cores the other
root's separation did not reach.  So the cores one game finds are worth keeping, and a family can grow along the chain:
the learning the owner asked for, carried in the bound rather than in the tree.  The pair that goes the other way is
the largest gap, 2170→2172, whose A did not converge in its 120 seconds.  Caveats: five pairs, roots only, one thread; a
stronger root is not yet a smaller tree; a larger family costs more per evaluation (17-34 ms an evaluation here); E2's
test (in progress) of 16 signings a frame against 8 overlaps with the effect; and the pilot pairs the final family with
the multipliers of the lowest L, as the `nodes` command does, whereas E2 is reported to find that the fit to keep is
the one of the lowest L evaluated, so that the pilot can only understate the carried bound.

**8. The two criteria for carrying work in the present solver.**  The design fixed them after the scratch runs above, so
the thresholds were written knowing the values; the values are far on the failing side of both, so the placement of the
thresholds does not decide.

| to pursue | criterion | measured | verdict |
|---|---|---|---|
| tree reuse (replay, joint search, the branching order carried) | on the median step pair, at least 30% of n + 1's steps at nodes n visited | 0.15% (14 pairs; 5-39% where the optimum moved by 8 to 296, 100% for the two near repeats) | killed |
| nogood carry built into the chain | the pinned carry lowers user CPU at least 15% in geometric mean, with no pair above +10% | ×1.34 (carry ×1.31), the worst pair ×2.74 (×2.09), 4 of 8 pairs faster | killed |

*Conclusion.*  The owner's suspicion is right about the difficulty and wrong about the work.  The difficulty does
repeat: the hard core is the same game after game (the gap's lag-1 autocorrelation is 0.95, half of consecutive
searched games have exactly the same gap, and a median of 46% of the next game's nogoods were learned by the previous
one), and that is why the bad patches come in runs.  The work does not: every step not settled by the ceiling moves the
optimum (by 12 to 1,664); the trees of consecutive games share 0.15% of their steps (median of 14 pairs), 5-39% where
the optimum moved by 8 to 296, and all of them only for the two near repeats; and a proof at one level shows that the
edit alone parts the trees at the first branch.  Theorem D says why: the next proof's obligation is disjoint from what
the previous proof established about values (or lies in a band it claimed nothing about).  Nothing that carries
soundly pays in the present solver: nogoods re-checked at the new root are valid (all across a restriction, 59% to
100% across a relaxation) and answer most lookups, yet make the search 1.31 times slower (pinned 1.34; 2 and 4 of 8
pairs faster), and perfect reuse between games whose cores match would give 1.2 to 1.4 times (18-29% of the nodes past
2175), so 10 times is out of reach of any reuse of work.  What repeats is worth carrying only in a bound that learns
it: the fields bound's signings and multipliers, which need no check, carry at 0.75 of the next root's own share and,
refitted for a tenth of the budget, at 1.08.

*What it implies.*

* *Nothing to build in the present solver.*  Not the replay of the previous tree, a joint search of a run, the branching
  order carried, forced members or weight splits carried, a nogood table carried or shared between the window's games, a
  wider `same_search` by replaying near repeats (2.1% of the nodes), nor the gap as an aspiration value (a few percent
  at best).  The open row about proving a run of neighbours that share a gap in one search is closed by sections 2, 3
  and 5: the trees of neighbours are disjoint, and a run costs at least its dearest member.
* *The bound is the lever, as before.*  What a bad patch needs is a bound that closes a share of the gap at the crest
  of the wave, and carrying only helps such a bound along.
* *Step 2, for the A1 track* (not built): a `fields carry` command, as the pilot's, in `examples/fields.rs`.  Where
  E2's `--save-fit` and `common/fit.rs` (which write and read a root's fit) and E3's `Warm` (which carries a fit from a
  node to its children) have landed, the command reads A's saved fit instead of fitting A again, and *refit* is E3's
  refit from it.  Over every searched game that follows one of E1's converged roots of 1700-2175 (E1's
  `solver/notebook/a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/roots.jsonl`), plus 2170→2172 and two pairs of the 2340s from the EC2 snapshot if their roots converge
  in the budget: about 30 roots at 36-120 s, under an hour of one thread, S = 120 s and a tenth of it for the refit.
  Criterion, fixed: the family carries if on the median pair *carried* ≥ 0.7 × *own* and *refit with separation* ≥
  0.95 × *own*; then E3 starts each game from the previous searched game's family.  Otherwise E3 fits each game's root
  afresh and carries only from a node to its children (E3's form (c) anyway).  The pilot passes this on the median
  (0.75 and 1.08) and fails at 2170→2172; the thresholds were written after seeing it, so step 2 proper is the test.
  And a *chain*: ten consecutive searched games of a high-gap run (2150-2175, and 3333-3336 with 3300 if their roots
  converge), each root refitted with separation for a tenth of the budget from the family carried from the one before,
  recording the share, the family's size and the time of an evaluation, against each root's own fit.  Criterion, fixed:
  the carried share stays at least the own share at every root of the chain, with the family pruned to the signings the
  master's support uses when it passes 64; then step 3.
* *Step 3, inside E3, if step 2 passes*: each game's search starts from the previous searched game's final family and
  multipliers (Lemma S) and hands its own to the next, through the window driver as the seed is (a family is a few
  thousand numbers); a game whose roots differ in their number of frames, or below 187, starts afresh.  Sound with no
  check.  Measured by E3's own criterion: the nodes and time of a bad patch with and without the carried family.
* *Rebuilding the instrument*, if a later study wants the step log: the scratch hooks were a static switch and a global
  log behind a mutex (fine for one thread; a rebuilt one for many threads would buffer per searcher and flush with the
  searcher's other counters, never a lock per step), the nogoods recorded where they are inserted, and the optional
  preloaded table, with the ladder's three options (record, check a nogood file against a root and load it, prove at a
  value) and a script for the overlap.  With the hooks off every tree must be unchanged (the probe's fingerprints, the
  tripwires and the one-thread chain 1..1300 are the gates).

## Later (2026-10): the program removed from master

The examples `diff` and `fields`, with the files of `examples/common/` they included, were removed from master on
2026-10-17.  They build at commit 8753189: `git checkout 8753189`, then
`cargo build --release --example diff --example fields` in `rust/`.  The `carry` command of the pilot of section 7 was
in a scratch copy of `fields` and is not in that commit.
