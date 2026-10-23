# Carrying a search's learning to the next game (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-15.  Concluded 2026-10-15.

**Question.**  In a bad patch of the chain, a run of games whose root gap is high (3388-3424 on the EC2 chain), does the
solver do 99% of the same search over and over, game after game, and can it carry the most important things one game's
search learns into the next game's?  **Answer.**  The difficulty repeats, the work does not.  Consecutive searched games
keep the same root gap and relearn many of the same nogoods, but their search trees share a median 0.15% of their steps,
and nogoods re-validated and carried make the next search 1.31 times slower in geometric mean.  Perfect reuse between
games whose cores match would save only 1.2 to 1.4 times.  What is worth carrying is what a better bound learns about
the core: the multi-field bound's signings and multipliers, which carry with no check, close a median 0.75 of the next
root's own share with no refit and 1.08 after a tenth of the budget.

## Method

Nothing in the solver changed, and one example was kept: `examples/diff.rs`, which compares the search problems of two
boards.  Everything else was scratch: a copy of master 2e8b851 with recording hooks and a few commands added to two
examples, run on the laptop at `nice -n 10`, one thread a run, about 35 minutes of CPU in all, nothing on EC2.  The copy
and its logs were not kept, so the tables below are the record.  A *searched game* is a game that poses a search problem
of its own (not a prime, not a repeat that `same_search` settles); its search ends by exhausting the tree or at its
ceiling.  The games are the chain's: the recorded games of `results/strict.jsonl` to 2175 and, past it, the EC2 chain's
results of 2026-10-15 (2176-3423, 64 threads), whose games to 3312 with their root gaps and nodes are in the [gap
entry's `gaps_chain.jsonl`](../the-gap-past-2175-2026-10/gaps_chain.jsonl).  The instruments:

* **`diff [--known FILE] A:B ...`** sets up the boards A and B as the solver does and compares their search problems
  frame by frame, the candidates gained and lost, `max_total` and `in_the_bag`, and whether the optimal promotions of
  each game are all candidates on the other board and can all be promoted from its root.
* **A step log** (scratch): per step of a searcher, the hash of the node's promotions, the hash of the child pushed, the
  outcome and the depth; and every nogood the search learned.  The ladder wrote it (`ladder --threads 1 --known
  results/strict.jsonl N`, which seeds a game from the recorded game of n − 1 and stops it at its ceiling, as `taxman
  solve` in order does).  A script compared two logs: the nodes both took steps at, the identical steps, the first step
  that differs, the nogoods both learned.
* **The proof at a fixed value** (scratch): the search with the incumbent a value V and no set behind it, as `wall nodes
  --mode opt` runs one, so that two games are proven at the same level.
* **A preloaded table** (scratch): game n's learned nogoods, each kept only if every member is a candidate of n + 1 and
  `promote_all_using` from n + 1's root fails, inserted into every searcher's table when it is made (*carry*, forgotten
  with the rest after the first 50,000 steps) or put back after each forgetting as well (*pin*).
* **`fields carry`** (scratch), for the pilot of section 7.

## Results

### 1. What changes from one searched game to the next

The end of the EC2 chain, 3372 to 3423, each searched game against the one before it (22 steps; the repeats and primes
between them pose the problem of one of them): every step is one local edit.

* **Relaxation** (13 steps): f leaves the moves of frame h, becomes a candidate of home frame h + 1, and the new move 2f
  has f as a private factor, as in [going higher](../going-higher-and-coming-back-down-2026-10/README.md), section 2.
  The earlier game's optimal promotions are promotable on the later root and not the reverse, and the later optimum
  contains the new candidate.  P\* rises by 12 to 1,664.
* **Restriction** (9 steps): a new move, or two new moves over a new factor (2266 and 3399 over 1133).  The later game's
  optimal promotions are promotable on the earlier root, and the earlier game's are not unless P\* stays (3405
  and 3411, which the ceiling settles).  P\* falls by 16 to 1,584.
* The nodes of the later game run from 25.7 M to 522.1 M (64 threads), and 0 for the two games the ceiling settles.
* Two near repeats are different problems with the same optimum, which `same_search` cannot see: 1832→1833 and
  2065→2067, each a swap of one factor's pair {2pq, 3pq} over pq in frame 0 for another's.

All but the steps the ceiling settles move P\*.  Of the 443 exhausted searches of 2176-3423 on the EC2 chain,
those whose P\* differs from the previous searched game's by more than 500 hold 62.8% of the nodes (243 games), by
101-500 19.3% (98), by 21-100 11.5% (58), by 1-20 4.3% (23), by 0 2.1% (21).

