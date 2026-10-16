#!/usr/bin/env bash
# What the long-run scripts (chain.sh, prover.sh, hunt.sh, certify.sh) share: one run at a time, a log, a pid file, a
# deadline, periodic copies to S3, a clean stop on a signal, and an optional power-off.  Source it after env.sh; do
# not run it.
#
#   run_begin NAME DIR...        NAME is the run's name (chain, prover, hunt, certify_PHASE); DIR... are directories
#                                 that must survive the instance (refused if on a memory filesystem).  Sets up
#                                 everything below.
#   run_child OUT ERR CMD...     run CMD in the background, its stdout and stderr appended to OUT and ERR, and wait
#                                 for it; its exit status is left in $status.  Under a deadline the command runs under
#                                 timeout(1): a stop by the deadline is status 124 (and so is a deadline already past,
#                                 without running anything).
#   run_start OUT ERR CMD...     the same, but do not wait: for several commands at once.  The pid is left in $launched
#                                 (empty if the deadline has passed: $status is 124); run_reap PID collects the command
#                                 once it has ended (status 0 and its exit status in $status), else returns 1.  A stop
#                                 signal is forwarded to every command started and not yet collected.
#   run_nap SECONDS              sleep that a signal interrupts at once
#   $stop_requested              non-empty once SIGTERM, SIGINT or SIGHUP has arrived
#   $outcome                     set it to complete when the work is done; cleanup acts on it (POWEROFF=done), and the
#                                 caller picks its exit status from it
#   $status                      the exit status of the last run_child or run_reap
#
# Environment (all optional):
#   DEADLINE_HOURS   stop the command (cleanly, with SIGTERM) this many hours after the FIRST start.  The deadline is
#                    kept in $TAXMAN_WORK/deadline (chain) or deadline.NAME, so a restart does not extend it; delete
#                    the file to set another.
#   POWEROFF         never (default) | done (when outcome is complete) | always (whenever the script ends): run
#                    `sudo shutdown -h now`, so that a forgotten instance stops billing
#   S3_DEST          s3://bucket/prefix: copy results, checkpoints and logs there every SAVE_EVERY seconds (default
#                    600) and when the script ends; see save.sh.  Every save runs at the lowest priority (`nice -n 19`,
#                    and `ionice -c 3` where there is an ionice, on Linux), so that it never competes with the workers
#
# SIGTERM and SIGINT are forwarded, once, to the command: the solver of chain.sh writes a final checkpoint and exits
# (a second stop signal would make it exit at once, without one), and exact of prover.sh has no handler, so the
# games in its pass are lost back to their last checkpoint.  The hunt of hunt.sh has none either: the pairs in progress
# are lost, and the finished ones are in its output.

: "${POWEROFF:=never}"
: "${SAVE_EVERY:=600}"
case "$POWEROFF" in never | done | always) ;; *) die "POWEROFF must be never, done or always" ;; esac

saver=""
child=""
children=""
launched=""
stop_requested=""
outcome="failed"
status=0
deadline=""
RUN_NAME=""

# Runs a command at the lowest CPU priority, and at the lowest I/O priority too where ionice(1) exists (Linux): the
# saves to S3 (an `aws s3 sync` costs about 0.4 s of CPU, a save about 1.5 s) squander nothing, and this keeps them out
# of the workers' way.
quietly() {
    if command -v ionice >/dev/null 2>&1; then
        nice -n 19 ionice -c 3 "$@"
    else
        nice -n 19 "$@"
    fi
}

run_cleanup() {
    if [ -n "$saver" ]; then
        pkill -P "$saver" 2>/dev/null
        kill "$saver" 2>/dev/null
    fi
    [ -n "${S3_DEST:-}" ] && quietly "$CLOUD_DIR/save.sh" >>"$LOGS/save.log" 2>&1
    rm -f "$TAXMAN_WORK/$RUN_NAME.pid"
    if [ "$POWEROFF" = always ] || { [ "$POWEROFF" = done ] && [ "$outcome" = complete ]; }; then
        log "powering off (POWEROFF=$POWEROFF, outcome $outcome)"
        sudo -n shutdown -h now || log "could not shut down: $?"
    fi
}

