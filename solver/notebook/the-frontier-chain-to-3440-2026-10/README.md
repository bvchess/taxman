# The frontier chain to 3440 (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  The day on one `c8g.16xlarge` that [the gap past 2175](../search/the-gap-past-2175-2026-10/README.md) projected (from the
committed file's 2175 games, about 4,100, with 3,700 to 4,850 across its models) was run.  How far did it take the chain
with every game proven within the frame model and nothing assumed; what did the dear games cost; do the independent
chains that cover its games agree with them; and what did its operators find wrong in the procedure
([RUNBOOK](../../docs/RUNBOOK.md))?

*What was run* (2026-10-15 04:05:42Z to 2026-10-16 04:05:42Z, commit 55287ff, a clean tree): `solver/tools/cloud/chain.sh
4095`, that is `taxman solve 1-4095 --window W --out strict.jsonl --checkpoint-dir ckpt --checkpoint-interval 300
--progress 60` on all 64 CPUs of one `c8g.16xlarge` (Graviton4, 124 GiB) on spot in us-east-1d at $0.6752 an hour, on
a copy of the committed file's 2175 lines, with a deadline of 24 hours, `S3_DEST` a prefix (`chain-2176/`) of the
project's bucket (results, logs and checkpoints, every 10 minutes) and `spot-watch.sh` beside it; no interruption
notice came.  A rehearsal of RUNBOOK 4.5 on the same instance came first (03:51 to 04:04Z, to 2185 and to 2300, with a
TERM and a KILL, both referees agreeing), and found that its stops must wait for a checkpoint to exist: the games past
2175 finish in seconds, and the first try ended before its stop and the second killed the solver with no checkpoint on
disk.

* *Instance 1* (i-0c23a62e915eac46e): window 16, the default.  Games 2176..3310 in 23,025 s (6.4 hours), then stopped
  by a signal, with a final checkpoint, at 10:29:26Z; 6.80 instance-hours, $4.59.  A window of 16 had been measured
  20-32% slower than 1 on the games past 2225 ([scaling to 192 threads](../scaling-to-192-threads-2026-10/README.md)), hence the
  move.
* *Instance 2* (i-0f588d4ad388bfd5c, launched 10:31:24Z): restored from the bucket, window 4, the solver from 10:40:41Z
  with 3310 games done.  Games 3311..3440 and part of 3441 in 62,701 s (17.4 hours), then stopped by the deadline (24
  hours after the first start, given as the hours that remained), with a final checkpoint; terminated; about 17.6
  instance-hours, about $11.87.  3311, 3312 and 3315 were in flight at the move and finished from their checkpoints
  (`invocations` is 2 in their lines).

About $16.46 in all, nothing left running; the prefix `chain-2176/` keeps the logs and the two checkpoints.

*What it made.*  The 1,265 games 2176..3440, each built from the one before, assuming nothing: 567 searches (448
exhausted, 119 at the ceiling, of which 116 decided before their first node), 543 repeats and 155 primes (the search
problem of 483 repeats is the one of n − 1, of 60 the one of n − 2).  The searches took 17.73 billion nodes, 70 games of
30 million nodes or more taking 16.34 billion of them.  By blocks of 200 n, the 448 exhausted searches:

| n | searches | nodes | search seconds, summed |
|---|---|---|---|
| 2176-2375 | 76 | 2.25 G | 19,352 |
| 2376-2575 | 76 | 1.32 G | 18,862 |
| 2576-2775 | 71 | 0.02 G | 258 |
| 2776-2975 | 58 | 0.02 G | 186 |
| 2976-3175 | 70 | 1.06 G | 21,532 |
| 3176-3375 | 73 | 3.27 G | 37,203 |
| 3376-3440 | 24 | 9.78 G | 72,330 |

(The seconds of games in flight overlap, so they add up to more than the day.)  The cost comes in patches: the 2340s
(2346 at 1.14 G nodes), 2375-2388, 3102-3112, 3174-3176 (about 210 M each), 3300 (635 M), the 3330s, and from 3399 on
three of the four dearest games of the run, 3424 (1.30 G, 2.0 hours on 64 threads), 3425 (1.23 G, 3.5 hours) and 3440
(3.72 G nodes, 18,184 s, 5.05 hours), with 3400 (522 M) and 3430 (804 M) beside them; the valleys (2576-2975) cost
minutes.  A least-squares line through the log of the search seconds of the 448 gives a decade per about 1,040 in n
(1,070 without 3440), 7 s at 2200 and 97 s at 3400 (93 without 3440), with a standard deviation of one decade around
it, nearly all of it the patches, against a decade per about 1,050-1,250 in n that [the gap past
2175](../search/the-gap-past-2175-2026-10/README.md) projected from the gaps.  The long searches
run at the engine's best: 3441 at 202 to 209 thousand nodes a second on 64 threads (3.2 thousand a thread), `eff`
100%, `idle` and `lock` 0.0%, 71 pauses for checkpoints (0.0%, the longest 29.9 ms), 4,100 to 6,100 units queued in its
last minutes, none spilled; its checkpoint is 5.5 MB.

