# The dear games: the problem or the chain's path (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-19.  Concluded 2026-10-19.

**Question.**  Are the chain's dear games dear because of the problem they pose, or because of the path the chain took
to opt(n) (the incumbent it started from, its threads)?  If the path, a better starting game would remove their cost,
and seeding the chain with the finder's game would be a big win instead of a small one.

**Answer.**  The problem, mostly.  In the chain, the residuals of the 11 dear games whose proof from opt(n) finished lie
1.81 decades above the 20 cheap games'.  Proved from opt(n) on one thread they still lie 1.70 above (0.94 of the 1.81),
so the chain's path is worth a factor of 1.3 on the dear games, and they stay 50 times dearer than the cheap ones.
Eight of the 20 dear games, the dearest, were not run, and a ninth did not finish.  The path does cost a great deal in
single games: a start 236 below opt(n) makes game 3020's proof 7.5 times as long, and the chain's search of 3020 was 8.9
times as long as the proof from opt(n).  Over the chain as a whole, starts below opt(n) cost about 15% of its search
time past 2175 (about 70 in n), in games of every residual: the 20 dear games took 4.9% of the search time of 2176-3312.
Seeding the chain is a modest item, and no way to the wall.

## Method

* A *dear game* is one of the 20 searched games of 2176-3312 whose tree in the chain is most above the cost model of
  [the gap past 2175](../the-gap-past-2175-2026-10/README.md), and a *cheap game* one of the 20 most below it.  The
  model is `log10 nodes = 2.668 + 0.04377 G / n^0.3` for G the game's root GF(2) gap, fitted to the 400 exhausted
  searches of 2176-3312 on the chain's nodes less 0.06 decades for the window and the threads, so that it speaks of one
  thread in order.  The *residual* of a game is log10 of its nodes over the model's.  The gaps of the two groups are
  comparable (a mean of 769 and of 807).
* A search's *start* is the incumbent it begins with, and is *below opt(n)* by d when its promotion sum is d less than
  opt(n)'s.  A *step* is what `wall nodes` counts; the chain's *nodes* are the same count.
* The four runs of a game, each on one thread:
  * **a**, the proof from opt(n): `wall nodes --mode opt`, the incumbent opt(n) with no set behind it, no seed and no
    ceiling;
  * **b**, the search from the chain's start in order: `--mode start`, which starts as the solver does (the greedy set,
    the seed of game n − 1 from the committed chain, the dive and the local search) and stops at the ceiling of game
    n − 1;
  * **c**, the search from the start of no seed: `--mode start --no-seed`;
  * **f**, a with the forcing window of the chain's build: `--mode opt --tactic force_window=80`.

The nodes in `results/strict.jsonl` are the EC2 chain's (64 threads, a window of 16 and then 4, the build of 55287ff).
With game 3020 beside them, whose chain search of 317,361 nodes against a proof of 35,857 steps made the question, that
is 41 games.

* a and b with the default tactics, the limit at 5 million steps, `wall nodes --known results/strict.jsonl`, built with
  `cargo build -j 2 --profile test --example wall --features studies` from c942cb4.  b is the same tree as a wherever
  the start is opt(n) (the 19 games where both ran have the same steps to the step), so b ran for the cheap games and
  the dear games whose start is below opt(n), and for the others it is a.
* f for 16 of them, because the chain ran the forcing window at 80 and the default has been `all` since 2026-10-16,
  which makes smaller trees.  `force_window=80` reproduces the chain's build: game 2170 takes 440,988 steps, the
  [prime-exchange entry](../prime-exchanges-in-the-search-2026-10/README.md)'s number.
