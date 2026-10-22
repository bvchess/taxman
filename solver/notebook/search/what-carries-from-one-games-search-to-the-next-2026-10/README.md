# What carries from one game's search to the next (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-22.

**Question.**  Consecutive games of the chain pose nearly the same search problem.  Their work does not carry
([carrying a search's learning](../carrying-a-searchs-learning-to-the-next-game-2026-10/README.md): the trees share
0.15% of their steps, carried nogoods cost more than they save).  Which *lessons* carry, then: what can game n's search
tell game n + 1's about how to search, or what can a search learn about itself while it runs, so that the searches get
faster?  A lesson that only guides the search may be wrong at times; it must leave the proof exact.

The work ran beside the chain on its 88-thread host, at most 8 threads at `nice -n 19`, on the snapshots of games 4899,
4900 and 4901 that the lab's studies took (three consecutive games, each searched with conflict branching) and on whole
games proved from the root against the known optimum.  Code: a branch of the solver with the studies' variants
`TAXMAN_CB_GAP` and `TAXMAN_PROBE` cherry-picked, and the example `carry` (below).

## 2026-10-22: what changes between consecutive games, and what that leaves to carry

**The edits.**  `carry dissolved M N` sets up both games as the solver does and lists the frames of M that have no twin
in N: no frame of N with the same moves, factors and freedoms and the same candidates promoted out of it and into it.

| step | P\* | what changes | search of N |
|---|---|---|---|
| 4898 → 4899 | 544,987 both | frame 0: moves 2546 and 3819 and factor 1273 (= 19 · 67) give way to 3266, 4899 and 1633 (= 23 · 71) | 2.43e8 steps |
| 4899 → 4900 | +2,444 | the source 2450 of frame 2 becomes a candidate of frame 3 and the factor of the new move 4900 | 2.30e8 |
| 4900 → 4901 | 547,431 both | frame 0: the move 2783 (= 11² · 23) gives way to 4901 (= 13² · 29); same candidates, same factors | 2.31e8 |
| 4879 → 4880, 4883 → 4884, 4887 → 4888, 4863 → 4864 | +1,868, +888, +552, +407 | as 4899 → 4900: one source of frame h becomes a candidate of frame h + 1 | |

4899 and 4901 are near repeats: the same candidates, `max_total` and P\* as the game before, and a swap of a few
numbers in frame 0, which `same_search` cannot see.  They are 2 of the 9 searched games from 4880 to 4901 and 18% of
their steps, but 4 of the 46 searched games since 4400 (2.5% of the steps, or 5.6% leaving out the 1.56e10 steps of
4875 before conflict branching).

**The obligation of the next proof, given the last.**  If game M is proven and P\*_M ≤ V, every promotion set of N worth
more than V is not one of M (Theorem D of the carrying entry, in the form that needs no relaxation).  A set of N that M
cannot promote breaks a frame of M that has no twin in N, and a frame that a set breaks stays broken under every larger
set, so the set holds a *dissolved conflict*: candidates touching that frame which M cannot promote and N can,
minimal in M; or a new candidate.  N's proof may then search only the subtrees that promote one of them.  Measured:

* In each one-source step the new candidate's subtree has exactly the root's gap (2048 at 4900, 2079 at 4880, 1982 at
  4884 and 4888, 1943 at 4864): every heaviest set holds it, so the restriction saves nothing.  The other changed
  frame gains a freedom, so its dissolved conflicts are its sets of freedoms + 1 members, too many to enumerate.
* In the near repeat 4900 → 4901 the changed frame is frame 0, with 581 candidates touching it.  Its one dissolved
  conflict of two members has gap −1,061; the sets of three are 166,000 pairs promotable in 4900 to extend, and the
  enumeration was stopped.  A stuck set of frame 0 in M that is not stuck in N must hold the move 2783, so the dissolved
  conflicts are the sets that exhaust both of its factors, 253 and 121, in every winning order: a property of the whole
  frame, which no small enumeration reaches.
* The common relaxation of a near repeat (frame 0 without the move that differs) wins one freedom in frame 0, so its
  P\* is above both games' ([going higher](../going-higher-and-coming-back-down-2026-10/README.md): a relaxation's root
  bound is never smaller).
