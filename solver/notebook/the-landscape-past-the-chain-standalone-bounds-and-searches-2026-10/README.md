# The landscape past the chain: standalone bounds and searches (2026-10)

Part of the [lab notebook](../EXPERIMENTS.md).

**Question.**  Brian's, with holes allowed in the chain ([the sweep](the-sweep-a-local-test-2026-10.md)): can each game's
standalone bounds, taken with no game of n − 1 (no seed, no ceiling, no prime formula), tell cheaply which games past
3440 are easy, and can those be proven one by one?  The two ends are `upper`, `in_the_bag` plus the root's GF(2) bound,
which the search computes at its first node, and `lower`, the game the search starts from (the better of the greedy set
and the matching dive, improved by the local search); their difference is the standalone gap, and a game whose ends
meet is settled at its first node.  The true root gap is `upper` less the optimum and the heuristic's shortfall is the
optimum less `lower`, so the standalone gap is their sum, wherever the optimum is known.

**What was run.**  One `c8g.16xlarge` at spot, 2026-10-17 from 08:53 to 10:33 PDT, about $1.10 (it was approved up to
$2), the code of commit 6f57399, in three stages one after the other on the 64 threads (the times are the run's log):

1. **Calibration**, 56 s: `wall landscape 2176-3440 --threads 64 --known results/strict.jsonl`, the bounds of the
   1,265 games the committed chain has past 2175, each beside its known optimum.
2. **Standalone searches**, 30 minutes: 64 games of 2196-3440 (`sample.txt`; their standalone gaps run from 404 to
   2,546, median 1,100 against the 1,265 games' 1,062; the chain searched 48 of them, settled 12 with the prime
   formula and 4 at their ceiling, and none as a repeat), all at once, one thread each, `taxman solve N --threads 1
   --time-limit 1800` with nothing of n − 1 in the file.
3. **The map**, 62 minutes (65.1 core-hours): `wall landscape 3441-8191 --threads 64`, the bounds of all 4,751 games
   past the end of the chain, with no optimum to compare.

The data, the commands and the scripts (`cal.py`, `map.py`) are in [bench/landscape/](../../bench/landscape/README.md);
the numbers below are their output.  Beside them, on the laptop, a quick measurement of when a search finds its best
game (`bench/landscape/ttb/`, last section).

**1. The calibration, 2176-3440.**  No game's bounds met: 0 of the 1,265 (the run took 0.95 core-hours, a median 2.08 s
of CPU a game and 8.98 s at most).  The heuristic ends short of the optimum by a median 227, and by 0 in 161 of the
1,265 games; the true root gap is a median 815, and the standalone gap, their sum, 1,062:

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

so the shortfall is not a constant: it was 146 to 237 and is 523 over the last 265 games, where no standalone gap is
under 1,000 (221 of the first 500 and 314 of the second 500 were).  The CPU of a game's bounds has a median of 1.6 s,
2.3 s and 5.0 s in the three bands.

How well each gap predicts the cost, over the committed chain's 448 exhausted searches of 2176-3440 (log10 of the
nodes the chain's search took against the gap, least squares):

| gap | a decade of nodes per | r | residual sd |
|---|---|---|---|
| the true root gap | 259 | 0.88 | 0.51 decades |
| the standalone gap | 579 | 0.77 | 0.69 decades |

The true root gap is the better predictor, as [the gap past 2175](the-gap-past-2175-2026-10.md) found (a decade per
about 250), and the standalone gap carries the shortfall's scatter (r² 0.59 against 0.77).  Both are fits to the chain's
searches, which had the seed and the ceiling, not to standalone searches; stage 2 tests those.

