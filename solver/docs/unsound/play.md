# Playing a game

Part of the [unsound mode](../unsound.md).

## How a game is played

A game is set up as a proven run sets it up: the board and its frames.  Four methods follow, each named in the line:

* **`trivial`**: a board with no promotion candidates has the moves with no promotions.
* **`prime`**: a prime n plays the prime formula on the moves for n − 1.
* **`repeat`**: a game whose search problem a line of the run or of the file posed, as a proven run finds a repeat,
  plays that line's promotions on its own board.
* **`search`**: any other game takes the start of a proven search, seeded with the moves for n − 1, then the rounds of
  its level of effort.

A `prime` or `repeat` game searches nothing, and keeps what the line it starts from falls short by, if it does.  The
moves for n − 1 are the run's line, else the file's, or the [best file's](best.md) if that scores more, else the
built-in table's, and `--from-known` takes the table's always.  A [game played twice](shortfalls.md#games-played-twice) is also searched with nothing from the games below it.

The setup does not depend on the moves for n − 1, so a run makes each game's setup, with the dive of a searched game's
start and the game's [bound](shortfalls.md#the-bound-and-the-gap), on a thread of its own while the game before it
plays.  On many threads they would otherwise be a large part of the time of a run at a low level of effort.

A line depends on the game, its start, the moves it starts from, the level, the seed and the options, and not on the
threads or the machine, but for its times and threads.

### The rounds

A round is 64 neighborhoods (`NEIGHBORHOODS` in `search::finder`) of the large neighborhood search of the finder.  A
neighborhood keeps fixed a share of the incumbent's promotions, drawn from the range of `--fix`, and searches the rest
exactly as a unit of at most 1,024 steps, starting from the incumbent as the round starts.  A smaller share fixed
leaves a neighborhood more to search.  Neighborhood k of round r draws its choices from a generator of the seed, n, r
and k alone, and the best set of the round, the lowest k's among equals, is the next incumbent.

The steps of a neighborhood depend on its unit alone, so the rounds are the same on any number of threads.  A stop
signal ends the run between rounds.

The threads do not wait for a round to end.  They search the neighborhoods of the next few rounds from a guess of the
incumbents those rounds start from, that the neighborhoods still running find nothing better, and search again any
neighborhood whose guess proves wrong.  So a round on many threads takes about its share of the threads' time, not as
long as its longest neighborhood, and it is the same round that it is on one thread
([finder.rs](../../frame/src/search/finder.rs), "Ahead of the end of a round").

### The level of effort

A level of effort is a budget of work for the rounds, and each of the six levels, 1 to 6, is twice the one below.  Level
L runs rounds until their work reaches a budget of 2^L × 65,536, the steps of a round of 64 neighborhoods that each
take their limit of 1,024.  The work of a round is its steps and 32 for each of its 64 neighborhoods, which cost time
beside their steps (`NEIGHBORHOOD_WORK` in `search::finder` has the measurements), so that work tracks time whatever
the options.  The budget is checked after each round, and the round that reaches it is the last.  The start alone is no
level, since it never repairs a shortfall, and `--effort` refuses 0 and anything above 6.

A level runs the rounds of every lower level first, so the `improvements` of a line at one level show what each lower
level would have scored, and each records the steps and the work of the rounds up to its own.

**What the levels gave.**  The notebook's [datasheet of the unsound
mode](https://github.com/bvchess/taxman-notebook/blob/main/search/the-datasheet-of-the-unsound-mode-2026-11/README.md)
played the known games 4561-5060 as five chains of 100 games at each level, each game starting from the run's own line
of the game before and the first from the built-in table's, with the default options and the seed 1.  Of the 500 games,
this share scored opt(n), and a chain of 100 games took this long on 4 threads of an idle machine (estimated from runs
on a busy one):

| level | budget (work) | games at opt(n) | hours per 100 games, 4 threads |
|---|---|---|---|
| 1 | 131,072 | 94.2% | 0.15 |
| 2 | 262,144 | 95.8% | 0.24 |
| 3 | 524,288 | 97.2% | 0.42 |
| 4 | 1,048,576 | 97.2% | 0.77 |
| 5 | 2,097,152 | 97.2% | 1.41 |
| 6 | 4,194,304 | 98.8% | 2.46 |

Each level took 1.6 to 1.8 times the time of the one below, since a game's fixed costs, such as its start, do not
double.

The table does not show:

* **More than one seed.**  Levels 3 to 5 leave the same 14 games short, and level 6's gain of 1.6 points is two stuck
  games and the games behind them, so a share has no interval.
* **A longer chain.**  A chain of 100 games starts from a proven game.  A shortfall passes to the games after it, so a
  chain of 200 games scored lower in the datasheet.
* **This build.**  The chains ran before the repair and the push on growth, which can raise the shares, and the stop on
  growth, which ends most games' rounds after level 1's, and before the rounds ran ahead of their ends, which gives
  the same lines in less time.
* **The games past 5060.**  The known games stand in for them, and their hardest games lie at the top of the range, so
  the share past 5060 may be lower.

The notebook's entries before this change number the levels one higher: their effort E is level E − 1 here, and their
lines' `budget` gives the work.

### The options that shape the rounds

A game follows one *trajectory* of incumbents, the one that begins from the start.  The options change what the rounds
search, and so the line, and every value of them keeps what the rounds guarantee above: a level runs the rounds of the
levels below it first.

**`--back J`** adds a trajectory for each distinct set that survives of the moves for the games n − 1, n − 2, ..., n − J:
their promotions that can still be promoted here, from the largest down, as the proven finder's seed rounds begin.  A
trajectory is added only if no trajectory before it, the start's included, began from the same set, and a game that
none of the sources has moves for adds none.  Neighborhood k of a round works from the incumbent of trajectory k mod T,
T the trajectories, as the round starts, and a trajectory's next incumbent is the best set of its own, the lowest k's
among equals, if that is heavier than before.  The game's set is the heaviest incumbent, the start's on a tie, and a
round that makes it heavier is an improvement, which names the trajectory that made it.  What survives of an earlier
game can be heavier than the start; the game's set is then that before any round, an improvement of round 0.  The line
says which games the trajectories began from (`trajectories`), and the score of what survives of each game's moves,
whether or not it began one (`survivor_scores`).  A game that is not seeded follows the start's trajectory alone.

**`--above J`** adds, after those of `--back`, a trajectory for each distinct set that survives of the moves for the
games n + 1, ..., n + J: the moves above n are left out, and what survives of the rest is taken as for `--back`.  The
moves are the run's lines, else FILE's, and with `--from-known` the built-in table's, which are optimal; without it the
table gives none.  A game played in order has no line above it in its own run, so the option is for measuring what the
games above could give a game; [the repair](shortfalls.md#the-repair) is what a run takes from them.

**`--tilt P`** makes P percent of a round's neighborhoods free the small promotions more often than the large ones, with
the same number freed as without it; the rest free them as a neighborhood does without the option.  Neighborhood k of a
round of T trajectories is the jth of its trajectory, j = k div T, and it is tilted if and only if ⌊(j + 1) P / 100⌋ >
⌊j P / 100⌋ (`draws_tilted` in `search::finder`).  So 0 tilts none, 100 every neighborhood, 50 the odd j and 25 every
fourth j (3, 7, 11, ...), and every trajectory has P percent of its neighborhoods tilted to within one, whatever T is.
A tilted neighborhood draws the promotions to free by weights that fall as x/n of a promotion x of game n rises, in
whole numbers so that a line is the same on every machine (`TILT` in `search::finder` has them).

**`--widen S`** gives a trajectory a *stage*, 0 to 2, which sets the range of the share its neighborhoods keep fixed:
0.7,0.9, then 0.5,0.7, then 0.4,0.6.  It starts at stage 0 and moves one stage down after S rounds in which its
incumbent did not grow heavier, counting from its first round and again after each move, stays at stage 2, and returns
to stage 0 when its incumbent grows heavier.  A trajectory's stage in a round depends only on the rounds before it.  The
line records the rounds each trajectory spent at each stage (`stage_rounds`) and the stage of each improvement.

### The run's ceiling

A searched game stops at the *run's ceiling*, n + score(n − 1) of the moves it starts from: its rounds end once its
score reaches it, before the first round if the start does, else after the round that does (which may pass it).  When
those moves are optimal, as the built-in table's are with `--from-known`, the ceiling is the [proven
one](../../../docs/GLOSSARY.md#the-game) and the stop loses nothing.  Otherwise a game could score above it, which would
prove the line of n − 1 short (`prev.short_by`); the stop gives that up for the time it saves.  A game with no moves for
n − 1 has no ceiling, and `--past-ceiling` takes the stop away.