* Nor is a near repeat a repeat under another name.  `carry gadget` lists the factors that pay for the swapped moves
  and what else they pay for: 121 and 253, which pay for 2783, also pay for the candidates 242, 363, 605, 847 and 506,
  759, 1265, 1771; 169 and 377, which pay for 4901, pay for 338, 507, 1183, 2197 and 754, 1131.  No relabeling of the
  numbers that are not candidates maps one game's frames onto the other's.

So the obligation carries no shortcut that is cheap to use.

**Near repeats search the same tree.**  The units of a snapshot are the nodes the engine holds at that moment.  Of the
6,590 units of 4901's snapshot (18.0e6 steps in), 861 have exactly the promotions and the candidates of a unit of
4900's snapshot (25.8e6 steps in), and all 861 the same gap; 936 have the same promotions.  Across the one-source step
4899 → 4900, 234 of 6,275 units match up to the new candidate, their gaps 6 higher (P\* rose by 2,444 for a candidate
of 2,450).  The root gaps of 4900 and 4901 are both 2,048, and their searches took 2.30e8 and 2.31e8 steps.

Twins search alike, step for step.  24 twin pairs of 4900 and 4901, drawn by gap band and depth (seed 7,
`pick_twins.py`), solved on one thread for at most 150 CPU seconds each:

* with the default search, the 20 pairs that ended took the same steps in both games, from 668 to 77,820; the 4 that
  the limit stopped were within 1% of each other;
* with the probe (a unit of depth 55 or more searched without conflict branching for 4,096 steps, then with it), the
  11 deep pairs again took the same steps in both games; the probe ended 5 of them, in 70, 78, 223, 295 and 1,463
  steps, where the default took 3,094, 34,892, 30,613, 25,707 and 3,720.

The same holds for a whole game: 3501 is a near repeat of 3500 (the same candidates and P\*; frame 0 gains the factor
1167 and the moves 2334 and 3501, and `max_total` falls from 212 to 211), and proved from its root on one thread it
takes 1,166,725 steps, exactly as 3500 does.

So across a near repeat both the cost of a node and which setting wins there carry exactly.  A near repeat spends its
whole search proving again what the game before it proved, node for node; a sound way to carry that proof would save
its whole cost, 18% of the steps from 4880 to 4901.  The obstacle is the argument: the units are the same, but their
problems differ in frame 0, and the obligation above gives no cheap check of which subtrees the swap cannot reach.  The
section on a cut below finds one.

