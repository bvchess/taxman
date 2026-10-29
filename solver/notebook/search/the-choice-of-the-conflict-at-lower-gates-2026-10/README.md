# The choice of the conflict at lower gates (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-28.  Concluded 2026-10-28.

**Question.**  Can a cheaper choice of the conflict let conflict branching act further down the tree, below the chain's
gate of 1,200, and take at least 7% less CPU than the chain's setting on a panel of hard games (the geometric mean of
the games' total-CPU ratios), with no game worse than 1.05?

**Answer.**  No.  At gates of 300 and 600 the lower gate adds steps on games 4984 and 4995 (1.41 and 1.19 of the
steps at 600), and only game 5052 gains (0.71), so even a choice that cost nothing would leave the panel at about 1.0.
At the gate of 1,200 the choice is 0.4% of a step, too little to save.  Cheaper choices that change the prices cost
steps, and none of eleven rules for the nodes between 600 and 1,200 kept 4984 from losing.

## Method

*The settings.*  Every run has the chain's tactics: conflict branching on, `price_reuse=on`, `warm_start=on`, the look on,
and the carried store of each game.  The reference, *g1200*, is the gate of 1,200; *g600* and *g300* are the same
search with the gate at 600 and 300.  The *mid band* is the nodes of a gap from the gate up to 1,200.

*The panel.*  Sixty units of each of three games, each solved to the end on one thread by `tune solve`, stopped at
150,000 steps, eight solves at once at `nice -n 19`:

* 4984: 60 units of gap 300 or more picked at random from `lab/work/patch/units-4984.jsonl` (`sample-4984.txt`), with
  the store of 144 units of 4983;
* 4995: the 60 units of `lab/work/patch4995/sample.txt`, with the chain's store of game 4992;
* 5052: the 60 units of the gate sweep of 2026-10-28 (`lab/work/gate-2026-10-28/5052/`), from a checkpoint at 3.6
  billion steps, with the store of game 5049.

The units of 4984 and 4995 come from checkpoints early in their games, and their gaps reach 1,700 and 1,900; those of
5052 come late in its game, and only two have a gap over 1,200.

*The counters.*  A build of `tune` with timers in the choice (`study.patch`, against 6959a61): the time stamp counter
read around each part of `conflict_to_branch_on`, inside `prices_excluding`, and around each step, written to stderr at
the end of a solve.  The variants are knobs of the same build, read from the environment.  Steps are exact.  CPU comes
from runs not paired on a core's two threads, so it drifts by 5 to 10% on a total; the steps decide.

## Results

**1. At the chain's gate the choice costs nothing; at 300 it is two fifths of a step.**  The choice's share of the
steps' cycles, and its parts:

| game | gate | steps | CPU s | choices a step | choice, % of a step | exact prices | ordered conflicts | split estimates | conflict members | look | table conflicts | rest | µs a choice |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 4984 | 1,200 | 1,120,965 | 470 | 0.001 | 0.5 | 0.2 | 0.0 | 0.0 | 0.0 | 0.1 | 0.0 | 0.0 | 2,350 |
| 4984 | 600 | 1,583,334 | 691 | 0.027 | 11.4 | 5.7 | 1.3 | 1.5 | 0.8 | 1.6 | 0.2 | 0.2 | 1,821 |
| 4984 | 300 | 1,644,319 | 1,206 | 0.198 | 39.5 | 18.6 | 5.3 | 4.8 | 3.6 | 4.8 | 1.6 | 0.8 | 1,460 |
| 4995 | 1,200 | 1,518,418 | 630 | 0.001 | 0.4 | 0.2 | 0.0 | 0.0 | 0.0 | 0.1 | 0.0 | 0.0 | 2,642 |
| 4995 | 600 | 1,812,699 | 835 | 0.030 | 12.2 | 5.8 | 1.4 | 1.7 | 0.9 | 1.8 | 0.3 | 0.3 | 1,894 |
| 4995 | 300 | 1,792,957 | 1,320 | 0.191 | 39.1 | 17.7 | 5.4 | 5.3 | 3.5 | 4.5 | 2.0 | 0.8 | 1,509 |
| 5052 | 1,200 | 1,085,320 | 504 | 0.000 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 | 1,522 |
| 5052 | 600 | 773,945 | 343 | 0.006 | 2.7 | 1.2 | 0.3 | 0.3 | 0.2 | 0.5 | 0.0 | 0.1 | 1,875 |
| 5052 | 300 | 594,323 | 415 | 0.120 | 28.6 | 14.3 | 3.9 | 2.9 | 2.3 | 3.5 | 0.9 | 0.7 | 1,652 |

