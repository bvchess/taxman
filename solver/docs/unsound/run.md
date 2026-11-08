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
the [best-known file](best-known.md) has moves that score more than the run's or FILE's, it starts from those.  A stop
signal (SIGINT or SIGTERM) ends the run between rounds, and the game in play is not recorded; the same command goes on
from the games FILE has, and from where the repair was.

The summary line on stdout gives n, the score, the method, the rounds, how many of them raised the score and the
seconds, then `unsound`, "at ceiling" if the game stopped at the run's ceiling, "n-1 short by" a number of points if
this game's score proves the line of n − 1 short by at least that many, the [growth of its
gap](shortfalls.md#the-bound-and-the-gap), with "stopped" if the growth ended its rounds, and for a [game played
twice](shortfalls.md#games-played-twice) that the push played it, and what its unseeded play scored, or that it was
kept.  A line that the repair replaces has a summary line of its own: n, the new score, the method of the line
replaced, the game its moves came from and how much they gain, and the seconds; a repair of the best-known file's line
alone says "best-known file only".  The run ends with the number of lines the repair replaced and the seconds its walks
took, those that replaced nothing included, and the number of lines it added to the best-known file, and how many of
those repaired its lines of games the run has none of.

## The flags

The unsound mode takes `--out`, `--threads`, `--tactic` and `--show` as `solve` does ([the
reference](../REFERENCE.md#1-the-flags-of-solve)), and refuses the flags of the proof search: `--window`, the finder's,
`--profile`, `--profile-depth`, `--carried-dir`, `--gate`, the gate trial's, the checkpoints', `--progress`, the limits,
`--mem-cap`, `--background` and `--force-search`.  Its own flags, besides `--unsound` itself:

| flag | meaning |
|---|---|
| `--effort L` | the level of effort, from 1 to 6; needed ([the levels](play.md#the-level-of-effort)) |
| `--seed S` | the seed of the rounds' random choices (default 1) |
| `--above J` | J from 0 to 8, default 0: besides the games before it, a game also follows a trajectory from each distinct set that survives of the moves for the games n + 1, ..., n + J that the run, FILE or the best-known file has.  A game played in order has no line above it in its own run, so this is for a run over a range whose files hold lines above it |
| `--stop-growth T` | T ≥ 0 or `off`, default 25: a searched game's rounds end once its [gap](shortfalls.md#the-bound-and-the-gap) is within T of the gap below and its last 4 rounds raised nothing, from the end of the rounds of level 1 on |
| `--best-known FILE` | FILE or `off`, default `best-known.jsonl` in the directory of `--out`: the [best-known file](best-known.md) that the run starts its games from, adds its lines to, and repairs lines in.  `off`: none |
| `--known-to N` | to rehearse the mode on games whose opt(n) the built-in table has: the run takes the table as holding the games up to N only, and a game above N is as if unproven.  No game starts from the table's moves for a game above N, the [best-known file](best-known.md#a-rehearsal-on-proven-games) may hold the games above N, and the checks against the table apply up to N.  The line records N |

Everything else is built in, so that a user names a level of effort, a seed and a few files: the shape of the rounds, the
[push](shortfalls.md#games-played-twice) of a game whose gap grows, the [repair](shortfalls.md#the-repair) of the lines
below a line, and the stop at the [run's ceiling](play.md#the-runs-ceiling).  [The options that shape the
rounds](play.md#the-options-that-shape-the-rounds) says what the settings are and which races chose them.
