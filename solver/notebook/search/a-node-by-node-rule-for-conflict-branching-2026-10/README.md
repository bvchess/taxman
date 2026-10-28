# A node-by-node rule for conflict branching (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-27.  Concluded 2026-10-28.

**Question.**  Is there one rule, applied node by node, for when a node branches on the conflict that conflict branching
chooses rather than the search's own, that takes at least 5% less CPU than the chain's gate of 1,200 on the units of the
hard games, weighted by each game's share of the chain's search time, with no game worse by more than about 5%?

**Answer.**  No rule tried clears that bar.  The best, a *floor* that keeps the search's own conflict unless it weighs
at least 3, takes 0.87 and 0.90 of the CPU on the units of games 4983 and 4984 that finished, but 1.04 to 1.18 on the
units of games 4991, 4992 and 4995: its threshold tells apart the conflicts at the top of 4983 and 4984 and not those of
the later games.  Every other rule is within the noise of the gate of 1,200, or worse.  Two findings explain why the
question is hard: a unit's steps change two to five times either way with any change of the choice near its top, and the
trees that conflict branching makes have steps about 10% cheaper than the trees of the search's own conflicts.

## Method

*The settings.*  Every run has the chain's tactics: `price_reuse=on`, `warm_start=on`, the look on, and the carried
store of each game but 4983, which has none (4984: the store of 144 units of 4983; 4991, 4992 and 4995: the chain's
stores of games 4984, 4991 and 4992).  The reference, *gate 1200*, is conflict branching from a gap of 1,200, as the
chain runs since game 4995; *off* is conflict branching off.  A *weight* is conflict branching's estimate of a
conflict's subtree against its node's, the sum over its members of 10^(−price/291) ([search.md, "Conflict
branching"](../../../docs/design/search.md#conflict-branching)); `w0` is the weight of the search's own conflict `K`.

*The panels.*  Units of each game's checkpoint, each solved to the end on one thread by `tune solve`, stopped at 150,000
steps, all with a gap of at least 1,150, where a rule at the gate of 1,200 can act:

* the training panel, 93 units: 29 of games 4983 and 4984 from the first samples (gaps 1,216 to 1,925), and 34 of 4983
  and 30 of 4984 picked at random by gap band (1,113 to 1,676);
* the fresh panel, 80 units: 40 of 4983 and 40 of 4984 picked at random by gap band (1,150 to 1,700), none used before
  the rule was chosen;
* the panel of the later games, 87 units: 42 of 4991, 26 of 4992 and 19 of 4995, those of the lab's samples of [the
  settings checks](../the-root-gap-and-the-size-of-a-game-2026-10/README.md) with a gap of at least 1,150.

*Two kinds of run.*  Steps are exact, and come from runs eight at once, the settings one after another on each unit.
CPU comes from *pairs*: the two settings of a unit solved at once on the two hardware threads of one core, four cores at
a time.  A pair of two runs of the same tree gives a ratio of 1.000 within 0.5%, while the CPU of unpaired runs drifts
by 10 to 30%.  The rules were knobs of one build, read from the environment (`study.patch`, against 29aa8ba); the floor
was then built as a tactic (`floor.patch`, against 7140548), which reproduced the knob's steps exactly on six units.

## Results

**1. A unit's steps are chaotic in the choice near its top.**  A gate of 1,150 or 1,250 in place of 1,200, which moves
only which nodes near the top choose, takes 1.06 and 1.02 of the steps on 29 units, and single units 0.48 to 2.51 times.
Over 70 to 80 units that finished, the 95% bootstrap interval of a rule's total steps is ±10 to 15%.  So a rule's effect
shows only on many units, and a rule fitted to a few units fits their noise.

**2. Conflict branching's trees have cheaper steps.**  On eight units of 4983 stopped at 150,000 steps under both
settings, the gate of 1,200 took 0.909 of off's CPU (0.81 to 1.02 a unit), in pairs.  Pricing alone does not do it: the
choice made and then thrown away, keeping the search's own conflict everywhere, takes 1.000 of off's CPU, with off's
trees.  The trees differ in how many steps make no GF(2) evaluation (a child that fails or is cut on sight): 46% of the
steps of gate 1200 on eight units, 40% of off's.  Over the training panel, a step of gate 1200 cost 0.513 ms and one of
off 0.570 ms.  So a rule that cuts the steps but leaves more of them to the search's own conflict gives back much of the
gain in the cost of a step.

**3. The rules on the training panel.**  Steps against gate 1200 on the units both finished, and CPU in pairs where it
was measured:

| rule | units | steps [95% interval] | geometric mean | CPU in pairs: finished, all |
|---|---|---|---|---|
| off | 71 | 0.947 [0.807, 1.106] | 0.881 | 0.895, 1.009 (fresh panel) |
| another conflict only if its weight is below 0.5 `w0` | 76 | 0.956 [0.862, 1.046] | 0.919 | 0.992, 0.973 |
| the same, below 0.3 `w0` | 72 | 0.928 [0.814, 1.053] | 0.863 | |
| the same, below 0.7 `w0` (29 units) | 29 | 0.974 | 0.882 | |
| the floor: another conflict only if `w0` is at least 3 | 76 | 0.920 [0.818, 1.023] | 0.867 | 0.941, 0.955 |
| gate 1,000 | 72 | 1.214 [1.102, 1.339] | 1.270 | |
| gate 1,400 | 71 | 0.989 [0.850, 1.137] | 0.925 | |
| below 0.5 `w0`, gate 900 (29 units) | 29 | 1.075 [0.900, 1.286] | 0.964 | |
| prices in context, from the split's child bound (29 units) | 29 | 0.999 | 0.999 | |

* *A relative margin* (another conflict only if it weighs less than a share of `w0`) cuts 3 to 7% of the steps and gives
  it back in the cost of a step: 0.97 to 0.99 of the CPU.
* *A lower gate loses.*  Gate 1,000 takes 1.21 of the steps; 900 with the margin 1.08.
* *Prices in the context of each child* weigh each member of a conflict by the bound of its child, with the members
  before it promoted.  Bounded by the split's child bound alone, they changed one tree of 29 (the table's last row).
  With each child made as the look makes it, and solved where the split does not bound it, they changed two units' trees
  by at most 114 steps (4983-1406: 26,319 against 26,331; 4983-3319: 46,867 against 46,981), and the pricing cost 7 to
  8% of a step: at the top of 4983-1406 the search's own conflict, whose two members of price 8 the weight counts as
  nearly the whole gap, keeps them cheap in context too.