* c for 3020, 3021 and 3024, and a sweep of 3020 with the incumbent d below opt(n) (`--delta -d`).
* The in-order start of the 164 searched games of 3313-3914 that took the most CPU time (92% of that stretch's time),
  `wall nodes --nodes 1`, which sets a game up and stops before its first step.
* About 100 minutes of the engine's time in 90 runs and a few minutes of set-ups on the laptop, one thread, `nice`, each
  run started while the 1-minute load was 8 or less.  Nothing ran on EC2.

Not done: eight of the 20 dear games, the dearest (2490, 2535, 2536, 2538, 2540, 2541, 2544 and 2550, which took 17 to
61 million nodes in the chain).  Game 2548 of the same wave took 13 minutes to reach the limit and the 11 proofs that
finished 6.5 minutes at the most, so eight more runs would most likely have told only that they reached it.

## Results

### 1. The games from each start

Steps of one thread; "residual of a" is a's against the same model.  `report.md` has every game; the 11 dear games whose
proof finished have chain / a of 1.6 to 8.4 and residuals of a of −0.01 to +0.72.  Examples, and the reference:

| n | gap | chain's residual | start below opt(n) | chain's nodes | b | a | chain / a | residual of a |
|---|---|---|---|---|---|---|---|---|
| 2254 | 713 | +0.85 | 8 | 4,530,650 | 1,664,430 | 1,406,613 | 3.2 | +0.40 |
| 2304 | 820 | +0.83 | 45 | 11,883,153 | 3,027,762 | 3,031,154 | 3.9 | +0.30 |
| 2324 | 815 | +0.85 | 0 | 11,662,760 | | 1,391,510 | 8.4 | −0.01 |
| 2910 | 663 | +0.81 | 154 | 1,558,045 | 747,809 | 381,606 | 4.1 | +0.26 |
| 3020 | 766 | −0.26 | 0 | 317,361 | | 35,857 | 8.9 | −1.14 |

Game 2548 (gap 859, residual +0.90, 16,126,191 nodes in the chain) did not finish in 5,000,191 steps from opt(n), so its
residual from opt(n) is +0.46 or more.  The 20 cheap games have chain residuals of −0.84 to −1.12 and residuals of a of
−0.85 to −1.48; 18 of them start at opt(n) (3016 starts 8 and 2205 starts 245 below).  A game's steps from a start that
is below opt(n) by a few points can be smaller than from opt(n): 3016 (8 below) has 0.77 of a's steps, and the sweep of
3020 below has 0.73 at 25.  The noise of one game is about a quarter.

### 2. A start at opt(n) does not bring the dear games back

Over the games whose proof from opt(n) finished (11 dear, 20 cheap), the mean residual of the dear games falls from
+0.85 in the chain to +0.48 from b and +0.41 from a, and the cheap games' from −0.96 to −1.27 and −1.29.  The dear
games' steps fall to 0.30 of the chain's from the chain's start and to 0.26 from opt(n), the cheap games' to 0.44 and
0.43.  Both groups fall, because the chain's build is the older one (section 3) and it ran 64 threads and a window; the
dear games' residual falls by 0.44 decades and the cheap games' by 0.33.  What belongs to the path of the dear games is
the difference, 0.11 decades.  The dear games' residual less the cheap games', as a share of the chain's 1.81: from b
1.75 (0.97), from a 1.70 (0.94).

Games close together pose nearly one problem, and a's steps show it: 2254, 2256 and 2260 take 1.41, 1.36 and 1.47
million steps from opt(n), at residuals of +0.40, +0.39 and +0.39.  One dear game does fall back: 2324 takes 1.39
million steps from opt(n), a residual of −0.01 against +0.85, 8.4 times fewer than the chain's 11.7 million (5.1 times
on the chain's own build, section 3), though its start in order is opt(n).  Something in the chain's path made that
factor, and not the start in order.

### 3. The chain's build

The chain ran with the forcing window at 80.  Its proofs from opt(n) with 80 (f) take the default's (a) steps times 1.47
at the 7 dear games (1.09 to 1.96), 1.39 at the 8 cheap games (0.94 to 1.82) and 1.25 at 3020; the chain's takes f's
steps times 2.20 (1.20 to 5.05), 1.51 (0.79 to 3.41) and 7.08.

The window of 80 makes the dear games' proofs 1.06 times as much as the cheap games' on these 15, so the build moves the
two groups together and the dear games' residual above the cheap games' from opt(n) is 1.72 decades, 0.95 of the
chain's.  On the chain's own build the cheap games' chain takes 1.5 times the steps of the proof from opt(n) on one
thread: the cost of 64 threads and of the window, with a start that is opt(n) in 18 of the 20 cheap games.  The dear
games' chain takes 1.46 times as many as the cheap games' (2.2 against 1.5), and 3020's 7.1.

### 4. What a start below opt(n) costs

Game 3020, the proof with the incumbent d below opt(n) and no set behind it, against the 35,857 steps of the proof from
opt(n) and the chain's 317,361:

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

The cost is flat to about 50 below, doubles at 150, climbs by 3.8 between 150 and 236 and by about 1.6 for each 100
after.  The search from the start of no seed (c) is the same tree as the sweep's 236 for 3020 and 3021 (the same
problem): 269,882 steps, 7.5 times a, against the chain's 317,361 at 3020 and 62,585 at 3021.  At 3024 c starts 220
below opt(n) and takes 164,515 steps, 5.5 times a.  So the chain's search of 3020 cost what a start about 236 below
opt(n) costs, while its search of 3021, the same problem a game later, took 1.7 times a.

