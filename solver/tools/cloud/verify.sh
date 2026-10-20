#!/usr/bin/env bash
# Verify a results file with both referees, and against the results committed with the code.
#
#   solver/tools/cloud/verify.sh [FILE...]        default: the production file ($RESULTS)
#
# 1. `taxman verify-results`: every game replayed, the chain from n-1, each line's proof and provenance, and that
#    the files agree wherever they share a game (results/README.md gives the range the committed strict.jsonl
#    covers).
# 2. solver/tools/check_results.py: the same, written independently in Python, plus the Franklin-Moniot bound and the best
#    heuristic chain (heuristics/results, to n = 2000).
#
# Safe on a file that a run is appending to (a half-written last line is dropped as torn).  The exit status is
# non-zero if either referee finds a problem.

set -u
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"

files=("$@")
[ "${#files[@]}" -gt 0 ] || files=("$RESULTS")
for f in "${files[@]}"; do
    [ -r "$f" ] || die "$f is not readable"
done
[ -x "$TAXMAN_BIN" ] || die "$TAXMAN_BIN is missing: run setup.sh"

status=0
log "taxman verify-results"
"$TAXMAN_BIN" verify-results "${files[@]}" "${COMMITTED_RESULTS[@]}" || status=1
log "solver/tools/check_results.py"
python3 "$TOOLS/check_results.py" "${files[@]}" "${COMMITTED_RESULTS[@]}" || status=1
if [ "$status" = 0 ]; then
    log "both referees agree: no problem found"
else
    log "PROBLEMS FOUND (see above)"
fi
exit "$status"
