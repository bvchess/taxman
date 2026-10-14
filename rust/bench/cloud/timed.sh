#!/usr/bin/env bash
# One measured run: timed.sh OUTDIR LABEL COMMAND [ARG...]
#
# Runs COMMAND with its output in OUTDIR/LABEL.out, the resource report of /usr/bin/time (GNU time -v on Linux, BSD
# time -l on macOS) in OUTDIR/LABEL.time and its exit status in OUTDIR/LABEL.status.  parse.py reads the three.

set -u
out="$1"
label="$2"
shift 2
mkdir -p "$out"
if [ "$(uname -s)" = Darwin ]; then flag=-l; else flag=-v; fi
/usr/bin/time "$flag" -o "$out/$label.time" "$@" > "$out/$label.out" 2>&1
echo $? > "$out/$label.status"
