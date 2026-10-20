# Round B: the chain on many threads (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-14.  Concluded 2026-10-14.

**Question.**  How does the chain keep the threads of a large machine busy when its games are short?  **Answer.**  A
window of games in flight (`--window K`), steered through a control on each search's engine, cut the chain 1..1300 on 4
threads from 13.4 s to 11.2 s at a window of 16, with the same chains.  It paid only with reclaims, hints, deferral and
suspension: speculation without them made the stretch 2226..2249 take 48.5-52.6 s against 33.5 s in order.

## Method

[Round A](../search/round-a-the-fixed-costs-of-a-game-2026-10/README.md) made the serial part of a game small.  On a
machine of 64 to 192 threads the chain must still keep its threads busy: game n + 1 needs only n's promotions and
score, so it can be set up while n searches and searched before n is proven, and many searches take milliseconds to a
second, so their ramp-up and tail idle most of 64 threads.  The proof of 2172 had scaled 58.5 times on 64 threads, most
of the loss a node rate a thread of 93-94%.  The round has the three parts of [the inner loop's
design](../search/the-inner-loop-of-the-gf2-search-2026-10/README.md): pipeline the chain, run short games side by side
moving threads between their searches, and report what a thread loses at many threads.  All runs were on the M1 Max at
up to 4 threads (`nice -n 10`, other jobs keeping the load at 3 to 6, so a few per cent of noise); the many-thread
measurements are commands for the cloud (items 7 and 8 of ["For a cloud
machine"](../scaling-to-192-threads-2026-10/README.md#for-a-cloud-machine)).  [The
window](../../docs/design/engine.md#the-window-games-in-flight) in the design document says how it works.

* **A control on an engine run** (00d4b68, 3815e82).  `engine::Control` steers a run from another thread: a ceiling that
  can come while it runs, incumbents offered from outside, a stop, and the number of workers, down to 0, which suspends
  the run with its units kept.  Without a control a run is unchanged.  New counters measure the ramp and tail idle time
  and the units' starts (the searcher's rebuild, and the first node).
* **The search through the control** (9ca2c13, 8462d2c).  The engine owns the ceiling, and the search offers its start
  to its control as it makes it.  The trees are the same.
* **The window** (8462d2c).  `--window K` keeps K consecutive games in flight, each set up and searched on a thread of
  its own and concluded in order.  A search may start before the game before it is proven, seeded from that game's
  provisional game and without a ceiling, and gets both at that game's conclusion.  The threads are a budget shared by
  a pure policy (`allot`): every 5 ms the claims are served oldest game first (a setup one thread; a ready search every
  free thread if alone, else one; a busy search doubles), and a search idle 25% or more of a tick gives idle workers
  back.
* **Speculation made to pay** (af545ac, 3815e82).  Reclaims: the oldest search, left short, takes threads back from the
  younger ones, youngest first, down to none (suspended).  Hints: every tick a running search is offered the survivors
  of the game before's provisional game when its score has improved.  Deferral: a search whose seed's survivors reach
  the provisional ceiling waits for the game before's conclusion, which then settles it at once if that game was final.
* **Tools** (13804bb, e4bfc4e).  The ladder's scaling table and `wall`'s lines carry the counters (`µs/step`,
  `µs/tree`, `reb %`, `1st %`, `ramp %`, `tail %`); `chain.sh` runs with `WINDOW`; `TAXMAN_WINDOW_TRACE` prints the
  driver's events.

## Results

**The same chains.**  With a window of 1, and with any window on one thread, the chain 1..1300 is line for line that of
master (b24a38e) in every field but the times and the build.  On 4 threads with a window of 8, every score, method,
proof, ceiling, repeat, assumption and previous game of 1..1300 is `results/strict.jsonl`'s, and both verifiers accept
the file ([trust.md](../../docs/design/trust.md#what-a-window-changes-in-the-results) says why no proof or repeat can
differ).  Move lists differ only as between thread counts, by other optimal sets: 1215 in one run of the final build,
539 and 1136 (with the repeats 1137-1139) in a run of an earlier build, and 1215 in master's own 4-thread chain.

**What it buys, on 4 threads.**  The chain 1..1300 (`taxman regress --to 1300`, no results file), two runs each:

| window | 1 | 4 | 8 | 16 | 32 |
|---|---|---|---|---|---|
| wall s | 13.51, 13.42 | 12.26, 12.40 | 11.80, 11.49 | 11.41, 11.22 | 10.93, 11.09 |
| user s | 35.5, 35.5 | 37.0, 37.6 | 40.0, 38.5 | 39.7, 38.5 | 37.6, 38.4 |

With the results file (`solve 1-1300 --out`, whose lines are synced one by one, about 5 ms each on macOS), master in
order took 20.50 and 20.73 s, and window 8 17.73 and 17.34 s.  The stretch 2226..2249 (24 games: 11 repeats, 3 primes,
3 searches proven by the ceiling, 7 exhausted searches of 1 to 15 s) took 33.98 and 34.89 s in order, 34.06 and 35.35 s
at window 8, and 34.74 and 33.01 s at window 16, about 2,500 games an hour in every case.  Those searches already keep
4 threads busy (effective parallelism 98.0 to 99.9%), so a window has nothing to fill there.  It pays where searches are
short against the threads: 1..1300 on 4 threads, and at 64 and 192 threads most of the chain past 2175.

**How it got there** (window 8; in order, 2226..2249 took 33.5-35.6 s and 1..1300 13.4-15.1 s):

* The first window: 1..1300 took 15.26 s.  A search in its start doubled its threads at every pass of the policy, and a
  speculative search could start only once the game before's engine ran.
* The start offered to the control: 15.27 s.  Seeds came earlier, but the starts still took every thread, and in
  1201..1300 only 9 of 45 searches started speculatively.
* A search in its start doubles once a tick, with 10 ms to settle: 1..1300 went from 15.1 s to 12.7 s (12.42-12.91 s
  over the runs), with 39 of 45 searches speculative.  But 2226..2249 took 48.5, 48.7, 49.2 and 52.6 s.
* Reclaims alone: 13.07-13.91 s, not kept then (churn on the short searches).
* Reclaims, hints and deferral (af545ac): 2226..2249 took 34.8-42.0 s and 1..1300 11.04-11.58 s.  Without deferral the
  stretch took 46.7 and 52.0 s, without hints 42.3 and 43.1 s, without reclaims 44.0 and 42.5 s.
* Suspension (3815e82): 34.47 and 36.51 s, and 11.08-11.44 s, the oldest search on all its threads again.
* The final build (e4bfc4e): 34.06 and 35.35 s, and 11.49 and 11.80 s.

Unchecked speculation searched far more: 2233 took 77k nodes against 29k in order, 2235 and 2236 (which the ceiling
settles at once) 77k and 50k against none, and the stretch's searched games 943k in all against 736k-763k in order and
752k-779k in the final build.  Three causes, which reclaims with suspension, hints and deferral answer in that order:
a younger search kept its threads while the game the chain waits for ran on fewer (2232 on 3 of 4 for 12 s); it was
seeded once, from the game before's best at its start, and searched a tree several times larger until the final seed
came; and the games the ceiling settles at once were searched to exhaustion until it came.

**Tried and not kept.**

* Holding a younger search back while an older one is busy: 2226..2249 took 34.26 and 33.71 s, but 1..1300 took 14.54
  and 14.31 s against 11.1, since it serializes the short searches the window is for.
* Reclaims that leave a younger search one worker (before an engine could be suspended): 2233 held one of the 4 threads
  for 12 s while 2232 ran.  Suspension took the stretch from 40.6-41.2 s to 34.5-36.5 s.
* Counting the settling time (no idle workers given back within 10 ms of a grant) from an engine's first sample as
  well: the oldest search still gave threads back at its engine's start, and suspension alone gave the whole gain.

**Where a thread's time goes** (`ladder --threads 1,2,4 --seed-opt`, the proofs of 2172 and 1472).  At 2172, 1, 2 and 4
threads took 61.84, 36.01 and 15.27 s (speedups 1.72 and 4.05), and a step cost 120.51, 121.33 and 129.04 µs.  At 1472
they took 33.96, 19.11 and 9.92 s (1.78 and 3.42), and a step cost 109.40, 112.85 and 116.48 µs.  So on 4 threads a
step costs 6-7% more than on one, the size of Graviton's loss at 64.  None of it is the units' starts (0.2% of the
time), the ramp or the tail (0.1%) or the lock (0.00%): the step itself is dearer, by caches, memory or the nogoods
that a thread learns from fewer of its own steps.  The tree also changes with the threads: 2172 took 470k to 590k
nodes.

## Decision

Kept: the control, the window (default 1; `chain.sh` 16), the policy with reclaims, hints, deferral and suspension, and
the counters.  Not kept: speculation unchecked, reclaims alone, holding a younger search back, and counting the
settling time from an engine's first sample.

A unit that carries its node's heaviest common independent set and its weight split, so that a rebuilt GF(2) level is
checked by the split instead of solved from the uncontended prefix, is a design and was not built: on 4 threads it would
save under 0.2%.  [Scaling to 192 threads](../scaling-to-192-threads-2026-10/README.md) measured the units' starts at 64
and 192 threads.  The window's constants (a tick of 5 ms, busy below 5% idle, giving back at 25%, 10 ms to settle) were
measured at 4 threads only, and the cloud sweep (item 8) tunes them.  Three refinements wait on the cloud runs: hints
reach a speculative search as an offer, without the local search that a search started in order runs on its seed; a
suspended search can be restarted between ticks and suspended again at the next; and a younger search in its start
keeps one thread, because its local search runs on its own thread whatever it is told.

## Files

The folder holds only this README.md.  The code is `solver/frame/src/engine.rs` (the control and the counters),
`solver/frame/src/solver/window.rs` (the window and `allot`) and `solver/tools/cloud/chain.sh`.

## Commits

* 00d4b68: the engine's control and its counters.
* 3815e82: suspending a search (an engine whose control sets 0 workers keeps its units and waits).
* 9ca2c13: the search's ceiling through the engine's control.
* 8462d2c: the window of games in flight.
* af545ac: reclaims, hints and deferral.
* 13804bb: the ladder's scaling table and `wall`'s lines carry the new counters.
* e4bfc4e: `chain.sh` runs with `WINDOW`, 16 by default.
* b24a38e: master's build, the base of the comparisons (the merge of the prime-exchange rule).