* *Exact prices* are half of the choice at every gate: a pricing searches the exchange graph for 40 members on
  average, at 36 to 46 µs a search, and 40 to 45% of the choices price at all (price reuse serves the rest).
* *The rest of it* is spread over five parts: the ordered conflicts (with their check by the whole peel, 0.4% of a
  step at 300), the weight split's estimate of every member, `conflict_members`, the look, and the table's conflicts.
* *The step outside the choice* is the bound (52 to 56% of a step at 1,200) and strong branching (12%).  Since Q4
  ([the root gap](../the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-23-a-cheaper-choice-of-the-conflict-below-the-gate-q4))
  a choice has fallen from 3.2 ms to 1.5 to 1.9 ms below the chain's gate.

**2. The lower gate loses steps on two games of three.**  Steps against g1200 by the units' gap (units in brackets):

| game | gate | gap 300-599 | 600-899 | 900-1,199 | 1,200 and up | all: steps | all: CPU |
|---|---|---|---|---|---|---|---|
| 4984 | 600 | 1.000 (17) | 2.061 (19) | 1.034 (7) | 1.407 (17) | 1.412 | 1.471 |
| 4984 | 300 | 1.266 | 2.284 | 1.185 | 1.456 | 1.467 | 2.568 |
| 4995 | 600 | 1.000 (15) | 0.938 (16) | 0.824 (12) | 1.232 (17) | 1.194 | 1.324 |
| 4995 | 300 | 0.681 | 0.802 | 0.845 | 1.217 | 1.181 | 2.093 |
| 5052 | 600 | 1.000 (25) | 0.829 (21) | 0.696 (12) | 0.677 (2) | 0.713 | 0.681 |
| 5052 | 300 | 0.421 | 0.405 | 0.539 | 0.647 | 0.548 | 0.823 |

