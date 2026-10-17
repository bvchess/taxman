# The ledger: the data and the commands

What was run and what it showed: [the entry](../../docs/experiments/the-ledger-bounds-from-both-sides-2026-10.md).
The program is `examples/ledger/` (its module comment defines every field of `ledger.jsonl` and `events.jsonl`), run
from `rust/` with the non-release build of the laptop: `cargo build --profile test --example ledger --bin taxman`
(opt-level 2, debug assertions on), so `ledger` is `target/debug/examples/ledger` and `taxman` `target/debug/taxman`.
Everything here ran under `nice -n 10`.

## Files and the commands that made them

| file | command | what it is |
|---|---|---|
| `cloud.sh` | `bench/ledger/cloud.sh FIRST-LAST` (environment in its comment) | the experiment on one machine: `ledger run` with the truth hidden, then `ledger report --known results/strict.jsonl`; the cloud run of 2176..3440 is [RUNBOOK section 14](../../tools/cloud/RUNBOOK.md#14-the-ledgers-experiment) |
| `events.py` | `./events.py DIR [CHAIN.jsonl]` | what a run spent its time on, beyond the report: setup, maps and searches, stopped searches and their dual bounds, the relations' changes by rule; with the chain's results file over the same stretch, its seconds beside |
| `dry/` | `OUT=DIR LEDGER_BIN=target/debug/examples/ledger WORKERS=6 CPU_HOURS=2 bench/ledger/cloud.sh 1301-1500` (2026-10-17, 20:01:50 to 20:04:16 UTC), with DIR a scratch directory whose files were copied here | the laptop's dry run of 1301..1500, the truth hidden: `run/ledger.jsonl.gz` and `run/events.jsonl.gz` (gzipped after the run; the checkpoints were all deleted, every board being proven), `run.err` (its status lines), `log` (the stages' times); `report.txt` was made again after the run by `ledger report bench/ledger/dry/run --known results/strict.jsonl` with the build that counts the last proof's CPU in the total (the run's own report said 595 s with 199 boards at it) |
| `dry/chain.jsonl.gz` | `taxman solve 1301-1500 --threads 1 --window 1 --progress 0 --out dry/chain.jsonl`, then gzipped | the chain over the same stretch on one thread with the same build, standalone at 1301 (no game of 1300), for the comparison: 447.5 s in all (`/usr/bin/time`: 449 s wall, 446.7 s user) |
| `dry/events.txt` | `./events.py dry/run dry/chain.jsonl` | the output the entry quotes |
