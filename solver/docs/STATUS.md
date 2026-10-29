# Status

Updated 2026-10-29.

The state of the work as of the date above: the priority, and the projects in flight.  This page is rewritten, not
appended to, when the state changes.  What was done and why is in the [lab notebook](../notebook/README.md), how far the
chain reaches in [results/README.md](../../results/README.md), and the research avenues in the [agenda](AGENDA.md).

## The priority: a larger n

The chain has passed game 5000, and so has the committed chain.  The priority is to push it further, with the least
computation.  Certifying [statement U](../../docs/theory/exactness.md#statement-u), for particular games or in general,
is not a priority.  The committed chain, `results/optimal.jsonl.gz`, is also the table built into the frame solver: the
chain's games are appended to it ([the runbook](runbook/chain.md)), and a rebuild takes them into the table.

## In flight

* **Extending the chain.**  The chain runs on a friend's 88-thread machine, and has proved games beyond the committed
  chain ([results/README.md](../../results/README.md) says how far that reaches).  Its search takes, at idle priority,
  every thread the machine leaves (`--background`), and starts from the better of two starting games ([the
  design](design/search.md#the-starting-incumbent-the-ceiling-and-the-seed)).  Since game 4995 it branches on conflicts
  only at the nodes whose gap is above 1200 ([the settings
  checks](../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md#2026-10-27-settings-checks-on-games-4991-4992-and-4995)).
* **Where a game's steps and time go.**  The chain records the profile of each game it searches (`taxman solve
  --profile`, [the design](design/engine.md#the-profile-of-a-search)), since 2026-10-23, and the work is to measure where
  the steps and the time go, and cut them.  There are two levers: a tighter bound everywhere, since a subtree grows about
  2.6 times per 100 of gap, and a cheaper bound, which takes most of a step.  The findings are in [the root gap and the
  size of a game](../notebook/search/the-root-gap-and-the-size-of-a-game-2026-10/README.md).
* **The lab.**  The studies that ran beside the chain are off since 2026-10-23, until a better approach; what they found
  is logged in [the live lab](../notebook/search/the-live-lab-2026-10/README.md).  Since game 4995 one branching policy
  holds for every game; the aim is a single rule judged on a panel of the hard games, not settings chosen game by game.
* **The documentation.**  Every long document is a folder of files with an index, and every doc follows
  [docs/WRITING.md](../../docs/WRITING.md), whose docs test checks the links, the citations, the sizes and the words.
  The project's owner reviews the docs one file at a time, and each review's changes land as they are decided.