**The hard region is the optimum's neighbourhood.**  A unit's promotions lie 96-97% inside the incumbent (the game's
optimal set, the previous game's lifted), at every gap; 4900's and 4901's optimal sets are the same 265 candidates.

**Whether off wins is a property of a region.**  Of the probed deep units of two studies (4900: off for 256 steps then
on, 26 units; 4901: 4,096 steps, 30 units), the verdict "off ended the unit within the probe" agrees between two units
in proportion to how close their promotions are:

| promotions differing by | pairs | verdicts agree |
|---|---|---|
| 0-9 | 14 | 11 |
| 10-19 | 81 | 46 (4900: 15 of 17) |
| 20-39 | 360 | 209 |
| 40 or more | 305 | 109 |

The units off ends have excluded more members of the optimum (neither promoted nor still candidates): a median of 5.5
against 3 at 4901, 5 against 3 at 4900.

**The members of the optimum a unit has left out.**  To test that, 33 units of 4901's snapshot of depth 40 to 99 and gap
600 to 1,100, three a cell of depth band (40-59, 60-79, 80-99) and members of the optimum left out (3 or fewer, 4, 5, 6
or more; seed 11, `pick_excl.py`), each solved on one thread for at most 150 CPU seconds with conflict branching on,
off, and with a study rule, `x5`: a node that has left out 5 or more members of the optimum branches on the search's own
conflict, as with the tactic off, and the others as with it on (`TAXMAN_CARRY_EXCL=5`, `TAXMAN_CARRY_REF` the
optimum's set).  The count only grows down a path, so `x5` is off from the root of a unit that has left out 5.

| left out at the unit | units | off beats on | steps on | off | x5 |
|---|---|---|---|---|---|
| 3 or fewer | 9 | 1 | 133,956 | 713,326 (2 stopped) | 346,304 |
| 4 | 9 | 2 | 148,524 | 500,520 (2 stopped) | 471,184 (2 stopped) |
| 5 | 9 | 5 | 46,939 | 47,490 | 47,490 |
| 6 or more | 6 | 6 | 25,072 | 4,260 | 4,260 |

(Sums of steps, a stopped run counted at its stop.)  Where a unit has left out 6 or more members, off wins every
time, by 5.9 times on the sum; at 4 or fewer it loses 15 times out of 18.  But switching off inside a subtree, at the
node where the count reaches 5, loses 2.6 to 3.2 times on the units that start below it: the count separates units,
not the nodes within their subtrees.  Units that have left out 5 or more hold 3% to 7% of the notebook line over each
of the three snapshots, units that have left out 2 or 3 hold 75% to 85%.

On whole games, proved from the root against the optimum on one thread, where every node is inside one unit, the rule
as a node switch is at best even (steps against the default; all ended at the optimum):

| game | default: steps, CPU s | x4 | x5 | x7 |
|---|---|---|---|---|
| 3000 | 107,724, 40 | 1.07 | 1.13 | 1.00 |
| 3200 | 1,054,752, 555 | 1.18 | 1.08 | 1.00 |
| 3500 | 1,166,725, 521 | 1.07 | 1.00 | 0.92 |

A count taken at a unit's root is what the engine has at hand: its units are the open nodes of a searcher's stack when
it yields, loops part way through, as a snapshot's are.  A loop's tried members are what such a node has left out, so
the count says how far its loop has gone past the optimum's members.  Which node features decide the setting is the
question of the study of when a node should use conflict branching (20c1f46); what carries is the reference, the
optimum, which is the previous game's lifted.

## 2026-10-22: does the better setting of one game hold for the next?

Of the settings that keep the proof exact, the gap gate of conflict branching is one whose better value changes from
game to game: the studies found gate 150 at 0.80 of gate 300's CPU on game 3500 and 1.21 times it on 3200.  Whether a
game's better gate predicts the next game's decides whether a setting can be carried.  `carry problems` lists the
search problems near each (a repeat shares its digest with the game before it), and every distinct problem of the two
stretches was proved whole from its root on one thread at gate 300 (the default) and at 150 (`TAXMAN_CB_GAP`):

| game | steps at 300 | CPU s at 300 | steps at 150 / 300 | CPU at 150 / 300 |
|---|---|---|---|---|
| 3195 | 1,317,851 | 764 | 0.65 | 1.29 |
| 3198 | 2,714,779 | 1,533 | stopped at 1,800 s | more than 1.17 |
| 3200 | 1,054,752 | 555 | 0.60 | 1.22 |
| 3201 | 1,191,375 | 675 | 0.73 | 1.53 |
| 3204 | 689,308 | 359 | 0.55 | 1.16 |
| 3495 | 2,027,938 | 1,105 | 0.39 | 0.93 |
| 3496 | 2,128,856 | 1,148 | 0.41 | 0.93 |
| 3498 | 2,180,830 | 1,189 | 0.38 | 0.89 |
| 3500 | 1,166,725 | 521 | 0.35 | 0.79 |
| 3501 | 1,166,725 | 524 | 0.35 | 0.79 |
| 3504 | 1,460,382 | 658 | 0.34 | 0.75 |

Every run that ended did so at the optimum.  Near 3200 gate 150 costs more on every problem, near 3500 less on every
one: the better gate is a property of a stretch of games, the same for consecutive games, and it turns over a few
hundred games.  So a setting chosen on game n, by a check on n's units or on a whole game near n, can be carried to
n + 1 and kept until the record shows it turning.  Near 4900 the gates 150 and 300 tie on 4884's units (study 4884);
no whole game there has been checked.

## 2026-10-22: what a lab that steers the chain's searches would need

The aim: a lab agent watches a search in flight, uses the record of recent searches, runs a few quick experiments, and
sets the search's strategy for this game and the ones after it.  The [lab channel](../../../docs/design/lab.md) has one
command for it, `branching`, which sets `conflict_branching` and, since a0954bc, `conflict_look` (the look of the
study of when a node should use conflict branching), on or off, for one game in flight, from its next units on.  The
setting is kept in the game's record `nNNNN.lab` beside its checkpoint, and `set_up_with_width` in `src/solver.rs`
reads it when the game is set up, as `GameLab::search_tactics` over the run's tactics.  Nothing reaches a game that
has not started.  What the measurements above say a steering lab needs, in the order to build it:

**1. A setting for the games to come.**  The better gate held for every consecutive problem of a stretch and turned
between 3200 and 3500, and twins of a near repeat take the same steps under every setting, so a setting chosen on game
n should reach n + 1 without a command each game.

* *The command*: `{"command": "policy", "from": 4905, "conflict_branching": "on", "conflict_look": "on"}`.  `from` is
  the first game it applies to, any game of the range not yet concluded; the setting fields are those of `branching`,
  at least one, and a field left out keeps the value the policy had.  It applies to every game from `from` on: to
  those set up later, when they are set up, and to those in flight at once, as a `branching` command for each.
* *Where it lives*: a file `run.lab` in the checkpoint directory, beside the games' `nNNNN.lab`: a JSON object with
  the list of policies given, each `{"from": N, "at": UTC, "file": the command's name, "conflict_branching": ...,
  "conflict_look": ...}`.  The policy for game n is the run's flags changed by the entries with `from` ≤ n, in the
  order given.  The file outlives the process, so a restart of `chain.sh` keeps it, and deleting it returns the run
  to its flags.
* *How a game's start picks it up*: `set_up_with_width` takes the run's search tactics, then the policy for n from
  `run.lab`, then the game's own record (`GameLab::search_tactics`): a `restart` names its tactics outright and a
  `branching` for game n wins over the policy.  A resumed game is set up the same way, and a checkpoint resumes under
  either setting of both tactics (bits 9 and 10 are not compared).
* *The record*: the game's line keeps the policy it was set up with in `lab.policy` (its `from` and its settings), so
  that the record of past searches knows what each game ran with.
* *Why it is exact*: it sets what `branching` sets, the choice between true conflicts, read by a searcher when it
  starts a unit ((b) of the lab channel); it only moves that choice from one game to the games after it.

**2. The levers, each a branching choice exact as (b) says.**

1. `conflict_look`, on master and in `branching` already: the first candidate.  The study that made it measured 0.61
   to 0.87 of the default's steps on units of 4875 to 4901, and 0.975 on whole games 3000 to 4050 together (game 3000
   alone: 117,197 steps with it, 107,724 without), so its gain on units shrinks on whole games, as the probe's did.
2. `gate`, the gap from which a node weighs conflicts (300): it changes sign by stretch, as measured above.  Study code:
   `TAXMAN_CB_GAP` (4f74667).
3. `probe`, a depth D and a budget B, which may grow with the unit's gap: a unit at least D deep is first searched
   without conflict branching for B steps, then searched again from its node with it.  The probe's steps add nothing
   to the proof but nogoods, which are conflicts, and offers, which are checked; if the engine splits the unit during
   the probe, the open nodes it pushes partition what is left, as any split does.  Study code: `TAXMAN_PROBE`
   (f4a5756).
4. `reference`, a count K: a unit that has left out K or more members of the incumbent when it starts is searched
   without conflict branching.  Off won on 6 of 6 units of 4901 that had left out 6 or more, and 3 of 18 at 4 or fewer;
   applied at every node instead, it lost 2.6 to 3.2 times on units, so it must be read at a unit's start only, as the
   probe is.  Study code: `TAXMAN_CARRY_EXCL` in `study-rules.patch` (a node rule; the unit rule is not written).

**3. A quick experiment, 15 minutes on 16 threads (14,400 CPU seconds).**

* *First, the kind of step* (one second, no search): `carry dissolved n−1 n` says whether n is a near repeat of n − 1.
* *A near repeat is a free paired test.*  Its search is the previous game's, unit for unit, under any one setting:
  24 of 24 twin pairs of 4900 and 4901 took the same steps under the default and under the probe.  So run game n
  under the policy in force and its near repeat n + 1 under the candidate, `conflict_look` first: the ratio of their
  steps is the candidate's effect on a whole game near 4900, at no extra cost and with little noise (4900 and 4901 took
  2.30e8 and 2.31e8 steps on 88 threads).  At one thread the same test runs in minutes on the 17 swaps among the near
  repeats of 3000 to 4000 (`near-repeats-2000-4900.txt`), each pair proved whole, the earlier game under the default,
  the later under the candidate.  The cut of the next section would leave little of a near repeat that swaps one
  move, but the swaps of a factor and its moves (4805, 4899) stay whole games, and they come three times as often
  (40 against 13 in games 2000 to 4900).
