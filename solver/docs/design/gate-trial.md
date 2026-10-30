# The gate trial

Part of the [design](../DESIGN.md).

The best gate of conflict branching differs from game to game, so a search can ask its own units which gate to use.
The *gate* is the gap from which a node branches on the conflict that the tactic `conflict_branching` chooses, rather
than on the search's own ([search.md, "Conflict branching"](search.md#conflict-branching)).  On units cut from the
checkpoints of four games near n = 5000, a gate of 600 took 0.68 to 1.55 of the CPU of the gate of 1,200, and the
spread of a game's open gaps stays the same through most of its search, so the game is what decides, not the phase
([the notebook's entry](../../notebook/search/a-gate-for-each-game-2026-10/README.md)).  With `--gate-trial GATES`, each
game's search with conflict branching on runs a *gate trial* once: it solves a sample of its own units under each gate
of GATES and under the gate it has, and keeps the gate that took the least CPU (`search::gate_trial`).  The gate a
search has is the one it starts with: the default, or the `--gate GAP` of the run, which sets the starting gate of every
game, so that `--gate 1200 --gate-trial 1000,800,600` compares 1000, 800 and 600 against 1200.

*How it runs.*  The trial waits for the first checkpoint written once the search has taken 50 million steps (over every
run of it).  A thread beside the workers reads that checkpoint's units, a uniform sample of 2,000 if there are more, and
finds each one's gap at the checkpoint's incumbent, as the profile's summary of the open work does.  Of the units of a
gap of 300 or more it picks 24 at evenly spaced ranks of the gap, so that the picks follow the units' own mix and still
reach the largest.  It solves each, under each gate, as a unit of the engine of its own on one thread, from the
checkpoint's incumbent, to at most 150,000 steps, unit by unit with every gate in turn so that the gates see the machine
alike, on four threads.  Each solve has its own searchers, nogood tables and incumbent, the search's carried nogoods,
and a copy of its near-repeat cut.  A gate that took at least 3% less CPU than the search's own becomes the search's
gate, for the units its searchers start from then on, as the lab's `branching` command sets it.  `--gate-trial-plan`
changes these numbers (`after`, `units`, `steps`, `threads`, `least_gap`), for tests and for small games.  A search
that resumes starts again from the gate its flags give (`--gate`, else the default), not from the gate a trial chose
before the stop, and runs its trial again at its first checkpoint: the steps it waits for count over every run.

*Why it cannot make an answer wrong.*  The trial changes only the gate, and any gate keeps the search exhaustive: the
children of any true conflict cover every completion of their node, whatever the other nodes chose ([lab.md,
(b)](lab.md#the-invariant)).  Its solves are searches of their own: what they find, learn or cut stays in them, and they
neither prune the search nor offer it anything.  The trial keeps nothing in the checkpoint, which resumes with any
gate.

*Who decides.*  The lab's word holds.  A game whose lab record has a gate of its own (the lab's `branching` command)
runs no trial.  If the lab changes the branching settings once the search has started, by a `branching` command or a
policy given while it runs, the trial does not run, or does not set its gate, and says so.  A policy's gate given
before the search started is the gate the trial starts from, and is tried against the others.  So is the run's
`--gate`, for a game whose lab record and policy give no gate: the lab's word holds over the flag as it does over the
default.

The trial needs the tactic `gf2` for the gaps and a checkpoint directory for the units; without either it does not run,
and with conflict branching off the gate means nothing, so it does not run either.

*What it costs and what it records.*  Near n = 5000, 24 units under three or four gates take 0.15 to 0.35 hours of one
core, a few minutes of four, at the scheduling priority of the workers, beside a search of hundreds of thread-hours.
The trial writes a line to stderr with each gate's CPU and what it chose.  The result line records it where it ran to
its end (`engine.gate_trial`: the search's steps at its checkpoint, the units, the gate it started from, each gate's
steps, CPU seconds and solves stopped by the limit, the gate chosen, whether the search took it, and its seconds), and
`options.gate_trial` records the gates where the flag was given, and `options.gate` the starting gate where `--gate`
was.