* *The floor* came from the first choice of 49 units of the training panel: where the search's own conflict at the top
  weighed at least 3 (nine members, three of them cheap, `w0` 3.25 in many units), conflict branching won; where it
  weighed about 2.5 (seven members, two of them of price 8), the search's own conflict won, in 4983-1406 in 5,074 steps
  against 26,331.

**4. The floor on fresh units and on the later games.**  CPU in pairs against gate 1200, on the units that finished and
on all of them (a unit stopped at 150,000 steps under both counts its steps' cost alone):

| panel | game | units finished | steps | CPU, finished | CPU, all |
|---|---|---|---|---|---|
| fresh | 4983 | 25 of 40 | 0.860 | 0.887 | 0.977 |
| fresh | 4984 | 28 of 40 | 0.671 | 0.716 | 0.900 |
| all three | 4983 | | 0.852 | 0.870 | 0.956 |
| all three | 4984 | | 0.868 | 0.899 | 0.935 |
| later games | 4991 | | 1.049 | 1.043 | 1.026 |
| later games | 4992 | | 1.101 | 1.111 | 1.044 |
| later games | 4995 | | 1.178 | 1.181 | 1.047 |

On the fresh panel the floor took 0.749 [0.605, 0.897] of the steps, better on 21 units and worse on 8.  On the later
games it took 1.087 [1.012, 1.182], better on 15 and worse on 26.  Weighted by the chain's search time (4983 0.39, 4984
0.50, 4991 0.03, 4992 0.06, 4995 0.02), the floor takes 0.910 of the CPU on the units that finished and 0.955 on all,
but it makes 4995 worse by 18% and 4992 by 11% on the units that finished.  The later games show why: at the top of
their units the search's own conflict often weighs 2.0 to 2.3, and there conflict branching's choice wins.  4995-640 has
two members of price 8 in its own conflict ([326, 8, 8, 163]), as the units of 4983 do, and conflict branching closed it
in 51,293 steps against 87,384.  So the floor's threshold separates the tops of 4983 and 4984, not a kind of node.

