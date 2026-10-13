# Design: a Rust engine for optimal Taxman

## Goals and constraints

* Find optimal games beyond n=1000 on one machine (Apple M1 Max: 10 cores,
  32 GB RAM, fast SSD).  No cloud.
* Search an enormous tree efficiently: fast per node on a single core, and
  scaling cleanly to the cores available, without depending on many of them.
* Never rely on everything fitting in RAM.  Memory use is bounded by
  configuration, independent of tree size; work that does not fit goes to
  disk.
* Every long run checkpoints frequently and resumes after a restart, losing
  at most one checkpoint interval (see the repo's CLAUDE.md); stopped by
  SIGINT or SIGTERM, it loses nothing.
* Every unproven tactic can be switched on and off, so its effect on both
  answers and speed can be measured.
* Fast iteration: correctness and performance are worked out on small and
  medium games that finish in seconds.  Big runs come last.

## The algorithm: the Java solver's, reimplemented — not ported

The optimization strategy is the one in the Java code and the
[wiki](https://github.com/bvchess/taxman/wiki): decompose the game into
frames of Taxman Mini, and search for the optimal set of *promotions*
(numbers moved from being tax in one frame to being a move in the frame
below).  The Java is research code — three generations of the same frame
idea layered on each other, allocation in every set operation — so the Rust
is a fresh implementation of the *algorithm*, designed for clarity and
speed, not a translation of the classes.  The Java's live path, which defines
what the algorithm computes, is:

| step | Java | role |
|---|---|---|
| 1 | `frame.FrameSolver.solve` | try to derive game n from game n−1, else search |
| 2 | `FrameSolver.solutionForPrime`, `reusePrev` | the derivation ("acceleration") |
| 3 | `bombus.Apiary`, `bombus.Hive` | hives: sources + maximal factors, mandatory and impossible moves, promotion candidates, per-hive solutions |
| 4 | `frame.FrameBuilder`, `frame.WorkingFrame` | group hives into levels (frames) |
| 5 | `search.SearchManager.create` | the search's starting point: frame columns, bounds, greedy incumbent, revised freedoms |
| 6 | `search.Frame`, `search.FrameSet`, `search.MemoizedFrame` | frame state; promote into / out of; winnability (peel); reduce |
| 7 | `search.UpperBoundEstimator` | preclusions; `blameGame` upper bound; `reviseFreedoms` |
| 8 | `search.LowerBoundEstimator.greedy` | initial incumbent |
| 9 | `search.BranchExplorer` | the branch-and-bound recursion |
| 10 | `Apiary(board, promotions).getSolution()` | turn promotions into an ordered game |
| 11 | `game.Solution.verify` | replay the game; compare with optimal.json |

Not ported: `oldsearch/` (the 2021 disk-queue search), `frame.GreedySolver`,
`util.Memo` and `WorkingFrame.fancyFit/fits` (only those two used them), the
picklers and compressor.

What must match the Java is *results*, not structure.  The setup (hives →
frames → candidates → greedy incumbent → revised freedoms) defines the
search space, so at first it must reproduce the Java's values exactly — Java
golden data in `testdata/golden/` is the check — and every final score must
equal optimal.json.  The search itself is designed for speed from scratch:
depth-first, in-place state with no allocation per node, efficient peeling,
caches only where profiling shows they pay.  Node counts need not match the
Java's.  Once parity is established, improvements (bounds, incumbents,
caching, new tactics) deliberately diverge, each measured on the benchmark
set and switchable where unproven.

### Known defects fixed in the port (each switchable back for parity checks)

* **Upper bound runner-up** (`UpperBoundEstimator.getRunnerUp`): when
  `largestThatFitInFrames` skips candidates because frame 1 or the last frame
  is at its cap, the Java replaces the failing number with the next candidate
  below the smallest bid member.  A skipped candidate from the failing
  number's frame can be larger, so the Java bound can be too low (exhaustive
  checks find it below the best completion, e.g. at the root of n=114: 358
  against 360).  Sound version: the heaviest set within the caps that leaves
  out some bid member at or above the failure, which by matroid exchange is
  the bid with one member swapped for the best outside candidate the caps
  allow.  (Simply taking the largest candidate not in the bid is also sound
  but swaps skipped candidates of a full frame in for members of other frames;
  it made n=700 take over 10 minutes instead of 18 s.)  (Re-solving n=2..760
  in Java with the simple fix changed no answers; n=740 timed out.)
* `Memo.test` computes its value twice — moot, `Memo` is not ported.
* `TxSet.MAX_VALUE = 1023` — gone; set width is chosen from n.

## Tactics

Each row is a tactic in `solver::Tactics`, set from the CLI with
`--tactic name=value` (`on` or `off`; `match_refine=0`…`7`;
`branching=largest|conflict-smallest|conflict-largest|conflict-strong`), except
the model assumptions, which define the search space and cannot be switched off
inside this model.  The ones that act inside the search are the fields of
`search::SearchTactics`; `examples/ladder.rs` takes the same `--tactic` names
and values for them.

