# The frame solver: reference

This page is the frame solver's reference: every flag of `solve`, the other commands, the output, and what a result
records and why it can be trusted.  [The frame solver's README](../frame/README.md) is the guide to it: what it
computes, how to build and run it, and the common tasks.  The [glossary](../../docs/GLOSSARY.md) defines the project's
words.  Below, "the solver" is the frame solver, and `taxman` is `solver/frame/target/release/taxman`, run from the top
of the repository.

* [1. The flags of `solve`](#1-the-flags-of-solve)
* [2. The other commands](#2-the-other-commands)
* [3. Reading the output](#3-reading-the-output)
* [4. Why the answers can be trusted](#4-why-the-answers-can-be-trusted)

## 1. The flags of `solve`

| flag | meaning |
|---|---|
| `--threads K` | worker threads (default: all CPUs; honors affinity masks and cgroup limits) |
| `--background` | run every search worker under the idle scheduling policy (Linux `SCHED_IDLE`), so that any other process, even one at `nice -n 19`, runs before the workers and the search takes the CPU the rest of the machine leaves.  Use it on a machine shared with other work, with every hardware thread (no `--threads`).  The coordinating thread of a search and the finder keep normal priority.  Elsewhere than Linux it does nothing and says so once on stderr.  It changes only the speed; a line records it in `options` |
| `--window K` | games of a range in flight at once (also for `regress`): game n + 1 is set up, and searched from the best moves so far for n, while n searches, and the threads move to the searches that keep them busy; games are still concluded and recorded in order, with the scores and proof kinds of a window of 1 (default 4; 1 is one game after another, and it is always 1 with one thread) |
| `--tactic NAME=VALUE` | set a tactic; repeatable ([design/tactics.md](design/tactics.md) lists them); the tactic `conflict_branching`, on by default, chooses the conflict a node branches on where the gap is large ([design, "Conflict branching"](design/search.md#conflict-branching)); `--tactic conflict_branching=off` branches on the smallest conflict the search knows; the tactic `conflict_look`, on by default, adds [a look](design/search.md#conflict-branching) at the search's own conflict first, and `--tactic conflict_look=off` leaves it out; the tactic `near_repeat`, on by default, cuts the nodes that the proven game of n − 1 shows hold nothing better than the incumbent ([design, "The near-repeat cut"](design/search.md#the-near-repeat-cut)), and `--tactic near_repeat=off` turns it off; the tactics `price_reuse` and `warm_start`, on by default, are the speed-ups of the chain's re-run ([design/tactics.md](design/tactics.md)), and `--tactic price_reuse=off` and `--tactic warm_start=off` turn them off |
| `--out FILE` | append result lines; skip the games the file completed; the game before the first comes from it |
| `--checkpoint-dir DIR`, `--checkpoint-interval S` | checkpoints (default every 60 s) |
| `--progress S` | seconds between progress lines on stderr (default 60; 0: none) |
| `--time-limit S`, `--node-limit N` | stop one game's search, keeping its checkpoint |
| `--mem-cap UNITS` | work units kept in memory before spilling to disk |
| `--force-search` | search where the previous game or an earlier search would give the answer (skips `prime_shortcut` and `same_search`) |
| `--finder-after NODES`, `--finder-seconds S` | once a game's search has taken NODES steps (over every run of it), start a finder beside it for S seconds (default 900): a thread that looks for better moves than the incumbent, from the incumbent and from the moves for n − 1, and offers it to the search ([design/finder.md](design/finder.md)).  NODES defaults to 1000000; 0 starts none.  Neither is a tactic or in a checkpoint's header, so a checkpoint resumes with any value, and neither changes an answer; a line records them in `options` unless NODES is 0 |
| `--profile DIR`, `--profile-depth D` | keep the profile of each game's search in `DIR/nNNNN.profile.jsonl` (`DIR` is made when the run starts, and the run fails if it cannot be): an id for every node at most D branchings below the root (default 30, at most 100,000 ids a game), the steps of each id and by number of promotions in each checkpoint interval, and the open units at each checkpoint ([design/engine.md, "The profile of a search"](design/engine.md#the-profile-of-a-search)); `solver/tools/read_profile.py FILE` prints where the steps went.  It is appended and synced at each checkpoint and goes on with a resumed search.  It is no tactic, no part of a checkpoint or of a results line, and changes no search |
| `--carried-dir DIR` | carry nogoods into each game's search, with the tactic `carried_nogoods`, which the flag turns on (`DIR` is made when the run starts, and the run fails if it cannot be): at its start and at each resume, a search reads the stores `DIR/nogoods-N.txt` of its game and the games before it, the latest game's first, and keeps those that are nogoods of its root; and it writes its own, `DIR/nogoods-N.txt`, of the nogoods it learns, before each checkpoint and at its end ([design/carried-nogoods.md](design/carried-nogoods.md)).  They only prune, and change no result |
| `--gate GAP` | the gate each game's search starts with, in place of the default, 300 ([design/search.md, "Conflict branching"](design/search.md#conflict-branching)): the gap, a whole number from 0 to 4,000, from which conflict branching chooses its conflict; a gate trial, if asked for, compares its gates with it.  It is refused where conflict branching is off (`--tactic conflict_branching=off`), since a gate does nothing there.  It changes only where the search branches, so no answer; it is no tactic and no part of a checkpoint, which resumes with any gate, and a line records it in `options` |
| `--gate-trial GATES`, `--gate-trial-plan SPEC` | off by default, so a search keeps its gate ([design/gate-trial.md](design/gate-trial.md)).  With `--gate-trial`, each game's search with conflict branching on tries the gates GATES (comma-separated, each from 0 to 4,000) and its own on a sample of its units, once it has taken 50 million steps, and branches from then on with the gate that took the least CPU, if it took 3% less than its own.  `--gate-trial none` says there is no trial.  SPEC, `NAME=VALUE` separated by commas, changes the trial's numbers: `after` (steps, default 50000000), `units` (24), `steps` (the most of each solve, 150000), `threads` (4) and `least_gap` (300); without `--gate-trial` it asks for a trial of the gates 1000, 800 and 600, and it is refused with `none`.  A trial runs only with a checkpoint directory.  It changes only the gate, so no answer; it is no tactic and no part of a checkpoint, and a line records it in `engine` and `options` |
| `--show` | print each game's promotions and moves |

## 2. The other commands

```
$ taxman regress --to 400                       # solve 1..400 in memory, check every score against the built-in table
solved 1..=400 (2 threads, tactics default) in 0.2 s
by method: trivial 13, prime 78, repeat 151, start 43, search 115
400 games match the table, 0 are not in it
```

`regress` is the quick end-to-end check that the build still gives the built-in table's scores.  It solves `--from 1
--to 400` by default, and takes the `--tactic` flags.  A score that differs stops it, as it stops any run.

```
$ taxman verify-results results/optimal.jsonl.gz  # replay every game of the committed chain; see section 4
$ taxman tables 85 8 45                         # the maximal factors and composites of 8 and 45 on the board 1..=85
8: maximal factors [4]; maximal composites [16, 24, 40, 56]
45: maximal factors [9, 15]; maximal composites []
```

* **`verify-results [--threads K] FILE...`** checks each file as `solve` checks its `--out FILE` when it opens it.  The
  checks run in order, but the search problems they set up again for the repeats, most of the time on a long file,
  are set up first, on K threads (default: all CPUs).  With `--threads 1` it sets them up one at a time, as the checks
  need them.  The result, and the first line to fail, are the same for any K.  `solve` takes the threads for this from
  its own `--threads`.
* **A results file named by path**, in `verify-results`, is read through the gzip decoder if the path ends in `.gz`, as
  `results/optimal.jsonl.gz` does, and as plain JSONL if it does not.  The files that `--out` writes are plain JSONL.
  Except in `verify-results`, which reads a file whole, a gzip file is decoded only as far as the lines asked for, and
  must be one game a line ([design/engine.md](design/engine.md)).

## 3. Reading the output

### The summary line (stdout, one per game)

Four lines from different runs:

```
n=250   score=19533     search  nodes=34   0.003 s  optimal ok  proof=exhausted  active=0.003 s  cpu=0.005 s  best=1213
n=1220  score=468662    search  nodes=1375952   8.209 s  incomplete  active=8.151 s  cpu=31.402 s  best=35959
n=1299  score=531389    repeat  nodes=0               0.079 s  optimal ok  proof=same_search of 1298  active=0.079 s  cpu=0.000 s  best=40571
n=1301  score=532272    prime   nodes=0               0.016 s  optimal ok  proof=prime_formula  active=0.016 s  cpu=0.000 s  best=-
```

The fields, in order:

1. n;
2. the score, or for an incomplete game the best found so far;
3. the method: `trivial`, `prime`, `repeat`, `start` or `search`;
4. the search's nodes, which are steps ([glossary](../../docs/GLOSSARY.md#the-search));
5. the seconds the game took in this invocation;
6. how the score compares with the built-in table: `optimal ok`, equal; `optimal unknown`, the table has no line for n
   (a game past its end); or `incomplete`.  A score that differs is a hard error, which stops the run, naming the game;
7. the proof kind, with the game whose search a `same_search` repeats;
8. what the game assumes, if anything (section 4);
9. `active=`, the seconds the game had a worker, over every invocation of the game, where the fifth field counts this
   invocation only: the `seconds.active_all_runs` of its results line ([the clock](design/engine.md#the-window-games-in-flight));
10. `cpu=`, the CPU seconds of the game's search workers, over every invocation (`seconds.cpu_all_runs`), 0.000 for a
    game with no search to run (a `trivial`, `prime` or `repeat` game);
11. `best=`, the game's best promotion sum: the score less `in_the_bag`, which for an incomplete game is the best so far,
    or `-` for a game that has no `in_the_bag` (a `prime` game).

The line of an incomplete game ends with them, after `incomplete`.

### The progress line (stderr, every `--progress` seconds and at the end)

```
12:34:56Z n=992 active 0.3s waited 0.0s cpu 0.5s nodes 4095 (stopped) best 24175 queue 12 spilled 0 | eff 94.3% idle 0.0% lock 0.0% splits 0 steals 0 full 0.2ms ramp 0.0% tail 0.0% rebuild 0.0% first 0.3% pauses 1 (5.6%) max 14.0ms park 0.0ms | nogoods learned 356 lookups 2975 hit 87.9% ruled out 1402 forgot 0 (table 0) largest 356
```

| field | meaning |
|---|---|
| `12:34:56Z` | the UTC time of day the line was printed |
| `n=992 active 0.3s waited 0.0s cpu 0.5s` | the game, and what it has spent over all invocations: the time it has been active (had a worker), the time it has waited (in flight with none), and the CPU time of its search's workers ([the clock](design/engine.md#the-window-games-in-flight)).  Times read `12.3s`, `1m05s`, `1h02m03s`, and CPU time from an hour on `112.3h`.  A search outside a window is active throughout and never waits |
| `nodes 4095 (stopped)` | steps taken; in the middle of a run the rate (`164.6k/s`), at the end `done`, `optimal` or `stopped` |
| `best 24175` | the incumbent: the largest *promotion sum* so far (the score is `in_the_bag` plus this) |
| `queue 12 spilled 0` | work units in the shared in-memory deque, and spilled to disk |
| `eff` | effective parallelism: the time the workers spent searching, over the time they were alive (the wall time times the threads, unless the number of workers changed) |
| `idle`, `lock` | the shares of that time spent waiting for work, and waiting for the deque's lock |
| `splits steals` | the times a worker handed work to idle ones, and the units taken from the front of the deque |
| `full 0.2ms` | the time until every worker first ran at once |
| `ramp`, `tail` | the shares of worker time spent waiting for work before every worker first ran at once, and after the last time a worker finished a unit while all were running: the search's start and end, when there is not yet, or no longer, work for all |
| `rebuild`, `first` | the shares of worker time from the start of a unit to its first step (rebuilding the search's state from the unit), and in that first step (its bounds solved from scratch) |
| `pauses 1 (5.6%) max 14.0ms park 0.0ms` | the checkpoints, each of which parks all workers: their share of worker time, the longest pause, and the longest wait for the workers to park |
| `nogoods learned 356 ... largest 356` | the searchers' nogood tables (tactic `nogoods`), summed: nogoods learned, lookups and the share they answered, candidates ruled out, forgettings with the mean table before, and the largest table |

### A results line (`--out`, schema 5)

A results file has one JSON object per line, compact.  Here is the line of game 250 from a run of 1-300 on two
threads, with the moves cut short, the fields regrouped and comments added, so it is not valid JSON as printed:

```
{"schema":5, "n":250, "score":19533, "moves":[241,169,221,...], "promotions":[84,98,104,105,110,112,114,117,120,124,125],
 "method":"search", "completed":true,
 "proof":"exhausted", "reached_cap":false, "assumes":[],
 "in_the_bag":18320,                      # score = in_the_bag + sum(promotions): 18320 + 1213 = 19533
 "prev":{"n":249,"score":19382,"source":"run","moves_digest":"44877b44d9bbf51a","proof":"same_search","assumes":[]},
 "same_search":null,                      # a repeat names the game it repeats here
 "nodes":2, "stats":{"promote_attempts":1,"promotions":10,"children":0,"match_bounds":2,"near_repeat_cut":0},
 "engine":{"splits":0,"checkpoints":0,"spilled_units":0,"max_queue":1},
 "setup":{"frames":4,"candidates":48,"max_total":11,"greedy_sum":1194,"cap":1312,
          "digest":"fe5e05eb67406e98"},
 "started":"2026-10-14T09:12:03Z", "finished":"2026-10-14T09:12:03Z",
 "seconds":{"frames":0.0002,"greedy":0.00001,"cpu":0.0011,"active":0.0013,"waiting":0.0003,
            "cpu_all_runs":0.0011,"active_all_runs":0.0013,"waiting_all_runs":0.0003,"total":0.0015},
 "invocations":1, "threads":2,
 "tactics":"default",
 "tactic_switches":{"prime_shortcut":true,"same_search":true,"fix_forced":true,"nogoods":true,
                    "forget_nogoods":true,"symmetry":true,
                    "conflict_branching":true,"conflict_look":true,"near_repeat":true,"price_reuse":true,
                    "carried_nogoods":false,"warm_start":true,"prev_cap":true,"seed_incumbent":true},
 "limits":{"seconds":null,"nodes":null},
 "options":{"force_search":false},
 "optimal_match":true, "build":{"commit":"e8c39cfac7e6...","dirty":false}}
```

Game 249 of that run repeated the search of 248.  Its line has `"method":"repeat"`, `"proof":"same_search"`,
`"same_search":{"n":248,"digest":"a1b7113c4c3d1567","promotion_sum":1187,"proof":"exhausted","assumes":[]}`, and a
`setup` with `"cap":null` and the same `digest`.

| field | meaning |
|---|---|
| `schema`, `n` | the format of the line (5, the only one this build reads), and n |
| `score`, `moves`, `best_so_far` | the game and its score; for an unfinished search `score` is null, and `best_so_far`, a key that only such a line has, holds the best so far |
| `promotions` | the promoted numbers, ascending |
| `method` | how the game was found: `trivial`, `prime`, `repeat`, `start` (the search's starting moves reached a ceiling, so the search took no step and `nodes` is 0) or `search` (its tree ran) |
| `completed` | false only if a limit or a signal stopped the search; such a line records an unfinished search, and its checkpoint is kept |
| `proof` | the proof kind, how the game was proven optimal: `trivial`, `prime_formula`, `cap`, `exhausted` or `same_search` (section 4); null if incomplete |
| `reached_cap` | the promotions reach `cap`, where the search stops (a search started before its ceiling was known, in a window of games in flight, may have run to the end of its tree; [solver/docs/design/trust.md](design/trust.md), "What a window changes in the results") |
| `assumes` | what the game assumes beyond the frame model, here or in the games it rests on (n − 1, or the game a repeat repeats), as plain statements; `[]`: nothing, which is every line of a chain from game 1 |
| `in_the_bag` | what the game with no promotions scores (null for the prime formula, which sets no board up) |
| `prev` | the moves for n − 1 this one was built from: its score, where it came from (`run`: this run's, or `results`: the results file's), a digest of its moves, its proof kind and what it assumes; null if there was none |
| `same_search` | for a repeat, the earlier game of the file whose search it repeats: its n, the digest of the search problem they share, the problem's best promotion sum, that game's proof kind and what it assumes; null otherwise |
| `nodes`, `stats` | the search's steps over all invocations, and the searcher's counters (`match_bounds` counts the node bounds computed); `stats.near_repeat_cut`, a key that only the line of a search armed with the [near-repeat cut](design/search.md#the-near-repeat-cut) has, counts the nodes the cut cut in the last invocation, 0 if it was armed and cut none |
| `engine` | this invocation's splits, checkpoints written, units spilled and largest in-memory queue; and, only on a line whose search started a finder (`--finder-after`), `finder`: `at_nodes` (the search's steps then), `from` (the incumbent's promotion sum then), `best` (the best promotion sum the finder reached), `improved` (its sets that became the incumbent), `refused` (its sets the checks refused: 0), `seconds` (how long it ran) and `found_seconds` (when it reached `best`); and, only on a line whose search ran a gate trial to its end (`--gate-trial`), `gate_trial`: `at_nodes` (the search's steps at the checkpoint it took its units from), `units`, `from` (the gate then), `scores` (for each gate tried: `gate`, `steps`, `cpu_seconds`, `stopped`), `chosen` and `seconds` |
| `setup` | null on a `trivial` or `prime` line, which set up no search; otherwise the frames and candidates found, `max_total` (the most promotions the freedoms allow), the greedy start, `cap` (the ceiling as a promotion sum, `n + score(n-1) - in_the_bag`; null on a repeat, which ran no search to stop), and `digest`, a digest of the search problem the setup poses ([solver/docs/design/same-search.md](design/same-search.md)), by which a later game finds this one to repeat |
| `started`, `finished` | the UTC times, to the second (RFC 3339, `2026-10-14T09:12:03Z`), at which this invocation started the game's setup and made its line |
| `seconds` | what the game spent: `frames` and `greedy`, the setup's phases; `cpu`, `active` and `waiting` ([the clock](design/engine.md#the-window-games-in-flight)); and `total`, the time the game's thread spent in its setup, repeat lookup, search (the wait of a suspended search included) and conclusion up to the replay.  `cpu` is the CPU seconds of the search's workers, 0 for a game with no search to run (a `trivial`, `prime` or `repeat` game); `active` the wall seconds the game had at least one worker, its setup included; `waiting` the wall seconds it was in flight with none, so that `active` and `waiting` add up to the time from `started` to `finished`.  Each is this invocation's, and `cpu_all_runs`, `active_all_runs` and `waiting_all_runs` add the invocations before it that a resumed game's clock kept |
| `invocations` | how many invocations ran the search (a resumed game has more than one) |
| `threads`, `tactics`, `tactic_switches`, `limits`, `options` | what the last invocation ran with; `tactics` is `default` or the changes from the defaults, `options` the flag `--force-search`, `gate_trial` (the gates of the gate trial, only where one was asked for) and, only where given, `background` (true), `finder_after` and `finder_seconds` (the numbers of the finder flags, recorded where the finder is on) and `gate` (the number `--gate` was given) |
| `optimal_match` | true: the score equals the built-in table's (a score that differs is a hard error, never written); null if the table has no line for n |
| `build` | the commit the binary was built from, and whether its sources differed from it |

This build reads schema 5 only: a results file of an older schema has to be made again.  A line is never rewritten.  A
resumed game that is stopped again adds another incomplete line, and the completed line comes last.

## 4. Why the answers can be trusted

Every completed line names how its game was proven optimal, its *proof kind*, and what that rests on beyond the frame
model, its *assumptions*.  The referee replays every game before it counts, and two verifiers that share no code
check every results file ([verifiers.md](verifiers.md) says what each checks).

### Proof kinds

| proof kind | meaning | rests on the moves for n − 1? |
|---|---|---|
| `trivial` | the game has no promotion candidates, so the game with no promotions is the only one | no |
| `exhausted` | the search explored its whole tree | no |
| `cap` | the search reached the [ceiling](../../docs/GLOSSARY.md#the-game) | yes |
| `prime_formula` | the [prime formula](../../docs/GLOSSARY.md#the-game), optimal if the moves for n − 1 are | yes |
| `same_search` | the game's [search problem](../../docs/GLOSSARY.md#the-frame-model) is the one an earlier game m of the file posed, and the way m was proven (`exhausted`, `cap` or `same_search`) settled its best promotion sum; the game is m's promotions on this board (tactic `same_search`; [design/same-search.md](design/same-search.md) has the argument) | no: on the game m it repeats |

A search that the near-repeat cut shortened is `exhausted` all the same: it explores its whole tree, with one more cut,
and the cut rests on the game that the line's `prev` names, which is proven and assumes nothing.  The line says that the
cut shortened the search with `stats.near_repeat_cut` above 0.

### Assumptions

What a game *assumes* is what its proof kind rests on beyond the frame model:

* A game whose proof kind rests on the moves for n − 1 assumes what that game assumes, and so on down the chain.
* A repeat rests on the game it repeats instead, and assumes what that game assumes.
* A game's own search rests on nothing beyond the frame model, so a chain from game 1 assumes nothing, and every line of
  `results/optimal.jsonl.gz` has `"assumes":[]`.  Moves for n − 1 that a caller of the library gives a run (`solver::Prev`)
  come with what they assume, and the games that rest on them assume it too.

A game that assumes nothing is proven within the frame model, given that the solver is right.  `taxman solve` prints
what a game assumes in its summary line, and `verify-results` counts the games by what they assume.

What a game rests on through the frame model, statement U at n ([design/frame-model.md, “What "optimal"
means”](design/frame-model.md#what-optimal-means)), is not recorded for the games that the frame model proves: those
proven by an exhausted search, the trivial games and the repeats.  It holds at every game the prover has proven; past
those, it is not yet proven.

### What a game had from n - 1

A line says what it had from the moves for n − 1 in its `prev`.

| `prev` | what the game had from the moves for n − 1 | what its proof kind can be |
|---|---|---|
| `null` | nothing: it was solved standalone (the game before it is missing), with no seed and no ceiling (`setup.cap` null) | `trivial`, `exhausted` or `same_search` on a completed game of the file |
| `proof` given | n − 1 proven: the seed, the ceiling n + score(n − 1), the prime formula for a prime n, and repeats | any |

The moves for n − 1 that a `run` or `results` line names are the completed line of n − 1 in the same file.  A game whose
search a limit or a signal stopped has an incomplete line, and, once it is resumed, the completed line comes last.

So a file that starts above game 1, has a gap (where a run began with a standalone game), or ends with a game that only an
unfinished search is recorded for, reads this way:

* A completed game was proven in the way its `proof` names, given what its `assumes` says.  A `cap` or `prime_formula`
  rests on a proven `prev`, a `same_search` on the game it repeats, and the others on nothing of n − 1.
* An unfinished game is a lower bound.
* Where a game and the one before it are both completed, the chain bounds hold between them.
