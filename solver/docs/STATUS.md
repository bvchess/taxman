# Status

Updated 2026-10-22.

The state of the work as of the date above: the priority, and the projects in flight.  This page is rewritten, not
appended to, when the state changes.  What was done and why is in the [lab notebook](../notebook/README.md), how far the
chain reaches in [results/README.md](../../results/README.md), and the research avenues in the [agenda](AGENDA.md).

## The priority: a larger n

The end of the committed chain pushed past 5000, with the least computation.  Certifying statement U, for particular
games or in general, is not a priority.  Nothing is published: `optimal.json` stays at 1300.

## In flight

* **Extending the chain past 5000.**  The chain runs on a friend's 88-thread machine: 72 threads for the chain and 16
  for the lab below.  Game 4875 was proven at 7,476,374 on 2026-10-22, in 186 million steps and 56 minutes with conflict
  branching, the default since a88dc7f ([the design](design/search.md#conflict-branching)); before it, the search of
  4875 took 8.5 billion steps over 12 hours and did not finish.
* **The lab.**  Ten minutes into a game's search, the chain starts a study: an agent session in a sandbox that may steer
  the search through the lab channel, which cannot change a result, and then studies what it judges most likely to make
  the searches faster ([the runbook page](runbook/lab.md#studies), [the design](design/lab.md)).  It runs beside the
  chain since 2026-10-22, at most eight studies a day; what the studies find is logged in [the live
  lab](../notebook/search/the-live-lab-2026-10/README.md).
* **The documentation.**  Every long document is a folder of files with an index, and every doc follows
  [docs/WRITING.md](../../docs/WRITING.md), whose docs test checks the links, the citations, the sizes and the words.
  The project's owner reviews the docs one file at a time, and each review's changes land as they are decided.
