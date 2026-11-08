# Status

Updated 2026-11-07.

The state of the work as of the date above: the priority, and the projects in flight.  This page is rewritten, not
appended to, when the state changes.  What was done and why is in the [lab
notebook](https://github.com/bvchess/taxman-notebook/blob/main/README.md), how far the chain reaches in
[results/README.md](../../results/README.md), and the research avenues in the
[agenda](https://github.com/bvchess/taxman-notebook/blob/main/agenda/AGENDA.md).

## The priority

The Nov 1 run finished on 2026-11-06 at 05:00 PST, and its file is `results/optimal.jsonl.gz`: the games 1 to 5060, each
proven, by one build ([results/runs.md](../../results/runs.md)).  The cleanup of the frame solver is finished; its last
step retired the bounds store, the ledger, `--use-prev`, the readers of older line formats, `certify`, the certificates
and the extended mode.  The prover's proofs of 2 to 2531 are done and committed.  The priority is the unsound mode.

## In flight

* **The prover's proofs of 2 to 2531.**  Done: one run of commit 9015f79 on batmobile, which ended on 2026-11-07 at 00:21
  PST, proved every game of 2 to 2530, and a one-game run of `exact` added game 2531 by the prime step.
  `tools/summarize.py check 2 2531` passes on the file, which is committed as
  `solver/noframe/data/exact_parity_2_2531.jsonl` and replaced the older `exact_parity` files ([the prover's
  proofs](../noframe/docs/proofs.md#the-run-of-2-to-2530) record the run).
* **The unsound mode.**  A mode of `taxman solve` that plays the games past 5060 with no proof, at six levels of effort,
  and repairs a line that a line above it shows short; its lines are lower bounds, kept apart from the proven ones
  ([the unsound mode](unsound.md)).  Two projects are in flight:
  * the full measurement of the levels on the whole machine;
  * the streaks of short games that a chain hands on.

## Open

* **The fixed gate.**  Why the gate of 300 lost on every game searched from 4984 to 5022, at 2.0 times the nodes of the
  earlier file's searches (a gate of 1200, or conflict branching switched off), and won on every one from 5024 to 5060
  against the same gate of 1200 and build; and whether a gate scaled to the game's root gap, never tested, does better
  ([the full run at gate
  300](https://github.com/bvchess/taxman-notebook/blob/main/search/the-full-run-at-gate-300-2026-11/README.md)).
