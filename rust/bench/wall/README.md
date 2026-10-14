# The wall regime: the data and the commands

What was measured and what it showed: [docs/EXPERIMENTS.md](../../docs/EXPERIMENTS.md#the-wall-regime-2026-10).
This directory holds the data that `examples/wall.rs` wrote (its module comment defines every field) and `wall.py`,
which makes the entry's tables from them (`./wall.py > wall_output.md`).  Everything ran on the M1 Max, one process
at a time under `nice -n 10`, one thread unless a line says otherwise, with other jobs on the machine (load 4 to 5), on
the code of this branch at the merge of master 5252d5d (9c99ca1).  Times are wall seconds of one thread; the cycles
and instructions of the `nodes` lines are the process's, and those of the start mode include the search's own setup.

## Files and the commands that made them

`wall` is `target/release/examples/wall` (`nice -n 10 cargo build --release -j 4 --example wall`), run from `rust/`;
`K` is `--known results/strict.jsonl`.

| file | command | what it is |
|---|---|---|
| `setup.jsonl` | `wall setup K 1000 1300 1600 1900 2100 2175 2200 2500 2800 3100 3400 3700 4000 4095` | the fixed costs of a game piece by piece, the sizes, the root's GF(2) bound and the gap |
| `setup_widths.jsonl` | `wall setup K --repeat 2 --width W N` for N = 2170 (W = 36, 48, 64) and 3000 (48, 64) | the same at other set widths |
| `start.jsonl` | `wall start K 1301 1306 ... 2171` (every fifth board) | the starting incumbent's local search: singles against pairs |
| `nodes_start_low.jsonl`, `nodes_opt_low.jsonl` | `wall nodes K --no-seed --nodes 5000 1300 1600 1900 2170`, and with `--mode opt` | the cost of a node from the start incumbent and in the proof of the optimum |
| `nodes_start_high.jsonl` | `wall nodes --no-seed --nodes 5000 2200 2500 2800 3100 3400 3700 4000 4095` | the cost of a node past 2175 |
| `gaps.jsonl` | `wall nodes K --no-seed --nodes 1000 1402 1502 ... 2102` and `2252 2352 ... 4052` | root gaps over the unseeded start (and the optimum where known), and node costs, every 100 |
| `gaps_c8g.jsonl` | `wall nodes --known KNOWN --nodes 1000 2184 2185 2190 2200 2220 2240 2250 2254 2256 2260 2268 2275`, `KNOWN` = `results/strict.jsonl` followed by `../cloud/c8g.chain-2176.jsonl` | the root gaps of the dear games past 2175, against their optima (seeded starts) |
| `widths_1300.jsonl`, `widths_2170.jsonl` | `wall nodes K --mode opt --nodes 5000 --width W N`, each width twice | the same proof at several set widths |
| `widths_3000.jsonl` | `wall nodes --no-seed --nodes 5000 --width W 3000`, W = 48, 56, 64, twice | the same, past 2175 |
| `ckpt_4000.jsonl` | `wall nodes --no-seed --nodes 5000 --ckpt-dir DIR --every 0 4000`, then `--every 1` | what checkpoints cost a thread at 4000 |
| `grow_1430.jsonl`, `.progress` | `wall grow K --delta 0 --nodes 1000000 --progress 2 --ckpt-dir DIR --every 10 1430` | a GF(2) proof of 706,620 nodes: monitor lines and summary (stdout), progress lines (stderr) |
| `grow_1230_matching.jsonl`, `.progress` | `wall grow K --tactic gf2=off --tactic lagrange=on --nodes 15000000 --progress 2 --ckpt-dir DIR --every 10 1230` | a matching-path proof of 10.45 million nodes |
| `grow_2170_t64c1.jsonl`, `.progress` | `wall grow K --threads 64 --cores 1 --seconds 90 --progress 10 --ckpt-dir DIR --every 10 2170` | 64 workers taking turns on one CPU: queue, checkpoint size, memory |
| `frames.jsonl` | `wall frames 1024 2048 3000 4095 4096 4500 5000 5500 6000 6500 7000 7500 8000 8191` | the frames, sizes and ranks beyond the widest set |
| `profile_4000_search.txt` (`.jsonl`) | macOS `sample` (1 ms) for 30 s of `wall nodes --no-seed --nodes 30000 4000`, attributed by `tools/profile/attribute.py` | where a node's time goes at 4000 |
| `profile_4000_setup.txt` | the same for 60 s of `wall setup --repeat 3 4000` | where the fixed costs go at 4000 |

In the two profiles `attribute.py` could not symbolicate the inlined frames of this binary, so its kinds and purposes
say `other`; the innermost functions are the useful part, as truncated symbols: `gf2::meet::Meet<3>::shortest_path`
(`...Meet...13s`), `Meet<3>::add` (`...3ad`), `Gf2Bound<64>::evaluate` (`...8eva`), `Meet<3>::insert_both` (`...11i`),
`Meet<3>::apply_path` (`...10a`), `model::peel::Scratch<64>::winnable` (`...peel...Scratch`), `hives::Apiary<64>::
count_freedoms` (`...6Apiary...14cou`), `model::peel::reduce_from` (`...11reduce_from`).

## For a cloud machine

What could not be run lightly on the laptop, as commands for an instance set up by `tools/cloud/setup.sh` (from
`rust/`, `K` as above; `K2` = `--known KNOWN` with `KNOWN` the committed results followed by the c8g chain's lines,
as for `gaps_c8g.jsonl`).  Each line is one process; the one-thread ones can run side by side, one a vCPU (the
calibration found no contention among 64 copies).

1. The fixed costs on Graviton, every 25 boards past 2175 (one thread, about 15 minutes):
   `wall setup K $(seq 2200 25 4095) > setup_c8g.jsonl`.
2. Large GF(2) trees at the set width of 64 words, on one thread each, a time series of 10^7 nodes and more: the dear
   games of the c8g chain, `wall grow K2 --nodes 40000000 --progress 30 --ckpt-dir ck_N --every 60 N > grow_N.jsonl
   2> grow_N.progress` for N = 2184 (25.6 million nodes on 64 threads), 2260, 2268, 2254 (hours each: about 0.5 ms a
   node).
3. The pause of a checkpoint with 192 (or 64) threads near the wall, the real one rather than one worker's: on a copy of
   the committed results cut after 2183 and extended with the c8g lines to 2183, `taxman solve 2184 --threads 192
   --out COPY --checkpoint-dir DIR --checkpoint-interval 1 --progress 10 --time-limit 600`; the end of its progress
   line gives `pauses N (x%) max Y park Z`.  Then the same at `--checkpoint-interval 60`.
4. Memory, the queue and the checkpoint at 192 threads at 4000, standalone (no game of 3999): `/usr/bin/time -v
   taxman solve 4000 --threads 192 --out scratch.jsonl --checkpoint-dir DIR --checkpoint-interval 60 --progress 10
   --time-limit 600`, with `ls -l DIR` once a minute (maximum resident set size from `time`; the queue and the pauses
   from the progress lines; `max_queue` and `checkpoints` from the incomplete results line).
5. The results check: `time taxman verify-results results/strict.jsonl` on the instance, and on the instance's own
   results file once the chain has gone past 2500 (the model in the entry says 104 s at 2500 and 10 minutes at 4095).
6. What an S3 save costs: with `S3_DEST` set, `/usr/bin/time -v rust/tools/cloud/save.sh` three times while a chain
   runs (user and system seconds per save; the entry estimates 1.5 s).