* *Otherwise, the game's own units*, paired:
  1. take the snapshot at 10 minutes and give each unit its gap, depth and members of the incumbent left out;
  2. draw about 40 units across those strata in proportion to the notebook line's share of each, keeping those the
     line puts under 30,000 steps so that each ends within a minute, and for the strata of the largest units use the
     look-ahead of study 4895 instead (`tune expand` 8,192 steps, then the units it leaves);
  3. solve each under the policy in force and under one or two candidates, CPU-limited to twice the line, 16 at a
     time: about 4 minutes a round, so three rounds fit;
  4. weigh each stratum's ratio by its share of the line, with a bootstrap over the units, and set the policy only when
     the whole interval is below 0.9, and only for a lever whose gains on units the record has seen carry to whole
     games.

The last condition matters: the probe took 0.53 of the steps on units and 0.97 on whole games at 7 threads (study
4900), `conflict_look` 0.61 to 0.87 on units and 0.975 on whole games, and the node rule `x5` above won on the units
it acts on from their root and lost 2.6 to 3.2 times on the others.  A candidate is judged on units drawn where the
line lies, and on whole games, not on the stratum it was made for.

**4. A record of past searches**, two JSON lines files in the lab's folder (`work/history/`), which the done event
appends and a study reads with a line of Python:

