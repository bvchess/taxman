# The gate trial

Part of the [design](../DESIGN.md).

A search can ask its own units which gate of conflict branching to use, and by default it does not: on whole games a
fixed gate of 300 from the root took a quarter to a third of the time of the trial's choices.  The *gate* is the gap
from which a node branches on the conflict that the tactic `conflict_branching` chooses, rather than on the search's
own ([search.md, "Conflict branching"](search.md#conflict-branching)).  The best gate differs from game to game: on
units cut from the checkpoints of four games near n = 5000, a gate of 600 took 0.68 to 1.55 of the CPU of the gate of
1,200, and the spread of a game's open gaps stays the same through most of its search, so the game is what decides,
not the phase ([the notebook's
entry](https://github.com/bvchess/taxman-notebook/blob/main/search/a-gate-for-each-game-2026-10/README.md)).  So a
*gate trial*, which `--gate-trial GATES` turns on and `--gate-trial none` turns off, runs once in a game's search with
conflict branching on: it solves a sample of the search's own units under each gate of GATES and under the gate it has,
and keeps the gate that took the least CPU (`search::gate_trial`).  The gate a search has is the one it starts with:
the default, 300, or the `--gate GAP` of the run, which sets the starting gate of every game.  The trial of the chain's
re-run, which compared 1000, 800 and 600 against a starting gate of 1,200, is `--gate 1200 --gate-trial 1000,800,600`.

*Why the trial is off by default.*  The trial measures the wrong place.  A study on 2026-10-31 and 2026-11-01, with the
build of the chain's re-run (commit 3ca11bf), searched whole games on all 88 threads of a machine, with the same flags
for both policies (`--force-search`, and `--use-prev`, a flag that build had, for the moves for n − 1 from the committed
chain):

| game | fixed gate of 300 | gate of 1,200, then the trial over 1000, 800 and 600 |
|---|---|---|
| 4875 | 0.29 h, 173 M nodes (gate 150: 0.33 h, 92 M) | 1.25 h, 1,141 M nodes |
| 4879 | 0.28 h, 167 M | 1.27 h, 1,152 M |
| 4893 | 0.40 h, 221 M | 1.11 h, 972 M |
| 5052 | 1.97 h, 1,113 M | not done at 3 h, 2,610 M |

The trial chose 600 every time.  It takes 24 small subtrees from deep in the search, after 50 million steps, where a
low gate costs more than it saves: on 22 of the costliest games, the trial's pieces put a fixed gate of 300 at 1.70
times the CPU of 1,200.  But the gate matters most at the top of the tree, which the trial never sees and which is built
at 1,200 before the trial runs.  So the whole-game numbers decide, and the trial stays as an option for measuring.

*How it runs.*  The trial waits for the first checkpoint written once the search has taken 50 million steps (over every
run of it).  A thread beside the workers reads that checkpoint's units, a uniform sample of 2,000 if there are more, and
finds each one's gap at the checkpoint's incumbent, as the profile's summary of the open work does.  Of the units of a
gap of 300 or more it picks 24 at evenly spaced ranks of the gap, so that the picks follow the units' own mix and still
reach the largest.  It solves each, under each gate, as a unit of the engine of its own on one thread, from the
checkpoint's incumbent, to at most 150,000 steps, unit by unit with every gate in turn so that the gates see the machine
alike, on four threads.  Each solve has its own searchers, nogood tables and incumbent, the search's carried nogoods,
and a copy of its near-repeat cut.  A gate that took at least 3% less CPU than the search's own becomes the search's
gate, for the units its searchers start from then on.  `--gate-trial-plan` changes these numbers (`after`, `units`,
`steps`, `threads`, `least_gap`), for tests and for small games, and asks for a trial of 1000, 800 and 600 where
`--gate-trial` names none.  A search that resumes starts again from the gate its flags give (`--gate`, else the
default), not from the gate a trial chose before the stop, and runs its trial again at its first checkpoint: the steps
it waits for count over every run.

*Why it cannot make an answer wrong.*  The trial changes only the gate, and any gate keeps the search exhaustive: the
children of any true conflict cover every completion of their node, whatever the other nodes chose ([search.md, "The
invariant of a running search"](search.md#the-invariant-of-a-running-search)).  Its solves are searches of their own:
what they find, learn or cut stays in them, and they neither prune the search nor offer it anything.  The trial keeps
nothing in the checkpoint, which resumes with any gate.

*Where it starts.*  The gate the trial starts from is the run's `--gate`, else the default, and is tried against the
others.

The trial needs a checkpoint directory for the units; without one it does not run, and with conflict branching off the
gate means nothing, so it does not run either.

*What it costs and what it records.*  Near n = 5000, 24 units under three or four gates take 0.15 to 0.35 hours of one
core, a few minutes of four, at the scheduling priority of the workers, beside a search of hundreds of thread-hours.
The trial writes a line to stderr with each gate's CPU and what it chose.  The result line records it where it ran to
its end (`engine.gate_trial`: the search's steps at its checkpoint, the units, the gate it started from, each gate's
steps, CPU seconds and solves stopped by the limit, the gate chosen, and its seconds), and
`options.gate_trial` records the gates of the trial (absent without one), and `options.gate` the starting gate where
`--gate` was given.
