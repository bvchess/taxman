# Status

Updated 2026-11-01.

The state of the work as of the date above: the priority, and the projects in flight.  This page is rewritten, not
appended to, when the state changes.  What was done and why is in the [lab
notebook](https://github.com/bvchess/taxman-notebook/blob/main/README.md), how far the chain reaches in
[results/README.md](../../results/README.md), and the research avenues in the
[agenda](https://github.com/bvchess/taxman-notebook/blob/main/agenda/AGENDA.md).

## The priority

The hunt for a larger n ended on 2026-10-29: the project's owner ended it at 17:20 PDT, and the chain was stopped at
18:01 PDT, after game 5060.  The committed chain, `results/optimal.jsonl.gz`, reaches game 5060.  The priority is a
run of the full chain, 1 to 5060, with one build and the one policy that makes the whole run the fastest; other work
(the last cleanup step, the prover's run to 2530, the unsound mode) waits until it ends.
Certifying [statement U](../../docs/theory/exactness.md#statement-u), for particular games or in general, is not a
priority.  The committed chain is also the table built into the frame solver: the chain's lines are appended to it
([the runbook](runbook/chain.md)), and a rebuild takes them into the table.

## In flight

* **The Nov 1 run.**  The chain runs from game 1 to 5060 on a friend's machine of 88 threads with the build of commit
  aaba59b and the solver's default policy: a fixed gate of conflict branching of 300 and no gate trial, which beat the
  previous policy (a gate of 1200 with the trial) by 1.5 to 5.5 times on whole games
  ([gate-trial.md](design/gate-trial.md)), with `price_reuse` and `warm_start` on and a finder of 900 seconds.  It started
  on 2026-11-01 at 08:24 PST as `chain.sh 5060` with only the flags of the machine and the run (`--background`,
  `--carried-dir`, `--profile`) and no `--use-prev`.  Each line records the game's CPU, active and waiting seconds and its
  start and finish in UTC.  When it completes, its file replaces `results/optimal.jsonl.gz`.  The Oct 29 run it replaces
  (tag `rerun-2026-10`, commit 3ca11bf, gate 1200 with the trial) stopped at game 4879 and is kept as a record on that
  machine.
* **The cleanup of the frame solver.**  The project's owner decided on 2026-10-29 to remove the lab, the studies and the
  idle experiments, to make the production policy the default, to split the notebook into a repository of its own, and
  to leave the prover alone.  The code is done: the removals and the defaults are on master, and a consistency pass has
  brought the docs and the code left behind into line.  The lab notebook and the agenda files have moved, with their
  history, to the repository [bvchess/taxman-notebook](https://github.com/bvchess/taxman-notebook).  One step remains.
  * **The retirement of the bounds store and the certificates.**  It waits for the Nov 1 run.  When its file
    replaces `results/optimal.jsonl.gz`, the games have no `known` lines, so the bounds store, the ledger, `--use-prev`
    and `check_bounds.py` retire, with the readers of older line formats and `REFERENCE.md`'s keys of older lines.  The
    certificates of statement U retire too (`taxman certify`, `verify-certificates`, the extended mode they need, the
    certificates of games 600 to 1300): the prover's own proofs, from game 2 up, replace them.
* **The documentation.**  Every long document is a folder of files with an index, and every doc follows
  [docs/WRITING.md](../../docs/WRITING.md), whose docs test checks the links, the citations, the sizes and the words.
  The project's owner reviews the docs one file at a time, and each review's changes land as they are decided.

## Where the hunt left off

* **The chain.**  It is stopped.  It ran on a friend's 88-thread machine and proved the games 4875 to 5060
  ([results/runs.md](../../results/runs.md) records the run); its search of game 5061 had taken 62 million nodes when it
  was stopped, and left no completed line.  Each search ran at idle priority (`--background`) and started from the better
  of two starting games ([the design](design/search.md#the-starting-incumbent-the-ceiling-and-the-seed)).  From game
  4995 on it branched on conflicts only at the nodes whose gap is above 1200 ([the settings
  checks](https://github.com/bvchess/taxman-notebook/blob/main/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-27-settings-checks-on-games-4991-4992-and-4995)).
* **The speed work.**  It ended with the hunt.  The chain recorded the profile of each game it searched (`taxman solve
  --profile`, [the design](design/engine.md#the-profile-of-a-search)) from 2026-10-23 to its stop.  What the profiles
  showed is in [the root gap and the size of a
  game](https://github.com/bvchess/taxman-notebook/blob/main/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md):
  a subtree grows about 2.6 times per 100 of gap, so the levers left were a tighter bound everywhere and a cheaper
  bound, which takes most of a step.  The last track, a gate chosen for each game by a trial, is in [a gate for each
  game](https://github.com/bvchess/taxman-notebook/blob/main/search/a-gate-for-each-game-2026-10/README.md).  The
  probe's windows are small under the current search, so speed work that resumes would first make windows of a hard game
  ([performance.md](design/performance.md)).
* **The lab.**  The studies that ran beside the chain were off from 2026-10-23, and the lab was removed from the solver in
  the cleanup; commit 3ca11bf still has it.  What the studies found is logged in [the live
  lab](https://github.com/bvchess/taxman-notebook/blob/main/search/the-live-lab-2026-10/README.md).  From game 4995 to
  the chain's stop one branching policy held for every game.
