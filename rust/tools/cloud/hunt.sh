#!/usr/bin/env bash
# The counterexample hunt over the open sources of the prover's `upper` file, on all the CPUs of a machine, resumable.
#
#   rust/tools/cloud/hunt.sh UPPER_FILE      hunt the games of UPPER_FILE (rust/prover/data/upper_*.jsonl)
#
# It runs the example `hunt` (rust/examples/hunt.rs, which has the method and the line format): for the games with the
# largest gap, and for each of their open sources u, a search of the board without u for a game that scores more than
# the recorded optimum.  A hit is a counterexample to that optimum.  Pairs are searched JOBS at a time, each on
# THREADS_PER threads, for MINUTES at most.
#
# Safe to run again at any time, after a crash, a reboot, a spot interruption or a SIGTERM: the hunt appends one line
# per finished pair to the output and skips the pairs the output already holds.  It has no stop handler, so a stop or
# a kill loses only the pairs in progress (at most MINUTES of each job).  A second hunt.sh is refused.
#
# What it does with the hunt's exit status:
#   0        every pair has a line: the script ends `complete` (and exits 0)
#   2        a COUNTEREXAMPLE: a game that scores more than the recorded optimum.  The found lines of the output are
#            logged loudly; nothing is retried.  Check it first (it was replayed on the full board, but read it)
#   130/143  stopped by SIGINT/SIGTERM: the script ends
#   124      the deadline (DEADLINE_HOURS) stopped it: the script ends
#   1        an error: the arguments or files, a hunt that failed, or a found game that does not replay on the full
#            board.  Nothing is retried; the last lines of hunt.err are shown
#   other    a crash (the OOM killer): run again after 10 s, MAX_FAILS in a row with no new line end the script
#
# The script exits 0 when complete, 2 for a counterexample, 143 when stopped, 3 for the deadline, 1 otherwise.
# A verdict of "none" is evidence, not a proof (rust/examples/hunt.rs says what each verdict is worth), and "timeout"
# says nothing.
#
# Output: $OUT (default $TAXMAN_WORK/results/hunt.jsonl, one line per pair), logs in $LOGS: hunt.log (this script),
# hunt.out (the hunt's summary, written when it ends), hunt.err (its progress, a line per pair).
#
# Environment (all optional):
#   GAMES           how many games, those with the largest gap, default 20
#   PER_GAME        how many open sources of each, those of largest margin; default all
#   MINUTES         the time limit of each pair's search, default 10
#   THREADS_PER     the engine threads of each pair, default 4
#   JOBS            pairs at once, default the number of CPUs divided by THREADS_PER (at least 1)
#   OUT             the output file
#   HUNT_BIN        the hunt example, default rust/target/release/examples/hunt (built by setup.sh)
#   EXTRA_ARGS      more flags for the hunt (for instance --list 3311:1771,3311:1749)
#   MAX_FAILS       default 5
#   DEADLINE_HOURS, POWEROFF, S3_DEST, SAVE_EVERY: as in run.sh (POWEROFF=done powers off only when complete)

set -u
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"

[ "$#" -eq 1 ] || die "usage: hunt.sh UPPER_FILE   (the prover's upper file, for instance" \
    "rust/prover/data/upper_2176_3314.jsonl)"
UPPER="$1"
[ -r "$UPPER" ] || die "$UPPER is not readable"
[ -x "$HUNT_BIN" ] || die "$HUNT_BIN is missing: run setup.sh"

cpus="$(ncpu)"
GAMES="${GAMES:-20}"
MINUTES="${MINUTES:-10}"
THREADS_PER="${THREADS_PER:-4}"
JOBS="${JOBS:-$((cpus / THREADS_PER > 0 ? cpus / THREADS_PER : 1))}"
OUT="${OUT:-$TAXMAN_WORK/results/hunt.jsonl}"
MAX_FAILS="${MAX_FAILS:-5}"
case "$GAMES$THREADS_PER$JOBS$MAX_FAILS${PER_GAME:-0}" in
    "" | *[!0-9]*) die "GAMES, PER_GAME, THREADS_PER, JOBS and MAX_FAILS are whole numbers" ;;
esac

args=(--upper "$UPPER" --games "$GAMES" --minutes "$MINUTES" --threads "$THREADS_PER" --jobs "$JOBS" --out "$OUT")
if [ -n "${PER_GAME:-}" ]; then
    args+=(--per-game "$PER_GAME")
fi
read -r -a extra <<<"${EXTRA_ARGS:-}"

# shellcheck source=run.sh
. "$here/run.sh"
run_begin hunt "$(dirname "$OUT")"

# The pairs with a line in $OUT.
lines_done() {
    if [ -r "$OUT" ]; then wc -l <"$OUT" | tr -d ' '; else echo 0; fi
}

log "hunt $UPPER: $GAMES games, ${PER_GAME:-all} sources each, $MINUTES minutes each;" \
    "$JOBS jobs of $THREADS_PER threads; output $OUT"
env_report | tee -a "$LOGS/hunt.log" >&2

fails=0
while :; do
    if [ -n "$stop_requested" ]; then
        outcome="stopped"
        break
    fi
    lines_before="$(lines_done)"
    log "hunt: $lines_before pairs have a line in $OUT"
    run_child "$LOGS/hunt.out" "$LOGS/hunt.err" "$HUNT_BIN" "${args[@]}" ${extra[@]+"${extra[@]}"}
    log "hunt exited with status $status"

    case "$status" in
        0)
            log "every pair has a line"
            outcome="complete"
            break
            ;;
        2)
            log "COUNTEREXAMPLE: a game scores more than the recorded optimum.  Not retrying.  The found lines of $OUT:"
            grep -F '"verdict":"found"' "$OUT" | cut -c1-400 | while IFS= read -r line; do
                log "COUNTEREXAMPLE: $line"
            done
            outcome="found"
            break
            ;;
        124)
            outcome="deadline"
            break
            ;;
        130 | 143)
            log "stopped by a signal (the pairs in progress are lost; the finished ones are in $OUT)"
            outcome="stopped"
            break
            ;;
        1)
            log "the hunt reported an error; not retrying.  The last lines of hunt.err:"
            tail -n 8 "$LOGS/hunt.err" >&2
            outcome="failed"
            break
            ;;
        *)
            log "the hunt crashed (status $status)"
            if [ "$(lines_done)" -gt "$lines_before" ]; then fails=0; else fails=$((fails + 1)); fi
            if [ "$fails" -ge "$MAX_FAILS" ]; then
                log "giving up after $fails runs in a row without a new line"
                outcome="failed"
                break
            fi
            run_nap 10
            ;;
    esac
done

log "ended: $outcome"
case "$outcome" in
    complete) exit 0 ;;
    found) exit 2 ;;
    stopped) exit 143 ;;
    deadline) exit 3 ;;
    *) exit 1 ;;
esac
