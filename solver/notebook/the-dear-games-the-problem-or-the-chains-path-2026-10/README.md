# The dear games: the problem or the chain's path (2026-10)

Part of the [lab notebook](../README.md).

**Question.**  Are the chain's dear games dear because of the problem they pose, or because of the path the chain took
to opt(n) (the incumbent it started from, its threads)?  If the path, a better starting game would remove their
cost, and seeding the chain with the finder's game would be a big win instead of a small one.

**Answer.**  The problem, mostly.  In the chain, the residuals of the 11 dear games whose proof from opt(n) finished lie
1.81 decades above the 20 cheap games'.  Proved from opt(n) on one thread they still lie 1.70 above (0.94 of the 1.81),
so the chain's path is worth a factor of 1.3 on the dear games, and they stay 50 times dearer than the cheap ones.  Eight
of the 20 dear games, the dearest, were not run, and a ninth did not finish.  The path does cost a great deal in single
games: a start 236 below opt(n) makes game 3020's proof 7.5 times as long, and the chain's search of 3020 was 8.9 times
as long as the proof from opt(n).  Over the chain as a whole, starts below opt(n) cost about 15% of its search time past
2175 (about 70 in n), in games of every residual: the 20 dear games took 4.9% of the search time of 2176-3312.  Seeding
the chain is a modest item, and no way to the wall.

## Words

* A *dear game* is one of the 20 searched games of 2176-3312 whose tree in the chain is most above the cost model of
  [the gap past 2175](../the-gap-past-2175-2026-10/README.md), and a *cheap game* one of the 20 most below it.  The
  model is `log10 nodes = 2.668 + 0.04377 G / n^0.3` for G the game's root GF(2) gap, fitted to the 400 exhausted
  searches of 2176-3312 on the chain's nodes less 0.06 decades for the window and the threads, so that it speaks of one
  thread in order.  The *residual* of a game is log10 of its nodes over the model's.
* A *step* is what `wall nodes` counts; the chain's *nodes* are the same count.
* A search's *start* is the incumbent it begins with.  It is *below opt(n)* by d when its promotion sum is d less than
  opt(n)'s.
* The four runs of a game, each on one thread:
  * **a**, the proof from opt(n): `wall nodes --mode opt`, the incumbent opt(n) with no set behind it, no seed and no
    ceiling;
  * **b**, the search from the chain's start in order: `wall nodes --mode start`, which starts as the solver does (the
    greedy set, the seed of game n - 1 from the committed chain, the dive and the local search) and stops at the
    ceiling of game n - 1;
  * **c**, the search from the start of no seed: `--mode start --no-seed`;
  * **f**, a with the forcing window of the chain's build: `--mode opt --tactic force_window=80`.

## What was run

The selection is the residual of the model above: the 20 searched games of 2176-3312 with the largest and the 20 with the
smallest (their gaps are comparable: a mean of 769 and of 807, and the model already scales the gap).  Their nodes in
`results/strict.jsonl` are the EC2 chain's (64 threads, a window of 16 and then 4, the build of 55287ff), which the
gap entry's data have too.  With game 3020 beside them, whose chain search of 317,361 nodes against a proof of 35,857
steps made the question, that is 41 games.

* a and b with the default tactics, the limit at 5 million steps, `wall nodes --known results/strict.jsonl`, built with
  `cargo build -j 2 --profile test --example wall --features studies` from c942cb4.  b is the same tree as a wherever
  the start is opt(n): the 19 games where both ran have the same steps to the step.  So b ran for the cheap games and
  the dear games whose start is below opt(n), and for the others it is a (marked).
* f for 16 of them, because the chain ran the forcing window at 80 and the default has been `all` since 2026-10-16,
  which makes smaller trees.  `force_window=80` reproduces the chain's build: game 2170 takes 440,988 steps, the
  [prime-exchange entry](../prime-exchanges-in-the-search-2026-10/README.md)'s number.
