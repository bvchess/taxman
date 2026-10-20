# Which tactics earn their keep (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-13.  Concluded 2026-10-13.

**Question.**  Which tactics and alternatives give meaningful acceleration in the case that matters, long chains at the
end of the chain, and which should be removed, with the support for old file formats that goes with them?  **Answer.**
Of the unproven or losing ones, none: `reuse_prev` saves 3.8 s of the committed chain's 1052 s of search (0.36%),
`revise_freedoms` changes no tree of seven proofs at the end of the chain, and the alternatives cost the proof of 1215
from 1.8 times the cycles (`preclusions`) to more than 17 times the nodes (`match_bound=off`).  The project's owner took
the recommendation and removed them, with schemas 1 to 4 of the results lines and the checkpoint headers before version
4, so every run is strict.

## Method

The project's owner's rule was that a tactic that is unproven and does not pay has no reason to stay.  The case that
matters is a long chain at its end (n of 1200 to 1400 then, more on a 192-core machine), each game built
from the one before, as `results/strict.jsonl` is.  The solver code was that of commit 243af9b, which made
`results/strict.jsonl` as it was then (unchanged to 72601a2).  Searches are the ladder's proof-only ones on one thread
(`ladder --threads 1 --seed-opt --known results/strict.jsonl --seconds S [--tactic …] N`), whose node counts are exact,
with the process's cycles from `/usr/bin/time -l`, two at a time on the laptop while another job measured cycles on it,
so cycles are good to a few percent.  What `reuse_prev` and `revise_freedoms` do was worked out from setups alone, with
no search, by a scratch example (not kept):

* `reuse_prev`: for every n of 2..1300 that is neither prime nor trivial, `accelerate::reuse_prev` with the committed
  chain's game of n − 1, its game compared with n's line in `strict.jsonl` (score, moves, proof, and
  `seconds.search_all_runs`); and a chain simulated with the tactic on, in the solver's order: prime, trivial,
  `reuse_prev`, else a repeat of the latest of the 8 games before n that set a search up and pose the same
  problem, else a search, with the derived games as n − 1 where it accepted.
* `revise_freedoms`: `Board::prepare` with the tactic on and off, one thread, for each searched or repeated game of
  1232..1300: what revising did, how long it took, and whether the two problems differ.

## Results

### `reuse_prev`: nothing left to save

With the committed chain's games as n − 1 it accepts 670 games of 2..1300, and the chain settles every one of them
without a search of any length:

| what the committed chain did with them | games | at the ceiling n + score(n − 1) | their search seconds |
|---|---|---|---|
| a repeat (`same_search`) | 545 | 300 | 0 |
| the cap, reached before the first node | 115 | 115 | 1.89 |
| an exhausted search | 10 | 0 | 1.92 |

The ten searches are of 51, 111, 119, 141, 183, 185 and 741 (each under 0.03 s), 1083 (0.11 s), 1183 (0.14 s) and 1209
(1.64 s, the only one over a second); the other 406 searches of the chain it rejects.  So a rerun of the chain with the
tactic on would save at most 3.8 s of its 1052 s of search (0.36%): 0.6 s of 51 s in 1..1000, 2.7 s of 188 s in
1001..1231, and 0.5 s of 813 s in 1232..1300, all of it there the setups of seven proofs by the cap.  The simulated
chain agrees: 670 derived, 406 searched and no repeat at all, since the tactic comes first and takes every game whose
problem is its predecessor's, and no game that the committed chain repeats would have to be searched instead.  No
derived game differs from the committed chain's, in its score or in a single move.  255 of the 670 are accepted below
the ceiling, where only the conjecture says that they are optimal; 245 of those are repeats, proven.  What the tactic
once saved, the cap with the seed (every game at the ceiling) and `same_search` (the rest) now prove.

### `revise_freedoms`: the same trees

Its premise is false for the whole frame game ([n = 833](../../../noframe/docs/longest-games.md),
`tests/soundness.rs`), and the project's owner decided to remove it, so measuring stopped there.  In every one of the 60
searched or repeated games of 1232..1300 it searches the two-frame game (282 to 2,238 nodes, 16 to 27 ms, 1.2 s for the
60) and lowers frame 0's freedoms by 4 to 6 and `max_total` by 0 to 5, so every problem changes; yet seven proofs at the
    end of the chain
have the same trees with it on and off, node for node:

| proof | nodes, on and off | G cycles, on | G cycles, off |
|---|---|---|---|
| 1215 | 1,111,451 | 32.6 | 32.7 |
| 1225 | 2,178,362 | 86.8 | 86.7 |
| 1240 | 496,460 | 18.7 | 18.7 |
| 1254 | 4,830,651 | 149.9 | 149.5 |
| 1272 | 4,341,863 | 129.2 | 128.8 |
| 1287 | 20,972,332 | 803.8 | 803.5 |
| 1300 | 28,024,916 | 985.0 | 982.2 |