### 2. How much of game n + 1's search is game n's

Each pair searched on one thread with the step log on: 14 consecutive searched games of 1460-2172 that one thread
records in minutes (29 thousand to 553 thousand steps), restrictions and relaxations, with shifts of P\* from 0
to 714.  *Steps at shared nodes* is the share of n + 1's steps taken at a node at which n also took a step.

* The two near repeats, 1832→1833 and 2065→2067, share 100% of their steps and 100% of their nogoods: the same tree step
  for step.
* A shift of 8 shares 38.9% (1830→1832) and 5.1% (1540→1544); a shift of 12 shares 12.6% (2170→2172), and one of 296
  shares 12.8% (1463→1464).  The first step that differs is the 16th to the 788th.
* A shift of 179 to 714 shares 0.2% (1470→1472) or less, in eight pairs: 0.1% at 2064→2065, and 0.0% at 1460→1463,
  1472→1475, 1475→1476, 1536→1539, 1539→1540 and 1464→1470, whose first differing step is at step 0 to 5.
* The nogoods the later game learned that the earlier game had learned too: 9% to 73% (the pairs of one edit that move
  P\*, 19% to 73%).

The trees are shared only where P\* barely moves.  The median of the 14 pairs is 0.15% of the steps at shared
nodes.  Past 2175 the games whose optimum moved by 20 or less hold 6.4% of the nodes.

*Is it the level or the edit?*  The restriction 1472→1475 proven at fixed levels takes 309,591 steps for 1472 at its
optimum 52,565, 9,196 for 1475 at 52,565 (1472's optimum, which is 1475's ceiling) and 489,761 for 1475 at its optimum
52,386.  At the same level the two trees share the root and nothing else, and 1475 at its two levels shares 117 of
489,761 steps.  So the edit alone changes the tree from its first branch, and so does the level alone.  The cost of a
restriction step is not above the ceiling (9 thousand steps) but in proving nothing above P\*, 179 below the ceiling,
(490 thousand, 53 times as much; see [what drives the slope](../what-drives-the-slope-2026-10/README.md)).

### 3. Why the work does not repeat: the two proofs have disjoint obligations

