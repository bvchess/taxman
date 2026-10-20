# Finders for good games that are not sound (2026-10)

Part of the [lab notebook](../../README.md).

**Question.**  Brian's: the ledger keeps a lower bound L(n), a real game the referee replays, and an upper bound U(n),
for many boards.  A lower bound needs only a legal game, so any method may find it, sound or not; only an upper bound
has to be proven.  Can searches that are not sound (they may miss the optimum) find better games quickly, and does a
better starting game make the sound search much cheaper?  The start incumbent falls short of the optimum by a median 227
on 2176-3440 ([the landscape](../the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10/README.md)), a neighbour's
heuristic game closes 40% of that ([seeding](../seeding-the-start-from-a-neighbours-heuristic-game-2026-10/README.md)), and a
standalone search finds its optimum after a median 77% of its time, so stopping it early is no cheap way to a good game.

**What was run.**  On the laptop on 2026-10-17, with the test-profile build of `wall`, one process a game and four at
once under `nice -n 10`, in three stages (the data, commands and tables are in
[solver/notebook/search/finders-for-good-games-that-are-not-sound-2026-10/README.md](README.md); the commands `wall sound` and `wall finders`, new in commit fa3127a,
are defined in `examples/wall/finders.rs`).  Times are CPU seconds of the process.  The laptop searched at about 7
thousand steps a second a thread, as the cloud did in the landscape's searches (6.8 thousand).

* **Stage 1, the most a better start can save.**  The sound search of 40 games of 2176-3440, standalone (one thread, no
  ceiling, no seed from n − 1), from (i) today's start incumbent and (ii) the known optimum, given as the seed.  22
  games are of the landscape's standalone searches (5 to 300 s there, no primes or repeats), 18 others are from every
  band of 250 in n, picked by the nodes of the chain's search of them; 36 have a start short of the optimum and 4 (10%;
  13% of all the games) have none.
