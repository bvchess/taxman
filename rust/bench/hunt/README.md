# The counterexample hunt: the data

What the hunt does and what each verdict is worth: the module comment of `examples/hunt.rs` and `sacrifice::hunt`
(the board without one open source of U\*, searched by the frame solver from an incumbent worth the recorded score, so
that it looks only for a better game).  Why: heuristics/THEORY.md, "The upper part" (U), and the entry
["The upper part game by game"](../../docs/EXPERIMENTS.md#the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10).
The open sources and their margins come from the prover's `upper` (`../../prover/data/upper_*.jsonl`).

| file | command | what it is |
|---|---|---|
| `calibration.jsonl` | `hunt --upper ../../prover/data/upper_1301_2175.jsonl --list 1302:682,1302:715,1302:754 --minutes 2`, then on `upper_2176_3314.jsonl` four pairs of 3311 (`--minutes 3`), five (`--minutes 6`), three of 3311 and three of 2346 (`--minutes 5`); one thread, one job, `nice -n 10`, on the M1 Max, the code of the commit that added the hunt (and one smoke pair, 1304:682) | 19 pairs, no counterexample: at 1302 (U proven there) `none` in milliseconds; at 3311 (gap 1303) and 2346 (gap 1346) `none` for every margin up to 899 (under 6 s to margin 781, 51 to 182 s at 835 to 899), `timeout` for every margin from 959 to 1215 |

The cost of a pair follows its margin, the barred parity bound's excess over the recorded score, not its game's gap:

| margin | pairs | verdict | seconds (one thread) | nodes |
|---|---|---|---|---|
| 10 to 582 | 7 | none | 0.01 to 2.7 | 1 to 9,634 |
| 674 to 781 | 3 | none | 1.0 to 5.5 | 2,485 to 15,108 |
| 835 to 899 | 3 | none | 51 to 182 | 152,993 to 1,146,404 |
| 959 to 1215 | 6 | timeout | 180 to 300 (the limit) | 710,842 to 2,046,333 |

`tools/cloud/RUNBOOK.md`, section 10.2, turns this into the estimate of the cloud run (20 games, 4,499 pairs); its
output is to go here as `hunt_2176_3314.jsonl` with its summary.
