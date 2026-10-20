# The tuning campaign at the end of the chain (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-16.  Concluded 2026-10-16.

**Question.**  The search's settings were set on games near n = 1000, where a root's gap is 130 to 300.  The end of the
chain (n above 3,400) has gaps over 1,000 and trees of billions of nodes.  Which setting makes the search there
cheapest, and does it hold for a whole proof on 64 threads, not only for a piece of one on a single thread?
**Answer.**  Only `force_window`: `all` takes 0.60-0.63 of the default's CPU and 0.41-0.43 of its nodes on one thread,
and 0.84 of the time and 0.61 of the nodes in whole proofs on 64 threads, so it became the default (it was 80).
Nothing else paid.

## Method

A setting's effect on a tree can only be measured by searches run to completion, and a game at the end of the chain is
far too big for that.  The benchmark is made of subtrees of real searches, solved on one thread, where the nodes of two
settings compare exactly and their CPU times with the noise of the machine only.  The settings were the tactics
`force_window`, `branching`, `strong_limit`, `strong_depth`, `deep_branching`, `more_forced`, `lagrange` and
`symmetry`, and the nogood knobs of [keeping and forgetting
nogoods](../keeping-and-forgetting-nogoods-at-the-end-of-the-chain-2026-10/README.md).

### The suite

`suite.jsonl` has 46 problems.  A *unit* is a node of a search, its promotions and the candidates its loop has still to
try (`search::TaxUnit`); solved alone, it is a search of its own that finishes.  Each problem starts from the best set
known: for 3300, 3425 and 3440 (in `results/strict.jsonl`) the optimum, so that a solve proves the subtree, and for the
unfinished 3441 the checkpoint's incumbent (274,462), which a subtree could beat (none of the 78 subtrees solved in
making the suite did).  A whole game is the proof from the optimum, `ladder --seed-opt` with the setup left out.

| game | checkpoint | units in it | in the suite | nodes | default CPU s |
|---|---|---|---|---|---|
| 3441 | the chain's, 626 million nodes after a 50-minute search on 64 threads (`chain-2176/ckpt` in the project's bucket) | 6,151 | 10 units | 36,500 to 1,099,410 | 9 to 314 |
| 3440 | `taxman solve 3440 --threads 4 --time-limit 120`, from `strict.jsonl` cut to 3439 | 340 | 10 units | 61,285 to 870,739 | 19 to 236 |
| 3425 | the same, cut to 3424 | 253 | 10 units | 33,333 to 721,555 | 9 to 216 |
| 3300 | the same, cut to 3299 | 190 | 10 units | 40,811 to 1,248,609 | 11 to 261 |
| whole games | 3128, 3129, 3213, 3270, 3272, 3275: the smallest searched games of 3000-3440 whose proof from the optimum takes 500,000 to 3 million nodes on one thread (14 were solved to see: 3010 to 3275, with 84,454 to 941,354) | | 6 games | 277,366 to 450,606 | 53 to 219 |

The suite was made at the default `force_window=80`, with units chosen for 100,000 to 3 million nodes at it (102,571 to
2,605,379; the whole games 500,530 to 608,203: 36.2 million nodes and 7,160 CPU seconds in all).  The table's figures
are the new default's, from `make_suite.py refresh --new-trees` after the change: the same best sum on every problem,
16.1 million nodes and 4,250 CPU seconds (8 jobs at once on an M1 Max).  The 31 problems the campaign ran with `all` on
a Graviton machine have the same nodes on this laptop.  The checkpoints of 3300, 3425 and 3440 are not in the
repository (a run on 4 threads is not repeatable); the suite holds the units taken from them.

A unit alone starts with no nogoods, where a searcher keeps them from unit to unit; the suite measures neither that
gain nor the loss of a unit split among 64 threads.

