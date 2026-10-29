# A gate for each game (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-28.  Concluded 2026-10-29.

**Question.**  Can a gate of conflict branching that changes while the chain searches, with the phase of a game, with
the gap where a unit's subtree began, relative to that gap, or learned within the game, bring the chain's total CPU to
0.95 or less of the fixed gate of 1,200, weighted by what each game costs?

**Answer.**  The phase cannot: the spread of a game's open gaps stays the same until its last few percent, so a gate
keyed to it acts on 0 to 2% of a game's steps.  The game is what decides.  On 60 units of each game, a gate of 600 took
0.68 of the CPU of 1,200 on 5052 and 0.83 on 5054, against 1.10 to 1.55 on 4984, 4992 and 4995, and units solved to
500,000 and 1 million steps keep each game's sign (0.57 to 0.86 of the steps on 5052 and 5054, 2.05 on 4984).  So a
search that tries the gates on 24 of its own units and keeps the cheapest would take a projected 0.86 to 0.89 of the
chain's CPU, where no fixed gate does better than 0.97.  It is built as an option, `--gate-trial`, off by default.

## Method

*The profiles.*  The chain's profiles of games 4944 to 5054 (`chain-data/profile/`): each `open` record holds the
spread of the open units' gaps (quantiles 0, 10, 25, 50, 75, 90 and 100% of a uniform sample of at most 1,000 units)
with the search's steps then.  `crossings.py` finds where each quantile first falls under a level, and the share of the
game's steps after that point.

*The panels.*  Sixty units of each game, picked at evenly spaced ranks of the gap among a checkpoint's units of gap
300 or more, each solved on one thread by `tune solve` under the chain's settings (conflict branching on,
`price_reuse=on`, `warm_start=on`, the look on, the game's carried store), stopped at 150,000 steps:

* 5052, 4995, 4992 and 4984: the gate sweep of 2026-10-28 (`lab/work/gate-2026-10-28/`), gates 400 to 1,200, from
  checkpoints at 89%, 6%, 93% and 55% of their games' steps;
* 5054, the game the chain was searching: a copy of its checkpoint at 1.50 billion steps (20:43 PDT), with its store
  `nogoods-5052.txt` (the newest below 5054, 20,000 sets), gates 600, 800 and 1,200, six solves at once at `nice -n 19`.

*The large units.*  The units of 5052, 5054 and 4984 that the gate of 1,200 left unfinished at 150,000 steps (four,
eight and three: `ids-*-big.txt`), solved to 1 million steps at 600 and 1,200, and 30 more units of 5054, of gap 1,106
to 1,828 at evenly spaced ranks among the 345 of its 2,000 sampled units of gap 1,100 or more (`ids-5054-high.txt`),
to 500,000.

*A test's noise.*  From each panel, K units drawn at random, the gate of least CPU over them, scored by that gate's
ratio over all 60 (`testnoise.py`, 2,000 draws).

*The projection.*  Each game's ratio under a policy times its steps in the chain (the profiles), against g1200's
(`project.py`).  The trial's ratio is the mean ratio of the gate a 24-unit test picks among 600, 800, 1,000 and 1,200,
applied to the game's steps after its first 50 million, plus the test's own CPU.  5054 is taken at 4.09 billion steps,
5052's, and the twins 4983 and 5049 (games whose first open units match 4984's and 5052's) at their twin's ratios.

## Results

**1. The spread of the open gaps stays the same until a game's last few percent.**  The share of each costly game's
steps taken after the open units' gaps first fall under a level:

| game | steps (billions) | largest under 1,400 | largest under 1,200 | 90% under 1,200 | 90% under 1,000 |
|---|---|---|---|---|---|
| 4983 | 8.77 | 0.01 | 0 | 0.79 | 0.13 |
| 4984 | 3.87 | 0 | 0 | 0.24 | 0 |
| 4992 | 1.95 | 0 | 0 | 0.61 | 0 |
| 4995 | 0.98 | 0 | 0 | 0.08 | 0 |
| 5049 | 4.05 | 0.05 | 0 | 0.96 | 0.23 |
| 5052 | 4.09 | 0.02 | 0 | 0.96 | 0.18 |
| 5054 (to 1.56) | | 0 | 0 | 0.89 | 0 |

Through 5052's search the spread was [0, 123, 301, 593, 908, 1158, 1745] at 1.0 billion steps and [0, 141, 351, 640,
916, 1113, 1630] at 3.4; 4984's was [2, 180, 393, 662, 1000, 1278, 1942] at 1.6.  So the 90% quantile sits near its
level from a game's first minutes (it crosses 1,200 at 4% of 5052's steps and hovers there), and the largest gaps stay
over 1,400 until the last 2 to 5%.  Most units at any time are small, and the few of large gap hold the work.

No game's open gaps were all under 1,200 before its end, so a gate lowered then (policy 1) saves nothing.  Keyed to the
90% quantile under 1,000, it would act on the last 13 to 23% of 4983, 5049 and 5052 only: 6 to 7% saved on 5049 and
5052 at their ratio, 7% lost on 4983 at 4984's, a total near 1.00.  A gate set by the spread (policy 4) is a fixed gate
a game, and the spread does not tell the games apart.