* `games.jsonl`, a line a searched game: n, the digest of its problem, the kind of step from n − 1 (from `carry
  dissolved`: near repeat by a swap, near repeat by an added move, one source moved, other) and the frames it changed,
  P\*, the root gap, the start's value and whether it was final; steps, own seconds, threads, steps a second; the
  policy and the `branching` commands in force, and when; the snapshot's units by gap, depth and members left out, the
  notebook line over them, and the steps after the snapshot over the line (3.2 to 3.7 for 4884 to 4901, a calibration
  that carries);
* `units.jsonl`, a line a unit run by any study: game, unit id, a digest of its promotions and candidates (so that the
  twins of a near repeat are found), gap, depth, members of the incumbent left out, candidates, the setting, steps,
  CPU seconds, whether it ended; and a line a whole game run to check a lever, with its threads.

## 2026-10-22: a cut that carries a near repeat's proof

The twins of 4900 and 4901 took the same steps because at those nodes the two games are the same problem.  `carry
states M N --units FILE` promotes each unit's promotions in both games and compares the frames they leave: of the
6,590 units of 4901's snapshot, 6,296 leave exactly the same frames (moves, factors and freedoms) in 4900 as in 4901,
and the swapped move is gone from both.  Of 4899's 4,928 units, 4 do in 4898.

**The cut.**  Let game M be proven, with best promotion sum P\*_M, and let the search of game N have an incumbent
worth V ≥ P\*_M.  At a node of N's search with promotions S and candidates C, if S can be promoted in M and every frame
it leaves in M has the moves, the freedoms and the factors that it leaves in N, but for factors that can pay for
nothing (no composite of theirs is a move of the frame or a member of C, and they are not in C), then the node holds no
completion worth more than V, and the search may cut it.

*Why it is exact.*  Every completion T of the node is S with a set U ⊆ C promoted from the node's state, in either
game, since a promotion set is promoted from the root one member after another and the search builds a child's state
from its parent's.  Promoting a member x of U takes the factor x out of its home frame and makes it a move of the frame
in front, in both games alike provided x has the same home and the same factors in both, which the study checks for
every candidate before it searches.  The frames that S leaves differ only in factors that pay for nothing; those stay
so under every promotion from C, since no member of C and no move has them as a factor; and a frame's winnability
depends only on its moves and the factors that pay for them.  So U keeps every frame winnable in M exactly where it does
in N: a completion promotable in N is promotable in M, so it is worth at most P\*_M ≤ V.  The argument needs P\*_M,
which the chain trusts already for its ceiling, and the same factor graph among the numbers that the frames hold,
which a builder would check for every number of both roots at setup.  It is a cut, not a branching choice: a tactic
that the solver proves and tests, not a lever of the lab.

*The study code* (`TAXMAN_CARRY_PREV=M`, `study-rules.patch`) runs the check at the start of each GF(2) check, by
promoting S in M from M's root at every node: 15% to 75% more CPU a step on the games below.