* c for 3020, 3021 and 3024, and a sweep of 3020 with the incumbent d below opt(n) (`--delta -d`).
* The in-order start of the 164 searched games of 3313-3914 that took the most CPU time (92% of that stretch's time),
  `wall nodes --nodes 1`, which sets a game up and stops before its first step.
* About 100 minutes of the engine's time in 90 runs and a few minutes of set-ups on the laptop, one thread, `nice`,
  each run started while the 1-minute load was 8 or less.  Nothing ran on EC2.

Not done: eight of the 20 dear games, the dearest (2490, 2535, 2536, 2538, 2540, 2541, 2544 and 2550, which took 17 to 61
million nodes in the chain).  Game 2548 of the same wave took 13 minutes to reach the limit, the 11 proofs that finished
6.5 minutes at the most, and the budget of one to two hours of CPU did not cover eight more runs that would most likely
have told only that they reached it.

## Results

### 1. The games from each start

Steps of one thread.  The residual is the chain's; "residual of a" is a's against the same model.  A star marks a b that
was not run (the start is opt(n), so b is a), and a `≥` or `≤` a run that stopped at the limit of 5 million steps.

| group | n | gap | residual | start below opt(n) | chain's nodes | b | a | chain / a | residual of a |
|---|---|---|---|---|---|---|---|---|---|
| dear | 2254 | 713 | +0.85 | 8 | 4,530,650 | 1,664,430 | 1,406,613 | 3.2 | +0.40 |
| dear | 2256 | 713 | +0.87 | 8 | 4,719,991 | 1,657,974 | 1,357,631 | 3.5 | +0.39 |
| dear | 2260 | 721 | +0.83 | 58 | 4,629,567 | 1,710,473 | 1,465,815 | 3.2 | +0.39 |
| dear | 2268 | 636 | +0.98 | 42 | 2,803,384 | 1,576,986 | 1,109,315 | 2.5 | +0.64 |
| dear | 2275 | 623 | +0.84 | 8 | 1,784,750 | 422,395 | 405,944 | 4.4 | +0.26 |
| dear | 2304 | 820 | +0.83 | 45 | 11,883,153 | 3,027,762 | 3,031,154 | 3.9 | +0.30 |
| dear | 2324 | 815 | +0.85 | 0 | 11,662,760 | 1,391,510* | 1,391,510 | 8.4 | -0.01 |
| dear | 2490 | 887 | +1.18 | 75 | 41,671,272 | - | - | - | - |
| dear | 2535 | 924 | +1.18 | 237 | 56,846,868 | - | - | - | - |
| dear | 2536 | 924 | +1.20 | 0 | 60,515,533 | - | - | - | - |
| dear | 2538 | 852 | +1.23 | 0 | 32,282,030 | - | - | - | - |
| dear | 2540 | 852 | +0.96 | 0 | 17,382,831 | - | - | - | - |
| dear | 2541 | 859 | +1.08 | 161 | 24,150,342 | - | - | - | - |
| dear | 2544 | 859 | +0.93 | 0 | 17,256,437 | - | - | - | - |
| dear | 2548 | 859 | +0.90 | 0 | 16,126,191 | ≥5,000,191* | ≥5,000,191 | ≤3.2 | ≥+0.46 |
| dear | 2550 | 942 | +0.96 | 85 | 40,880,016 | - | - | - | - |
| dear | 2625 | 713 | +0.83 | 80 | 3,126,461 | 1,721,884 | 1,407,883 | 2.2 | +0.54 |
| dear | 2910 | 663 | +0.81 | 154 | 1,558,045 | 747,809 | 381,606 | 4.1 | +0.26 |
| dear | 2925 | 497 | +0.86 | 60 | 372,138 | 253,467 | 235,049 | 1.6 | +0.72 |
| dear | 2928 | 506 | +0.80 | 47 | 353,088 | 190,037 | 187,239 | 1.9 | +0.58 |
| cheap | 2205 | 604 | -0.84 | 245 | 32,540 | 42,912 | 12,477 | 2.6 | -1.20 |
| cheap | 2725 | 868 | -0.85 | 0 | 264,178 | 226,210 | 226,210 | 1.2 | -0.85 |
| cheap | 2739 | 767 | -0.86 | 0 | 98,943 | 20,985 | 20,985 | 4.7 | -1.47 |
| cheap | 2740 | 767 | -0.85 | 0 | 100,215 | 20,384 | 20,384 | 4.9 | -1.48 |
| cheap | 2745 | 767 | -0.97 | 0 | 75,808 | 23,907 | 23,907 | 3.2 | -1.41 |
| cheap | 2748 | 767 | -0.87 | 0 | 94,424 | 23,907 | 23,907 | 3.9 | -1.41 |
| cheap | 2760 | 867 | -0.93 | 0 | 209,670 | 60,169 | 60,169 | 3.5 | -1.41 |
| cheap | 2765 | 867 | -0.88 | 0 | 233,588 | 65,248 | 65,248 | 3.6 | -1.37 |
| cheap | 2772 | 875 | -1.01 | 0 | 184,331 | 66,307 | 66,307 | 2.8 | -1.40 |
| cheap | 2776 | 875 | -1.07 | 0 | 159,335 | 64,949 | 64,949 | 2.5 | -1.40 |
| cheap | 2780 | 875 | -1.08 | 0 | 157,022 | 58,571 | 58,571 | 2.7 | -1.45 |
| cheap | 2783 | 860 | -1.04 | 0 | 149,810 | 62,670 | 62,670 | 2.4 | -1.36 |
| cheap | 2795 | 862 | -1.01 | 0 | 161,546 | 77,364 | 77,364 | 2.1 | -1.27 |
| cheap | 2796 | 862 | -0.98 | 0 | 170,506 | 77,364 | 77,364 | 2.2 | -1.27 |
| cheap | 3003 | 769 | -0.98 | 0 | 61,880 | 29,997 | 29,997 | 2.1 | -1.24 |
| cheap | 3008 | 791 | -0.91 | 0 | 88,328 | 38,566 | 38,566 | 2.3 | -1.21 |
| cheap | 3009 | 791 | -0.99 | 0 | 74,537 | 38,465 | 38,465 | 1.9 | -1.21 |
| cheap | 3016 | 766 | -0.91 | 8 | 70,134 | 29,573 | 38,468 | 1.8 | -1.11 |
| cheap | 3021 | 766 | -0.96 | 0 | 62,585 | 35,857 | 35,857 | 1.7 | -1.14 |
| cheap | 3024 | 766 | -1.12 | 0 | 42,927 | 29,994 | 29,994 | 1.4 | -1.22 |
| reference | 3020 | 766 | -0.26 | 0 | 317,361 | 35,857 | 35,857 | 8.9 | -1.14 |

A game's steps from a start that is below opt(n) by a few points can be smaller than from opt(n): 3016 (8 below) has 0.77
of a's steps, and the sweep of 3020 below has 0.73 at 25.  The noise of one game is about a quarter.

### 2. A start at opt(n) does not bring the dear games back

Over the games whose proof from opt(n) finished (11 dear, 20 cheap):

| group | games | mean gap | mean residual: chain | b | a | geometric mean steps: chain | b | a | sum of steps: chain | b | a |
|---|---|---|---|---|---|---|---|---|---|---|---|
| dear | 11 | 675 | +0.85 | +0.48 | +0.41 | 2,642,882 | 982,296 | 828,183 | 47,423,987 | 14,364,727 | 12,379,759 |
| cheap | 20 | 807 | -0.96 | -1.27 | -1.29 | 107,647 | 45,165 | 43,022 | 2,492,307 | 1,093,399 | 1,071,859 |

The dear games' steps fall to 0.30 of the chain's from the chain's start and to 0.26 from opt(n), the cheap games' to 0.44
and 0.43.  Both groups fall, because the chain's build is the older one (section 3) and it ran 64 threads and a window;
the dear games' residual falls by 0.44 decades and the cheap games' by 0.33.  What belongs to the path of the dear games
is the difference, 0.11 decades:

| from | dear's residual less cheap's | share of the chain's |
|---|---|---|
| the chain | 1.81 | 1.00 |
| b | 1.75 | 0.97 |
| a | 1.70 | 0.94 |

Games close together pose nearly one problem, and a's steps show it: 2254, 2256 and 2260 take 1.41, 1.36 and 1.47
million steps from opt(n), at residuals of +0.40, +0.39 and +0.39.  Game 2548, the cheapest of the dearest wave in the
chain (16 million nodes), did not finish in 5.0 million steps from opt(n): its residual from opt(n) is +0.46 or more,
against +0.90 in the chain.

One dear game does fall back: 2324 takes 1.39 million steps from opt(n), a residual of -0.01 against +0.85, 8.4 times
fewer than the chain's 11.7 million (5.1 times on the chain's own build, section 3), though its start in order is
opt(n).  Something in the chain's path made that factor, and not the start in order.

