# Status

Updated 2026-10-21.

The state of the work as of the date above: the priority, and the projects in flight.  This page is rewritten, not
appended to, when the state changes.  What was done and why is in the [lab notebook](../notebook/README.md), how far the
chain reaches in [results/README.md](../../results/README.md), and the research avenues in the [agenda](AGENDA.md).

## The priority: a larger n

The end of the committed chain pushed past 5000, with the least computation.  Certifying statement U, for particular
games or in general, is not a priority.  Nothing is published: `optimal.json` stays at 1300.

## In flight

* **Extending the chain past 5000.**  The committed chain ends at 4874, and the next run waits on a hardware plan.
  The run past 5000 added 4548..4874 and stopped in game 4875, by the choice of whoever owns the AWS account, to plan
  different hardware for the next run.  Game 4875 had searched 6.9 billion nodes without finishing; its checkpoint is
  kept outside the repository, and resumes on the build that made it and on master ([the
  entry](../notebook/the-run-past-5000-2026-10/README.md)).  The bounds store has proven no game past 4783, so every
  game from 4875 on needs a search or a shortcut ([results/README.md](../../results/README.md#strictjsonl)).  A finder
  that starts beside a long search is an option for 4875 (`--finder-after`, `--finder-seconds`; [the
  entry](../notebook/search/the-finder-on-long-searches-and-game-4875-2026-10/README.md)).  A step of the search costs
  about 47% fewer cycles than in the run that reached 4547, with the same results ([the
  entry](../notebook/search/what-a-gf2-step-carries-from-the-last-same-trees-2026-10/README.md)).  Smaller trees would
  have to come from the bound, and every avenue tried is closed: branching has little headroom ([branching in
  hindsight](../notebook/search/the-headroom-of-branching-in-hindsight-2026-10/README.md)), the knot learning does not
  pay on whole games ([the
  verdict](../notebook/search/conflicts-closed-under-cheap-exchanges-2026-10/README.md#later-2026-10-20-closed-not-adopted)),
  and the bound's error sits in frame 2, whose exact problem costs what the search does
  ([frame 2](../notebook/search/the-exact-problem-of-frame-2-2026-10/README.md)).  An idea that changes the tree is
  judged on whole games, not units.  Next: a run from 4875 past 5000, which needs the account owner's approval with a
  cost estimate first.
* **The documentation.**  Every long document is a folder of files with an index, and every doc follows
  [docs/WRITING.md](../../docs/WRITING.md), whose docs test checks the links, the citations, the sizes and the words.
  The project's owner reviews the docs one file at a time, and each review's changes land as they are decided.
