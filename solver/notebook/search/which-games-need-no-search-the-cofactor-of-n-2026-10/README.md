# Which games need no search: the cofactor of n (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-24.  Concluded 2026-10-24.

**Question.**  Can we tell in advance which games of the chain need no search?  **Answer.**  Mostly, from n alone.
Write n = c · P with P the largest prime factor of n, and call c the *cofactor*.  Whether a game was searched depends
almost only on c.  A game whose cofactor is 1, a prime, twice a prime or 9 was never searched.  This rule names 1,816 of
the 2,126 games of 1301–4979 that were settled without a search (85.4%), and none of the games that were searched.  The
other 310 are mostly settled at the start, by the ceiling, which no cofactor predicts.  The rule forecasts the chain's
work and settles no game: the solver still settles each game by its own checks.  Its conditions (a repeated search
problem for a prime or twice a prime, the ceiling for 9) hold in every recorded game from 188 to 4949, and the theory
states the rule there ([the cofactor rule](../../../../docs/theory/arrival.md#the-cofactor-rule)).

## Method

* **The games.**  The first completed line of each n of the [committed chain](../../../../results/README.md#optimaljsonlgz)
  (`results/strict.jsonl`, games to 4949) and, for 4950–4979, of the running chain's file, which the study reads on
  2026-10-24 and never writes.  The study leaves out the 382 lines of method `known`, games 3984–4783 that the [bounds
  store](../../../../results/README.md#boundsjsonl) had proven and the chain took as copies, since no search was in
  question for them.  That leaves 3,297 games in 1301–4979.
* **Searched and free.**  A game is *searched* when the method of its line is `search`.  It is *free* when it was settled
  without a search, by one of four methods: `prime` (the [prime formula](../../../../docs/GLOSSARY.md#the-game)),
  `repeat` (a [repeat](../../../../docs/GLOSSARY.md#results-and-trust) of an earlier game's search problem), `start`
  (the start reached the [ceiling](../../../../docs/GLOSSARY.md#the-game) before the first node) and `trivial`.
* **The cofactor** of n is n divided by its largest prime factor: 4971 = 3 · 1657 has the cofactor 3, and a prime has the
  cofactor 1.
* **The classes** come from 2176–4979, for the cofactors with at least 5 games there: *never* searched, *always*
  searched, or *mixed*.  The rule was stated from them, so 1301–2175 lies outside them.  Games below 1301 were not
  looked at.
* **The held-out test.**  Add to the rule the cofactors it does not name that are free in at least 90% of at least 5
  games of 1301–3500, and count the rule on 3501–4979, which those cofactors were not chosen from.
* **The script** `cofactor.py` recomputes every number below from the two files, and `cofactor.txt` is its output.  A
  second count of the headline numbers, with its own loader and its own factoring, gave the same 2,126, 1,816 and 0.

## Results

### Which cofactors are searched

Of the 2,422 counted games of 2176–4979, the cofactors with at least 5 games hold 2,131.

| class | cofactors | games | searched | repeat | start | prime |
|---|---|---|---|---|---|---|
| never searched | 37 | 1,382 | 0 | 996 | 89 | 297 |
| always searched | 55 | 568 | 568 | 0 | 0 | 0 |
| mixed | 20 | 181 | 60 | 11 | 110 | 0 |
| fewer than 5 games | 131 | 291 | 229 | 23 | 39 | 0 |

* **Never searched** are the cofactors 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 13, 14, 15, 17, 19, 22, 23, 26, 29, 31, 34, 37, 38,
  41, 42, 43, 44, 46, 47, 53, 58, 59, 62, 68, 74, 75 and 82.  Cofactor 3 gives 115 games of 115 free.
* **Always searched** are the cofactors 8, 12, 16, 20, 21, 24, 25, 30, 32, 35, 36, 39, 40, 45, 48, 49, 51, 54 and 55,
  among others (`cofactor.txt` lists all 55).  Cofactor 8 gives 47 games of 47 searched.  Three times a prime p is always
  searched for p = 7 and for every prime from 13 to 31 (21, 39, 51, 57, 69, 87 and 93), and for 13 of the 14 games of 33
  (p = 11), where 6, 9 and 15 (p = 2, 3 and 5) never were.
* **Mixed** cofactors, each with its searched games out of its games: 18 (2 of 22), 27 (3 of 18), 28 (2 of 17), 33 (13
  of 14), 50 (4 of 10), 52 (4 of 10), 63 (1 of 9), 66 (2 of 9), 72 (6 of 9), 76 (1 of 7), 92 (2 of 6), 98 (1 of 6), 99
  (1 of 6), 102 (3 of 7), 108 (1 of 5), 112 (2 of 5), 114 (3 of 5), 117 (4 of 5), 130 (4 of 6) and 169 (1 of 5).  A game
  of these that was not searched was nearly always settled at the start (110 of 121).

The cofactors that were never searched, by family, with those of fewer than 5 games included (53 cofactors, 1,410
games):

| family | cofactors | games | repeat | start | prime |
|---|---|---|---|---|---|
| c = 1 (n is prime) | 1 | 297 | 0 | 0 | 297 |
| c prime | 19 | 643 | 643 | 0 | 0 |
| c twice a prime | 15 | 337 | 337 | 0 | 0 |
| c = 9 | 1 | 42 | 0 | 42 | 0 |
| other | 17 | 91 | 33 | 58 | 0 |

The family "other" holds 15, 42, 44, 68 and 75, which have at least 5 games each, and twelve cofactors of fewer.  The
prime 2 makes 4 twice a prime.

### The rule

The rule says: no search when c is 1, a prime, twice a prime or 9.

| games | free | named | share | searched games named |
|---|---|---|---|---|
| 1301–4979 | 2,126 | 1,816 | 85.4% | 0 |
| 1301–2175 | 561 | 497 | 88.6% | 0 |
| 2176–3500 | 851 | 726 | 85.3% | 0 |
| 3501–4979 | 714 | 593 | 83.1% | 0 |

The share falls from range to range.  Where the rule is silent, a game was searched in 1,171 of 1,481 cases (79.1%).  Of
the 310 free games the rule misses, 254 are settled at the start and 56 are repeats.  The cofactors 15, 18, 27, 28, 44,
42, 68, 75 and 63 hold 40, 23, 21, 19, 18, 14, 11, 9 and 9 of them.

### The held-out test

Three cofactors are free in at least 90% of at least 5 games of 1301–3500 and not named: 15 (28 games, all settled at
the start), 44 (12 games, 1 searched, 11 repeats) and 68 (7 games, 5 repeats and 2 at the start).

| games | free | named with 15, 44 and 68 | share | searched games named | use |
|---|---|---|---|---|---|
| 1301–2175 | 561 | 515 | 91.8% | 1 (1364 = 44 · 31) | fitted |
| 2176–3500 | 851 | 754 | 88.6% | 0 | fitted |
| 3501–4979 | 714 | 616 | 86.3% | 0 | held out |

On the games the three were not chosen from, they lift the share from 83.1% to 86.3% and add no wrong call.  The one
wrong call is in 1301–2175, game 1364.

### The setup against the problem before it

One guess for why the cofactor decides, not tested: n = c · P is paid through its large prime P, whose frame is small and
apart from the rest of the board, so adding n leaves the search problem as it was.  The one check that costs nothing
compares the setup of each game of 2176–4979 with that of the latest game before it that has one (a prime has none).  The
digest names the [search problem](../../../../docs/GLOSSARY.md#the-frame-model).

| cofactor class | method | games | same digest | same candidates | same max_total | same greedy_sum |
|---|---|---|---|---|---|---|
| never searched | repeat | 1,013 | 1,013 | 1,013 | 1,013 | 1,013 |
| never searched | start | 100 | 0 | 83 | 19 | 82 |
| always searched | search | 759 | 0 | 192 | 357 | 59 |
| mixed | repeat | 17 | 17 | 17 | 17 | 17 |
| mixed | search | 98 | 0 | 37 | 29 | 15 |
| mixed | start | 138 | 0 | 45 | 52 | 42 |

* **The repeats are exact.**  All 1,030 repeats of 2176–4979 have the digest of the game they repeat, which is n − 1 for
  925 of them and n − 2 for 105 (n − 1 being prime).  That is what a repeat is, so the check confirms the data and no
  more.
* **The games of the cofactors that are always searched change the problem.**  None keeps its digest, 75% change the
  candidates, 53% the promotion limit (`max_total`) and 92% the greedy sum.
* **The games of the never-searched cofactors that were settled at the start** (100, 42 of them with cofactor 9) keep the
  candidates in 83 and the greedy sum in 82, and change the limit in 81.  The problem is nearly as it was, and the ceiling
  settles the game.

So the data agree with the guess, and no more than agree: nothing here shows why c, rather than something else about n,
leaves the problem as it was.

### The rule's conditions, from 188

Added 2026-10-24.  The rule's families stand for checkable conditions: a game whose cofactor is a prime or twice a prime
poses the search problem of game n − 1, or of n − 2 when n − 1 is prime, and a game of cofactor 9 is at its ceiling.
`conditions.py` checks them on every game of the committed chain (to 4949), below 1301 too, from the digest of each
game's setup and the scores, in a few seconds; `conditions.txt` is its output.

* **From 188 on, no game breaks them.**  Every recorded game of the four families obeys, outside the 382 copies of
  3984–4783, which record no setup.  From 188 the four families hold 2,481 of the 2,856 games settled without a
  search (86.9%), and none that was searched.
* **Below 188, 38 games break them**, the last 187 = 11·17, which was searched.  The small games settled by the trivial
  case (4 to 26) count as breaks, since they have no setup to compare.
* **Layer 2 marks the boundary.**  Board 186 is the last whose layer 2 has a frame with a freedom ([the square
  theorem](../../../../docs/theory/frame-model.md#the-square-theorem)): its frame of 11 and 13 holds the squares 121 and
  169, and the frame of every other prime up to 89 holds none.  The source 187 = 11·17 joins them.  From 28 on, every
  game whose layer 2 has no freedom, in its own game and in the game it is compared with, keeps the conditions (1,946
  games, below 188 too); the 25 games that break them all have a freedom there; and 19 games with one keep them.  The
  script forms the frames of layer 2 itself, and finds the square theorem's 80 boards with a freedom.

## Decision

No change to the solver.  Its own checks (the prime formula, the repeat check and the ceiling) already settle these games
in 0.1 to 0.9 s each, as in 4950–4979, and a rule fitted to data proves nothing.  The rule serves to forecast: it says
which of the games ahead of the chain are free, so how many searches are left.

What would reopen it:

* **A wrong call.**  A game the rule names that was searched.  The rule has none in 1301–4979, and its three added
  cofactors have one (1364).
* **A reason.**  The frames of a repeat and of a searched game, compared, which would turn the rule into a theorem about
  the game before it, or show where it fails.
* **More games.**  The rule is counted to 4979, and the games below 1301 are untested.  Run the script again on the
  committed chain once it passes 4979.

## Files

* `cofactor.py`: the study.  From the top of the repository:

  ```text
  solver/notebook/search/which-games-need-no-search-the-cofactor-of-n-2026-10/cofactor.py results/strict.jsonl \
      ~/taxman_work/chain-data/results/strict.jsonl --last 4979
  ```

  Each file supplies the games past the last game of the file before it.  Once `results/strict.jsonl` holds the games to
  4979, it can run alone.
* `cofactor.txt`: its output, the source of every number above but those of the conditions.
* `conditions.py`: the check of the rule's conditions, from the top of the repository with no argument (the committed
  chain); `--from N` starts at N.  `conditions.txt`: its output.
