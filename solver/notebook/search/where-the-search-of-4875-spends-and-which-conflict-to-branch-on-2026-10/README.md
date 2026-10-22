# Where the search of 4875 spends, and which conflict to branch on (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-21.

**Question.**  Game 4875's search had taken 6.9 billion steps when the run past 5000 stopped in it.  Where does the proof
that nothing beats its best, 538080, spend its steps, and can an idea not yet tried make such a search twice as cheap,
soundly?  The register's closed avenues are not repeated here: the stronger bounds, the learning, branching in
hindsight, the decompositions and the floors ([stronger bounds](../../register/stronger-bounds.md), [the
search](../../register/search.md), [nogoods and conflicts](../../register/nogoods-and-conflicts.md)).

## 2026-10-21: the tree of 538080, and two screens

**A fresh tree.**  The run's own checkpoints were split while the incumbent was still 537927 to 537971, so their units
describe the start problem, which [the finder](../the-finder-on-long-searches-and-game-4875-2026-10/README.md) now
removes.  So the search of 4875 was started again on one thread, with the finder after the first step: it offered
538080 after 0.3 s, at 1,023 steps, and the search stopped at 3.0 million steps with its checkpoint.  `tune units`
cut 96 units from it, with 538080 as the known best (`units-fresh.jsonl`).

**Where the work is.**  In the loops that the top twenty levels have left.  The root's gap is 2,029, and its own loop,
without the first member it branched on (2107), has a gap of only 997: leaving 2107 out costs 1,032.  The costly loops lie
a few levels down: fourteen units at depths 2 to 33 have gaps of 1,559 to 1,900, and by the fit of [the finder's
entry](../the-finder-on-long-searches-and-game-4875-2026-10/README.md) (log10 steps = 0.178 + 0.471 · gap/100) they hold
97% of the 3.5 billion steps it predicts; the one at depth 4 (gap 1,900) holds 38%.  The fit was made on gaps of 209 to
959, so the total is uncertain by a large factor; the shares are what matter here.

