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
  at most one checkpoint interval (see the repo's CLAUDE.md).
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
| `prime_shortcut` | on | proven | prime n: replace the n−1 solution's first move with n |
| `reuse_prev` | on | unproven (conjecture-grade "upper-delta" certificate) | `FrameSolver.reusePrev`; accepts a derived solution without search |
| `revise_freedoms` | on | unproven ("max-sum promotion set also has max count") | `UpperBoundEstimator.reviseFreedoms`; can shrink frame 0's freedoms |
| `preclusions` | on | sound (promotion failure is monotone: more moves or fewer factors never make a frame winnable) | depth-2 preclusion table; prunes candidates |
| `match_bound` | on | sound (matchability relaxes winnability) | node bound = min(blameGame, matching relaxation), the matching one computed only when blameGame fails to prune; a heaviest matching that can be promoted is the node's best completion (offered as incumbent, node done) |
| `branching` | `conflict-strong` | sound (any order explores every promotion set) | which candidate a node promotes next: `largest` (the Java's order), or a member of a conflict of the heaviest matching (these need `match_bound`): the smallest (`conflict-smallest`), the largest (`conflict-largest`), or the one whose removal lowers the matching bound most (`conflict-strong`) |
| `match_refine` | 1 | sound (see `search::Refiner`) | refine the matching bound by branching on conflicts this many levels deep, where it does not prune (0 = off, at most 7; needs `match_bound`) |
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
`match_refine` as a number and `branching` as its name), and whether any
unproven tactic was on.  A checkpoint records the six search tactics and is
refused when they differ.

## Architecture

One Cargo crate in `rust/` (library + one binary, `taxman`), no third-party
dependencies.  Modules:

| module | contents |
|---|---|
| `bits` | `Bits<const W: usize>`: fixed-width bitset over `[u64; W]`, `Copy`, no heap; capacity `64·W − 1`.  Set ops, popcount, sum, min/max, ascending and descending iteration, `nth_largest`, `largest(k)`, `next_below(x)`, `is_subset`, hashing. |
| `numtheory` | smallest-prime-factor sieve; for 1..=n: maximal factors (n/p for each prime p dividing n; {1} for primes; {} for 1) and maximal composites (k·p ≤ n), as `Vec<Vec<u32>>` and as bitsets; all divisors; prime test |
| `json` | minimal JSON reader/writer (enough for optimal.json, golden data, results, checkpoints' headers) |
| `game` | `replay(n, moves) -> Result<score>`: independent legality check (each move on the board, pays ≥ 1 tax, takes all divisors still present); optimal.json loader |
| `hives` | port of `Apiary`/`Hive`/`Namer`-free equivalent: hive formation with `treat_as_sources`, unzip, removeImpossible, finishSetup propagation, merge, promotion candidates, per-hive solution, apiary solution order |
| `frames` | port of `FrameBuilder` (levels of hives) producing columns (factors, moves) per frame bottom-up, candidates, `estimateMaxPromotions` |
| `accelerate` | deriving game n from game n−1 (prime shortcut, `reusePrev`), and turning a promotion set into an ordered game |
| `model` | the search state: `Frame`, `FrameSet` (fixed array of frames), `FactorProvider` (factor/composite lists restricted to frame numbers), `FrameMapper` (home frame of each candidate), `promote_into`, `promote_out_of`, `promote`, `can_promote`, `is_winnable` (peel), `reduce`, `narrow_candidates`, `max_promotions`; `promote_all` (a set checked frame by frame); `Certificate` (a winning order that answers most single-edit winnability questions without a peel); conflicts: the *core* of an unwinnable frame (`Scratch::stuck`, the moves a peel cannot drop) and `conflict_using`/`conflict_cached` (a small, by default minimal, subset of a promotion set that cannot be promoted), with `ConflictCache` |
| `bounds` | preclusion table; `upper_bound` (blameGame + largestThatFitInFrames, sound or legacy runner-up) and its exact threshold test `reaches`; candidate lists; greedy lower bound |
| `matchbound` | the matching bound (see "Improvements after parity"): `match_bound`, a min-cost-flow reference; `MatchGraph` (the root's graph) and `MatchBound` (an optimal matching and its potentials per level of the DFS path, repaired incrementally by Hungarian searches on a radix heap; levels with a candidate excluded, which can be kept for the parent) |
| `engine` | problem-independent parallel depth-first branch-and-bound driver: work units, shared deque with disk spill, incumbent, yielding/splitting, checkpoint and resume, progress reporting, statistics |
| `search` | the Taxman searcher (port of `BranchExplorer`) implementing the engine's `Searcher` trait; `prepare` (preclusions, greedy, `revise_freedoms`) and `search` (seed, cap, checkpoint and resume); `Refiner` (conflicts of the heaviest matching: exactness, refinement, branching); the starting incumbent's `dive` and local search (`Neighbourhood`); `search_with` and `Diagnostics` for measurements |
| `examples/ladder.rs` | the ladder harness: standalone searches of hard games from the frame builder with a time limit (see Performance) |
| `solver` | `Tactics` (every tactic above, by name and value), `SolveOptions`, `solve_game` (the per-game pipeline below) and `solve_range` (a range of games, resumable through a results file) |
| `main.rs`, `cli.rs`, `commands.rs` | CLI: `solve`, `regress`, `bench`, `verify-optimal`, `tables` (`taxman help` lists the options) |

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
filter, exactly as in `BranchExplorer.makeNewCandidateList`, then the
conflicts known along the path propagated (see "Improvements after parity").
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
  holds the shallowest — largest — subtrees.  A worker yields when (a) some
  worker is idle and the deque is empty (only one worker answers each such
  request), (b) a checkpoint is due, or (c) a stop is requested (time limit or
  node limit).  With one thread the run is deterministic (no checkpointing).
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
  The caller's header stores everything the setup computed that is
  expensive or must not change on resume (n, tactics, revised freedoms,
  max_total, root candidates).
* **Progress**: every `progress_interval` (default 60 s) one line on stderr:
  elapsed, nodes, nodes/s, incumbent, deque size, spilled units.

### Per-game pipeline (`solver`)

1. Board tables for n.
2. If `prime_shortcut` and n is prime and the n−1 solution is known: replace
   its first move with n (method `prime`).
3. Hives → frames.  With no promotion candidates the game with no promotions
   is the only one (method `trivial`).
4. Else if `reuse_prev`: port of `FrameSolver.reusePrev` (method `reuse`).
5. Else search (method `search`): columns → `FrameSet`; preclusions;
   greedy; `revise_freedoms` (which can run a nested two-frame search through
   the same engine); `max_total`; the starting incumbent (the best of greedy,
   the seed, and the root matching's rounding improved by local search); run
   the engine.
6. Solution = `Apiary(board, promotions)` moves in hive order; replay to check
   legality; compare the score with optimal.json when n is there (mismatch
   is a hard error).
7. Append one JSON line to the results file: n, score, moves, promotions,
   method, completed, nodes, search counters, seconds per phase, tactics (and
   which unproven ones were on), whether the score matched optimal.json.

Steps 2 and 4 are skipped by `force_search`, and when the game has a
checkpoint (a search that was interrupted is resumed, never replaced by a
derivation).  A checkpoint is `<dir>/n<NNNN>.ckpt`; resuming one made for
another game, width or search tactics is refused rather than restarted.  A
search stopped by a time or node limit gives an incomplete result (`completed`
false, the best game so far) and keeps its checkpoint; a range stops there.

Range runs feed each game's solution to the next and resume from the
results file (games completed in it are skipped; each result is appended and
synced as soon as it is done).  The n−1 solution comes from, in order: this
run, the results file, optimal.json.

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
  quarter are single members: candidates the Java's filter leaves in that
  cannot be promoted at all).  A subset of a promotable set is promotable, so
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
  thread.

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
  tactic off or changed (each `branching`, `match_refine` 0 and 3).
* Bound soundness: on small frame sets, the upper bound is never below the
  exhaustively computed best completion; the same for the matching bound and
  for its refinement by conflicts (whose exact completions must be
  promotable, and whose conflicts, carried down a path, must stay
  conflicts).  Conflicts and cores are checked against promoting the sets.
* Golden parity: the searched golden games (93, up to 760) give the Java's
  promotion sums with the default tactics; the tactic variants give the same
  answers on those up to 400.
* Engine: a toy problem with known optimum, run with 1 and many threads,
  with a tiny `queue_mem_cap` to force spilling, and killed/resumed from
  checkpoints, must give the same answer.

Performance:

* `taxman bench` runs a fixed set of games that need search and finish in
  seconds — n = 480, 500, 600, 640, 660, 700, 720, 760 — standalone (n−1 from
  optimal.json, acceleration off), and prints per-phase time and node counts.
  Java reference wall times, whole game including JVM start: 10 threads
  1.5, 1.4, 1.7, 1.6, 2.1, 4.8, 5.6, 12.4 s; one thread (ForkJoin
  parallelism 1) 2.3, 2.1, 2.7, 2.0, 4.1, 18.0, 23.2, 65.6 s.  Compare with
  1 thread (deterministic) and with all threads.
* Larger games (738, 750, 840; Java 56–90 s) only after the bench set is
  fast and correct.
* The ladder, `cargo run --release --example ladder -- [--threads T]
  [--seconds S] [--diag] [--seed-opt] [--tactic …] N…`: standalone searches
  of hard games (default 760 800 840 880 900 960 992 1000) from the frame
  builder, seeded from optimal.json's n−1 game and capped as the solver does,
  each with a time limit; reports completion, nodes, time and when the final
  incumbent was found.  `--seed-opt` seeds with the game for n itself, to
  measure the proof alone; `--diag` prints `search::Diagnostics` (nodes by
  depth, what pruned, bound gaps, conflict sizes, matching work).  Fast
  progress benchmarks: 880 and 936 on one thread (seconds), 1000 on ten.
