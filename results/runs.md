# How results/optimal.jsonl.gz was made

Part of [the results](README.md).

It was made in six runs, each a continuation of the one before.  Each run wrote its results to a plain file of its own,
`strict.jsonl` in its working directory, and the games were brought home from there into the committed file, which was
`results/strict.jsonl`, a plain file, in the commits that the table names:

| games | where | build | git has the file before the run as |
|---|---|---|---|
| 1–2175 | one invocation, which nothing stopped, on a laptop | commit feba6d0 | (the earlier files, below) |
| 2176–3440 | appended to a copy of those lines on a cloud machine | commit 55287ff | `git show 4f1c35b:rust/results/strict.jsonl`, whose 2175 lines are this file's first 2175, byte for byte |
| 3441–3914 | appended from a run on a cloud machine of the same kind | commit eb84a6f | `git show b23f497:rust/results/strict.jsonl`, whose 3440 lines are this file's first 3440, byte for byte |
| 3915–4547 | appended from a run on a cloud machine of the same kind, which took the games before from this file and the bounds store (`--use-prev`) | commit ca357bf | `git show c7d81c2:rust/results/strict.jsonl`, whose 3914 lines are this file's first 3914, byte for byte |
| 4548–4874 | appended from a run on a cloud machine of the same kind, which took the games before from this file and the bounds store (`--use-prev`) | commit 4974a71 | `git show f48df13:results/strict.jsonl`, whose 4547 lines are this file's first 4547, byte for byte |
| 4875–5060 | appended from a run on a friend's machine of 88 threads, which took the games before from this file and the bounds store (`--use-prev`) | commits a88dc7f, bcfde26, 551f883, 9b4b308, d45a513, afbc256, d10e27a, c3357cd, 88b1228, 947946a, 29aa8ba, 7140548, ee9d945, 1b59354 and 85dd216 | `git show 18232de:results/strict.jsonl`, whose 4874 lines are this file's first 4874, byte for byte |

Each build was of a clean tree, which every line records.  Every line records the `default` tactics: no tactic is
unproven any more, so none is switched off.  The sixth run's lines come from fifteen builds, one for each stretch of its
games.

**The first run, games 1–2175.**  The command was, up to the paths, `taxman solve 1-2175 --threads 8 --out
strict.jsonl --checkpoint-dir ckpt --checkpoint-interval 60 --progress 0`, run once to the end on an Apple M1 Max
(10 CPUs) with nothing else running.  Every line records the build of commit feba6d0 (`dirty` false), 8 threads, no
`--time-limit` or `--node-limit`, and the `default` tactics: the GF(2) bound, `gf2`, on, and the Lagrangian bound over
the nogoods, `lagrange`, off.  No game was resumed (`invocations` is 1 in every line of 1–2175), and none was built from
a results file or from the table (`prev.source` is `run` from 2 on).  The [entry on the
run](../solver/notebook/the-committed-chain-to-2175-2026-10/README.md) adds what it showed.

*Details.*  The checkpoint interval is not recorded, but the `engine.checkpoints` counts imply about 60 s: 1 each for
2170 and 2172, the only games that searched for more than 60 s.  The run took 1638.6 s of wall time and 10,890 s of CPU
(user time, timed from outside; the lines record each game's own seconds).  The searches add up to 1450.5 s
(`seconds.search_all_runs`) and 48.4 M nodes:

* 28.4 s and 1.08 M nodes for 1–1300: 7.5 s for 1–1000, 11.8 s for 1001–1231 and 9.1 s for 1232–1300;
* 1422.1 s and 47.4 M nodes for 1301–2175: 681.3 s for 1301–1600, 396.8 s for 1601–2000 and 344.1 s for 2001–2175.

The slowest games were 2172 (73.1 s, 1.31 M nodes), 2170 (68.5 s), 1475 (50.1 s, 1.92 M nodes, the most of any game),
1472 (37.8 s) and 1430 (35.0 s); the next is 1521 at 34.3 s.  The queue of work units never held more than 178, and
none spilled to disk.

The cost of the first 1300 games fell with the tactics that came after the earlier chains.  The third earlier chain
took 1022.5 s of search and 332.0 M nodes, on 4 threads of a machine shared with four other jobs, which makes its times
pessimistic; the second took 1052 s, and the first 3359 s.  Three tactics account for it:

* `same_search` concludes 545 of those games as repeats of an earlier game's search ([the
  notebook](../solver/notebook/search/same-search-2026-10/README.md));
