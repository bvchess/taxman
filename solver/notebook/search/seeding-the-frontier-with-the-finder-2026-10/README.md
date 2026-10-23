# Seeding the frontier with the finder (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-21.  Concluded 2026-10-21.

**Question.**  How much of the search time of the run that added 3915..4547 would seeding every game with the finder's
game have saved?  **Answer.**  At most 5.0% of the stretch's 29.7 billion nodes, against 8.0% if every game started at
opt(n) (valued by game 3020's sweep of what a start below opt(n) costs; 4.5% and 7.8% by the multiples of the real
starts).  102 of the 151 searched games start at opt(n) already, the finder lifts 18 of the other 49 to opt(n), and
the games it leaves far below hold the rest of the 8.0%: game 4192 alone holds 2.1 points of it.  The 5.0% is an upper
bound and probably a high one.  The 18 games that start 100 or more below opt(n) have trees as large as a model of the
games that start at opt(n) predicts (a mean residual of −0.03 decades), where the sweep predicts +0.81.

## Method

* **The games.**  The 207 lines of `results/strict.jsonl` in 3915..4547 with `method` "search".  56 reached the
  ceiling before their first node and have no nodes to save.  Game 3933 reached it after 3,093 nodes (its search took
  5,785 s, so its nodes undercount its cost); it starts at opt(n).  The other 150 exhausted their trees.  The 151 with
  nodes hold 29,748,394,824 nodes, counted on 64 threads of the cloud by the build ca357bf.