For a search problem Π (a game's root) let Prom(Π) be its promotion sets and P\*(Π) the largest sum ΣT over T ∈ Prom(Π).
The *obligation* O(Π, V) = {T ∈ Prom(Π) : ΣT > V} is what a search whose incumbent is worth V must show empty.  Π′
*relaxes* Π when Prom(Π) ⊆ Prom(Π′).

**Theorem D.**  (a) If Π_{n+1} relaxes Π_n, then O(Π_{n+1}, V) ∩ Prom(Π_n) = ∅ for every V ≥ P\*_n: every set the proof
of n + 1 must refute is unpromotable in Π_n.  (b) If Π_n relaxes Π_{n+1} (a restriction step), then O(Π_{n+1}, V) = {T ∈
Prom(Π_{n+1}) : V < ΣT ≤ P\*_n}: the obligation of n + 1 lies in the band (P\*_{n+1}, P\*_n], about which the proof of n
claimed nothing.  *Proof.*  (a) T ∈ Prom(Π_n) gives ΣT ≤ P\*_n ≤ V.  (b) T ∈ Prom(Π_{n+1}) ⊆ Prom(Π_n) gives ΣT ≤ P\*_n.
∎

What it means for each kind of fact a search establishes:

* *Value facts*, which a branch and bound tree is made of, bound sets of Π_n across a relaxation, none of which is in
  the next obligation: no use.  Across a restriction they stay true, but settle a subtree of n + 1 only if cut with a
  margin of at least P\*_n − P\*_{n+1}, and the trees share no nodes anyway.  The one value fact that carries in full is
  the ceiling, which the chain already carries.
* *Facts of promotability* (nogoods) do not depend on values: they carry wherever they stay true.
* *Bound state that is not a value* (signings and multipliers of a Lagrangian bound) carries unconditionally, since any
  signing and any multipliers give a valid bound.

### 4. What does repeat: the hard core

* **The gap.**  Over the 509 searched games of 2176-3312 the root GF(2) gaps of consecutive searched games have lag-1
  autocorrelation 0.95.  Of consecutive searched games at most 6 apart, half (51%) have exactly the same gap and three
  quarters gaps within 50 (median change 0, mean 33, against an sd of 211).  The edit moves the bound and P\*
  together, so the odd cores of frames 1-3 that make the gap are the same game after game, which is why the cost
  residuals run (lag-1 0.84, [the gap past 2175](../the-gap-past-2175-2026-10/README.md)): a bad patch is the same core
  priced again at 10^(G/250) a game.
* **The conflicts.**  19% to 73% of the nogoods the next game learns were learned by the previous one in the 11 pairs of
  one edit that move P\* (median 46%; 9% across three edits, 100% for the near repeats).  Across a restriction
  step every nogood of the previous game is still a nogood (1,102 of 1,102 and 2,240 of 2,240 in the pairs of section 6;
  326 of 326 at 2064→2065); across a relaxation most are (651 of 651, 967 of 968, 1,628 of 1,628, 1,942 of 2,186, 612 of
  1,035).

Each game refutes the same core along its own path, because the bound cannot hold what the previous game learned about
it.  Reuse cannot lower the crest of a wave of the gap, and a bound that closes a share of G can.  The default search
keeps no bound state from game to game: the GF(2) bound is a function of the problem, its per-node state is tied to the
node's need and recomputed in micro- to milliseconds, and the nogood table is a cache forgotten every 50,000 steps.  The
one expensive, value-free bound state is the multi-field study's core-derived GF(3) signings and their multipliers, tens
of seconds to fit at a root.

### 5. The limit of any reuse of work

A search that reuses every piece of work from game to game still contains a proof of the run's dearest game: with
perfect reuse a run costs its dearest member.  Over the EC2 chain's 400 exhausted searches of 2176-3312, with runs cut
where consecutive searched games are more than 6 apart or their gaps differ by more than a tolerance:

| runs (gap tolerance) | total over the sum of the runs' dearest | saving if each run cost its dearest |
|---|---|---|
| equal gaps (252 runs) | 1.22 | 18% |
| within 10 (224) | 1.29 | 22% |
| within 50 (134) | 1.41 | 29% |
| within 100 (63) | 1.84 | 46% |
| any gap (15 runs) | 2.95 | 66% |

On the patch 3396-3423 (10 exhausted searches, 2.05 billion nodes, the dearest 3400 with 0.52 billion) the ratio is 3.9
times, and 2.6 with 3424 at 1.3 billion nodes (a figure the snapshot lacks).  Over the committed chain's 1301-2175,
solved again by the then-current build on one thread
([`resolved_1301_2175.jsonl`](../the-gap-past-2175-2026-10/resolved_1301_2175.jsonl), 314 exhausted searches), it is
1.28 with equal gaps and 1.47 within 10.  So perfect reuse between games whose cores match would give 1.2 to 1.4 times,
and 10 times is out of reach of any reuse of work: it needs the dearest games themselves to get cheaper, which only a
tighter bound gives.

### 6. What can be carried soundly

What can be carried soundly, what it can buy, and what was measured:

* **In use:** the incumbent (the seed, always valid) and the ceiling (n + opt(n − 1), `prev_cap`), which in a
  restriction step is all of n's proof that bears on n + 1.
* **Killed:** nogoods (valid when every member is a candidate and not promotable from n + 1's root, a peel per frame,
  with no check across a restriction; ×0.90 to ×2.74 the time, ×1.31 in geometric mean, below); subtree certificates,
  value facts valid in restriction steps only as upper bounds cutting with a margin of at least P\*_n − P\*_{n+1} (0% of
  steps shared in restriction steps); the branching order (the trees part at the first branches); a joint search of a
  run, sound if a node is cut only where it is cut for every member (the trees share no node even at one level).
* **Not worth building:** the gap as an aspiration value, which could save the cost of finding P\* late (12% at
  1472, 11% at 1475); the replay of a near repeat with each verdict re-certified (near repeats hold 2.1% of the nodes
  past 2175).
* **Not carried:** forced members, weight splits and certified levels (tied to the need and to the matroids); the root's
  matroids, symmetric pairs and the problem's setup (recomputed per game in milliseconds).
* **Worth building, inside A1's E3:** the signings and multipliers of the multi-field bound, valid always (the signing
  by numbers, multipliers by candidate number), buying the next root's family and fit and a family that grows along the
  chain (section 7).

**Nogoods carried on the GF(2) path** (measured).  Game n's learned nogoods, carried and pinned as in the method, into
game n + 1 searched as in section 2; user CPU seconds from `/usr/bin/time`, one thread:

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
| geometric mean of the eight | | | ×1.31 (steps ×1.27); 2 of 8 faster | ×1.34 (steps ×1.25); 4 of 8 faster | |