* the Lagrangian bound over the nogoods makes the searches that remain shorter ([the
  notebook](../solver/notebook/search/a-lagrangian-bound-over-the-nogoods-2026-10/README.md));
* the GF(2) bound, which this is the first committed chain made with, makes them shorter again ([the
  notebook](../solver/notebook/search/the-gf2-bound-in-the-search-2026-10/README.md)).

**The second run, games 2176–3440.**  The command was `solver/tools/cloud/chain.sh 4095`, which runs `taxman solve
1-4095 --window W --out strict.jsonl --checkpoint-dir ckpt --checkpoint-interval 300 --progress 60` on all the CPUs.  It
ran on a copy of the first run's 2175 lines, so that the first game it solved was 2176, and its 24-hour deadline
stopped it after game 3440 (status 124, with a final checkpoint written).  Every line of 2176–3440 records the build of
commit 55287ff (`dirty` false), 64 threads, no `--time-limit` or `--node-limit`, and the `default` tactics.  The [entry
on the run](../solver/notebook/the-frontier-chain-to-3440-2026-10/README.md) has its story and cost, and [the
RUNBOOK](../solver/docs/RUNBOOK.md) the procedure.

*Details.*  The machine was one `c8g.16xlarge` (64 vCPUs of Graviton4, 124 GiB) on spot in us-east-1d, at $0.6752 an
hour, in two instances.  The first ran from 2026-10-15 04:05:42Z with a window of 16 games in flight (`W`) to game
3310, and was stopped on purpose.  The second, restored from the saved state, ran with a window of 4 from 10:40:41Z to
the deadline.  The run cost about $16.46 in all.

