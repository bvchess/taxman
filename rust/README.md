# Taxman: the frame solver in Rust

`rust/` is a Cargo crate (a library and one binary, `taxman`; no third-party dependencies): the *frame solver*, which
finds optimal games of Taxman.  It reimplements the algorithm of the Java solver in the repository root
(`src/main/java`) with a search built for speed and for many threads, and it has found optimal games for every n of the
committed chain ([results/README.md](results/README.md) gives its range; the Java solver got to 1000).  A second
program, the prover ([prover/](prover/README.md)), checks the frame solver's answers on the whole board, without the
frame model, so far to 2175.  Below, "the solver" is the frame solver.

This page is the user guide: what the frame solver computes and what "optimal" means here, how to build and run it, how
to read its output, why its answers can be trusted (and what they assume), the project's vocabulary,
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

Finding an optimal game is a search over an enormous tree.  The frame solver's results are the optimal games, as
sequences of moves with their scores, for every n of the committed chain: `results/strict.jsonl` (its range is in
[results/README.md](results/README.md)) is a chain from n = 1 (section 3), whose games 1 to 1000 reproduce the Java's
table, whose games 1001 to 1300 go beyond it (the table `optimal.json` has them now too, see `extend-optimal` in
section 3) and whose games from 1301 on are past the table.  Its games 2 to 2175 are also proven on the whole board,
by the prover; its games from 2176 on are proven within the frame model, which is exact given statement U (below,
"What optimal means"), and U is not yet certified past 2175 (`taxman certify` will check it game by game).  The code
can attempt any larger n up to 8191, the largest board the set widths cover.  With the GF(2) bound (the default since
2026-10), a chain from `optimal.json`'s game of 1300 reached 2175 in 92 minutes on two threads of a laptop, and the
committed chain from n = 1 to 2175 in 27 minutes on eight
([results/README.md](results/README.md)), the cost in waves with no growth a fit over those games resolves
([docs/EXPERIMENTS.md](docs/experiments/the-gf2-bound-in-the-search-2026-10.md)); before it, the cost of a game grew by a
factor of ten for every 55 to 70 in n past 1200 ([bench/curve/README.md](bench/curve/README.md)).  Past 2175 it grows:
the search times of the games 2176 to 3440, which a day on one 64-thread cloud machine made for about $16, rise
tenfold per about 1,040 in n, with about a decade of scatter
([docs/EXPERIMENTS.md](docs/experiments/the-frontier-chain-to-3440-2026-10.md)).

### How the solver sees the game (the frame model)

1. Every number above n/2 divides nothing else on the board, so it can only be a move, never tax: these are the
   *sources* (played unless they are found impossible, see the glossary).  Every other number is a *factor*: it
   can be tax for a larger number, and sometimes a move itself.
2. A move m is paid for by one of its *maximal factors* m/p (p a prime dividing m); a factor can pay for one move.
   Maximal factors are enough although the game takes *every* divisor of a move: if some proper divisor d of m is
   still in the pot, so is every f with d | f | m other than m itself (whatever had removed f would have removed d
   too), and one of those f is a maximal factor of m.  So m is playable exactly when one of its maximal factors is
   in the pot, and the model needs nothing else of divisibility (the *lifting* argument of
   [src/numtheory.rs](src/numtheory.rs) and Theorem 1 of [../heuristics/THEORY.md](../heuristics/THEORY.md)).
3. Sources and the factors they compete for fall into independent groups, the *frames*: sources that share a factor are
   in one frame, with all the factors they can be paid with.  Frame X is *in front of* frame Y (and Y *behind* X) when
   a number of Y has a maximal factor in X, that is, when Y's numbers are multiples of X's; the frames in front are
   played first.  Each frame is a small game, *Taxman Mini*: a set of moves and a set of factors, in which the moves
   must be playable in some order.  Setup drops a frame's *impossible* sources, settles the moves a source with one
   usable factor must make, and counts its *freedoms*, its factors minus its moves: how many factors it can spare.  A
   frame with none is *closed* and takes no part in the search.  The others are numbered from the front, by *layer*:
   all the sources of a frame have the same number of prime factors, the frames in front of it have one fewer, and
   frame 0 is the frames with freedoms whose sources have the fewest, frame 1 those with one more, and so on (so frame
   0 is every one that has no other in front of it, frame 1 every one directly behind those, on every board the solver
   supports).  Frames with the same
   number are merged into one, so that a promotion always has one frame to go to.
4. What a game can choose is which of the spare factors to *promote*: to play it as a move instead of leaving it as
   tax.  Promoting a factor of frame h makes it a move of frame h-1, one frame forward: its own maximal factors, which
   would pay for it, are in the frames directly in front of its frame, and the numbering puts those with freedoms in
   frame h-1 (that nothing further forward is needed is proven: see "What optimal means" below).
   Both frames must stay winnable.
5. The score of a game is `in_the_bag` (what the game with no promotions scores) plus the sum of its promotions.  So the
   frame solver searches for the *promotion set* with the largest sum, with a branch-and-bound over which candidates to
   promote; the moves of the game follow from the promotions (`frames::BoardFrames`, see the pipeline below).

