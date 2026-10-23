# The root gap and the size of a game (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-23.

**Question.**  What decides how many steps a game of the chain takes?  The first findings are that the root's gap does
not, that a game's size is decided below the root, and that hard games differ from easy ones in how far the bound falls
at side branches in the top of the tree.  The entry is a log, and the work goes on with the [profile of a
search](../../../docs/design/engine.md#the-profile-of-a-search), which `taxman solve --profile DIR` records.  A game is
named by n.

## 2026-10-23: the root's gap does not predict a game's size

The root's gap does not predict the steps of a game: across the chain's games it has the wrong sign (r = -0.71).  The
data are 35 searched games, 4851 to 4945, each run as a ladder of 2,000 steps with `--profile`, and a script
(`gapfit.py`, in the lab's work folder `~/taxman_work/lab/work/rootgap/`) fitted the steps to the root gap:

* games 4900 to 4929 all have a root gap of 2,048, and took 2.3 × 10⁸ to 4.0 × 10⁸ steps;
* games 4935 to 4945 have root gaps of 2,137 to 2,191, and took 0.8 × 10⁸ to 1.3 × 10⁸ steps.

So a game's size is decided below the root.

## 2026-10-23: game 4949, the first chain game profiled whole

In a game's tree a few subtrees hold nearly all of the steps, and a subtree's size follows its gap.  Game 4949 took
82.5 × 10⁶ steps, and the profile gives:

* the largest subtree holds 88% of the steps at depth 4, 36% at depth 17, and 5.6% at depth 30, where the largest 100
  hold 76%;
* within the game a subtree's size follows its gap: +0.42 decades of steps per 100 of gap, with a rank correlation of
  0.88 to 0.96 at depths 4, 17 and 30;
* the profile made 91,565 ids, close to the limit of 100,000 a game;
* a step cost 1.24 ms on 86 threads: the GF(2) bound 66%, conflict branching's pricing 20% and strong branching 7%;
* steps lie at nodes of 130 to 210 promotions.

A subtree thus grows about 2.6 times per 100 of gap, so a tighter bound everywhere and a cheaper GF(2) solve are the two
levers.

## 2026-10-23: hard games and easy games at the top of the tree

In hard games the gaps of the nodes made at depths 12 to 21 stay high, which the first path does not show.  The runs are
ladders at afbc256 with `--seed-opt` on 8 threads, about 270,000 steps each, and their scripts are in the lab's work
folder `~/taxman_work/lab/work/rootgap/top/`.

* **Along the first path** the gap ends about the same: 2,048 to 1,841 by depth 8 in the hard games 4921 and 4929, and
  2,191 to 1,855 by depth 10 in the easy games 4941 and 4945.
* **Across the nodes made** the p90 of the gaps at depths 12, 15, 18 and 21 is 1,631, 1,620, 1,620 and 1,175 in the hard
  games, and 996, 766, 671 and 578 in the easy ones.
* **Neighboring games share the top of their tree**: 4921 and 4929 have the same gaps at every depth, as do 4941 and
  4945, which is why hard and easy games come in runs.

**Open hypothesis.**  In hard games, excluding a member leaves near-equal alternatives, so the bound barely falls at a
side branch.  Nothing here tests it.

## 2026-10-23: where the hard games spend their steps (Q1)

The hard games are 4920-4929, which took 3 to 4 times the steps of their neighbors; the easy games are 4932-4949.  The
study's runs are whole-game profiles of 4947, 4949 and 4950 and 1-thread deep profiles of 4921, 4925, 4929 (hard) and
4941, 4945, 4949 (easy), in the lab's `hardgames/` folder (see Files).

* **The top of the tree.**  Every node of the spine has two or three children.  `#0` promotes a member of the heaviest
  set, and the bound does not move (drop 0 at 90% of such children).  `#1` goes on without that member, and its drop is
  the price of leaving it out.  `#2` goes on without both and forces many members at once (drops of 960 to 1,700, tiny
  subtrees).  So the spine's gap never falls by branching (4949: 1,855 from depth 9 to past 30; 4950: 2,049), and the
  tree is the sum of the `#1` subtrees hung off the spine.  The same members are branched on in the same order in 4949
  and 4950, at prices equal to a few points (544, 344, 309, 328 at depths 2, 3, 4, 6; 543, 439, 357, 235 at depths 16,
  20, 24, 29 in 4949).  What differs is the gap they are taken from: 4950's root gap is 2,424 against 4949's 2,191,
  every node of 4950 sits 200-230 points higher, and at a decade per 240 points that is the factor 5 to 8 between the
  games' subtrees (4950's `#1` at depth 4 holds 120 × 10⁶ steps, 4949's 21 × 10⁶).
* **Below the top** every side child is `forced`.  Its drop against its parent, by the parent's gap:

  | parent gap | hard 4921 = 4925 | hard 4929 | easy 4941 | easy 4945 = 4949 |
  |---|---|---|---|---|
  | 0-299: median drop; share under 50 | 33; 0.86 | 33; 0.87 | 82; 0.28 | 48; 0.57 |
  | 300-599: median | 181 | 194 | 176 | 176 |
  | 600-899: median | 247 | 251 | 266 | 272 |
  | 900 and up: median | 384 | 365 | 384 | 389 |

  The growth weight of a node (the sum over its children of 10^(-drop/240)) at parent gaps 150-299 is 1.12 in the hard
  game against 0.96 and 1.07 in the easy; at gaps of 300 and more it is the same (0.93-0.97).

**Answer.**  The open hypothesis of the previous section is false at the top and in the middle of the tree: leaving a
member out costs the bound the same in hard and easy games.  The difference is in nodes of gap under 300, where in hard
games 86-87% of the exclusions cost under 50 points (28-57% in easy games).

## 2026-10-23: a measure known in the first minutes (Q2)

**What does not work.**  Gap measures of the top of the tree from a 60-second ladder run (37 games 4851-4949: the root
gap, the spine's gap at depth 30, the mass sum 10^(gap/240) of the side branches made, and of the open units at the
run's checkpoints) correlate negatively with log10 of the chain's steps (r -0.36 to -0.75).  A 60-second run sees only 4
to 11 side branches of the top, and the games before 4875 ran with other tactics, so the 37 games are not one
population.

**What works: the frontier's mass times the size of a subtree at a fixed gap.**  For each of 14 games 4920-4949, a
60-second 8-thread ladder run with `--seed-opt` keeps its final checkpoint, and `tune units` lists its 434-488 units with
their gaps.  E is log10 of the sum over the units of 10^(gap/240), the frontier's mass.  Twelve units a game, 4 in each
gap band 500-599, 700-799 and 900-999, are solved to the end on one thread, and a is the fit of log10 steps at gap 750
(a line in the gap).

| game | log10 steps (chain) | E | a(750) | E + a |
|---|---|---|---|---|
| 4920 | 8.51 | 8.23 | 3.47 | 11.70 |
| 4921 | 8.59 | 8.23 | 3.53 | 11.76 |
| 4925 | 8.60 | 8.23 | 3.26 | 11.49 |
| 4928 | 8.58 | 8.23 | 3.26 | 11.49 |
| 4929 | 8.60 | 8.23 | 3.41 | 11.64 |
| 4932 | 8.41 | 8.06 | 3.43 | 11.49 |
| 4935 | 8.11 | 7.86 | 3.28 | 11.14 |
| 4936 | 8.02 | 7.86 | 3.29 | 11.15 |
| 4940 | 7.92 | 8.07 | 3.15 | 11.22 |
| 4941 | 7.99 | 8.10 | 2.97 | 11.07 |
| 4944 | 7.91 | 8.10 | 3.20 | 11.30 |
| 4945 | 8.02 | 8.10 | 3.12 | 11.22 |
| 4947 | 8.02 | 8.10 | 3.07 | 11.17 |
| 4949 | 7.92 | 8.10 | 2.90 | 11.00 |

Against log10 steps, E has r 0.64, a 0.75, and E + a 0.89 (slope 1.07, rms residual 0.13 decades against a spread of
0.29).  The slope near 1 is what the model "size = frontier mass × size of a subtree at a given gap" predicts.  The hard
games' frontiers are only 0.13 decades heavier than the easy ones' (4920-4929 share E); their subtrees at the same gap
are 0.3 decades (2 ×) bigger.  A second sample of 12 units a game gives a(750) with r 0.62 against the first and a mean
difference of 0.12 decades, so 12 units are noisy; pooled (24 a game) a has r 0.78, E + a r 0.86, slope 1.07, rms
residual 0.15, and the hard games have a 3.25-3.54 (mean 3.39), the easy 3.00-3.30 (mean 3.16).  Neighbors share the top
of their trees, so the 14 games are about four independent clusters (4920-4929, 4932, 4935-4936, 4940-4949).  The
measure costs one 8-thread minute and 1 to 3 CPU minutes of solves a game.

**Is 4950's incumbent too low?**  Its root gap of 2,424 is 233 above any other game's, and its search has not improved the
seed of 549,269 in over 145 × 10⁶ steps.  The profile cannot tell a low incumbent from a high bound, since both shift
every node's gap alike.  The finder run at the end of this entry tests the first reading.

## 2026-10-23: a lower gate for conflict branching in hard games (Q3)

**Question.**  The hard games' excess sits in nodes of gap under 300, below the gate (`CONFLICT_BRANCHING_GAP` = 300),
where the search branches on its own conflict and its cheapest members cost under 50.  Does choosing the conflict there
too (a lower gate, with `conflict_look`) remove the excess?  A second lever, a bound that charges a twin ladder at once,
needs a proof per rule and is untried.

**Method.**  The pooled samples of the previous section, deduplicated: 315 distinct units of the 14 games (18 to 24 a
game).  Each is solved to the end on one thread (`tune solve` of the lab's build at d45a513, which gives the same trees
as afbc256) under four settings given explicitly, interleaved per unit, at most 8 solves at once at `nice -n 19`: A gate
300 and `conflict_look` off, B gate 300 and look on, C gate 150 and look on, D gate 0 and look on.  All 1,260 solves
completed.  Steps and CPU seconds are summed over a group and given relative to A; CPU is the solve's own thread time,
which the chain's load does not change.

| group | A: steps, CPU | B: steps, CPU | C: steps, CPU | D: steps, CPU | a(750): A, B, C, D |
|---|---|---|---|---|---|
| hard 4920-4929 (116 units) | 834,000; 1,197 s | 0.76, 0.71 | 0.44, 0.81 | 0.41, 0.94 | 3.39, 3.07, 2.91, 2.91 |
| easy 4932-4949 (199 units) | 863,000; 1,545 s | 0.50, 0.54 | 0.36, 0.62 | 0.36, 0.70 | 3.17, 2.65, 2.64, 2.65 |

The CPU a step costs, relative to A, is 0.93 (B), 1.85 (C) and 2.28 (D) in the hard games, and 1.09, 1.70 and 1.94 in
the easy.  Per game, the steps relative to A run from 0.23 to 0.83 for B, 0.17 to 0.60 for C and 0.17 to 0.55 for D, and
the CPU from 0.30 to 0.96, 0.33 to 0.98 and 0.36 to 1.05.  A unit of a hard game takes 10.3 s in A and 7.3 s in B, an
easy one 7.8 s and 4.2 s, so the hard games' CPU a unit is 1.33 times the easy games' in A and 1.75, 1.74 and 1.79 times
in B, C and D.  The a(750) excess, hard less easy, is 0.23 decades in A, 0.42 in B, 0.27 in C and 0.26 in D.

1. **The look pays everywhere, and less in the hard games.**  B cuts the steps to 0.76 (hard) and 0.50 (easy) and the
   CPU to 0.71 and 0.54, so the hard games' excess grows from 0.23 to 0.42 decades.  The whole games of the live lab
   agree in direction: the look took 0.85 of its neighbor's steps (geometric mean).
2. **A lower gate gives back steps and costs CPU.**  From B to C the hard games' steps fall by 0.32 of A's and the easy
   ones' by 0.14, and the excess falls from 0.42 to 0.27, but a step costs 1.85 times as much (1.70 easy), so the CPU
   rises from 0.71 to 0.81 of A (0.54 to 0.62).  D adds nothing in steps over C and costs more CPU (0.94, 0.70).  Neither
   lower gate removes the excess: it stays at 0.26-0.27 decades against 0.23 in A, and the CPU excess stays at 1.7-1.8
   against 1.33 in A.
3. **B has the lowest CPU in every game.**  A gate set per game does not help.

**Confidence.**  High that the look at gate 300 cuts CPU by a third to a half on these units and that lower gates cost
1.7 to 2.3 times the CPU a step.  Medium that a lower gate does not remove the excess: a(750) rests on 18 to 24 units a
game and the hard games are about two independent clusters.  The units are subtrees of 60-second checkpoints at gaps of
500 to 999, so a whole game's gain differs: the live lab's whole games took 0.79 to 0.97 of the steps with the look on,
less gain than these units' 0.50 to 0.76.

**Decision.**  A lower gate is not adopted.  The look at gate 300 pays in all games (the lab turns it on from 4948; the
code's default is left open).  The hard games' excess below the gate is not a matter of which conflict is chosen; the
levers left are a bound that charges a twin ladder at once and a cheaper GF(2) solve.

## 2026-10-23: the finder on 4950's start

**Question.**  Is there a better game of 4950 that its seed missed?  **Method.**  `finder 4950 --target 549270 --known
strict.jsonl --seconds 600 --seeds 0` (the lab's build, one thread, `nice -n 19`).  Its start incumbent, made from the game
of 4949, has promotion sum 548,948 (41 s), and the finder runs from there for 600 s.  **Result.**  It reached 549,126
at 30 s (one improvement) and nothing after: neither the target 549,270 nor the seed's 549,269.  The finder finds no
game above the seed, which is evidence for the seed, but weak: it ended 143 short of the seed from its own start, so
it could miss a game a few points better.  One seed on one thread; confidence medium-low.  The root gap's excess of
4950 stays unexplained.

## 2026-10-23: a cheaper choice of the conflict below the gate (Q4)

**Question.**  A lower gate (the previous section) cuts steps and costs 1.7 to 2.3 times the CPU a step.  The choice only
selects among true conflicts, so its prices need not be exact: does a cheaper choice at nodes of gap 150 to 299 keep
most of C's step savings at nearly B's cost, and so beat B's CPU in hard and easy games?

**Where C's extra CPU goes.**  The phase clock of the profile (one step in 64 timed) and timers in the choice, on 20 hard
units (206,466 steps and 266 s in B, 115,782 steps and 315 s in C, one thread each, the same units in both):

| seconds on the 20 units | bound | pricing | strong branching | the rest | total |
|---|---|---|---|---|---|
| B (gate 300, look) | 148 | 71 | 25 | 22 | 266 |
| C (gate 150, look) | 132 | 164 | 21 | 12 | 315 |

C saves 16 s in the bound and 4 s in strong branching, and the choice costs 93 s more.  Of the pricing's 164 s (B's 71
s): the exact prices (`prices_excluding`) 99 s (38), the ordered conflicts (`conflicts_ordered`) 39 s (16), the look 10
s (10), the members of the failing frames' conflicts 7 s (3).  The choice runs at 0.445 of C's steps against 0.096 of B's,
and costs 3.2 ms a call in both (92 members priced exactly a call in C, 109 in B).  So the extra is the choice at the
nodes between the gates, mostly the exact prices, and a cheaper estimate has something to cut: a free choice would put C at
about 0.84 of B's CPU on these units.

**Variants.**  Run as environment knobs of one build of `tune` (a temporary patch, since reverted, applying to nodes of gap
under 300, with `conflict_look` on and gate 150 unless said), each solving the 315 units of the previous section on one
thread:

* M1: no exact prices below 300, the weight split's estimate for every member.
* M2: exact prices only for the members whose split estimate of the price is under a threshold (60).
* M3: M1 with no look below 300.  M5: the look off below 300 and nothing else.
* M7: exact prices of the members of the search's own conflict `K` first; if the weight of `K` is under `W`, branch on `K`
  at once, else the whole choice.  Measured with `W` from 0.75 to 2.5 and gates 150 and 200.

**Result.**  Steps and CPU relative to B, rerun with the same build (hard: 116 units, 635,034 steps, 849 s; easy: 199
units, 429,628 steps, 852 s).  B's rerun matches the previous section's (hard 0.76 and 0.71 of A, 0.44 and 0.81 for C).

| setting | hard: steps, CPU | easy: steps, CPU |
|---|---|---|
| C: gate 150, full choice | 0.571, 1.152 | 0.730, 1.130 |
| M1: split prices only | 0.737, 1.202 | 0.655, 1.030 |
| M2: exact where split price under 60 | 0.573, 1.092 | 0.745, 1.114 |
| M3: M1, no look | 0.741, 1.068 | 0.665, 0.941 |
| M5: C, no look below 300 | 0.574, 1.118 | 0.774, 1.137 |
| M7, W 0.75, gate 150 | 0.570, 1.112 | 0.732, 1.094 |
| M7, W 1.0, gate 150 | 0.573, 1.085 | 0.735, 1.068 |
| M7, W 1.5, gate 150 | 0.591, 0.990 | 0.754, 1.010 |
| M7, W 1.5, gate 200 | 0.663, 0.963 | 0.778, 0.976 |
| M7, W 2.0, gate 200 | 0.706, 0.942 | 0.796, 0.939 |
| M7, W 2.5, gate 150 | 0.696, 0.964 | 0.811, 0.947 |

1. **The split's estimate of a price is too loose to choose with.**  The weight split bounds the set without a member
   from above, so its price is a lower bound, and it is under 60 for nearly every member (M2 prices 82 members exactly a
   call against 92).  M1 and M3 lose most of the step savings in the hard games (0.74 against 0.57), and the bound costs
   more where the pricing no longer prepares the children's levels: M3's bound takes 150 s on the 20 units for 150,000
   steps, B's 148 s for 206,000.
2. **The choice changes `K` at about half the nodes it runs at, and mostly where `K` is heavy.**  On the 20 units at gate
   150, 31,019 calls below 300: of the calls with the weight of `K` under 0.5 (652) 19 chose another conflict, under 0.75
   (4,224) 465, under 1 (9,091) 1,419, under 1.5 (18,904) 5,696, and of 12,115 calls at 1.5 or more 10,132 (84%).
3. **Skipping the choice where `K` is already light gives back CPU a few percent at a time.**  M7 keeps the steps at W
   up to 1.0 (0.573) and its CPU stays above B's; at W of 1.5 to 2.5 it falls to 0.94-0.99 of B's in both groups, with the
   steps at 0.59 to 0.81.  The best, 0.94 of B's CPU in both groups (steps 0.71 and 0.80), is a gain of 6%, within the
   noise of CPU on a machine with every other hyperthread busy.

**Confidence.**  High that none of the variants beats B's CPU by more than 6% in either group, and that the extra of the
lower gate is the exact pricing and the ordered conflicts of the nodes between the gates; medium on the 6% of the best
(one run of each, CPU of a thread on a busy machine).  The variants are a few of many: a cheaper price from the parent's
nodes or a cheaper `conflicts_ordered` are untried.

**Decision.**  Not kept; no tactic or second gate is added, and the patch is not committed.  The gate stays 300.  If the
levers are taken up again: carry a node's prices to its next step (the node's loop prices again at every step), and make
`conflicts_ordered` cheaper, which is a quarter of the choice's cost.

## Files

The scripts and their output are in the lab's work folder on the machine of the chain, not copied here.
`~/taxman_work/lab/work/rootgap/` has `gapfit.py` and the 35 ladder runs, and its `top/` has the scripts of the
top-of-tree runs.  `~/taxman_work/lab/work/hardgames/` has the study's scripts (`units.sh`, `solve-bands.sh`,
`bands.py`, `drops.py`, `spine.py`, `deepdrops.py`, `fanout.py`), its runs (`units/`, `ckk/`, `solves/`, `solves2/`,
`solves-all/`, `deep/`, `prof/`) and, in `gate/`, the gate experiment (`run.sh`, `analyze.py`, `analysis.txt`, `out/`
with a file a solve) and the finder run (`finder-4950.jsonl`); `cheapchoice/` has the cheap-choice runs (`run.sh`, `phases2.sh`,
`analyze.py`, `phase_table.py`, `analysis.txt`, `out/` and `ph/` with a file a solve).