* The window is not in the lines, since a window changes neither a score nor a proof ([the frame solver's
  README](../solver/frame/README.md#games-in-flight)).  `invocations` is: it is 1 in every line of 2176–3440 but 3311,
  3312 and 3315.  Those games were in flight when the first instance stopped, and the second took them up from their
  checkpoints (2 each).
* The game of n − 1 came from the results file, not from the run, for the first game of each instance: `prev.source` is
  `results` for 2176 and 3311.  The lines it read assume nothing, and so do these.
* The 1265 games are 567 searches (448 exhausted, 119 at the ceiling), 543 repeats and 155 primes.  The searches took
  17.73 G nodes, of which 70 games of 30 M nodes or more took 16.34 G.  The dearest were 3440 (3.72 G nodes, 18,184 s or
  5.05 h on 64 threads), 3425 (1.23 G, 3.5 h), 3424 (1.30 G, 2.0 h), 2346 (1.14 G) and 3430 (0.80 G).  The patches of
  dear games are the 2340s, 2375–2388, 3102–3112, 3174–3176, 3300, the 3330s and 3399–3440.
* The search seconds of the 448 exhausted searches rise tenfold per about 1,040 in n, by a least-squares line through
  log10 of `seconds.search_all_runs`: 7 s at 2200, 97 s at 3400.  The scatter around it is about one decade (sd 1.0),
  concentrated in the patches.
* The queue of work units never held more than 7,298, and none spilled to disk.
* The run was left in the middle of game 3441, at 626 million nodes after 5.9 hours, with 3444 also begun.  Their
  checkpoints (format 4) are not part of the results, and the run left no line for either; the third run made them.

**The third run, games 3441–3914.**  The command was `taxman solve` from n = 1, with a window of 4 games in flight on
64 threads of one `c8g.16xlarge` at spot, with the build of commit eb84a6f.  It was launched on 2026-10-17 at 00:15 PDT
and stopped on request on 2026-10-18 at 18:58Z, in the middle of game 3915, which has no line.  It solved games 1–3440
again first, with this file's scores in all (checked), and only its lines from 3441 on went into this file.  So the
lines before them are the second run's, unchanged, and the game of 3440 that line 3441 names (`prev`, by digest) is this
file's.  Every kept line records that build (`dirty` false), 64 threads, one invocation, no `--time-limit` or
`--node-limit`, the `default` tactics, and a `prev.source` of `run`, 3441's too.  The [entry on the
run](../solver/notebook/the-run-to-the-wall-2026-10/README.md) has its story.

*Details.*  The 474 games are 221 searches (178 exhausted, 43 at the ceiling), 193 repeats and 60 primes.  The searches
took 21.28 G nodes and 49.0 hours of game time; the games overlap in the window.  107 games of 30 M nodes or more took
20.65 G of the nodes.  The dearest were 3441 (0.79 G nodes, 9,973 s or 2.77 h), then 3864, 3807, 3885 and 3744 (0.68,
0.68, 0.67 and 0.50 G).  By time, after 3441, they were 3888 (5,271 s) and 3444 (4,687 s).  The queue of work units
never held more than 5,815, and none spilled to disk.

**The fourth run, games 3915–4547.**  The command was `taxman solve 1-8191 --use-prev`, with a window of 4 games in
flight on 64 threads of one `c8g.16xlarge` at spot, with the build of commit ca357bf; `solver/tools/cloud/wall.sh`
launched it ([solver/docs/runbook/wall.md](../solver/docs/runbook/wall.md)).  With `--use-prev` it took games 1–3914
from this file and each game the bounds store had proven from the store, with no search, and searched the others, from
3915 on.  Its own file started with `known` copies of games 1–3914.  The lines of games 3915–4547 were appended to this
file, byte for byte; those copies and the run's three incomplete lines (of games 3940, 4340 and 4548) were left out.
Every kept line records that build (`dirty` false), 64 threads, no `--time-limit` or `--node-limit`, the `default`
tactics and the option `use_prev`.  The first line, 3915, records its game of 3914 as `known`, the run's copy of this
file's line; both verifiers accept that record, because the score, the moves and the assumptions are the line's
([solver/docs/runbook/chain.md, "Verifying, and bringing the results
home"](../solver/docs/runbook/chain.md#verifying-and-bringing-the-results-home)).

*Details.*  The run was launched on 2026-10-18 at 12:27 PDT, continued with `wall.sh launch --continue` when its first
budget of 36 instance-hours ran out, and ended on 2026-10-20 at 05:31 PDT on its budget of 40.5, in the middle of game
4548, which has a checkpoint and no completed line.  Four instances ran the solver, for $27.32: spot interruptions
replaced the first two, and the budget ended the third, which `--continue` took up with a fourth.  A fifth, the first
try of `--continue`, idled for about 25 minutes (about $0.30) on the old layout of the scripts.  [The notebook
entry](../solver/notebook/the-run-to-5000-2026-10/README.md) records the run.

* `invocations` is 1 in every kept line but 3940, 3942, 4340, 4431, 4432 and 4433.  Those games were in flight when an
  instance stopped, and the next took them up from their checkpoints (2 each).  `prev.source` is `results` for 3940,
  4340 and 4431, the first game of each instance after the first, which read the game of n − 1 from the results file,
  and `run` for the others.
* The 633 games are 207 searches (150 exhausted, 57 at the ceiling), 204 repeats, 55 primes and 167 games of the store:
  3984–3999, 4011–4014, 4053–4055, 4080–4094, 4179, 4300–4304, 4323–4331, 4347–4370, 4374–4375, 4446–4529 and
  4540–4543.  Each cites its two lines of the store, from the runs `ledger-frontier-stage1`, `ledger-frontier-stage2`
  and `ledger-survey-stage3`.
* The searches took 29.75 G nodes and 64.5 hours of game time; the games overlap in the window.  115 games of 30 M nodes
  or more took 29.24 G of the nodes.  The dearest by nodes were 3940 (1.55 G nodes, 10,725 s or 2.98 h), 3930, 4225,
  3942 and 3936 (1.40, 1.37, 1.20 and 1.16 G); by time, 3942 (15,942 s or 4.43 h), 4433 (11,032 s), 3940 and 3939
  (9,033 s).  The queue of work units never held more than 7,380, and none spilled to disk.

**The fifth run, games 4548–4874.**  The command was `taxman solve 1-8191 --use-prev`, with a window of 4 games in
flight on 64 threads of one `c8g.16xlarge` at spot, with the build of commit 4974a71, launched by
`solver/tools/cloud/wall.sh`.  It took games 1–4547 from this file and each game the bounds store had proven from the
store, with no search, and searched the others, from 4548 on.  Its own file started with `known` copies of games
1–4547.  The lines of games 4548–4874 were appended to this file, byte for byte, but for 14 lines of method `search`
with no node (4644, 4785, 4794, 4797, 4810, 4816, 4818, 4824, 4833, 4842, 4844, 4845, 4850 and 4869), which the rule of
commit 7987c86 names `start`, as for every line of a build before it.  Those copies and the run's incomplete line (of
game 4875) were left out.  Every kept line records that build (`dirty` false), 64 threads, one invocation, no
`--time-limit` or `--node-limit`, the `default` tactics and the option `use_prev`.  The first line, 4548, records its
game of 4547 as `known`, which both verifiers accept, as for 3915.

*Details.*  The run was launched on 2026-10-20 at 18:50 PDT, and the account's owner stopped it with `wall.sh stop` on
2026-10-21 at 13:42 PDT, in the middle of game 4875, which has a checkpoint and no completed line.  One instance ran the
solver, with no interruption, for 18.86 instance-hours and $13.58 at the ledger's price.  [The notebook
entry](../solver/notebook/the-run-past-5000-2026-10/README.md) records the run.

* `prev.source` is `run` in every kept line.
* The 327 games are 35 searches (all exhausted), 14 proofs at the ceiling (method `start`), 13 primes, 50 repeats and
  215 games of the store: 4550–4640, 4650–4759 and 4770–4783, all that the store had proven in 4548–4783.  Each cites
  its two lines of the store, from the runs `ledger-frontier-stage1` and `ledger-frontier-stage2`.
* The searches took 9.18 G nodes and 14.9 hours of game time; the games overlap in the window.  Every search took 30 M
  nodes or more.  The dearest by nodes were 4864 (1.13 G nodes, 4,636 s), 4865, 4872, 4860 and 4856 (1.01, 0.945, 0.857
  and 0.532 G); by time, 4865 (8,995 s or 2.50 h), 4864, 4872 and 4860.  The queue of work units never held more than
  5,617, and none spilled.

**The sixth run, games 4875–5060.**  The command was `solver/tools/cloud/chain.sh 8191`, which runs `taxman solve 1-8191
--use-prev` with a window of 4 games in flight and [the wall](../docs/GLOSSARY.md#the-search) at 48 hours of a game's
own search (the details below say where the window and the wall changed), on a friend's machine of 88 threads: two Intel
Xeon E5-2699 v4 (x86_64, Ubuntu).  It continued the fifth run's results file and its checkpoints, which were copied from
the cloud machine.  It took games 1–4874 from the file and each game the bounds store had proven from the store, with no
search, and searched the others, from 4875 on.  Its own file started with `known` copies of games 1–4874.  The lines of
games 4875–5060 were appended to this file, byte for byte.  Those copies and the run's incomplete lines (two of game
4875, one each of 4879, 4880, 4895, 4899, 4914, 4921, 4944, 4950, 4953, 4956 and 4960, three of 4984, one each of 4991,
4992, 5037 and 5047, two of 5049, and one each of 5054 and 5061) were left out.  No line was relabeled: every build is
after commit 7987c86, so a search that took no step already says `start`.  Every kept line records that the tree was
clean (`dirty` false), no `--time-limit` or `--node-limit`, the `default` tactics, with the tactic `conflict_branching`
on to 4983, and the option `use_prev`; the lines of 4899 to 5060 record `background` and `finder_seconds` 900 too, and
those of 4914 to 5060 the tactics `conflict_look` and `near_repeat`, on a build that has them.  The lines of 4953 on,
made by builds in which `conflict_look` is on by default, no longer name it in `tactics`, and those of 4960 on record
`price_reuse=on`, which the command switched on.  The lines of 4984 on record `carried_nogoods=on` and `warm_start=on`
too, and `conflict_branching=off` in all but 4991, 4992, 4995, 4998, 5000 and the 30 searched games of 5001–5060, whose
searches the lab switched to conflict branching at a gate of 1200 (below).  The first line, 4875, records its game of
4874 as the line of this file, with its proof, `same_search`, which both verifiers accept.

*Details.*  `chain.sh` ran twenty-three times, from 2026-10-21 to 30 (times UTC).  Each stop was a SIGTERM to
`chain.sh`: the solver wrote a final checkpoint, and the next invocation resumed from it; the last stop ended the run.
The lines come from the second to the thirteenth, the sixteenth to the twentieth, the twenty-second and the
twenty-third (the fourteenth and the fifteenth searched 4984 and wrote only its incomplete lines, and the twenty-first
searched 5049 and wrote only an incomplete line):

1. 21:41 to 01:54: build 88915fb, on every thread (88), with the default tactics of that build, which had no conflict
   branching.  It resumed 4875 from the fifth run's checkpoint and was stopped with 4875 unfinished (the incomplete line
   says 8.54 G nodes over two invocations, the cloud machine's and this one's).  Its checkpoint was set aside, and no
   line of it is kept.
