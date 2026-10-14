#!/usr/bin/env bash
# What the long-run scripts (chain.sh, evidence.sh) share: one run at a time, a log, a pid file, a deadline, periodic
# copies to S3, a clean stop on a signal, and an optional power-off.  Source it after env.sh; do not run it.
#
#   run_begin NAME DIR...        NAME is the run's name (chain, evidence); DIR... are directories that must survive
#                                 the instance (refused if on a memory filesystem).  Sets up everything below.
#   run_child OUT ERR CMD...     run CMD in the background, its stdout and stderr appended to OUT and ERR, and wait
#                                 for it; its exit status is left in $status.  Under a deadline the command runs under
#                                 timeout(1): a stop by the deadline is status 124 (and so is a deadline already past,
#                                 without running anything).
#   run_nap SECONDS              sleep that a signal interrupts at once
#   $stop_requested              non-empty once SIGTERM, SIGINT or SIGHUP has arrived
#   $outcome                     set it to complete when the work is done; cleanup acts on it (POWEROFF=done), and the
#                                 caller picks its exit status from it
#   $status                      the exit status of the last run_child
#
# Environment (all optional):
#   DEADLINE_HOURS   stop the command (cleanly, with SIGTERM) this many hours after the FIRST start.  The deadline is kept
#                    in $TAXMAN_WORK/deadline (chain) or deadline.NAME, so a restart does not extend it; delete the file
#                    to set another.
#   POWEROFF         never (default) | done (when outcome is complete) | always (whenever the script ends): run
#                    `sudo shutdown -h now`, so that a forgotten instance stops billing
#   S3_DEST          s3://bucket/prefix: copy results, checkpoints and logs there every SAVE_EVERY seconds (default
#                    600) and when the script ends; see save.sh
#
# SIGTERM and SIGINT are forwarded, once, to the command: the solver of chain.sh writes a final checkpoint and exits
# (a second stop signal would make it exit at once, without one), and exact of evidence.sh has no handler, so the
# games in its pass are lost back to their last checkpoint.

: "${POWEROFF:=never}"
: "${SAVE_EVERY:=600}"
case "$POWEROFF" in never | done | always) ;; *) die "POWEROFF must be never, done or always" ;; esac

saver=""
child=""
stop_requested=""
outcome="failed"
status=0
deadline=""
RUN_NAME=""

# This script's messages go to the terminal and to $LOGS/NAME.log, written directly (not through a tee, which would
# die with a Ctrl-C aimed at the whole foreground group and take the log with it).
log() {
    local line
    line="$(printf '%s %s' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*")"
    [ -n "$RUN_NAME" ] && printf '%s\n' "$line" >>"$LOGS/$RUN_NAME.log"
    printf '%s\n' "$line" >&2
}

run_cleanup() {
    if [ -n "$saver" ]; then
        pkill -P "$saver" 2>/dev/null
        kill "$saver" 2>/dev/null
    fi
    [ -n "${S3_DEST:-}" ] && "$CLOUD_DIR/save.sh" >>"$LOGS/save.log" 2>&1
    rm -f "$TAXMAN_WORK/$RUN_NAME.pid"
    if [ "$POWEROFF" = always ] || { [ "$POWEROFF" = done ] && [ "$outcome" = complete ]; }; then
        log "powering off (POWEROFF=$POWEROFF, outcome $outcome)"
        sudo -n shutdown -h now || log "could not shut down: $?"
    fi
}

# SIGTERM and SIGINT go to the command, which exits with 128 + the signal.
run_on_signal() {
    stop_requested=1
    log "signal received: asking the command to stop"
    [ -n "$child" ] && kill -TERM "$child" 2>/dev/null
}

run_nap() {
    sleep "$1" &
    wait $!
}

run_begin() {
    RUN_NAME="$1"
    shift
    mkdir -p "$CKPT" "$LOGS" "$@"

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
        awk -v now="$(date +%s)" -v hours="$DEADLINE_HOURS" 'BEGIN {printf "%d\n", now + hours * 3600}' >"$deadline_file"
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
                "$CLOUD_DIR/save.sh" >>"$LOGS/save.log" 2>&1
            done
        ) &
        saver=$!
        log "saving to $S3_DEST every ${SAVE_EVERY}s"
    fi
}

run_child() {
    local out="$1" err="$2"
    shift 2
    local wrap=()
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
    child=$!
    # A trapped signal ends `wait` early, while the command is still stopping: wait again until it is gone.
    while :; do
        wait "$child"
        status=$?
        kill -0 "$child" 2>/dev/null || break
    done
    child=""
}
