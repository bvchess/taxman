# Tuning the search's settings on problems like the end of the chain's

The search has settings: the tactics of `taxman solve --tactic` (`lagrange`, `symmetry`, `match_refine`, `force_window`,
`branching`, ...), and more to come.  They were tuned on games near n = 1000, where a root's gap is 130 to 300; the
hardest games (n above 3,400, gaps over 1,000, billions of nodes) may want other values.  A setting's effect on the tree
can only be measured by searches run to completion, and one of those games is far too big for that.  So the benchmark
is made of *subtrees* of real searches of them.  A *unit* is a node of a search, its promotions and the candidates its
loop has still to try (`search::TaxUnit`): the work the engine hands from thread to thread and writes to its
checkpoints.  Solved to completion, a unit is a search of its own, one that finishes; and a search on one thread is
deterministic, so the nodes of two settings compare exactly, and their CPU times with the noise of the machine only.
The suite also has a few small whole games.

## The suite

`suite.jsonl` has 46 problems: 40 units, 10 from each of four games, with 33,333 to 1,248,609 nodes at the default
settings, and 6 whole games with 277,366 to 450,606.  All together 16.1 million nodes: 4,250 CPU seconds, 1.2 hours
of one CPU, at the default (measured with 8 jobs at once on an M1 Max, the build of the commit in the first line of the
file).

The suite was made when the default of `force_window` was 80, and its units were chosen for 100,000 to 3 million nodes
at that default (the units' nodes were 102,571 to 2,605,379, the whole games' 500,530 to 608,203, 36.2 million in all,
7,160 CPU seconds).  When the default became `all` the suite's `default` lines were measured again
(`make_suite.py refresh --new-trees`: the same best sum on every problem, 0.45 of the nodes and 0.59 of the CPU seconds
in all); the figures below and in the suite are the new default's, and the nodes of the 31 problems that the campaign
ran with `force_window=all` on a Graviton machine are the same on this laptop.  The campaign's own `default` is the
old one ([its README](campaign-2026-10/README.md)).

| game | checkpoint | units in it | in the suite | nodes | default CPU s |
|---|---|---|---|---|---|
| 3441 | the chain's, 626 million nodes after a 50-minute search on 64 threads, stopped (`chain-2176/ckpt` in the project's bucket) | 6,151 | 10 units | 36,500 to 1,099,410 | 9 to 314 |
| 3440 | `taxman solve 3440 --threads 4 --time-limit 120`, from `strict.jsonl` cut to 3439 | 340 | 10 units | 61,285 to 870,739 | 19 to 236 |
| 3425 | the same, cut to 3424 | 253 | 10 units | 33,333 to 721,555 | 9 to 216 |
| 3300 | the same, cut to 3299 | 190 | 10 units | 40,811 to 1,248,609 | 11 to 261 |
| whole games | 3128, 3129, 3213, 3270, 3272, 3275: the smallest of the searched games of 3000-3440 whose proof from the optimum takes 500,000 to 3 million nodes on one thread at the old default (14 were solved to see: 3010 to 3275, with 84,454 to 941,354) | | 6 games | 277,366 to 450,606 | 53 to 219 |

A unit is solved starting from the best set known, so that it is a proof where the game is known: for 3300, 3425 and
3440 (in `results/strict.jsonl`) the incumbent is the game's optimum, and the subtree is proved (nothing in it beats the
optimum); for 3441, which is unfinished, it is the checkpoint's incumbent (274,462), and a subtree could hold something
better (none of the 78 subtrees of it solved in making the suite did).  A whole game is the proof from the optimum,
`ladder --seed-opt` with the setup left out (its nodes are the same).  The default's nodes are exact and the same on
every machine (`default.nodes` of each line); its CPU seconds are this laptop's, for scale only: the driver measures the
default again on the machine it runs on.