**What the tree is made of.**  `wall anatomy --root` traced the first 2.0 million steps of the same search
(`anatomy-root.txt`, made by the anatomy entry's `analyze.py`):

* *Gap and size.*  A node's subtree grows a decade per 291 points of its gap.
* *Exclusions.*  The gap falls by exclusions: 8 on a path to a leaf (p10 to p90: 6 to 11), each costing the bound a
  median 162 points, 8.9% of the member's value; leaves close at a median gap of 204.  Promoting the member branched on
  leaves the gap where it was (median change 0).
* *Substitutes.*  58% of the exclusions swap the member for one other candidate, of the same home in 78% of those, and
  most often by one prime on a fixed cofactor: 27·79 for 27·73 (34,051 times), 63·31 for 63·29, 66·31 for 66·29, 98·19
  for 98·17.
* *Frames.*  The conflicts branched on lie in frame 3 (27%), frame 2 (26%), across frames 2 and 3 (25%) and across
  frames 1 and 2 (16%).  At the root the heaviest set's excess over the best game is at homes 3 (+2,419) and 4
  (+2,174), offset at homes 2 (−915) and 5 (−1,648); the bound study's games had theirs at home 3
  ([a bound exact in a few frames](../a-bound-exact-in-a-few-frames-2026-10/README.md)).
* *Depth.*  The stack reaches 149 nodes deep, and half the steps lie below depth 100; 57% of the nodes have one child.

**The cheap member.**  A node branches on a conflict, every child leaving out one member of it.  The child that leaves
out the cheapest member keeps nearly the whole gap, and it is the heaviest child.  So how far a node's subtree falls
below the node's gap is set by the conflict's cheapest member, and the conflicts the search branches on have cheap
ones: at the 49 nodes of [the exact problem of frame 2](../the-exact-problem-of-frame-2-2026-10/README.md)
(`instances.jsonl.gz`, gaps 185 to 1,211) the cheapest member of the search's conflict costs a median 16 points.

**Screen 1: charging disjoint cores (closed).**  A bound that charged every disjoint conflict of the heaviest set its
cheapest member would take off the sum of those prices.  In each frame that the heaviest set leaves unwinnable, the
peel strands 100 to 165 movers, yet the residual is one knot that a few candidates hold: a deletion filter finds 1.7
disjoint conflicts a frame, whose cheapest members cost 2 to 34.  Their prices add up to a median 12% of the node's gap
(`disjoint.py`, `disjoint-old.txt`), so such a bound cuts nothing where the tree is.  This agrees with [the node
choice](../the-node-choice-where-the-strong-bound-pays-2026-10/README.md), whose refits over the nogoods cut no node of
gap 300 or more.

**Screen 2: near-symmetric prime pairs (closed).**  The swaps above are exchanges of two primes on one cofactor, and the
rule of symmetric pairs (THEORY.md, Theorem 13) prunes by such exchanges where they map the root onto itself.  4875 has
383 such pairs, all of primes above 70.  The pairs of the frequent swaps nearly map the root onto itself: 73 and 79 move
8 numbers out of their roles (four sources 63·73 to 66·73 with no image, and 2336 and 2409, factors whose images are
moves), 67 and 71 move 9 (`nearpairs.py`, `pairdefect.py`).  But the best game itself has more on the smaller prime
(the cofactor sums c₇₃ = 123 against c₇₉ = 105, and c₆₇ = 91 against c₇₁ = 82; `optpairs.py`), so no rule of that form
holds for near pairs.

**Screen 3: a conflict whose members all cost much to leave out.**  Of the minimal conflicts within a node's heaviest set, the search
branches on the smallest it knows (the table's, else the peel's, made by leaving out members from the smallest up).
Other minimal conflicts exist, and a deletion filter that leaves out the cheapest members first finds some whose
members all cost much.  Weighing a conflict by the sum over its members of 10^(−price/291), the children's subtrees against the node's if
each follows its gap, the best of the frames' conflicts weighs a median 0.65 of the search's at the 49 nodes, and its
cheapest member costs a median 51 points against 16 (`conflictchoice.py`, `conflictchoice-old.txt`).  [The headroom of
branching](../the-headroom-of-branching-in-hindsight-2026-10/README.md) measured the choice of member within the search's
conflict, and the members of the heaviest set's other disjoint conflicts at one unit's root (3940-104, no better); it
did not try conflicts whose members all cost much, nor a choice made at every node.  So the choice was tried as a rule.

## 2026-10-21: conflict branching

**The rule**, *conflict branching* (first a study patch in the studies build, set by environment variables and not
kept; the tactic `conflict_branching` replaced it, below).  At
each GF(2) check that finds a conflict, the search prices every member of the heaviest set: the bound less the bound
without the member, read off the exchange graph as strong branching reads it (`Gf2Bound::certified_excluding`), or the
weight split's estimate where that is not certified.  It then weighs three kinds of conflict:

* the one the search found (the table's smallest, or the peel's);
* every conflict the nogood table knows within the heaviest set (`Nogoods::all_within`);
* in each frame that the heaviest set leaves unwinnable, the minimal conflict made by leaving out the cheapest members
  first (`FrameGraph::conflicts_ordered`).

A conflict weighs the sum over its members of 10^(−price/291).  The node branches on the lightest, which is learned as
a nogood if it was made here.

**Why it is exact.**  It changes only which conflict the node branches on, and every completion leaves out some member
of any conflict, so every completion lies in one of the children.  Each conflict made here is one: with its members
alone applied, its frame keeps a stuck set of moves, so the frame is not winnable
([decision-procedure.md](../../../../docs/theory/decision-procedure.md), Theorem 3).  A stuck set survives any larger
set of promotions, which only adds moves and takes factors (Theorem 4 there), so no completion holding the whole
conflict leaves the frame winnable, and none can be promoted ([frame-model.md](../../../../docs/theory/frame-model.md),
Theorem M).  The study build checks each one with the whole peel (`assert!` in `conflicts_ordered`).  A conflict whose
members all cost much is often a set of forced members, and the search already cuts a node whose conflict its forced members contain:
every completion that beats the incumbent promotes all of them, and they cannot all be promoted.

**Units of 4875** (`tune solve`, one thread, fresh tables, from 538080; every one completed with best 538080, proved):

| unit | gap | default: steps | CPU s | conflict branching: steps | CPU s | steps, ratio | CPU, ratio |
|---|---|---|---|---|---|---|---|
| 4875-22 | 931 | 67,433 | 23.4 | 8,413 | 10.2 | 0.125 | 0.44 |
| 4875-65 | 970 | 150,639 | 32.5 | 15,148 | 13.3 | 0.101 | 0.41 |
| 4875-36 | 1,102 | 537,601 | 118.1 | 16,123 | 16.9 | 0.030 | 0.14 |
| 4875-10 | 1,166 | 1,542,879 | 491.0 | 35,124 | 51.1 | 0.023 | 0.10 |

The default search of 4875-22 took 67,433 steps on the build before the patch and on the build with it, so the patch
changes nothing when it is off.  The share of the steps saved grows with the unit's gap.  A step of the rule costs three
to five times a default step, most of it the pricing and the peels of the ordered conflicts.  In 5 s of samples of the
whole game 4176 with residual pricing (`sample`, not kept), the cycle searches took 31% and their queue 15% more, and
the peels 10%.

**Soundness checks.**  Every run below completed with its best equal to the known optimum.  `taxman regress --to 1300
--threads 1 --tactic same_search=off --tactic prime_shortcut=off`, with the rule on and the assertion built in, searched
866 of the 1,300 games from their own starting incumbents, so that a false conflict could have cut the optimum: all 1,300
scores match `optimal.json` (`regress-rule.txt`).  The rule makes those small games slower: 60.2 s against 37.9 s
(`regress-base.txt`).

**Units of the hardest searches** of the run to 4547 ([its suite](../the-hardest-searches-of-the-run-to-5000-2026-10/README.md),
`suite-hard.jsonl`; one thread, fresh tables, from the committed optimum).  *All* prices every member of the heaviest
set exactly; *residual* only the members that some failing frame's conflict can hold,
the others by the weight split's estimate.  The two make the same trees.

| unit | default: steps | CPU s | the rule: steps | CPU s, all | CPU s, residual | steps, ratio | CPU, ratio (residual) |
|---|---|---|---|---|---|---|---|
| 3940-104 | 104,080 | 12.0 | 7,838 | 4.2 | 3.5 | 0.075 | 0.29 |
| 4416-95 | 119,309 | 17.2 | 26,318 | 20.0 | 14.5 | 0.22 | 0.84 |
| 4225-1.15 | 136,320 | 23.4 | 14,011 | 13.7 | 10.3 | 0.10 | 0.44 |
| 4548-13 | 282,833 | 46.5 | 79,653 | 57.4 | 43.0 | 0.28 | 0.92 |
| 4548-107 | 116,900 | 19.5 | 31,564 | 25.2 | 18.8 | 0.27 | 0.96 |

**Whole games** (`tune solve N --ids root`, one thread, fresh tables, from the committed optimum; the two games whose
whole proofs [the hardest searches](../the-hardest-searches-of-the-run-to-5000-2026-10/README.md) ran):

| game | gap | default: steps | CPU s | the rule: steps | CPU s, all | CPU s, residual | steps, ratio | CPU, ratio (residual) |
|---|---|---|---|---|---|---|---|---|
| 4050 | 1,012 | 3,086,223 | 470.0 | 322,587 | 233.6 | 221.2 | 0.105 | 0.47 |
| 4176 | 1,010 | 5,342,669 | 891.1 | 324,104 | 351.1 | 306.0 | 0.061 | 0.34 |

The default search of 4050 took the same 3,086,223 steps as on the build of the hardest searches.  So on whole games the
rule takes a tenth to a sixteenth of the steps and a half to a third of the CPU, as on the units; unlike the learning,
it does not turn around on whole games.  Its own cost per step is what holds the CPU back.

**Cheaper pricing, and a gap gate.**  Three ways to spend less on the rule's own work were tried:

* *Estimates only*: the weight split's estimate of every price, with no cycle search.  It
  saves CPU where the trees are small (3940-104 2.7 CPU s, 4416-95 9.9, 4548-13 33.4, 4875-10 27.8) but loses the
  tree where the gap is large: 4875-36 took 102,149 steps and 45.1 s, against 16,123 and 16.9 with exact prices.  Not
  kept.
* *Residual* exact prices (above): the same trees as pricing every member, 5-27% less CPU.
* *A gap gate* G0: the rule chooses only at checks whose gap, the bound less the need, is at least
  G0, and below it the node branches on the search's own conflict, so the small subtrees do not pay the pricing.

With residual prices:

| gap gate G0 | 4050 whole: steps | CPU s | 4548-13: steps | CPU s | 4875-36: steps | CPU s |
|---|---|---|---|---|---|---|
| default search | 3,086,223 | 470.0 | 282,833 | 46.5 | 537,601 | 118.1 |
| 0 | 322,587 | 221.2 | 79,653 | 43.0 | 16,123 | 16.9 (every member priced) |
| 200 | 387,868 | 151.4 | 88,598 | 31.8 | 18,269 | 12.7 |
| 300 | 582,353 | 122.8 | 109,205 | 27.5 | 27,267 | 11.7 |
| 400 | 860,853 | 131.3 | 149,007 | 28.2 | 59,088 | 17.0 |

G0 = 300 is the best of the four on all three: 4050 takes 0.26 of the default's CPU (3.8 times less), 4548-13 0.59
and 4875-36 0.099.  The whole game 4176 at G0 = 300 took 834,688 steps and 236.6 CPU s, 0.16 and 0.27 of the
default's.

**What a fresh search of 4875 would cost with the rule** (residual prices, G0 = 300; `project.py`).  The rule solved
59 fresh units of gap 635 to 1,479 to the end, one thread each, every one with best 538080 (`fit-g300.jsonl`, 6.07
million steps and 2,955 CPU s in all).  Their steps follow the line log10 steps = 0.591 + 0.347 · gap/100, with a
residual standard deviation of 0.40 decades, against the default's 0.178 + 0.471 · gap/100 of [the finder's
entry](../the-finder-on-long-searches-and-game-4875-2026-10/README.md).  Three units run without the gate check the
line where the cost is: 581,917 steps at a gap of 1,466, 593,890 at 1,478 and 871,472 at 1,574 (`big-rule.jsonl`),
within a factor of 1.3 of it.  Summed over the 96 units, each unit run to the end counted at its own steps and the
others at the line, with the residuals drawn at random for the spread:

