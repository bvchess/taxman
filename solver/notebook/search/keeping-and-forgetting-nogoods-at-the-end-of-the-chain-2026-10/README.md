# Keeping and forgetting nogoods at the end of the chain (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-16.  Concluded 2026-10-16.

**Question.**  Does keeping more nogoods, or keeping better ones, make the trees smaller at the end of the chain?
**Answer.**  No: every policy that keeps more or longer made the trees larger, by 6% to 86%, and no shorter period
gained more than the noise shows (0.2-2.7% fewer nodes on ten games, standard error 3.4-4.4%).  The knobs and counters
are kept, with the defaults unchanged.

## Method

Each searcher keeps its own table of nogoods, looks a node's conflict up by scanning it, and clears it after every
50,000 of its own steps ([forgetting](../forgetting-nogoods-2026-10/README.md), measured at games 1024-1225 on the
matching path).  Nogood work is under 1% of a node's time at the end of the chain ([the calibration for the run to
4000](../../the-calibration-for-the-run-to-4000-2026-10/README.md)), so the question is not what the table costs but
whether keeping more, or keeping better, makes the trees smaller.  Every policy is sound (the table is a cache of
nogoods that hold at every node), so a policy changes the trees and never the optimum.

**The knobs** (commit ea0eda6; [the tactics guide](../../../docs/design/tactics.md)).  Three tactics, set by `--tactic`
and by `examples/ladder.rs`, none recorded in a checkpoint:

| tactic | default | values | what it does |
|---|---|---|---|
| `nogood_period` | 50000 | a whole number from 1 | steps of a searcher between its forgettings; `forget_nogoods=off` is never |
| `nogood_keep` | none | `none`, `hit`, `size2` (`sizeN`), `top64` (`topN`) | what a forgetting keeps: nothing; the nogoods used since the last forgetting; those of at most N members; the N most used since then |
| `nogood_limit` | 0 | a whole number, 0 for none | the table holds at most this many nogoods, the least recently used dropped to make room |

A nogood is *used* when a lookup answers with it or it rules a candidate out of a child.  `nogood_limit` is independent
of the period: with `forget_nogoods=off` the limit is the only way anything goes.  Counters (`model::NogoodStats`) give
the nogoods learned, the lookups and the share answered, the candidates ruled out, the forgettings with the mean table
before and the mean kept, the nogoods evicted, and the largest table; the ladder's line and the engine's progress lines
print them.  With the defaults every tree is what it was (the probe's fingerprints 7e8e724717d26545, 5b83bbebf0bb2899
with `lagrange=off` and f3deaac0dbdc030a with `gf2=on`, the tripwires, `taxman regress --to 1000 --threads 1`), and the
tests check that every policy gives the golden games' optimum.

**The measurement.**  The ladder (`--seed-opt --known results/strict.jsonl --threads 1`), so that a search only proves
its optimum and the nodes are exact.  Six games of 2211-3260 whose recorded proofs (64 threads) took 1.0-1.6 million
nodes and are `exhausted` (a proof at the ceiling would be no search): 2211, 2250, 2862, 2910, 3050 and 3260, which need
0.4-0.8 million on one thread.  Twenty policies ran on them.  Four more games, 2316, 2625, 3135 and 3228 (1.0-2.0
million nodes on one thread), ran under eight policies.  CPU seconds are the process's, on a laptop that others shared
(load 6 to 200), so nodes are the measure and the CPU figures are good to a few percent.

## Results

