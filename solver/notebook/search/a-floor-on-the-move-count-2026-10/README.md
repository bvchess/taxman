# A floor on the move count (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-20.  Concluded 2026-10-21.

**Question.**  The move count of the committed chain's optimal games never shrinks from n − 1 to n.  If "some optimal
game of n has at least as many moves as the game of n − 1" were a theorem, how many steps would a floor on the move
count save the frame solver's search?  **Answer.**  Little: the floor cut nodes in 1 of 35 searches measured and saves
no step in the other 34.  The one is 4176, where it takes 13.8% of the steps (14.7% of the CPU); on the five whole
games together it saves 2.6% of the steps, and on 30 units of the dearest games (3940, 4225, 4416 and 4548), 8.9
million steps, it saves none.  The floor has something to say only at a bound check where the heaviest set is short of
it, 747 of the 3.3 million checks of 4176 and none elsewhere: at every other check the heaviest set has the members that
the floor leaves the node to add, or one to spare.  The statement is unproven, so nothing is built; the probe is a patch
(`probe.patch`), and the entry measures what a proof would buy.  [The later measurement](#later-2026-10-21-more-searches)
adds the units and three more whole games.

## Method

**The data.**  In `results/strict.jsonl` (n = 2 to 4547), the recorded optimal game's move count, `len(moves)`, never
shrinks from n − 1 to n and never grows by more than one (`check_floor.py moves`, `chain-checks.txt`):

* 2,583 of the 4,546 steps stay the same and 1,963 grow by one;
* at the ceiling (1,462 games, score = n + score(n − 1)) the count always grows by one;
* below the ceiling 2,583 stay the same and 501 grow by one.

**Moves and promotions.**  In the frame model a game's moves are the moves of the game with no promotions
(`Board::plain`) and one move for each promotion: `accelerate::solution_for_promotions` treats each promoted number as a
source, and for a set the search accepts no source is lost, so the game scores `in_the_bag` plus its promotions.  The
count follows on every line of the chain: `len(moves) − len(promotions)` is the length of `Board::plain` for all 4,546
lines (`check_floor.py every`, with the example `floor_plain` that prints the length), the prime games and the games
taken from the bounds store included.

**The floor.**  So the move count of n is at least that of n − 1 exactly when the game of n has at least `F =
len(moves(n − 1)) − len(plain(n))` promotions.  On the chain `len(promotions(n)) ≥ F` for all 4,545 games n = 3 to 4547:
equal for the 2,583 games that keep their move count, and one above for the 1,962 that gain a move.  The two whole games
are of the first kind: 4176 has `F = 1808 − 1586 = 222` promotions and 4050 has `F = 1751 − 1536 = 215`.

**The probe** (`probe.patch`, against commit af1b0e9; never merged).  The floor is the environment variable
`TAXMAN_FLOOR`, off when it is unset.  In `check_gf2` (`src/search/searcher.rs`), after the ordinary cut of the bound, a
node of `p` promotions that can add `cap` more has to add `r = F − p`.  The bound's heaviest common independent set `I*`
of at most `cap` members, which the check has found worth at least `need`, has what the floor asks for if `r ≤ |I*|`,
and the floor says nothing.  Else:

* the weights of the heaviest common independent sets by size are concave, and `I*` is the heaviest of all sizes (it has
  fewer than `cap` members), so the heaviest set with at least `r` members is the heaviest with exactly `r`;
* Lawler's augmentations from `I*` give it, one member at a time, accepting paths that gain nothing or lose
  (`Meet::solve_at_least`, the solve of `src/gf2/meet.rs` with a least size; `Levels::heaviest_at_least` in
  `src/gf2.rs`);
* the node is cut, with a new cut kind (`cut::FLOOR`), if `r > cap`, if no augmenting path reaches `r` members, or if
  the set of `r` members weighs less than `need`.

The solve runs on a workspace of its own, from a fresh tableau of the level's vectors, and reads the level's live
candidates and its last history entry without changing them, so the history and the tableau that children reuse stay as
the search left them.

**Learning.**  The floor's cut learns nothing, as the ordinary cut of the bound learns nothing: both return from
`check_gf2` before it looks for the heaviest set's conflict, which is where a nogood is learned, and the callers that
end a node's loop or drop a child on a cut record nothing of their own.  So no nogood or conflict rests on the floor,
and the conflicts and nogoods of the nodes the floor leaves are facts about what can be promoted, as before.

**The counters** (`src/search/floor.rs`) are printed by `wall images --count`:

* `floor_open`: the bound checks that the ordinary cut let through;
* `floor_applies`: of those, the checks where `r > |I*|`, the ones the floor has something to say about; `floor_tight`
  counts the checks where `r = |I*|`, and `floor_spare` the members to spare over the checks where `r ≤ |I*|`;
* `floor_cuts`: the nodes cut, by their reason in `floor_over_cap`, `floor_no_set` and `floor_below_need`;
* `floor_solves`, `floor_augmentations` and `floor_ns`: the solves, their augmentations and their time.

**The checks of the probe.**

* Two unit tests against brute force (`cargo test --profile test --features studies --lib
  the_heaviest_set_of_at_least`). `solve_at_least` finds the heaviest common independent set of at least `r` members for
  900 random problems of up to 11 elements, a third of them bipartite matchings, with weights 1, 3, 9, ..., 243 (so that
  the heaviest set is often smaller than the largest), from the heaviest set and from the empty start, on a fresh
  workspace and on one that had solved other problems, and says so where no set has `r` members.  `heaviest_at_least`
  agrees with enumeration on over a hundred reweighted boards (to 200, at most 13 candidates, weights 1, 3, ..., 243)
  and leaves the level's evaluation as it was.
* With the floor off the trees are master's: `wall images --root --count` on 1215, 2000 and 3000 takes the same steps,
  tree nodes and leaves as the build of af1b0e9 (3,704, 32,513 and 167,554 steps; 1,471, 12,792 and 65,331 tree nodes;
  380, 3,234 and 15,828 leaves), and so does 4176 (below).  `small-games.jsonl` has the lines.
* With the floor one above the game's promotions, the root is cut for 60 of 60 searched games of 1000 to 4547 sampled
  (`rank_scan.py`, `rank-scan.txt`): 54 for want of a common independent set with that many members, 6 because the root
  can add fewer promotions.

**The runs** are those of the whole games of [the hardest
searches](../the-hardest-searches-of-the-run-to-5000-2026-10/README.md): `wall images --known results/strict.jsonl
--root --count --nodes 20000000 N`, from the build of `--profile test --features studies`, one thread, fresh tables,
from the known best set, under `nice -n 10` and `/usr/bin/time -l`, two at a time, each started with the one-minute load
at most 8 (it was 1.5 to 1.6 at each start), on the laptop on 2026-10-20.  The build is the probe's (`CARGO_BUILD_JOBS=2
cargo build --profile test --features studies --example wall`).  `run.py` makes a run and appends its line to
`runs.jsonl`: `python3 -I run.py WALL runs.jsonl 4176 [--floor 222] --label default`.  The steps and the CPU seconds are
those of `wall images`' and `time -l`'s (user plus system).

## Results

| game | floor | steps | CPU s | open bound checks | checks where the floor applied | nodes cut by the floor | floor's solves, s |
|---|---|---|---|---|---|---|---|
| 4176 | off | 5,342,669 | 1,678.7 | | | | |
| 4176 | 222 | 4,604,053 (−13.8%) | 1,432.1 (−14.7%) | 3,310,234 | 747 (0.023%) | 545 | 0.24 |
| 4050 | off | 3,086,223 | 989.1 | | | | |
| 4050 | 215 | 3,086,223 (0.0%) | 986.9 (−0.2%) | 2,342,964 | 0 | 0 | 0 |

Every run completed with the committed score (4176: best promotion sum 389,035, score 5,490,591; 4050: 368,462 and
5,160,836).  The two games together take 8.8% fewer steps (8,428,892 to 7,690,276) and 9.3% less CPU (2,667.8 to 2,419.0
s).  The F − 1 runs, for the form a weaker theorem would give, were not made: the saving is below the 25% that was to
trigger them, and a weaker floor applies at no check, and cuts no node, that the floor does not.

**Where 4176's saving comes from.**  The floor applied at 747 checks.  At each, the heaviest set was one member short of
the floor (747 solves, 747 augmentations), and the heaviest set of `r` members weighed less than `need` at 545 of them,
which are the cuts; at the other 202 it weighed enough and the search went on as before.  The 545 cuts removed 738,616
steps, 1,355 each, and tree nodes fell by 13.9% (2,110,845 to 1,818,464) and leaves by 13.3%.  The floor's 747 solves
took 0.24 s, 0.017% of the run, so the floor costs nothing here, and a step costs what it did (0.314 ms; 0.311 ms with
the floor on).  At the other 3,309,487 checks of 4176, and at all 2,342,964 of 4050, `I*` has exactly the `r` members
the floor leaves the node to add: no member to spare, and none short.

**The small games.**  At F the floor changes no tree on the five small games run with it (`small-games.jsonl`: 1215,
1500, 1580, 2000 and 3000, from 3,704 to 167,554 steps).  The games that gain a move (1215, 1500 and 1580) have one
member to spare at nearly every check (`floor_spare` is `floor_open`, but for 12 tight checks of 1580); the games that
keep it (2000 and 3000) have none.  At F + 1, which is the promotions of the three games that gain a move, only 1580
changes (9 checks where the floor applies, 5 cuts, 8 steps fewer of 6,499).  At F + 1 for the two that keep their count
the root is cut and the search ends at its first step: those games promote as many numbers as a search can at all.

**The master build against the hardest searches' table.**  The default search of 4176 takes 5,342,669 steps in this
build, where the table of [the hardest searches](../the-hardest-searches-of-the-run-to-5000-2026-10/README.md#results)
has 5,962,919, and 4050 takes the same 3,086,223 in both.  The build of af1b0e9 takes the same 5,342,669 steps on 4176,
with the same 2,110,845 tree nodes and 450,259 leaves and 20.0970 × 10¹² instructions against the probe's 20.0969 × 10¹²
(its 1,639 CPU s against the probe's 1,679: the cycles differ by 0.1%, so the 2.4% is the laptop's clock), so the probe
with the floor off is master's, and the difference from the table is master's: leaving the loops out of a child's live
candidates (1cef63c) moves the trees where strong branching ties, as [its
entry](../what-a-gf2-step-carries-from-the-last-same-trees-2026-10/README.md#2026-10-20-the-loops-left-out-adopted)
says, and its whole-game runs were not repeated.

## Decision

Measured, not built.  The floor is a cut that needs the never-shrinks statement, which nobody has proven, so the search
cannot use it; if it were a theorem, the cut at the bound's check would take 14% off the steps of the one game of two
where it applies, and 9% off the two together, for no measurable cost per step.  That is a small constant factor, below
the 25% that would have justified the run with a weaker floor, and it does not justify a proof, given that the [status
page](../../../docs/STATUS.md) puts the priority on a larger n with the least computation.  The reason it is small is in
the counters: the games that keep their move count promote as many numbers as the frames allow, and the bound's heaviest
set has that many members at nearly every node, so the floor repeats what the bound's set already holds.

What would reopen it: a proof of the statement that comes with other work; whole games where the heaviest set is short
of the floor more often (4176 has 747 checks of 3.3 million); or a use of the floor beyond the cut that the probe tried,
such as the members without which the set of `r` members weighs less than `need` (forced members), which was not
measured.  [The later measurement](#later-2026-10-21-more-searches) found the same on 30 units and three more whole
games: the decision stands.

## Later (2026-10-21): more searches

**Question.**  Is 4176 typical of the dear searches?  **Answer.**  No, it is the exception, and the floor does not save
10% across the dear searches: it cut nodes in 1 of 35 items, 4176, and saved no step on 30 units of the dearest games
and 2.6% of the steps of the five whole games.  Two more whole games, 4017 and 4060, were stopped unfinished by the
owner's decision, as the floor had applied at no check of the 34 other items finished.

**The items.**  Run as the whole games above, with the floor off and on:

* *The units* of [the hardest searches](../the-hardest-searches-of-the-run-to-5000-2026-10/README.md): the 30 of
  `suite-hard.jsonl`, the dearest games' (3940: 6 units, 4225: 5, 4416: 9, 4548: 10), of 104,080 to 1,048,736 steps in
  the suite's probes.  A unit is solved with `wall images --suite ... --ids ID --count` from the game's known best set,
  or for 4548, which has no line in the chain, from the incumbent of the suite's `game` line.
* *Whole games* from the ranking's smallest searches: 4005, and 4004 and 3978.  The searches of 3 to 15 million nodes
  all lie in 3978 to 4176, so the whole games cannot spread over 3915 to 4547; the dearest games, which a laptop
  cannot run, are the units' games.  4017 and 4060 were planned too (below).
* *The floor* of a game is F as above, from the chain: 206 for 3940, 223 for 4225, 234 for 4416, 210 for 4005, 211 for
  4004, 208 for 3978.  The chain has no line for 4548, so its F is the moves of 4547 less the plain moves of 4548, 236,
  which holds if the statement does.  `plan.py` makes the plan, `batch.py` runs it two runs at a time under `nice -n
  10` and `/usr/bin/time -l`, each started with the one-minute load at most 8 (1.7 to 7.6 at the starts).

**The check of `p`.**  The floor's `r = F − p` counts the promotions on a unit's path from the game's root, as built.  A
unit's `promoted` is the set promoted on that path (the suite's `depth` is its size), the searcher's node starts from
it, and `promoted.len()` in `check_gf2` counts all of it; the node's cap is `max_total` less the same count
(`max_additional`).  To make sure, the probe gained two counters (`probe-first-check.patch`): the promotions and the
cap of the first check the floor sees.  For all 30 units the first check has the unit's depth as its promotions, and
`max_total − depth` as its cap (`check_units.py`, `check-units.txt`).  Nothing needed fixing.

**The floor-off runs left out.**  A floor that applies at no check cuts nothing, so its search is the floor-off search.
The units of 4548, and 4004 and 3978, were run with the floor on first; the floor applied at none of their checks, so
no floor-off run was made, and their rows take the floor-on steps (CPU is compared only where both ran).  This holds in
the 21 items of this section where both ran, the 20 units of the other three games and 4005: the floor-on steps, tree
nodes and best are the floor-off ones, and the CPU differs by −0.3% to +0.4%, the laptop's noise.  For 4548's ten units
the floor-on steps and best are also those of the suite's default probes.  So the ten units of 4548, 4004 and 3978 are
one run each, and the other 20 units and 4005 two.

**Results.**  A row of the units is their sum; `later-tables.txt` has a row for each unit, and `later_table.py` makes
both (`python3 -I later_table.py all`).  The rows of 4176 and 4050 are the first table's.

| items | n | floor | steps, off | steps, on | steps | CPU s, off | CPU s, on | CPU | open bound checks | checks where the floor applied | nodes cut |
|---|---|---|---|---|---|---|---|---|---|---|---|
| units of 3940 | 6 | 206 | 1,840,777 | 1,840,777 | 0.0% | 453 | 453 | 0.0% | 1,296,050 | 0 | 0 |
| units of 4225 | 5 | 223 | 808,070 | 808,070 | 0.0% | 261 | 261 | 0.0% | 609,728 | 0 | 0 |
| units of 4416 | 9 | 234 | 2,951,022 | 2,951,022 | 0.0% | 930 | 930 | 0.0% | 2,160,825 | 0 | 0 |
| units of 4548 | 10 | 236 | 3,283,394 | 3,283,394 | 0.0% | | | | 2,587,985 | 0 | 0 |
| 4176 whole | 1 | 222 | 5,342,669 | 4,604,053 | −13.8% | 1,679 | 1,432 | −14.7% | 3,310,234 | 747 | 545 |
| 4050 whole | 1 | 215 | 3,086,223 | 3,086,223 | 0.0% | 989 | 987 | −0.2% | 2,342,964 | 0 | 0 |
| 4005 whole | 1 | 210 | 5,723,023 | 5,723,023 | 0.0% | 1,796 | 1,796 | 0.0% | 4,242,818 | 0 | 0 |
| 4004 whole | 1 | 211 | 6,721,978 | 6,721,978 | 0.0% | | | | 4,954,983 | 0 | 0 |
| 3978 whole | 1 | 208 | 7,683,724 | 7,683,724 | 0.0% | | | | 5,455,197 | 0 | 0 |

Every run completed with the committed score (the known best of its game, and for 4548's units the incumbent they
start from, 454,465, as the default search ends).  The floor made no solve in these runs, so it cost nothing; its 747
solves at 4176 took 0.24 s.

**The three totals.**

* *Steps saved over all the units together:* none, 0 of 8,883,263 (30 units, 4 games).
* *Steps saved over all the whole games together:* 738,616 of 28,557,617, 2.6% (5 games).  Without 4176 it is none, 0 of
  23,214,948.
* *Items with any cut:* 1 of 35 (4176).

**Why the units and most games save nothing.**  The counters of the floor-on runs split the games in two:

* In the games that keep their move count from n − 1 (3940, 4050, 4004, 3978 and 4176 by the chain, 4548 by its
  counters) the heaviest set `I*` has exactly the `r` members the floor leaves a node to add at every check:
  `floor_tight` is `floor_open`, but for the 747 checks of 4176 where `I*` is one short.
* In the games that gain a move (4225, 4416, 4005) the chain's game has F + 1 promotions, and `I*` has a member to spare
  at nearly every check: `floor_tight` is 0 for 4225's units, 4005 and six of 4416's nine units, and 1,324, 6 and 102
  of their 434,756, 248,167 and 122,588 checks for the other three.  The floor says nothing there, and a floor of F + 1,
  which would, needs to know in advance that a game gains.

So the floor can cut only in the games that keep their count.  They carry 59.6% of the nodes of the twenty dearest
searches of 3915 to 4547 and 66.3% of all 150 (`check_floor.py keeps`, `keeps.txt`), and in them the floor cut in 1 of
the 4 whole games and in none of the 16 units of 3940 and 4548.

**Stopped.**  Two floor-on runs of whole games, 4017 (F = 211) and 4060 (F = 216), both of games that keep their count,
were stopped before they finished, at 5,157 and 1,151 CPU s by the last look (`later-runs-stopped.jsonl`), on the
owner's decision that the evidence was enough: the floor had applied at no check of the 34 other finished items.  4017
had taken more than twice the 2,400 s that its 7.5 million nodes on 64 threads would cost at the 0.32 ms a step of these
runs.  Neither has a line, and the verdict does not rest on them.  The finished runs took 11,932 CPU s.

**Verdict.**  The floor does not save more than about 10% across the dear searches.  A proof of the statement would buy
a cut that matters at 4176 (14% of its steps) and nowhere else in 35 items: 2.6% over the five whole games and nothing
over the 30 units, which is not worth a proof by itself.  The sample is small (five whole games, the units of four
games), and the cut happens only where the heaviest set is one short of the floor, which these searches show rarely: at
20.3 million checks of whole games the floor applies at 747, all in one game.  What would reopen it is as in the
Decision; the units, the nearest thing to the dear games that a laptop runs, are untouched.

## Files

* `README.md`: this entry.
* `probe.patch`: the probe, against af1b0e9 (`git apply probe.patch`): `src/search/floor.rs` (the floor, `TAXMAN_FLOOR`,
  its counters), the cut in `check_gf2`, `Meet::solve_at_least` and `Levels::heaviest_at_least`, their two tests, and
  the example `floor_plain`.  The code stays off master.
* `run.py`: one run of `wall images --count` on a whole game or a unit with the floor set or not, waiting for a quiet
  machine.
* `runs.jsonl`: the four runs of the table, one line each (`wall images`' line with the floor's counters, `time -l`'s
  numbers and the load).  `table.py` makes the table from it.
* `identity.jsonl`: the build of af1b0e9 on 4176, by `run.py` with the master build's `wall`.
* `small-games.jsonl`: the five small games, each with the floor off, at F and at F + 1.
* `check_floor.py`: the data under the floor (`moves`, `plain`, `floor`, `every`, `keeps`); `chain-checks.txt` is the
  output of `moves` and `every`, `keeps.txt` of `keeps`.
* `rank_scan.py`, `rank-scan.txt`: the root cut at promotions + 1, for 60 sampled games.

The later measurement's files:

* `probe-first-check.patch`: the two counters of the first check (`floor_first_promoted`, `floor_first_cap`), on top of
  `probe.patch`.
* `plan.py`, `plan1.jsonl`, `plan2.jsonl`: the plan, as a line a run: `plan1.jsonl` has the units of 3940, 4225 and 4416
  and 4005, each with the floor off and on (`plan.py PLAIN_BIN --units 3940,4225,4416 --wholes 4005`); `plan2.jsonl` has
  the floor-on runs of the units of 4548 and of 4004, 4017, 3978 and 4060 (`--units 4548 --wholes 4004,4017,3978,4060
  --labels on`).
* `batch.py`: makes a plan's runs two at a time and appends their lines to `later-runs.jsonl`: 54 lines, with the
  floor's counters, `time -l`'s numbers and the load.  `later-runs-stopped.jsonl` records the two runs stopped.
* `check_units.py`, `check-units.txt`: the check that a unit's path counts toward `p`, for the 30 units.
* `later_table.py`, `later-tables.txt`: the tables (`python3 -I later_table.py all`), with a row for each unit.
