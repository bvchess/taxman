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

## 2026-10-23: a postmortem of the game before as a guide to the choice (Q5)

**Question.**  Can a postmortem of game n − 1 tell game n which conflict to branch on, so that n skips the pricing of
the previous section?  Any true conflict keeps the proof exact, so a hint only has to be checked as a conflict at n's
node.  Stage 1 asks how often the conflict n's search chooses was also chosen in n − 1's search.  Stage 2, a hint patch,
was not built.

**Answer.**  Raw recurrence is high, 75-87% in the hard pairs, but it is not information a hint can use.  Most of it is
a conflict used again within the same search (59-71% of the choices); a game far from n shares 36-70% of them; and the
earlier game's choices inside a node's heaviest set number in the hundreds, among which the most often chosen one is the
right one under 2% of the time.  A table keyed by the node serves 31-35% of the choices in two hard adjacent pairs and
none in the other pairs.  The idea is dropped.

**Method.**  A temporary logging patch (`postmortem/logging.patch`, not committed, steps identical to the unpatched
build on all 630 solves) writes, at every choice made by `conflict_to_branch_on`, the conflict chosen `k` and the
search's own `K`, the node's gap, the node's heaviest set, and the steps of the subtree below the call.  A conflict is
named by the sorted list of its members' numbers: a member is the number promoted, the same number in every game, and a
candidate keeps its home frame in both games ([Lemma R](../../../../docs/theory/search-rules.md#the-near-repeat-lemma)), so the
list names the same set of moves in n and in n − 1.  The 315 units of the section on the lower gate (14 games, 18 to 24
units a game) were solved under B (gate 300, `conflict_look` on) and C (gate 150, look on), one thread each, 8 at once,
with the known games of the chain's file as the near-repeat cut's reference.  The logs hold 125,000 choices (B) and
305,000 (C).  A game's reference is the union of the conflicts chosen in the units of the earlier game, which is a
sample of a twentieth of that game's units, so every share below is a lower bound on what the whole earlier game gives.
Games 4924 and 4927 have no units, so the pairs are the ones the units allow.

**Recurrence.**  The share of the choices of n whose conflict was chosen anywhere in the units of the reference game,
for B and C.  "Own search" is the same share against the other units of the same game: the mean over the 14 games with
12 reference units is 0.54 (B), and with 2, 4 and 8 units 0.23, 0.35 and 0.45, so the twenty or so units of the earlier
game, which give 0.75 for the hard pairs, hold more than the own search's twelve do.

| n | reference | kind | B | C |
|---|---|---|---|---|
| 4921 | 4920 (n − 1) | hard | 0.774 | 0.870 |
| 4929 | 4928 (n − 1) | hard | 0.747 | 0.812 |
| 4936 | 4935 (n − 1) | easy | 0.749 | 0.800 |
| 4941 | 4940 (n − 1) | easy | 0.493 | 0.619 |
| 4945 | 4944 (n − 1) | easy | 0.576 | 0.649 |
| 4949 | 4947 (n − 2) | easy | 0.554 | 0.622 |
| 4928 | 4925 (n − 3) | hard | 0.585 | 0.720 |
| 4929 | 4925 (n − 4) | hard | 0.658 | 0.770 |
| 4936 | 4932 (n − 4) | easy | 0.721 | 0.780 |
| 4945 | 4941 (n − 4) | easy | 0.537 | 0.687 |
| 4949 | 4945 (n − 4) | easy | 0.739 | 0.790 |
| 4949 | 4929 (far) | easy against hard | 0.572 | 0.697 |
| 4945 | 4929 (far) | easy against hard | 0.521 | 0.663 |
| 4929 | 4949 (far) | hard against easy | 0.381 | 0.522 |
| 4921 | 4949 (far) | hard against easy | 0.355 | 0.522 |

Weighted by the steps below each choice the shares are lower, 0.43-0.71 (B) in the near pairs and 0.27-0.38 in the far
controls.  Of the choices where the search's own conflict `K` was not kept, the share is 0.65-0.68 (B) for the hard n −
1 pairs, 0.20-0.44 for the far controls.  Counting each distinct conflict once, the shares for the hard n − 1 pairs are
0.57 (B) and 0.57-0.62 (C), and for the far controls 0.14-0.30 (B).  Pairs of near-repeat nodes are not separated: the
log does not tell them from other nodes, and the cut ends them before the choice, so they are not among the choices.

**What the recurrence is.**  Three cuts of the same logs:

1. **Most of it is the search's own memory.**  59-71% of the choices of the hard games repeat a conflict chosen earlier
   in the same unit, and 54-74% in the easy.  The earlier game is needed only for the others, the novel choices (27-46%
   of the calls).  Of those, the share found in the earlier game is 0.665-0.736 for the hard n − 1 pairs, 0.375-0.680
   for the easy n − 1 pairs and 0.230-0.384 (B) for the far controls.
2. **There is nothing to pick the right one with.**  A hint must be a conflict of the earlier game that lies inside the
   node's heaviest set.  The median node has 160 to 960 such conflicts in B (270 to 1,300 in C).  Offering the one
   chosen most often in the earlier game, on a sample of 4,000 novel choices a pair, gives the right conflict in
   0.2-1.6% of them (B) and 0.1-0.6% (C), and within the three most often chosen 0.6-3.6% and 0.4-1.3%.  Choosing among
   hundreds of conflicts is the pricing the hint was to save.
3. **A table keyed by the node does better, in two pairs.**  Keyed by the node's heaviest set (the log has no other name
   for a node), a table of the earlier game's choices serves 0.31 (B) and 0.35 (C) of the choices of 4921, and 0.34 and
   0.34 of 4929, with the same conflict in 99-100% of the nodes it finds.  In the other pairs (the easy n − 1 pairs, n −
   2 to n − 5, the far controls, and 4925 against 4920 and 4921) it finds at most 0.3% of the choices.  The two pairs
   that match are hard games whose units overlap in nodes; why only they do was not checked.  The units sampled for each
   game differ, so the two matches are not a measure of a whole game's.

**The ceiling.**  The choices cost 3.2 ms each (the figure of the previous section), which is 0.227 of the CPU of the
hard units in B and 0.237 of the easy, 0.507 in C and 0.474 (the logs give 0.097 and 0.148 choices a step in B, 0.437
and 0.466 in C, as the previous section's 0.096 and 0.445).  A share X of the choices taken from a hint at no cost saves
X times that:

| X | B hard | B easy | C hard | C easy |
|---|---|---|---|---|
| 0.1 | 0.023 | 0.024 | 0.051 | 0.047 |
| 0.3 | 0.068 | 0.071 | 0.152 | 0.142 |
| 0.5 | 0.113 | 0.119 | 0.253 | 0.237 |
| 0.75 | 0.170 | 0.178 | 0.380 | 0.355 |

(shares of the CPU of the 315 units: B 869 s hard and 860 s easy, C 1,003 s and 988 s, the logged runs; C costs 1.154 of
B in the hard games.)  C needs X above 0.26 of its choices to match B on CPU.  A node table at the 0.33 of the two
matching pairs puts C at 0.96 of B's CPU and B at 0.93 of itself; a hint that is always the best conflict, X of 0.8,
puts C at 0.69 of B and B at 0.82 of itself, the most this idea can give.  Both leave out the cost of checking a hint as
a conflict and the steps lost when the hint is not the best, so they are ceilings.

**Confidence.**  High that raw recurrence is over 50% in the hard pairs and that it is no guide to the choice: the
memory inside the search, the far controls and the selector's 2% say so from the same logs.  Medium that a node table
does not generalize: two pairs match and the rest do not, in samples of a twentieth of each game, and the heaviest set
is only a name for a node.  Not measured: a table from the whole earlier game (every node, not a sample of units), whose
hit rate in the hard adjacent games could be above the 0.33 seen here; it would need the log of a whole game, about 10⁸
choices.

**Decision.**  Drop the conflict-keyed hint: the share it could take is mostly the search's own memory, and nothing
cheap picks the right conflict among the hundreds it could offer.  If the idea is taken up again, the form to try is a
table keyed by the node, built from the earlier game's whole log, for the hard games' adjacent pairs, and its ceiling at
the 0.33 seen takes C to 0.96 of B's CPU and saves 7% in B.  The rule of this stage (under 30% drop, over 50% go on) was
written for a hint that could be picked; the raw share passes it and the pick fails.

## 2026-10-23: price reuse in the choice (Q6)

**Question.**  The choice of the conflict (Q4) is 23 to 25% of the CPU at gate 300, and a node's loop prices its members
again at every step although a step changes the node little.  Does keeping a node's prices from one check to the next
(the tactic `price_reuse`, off by default) cut the CPU, in the hard games and the easy, and does it make a lower gate
pay?  **Answer.**  Yes at gate 300: on the 315 units of games 4920-4949 the tactic takes 0.82 of the CPU in the hard
games and 0.86 in the easy, with 0.86 and 0.97 of the steps, and every one of the 14 games falls (0.72 to 0.92).  At
gate 150 it helps too, but not past gate 300 with the tactic, so the gate stays 300.

**Method.**  The tactic keeps, for the node at each depth of a searcher's path, the price of every member that the choice
priced exactly (what leaving it out costs the bound); a later check of the node, and the first check of a child down to
a number of generations, takes those prices and makes only the missing ones; the ordered conflicts of the check before
are taken too, those still within the heaviest set ([price-reuse.md](../../../docs/design/price-reuse.md)).  It was built as environment knobs of one build of `tune` (the units of Q3: 315
distinct units of the 14 games, `tune solve`, one thread each, at most 8 solves at once at `nice -n 19`), with counters of
the choice's time.  Steps are exact; CPU seconds are a thread's, and on this machine they drift with the load by tens of
percent between runs made one after the other, so the measurements that decide are *interleaved*: each unit is solved
under every setting back to back (B solved twice, at the start and the end: within 1%).  The final measurement is of
the committed code, 315 units under B and under B with the tactic, and 50 of them (25 hard, 25 easy) at gate 150 with and
without it.

**What the cost is made of.**  B on the 105-unit sample: the choice is 23.7% of the CPU, of which the exact prices
(`prices_excluding`) 13.9%, `conflicts_ordered` 5.7%.  A pricing that must search the exchange graph for each member
costs 6.9 ms; a pricing that carries its parent's cycles costs 0.2 ms or less.  B makes 49,445 pricing calls on the
sample, in 95 s.  In the first run with reuse (a child takes its parent's prices, with a bound within 40 and a limit of
8 checks), 36,378 of the calls priced nothing and 13,054 still priced, at 6.9 ms and 89 s together, so the calls it
saved had cost B about 6 s.  That run took 1.00 and 0.96 of B's CPU (hard, easy).  So taking a child's prices from its
parent's saves little, and it can cost: a check that makes no price leaves its children's pricing without a
parent's cycles, which is what `Gf2Bound::inherit_pricing` repairs.  The expensive prices are a node's later checks,
whose heaviest set has changed, and the first check of a line of descendants.

**Results.**  Steps and CPU relative to B, interleaved runs (hard: games 4920-4929; easy: the others):

| setting | units | hard: steps, CPU | easy: steps, CPU |
|---|---|---|---|
| B (gate 300), second run | 92 | 1.000, 1.001 | 1.000, 0.996 |
| reuse, a child takes its parent's prices for 1 generation | 98 | 0.990, 0.935 | 0.970, 0.913 |
| reuse, 3 generations | 93 | 0.916, 0.878 | 0.976, 0.884 |
| C (gate 150) | 96 | 0.578, 1.156 | 0.732, 1.225 |
| C with reuse, 1 generation | 94 | 0.586, 0.990 | 0.745, 1.048 |
| C with reuse, 3 generations | 92 | 0.554, 0.921 | 0.751, 1.027 |
| reuse, 3 generations (50 units) | 50 | 0.881, 0.841 | 1.017, 0.900 |
| reuse, 6 generations (50 units) | 50 | 0.755, 0.754 | 1.021, 0.903 |
| reuse, 3 generations, bound may fall 150 (50 units) | 50 | 0.840, 0.816 | 0.994, 0.912 |
| reuse, 3 generations, at most 4 checks a node (50 units) | 50 | 0.881, 0.840 | 1.017, 0.911 |

The committed schedule (6 generations, a fall of the bound of 40, no limit on the checks of a node) on the final build:

| setting | units | hard: steps, CPU | easy: steps, CPU |
|---|---|---|---|
| B (gate 300), 635,034 steps and 849 s hard, 429,628 and 797 s easy | 315 | 1.000, 1.000 | 1.000, 1.000 |
| B with `price_reuse=on` | 315 | 0.859, 0.824 | 0.974, 0.855 |
| B with `price_reuse=on` | 50 | 0.755, 0.754 | 1.021, 0.898 |
| C (gate 150) | 50 | 0.536, 1.232 | 0.733, 1.184 |
| C with `price_reuse=on` | 50 | 0.431, 0.813 | 0.744, 0.917 |

Every solve completed and found the best of B's.  Steps of a unit change in either direction (the tree depends on the
ranking), and the hard games' steps fall more because their trees are deep, so the tactic's gain in steps is less
certain than its gain in CPU.

1. **The gain is in the CPU of a step, and in a few steps.**  The choice falls from 23 to 25% of the CPU to 17 to 18%
   (the exact prices 13 to 15% to 11%, the ordered conflicts 5.5 to 6% to 1.6 to 2%), and the steps fall by 3 to 14%.
2. **Gate 150 stays behind gate 300 with the tactic.**  C costs 1.16 to 1.23 of B's CPU, as Q4 found; with the tactic it
   falls to 0.81 and 0.92 of B's, against 0.75 and 0.90 for gate 300 with it.  The earlier sequential runs, which put C
   at 0.745 of B's CPU in the hard games, were load drift: the interleaved runs do not.
3. **The prices want to be fresh within a line, not forever.**  Never making a node's prices again (no limit on
   generations or the bound's fall) raised the steps by 17% and the CPU to 1.01; own-step reuse alone (no parent's
   prices) took 0.94 and 0.90; parent's prices for any number of generations and a fall of 100 took 0.97 and 0.93.