To make it: `tune units GAME CKPT --known results/strict.jsonl --sample 100000 --out units_GAME.jsonl` lists a
checkpoint's units with their depth, candidates, bound against need (`gap`) and both sets in hex; `make_suite.py probe`
solves the default on the units of gap 740 to 1000 and 930 to 1050 (a tree grows about ten times with every 175 of
gap), with node limits 120,000, 400,000, 1.2 million and 3 million in turn, until a game has 12 to 14 in range
(`--wholes` names the whole games; `make_suite.py candidates` lists them by nodes); `make_suite.py assemble --count 40`
picks 10 units a game that finished with 100,000 to 3 million nodes, spread over that range on a log scale, and the six
smallest whole games; `make_suite.py refresh` puts a later build's default CPU seconds into the suite and refuses a
build whose trees are not the suite's (`--new-trees` also takes new nodes, and still refuses a different best sum).

### The driver

```text
cargo build --release --features studies --example tune     # in solver/frame
python3 -I solver/notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/tune.py --configs CONFIGS.json --results RESULTS.jsonl [--workers P] [--nice N]
python3 -I solver/notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/tune.py --configs CONFIGS.json --results RESULTS.jsonl --report
```

`CONFIGS.json` names lists of settings, `NAME=VALUE` as `--tactic` takes them (`configs.example.json`); `default` is
added and is the yardstick.  A job is one configuration on one problem, one `tune solve` process on one thread, `P` at a
time; its result goes to the results file as it ends, and a pair already in the file is not run again, so an interrupted
run starts again with the same command.  A job stops at 10 times the default's nodes (none did).  A job is

```text
tune solve 3441 --known results/strict.jsonl --ids 3441-1234 --tactic lagrange=on \
    --units solver/notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/suite.jsonl
```

(`--ids root` solves the whole game).

