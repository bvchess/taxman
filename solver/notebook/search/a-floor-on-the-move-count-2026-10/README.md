# A floor on the move count (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-20.  Concluded 2026-10-20.

**Question.**  The move count of the committed chain's optimal games never shrinks from n − 1 to n.  If "some optimal
game of n has at least as many moves as the game of n − 1" were a theorem, how many steps would a floor on the move
count save the frame solver's search?  **Answer.**  On two whole games it saves 8.8% of the steps and 9.3% of the CPU,
and the saving is all in one of them: 13.8% of the steps of 4176 (14.7% of its CPU) and none of 4050.  The floor has
something to say at 747 of the 3.3 million bound checks of 4176 and at none of the 2.3 million of 4050: at every other
check the bound's heaviest set has exactly the members that the floor leaves the node to add.  The statement is
unproven, so nothing is built; the probe is a patch (`probe.patch`), and the entry measures what a proof would buy.

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
measured.

## Files

* `README.md`: this entry.
* `probe.patch`: the probe, against af1b0e9 (`git apply probe.patch`): `src/search/floor.rs` (the floor, `TAXMAN_FLOOR`,
  its counters), the cut in `check_gf2`, `Meet::solve_at_least` and `Levels::heaviest_at_least`, their two tests, and
  the example `floor_plain`.  The code stays off master.
* `run.py`: one run of `wall images --root --count` with the floor set or not, waiting for a quiet machine.
* `runs.jsonl`: the four runs of the table, one line each (`wall images`' line with the floor's counters, `time -l`'s
  numbers and the load).  `table.py` makes the table from it.
* `identity.jsonl`: the build of af1b0e9 on 4176, by `run.py` with the master build's `wall`.
* `small-games.jsonl`: the five small games, each with the floor off, at F and at F + 1.
* `check_floor.py`: the data under the floor (`moves`, `plain`, `floor`, `every`); `chain-checks.txt` is the output of
  `moves` and `every`.
* `rank_scan.py`, `rank-scan.txt`: the root cut at promotions + 1, for 60 sampled games.
