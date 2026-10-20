# Round A: the fixed costs of a game (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-14.  Concluded 2026-10-14.

**Question.**  Everything a game of the chain does besides its search runs on one thread while the others wait, and it
grows with n.  [The wall regime](../the-wall-regime-2026-10/README.md) put it at 1.2 s a game at 2200 and 4.9 at 4000,
and the results check at 55 s for each start of the solver at 2225 games and about 10 minutes at 4095; the profile of
2126-2135 in [the inner loop of the GF(2) search](../the-inner-loop-of-the-gf2-search-2026-10/README.md) found the same
parts: the move list from the promotions 45% of a game's samples, the frames 22%, the local search on the start 30%, the
repeat's search problem 8%, and the start-up's check of 2125 games 43 s on Graviton4.  Can they be cut with every tree,
result and move list unchanged?  **Answer.**  Yes: from 2200 to 4095 the board layer is 13 to 15 times faster and the
moves from the promotions 56 to 83 times, a repeat takes 0.02 s a game instead of 0.78, and 2,225 games check in 9.8 s
instead of 55.7 on one thread.  The fixed costs of the games 2200..4095 go from 1.48 hours of one thread to 0.69.

## Method

Seven changes, each a pure speedup: the same frames, trees and games.

1. *No dive or local search at the ceiling* (fd2c67f).  When the best of the greedy set and the seed's survivors reaches
   the ceiling (`cap`), nothing can beat it, so the dive is skipped, and the local search when the incumbent after the
   dive reaches it: neither could change it while the ceiling holds, as the proof by the ceiling assumes.
2. *The local search on the search's threads* (fd2c67f).  `Neighbourhood::improve_on`: the refills of one pass are
   independent, so the threads take the moves of a pass in order from one counter and the improvement with the lowest
   number is kept: the sequential answer (tested against the nested loops at 0 to 4 threads on four boards, from the
   greedy set and the seed).
3. *The results check on the run's threads* (239afd3).  The repeats' search problems are set up first, side by side; the
   checks then run in order on one thread, with the same result and the same first error (`verify-results --threads`;
   `taxman solve` uses its own threads when it opens its results file).
4. *Repeats compared from memory* (fd2c67f).  `solver::Earlier` keeps the problems of the boards the run set up (the
   last eight), so a repeat no longer sets the earlier game's board up again; and a previous game the run made is not
   refereed a second time (`conclude` already replayed it).
5. *Set widths of 36, 40, 48 and 56 words* (acf964e).  Boards 2048-2303, 2304-2559, 2560-3071 and 3072-3583 run at those
   widths instead of 64 (the same trees at both, `tests/wide.rs`); a release build of the binaries takes 74 s instead of
   50.  A checkpoint of a game of 2048..3583 made at 64 words is refused by this build.
6. *The board layer* (f7e8adb).  The frames' peel (`count_freedoms`) works on a graph of the frame's own moves and
   factors, numbered by position, instead of sets of the board's width, and reuses its counters instead of zeroing 16 KB
   a call; the factor and composite tables are lists; each frame remembers which numbers it can take on and whether
   `removeImpossible` has been through it; `unzip` asks each question once.  The same steps in the same order, checked
   against the old code by a dump of every frame field, the candidates and the games, on every board 2..2400, every
   211th to 4095, all 1,836 searched or repeated games of the committed chain, random promotion sets and 768 punctured
   boards; and by `tests/board_layer.rs`: for every 25th searched game of the committed chain the moves its promotions
   make and its search problem's digest are the line's.
7. *Cloud settings* (acf964e).  `chain.sh` checkpoints every 300 s by default; `run.sh` runs every S3 save under
   `nice -n 19` (and `ionice -c 3` on Linux).

## Results

**The same trees.**  `cargo test --release` (all 17 test binaries) and the prover's tests pass, with clippy and fmt
clean; the probe's fingerprints 7e8e724717d26545 and f3deaac0dbdc030a (`gf2=on`) and the tripwires are unchanged;
`taxman regress --to 1000` (one thread) has every score, in 8.9 s.  The chain 1..1300 on one thread is identical line
for line (moves, promotions, nodes, proofs, setups) to the same chain by the build before this round (402,205 nodes;
40.8 s of wall time against 58.7), its scores, methods and proofs are those of `results/strict.jsonl` (move lists the
same but at 1215, another optimal set), and both referees (`solver/tools/cloud/verify.sh`) accept it.  The chain
2226..2249 below is identical line for line before and after.

