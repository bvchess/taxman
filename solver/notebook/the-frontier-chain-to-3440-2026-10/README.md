# The frontier chain to 3440 (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-14.  Concluded 2026-10-16.

**Question.**  How far does the day on one `c8g.16xlarge` that [the gap past
2175](../search/the-gap-past-2175-2026-10/README.md) projected (about 4,100 from the committed file's 2175 games, with
3,700 to 4,850 across its models) take the chain, with every game proven within the frame model and nothing assumed?
**Answer.**  To 3440: 1,265 games, 2176..3440, in 24 hours for $16.46, with game 3441 unfinished at 626 M nodes.  The
day fell short of the projection at the end, where the last forty games held three of the four dearest, and both
referees pass the file.

## Method

The run was `solver/tools/cloud/chain.sh 4095`, that is `taxman solve 1-4095 --window W --out strict.jsonl
--checkpoint-dir ckpt --checkpoint-interval 300 --progress 60`, from 2026-10-15 04:05:42Z to 2026-10-16 04:05:42Z.  It
ran at commit 55287ff, a clean tree, on all 64 CPUs of one `c8g.16xlarge` (Graviton4, 124 GiB) on spot in us-east-1d at
$0.6752 an hour, on a copy of the committed file's 2175 lines, with a deadline of 24 hours.  `S3_DEST` was a prefix
(`chain-2176/`) of the project's bucket, which received the results, logs and checkpoints every 10 minutes, and
`spot-watch.sh` ran beside it; no interruption notice came.

