# The hardest searches of the run to 5000 (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-20.  Concluded 2026-10-20.

**Question.**  Does the learning of conflicts closed under the cheap exchanges (`wall images --learn --closure`, *the
learning* below; [its entry](../conflicts-closed-under-cheap-exchanges-2026-10/README.md)) hold on subtrees of the
hardest searches of the committed chain's run to 4547, and does a whole game's learning match its units'?  **Answer.**
On the units, yes, a little under the old suite's size: it takes 20.0% off their steps.  On whole games it went both
ways: 4176 took 24.8% fewer steps and 13.4% less CPU, 4050 took 26.6% more steps and 30.9% more CPU, and 4005, stopped
at about 4,600 CPU s, had spent more than 2.2 times the default search's CPU without finishing.  A game's learning
matched its units' for two games of three.  These whole games settled the learning as [closed, not
adopted](../conflicts-closed-under-cheap-exchanges-2026-10/README.md#later-2026-10-20-closed-not-adopted).  The suite is
the data for [the study of what the learning
leaves](../what-the-learning-leaves-in-the-hardest-searches-2026-10/README.md); this entry makes it and measures the
learning on it, and does not explain.

## Method

* **The ranking** (`ranking.py`).  The run that added games 3915 to 4547 searched 150 games for 29.75 G steps, 62.9 wall
  hours.  The five dearest searches carry 22.4% of the steps, the ten dearest 37.6%, the twenty 57.2% and the forty
  76.5%; `ranking.txt` lists them (3940, 3930, 4225, 3942, 3936 and 4416 lead).
* **Checkpoints** (`checkpoint.py`).  For each of 3940, 4225, 4416 and 4548, as the [tuning
  campaign](../the-tuning-campaign-at-the-end-of-the-chain-2026-10/README.md#the-suite) made its checkpoints: `taxman
  solve N --threads 4 --time-limit 120` with `--out` the committed chain cut to N − 1, and, for the games past 3984
  (whose chain lines cite the bounds store), `--use-prev --force-search`.  The checkpoints of the three whole games,
  4050, 4176 and 4005, were made the same way.  They are not in the repository (a run on 4 threads is not repeatable);
  the units cut from them are in `suite.jsonl`.  Seven checkpoints took 3,692 CPU seconds.
* **Units** (`tune units`, `tune expand`, `make_suite.py probe`, `assemble`).  The checkpoints hold 144 to 410 units.  A
  unit is solved on one thread from the game's known best set, with fresh tables; the default search's steps and CPU
  seconds are its *probe*.  Three things differ from the tuning suite:
  * *The gap band 740-1000 does not fit.*  At 4416 a unit with a gap of 770 to 1,040 took 760 to 62,000 steps, and units
    of 100,000 steps and more began near a gap of 1,150.  The gap predicts the size weakly (a correlation of +0.07 to
    +0.70 by game, 0.05 to 0.86 decades of steps for each 100 of gap, where the tuning suite's band assumed 0.57), so
    the bands were found by probing, game by game.
  * *The checkpoints hold few units of the middle sizes.*  A worker of the 120-second search stood at a shallow node,
    whose loop is huge, or at a deep one, whose loop is small: of the 22 units of 3940's checkpoint that were probed, 4
    took 100,000 to 3,000,000 steps, 7 fewer and 11 did not finish within the limits tried (up to 1.2 million).  `tune
    expand` (new, in the studies build) solves a unit for 2,048 steps and writes the nodes of the searcher's stack as
    units: 47 units of 3940 gave 2,178, 26 of 4225 gave 910 and 53 of 4548 gave 2,262.  An *expanded unit* has an id
    such as 3940-0.4, the fourth node of the stack that unit 0 left; 13 of the 30 chosen are expanded.
  * *A unit of 1.2 million steps or more cost too much to find.*  Each unit that did not finish within 1.2 million steps
    cost 7 CPU minutes, and 9,362 of the 13,684 CPU seconds of the probes went to solves that stopped at their limit.
    So the suite has no unit above 1,048,736 steps, and 3940 and 4225 are thin (6 and 5 units).
* **The suite** (`suite.jsonl`).  `make_suite.py assemble --count 32 --lo 100000 --hi 3000000` over the four hard games
  gave 30 units (3940: 6, 4225: 5, 4416: 9, 4548: 10) of 104,080 to 1,048,736 steps.  For the three whole games, 12
  units of 50,000 steps or more were assembled, 4 a game, of 53,315 to 653,457 steps; the two largest, 4050-78 and
  4005-108, were left out of the runs to keep the CPU under 10 hours, so the suite has 10.  The whole games are 4050,
  4176 and 4005, the three smallest searches of the run with a million steps or more (3.2, 4.4 and 5.8 M steps on 64
  threads).
* **The whole games** were not probed: their default search, counted, is the probe.  `wholes-start.jsonl` gives each a
  nominal 10 M steps, which only sets the limits of the runs (20 M for the default search, 30 M for the learning), with
  the root gap less one from `wall landscape` (1,012, 1,010 and 1,148); the count runs' steps then became the sizes.
* **The runs** (`suite.py run`, the closure entry's, which now takes `--suite` and `--together`): `wall images` from the
  build of `--profile test --features studies`, one thread, fresh tables, from the known best set, two processes at a
  time under `nice -n 10`, each started with the load average at most 8, on the laptop on 2026-10-20.  The hard games'
  units went with the learning only (`--learn --closure`), their default steps being the probes'.  The three whole games
  and their 10 units went both ways, `--count` (the default search with its tree counted) and the learning, side by
  side.  The units of a game with no known best set, 4548's, start from the incumbent that the units file's `game` line
  holds, as `tune solve` does; their lines have `opt` null.
* **The check.**  All 42 problems with a learning run completed with the best set of the default search, and with the
  known best of their game where there is one.  The ten units of 4548 ended with the incumbent they started from,
  454,465, as the default search did: none beat it.

## Results

**The units.**  The learning takes 20.0% off the steps of the 30 units (9.09 M to 7.27 M), and every unit ended with the
best set of the default search.  The old suite's units gained 22.6%.

| game | units | default steps | learning steps | change: total | median | geometric mean | ms a step |
|---|---|---|---|---|---|---|---|
| all | 30 | 9,088,253 | 7,270,313 | −20.0% | −14.7% | −22.2% | 0.325 |
| 3940 | 6 | 1,807,434 | 1,346,483 | −25.5% | −24.0% | −24.1% | 0.283 |
| 4225 | 5 | 808,070 | 568,112 | −29.7% | −14.6% | −34.9% | 0.343 |
| 4416 | 9 | 3,189,355 | 2,309,434 | −27.6% | −24.7% | −26.8% | 0.361 |
| 4548 | 10 | 3,283,394 | 3,046,284 | −7.2% | −2.3% | −9.0% | 0.312 |

The median unit gains 14.7%, and 3 units take more steps (the largest, +5.4% at 4548-1.9); the biggest gains are 77.0%
at 4225-4.24 and 50.2% at 3940-104.  The gain does not rise with the gap (a correlation of −0.12 with the change in
steps) or with the size of the tree (+0.01).  The units of 300,000 to 600,000 steps gain least (3.6%, four of their five
are 4548's), and the four of 600,000 steps and more gain 28.8%; the suite has no unit of 1.2 million steps or more, so
the old suite's trend, in which larger trees gained more, cannot be seen past 1.05 M.  4548 is the one game with no
known best set, and gains 7.2% of the steps against 25.5% to 29.7% at the other three; this entry does not say why.

**The whole games** went both ways, and their units one by one more widely than a game differs from its units together:
the 10 units vary from −55% to +35% of the steps.

| problem | default steps | learning steps | change | default CPU s | learning CPU s | change |
|---|---|---|---|---|---|---|
| 4176 whole | 5,962,919 | 4,483,040 | −24.8% | 1840.1 | 1592.8 | −13.4% |
| 4050 whole | 3,086,223 | 3,906,598 | +26.6% | 1083.2 | 1417.8 | +30.9% |
| 4005 whole | 6,186,795 | stopped | | 2,084 | about 4,600, stopped | 2.2 times |
| 4176 units (4) | 669,274 | | −26.0% | | | −17.1% |
| 4050 units (3) | 215,142 | | +14.5% | | | +15.1% |
| 4005 units (3) | 356,608 | | −8.8% | | | −4.2% |

The learning on 4005 was stopped by hand to keep the CPU spent under 10 hours, and has no line (`runs-stopped.jsonl`).
A whole game's learning matched its units' at 4176 and 4050 and went the other way at 4005.  A step with the learning
costs 3.4% more than the default's at 4050 (0.363 ms against 0.351) and 14.9% more at 4176 (0.355 against 0.309).  The
old suite's whole games gained 6.8% of the steps and 4.5% of the CPU.  The two finished whole games together take 7.3%
fewer steps and 3.0% more CPU (2,923 to 3,011 s); their ten units together take 14.0% fewer steps and 7.0% less CPU,
with four of the ten taking less CPU.  The dearest problems with the learning on are the whole games, 4005 (over 4,600
s), 4176 (1,593 s) and 4050 (1,418 s), then 4416-83 (297 s), 4416-148 (216 s) and 3940-71 (198 s).

**The gap against a unit's size,** over the probes that finished (a unit that stopped at its limit is not in it, so the
line is low for the large gaps): the line rises by 0.05 to 0.86 decades of steps for each 100 of gap, and crosses
100,000 steps at a gap of 780 (4050) to 1,459 (4416), and 3,000,000 steps at 952 (4050) to 4,339 (4548).

The whole study took 35,704 CPU seconds, 9.92 hours.

## Decision

The learning is closed, not adopted: units gain from it, and whole games, where the chain's cost lies, do not gain
reliably.  It stays in the studies build as a record, and the searcher does not get it.  What would reopen the question
is a whole-game result that gains reliably, which the three here do not give.

## Files

* `ranking.py`: the ranking (`ranking.py top`, `ranking.py smallest --count 8`); `ranking.txt` is its output.
* `checkpoint.py`: makes a game's checkpoint (`checkpoint.py 3940 --dir DIR --log checkpoints.jsonl`);
  `checkpoints.jsonl` has a line for each of the seven, with its CPU seconds.
* `suite.jsonl`: the problems, each unit with its hex sets, its `gap` and the default search's steps, CPU seconds and
  best set (`default`, the probe's).  It is the three `make_suite.py assemble` outputs concatenated:
  `suite-hard.jsonl` (the four hard games' 30 units), `suite-wholes-units.jsonl` (the whole games' 12 units) and
  `suite-wholes.jsonl` (the three whole games):

  ```text
  make_suite.py assemble --units UNITS_3940 UNITS_4225 UNITS_4416 UNITS_4548 --probes probes.jsonl --count 32 --lo 100000 --hi 3000000 --out suite-hard.jsonl
  make_suite.py assemble --units UNITS_4050 UNITS_4176 UNITS_4005 --probes probes.jsonl --count 12 --lo 50000 --hi 3000000 --out suite-wholes-units.jsonl
  make_suite.py assemble --units UNITS_4050 --probes probes.jsonl --runs runs.jsonl --count 0 --wholes 4050,4176,4005 --whole-lo 0 --whole-hi 20000000 --whole-gaps 4050=1012,4176=1010,4005=1148 --out suite-wholes.jsonl
  ```

  `UNITS_N` is the units file that `tune units N CKPT --known results/strict.jsonl --sample 100000 --out UNITS_N`
  wrote (all of the checkpoint's units), with the lines of `tune expand N --units UNITS_N --ids ... --known
  results/strict.jsonl --out EXPANDED` appended for 3940 (the units of gap 850 and up), 4225 (600 and up) and 4548 (850
  and up).  The units files are not kept.
* `probes.jsonl`: every solve of `make_suite.py probe`, one line each, of the default search with a node limit;
  `probed-units.jsonl` has the gap and depth of the units the probes tried.  The passes were
  `make_suite.py probe --units UNITS_N --probes probes.jsonl --bin tune --nice 10 --max-load 8 --workers 1 ...` with:
  3940: `--gap-lo 750 --gap-hi 1000 --ladder 120000,400000,1200000 --per-game 4 --max-failures 1`, the same on 800-1,350
  (8, 2), three passes by `--ids` over the stacks of units 72-84 and 101-105, 155-157 and 162, and on the expanded units
  820-1,150 (12, 2, `--seed 2`) and 790-1,000 (ladder 120000,400000; 12, 5; `--seed 5`); 4416: 750-1,050 (4, 1),
  1,150-1,450 (ladder up to 3000000; 8, 2) and 1,450-1,700 (ladder 400000,1200000 from 400000; 12, 2); 4225: 600-1,000
  (8, 2) and, on the expanded units, 760-1,100 (8, 3; stopped by hand); 4548: 800-1,300 (10, 2) and 850-1,250 (10, 2,
  `--seed 3`); the whole games' units: 550-900, `--lo 50000`, `--per-game 6`, `--max-failures 2`.  The tuning campaign's
  tool gained `--ladder`, `--ids`, `--max-load`, `--whole-limits`, `--whole-gaps`, `--runs` and, for `candidates`,
  `--from` and `--to`, with defaults that keep its behaviour; `test_tune.py` tests them.
* `runs.jsonl`: the lines of `wall images`, one a run, with the problem's `gap`, the one-minute `load` at its start and
  its `wall_s`: 30 learning runs of the hard games' units, and the default search and the learning of the three whole
  games and their units.  `suite.py run runs.jsonl --suite suite-hard.jsonl --modes closure --only IDS` made the first;
  `suite.py run runs.jsonl --suite wholes-start.jsonl --modes count,closure --together --workers 1 --only
  4050-root,4176-root,4005-root` and the same with `--suite suite-wholes-units.jsonl` the others.  `runs-stopped.jsonl`
  records the run stopped by hand.
* `wholes_start.py`, `wholes-start.jsonl`: the whole games before their runs, with nominal sizes; `landscape.jsonl` is
  `wall landscape --known results/strict.jsonl 4050 4176 4005`.
* `tables.py`: the tables of the runs (`python3 -I tables.py all`); `tables.txt` is its output, with the per-unit rows
  and the dearest problems that this record leaves out.  `ledger.py` runs a command and adds its CPU seconds to
  `cpu.jsonl`, for what no other file records: builds, tests, `tune units`, `tune expand`, `wall landscape`.

The code: `examples/tune.rs` gained `expand` (and `start_set`, `unit_line`, factored from `solve` and `units`), with the
test `a_stopped_unit_leaves_its_stack_as_units`; `examples/wall/images.rs` and `examples/wall/anatomy.rs` the start from
a units file's incumbent (the test `a_game_with_no_known_optimum_starts_from_its_units_incumbent`).  Both are
studies-build examples; the default search is untouched.

## Commits

* `3609574` (2026-10-20): the runs, the tables and the entry.
* `0e57e1e` (2026-10-20): the entry's register row and index line; the quick check's CPU in the ledger.
* `0343c5a` (2026-10-20): the verdict, closed, not adopted.
