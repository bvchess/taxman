# The wall regime (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-14.  Concluded 2026-10-14.

**Question.**  The committed chain reaches 2175 with costs that barely grow, and the set widths stop at n = 4095.  Near
the wall trees will be large and n larger: what grows with n, and with the size of a tree, apart from parallel scaling
(another study's), and what should change first?  **Answer.**  A node costs +0.042 decades more per 100 n (379 µs at
2200 and 1,378 µs at 4000 on one thread), and the GF(2) intersection is 98% of it.  Nothing grows with the size of a
tree on a thread: memory, the nogood tables, the deque, checkpoints, the set widths and the S3 saves all stay small.
What else grows is serial: the fixed cost of a game (1.2 s at 2200 and 4.9 s at 4000 on average, 1.1 to 1.6 hours of one
thread over 2176..4095) and the results check at every start of the solver (10 minutes at 4095).  The design at the end
ranks nine changes, of which the first four leave every tree unchanged.

## Method

The measurements are on the M1 Max, one process at a time under `nice -n 10`, one thread unless said, with other jobs
running at load 4 to 5, and change nothing in the solver.  `examples/wall.rs` (commits 49c7917 and dddc5fd) has five
commands, and its module comment defines every field of the data:

* `setup`: the fixed costs of a game piece by piece, the sizes, the root's GF(2) bound and its gap, at any set width;
* `start`: the starting incumbent's local search, singles against pairs;
* `nodes`: a node-limited one-thread search from the start incumbent or proving the optimum, at any width, with
  checkpoints if asked;
* `grow`: a large tree with the engine's progress lines and a monitor of memory, checkpoint and spill; any tactics, any
  threads, emulated workers;
* `frames`: the frames of boards to 8191, at 128 words;

and `counters::max_rss`.  The optimum is unknown past 2276 (the committed chain ends at 2175, the Graviton calibration's
c8g chain at 2276), so past it the gaps are over the start incumbent, upper bounds of the true gap: the trend is what
matters.  `wall.py` makes the tables from the data (`wall_output.md`).

## Results

**1. A node against n.**  One thread, 5,119 steps from the start incumbent (no seed), every set at the width the solver
picks:

| n | W | frames | cands | members (words) | max_total | top rank | gap: start (opt) | µs/node | µs/aug. | augs/node |
|---|---|---|---|---|---|---|---|---|---|---|
| 1300 | 24 | 7 | 330 | 746 (12) | 79 | 55 (1) | 151 (151) | 113 | 5.1 | 22 |
| 1600 | 28 | 7 | 410 | 917 (15) | 96 | 67 (2) | 657 (349) | 158 | 5.9 | 27 |
| 1900 | 32 | 7 | 485 | 1088 (17) | 110 | 82 (2) | 290 (243) | 163 | 6.4 | 25 |
| 2170 | 64 | 8 | 572 | 1274 (20) | 130 | 88 (2) | 1170 (767) | 361 | 9.5 | 38 |
| 2200 | 64 | 8 | 571 | 1263 (20) | 132 | 90 (2) | 921 | 379 | 9.6 | 40 |
| 2500 | 64 | 8 | 670 | 1486 (24) | 151 | 103 (2) | 1123 | 457 | 11.1 | 41 |
| 2800 | 64 | 8 | 727 | 1630 (26) | 156 | 116 (2) | 774 | 431 | 9.2 | 47 |
| 3100 | 64 | 8 | 836 | 1824 (29) | 189 | 123 (2) | 1947 | 782 | 12.1 | 65 |
| 3400 | 64 | 8 | 916 | 1992 (32) | 207 | 136 (3) | 1964 | 876 | 17.1 | 51 |
| 3700 | 64 | 8 | 996 | 2183 (35) | 217 | 150 (3) | 2017 | 1689 | 18.2 | 93 |
| 4000 | 64 | 8 | 1079 | 2377 (38) | 230 | 165 (3) | 1316 | 1378 | 19.4 | 71 |
| 4095 | 64 | 8 | 1085 | 2383 (38) | 231 | 169 (3) | 1671 | 1131 | 16.3 | 69 |