**On units** (one thread, at most 150 CPU seconds each, `jobs-prev-units.txt`), random units of gap 600 to 1,300:

| game, from | units | cut at once | steps: default | with the cut | CPU: default | with the cut |
|---|---|---|---|---|---|---|
| 4901, from 4900 (one move swapped) | 40 | 36 | 648,883 (3 stopped) | 61 | 1,045 s | 0 s |
| 4899, from 4898 (a factor and its two moves swapped) | 30 | 0 | 693,110 (4 stopped) | 669,747 (5 stopped) | 1,347 s | 1,392 s |

The 4 units of 4901 not cut at once ended in 2 to 17 steps.  Every unit ended at the optimum or was stopped at it.

**The lemma, checked at random** (`carry verify`): at a unit, four completions made greedily in each game (the unit's
candidates in a random order, or by value from the largest, each added while the set stays promotable), each tested
in the other game.  At the 395 units of 4901's snapshot where the condition holds, 0 of 1,580 completions are
promotable in one game only (and 0 of 4 at the one such unit of 4899's).  Where it does not hold, at 410 units of
4899's snapshot, 127 of 1,640 are, so the test can tell the games apart.

**On whole games**, proved from the root against the optimum on one thread, the near repeats by a swap that the scan
below found, by kind of swap:

| step | what is swapped in frame 0 | default: steps, CPU s | with the cut: steps, CPU s | nodes cut |
|---|---|---|---|---|
| 4900 → 4901 | one move of the form p²q | 230,510,602, 4,920 s on 88 threads (the chain) | 750, 9 | 184 |
| 4692 → 4693 | one move, p²q | 572,354, 406 | 30, 0.3 | 9 |
| 2525 → 2527 | one move, p²q | 4,345,210, 1,372 | 62,819, 98 | 15,879 |
| 3366 → 3367 | one move of three primes | 2,543,218, 1,771 | 699,947, 861 | 118,328 |
| 3729 → 3731 | one move, three primes | stopped at 1,800 s, at 3,471,564 | stopped at 1,800 s, at 1,502,619 | 214,383 |
| 3856 → 3857 | one move, three primes | stopped, at 2,824,867 | stopped, at 1,832,094 | 270,313 |
| 3912 → 3913 | one move, three primes | stopped, at 3,335,265 | stopped, at 2,155,239 | 339,738 |
| 3020 → 3021 | a factor and its two moves | 19,560, 10 | 19,718, 11 | 405 |
| 3080 → 3081 | a factor and its two moves | 908,521, 402 | 875,237, 447 | 2,203 |
| 3240 → 3243 | a factor and its two moves | not run | 1,576,693, 1,042 | 81,621 |
| 3475 → 3477 | a factor and its two moves | not run | 1,709,546, 1,141 | 36,503 |

Every run that ended did so at the optimum.  A swap of one move of the form p²q leaves almost nothing to search: 4901
in 750 steps where the chain took 2.3e8, 4693 in 30 steps where the default takes 572,354, 2527 in a seventieth of the
default's steps.  A swap of one move of three primes keeps its gadget alive deeper: 3367 takes 0.28 of the steps and
0.49 of the CPU, and the three stopped pairs are undecided.  A swap of a factor and its moves gains nothing (0.96 to
1.01 of the steps), as on 4899's units.  The study's check costs 15% to 75% more CPU a step on these games, since it
promotes from the root at every node.  The earlier games of the pairs take the later games' steps exactly where they
were run (2525 and 2527: 4,345,210; 3020 and 3021: 19,560).

**How often.**  `carry problems 2000 4900` and `carry dissolved` find 215 near repeats among games 2000 to 4900: 161
add a move (or a factor with two moves), restriction steps that the chain's ceiling settles with no search (every one
of them past 4780 in `strict.jsonl`); 40 swap a factor and its two moves; 13 swap one move, 11 of three primes and 2 of
the form p²q, as 4901 is.  Of the 60 games the chain searched from 4548 to 4901, the near repeats were 4805 and 4899
(a factor swapped, 9.8e7 and 2.7e8 steps) and 4823 and 4901 (one move, 8.7e7 and 2.3e8): 2.5% of the steps, 5.6%
leaving out 4875's search before conflict branching, of which the swaps of one move are 1.1% and 2.6%.