The units of 4984 and 4995 of gap 1,200 or more take 1.2 to 1.5 times the steps at the lower gates: at 600, 13 of 17
of 4984 are worse by more than 3% and 1 better, 9 of 17 of 4995 worse and 4 better, single units up to 5.4 times;
those units are most of their games' CPU.  Below a gap of 1,200 the
lower gate gains on 4995 and 5052 and loses on 4984.  The gate sweep of the same day ([its
analysis](#files), four games and gates 400 to 1,200 with the chain's choice) agrees: steps at 400 of 0.62 (5052), 1.22
(4995), 1.14 (4992) and 1.74 (4984).

**3. A free choice would not clear the bar.**  Removing the choice's cycles from the lower gates' CPU leaves 4984 at
1.30 and 1.55 of g1200 (gates 600 and 300), 4995 at 1.16 and 1.28, and 5052 at 0.66 and 0.59: a panel of 1.00 and
1.05.  At the gates of 800 and 1,000, whose panels are 1.01 and 1.00 here (steps of 1.25 and 1.09 on 4984, 1.08 and
1.00 on 4995, 0.82 and 0.99 on 5052), the choice is 5.0 and 1.7% of a step on 4984, 5.2 and 1.8% on 4995 and 0.4 and
0.1% on 5052, so a free one would take the panel to 0.98 and 0.99.

**4. Cheaper prices cost more steps than they save.**  Two choices that price only the members of the candidate
conflicts, with the ordered conflicts made in the order of the prices known (the price memo's, else the weight split's
estimate):

* *all at once* (knob 1): the members of `K`, the table's conflicts and the ordered conflicts priced in one pricing;
* *one at a time* (knob 2): the candidates by their weight from the prices known, each member priced with its search
  stopped at the price that would take its candidate past the best weight, and a candidate dropped once its known
  terms reach that weight.

On 5052, against the same gate with the full choice: at 300, 1.10 and 1.09 of the steps and 1.13 and 1.12 of the CPU;
at 600, 0.96 and 1.02 of the steps and 1.02 and 1.08 of the CPU.  A choice cost 1,374 and 1,204 µs against 1,652, but
the trees made 0.159 and 0.179 choices a step against 0.120.  The exact order of the prices matters to the trees, and
the parts other than the searches stay.

**5. No rule for the mid band kept 4984 from losing.**  Each at the gate of 600, acting only in the mid band, so that
the nodes of a gap of 1,200 or more choose as the chain does:

| rule in the mid band | units of 4984, 4995, 5052 | steps against g600 | steps against g1200 |
|---|---|---|---|
| the chosen conflict not learned as a nogood | 22, 0, 5 | 0.85, -, 1.13 | 1.19, -, 0.72 |
| a term of 0.3 added to the weight for each member | 22, 0, 6 | 0.97, -, 0.99 | 1.36, -, 0.64 |
| a term of 1.0 a member | 22, 0, 5 | 0.88, -, 1.44 | 1.24, -, 0.91 |
| another conflict only if it weighs under half of `K` | 22, 0, 5 | 0.86, -, 1.09 | 1.21, -, 0.69 |
| `K` priced first, and kept if it weighs under 1.5 | 22, 0, 5 | 0.99, -, 1.01 | 1.39, -, 0.64 |
| the same under 3 | 22, 0, 5 | 0.96, -, 1.09 | 1.35, -, 0.69 |
| a slope of 150 in place of 291 | 14, 14, 1 | 1.12, 1.16, - | 1.50, 1.36, - |
| a slope of 600 | 14, 14, 1 | 0.97, 0.87, - | 1.30, 1.01, - |
| no choice where the parent's check chose another conflict | 14, 16, 1 | 0.87, 0.81, - | 1.17, 0.99, - |
| no choice where the parent's check made a choice | 14, 15, 1 | 0.89, 0.79, - | 1.20, 1.00, - |
| no choice at a gap of 1,200 or more either | 14, 16, 1 | 0.88, 0.86, - | 1.18, 1.06, - |

The rows with few units are the largest units of each game, run first and stopped once the rule had lost on 4984.
Over the whole panel, at the gate of 600, price reuse off took 1.01 (4984) and 0.99 (5052) of g600's steps, and the
look off 1.33 and 1.02.  Not learning the chosen conflict anywhere, at the gates of 1,200, 900 and 600, took 0.98, 1.01
and 1.09 of g1200's steps on 4984, 1.06, 1.09 and 1.06 on 4995, and 1.00, 0.92 and 0.82 on 5052: a panel of 1.03, 1.03
and 1.14 in CPU.  So learning the chosen conflicts is part of what the lower gate costs 4984 and 4995, and part of what
it gains on 5052.

**6. What a gap tied to the unit would give.**  Taking g600's result for the units of gap under 1,200 and g1200's for
the others, as a gate set by each unit's own gap would, gives 1.02 (4984), 0.99 (4995) and 0.78 (5052) of g1200's CPU,
a panel of 0.92; on the four games of the gate sweep, 1.07 (4984), 0.99 (4992), 0.99 (4995) and 0.77 (5052), a panel of
0.95, and 0.96 with 800 for the low units.  This is the units' arithmetic, not a rule the solver has: the chain splits
its work into units at every yield, so a unit's root says little about where it lies in the game's tree.

**7. An exact saving.**  Making the split's estimate only for the members whose price a weight or the order reads
(those of `conflict_members` that the pricing did not certify, and the others of the table's conflicts) leaves every
choice as it was: the same steps on all 27 units run (22 of 4984, 5 of 5052, gate 600).  On the units of 4984 the
split estimates fall from 245 to 16 µs a choice, and a choice from 1,821 to 1,566 µs, 1.4% of a step at 600; at 300
the split estimates are 3 to 5% of a step.

## Decision

Not kept: no choice or rule clears the bar, and no code changes.  The chain stays at the gate of 1,200 with the choice
as it is.  The profile, the knobs and the counters stay in `study.patch`.

What would reopen it, the likeliest first:

1. **A gate for the late part of a game.**  Games like 5052 gain 20 to 30% in their late units at 600 (result 6).  A lab
   policy that lowers the gate once the open units' gaps are all under 1,200 is the form to test, on whole games, since
   a checkpoint's units leave out the work done before it.  The profile's summary of the open work already bounds each
   open unit.
2. **Why the lower gate hurts inside units of a high gap.**  The same nodes gain in units of a lower gap (4995's bands
   under 1,200), and not learning the chosen conflicts cuts 4984's loss of steps at 600 from 41% to 9% while 5052's
   gain falls from 29% to 18%.  A trace of the choices in a few units where the two disagree would say which nogoods
   do it.
3. **The exact savings of the choice**, only if the gate falls to 600 or below for long: the lazy split estimates (3 to
   5% of a step at 300), the frames' cores of `conflict_members` cached as `ConflictCache` caches conflicts (2 to 4%),
   and the pricing's searches by A* with the cycle kept rather than Dijkstra's (the pricing is 14 to 19% of a step at
   300).
4. Inexact pricing is a dead end: the order of the exact prices is what the trees need.

## Files

The runs are in `~/taxman_work/lab/work/choice-2026-10-28/` on the machine of the chain (`out-*/`, a result file and a
stderr file with the counters a solve; `bin/` the builds; `ref/` the profile's runs, the references).  The gate sweep
of 2026-10-28 is in `~/taxman_work/lab/work/gate-2026-10-28/` (`analysis.txt`).  Copied here:

* `study.patch`: the counters and the knobs, against 6959a61.  `TAXMAN_CH=1|2` (the cheaper prices of result 4),
  `TAXMAN_CH_STOP`, `TAXMAN_CH_CAP` and `TAXMAN_CH_ITER` (their limits and rounds), `TAXMAN_CH_LAZY` (result 7),
  `TAXMAN_CH_MID` (the top of the mid band), and in it `TAXMAN_CH_NOLEARN`, `TAXMAN_CH_C`, `TAXMAN_CH_RHO`,
  `TAXMAN_CH_FLOOR`, `TAXMAN_CH_SLOPE`, `TAXMAN_CH_NOLOOK` and `TAXMAN_CH_NOSTACK`; `TAXMAN_CH_TOP` (no choice from
  that gap up).  `tune` writes the counters as `PROF` lines.
* `settings.txt`: every setting run, a line each (`NAME BINARY ENV ARGS`).
* `run.py MODE SETFILE IDS OUT NODELIMIT`: the runner (`solo`: one solve a CPU; `pair`: two settings on a core's two
  threads, not used here).
* `summ.py` (the counters by setting and game), `proftable.py` (the table of result 1), `bands.py` (result 2),
  `panel.py` (a setting against a reference by game, and the panel), `units.py` (steps a unit), `toprule.py` and
  `toprule_sweep.py` (result 6), `runs.py` (makes `runs.txt`).
* `sample-4984.txt`: the units of 4984; `order-all.txt`: the 180 units with their gaps, largest first.
* `runs.txt`: every solve, a line each: its folder, setting, unit, gap, steps, CPU seconds, whether it finished, and
  its best.

## Commits

This entry and its register row.  No code changed.
