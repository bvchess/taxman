# Status

Updated 2026-10-20.

The state of the work as of the date above: the priority, and the projects in flight.  This page is rewritten, not
appended to, when the state changes.  What was done and why is in the [lab notebook](../notebook/README.md), how far the
chain reaches in [results/README.md](../../results/README.md), and the research avenues in the [agenda](AGENDA.md).

## The priority: a larger n

The end of the committed chain pushed past 5000, with the least computation.  Certifying statement U, for particular
games or in general, is not a priority.  Nothing is published: `optimal.json` stays at 1300.

## In flight

* **Extending the chain past 5000.**  The committed chain ends at 4547
  ([results/README.md](../../results/README.md#strictjsonl)); the bounds store has proven 215 games of 4548..4783, which
  the next run takes with no search (`--use-prev`).  A step of the search now costs 31-39% fewer cycles, with every tree
  the same ([the entry](../notebook/what-a-gf2-step-carries-from-the-last-same-trees-2026-10/README.md)), and a second
  round of such savings is under way.  Smaller trees must come from the bound: branching has little headroom ([branching
  in hindsight](../notebook/the-headroom-of-branching-in-hindsight-2026-10/README.md)) and the knot learning is closed
  ([the
  verdict](../notebook/conflicts-closed-under-cheap-exchanges-2026-10/README.md#later-2026-10-20-closed-not-adopted)).
  A study under way asks whether the bound's error sits in one home, where an exact local bound could remove it ([the
  lead](../notebook/decomposing-across-the-frames-a5s-cheapest-experiment-2026-10/README.md)).  An idea that changes the
  tree is judged on whole games, not units.  The next run, from 4548, needs the account owner's approval.
* **The documentation.**  Every long document is a folder of files with an index, and every doc follows
  [docs/WRITING.md](../../docs/WRITING.md), whose docs test checks the links, the citations, the sizes and the words.
  The project's owner reviews the docs one file at a time, and each review's changes land as they are decided.