**5. Strong branching's cost bounds reliability branching.**  On four units under gate 1200, a step made 0.28 to 0.32
strong branchings, on conflicts of two members (32 to 48%), three, four and more; they took about 10% of the CPU, two
thirds to three quarters of it in the bounds read off the exchange graph.  At most 9 members of a unit were solved, and
a forced member was found in at most 7 calls of a unit.  The member the split ranks first was the one chosen in 57 to
69% of the calls.  So pseudo-costs could save at most about 10% of the CPU, and the cheap predictor at hand misses a
third of the choices; not built.

## Decision

Not kept: no rule clears the bar.  The floor is not adopted: its gain on the panel comes from games 4983 and 4984, whose
tops the chain has passed, and it makes the later games worse.  The tactic `conflict_floor` (`floor.patch`: the
constant, the tactic off by default and recorded in no checkpoint, its tests and docs) stays here, for a game whose top
looks like those of 4983 and 4984.  It was built in release and debug builds and run on games 1000 and 2000 and on
units; its tests were not run.

What would reopen it: a signal at the node that tells the own conflicts of 4983's tops (two members of price 8 among
seven) from those of 4991 to 4995 (two members of price 8 among four, or one among five), where the two choices win; or
a whole-game measure, since units of 150,000 steps leave out the largest subtrees, where a game spends most.

## Files

The runs are in `~/taxman_work/lab/work/branch-2026-10-27/` on the machine of the chain (a folder a run: `out-*/`
unpaired, `pair-*/` paired, a result file and a log a solve; `bin/` the builds).  Copied here:

* `study.patch`: the knobs of the study, against 29aa8ba: `TAXMAN_BR_RHO` (the relative margin), `TAXMAN_BR_W0MIN` (the
  floor), `TAXMAN_BR_CTX` and `TAXMAN_BR_CTXSOLVE` (prices in context), `TAXMAN_BR_SLOPE`, `TAXMAN_BR_LOOKPC`, and the
  counters `TAXMAN_BR_TRACE` (a line a choice), `TAXMAN_BR_PHASES`, `TAXMAN_BR_GAPPROF` (steps and time by gap and
  depth) and `TAXMAN_BR_SBSTAT` (strong branching).
* `floor.patch`: the tactic `conflict_floor`, against 7140548.
* `run.sh SETFILE IDS OUT NODELIMIT` (unpaired, eight at once) and `pair.sh SETFILE IDS OUT REPS NODELIMIT` (paired); a
  line of SETFILE is `NAME BINARY ENV ARGS`.
* `collect.py` (a table of results), `cmp.py` (settings against a reference, by game), `stats.py` (bootstrap intervals
  of the steps), `restrict.py` (a table limited to a list of units), `feat.py` (the first choice of each unit against
  its outcome), `tsum.py` (the trace), `phases.py`, `gapsum.py` (the counters), `pickc.py` (the fresh panel).
* `runs.txt`: every run, a line each: its folder, setting (`#1` a paired run), unit, gap, steps, CPU seconds, whether it
  finished, and its best.

## Commits

This entry and its register rows.  No code changed.
