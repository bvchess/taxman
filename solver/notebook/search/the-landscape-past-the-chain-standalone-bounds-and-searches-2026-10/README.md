# The landscape past the chain: standalone bounds and searches (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-17.  Concluded 2026-10-17.

**Question.**  With holes allowed in the chain ([the sweep](../../the-sweep-a-local-test-2026-10/README.md)), can each
game's standalone bounds, taken with no game of n − 1 (no seed, no ceiling, no prime formula), tell cheaply which games
past 3440 are easy, and can those be proven one by one?  **Answer.**  No: no game's bounds meet, neither the 1,265
games of 2176-3440 nor the 4,751 of 3441-8191, and the smallest standalone gap past 3440 is 1,196, where games of that
gap took 1.5 to 30 minutes of one thread on 2176-3440.

## Method

The two ends of a game's standalone bounds are `upper`, `in_the_bag` plus the root's GF(2) bound, which the search
computes at its first node, and `lower`, the game the search starts from (the better of the greedy set and the matching
dive, improved by the local search).  Their difference is the standalone gap, and a game whose ends meet is settled at
its first node.  Where the optimum is known, the true root gap is `upper` less the optimum, the heuristic's shortfall is
the optimum less `lower`, and the standalone gap is their sum.

One `c8g.16xlarge` at spot ran three stages one after the other on its 64 threads, on 2026-10-17 from 08:53 to 10:33
PDT, at the code of commit 6f57399, for about $1.10 (approved up to $2):

1. **Calibration**, 56 s: `wall landscape 2176-3440 --threads 64 --known results/strict.jsonl`, the bounds of the 1,265
   games the committed chain has past 2175, each beside its known optimum.
2. **Standalone searches**, 30 minutes: 64 games of 2196-3440 (`sample.txt`; standalone gaps from 404 to 2,546, median
   1,100 against the 1,265 games' 1,062; the chain searched 48 of them, settled 12 with the prime formula and 4 at their
   ceiling, and none as a repeat), all at once, one thread each, `taxman solve N --threads 1 --time-limit 1800` with
   nothing of n − 1 in the file.
3. **The map**, 62 minutes (65.1 core-hours): `wall landscape 3441-8191 --threads 64`, the bounds of all 4,751 games
   past the end of the chain, with no optimum to compare.

Beside them, on the laptop, a quick measurement of when a standalone search finds its best game (`ttb/`): 60 games of
1301-2175, one thread, the non-release build of the ladder (`cargo build --profile test --example ladder`), no seed, no
ceiling, 300 s at most.

## Results

### 1. The calibration, 2176-3440

No game's bounds met: 0 of the 1,265 (the run took 0.95 core-hours, a median 2.08 s of CPU a game and 8.98 s at most).
The heuristic ends short of the optimum by a median 227, and by 0 in 161 of the 1,265 games:

| | median | p10 | p90 | mean |
|---|---|---|---|---|
| the heuristic's shortfall (optimum less `lower`) | 227 | 0 | 782 | 329.5 |
| the true root gap (`upper` less the optimum) | 815 | 595 | 1,227 | 859.8 |
| the standalone gap (`upper` less `lower`) | 1,062 | 686 | 1,906 | 1,189.2 |

By band of n the medians are:

| n | games | shortfall | true root gap | standalone gap |
|---|---|---|---|---|
| 2176-2675 | 500 | 237 | 748 | 1,030 |
| 2676-3175 | 500 | 146 | 776 | 908 |
| 3176-3440 | 265 | 523 | 1,177 | 1,699 |

So the shortfall is not a constant: it was 146 to 237 and is 523 over the last 265 games, where no standalone gap is
under 1,000 (221 of the first 500 and 314 of the second 500 were).  The CPU of a game's bounds has a median of 1.6 s,
2.3 s and 5.0 s in the three bands.

How well each gap predicts the cost, over the committed chain's 448 exhausted searches of 2176-3440 (log10 of the nodes
the chain's search took against the gap, least squares):

| gap | a decade of nodes per | r | residual sd |
|---|---|---|---|
| the true root gap | 259 | 0.88 | 0.51 decades |
| the standalone gap | 579 | 0.77 | 0.69 decades |

The true root gap is the better predictor, as [the gap past 2175](../the-gap-past-2175-2026-10/README.md) found (a
decade per about 250), and the standalone gap carries the shortfall's scatter (r² 0.59 against 0.77).  Both are fits to
the chain's searches, which had the seed and the ceiling, not to standalone searches.

### 2. The standalone searches

