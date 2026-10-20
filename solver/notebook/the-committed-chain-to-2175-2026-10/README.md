# The committed chain to 2175 (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-14.  Concluded 2026-10-14.

**Question.**  Does the GF(2) bound let one invocation make the committed chain from n = 1 to 2175?  **Answer.**  Yes:
`taxman solve` proved every game of 1..2175 in 1638.6 s of wall time on 8 threads, assuming nothing, with every score
the earlier chains' scores; `results/strict.jsonl` became this file.

## Method

The run is the clean build of commit feba6d0, which merged the GF(2) bound into the search, on the 10-core M1 Max with
nothing else running, in one invocation from n = 1:

```
taxman solve 1-2175 --threads 8 --out FILE --checkpoint-dir DIR --checkpoint-interval 60 --progress 0
```

## Results

The run took 1638.6 s of wall time and 10,890 s of CPU.  No game assumes anything.  The proofs are trivial 13, prime
formula 326, cap 192, exhausted 730 and same search 914.  The queue never held more than 178 units, and none spilled.

| Games | Search seconds | Nodes | Note |
|---|---|---|---|
| 1..1300 | 28.4 | 1.08 M | 7.5, 11.8 and 9.1 s for 1..1000, 1001..1231 and 1232..1300; dearest game 1287, 1.6 s |
| 1301..2175 | 1422.1 | 47.4 M | the chain from 1300 on 2 threads took 48.9 M nodes |

The chain of 271c817 (4 threads, other jobs running) took 1022.5 search seconds and 332.0 M nodes for 1..1300.  The
slowest games of this run are 2172 (73.1 s), 2170 (68.5), 1475 (50.1), 1472 (37.8) and 1430 (35.0).

The file agrees with three earlier results:

* **The previous file (271c817) in 1..1300**: every score, proof kind and method, game by game.  The moves differ in ten
  games, each a different one of the several optimal sets (539, 684-687, 770-773 and 1215).  This file's sets are
  `optimal.json`'s except at 539, 770-773 and 1215, where the previous file's were the table's, and the previous file's
  differed from the table at 684-687.  Of the 192 proofs by the cap, 191 were decided before the first node and 686's
  took 29.
* **The chain in 1301..2175** from `optimal.json`'s game of 1300 (build 716ebd9, 2 threads; its
  [chain.jsonl](../search/the-gf2-bound-in-the-search-2026-10/chain.jsonl)): every score, proof kind and method, and
  every move list but 1560-1563 (an exchange of five numbers of the sum 3008).  Both used the GF(2) bound, so this is a
  second run of the same search, not a check of the bound.  The check on the whole board is the prover's, whose proofs
  of 1301..2175 were done on 2026-10-14 and 15
  ([the run](../the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md)): every score of this file
  is the optimum.
* **`optimal.json`** in 1..1300 (`optimal_match` true).

`taxman verify-results` takes 47.6 s on the file (6.4 s for its first 1300 games) and `tests/results.rs` 49 s; the
Python referee takes 1 s.

## Decision

`results/strict.jsonl` was replaced by this file, which [results/README.md](../../../results/README.md) describes.
Entries older than this one that quote `results/strict.jsonl` mean the chain of 271c817, which git holds at the commit
before the one that replaced it, unless they say otherwise.

`optimal.json` still ends at 1300 and was not extended: the lines of 1301..2175 say `optimal_match` null.
`taxman extend-optimal results/strict.jsonl` would add 875 games.

## Files

* `README.md`: this record.  The run's output is `results/strict.jsonl`, outside the folder.

## Commits

* feba6d0: the GF(2) bound merged into the search, the build of this run.
* 271c817: the build of the previous chain, the one this file replaced.
* 716ebd9: the build of the 2-thread chain of 1301..2175 that this file agrees with.
