#!/usr/bin/env bash
# The production run: the chain of optimal games from n = 1 to N, resumable, with logging.
#
#   rust/tools/cloud/chain.sh N
#
# It runs
#
#   taxman solve 1-N [--threads $THREADS] \
#       --out $RESULTS --checkpoint-dir $CKPT --checkpoint-interval 60 --progress 60
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
#   CHECKPOINT_INTERVAL  seconds, default 60        PROGRESS  seconds between progress lines, default 60
#   EXTRA_ARGS           more flags for `taxman solve`, e.g. "--mem-cap 100000"
#   DEADLINE_HOURS       stop the solver (cleanly) this many hours after the FIRST start.  The deadline is kept in
#                        $TAXMAN_WORK/deadline, so a restart does not extend it; delete the file to change it.
#   POWEROFF             never (default) | done (when the range is complete) | always (whenever this script ends):
#                        run `sudo shutdown -h now`, so that a forgotten instance stops billing
#   MAX_FAILS            default 5
#   S3_DEST              s3://bucket/prefix: copy results, checkpoints and logs there every SAVE_EVERY seconds
#                        (default 600) and when the script ends; see save.sh
#   CHAIN_TO             N, if not given as the argument

set -u
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"

to="${1:-${CHAIN_TO:-}}"
case "$to" in
    '' | *[!0-9]*) die "usage: chain.sh N   (the last board size; the chain starts at 1)" ;;
esac
[ "$to" -ge 1 ] || die "N must be at least 1"
[ -x "$TAXMAN_BIN" ] || die "$TAXMAN_BIN is missing: run setup.sh"

# Without THREADS the solver uses every CPU it is allowed (in a container, the cgroup quota).
thread_args=()
[ -n "${THREADS:-}" ] && thread_args=(--threads "$THREADS")
CHECKPOINT_INTERVAL="${CHECKPOINT_INTERVAL:-60}"
PROGRESS="${PROGRESS:-60}"
MAX_FAILS="${MAX_FAILS:-5}"
SAVE_EVERY="${SAVE_EVERY:-600}"
POWEROFF="${POWEROFF:-never}"
case "$POWEROFF" in never | done | always) ;; *) die "POWEROFF must be never, done or always" ;; esac

mkdir -p "$(dirname "$RESULTS")" "$CKPT" "$LOGS"

# This script's messages go to the terminal and to chain.log, written directly (not through a tee, which would die
# with a Ctrl-C aimed at the whole foreground group and take the log with it).
log() {
    local line
    line="$(printf '%s %s' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*")"
    printf '%s\n' "$line" >>"$LOGS/chain.log"
    printf '%s\n' "$line" >&2
}

# Job control: the solver runs in its own process group, so a Ctrl-C, or a signal sent to the whole group, reaches
# only this script, which forwards ONE SIGTERM.  (A second stop signal makes the solver exit at once, without its
# final checkpoint.)
set -m

# Checkpoints and results must survive a stop: not tmpfs.
for dir in "$CKPT" "$(dirname "$RESULTS")"; do
    case "$(df -T "$dir" 2>/dev/null | awk 'NR == 2 {print $2}')" in
        tmpfs | ramfs) die "$dir is on a memory filesystem: it would vanish with the instance (set TAXMAN_WORK)" ;;
    esac
done

# One chain.sh at a time (the solver's own lock on the checkpoint directory would also refuse a second solver).
if command -v flock >/dev/null 2>&1; then
    exec 9>"$TAXMAN_WORK/chain.lock"
    flock -n 9 || die "another chain.sh is running (it holds $TAXMAN_WORK/chain.lock)"
else
    log "warning: no flock(1); relying on the checkpoint directory lock"
fi
echo $$ >"$TAXMAN_WORK/chain.pid"

