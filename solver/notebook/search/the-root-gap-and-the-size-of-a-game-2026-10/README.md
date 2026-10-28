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

## 2026-10-27: an A* search for forced members

**Question.**  After the cheaper search for forced members, where does a step of game 4984 spend its CPU, and what cuts
the next 7% with the same trees?  **Answer.**  The searches for forced members still took the most, and what they cost
is the vertices they take, not the arcs they read.  An A* search for the shortest cycle within the slack, searches that
leave out the elements free in B that cannot lower the hub, and two smaller changes take 24 units of 4984 to 0.829 of
the CPU of 947946a, with every tree and every counter the same.

**Method.**  The 24 units of the sections above under the chain's settings: conflict branching off, price reuse, the
warm start, and the store of 144 units of 4983, 127,434 steps in all.  A scratch build counted, by the kind of level,
the searches for forced members: how many there were, what they found, and the vertices and arcs they took.  It also
timed the splits and the tableaux made again.  Switches in the same binary turned each idea on, and every unit took the
same steps, best and counters of `tune` under every switch.  Then the old build (947946a) and the final one solved each
unit at once on the two hyperthreads of one core, as in the sections above.

**Where the time goes.**  At 1.80 M cycles a step (the CPU time at 2.2 GHz):

| part | share of a step |
|---|---|
| the levels nodes keep for themselves: their searches for forced members, losses, kept cycles and arcs | 22.9% |
| of which the searches that find a cycle, 12.4 an evaluation at 59 k cycles each | 12.6% |
| of which the searches that find none, 28.3 an evaluation at 7.4 k cycles each | 3.6% |
| the same for warm children | 9.7% |
| the weight splits of the kept levels, inherited children and warm children | 24.6% |
| the loss of the smaller sets (`Meet::smaller_loss`) | 3.6% |

Three findings decided the change:

* *A search costs the vertices it takes, about 450 cycles each.*  Reading only the arcs within the slack (a member's
  column filtered and sorted once an evaluation, an element's circuit in B masked) cut what a search reads from 375
  column entries and 127 circuit bits to 79 and 16, and a search that finds a cycle cost 58.1 k cycles against 59.1 k.
  Nor is the cost the memory: a search made twice in a row was 6% faster the second time.  Nor is it the queue: a
  binary heap of the same keys cost the same, and a sorted vector more.
* *Half the vertices taken are elements free in B.*  A search that finds a cycle took 131 entries off its queue, and 62
  of them were elements free in B.  Such an element reaches the members only through the hub, the least distance less
  `w2` of such elements taken, and most could not lower it.
* *The first cycle found is long.*  A search stopped at the first cycle within the slack, 185 long on average against a
  slack of 211, and such cycles serve few of the levels after.  A search for the shortest (141 long) freed 8.4 more
  members an evaluation on the cycles kept and left 4.3 searches that find a cycle instead of 12.4, but each cost 2.4
  times as much, and the CPU fell by 1% only.

**The change.**  Four parts, each leaving every result the same:

1. *An A* search for the shortest cycle.*  `Meet::has_cycle_within` takes the vertices in the order of their distance
   from the member plus a lower bound on the length of the path back to it.  The bound of an element outside the set
   that a member's column offers is its least arc out: to a member of its circuit in B, or for one free in B to the
   member of least `w2`.  It is found when the element is first offered on the tableau, and kept for the other searches
   there.  Members and elements free in A, which come through the hubs in the order of `w2` and `w1`, take 0.  Each
   bound is at most every arc out of its vertex plus the bound of the arc's head, so the keys taken never decrease, and
   the search finds the shortest cycle: it stops at the first key at or past the shortest cycle found, or past the
   slack.  The forced members are the same, since a member is forced exactly when no cycle within the slack passes
   through it, and the cycles kept are the shortest.  Any shortest cycle will do, so the distances leave out the count
   of arcs that the other searches pack with them, and the queue, their radix heap, takes keys that tie in distance
   with the last one taken: it needs only that no key falls in distance.
2. *The elements free in B that cannot lower the hub are not offered*, in the A* search and in the search for the bound
   without a member (`Meet::without`, `Meet::without_any_size`).  Such an element has arcs to the members alone, which
   the hub reaches no later, so no distance, predecessor or cycle changes; taking it offered nothing, so the order of
   the offers is kept too.  The search for the loss of the smaller sets offers none of them, since none lowers anything.
3. *The copy of a tableau without its residues.*  The copy that a node keeps for itself (`Snapshot`) keeps each
   element's circuit and whether it is free, since a derived tableau reads its residues only for being zero.  A copy
   costs 86 k cycles, against 152 k in the profile of the section above, and the tableau derived from it 313 k against
   355 k.
4. *The elements that enter a derived tableau listed once* (`Meet::derive`), not sought in every block: 153 k cycles a
   tableau derived, against 167 k.

**Results.**  The scratch build, one build at a time, under the chain's settings (CPU of the 24 units, solved four at
once on cores of their own; the A* variants differ by less than the noise):

| scratch build | CPU |
|---|---|
| 947946a, with the counters | 104.1 s |
| with parts 3 and 4 | 103.4 s |
| and part 2 | 98.1 s |
| and the shortest cycle by the old search | 95.0 s |
| with A* whose bounds of every element are found at each evaluation that searches | 84.3 s |
| with A* whose bounds are found when first offered (the final change) | 83.8 s |

Paired, the final build against 947946a:

| units | runs a unit | total CPU | geometric mean of the units' ratios | the units' ratios | steps |
|---|---|---|---|---|---|
| the 24 above | 2, the hyperthreads swapped | 0.829 | 0.826 ± 1.5% | 0.764 to 0.874 | the same at every unit |
| 24 others, gaps 500 to 1,300 | 1 | 0.828 | 0.853 ± 2.6% | 0.730 to 0.983 | the same at every unit |

Every unit of the first set also kept every counter of `tune`, and the build committed, whose code differs only in
its checks and comments, took 0.830 of the CPU in one more run.  The whole game 3000 keeps its 117,197 steps under the
default tactics and its 161,460 under the chain's, with every counter of `tune`, at 0.883 and 0.847 of the CPU
(unpaired).

**Tried and not kept.**

* *Only the arcs within the slack read* (above): the same cost.
* *A binary heap or a sorted vector for the searches' queue*: the same cost, or more.
* *The columns of A in one flat array by member*, counted and then filled: 230 k cycles a build against 122 k.
* *The search for the loss of the smaller sets stopped once every path left reaches what it must certify*: it stopped
  early at 98.5% of its calls, but cost 601 k cycles against 640 k.
* *The bounds of every element found at every evaluation*, even those that search nothing: 141 k cycles each, 7% of a
  step, most of it at inherited children, which search 0.3 members.
* *The first cycle found by A*, not the shortest*: the same cost.
* *Distances packed with the count of arcs in the A* search*, as in the other searches, so that no key ties below the
  last one taken: 1.064 of the final build's CPU on the 24 units (paired, one run).

**What is left.**  In the scratch build with the final change, at 1.45 M cycles a step, the weight splits take 30%:
Bellman–Ford 13%, the tableaux derived 9%, the columns of A 7% and the copies restored 5%.  The searches for forced
members take 9%, and the loss of the smaller sets 3.5%.

**Decision.**  Kept, as the default: every tree is the same, so it is no tactic.

## 2026-10-27: settings checks on games 4991, 4992 and 4995

