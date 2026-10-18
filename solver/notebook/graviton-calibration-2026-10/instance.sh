#!/usr/bin/env bash
# The calibration of one cloud instance, in order (all but the chain): instance.sh OUTDIR
#
# The short one-thread runs come first, so that the per-core speed is known early; the two long proofs (2170 and 2172,
# a quarter of an hour on one thread each) run together, two cores of the machine's 64.  The chain
# (calibrate.sh chain) is run afterwards, by hand, on the instance with the fastest cores.

set -eu
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="$1"
cal="$here/calibrate.sh"

"$cal" probe "$out"
"$cal" ladder "$out" 1215 1232 1287
"$cal" prover "$out" 1300 990
"$cal" contention-solver "$out" 1 16 32 64
"$cal" contention-prover "$out" 1 64
"$cal" ladder "$out" 2170 &
"$cal" ladder "$out" 2172 &
wait
"$cal" scaling "$out" 4 16 64
echo "$(date -u +%Y-%m-%dT%H:%M:%SZ) instance.sh finished" > "$out/FINISHED"
