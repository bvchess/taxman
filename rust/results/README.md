# Results of the solver

One JSON line per game, as `taxman solve --out` writes it (the fields are described in `src/results.rs` and in
[../README.md](../README.md#a-results-line---out-schema-5)).  This page says what each file is, how it was made, and
what supports the scores in it.  The counts in it are pinned by `tests/results.rs`, so changing a file means updating
both together.  The one file of another kind, `bounds.jsonl` (bounds on opt(n), a line each, for boards the chain has
not proven), is described at the end.

## The files

| file | games | schema | tactics, as the lines record them | made by |
|---|---|---|---|---|
| `strict.jsonl` | 1–3440 | 5 | `default`: no tactic is unproven any more, so none is switched off | commit feba6d0 for 1–2175 and commit 55287ff for 2176–3440, a clean tree each (recorded in every line) |

`strict.jsonl` is the file to build on: a chain from n = 1, with full provenance.  Its games 2..2175 are also proven on
the whole board, by the prover; its games 2176..3440 are proven within the frame model, which is exact given statement
U (below, "What supports the scores"), and U is not yet certified past 2175.  Three other files were checked with
it and agree with it (see "What supports the scores"): `default.jsonl` (1001–1231, the first run past 1000, made with
the unproven tactics `reuse_prev` and `revise_freedoms` on, schema 1), which is gone from the tree (`git show
f72c332:rust/results/default.jsonl` has it); the cost-curve study's `bench/curve/chain.jsonl` (a chain 1001–1231 with
those tactics off, schema 1, commit e710d16); and the GF(2) study's `bench/gf2/chain.jsonl` (a chain 1301–2175 from
`optimal.json`'s game of 1300, schema 5, commit 716ebd9, 2 threads).  The two chains stay as their studies' data; the
verifiers read the second beside `strict.jsonl`, and no longer read the first.

**`explore.jsonl` is not a chain, and never goes into `strict.jsonl`.**  The file of a sweep
([README, "Keeping going past limits, and the sweep"](../README.md#keeping-going-past-limits-and-the-sweep);
`tools/cloud/sweep.sh`) merges the games of many blocks of consecutive board sizes, each given a limited search: it has
board sizes with only an unfinished search (a lower bound, no proof) and board sizes that no block reached, games built
on the best game of an unfinished search (no ceiling, no prime formula), and the same game's several lines.  Each
completed game in it is proven by its own proof, given what it assumes, in the frame model; it is evidence for those
games alone, and a game that follows a gap may rest on nothing the chain would have given it.  It is not committed
here; a sweep's results go to a file of their own, with a notebook entry saying what they are.

## strict.jsonl

The committed chain from n = 1 to n = 3440: every game is built from the one before, and nothing in it rests on
`optimal.json` or on any other file.  (The name is from the builds that had unproven tactics, which the chain was made
without; none is left, so every run from n = 1 is made without them.)  It has 3440 lines, the completed games in order
of n, so there is no line of an unfinished search.  It was made in two runs, the second a continuation of the first:
games 1–2175 in one invocation that nothing stopped, on a laptop, and games 2176–3440 appended to a copy of those lines
on a cloud machine (below); the file as it was before the second run is `git show
4f1c35b:rust/results/strict.jsonl`, and its 2175 lines are the first 2175 of this file, byte for byte.  It is the
fourth file of its name.  The earlier three were chains to 1300 and are in git: the first of commit bd9bb5d (schema 3,
`git show cd3d32f:rust/results/strict.jsonl`), the second of commit 243af9b (schema 4, `git show
2487671:rust/results/strict.jsonl`) and the third of commit 271c817 (schema 5, 4 threads, `git show
0133afe:rust/results/strict.jsonl`).  All three have this file's scores in games 1–1300, and the second and the third
its proofs and methods too; the first, made before the tactic `same_search` existed, searched the 545 games that this
file concludes as repeats.  Their moves differ from this file's in a few games (below).

How the first run, games 1–2175, was made:

* The command was, up to the paths, `taxman solve 1-2175 --threads 8 --out strict.jsonl --checkpoint-dir ckpt
  --checkpoint-interval 60 --progress 0`, run once to the end, with the build of commit feba6d0.  Every line records
  that build (`dirty` false), 8 threads, no `--time-limit` or `--node-limit` and the `default` tactics (the GF(2) bound,
  `gf2`, on; the Lagrangian bound, `lagrange`, off); the checkpoint interval is not recorded, but the
  `engine.checkpoints` counts (1 each for 2170 and 2172, the only games that searched for more than 60 s) imply about
  60 s.  No game was resumed (`invocations` is 1 in every line of 1–2175), and none was built from a results file or
  from `optimal.json` (`prev.source` is `run` from 2 on).
* The machine was an Apple M1 Max (10 CPUs) with nothing else running.  The run took 1638.6 s of wall time and
  10,890 s of CPU (user time, timed from outside; the lines record each game's own seconds).  The searches add up to
  1450.5 s (`seconds.search_all_runs`) and 48.4 M nodes: 28.4 s for 1–1300 (7.5 s for 1–1000, 11.8 s for 1001–1231 and
  9.1 s for 1232–1300; 1.08 M nodes) and 1422.1 s for 1301–2175 (681.3 s for 1301–1600, 396.8 s for 1601–2000 and
  344.1 s for 2001–2175; 47.4 M nodes).  The slowest games were 2172 (73.1 s, 1.31 M nodes), 2170 (68.5 s), 1475
  (50.1 s, 1.92 M nodes, the most of any game), 1472 (37.8 s) and 1430 (35.0 s); the next is 1521 at 34.3 s.  The queue
  of work units never held more than 178, and none spilled to disk.
* The cost of the first 1300 games fell with the tactics that came after the earlier chains: the third took 1022.5 s of
  search and 332.0 M nodes (on 4 threads of a machine shared with four other jobs, which makes its times pessimistic),
  the second 1052 s and the first 3359 s.  `same_search` concludes 545 of them as repeats of an earlier game's search
  ([EXPERIMENTS](../docs/experiments/same-search-2026-10.md)), the Lagrangian bound over the nogoods makes the searches
  that remain shorter
  ([EXPERIMENTS](../docs/experiments/a-lagrangian-bound-over-the-nogoods-2026-10.md)), and the GF(2) bound, which this
  is the first committed chain made with, makes them shorter again
  ([EXPERIMENTS](../docs/experiments/the-gf2-bound-in-the-search-2026-10.md)).  What the run adds to those entries is in
  [EXPERIMENTS](../docs/experiments/the-committed-chain-to-2175-2026-10.md).

How the second run, games 2176–3440, was made ([EXPERIMENTS](../docs/experiments/the-frontier-chain-to-3440-2026-10.md)
has the run's story and cost; [the runbook](../tools/cloud/RUNBOOK.md) the procedure):

* The command was `rust/tools/cloud/chain.sh 4095`, which runs `taxman solve 1-4095 --window W --out strict.jsonl
  --checkpoint-dir ckpt --checkpoint-interval 300 --progress 60` on all the CPUs, on a copy of the first run's 2175
  lines, so that the first game it solved was 2176; it was stopped by its 24-hour deadline (status 124, a final
  checkpoint written), after game 3440.  Every line of 2176–3440 records the build of commit 55287ff (`dirty` false),
  64 threads, no `--time-limit` or `--node-limit` and the `default` tactics.  The machine was one `c8g.16xlarge` (64
  vCPUs of Graviton4, 124 GiB) on spot in us-east-1d at $0.6752 an hour, in two instances: the first ran from
  2026-10-15 04:05:42Z with a window of 16 games in flight (`W`) to game 3310, and was stopped on purpose; the second,
  restored from the saved state, ran with a window of 4 from 10:40:41Z to the deadline.  About $16.46 in all.
* The window is not in the lines (a window changes neither a score nor a proof,
  [README](../README.md#games-in-flight)), but `invocations` is: it is 1 in every line of 2176–3440 but 3311, 3312 and
  3315, which were in flight when the first instance stopped and were taken up by the second from their checkpoints (2
  each).  The game of n − 1 came from the results file, not from the run, for the first game of each instance
  (`prev.source` is `results` for 2176 and 3311); the lines it read assume nothing, and so do these.
* The 1265 games are 567 searches (448 exhausted, 119 at the ceiling), 543 repeats and 155 primes.  The searches took
  17.73 G nodes: 70 games of 30 M nodes or more took 16.34 G of them, and the dearest were 3440 (3.72 G nodes, 18,184 s
  or 5.05 h on 64 threads), 3425 (1.23 G, 3.5 h), 3424 (1.30 G, 2.0 h), 2346 (1.14 G) and 3430 (0.80 G); the patches of
  dear games are the 2340s, 2375–2388, 3102–3112, 3174–3176, 3300, the 3330s and 3399–3440.  The search seconds of the
  448 exhausted searches rise tenfold per about 1,040 in n (a least-squares line through log10 of
  `seconds.search_all_runs`: 7 s at 2200, 97 s at 3400), with about one decade of scatter around it (sd 1.0),
  concentrated in the patches.  The queue of work units never held more than 7,298, and none spilled to disk.
* The run was left in the middle of game 3441, at 626 million nodes after 5.9 hours, with 3444 also begun; their
  checkpoints (format 4) are not part of the results, and there is no line for either.

What the lines say, as `taxman verify-results` and `tools/check_results.py` report them:

| | 1–1300 | 1301–2175 | 2176–3440 | `strict.jsonl` |
|---|---|---|---|---|
| games | 1300 | 875 | 1265 | 3440 |
| proof `trivial` | 13 | 0 | 0 | 13 |
| proof `exhausted`: the search explored its whole tree | 416 | 314 | 448 | 1178 |
| proof `cap`: the search reached the ceiling n + score(n − 1) | 115 | 77 | 119 | 311 |
| proof `prime_formula` | 211 | 115 | 155 | 481 |
| proof `same_search`: a repeat of an earlier game's search | 545 | 369 | 543 | 1457 |
| rest on nothing beyond the frame model | 1300 | 875 | 1265 | 3440 |
| score = n + score(n − 1) | 424 | 270 | 402 | 1096 |
| score = n + score(n − 1) − P (a prime n) | 210 | 115 | 155 | 480 |

(`default.jsonl` had 34 prime, 78 exhausted and 119 proofs by `reuse_prev`, 19 games resting on nothing unproven;
`chain.jsonl` 34 prime, 127 exhausted and 70 at the ceiling, all 231 resting on nothing unproven.)

By range, the proofs in `strict.jsonl` are: 1–1000, 311 exhausted, 92 at the ceiling, 168 prime, 13 trivial and 416
repeats; 1001–1231, 81 exhausted, 16 at the ceiling, 34 prime and 100 repeats; 1232–1300, 24 exhausted, 7 at the
ceiling, 9 prime and 29 repeats (the 69 games of 1232–1300 are 31 searches and 29 repeats, and the primes 1237, 1249,
1259, 1277, 1279, 1283, 1289, 1291 and 1297); 1301–2175, 314 exhausted, 77 at the ceiling, 115 prime and 369 repeats
(the 875 games are 391 searches, 369 repeats and the 115 primes); 2176–3440, 448 exhausted, 119 at the ceiling, 155
prime and 543 repeats (the 1265 games are 567 searches, 543 repeats and the 155 primes).  The 1489 games that searched
are 1178 exhausted and 311 at the ceiling; the proofs at the ceiling were all decided before the search explored a node
(0 nodes) but four: 686's, which took 29, and 2695's, 3052's and 3306's, which took 7,175, 2,826 and 5,366.  A repeat
takes the promotions of the latest earlier game that posed the same search problem: n − 1 for 1287 of the 1457 and n − 2
for the other 170 (n − 1 is prime for each of those).

Several games have more than one optimal set of moves, and which one a run finds depends on the order of its search:
scores are reproducible, move lists need not be.  The sets of this file differ from the earlier files' in these games:

* 539, 684–687 and 770–773, which the prover lists with two or three optimal sets each
  ([the prover's README](../prover/README.md#5-is-the-optimal-set-unique) has the exchanges).  `optimal.json`'s games
  1–1000 are the Java's, and this file has the same set of moves in each of them but 539 and 770–773, where the chain
  found another optimal set.  The previous file (the chain of commit 271c817, and that of 243af9b) had the table's
  sets at 539 and 770–773 and another at 684–687, where this file has the table's.
* 1215, where `optimal.json` and every earlier chain have {312, 450} and this file {342, 420}: an exchange of two
  numbers of the same sum (762).  The prover enumerates optimal sets only to n = 989.
* 1221–1224, where the chain of commit bd9bb5d and `bench/curve/chain.jsonl` found {364, 456, 544} and this file,
  `default.jsonl` and the chains of 243af9b and 271c817 have {348, 408, 608}: an exchange of three numbers of the same
  sum (1364) that starts at 1221 and is carried by 1222, 1223 and 1224.  The prover describes exchanges like it,
  which first appear at game 425.
* 1560–1563, where `bench/gf2/chain.jsonl` has {510, 594, 620, 624, 660} and this file {476, 528, 616, 644, 744}: an
  exchange of five numbers of the same sum (3008), found by the search of 1560 and carried by the three repeats after
  it.

Each chain that found a set proved its game optimal, so each set is.  `tests/results.rs` pins the games where this file
differs from the table.

## What supports the scores

"Optimal" here means optimal within the frame model, which is opt(n) wherever one open statement about the game,
U, holds ([DESIGN.md](../DESIGN.md#what-optimal-means); it holds for every n ≤ 2175, where the prover proves the
optimum on the whole board, and is not yet certified past it: `taxman certify` will check it game by game for 2176–3440;
a game proven at its ceiling or by the prime formula does not need it), and given that the solver is right.  Two
verifiers, about 10 s (42 s of CPU, on several threads) and 3 s on the laptop:

    taxman verify-results results/strict.jsonl
    python3 tools/check_results.py results/strict.jsonl

(from the `rust` directory; the proof kinds and the assumptions are defined in
[DESIGN.md](../DESIGN.md#8-results-and-the-trust-model))

The first is the solver's own (also run on any results file `solve --out` opens, and by `tests/results.rs`).  The
second is an independent referee in Python that shares no code with the solver; it also checks every score against the
Franklín–Moniot upper bound and the best-known heuristic chain in `heuristics/results`.  Both replay every game
(legal, its recorded score, no legal move left, opening with the largest prime), confirm its score against
optimal.json, check score(n − 1) ≤ score(n) ≤ n + score(n − 1), the arithmetic of each line and the predecessor it
names, and that the files agree; both derive each line's proof kind and what it assumes by the rules in
`src/results.rs`.  Both pass `strict.jsonl`.

What that evidence amounts to:

1. **Arithmetic certificates.**  A score equal to n + score(n − 1) (the ceiling opt(n) ≤ n + opt(n − 1)), or for a
   prime n to n + score(n − 1) − P with P the largest prime below n (proven in `src/accelerate.rs`), is optimal
   whenever the game of n − 1 is, whatever found it: 1576 of `strict.jsonl`'s games (1096 at n + score(n − 1), among
   them its 311 proofs at the ceiling, and 480 prime ones).
2. **Exhaustive searches.**  `strict.jsonl`'s 1178 `exhausted` games are proven by a search that explored its whole
   tree, as the solver reports it.  The rest of the file is its 311 proofs at the ceiling, its 481 prime games, its 13
   trivial ones (no promotion candidate, so there is only one game) and its 1457 repeats, which take the promotions and
   the proof of an earlier game that posed the same search problem (both verifiers check it, `src/results.rs`).
3. **Standalone re-solves.**  `bench/curve/calib.jsonl` holds 197 standalone searches, one for every non-prime n in
   1001–1231, with no seed and no ceiling (`ladder --no-seed --no-cap --tactic revise_freedoms=off`, 8 threads, commit
   e710d16; see `bench/curve/README.md`).  All of them finished, and each found the promotion sum of the score recorded
   here.  These proofs rest on no other game: every non-prime game in 1001–1231 is proven without the chain, and every
   prime one by the prime formula from a non-prime game so proven.  There is no such re-solve for 1232–1300: of those 69
   games, 24 are proven by an exhausted search of their own, and the 7 proofs at the ceiling, the 9 prime ones and the
   29 repeats rest on the game before them or on the game they repeat, so on the chain from n = 1 (see 5).  Past 1300,
   `bench/curve/bands.jsonl` has ten games of 1302–1352 solved standalone (the matching path, 8 threads), each finishing
   with the promotion sum recorded here; the rest of 1301–3440 is the chain's (762 exhausted searches of their own, and
   196 proofs at the ceiling, 270 prime ones and 912 repeats that rest on the games before them).
4. **Agreement between builds and files.**
   * n ≤ 1000: `strict.jsonl`'s scores are optimal.json's, all 1000 (each line's `optimal_match` is true;
     `tests/results.rs` checks it), 797 of the games are optimal.json's move for move, and 995 are its sets of moves
     (all but 539 and 770–773, above).
   * n = 1001–1300: optimal.json holds the games of the chain of commit 243af9b, copied move for move by `taxman
     extend-optimal`, and `strict.jsonl`'s games are those, move for move but 1215 (above; `tests/results.rs` checks
     it).  The lines say `optimal_match` true here too: when the run was made the table held those games, so this is a
     second run reproducing the first's scores, not a comparison with another source.
   * n = 1301–2175: optimal.json has no game (the lines say `optimal_match` null), so the comparison is with another
     run of the same search: `bench/gf2/chain.jsonl`, a chain from optimal.json's game of 1300 (`--use-prev`) made by
     the build of commit 716ebd9 on 2 threads in three invocations, which reached 2175 from 1301 in 92 minutes of game
     time.  Its 875 games have this file's scores, proofs and methods, in every game, and its moves in all but
     1560–1563 (above); only its game of 1301 assumes anything (that optimal.json's game of 1300 is optimal).
     `taxman verify-results results/strict.jsonl bench/gf2/chain.jsonl` (93 s) and `tools/check_results.py` on both
     accept them and find 875 board sizes in both files, with equal scores in all.
   * n = 2176–3440: optimal.json has no game here either, and no other chain reaches far.  `bench/cloud/c8g.chain-2176.jsonl`,
     the Graviton calibration's chain (64 threads, one game after another, the build of commit 407645f, 15 minutes),
     has the games 2176–2276, 101 in all; and a rehearsal of the second run, made on the same machine type just before
     it started (the build of commit 55287ff, 64 threads, a window of 16, stopped and killed and resumed on purpose; not
     committed), has 2176–2300, 125 in all.  Both have this file's scores, proofs, methods, promotions and moves in
     every game they share with it (checked on the final file).  Past 2300 the evidence is the run's own: the referees
     on the whole file.
   * 1001–1231: the scores of `strict.jsonl`, `bench/curve/chain.jsonl` (commit e710d16, 8 threads, a chain with the
     unproven tactics off) and `default.jsonl` (the earlier solver, with unproven tactics on) are equal in all 231
     games.  `strict.jsonl`'s games are `default.jsonl`'s move for move but 1215, and `chain.jsonl`'s but 1215 and
     1221–1224 (other games, the same scores).
   * The earlier committed chains (commits bd9bb5d, 243af9b and 271c817) have the same scores in all 1300 games.  The
     second's and the third's moves are the same as each other's and differ from this file's at 539, 684–687, 770–773
     and 1215; the first's differ at those and at 1221–1224.
5. **Assumptions.**  `strict.jsonl` assumes nothing beyond the frame model: it starts at n = 1, so no game of it is
   built from optimal.json (which is only compared with, up to 1300: a score that differs would be a hard error), and
   all 3440 games record that they assume nothing.
6. **The prover's proofs.**  The prover proves every n up to 2175 optimal without the frame model (no frames or
   promotions: [the prover's README](../prover/README.md); [THEORY.md](../../heuristics/THEORY.md),
   Theorem 10), so `strict.jsonl`'s scores to 2175 do not rest on the frame model either: those 2175 are the optimum on
   the whole board (1301–2175 in the cloud run of 2026-10-14 and 15,
   [EXPERIMENTS](../docs/experiments/the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10.md)).  The 1265
   games of 2176–3440 are proven within the frame model, which is exact given U ([THEORY.md, "The frame model: what is
   proven and what remains"](../../heuristics/THEORY.md#the-frame-model-what-is-proven-and-what-remains)), and U is
   not yet certified past 2175: `taxman certify` will check it game by game, with the prover's barred bounds
   ([the prover's README](../prover/README.md#the-upper-part-game-by-game) has them to 3314).  Until then their
   scores are opt(n) wherever U holds at n, and a game of them proven at its ceiling or by the prime formula is opt(n)
   wherever the game before it is.

The checks cannot catch a game that is legal, finished and within the chain bounds but not optimal when the game of
n − 1 is not optimal either.  That is what the proof kinds and the assumptions track, and why the chain from n = 1 and
the standalone re-solves (3) matter.

## optimal.json

optimal.json's games 1–1000 were made by the Java with `reuse_prev` and `revise_freedoms` on, and 1001–1300 are
the chain's, added by `taxman extend-optimal` (first from the chain of commit bd9bb5d, then from that of commit
243af9b, which differs from it at 1221–1224 only; `strict.jsonl` has the second's games but at 1215).  The file records
no proofs and no provenance, so a game taken from it (`--use-prev`) is assumed optimal, and the results whose proofs
rest on it say so ("the game of n in optimal.json is optimal"); a chain that assumes nothing beyond the frame model
starts at n = 1.  `strict.jsonl` is such a chain, and its games 1–1000 reproduce optimal.json's scores with no unproven
tactic on: 416 repeats, 311 exhausted, 92 at the ceiling, 168 prime and 13 trivial, 7.5 s of search in all on 8 threads,
the slowest n = 992 at 0.7 s.  The table has not been extended past 1300, and stays there: `taxman extend-optimal
results/strict.jsonl` would add the 2140 games of 1301–3440.

## bounds.jsonl

The bounds on opt(n) that [the ledger](../docs/experiments/the-ledger-bounds-from-both-sides-2026-10.md) finds, kept for
good, for the boards the chain has not proven.  The first `ledger export` made it, from stage 1 of the ledger past 4400
(2026-10-18), and each export since appends to it.  A line is a bound, not a game of `taxman solve`: it is not in the schema of the results lines, `taxman
verify-results` and `tools/check_results.py` do not read it, a board that `strict.jsonl` has proven has no line in it,
and it is not `explore.jsonl`'s kind of file either (that one is the sweep's games).  Its lines are bounds that hold:
a lower bound is the score of a finished game that the referee replays, an upper bound is the frame model's, carried
between boards by the lemmas of [THEORY.md](../../heuristics/THEORY.md#the-ledger-bounds-between-boards).  A board is
proven when its best lower bound equals its best upper bound, and the file is the record of those proofs.

**Append-only.**  A line is never changed or removed.  A new line is added only when it improves the file's best bound
of its board and side (a lower bound above every earlier lower bound of the board, an upper bound below every earlier
one), so a second export of the same ledger appends nothing; both checkers refuse a line that improves nothing.

**The commands** (from `rust/`; the program is the example `ledger`, built with `cargo build --profile test --example
ledger` on the laptop; its module comment, `examples/ledger/main.rs`, has every flag):

    ledger export RUN_DIR --bounds results/bounds.jsonl --source NAME [--checkpoints S3_PREFIX]
    ledger run A-B --dir DIR --cpu-hours H ... --bounds results/bounds.jsonl
    ledger verify-bounds results/bounds.jsonl
    ledger games results/bounds.jsonl > games.jsonl
    python3 -I tools/check_bounds.py results/bounds.jsonl --games games.jsonl

`export` appends the bounds of the run's `ledger.jsonl` that improve on the file's best, for the boards not already
games of `results/strict.jsonl` (`--strict` names another file), creating the file if there is none, and prints what it
appended: boards, lower and upper bounds that are first of their board or improve one, boards newly proven.  `--source`
names the run (every line records it, with the day); `--checkpoints` is the place of the run's checkpoints (the prefix of
`nN.ckpt`, for instance `s3://BUCKET/ledger-DATE/out/run/ckpt`), recorded on the dual bounds of the boards not proven.
`run --bounds` starts the run from the file's best bounds for the boards of its stretch: each lower bound's game is
replayed by the referee, each upper bound is taken as the file has it, and a board with both needs no map; its lower
bound's game is its incumbent and the finder's input, as a proven game that is given is.  Run `verify-bounds` on the file
first: the run takes its upper bounds on its word.

**The line.**  One JSON object, with the fields below and no others:

| field | on | meaning |
|---|---|---|
| `schema` | every line | 1 |
| `n`, `side`, `value`, `rule` | every line | the board, `lower` or `upper`, the bound on opt(n), and how it was derived |
| `from` | the relations | the board the bound came from |
| `from_value` | an upper bound by a relation; a lower bound whose game is cited | the value of the upper bound of `from` it was derived from; the lower bound of `from` whose game the line carries across |
| `bag`, `from_bag` | an upper bound by `repeat` | `in_the_bag` of `n` and of `from` |
| `rests_on` | an upper bound | the boards at which statement U is assumed, ascending |
| `checkpoint` | a `dual` upper bound | where the checkpoint of the search that gave it is, if the export was told |
| `source`, `date` | every line | the run (`--source`) and the day of the export, `YYYY-MM-DD` |
| `promotions` or `moves` | a lower bound with its game | the game: the promotions it was made from, or its moves |

The rules of a lower bound: `start` (the standalone start incumbent of the board's map), `search` (a search's best
game), `lns` (the finder's game), `given` (a proven game given to the run); and the relations, `monotone` (the game of
n − 1 followed by n), `ceiling` (the game of n + 1 without the move n + 1, completed), `prime` (the game of the board
next to a prime with its opening exchanged) and `repeat` (the promotions of the game of a board with the same search
problem, played on this board).  The rules of an upper bound: `root` (the GF(2) bound of the root), `dual` (the dual
bound of a search that a limit stopped), `exhausted` (a search that explored its whole tree), `floor` (a search under
a floor F, an incumbent worth F and no set, that completed having found nothing: no promotion set weighs more than F,
THEORY.md Lemma F, so the bound is `in_the_bag` and F), `nogoods` (the Lagrangian bound over nogoods that were each
checked to fail to promote, with the GF(2) bound as its inner problem, as the removed `wall nogoods` made it; nothing
makes it now), `given`, and the relations
`ceiling` (U(n) ≤ U(n − 1) + n), `monotone` (U(n − 1) ≤ U(n)), `prime` (U(p) = U(p − 1) + p − q, q the largest prime
below p, both ways) and `repeat` (U(n) − bag(n) ≤ U(m) − bag(m)).  A line of 4501 might read:

    {"schema":1,"n":4501,"side":"upper","value":9051234,"rule":"ceiling","from":4500,"from_value":9046733,"rests_on":[4500],"source":"ledger-4400-8191","date":"2026-10-18"}

**A lower bound's game** is given in one of three forms, the most compact that is possible.  A game made from promotions
(a search's, the finder's, the start's, a repeat of such a game) is its `promotions`, ascending, which
`solution_for_promotions` turns into moves (about 330 bytes at n = 900, about 1.1 kilobytes at n = 3440, where the
moves would take 9).  A game that a relation (`monotone`, `ceiling`, `prime`) made from the game of another board is
not written but cited: the line has `from` and `from_value` and no game, and its game is the lemma's construction
applied to the game of the lower bound of `from` that is worth `from_value`, which a line before it gives (a game of
the chain is never cited: a run need not have the chain).  Any other game is its `moves`.

**What an upper bound rests on.**  Every upper bound is a bound of the frame model at some board (a root bound, a dual
bound, a completed search, a given game), which is opt(n) where statement U holds at that board; the lemmas carry it
between boards without more, except the upper half of Lemma L4, which rests on U at the board it is carried to
([THEORY.md, "In the ledger"](../../heuristics/THEORY.md#the-ledger-bounds-between-boards)).  `rests_on` lists the
boards: a bound of the frame model has its own board alone, a relation has what the bound it comes from rests on (a
board of the chain that it cites counts as resting on itself), and a repeat adds its own board.  U is certified for
every n up to 2175 (DESIGN.md, "What optimal means"), so a bound whose list holds only boards up to 2175 rests on no open
statement; a proven board is opt(n) wherever U holds at the boards of the `rests_on` of its upper bound.

**Two referees**, as the results files have.  `ledger verify-bounds FILE` (Rust; `examples/ledger/verify.rs`) checks
every line: its form; that it improves on the lines before it; that every lower bound's game replays under the
referee (`replay_finished`: legal, over, opening with the largest prime) at the value of the line, a cited game made
from the one it cites; that every `root` bound is recomputed and equal; that every repeat's bags and search problems
are recomputed and equal to its source's; that every relation's arithmetic holds against the line it cites (a line
before it with a bound of that board at most `from_value`, or a game of the chain) and rests on what that line rests on;
and that where the chain has a game of a board, the file's bounds enclose its score.  It reports the proven boards, and
lists the bounds it takes on their record: the `exhausted`, `dual` and `floor` bounds of searches and the `nogoods`
bounds, which no check short of the search (or the fit) recomputes, and the `given` games.  `tools/check_bounds.py` (Python, standard library only, run with `python3
-I`) is the second referee of the games and of the arithmetic: it shares no code with the Rust crate, loads the rules
of the game from `tools/check_results.py`, and writes out the arithmetic of the lemmas and the constructions of the
cited games again.  It cannot rebuild a game from promotions, which needs the frame model, so `ledger games FILE` writes
the moves of every promotion set (the game that `solution_for_promotions` makes of it) and the checker replays those by
its own rules and checks that they contain the promotions and score the line's value: whatever the frame model did to
make them, a game that is not a game of its value is refused, and the link between the promotions and the moves is the
Rust program's.  What the Python referee takes on the line's record, because only the Rust one can compute it: root
bounds, a repeat's bags and the identity of its two search problems, and the bounds of searches and of nogoods.  Both exit with status 1
if any check fails and name the line (`line K (n=N)`).

**Size**, from the laptop's dry run of 901..960 with the truth hidden ([bench/ledger/store/](../bench/ledger/store/)):
58 boards of the stretch not in the chain, two exports of one stretch in all, 171 lines of 45 kilobytes, 778 bytes a
board, from lines of 330 bytes for a game of promotions, 135 for a cited game, 1.7 kilobytes for a game of moves and 155
for an upper bound.  At the end of the chain (4400..8191) a promotion set is about 2 kilobytes, a game of moves about 15, a cited
game and an upper bound about 150 and 220 bytes: of the order of 3 kilobytes a board in a first export (about 12
megabytes for 3,800 boards), less than `strict.jsonl`'s 5 (17.7 megabytes for 3,440 games).
