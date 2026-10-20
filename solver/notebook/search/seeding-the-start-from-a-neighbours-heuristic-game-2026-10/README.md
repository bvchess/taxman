# Seeding the start from a neighbour's heuristic game (2026-10)

Part of the [lab notebook](../../README.md).

**Question.**  Brian's: for games no chain has proven, how much better a lower bound do we get if each game's start
incumbent is seeded from a neighbour's start incumbent (a heuristic game, never proven), the way a chain seeds from
the proven game of n − 1?  [The landscape past the chain](../the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10/README.md)
took every game standalone, with no seed: the heuristic fell short of the optimum by a median 227 on 2176-3440.
Seeded from the proven game of n − 1 (`wall start --known`) the start is exact in 88% of games, with a mean shortfall of
11: the floor for any seeding.  Where the two lie matters for the games past 3440, where no neighbour is proven.

**What was run.**  `wall chain RANGE [--down] [--restart K] [--known FILE] [--out FILE]`, new in commit a8ee892: it goes
through the range on one thread (descending with `--down`) and makes each game's start incumbent
(`search::start_incumbent`, the same call `landscape` makes for its `lower`) seeded with the promotions of the previous
game's start incumbent, standalone at the first game and at every K-th with `--restart K`; one line a game, with the
score, the seed's origin, the games since the last restart, the times and (with `--known`) the optimum and the
shortfall.  A test in the quick suite runs 900-925 against optimal.json: every score is at most the optimum, and
`--restart 1` equals `landscape`'s `lower` game for game.  Then, on the laptop on 2026-10-17, six one-thread processes
at once under `nice -n 10` with the test-profile build, over the 1,265 games of 2176-3440 against the committed chain:

* **A**: up, unbroken (a seed from n − 1 all the way from 2176); **A64** and **A256**: up, standalone at every 64th
  and every 256th game.
* **B**: down from 3440, unbroken (a seed from n + 1).
* **R1**: `--restart 1`, the standalone start of every game, here on the laptop beside the others.  It equals the
  cloud calibration's `lower` in all 1,265 games.
* **C**: the existing `wall start --known results/strict.jsonl`, seeded from the proven game of n − 1 (its local
  search, with pairs; the matching dive is not in it).
* **max(A,B)**: for each game the better of A's and B's, which costs two threads.

The data, commands and the analysis script (`seeding.py`, whose output is `seeding.txt`) are in
[solver/notebook/search/seeding-the-start-from-a-neighbours-heuristic-game-2026-10/](README.md); the numbers below are its output.