43 of the 64 finished within 1,800 s, all with the chain's optimum as their best game; 21 stopped at the limit.  By
standalone gap (the finished games' seconds):

| standalone gap | games | finished | median of the finished | longest |
|---|---|---|---|---|
| under 700 | 7 | 7 | 13.9 s | 36.9 s |
| 700-999 | 14 | 14 | 46.3 s | 287.6 s |
| 1,000-1,299 | 20 | 16 | 215.8 s | 1,770.9 s |
| 1,300-1,599 | 11 | 5 | 1,033.4 s | 1,319.6 s |
| 1,600 and over | 12 | 1 | 796.7 s | 796.7 s |

Every game under 1,000 finished, the slowest in 288 s (game 2478).  The nine games of a standalone gap 1,100 to 1,300
took from 1.5 minutes to 30 or did not finish: 3129 (gap 1,117) 89 s, 2560 (1,283) 118 s, 2555 (1,176) 448 s, 2265
(1,277) 493 s, 3221 (1,279) 648 s, 3156 (1,230) 1,450 s and 2392 (1,259) 1,771 s; 2535 (1,161) and 3171 (1,257) did
not finish.  The gap ranks the games but leaves more than a decade of scatter: 2565 (gap 1,383) took 53 s and 2299
(1,626) 797 s, while 2528 (1,054) took 1,036 s and 2530 and 2531 (1,083 each) did not finish.  The 21 that stopped have
standalone gaps from 1,083 to 2,546 (game 3404).  After 30 minutes the best game of 8 of them was the chain's optimum
already; the other 13 were below it by 4 (game 2345) to 966 (game 3404).  Of the 16 games the chain settled for nothing
(12 by the prime formula, 4 at their ceiling), 11 finished alone, in 7.5 s (game 2203) to 1,320 s (game 3209), and 5
did not.

### 3. The map, 3441-8191

No game's bounds met (0 of 4,751).  The smallest standalone gap is 1,196 (games 4176 to 4179), then 1,211 (4698 and
4700); 77 games are under 1,300, 234 under 1,500, 934 under 2,000 and 1,640 under 3,000.  By band of 500 games (the
gaps, and the median CPU seconds of a game's bounds):

| n | games | gap p10 | median | p90 | cpu s |
|---|---|---|---|---|---|
| 3441-3940 | 500 | 1,436 | 1,781 | 2,193 | 7.0 |
| 3941-4440 | 500 | 1,413 | 1,837 | 2,784 | 11.8 |
| 4441-4940 | 500 | 1,482 | 2,105 | 2,447 | 16.7 |
| 4941-5440 | 500 | 2,719 | 3,199 | 4,538 | 22.4 |
| 5441-5940 | 500 | 3,800 | 4,589 | 5,426 | 47.5 |
| 5941-6440 | 500 | 4,007 | 5,667 | 6,951 | 54.1 |
| 6441-6940 | 500 | 5,504 | 6,493 | 7,428 | 66.5 |
| 6941-7440 | 500 | 4,553 | 5,374 | 6,425 | 66.7 |
| 7441-7940 | 500 | 5,835 | 6,804 | 7,922 | 82.4 |
| 7941-8191 | 251 | 7,215 | 8,305 | 9,310 | 103.2 |

The median standalone gap rises from about 1,800 at 3441-4440 to 2,100 at 4441-4940, 3,200 at 4941-5440 and 4,600 to
8,300 beyond.  A game's bounds cost a median 7 s of one thread at 3441-3940, rising to 103 s at 7941-8191, almost all of
it the start incumbent's local search (the setup takes 0.1 to 0.2 s, the bound none); the longest game took 239 s.  The
sweep's plan estimated the setup of a game at 5.1 s at 3441, 22.6 at 5000, 90.7 at 7000 and 173.5 at 8191 by power laws
through four measured points (76 core-hours for the 4,751 games); the medians here are 7.0 (3441-3940), 22.4
(4941-5440), 66.7 (6941-7440) and 103.2 (7941-8191), 65.1 core-hours in all, so the extrapolation held to 5000 and ran
high beyond 6000.

### 4. The time to the best game, on the laptop

All 60 searches finished at the known optimum (the longest in 116 s, 452 s in all).  In 10 of them the heuristic's start
was already the optimum.  In the other 50 the optimum was found after a median 77% of the search's time (p10 41%, p90
96%).  So most of a search's time passes before it has the optimum, and stopping a search early is no cheap way to it;
stage 2 agrees (13 of its 21 unfinished searches still had a worse game than the chain's after 30 minutes).

## Decision

The standalone bounds prove nothing by themselves and rank the games with a wide scatter; they find no game past 3440
that is easy.

* The heuristic's shortfall has a median 227 and p90 782 over 2176-3440, and grows to a median 523 over 3176-3440.  The
  true root gap has a median 815, the standalone gap 1,062.
* The chain's nodes follow the true root gap with r = 0.88 (a decade per about 260 of gap, residual 0.5 decades) and the
  standalone gap with r = 0.77 (a decade per about 580, residual 0.7).
* Games under a standalone gap of 1,000 finished in under five minutes of one thread (21 of 21); those about 1,200 took
  1.5 to 30 minutes; of the 23 above 1,300 only 6 finished in 30 minutes.
* With today's heuristic no game past 3440 is within a few minutes of one-thread search: the map has 77 games under
  1,300 and none under 1,196, none is under 1,000, and a node costs more past 3440 ([the wall
  regime](../the-wall-regime-2026-10/README.md): +0.042 decades per 100 n).

A better lower end would help, within a limit: the true root gap, which a perfect start incumbent would leave, is a
median 1,177 over 3176-3440, and the search must still close it.  Measurements that would reopen the question: a
standalone search of any game past 3440; whether the shortfall keeps the 523 of 3176-3440 past it (the map has no
optima); searches of more than 30 minutes or on more than one thread; a different start incumbent (a longer local
search, or a second heuristic), for which the shortfall of 227 to 523 is the room.  Stage 2's 64 searches ran at once on
one machine and are a sample, not the 1,265 games.  The start incumbents seeded from a neighbor's, against the
standalone start, are in [the seeding entry](../seeding-the-start-from-a-neighbors-heuristic-game-2026-10/README.md).

## Files

The data of the run, of the laptop measurement beside it, and the scripts that make the entry's numbers.  `wall` is
`target/release/examples/wall` (`cargo build --release --features studies --example wall`) and `taxman`
`target/release/taxman`, run from `rust/`.  The module comment of `examples/wall.rs` ("landscape") defines every field
of a `cal.jsonl` and `map.jsonl.gz` line.  The files were written on the instance, synced to the project's bucket
(`landscape-20261017/out`) every five minutes and at the end, and copied from there; the run's stdout copies of the two
landscape files (`cal.out` and `map.out`, byte for byte the `.jsonl` files) are not kept.

| file | command | what it is |
|---|---|---|
| `userdata.sh` | the instance's user data, as it ran | installs the build (`solver/tools/cloud/setup.sh` at 6f57399), runs the three stages one after the other from a tmux session, syncs the output, and powers off; `shutdown -h +240` is its backstop |
| `log` | | the times (UTC) at which each stage began and ended: the run began 15:59:00, the calibration ended 15:59:56, the standalone searches 16:30:05, the map 17:32:14 |
| `cal.jsonl` | `wall landscape 2176-3440 --threads 64 --known results/strict.jsonl --out cal.jsonl` | the calibration, 56 s: a line for each of the 1,265 games, standalone: the bounds `upper` and `lower`, their `gap`, whether they `meet`, the times, and, from the committed chain, the game's optimum `opt` and with it `root_gap` and `shortfall` |
| `sample.txt` | | the 64 games of the standalone searches (2196 to 3440), followed by their count |
| `sa/N.jsonl`, `sa/N.out` | for the 64 games of `sample.txt`, all at once: `taxman solve N --threads 1 --time-limit 1800 --progress 0 --out sa/N.jsonl > sa/N.out 2>&1` | the standalone search of game N: the results file with the one line the search wrote (an unfinished search has `completed` false, `score` null and its best game's score in `best_so_far`) and what `solve` printed |
| `map.jsonl.gz` | `wall landscape 3441-8191 --threads 64 --out map.jsonl`, then gzipped | the map, 62 minutes: the same lines for the 4,751 games of 3441-8191, with `opt`, `opt_proof`, `root_gap` and `shortfall` null |
| `low.jsonl.gz` | `wall landscape 2-2175 --known results/strict.jsonl --out low.jsonl` on the laptop (2026-10-17, about two minutes, 497 CPU s), then gzipped; the command is that of `cal.jsonl` for the stretch below it (the run's own record was not kept) | the quick bounds of the 2,174 games of 2-2175 with their optima |
| `plot_gap.py`, `gap_landscape.svg` | `./plot_gap.py gap_landscape.svg` | the standalone gap of every game 2-8191 from `low.jsonl.gz`, `cal.jsonl` and `map.jsonl.gz`, dots of one game each and rolling medians of 101 games, and the quick upper bound less the proven optimum to 3440: an SVG with a linear y axis |
| `land.py` | | what the scripts share: the paths, and loading a file of lines (gzipped or not) |
| `cal.py` | `./cal.py` | the calibration's tables from `cal.jsonl` and `results/strict.jsonl`, and the fits of the chain's nodes (log10) to the two gaps |
| `map.py` | `./map.py` | the standalone searches against the chain's own search (`sa/`, `cal.jsonl`, `results/strict.jsonl`), and the map by bands of 500 games (`map.jsonl.gz`) |
| `ttb/` | `ttb/one.sh N` from `rust/` for each of the 60 games of `ttb/games.txt` (1301-2175), a few at a time under `nice -n 10` | the time to the best game: `N.txt` is the ladder line of game N (the search's time and nodes, the game found against the known optimum, the start incumbent, and when the final game was found); `ttb.py` (`./ttb.py`) gives the time at which the best game was found as a share of the search's time, for each game and in all |

## Commits

* 51da910 (2026-10-17): `wall landscape`, each game standalone.
* 6f57399 (2026-10-17): the merge of `wall landscape`, the build the run used.
