# How strict.jsonl was made

Part of [the results](README.md).

It was made in six runs, each a continuation of the one before:

| games | where | build | git has the file before the run as |
|---|---|---|---|
| 1–2175 | one invocation, which nothing stopped, on a laptop | commit feba6d0 | (the earlier files, below) |
| 2176–3440 | appended to a copy of those lines on a cloud machine | commit 55287ff | `git show 4f1c35b:rust/results/strict.jsonl`, whose 2175 lines are this file's first 2175, byte for byte |
| 3441–3914 | appended from a run on a cloud machine of the same kind | commit eb84a6f | `git show b23f497:rust/results/strict.jsonl`, whose 3440 lines are this file's first 3440, byte for byte |
| 3915–4547 | appended from a run on a cloud machine of the same kind, which took the games before from this file and the bounds store (`--use-prev`) | commit ca357bf | `git show c7d81c2:rust/results/strict.jsonl`, whose 3914 lines are this file's first 3914, byte for byte |
| 4548–4874 | appended from a run on a cloud machine of the same kind, which took the games before from this file and the bounds store (`--use-prev`) | commit 4974a71 | `git show f48df13:results/strict.jsonl`, whose 4547 lines are this file's first 4547, byte for byte |
| 4875–4900 | appended from a run on a friend's machine of 88 threads, which took the games before from this file and the bounds store (`--use-prev`) | commits a88dc7f, bcfde26 and 551f883 | `git show 18232de:results/strict.jsonl`, whose 4874 lines are this file's first 4874, byte for byte |

Each build was of a clean tree, which every line records.  Every line records the `default` tactics: no tactic is
unproven any more, so none is switched off.  The sixth run had three builds, one for each stretch of its games.

**The first run, games 1–2175.**  The command was, up to the paths, `taxman solve 1-2175 --threads 8 --out
strict.jsonl --checkpoint-dir ckpt --checkpoint-interval 60 --progress 0`, run once to the end on an Apple M1 Max
(10 CPUs) with nothing else running.  Every line records the build of commit feba6d0 (`dirty` false), 8 threads, no
`--time-limit` or `--node-limit`, and the `default` tactics: the GF(2) bound, `gf2`, on, and the Lagrangian bound over
the nogoods, `lagrange`, off.  No game was resumed (`invocations` is 1 in every line of 1–2175), and none was built from
a results file or from `optimal.json` (`prev.source` is `run` from 2 on).  The [entry on the
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

**The sixth run, games 4875–4900.**  The command was `solver/tools/cloud/chain.sh 8191`, which runs `taxman solve 1-8191
--use-prev` with a window of 4 games in flight and [the wall](../docs/GLOSSARY.md#the-search) at 48 hours of a game's
own search, on a friend's machine of 88 threads: two Intel Xeon E5-2699 v4 (x86_64, Ubuntu).  It continued the fifth
run's results file and its checkpoints, which were copied from the cloud machine.  It took games 1–4874 from the file
and each game the bounds store had proven from the store, with no search, and searched the others, from 4875 on.  Its
own file started with `known` copies of games 1–4874.  The lines of games 4875–4900 were appended to this file, byte for
byte.  Those copies and the run's incomplete lines (two of game 4875, and one each of 4879, 4880, 4895 and 4899) were
left out.  No line was relabelled: every build is after commit 7987c86, so a search that took no step already says
`start`.  Every kept line records that the tree was clean (`dirty` false), no `--time-limit` or `--node-limit`, the
`default` tactics, with the tactic `conflict_branching` on, and the option `use_prev`; the lines of 4899 and 4900 record
`background` and `finder_seconds` 900 too.  The first line, 4875, records its game of 4874 as the line of this file,
with its proof, `same_search`, which both verifiers accept.

*Details.*  `chain.sh` ran six times, from 2026-10-21 to 22 (times UTC).  Each stop was a SIGTERM to `chain.sh`: the
solver wrote a final checkpoint, and the next invocation resumed from it.  The lines come from the second to the
sixth:

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
6. From 14:13 on 2026-10-22: build 551f883, on every thread, with `--background` and `--finder-seconds 900`: 4899 and
   4900, and it went on past them.

* `invocations` is 1 in every kept line but 4879, 4880, 4895, 4899 and 4900, which a stop interrupted: 4880 records 3,
  the others 2.  `prev.source` is `results` for 4875, 4879, 4880, 4895 and 4899, the first game of each of the five
  invocations whose lines are kept, which read the game of n − 1 from the results file, and `run` for the other 21.
  Threads: 88 in 4875–4878, 4899 and 4900, and 72 in 4879–4898.
* The 26 games are 10 searches (all exhausted), 4 proofs at the ceiling (method `start`: 4876, 4878, 4887 and 4896),
  2 primes (4877 and 4889) and 10 repeats.  No game came from the bounds store, which has proven none past 4783.
* The searches took 2.63 G nodes and 22.97 hours of game time; the games overlap in the window.  Every search took 159 M
  nodes or more.  The dearest by nodes were 4893 (614 M nodes, 17,593 s), 4880 (290 M), 4890 (283 M) and 4895 (272 M,
  17,633 s).  The queue of work units never held more than 9,429, and none spilled.  The lines of 4899 and 4900 record
  `seconds.own`, the game's own search time ([glossary](../docs/GLOSSARY.md#the-search)): 5,061 s and 4,974 s.
* During the fourth to the sixth invocations the lab ran studies in a sandbox beside the chain, on 16 threads at nice
  19.  Its command folder carried no command to the solver, and no line has a `lab` key, so the lab changed no search.

**The earlier files.**  This is the fourth file of its name.  The earlier three were chains to 1300, and git has them:

* the first, of commit bd9bb5d, in schema 3: `git show cd3d32f:rust/results/strict.jsonl`;
* the second, of commit 243af9b, in schema 4: `git show 2487671:rust/results/strict.jsonl`;
* the third, of commit 271c817, in schema 5, on 4 threads: `git show 0133afe:rust/results/strict.jsonl`.

All three have this file's scores in games 1–1300, and the second and the third its proofs and methods too, but that a
game whose search took no step is `search` in them and `start` here.  The first was made before the tactic
`same_search` existed, so it searched the 545 games that this file concludes as repeats.
Their moves differ from this file's in a few games ([README.md, "Games with several optimal
sets"](README.md#games-with-several-optimal-sets)).

**Other files checked with it.**  Three other files were checked with `strict.jsonl`, and agree with it ([README.md,
"What supports the scores"](README.md#what-supports-the-scores)):

* `default.jsonl` (1001–1231, schema 1), the first run past 1000, made with the unproven tactics `reuse_prev` and
  `revise_freedoms` on.  It is gone from the tree; `git show f72c332:rust/results/default.jsonl` has it.  It had 34
  prime, 78 exhausted and 119 proofs by `reuse_prev`, and 19 games resting on nothing unproven.
* The cost-curve study's `solver/notebook/search/benchmarks-the-probe-pgo-and-scaling/chain.jsonl` (1001–1231, schema 1,
  commit e710d16), a chain with those tactics off.  It has 34 prime, 127 exhausted and 70 at the ceiling, all 231
  resting on nothing unproven.
* The GF(2) study's `solver/notebook/search/the-gf2-bound-in-the-search-2026-10/chain.jsonl` (1301–2175, schema 5,
  commit 716ebd9, 2 threads), a chain from `optimal.json`'s game of 1300.

The two chains stay as their studies' data.  The verifiers read the second beside `strict.jsonl`, and no longer read
the first.