**Decision.**  Conflict branching on at a gate of 1200, so only at the nodes whose gap is above 1200, against the
chain's setting of conflict branching off.  The lab channel applied it to the searches of 4991 and 4992 as they ran, and
then as a policy from game 4995 on.  In each of the three games it took 0.90 to 0.92 of the chain's CPU on units of the
game.  On 4984 the sweep had found conflict branching off best
([above](#2026-10-27-a-sweep-of-the-settings-on-game-4984)); on the later builds, with the carried store, the gate won
three games in a row.

**Method.**  For each game, units cut from its live checkpoint, each solved to the end or to 150,000 steps on one thread
by `tune solve` at `nice -n 19`, the settings interleaved per unit: 108 units of 4991, and 60 each of 4992 and 4995, of
gaps from 300 to about 2,100.  The baseline B is the chain's setting: conflict branching off, price reuse, the warm
start, and the store of the last game searched before (4984 for 4991, 4991 for 4992, 4992 for 4995).  Ratios are
against B on the same units.

**Results.**  Steps and CPU relative to B:

| game | units | gate 1200: steps, CPU | the other settings: steps, CPU |
|---|---|---|---|
| 4991 | 108 | 0.94, 0.92 | gate 300: 0.98, 2.03; no store: 1.10, 1.02 |
| 4992 | 60 | 0.98, 0.90 | gate 2400: 1.00 of the steps; it never engages, since no unit's gap reaches 2,400 |
| 4995 | 60 | 0.96, 0.91 | gate 800: 1.03, 0.99 |

* In each game, gate 1200 changes nothing below a gap of 1200: every unit there takes B's steps.
* Above it, a step costs less.  On the units that both settings capped, gate 1200 took 0.93, 0.88 and 0.94 of B's CPU
  at equal steps.
* The gain in steps rests on a few units.  In 4992 it rests on one, 4992-1098 (59,185 steps to 24,393), without which
  gate 1200 takes 1.004 of B's steps.  In 4995 it rests on three, 4995-640, 4995-6410 and 4995-6591, without which it
  takes 1.017.  In 4991, 4991-7103 went from 93 k steps to 40 k and 4991-6006 from 30 k to 14 k, while 4991-934 rose
  from 66 k to 79 k.
* The lower gates lose.  Gate 300 took fewer steps on 4991 but cost 2.03 of the CPU, and gate 800 took 1.22 of B's steps
  on the 11 units of gap 1200 and more that both finished in 4995.
* The carried store helps: without it, 4991's units took 1.10 of the steps.
* No unit's best differs between the settings.  The controls, gate 2400 on 4992 and gate 1200 below its gate on 4995,
  took 0.97 to 1.02 of B's CPU on the same trees, so the CPU ratios are good to a few percent.

## 2026-10-27: splits started from what changed, and tableaux derived with less

**Question.**  After the A* search, the weight splits and the tableaux made again took 30% of a step of 4984.  Can
they be made cheaper with the same trees, by 7% of the CPU?  **Answer.**  By 7% under the chain's setting from game
4995 on (conflict branching on at gate 1,200) and by 6% with conflict branching off: a split started from what the
derivation of its tableau marked changed, columns of A and tableaux derived with less, the gap carried down as a floor,
and no residues in derived tableaux.  Every tree and counter is the same.  The counters of the scratch builds overstated
what each change would save, two to three times: on this machine a step waits on memory, and the cost the counters give
one part moves to the others when it goes.

**Method.**  The 24 units of the sections above, under the chain's settings of 2026-10-27 (conflict branching off,
price reuse, the warm start, the store of 144 units of 4983), 127,434 steps in all.  Scratch builds with cycle counters
by part of a step and by kind of level (`prof1` on 29aa8ba, `prof2` and `prof3` on the changes), each change then
paired against the old build as in the sections above: the two builds solve each unit at once on the two hyperthreads
of one core, two runs with the hyperthreads swapped.  A core's second-level cache, 256 KB, is shared by its two
hyperthreads and holds less than the workspace, the levels' vectors and the copies of a step.

**Where the time goes.**  At 29aa8ba, 1.45 M cycles a step (counters; the parts overlap):

| part | share of a step |
|---|---|
| the levels nodes keep for themselves: their checks | 29.5% |
| of which the weight split (the tableau restored and derived 6.2%, the columns 3.5%, Bellman–Ford 6.6%) | 16.7% |
| of which the searches for forced members and the losses | 12.5% |
| the children that keep their parent's heaviest set: their checks | 14.1% |
| of which the weight split (the tableau derived 3.2%, the columns 3.9%, Bellman–Ford 3.0%) | 10.5% |
| the warm children: their checks, and their warm starts | 11.8%, 8.1% |
| the child's vectors copied and contracted | 6.6% |
| the child made (the promotion, the nogoods, the symmetry) | 11.3% |
| strong branching | 12.2% |

Three findings decided the change:

* *Every split is read.*  Each of the levels' 99,079 splits was used by the searches, strong branching or a warm start,
  so none can be left until asked for.
* *The start of a split costs more than its rounds.*  Of the 121 k cycles of a split at a child that keeps its parent's
  heaviest set, the walk that checks which of the start's potentials still hold (every vertex's path of predecessors)
  took 89 k and the rounds 1.6 k.  At a level a node keeps for itself, the walk took 129 k and the rounds 166 k: the
  flip along the certified cycle adds arcs, so the first round relaxed every vertex.
* *The columns of A are built again at every split*, 120 to 160 k cycles, though a child's are its parent's less a few
  elements.

**The change.**  Seven parts, each leaving every result the same:

1. *A split from what changed* (`Meet::repair_from`).  `Meet::derive` marks, in each matroid, the elements whose
   circuit, freedom or side it changes, and keeps the tableaux the workspace's derives from.  A split found on one of
   those starts the next: only an arc into an element changed in A, or out of one changed in B, can be new, so the
   paths of predecessors that break are found from the changed elements and the ones no longer live, down the forest
   of the start's predecessors (a counting sort, made when first asked for).  Only the vertices whose paths broke and
   the changed elements are pulled along their arcs in and relaxed in the first round.  Bellman–Ford reaches the same
   split from any potentials that are lengths of paths, and debug builds check every split against the one from
   scratch.  Bellman–Ford's predecessors form a forest, since a cycle of them would be a negative cycle (Cormen et al.,
   Lemma 24.16), and debug builds check that too.
2. *A warm child's splits on the way to its set* start from its parent's split, and each later one from the one before.
3. *Columns of A derived* (`Meet::derive_columns`).  Where the workspace has the source tableau's columns, a derived
   tableau drops the elements no longer live from them and empties the columns of the members gone.  Where elements
   changed, it also drops the changed ones, empties the column of each member that came in, and pushes each changed
   element into the columns of its new circuit.  Debug builds check them against columns built.
4. *A derivation by contraction* (`Meet::derive_by_contraction`): a child that keeps its parent's heaviest set loses
   only the promoted members and the elements it dropped, so its tableau is made with no marks, exchanges or reductions.
