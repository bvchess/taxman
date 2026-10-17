#!/usr/bin/env bash
# The ledger's experiment on one machine: `ledger run` over a stretch of boards for a CPU budget, with the truth hidden
# from it (no --proven), then `ledger report` against the committed chain, which checks every bound.
#
#   bench/ledger/cloud.sh FIRST-LAST          from rust/, once tools/cloud/setup.sh has built the release examples
#
# Environment (all optional):
#   WORKERS     searches at once, one thread each (default: every CPU)
#   CPU_HOURS   the budget, in hours of CPU (default: WORKERS, an hour of every worker)
#   BASE        the first search's limit of a board, in seconds (default 2); MAX_LIMIT the most (default 3600)
#   PROVEN      a results file whose proven games the run is given (default none: the truth stays hidden)
#   KNOWN       the results file the report checks against (default results/strict.jsonl)
#   LEDGER_BIN  the program (default target/release/examples/ledger; a laptop's test is target/debug/examples/ledger,
#               built by `cargo build --profile test --example ledger`)
#   OUT         where everything goes (default ~/ledger): run/ (the ledger, its events and checkpoints), run.out and
#               run.err (what the run printed), report.txt, log (the times of the stages)
#   S3_DEST     if set, OUT is synced to S3_DEST/out every five minutes and at the end
#   POWEROFF=1  shut the machine down at the end
#
# The run goes on from OUT/run if it holds a ledger (after an interruption: the same command again).  The script exits
# with the report's status: 1 if the check found a bound on the wrong side of a known optimum or a game that does not
# replay, which is a bug to report.

set -u
cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1
range="${1:?usage: bench/ledger/cloud.sh FIRST-LAST}"
workers="${WORKERS:-$(getconf _NPROCESSORS_ONLN)}"
cpu_hours="${CPU_HOURS:-$workers}"
out="${OUT:-$HOME/ledger}"
known="${KNOWN:-results/strict.jsonl}"
ledger="${LEDGER_BIN:-target/release/examples/ledger}"
mkdir -p "$out"

stamp() { echo "$1 $(date -u +%FT%TZ)" >> "$out/log"; }
sync_out() { if [ -n "${S3_DEST:-}" ]; then aws s3 sync --quiet "$out" "$S3_DEST/out"; fi; }

[ -x "$ledger" ] || { echo "no $ledger: run tools/cloud/setup.sh first" >&2; exit 2; }
( while sleep 300; do sync_out; done ) &
syncer=$!

stamp "start $range workers=$workers cpu_hours=$cpu_hours"
# shellcheck disable=SC2086 # --proven and its file, or nothing
"$ledger" run "$range" --dir "$out/run" --workers "$workers" --cpu-hours "$cpu_hours" --base "${BASE:-2}" \
    --max-limit "${MAX_LIMIT:-3600}" ${PROVEN:+--proven "$PROVEN"} > "$out/run.out" 2> "$out/run.err"
stamp "run done, status $?"
"$ledger" report "$out/run" --known "$known" > "$out/report.txt" 2>&1
status=$?
stamp "report done, status $status"

kill "$syncer" 2> /dev/null
sync_out
if [ "${POWEROFF:-0}" = 1 ]; then sudo shutdown -h now; fi
exit "$status"