| | steps | CPU hours on one thread of this laptop |
|---|---|---|
| the rule, the line | 6.4 × 10⁷ | 8.6 |
| the rule, the median of the draws; p10 to p90 | 8.6 × 10⁷; 6.0 × 10⁷ to 1.3 × 10⁸ | 11.6; 8.1 to 17.6 |
| the default, by the finder entry's line | 3.5 × 10⁹ | about 310 |

The rule's step cost 0.487 ms on this laptop over the units it solved, against about 0.3 ms for a default step.  Its
largest share is the unit at depth 4 (gap 1,900), about 1.5 × 10⁷ steps by the line (`project-g300.txt`).  The
default's line is the optimistic one: the four fresh units run both ways took two to three times the steps it predicts.
So a fresh search with the rule should take some 20 to 35 times less CPU than the default's line gives, and more if the
default runs above its line as those units did.  On the run's machine, which takes 117,000 default steps a second on
88 threads, the rule would take roughly 72,000 of its own steps a second, if its step costs the same against a default
step there and it parallelizes as well.  That is about 15 to 30 minutes for 6 × 10⁷ to 1.3 × 10⁸ steps, against about
12 hours (2 to 38) for the 5 billion default steps the stopped run has left.  Neither the rule on many threads nor on
Graviton has been measured.

