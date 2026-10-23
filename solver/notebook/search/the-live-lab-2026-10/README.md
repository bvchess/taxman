# The live lab (2026-10)

Part of the [lab notebook](../../README.md).

Started 2026-10-22.

**Question.**  What do the studies of the chain's long searches find, and do they pay?  A study is an agent session that
measures a search the chain has run long and estimates the work left.  The entry is a log: each study adds a dated
section, and the first studies are here.

**The setup.**  The chain runs on a machine of 88 threads, 72 of them the chain's and 16 the lab's.  When the search of
a game has run an hour, the solver runs `solver/tools/lab/on-event.sh`, which starts a study in a sandbox, and when the
search ends it runs the script again, so that a study can compare the outcome with its prediction ([the runbook,
Studies](../../../docs/runbook/lab.md#studies), and [the design, the hook](../../../docs/design/lab.md#the-hook)).  A
study copies the search's checkpoint, solves the *units* of the copy (each the root of a subtree that can be solved by
itself) on one thread each with `tune`, fits the [steps](../../../../docs/GLOSSARY.md#the-search) of the finished units
to their [gap](../../../../docs/GLOSSARY.md#bounds), and sums the fit over all the units to estimate the steps left.
The notebook's line, log10 steps = 0.511 + 0.356 · gap/100, is the fit of [the fresh units of
4875](../where-the-search-of-4875-spends-and-which-conflict-to-branch-on-2026-10/README.md#2026-10-21-the-projection-checked-on-four-of-the-largest-units).
Each study writes a report and a journal entry with its predictions; the reports and the journal are copied here, and
the Files section names them.  All times are UTC, a game is named by n, and a search's *best* is the promotion sum of
its [incumbent](../../../../docs/GLOSSARY.md#the-search).

**Conflict branching**, which the sections below also call *the rule*, is the tactic that makes a node branch on the
conflict whose members cost the bound most to leave out ([glossary](../../../../docs/GLOSSARY.md#the-search)).  It is on
by default, and a node uses it only where its gap is at least the *gate*, 300 by default; below the gate the node
branches on the smallest conflict the search knows.  "Off" means the tactic off, or a gate above every gap of the unit.

## 2026-10-22: game 4875, a replay on the archived search

The first study found that conflict branching shrinks the units that finish, most at gaps of 1,000 and above, and that a
unit's gap alone predicts its size badly.  [The report](4875-20261022T044718Z.txt) belongs to a replay: the event
pointed at the archived checkpoint of the old search of 4875, with conflict branching off, which the chain had already
replaced.  The archive held 8.54 × 10⁹ steps, the best 538080 and 43,702 s of the search's own time.  The chain's fresh
search with conflict branching proved 4875 in 1.86 × 10⁸ steps and 56 minutes.  The copy of the old search held 14,230
units, with gaps from −48 to 1,836 (median 563).  There was no command folder, so the study acted on nothing.  Two
earlier starts of the same event ended by a signal after 4 and 3 minutes, with no result.

**The estimate against the outcome.**  The study estimated three ways to go on:

| search | steps left | time | outcome |
|---|---|---|---|
| the old search, left as it was | 7.7 × 10⁹ to 2.5 × 10¹⁰ | 18 to 58 h at 120,000 steps/s | replaced, so not known |
| the old search, switched to conflict branching | 3.0 × 10⁸ to 1.5 × 10⁹ | 1.5 to 8 h at 55,000 steps/s | not run |
| a fresh search, branching on conflicts | 5 × 10⁷ to 2 × 10⁸ | 0.3 to 1 h at 55,000 steps/s | 1.86 × 10⁸, 56 min |

A switch applies only to units started after it, and the old queue held 14,230 deep units, so a restart beat both
staying and switching.  It was at least 18 times cheaper than staying, where the procedure asks 3 for a `restart`.  The
chain had restarted already.

The fresh search's range came from the notebook's line, whose sum over the 96 fresh units is 5.3 × 10⁷ to 7.2 × 10⁷
steps, and from the largest units sitting 2.5 times above it, as the notebook measured, which gives about 2 × 10⁸.  The
chain's 1.86 × 10⁸ steps are 2.6 times the line's sum, at the top of the range.  The study's own line through five
finished units, log10 steps = −0.99 + 0.500 · gap/100, gave 5.6 × 10⁸, three times the outcome: five scattered points
fix no slope.

**Conflict branching on and off.**  Each unit ran on one thread from the proven best, 538080, and ended with it.  A
step cost 1.3 to 2.8 ms with conflict branching on and 0.67 to 1.16 ms off, with 14 to 16 jobs on the machine.  The
runs were limited by steps, so "more than" marks a unit that the limit stopped.

| unit | gap | on: steps | on: CPU s | off: steps | off: CPU s |
|---|---|---|---|---|---|
| 4875-1226 | 800 | 1,576 | 3.6 | 3,819 | 3.7 |
| 4875-1147 | 900 | 624 | 1.0 | 714 | 0.8 |
| 4875-1583 | 1,000 | 29,421 | 59.3 | 315,017 | 285.4 |
| 4875-984 | 1,100 | 64,291 | 82.2 | 216,314 | 151.3 |
| 4875-294 | 1,200 | more than 1,000,447 | 1,514.9 | more than 1,500,159 | 1,009.1 |
| 4875-3742 | 1,300 | 199,188 | 401.8 | not run | |
| 4875-1307, 1989 to 1992 | 1,400 to 1,836 | more than 250,879 | 398 to 694 | more than 400,383 | 330 to 464 |

On the four units that finished both ways, the rule took 0.41, 0.87, 0.093 and 0.30 of the default's steps and 0.97,
1.25, 0.21 and 0.54 of its CPU: the gain is small at gaps 800 and 900, and large at 1,000 and 1,100, as the notebook
measured on fresh units.  No setting finished the four largest units (gaps 1,780 to 1,836) in 330 to 694 CPU s, three
to six times the 120 s that the request set.

**A unit's gap predicts its size badly in an old queue.**  Units 100 points apart took 624 and 29,421 steps, and
4875-294 (gap 1,200) took more than 10⁶ steps where the line gives 10⁵.  An ask to the finished study, what it would try
next, put first the search for a second variable (depth, the number of candidates, the price of the cheapest member of
the root's conflict) on about 40 old units, at about 5 CPU hours.  The request of 06:59, open at the time, asks the same
of the next study.

**What the procedure lacked.**  `tune solve` took no `--cpu-limit` in the build the study used (it has had one since
bcfde26, and the later studies used it), so the study limited by steps sized by a guess of the step's cost, and the runs
took three to five times the 120 s asked.  A step of the study's own fresh search ran three to five times slower than
the notebook's figure on the laptop, on a machine shared with the chain, so times from a loaded machine and an unloaded
one do not compare.

## 2026-10-22: game 4880, an estimate that held

The study of 4880 predicted the work left to within a factor of 1.3, and found that staying the course beat a fresh
search ten times over.  At the event the search had taken 258,200,571 steps and 5,523 s of its own time, with conflict
branching on and the best 542,420.  The copy held 2,359 units, with gaps from 1 to 1,556 (median 428), and 14 of them
finished (gaps 400 to 1,203).  The fit of those 14, log10 steps = 0.605 + 0.391 · gap/100 with a residual spread of 0.42
decades, is close to the notebook's line.  Its sum over the 2,359 units was 4.0 × 10⁷ steps left.  Allowing for the
scatter and for units like 4875-294 that sit ten times above any line, the range was 3 × 10⁷ to 2 × 10⁸ steps, 8 minutes
to 1.3 h at the chain's 42,000 to 64,000 steps/s.  A fresh search was estimated at 4 × 10⁸ steps (1.6 × 10⁸ to 3.5 ×
10⁹), ten times as many, so the study recommended staying and sent no command.  [The report](4880-20261022T053931Z.txt)
has the rest.

**The outcome.**  The search ended exhausted at 289,485,607 steps and 6,139.9 s, so 3.13 × 10⁷ steps and 617 s (10.3
minutes) after the copy, at 50,700 steps/s.  That is 0.78 of the point estimate, and the fit summed over all the units
was 0.1 decade high.  The prediction was written at 05:39, after the game had ended at 05:31, but from the copy alone
and before the study saw the result.  The same proof gives game 4881 its result (`same_search`).  [The study of the
`done` event](4880-20261022T054120Z-done.txt) wrote this comparison in a short report and in the journal; the fresh
search's estimate could not be checked.

**Conflict branching on and off**, each unit from its copy's best on one thread, limited to 120 CPU s:

| unit | gap | on: steps | off: steps | off / on |
|---|---|---|---|---|
| 4880-22 | 599 | 480 | 3,588 | 7.5 |
| 4880-39 | 808 | 14,281 | 113,409 | 7.9 |
| 4880-10 | 897 | 5,750 | 110,750 | 19 |
| 4880-15 | 1,004 | 18,236 | more than 87,304 | more than 4.8 |
| 4880-241 | 1,476 | more than 49,957 | 20,396 | under 0.41 |

On three units of gap 1,462 to 1,556 neither setting finished.  The rule wins on four of the five, and loses on
4880-241, where the default finished in under half the steps.  The lab switches the rule only for a whole game, so the
study saw no use for the exception; a portfolio over units would be a new feature, and was not tried.

**What the procedure lacked.**  Three faults, which commit a23ca1d mended in `study.md` and `on-event.sh` before the
study of 4884:

* The fresh search's command did not chain, since `taxman solve 4880 --use-prev` without `--out` found no game of 4879.
  From a23ca1d the procedure copies the chain's results file and passes the copy as `--out`.
* A headless session stopped a background command after ten minutes, which ended both fresh searches and the 600 s unit
  runs early.  From a23ca1d a command of a session may run an hour (`BASH_DEFAULT_TIMEOUT_MS`).
* A game that ends within minutes cannot be helped by a study, and the estimate that shows it takes seconds from the
  units alone.  From a23ca1d a `long` event starts with that estimate, and with under 30 minutes left the study skips
  the measuring and the fresh search; a `done` event whose outcome the journal holds appends one comparison line and
  ends.

## 2026-10-22: game 4884, a triage and the gate of conflict branching

The study of 4884 estimated the work left from the units alone, spent its session on whether the gate of conflict
branching should depend on the game's size, and found no sign that it should.  At the event the search had taken 1.726 ×
10⁸ steps and 3,600 s of its own time, at about 52,000 steps/s, with conflict branching on and the best 543,308.  The
copy (172,689,331 steps) held 1,880 units, with gaps from 1 to 1,080, 957 of them under 300.  [The
report](4884-20261022T065716Z.txt) has every table.

**The estimate against the outcome.**  The triage summed the notebook's line over the units and got 5.8 × 10⁵ steps; the
fit of 4880 gives 1.36 × 10⁶; at 52,000 steps/s, 11 to 26 s.  The search ended exhausted at 174,744,268 steps and 3,638
s, so 2.05 × 10⁶ steps and about 38 s after the copy, with the best unchanged: 3.5 times the notebook's line and 1.5
times the fit of 4880.  The units of this game sit above the line: 4884-443 (gap 1,023) took 44,424 steps against the
line's 1.4 × 10⁴ (3.1 times), 4884-1560 (gap 1,080) took 45,202 against 2.3 × 10⁴ (2.0 times), and the units of gaps
300 to 600 run 2 to 5 times above.  The study sent no command, since the game had ended.  The `done` event wrote one
comparison line in the journal, and no report.

**The request and the method.**  The request of 05:21 noted that conflict branching costs steps on game 1000 solved from
scratch on one thread (3,360 against 2,026 without it) and asked whether the gate, the gap of 300 below which a check
does not use the rule, should depend on the game's size or the unit's gap.  The study built a variant on a local branch
of the lab's checkout (`study-4884-gate`, 4f74667, not in the repository) in which the environment variable
`TAXMAN_CB_GAP` sets the gate.  It ran the units of 4884 alone on one thread from the copy's best, limited to 120 CPU s,
and games 1000 to 3000 from the root, proved against the known optimum (`tune solve N --ids root --known`), limited to
300 CPU s.  The runs went 16 at a time at nice 19 beside the chain, so the CPU times carry about 10% noise by the
study's guess, and the steps are exact.  The gates were 0 to 800; the tables give 0, 150, 300 (the default), 450, 600
and off, and on the units gate 600 takes the steps of off.

**The rule just above the gate.**  On the 17 units of gap 300 to 571 the rule cost steps on 9, saved on 7 and tied on
1.  The median ratio of its steps to off's is 1.4, and the worst is 4884-1474 with 1,535 steps against 36.  The losses
are under 1,500 steps each, and the gains are large: 4884-1134 took 1,811 steps against 14,100, and 4884-591 took 293
against 3,185.  In all the rule took 0.31 of off's steps and 0.54 of its CPU.  Of the four units of gap above 1,000,
the rule finished two in 120 CPU s and off finished none.

**The gate on the 17 units and on the two large units that finished**, as the sum of steps and CPU seconds (the large
units' steps, with their CPU seconds in brackets; "+" marks a unit that the 120 s limit stopped):

| | gate 0 | 150 | 300 (default) | 450 | off |
|---|---:|---:|---:|---:|---:|
| the 17 units of gap 300 to 571: steps | 3,008 | 3,956 | 6,464 | 14,274 | 21,147 |
| the 17 units: CPU s | 12.9 | 10.7 | 10.8 | 14.1 | 19.9 |
| 4884-443, gap 1,023 | 24,692 (110) | 26,679 (106) | 44,424 (87) | 125,027+ | 196,916+ |
| 4884-1560, gap 1,080 | 28,366 (104) | 29,898 (97) | 45,202 (82) | 140,278 (117) | 171,853+ |

**The gate on whole games**, proved from the root against the known optimum, as steps / CPU s (a "+" marks a run that
the 300 s limit stopped; game 2500 stopped at every gate, so it decides nothing):

| game | gate 0 | 150 | 300 (default) | 450 | 600 | off |
|---|---:|---:|---:|---:|---:|---:|
| 1000 | 1,179 / 0.47 | 1,704 / 0.26 | 1,734 / 0.24 | 1,734 / 0.24 | 1,734 / 0.24 | 1,734 / 0.25 |
| 1500 | 5,831 / 4.39 | 4,260 / 1.56 | 6,538 / 1.58 | 8,857 / 1.96 | 8,857 / 1.90 | 8,857 / 1.95 |
| 2000 | 14,099 / 12.5 | 19,033 / 8.06 | 25,845 / 7.86 | 28,066 / 8.37 | 32,513 / 9.42 | 32,513 / 9.37 |
| 2500 | 342,884+ | 575,826+ | 1,012,869+ | 984,155+ | 965,239+ | 1,036,766+ |
| 3000 | 40,458 / 50.1 | 55,270 / 36.5 | 107,724 / 41.3 | 131,197 / 47.9 | 167,410 / 59.9 | 167,554 / 61.8 |

**What the sweep says.**

* By steps, a lower gate wins, except that gate 150 beats gate 0 on game 1500 (4,260 steps against 5,831): gate 0 takes
  about half of gate 300's (0.47 on the 17 units, 0.38 on game 3000).
* By CPU, gate 0 loses, since a step costs more.  A step costs about twice the CPU on the large units (225 to 272 steps
  a second against 508 to 554 at gate 300) and three times on whole games (game 3000: 808 against 2,608 steps a second).
* Gate 300 is the best by CPU on the large units of 4884 (82 to 87 s against 97 to 110 s at gates 150 and 0, and 117 s
  or more at 450).  Gates 150 and 300 tie on the mid units and on games 1500 and 2000.  Gate 450 and above is worse
  everywhere it differs.
* The best gate does not drift with the game's size from 1000 to 3000.  Two hints stay within the noise: gate 150 is 12%
  cheaper than 300 on game 3000 only (36.5 s against 41.3 s), and gate 0's premium over 300 shrinks as the work grows
  (2.0, 2.8, 1.6 and 1.2 times the CPU at games 1000, 1500, 2000 and 3000).  Neither shows on the units of 4884, a game
  of the size the chain runs.

**Game 1000.**  Proved against its optimum, game 1000 takes 1,734 steps at every gate from 300 up, because no check
reaches a gap of 300 there, so the rule's cost of 3,360 steps against 2,026 does not show in a proof.  It lies in a
search from scratch, where the incumbent is weak and the gaps are larger.  This is an inference from the sweep, not a
measurement: running the gate variant on a search from scratch needs `taxman solve` built from the branch, which the
study did not do.  The chain starts each game from the previous game's best game, so by this inference it is not
affected.

**Open in the report.**  Is gate 150 better than 300 on games of 3000 to 4000?  Two or three whole games with fewer runs
at once would settle it.  Does game 1000's cost come from checks made while the incumbent is weak, and would a gate tied
to the incumbent's age help?  Why does a step at gate 0 cost two to three times the CPU?  The notebook's [cost of a
step](../where-the-search-of-4875-spends-and-which-conflict-to-branch-on-2026-10/README.md#2026-10-21-the-cost-of-a-step-of-conflict-branching)
puts a step's CPU in the GF(2) bound's solves and the cycle search of each member priced.

## 2026-10-22: what the studies cost

Six sessions and the two killed starts, to 06:58.  The log, `on-event-20261022T0658Z.log`, records for each start the
event, the turns and seconds of the session's last result, and the cost the session reported (at list prices).

| study | started | ended | turns | cost |
|---|---|---|---|---|
| 4875 `long`, two starts | 03:50 and 04:02 | 03:54 and 04:04 | none | not logged |
| 4875 `long`, the replay | 04:06:54 | 04:56:28 | 4, see below | $1.77 |
| 4875 ask | 05:00:14 | 05:00:39 | 1 | $0.05 |
| 4880 `long` | 05:20:51 | 05:41:03 | 49 | $1.25 |
| 4880 `done` | 05:41:03 | 05:41:53 | 9 | $0.26 |
| 4884 `long` | 06:31:08 | 06:58:20 | 48 | $1.20 |
| 4884 `done` | 06:58:20 | 06:58:40 | 4 | $0.14 |

* The six logged sessions cost $4.66 in all, on the `opus` model.  A sandbox proof costs a few cents.
* A `long` study took 20 to 50 minutes of wall time, a `done` study under a minute, and an ask 25 s.  The ask is a
  turn of the replay's session, whose total went from $1.77 to $1.82.
* For the replay the log's 4 turns and 13 s belong to the last result of a session that a finished background job woke
  after 50 minutes; its session file holds 80 tool calls.
* No study sent a command to the chain, and none changed anything in it.  The studies' runs took the lab's 16 threads at
  nice 19.

## 2026-10-22: what the first studies add up to

* **The estimate from the gaps is usable, and the notebook's line runs low.**  A fit on the game's own finished units
  came within a factor of 1.3 at 4880.  The notebook's line ran 2.6 times low for 4875's fresh search and 3.5 times low
  for 4884, whose units sit 2 to 5 times above it.
* **Neither live search left time to act.**  The searches of 4880 and 4884 ended within 11 minutes and within a minute
  of their studies' start, and the one decision a study had to make, at 4880, was to stay, which the outcome bore out.
  Whether a study pays by changing a run waits for a search that outlasts its triage; the cost so far is $4.66, and the
  gain is the measurements here.
* **Conflict branching wins in steps at the gaps that matter.**  On all ten units of gap 599 to 1,100 where at least one
  setting finished (four of 4875, four of 4880, two of 4884), the rule took fewer steps, and it took less CPU on nine of
  them, the exception being 4875-1147 (gap 900), where it took 1.25 times the default's.  At 4880-241 (gap 1,476) the
  default won, and just above the gate (gap 300 to 571) the rule cost steps on 9 of 17 units yet took 0.31 of off's
  steps in all.
* **The gate of 300 stays**, with no sign of a dependence on the game's size.  The cost of the rule on a search from
  scratch is in [the tripwires of game
  1000](../where-the-search-of-4875-spends-and-which-conflict-to-branch-on-2026-10/README.md#2026-10-22-the-tripwires-of-game-1000).
* **Next:** the request of 06:59 asks the next study to compare the units far above the line with those on it.

## 2026-10-23: the look, tested on whole games

With the look on, a game took 0.79 to 0.97 of its neighbour's steps in all five pairs that could be compared, 0.85 in
geometric mean.  The test ran on the chain on the 88-thread machine: the lab's policy file (`run.lab`) turned the look
([glossary](../../../../docs/GLOSSARY.md#the-search), tactic `conflict_look`) on for the even games 4918 to 4948 and off
for the odd ones.  The pairs, each a game with the look on beside neighbours with it off, are 4920 and 4921, 4928 and
4925 with 4929, 4936 and 4935, 4940 and 4941, and 4944 and 4945.  The comparison counts steps.

**Decision.**  The lab's policy turns the look on for every game from 4948.  Whether it becomes the code's default is
open, for the project's owner.  Five pairs are few, though neighbouring games share the top of their trees (see [the
root gap and the size of a game](../the-root-gap-and-the-size-of-a-game-2026-10/README.md)), which makes the pairs fair.

## 2026-10-23: the studies stopped

The studies stopped at 09:34 UTC on 2026-10-23, and stay off until a better approach.  They were not paying off, and
adapting them cost the chain.  The `done` hook still appends a line per finished game to the lab's games history ([the
runbook](../../../docs/runbook/lab.md)).

## Files

The reports, the journal and the log are copies of the files in `~/taxman-lab/` on the machine of the chain, read with
ssh and `cat`; nothing was written there.  The reports and the journal are markdown there and plain text here, so that
they stay as the studies wrote them.

* `4875-20261022T044718Z.txt`, `4880-20261022T053931Z.txt`, `4880-20261022T054120Z-done.txt` and
  `4884-20261022T065716Z.txt`: the reports of the studies, named for the game and the time the report was written.  The
  `done` study of 4884 wrote none.
* `journal-20261022T0658Z.txt`: the studies' journal at 06:58, with their predictions and comparisons.
* `on-event-20261022T0658Z.log`: `~/taxman-lab/ctl/on-event.log` at 06:58, which `lab.sh studies` summarizes, with the
  home directory shortened to `~`.

## Commits

The studies' procedure: bcfde26 (the hook, the sandbox and `tune solve --cpu-limit`), c63c45f (the proof renewed by a
study) and a23ca1d (the fresh search that chains, commands that may run an hour, the triage).