# SIGTERM and SIGINT go to the command (to each of the commands run_start has started), which exits with 128 + the
# signal.
run_on_signal() {
    stop_requested=1
    log "signal received: asking the command to stop"
    [ -n "$child" ] && kill -TERM "$child" 2>/dev/null
    local pid
    for pid in $children; do
        kill -TERM "$pid" 2>/dev/null
    done
    return 0
}

run_nap() {
    sleep "$1" &
    wait $!
}

run_begin() {
    RUN_NAME="$1"
    shift
    mkdir -p "$CKPT" "$LOGS" "$@"
    LOG_FILE="$LOGS/$RUN_NAME.log" # this script's messages (env.sh's log) go to it as well as the terminal

    # Job control: the command runs in its own process group, so a Ctrl-C, or a signal sent to the whole group, reaches
    # only this script, which forwards ONE SIGTERM.
    set -m

    # Checkpoints and results must survive a stop: not tmpfs.
    local dir
    for dir in "$CKPT" "$@"; do
        case "$(df -T "$dir" 2>/dev/null | awk 'NR == 2 {print $2}')" in
            tmpfs | ramfs) die "$dir is on a memory filesystem: it would vanish with the instance (set TAXMAN_WORK)" ;;
        esac
    done

    # One run of this name at a time.
    if command -v flock >/dev/null 2>&1; then
        exec 9>"$TAXMAN_WORK/$RUN_NAME.lock"
        flock -n 9 || die "another $RUN_NAME run is going (it holds $TAXMAN_WORK/$RUN_NAME.lock)"
    else
        log "warning: no flock(1); relying on the checkpoint directory lock"
    fi
    echo $$ >"$TAXMAN_WORK/$RUN_NAME.pid"

    # The deadline: fixed at the first start, in seconds since the epoch.
    local deadline_file="$TAXMAN_WORK/deadline"
    [ "$RUN_NAME" = chain ] || deadline_file="$deadline_file.$RUN_NAME"
    if [ -n "${DEADLINE_HOURS:-}" ] && [ ! -e "$deadline_file" ]; then
        awk -v now="$(date +%s)" -v hours="$DEADLINE_HOURS" 'BEGIN {printf "%d\n", now + hours * 3600}' \
            >"$deadline_file"
    fi
    [ -r "$deadline_file" ] && deadline="$(cat "$deadline_file")"
    if [ -n "$deadline" ] && ! command -v timeout >/dev/null 2>&1; then
        log "warning: no timeout(1): the deadline cannot be enforced"
        deadline=""
    fi

    trap run_cleanup EXIT
    trap run_on_signal TERM INT HUP

    if [ -n "${S3_DEST:-}" ]; then
        (
            while sleep "$SAVE_EVERY"; do
                quietly "$CLOUD_DIR/save.sh" >>"$LOGS/save.log" 2>&1
            done
        ) &
        saver=$!
        log "saving to $S3_DEST every ${SAVE_EVERY}s"
    fi
}

# Starts CMD in the background, its output appended to OUT and ERR, under timeout(1) if there is a deadline, and leaves
# its pid in $launched.  A deadline already past starts nothing: $launched is empty and $status is 124.
run_launch() {
    local out="$1" err="$2"
    shift 2
    local wrap=()
    launched=""
    if [ -n "$deadline" ]; then
        local left=$((deadline - $(date +%s)))
        if [ "$left" -le 0 ]; then
            log "the deadline has passed"
            status=124
            return 0
        fi
        wrap=(timeout --signal=TERM --kill-after=120 "$left")
    fi
    ${wrap[@]+"${wrap[@]}"} "$@" </dev/null >>"$out" 2>>"$err" &
    launched=$!
}

run_child() {
    run_launch "$@"
    [ -n "$launched" ] || return 0
    child=$launched
    # A trapped signal ends `wait` early, while the command is still stopping: wait again until it is gone.
    while :; do
        wait "$child"
        status=$?
        kill -0 "$child" 2>/dev/null || break
    done
    child=""
}

# For several commands at once: run_start starts one and does not wait; run_reap collects it once it has ended.
run_start() {
    run_launch "$@"
    [ -z "$launched" ] || children="$children $launched"
    return 0
}

run_reap() {
    local pid="$1" other rest=""
    kill -0 "$pid" 2>/dev/null && return 1
    wait "$pid"
    status=$?
    for other in $children; do
        [ "$other" = "$pid" ] || rest="$rest $other"
    done
    children="$rest"
    return 0
}
