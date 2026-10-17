# Keeping and forgetting nogoods at the end of the chain (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  Each searcher keeps its own table of nogoods, looks a node's conflict up by scanning it, and clears it after
every 50,000 of its own steps ([forgetting](forgetting-nogoods-2026-10.md), measured at 1024-1225 on the matching path).
Nogood work is under 1% of a node's time at the end of the chain (the profile in
[bench/chain_end](../../bench/chain_end/README.md)), so the question is not what the table costs but whether keeping more,
or keeping better, makes the trees smaller.  Every policy is sound (the table is a cache of nogoods that hold at every
node), so a policy changes the trees and never the optimum.

*Status.*  **Partial.**  The knobs and the counters are done and kept (defaults unchanged); the sweep below is six games
under twenty policies and four more under eight, all on one thread.  The runs on games of 4-10 million nodes that the
plan asked for if a policy gained 5% were stopped before one ended, because Brian moved the sweep to the cloud machine,
where the tuning campaign (`bench/tuning/`) runs these policies on the suite of subtrees of the dearest games and six
whole games.  That campaign is where the sweep continues.

*What was built* (commit ea0eda6; DESIGN.md section 4, "Forgetting nogoods", and section 5).  Three tactics, set by
`--tactic` like the others and by `examples/ladder.rs`, none of them recorded in a checkpoint (the run that resumes says
which):

| tactic | default | values | what it does |
|---|---|---|---|
| `nogood_period` | 50000 | a whole number from 1 | steps of a searcher between its forgettings; `forget_nogoods=off` is never |
| `nogood_keep` | none | `none`, `hit`, `size2` (`sizeN`), `top64` (`topN`) | what a forgetting keeps: nothing; the nogoods used since the last forgetting; those of at most N members; the N most used since then (only those used at all, the most recently used first among equals) |
| `nogood_limit` | 0 | a whole number, 0 for none | the table holds at most this many nogoods, the least recently used dropped to make room for a new one (a rebuild of the table, so cheap only for small limits) |

A nogood is *used* when a lookup answers with it or it rules a candidate out of a child.  `nogood_limit` is independent
of the period: with a period (`forget_nogoods=on`) the table is limited between forgettings and forgotten at each
period, and with `forget_nogoods=off` the limit is the only way anything goes (`forget_nogoods=off,nogood_limit=256` in
the tables).  The counters are in `model::NogoodStats`, summed over the searchers and printed by the ladder's line and by
the engine's progress lines (published at the end of every unit and every 32,768 steps of a searcher, so a line lags):
`nogoods learned L lookups K hit P% ruled out R forgot F (table T kept C) evicted E largest M`, that is nogoods added,
lookups of a table that held one and the share answered, candidates ruled out of children, forgettings with the mean
table before and the mean kept, nogoods evicted by the limit, and the largest table.  With the defaults every tree is
what it was: the probe's fingerprints (7e8e724717d26545, 5b83bbebf0bb2899 with `lagrange=off`, f3deaac0dbdc030a with
`gf2=on`), the tripwires and `taxman regress --to 1000 --threads 1` unchanged, and every default run below has the node
count of the build before the knobs.  Tests: the table's answers against their definition under every policy, each
policy's uses and evictions, every policy giving the golden games' optimum (also in the soundness harness), the
parsing, and the header ignoring the policy.

*How it was measured.*  The ladder (`--seed-opt --known results/strict.jsonl --threads 1`), so that a search only
proves its optimum and the nodes are exact.  Six games of 2211-3260 whose recorded proofs (64 threads) took 1.0-1.6
million nodes and are `exhausted` (a proof at the ceiling would be no search): 2211, 2250, 2862, 2910, 3050, 3260; one
thread needs 0.4-0.8 million for them.  Four more, 2316, 2625, 3135, 3228 (1.3-3.1 million recorded, 1.0-2.0 million on one thread), for eight
policies.  The
data, the scripts and the tables are in [bench/nogoods](../../bench/nogoods/README.md).  CPU seconds are the process's,
on a laptop others shared (load 6 to 200), and the later runs went eight at a time (about 8% fewer instructions per CPU
second), so nodes are the measure and the CPU figures are good to a few percent.

