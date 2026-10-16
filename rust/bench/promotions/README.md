# A3: split by the number of promotions (2026-10)

`promotions.jsonl` is the output of `examples/promotions.rs`, the diagnostic of [agenda A3](../../docs/AGENDA.md): for
each game, the root's *window* of total sizes that can still beat the optimum's promotion sum, and, for every node of
the game's fixture, whether the total size of its heaviest completion lies below, inside or above it (one `node` line
a node, one `game` line a game).  The entry is [Split by the number of
promotions](../../docs/EXPERIMENTS.md#split-by-the-number-of-promotions-a3s-test-2026-10).

* 936 and 1215 read the probe's fixtures (`testdata/perf`): `promotions --known results/strict.jsonl 936 1215`.
* 2112 reads `n2112.nodes` here, made by `perf make-nodes 2112 --known results/strict.jsonl --every 500 --keep 300
  --fixtures bench/promotions` (the proof's 15,288 nodes handed the searcher 212 distinct units; all are kept):
  `promotions --known results/strict.jsonl --fixtures bench/promotions 2112`.

Both are deterministic and take seconds on one thread; the commands run from `rust/`.