### 3. The chain's build

The chain ran with the forcing window at 80.  Its proofs from opt(n) with 80 (f) against the default (a), and the chain
against f, on the 16 games with an f:

| group | games | f / a: geometric mean | lowest | highest | chain / f: geometric mean | lowest | highest |
|---|---|---|---|---|---|---|---|
| dear | 7 | 1.47 | 1.09 | 1.96 | 2.20 | 1.20 | 5.05 |
| cheap | 8 | 1.39 | 0.94 | 1.82 | 1.51 | 0.79 | 3.41 |
| reference (3020) | 1 | 1.25 | 1.25 | 1.25 | 7.08 | 7.08 | 7.08 |

The window of 80 makes the dear games' proofs 1.06 times as much as the cheap games' on these 15, so the build moves the
two groups together and the dear games' residual above the cheap games' from opt(n) is 1.72 decades, 0.95 of the chain's.
On the chain's own build the cheap games' chain takes 1.5 times the steps of the proof from opt(n) on one thread (0.8 to
3.4): the cost of 64 threads and of the window, with a start that is opt(n) in 18 of the 20 cheap games.  The dear games'
chain takes 2.2 times (1.2 to 5.1), 1.46 times the cheap games', and 3020's 7.1.

### 4. What a start below opt(n) costs

