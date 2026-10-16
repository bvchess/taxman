#!/bin/bash
#
# One run of the measurement at the end of the chain: resumes the checkpoint of game 3441 (a copy) on THREADS threads
# for SECONDS seconds of search, as the chain does (its predecessors from a copy of results/strict.jsonl), under
# nice -n 10, and keeps what the solver printed.  With PROFILE_SECONDS the run is sampled by macOS `sample` (every millisecond)
# from 25 s into the search instead of being timed with /usr/bin/time -l.
#
#   TAXMAN_BENCH_SRC=DIR TAXMAN_BENCH_OUT=DIR bench/chain_end/run.sh THREADS SECONDS TAG [PROFILE_SECONDS]
#
# SECONDS 0 stops after the first batch of nodes (--node-limit 1): the setup alone, whose cycles and CPU time the
# timed runs' are measured net of.  The search's clock starts when the workers do, after the setup (about 40 s on
# one thread, 11 s on eight: the start incumbent's local search is parallel).
#
# DIR of TAXMAN_BENCH_SRC holds n3441.ckpt (the cloud run's checkpoint) and strict.jsonl (games 1..3440); neither is
# modified.  Writes TAXMAN_BENCH_OUT/TAG/: ckpt/ and out.jsonl (the copies the run uses), progress.txt (the solver's
# stderr), stdout.txt, time.txt (/usr/bin/time -l) or sample.txt, and the machine's load before and after.
# Before the run it waits, a minute at a time, for the one-minute load average to fall under MAX_LOAD (default 6).
# From rust/, after cargo build --release.
set -euo pipefail
if [ $# -lt 3 ] || [ $# -gt 4 ]; then
    echo "usage: $0 THREADS SECONDS TAG [PROFILE_SECONDS]" >&2
    exit 2
fi
THREADS=$1; SECS=$2; TAG=$3; PROFILE=${4:-0}
CRATE=$(cd "$(dirname "$0")/../.." && pwd)
BIN=$CRATE/target/release/taxman
SRC=${TAXMAN_BENCH_SRC:?set TAXMAN_BENCH_SRC}
DIR=${TAXMAN_BENCH_OUT:?set TAXMAN_BENCH_OUT}/$TAG
MAX_LOAD=${MAX_LOAD:-6}

load() { sysctl -n vm.loadavg | awk '{print $2}'; }
for _ in $(seq 20); do
    if awk -v l="$(load)" -v m="$MAX_LOAD" 'BEGIN { exit !(l < m) }'; then break; fi
    sleep 60
done
awk -v l="$(load)" -v m="$MAX_LOAD" 'BEGIN { exit !(l < m) }' || { echo "load stayed above $MAX_LOAD" >&2; exit 3; }

rm -rf "$DIR"
mkdir -p "$DIR/ckpt"
cp "$SRC/n3441.ckpt" "$DIR/ckpt/"
cp "$SRC/strict.jsonl" "$DIR/out.jsonl"
echo "$(date -u +%FT%TZ) load $(sysctl -n vm.loadavg)" > "$DIR/load_before.txt"
LIMIT=(--time-limit "$SECS")
if [ "$SECS" -eq 0 ]; then
    LIMIT=(--node-limit 1)  # the setup alone (the checkpoint's load, the frames, the start incumbent), to subtract
fi
SOLVE=("$BIN" solve 3441 --threads "$THREADS" --out "$DIR/out.jsonl" --checkpoint-dir "$DIR/ckpt"
    --checkpoint-interval 300 --progress 10 "${LIMIT[@]}")
if [ "$PROFILE" -gt 0 ]; then
    nice -n 10 "${SOLVE[@]}" > "$DIR/stdout.txt" 2> "$DIR/progress.txt" &
    PID=$!
    sleep 25
    /usr/bin/sample "$PID" "$PROFILE" 1 -file "$DIR/sample.txt" > /dev/null 2>&1 || true
    wait "$PID" || true
else
    /usr/bin/time -l -o "$DIR/time.txt" nice -n 10 "${SOLVE[@]}" > "$DIR/stdout.txt" 2> "$DIR/progress.txt" || true
fi
echo "$(date -u +%FT%TZ) load $(sysctl -n vm.loadavg)" > "$DIR/load_after.txt"
tail -3 "$DIR/progress.txt"