2. 01:54 to 03:10: build a88dc7f, in which `conflict_branching` is on by default, on 88 threads.  It started 4875 afresh
   and exhausted it in 186 M nodes and 3,383 s ([the notebook's
   study](../solver/notebook/search/where-the-search-of-4875-spends-and-which-conflict-to-branch-on-2026-10/README.md)),
   and did 4876–4878 too.
3. 03:10 to 05:20: build a88dc7f on 72 threads: 4879.
4. 05:20 to 12:35: build bcfde26 on 72 threads, with the lab's command folder and hook (`--lab-dir`, `--hook`; [the
   lab's design](../solver/docs/design/lab.md)): 4880–4894.
5. 12:35 to 14:10: build bcfde26 on 72 threads, with `--hook-after 600`: 4895–4898.
6. From 14:13 on 2026-10-22 to 01:57 on 2026-10-23: build 551f883, on every thread, with `--background` and
   `--finder-seconds 900`: 4899 to 4913.
7. 01:57 to 05:31 on 2026-10-23: build 9b4b308 on every thread, with the same flags: 4914 to 4920.
8. 05:31 to 12:59: build d45a513 on every thread, with the same flags: 4921 to 4943.
9. 12:59 to 14:47: build afbc256 on every thread, with the same flags and `--profile` (the profile of the search, which
   observes it and changes nothing of it): 4944 to 4949.
