# The headroom of branching, in hindsight (2026-10)

Part of the [lab notebook](../README.md).

**Question.**  How much smaller could a hard unit's tree be if the search branched on other members, chosen in
hindsight, and is there a cheap signal that picks them?

**Answer.**  Little, and no.  Where hindsight can be measured cleanly, in a search without the nogood table, the best
members at the top three levels of 3940-104 make its tree 16% smaller, all but 1.4 points of it at the root, and the
members of the heaviest set's other conflicts do no better.  With the learning, hindsight at the root finds 0-38% on
three units, but most of it is the table's history, not the choice: changing only the period at which the table
forgets moves strong branching's own trees by −26% to +17%, and under six such histories the root's best member of
4416-95 saves 5% on average.  One choice holds up, 3940-104's root member 1350, which saves a third under every
history; but nothing the search sees at that node or in the child's first check sets it apart.  The swings of ±50%
that small changes cause are this noise, around a branching that hindsight cannot beat by much: the bound is what
matters.

The question was asked because two changes that leave the bound alone, keeping the nogoods used at a clearing and
learning more nogoods in twin contexts, moved trees by −63% to +197% ([what the learning
leaves](../what-the-learning-leaves-in-the-hardest-searches-2026-10/README.md)).  If such swings came from better or
worse branching, the gap between strong branching and the best branching could be large.

## Words

* A *decision* is a step at which strong branching found no member of the node's conflict forced, so the node may
  branch on any member: it promotes the member in a child, then goes on without it.  Any member keeps the search
  exact.  (A node with forced members has no choice: its child promotes them all.)
* A decision's *subtree* is the steps from it to the end of its node's loop.  Its *child* is the part under the member
  promoted; its *rest* is the part after, where the node goes on without the member.
* *Hindsight at level 1* solves the unit once for each member of its first decision's conflict and keeps the member
  whose subtree is smallest.  *Level L + 1* does the same at the biggest decisions in the child and the rest of each
  decision of level L, with the choices made so far: the greedy best at the top L levels.
* A search's *history* is what its nogood table holds when it reaches a node.  It depends on everything the search
  did before, so a node's subtree can change when nothing in it changed.  A *history-free* search runs without the
  table (`--tactic nogoods=off`): each node's subtree is then a function of the node alone, and hindsight is exact.
* A *butterfly* is one small decision changed: a decision of the first fifth of the steps whose subtree holds 20 to 300
  steps, branched on another member of its conflict, and the unit solved to the end.