# The deadline: fixed at the first start, in seconds since the epoch.
deadline=""
deadline_file="$TAXMAN_WORK/deadline"
if [ -n "${DEADLINE_HOURS:-}" ] && [ ! -e "$deadline_file" ]; then
    awk -v now="$(date +%s)" -v hours="$DEADLINE_HOURS" 'BEGIN {printf "%d\n", now + hours * 3600}' >"$deadline_file"
fi
[ -r "$deadline_file" ] && deadline="$(cat "$deadline_file")"
if [ -n "$deadline" ] && ! command -v timeout >/dev/null 2>&1; then
    log "warning: no timeout(1): the deadline cannot be enforced"
    deadline=""
fi

saver=""
child=""
stop_requested=""
outcome="failed"

cleanup() {
    if [ -n "$saver" ]; then
        pkill -P "$saver" 2>/dev/null
        kill "$saver" 2>/dev/null
    fi
    [ -n "${S3_DEST:-}" ] && "$here/save.sh" >>"$LOGS/save.log" 2>&1
    rm -f "$TAXMAN_WORK/chain.pid"
    if [ "$POWEROFF" = always ] || { [ "$POWEROFF" = done ] && [ "$outcome" = complete ]; }; then
        log "powering off (POWEROFF=$POWEROFF, outcome $outcome)"
        sudo -n shutdown -h now || log "could not shut down: $?"
    fi
}
trap cleanup EXIT

# SIGTERM and SIGINT go to the solver, which writes a final checkpoint and exits with 128 + the signal.
on_signal() {
    stop_requested=1
    log "signal received: asking the solver to stop"
    [ -n "$child" ] && kill -TERM "$child" 2>/dev/null
}
trap on_signal TERM INT HUP

# Sleep that a signal interrupts at once.
nap() {
    sleep "$1" &
    wait $!
}

# The lines of the results file (0 if there is none yet).
line_count() {
    if [ -r "$RESULTS" ]; then wc -l <"$RESULTS" | tr -d ' '; else echo 0; fi
}

log "chain 1-$to; threads: ${THREADS:-all available}; results $RESULTS; checkpoints $CKPT"
env_report | tee -a "$LOGS/chain.log" >&2

if [ -n "${S3_DEST:-}" ]; then
    (
        while sleep "$SAVE_EVERY"; do
            "$here/save.sh" >>"$LOGS/save.log" 2>&1
        done
    ) &
    saver=$!
    log "saving to $S3_DEST every ${SAVE_EVERY}s"
fi

fails=0
while :; do
    if [ -n "$stop_requested" ]; then
        outcome="stopped"
        break
    fi
    wrap=()
    if [ -n "$deadline" ]; then
        left=$((deadline - $(date +%s)))
        if [ "$left" -le 0 ]; then
            log "the deadline has passed"
            outcome="deadline"
            break
        fi
        wrap=(timeout --signal=TERM --kill-after=120 "$left")
    fi

    started="$(date +%s)"
    lines_before="$(line_count)"
    games="$([ -r "$RESULTS" ] && grep -c '"completed":true' "$RESULTS" || true)"
    log "starting the solver (games done so far: ${games:-0})"
    # EXTRA_ARGS is a list of words on purpose.
    # shellcheck disable=SC2086
    ${wrap[@]+"${wrap[@]}"} "$TAXMAN_BIN" solve "1-$to" ${thread_args[@]+"${thread_args[@]}"} --out "$RESULTS" \
        --checkpoint-dir "$CKPT" --checkpoint-interval "$CHECKPOINT_INTERVAL" --progress "$PROGRESS" ${EXTRA_ARGS:-} \
        </dev/null >>"$LOGS/chain.out" 2>>"$LOGS/chain.err" &
    child=$!
    # A trapped signal ends `wait` early, while the solver is still stopping: wait again until it is gone.
    while :; do
        wait "$child"
        status=$?
        kill -0 "$child" 2>/dev/null || break
    done
    child=""
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
    [ "$crashed" = 1 ] && nap 10
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