A unit alone differs from the same unit inside a search in one respect: a searcher learns nogoods as it goes and keeps
them from unit to unit, and a unit solved alone starts with none.  Most units outlast the forgetting period (50,000
steps; the smallest, with 33,333 nodes, do not), so a policy for retaining nogoods has room to matter, but the suite
does not measure what a searcher gains from the units before.  It does not measure many threads either (a unit
split among 64 threads loses some of its nogood sharing).

## How it was made

1. `tune units GAME CKPT --known results/strict.jsonl --sample 100000 --out units_GAME.jsonl` for each checkpoint:
   every unit, with its depth, its candidates, its bound against what it needs (`gap`), and both sets in hex.
2. `make_suite.py probe` solves the default on units of each game whose gap is from 740 to 1000, and from 930 to 1050 (a
   tree grows by about a factor of ten with every 175 of gap, so these are the units likely to be of 100,000 to 3
   million nodes; a unit that does not finish costs its whole limit, which is why the others are not tried), with node
   limits 120,000, 400,000, 1.2 million and 3 million in turn, until a game has 12 to 14 in range, and the whole games
   named by `--wholes` (`make_suite.py candidates` lists the searched games by the nodes of the chain's search).  Every
   solve is appended to a probes file.  (An earlier try walked the units of a worker's stack from the deepest up, which
   is not it: the subtrees along a stack do not grow steadily.  The gap, which costs one bound a unit, does.)
3. `make_suite.py assemble --count 40` picks 40 of the units that finished with 100,000 to 3 million nodes, the same
   number from each eighth of that range on a log scale where the probes allow, and the same number (10) from each
   game, and the six smallest whole games with 500,000 to 3 million nodes; it writes `suite.jsonl`.
4. `make_suite.py refresh` puts into the suite the CPU seconds of the default on a later build, measured by `tune.py`,
   and refuses a build whose trees are not the suite's.  The first measurement (the probes) was with the build before
   the merge of cheaper GF(2) nodes (6460c86), whose trees are the same; the suite had the later build's.  When the
   default's trees change on purpose (a tactic's default), `refresh --new-trees` takes the new nodes as well, and still
   refuses a best sum that is not the suite's: it did so when the default of `force_window` became `all` (below).

The checkpoints of 3300, 3425 and 3440 are not in the repository (a run on 4 threads is not repeatable): the suite
holds the units taken from them.  To enlarge the suite, make a checkpoint, `tune units` it, and run `probe` and
`assemble` again.

## The driver

```text
cargo build --release --example tune
python3 -I bench/tuning/tune.py --configs CONFIGS.json --results RESULTS.jsonl [--workers P] [--nice N]
python3 -I bench/tuning/tune.py --configs CONFIGS.json --results RESULTS.jsonl --report
```

`CONFIGS.json` is an object of named lists of settings, `NAME=VALUE` as `--tactic` takes them (`configs.example.json`);
the configuration `default` is added, and is the yardstick.  A new knob needs nothing here: `tune solve` hands every
`--tactic` to the solver's parser.  Each job is one configuration on one problem, one `tune solve` process on one
thread, `P` at a time (the number of CPUs by default; on a laptop under other load, `--workers 2 --nice 10`).  Its
result goes to the results file as it ends, and a pair already in the file is not run again, so a run that stops (a
spot machine that goes, say) is started again with the same command.  A job stops at 10 times the default's nodes
(`--cutoff`); the report counts these, and their ratios are lower bounds.

Successive halving: rung 0 runs every configuration on a random third of the suite (seeded: `--seed`) and keeps the
best third by the geometric mean of CPU seconds relative to the default on the same problems; rung 1 runs those on two
thirds, rung 2 on the whole suite (`--rungs 1` stops after rung 0).  The default runs on everything, once.  The report
(printed after each rung, and by `--report`) has a line per configuration: the problems both it and the default ran,
the geometric means of its CPU seconds and of its nodes against the default's (below 1 is better), the worst problem's
ratio and which, the jobs cut off, and its CPU seconds.  It says loudly:

