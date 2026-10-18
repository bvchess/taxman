# The committed chain, made again (2026-10)

Part of the [lab notebook](../README.md).

`results/strict.jsonl` was replaced once more, by a run of the clean build of commit 271c817, the cleanup of "Which
tactics earn their keep" (every run strict, schema 5): `taxman solve 1-1300 --threads 4 --out FILE --checkpoint-dir DIR
--checkpoint-interval 60 --progress 0` from n = 1, in one invocation, on the laptop while four other jobs of two
threads each ran: 1048.7 s of wall time and 3965 s of CPU.  Search seconds 46.3, 167.9 and 808.4 for 1..1000,
1001..1231 and 1232..1300 (1022.5 in all), against 51.3, 187.8 and 813.2 (1052.2) of the chain of 243af9b.  Every
score equals optimal.json's, and every move list, proof and method is that chain's (same_search 545, exhausted 416,
prime formula 211, cap 115, trivial 13), with no game assuming anything; the move sets are optimal.json's but at
684..687 ([results](../../../results/README.md#strictjsonl) has the rest).  The node counts of the other 1299 games total
279.0 M against 280.1 M before; 1232 took 63% more nodes (53.0 M against 32.6 M) at a higher rate (256 k/s against
178 k/s, under another load), so its 206.7 s against 183.6 s are the extra nodes.  This is the rerun that "Which tactics
earn their keep" left for the owner: `tests/results.rs` no longer skips the file.  Entries above that quote
`results/strict.jsonl` from the first replacement on mean the chain of 243af9b.