**Confidence.**  High that the tactic takes less CPU than B on these units (every game, interleaved, B repeated within
1%); medium on the size (0.82 and 0.86, with the hard games' steps varying by several percent with the schedule: 0.755
to 0.88 on 25 units); medium-low that a whole game gains as much, since the units are subtrees of 60-second checkpoints
(Q3).  The schedule was chosen on the units it is measured on, from five to eight settings.

**Decision.**  The tactic is kept, off by default, so that the default trees and the tripwires do not change.  The
chain turns it on with `--tactic price_reuse=on` in its run script, at the default gate of 300; a different gate is not
worth the change.  A checkpoint written with the tactic on has its bit 12 set, which a build before the tactic refuses
as unknown tactics; the tactic binds nothing, so a build with it resumes a checkpoint with the tactic on or off.  The
gate is no flag of `taxman solve`: it is set by the lab's `policy` or `branching` command ([the
lab channel](../../../docs/design/lab.md#a-policy-for-the-games-to-come), `{"command": "policy", "from": N, "gate": 150}`
in the channel folder), and kept in `run.lab`.

## 2026-10-27: the GF(2) bound at a node, and a cheaper search for augmenting paths

**Question.**  In the hardest games the GF(2) bound takes most of a step.  Does it still carry the frames that close as
promotions are made below the root, and can its inner loop be made cheaper with the same trees?  **Answer.**  It
carries no closed frame: a frame closed at a node leaves no live element and no part of a block behind.  Half of these
units' CPU is the search for augmenting paths in the bound's solves, and two exact changes to it take 8.4% off the CPU
of 24 units of game 4984 under the chain's settings, with every tree the same.

