# Searches under a floor: the data and the commands

What was measured and what it showed:
[docs/EXPERIMENTS.md](../../docs/experiments/proving-an-upper-bound-part-of-the-way-searches-under-a-floor-2026-10.md).
The runs were made on the laptop on 2026-10-18, one process a game and four at once under `nice -n 10`, with the
test-profile build of `wall` (`cargo build --profile test --example wall` in `rust/`: opt-level 2, debug assertions on)
of commit 742e88a (copied out of `target/` before the run, so that a rebuild could not change it), from `rust/`, against
the committed chain (`results/strict.jsonl`, which gives each game its optimum).  The command `target` was removed from
master on 2026-10-18; it builds at commit 9d9dae3 (`git checkout 9d9dae3` first), where it is defined in the module
comment of `examples/wall/finders.rs`, which defines every field of a line.  Every time is the CPU seconds of the
process (the laptop was shared with other work, so the wall time says little), and the searches are compared by their
nodes as well.

## Files and the commands that made them

| file | command | what it is |
|---|---|---|
| `target.jsonl` | `./bench/target/run.py bench/target/target.jsonl --wall PATH` | one line a game of the 14 in `run.py` (`wall target --known results/strict.jsonl --fractions 0.1,0.2,0.25,0.3,0.4,0.5,0.6,0.7,0.75,0.8,0.9 --cap CAP N`, the cap in CPU seconds 1.3 times the game's standalone search in `../landscape/sa/`): the gap, and for each share of it the floor search's completion, nodes and CPU seconds, and the full proof's |
| `tables.py`, `tables.txt` | `./bench/target/tables.py > bench/target/tables.txt` | the tables the entry quotes |

(The repository's `.gitignore` ignores directories called `target`, for the builds; it has an exception for this one.)

`GAMES` of `run.py` is the 14 games: the ten **main** games are those of `../landscape/sa/` whose standalone one-thread
search finished in 286 to 1,450 s there (2265, 2299, 2423, 2485, 2528, 2555, 3156, 3204, 3221, 3259: root gaps 628 to
1,113), the four **extra** (2416, 2560, 2696, 3064) cost 28 to 160 s of full proof and add points to the curve.
`run.py` runs the dearest first and skips the games the file has, so an interrupted run goes on where it was.