*Results, six games* (nodes summed; the policy's tactics in the first column):

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

Ten games, the eight policies that were run on all of them (2625, 3135, 3228, 2316 added; nodes 9.6 million at the
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

*What the numbers say.*

* **No policy keeps the trees smaller than clearing everything every 50,000 steps would.**  Everything that keeps more
  or longer makes them larger: a longer period (+19% at 200,000, +23% at a million or never on six games), keeping the
  nogoods used (+31%), the 256 most used (+31%) or the nogoods of at most 16 members (+17%), and every limit (+12% at
  64, +27% at 1024, +33% at 256).  Of the ten games, keeping the nogoods used makes 9 larger (by up to 68%) and never
  forgetting 8 (by up to 87%), one of them hugely: 3228 with no forgetting took 7.10 million nodes, 6.9 times the
  default's 1.03 million, from a table that never held more than 1,180 nogoods.
* **Shorter periods are the same within noise.**  On the first six games every period from 10,000 to 40,000 was 2.5-8.7%
  below the default in nodes (three of the four by more than 5%); on ten games the three that were run on all of them
  are 0.2-2.7% below in nodes and 2.5-5.0% in the geometric mean, with standard errors of 3.4-4.4%.  The tree is a
  chaotic function of the policy: a period of 62,500 instead of 50,000 changes a game's tree by −17% to +25%, and the
  games' ratios have a standard deviation of about 12-14%, so six games cannot see a gain under about 12% and ten
  cannot see one under about 8%.  The default is itself one such draw, so every period compared with it on the same
  games inherits its luck: the four added games took the geometric means of 10,000, 20,000 and 40,000 from −9.4%,
  −4.8% and −8.5% to −5.0%, −4.2% and −2.5%.  If a shorter period gains anything, it is a few percent.
* **Nogoods of at most 4 members do not occur.**  `size2`, `size3` and `size4` give the default's trees to the node: at
  no forgetting did the table hold one.  The nogoods of this search are large: 5% of the table has at most 8 members
  (20 of 424) and 28% at most 16 (117 of 421), and keeping the short ones (+7%, +17%) makes the trees larger, as it
  did on the matching path ([IDEAS 3](../IDEAS.md#3-nogoods-with-very-many-threads)).  (Means over the forgettings
  of the six games; the table averages 424 nogoods.)
* **The counters say why the table does not pay, and what it does not cost.**  With no forgetting 5,379 nogoods were
  learned on the six games against 34,463 at the default: each is learned 6.4 times over, which by the notebook's
  50-90 thousand cycles for a conflict is about 0.1% of the cycles, and a table of 436 nogoods on average (largest
  1,137) costs a node nothing visible: CPU 171-192 µs a node at every policy on the first six games, no trend with the
  table, and 2625, whose table reaches 4,786 nogoods without forgetting (1,606 at the default), took 212 µs a node
  against 219.  The bigger table answers more lookups (98.7% at the default, 99.9% never forgetting) and rules out
  more candidates (629 thousand against 856 thousand), yet the trees grow by a fifth: a stale nogood may be the
  smallest conflict the table knows without being the conflict of the node's own heaviest set, and the branching it
  gives is then worse.  That is a reading, not a measurement; what the data show is the association of a bigger table
  with a bigger tree.
* **Time follows nodes.**  CPU seconds and cycles move with the nodes, within about six points of them in every total (the
  load of the machine and the eight-at-a-time runs account for the rest).

*Which policies deserve a test at 64 threads.*  None of the keeping policies.  If the campaign tries one, it is a
period near 20,000-40,000, for which the data show no loss and a gain too small for ten games to see (the idea of
[IDEAS 3](../IDEAS.md#3-nogoods-with-very-many-threads) that a period of about 30,000 might suit 192 threads is not
refuted); `nogood_period` is the only knob that has a chance, and at 64 threads each searcher's table is smaller than
here, which can only shrink the differences between policies.

*Commits.*  ea0eda6 (the knobs, the counters, the tests), the merges of master that followed, and the commit of this
entry with `bench/nogoods/`.

## Later (2026-10): the tuning campaign's runs

The sweep went on in the [tuning campaign](the-tuning-campaign-at-the-end-of-the-chain-2026-10.md), on the suite of
subtrees of 3300, 3425, 3440 and 3441 and six whole games, one thread each.  It agrees with this entry: against the
default (9 problems, stage A) the periods 5,000, 10,000, 20,000, 30,000 and 200,000 took 0.998, 1.007, 0.970, 0.996 and
1.036 of the CPU and 0.919, 0.959, 0.950, 0.984 and 1.055 of the nodes, `nogood_keep=top64` 1.131 and 1.168 and
`nogood_limit=256` 1.096 and 1.098; and on top of `force_window=all` (16 problems) the periods 5,000, 10,000 and 20,000
took 1.036, 1.044 and 1.018 of `all`'s CPU.  No policy was adopted.