The chain's starts in order seldom lie that far below: 29 of the 509 games of 2176-3312 start 150 or more below opt(n).
The 12 games run from both whose start is below opt(n) have b / a of 1.18, 1.22, 1.04 and 0.77 (8 below: 2254, 2256,
2275, 3016), 1.42 (42, 2268), 1.00 (45, 2304), 1.01 (47, 2928), 1.17 (58, 2260), 1.08 (60, 2925), 1.22 (80, 2625), 1.96
(154,
2910) and 3.44 (245, 2205).  Their geometric mean is 1.27, and the points at 154 and 245 follow the sweep (1.96 at 150)
      or
lie under it (3.4 against 7.5 at 236).

### 5. What a start at opt(n) is worth to the chain

The dear 20 are not where the chain's time is.  They took 67 of its 1,382 CPU hours over the 509 searched games of
2176-3312 (4.9%), the cheap 20 one hour.  Where the time is, by how far the start in order lies below opt(n) (the games
of 2176-3312 are the gap entry's 509, those of 3313-3914 the 164 that took the most CPU time):

| start below opt(n) | games 2176-3312 | share of their time | games 3313-3914 | share of their time |
|---|---|---|---|---|
| 0 | 372 | 0.49 | 107 | 0.57 |
| 1-49 | 59 | 0.13 | 18 | 0.14 |
| 50-99 | 35 | 0.13 | 16 | 0.08 |
| 100-149 | 14 | 0.04 | 4 | 0.09 |
| 150-249 | 17 | 0.12 | 12 | 0.08 |
| 250 or more | 12 | 0.09 | 7 | 0.04 |

A start at opt(n) from the beginning would take what these starts cost.  Valued by game 3020's sweep and by the 12 real
starts of section 4 (the multiple of the last point beyond it, none extrapolated):

| stretch | CPU hours | by 3020's sweep | by the real starts |
|---|---|---|---|
| 2176-3312 | 1,382 | 19.9% (275 hours) | 18.0% (249 hours) |
| 3313-3914, the 164 games | 4,412 | 13.9% (615 hours) | 13.4% (590 hours) |

That is about 15% of the 5,794 CPU hours (890 and 840).  The chain's own residuals say as much or more: its 137
exhausted searches of 2176-3312 whose start is below opt(n) have a mean residual of +0.18 against −0.09 for the 263
whose start is opt(n), a difference of 0.26 decades or a factor of 1.8 (the sd is 0.3 to 0.5 in each band), and these
games hold 51% of the search time.  That difference is an upper bound on the cost of the start, since the games whose
start is below opt(n) are not a random sample.  In n, 15% is about 70: a doubling of speed is worth about 310
([status](../../../docs/STATUS.md)), and 15% is 0.23 of a doubling.

## Decision

* **The dear games are the problem's.**  A start at opt(n) takes 6% of the residual that sets them apart, so no start
  brings them back to their neighbours.  They go with the rest of the cost, to the bound ([completions near the
  optimum](../completions-near-the-optimum-in-the-hard-subtrees-2026-10/README.md) finds the bound's error in their
  trees), not to the chain's path.
* **Seeding the chain with a better start is a modest item, not the big one.**  It is worth about 15% of the search time
  past 2175, about 70 in n.  The gain is spread over the games whose start in order is below opt(n): 27% of the games
  and 51% of the time in 2176-3312, 35% and 43% in 3313-3914.  The 48 whose start is 150 or more below hold 21% and 12%
  of the time of the two stretches, and most of the gain.  The finder's game after 5 CPU seconds of one thread is at
  opt(n) in 64% of the games, so seeding every game with it would reach most of that for about an hour of CPU in all.
  [The finders entry](../finders-for-good-games-that-are-not-sound-2026-10/README.md)'s 0.44 of the nodes is against the
  standalone start (no seed, no ceiling), which is c here and lies 220 to 240 below opt(n); the chain's start in order
  is already opt(n) in 73% of the games.
* The run to 5000 goes on as launched, and it already seeds from the bounds store's games where the store has one
  (`--use-prev`).  Seeding the rest joins the list for the next change to the chain's driver; nothing here moves the
  plan for the bound.

What this does not show, and what would reopen it:

* The eight dearest dear games were not run, and 2548, their wave's cheapest, did not finish in 5.0 million steps: the
  wave of 2490 and 2535-2550 is the part of the claim that rests on one game.  Its chain nodes (17 to 61 million) are
  far above those of the games run (at most 12 million).
* The start in order is the start without speculation.  The chain's own, with a window, takes the provisional game of
  n − 1 and may lie further below opt(n): 3020 shows it, and the committed lines do not record the start.  The gap entry
  puts the window at −0.04 ± 0.03 decades on average (a window of 1 or 4 against 16), which would add a few percent to
  the 15%, and the start of no seed (c) is its extreme.
* One thread is not 64.  At 2205 the chain took 32,540 nodes, fewer than b's 42,912 steps from a start 245 below
  opt(n): 64 threads may find opt(n) sooner, and then the 15% is an overestimate.
* The multiples come from one sweep (3020) and 12 real starts, 10 of them within 60 of opt(n).
* The games that hold the chain's time are not these.  The 47 searches of gap 1,100 or more hold 77% of the nodes ([the
  gap past 2175](../the-gap-past-2175-2026-10/README.md)); the extremes of the residual hold 5%.