* **Stage 2, two finders and a control**, from the same start incumbent, each for 5 CPU s on one thread, on every 5th
  game of 2176-3440 (253 games; none has a start that meets the root bound): **LNS**, large neighbourhood search (keep a
  random 70-90% of the incumbent's promotions fixed, search the rest exactly as a unit of the engine, with the fixed
  promotions and every other candidate, for at most 1,024 steps, take any better game and repeat); **restricted**, the
  sound search on the candidates in the start incumbents of n − 1, n and n + 1 and the 32 best others (those in the
  heaviest set of the root's GF(2) bound first, then the larger numbers), which doubles the others and goes on from its
  best game when it finishes before the time is up; and the **control**, the sound search itself from the start for the
  same time.  Every game any of them returned was replayed by the referee at its score, and none was above the optimum
  (an error would have stopped the run).  The finders' settings were tuned lightly on 26 other games (n = 2178 + 50k) at
  5 s: LNS keeping 70-90% fixed (mean shortfall 15.5) beat 50-80% (18.3), 60-95% (17.2) and 80-95% (28.4), and 4,096
  steps a unit changed nothing (99.9% of the units finish within 1,024); restricted took the pool by the root bound's
  heaviest set (164) rather than by the larger numbers (255), and the number of others it begins with (8, 32 or 128)
  changed nothing because it doubles them.  Then the same at 20 s on every 25th game (51 games).
* **Stage 3**, as stage 1's ceiling was large (below) and a finder beat the control: the sound search of the 40 games of
  stage 1 seeded with the game LNS returns after 5 s, and after 1 s, counting the finder's time and the start's.

**1. The most a better start can save (stage 1).**  All 80 searches finished: (i) took 5,333 CPU s (5 to 531 s a
game), (ii) 2,135 s.  The nodes of (ii) over (i):

| games | median | mean | range | in total (nodes, CPU) |
|---|---|---|---|---|
| all 40 | 0.474 | 0.521 | 0.041-1.000 | 0.380, 0.400 |
| start short of the optimum (36) | 0.418 | 0.468 | 0.041-0.968 | 0.370, 0.393 |
| start is the optimum (4) | 1.000 | 1.000 | 1.000 | 1.000, 0.954 |
| (i) took 300 s or less (34) | 0.567 | 0.574 | 0.041-1.000 | 0.529, 0.561 |
| (i) took more than 300 s (6) | 0.192 | 0.222 | 0.061-0.513 | 0.177, 0.228 |

By the start's shortfall the median ratio is 0.68 (1-150, 7 games), 0.60 (151-300, 11), 0.33 (301-500, 11) and 0.25
(501 and over, 7).  The CPU follows the nodes (a median 0.479).  So the optimum as a seed halves the nodes in the median
game, and cuts the cost of the games that are dear to a fifth; this is the most any finder can buy (it takes an exact
finder to get it), and it is large: more where the start is worse and the search longer, nothing where the start is
already optimal.

**2. The finders against the control (stage 2).**  The shortfall (the optimum less the game's score) of the 253 games,
at 5 CPU s:

| | median | mean | p90 | exact | better than the control | worse | CPU s a game |
|---|---|---|---|---|---|---|---|
| start | 227 | 328.7 | 782 | 12.6% | | | 3.0 |
| LNS | 0 | 27.9 | 96 | 64.4% | 77.9% | 3.6% | 5.0 |
| restricted | 76 | 189.7 | 629 | 33.2% | 63.2% | 6.3% | 5.0 + 6.1 for the neighbours' starts |
| control | 195 | 297.4 | 752 | 20.6% | | | 5.0 |

By band of n, the mean shortfall and the share exact:

| n | start | LNS | restricted | control |
|---|---|---|---|---|
| 2176-2425 | 560 (4%) | 61 (50%) | 359 (22%) | 537 (6%) |
| 2426-2675 | 223 (14%) | 25 (58%) | 115 (48%) | 172 (24%) |
| 2676-2925 | 103 (36%) | 0.5 (92%) | 27 (72%) | 67 (56%) |
| 2926-3175 | 204 (10%) | 7.8 (80%) | 95 (26%) | 183 (18%) |
| 3176-3440 | 542 (0%) | 44 (43%) | 343 (0%) | 515 (0%) |

On the 94 games that need a search of their own (no primes, no repeats) the numbers are alike (LNS mean 20.7, exact
69%; control 311, 20%).  LNS gets the optimum in 64% of the games in 5 s where the control gets it in 21%, and where it
misses it falls short by less (a mean of 28 against 297); on 3176-3440, where no start is exact, its median is 8 and the
control's 504.  It makes about 2,560 units a game (2 ms each), 99.9% of them searched to the end, and finds 5.4
improvements; in stage 3 its last improvement came at a median 0.32 s, and within 1 s in 35 of 40 games.  Restricted is
better than the control (a mean of 190 against 297) but not close to LNS, and it costs more: the neighbours' start
incumbents cost 6.1 s a game besides its 5 (a sweep of a stretch has them anyway).  It ended with 46% of the candidates
after 3.2 rounds, and finished the whole sound search in 3 games where the control did in 8.

With 20 s a method on the 51 games, the mean shortfall of LNS goes from 29.0 (57% exact) at 5 s to 20.8 (65%),
restricted's from 170.8 (29%) to 121.5 (49%), and the control's from 325.7 (16%) to 287.1 (26%).  So LNS has most of
what it gets in the first seconds, and the games it misses it keeps missing: probably its neighbourhoods do not reach
the optimum of about a third of the games, where the others still gain with time.

**3. The sound search seeded with LNS's game (stage 3).**  The nodes of the search over (i)'s, and the CPU in all (the
start, the finder and the search over (i)'s start and search):

| LNS budget | seed's mean shortfall | nodes, median | nodes, in total | of (ii)'s | CPU in all, in total | games that cost less in all |
|---|---|---|---|---|---|---|
| 5 s | 14 (start 294) | 0.496 | 0.416 | 1.10 | 2,656 s of (i)'s 5,434 (0.49) | 32 of 40 |
| 1 s | 25 | 0.567 | 0.437 | 1.15 | 2,628 s of 5,434 (0.48) | 36 of 40 |

The seeded search is within 10 to 15% of what the exact seed gives (0.416 and 0.437 of (i)'s nodes against 0.380 for the
optimum), and the finder pays for itself in all but a few games: those whose search is shorter than the finder's time (a
game of 5 s search costs 11 s) or whose start is the optimum.  A 1 s finder is the better trade: the CPU in all is the
same as with 5 s but it costs more than (i) in 4 games, not 8, at most 1.16 times it (1.8 with 5 s).  Seeded searches
may also cost more nodes than (i) (at most 1.06 times in one game), since a search's tree is not monotone in its
incumbent.

**Findings.**

* The sound search from the optimum takes a median 0.47 of the nodes of the search from today's start (range 0.04 to
  1.0; 0.19 for the six games that take more than 300 s) and 0.38 in total: a better start makes the search much
  cheaper, most where the start is worst and the search longest.
* LNS, the search of a unit that fixes 70-90% of the incumbent's promotions, repeated, finds the optimum in 64% of the
  games of 2176-3440 in 5 CPU s of one thread (the control 21%), with a mean shortfall of 28 against the start's 329; on
  3176-3440 it has a median of 8 against the start's 510.  Restricted on the starts of the neighbours is better than the
  control and worse than LNS (mean 190).
* Seeded with LNS's game after 1 s, the sound search takes 0.44 of (i)'s nodes (0.38 from the optimum) and, with the
  start and the finder counted, 0.48 of its CPU.  Lower bounds can be found by a finder for a fraction of the time of
  the search they would shorten.
* LNS has most of its gain in 1-2 s and saturates: 20 s gives 65% exact against 57% on the same games.

**Not measured.**  Games past 3440, where no optimum is known to score the finders against (the finders' games stand as
lower bounds of the ledger, and the seeded search's incumbent says how far they are from the optimum there); LNS from
the proven game of n − 1 or from a neighbour's heuristic game instead of the standalone start; a finder budget that
scales with the game (the start incumbent alone costs 1 to 7 s); nogood tables carried across units, adaptive
neighbourhoods (the share fixed, or whole frames), perturbation, for the third of the games LNS does not solve; the 8 of
stage 3's 40 games that cost more CPU than (i) at 5 s, which a scaled budget would shrink; threads; the cloud's CPUs
(the nodes of the searches are as measured, the times about as in the landscape's cloud searches).  The CPU spent was
6.8 core-hours (stage 1 2.1, stage 2 1.7 and 1.0 at 20 s, stage 3 1.5, the tuning 0.5), 1.7 hours of four threads
against the 1 to 1.5 planned, from stage 2's 20 s and stage 3's second run.  The neighbours' starts of `restricted` in
the tuning runs are not in its CPU.

**Used in the ledger.**  LNS is the ledger's finder since 2026-10-17 (`--lns-seconds`, 1 s by default; its code moved to
`examples/common/lns.rs`, which `wall finders` shares): [the finder in the
ledger](../the-ledger-bounds-from-both-sides-2026-10/README.md#later-2026-10-17-the-finder-in-the-ledger).

**Commits.**  fa3127a (the commands `sound` and `finders`, their tests, and `examples/common/known.rs` and `rng.rs`,
which `tune` and `wall` share); this entry adds the data and the scripts in this entry's folder.

## Later (2026-10): the commands removed from master

The commands `wall sound` and `wall finders` were removed from the tree on 2026-10-18.  They build at commit 9d9dae3
(`git show 9d9dae3:rust/examples/wall/finders.rs`): `git checkout 9d9dae3`, then `cargo build --release --example wall`
in `rust/`.  The finder itself stays in `examples/common/lns.rs`, which the ledger uses.

## Files

The runs were
made on the laptop on 2026-10-17, one process a game and four at once under `nice -n 10`, with the test-profile build
of `wall` (`cargo build --profile test --example wall` in `rust/`), from `rust/`, against the committed chain
(`results/strict.jsonl`).  The commands `sound` and `finders` were removed from master on 2026-10-18; they build at
commit 9d9dae3 (`git checkout 9d9dae3` first), where they are defined in the module comment of
`examples/wall/finders.rs`, which defines every field of a line.  Every time is the CPU seconds of the process (the
laptop was shared with other work, so the wall time says little), and the searches are compared by their nodes as well.

`run.py` runs a command over a list of games, four processes at a time, and appends each game's line to the stage's
file as it ends; a run that is interrupted goes on where it was.  `tables.py` makes the tables of the entry from the
files and `../landscape/cal.jsonl`; its output is `tables.txt`.

### Files and the commands that made them

`GAMES1` is the 40 games of stage 1: 2180 2196 2200 2264 2312 2397 2416 2430 2440 2451 2478 2556 2560 2565 2592 2601
2632 2640 2691 2696 2700 2704 2712 2720 2799 2849 2891 2925 2964 2976 3000 3003 3010 3064 3128 3129 3213 3260 3267
3272.  The 22 of them in `../landscape/sa/` were picked by the time of their standalone search there (5 to 300 s, no
primes or repeats; four have a start that is already the optimum, as 13% of the games do); the 18 others, of every band
of 250 in n, by the nodes of the chain's search of them (100,000 to 1,200,000), with a start short of the optimum.

| file | command | what it is |
|---|---|---|
| `stage1.jsonl` | `./bench/finders/run.py sound stage1.jsonl --from opt --cap 600 --shuffle GAMES1`, then the same with `--from start --cap 900` | stage 1: the sound search of each of the 40 games, one thread, no seed from n − 1 and no ceiling, from the known optimum (`from` is `opt`: (ii)) and from today's start (`start`: (i)); 80 lines |
| `stage2.jsonl` | `./bench/finders/run.py finders stage2.jsonl --cpu 5 --shuffle 2176-3440/5` | stage 2: the three methods and the start, each of the 253 games 2176, 2181, ..., 3436, 5 CPU s a method (it predates the field `lns_found_ms`) |
| `stage2-20s.jsonl` | `./bench/finders/run.py finders stage2-20s.jsonl --cpu 20 --shuffle 2176-3440/25` | the same, 20 CPU s a method, on every 25th game (51) |
| `stage3-lns-5s.jsonl`, `stage3-lns-1s.jsonl` | `./bench/finders/run.py sound stage3-lns-5s.jsonl --from lns --cpu 5 --cap 900 --shuffle GAMES1`, and with `--cpu 1` | stage 3: the sound search of the 40 games seeded with the game the LNS finder returns after 5 and after 1 CPU s |
| `tuning/*.jsonl` | `./bench/finders/run.py sound tuning/NAME.jsonl --from lns --cap 0.01 --cpu 5 2178-3428/50 [--fix LO-HI] [--unit-nodes K]`, and `--from restricted` with `[--extra K] [--pool value|chosen]` (`value` was the default while the `value` files were made; `chosen` is the default now) | the light tuning on 26 games n = 2178 + 50k, which stage 2 does not play (n = 3 mod 5); the name says the setting, the file the game's line (`--cap 0.01` stops the sound search after the finder, whose game is the line's `lns` or `restricted`) |
| `tables.py`, `tables.txt` | `./bench/finders/tables.py > tables.txt` | the tables the entry quotes |
