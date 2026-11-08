# Reading an unsound line

Part of the [unsound mode](../unsound.md).

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
| `prev` | the moves for n − 1 the game started from: `n`, `score`, `source` (`run`: this run's, `results`: the file's, `best_known`: the [best-known file's](best-known.md), or `table`: the built-in table's), `moves_digest`, and `short_by`, the points by which they fall short of opt(n − 1) at least, as this game's score proves (its score less n + score(n − 1), where that is above 0); null if there were none, and on a line of an unseeded play |
| `ceiling`, `at_ceiling` | the run's ceiling on the score (null without moves for n − 1, with `--past-ceiling`, and on a line of an unseeded play), and whether the score reached it |
| `bound`, `growth` | the game's [bound](shortfalls.md#the-bound-and-the-gap), and the growth of its gap over the moves for n − 1 it started from, which a line of an unseeded play takes from the run's line below (null without such moves, and with `--unseeded`) |
| `growth_stop`, `pushed` | whether `--stop-growth` ended the rounds before the budget, and whether `--push-growth` played the game twice |
| `quiet_rounds` | where `--stop-growth` ended the rounds, their quiet rounds; null otherwise |
| `repeats` | for a repeat, the n of the line whose promotions it plays; null otherwise |
| `setup` | as in [a results line](../REFERENCE.md#a-results-line---out-schema-5), without `cap`; null on a `trivial` or `prime` line |
| `effort`, `budget`, `seed` | the level of effort, the work its rounds run until, and the seed of the rounds' random choices |
| `fix`, `tilt` | the range of `--fix` (the lowest and the highest share of the incumbent's promotions a neighborhood keeps fixed; a game that widens takes the ladder's shares instead, and this is the default range), and `--tilt`'s P (0 without it) |
| `back`, `above`, `widen` | `--back`'s J (0 without it), `--above`'s J (0 without it), and `--widen`'s S (null without it) |
| `unseeded_every`, `unseeded` | `--unseeded-every`'s K (0 without it), and whether the line is of an [unseeded play](shortfalls.md#games-played-twice) (as `--unseeded` plays every game) |
| `unseeded_effort`, `stop_growth`, `stop_quiet`, `push_growth` | the level of `--unseeded-effort`, the T of `--stop-growth` and of `--push-growth`, and the K of `--stop-quiet` (0 for none); null for none |
| `start` | the score of a searched game's start; null on any other line |
| `trajectories` | for a searched game, what each incumbent its rounds kept began from: `"start"`, then the games whose survivors it began from, such as `["start",4231,4224]`; null on any other line |
| `survivor_scores` | for a searched game, each game whose moves a trajectory could begin from, in the order of `trajectories`, with the score of what survives of its moves here, whether or not it began one, such as `[[594,110682],[593,110489]]`; null on any other line |
| `rounds`, `neighborhoods`, `steps`, `work` | the rounds done, their neighborhoods, the engine's steps in them, and their work (the steps and 32 for each neighborhood) |
| `stage_rounds` | for a searched game, for each trajectory, the rounds it spent at each of the three stages of `--widen` (all at stage 0 without it); null on any other line |
| `improvements` | for each round that raised the score: `round` (0: before any round, a trajectory that began heavier than the start), the `trajectory` (an index in `trajectories`) that made it and its `stage` in that round, the `neighborhoods`, the `steps` and the `work` of the rounds up to and including this one, the `seconds` until it ended, and the new `score` |
| `other` | for a [game played twice](shortfalls.md#games-played-twice), the play this line is not: `unseeded`, whether it was the unseeded play, and its `method`, `score`, `rounds`, `steps`, `work` and `seconds`; null on any other line |
| `seconds`, `start_seconds` | the game's wall time, from the end of the game before and of the repair's walk after it (or the start of the run) to its line (both plays of a game played twice), and the part of its play before the rounds; a game's setup is made while the game before plays, so it counts only in the first game of a run |
| `threads` | the threads it ran on |
| `from_known` | true if `--from-known` started the game from the built-in table's moves for n − 1 |
| `stop_at_known` | with `--stop-at-known`: `known`, the built-in table's score, and `round`, the round after which the game had that score (0: the start or the start of a trajectory, or a game that runs no round), or null if it never had; null without the flag |
| `repair` | true unless `--no-repair` |
| `repaired` | null on a line as its game was played; on a line that [the repair](shortfalls.md#the-repair) wrote again, `from`, the game whose line gave the new moves (a game above, or for a prime the game below), `was`, the score of the line it replaced, and `seconds`, the time the repair took |
| `tactics`, `build` | as in a results line, `build` the one that wrote the line |
| `best_known` | only on a line of the [best-known file](best-known.md), after the others: `file`, the name of the run's `--out` file that the line came from (null for a run with no file), and `was`, the score of the game's line in the best-known file before this one (null for the first) |
