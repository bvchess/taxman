# Nogoods from the finder's work (2026-10)

Part of the [lab notebook](../../README.md).

**Question.**  Can a burst of the finder's work produce nogoods that lower a game's upper bound?  The finder (large
neighbourhood search, [the finders entry](../finders-for-good-games-that-are-not-sound-2026-10/README.md)) fixes part of an
incumbent's promotions and runs the sound search on the rest as a unit, over and over; every one of those searches
learns nogoods (sets of candidates that cannot all be promoted from the root, which hold everywhere in the search of
the game).  Any nogoods with any non-negative multipliers give a valid upper bound by Lagrangian relaxation of "at most
`|N| - 1` members of `N`".  Two earlier studies measured what such bounds can reach at n ≤ 1300: a Lagrangian over
conflicts with the GF(2) intersection as the inner problem closed 7-36% of the root's GF(2) gap
([matroid study](../matroid-intersection-bounds-2026-10/README.md)), and the cover LP over all conflicts closed 46% on average
over 880-1300, a share that fell 3.8 points per 100 in n ([how far nogoods can go](../how-far-can-nogoods-go-2026-10/README.md)).
Here the nogoods come from searches, at n = 3000-4800, and are a few seconds' work.

**What was run.**  On the laptop on 2026-10-18, with the test-profile build of `wall`, one process a game and four at
once under `nice -n 10` (the data, commands and tables are in [solver/notebook/search/nogoods-from-the-finders-work-2026-10/](README.md);
the command `wall nogoods`, new in commit 05e3744 and changed by d47ea73, is defined in `examples/wall/nogoods.rs`).
For each game: the standalone start incumbent; the finder for S CPU seconds, harvesting the nogood table of every
unit's searcher; as the control, the sound search standalone from the same start for S CPU seconds, harvested the
same way; every harvested nogood checked to fail to promote from the root (`promote_all_using`); the root's GF(2) bound
(the baseline); and a Lagrangian bound over each pool (the finder's, the search's, both) whose inner problem is the
crate's own GF(2) bound on the same matroids with each candidate `x` weighing `64x` less the multipliers of the nogoods
holding it (`Gf2Graph::reweighted`, tested against enumeration of the common independent sets on the boards up to 200 that have 16 elements or fewer).  The
multipliers are fitted by subgradient steps (Polyak's rule towards the incumbent plus one, damping halved after 20 steps
without a lower bound, a direction with 0.9 of the last direction added) and the best bound seen is the bound: K = 200
steps is the answer, and the same fit run on to L = 2,000 steps shows how far it goes.  Any multipliers make a bound,
so nothing about the fit can make it wrong, only loose.  The tables never forget (the search's tables are not cleared
every 50,000 steps, as a real search's are, so that every nogood it learned is in its pool).  Two small changes in
`solver/frame/src`, neither of which changes a search: `TaxSearcher::nogoods` (the table a searcher holds) and
`Gf2Graph::reweighted`.

* **Eight games with known optima**, 3016, 3156, 3216, 3276, 3311, 3336, 3381 and 3396: games of 3000-3440 that the
  chain searched (not primes, not repeats, not stopped at the ceiling), picked for a spread of root gaps (`upper` less
  `opt`: 766 to 1,427; mean 1,113), with S = 10 and S = 120.
* **Four open games**, 4300, 4522, 4761 and 4788, with S = 10 and S = 120, against the best upper bound of
  `results/bounds.jsonl` (4300 has no line; the store starts at 4400).

**1. The share of the root gap that the bound closes.**  Mean over the eight games of `(upper - bound) / (upper - opt)`:

| S | pool | distinct nogoods (median) | K = 200 steps | L = 2,000 steps |
|---|---|---|---|---|
| 10 s | the finder's | 2,888 | 0.380 (0.25-0.46) | 0.394 |
| 10 s | the search's | 365 | 0.317 (0.22-0.41) | 0.328 |
| 10 s | both | 3,166 | 0.386 | 0.399 |
| 120 s | the finder's | 13,060 | 0.353 (0.25-0.45) | 0.366 |
| 120 s | the search's | 518 | 0.347 (0.22-0.44) | 0.360 |
| 120 s | both | 13,318 | 0.359 | 0.369 |

At 10 s the finder's nogoods close more than the search's in all eight games (a mean 0.38 against 0.32), with eight
times as many: 19,840 nogoods learned (2,775 distinct) in 3,220 units and 74,600 steps against 372 in 47,800 steps.  At
120 s the finder's pool is 13,000 distinct nogoods (240,700 learned, 41,000 units, 869,000 steps) and closes no more
than at 10 s (0.35 against 0.38; it is ahead of the search's in five of eight games).  The 120 s run goes through the
same units as the 10 s run (the seed is the same) and then on, so its pool holds the 10 s pool and its best possible bound
is no worse: it is the fit that falls short, which 2,000 steps do not carry to the bound a pool of 13,000 nogoods
allows.  What is left is large: the mean bound of both pools is still 690 (10 s) and 720 (120 s) above the optimum (of a
mean gap of 1,113), and no fit came near it (none proved an incumbent optimal).  Of the games,
3396 closes most (0.44-0.46, gap 1,234) and 3156 least (0.25, gap 1,084); the share is not a function of the gap.  The
finder reaches the optimum in six of eight games at either S, the control in none at 10 s and one at 120 s.  No
harvested nogood could be promoted from the root (the 48 lines of the data check 463,000, the pools of both added up),
and no bound was below the optimum or the incumbent.

The costs: the nogoods are free (a table read after each unit); 200 steps of the fit take 0.3-0.8 s a pool at 3016-3396
(about 3 ms a step; the root's GF(2) bound alone takes 2.1 ms) and 2,000 steps 1.4-3.5 s, and at 4300-4800 0.7-2.5 s and
3.3-8.0 s.

**2. The fit decided the first answer.**  The first pass used plain subgradient steps.  It put the finder's 120 s pool
below its 10 s pool, and below the search's (means at K: finder 0.286, search 0.348, both 0.292; at 10 s 0.340, 0.317,
0.341; the files are `plain-*.jsonl`).  On the 60 s pools of 3276, 3336 and 3381, saved to files, a smaller start for the
damping and a shorter wait before halving it were worse, and 0.9 or 0.95 of the last direction added to the subgradient
(a heavy ball) raised the finder's bound at K from 0.297, 0.257, 0.312 to 0.387, 0.325, 0.396 (0.98 and above no
better) and left the search's few hundred nogoods where they were (0.31-0.32, 0.27, 0.32).  All the lines of the tables
are with momentum 0.9; the pools of a run differ a little from a run to another (the budgets are in CPU seconds).  The
subgradient method is the weak part: the cover LP of the earlier study, which the fit approximates, was solved by column
generation, and for a pool of 13,000 nogoods its value is probably well above what 2,000 steps reach.

**3. The open games.**  Bounds as scores, from the best of the three pools and of K and L, against the root's GF(2)
bound and the store:

| n | root GF(2) bound | S | best nogood bound | off the root | store's upper bound (rule) | nogood bound below it by | over the best lower bound |
|---|---|---|---|---|---|---|---|
| 4300 | 5814506 | 10 | 5813918 | 588 | none | | 1,048 |
| 4300 | | 120 | 5813994 | 512 | | | 1,124 |
| 4522 | 6432129 | 10 | 6431399 | 730 | 6431916 (ceiling) | 517 | 1,369 |
| 4522 | | 120 | 6431436 | 693 | | 480 | 1,406 |
| 4761 | 7132771 | 10 | 7132091 | 680 | 7131167 (ceiling) | -924 | 1,283 |
| 4761 | | 120 | 7132124 | 647 | | -957 | 1,316 |
| 4788 | 7214567 | 10 | 7213796 | 771 | 7214319 (ceiling) | 523 | 1,177 |
| 4788 | | 120 | 7213831 | 736 | | 488 | 1,212 |

The nogood bound is 500-770 under the root's, as at the known games.  It beats the store's upper bound at 4522 and 4788
(by about 500, ceilings that rest on one to three proven neighbours), not at 4761, whose ceiling rests on 4755 and
4757 and is 920-960 under the nogood bound.  In no game is it near the best lower bound: 1,050-1,400 above, so no board is closer to a
proof.  At 120 s the finder's pool is no better than at 10 s (4522: 6431436 against 6431399); the search's pool is as
good as the finder's at 4300 and 4522.  Unlike the ceiling, the nogood bound needs no proven neighbour.

**4. What it says.**  Yes, the finder's work produces nogoods that lower a game's upper bound: a burst of 10 CPU
seconds, a few thousand nogoods and a fit of 200 steps close 0.25-0.46 of the root's GF(2) gap at 3016-3396 and
500-770 off it at 4300-4800, a valid bound within the frame model.  Three things keep it from mattering yet.  (i) The
sound search's own nogoods from the same time do almost as well (0.32 against 0.38 at 10 s, 0.35 against 0.35 at
120 s) with a tenth or less of the pool, so the finder is not the better source; the bound comes from nogoods, not from
the finder's games.  (ii) What is left is far from a proof: 690 above the optimum at the known games and over 1,000
above the lower bound at the open ones, where a proof needs the two to meet.  (iii) A root bound lowered by a third is
the kind of result [A1](../the-a1-bound-in-a-branch-and-bound-the-e3-pilot-2026-10/README.md) found not to change the trees (its
multi-field bound closed 0.4-0.6 of the gap at the roots, and the trees fell by a factor that fell as the gap grew), and
the Lagrangian bound over the nogoods in the search (the matching as its inner problem) gained nothing in
[the tuning campaign](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md); nothing here measures trees.