Successive halving: rung 0 runs every setting on a random third of the suite and keeps the best third by the geometric
mean of CPU seconds against the default, rung 1 runs those on two thirds, rung 2 the best third of them on all 46.  The
report gives, per setting, the geometric means of CPU seconds and of nodes against the default over the problems both
ran (below 1 is better), the worst problem, and the jobs cut off; it flags a best sum that differs from the default's
(a sound setting changes the tree, not the answer, except `symmetry` on 3441's units, which have no known optimum).
Comparisons are between runs at the same concurrency, since a thread's CPU time grows with its neighbours' memory
traffic and the nodes do not.

### The campaign

On 2026-10-16, on one `c8g.16xlarge` (64 Graviton4 threads) on spot at $0.68 an hour, the build of c70f8f1 with `-C
target-cpu=native`: about 3.8 hours of the machine (23:55 to 03:35 UTC), about $2.50 of the $10 the project's owner
approved.

* **Stage A:** 33 settings and the default, rung 0 only (16 problems), the dearest first.  Stopped by hand after 53
  minutes, once the forcing window's result was clear: 269 of 544 planned jobs had ended, the default and the six
  widest windows on 10 problems, most settings on 9, the very slow ones on 4 to 8.
* **Stage B**, on top of `force_window=all`: 11 settings (the best nogood knobs, `strong_limit`, `more_forced`,
  `lagrange`, and the windows 240 and 400, alone and together), 312 jobs through all three rungs (31 problems at rung 1,
  46 at rung 2), 23 minutes.
* **Stage C:** complete proofs on 64 threads (`ladder --threads 64 --seed-opt`, an hour's limit each) of seven games,
  3232, 3360, 3388, 3179, 3328, 3395 and 3300, each seeded with its optimum, one run each of four settings: the default,
  `force_window=all`, with `strong_limit=4`, and with `strong_limit=4` and `more_forced=off`; 2 hours 21 minutes.

## Results

**Stage A**, against the default (`force_window=80`) over the problems both ran (below 1 is better; no job was cut off
and no best sum differed):

| setting | problems | CPU | nodes |
|---|---|---|---|
| `force_window=all` | 10 | 0.565 | 0.410 |
| `force_window=400` | 10 | 0.600 | 0.437 |
| `force_window=240` | 10 | 0.613 | 0.491 |
| `force_window=160` | 10 | 0.678 | 0.601 |
| `force_window=120` | 10 | 0.764 | 0.728 |
| `nogood_period=20000` | 9 | 0.970 | 0.950 |
| `strong_limit=8` | 9 | 0.980 | 0.984 |
| `nogood_period=30000` | 9 | 0.996 | 0.984 |
| `nogood_period=5000` | 9 | 0.998 | 0.919 |
| the default | 10 | 1.000 | 1.000 |
| `nogood_period=10000` | 9 | 1.007 | 0.959 |
| `strong_limit=4` | 9 | 1.029 | 1.062 |
| `symmetry=off` | 9 | 1.035 | 1.060 |
| `nogood_period=200000` | 9 | 1.036 | 1.055 |
| `more_forced=off` | 9 | 1.042 | 1.110 |
| `lagrange=on` | 9 | 1.043 | 1.000 |
| `nogood_limit=256` | 9 | 1.096 | 1.098 |
| `nogood_keep=top64` | 9 | 1.131 | 1.168 |
| `force_window=40` | 9 | 1.276 | 1.321 |
| `force_window=100%` | 9 | 1.381 | 1.497 |
| `strong_limit=2` | 8 | 1.482 | 1.676 |
| `force_window=20` | 9 | 1.501 | 1.612 |
| `force_window=50%` | 9 | 1.535 | 1.760 |
| `force_window=25%` | 9 | 1.570 | 1.799 |
| `strong_depth=20`, `deep_branching=forced` | 6 | 4.389 | 2.445 |
| `strong_depth=10`, `deep_branching=forced` | 6 | 4.435 | 2.450 |
| `branching=forced` | 6 | 4.443 | 2.452 |
| `strong_depth=5` | 4 | 5.687 | 2.954 |
| `strong_depth=10` | 4 | 5.701 | 2.954 |
| `strong_depth=40` | 6 | 5.706 | 2.779 |
| `branching=split` | 4 | 5.874 | 3.037 |
| `strong_depth=10`, `deep_branching=largest` | 4 | 6.011 | 3.123 |
| `branching=largest` | 4 | 6.183 | 3.214 |
| `strong_depth=20` | 4 | 6.362 | 3.323 |

The default's ten problems are the dearest of rung 0 (164 to 413 CPU seconds each).  The worst problem of a setting
took 0.99 of the default's CPU for `force_window=all` and 400, 1.2 to 1.7 times for the nogood knobs, `strong_limit` 4
and 8, `more_forced=off` and `symmetry=off`, 1.9 to 2.3 times for the narrower windows and `strong_limit=2`, and 7.1 to
8.5 times for the cheaper branching.

**Stage B**, on top of `force_window=all` (named `all`), against the default.  The problems column is how far a setting
got: 16 is rung 0, 31 rung 1, 46 rung 2.

| setting | problems | CPU | nodes | worst ratio |
|---|---|---|---|---|
| `all` and `strong_limit=4` | 46 | 0.601 | 0.414 | 1.04 |
| `all` and `more_forced=off` | 46 | 0.611 | 0.419 | 1.05 |
| `all` | 31 | 0.629 | 0.431 | 1.02 |
| `all` and `nogood_period=20000` | 16 | 0.638 | 0.457 | 1.05 |
| `force_window=240` | 31 | 0.643 | 0.492 | 1.45 |
| `all`, `nogood_period=20000` and `strong_limit=8` | 16 | 0.644 | 0.459 | 1.05 |
| `all` and `nogood_period=5000` | 16 | 0.650 | 0.468 | 1.02 |
| `all` and `lagrange=on` | 16 | 0.650 | 0.442 | 1.06 |
| `all` and `nogood_period=10000` | 16 | 0.655 | 0.471 | 1.06 |
| `force_window=400` | 16 | 0.656 | 0.468 | 1.05 |
| `all` and `strong_limit=8` | 16 | 0.663 | 0.468 | 1.07 |

On the 31 problems that `all`, `all` with `strong_limit=4` and `all` with `more_forced=off` all ran, they take 0.629,
0.599 and 0.625 of the default's CPU and 0.431, 0.412 and 0.431 of its nodes.  Against `all` itself:

| setting | problems | CPU | nodes | worst ratio |
|---|---|---|---|---|
| `strong_limit=4` | 31 | 0.953 | 0.954 | 1.46 |
| `more_forced=off` | 31 | 0.995 | 0.998 | 1.00 |
| `nogood_period=20000` | 16 | 1.018 | 1.033 | 1.70 |
| `nogood_period=5000` | 16 | 1.036 | 1.059 | 1.97 |
| `nogood_period=10000` | 16 | 1.044 | 1.065 | 1.88 |
| `lagrange=on` | 16 | 1.036 | 1.000 | 1.05 |
| `strong_limit=8` | 16 | 1.057 | 1.059 | 1.39 |
| `nogood_period=20000`, `strong_limit=8` | 16 | 1.027 | 1.039 | 1.42 |
| `force_window=400` | 16 | 1.046 | 1.059 | 1.35 |
| `force_window=240` | 31 | 1.022 | 1.141 | 1.75 |

By the source of the problems (`all` against the default; geometric means of CPU and of nodes, the problems in
brackets): subtrees of 3441 0.516 / 0.345 (6), of 3425 0.556 / 0.390 (7), of 3440 0.624 / 0.381 (7), the three together
0.566 / 0.373 (20), subtrees of 3300 0.714 / 0.519 (7), whole games of 3128-3275 0.849 / 0.644 (4).  With
`strong_limit=4` or `more_forced=off` over all 46 problems the figures are within 0.07 of these.

**Stage C** (64 threads, one run each; search seconds and millions of nodes; the total row adds thousands of nodes a
second):

| game | default | `all` | `all`, `strong_limit=4` | `all`, `strong_limit=4`, `more_forced=off` |
|---|---|---|---|---|
| 3232 | 54.4 s, 16.9 M | 40.1 s, 10.0 M | 39.3 s, 9.8 M | 40.2 s, 9.8 M |
| 3360 | 124.8 s, 36.4 M | 103.2 s, 22.9 M | 112.4 s, 25.1 M | 116.0 s, 26.0 M |
| 3388 | 139.1 s, 41.3 M | 133.3 s, 28.9 M | 123.6 s, 27.0 M | 115.9 s, 25.2 M |
| 3179 | 66.8 s, 25.5 M | 57.8 s, 16.5 M | 66.6 s, 19.0 M | 62.8 s, 17.9 M |
| 3328 | 238.4 s, 74.2 M | 188.6 s, 38.2 M | 164.7 s, 33.7 M | 159.2 s, 32.5 M |
| 3395 | 235.5 s, 67.7 M | 214.0 s, 46.5 M | 217.4 s, 47.8 M | 199.2 s, 43.7 M |
| 3300 | 1,523.3 s, 498.3 M | 1,270.1 s, 300.4 M | 1,331.8 s, 316.0 M | 1,325.6 s, 315.9 M |
| total | 2,382 s, 760.2 M, 319 k/s | 2,007 s, 463.4 M, 231 k/s | 2,056 s, 478.5 M, 233 k/s | 2,019 s, 471.1 M, 233 k/s |
| against the default | | 0.842, 0.610 | 0.863, 0.629 | 0.847, 0.620 |

Every game ended `DONE` with its seeded optimum as the best sum, so each run is another proof of the committed score,
at 99.8-100% efficiency.

What the numbers say:

* **`force_window=all` is never meaningfully worse and usually much better.**  On one thread no problem is more than 4%
  slower, and the best (3441-4840) is 3.9 times faster (0.259).  The wider the window the better: in stage A 120 took
  0.764 of the CPU, 160 0.678, 240 0.613, 400 0.600 and `all` 0.565.  A wider window forces more members, so a node
  costs more (1.46 times the CPU in stage B; 319 to 231 thousand nodes a second, 1.38 times, in stage C), because every
  member the split leaves free is checked exactly, and the trees fall by more.
* **The gain is larger the nearer the end of the chain.**  On the chain 1..1300 `all` takes 5% fewer nodes and 4.5%
  more time (a laptop, one run each: 276,918 nodes and 24.8 s against 290,503 and 23.7 s).  The earlier sweep at
  936-1350 ([the inner loop of the GF(2) search](../the-inner-loop-of-the-gf2-search-2026-10/README.md), the source of
  80) found the same shape: searching every member took 5% fewer nodes than 80 (85,349 against 89,751) and 6% more
  cycles (18.0 G against 16.9).  At 64 threads, whole proofs of 3179-3395 take 0.84 of the time and 0.61 of the nodes,
  per game 0.74-0.96 and 0.52-0.70; 3300 takes 0.83 and 0.60, where the suite's subtrees of 3300 gain more (0.69-0.71
  of the CPU).  The nogoods a searcher keeps between units, a unit split among threads and the suite's choice of
  subtrees by gap may be why; it was not measured.
* **Narrower windows are worse.**  Against 80, 40 takes 1.276 of the CPU and 1.321 of the nodes, 20 takes 1.501 and
  1.612, and a percentage of the node's slack (100%, 50%, 25%) takes 1.381-1.570 and 1.497-1.799: it admits far fewer
  members than 80 does.
* **`strong_limit=4` and `more_forced=off` are within a few percent of `all`.**  On one thread `strong_limit=4` takes
  0.953 of `all`'s CPU and `more_forced=off` 0.995.  At 64 threads `strong_limit=4` took 1.024 of `all`'s time (3.2%
  more nodes) and, with `more_forced=off`, 1.006 (1.7% more nodes): the one-thread gain of 5% did not appear.  The
  per-game times differ from `all`'s by -16% to +15%, so one run each cannot resolve a few percent.
* **Shorter nogood periods and `lagrange=on` do not pay.**  In stage A the periods took 0.970-1.007 of the CPU and
  0.919-0.984 of the nodes (the nodes fall by 2% to 8%, the time does not), 200,000 took 1.036, `nogood_keep=top64`
  1.131 (nodes 1.17) and `nogood_limit=256` 1.096.  On top of `all` the periods 5,000, 10,000 and 20,000 took 1.036,
  1.044 and 1.018 of the CPU, and their worst problems 1.7-2.0 times `all`'s: a tree is a chaotic function of the
  policy.  `lagrange=on` changes no tree with the GF(2) bound (nodes 1.000) and costs 3.6-4.3%.  `symmetry=off` took
  1.035 of the CPU and 1.060 of the nodes, so the rule of symmetric pairs stays.
* **Cheaper branching at depth is 4 to 6 times worse.**  `branching` split, largest or forced, and `strong_depth` 5 to
  40 with `deep_branching`, took 4.39-6.36 times the default's CPU and 2.4-3.3 times its nodes on the 4 to 6 problems
  each finished in the time stage A ran.  No problem was cut off, so these are measured, not bounds.  A node that does
  not choose its branch by the exact bounds on each member of the conflict takes trees too large for the cheaper node to
  pay.

Not measured: a whole search on many threads has only stage C's proofs, each run once.  Stage B kept the best third at
each rung, so `all` alone has no figure for the whole suite; the two settings that ran all 46 stand for it, and differ
from it by -5% (`strong_limit=4`) and -0.5% (`more_forced=off`) on the 31 problems they share.  The subtrees of 3441
have no known optimum, so a subtree could hold something heavier; no setting found a different best sum on any
problem.  Nothing is wider than `all`.

## Decision

`force_window` is `all` by default (5d9c243).  Nothing else was adopted: the other settings are within the noise of a
chaotic tree, or worse.  The change alters the default's trees and never the answers.  It moved the node counts of the
tripwires for 660, 760, 936 and 1000 (1000: 2,172 to 2,026 nodes), the extended problem's tripwire and the probe's two
windows (made again as headers of version 6), and the probe's fingerprint with `--tactic gf2=on`, f3deaac0dbdc030a to
a58ade4629ab6186 (`--tactic gf2=on --tactic force_window=80` still gives the old one exactly).  The matching path's
fingerprints, 7e8e724717d26545 and 5b83bbebf0bb2899 with `lagrange=off`, and `fix_forced=off`, which has no forced
members to search, do not move.

`taxman regress --to 1000 --threads 1` scores all equal.  The one-thread chain 1..1300 has the committed chain's score,
proof, method and `completed` for every game, and its moves for every game but 1215, whose committed game is another
optimal set (the build before the change finds that set on one thread).  A checkpoint's header of version 5 (and 4)
implies the window of 80: a search tuned that way (`--tactic force_window=80`) still writes version 5, the default's
search, with window `all`, writes version 6, and the default's search resumes a checkpoint of an older build only as
`--tactic force_window=80` (3441's checkpoint is one).

What would reopen it: `strong_limit=4` (5% better on one thread, 2% worse at 64 threads, each from one run) and the
nogood period near 20,000 (3% off the CPU and 5% off the nodes against 80 in stage A, 1.8% and 3.3% on against `all` in
stage B, a difference of the size a chaotic tree gives) want repeated 64-thread runs, and neither is worth a run alone.
The run to the wall used this entry's default and nothing else changed.

## Files

The times in the logs are UTC; the files' times are those of the laptop they were fetched to (UTC-7).  The scripts are
as they ran but for two words in comments (the project's word for the hardest games is no longer "frontier", which the
docs test refuses).  `solver/frame/examples/tune.rs` holds the harness's Rust side: `tune units` samples the units of a
checkpoint, and `tune solve` solves units (or `root`, the whole game) on one thread and prints a line each: nodes, CPU
seconds of the thread, whether it finished and the best sum.

| file | what it is |
|---|---|
| `suite.jsonl` | the suite: a `suite` line (the commit, the ranges, the default's CPU seconds, the concurrency they were measured at), a `game` line for each game (its candidates, the checkpoint's incumbent, the known optimum), the `unit` lines (id `GAME-INDEX`, depth, candidates, the sum of the promotions, bound, need and gap, both sets in hex) and the `whole` lines, each with `default`: the nodes, CPU seconds, wall seconds and best sum of the default's solve.  Its default is the new one, so `tune.py --report` on the campaign's results with it says that every problem's default nodes differ from the suite's; `git show c70f8f1:rust/bench/tuning/suite.jsonl` is the suite the campaign used |
| `tune.py` | the driver and its report |
| `make_suite.py` | makes the suite (`probe`, `assemble`, `refresh`, `candidates`) |
| `test_tune.py` | tests of both scripts, with a stand-in for `tune solve`: `python3 -I solver/notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/test_tune.py` |
| `configs.example.json` | a configurations file |
| `stageA.json` | stage A's configurations: 33 settings, each against the default |
| `stageB.json` | the first list for stage B (16 settings on top of `force_window=all`), run for about a minute and stopped, overlapping the next list for that minute (128 jobs on 64 cores, which may inflate a few early CPU times and moves no node); nothing of it was kept |
| `stageB2.json` | the list stage B ran (11 settings, without those stage A showed far worse) |
| `campaign.sh` | the script of the machine: stage A, then (it waited for them) stage B and stage C from files put in the bucket; its own stage C was replaced by `stageC.sh` |
| `stageB2.sh` | stage B as it ran, started by hand |
| `stageC.sh` | stage C as it ran: complete proofs of seven games on 64 threads for four settings |
| `out/stageA.jsonl` | stage A's results as the driver wrote them, a line a job (269 jobs ended) |
| `out/stageA_report.txt` | the driver's report on them |
| `out/stageB2.jsonl`, `out/stageB2_report.txt` | stage B's results (312 jobs) and report |
| `out/stageC_default.txt`, `stageC_fw_all.txt`, `stageC_all_sl4.txt`, `stageC_all_sl4_mfo.txt` | the ladder's output of stage C for each setting, a line a game |
| `out/stageC.log`, `out/campaign.log` | the machine's logs: the start and end of each run, stage A's report and what stopped |
| `tables.py` | recomputes the tables above from these files: `python3 -I solver/notebook/search/the-tuning-campaign-at-the-end-of-the-chain-2026-10/tables.py` |

A line of a results file is `{"config", "problem", "rung", "settings", "ok", "result": {"nodes", "cpu_s", "wall_s",
"completed", "best", ...}}` (`tune.py` documents them); no job was cut off, and none found another best sum than the
default's.

## Commits

* c70f8f1: the build the campaign ran on (the merge of keeping and forgetting nogoods).
* be6a618: this entry, with the data in this folder.
* 5d9c243: the default `force_window=all`, the headers, the tripwires and the probe's windows.
* 6edac9e: the suite's `default` lines measured again with the new default.
