# Results of the Rust solver

One JSON line per game, as `taxman solve --out` writes it (the fields are described in `src/results.rs` and in
[../README.md](../README.md#a-results-line---out-schema-5)).  This page says what each file is, how it was made, and
what supports the scores in it.  The counts in it are pinned by `tests/results.rs`, so changing a file means updating
both together.

## The files

| file | games | schema | tactics, as the lines record them | made by |
|---|---|---|---|---|
| `strict.jsonl` | 1–1300 | 5 | `default`: no tactic is unproven any more, so none is switched off | commit 271c817, a clean tree (recorded in every line) |

`strict.jsonl` is the file to build on: a chain from n = 1, with full provenance.  Two other files were checked with
it until 2026-10 and agree with it (see "What supports the scores"): `default.jsonl` (1001–1231, the first run past
1000, made with the unproven tactics `reuse_prev` and `revise_freedoms` on, schema 1), which is gone from the tree
(`git show f72c332:rust/results/default.jsonl` has it), and the cost-curve study's `bench/curve/chain.jsonl` (a chain
1001–1231 with those tactics off, schema 1, commit e710d16), which stays as that study's data but is no longer a file
the verifiers read.

## strict.jsonl

The strict chain from n = 1 to n = 1300: every game is built from the one before, and nothing in it rests on
`optimal.json` or on any other file.  (The name is from the builds that had unproven tactics, which a strict run
switched off; none is left, so every run from n = 1 is one.)  It has 1300 lines, the completed games in order of n: it
was made in one invocation that nothing stopped, so there is no line of an unfinished search.  It is the third file of
its name.  The first, a chain of commit bd9bb5d (schema 3, 6 threads, made before the tactic `same_search` and the
Lagrangian bound existed), and the second, of commit 243af9b (schema 4), are in git (`git show
cd3d32f:rust/results/strict.jsonl`, `git show 2487671:rust/results/strict.jsonl`).  This file has the second's moves,
proofs and methods in every game, and the first's scores in every game and its moves in all but 1221–1224 (see below).

How it was made:

* The command was, up to the paths, `taxman solve 1-1300 --threads 4 --out strict.jsonl --checkpoint-dir ckpt
  --checkpoint-interval 60 --progress 0`, run once to the end, with the build of commit 271c817.  Every line records
  that build (`dirty` false), 4 threads, no `--time-limit` or `--node-limit` and the `default` tactics; the checkpoint
  interval is not recorded, but the `engine.checkpoints` counts (3 for the 207 s of 1232, 1 for each of 1287, 1290, 1295
  and 1300) imply about 60 s.  No game was resumed (`invocations` is 1 in every line), and none was built from a
  results file or from `optimal.json` (`prev.source` is `run` from 2 on).
* The machine was an Apple M1 Max (10 cores) shared with four other jobs of two threads each, so the times are
  pessimistic, and not comparable with those of `chain.jsonl` (8 threads).  The run took 1048.7 s of wall time and
  3965 s of CPU (timed from outside; the lines record each game's own seconds).  The searches add up to 1022.5 s
  (`seconds.search_all_runs`): 46.3 s for 1–1000, 167.9 s for 1001–1231 and 808.4 s for 1232–1300.  Five games searched
  for more than 50 s: 1232 (206.7 s, the slowest, 53.0 M nodes), 1295 (104.2 s), 1300 (87.0 s), 1287 (75.5 s) and 1290
  (69.7 s); the next is 1230 at 46 s.  The queue of work units never held more than 110.
* The chain of commit 243af9b, made the same way (with `--strict`) by the build before the cleanup that made every run
  strict, under another load, took 1078 s of wall time and 1052 s of search (51 s, 188 s and 813 s for the same
  ranges), and the chain of commit bd9bb5d 3359 s of search (83 s, 410 s and 2866 s) on 6 threads of the same machine,
  shared with another CPU-heavy job.  Two tactics that the latter lacked account for the difference: `same_search`
  concludes 545 games as repeats of an earlier game's search, which took 953 s of that chain, 851 s of them in
  1232–1300 ([EXPERIMENTS](../docs/EXPERIMENTS.md#same-search-2026-10)), and `lagrange`, the Lagrangian bound over the
  nogoods, makes the searches that remain shorter
  ([EXPERIMENTS](../docs/EXPERIMENTS.md#a-lagrangian-bound-over-the-nogoods-2026-10)).

What the lines say, as `taxman verify-results` and `tools/check_results.py` report them:

| | `strict.jsonl` |
|---|---|
| games | 1–1300 |
| proof `trivial` | 13 |
| proof `exhausted`: the search explored its whole tree | 416 |
| proof `cap`: the search reached n + score(n − 1) | 115 |
| proof `prime_formula` | 211 |
| proof `same_search`: a repeat of an earlier game's search | 545 |
| rest on nothing beyond the frame model | 1300 |
| score = n + score(n − 1) | 424 |
| score = n + score(n − 1) − P (a prime n) | 210 |

(`default.jsonl` had 34 prime, 78 exhausted and 119 proofs by `reuse_prev`, 19 games resting on nothing unproven;
`chain.jsonl` 34 prime, 127 exhausted and 70 by the cap, all 231 resting on nothing unproven.)

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

Games 684–687 too have two or three optimal sets each, and there the table differs from this file: `optimal.json`'s
games 1–1000 are the Java's, and this file has the same set of moves in each of them but these four, where the chain
found another optimal set ([evidence/README.md](../evidence/README.md#5-is-the-optimal-set-unique) lists the
exchanges).  `tests/results.rs` pins the four.

## What supports the scores

"Optimal" here means optimal within the frame model, whose assumptions are listed in
[DESIGN.md](../DESIGN.md#what-optimal-means) (the same holds for optimal.json beyond the brute-force certified
n ≤ 62), and given that the solver is right.  Two checkers, about 6 s and under 1 s on the laptop:

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
   whenever the game of n − 1 is, whatever found it: 634 of `strict.jsonl`'s games (424 at n + score(n − 1), among
   them its 115 proofs by the cap, and 210 prime ones).
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
     `tests/results.rs` checks it), 798 of the games are optimal.json's move for move, and 996 are its sets of moves
     (all but 684–687, above).
   * n = 1001–1300: optimal.json holds the games of the chain of commit 243af9b, copied move for move by `taxman
     extend-optimal`, and `strict.jsonl`'s games are those, move for move (`tests/results.rs` checks it).  The lines say
     `optimal_match` true here too: when the run was made the table held those games, so this is a second run
     reproducing the first's scores, not a comparison with another source.
   * 1001–1231: the scores of `strict.jsonl` (commit 271c817, 4 threads), `bench/curve/chain.jsonl` (commit e710d16,
     8 threads, a strict chain) and `default.jsonl` (the earlier solver, with unproven tactics on) are equal in all
     231 games.  `strict.jsonl`'s games are `default.jsonl`'s move for move, and `chain.jsonl`'s in all but 1221–1224
     (other games, the same scores): scores are reproducible, move lists need not be, since which of several optimal
     games a search finds depends on its order.
   * The strict chains of commits bd9bb5d and 243af9b that came before `strict.jsonl` had the same scores in all 1300
     games; the second's moves are `strict.jsonl`'s, and the first's are the same in all but 1221–1224 (those of
     `chain.jsonl`).
5. **Assumptions.**  `strict.jsonl` assumes nothing beyond the frame model: it starts at n = 1, so no game of it is
   built from optimal.json (which is only compared with: a score that differs would be a hard error), and all 1300
   games record that they assume nothing.

The checks cannot catch a game that is legal, finished and within the chain bounds but not optimal when the game of
n − 1 is not optimal either.  That is what the proof kinds and the assumptions track, and why the chain from n = 1 and
the standalone re-solves (3) matter.

## optimal.json

optimal.json's games 1–1000 were made by the Java with `reuse_prev` and `revise_freedoms` on, and 1001–1300 are
the strict chain's, added by `taxman extend-optimal` (first from the chain of commit bd9bb5d, then from that of commit
243af9b, which differs from it at 1221–1224 only; `strict.jsonl` has the second's games).  The file records no proofs
and no provenance, so a game taken from it (`--use-prev`) is assumed optimal, and the results whose proofs rest on it
say so ("the game of n in optimal.json is optimal"); a chain that assumes nothing beyond the frame model starts at
n = 1.  `strict.jsonl` is such a chain, and its games 1–1000 reproduce optimal.json with no unproven tactic on: 416
repeats, 311 exhausted, 92 by the cap, 168 prime and 13 trivial, 46 s of search in all on 4 threads, the slowest
n = 992 at 13 s.