10. 14:48 on 2026-10-23 to 20:40: the afbc256 binary again, on the checkout of commit 4635fad, in the lab machine's new
    layout (the chain's checkout and results under `~/taxman_work`), with the same flags and the wall of 48 hours
    (`--game-hours 48`): 4950 to 4952.
11. 20:40 to 22:25: build d10e27a, in which `conflict_look` is on by default, with the same flags and no wall: 4953 to
    4955.
12. 22:25 to 00:32 on 2026-10-24: the same build, with a window of 1 game (`--window 1`): 4956 to 4959.
13. 00:32 on 2026-10-24 to 14:47 on 2026-10-27: build c3357cd, with `--tactic price_reuse=on` added and the window back
    to 4: 4960 to 4983.
14. 14:47 to 16:44: build 2f124a6, with the same flags less `--tactic conflict_branching=on`, and conflict branching off
    in 4984 by the lab's command (below): 4984, unfinished.
15. 16:44 to 17:37: build 3e15799, with `--tactic conflict_branching=off` and `--carried-dir` added, so that each game's
    search reads the nogoods that earlier games learned (the tactic `carried_nogoods`): 4984, unfinished.  Its store of
    4983 was made by the lab from units of 4983's checkpoint.
16. 17:37 to 21:21: build 88b1228, with `--tactic warm_start=on` added, on a build in which each game's search writes
    its own store for the games after it: 4984 to 4990.
17. 21:21 on 2026-10-27 to 02:44 on 2026-10-28: build 947946a, with the same flags: 4991.
18. 02:44 to 09:06 on 2026-10-28: build 29aa8ba, with the same flags: 4992 to 5036.
19. 09:06 to 12:02: build 7140548 (cheaper weight splits and derived tableaux in the GF(2) bound, with the same trees),
    with the same flags: 5037 to 5046.
20. 12:02 to 15:08: build ee9d945 (warm starts stopped at the need, and A* for the GF(2) bound's other searches, with
    the same trees), with the same flags: 5047 and 5048, and 5049 unfinished.
21. 15:08 to 18:42: build ff89271 (searches that stop where their checks do, and a child's contractions a block at a
    time, with the same trees), with the same flags: 5049, unfinished.
22. 18:42 on 2026-10-28 to 06:30 on 2026-10-29: build 1b59354 (floors from the level before and copies swapped back,
    with the same trees), with the same flags: 5049 to 5053, and 5054 unfinished.
23. 06:30 on 2026-10-29 to 01:01 on 2026-10-30: build 85dd216, with `--gate-trial 1000,800,600` added, so that each
    game's search tries those gates against the policy's own once it has taken 50 M steps and keeps the cheapest ([the
    design](../solver/docs/design/gate-trial.md)): 5054, which the twenty-second had left unfinished, and 5055 to 5060.
    The project's owner ended the hunt for a larger n at 00:20 on 2026-10-30 (17:20 PDT on 2026-10-29), and this
    invocation was stopped by SIGTERM to `chain.sh` at 01:01:29 (18:01 PDT), after 5060, whose line is the last in this
    file.  The search of 5061 had taken 62.1 M nodes in 305 s of its own time and left only an incomplete line, which is
    left out.

* `invocations` is 1 in every kept line but 4879, 4880, 4895, 4899, 4900, 4901, 4914, 4921, 4944, 4945, 4950, 4952,
  4953, 4956, 4960, 4984, 4991, 4992, 4995, 5037, 5040, 5047, 5048, 5049, 5052 and 5054, which a stop interrupted: 4984
  and 5049 record 4, 4880, 4953, 4956 and 4992 record 3, and the others 2.  `prev.source` is `results` for 4875, 4879,
  4880, 4895, 4899, 4914, 4921, 4944, 4950, 4953, 4956, 4960, 4984, 4991, 4992, 5037, 5047, 5049 and 5054, the first
  game of each of the nineteen invocations whose lines are kept that read the game of n − 1 from the results file, and
  `run` for the other 167.  Threads: 88 in 4875–4878 and 4899–5060, and 72 in 4879–4898.
* The 186 games are 71 searches (all exhausted), 19 proofs at the ceiling (method `start`: 4876, 4878, 4887, 4896,
  4902, 4917, 4923, 4959, 4965, 4977, 4986, 5005, 5012, 5013, 5025, 5031, 5044, 5055 and 5058), 25 primes (4877, 4889,
  4903, 4909, 4919, 4931, 4933, 4937, 4943, 4951, 4957, 4967, 4969, 4973, 4987, 4993, 4999, 5003, 5009, 5011, 5021,
  5023, 5039, 5051 and 5059) and 71 repeats.  No game came from the bounds store, which has proven none past 4783.
* The searches of 4875–4900 took 2.63 G nodes and 22.97 hours of game time (the games overlap in the window), and every
  one 159 M nodes or more.  The dearest by nodes were 4893 (614 M nodes, 17,593 s), 4880 (290 M), 4890 (283 M) and 4895
  (272 M, 17,633 s).  The queue of work units never held more than 9,429, and none spilled.  The 21 searches of
  4901–4949 took 4.65 G nodes, 81.5 M or more each; the dearest by nodes were 4929 (400 M), 4925 (394 M), 4921 (386 M)
  and 4928 (384 M), and by `seconds.own`, the game's own search time ([glossary](../docs/GLOSSARY.md#the-search)), 4921
  (6,803 s), 4929 (5,744 s) and 4913 (5,670 s).  The 12 searches of 4950–4983 took 18.33 G nodes, 502 M or more each;
  the dearest, by nodes and by `seconds.own`, were 4983 (8.77 G nodes, 160,387 s or 44.6 h, under the 48 hours of the
  wall, which the invocations from the eleventh on did not have), 4968 (1.39 G, 19,448 s), 4970 (1.29 G, 18,269 s) and
  4980 (1.24 G, 17,252 s).  The queue of work units never held more than 10,549, and none spilled.  The 6 searches of
  4984–5000 took 8.40 G nodes; the dearest, by nodes and by `seconds.own`, were 4984 (3.87 G nodes, 63,534 s over four
  invocations), 4992 (1.95 G, 15,680 s), 4991 (1.21 G, 11,143 s) and 4995 (978 M, 6,287 s), and 5000 took 28.6 M nodes
  and 189 s.  Their queues of work units never held more than 10,314, and none spilled.  The 12 searches of 5001–5036
  took 1.54 G nodes, 16.8 M or more each; the dearest by nodes were 5022 (287 M, 2,360 s), 5032 (196 M) and 5035
  (194 M).  The 10 searches of 5037–5060 took 27.30 G nodes, 193 M or more each; the dearest, by nodes, were 5054 (7.45
  G, 37,449 s over two invocations), 5056 (6.20 G, 30,980 s), 5052 (4.09 G, 21,088 s), 5049 (4.05 G, 22,543 s over four
  invocations), 5060 (3.18 G, 16,094 s), 5040 (552 M, 3,843 s) and 5043 (537 M, 3,850 s).  The queue of work units in
  5001–5060 never held more than 15,292 (5056's), and none spilled.  The lines of 4899 to 5060 record `seconds.own`.
  The lines of 5054 to 5060 record `options.gate_trial`, 1000, 800 and 600.  The line of 5054 has a gate of 600 in its
  `lab` key, which the lab set at 04:16 on 2026-10-29, and records no trial.  The lines of 5056 and 5060 record the
  trial in `engine.gate_trial`: 24 units at 57.2 M and 58.3 M steps, a CPU for the gate of 600 of 0.855 and 0.874 of
  the gate of 1200's, and the gate 600 chosen and applied in both ([the
  notebook](../solver/notebook/search/a-gate-for-each-game-2026-10/README.md)).  5055 and 5058 were proofs at the
  ceiling with no node.
