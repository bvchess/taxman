# Round B: the chain on many threads (2026-10)

Part of the [lab notebook](../README.md).

*Question.*  Round A made the serial part of a game small (a repeat 0.02 s, a searched game's setup 0.02 s plus its
local search on all threads).  On a machine of 64 to 192 threads the chain must still keep its threads busy: game n + 1
needs only n's promotions and score, so it can be set up while n searches and searched before n is proven; many
searches take milliseconds to a second, whose ramp-up and tail idle most of 64 threads; and the proof of 2172 scaled
58.5 times on 64 threads (Graviton calibration), most of the loss a node rate a thread of 93-94%.  Three parts, in the
inner loop's design ("Many threads: what the measurements say, and a design", items 1 to 3): pipeline the chain, with
the provenance of a chain solved in order; run short games side by side, moving threads between their searches; and
make a run report what a thread loses at many threads.  Measured on the M1 Max at up to 4 threads (`nice -n 10`, other
jobs keeping the load at 3 to 6, so a few per cent of noise); the many-thread measurements are commands for the cloud.

*What was built.*

1. *A control on an engine run* (00d4b68; suspension in 3815e82).  `engine::Control`, attached with
   `EngineConfig::control`, steers a run from another thread: a ceiling, given before or while it runs, ends the run
   once the incumbent reaches it (an offer stores the incumbent and then loads the ceiling, and giving the ceiling
   stores it and then loads the incumbent, all `SeqCst`, so one of them sees the other); incumbents offered from
   outside; a stop like a limit's; and the number of workers, which the coordinator raises by starting workers and
   lowers by retiring workers that hold no unit (a running one beyond the number hands all its units to the deque at its
   next poll), down to 0, which suspends the run with every unit in its deque.  A control remembers what it is told, so
   a request made before the run applies when it starts.  Without a control a run is unchanged.  The counters (per
   invocation, not in checkpoints): the capacity (the workers' lifetimes, wall × threads when their number is fixed,
   which `share` and the effective parallelism divide by), the most workers alive, those added and retired, the idle
   time of the ramp (before all first ran at once) and of the tail (after the last moment a worker finished a unit while
   all ran), and the units' starts: from a unit's start to its first poll (the searcher's rebuild) and from its first
   poll to its second (its first node, the GF(2) bound solved from scratch), found by one test of a byte in `poll`.  The
   progress line shows ramp, tail, rebuild and first.
2. *The search through the control* (9ca2c13, and in 8462d2c).  The ceiling is the engine's: `SearchSpace::cap` and the
   searcher's check went, and `search` gives its cap to its control, so a ceiling can come while the search runs.  The
   start (greedy set and seed, dive, local search) is offered to the control as it is made, so whoever steers the
   search has its best so far before the engine starts; the local search reads its threads and the ceiling before each
   pass.  The same trees: the probe's fingerprints 7e8e724717d26545 and f3deaac0dbdc030a, the tripwires.
3. *The window* (8462d2c; `solver::window`).  `--window K` keeps K consecutive games in flight: setup on a thread of the
   game's own, the repeat lookup in the order of n (a game of the window that set a search up counts), the search on
   the game's thread steered through its control, and the conclusion on the driver's thread, in order.  A search may
   start before the game before it is proven, seeded from its provisional game and without a ceiling, and gets both at
   that game's conclusion.  The threads are a budget shared by a pure policy (`allot`): claims served oldest game
   first every 5 ms (a setup one thread; a ready search every free thread if alone, else one; a busy search doubles);
   a search idle 25% or more of a tick gives idle workers back to another claim.  `TAXMAN_WINDOW_TRACE` prints the
   driver's events.  DESIGN.md (sections 7 and 8) has the design and why the results are those of a chain in order.
4. *Speculation made to pay* (af545ac, 3815e82): reclaims (the oldest search, left short, takes threads back from the
   younger ones, youngest first, down to none: suspended); hints (every tick a running search is offered the survivors
   of the game before's provisional game when its score has improved); deferring (a search whose seed's survivors reach
   the provisional ceiling, n + the provisional game's score − `in_the_bag`, waits for the game before's conclusion,
   which then settles it at once if the provisional game was final).
5. *Tools* (13804bb, e4bfc4e): the ladder's scaling table has `µs/step` (busy time a step), `µs/tree` (the same without
   the units' starts), `reb %`, `1st %`, `ramp %` and `tail %`; `wall`'s lines have the new counters; `chain.sh` runs
   with `WINDOW` (16 by default); `solver/notebook/search/the-wall-regime-2026-10/README.md`, "For a cloud machine", items 7 (where a step's time goes at 64
   and 192 threads) and 8 (the window at 64 and 192 threads).

*The same chains.*  With a window of 1, and with any window on one thread, the chain 1..1300 is line for line that of
master (b24a38e, the build with the prime-exchange rule) in every field but the times and the build: window 1 and window
8, 0 lines differ.  On 4 threads with a window of 8, every score, method, proof, ceiling, repeat (`same_search`),
assumption and previous game (n, score, proof) of 1..1300 is `results/strict.jsonl`'s, and `taxman verify-results` and
`solver/tools/check_results.py` accept the file (the two steps of `solver/tools/cloud/verify.sh`, run with 4 threads); no line's
proof or repeat differs, as the argument of DESIGN.md says none can (a completed search's sum is its problem's best, and
its proof is `cap` exactly when that best reaches the ceiling).  Move lists that differ (other optimal sets, as between
thread counts): 1215 in one run and none in another of the final build; 539 and 1136 (with the repeats 1137-1139 that
play 1136's promotions) in a run of an earlier build; master's own 4-thread chain differs at 1215.  On 2226..2249 the
window's lines have master's scores, methods, proofs, repeats and ceilings.

*What it buys* (4 threads).  The chain 1..1300 (`taxman regress --to 1300`, no results file), seconds of wall time,
two runs each, alternated:

| window | 1 | 4 | 8 | 16 | 32 |
|---|---|---|---|---|---|
| wall s | 13.51, 13.42 | 12.26, 12.40 | 11.80, 11.49 | 11.41, 11.22 | 10.93, 11.09 |
| user s | 35.5, 35.5 | 37.0, 37.6 | 40.0, 38.5 | 39.7, 38.5 | 37.6, 38.4 |

With the results file (`solve 1-1300 --out`, whose lines are synced one by one, about 5 ms each on macOS): master in
order 20.50 and 20.73 s, window 8 17.73 and 17.34 s.  Games 2226..2249 behind a copy of the committed chain and the c8g
chain's lines to 2225 (24 games: 11 repeats, 3 primes, 3 searches proven by the ceiling, 7 exhausted searches of 1 to
15 s; about 4 s of start-up, the results file's check): master in order 33.98 and 34.89 s, window 8 34.06 and 35.35 s,
window 16 34.74 and 33.01 s, so about 2,500 games an hour in every case (24 games over the wall time).  Those searches
already keep 4 threads busy (effective parallelism 98.0 to 99.9%, full occupancy after 20-25 ms), so a window has
nothing to fill there; it pays where searches are short against the threads: 1..1300 on 4 threads here, and at 64 and
192 threads most of the chain past 2175, which is what the cloud commands measure.

*How it got there* (window 8, 4 threads; in order 33.5-35.6 s and 13.4-15.1 s):

| build | 2226..2249, s | 1..1300 (regress), s | what changed |
|---|---|---|---|
| the first window (before the two fixes below, both in 8462d2c) | | 15.26 | a search in its start doubled its threads at every pass of the policy, and a speculative search could start only once the game before's engine ran |
| the start offered to the control, the local search stopping at the ceiling | | 15.27 | seeds earlier, but the starts still took every thread: in 1201..1300 9 of 45 searches started speculatively (`TAXMAN_WINDOW_TRACE`) |
| a search in its start doubles once a tick, 10 ms to settle | 48.5, 48.7, 49.2, 52.6 | 12.42-12.91 | 39 of 45 speculative; but on the stretch the speculative searches did several times the work |
| reclaims alone | | 13.07-13.91 | not kept then: churn on the short searches |
| reclaims, hints, deferring (af545ac) | 34.8-42.0 (6 runs) | 11.04-11.58 | without deferring 46.7/52.0, without hints 42.3/43.1, without reclaims 44.0/42.5 |
| suspension (3815e82) | 34.47, 36.51 | 11.08-11.44 | the oldest search on all its threads again |
| final (e4bfc4e) | 34.06, 35.35 | 11.49, 11.80 | |

Nodes of the stretch's searched games (in order / unchecked speculation / final, window 8): 2232 413k-439k / 417k /
438k-447k; 2233 29k / 77k / 47k; 2235 and 2236 0 / 77k and 50k / 0 and 0; 2240 103k-105k / 170k / 97k-102k; 2241 61k /
188k / 66k-68k; 2244 32k-40k / 88k / 31k-57k; in all 736k-763k / 943k / 752k-779k.  Speculation unchecked cost three
ways: a younger search kept its threads while the game the chain waits for ran on fewer (2232 on 3 of 4 for 12 s); it
was seeded once, from the game before's best at its start, often only that game's start, and searched a tree several
times larger with the weak incumbent until the final seed came; and the games the ceiling settles at once (their seed's
survivors reach it) were searched to exhaustion until the ceiling came.  Reclaims with suspension, hints and deferring
answer the three.

*Tried and not kept.*

* Holding a younger search back while an older one is busy: 2226..2249 34.26 and 33.71 s, but 1..1300 14.54 and 14.31 s
  against 11.1: it serializes the short searches the window is for.
* Reclaims that leave a younger search one worker (before the engine could be suspended): 2233 held one of the 4
  threads for 12 s while 2232 ran; suspension took the stretch from 40.6-41.2 s to 34.5-36.5 s.
* Counting the settling time (no idle workers given back within 10 ms of a grant) from an engine's first sample as
  well, so that its first splits are waited for: the oldest search still gave threads back at its engine's start (an
  engine's first split rounds take about 10 ms each at 2232, 64 polls of 150 µs), and suspension alone gave the whole
  gain; the policy would need the engine's own full occupancy in `Status`.

*Where a thread's time goes* (part 3; `ladder --threads 1,2,4 --seed-opt`, the proofs of 2172 and 1472):

| game, threads | search s | speedup | nodes | µs/step | µs/tree | eff % | idle % | reb % | 1st % | ramp % | tail % | runs |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2172, 1 | 61.84 | 1.00 | 510,340 | 120.51 | 120.50 | 100.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 1 |
| 2172, 2 | 36.01 | 1.72 | 590,378 | 121.33 | 121.25 | 100.0 | 0.0 | 0.0 | 0.1 | 0.0 | 0.0 | 50 |
| 2172, 4 | 15.27 | 4.05 | 469,648 | 129.04 | 128.84 | 99.9 | 0.1 | 0.0 | 0.2 | 0.1 | 0.0 | 131 |
| 1472, 1 | 33.96 | 1.00 | 309,591 | 109.40 | 109.40 | 100.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 1 |
| 1472, 2 | 19.11 | 1.78 | 337,641 | 112.85 | 112.75 | 99.9 | 0.0 | 0.0 | 0.1 | 0.0 | 0.0 | 71 |
| 1472, 4 | 9.92 | 3.42 | 339,455 | 116.48 | 116.28 | 99.9 | 0.1 | 0.0 | 0.2 | 0.1 | 0.0 | 152 |

On 4 threads a step costs 6-7% more than on one (the size of Graviton's loss at 64), and none of it is the units'
starts (0.2%), the lock (0.00%), the ramp or the tail (0.1%): the step itself is dearer, by caches, memory or the
nogoods that a thread learns from fewer of its own steps (the tree also changes with the threads: 2172 470k to 590k
nodes).  At 64 and 192 threads the units are more and the trees per thread smaller, so the units' starts may matter
there: item 7 of the cloud commands measures it.  The fix for them, if they do, is a design for after the measurement:
a unit that carries its node's heaviest common independent set and its weight split (in memory; spill and checkpoints
would keep only the two bitsets), so that the rebuilt GF(2) level is checked by the split (every element's reduced cost
non-negative) instead of solved from the uncontended prefix; with the same set the tie-breaking is the original
searcher's, so trees stay close to one thread's, and a gate (a tactic) keeps one-thread trees identical when it is off.
Not built: on 4 threads it would save under 0.2%.

*What is left.*  The window's policy was measured at 4 threads only; its constants (a tick of 5 ms, busy below 5% idle,
giving back at 25%, 10 ms to settle) are for the cloud sweep to tune (item 8).  Hints reach a speculative search as an
offer, without the local search a search started in order runs on its seed; a suspended search can be restarted
between ticks and suspended again at the next (the trace shows it); a younger search in its start keeps one thread
(its local search runs on its own thread whatever it is told).  Each is a possible refinement once the cloud runs show
which matters.
