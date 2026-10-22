# Running the solver, and the prover's proofs, on a cloud machine

The runbook is the procedure for running the frame solver and the prover on an AWS machine.  The target is a mid-size
Graviton instance (`c8g`, `c9g` or `c7g`) running Linux, for hours or days, and a smaller one for the prover's proofs
([prover.md](runbook/prover.md)); [Instance](runbook/machine.md#instance) gives the sizes.  The scripts take the thread
count from the machine, and they also work on x86_64, on Amazon Linux 2023 and on Ubuntu.  [The frame solver's
README](../frame/README.md) says what the solver is, and [the reference](REFERENCE.md) says how to read its output.

## The files

Every launch follows [launch.md](runbook/launch.md), which has the rules for every launch: read it before you start
anything.  The general advice:

| topic | file | read it to |
|---|---|---|
| every launch | [launch.md](runbook/launch.md) | follow the rules, launch a machine, keep its cost in bounds, bring the results home, and handle a failure |
| the machine | [machine.md](runbook/machine.md) | choose the instance, spot or on demand, and the disk, and check the quota |
| a big run | [before-a-big-run.md](runbook/before-a-big-run.md) | check a long run cheaply first: in Docker, on a small instance, then on the big one |
| the build cache | [build-cache.md](runbook/build-cache.md) | build the release binaries once, test them, and share them between machines |
| the scripts | [scripts.md](runbook/scripts.md) | find a script, and where it has run |

The jobs, a file a script.  Each says what its script does, what it costs, its commands, and only what differs from
launch.md, and ends with the campaigns that used it, whose notebook entries hold their specifics:

| topic | file | read it to |
|---|---|---|
| the chain (`chain.sh`) | [chain.md](runbook/chain.md) | run the chain of optimal games, watch it, verify it and commit it |
| the run to the wall (`wall.sh`) | [wall.md](runbook/wall.md) | run the chain with no one watching, until it reaches [the wall](../../docs/GLOSSARY.md#the-search) |
| the sweep (`sweep.sh`) | [sweep.md](runbook/sweep.md) | prove as many games of a long stretch as a cheap machine allows |
| the ledger (`ledger/cloud.sh`) | [ledger.md](runbook/ledger.md) | run the ledger over a stretch of games for a budget of CPU |
| the prover (`prover.sh`) | [prover.md](runbook/prover.md) | prove opt(n) on the whole board with the prover |
| certificates of U (`certify.sh`) | [certify.md](runbook/certify.md) | certify statement U game by game |
| the lab (`lab/lab.sh`) | [lab.md](runbook/lab.md) | study the largest units of a live search on a machine's spare threads |