The lower count of promotions and frame 0's lower freedoms cut nothing that the matching bound and the conflicts do not
cut first.  In the chain the tactic adds about 20 ms of setup a game and a taint, nothing else.

### The alternatives that are off

The measurements that retired `preclusions`, `match_bound=off` and the other branchings predate the Lagrangian bound
(the study of [the conflicts](../conflicts-of-the-matching-bound-2026-10/README.md) had `conflict-smallest` giving the
smallest proof trees at 880 and 936 but finding good incumbents late, and `conflict-largest` following the dive and
finding them early), so they were repeated with it (one thread, proof only, the strict tactics with the one named
changed; a limit of 180 s):

| proof | tactics | nodes | search s | G cycles | against the strict tactics |
|---|---|---|---|---|---|
| 1184 | strict | 52,259 | 0.47 | 1.6 | |
| 1184 | `preclusions=on` | 54,883 | 0.52, and 3.45 building the table | 12.5 | nodes +5%, cycles ×7.7 |
| 1215 | strict | 1,111,451 | 10.6 | 32.6 | |
| 1215 | `preclusions=on` | 1,569,468 | 14.6, and 4.46 building the table | 58.7 | nodes +41%, cycles +80% |
| 1215 | `branching=conflict-largest` | 9,098,354 | 66.6 | 204.8 | nodes ×8.2, cycles ×6.3 |
| 1215 | `branching=conflict-smallest` | more than 21.5 M | stopped at 180 | 554.9 | more than ×17 |
| 1215 | `branching=largest` | more than 32.0 M | stopped at 180 | 562.0 | more than ×17 |
| 1215 | `match_bound=off` | more than 67.7 M | stopped at 180 | 558.8 | more than ×17 |
| 1215 | `match_refine=0` | 1,123,153 | 9.3 | 28.7 | nodes +1%, cycles −12% |

