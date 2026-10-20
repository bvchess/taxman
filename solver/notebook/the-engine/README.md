# The engine

Part of the [lab notebook](../README.md).

Started 2026-10-12.  Concluded 2026-10-12.

**Question.**  How should the engine ([DESIGN.md, "Engine"](../../docs/design/engine.md#engine); the protocol is in the
module comment of `src/engine.rs`) hand out work, so that many threads stay busy, and pause for a checkpoint at any
interval?  **Answer.**  Keep the one locked deque, give every idle worker its own open split request, leave nodes with
nothing to do out of a yield, and answer a pause only after 64 polls of a unit: all workers are busy within 3-8 ms, the
lock waits 0.00-0.02% of the time at 192 threads, and every pause is followed by work.

## Results

* **Yields leave out nodes with nothing to do.**  The Taxman searcher leaves out of its yield the nodes of its stack
  that have no candidates left (a node whose loop `fix_forced` ended): as units they would only be runs of one step
  that end the loop.  At 2 threads on games 936, 1100 and 1184 (twice each, `ladder --threads 2,2`), the runs of at most
  one poll fell from 60 and 91 of 256 and 305 to 13 and 10 of 227 and 200.  The nodes and times did not change.
* **Splits sized for many cores** (for a 192-core machine).  The first design answered one split request at a time,
  which serialized the distribution of work at many threads.  The engine keeps one open split request per idle worker
  that the queued and held units will not serve, and any running worker can claim one (an atomic decrement of a count in
  the word `poll` reads), so as many workers split at once as there are idle workers to feed.  From one unit, all
  workers are busy after a few rounds in which every running worker splits, within 3-8 ms (the ladder's `full`).  A
  worker answers only after 64 polls of its current unit, so a fresh unit first gets deep enough to have something to
  give, and no worker restarts its unit at every step while the demand lasts.
* **One deque, kept.**  A unit exists only after a yield, and each costs its searcher a state rebuild (for Taxman a
  matching solved from scratch, 30-170 µs already at games 480 to 760), against about a microsecond under the lock.
  With every worker busy nobody takes the lock, so per-worker deques with stealing would remove contention that is not
  there, at the price of distributed ordering, spill, checkpoint and termination arguments.  `Counters::lock_wait`
  measures the lock's wait time: 0.00-0.02% at 192 threads on Graviton4 ([scaling to 192
  threads](../scaling-to-192-threads-2026-10/README.md)).
* **Checkpoint pauses at any interval.**  An interval of 0 paused again at once without letting go of the lock, and the
  workers never ran again.  A periodic pause begins only once the workers released from the last one all run again,
  and no sooner than the last pause lasted; a worker answers it (as a split request) only after 64 polls of its current
  unit, so every pause is followed by work.  A pause costs every worker the wait for the last one to park, bounded by
  the longest stretch between two polls, one Taxman step: about 10 µs, over a millisecond about once in 10⁵ steps, the
  longest measured 10-15 ms at games 1215 to 1225, plus the write and fsync.
* **Checkpoint headers.**  Headers before version 3 (magic `TXS2`) recorded neither n nor the frames: a checkpoint of
  game 140 passed for one of game 141 (same candidates, same frame 0, other frames).  The engine refuses them with that
  reason rather than accepting them unchecked.
* **Set widths.**  Widths of 20, 24 and 28 words between 16 and 32 pay near the frontier: `Bits<20>` instead of
  `Bits<32>` takes 6% fewer cycles at games 1184 and 1215.  Each width is a copy of the search's code: the release build
  took 25 s with four widths, 50 s with the seven built, and 62 s with 12 and 48 words too.  A checkpoint records its
  width, and a search resumes at that width if the build has it (an older build chose 32 for games 1024 to 2047).

## Decision

The five changes are kept.  Per-worker deques are not built; contention on the lock in a many-core run would reopen
that, and `Counters::lock_wait` shows it.  The widths of 12 and 48 words were left out on 2026-10-12, when games 512 to
767 were solved and games 2048 to 3071 far off ([faster nodes](../search/faster-nodes-same-trees-2026-10/README.md));
when games 2048 to 3583 came into reach, 48 and three other widths were added.

## Commits

* 86b1918: yields leave out nodes with no candidates left.
* 988cc85: concurrent splits and instrumentation for many cores.
* 1ad2e55: checkpoint pauses at any interval, and the checkpoint headers.
* b4cb2b6: finer set widths (8, 16, 20, 24, 28, 32 and 64 words).
* acf964e (2026-10-14): widths of 36, 40, 48 and 56 words for games 2048 to 3583.
