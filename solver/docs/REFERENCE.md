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
| `--background` | run every search worker under the idle scheduling policy (Linux `SCHED_IDLE`), so that any other process, even one at `nice -n 19`, runs before the workers and the search takes the CPU the rest of the machine leaves.  Use it on a machine shared with other work, with every hardware thread (no `--threads`).  The coordinating thread of a search, the finder and the lab's hook keep normal priority.  Elsewhere than Linux it does nothing and says so once on stderr.  It changes only the speed; a line records it in `options` |
| `--window K` | games of a range in flight at once (also for `regress`): game n + 1 is set up, and searched from n's best game so far, while n searches, and the threads move to the searches that keep them busy; games are still concluded and recorded in order, with the scores and proofs of a window of 1 (default 4; 1 is one game after another, and it is always 1 with one thread) |
| `--tactic NAME=VALUE` | set a tactic; repeatable ([design/tactics.md](design/tactics.md) lists them); the tactic `conflict_branching`, on by default, chooses the conflict a node branches on where the gap is large ([design, "Conflict branching"](design/search.md#conflict-branching)); `--tactic conflict_branching=off` branches on the smallest conflict the search knows, as the builds before it did; the tactic `conflict_look`, on by default, adds [a look](design/search.md#conflict-branching) at the search's own conflict first, and `--tactic conflict_look=off` leaves it out; the tactic `near_repeat`, on by default, cuts the nodes that the proven game of n − 1 shows hold nothing better than the incumbent ([design, "The near-repeat cut"](design/search.md#the-near-repeat-cut)), and `--tactic near_repeat=off` turns it off |
| `--out FILE` | append result lines; skip the games the file completed; the game before the first comes from it |
| `--checkpoint-dir DIR`, `--checkpoint-interval S` | checkpoints (default every 60 s) |
| `--progress S` | seconds between progress lines on stderr (default 60; 0: none) |
| `--time-limit S`, `--node-limit N` | stop one game's search, keeping its checkpoint |
| `--game-hours H` | the wall: end the range at the first game whose own search has passed H hours in all, with the exit status 4 and nothing recorded for that game ([README, "Checkpoints, resuming, signals"](../frame/README.md#checkpoints-resuming-signals)) |
| `--mem-cap UNITS` | work units kept in memory before spilling to disk |
| `--force-search` | search where the previous game, an earlier search or a known source would give the answer (skips `prime_shortcut`, `same_search` and the known games of `--use-prev`) |
| `--use-prev` | draw on the known games of `optimal.json`, the committed chain and the bounds store ([README, "Known games"](../frame/README.md#known-games)): a game one of them has proven is recorded with no search, citing it and what it assumes; the known game of n − 1 gives the ceiling and the seed when neither the run nor `--out` has it; the store's other games start their searches from its lower bound's game as well as the game of n − 1, the better start of the two, and stop at its upper bound |
| `--keep-going` | a game that a limit stops does not end the range: its best game so far is an unfinished line and seeds the next game, which takes nothing that rests on its being optimal; the games are solved one at a time; a rerun passes over the unfinished games ([README, "Keeping going past limits"](../frame/README.md#keeping-going-past-limits-and-the-sweep)) |
| `--retry`, `--retry-improved` | with `--keep-going`, search again every unfinished game of `--out`, or only those whose game before now has more to give than when they were tried |
| `--pass N` | record N as `options.pass` of every line written, for a script that goes over the games more than once; no other effect |
| `--finder-after NODES`, `--finder-seconds S` | once a game's search has taken NODES steps (over every run of it), start a finder beside it for S seconds (default 60): a thread that looks for a better game than the incumbent, from the incumbent and from the game of n − 1, and offers it to the search ([design/finder.md](design/finder.md)).  NODES defaults to 1000000; 0 starts none.  Neither is a tactic or in a checkpoint's header, so a checkpoint resumes with any value, and neither changes an answer; a line records them in `options` unless NODES is 0 |
| `--lab-dir DIR` | the command folder of a lab ([design/lab.md](design/lab.md)): at each progress tick the run takes the files of `DIR/in`, one JSON command each (`checkpoint`, `offer`, `branching`, `policy`, `restart`, `threads`, `finder`, `redeploy`), applies or refuses each, writes its outcome to `DIR/out` and moves it to `DIR/done`; a lab changes how the games are searched, never their results.  A `redeploy` ends the range with every search checkpointed, prints `redeploy to COMMIT` and exits with the status 5 |
| `--profile DIR`, `--profile-depth D` | keep the profile of each game's search in `DIR/nNNNN.profile.jsonl`: an id for every node at most D branchings below the root (default 30, at most 100,000 ids a game), the steps of each id and by number of promotions in each checkpoint interval, and the open units at each checkpoint ([design/engine.md, "The profile of a search"](design/engine.md#the-profile-of-a-search)); `solver/tools/read_profile.py FILE` prints where the steps went.  It is appended and synced at each checkpoint and goes on with a resumed search.  It is no tactic, no part of a checkpoint or of a results line, and changes no search |
| `--hook PROGRAM`, `--hook-after S` | start PROGRAM, detached, with one JSON argument: the event `long` once a game's own search passes S seconds (default 3600; once a game), and `done` when such a game has its line.  The argument's fields: `event`, `n`, `nodes` (steps over every run), `best` (the best promotion sum), `own_seconds`, `checkpoint_dir` and `lab_dir` (absolute paths, or null), `commit`, and for `done` `result`, the game's line as an object; others may follow ([design/lab.md, "The hook"](design/lab.md#the-hook)) |
| `--show` | print each game's promotions and moves |

## 2. The other commands

```
$ taxman regress --to 400                       # solve 1..400 in memory and check every score against optimal.json
solved 1..=400 (2 threads, tactics default) in 0.2 s
by method: trivial 13, prime 78, repeat 151, start 43, search 115, known 0
400 games match optimal.json, 0 are not in it
```

`regress` is the quick end-to-end check that the build still gives the table's scores.  It solves `--from 1 --to 400`
by default, and takes the `--tactic` flags and `--use-prev`, which takes only the game before its range, from the
table.  A score that differs stops it, as it stops any run.

```
$ taxman verify-optimal                         # replay every game of optimal.json
verified 1300 games (n up to 1300, 367449 moves)
load+parse 12.8 ms, replay 18.6 ms
$ taxman verify-results results/strict.jsonl    # see section 4
$ taxman extend-optimal results/strict.jsonl --out /tmp/optimal.json     # the table plus the games it lacks
$ taxman tables 85 8 45                         # the maximal factors and composites of 8 and 45 on the board 1..=85
8: maximal factors [4]; maximal composites [16, 24, 40, 56]
45: maximal factors [9, 15]; maximal composites []
$ taxman bench --history /tmp/h.jsonl           # time the benchmark games 936 ... 1184, each on its own, no derivation
```

* **`bench`** appends one JSON line per run to its history file: `solver/frame/bench/history.jsonl` of the repository,
  unless `--history FILE` names another (pass it to keep the checkout clean).  It takes the games past `optimal.json`'s
  from `results/strict.jsonl`, unless `--known FILE` names another results file.
* **The ladder** is the harness for hard games and for scaling over thread counts: `cargo run --release --example
  ladder -- --scaling --seed-opt --known ../../results/strict.jsonl --seconds 600 1215`, in `solver/frame`.  Its
  flags are documented at the top of `examples/ladder.rs`.
* **`verify-results [--threads K] FILE...`** checks each file as `solve` checks its `--out FILE` when it opens it.  The
  checks run in order, but the search problems they set up again for the repeats, most of the time on a long file,
  are set up first, on K threads (default: all CPUs).  With `--threads 1` it sets them up one at a time, as the checks
  need them.  The result, and the first line to fail, are the same for any K.  `solve` takes the threads for this from
  its own `--threads`.
* **`extend-optimal RESULTS --out FILE`** writes the table of optimal games to FILE with the games of the results file
  RESULTS appended.  The table is the built-in one, or the file that `--table FILE` names, say a table extended before
  the binary was rebuilt with it.  It reads RESULTS with the checks of `verify-results`, which refuse it if a score
  differs from the table's for any n they share.  Then, from the game after the table's last, it takes each game that
  RESULTS completed, up to the first that is missing or only an unfinished search, and says which and why.  A game that
  assumes a game of the table optimal is taken too: it assumes nothing the table does not stand for.  The old games are
  written back byte for byte, FILE is written whole or not at all, and RESULTS can never be FILE.  The table stays at
  1300, so the command adds every game of `results/strict.jsonl` past it; on 2026-10-23 it said:

  ```
  $ taxman extend-optimal results/strict.jsonl --out /tmp/optimal.json
  added 3649 games (n = 1301..=4949); the table has 4949 games, n up to 4949; wrote /tmp/optimal.json
  stopped at n = 4950: the results end at n = 4949
  ```

  Check the new file with `taxman verify-optimal /tmp/optimal.json` before it replaces `results/optimal.json`.  The
  binaries built after that carry the new table.
* **`certify (A-B | --list FILE) --results FILE... --upper FILE... --out FILE`** proves, game by game, that statement U
  holds: that the recorded scores of A..B that rest on the frame model are opt(n) on the whole board
  ([design/frame-model.md, “What "optimal" means”](design/frame-model.md#what-optimal-means)).  The list FILE instead
  names the games, one n a line, in order.  For each game that the prover's barred bounds (`--upper`,
  `solver/noframe/data/upper_*.jsonl`) say needs it, it searches the extended problem, with the bounds' open members
  optional, from the recorded game.  The search ends `certified` (nothing beats the score) or `beaten`.  A game that
  beats the score is written to `OUT.counterexample.jsonl` too, and the run exits with the status 2.  `certify` takes
  `--threads`, `--background`, the checkpoint flags and the limits of `solve`, and resumes like it.  The extended
  problem keeps the claims of `unzip` that [the kept-claims lemma
  (E2)](../../docs/theory/extended-problem.md#the-kept-claims-lemma) allows, unless `--no-claims`, the formulation of
  the certificates made before it; a line records which.
* **`verify-certificates CERTS... --results FILE... --upper FILE... [--exact FILE...] [--threads K]`** checks the files
  that `certify` wrote, without changing them, line by line.  It says how far the proofs of the recorded scores reach:
  the prover's, the certificates' and the ceilings'.  [design/certificates.md](design/certificates.md) has what they
  check and what a line holds.

## 3. Reading the output

### The summary line (stdout, one per game)

Five lines from different runs:

```
n=250   score=19533     search  nodes=34   0.003 s  optimal ok  proof=exhausted  own=0.003 s  best=1213
n=1220  score=468662    search  nodes=1375952   8.209 s  incomplete  own=8.151 s  best=35959
n=1299  score=531389    repeat  nodes=0               0.079 s  optimal ok  proof=same_search of 1298  own=0.000 s  best=40571
n=1301  score=532272    prime   nodes=0               0.016 s  optimal unknown  proof=prime_formula  assumes the game of 1300 in optimal.json is optimal  own=0.000 s  best=-
n=4754  score=7106699   known   nodes=0               0.147 s  optimal unknown  proof=known from the bounds store (bounds.jsonl)  assumes statement U holds at n=4753  own=0.000 s  best=-
```

The fields, in order:

1. n;
2. the score, or for an incomplete game the best found so far;
3. the method: `trivial`, `prime`, `repeat`, `start`, `search` or `known`;
4. the search's nodes, which are steps ([glossary](../../docs/GLOSSARY.md#the-search));
5. the seconds the game took in this invocation;
6. how the score compares with `optimal.json`: `optimal ok`, equal; `optimal unknown`, the table has no game of n; or
   `incomplete`.  A score that differs is a hard error, which stops the run, naming the game;
7. the proof, with the game whose search a `same_search` repeats and the source that a `known` cites;
8. what the proof assumes, if anything (section 4);
9. `own=`, the game's own search time ([glossary](../../docs/GLOSSARY.md#the-search)) in seconds, over every invocation
   of the game, where the fifth field counts this invocation only: the `seconds.own` of its results line, 0.000 for a
   game with no search to run (a `trivial`, `prime`, `repeat` or `known` game);
10. `best=`, the game's best promotion sum: the score less `in_the_bag`, which for an incomplete game is the best so far,
    or `-` for a game that has no `in_the_bag` (a `prime` or `known` game).

A line that a build before 2026-10-22 printed lacks the last two fields, so a program that reads the line takes them as
optional.  The line of an incomplete game ends with them, after `incomplete`.

### The progress line (stderr, every `--progress` seconds and at the end)

```
n=992 0.3s nodes 4095 (stopped) best 24175 queue 12 spilled 0 | eff 94.3% idle 0.0% lock 0.0% splits 0 steals 0 full 0.2ms ramp 0.0% tail 0.0% rebuild 0.0% first 0.3% pauses 1 (5.6%) max 14.0ms park 0.0ms | nogoods learned 356 lookups 2975 hit 87.9% ruled out 1402 forgot 0 (table 0 kept 0) evicted 0 largest 356
```

| field | meaning |
|---|---|
| `n=992 0.3s` | the game, and the time its search has run over all invocations (`1m05s`, `1h02m03s` later) |
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
| `nogoods learned 356 ... largest 356` | the searchers' nogood tables (tactic `nogoods`), summed: nogoods learned, lookups and the share they answered, candidates ruled out, forgettings with the mean table before and the mean kept, nogoods evicted under `nogood_limit`, and the largest table |

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
 "seconds":{"frames":0.0002,"greedy":0.00001,"search":0.0008,"search_all_runs":0.0008,"own":0.0008,"total":0.0015},
 "invocations":1, "threads":2,
 "tactics":"default",
 "tactic_switches":{"prime_shortcut":true,"same_search":true,"match_refine":1,"fix_forced":true,"nogoods":true,
                    "forget_nogoods":true,"nogood_period":50000,"nogood_keep":"none","nogood_limit":0,
                    "lagrange":false,"gf2":true,"symmetry":true,"force_window":"all","branching":"exact",
                    "strong_limit":"all","strong_depth":"all","deep_branching":"split","more_forced":true,
                    "conflict_branching":true,"conflict_look":true,"near_repeat":true,"price_reuse":false,"prev_cap":true,
                    "seed_incumbent":true},
 "limits":{"seconds":null,"nodes":null},
 "options":{"force_search":false,"use_prev":false},
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
| `method` | how the game was found: `trivial`, `prime`, `repeat`, `start` (the search's starting game reached a ceiling, so the search took no step and `nodes` is 0), `search` (its tree ran) or `known` (taken from a known source with no search, `--use-prev`) |
| `completed` | false only if a limit or a signal stopped the search; such a line records an unfinished search, and its checkpoint is kept (a `--keep-going` run writes none for a signal) |
| `proof` | how optimality is established: `trivial`, `prime_formula`, `cap`, `exhausted`, `same_search` or `known` (section 4); null if incomplete |
| `reached_cap` | the promotions reach `cap`, where the search stops (a search started before its ceiling was known, in a window of games in flight, may have run to the end of its tree; solver/docs/design/trust.md, "What a window changes in the results") |
| `assumes` | what the proof assumes beyond the frame model, here or in the games it rests on (n − 1, the game a repeat repeats, or the known source a `known` proof cites): for a game of n − 1 or of n taken from `optimal.json`, "the game of n in optimal.json is optimal"; for a bound of the bounds store, "statement U holds at n=4501" for each game its upper bound rests on; `[]`: nothing |
| `in_the_bag` | what the game with no promotions scores (null for the prime formula and known games, which set no board up) |
| `prev` | the game of n − 1 this one was built from: its score, where it came from (`run`, `results`, `optimal`, `strict`, `bounds`), a digest of its moves, its proof and what it assumes; null if there was none.  A `run` or `results` game with no proof is the best game of an unfinished search (`--keep-going`): a seed, proven by nothing (section 4, "What a game had from n − 1") |
| `same_search` | for a repeat, the earlier game of the file whose search it repeats: its n, the digest of the search problem they share, the problem's best promotion sum, that game's proof and what it assumes; null otherwise |
| `known` | a key that only a line citing a known source has (section 4, "Citing a known source"): `{"source":"optimal"}` or `{"source":"strict"}` for a game taken from `optimal.json` or the committed chain, or `{"source":"bounds","lower":{"rule":..,"source":..},"upper":{"rule":..,"source":..}}` for the lines of the bounds store whose bound of n is the score (`lower` null when a search found the game and the upper bound stopped it) |
| `nodes`, `stats` | the search's steps over all invocations, and the searcher's counters (`match_bounds` counts the node bounds computed, GF(2) or matching); `stats.near_repeat_cut`, a key that only the line of a search armed with the [near-repeat cut](design/search.md#the-near-repeat-cut) has, counts the nodes the cut cut in the last invocation, 0 if it was armed and cut none |
| `engine` | this invocation's splits, checkpoints written, units spilled and largest in-memory queue; and, only on a line whose search started a finder (`--finder-after`), `finder`: `at_nodes` (the search's steps then), `from` (the incumbent's promotion sum then), `best` (the best promotion sum the finder reached), `improved` (its sets that became the incumbent), `refused` (its sets the checks refused: 0), `seconds` (how long it ran) and `found_seconds` (when it reached `best`) |
| `setup` | null on a `trivial`, `prime` or `known` line, which set up no search; otherwise the frames and candidates found, `max_total` (the most promotions the freedoms allow), the greedy start, `cap` (the ceiling as a promotion sum, `n + score(n-1) - in_the_bag`; null on a repeat, which ran no search to stop), and `digest`, a digest of the search problem the setup poses (solver/docs/design/same-search.md), by which a later game finds this one to repeat |
| `seconds` | per phase, this invocation (`search_all_runs`: the search over every invocation of a resumed game); and `own`, the game's own search time ([glossary](../../docs/GLOSSARY.md#the-search)) over every invocation, the clock of the wall.  It is 0 for a game with no search to run (a `trivial`, `prime`, `repeat` or `known` game), and for a search that ran only before its game was the oldest of the window.  A line written before 2026-10-22 lacks `own`; the readers ignore it, and the schema is 5 still |
| `invocations` | how many invocations ran the search (a resumed game has more than one) |
| `lab` | a key that only the line of a game whose search a lab changed has (`--lab-dir`, [design/lab.md](design/lab.md)): `restarts`, each restart's time `at`, the game's `own_seconds` then, the `nodes` of the search abandoned, its `tactics` and the checkpoint put `aside`; `conflict_branching` and `conflict_look`, the lab's last setting of each (`"on"`, `"off"` or null), and `gate`, its last gap from which conflict branching chooses (a number or null); `branching_changes`, how many `branching` commands the lab gave; and `policy`, the policy for the games to come that reached it (`from` and the three settings), or null.  `tactics` and `tactic_switches` are then those the search ran with, its branching as it ended |
| `threads`, `tactics`, `tactic_switches`, `limits`, `options` | what the last invocation ran with; `tactics` is `default` or the changes from the defaults, `options` the flags `--force-search` and `--use-prev` and, only where given, `keep_going`, `retry`, `retry_improved`, `background` (true), `pass` (the number `--pass` was given), and `finder_after` and `finder_seconds` (the numbers of the finder flags) |
| `optimal_match` | true: the score equals `optimal.json`'s (a score that differs is a hard error, never written); null if the table has no game of n |
| `build` | the commit the binary was built from, and whether its sources differed from it |

This build reads schema 5 only: a results file of an older schema has to be made again.  A line is never rewritten.  A
resumed game that is stopped again adds another incomplete line, and the completed line comes last.

## 4. Why the answers can be trusted

Every completed game records how its optimality is established, its *proof*, and what that proof rests on beyond the
frame model, its *assumptions*.  The referee replays every game before it counts, and two verifiers that share no code
check every results file ([verifiers.md](verifiers.md) says what each checks).

### Proof kinds

| proof | meaning | rests on the game of n − 1? |
|---|---|---|
| `trivial` | the game has no promotion candidates, so the game with no promotions is the only one | no |
| `exhausted` | the search explored its whole tree | no |
| `cap` | the search reached the [ceiling](../../docs/GLOSSARY.md#the-game) | yes |
| `prime_formula` | the [prime formula](../../docs/GLOSSARY.md#the-game), optimal if the game of n − 1 is | yes |
| `same_search` | the game's [search problem](../../docs/GLOSSARY.md#the-frame-model) is the one an earlier game m of the file posed, and m's proof (`exhausted`, `cap`, `same_search`, or `known` where an upper bound ended its search) settled its best promotion sum; the game is m's promotions on this board (tactic `same_search`; [design/same-search.md](design/same-search.md) has the argument) | no: on the game m it repeats |
| `known` | a known source (`--use-prev`) holds the game, or an upper bound of it ended the search; the line's `known` field cites the source, and what the proof assumes follows from it (below) | no: on the source |

A search that the near-repeat cut shortened is `exhausted` all the same: it explores its whole tree, with one more cut,
and the cut rests on the game that the line's `prev` names, which is proven and assumes nothing.  The line says that the
cut shortened the search with `stats.near_repeat_cut` above 0.

### Citing a known source

A line of proof `known` has a `known` field that says which source it rests on:

| `known` | the line | the check |
|---|---|---|
| `{"source":"optimal"}` | method `known`: the table's game of n, assumed optimal | the table has a game of n that scores the line's score |
| `{"source":"strict"}` | method `known`: the committed chain's game of n, as proven as its line is | the committed chain has a game of n that scores it; the line assumes what that line does |
| `{"source":"bounds","lower":{..},"upper":{..}}` | method `known`: a game that the bounds store has proven; the `lower` bound is the game played | the store has a lower line and an upper line of game n worth the score, by the `rule` from the `source` (the run that found it) that the field says |
| `{"source":"bounds","lower":null,"upper":{..}}` | method `search` or `start`: the search found the game and stopped at the store's upper bound, which it meets; `reached_cap` is false, since the ceiling of n − 1 is not what it reached | the store has an upper line of game n worth the score, as cited |

The score of the line is the cited value of game n.  The store has one line of a game, side and value, so the game,
the side and the score name the line; `rule` and `source` are that line's own, as `bounds.jsonl` has them.  The score
is optimal wherever the cited bound holds: a finished game that scores what an upper bound allows is the best.  A lower
bound is a game, and a game that the referee has replayed is a lower bound.

When the game before the range came from the committed chain or the store, the first line's `prev.source` is `strict`
or `bounds`, with that game's proof (`known` for the store's) and assumptions.  A proof that rests on such a game
inherits what it assumes, as for a game from `optimal.json`: a proof by the ceiling or the prime formula, or a repeat of
a search that stopped at the store's bound.  The `known` field, method and proof were added within schema 5, so every
file of schema 5 reads, with them or without.

The verifiers check that the game plays and scores the line's score, and that the cited lines are in the sources and
say what the line says.  They do not check the sources again: `ledger verify-bounds` and
`solver/tools/check_bounds.py` check the store, and the verifiers of results files check the committed chain.  Which
game the line plays is not compared with the source's.  So a file with `known` lines is checked against the sources in
the repository's `results/`, the defaults of `taxman verify-results`; `solver/tools/check_results.py` takes them as
`--committed FILE` and `--bounds FILE`.  A file that cites a source that was not given is refused.

A known game that is wrong would be caught three ways: by the chain, since a score outside `score(n − 1) <= score(n)
<= n + score(n − 1)` is an error; by the table, where it has the game; and by the next `ledger verify-bounds`.  A search
that finds a game above the store's bound stops the run, naming the game: either the store is wrong, or statement U
fails at a game that its bound rests on.

### Assumptions

What a game *assumes* is what its proof rests on beyond the frame model:

* A proof that rests on the game of n − 1 assumes what that game assumes, and so on down the chain.
* A repeat rests on the game it repeats instead, and assumes what that game assumes.
* A `known` proof rests on its source, and assumes what the source's game does.
* A game taken from `optimal.json` (`--use-prev`) is assumed optimal, since the table records nothing about how a game
  was found.  A proof that rests on its game of n − 1 assumes "the game of n − 1 in optimal.json is optimal", and a
  game of n taken from it "the game of n in optimal.json is optimal".
* A bound of the bounds store holds wherever statement U holds at the games its upper bound rests on (`rests_on`).  A
  bound of the frame model at game n rests on n, and a bound carried by a relation rests on what its source rests on.
  A game that rests on such a bound assumes "statement U holds at n=4501" for each of those games, and a game that
  rests on that game assumes the same.
* The committed chain's games assume what their lines do: nothing, but for the games it took from the bounds store,
  which assume statement U at the games their bounds rest on ([results/README.md](../../results/README.md#strictjsonl)
  counts them).

A game that assumes nothing is proven within the frame model, given that the solver is right.  `taxman solve` prints
what a game assumes in its summary line, and `verify-results` counts together the games that assume statement U at
some game.

What a proof rests on through the frame model, statement U at n ([design/frame-model.md, “What "optimal"
means”](design/frame-model.md#what-optimal-means)), is not recorded for the proofs that the frame model gives: an
exhausted search, a trivial game and a repeat.  It holds at every game the prover has proven; past those, it is not yet
certified.

### What a game had from n - 1

A line says what it had from the game of n − 1 in its `prev`.  In `options.pass` it says which pass over the games
made it: a sweep goes over them more than once, and a run with no `--pass` has none.

| `prev` | what the game had from the game of n − 1 | what its proof can be |
|---|---|---|
| `null` | nothing: it was solved standalone (a block's first game, or the game before it is missing), with no seed and no ceiling (`setup.cap` null) | `trivial`, `exhausted`, `same_search` on a completed game of the file, or `known` |
| `source` `run` or `results`, `proof` null | the best game of an unfinished search of n − 1, as a seed, and no more: no ceiling (`setup.cap` null), no prime formula, nothing that rests on it | the same: never `cap` or `prime_formula` |
| `proof` given (or `source` `optimal`, `strict`, `bounds`) | n − 1 proven (or assumed, from `optimal.json`; or the committed chain's or the bounds store's game, `--use-prev`): the seed, the ceiling n + score(n − 1), the prime formula for a prime n, and repeats | any |

The game of n − 1 that a `results` line names is in the same file.  It is a completed game or, with no proof, an earlier
incomplete line with the score and moves it records.  A record of the proof `known` (where the run took its copy from a
known source) stands on the file's completed game with that score, those moves and those assumptions, whatever its own
proof ([runbook/chain.md, "Verifying, and bringing the results
home"](runbook/chain.md#verifying-and-bringing-the-results-home)).  A game that a retry searched again has several
lines: an incomplete one made when the game of n − 1 had less to give, or was missing, and a later one made when it had
more.  Each says what it had.

So a file of a sweep, with its games that only an unfinished search is recorded for, reads this way:

* A completed game is proven by its `proof`, given what its `assumes` says.  A `cap` or `prime_formula` rests on a
  proven `prev`, a `same_search` on the game it repeats, and the others on nothing of n − 1.
* An unfinished game is a lower bound.
* Where a game and the one before it are both completed, the chain bounds hold between them.