**Where this leaves the question.**  The cost of 4875 is the bound's error in frames 2 and 3, paid by exclusions, and
the search paid it slowly because the conflicts it branched on had a cheap member: the child that left that member out
kept nearly the whole gap.  Branching on a conflict whose members all cost much makes every child's gap fall, and often
closes the node at once, since such a conflict is often all forced.  That is a branching rule, so it needs no new
theorem: the conflicts it makes rest on Theorems 3, 4 and M, and each is checked by the whole peel in the study build.
Its gain grows with the gap (on 4875's units, a tenth of the default's steps at a gap of 900-1,000, a thirtieth to a
fortieth at 1,100-1,200), so it bends the cost curve rather than shifting it: the rule's trees grow a decade per 288
points of gap on 4875's units, the default's a decade per 212 by the finder entry's line.

What is not done:

* the rule as a tactic of the solver (it is a study patch read from the environment, so no checkpoint header records
  it), with its tests and the tripwires' new fingerprints;
* its cost per step, three to five times a default step: the pricing's cycle searches and the ordered conflicts' peels
  are made afresh at every check, where strong branching prices the chosen conflict again and a frame's situation often
  repeats;
* the rule on many threads, and on Graviton;
* an early exit that keeps the search's conflict where its members all cost much already (built in the study patch, not
  measured), and other slopes than 291 in the weight.

## 2026-10-21: the rule as a tactic

