# Finders for good games that are not sound (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-17.  Concluded 2026-10-17.

**Question.**  Can searches that are not sound, which may miss opt(n), find better games quickly, and does a better
starting game make the sound search much cheaper?  **Answer.**  Yes to both.  Large neighbourhood search (LNS) finds the
optimum in 64.4% of 253 games of 2176-3440 in 5 CPU s of one thread, where the sound search itself finds it in 20.6%.
The sound search seeded with opt(n) takes a median 0.474 of the nodes it needs from the start incumbent, and seeded
with LNS's game after 1 s it takes 0.437 of them in total, 0.48 of the CPU with the finder and the start counted.

## Method

The ledger keeps a lower bound L(n), a real game the referee replays, and an upper bound U(n) for many boards.  A lower
bound needs only a legal game, so any method may find it, sound or not; only an upper bound has to be proven.  The
start incumbent falls short of opt(n) by a median 227 on 2176-3440 ([the
landscape](../the-landscape-past-the-chain-standalone-bounds-and-searches-2026-10/README.md)), a neighbour's heuristic
game closes 40% of that ([seeding](../seeding-the-start-from-a-neighbours-heuristic-game-2026-10/README.md)), and a
standalone search finds its optimum after a median 77% of its time, so stopping it early is no cheap way to a good game.

The runs were on the laptop on 2026-10-17, with the test-profile build of `wall`, one process a game and four at once
under `nice -n 10`, against the committed chain.  The commands `wall sound` and `wall finders` (commit fa3127a) are
defined in the module comment of `examples/wall/finders.rs`, which defines every field of a line.  Times are CPU seconds
of the process, since the laptop was shared, and searches are compared by their nodes as well.  The laptop searched at
about 7 thousand steps a second a thread, as the cloud did in the landscape's searches (6.8 thousand).

* **Stage 1, the most a better start can save.**  The sound search of 40 games of 2176-3440, standalone (one thread, no
  ceiling, no seed from n − 1), from (i) the start incumbent and (ii) the known optimum, given as the seed.  22 games
  are of the landscape's standalone searches (5 to 300 s there, no primes or repeats); 18 others are from every band
  of 250 in n, picked by the nodes of the chain's search of them (100,000 to 1,200,000).  36 have a start short of the
  optimum and 4 have none (13% of all games have none).
* **Stage 2, two finders and a control**, from the same start incumbent, each for 5 CPU s on one thread, on every 5th
  game of 2176-3440 (253 games, none with a start that meets the root bound), and again at 20 s on every 25th (51
  games).  **LNS** keeps a random 70-90% of the incumbent's promotions fixed, searches the rest exactly as a unit of the
  engine for at most 1,024 steps, takes any better game and repeats.  **Restricted** is the sound search on the
  candidates in the start incumbents of n − 1, n and n + 1 and the 32 best others, going on from its best game when it
  finishes early.  The **control** is the sound search itself from the start for the same time.  The referee replayed
  every game any of them returned at its score, and none was above opt(n).
* **Stage 3**, seeding the sound search of the 40 games of stage 1 with the game LNS returns after 5 s and after 1 s,
  counting the finder's time and the start's.
* **Tuning**, light, on 26 other games (n = 2178 + 50k) at 5 s.  LNS keeping 70-90% fixed (mean shortfall 15.5) beat
  50-80% (18.3), 60-95% (17.2) and 80-95% (28.4), and 4,096 steps a unit changed nothing.  Restricted did better with
  the pool taken by the root bound's heaviest set (164) than by the larger numbers (255), and the number of others it
  begins with (8, 32 or 128) changed nothing.

## Results

**1. The most a better start can save (stage 1).**  All 80 searches finished: (i) took 5,333 CPU s (5 to 531 s a game),
(ii) 2,135 s.  The nodes of (ii) over (i):

| games | median | mean | range | in total (nodes, CPU) |
|---|---|---|---|---|
| all 40 | 0.474 | 0.521 | 0.041-1.000 | 0.380, 0.400 |
| start short of opt(n) (36) | 0.418 | 0.468 | 0.041-0.968 | 0.370, 0.393 |
| start is opt(n) (4) | 1.000 | 1.000 | 1.000 | 1.000, 0.954 |
| (i) took more than 300 s (6) | 0.192 | 0.222 | 0.061-0.513 | 0.177, 0.228 |