The carried nogoods are mostly valid (every one across a restriction; 59% to 100% across a relaxation, the rest made
promotable by the freedom frame h gains), are re-checked in microseconds, answer most lookups, and are worse on balance.
As on the matching path in [carrying nogoods over](../carrying-nogoods-over-2026-10-measured-not-kept/README.md) (1227,
the best a carried table can be: −10% nodes, +3% time; the chain 1201-1231 617 s to 941 s), a carried table changes
which conflict a node branches on, so it changes the tree, and the change is noise around a loss.  A larger table also
costs every node ([forgetting nogoods](../forgetting-nogoods-2026-10/README.md)).

**The lemmas** behind the rows.  Lemma N: a set N of candidates of Π_n that cannot be promoted from its root is a nogood
of Π_{n+1} exactly when N ⊆ C_{n+1} and `promote_all_using` from Π_{n+1}'s root fails, and with no check when Π_n
relaxes Π_{n+1}.  Lemma R: across a restriction, every upper bound on the completions of a node in Π_n bounds those of
Π_{n+1}, but the rule of symmetric pairs does not carry, since a pair of n need not be a pair of n + 1.  Lemma A: a
search started with the incumbent value V − 1 and no set either finds a set worth V or more or proves P\*_{n+1} ≤ V − 1,
exact either way (it saves the lateness of finding P\*: 352,483 steps against 309,591 at 1472, 12%; 553,347
against 489,761 at 1475, 11%).  Lemmas J and P: a joint search is exact and costs the union of what each member needs
(the sum, with disjoint trees), and a replay of a near repeat saves strong branching and nothing else where the trees
differ.

