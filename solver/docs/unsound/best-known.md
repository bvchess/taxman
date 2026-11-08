# The best-known file

Part of the [unsound mode](../unsound.md).

## What the best-known file is

The best-known file holds the best-known line of each game past the built-in table: the best line any run has found, at
any level of effort.  A run of the unsound mode starts its games from it and adds what it finds to it, so every run may
raise the best-known line of some n, and the file is where to look for the best-known line of n.  Its lines are
unproven: each is a lower bound on opt(n) that a run found, and every reader of proven lines refuses the file, as it
refuses any file of unsound lines ([what the mode is and is not](../unsound.md#what-it-is-and-is-not)).  A game of the
built-in table is never in it, since the table's proven line is that game's best-known line, except in a
[rehearsal](#a-rehearsal-on-proven-games).

`--best-known FILE` names the file.  By default it is `best-known.jsonl` in the directory of `--out`, and
`--best-known off` gives a run none.  The file is created if it is missing.

## How a run uses it

* **It starts from the best-known lines.**  Where a game takes the moves for the games below it (and above it, with
  `--above`), it takes, of the run's line (its own, else `--out`'s) and the best-known file's, the one that scores more,
  the run's on a tie, and then the built-in table's.  A line says so: `prev.source` is `best_known`.  Before each game
  the run reads what the other runs appended since it last read the file.
* **It adds every line it records.**  A line of a game, or a repair of one, is appended to the best-known file if it
  scores more than the best-known file's line of its game.
* **It offers its file when it opens it.**  Before its first game, a run offers the best-known file the last line of
  each game of `--out`.  So an old file is folded in by a run over its range with a copy of it as `--out`: every game is
  in the copy, so the run plays nothing, and it offers every line.
* **It repairs the best-known file's lines below its range.**  The walk of [the repair](shortfalls.md#the-repair) goes
  down from each line the run records.  Where it reaches a game below the run's first, which the run has no line of, it
  weighs the best-known file's line of that game against the run's lines that are within the repair's reach above it,
  and appends a repair of it to the best-known file alone.  Each board it sets up for this takes a few seconds at
  n ≈ 6,000.

A run to polish the lines of a range, for instance, searches the games at a high level, from the best-known lines:

```
$ taxman solve 6573-6590 --unsound --effort 6 --stop-growth off --out polish.jsonl --best-known best-known.jsonl
```

and the best-known file keeps each line that beats the one it had.

## The file

A best-known file is a file of unsound lines with one more key on each, `best_known`: `file`, the name of the run's
`--out` file that the line came from, and `was`, the score of the game's line in the best-known file before it, or null
for the first.  The lines of a game score more, line after line, and the last is the game's.  The first line of a game
may record a repair (`repaired`), since the line it replaced is in the file of the run that made it.  A run reads the
file with every check of a run's own file (the moves replay to the score, and so on), and checks that no line is of a
game of the built-in table or scores above its `bound`, and that the lines of a game are in that order.

Several runs may use one file at once.  A run locks it only to read what is new and to append, never for its life, so
none waits for another's game.  A run that stops in the middle of a line leaves a torn last line, which the next run to
use the file cuts off.

## A rehearsal on proven games

A best-known file made with `--known-to N` is for runs with the same N: a rehearsal on games whose opt(n) is proven.  The
file never holds a game of the built-in table, so a pass cannot ratchet on such games; `--known-to N` cuts the table at
N for the run ([the flag](run.md#the-flags)), the file may hold the games above N, and the run's reader refuses only the
games up to N.  A run without the flag, or with a larger N, refuses the file, since it holds games that run takes as
proven.  An old file of the games above N is folded in as any old file is, by a run over their range with a copy of it
as `--out`.

## What a line then depends on

A line depends on the moves it started from, and with a shared best-known file those depend on what the other runs had
found when its game began.  The line's `prev` says what it started from, the source and the digest of the moves, so that
the game can be played again from the same start.  An experiment that compares settings must run with
`--best-known off`, or with a best-known file of its own for each arm, so that the arms do not feed each other.
