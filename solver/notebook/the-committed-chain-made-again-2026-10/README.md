# The committed chain, made again (2026-10)

Part of the [lab notebook](../README.md).

Started 2026-10-13.  Concluded 2026-10-13.

**Question.**  Does the build after the tactics cleanup (commit 271c817) make the committed chain again?
**Answer.**  Yes: it made the same moves, proofs and scores from n = 1 in 1022.5 search seconds against 1052.2, and
`results/strict.jsonl` was replaced by its file.

## Method

The clean build of commit 271c817, the cleanup of [which tactics earn their
keep](../search/which-tactics-earn-their-keep-2026-10/README.md) (every run strict, schema 5), ran `taxman solve 1-1300
--threads 4 --out FILE --checkpoint-dir DIR --checkpoint-interval 60 --progress 0` from n = 1 in one invocation.  It ran
on the laptop while four other jobs of two threads each ran, and took 1048.7 s of wall time and 3965 s of CPU.

## Results

| search seconds | 1..1000 | 1001..1231 | 1232..1300 | all |
|---|---|---|---|---|
| this chain (271c817) | 46.3 | 167.9 | 808.4 | 1022.5 |
| the chain of 243af9b | 51.3 | 187.8 | 813.2 | 1052.2 |

* Every score equals optimal.json's, and every move list, proof and method is the earlier chain's: same_search 545,
  exhausted 416, prime formula 211, cap 115, trivial 13.  No game assumes anything.  The move sets are optimal.json's
  but at 684..687 ([the results](../../../results/README.md#optimaljsonlgz) has the rest).
* The steps of the other 1299 games total 279.0 M against 280.1 M before.  Game 1232 took 63% more steps (53.0 M against
  32.6 M) at a higher rate (256 k/s against 178 k/s, under another load), so its 206.7 s against 183.6 s are the extra
  steps.

## Decision

Kept: `results/strict.jsonl` is this chain.  It is the rerun that "Which tactics earn their keep" left for the project's
owner, and `tests/results.rs` no longer skips the file.  Entries that quote `results/strict.jsonl` from the first
replacement on mean the chain of 243af9b.

## Files

The folder holds only this README.md.  The chain itself is `results/strict.jsonl`.

## Commits

* 271c817: the build that made the chain (the merge of the tactics cleanup, 2026-10-13).
* 243af9b: the chain this one repeats.
* 0133afe: the replacement of `results/strict.jsonl` by this chain (2026-10-13).
