# The tuning campaign at the end of the chain (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

*Question.*  The search's settings (the tactics `force_window`, `branching`, `strong_limit`, `strong_depth`,
`deep_branching`, `more_forced`, `lagrange`, `symmetry` and the nogood knobs of
[the last entry](keeping-and-forgetting-nogoods-at-the-end-of-the-chain-2026-10.md)) were set on games near n = 1000,
where a root's gap is 130 to 300.  The end of the chain (n above 3,400) has gaps over 1,000 and trees of billions of
nodes.  Which setting makes the search there cheapest, and does it hold for a whole proof on 64 threads, not only for
a piece of one on a single thread?

*Status.*  Done.  One setting changed: **`force_window` is `all` by default** (it was 80; commit 5d9c243).  Nothing
else was adopted; the rest of the findings are below.

*How it was run* (2026-10-16, one `c8g.16xlarge`, 64 Graviton4 threads, on spot at $0.68 an hour; Brian approved up to
$10; about 3.8 hours of the machine, the campaign's logs running from 23:55 to 03:35 UTC; about $2.50).  The build of
c70f8f1, `-C target-cpu=native`; the data, the scripts and `tables.py`, which recomputes every table here, are in
[bench/tuning/campaign-2026-10](../../bench/tuning/campaign-2026-10/README.md).

* *The suite* ([bench/tuning](../../bench/tuning/README.md)): 46 problems solved to completion on one thread, each from
  the best set known (the game's optimum, so that a solve proves it, but for the subtrees of 3441, which is
  unfinished): 40 subtrees of the most expensive searches (10 each of 3441, 3440, 3425 and 3300, with 102,571 to
  2,605,379 nodes at the campaign's default, `force_window=80`) and 6 whole games of 3128-3275 (500,530 to 608,203
  nodes).  At that default, 36.2 million nodes, 7,254 CPU seconds on this machine (7,160 on the M1 Max the suite was
  made on).  A subtree is not a whole search: a searcher's nogoods carried from unit to unit are not measured, nor
  the loss of a unit split among 64 threads.
* *The driver* (`tune.py`): 64 jobs at a time, one thread each, CPU time of the solving thread, a job cut off at ten
  times the default's nodes (none was), the report's geometric means of the CPU seconds and of the nodes against the
  default over the problems both ran.  Successive halving: rung 0 runs every setting on a random third of the suite
  (16 problems: 5 of 3300, 3 of 3425, 3 of 3440, 4 of 3441 and 1 whole game), rung 1 the best third on two thirds (31:
  7, 7, 7, 6 and 4), rung 2 the best third of those on all 46.
* *Stage A*: 33 settings and the default (544 jobs planned), through rung 0, the dearest problems first.  Stopped by
  hand after 53 minutes (23:55 to 00:49 UTC), once the forcing window's result was clear: 269 jobs had ended, the
  default and the six widest windows on 10 problems, most settings on 9, the very slow ones on 4 to 8 (a setting 5 to
  6 times dearer finishes fewer).
* *Stage B*, on top of `force_window=all`: 11 settings (the best of the nogood knobs, `strong_limit`, `more_forced`,
  `lagrange` and the windows 240 and 400, alone and together; not the settings stage A showed 4 to 6 times worse),
  192 jobs in rung 0, 75 in rung 1 (the best four on 31 problems), 45 in rung 2 (the best two on all 46); 312 jobs,
  about 23 minutes.  A first list of 16 settings, which still held cheaper branching at depth, was started and
  stopped after about a minute, and overlapped this one for that minute (128 jobs on 64 cores), which may inflate a
  few early CPU times and moves no node.
* *Stage C*: complete proofs on 64 threads (`ladder --threads 64 --seed-opt`, an hour's limit each) of seven games,
  3232, 3360, 3388, 3179, 3328, 3395 and 3300, each seeded with its optimum, one run each of four settings in turn:
  the default, `force_window=all`, with `strong_limit=4`, and with `strong_limit=4` and `more_forced=off`; 2 hours 21
  minutes (01:14 to 03:35 UTC).

*Stage A* (against the default, `force_window=80`, over the problems both ran; below 1 is better; no job was cut off
and no best sum differed):

| setting | problems | CPU | nodes | worst ratio |
|---|---|---|---|---|
| `force_window=all` | 10 | 0.565 | 0.410 | 0.99 |
| `force_window=400` | 10 | 0.600 | 0.437 | 0.99 |
| `force_window=240` | 10 | 0.613 | 0.491 | 1.03 |
| `force_window=160` | 10 | 0.678 | 0.601 | 0.88 |
| `force_window=120` | 10 | 0.764 | 0.728 | 1.00 |
| `nogood_period=20000` | 9 | 0.970 | 0.950 | 1.19 |
| `strong_limit=8` | 9 | 0.980 | 0.984 | 1.17 |
| `nogood_period=30000` | 9 | 0.996 | 0.984 | 1.16 |
| `nogood_period=5000` | 9 | 0.998 | 0.919 | 1.56 |
| the default | 10 | 1.000 | 1.000 | 1.00 |
| `nogood_period=10000` | 9 | 1.007 | 0.959 | 1.48 |
| `strong_limit=4` | 9 | 1.029 | 1.062 | 1.27 |
| `symmetry=off` | 9 | 1.035 | 1.060 | 1.45 |
| `nogood_period=200000` | 9 | 1.036 | 1.055 | 1.65 |
| `more_forced=off` | 9 | 1.042 | 1.110 | 1.23 |
| `lagrange=on` | 9 | 1.043 | 1.000 | 1.05 |
| `nogood_limit=256` | 9 | 1.096 | 1.098 | 1.61 |
| `nogood_keep=top64` | 9 | 1.131 | 1.168 | 1.70 |
| `force_window=40` | 9 | 1.276 | 1.321 | 1.39 |
| `force_window=100%` | 9 | 1.381 | 1.497 | 2.05 |
| `strong_limit=2` | 8 | 1.482 | 1.676 | 2.28 |
| `force_window=20` | 9 | 1.501 | 1.612 | 1.92 |
| `force_window=50%` | 9 | 1.535 | 1.760 | 2.06 |
| `force_window=25%` | 9 | 1.570 | 1.799 | 1.75 |
| `strong_depth=20`, `deep_branching=forced` | 6 | 4.389 | 2.445 | 7.36 |
| `strong_depth=10`, `deep_branching=forced` | 6 | 4.435 | 2.450 | 7.36 |
| `branching=forced` | 6 | 4.443 | 2.452 | 7.36 |
| `strong_depth=5` | 4 | 5.687 | 2.954 | 7.14 |
| `strong_depth=10` | 4 | 5.701 | 2.954 | 7.15 |
| `strong_depth=40` | 6 | 5.706 | 2.779 | 7.67 |
| `branching=split` | 4 | 5.874 | 3.037 | 7.15 |
| `strong_depth=10`, `deep_branching=largest` | 4 | 6.011 | 3.123 | 8.28 |
| `branching=largest` | 4 | 6.183 | 3.214 | 8.46 |
| `strong_depth=20` | 4 | 6.362 | 3.323 | 7.77 |

The ten problems of the default: 3440-13, 3425-78, 3300-13, 3300-83, 3425-60, 3128 (whole), 3300-11, 3441-571, 3440-90
and 3441-4840, the first ten of rung 0's sixteen in the driver's order (the dearest by the suite's CPU seconds first;
164 to 413 CPU seconds each at the default here).  The "worst ratio" column is the largest ratio of CPU seconds on any
problem the setting ran (the problems are in the driver's report, `out/stageA_report.txt`).

*Stage B* (on top of `force_window=all`; against the default over the problems both ran).  The settings are named as
the campaign named them: `all_` is `force_window=all` and then the setting.  The first column is how far a setting
got: 16 problems is rung 0, 31 rung 1, 46 rung 2.

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

The same problems for the settings that got furthest: on the 31 problems that `all` itself, `all` with
`strong_limit=4` and `all` with `more_forced=off` all ran, they take 0.629, 0.599 and 0.625 of the default's CPU and
0.431, 0.412 and 0.431 of its nodes; `force_window=240` 0.643 and 0.492.  Against `force_window=all` itself, over the
problems both ran:

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

By the source of the problem (CPU and nodes against the default, geometric means; the number of problems in brackets):

| source | `all` | `all`, `strong_limit=4` | `all`, `more_forced=off` |
|---|---|---|---|
| subtrees of 3441 | 0.516 / 0.345 (6) | 0.518 / 0.344 (10) | 0.557 / 0.374 (10) |
| subtrees of 3425 | 0.556 / 0.390 (7) | 0.500 / 0.349 (10) | 0.492 / 0.341 (10) |
| subtrees of 3440 | 0.624 / 0.381 (7) | 0.611 / 0.383 (10) | 0.619 / 0.383 (10) |
| the three together | 0.566 / 0.373 (20) | 0.541 / 0.359 (30) | 0.553 / 0.365 (30) |
| subtrees of 3300 | 0.714 / 0.519 (7) | 0.688 / 0.513 (10) | 0.704 / 0.516 (10) |
| whole games of 3128-3275 | 0.849 / 0.644 (4) | 0.814 / 0.591 (6) | 0.794 / 0.582 (6) |

*Stage C* (64 threads, one run each; search seconds, millions of nodes, thousands of nodes a second):

| game | default | `all` | `all`, `strong_limit=4` | `all`, `strong_limit=4`, `more_forced=off` |
|---|---|---|---|---|
| 3232 | 54.4 s, 16.9 M, 310 k/s | 40.1 s, 10.0 M, 248 k/s | 39.3 s, 9.8 M, 250 k/s | 40.2 s, 9.8 M, 245 k/s |
| 3360 | 124.8 s, 36.4 M, 291 k/s | 103.2 s, 22.9 M, 222 k/s | 112.4 s, 25.1 M, 224 k/s | 116.0 s, 26.0 M, 224 k/s |
| 3388 | 139.1 s, 41.3 M, 297 k/s | 133.3 s, 28.9 M, 217 k/s | 123.6 s, 27.0 M, 218 k/s | 115.9 s, 25.2 M, 217 k/s |
| 3179 | 66.8 s, 25.5 M, 382 k/s | 57.8 s, 16.5 M, 285 k/s | 66.6 s, 19.0 M, 285 k/s | 62.8 s, 17.9 M, 286 k/s |
| 3328 | 238.4 s, 74.2 M, 311 k/s | 188.6 s, 38.2 M, 203 k/s | 164.7 s, 33.7 M, 205 k/s | 159.2 s, 32.5 M, 204 k/s |
| 3395 | 235.5 s, 67.7 M, 288 k/s | 214.0 s, 46.5 M, 217 k/s | 217.4 s, 47.8 M, 220 k/s | 199.2 s, 43.7 M, 219 k/s |
| 3300 | 1,523.3 s, 498.3 M, 327 k/s | 1,270.1 s, 300.4 M, 237 k/s | 1,331.8 s, 316.0 M, 237 k/s | 1,325.6 s, 315.9 M, 238 k/s |
| total | 2,382 s, 760.2 M, 319 k/s | 2,007 s, 463.4 M, 231 k/s | 2,056 s, 478.5 M, 233 k/s | 2,019 s, 471.1 M, 233 k/s |
| against the default | | 0.842, 0.610 | 0.863, 0.629 | 0.847, 0.620 |

Every game ended `DONE` with its seeded optimum as the best sum (so each run is another proof of the committed score),
at 99.8-100% efficiency in every run; the default is the build's, `force_window=80`.

*What the numbers say.*

* **`force_window=all` is never meaningfully worse and usually much better.**  On one thread over the suite it takes
  0.60-0.63 of the default's CPU seconds and 0.41-0.43 of its nodes (0.629 and 0.431 for `all` alone over 31 problems;
  0.601 and 0.414, 0.611 and 0.419 with `strong_limit=4` or `more_forced=off`, over all 46).  No problem is more than
  4% slower (`all` alone: 1.016 on 3213, 1.024 on 3300-39, over 31 problems; with `strong_limit=4` 1.042 at worst
  over 46), and the best is 3.9 times faster (3441-4840; 0.259).  In stage A the wider the window the better, against
  1.0 for 80: 120 0.764, 160 0.678, 240 0.613, 400 0.600, `all` 0.565 of the CPU (0.728 to 0.410 of the nodes).  A
  wider window forces more members, so the trees fall, and a node costs more (stage B: 1.46 times the CPU a node;
  stage C: 319 to 231 thousand nodes a second, 1.38 times), because every member the split leaves free is checked
  exactly; the trees fall by more.
* **The gain is larger the nearer the end of the chain.**  By source, on one thread: subtrees of 3425, 3440 and 3441
  0.49-0.62 of the CPU (`all` alone 0.56, 0.62 and 0.52), subtrees of 3300 0.69-0.71, whole games of 3128-3275
  0.79-0.85; the chain 1..1300, whose games are smaller still, takes 5% fewer nodes and 4.5% more time (a laptop, one
  run each: 276,918 nodes and 24.8 s of search against 290,503 and 23.7 s).  The earlier sweep at 936-1350
  ([cheaper GF(2) nodes](cheaper-gf2-nodes-same-trees-2026-10.md); the source of 80) found no window 6% dearer in
  cycles than 80 (18.0 G against 16.9) and 5% smaller in nodes (85,349 against 89,751): the same shape.  At 64
  threads, whole proofs of 3179-3395 take 0.84 of the time (2,382 s to 2,007 s) and 0.61 of the nodes (760 M to
  463 M), per game 0.74-0.96 of the time and 0.52-0.70 of the nodes; 3300, 64% of the default's time, 0.83 and 0.60.
  That is the whole-proof figure.  The suite's subtrees of 3300 gain more (0.69-0.71) than the proof of 3300 (0.83);
  what the suite does not have, the nogoods that a searcher keeps from unit to unit, a unit split among threads, and
  its choice of subtrees by their gap, may be why, and was not measured.
* **Narrower windows are worse.**  Against 80: 40 1.276 of the CPU and 1.321 of the nodes, 20 1.501 and 1.612, and
  the windows as a percentage of the node's slack 1.381-1.570 and 1.497-1.799 (100%, 50%, 25%).  So a percentage of
  the slack admits far fewer members than 80 does.
* **`strong_limit=4` and `more_forced=off` are within a few percent of `all`, and not adopted.**  On one thread over
  31 problems `strong_limit=4` takes 0.953 of `all`'s CPU and nodes (a worst problem of 1.46) and `more_forced=off`
  0.995 and 0.998; `strong_limit=8` was 1.057 over 16 and `strong_limit=2` 1.482 against 80 in stage A.  At 64
  threads in stage C, one run each, `strong_limit=4` took 1.024 of `all`'s time (2,056 s against 2,007 s; 3.2% more
  nodes) and with `more_forced=off` as well 1.006 (2,019 s; 1.7% more nodes): the one-thread gain of 5% did not
  appear, and the per-game times of these runs differ from `all`'s by -16% to +15%, so one run each cannot resolve a
  few percent.  A 64-thread run is not repeatable to the node.
* **Shorter nogood periods and `lagrange=on` do not pay.**  In stage A (against 80) the periods 5,000, 10,000,
  20,000 and 30,000 took 0.998, 1.007, 0.970 and 0.996 of the CPU (0.919-0.984 of the nodes: the nodes fall by 2% to
  8%, the time does not), 200,000 1.036; `nogood_keep=top64` 1.131 and `nogood_limit=256` 1.096, as the last entry
  expected.  On top of `all` (against `all`, 16 problems) 5,000, 10,000 and 20,000 took 1.036, 1.044 and 1.018 of the
  CPU and 1.059, 1.065 and 1.033 of the nodes; `lagrange=on` 1.036 of the CPU with the nodes identical (1.000: with the
  GF(2) bound it changes no tree and costs 3.6-4.3%); and `symmetry=off`, in stage A, 1.035 of the CPU and 1.060 of the
  nodes, so the rule of symmetric pairs stays.  The worst problems of the periods were 1.7-2.0 times `all`'s: a tree
  is a chaotic function of the policy, as the last entry found.
* **Cheaper branching at depth is 4 to 6 times worse.**  `branching=split`, `largest` or `forced`, and
  `strong_depth` 5, 10, 20 and 40 with `deep_branching` split, forced or largest, took 4.39-6.36 times the default's
  CPU and 2.4-3.3 times its nodes, on the 4 to 6 problems each finished in the time stage A ran; no problem was cut off,
  so these are measured, not bounds.  A node that does not choose its branch by the exact bounds without each member
  of the conflict takes trees too large for the cheaper node to pay.

*What was not measured.*  The nodes of a unit are those of a unit solved alone (no nogoods from the units before,
a whole subtree on one thread); stage C's proofs are the only test of the settings in a whole search on many threads,
and each ran once.  Stage B kept the best third at each rung: `all` alone and `force_window=240` stopped after 31
problems, `force_window=400`, `strong_limit=8`, `lagrange=on` and the nogood knobs after 16, so `all` alone has no
figure for the whole suite; the two settings that ran all 46 stand for it, and differ from it by -5% (`strong_limit=4`)
and -0.5% (`more_forced=off`) on the 31 problems they share.  The subtrees of 3441 have no known optimum (the incumbent
of its checkpoint), so a subtree could hold something heavier; none of the settings found a different best sum on any
problem.  Windows narrower than 80 were tried only as 20, 40 and percentages; nothing is wider than `all`.

*What changed in the solver* (5d9c243): `ForceWindow::default()` is `All`.  The trees of the default change, never the
answers: the node counts of the tripwires for 660, 760, 936 and 1000 (1000: 2,172 to 2,026 nodes; the matching path's
rows and `fix_forced=off`, which has no forced members to search, are the same), the extended problem's, the probe's
fingerprint with `--tactic gf2=on` (f3deaac0dbdc030a to a58ade4629ab6186, which `--tactic gf2=on --tactic
force_window=80` still gives exactly: the window is the only difference; 7e8e724717d26545 and 5b83bbebf0bb2899 with
`lagrange=off`, the matching path's, are unchanged), the probe's two windows (made again, as headers of version 6), the
suite's `default` lines (measured again with `make_suite.py refresh --new-trees`: the same best sum on all 46
problems, 16.1 million nodes against 36.2 and 4,250 CPU seconds against 7,160, 8 jobs at a time on the M1 Max; the
31 problems that the campaign ran with `force_window=all` have the same nodes and best sums on this laptop as on the
Graviton machine; the campaign's data keep the old default).
`taxman regress --to 1000 --threads 1` scores all equal; the one-thread chain 1..1300 has the committed chain's
score, proof, method and `completed` for every game and the committed moves for every game but 1215, whose game of
the committed chain is another optimal set (it is the same set that the build before this change finds on one
thread).  A checkpoint's header of version 5 (and 4) is of a search with the window of 80, which it implies: a search
tuned that way (`--tactic force_window=80`) still writes version 5, the default's, whose window is `all`, writes
version 6, and the default's search resumes a checkpoint of an older build only as `--tactic force_window=80` (3441's
checkpoint, made before the change, is one).

*Commits.*  The campaign ran on c70f8f1; be6a618 (this entry, with the data in `bench/tuning/campaign-2026-10/`);
5d9c243 (the default, the headers, the tripwires, the probe's windows); and the commit after them, with the suite's
`default` lines measured again.

*Open.*  `strong_limit=4` (5% better on one thread, 2% worse at 64 threads, each from one run) wants repeated 64-thread
runs to be decided; so does the nogood period near 20,000, which took 3% off the CPU and 5% off the nodes against 80
in stage A and put 1.8% and 3.3% on against `all` in stage B, a difference of the size that a chaotic tree gives.
Neither is worth a run alone.  The run to the wall is next, with this entry's default and nothing else changed.
