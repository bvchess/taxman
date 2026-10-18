# Seeding the start incumbent from a neighbour's heuristic game: the data and the commands

What was measured and what it showed:
[docs/EXPERIMENTS.md](../../../docs/experiments/seeding-the-start-from-a-neighbours-heuristic-game-2026-10.md).  The
runs were made on the laptop on 2026-10-17, one thread each, six at once under `nice -n 10`, with the test-profile
build of `wall` (`cargo build --profile test --example wall` in `rust/`) at commit a8ee892, from `rust/`, over
2176-3440 against the committed chain (`results/strict.jsonl`).  The command `chain` was removed from master on
2026-10-18; it builds at commit 9d9dae3 (`git checkout 9d9dae3` first), where each `chain` line is defined in the module
comment of `examples/wall.rs`; `shortfall` is the optimum less the start's score.  (`wall start`, the command of `c`,
is still there: `cargo build --release --features studies --example wall`.)

| file | command | what it is |
|---|---|---|
| `a.jsonl` | `wall chain --known results/strict.jsonl --out a.jsonl 2176-3440` | A: up, seeded from the game before, unbroken |
| `a64.jsonl`, `a256.jsonl` | the same with `--restart 64`, `--restart 256` | A64 and A256: up, standalone at every 64th and 256th game |
| `b.jsonl` | the same with `--down` | B: down from 3440, seeded from the game after, unbroken |
| `r1.jsonl` | the same with `--restart 1` | the standalone start of every game, here on the laptop beside the others (equal, game for game, to `lower` in `../cal.jsonl`) |
| `c.jsonl` | `wall start --known results/strict.jsonl 2176-3440 > c.jsonl` | C: the start seeded from the proven game of n − 1 (`pairs` is its promotion sum; the matching dive is not part of it) |
| `seeding.py`, `seeding.txt` | `./seeding.py > seeding.txt` | the analysis (it reads `land.py` and `../cal.jsonl`) and its output, which the entry quotes |