* *Unit*, *price* and *core* are [the anatomy's](../the-anatomy-of-a-hard-subtree-2026-10/README.md#words); *the
  learning* is `wall images --learn --closure` ([what the learning
  leaves](../what-the-learning-leaves-in-the-hardest-searches-2026-10/README.md#words)); the other terms are the
  [glossary's](../../../docs/GLOSSARY.md).

## What was run

**The instrument** is `wall headroom`, a study command kept as a patch (`headroom.patch`, against commit 620a72a), not
in the code.  It solves a unit as `wall images` does: one thread, fresh tables, from the known optimum, with or without
the learning.  Its hook (`search::StepHook::branch`) records every decision, keyed by its node's promotions (no two
nodes of a tree have the same) and its place in the node's loop, and branches where a plan says.  It runs the base,
the butterflies and the levels of hindsight; the module comment of `examples/wall/headroom.rs` in the patch has the
options and the lines.  Every full solve completed with the known optimum as its best, so every tree here is a proof.

**The units** are the smallest with the learning of each game with a known optimum in `suite-hard.jsonl` of [the
hardest searches of the run to 5000](../the-hardest-searches-of-the-run-to-5000-2026-10/README.md):

| | 3940-104 | 4225-4.24 | 4416-95 |
|---|---|---|---|
| gap | 1,060 | 1,001 | 1,210 |
| steps with the learning | 51,790 | 44,826 | 89,884 |
| steps of the default search | 104,080 | 194,716 | 119,309 |
| steps without the table | 58,623 | unfinished at 10 CPU minutes, ten times the default's | not run |
| the first decision: depth, conflict | 0, 11 members | 1, 2 members | 0, 21 members |

All ran on this laptop on 2026-10-20, under `nice`, one solver at a time, from the build of `--profile test`: about
2 CPU hours in all, nothing on EC2.

## Results

**1. With the learning, hindsight at the root finds 0-38%, and deeper levels do not add up.**

| | 3940-104 | 4225-4.24 | 4416-95 |
|---|---|---|---|
| strong branching's tree | 51,790 | 44,826 | 89,884 |
| the root's best member: its tree | 1350: 32,324 (0.62) | strong's (1.00) | 1800: 69,291 (0.77) |
| the root's members, median | 1.00 | the other member over 3 | 1.00 |
| the plan of levels 1-2 | 0.50 | 2.01 | 0.77 |
| the plan of levels 1-3 | 0.97 | 2.00 | not run |

Each level's choices were the best for their own subtrees, yet the plans of levels 2 and 3 doubled 4225-4.24's tree
and gave back 3940-104's gains.  At 4225-4.24, choosing 1520 over 1824 at a decision of 79 steps saved 19 steps there
and more than doubled the rest of the tree, which it does not contain: the next decision of the root, whose
subtree had been under 44,742 steps, took 108,111.  The nogoods learned in those 79 steps changed which conflicts the
later nodes branched on.  (These levels took the first decision of each part; the instrument in the patch takes the
biggest ones, as the history-free run below did.)

**2. Small changes move the rest of the tree, through the table.**  Of 24 butterflies with the learning, four moved
the tree by more than 5%: 4225-4.24's first one by ×2.41, and three of 4416-95's by ×0.87 to ×0.93.  The default
search does the same: three of eight at 3940-104, by ×0.77 to ×1.13.  Without the table the four butterflies of
3940-104 moved the tree by exactly their own change (58,623 less 29 plus 27, and so on), as a history-free search
must.

| search | unit | butterflies that move the tree by more than 5% | the tree, against the base |
|---|---|---|---|
| with the learning | 3940-104 | 0 of 8 | 0.998-1.034 |
| with the learning | 4225-4.24 | 1 of 8 | 1.000-2.413 |
| with the learning | 4416-95 | 3 of 8 | 0.873-1.000 |
| default | 3940-104 | 3 of 8 | 0.767-1.129 |
| without the table | 3940-104 | 0 of 4 | exactly additive |

The forgetting period is a cleaner probe of the noise, since it changes no choice at all.  With the learning and the
periods 10,000, 20,000, 30,000, 40,000 and 50,000 steps, or no forgetting, strong branching's own tree ranges over
38,152-60,846 steps at 3940-104 and 69,538-100,813 at 4416-95.

**3. Without the table, hindsight at the top three levels saves 16%, mostly at the root.**  At 3940-104 the best
members make the tree 0.855 of strong branching's at level 1, 0.850 at level 2 and 0.841 at level 3; the history-free
search is exact, so the plans' totals are the sums of their slots' savings.  The root's best is 1452 (price 152);
strong branching's 1936 (price 832) ties with three other high-priced members at exactly 58,623 steps.  The heaviest
set at the root holds four disjoint minimal conflicts (`wall anatomy`'s cores).  Branching on a member of another one
does no better: the best of each of the other three is 1.68, 1.00 and 1.00 of strong branching's tree, against 0.86
for the core the search computes.  The other units could not be measured this way: 4225-4.24 without the table was
unfinished after ten times the CPU of its default search.

**4. Under other histories, one root choice holds up and the other does not.**

| forgetting | 3940-104: strong; 1350; 1452 | 4416-95: strong; 1800 |
|---|---|---|
| every 50,000 steps (the default) | 51,790; 0.62; 1.26 | 89,884; 0.77 |
| every 10,000 | 48,876; 0.70; 0.58 | 100,813; 1.03 |
| every 20,000 | 38,152; 0.67; 1.03 | 92,816; 0.92 |
| every 30,000 | 39,006; 0.81; 1.57 | 84,339; 0.92 |
| every 40,000 | 45,374; 0.71; 1.65 | 71,303; 1.07 |
| never | 60,846; 0.53; 1.21 | 69,538; 0.99 |
| geometric mean of the ratios | 0.67; 1.15 | 0.95 |

At 3940-104, 1350 beats strong branching under every history, by a third on average, and its child takes
10,038-10,063 steps every time, against 38,000-61,000 for the child of strong branching's 1936.  1452, the best
without the table, is erratic with it.  At 4416-95, 1800 saves 5% on average, and 23% only under the default period.

**5. No cheap signal.**  At 3940-104's root, with the learning, every member's child looks the same at its first
check: the gap stays at 1,060, nothing is forced, and the child's conflict is the other ten members.  So neither the
child's bound, nor its gap, nor its forced members, nor the gap-based estimate of its subtree can tell 1350 apart.
The member's own features do not either:

* *price*: the high-priced members tie with strong branching (at 4416-95, six of the other 20 members give exactly
  its 89,884 steps), the cheapest are the worst (×1.2 to over ×3), and the middle is mixed: 1350 (324) saves a
  third, 1692 (351) costs 6%, 1452 (152) costs 26%;
* *home*: the good and the bad members share homes 3 and 4;
* *twins among the candidates*: 3 for 1350 and for 1500, which ties with strong branching; 3 for 1800 and for 2025 at
  4416-95, one better and one equal;
* *the conflict*: the search's own core was the best of four at 3940-104.

A rule needs a signal that holds across units, and the one robust gain found here, on one unit of three, has none, so
no rule was tested.

## Decision

Branching is not where the next cloud run's cost can come down.  Strong branching is within about 15% of the best
branching hindsight finds on the one unit where hindsight is exact, and the larger gains hindsight finds with the
learning are mostly the table's history, which no choice controls.  The swings of earlier studies (keeping the used
nogoods, learning in twin contexts) are likely of the same kind: changes to what the table holds, which move a tree by
up to a factor of two either way.  A study that compares two searches by one unit's tree should therefore compare them
under several histories, such as several forgetting periods: the period alone moves a tree by up to a quarter.  The
same entry that made these units found that the learning's effect on units does not carry to whole games of this era
(4176 −13% CPU, 4050 +31%), so a rule that did pass on units would still have to be judged on 4050 and 4176.  What
shrinks a tree is a stronger bound, as [the anatomy](../the-anatomy-of-a-hard-subtree-2026-10/README.md) and
[what the learning leaves](../what-the-learning-leaves-in-the-hardest-searches-2026-10/README.md) concluded.

`wall headroom` is not kept in the code; the patch keeps it.

## Files

* `headroom.patch`: the study's code against commit 620a72a: `examples/wall/headroom.rs` (the command), its options
  in `examples/wall.rs`, `wall anatomy`'s cores shared, and `StepHook::branch` widened to any candidate of the node,
  which a study of the choice of conflict needs (the searcher asserts it).  It applies to 8019e71 as well.  Apply it
  with `git apply`, then build with `cargo build -j 2 --profile test --example wall --features studies` in
  `solver/frame`.
* `runs.jsonl`: every line of `wall headroom` of this entry, with two fields added: `label` (`levels`, `root`,
  `conflicts`, `butterflies` or `perturbed`) and `tactics` (the learning and the tactics of the run).
* `headroom.py`: the tables of this entry, from `runs.jsonl` (`python3 -I headroom.py`).
* The commands, from `solver/frame`, with `SUITE` the suite of the run to 5000 and `../../results/strict.jsonl` as
  `--known`:
  * `levels`: `target/debug/examples/wall headroom --known K --suite SUITE --ids ID --learn --closure --levels 3 N`
    (4416-95 was stopped after level 2), and `--tactic nogoods=off --butterflies 4 --levels 3` without the learning
    for 3940-104;
  * `root`: the same with `--levels 1`, after the child's check was added to the lines;
  * `conflicts`: `--ids 3940-104 --tactic nogoods=off --conflicts --levels 1 3940`;
  * `butterflies`: `--learn --closure --butterflies 8`, and `--butterflies 8` alone for the default search;
  * `perturbed`: `--learn --closure --levels 1 --only 1350,1452` (3940-104) or `--only 1800` (4416-95), with each
    `--tactic nogood_period=P` for P of 10,000-40,000, and `--tactic forget_nogoods=off`.
* The history-free solve of 4225-4.24 was stopped after 10 CPU minutes, without a line.

Commits: the commit of this entry.
