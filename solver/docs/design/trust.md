# Results and the trust model

Part of the [design](../DESIGN.md).

Every completed game records how its optimality is proven, its *proof*, and what that proof takes as given beyond the
frame model, what it *assumes*.  The referee replays every game before it counts, and two verifiers that share no code
check every results file.  [REFERENCE.md, "Why the answers can be
trusted"](../REFERENCE.md#4-why-the-answers-can-be-trusted) has the proof kinds, the assumptions and the verifiers as a
results line shows them, [frame-model.md](frame-model.md) says what each kind of proof needs of the frame model, and
[results/README.md](../../../results/README.md#what-supports-the-scores) says what supports each committed score.  This
file is the design behind them; `src/results.rs` has the full rules and the checks.

**What a game assumes** is inherited down the chain: a proof that rests on the game of n − 1, a repeat and a `known`
proof each assume what the game they rest on assumes.  Only two sources add an assumption, a game taken from
`optimal.json` and a bound of the bounds store ([REFERENCE.md, "Assumptions"](../REFERENCE.md#assumptions) words them).
So a chain from n = 1 that takes no game from `optimal.json` or the bounds store assumes nothing: it is proven within
the frame model, given that the solver is right.  The solver
refuses no game for what it assumes: the line says it, and the verifiers count it.

**Checks and referees.**  `game::replay_finished` replays every game before it counts, the solver checks its arithmetic
and the ceilings `score(n−1) ≤ score(n) ≤ n + score(n−1)`, and two verifiers read results files without changing them:
`taxman verify-results`, and `solver/tools/check_results.py`, an independent referee in Python ([REFERENCE.md, "The
verifiers"](../REFERENCE.md#the-verifiers) says what each checks).  Both read results lines of schema 5 only.  Both
derive the proof and the assumptions again from a line's fields, so both refuse a line whose fields contradict each
other (say a sum at the ceiling without `reached_cap`).  A line that cites a known source is checked against the
sources, which both read as they are (`known::Known::check_cite`; the Python referee from the lines of the files).  No
check can catch a game that is legal, finished and within the ceilings but not optimal, when the game of n − 1 is not
optimal either: that is what the proof kinds and the assumptions track, and why a chain from n = 1 matters.

**Result lines** are one JSON object per line ([REFERENCE.md, "A results line
(`--out`, schema 5)"](../REFERENCE.md#a-results-line---out-schema-5)).  A line is never rewritten, and a run locks the
results file it appends to.

**What identifies a checkpoint.**  The header that a search stores identifies the search, so that a build never resumes
a checkpoint of another search.  It holds:

* a format version, the set width and n;
* digests of the root's candidates, of the frames (every frame's moves, factors and freedoms, as built) and of the
  one-sided candidates of the extended problem ([search.md](search.md#extended-mode-one-sided-candidates));
* `max_total` and the search tactics, and from format version 6 the tactics that tune the GF(2) search (`force_window`
  to `more_forced`, [tactics.md](tactics.md)).

A search tuned as the builds before those tactics were (`force_window=80` and the other five at their defaults) writes
version 5, which implies that tuning, so its checkpoint is the one those builds wrote; a search tuned otherwise writes
version 6.  The default's window is `all`, so the default's search writes version 6, and resumes a checkpoint of
version 5 (or 4) only as `--tactic force_window=80`, as for any tactic that differs; a build from before that change
reads a checkpoint of version 6 as the search it is.  The tactics that set how the nogoods are kept (`nogood_period`,
`nogood_keep` and `nogood_limit`) are in no version: they change the trees and never the answer, and a resumed search
starts with empty tables, so the run that resumes says which it uses.  The finder (`--finder-after`, [search.md, "The
finder"](search.md#the-finder)) is in none either: it only offers sets, each after the check below, so a checkpoint
resumes with any finder or none.  The switch `conflict_branching` has a bit of its own among the search tactics, clear
in every checkpoint of the builds before it, so those resume only with `--tactic conflict_branching=off`; a build before
it refuses a checkpoint with it on, its default, as unknown tactics.

This build also reads version 4, the format of the builds before the extended problem's search, whose headers lack the
digest of the one-sided candidates.  Those builds searched no problem with one-sided candidates, so a version-4 header
reads as version 5 with the digest of none: every other field is checked as for version 5, the run goes on under the
header it was written with (the engine compares it byte for byte), and a version-4 checkpoint asked to resume a search
with one-sided candidates is refused, saying so.  So a chain stopped by a build before the extended problem's search
resumes; `tests/wide.rs` resumes a real version-4 checkpoint to the node count of the build that wrote it.  Every other
version is refused with one message, so a checkpoint of an older build cannot resume with tactics that are gone.

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
  P\*, and its proof is `cap` exactly when P\* reaches the ceiling, however the search ran.  A search started before its
  ceiling was known may have explored its whole tree to an optimal set at the ceiling: its line records `cap` and
  `reached_cap`, the weaker of the two proofs it has, as the verifiers' rule wants (a line reaches the ceiling exactly
  when its promotions sum to it).  One stopped by a signal or a limit with its best at the ceiling is complete by the
  ceiling, as it would have stopped there given the ceiling from the start.
* A seed from a provisional game, a hint offered while a search runs, a ceiling given late, a deferral and a suspension
  change how a search runs, never what it proves.  Every set offered is promotable (the survivors of a seed), a deferral
  only delays a search, a suspended search keeps every unit, and the ceiling is proven given the game of n − 1 optimal,
  which a proof by `cap` records that it rests on.
* What may differ: the moves and promotions where a problem has several optimal sets (as between thread counts), and
  with them the next line's `prev.moves_digest`; the nodes, the search's counters, the engine's, and the times.  The
  nodes decide the method of a game at its ceiling: `start` when its search took no step, as in order, `search` when it
  began before the ceiling was known and took some.

## Past limits

With `--keep-going`, a game whose search a limit stops is recorded as an unfinished line (its best game so far, no
proof), and the range goes on (`solver::SolveOptions::keep_going`; the driver is `window`, "Past limits").  The next
game may take from that game only what does not rest on its being optimal.  A *seed* is any promotable set the board can
use, offered as an incumbent, so it cannot change what a search proves.  Three things do rest on opt(n − 1): the ceiling
`n + opt(n − 1)` (a `cap` proof), the prime formula (proved at `accelerate::prime_shortcut`), and the tripwire that a
score above that ceiling proves the game before it not optimal.  The solver takes each of them from a game only if
`Prev::proven`, which is true of a completed game and of `optimal.json`'s (assumed), and false of the best game of a
stopped search.  A repeat rests on the earlier game it repeats, which is a completed game of the file or the range,
never an unfinished one.  The check that a completed game scores no less than the game before it stays, since a game of
n − 1 is a game of n, optimal or not.

The lines say what a game had: a game built on an unfinished one has `prev` with no proof, `setup.cap` null and no `cap`
or `prime_formula` proof, and both referees refuse a line that claims one of these on it ([REFERENCE.md, "What a game
had from n - 1"](../REFERENCE.md#what-a-game-had-from-n---1)).  The format of a line is the same, except that `options`
also names `keep_going`, `retry`, `retry_improved` and `pass` where they were given.  The games of such a range are
solved one at a time, because a game that went on from a game in flight would not know whether to take its ceiling.

## Known games

`--use-prev` draws on the games that other work has made (`src/known.rs`): `optimal.json`, the committed chain
`results/strict.jsonl`, and the proven games of the bounds store `results/bounds.jsonl` (`src/store.rs`; the ledger
writes it).  They are the repository's `results` (`paths`), and no option names another.  `Known::game(n)` is the game
of n from the best source that holds one: the committed chain's, which is as proven as its line and assumes what the
line does, then the store's, then the table's.  A game of the store is *proven* when its best lower bound equals its
best upper bound.  Its game is the lower bound's, played by the referee at the value (a line that cites the game of
another line is carried from it, `store::carried`), and what it rests on is the upper line's `rests_on`.  In the
options, `SolveOptions::use_prev` turns it on and `SolveOptions::known` names the files and how far it goes; `regress`
sets only the first, so that it takes just the game before its range, from the table.

* *The game before the range* (`previous_game`) is the results file's, else the known one.  It gives the ceiling, the
  seed and the prime formula as any proven game of n − 1 does, and a line built on it says where it came from in
  `prev.source` (`optimal`, `strict` or `bounds`), with that game's proof and assumptions.
* *A known game* (`Derived::Known`, step 1a of the pipeline, `known.boards`) is recorded without a search.  Its line
  has the method `known`, the proof `known`, no `setup` and no `in_the_bag` (it set no board up), and `known`: the
  citation.  Its promotions are the board's promotion candidates that the game plays, as a prime game's are.
  `conclude` checks it like any game (the replay, the score the source gave, the chain bounds against n − 1, the
  table), and it becomes the game of n − 1 of the next game, whose proofs that rest on n − 1 inherit its assumptions.
  It sets up no search problem, so no later game repeats it, unless its proof is the store's bound reached by a search
  (below).
* *An open game of the store* lends its lower bound's game as a seed, and its upper bound as a second ceiling
  (`StoreCap`, `search_cap`: the engine is given the lower of it and the ceiling of the game before, at the start and
  when the game before is concluded).  A search whose best promotion sum reaches the store's bound, when it is not the
  chain's ceiling that it reached, is proven by the bound: proof `known`, method `search` (or `start`, if the starting
  game reached the bound before a step), `known` citing the upper line alone, `reached_cap` false and `setup.cap` as
  for any search.  Such a game settles its search problem, as an exhausted search or one at the ceiling does: its
  promotions are a promotable set of that sum, and no promotable set weighs more than the bound allows (a game of the
  frame model is a game of the board, so it scores at most opt(n), which the bound bounds).  So a later game that poses
  the same problem repeats it, and assumes what the bound does.  A search that finds a game above the store's bound
  fails the run at that game: the store is wrong, or statement U fails at a game that its bound rests on.
* *`force_search`* leaves out the known games and the store's help, since a forced search is the chain's own.  A game
  with a checkpoint is resumed, never replaced.

The referees (`Known::check_cite`, and its Python counterpart in `solver/tools/check_results.py`) check a citation: the
table has a game of n with the line's score; so has the committed chain; the store has a lower line and an upper line
of game n worth the score, of the rule and from the run that the citation names (a store has one line of a game, side
and value).  They take the sources as they are: the referees of results files check the committed chain, and `ledger
verify-bounds` and `solver/tools/check_bounds.py` check the store.  What the citation lets them conclude is that the
game, a finished game of that score, is optimal where the source's bounds hold; they do not compare which game the line
plays with the source's.  The assumptions are those of the cited lines (`assumed_optimal`, the committed line's, and
`assumed_u` for each game of the upper line's `rests_on`), and the referees refuse a line that records others.  Results
lines stay in schema 5: `known` is a key that only such a line has, a method and a proof, and `strict` and `bounds` are
sources of `prev`.  Every older file reads as it did, and an older build refuses a file with a `known` line, whose
proof it does not know.

## The ledger

The ledger proves many games at once by bounding each from both sides: for every game of a stretch it keeps a lower
bound L(n), the score of a finished game that the referee replays, and an upper bound U(n), each with its derivation,
and it proves a game when they meet.  It is `examples/ledger/`, whose module comment has the commands and the files.  It
reads nothing of the truth (`optimal.json`, the results files) unless it is given proven games.

*Its sources of bounds:*

* the *map* of each game: a search stopped before its first node, which gives the start incumbent and the root's GF(2)
  bound;
* limited one-thread searches, each resumed from its checkpoint, seeded with the games of n − 1, n and n + 1, and with
  U(n) as its ceiling.  A stopped search's upper bound is its *dual bound* (`search::dual_bound`): the largest of its
  incumbent and the bounds of the open units of its final checkpoint, each the GF(2) bound of the node as the search
  computes it when it starts the unit (`search::NodeBound`);
* given games;
* the relations between games, applied to a fixpoint after every change: monotonicity, the ceiling and the game below
  it, the prime formula both ways, and repeats (docs/theory/ledger.md, Lemmas L1 to L4
  and D, with what each rests on).  The referee replays every game that a relation makes before it counts, if it could
  raise a game's L (a game that cannot is not shown to the referee, or not made), and an L above a U stops the run as
  an error;
* the *finder*, a large neighbourhood search (`search/finder.rs`; [finders for good games that are not
  sound](../../notebook/search/finders-for-good-games-that-are-not-sound-2026-10/README.md)).  It is not sound and
  proves nothing, but a lower bound needs only a legal game.  It runs for `--lns-seconds` seconds of a worker, after a
  game's map and before a search whose best seed has changed since it last ran on the game (a climb's always has).  Its
  game, which the referee replays like every lower bound, is offered as the game's L, and seeds the search if it is
  better.

*Its order.*  The maps of all the games come first, on the workers, and the relations carry their bounds once, when the
last map is in; the main thread, which applies every result while the workers whose searches ended wait for it, does
little for each.  Then the searches go where a proof looks cheapest, with *climbs* ahead of them:

* a game whose game below its repeat class is proven, however it was proven, is searched next, for a limit of its own
  (`--climb-limit`), seeded with that game and capped by the ceiling it gives; when a climb proves its game, the game
  above its class is climbed to (a climb up);
* after those, a game whose game above its repeat class is proven is climbed to downward, seeded with that game but
  under the ledger's own, loose ceiling (`--climb` picks the ways);
* a climb that stops parks its game in the ordinary order, and the games just outside the class of a game being
  climbed to wait for it;
* an open game that is not climbed to is *stepped down* before its proof (`--floor-step`): a search under a floor below
  its upper bound (`search::search_above`, Lemma F) which, if it completes having found nothing, makes the floor the
  upper bound (the rule `floor`).

The cost of a search under a floor is about log-linear in the floor's distance below the root's bound, so each game has
a line, fitted from its completed steps, which forecasts its proof.  Each step costs tenfold more than the depth already
proven.  A game goes to its proof (the ordinary search, seeded and capped) when its gap is within two steps.  After the
climbs and the games that a proven neighbour has changed, the games go by the forecast seconds of their proofs, cheapest
first (`examples/ledger/cost.rs`; with no floor steps, by least `level + gap / gap_unit`, which finds the games proven
with no help from below).

A proven game of the ledger is proven within the frame model, as a game of the chain past 2175 is, given U at the games
its upper bound came through.  The ledger is not a results file, and nothing of it goes into `results/strict.jsonl`.

**The bounds store** keeps the bounds that a run finds for good: `results/bounds.jsonl`, one JSON line a bound, in
schema 1, appended and never changed.  A line is added only when it improves the file's best bound of its game and side,
and none for a game that the chain has proven.  `src/store.rs` has the format and the relations between games, which
`taxman solve --use-prev` reads too ("Known games", above); `examples/ledger/store.rs` has the ledger's checks of a line
against the lines before it; and [results/README.md, "bounds.jsonl"](../../../results/README.md#boundsjsonl) has the
format, the commands and the referees.

* A lower bound is a game that the referee replays at the line's value: its promotions where it was made from them,
  which `solution_for_promotions` turns into moves; cited (`from` and `from_value`) where a relation made it from the
  game of another line; else its moves.
* An upper bound has its rule, the bound and game it was derived from (`from`, `from_value`, and for a repeat the two
  bags), and `rests_on`, the games at which it assumes statement U: its own game for a bound of the frame model, what
  the bound it comes from rests on for a relation, and its own game added for a repeat (Lemma L4's inequality between
  upper bounds).
* `ledger export` appends a run's improvements, and `ledger run --bounds` starts a run from the file's best bounds (a
  game with both needs no map).  `ledger verify-bounds` is the Rust referee: it replays every game, computes again every
  root bound and every repeat's bags and search problem, and checks every relation's arithmetic against the line it
  cites.  `solver/tools/check_bounds.py` is the Python one: it replays the games, the cited ones rebuilt by its own
  constructions and promotion sets from the moves that `ledger games` writes, and checks the arithmetic.

The store holds only bounds that hold: the finder's games are in it because the referee has replayed them.