**2. The game decides.**  CPU against g1200 on each game's 60 units (in brackets, the least and the most with one unit
left out):

| game | checkpoint | g1000 | g800 | g600 | g400 |
|---|---|---|---|---|---|
| 5052 | 89% | 0.976 | 0.784 | 0.677 (0.64-0.73) | 0.706 |
| 5054 | 1.5 billion | | 0.892 | 0.826 (0.81-0.86) | |
| 4992 | 93% | 0.929 | 0.946 | 1.100 (1.06-1.19) | 1.476 |
| 4995 | 6% | 0.978 | 1.084 | 1.317 (1.23-1.39) | 1.674 |
| 4984 | 55% | 1.107 | 1.224 | 1.551 (1.44-1.75) | 2.166 |

The phase of the checkpoint does not order them (5052 late and 4992 later; 4984 halfway), and within a game the bands
of gap mostly agree: 4984 loses at 600 in every band from 600 up, 5054 gains in every band from 600 up (0.83, 0.77,
0.87 for 600-899, 900-1,199, 1,200 and up).  Single units swing from 0.07 to 18 times, in every band and every game.
Even the direction of the gap's effect is the game's: a unit's log ratio of steps (600 against 1,200, units of 2,000
steps or more) ranks with its gap at +0.48 to +0.55 (Spearman) on 4984, 4992 and 4995, at -0.42 on 5052 and +0.10 on
5054, and its depth and number of candidates change sign as well (`rankcorr.py`).  Whole games agree that the
better gate belongs to a stretch of games: of gates 150 and 300 near n = 3200 and 3500, the same won on every one of 11
consecutive games ([the register, 2026-10-22](../../register/tactics-and-shortcuts.md)).

