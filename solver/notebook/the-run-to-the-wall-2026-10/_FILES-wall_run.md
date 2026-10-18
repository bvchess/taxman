# The run to the wall: the data and the commands

What the run was and what it showed: [the entry](../../docs/experiments/the-run-to-the-wall-2026-10.md).  The run
(2026-10-17 07:20:47Z to 2026-10-18 18:58:04Z, commit eb84a6f, one spot `c8g.16xlarge`, 35.72 instance-hours, about
$24.14) launched with `rust/tools/cloud/wall.sh` ([RUNBOOK section
12](../../tools/cloud/RUNBOOK.md#12-the-unattended-run-to-the-wall)); this directory holds what is worth keeping of its
prefix in S3.  Its games 1..3914 are in [results/strict.jsonl](../../results/README.md) (the third run there), 3441..3914
byte for byte; its checkpoints, results file and the stop marker are not kept (below).

## Files and the commands that made them

| file | command | what it is |
|---|---|---|
| `run/config` | written by `wall.sh launch` | the run's settings: the commit, the instance type, the chain's end (`CHAIN_TO` 8191), `WINDOW` 4, the wall (`GAME_HOURS` 24), the budget (`BUDGET_HOURS` 72 instance-hours, `LEDGER_MARGIN` 300 s), `MAX_INSTANCES`, the price the cost is counted at and the date (`RUN_UNTIL`, 2026-10-22T00:00Z) |
| `run/ledger` | kept by the machine in S3 | the run's clock: `start` (seconds since the epoch, 2026-10-17T07:20:47Z), the `boot` that holds it, `base` (instance-hours before it), `hours` so far (35.7161 at the end) and `updated` |
| `run/ended` | written by the machine | how it ended: the time (2026-10-18T18:58:07Z) and `reason=stopped on request (wall.sh stop)` |
| `logs/chain.log` | | `chain.sh`'s own log: its settings, the signal that stopped the solver at 18:58:04Z (status 143, after 128,236 s, a final checkpoint written) |
| `logs/chain.out.gz` | gzipped | the solver's standard output, a line a game concluded: `n=3441 score=... search nodes=... 9972.8 s optimal unknown proof=exhausted`, and the stopped game 3915's last line |
| `logs/chain.err.gz` | gzipped | its progress lines, every 60 s for the games in flight: nodes and rate, the queue, the engine's counters (`eff`, `idle`, `lock`, splits, steals, pauses) and the nogood table's |
| `logs/setup.log` | | `setup.sh` on the machine: the build and the smoke checks (`optimal.json` replayed, both referees on the committed files, the first 400 games, the prover on 2..400), all passing; the machine, the rustc and the commit |
| `logs/wall-node.log`, `logs/spot-watch.log`, `logs/save.log.gz` | | the node script's events (setup, `chain.sh`, the stop request, the end), the spot-interruption watch (it watched and saw none: one machine ran all the run) and the saves to S3 every 10 minutes |
| `reproof.jsonl.gz` | the first 3440 lines of the run's own `results/strict.jsonl`, gzipped (18 MB, 0.62 MB gzipped); `gunzip -c reproof.jsonl.gz > FILE`, then `python3 -I ../../tools/check_results.py FILE`: all checks pass (3,440 completed games, nothing assumed; `taxman verify-results FILE` is the solver's referee) | the re-proof of 1..3440, which the committed file does not hold (it has the second run's lines there); 3441..3914 are the committed ones, so they are not repeated |
| `reproof_check.py` | `./reproof_check.py` (from anywhere; standard library) | compares `reproof.jsonl.gz` with the committed file (score, proof and method: 3,440 of 3,440 equal), and prints the search seconds and nodes by range against the committed lines (2176..3440: 14.13 hours and 7.80 G nodes against 47.33 and 17.73) and the run's longest games |
| `plot_search_times.py`, `search_times_wall_run.svg` | `./plot_search_times.py search_times_wall_run.svg` | the search seconds of each searched game of the run (blue), against the second run's on 2176..3440 (gray), log scale; from `reproof.jsonl.gz` and the committed file.  A first picture, drawn when the run stood at game 3399 and left untracked at the repository's root, was this plot of that day's data (this script reproduces it byte for byte when cut at 3399); this one is the finished run |

## What is not kept

* `results/strict.jsonl` (22 MB, games 1..3914 and the stopped game's line 3915 with `score` null): 3441..3914 are the
  committed file's lines, byte for byte (compared when this was written), and 1..3440 are in `reproof.jsonl.gz`.
* `ckpt/`: the checkpoint and clock of game 3915 (2.9 MB, at 49 minutes and 138 million nodes of its search).  Only the
  build that made it, eb84a6f, can resume it (a checkpoint belongs to its build, RUNBOOK section 5), and the next run
  draws on the known games instead.
* `run/stop` (the marker `wall.sh stop` writes) and `run/boots/` (one file, the machine's boot).

## Where the other charts of the run's data are

`../ledger/cloud-20261017/report_page.py` reads `reproof.jsonl.gz` for the chain's cost per game on 2176..3440, and
`../ledger/past4300/where_chart.py` the committed file for how far the chain reaches.
