# Taxman: the frame solver in Rust

The frame solver finds optimal games of Taxman and proves them optimal.  It splits each game into independent groups
of numbers, the *frames*, and searches for the best set of numbers to play beyond the ones the frames settle.  It runs
on many threads, and its long runs resume after an interruption.  Below, "the solver" is the frame solver.

This page is its user guide and reference: what it computes, how to build and run it, how to read its output, and why
its answers can be trusted.  [DESIGN.md](../docs/DESIGN.md) explains how the solver works and why, the [lab
notebook](../notebook/README.md) holds the measurements behind it, and the [glossary](../../docs/GLOSSARY.md) defines
the project's words.  This page links to them instead of repeating them.

`solver/frame/` is a Cargo crate: a library and one binary, `taxman`, with no third-party dependencies.  It
reimplements the algorithm of the Java solver, which reached n = 1000, with a search built for speed and for many
threads.  The Java solver is no longer in the tree; the git tag `java-final` is the last commit that has it.  A second
program, [the prover](../noframe/README.md), checks the solver's answers on the whole board, without the frame model.

* [1. What it solves](#1-what-it-solves)
* [2. Build, test, run](#2-build-test-run)
* [3. Commands](#3-commands)
* [4. Reading the output](#4-reading-the-output)
* [5. Why the answers can be trusted](#5-why-the-answers-can-be-trusted)
* [6. The pipeline](#6-the-pipeline)
* [7. Glossary](#7-glossary)
* [8. Reading order of the code](#8-reading-order-of-the-code)

## 1. What it solves

### The game

The pot holds the numbers 1 to n.  The player takes a number c that still has a proper divisor in the pot, and scores
c.  The taxman then takes every divisor of c still in the pot.  The game ends when no number in the pot has a proper
divisor left in it, and the taxman takes whatever remains.  An *optimal game* scores the most, opt(n).  Game 4 has the
optimal game 3, 4: the taxman takes 1, then 2, and the player scores 7.  The values of opt(n) are the Taxman sequence
([OEIS A019312](https://oeis.org/A019312)).

The solver's results are optimal games, each a sequence of moves with its score.  Its committed chain,
`results/strict.jsonl`, holds one for every n from 1 to the end of the chain, each built from the one before (section
3, "A chain from n = 1").  Its games 1 to 1000 reproduce the Java's table of optimal games, `optimal.json`.  Its games
1001 to 1300 went beyond the table, and have since been added to it (`extend-optimal`, section 3); its games from 1301
on are past the table.  The code can attempt any n up to 8191, the largest board its set widths cover.

[results/README.md](../../results/README.md) gives how far the chain reaches and how each game is proven.  The first
part is proven on the whole board by the prover, and the rest within the frame model, given statement U (below, "What
optimal means").

*What a game costs.*  Finding an optimal game is a search over an enormous tree, and its cost grows with n, in waves:

* Before the GF(2) bound, the cost of a game grew tenfold for every 55 to 70 in n past 1200 ([the cost
  curve](../notebook/benchmarks-the-probe-pgo-and-scaling/cost-curve.md)).
* With the GF(2) bound, the default since 2026-10, the committed chain from n = 1 to 2175 took 27 minutes on eight
  threads of a laptop.  A chain from `optimal.json`'s game of 1300 reached 2175 in 92 minutes on two.  Over those
  games the cost has waves but no growth that a fit resolves ([the GF(2) bound in the
  search](../notebook/the-gf2-bound-in-the-search-2026-10/README.md)).
* Past 2175 the cost grows again.  The search times of the games 2176 to 3440, made in a day on one 64-thread cloud
  machine for about $16, rise tenfold for about every 1,040 in n, with about a decade of scatter ([the chain to
  3440](../notebook/the-frontier-chain-to-3440-2026-10/README.md)).

### How the solver sees the game (the frame model)

The solver does not search the game move by move.  It splits the numbers into independent groups, the frames,
settles what it can before it searches, and then searches only over which spare numbers to play as moves.  This
decomposition is the *frame model*.  The [glossary](../../docs/GLOSSARY.md#the-frame-model) defines its words, and
[DESIGN.md, section 2](../docs/DESIGN.md#2-the-game-and-the-frame-model) gives it in full.

1. **Sources and factors.**  Every number above n/2 divides nothing else on the board, so it can only be a move,
   never tax.  It is a *source*, played unless setup finds it impossible (step 4).  Every other number is a *factor*:
   tax for a larger number, and sometimes a move itself.
2. **Maximal factors are enough.**  A move m is paid for by one of its *maximal factors* m/p, for a prime p that
   divides m, and a factor pays for one move.  Taking m removes every divisor of m still in the pot, but the model
   needs only the maximal ones.  If some proper divisor d of m is still in the pot, so is every f with d | f | m other
   than m, since whatever removed f would have removed d too; and one of those f is a maximal factor of m.  So m can be
   played exactly when one of its maximal factors is still in the pot.  This is the *lifting* argument of
   [src/numtheory.rs](src/numtheory.rs) and [THEORY.md](../../docs/THEORY.md), Theorem 1.
3. **Frames.**  Sources that share a factor are in one *frame*, with all the factors they can be paid with.  Frame X
   is *in front of* frame Y, and Y is *behind* X, when a number of Y has a maximal factor in X: Y's numbers are
   multiples of X's.  The frames in front are played first.  Each frame is a small game, *Taxman Mini*: a set of
   moves and a set of factors, in which the moves must be playable in some order.
4. **Setup.**  Before the search, setup simplifies each frame.  It drops the *impossible* sources, which cannot be
   played together with the larger ones.  It settles the *mandatory moves*, such as a source with one usable factor,
   which must take it.  And it counts the frame's *freedoms*, its factors minus its moves: how many factors it can
   spare.  A frame with no freedoms is *closed* and takes no part in the search.
5. **Numbering.**  Setup numbers the frames that have freedoms from the front, by *layer*: the number of prime factors
   of their sources.  All the sources of a frame have the same number of prime factors, and the frames in front of
   it have one fewer.  Frame 0 is the frames with freedoms whose sources have the fewest, frame 1 those with one more,
   and so on.  Frames with the same number are merged into one, so that a promotion always has one frame to go to.
   On every board the solver supports, frame 0 is every frame with freedoms that has no other in front of it, and
   frame 1 every one directly behind those.
6. **Promotions.**  What a game can still choose is which spare factors to *promote*: to play as moves instead of
   leaving them as tax.  Promoting a factor of frame h makes it a move of frame h − 1, one frame forward, and both
   frames must stay winnable.  The number's own maximal factors, which pay for it, lie in the frames directly in front
   of its frame, and the numbering puts those that have freedoms in frame h − 1.  That nothing further forward is
   needed is proven (below, "What optimal means").
7. **The score.**  A game of the model scores `in_the_bag`, what the game with no promotions scores, plus the sum of
   its promotions.  So the solver searches for the *promotion set* with the largest sum, P\*, by a branch and bound
   over which candidates to promote.  The moves of the game follow from its promotions (`frames::BoardFrames`; section
   6).

**An example: game 21.**  The board 1 to 21 has four frames.  They come from the Java's golden data,
`testdata/golden/n0021.json`, and the wiki's [walkthrough of game
21](https://github.com/bvchess/taxman/wiki/Walkthrough-for-N=21) shows them in pictures:

```
frame A   sources 19 11 13 17   factors 1           closed: 11, 13 and 17 are impossible (the 1 pays for one of them)
frame B   sources 14 15 21      factors 2 3 5 7     1 freedom (15 must take 5)             frame 0
frame C   sources 12 18 20      factors 4 6 9 10    1 freedom (20 must take 10)            frame 1
frame D   sources 16            factors 8           closed
candidates: 4 9 (factors of C that B can accept as moves and still play all its own)
```

B is in front of C, because 9, a factor of C, has the maximal factor 3 in B.  The game with no promotions plays every
source that is not impossible, and scores `in_the_bag` = 135 (19 + 14 + 15 + 21 + 12 + 18 + 20 + 16).  Promoting 9
takes it out of C's factors and makes it a move of B, and the game scores 144; promoting 4 instead would score 139.
Both frames stay winnable.

After a promotion the frames are *reduced*: a move left with one live factor must be paid by it, so the pair drops
out, repeatedly.  Here 9's only maximal factor is 3, so the pair (9, 3) drops out of B.  That leaves 21 only the factor
7 and 14 only the factor 2, so they drop out too.  In C, 18 is left with 6 and 12 with 4.  Each frame has spent its one
freedom, and the game plays the frames from the front: 19; 9, 15, 21, 14; 18, 12, 20; 16.

The search is over such choices.  Game 21 has 2 candidates, game 85 has 10, and game 1100 has 273.

Frames are first merged at game 20.  There the frames of 14 and of 15 share no factor, but neither has a frame with
freedoms in front of it, so both are frame 0.  Both sit directly in front of the frame of 12, 18 and 20, whose factor 6
has a maximal factor in each (2 and 3).  A promotion from that frame needs one frame to go to, so the two are merged
into one frame 0 with two freedoms.

### What "optimal" means

The solver's answer for game n is `in_the_bag` + P\*.  That is opt(n) given three things: the frame model is exact at
n, the search prunes nothing that could win, and, for some proofs, the game of n − 1 is optimal.

* **The frame model is exact, given statement U.**  The decomposition above once rested on five assumptions:
  * the greedy removal of impossible sources;
  * the mandatory moves of `unzip`;
  * freedoms = factors − sources;
  * closed frames taking no part in the search;
  * a promoted number paying tax only from the frame one forward.

  All five are theorems ([THEORY.md, "The frame model: what is proven and what
  remains"](../../docs/THEORY.md#the-frame-model-what-is-proven-and-what-remains)).  What is left is one statement
  about the game:

  > **U**: some optimal game plays every source with three or more prime factors that setup keeps.

  The sources that setup keeps are called U\*.  The games that play all of U\* are exactly U\* with a promotion set
  added (Theorem M).  Theorem M needs the numbering to be *clean*: every frame with freedoms directly in front of
  frame h is frame h − 1, as the numbering by layer is by construction.  Every optimal game plays the members of U\*
  with one or two prime factors (Theorem B).  So the heaviest promotion set gives opt(n) exactly when U holds at n.

  U is open in general.  It holds at every game the [prover](../noframe/README.md) has proven, since the prover
  proves opt(n) on the whole board, without the frame model.  Past those games, a proof by an exhausted search, a
  trivial board or a repeat assumes U at n, and `taxman certify` (section 3) checks it game by game.  A proof by the
  ceiling or by the prime formula does not assume U at n (section 5).  The results lines do not record U;
  [results/README.md](../../results/README.md) counts the committed games of each kind of proof, and [DESIGN.md, "What
  optimal means"](../docs/DESIGN.md#what-optimal-means) says what each kind needs of the model.
* **The pruning is exact.**  Every tactic, a switchable piece of the search, is proven or sound: none prunes a
  promotion set that could win ([DESIGN.md, section 5](../docs/DESIGN.md#5-tactics) grades each).  So a result rests
  on nothing but U, where its proof uses the frame model; the correctness of the code; and, where its proof says so,
  the game it was built from.
* **The game of n − 1 is optimal, where a proof rests on it.**  The game of n is built from the game of n − 1.  Its
  search starts from the previous game's promotions, and stops as soon as it reaches the proven ceiling n + score(n −
  1).  So a chain of games from n = 1 is proven game by game.  Section 5 says how each result records what it rests
  on.

## 2. Build, test, run

### Build

The solver runs on Linux or macOS: the engine uses POSIX signals and `flock`.  It needs a C linker and a Rust
toolchain of at least 1.89 (`rust-version` in `Cargo.toml`; the engine uses `File::try_lock`), and has no crates to
download.
The Python verifier, `solver/tools/check_results.py`, needs Python 3 and its standard library only.

```
cd solver/frame
cargo build --release              # about 100 seconds on an M1 Max; the binary is target/release/taxman
cargo build --release --examples   # also the ladder (scaling and progress harness), the probe and the ledger
cargo build --release --examples --features studies   # and the study tools: the tuning benchmark, the census of
                                   # the frames, the extended problem's chain and the wall regime's measurements
./target/release/taxman help
```

Cargo's commands run in `solver/frame`.  The commands of the other sections run from the top of the repository, where
`results/` is, and `taxman` in them is `solver/frame/target/release/taxman`.

The binary carries the table of optimal games built in: `results/optimal.json` as it was when the binary was compiled,
with the Java's games to 1000 and the chain's to 1300.  Every run checks its scores against that table, wherever the
binary runs, and no option names another.  A changed table takes a rebuild, which cargo does by itself when the file
has changed.

*Details.*  The study tools, the examples `tune`, `extended`, `structure` and `wall`, are behind the feature `studies`,
which is off by default.  So a build, `cargo test` and `cargo clippy --all-targets` do not compile them, and a command
that names one (`cargo build --release --example wall`) needs `--features studies` too.  The build is long for a
program of this size because the search is compiled once for every set width ([DESIGN.md, section
3](../docs/DESIGN.md#3-architecture), "Width dispatch").  `taxman bench` finds its two files in the repository: above
the working directory and, if they are not there, above the binary.

*Build options.*  The release profile uses fat LTO and `panic = "abort"`.  The `test` profile is optimised
(`opt-level = 2`) because the tests replay real games.  A build is generic for its target architecture, so its binary
runs on any machine of that architecture.  Tuning for the build machine's CPU is opt-in, for a machine that will run
only that binary: `RUSTFLAGS="-C target-cpu=native" cargo build --release`, as the cloud scripts do
([RUNBOOK](../docs/RUNBOOK.md)).  It gains nothing on the Apple M1, where it was measured, and is unmeasured elsewhere;
the generic x86-64 target has no hardware popcount, so measure there.  `solver/tools/pgo.sh` makes a profile-guided
build in `target/pgo/release`, with 0 to 2% fewer cycles on the M1 and the same answers and trees.  Its header says
how, and [DESIGN.md, section 10](../docs/DESIGN.md#10-performance) what it gains.  The probe, `examples/perf.rs`,
measures the hot paths.

### A first run: the games 1 to 22

The games 1 to 22 show every way the solver finds and proves a game but one.  From the top of the repository:

```
$ taxman solve 1-22 --threads 1 --progress 0 --out first.jsonl
n=1     score=0         trivial nodes=0               0.000 s  optimal ok  proof=trivial
n=2     score=2         prime   nodes=0               0.000 s  optimal ok  proof=prime_formula
n=3     score=3         prime   nodes=0               0.000 s  optimal ok  proof=prime_formula
n=4     score=7         trivial nodes=0               0.000 s  optimal ok  proof=trivial
...
n=17    score=93        prime   nodes=0               0.000 s  optimal ok  proof=prime_formula
n=18    score=111       search  nodes=0               0.000 s  optimal ok  proof=cap
n=19    score=113       prime   nodes=0               0.000 s  optimal ok  proof=prime_formula
n=20    score=124       search  nodes=1               0.007 s  optimal ok  proof=exhausted
n=21    score=144       search  nodes=1               0.015 s  optimal ok  proof=exhausted
n=22    score=166       repeat  nodes=0               0.000 s  optimal ok  proof=same_search of 21
```

Each line is one game: n, its score, how the solver found it, the search's steps, the seconds, a check against the
built-in table and the proof.  Section 4 reads a line field by field.  The methods:

* `trivial` (games 1, 4, 6 and others): the game has no candidate to promote, so the game with no promotions is the
  only one.
* `prime` (games 2, 3, 5 and every other prime): the game of n − 1 with its first move replaced by n, which is the
  *prime formula*.  For game 7, 7 + 15 − 5 = 17.
* `search` with the proof `cap` (game 18): the search's starting game reached the *ceiling* n + score(n − 1) =
  18 + 93 = 111, which no game of 18 can exceed, before it took a step.
* `search` with the proof `exhausted` (games 20 and 21): the search explored its whole tree.
* `repeat` (game 22): game 22 poses the same search problem as game 21, which settled it, so the solver takes 21's
  promotions without a search.

The fifth method, `known`, takes a game that the repository has already proven (section 3, "Known games").  "optimal
ok" says that the score equals the table's: a check, not a proof.

`--show` prints a game's promotions and moves.  Solved alone, game 21 is the example of section 1:

```
$ taxman solve 21 --show --progress 0
note: no previous game for n=21 (the game of 20 is neither in this run nor in the results file): solving it standalone, with no seed and no ceiling, which costs more; to chain, give --out with the earlier games, or --use-prev to take the known game of 20
n=21    score=144       search  nodes=1               0.001 s  optimal ok  proof=exhausted
  promotions: [9]
  moves: [19, 9, 15, 21, 14, 18, 12, 20, 16]
```

The note says that game 21 has no game of 20 to start from here, so the solver solves it *standalone*.  Finally, the two
verifiers check the file the first command wrote.  Each replays every game and derives every proof again (section 5):

```
$ taxman verify-results first.jsonl
first.jsonl: 22 completed games, n = 1..=22 (0 missing), 0 incomplete lines
  proof: trivial 10, prime_formula 8, cap 1, exhausted 2, same_search 1
  assumptions: none in 22 of 22 games
  scores at a proven ceiling, given the game of n-1: n + score(n-1) 9, prime formula 7
0 board sizes are in more than one file, with equal scores in all
checked in 185.3 ms
$ python3 solver/tools/check_results.py first.jsonl
...
all checks passed
```

### Tests

From `solver/frame`:

```
python3 -I ../tools/quick_check.py   # the quick check: formatting, lints and the tests, the long ones skipped, within time limits
cargo test                           # the same tests, a binary at a time
cargo test --release                 # the full suite
```

The tests run as many threads as the machine has CPUs.  None of these builds the study tools; `--features studies` adds
them and their tests.  [DESIGN.md, section 9](../docs/DESIGN.md#9-testing) says what the suite checks, what each test
takes and when each runs.  `tests/tripwires.rs` asserts exact node counts, so a change that alters the search tree on
purpose fails it; DESIGN.md says how to update the counts.

The ignored tests are for large machines and long runs:

* `cargo test --release --test engine -- --ignored many_threads`: 64 and 192 worker threads;
* `TXENGINE_STRESS=2000 cargo test --release --test engine -- --ignored stress`: random engine configurations through
  resumes;
* `TXENGINE_CONSERVATION=500 cargo test --release --test robustness conservation`;
* the benchmarks under `bench` in `src/` and `tests/engine.rs`.

## 3. Commands

`taxman help` lists every command and flag.  This section shows each command with sample output.  A *tactic* is a
switchable piece of the algorithm, set with `--tactic NAME=VALUE`, which is repeatable.  `taxman help` and [DESIGN.md,
section 5](../docs/DESIGN.md#5-tactics) give the names, values and defaults.  Every tactic is proven or sound, so
switching one changes how fast an answer comes, never the answer.  The other flags of `solve` are in the table at the
end of this section.

### Solve one game

```
$ taxman solve 480 --use-prev --force-search --show
n=480   score=73120     search  nodes=37              0.005 s  optimal ok  proof=exhausted
  promotions: [68, 138, 147, 153, 168, 175, 180, 182, 190, 195, 196, 198, 200, 207, 208, 210, 220, 222, 224, ...]
  moves: [479, 361, 437, 299, 403, 341, 473, 301, 469, 335, 445, 267, 471, 314, 478, 466, 458, 454, 446, ...]
$ taxman solve 480 --use-prev
n=480   score=73120     known   nodes=0               0.001 s  optimal ok  proof=known from the committed chain (strict.jsonl)
```

`--use-prev` draws on the *known games*, the games the repository has already proven (below, "Known games").  The known
game of n − 1 seeds the search and gives it a ceiling.  A game that a known source has proven is recorded as it is,
with no search: that is the second command.  The first, with `--force-search`, searches 480 from the known game of
479.  Without `--use-prev` a single game has no game before it, and is solved standalone (below, "Past n = 1300").

The node count of a multi-threaded search varies a little from run to run.  "optimal ok" means that the score equals
the table's, not that it is proven (section 5).

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

Each game of a range is built from the one before it.  Moving from one game to the next often leaves the search
untouched.  A game whose search problem an earlier game of the range or of the results file already posed and settled
is concluded at once with that game's promotions: it is a *repeat* (tactic `same_search`, method `repeat`, proof
`same_search`; section 5).  About half of the committed chain's games that need a search are repeats
([results/README.md](../../results/README.md#strictjsonl) has the counts).

With `--out FILE`, each result is appended to the file as soon as it is done: one JSON line per game, flushed and
synced.  A rerun skips every game the file already completed, whatever the tactics of the rerun, since the file's lines
keep the tactics they were made with and what they assume:

```
$ taxman solve 1-300 --out mine.jsonl --checkpoint-dir ckpt
$ taxman solve 1-300 --out mine.jsonl
skipped 300 games already done in mine.jsonl
```

### Past n = 1300

`optimal.json` stops at 1300, so a game above 1300 needs the game before it from the run or from the results file.
Either extend a copy of a file that has it, or give `--use-prev`, which takes the known game of n − 1.  That is the
committed chain's, which assumes nothing, or the bounds store's, where the store has proven the game.  The committed
`results/strict.jsonl` is a chain from n = 1 that assumes nothing, and a copy of it extended stays so.  The copy
matters: `--out` appends to the file, and `results/` must not be overwritten.

The games 1301 and 1302 are known, from the committed chain.  So to search them from the known game of 1300, as a
range past the table does, `--force-search` leaves them to their searches:

```
$ taxman solve 1301-1302 --use-prev --force-search --threads 2 \
      --out mine.jsonl --checkpoint-dir ckpt --node-limit 4000 --progress 4
n=1301 0.1s nodes 2979 (done) best 40800 queue 0 spilled 0 | eff 95.4% idle 4.4% lock 0.0% ...
n=1301  score=532272    search  nodes=2979            0.191 s  optimal unknown  proof=exhausted
n=1302 0.1s nodes 3723 (done) best 41436 queue 0 spilled 0 | eff 96.3% idle 3.6% lock 0.0% ...
n=1302  score=533559    search  nodes=3723            0.178 s  optimal unknown  proof=exhausted
```

Game 1301 is a prime, but `--force-search` also skips the prime formula.  Game 1302 started from 1301.  A node limit
would have stopped either with its checkpoint; the engine checks the limit every 1,024 steps of a thread.

A game whose predecessor is in neither the run nor the results file, and not taken with `--use-prev`, runs
*standalone*, with no seed and no ceiling: for example `taxman solve 1400`, or `taxman solve 480` without the flag.
`solve` says so on stderr (`note: no previous game for n=...`), and the result line has `"prev": null`.  The answer is
still correct, but costs about twice as much on the hard games, and many times more on the games that a chain settles
at once by reaching the ceiling ([the cost curve](../notebook/benchmarks-the-probe-pgo-and-scaling/cost-curve.md)).

### Known games

`--use-prev` draws on three sources of games that the run has not made.  All three are in the repository's
`results/`, and no option names another:

| source | holds | a game of it rests on |
|---|---|---|
| `optimal.json` (built in) | the table of optimal games, n ≤ 1300 | nothing but the table's word: the line records that it assumes the game optimal |
| `results/strict.jsonl` | the committed chain, each game with its proof | what its line assumes (nothing) |
| `results/bounds.jsonl` | the bounds the ledger found ([results/README.md](../../results/README.md#boundsjsonl)); a game whose best lower bound is its best upper bound is *proven* | statement U at the boards its upper bound rests on (`rests_on`), one assumption each |

Of a game that several sources hold, the solver takes the committed chain's, then the store's, then the table's.  What
it does with them:

* **A game that a source has proven is recorded with no search.**  Its line has the method `known`, the proof
  `known`, no `setup`, and a `known` field that cites the source (section 4; section 5, "Citing a known source").  The
  referee replays it like every game, and it is checked against the game before it (`score(n − 1) <= score(n) <= n +
  score(n − 1)`) and against the table.  Game n + 1 has the game of n for its ceiling and seed as usual, and a proof
  that rests on it, by the ceiling or the prime formula, assumes what it does.
* **The game before the range** is the known game of A − 1, if `--out` lacks it.  The `prev` of the first line says
  where it came from: `optimal`, `strict` or `bounds`.
* **A game that the store has not proven** still helps its search, whether the store has a lower bound, an upper bound
  or both with a gap between them.  The game of its best lower bound is a seed beside the game of n − 1, and the one
  that keeps more is used.  Its best upper bound is a ceiling next to the one that the game of n − 1 gives, and a
  search that reaches the lower of the two stops.  A search that stopped at the store's bound is proven by it: proof
  `known`, with `known` citing the bound, assuming U where the bound rests.
* `--force-search` leaves the games to their searches, and the store does not help them, as it leaves the prime
  formula and the repeats.  `regress --use-prev` takes the game before its range from the table only.

A chain from n = 1 with `--use-prev` takes its games from the committed chain as far as that goes, so it proves nothing
again there.  That is for a run that goes past the committed chain ([RUNBOOK, section
12](../docs/RUNBOOK.md#12-the-unattended-run-to-the-wall)), not for one that checks the build.

### A chain from n = 1

```
$ taxman solve 1-2175 --out strict.jsonl --checkpoint-dir ckpt --checkpoint-interval 60 --progress 60
```

Started at n = 1, a chain takes no game from `optimal.json`, which it only compares with, up to 1300, where the table
ends; a score that differs is a hard error.  So no proof in the file assumes anything beyond the frame model.  A run
that takes a game from `optimal.json` (`--use-prev`) records, in every result whose proof rests on it, that it assumes
that game optimal ("Known games", above).

The committed `results/strict.jsonl` is such a chain, made in parts ([results/README.md](../../results/README.md) has
the details of each).  Its games 1 to 2175 were made by this command, up to the paths and the thread count, and a run
of it reproduces their scores.  Its later games continue them, in runs on a cloud machine, the first started from a
copy of the 2175 lines.

### Games in flight

```
$ taxman solve 1-2175 --threads 64 --window 4 --out strict.jsonl --checkpoint-dir ckpt
```

With `--window K`, a range keeps up to K consecutive games in flight, so that a machine of many threads stays busy
where one game's search cannot use them all.  Game n + 1 is set up while n searches, and its search starts from n's
best game so far, without the ceiling.  It gets the ceiling, with n's final game as a seed, the moment n is concluded.
While the searches run, the threads move between them, to the oldest game that keeps them busy.

Games are still concluded, checked and recorded in order.  Every line has the score, proof, repeat and assumptions that
a range solved one game after another gives it.  The moves may be another optimal set, and the nodes and times differ
([DESIGN.md, "The window: games in flight"](../docs/DESIGN.md#the-window-games-in-flight) and "What a window changes in
the results").

The default window is 4, and with one thread the window is always 1, one game after another.  On 4 threads of the
laptop, a window of 16 makes 1..1300 about 16% faster, and the games past 2225, whose searches already keep 4 threads
busy, no slower ([round B](../notebook/round-b-the-chain-on-many-threads-2026-10/README.md)).  On 64 threads, a window
of 16 makes the games past 2225 20 to 32% slower than one game after another, and 4 is the best measured.  So 4 is the
default, and what `solver/tools/cloud/chain.sh` runs with ([scaling to 192
threads](../notebook/scaling-to-192-threads-2026-10/README.md)).

With the environment variable `TAXMAN_WINDOW_TRACE` set, the run prints the window's events on stderr: games set up,
searches started, threads moved, deferrals, hints and conclusions.

### Checkpoints, resuming, signals

With `--checkpoint-dir DIR`, a game's search writes `DIR/nNNNN.ckpt` every `--checkpoint-interval` seconds (default
60), and on any stop, atomically and synced.  The same command resumes it:

```
$ taxman solve 992 --use-prev --force-search --threads 1 --node-limit 4000 --progress 1 --out r992.jsonl --checkpoint-dir c992
n=992 0.3s nodes 4095 (stopped) best 24175 queue 12 spilled 0 | eff 94.3% ... pauses 1 (5.6%) max 14.0ms park 0.0ms | nogoods learned 356 ...
n=992   score=310675    search  nodes=4095            0.300 s  incomplete
stopped: the search for n=992 hit a limit before it finished; run the same command to resume it
$ taxman solve 992 --use-prev --force-search --threads 1 --node-limit 4000 --out r992.jsonl --checkpoint-dir c992   # resumes, stops again
$ taxman solve 992 --use-prev --force-search --threads 1 --out r992.jsonl --checkpoint-dir c992   # resumes and finishes
n=992   score=310690    search  nodes=14789           0.454 s  optimal ok  proof=exhausted
```

The last line is the third invocation of the game: its `nodes` cover all three, its seconds only the last.  The results
file then holds two incomplete lines and one completed line for game 992, and `taxman verify-results r992.jsonl` counts
them.

* **Signals.**  SIGINT or SIGTERM stops the search as a limit does: a final checkpoint, an incomplete result line, and
  the exit status 130 or 143 (128 + the signal).  Nothing is lost.  A second signal kills the process.  SIGKILL, a crash
  or a power loss loses at most one checkpoint interval.
* **One process per checkpoint directory, and per results file.**  The directory holds a `taxman.lock`, taken with
  `flock`, and a second process is refused, naming the first.  `--out` is locked the same way while the run appends to
  it.  Keep the directory on a local disk that survives a restart, not on tmpfs.
* **A checkpoint belongs to its game and its search tactics.**  A checkpoint made for another game, by a build with a
  different frame builder, or with other search tactics is refused, saying what differs.  Resume with the same
  `--tactic` flags, or delete the file.
* **Limits.**  `--time-limit S` and `--node-limit N` bound one game's search in one invocation, not the whole range.  A
  game that a limit stops gets an incomplete result line (`completed` false, with `best_so_far`), and the range stops
  there with the exit status 0.  With `--keep-going` it goes on (next subsection).
* **The wall.**  `--game-hours H` ends the range at the first game whose own search passes H hours.  The hours are
  summed over every invocation, and counted only while the game is the oldest of the window with its search running,
  not while it waits behind another.  Every search in flight stops with a final checkpoint, and nothing is recorded for
  that game, not even an incomplete line.  `solve` prints `wall at N after H hours` and exits with the status 4.  The
  game's clock is kept beside its checkpoint (`nNNNN.clock`), so the same command with a larger H goes on from both
  ([DESIGN.md](../docs/DESIGN.md#the-window-games-in-flight)).
* **A result is recorded before its checkpoint is deleted**, so a crash never loses a finished game.  At worst the
  checkpoint of a finished game is left over, and the next run deletes it.
* **Memory.**  A searcher's state is bounded.  The shared queue of work units spills to `DIR/spill` beyond `--mem-cap`
  units (default 1048576).  The runs of the committed chain never came near it
  ([results/README.md](../../results/README.md#strictjsonl) gives their largest queues).

### Keeping going past limits, and the sweep

```
$ taxman solve 3441-3600 --threads 1 --time-limit 180 --keep-going --out part.jsonl
```

With `--keep-going`, a game whose search a limit stops does not end the range.  Its best game so far is recorded as an
unfinished line (`completed` false, with `best_so_far` and the moves).  The next game goes on from that game as a seed
and nothing more, since it is not known to be optimal: it gets no ceiling n + score(n − 1) from it, no prime formula,
and no proof that rests on it.  Its line says so: its `prev` has no proof (section 5, "What a game had from n − 1").
What does not rest on the game before is as ever: an exhausted search is a proof, and so is a repeat of a completed
game of the file.  The games are solved one at a time (`--window` is 1).

* **A rerun** on the same `--out` passes over the games that the file holds only unfinished lines of, and solves the
  others.  It seeds the game after the last of them with its best game, the best of its lines.
* **`--retry`** searches the unfinished games again instead, each from its checkpoint if `--checkpoint-dir` kept one,
  as a second pass with a longer limit does.
* **`--retry-improved`** searches again only the unfinished games whose game before has more to give than it had when
  they were tried: it is proven now, or has a better best game than their seed, or they had none.  It passes over the
  rest.  The games it searches again go in increasing n, so that one it has just proven helps the next.
* **A signal** still ends the range, but with no line for the game it stopped, so that a rerun resumes it.
* **`--pass N`** records N in `options.pass` of every line the run writes, and changes nothing else.

**The sweep** (`solver/tools/cloud/sweep.sh FIRST LAST`, [RUNBOOK, section 13](../docs/RUNBOOK.md#13-the-sweep))
goes over a long stretch of games for as many proven games as the time allows, holes allowed.  It splits the stretch
into blocks of consecutive games of about equal expected work.  Each block is one `taxman solve --keep-going` on one
thread, with a limit on each game's search (`GAME_SECONDS`, 180).  Then passes of `--retry-improved` go over the
merged file, so that each game sees the game before it whichever block made it, until a pass changes nothing.

Its file, `explore.jsonl`, holds every line of every block and pass.  It is **not a chain**, and never goes into
`results/strict.jsonl`: some of its games have only an unfinished search, or none, and each proof in it rests only on
the games it holds.  Both verifiers accept it (section 5).

**The ledger** works on a whole stretch of games at once instead of in order.  For every game it keeps a lower bound
with a game that reaches it, and an upper bound, each with how it was derived, and a game is proven when its bounds
meet.  It tightens them where a proof looks cheapest (its *forecast*), in three ways:

* limited one-thread searches, P at once, each going on from its checkpoint, with its upper bound read from the
  checkpoint (the *dual bound*);
* searches under floors that step an open game's upper bound down, each costing about tenfold more than the one before
  and giving the game a line that forecasts its proof;
* the relations between games, both ways: the ceiling, the prime formula, repeats and monotonicity.

A finder of good games, a large neighbourhood search, adds lower bounds.  It is not sound, so its games count only once
the referee has replayed them.  The ledger never reads the known optima unless it is given proven games (`--proven
FILE`), so a run on a stretch that the chain has proven is a fair test, which `report --known` checks.

```
$ cargo run --release --example ledger -- run A-B --dir DIR --cpu-hours H --workers P
$ cargo run --release --example ledger -- report DIR --known FILE
```

The module comment of `examples/ledger/main.rs` has the flags and the files.  What the ledger finds is kept in the
bounds store, `results/bounds.jsonl` (`ledger export`; `run --bounds` starts from it), which has two verifiers of its
own ([results/README.md](../../results/README.md#boundsjsonl)) and which `taxman solve --use-prev` draws on too
("Known games", above).  The ledger's files are its own, not results lines ([DESIGN.md, "The
ledger"](../docs/DESIGN.md#the-ledger); the experiment: [the
ledger](../notebook/the-ledger-bounds-from-both-sides-2026-10/README.md)).

### All flags of `solve`

| flag | meaning |
|---|---|
| `--threads K` | worker threads (default: all CPUs; honours affinity masks and cgroup limits) |
| `--window K` | games of a range in flight at once (also for `regress`): game n + 1 is set up, and searched from n's best game so far, while n searches, and the threads move to the searches that keep them busy; games are still concluded and recorded in order, with the scores and proofs of a window of 1 (default 4; 1 is one game after another, and it is always 1 with one thread) |
| `--tactic NAME=VALUE` | set a tactic; repeatable |
| `--out FILE` | append result lines; skip the games the file completed; the game before the first comes from it |
| `--checkpoint-dir DIR`, `--checkpoint-interval S` | checkpoints (default every 60 s) |
| `--progress S` | seconds between progress lines on stderr (default 60; 0: none) |
| `--time-limit S`, `--node-limit N` | stop one game's search, keeping its checkpoint |
| `--game-hours H` | the wall: end the range at the first game whose own search has passed H hours in all, with the exit status 4 and nothing recorded for that game (above) |
| `--mem-cap UNITS` | work units kept in memory before spilling to disk |
| `--force-search` | search where the previous game, an earlier search or a known source would give the answer (skips `prime_shortcut`, `same_search` and the known games of `--use-prev`) |
| `--use-prev` | draw on the known games of `optimal.json`, the committed chain and the bounds store (above, "Known games"): a game one of them has proven is recorded with no search, citing it and what it assumes; the known game of n − 1 gives the ceiling and the seed when neither the run nor `--out` has it; the store's other games seed their searches with its lower bound's game and stop at its upper bound |
| `--keep-going` | a game that a limit stops does not end the range: its best game so far is an unfinished line and seeds the next game, which takes nothing that rests on its being optimal; the games are solved one at a time; a rerun passes over the unfinished games (above) |
| `--retry`, `--retry-improved` | with `--keep-going`, search again every unfinished game of `--out`, or only those whose game before now has more to give than when they were tried (above) |
| `--pass N` | record N as `options.pass` of every line written, for a script that goes over the games more than once; no other effect |
| `--show` | print each game's promotions and moves |

### The other commands

```
$ taxman regress --to 400                       # solve 1..400 in memory and check every score against optimal.json
solved 1..=400 (2 threads, tactics default) in 0.2 s
by method: trivial 13, prime 78, repeat 151, search 158, known 0
400 games match optimal.json, 0 are not in it
```

`regress` is the quick end-to-end check that the build still gives the Java's answers.  It solves `--from 1 --to 400`
by default, and takes the `--tactic` flags and `--use-prev`, which takes only the game before its range, from the
table.  A score that differs stops it, as it stops any run.

```
$ taxman verify-optimal                         # replay every game of optimal.json
verified 1300 games (n up to 1300, 367449 moves)
load+parse 12.8 ms, replay 18.6 ms
$ taxman verify-results results/strict.jsonl    # see section 5
$ taxman extend-optimal results/strict.jsonl --out /tmp/optimal.json     # the table plus the games it lacks
$ taxman tables 85 8 45                         # the maximal factors and composites of 8 and 45 on the board 1..=85
8: maximal factors [4]; maximal composites [16, 24, 40, 56]
45: maximal factors [9, 15]; maximal composites []
$ taxman bench --history /tmp/h.jsonl           # time the benchmark games 936 ... 1184 (standalone, no derivation)
```

* **`bench`** appends one JSON line per run to its history file: `solver/frame/bench/history.jsonl` of the repository,
  unless `--history FILE` names another (pass it to keep the checkout clean).  It takes the games past `optimal.json`'s
  from `results/strict.jsonl`, unless `--known FILE` names another results file.
* **The ladder** is the harness for hard games and for scaling over thread counts: `cargo run --release --example
  ladder -- --scaling --seed-opt --known results/strict.jsonl --seconds 600 1215`.  Its flags are documented at the
  top of `examples/ladder.rs`.
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
  written back byte for byte, FILE is written whole or not at all, and RESULTS can never be FILE.  This is how the
  games 1001 to 1300 were added to the Java's 1000.  The table stays at 1300, so the command adds all the committed
  file's games past it; on 2026-10-18 it said:

  ```
  $ taxman extend-optimal results/strict.jsonl --out /tmp/optimal.json
  added 2614 games (n = 1301..=3914); the table has 3914 games, n up to 3914; wrote /tmp/optimal.json
  stopped at n = 3915: the results end at n = 3914
  ```

  Check the new file with `taxman verify-optimal /tmp/optimal.json` before it replaces `results/optimal.json`.  The
  binaries built after that carry the new table.
* **`certify (A-B | --list FILE) --results FILE... --upper FILE... --out FILE`** proves, game by game, that statement
  U holds: that the recorded scores of A..B that rest on the frame model are opt(n) on the whole board ("What optimal
  means", section 1).  The list FILE instead names the games, one n a line, in order.  For each game that the prover's
  barred bounds (`--upper`, `solver/noframe/data/upper_*.jsonl`) say needs it, it searches the extended problem, with
  the bounds' open members optional, from the recorded game.  The search ends `certified` (nothing beats the score) or
  `beaten`.  A game that beats the score is written to `OUT.counterexample.jsonl` too, and the run exits with the
  status 2.  `certify` takes `--threads`, the checkpoint flags and the limits of `solve`, and resumes like it.  The
  extended problem keeps the claims of `unzip` that Lemma M5⁺ allows, unless `--no-claims`, the formulation of the
  certificates made before it; a line records which.
* **`verify-certificates CERTS... --results FILE... --upper FILE... [--exact FILE...] [--threads K]`** checks the
  files that `certify` wrote, without changing them, line by line.  It says how far the proofs of the recorded scores
  reach: the prover's, the certificates' and the ceilings'.  [DESIGN.md, "Certificates of
  U"](../docs/DESIGN.md#certificates-of-u) has what they check and what a line holds.

## 4. Reading the output

### The summary line (stdout, one per game)

Five lines from different runs:

```
n=250   score=19533     search  nodes=34   0.003 s  optimal ok  proof=exhausted
n=1220  score=468662    search  nodes=1375952   8.209 s  incomplete
n=1299  score=531389    repeat  nodes=0               0.079 s  optimal ok  proof=same_search of 1298
n=1301  score=532272    prime   nodes=0               0.016 s  optimal unknown  proof=prime_formula  assumes the game of 1300 in optimal.json is optimal
n=4754  score=7106699   known   nodes=0               0.147 s  optimal unknown  proof=known from the bounds store (bounds.jsonl)  assumes statement U holds at n=4753
```

The fields, in order:

1. n;
2. the score, or for an incomplete game the best found so far;
3. the method: `trivial`, `prime`, `repeat`, `search` or `known`;
4. the search's nodes, which are steps ([glossary](../../docs/GLOSSARY.md#the-search));
5. the seconds the game took in this invocation;
6. how the score compares with `optimal.json`: `optimal ok`, equal; `optimal unknown`, the table has no game of n; or
   `incomplete`.  A score that differs is a hard error, which stops the run, naming the game;
7. the proof, with the game whose search a `same_search` repeats and the source that a `known` cites;
8. what the proof assumes, if anything (section 5).

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
 "nodes":2, "stats":{"promote_attempts":1,"promotions":10,"children":0,"match_bounds":2},
 "engine":{"splits":0,"checkpoints":0,"spilled_units":0,"max_queue":1},
 "setup":{"frames":4,"candidates":48,"max_total":11,"greedy_sum":1194,"cap":1312,
          "digest":"fe5e05eb67406e98"},
 "seconds":{"frames":0.0002,"greedy":0.00001,"search":0.0008,"search_all_runs":0.0008,"total":0.0015},
 "invocations":1, "threads":2,
 "tactics":"default",
 "tactic_switches":{"prime_shortcut":true,"same_search":true,"match_refine":1,"fix_forced":true,"nogoods":true,
                    "forget_nogoods":true,"nogood_period":50000,"nogood_keep":"none","nogood_limit":0,
                    "lagrange":false,"gf2":true,"symmetry":true,"force_window":"all","branching":"exact",
                    "strong_limit":"all","strong_depth":"all","deep_branching":"split","more_forced":true,
                    "prev_cap":true,"seed_incumbent":true},
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
| `method` | how the game was found: `trivial`, `prime`, `repeat`, `search` or `known` (taken from a known source with no search, `--use-prev`) |
| `completed` | false only if a limit or a signal stopped the search; such a line records an unfinished search, and its checkpoint is kept (a `--keep-going` run writes none for a signal) |
| `proof` | how optimality is established: `trivial`, `prime_formula`, `cap`, `exhausted`, `same_search` or `known` (section 5); null if incomplete |
| `reached_cap` | the promotions reach `cap`, where the search stops (a search started before its ceiling was known, in a window of games in flight, may have run to the end of its tree; DESIGN.md, "What a window changes in the results") |
| `assumes` | what the proof assumes beyond the frame model, here or in the games it rests on (n − 1, the game a repeat repeats, or the known source a `known` proof cites): for a game of n − 1 or of n taken from `optimal.json`, "the game of n in optimal.json is optimal"; for a bound of the bounds store, "statement U holds at n=4501" for each board its upper bound rests on; `[]`: nothing |
| `in_the_bag` | what the game with no promotions scores (null for the prime formula and known games, which set no board up) |
| `prev` | the game of n − 1 this one was built from: its score, where it came from (`run`, `results`, `optimal`, `strict`, `bounds`), a digest of its moves, its proof and what it assumes; null if there was none.  A `run` or `results` game with no proof is the best game of an unfinished search (`--keep-going`): a seed, proven by nothing (section 5, "What a game had from n − 1") |
| `same_search` | for a repeat, the earlier game of the file whose search it repeats: its n, the digest of the search problem they share, the problem's best promotion sum, that game's proof and what it assumes; null otherwise |
| `known` | a key that only a line citing a known source has (section 5, "Citing a known source"): `{"source":"optimal"}` or `{"source":"strict"}` for a game taken from `optimal.json` or the committed chain, or `{"source":"bounds","lower":{"rule":..,"source":..},"upper":{"rule":..,"source":..}}` for the lines of the bounds store whose bound of n is the score (`lower` null when a search found the game and the upper bound stopped it) |
| `nodes`, `stats` | the search's steps over all invocations, and the searcher's counters (`match_bounds` counts the node bounds computed, GF(2) or matching) |
| `engine` | this invocation's splits, checkpoints written, units spilled and largest in-memory queue |
| `setup` | null on a `trivial`, `prime` or `known` line, which set up no search; otherwise the frames and candidates found, `max_total` (the most promotions the freedoms allow), the greedy start, `cap` (the ceiling as a promotion sum, `n + score(n-1) - in_the_bag`; null on a repeat, which ran no search to stop), and `digest`, a digest of the search problem the setup poses (DESIGN.md, "Same search"), by which a later game finds this one to repeat |
| `seconds` | per phase, this invocation (`search_all_runs`: the search over every invocation of a resumed game) |
| `invocations` | how many invocations ran the search (a resumed game has more than one) |
| `threads`, `tactics`, `tactic_switches`, `limits`, `options` | what the last invocation ran with; `tactics` is `default` or the changes from the defaults, `options` the flags `--force-search` and `--use-prev` and, only where given, `keep_going`, `retry`, `retry_improved` (true) and `pass` (the number `--pass` was given) |
| `optimal_match` | true: the score equals `optimal.json`'s (a score that differs is a hard error, never written); null if the table has no game of n |
| `build` | the commit the binary was built from, and whether its sources differed from it |

This build reads schema 5 only: a results file of an older schema has to be made again.  A line is never rewritten.  A
resumed game that is stopped again adds another incomplete line, and the completed line comes last.

## 5. Why the answers can be trusted

Every completed game records how its optimality is established, its *proof*, and what that proof rests on beyond the
frame model, its *assumptions*.  The referee replays every game before it counts, and two verifiers that share no code
check every results file.

### Proof kinds

| proof | meaning | rests on the game of n − 1? |
|---|---|---|
| `trivial` | the game has no promotion candidates, so the game with no promotions is the only one | no |
| `exhausted` | the search explored its whole tree | no |
| `cap` | the search reached the ceiling n + score(n − 1), which no game of n can exceed: removing the move n from an optimal game of n leaves a game of n − 1 | yes |
| `prime_formula` | a prime n plays the game of n − 1 with its first move replaced by n, which is optimal if the game of n − 1 is (proved in `src/accelerate.rs`, at `prime_shortcut`) | yes |
| `same_search` | the game's search problem (frames, candidates, `max_total`, the factor graph among their numbers) is the one an earlier game m of the file posed, and m's proof (`exhausted`, `cap` or `same_search`) settled its best promotion sum; the game is m's promotions on this board (tactic `same_search`; [DESIGN.md](../docs/DESIGN.md#same-search) has the argument) | no: on the game m it repeats |
| `known` | a known source (`--use-prev`) holds the game, or an upper bound of it ended the search; the line's `known` field cites the source, and what the proof assumes follows from it (below) | no: on the source |

### Citing a known source

A line of proof `known` has a `known` field that says which source it rests on:

| `known` | the line | the check |
|---|---|---|
| `{"source":"optimal"}` | method `known`: the table's game of n, assumed optimal | the table has a game of n that scores the line's score |
| `{"source":"strict"}` | method `known`: the committed chain's game of n, as proven as its line is | the committed chain has a game of n that scores it; the line assumes what that line does |
| `{"source":"bounds","lower":{..},"upper":{..}}` | method `known`: a game that the bounds store has proven; the `lower` bound is the game played | the store has a lower line and an upper line of game n worth the score, by the `rule` from the `source` (the run that found it) that the field says |
| `{"source":"bounds","lower":null,"upper":{..}}` | method `search`: the search found the game and stopped at the store's upper bound, which it meets; `reached_cap` is false, since the ceiling of n − 1 is not what it reached | the store has an upper line of game n worth the score, as cited |

The score of the line is the cited value of game n.  The store has one line of a game, side and value, so the game,
the side and the score name the line; `rule` and `source` are that line's own, as `bounds.jsonl` has them.  The score
is optimal wherever the cited bound holds: a finished game that scores what an upper bound allows is the best.  A lower
bound is a game, and a game that the referee has replayed is a lower bound.

When the game before the range came from the committed chain or the store, the first line's `prev.source` is `strict`
or `bounds`, with that game's proof (`known` for the store's) and assumptions.  A proof that rests on such a game
inherits what it assumes, as for a game from `optimal.json`: a proof by the ceiling or the prime formula, or a repeat of
a search that stopped at the store's bound.  The `known` field, method and proof came after schema 5 was set, as names
that a line without them does without, so every file of schema 5 reads as it did.

The verifiers check that the game plays and scores the line's score, and that the cited lines are in the sources and
say what the line says.  They do not check the sources again: `ledger verify-bounds` and
`solver/tools/check_bounds.py` check the store, and the verifiers of results files check the committed chain.  Which
game the line plays is not compared with the source's.  So a file with `known` lines is checked against the sources in
the repository's `results/`, the defaults of `taxman verify-results`; `solver/tools/check_results.py` takes them as
`--committed FILE` and `--bounds FILE`.  A file that cites a source that was not given is refused.

A known game that is wrong would be caught three ways: by the chain, since a score outside `score(n − 1) <= score(n)
<= n + score(n − 1)` is an error; by the table, where it has the game; and by the next `ledger verify-bounds`.  A search
that finds a game above the store's bound stops the run, naming the game: either the store is wrong, or statement U
fails at a board that its bound rests on.

### Assumptions

What a game *assumes* is what its proof rests on beyond the frame model:

* A proof that rests on the game of n − 1 assumes what that game assumes, and so on down the chain.
* A repeat rests on the game it repeats instead, and assumes what that game assumes.
* A `known` proof rests on its source, and assumes what the source's game does.
* A game taken from `optimal.json` (`--use-prev`) is assumed optimal, since the table records nothing about how a game
  was found.  A proof that rests on its game of n − 1 assumes "the game of n − 1 in optimal.json is optimal", and a
  game of n taken from it "the game of n in optimal.json is optimal".
* A bound of the bounds store holds wherever statement U holds at the boards its upper bound rests on (`rests_on`).  A
  bound of the frame model at a board rests on that board, and a bound carried by a relation rests on what its source
  rests on.  A game that rests on such a bound assumes "statement U holds at n=4501" for each of those boards, and a
  game that rests on that game assumes the same.
* The committed chain's games assume what their lines do: nothing.

A game that assumes nothing is proven within the frame model, given that the solver is right.  `taxman solve` prints
what a game assumes in its summary line, and `verify-results` counts together the games that assume statement U at
some board.

What a proof rests on through the frame model, statement U at n (section 1, "What optimal means"), is not recorded
for the proofs that the frame model gives: an exhausted search, a trivial game and a repeat.  It holds at every game
the prover has proven; past those, it is not yet certified.

### What a game had from n - 1

A line says what it had from the game of n − 1 in its `prev`.  In `options.pass` it says which pass over the games
made it: a sweep goes over them more than once, and a run with no `--pass` has none.

| `prev` | what the game had from the game of n − 1 | what its proof can be |
|---|---|---|
| `null` | nothing: it was solved standalone (a block's first game, or the game before it is missing), with no seed and no ceiling (`setup.cap` null) | `trivial`, `exhausted`, or `same_search` on a completed game of the file |
| `source` `run` or `results`, `proof` null | the best game of an unfinished search of n − 1, as a seed, and no more: no ceiling (`setup.cap` null), no prime formula, nothing that rests on it | the same: never `cap` or `prime_formula` |
| `proof` given (or `source` `optimal`, `strict`, `bounds`) | n − 1 proven (or assumed, from `optimal.json`; or the committed chain's or the bounds store's game, `--use-prev`): the seed, the ceiling n + score(n − 1), the prime formula for a prime n, and repeats | any |

The game of n − 1 that a `results` line names is in the same file.  It is a completed game or, with no proof, an
earlier incomplete line with the score and moves it records.  A game that a retry searched again has several lines:
an incomplete one made when the game of n − 1 had less to give, or was missing, and a later one made when it had more.
Each says what it had.

So a file of a sweep, with its games that only an unfinished search is recorded for, reads this way:

* A completed game is proven by its `proof`, given what its `assumes` says.  A `cap` or `prime_formula` rests on a
  proven `prev`, a `same_search` on the game it repeats, and the others on nothing of n − 1.
* An unfinished game is a lower bound.
* Where a game and the one before it are both completed, the chain bounds hold between them.

### The verifiers

* **The referee.**  Every `solve` replays each game through `game::replay_finished`, an independent referee that
  shares no code with the solver's number theory: the game must be legal, be over, open with the largest prime, and
  score what is recorded.  `solve` also checks the arithmetic and the ceilings `score(n-1) <= score(n) <= n +
  score(n-1)` before the game counts.  A game that fails after it was found goes to `<results>.rejected.jsonl`, and
  the run fails.
* **`taxman verify-results FILE...`** reads results files without changing them.  It replays every game, confirms its
  score against `optimal.json`, and checks the chain from n − 1, each line's arithmetic and provenance, and that the
  files agree.  It checks that a line's fields agree with each other as the solver writes them, so that a hand edit
  cannot drop an assumption or change a proof.  It counts the games by proof and by what they assume.  For a repeat it
  sets the search problems of both games up again and compares them.  For a `known` line it checks the citation
  against the sources ("Citing a known source", above): the table, the committed chain and the bounds store of the
  repository, read as they are, since their own verifiers check them.  Opening a results file for `--out` runs the
  same checks, with the sources that `--use-prev` draws on.
* **Both verifiers accept a file with holes**, such as a `--keep-going` run's or the sweep's `explore.jsonl`, whose
  games may have no line or only incomplete ones.  They check each line against what it rests on in the same file:
  * a ceiling (`setup.cap`), or a `cap` or `prime_formula` proof, needs the game of n − 1 completed in the file;
  * a repeat needs the game it repeats completed on an earlier line;
  * an exhausted search needs nothing of n − 1;
  * a line built on the best game of an unfinished search names an earlier incomplete line of n − 1 with that score and
    those moves, scores no less than it, and has no ceiling and no such proof;
  * an incomplete line that records its game (`best_so_far`, the moves) is checked to be legal, to score the
    `best_so_far`, and to be no better than `optimal.json`'s score.

  The report names the games that only an unfinished search is recorded for.  A line whose game of n − 1 is not in the
  file at all is still refused.  `tests/results.rs` shows each case with both verifiers.
* **`python3 solver/tools/check_results.py [FILE...]`** is an independent verifier in Python: standard library only,
  and no code shared with the solver.  It derives every proof and assumption again, and also checks each score against
  `optimal.json`, the Franklín–Moniot upper bound and the best-known heuristic chain in `heuristics/results`.  With no
  arguments it checks `results/*.jsonl` but the store, `bounds.jsonl`, which is not a file of results.  It cannot
  rebuild the frame model, so it takes an `exhausted` line at its word.  Of a repeat it checks only what the lines show:
  the game repeated is earlier in the file and settles its search, and the recorded digests and promotions agree.  It
  checks a `known` line against the sources (`--committed FILE` and `--bounds FILE`, by default the committed chain
  and the store of `results/`), from the lines of the files alone.  It cannot play the store's promotion sets, so it
  does not check the digest of a game of n − 1 taken from the store.

On 2026-10-18, the committed chain gave, on four threads of the laptop:

```
$ taxman verify-results --threads 4 results/strict.jsonl
results/strict.jsonl: 3914 completed games, n = 1..=3914 (0 missing), 0 incomplete lines
  proof: trivial 13, prime_formula 541, cap 354, exhausted 1356, same_search 1650
  assumptions: none in 3914 of 3914 games
  scores at a proven ceiling, given the game of n-1: n + score(n-1) 1244, prime formula 540
0 board sizes are in more than one file, with equal scores in all
checked in 12724.7 ms
```

What the checks cannot catch is a game that is legal, finished and within the chain bounds but not optimal, when the
game of n − 1 is not optimal either.  That is what the proof kinds and the assumptions track, and why a chain from n =
1 matters.  [results/README.md](../../results/README.md#what-supports-the-scores) says what supports each committed
score: solves from scratch, a second build's chain, and what the games assume.

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

The step numbers are those of [DESIGN.md, section 7](../docs/DESIGN.md#7-per-game-pipeline-solver), which gives the Java
class each step replaces, and of the module comment of `src/solver.rs`, which gives the checks of each.

## 7. Glossary

The project's glossary is a page of its own, [docs/GLOSSARY.md](../../docs/GLOSSARY.md): the words of the game, the
frame model, the search, the bounds and the results, each with the one name the docs use for it.

## 8. Reading order of the code

The code, in the order to read it, which is the order of the module map in `src/lib.rs`.  Each file starts with a
module comment that is worth reading first, and section 6 above maps how the pieces fit, so keep it open.

1. `src/game.rs`: the rules as a referee, and `optimal.json`.
2. `src/numtheory.rs`: maximal factors and composites; and `src/playable.rs`, whether a set of numbers can all be played
   on the whole board, with nothing of the frame model (the oracle of the tests of the model, and the referee of
   `taxman certify`).
3. `src/bits.rs`: `Bits<W>`, the fixed-width bitsets of game numbers that everything below is generic over, and the
   set widths.  The module comment is enough at first.
4. `src/json.rs`: the small JSON reader and writer.  Skim it; it matters once you read results lines.
5. `src/frames/`: forming and simplifying the frames, numbering and merging them.  The module comment of `mod.rs` is
   the story (sources, factors, `unzip`, impossible moves, freedoms, numbering, merging) and builds `SearchFrames`;
   `board_frames.rs` has `Frame` and `BoardFrames`.
6. `src/model/`: `mod.rs` says what is where: `Frame`, `FrameSet` (`frame.rs`), `FrameGraph`, winnability and reduction
   (`peel.rs`), promotion (`promote.rs`), then conflicts (`conflict.rs`) and nogoods (`nogoods.rs`).
7. `src/accelerate.rs`: the shortcuts from the game of n − 1 (read the comment of `prime_shortcut` for the proof of the
   prime formula), and the last step from promotions to moves.
8. `src/matchbound.rs`: the matching bound; the module comment is the best explanation of it.  Its child
   `src/matchbound/lagrange.rs` is the Lagrangian bound over the nogoods, built on the same levels and repairs.
9. `src/gf2.rs`: the GF(2) bound, the default node bound.  The module comment explains the two frame matroids, how a
   node's are kept by contraction along the search path, and the forced members; `src/gf2/meet.rs` is their weighted
   intersection (the tableau, the augmenting paths, the weight split).  `docs/THEORY.md`, Theorem 11, is the proof.
10. `src/search/`: `mod.rs` explains the step and says what is where: `searcher.rs` (`TaxSearcher` and `Stepper::step`,
    whose phases are `check_node`, `choose_branch`, `make_child` and `check_child`), `refine.rs` (`Refiner`: conflicts,
    refinement; `strongest`), `lagrange.rs` (the Lagrangian bound in the search: where, forced members, refits),
    `setup.rs` (`prepare`), `tactics.rs`, `incumbent.rs`, and `header.rs` (the checkpoint header).  The
    `if let Some(p) = &mut self.probe` lines in `searcher.rs` are instrumentation (`diagnostics.rs`), which can be
    skipped.
11. `src/bounds.rs`: two small functions the Java has too, the limit on a node's further promotions and the greedy
    incumbent.
12. `src/engine.rs`: the protocol in the module comment (conservation, splitting, checkpoint, termination).
13. `src/tactics.rs`, `src/board.rs`, `src/solver.rs`, `src/results.rs`, `src/store.rs`, `src/known.rs`,
    `src/certify.rs`, `src/paths.rs`: the switchable tactics, a board prepared for a search, the per-game pipeline,
    provenance and verification, the bounds store's line and the relations between boards, the known games that
    `--use-prev` draws on, the certificates of `taxman certify`, and the default files.  The range driver is
    `src/solver/window.rs`, the window of games in flight, and its child `window/allot.rs` says how they share the
    threads.
14. `src/main.rs`, `src/cli.rs`, `src/commands.rs`: the command line.
15. Studies, skipped on a first read: `src/brute.rs` (exhaustive search, the test oracle) and `src/sacrifice.rs` (the
    sacrifice bound).

The map of the documents is [solver/README.md](../README.md#the-documents).
