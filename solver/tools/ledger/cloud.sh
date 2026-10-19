#!/usr/bin/env bash
# The ledger's experiment on one machine: `ledger run` over a stretch of boards for a CPU budget, with the truth hidden
# from it (no --proven), then `ledger report` against the committed chain, which checks every bound.
#
#   solver/tools/ledger/cloud.sh FIRST-LAST   from the top of the repository, once solver/tools/cloud/setup.sh has the
#                                             release `ledger` example
#                                             (built there, or taken from the build cache: RUNBOOK section 11; set
#                                             BUILD="--bin taxman --example ledger" for setup.sh: the solver is there
#                                             for its smoke checks)
#
# Environment (all optional):
#   WORKERS     searches at once, one thread each (default: every CPU)
#   CPU_HOURS   the budget, in hours of CPU (default: WORKERS, an hour of every worker)
#   BASE        the first search's limit of a board, in seconds (default 2); MAX_LIMIT the most (default 3600)
#   CLIMB       the ways of the climbs, the searches of a board next to a proven one: up, down, both or off (the
#               program's default, both)
#   CLIMB_LIMIT the limit of a climb in seconds (the program's default, 60; 0: no climbs)
#   LNS_SECONDS the seconds of one thread the finder (large neighbourhood search) gets after a board's map and before a
#               search whose best seed has changed (the program's default, 1; 0: no finder)
#   FLOOR_STEP  the points a floor step goes down below a board's upper bound, at the median slope of the cost of a search
#               under a floor (the program's default, 150; 0: no floor steps, the ordinary searches whose limit doubles)
#   FLOOR_SLACK a floor step's limit is this many times its forecast time (the program's default, 4)
#   PROVEN      a results file whose proven games the run is given (default none: the truth stays hidden)
#   BOUNDS      a bounds store (results/bounds.jsonl) whose best bounds the run starts from (default none); the run's
#               ledger is exported to it afterwards with `ledger export` (RUNBOOK section 6)
#   KNOWN       the results file the report checks against (default results/strict.jsonl)
#   LEDGER_BIN  the program (default solver/frame/target/release/examples/ledger; a laptop's test is
#               solver/frame/target/debug/examples/ledger, built by `cargo build --profile test --example ledger`
#               in solver/frame)
#   OUT         where everything goes (default ~/ledger): run/ (the ledger, its events and checkpoints), run.out and
#               run.err (what the run printed), report.txt, log (the times of the stages)
#   S3_DEST     if set, OUT is synced to S3_DEST/out every five minutes and at the end
#   POWEROFF=1  shut the machine down at the end
#
# The run goes on from OUT/run if it holds a ledger (after an interruption: the same command again).  The script exits
# with the report's status: 1 if the check found a bound on the wrong side of a known optimum or a game that does not
# replay, which is a bug to report.

set -u
cd "$(dirname "${BASH_SOURCE[0]}")/../../.." || exit 1
range="${1:?usage: solver/tools/ledger/cloud.sh FIRST-LAST}"
workers="${WORKERS:-$(getconf _NPROCESSORS_ONLN)}"
cpu_hours="${CPU_HOURS:-$workers}"
out="${OUT:-$HOME/ledger}"
known="${KNOWN:-results/strict.jsonl}"
ledger="${LEDGER_BIN:-solver/frame/target/release/examples/ledger}"
mkdir -p "$out"

stamp() { echo "$1 $(date -u +%FT%TZ)" >> "$out/log"; }
sync_out() { if [ -n "${S3_DEST:-}" ]; then aws s3 sync --quiet "$out" "$S3_DEST/out"; fi; }

[ -x "$ledger" ] || { echo "no $ledger: run solver/tools/cloud/setup.sh first" >&2; exit 2; }
( while sleep 300; do sync_out; done ) &
syncer=$!

stamp "start $range workers=$workers cpu_hours=$cpu_hours"
# shellcheck disable=SC2086 # an option and its value, or nothing
"$ledger" run "$range" --dir "$out/run" --workers "$workers" --cpu-hours "$cpu_hours" --base "${BASE:-2}" \
    --max-limit "${MAX_LIMIT:-3600}" ${CLIMB:+--climb "$CLIMB"} ${CLIMB_LIMIT:+--climb-limit "$CLIMB_LIMIT"} \
    ${LNS_SECONDS:+--lns-seconds "$LNS_SECONDS"} ${FLOOR_STEP:+--floor-step "$FLOOR_STEP"} \
    ${FLOOR_SLACK:+--floor-slack "$FLOOR_SLACK"} ${PROVEN:+--proven "$PROVEN"} ${BOUNDS:+--bounds "$BOUNDS"} \
    > "$out/run.out" 2> "$out/run.err"
stamp "run done, status $?"
"$ledger" report "$out/run" --known "$known" > "$out/report.txt" 2>&1
status=$?
stamp "report done, status $status"

kill "$syncer" 2> /dev/null
sync_out
if [ "${POWEROFF:-0}" = 1 ]; then sudo shutdown -h now; fi
exit "$status"