| tactic | default | status | effect |
|---|---|---|---|
| `prime_shortcut` | on | proven (model-free, given an optimal n−1 game: see `sacrifice`) | prime n: replace the n−1 solution's first move with n |
| `reuse_prev` | on | unproven (conjecture-grade "upper-delta" certificate) | `FrameSolver.reusePrev`; accepts a derived solution without search |
| `revise_freedoms` | on | unproven ("max-sum promotion set also has max count") | `UpperBoundEstimator.reviseFreedoms`; can shrink frame 0's freedoms |
| `preclusions` | off | sound (promotion failure is monotone: more moves or fewer factors never make a frame winnable) | depth-2 preclusion table; prunes candidates (off since 2026-10: with nogoods it no longer pays for its setup) |
| `match_bound` | on | sound (matchability relaxes winnability) | node bound = the matching relaxation instead of blameGame (which never pruned where the matching bound did not), and the children's candidate lists without the Java's promotability checks (the conflicts find those candidates); a heaviest matching that can be promoted is the node's best completion (offered as incumbent, node done) |
| `branching` | `conflict-strong` | sound (any order explores every promotion set) | which candidate a node promotes next: `largest` (the Java's order), or a member of a conflict of the heaviest matching (these need `match_bound`): the smallest (`conflict-smallest`), the largest (`conflict-largest`), or the one whose removal lowers the matching bound most (`conflict-strong`) |
| `match_refine` | 1 | sound (see `search::Refiner`) | refine the matching bound by branching on conflicts this many levels deep, where it does not prune (0 = off, at most 7; needs `match_bound`) |
| `fix_forced` | on | sound (every completion that beats the incumbent promotes them) | with `conflict-strong` branching, a node's child promotes all its *forced* candidates (those whose removal alone brings the matching bound below what the node needs): the members of the heaviest matching that the potentials show forced, else the forced members of the conflict found by strong branching; the node's loop ends after that child; a node with a known conflict all forced is done |
| `nogoods` | on | sound (a nogood holds at every node; see `model::Nogoods`) | learn each conflict as a *nogood* of the root and use the table at every node: the smallest conflict it knows of the heaviest matching is the one to branch on and refine with (computed and learned only when it knows none), and nogoods rule candidates out of children (needs `match_bound`) |
| `forget_nogoods` | on | sound (the table is a cache) | with `nogoods`, each searcher forgets every nogood it knows after each 50,000 of its steps (`search::NOGOOD_PERIOD`): old ones cost every lookup more than they help |
| `legacy_bound` | off | unsound (the runner-up defect) | reproduce the Java bound exactly |
| `prev_cap` | on | proven if opt(n−1) is optimal | stop as soon as the incumbent reaches n + opt(n−1) − inTheBag (`FrameSolver.getMaxPromotionSum`) |
| `seed_incumbent` | on | sound (any feasible promotion set is a valid incumbent) | start from the better of greedy and the n−1 solution's promotions that are still feasible |

Model assumptions (unproven, inherent to the frame model; validated by
optimal.json agreement and, below n=63, by brute-force certification):
greedy removal of impossible sources inside hives (`Hive.removeImpossible`);
mandatory moves (`Hive.unzip`); `freeFactorCount = factors − sources`
(`Hive.finishSetup`); closed hives take no part in the search; a promoted
number pays tax only from the frame directly below.

Results record the tactics they were produced with (`tactics`: the changes
from the defaults, such as `branching=largest,match_refine=0`;
`tactic_switches`: every tactic's value, the booleans as booleans,
`match_refine` as a number and `branching` as its name), whether any
unproven tactic was on, and which ones the game's proof actually rests on
(`taint`, see `results.rs`).  A checkpoint records the nine search tactics and
is refused when they differ (the error names each tactic that differs and the
`--tactic` flags that resume it); one made before a sound search tactic
existed (`fix_forced`, `nogoods`, `forget_nogoods`) reads back with it at its
default.

## Architecture

One Cargo crate in `rust/` (library + one binary, `taxman`), no third-party
dependencies.  Modules:

| module | contents |
|---|---|
| `bits` | `Bits<const W: usize>`: fixed-width bitset over `[u64; W]`, `Copy`, no heap; capacity `64·W − 1`.  Set ops, popcount, sum, min/max, ascending and descending iteration, `nth_largest`, `largest(k)`, `next_below(x)`, `is_subset`, hashing. |
| `numtheory` | smallest-prime-factor sieve; for 1..=n: maximal factors (n/p for each prime p dividing n; {1} for primes; {} for 1) and maximal composites (k·p ≤ n), as `Vec<Vec<u32>>` and as bitsets; all divisors; prime test; optionally *holes*, numbers taken off the board (`Tables::punctured`, for the sacrifice study; the hives leave them out) |
| `json` | minimal JSON reader/writer (enough for optimal.json, golden data and results files; checkpoints are binary) |
| `game` | `replay(n, moves) -> Result<score>`: independent legality check (each move on the board, pays ≥ 1 tax, takes all divisors still present); `replay_finished`: also over (no legal move left) and opening with the largest prime, as every optimal game is; optimal.json loader |
| `hives` | port of `Apiary`/`Hive`/`Namer`-free equivalent: hive formation with `treat_as_sources`, unzip, removeImpossible, finishSetup propagation, merge, promotion candidates, per-hive solution, apiary solution order |
| `frames` | port of `FrameBuilder` (levels of hives) producing columns (factors, moves) per frame bottom-up, candidates, `estimateMaxPromotions` |
| `accelerate` | deriving game n from game n−1 (prime shortcut, `reusePrev`), and turning a promotion set into an ordered game |
| `model` | the search state: `Frame`, `FrameSet` (fixed array of frames), `FactorProvider` (factor/composite lists restricted to frame numbers), `FrameMapper` (home frame of each candidate), `promote_into`, `promote_out_of`, `promote`, `can_promote`, `is_winnable` (peel), `reduce`, `narrow_candidates`, `max_promotions`; `promote_all` (a set checked frame by frame); `Certificate` (a winning order that answers most single-edit winnability questions without a peel); conflicts: the *core* of an unwinnable frame (`Scratch::stuck`, the moves a peel cannot drop) and `conflict_using`/`conflict_cached` (a small, by default minimal, subset of a promotion set that cannot be promoted), with `ConflictCache`; nogoods: `nogood` (a conflict of a state made a conflict of the root) and the table `Nogoods` |
| `bounds` | preclusion table; `upper_bound` (blameGame + largestThatFitInFrames, sound or legacy runner-up) and its exact threshold test `reaches`; candidate lists; greedy lower bound |
| `matchbound` | the matching bound (see "Improvements after parity"): `match_bound`, a min-cost-flow reference; `MatchGraph` (the root's graph) and `MatchBound` (an optimal matching and its potentials per level of the DFS path, repaired incrementally by Hungarian searches on a radix heap; levels with a candidate excluded, which can be kept for the parent) |
| `engine` | problem-independent parallel depth-first branch-and-bound driver: work units, shared deque with disk spill, incumbent, yielding/splitting, checkpoint and resume, stopping on SIGINT/SIGTERM, a lock on a checkpoint directory, progress reporting, statistics, instrumentation (`Counters`) |
| `search` | the Taxman searcher (port of `BranchExplorer`) implementing the engine's `Searcher` trait; `prepare` (preclusions, greedy, `revise_freedoms`) and `search` (seed, cap, checkpoint and resume); `Refiner` (conflicts of the heaviest matching: exactness, refinement, branching); the starting incumbent's `dive` and local search (`Neighbourhood`); `search_with` and `Diagnostics` for measurements |
| `examples/ladder.rs` | the ladder harness: standalone searches of hard games from the frame builder with a time limit, at one or several thread counts (see Performance) |
| `examples/perf.rs` | the performance probe: the hot paths replayed from fixtures, in cycles and instructions, with fingerprints (see Performance) |
| `solver` | `Tactics` (every tactic above, by name and value), `SolveOptions`, `solve_game` (the per-game pipeline below) and `solve_range` (a range of games, resumable through a results file) |
| `results` | what a result line records about its provenance (proof kind, taint, the n−1 game it was built from, the build), the checks a results file passes when it is opened (`read_results`), `ResultsFile`, and `verify_files` behind `taxman verify-results`; `tools/check_results.py` is an independent referee of the same files |
| `brute` | exact Taxman on boards up to 127 with holes, by memoized search over pot states (no frame model), under the rules or the maximal-factor rule |
| `sacrifice` | study, not used by the solver: the proven bound opt(n) ≤ max(opt(n−1), n + max over maximal factors f of n of opt_M({1..n−1} ∖ {f})), the punctured games it needs (frame model on a board with a hole), and what it was measured to be worth (`examples/sacrifice.rs`) |
| `main.rs`, `cli.rs`, `commands.rs` | CLI: `solve`, `regress`, `bench`, `verify-optimal`, `verify-results`, `tables` (`taxman help` lists the options) |

Width dispatch: everything that holds sets of game numbers is generic over
`const W: usize`; the CLI picks `W ∈ {8, 16, 32, 64}` from n (n ≤ 511, 1023,
2047, 4095).

### Search state and order

A node is `(state: FrameSet, promoted: Bits, cands: Bits, max_add: u32)`.
A node's state is the root with `promoted` promoted, in any order (the result
does not depend on it), so a *work unit* is just `(promoted, cands)` — two
bitsets — from which the state is rebuilt in one pass.  (With the Java's
branching, promotions happen in decreasing order along every path; with
conflict branching they need not.)

The expansion step is `BranchExplorer.explore` made lazy (children explored
depth-first as they are generated, rather than all generated first and then
forked):

```
loop while cands non-empty:
    if upper_bound(state, cands, max_add) + sum(promoted) <= best: break
    n = the candidate to branch on (tactic branching; the Java's: cands.max())
    cands.remove(n)
    new_state = state.promote(n) else continue
    new_promoted = promoted ∪ {n}
    max_next = max_additional(new_state, new_promoted, cands)
    child_cands = new_candidate_list(n, max_next, new_state, cands, promoted)
    if sum(promoted) + n + upper_bound(new_state, child_cands, max_next) > best:
        if sum(new_promoted) > best: offer new_promoted as incumbent
        push child (new_state, new_promoted, child_cands,
                    max_additional(new_state, new_promoted, child_cands))
```

with `max_additional(s, p, c) = min(max_total − |p|, s.max_promotions(c))`,
and `new_candidate_list` = narrow → remove preclusions → `can_promote`
filter, exactly as in `BranchExplorer.makeNewCandidateList` (the filter only
without `match_bound`), then the conflicts known along the path propagated
(see "Improvements after parity").
The Java breaks only when the bound is below `best`; a bound equal to `best`
cannot lead to a better incumbent, so this loop breaks then too.  The Java
generates all children of a node before exploring any; depth-first is
equivalent in answer, finds good incumbents sooner, and needs memory only
proportional to depth.

Rebuilding a partially expanded node from a unit recomputes `max_add` from
the remaining candidates, which can only tighten it (still valid).  The
conflicts a searcher has learned along its path are a cache that units do
not record, so a rebuilt node may prune a little differently from the node it
replaces (same answer).

### Engine

```rust
pub trait Unit: Send + Sized + 'static {
    fn encode(&self, out: &mut Vec<u8>);
    fn decode(bytes: &[u8]) -> Self;
}

pub trait Searcher<U: Unit>: Send {
    /// Explore `unit` depth-first.  Call `ctx.poll()` every few thousand
    /// nodes or less; when it returns `Poll::Yield`, push ALL remaining
    /// unexplored work as units (shallowest first, deepest last) and return.
    fn run(&mut self, unit: U, ctx: &mut Ctx<U>);
}
```

`Ctx` gives: `poll()`, `push(unit)`, `best() -> u64` (relaxed atomic read of
the incumbent value), `offer(value, payload) -> bool`, and a per-worker
`Stats` (counters merged at the end and in progress reports).

* **Workers**: `threads` OS threads (std only), each with its own `Searcher`
  instance (so per-thread caches need no locks).
* **Work distribution**: one shared deque of units.  A worker that yields
  pushes its stack's remaining work (shallowest first) and then pops from the
  back, resuming its own deepest work; idle workers take from the front, which
  holds the shallowest — largest — subtrees.  A worker yields when (a) idle
  workers are left that the queued units cannot serve (a *split*), (b) a
  checkpoint is due, or (c) a stop is requested (time limit or node limit).
  With one thread the deque is a pure stack: units are explored in
  depth-first order, unchanged by checkpoints and spilling.  The Taxman
  searcher leaves out of its yield the nodes of its stack that have no
  candidates left (a node whose loop `fix_forced` ended): as units they would
  only be runs of one step that end the loop.  At 2 threads on 936, 1100 and
  1184 (twice each, `ladder --threads 2,2`) the runs of at most one poll fell
  from 60 and 91 of 256 and 305 to 13 and 10 of 227 and 200, the nodes and
  times unchanged.
* **Splits, sized for many cores** (2026-10, for a 192-core machine): the
  engine keeps one open split request per idle worker that the queued and
  held units will not serve, and any running worker can claim one (an atomic
  decrement of a count in the word `poll` reads), so as many workers split at
  once as there are idle workers to feed.  From one unit, all workers are busy
  after a few rounds in which every running worker splits, not after one
  split per idle worker (the first design answered one request at a time).  A
  worker answers only after 64 polls of its current unit, so a fresh unit
  first gets deep enough to have something to give, and no worker restarts
  its unit at every step while the demand lasts.  The deque stays a single
  locked one: units exist only after yields, each costs its searcher a state
  rebuild (for Taxman a matching solved from scratch, 30–170 µs already at
  n = 480–760, against about a microsecond under the lock), and with every
  worker busy nobody takes the lock;
  per-worker deques with stealing would remove contention that is not there,
  at the price of distributed ordering, spill, checkpoint and termination
  arguments.  The lock's wait time is measured (below) so that a many-core run
  shows whether that ever changes.
* **Memory bound**: the in-memory deque holds at most `queue_mem_cap` units;
  beyond that the front (oldest) units are spilled to segment files in
  `spill_dir` and reloaded when memory runs dry.  Searchers keep only their
  DFS stack plus bounded caches (capacity set by configuration, cleared or
  evicted when full — never unbounded).
* **Incumbent**: `AtomicU64` value plus a mutex-protected payload (the
  promotion set, encoded); offers only ever raise it.
* **Checkpoint**: every `checkpoint_interval`, all workers yield and wait;
  the engine writes caller header, incumbent, statistics, and every unit
  (memory and spilled) to `checkpoint.tmp`, fsyncs, renames over
  `checkpoint`, and releases the workers.  Resume loads the file and
  continues; nothing is lost or duplicated beyond partially explored nodes.
  The caller's header stores what identifies the search and what the setup
  computed that is expensive or must not change on resume: a format
  version, the width, n, digests of the root candidates and of the frames
  (every frame's moves, factors and freedoms, as built), frame 0's revised
  freedoms, max_total and the tactics.  Resuming checks all of them and says
  which differs, and checks that the stored incumbent is a set of
  candidates that can be promoted from the root and is worth the stored
  value, so a checkpoint of another game, or of an older build whose frame
  builder differs, is refused instead of pruning with a foreign incumbent or
  failing to rebuild its units.  Headers of the format before version 3
  (magic `TXS2`, made before 2026-10-12) recorded neither n nor the frames:
  a checkpoint of 140 passed for one of 141 (same candidates, same frame 0,
  other frames).  They are refused with that reason rather than accepted
  unchecked; finish such a game with the build that made it, or delete the
  file.  A resumed search whose incumbent already reaches the cap finishes
  at once.  A pause costs every worker the wait for the
  last one to park — bounded by the longest stretch between two polls, one
  Taxman step: about 10 µs, over a millisecond about once in 10⁵ steps, the
  longest measured 10–15 ms at n = 1215–1225 — plus the write and fsync.
  After the rename the directory is synced too, which Linux needs for the
  rename to survive a crash.  Any interval works, zero included: a periodic
  pause begins only once the workers released from the last one all run
  again, and no sooner than the last pause lasted; and a worker answers it
  (as a split request) only after 64 polls of its current unit, so every
  pause is followed by work.  (Before, an interval of 0 paused again at once
  without letting go of the lock, and the workers never ran again.)  Times
  too large to add to the clock count as never.  A run that completes
  deletes its checkpoint, unless the caller asks to keep it
  (`keep_completed_checkpoint`) and deletes it itself once the result is
  recorded.
* **Stopping and sharing**: `taxman solve` turns SIGINT and SIGTERM into a
  stop like a time limit's (the handler sets an atomic flag that the
  coordinator polls every 50 ms; a final checkpoint, an incomplete result,
  exit status 128 + the signal); a second signal kills the process at once.
  It takes an exclusive `flock` on `taxman.lock` in its checkpoint
  directory, so a second process there fails at once instead of writing the
  same `.tmp` files.  Spill segments carry their process id: a run deletes
  those of processes that no longer exist from its spill directory.
  Progress lines and the results' lines on stdout ignore write errors, so a
  closed pipe (a dead `tee`, `| head`) cannot kill a run.
* **Instrumentation** (`engine::Counters`, per invocation): each worker's time
  split into busy (in the searcher), idle (waiting for work), parked (in
  pauses) and lock wait (for the deque's lock); the effective parallelism
  busy / (wall × threads); splits and the units they gave, steals, runs and
  tiny runs (units that polled at most once); when all workers first ran at once;
  the number of pauses, the longest pause and the longest wait for the
  workers to park.
* **Progress**: every `progress_interval` (default 60 s) one line on stderr:
  elapsed, nodes, nodes/s, incumbent, deque size, spilled units, and the
  counters in short (`eff 97.1% idle 2.3% lock 0.0% splits 1.2k steals 950
  full 3.1ms pauses 4 (0.1%) max 21.0ms park 2.3ms`: the pauses' share of
  the workers' time, the longest pause, the longest wait to park).  It is
  printed outside the lock, so a blocked stderr cannot stall the workers.

### Per-game pipeline (`solver`)

1. Board tables for n.
2. If `prime_shortcut` and n is prime and the n−1 solution is known: replace
   its first move with n (method `prime`).
3. Hives → frames.  With no promotion candidates the game with no promotions
   is the only one (method `trivial`).
4. Else if `reuse_prev`: port of `FrameSolver.reusePrev` (method `reuse`).
5. Else search (method `search`): columns → `FrameSet`; preclusions;
   greedy; `revise_freedoms` (which can run a nested two-frame search through
   the same engine, within the run's time and node limits: if they stop it,
   the game stops there too, incomplete and with no checkpoint, and the next
   run starts it again); `max_total`; the starting incumbent (the best of
   greedy, the seed, and the root matching's rounding improved by local
   search); run the engine.
6. Solution = `Apiary(board, promotions)` moves in hive order, then the checks
   of `solver::conclude`: legal replay; a completed game over and opening
   with the largest prime; a game from the frame model scoring inTheBag plus
   its promotions; no game above n + score(n−1) and no promotions above the
   cap (either proves the n−1 game is not optimal, since opt(n) ≤ n +
   opt(n−1)); a completed game not below score(n−1); the score against
   optimal.json when n is there.  Each failure is a hard error; a game that
   fails after it was found goes to `<results>.rejected.jsonl`.
7. Append one JSON line to the results file (`results.rs` describes the
   format): the game, its proof kind and taint, the n−1 game it was built
   from, nodes, search counters, seconds per phase (this invocation, and all
   of a resumed search's), invocations, tactics, limits, the build's commit.

Before anything uses the n−1 solution it is checked: legal, scoring what it
is given as, over, and opening with the largest prime below n.  A run with no
unproven tactic on refuses one whose proof rests on something unproven (its
taint, such as `optimal.json` for that file's games); `--allow-tainted-prev`
overrides, and the results that rest on it inherit the taint.

Steps 2 and 4 are skipped by `force_search`, and when the game has a
checkpoint (a search that was interrupted is resumed, never replaced by a
derivation).  A checkpoint is `<dir>/n<NNNN>.ckpt`; resuming one made for
another game, width or search tactics is refused rather than restarted.  A
search stopped by a time or node limit gives an incomplete result (`completed`
false, `best_so_far` the best game so far) and keeps its checkpoint; a range
stops there.

Range runs feed each game's solution to the next and resume from the
results file, which is checked when it is opened (`results.rs`: every game
replayed, the chain from n−1, each line's arithmetic and provenance).  Games
completed in it are skipped; each result is appended and synced as soon as it
is done, and only then is the game's checkpoint deleted (a checkpoint left
for a game the file has completed is deleted when a range starts).  The n−1
solution comes from, in order: this run, the results file, optimal.json.

`taxman bench` also appends one JSON line per run to `bench/history.jsonl`
(commit, whether the sources were modified, threads, every tactic's value, and
per game the nodes, seconds per phase and search counters), so that
performance can be followed across changes.

## Improvements after parity

* **Matching bound** (sound).  Relax winnability (peel) to plain
  matchability across the whole frame chain: maximize the promoted sum
  subject to every original move and every promoted number getting a
  distinct maximal factor in its frame, and no promoted number being used
  as a factor.  Edges v → v/p change Ω(v) by one, so this is a max-weight
  *bipartite* matching (min-cost flow), exactly solvable.  Measured at the
  root of all 94 golden games that need search (scratch study): mean gap to
  the optimum 31 vs 76 for the Java bound; on the bench games 107–292 vs
  363–769.  The cardinality cap never binds.  To be used incrementally
  (re-optimized along the DFS path) and combined with the Java bound
  (take the minimum).
  *Implemented* in module `matchbound` (used by `search`, tactic
  `match_bound`): an optimal matching and its dual potentials per level of the DFS path,
  repaired by Hungarian searches after each change; 0.7–2.4 µs per
  evaluation in a DFS at n=480–760, against 30–170 µs from scratch.  A test
  DFS that starts with the optimum as incumbent and evaluates the matching
  bound only where blameGame fails to prune visits 85, 34, 1.5, 2.7, 1.7,
  32 and 35 times fewer nodes at n=480, 500, 600, 640, 660, 700 and 720
  (taking 57, 26, 1.5, 2.5, 1.6, 24 and 28 times less time), and at n=760
  finishes in 90k nodes where blameGame alone passes 3M.  The cap never
  bound there either.
* Seeding the incumbent from n−1 and stopping at n + opt(n−1) (tactics
  `seed_incumbent`, `prev_cap`).

### Conflicts of the matching bound (2026-10)

Measured with `examples/ladder.rs` (see Performance) on 1 thread for 760,
880 and 936 and on 10 threads for 1000 and the hard games near it.  With the
matching bound alone, n=1000 was not proven in 4 minutes on 10 threads.

What the diagnostics showed first (n=760–1000, `--diag`): the matching bound
does nearly all the pruning (blameGame almost never prunes where it does not),
and at the nodes it fails to prune it is 25–45 above what was needed
(median); the heaviest matching there essentially never promotes a feasible
set; the tree peaks at 25–40 promotions deep.  Making the bound tighter was
the lever, not cheaper nodes.

* **Conflicts** (`model::conflict_using`).  When the heaviest matching's
  candidates `Q*` cannot all be promoted, some frame is unwinnable with them;
  the moves a peel of it cannot drop (its *core*) are stuck however the order
  goes, and the members of `Q*` the core needs (promoted into the frame and
  core moves, or promoted out of it and factors of core moves) cannot be
  promoted together.  Minimized by peeling only the core, these conflicts are
  tiny: two members at the median and p75 at n=760, p50 2 / p90 5 at 996 (a
  quarter are single members: a candidate that cannot be promoted from that
  node's state, though it can from the root; see "Single-member conflicts
  belong to the node").  A subset of a promotable set is promotable, so
  every completion leaves out some member of a conflict — the basis of all
  that follows, and of `search::Refiner`'s soundness proof.
* **Refinement** (`match_refine`).  Bound the branches of the conflict too
  (`k₁` out; `k₁` in and `k₂` out; …), a few levels deep.  Alone, on the
  Java's branching, it cut nodes 2–70x but cost more per node than it saved
  (n=880: 4.8M → 68k nodes at depth 7, 31 s → 21 s; at depth 1–3 slower than
  without).  Caching known conflicts along the path and branching disjointly
  made it pay at 760; with conflict branching it is worth one level (default
  1; 2–15% faster than 0 on the benchmarks, deeper is slower).
* **Conflict branching** (`branching`).  Branch on a member of the conflict
  instead of the largest candidate, so that neither the child nor the node
  going on can reproduce the matching: the decisive change.  `conflict-strong`
  (default) branches on the member whose removal lowers the matching bound
  most (one matching solve per member; the level for the chosen one is kept
  for the node, `MatchBound::keep_excluded`).  Promoting a member is not worth
  looking at: the heaviest matching promotes them all already; and a lookahead
  that also scores the child with the other member of a pair out did worse.
  `conflict-smallest` gives the smallest proof trees at 880/936 but finds
  good incumbents late; `conflict-largest` follows the dive and finds them
  early.  Branching on the members of every failing frame's conflict instead
  of the smallest conflict was worse (936: 754k → 1.0M nodes).
* **Exact matchings**.  A heaviest matching that can be promoted is the
  node's best completion: offered as incumbent, node done.
* **Propagation**.  A known conflict of which a child promotes all but one
  member rules that member out of the child's candidates.  Without it the
  child's matching bound equals the node's (its matching promotes the child's
  number already).  −17% nodes.
* **Starting incumbent**.  `dive` rounds the root's matching (drop the
  smallest member of a conflict until feasible) — weaker than greedy on most
  games; a one-step lookahead in the dive was no better.  A local search
  (`Neighbourhood::improve`: fill greedily, then drop one or two members and
  refill) reaches the optimum at the start for 945 and 988 and gets close for
  950/984/990 (20–40 ms).  Running it on every incumbent found during the
  search, and dives at search nodes, gave nothing measurable.
* **Cheaper nodes**: a per-frame cache of conflict answers (consecutive
  matchings change few frames; −5%); a radix heap in the Hungarian search
  (−16–19%); stopping the scan of frames at a single-member conflict.
  Choosing the frame by the smallest *core* and minimizing only it was much
  cheaper per node but its conflicts branch far worse (n=1000 unfinished in
  2 min).  Checking every candidate a promotion affects (a complete
  `can_promote` filter instead of the Java's partial one) cut nodes 25–35% but
  quadrupled the cost per node.  The preclusion table still pays on long
  searches (936: 15.7 s → 11.8 s) though it costs ~1.4 s of setup on one
  thread.  (No longer: see "Forced members and nogoods", below.)

Per change (search seconds and nodes; 1 thread for 760/880/936 seeded from
n−1, 10 threads for 1000; 10-thread times vary with when the optimum is
found):

| commit | change | 760 | 880 | 936 | 1000 (10 thr.) |
|---|---|---|---|---|---|
| f8d922d | conflicts, ladder, diagnostics (Java's branching) | 1.13 s 205k | 30.7 s 4.78M | 90.8 s 12.3M | open at 120 s |
| 6c4e6ee | branch on conflict (largest member) | 0.13 s 10.1k | 11.2 s 909k | 31.8 s 2.62M | 27.5 s |
| 264d1ab | cache of frames' conflicts | 0.12 s 10.1k | 10.5 s 909k | 29.5 s 2.62M | 30.7 s |
| d1c256c | strong branching | 0.12 s 7.2k | 3.8 s 243k | 17.7 s 909k | 22.8 s |
| 75155b4 | propagation into children | 0.12 s 6.5k | 3.6 s 202k | 16.8 s 759k | 18.7 s |
| 5ef83ea | keep the strong-branching level | 0.11 s 6.5k | 3.25 s 202k | 15.0 s 759k | 18.7 s |
| f6aef0f | local search on the start | 0.12 s 6.5k | 3.2 s 202k | 14.8 s 758k | 17.1 s |
| 1e7a3de | radix heap | 0.11 s 6.5k | 2.7 s 202k | 12.0 s 758k | 12.6 s |
| 38cf168 | refine one level | 0.09 s 4.0k | 2.6 s 142k | 11.1 s 500k | 11.1 s |

The ladder (10 threads, standalone from the frame builder, seeded from n−1):
before, 992 and 1000 were open at 100 s (992's incumbent not even optimal),
960 took 37 s and 800 13 s, and 945, 948, 950, 969, 976, 984, 987, 988, 990,
996, 999 were open at 20–30 s.  After, all of them complete: 760 0.03 s, 800
0.5 s, 840 0.1 s, 880 0.3 s, 900 0.1 s, 960 1.6 s, 945–999 1.4–24 s except
992 (62 s, of which 42 s finding the optimum) and 996 (69 s, all proof).
The whole range 761–1000 through `taxman solve` (prime shortcut and
`reuse_prev` on; 81 games searched) takes about 5 minutes of game time on 10
threads, every score matching optimal.json; 1001–1100 about 6.5 minutes
(36 searched, slowest 1025 at 100 s) and 1101–1200 about 2.5 minutes (with
`reuse_prev` and `revise_freedoms`, as the Java's results).  Threads scale
linearly to the 8 performance cores; the 2 efficiency cores add little.

Left to try: making the per-node conflict work cheaper (a quarter of the
time: peeling each frame `Q*` touches), fewer matching solves per node, and
incumbents for games like 992 whose optimum is found late.

### Forced members and nogoods (2026-10, second round)

Measured with the ladder on one thread (deterministic node counts) with the
frontier's tactics (`revise_freedoms=off`), seeded from n−1 (from
`results/strict.jsonl` past 1000, `--known`), on a fast set that each finish
in under 25 s: 880, 936, 1050 (optimum found late), 1100, 1170, 1184 (proof
only).  (The first rows were measured while a production run used 6 cores,
the rest on an idle machine; the first row measured again idle came out the
same to 1%.)  Where the time went at 1184 (`sample`): peeling for conflicts 29%
(nearly all of it minimizing them), matching repairs for refinement 18% and
for strong branching 16%, the candidate filter's peels 16%.

The tree is mostly chains: at four nodes in five the node's loop ends right
after its first child, because removing the member that strong branching
chose already brings the bound below what the node needs.  That member is
*forced*: every completion that beats the incumbent promotes it.  Strong
branching computes the bound without each member of the conflict, and at
1184 more than one member was forced at 54% of the nodes (two at 16%, three
at 10%, four or more at 28%).  Before, the child promoted one of them and
found the others again, one level each, further down.

* **Forced members** (tactic `fix_forced`, sound).  The child promotes every
  forced member of the conflict, not only the chosen one: the child's
  completions are some of the node's, so those that beat the incumbent
  promote them all, and if one cannot be promoted any more (or the child's
  candidate filter dropped it) the child has no such completion at all.
  Nodes −31% (880) to −60% (1170), time −21% to −36%.
* **Reusing the child check.**  A child's bound is checked (with refinement
  and its conflict) when it is made; its loop's first step repeated that with
  the same need for the same verdict.  It now keeps the verdict.  And a child
  that promotes several numbers makes its candidate list once, after all of
  them.  −3% to −23% time.
* **Nogoods** (tactic `nogoods`, sound).  A conflict `K` found at a node with
  promotions `P` is relative to `P`; a minimal subset of `P ∪ K` that keeps
  a frame of the *root* unwinnable is a *nogood*: it cannot be promoted from
  the root, so at every node anywhere in the tree (any thread, any path, and
  after a resume) the nogood minus that node's promotions is a conflict of
  the node (a subset of a promotable set is promotable; the proof is in
  `model::Nogoods`).  Measured first: at 1184, 291k conflicts were computed
  but they were only 854 distinct nogoods — the search found the same few
  hundred over and over.  So each searcher keeps a table of them (at most
  65536, forgotten all at once when full; about a thousand are learned near
  n = 1200), and a node's conflict is the smallest the table knows within
  `P ∪ Q*` (of those as small, the one with the largest member); only when it knows none
  is one computed, and then learned.  Nogoods also rule candidates out of a
  child (a nogood all of whose other members the child promotes, indexed by
  member), beyond the conflicts known along the path.  Looking conflicts up
  rather than computing them is cheaper, and also better for the tree: the
  smallest of the many conflicts known is often smaller than the one the
  minimization of a single frame finds, and branching on it and on the
  child's propagation cut nodes (936: 320k → 50k).  The tree now depends on
  what the searcher has learned, so on its history (threads, interruptions),
  not only on the node; answers do not.  Alone, lookups gave most of it
  (880 → 39k nodes, 936 → 57k, 1184 → 287k); propagation alone, with every
  conflict computed and made a nogood, cut nodes less (880 → 61k) and cost
  more than it saved.
* **All members forced.**  When every member of the conflict is forced, no
  completion beats the incumbent: the node is done (free once strong
  branching has computed the bounds; only matters with `match_refine=0`,
  since refinement prunes those nodes first).
* **Bounds without a member from the potentials** (`MatchBound::bound_excluding`).
  With the conflicts looked up, matching repairs were two thirds of the
  time, nearly all of them for the bound without one member of a conflict
  (strong branching, and refinement's branches).  The level's potentials are
  a dual solution whose sum is its bound; without the member as a candidate
  they stay feasible, and lowering the member's potential and that of the
  factor it paid with as far as their remaining edges allow gives an upper
  bound at once: `value − x + y′ₓ + y′_w`.  At 1184 it shows 89% of the forced
  members forced (and is never below the exact value, which the matching
  tests check).  Strong branching (with `fix_forced`) and refinement solve
  only for the members it does not settle; when some member is forced, the
  node's loop ends after the child that promotes them (it covers every
  completion that can beat the incumbent), so no level is kept for it.  The
  same trees, 13–28% less time.
* **Forced members of the whole matching.**  The bound from the potentials
  is cheap enough to compute for every member of the heaviest matching, not
  only for the conflict's: at each check that refines (or wants a conflict)
  the forced members of `Q*` are found that way.  If the nogood table knows
  a conflict all of whose members are forced, or the conflict found is, the
  node is done (diagnostics had shown that refining by the fresh minimal
  conflict instead of the table's would prune 16% of the nodes left open:
  other conflicts matter).  Otherwise, if any are forced, the child promotes
  them all and strong branching is not needed; only when none is does strong
  branching solve for its conflict's members.  The table is scanned once
  for both (the smallest conflict, and any conflict all forced), its
  nogoods' members ordered with the conflict they came from first, as those
  are the likeliest to be missing at another node.  Nodes −8% (936) to −52%
  (1184), time −2% to −39%; 992 seeded with its optimum (the proof alone,
  one thread): 2.89M → 2.11M nodes.

  Tried and not kept: proving more members forced with the forced ones
  required (a second pass of the bounds from the potentials on a level with
  them promoted) pruned nothing more; skipping the exact solves of strong
  branching when the bounds from the potentials found some member forced
  (instead of whenever they found any candidate forced), or of the
  refinement's branches, or choosing the member to branch on by those bounds
  alone, each traded nodes for time at about even.
* **Cheaper children.**  Peeling had become the largest cost (37% of the
  time at 1215: promotions and the candidate filter's checks).  With the
  matching bound, a child's candidate list leaves out the Java's
  promotability checks: a candidate that cannot be promoted is found by the
  conflicts and nogoods anyway, and nodes stayed within ±8%.  The forced
  members are promoted together, each frame checked once
  (`Model::promote_all_using`) instead of two frames per member (together
  they fit the frames' freedoms exactly when no frame they touch ends below
  zero).  And blameGame is not computed with the matching bound: it never
  pruned where the matching bound did not (diagnostics: 0 at every check).
  −30% to −59% time.
* **Nogood signatures.**  The table's scans were then 17% of the time at
  1215 (a few hundred nogoods, a lookup per check).  Each nogood keeps a
  256-bit signature (bit `x mod 256` per member), and a scan skips those
  whose signature is not within the node's; the conflicts chosen are the
  same as before (the smallest, then the one with the largest member).
  −5% to −10%.
* **No preclusion table** (`preclusions` now off by default).  With the
  nogoods it no longer pays for its setup: the searches of 992 and 1215
  seeded with their optimum take +5% and −4% without it (one thread), and
  1101–1217 through `taxman solve` on 8 threads spent 53 s of its 131 s
  building tables (0.8 s per searched game, 4.4 s on one thread at 1215) and
  72 s searching; without them 80 s in all, the search 74 s.
* **No refinement where members are forced.**  A node with forced members
  has a single child, which promotes them; refining the node's conflict is
  left to that child.  Nodes +3% to +11%, time −2% to −8% (992 and 1215
  seeded with their optimum: −3.5% and −6.5%).

Tried and not kept in this round:

* Rounding the heaviest matching into an incumbent at every node (drop the
  smallest member of a conflict until the rest can be promoted, then fill
  greedily): five times slower per node, and the optimum of 1050 was found
  at the same step.
* A large-neighbourhood search before the search: free a window of the
  incumbent's members (by frame pair, or 15–40 consecutive members), keep
  the rest promoted, and search that unit with a node limit.  At 1025 it
  reached 25080 in 1.3 s (the search takes 11 s to), but the optimum, 7
  members out and 8 in from there across frames 1–4, was found only by the
  search, at about the same point (one thread: 160 s instead of 170 s).
  The incumbents of 1025 and 1050 improve late because the search reaches
  the subtree that holds them late, not because a nearby set is missed.

One thread, fast set (search seconds and nodes; 880 and 936 seeded from
optimal.json, the rest from `results/strict.jsonl`):

| commit | change | 880 | 936 | 1050 | 1100 | 1170 | 1184 |
|---|---|---|---|---|---|---|---|
| 8686be0 | before | 2.72 s 142k | 11.65 s 500k | 24.8 s 598k | 4.81 s 161k | 4.58 s 165k | 21.6 s 786k |
| 53a442a | forced members | 2.14 s 98k | 8.61 s 320k | 18.2 s 342k | 3.17 s 74k | 3.09 s 66k | 14.7 s 338k |
| 261b342 | reuse the child check | 1.97 s 98k | 8.05 s 320k | 16.8 s 340k | 2.65 s 74k | 2.35 s 66k | 11.9 s 338k |
| c33c3c2 | nogoods | 0.66 s 34k | 1.09 s 50k | 11.8 s 318k | 2.12 s 75k | 1.18 s 46k | 6.41 s 276k |
| 667b24d | bounds from the potentials | 0.52 s 34k | 0.85 s 50k | 8.72 s 318k | 1.65 s 75k | 0.91 s 46k | 4.87 s 276k |
| 356ef92 | forced members of the whole matching | 0.46 s 31k | 0.83 s 45k | 6.45 s 241k | 1.31 s 51k | 0.92 s 35k | 2.94 s 133k |
| fe73c1c | cheaper children | 0.29 s 30k | 0.53 s 47k | 4.68 s 264k | 0.73 s 51k | 0.40 s 35k | 1.28 s 126k |
| 372c3f9 | nogood signatures | 0.26 s 30k | 0.49 s 47k | 4.43 s 264k | 0.68 s 51k | 0.38 s 35k | 1.21 s 126k |
| 49517a1 | no refinement where members are forced | 0.24 s 32k | 0.45 s 52k | 3.99 s 290k | 0.67 s 56k | 0.37 s 38k | 1.13 s 130k |
| 8bcfd36 | no preclusion table | 0.23 s 31k | 0.50 s 57k | 3.88 s 289k | 0.63 s 55k | 0.36 s 38k | 1.34 s 159k |

(Setup is left out: the preclusion table took 1.3–3.9 s of it on one thread
for these games, now nothing.)

The hard games, standalone in the ladder as above, on an idle machine (search
seconds; with several threads the times vary from run to run with when the
optimum is found, so two runs are shown for the last version; in
parentheses, when the final incumbent was found):

| game | 8686be0, 8 threads | 8bcfd36, 8 threads | 8bcfd36, 2 threads |
|---|---|---|---|
| 1215 | 195.6 s (71.4) | 12.1 s (7.4), 9.8 s (2.6) | 35.7 s (13.8) |
| 1216 | 51.5 s (start) | 3.1 s, 3.3 s | 10.4 s |
| 1025 | 100.1 s (76.7) | 12.5 s (4.6), 23.9 s (15.0) | 93.5 s (70.0) |
| 992 | 71.4 s (56.8) | 11.8 s (11.4), 12.0 s (11.8) | 40.0 s (39.2) |
| 996 | 76.4 s (start) | 10.2 s, 10.0 s | 31.5 s |

`taxman solve 1001-1217 --threads 8 --tactic reuse_prev=off --tactic
revise_freedoms=off` (no unproven tactic) reproduces every score of
`results/strict.jsonl` in 164 s of game time; those results took 1244 s on
10 threads.  Its slowest games: 1025 21 s (110 s before), 1215 10 s (161 s),
1020 6 s (40 s).

Left to try: the incumbents of games like 992 and 1025, whose optimum the
search reaches late (992 on 8 threads: at 11.8 s of 12.0 s; seeded with its
optimum it takes about 6 s); the matching repairs, still about 40% of the
time (strong branching where nothing is forced, and refinement); and the
multi-thread search: each thread learns its own nogoods (sharing them did
not look worth it: a proof on 8 threads takes about as many nodes as on one).

### Carrying nogoods over (2026-10): measured, not kept

A nogood is a fact about the root of a game's frame model, and consecutive
games have nearly the same model, so the nogoods a search ends with could
start the tables of the next game's search, each checked at the new root
first: a set that cannot be promoted from the root (an exact test, frame by
frame) is a nogood of it whatever it came from, and `conflict_using` makes it
minimal again.  For the same reason a checkpoint could store the tables and a
resumed search start with them.  A prototype measured both (commit f41fff8:
`search::valid_nogoods`, the solver's tactic `carry_nogoods`, `ladder
--carry`, `examples/resume_carry.rs`); neither pays, and both were taken out
again.

* Nearly every nogood survives into the next game: 1374 of 1374 from 1024 at
  1025, 482 of 494 from 1219 at 1220, 1216 of 1251 from 1229 at 1230.  But
  game n − 1 often does not search: a prime takes the shortcut, and 1214,
  1224 and 1226 reach the cap with their seed (no nodes), so in a range the
  nogoods come from the last game searched (1227's from 1225, through 1226).
* The table never stops growing, but learning costs little.  On one thread,
  half of what a hard game learns is learned in the first 3–27% of its search,
  90% by 18–64%, and the last nogood in its last 1–17% (1024, 1025, 1215,
  1219, 1220, 1225, and 1230 stopped at 560 s); each is one conflict computed
  where the table knew none, a few thousand in a search of millions of nodes.
  So what a preloaded table could save is not the learning but the search's
  knowing smaller conflicts sooner, which changes the tree.
* It changes, but not for the better (one thread, strict tactics, seeded from
  n − 1 and capped as the solver does; search seconds and nodes):

  | game | tables start empty | with the nogoods of |
  |---|---|---|
  | 1025 | 158 s 8.50M | 1024: 136 s 6.30M |
  | 1025, proof only (seeded with its optimum) | 99 s 5.25M | 1024: 104 s 4.97M |
  | 1220 | 186 s 13.72M | 1219: 232 s 16.47M |
  | 1220, proof only | 177 s 13.51M | 1219: 240 s 17.81M |
  | 1227 | 260 s 17.42M | 1225: 267 s 15.72M |
  | 1230 | open at 560 s (best 36570 at 118 s) | 1229: open at 560 s (36570 at 186 s) |
  | 1025, 4 threads | 35.2 s 8.51M | 1024: 38.4 s 6.82M |
  | 1220, 4 threads | 54.0 s 17.16M | 1219: 68.0 s 15.91M |

  1227 is the ceiling: 1225–1229 have the same frames and candidates (the
  same search, 17,417,736 nodes on one thread), so 1227 started with all that
  1225 learned is the best a carried table can be, and it takes 10% fewer
  nodes and 3% more time.  The chain `taxman solve 1201-1231 --threads 4
  --tactic reuse_prev=off --tactic revise_freedoms=off` from 1200 in
  `results/strict.jsonl` took 617 s (154.7M nodes) without and 941 s (174.7M)
  with it, every searched game slower (1230: 305 s against 445 s); there the
  table grows from game to game.
* Why: a larger table costs every node.  A lookup scans every nogood's
  signature and the nogoods of each new promotion rule candidates out, so
  nodes are slower the more the table holds (1227: 59k nodes/s from a full
  table against 67k/s from an empty one; on 4 threads, whose tables start
  with the union of all threads' nogoods, 177k/s against 242k/s at 1025).
* Keeping the tables over a resume: a search stopped every K nodes and
  resumed from its checkpoint, with the tables as they were when it stopped
  (in memory, what a stored table would give), reproduces the uninterrupted
  search; resumed with empty tables it is a different tree, and faster (one
  thread):

  | game, stopped every | uninterrupted | resumed with empty tables | with the tables kept |
  |---|---|---|---|
  | 1024, 250k nodes | 41 s 2.28M | 34 s 2.32M | 42 s 2.26M |
  | 1215, 500k | 66 s 5.49M | 60 s 6.22M | 69 s 5.47M |
  | 1220, 1.5M | 186 s 13.72M | 143 s 11.91M | 191 s 13.48M |

  So a resume loses nothing by starting empty, and checkpoints stay as they
  were.  That old nogoods cost more than they help is taken up in
  "Forgetting nogoods", below.

Noticed on the way: many games of a range have exactly the frames and
candidates of an earlier one (in 1201–1231: 1202–1203 those of 1201, 1205 of
1204, 1207 of 1206, 1210–1211 of 1209, 1213–1214 of 1212, 1217 of 1216, 1219
of 1218, 1222–1223 of 1221, 1226–1229 of 1225, 1231 of 1230), so the same best
promotion set, which a search of one of them has already established.  Only
the cap differs: where it is higher than the optimum, the search is repeated
in full.  In the strict chain above that is 1203, 1205, 1207, 1211, 1219 and
1227: 90 s of the 617 s.

### Forgetting nogoods (2026-10)

The measurements of carrying nogoods over pointed the other way: a search
resumed with empty tables was faster than one that kept them.  So each
searcher now clears its table after each 50,000 of its own steps (tactic
`forget_nogoods`, `search::NOGOOD_PERIOD`; sound, since the table is a cache
and every search starts with an empty one anyway; deterministic on one
thread).

The interval, on one thread (search time of 1024, 1025, 1215 and 1220 added
up, against never forgetting run alongside): every 10,000 steps 22% less
time, 25,000 22%, 50,000 26%, 100,000 25%, 250,000 21%, 1,000,000 15%.  The
node counts move both ways (1025 +7% to +28%, 1220 −11% to −28%); what the
interval buys is cheaper nodes, since a lookup scans the whole table (1215:
110k steps/s forgetting every 10,000 steps, 83k/s never).  A game of fewer
steps than the interval is unchanged.  Tried and not kept:

* Forgetting by size, clearing the table whenever it holds 64, 128 or 256
  nogoods: 15%, 22% and 17% less time on those four, but the smaller games
  slower, since they clear while still learning (936: 0.51 s against
  0.85/0.61/0.53 s; 1050: 3.96 s against 5.19/4.49/4.33 s).
* Keeping the nogoods used since the last clearing (returned by a lookup, or
  ruling a candidate out of a child): every 100,000 steps 15% less time,
  every 500,000 6%.  The ones kept cost lookups as much as the others and help
  no more.

Never forgetting against forgetting every 50,000 steps (search seconds and
nodes; strict tactics, seeded from n − 1 and capped as the solver does; the
fast set 880–1184 is unchanged to ±4%):

| game | one thread, never | one thread, forgetting | 4 threads, never (two runs) | 4 threads, forgetting |
|---|---|---|---|---|
| 992 | 73.3 s 4.00M | 52.6 s 4.20M | 19.5 s, 16.9 s | 18.0 s, 16.5 s |
| 996 | 69.0 s 4.17M | 61.9 s 4.78M | | |
| 1024 | 43.0 s 2.28M | 34.3 s 2.38M | | |
| 1025 | 168.5 s 8.50M | 140.5 s 10.11M | 35.3 s, 35.3 s | 32.1 s, 32.3 s |
| 1215 | 69.6 s 5.49M | 50.6 s 5.16M | 21.8 s, 21.0 s | 15.3 s, 16.1 s |
| 1216 | 22.4 s 2.10M | 21.1 s 2.22M | | |
| 1220 | 192.2 s 13.72M | 125.4 s 10.51M | 60.6 s, 55.1 s | 38.8 s, 40.5 s |
| 1225 | 262.4 s 17.42M | 154.2 s 12.07M | 54.7 s, 55.2 s | 31.9 s, 33.4 s |

The strict chain 1201–1231 on 4 threads (as above) takes 377 s instead of
571 s, with every score the same (1230: 158 s instead of 275 s; 1225 33 s
instead of 55 s; 1220 39 s instead of 62 s).

### Single-member conflicts belong to the node (2026-10)

Many of the conflicts the search computes have one member (`--diag`'s minimal
conflict sizes: the 25th percentile is 1 at 936, the 10th at 1184).  They
looked like candidates that the Java's filter lets through and that cannot be
promoted from the root at all, which every searcher, work unit and resume
would find again; testing each candidate once from the root and dropping the
failures would then be free.  They are not root facts:

* **The frame builder's candidates all pass that test.**  A hive's promotion
  candidates are its factors that can leave it (the remaining sources can
  still be matched without them) and that some hive can take on as a source,
  which comes to what `Model::can_promote` checks at the root (the home frame
  without the factor, the frame below with it as a move).  For every game from
  2 to 1230 none of the candidates fails that check (a test), so a filter at
  the root drops nothing.  A version built as a tactic (the checkpoint header
  recording the candidates it leaves, older checkpoints refused as a different
  candidate set) searched 936, 1100, 1184 and 1215 on one thread in exactly the
  same nodes as without it (58,903, 56,399, 167,960 and 5,161,817), with 0 of
  225, 273, 287 and 302 candidates dropped; it was not kept.
* **The conflicts are relative to the node.**  A nogood of one member would be
  a candidate that cannot be promoted from the root, so the table never holds
  one.  A single-member conflict is a candidate that cannot be promoted once the
  node's promotions are made (its frame may have used it up in a forced pair,
  say); in another branch, with other promotions, it is a nogood of two or more
  members.  Looking it up is cheap, and the node's loop then goes on without
  the candidate.
* **Little is left to save.**  The promotions that fail are 2,282 of 58,903
  steps at 936 (3.9%) and 698 of 167,960 at 1184 (0.4%) (`--diag`, one thread),
  which bounds what a search that never tried a dead candidate could gain.

## Testing

Correctness, all on games that finish in seconds:

* Unit tests per module (`cargo test`), including the Java tests' cases
  (FactorTableTest, FrameTest, FrameSetTest for n=85).
* Golden tests: every stage of the port against `testdata/golden/n*.json`
  (exported from the Java by `tools/golden/export.sh`): hives, frames,
  candidates, reduced frame set, greedy incumbent, revised freedoms, solve
  result.
* Regression: `taxman regress --to N` solves 1..N as one range and checks
  every score against optimal.json (the Java's own test: 146 accelerated
  games in 1..200).
* Tactic invariance: the regression with each unproven tactic off must give
  identical scores; so must games 1..300 forced to search with each sound
  tactic off or changed (each `branching`, `match_refine` 0 and 3,
  `fix_forced`, `nogoods`, `forget_nogoods`, ...), and a checkpoint is refused
  when any of the nine search tactics differs.
* Bound soundness: on small frame sets, the upper bound is never below the
  exhaustively computed best completion; the same for the matching bound and
  for its refinement by conflicts (whose exact completions must be
  promotable, and whose conflicts, carried down a path, must stay
  conflicts).  Conflicts and cores are checked against promoting the sets.
* Golden parity: the searched golden games (94, up to 760) give the Java's
  promotion sums with the default tactics; the tactic variants give the same
  answers on those up to 400.
* Engine: a toy problem with known optimum, run with 1 and many threads,
  with a tiny `queue_mem_cap` to force spilling, and killed/resumed from
  checkpoints, must give the same answer; also with 64 and 192 threads (far
  more than the cores, so any scheduling happens), with checkpoints every
  2 ms, spilling, resuming and a panicking worker (these use every core for
  seconds, so they are ignored by default: `cargo test --release --test
  engine -- --ignored many_threads`).  An ignored stress test
  draws random configurations (1 to 192 threads, caps, intervals, limits)
  and runs each to completion through any number of resumes
  (`TXENGINE_STRESS=5000`: 200 s, all passed); the instrumentation's
  accounting is checked on every run.  A tree with no pruning
  (`tests/robustness.rs`) checks conservation exactly: under 40 random
  configurations (1 to 16 threads, caps, intervals, limits, searchers that
  leave childless nodes out of their yields; `TXENGINE_CONSERVATION=<n>`
  for more) the nodes counted must equal the tree's size, so one unit lost
  or explored twice shows, which an optimum alone does not.
* Robustness (`tests/robustness.rs`): a checkpoint interval of zero
  finishes; intervals and limits too large for the clock are never reached;
  a completed run can keep its checkpoint; spill segments of dead processes
  are removed; a locked checkpoint directory is refused; game 140's
  checkpoint renamed to 141's, one with other tactics (named in the error),
  and one whose incumbent is not a promotion set of the game or not worth
  its stored value are refused; a resumed search already at its cap
  finishes at once; a limit stops `revise_freedoms`' own search; and the
  binary rejects bad times and limits, survives a closed stdout and stderr,
  and stops on SIGINT and SIGTERM with a checkpoint it then resumes.
* Large boards (`tests/wide.rs`): `optimal.json` and the Java stop at 1000,
  and the sets that hold a board are 8, 16, 32 and 64 words wide for n up to
  511, 1023, 2047 and 4095, so nothing above 1023 meets anything but the
  committed results (`results/strict.jsonl`, `default.jsonl`) and these
  tests.  The same game at every width that holds it has the same frames,
  candidates, greedy set, promotion cap, answer and tree (nodes counted); the
  last board of each width (511, 1023, 2047) at the width and at the next;
  game 1133 (W = 32, about 2,400 nodes) under every sound tactic against
  `strict.jsonl`, on one thread and two; a slice of the strict chain
  1127..1134; the sacrifice bound proves game 1133 by an independent route;
  a W = 64 board (2050) through frames, matching bound, conflicts, greedy set
  and a node-limited search; a W = 32 search interrupted and resumed under
  several tactics, and a committed checkpoint of game 1133
  (`testdata/n1133_after_1000_nodes.ckpt`) that must still resume (a
  deliberate change of the checkpoint format fails it; the test says how to
  make the file again); and the levels of a real depth-first stack of game
  1215 (`testdata/n1215_deep_units.txt`, 0 to 34 promotions deep): the
  incremental matching bound and the quick bounds without a member against the
  min-cost-flow reference, and the subtrees of the deep levels under every
  tactic.  The Bits unit tests cover W = 16, 32 and 64, every word included.
* Java fixtures past 760 (`tests/golden_hives.rs`): `testdata/golden_wide/`
  holds the Java's structure (hives, frames, candidates, reduced frame set,
  greedy incumbent) for n = 511, 512, 1000, 1001, 1023, 1024, 1025, 1100,
  1231, 2047 and 2048, checked at the width `words_for(n)` picks;
  `accel_java.txt` the Java's acceptance of the previous game's promotions
  for 94 boards of 763..1228 (with the fingerprint of the previous game the
  decision was made with); `games_java.txt` its apiary game, move for move,
  for 171 sets of promotions at 762..2049 that are not only the optimal ones.
  The Java was run without `-ea` (its limit of 1023 is an `assert`, and
  `tools/golden/export.sh` runs with `-ea`, so for n > 1023 take its last line
  without the flag) with `GoldenExport --structure-only`, and with
  `tools/golden/org/.../AccelProbe.java` and `GameProbe.java` for the text
  files, which say how their lines are made.  Every second line of the two
  text files is checked (`FIXTURE_STRIDE`).
* Node-count tripwires (`tests/tripwires.rs`): a pruning tactic that stops
  working changes no answer, so no assertion on answers sees it (a mutant
  that ignored the result of `Nogoods::ruled_out` costs 27 to 67 % more
  nodes).  With one thread the tree is deterministic, so the exact node count
  of a fixed game under fixed tactics (n = 480 to 1133, every tactic off or
  changed in turn) is asserted, and the tree of game 600 with the searchers
  forgetting their nogoods every 500 steps (which sees the propagation of
  known conflicts, that no game below 50,000 steps does otherwise).  **A
  change that alters the tree on purpose fails them.**  Run `cargo test
  --test tripwires -- --nocapture`: it prints the rows as observed, in the
  layout of the table at the top of the file; check that the promotion sums
  (the answers) have not moved, paste the rows in, and update the node counts
  quoted in this file.
* Soundness (`tests/soundness.rs`): the search is started with an incumbent
  worth 1 or 5 less than the optimum and an empty payload, so that any
  pruning that loses every best completion leaves the fake incumbent in
  place (this tests the `<` and `<=` boundaries too): every game 2..=300
  under each of 20 configurations (every tactic, forgetting every step, every
  third, never; one thread and two); the same for 500 perturbed frame sets
  (numbers dropped, role flipped, moved a frame; half of them with frame 0's
  freedoms revised as `prepare` does) with the truth found exhaustively, at
  the root and three levels down; promoting a set at once against one by
  one in random orders, conflicts and nogoods on perturbed models; and
  searches interrupted every 1,500 nodes, resumed from a checkpoint every
  millisecond with units spilling.  A perturbed game whose revision changes
  the best sum is left out: the search takes the revised freedoms for a fact
  about the best set (the conjecture of `revise_freedoms`), and for a game
  that breaks it (one in about 1,300 of the perturbed ones) may return a set
  that they forbid.
* Command line (`tests/cli.rs`): every subcommand's happy path and its usage
  errors through the real binary; `bench --history FILE` keeps the tracked
  `bench/history.jsonl` out of the tests.
* Cost: the whole suite takes about 28 s with two test threads in the test
  profile (`opt-level = 2`), 24 s in release (before the tests of the
  last four bullets and the trimming: 30 s and 26 s).  A few tests sample what an
  exhaustive run would cover (every 18th golden board for the peeling
  agreement, every sixth board above 511 for the root promotability of the
  frame builder's candidates, every second line of the Java fixtures); the
  comment on each says what the whole run costs.

Performance:

* `taxman bench` runs a fixed set of games that need search and take about a
  second each on one thread — n = 936 (W = 16) and 1050, 1100, 1133, 1170,
  1184 (the frontier's width) — standalone (n−1 from optimal.json, past it
  from `results/strict.jsonl`; acceleration off), prints per-phase time and
  node counts, and appends them to `bench/history.jsonl`.  Until 2026-10 the
  set was 480–760 (W = 8 and 16, milliseconds per game), which no longer
  exercised the frontier's hot paths; the Java took 2.0–65.6 s for those on
  one thread and 1.4–12.4 s on ten.  A window of a long proof is not a game
  the solver can run, so it is in the probe.
* The probe (`examples/perf.rs`): replays of the hot paths at the frontier
  from fixtures in `testdata/perf/` (0.2 MB), measured in cycles and
  instructions retired of the process (`proc_pid_rusage`), which other load
  on the machine barely moves, unlike wall time; each with a fingerprint of
  what it computes, which an exact optimization must leave unchanged.  F1:
  300 search nodes each of the proofs of 936 and 1215 (`nNNNN.nodes`), every
  kernel over all of them: the rebuild of a node, the promotion (peel) of
  every candidate, conflicts, the matching from scratch and warm-started,
  bounds without each member, strong branching's exact solves, children,
  nogood lookups and rule-outs.  F2: the frame checks of those promotions.
  F3: a checkpoint of each proof (`nNNNN.window.ckpt`, TXSH v3; 1215 after
  1.5M nodes) resumed on one thread for 50,000 steps, one period of
  forgetting nogoods: the search itself, fingerprinted by its counters.
  About 5 s in all; `perf make-nodes` and `perf make-window` make the
  fixtures again (deterministically).  For whole searches, `/usr/bin/time
  -l` reports the same counters per process.
* The ladder, `cargo run --release --example ladder -- [--threads T[,T…] |
  --scaling] [--seconds S] [--diag] [--seed-opt] [--known FILE] [--tactic …]
  N…`: standalone searches of hard games (default 760 800 840 880 900 960 992
  1000) from the frame builder, seeded from the known n−1 game and capped as
  the solver does, each with a time limit; reports completion, nodes, time,
  when the final incumbent was found, and the engine's counters.  The known
  games are optimal.json's plus, with `--known`, a results file's
  (`results/strict.jsonl` for games past 1000).  `--seed-opt` seeds with the
  game for n itself, to measure the proof alone; `--diag` prints
  `search::Diagnostics` (nodes by depth, what pruned, bound gaps, conflict
  sizes, matching work).  Fast progress benchmarks (one thread, `--tactic
  revise_freedoms=off`): 880, 936, 1050, 1100, 1170, 1184; harder ones: 992,
  996, 1025, 1215, 1216 on several threads.
* Scaling: the ladder with a list of thread counts (`--threads 1,2,4,8`, or
  `--scaling` for 1, 2, 4, … up to all cores) searches each game with each
  count and prints a table per game and one summed over the games: search
  time, speedup, nodes per second per thread, effective parallelism, idle
  and lock-wait time, splits and the units they gave, steals, runs, tiny
  runs and the time to full occupancy.  With
  `--seed-opt` the node count barely depends on the threads, so the speedup
  measures the parallel proof alone.  On the M1 Max (8 performance cores,
  with another job on up to 4 of them) with the proof of 1215 (`--seed-opt
  --known results/default.jsonl --tactic revise_freedoms=off`): 62.5 s on
  one thread, 24.6 s on 2, 15.8 s on 4 and 12.9 s on 6, effective
  parallelism 99.9–100%, all workers busy within 3 ms; node counts vary by
  ±10% with the threads (what each thread's nogood table has learned).
  Games that take minutes on one thread (1215, 1220, 1225 with `--seed-opt`:
  about 60, 150 and 200 s) suit a many-core machine; at 192 threads a game
  of a minute is a third of a second, mostly ramp-up and tail.  The engine's
  own benchmark (`cargo test --release --test engine -- --ignored
  --nocapture bench`, sizes and thread counts set by `TXENGINE_BENCH_*`)
  measures its overhead per node with no work per node.