**3. A gate for the lineage or relative to it has no unit to anchor to.**  A worker that yields pushes its stack's open
loops as units and resumes its deepest one ([engine.md](../../../docs/design/engine.md#engine)), at every checkpoint, so
a unit of the chain is a node deep in the subtree a worker had, and its own gap falls with every yield.  Carried through
the yields, a unit's gap at the start of its lineage is the root's.  Result 6 of [the choice at lower
gates](../the-choice-of-the-conflict-at-lower-gates-2026-10/README.md) (g600 inside the units of gap under 1,200) is a
rule for a checkpoint's units, which no rule of a node reproduces; policies 2 and 3 were not built.

**4. A test of 24 units picks a game's gate.**  Drawing K of a game's 60 units and taking the gate of least CPU:

| game | best of 60 | K = 10 | K = 20 | K = 30 |
|---|---|---|---|---|
| 5052 | 0.677 | 0.708 | 0.689 | 0.684 |
| 5054 | 0.826 | | 0.834 | 0.828 |
| 4992 | 0.929 | 0.960 | 0.944 | 0.939 |
| 4995 | 0.978 | 1.022 | 1.001 | 0.989 |
| 4984 | 1.000 | 1.047 | 1.022 | 1.011 |

By steps instead of CPU the test picks worse (5052: g400, 0.706), since a lower gate costs more a step.  The trial's
own pick, 24 units at evenly spaced ranks of the gap with the margin of 3% (`stratified.py`, ten offsets within a
slice), is steadier: 600 every time on 5052 and 5054, 1,200 every time on 4984, 1,000 eight times of ten on 4992 (0.943)
and four of ten on 4995 (0.991).  Of 24 units under three or four gates, the test costs 0.14 to 0.35 thread-hours,
against 120 to 1,100 for the games themselves.

**5. The large units keep their game's sign.**  The work of a costly game lies in subtrees far above 150,000 steps
(5052's checkpoint held 0.44 billion steps in 3,485 units), so the panels see the large units only as far as the limit.
Solved further, the steps at 600 against 1,200 over the units that at least one gate finished (a unit stopped at the
limit counted at it, which favors the gate that stopped):

| units | finished by a gate | smaller at 600 | larger | by neither | steps at 600 against 1,200 |
|---|---|---|---|---|---|
| 5052, to 1 million | 3 | 2 | 1 | 1 | 0.86 |
| 5054, to 1 million | 5 | 4 | 1 | 3 | 0.57 |
| 5054, high gap, to 500,000 | 21 | 17 | 4 | 9 | 0.67 |
| 4984, to 1 million | 3 | 0 | 3 | 0 | 2.05 |

Single units still swing both ways (5054-6505 266,000 to 437,000 steps; 5054-74 736,000 to 226,000), but each game's
large units go its panel's way, more strongly if anything (`large.py`).

**6. The projection.**  Each game's cost-weighted share and ratio against g1200:

| game | steps (billions) | weight | g1000 | g800 | g600 | the trial |
|---|---|---|---|---|---|---|
| 5052 | 4.09 | 0.147 | 0.976 | 0.784 | 0.677 | 0.683 |
| 5049 (as 5052) | 4.05 | 0.146 | 0.976 | 0.784 | 0.677 | 0.683 |
| 5054 | 4.09 (taken) | 0.147 | | 0.892 | 0.826 | 0.834 |
| 4992 | 1.95 | 0.070 | 0.929 | 0.946 | 1.100 | 0.945 |
| 4995 | 0.98 | 0.035 | 0.978 | 1.084 | 1.317 | 0.996 |
| 4984 | 3.87 | 0.139 | 1.107 | 1.224 | 1.551 | 1.017 |
| 4983 (as 4984) | 8.77 | 0.315 | 1.107 | 1.224 | 1.551 | 1.017 |
| all seven | 27.80 | 1 | 1.036 | 1.022 | 1.148 | 0.886 |
| the five measured | 14.98 | | 1.011 | 0.968 | 1.040 | 0.865 |

The trial's column takes random draws of 24 units; with its own stratified pick (result 4) the totals are 0.877 and
0.858.  No fixed gate clears 0.95; the trial does by a margin, so long as a game's 60 units say what its whole search
does.  That assumption is the weak one, since a game's largest subtrees are what it costs; result 5 supports it for
5052, 5054 and 4984 up to a million steps, and nothing here measures a whole game.

**7. The gate trial.**  `--gate-trial GATES` ([design/gate-trial.md](../../../docs/design/gate-trial.md)): once a
game's search has taken 50 million steps, a thread beside it takes the next checkpoint's units, picks 24 of gap 300 or
more at evenly spaced ranks of the gap, solves each under each gate and the search's own on one thread to 150,000
steps, four at once, and sets the gate of least CPU if it took 3% less than the search's.  A game whose lab record has
a gate, or whose branching settings the lab changes after its search started, keeps the lab's.  On game 1020 (a debug
build, two threads, checkpoints every second, `--gate-trial 0,4000 --gate-trial-plan
after=0,units=3,steps=500,least_gap=0,threads=2`), the trial took its units at 594 steps, solved three under 300, 0 and
4,000 (2.1, 4.5 and 2.2 s of CPU), kept 300, and the line recorded it in `engine.gate_trial`; the score was optimal.

**8. The live game.**  On these numbers the lab set game 5054's gate to 600 at 21:16:36 PDT (04:16:36Z), at about 1.86
billion steps and 8 h 39 min of its search.  Its profile from then on is a live data point with no control.

## Decision

Built, off by default: the gate trial, `--gate-trial GATES` with `--gate-trial-plan`.  To run it on the chain:
`--gate-trial 1000,800,600` (the policy's 1,200 is tried too).  It changes only the gate, which changes no answer.  The
fixed gates and the policies of the phase, the lineage and the relative gate are not kept.

What would reopen it, the likeliest first:

1. **The trial's own record on the chain.**  Each game's line keeps what its trial measured and chose; a game whose
   trial chose 600 or 800 and whose search then took longer than its twin or its neighbors says the 150,000-step units
   misled it.  `--gate-trial-plan steps=500000,least_gap=900` sees larger subtrees, at about three times the cost.
2. **5054's search after the switch**, against 5049's and 5052's at the same steps.
3. **A trial again later in a game**, if the best gate turns out to move within one; nothing here says it does.

## Files

The runs are in `~/taxman_work/lab/work/dyngate-2026-10-28/` on the machine of the chain (`5054/`, `5052/` and `4984/`:
units, store, `out/` a result and a stderr file a solve; `exercise/` the run of game 1020).  The gate sweep's runs are
in `~/taxman_work/lab/work/gate-2026-10-28/`.  Copied here:

* `crossings.py PROFILES`: result 1, from a file listing the profiles (`profiles.txt`).
* `features.py`, `bysize.py`, `rankcorr.py`: the units' features and sizes against their ratios (result 2).
* `testnoise.py [GAMEDIR ...]`, `stratified.py`: result 4.
* `large.py`: result 5, from `ids-5052-big.txt`, `ids-5054-big.txt`, `ids-4984-big.txt` and `ids-5054-high.txt`.
* `project.py`: result 6.
* `pick.py GAMEDIR [N]`, `run.py GAMEDIR SETFILE IDFILE NODELIMIT [P]`, `panel.py GAMEDIR BASE SET...`: picking the
  units, the runner (settings in `set-gates.txt` and `set-big.txt`), a setting against a base by band.
* `runs.txt` (made by `runs.py`): every solve here, of 5054, 5052 and 4984, a line each: its folder, setting (`g`: to
  150,000 steps; `G`: further), unit, gap, steps, CPU seconds, whether it finished, and its best (each game's the same
  in every solve).

## Commits

One commit: the gate trial (`src/search/gate_trial.rs`, the flags `--gate-trial` and `--gate-trial-plan`, the line's
`engine.gate_trial` and `options.gate_trial`, [design/gate-trial.md](../../../docs/design/gate-trial.md)), this entry
and its register row.