A small example, the board 1..=21 (the Java's golden data, `testdata/golden/n0021.json`; the wiki's [walkthrough for
n = 21](https://github.com/bvchess/taxman/wiki/Walkthrough-for-N=21) shows it in pictures):

```
frame A   sources 19 11 13 17   factors 1           closed: 11, 13 and 17 are impossible (the 1 pays for one of them)
frame B   sources 14 15 21      factors 2 3 5 7     1 freedom (15 must take 5)             frame 0
frame C   sources 12 18 20      factors 4 6 9 10    1 freedom (20 must take 10)            frame 1
frame D   sources 16            factors 8           closed
candidates: 4 9 (factors of C that B can accept as moves and still play all its own)
```

B is in front of C, because 9, a factor of C, has the maximal factor 3 in B.  The game with no promotions plays every
source that is not impossible and scores `in_the_bag` = 135 (19 + 14 + 15 + 21 + 12 + 18 + 20 + 16).  Promoting 9 (a
factor of C) takes it out of C's factors and makes it a move of B, so the game scores 144; promoting 4 would score 139.
Both frames stay winnable.  The frames are then *reduced*: 9's only maximal factor is 3, so the pair (9, 3) drops out
of B, which leaves 21 only the factor 7 and 14 only the factor 2, so they drop out too; in C, 18 is left with 6 and
12 with 4.  Both frames end with no freedom: one freedom spent on each side.  The game plays the frames from the
front: 19; 9, 15, 21, 14; 18, 12, 20; 16.  The search is over such choices: n = 21 has 2 candidates, n = 85 has 10,
n = 1100 has 273.

Frames are merged from n = 20 on.  At n = 20 the frames of 14 and of 15 share no factor, but neither has a frame
with freedoms in front of it, so both are frame 0, and they are merged into one frame with two freedoms: both sit
directly in front of the frame of 12, 18 and 20, whose factor 6 has a maximal factor in each (2 and 3), and a
promotion from that frame needs one frame to go to.

### What "optimal" means

* **The frame model: proven, but for one statement.**  The decomposition above used to rest on five assumptions:
  the greedy removal of impossible sources inside frames, the mandatory moves of `unzip` (a source with only one
  usable factor must take it), freedoms = factors - sources, closed frames (those with no freedom) taking no part in
  the search, and a promoted number paying tax only from the frame one forward.  They are now theorems
  ([heuristics/THEORY.md, "The frame model: what is proven and what
  remains"](../heuristics/THEORY.md#the-frame-model-what-is-proven-and-what-remains)), and what is left is one
  statement about the game:

  > **U**: some optimal game plays every source with three or more prime factors that setup keeps (U\*).

  Precisely: the games that play every source setup keeps are exactly U\* with a promotion set added (Theorem M,
  which needs of the numbering that it is *clean*, every frame with freedoms directly in front of frame h being frame
  h - 1, as the numbering by layer is by construction); every
  optimal game plays the sources setup keeps with one or two prime factors (Theorem B); so the heaviest promotion set
  gives opt(n) exactly when U holds at n.  U is open in general.  It holds for every n <= 2175, where [the
  prover](prover/README.md) proves the optimum on the whole board, without the frame model.  So a game proven above
  2175 by an exhausted search, a trivial board or a repeat assumes U at n: the committed games above 2175 proven that
  way (the [results README](results/README.md) counts the proofs) do, and U is not yet certified for them (`taxman
  certify` checks it game by game, section 3).  A game proven at its ceiling or by the prime formula does not (section
  5).  The results lines do not record it.
  [DESIGN.md](DESIGN.md#what-optimal-means) says what each proof kind needs of the model.
* **Given the pruning is exact.**  Every tactic is proven or sound: none prunes a promotion set that could win
  ([DESIGN.md, Tactics](DESIGN.md#5-tactics) grades each).  So a result rests on nothing but U (where its proof uses
  the frame model), the correctness of the code and, where its proof says so, the game it was built from.
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
cargo build --release              # about 100 seconds on an M1 Max; the binary is target/release/taxman
cargo build --release --examples   # also the ladder (scaling and progress harness), the probe and the ledger
cargo build --release --examples --features studies   # and the study tools: the tuning benchmark, the census of
                                   # the frames, the extended problem's chain and the wall regime's measurements
./target/release/taxman help
```

The study tools (the examples `tune`, `extended`, `structure` and `wall`) are behind the feature `studies`, which is
off by default, so that a build, `cargo test` and `cargo clippy --all-targets` do not compile them; a command that names
one (`cargo build --release --example wall`) needs `--features studies` too.

The build is long for a program of this size because the search is compiled once for every set width
([DESIGN.md](DESIGN.md#3-architecture), "Width dispatch").

The binary carries `optimal.json` (the table of optimal games, n <= 1300: the Java's games to 1000, then those of the
chain from n = 1) built in: `src/main/resources/optimal.json` as it was when the binary was compiled.  Every run
confirms its scores against that table, wherever the binary runs, and no option names another; a changed table takes a
rebuild, which cargo does by itself when the file has changed.  `taxman bench` finds its two files in the repository
(below), above the working directory and, if they are not there, above the binary.

The release profile uses fat LTO and `panic = "abort"`; the `test` profile is optimised (`opt-level = 2`) because the
tests replay real games.  A build is generic for its target architecture, so a binary runs on any machine of that
architecture; tuning for the build machine's CPU is opt-in, `RUSTFLAGS="-C target-cpu=native" cargo build --release`,
for a machine that will run only that binary (the cloud scripts do this, see
[tools/cloud/RUNBOOK.md](tools/cloud/RUNBOOK.md)).  It gains nothing on the Apple M1, where it was measured; it is
unmeasured elsewhere (the generic x86-64 target has no hardware popcount, so measure there).  `tools/pgo.sh` makes a
profile-guided build in `target/pgo/release` (0-2% fewer cycles on the M1, the same answers and trees; its header says
how, and DESIGN.md "Performance" what it gains); `examples/perf.rs` measures the hot paths.

```
cargo test                         # the quick check: the long tests are skipped
cargo test --release               # the full suite
```

Both run as many test threads as the machine has CPUs, and neither builds the study tools (`--features studies` adds
them and their tests).  What each takes, and when each is run, is in [DESIGN.md](DESIGN.md#9-testing).

The ignored tests are for large machines and long runs: `cargo test --release --test engine -- --ignored
many_threads` (64 and 192 worker threads), `TXENGINE_STRESS=2000 cargo test --release --test engine -- --ignored
stress` (random engine configurations through resumes), `TXENGINE_CONSERVATION=500 cargo test --release --test
robustness conservation`, and the benchmarks under `bench` in `src/` and `tests/engine.rs`.  What the suite checks is
listed under "Testing" in [DESIGN.md](DESIGN.md#9-testing); `tests/tripwires.rs` asserts exact node counts, so a change
that alters the search tree on purpose fails it (DESIGN.md says how to update the counts).

## 3. Commands

`taxman help` lists everything; this section shows each command with output from this build.  A tactic is one
switchable piece of the algorithm; `--tactic NAME=VALUE` is repeatable, and the names, values and defaults are in
`taxman help` and [DESIGN.md](DESIGN.md#5-tactics).  Every tactic is proven or sound and on by default
(`match_refine` is 1), so switching one changes how fast an answer comes, never the answer.  The other flags of
`solve` are in the table at the end of this section.

### Solve one game

```
$ taxman solve 480 --use-prev --force-search --show
n=480   score=73120     search  nodes=37              0.005 s  optimal ok  proof=exhausted
  promotions: [68, 138, 147, 153, 168, 175, 180, 182, 190, 195, 196, 198, 200, 207, 208, 210, 220, 222, 224, ...]
  moves: [479, 361, 437, 299, 403, 341, 473, 301, 469, 335, 445, 267, 471, 314, 478, 466, 458, 454, 446, ...]
$ taxman solve 480 --use-prev
n=480   score=73120     known   nodes=0               0.001 s  optimal ok  proof=known from the committed chain (strict.jsonl)
```

`--use-prev` draws on the *known games* (below, "Known games"): the game of n - 1 from them seeds the search and gives
it a ceiling, and a board that one of them has proven is recorded as it is, with no search (the second command; the
first, with `--force-search`, searches 480 from the known game of 479).  Without `--use-prev` a single game has no
predecessor and is solved standalone (below).  The node count of a multi-threaded search varies a little from run to
run.  "optimal ok" means the score equals the table's, not that it is proven (section 5).

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

Each game is built from the one before it.  A game whose search problem an earlier game of the range or of the results
file already posed and settled is concluded at once with that game's promotions (tactic `same_search`, method
`repeat`, proof `same_search`, section 5): 1457 of the 2946 games of a chain to 3440 that would be searched.
With `--out FILE`, each result is appended to the file (one JSON line per game, flushed and synced) as soon as it is
done, and a rerun skips every game the file already completed, whatever the tactics of the rerun (the file's lines
keep the tactics they were made with and what they assume):

```
$ taxman solve 1-300 --out mine.jsonl --checkpoint-dir ckpt
$ taxman solve 1-300 --out mine.jsonl
skipped 300 games already done in mine.jsonl
```

### Past n = 1300

`optimal.json` stops at 1300.  A game above 1300 needs its predecessor in the run or the results file: extend a copy
of a file that has it, or give `--use-prev`, which takes the known game of n - 1 (the committed chain's, which assumes
nothing; the bounds store's where it has proven the board).  The committed `results/strict.jsonl` (a chain from n = 1)
assumes nothing, and a copy of it extended stays so.  The copy matters: `--out` appends to the file and `results/` must
not be overwritten.  The games 1301 and 1302 are known (the committed chain's), so to search them from the known game
of 1300 as a range past the table does, `--force-search` leaves them to their searches:

```
$ taxman solve 1301-1302 --use-prev --force-search --threads 2 \
      --out mine.jsonl --checkpoint-dir ckpt --node-limit 4000 --progress 4
n=1301 0.1s nodes 2979 (done) best 40800 queue 0 spilled 0 | eff 95.4% idle 4.4% lock 0.0% ...
n=1301  score=532272    search  nodes=2979            0.191 s  optimal unknown  proof=exhausted
n=1302 0.1s nodes 3723 (done) best 41436 queue 0 spilled 0 | eff 96.3% idle 3.6% lock 0.0% ...
n=1302  score=533559    search  nodes=3723            0.178 s  optimal unknown  proof=exhausted
```

Game 1301 is a prime, but `--force-search` also skips the prime formula; game 1302 started from 1301, and a node limit
would have stopped either with its checkpoint (the engine checks the limit every 1,024 steps of a thread).  A game
whose predecessor is in neither the run nor the results file, and not taken with `--use-prev` (for example `taxman
solve 1400`, or `taxman solve 480` without the flag), runs *standalone*, with no seed and no ceiling; `solve` says so on
stderr (`note: no previous game for n=...`) and the result line has `"prev": null`.  That is still correct, but costs
about twice as much on the hard games, and many times more on games that a chain settles at once by reaching the
ceiling ([bench/curve/README.md](bench/curve/README.md)).

### Known games

`--use-prev` draws on three sources of games that the run has not made, all in the repository (`rust/results`, no
option names another):

| source | holds | a game of it rests on |
|---|---|---|
| `optimal.json` (built in) | the table of optimal games, n <= 1300 | nothing but the table's word: the line records that it assumes the game optimal |
| `results/strict.jsonl` | the committed chain, each game with its proof | what its line assumes (nothing) |
| `results/bounds.jsonl` | the bounds the ledger found ([results/README.md](results/README.md#boundsjsonl)); a board whose best lower bound is its best upper bound is *proven* | statement U at the boards its upper bound rests on (`rests_on`), one assumption each |

Of a board that several hold, the committed chain's game is taken, then the store's, then the table's.  What it does:

* **A board that a source has proven is recorded with no search**: method `known`, proof `known`, no `setup`, and a
  `known` field that cites the source (section 4; section 5, "Citing a known source").  The game is replayed by the referee, like
  every game, and checked against the game before it (`score(n - 1) <= score(n) <= n + score(n - 1)`) and the table.
  `n + 1` has the game of `n` for its ceiling and seed as usual; a proof that rests on it, the ceiling's or the prime
  formula's, assumes what it does.
* **The game before the range** is the known game of A - 1, if `--out` lacks it (the `prev` of the first line says
  where it came from: `optimal`, `strict` or `bounds`).
* **A board that the store has not proven** (a lower bound, an upper bound, or a gap) still helps its search: the game of
  its best lower bound is a seed beside the game of n - 1 (the one that keeps more is used), and its best upper bound
  is a ceiling next to the one the game of n - 1 gives: a search that reaches the lower of the two stops, and one that
  stopped at the store's bound is proven by it (proof `known`, `known` citing the bound, assuming U where it rests).
* `--force-search` leaves the boards to their searches (and the store does not help them), as it does for the prime
  formula and the repeats.  `regress --use-prev` takes the game before its range from the table only.

A chain from n = 1 with `--use-prev` takes its games from the committed chain as far as that goes, so it re-proves
nothing there; that is for a run that goes past the committed chain (RUNBOOK section 12), not for one that checks the
build.

### A chain from n = 1

```
$ taxman solve 1-2175 --out strict.jsonl --checkpoint-dir ckpt --checkpoint-interval 60 --progress 60
```

Starting at n = 1, no game is taken from `optimal.json` (it is only compared with, up to 1300, where it ends: a score
that differs is a hard error), so no proof in the file assumes anything beyond the frame model.  The committed
`results/strict.jsonl` is such a run, in parts ([results/README.md](results/README.md) has the details of them): its
games 1 to 2175 were made by this command (up to the paths and the thread count), and a run of it reproduces their
scores; its later games continue them (runs on a cloud machine, the first started from a copy of the 2175 lines).  A
run that takes a game from `optimal.json` (`--use-prev`) records, in every result whose proof rests on it, that it
assumes that game optimal ("Known games" above).

### Games in flight

```
$ taxman solve 1-2175 --threads 64 --window 4 --out strict.jsonl --checkpoint-dir ckpt
```

With `--window K` a range keeps up to K consecutive games in flight, so that a machine of many threads is kept busy
where one game's search cannot use them all: game n + 1 is set up while n searches, its search starts from n's best
game so far, without the ceiling, which it gets (with n's final game as a seed) the moment n is concluded, and the
threads move between the searches while they run, to the oldest game that keeps them busy.  Games are still concluded,
checked and recorded in order, and every line has the score, proof, repeat and assumptions that the range solved one
game after another gives it; the moves may be another optimal set, and the nodes and times differ (DESIGN.md, "The
window: games in flight" and "What a window changes in the results").  The default is 4, and with one thread the window
is always 1 (one game after another).  On 4 threads of the laptop a window of 16 makes 1..1300 about 16% faster and
the games past 2225, whose searches already keep 4 threads busy, no slower ([round
B](docs/experiments/round-b-the-chain-on-many-threads-2026-10.md)); on 64 threads a window of 16 makes the games past
2225 20 to 32% slower than one game after another, and 4 is the best measured, so it is the default, and what
`tools/cloud/chain.sh` runs with ([scaling to 192
threads](docs/experiments/scaling-to-192-threads-2026-10.md)).  With the
environment variable `TAXMAN_WINDOW_TRACE` set, the run prints the window's events on stderr (games set up, searches
started, threads moved, deferrals, hints, conclusions).

### Checkpoints, resuming, signals

With `--checkpoint-dir DIR`, a game's search writes `DIR/nNNNN.ckpt` every `--checkpoint-interval` seconds (default 60),
atomically and synced, and on any stop.  The same command resumes it:

```
$ taxman solve 992 --use-prev --force-search --threads 1 --node-limit 4000 --progress 1 --out r992.jsonl --checkpoint-dir c992
n=992 0.2s nodes 4095 (stopped) best 24175 queue 12 spilled 0 | eff 94.1% ... pauses 1 (5.9%) max 13.9ms park 0.0ms
n=992   score=310675    search  nodes=4095            0.282 s  incomplete
stopped: the search for n=992 hit a limit before it finished; run the same command to resume it
$ taxman solve 992 --use-prev --force-search --threads 1 --node-limit 4000 --out r992.jsonl --checkpoint-dir c992   # resumes, stops again
$ taxman solve 992 --use-prev --force-search --threads 1 --out r992.jsonl --checkpoint-dir c992   # resumes and finishes
n=992   score=310690    search  nodes=14789           0.431 s  optimal ok  proof=exhausted
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
  stops there, exit status 0 (with `--keep-going` it goes on: next subsection).
* **The wall.**  `--game-hours H` ends the range at the first game whose own search passes H hours, summed over every
  invocation and counted only while the game is the oldest of the window with its search running (not while it
  waits behind another).  Every search in flight stops with a final checkpoint, nothing is recorded for that game, not
  even an incomplete line, `solve` prints `wall at N after H hours` and exits with status 4; the game's clock is kept
  beside its checkpoint (`nNNNN.clock`), so the same command with a larger H goes on from both
  ([DESIGN.md](DESIGN.md#the-window-games-in-flight)).
* **The result is recorded before the checkpoint is deleted**, so a crash never loses a finished game; at worst the
  checkpoint of a finished game is left over, and the next run deletes it.
* **Memory.**  Searcher state is bounded; the shared queue of work units spills to `DIR/spill` beyond `--mem-cap` units
  (default 1048576).  The committed run never got near it: its queue peaked at 178 units.

### Keeping going past limits, and the sweep

```
$ taxman solve 3441-3600 --threads 1 --time-limit 180 --keep-going --out part.jsonl
```

With `--keep-going`, a game whose search a limit stops does not end the range.  Its best game so far is recorded as an
unfinished line (`completed` false, `best_so_far`, the moves), and the next game goes on from that game as a seed and
nothing more, since it is not known to be optimal: the next game gets no ceiling `n + score(n - 1)` from it, no prime
formula, and no proof that rests on it, and its line says so (`prev` has no proof; section 5, "What a game had from
n - 1").  What does not rest on the game before it is as ever: an exhausted search is a proof, and so is a repeat of a
completed game of the file.  The games are solved one at a time (`--window` is 1).  A rerun on the same `--out`
passes over the games the file holds only unfinished lines of, seeding the game after the last with its best (the best of
its lines), and solves the others.  `--retry` searches the unfinished games again instead (each from its checkpoint, if
`--checkpoint-dir` kept one), as a second pass with a longer limit does.  `--retry-improved` searches again only the
unfinished games whose game before it has more to give than it had when they were tried (it is proven now, or has a
better best game than their seed, or they had none), and passes over the rest; the games it searches again go in
increasing n, so that one it has just proven helps the next.  A signal still ends the range, but with no line for the
game it stopped, so that a rerun resumes it.  `--pass N` records N in `options.pass` of every line the run writes, and
changes nothing else.

**The sweep** (`rust/tools/cloud/sweep.sh FIRST LAST`, [RUNBOOK section 13](tools/cloud/RUNBOOK.md#13-the-sweep))
goes over a long stretch of board sizes for as many proven games as the time allows, holes allowed: the stretch is
split into blocks of consecutive games of about equal expected work, one `taxman solve --keep-going` on one thread
for each, with a limit on each game's search (`GAME_SECONDS`, 180); then passes of `--retry-improved` on the merged
file, so that each game sees the game before it whichever block made it, until a pass changes nothing.  Its file,
`explore.jsonl`, holds every line of every block and pass.  It is **not a chain** and never goes into
`results/strict.jsonl`: it has board sizes with only an unfinished search, or none, and each proof in it rests only
on the games it holds.  Both referees accept it (section 5).

**The ledger** (`cargo run --release --example ledger -- run A-B --dir DIR --cpu-hours H --workers P`, then `ledger
report DIR --known FILE`; the module comment of `examples/ledger/main.rs` has the flags and the files) works on a
whole stretch at once instead of in order: for every board a lower bound, a game it holds, and an upper bound, each
with how it was derived, tightened by limited one-thread searches (P at once, each going on from its checkpoint, with
its upper bound read from the checkpoint: the *dual bound*), by searches under floors that step an open board's upper
bound down (each costs about a tenfold more than the one before, and gives the board a line that forecasts its proof)
and by the relations between boards (the ceiling, the prime formula, repeats, monotonicity, both ways), where a proof
looks cheapest (the forecast), with a finder of good games (large
neighbourhood search, which is not sound: its games are lower bounds once the referee has replayed them); a board is
proven when its bounds meet.  It never reads the known optima unless it is given proven games (`--proven FILE`), so a run on a stretch the
chain has proven is a fair test, which `report --known` checks.  What it finds is kept in the bounds store,
`results/bounds.jsonl` (`ledger export`; `run --bounds` starts from it; two referees:
[results/README](results/README.md#boundsjsonl)), which `taxman solve --use-prev` draws on too (below, "Known games").  Its files are its own, not results lines
([DESIGN.md, "The ledger"](DESIGN.md#the-ledger); the experiment: [the
ledger](docs/experiments/the-ledger-bounds-from-both-sides-2026-10.md)).

### All flags of `solve`

| flag | meaning |
|---|---|
| `--threads K` | worker threads (default: all CPUs; honours affinity masks and cgroup limits) |
| `--window K` | games of a range in flight at once (also for `regress`): game n + 1 is set up, and searched from n's best game so far, while n searches, and the threads move to the searches that keep them busy; games are still concluded and recorded in order, with the scores and proofs of a window of 1 (default 4; 1 is one game after another, and it is always 1 with one thread) |
| `--tactic NAME=VALUE` | set a tactic, repeatable |
| `--out FILE` | append result lines; skip games the file completed; the predecessor of the first game comes from it |
| `--checkpoint-dir DIR`, `--checkpoint-interval S` | checkpoints (default every 60 s) |
| `--progress S` | seconds between progress lines on stderr (default 60; 0: none) |
| `--time-limit S`, `--node-limit N` | stop one game's search, keeping its checkpoint |
| `--game-hours H` | the wall: end the range, with exit status 4 and nothing recorded for the game, at the first game whose own search has passed H hours in all (above) |
| `--mem-cap UNITS` | work units kept in memory before spilling to disk |
| `--force-search` | search where the previous game, an earlier search or a known source would give the answer (skips `prime_shortcut`, `same_search` and the known boards of `--use-prev`) |
| `--use-prev` | draw on the known games of `optimal.json`, the committed chain and the bounds store (above, "Known games"): a board one of them has proven is recorded with no search, citing it and what it assumes; the known game of n - 1 gives the ceiling and the seed when neither the run nor `--out` has it; the store's other boards seed their searches with its lower bound's game and stop at its upper bound |
| `--keep-going` | a game that a limit stops does not end the range: its best game so far is an unfinished line and seeds the next game, which takes nothing that rests on its being optimal; the games are solved one at a time; a rerun passes over the unfinished games (above) |
| `--retry`, `--retry-improved` | with `--keep-going`, search again every unfinished game of `--out`, or only those whose game before it now has more to give than when they were tried (above) |
| `--pass N` | record N as `options.pass` of every line written (for a script that goes over the games more than once); no other effect |
| `--show` | print each game's promotions and moves |

### The other commands

```
$ taxman regress --to 400                       # solve 1..400 in memory and check every score against optimal.json
solved 1..=400 (2 threads, tactics default) in 0.4 s
by method: trivial 13, prime 78, repeat 151, search 158
400 games match optimal.json, 0 are not in it
```

`regress` (default `--from 1 --to 400`; `--tactic` flags and `--use-prev` apply, the latter taking only the game before
its range, from the table) is the quick end-to-end check that the build still gives the Java's answers; a score that
differs stops it, as it stops any run.

```
$ taxman verify-optimal                         # replay every game of optimal.json (1300 games, ~170 ms)
verified 1300 games (n up to 1300, 367449 moves)
$ taxman verify-results results/strict.jsonl         # see section 5
$ taxman extend-optimal results/strict.jsonl --out /tmp/optimal.json     # the table plus the games it lacks
$ taxman tables 85 8 45                         # the maximal factors and composites of 8 and 45 on the board 1..=85
8: maximal factors [4]; maximal composites [16, 24, 40, 56]
45: maximal factors [9, 15]; maximal composites []
$ taxman bench --history /tmp/h.jsonl           # time the benchmark games 936 ... 1184 (standalone, no derivation)
```

`bench` appends one JSON line per run to its history file, `rust/bench/history.jsonl` of the repository unless
`--history FILE` names another (pass it to keep the checkout clean), and takes the games past `optimal.json`'s from
`rust/results/strict.jsonl` unless `--known FILE` names another results file.  The ladder is the harness for
hard games and for scaling over thread counts: `cargo run --release --example ladder -- --scaling --seed-opt --known
results/strict.jsonl --seconds 600 1215` (its flags are documented at the top of
`examples/ladder.rs`).

`verify-results [--threads K] FILE...` checks each file as `solve` checks its `--out FILE` when it opens it.  The
checks run in order, but the search problems that they set up again for the repeats (most of the time on a long file)
are set up first, on K threads (default: all CPUs); `--threads 1` sets them up one at a time, as the checks need them.
The result and the first line to fail are the same for any K.  `solve` takes the threads for this from its own
`--threads`.

`extend-optimal RESULTS --out FILE` writes the table of optimal games (the built-in one, or the file `--table FILE`
names, say a table extended before the binary was rebuilt with it) to FILE with the games of the results file RESULTS
appended.  It reads RESULTS with the checks of `verify-results`, which refuse it if a score differs from the table's
for any n they share.  Then, from the board size after the table's last, it takes each game that RESULTS completed, up
to the first that is missing or only an unfinished search, and says which and why.  A game that assumes a game of the
table optimal is taken too: it assumes nothing the table does not stand for.  The old games are written back byte for
byte, FILE is written whole or not at all, and RESULTS can never be FILE.  This is how games 1001 to 1300 were added to
the Java's 1000.  The table has not been extended past 1300 (and stays there for now), so the same command on the
committed file adds all its games past 1300:

```
$ taxman extend-optimal results/strict.jsonl --out /tmp/optimal.json
added 2614 games (n = 1301..=3914); the table has 3914 games, n up to 3914; wrote /tmp/optimal.json
stopped at n = 3915: the results end at n = 3914
```

Check the new file with `taxman verify-optimal /tmp/optimal.json` before it replaces `src/main/resources/optimal.json`;
the binaries built after that carry the new table.

`certify (A-B | --list FILE) --results FILE... --upper FILE... --out FILE` proves, game by game, that the recorded
scores of A..B (or of the boards of the list FILE, one board size a line, in that order) that rest on the frame model
are the optima on the whole board (statement U at each n; section 1, "What optimal means"):
for each game the prover's barred bounds (`--upper`, `prover/data/upper_*.jsonl`) say needs it, a search of the
extended problem with the bounds' open members optional, started from the recorded game, ends `certified` (nothing
beats it) or `beaten` (a game that does, written to `OUT.counterexample.jsonl` too; the run exits with status 2).  It
takes `--threads`, the checkpoint flags and the limits of `solve`, and resumes like it; the extended problem keeps the
claims of `unzip` that Lemma M5⁺ allows unless `--no-claims` (the formulation of the certificates made before it), and
a line records which.
`verify-certificates CERTS... --results FILE... --upper FILE... [--exact FILE...] [--threads K]` checks the files it
wrote without changing them, line by line, and says how far the proofs of the recorded scores reach (the prover's, the
certificates' and the ceilings').
[DESIGN.md, "Certificates of U"](DESIGN.md#certificates-of-u) has what they check and what a line holds.

## 4. Reading the output

### The summary line (stdout, one per game)

```
n=250   score=19533     search  nodes=34   0.003 s  optimal ok  proof=exhausted
n=1220  score=468662    search  nodes=1375952   8.209 s  incomplete
n=1299  score=531389    repeat  nodes=0               0.079 s  optimal ok  proof=same_search of 1298
n=1301  score=532272    prime   nodes=0               0.016 s  optimal unknown  proof=prime_formula  assumes the game of 1300 in optimal.json is optimal
n=4754  score=7106699   known   nodes=0               0.147 s  optimal unknown  proof=known from the bounds store (bounds.jsonl)  assumes statement U holds at n=4753
```

Board size; score (for an incomplete game, the best found so far); the method (`trivial`, `prime`, `repeat`,
`search`, `known`); search nodes (steps, see the glossary); seconds for the game in this invocation; how it compares
with `optimal.json` (`optimal ok`: equal; `optimal unknown`: the table has no game of n; `incomplete`; a score that
differs is a hard error, which stops the run naming the game); the proof kind, for `same_search` the game whose search
it repeats and for `known` the source it cites; and what the proof assumes, if anything (section 5).

### The progress line (stderr, every `--progress` seconds and at the end)

```
n=992 0.3s nodes 11527 (stopped) best 24175 queue 42 spilled 0 | eff 95.8% idle 1.0% lock 0.0% splits 1 steals 1 full 6.6ms ramp 1.0% tail 0.0% rebuild 0.1% first 1.0% pauses 1 (3.2%) max 9.8ms park 0.0ms
```

| field | meaning |
|---|---|
| `n=992 0.3s` | the game and the time its search has run over all invocations (`1m05s`, `1h02m03s` later) |
| `nodes 11527 (stopped)` | steps taken; in the middle of a run the rate (`164.6k/s`), at the end `done`, `optimal`, or `stopped` |
| `best 24175` | the incumbent: the largest *promotion sum* so far (the score is `in_the_bag` plus this) |
| `queue 42 spilled 0` | work units in the shared in-memory deque, and spilled to disk |
| `eff` | effective parallelism: time workers spent searching over the time they were alive (wall time times threads, unless the number of workers changed) |
| `idle`, `lock` | shares of that time spent waiting for work, and waiting for the deque's lock |
| `splits steals` | times a worker handed work to idle ones, and units taken from the front of the deque |
| `full 6.6ms` | time until every worker first ran at once |
| `ramp`, `tail` | shares of worker time spent waiting for work before every worker first ran at once, and after the last time a worker finished a unit while all were running (the search's start and its end, when there is not yet or no longer work for all) |
| `rebuild`, `first` | shares of worker time from the start of a unit to its first step (rebuilding the search's state from the unit), and in that first step (its bounds solved from scratch) |
| `pauses 1 (3.2%) max 9.8ms park 0.0ms` | checkpoints (each parks all workers): their share of worker time, the longest pause, the longest wait for the workers to park |

### A results line (`--out`, schema 5)

One JSON object per line, compact.  Shown here for n = 250 (a real line, from a run of 1-300), with the moves cut
short, the fields regrouped and comments added, so it is not valid JSON as printed:

```
{"schema":5, "n":250, "score":19533, "moves":[241,169,221,...], "promotions":[84,98,104,105,110,112,114,117,120,124,125],
 "method":"search", "completed":true,
 "proof":"exhausted", "reached_cap":false, "assumes":[],
 "in_the_bag":18320,                      # score = in_the_bag + sum(promotions): 18320 + 1213 = 19533
 "prev":{"n":249,"score":19382,"source":"run","moves_digest":"44877b44d9bbf51a","proof":"same_search","assumes":[]},
 "same_search":null,                      # a repeat names the game it repeats here
 "nodes":34, "stats":{"promote_attempts":20,"promotions":36,"children":13,"match_bounds":28},
 "engine":{"splits":0,"checkpoints":0,"spilled_units":0,"max_queue":1},
 "setup":{"frames":4,"candidates":48,"max_total":11,"greedy_sum":1194,"cap":1312,
          "digest":"fe5e05eb67406e98"},
 "seconds":{"frames":0.0005,"greedy":0.00002,"search":0.0015,"search_all_runs":0.0015,"total":0.0029},
 "invocations":1, "threads":2,
 "tactics":"default",
 "tactic_switches":{"prime_shortcut":true,"same_search":true,"match_refine":1,"fix_forced":true,"nogoods":true,
                    "forget_nogoods":true,"lagrange":false,"gf2":true,"symmetry":true,"prev_cap":true,
                    "seed_incumbent":true},
 "limits":{"seconds":null,"nodes":null},
 "options":{"force_search":false,"use_prev":false},
 "optimal_match":true, "build":{"commit":"8a6df9028f54...","dirty":true}}
```

Game 249 of that run repeated the search of 248; its line has `"method":"repeat"`, `"proof":"same_search"`,
`"same_search":{"n":248,"digest":"a1b7113c4c3d1567","promotion_sum":1187,"proof":"exhausted","assumes":[]}`, and a
`setup` with `"cap":null` and the same `digest`.

| field | meaning |
|---|---|
| `schema`, `n` | the format of the line (5, the only one this build reads) and the board size |
| `score`, `moves`, `best_so_far` | the game and its score; for an unfinished search `score` is null and `best_so_far` (a key only such a line has) has the best so far |
| `promotions` | the promoted numbers, ascending |
| `method` | `trivial`, `prime`, `repeat`, `search`, `known` (how the game was found; `known`: taken from a known source with no search, `--use-prev`) |
| `completed` | false only if a limit or a signal stopped the search; such a line is an unfinished-search record, and its checkpoint is kept (a `--keep-going` run writes none for a signal) |
| `proof` | how optimality is established: `trivial`, `prime_formula`, `cap`, `exhausted`, `same_search`, `known` (section 5); null if incomplete |
| `reached_cap` | the promotions reach `cap`, where the search stops (one started before its ceiling was known, in a window of games in flight, may have run to the end of its tree; DESIGN.md, "What a window changes in the results") |
| `assumes` | what the proof assumes beyond the frame model, here or in the games it rests on (n - 1, or the game a repeat repeats, or the known source a `known` proof cites): for a game of n - 1 or of n taken from `optimal.json`, "the game of n in optimal.json is optimal"; for a bound of the bounds store, "statement U holds at n=4501" for each board its upper bound rests on; `[]`: nothing |
| `in_the_bag` | what the game with no promotions scores (null for prime shortcuts and known games, which set no board up) |
| `prev` | the game of n - 1 this one was built from: score, where it came from (`run`, `results`, `optimal`, `strict`, `bounds`), a digest of its moves, its proof and what it assumes; null if there was none.  A `run` or `results` game with no proof is the best game of an unfinished search (`--keep-going`): a seed, proven by nothing (section 5, "What a game had from n - 1") |
| `same_search` | for a repeat, the earlier game of the file whose search it repeats: its board size, the digest of the search problem they share, the problem's best promotion sum, that game's proof and what it assumes; null otherwise |
| `known` | a key only a line that cites a known source has (section 5, "Citing a known source"): `{"source":"optimal"}` or `{"source":"strict"}` for a game taken from `optimal.json` or the committed chain, or `{"source":"bounds","lower":{"rule":..,"source":..},"upper":{"rule":..,"source":..}}` for the lines of the bounds store whose bound of n is the score (`lower` null when a search found the game and the upper bound stopped it) |
| `nodes`, `stats` | search steps over all invocations, and the searcher's counters (`match_bounds` is the number of node bounds computed, GF(2) or matching) |
| `engine` | this invocation's splits, checkpoints written, units spilled, largest in-memory queue |
| `setup` | null on a `trivial`, `prime` or `known` line, which set up no search; otherwise the frames and candidates found, `max_total` (the most promotions the freedoms allow), the greedy start, `cap` (ceiling on the promotion sum, `n + score(n-1) - in_the_bag`; null on a repeat, which ran no search to stop), and `digest`, a digest of the search problem the setup poses (DESIGN.md, "Same search"), by which a later game finds this one to repeat |
| `seconds` | per phase, this invocation (`search_all_runs`: the search over every invocation of a resumed game) |
| `invocations` | how many invocations ran the search (a resumed game has more than one) |
| `threads`, `tactics`, `tactic_switches`, `limits`, `options` | what the last invocation ran with; `tactics` is `default` or the changes from the defaults, `options` the flags `--force-search` and `--use-prev`, and, only where given, `keep_going`, `retry`, `retry_improved` (true) and `pass` (the number `--pass` was given) |
| `optimal_match` | true: the score equals `optimal.json`'s (a score that differs is a hard error, never written); null if the table has no game of n |
| `build` | the commit the binary was built from, and whether its sources differed from it |

This build reads schema 5 only: a results file of an older schema has to be made again.  A line is never rewritten: a
resumed game that is stopped again adds another incomplete line, and the completed line comes last.

## 5. Why the answers can be trusted

### Proof kinds

Each completed game records how its optimality is established (`proof`):

| proof | meaning | rests on the game of n - 1? |
|---|---|---|
| `trivial` | the board has no promotion candidates; the game with no promotions is the only one | no |
| `exhausted` | the search explored its whole tree | no |
| `cap` | the search reached `n + score(n-1)`, which no game of n can exceed (removing the move n from an optimal game of n leaves a game of n - 1) | yes |
| `prime_formula` | a prime n plays the game of n - 1 with its first move replaced by n; optimal if the game of n - 1 is (proved in `src/accelerate.rs`, at `prime_shortcut`) | yes |
| `same_search` | the game's search problem (frames, candidates, `max_total`, the factor graph among their numbers) is the one an earlier game m of the file posed, and m's proof (`exhausted`, `cap` or `same_search`) settled its best promotion sum; the game is m's promotions on this board (tactic `same_search`; the argument is in [DESIGN.md](DESIGN.md#same-search)) | no: on the game m it repeats |
| `known` | a known source (`--use-prev`) holds the game, or an upper bound of it ended the search: the line's `known` field cites the source, and what the proof assumes follows from it (below) | no: on the source |

### Citing a known source

A line of proof `known` has a `known` field that says which source it rests on:

| `known` | the line | the check |
|---|---|---|
| `{"source":"optimal"}` | method `known`: the table's game of n, assumed optimal | the table has a game of n that scores the line's score |
| `{"source":"strict"}` | method `known`: the committed chain's game of n, as proven as its line is | the committed chain has a game of n that scores it; the line assumes what that line does |
| `{"source":"bounds","lower":{..},"upper":{..}}` | method `known`: a board the bounds store has proven; the `lower` bound is the game played | the store has a lower line and an upper line of board n worth the score, by the `rule` from the `source` (the run that found it) the field says |
| `{"source":"bounds","lower":null,"upper":{..}}` | method `search`: the search found the game and stopped at the store's upper bound, which it meets; `reached_cap` is false (the ceiling of n - 1 is not what it reached) | the store has an upper line of board n worth the score, as cited |

The score is optimal, wherever the cited bound holds: a finished game that scores what an upper bound of the board
allows is the best (a lower bound is a game, and a game replayed by the referee is a lower bound).  The referees check
that the game plays and scores the line's score, and that the cited lines are in the sources and are what the line
says; they do not check the sources again (`ledger verify-bounds` and `tools/check_bounds.py` check the store, the
referees of results files the committed chain), and which game the line plays is not compared with the source's.  So
a file with `known` lines is checked against the sources of the repository's `rust/results` (the defaults of `taxman
verify-results`; `tools/check_results.py` takes `--committed FILE` and `--bounds FILE`), and one that cites a source
that was not given is refused.  A known game that is wrong would be caught by the chain (a score outside
`score(n - 1) <= score(n) <= n + score(n - 1)` is an error), by the table where it has the game, and by the next
`ledger verify-bounds`; a search that finds a game above the store's bound stops the run, naming the board: the store is
wrong, or statement U fails at a board its bound rests on.

### Assumptions

What a game *assumes* is what its proof rests on beyond the frame model: if its proof rests on the game of n - 1, what
that game assumes, and so on down the chain; a repeat rests on the game it repeats instead, and assumes what that game
assumes; a `known` proof rests on its source and assumes what the source's game does.  A game taken from
`optimal.json` (`--use-prev`) is assumed optimal, since the table records nothing about how a game was found: a proof
that rests on its game of n - 1 assumes "the game of n - 1 in optimal.json is optimal", and a game of n taken from it
"the game of n in optimal.json is optimal".  A bound of the bounds store holds wherever statement U holds at the boards
its upper bound rests on (`rests_on`; a bound of the frame model at a board rests on that board, a relation on what the
bound it comes from rests on): a game that rests on it assumes "statement U holds at n=4501" for each of those boards,
and a game that rests on that game assumes the same.  The committed chain's games assume what their lines do (nothing).
A game that assumes nothing is proven within the frame model, given that the solver is right.  `taxman solve` prints
what a game assumes in the summary line, and `verify-results` counts the games that assume statement U at some board
together.  What a proof rests on through the frame model, the statement U at n (section 1, "What optimal means"), is
not recorded for the proofs that the frame model gives (an exhausted search, a trivial board, a repeat): it holds for
every n <= 2175 (the prover); past 2175 it is not yet certified.

### What a game had from n - 1

A line says it plainly in its `prev`, and in `options.pass` which pass over the games made it (a sweep goes over them
more than once; a run with no `--pass` has none):

| `prev` | what the game had from the game of n - 1 | what its proof can be |
|---|---|---|
| `null` | nothing: it was solved standalone (a block's first game, or the game before it is missing), with no seed and no ceiling (`setup.cap` null) | `trivial`, `exhausted`, or `same_search` on a completed game of the file |
| `source` `run` or `results`, `proof` null | the best game of an unfinished search of n - 1, as a seed, and no more: no ceiling (`setup.cap` null), no prime formula, nothing that rests on it | the same: never `cap` or `prime_formula` |
| `proof` given (or `source` `optimal`, `strict`, `bounds`) | n - 1 proven (or assumed, from `optimal.json`; or the committed chain's or the bounds store's game, `--use-prev`): the seed, the ceiling `n + score(n - 1)`, the prime formula for a prime n, and repeats | any |

The game of n - 1 that a `results` line names is in the same file: a completed game, or, with no proof, an earlier
incomplete line with the score and moves it records.  A game that a retry searched again has several lines, an
incomplete one made when n - 1 was less (or missing) and a later one made when it was more; each says what it had.  So
a file of a sweep, with its board sizes that only an unfinished search is recorded for, reads: a completed game is
proven by its `proof`, given what its `assumes` say (a `cap` or `prime_formula` rests on a proven `prev`, a
`same_search` on the game it repeats, the others on nothing of n - 1); an unfinished game is a lower bound; and where
a game and the one before it are both completed, the chain bounds hold between them.

### The verifiers

* Every `solve` replays each game through `game::replay_finished`, an independent referee that shares no code with the
  solver's number theory (legal, over, opens with the largest prime, scores what is recorded), and checks the arithmetic
  and the ceilings `score(n-1) <= score(n) <= n + score(n-1)` before the game counts.  A game that fails after it was
  found goes to `<results>.rejected.jsonl` and the run fails.
* `taxman verify-results FILE...` reads results files without changing them: every game replayed and its score confirmed
  against `optimal.json`, the chain from n - 1, each line's arithmetic and provenance, that its fields agree with each
  other as the solver writes them (so that a hand edit cannot drop an assumption or change a proof), that the files
  agree, and a count of games by proof kind and by what they assume.  For a repeat it sets the search problems of both
  games up again and compares them.  For a `known` line it checks the citation against the sources ("Citing a known source"
  above): the table, the committed chain and the bounds store of the repository, which it reads as they are, since their
  own referees check them (`ledger verify-bounds` and `tools/check_bounds.py` the store).  Opening a results file for
  `--out` runs the same checks, with the sources `--use-prev` draws on.
* **Both accept a file with holes** (a `--keep-going` run's, the sweep's `explore.jsonl`: board sizes with no line, or
  with only incomplete ones), and check each line against what it rests on in the same file: a ceiling (`setup.cap`)
  or a `cap` or `prime_formula` proof needs the game of n - 1 completed in the file, a repeat needs the game it
  repeats completed on an earlier line, and an exhausted search needs nothing of n - 1.  A line built on the best game
  of an unfinished search names an earlier incomplete line of n - 1 with that score and those moves, scores no less
  than it, and has no ceiling and no such proof.  An incomplete line that records its game (`best_so_far`, the moves)
  is checked to be legal, to score the `best_so_far`, and to be no better than `optimal.json`'s score.  The report
  names the board sizes that only an unfinished search is recorded for.  A line whose game of n - 1 is not in the file
  at all is still refused, as it was (`tests/results.rs` shows each case with both referees).
* `python3 tools/check_results.py [FILE...]` is an independent referee in Python (standard library only, no shared
  code).  It re-derives every proof and assumption, and also checks each score against `optimal.json`, the
  Franklín-Moniot upper bound and the best-known heuristic chain in `heuristics/results`.  With no arguments it checks
  `results/*.jsonl` but the store `bounds.jsonl`, which is not a file of results.  It cannot rebuild the frame model: it
  takes an `exhausted` line at its word, and of a repeat checks only what the lines show (the game repeated is earlier
  in the file and settles its search, the recorded digests and promotions agree).  A `known` line it checks against
  the sources (`--committed FILE` and `--bounds FILE`, by default the committed chain and the store of `results/`),
  from the lines of the files alone; it cannot play the store's promotion sets, so the digest of a game of n - 1 taken
  from the store is not one it checks.

```
$ taxman verify-results results/strict.jsonl
results/strict.jsonl: 3914 completed games, n = 1..=3914 (0 missing), 0 incomplete lines
  proof: trivial 13, prime_formula 541, cap 354, exhausted 1356, same_search 1650
  assumptions: none in 3914 of 3914 games
  scores at a proven ceiling, given the game of n-1: n + score(n-1) 1244, prime formula 540
0 board sizes are in more than one file, with equal scores in all
```

What the checks cannot catch is a game that is legal, finished and within the chain bounds but not optimal when the game
of n - 1 is not optimal either; that is what the proof kinds and the assumptions track, and why a chain from n = 1
matters.  [results/README.md](results/README.md) says what supports each committed score (re-solves from scratch, a
second build's chain, what the games assume).

## 6. The pipeline

```
 taxman solve A-B --out FILE --checkpoint-dir DIR
   |  solver::solve_range_each        n = A..B, concluded in order (with --window K up to K games in flight, each set
   |                                  up and searched on threads of its own: solver/window.rs); the game of n-1 comes
   |                                  from this run, FILE or (--use-prev) the known games (known::Known),
   |                                  and is checked (legal, over, opens with the largest prime) before anything uses it
   v
 solver::set_up(n) -> set_up_with_width::<W>      W = 8 to 128 words of 64 bits (bits::WIDTHS), by n
   |
   |  [1] numtheory::Tables            maximal factors and maximal composites of 1..=n; the set width W
   |  [2] n prime ............ accelerate::prime_shortcut   the game of n-1 with its first move replaced by n
   |                                                         method=prime        proof=prime_formula
   |  [3] board::Board::new            frames::BoardFrames: sources (never tax) + factors -> frames; unzip,
   |                                   removeImpossible
   |                                   frames::build_frames: frames with freedoms numbered from the front (0 = front)
   |                                   and merged; candidates; in_the_bag
   |      no candidates ...... the game with no promotions   method=trivial      proof=trivial
   |  [4] search ............. method=search                 proof=exhausted | cap
   |        Board::prepare             search::prepare: model::FrameGraph::build -> the root FrameSet; greedy set;
   |                                   max_total; the search::Problem it poses, and its digest
   |        same_search ...... one of the 8 games before it posed the same problem and settled it: its promotions
   |                                                         method=repeat       proof=same_search
   |        search::search             starting incumbent = best of greedy / seed (n-1's promotions) / rounded root
   |                                   matching + local search; ceiling = n + score(n-1) - in_the_bag
   |          engine::run              worker threads; shared deque with disk spill; incumbent; checkpoints; signals
   |            Stepper::step          per node (search/searcher.rs): check_node = the rule of symmetric pairs
   |                                   (symmetry, search::symmetry) -> GF(2) bound (gf2; with gf2=off
   |                                   the matching bound, matchbound, and its refinement) -> conflicts, forced
   |                                   members, nogoods (search::refine, model::Nogoods) -> with lagrange the
   |                                   Lagrangian bound over the nogoods (search::lagrange);
   |                                   choose_branch (a candidate) -> make_child -> check_child
   |      promotions -> accelerate::solution_for_promotions   (frames::BoardFrames with the promotions as sources)
   |                                                         -> ordered moves
   |
   |  [2] and same_search are skipped by --force-search and when the game has a checkpoint (a search is resumed,
   |  never replaced)
   v
 [5] solver::conclude                replay (game::replay_finished); score == in_the_bag + promotions; the ceilings;
   |                                 confirm the score against optimal.json; the proof and what it assumes
   v
 [6] results::ResultsFile::append    the JSON line, synced; only then is the game's checkpoint deleted;
                                     the game becomes the previous game of n+1
```

The numbers are those of [DESIGN.md](DESIGN.md#7-per-game-pipeline-solver), which gives the Java class each step
replaces, and of the module comment of `src/solver.rs`, which gives the checks of each.

## 7. Glossary

Words that mean more than one thing in this code base are marked **(overloaded)**, with each meaning.  Words used
inside one part of the code are defined where they are used, not here: the setup's (`unzip`, mandatory moves, claimed
factors) in the module comment of `src/frames/mod.rs`; the peel, `reduce` and the home frame in `src/model/`; the
bounds' internals (potentials, weight splits, contraction, refinement, refits) in [DESIGN.md, section
4](DESIGN.md#4-the-search); the engine's (units, splits, steals, spills, pauses) in [section
6](DESIGN.md#6-engine) and the module comment of `src/engine.rs`; the fields of the progress line in section 4 above;
the Java's and the wiki's own words, mapped to these, in [DESIGN.md](DESIGN.md#the-java-solver-and-the-frame-solver).

### The game

* **board, pot, n**: the numbers 1..=n; the pot is what is still on it during a game.  In the code `board::Board` is a
  board analysed for the search: its tables, frames, candidates and `in_the_bag`.
* **move, tax**: a number the player takes, and the divisors the taxman takes with it.
* **score, opt(n), optimal game**: the sum of the player's moves; the best score on the board 1..=n; a game that
  scores it.
* **maximal factor** of m: m/p for a prime p dividing m (a divisor reached by deleting one prime).  Its inverse is the
  **maximal composite**; the **maximal factor graph** joins each number to its maximal factors.  A move is playable if
  one of its maximal factors is still in the pot (equivalent, on a board without holes, to "some divisor is": the
  lifting argument in `src/numtheory.rs`).

### The frame model

* **source**: a number with no maximal composite (every number above n/2): never tax, and played unless it is found
  impossible (below).  **factor**: any number that is not a source.
* **frame**: a connected group of sources together with all the factors they compete for (sources that share a factor
  are in one frame).  Setup simplifies the frames, numbers the ones that have freedoms by layer, from the front (0 is
  played first), and merges those with the same number, so that a promotion always has one frame to go to; the search works
  on the merged frames.  In the code a frame is `frames::Frame` as setup forms it (`frames::BoardFrames` holds all of a
  board's) and `model::Frame` as the search takes it (`frames::SearchFrames` holds those).
* **front, back**: frame X is in front of frame Y (and Y behind X) when a number of Y has a maximal factor in X, that
  is, when Y's numbers are multiples of X's.  Frames in front are played first, so frame 0 is the front, and a
  promotion moves a factor of frame h to a move of frame h - 1, one frame forward.
* **Taxman Mini**: the game a frame plays: a set of moves and a set of factors; each move must still have one of its
  maximal factors in the frame when taken, and taking it removes all of its factors still there.
* **impossible**: a source that cannot be played together with the sources already kept (found greedily from the
  largest).  Impossible sources are dropped from the frame's moves.
* **freedoms**: a frame's factors minus its moves: how many factors it can spare.  A promotion into or out of the frame
  uses one up, and none may go below zero.  A frame with none is **closed** and takes no part in the search.
* **winnable**: some order takes every move of a frame.  A **peel** tests it ([DESIGN.md, section
  2](DESIGN.md#2-the-game-and-the-frame-model)).
* **promotion, promote**: play a factor as a move: take it out of its frame's factors and make it a move of the frame
  in front of it.  Legal when both frames stay winnable.
* **candidate** **(overloaded)**: (1) a number setup says could be promoted (`SearchFrames::candidates`,
  `BoardFrames::promotion_candidates`); (2) `cands` in a search node: the candidates the node's loop still has to try;
  (3) a vertex role in the matching graph: a number that may be matched as a promoted mover, may pay for one behind it,
  or stay free.
* **promotion set, promotable**: a set of candidates that can be promoted together from the root, keeping every frame
  winnable and its freedoms at least 0.  The score of a game of the model is `in_the_bag` plus the sum of its promotion
  set.
* **in the bag** (`in_the_bag`, the Java's `inTheBag`): what the game with no promotions scores.
* **max_total** (`maxTotalPromotions`): the most promotions the frames' freedoms allow.
* **the frame model**: this decomposition (section 1), exact given that the statement U holds (section 1, "What
  optimal means").  `model::FrameGraph` is the immutable factor graph and home frames, not "the model".
* **layer**: the number of prime factors of a number, counted with multiplicity; a frame's layer is its sources'.  The
  frames in front of a frame lie one layer lower, and frame h is the frames with freedoms of the lowest layer that has
  some, plus h.
* **clean numbering** (N): every frame with freedoms directly in front of frame h is frame h - 1.  The numbering by
  layer is clean by construction, and on every board the solver supports it is the Java's numbering (breadth first
  from the front).

### The search

* **tactic**: a switchable piece of the algorithm (`--tactic`).  **sound / proven**: the grading in
  [DESIGN.md](DESIGN.md#5-tactics); every tactic is one or the other.
* **node, step**: a search node is a set of promotions made from the root plus the candidates left; the `nodes` that
  results and progress lines count are *steps*, one per candidate a node's loop tries.
* **incumbent**: the best promotion set found so far, shared by all threads.  **greedy**: promote every candidate that
  can be, from the largest down; the search starts from the best of that, the previous game's promotions and a rounding
  of the root's matching ([DESIGN.md, section 4](DESIGN.md#the-starting-incumbent-the-ceiling-and-the-seed)).
* **GF(2) bound** (`gf2`, the default): what no completion of a node can exceed, from the heaviest set that is
  independent in two matroids over GF(2), one for the even frames and one for the odd ones (heuristics/THEORY.md,
  Theorem 11; DESIGN.md, section 4).  **matching bound** (`gf2=off`): winnability relaxed to matchability, the largest
  sum of promotions such that every move and every promoted number gets a distinct maximal factor in its frame, a
  maximum-weight bipartite matching kept along the search path (`matchbound`).
* **conflict**: a subset of a promotion set that cannot be promoted together from a state.  Every completion leaves out
  one of its members.  **nogood**: a conflict of the *root* learned from a conflict found at a node; valid at every
  node, any thread, any path.  Each searcher keeps a table of them and forgets it every 50,000 of its steps
  (`forget_nogoods`).
* **strong branching**: which candidate a node promotes next: the member of the conflict whose removal lowers the
  bound most (with `gf2`, exactly for each member: read off the exchange graph of the heaviest set where that
  certifies it, solved where not).
* **forced member**: a member of a node's heaviest set whose removal alone brings the bound below what the node
  needs, so every completion that beats the incumbent promotes it (tactic `fix_forced`: the child promotes all of them
  together).
* **prime exchange, symmetric pair**: exchanging two primes `p < q` in every number; a pair whose exchange maps the
  root's frames and candidates onto themselves is *symmetric*, and maps promotion sets to promotion sets worth
  `(q − p)(c_p − c_q)` more (`c_r` the sum of a set's cofactors `x / r`), so every heaviest set keeps `c_p ≤ c_q`
  for every symmetric pair (heuristics/THEORY.md, Theorem 13).  The search cuts a node none of whose completions
  can keep them, and drops the candidates none that does holds (tactic `symmetry`; DESIGN.md, section 4).
* **dual bound**: what a search that a limit stopped proves from above: the largest of its incumbent and, over the
  open units of its final checkpoint, each node's promotions plus its GF(2) bound (`search::dual_bound`;
  heuristics/THEORY.md, Lemma D).  Stopped before its first node, it is the root's bound.

### Results and trust

* **chain**: games each built from the one before (n = 1, 2, 3 ...): a game's search starts from the previous game's
  promotions and stops at its ceiling.  A game solved with no predecessor is *standalone*.
* **window** **(overloaded)**: (1) the games of a range in flight at once (`--window K`, `solver::window`; section 3,
  "Games in flight"): a search started before the game before it is proven is *speculative*, seeded from that game's
  *provisional game* (its best so far), and a speculative search may be *deferred* (not started while the ceiling
  that its provisional game gives would settle it) or *suspended* (its workers taken for the oldest game, its units
  kept); (2) the eight board sizes before a game among which it looks for one whose search it repeats
  (`SAME_SEARCH_WINDOW`); (3) the probe's windows, a checkpoint resumed for 50,000 steps (DESIGN.md, "Performance").
* **ceiling**: `n + score(n-1)`, which no game of n can exceed; a score that reaches it needs no search to prove (it is
  recorded as a `results::CeilingProof`).  As a bound on the promotion *sum* it is `n + score(n-1) - in_the_bag`, which
  the results lines and the tactic call `cap` (proof `cap`, `reached_cap`, `setup.cap`, tactic `prev_cap`).
* **prime formula**: for a prime n, the game of n - 1 with its first move replaced by n (tactic `prime_shortcut`, proof
  `prime_formula`).
* **search problem, digest, repeat**: what a game's setup gives the search, in a canonical form (`search::Problem`:
  the frames, the candidates, `max_total` and the factor graph among their numbers), and a 64-bit digest of it
  (`setup.digest`).  A game whose problem an earlier game posed and settled *repeats* its search: the tactic
  `same_search` concludes it with that game's promotions (method `repeat`, proof `same_search`).
* **proof (kind)**: how a completed game's optimality is established: `trivial`, `exhausted`, `cap`, `prime_formula`,
  `same_search`, `known` (section 5).  **assumes**: what a proof rests on beyond the frame model, inherited down the
  chain; a game of n - 1 or of n taken from `optimal.json` (`--use-prev`) is assumed optimal, and a bound of the bounds
  store assumes statement U at the boards it rests on (section 5).
* **known game, known source**: a game that `--use-prev` takes from `optimal.json`, the committed chain
  (`results/strict.jsonl`) or a board the bounds store (`results/bounds.jsonl`) has proven, instead of searching for it
  (section 3, "Known games"; `src/known.rs`).  **bounds store**: the bounds on opt(n) that the ledger finds, a line
  each, kept for good ([results/README.md](results/README.md#boundsjsonl); `src/store.rs`).
* **optimal ok / optimal match**: the score equals `optimal.json`'s, the table built into the binary.  It is not a
  proof, and does not exist above the table's last game; a score that differs is a hard error.
* **referee**: code that checks a game or a results file independently of the code that made it: `game::replay`,
  `taxman verify-results`, `tools/check_results.py`.  **rejected**: a game that failed a check after it was found,
  written to `<results>.rejected.jsonl`.
* **results line, schema 5**: the format of a line of `--out` (section 4); this build reads no other.  Two words
  survive only as field names there: `nodes` counts steps, and `prev.source` says where the game of n - 1 came from.
* **checkpoint**: a periodic pause in which all workers stop and the engine writes the unexplored work, the incumbent
  and statistics to `nNNNN.ckpt`; its **header** is what identifies the search (n, width, digests of candidates and
  frames, `max_total`, the tactics).

### Other programs and bounds

* **the frame solver** (this crate, the binary `taxman`; "the solver" for short): a branch and bound over promotions
  within the frame model, which finds the games.  **the Java solver** (`src/main/java`): the original, whose algorithm
  the frame solver reimplements.  **optimal.json**: the table of optimal games, n <= 1300 (the Java's games 1 to 1000,
  then those of the chain from n = 1, appended by `taxman extend-optimal`), built into the binary.
* **the prover** ([prover/](prover/README.md)): a branch and bound over sets of moves on the whole board, which decides
  playability by peel and uses only proven bounds; it proves that no set of moves beats a recorded score, with none of
  the frame solver's code.
  **Franklín–Moniot bound**: the maximum-weight matching relaxation of the whole game, an upper bound on opt(n);
  `tools/check_results.py` compares scores with it.  **parity bound**: the prover's sharper bound, from two matroids
  over GF(2) split by the parity of Ω (heuristics/THEORY.md, Theorem 10).
* **brute force**: `src/brute.rs`, the exhaustive search that is the test oracle for small boards.  **U\***:
  the sources that setup keeps, by descending greedy on each frame; heuristics/THEORY.md's name for the heaviest
  playable set of sources, which they are wherever the upper-half conjecture holds, so on every board to 100,000
  ([prover/README.md, section 2](prover/README.md#2-conjecture--at-scale)).  **U**: the statement that some optimal
  game plays every member of U\* with three or more prime factors, on which the frame solver's answers rest (section
  1, "What optimal means").

## 8. Reading order and map of the documents

The code, in the order to read it, which is the order of the module map in `src/lib.rs` (each file starts with a module
comment that is worth reading first; section 6 above is the map of how the pieces fit, so keep it open):

1. `src/game.rs`: the rules as a referee, and `optimal.json`.
2. `src/numtheory.rs`: maximal factors and composites; and `src/playable.rs`, whether a set of numbers can all be played
   on the whole board, with nothing of the frame model (the oracle of the tests of the model, and the referee of
   `taxman certify`).
3. `src/bits.rs`: `Bits<W>`, the fixed-width bitsets of game numbers that everything below is generic over, and the
   set widths.  The module comment is enough at first.
4. `src/json.rs`: the small JSON reader and writer.  Skim; it matters once you read results lines.
5. `src/frames/`: forming and simplifying the frames, numbering and merging them.  The module comment of `mod.rs` is
   the story (sources, factors, unzip, impossible moves, freedoms, numbering, merging) and builds `SearchFrames`;
   `board_frames.rs` has `Frame` and `BoardFrames`.
6. `src/model/`: `mod.rs` says what is where: `Frame`, `FrameSet` (`frame.rs`), `FrameGraph`, winnability and reduction
   (`peel.rs`), promotion (`promote.rs`), then conflicts (`conflict.rs`) and nogoods (`nogoods.rs`).
7. `src/accelerate.rs`: the shortcuts from the game of n - 1 (read the comment of `prime_shortcut` for the proof of the
   prime shortcut), and the last step from promotions to moves.
8. `src/matchbound.rs`: the matching bound; the module comment is the best explanation of it.  Its child
   `src/matchbound/lagrange.rs` is the Lagrangian bound over the nogoods, built on the same levels and repairs.
9. `src/gf2.rs`: the GF(2) bound, the default node bound: the module comment explains the two frame matroids, how
    a node's are kept by contraction along the search path, and the forced members; `src/gf2/meet.rs` is their
    weighted intersection (the tableau, the augmenting paths, the weight split).  `heuristics/THEORY.md`, Theorem 11,
    is the proof.
10. `src/search/`: `mod.rs` explains the step and says what is where: `searcher.rs` (`TaxSearcher` and `Stepper::step`,
    whose phases are `check_node`, `choose_branch`, `make_child` and `check_child`), `refine.rs` (`Refiner`: conflicts,
    refinement; `strongest`), `lagrange.rs` (the Lagrangian bound in the search: where, forced members, refits),
    `setup.rs` (`prepare`), `tactics.rs`, `incumbent.rs`, and `header.rs` (the checkpoint header).  The
    `if let Some(p) = &mut self.probe` lines in `searcher.rs` are instrumentation
    (`diagnostics.rs`), which can be skipped.
11. `src/bounds.rs`: two small functions the Java has too, the limit on a node's further promotions and the greedy
    incumbent.
12. `src/engine.rs`: the protocol in the module comment (conservation, splitting, checkpoint, termination).
13. `src/tactics.rs`, `src/board.rs`, `src/solver.rs`, `src/results.rs`, `src/store.rs`, `src/known.rs`,
    `src/certify.rs`, `src/paths.rs`: the switchable tactics, a board prepared for a search, the per-game pipeline,
    provenance and verification, the bounds store's line and the relations between boards, the known games that
    `--use-prev` draws on, the certificates of `taxman certify`, the default files.  The range driver is
    `src/solver/window.rs`, the window of games in flight, and its child `window/allot.rs` how they share the
    threads.
14. `src/main.rs`, `src/cli.rs`, `src/commands.rs`: the command line.
15. Studies, skipped on a first read: `src/brute.rs` (exhaustive search, the test oracle) and `src/sacrifice.rs` (the
    sacrifice bound).

| document | what it holds |
|---|---|
| this page | the user guide |
| [DESIGN.md](DESIGN.md) | the design as it is now: goals, the game and the frame model, the architecture, the search, the tactics and their grades, the engine, the pipeline, the trust model, the tests, performance |
| [docs/STATUS.md](docs/STATUS.md) | the current state of the work: what is in flight, paused, decided, waiting on a decision (read it first) |
| [docs/EXPERIMENTS.md](docs/EXPERIMENTS.md) | the register of everything tried, kept or not (check it before proposing an experiment), and the lab notebook: every round of improvements with its measurements, one file an entry in [docs/experiments/](docs/experiments/) |
| [docs/IDEAS.md](docs/IDEAS.md) | ideas borrowed from other fields (SAT, MaxSAT, MIP, #SAT, ...), ranked, with pilot measurements, the profile of the hardest games and the study of nogoods at many threads |
| [docs/AGENDA.md](docs/AGENDA.md) | the research agenda (2026-10): what limits the end of the chain, the avenues ranked, conjectures, and what the cloud runs should measure |
| [results/README.md](results/README.md) | the committed results files and what supports their scores |
| [bench/curve/README.md](bench/curve/README.md) | how the cost grows with n beyond the end of the chain, and projections for a larger machine |
| [prover/README.md](prover/README.md) | the prover: independent proofs and checks of the frame solver's results on the whole board, and the conjecture checks |
| [tools/cloud/RUNBOOK.md](tools/cloud/RUNBOOK.md) | running the solver, and the prover's proofs, on a many-core cloud machine |
| [../heuristics/THEORY.md](../heuristics/THEORY.md) | the theory (maximal factors, playable sets) the model rests on |
| the [wiki](https://github.com/bvchess/taxman/wiki) | the Java solver's algorithm and definitions |