As a result in the store the bound would need a checkable certificate (the nogoods, each checked by
`promote_all_using`, and the multipliers: the bound is then one weighted GF(2) solve); the store takes none yet, so
nothing was appended to `results/bounds.jsonl`.  On the earlier studies' numbers: the share at n ≤ 1300 fell with n,
and a straight line through it would be below zero from about n = 2300; the shares here, 0.25-0.46 at 3016-3396 from
pools of a few thousand nogoods, are above it, and the cover LP over all conflicts (never solved past 1300) is at least
as good as any pool.

**Commits.**  05e3744 (the command and the two accessors), d47ea73 (the fit's momentum); the data are in the commit
that has this entry.  The `lagrange` command of the removed `matroid` example (`git show
8753189^:solver/frame/examples/matroid.rs`) was the model, with the research intersection of `common/meet.rs` replaced by the
crate's own GF(2) bound.

## Later (2026-10): the command removed from master

The command `wall nogoods` was removed from the tree on 2026-10-18.  It builds at commit 9d9dae3 (`git show
9d9dae3:rust/examples/wall/nogoods.rs`): `git checkout 9d9dae3`, then `cargo build --release --example wall` in `rust/`.
The harvest of nogoods and the fit of the multipliers stay in `examples/wall/nogoods.rs`, which `wall choice-prepare`
and `wall choice` use.

## Files

The runs were made on the
laptop on 2026-10-18, one process a game and four at once under `nice -n 10`, with the test-profile build of `wall`
(`cargo build --profile test --example wall` in `rust/`), from `rust/`, against the committed chain
(`results/strict.jsonl`).  The command `nogoods` was removed from master on 2026-10-18; it builds at commit 9d9dae3
(`git checkout 9d9dae3` first), where it is defined in the module comment of `examples/wall/nogoods.rs`, which defines
every field of a line.  Every time is the CPU seconds of the process (or the wall seconds of a bound, which is one
thread), and the budgets of the finder and of the control are CPU seconds, so the pools of two runs of one game differ a
little (a budget cuts a search at a different step).

`run.py` runs `wall nogoods` over a list of games, four processes at a time, and appends each game's line to the file
as it ends (a run that is interrupted goes on where it was); it takes the helpers of `../run.py`.  `tables.py` makes
the tables of the entry from the files and `../../../../results/bounds.jsonl`; its output is `tables.txt`.

### Files and the commands that made them

The eight games with known optima are 3016, 3156, 3216, 3276, 3311, 3336, 3381 and 3396: games of 3000-3440 that
the chain searched (not primes, not repeats, not stopped at the ceiling), picked from `../stage2.jsonl` for a spread
of root gaps (766 to 1,427; `upper` less `opt`).  The open games are 4300, 4522, 4761 and 4788.

| file | command | what it is |
|---|---|---|
| `known-10s.jsonl`, `known-120s.jsonl` | `./bench/finders/nogoods/run.py known-10s.jsonl --cpu 10 --shuffle 3016 3156 3216 3276 3311 3336 3381 3396`, and with `--cpu 120` | the eight games with known optima, the finder and the sound search 10 and 120 CPU s each |
| `open-10s.jsonl`, `open-120s.jsonl` | `./bench/finders/nogoods/run.py open-10s.jsonl --cpu 10 4300 4522 4761 4788`, and with `--cpu 120` | the four open games |
| `plain-known-10s.jsonl`, `plain-known-120s.jsonl`, `plain-open-10s.jsonl`, `plain-open-120s.jsonl` | the same four commands, made before the fit had momentum (commit 05e3744: plain subgradient steps) | the first pass, kept because it shows that the fit decides the answer for a large pool: with plain steps the finder's pool of 120 s, thirteen thousand nogoods, closed less of the gap than the pool of 10 s (the entry has the numbers) |
| `tables.py`, `tables.txt` | `./bench/finders/nogoods/tables.py > tables.txt` | the tables the entry quotes |
