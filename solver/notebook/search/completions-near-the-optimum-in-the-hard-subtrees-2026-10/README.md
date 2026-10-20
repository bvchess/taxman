# Completions near the optimum in the hard subtrees (2026-10)

Part of the [lab notebook](../../README.md).

**Question.**  Is a hard search hard in itself?  If a hard subtree holds a great many completions within a small
distance of the optimum, any exact proof must tell them apart, and no bound short of a near-exact one prunes them
cheaply.  If it holds few, the bound is at fault, and a better bound could exist.

**Answer.**  Mostly the bound's fault.  Four of the five units we counted, hard and easy alike, hold no completion
within 100 points of the optimum (the contrast none within 200).  Their whole trees, up to 1.1 million steps, refute the
bound's own excess, and a bound within 100 points of the truth would end each of them at its first node.  The fifth,
3300-14, is the exception: it has a completion 41 below the optimum, and at least 4,427 within 91 of it.  A proof
whose bound errs by 92 points or more must give each of those a leaf of its own: a few thousand leaves, against the
tree's 104,400.  The whole games agree: game 2255 takes 13 times the steps of game 2220 at the same gap, with 1.8
times as many games within 30 of the optimum.

Wherever completions come near the best, they are the best with a few twin swaps inside a home, mostly paired exchanges
of two close primes on two cofactors.  These are the moves that make the bound's exclusions cheap
([the anatomy](../the-anatomy-of-a-hard-subtree-2026-10/README.md)).  Their supply at the root is the same in the hard
and the easy problems, so nothing in the arithmetic of the root tells a hard search from an easy one at the same gap.

This is a diagnosis, not a method.  Two ideas with cheap tests close the entry.

## Words

* A *unit* is a node of a search with the candidates its loop has left ([the
  anatomy](../the-anatomy-of-a-hard-subtree-2026-10/README.md#words)); a whole game is the unit of its root.  Its
  *gap* is its GF(2) bound less what a completion needs to beat the optimum (for a root, the bound less the optimum).
* A *completion* of a unit is a set of its candidates that its node can promote, with at most `max_total` promotions
  in all.  Its *value* is the promotion sum of the unit's promotions and its own: the game's score, less `in_the_bag`.
  A completion *within d of the optimum* has a value of at least opt − d.  The unit's *best* is its largest value; the
  proof of a unit shows that the best is at most the optimum.
* Two numbers are *twins* when they share a maximal factor: x = m·p and y = m·q with p and q prime (1292 = 68·19 and
  1156 = 68·17).  A *twin swap* of a set takes x out and puts y in, in the same home.
* A *paired exchange* is two twin swaps of one pair of primes in opposite directions: m₁·p and m₂·q out, m₁·q and m₂·p
  in.  It costs (p − q)(m₁ − m₂), small when the primes are close and the cofactors near: 1608 = 24·67 and 1220 = 20·61
  out for 1464 = 24·61 and 1340 = 20·67 costs 6·4 = 24.
* A move is *cheap* when it costs less than 50 points, the anatomy's line for a cheap exclusion.

## What was run

**The count is the solver's own search, made to count.**  A counting search makes every completion that reaches its
floor as a child exactly once, and a hook counts the children (`wall near`,
[examples/wall/near.rs](../../../frame/examples/wall/near.rs), with a test).  It differs from the search in two ways, both
in the studies build only (`StepHook::counting`, in [src/search/hooks.rs](../../../frame/src/search/hooks.rs)):

* it offers nothing, so the incumbent stays one below the floor and no node's need moves;
* a node whose heaviest set can be promoted branches on that set's members as on a conflict's, where the search would
  stop with the set as its best completion.  Every other completion leaves out one of the set's members, so the
  branches make the set and every other.

The rule of symmetric pairs is off, since it keeps one side of each pair.  Everything else prunes as in the search:
the forced members, the nogoods, the conflicts known along the path.  Each prunes only completions below the floor, so
the count is exact.  The test counts every promotable set of game 150 (30,435 sets) at four floors and finds each
count equal to a plain listing.  A default build is unchanged: the new site of the hook is compiled out of it.

A first build counted with a plainer enumeration (the GF(2) bound alone, no nogoods, no strong branching).  At
3441-5459 within 0 of the optimum it took 1,361,633 steps and 557 CPU seconds, against 52,855 steps for the unit's
proof; the counting search takes 48,555 steps and 14 seconds there, so it replaced the enumeration.

**The problems.**  The anatomy's three units of the [tuning
suite](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md), and two more pairs, each a hard problem and an
easy one at the same gap:

* 3300-14 and 3441-1262, the big units, against 3441-5459, the anatomy's contrast, of about the same gap and a
  twentieth of the size;
* 3425-149 and 3425-150, two units of one game at gaps of 872 and 879, 253,336 steps against 48,615;
* whole games from the runs of the chain's cost model that cost far more per point of gap than their neighbours ([the
  gap past 2175](../the-gap-past-2175-2026-10/README.md), A4): 3020 against 3024, both at a gap of 766, and 2255
  against 2220, at 642 and 633.  For them, `wall nodes --mode opt` gave the one-thread proof from the optimum.

