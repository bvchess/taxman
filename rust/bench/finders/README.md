# Finders for good games: the data and the commands

What was measured and what it showed:
[docs/EXPERIMENTS.md](../../docs/experiments/finders-for-good-games-that-are-not-sound-2026-10.md).  The runs were
made on the laptop on 2026-10-17, one process a game and four at once under `nice -n 10`, with the test-profile build
of `wall` (`cargo build --profile test --example wall` in `rust/`), from `rust/`, against the committed chain
(`results/strict.jsonl`).  The commands `sound` and `finders` are defined in the module comment of
`examples/wall/finders.rs`, which defines every field of a line.  Every time is the CPU seconds of the process (the
laptop was shared with other work, so the wall time says little), and the searches are compared by their nodes as well.

`run.py` runs a command over a list of games, four processes at a time, and appends each game's line to the stage's
file as it ends; a run that is interrupted goes on where it was.  `tables.py` makes the tables of the entry from the
files and `../landscape/cal.jsonl`; its output is `tables.txt`.

## Files and the commands that made them

`GAMES1` is the 40 games of stage 1: 2180 2196 2200 2264 2312 2397 2416 2430 2440 2451 2478 2556 2560 2565 2592 2601
2632 2640 2691 2696 2700 2704 2712 2720 2799 2849 2891 2925 2964 2976 3000 3003 3010 3064 3128 3129 3213 3260 3267
3272.  The 22 of them in `../landscape/sa/` were picked by the time of their standalone search there (5 to 300 s, no
primes or repeats; four have a start that is already the optimum, as 13% of the games do); the 18 others, of every band
of 250 in n, by the nodes of the chain's search of them (100,000 to 1,200,000), with a start short of the optimum.

| file | command | what it is |
|---|---|---|
| `stage1.jsonl` | `./bench/finders/run.py sound bench/finders/stage1.jsonl --from opt --cap 600 --shuffle GAMES1`, then the same with `--from start --cap 900` | stage 1: the sound search of each of the 40 games, one thread, no seed from n − 1 and no ceiling, from the known optimum (`from` is `opt`: (ii)) and from today's start (`start`: (i)); 80 lines |
| `stage2.jsonl` | `./bench/finders/run.py finders bench/finders/stage2.jsonl --cpu 5 --shuffle 2176-3440/5` | stage 2: the three methods and the start, each of the 253 games 2176, 2181, ..., 3436, 5 CPU s a method (it predates the field `lns_found_ms`) |
| `stage2-20s.jsonl` | `./bench/finders/run.py finders bench/finders/stage2-20s.jsonl --cpu 20 --shuffle 2176-3440/25` | the same, 20 CPU s a method, on every 25th game (51) |
| `stage3-lns-5s.jsonl`, `stage3-lns-1s.jsonl` | `./bench/finders/run.py sound bench/finders/stage3-lns-5s.jsonl --from lns --cpu 5 --cap 900 --shuffle GAMES1`, and with `--cpu 1` | stage 3: the sound search of the 40 games seeded with the game the LNS finder returns after 5 and after 1 CPU s |
| `tuning/*.jsonl` | `./bench/finders/run.py sound bench/finders/tuning/NAME.jsonl --from lns --cap 0.01 --cpu 5 2178-3428/50 [--fix LO-HI] [--unit-nodes K]`, and `--from restricted` with `[--extra K] [--pool value|chosen]` (`value` was the default while the `value` files were made; `chosen` is the default now) | the light tuning on 26 games n = 2178 + 50k, which stage 2 does not play (n = 3 mod 5); the name says the setting, the file the game's line (`--cap 0.01` stops the sound search after the finder, whose game is the line's `lns` or `restricted`) |
| `tables.py`, `tables.txt` | `./bench/finders/tables.py > bench/finders/tables.txt` | the tables the entry quotes |