The rule is the tactic `conflict_branching` (`--tactic conflict_branching=on`, off by default), with the settings chosen
above as constants of `search`: `CONFLICT_BRANCHING_GAP` (300) and `CONFLICT_BRANCHING_SLOPE` (291), residual pricing,
and no environment.  It has bit 9 of a checkpoint's tactics, so a checkpoint of a build before it resumes with the tactic
off and refuses a run with it on, and a build before it refuses a checkpoint with it on.  A result line records it among
`tactic_switches`.  The soundness tests take it at every check, whatever the gap, on one thread and two
(`tests/soundness.rs`, configurations 41 and 42), since the small games there never reach a gap of 300.  The design,
the reference, the table of tactics and the glossary describe it ([search.md, "Conflict
branching"](../../../docs/design/search.md#conflict-branching)).

The release build with the tactic proves the whole game 4050 in 582,353 steps, the study patch's count at the same
settings, both by `taxman solve 4050 --use-prev --force-search --threads 1 --tactic conflict_branching=on` (score
5160836, the committed one; `release-4050.jsonl`) and by `tune solve` from the optimum (`tactic-4050.jsonl`).  The
assertion in `conflicts_ordered` costs 0.5% of the CPU of that proof: 138.9 s against 138.1 s for a copy without it,
side by side.

## 2026-10-21: conflict branching by default

The tactic is on by default from the commit after the one that added it.  The matching path keeps it off
(`SearchTactics::matching`), since it acts only with the GF(2) bound, so the committed checkpoints of that path still
resume; a checkpoint of the default search written before the tactic resumes only with `--tactic
conflict_branching=off`.  The soundness tests keep a configuration with the search's own conflict everywhere
(configuration 43), and the solver's variants test `conflict_branching=off`.  No tripwire's pinned tree changed: the
games they pin never reach a gap of 300.

`taxman regress --to 1300` on all ten CPUs, twice on each build, alternating: the build of master took 7.1 and 7.1 s
(33.96 and 33.74 CPU s), this build with the tactic on by default 7.2 and 7.1 s (33.82 and 33.78 CPU s), and both
matched `optimal.json` at all 1,300 games, 417 of them searched (`regress-master.txt`, `regress-branching.txt` and their
second runs).  The games to 1300 never reach the gap of 300, so the default changes nothing there.

## 2026-10-21: the projection checked on four of the largest units

The tactic, at its settings, ran four of the fresh units of 4875 with gaps of 1,647 to 1,701, where the projection's
line is extrapolated, each on one thread of its own, side by side (`run-big-tactic.sh`).  Two finished with best
538080: 4875-12 (gap 1,647) in 5,135,709 steps and 2,342 CPU s, and 4875-18 (gap 1,652) in 4,718,391 steps and 2,038
CPU s (`big-tactic-done.jsonl`), 2.5 and 2.2 times the line, within its residual spread of 0.40 decades but both above
it.  The default's line puts these units at 8.6 × 10⁷ and 9.1 × 10⁷ steps, seventeen and nineteen times as many.  The
other two, 4875-2 (gap 1,685) and 4875-3 (1,701), were stopped unfinished after 2,370 CPU s each, more than five million
steps.  With the two finished units in the fit, the line is log10 steps = 0.511 + 0.356 · gap/100, and the sum over the
96 units is 8.0 × 10⁷ steps on the line, a median of 1.0 × 10⁸ over the draws and 7.4 × 10⁷ to 1.6 × 10⁸ from p10 to
p90, 10 to 20 CPU hours on one thread of this laptop (`project-g300-big.txt`).  If the largest units all run two to
three times above the line, as these did, the total is about 2 × 10⁸ steps: still a fifteenth of the default's line,
and under an hour on the run's machine at the rate guessed above.

## 2026-10-21: the cost of a step of conflict branching

**No change pays: a step costs what its cycle searches and the bound's solves cost.**  Each member the rule prices
exactly costs one cycle search of the exchange graph, and the ordered conflicts need those prices to choose well.  The
work that the tree could share or skip is a few percent of a step.  Fewer cycle searches change the tree, and the ones
tried lost as many steps as they saved CPU or saved nothing.  This round changed no code.

**Where a step's CPU goes.**  At the default (the rule on, gap gate 300), a step of the whole game 4050 costs 212 µs and
one of the unit 4875-10 703 µs, one thread each (`tune solve`, from the optimum).  A default step without the rule
cost about 150 and 320 µs (the tables above).  Of 20 s of samples of each (`/usr/bin/sample`, read by `profile.py`):

| share of the CPU | 4050 | 4875-10 |
|---|---|---|
| the GF(2) bound's solves (`Gf2Bound::evaluate`) | 62% | 39% |
| the rule's prices: one cycle search a member (`certified_excluding` in the check) | 14% | 43% |
| the peels that make conflicts, in the rule's choice (4050) or in the whole check (4875-10) | 2.6% | 5.5% |
| strong branching's prices (`certified_excluding` outside the check) | 3.5% | 2.9% |
| sorting the members by price | 1.8% | 0.4% |
| the map of the prices (`HashMap`) | under 0.1% | under 0.1% |

**The same tree for less CPU.**  An idea that keeps the tree saves at most the share of the work it removes:

* *Strong branching reusing the rule's prices*: at most 3.5%, all of strong branching's pricing.
* *Caching each frame's conflicts*: at most 2.6% on 4050 and 5.5% on 4875-10, all of the peels.
* *A vector of prices in place of the map*, sorted directly: at most 1.8%.

None was built.  The pricing code of the variants below, run with no variant set, gives master's trees (582,353,
27,267 and 45,962 steps) at master's CPU within the noise of runs side by side: 126.6 and 125.4 s, 12.1 and 12.1 s,
33.3 and 33.4 s, against 123.2, 11.9 and 32.3 s.

**Fewer cycle searches.**  Three variants price fewer members or search less far.  Each changes the tree, and each is
set by an environment variable on top of master (`cost-variants.patch`, `run-cost.sh`):

* *A limit of 600*: each cycle search stopped at 600, and none made for a member whose estimate is 600 or more.
* *Late pricing*: the ordered conflicts made in the order of the weight split's estimates, and only their members and
  the search's own conflict's priced exactly.
* *An early exit at P*: the search's own conflict kept, before any other member is priced, when each of its members
  costs at least P.

| variant | 4050: steps | CPU s | 4875-36: steps | CPU s | 4875-10: steps | CPU s |
|---|---|---|---|---|---|---|
| master | 582,353 | 123.2 | 27,267 | 11.9 | 45,962 | 32.3 |
| a limit of 600 | 582,534 | 127.3 | 27,372 | 12.0 | 45,909 | 32.7 |
| late pricing | 934,446 | 161.0 | 37,083 | 13.5 | 65,559 | 27.1 |
| an early exit at 300 | 592,058 | 124.9 | 27,267 | 12.2 | 45,962 | 33.5 |
| an early exit at 600 | 582,353 | 124.7 | 27,267 | 12.2 | 45,962 | 33.4 |

Every run completed with best equal to the known optimum (`cost.jsonl`).  None is kept:

* The limit saves nothing: the cycle searches that cost are those of the cheap members, which it does not touch.
* Late pricing makes a step 17-41% cheaper, but its order makes worse conflicts: 1.36 to 1.60 times the steps, and more
  CPU on 4050 and 4875-36.
* The early exit almost never fires, because the search's own conflict nearly always has a cheap member.

**The gap gate stands.**  Pricing did not get cheaper, so the gates of 200 and 0 were not measured again; the gate
table above, where 300 is the best of the four on all three cases, still holds.

**On eight threads, the rule's steps spread almost as well as the default's.**  The release build proved the whole game
4050 from its committed optimum (`taxman solve 4050 --use-prev --force-search --finder-after 0`), one run at a time, on
this laptop, an M1 Max with eight performance CPUs and two efficiency CPUs (`run-threads.sh`):

| threads | rule | steps | search s | CPU s | steps a second | CPU µs a step |
|---|---|---|---|---|---|---|
| 1 | on | 582,353 | 126.4 | 126.0 | 4,607 | 216 |
| 8 | on | 608,353 | 18.1 | 140.6 | 33,669 | 231 |
| 1 | off | 3,086,223 | 465.6 | 463.4 | 6,629 | 150 |
| 8 | off | 2,831,411 | 55.4 | 437.9 | 51,111 | 155 |

On eight threads the rule takes 7.3 times its one-thread rate of steps, the default 7.7 times; a step of the rule costs
7% more CPU there, a default step 3%.  The rule's tree grew 4% on eight threads and the default's shrank 8%, so the
rule's proof ran 7.0 times as fast and the default's 8.4 times.  The rule proves 4050 3.7 times as fast as the default on
one thread and 3.1 times on eight.  Every run proved the committed score, 5160836 (`threads.jsonl`).  Many threads, as
on the run's machine at 88, have not been measured.

## 2026-10-22: the tripwires of game 1000

Game 1000, solved from scratch as the tripwires solve it (`taxman solve 1000 --threads 1`), takes 3,360 steps with the
rule and 2,026 without it (`--tactic conflict_branching=off`): the rule costs steps on a small game whose gaps pass 300.
The note above that no tripwire's tree changed (the default since a88dc7f) held for the quick check only; `cargo test
--release` found the six rows of game 1000 moved, and `tests/tripwires.rs` pins them at the counts with the rule.

## Files

* `units-fresh.jsonl`: the 96 units of the fresh search's checkpoint at 3.0 million steps (`run-fresh.sh`, then
  `run-units.sh`), with the gap of each; the checkpoint itself is not kept.  `known4875.jsonl` gives the tools 538080 as
  the known best: the line of 4874 and the finder's game of 4875.
* `anatomy-root.txt`: the anatomy entry's `analyze.py` on the trace of the first 2.0 million steps
  (`run-anatomy-root.sh`; the trace, 330 MB, is not kept).
* `disjoint.py` and `conflictchoice.py`: screens 1 and 3, on the instances of [the exact problem of frame
  2](../the-exact-problem-of-frame-2-2026-10/README.md) (`instances.jsonl.gz`) and on 20 nodes of the units 4875-65 and
  4875-22 (`run-local.sh`); their outputs `disjoint-old.txt`, `disjoint-4875.txt`, `conflictchoice-old.txt` and
  `conflictchoice-4875.txt`.  They import `f2lib.py` from that entry's folder.
* `nearpairs.py`, `pairdefect.py`, `optpairs.py`: screen 2, on the root's instance (`root-instance.jsonl`, `wall local
  --root --out`) and the best game (`../the-finder-on-long-searches-and-game-4875-2026-10/game-4875-538080.json`).
* The runs, one `tune solve` line each (`run-solve.sh`, `run-suite.sh`, `run-whole.sh`, `run-regress.sh`,
  `run-variants.sh`, `run-gate.sh`, `run-whole-tactic.sh`, `run-release-4050.sh`): the default `base.jsonl`,
  `base2.jsonl`, `suite-base.jsonl`, `whole-base.jsonl`; the rule with every member priced `rule.jsonl`, `rule2.jsonl`,
  `suite-rule.jsonl`, `whole-rule.jsonl`; residual prices `suite-resid.jsonl`, `whole-resid.jsonl`, `big-rule.jsonl`;
  estimates only `var-split.jsonl`; the gates `gate200.jsonl`, `gate300.jsonl`, `gate400.jsonl`, `whole-g300.jsonl`;
  the fit `fit-g300.jsonl`; the tactic `tactic-4050.jsonl` and `release-4050.jsonl` (a results line); and
  `regress-rule.txt`, `regress-base.txt`, and the full regress on both builds `regress-master.txt`,
  `regress-master-2.txt`, `regress-branching.txt`, `regress-branching-2.txt` (`run-regress-both.sh`).  `table.py
  LABEL=FILE...` sets runs side by side.
* `project.py`: the projection, `project.py units-fresh.jsonl fit-g300.jsonl --steps-done 3000309`; its output
  `project-g300.txt`, and with the two large units `project-g300-big.txt` (`... fit-g300.jsonl big-tactic-done.jsonl
  ...`).
* The cost of a step: `run-cost.sh` runs a build on the three cases, `cost-variants.patch` holds the variants, and
  `cost.jsonl` has every run, each line labelled with its `variant` (`same` and `same2` are the patch with no variant
  set).  `profile.py` reads a `sample` call graph; `run-profile.sh` wrote `profile-4050.txt` and `profile-4875-10.txt`
  (the samples, 0.35 MB each, are not kept).  `run-threads.sh` ran the release build on one and eight threads, with
  its result lines in `threads.jsonl`.

The scripts name the scratch directory they ran in (`S=`); set it to run them again.  The runs before the tactic used the
study patch, not kept, whose environment variables set the rule on, the pricing and the gap gate; the tactic
`conflict_branching` has replaced it, with the settings chosen, and gives the same trees at them (`tactic-4050.jsonl`).