*Against the projection.*  The projection gave 4,088 for a day of the whole chain from 2176 in its centre model (10th
to 90th percentile 3,726 to 4,740) and 3,720 to 4,613 across the others.  The day reached 3440: under the 10th
percentile of five of the seven scenarios (all but the two that use the gap alone, whose 10th percentiles are 3,427 and
3,420) and with 3441 unfinished, at 626 million nodes after 5.9 hours (best so far 3,729,369, the ceiling being
3,731,167; 3444 was begun, at 63 nodes).  What fell short is the end: the first instance made 2176..3310 in 6.4 hours,
about where the model put it (6.1 hours), and the remaining 130 games took 17.4, the last forty holding three of the
four dearest games of the run.  The projection's range for a single run's luck (−400 to +700 around 4,100) does not
reach 3440; whether the patch at the end is luck or the trend is open, and what the gaps of 3315..3440 were is not
measured (the committed games can be fed to `solver/notebook/search/the-gap-past-2175-2026-10/gap.py` and `examples/wall.rs nodes --nodes 1`: one thread, a
few minutes).

*Agreement.*  Both referees on the file (`solver/tools/cloud/verify.sh`: `taxman verify-results`, 10 s and 42 s of CPU on
several threads, and `solver/tools/check_results.py`, 3 s): every game replayed legally and finished, the chain bounds, the
Franklín–Moniot bound, the heuristic chain; proofs trivial 13, prime formula 481, cap 311, exhausted 1178, same search
1457; nothing assumed; scores at a proven ceiling given the game before: n + score(n − 1) 1096, prime formula 480.  The
file's first 2175 lines are the committed file's, byte for byte.  The Graviton calibration's chain
(`solver/frame/testdata/c8g.chain-2176.jsonl`: 101 games, 2176..2276) and a rehearsal of this run's own (125 games, 2176..2300; not
committed) have this file's scores, proofs, methods, promotions and moves in every game they share with it, move for
move.  Past 2300 nothing else covers the games; the prover's proofs on the whole board stop at 2175.

*What the file does and does not say.*  The 991 games of 2176..3440 proven by an exhausted search or a repeat rest on
statement U (the frame solver's answer is opt(n) exactly when U holds at n), and the 274 proven at the ceiling or by the
prime formula on the games before them.  U holds where the prover has proven the optimum (n ≤ 2175) and is not certified
past it: the barred bounds of `solver/noframe/data/upper_2176_3314.jsonl` leave members open in every game, and `taxman
certify` (E8 stage 2) will close them game by game with the open ones optional.  `optimal.json` stays at 1300.

*What the operators found* (each is now in the [RUNBOOK](../../docs/RUNBOOK.md)): the rehearsal's stops wait for a
checkpoint (4.5); `spot-watch.sh` logs only to its terminal, so it is run through `tee` into `logs/spot-watch.log`; the
copy of `save.log` in S3 trails by one save; `save.sh --restore` brings back the results and the checkpoints but not
the logs (copy them back first, or the next save overwrites the old instance's); the deadline file is not saved, so a
replacement machine starts a new deadline unless `DEADLINE_HOURS` is set to the hours that remain; `run-instances`
needs `SpotOptions` with a `MaxPrice`; and `chain.sh`'s default window, 16, is documented as measured worse than 4 at 64
threads.  The checkpoints of 3441 and 3444 (format 4) are read by the present build: 3441 resumed from the saved file at
its 625,763,557 nodes on a laptop (20 seconds on 2 threads, to 625,901,481).

*Cost.*  $16.46 on AWS; the laptop only checked the file afterwards (both referees, the tests of the chain, the extension
of `optimal.json` into a scratch file, the resume of 3441).  The tests that pin the chain
(`tests/results.rs`, `tests/board_layer.rs`, `tests/bottom_layers.rs`) are brought up to 3440, and the documents that
said 2175 now say what is proven and by what (the prover to 2175, the frame model to 3440).
