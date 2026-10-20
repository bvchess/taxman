# Running the solver, and the prover's proofs, on a cloud machine

The runbook is the procedure for running the frame solver and the prover on an AWS machine, for Brian, the project's
owner, or an agent he has approved.  The target is a mid-size Graviton instance (`c8g`, `c9g` or `c7g`) running
Linux, for hours or days, and a smaller one for the prover's proofs ([prover.md](runbook/prover.md));
[Instance](runbook/machine.md#instance)
gives the sizes.  The scripts take the thread count from the machine, and they also work on x86_64, on Amazon Linux
2023 and on Ubuntu.  [The frame solver's README](../frame/README.md) says what the solver is, and [the
reference](REFERENCE.md) says how to read its output.

## The files

Every launch follows [launch.md](runbook/launch.md), which has the rules for every launch: read it before you start
anything.  Each job has a file of its own that says what the job computes, what it costs, its command, and only what
differs from launch.md.  The old section numbers are given for the citations that still use them.

| old section | file | read it to |
|---|---|---|
| Rules for every launch; 2. Every launch, step by step; 10. When something goes wrong | [launch.md](runbook/launch.md) | launch a machine, keep its cost in bounds, bring the results home, and handle a failure |
| 1. Choosing the machine | [machine.md](runbook/machine.md) | choose the instance, spot or on demand, and the disk, and check the quota |
| 3. Before a big run | [before-a-big-run.md](runbook/before-a-big-run.md) | check a long run cheaply first: in Docker, on a small instance, then on the production instance |
| 4. The production chain | [chain.md](runbook/chain.md) | run the chain of optimal games, watch it, verify it and commit it |
| 5. The unattended run to the wall | [wall.md](runbook/wall.md) | run the chain with no one watching, until it reaches [the wall](../../docs/GLOSSARY.md#the-search) |
| 6. The ledger's experiment | [ledger.md](runbook/ledger.md) | run the ledger over a stretch of games, with the truth hidden from it |
| 7. The sweep | [sweep.md](runbook/sweep.md) | prove as many games of a long stretch as a cheap machine allows |
| 8. The prover's proofs of opt(n) | [prover.md](runbook/prover.md) | prove opt(n) on the whole board with the prover |
| 9. Certifying U (stage 4) | [certify.md](runbook/certify.md) | certify statement U game by game |
| 11. The build cache and the release suite | [build-cache.md](runbook/build-cache.md) | build the release binaries once, test them, and share them between machines |
| 12. The scripts, and what has run where | [scripts.md](runbook/scripts.md) | find a script, or what has already run on which machine |
