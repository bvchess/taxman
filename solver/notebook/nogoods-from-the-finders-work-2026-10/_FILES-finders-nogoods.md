# Nogoods from the finder's work: the data and the commands

What was measured and what it showed:
[docs/EXPERIMENTS.md](../../../docs/experiments/nogoods-from-the-finders-work-2026-10.md).  The runs were made on the
laptop on 2026-10-18, one process a game and four at once under `nice -n 10`, with the test-profile build of `wall`
(`cargo build --profile test --example wall` in `rust/`), from `rust/`, against the committed chain
(`results/strict.jsonl`).  The command `nogoods` was removed from master on 2026-10-18; it builds at commit 9d9dae3
(`git checkout 9d9dae3` first), where it is defined in the module comment of `examples/wall/nogoods.rs`, which defines
every field of a line.  Every time is the CPU seconds of the process (or the wall seconds of a bound, which is one
thread), and the budgets of the finder and of the control are CPU seconds, so the pools of two runs of one game differ a
little (a budget cuts a search at a different step).

`run.py` runs `wall nogoods` over a list of games, four processes at a time, and appends each game's line to the file
as it ends (a run that is interrupted goes on where it was); it takes the helpers of `../run.py`.  `tables.py` makes
the tables of the entry from the files and `../../../results/bounds.jsonl`; its output is `tables.txt`.

## Files and the commands that made them

The eight games with known optima are 3016, 3156, 3216, 3276, 3311, 3336, 3381 and 3396: games of 3000-3440 that
the chain searched (not primes, not repeats, not stopped at the ceiling), picked from `../stage2.jsonl` for a spread
of root gaps (766 to 1,427; `upper` less `opt`).  The open games are 4300, 4522, 4761 and 4788.

| file | command | what it is |
|---|---|---|
| `known-10s.jsonl`, `known-120s.jsonl` | `./bench/finders/nogoods/run.py bench/finders/nogoods/known-10s.jsonl --cpu 10 --shuffle 3016 3156 3216 3276 3311 3336 3381 3396`, and with `--cpu 120` | the eight games with known optima, the finder and the sound search 10 and 120 CPU s each |
| `open-10s.jsonl`, `open-120s.jsonl` | `./bench/finders/nogoods/run.py bench/finders/nogoods/open-10s.jsonl --cpu 10 4300 4522 4761 4788`, and with `--cpu 120` | the four open games |
| `plain-known-10s.jsonl`, `plain-known-120s.jsonl`, `plain-open-10s.jsonl`, `plain-open-120s.jsonl` | the same four commands, made before the fit had momentum (commit 05e3744: plain subgradient steps) | the first pass, kept because it shows that the fit decides the answer for a large pool: with plain steps the finder's pool of 120 s, thirteen thousand nogoods, closed less of the gap than the pool of 10 s (the entry has the numbers) |
| `tables.py`, `tables.txt` | `./bench/finders/nogoods/tables.py > bench/finders/nogoods/tables.txt` | the tables the entry quotes |