**The gain on one thread** (`wall setup`, seconds, before / after):

| n | board | the earlier game's problem | moves | prime path | local search | the search's setup |
|---|---|---|---|---|---|---|
| 2200 | 0.195 / 0.015 | 0.197 / 0.015 | 0.281 / 0.005 | 0.102 / 0.009 | 1.192 / 1.046 | 1.447 / 1.304 |
| 3000 | 0.365 / 0.027 | 0.366 / 0.027 | 0.650 / 0.010 | 0.191 / 0.016 | 2.430 / 2.308 | 2.451 / 2.333 |
| 4000 | 0.677 / 0.048 | 0.676 / 0.047 | 1.205 / 0.016 | 0.354 / 0.029 | 6.169 / 6.131 | 6.226 / 6.189 |
| 4095 | 0.727 / 0.049 | 0.726 / 0.048 | 1.419 / 0.017 | 0.381 / 0.030 | 9.313 / 9.265 | 9.370 / 9.320 |

**The chain 2226..2249** on one thread (behind a copy of the committed chain extended with the c8g chain's lines to
2225; 24 games: 11 repeats, 3 primes, 3 searches ended by the ceiling, 7 exhausted searches), seconds a game outside the
search:

| | repeat | prime | search at the ceiling | exhausted search | all games | the run (wall) |
|---|---|---|---|---|---|---|
| before | 0.775 | 0.117 | 1.000 | 0.566 | 140.0 | 198.8 |
| after | 0.021 | 0.010 | 0.022 | 0.021 | 119.0 | 129.3 |

The start-up (the run less its games) went from 59 s to 10 s, and the exhausted searches' own seconds fell 4.5%, with
the same nodes (sets of 36 words, the board's share of the search's setup, and the noise of a busy machine).  The
results check of those 2,225 games takes 55.7 s before and 9.8 s after on one thread (the boards set up again cost a
fourteenth), and 3.9 s on 4 threads.  The local search on 4 threads: the search's setup and first nodes of 4000
(standalone, `--threads 4 --node-limit 1`) take 6.8 s before and 2.2 s after.  By the study's model, the fixed costs of
the games 2200..4095 go from 1.48 hours of one thread to 0.69 (one thread), 0.20 (4 threads) and 0.04 (64, if the
refills scale); a game at 4000 costs 0.04 s (prime), 0.07 (repeat), 0.07 (searched, at its ceiling) and 0.07 plus the
local search on the run's threads (searched).

## Decision

All seven changes are kept, as a pure speedup.  What is left: the local search's refills (5 s of one thread a searched
game at 4000; on many threads a share of a second, though a pass of a few moves on 192 threads pays more to spawn them
than it saves), the start-up's check (10 s at 2225 games on one thread, a few on many), and the serial order of the
games, which the next round pipelines ([round B](../../round-b-the-chain-on-many-threads-2026-10/README.md)).

[Scaling to 192 threads](../../scaling-to-192-threads-2026-10/README.md#for-a-cloud-machine) measured the fixed costs on
Graviton (item 1) and the checkpoint's pause at 300 s (item 3).  Not measured: an S3 save's CPU under `nice`, the local
search's speedup at 64 and 192 threads, which that run did not separate from the search, and a profile-guided build
(`solver/tools/pgo.sh` now trains a game of every width).

## Files

The folder holds this README.md only.

## Commits

* fd2c67f (2026-10-14): no dive or local search at the ceiling, the local search on the search's threads, repeats
  compared from memory.
* 239afd3 (2026-10-14): the results check sets the repeats' search problems up on the run's threads first.
* acf964e (2026-10-14): set widths of 36, 40, 48 and 56 words for boards 2048..3583; the chain's checkpoints every 300
  s; the S3 saves at low priority.
* f7e8adb (2026-10-14): the board layer 11 to 70 times faster, the same frames and games.