(*Members* are the distinct numbers in the root's frames and candidates, the numbers the search ever touches.)  Proving
the optimum instead (`--mode opt`) a node costs the same: 113, 239, 173 and 392 µs at 1300, 1600, 1900 and 2170 (1.23 M
cycles at 2170, the committed chain's 389 µs).  Over 39 boards of 1300..4095 (`gaps.jsonl`) a node costs +0.042 decades
per 100 n (ten times per 2,400 n; the chain to 2175 had +0.045), half of it from the cost of an augmentation (+0.020:
larger blocks, 3 words of rank from 3400) and half from the augmentations a node makes (+0.021: larger heaviest sets,
`max_total` 130 to 230).  The root's bound is no cost: the matroids built in 2 ms and solved in 3 ms at 4000 (190
augmentations).  At 4000 (30 s of `sample`, `profile_4000_search.txt`) the GF(2) intersection takes 98% of the time
(`Meet::shortest_path` 69%, `Meet::add` 17%, the level's own work in `Gf2Bound::evaluate` 7%, the rest of `meet` 5%),
peels 0.7%, the step 0.7% and copies 0.2%; at 1287 the shortest paths were 57%.  The whole one-thread process takes 33
to 65 MB.

**The gap.**  Over the unseeded start the root's gap rises +57 per 100 n over the 39 boards, from 600-1200 at 2200-2950
to 1300-2600 at 3100-4095.  Where the optimum is known the unseeded start is 0 to 403 below it (mean 172; a seeded one
up to 472, at 2200), so the true gaps past 3100 are likely 1,000 or more, against at most 767 to 2175.  The true gaps of
the c8g chain's dear games past 2175 (`gaps_c8g.jsonl`, seeded starts) are close to 2170's, yet their trees are not:
2184 has gap 887 and 25.6 million nodes (64 threads), 2260 721 and 10.4 M, 2268 636 and 6.6 M, 2254 and 2256 713 and 5.3
M and 5.0 M, against 2170's 767 and 1.43 M (one thread).  The tenfold per 236 of gap of 1301..2175 predicts 4.6 M for
2184, so trees past 2175 grow faster than their root gaps say.

**2. Large trees.**  A looser incumbent does not make a large GF(2) tree: the proof of 1215 (10,371 steps) grows to
37,709 steps with the incumbent 200 below the optimum and saturates at 47,757 with 2,000 below, because the search finds
the optimum early.  So the large trees are real ones: the GF(2) proof of 1430 on one thread (706,620 steps), the
matching path's proof of 1230 (`gf2=off,lagrange=on`, 10.45 million steps, a tree of the wall's size at a tenth of the
cost a node), and 64 workers taking turns on one CPU at 2170 for 90 s (`EngineConfig::cores`):

| run | steps | s | µs/step: tenths | fifths | RSS (MB) | queue | checkpoint (KB) | pauses (longest; park, ms) |
|---|---|---|---|---|---|---|---|---|
| 1430, GF(2), 1 thr. | 706,620 | 116 | 153 to 175 | 160-170 | 42.3 to 42.7 | ≤ 33 | 5 to 14 | 11 (25.5; 0.7) |
| 1230, matching, 1 thr. | 10,450,026 | 144 | 12.6 to 15.2 | 13.5-13.8 | 42.6 to 43.8 | ≤ 34 | ≤ 11.6 | 14 (26; 0.12) |
| 2170, GF(2), 64 on 1 CPU | 207,356 | 90 | | | 334 | ≤ 1,705 | 1,400 to 1,800 | 8 |

Nothing grows with the tree on a thread: the cost of a step stays within ±7% from the first tenth to the last,
forgetting the nogoods every 50,000 steps leaves no sawtooth (±4% across the period), memory is flat after the first
seconds, the queue holds the path (one unit a node with candidates left), a checkpoint is a few KB, and its pause is the
fsync (macOS `F_FULLFSYNC`, 25 ms).  What grows is per worker: 4.5 MB a searcher at 2170 (64 workers 334 MB; c8g
measured 363 to 374 MB at 64 threads), and the queue after checkpoints and splits (1,705 units of 1 KiB at 64 workers).
At 4000 a searcher's state is larger (the stack 63 to 71 deep, levels over 1,079 elements of 3-word vectors): about 15
to 20 MB, so 3 to 4 GB at 192 threads, which no instance considered lacks.  Large trees are a question of parallel
scaling, not of a thread's state.

**3. The fixed costs of a game.**  Everything a game costs besides its search runs on one thread while the others wait.
Piece by piece (`setup`, one thread, seconds; the search's setup is the search stopped before its first node: matching
graph, seed, dive, local search, GF(2) graph, the engine started; unseeded past 2175, seeded below):

| n | board | earlier game's problem | local search | search's setup | moves | prime path |
|---|---|---|---|---|---|---|
| 1300 | 0.025 | 0.025 | 0.145 | 0.104 | 0.030 | 0.013 |
| 1900 | 0.073 | 0.073 | 0.377 | 0.229 | 0.099 | 0.039 |
| 2175 | 0.188 | 0.190 | 0.756 | 0.389 | 0.281 | 0.098 |
| 2500 | 0.263 | 0.263 | 1.834 | 1.858 | 0.456 | 0.136 |
| 3100 | 0.398 | 0.398 | 2.772 | 2.771 | 0.456 | 0.210 |
| 4000 | 0.710 | 0.693 | 6.420 | 6.449 | 1.250 | 0.364 |
| 4095 | 0.749 | 0.747 | 9.759 | 9.701 | 1.443 | 0.391 |

A prime game costs the prime path and two replays (7 ms at 4000); a repeat the board, the earlier game's problem set up
again, the moves and the replays; a searched game the board, the search's setup and the moves.  The model reproduces the
committed chain's 2001..2175 (a repeat 0.568 s against the 0.519 measured, a prime 0.087 against 0.081).  A game costs,
before its search, 0.11, 0.69 and 1.97 s (prime, repeat, searched) at 2200; 0.20, 1.26 and 3.25 at 3000; 0.38, 2.67 and
8.42 at 4000.  With the chain's mix past 1300 (13% prime, 42% repeat, 45% searched) the mean is 1.19 s at 2200, 2.01 at
3000, 4.94 at 4000 and 6.62 at 4095, and 1.6 hours of one thread over 2176..4095 (1.1 if a seeded search's setup costs
half the unseeded): the search's setup 0.97 h, the moves 0.33, the board 0.20, the earlier game's problem 0.10, primes
0.02.  On 192 threads a game's 5 s of fixed time at 4000 is 960 thread-seconds; on the c8g run past 2175 (64 threads)
the fixed time was 57 s of the 769 s of 101 games, plus 43 s of start-up.

Where it goes (60 s of `sample` of `setup` at 4000, `profile_4000_setup.txt`): peels 60% and reductions 12%, nearly all
in the local search; the board layer's own peel (`Apiary::count_freedoms`) 20%; promotions 4%; copies 2.5%.  The local
search (`Neighborhood::improve`) refills the incumbent without each member and then without each pair of members, until
nothing improves: its last pass alone is |I|²/2 refills of the whole state (26,000 at 4000).  On 175 games of 1301..2175
(`start.jsonl`, from the better of greedy and the seed's survivors) the singles take 2 to 5 ms and the pairs 0.09 to
0.37 s (70 to 80 times as long).  Of the 63 searched games among them, 16 start at the ceiling (the local search cannot
change anything: 3.7 of their 13.9 s of pairs were wasted), and in 16 of the 47 exhausted searches the pairs raise the
start after the singles found nothing (by 1 to 589).  The moves (`solution_for_promotions`, the board's frames rebuilt
with the promotions as sources) cost five times the game with no promotions (1.25 against 0.24 s at 4000).

The results check (`ResultsFile::open` at every start of `taxman solve` on a results file, and `verify-results`) sets a
search problem up again for every repeat line and the game it repeats (67% of the boards) and replays every game, on one
thread.  The model from `setup`'s times gives 53 s for the committed file (measured: 47.6 s on the laptop, 43 s on c8g),
104 s at 2500, 212 s at 3000, 362 s at 3500 and 614 s at 4095, paid again at each restart of `chain.sh` (a spot
interruption, a crash) with every other thread idle.

**4. The set widths, renumbering, and beyond 4095.**  Boards 2048..4095 run at W = 64 although they need 33 to 64 words
(2200 needs 35, 3000 47).  The same search at other widths makes the same tree (at 2170 for W = 36 to 64, at 1300 for 24
to 64, at 3000 for 48 to 64), and costs about the same: at 2170, 1,220k cycles a node at W = 36 and 1,225k at 64
(+0.4%), a fit of 0.20k cycles a word a node, 1.1% of a node at 64 words; at 1300 0.21k a word, 3.7% at 64 words; at
3000 0.6% more busy time at 64 than 48.  The fixed costs are another matter: at 2170, W = 36 against 64 takes 0.072
against 0.125 s for the frames, 0.150 against 0.281 for the moves and 0.057 against 0.099 for the prime path (at 3000,
W = 48 against 64: 0.181 and 0.250, 0.487 and 0.681, 0.147 and 0.201).  A repeat is 44% cheaper at 2170 (0.37 against
0.66 s with the earlier game's problem set up at the same width), a searched game 27%.

*Renumbering* would give the numbers the search touches dense indices, so that no set carries the bits of numbers out of
the game.  The search touches only the root's members: 1,274 numbers at 2175 (20 words, against 64), 2,383 at 4095 (38).
The work that depends on W is 1.1% of a node at 2170 and 3.7% at 1300, and under 2% at 4000, because a GF(2) node is the
matroid intersection, whose rows are already compact (`gf2.rs`, one to three words a block).  Renumbering would take
about 1% off a node near the wall, 7 to 9% off the local search, nothing off the board layer, and make units and
checkpoints three times smaller (1 KiB to 320 B a unit; 13 MB to 4 MB a checkpoint at 192 threads).  The design, if it
is ever wanted: an index in increasing order of the root's members, so that every iteration, `min`, `max` and tie-break
is the same and every tree is identical by construction (candidates first would change the order between a candidate and
a move, and so the trees); the search's width becoming words(members), where 64 words hold boards up to about 6,800.  It
is a refactor through `model`, `peel`, `gf2`, `nogoods`, `matchbound`, `search` and the probe, checked by the probe's
fingerprints.

*Beyond 4095* (`frames.jsonl`): 9 frames from 4096 (`MAX_FRAMES` is 8); the largest rank is 169 at 4095, 198 at 5000 and
261 at 6500, so the GF(2) bound's vectors need 4 words from about 4,800 and 5 from about 6,400 (`gf2::MAX_WORDS` is 3);
the members are 2,426 at 4096 and 5,012 at 8191 (38 and 79 words); the frames take 1.17 s at 4096 at 128 words against
0.50 at 4095 at 64.  It takes a width of 128 in `WIDTHS` and `with_words!` (about 6 s of build each), the peel's
`Worklist` summarizing more than 64 words, `MAX_FRAMES` 9, the GF(2) dispatch with R = 4 and 5, `MAX_N`, and
`tests/wide.rs` at 4096, 5000 and 8191.  A node at 128 words would cost about 1.3% more than at 64 by the width fit, the
board layer about twice as much.  [Boards past 4095](../../boards-past-4095-2026-10/README.md) built it.

**5. Checkpoints and the S3 saves.**  One thread at 4000 (`ckpt_4000.jsonl`, 5,119 steps from the start) is busy 7.05 s
without checkpoints.  With one every second, 8 pauses cost 5% more busy time (68 more unit rebuilds, about 5 ms each)
and 0.11 s of pauses (the longest 18.6 ms, park 4.8 ms); with an interval of 0, 77 pauses cost 13% (253 rebuilds, 3.6 ms
each) and 4.25 s, the longest pause 0.42 s with a park of 0.41 s: a step at 4000 can run 0.4 s (strong branching's exact
solves), against 0.7 ms at 1430 and 0.12 ms on the matching path.  A checkpoint of one thread at 4000 is 58 KB (63 to 71
units of 1 KiB: the path); 64 workers at 2170 wrote 1.4 to 1.8 MB.  At 192 threads near 4000, that is about 13 MB a
checkpoint, a pause costing every worker the slowest worker's step (up to 0.4 s) and its path's rebuilds (about 40 ms),
so about 0.7% of the machine at the chain's interval of 60 s and 0.15% at 300 s.  The real figure needs the machine
([the scaling run's "For a cloud machine"](../../scaling-to-192-threads-2026-10/README.md#for-a-cloud-machine), 3).

One `aws s3 sync` (aws-cli 2.38) costs 0.38 s of CPU (0.30 user, 0.08 system, 88 MB) before its first request.
`save.sh` makes three, and uploads the results file whole whenever it changed (7.7 MB then, about 27 MB at 4095) and the
checkpoints: about 1.5 s of CPU a save, 9 s an hour at the default `SAVE_EVERY` of 600 s, 0.0013% of a 192-vCPU machine
(0.016% of a 16-vCPU one; the prover's runs save every 300 s: twice that).  It squanders nothing; `nice -n 19` (and
`ionice -c 3` on Linux) on the saver costs nothing and keeps it out of the workers' way, and saving more often than the
checkpoint interval gains nothing.

## Decision

**What will dominate near the wall:**

* The search of the dear games: trees past 2175 are already 4 to 18 times the dearest of 1301..2175 at equal root gaps
  (2184: 25.6 million steps), a node is three to four times dearer at 4000 than at 2200, and 98% of a node is in the
  GF(2) intersection (shortest paths 69%).  That is the parallel study's and the inner loop's ground.
* For everything else, the serial fixed costs: 1 to 7 s a game (1.1 to 1.6 hours of one thread to 4095, idling every
  other thread), most of it the start's local search, and the results check at every start of the solver (1 to 10
  minutes).  On a machine of 64 to 192 threads, these decide how many cheap games an hour it makes.
* Not the width of the sets (1% of a node), memory (tens of MB a searcher), the nogood tables, checkpoints (a few MB,
  under 1% of the time at 60 s) or the S3 saves.

**The design: what to change, in order.**

1. **Skip the dive and the local search when the start already reaches the ceiling**, since nothing can beat it.  Gain
   at 3000-4000: the search setup of a game at its ceiling, 2.6 to 6.4 s (20% of searched games past 1300), 0.2 h of
   2176..4095.  Trees identical; no risk.
2. **The local search on all threads**: the refills of one pass are independent, so compute them in parallel and take
   the first improvement in the pass's order.  The search's setup falls from 6.4 s to well under 0.1 s at 192 threads,
   0.5 to 0.8 h of 2176..4095.  Trees identical; low risk.
3. **The results check on all threads**, or of the new lines only behind a checked prefix: 3.5 to 10 minutes become
   seconds at every start of the solver.  No search is involved; low risk.
4. **Keep the search problems of the games of this run**, so that a repeat compares in memory, and skip the replay of a
   previous game this run made: 0.69 s of every repeat at 4000, 0.1 h of 2176..4095.  Trees identical; no risk.
5. **Set widths of 36, 40, 48 and 56 words** (2048..3583 pay 64 today): the board layer 28 to 44% cheaper (a repeat 0.66
   to 0.37 s at 2170, 1.44 to 1.03 at 3000), a node 0.4 to 0.6%.  Trees identical; no risk.
6. **The moves**: fix why the promotions make the board five times dearer to play, or overlap the next game's board and
   problem with the current search on a spare thread: 1.25 s of every game at 4000 (0.33 h), the board 0.7 s (0.2 h).
   Trees identical; low risk.
7. **Checkpoints every 300 s near the wall**, and `nice -n 19` and `ionice -c 3` on the S3 saver: 0.7% falls to 0.15% of
   the machine (an estimate to confirm on the machine).
8. **Boards past 4095**: a width of 128, `MAX_FRAMES` 9, a two-word worklist summary, the GF(2) dispatch to R = 5.  This
   makes them possible, at a node about 1.3% dearer at 128 words; trees identical below 4096; low risk.
9. **The renumbering**: about 1% of a node, 7 to 9% of the local search (gone with 2), smaller checkpoints.  Trees
   identical by construction; low risk.

Build first 1 to 4, in one round with no tree changed anywhere (the one-thread node counts and the probe's fingerprints
must not move), measured by the chain's games an hour on the cloud machine; then 5 if 2048..3583 is where the chain is.
Item 9 is not recommended: it is a large refactor for about 1% of a node.  Not in this study: the inner loop (the GF(2)
intersection, 98% of a node) and parallel scaling, which the dear games past 2175 will need most.

## Files

This directory holds the data that `examples/wall.rs` wrote and `wall.py`, which makes the entry's tables from them
(`./wall.py > wall_output.md`; the output is `wall_output.md`).  Everything ran on the M1 Max, one process at a time
under `nice -n 10`, one thread unless a line says otherwise, with other jobs on the machine (load 4 to 5), on the code
of this branch at the merge of master 5252d5d (9c99ca1).  Times are wall seconds of one thread; the cycles and
instructions of the `nodes` lines are the process's, and those of the start mode include the search's own setup.

### Files and the commands that made them

`wall` is `target/release/examples/wall` (`nice -n 10 cargo build --release -j 4 --features studies --example wall`),
run from `rust/`; `K` is `--known results/strict.jsonl`.

* `setup.jsonl`: `wall setup K 1000 1300 1600 1900 2100 2175 2200 2500 2800 3100 3400 3700 4000 4095`; the fixed costs
  of a game piece by piece, the sizes, the root's GF(2) bound and the gap.
* `setup_widths.jsonl`: `wall setup K --repeat 2 --width W N` for N = 2170 (W = 36, 48, 64) and 3000 (48, 64); the same
  at other set widths.
* `start.jsonl`: `wall start K 1301 1306 ... 2171` (every fifth board); the starting incumbent's local search, singles
  against pairs.
* `nodes_start_low.jsonl`, `nodes_opt_low.jsonl`: `wall nodes K --no-seed --nodes 5000 1300 1600 1900 2170`, and with
  `--mode opt`; the cost of a node from the start incumbent and in the proof of the optimum.
* `nodes_start_high.jsonl`: `wall nodes --no-seed --nodes 5000 2200 2500 2800 3100 3400 3700 4000 4095`; the cost of a
  node past 2175.
* `gaps.jsonl`: `wall nodes K --no-seed --nodes 1000 1402 1502 ... 2102` and `2252 2352 ... 4052`; root gaps over the
  unseeded start (and the optimum where known), and node costs, every 100.
* `gaps_c8g.jsonl`: `wall nodes --known KNOWN --nodes 1000 2184 2185 2190 2200 2220 2240 2250 2254 2256 2260 2268 2275`,
  `KNOWN` being `results/strict.jsonl` followed by `../cloud/c8g.chain-2176.jsonl`; the root gaps of the dear games past
  2175, against their optima (seeded starts).
* `gaps_c8g_rest.jsonl`: the same with `--nodes 1` for the other 33 searched games of 2176..2276 (2176 2178 2180 2187
  2192 2193 2196 2205 2208 2210 2211 2212 2214 2216 2223 2224 2225 2226 2232 2233 2235 2236 2241 2244 2247 2248 2255
  2259 2261 2262 2264 2265 2272), made with a later build of `wall` (2026-10-14); with `gaps_c8g.jsonl`, the root gap of
  every searched game of the c8g chain, for the fit of [the root gaps of the c8g
  chain](../measurements-made-for-the-research-agenda-2026-10/README.md#71-the-root-gaps-of-the-c8g-chain).
* `widths_1300.jsonl`, `widths_2170.jsonl`: `wall nodes K --mode opt --nodes 5000 --width W N`, each width twice; the
  same proof at several set widths.
* `widths_3000.jsonl`: `wall nodes --no-seed --nodes 5000 --width W 3000`, W = 48, 56, 64, twice; the same, past 2175.
* `ckpt_4000.jsonl`: `wall nodes --no-seed --nodes 5000 --ckpt-dir DIR --every 0 4000`, then `--every 1`; what
  checkpoints cost a thread at 4000.
* `grow_1430.jsonl`, `grow_1430.progress`: `wall grow K --delta 0 --nodes 1000000 --progress 2 --ckpt-dir DIR --every 10
  1430`; a GF(2) proof of 706,620 nodes: monitor lines and summary (stdout), progress lines (stderr).
* `grow_1230_matching.jsonl`, `grow_1230_matching.progress`: `wall grow K --tactic gf2=off --tactic lagrange=on --nodes
  15000000 --progress 2 --ckpt-dir DIR --every 10 1230`; a matching-path proof of 10.45 million nodes.
* `grow_2170_t64c1.jsonl`, `grow_2170_t64c1.progress`: `wall grow K --threads 64 --cores 1 --seconds 90 --progress 10
  --ckpt-dir DIR --every 10 2170`; 64 workers taking turns on one CPU: queue, checkpoint size, memory.
* `frames.jsonl`: `wall frames 1024 2048 3000 4095 4096 4500 5000 5500 6000 6500 7000 7500 8000 8191`; the frames, sizes
  and ranks beyond the widest set.
* `profile_4000_search.txt`, `profile_4000_search.jsonl`: macOS `sample` (1 ms) for 30 s of
  `wall nodes --no-seed --nodes 30000 4000`, attributed by `solver/tools/profile/attribute.py`; where a node's time goes
  at 4000.
* `profile_4000_setup.txt`: the same for 60 s of `wall setup --repeat 3 4000`; where the fixed costs go at 4000.
* `wall.py`, `wall_output.md`: `./wall.py > wall_output.md`; the tables of this entry, from the data.

In the two profiles `attribute.py` could not symbolicate the inlined frames of this binary, so its kinds and purposes
say `other`; the innermost functions are the useful part, as truncated symbols (`...Meet...13s` is
`Meet<3>::shortest_path`, `...3ad` `Meet<3>::add`, `...8eva` `Gf2Bound<64>::evaluate`, `...6Apiary...14cou` the board
layer's `count_freedoms`).

## Commits

* 49c7917 (2026-10-14): `examples/wall`, the wall regime's measurements.
* dddc5fd (2026-10-14): `--width` for `setup`, `--tactic` for `grow`, and the command `start`.
* 5252d5d (2026-10-14): the merge of master that the data ran on (9c99ca1 the branch's merge).
