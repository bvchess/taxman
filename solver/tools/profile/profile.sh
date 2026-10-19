#!/bin/bash
#
# Where the time of a hard game goes (section 2 of the entry
# solver/notebook/ideas-from-other-fields-the-pilots-2026-10/README.md): solves game N of the chain from 1 for SECONDS
# on THREADS threads, as the chain solves it (its predecessor from a copy of results/strict.jsonl cut before N), samples
# it with macOS `sample` every millisecond, and charges each sample to a kind of work and a purpose (attribute.py).
#
#   solver/tools/profile/profile.sh N SECONDS THREADS OUTDIR        (after cargo build --release in solver/frame)
#
# Writes OUTDIR/nN/: prev.jsonl (the copy, then the incomplete result line), ckpt/, solve.txt, progress.txt,
# sample.txt, and prints the attribution.  The committed results are only read.  macOS only (`sample`, `atos`).
set -euo pipefail
if [ $# -ne 4 ]; then
    echo "usage: $0 N SECONDS THREADS OUTDIR" >&2
    exit 2
fi
N=$1; SECS=$2; THREADS=$3; OUT=$4
HERE=$(cd "$(dirname "$0")" && pwd)
CRATE=$(cd "$HERE/../../frame" && pwd)
REPO=$(cd "$HERE/../../.." && pwd)
BIN=$CRATE/target/release/taxman
# The repository's virtual environment if there is one, else the system's python3 (attribute.py needs only the
# standard library).
PY=$(git -C "$CRATE" rev-parse --show-toplevel 2>/dev/null)/.venv/bin/python
[ -x "$PY" ] || PY=python3
DIR=$OUT/n$N
mkdir -p "$DIR"
rm -rf "$DIR/ckpt" "$DIR/prev.jsonl"
"$PY" -c "
import json, sys
with open(sys.argv[1]) as f, open(sys.argv[2], 'w') as g:
    for line in f:
        if json.loads(line)['n'] < int(sys.argv[3]):
            g.write(line)
" "$REPO/results/strict.jsonl" "$DIR/prev.jsonl" "$N"
"$BIN" solve "$N" --threads "$THREADS" --out "$DIR/prev.jsonl" --checkpoint-dir "$DIR/ckpt" \
    --time-limit "$SECS" --progress 30 > "$DIR/solve.txt" 2> "$DIR/progress.txt" &
PID=$!
sleep 5
/usr/bin/sample "$PID" $((SECS - 10)) 1 -file "$DIR/sample.txt" > /dev/null 2>&1 || true
wait "$PID" || true
tail -1 "$DIR/progress.txt"
"$PY" "$HERE/attribute.py" "$BIN" "$DIR/sample.txt"
