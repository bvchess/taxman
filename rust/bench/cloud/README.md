# The cloud measurements: the scripts and the data

Two runs on EC2: the Graviton calibration (below; what it showed:
[docs/EXPERIMENTS.md](../../docs/EXPERIMENTS.md#graviton-calibration-2026-10)), and the scaling to 192 threads on a
`c8g.48xlarge`, whose files are in `c8g48/` ([the last section](#scaling-to-192-threads-c8g48); what it showed:
[docs/EXPERIMENTS.md](../../docs/EXPERIMENTS.md#scaling-to-192-threads-2026-10)).  This directory holds the scripts that
made the measurements and the data they produced.

## Scripts

| file | what it does |
|---|---|
| `calibrate.sh PHASE OUTDIR ...` | one phase of the calibration of a machine: `probe`, `ladder`, `prover`, `contention-solver`, `contention-prover`, `scaling`, `chain`; the header lists the phases and their arguments |
| `instance.sh OUTDIR` | the phases of one cloud instance in the order they ran (the chain is run afterwards, by hand) |
| `timed.sh`, `copies.sh` | one run under `/usr/bin/time` (GNU `-v` on Linux, BSD `-l` on macOS), and K copies of a run at once |
| `parse.py MACHINE DIR` | the logs of a machine as one JSON line per run |
| `summary.py LAPTOP.jsonl MACHINE.jsonl...` | the tables of the entry, from those lines |

## Data

| file | content |
|---|---|
| `laptop.jsonl`, `c7g.jsonl`, `c8g.jsonl` | one line per run (`parse.py`): label, kind, game, threads, wall, user and system seconds, maximum resident set size, and the program's own figures (search seconds and nodes of a proof, the prover's nodes and seconds, the probe's time and fingerprint with its kernels in ns an operation); then one line per game of each chain run |
| `machines/` | what each machine is |
| `c7g.resolve-2126-2175.jsonl`, `c8g.resolve-2126-2175.jsonl` | the results lines of games 2126..2175 solved again on 64 threads, on a copy of `results/strict.jsonl` cut after game 2125; the scores are the committed file's |
| `c8g.chain-2176.jsonl` | the results lines of the games 2176..2277 that `taxman solve 2176-4095 --threads 64` solved in 15 minutes on a copy of `results/strict.jsonl` (checked with `tools/check_results.py` beside it) |
| `c8g.startup-and-fixed-cost.jsonl` | games 2126 and 2127 (repeats of 2125's search) solved at 1, 8 and 64 threads: the fixed cost of a game, and the start-up of a run |

These are measurements, not results: nothing here replaces or extends `results/strict.jsonl`.  The runs used the
binaries as built by `cargo build --release --bins --examples` and, in `rust/prover/`, `cargo build --release`, with
`RUSTFLAGS="-C target-cpu=native"` (what `tools/cloud/setup.sh` does), from `git archive` of the code of 407645f.  The
cloud runs were made before the prover's directory was renamed; `calibrate.sh` has the new path.

## Scaling to 192 threads (`c8g48/`)

One `c8g.48xlarge` (192 vCPUs, two sockets, two NUMA nodes; `machines/c8g48.txt`) on 2026-10-15, the code of 55287ff
built by `tools/cloud/setup.sh`, the items of [bench/wall/README.md](../wall/README.md#for-a-cloud-machine), "For a
cloud machine", that the entry names.  The 64-thread runs of the window and of the next games are bound to 64 CPUs of
node 0 (`taskset`); the one-thread `wall setup` (item 1) ran beside them on CPU 191, and its lines are
[../wall/setup_c8g.jsonl](../wall/setup_c8g.jsonl).

| file | content |
|---|---|
| `c8g48/measure.sh` | the run: items 7, 8 (a and b), 1, 3 and 4 in that order, one process at a time, every output under one directory copied to S3 after each item; then 2226..2276 again at windows 1 and 16, and at 32, on both thread counts |
| `c8g48/extras.sh`, `c8g48/extras2.sh` | run after it: the first ladder again and a window of 4 (`extras.sh`); the games from 2277 on at 192 and at 64 threads, five minutes each (`extras2.sh`) |
| `c8g48/runs.py runs DIR`, `runs.py games DIR` | `runs.jsonl` and `games.jsonl` from the directory the scripts wrote |
| `c8g48/steps.log` | every run of the three scripts: its command, the time it started and ended, its exit status and wall seconds |
| `c8g48/runs.jsonl` | one line per run (`runs.py runs`): threads, window, CPUs, status, wall, user and system seconds, maximum resident set size, context switches, page faults, and `regress`'s own seconds |
| `c8g48/games.jsonl` | one line per game of each `solve` run (`runs.py games`): method, proof, score, nodes, threads, search and total seconds |
| `c8g48/ladder.txt` | the five ladders (item 7): each line of a search and the scaling tables, under the command that made them |
| `c8g48/progress.txt` | the progress lines of 2184 with a checkpoint every second and every 300 s (item 3), of 4000 standalone (item 4), and of the runs from 2277 on |
| `c8g48/window-trace.txt` | the window's events (`TAXMAN_WINDOW_TRACE`) for 2226..2276 at 192 threads with a window of 16 |
| `c8g48/checks.txt` | the scores, proofs and `same_search` of every window run's 2226..2276 against `c8g.chain-2176.jsonl`, the trace's counts, and both referees on the window-16 files at 64 and 192 threads |
| `c8g48/solve-4000.jsonl` | the line of the standalone search of 4000 on 192 threads (exhausted in 43 s) |

Like the calibration's files, these are measurements, not results: the games of the runs (2226..2276 again, 2184 again,
2277..2336 and 4000 new) are in no results file.