**1. The shortfall (the optimum less the start's score), 2176-3440.**

| | median | mean | p90 | exact | better than R1 | worse than R1 | start's time, median |
|---|---|---|---|---|---|---|---|
| R1, standalone | 227 | 329.5 | 782 | 12.7% | | | 2.35 s |
| A, up | 150 | 198.7 | 457 | 14.1% | 50.9% | 20.5% | 1.02 s |
| A64, restart every 64 | 227 | 281.6 | 613 | 12.0% | 34.3% | 28.5% | 1.03 s |
| A256, restart every 256 | 157 | 206.9 | 510 | 14.1% | 47.3% | 22.0% | 1.02 s |
| B, down | 158 | 204.3 | 448 | 12.3% | 49.1% | 25.5% | 1.00 s |
| max(A,B) | 111 | 146.6 | 367 | 16.8% | 60.7% | 8.9% | two threads |
| C, the proven game of n − 1 | 0 | 12.2 | 19 | 87.6% | 85.8% | 1.1% | (not the same work) |

A neighbour's heuristic game closes about 40% of the standalone start's mean shortfall (330 to 199 up, 204 down) and
the better of the two directions 55% (147); the proven game closes 96% (12).  The heuristic neighbour leaves a mean
shortfall 12 to 17 times the proven one's, and is exact in 14% of games, not 88%.  The seed leaves some games worse than
standalone: 20.5% of A's, 28.5% of A64's, 22.0% of A256's, 25.5% of B's and 8.9% of the better of two.

By band of 250 n, the median and the mean of the shortfall (the last band is 15 games; medians rounded):

| n | R1 | A | A64 | A256 | B | max(A,B) | C |
|---|---|---|---|---|---|---|---|
| 2176-2425 | 398 / 549 | 166 / 213 | 285 / 325 | 166 / 213 | 264 / 287 | 139 / 173 | 0 / 28 |
| 2426-2675 | 155 / 217 | 80 / 110 | 166 / 238 | 80 / 134 | 149 / 152 | 75 / 80 | 0 / 8.5 |
| 2676-2925 | 72 / 103 | 67 / 90 | 67 / 90 | 67 / 90 | 74 / 85 | 67 / 72 | 0 / 4.4 |
| 2926-3175 | 196 / 210 | 85 / 127 | 221 / 225 | 85 / 127 | 146 / 165 | 80 / 119 | 0 / 8.0 |
| 3176-3425 | 523 / 553 | 429 / 427 | 506 / 523 | 447 / 444 | 343 / 302 | 212 / 262 | 0 / 11.6 |
| 3426-3440 | 757 / 595 | 692 / 632 | 373 / 392 | 692 / 632 | 813 / 711 | 692 / 605 | 0 / 19 |

and the share of games where the start is exact:

| n | R1 | A | A64 | A256 | B | max(A,B) | C |
|---|---|---|---|---|---|---|---|
| 2176-2425 | 4.0% | 1.2% | 0.0% | 1.2% | 1.2% | 2.4% | 80.0% |
| 2426-2675 | 15.2% | 10.8% | 2.4% | 10.8% | 8.0% | 16.8% | 86.0% |
| 2676-2925 | 34.8% | 41.6% | 41.6% | 41.6% | 34.4% | 42.0% | 94.0% |
| 2926-3175 | 10.4% | 17.6% | 16.8% | 17.6% | 18.8% | 24.0% | 92.0% |
| 3176-3425 | 0% | 0% | 0% | 0% | 0% | 0% | 86.0% |
| 3426-3440 | 0% | 0% | 0% | 0% | 0% | 0% | 86.7% |

Where the standalone start is worst, the heuristic neighbour helps least: on 3176-3425 (the end of the chain, the
nearest to the games past it) no game of any unproven variant is exact, the median is 429 for A and 343 for B against
523 standalone, and only taking the better of the two gets to 212; the proven game gets all of them to a median 0.  B
is the better direction there (mean 302 against A's 427), and A the better one up to 3175 (except on 2676-2925, where
they are alike).

**2. Drift: the shortfall against the games since the last restart.**  The ratio is the variant's mean shortfall over
standalone's (R1) on the same games, which takes out the games' own difficulty; below 1 the seed helps:

| games since the last restart | A64 | A256 | A | B |
|---|---|---|---|---|
| 1-15 | 1.02 (300 games) | 0.99 (75) | 0.77 (15) | 1.24 (15) |
| 16-63 | 0.79 (945) | 1.11 (240) | 1.15 (48) | 0.42 (48) |
| 64-255 | | 0.54 (945) | 0.35 (192) | 0.58 (192) |
| 256-1023 | | | 0.62 (768) | 0.76 (768) |
| 1024 and more | | | 0.80 (241) | 0.53 (241) |

There is no drift in the sense of a seed that loses its worth with distance.  The first 15 games after a restart gain
nothing (the ratio is 1.02 for A64 and 0.99 for A256, the bins that are large; 30% of A64's games are better than
standalone and 21% worse); the gain then grows with the length of the chain, to 0.54 for A256, 0.58 for B and 0.35 for
A at 64-255 games, and holds or fluctuates out to 1,264 games (A 0.62 and 0.80, B 0.76 and 0.53; A's and B's bins are
each a stretch of n, so the later ones are not a trend).  A restart throws the gain away: on the 285 games within 15
after a restart of A64 at which A had gone on for 64 games or more, A's mean shortfall is 200 and A64's 370 (R1's 358);
on the 60 of A256, 294 against 357 (346).  So A (199) is only 4% ahead of A256 (207) over all 1,265 games, while A64
(282) keeps about a third of A's gain over R1.  What a long chain buys is the accumulation over the first 64 to 255
games, and no more beyond.

**3. Why only 40%: neighbours have nearly the same shortfall, seeded or not.**  Without any seed, R1's shortfall at n
equals the one at n − 1 in 78.5% of games (correlation 0.91).  Seeded, A's equals its seed game's in 81.2% (0.96),
A256's in 81.3% and B's in 80.5% (0.96), and it is lower in 8.9%, 9.0% and 9.5% (R1: 10.6%) and higher in 10.0%, 9.8%
and 10.0% (R1: 10.9%).  So the seeded start is no more likely to repair its neighbour's shortfall than a standalone
start is to differ from it, and the gain of A over R1 is not made step by step; it shows in the level over stretches of
n (the ratios above).  An inference from these two facts, not a test: a heuristic game of the neighbour is a start no
better than what the game's own heuristic finds, plus what the chain has accumulated.  The proven game of n − 1 is an
optimal game, and its start is exact in 88%.

**4. The time.**  The start incumbent takes 0.41 of the standalone start's time in A, A256 and B and 0.42 in A64 (the
sums over the 1,265 games: R1 3,807 s, A 1,552 s, A64 1,609 s, A256 1,561 s, B 1,564 s), a median 1.0 s against 2.35 s;
the ratio is 0.34 to 0.61 in every band of A and B (the median ms of R1, A, B: 1,672, 658, 748 at 2176-2425; 5,700,
1,933, 2,004 at 3176-3425).  Probably the seed gives the local search a better set to start from, so that it makes
fewer passes (not measured separately).  The better of two directions costs 0.82 of one standalone start.  R1's median
is 2.35 s on the laptop (six processes at once on ten CPUs), the cloud calibration's 2.06 s on 64 busy threads; C's
0.99 s is the seed's survivors and the local search only (no matching graph or dive), and not comparable.

**Findings.**

* A start seeded from a neighbour's heuristic game closes about 40% of the standalone start's mean shortfall (330 to 199
  going up, 204 going down), 55% with the better of the two directions (147; median 111, p90 367, exact in 16.8%), at
  0.41 of the standalone start's time per direction.  The proven game of n − 1 closes 96% (mean 12, exact in 88%).
* No unproven variant gets a game of 3176-3440 exact, the end of the chain being the nearest to the games past it: the
  better of two directions has a median 212 there against standalone's 523.
* The seed's worth does not decay with the chain's length; it starts to show after about 15 games, is largest after
  64-255, and restarting throws it away (A64 keeps a third of A's gain).  A chain of 256 games between restarts is as
  good as an unbroken one (mean 207 against 199).
* Neighbours have nearly the same shortfall seeded or not (equal in 81% of steps seeded, 78.5% standalone), which is
  why a heuristic neighbour helps by 40%; a proven neighbour is an optimal game and helps by 96%.

**Not measured.**  What a better start does to a search: its tree, its time or whether it finishes (the fit of the
chain's nodes to the standalone gap, a decade per 579, would put an average 183 less of gap at about a factor of two in
nodes, but a fit across games is not a measurement of one game's search); games past 3440, where no optimum is known to
compare with, and whose shortfall stood at a median 523 on 3176-3440 and may rise; a second heuristic, or a longer local
search, for the neighbour's games; the proven game of n + 1 as a seed going down; the times on the cloud's CPUs.

**Commits.**  a8ee892 (the subcommand `wall chain`, with its test, and `set_up` and `start_of` that `landscape` shares
with it); this entry adds the data and the script in `solver/notebook/search/seeding-the-start-from-a-neighbours-heuristic-game-2026-10/`.

## Later (2026-10): the command removed from master

The command `wall chain` was removed from the tree on 2026-10-18.  It builds at commit 9d9dae3 (`git show
9d9dae3:rust/examples/wall.rs`): `git checkout 9d9dae3`, then `cargo build --release --example wall` in `rust/`.

## Files

The
runs were made on the laptop on 2026-10-17, one thread each, six at once under `nice -n 10`, with the test-profile
build of `wall` (`cargo build --profile test --example wall` in `rust/`) at commit a8ee892, from `rust/`, over
2176-3440 against the committed chain (`results/strict.jsonl`).  The command `chain` was removed from master on
2026-10-18; it builds at commit 9d9dae3 (`git checkout 9d9dae3` first), where each `chain` line is defined in the module
comment of `examples/wall.rs`; `shortfall` is the optimum less the start's score.  (`wall start`, the command of `c`,
is still there: `cargo build --release --features studies --example wall`.)

| file | command | what it is |
|---|---|---|
| `a.jsonl` | `wall chain --known results/strict.jsonl --out a.jsonl 2176-3440` | A: up, seeded from the game before, unbroken |
| `a64.jsonl`, `a256.jsonl` | the same with `--restart 64`, `--restart 256` | A64 and A256: up, standalone at every 64th and 256th game |
| `b.jsonl` | the same with `--down` | B: down from 3440, seeded from the game after, unbroken |
| `r1.jsonl` | the same with `--restart 1` | the standalone start of every game, here on the laptop beside the others (equal, game for game, to `lower` in `../cal.jsonl`) |
| `c.jsonl` | `wall start --known results/strict.jsonl 2176-3440 > c.jsonl` | C: the start seeded from the proven game of n − 1 (`pairs` is its promotion sum; the matching dive is not part of it) |
| `seeding.py`, `seeding.txt` | `./seeding.py > seeding.txt` | the analysis (it reads `land.py` and `../cal.jsonl`) and its output, which the entry quotes |