**Method.**  Units of game 4984's checkpoint (`~/taxman_work/lab/work/patch/units-4984.jsonl`): 24 that take under a
second to 95 seconds on one thread (gaps 676 to 1,401), solved by `tune solve` with `price_reuse=on`, and with conflict
branching both on (the chain's setting until 2026-10-27 07:19 PDT) and off (its setting since, for 4984 and after).  A
scratch build with cycle counters around the bound's functions profiled six of them, with conflict branching on.  For
the comparison, the old build and the new solve each unit at the same time on the two hyperthreads of one physical core
(`taskset`, four cores at once).  Both see the same core, caches and clock, and no thread of the chain runs beside
either.  Paired so, the unit ratios of a 1.4% change fall between 0.979 and 0.996.  Unpaired runs, interleaved but each
beside a thread of the chain, spread from 0.71 to 1.43 on the same units.

**Closed frames carry nothing.**  The bound's matroids at a node are the root's contracted by the node's promotions
(the module docs of `gf2`), and a block holds a whole layer's frames.

* *Their candidates leave.*  When a frame closes, the promotions' vectors have used up its coordinates.  A candidate
  whose vector lay in that frame alone becomes a loop, a zero vector, and leaves the node's live elements; one that
  also joins an open frame keeps only the coordinates there.  The tableaux, lists and searches of a solve hold live
  elements only.
* *No block splits.*  At 1,485 sampled evaluations on six units, the coordinates that the live elements' vectors join
  formed one piece in every block of both matroids: the sum of the squared sizes of the pieces equals that of the blocks
  to 0.005%.  So no part of a block closes off at a node, to be dropped or solved apart.
* *What remains is small.*  The eliminated coordinates stay as zero bits of four-word vectors, a width set by the root's
  largest rank (197 at 4984).  A child's vectors are copied and contracted over all the root's elements, dead ones too,
  for 0.8% and 1.2% of the CPU.

At 4984 the root has nine frames, of ranks 77, 197, 158, 88, 45, 21, 7, 3 and 1, and 1,349 elements: the blocks of `A`
hold 564, 606, 148, 28 and 3 of them, those of `B` 959, 312, 68 and 10.

**Where the time goes.**  On the six profiled units (110,205 steps, 4.9 M cycles a step, conflict branching on), as
shares of their CPU:

| part | share |
|---|---|
| the GF(2) bound, all its calls | 91.5% |
| its solves (`evaluate`) | 54% |
| of which the search for augmenting paths | 46.5% |
| of which its first pass over the elements | 10% |
| of which the arcs from outside elements to members | 13% |
| of which the arcs from members to outside elements | 24% |
| the paths applied (pivots, blocks rebuilt) | 7.3% |
| the weight splits, with tableaux made again | 11.4% |
| the searches for forced members | 9.7% |
| the bounds without a member, for the choice of the conflict | 12.5% |

A solve starts from a dominant set of 30 members on average and ends at 103: 72 augmentations, each a Bellman–Ford
search from scratch of 2.9 rounds on average.  The 14,238 solves, 0.13 a step, are 54% of the CPU.

With conflict branching off, the same six units (63,761 steps, 3.9 M cycles a step) spend 93% of their CPU in the
bound: 60.5% in its solves and 52% in the search for augmenting paths.  There a solve starts from 3.3 members on
average, nearly from nothing, and ends at 108, after 105 augmentations; the 6,001 solves are 0.09 a step.

**The change.**  Both parts are exact: the searches visit the same vertices in the same order.

1. *Flags of the free elements.*  The workspace keeps, for each live element and matroid, whether its residue is not
   zero (the element is free: the set plus it is independent there).  The flag is written wherever a residue is: a
   block built, a pivot, a derived tableau's slots and reduced rows, a restored snapshot.  The searches read a byte
   instead of a vector of four words: the augmenting path's Bellman–Ford, the split's rounds, the cycle and loss
   searches, the columns.  Debug builds check each flag read against its residue.
2. *Outside elements by kind.*  In a round of the Bellman–Ford, an element free in `A` can gain only from the hub (the
   best member), whose offer is new only when a member improved; an element in the span of `A` gains only from a
   member of its circuit that changed.  So the search lists each block's outside elements apart by kind at its start,
   in the order of the block's list, and where only one kind can gain it visits only that list.  The same elements
   change in the same order as in a scan of the whole list; a test checks every search of 200 random problems with
   tied weights against such a scan (sink, rounds, and every distance and predecessor).

**Results.**  Every unit's steps and best are the same in every run.  The new build's CPU, as a share of the old's:

| conflict branching | runs a unit | total | geometric mean of the units' ratios | the units' ratios |
|---|---|---|---|---|
| off (the chain's setting since 2026-10-27) | 2, the hyperthreads swapped | 0.916 | 0.912 ± 0.5% | 0.898 to 0.935 |
| on | 1 | 0.934 | 0.936 ± 0.5% | 0.919 to 0.971 |
| on, the lists alone | 1 | 0.986 | 0.986 ± 0.2% | 0.979 to 0.996 |

With conflict branching off there is no pricing, so the solves are a larger share of a step and the change gains more.
With the bound at 93% of the CPU with conflict branching off and 91.5% with it on, the change takes about 9% and 7%
off the bound's.  The whole game 3000 keeps its 117,197 steps, 115,973 with conflict branching and price reuse, and
every counter of `tune`.

**Decision.**  Kept, for the chain's next build.  The solves remain the largest part: with conflict branching off, a
solve's 105 augmentations start from almost nothing.  A start near the end, such as the parent's heaviest set less what
the child dropped, with its weight split to keep it extreme, would cut most of them, but it can change the trees where
heaviest sets tie, so it needs a measurement of steps; it is the next thing to try.  Two exact changes were judged not
worth a build: a per-element summary of the circuits' slots in front of the arcs from members (a third of the scanned
elements miss, for about 1%), and copying and contracting only a child's live vectors (at most 2%).

## 2026-10-27: a sweep of the settings on game 4984

**Question.**  Which settings of the search take the fewest steps and the least CPU on the units of game 4984?  The
earlier gate study found gate 300 with the look best on games 4920-4949
([Q3](#2026-10-23-a-lower-gate-for-conflict-branching-in-hard-games-q3)), and the gaps of 4984's units run higher, to
1,672.  **Answer.**  Conflict branching off, the same search as a gate of 2,200: it took 0.41 of the steps and 0.37 of
the CPU of the chain's setting on a fresh set of 30 units, and 0.60 and 0.48 on 12 hard units.  No other setting did as
well: the other gates cut fewer steps, and the rest changed them by 3% or less or raised them.

**Why a gate of 2,200 is conflict branching off.**  At gates of 1,600 and 2,200 the search tree is the same on all 12
hard units, since no check on them has a gap above 1,600.  So a gate of 2,200 never uses conflict branching, and the
default strong branching chooses every conflict.  The two spellings gave the same steps and CPU on 8 units checked
directly.  That off closes deep units in a small share of the steps agrees with [the study of game
4895](../when-a-node-should-use-conflict-branching-2026-10/README.md).

**Method.**  Units of game 4984's checkpoint (`units-4984.jsonl`), each solved to the end on one thread by `tune solve`
at `nice -n 19`, 12 solves at once, the settings interleaved per unit.  The baseline B is the chain's setting then:
conflict branching on, price reuse on, gate 300, the look on.  Ratios are against B on the same units.  On this machine,
at a load of about 90, a unit's CPU drifts by 10 to 30% between runs, so the steps are the measure and a CPU ratio
within 15% of 1 means nothing: settings that cannot change the tree (`nogood_period` of 25k and 100k, `more_forced`
off, `strong_limit` 4) showed CPU ratios of 0.83 to 0.91 at 1.00 of the steps, and B solved twice on the 12 hard units
took the same steps and 1.13 of the CPU the second time.  Run 1 solved 52 units of gap 300 to 1,500 (node limit 60,000,
all solved under B) under 17 settings.  Run 2 solved 12 hard units of gap 1,215 to 1,498 (limit 200,000; B capped on
6), and run 3 a fresh set of 30 of gap 306 to 1,672 (limit 150,000; B capped on 6), each under the gates.

**Results.**  Steps and CPU relative to B, the gate in each row:

| setting | 52 units: steps, CPU | 12 hard units: steps, CPU | 30 fresh units: steps, CPU |
|---|---|---|---|
| gate 150 | 0.98, 1.12 | - | - |
| gate 200 | 0.92, 1.06 | - | - |
| gate 450 | 1.01, 0.86 | - | - |
| gate 600 | 0.84, 0.69 | 0.93, 0.76 | 1.00, 0.92 |
| gate 900 | 0.55, 0.43 | 0.64, 0.52 | 0.71, 0.70 |
| gate 1,200 | - | 0.58, 0.44 | 0.61, 0.56 |
| gate 1,600 | - | 0.60, 0.47 | - |
| conflict branching off (gate 2,200) | - | 0.60, 0.48 | 0.41, 0.37 |

On the fresh set, off took fewer steps than B on 27 units and more on 3, and capped on none, where B capped on 6.  The
gap between gate 1,200 and off comes from the units of gap 1,450 to 1,670, which B and the gates of 600 and 900 cap on
and off finishes in 13,000 to 92,000 steps.  On the 12 hard units gate 1,200 and off are within noise of each other.
Below a gap of 1,200 the settings differ little, and the units there cost almost nothing under any of them.

The other settings, all on the 52 units, did not help.  The look off took 1.97 of the steps (3 units capped); the
forcing window of 80 and of 25% took 2.1 and 3.1; deep strong branching with a split (`strong_depth` 40,
`deep_branching=split`) and `branching=forced` took 1.4 each.  Price reuse off, the Lagrangian bound, `more_forced` off,
`strong_limit` 4 and the two nogood periods took 0.99 to 1.03, and price reuse still does nothing for the steps here.
`match_refine` was left out, since it has no effect with the GF(2) bound.

**Caveats.**  The steps are exact, and the fresh set agrees with the hard one.  The samples are capped, which works
against off, so its gain is probably larger than shown: the hardest units, of gaps 1,500 to 2,000, are
under-represented, and unit 4984-0 (gap 1,407) took B 13 minutes.  Off was not measured at gaps well above 1,700,
because no unit's checks reach them.  These are units; the effect on the whole game shows as fewer steps for the CPU
once the setting is applied.

**Decision.**  Kept.  The lab channel applied it to the running chain at 07:19 PDT: `branching` with conflict branching
off for game 4984 and `policy` with it off from game 4985.  The redeploy at 07:47 PDT on 2f124a6 kept it, and the
chain's run script no longer passes `--tactic conflict_branching=on`.  Gate 1,200, within noise of off on the hard
units, would be the hedge.

## 2026-10-27: 88 threads against 44

**Question.**  Does the chain take more steps a second with 44 workers than with 88?  The machine has 44 cores of two
hardware threads each.  **Answer.**  No: 44 workers took 0.80 of the steps a second that 88 did, in both of two pairs of
blocks, and the chain stays at 88.

**Method.**  On the live chain, in game 4984 with conflict branching still on, the lab channel's `threads` command
([the commands](../../../docs/design/lab.md#the-command-folder)) set the run's budget of threads to 88, 44, 88, 44 and
88 again, in blocks of about 12 minutes from 05:38 to 06:29 PDT.  The rate of a block is the growth of the search's
steps over its steady window, 9 minutes that start about 3 minutes into the block (the switch took effect within 12
seconds), from the one-minute `nodes` counters of the chain's `chain.err`.  Every engine thread was at idle priority
(`SCHED_IDLE`); the 88 blocks showed 88 engine threads and the 44 blocks 44.

**Results.**

| block | workers | steady window (PDT) | steps in it | steps a second |
|---|---|---|---|---|
| 1 | 88 | 05:41:26 to 05:50:26 | 24,578,000 | 45.5k |
| 2 | 44 | 05:53:26 to 06:02:26 | 19,515,629 | 36.1k |
| 3 | 88 | 06:05:26 to 06:14:26 | 26,374,130 | 48.8k |
| 4 | 44 | 06:17:26 to 06:26:26 | 21,187,181 | 39.2k |

The 88 blocks average 47.2k a second and the 44 blocks 37.7k.  Block 2 is 0.79 of block 1 and block 4 is 0.80 of block
3.  The 44 workers did not sit one to a core: at 05:57 PDT they were on 35 distinct physical cores, 9 of them holding
two workers, and at 06:21 on 36, 8 of them holding two.  A core in use does more with one worker than with two, 0.82k
to 0.89k steps a second against 0.63k, but 88 workers use more of the machine (70.9 and 77.4 cores of the chain's
against 44.2 and 44.1) and get more done.

**What could mislead.**  Other jobs at `nice -n 19` ran throughout, mostly `tune` (the sweep above among them),
`tune-carry`, `rustc` and `certify`, sampled with `top` every 27 s: 16.4 busy cores in block 1 (from 05:44), 18.9 in
block 2, 10.2 in block 3 and 16.7 in block 4.  The load was heavier in both 44 blocks than in block 3, but block 1 held
45.5k a second at a similar load, so the 20% is not an artifact of the load.

**Decision.**  The setting stays at 88 and was restored at 06:29:46 PDT, with 88 engine threads confirmed.  The
switches are the outcome files `20261027T1250Z-threads-44`, `20261027T1302Z-threads-88`, `20261027T1314Z-threads-44`
and `20261027T1329Z-threads-88` in the channel's `out/` folder, named in UTC; the load sample was not kept.

## 2026-10-27: carrying the hard core's nogoods into the next game (Q7)

**Question.**  Games 4983 and 4984 have the same top, the same structure of the root's gap, and 4983 took 8.8 billion
steps (45 hours).  Does a store of the nogoods that 4983's search learned, checked at 4984's root and used only to prune,
never to choose the conflict a node branches on, take 7% or more off the CPU of 4984's search?  **Answer.**  Yes under
the chain's settings since 07:19 PDT that day (conflict branching off, price reuse on), though not on every unit: over
71 units of 4984 a store from 144 units of 4983 takes 0.62 of the steps and 0.85 of the CPU, and one from 48 units 0.65
and 0.88, and a third of the units cost more CPU.  With conflict branching on, as the chain ran 4984 before then, the
store takes 0.87 to 0.90 of the steps but 0.97 to 0.99 of the CPU, no gain.  The steps it saves are cheap ones, and each
step left costs 10 to 33% more: a child that loses a member of its parent's heaviest set solves its GF(2) bound again
instead of inheriting it.

**Method.**  The design was fixed beforehand, from what sank the carried tables of [carrying nogoods
over](../carrying-nogoods-over-2026-10-measured-not-kept/README.md) and [carrying a search's learning to the next
game](../carrying-a-searchs-learning-to-the-next-game-2026-10/README.md): a store apart from the searcher's table, every
set checked at the new root before use, an index so that no question scans the store, and no say in which conflict a
node branches on ([carried-nogoods.md](../../../docs/design/carried-nogoods.md)).

1. *What 4983 learned.*  A logging patch (`probe.patch`) wrote out every nogood a table learned.  `tune solve` on 48
   units of 4983 (the units of its live checkpoint at 2.9 billion steps that the forecast cut, `live4983.jsonl`; gaps 300
   to 1,100; at most 30,000 steps each, the chain's tactics of the time) took 180,319 steps and learned 2,456 nogoods,
   1,385 distinct: the *store of 48 units*.  With 96 more units (gaps 300 to 1,500) the *store of 144 units* holds
   6,560 from 1.61 million steps.  The nogoods are large, a median of 31 members (quartiles 26 and 36): a nogood is a
   subset of a node's promotions and its conflict, and these nodes are 50 to 140 promotions deep.  Every one of them is
   a nogood of 4984's root, and the check at that root made none smaller.
2. *Would the store act (the stop rule)?*  The patch also held the store of 48 units beside the searches of 48 units
   of 4984 (its live checkpoint at 2.1 billion steps, `units-4984.jsonl` of the patch study; gaps 300 to 1,100), and
   counted without changing anything.  The searches computed a conflict 887 times in 179,468 steps, where their own
   tables knew none; the store held a conflict there every time, and the very nogood the search then learned 670 times
   (76%).  Of the 78,600 candidates that the tables and the known conflicts ruled out of children the store would have
   ruled out 79%, and 57,684 more besides, at 27% of the children; and its nogoods held all of a check's forced members
   at 6.5% of the checks.  The stop rule's bar was 10%.
3. *The tactic.*  `carried_nogoods` gives a search a store (`tune solve --carried FILE`): a unit's node and each child
   drop the candidates that would complete a nogood, and a check ends a node whose forced members complete one.  The
   first version tested each nogood's members at each child; the second keeps two watched members of each nogood along
   the searcher's path, as SAT solvers do, and lists each nogood's members from the rarest in the store.  Both grow the
   same trees (all 96 solves compared), and the watched version's cost of a step does not grow with the store.
4. *The measurement.*  48 more units of 4984 (12 in each gap band from 500 to 700, 700 to 900, 900 to 1,100 and 1,100
   to 1,400), each solved under every setting on one thread, at most 8 solves at once, `nice -n 19`, by a build on
   93e486b, before the cheaper search for augmenting paths of the section on the GF(2) bound at a node.  Steps are
   exact.  CPU is the thread's, and on this machine it drifts with the load: in the first round, which ran a unit's
   settings one after the other, the default's repeat took 1.22 of its first CPU on the same trees.  So the rounds
   below run all the settings of a unit at the same time.  Every solve completed and found the same best.

**Results.**  Steps and CPU of each setting against the default with the same tactics; in brackets the geometric mean
over the units of 1,000 steps or more (29 to 33 units):

| tactics | the default (steps, CPU) | store of 48 units | store of 144 units | 144, without the cut |
|---|---|---|---|---|
| conflict branching on (gate 300), price reuse | 1,335,495 steps, 2,853 s | 0.900, 0.991 (0.824, 0.947) | 0.874, 0.972 (0.809, 0.946) | 0.882, 0.975 (0.806, 0.934) |
| conflict branching off, price reuse | 595,494 steps, 1,003 s | 0.666, 0.884 (0.640, 0.873) | 0.566, 0.748 (0.578, 0.802) | 0.616, 0.730 (0.673, 0.811) |
| the same, on 23 more units (gaps 700 to 1,500) | 544,142 steps, 813 s | 0.629, 0.872 (0.626, 0.883) | 0.673, 0.973 (0.579, 0.835) | |
| the same, the 71 units together | 1,139,636 steps, 1,816 s | 0.648, 0.879 (0.634, 0.877) | 0.617, 0.849 (0.578, 0.814) | |

The 23 more units are those of a second sample of 24 (none of the others') that every setting completed; the default
took more than 400,000 steps on the 24th.  Over the 71 units, 41 and 40 take less CPU with the stores of 48 and 144
units, and of the 11 of 20,000 steps or more, 9 and 7 (0.86 and 0.83 of the CPU together).

1. **The store prunes much more with conflict branching off.**  The search then branches on the smallest conflict its
   table knows, and the store's rule-outs and cuts take a third to two fifths of its steps; with conflict branching on,
   whose choice already avoids much of what the store prunes, a tenth.
2. **A step costs more.**  With conflict branching off, a step with the store costs 1.32 times the default's (1.18
   without the cut), and the GF(2) bound solves 0.146 to 0.150 times a step against 0.107 (augmentations 13.9 to 14.1
   against 9.7).  A child keeps its parent's heaviest set, with no solve, only if it keeps all its members
   (`Gf2Bound::push_all`), and the candidates the store rules out are often members of it.  The cost does not come
   from the store's own questions: it is the same with 1,385 and 6,560 nogoods.  A cheaper solve, such as the warm
   start that the section on the GF(2) bound at a node proposes, would make the store's steps cheaper too.
3. **A larger store prunes more, mostly.**  The store of 144 units takes 15% fewer steps than the store of 48 on the
   first set and 5% fewer over the 71 units, though 7% more on the second set, at the same cost of a step; loading and
   checking its 6,560 nogoods takes 1.4 s, once a search.
4. **The cut pays in steps, not always in CPU.**  Without it the store of 144 units takes 0.62 of the steps against 0.57,
   and about the same CPU (0.73 against 0.75): the steps the cut saves are the cheapest.
5. **Sparing the heaviest set loses.**  A variant that leaves in a child the members of the heaviest set it would
   inherit, so that it inherits it, took 1.31 times the steps and 1.12 times the CPU of the store of 144 units without
   it, on 30 of the units run side by side (all 10 of 1,000 steps or more dearer), and was not kept: the pruning is worth
   more than the solves it costs.

**Confidence.**  High that the store takes steps off 4984's units under both settings (exact, 24 to 29 of 29 to 33
units fewer); medium that it takes 12 to 15% off their CPU with conflict branching off (two unit sets, settings run side
by side, a third of the units dearer, 3 to 25% on each set with each store); medium-low that a whole game gains as much:
the units are subtrees of a 2.1-billion-step checkpoint, and the stores come from units of 4983, not from its whole
search.

**How the chain would make the store.**  Each searcher would hand the nogoods of its table to a set the search shares at
each forgetting (every 50,000 steps) and when it ends, distinct and capped (at about 100,000, the most recent kept).
The search would write the set beside its checkpoint, so that a resumed search keeps it, and at its end as the store
of the next game (`nogoods-N.txt` in the chain's data folder); the next game's search would read the latest store,
check it at its own root, and add what it learns to its own.  The solver gives no store yet: `taxman solve` would need
a flag to read one and the shared set to write one.

**Decision.**  The tactic is kept, off by default, so that the default trees and the tripwires do not change; a
checkpoint records it in bit 13 and does not bind it.  With conflict branching off it passes the bar of 7% of the CPU
on units, so the next step is the chain's: the solver writing and reading a store as above, then whole games of the
patch with it against their neighbors without it, as the look was tested ([the live
lab](../the-live-lab-2026-10/README.md#2026-10-23-the-look-tested-on-whole-games)).

## 2026-10-27: the store in the solver, and a store from the game's own units (Q7, continued)

**Question.**  Does a store learned from units of 4984 itself help other units of 4984 more than 4983's store does, so
that the searchers of a game should share what they learn as they go?  **Answer.**  No: a store learned from 144 units
of 4984 takes 0.60 of the steps and 0.78 of the CPU, 4983's 0.57 and 0.74, and their union 0.56 and 0.74.  So the
solver carries each game's store to the games after it, and its searchers do not share theirs as they go.

**Method.**  144 units of 4984 that neither unit set above holds (gaps 300 to 1,500), solved with the logging patch under
the chain's settings, at most 30,000 steps each (711,119 steps in all), learned 6,702 nogoods, 2,364 distinct, 843 of
them in 4983's store of 144 units.  The 48 units of the first set were solved under each store and without one, the four
settings of a unit at the same time, by a build on 2f124a6.

| store | nogoods | steps | CPU | geometric mean over 29 units of 1,000 steps or more: steps, CPU |
|---|---|---|---|---|
| none: 595,494 steps, 904 s | | 1 | 1 | |
| 144 units of 4983 | 6,560 | 0.566 | 0.739 | 0.578, 0.785 |
| 144 units of 4984 | 2,364 | 0.601 | 0.782 | 0.644, 0.870 |
| both | 8,081 | 0.564 | 0.739 | 0.574, 0.789 |

The store of 4984's own units is smaller, since a search with conflict branching off learns fewer distinct nogoods, and
it prunes less; the union prunes as 4983's does.  On this build 4983's store takes 0.74 of the CPU, against 0.75 on the
build before the cheaper search for augmenting paths.

**The solver.**  `taxman solve --carried-dir DIR` turns the tactic on: each game's search reads the stores of its game
and the games before it in DIR, checks them at its root, and writes its own there, of what its searchers learn
([carried-nogoods.md](../../../docs/design/carried-nogoods.md)).  Games 1001 to 1010 solved so, with a store that game
1000's search learned, all match `optimal.json` (1001 in 1,663 steps against 1,790), and game 3000 from the root with no
store takes its 117,197 steps.  The runs are in `~/taxman_work/lab/work/carry/b1/` (the learning) and `b2/` (the
measurement).

## 2026-10-27: warm starts by deletion

**Question.**  With conflict branching off, a solve of the GF(2) bound starts from 3.3 members and makes 105
augmentations to a heaviest set of about 108.  Can a level start from its parent's heaviest set instead?  **Answer.**
Yes, by deleting from that set the members the level lost, one at a time, each along one shortest cycle of an exchange
graph that admits every size, with no augmentation (the tactic `warm_start`, off by default).  On 24 units of game 4984
under the chain's settings it takes the CPU to 0.48 of the old build's (unit ratios 0.30 to 0.64), with every tree the
same.  The tactic stays off by default because a tree can change where several heaviest sets weigh the same, as game
3000's does with conflict branching off.

**Where the solves come from.**  A counting build (a level tagged with how it was made, each solve counted by the tag)
ran the six units profiled in the section on the GF(2) bound at a node (63,761 steps), with conflict branching off and
price reuse on:

| the level solved | solves | share of the solves' cycles | augmentations a solve |
|---|---|---|---|
| a child that lost one member of its parent's heaviest set, which the cycle did not certify | 4,448 | 73.3% | 104 |
| a child that lost two to four (379 two, 41 three, 2 four) | 422 | 5.4% | 78 |
| a node that went on without a member of its own heaviest set, no level kept for it | 1,121 | 21.1% | 115 |
| a unit's root, and two children whose parent's tableau the workspace no longer held | 8 | 0.3% | 200 |

Of the 31,851 children pushed, 63.6% inherited their parent's heaviest set and 21.1% were certified by one cycle, with
no solve; the solved ones are the rest.  A child keeps only its parent's dominant sets that hold what it promotes and
avoid what it drops (`gf2`'s module docs, "Warm starts").  The members it drops are heavy, the divisors of the number
promoted among them, so they came in with the first few augmentations, and the sets from before them are the few members
a solve starts from.  The nodes go on without a member that strong branching did not price: the member of a conflict of
one, or one that a rule drops from the node's candidates at the start of a step.

**Why the parent's set less the lost members is no start.**  The augmentations may start from any common independent
set that is extreme (the heaviest of its size, which feasible potentials certify) and heavier than every smaller one.
Contracting the members promoted keeps the parent's heaviest set `I` the heaviest of all sizes, and its weight split a
split, and so does deleting elements outside `I`: circuits only lose members, and free elements stay free.  Deleting a
member does not.  Every element whose circuit held it becomes free in that matroid, and the split's condition for a
free element (`w1 ≤ 0` in `A`, `w2 ≤ 0` in `B`) fails wherever its `w1` or `w2` is positive, so the set less the member
is in general not even extreme (the test of 2026-10-13 found it extreme at 2 of 2,543 nodes).  The look child of the
cheaper look is the same case seen from the other side: its heaviest set without the member and its substitute is
smaller than the node's, and no cycle of the node's exchange graph, which keeps the size, reaches it.

**The deletion.**  The heaviest set without a member `x`, of every size, is `I` flipped along the shortest cycle through
`x` in the exchange graph of `I` with two elements of weight 0 added, each independent of everything: `o` outside the
set and `i` in it (`Meet::without_any_size`).  A cycle through `o` is a path between members of `I`'s own graph, which
makes a set one smaller, and a cycle through `i` is a path from an element free in `A` to one free in `B`, which makes
a set one larger.  Every set without `x`, with copies of `o` and `i` added to make its size `I`'s, differs from `I` by
cycles of lengths that are not negative under the split, one of them through `x`, and a cycle through two copies of
either is no shorter than one through a single copy.  So no bound on what the other sizes lose is needed, which is all
`Meet::without` lacked; the search is its Dijkstra over the split's reduced lengths, `o` reached as an element free in
both matroids and `i` as a member through the hub.  The levels use it three ways, each set found the heaviest of all
sizes of its problem:

* *A child that lost one member* finds its heaviest set by one search of its parent's graph, the promoted members and
  the other elements it dropped banned (`warm_child`).
* *A child that lost more* deletes the others in its own problem, from the tableau of the set so far, derived from the
  parent's, and its weight split, found anew each time.
* *A node that goes on without a member* deletes it in place (`warm_delete`) and keeps a copy of the tableau for its
  own to be derived from, after the child's subtree has used the workspace; where the node has no split yet (a rule
  dropped the candidate at the start of a step), it is found first.

The first build made no split for a node that had none when it went on without a member, and left 1,017 solves on the
six units, 1,009 of them such nodes: it took the CPU of the 24 units to 0.577 (geometric mean 0.546, unit ratios 0.351
to 0.692, one run each), with 2,520 solves left of 16,840.  The final build leaves the units' 24 roots and 43 other
solves.

**Tests.**  `without_any_size` against brute force for every member of the heaviest set of 120 random problems (4,000 in
the long test), with and without banned elements, its set checked common independent and of its weight; the walk of
random paths of the boards 60 to 330 with the tactic, every node's bound against the matroids built afresh, and in debug
builds every child and node warm-started against a solve; the soundness configurations 52 and 53, and
`conflict_branching=off,warm_start=on` among the solver's variants of every tactic.

**Results.**  Paired as in the section on the GF(2) bound at a node: the old build (2f124a6) and the new one solve each
unit at the same time on the two hyperthreads of one core, four cores at once, two runs a unit with the hyperthreads
swapped:

| conflict branching | runs a unit | total CPU | geometric mean of the units' ratios | the units' ratios | steps |
|---|---|---|---|---|---|
| off (the chain's setting) | 2 | 0.480 | 0.442 ± 9.7% | 0.303 to 0.637 | the same at every unit |
| on | 1 | 0.481 | 0.488 ± 14.1% | 0.264 to 0.943 | the same at every unit |

The two runs of a unit agree within 3.5%.  The best is the same at every unit.  Under the chain's settings the solves of
the 24 units fall from 16,840 to 67, and their augmentations from 1.70 million to 13,329; with conflict branching on,
from 35,743 to 26.  The units that gain least are those that solved least (6327 and 4790 with it on, 45 and 121 solves).
The whole game 3000 keeps its 117,197 steps with the tactic (115,973 with conflict branching and price reuse), its
solves falling from 2,233 (2,103) to the root's one; with conflict branching off its steps go from 167,554 to 161,460, a
tree changed by a tie, and its solves from 3,039 to one.  With the tactic off, game 3000 keeps every counter of
the old build.

**Decision.**  Kept, off by default, for the chain to take with `--tactic warm_start=on`: the bound at every node is the
same, so a checkpoint records the tactic (bit 14) and does not bind it, and a run may resume a search with it on or off.
The solves are gone from these units, so the bound's cost is now its splits, its searches for forced members and the
deletions themselves; a profile of the new build is the next measurement.

## 2026-10-27: a cheaper search for forced members

**Question.**  With the warm start, where does a step of game 4984 spend its CPU under the chain's settings, and can
the largest part be made cheaper with the same trees?  **Answer.**  The weight splits and the searches for forced
members take half of a step.  Finding the split's bounds only where they are read, and bounding the searches by each
member's cheapest arcs, with two smaller changes, take 24 units of 4984 to 0.868 of the CPU, with every tree the same.

**Method.**  The 24 units of the sections above, each solved by `tune solve` under the chain's tactics: conflict
branching off, price reuse, the warm start, and the store of 144 units of 4983 (`carried_nogoods`), 127,434 steps in
all.  A scratch build with cycle counters around the bound's functions gave each part's share of the steps' cycles,
with what it calls.  Its switches turned each change on in the same binary, and every unit took the same steps under
every switch.  Then the old build (88b1228) and the final one solved each unit at the same time on the two hyperthreads
of one core, as in the sections above.

**Where the time goes.**  At 1.93 M cycles a step:

| part | share of a step |
|---|---|
| the bound's evaluations, a node's own check and its children's | 62.9% |
| the weight splits | 32.3% |
| of which the split's Bellman–Ford | 9.8% |
| of which the bounds without each member | 9.0% |
| of which the tableaux made again for them | 9.1% |
| of which the columns of `A` | 5.4% |
| the searches for forced members | 23.0% |
| of which those that prove a member forced, 11.0 a step | 8.3% |
| of which those that find a cycle, 4.8 a step | 14.2% |
| what the sets of other sizes lose (`Losses`) | 5.0% |
| strong branching's bounds without 3.4 members of a conflict | 7.8% |
| the copy of the tableau a node keeps for itself (`keep_certified`) | 3.2% |
| the children's warm starts | 7.7% |
| the child's promotion, a peel of two frames | 5.5% |

Two findings decided the change:

* *The split's bounds are a pass over every circuit, and little of it is read.*  The pass costs 205,000 cycles a split.
  Its bounds force about 6 members an evaluation, which the searches would find forced anyway (below), and they order
  the 3.4 members of a conflict that strong branching tries.
* *The searches come from the level a node keeps for itself.*  That level is the node without the member strong
  branching chose, the one whose loss lowers the bound most, so its slack is the least.  Its next check costs 2.1 M
  cycles, 38.7% of a step, of which 12.4 searches a step prove a member forced and 3.8 find a cycle (21% of a step).  A
  warm child takes 3.0 and 0.9 a step; an inherited child almost none, since its parent's cycles hold.

**The change.**  Four parts, each leaving every result the same:

1. *The split's bounds where they are read.*  With the window `all` of `force_window` (the default), the searches find
   every member forced that a split bound shows forced: a cycle through a member leaves it by one arc and comes back
   by another, which together are at least what the bound takes off without it.  So a split finds its potentials
   alone (`Meet::split_from`).  Strong branching finds the bound of each member it orders one at a time, from the
   member's column of `A` and its block of `B` (`Meet::split_bound_without`, 5,700 cycles).  All of them are found
   where the searches cannot run: at 248 of 97,220 evaluations, where the other sizes do not lose enough.
2. *The searches bounded by each member's cheapest arcs.*  Where four or more members are left to search, one pass
   over the circuits of `B` finds the least arc into each member (`Meet::closing_lengths`, 65,000 cycles), and a
   member's column the least arc out of it (`Meet::opening_length`).  A member whose two arcs alone are longer than
   the slack is forced with no search: 4.5 a step, the members the split's bounds forced before.  The search for each
   other member offers no vertex a distance from which no cycle within the slack can close.  Those vertices come after
   every vertex from which such a cycle closes, so the search finds the cycle it finds without the bound.  A search
   that proves a member forced falls from 13,600 cycles to 7,400.
3. *The flip by circuits.*  `Meet::without` checks that `I*` flipped along its cycle is common independent, by
   inserting the members of each block the cycle touches in a span.  Where every element that comes in is in the span
   of `I*`, as in 86% of those blocks, its circuit decides.  An element in the span is the sum of its circuit, so the
   elements that come in are independent of the members that stay exactly when their circuits, cut to the slots of the
   members that leave, are independent: a few rows of four words.  Debug builds check every answer against the span.
4. *Warm children's splits without bounds,* which the children never read.

Besides, the pass that finds every bound reads the tableau's circuits in every block.  It used to build a span of the
members of positive weight where a member weighs 0, against a case that cannot arise: the split gives every member of
an element's circuit at least the element's weight in the matroid (an arc of the exchange graph), so a positive
element's circuit holds no member of weight 0.  Alone, this took 1.2% off a step, with the same trees.

**Results.**  In the scratch build a step fell from 1.93 M cycles to 1.79 M with the first, third and fourth parts,
and to 1.72 M with all four.  Paired, the final build against 88b1228:

| units | runs a unit | total CPU | geometric mean of the units' ratios | the units' ratios | steps |
|---|---|---|---|---|---|
| the 24 above | 2, the hyperthreads swapped | 0.868 | 0.869 ± 0.4% | 0.855 to 0.889 | the same at every unit |
| 24 others, gaps 500 to 1,300 | 1 | 0.867 | 0.889 ± 1.4% | 0.849 to 0.967 | the same at every unit |

The two runs of a unit agree within 3%; the smallest units gain least.  The whole game 3000 keeps its 117,197 steps
under the default tactics and its 161,460 under the chain's, with every counter of `tune`, at 0.871 and 0.842 of the
CPU (unpaired).

**Tried and not kept.**  The copy of the tableau a node keeps for itself, made by copying the workspace's arrays whole
instead of element by element: 1.79 M cycles a step either way, the whole copies as slow as the gather.

**What is left.**  After the change, a step of these units spends 16% in the searches that find a cycle, 4.8 a step
at 57,000 cycles each, three quarters of them at the levels nodes keep for themselves; the bound by the arcs into a
member does not shorten them.  A search from both ends, with the circuits of `B` listed by member, or cycles for the
kept level made from the node's own, are the next things to try.  The split's Bellman–Ford (10%) and the tableaux
made again (10%) come next.

**Decision.**  Kept, as the default: every tree is the same, so it is no tactic.

## Files

The scripts and their output are in the lab's work folder on the machine of the chain, not copied here.
`~/taxman_work/lab/work/rootgap/` has `gapfit.py` and the 35 ladder runs, and its `top/` has the scripts of the
top-of-tree runs.  `~/taxman_work/lab/work/hardgames/` has the study's scripts (`units.sh`, `solve-bands.sh`,
`bands.py`, `drops.py`, `spine.py`, `deepdrops.py`, `fanout.py`), its runs (`units/`, `ckk/`, `solves/`, `solves2/`,
`solves-all/`, `deep/`, `prof/`) and, in `gate/`, the gate experiment (`run.sh`, `analyze.py`, `analysis.txt`, `out/`
with a file a solve) and the finder run (`finder-4950.jsonl`); `cheapchoice/` has the cheap-choice runs (`run.sh`, `phases2.sh`,
`analyze.py`, `phase_table.py`, `analysis.txt`, `out/` and `ph/` with a file a solve).
`postmortem/` has the postmortem runs (Q5): the logging patch (`logging.patch`, against master at 3407bd4), `build.sh`,
`run.sh`, `check.sh` (steps with and without the log), `compare.py` (steps against the gate runs), `analyze.py` to
`analyze4.py` with their output (`analysis*.txt`), `out/` with a result file a solve and `pm/` with the choice log of
each.
`reuse/` has the price-reuse runs (Q6): the runners (`run.sh`, `irun.sh` with the config files `cfg*.txt`, `final.sh`),
`analyze.py` and `counts.py`, the id lists, the builds of `tune` of each stage, and `out/` with a result file a solve.
`~/taxman_work/lab/work/bound-2026-10-27/` has the runs of the GF(2) bound at a node (2026-10-27): in `tools/` the
paired runner (`pair.sh`) and its summary (`psumm.py`), the unpaired runner (`ab.sh`, `summ.py`), the counters' summary
(`profsum.py`) and the unit picker (`pick.py`); the counters as patches against 93e486b, each with its module
`gf2::prof` (`prof.patch` with `prof.rs`, and the finer `prof2.patch` with `prof2.rs`); the unit lists (`set24.txt`,
`set6.txt`, `cand40*.txt`); and a file a solve in `probe*/` (the units timed), `prof1/` and `prof2/` (the profiles),
`ab1/` (unpaired), `pair1/` (the lists alone), `pair2/` (the change), `pair3/` (the change, conflict branching off),
`prof3/` (the profile with conflict branching off) and `ident/` (game 3000).
`~/taxman_work/lab/work/patch/sweep/` has the settings sweep of game 4984 (2026-10-27) on the units of
`~/taxman_work/lab/work/patch/units-4984.jsonl`: the runner (`run.sh SETFILE SAMPLEFILE OUTDIR NODELIMIT`, resumable),
the settings (`settings.txt` for run 1, `settings2.txt` and `settings3.txt` for runs 2 and 3, `settings4.txt` for the
check of conflict branching off against a gate of 2,200), the unit ids (`sample1.txt` to `sample4.txt`, and
`sample3b.txt`), a file a solve in `out1/` to `out4/` (a directory a run), the screening (`screen.sh`, `screen.json` and
`screen/`) and the summaries (`analyze.py`, `summ.py`).
`~/taxman_work/lab/work/carry/` has the carried store's runs (Q7): the logging patch (`probe.patch`), the builds of
`tune` (`bin/`), the id lists (`units/`), and a folder a stage: `s1/` the logging runs (the nogoods learned, `dump-*.txt`,
the stores `store-4983-48.txt` and `store-4983-144.txt`, and `summ.py`), `s2/` the first measurement (settings one after
another), `s3/` and `s4/` the measurements with conflict branching on and off, `s5/` the variant that spares the
heaviest set, and `s6/` the second set of units; each with its runner (`run.sh`), `analyze.py`, `perstep.py` and a
result file a solve.
`~/taxman_work/lab/work/warm-2026-10-27/` has the runs of the warm start: in `tools/` the paired runner (`pair.sh`), its
summary for trees that may differ (`wsumm.py`), the runner of one build (`solo.sh`), the counters' module and summaries
(`prof.rs`, `profsum.py`, `profsum2.py`) and game 3000's runs (`ident.sh`); the counting build's patch
(`wip-with-prof.patch`) and source (`prof-src/`); and a file a solve in `prof0/` (where the solves come from), `prof2w/`
(the solves left by the first build), `pair1-v1/` (the first build), `pair2/` (the final one), `pair3-cbon/`
(conflict branching on) and `ident/` (game 3000).
`~/taxman_work/lab/work/step-2026-10-27/` has the runs of the cheaper search for forced members: in `tools/` the runner
of one build (`solo.sh`), the paired runner with the chain's tactics and store (`pair.sh`), the summaries (`profsum.py`
for the counters, `wsumm.py` for the pairs), the unit picker (`pick.py`), game 3000's runs (`ident.sh`) and the edits of
the scratch builds (`edit_*.py`); the scratch build's counters and switches as a patch against 88b1228
(`prof-switches.patch`, with its module `prof.rs`); the unit lists (`set24.txt`, `setB.txt`); and a file a solve in
`base0/` (the old build), `prof1/` to `prof6/` (the profiles, with each switch), `pair1/` and `pair2/` (the paired runs)
and `ident/` (game 3000).
