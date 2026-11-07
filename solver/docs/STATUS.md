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
and the extended mode.  The prover's run of 2 to 2530 is done and committed.  The priority is the unsound mode.

## In flight

* **The prover's run of 2 to 2530.**  Done: one run of commit 9015f79 on batmobile, which ended on 2026-11-07 at 00:21
  PST, proved every game of 2 to 2530.  `prover.sh --verify 2 2530` passes on its file, which is committed as
  `solver/noframe/data/exact_parity_2_2530.jsonl` and replaced the older `exact_parity` files ([the prover's
  proofs](../noframe/docs/proofs.md#the-run-of-2-to-2530) record the run).

## Open

* **The fixed gate.**  Why the gate of 300 lost on every game searched from 4984 to 5022, at 2.0 times the nodes of the
  earlier file's searches (a gate of 1200, or conflict branching switched off), and won on every one from 5024 to 5060
  against the same gate of 1200 and build; and whether a gate scaled to the game's root gap, never tested, does better
  ([the full run at gate
  300](https://github.com/bvchess/taxman-notebook/blob/main/search/the-full-run-at-gate-300-2026-11/README.md)).
* **The unsound mode.**  A mode of `taxman solve` that searches for a high score and skips the proof, for the games past
  5060; its lines are lower bounds, kept apart from the proven ones ([the reference](REFERENCE.md#an-unsound-line)).
  Its first form is built, with a level of effort that is a number of rounds; the next step measures, on the games
  3441-5060, the rounds that each game needs to reach its proven score.
