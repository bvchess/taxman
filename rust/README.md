# Taxman: the exact solver in Rust

`rust/` is a Cargo crate (a library and one binary, `taxman`; no third-party dependencies) that finds optimal games of
Taxman.  It reimplements the algorithm of the Java solver in the repository root (`src/main/java`) with a search built
for speed and for many cores, and it has found optimal games for every n up to 1300 (the Java's table stops at 1000).

This page is the user guide: what the solver computes and what "optimal" means here, how to build and run it, how to
read its output, why its answers can be trusted (and where they rest on something unproven), the project's vocabulary,
and where to start reading the code.  The design is in [DESIGN.md](DESIGN.md) and the measurements behind it in
[docs/EXPERIMENTS.md](docs/EXPERIMENTS.md); this page links there instead of repeating them.

* [1. What it solves](#1-what-it-solves)
* [2. Build, test, run](#2-build-test-run)
* [3. Commands](#3-commands)
* [4. Reading the output](#4-reading-the-output)
* [5. Why the answers can be trusted](#5-why-the-answers-can-be-trusted)
* [6. The pipeline](#6-the-pipeline)
* [7. Glossary](#7-glossary)
* [8. Reading order and map of the documents](#8-reading-order-and-map-of-the-documents)

## 1. What it solves

### The game

The pot holds the numbers 1 to n.  The player takes a number c that still has at least one proper divisor in the pot;
the player scores c, and the taxman takes c and every one of its divisors that is still in the pot.  The game ends
when no number in the pot has a divisor left in the pot; the taxman sweeps whatever remains.  An *optimal game*
scores the most.  The best score for each n is the Taxman sequence ([OEIS A019312](https://oeis.org/A019312)).  For
n = 4 the optimal game is 3, 4 (the taxman takes 1, then 2) and scores 7.

Finding an optimal game is a search over an enormous tree.  The solver's results are the optimal games, as sequences of
moves with their scores, for every n from 1 to 1300: `results/strict.jsonl` is a strict chain (section 3) from n = 1,
whose games 1 to 1000 reproduce the Java's table and whose games 1001 to 1300 go beyond it.  The code can attempt
any larger n up to 4095, the largest board the set widths cover.  The cost of a game grows by a factor of ten for every
55 to 70 in n past 1200, so what can be reached is a question of machine time
([bench/curve/README.md](bench/curve/README.md)).

### How the solver sees the game (the frame model)

1. Every number above n/2 divides nothing else on the board, so it can only be a move, never tax: these are the
   *sources* (played unless the hive finds them impossible, see the glossary).  Every other number is a *factor*: it
   can be tax for a larger number, and sometimes a move itself.
2. A move m is paid for by one of its *maximal factors* m/p (p a prime dividing m); a factor can pay for one move.
   Maximal factors are enough although the game takes *every* divisor of a move: if some proper divisor d of m is
   still in the pot, so is every f with d | f | m other than m itself (whatever had removed f would have removed d
   too), and one of those f is a maximal factor of m.  So m is playable exactly when one of its maximal factors is
   in the pot, and the model needs nothing else of divisibility (the *lifting* argument of
   [src/numtheory.rs](src/numtheory.rs) and Theorem 1 of [../heuristics/THEORY.md](../heuristics/THEORY.md)).
3. Sources and the factors they compete for fall into independent groups, the *hives*.  One hive is *downstream* of
   another (the other is *upstream*) when the other's numbers are multiples of its own.  Hives that have factors to
   spare are stacked into levels, bottom up: the first level is every such hive with none downstream of it, the next
   is every one directly upstream of those, and so on.  A level is merged into one *frame*; frame 0 is the bottom,
   the downstream end.  Each frame is a small game, *Taxman Mini*: a set of moves and a set of factors, in which the
   moves must be playable in some order.
4. What a game can choose is which of the spare factors to *promote*: to play it as a move instead of leaving it as
   tax.  Promoting a factor of frame h makes it a move of frame h-1: its own maximal factors, which would pay for it,
   are in the hives directly downstream of its hive, and the levels put the free ones in frame h-1 (that nothing
   further down is needed is an assumption of the model, see "What optimal means" below).  Both frames must stay
   winnable.
5. The score of a game is `in_the_bag` (what the game with no promotions scores) plus the sum of its promotions.  So
   the solver searches for the *promotion set* with the largest sum, with a branch-and-bound over which candidates to
   promote; the moves of the game follow from the promotions (`Apiary`, see the pipeline below).

A small example, the board 1..=85 (`frames::tests::frames_of_85`; it comes from the Java tests):

```
frame 0   moves 45 50 63 75    factors 4 6 9 10 14 15 21 25    4 freedoms (factors minus moves)
frame 1   moves 56 60 84       factors 8 12 20 28 30 42        3 freedoms
frame 2   moves 48 72 80       factors 16 24 36 40             1 freedom
candidates: 8 12 16 20 24 28 30 36 40 42
```

Promoting 8 (a factor of frame 1) takes it out of frame 1's factors and makes it a move of frame 0.  Frame 0 stays
winnable.  The frames are then *reduced*: 8's only maximal factor is 4, so (8, 4) is a forced pair and drops out of
frame 0; in frame 1, 56 has lost the factor 8 and is left with 28, so (56, 28) drops out there.  Frame 0 ends with 3
freedoms and frame 1 with 2: one freedom spent on each side.  In the game, 8 is now played as a move that pays its tax
from frame 0 (the factor 4) and scores 8.  The search is over such choices: n = 85 has 10 candidates, n = 1100 has 273.

### What "optimal" means

* **Within the frame model.**  The decomposition above rests on assumptions that are validated, not proven: the greedy
  removal of impossible sources inside hives, the mandatory moves of `unzip` (a source with only one usable factor
  must take it), `free factors = factors - sources`, closed hives (those with no free factor) taking no part in the
  search, and a promoted number paying tax only from the frame directly below
  ([DESIGN.md, Tactics](DESIGN.md#5-tactics)).  They were validated by agreement with the Java's
  `optimal.json` for n <= 1000, by exhaustive search (`src/brute.rs`, `heuristics/evaluation/certify.py`) for small
  boards (n <= 62), and by a model-free exact search ([`evidence/`](evidence/README.md)) that confirms the optimum
  for every n <= 989.
* **Given the pruning is exact.**  The default search tactics are sound: they never prune a promotion set that could
  win ([DESIGN.md, Tactics](DESIGN.md#5-tactics) grades each).  Two tactics are on by default for speed but are
  *unproven*: `reuse_prev` and `revise_freedoms`.  A run with both turned off is called *strict* in this
  repository, and a strict result rests on nothing unproven but the frame model and the correctness of the code.
* **Given the previous game is optimal.**  A game of n is built from the game of n - 1: its search starts from the
  previous game's promotions and stops as soon as it reaches the proven ceiling `n + score(n-1)`.  So a chain of
  games from n = 1 is proven game by game from the one before.  Section 5 describes how each result records what it
  rests on.

## 2. Build, test, run

Requirements: Linux or macOS (the engine uses POSIX signals and `flock`); a Rust toolchain of at least 1.89
(`rust-version` in `Cargo.toml`; the engine uses `File::try_lock`); a C linker.  There are no crates to download.
Python 3 (standard library only) for `tools/check_results.py`.

```
cd rust
cargo build --release              # under a minute on an M1 Max; the binary is target/release/taxman
cargo build --release --examples   # also the ladder (scaling and progress harness), the probe and the sacrifice study
./target/release/taxman help
```

The build is long for a program of this size because the search is compiled once for every set width
([DESIGN.md](DESIGN.md#3-architecture), "Width dispatch").

The binary finds `optimal.json` (the Java's table of optimal games for n <= 1000) at run time: `$TAXMAN_OPTIMAL` if that
is set, else `src/main/resources/optimal.json` of the repository, which it looks for above the working directory and, if
it is not there, above the binary.  So it works from any directory of a checkout, and from a copy of the binary while
the working directory is in one; elsewhere give `--optimal FILE` (or `--optimal none` to compare with nothing), which
is what the error says.  `taxman bench` finds its two files in the repository the same way (below).

The release profile uses fat LTO and `panic = "abort"`; the `test` profile is optimised (`opt-level = 2`) because the
tests replay real games.  A build is generic for its target architecture, so a binary runs on any machine of that
architecture; tuning for the build machine's CPU is opt-in, `RUSTFLAGS="-C target-cpu=native" cargo build --release`,
for a machine that will run only that binary (the cloud scripts do this, see
[tools/cloud/RUNBOOK.md](tools/cloud/RUNBOOK.md)).  It gains nothing on the Apple M1, where it was measured; it is
unmeasured elsewhere (the generic x86-64 target has no hardware popcount, so measure there).  `tools/pgo.sh` makes a
profile-guided build in `target/pgo/release` (0-2% fewer cycles on the M1, the same answers and trees; its header says
how, and DESIGN.md "Performance" what it gains); `examples/perf.rs` measures the hot paths.

```
cargo test                         # the whole suite: about 1.5 minutes to compile, then about 30 s to run
```

`cargo test` runs as many test threads as the machine has cores; the timings here and in DESIGN.md are with
`-- --test-threads=2`.

The ignored tests are for large machines and long runs: `cargo test --release --test engine -- --ignored
many_threads` (64 and 192 worker threads), `TXENGINE_STRESS=2000 cargo test --release --test engine -- --ignored
stress` (random engine configurations through resumes), `TXENGINE_CONSERVATION=500 cargo test --release --test
robustness conservation`, and the benchmarks under `bench` in `src/` and `tests/engine.rs`.  What the suite checks is
listed under "Testing" in [DESIGN.md](DESIGN.md#9-testing); `tests/tripwires.rs` asserts exact node counts, so a change
that alters the search tree on purpose fails it (DESIGN.md says how to update the counts).

## 3. Commands

`taxman help` lists everything; this section shows each command with output from this build.  A tactic is one
switchable piece of the algorithm; `--tactic NAME=VALUE` is repeatable, and the names, values and defaults are in
`taxman help` and [DESIGN.md](DESIGN.md#5-tactics).  Two of the tactics are on by default for speed but unproven
(`reuse_prev` and `revise_freedoms`); a run with both off is called *strict*, and `--strict` (in `solve`, `regress` and
`bench`) is shorthand for `--tactic reuse_prev=off --tactic revise_freedoms=off`.  A `--tactic` given as well is applied
after it.  The other flags of `solve` are in the table at the end of this section.

### Solve one game

```
$ taxman solve 480 --show
n=480   score=73120     search  nodes=385             0.010 s  optimal ok  proof=exhausted
  promotions: [68, 138, 147, 153, 168, 175, 180, 182, 190, 195, 196, 198, 200, 207, 208, 210, 220, 222, 224, ...]
  moves: [479, 361, 437, 299, 403, 341, 473, 301, 469, 335, 445, 267, 471, 314, 478, 466, 458, 454, 446, ...]
```

A single game takes its predecessor from `optimal.json` (n - 1 <= 1000), which seeds and caps the search.  The node
count of a multi-threaded search varies a little from run to run.  "optimal ok" means the score equals the Java's
table, not that it is proven (section 5).

### Solve a range

```
$ taxman solve 1-12 --show
n=1     score=0         trivial nodes=0               0.000 s  optimal ok  proof=trivial
  promotions: []
  moves: []
n=2     score=2         prime   nodes=0               0.000 s  optimal ok  proof=prime_formula
  ...
n=12    score=50        trivial nodes=0               0.000 s  optimal ok  proof=trivial
```

Each game is built from the one before it.  With `--out FILE`, each result is appended to the file (one JSON line per
game, flushed and synced) as soon as it is done, and a rerun skips every game the file already completed, whatever the
tactics of the rerun (the file's lines keep the tactics and taint they were made with):

```
$ taxman solve 1-300 --strict --out mine.jsonl --checkpoint-dir ckpt
$ taxman solve 1-300 --out mine.jsonl
skipped 300 games already done in mine.jsonl
```

### Past n = 1000

`optimal.json` stops at 1000.  A game above 1000 needs its predecessor in the results file, so copy one that has it and
extend it; the committed files are `results/strict.jsonl` (1-1300, strict) and `bench/curve/chain.jsonl` (1001-1231,
strict).  The copy matters: `--out` appends to the file and `results/` must not be overwritten.

```
$ cp results/strict.jsonl mine.jsonl
$ taxman solve 1297-1302 --threads 2 --strict \
      --out mine.jsonl --checkpoint-dir ckpt --time-limit 8 --progress 4
n=1301  score=532272    prime   nodes=0               0.015 s  optimal unknown  proof=prime_formula
n=1302 4.0s nodes 914429 (228.4k/s) best 41423 queue 8 spilled 0 | eff 100.0% idle 0.0% lock 0.0% splits 1 steals 1 full 3.1ms
n=1302 8.0s nodes 1814893 (stopped) best 41423 queue 52 spilled 0 | eff 99.9% idle 0.0% lock 0.0% ... pauses 1 (0.1%) ...
n=1302  score=533546    search  nodes=1814893         8.164 s  incomplete
skipped 4 games already done in mine.jsonl
stopped: the search for n=1302 hit a limit before it finished; run the same command to resume it
```

Games 1297-1300 were skipped, game 1301 (a prime) took the prime formula, game 1302 started from 1301, and the time
limit stopped it with its checkpoint.  A game
whose predecessor is in neither the results file nor `optimal.json` (for example `taxman solve 1100` without `--out`)
runs *standalone*, with no seed and no ceiling; `solve` says so on stderr (`note: no previous game for n=...`) and the
result line has `"prev": null`.  That is still correct, but costs about twice as much on the hard games, and many times
more on games that a chain settles at once by reaching the ceiling ([bench/curve/README.md](bench/curve/README.md)).

### Strict mode, from n = 1

```
$ taxman solve 1-1300 --strict --out strict.jsonl --checkpoint-dir ckpt --checkpoint-interval 60 --progress 60
```

Starting at n = 1, no game rests on `optimal.json` (it is only compared with, up to 1000: a mismatch is a hard error),
so every proof in the file rests on nothing unproven.  The committed `results/strict.jsonl` is such a run
([results/README.md](results/README.md) has the details of it), and this command reproduces its scores.  A strict run
that starts from an `optimal.json` game or a result with an unproven taint refuses to, unless given
`--allow-tainted-prev`, and then marks the results that rest on it.  The default tactics (both unproven ones on) are
faster and good for exploring; a result made with them says so, as the n = 992 line in section 4 does: `(rests on
revise_freedoms)`.

### Checkpoints, resuming, signals

With `--checkpoint-dir DIR`, a game's search writes `DIR/nNNNN.ckpt` every `--checkpoint-interval` seconds (default 60),
atomically and synced, and on any stop.  The same command resumes it:

```
$ taxman solve 992 --threads 2 --time-limit 3 --progress 1 --out r992.jsonl --checkpoint-dir c992
n=992 3.0s nodes 576381 (stopped) best 24144 queue 30 spilled 0 | eff 99.5% ... pauses 1 (0.4%) max 13.1ms park 0.1ms
n=992   score=310644    search  nodes=576381          3.097 s  incomplete
stopped: the search for n=992 hit a limit before it finished; run the same command to resume it
$ taxman solve 992 --threads 2 --out r992.jsonl --checkpoint-dir c992      # resumes (here a SIGTERM stopped it again)
$ taxman solve 992 --threads 2 --out r992.jsonl --checkpoint-dir c992      # resumes and finishes
n=992   score=310690    search  nodes=6175330        30.925 s  optimal ok  proof=exhausted (rests on revise_freedoms)
```

The last line is the third invocation of the game: its `nodes` cover all three, its seconds only the last.  The
results file now holds two incomplete lines and one completed one for n = 992, and `taxman verify-results r992.jsonl`
counts them.

* **Signals.**  SIGINT or SIGTERM stops the search like a limit does: a final checkpoint, an incomplete result line, and
  exit status 130 or 143 (128 + the signal).  Nothing is lost.  A second signal kills the process; SIGKILL, a crash or
  a power loss loses at most one checkpoint interval.
* **One process per checkpoint directory** (a `taxman.lock` in it, taken with `flock`; a second process is refused,
  naming the first), **and per results file** (`--out` is locked the same way while the run appends to it).  Keep the
  directory on a local disk that survives a restart, not on tmpfs.
* **A checkpoint belongs to its game and its search tactics.**  One made for another game, by a build with a
  different frame builder, or with other search tactics is refused, saying what differs; resume with the same
  `--tactic` flags, or delete the file.
* **Limits.**  `--time-limit S` and `--node-limit N` bound one game's search in one invocation (not the whole
  range).  A game stopped by a limit gets an incomplete result line (`completed` false, `best_so_far`), and the range
  stops there, exit status 0.
* **The result is recorded before the checkpoint is deleted**, so a crash never loses a finished game; at worst the
  checkpoint of a finished game is left over, and the next run deletes it.
* **Memory.**  Searcher state is bounded; the shared queue of work units spills to `DIR/spill` beyond `--mem-cap` units
  (default 1048576).  The committed runs never got near it: their queue peaked at 313 units.

### All flags of `solve`

| flag | meaning |
|---|---|
| `--threads K` | worker threads (default: all cores; honours affinity masks and cgroup limits) |
| `--tactic NAME=VALUE` | set a tactic, repeatable |
| `--strict` | no unproven tactic: `--tactic reuse_prev=off --tactic revise_freedoms=off` (a `--tactic` is applied after it) |
| `--out FILE` | append result lines; skip games the file completed; the predecessor of the first game comes from it |
| `--checkpoint-dir DIR`, `--checkpoint-interval S` | checkpoints (default every 60 s) |
| `--progress S` | seconds between progress lines on stderr (default 60; 0: none) |
| `--time-limit S`, `--node-limit N` | stop one game's search, keeping its checkpoint |
| `--mem-cap UNITS` | work units kept in memory before spilling to disk |
| `--force-search` | search where the previous game would give the answer (skips `prime_shortcut`, `reuse_prev`) |
| `--allow-tainted-prev` | with no unproven tactic on, still build on a predecessor whose proof rests on something unproven |
| `--optimal FILE` | `optimal.json` to compare with and take predecessors from (`none`: no file) |
| `--show` | print each game's promotions and moves |

### The other commands

```
$ taxman regress --to 400                       # solve 1..400 in memory and check every score against optimal.json
solved 1..=400 (2 threads, tactics default) in 0.4 s
by method: trivial 13, prime 78, reuse 200, search 109
games 1..=200 derived without search: 146
400 games match optimal.json, 0 do not, 0 are not in it
```

`regress` (default `--from 1 --to 400`; `--tactic` flags apply) is the quick end-to-end check that the build still
gives the Java's answers.

```
$ taxman verify-optimal                         # replay every game of optimal.json (1000 games, ~75 ms)
verified 1000 games (n up to 1000, 217199 moves)
$ taxman verify-results results/strict.jsonl bench/curve/chain.jsonl     # see section 5
$ taxman tables 85 8 45                         # the maximal factors and composites of 8 and 45 on the board 1..=85
8: maximal factors [4]; maximal composites [16, 24, 40, 56]
45: maximal factors [9, 15]; maximal composites []
$ taxman bench --history /tmp/h.jsonl           # time the benchmark games 936 ... 1184 (standalone, no derivation)
```

`bench` appends one JSON line per run to its history file, `rust/bench/history.jsonl` of the repository unless
`--history FILE` names another (pass it to keep the checkout clean), and takes the games past `optimal.json`'s from
`rust/results/strict.jsonl` unless `--known FILE` names another results file.  The ladder is the harness for
hard games and for scaling over thread counts: `cargo run --release --example ladder -- --scaling --seed-opt --known
bench/curve/chain.jsonl --tactic revise_freedoms=off --seconds 600 1215` (its flags are documented at the top of
`examples/ladder.rs`).

## 4. Reading the output

### The summary line (stdout, one per game)

```
n=250   score=19533     search  nodes=34   0.003 s  optimal ok  proof=exhausted
n=992   score=310690    search  nodes=6175330   30.925 s  optimal ok  proof=exhausted (rests on revise_freedoms)
n=1220  score=468662    search  nodes=1375952   8.209 s  incomplete
n=1100  score=381834    search  nodes=56664   0.420 s  optimal unknown  proof=exhausted
```

Board size; score (for an incomplete game, the best found so far); the method (`trivial`, `prime`, `reuse`, `search`);
search nodes (steps, see the glossary); seconds for the game in this invocation; how it compares with `optimal.json`
(`optimal ok`: equal; `OPTIMAL MISMATCH`: a hard error; `optimal unknown`: n above 1000 or `--optimal none`;
`incomplete`); the proof kind; and, if the proof rests on something unproven, which.

### The progress line (stderr, every `--progress` seconds and at the end)

```
n=992 3.0s nodes 576381 (stopped) best 24144 queue 30 spilled 0 | eff 99.5% idle 0.0% lock 0.0% splits 1 steals 1 full 2.8ms pauses 1 (0.4%) max 13.1ms park 0.1ms
```

| field | meaning |
|---|---|
| `n=992 3.0s` | the game and the time its search has run over all invocations (`1m05s`, `1h02m03s` later) |
| `nodes 576381 (stopped)` | steps taken; in the middle of a run the rate (`164.6k/s`), at the end `done`, `optimal`, or `stopped` |
| `best 24144` | the incumbent: the largest *promotion sum* so far (the score is `in_the_bag` plus this) |
| `queue 30 spilled 0` | work units in the shared in-memory deque, and spilled to disk |
| `eff` | effective parallelism: time workers spent searching over wall time times threads |
| `idle`, `lock` | shares of that time spent waiting for work, and waiting for the deque's lock |
| `splits steals` | times a worker handed work to idle ones, and units taken from the front of the deque |
| `full 2.8ms` | time until every worker first ran at once |
| `pauses 1 (0.4%) max 13.1ms park 0.1ms` | checkpoints (each parks all workers): their share of worker time, the longest pause, the longest wait for the workers to park |

### A results line (`--out`, schema 3)

One JSON object per line, compact.  Shown here for n = 250 (a real line, from a strict run of 1-300), with the moves
cut short, the fields regrouped and comments added, so it is not valid JSON as printed:

```
{"schema":3, "n":250, "score":19533, "moves":[241,169,221,...], "promotions":[84,98,104,105,110,112,114,117,120,124,125],
 "method":"search", "completed":true,
 "proof":"exhausted", "reached_cap":false, "taint":[],
 "in_the_bag":18320,                      # score = in_the_bag + sum(promotions): 18320 + 1213 = 19533
 "prev":{"n":249,"score":19382,"source":"run","moves_digest":"44877b44d9bbf51a","proof":"exhausted","taint":[]},
 "nodes":34, "stats":{"promote_attempts":20,"promotions":36,"upper_bounds":0,"can_promotes":0,"children":13,
                      "precluded":0,"certificates":0,"match_bounds":28},
 "engine":{"splits":0,"checkpoints":0,"spilled_units":0,"max_queue":1},
 "setup":{"frames":4,"candidates":48,"max_total":11,"greedy_sum":1194,"cap":1312,"revise":"Off"},
 "seconds":{"frames":0.0006,"preclusions":0.0,"greedy":0.00002,"revise":0.0,"search":0.0015,"search_all_runs":0.0015,"total":0.0031},
 "invocations":1, "threads":2,
 "tactics":"reuse_prev=off,revise_freedoms=off",
 "tactic_switches":{"prime_shortcut":true,"reuse_prev":false,"revise_freedoms":false,"preclusions":false,"match_bound":true,
                    "branching":"conflict-strong","match_refine":1,"fix_forced":true,"nogoods":true,"forget_nogoods":true,
                    "legacy_bound":false,"prev_cap":true,"seed_incumbent":true},
 "unproven_tactics_on":[], "limits":{"seconds":null,"nodes":null},
 "options":{"force_search":false,"allow_tainted_prev":false,"allow_optimal_mismatch":false,"optimal_file":"default"},
 "optimal_match":true, "prev_source":"run", "build":{"commit":"c759019776dd...","dirty":false}}
```

| field | meaning |
|---|---|
| `schema`, `n` | the format of the line (3; 2 differed only in the taint of a proof by the cap) and the board size |
| `score`, `moves`, `best_so_far` | the game and its score; for an unfinished search `score` is null and `best_so_far` (a key only such a line has) has the best so far |
| `promotions` | the promoted numbers, ascending |
| `method` | `trivial`, `prime`, `reuse`, `search` (how the game was found) |
| `completed` | false only if a limit or a signal stopped the search; such a line is an unfinished-search record, and its checkpoint is kept |
| `proof` | how optimality is established: `trivial`, `prime_formula`, `cap`, `exhausted`, `reuse_prev` (section 5); null if incomplete |
| `reached_cap` | the search stopped because its incumbent reached `cap` |
| `taint` | the unproven things the proof rests on, here or in the games of n - 1 it rests on; `[]` is clean |
| `in_the_bag` | what the game with no promotions scores (null for prime shortcuts) |
| `prev` | the game of n - 1 this one was built from: score, where it came from (`run`, `results`, `optimal`), a digest of its moves, its proof and taint; null if there was none |
| `nodes`, `stats` | search steps over all invocations, and the searcher's counters (`match_bounds` is the number of matching bounds computed) |
| `engine` | this invocation's splits, checkpoints written, units spilled, largest in-memory queue |
| `setup` | null on a `trivial`, `prime` or `reuse` line, which set up no search; otherwise the frames and candidates found, `max_total` (cap on the number of promotions), the greedy start, `cap` (ceiling on the promotion sum, `n + score(n-1) - in_the_bag`), and what `revise_freedoms` did in `revise` (`Off`, `NotNeeded`, `GreedyIsMax(k)`, `Unchanged`, `Searched{..}`, `Stopped{..}` when a limit stopped its own search; for a resumed search with the tactic on, `FromCheckpoint` if the checkpoint's frame 0 has revised freedoms, fewer than built, and `ResumedAsBuilt` if it has the built ones) |
| `seconds` | per phase, this invocation (`search_all_runs`: the search over every invocation of a resumed game) |
| `invocations` | how many invocations ran the search (a resumed game has more than one) |
| `threads`, `tactics`, `tactic_switches`, `unproven_tactics_on`, `limits`, `options` | what the last invocation ran with; `tactics` is `default` or the changes from the defaults |
| `optimal_match` | the score equals `optimal.json`'s (null above 1000) |
| `prev_source` | as `prev.source` |
| `build` | the commit the binary was built from, and whether its sources differed from it |

The committed `results/strict.jsonl` is schema 3, made by commit bd9bb5d.  `results/default.jsonl` and
`bench/curve/chain.jsonl` are schema 1, kept as they were made (the first run past 1000, and the cost-curve study's
chain): no `schema`, `proof`, `taint`, `prev`, `in_the_bag` or `build`; their provenance is derived from the fields
they have, by the same rules.  [results/README.md](results/README.md) says what is known about each file.  A line is
never rewritten: a resumed game that is stopped again adds another incomplete line, and the completed line comes last.

## 5. Why the answers can be trusted

### Proof kinds

Each completed game records how its optimality is established (`proof`):

| proof | meaning | rests on the game of n - 1? |
|---|---|---|
| `trivial` | the board has no promotion candidates; the game with no promotions is the only one | no |
| `exhausted` | the search explored its whole tree | no |
| `cap` | the search reached `n + score(n-1)`, which no game of n can exceed (removing the move n from an optimal game of n leaves a game of n - 1) | yes |
| `prime_formula` | a prime n plays the game of n - 1 with its first move replaced by n; optimal if the game of n - 1 is (proved in `src/accelerate.rs`, at `prime_shortcut`) | yes |
| `reuse_prev` | the tactic of that name accepted a game derived from the previous one: a conjecture | yes |

### Taint

The *taint* of a game is the set of unproven things its proof rests on: the unproven tactics it used itself
(`reuse_prev` if it derived the game; `revise_freedoms` if frame 0's freedoms came from it; `legacy_bound`, which is
known to be unsound), plus, if its proof rests on the game of n - 1, that game's taint, and so on down the chain.  A
proof by the cap rests on nothing the game used itself: the game replays to `n + score(n-1)`, the ceiling, whatever
found it, so its taint is only that of the game of n - 1 (schema 2 lines counted its own tactics too).  A
game taken from `optimal.json` has the taint `optimal.json` (the Java made that file with both unproven tactics on and
records nothing about which games used them).  An empty taint means: proven within the frame model, given that the
solver is right.  `taxman solve` prints the taint in the summary line, and a strict run refuses a predecessor with
one.

### The verifiers

* Every `solve` replays each game through `game::replay_finished`, an independent referee that shares no code with the
  solver's number theory (legal, over, opens with the largest prime, scores what is recorded), and checks the arithmetic
  and the ceilings `score(n-1) <= score(n) <= n + score(n-1)` before the game counts.  A game that fails after it was
  found goes to `<results>.rejected.jsonl` and the run fails.
* `taxman verify-results FILE...` reads results files without changing them: every game replayed, the chain from n - 1,
  each line's arithmetic and provenance, that its fields agree with each other as the solver writes them (so that a
  hand edit cannot drop a taint or change a proof), that the files agree, and a count of games by proof kind and by
  taint.  Opening a results file for `--out` runs the same checks.
* `python3 tools/check_results.py [FILE...]` is an independent referee in Python (standard library only, no shared
  code).  It re-derives every proof and taint, and also checks each score against the Franklín-Moniot upper bound and
  the best-known heuristic chain in `heuristics/results`.  With no arguments it checks `results/*.jsonl`.

```
$ taxman verify-results results/strict.jsonl bench/curve/chain.jsonl
results/strict.jsonl: 1300 completed games, n = 1..=1300 (0 missing), 4 incomplete lines
  proof: trivial 13, prime_formula 211, cap 415, exhausted 661
  taint: 1300 of 1300 games rest on nothing unproven
  scores at a proven ceiling, given the game of n-1: n + score(n-1) 424, prime formula 210
bench/curve/chain.jsonl: 231 completed games, n = 1001..=1231 (0 missing), 0 incomplete lines
  proof: prime_formula 34, cap 70, exhausted 127
  taint: 231 of 231 games rest on nothing unproven
  scores at a proven ceiling, given the game of n-1: n + score(n-1) 70, prime formula 34
231 board sizes are in more than one file, with equal scores in all
```

What the checks cannot catch is a game that is legal, finished and within the chain bounds but not optimal when the game
of n - 1 is not optimal either; that is what the proof kinds and the taint track, and why a chain from n = 1 matters.
[results/README.md](results/README.md) says what supports each committed score (re-solves from scratch, a second build's
chain, taint counts).

## 6. The pipeline

```
 taxman solve A-B --out FILE --checkpoint-dir DIR
   |  solver::solve_range_each        n = A..B in order; the game of n-1 comes from this run, FILE or optimal.json,
   |                                  and is checked (legal, over, opens with the largest prime) before anything uses it
   v
 solver::solve_game(n) -> solve_with_width::<W>      W = 8, 16, 20, 24, 28, 32 or 64 words of 64 bits (bits::WIDTHS), by n
   |
   |  [1] numtheory::Tables            maximal factors and maximal composites of 1..=n; the set width W
   |  [2] n prime ............ accelerate::prime_shortcut   the game of n-1 with its first move replaced by n
   |                                                         method=prime        proof=prime_formula
   |  [3] board::Board::new            hives::Apiary: sources (never tax) + factors -> hives; unzip, removeImpossible
   |                                   frames::build_frames: free hives stacked into levels = frames (0 = bottom);
   |                                   candidates; in_the_bag
   |      no candidates ...... the game with no promotions   method=trivial      proof=trivial
   |  [4] reuse_prev (unproven) accelerate::reuse_prev        method=reuse        proof=reuse_prev
   |  [5] search ............. method=search                 proof=exhausted | cap
   |        Board::prepare             search::prepare: model::Model::build -> the root FrameSet; greedy set;
   |                                   revise_freedoms -> max_total
   |        search::search             starting incumbent = best of greedy / seed (n-1's promotions) / rounded root
   |                                   matching + local search; cap = n + score(n-1) - in_the_bag
   |          engine::run              worker threads; shared deque with disk spill; incumbent; checkpoints; signals
   |            Stepper::step          per node (search/searcher.rs): check_node = matching bound (matchbound) ->
   |                                   conflicts, forced members, nogoods (search::refine::Refiner, model::Nogoods);
   |                                   choose_branch (a candidate) -> make_child -> check_child
   |      promotions -> accelerate::solution_for_promotions   (Apiary with the promotions as sources) -> ordered moves
   |
   |  [2] and [4] are skipped by --force-search and when the game has a checkpoint (a search is resumed, never replaced)
   v
 [6] solver::conclude                replay (game::replay_finished); score == in_the_bag + promotions; the ceilings;
   |                                 compare with optimal.json; proof and taint
   v
 [7] results::ResultsFile::append    the JSON line, synced; only then is the game's checkpoint deleted;
                                     the game becomes the previous game of n+1
```

The numbers are those of [DESIGN.md](DESIGN.md#7-per-game-pipeline-solver), which gives the Java class each step
replaces, and of the module comment of `src/solver.rs`, which gives the checks of each.

## 7. Glossary

Words that mean more than one thing in this code base are marked **(overloaded)**, with each meaning.

### The game

* **board, pot, n**: the numbers 1..=n; the pot is what is still on it during a game.  In the code `board::Board` is a
  board analysed for the search: its tables, frames, candidates and `in_the_bag`.
* **move, tax**: a number the player takes, and the divisors the taxman takes with it.
* **maximal factor** of m: m/p for a prime p dividing m (a divisor reached by deleting one prime).  Its inverse is the
  **maximal composite**.  A move is playable if one of its maximal factors is still in the pot (equivalent, on a board
  without holes, to "some divisor is": the lifting argument in `src/numtheory.rs`).
* **score, opt(n)**: the sum of the player's moves; the best score on the board 1..=n.
* **hole**: a number taken off the board; used only by the sacrifice study and the brute-force solver.

### The frame model

* **source**: a number with no maximal composite (every number above n/2): never tax, and played unless it is found
  impossible (below).  (Hives call moves *sources*, frames call them *moves*, and the matching code *movers*; same
  thing.)
* **factor**: any number that is not a source.
* **hive**: a connected group of sources together with all the factors they compete for.  The code's bee names
  (**apiary** = all hives of a board, **unzip**, **removeImpossible**) come from the Java.  **downstream / upstream**:
  hive B is downstream of hive A, and A upstream of B, when a number of A has a maximal factor in B (A's numbers are
  multiples of B's).  In a game the downstream hives are played first.
* **unzip**: finds the *mandatory* moves of a hive: a source with one usable factor must take it, and so on.  The
  Java docs say "forced"; see **forced**.  **claimed** factors are the ones those moves use up.
* **impossible**: a source that cannot be played together with the sources already kept (found greedily from the
  largest).  Impossible moves are dropped from the frame's moves.
* **free factor count**: factors minus sources of a hive: how many factors it can spare.  A **free** hive has some; a
  **closed** hive has none and takes no part in the search.  **(overloaded)** *free* is also a hive, a factor, and a
  vertex of the matching that is unmatched.
* **frame, level**: free hives stacked in levels (a level is every free hive with no free hive downstream of it, then
  the next level up, ...) and merged; each level is one frame.  Frame 0 is the bottom (downstream).  A promotion moves a
  factor of frame h to a move of frame h - 1.  "Above", "upstream" and "top" mean larger h; "below", "downstream" and
  "bottom" mean smaller.
* **Taxman Mini**: the game a frame plays: a set of moves and a set of factors; each move must still have one of its
  maximal factors in the frame when taken, and taking it removes all of its factors still there.
* **winnable**: some order takes every move of a frame.
* **peel**: the test for winnable: repeatedly drop a move that has a *private* live factor (one no other move has);
  winnable exactly when all moves drop.  **(overloaded)** `hives::count_freedoms` is the same kind of peel, which also
  takes a move with a single live factor first, for a hive's moves; and `heuristics/core.py` has a `peel` that is a
  different procedure.
* **reduce**: remove forced pairs: a move left with a single live factor must be paid by it, so the pair is dropped,
  repeatedly.  **(forced pair)**
* **freedoms**: a frame's factors minus its moves, which drops by one on each promotion into or out of it.  Frame 0's
  can be lowered by `revise_freedoms`.  The same quantity as the *free factor count* at the start (two names).
* **promotion, promote**: play a factor as a move: take it out of its home frame's factors and make it a move of the
  frame below.  Legal when both frames stay winnable.  **home frame**: the frame a number is a factor of.
* **candidate**: **(overloaded)** (1) a number the frame builder says could be promoted (`FrameColumns::candidates`,
  `Apiary::promotion_candidates`); (2) `cands` in a search node: the candidates the node's loop still has to try; (3) a
  vertex role in the matching graph: a number that may be matched as a promoted mover, may pay for one above it, or stay
  free.
* **in the bag** (`in_the_bag`, the Java's `inTheBag`): what the game with no promotions scores.  The score of any game
  of the model is `in_the_bag` plus the sum of its promotions.
* **max_total** (`maxTotalPromotions`): a cap on the *number* of promotions, from the frames' freedoms.
* **cap, prev_cap** **(overloaded)**: (1) the ceiling on the promotion *sum*: `n + score(n-1) - in_the_bag`; reaching
  it ends the search (tactic `prev_cap`, proof `cap`); (2) `max_total`, "the cap on the number of promotions"; (3) the
  per-frame caps inside the Java bound (`largest_that_fit`); (4) `queue_mem_cap` and `NOGOOD_CAPACITY`, sizes.
* **model assumptions**: see section 1.  `model::Model` is the immutable factor graph and home frames, not "the
  model".
* **prime shortcut**: for a prime n, the game of n - 1 with its first move replaced by n.  **sacrifice**: study code
  (`src/sacrifice.rs`, not used by the solver) with the proven bound `opt(n) <= max(opt(n-1), n + max over maximal
  factors f of opt_M([n-1] minus f))`.

### The search

* **tactic**: a switchable piece of the algorithm (`--tactic`).  **sound / proven / unproven / unsound**: the grading in
  [DESIGN.md](DESIGN.md#5-tactics).  **strict**: no unproven tactic on.
* **incumbent**: the best promotion set found so far, shared by all threads; **offer** raises it.  **greedy**: promote
  every candidate that can be, from the largest down.  **seed**: the previous game's promotions that are still
  candidates.  **dive**: round the root's matching into a feasible set.  The starting incumbent is the best of the
  three, improved by a local search.
* **node, step, unit** **(overloaded)**: a search node is a set of promotions made from the root plus the candidates
  left; a **unit** is a node as the engine passes it around, `(promoted, cands)`, from which the state is rebuilt; and
  the `nodes` that results and progress lines count are *steps*, one per candidate a node's loop tries.
* **upper bound**: what no completion of a node can exceed.  **blameGame** is the Java's (`bounds::Bounds::upper_bound`:
  a bid of the largest candidates that fit the frames' limits, with the first failing member swapped for a runner-up;
  DESIGN.md, section 4), kept for parity; the default is the **matching bound**.
* **matching bound**: relax winnability to matchability: choose promotions maximising their sum such that every move
  and every promoted number gets a distinct maximal factor in its frame.  A maximum-weight bipartite matching, kept
  incrementally with dual **potentials** along the search path (`matchbound`).  **Q\***: the promotions of the
  heaviest matching.
* **conflict**: a subset of a promotion set that cannot be promoted together from a state.  Every completion leaves out
  one of its members.  **core**: the moves of an unwinnable frame that a peel cannot drop; a conflict is built from it.
* **refinement** (`match_refine`): bound the branches of a conflict too (member k1 out; k1 in and k2 out; ...) to a few
  levels.
* **branching**: which candidate a node promotes next.  **strong branching** (`conflict-strong`, the default): pick
  the member of the conflict whose removal lowers the matching bound most.
* **forced member** **(overloaded)**: in the search, a member of a conflict whose removal alone brings the bound below
  what the node needs, so every completion that beats the incumbent promotes it (tactic `fix_forced`: the child promotes
  all of them together).  Also: a hive's *mandatory* move (`unzip`, "forced moves" in its docs), and a *forced pair*
  in `reduce`.  Three unrelated ideas.
* **nogood**: a conflict of the *root* learned from a conflict found at a node; valid at every node, any thread, any
  path.  Each searcher keeps a table, scanned exactly over compact member indices, and forgets it every 50,000 of its
  steps (`forget_nogoods`).
* **preclusion**: a depth-2 table that rules candidates out (tactic `preclusions`, off by default).
* **certificate** **(overloaded)**: `model::Certificate` is a winning order that answers most single-edit winnability
  questions without a peel; `results::Certificate` is a score that equals a proven ceiling; the Python heuristics'
  "upper-delta certificate" is a similar prediction from the previous game's score, defined once, in the comment of
  `reuse_prev` in `src/accelerate.rs`; `certify.py` is the brute-force certification of small boards.
* **revise freedoms** (`revise_freedoms`): lower frame 0's freedoms, hence `max_total`, by solving the two-frame game of
  frames 0 and 1 (unproven: it assumes the max-sum set also has the max count).
* **reuse_prev**: build a game from the previous game's promotions that are still possible and accept it with no search
  if it scores what the previous game suggests (unproven).

### Results and trust

* **proof (kind)**: how a completed game's optimality is established: `trivial`, `exhausted`, `cap`, `prime_formula`,
  `reuse_prev` (section 5).  **ceiling**: `n + score(n-1)`, which no game of n can exceed; a score that reaches it needs
  no search to prove.
* **taint**: the unproven things a proof rests on, inherited down the chain of games (section 5).  **strict / default**:
  a run with no unproven tactic on, and a run with the defaults (both unproven tactics on).
* **optimal ok / optimal match**: the score equals `optimal.json`'s.  It compares with the Java's table; it is not a
  proof, and does not exist above 1000.
* **referee**: code that checks a game or a results file independently of the code that made it: `game::replay`,
  `taxman verify-results`, `tools/check_results.py`.  **rejected**: a game that failed a check after it was found,
  written to `<results>.rejected.jsonl`.
* **schema 1 / 2 / 3**: the formats of a results line (section 4).

### The engine and a run

* **engine**: the problem-independent parallel depth-first branch-and-bound driver (`engine.rs`); the Taxman search is a
  `Searcher` it runs.  **worker**: a thread with its own searcher.  **deque**: the shared queue of units.
* **yield, split, steal**: a searcher gives its unexplored work back by *yielding*; a *split* is a yield answering an
  idle worker; a *steal* takes a unit from the front of the deque (the shallowest, largest subtree).  **poll**: the
  searcher asking whether to yield.
* **spill**: units moved from memory to segment files beyond `queue_mem_cap`.
* **checkpoint, pause, park**: a periodic pause in which all workers yield and park, and the engine writes every unit,
  the incumbent and statistics to `nNNNN.ckpt`.  **header**: what identifies the search (n, width, digests of candidates
  and frames, frame 0's freedoms, `max_total`, the tactics).
* **eff, idle, lock, full occupancy, tiny run**: the engine's counters; see section 4 and the scaling reading guide in
  [tools/cloud/RUNBOOK.md](tools/cloud/RUNBOOK.md).
* **W, `Bits<W>`**: the width in 64-bit words of the bitsets that hold game numbers; chosen from n.
* **standalone vs chained**: a game solved alone (no predecessor: no seed, no cap) or as part of a chain from n - 1
  ([bench/curve/README.md](bench/curve/README.md)).
* **frontier** **(overloaded)**: the largest n solved contiguously (1300).  Unrelated to the "frontier" argument of
  `heuristics/THEORY.md`.
* **ladder**: a harness (`examples/ladder.rs`) that searches hard games standalone, with time limits, at several thread
  counts.  **Java**: the original solver in `src/main/java`; **optimal.json**: its table of optimal games, n <= 1000.

## 8. Reading order and map of the documents

The code, in the order to read it, which is the order of the module map in `src/lib.rs` (each file starts with a module
comment that is worth reading first; section 6 above is the map of how the pieces fit, so keep it open):

1. `src/game.rs`: the rules as a referee, and `optimal.json`.
2. `src/numtheory.rs`: maximal factors and composites.
3. `src/bits.rs`: `Bits<W>`, the fixed-width bitsets of game numbers that everything below is generic over, and the
   set widths.  The module comment is enough at first.
4. `src/json.rs`: the small JSON reader and writer.  Skim; it matters once you read results lines.
5. `src/hives.rs`: sources, factors, hives, unzip, impossible moves, free factor counts.
6. `src/frames.rs`: hives stacked into frames; the candidates.
7. `src/model/`: `mod.rs` says what is where: `Frame`, `FrameSet` (`frame.rs`), `Model`, winnability and reduction
   (`peel.rs`), promotion (`promote.rs`), then conflicts (`conflict.rs`) and nogoods (`nogoods.rs`).
8. `src/accelerate.rs`: the shortcuts from the game of n - 1 (read the comment of `prime_shortcut` for the proof of the
   prime shortcut), and the last step from promotions to moves.
9. `src/matchbound.rs`: the matching bound; the module comment is the best explanation of it.
10. `src/search/`: `mod.rs` explains the step and says what is where: `searcher.rs` (`TaxSearcher` and `Stepper::step`,
    whose phases are `check_node`, `choose_branch`, `make_child` and `check_child`), `refine.rs` (`Refiner`: conflicts,
    refinement; `strongest`), `setup.rs` (`prepare`), `tactics.rs`, `incumbent.rs`, and `header.rs` (the checkpoint
    header).  The `if let Some(p) = &mut self.probe` lines in `searcher.rs` are instrumentation
    (`diagnostics.rs`), which can be skipped.
11. `src/bounds.rs`: the Java's `blameGame` bound and preclusions, mostly off the default path.  The default step does
    use three of its functions (`max_additional`, `greedy`, `new_candidate_list_after`) and the peel scratch space its
    `Bounds` owns; the module comment lists them, and the rest can wait.
12. `src/engine.rs`: the protocol in the module comment (conservation, splitting, checkpoint, termination).
13. `src/tactics.rs`, `src/board.rs`, `src/solver.rs`, `src/results.rs`, `src/paths.rs`: the switchable tactics, a board
    prepared for a search, the per-game pipeline and the range driver, provenance and verification, the default files.
14. `src/main.rs`, `src/cli.rs`, `src/commands.rs`: the command line.
15. Studies, skipped on a first read: `src/brute.rs` (exhaustive search, the test oracle) and `src/sacrifice.rs` (the
    sacrifice bound).

| document | what it holds |
|---|---|
| this page | the user guide |
| [DESIGN.md](DESIGN.md) | the design as it is now: goals, the game and the frame model, the architecture, the search, the tactics and their grades, the engine, the pipeline, the trust model, the tests, performance |
| [docs/EXPERIMENTS.md](docs/EXPERIMENTS.md) | the lab notebook: every round of improvements with its measurements, and the ideas measured and not kept |
| [results/README.md](results/README.md) | the committed results files and what supports their scores |
| [bench/curve/README.md](bench/curve/README.md) | how the cost grows with n beyond the frontier, and projections for a larger machine |
| [tools/cloud/RUNBOOK.md](tools/cloud/RUNBOOK.md) | running the solver on a many-core cloud machine |
| [../heuristics/THEORY.md](../heuristics/THEORY.md) | the theory (maximal factors, playable sets) the model rests on |
| the [wiki](https://github.com/bvchess/taxman/wiki) | the Java solver's algorithm and definitions |
