# Running the unsound mode

Part of the [unsound mode](../unsound.md).

## The command

```
$ taxman solve 5061-5063 --unsound --effort 1 --threads 8 --out unsound.jsonl
n=5061  score=8057912   search  rounds=5      improved=1      29.567 s  unsound  growth 516  pushed, unseeded play 8057844
n=5062  score=8062974   repeat  rounds=0      improved=0       0.092 s  unsound  at ceiling  growth 0
n=5063  score=8063376   repeat  rounds=0      improved=0       0.374 s  unsound  growth 0
repaired 0 lines; the repair took 0.021 s
```

`solve A-B --unsound --effort L --out FILE` plays the games A to B in order, one at a time on all the threads, and
appends the line of each to FILE as its game ends, and each line that [the repair](shortfalls.md#the-repair) replaces.
It needs `--effort` and `--out`.  A game starts from the moves for n − 1 that the run, else FILE, else the built-in
table has, so the first game of a range past the table's end starts from the moves for the last game of the table; if
the [best file](best.md) has moves that score more than the run's or FILE's, it starts from those.  A stop signal
(SIGINT or SIGTERM) ends the run between rounds, and the game in play is not recorded; the same command goes on from
the games FILE has, and from where the repair was.

The summary line on stdout gives n, the score, the method, the rounds, how many of them raised the score and the
seconds, then `unsound`, what `--stop-at-known` recorded, "at ceiling" if the game stopped at the run's ceiling,
"n-1 short by" a number of points if this game's score proves the line of n − 1 short by at least that many, the
[growth of its gap](shortfalls.md#the-bound-and-the-gap), with "stopped" if the growth ended its rounds, and for a [game
played twice](shortfalls.md#games-played-twice) whether the push played it, and what its unseeded play scored, or that
it was kept.  A line that the repair replaces has a summary line of its own: n, the new score, the method of the line
replaced, the game its moves came from and how much they gain, and the seconds; a repair of the best file's line alone
says "best file only".  The run ends with the number of lines the repair replaced and the seconds its walks took, those
that replaced nothing included, and the number of lines it added to the best file, and how many of those repaired its
lines of games the run has none of.

## The flags

The unsound mode takes `--out`, `--threads`, `--tactic` and `--show` as `solve` does ([the
reference](../REFERENCE.md#1-the-flags-of-solve)), and refuses the flags of the proof search: `--window`, the finder's,
`--profile`, `--profile-depth`, `--carried-dir`, `--gate`, the gate trial's, the checkpoints', `--progress`, the limits,
`--mem-cap`, `--background` and `--force-search`.  Its own flags, besides `--unsound` itself:

| flag | meaning |
|---|---|
| `--effort L` | the level of effort, from 1 to 6; needed ([the levels](play.md#the-level-of-effort)) |
| `--seed S` | the seed of the rounds' random choices (default 1) |
| `--back J` | J from 0 to 8, default 8: besides the start's, a game follows a trajectory from each distinct set that survives of the moves for the games n − 1, ..., n − J.  0: the start's alone |
| `--above J` | J from 0 to 8, default 0: a game also follows a trajectory from each distinct set that survives of the moves for the games n + 1, ..., n + J that the run, FILE or the best file has, or with `--from-known` the built-in table.  A game played in order has no line above it in its own run, so this is for measuring |
| `--tilt P` | P from 0 to 100, default 25: P percent of a round's neighborhoods free the small promotions more often.  0: none |
| `--widen S` | S ≥ 1, default 4: a trajectory that has not grown heavier in S rounds moves to larger neighborhoods.  0: no widening |
| `--fix LO,HI` | the share of the incumbent's promotions that each neighborhood keeps fixed, drawn at random from LO to HI, 0 ≤ LO ≤ HI ≤ 1 (default 0.7,0.9).  It turns widening off, and is refused with `--widen S` of 1 or more |
| `--unseeded-every K` | K ≥ 0, default 0: a game whose n is a multiple of K is played twice, also unseeded, and keeps the better line ([games played twice](shortfalls.md#games-played-twice)).  0: none |
| `--stop-growth T` | T ≥ 0 or `off`, default 25: a searched game's rounds end once its [gap](shortfalls.md#the-bound-and-the-gap) is within T of the gap below, from the end of the rounds of level 1 on |
| `--stop-quiet K` | K ≥ 0, default 4: the stop on growth also needs K [quiet rounds](shortfalls.md#the-bound-and-the-gap).  0: the gap alone |
| `--push-growth T` | T or `off`, default 25: a game whose gap grows by more than T in its play is [played twice](shortfalls.md#games-played-twice) |
| `--unseeded-effort L` | the level of effort of the unseeded play of a game played twice, from 1 to 6 (default: `--effort`'s), for measuring |
| `--best FILE` | FILE or `off`, default `best.jsonl` in the directory of `--out`: the [best file](best.md) that the run starts its games from, adds its lines to, and repairs lines in.  `off`: none.  The measuring options `--stop-at-known` and `--from-known` have none by default |
| `--past-ceiling` | no stop at the [run's ceiling](play.md#the-runs-ceiling) |
| `--no-repair` | no [repair](shortfalls.md#the-repair) of the lines below a line |
| `--stop-at-known`, `--from-known` | to measure the mode on the games whose opt(n) the built-in table has, and refused for any other: end a game's rounds as soon as its score is the table's, and start every game from the table's moves for n − 1 |
| `--unseeded` | to measure what a game makes of its board alone: every game is played [unseeded](shortfalls.md#games-played-twice), and none twice |

The options `--back`, `--tilt` and `--widen` shape the rounds ([the options](play.md#the-options-that-shape-the-rounds)),
and are on unless given 0, as `--stop-growth` and `--push-growth` are unless given `off`, so that a user names a level
of effort and nothing else; `--above` shapes them too, and is off unless given.  The three were chosen together by
two races on the hard games of 3441-5060, at equal work ([the first
race](https://github.com/bvchess/taxman-notebook/blob/main/search/tilted-back-and-wider-rounds-2026-11/README.md), [the
second](https://github.com/bvchess/taxman-notebook/blob/main/search/a-share-of-tilted-neighborhoods-2026-11/README.md)).
At every level, `--tilt 25` and `--tilt 50` are within two games of each other, so 25 is chosen for its lead at the
low levels, not by a significant margin.