## Files

All made by `dear.py` in this folder, from `solver/frame` built as above.

* `games.jsonl`: the 41 games: group, gap, the residual of the cost model (by the gap alone as well), the chain's nodes
  and seconds (those of `results/strict.jsonl` too), the start in order and how far it is below opt(n).  Made by
  `./dear.py select > games.jsonl`.
* `runs.jsonl`: one line per run: `run` (a, b, c, f or dK), the steps, whether it finished, the seconds, the start and
  opt(n).  Made by `./dear.py run a --limit 5000000`, then `run b`, `run f`, `run c` and
  `run d25 d50 d100 d150 d236 d300 d400` with the games named.
* `starts.jsonl`: the in-order start of the 164 searched games of 3313-3914 that took the most CPU time, with the
  chain's nodes and seconds.  Made by `./dear.py starts 100` after one run in order of n that was stopped.
* `report.md`: the tables of this entry, with the games' f and c and the residuals of the chain's searches by start.
  Made by `./dear.py report > report.md`.
* `dear.py`: the driver (`select`, `run`, `starts` and `report`); it reads [the gap entry's
  functions](../the-gap-past-2175-2026-10/gap.py) for the residuals.

The commands of the runs are `wall nodes --known ../../results/strict.jsonl --nodes 5000000 --mode opt N` (a), `--mode
start` (b), `--mode start --no-seed` (c), `--mode opt --tactic force_window=80` (f) and `--mode opt --delta -K` (dK),
from `solver/frame`, with the build's `target/debug/examples/wall`.

## Commits

* 3eb5c92: this entry, with its data and `dear.py`.
* c942cb4: the commit `wall` was built from.
* 55287ff: the build of the EC2 chain whose nodes the entry takes.