**Lemma S (signings and multipliers across games).**  A signing of frame h over GF(3), given by numbers (a nonzero value
on each incidence, a default elsewhere), read on Π_{n+1}'s frame h is a signing of it, so every promotion set of Π_{n+1}
is independent in its matroid, and with any multipliers (n's by candidate number, 0 for a new candidate) L bounds every
promotion set ([THEORY.md, signed frame
matroids](../../../../docs/theory/bounds.md#signed-frame-matroids-the-frame-bound-over-other-fields); frame h is the
same layer in both games, [numbering by layer](../../../../docs/theory/frame-model.md#numbering-by-layer)).  Nothing is
re-checked: validity is unconditional, only the strength depends on the fit.

**Methods of other fields.**  Incremental SAT, proof reuse in verification and nogood recording in constraint
programming fit the nogoods and the replay of a near repeat, and measured here as a loss.  Lagrangian warm starts across
related instances fit the multi-field bound's multipliers.  MIP reoptimization assumes the previous tree's leaves mostly
stay pruned, which Theorem D denies, and parametric analysis in IP varies a parameter of the objective where our edits
change the constraints.  Benders cut reuse needs a search decomposed at a frame boundary, avenue A5's AND/OR search:
noted, not proposed.

### 7. The A1 pilot: do the multi-field bound's signings and multipliers carry?

In a bad patch the cores persist (section 4), so the core-derived signings fitted at one root should see most of the
next root's gap.  The pilot's `fields carry --known results/strict.jsonl --seconds 120 --refit 0.1 A:B ...` fits A's
root with the main arm for 120 s and keeps the family's signings and the multipliers.  At B's root: *own* is B's main
arm with the budget (and *own short* with a tenth of it); *carried* is one evaluation of A's signings, frozen, at A's
multipliers re-indexed by number; *refit* and *refit with separation* refit them for 12 s.  Each is the share of B's
root GF(2) gap it closes, with every bound asserted at least P\*_B; in brackets its ratio to B's own fit.  One thread,
about 8 minutes in all:

| A→B (P\* change) | gaps of A, B | A's own fit (s) | B's own fit (s) | carried, no refit | refit, 12 s | refit with separation, 12 s |
|---|---|---|---|---|---|---|
| 2075→2076 (+36) | 327, 327 | 0.948 (15) | 0.847 (23) | 0.636 (0.75) | 0.948 (1.12) | 0.994 (1.17) |
| 2100→2104 (+24) | 526, 526 | 0.633 (34) | 0.671 (15) | 0.443 (0.66) | 0.631 (0.94) | 0.684 (1.02) |
| 1925→1926 (+895) | 215, 243 | 0.744 (11) | 0.593 (10) | 0.490 (0.83) | 0.638 (1.08) | 0.638 (1.08) |
| 1800→1805 (−834) | 507, 474 | 0.611 (12) | 0.587 (6) | 0.549 (0.94) | 0.610 (1.04) | 0.673 (1.15) |
| 2170→2172 (+12) | 767, 767 | 0.433 (120, open) | 0.505 (82) | 0.297 (0.59) | 0.408 (0.81) | 0.416 (0.82) |

A family carried from one root and refitted briefly closes more of the next root's gap than the next root's own
converged fit in four of five pairs (by 2, 8, 15 and 17%): the carried signings and the ones the next root's separation
adds make a larger family (43-63 signings against the 26-33 one root builds), each seeing cores the other root's
separation did not reach.  So the cores one game finds are worth keeping, and a family can grow along the chain.  The
pair that goes the other way is the largest gap, 2170→2172, whose A did not converge in its 120 seconds.  The pilot has
five pairs, roots only, on one thread; a stronger root is not yet a smaller tree; a larger family costs more per
evaluation (17-34 ms here); and it understates the carried bound if the best fit to keep is the lowest L evaluated, as
E2 is reported to find.

### 8. The two criteria for carrying work in the present solver

The design fixed them after the scratch runs, so the thresholds were written knowing the values; the values are far on
the failing side of both.

| to pursue | criterion | measured | verdict |
|---|---|---|---|
| tree reuse (replay, joint search, the branching order carried) | on the median step pair, at least 30% of n + 1's steps at nodes n visited | 0.15% (14 pairs; 5-39% where P\* moved by 8 to 296, 100% for the two near repeats) | killed |
| nogood carry built into the chain | the pinned carry lowers user CPU at least 15% in geometric mean, with no pair above +10% | ×1.34 (carry ×1.31), the worst pair ×2.74 (×2.09), 4 of 8 pairs faster | killed |

## Decision

The difficulty repeats and the work does not.  Nothing that carries soundly pays in the present solver, and nothing is
built:

* **Not built.**  The replay of the previous tree, a joint search of a run, the branching order carried, forced members
  or weight splits carried, a nogood table carried or shared between the window's games, a wider `same_search` by
  replaying near repeats (2.1% of the nodes), and the gap as an aspiration value (a few percent at best).  Proving a run
  of neighbors that share a gap in one search is closed by sections 2, 3 and 5: the trees of neighbors are disjoint,
  and a run costs at least its dearest member.
* **The bound is the lever.**  A bad patch needs a bound that closes a share of the gap at the crest of the wave, and
  carrying only helps such a bound along.
* **Step 2 for the A1 track** (not built): a `fields carry` command over every searched game that follows one of E1's
  converged roots of 1700-2175 ([`roots.jsonl`](../a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/roots.jsonl)),
  plus 2170→2172 and two pairs of the 2340s: about 30 roots at 36-120 s, under an hour of one thread.  Criterion, fixed:
  the family carries if on the median pair *carried* ≥ 0.7 × *own* and *refit with separation* ≥ 0.95 × *own*.  The
  pilot passes this on the median (0.75 and 1.08) and fails at 2170→2172; the thresholds were written after seeing it,
  so step 2 proper is the test.  A chain test follows: ten consecutive searched games of a high-gap run (2150-2175, and
  3333-3336 with 3300), each root refitted from the family carried from the one before, which must keep the carried
  share at least the own share at every root.
* **Step 3, inside E3, if step 2 passes**: each game's search starts from the previous searched game's final family and
  multipliers (Lemma S) and hands its own to the next, through the window driver as the seed is.  E3's own criterion
  would measure it, the nodes and time of a bad patch with and without the carried family.
* **Rebuilding the step log**, if a later study wants it: buffer per searcher and flush with the searcher's other
  counters, never a lock per step; with the hooks off every tree must be unchanged (the probe's fingerprints, the
  tripwires and the one-thread chain 1..1300 are the gates).

## Files

The folder holds only this README.md.  The scratch copy, its logs and the `carry` command of the pilot were not kept.
`examples/diff.rs` and the example `fields`, with the files of `examples/common/` they included, were removed from
master on 2026-10-17 and build at commit 8753189: `git checkout 8753189`, then
`cargo build --release --example diff --example fields` in `rust/`.  The data the tables draw on are in [the gap
entry](../the-gap-past-2175-2026-10/README.md) (`gaps_chain.jsonl`, `resolved_1301_2175.jsonl`) and [the fields
study](../a-bound-over-gf2-and-gf3-signed-frame-matroids-2026-10/README.md) (`roots.jsonl`).

## Commits

* 6effb18: the entry and `examples/diff.rs`; 738d538 merged it into master (both 2026-10-15).
* 2e8b851: the master the scratch copy was made from.
* 8753189: the last commit with the `diff` and `fields` examples.