* During the fourth to the sixth invocations the lab ran studies in a sandbox beside the chain, on 16 threads at nice
  19.  Its command folder carried no command to the solver, and no line up to 4919 has a `lab` key, so the lab changed
  no search there.  From 4920 on the lab set a policy for the tactic `conflict_look` (the `lab` key of 67 lines, 4920 to
  5060, and `tactics` `conflict_look=on` on the 9 where it was on: 4920, 4928, 4932, 4936, 4940, 4944, 4949, 4950 and
  4952); the tactic only chooses between two true conflicts of a node, so both verifiers accept the lines, which they do
  for any tactic.
* On 2026-10-27 and 28 the lab also changed the run's threads and its branching, through its command folder:
  * the threads to 44 from 12:50 to 13:02 and from 13:14 to 13:29 on 2026-10-27, in 4984's search, and back to 88;
  * conflict branching off for 4984, and by a policy for the games from 4985 on, at 14:19 on 2026-10-27;
  * conflict branching on at a gate of 1200 for 4991 at 21:54, for 4992 at 02:45 on 2026-10-28, and by a policy for the
    games from 4995 on at 03:12, which reached the searches of 4995, 4998 and 5000 and the 30 of 5001–5060, and was
    not changed again.

  Conflict branching only chooses which true conflict a node branches on, so both verifiers accept the lines.  The
  notebook's entry on the root gap records why the lab chose these settings: [the sweep of
  4984](../solver/notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-27-a-sweep-of-the-settings-on-game-4984)
  and [the settings checks of 4991, 4992 and
  4995](../solver/notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-27-settings-checks-on-games-4991-4992-and-4995).