* **The start in order.**  `wall nodes --nodes 1 --known results/strict.jsonl N`, as
  [the dear games](../the-dear-games-the-problem-or-the-chains-path-2026-10/README.md#method) took it: the game set up as
  the chain sets it up (the greedy set, the seed of game n − 1 from the committed chain, the dive and the local
  search), stopped before its first step.  Its promotion sum is the `start`, and **d_start** is opt(n) less it.  The
  opt of every line agrees with the chain's.
* **The finder's game.**  `ledger run N --dir DIR --cpu-hours 0.0001 --workers 1 --lns-seconds 5 --climb off
  --floor-step 0`, a ledger of the game alone.  Its first job, the map, sets the game up standalone (no game of n − 1),
  finds the start incumbent and runs the finder from it for 5 s of one thread: large neighborhood search, the code of
  `examples/common/lns.rs`, its game replayed by the referee.  The map spends the budget, so no search follows.  The
  map's event gives the finder's start (`lns_from`) and its game (`lns_found`).  **d_finder** is opt(n) less the
  better of the start in order and the finder's game, which is what a seeded search would begin with.  A game that
  starts at opt(n) has nothing for the finder to add, and it was not run on the 102 of them.  The ledger's runs had
  the whole of a core (CPU over wall seconds), so the 5 wall seconds are 5 CPU seconds.
* **The estimate**, as [section
  5](../the-dear-games-the-problem-or-the-chains-path-2026-10/README.md#5-what-a-start-at-optn-is-worth-to-the-chain)
  of the dear games took it.  A game's tree from a start d below opt(n) is its tree from opt(n) times a multiple m(d),
  so seeding a game whose start is d_start with a game d_new below opt(n) takes its nodes to nodes × m(d_new) /
  m(d_start).  The finder's seeding has d_new = d_finder, and a perfect start d_new = 0.  The multiples are two curves,
  interpolated in log and held at the last one beyond it: game 3020's sweep (×1.0 to 50 below, ×1.4 at 100, ×2.0 at
  150, ×7.5 at 236, ×10.6 at 300 and ×16.6 at 400), and the real starts of that entry (×1.08 at 24, ×1.15 at 66, ×1.96
  at 154 and ×3.44 at 245).  `seeding.py` takes both from the dear games' `dear.py`.
* **The laptop.**  `wall` and `ledger` built from f496332 with `CARGO_BUILD_JOBS=2 cargo build --profile test
  --features studies --example wall --example ledger` (4 min 22 s), two processes at a time under `nice -n 10`, each
  started while the 1-minute load was 8 or less.  The starts took 816 CPU s (5.4 s a game), the finders 933 CPU s (19 s
  a game: 5 of them the finder's), a probe 360 CPU s.  Nothing ran on EC2.

## Results

### 1. Where the starts lie

102 of the 151 games start at opt(n), with 62.5% of the nodes.  The other 49 hold 37.5%.  By how far the start in order
lies below opt(n), with what each seeding saves as a share of the stretch's nodes (game 3020's sweep):

| start below opt(n) | games | share of the nodes | finder lifts to opt(n) | perfect start saves | finder's game saves |
|---|---|---|---|---|---|
| 0 | 102 | 62.5% | | | |
| 1-49 | 20 | 16.0% | 3 | 0.0% | 0.0% |
| 50-99 | 11 | 13.9% | 7 | 1.9% | 1.6% |
| 100-149 | 3 | 1.1% | 2 | 0.4% | 0.1% |
| 150-249 | 5 | 3.2% | 2 | 2.6% | 0.5% |
| 250-399 | 7 | 2.2% | 4 | 2.0% | 1.9% |
| 400 or more | 3 | 1.1% | 0 | 1.0% | 1.0% |
| all | 151 | 100% | 18 | 8.0% | 5.0% |

The sweep is flat to 50 below, so the 20 games in 1-49 save nothing in it (16.0% of the nodes).  Few games lie far
below: 15 start 150 or more below, with 6.5% of the nodes, against the 12% of the time that [the dear
games](../the-dear-games-the-problem-or-the-chains-path-2026-10/README.md#5-what-a-start-at-optn-is-worth-to-the-chain)
found at 150 or more in 3313-3914.

### 2. The finder's game

The finder's own start, the standalone start incumbent, lies 52 to 1,583 below opt(n) (median 397), and in 42 of the 49
games below the start in order: the seed from the game before is the better start.  After 5 s its game lies a median 21
below (mean 68, at most 278).  It is at opt(n) in 18 of the 49 games (37%, with 38% of their nodes), and beats the
start in order in 29.  Of the 18 games that start 100 or more below it lifts 8, and of the 15 that start 150 or more
below, 6.  Of the 20 games with the most nodes (57.2% of all), 7 start below opt(n), and it lifts 4 of them (4416,
4431, 3952 and 3950); of the 20 dearest among the 49, it lifts 7.  Ten of its 18 lifts are in the bands 1-49 and 50-99
(3 of the 20 games and 7 of the 11), where the sweep charges little.

The 3.0 points that the finder leaves of the perfect start's 8.0 lie in the games it leaves far below.  The ten games
where a perfect start would save most, in millions of nodes by the sweep:

| n | nodes | start below opt(n) | after the finder | perfect start saves | finder saves |
|---|---|---|---|---|---|
| 4192 | 740 | 228 | 228 | 629 | 0 |
| 4445 | 222 | 296 | 0 | 201 | 201 |
| 3915 | 226 | 260 | 0 | 199 | 199 |
| 4384 | 202 | 504 | 172 | 190 | 169 |
| 4431 | 771 | 85 | 0 | 162 | 162 |
| 3952 | 715 | 78 | 0 | 123 | 123 |
| 4389 | 246 | 137 | 137 | 109 | 0 |
| 3950 | 475 | 84 | 0 | 97 | 97 |
| 4180 | 88 | 932 | 148 | 82 | 78 |
| 4530 | 75 | 251 | 0 | 66 | 66 |

Game 4192 is 2.1 points of the perfect start's 8.0, and the finder's best game in 5 s is 278 below opt(n), worse than
its start of 228.

### 3. The estimate

| valued by | perfect start saves | finder's game saves | finder's share of the perfect | in millions of nodes |
|---|---|---|---|---|
| game 3020's sweep | 8.0% | 5.0% | 0.62 | 2,383 and 1,479 |
| the real starts | 7.8% | 4.5% | 0.57 | 2,325 and 1,333 |

Continuing the lines beyond their last points moves them by at most 0.7 points (the real starts: 8.2% and 5.2%).
Weighted by the cloud's CPU time instead of its nodes (4,127 CPU hours), the sweep gives 6.2% and 3.4%.  The finder
costs 247 s of its own on the 49 games, 0.26 CPU hours with the ledger's set-up, against 4,127 CPU hours of search: its
cost is nothing beside its gain, if the gain is anywhere near the estimate.

Earlier estimates: 18-20% of the search time of 2176-3312 and 13-14% of 3313-3914 for a perfect start.  The frontier's
8.0% is lower because fewer games lie far below.

### 4. What the chain's own trees say

The sweep is one game's, a tree of 36 thousand steps; the multiples of its real starts come from trees of up to a few
million steps.  The trees here have 3 million to 1.5 billion nodes (3933 aside).  The chain's own tree sizes check the
multiples.  A cost
model fitted to the 101 games that start at opt(n) (`log10 nodes = 6.019 + 0.01532 G / n^0.3` for G the root gap over
opt(n); residuals of sd 0.45, a weak model) gives each other game a residual, which should rise with the multiple:

| start below opt(n) | games | mean residual | standard error | the sweep's mean log10 multiple |
|---|---|---|---|---|
| 1-49 | 20 | +0.12 | 0.13 | 0.00 |
| 50-99 | 11 | +0.21 | 0.16 | +0.07 |
| 100-149 | 3 | −0.26 | 0.12 | +0.21 |
| 150-249 | 5 | +0.15 | 0.26 | +0.62 |
| 250-399 | 7 | −0.12 | 0.14 | +1.02 |
| 400 or more | 3 | +0.13 | 0.18 | +1.22 |

The 18 games that start 100 or more below opt(n) have a mean residual of −0.03 (standard error 0.10) where the sweep
expects +0.81.  Nothing in the chain's trees shows a cost of the start at the frontier beyond the 0.1 to 0.2 decades of
the games within 100 below opt(n), which the sweep puts at 0.0 to 0.1.  The multiples of one small game are not those
of a tree a thousand times larger, where the early work with a poor incumbent may be an extra cost that does not
grow with the tree.  If a start's cost were the extra steps game 3020 takes (35,857 steps from opt(n) times the
multiple less 1) and not a multiple of the game's own tree, a perfect start would save 0.02% of the nodes and the finder
0.01%.  So the estimate lies between 0.01% and 5.0%, and the chain's own trees favor the low end for the games far
below.  One game does look like a costly start: 4192, the one the finder leaves at 228 below, has a residual of +1.17.
The other four games in 150-249 have −0.31 to +0.09, and the ten that start 250 or more below −0.47 to +0.49 (mean
−0.04).

### 5. The finder from the seed of the game before (a probe)

The finder above starts from the standalone start incumbent, which is below the start in order in 42 of 49 games; a
finder that began from the start in order or the seed would reach further.  A probe through the ledger, with the game of
n − 1 given to it (`--proven`, one line of the chain), re-ran the 10 games the finder leaves 50 or more below opt(n).
The ledger runs its finder before a search from the best seed of the neighboring games, and by its rule only when
that seed differs from the one the finder last ran from (the first finder's game, when that is the best seed).  It ran
the finder in 2 of the 10: at 4224 the finder began 90 below opt(n) and reached it in 5 s, and at 4232 it began 94
below and stayed there.  In the other 8 it did not run it.  The probe settles nothing about those 8, and a finder run
from the start in order, which the ledger cannot be asked for, is not measured.

## Decision

* **Seeding every game with the finder's game is worth at most 5.0% of the frontier's nodes, and probably less.**  A
  perfect start is worth at most 8.0%.  The chain's own trees do not show the cost of a poor start that the estimate
  charges, so neither number is a reason to change the chain's driver.  It is cheap (5 s a game starting below opt(n)),
  and it stays on the list for the next change to the driver, as the dear games entry left it.
* **The finder is short of a perfect start where the start is far below:** it leaves 10 games 50 or more below opt(n),
  with 7.1% of the nodes.  A longer finder, or one begun from the start in order, is the way to reach them.

What this does not show, and what would reopen it:

* One thread is not 64.  On 64 threads the search may find opt(n) sooner from a poor start, which would lower the
  estimate further.
* The start in order is the start without speculation.  The chain's own start, with a window of games in flight, takes
  the provisional game of n − 1 and may lie further below, which would raise the estimate; the committed lines do not
  record it.
* The multiples come from one game and 12 real starts, none from a tree of the frontier's size.  A game of the
  frontier that starts far below, such as 4192 (740 million nodes, 228 below) or 3915 (226 million, 260 below), run
  from opt(n) and from its start on the cloud, would give the multiple that matters.  It costs money, which needs the
  approval of whoever owns the AWS account, with an estimate first.
* The finder began from the standalone start incumbent, not the start in order, and ran 5 s on a laptop, not on a cloud
  core.  The ledger's finder is the only one that was tried.
* The estimate rests on a few games: the 15 that start 150 or more below hold 5.6 of the 8.0 points, and 4192 and 4389
  alone 2.5.

## Files

All made by `seeding.py` in this folder, from `solver/frame` built as above.

* `games.jsonl`: the 207 searched games of 3915..4547 with the committed line's facts (`nodes`, `proof`, `search_s`,
  `threads`, `score`, `in_the_bag`, and `opt`, the promotion sum).  Made by `./seeding.py games > games.jsonl`.
* `starts.jsonl`: the start in order of the 151 games with nodes: `wall nodes --nodes 1`'s `start`, `opt`, `root_bound`
  and `gap_opt`, and the process's CPU seconds.  Made by `./seeding.py starts`.
* `finders.jsonl`: the ledger's runs on the 49 games that start below opt(n): the map's event (`lns_from`, `lns_found`,
  `lns_s`, scores that include `in_the_bag`) and the run's end, and the CPU seconds.  Made by `./seeding.py finders`.
* `seeded.jsonl`: the probe of section 5, the same with the game of n − 1 given; `searches` holds the search events
  that followed the map.  Made by `./seeding.py seeded`.
* `report.md`: every table of this entry, and the 151 games one by one.  Made by `./seeding.py report > report.md`.
* `seeding.py`: the driver (`games`, `starts`, `finders`, `seeded` and `report`); it reads the dear games' `dear.py`
  and `runs.jsonl`.  The runs resume where they stopped.

## Commits

* f496332: the commit `wall` and `ledger` were built from.
* ca357bf: the build of the cloud's run that made the nodes of 3915..4547.
