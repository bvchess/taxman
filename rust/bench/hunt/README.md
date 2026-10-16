# The counterexample hunt: the data

What the hunt does and what each verdict is worth: the module comment of `examples/hunt.rs` and `sacrifice::hunt`
(the board without one open source of U\*, searched by the frame solver from an incumbent worth the recorded score, so
that it looks only for a better game).  Why: heuristics/THEORY.md, "The upper part" (U), and the entry
["The upper part game by game"](../../docs/experiments/the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10.md).
The open sources and their margins come from the prover's `upper` (`../../prover/data/upper_*.jsonl`).

| file | command | what it is |
|---|---|---|
| `calibration.jsonl` | `hunt --upper ../../prover/data/upper_1301_2175.jsonl --list 1302:682,1302:715,1302:754 --minutes 2`, then on `upper_2176_3314.jsonl` four pairs of 3311 (`--minutes 3`), five (`--minutes 6`), three of 3311 and three of 2346 (`--minutes 5`); one thread, one job, `nice -n 10`, on the M1 Max, the code of the commit that added the hunt (and one smoke pair, 1304:682) | 19 pairs, no counterexample: at 1302 (U proven there) `none` in milliseconds; at 3311 (gap 1303) and 2346 (gap 1346) `none` for every margin up to 899 (under 6 s to margin 781, 51 to 182 s at 835 to 899), `timeout` for every margin from 959 to 1215 |

| `recalibration.jsonl` | `hunt --upper ../../prover/data/upper_1301_2175.jsonl --list 1302:682,1302:754,1302:715`, then `hunt --upper ../../prover/data/upper_2176_3314.jsonl --list 3311:1826,2346:1265,3311:1978,3311:1818,2346:1495,3311:1926,3311:1856,3311:1792,3311:2511`, both with `--threads 1` and the defaults of the policy (margin up to 900 to completion in 10 minutes, above it a probe of 3 minutes), `nice -n 10`, on the M1 Max, 2026-10-15, the code of the commit that added the probes (master with the numbering by layer) | 12 of the pairs above, no counterexample: the same nodes as `calibration.jsonl` at every pair that finished, `none` for every margin up to 899, `probed` for 959, 1126 and 1215 (they were `timeout`); a probe's `start_score` is the score the solver's start (greedy set, dive, local search) reached |

The cost of a pair follows its margin, the barred parity bound's excess over the recorded score, not its game's gap:

| margin | pairs | verdict | seconds (one thread) | nodes |
|---|---|---|---|---|
| 10 to 582 | 7 | none | 0.01 to 2.7 | 1 to 9,634 |
| 674 to 781 | 3 | none | 1.0 to 5.5 | 2,485 to 15,108 |
| 835 to 899 | 3 | none | 51 to 182 | 152,993 to 1,146,404 |
| 959 to 1215 | 6 | timeout | 180 to 300 (the limit) | 710,842 to 2,046,333 |

With the policy (RUNBOOK 10.2: the pairs in decreasing margin, up to 900 to completion under 10 minutes, above it a probe
of the solver's start and 3 minutes, recorded `probed`, never `none`), the same pairs rerun in `recalibration.jsonl`
(one thread, the seconds of the pairs that finished the same as the first calibration within load noise, since only a
probe makes the start):

| margin | pairs | verdict | seconds (one thread) | nodes | V − the start's game |
|---|---|---|---|---|---|
| 10 to 582 | 5 | none | 0.01 to 2.7 | 1 to 9,634 | |
| 734 | 1 | none | 5.6 | 15,065 | |
| 835 | 1 | none | 52.8 | 152,993 | |
| 896 to 899 | 2 | none | 120.9 to 184.9 | 351,905 to 1,146,404 | |
| 959 to 1215 | 3 | probed | 181.8 to 186.9 (a 180 s search and a start of 2 to 7 s) | 551,015 to 1,015,324 | 598 to 854 |

The start (greedy set, dive, local search) reaches within 0.02 to 0.05 per cent of the recorded score at these margins
and is never above it.  Forced to run as probes at 1302 (`--probe-above 0 --probe-minutes 1`, the three pairs above, whose
searches end in milliseconds, so the lines say `"finished":true` and `probed`), the start with the recorded game of
1302 as its seed (`--results ../../results/strict.jsonl`) reached 44, 116 and 308 below the score, without the seed 66,
138 and 173 below: the seed does not decide it.  Of 4,499 pairs, 3,533 have a margin up to 800, 388 between 800 and 900
and 578 above 900.

`tools/cloud/RUNBOOK.md`, section 10.2, turns this into the estimate of the cloud run (20 games, 4,499 pairs); its
output is to go here as `hunt_2176_3314.jsonl` with its summary.
