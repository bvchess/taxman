# Status

Updated 2026-11-06.

The state of the work as of the date above: the priority, and the projects in flight.  This page is rewritten, not
appended to, when the state changes.  What was done and why is in the [lab
notebook](https://github.com/bvchess/taxman-notebook/blob/main/README.md), how far the chain reaches in
[results/README.md](../../results/README.md), and the research avenues in the
[agenda](https://github.com/bvchess/taxman-notebook/blob/main/agenda/AGENDA.md).

## The priority

The Nov 1 run finished on 2026-11-06 at 05:00 PST, and its file is `results/optimal.jsonl.gz`: the games 1 to 5060, each
proven, by one build ([results/runs.md](../../results/runs.md)).  The cleanup of the frame solver is finished; its last
step retired the bounds store, the ledger, `--use-prev`, the readers of older line formats, `certify`, the certificates
and the extended mode.  The priority is the prover's proof of the games 2 to 2530 by one build, and after it the unsound
mode.

## In flight

* **The prover's run of 2 to 2530.**  It runs on batmobile, from a clone at commit 9015f79, on every thread at idle
  priority ([the runbook](runbook/prover.md#on-a-machine-of-your-own) says how).  It started by itself on 2026-11-06 at
  05:02 PST, when the chain ended.  When it ends, its output is checked against the results file (`prover.sh --verify 2
  2530`) and committed under `solver/noframe/data/`, where it supersedes the older `exact_parity` files ([the
  runbook](runbook/prover.md#bring-the-data-home-verify-commit) says how).

## Open

* **The fixed gate.**  Why the gate of 300 lost on every game searched from 4984 to 5022, at 2.0 times the nodes of the
  earlier file's searches (a gate of 1200, or conflict branching switched off), and won on every one from 5024 to 5060
  against the same gate of 1200 and build; and whether a gate scaled to the game's root gap, never tested, does better
  ([the full run at gate
  300](https://github.com/bvchess/taxman-notebook/blob/main/search/the-full-run-at-gate-300-2026-11/README.md)).
* **The unsound mode.**  A mode of `taxman solve` that searches for a high score and skips the proof, for the games past
  5060; its lines would be lower bounds, kept apart from the proven ones.  It is not built.