Game 3020, the proof with the incumbent d below opt(n) and no set behind it (the start of no seed is 236 below), against
the 35,857 steps of the proof from opt(n) and the chain's 317,361:

| d | steps | times the proof from opt(n) |
|---|---|---|
| 0 | 35,857 | 1.00 |
| 25 | 26,109 | 0.73 |
| 50 | 29,874 | 0.83 |
| 100 | 50,236 | 1.40 |
| 150 | 70,345 | 1.96 |
| 236 | 269,882 | 7.53 |
| 300 | 378,502 | 10.56 |
| 400 | 595,786 | 16.62 |

The cost is flat to about 50 below, doubles at 150, climbs by 3.8 between 150 and 236 and by about 1.6 for each 100 after.
The search from the start of no seed (c) is the same tree as the sweep's 236 for 3020 and 3021 (the same problem):

| n | chain's nodes | a | c | c's start below opt(n) | c / a |
|---|---|---|---|---|---|
| 3020 | 317,361 | 35,857 | 269,882 | 236 | 7.5 |
| 3021 | 62,585 | 35,857 | 269,882 | 236 | 7.5 |
| 3024 | 42,927 | 29,994 | 164,515 | 220 | 5.5 |

So the clue was right for 3020: the chain's search cost what a start about 236 below opt(n) costs, while its search of
3021, the same problem a game later, took 1.7 times a.  The chain's starts in order seldom lie that far below: 29 of the
509 games of 2176-3312 start 150 or more below opt(n).  The 12 games run from both whose start is below opt(n), b
against a:

| start below opt(n) | n | b / a |
|---|---|---|
| 8 | 2254 | 1.18 |
| 8 | 2256 | 1.22 |
| 8 | 2275 | 1.04 |
| 8 | 3016 | 0.77 |
| 42 | 2268 | 1.42 |
| 45 | 2304 | 1.00 |
| 47 | 2928 | 1.01 |
| 58 | 2260 | 1.17 |
| 60 | 2925 | 1.08 |
| 80 | 2625 | 1.22 |
| 154 | 2910 | 1.96 |
| 245 | 2205 | 3.44 |

Their geometric mean is 1.27, and the points at 154 and 245 follow the sweep (1.96 at 150) or lie under it (3.4 against
7.5 at 236).

### 5. What a start at opt(n) is worth to the chain