The branchings other than `conflict-strong` lose more than before because the forced members (the matching's and the
Lagrangian bound's, tactic `fix_forced`) work only with it; without `match_bound` nothing of the last two rounds works
at all.  `legacy_bound` needs no measurement: it changes the runner-up of `blameGame`, which the default search never
computes, so it acts only with `match_bound=off`, and there it is unsound.  The last row is the open question of the
Lagrangian round, whether refinement still pays (there 0 to 26% fewer cycles without it on four proofs); its default
needs the bench set and the small games before it moves.

**The proven tactics' switches.**  None of the proven tactics has code of its own for the off position worth the name:
each is a condition or an `Option` that the on position needs anyway.  A switch costs a slot in `Tactics`, a bit of the
checkpoint header, and its share of the tests (the variants of `tests/solver.rs`, the soundness configurations, the
tripwire rows).

### Old file formats

Everything that exists only to read what older builds wrote:

* **Results lines of schemas 1 to 3**, in `src/results.rs` and `solver/tools/check_results.py`, with the fixture
  `testdata/strict_schema3_1_90.jsonl` (124 KB) and the tests that read it.
* **The committed schema-1 file** `results/default.jsonl` (1001..1231, the first run past 1000, its games resting on
  `reuse_prev` and `revise_freedoms`): nothing in it that `strict.jsonl` does not show (its scores, and its moves but
  for 1221..1224, are `strict.jsonl`'s).  The cost-curve study's chain 1001..1231 of commit e710d16
  (`solver/notebook/search/benchmarks-the-probe-pgo-and-scaling/chain.jsonl`) is that study's raw data, read for its
  seconds, and stays as data.
* **`strict.jsonl` itself was schema 4**, which the removals would make an old format: either the verifiers keep reading
  schema 4, or `strict.jsonl` is replaced by a run of the new build, after which only schema 5 is read.
* **Checkpoints**: version 2 refused with a message of its own, the tactics added after the format recorded by the bits
  of their non-default values, and the widening that resumes a checkpoint at the set width an older build chose.  The
  removals change the header anyway, so a version 4 with the remaining tactics recorded plainly can refuse everything
  older with one message.
* **The Java's golden data.**  What checks the frame builder and the setup stays, since the default search starts from
  it: `testdata/golden` (318 boards to 760), `testdata/golden_wide` (11 boards to 2048) and `testdata/games_java.txt`
  with their tests.  What reproduces the Java's tactics goes with them: `testdata/accel_java.txt` (with `reuse_prev`),
  the revised freedoms, `findPreclusions` and `blameGame`.

## Decision

The grades and the recommendations of the measurements, all taken:

| tactic or alternative | grade | measured benefit at the end of the chain | outcome |
|---|---|---|---|
| `reuse_prev` | unproven | 3.8 s of 1052 s of search (0.36%); every game it accepts is settled anyway | removed |
| `revise_freedoms` | unproven, premise refuted at n = 833 | none: the same trees in seven proofs at the end of the chain | removed |
| `legacy_bound` | unsound | none: it acts only with `match_bound=off` | removed |
| `preclusions` | sound | a loss: 1215 +80% cycles, 4.5 s to build the table | removed |
| `match_bound=off` (the Java's search) | sound | a loss: 1215 unfinished at 180 s, more than ×17 | removed |
| `branching=largest`, `conflict-smallest` | sound | a loss: more than ×17 | removed |
| `branching=conflict-largest` | sound | a loss: ×6.3 | removed |
| `same_search`, `lagrange`, `fix_forced`, `nogoods`, `forget_nogoods`, `prev_cap`, `seed_incumbent`, `prime_shortcut`, `match_refine` | proven or sound | on by default, the speed at the end of the chain (earlier rounds) | kept, as switches for A/B measurements |

The removals are commits f72c332, 8a6df90, 49ec778 and a2bc798 (on 4372b36 by the merge 39ed18b): about 4,800 lines of
code removed and 1,650 added, 3,600 and 990 of them in `src/`.

* **Tactics.**  `reuse_prev`, `revise_freedoms`, `legacy_bound`, `preclusions`, `match_bound=off` with `blameGame` and
  the counters only they used, and the tactic `branching` with its three losing values are gone, with their tests,
  tripwire rows and fixtures.  `match_refine=0` stays open.
* **No strict runs.**  With no unproven tactic left there is one kind of run: `--strict`, `UNPROVEN`,
  `Tactics::strict`, `unproven_tactics_on` and the rules of a game's own taint are gone.
* **Assumptions instead of taint.**  A range takes the game of n − 1 from the run or from the `--out` file and
  otherwise solves its first game standalone; `--use-prev` takes it from optimal.json.  A result whose proof rests on
  such a game records `"assumes": ["the game of 1300 in optimal.json is optimal"]` (say), carried down the chain as
  taint was, and `[]` otherwise.  Nothing refuses a game for what it assumes (`--allow-tainted-prev` is gone), and
  `extend-optimal` appends such games.
* **optimal.json is built in.**  The binary carries the table it was compiled with (`include_str!`) and confirms every
  score against it, a difference being a hard error naming the game: `--optimal FILE`, `--optimal none`,
  `TAXMAN_OPTIMAL` and `allow_optimal_mismatch` are gone.  Tests that need another table use the library;
  `verify-optimal [FILE]` and `extend-optimal RESULTS --out FILE [--table FILE]` still take files.
* **Old formats.**  Results lines are schema 5 (`assumes`, `prev.assumes`, `same_search.assumes`, `options.use_prev`),
  and both verifiers read schema 5 only; `results/default.jsonl` and `testdata/strict_schema3_1_90.jsonl` are deleted
  (`git show f72c332:rust/results/default.jsonl`).  The checkpoint header is version 4 (no frame 0, the tactics recorded
  plainly), every other version is refused with one message, and the widening to an older build's width is gone;
  `testdata/n1133_after_1000_nodes.ckpt` (now W = 20) and the probe's windows were made again, the probe's fingerprint
  now 7e8e724717d26545 (5b83bbebf0bb2899 with `lagrange=off`).
* **Left for the owner.**  `results/strict.jsonl` was schema 4 and unreadable by the new verifiers until the chain was
  made again from n = 1 (about 18 minutes on four threads): [the committed chain, made
  again](../../the-committed-chain-made-again-2026-10/README.md) did it.

The question of `match_refine=0` (cycles −12% and nodes +1% on 1215) would reopen with the bench set and the small
games.

## Files

The folder holds only this README.md.  The examples `sacrifice` and `repeats` were removed from master on 2026-10-17;
they build at commit 8753189: `git checkout 8753189`, then `cargo build --release --example sacrifice --example
repeats` in `rust/`.

## Commits

* 243af9b: the committed chain whose code the measurements used (unchanged to 72601a2).
* 042e4cf: the measurements and the recommendation, recorded.
* f72c332: the search alternatives that lose removed (`blameGame`, `preclusions`, the other branchings).
* 8a6df90: `reuse_prev` and `revise_freedoms` removed, and with them strict runs and results schemas 1-3.
* 49ec778: `optimal.json` built in, taint replaced by assumptions, `--use-prev`, results schema 5.
* a2bc798: checkpoint header version 4, every older layout refused with one message.
* 39ed18b, 4372b36: the merge of the cleanup with master (the slope study).
* 271c817: the merge of the cleanup into master.
* 8753189: the last commit with the `sacrifice` and `repeats` examples.