Everything ran on this laptop on one thread, on 2026-10-18, from the build of `--profile test`: about 72 CPU minutes
of counts and proofs, and about 30 of builds and checks.  Nothing ran on EC2.

## Results

### 1. How many completions lie near the optimum

The units' proofs are the suite's (one thread, from the known optimum); the roots' are `wall nodes --mode opt`.  A count
stopped by its step limit gives lower bounds (≥).

| problem | gap | proof's steps | counted within | count's steps | best | within 0 | 10 | 30 | 50 | 100 | 200 |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 3300-14 | 998 | 1,248,609 | 91 | 6,000,639, stopped | opt − 41 or above | ≥ 0 | ≥ 0 | ≥ 0 | ≥ 16 | - | - |
| 3441-1262 | 970 | 1,099,410 | 100 | 4,387,612 | below opt − 100 | 0 | 0 | 0 | 0 | 0 | - |
| 3441-5459 | 960 | 52,855 | 200 | 372,204 | below opt − 200 | 0 | 0 | 0 | 0 | 0 | 0 |
| 3425-149 | 872 | 253,336 | 100 | 818,028 | below opt − 100 | 0 | 0 | 0 | 0 | 0 | - |
| 3425-150 | 879 | 48,615 | 100 | 377,822 | below opt − 100 | 0 | 0 | 0 | 0 | 0 | - |
| game 2255 | 642 | 561,092 | 30 | 1,006,618 | opt | 1 | 5 | 101 | - | - | - |
| game 2220 | 633 | 41,776 | 30 | 60,382 | opt | 1 | 5 | 56 | - | - | - |
| game 3020 | 766 | 35,857 | 30 | 49,091 | opt | 1 | 11 | 374 | - | - | - |
| game 3024 | 766 | 29,994 | 30 | 44,599 | opt | 1 | 11 | 345 | - | - | - |

**Four units of five have nothing near the optimum.**  Hard (3441-1262, 3425-149) and easy (3441-5459, 3425-150) alike,
their best completions lie more than 100 points below it.  The anatomy's search below the node found completions 839
and 619 below at 3441-1262 and 3441-5459, so their bests probably lie far lower still.  We did not find those bests:
each 100 points more of floor multiplies a count by 3 to 8.

**3300-14 holds a dense cloud just under the optimum.**  Its best known completion is 41 below the optimum, the one
the anatomy's search found.  The count within 91 stopped at 6 million steps (1,165 CPU seconds) with 4,427
completions; a count within 41 stopped at 3 million steps (579 seconds) with that one completion.  Neither covered the
tree, so a completion nearer the optimum is not excluded, and the counts are lower bounds.  Within d of the optimum,
so within d − 41 of the best known:

| d | 41 | 51 | 71 | 91 | 141 | 241 | 41 + the gap (1,039) |
|---|---|---|---|---|---|---|---|
| completions, at least | 1 | 19 | 572 | 4,427 | not counted | not counted | not counted |

