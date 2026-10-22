# The lab channel

Part of the [design](../DESIGN.md).

A *lab* is a program that studies the dearest searches of a live chain and steers them: it may write checkpoints,
offer games, change how nodes branch, restart a game, move threads, run a finder, or ask for another build.  The *lab
channel* is how a run of `taxman solve` lets it: a *hook*, a program the run starts when a game runs long, and a
*command folder*, from which the run takes the lab's commands.  Whatever the lab does, a result is never wrong, because
the lab can change how and when the games are searched but never what proves them.  This file is that argument, then
the channel itself.  [REFERENCE.md](../REFERENCE.md#1-the-flags-of-solve) has the flags, `--lab-dir`, `--hook` and
`--hook-after`, and `solver/tools/cloud/chain.sh` passes them as `LAB_DIR` and `LAB_HOOK`.

## The invariant

The lab can affect only:

* **(a)** the order and allotment of work and threads;
* **(b)** branching choices;
* **(c)** the incumbent, through validated offers;
* **(d)** whether a game restarts fresh, and with which tactics;
* **(e)** which build of `origin/master` runs.

It can never close a subtree, set a bound, add a nogood, change a pruning tactic in the middle of a search, or write a
result.  Each command below touches one of (a) to (e), and the code holds to the list: the driver of the window
(`src/solver/window.rs`) carries the commands out with the means the solver already had for each, and nothing of the
lab reaches the bounds, the nogoods or the results file.

**(a) Work and threads.**  A search's answer does not depend on which worker explores which unit, or when.  The engine
conserves every unexplored subtree as it grows and retires workers, and while a search is suspended its units wait in
its deque ([engine.md, "Engine"](engine.md#engine), the item "Control").  A checkpoint asked for early is the same
pause and write as a periodic one.  The window's sharing of threads is no part of any proof: a window of K games gives
the scores and proofs of a window of 1 ([trust.md, "What a window changes in the
results"](trust.md#what-a-window-changes-in-the-results)).

**(b) Branching.**  The children of any true conflict cover every completion of their node, whatever the other nodes
chose: every promotable completion leaves out some member of a conflict, so branching on its members, each child without
one, explores every promotable set ([search.md, "The default step"](search.md#the-default-step), fact 2).  The conflicts
that `conflict_branching` chooses among are true conflicts: the search's own, the nogoods within the heaviest set, and
the minimal conflicts that `FrameGraph::conflicts_ordered` makes, each checked unwinnable by the whole peel
(docs/theory/decision-procedure.md, Theorems 3 and 4; docs/theory/frame-model.md, Theorem M; [search.md, "Conflict
branching"](search.md#conflict-branching)).  The look that `conflict_look` makes chooses between two of them, the
search's own conflict and the one `conflict_branching` chose, and leaves the node's state as it found it ([search.md,
"Conflict branching"](search.md#conflict-branching), the paragraph "Why the look is exact").  The lab only switches the
two on or off, and a searcher reads the switches when it starts a unit.  A unit is a node with the candidates it has
left, whose subtree its searcher explores to the end, so a search whose units branched differently is exhaustive unit by
unit.  The nogoods learned on the way are conflicts whichever conflict a node chose, and they hold at every node
([search.md, "The default step"](search.md#the-default-step), fact 4).

**(c) The incumbent.**  An offer becomes the incumbent only through `search::offer_found`: the set must be a set of
candidates that can be promoted from the root, worth what the offer claims, the same check as a checkpoint's incumbent
on resuming.  A promotable set is a game, so its sum is at most P\*, the game's best promotion sum.  The incumbent only
prunes nodes whose bound is below it plus one, which hold no completion beating it, so the search still finds P\* and
proves it.  The finder the lab may start offers its games the same way ([search.md, "The
finder"](search.md#the-finder)).  A set the check refuses changes nothing, and the lab hears why.

**(d) A fresh search.**  A restarted game is searched from nothing: its root, the run's seed and ceiling, and the
tactics the lab names.  Every tactic is proven or sound ([tactics.md](tactics.md)), so any set of them gives a complete
search, whose proof is its own tree, exhausted or stopped at the ceiling.  The abandoned search contributes nothing: its
checkpoint is renamed aside in the checkpoint directory, never deleted, and never resumed by the new search.  Its line
records the tactics it ran with and that the lab restarted it, and when.

**(e) The build.**  A redeploy only stops the run, with a final checkpoint of every search, as SIGTERM does.  The
chain's script then builds the commit only if it is `origin/master` or an ancestor of it, so a lab's own experimental
builds never run in the chain, and the build resumes a checkpoint only if it sets up the same search
([trust.md](trust.md), the paragraph "What identifies a checkpoint").

**The solver is the only writer of results.**  The lab writes only to the folder's `in/`.  The driver concludes every
game as it does without a lab: the referee replays it, `conclude` checks its arithmetic, its ceilings and
`optimal.json`, and only then is its line appended.  What the lab did to a game is kept in `nNNNN.lab` beside its
checkpoint, and sets nothing but the game's search tactics, any of which is sound, and the record its line carries.

## Which tactics may change while a search runs

A tactic may change between a search's units only if it chooses how a node branches and prunes nothing.  Two do:

| tactic | kind | why |
|---|---|---|
| `conflict_branching` | branching: changeable | it chooses which true conflict a node branches on, and the children of any conflict cover the node's completions; a conflict all of whose members are forced closes the node, which is the same deduction branching on it would make |
| `conflict_look` | branching: changeable | where `conflict_branching` chooses, it chooses between the search's own conflict and the one chosen, both true conflicts; the child it looks at is made aside and unmade, and the conflict that cuts it is a nogood, which holds at every node |

A checkpoint records `conflict_branching` and `conflict_look` (bits 9 and 10 of its tactics,
`SearchTactics::BRANCHING_BITS`) as the search began with them, and does not compare them on resuming, so a checkpoint
resumes with each tactic on or off.  The searchers read them through `search::Branching`: a generation number, loaded
when a searcher starts a unit, and the settings, read again only when the generation has changed.  A step reads the
searcher's own copy.  So a checkpoint of a search begun with conflict branching off, game 4875's among them, resumes
with it on.  A result line records the settings the search ended with in `tactic_switches`, and the lab's changes in
`lab`.

The others stay fixed for the life of a search, each for its reason:

| tactic | why it is fixed |
|---|---|
| `match_refine` | refines the matching bound, which cuts nodes |
| `fix_forced` | ends a node's loop after the child that promotes the forced members: it prunes the other children |
| `nogoods` | rules candidates out of children, and cuts a node whose conflict is all forced or promoted |
| `lagrange` | a bound, which cuts nodes and finds forced members |
| `gf2` | the node bound itself |
| `symmetry` | cuts nodes, and drops candidates no completion keeping the inequalities holds; its trees rest on Theorem 13 for the whole search |
| `force_window` | says which members are searched for being forced, and forced members end loops |
| `branching`, `strong_limit`, `strong_depth`, `deep_branching`, `more_forced` | they choose the member to branch on, but strong branching also finds forced members, the members without which the bound falls below the need, and those end loops |

The policy of the nogood tables, `forget_nogoods`, `nogood_period`, `nogood_keep` and `nogood_limit`, is a cache's,
sound whatever it says, and was free already: no checkpoint records it, and the run that resumes a search sets it.  The
lab does not switch it during a search, since a searcher's table is its own and nothing needs it.  The tactics outside
the search, `prime_shortcut`, `same_search`, `prev_cap` and `seed_incumbent`, decide whether a game is searched, its
ceiling and its seed: a restart keeps the run's, and the lab names search tactics only.

## The hook

`--hook PROGRAM` names a program the run starts when a game runs long.  The run starts it detached, in a process group
of its own, with no input or output and the event as its one argument, a JSON object.  It never waits for the program
and ignores its failure; each start, and each failure to start, is a line on stderr.  A thread of its own collects the
program when it ends, so that none is left a zombie.

* **`long`**, once a game: when the game's own search ([glossary](../../../docs/GLOSSARY.md#the-search)) passes
  `--hook-after` seconds (3600 by default), as soon as its engine runs.
* **`done`**, when a game that had `long` has its line.

The argument is one JSON object, with these fields; a program should ignore any other:

| field | what it holds |
|---|---|
| `event` | `"long"` or `"done"` |
| `n` | the game |
| `nodes` | its search's steps so far, over every run of it (for `done`, the line's `nodes`) |
| `best` | its best promotion sum so far (for `done`, the sum of the line's promotions) |
| `own_seconds` | its own search time, in seconds |
| `checkpoint_dir` | the checkpoint directory, an absolute path, or null without `--checkpoint-dir` |
| `lab_dir` | the command folder, an absolute path, or null without `--lab-dir` |
| `commit` | the commit the build was made from, or null if it is not known |
| `result` | for `done` only: the game's line, as an object, as the results file has it |
| `pid`, `at` | the run's process, and the time in UTC |

Once a game holds over invocations: the game's record, `nNNNN.lab`, keeps that `long` was sent.  Without `--hook`, the
driver makes one test a loop and starts nothing.  The lab's own tools are in [runbook/lab.md](../runbook/lab.md).

## The command folder

`--lab-dir DIR` makes `DIR/in/`, `DIR/out/` and `DIR/done/`.  At each progress tick (every `--progress` seconds, or 60
when the run prints no progress lines) the driver takes the files of `in/`, in the order of their names, and for each:

1. moves it to `done/` (beside an earlier one of the same name, as `NAME.1`, `NAME.2`, ...);
2. reads it as one JSON command, checks it against the run, and applies it or refuses it;
3. writes the outcome to `out/NAME`: `{"command": ..., "status": "applied" or "refused", "reason": ..., "at": ...}`.

A file whose name starts with a dot is left alone, so a lab writes a command under such a name and renames it into
`in/`.  The work is the driver's, on its own thread: no worker reads the folder, and with an empty folder a tick costs
one read of a directory.  A command names a game by `n`, which must be in flight unless it says otherwise; a field the
command does not take is refused, so a misspelt one is not ignored.

| command | fields | what it does |
|---|---|---|
| `checkpoint` | | every running search writes its checkpoint at once (a pause, as at the interval) |
| `offer` | `n`, `set`, `value` | offers the promotions `set`, claimed to be worth `value` (their sum when left out), through `offer_found`; applied whether or not the set beats the incumbent, and the reason says which |
| `branching` | `n`, `conflict_branching`, `conflict_look` | `"on"` or `"off"` for either or both (at least one), for the units game n's searchers start from then on; kept in the game's record, so it holds over invocations; a field left out keeps its setting; `conflict_look` acts only where `conflict_branching` is on |
| `restart` | `n`, `tactics` | abandons game n's search and starts it fresh with the run's search tactics changed by `tactics`, `name=value` separated by commas; a running search is stopped first, with a final checkpoint, and the outcome is written once it has stopped |
| `threads` | `total` | the run's budget of threads; a search above its share gives workers back, the youngest first |
| `threads` | `n`, `workers` | game n's search keeps `workers` workers, outside the policy that shares the rest, until it is concluded or `workers` is `null`; 0 suspends it, or keeps it from starting; n may be any game of the range not yet concluded |
| `finder` | `n`, `seconds` | a finder beside game n's running search for that long; it ends with the search, and its report goes to `out/report-NAME` |
| `redeploy` | `commit` | ends the run, every search stopped with a final checkpoint: `solve` prints `redeploy to COMMIT` and exits with status 5 |

*A restart.*  Game n's checkpoint becomes `nNNNN.ckpt.restartK` and its clock `nNNNN.clock.restartK`, for its K-th
restart, names that no tool takes for a game's checkpoint or clock (`n*.ckpt`), and its record, `nNNNN.lab`, gets the
restart: when, the game's own seconds then, the steps of the search abandoned, the tactics, and the name of the
checkpoint put aside.  The game is then set up again, with those tactics, and searched whatever an earlier game settled,
as a game with a checkpoint is; its clock starts again, as a fresh search's does.  A search that completes before it
stops is not restarted, and neither is one that a stop signal ends.  The record goes into the game's line as `lab`, with
the tactics the search ran with in `tactics` and `tactic_switches`.  A run without `--lab-dir` reads the record too, so
it resumes a restarted game with its tactics.

*A redeploy.*  `chain.sh`, seeing the status 5, reads the commit from the line, fetches `origin` and checks that the
commit is `origin/master` or an ancestor of it.  If it is, the script makes a worktree of the commit under
`$TAXMAN_WORK/builds/`, builds it with that commit's `setup.sh`, as a machine's build is made, and runs the same command
with it; if not, or if the build fails, it says why and runs the build it has.  The checkpoints resume on the new build
only if it sets up the same searches; otherwise the solver refuses them, saying why, and the script ends.

*What is refused, and why.*  A command is refused, with its reason in `out/`, when it cannot be read or is not a known
command; when its game is not in the range, is concluded, is not in flight, or has no search of its own (a repeat, a
prime or trivial game, one still being set up); when an offer fails `offer_found` (a number off the board or twice, a
set that cannot be promoted, a claimed value that is not its sum); when a restart names a tactic that is not a search
tactic, or a value a tactic does not take; when the threads that games are given exceed the run's; and, for
`checkpoint` and `redeploy`, when the run keeps no checkpoints.

## Room for advice

A later command may advise a search to branch a given unit on a given conflict: `{"command": "advice", "n": N,
"unit": {"promoted": [...], "cands": [...]}, "conflict": [...]}`.  Before a searcher used it, the conflict would be
checked a true conflict by the whole peel at that unit, as `conflicts_ordered` checks its own, so it would be one more
branching choice, (b).  The format leaves room for it: the name `advice` is reserved, and refused for now.

## What was decided against

* *Reading the folder in the searchers.*  A searcher never looks at the lab: the driver reads the folder, and a searcher
  reads a generation number when it starts a unit.  A search without a lab takes the same steps as before.
* *Changing strong branching's settings live.*  They choose a member, but they also find forced members, which prune.
* *Restarting with tactics outside the search.*  `prev_cap` or `seed_incumbent` would change the game's ceiling or seed
  apart from the run's; the lab offers games through (c) instead.
* *Recording in the checkpoint the setting in force.*  The engine resumes a checkpoint only under the very header it was
  written with, so bit 9 says how the search began; the line says how it ended.
* *Waiting for the hook, or killing it.*  The run never depends on the lab's program.
* *Deleting the checkpoint of a restarted search.*  It is renamed aside; what to keep is the lab's choice.
