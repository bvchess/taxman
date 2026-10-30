# The engine and the per-game pipeline

Part of the [design](../DESIGN.md).

## Engine

The engine (`engine`) is a parallel depth-first branch-and-bound driver that knows nothing of Taxman: the Taxman search
is a `Searcher` that it runs.  The module comment of `src/engine.rs` has its protocol (conservation, splitting, spill,
checkpoint, termination), with the types (`Unit`, `Searcher`, `Ctx`, `EngineConfig`, `run`) next to it; this section is
the design in outline.

* **Units and workers.**  The caller's problem is a search tree whose subtrees are *units*, serialized by the `Unit`
  trait (for Taxman, two bitsets).  The engine runs `threads` OS threads (std only), each with its own `Searcher` made
  on that thread, so per-thread caches need no locks and, on a NUMA machine, each searcher's state is local.  The
  threads explore units depth first.  A searcher calls `ctx.poll()` at least every 1,000 nodes or so; when it answers
  `Yield`, the searcher pushes all of its unexplored work as units, shallowest first, and returns.
* **Work distribution.**  One shared deque holds the units, shallow at the front and deep at the back.  A worker that
  yields pushes its stack's remaining work and then pops from the back, resuming its own deepest work; idle workers take
  from the front, which holds the largest subtrees.  A worker yields when idle workers are left that the queued units
  cannot serve (a *split*), when a checkpoint is due, or when a stop is requested.  With one thread the deque is a pure
  stack: units are explored in exact depth-first order, whatever the checkpoints and the spilling.  The engine keeps one
  open split request per idle worker, and any running worker can claim one with an atomic decrement, so as many workers
  split at once as there are idle workers to feed.  A worker answers only after 64 polls of its unit, so that a fresh
  unit first gets deep enough to have something to give.  One locked deque is enough, because units exist only after
  yields and a rebuild costs the searcher far more than the lock ([the engine](../../notebook/the-engine/README.md)).
* **Memory bound.**  The in-memory deque holds at most `queue_mem_cap` units; beyond that the engine spills the oldest
  to segment files in `spill_dir`, and reloads them when memory runs dry.  Searchers keep only their stack and bounded
  caches.
* **Incumbent.**  An `AtomicU64` value with a payload behind a mutex (the promotion set, encoded).  Offers only ever
  raise it.  A searcher can declare it optimal, which ends the run, as an incumbent that reaches the ceiling given
  through the control (below) does.
