# The committed chain to 2175 (2026-10)

Part of the [lab notebook](../README.md).

`results/strict.jsonl` was replaced once more, and now reaches 2175: a run of the clean build of commit feba6d0, the
merge of the GF(2) bound into the search ("The GF(2) bound in the search"), `taxman solve 1-2175 --threads 8 --out FILE
--checkpoint-dir DIR --checkpoint-interval 60 --progress 0` from n = 1, in one invocation, on the 10-core M1 Max with
nothing else running: 1638.6 s of wall time and 10,890 s of CPU.  Search seconds 28.4 for 1..1300 (7.5, 11.8 and 9.1
for 1..1000, 1001..1231 and 1232..1300; 1.08 M nodes, the dearest game 1287 at 1.6 s) and 1422.1 for 1301..2175 (47.4 M
nodes, against 48.9 M of the chain from 1300 on 2 threads), against 1022.5 s and 332.0 M nodes for 1..1300 in the chain
of 271c817 (4 threads, other jobs running).  The slowest games are 2172 (73.1 s), 2170 (68.5), 1475 (50.1), 1472 (37.8)
and 1430 (35.0); the queue never held more than 178 units and none spilled.  No game assumes anything.  Proofs:
trivial 13, prime formula 326, cap 192, exhausted 730, same search 914.

What it agrees with:

* The previous file (271c817) in 1..1300: every score, proof kind and method, game by game.  The moves differ in ten
  games, each a different one of the several optimal sets (539, 684-687, 770-773 and 1215): this file's sets are
  `optimal.json`'s but at 539, 770-773 and 1215, where the previous file's were the table's, and the previous file's
  differed from the table at 684-687.  Of the 192 proofs by the cap, 191 were decided before the first node and 686's
  took 29.
* The chain from `optimal.json`'s game of 1300 of the previous entry (`solver/notebook/the-gf2-bound-in-the-search-2026-10/chain.jsonl`, build 716ebd9, 2
  threads) in 1301..2175: every score, proof kind and method, and every move list but 1560-1563 (an exchange of five
  numbers of the sum 3008).  Both used the GF(2) bound, so this is a second run of the same search, not a check of the
  bound; the check on the whole board is the prover's, whose proofs of 1301..2175 were pending when this file was made
  and have since been done ([the run](../the-model-free-proofs-of-1301-2175-cost-and-the-cloud-run-2026-10/README.md), 2026-10-14
  and 15): every score of this file is the optimum.
* `optimal.json` in 1..1300 (`optimal_match` true).  It still ends at 1300 and was not extended (the lines of
  1301..2175 say `optimal_match` null); `taxman extend-optimal results/strict.jsonl` would add 875 games.

`taxman verify-results` takes 47.6 s on the file (6.4 s for its first 1300 games) and `tests/results.rs` 49 s; the
Python referee takes 1 s.  Entries above that quote `results/strict.jsonl` mean the chain of 271c817 (git has it at the
commit before the one that replaced it), unless they say otherwise; [results/README.md](../../../results/README.md)
describes the present file.
