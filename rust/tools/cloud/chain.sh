#!/usr/bin/env bash
# The production run: the chain of optimal games from n = 1 to N, resumable, with logging.
#
#   rust/tools/cloud/chain.sh N
#
# It runs
#
#   taxman solve 1-N [--threads $THREADS] --window $WINDOW \
#       --out $RESULTS --checkpoint-dir $CKPT --checkpoint-interval 300 --progress 60
#
# in a loop.  From n = 1, so that no proof rests on anything beyond the frame model.  Safe to run again at any time, after
# a crash, a reboot, a spot interruption or a SIGTERM: the solver skips the games the results file has completed and
# resumes the game whose checkpoint it finds, losing at most one checkpoint interval.  A second chain.sh while one
# runs is refused.
#
# What it does with the solver's exit status:
#   0        the range is done (checked with `taxman verify-results`): verify.sh runs and the script ends.  If the
#            range is not done (a --time-limit in EXTRA_ARGS stopped a game), the solver is run again at once; that
#            is how to rehearse many interruptions and resumes (RUNBOOK.md)
#   130/143  stopped by SIGINT/SIGTERM (this script forwards them, once, to the solver, which writes a final
#            checkpoint): the script ends
#   124      the deadline (DEADLINE_HOURS) stopped the solver, with a final checkpoint: the script ends
#   1, 2     the solver reported an error (a refused checkpoint, a game that failed a check, ...): the script ends,
#            nothing is retried, because the error would repeat; read chain.err
#   other    a crash (abort, SIGKILL by the OOM killer): run again after 10 s.  MAX_FAILS passes in a row that add
#            no line to the results file and last under five minutes end the script
#
# Output: $LOGS/chain.out (the solver's result lines), chain.err (its progress lines), chain.log (this script).
#
# Environment (all optional):
#   THREADS              worker threads, default all CPUs the solver may use
#   WINDOW               the games in flight at once, default 16; 1 solves them one after the other; see
#                        `taxman solve --window`
#   CHECKPOINT_INTERVAL  seconds, default 300       PROGRESS  seconds between progress lines, default 60
#                        (a checkpoint stops every worker for the slowest one's step, up to 0.4 s near n = 4000: 0.15%
#                        of the machine at 300 s, 0.7% at 60 s; SIGTERM still writes a final one)
#   EXTRA_ARGS           more flags for `taxman solve`, e.g. "--mem-cap 100000"
#   MAX_FAILS            default 5
#   CHAIN_TO             N, if not given as the argument
#   DEADLINE_HOURS, POWEROFF, S3_DEST, SAVE_EVERY: as in run.sh, which this script and prover.sh share

set -u
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"
# shellcheck source=run.sh
. "$here/run.sh"

to="${1:-${CHAIN_TO:-}}"
case "$to" in
    '' | *[!0-9]*) die "usage: chain.sh N   (the last board size; the chain starts at 1)" ;;
esac
[ "$to" -ge 1 ] || die "N must be at least 1"
[ -x "$TAXMAN_BIN" ] || die "$TAXMAN_BIN is missing: run setup.sh"

# Without THREADS the solver uses every CPU it is allowed (in a container, the cgroup quota).
thread_args=()
[ -n "${THREADS:-}" ] && thread_args=(--threads "$THREADS")
WINDOW="${WINDOW:-16}"
CHECKPOINT_INTERVAL="${CHECKPOINT_INTERVAL:-300}"
PROGRESS="${PROGRESS:-60}"
MAX_FAILS="${MAX_FAILS:-5}"

run_begin chain "$(dirname "$RESULTS")"

# The lines of the results file (0 if there is none yet).
line_count() {
    if [ -r "$RESULTS" ]; then wc -l <"$RESULTS" | tr -d ' '; else echo 0; fi
}

log "chain 1-$to; threads: ${THREADS:-all available}; window $WINDOW; results $RESULTS; checkpoints $CKPT"
env_report | tee -a "$LOGS/chain.log" >&2

fails=0
while :; do
    if [ -n "$stop_requested" ]; then
        outcome="stopped"
        break
    fi

    started="$(date +%s)"
    lines_before="$(line_count)"
    games="$([ -r "$RESULTS" ] && grep -c '"completed":true' "$RESULTS" || true)"
    log "starting the solver (games done so far: ${games:-0})"
    # EXTRA_ARGS is a list of words on purpose.
    # shellcheck disable=SC2086
    run_child "$LOGS/chain.out" "$LOGS/chain.err" "$TAXMAN_BIN" solve "1-$to" ${thread_args[@]+"${thread_args[@]}"} \
        --window "$WINDOW" --out "$RESULTS" --checkpoint-dir "$CKPT" --checkpoint-interval "$CHECKPOINT_INTERVAL" \
        --progress "$PROGRESS" ${EXTRA_ARGS:-}
    elapsed=$(($(date +%s) - started))
    log "the solver exited with status $status after ${elapsed}s"

    crashed=0
    case "$status" in
        0)
            report="$("$TAXMAN_BIN" verify-results "$RESULTS" 2>&1)" || log "verify-results failed: $report"
            if printf '%s\n' "$report" | grep -q "n = 1\.\.=$to (0 missing)"; then
                log "the range 1-$to is complete"
                outcome="complete"
                break
            fi
            log "the solver ended before $to (a limit in EXTRA_ARGS?); going on"
            ;;
        124)
            log "the deadline stopped the solver (a final checkpoint was written)"
            outcome="deadline"
            break
            ;;
        130 | 143)
            log "the solver was stopped by a signal (a final checkpoint was written)"
            outcome="stopped"
            break
            ;;
        1 | 2)
            log "the solver reported an error; not retrying.  The last lines of chain.err:"
            tail -n 5 "$LOGS/chain.err" >&2
            outcome="failed"
            break
            ;;
        *)
            log "the solver crashed (status $status)"
            crashed=1
            ;;
    esac

    # A pass that added a line to the results file (a game, or a search stopped at a limit) or ran for five minutes
    # made progress; MAX_FAILS passes in a row without it end the script.
    if [ "$(line_count)" -gt "$lines_before" ] || [ "$elapsed" -ge 300 ]; then fails=0; else fails=$((fails + 1)); fi
    if [ "$fails" -ge "$MAX_FAILS" ]; then
        log "giving up after $fails passes in a row without progress"
        outcome="failed"
        break
    fi
    [ "$crashed" = 1 ] && run_nap 10
done

log "ended: $outcome"
if [ "$outcome" = complete ]; then
    "$here/verify.sh" "$RESULTS" 2>&1 | tee -a "$LOGS/verify.log" || outcome="failed"
fi
case "$outcome" in
    complete) exit 0 ;;
    stopped) exit 143 ;;
    deadline) exit 3 ;;
    *) exit 1 ;;
esac