The dear 20 are not where the chain's time is.  They took 67 of its 1,382 CPU hours over the 509 searched games of 2176-3312
(4.9%), the cheap 20 one hour.  Where the time is, by how far the start in order lies below opt(n) (the games of 2176-3312
are the gap entry's 509, those of 3313-3914 the 164 that took the most CPU time):

| start below opt(n) | games 2176-3312 | share of their time | games 3313-3914 | share of their time |
|---|---|---|---|---|
| 0 | 372 | 0.49 | 107 | 0.57 |
| 1-49 | 59 | 0.13 | 18 | 0.14 |
| 50-99 | 35 | 0.13 | 16 | 0.08 |
| 100-149 | 14 | 0.04 | 4 | 0.09 |
| 150-249 | 17 | 0.12 | 12 | 0.08 |
| 250 or more | 12 | 0.09 | 7 | 0.04 |

A start at opt(n) from the beginning would take what these starts cost.  Valued by game 3020's sweep and by the 12 real
starts of section 4 (the multiple of the last point beyond it, none extrapolated), that is

| stretch | CPU hours | by 3020's sweep | by the real starts |
|---|---|---|---|
| 2176-3312 | 1,382 | 19.9% (275 hours) | 18.0% (249 hours) |
| 3313-3914, the 164 games | 4,412 | 13.9% (615 hours) | 13.4% (590 hours) |

about 15% of the 5,794 CPU hours (890 and 840).  The chain's own residuals say as much or more: its 137 exhausted searches
of 2176-3312 whose start is below opt(n) have a mean residual of +0.18 against -0.09 for the 263 whose start is opt(n), a
difference of 0.26 decades or a factor of 1.8 (the sd is 0.3 to 0.5 in each band), and these games hold 51% of the
search time.  That difference is an upper bound on the cost of the start: the games whose start is below opt(n) are not
a random sample.  In n, 15% is about 70: a doubling of speed is worth about 310 ([status](../../docs/STATUS.md)), and
15% is 0.23 of a doubling.

## Decision

* **The dear games are the problem's.**  A start at opt(n) takes 6% of the residual that sets them apart, so no start
  brings them back to their neighbours.  They go with the rest of the cost, to the bound ([completions near the
  optimum](../completions-near-the-optimum-in-the-hard-subtrees-2026-10/README.md) finds the bound's error in their
  trees), not to the chain's path.
* **Seeding the chain with a better start is a modest item, not the big one.**  It is worth about 15% of the search time
  past 2175, about 70 in n.  The gain is spread over the games whose start in order is below opt(n): 27% of the games
  and 51% of the time in 2176-3312, 35% and 43% in 3313-3914.  The 48 whose start is 150 or more below hold 21% and
  12% of the time of the two stretches, and most of the gain.  The finder's game after 5 CPU seconds of one thread is
  at opt(n) in 64% of the games, so seeding every game with it would reach most of that for about an hour of CPU in all.
  [The finders entry](../finders-for-good-games-that-are-not-sound-2026-10/README.md)'s 0.44 of the nodes is against
  the standalone start (no seed, no ceiling), which is c here and lies 220 to 240 below opt(n); the chain's start in
  order is already opt(n) in 73% of the games.
* The run to 5000 goes on as launched, and it already seeds from the bounds store's games where the store has one
  (`--use-prev`).  Seeding the rest joins the list for the next change to the chain's driver; nothing here moves the
  plan for the bound.

## What this does not show

* The eight dearest dear games were not run, and 2548, their wave's cheapest, did not finish in 5.0 million steps: the
  wave of 2490 and 2535-2550 is the part of the claim that rests on one game.  Its chain nodes (17 to 61 million) are
  far above those of the games run (at most 12 million).
* The start in order is the start without speculation.  The chain's own, with a window, takes the provisional game of
  n - 1 and may lie further below opt(n): 3020 shows it, and it cannot be told from here, since the committed lines do
  not record the start.  The gap entry puts the window at -0.04 ± 0.03 decades on average (a window of 1 or 4 against
  16), which would add a few percent to the 15%, and the start of no seed (c) is its extreme.
* One thread is not 64.  At 2205 the chain took 32,540 nodes, fewer than b's 42,912 steps from a start 245 below opt(n):
  64 threads may find opt(n) sooner, and then the 15% is an overestimate.
* The multiples come from one sweep (3020) and 12 real starts, 10 of them within 60 of opt(n).
* The games that hold the chain's time are not these.  The 47 searches of gap 1,100 or more hold 77% of the nodes
  ([the gap past 2175](../the-gap-past-2175-2026-10/README.md)); the extremes of the residual hold 5%.

## Files

All made by `dear.py` in this folder, from `solver/frame` built as above.

| file | command | what it is |
|---|---|---|
| `games.jsonl` | `./dear.py select > games.jsonl` | the 41 games: group, gap, the residual of the cost model (by the gap alone as well), the chain's nodes and seconds (those of `results/strict.jsonl` too), the start in order and how far it is below opt(n) |
| `runs.jsonl` | `./dear.py run a --limit 5000000`, then `run b`, `run f`, `run c` and `run d25 d50 d100 d150 d236 d300 d400` with the games named | one line per run: `run` (a, b, c, f or dK), the steps, whether it finished, the seconds, the start and opt(n); about 100 minutes of the engine on one thread, started while the 1-minute load was 8 or less |
| `starts.jsonl` | `./dear.py starts 100` (after one run in order of n that was stopped) | the in-order start of the 164 searched games of 3313-3914 that took the most CPU time, with the chain's nodes and seconds |
| `report.md` | `./dear.py report > report.md` | the tables of this entry, with the games' f and c and the residuals of the chain's searches by start |
| `dear.py` | `./dear.py` | the driver: `select`, `run`, `starts` and `report`; it reads [the gap entry's](../the-gap-past-2175-2026-10/gap.py) functions for the residuals |

The commands of the runs are `wall nodes --known ../../results/strict.jsonl --nodes 5000000 --mode opt N` (a), `--mode
start` (b), `--mode start --no-seed` (c), `--mode opt --tactic force_window=80` (f) and `--mode opt --delta -K` (dK),
from `solver/frame`, with the build's `target/debug/examples/wall`.  The chain's `results/strict.jsonl` is the file of
the commit of this entry.

Commits: the commit of this entry.