By the start's shortfall the median ratio is 0.68 (1-150, 7 games), 0.60 (151-300, 11), 0.33 (301-500, 11) and 0.25
(501 and over, 7).  So opt(n) as a seed halves the nodes in the median game and cuts the cost of the dear games to
a fifth: the most any finder can buy, more where the start is worse and the search longer, nothing where the start is
already optimal.

**2. The finders against the control (stage 2).**  The shortfall (opt(n) less the game's score) of the 253 games
at 5 CPU s:

| | median | mean | p90 | exact | better than the control | worse | CPU s a game |
|---|---|---|---|---|---|---|---|
| start | 227 | 328.7 | 782 | 12.6% | | | 3.0 |
| LNS | 0 | 27.9 | 96 | 64.4% | 77.9% | 3.6% | 5.0 |
| restricted | 76 | 189.7 | 629 | 33.2% | 63.2% | 6.3% | 5.0 + 6.1 for the neighbours' starts |
| control | 195 | 297.4 | 752 | 20.6% | | | 5.0 |

* The 94 games that need a search of their own (no primes, no repeats) give alike numbers (LNS mean 20.7, exact 69%;
  control 311, 20%).  Where LNS misses it falls short by less (28 against 297).  On 3176-3440, where no start is exact,
  its median shortfall is 8 against the control's 504.
* LNS makes about 2,560 units a game (2 ms each), 99.9% of them searched to the end, and finds 5.4 improvements; in
  stage 3 its last improvement came at a median 0.32 s, and within 1 s in 35 of 40 games.
* Restricted beats the control but not LNS, and costs more.  It finished the whole sound search in 3 games where the
  control did in 8.
* At 20 s on 51 games, the mean shortfall of LNS goes from 29.0 (57% exact) at 5 s to 20.8 (65%), restricted's from
  170.8 (29%) to 121.5 (49%), and the control's from 325.7 (16%) to 287.1 (26%).  So LNS has most of what it gets in the
  first seconds, and the games it misses it keeps missing: its neighbourhoods probably do not reach opt(n) of about
  a third of the games.

**3. The sound search seeded with LNS's game (stage 3).**  The nodes of the search over (i)'s, and the CPU in all (the
start, the finder and the search):

| LNS budget | seed's mean shortfall | nodes, median | nodes, in total | of (ii)'s | CPU in all, in total | games that cost less in all |
|---|---|---|---|---|---|---|
| 5 s | 14 (start 294) | 0.496 | 0.416 | 1.10 | 2,656 s of (i)'s 5,434 (0.49) | 32 of 40 |
| 1 s | 25 | 0.567 | 0.437 | 1.15 | 2,628 s of 5,434 (0.48) | 36 of 40 |

The median seeded search takes 0.50 and 0.57 of (i)'s nodes.  In total the seeded searches are within 10 to 15% of what
opt(n) gives (0.416 and 0.437 of (i)'s nodes against 0.380), and the finder pays for itself in all but a few games:
those whose search is shorter than the finder's time (a game of 5 s search costs 11 s) or whose start is opt(n).  A
1 s finder is the better trade: the CPU in all is the same as with 5 s but it costs more than (i) in 4 games, not 8, at
most 1.16 times it (1.8 with 5 s).  A seeded search may cost more nodes than (i) (at most 1.06 times in one game), since
a search's tree is not monotone in its incumbent.

## Decision

* The sound search from opt(n) takes a median 0.47 of the nodes of the search from the start incumbent (0.19 for
  the six games that take more than 300 s), and 0.38 in total: a better start makes the search much cheaper.
* LNS is the finder to use: it finds opt(n) in 64% of the games in 5 CPU s, and has most of its gain in 1-2 s.
  Restricted on the neighbours' starts is better than the control and worse than LNS.  Lower bounds can be found by a
  finder for a fraction of the time of the search they would shorten.
* LNS is the ledger's finder since 2026-10-17 (`--lns-seconds`, 1 s by default; its code is `examples/common/lns.rs`):
  [the finder in the
  ledger](../the-ledger-bounds-from-both-sides-2026-10/README.md#later-2026-10-17-the-finder-in-the-ledger).
* The commands `wall sound` and `wall finders` were removed from the tree on 2026-10-18; they build at commit 9d9dae3
  (`git checkout 9d9dae3`, then `cargo build --release --example wall` in `rust/`).

Not measured, and what would extend the study: games past 3440, where no optimum is known to score the finders
against; LNS from the proven game of n − 1 or from a neighbour's heuristic game; a finder budget that scales with the
game (the start incumbent alone costs 1 to 7 s), which would shrink the 8 of stage 3's 40 games that cost more CPU
than (i) at 5 s; nogood tables carried across units, adaptive neighbourhoods and perturbation, for the third of the
games LNS does not solve; and threads.  The study spent 6.8 core-hours.

## Files

`run.py` runs a command over a list of games, four processes at a time, and appends each game's line to the stage's file
as it ends; an interrupted run goes on where it was.  `tables.py` makes the tables of the entry from the files and the
landscape entry's `cal.jsonl`, and its output is `tables.txt`.  The commands below ran from `rust/` at commit 9d9dae3,
with `wall` built by `cargo build --profile test --example wall`.

`GAMES1` is the 40 games of stage 1: 2180 2196 2200 2264 2312 2397 2416 2430 2440 2451 2478 2556 2560 2565 2592 2601
2632 2640 2691 2696 2700 2704 2712 2720 2799 2849 2891 2925 2964 2976 3000 3003 3010 3064 3128 3129 3213 3260 3267 3272.

* `stage1.jsonl`: stage 1, the sound search of each of the 40 games, one thread, no seed from n − 1 and no ceiling, from
  the known optimum (`opt`: (ii)) and from the start incumbent (`start`: (i)); 80 lines.  Made by `run.py sound
  stage1.jsonl --from opt --cap 600 --shuffle GAMES1`, then the same with `--from start --cap 900`.
* `stage2.jsonl`: stage 2, the three methods and the start, each of the 253 games 2176, 2181, ..., 3436, 5 CPU s a
  method (it predates the field `lns_found_ms`).  Made by `run.py finders stage2.jsonl --cpu 5 --shuffle 2176-3440/5`.
* `stage2-20s.jsonl`: the same at 20 CPU s a method, on every 25th game (51).  Made by `run.py finders stage2-20s.jsonl
  --cpu 20 --shuffle 2176-3440/25`.
* `stage3-lns-5s.jsonl`, `stage3-lns-1s.jsonl`: stage 3, the sound search of the 40 games seeded with the game the LNS
  finder returns after 5 and after 1 CPU s.  Made by `run.py sound stage3-lns-5s.jsonl --from lns --cpu 5 --cap 900
  --shuffle GAMES1`, and with `--cpu 1`.
* `tuning/*.jsonl`: the light tuning on 26 games n = 2178 + 50k, which stage 2 does not play (n = 3 mod 5).  The name
  says the setting, the file holds the game's line (`--cap 0.01` stops the sound search after the finder, whose game is
  the line's `lns` or `restricted`).  Made by `run.py sound tuning/NAME.jsonl --from lns --cap 0.01 --cpu 5 2178-3428/50
  [--fix LO-HI] [--unit-nodes K]`, and with `--from restricted` and `[--extra K] [--pool value|chosen]`.
* `tables.py`, `tables.txt`: the tables the entry quotes, made by `tables.py > tables.txt`.

The `tuning/` files are `lns-fix50-80-nodes1024`, `lns-fix60-95-nodes1024`, `lns-fix70-90-nodes1024`,
`lns-fix70-90-nodes4096` and `lns-fix80-95-nodes1024` for LNS, and `restricted-extra8-value`,
`restricted-extra32-value`, `restricted-extra128-value` and `restricted-extra32-chosen` for restricted (`value` was the
default pool while the `value` files were made, `chosen` is the default since).

## Commits

* fa3127a: `wall sound` and `wall finders`, their tests, and `examples/common/known.rs` and `rng.rs`, which `tune` and
  `wall` share.
* 4621d97: the finder in the ledger: LNS moves to `examples/common/lns.rs`, shared with `wall finders`.
* dfd524b: the finished experiment commands of the wall example removed (2026-10-18); 9d9dae3 is the last commit that
  has them.
