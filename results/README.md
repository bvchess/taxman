# Results of the solver

This directory holds the project's results: a chain of optimal games, the table of optimal games, bounds on opt(n) for
the games past the chain, and the certificates of statement U.  This page says what each file holds, how far it reaches,
how it was made and what supports its scores.  The [glossary](../docs/GLOSSARY.md) defines the terms; the frame model
and statement U, on which the frame solver's scores rest, are in [THEORY.md, "The upper
part"](../docs/THEORY.md#the-upper-part), and the prover in [its README](../solver/noframe/README.md).

A results file has one JSON line per game, as `taxman solve --out` writes it.  The fields are described in
`src/results.rs` and in [the frame solver's reference](../solver/docs/REFERENCE.md#a-results-line---out-schema-5).  The
counts on this page are pinned by `tests/results.rs`, so changing a file means updating both together.

## The files

| file | holds | games | made by |
|---|---|---|---|
| `strict.jsonl` | the committed chain: an optimal game for every n, each built from the one before | 1–4547 | four runs of `taxman solve` ([below](#strictjsonl)) |
| `optimal.json` | the table of optimal games, built into the frame solver | 1–1300 | the Java solver for 1–1000, the chain for 1001–1300 ([below](#optimaljson)) |
| `bounds.jsonl` | the bounds store: bounds on opt(n) for the games from 3915 on, one line a bound | 3915–8191 | the ledger's exports ([below](#boundsjsonl)) |
| `certificates/` | certificates of statement U | 600–1300 | `taxman certify` ([below](#certificates)) |

How far each kind of proof reaches:

* **Every game of `strict.jsonl`** is proven within the frame model.
* **Games 2 to 2175** are also proven on the whole board by the prover, so their scores are opt(n) without the frame
  model.
* **Games 2176 to 4547** are proven within the frame model, which is exact given statement U.  U is not certified past
  2175, and `taxman certify` can check it game by game.  A game of them proven by the ceiling or the prime formula is
  opt(n) wherever the game before it is.

"What supports the scores", below, gives the evidence for each.

**`explore.jsonl` is not a chain, and never goes into `strict.jsonl`.**  The file of a sweep ([the frame solver's
README, "Keeping going past limits, and the
sweep"](../solver/frame/README.md#keeping-going-past-limits-and-the-sweep); `solver/tools/cloud/sweep.sh`) merges the
games of many blocks of consecutive games, each given a limited search.  So it has:

* games with only an unfinished search, which is a lower bound and no proof;
* games that no block reached;
* games built on the best game of an unfinished search, with no ceiling and no prime formula;
* several lines of one game.

Each completed game in it is proven by its own proof, given what it assumes, in the frame model.  The file is evidence
for those games alone, and a game that follows a hole may rest on nothing the chain would have given it.  It is not
committed here: a sweep's results go to a file of their own, with a notebook entry saying what they are.

## strict.jsonl

The chain of optimal games from n = 1 to n = 4547, and the file to build on.  Every game is built from the one before,
every line has its full provenance, and no game rests on `optimal.json`.  It has 4547 lines in schema 5, the completed
games in order of n, and no line of an unfinished search.  167 of its lines, all in 3984–4543, are games that the bounds
store had proven (proof `known`): the fourth run took each from the store with no search, and its line cites the
store's two lines ([the frame solver's reference, "Citing a known
source"](../solver/docs/REFERENCE.md#citing-a-known-source)).  The name is from the builds that had unproven tactics,
which the chain was made without; none is left, so every run from n = 1 is made without them.

Its games, by proof, as `taxman verify-results` and `solver/tools/check_results.py` report them:

| | 1–1300 | 1301–2175 | 2176–3440 | 3441–3914 | 3915–4547 | `strict.jsonl` |
|---|---|---|---|---|---|---|
| games | 1300 | 875 | 1265 | 474 | 633 | 4547 |
| proof `trivial` | 13 | 0 | 0 | 0 | 0 | 13 |
| proof `exhausted`: the search explored its whole tree | 416 | 314 | 448 | 178 | 150 | 1506 |
| proof `cap`: the search reached the [ceiling](../docs/GLOSSARY.md#the-game) | 115 | 77 | 119 | 43 | 57 | 411 |
| proof `prime_formula` | 211 | 115 | 155 | 60 | 55 | 596 |
| proof `same_search`: a repeat of an earlier game's search | 545 | 369 | 543 | 193 | 204 | 1854 |
| proof `known`: a game of the bounds store, taken with no search | 0 | 0 | 0 | 0 | 167 | 167 |
| score = n + score(n − 1) | 424 | 270 | 402 | 148 | 218 | 1462 |
| score = n + score(n − 1) − P (a prime n) | 210 | 115 | 155 | 60 | 75 | 615 |

Of its 1917 searched games, 1506 are exhausted and 411 at the ceiling.  The proofs at the ceiling were all decided
before the search explored a node but six: 686's took 29 nodes, and 2695's, 3052's, 3306's, 3796's and 3933's took
7,175, 2,826, 5,366, 5,530 and 3,093.  A repeat takes the promotions of the latest earlier game that posed the same
search problem: the game of n − 1 for 1647 of the 1854 repeats, and of n − 2 for the other 207, where n − 1 is prime.

*The proofs by range, in detail:*

* 1–1000: 311 exhausted, 92 at the ceiling, 168 prime, 13 trivial and 416 repeats.
* 1001–1231: 81 exhausted, 16 at the ceiling, 34 prime and 100 repeats.
* 1232–1300: 24 exhausted, 7 at the ceiling, 9 prime and 29 repeats.  Its 69 games are 31 searches, 29 repeats and the
  primes 1237, 1249, 1259, 1277, 1279, 1283, 1289, 1291 and 1297.
* 1301–2175: 314 exhausted, 77 at the ceiling, 115 prime and 369 repeats: 391 searches, 369 repeats and 115 primes.
* 2176–3440: 448 exhausted, 119 at the ceiling, 155 prime and 543 repeats: 567 searches, 543 repeats and 155 primes.
* 3441–3914: 178 exhausted, 43 at the ceiling, 60 prime and 193 repeats: 221 searches, 193 repeats and 60 primes.
* 3915–4547: 150 exhausted, 57 at the ceiling, 55 prime, 204 repeats and 167 from the store: 207 searches, 204 repeats,
  55 primes and 167 games of the store.

### Games with several optimal sets

Several games have more than one optimal set of moves, and which one a run finds depends on the order of its search.
So scores are reproducible, but move lists need not be.  Each chain that found a set proved its game optimal, so each
set is optimal.  `tests/results.rs` pins the games where this file differs from the table.  The sets of this file
differ from the earlier files' in these games:

* **539, 684–687 and 770–773**, which the prover lists with two or three optimal sets each ([the prover's
  README](../solver/noframe/README.md#5-is-the-optimal-set-unique) has the exchanges).  This file has `optimal.json`'s
  set of moves in each of them but 539 and 770–773, where the chain found another optimal set.  The earlier chains of
  commits 243af9b and 271c817 had the table's sets at 539 and 770–773 and another at 684–687.
* **1215**, where `optimal.json` and every earlier chain have {312, 450} and this file {342, 420}: an exchange of two
  numbers of the same sum, 762.  The prover enumerates optimal sets only to n = 989.
* **1221–1224**, where the chain of commit bd9bb5d and the cost-curve study's chain found {364, 456, 544}, and this
  file, `default.jsonl` and the chains of 243af9b and 271c817 have {348, 408, 608}.  It is an exchange of three
  numbers of the same sum, 1364, that starts at 1221 and is carried by 1222, 1223 and 1224.  The prover describes
  exchanges like it, which first appear at game 425.
* **1560–1563**, where the GF(2) study's chain has {510, 594, 620, 624, 660} and this file {476, 528, 616, 644, 744}:
  an exchange of five numbers of the same sum, 3008, found by the search of 1560 and carried by the three repeats after
  it.

### How the file was made

It was made in four runs, each a continuation of the one before:

| games | where | build | git has the file before the run as |
|---|---|---|---|
| 1–2175 | one invocation, which nothing stopped, on a laptop | commit feba6d0 | (the earlier files, below) |
| 2176–3440 | appended to a copy of those lines on a cloud machine | commit 55287ff | `git show 4f1c35b:rust/results/strict.jsonl`, whose 2175 lines are this file's first 2175, byte for byte |
| 3441–3914 | appended from a run on a cloud machine of the same kind | commit eb84a6f | `git show b23f497:rust/results/strict.jsonl`, whose 3440 lines are this file's first 3440, byte for byte |
| 3915–4547 | appended from a run on a cloud machine of the same kind, which took the games before from this file and the bounds store (`--use-prev`) | commit ca357bf | `git show c7d81c2:rust/results/strict.jsonl`, whose 3914 lines are this file's first 3914, byte for byte |

Each build was of a clean tree, which every line records.  Every line records the `default` tactics: no tactic is
unproven any more, so none is switched off.

**The first run, games 1–2175.**  The command was, up to the paths, `taxman solve 1-2175 --threads 8 --out
strict.jsonl --checkpoint-dir ckpt --checkpoint-interval 60 --progress 0`, run once to the end on an Apple M1 Max
(10 CPUs) with nothing else running.  Every line records the build of commit feba6d0 (`dirty` false), 8 threads, no
`--time-limit` or `--node-limit`, and the `default` tactics: the GF(2) bound, `gf2`, on, and the Lagrangian bound over
the nogoods, `lagrange`, off.  No game was resumed (`invocations` is 1 in every line of 1–2175), and none was built from
a results file or from `optimal.json` (`prev.source` is `run` from 2 on).  The [entry on the
run](../solver/notebook/the-committed-chain-to-2175-2026-10/README.md) adds what it showed.

*Details.*  The checkpoint interval is not recorded, but the `engine.checkpoints` counts imply about 60 s: 1 each for
2170 and 2172, the only games that searched for more than 60 s.  The run took 1638.6 s of wall time and 10,890 s of CPU
(user time, timed from outside; the lines record each game's own seconds).  The searches add up to 1450.5 s
(`seconds.search_all_runs`) and 48.4 M nodes:

* 28.4 s and 1.08 M nodes for 1–1300: 7.5 s for 1–1000, 11.8 s for 1001–1231 and 9.1 s for 1232–1300;
* 1422.1 s and 47.4 M nodes for 1301–2175: 681.3 s for 1301–1600, 396.8 s for 1601–2000 and 344.1 s for 2001–2175.

The slowest games were 2172 (73.1 s, 1.31 M nodes), 2170 (68.5 s), 1475 (50.1 s, 1.92 M nodes, the most of any game),
1472 (37.8 s) and 1430 (35.0 s); the next is 1521 at 34.3 s.  The queue of work units never held more than 178, and
none spilled to disk.

The cost of the first 1300 games fell with the tactics that came after the earlier chains.  The third earlier chain
took 1022.5 s of search and 332.0 M nodes, on 4 threads of a machine shared with four other jobs, which makes its times
pessimistic; the second took 1052 s, and the first 3359 s.  Three tactics account for it:

* `same_search` concludes 545 of those games as repeats of an earlier game's search ([the
  notebook](../solver/notebook/same-search-2026-10/README.md));
* the Lagrangian bound over the nogoods makes the searches that remain shorter ([the
  notebook](../solver/notebook/a-lagrangian-bound-over-the-nogoods-2026-10/README.md));
* the GF(2) bound, which this is the first committed chain made with, makes them shorter again ([the
  notebook](../solver/notebook/the-gf2-bound-in-the-search-2026-10/README.md)).

**The second run, games 2176–3440.**  The command was `solver/tools/cloud/chain.sh 4095`, which runs `taxman solve
1-4095 --window W --out strict.jsonl --checkpoint-dir ckpt --checkpoint-interval 300 --progress 60` on all the CPUs.  It
ran on a copy of the first run's 2175 lines, so that the first game it solved was 2176, and its 24-hour deadline
stopped it after game 3440 (status 124, with a final checkpoint written).  Every line of 2176–3440 records the build of
commit 55287ff (`dirty` false), 64 threads, no `--time-limit` or `--node-limit`, and the `default` tactics.  The [entry
on the run](../solver/notebook/the-frontier-chain-to-3440-2026-10/README.md) has its story and cost, and [the
RUNBOOK](../solver/docs/RUNBOOK.md) the procedure.

*Details.*  The machine was one `c8g.16xlarge` (64 vCPUs of Graviton4, 124 GiB) on spot in us-east-1d, at $0.6752 an
hour, in two instances.  The first ran from 2026-10-15 04:05:42Z with a window of 16 games in flight (`W`) to game
3310, and was stopped on purpose.  The second, restored from the saved state, ran with a window of 4 from 10:40:41Z to
the deadline.  The run cost about $16.46 in all.

* The window is not in the lines, since a window changes neither a score nor a proof ([the frame solver's
  README](../solver/frame/README.md#games-in-flight)).  `invocations` is: it is 1 in every line of 2176–3440 but 3311,
  3312 and 3315.  Those games were in flight when the first instance stopped, and the second took them up from their
  checkpoints (2 each).
* The game of n − 1 came from the results file, not from the run, for the first game of each instance: `prev.source` is
  `results` for 2176 and 3311.  The lines it read assume nothing, and so do these.
* The 1265 games are 567 searches (448 exhausted, 119 at the ceiling), 543 repeats and 155 primes.  The searches took
  17.73 G nodes, of which 70 games of 30 M nodes or more took 16.34 G.  The dearest were 3440 (3.72 G nodes, 18,184 s or
  5.05 h on 64 threads), 3425 (1.23 G, 3.5 h), 3424 (1.30 G, 2.0 h), 2346 (1.14 G) and 3430 (0.80 G).  The patches of
  dear games are the 2340s, 2375–2388, 3102–3112, 3174–3176, 3300, the 3330s and 3399–3440.
* The search seconds of the 448 exhausted searches rise tenfold per about 1,040 in n, by a least-squares line through
  log10 of `seconds.search_all_runs`: 7 s at 2200, 97 s at 3400.  The scatter around it is about one decade (sd 1.0),
  concentrated in the patches.
* The queue of work units never held more than 7,298, and none spilled to disk.
* The run was left in the middle of game 3441, at 626 million nodes after 5.9 hours, with 3444 also begun.  Their
  checkpoints (format 4) are not part of the results, and the run left no line for either; the third run made them.

**The third run, games 3441–3914.**  The command was `taxman solve` from n = 1, with a window of 4 games in flight on
64 threads of one `c8g.16xlarge` at spot, with the build of commit eb84a6f.  It was launched on 2026-10-17 at 00:15 PDT
and stopped on request on 2026-10-18 at 18:58Z, in the middle of game 3915, which has no line.  It solved games 1–3440
again first, with this file's scores in all (checked), and only its lines from 3441 on were kept.  So the lines before
them are the second run's, unchanged, and the game of 3440 that line 3441 names (`prev`, by digest) is this file's.
Every kept line records that build (`dirty` false), 64 threads, one invocation, no `--time-limit` or `--node-limit`,
the `default` tactics, and a `prev.source` of `run`, 3441's too.  The [entry on the
run](../solver/notebook/the-run-to-the-wall-2026-10/README.md) has its story.

*Details.*  The 474 games are 221 searches (178 exhausted, 43 at the ceiling), 193 repeats and 60 primes.  The searches
took 21.28 G nodes and 49.0 hours of game time; the games overlap in the window.  107 games of 30 M nodes or more took
20.65 G of the nodes.  The dearest were 3441 (0.79 G nodes, 9,973 s or 2.77 h), then 3864, 3807, 3885 and 3744 (0.68,
0.68, 0.67 and 0.50 G).  By time, after 3441, they were 3888 (5,271 s) and 3444 (4,687 s).  The queue of work units
never held more than 5,815, and none spilled to disk.

**The fourth run, games 3915–4547.**  The command was `taxman solve 1-8191 --use-prev`, with a window of 4 games in
flight on 64 threads of one `c8g.16xlarge` at spot, with the build of commit ca357bf; `solver/tools/cloud/wall.sh`
launched it ([RUNBOOK, section 5](../solver/docs/RUNBOOK.md#5-the-unattended-run-to-the-wall)).  With `--use-prev` it
took games 1–3914 from this file and each game the bounds store had proven from the store, with no search, and
searched the others, from 3915 on.  Its own file started with `known` copies of games 1–3914.  The lines of games
3915–4547 were appended to this file, byte for byte; those copies and the run's three incomplete lines (of games 3940,
4340 and 4548) were left out.  Every kept line records that build (`dirty` false), 64 threads, no `--time-limit`
or `--node-limit`, the `default` tactics and the option `use_prev`.  The first line, 3915, records its game of 3914 as
`known`, the run's copy of this file's line; both verifiers accept that record, because the score, the moves and the
assumptions are the line's ([RUNBOOK, section
4.4](../solver/docs/RUNBOOK.md#44-verifying-and-bringing-the-results-home)).

*Details.*  The run was launched on 2026-10-18 at 12:27 PDT, continued with `wall.sh launch --continue` when its first
budget of 36 instance-hours ran out, and ended on 2026-10-20 at 05:31 PDT on its budget of 40.5, in the middle of game
4548, which has a checkpoint and no completed line.  Four instances ran the solver, for $27.32: spot interruptions
replaced the first two, and the budget ended the third, which `--continue` took up with a fourth.  A fifth, the first
try of `--continue`, idled for about 25 minutes (about $0.30) on the old layout of the scripts.  [The notebook
entry](../solver/notebook/the-run-to-5000-2026-10/README.md) records the run.

* `invocations` is 1 in every kept line but 3940, 3942, 4340, 4431, 4432 and 4433.  Those games were in flight when an
  instance stopped, and the next took them up from their checkpoints (2 each).  `prev.source` is `results` for 3940,
  4340 and 4431, the first game of each instance after the first, which read the game of n − 1 from the results file,
  and `run` for the others.
* The 633 games are 207 searches (150 exhausted, 57 at the ceiling), 204 repeats, 55 primes and 167 games of the store:
  3984–3999, 4011–4014, 4053–4055, 4080–4094, 4179, 4300–4304, 4323–4331, 4347–4370, 4374–4375, 4446–4529 and
  4540–4543.  Each cites its two lines of the store, from the runs `ledger-frontier-stage1`, `ledger-frontier-stage2`
  and `ledger-survey-stage3`.
* The searches took 29.75 G nodes and 64.5 hours of game time; the games overlap in the window.  115 games of 30 M nodes
  or more took 29.24 G of the nodes.  The dearest by nodes were 3940 (1.55 G nodes, 10,725 s or 2.98 h), 3930, 4225,
  3942 and 3936 (1.40, 1.37, 1.20 and 1.16 G); by time, 3942 (15,942 s or 4.43 h), 4433 (11,032 s), 3940 and 3939
  (9,033 s).  The queue of work units never held more than 7,380, and none spilled to disk.

**The earlier files.**  This is the fourth file of its name.  The earlier three were chains to 1300, and git has them:

* the first, of commit bd9bb5d, in schema 3: `git show cd3d32f:rust/results/strict.jsonl`;
* the second, of commit 243af9b, in schema 4: `git show 2487671:rust/results/strict.jsonl`;
* the third, of commit 271c817, in schema 5, on 4 threads: `git show 0133afe:rust/results/strict.jsonl`.

All three have this file's scores in games 1–1300, and the second and the third its proofs and methods too.  The first
was made before the tactic `same_search` existed, so it searched the 545 games that this file concludes as repeats.
Their moves differ from this file's in a few games ("Games with several optimal sets", above).

**Other files checked with it.**  Three other files were checked with `strict.jsonl`, and agree with it ("What
supports the scores", below):

* `default.jsonl` (1001–1231, schema 1), the first run past 1000, made with the unproven tactics `reuse_prev` and
  `revise_freedoms` on.  It is gone from the tree; `git show f72c332:rust/results/default.jsonl` has it.  It had 34
  prime, 78 exhausted and 119 proofs by `reuse_prev`, and 19 games resting on nothing unproven.
* The cost-curve study's `solver/notebook/benchmarks-the-probe-pgo-and-scaling/chain.jsonl` (1001–1231, schema 1,
  commit e710d16), a chain with those tactics off.  It has 34 prime, 127 exhausted and 70 at the ceiling, all 231
  resting on nothing unproven.
* The GF(2) study's `solver/notebook/the-gf2-bound-in-the-search-2026-10/chain.jsonl` (1301–2175, schema 5, commit
  716ebd9, 2 threads), a chain from `optimal.json`'s game of 1300.

The two chains stay as their studies' data.  The verifiers read the second beside `strict.jsonl`, and no longer read
the first.

## What supports the scores

"Optimal" here means optimal within the frame model, given that the solver is right.  That is opt(n) wherever statement
U holds ([DESIGN.md](../solver/docs/DESIGN.md#what-optimal-means)), and ["The files"](#the-files), above, says how far
each kind of proof reaches.

Two verifiers check the files.  From the top of the repository:

    taxman verify-results results/strict.jsonl
    python3 solver/tools/check_results.py results/strict.jsonl

The first is the frame solver's own.  It also runs on any results file that `solve --out` opens, and in
`tests/results.rs`.  The second is an independent verifier in Python, which shares no code with the solver; it also
checks every score against the Franklín–Moniot upper bound and the best-known heuristic chain in `heuristics/results`.
Both do the same checks:

* they replay every game: legal, scoring what is recorded, with no legal move left, and opening with the largest prime;
* they confirm each score against `optimal.json`, and check score(n − 1) ≤ score(n) ≤ n + score(n − 1);
* they check the arithmetic of each line and the predecessor it names, and that the files agree;
* they derive each line's proof kind, and what it assumes, by the rules in `src/results.rs`.

Both pass `strict.jsonl`, in about 10 s (42 s of CPU, on several threads) and 3 s on the laptop.  [DESIGN.md, section
8](../solver/docs/DESIGN.md#8-results-and-the-trust-model) defines the proof kinds and the assumptions.

What that evidence amounts to:

1. **Arithmetic certificates.**  A score at the [ceiling](../docs/GLOSSARY.md#the-game), n + score(n − 1), is optimal
   whenever the game of n − 1 is, whatever found it.  So is a score equal to n + score(n − 1) − P for a prime n, with P
   the largest prime below n (the [prime formula](../docs/GLOSSARY.md#the-game), proven in `src/accelerate.rs`).  2077
   of `strict.jsonl`'s games have such a score: 1462 at n + score(n − 1), among them its 411 proofs at the ceiling, and
   615 prime ones.
2. **Exhaustive searches.**  `strict.jsonl`'s 1506 `exhausted` games are proven by a search that explored its whole
   tree, as the solver reports it.  The rest of the file is its 411 proofs at the ceiling, its 596 prime games, its 13
   trivial ones (no promotion candidate, so there is only one game), its 1854
   [repeats](../docs/GLOSSARY.md#results-and-trust), which both verifiers check against the earlier game
   (`src/results.rs`), and its 167 games of the bounds store.  A game of the store is a game the referee replays at the
   score of an upper bound the store holds ([below](#boundsjsonl)).
3. **Standalone solves.**  Some games were solved again on their own, with no seed and no ceiling:
   * `solver/notebook/benchmarks-the-probe-pgo-and-scaling/calib.jsonl` holds 197 standalone searches, one for every
     game of 1001–1231 that is not prime (`ladder --no-seed --no-cap --tactic revise_freedoms=off`, 8 threads, commit
     e710d16; see `solver/notebook/benchmarks-the-probe-pgo-and-scaling/cost-curve.md`).  All of them finished, and
     each found the promotion sum of the score recorded here.  These proofs rest on no other game.  So every game of
     1001–1231 that is not prime is proven without the chain, and every prime one by the prime formula from a game so
     proven.
   * There is no such solve for 1232–1300.  Of those 69 games, 24 are proven by an exhausted search of their own, and
     the 7 proofs at the ceiling, the 9 prime ones and the 29 repeats rest on the game before them or on the game they
     repeat, so on the chain from n = 1 (see 5).
   * Past 1300, `solver/notebook/benchmarks-the-probe-pgo-and-scaling/bands.jsonl` has ten games of 1302–1352 solved
     standalone (the matching path, 8 threads), each finishing with the promotion sum recorded here.  The rest of
     1301–4547 is the chain's: 1090 exhausted searches of their own, 296 proofs at the ceiling, 385 prime ones and 1309
     repeats that rest on the games before them, and 167 games of the store.
4. **Agreement between builds and files.**
   * n ≤ 1000: `strict.jsonl`'s scores are `optimal.json`'s, all 1000 (each line's `optimal_match` is true;
     `tests/results.rs` checks it).  797 of the games are `optimal.json`'s move for move, and 995 are its sets of moves:
     all but 539 and 770–773 (above).
   * n = 1001–1300: `optimal.json` holds the games of the chain of commit 243af9b, copied move for move by `taxman
     extend-optimal`, and `strict.jsonl`'s games are those, move for move, but 1215 (above; `tests/results.rs` checks
     it).  The lines say `optimal_match` true here too.  When the run was made the table held those games, so this is a
     second run reproducing the first's scores, not a comparison with another source.
   * n = 1301–2175: `optimal.json` has no game here (the lines say `optimal_match` null), so the comparison is with
     another run of the same search.  The GF(2) study's chain was made from `optimal.json`'s game of 1300
     (`--use-prev`) by the build of commit 716ebd9, on 2 threads in three invocations, and reached 2175 from 1301 in 92
     minutes of game time.  Its 875 games have this file's scores, proofs and methods in every game, and its moves in
     all but 1560–1563 (above).  Only its game of 1301 assumes anything: that `optimal.json`'s game of 1300 is optimal.
     `taxman verify-results results/strict.jsonl solver/notebook/the-gf2-bound-in-the-search-2026-10/chain.jsonl` (93 s)
     and `solver/tools/check_results.py` on both accept them, and find 875 games in both files, with equal scores in
     all.
   * n = 2176–3440: `optimal.json` has no game here either, and no other chain reaches far.
     `solver/frame/testdata/c8g.chain-2176.jsonl`, the Graviton calibration's chain (64 threads, one game after
     another, the build of commit 407645f, 15 minutes), has the games 2176–2276, 101 in all.  A rehearsal of the second
     run, made on the same machine type just before it started (the build of commit 55287ff, 64 threads, a window of
     16, stopped and killed and resumed on purpose; not committed), has 2176–2300, 125 in all.  Both have this file's
     scores, proofs, methods, promotions and moves in every game they share with it (checked on the final file).  Past
     2300 the evidence is the run's own: the verifiers on the whole file.
   * n = 3441–3914: no other chain reaches here either.  The evidence is the third run's own: it solved 1–3440 again
     before these games, with this file's scores in all of them (the lines it made of those games are not kept), and
     the verifiers pass the whole file.
   * n = 3915–4547: no other chain reaches here either.  The evidence is the fourth run's own, which took games 1–3914
     from this file and did not solve them again, and the verifiers pass the whole file.
   * 1001–1231: the scores of `strict.jsonl`, the cost-curve study's chain (commit e710d16, 8 threads, a chain with the
     unproven tactics off) and `default.jsonl` (the earlier solver, with unproven tactics on) are equal in all 231
     games.  `strict.jsonl`'s games are `default.jsonl`'s move for move but 1215, and the cost-curve chain's but 1215
     and 1221–1224: other games, with the same scores.
   * The earlier committed chains, of commits bd9bb5d, 243af9b and 271c817, have the same scores in all 1300 games.  The
     second's and the third's moves are the same as each other's, and differ from this file's at 539, 684–687, 770–773
     and 1215; the first's differ at those and at 1221–1224.
5. **Independence from `optimal.json`.**  `strict.jsonl` starts at n = 1, so no game of it is built from
   `optimal.json`, which is only compared with, up to 1300; a score that differs would be a hard error.
6. **The prover's proofs.**  The prover proves every n up to 2175 optimal without the frame model, with no frames or
   promotions ([the prover's README](../solver/noframe/README.md); [THEORY.md](../docs/THEORY.md), Theorem 10).  So
   `strict.jsonl`'s scores to 2175 do not rest on the frame model either: those 2175 are opt(n) on the whole board.  The
   games 1301–2175 were proven in the cloud run of 2026-10-14 and 15 ([the
   notebook](../solver/notebook/the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md)).
   The 2372 games of 2176–4547 are proven within the frame model, which is exact given U ([THEORY.md, "The frame model:
   what is proven and what remains"](../docs/THEORY.md#the-frame-model-what-is-proven-and-what-remains)).  `taxman
   certify` can check U there game by game, with the prover's barred bounds ([the prover's
   README](../solver/noframe/README.md#the-upper-part-game-by-game) has them to 3440).

The checks cannot catch a game that is legal, finished and within the chain bounds but not optimal, when the game of n
− 1 is not optimal either.  That is what the proof kinds and the assumptions track, and why the chain from n = 1 and
the standalone solves (3) matter.

## optimal.json

The table of optimal games, built into the frame solver.  Its games 1–1000 were made by the Java solver, with the
tactics `reuse_prev` and `revise_freedoms` on.  Its games 1001–1300 are the chain's, added by `taxman extend-optimal`:
first from the chain of commit bd9bb5d, then from that of commit 243af9b, which differs from it at 1221–1224 only.
`strict.jsonl` has the second's games but at 1215.

The file records no proofs and no provenance.  So a game taken from it (`--use-prev`) is assumed optimal, and the
results whose proofs rest on it say so: "the game of n in optimal.json is optimal".  A chain that assumes nothing
beyond the frame model starts at n = 1.  `strict.jsonl` is such a chain, and its games 1–1000 reproduce
`optimal.json`'s scores with no unproven tactic on, in 7.5 s of search in all on 8 threads, the slowest n = 992 at
0.7 s.

The table stays at 1300.  `taxman extend-optimal results/strict.jsonl` would add the 3247 games of 1301–4547.

## bounds.jsonl

The bounds store: the bounds on opt(n) that [the
ledger](../solver/notebook/the-ledger-bounds-from-both-sides-2026-10/README.md) finds, kept for good, for the games
from 3915 on.  A game is *proven* when its best lower bound equals its best upper bound, and the file is the record of
those proofs, which the lines of proof `known` in `strict.jsonl` cite.  The first `ledger export` made it, from stage 1
of the ledger past 4400 (2026-10-18), and each export since appends to it.

Its lines are bounds that hold.  A lower bound is the score of a finished game that the referee replays.  An upper
bound is the frame model's, carried between games by the lemmas of [THEORY.md, "The ledger: bounds between
boards"](../docs/THEORY.md#the-ledger-bounds-between-boards).

A line is a bound, not a game of `taxman solve`.  It is not in the schema of the results lines, and an export adds
no line of a game that `strict.jsonl` has.  It is not `explore.jsonl`'s kind of file either: that one holds the
sweep's games.

* `taxman solve --use-prev` draws on it: a game it has proven is recorded with no search, and its other games seed and
  end the searches ([the frame solver's README, "Known games"](../solver/frame/README.md#known-games)).
* `taxman verify-results` and `solver/tools/check_results.py` read it for the results lines that cite it (`known`).
  They look the cited lines up, and do not check the store, which its own verifiers do.

**Append-only.**  A line is never changed or removed.  A new line is added only when it improves the file's best bound
of its game and side: a lower bound above every earlier lower bound of the game, or an upper bound below every earlier
one.  So a second export of the same ledger appends nothing, and both verifiers refuse a line that improves nothing.

**The commands**, from the top of the repository.  The program is the example `ledger`, built in `solver/frame` on the
laptop with `cargo build --profile test --example ledger`; its module comment, `examples/ledger/main.rs`, has every
flag.

    ledger export RUN_DIR --bounds results/bounds.jsonl --source NAME [--checkpoints S3_PREFIX]
    ledger run A-B --dir DIR --cpu-hours H ... --bounds results/bounds.jsonl
    ledger verify-bounds results/bounds.jsonl
    ledger games results/bounds.jsonl > games.jsonl
    python3 -I solver/tools/check_bounds.py results/bounds.jsonl --games games.jsonl

* **`export`** appends the bounds of the run's `ledger.jsonl` that improve on the file's best, for the games that
  `results/strict.jsonl` does not have (`--strict` names another file).  It creates the file if there is none, and
  prints what it appended: the games, the lower and upper bounds that are the first of their game or improve one, and
  the games newly proven.  `--source` names the run, which every line records with the day.  `--checkpoints` is the
  place of the run's checkpoints, the prefix of `nN.ckpt` (for instance `s3://BUCKET/ledger-DATE/out/run/ckpt`), which
  is recorded on the dual bounds of the games not proven.
* **`run --bounds`** starts the run from the file's best bounds for the games of its stretch.  Each lower bound's game
  is replayed by the referee, each upper bound is taken as the file has it, and a game with both needs no map.  Its
  lower bound's game is its incumbent and the finder's input, as a proven game that is given is.  Run `verify-bounds`
  on the file first, since the run takes its upper bounds on their word.
* **`taxman solve --use-prev`** takes the upper bounds on their word too, and reads the same best bounds
  (`src/store.rs`, the one copy of the format).  A game whose best bounds meet is recorded as the game of its lower
  bound, replayed by the referee, with proof `known`, citing the two lines and assuming U at the games the upper line
  rests on.  A game with one bound, or a gap between two, seeds its search with the lower bound's game and stops it at
  the upper bound.  The verifiers of results files check that the cited lines exist and say what the results line says
  (rule, run, value).  The bounds themselves are checked by `ledger verify-bounds` and `solver/tools/check_bounds.py`,
  not again by them.

**The line.**  One JSON object, with the fields below and no others:

| field | on | meaning |
|---|---|---|
| `schema` | every line | 1 |
| `n`, `side`, `value`, `rule` | every line | the game, `lower` or `upper`, the bound on opt(n), and how it was derived |
| `from` | the relations | the game the bound came from |
| `from_value` | an upper bound by a relation; a lower bound whose game is cited | the value of the upper bound of `from` it was derived from; the lower bound of `from` whose game the line carries across |
| `bag`, `from_bag` | an upper bound by `repeat` | `in_the_bag` of `n` and of `from` |
| `rests_on` | an upper bound | the games at which statement U is assumed, ascending |
| `checkpoint` | a `dual` upper bound | where the checkpoint of the search that gave it is, if the export was told |
| `source`, `date` | every line | the run (`--source`) and the day of the export, `YYYY-MM-DD` |
| `promotions` or `moves` | a lower bound with its game | the game: the promotions it was made from, or its moves |

The rules of a lower bound:

* `start`, the standalone start incumbent of the game's map;
* `search`, a search's best game;
* `lns`, the finder's game;
* `given`, a proven game given to the run;
* and the relations: `monotone`, the game of n − 1 followed by n; `ceiling`, the game of n + 1 without the move n + 1,
  completed; `prime`, the game next to a prime with its opening exchanged; and `repeat`, the promotions of the game of
  another n with the same search problem, played on this board.

The rules of an upper bound:

* `root`, the GF(2) bound of the root;
* `dual`, the dual bound of a search that a limit stopped;
* `exhausted`, a search that explored its whole tree;
* `floor`, a search under a floor F (an incumbent worth F and no set) that completed having found nothing.  No promotion
  set weighs more than F (THEORY.md, Lemma F), so the bound is `in_the_bag` and F;
* `nogoods`, the Lagrangian bound over nogoods that were each checked to fail to promote, with the GF(2) bound inside,
  as the removed `wall nogoods` made it; nothing makes it any more;
* `given`;
* and the relations: `ceiling`, U(n) ≤ U(n − 1) + n; `monotone`, U(n − 1) ≤ U(n); `prime`, U(p) = U(p − 1) + p − q
  with q the largest prime below p, both ways; and `repeat`, U(n) − bag(n) ≤ U(m) − bag(m).

A line of 4501 might read:

    {"schema":1,"n":4501,"side":"upper","value":9051234,"rule":"ceiling","from":4500,"from_value":9046733,"rests_on":[4500],"source":"ledger-4400-8191","date":"2026-10-18"}

**A lower bound's game** is given in one of three forms, the most compact that is possible:

* A game made from promotions (a search's, the finder's, the start's, or a repeat of such a game) is its
  `promotions`, ascending, which `solution_for_promotions` turns into moves.  That takes about 330 bytes at n = 900
  and about 1.1 kilobytes at n = 3440, where the moves would take 9.
* A game that a relation (`monotone`, `ceiling`, `prime`) made from the game of another n is not written but cited.
  The line has `from` and `from_value` and no game, and its game is the lemma's construction applied to the game of
  the lower bound of `from` that is worth `from_value`, which a line before it gives.  A game of the chain is never
  cited, since a run need not have the chain.
* Any other game is its `moves`.

**What an upper bound rests on.**  Every upper bound is a bound of the frame model at some game: a root bound, a dual
bound, a completed search or a given game.  That is opt(n) where statement U holds at that game.  The lemmas carry it
between games without more, except the second part of Lemma L4, the bound on opt(n) − in_the_bag(n), which rests on U
at the game it is carried to ([THEORY.md, "In the ledger"](../docs/THEORY.md#the-ledger-bounds-between-boards)).
`rests_on` lists the games:

* a bound of the frame model has its own game alone;
* a relation has what the bound it comes from rests on, and a game of the chain that it cites counts as resting on
  itself;
* a repeat adds its own game.

U is certified for every n up to 2175 ([DESIGN.md, "What optimal means"](../solver/docs/DESIGN.md#what-optimal-means)),
so a bound whose list holds only games up to 2175 rests on no open statement.  A proven game is opt(n) wherever U holds
at the games of the `rests_on` of its upper bound.  A results line that rests on a proven game assumes that, one
statement a game (`"statement U holds at n=4501"`), in its `assumes`.

**Two verifiers**, as the results files have:

* **`ledger verify-bounds FILE`** (Rust; `examples/ledger/verify.rs`) checks every line:
  * its form, and that it improves on the lines before it;
  * that every lower bound's game replays under the referee (`replay_finished`: legal, over, opening with the largest
    prime) at the value of the line, a cited game made from the one it cites;
  * that every `root` bound is computed again and equal, and that every repeat's bags and search problems are computed
    again and equal to its source's;
  * that every relation's arithmetic holds against the line it cites, a line before it with a bound of that game at
    most `from_value` or a game of the chain, and rests on what that line rests on;
  * and that where the chain has a game, the file's bounds enclose its score.

  It reports the proven games, and lists the bounds it takes on their record: the `exhausted`, `dual` and `floor`
  bounds of searches and the `nogoods` bounds, which no check short of the search (or the fit) computes again, and the
  `given` games.
* **`solver/tools/check_bounds.py`** (Python, standard library only, run with `python3 -I`) is the second verifier of
  the games and of the arithmetic.  It shares no code with the Rust crate.  It loads the rules of the game from
  `solver/tools/check_results.py`, and writes out again the arithmetic of the lemmas and the constructions of the cited
  games.  It cannot rebuild a game from promotions, which needs the frame model.  So `ledger games FILE` writes the
  moves of every promotion set, the game that `solution_for_promotions` makes of it, and the script replays those by
  its own rules and checks that they contain the promotions and score the line's value.  Whatever the frame model did
  to make them, a game that is not a game of its value is refused; the link between the promotions and the moves is the
  Rust program's.  What the Python verifier takes on the line's record, because only the Rust one can compute it: the
  root bounds, a repeat's bags and the identity of its two search problems, and the bounds of searches and of nogoods.

Both exit with status 1 if any check fails, and name the line (`line K (n=N)`).

**Size.**  The laptop's dry run of 901..960, with the truth hidden, made [the ledger entry's
`store/`](../solver/notebook/the-ledger-bounds-from-both-sides-2026-10/store/): 58 games of the stretch not in the
chain, two exports of one stretch in all, 171 lines of 45 kilobytes, 778 bytes a game.  Its lines took 330 bytes for a
game of promotions, 135 for a cited game, 1.7 kilobytes for a game of moves and 155 for an upper bound.  At the end of
the chain (4400..8191) a promotion set is about 2 kilobytes, a game of moves about 15, and a cited game and an upper
bound about 150 and 220 bytes.  That is of the order of 3 kilobytes a game in a first export, about 12 megabytes for
3,800 games, less than `strict.jsonl`'s 7 (30.0 megabytes for 4,547 games).

## certificates

`certificates/` holds the committed certificates of U that `taxman certify` made ([DESIGN.md, "Certificates of
U"](../solver/docs/DESIGN.md#certificates-of-u) says what a certificate is and what it proves).

`certificates_600_1300.jsonl` has a certificate for each of the 377 games of 600..1300 that need one: 296 certified by
a search, and 81 by posing the same problem as an earlier game.  `taxman verify-certificates` passes it, in 4.6 s.  It
was made by `taxman certify 600-1300 --results results/strict.jsonl U --out
results/certificates/certificates_600_1300.jsonl --threads 1 --progress 0`, in 39 s, on the code of commit 35c614b.
`U` there stands for the four `--upper` files, the prover's barred bounds for 2..1300, 1301..2175, 2176..3314 and
3315..3440 (`solver/noframe/data/upper_*.jsonl`).

The certificates are in a directory of their own because `tests/results.rs` and `solver/tools/check_results.py` take
every `results/*.jsonl` for a chain or the bounds store.  `tests/certify.rs` checks the work list's prediction against
the file: the games that need a certificate, their open and rejected sources, and the games that the rule of the same
problem settles ([the work list of 2176-3440](../solver/tools/certify/README.md)).
