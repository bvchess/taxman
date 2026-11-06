# Results and the trust model

Part of the [design](../DESIGN.md).

Every completed line names how its game was proven optimal, its *proof kind*, and what that takes as given beyond the
frame model, what it *assumes*.  The referee replays every game before it counts, and two verifiers that share no code
check every results file.  [REFERENCE.md, "Why the answers can be
trusted"](../REFERENCE.md#4-why-the-answers-can-be-trusted) has the proof kinds, the assumptions and the verifiers as a
results line shows them, [frame-model.md](frame-model.md) says what each proof kind needs of the frame model, and
[results/README.md](../../../results/README.md#what-supports-the-scores) says what supports each committed score.  This
file is the design behind them; `src/results.rs` has the full rules and the checks.

**What a game assumes** is inherited down the chain: a game whose proof kind rests on the moves for n − 1 and a repeat
each assume what the game they rest on assumes ([REFERENCE.md, "Assumptions"](../REFERENCE.md#assumptions)).  A game's
own search assumes nothing.  So a chain from n = 1 assumes nothing: it is proven within the frame model, given that the
solver is right, and the only moves that could add an assumption are previous moves that a caller of the library gives a
run.  The solver refuses no line for what it assumes: the line says it, and the verifiers count it.

**Checks and referees.**  `game::replay_finished` replays every game before it counts, the solver checks its arithmetic
and the ceilings `score(n−1) ≤ score(n) ≤ n + score(n−1)`, and two verifiers read results files without changing them:
`taxman verify-results`, and `solver/tools/check_results.py`, an independent referee in Python
([verifiers.md](../verifiers.md) says what each checks).  Both read results lines of schema 5 only.  Both derive the
proof kind and the assumptions again from a line's fields, so both refuse a line whose fields contradict each other (say
a sum at the ceiling without `reached_cap`).  No check can catch a game that is legal, finished and within the ceilings
but not optimal, when the moves for n − 1 are not optimal either: that is what the proof kinds and the assumptions
track, and why a chain from n = 1 matters.

**Result lines** are one JSON object per line ([REFERENCE.md, "A results line
(`--out`, schema 5)"](../REFERENCE.md#a-results-line---out-schema-5)).  A line is never rewritten, and a run locks the
results file it appends to.

**What identifies a checkpoint.**  The header that a search stores identifies the search, so that a build never resumes
a checkpoint of another search.  It holds:

* a format version, the set width and n;
* digests of the root's candidates and of the frames (every frame's moves, factors and freedoms, as built);
* `max_total` and the search tactics, one bit for each that is on ([tactics.md](tactics.md)).

The header's format is version 8, and this build reads no other: a header of any other version is refused with one
message, naming the version, and a search is finished by the build that made it.  The finder (`--finder-after`,
[finder.md](finder.md)) is in no version: it only offers sets, each after the check below, so a checkpoint resumes with
any finder or none.  The switches `conflict_branching` and `conflict_look` have a bit each among the search tactics,
which records how the search began and binds nothing, since the switches only choose which true conflict a node branches
on ([search.md, "The invariant of a running search"](search.md#the-invariant-of-a-running-search)): a checkpoint
resumes with them on or off.  So does `near_repeat`, whose bit binds nothing either: each run of a search sets the cut
up again from the game before, as it takes its ceiling.  So does `price_reuse`, which only changes how the prices that
rank the conflicts are made, so does `carried_nogoods`, whose store holds only nogoods checked at the root
([carried-nogoods.md](carried-nogoods.md)), and so does `warm_start`, which only changes which of several heaviest sets
of the same weight a node finds.  The other switches bind: a checkpoint is resumed only by a search with the same
`fix_forced`, `nogoods`, `forget_nogoods` and `symmetry`.

Resuming checks all of these and says which differs.  It also checks that the stored incumbent is a set of candidates
that can be promoted from the root and is worth the stored value.  So a checkpoint of another game, or of an older build
whose frame builder differs, is refused, instead of pruning with a foreign incumbent or failing to rebuild its units.  A
resumed game rests on nothing that a game run in one piece does not.

## What a window changes in the results

A range solved with a window of games in flight ([Per-game pipeline (`solver`)](engine.md#per-game-pipeline-solver))
records what the range solved in order records, but for what a multi-threaded search may vary:

* A line is appended only after every line it rests on: that of n − 1 (its `prev`) and, for a repeat, that of the game
  m it repeats.  Both are earlier lines, and lines are appended in the order of n.
* `prev`, `same_search` and `assumes` are derived as in order, from concluded games.  A repeat names the same m: game n
  looks among the eight games before it for the latest game whose problem is n's, counting a game of the window that has
  set a search up and is not concluded yet.  In order, that game is concluded when n looks, and every search or repeat
  that completes settles its problem, while one that does not complete stops the range, or fails it, before n's line
  could be written.
* The proof kinds are those of the range solved in order.  A completed search's promotion sum is its problem's best,
  P\*, and its proof kind is `cap` exactly when P\* reaches the ceiling, however the search ran.  A search started
  before its ceiling was known may have explored its whole tree to an optimal set at the ceiling: its line records `cap`
  and `reached_cap`, the weaker of the two proof kinds it has, as the verifiers' rule wants (a line reaches the ceiling
  exactly when its promotions sum to it).  One stopped by a signal or a limit with its best at the ceiling is complete
  by the ceiling, as it would have stopped there given the ceiling from the start.
* A seed from provisional moves, a hint offered while a search runs, a ceiling given late, a deferral and a suspension
  change how a search runs, never what it proves.  Every set offered is promotable (the survivors of a seed), a deferral
  only delays a search, a suspended search keeps every unit, and the ceiling is proven given the moves for n − 1
  optimal, which a game of proof kind `cap` rests on.
* What may differ: the moves and promotions where a problem has several optimal sets (as between thread counts), and
  with them the next line's `prev.moves_digest`; the nodes, the search's counters, the engine's, and the times.  The
  nodes decide the method of a game at its ceiling: `start` when its search took no step, as in order, `search` when it
  began before the ceiling was known and took some.

## Files that do not start at game 1

A results file may start above game 1 or have a gap, where a run began with a standalone game: one whose game before it
is neither in the run nor in the results file, so that it has no seed and no ceiling.  Its line has no `prev`.  Three
things rest on opt(n − 1): the ceiling `n + opt(n − 1)` (proof kind `cap`), the prime formula (proved at
`accelerate::prime_shortcut`), and the tripwire that a score above that ceiling proves the moves for the game before it
not optimal.  A *seed*, any promotable set the board can use, offered as an incumbent, rests on nothing: it cannot
change what a search proves.  A standalone game has no ceiling and no prime formula, so it is an exhausted search, a trivial game or a repeat, and both
verifiers refuse a line that claims a ceiling, or the proof kind `cap` or `prime_formula`, without a game before it
([REFERENCE.md, "What a game had from n - 1"](../REFERENCE.md#what-a-game-had-from-n---1)).  A repeat rests on the
earlier game it repeats, which is a completed line of the file.

A search that a limit or a signal stopped leaves an incomplete line: its best moves so far, no proof kind.  The range
ends there, so no game is built on it, and the line counts the invocations a resumed search has had.