A rehearsal of the [chain's stops and
resumes](../../docs/runbook/before-a-big-run.md#rehearsal-of-the-chain-with-interruptions) on the same instance came
first (03:51 to 04:04Z, to 2185 and to 2300, with a TERM and a KILL, both referees agreeing).  It found that a stop must
wait for a checkpoint to exist: the games past 2175 finish in seconds, so the first try ended before its stop, and the
second killed the solver with no checkpoint on disk.

* **Instance 1** (i-0c23a62e915eac46e): window 16, the default.  Games 2176..3310 in 23,025 s (6.4 hours), then stopped
  by a signal, with a final checkpoint, at 10:29:26Z; 6.80 instance-hours, $4.59.  A window of 16 had been measured
  20-32% slower than 1 on the games past 2225 ([scaling to 192
  threads](../scaling-to-192-threads-2026-10/README.md)), hence the move.
* **Instance 2** (i-0f588d4ad388bfd5c, launched 10:31:24Z): restored from the bucket, window 4, the solver from
  10:40:41Z with 3310 games done.  Games 3311..3440 and part of 3441 in 62,701 s (17.4 hours), then stopped by the
  deadline (24 hours after the first start, given as the hours that remained), with a final checkpoint; about 17.6
  instance-hours, about $11.87.  3311, 3312 and 3315 were in flight at the move and finished from their checkpoints
  (`invocations` is 2 in their lines).

The cost was $16.46 in all, with nothing left running; the prefix `chain-2176/` keeps the logs and the two checkpoints.
The laptop only checked the file afterwards.

## Results

**What the run made.**  The 1,265 games 2176..3440, each built from the one before, assuming nothing: 567 searches (448
exhausted, 119 at the ceiling, of which 116 decided before their first node), 543 repeats and 155 primes.  The search
problem of 483 repeats is the one of n − 1, of 60 the one of n − 2.  The searches took 17.73 billion nodes, 70 games of
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

The seconds of games in flight overlap, so they add up to more than the day.  The cost comes in patches: the 2340s (2346
at 1.14 G nodes), 2375-2388, 3102-3112, 3174-3176 (about 210 M each), 3300 (635 M), the 3330s, and from 3399 on three of
the four dearest games of the run: 3424 (1.30 G, 2.0 hours on 64 threads), 3425 (1.23 G, 3.5 hours) and 3440 (3.72 G
nodes, 18,184 s, 5.05 hours), with 3400 (522 M) and 3430 (804 M) beside them.  The valleys (2576-2975) cost minutes.

A least-squares line through the log of the search seconds of the 448 gives a decade per about 1,040 in n (1,070 without
3440), 7 s at 2200 and 97 s at 3400 (93 without 3440), with a standard deviation of one decade around it, nearly all of
it the patches.  The gaps had projected a decade per about 1,050-1,250 in n.  The long searches ran at the engine's
best: 3441 at 202 to 209 thousand nodes a second on 64 threads, `eff` 100%, `idle` and `lock` 0.0%, 71 pauses for
checkpoints (the longest 29.9 ms) and none spilled.

**Against the projection.**  The projection gave 4,088 for a day of the whole chain from 2176 in its center model (10th
to 90th percentile 3,726 to 4,740) and 3,720 to 4,613 across the others.  The day reached 3440, under the 10th
percentile of five of the seven scenarios (all but the two that use the gap alone, whose 10th percentiles are 3,427 and
3,420), with 3441 unfinished at 626 million nodes after 5.9 hours (best so far 3,729,369, the ceiling being 3,731,167).
What fell short is the end: the first instance made 2176..3310 in 6.4 hours, about where the model put it (6.1 hours),
and the remaining 130 games took 17.4.  The projection's range for a single run's luck (−400 to +700 around 4,100) does
not reach 3440.

**Agreement.**  Both referees pass the file (`solver/tools/cloud/verify.sh`: `taxman verify-results`, 10 s and 42 s of
CPU on several threads, and `solver/tools/check_results.py`, 3 s).  Every game replayed legally and finished, and the
chain bounds, the Franklín–Moniot bound and the heuristic chain agree.  The proofs are trivial 13, prime formula 481,
cap 311, exhausted 1178 and same search 1457; nothing is assumed; the scores at a proven ceiling given the game before
(n + score(n − 1)) are 1096, and 480 by the prime formula.  The file's first 2175 lines are the committed file's, byte
for byte.  Two other chains cover its games and have this file's scores, proofs, methods, promotions and moves, move for
move, in every game they share with it:

* the Graviton calibration's chain, `solver/frame/testdata/c8g.chain-2176.jsonl` (101 games, 2176..2276);
* a rehearsal of this run's own chain (125 games, 2176..2300; not committed).

Past 2300 nothing else covers the games; the prover's proofs on the whole board stop at 2175.

**What the operators found wrong in the procedure** is in the [RUNBOOK](../../docs/RUNBOOK.md): the rehearsal's stops
wait for a checkpoint; `spot-watch.sh` runs through `tee` into `logs/spot-watch.log`; `save.sh --restore` does not bring
back the logs, and the deadline file is not saved, so a replacement machine needs `DEADLINE_HOURS` set to the hours that
remain; `run-instances` needs `SpotOptions` with a `MaxPrice`; and `chain.sh`'s default window, 16, was measured worse
than 4 at 64 threads.  The checkpoints of 3441 and 3444 (format 4) read back: 3441 resumed from the saved file at its
625,763,557 nodes on a laptop (20 seconds on 2 threads, to 625,901,481).

## Decision

The 1,265 games are committed in `results/strict.jsonl`, and the tests that pin the chain (`tests/results.rs`,
`tests/board_layer.rs`, `tests/bottom_layers.rs`) were brought up to 3440.  The documents that said 2175 say what is
proven and by what: the prover to 2175, the frame model to 3440.  `optimal.json` stays at 1300.

The file's answers rest on statement U where U is not proven.  The 991 games of 2176..3440 proven by an exhausted search
or a repeat rest on U (the frame solver's answer is opt(n) exactly when U holds at n), and the 274 proven at the ceiling
or by the prime formula rest on the games before them.  U holds where the prover has proven the optimum (n ≤ 2175).
Past it U is not certified: the barred bounds of `solver/noframe/data/upper_2176_3314.jsonl` leave members open in every
game, and `taxman certify` (E8 stage 2) will close them game by game with the open ones optional.

Whether the patch at the end of the run is luck or the trend is open, and the gaps of 3315..3440 are not measured.  The
committed games can be fed to [gap.py](../search/the-gap-past-2175-2026-10/gap.py) and `examples/wall.rs nodes --nodes
1`, which takes one thread and a few minutes.  A measured trend would reopen the projection.

## Files

* `README.md`: this record.  The folder holds no data.  The run's games are the lines 2176..3440 of
  `results/strict.jsonl`, and its logs and checkpoints are in the bucket's prefix `chain-2176/`.

## Commits

* 55287ff (2026-10-14): the build of the run (the runbook brought up to date with EC2).
