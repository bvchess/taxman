# Results of the Rust solver

One JSON line per game, as `taxman solve --out` writes it (the fields are described in `src/results.rs` and in
[../README.md](../README.md#a-results-line---out-schema-4)).  This page says what each file is, how it was made, and
what supports the scores in it.  The counts in it are pinned by `tests/results.rs`, so changing a file means updating
both together.

## The files

| file | games | schema | tactics, as the lines record them | made by |
|---|---|---|---|---|
| `strict.jsonl` | 1–1300 | 4 | `reuse_prev=off,revise_freedoms=off`: no unproven tactic on | commit 243af9b, a clean tree (recorded in every line) |
| `default.jsonl` | 1001–1231 | 1 | `default`: that build's defaults, which include the unproven `reuse_prev` and `revise_freedoms` | not recorded; the solver code of commit d85bfa6 |

`strict.jsonl` is the file to build on: a strict chain from n = 1, with full provenance.  `default.jsonl` is kept as
it was made, as the historical record of the first run past 1000, and is not regenerated: it is schema 1 (no `schema`
field) and its games rest on unproven tactics.  The cost-curve study's `bench/curve/chain.jsonl` (a strict chain
1001–1231, also schema 1, made by commit e710d16 on 8 threads) is the third file the checks below compare with.

## strict.jsonl

The strict chain from n = 1 to n = 1300: every game is built from the one before, no unproven tactic is on, and
nothing in it rests on `optimal.json` or on any other file.  It has 1300 lines, the completed games in order of n: it
was made in one invocation that nothing stopped, so there is no line of an unfinished search.  It replaces the strict
chain of commit bd9bb5d (schema 3, 6 threads, made before the tactic `same_search` and the Lagrangian bound existed;
`git show cd3d32f:rust/results/strict.jsonl` has it): the same 1300 scores, and the same moves but for 1221–1224 (see
below).

How it was made:

* The command was, up to the paths, `taxman solve 1-1300 --strict --threads 4 --out strict.jsonl --checkpoint-dir
  ckpt --checkpoint-interval 60 --progress 0`, run once to the end, with the build of commit 243af9b.  Every line
  records that build (`dirty` false), 4 threads, no `--time-limit` or `--node-limit` and
  `reuse_prev=off,revise_freedoms=off`; the checkpoint interval is not recorded, but the `engine.checkpoints` counts
  (3 for the 184 s of 1232) imply about 60 s.  No game was resumed (`invocations` is 1 in every line), and none was
  built from a results file or from `optimal.json` (`prev_source` is `run` from 2 on).
* The machine was an Apple M1 Max (10 cores) shared with other jobs, so the times are pessimistic, and not comparable
  with the other files' (`default.jsonl` used 10 and 6 threads, `chain.jsonl` 8).  The run took 1078 s of wall time
  and 4014 s of CPU (timed from outside; the lines record each game's own seconds).  The searches add up to 1052 s
  (`seconds.search_all_runs`): 51 s for 1–1000, 188 s for 1001–1231 and 813 s for 1232–1300.  Six games searched for
  more than 50 s: 1232 (184 s, the slowest, 32.6 M nodes), 1295 (96 s), 1300 (89 s), 1287 (81 s), 1290 (71 s) and 1230
  (51 s); the next is 1236 at 38 s.  The queue of work units never held more than 123.
* The chain it replaces took 3359 s of search (83 s, 410 s and 2866 s for the same ranges) on 6 threads of the same
  machine, shared with another CPU-heavy job.  Two tactics that it lacked account for the difference: `same_search`
  concludes 545 games as repeats of an earlier game's search, which took 953 s of that chain, 851 s of them in
  1232–1300 ([EXPERIMENTS](../docs/EXPERIMENTS.md#same-search-2026-10)), and `lagrange`, the Lagrangian bound over the
  nogoods, makes the searches that remain shorter
  ([EXPERIMENTS](../docs/EXPERIMENTS.md#a-lagrangian-bound-over-the-nogoods-2026-10)).

What the lines say, as `taxman verify-results` and `tools/check_results.py` report them:

| | `strict.jsonl` | `default.jsonl` | `bench/curve/chain.jsonl` |
|---|---|---|---|
| games | 1–1300 | 1001–1231 | 1001–1231 |
| proof `trivial` | 13 | 0 | 0 |
| proof `exhausted`: the search explored its whole tree | 416 | 78 | 127 |
| proof `cap`: the search reached n + score(n − 1) | 115 | 0 | 70 |
| proof `prime_formula` | 211 | 34 | 34 |
| proof `reuse_prev`: a conjecture | 0 | 119 | 0 |
| proof `same_search`: a repeat of an earlier game's search (a tactic newer than the other files) | 545 | 0 | 0 |
| rest on nothing unproven | 1300 | 19 | 231 |
| score = n + score(n − 1) | 424 | 70 | 70 |
| score = n + score(n − 1) − P (a prime n) | 210 | 34 | 34 |

By range, the proofs in `strict.jsonl` are: 1–1000, 311 exhausted, 92 cap, 168 prime, 13 trivial and 416 repeats;
1001–1231, 81 exhausted, 16 cap, 34 prime and 100 repeats; 1232–1300, 24 exhausted, 7 cap, 9 prime and 29 repeats (the
69 games of 1232–1300 are 31 searches and 29 repeats, and the primes 1237, 1249, 1259, 1277, 1279, 1283, 1289, 1291
and 1297).  The 531 games that searched are 416 exhausted and 115 by the cap; the proofs by the cap were all decided
before the search explored a node (0 nodes).  A repeat takes the promotions of the latest earlier game that posed the
same search problem: n − 1 for 471 of the 545 and n − 2 for the other 74 (n − 1 is prime for each of those).

Games 1221–1224 have at least two optimal sets each.  The chain of commit bd9bb5d and `bench/curve/chain.jsonl` found
{364, 456, 544} where this file and `default.jsonl` have {348, 408, 608}: an exchange of three numbers of the same
sum (1364) that starts at 1221 and is carried by 1222, 1223 and 1224; all else in each game is the same.  The strict
chains that found them proved their games optimal (each searched 1221 to exhaustion), so both sets are.  The evidence
crate describes exchanges like it, which first appear at game 425, though it enumerates the optimal sets only to
n = 989 ([evidence/README.md](../evidence/README.md#5-is-the-optimal-set-unique)).

## default.jsonl

Written before result lines carried their provenance: schema 1, with n, score, moves, promotions, method, completed,
nodes, search counters, seconds, threads, `tactics` and `tactic_switches`, `unproven_tactics_on` (the unproven
tactics that were on, used or not), `optimal_match` (null past n = 1000, where the table then stopped) and
`prev_source`.  Its proofs and taints are derived from those fields by the rules of `src/results.rs`.

* Threads: 10 for 1001–1200 and 6 for 1201–1231.  At least 5 invocations, starting at 1001, 1201, 1218, 1221 and
  1230; 1218, 1221 and 1230 resumed a checkpoint (`setup.revise: "FromCheckpoint"`).  The runs on 6 threads were made
  in resumable chunks with checkpoints every 30 s (the `engine.checkpoints` counts agree).
* The commits that added the lines (2063771 for 1001–1200, e26bd71 for the rest, 8686be0 in between) changed nothing
  outside this directory, so the solver code in the tree was that of commit d85bfa6 throughout.  The lines do not
  record the build, so whether the binaries were built from exactly that code is not known.
* `default` means the defaults of that build, which differ from today's: `preclusions` on, and no `fix_forced`,
  `nogoods` or `forget_nogoods` (they did not exist yet, and `tactic_switches` has no key for them).  The same command
  today runs a different, faster search, which must find the same scores (see the agreement below).
* Its first game was built from optimal.json's game of 1000 (`prev_source: "optimal"`); each later invocation shows
  as a game with `prev_source: "results"`.
* For a resumed game `nodes` covers every invocation but `seconds` only the last.  The slowest whole searches
  recorded are 1220 (365 s on 6 threads) and 1225 (337 s); 1230 recorded 438 s in its last invocation alone.  A run of
  1232 was stopped unfinished; nothing of it is kept.
* In it, 132 games rest on `reuse_prev` (their own or a predecessor's) and 188 on `revise_freedoms` (counted wherever
  it set frame 0's freedoms); 19 rest on nothing unproven.

## What supports the scores

"Optimal" here means optimal within the frame model, whose assumptions are listed in
[DESIGN.md](../DESIGN.md#what-optimal-means) (the same holds for optimal.json beyond the brute-force certified
n ≤ 62), and given that the solver is right.  Two checkers, each under a second:

    taxman verify-results results/default.jsonl results/strict.jsonl bench/curve/chain.jsonl
    python3 tools/check_results.py results/default.jsonl results/strict.jsonl bench/curve/chain.jsonl

(from the `rust` directory; the proof kinds and the taint are defined in
[DESIGN.md](../DESIGN.md#8-results-and-the-trust-model))

The first is the solver's own (also run on any results file `solve --out` opens, and by `tests/results.rs`).  The
second is an independent referee in Python that shares no code with the solver; it also checks every score against the
Franklín–Moniot upper bound and the best-known heuristic chain in `heuristics/results`.  Both replay every game
(legal, its recorded score, no legal move left, opening with the largest prime), check score(n − 1) ≤ score(n) ≤ n +
score(n − 1), the arithmetic of each line and the predecessor it names, and that the files agree; both derive each
line's proof kind and taint by the rules in `src/results.rs`.  They pass on all three files.

What that evidence amounts to:

1. **Arithmetic certificates.**  A score equal to n + score(n − 1) (the ceiling opt(n) ≤ n + opt(n − 1)), or for a
   prime n to n + score(n − 1) − P with P the largest prime below n (proven in `src/accelerate.rs`), is optimal
   whenever the game of n − 1 is, whatever found it: 634 of `strict.jsonl`'s games (424 at n + score(n − 1), among
   them its 115 proofs by the cap, and 210 prime ones) and 104 of `default.jsonl`'s.
2. **Exhaustive searches.**  `strict.jsonl`'s 416 `exhausted` games are proven by a search that explored its whole
   tree, as the solver reports it.  The rest of the file is its 115 proofs by the cap, its 211 prime games, its 13
   trivial ones (no promotion candidate, so there is only one game) and its 545 repeats, which take the promotions and
   the proof of an earlier game that posed the same search problem (both verifiers check it, `src/results.rs`).
3. **Standalone re-solves.**  `bench/curve/calib.jsonl` holds 197 standalone searches, one for every non-prime n in
   1001–1231, with no seed and no cap (`ladder --no-seed --no-cap --tactic revise_freedoms=off`, 8 threads, commit
   e710d16; see `bench/curve/README.md`).  All of them finished, and each found the promotion sum of the score
   recorded here.  These proofs rest on no other game: every non-prime game in 1001–1231 is proven without the
   chain, and every prime one by the prime formula from a non-prime game so proven.  There is no such re-solve for
   1232–1300: of those 69 games, 24 are proven by an exhausted search of their own, and the 7 proofs by the cap, the 9
   prime ones and the 29 repeats rest on the game before them or on the game they repeat, so on the strict chain from
   n = 1 (see 5).
4. **Agreement between builds and files.**
   * n ≤ 1000: `strict.jsonl`'s scores are optimal.json's, all 1000 (each line's `optimal_match` is true;
     `tests/results.rs` checks it), and 798 of the games are optimal.json's move for move.
   * n = 1001–1300: optimal.json now holds `strict.jsonl`'s games, copied move for move by `taxman extend-optimal` (as
     `tests/results.rs` checks).  The lines say `optimal_match` true here too: when the run was made the table held
     the games of the chain of commit bd9bb5d, whose scores are the same.  Taking the table's games 1001–1300 again
     from this file changed 1221–1224 only.
   * 1001–1231: the scores of `strict.jsonl` (commit 243af9b, 4 threads), `bench/curve/chain.jsonl` (commit e710d16,
     8 threads, a strict chain) and `default.jsonl` (the earlier solver, with unproven tactics on) are equal in all
     231 games.  `strict.jsonl`'s games are `default.jsonl`'s move for move, and `chain.jsonl`'s in all but 1221–1224
     (other games, the same scores): scores are reproducible, move lists need not be, since which of several optimal
     games a search finds depends on its order.
   * The strict chain of commit bd9bb5d that `strict.jsonl` replaces had the same scores in all 1300 games, and the
     same moves in all but 1221–1224 (those of `chain.jsonl`).
5. **Taint.**  `strict.jsonl` rests on nothing unproven: no unproven tactic was on, and it starts at n = 1, so no
   game of it is built from optimal.json (which is only compared with: a mismatch would be a hard error), and all
   1300 games have an empty taint.  `default.jsonl` is built from optimal.json's game of 1000 and rests on unproven
   tactics for all but 19 of its games.

The checks cannot catch a game that is legal, finished and within the chain bounds but not optimal when the game of
n − 1 is not optimal either.  That is what the proof kinds and the taint track, and why the chain from n = 1 and the
standalone re-solves (3) matter.

## optimal.json

optimal.json's games 1–1000 were made by the Java with `reuse_prev` and `revise_freedoms` on, and 1001–1300 are
`strict.jsonl`'s, added by `taxman extend-optimal` (first from the chain of commit bd9bb5d, then from this file, which
differs from it at 1221–1224 only).  The file records no proofs and no provenance, so a game taken from it counts as
resting on something unproven (taint `optimal.json`), whichever game it is.  A strict run that starts from it needs
`--allow-tainted-prev`, and its results that rest on it say so; a strict chain that rests on nothing unproven starts at
n = 1.  `strict.jsonl` is such a chain, and its games 1–1000 reproduce optimal.json with no unproven tactic on: 416
repeats, 311 exhausted, 92 by the cap, 168 prime and 13 trivial, 51 s of search in all on 4 threads, the slowest
n = 992 at 13 s.
