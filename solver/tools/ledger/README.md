# The ledger's scripts

The program is the example `ledger` of the frame solver (`solver/frame/examples/ledger/`; its module comment defines
every field of `ledger.jsonl` and `events.jsonl`).  What was run with it and what it showed is in [the
entry](../../notebook/search/the-ledger-bounds-from-both-sides-2026-10/README.md), which also lists the data of every
run.  These two scripts launch a run and read it.

| file | command | what it is |
|---|---|---|
| `cloud.sh` | `solver/tools/ledger/cloud.sh FIRST-LAST` from the top of the repository (environment in its comment) | the experiment on one machine: `ledger run` over the stretch for a CPU budget with the truth hidden, then `ledger report --known results/strict.jsonl`, which checks every bound; the cloud run of 2176..3440 is [solver/docs/runbook/ledger.md](../../docs/runbook/ledger.md) |
| `events.py` | `solver/tools/ledger/events.py DIR [CHAIN.jsonl]` | what a run spent its time on, beyond the report: setup, maps and searches, stopped searches and their dual bounds, the finder's runs and the games that seeded the searches, the searches by whether n − 1 and n + 1 were proven when they began, the relations' changes by rule, the main thread's busy time, the CPU over the run's wall time; with the chain's results file over the same stretch, its seconds beside |

`cloud.sh` needs the release `ledger` example, which `solver/tools/cloud/setup.sh` builds (or takes from the build
cache: [solver/docs/runbook/build-cache.md](../../docs/runbook/build-cache.md)); on a laptop,
`LEDGER_BIN=solver/frame/target/debug/examples/ledger` names the non-release build that
`cargo build --profile test --example ledger --bin taxman` makes in `solver/frame` (opt-level 2, debug assertions on).
