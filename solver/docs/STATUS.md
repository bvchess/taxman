# Status

Updated 2026-10-29.

The state of the work as of the date above: the priority, and the projects in flight.  This page is rewritten, not
appended to, when the state changes.  What was done and why is in the [lab notebook](../notebook/README.md), how far the
chain reaches in [results/README.md](../../results/README.md), and the research avenues in the [agenda](AGENDA.md).

## The priority

The hunt for a larger n ended on 2026-10-29: the project's owner ended it at 17:20 PDT, and the chain was stopped at
18:01 PDT, after game 5060.  The committed chain, `results/optimal.jsonl.gz`, reaches game 5060.  The next steps are to
be planned with the owner.  Certifying [statement U](../../docs/theory/exactness.md#statement-u), for particular games
or in general, is not a priority.  The committed chain is also the table built into the frame solver: the chain's games
are appended to it ([the runbook](runbook/chain.md)), and a rebuild takes them into the table.

## In flight

* **The documentation.**  Every long document is a folder of files with an index, and every doc follows
  [docs/WRITING.md](../../docs/WRITING.md), whose docs test checks the links, the citations, the sizes and the words.
  The project's owner reviews the docs one file at a time, and each review's changes land as they are decided.
* **The re-run.**  The chain runs again from game 1 to 5060 on a friend's machine of 88 threads, with one
  build (the tag `rerun-2026-10`, commit 3ca11bf) and one policy, so that its timings come from one machine and one
  build.  It started on 2026-10-29 at 19:55 PDT as `chain.sh 5060` with `--gate 1200 --gate-trial 1000,800,600 --tactic
  price_reuse=on --tactic warm_start=on --finder-seconds 900 --background --carried-dir DIR --profile DIR` (DIR is a
  folder of the run) and no `--use-prev`.  That build needs the policy flags; the solver of the master branch has the
  policy as its defaults (a gate of 1200, the gate trial of 1000, 800 and 600, `price_reuse` and `warm_start` on, a
  finder of 900 seconds), so a plain `taxman solve` searches as the re-run does.  Only the flags that depend on the
  machine and the run (`--background`, `--carried-dir`, `--profile`) are still given.  When it completes, its file
  replaces `results/optimal.jsonl.gz`.
* **The cleanup of the frame solver.**  The project's owner decided on 2026-10-29 to remove the lab, the studies and the
  idle experiments; to make the production policy the default; to split the notebook into a repository of its own; and
  to leave the prover alone.

## Where the hunt left off

* **The chain.**  It is stopped.  It ran on a friend's 88-thread machine and proved the games 4875 to 5060
  ([results/runs.md](../../results/runs.md) records the run); its search of game 5061 had taken 62 million nodes when it
  was stopped, and left no completed line.  Each search ran at idle priority (`--background`) and started from the better
  of two starting games ([the design](design/search.md#the-starting-incumbent-the-ceiling-and-the-seed)).  From game
  4995 on it branched on conflicts only at the nodes whose gap is above 1200 ([the settings
  checks](../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-27-settings-checks-on-games-4991-4992-and-4995)).
* **The speed work.**  It ended with the hunt.  The chain recorded the profile of each game it searched (`taxman solve
  --profile`, [the design](design/engine.md#the-profile-of-a-search)) from 2026-10-23 to its stop.  What the profiles
  showed is in [the root gap and the size of a game](../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md):
  a subtree grows about 2.6 times per 100 of gap, so the levers left were a tighter bound everywhere and a cheaper bound,
  which takes most of a step.  The last track, a gate chosen for each game by a trial, is in [a gate for each
  game](../notebook/search/a-gate-for-each-game-2026-10/README.md).
* **The lab.**  The studies that ran beside the chain were off from 2026-10-23, and the lab itself (the hook, the
  command folder and the studies' tools) was removed from the solver on 2026-10-29; it builds at commit 3ca11bf.  What
  the studies found is logged in [the live lab](../notebook/search/the-live-lab-2026-10/README.md).  From game 4995 to
  the chain's stop one branching policy held for every game.
