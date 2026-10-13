# Results of the Rust solver

One JSON line per game, as `taxman solve --out` writes it.  Both files here
were written before result lines carried their provenance: they are schema 1
(no `schema` field), with n, score, moves, promotions, method, completed,
nodes, search counters, seconds, threads, `tactics` and `tactic_switches`,
`unproven_tactics_on` (the unproven tactics that were on, used or not),
`optimal_match` (null past n = 1000) and `prev_source`.  The final build will
regenerate them with full provenance (schema 2, described in `src/results.rs`:
each game's proof kind and taint, a record of the game of n − 1 it was built
from, the build's commit, the time and invocations of a resumed search, the
limits).  Until then, this page says what is known about how they were made
and what supports them.

## The files

| file | games | tactics, as the lines record them | threads | invocations (at least) |
|---|---|---|---|---|
| `default.jsonl` | 1001–1231 | `default`: that build's defaults, which include the unproven `reuse_prev` and `revise_freedoms` | 10 for 1001–1200, 6 for 1201–1231 | 5, starting at 1001, 1201, 1218, 1221 and 1230; 1218, 1221 and 1230 resumed a checkpoint |
| `strict.jsonl` | 1001–1219 | `reuse_prev=off,revise_freedoms=off`: no unproven tactic on | 10 for 1001–1217, 6 for 1218–1219 | 4, starting at 1001, 1195, 1201 and 1218 |

* The commits that added the lines (2063771 for 1001–1200, e26bd71 for the
  rest, 8686be0 in between) changed nothing outside this directory, so the
  solver code in the tree was that of commit d85bfa6 throughout.  The lines
  do not record the build, so whether the binaries were built from exactly
  that code is not known.
* `default` means the defaults of that build, which differ from today's:
  `preclusions` on, and no `fix_forced`, `nogoods` or `forget_nogoods` (they
  did not exist yet, and `tactic_switches` has no key for them).  The same
  command today runs a different, faster search, which must find the same
  scores (see the cross-version agreement below).
* The first game of every file was built from optimal.json's game of 1000
  (`prev_source: "optimal"`).  Each later invocation shows as a game with
  `prev_source: "results"`; a resumed default game has `setup.revise:
  "FromCheckpoint"`.  A resumed strict game cannot be told from its line,
  since `revise_freedoms` is off.  The runs on 6 threads were made in
  resumable chunks with checkpoints every 30 s (the `engine.checkpoints`
  counts agree).
* For a resumed game `nodes` covers every invocation but `seconds` only the
  last one.  The slowest whole searches recorded are 1220 (365 s on 6
  threads) and 1225 (337 s); 1230 recorded 438 s in its last invocation alone.
  A run of 1232 was stopped unfinished; nothing of it is kept.
* Scores are reproducible; move lists need not be, since which of several
  optimal games a search finds depends on its order (`bench/curve/chain.jsonl`
  has other moves than `default.jsonl` at 1221–1224, with the same scores).

## What supports the scores

"Optimal" here means optimal within the frame model, whose assumptions are
listed in [DESIGN.md](../DESIGN.md#what-optimal-means) (the same holds for
optimal.json beyond the brute-force certified n ≤ 62), and given that the solver is right.  Two checkers, each
under a second:

    taxman verify-results results/default.jsonl results/strict.jsonl bench/curve/chain.jsonl
    python3 tools/check_results.py results/default.jsonl results/strict.jsonl bench/curve/chain.jsonl