* The near-repeat cut was armed from 4914 on.  It cut nodes in the searches of 4929 (20,458,994), 4936 (52,079),
  4952 (2,670), 4953 (165), 4991 (58,300,645), 5037 (254,558), 5043 (12,716,671), 5047 (1,925,714) and 5048 (585,636),
  and no other.
* The tactic `price_reuse` ([design](../solver/docs/design/price-reuse.md)), which keeps the prices that conflict
  branching makes for the nodes of a searcher's path, was on from the thirteenth invocation, in 4960 to 5060.  It only
  ranks true conflicts of a node, so both verifiers accept the lines.
* The tactics `carried_nogoods` ([design](../solver/docs/design/carried-nogoods.md)) and `warm_start`
  ([design](../solver/docs/design/gf2-bound.md#the-gf2-bound)) were on in 4984 to 5060.  A carried nogood is checked at
  the game's root before the search uses it, and the warm start changes only which of several heaviest sets the GF(2)
  bound finds, so both verifiers accept the lines.

**The earlier chains.**  This is the fourth chain committed.  The earlier three were chains to 1300, and git has them:

* the first, of commit bd9bb5d, in schema 3: `git show cd3d32f:rust/results/strict.jsonl`;
* the second, of commit 243af9b, in schema 4: `git show 2487671:rust/results/strict.jsonl`;
* the third, of commit 271c817, in schema 5, on 4 threads: `git show 0133afe:rust/results/strict.jsonl`.

All three have this file's scores in games 1–1300, and the second and the third its proofs and methods too, but that a
game whose search took no step is `search` in them and `start` here.  The first was made before the tactic
`same_search` existed, so it searched the 545 games that this file concludes as repeats.
Their moves differ from this file's in a few games ([README.md, "Games with several optimal
sets"](README.md#games-with-several-optimal-sets)).

**Other files checked with it.**  Three other files were checked with this file, and agree with it ([README.md,
"What supports the scores"](README.md#what-supports-the-scores)):

* `default.jsonl` (1001–1231, schema 1), the first run past 1000, made with the unproven tactics `reuse_prev` and
  `revise_freedoms` on.  It is gone from the tree; `git show f72c332:rust/results/default.jsonl` has it.  It had 34
  prime, 78 exhausted and 119 proofs by `reuse_prev`, and 19 games resting on nothing unproven.
* The cost-curve study's `solver/notebook/search/benchmarks-the-probe-pgo-and-scaling/chain.jsonl` (1001–1231, schema 1,
  commit e710d16), a chain with those tactics off.  It has 34 prime, 127 exhausted and 70 at the ceiling, all 231
  resting on nothing unproven.
* The GF(2) study's `solver/notebook/search/the-gf2-bound-in-the-search-2026-10/chain.jsonl` (1301–2175, schema 5,
  commit 716ebd9, 2 threads), a chain from the table's game of 1300.

The two chains stay as their studies' data.  The verifiers read the second beside this file, and no longer read
the first.