**2. The standalone searches.**  43 of the 64 finished within 1,800 s, all with the chain's optimum as their best game;
21 stopped at the limit.  By standalone gap (the finished games' seconds):

| standalone gap | games | finished | median of the finished | longest |
|---|---|---|---|---|
| under 700 | 7 | 7 | 13.9 s | 36.9 s |
| 700-999 | 14 | 14 | 46.3 s | 287.6 s |
| 1,000-1,299 | 20 | 16 | 215.8 s | 1,770.9 s |
| 1,300-1,599 | 11 | 5 | 1,033.4 s | 1,319.6 s |
| 1,600 and over | 12 | 1 | 796.7 s | 796.7 s |

Every game under 1,000 finished, the slowest in 288 s (2478).  Games of a standalone gap about 1,200 (1,100 to 1,300:
nine games) took from 1.5 minutes to 30: 3129 (gap 1,117) 89 s, 2560 (1,283) 118 s, 2555 (1,176) 448 s, 2265 (1,277)
493 s, 3221 (1,279) 648 s, 3156 (1,230) 1,450 s and 2392 (1,259) 1,771 s, and two did not finish, 2535 (1,161) and 3171
(1,257).  The gap ranks the games but leaves more than a decade of scatter: 2565 (gap 1,383) took 53 s and 2299
(1,626) 797 s, while 2528 (1,054) took 1,036 s and 2530 and 2531 (1,083 each) did not finish.

The 21 stopped at the limit, by standalone gap (the game): 1,083 (2530), 1,083 (2531), 1,161 (2535), 1,257 (3171),
1,349 (3115), 1,403 (3169), 1,412 (3125), 1,430 (2345), 1,547 (2388), 1,583 (3112), 1,650 (3393), 1,772 (3384), 1,788
(3105), 1,864 (3318), 1,906 (3368), 1,939 (3331), 1,985 (3335), 1,996 (2289), 2,195 (2351), 2,244 (3440) and 2,546
(3404).  After 30 minutes the best game of 8 of them was the chain's optimum already (2530, 2531, 2388, 3171, 3125,
3112, 3368 and 3105); the other 13 were below it by 4 (2345) to 966 (3404).  Of the 16 games the chain settled for
nothing (12 by the prime formula, 4 at their ceiling), 11 finished alone, in 7.5 s (2203) to 1,320 s (3209), and 5 did
not (2531, 2351, 3331, 3169 and 3318).