**What it would take.**  The cut is a tactic of the solver, not a lever of the lab: it closes subtrees.  The solver
would set up game n − 1's problem beside n's when n − 1 is proven and the seed is worth at least its P\*, check that
every number of both roots has the same home and factors, keep each node's state in n − 1's problem beside its
state in n's (a child's state promoted from its parent's, as the search does, where the study promotes from the root
at each node), and cut where the two states agree as above.  A swap of a factor with its moves keeps the two states
apart by the names of the swapped numbers; the ways to reach it are in the last section.

## 2026-10-22: where it stands

What carries from one game to the next, as measured here:

* **The whole search, across a near repeat.**  Consecutive games that pose the same candidates and P\* search the same
  tree, step for step, under any one setting.  Where the near repeat swaps one move, a cut that carries the previous
  game's proof ends the search almost at once (4901: 750 steps on one thread, against 2.3e8 on the chain's 88).
* **The better setting, across a stretch.**  The better gate held for every consecutive problem near 3200 and near
  3500, and turned between them.
* **The optimum, as a map.**  The hard units lie inside the optimum, which the previous game hands on; whether
  conflict branching off wins a unit follows how many of the optimum's members it has left out.

What does not carry, or does not pay: the obligation of the next proof restricted to the sets the last game could not
promote, on the steps that move one source (the new candidate's subtree keeps the root's gap); the members left out as
a switch at every node (2.6 to 3.2 times the steps on units, even on whole games).

Recommendations, best first:

1. **The near-repeat cut, as a tactic of the solver.**  Expected gain: nearly the whole search of each near repeat that
   swaps one move of the form p²q (4901 in 750 steps, 4693 in 30, 2527 in a seventieth of the default's), and part
   of it where the move has three primes (3367 in 0.28 of the steps; three such pairs undecided).  That is 1.1% of
   the chain's steps from 4548 to 4901, 2.6% leaving out 4875 (estimated from 4823 and 4901), and a whole game when one
   comes (4901: 1.4 hours of the 88-thread host).  Evidence: the whole games of the table above, the units, the
   argument and its check at random.  Cost: setting up n − 1's problem beside n's, its state kept beside each node's,
   the check, a bit of the checkpoint's tactics, and the tests of a cut (the soundness configurations at every check,
   the pairs above as tripwires).  The test that decides it: the production build on the pairs above, then the chain's
   next swap of one move.  The ways it might widen follow the list.
2. **A setting for the games to come** (the `policy` command above).  Expected gain: none alone; it is what lets any
   lesson that holds for a stretch be carried.  Evidence: the gate's stretches.  Cost: small, in `solver/lab.rs`,
   `window.rs` and `set_up_with_width`.  Test: the lab's unit tests and one game set up under a policy.
3. **Near repeats as paired tests of the levers, `conflict_look` first.**  Expected gain: what the lever is worth,
   measured on a whole game near 4900 at no cost; the other study measured 0.975 on whole games 3000 to 4050.
   Evidence: twins take equal steps.  Cost: nothing but the choice of setting for the second game of a pair.  Test:
   the next near repeat.
4. **The gate, chosen by stretch.**  Expected gain: 7% to 25% of the CPU in a stretch like 3500's, a loss of 16% to 53%
   in one like 3200's if chosen wrongly; unknown near 4900.  Test: a near-repeat pair near 4900, or whole games near
   4500 at 12 threads, at gates 150 and 300.
5. **A probe filtered by the members of the incumbent left out**: a few percent at most (units that have left out 5 or
   more hold 3% to 7% of the line); for the study of a per-node rule to weigh.

