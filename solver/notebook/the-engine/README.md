# The engine

Part of the [lab notebook](../README.md).

How the engine ([DESIGN.md, "Engine"](../../docs/design/engine.md#engine); the protocol is in the module comment of
`src/engine.rs`) came to be as it is.  All of this is from 2026-10 unless noted.

* **Yields leave out nodes with nothing to do.**  The Taxman searcher leaves out of its yield the nodes of its stack
  that have no candidates left (a node whose loop `fix_forced` ended): as units they would only be runs of one step
  that end the loop.  At 2 threads on 936, 1100 and 1184 (twice each, `ladder --threads 2,2`) the runs of at most
  one poll fell from 60 and 91 of 256 and 305 to 13 and 10 of 227 and 200, the nodes and times unchanged.
* **Splits, sized for many cores** (for a 192-core machine).  The first design answered one split request at a time,
  which serialized the distribution of work at many threads.  The engine now keeps one open split request per idle
  worker that the queued and held units will not serve, and any running worker can claim one (an atomic decrement
  of a count in the word `poll` reads), so as many workers split at once as there are idle workers to feed.  From
  one unit, all workers are busy after a few rounds in which every running worker splits, not after one split per
  idle worker.  A worker answers only after 64 polls of its current unit, so a fresh unit first gets deep enough
  to have something to give, and no worker restarts its unit at every step while the demand lasts.
* **One deque.**  The deque stays a single locked one: units exist only after yields, each costs its searcher a
  state rebuild (for Taxman a matching solved from scratch, 30–170 µs already at n = 480–760, against about a
  microsecond under the lock), and with every worker busy nobody takes the lock.  Per-worker deques with stealing
  would remove contention that is not there, at the price of distributed ordering, spill, checkpoint and
  termination arguments.  The lock's wait time is measured (`Counters::lock_wait`) so that a many-core run shows
  whether that ever changes.
* **Checkpoint pauses at any interval.**  Before, an interval of 0 paused again at once without letting go of the
  lock, and the workers never ran again.  Now a periodic pause begins only once the workers released from the last
  one all run again, and no sooner than the last pause lasted; and a worker answers it (as a split request) only
  after 64 polls of its current unit, so every pause is followed by work.  A pause costs every worker the wait for
  the last one to park, bounded by the longest stretch between two polls, one Taxman step: about 10 µs, over a
  millisecond about once in 10⁵ steps, the longest measured 10–15 ms at n = 1215–1225, plus the write and fsync.
* **Checkpoint headers (2026-10-12).**  Headers before version 3 (magic `TXS2`) recorded neither n nor the frames: a
  checkpoint of 140 passed for one of 141 (same candidates, same frame 0, other frames).  They are refused with that
  reason rather than accepted unchecked.
* **Set widths.**  The widths of 20, 24 and 28 words between 16 and 32 pay near the frontier: `Bits<20>` instead of
  `Bits<32>` takes 6% fewer cycles at 1184 and 1215 (2026-10).  Each width is a copy of the search's code: the
  release build took 25 s with four widths, 50 s with the seven now built (62 s with 12 and 48 too, which were left
  out: 512–767 is solved, and 2048–3071 far off).  A checkpoint records its width, and a search resumes at that
  width if this build has it (an older build chose 32 for 1024–2047).