Six games (nodes summed; the policy's tactics in the first column):

| policy | nodes | against the default | geometric mean of the games, ± s.e. |
|---|---|---|---|
| default (period 50,000, keep nothing) | 3,893,882 | | |
| `nogood_period=10000` | 3,628,599 | −6.8% | −9.4% ± 5.8 |
| `nogood_period=20000` | 3,797,455 | −2.5% | −4.8% ± 5.3 |
| `nogood_period=30000` | 3,598,783 | −7.6% | −9.0% ± 6.5 |
| `nogood_period=40000` | 3,556,287 | −8.7% | −8.5% ± 1.9 |
| `nogood_period=62500` | 4,120,152 | +5.8% | +4.9% ± 5.9 |
| `nogood_period=200000` | 4,626,579 | +18.8% | +19.8% ± 5.6 |
| `nogood_period=1000000` = `forget_nogoods=off` | 4,773,619 | +22.6% | +24.1% ± 7.0 |
| `nogood_keep=hit` | 5,118,381 | +31.4% | +31.3% ± 9.4 |
| `nogood_keep=size2`, `size3`, `size4` | 3,893,882 | 0.0% | 0.0 |
| `nogood_keep=size8` | 4,163,576 | +6.9% | +7.6% ± 2.8 |
| `nogood_keep=size16` | 4,549,147 | +16.8% | +18.7% ± 5.6 |
| `nogood_keep=top64` | 4,199,410 | +7.8% | +6.2% ± 6.6 |
| `nogood_keep=top256` | 5,094,397 | +30.8% | +29.8% ± 10.5 |
| `forget_nogoods=off,nogood_limit=64` | 4,359,452 | +12.0% | +10.1% ± 6.2 |
| `forget_nogoods=off,nogood_limit=256` | 5,173,971 | +32.9% | +28.3% ± 12.2 |
| `forget_nogoods=off,nogood_limit=1024` | 4,925,984 | +26.5% | +28.6% ± 6.6 |

Ten games, the eight policies that ran on all of them (2625, 3135, 3228 and 2316 added; 9.6 million nodes at the
default):

| policy | nodes | against the default | geometric mean, ± s.e. |
|---|---|---|---|
| default | 9,593,184 | | |
| `nogood_period=10000` | 9,526,578 | −0.7% | −5.0% ± 4.4 |
| `nogood_period=20000` | 9,337,062 | −2.7% | −4.2% ± 3.7 |
| `nogood_period=40000` | 9,570,249 | −0.2% | −2.5% ± 3.4 |
| `nogood_period=200000` | 10,760,769 | +12.2% | +15.5% ± 4.4 |
| `forget_nogoods=off` | 17,832,212 | +85.9% | +49.0% ± 18.2 |
| `nogood_keep=hit` | 12,316,958 | +28.4% | +30.3% ± 6.2 |
| `nogood_keep=top64` | 10,725,178 | +11.8% | +9.9% ± 4.5 |

* **Keeping more, or longer, makes the trees larger.**  A longer period costs +19% at 200,000 and +23% at a million or
  never on six games; keeping the nogoods used +31%, the 256 most used +31% and those of at most 16 members +17%; and
  every limit +12% at 64, +27% at 1024 and +33% at 256.  Of the ten games, keeping the nogoods used makes 9 larger (by
  up to 68%) and never forgetting 8 (by up to 87%): game 3228 with no forgetting took 7.10 million nodes, 6.9 times the
  default's 1.03 million, from a table that never held more than 1,180 nogoods.
* **Shorter periods are the same within noise.**  On the first six games every period from 10,000 to 40,000 was 2.5-8.7%
  below the default in nodes; on ten games the three that ran on all of them are 0.2-2.7% below in nodes and 2.5-5.0% in
  the geometric mean, with standard errors of 3.4-4.4%.  The tree is a chaotic function of the policy: a period of
  62,500 instead of 50,000 changes a game's tree by −17% to +25%, and the games' ratios have a standard deviation of
  about 12-14%, so six games cannot see a gain under about 12% and ten cannot see one under about 8%.  The default is
  one such draw too, so every period compared with it inherits its luck.  If a shorter period gains anything, it is a
  few percent.
* **Nogoods of at most 4 members do not occur.**  `size2`, `size3` and `size4` give the default's trees to the node,
  since the table held none.  The nogoods of this search are large: 5% of the table has at most 8 members (20 of 424)
  and 28% at most 16 (117 of 421), and keeping the short ones (+7%, +17%) makes the trees larger, as it did on the
  matching path ([IDEAS 3](../ideas-from-other-fields-the-pilots-2026-10/README.md#3-nogoods-with-very-many-threads)).
* **The table costs nothing visible, and the trees grow.**  With no forgetting 5,379 nogoods were learned on the six
  games against 34,463 at the default: each is learned 6.4 times over, about 0.1% of the cycles.  A table of 436
  nogoods on average (largest 1,137) costs 171-192 µs a node at every policy, with no trend; game 2625, whose table
  reaches 4,786 nogoods without forgetting (1,606 at the default), took 212 µs a node against 219.  The bigger table
  answers more lookups (99.9% never forgetting, 98.7% at the default) and rules out more candidates (856 thousand
  against 629 thousand), yet the trees grow by a fifth.  A reading, not a measurement: a stale nogood may be the
  smallest conflict the table knows without being the conflict of the node's own heaviest set, and the branching it
  gives is then worse.  Time follows nodes, within about six points in every total.

The tuning campaign ran the sweep on, on the suite of subtrees of games 3300, 3425, 3440 and 3441 and six whole games,
one thread each ([the tuning campaign](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md)), and agrees.
Against the default (9 problems, stage A), the periods 5,000, 10,000, 20,000, 30,000 and 200,000 took 0.998, 1.007,
0.970, 0.996 and 1.036 of the CPU and 0.919, 0.959, 0.950, 0.984 and 1.055 of the nodes; `nogood_keep=top64` took 1.131
and 1.168 and `nogood_limit=256` 1.096 and 1.098; and on top of `force_window=all` (16 problems) the periods 5,000,
10,000 and 20,000 took 1.036, 1.044 and 1.018 of `all`'s CPU.

## Decision

No policy was adopted, and the knobs stay as research knobs with the defaults unchanged.  None of the keeping policies
deserves a test at 64 threads.  The one with a chance is `nogood_period` near 20,000-40,000, for which the data show no
loss and a gain too small for ten games to see; the idea of [IDEAS
3](../ideas-from-other-fields-the-pilots-2026-10/README.md#3-nogoods-with-very-many-threads) that a period of about
30,000 might suit 192 threads is not refuted.  At 64 threads each searcher's table is smaller than here, which can only
shrink the differences between policies.  The plan went on to three games whose recorded proofs took 4-10 million nodes
(2260, 3243, 3340) under the default and four periods; the sweep moved to the cloud machine's tuning campaign before one
of those runs ended, so there are none.

## Files

The sweep: games of 2211-3260 proved on one thread by the ladder, seeded with their own optimum, under the default
policy and the others.  A search on one thread is deterministic, so node counts compare exactly; CPU seconds and cycles
are the process's over the search (`getrusage`, `proc_pid_rusage`), so they compare only between runs made under the
same load.  The ladder was `target/release/examples/ladder` of commit ea0eda6; the policies are the `--tactic` specs of
the first column of the tables.  Every `default` run has the node count of the build before the knobs (506,416 at game
2211, 415,764 at 2910, 683,587 at 2250, 761,406 at 3050, 814,937 at 2862), and the merge with the node-speed round
(6460c86) changes none (2910 again: 415,764).  The summary's column of CPU microseconds a node weighs the games by their
nodes, so it moves with which game a policy made large; compare a game with itself.

| file | what it is |
|---|---|
| `run.sh` | the sweep: `nice -n 10 solver/notebook/search/keeping-and-forgetting-nogoods-at-the-end-of-the-chain-2026-10/run.sh OUT "GAME ..." SECONDS POLICY...` from the top of the repository; one line per run appended to OUT as it ends, runs already in OUT skipped, so an interrupted sweep resumes |
| `parallel.sh` | `nice -n 10 solver/notebook/search/keeping-and-forgetting-nogoods-at-the-end-of-the-chain-2026-10/parallel.sh JOBS 8`: a file of `run.sh` arguments, one line a run, eight at a time |
| `summarize.py` | `summarize.py RESULTS [--games=G,...] [--policies=P,...]`: nodes by game and policy, each policy against the default by game with the geometric mean and its standard error, the totals with the CPU time of a node, and the counters of the tables |
| `sweep.txt` | 152 runs: six games (2211, 2910, 2250, 3050, 2862, 3260) under 20 policies, four more (2625, 3135, 3228, 2316) under 8; each line is `GAME<TAB>POLICY<TAB>the ladder's line` |
| `sweep.md` | `summarize.py sweep.txt`, for the six games and for the ten games of the eight policies run on all of them |

## Commits

* ea0eda6 (2026-10-16): the knobs, the counters and the tests.
* ac706cc (2026-10-16): this entry and the data of the sweep.
* The merges of master that followed, among them 6460c86, the node-speed round.