Ways the cut might widen, best first.  The shares are of the steps the chain searched since 4400 (its own records start
at 4548; the second figure leaves out 4875's search before conflict branching), and they are guesses unless said:

1. **"No harder" in place of "the same".**  Cut where every frame the node leaves in N is at least as hard as its twin
   in M: the same frame with moves added, factors taken away, or a factor added that pays only for the added moves.
   It fires as soon as the earlier game's swapped numbers are gone from the node's frames, whatever is left of the
   later game's, so it reaches every near repeat by a swap, the factor swaps (4805, 4899) included: 2.5% (5.6%),
   measured as the share of those games, against 1.1% (2.6%) for the cut as it is.  Proof: a winning order of the
   harder frame, with the added moves left out, wins the easier one, since every factor stays at least as available;
   and a factor that pays only for moves left out pays for nothing.  These are facts of the kind of Theorem 4 of the
   decision procedure, a page each.
2. **A renaming of the swapped numbers.**  Compare the two states up to a bijection of the numbers that are not
   candidates which keeps which factors pay for which moves.  It reaches the factor swaps higher in their trees than
   the first way: up to 1.4% (3.0%).  Proof: a bijection that keeps the frames' structure keeps winnability; finding
   it is a small matching problem around the swapped numbers.
3. **The steps where P\* rises** (most of the chain's steps: the steps that move P\* by more than 500 hold 70% of them).
   The cut needs only V ≥ P\*_M, so it holds there, but only at nodes that leave out the new candidate (a completion
   holding it is no set of M) and whose two changed frames are no easier than M's.  In N the source that became the
   candidate has left the moves of its frame, which makes that frame easier, so the cut fires only below nodes where
   M's frame has lost that move too.  Guess: under 5%, since the new candidate is in every heaviest set and the
   subtrees that leave it out are already the smaller ones.  Proof: the cut's, with the node's candidates within M's.
4. **Any proven bound, not only game n − 1.**  The argument needs only an upper bound at most V on the best
   promotion sum of some problem M whose frames match the node's: games two or three back, or any board whose bound is
   proven.  Chosen per game, it adds little beyond n − 1 (guess: under 1%); chosen per subtree, from a library of
   proven problems, it is a far larger idea whose share no measurement here bounds.  Proof: none beyond the cut's.

## Files

The runs were made on the chain's host in a clone of the solver at aca994e with the studies' commits 4f74667
(`TAXMAN_CB_GAP`) and f4a5756 (`TAXMAN_PROBE`) cherry-picked and `study-rules.patch` applied, built with `--features
studies`; the scripts name that host's folders.

* `carry.rs`: the example `carry` (copy it to `solver/frame/examples/` to build it): `dissolved M N` (the frames that
  change and the dissolved conflicts), `problems FROM TO` (each game's problem digest, candidates, `max_total`, P\* and
  root gap), `optimum N N` (the known optimal promotions in hex), `gadget M N` (what pays for the moves that differ),
  `states M N --units FILE` (whether each unit's promotions leave the same frames in both games), `verify M N --units
  FILE` (the check at random of the cut's lemma).
* `study-rules.patch`: the study rule `TAXMAN_CARRY_EXCL` / `TAXMAN_CARRY_REF` and the study cut `TAXMAN_CARRY_PREV`
  (`search/searcher.rs`, `search/mod.rs`, `examples/tune.rs`).
* `twins.py SNAP_A SNAP_B`: the units of two snapshots against each other and against the optima; `pairs.py SNAP_A
  SNAP_B OUT`: the twin units, written to `twins-4900-4901.txt` (4900's id, 4901's id, depth, both gaps).
* `shifts.py STRICT FROM TO`: the searched games by the shift of P\* from game n − 1, and their share of the steps.
* `verdicts.py SNAP RESULTS SETTING BUDGET`: the probe's verdicts of a study's units against the distance between them.
* `pick_twins.py`, `pick_excl.py`: the draws of units (`jobs-twins.txt`, `jobs-excl.txt`; `jobs-prev-units.txt` was
  drawn at random, seed 5, from the units of gap 600 to 1,299), lines of `unit.sh`'s arguments; `unit.sh SNAP ID
  SETTING` and `game.sh N SETTING [CPU]`: one unit or one whole game on one thread at `nice -n 19` (`xargs -P 8 -L 1
  ./unit.sh < jobs-twins.txt`).  A setting is `on`, `off`, `gGAP` (the gate), `pSTEPS_DEPTH` (the probe), `xK` (the
  members left out) or `cM` (the cut from game M).
* `twin_table.py`, `excl_table.py`: the tables of the twin and the stratified runs.
* `near-repeats-2000-4900.txt`: the near repeats of 2000 to 4900, the frames' moves and factors that differ.
* `units-results.jsonl`: every unit run (snapshot, id, setting, depth, gap, members of the optimum left out, steps, CPU
  seconds, whether it ended, whether its best is the optimum: always).
* `games.jsonl`: every whole game run (game, setting, steps, CPU seconds, whether it ended, whether its best is the
  optimum: always).
