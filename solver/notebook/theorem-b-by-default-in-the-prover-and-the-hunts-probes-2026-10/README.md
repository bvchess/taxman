# Theorem B by default in the prover, and the hunt's probes (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  The third consultation of 2026-10-15 (the theory track's program, part III, section 15) answered four
questions on the entry "The upper part game by game": (c) should the prover run from Theorem B's roles from now on, (a)
what should E9(a) spend on a pair, (d) does the hunt need rebuilding after the numbering by layer, and (b) is 2B's
test of the certificates strong enough.

*(c) The roles by default* (commit 52fc685).  `exact` starts every search from the roles of Theorem B unless told
`--fix-low 0` (the plain root, which made the committed `data/exact_*.jsonl`); every line records `"fix_low"` (1 or 0)
and `fixed_bound`; a search from the roles keeps its own checkpoint (`exact_N_low.ckpt`); `summarize.py exact` and
`check` count the proofs that rest on Theorem B (none in the committed files, which still pass `check 2 1300` and
`check 1301 2175`).  The frames of layer 2 are closed when U\*_2 has as many members as there are primes up to n/2, and
`upper::layer_two_closed` now decides it for each board instead of trusting n ≥ 187: the numbers of layer 2 up to n/2
are barred where it holds and free where it does not, which is the 80 boards of 2..1300 that THEORY.md lists, all
below 187 (the old function panicked on a board from 187 up where it failed; there is none to 1300).  Checks: for every
game of 2..2175 the recorded game obeys the roles and their parity bound is at least its score (`exact --budget 1`,
50 s and 214 s on one thread); the 391 searches of 2..999 prove again, in 588,026 nodes and 357 s against the plain
root's 614,900 nodes and 712 s (the nodes 4% fewer, the time a node halved); `tests/exact.rs` runs the binary over
187..300 in the default, `--fix-low 0` and `--fix-upper 1` modes.  A new full proof would cost about half what
RUNBOOK section 9 estimates.

*(b) A mutation check for the certificates* (commit 7d67ef5).  The test of the barred bounds already recomputed the
certification at each claim T of V − 1, V − 2 and V − 5 and checked every playable set that obeys Theorem B and is worth
more than T (boards 2..60, 1,403 checks); the boards and claims are enumerated once now, and a second test applies the
same check to a deliberately wrong rule, which certifies a member also when its barred bound is at most T + 10: it is
caught, 12 false certifications over the boards and claims giving 15 (set, member) misses on 12 boards, so the test can
see the failure it guards against.

*(a, d) The hunt's probes* (commit c308c77).  The advisor's answer to what E9(a) should spend on a pair: do not
drop the pairs of margin above 900, which matter most (a counterexample needs the punctured problem's own slack to be
below the margin, the less the closer the margin is to the root's gap), but they will not finish, so run the pairs in
decreasing margin, those up to 900 to completion under the 10-minute limit, and those above it as probes: the solver's own
start and 3 minutes of search, recorded as `probed`, never as `none`.  `examples/hunt.rs` does that (`--probe-above`,
`--probe-minutes`, a `finished` flag on a `probed` line whose search happened to end, `start_score`, and `--results FILE`
for the recorded games that seed the start); `sacrifice::hunt` makes the start on request (`PuncturedOptions::start`
and `seed`), from `search::start_incumbent`, the start of `search` taken out as it was (no tree changes, the suite and
the tripwires are the check); `hunt.sh` has `PROBE_ABOVE` and `PROBE_MINUTES`.  Only a probe makes the start: a search
that runs to its end finds every game above V without it, and it costs 2 to 7 seconds a pair at 3311.  Tests: the hunt
with the start, seeded or not, agrees with the hunt without on 298 pairs of the boards 30 to 150 (the start alone finds
the best game on 292), and `cargo t --release --example hunt` has six tests of the order, the probes, the verdicts and
the seeds.  Rebuilt on master (numbering by layer) and rerun on one laptop thread, 12 of the calibration pairs
(`solver/notebook/theorem-b-by-default-in-the-prover-and-the-hunts-probes-2026-10/recalibration.jsonl`): the same nodes at every pair that finished (301, 11, 4 at 1302; 9,634, 15,065,
152,993, 351,905 and 1,146,404 at 3311 and 2346), `none` for every margin up to 899 in the first calibration's seconds
(53 at 835, 121 and 185 at 896-899), `probed` for 959, 1126 and 1215 (181.8 to 186.9 s each, the start reaching 598 to
854 below V), no counterexample.  The cost of the cloud run falls from about 400 vCPU-hours (every pair using its 10
minutes) to about 140: the probes, 578 pairs of 3.1 minutes on 4 threads, are 120 of them; a `c8g.16xlarge` for about 2.6
hours with the setup, about $2 at spot, $7 on demand, deadline 5 hours.  Neither E9(a) nor E6 is launched: the owner
deferred them to a later batch of AWS work.

## Later (2026-10): the program removed from master

The example `hunt` and its script `solver/tools/cloud/hunt.sh` were removed from master on 2026-10-17.  They build at commit
8753189: `git checkout 8753189`, then `cargo build --release --example hunt` in `rust/`.

## Files

The probes of the hunt, from the data of [the upper part](../the-upper-part-game-by-game-and-two-cloud-runs-prepared-2026-10/README.md#files); the command, the build and the margins are described there.

| file | command | what it is |
|---|---|---|
| `recalibration.jsonl` | `hunt --upper ../../prover/data/upper_1301_2175.jsonl --list 1302:682,1302:754,1302:715`, then `hunt --upper ../../prover/data/upper_2176_3314.jsonl --list 3311:1826,2346:1265,3311:1978,3311:1818,2346:1495,3311:1926,3311:1856,3311:1792,3311:2511`, both with `--threads 1` and the defaults of the policy (margin up to 900 to completion in 10 minutes, above it a probe of 3 minutes), `nice -n 10`, on the M1 Max, 2026-10-15, the code of the commit that added the probes (master with the numbering by layer) | 12 of the pairs above, no counterexample: the same nodes as `calibration.jsonl` at every pair that finished, `none` for every margin up to 899, `probed` for 959, 1126 and 1215 (they were `timeout`); a probe's `start_score` is the score the solver's start (greedy set, dive, local search) reached |

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