5. *The gap as a floor.*  What the larger sets lose is carried down as what the smaller sets lose is: a child that keeps
   its parent's heaviest set has the parent's gap as its floor (its larger sets, with the promoted, are the parent's),
   and a node's level without a member the node's gap less the cycle's length.  The gap is found only where the floor
   does not decide.
6. *No residues in derived tableaux.*  Outside a solve a residue is read only for being zero, which the flag of a free
   element already says, so a derived or restored tableau writes only the flags.
7. *The vectors of the level a node keeps for itself* are the node's, which it takes when it becomes the level, with no
   copy.

**Results.**  The build committed paired against 29aa8ba, one run a unit; every unit took the same steps and best:

| units, setting | total CPU | geometric mean of the units' ratios | the units' ratios |
|---|---|---|---|
| the 24 above, conflict branching on at gate 1,200 (from 4995) | 0.927 | 0.930 ± 0.6% | 0.910 to 0.968 |
| the 24 above, conflict branching off | 0.941 | 0.930 ± 0.7% | 0.907 to 0.963 |
| 24 others (gaps 500 to 1,300), conflict branching on at gate 1,200 | 0.932 | 0.944 ± 0.8% | 0.917 to 0.984 |
| 24 others, conflict branching off | 0.930 | 0.936 ± 0.7% | 0.915 to 0.978 |

Game 3000 keeps every counter of `tune` under the default tactics and under the chain's settings.  Before the fault
below was fixed, two runs a unit on the 24 units gave 0.930 with conflict branching on and 0.944 with it off.  On the
way, with conflict branching off: the first part with the second (the split from what changed) took 0.986 of the CPU;
with the third, the fifth and one pass fewer of the derivation, 0.960.  Against that build, the seventh with columns
kept in a tableau's copy took 1.004, and the fourth with both 0.995; the final build leaves the columns out of the
copies and adds the sixth.

**A fault the tests found.**  The first version of the derived columns emptied only the columns of the slots no member
held.  Where a block's span changes, a member that comes in takes the first free slot, which can be one that a member
contracted in the same derivation left (a warm child's derivation both contracts and flips).  That column kept the
elements of the old member's circuit, whose circuits had lost it, so the columns held arcs the exchange graph does not
have.  The release runs had not shown it, every tree the same on 48 units and game 3000; the check of debug builds
did, in the quick check and at game 1000's root under the chain's settings.  The fix empties the column of every member
that came in; debug builds of `tune` then solved the roots of 1000 to 1700 under the chain's settings, the default
tactics and conflict branching with the warm start, every check passed.  The same quick check found the check itself
built its columns from the lists of the tableau before (it ran before the derivation remade them), that a restored
tableau was still marked as keeping its residues, and that the check of a free element's flag against its residue
read elements no longer live, whose residues a solve leaves stale, where a walk of predecessors asks about them (its
answer for such an element is not used).  The checks are fixed, the columns compared as sets, since their order
decides only which of equal offers a search takes first; the flags of the live elements were right.

**Tried and not kept.**

* *Columns of A as bits*, the circuits' masks transposed 64 by 64: 1.062 of the CPU.  A column read became a scan of
  its words, and the transposition alone costs 57 k cycles against 72 k for the lists (a microbenchmark, caches warm).
* *The first version of the split from what changed*, its forest found with a search for vertices that no root
  reaches, its marks in one array for both matroids: 1.004 of the CPU of the build without it.  The forest cost 130 k
  cycles a split; the counting sort that replaced it, 16 k in a microbenchmark and 53 k in the search.
* *The contraction of a child's members in one pass a block*: its share of a step stayed at 4.7%.
* *A lower bound on what the smaller sets lose* (a member alone, or a member's least arc out and another's least arc
  in) certified 411 of the 12,544 searches for it at warm children.
* *Columns kept in a tableau's copy*, for the level a node keeps for itself: 1.004 of the CPU, with the seventh part.
* *The path's vectors kept once*, the top level's, contracted in place with a log of what changed and put back at a
  pop, instead of a copy a child: 1.014 of the CPU of the final build.

**What is left.**  In the scratch build `prof3` (1.35 M cycles a step), the levels nodes keep for themselves take 28.8%,
their searches for forced members 9.1% and their tableaux restored and derived 7.4%; the GF(2) levels pushed 16.2%; the
child made 12.1%; and the searches of the exchange graph 28% together (forced members 11.3%, strong branching 8.3%,
warm starts 5.0%, the smaller sets' loss 3.6%).

**Decision.**  Kept, as the default: every tree is the same, so it is no tactic.

## 2026-10-28: what the store costs a step

**Question.**  On units of game 4995 a step with the store of 20,000 nogoods cost 18% more than one without a store, and
26% more with 50,545 nogoods, so that the larger store's 5% fewer steps saved no CPU.  Where does that cost go, and can
the store's own work be made nearly free with the same trees, for 7% of the CPU or more?  **Answer.**  Most of it is not
the store's work but the trees it makes.  With the trees held the same, the store's questions cost about 4.5% of the CPU
with 20,000 nogoods and about 10% with 50,545.  The rest comes from what the store prunes: it removes cheap steps, so the
steps left check more bounds, and its children lose members of their parent's heaviest set, which the GF(2) bound then
deletes.  Keeping each nogood as the set of its members' ranks takes the CPU to 0.991 of the old build's with 20,000
nogoods and 0.974 with 50,545, with the same trees: under the bar, and not kept.  With it, the store of 50,545 still
saves no CPU over the store of 20,000, so the most a search reads stays at 20,000 nogoods.

**Method.**  The units of the sweep of game 4995 (`units-4995.jsonl`, the 60 of `sample.txt`), under the chain's
settings: conflict branching on, a gate of 1,200, price reuse, the warm start, and the stores of the sweep.  The store of
20,000 nogoods holds 333 numbers, the one of 50,545 holds 366, and a nogood has 38 members on average.

1. *Where a step's cycles go.*  A scratch build counted the cycles of the parts of a step (the processor's time-stamp
   counter) on 8 of the sweep's units of 39,000 steps or more, stopped at 30,000 steps each, each solved alone on one
   thread.
2. *The store's questions with the trees held.*  A switch of the same build asks the store its questions and drops the
   answers, so that the trees are those without a store.  It was paired against the same build without a store on the
   21 units of the sweep of 5,000 steps or more, at most 60,000 steps each: both at once on the two hyperthreads of one
   core, as in the sections above.
3. *The change tried.*  The old build (29aa8ba) and the new one, paired the same way on the same 21 units: two runs
   with the store of 20,000, the hyperthreads swapped, and one with the store of 50,545.  Every unit took the same steps
   and best, and the store ruled out the same candidates and ended the same nodes.

**Where a step's cycles go.**  Cycles a step on the 8 units, by the old build, and the parts' calls a step:

| part | no store | 20,000 nogoods | 50,545 nogoods |
|---|---|---|---|
| a step | 996 k | 1,241 k | 1,388 k |
| the store's questions (a child's rule-outs, a check's cut) | 0 | 57 k | 144 k |
| the GF(2) bound's evaluations | 545 k (0.79 a step) | 618 k (0.89) | 639 k (0.89) |
| the GF(2) levels of children | 121 k (0.49 a step, 245 k each) | 200 k (0.58, 345 k each) | 220 k (0.58, 377 k each) |
| the choice of the branch | 121 k | 144 k | 156 k |
| the making of a child, less the store's questions | 158 k | 171 k | 177 k |

Three findings:

* *The store prunes cheap steps.*  Without a store 0.13 of the steps make a child that cannot be promoted, and end there;
  with the store 0.02 do, since it rules out the candidates whose promotion would complete a nogood.  So the steps left
  check more bounds: 0.58 children checked a step against 0.49, and 0.89 GF(2) evaluations against 0.79.  This is no
  cost of the store: on the units every setting completes, the store of 20,000 takes 0.65 of the steps and 0.77 of the
  CPU of no store (the sweep's `analysis.txt`).
* *A child's GF(2) level costs 41% more with the store* (54% more with 50,545 nogoods).  The candidates the store rules
  out of a child are often members of its parent's heaviest set, which the warm start then deletes one at a time.  This
  is 4.7% of a step with 20,000 nogoods, in the GF(2) bound, and the store's answers decide it.
* *The store's own questions are 4.6% of a step with 20,000 nogoods and 10.4% with 50,545*, and grow with the store.  A
  child looked at 124 nogoods with 20,000 and 295 with 50,545 (those that watch what it promotes, 7.9 members a child),
  at 650 cycles each, and moved the watch of nearly all of them; the look scanned 5.8 members of the nogood, from the
  rarest, for one not promoted, in a list of all the store's members.  The check of forced members looked at 125 and 261
  nogoods, at 110 cycles each, 0.8% and 1.5% of a step.  The store's memory hardly slows the rest of a step: in the
  dry run of the new build (method 2), alone on the 8 units, the questions cost what the counters say, and the rest of a
  step 0.2% more than without a store with 20,000 nogoods and 1.6% more with 50,545.

**The change tried.**  The members of a store's nogoods are a few hundred numbers of the hard core, so the store ranks
them from the rarest (in the fewest nogoods, then the smallest) and keeps each nogood as the set of its members' ranks,
six words with 333 numbers.  The member a nogood watches next is the lowest rank of the nogood less the child's
promotions (made a set of ranks once a child) and the member it watches besides, a few word operations and one read of
memory.  The watches are the same as before, member for member, so every answer is too.

**Results.**  CPU of the new build over the old, paired, with every tree the same:

| measure | 20,000 nogoods | 50,545 nogoods |
|---|---|---|
| the store's questions with the trees held (dry run over no store, new build) | 1.036 | 1.075 |
| the new build over the old (21 units) | 0.991 (geometric mean 0.992, unit ratios 0.984 to 1.001, two runs) | 0.974 (geometric mean 0.978, unit ratios 0.959 to 1.006, one run) |

In the scratch build a nogood looked at costs 390 cycles against 650 (360 against 710 with 50,545 nogoods), and the
questions 3.2% of a step against 4.6% (6.3% against 10.4%).  Also tried, on the change:

* *A watch on a member that is no candidate*, preferred to the rarest: no node below the child promotes it, so the
  nogood is not looked at again there.  A child looked at 101 nogoods against 124, and the cycles were the same (28.6 k a
  step against 27.9 k), presumably since a member that is no candidate of a deep child is often one of its parent's,
  which a sibling then promotes.
* *The watched ranks in 16 bits*, halving the watches' memory: 0.999 of the new build's CPU (one run).  With the nogoods
  also ordered by their ranks, so that those that watch the same members lie together: 0.992 (one run, unit ratios
  0.987 to 1.004), left out as too small for the code it takes.

**The size of the store.**  With the new build, the store of 50,545 against the store of 20,000 on the 60 units,
paired, at most 150,000 steps each: 0.946 of the steps and 0.994 of the CPU (geometric mean 1.033: the small units pay
for the larger store's start).  So a larger store still buys no CPU, and the most a search reads (`DIR_CAP`) stays at
20,000 nogoods.

**Decision.**  Not kept: 0.991 of the CPU with the store of 20,000 nogoods and 0.974 with 50,545, under the bar of 7%,
and a larger store still does not pay with it.  No change to the store's questions can reach the bar with 20,000
nogoods, since they cost 4.5% in all.  The largest cost the store causes that is left is the GF(2) levels of the
children it prunes, 4.7% of a step; the rest of what a step costs more is the pruning itself, which pays.  The most a
search reads (`DIR_CAP`) stays at 20,000 nogoods.

## 2026-10-28: warm starts stopped at the need, and A* for the bound's other searches

**Question.**  After the splits started from what changed, the GF(2) bound still takes most of a step of game 4984.  Can
another 7% of the CPU be cut under the chain's settings, with the same trees?  **Answer.**  Yes: 0.918 of the CPU of
7140548 on the 24 units of the sections above (geometric mean 0.929), 0.915 on 24 others and 0.913 on 21 units of 4995,
with every tree the same.  Most of it comes from warm children, most of which their own bound cuts: their deletions
stop at the first set below the need, and the first is made before the child's vectors are.  Strong branching
finds its bounds' lengths by the A* search of the forced members, and the smaller sets' loss and the gap are found by
A* too; a gap of no larger set at all, which every level with a child had, is carried down as a floor.

**Method.**  The 24 units of the sections above under the chain's settings since game 4995: conflict branching on at a
gate of 1,200, price reuse, the warm start, and the store of 144 units of 4983 (106,567 steps in all).  Three scratch
builds counted the cycles of a step's parts (round 3's counters, with those of the parts outside the bound), what the
children checked and the warm starts came to, and how the gap was found.  Each change was then paired against the build
before it, the two builds solving each unit at once on the two hyperthreads of one core, as in the sections above, and
three in one build were each paired against the build with it switched off.  These builds, like those before, are made
without `target-cpu=native`, which the chain's builds use.

**Where a step goes.**  At 1.24 M cycles a step (the counters overlap):

| part | share of a step |
|---|---|
| the levels nodes keep for themselves: their checks | 28.4% |
| children pushed: vectors copied (2.2%) and contracted (5.1%), warm starts (9.7%) | 17.5% |
| children that keep their parent's heaviest set: their checks | 11.0% |
| warm children: their checks | 10.6% |
| strong branching's bounds without the members of a conflict | 8.6% |
| a child's promotion (a peel of each frame it touches; a child promotes 8.3 numbers on average) | 8.4% |
| the copy of the tableau a node keeps for itself | 2.7% |
| the gap, by Bellman–Ford, at 0.75 checks a step | 2.4% |

Three findings decided the change:

* *Most warm children are cut by their own bound.*  Of 24,702 warm children (0.23 a step), 15,875 had a bound below
  their need, and the first deletion showed it for 14,360 of them.  Of those, 2,360 had lost two members or more, and
  each deletion after the first derives a tableau, finds a split and searches (0.08 a step, about 620 k cycles each).
  Each child's vectors were copied and contracted before its first deletion, which does not read them.
* *The gap was found at every level checked.*  A floor carried down spares the search for the gap, but the floor was 0
  at all 79,550 checks that found it.  Where no common independent set is larger than the heaviest set the gap is
  `i64::MAX`, which the floor left out, and so was every gap of a level with a child.
* *Strong branching reads each member's cycle for its length alone*, but for the member whose level the node keeps.

The levels nodes keep for themselves end with 37.6 members forced an evaluation, 11.2 by their two arcs and 26.5 by a
search, against 4.0 searches that found a cycle and 75.9 members freed by cycles kept.

**The change.**  Six parts, each leaving every result the same:

1. *Warm starts stopped at the need.*  Each deletion of a warm start only lowers the heaviest set, and the search cuts a
   child whose bound is below its need.  So a child pushed at its need (`Gf2Bound::push_all_at`) stops its deletions at
   the first set below it, and `evaluate` returns that set's weight, an upper bound on the child's, which cuts the child
   as its bound would.  Debug builds solve each child cut so and check that its bound is no higher.
2. *The first deletion before the child is made* (`warm_first`), in the parent's problem, so that a child it cuts is
   never copied or contracted.  The loops of the contraction are not banned from its search, as they were once the
   child was made, and that changes nothing: a loop's circuit in the matroid where it is one lies within the promoted
   members, all banned, so its arcs there run only to or from banned members, and no cycle that avoids them passes
   through it.
3. *The gap of no larger set carried down as a floor*, as a finite gap is: a level below has no larger set either.
4. *Strong branching by the length of the shortest cycle.*  The A* search of the forced members finds each member's
   (`Meet::shortest_cycle_length`), and `Meet::without` finds the cycle of the member whose level the node keeps alone,
   when it keeps it: the shortest with the fewest arcs, as before, so the level is the same.  Debug builds check every
   length against `Meet::without`'s.
5. *The smaller sets' loss and the gap by A** over the split's reduced lengths, with the bounds of the search for
   forced members: the loss from the lightest member's weight (`Meet::smaller_loss`), the gap from the elements free in
   A (`Meet::gap_with`).  Debug builds check each against Dijkstra's search with no bounds and against Bellman–Ford's.
6. *Two small ones*: a derivation by contraction does not scan for marks it cannot have made, and `make_child` asks once
   whether the child promotes one number.

**Results.**  CPU of each build over the one named or the one before it, paired, every tree the same (the builds
between them had the branchless contraction and the deferred copy below):

| build | runs a unit | total CPU | geometric mean of the units' ratios | the units' ratios |
|---|---|---|---|---|
| part 1 with the split's bound before each warm start, over 7140548 | 2 | 0.971 | 0.973 ± 0.8% | 0.940 to 1.003 |
| parts 4 and 5 for the smaller sets' loss | 1 | 0.982 | 0.979 ± 2.0% | 0.787 to 1.005 |
| the gap by A* (a switch in one build) | 1 | 0.989 | | |
| the build committed, over 7140548 | 2 | 0.918 | 0.929 ± 1.1% | 0.884 to 0.986 |
| the same, on 24 other units of 4984 (gaps 500 to 1,300) | 1 | 0.915 | 0.940 ± 1.1% | 0.895 to 0.988 |
| the same, on 21 units of 4995 with its store of 20,000 nogoods, at most 20,000 steps each | 1 | 0.913 | 0.914 ± 0.6% | 0.888 to 0.943 |

Game 3000 keeps every counter of `tune` under the default tactics (117,197 steps), the chain's (161,460) and conflict
branching off (161,460), at 0.95 to 0.96 of the CPU (unpaired).  Debug builds of `tune` solved the roots of 1000 to
1700 under the chain's settings, with conflict branching off and with the default tactics, and six units of 4984 with
the store, every check passing.

**Tried and not kept.**

* *The split's bound without each member a warm child loses*, before its warm start: it cut 11 of the 24,702.
* *A branchless contraction of a child's vectors*, which rewrites every vector: 1.037 of the CPU (one run, a switch in
  one build).
* *The copy of the tableau a node keeps for itself made only when the workspace first changes*, so that none is made
  where a cut child leaves the workspace's tableau to the node: 0.997 of the CPU (one run, a switch), too little for
  its code.
* *A set width of 80 words*, for the boards 4096 to 5119 (4984 needs 78): 0.981 of the CPU (one run), with the same
  trees.  The chain's builds use `target-cpu=native`, whose popcount instruction makes the wide sets cheaper than in
  these builds, and a checkpoint made at another width is refused, so a search under way could not resume at it.

**Decision.**  Kept, as the default: every tree is the same, so it is no tactic.

## 2026-10-28: searches that stop where their checks do, and contractions a block at a time

**Question.**  After the warm starts stopped at the need, where does a step of game 4984 go under the chain's settings,
and can another 7% of the CPU be cut with the same trees?  **Answer.**  Yes: 0.916 of the CPU of daeade5 on the 24
units of the sections above, 0.911 on 24 others and 0.912 on 21 units of 4995, with every tree the same.  Two
searches went further than their checks need: a warm child's first deletion went on to the member's weight where only
the child's need mattered, and the loss of the smaller sets to the lightest member's weight where only the slack did.
And a child that promotes its node's forced members contracted its vectors one member at a time, a pass over a block
for each.

**Method.**  The 24 units of the sections above under the chain's settings since game 4995: conflict branching on at a
gate of 1,200, price reuse, the warm start, and the store of 144 units of 4983 (106,567 steps in all).  A scratch build
with round 4's counters, and those of a child's promotion and contraction split by how many numbers it promotes,
profiled the old build and the change.  Each build was then paired against the one before it, the two solving each unit
at once on the two hyperthreads of one core, as in the sections above.

**Where a step goes.**  At daeade5, 1.15 M cycles a step (the counters overlap):

| part | share of a step |
|---|---|
| the levels nodes keep for themselves: their checks | 30.6% |
| of which the tableau restored and derived, its columns and its split | 16.3% |
| of which the searches for forced members | 10.5% |
| warm children: their checks | 11.8% |
| of which what the smaller sets lose, 530 k cycles a search | 3.8% |
| children that keep their parent's heaviest set: their checks | 10.7% |
| strong branching: the cycles' lengths (4.9%), the kept member's cycle (2.7%) and the copy of the tableau (2.8%) | 10.7% |
| a child's promotion: 101 k cycles for one number, 282 k for several | 9.1% |
| a child's vectors copied (2.0%) and contracted (4.5%) | 6.5% |
| warm children's first deletions, whose searches take 244 k cycles each | 5.6% |

Three findings decided the change:

* *The first deletion of a cut child searched past its need.*  Of the 24,702 warm children, 14,349 (0.135 a step) are
  cut by their first deletion.  It finds the shortest cycle through the member in an exchange graph that admits every
  size, by Dijkstra's search with no bound but the member's weight, and so takes every vertex closer than that cycle; a
  cut child's cycle is longer than its slack, and the search went on to it.
* *The loss of the smaller sets was found exactly at every warm child checked.*  Their floor is 0, so the check of
  forced members searched for it each time, up to the lightest member's weight, where the check asks only whether it is
  more than the slack.
* *A child's contraction was a pass over a block for each member it promotes*, 8.6 members a push on average: 17 k
  cycles where it promotes one number (0.27 a step), about 250 k where it promotes several (0.19 a step, the forced
  members of a node).

**The change.**  Four parts, each leaving every result the same:

1. *A deletion asked for a set at least as heavy as the need* (`Meet::without_any_size_reaching`).  A warm child's
   deletions, its first in the parent's problem and the others in its own, search no cycle longer than the set's weight
   less the need.  Where none is that short, the child's heaviest set is below the need, the child is cut, and its
   bound is given as the need less 1, an upper bound on it that cuts it as its own would.  Where one is, the search
   takes the same vertices in the same order as without the need up to the cycle it closes, which is the same: offers
   at or past the bound are dropped, but none of them is taken before that cycle.
2. *The loss of the smaller sets within the slack* (`Meet::smaller_loss_within`).  The check of forced members asks
   whether the smaller sets lose more than the slack, so its search starts from the slack plus 1 where the lightest
   member weighs more, and takes no key from there on.  A loss found no shorter is kept as the level's floor, which
   is all it is; a later check that asks about a longer loss searches again.
3. *A child's contractions a block at a time* (`contract_all`).  The members a child promotes in each block become rows
   whose pivots, the lowest bit each has once reduced by those before it, no other row has, and each vector of the block
   is reduced by the rows of the pivots it has, in one pass.  One at a time, each contraction clears its pivot from
   every vector and sets none cleared before, so the vectors end with no pivot set, each its own plus a combination of
   the promoted vectors, which is unique.  The rows are reduced against each other, so an element finds the rows it
   needs in its own vector masked by the pivots, a few word operations, where the pass a block tried on 2026-10-27,
   which did not pay, tested each row in turn on each element.  Debug builds check every child's vectors and live set
   against the contractions one at a time.
4. *The kept member's cycle within its length.*  Strong branching knows the length of the cycle of the member whose
   level the node keeps (the A* search found it), so `Meet::without`, which finds that cycle again with the fewest arcs,
   searches no further.

**Results.**  CPU of each build over the one named, paired, every tree the same:

| build | units | runs a unit | total CPU | geometric mean of the units' ratios | the units' ratios |
|---|---|---|---|---|---|
| parts 1 to 3 over daeade5 | the 24 above | 2 | 0.922 | 0.923 ± 0.7% | 0.895 to 0.961 |
| the same | 24 others (gaps 500 to 1,300) | 1 | 0.915 | 0.928 ± 0.8% | 0.899 to 0.964 |
| the same | 21 units of 4995 with its store of 20,000 nogoods, at most 20,000 steps each | 1 | 0.921 | 0.921 ± 0.6% | 0.900 to 0.948 |
| the build committed over daeade5 | the 24 above | 2 | 0.916 | 0.917 ± 0.7% | 0.886 to 0.944 |
| the same | 24 others | 1 | 0.911 | 0.918 ± 0.8% | 0.875 to 0.946 |
| the same | 21 units of 4995 | 1 | 0.912 | 0.912 ± 0.6% | 0.890 to 0.937 |

In the scratch build with parts 1 to 3 a step fell from 1.15 M cycles to 1.06 M.  The first deletions fell from 5.6% of
a step to 1.8% (78 k cycles a deletion's search against 244 k), the loss of the smaller sets from 3.9% to 2.5% (318 k
cycles a search against 530 k), and the contractions from 4.5% to 2.8% (132 k cycles where a child promotes several
numbers).  Game 3000 keeps every counter of `tune` under the default tactics (117,197 steps), the chain's (161,460) and
conflict branching off (161,460), at 0.975, 0.951 and 0.949 of the CPU (paired, one run each).  Debug builds of `tune`
solved the roots of 1000 to 1700 under the chain's settings, of 1000 and 1350 with conflict branching off, of 1000 and
1300 with price reuse and the warm start off too, and six units of 4984 with the store, every check passing.

**Tried and not kept.**

* *The least arc into the member as every vertex's bound in the A* search for forced members*, besides its least arc
  out: 1.000 of the CPU of parts 1 to 3 (geometric mean 0.996, two runs).  A search that proves a member forced takes
  the same vertices with it, since it already offered none a distance past the slack less that arc, and the searches
  that find a cycle took few fewer.
* *A child's vectors copied and contracted in one pass over the elements*, each reduced as it is copied: 1.010 of the
  CPU of parts 1 to 3, in a build with part 4 as well (unit ratios 1.002 to 1.018, two runs).  The blocks that a child
  does not touch, a plain copy before, became a loop with a test an element.

**What is left.**  At 1.06 M cycles a step (parts 1 to 3), the levels nodes keep for themselves take 33%, their
tableaux restored, derived and split 18% and their searches for forced members 11%; a child's promotion takes 10%, two
thirds of it where it promotes a node's forced members (282 k cycles each, a peel of each frame they touch); and strong
branching 12%.

**Decision.**  Kept, as the default: every tree is the same, so it is no tactic.

## 2026-10-28: floors from the level before, offers dropped before their bounds, and copies swapped back

**Question.**  After the searches that stop where their checks do, where does a step of game 4984 go under the chain's
settings, and can another 7% of the CPU be cut with the same trees?  **Answer.**  Yes: 0.906 of the CPU of ff89271 on
the 24 units of the sections above, 0.903 on 24 others and 0.899 on 21 units of 4995, with every tree the same.  A warm
child takes what the sets of other sizes lose from its parent, which spares the search for the smaller sets' loss at
nearly every warm child; the A* search for forced members finds an element's bound only where the offer could be made
at all; and the copy of the tableau a node keeps for itself is taken back by swapping its arrays into the workspace.

**Method.**  The 24 units of the sections above under the chain's settings since game 4995: conflict branching on at a
gate of 1,200, price reuse, the warm start, and the store of 144 units of 4983 (106,567 steps in all).  Scratch builds
counted the cycles of a step's parts by the kind of level, with the level a node keeps for itself tagged apart from the
children that inherit their parent's heaviest set, and inside the A* search: the members and elements it takes, the
entries of the members' columns it reads, the bounds it finds and the offers it makes, by whether it finds a cycle.  A
third build counted the members a level a node keeps for itself took in by its flip.  Each build was then paired
against the one before it or against ff89271, the two solving each unit at once on the two hyperthreads of one core,
as in the sections above.

**Where a step goes.**  At ff89271, 1.05 M cycles a step (the counters overlap):

| part | share of a step |
|---|---|
| the levels nodes keep for themselves: their checks | 33.2% |
| of which the tableau restored (92 k cycles), derived (203 k) and its columns built (136 k) | 11.3% |
| of which the A* searches that find a cycle, 4.0 an evaluation at 52 k cycles each | 6.1% |
| of which the A* searches that show a member forced, 26.5 an evaluation at 5.3 k cycles each | 4.1% |
| warm children: their checks | 11.2% |
| of which what the smaller sets lose, 357 k cycles a check | 2.8% |
| the copy of the tableau a node keeps for itself | 3.1% |
| a child's promotion: its peels, 2.3 frames a step at 39 k cycles each (215 moves a frame) | 10.0% |

Three findings decided the change:

* *A warm child searched for what its smaller sets lose at every check*, where its parent's floor shows it.  A warm
  child's sets, with what it promoted, are its parent's, and its heaviest set with them weighs less than the parent's by
  what the deletions lost, so its slack is the parent's less that loss.  The parent's check of forced members had shown
  that its smaller sets lose more than its slack, so the child's lose more than its own.
* *The A* search found the bound of every element its members' columns offer before testing the offer.*  A search that
  finds a cycle read 1,083 entries of the columns of the 22.8 members it took, and offered 45 of them.
* *The copy of the tableau a node keeps for itself was gathered element by element and scattered back*: 101 k and 92 k
  cycles.

**The change.**  Three parts, each leaving every result the same:

1. *Floors after a warm start or a deletion* (`floors_after`).  A level that came from another by promoting members `F`
   of its heaviest set `I` and deleting candidates (a warm child), or by deleting candidates alone (a node that goes on
   without a member of `I` by a deletion), makes every common independent set `K` of its problem, with `F`, one of the
   other's.  Its heaviest set `J`, with `F`, weighs `lost` less than `I`.  Where `|J ∪ F| ≤ |I|`, a set `K` smaller
   than `J` makes `K ∪ F` smaller than `I`, which weighs at most `w(I)` less the floor of what `I`'s smaller sets lose,
   so `K` loses at least that floor less `lost` against `J`; and where `|J ∪ F| ≥ |I|` the larger sets likewise, a gap
   of no larger set staying one.  Debug builds check every floor of the smaller sets' loss so carried against that
   loss, found by A* and checked against Dijkstra's search, and every floor of the gap against the gap.
2. *Offers dropped before their bounds.*  The A* search for a cycle through a member makes no offer at a distance at or
   past its limit or the shortest cycle found, whatever the vertex's bound, which is never negative; so it tests that
   before it finds the element's bound.
3. *The copy swapped back.*  The copy of a tableau (`Snapshot`) is the workspace's arrays copied whole: each element's
   mask, flag and slot, the blocks' slots and live elements, the set and its lists.  It is taken back by exchanging the
   arrays with the workspace's, and so serves once; the columns of A are built again where they are read, as before.

**Results.**  CPU of each build over the one named, paired, every tree the same:

| build | units | runs a unit | total CPU | geometric mean of the units' ratios | the units' ratios |
|---|---|---|---|---|---|
| the copy swapped back, with its columns, over ff89271 | the 24 above | 1 | 0.994 | 0.994 ± 0.3% | 0.978 to 1.006 |
| that and parts 1 and 2, over ff89271 | the 24 above | 1 | 0.928 | 0.931 ± 0.6% | 0.907 to 0.955 |
| the same | 24 others (gaps 500 to 1,300) | 1 | 0.930 | 0.940 ± 0.9% | 0.905 to 0.981 |
| the same | 21 units of 4995 with its store of 20,000 nogoods, at most 20,000 steps each | 1 | 0.937 | 0.935 ± 0.8% | 0.883 to 0.954 |
| the copy without its columns (the build committed), over the build above | the 24 above | 1 | 0.968 | 0.970 ± 0.3% | 0.956 to 0.988 |
| the copy swapped back against the copy scattered, both builds with the sorted columns below | the 24 above | 1 | 0.971 | 0.972 ± 0.2% | 0.961 to 0.988 |
| the build committed over ff89271 | the 24 above | 2 | 0.906 | 0.914 ± 0.7% | 0.886 to 0.960 |
| the same | 24 others | 1 | 0.903 | 0.930 ± 2.1% | 0.883 to 1.155 (a unit of 49 steps) |
| the same | 21 units of 4995 | 1 | 0.899 | 0.898 ± 1.6% | 0.819 to 0.927 |

In the scratch build of the change a step fell from 1.05 M cycles to 0.98 M, less than the CPU did.  At a warm child the
smaller sets' loss took 3.4 k cycles against 357 k; the A* searches that find a cycle at the levels nodes keep for
themselves took 44 k against 52 k, and those that show a member forced 4.0 k against 5.3 k; the copy took 81 k cycles
against 101 k, and taking it back 3.4 k against 92 k.  Game 3000 keeps every counter of `tune` under the default tactics
(117,197 steps), the chain's (161,460) and conflict branching off (161,460), at 0.968, 0.929 and 0.929 of the CPU
(paired, one run each).  A debug build of `tune` that checked the floor of the smaller sets' loss at every check of
forced members solved the roots of 1000 to 1700 by 50 under the chain's settings, of 1000 and 1350 with conflict
branching off, of 1000 and 1300 with price reuse and the warm start off too, and six units of 4984 with the store,
every check passing, each in the steps and best of the base; the build committed, which checks a floor where it was
carried from the level before, solved seven of those roots and four of those units so.

**Tried and not kept.**

* *The columns of A kept in the copy*, so that the level a node keeps for itself derives its columns from the flip:
  the copy cost 158 k cycles against 101 k, and the columns derived from a flip 148 k against 136 k built.  Leaving them
  out took 0.968 of the CPU.
* *Columns derived from a log of the circuits that changed*, each element that changed taken out of the columns of its
  circuit before, instead of a pass over every column: 1.070 of the CPU of ff89271 with the copy swapped back (1.076 of
  that alone).  A column holds 47 elements on average and many more in the large blocks, and finding each element in
  it costs more than the pass.
* *Columns sorted by `w1` for the searches*, each made when a search first reads it on a tableau, so that a search reads
  a member's column only as far as its first offer too long: 1.090 of the CPU of the build committed.  A column is read
  few times on a tableau (at the levels nodes keep for themselves, about 150 reads an evaluation for 118 members), and a
  sort costs more than the entries it spares.

**What is left.**  At 0.98 M cycles a step, the levels nodes keep for themselves take 32%: their A* searches 10%, their
tableaux derived 8% and their columns built 3%.  Of the 4.0 searches an evaluation there that find a cycle, 0.7 are
for the 2.4 members that the flip took in and 3.4 for members from before: the flip breaks the cycles that the levels
before found.  The A* searches read the members' columns for few offers, 1,083 entries for 45 in a search that finds a
cycle, at about 26 cycles an entry, about 10% of a step in all with strong branching's searches for the lengths of the
cycles (1.1 a step, 45 k cycles each).  A child's promotion takes 11%, nearly all of it in the peels of the frames it
touches.

**Decision.**  Kept, as the default: every tree is the same, so it is no tactic.

## 2026-10-28: what a step's parts cost made twice, and changes that did not pay

**Question.**  After the floors from the level before, can another 7% of the CPU of a step of game 4984 be cut under the
chain's settings, with the same trees?  **Answer.**  Not found.  The changes to the CPU's work that the last profile
pointed to took 0.992 to 1.002 of the CPU each and 1.002 together.  Making a part twice, which measures what it costs
on warm caches, put the A* searches for forced members at 8.1% of the CPU and the peels of a child's frames at 6.6%.
Peel orders kept along the path, which check a promotion's frames from the order the parent's peel found instead of
peeling them, took 1.002 to 1.066 of the CPU in six versions, never less than the peels.

**Method.**  The 24 units of the sections above under the chain's settings since game 4995: conflict branching on at a
gate of 1,200, price reuse, the warm start, and the store of 144 units of 4983 (106,567 steps in all).  Each idea was a
switch of a scratch build, read from the environment, and the build with the switch off and on solved each unit at once
on the two hyperthreads of one core, one run a unit; the base build paired against itself so took 0.9987 of the CPU
(geometric mean 0.9992 ± 0.1%).  A switch that makes a part twice, the second time just after the first, measures that
part's cost on warm caches, and since the second run leaves every result as the first left it, the trees stay the same.
Four scratch builds counted cycles: round 6's counters on master with more of this round's, and three builds of the peel
orders.  The peel orders were
built in the solver itself and paired against 1b59354; debug builds of each version solved the roots of 1000 and 1100
under the chain's settings, of 1000 and 1350 with conflict branching off, and three units of 4984, every check passing
and each in the base's steps.  Every pair took the same steps and found the same best at every unit.

**Changes to the CPU's work.**  None paid.  The last profile put about 26 cycles on each entry of a member's column that
the A* search reads, but filtering a column without branches before the offers took 0.992 of the CPU; the parts below
took no more than the noise of a pair.

| change | CPU |
|---|---|
| the A* search for forced members: a column's entries filtered without branches before the offers | 0.992 |
| the same in the search of `Meet::without` | 0.997 |
| the same in the split's rounds | 0.998 |
| `w2` of the member in each slot of `B`, for the A* search's bounds and its circuits of `B` | 0.996 |
| kept cycles: whether a cycle frees a member asked before whether its elements are live | 0.997 |
| a frame that only loses a factor no move has, not peeled (it never happened: 0 of 89,811 checks of one number) | 0.998 |
| the peel: each move's live factors listed as it is counted, by its index among the moves, and the factors with one move pushed after the count | 1.000 |
| a block's exchanges in a derivation applied in one pass over its elements | 1.002 |
| all of the above but the peel | 1.002 |

**Parts made twice.**  Each row is the CPU of the build that makes the part twice over the one that makes it once.
The second time finds the caches warm, so a part costs at least this much.

| part made twice | CPU |
|---|---|
| the A* searches of the check of forced members (`Meet::has_cycle_within`) | 1.081 |
| the peels of the frames a promotion changes | 1.066 |
| the columns of `A` built | 1.024 |
| the least arcs into the members (`Meet::closing_lengths`) | 1.021 |
| the forest of a split's predecessors | 1.016 |
| a tableau's lists | 1.015 |
| a split saved | 1.003 |
| the copy of the tableau a node keeps for itself | 1.002 |
| a child's vectors copied | 1.002 |

A copy of a tableau or of a child's vectors, 2.7% and 2.4% of a step by the counters, costs little made again: its cost
is the first touch of the memory.  The searches and the peels cost the same warm: they are work.  Padding the GF(2)
vectors and tableaux to five words where four do took 1.023 of the CPU, a measure of what their width costs.

**Peel orders.**  The peel finds an order in which it drops a frame's moves, each with its *witness*, the factor that
no move left has when the move drops.  After a promotion each move keeps its witness unless the promotion took it away
or gave it to a new move, so a frame's order in the parent checks the child's frame with only the moves those touch
re-dropped: a new move is held back until its factors' last moves in the order have dropped, and a move whose witness a
move held back has is held back too.  The orders were kept by slot of the searcher's stack, a frame's order in the slot
of the node whose promotion last changed it.  Debug builds compared every check with a peel, and every order with the
frame.

| version | CPU |
|---|---|
| orders found by the children's peels, none at a unit's node | 1.002 |
| a unit's node's orders made at its start; moves held back to the end | 1.017 |
| moves held back dropped as soon as one of their factors has no move left, found by a scan of the held moves a turn | 1.066 |
| the same with a count of the held moves a factor has, and the next turn one can drop | 1.006 |
| a factor's last move in the order found from the factor's moves, not from a pass over every move's factors | 1.016 |
| the same, each factor's last move kept once found, and up to 32 moves held back | 1.016 |

The counters of the second version show why the order does not carry: of 207,632 checks from an order, 122,267 (59%)
held back more moves than the check had room for and peeled the frame after all, and the others held back 7.7 moves
on average.  Most factors are some move's witness, so a new move holds back the moves whose witnesses it has, and those
hold back the moves whose witnesses they have.  With drops as soon as they can, 22% of the checks still peeled the frame
(16 moves held back at most), and a check took 46 k cycles on average, the peels it fell back to included, more than a
peel's 38 k.

**What the profile found besides.**  At a push the parent has 1,141 live elements of the root's 1,349, and the units'
nodes 1,248 to 1,329 candidates, so the elements a unit can no longer use are too few to shrink the arrays by.  A peel
counts 215 moves and 265 factors; 7.3% of the frames a number is promoted into have a factor that no move of theirs
has, and a number promoted out of its frame is a live factor of a move there every time.  A repair of the split takes
104 k cycles at a child that keeps its parent's heaviest set, of which the forest of the start's predecessors takes
46.6 k and the passes over the live elements 51.9 k, against 3.5 k for the subtrees that break (14.5 roots, 9.2 vertices
pulled); at a level a node keeps for itself, 160 k, of which 76.2 k the passes and 64.6 k the pulls and scans of `B`.
The child's candidates take 4.4% of a step beyond its promotion, 2.5% of it the carried store and the rule of symmetry.

**Ideas left, not built.**

* *An element's search state in one record*: in the A* search for forced members, its distance, bound, stamps,
  predecessor and `w1`, `w2` and freedom in `B`, so that an offer reads one cache line instead of six arrays.  The searches cost 8.1% warm, and the
  passing entries (342 of the 1,083 a search that finds a cycle reads) do most of their work.
* *A split repaired with no pass over every live element* where a child only contracted members: the forest of the
  start's predecessors and the passes are 94% of such a repair, which is 3.7% of a step by the counters.
* *A set width of 80 words* for the boards 4096 to 5119 (0.981 of the CPU in round 3), taken at a game boundary, since a
  checkpoint made at another width is refused.
* *Profile-guided optimization of the build*, which needs `llvm-profdata` (the toolchain's `llvm-tools`, not installed
  here) and a training run in the chain's build.

**Decision.**  Nothing kept; the code stays at 1b59354.

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
`~/taxman_work/lab/work/step2-2026-10-27/` has the runs of the A* search for forced members: in `tools/` the runner of
one build (`solo.sh`) and of two at once (`two.sh`), the paired runner (`pair.sh`), the summaries (`profsum2.py` for the
counters by kind of level, `brief.sh` for the comparisons, `wsumm.py` for the pairs), game 3000's runs (`ident.sh`) and
the scripts that edited the scratch build (`edit_astar.py`, `edit_astar2.py`); the scratch build's counters as patches
against 947946a (`prof.patch`, the first, and `prof-switches.patch`, the last with every switch, with its module
`prof.rs`); the unit lists (`set24.txt`, `setB.txt`); the quick checks' logs (`quick*.log`); and a file a solve in
`profA/` to `profM4/` (the profiles, a folder a switch), `pair1/` and `pair2/` (the paired runs), `pair3/`, `pair5/`
and `pair6/` (the A* search with the arcs packed), `pair7/` (the build committed) and `ident/` (game 3000).
`~/taxman_work/lab/work/patch4991/`, `patch4992/` and `patch4995/` have the settings checks of 4991, 4992 and 4995, a
folder a game: the live checkpoint the units were cut from (`nNNNN-live.ckpt`) and the units (`units-NNNN.jsonl`), the
build of `tune`, the runner (`run.sh SETFILE SAMPLEFILE OUTDIR NODELIMIT`, resumable), the settings (`settings.txt`),
the unit ids (`sample.txt`, and `sample2.txt` for 4991's second batch), a file a solve in `out/`, and `analyze.py` with
its output, `analysis.txt`; `patch4995/` has the unit picker (`pick.py`) and `notes.txt` too.
`~/taxman_work/lab/work/step3-2026-10-27/` has the runs of the splits started from what changed: in `tools/` the runner
of one build (`solo.sh`, `one.sh`), the paired runner (`pair.sh`, with `EXTRA` for other tactics) and its summary
(`wsumm.py`), game 3000's runs (`ident.sh`), the counters' summaries (`psum.py`, `psum2.py`, `psum3.py`) and the scripts
that made each version (`edit_*.py`, the counters `prof2_apply.py` and `prof3_apply.py`); the counters' module
(`prof.rs`) and first patch (`prof.patch`, on 29aa8ba); each version's patch (`real-*.patch`); the microbenchmarks of
the columns and of the forest (`mb/`); the unit lists (`set24.txt`, `setB.txt`); and a file a solve in `prof1/` to
`prof3/` (the profiles), `pair1/` to `pair11/` (the versions paired, in the order they were built),
`pairB/`, `pairC/` and `pairBC/` (that version on the other units and with conflict branching on) and `ident-v7/`
(game 3000); and, after the fault the tests found, the quick check's log (`qc/`), the runner of a debug build on games'
roots (`tools/dbg.sh`), and the runs of the build committed (`tools/recheck.sh`: `fixC/`, `fixO/`, `fixBC/`, `fixBO/`
and `ident-fix/`).
`~/taxman_work/lab/work/store-2026-10-27/` has the runs of what the store costs a step (2026-10-28): in `tools/` the
runner of one build (`solo.sh`), the paired runners (`pair.sh` with one store, `pair2.sh` with a store and a switch a
side), the summaries (`profsum.py`, `percall.py` and `perunit.py` for the counters, `pairsum.py` for the pairs,
`same.py` for the trees) and the unit picker (`pick.py`); the counters as patches against 29aa8ba (`prof.patch` on the
old store, `prof-v1-dry.patch` on the rank sets, with the dry switch); the variants' code (`carried-v1-clean.rs` the
change tried, `carried-v2.rs` the watch on a member that is no candidate, `carried-v3-scratch.rs` the 16-bit ranks and the
order); the builds (`bin/`); the unit lists (`prof8.txt`, `pair21.txt`, `debug8.txt`); and a file a solve in `profA/`
to `profC/` (the profiles of the old build, the change and the variant), `pairA/` and `pairC/` (the change against the
old build, the two stores), `pairB/` (the dry runs), `pairD/` (the two stores on the new build), `pairE/` (the 16-bit
ranks and the order) and `debug/` (a debug build).
`~/taxman_work/lab/work/step4-2026-10-28/` has the runs of the warm starts stopped at the need (2026-10-28): in `tools/`
the runner of one build (`solo.sh`), the paired runners (`pair.sh` on 4984, `pair4995.sh` on 4995, `final.sh` for the
build committed), the debug runners (`dbg.sh` on games' roots, `dbgunits.sh` on units), game 3000's runs (`ident.sh`),
the counters' summary (`psum4.py`, over round 3's `psum3.py`) and the second script of counters (`prof4_apply.py`, after
round 3's `prof3_apply.py`); the counters as a patch against 7140548 (`prof.patch`, with its module `prof.rs`), the
width of 80 words (`w80.patch`) and the build with switches (`sw.patch`, against the build committed); the builds
(`bin/`, the switches as wrappers `sw-s1` to `sw-s3`); the unit lists (`set24.txt`, `setB.txt`); and a file a solve in
`prof1/` to `prof3/` (the profiles), `pairA/` to `pairD/` (the builds in turn), `pairW/` (the width), `swS1/` to
`swS3/` (the switches), `final-v6/`, `finalB-v6/` and `p4995-v6/` (the build committed), `p4995-v4/` (a build before
it on 4995) and `ident-v6/` (game 3000), with the debug runs' output in `debug*.txt`.
`~/taxman_work/lab/work/step5-2026-10-28/` has the runs of the searches that stop where their checks do (2026-10-28): in
`tools/` round 4's runners (`solo.sh`, `pair.sh`, `pair4995.sh`, `final.sh`, `dbg.sh`, `dbgunits.sh`, `ident.sh`, with
`ident_pair.sh` for game 3000 paired) and summaries (`psum3.py`, `psum4.py`, `wsumm.py`, with `praw.py` for single
counters), the scripts that put round 4's counters on the old build and on the change (`fixprof.py`, `fixprof2.py`,
`fixprof3.py`, after round 4's `prof.patch`), and the scripts that edited the files (`splice.py` with
`contract_all.rs.txt`, `reflow.py`, `refill_tail.py`); the scratch builds' sources (`prof-src/` on daeade5, `prof2-src/`
on parts 1 to 3); the builds (`bin/`); the unit lists (`set24.txt`, `setB.txt`); and a file a solve in `prof1/` and
`prof2/` (the profiles of the old build and of parts 1 to 3), `pairA/`, `pairAB/` and `p4995-v1/` (parts 1 to 3 over
daeade5), `pairB/` (the bound of the arc into the member), `pairC/` (the copy and contraction in one pass), `final/`,
`finalB/` and `p4995-final/` (the build committed), and `ident/` and `ident-pair/` (game 3000, unpaired and paired),
with the debug runs' output in `debug*.txt`.
`~/taxman_work/lab/work/step6-2026-10-28/` has the runs of the floors from the level before (2026-10-28): in `tools/`
round 4's and 5's runners (`solo.sh`, `pair.sh`, `pair4995.sh`, `final.sh`, `ident_pair.sh`, `dbg.sh`, `dbgunits.sh`),
the debug drivers (`debug_all.sh`, `debug_final.sh`), the base's runs of the same roots and their comparison
(`roots_base.sh`, `roots_cmp.py`), the pairs' summary (`wsumm.py`), the scripts that put the counters on a copy of the
source
(`prof_apply.py` with `prof.rs`, then `prof_apply2.py` to `prof_apply5.py`) and their summary (`psum6.py`), and the
script that put the original copy into a build (`old_snapshot.py`); the variants as patches against ff89271
(`v2-plus-floors.patch`, the columns derived from a log, with the floors; `v3.patch`, the columns kept in the copy;
`v4.patch`, the build committed before its comments were written; `v5.patch`, the sorted columns); the scratch builds'
sources (`prof-src/` to `prof5-src/`, `v5a-src/`); the builds (`bin/`); the unit lists (`set24.txt`, `setB.txt`); and a
file a solve in `prof0/` (the first profile, whose counters of the split overlapped), `prof2/` and `prof3/` (the
profiles of ff89271), `prof4/` and `prof5/` (of the columns kept in the copy and of the build committed), `pairV1/` to
`pairV3/`, `pairBV3/` and `p4995-v3/` (the builds on the way over ff89271), `pairV4v3/` (the copy without its
columns), `pairV5v4/` (the sorted columns), `pairV5aV5/` (the copy swapped back against the copy scattered),
`final-v4/`, `finalB-v4/` and `p4995-v4/` (the build committed), `ident-v4/` (game 3000), `debug4/` and `debug5/` (the
debug runs) and `roots-base/` (the base on the debug runs' roots).
`~/taxman_work/lab/work/step7-2026-10-28/` has the runs of the parts made twice and the changes that did not pay
(2026-10-28): in `tools/` round 6's runners and summaries, `pairenv.sh` (a pair with an environment of its own on each
side), `switches.sh` (each switch of a scratch build paired against its build with none) and `unitsize.py` (the units'
candidates and depths); the scratch builds' sources, each switch read from `TAXMAN_X` (`x1-src/` the changes to the
CPU's work, `x2-src/` with `x2_apply.py` the slots' `w2` and the exchanges in one pass, `x3-src/` the peels made twice
and the five-word vectors, `x5-src/` with `x5_more.py` the other parts made twice), the counters' sources (`p1-src/`,
round 6's counters with this round's; `p2-src/` and `p3-src/`, the peel orders' second and fifth versions with
`p2_apply.py`, `p4_apply.py` and `p6_apply.py`); the peel orders' last version as a patch against 1b59354
(`peel-orders-v6.patch`); the builds (`bin/`); the working notes (`notes.md`); and a file a solve in `aa1/` (the base
against itself), `sw-x1-*` to `sw-x5-*` (each switch, by its mask), `v1-a/` to `v6-a/` (the peel orders over the
base), `prof-p1/`, `prof-p2/`, `prof-p5/` and `prof-p6/` (the counters), with the debug runs in `dbg1/` and `dbg3/` to `dbg6/`.
