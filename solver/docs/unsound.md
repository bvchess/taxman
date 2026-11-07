# The unsound mode

The unsound mode plays each game of a range as well as a level of effort finds, and gives no proof that its score is the
best.  It is for a user who wants scores fast and accepts that none is proven.  A proof of a hard game takes hours,
and nearly all of it goes to proving a set that the search found early; this mode keeps the finding and drops the proof.
Below, `taxman` is `solver/frame/target/release/taxman`, run from the top of the repository ([the frame solver's
README](../frame/README.md) says how to build it), and the words of the search are in the
[glossary](../../docs/GLOSSARY.md#the-search).

* [What it is and is not](#what-it-is-and-is-not)
* [The command](#the-command)
* [The flags](#the-flags)
* [How a game is played](#how-a-game-is-played)
* [The repair](#the-repair)
* [An unsound line](#an-unsound-line)

## What it is and is not

Each line of a run is a real game.  Its moves are legal and are replayed before they count, so its score is a lower
bound on opt(n), the best score of game n.  The score is opt(n) if the search found the best game, and nothing in the
run says whether it did.  A higher level of effort buys a better chance, with more time, and never scores less than a
lower one, since it plays the rounds of the lower one first.  A score above the built-in table's, which is proven, is a
hard error.

An unsound line never mixes with proven ones.  Its key `"unsound":true` makes `solve` without `--unsound`,
`verify-results`, `solver/tools/check_results.py` and the built-in table refuse any line that has it, and `--unsound`
refuses a file that holds a results line.  So a file of unsound lines is a file of its own, and nothing takes it for a
proof.

## The command

```
$ taxman solve 5061-5063 --unsound --effort 2 --out unsound.jsonl
n=5061  score=8057912   search  rounds=5      improved=1      21.565 s  unsound
n=5062  score=8062974   repeat  rounds=0      improved=0       0.427 s  unsound  at ceiling
n=5063  score=8063376   repeat  rounds=0      improved=0       0.405 s  unsound
repaired 0 lines; the repair took 0.027 s
```

`solve A-B --unsound --effort E --out FILE` plays the games A to B in order, one at a time on all the threads, and
appends the line of each to FILE as its game ends, and each line that [the repair](#the-repair) replaces.  It needs
`--effort` and `--out`.  A game starts from the moves for n − 1 that the run, else FILE, else the built-in table has,
so the first game of a range past the table's end starts from the moves for the last game of the table.  A stop signal
(SIGINT or SIGTERM) ends the run between rounds, and the game in play is not recorded; the same command goes on from
the games FILE has, and from where the repair was.

The summary line on stdout gives n, the score, the method, the rounds, how many of them raised the score and the
seconds, then `unsound`, what `--stop-at-known` recorded, "at ceiling" if the game stopped at the run's ceiling, and
"n-1 short by" a number of points if this game's score proves the line of n − 1 short by at least that many.  A line
that the repair replaces has a summary line of its own: n, the new score, the method of the line replaced, the game its
moves came from and how much they gain, and the seconds.  The run ends with the number of lines the repair replaced and
the seconds its walks took, those that replaced nothing included.

## The flags

The unsound mode takes `--out`, `--threads`, `--tactic` and `--show` as `solve` does ([the
reference](REFERENCE.md#1-the-flags-of-solve)), and refuses the flags of the proof search: `--window`, the finder's,
`--profile`, `--profile-depth`, `--carried-dir`, `--gate`, the gate trial's, the checkpoints', `--progress`, the limits,
`--mem-cap`, `--background` and `--force-search`.  Its own flags, besides `--unsound` itself:

| flag | meaning |
|---|---|
| `--effort E` | the level of effort, from 0 (the search's start alone) to 32; needed ([the levels](#the-level-of-effort)) |
| `--seed S` | the seed of the rounds' random choices (default 1) |
| `--back J` | J from 0 to 8, default 8: besides the start's, a game follows a trajectory from each distinct set that survives of the moves for the games n − 1, ..., n − J.  0: the start's alone |
| `--above J` | J from 0 to 8, default 0: a game also follows a trajectory from each distinct set that survives of the moves for the games n + 1, ..., n + J that the run or FILE has, or with `--from-known` the built-in table.  A game played in order has no line above it in its own run, so this is for measuring |
| `--tilt P` | P from 0 to 100, default 25: P percent of a round's neighborhoods free the small promotions more often.  0: none |
| `--widen S` | S ≥ 1, default 4: a trajectory that has not grown heavier in S rounds moves to larger neighborhoods.  0: no widening |
| `--fix LO,HI` | the share of the incumbent's promotions that each neighborhood keeps fixed, drawn at random from LO to HI, 0 ≤ LO ≤ HI ≤ 1 (default 0.7,0.9).  It turns widening off, and is refused with `--widen S` of 1 or more |
| `--past-ceiling` | no stop at the [run's ceiling](#the-runs-ceiling) |
| `--no-repair` | no [repair](#the-repair) of the lines below a line |
| `--stop-at-known`, `--from-known` | to measure the mode on the games whose opt(n) the built-in table has, and refused for any other: end a game's rounds as soon as its score is the table's, and start every game from the table's moves for n − 1 |

The options `--back`, `--tilt` and `--widen` shape the rounds ([the options](#the-options-that-shape-the-rounds)), and
are on unless given 0, so that a user names a level of effort and nothing else; `--above` shapes them too, and is off
unless given.  The three were chosen together by
two races on the hard games of 3441-5060, at equal work ([the first
race](https://github.com/bvchess/taxman-notebook/blob/main/search/tilted-back-and-wider-rounds-2026-11/README.md), [the
second](https://github.com/bvchess/taxman-notebook/blob/main/search/a-share-of-tilted-neighborhoods-2026-11/README.md)).
At level 2 and up, `--tilt 25` and `--tilt 50` are within two games of each other, so 25 is chosen for its lead at the
low levels, not by a significant margin.  `--fix` replaces the ladder of the widening with a share of its own, which is
why it turns widening off.

## How a game is played

A game is set up as a proven run sets it up: the board and its frames.  Four methods follow, each named in the line:

* **`trivial`**: a board with no promotion candidates has the moves with no promotions.
* **`prime`**: a prime n plays the prime formula on the moves for n − 1.
* **`repeat`**: a game whose search problem a line of the run or of the file posed, as a proven run finds a repeat,
  plays that line's promotions on its own board.
* **`search`**: any other game takes the start of a proven search, seeded with the moves for n − 1, then the rounds of
  its level of effort.

A `prime` or `repeat` game searches nothing, and keeps what the line it starts from falls short by, if it does.  The
moves for n − 1 are the run's line, else the file's, else the built-in table's, and `--from-known` takes the table's
always.

The setup does not depend on the moves for n − 1, so a run makes each game's setup while the game before it plays, on
a thread of its own beside the threads of `--threads`, with the part of a searched game's start that depends on the
board alone (its dive).  Both run on one thread, and on many threads they would otherwise be a large part of the time
of a run at a low level of effort.

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
([finder.rs](../frame/src/search/finder.rs), "Ahead of the end of a round").

### The level of effort

Effort 0 is the start alone.  Effort E ≥ 1 runs rounds until their work reaches a budget of 2^(E−1) × 65,536, the
steps of a round of 64 neighborhoods that each take their limit of 1,024.  The work of a round is its steps and 32 for
each of its 64 neighborhoods, which cost time beside their steps (`NEIGHBORHOOD_WORK` in `search::finder` has the
measurements), so that work tracks time whatever the options.  The budget is checked after each round, and the round
that reaches it is the last.  The most effort is 32.

A level runs the rounds of every lower level first, so the `improvements` of a line at one level show what each lower
level would have scored, and each records the steps and the work of the rounds up to its own.

### The options that shape the rounds

A game follows one *trajectory* of incumbents, the one that begins from the start.  The options change what the rounds
search, and so the line, and every value of them keeps what the rounds guarantee: a line depends on the game, its
start, the moves it starts from, the level, the seed and the options only, and a level runs the rounds of the levels
below it first.

**`--back J`** adds a trajectory for each distinct set that survives of the moves for the games n − 1, n − 2, ..., n − J:
their promotions that can still be promoted here, from the largest down, as the proven finder's seed rounds begin.  A
trajectory is added only if no trajectory before it, the start's included, began from the same set, and a game that
none of the sources has moves for adds none.  Neighborhood k of a round works from the incumbent of trajectory k mod T,
T the trajectories, as the round starts, and a trajectory's next incumbent is the best set of its own, the lowest k's
among equals, if that is heavier than before.  The game's set is the heaviest incumbent, the start's on a tie, and a
round that makes it heavier is an improvement, which names the trajectory that made it.  What survives of an earlier
game can be heavier than the start; the game's set is then that before any round, an improvement of round 0, and at
effort 0 the game's score is the heaviest of the trajectories' starts.  The line says which games the trajectories
began from (`trajectories`), and the score of what survives of each game's moves, whether or not it began one
(`survivor_scores`).  A game that is not seeded follows the start's trajectory alone.

**`--above J`** adds, after those of `--back`, a trajectory for each distinct set that survives of the moves for the
games n + 1, ..., n + J: the moves above n are left out, and what survives of the rest is taken as for `--back`.  The
moves are the run's lines, else FILE's, and with `--from-known` the built-in table's, which are optimal; without it the
table gives none.  A game played in order has no line above it in its own run, so the option is for measuring what the
games above could give a game; [the repair](#the-repair) is what a run takes from them.

**`--tilt P`** makes P percent of a round's neighborhoods free the small promotions more often than the large ones, with
the same number freed as without it; the rest free them as a neighborhood does without the option.  Neighborhood k of a
round of T trajectories is the jth of its trajectory, j = k div T, and it is tilted if and only if ⌊(j + 1) P / 100⌋ >
⌊j P / 100⌋ (`draws_tilted` in `search::finder`).  So 0 tilts none, 100 every neighborhood, 50 the odd j and 25 every
fourth j (3, 7, 11, ...), and every trajectory has P percent of its neighborhoods tilted to within one, whatever T is.
A tilted neighborhood's draw of which promotions to free is a weighted draw without replacement, the weight of a
promotion x of game n set by x/n in ten bins of width 0.025 from 0.25, from 487 for the lowest down to 33 for the
highest (`TILT` in `search::finder`, which also draws in whole numbers, so that a line is the same on every machine).

**`--widen S`** gives a trajectory a *stage*, 0 to 2, which sets the range of the share its neighborhoods keep fixed:
0.7,0.9, then 0.5,0.7, then 0.4,0.6.  It starts at stage 0 and moves one stage down after S rounds in which its
incumbent did not grow heavier, counting from its first round and again after each move, stays at stage 2, and returns
to stage 0 when its incumbent grows heavier.  A trajectory's stage in a round depends only on the rounds before it.  The
line records the rounds each trajectory spent at each stage (`stage_rounds`) and the stage of each improvement.

### The run's ceiling

A searched game stops at the *run's ceiling*, n + score(n − 1) of the moves it starts from: its rounds end once its
score reaches it, before the first round if the start does, else after the round that does (which may pass it).  When
those moves are optimal, as the built-in table's are with `--from-known`, the ceiling is the [proven
one](../../docs/GLOSSARY.md#the-game) and the stop loses nothing.  Otherwise a game could score above it, which would
prove the line of n − 1 short (`prev.short_by`); the stop gives that up for the time it saves.  A game with no moves for
n − 1 has no ceiling, and `--past-ceiling` takes the stop away.

## The repair

A game that falls short hands its shortfall on: the games above it start from its line, a prime and a repeat copy it,
and a searched game stops at the lower ceiling it sets.  The repair takes the shortfall back down when a line above does
better.  It rests on a fact of the game: the moves of a game of N, less those above n, are a game of n that scores the
moves kept, since leaving a move out only leaves more numbers for the others ([the ceiling lemma
(N4)](../../docs/theory/neighbors.md#the-ceiling-lemma), once for each number above n).  So what survives in game n of
a line above it is a real game of n, and when it scores more than the line of n, that line is short and is replaced.

* **When.**  After each line the run records, it walks down the games below it, from the highest: for each game, what
  survives of the lines within 8 games above it that the walk has written, the line just recorded and those it has
  replaced, the heaviest, the nearest on a tie.  If that scores more than the game's line, it replaces it, and the walk
  goes on from it too.  The walk ends at the first game with none of its lines within 8 above.  What survives is taken
  as `--back` takes it, so the new line is a set of promotions and the moves they make, which a repeat can play.
* **Primes, trivial games and the table.**  A prime's line is not weighed: it follows from the line below it, and is
  played again from it once that is replaced.  A trivial line is optimal, and the built-in table's lines are proven, so
  neither is weighed.
* **What it changes above.**  Nothing the walk does not reach: the games above the line just recorded are not played
  yet, and those between it and a game repaired are weighed first.  A repeat of a game whose line is replaced plays its
  new promotions, and the games that follow begin their trajectories from the new lines.
* **The file.**  A replaced line is written again, appended: the line as its game was played, with the new moves,
  promotions and score, `at_ceiling` and `prev.short_by` of the new score, and `repaired`, which says what replaced it.
  The last line of a game is the game's.  A run that opens FILE goes on with the walk where its last lines leave it,
  from the last line that is no repair and the repairs after it, so a run that a stop cuts short ends with the lines of
  one that ran on.
* **The cost.**  The repair runs no rounds, so the work of a level is that of its rounds, as without it.  Its time is
  that of the survivors on the boards the run kept from playing the games, which is small beside a game's, and of the
  boards it sets up again, a few seconds each at n ≈ 5,000: the run keeps the boards of its last 8 games, so a walk
  that goes further needs them, and so does the walk that a run takes up from FILE.

A line depends on the repair as on any option (`repair` in the line), and `--no-repair` turns it off.  The rounds of a
game at a level begin with those of the levels below, from the same moves, as before; in a range the repair can change
the moves that later games start from, as a level does.

## An unsound line

An unsound line holds legal moves for n and their score, a lower bound on opt(n) that nothing proves.  Here is the line
of game 595 at effort 4 on four threads, with `--from-known` and `--stop-at-known`, with the moves and the promotions cut
short, the fields regrouped and comments added:

```
{"unsound":true, "n":595, "score":110860, "moves":[593,529,437,...], "promotions":[171,186,195,...],
 "method":"search", "in_the_bag":102413,
 "prev":{"n":594,"score":110282,"source":"table","moves_digest":"623c70271b00b128","short_by":0},
 "ceiling":110877, "at_ceiling":false,
 "repeats":null,                          # a repeat names the line whose promotions it plays here
 "setup":{"frames":6,"candidates":141,"max_total":33,"greedy_sum":8396,"digest":"c6fe37bd55f2b043"},
 "effort":4, "budget":524288, "seed":1, "fix":[0.7,0.9], "tilt":25, "back":8, "above":0, "widen":4,
 "start":110839,                          # the start fell 21 short of the table's score
 "trajectories":["start",594,593,591],    # the moves for 592 and for 590 to 587 gave a set one of these began from
 "survivor_scores":[[594,110682],[593,110489],[592,110489],[591,110303],...,[587,110303]],
 "rounds":1, "neighborhoods":64, "steps":699, "work":2747,
 "stage_rounds":[[1,0,0],[1,0,0],[1,0,0],[1,0,0]],
 "improvements":[{"round":1,"trajectory":1,"stage":0,"neighborhoods":64,"steps":699,"work":2747,"seconds":0.0433,
                  "score":110860}],
 "seconds":0.0454, "start_seconds":0.0158, "threads":4,
 "from_known":true, "stop_at_known":{"known":110860,"round":1},   # so effort 1 would have reached it
 "repair":true, "repaired":null,          # a line the repair wrote again says what replaced its moves
 "tactics":"default", "build":{"commit":"50db88e20150...","dirty":true}}
```

A line that the repair replaced is the line as its game was played, with new moves, written again.  Game 5042 of a run
of 5040-5045 at effort 4, which began from the lines of 4961-5039 of another run, played as a repeat 11 short of the
table's score, and what survived of the line of 5043, which reached it, replaced it; the summary lines on stdout were

```
n=5042  score=8000285   repeat  rounds=0      improved=0       0.250 s  unsound  at ceiling
n=5043  score=8002601   search  rounds=10     improved=1      61.890 s  unsound
n=5042  score=8000296   repeat  repaired from 5043, +11     0.082 s  unsound
```

and the second line of 5042 in the file has `"score":8000296`, the new moves and promotions, `"at_ceiling":true`
still, `prev.short_by` 11, since the new score proves the line of 5041 short by 11, and
`"repaired":{"from":5043,"was":8000285,"seconds":0.082}`.  The walk went on to replace the lines of 5041 and 5040 in
the same way, each from the line above it.

| field | meaning |
|---|---|
| `unsound` | true: the mark of an unsound line |
| `n`, `score`, `moves`, `promotions` | the game, its score, its moves, and the promoted numbers, ascending |
| `method` | `trivial`, `prime`, `repeat` or `search` |
| `in_the_bag` | what the game with no promotions scores (null for the prime formula, which sets no board up) |
| `prev` | the moves for n − 1 the game started from: `n`, `score`, `source` (`run`: this run's, `results`: the file's, or `table`: the built-in table's), `moves_digest`, and `short_by`, the points by which they fall short of opt(n − 1) at least, as this game's score proves (its score less n + score(n − 1), where that is above 0); null if there were none |
| `ceiling`, `at_ceiling` | the run's ceiling on the score (null without moves for n − 1, and with `--past-ceiling`), and whether the score reached it |
| `repeats` | for a repeat, the n of the line whose promotions it plays; null otherwise |
| `setup` | as in [a results line](REFERENCE.md#a-results-line---out-schema-5), without `cap`; null on a `trivial` or `prime` line |
| `effort`, `budget`, `seed` | the level of effort, the work its rounds run until (0 at effort 0), and the seed of the rounds' random choices |
| `fix`, `tilt` | the range of `--fix` (the lowest and the highest share of the incumbent's promotions a neighborhood keeps fixed; a game that widens takes the ladder's shares instead, and this is the default range), and `--tilt`'s P (0 without it) |
| `back`, `above`, `widen` | `--back`'s J (0 without it), `--above`'s J (0 without it), and `--widen`'s S (null without it) |
| `start` | the score of a searched game's start; null on any other line |
| `trajectories` | for a searched game, what each incumbent its rounds kept began from: `"start"`, then the games whose survivors it began from, such as `["start",4231,4224]`; null on any other line |
| `survivor_scores` | for a searched game, each game whose moves a trajectory could begin from, in the order of `trajectories`, with the score of what survives of its moves here, whether or not it began one, such as `[[594,110682],[593,110489]]`; null on any other line |
| `rounds`, `neighborhoods`, `steps`, `work` | the rounds done, their neighborhoods, the engine's steps in them, and their work (the steps and 32 for each neighborhood) |
| `stage_rounds` | for a searched game, for each trajectory, the rounds it spent at each of the three stages of `--widen` (all at stage 0 without it); null on any other line |
| `improvements` | for each round that raised the score: `round` (0: before any round, a trajectory that began heavier than the start), the `trajectory` (an index in `trajectories`) that made it and its `stage` in that round, the `neighborhoods`, the `steps` and the `work` of the rounds up to and including this one, the `seconds` until it ended, and the new `score` |
| `seconds`, `start_seconds` | the game's wall time, from the end of the game before and of the repair's walk after it (or the start of the run) to its line, and the part of it before the rounds; a game's setup is made while the game before plays, so it counts only in the first game of a run |
| `threads` | the threads it ran on |
| `from_known` | true if `--from-known` started the game from the built-in table's moves for n − 1 |
| `stop_at_known` | with `--stop-at-known`: `known`, the built-in table's score, and `round`, the round after which the game had that score (0: the start or the start of a trajectory, or a game that runs no round), or null if it never had; null without the flag |
| `repair` | true unless `--no-repair` |
| `repaired` | null on a line as its game was played; on a line that [the repair](#the-repair) wrote again, `from`, the game whose line gave the new moves (a game above, or for a prime the game below), `was`, the score of the line it replaced, and `seconds`, the time the repair took |
| `tactics`, `build` | as in a results line, `build` the one that wrote the line |
