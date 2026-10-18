# Root-fitted nogood multipliers at the nodes: the data and the commands

What was measured and what it showed:
[the entry](../../docs/experiments/root-fitted-nogood-multipliers-at-the-nodes-2026-10.md).  The runs were made on the
laptop on 2026-10-18, one process a game and four at once under `nice -n 10`, with the test-profile build of `wall` and
`perf` (`cargo build --profile test --example wall --example perf` in `rust/`: opt-level 2, debug assertions on), copied
out of `target/` before the runs, from `rust/`, against the committed chain (`results/strict.jsonl`, which gives each
game its optimum).  The command `relaxed` is defined in the module comment of `examples/wall/relaxed.rs`, which defines
every field of a line.  The times are CPU seconds of the process, which was shared with other work.

## Files and the commands that made them

| file | command | what it is |
|---|---|---|
| `nodes.jsonl` | `wall relaxed --cpu 10 --steps 200 --paths 50 --fixtures DIR --suite bench/tuning/suite.jsonl --known results/strict.jsonl N`, one process a game, for N = 936, 1215, 2700, 2976, 3129, 3213, 3260, 3267, 3300, 3425 and 3440, the lines joined in order of N | a line a game: the pool of nogoods (the finder's and the sound search's, 10 CPU s each), the fit at the root, and every node measured: today's bound and the root-λ bound with both constants, and their times |
| `tables.py`, `tables.txt` | `./bench/nogood_bound/tables.py > bench/nogood_bound/tables.txt` | the tables the entry quotes |

`DIR` held the nodes fixtures: the probe's `testdata/perf/n0936.nodes` and `n1215.nodes`, and for 2700, 2976, 3129,
3213, 3260 and 3267 fixtures made the same way and not kept (each about 240 kB; the command makes them again, the
same, in a minute or two each): `perf make-nodes N --known results/strict.jsonl --every 2000 --keep 300 --fixtures
DIR` (the proof of N seeded with the optimum, one thread, in runs of 2,000 steps each resuming the last one's
checkpoint; 300 of the units it handed its searcher, evenly spaced).  The proofs took 109,331 (2700), 157,075 (2976),
308,708 (3129), 439,812 (3213), 331,761 (3260) and 129,513 (3267) steps and handed 854 to 3,911 distinct units.  3300,
3425 and 3440 have no fixture: their nodes are the tuning suite's units (10 a game, from checkpoints of searches at the
end of the chain, each with the nodes of its subtree when the default solved it).  Every game has 50 nodes on the way to
its optimum besides, which check the bound's validity.
