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
* [Games played twice](#games-played-twice)
* [The bound and the gap](#the-bound-and-the-gap)
* [An unsound line](#an-unsound-line)

## What it is and is not

Each line of a run is a real game.  Its moves are legal and are replayed before they count, so its score is a lower
bound on opt(n), the best score of game n.  The score is opt(n) if the search found the best game, and nothing in the
run says whether it did.  A higher level of effort buys a better chance, with more time, and its seeded play never
scores less than a lower one's, since it plays the rounds of the lower one first.  A score above the built-in table's,
which is proven, or above the game's [bound](#the-bound-and-the-gap) is a hard error.

An unsound line never mixes with proven ones.  Its key `"unsound":true` makes `solve` without `--unsound`,
`verify-results`, `solver/tools/check_results.py` and the built-in table refuse any line that has it, and `--unsound`
refuses a file that holds a results line.  So a file of unsound lines is a file of its own, and nothing takes it for a
proof.

## The command

```
$ taxman solve 5061-5063 --unsound --effort 1 --threads 8 --out unsound.jsonl
n=5061  score=8057912   search  rounds=5      improved=1      29.567 s  unsound  growth 516  pushed, unseeded play 8057844
n=5062  score=8062974   repeat  rounds=0      improved=0       0.092 s  unsound  at ceiling  growth 0
n=5063  score=8063376   repeat  rounds=0      improved=0       0.374 s  unsound  growth 0
repaired 0 lines; the repair took 0.021 s
```

`solve A-B --unsound --effort L --out FILE` plays the games A to B in order, one at a time on all the threads, and
appends the line of each to FILE as its game ends, and each line that [the repair](#the-repair) replaces.  It needs
`--effort` and `--out`.  A game starts from the moves for n − 1 that the run, else FILE, else the built-in table has,
so the first game of a range past the table's end starts from the moves for the last game of the table.  A stop signal
(SIGINT or SIGTERM) ends the run between rounds, and the game in play is not recorded; the same command goes on from
the games FILE has, and from where the repair was.

The summary line on stdout gives n, the score, the method, the rounds, how many of them raised the score and the
seconds, then `unsound`, what `--stop-at-known` recorded, "at ceiling" if the game stopped at the run's ceiling,
"n-1 short by" a number of points if this game's score proves the line of n − 1 short by at least that many, the
[growth of its gap](#the-bound-and-the-gap), with "stopped" if the growth ended its rounds, and for a [game played
twice](#games-played-twice) whether the push played it, and what its unseeded play scored, or that it was kept.  A
line that the repair replaces has a summary line of its own: n, the new score, the method of the line replaced, the
game its moves came from and how much they gain, and the seconds.  The run ends with the number of lines the repair
replaced and the seconds its walks took, those that replaced nothing included.

## The flags

The unsound mode takes `--out`, `--threads`, `--tactic` and `--show` as `solve` does ([the
reference](REFERENCE.md#1-the-flags-of-solve)), and refuses the flags of the proof search: `--window`, the finder's,
`--profile`, `--profile-depth`, `--carried-dir`, `--gate`, the gate trial's, the checkpoints', `--progress`, the limits,
`--mem-cap`, `--background` and `--force-search`.  Its own flags, besides `--unsound` itself:

| flag | meaning |
|---|---|
| `--effort L` | the level of effort, from 1 to 6; needed ([the levels](#the-level-of-effort)) |
| `--seed S` | the seed of the rounds' random choices (default 1) |
| `--back J` | J from 0 to 8, default 8: besides the start's, a game follows a trajectory from each distinct set that survives of the moves for the games n − 1, ..., n − J.  0: the start's alone |
| `--above J` | J from 0 to 8, default 0: a game also follows a trajectory from each distinct set that survives of the moves for the games n + 1, ..., n + J that the run or FILE has, or with `--from-known` the built-in table.  A game played in order has no line above it in its own run, so this is for measuring |
| `--tilt P` | P from 0 to 100, default 25: P percent of a round's neighborhoods free the small promotions more often.  0: none |
| `--widen S` | S ≥ 1, default 4: a trajectory that has not grown heavier in S rounds moves to larger neighborhoods.  0: no widening |
| `--fix LO,HI` | the share of the incumbent's promotions that each neighborhood keeps fixed, drawn at random from LO to HI, 0 ≤ LO ≤ HI ≤ 1 (default 0.7,0.9).  It turns widening off, and is refused with `--widen S` of 1 or more |
| `--unseeded-every K` | K ≥ 0, default 0: a game whose n is a multiple of K is played twice, also unseeded, and keeps the better line ([games played twice](#games-played-twice)).  0: none |
| `--stop-growth T` | T ≥ 0 or `off`, default 25: a searched game's rounds end once its [gap](#the-bound-and-the-gap) is within T of the gap below, from the end of the rounds of level 1 on |
| `--stop-quiet K` | K ≥ 0, default 4: the stop on growth also needs K [quiet rounds](#the-bound-and-the-gap).  0: the gap alone |
| `--push-growth T` | T or `off`, default 25: a game whose gap grows by more than T in its play is [played twice](#games-played-twice) |
| `--unseeded-effort L` | the level of effort of the unseeded play of a game played twice, from 1 to 6 (default: `--effort`'s), for measuring |
| `--past-ceiling` | no stop at the [run's ceiling](#the-runs-ceiling) |
| `--no-repair` | no [repair](#the-repair) of the lines below a line |
| `--stop-at-known`, `--from-known` | to measure the mode on the games whose opt(n) the built-in table has, and refused for any other: end a game's rounds as soon as its score is the table's, and start every game from the table's moves for n − 1 |
| `--unseeded` | to measure what a game makes of its board alone: every game is played [unseeded](#games-played-twice), and none twice |

The options `--back`, `--tilt` and `--widen` shape the rounds ([the options](#the-options-that-shape-the-rounds)), and
are on unless given 0, as `--stop-growth` and `--push-growth` are unless given `off`, so that a user names a level of
effort and nothing else; `--above` shapes them too, and is off unless given.  The three were chosen together by
two races on the hard games of 3441-5060, at equal work ([the first
race](https://github.com/bvchess/taxman-notebook/blob/main/search/tilted-back-and-wider-rounds-2026-11/README.md), [the
second](https://github.com/bvchess/taxman-notebook/blob/main/search/a-share-of-tilted-neighborhoods-2026-11/README.md)).
At every level, `--tilt 25` and `--tilt 50` are within two games of each other, so 25 is chosen for its lead at the
low levels, not by a significant margin.

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
always.  A [game played twice](#games-played-twice) is also searched with nothing from the games below it.

The setup does not depend on the moves for n − 1, so a run makes each game's setup, with the dive of a searched game's
start and the game's [bound](#the-bound-and-the-gap), on a thread of its own while the game before it plays.  On many
threads they would otherwise be a large part of the time of a run at a low level of effort.

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
games above could give a game; [the repair](#the-repair) is what a run takes from them.

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

A line depends on the repair as on any option (`repair` in the line), and `--no-repair` turns it off.  In a range the
repair can change the moves that later games start from, as a level does.

## Games played twice

A game played twice is played as any game is and again *unseeded*, and its line is the better of the two plays.  It is
how a run gets past a short line that the repair cannot pull up.  The repair replaces a short line only from a line
above it that does better, and in a run the lines above a short game are made from it: a prime and a repeat copy it,
and a searched game starts from it and stops at the ceiling it sets.  An unseeded play takes nothing from the games
below, so the short line does not hold it down; once it does better, the games above start from it and the repair walks
down from it.

* **Which games.**  With `--push-growth T`, on by default, a game whose [gap grows](#the-bound-and-the-gap) by more
  than T in its play, and with `--unseeded-every K` a game whose n is a multiple of K (K = 8: games 4952, 4960 and so
  on), unless it is trivial.  Which games depends on n and on the moves for n − 1 alone.
* **The unseeded play.**  It is searched on the game's board, never played by the prime formula or as a repeat: its
  start has no seed, its rounds follow the start's trajectory alone (and those of `--above`, if given), and it has no
  run's ceiling and no stop on growth.  A prime, which has no board, takes the one set up for its bound.  `--unseeded`
  plays every game this way and none twice, to measure what a game makes of its board alone.
* **The line.**  The better play's, the seeded one on a tie, as that play alone writes it, with `other`, what the other
  play came to.  A line of the unseeded play has no `prev` and no ceiling.  Its `seconds` are those of both plays.
* **The work.**  The unseeded play runs its rounds to the level's budget (or `--unseeded-effort`'s), as a searched game
  does that is not stopped at a ceiling, so a game played twice does up to twice a level's work: each play counts
  against a budget of its own, whether the game is played twice for its n or by the push.  Each play of a level runs
  the rounds of the levels below first.
* **What it gains.**  A streak of short games ends within K games of where it starts, if the unseeded play there
  reaches a better score: in the chain of 4861-5060 at level 3 (524,288 work), K = 8 took the games at opt(n) from 182
  to 198 of 200, since the unseeded play of 4952 ended the streak that the stuck 4950 starts after 2 games, not 18, for
  24% more time ([the
  measurement](https://github.com/bvchess/taxman-notebook/blob/main/search/unseeded-games-above-a-short-game-2026-11/README.md)).
  The push plays 4950 itself twice, and with it K = 8 changed no score of 4861-5060 or 5061-5260 for half again the
  time, so the default is 0 ([the bound and the gap](#the-bound-and-the-gap)).  An unseeded play alone is weaker than
  a seeded one, and reaches opt(n) less often; it pays only where the seeded play is held down.

## The bound and the gap

The stop and the push compare how far a game falls below a bound that no play exceeds with how far the game below it
fell: they end the rounds of a game that is probably done, and play twice a game that is probably short.

* **The bound.**  A game's bound is `in_the_bag` plus the GF(2) bound of the root's node, the parity bound of n
  ([frame-bounds.md](../../docs/theory/frame-bounds.md#the-frame-bound), the paragraph "Against the parity bound"); a
  score above it is a hard error.  It costs about 0.02 s a game at n = 5,000 and 0.05 s at 8,000, and a prime, which
  the prime formula plays with no board, 0.2 and 0.5 s more to set one up, on the thread that makes the next game's
  setup.
* **The gap and its growth.**  A line's gap is its bound less its score, and its growth the gap less the gap of the
  moves for n − 1 it starts from.  On the proven games of 3001-5060 the gap does not grow in 81% of the games, and
  grows by more than 50 in 5.6% and by more than 200 in 1.0%.  A short game makes it jump where a streak of short games
  begins (by 376 at the stuck 4950), and the games that it hands its shortfall on to carry the gap on, with a growth of
  about 0.
* **`--stop-growth T`** ends a searched game's rounds once its gap is within T of the gap below, asked after each
  round from the one that reaches the budget of level 1 on.  Most games reach their score in level 1's rounds; a gap
  that shrank by more than T says that the game below may be short or that this one has more to give, so the rounds go
  on.  Level 1 plays as without the stop, and a level still plays the rounds of the levels below first.
* **`--stop-quiet K`** makes the stop wait for K *quiet rounds*, K rounds in a row that did not raise the game's
  score, since a game that gains late may gain again.  The start counts as round 0, and K = 0 asks the gap alone.
* **`--push-growth T`** plays a game twice, as `--unseeded-every` does, when its gap has grown by more than T in its
  play: the unseeded play is the one that a short line below cannot hold down.  It counts against a budget of its own,
  the level's.  A pushed game at one level may score more than the same game at a higher level, whose seeded play
  brought the growth to T or below and was not pushed.
* **What they gave.**  At level 3, with a push of 50, the gap alone for the stop and no game played twice for its n,
  they took the chain of 4861-5060 from 198 of 200 games at opt(n) to 200 in 44% less time: the push played the stuck
  4950 twice, and its streak never began.  On 5061-5260 no line changed, in 43% less time ([the
  measurement](https://github.com/bvchess/taxman-notebook/blob/main/search/stopping-and-pushing-by-the-gap-2026-11/README.md)).
* **Where the defaults come from.**  A push of 25 took level 2 from 1,015 games at opt(n) to 1,039 on 4001-5060, for 4%
  more time ([the
  measurement](https://github.com/bvchess/taxman-notebook/blob/main/search/tuning-the-unsound-mode-on-a-long-chain-2026-11/README.md)).
  On lines made with no stop, 4 quiet rounds would have lost 7 of 2,060 games of level 2 where the gap alone loses 12,
  and saved 28.5% of the work against 28.9% ([the
  simulation](https://github.com/bvchess/taxman-notebook/blob/main/search/simulating-stop-rules-on-lines-already-made-2026-11/README.md)).
  The mode itself has not been measured with `--stop-quiet 4`.

## An unsound line

An unsound line holds legal moves for n and their score, a lower bound on opt(n) that nothing proves.  Here is the line
of game 595 at level 3 on four threads, with `--from-known` and `--stop-at-known`, with the moves and the promotions cut
short, the fields regrouped and comments added:

```
{"unsound":true, "n":595, "score":110860, "moves":[593,529,437,...], "promotions":[171,186,195,...],
 "method":"search", "in_the_bag":102413,
 "prev":{"n":594,"score":110282,"source":"table","moves_digest":"623c70271b00b128","short_by":0},
 "ceiling":110877, "at_ceiling":false,
 "bound":110966, "growth":4,             # its gap, 106, is 4 more than the table's 594's
 "growth_stop":false, "quiet_rounds":null, "pushed":false,
 "repeats":null,                          # a repeat names the line whose promotions it plays here
 "setup":{"frames":6,"candidates":141,"max_total":33,"greedy_sum":8396,"digest":"c6fe37bd55f2b043"},
 "effort":3, "budget":524288, "seed":1, "fix":[0.7,0.9], "tilt":25, "back":8, "above":0, "widen":4,
 "unseeded_every":0, "unseeded_effort":null, "stop_growth":25, "stop_quiet":4, "push_growth":25, "unseeded":false,
 "start":110839,                          # the start fell 21 short of the table's score
 "trajectories":["start",594,593,591],    # the moves for 592 and for 590 to 587 gave a set one of these began from
 "survivor_scores":[[594,110682],[593,110489],[592,110489],[591,110303],...,[587,110303]],
 "rounds":1, "neighborhoods":64, "steps":699, "work":2747,
 "stage_rounds":[[1,0,0],[1,0,0],[1,0,0],[1,0,0]],
 "improvements":[{"round":1,"trajectory":1,"stage":0,"neighborhoods":64,"steps":699,"work":2747,"seconds":0.0433,
                  "score":110860}],
 "other":null,                            # a game played twice says what its other play came to
 "seconds":0.0454, "start_seconds":0.0158, "threads":4,
 "from_known":true, "stop_at_known":{"known":110860,"round":1},   # so level 1 would have reached it
 "repair":true, "repaired":null,          # a line the repair wrote again says what replaced its moves
 "tactics":"default", "build":{"commit":"50db88e20150...","dirty":true}}
```

A line that the repair replaced is the line as its game was played, with new moves, written again.  Game 5042 of a run
of 5040-5045 at level 3 played as a repeat 11 short of the table's score, and what survived of the line of 5043, which
reached it, replaced it; the summary lines on stdout were

```
n=5042  score=8000285   repeat  rounds=0      improved=0       0.250 s  unsound  at ceiling
n=5043  score=8002601   search  rounds=10     improved=1      61.890 s  unsound
n=5042  score=8000296   repeat  repaired from 5043, +11     0.082 s  unsound
```

and the second line of 5042 in the file has `"score":8000296`, the new moves and promotions, `"at_ceiling":true`
still, `prev.short_by` 11, since the new score proves the line of 5041 short by 11, and
`"repaired":{"from":5043,"was":8000285,"seconds":0.082}`.  The walk went on to replace the lines of 5041 and 5040.

| field | meaning |
|---|---|
| `unsound` | true: the mark of an unsound line |
| `n`, `score`, `moves`, `promotions` | the game, its score, its moves, and the promoted numbers, ascending |
| `method` | `trivial`, `prime`, `repeat` or `search` |
| `in_the_bag` | what the game with no promotions scores (null for the prime formula, which sets no board up) |
| `prev` | the moves for n − 1 the game started from: `n`, `score`, `source` (`run`: this run's, `results`: the file's, or `table`: the built-in table's), `moves_digest`, and `short_by`, the points by which they fall short of opt(n − 1) at least, as this game's score proves (its score less n + score(n − 1), where that is above 0); null if there were none, and on a line of an unseeded play |
| `ceiling`, `at_ceiling` | the run's ceiling on the score (null without moves for n − 1, with `--past-ceiling`, and on a line of an unseeded play), and whether the score reached it |
| `bound`, `growth` | the game's [bound](#the-bound-and-the-gap), and the growth of its gap over the moves for n − 1 it started from, which a line of an unseeded play takes from the run's line below (null without such moves, and with `--unseeded`) |
| `growth_stop`, `pushed` | whether `--stop-growth` ended the rounds before the budget, and whether `--push-growth` played the game twice |
| `quiet_rounds` | where `--stop-growth` ended the rounds, their quiet rounds; null otherwise |
| `repeats` | for a repeat, the n of the line whose promotions it plays; null otherwise |
| `setup` | as in [a results line](REFERENCE.md#a-results-line---out-schema-5), without `cap`; null on a `trivial` or `prime` line |
| `effort`, `budget`, `seed` | the level of effort, the work its rounds run until, and the seed of the rounds' random choices |
| `fix`, `tilt` | the range of `--fix` (the lowest and the highest share of the incumbent's promotions a neighborhood keeps fixed; a game that widens takes the ladder's shares instead, and this is the default range), and `--tilt`'s P (0 without it) |
| `back`, `above`, `widen` | `--back`'s J (0 without it), `--above`'s J (0 without it), and `--widen`'s S (null without it) |
| `unseeded_every`, `unseeded` | `--unseeded-every`'s K (0 without it), and whether the line is of an [unseeded play](#games-played-twice) (as `--unseeded` plays every game) |
| `unseeded_effort`, `stop_growth`, `stop_quiet`, `push_growth` | the level of `--unseeded-effort`, the T of `--stop-growth` and of `--push-growth`, and the K of `--stop-quiet` (0 for none); null for none |
| `start` | the score of a searched game's start; null on any other line |
| `trajectories` | for a searched game, what each incumbent its rounds kept began from: `"start"`, then the games whose survivors it began from, such as `["start",4231,4224]`; null on any other line |
| `survivor_scores` | for a searched game, each game whose moves a trajectory could begin from, in the order of `trajectories`, with the score of what survives of its moves here, whether or not it began one, such as `[[594,110682],[593,110489]]`; null on any other line |
| `rounds`, `neighborhoods`, `steps`, `work` | the rounds done, their neighborhoods, the engine's steps in them, and their work (the steps and 32 for each neighborhood) |
| `stage_rounds` | for a searched game, for each trajectory, the rounds it spent at each of the three stages of `--widen` (all at stage 0 without it); null on any other line |
| `improvements` | for each round that raised the score: `round` (0: before any round, a trajectory that began heavier than the start), the `trajectory` (an index in `trajectories`) that made it and its `stage` in that round, the `neighborhoods`, the `steps` and the `work` of the rounds up to and including this one, the `seconds` until it ended, and the new `score` |
| `other` | for a [game played twice](#games-played-twice), the play this line is not: `unseeded`, whether it was the unseeded play, and its `method`, `score`, `rounds`, `steps`, `work` and `seconds`; null on any other line |
| `seconds`, `start_seconds` | the game's wall time, from the end of the game before and of the repair's walk after it (or the start of the run) to its line (both plays of a game played twice), and the part of its play before the rounds; a game's setup is made while the game before plays, so it counts only in the first game of a run |
| `threads` | the threads it ran on |
| `from_known` | true if `--from-known` started the game from the built-in table's moves for n − 1 |
| `stop_at_known` | with `--stop-at-known`: `known`, the built-in table's score, and `round`, the round after which the game had that score (0: the start or the start of a trajectory, or a game that runs no round), or null if it never had; null without the flag |
| `repair` | true unless `--no-repair` |
| `repaired` | null on a line as its game was played; on a line that [the repair](#the-repair) wrote again, `from`, the game whose line gave the new moves (a game above, or for a prime the game below), `was`, the score of the line it replaced, and `seconds`, the time the repair took |
| `tactics`, `build` | as in a results line, `build` the one that wrote the line |