* **Checkpoint.**  At every checkpoint interval (`EngineConfig::checkpoint`) all workers yield and park.  The engine
  writes the caller's header, the
  incumbent, statistics and every unit, in memory and spilled, to `<checkpoint>.tmp`, fsyncs it, renames it over the
  checkpoint, syncs the directory, and releases the workers.  Resuming loads the file and goes on, losing and
  duplicating nothing beyond partly explored nodes.  Any interval works, zero included.  The caller's header identifies
  the search, so that a checkpoint of another game, width, frames, candidates or tactics refuses to resume
  ([trust.md](trust.md)); a build that sets up the same search resumes it.  A
  run that completes deletes its checkpoint, unless the caller asks to keep it until it has recorded the result.  A
  caller's hook (`EngineConfig::checkpoint_hook`) is told of each checkpoint while the workers are parked, before the
  file is written and after: [the profile of a search](#the-profile-of-a-search) is written then.
* **Stopping and sharing.**  Time and node limits, and SIGINT and SIGTERM (after `stop_on_signals`), stop a run with a
  final checkpoint; a second signal kills the process.  A stop that already holds when a run starts (a time limit of
  zero, a signal received before) does no search work, however the threads are scheduled: the run checks before it
  starts a worker (engine docs, "Starting stopped").  `taxman solve` takes an exclusive `flock` on `taxman.lock` in its
  checkpoint directory, so a second process there fails at once.  Spill segments carry their process id, and a run
  deletes those of processes that no longer exist.  Progress lines and results on stdout ignore write errors, so a
  closed pipe cannot kill a run.  A panic in a release build aborts (`panic = "abort"`), and the previous checkpoint
  stays valid, which is the property that matters.
* **Control** (`engine::Control`).  Another thread steers a run through this handle while it goes: a ceiling (a proven
  upper bound, which ends the run once the incumbent reaches it), outside incumbents, a stop, a checkpoint now, and the
  number of workers.  The number grows by starting workers and shrinks by retiring workers that hold no unit (a running one hands
  all its units to the deque first); 0 suspends the search, its units waiting in the deque until the number rises
  again.  Requests made before the run apply when it starts (engine docs, "The control").
* **Instrumentation** (`engine::Counters`, per invocation).  The engine measures each worker's time, split into busy,
  idle, parked and waiting for the lock; the effective parallelism, busy time over capacity (the workers' lifetimes
  summed, the wall time times the threads when their number is fixed); splits and the units they gave, steals, runs
  and tiny runs; when all workers first ran at once; the workers added and retired; the idle time of the ramp and of
  the tail; the time from a unit's start to its first step (the rebuild) and the time in that step; and the pauses and
  the longest wait for the workers to park.  Every progress line ends with them in short ([REFERENCE.md, "Reading the
  output"](../REFERENCE.md#3-reading-the-output)).

## Per-game pipeline (`solver`)

Each game goes through six steps, from its board's tables to its line in the results file.  The shortcuts of steps
1a and 2, and the repeat in step 4, conclude a game without a search when another game already gives its answer.
`solver::solve_game` runs the steps for one game and `solve_range` for a range; the module comment of `src/solver.rs`
has the exact steps and checks.  The steps:

| step | what |
|---|---|
| 1 | the board's tables, and the set width |
| 1a | with `use_prev` and `known.boards` (`solve --use-prev`): a game that a known source has proven (the committed chain, the bounds store, the table, in that order) is taken with no search (method `known`; [Known games](trust.md#known-games)) |
| 2 | the prime shortcut: a prime n plays the game of n − 1 with its first move replaced by n (method `prime`) |
| 3 | the frames, formed, simplified, numbered and merged; with no candidates the game with no promotions is the only one (method `trivial`) |
| 4 | the search (method `search`, or `start` when its starting game reaches a ceiling before the search takes a step): the frame set and the greedy incumbent (`search::prepare`); a repeat of an earlier game's search (method `repeat`, tactic `same_search`; [same-search.md](same-search.md)); the starting incumbent, the ceiling and the seed; the engine; promotions → moves |
| 5 | `conclude`: replay and check the game, confirm its score against the built-in table, assign its proof and what it assumes |
| 6 | record the result (a range) |

The steps again, with the functions that take them:

```
 taxman solve A-B --out FILE --checkpoint-dir DIR
   |  solver::solve_range_each        n = A..B, concluded in order (with --window K up to K games in flight, each set
   |                                  up and searched on threads of its own: solver/window.rs); the game of n-1 comes
   |                                  from this run, FILE or (--use-prev) the known games (known::Known),
   |                                  and is checked (legal, over, opens with the largest prime) before anything uses it
   v
 solver::set_up(n) -> set_up_with_width::<W>      W = 8 to 128 words of 64 bits (bits::WIDTHS), by n
   |
   |  [1] numtheory::Tables            maximal factors and maximal composites of 1..=n; the set width W
   |  [2] n prime ............ accelerate::prime_shortcut   the game of n-1 with its first move replaced by n
   |                                                         method=prime        proof=prime_formula
   |  [3] board::Board::new            frames::BoardFrames: sources (never tax) + factors -> frames; unzip,
   |                                   removeImpossible
   |                                   frames::build_frames: frames with freedoms numbered from the front (0 = front)
   |                                   and merged; candidates; in_the_bag
   |      no candidates ...... the game with no promotions   method=trivial      proof=trivial
   |  [4] search ............. method=search                 proof=exhausted | cap
   |        Board::prepare             search::prepare: model::FrameGraph::build -> the root FrameSet; greedy set;
   |                                   max_total; the search::Problem it poses, and its digest
   |        same_search ...... one of the 8 games before it posed the same problem and settled it: its promotions
   |                                                         method=repeat       proof=same_search
   |        search::search             starting incumbent = best of greedy / seed (n-1's promotions) / rounded root
   |                                   matching + local search; ceiling = n + score(n-1) - in_the_bag
   |          engine::run              worker threads; shared deque with disk spill; incumbent; checkpoints; signals
   |            Stepper::step          per node (search/searcher.rs): check_node = the rule of symmetric pairs
   |                                   (symmetry, search::symmetry) -> GF(2) bound (gf2; with gf2=off
   |                                   the matching bound, matchbound, and its refinement) -> conflicts, forced
   |                                   members, nogoods (search::refine, model::Nogoods) -> with lagrange the
   |                                   Lagrangian bound over the nogoods (search::lagrange);
   |                                   choose_branch (a candidate) -> make_child -> check_child
   |      promotions -> accelerate::solution_for_promotions   (frames::BoardFrames with the promotions as sources)
   |                                                         -> ordered moves
   |
   |  [2] and same_search are skipped by --force-search and when the game has a checkpoint (a search is resumed,
   |  never replaced)
   v
 [5] solver::conclude                replay (game::replay_finished); score == in_the_bag + promotions; the ceilings;
   |                                 confirm the score against the built-in table; the proof and what it assumes
   v
 [6] results::ResultsFile::append    the JSON line, synced; only then is the game's checkpoint deleted;
                                     the game becomes the previous game of n+1
```

*The shortcuts.*  Step 2 needs the game of n − 1, and the repeat of step 4 an earlier game.  `force_search` skips both,
and step 1a, and so does a checkpoint of the game: a search that was interrupted is resumed, never replaced by a
derivation.  With `use_prev`, the search of step 4 takes two things from the bounds store for a game the store has not
proven: the game of its best lower bound, a seed beside the game of n − 1 (the search starts from the better start of
the two), and its best upper bound, a ceiling beside the one that the game of n − 1 gives (the lower of the two stops
the search).  `force_search` leaves the store out.

*The game of n − 1* is checked before anything uses it: it must replay legally to its recorded score, be over, and open
with the largest prime (`conclude` has already checked a game that this run made).  The results whose proofs rest on it
assume what it assumes ([trust.md](trust.md)).

*Limits.*  A search that a time or node limit stops gives an incomplete result (`completed` false, `best_so_far` the
best game so far) and keeps its checkpoint, and a range stops there (but see "Past limits", trust.md).

*Range runs* pass each game on to the next as its game of n − 1, and resume from the results file.  The solver checks
the file when it opens it: it replays every game, and checks the chain from n − 1 and each line's arithmetic and
provenance (the search problems that the checks of repeats set up again are set up first, on the run's threads).  A
range skips the games the file completed.  It appends each result and syncs it as soon as the game is concluded, in the
order of n, and only then deletes the game's checkpoint; a checkpoint left for a game that the file has completed is
deleted when a range starts.  The game of n − 1 comes from, in order: this run, the results file, and with `use_prev`
(`--use-prev`) the known games ([Known games](trust.md#known-games): the committed chain's, the bounds store's,
the built-in table's).  With none, the game is solved *standalone*, with no seed and no ceiling.

### The window: games in flight

A range keeps several consecutive games in flight, so that a machine of many threads stays busy where one game's search
cannot use them all.  A game of the chain needs two things of the game before it: its moves, for the seed, and its
score, for the ceiling.  Everything else (its board, its frames, its search problem, whether an earlier game settled
that problem) is its own, and the ceiling only stops a search early.  So a range keeps a *window* of K consecutive games
in flight (`--window K`, `SolveOptions::window`; 4 by default, as in `solver/tools/cloud/chain.sh`, and always 1 with
one thread).  Game n + 1 is set up while n searches.  Its search may start before n is proven, *speculatively*, seeded
from n's best game so far and without a ceiling, and it takes n's ceiling and the seed of n's final game the moment n is
concluded, through its engine's control ([Engine](#engine)).  `solver::window` is the driver.  With K = 1 it is the
range solved in order, line for line: a game enters the window only once the game before it is concluded, so its search
starts with the final seed and the ceiling, on every thread.

* *Phases.*  A game is set up on a thread of its own (the prime shortcut, the trivial game, or the search problem).  The
  driver looks its repeat up on its own thread, in the order of n; a game of the window that has set a search up counts
  ([trust.md](trust.md)).  The game's search runs on its own thread.  The driver concludes the game when it is the
  oldest game of the window and the game before it is concluded: it builds and checks the result (`conclude`), appends
  and syncs it, and only then tells the next game its ceiling.  A repeat waits for the game it repeats.
* *Seeds.*  A speculative search starts once the game before it has a game to seed it with: the concluded game, its
  search's best so far (a search offers its start to its control as the start is made; [search.md](search.md)), or what
  that gives through a repeat or the prime formula.  Every 5 ms the driver offers it the survivors of that game again
  whenever its score has improved, and at the conclusion those of the final game.
* *Deferring.*  A search whose seed's survivors reach the *provisional ceiling*, n plus the score of the best game so
  far of the game before, less `in_the_bag`, waits for that game's conclusion: if that best is final, the ceiling
  settles the game at once, and a speculative search of it would be wasted.
* *Threads.*  `--threads T` is a budget that the window shares (`solver::window::allot`, a pure policy, unit-tested).
  A setup holds one thread, and a search the workers of its engine, which takes on workers and retires them while it
  runs (`Control::set_workers`; 0 suspends it, its units waiting in its deque).  Every 5 ms the driver samples each
  engine and serves the claims, oldest game first:
  * a setup asks for one thread;
  * a search ready to start asks for every free thread if it is the only claim, and for one otherwise;
  * a busy search (idle under 5% of the last tick) asks to double;
  * a search idle 25% or more gives its idle workers back to another game's claim, unless it was given threads in the
    last 10 ms;
  * and the oldest search, if it is left short, reclaims threads from the younger ones, youngest first, suspending
    them if need be.

  So the game the chain waits for runs on every thread it keeps busy, speculative searches run on what it leaves, and
  the threads of a search that ends or idles go to the oldest game that can use them.  The driver does not estimate
  the tree: a search earns threads by keeping them busy, the online form of a running estimate of what is left of it.
* *Stops.*  The driver reports a failure, and ends the range at an unfinished search, only when its game is the
  oldest, so a run fails or stops at the game, and with the lines before it, where a range solved in order would.  It
  then stops the searches still in flight with a final checkpoint (the next run resumes them), and drops what they
  found.
* *The wall.*  Every game has a clock, its own search ([glossary](../../../docs/GLOSSARY.md#the-search)), which is not
  the time the game waited behind an older one, as its checkpoint's elapsed time counts it: a younger search is
  suspended while the oldest keeps the threads busy, so a game that follows a dear one would arrive at its own search
  with that time already on its clock.  The driver keeps the clock in `nNNNN.clock` beside the checkpoint, every
  checkpoint interval and when the search stops, reads it back when a search starts from a checkpoint, and writes it in
  the game's line as `seconds.own` ([reference](../REFERENCE.md#a-results-line---out-schema-5)).  A game that restarts
  from its beginning starts its clock again, and a finished game's clock goes with its checkpoint.  `--game-hours H`
  (`SolveOptions::game_limit`) ends the range at the first game whose clock passes H hours, to find the game at which a
  machine's day is not enough.  At the limit the driver stops the game's search, and once it has ended the range ends as
  it does at a stop signal, every search in flight stopped with a final checkpoint, but records nothing for the game,
  not even an incomplete line; `solve` prints `wall at N after H hours` and exits with the status 4
  (`RangeSummary::walled`).  A search that completes before the stop reaches it is recorded like any other, and the
  range goes on.  Run again with a larger limit, the game goes on from its checkpoint and its clock.
* *Trace.*  With `TAXMAN_WINDOW_TRACE` set, the driver prints its events on stderr: setups, the starts of searches with
  their seeds and ceilings, workers given and taken back, hints, deferrals and conclusions.

[Round B](../../notebook/round-b-the-chain-on-many-threads-2026-10/README.md) has what the window buys and what was
tried on the way.

**The table of optimal games is built into the binary** (`results/optimal.jsonl.gz`, through `include_bytes!` and
`game::Optimal::built_in`): the games of the file as it was when the crate was compiled, which a rebuild picks up when
the file changes.  The solver confirms every completed game's score against it, and a score that differs is a hard error
that names the game.  No option of `solve`, `regress` or `certify` names another table, so their runs cannot be checked
against anything else; the library takes another table where a test needs one (`solver::solve_range_each_with`,
`results::read_results`).

The file decodes to tens of megabytes, so the table is not decoded when a process starts.  The file is a sequence of
gzip members, each holding the lines of a batch of games, line n for game n.  `Optimal::built_in` decodes the members
through [the gzip crate](../../gzip/README.md) in order, only as far as the game asked for, and parses the line of a
game the first time it is asked for.  A process that asks for no game decodes nothing, one that asks for a few decodes
as far as the last of them, and one that goes over every game decodes the whole file.

A results file given by a path that ends in `.gz` is read the same way where it is used as a source of games: the
committed chain that `--use-prev` and `verify-results` draw on (`known::Known::load`), and the `--results` of `certify` and
`verify-certificates` and the `--known` of `bench` (`Optimal::with_results`).  Such a file must be one game a line, line n
for game n, as the committed results are.  It is read into memory when it is opened, with its first member decoded (so
that a file that is no gzip file is refused at once), and its other members are decoded as games are asked for, so a
check that cites few of its games reads a member or two of it.  A fault in a member that is decoded later stops the
call that needs the game and names the file.  A plain file is read whole when it is opened.  The measurement tools take
their `--known` files through `Optimal::with_results_files`, which checks that a game of a file scores what the games
before it say: for a plain file when it is opened, for a gzip file when a game of it is looked up, where a difference
stops the lookup and names the file, n and both scores.

## The profile of a search

The profile of a search records where the search's steps go, while it runs, so that the shape of a tree is measured
rather than estimated from one checkpoint.  `taxman solve --profile DIR` keeps one for every game it searches, in
`DIR/nNNNN.profile.jsonl`, and `solver/tools/read_profile.py` prints it: the depth histogram, the largest subtrees at
the depths where the tree branches with what was known when each was made, how much of the game the largest hold, how
their size follows their gap, and the timeline.  The profile observes the search and changes nothing of it: the same
game gives the same tree and the same answer with it and without, and it is no tactic, no part of a checkpoint's
header and no part of a results line.

**What it records.**  A node's *depth* is the number of branchings from the root to it, not its number of promotions
(a child that promotes forced members adds several).  Every node at depth D or less (`--profile-depth`) is *tagged*
with an id when it is made, up to the most ids a game may have (`search::PROFILE_MOST_IDS`).  Past them a new node
counts to its nearest tagged ancestor, and the profile says it is full.  The default depth (`search::PROFILE_DEPTH`) is
deep because the top of a tree is a spine: each node near the root makes one or two children, of which one holds
nearly all of the steps, and the steps spread over many subtrees only some twenty branchings down (the constant's
comment has the measurement).

The *own steps* of a tagged node are the steps taken at the node and, at depth D, in its whole subtree; a subtree's
steps are the own steps summed over it, which the reader does.  Each searcher keeps the ids of the nodes on its stack,
counts its steps by id and by the number of promotions of the node, and hands its counts to the profile when its run
ends (`search::profile`, `Tags`).

**The file** is JSON lines, appended, and synced at each checkpoint before the engine writes it:

| line | when | what |
|---|---|---|
| `profile` | first | n, the set width, the depth D and the most ids |
| `start` | each run | whether it resumed, whether the file has the tree from the root (`rooted`), the steps before, the threads, the incumbent, `unplaced` (steps the profile did not see), `frozen` (see "Resuming") |
| `node` | at the checkpoint after the node is made | `id`, `parent`, `depth`, `child` (its index among its parent's tagged children), `branch` (`default`: the search's own conflict; `conflict`: the conflict that conflict branching chose; `forced`: the parent's forced members), `added` (the promotions it added to its parent's), `promotions`, `sum`, `cands`, `bound`, `need`, `gap`, `t` (the search's time) |
| `steps` | each checkpoint, and the end of a run | `seq`, `nodes` (the search's steps, as the checkpoint records them), `counted`, `elapsed`, `ids` (each id's own steps in the interval), `unplaced`, `promotions` (the steps by number of promotions in the interval), `made` (ids so far), `full`, `phases` (see "The phases of a step") |
| `open` | after each checkpoint | its units by id, the spread of the gaps of a sample of them (at most `OPEN_BOUNDED` of `src/search/profile.rs`), and the seconds the summary took |
| `end` | the end of a run | `nodes`, `completed`, `best`, `elapsed`, `made`, `full` |

A node's `bound`, `need` and `gap` are those of the units file of `examples/tune.rs`: the GF(2) bound of what the node's
completions add (the matching bound on the matching path), the incumbent's sum less the node's plus one, and the
difference.  The bound is the one the node's own check found when it was made, whose limit on the number of promotions
counts the candidates before the child's were narrowed, so it can exceed the units file's.  The need is at the
incumbent of that moment; the reader also gives the gap at the final best.  The open units' gaps are computed after
each checkpoint by a thread beside the search, at the checkpoint's incumbent, as the units file computes them.

**Placing a unit by its promotions.**  A unit carries only its node's promotions and candidates (`search::TaxUnit`),
and the checkpoints keep that format, so a searcher that starts a unit finds the unit's id from its promotions
(`SearchProfile::locate`): from the root, it goes to the first child, in the order they were made, whose added
promotions the unit's node holds, for as long as there is one.  The walk is exact because sibling subtrees are
disjoint:

* A node v branches on a member x of a conflict (`choose_branch`): the child promotes x, alone or with the other
  forced members, and v takes x out of its candidates before its loop makes another child.
* A node's candidates are among its parent's when it is made (`make_child` narrows them, and a rule only drops more),
  so no later child of v, and no node below one, can promote x.
* So a node below v holds the added promotions of v's child c only if it is in c's subtree, and the first child whose
  added promotions it holds is the one whose subtree holds it.  v's own loop, the unit v leaves when its worker
  yields, holds no child's: the walk stops at v, as it should.

The unit's node is the tagged node the walk stops at when it has as many promotions as that node.  Debug builds check
the placement each time a searcher yields: every unit it hands over is placed where its node stands.  The profile
keeps of a tagged node only its depth, its added promotions and its children in order, a few words, where its promotion
and candidate sets would take 2 KB each at 128 words, and a unit is placed with a few bit tests a level.

**Resuming.**  The profile is synced before the engine writes a checkpoint, so it is never behind a checkpoint on disk.
A resumed search reads its profile up to the last `steps` line whose `nodes` are the checkpoint's, cuts the file
there (a crash between the two writes leaves the profile a block ahead), and goes on with its tree, its ids and its
counts.  A profile behind its checkpoint, after a failed write, counts the steps it lacks as unplaced and is *frozen*:
it tags no more nodes, since a unit below a node it lost is placed at that node's nearest ancestor in the file, which a
new child of that ancestor could take from it.  A search resumed with no profile, as from a checkpoint of a build
without it, gets one that cannot place its units: its steps are unplaced, and its counts by number of promotions and
its open units are recorded.  A resumed profile keeps its depth and its most ids.  A fresh search sets an earlier
file of its game aside, as `nNNNN.profile.jsonl.K`: a restart or a rerun does not lose it.

**The phases of a step.**  Each searcher times one step in `PHASE_EVERY` (`src/search/profile.rs`), reading the clock
as each phase of it ends, and a `steps` line has the nanoseconds of each phase in its interval, with the steps timed:
`bound` (the GF(2) bound's solve, with the child's level pushed, or the matching bound and its refinement),
`conflict` (the conflict of the heaviest set: the nogood table's lookup, and a peel where it has none), `pricing`
(conflict branching's choice), `lagrange` (the Lagrangian bound's check and refits), `branching` (strong branching's
solves), `promotion` (the child's), `children` (the child's candidates: narrowed, and ruled out by the known
conflicts, the nogoods and the rule of symmetry) and `other` (the cheap checks, the rule of symmetry and the
near-repeat cut at a node, the offers).  The time is the thread's on the wall, which is its CPU time while it runs.

**Cost.**  A step costs the profile a comparison and an increment; a tagged node, a lock and a line; a checkpoint, a
block written and synced while the workers are parked; and the thread beside the search, one GF(2) solve for each open
unit of its sample after each checkpoint (it takes the idle policy with `--background`).  A write that fails says so
on stderr and ends the profile; the search goes on.