* **a best sum that differs** from the default's on a problem (`BUG?`): a setting that is sound changes the tree, not
  the answer.  Only `symmetry` can differ without a bug, on a unit with no known optimum (3441's): the rule keeps the
  completions that keep the root's symmetric inequalities, and a subtree can hold a heavier one that does not.  Only
  searches that finished are compared;
* **another default than the suite's**: the default's nodes on a problem are not the suite's, so the build is not the
  one the suite was made with, and the search's trees have changed (`make_suite.py refresh` says the same, and
  refuses).

Cost: C configurations take about C/3 + 3 times the suite's CPU at the default (rung 0, then the best third on the
next third, then the best third of those on the last), 1.2 hours of one CPU each here: for 12 configurations about 8
hours of CPU, 8 minutes on 64 CPUs (the campaign's stage B, 11 configurations, took 23 minutes).  Nothing else should
run on the machine meanwhile, and a comparison is between runs made at the same concurrency (a thread's CPU time grows
with the memory traffic of its neighbours; the nodes do not).

## Files

| file | what it is |
|---|---|
| `suite.jsonl` | the suite: a `suite` line (the commit, the ranges, the default's CPU seconds, the concurrency they were measured at), a `game` line for each game (its candidates, the checkpoint's incumbent, the known optimum), the `unit` lines (id `GAME-INDEX`, depth, candidates, the sum of the promotions, bound, need and gap, both sets in hex) and the `whole` lines, each with `default`: the nodes, CPU seconds, wall seconds and best sum of the default's solve |
| `tune.py` | the driver and its report |
| `make_suite.py` | makes the suite (`probe`, `assemble`, `refresh`, `candidates`) |
| `test_tune.py` | tests of both scripts, with a stand-in for `tune solve`: `python3 -I bench/tuning/test_tune.py` |
| `configs.example.json` | a configurations file |
| `campaign-2026-10/` | the data of the first campaign: its configurations, scripts and results, and `tables.py`, which recomputes the tables of [its entry](../../docs/experiments/the-tuning-campaign-at-the-end-of-the-chain-2026-10.md) ([README](campaign-2026-10/README.md)) |
| `../../examples/tune.rs` | `tune units` samples the units of a checkpoint; `tune solve` solves units (or `root`, the whole game) on one thread and prints a line each: nodes, CPU seconds of the thread, whether it finished, the best sum found; its tests are run by `cargo test` |

`tune solve` reads the units from the suite itself, so a job is
`tune solve 3441 --known results/strict.jsonl --units bench/tuning/suite.jsonl --ids 3441-1234 --tactic lagrange=on`
(`--ids root` for the whole game).  The units stay valid as long as the frame builder gives the same candidates (it
checks, and says so if not).

## A smoke run

The driver, run on this laptop on rung 0 (16 of the 46 problems, 8 jobs at a time, `nice -n 10`), with the default
and two configurations (`lagrange=on`, `symmetry=off`), after the default had run on all 46 (for the suite's CPU
seconds); the default was `force_window=80` then (the numbers are of that day):

```text
python3 -I bench/tuning/tune.py --configs smoke.json --results smoke.jsonl --workers 8 --nice 10 --rungs 1
```

```text
configuration                  n cpu ratio nodes ratio   worst  worst problem  cut     cpu s
lagrange_on                   16     0.992       1.000    1.01  3440-89          0    2973.4
default                       16     1.000       1.000    1.00  3441-571         0    2991.4
symmetry_off                  16     1.037       1.129    1.42  3441-5459        0    3053.8
```

With the GF(2) bound `lagrange` changes no tree (the nodes are the same on every problem: the check that the units
are solved the same every time), and its CPU seconds differ from the default's by 0.8%, which is what the noise of
this comparison, at 8 jobs at once, comes to.  `symmetry=off` searches 13% more nodes and takes 3.7% more CPU; its
worst problem (a unit of 3441, which has no known optimum) takes 1.42 times the CPU.  No problem's best sum differed.