(from the `rust` directory; the proof kinds and the taint are defined in
[DESIGN.md](../DESIGN.md#8-results-and-the-trust-model))

The first is the solver's own (also run on any results file `solve --out`
opens, and by `tests/results.rs`).  The second is an independent referee in
Python that shares no code with the solver; it also checks every score
against the Franklín–Moniot upper bound and the best-known heuristic chain in
`heuristics/results`.  Both replay every game (legal, its recorded score, no
legal move left, opening with the largest prime), check score(n − 1) ≤
score(n) ≤ n + score(n − 1), the arithmetic of each line and the predecessor
it names, and that the files agree; both derive each line's proof kind and
taint by the rules in `src/results.rs`, and both report:

| | `default.jsonl` | `strict.jsonl` | `bench/curve/chain.jsonl` |
|---|---|---|---|
| proof `exhausted`: the search explored its whole tree | 78 | 122 | 127 |
| proof `cap`: the search reached n + score(n − 1) | 0 | 66 | 70 |
| proof `prime_formula` | 34 | 31 | 34 |
| proof `reuse_prev`: a conjecture | 119 | 0 | 0 |
| rest on nothing unproven | 19 | 219 | 231 |
| score = n + score(n − 1) | 70 | 66 | 70 |
| score = n + score(n − 1) − P (a prime n) | 34 | 31 | 34 |

What that evidence amounts to:

1. **Arithmetic certificates.**  A score equal to n + score(n − 1) (the
   ceiling opt(n) ≤ n + opt(n − 1)), or for a prime n to n + score(n − 1) − P
   with P the largest prime below n (proven in `src/accelerate.rs`), is
   optimal whenever the game of n − 1 is, whatever found it: 97 of
   `strict.jsonl`'s games and 104 of `default.jsonl`'s.
2. **Exhaustive searches.**  `strict.jsonl`'s other 122 games are proven by a
   search that explored its whole tree, as the solver reports it.  Its 66
   `cap` games are the 66 at n + score(n − 1), all decided before the search
   explored a node.
3. **Standalone re-solves.**  `bench/curve/calib.jsonl` holds 197 standalone
   searches, one for every non-prime n in 1001–1231, with no seed and no cap
   (`ladder --no-seed --no-cap --tactic revise_freedoms=off`, 8 threads,
   commit e710d16; see `bench/curve/README.md`).  All of them finished, and
   each found the promotion sum of the score recorded here.  These proofs
   rest on no other game: every non-prime game in 1001–1231 is proven without
   the chain, and every prime one by the prime formula from a non-prime game
   so proven.
4. **Cross-version agreement.**  `bench/curve/chain.jsonl` is a strict chain
   1001–1231 made by a later build (e710d16: `fix_forced`, `nogoods` and
   `forget_nogoods` on, `preclusions` off).  Its 231 scores equal
   `default.jsonl`'s, and its games 1001–1219 equal `strict.jsonl`'s move for
   move.
5. **Taint.**  `strict.jsonl` rests on nothing unproven: no unproven tactic
   was on, and its first game, though built from optimal.json's 1000, was
   proven by exhausting its tree, so no proof in the file rests on
   optimal.json.  In `default.jsonl`, 132 games rest on `reuse_prev` (their
   own or a predecessor's) and 188 on `revise_freedoms` (counted wherever it
   set frame 0's freedoms); 19 rest on nothing unproven.

The checks cannot catch a game that is legal, finished and within the chain
bounds but not optimal when the game of n − 1 is not optimal either.  That is
what the proof kinds and the taint track, and why the standalone re-solves
(3) are the strongest evidence for 1001–1231.

## optimal.json

optimal.json was made by the Java with `reuse_prev` and `revise_freedoms` on,
and records no proofs, so a game taken from it counts as resting on
something unproven (taint `optimal.json`).  A strict run that starts from it
now needs `--allow-tainted-prev`, and its results that rest on it say so; a
strict chain that rests on nothing unproven starts at n = 1.

At commit 2063771, `taxman solve 1-1000 --threads 10 --tactic reuse_prev=off
--tactic revise_freedoms=off` reproduced every score in optimal.json with no
unproven tactic on (819 games searched, 168 prime, 13 trivial; 8.6 minutes in
total on 10 threads; slowest n=992, 75 s).  Its output was not kept, so this
cannot be checked from the repository.
