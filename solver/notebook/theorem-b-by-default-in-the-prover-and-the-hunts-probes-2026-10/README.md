# Theorem B by default in the prover, and the hunt's probes (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

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
(`bench/hunt/recalibration.jsonl`): the same nodes at every pair that finished (301, 11, 4 at 1302; 9,634, 15,065,
152,993, 351,905 and 1,146,404 at 3311 and 2346), `none` for every margin up to 899 in the first calibration's seconds
(53 at 835, 121 and 185 at 896-899), `probed` for 959, 1126 and 1215 (181.8 to 186.9 s each, the start reaching 598 to
854 below V), no counterexample.  The cost of the cloud run falls from about 400 vCPU-hours (every pair using its 10
minutes) to about 140: the probes, 578 pairs of 3.1 minutes on 4 threads, are 120 of them; a `c8g.16xlarge` for about 2.6
hours with the setup, about $2 at spot, $7 on demand, deadline 5 hours.  Neither E9(a) nor E6 is launched: the owner
deferred them to a later batch of AWS work.

## Later (2026-10): the program removed from master

The example `hunt` and its script `tools/cloud/hunt.sh` were removed from master on 2026-10-17.  They build at commit
8753189: `git checkout 8753189`, then `cargo build --release --example hunt` in `rust/`.
