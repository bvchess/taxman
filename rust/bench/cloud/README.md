# Graviton calibration: the scripts and the data

What was measured and what it showed: [docs/EXPERIMENTS.md](../../docs/EXPERIMENTS.md#graviton-calibration-2026-10).
This directory holds the scripts that made the measurements and the data they produced.

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