They grow tenfold for about every 30 points at the edge of the count (166 completions at 81 below the optimum, 359 at
91), so a count to d = 141 would pass 10⁵ if the growth held, and one to the gap is out of reach.  Sampling did not
reach them either.  Thirty random descents of a plainer tree of the same completions, each counted exactly from a node
at a gap of 100 or 450 (Knuth's estimator, `--probes`), met none of the 4,427 (`estimate` lines of `runs.jsonl`).  The
first build's 200 random paths to the leaves at 3441-5459, within 1,200 of the optimum, met no completion, though the
anatomy found one 619 below it.  The cloud fills a sliver of the tree around the best, and the paths miss it.

**No problem has ties at its best.**  Each whole game's optimum is the only game at its score, and 3300-14's best
known completion is the only one at its value in the parts of the tree the two counts covered.

### 2. What the near-optimal completions are: the best with paired exchanges

Every completion counted has as many promotions as the best, and differs from it by a few twin swaps.  Against the best
(at 3300-14 the best known), within the counts (`analysis.txt`, section 2):

| problem | others | members swapped, median (most) | that pair off as twins of their home | completions that pair off entirely | twin pairs in paired exchanges | members of the best that move |
|---|---|---|---|---|---|---|
| 3300-14 (within 91 of the optimum) | 4,426 | 8 (16) | 82% | 31% | 67% | 37 of 165 |
| game 2255 (within 30) | 100 | 4 (10) | 88% | 56% | 72% | 22 of 128 |
| game 2220 (within 30) | 55 | 4 (6) | 99% | 95% | 62% | 18 of 126 |
| game 3020 (within 30) | 373 | 6 (11) | 91% | 49% | 88% | 29 of 163 |
| game 3024 (within 30) | 344 | 6 (11) | 92% | 53% | 87% | 28 of 164 |

So the near-optimal completions form a cloud around the best, not other structures.  Each is the best with two to
sixteen of its members exchanged for twins, two thirds or more of the twin pairs in paired exchanges.  The moves come
from a small pool: a seventh to a fifth of the best's members ever move.  Most are in the front homes: of the members
swapped out, 52% are of home 1 and 32% of home 2 at 3300-14, 49-70% and 21-36% at the games.

The primes exchanged are close pairs, and the cofactors small.  The pairs met most are 61 ↔ 67, 47 ↔ 53, 197 ↔ 199,
59 ↔ 61 and 83 ↔ 89 at 3300-14; 137 ↔ 139, 41 ↔ 43 and 83 ↔ 89 at 2255; 109 ↔ 113, 79 ↔ 83 and 179 ↔ 181 at 3020 and
3024.  The commonest move at 3300-14 is the paired exchange of 61 ↔ 67 on 24 and 20: 1608 = 24·67 out for 1464, and
1220 = 20·61 out for 1340, in 38% of the completions.  The next are 47 ↔ 53 on 21 and 20 (1113 out for 987, 940 out
for 1060, 37%) and 197 ↔ 199 on 8 and 6 (1592 out for 1576, 1182 out for 1194, 32-36%).  The bound's cheap
exclusions in the tree of 3300-14 are the same moves: the two met most there leave out 1220 and 1608, with 1340 and
1464 coming in, 5,765 and 5,256 times ([local exchanges](../local-prime-exchanges-at-the-nodes-2026-10/README.md)).  A
completion one swap from the best is rare: 14 at 3300-14 (priced 8 to 48), 3 to 5 at the games.

The clouds of 3020 and 3024 are nearly the same: the same moves, 374 and 345 games within 30.  So are their proofs from
the optimum, 35,857 and 29,994 steps, though the chain's searches of them took 317,361 and 42,927 nodes.  That pair's
contrast in the cost model lies in the chain's run, not in the problem.

### 3. Hard against easy at the same gap: the arithmetic does not tell them apart

For each problem, its first heaviest set (the GF(2) bound's, at the unit's node) against its candidates
(`analysis.txt`, section 3):

| problem | proof's steps | heaviest set | twins of a member in its home (median) | members with a twin under 50 below | cheap twin swaps | cheap paired exchanges | members in a cheap move | 7-smooth members |
|---|---|---|---|---|---|---|---|---|
| 3300-14 | 1,248,609 | 165 | 14 | 13% | 18 | 92 | 40% | 13% |
| 3441-1262 | 1,099,410 | 181 | 13 | 10% | 16 | 94 | 35% | 12% |
| 3441-5459 | 52,855 | 147 | 14 | 10% | 15 | 58 | 37% | 11% |
| 3425-149 | 253,336 | 139 | 15 | 14% | 16 | 85 | 45% | 12% |
| 3425-150 | 48,615 | 138 | 15 | 14% | 16 | 84 | 45% | 10% |
| game 2255 | 561,092 | 128 | 11 | 16% | 15 | 64 | 47% | 15% |
| game 2220 | 41,776 | 126 | 12 | 17% | 16 | 62 | 48% | 15% |
| game 3020 | 35,857 | 163 | 13 | 12% | 13 | 49 | 33% | 13% |
| game 3024 | 29,994 | 164 | 13 | 12% | 13 | 49 | 32% | 13% |

Every problem's heaviest set has 11-15 twins a member in its home, 13-18 cheap twin swaps and 49-94 cheap paired
exchanges; a third to a half of its members can make a cheap move.  The hard and the easy problem of a pair do not
differ: 2255 and 2220 have 15 and 16 cheap swaps and 64 and 62 cheap exchanges, and 3425-149 and 3425-150 have 16
cheap swaps each and 85 and 84 cheap exchanges.  Only 3441-5459 has fewer cheap exchanges than its hard partner (58
against 94), but 3425-150, as easy, has 84.  The share of 7-smooth members does not differ either (10-15%).

The anatomy's cores, the minimal conflicts of the first heaviest set, tell the same.  Their members have a median 14,
15 and 12 twins in their homes at 3300-14, 3441-5459 and 3441-1262, and 7, 5 and 5 of them a twin under 50 below; the
cores of all three lie in homes 1-4.  This agrees with A4, where 19 features of the root explained none of the cost
model's residual.

## Conclusion

**The trees are far above what an exact method must examine.**  A leaf of a proof can hold a completion of value v only
if its bound errs there by less than the need less v.  Near the optimum, where a proof must tell completions apart,
four of five units hold nothing within 100 points, and their trees of 0.05 to 1.1 million steps are all the bound's
error.  At 3300-14 the cloud is real: a bound that errs by 92 points or more must give each of its 4,427 or more
completions within 91 of the optimum a leaf of its own, at least a twenty-fourth of the tree's 104,400 leaves.  At the
games the trees do not follow the clouds: 2255 spends 5,500 steps a game within 30 of the optimum, 2220 750.  So the
hardness is not intrinsic in the sense of the question, and a better bound could exist.

**What makes a search hard is not a count of the numbers' arithmetic.**  The moves behind both the near-optimal clouds
and the cheap exclusions are twin swaps of close primes on small cofactors, mostly in paired exchanges.  The front
homes hold most of them.  Every problem has dozens at its root, hard or easy.  What differs is how they combine along a
tree: the anatomy measured cheap exclusions at a fifth of the exclusions in the hard units and 3.7% in the contrast.
That is a property of the bound's heaviest sets along the paths, not of the root's numbers.

## Ideas

1. **Conflicts closed under paired exchanges.**  The bound's cheap exclusions are paired exchanges of its heaviest set,
   and so are the near-optimal completions.  If a conflict K of a node usually stays a conflict when a cheap paired
   exchange is applied to it, the search could learn K's images with K, as nogoods or as one cut on the class.  Then
   the heaviest set could not escape K by a cheap exchange, and the cheap exclusions, which add depth for under 50
   points each, would go.  *Cheap test*: with a hook like `wall exchanges`' (`StepHook::excluding`), at each exclusion
   of 3300-14 and 3441-1262 whose substitution is a paired exchange, apply the exchange to the node's conflict and ask
   `FrameGraph::conflict_using` whether the image is a conflict there.  The share that is, and the steps under those
   exclusions, bound what the idea could save; one hooked solve of each unit, about 12 CPU minutes.
2. **Separate the problem from the run in A4's residuals.**  The chain's search of 3020 took 7.4 times the nodes of
   3024's at the same gap, but their one-thread proofs from the optimum are 1.2 times apart and their clouds the same;
   2255 and 2220 stay 13 times apart from the optimum.  Part of A4's runs of dear games may be the chain's path (its
   start incumbent, its threads), which a better start removes, not the problem.  *Cheap test*: one-thread proofs from
   the optimum (`wall nodes --mode opt`) for the 20 games of 2176-3312 with the largest residuals of the cost model and
   the 20 with the smallest, about an hour of one thread; if the residuals shrink by half or more, seed the chain with
   the finder's game first.

## Files

* `runs.jsonl`: the lines of `wall near` (`run` `count` or `estimate`) and the fields n, nodes, completed, seconds,
  root_bound, opt and gap_opt of `wall nodes --mode opt` (`run` `proof`).  The fields of `wall near` are in the module
  comment of [examples/wall/near.rs](../../../frame/examples/wall/near.rs).  From `solver/frame`, after `cargo build -j 2
  --profile test --example wall --features studies`:
  `target/debug/examples/wall near --suite ../notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl
  --known ../../results/strict.jsonl --ids ID --within D [--nodes K] --sets DIR N` for a unit, and `--root` in place
  of `--suite` and `--ids` for a whole game; `--probes K --probe-gap G` for the estimates; `wall nodes --known
  ../../results/strict.jsonl --mode opt --nodes 5000000 N` for the proofs.  The units were counted within 91 and 41
  (3300-14, `--nodes 6000000` and `3000000`), 100 (3441-1262, 3425-149 and 3425-150) and 0 and 200 (3441-5459), the
  games within 30.  The lines of 3441-5459 and the estimates were copied from the terminal; the two marked `build`
  `first` are the first build's plain enumeration and random paths (its count line has `nodes` for the children
  made).  The proofs' lines keep only the fields above.
* `sets/`: the completions counted, ID.sets.tsv for each problem, as `--sets` writes them.  The first counts (3300-14,
  3441-1262 and 3441-5459) come from a build that wrote a weight column too, always 1; `analyze.py` reads both.
* `candidates/`: each game's candidates with their home frames and maximal factors, as `--sets` writes them.
* `analyze.py` makes the tables (`python3 -I analyze.py runs.jsonl sets candidates
  ../the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl`), and `analysis.txt` is its output.

Commits: the commit of this entry (`StepHook::counting` and `wall near`, with its test).