**3. The map, 3441-8191.**  No game's bounds met (0 of 4,751).  The smallest standalone gap is 1,196 (4176, 4177, 4178
and 4179), then 1,211 (4698 and 4700); 77 games are under 1,300, 234 under 1,500, 934 under 2,000 and 1,640 under 3,000.
By band of 500 (the gaps, and the medians of the CPU seconds of a game's bounds with their parts):

| n | games | gap p10 | median | p90 | cpu s | setup s | start s | bound s |
|---|---|---|---|---|---|---|---|---|
| 3441-3940 | 500 | 1,436 | 1,781 | 2,193 | 7.0 | 0.1 | 6.9 | 0.0 |
| 3941-4440 | 500 | 1,413 | 1,837 | 2,784 | 11.8 | 0.1 | 11.7 | 0.0 |
| 4441-4940 | 500 | 1,482 | 2,105 | 2,447 | 16.7 | 0.1 | 16.6 | 0.0 |
| 4941-5440 | 500 | 2,719 | 3,199 | 4,538 | 22.4 | 0.1 | 22.3 | 0.0 |
| 5441-5940 | 500 | 3,800 | 4,589 | 5,426 | 47.5 | 0.1 | 47.3 | 0.0 |
| 5941-6440 | 500 | 4,007 | 5,667 | 6,951 | 54.1 | 0.1 | 53.9 | 0.0 |
| 6441-6940 | 500 | 5,504 | 6,493 | 7,428 | 66.5 | 0.2 | 66.3 | 0.0 |
| 6941-7440 | 500 | 4,553 | 5,374 | 6,425 | 66.7 | 0.2 | 66.5 | 0.0 |
| 7441-7940 | 500 | 5,835 | 6,804 | 7,922 | 82.4 | 0.2 | 82.1 | 0.0 |
| 7941-8191 | 251 | 7,215 | 8,305 | 9,310 | 103.2 | 0.2 | 102.9 | 0.0 |

The median standalone gap rises from about 1,800 at 3441-4440 to 2,100 at 4441-4940, 3,200 at 4941-5440 and 4,600 to
8,300 beyond.  A game's bounds cost a median 7 s of one thread at 3441-3940, rising to 103 s at 7941-8191, almost all
of it the start incumbent's local search; the longest game took 239 s.  The sweep's plan estimated the setup of a game
at 5.1 s at 3441, 22.6 at 5000, 90.7 at 7000 and 173.5 at 8191 by power laws through four measured points (76
core-hours for the 4,751 games); the medians here are 7.0 (3441-3940), 22.4 (4941-5440), 66.7 (6941-7440) and 103.2
(7941-8191), 65.1 core-hours in all, so the extrapolation held to 5000 and ran high beyond 6000.

**4. The time to the best game, on the laptop** (`bench/landscape/ttb/`).  Standalone one-thread searches of 60 games
of 1301-2175 with the non-release build of the ladder (`cargo build --profile test --example ladder`), no seed, no
ceiling, 300 s at most; the ladder prints when the final game was found.  All 60 finished (the longest in 116 s,
452 s in all) at the known optimum.  In 10 of them the heuristic's start was already the optimum.  In the other 50 the
optimum was found after a median 77% of the search's time (p10 41%, p90 96%).  So most of a search's time passes before
it has the optimum, and stopping a search early is no cheap way to it; stage 2 agrees (13 of its 21 unfinished
searches still had a worse game than the chain's after 30 minutes).

**Findings.**

* The answer to Brian's question is no, with today's heuristic: the standalone bounds find no game past 3440 that is
  easy.  None meets, the smallest gap is 1,196, and games of a gap that size took 1.5 to 30 minutes of one thread on
  2176-3440.
* On 2176-3440 no game's bounds met, and on 3441-8191 none do: the standalone bounds prove nothing by themselves.
  They rank the games, with a wide scatter.
* The heuristic's shortfall (optimum less `lower`) has a median 227, p90 782, and is 0 in 161 of 1,265 games; it grew to
  a median 523 over 3176-3440.  The true root gap has a median 815, the standalone gap 1,062.
* The chain's nodes follow the true root gap with r = 0.88 (a decade per about 260 of gap, residual 0.5 decades) and
  the standalone gap with r = 0.77 (a decade per about 580, residual 0.7).
* The standalone searches: games under a standalone gap of 1,000 finished in under five minutes of one thread (21 of 21);
  those about 1,200 took 1.5 to 30 minutes; of the 23 above 1,300 only 6 finished in 30 minutes.
* The map's smallest standalone gap is 1,196, at 4176-4179, and the medians rise from about 1,800 at 3441-4440 to 2,100
  at 4441-4940, 3,200 at 4941-5440 and 4,600 to 8,300 beyond; none is under 1,000, where every sampled game finished in
  five minutes or less.  So with today's heuristic no game past 3440 is within a few minutes of one-thread search:
  games of standalone gap about 1,200 took 1.5 to 30 minutes on 2176-3440, the map has 77 games under 1,300 and none
  under 1,196, and a node costs more past 3440 ([the wall regime](the-wall-regime-2026-10.md): +0.042 decades per
  100 n).
* What would help is a better lower end, and it has a limit: the true root gap, which a perfect start incumbent would
  leave, is a median 1,177 over 3176-3440 and the search must still close it.

**Not measured.**  A standalone search of any game past 3440; whether the shortfall keeps the 523 of 3176-3440 past it
(the map has no optima); stage 2's 64 searches ran at once on one machine and are a sample, not the 1,265 games;
searches of more than 30 minutes or on more than one thread; a different start incumbent (a longer local search, or a
second heuristic): the shortfall of 227 to 523 is the room it has.

**Commits.**  6f57399, the run's build (the merge of `wall landscape`, 51da910); this entry adds the data and the
scripts in `bench/landscape/`.
