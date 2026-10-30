#!/usr/bin/env bash
# What the long-run scripts (chain.sh, prover.sh, certify.sh) share: one run at a time, a log, a pid file, a
# deadline, periodic copies to S3, a clean stop on a signal, and an optional power-off.  Source it after env.sh; do
# not run it.
#
#   run_begin NAME DIR...        NAME is the run's name (chain, prover, certify_PHASE); DIR... are directories
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
#   $deadline_reason             what the deadline of a status 124 is: the deadline of DEADLINE_HOURS, the budget, or
#                                 the date, whichever comes first
#
# Environment (all optional):
#   DEADLINE_HOURS   stop the command (cleanly, with SIGTERM) this many hours after the FIRST start.  The deadline is
#                    kept in $TAXMAN_WORK/deadline (chain) or deadline.NAME, so a restart does not extend it; delete
#                    the file to set another.  It is kept on this machine only: for a run that a replacement machine
#                    continues, BUDGET_HOURS and RUN_UNTIL
#   BUDGET_HOURS     the instance-hours the whole run may use, counted across every machine that runs it (needs
#                    S3_DEST).  The run's ledger, $S3_DEST/run/ledger, keeps when the run began and the hours the
#                    machines before this one used; this machine adds its uptime (what an instance is billed for, to
#                    within its boot) and writes the sum every LEDGER_EVERY seconds (default 60) and when the script
#                    ends, so a replacement machine goes on with the same count.  The command is stopped as by
#                    DEADLINE_HOURS, LEDGER_MARGIN seconds (default 300, for the final checkpoint and save) before the
#                    hours are used up; $deadline_reason says which limit stopped it
#   RUN_UNTIL        a date, in seconds since the epoch: the same stop, LEDGER_MARGIN seconds before it.  The date is
#                    the same on every machine, so it needs no ledger; with S3_DEST the ledger records it all the same
#   POWEROFF         never (default) | done (when outcome is complete) | always (whenever the script ends): run
#                    `sudo shutdown -h now`, so that a forgotten instance stops billing
#   S3_DEST          s3://bucket/prefix: copy results, checkpoints and logs there every SAVE_EVERY seconds (default
#                    600) and when the script ends; see save.sh.  Every save runs at the lowest priority (`nice -n 19`,
#                    and `ionice -c 3` where there is an ionice, on Linux), so that it never competes with the workers
#
# SIGTERM and SIGINT are forwarded, once, to the command: the solver of chain.sh writes a final checkpoint and exits
# (a second stop signal would make it exit at once, without one), and exact of prover.sh has no handler, so the
# games in its pass are lost back to their last checkpoint.

: "${POWEROFF:=never}"
: "${SAVE_EVERY:=600}"
: "${LEDGER_EVERY:=60}"
: "${LEDGER_MARGIN:=300}"
AWS_CLI="${AWS_CLI:-aws}"
case "$POWEROFF" in never | done | always) ;; *) die "POWEROFF must be never, done or always" ;; esac

saver=""
ledgerer=""
child=""
children=""
launched=""
stop_requested=""
outcome="failed"
status=0
deadline=""
deadline_reason=""
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

# --- the run's ledger ---------------------------------------------------------------------------------------------
#
# The ledger is the object $S3_DEST/run/ledger, five lines of name=value:
#   start      when the run began, in seconds since the epoch (the first machine writes it, none after)
#   boot       the boot id of the machine that wrote it last
#   base       the instance-hours of the machines before that one; hours: base plus that machine's uptime then
#   updated    when it was written
# A machine that starts a run finds the ledger, takes its hours as the base of its own count (unless the boot is its
# own, which a second script of the same boot is), and counts on.  Only one machine writes at a time: a run has one
# instance at most.

# Whether the run keeps a ledger.
ledger_active() {
    [ -n "${S3_DEST:-}" ] && { [ -n "${BUDGET_HOURS:-}" ] || [ -n "${RUN_UNTIL:-}" ]; }
}

# Seconds since this machine booted: the instance's billed time, to within its boot.  UPTIME_SECONDS stands in for it.
ledger_uptime() {
    local booted
    if [ -n "${UPTIME_SECONDS:-}" ]; then
        echo "$UPTIME_SECONDS"
    elif [ -r /proc/uptime ]; then
        awk '{printf "%d\n", $1}' /proc/uptime
    else
        booted="$(sysctl -n kern.boottime 2>/dev/null | sed -n 's/.*sec = \([0-9]*\).*/\1/p')"
        echo $(($(date +%s) - ${booted:-$(date +%s)}))
    fi
}

# This boot of this machine.  BOOT_ID stands in for it.
ledger_boot_id() {
    if [ -n "${BOOT_ID:-}" ]; then
        echo "$BOOT_ID"
    elif [ -r /proc/sys/kernel/random/boot_id ]; then
        cat /proc/sys/kernel/random/boot_id
    else
        sysctl -n kern.boottime 2>/dev/null | sed -n 's/.*sec = \([0-9]*\).*/boot-\1/p'
    fi
}

# The value of NAME in the ledger text TEXT.
ledger_field() {
    printf '%s\n' "$1" | sed -n "s/^$2=//p" | tail -n 1
}

# Dies unless VALUE is a number (digits and at most one point) fit to be NAME of the ledger.
ledger_number() {
    case "$2" in
        '' | *[!0-9.]* | *.*.*) die "the run's ledger $S3_DEST/run/ledger has no number for $1 (\"$2\")" ;;
    esac
}

# The instance-hours used up to now: the base plus this machine's uptime.
ledger_hours() {
    awk -v base="$ledger_base" -v up="$(ledger_uptime)" 'BEGIN {printf "%.4f\n", base + up / 3600}'
}

ledger_write() {
    printf 'start=%s\nboot=%s\nbase=%s\nhours=%s\nupdated=%s\n' "$ledger_start" "$ledger_boot" "$ledger_base" \
        "$(ledger_hours)" "$(date +%s)" | quietly "$AWS_CLI" s3 cp - "$S3_DEST/run/ledger" --only-show-errors
}

# Makes the earlier of the deadline so far and the epoch time $1 the deadline, for the reason $2.
ledger_deadline() {
    if [ -z "$deadline" ] || [ "$1" -lt "$deadline" ]; then
        deadline="$1"
        deadline_reason="$2"
    fi
}

# Reads the ledger (a run's first machine finds none), counts this machine in, writes it back, and sets the deadline
# the budget and the date make.  Dies if the ledger cannot be read or written: a machine that cannot count its hours
# must not run.
ledger_begin() {
    local text out now hours
    now="$(date +%s)"
    ledger_boot="$(ledger_boot_id)"
    if out="$("$AWS_CLI" s3 cp "$S3_DEST/run/ledger" - --only-show-errors 2>&1)"; then
        text="$out"
    elif printf '%s' "$out" | grep -q '(404)'; then
        text=""
    else
        die "cannot read the run's ledger $S3_DEST/run/ledger: $out"
    fi
    if [ -z "$text" ]; then
        ledger_start="$now"
        ledger_base=0
        log "the run's ledger is new: the run starts now"
    else
        ledger_start="$(ledger_field "$text" start)"
        hours="$(ledger_field "$text" hours)"
        ledger_number start "$ledger_start"
        ledger_number hours "$hours"
        if [ "$(ledger_field "$text" boot)" = "$ledger_boot" ]; then
            ledger_base="$(ledger_field "$text" base)"
            ledger_number base "$ledger_base"
        else
            ledger_base="$hours"
        fi
    fi
    ledger_write || die "cannot write the run's ledger $S3_DEST/run/ledger"
    if [ -n "${BUDGET_HOURS:-}" ]; then
        ledger_deadline "$(awk -v now="$now" -v budget="$BUDGET_HOURS" -v used="$(ledger_hours)" \
            -v margin="$LEDGER_MARGIN" 'BEGIN {printf "%d\n", now + (budget - used) * 3600 - margin}')" \
            "the budget of $BUDGET_HOURS instance-hours"
    fi
    if [ -n "${RUN_UNTIL:-}" ]; then
        ledger_deadline "$((RUN_UNTIL - LEDGER_MARGIN))" "the date $(epoch_date "$RUN_UNTIL")"
    fi
    log "the run began $(epoch_date "$ledger_start"); instance-hours so far: $(ledger_hours)"
}

run_cleanup() {
    if [ -n "$saver" ]; then
        pkill -P "$saver" 2>/dev/null
        kill "$saver" 2>/dev/null
    fi
    if [ -n "$ledgerer" ]; then
        pkill -P "$ledgerer" 2>/dev/null
        kill "$ledgerer" 2>/dev/null
    fi
    [ -n "${S3_DEST:-}" ] && quietly "$CLOUD_DIR/save.sh" >>"$LOGS/save.log" 2>&1
    # After the save, which is part of the hours.
    [ -n "$ledgerer" ] && { ledger_write || log "could not write the ledger"; }
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
    sleep "$1" 9>&- &
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

    # One run of this name at a time.  The lock, on fd 9, is this script's alone: every process it starts has fd 9
    # closed (9>&-), so that a program that outlives its parent (one that a solver started, say) cannot hold the lock
    # after the script has ended and keep the next run out.  What still guards against two solvers on one checkpoint
    # directory is the solver's own lock on it (taxman.lock): a run that takes this lock while an orphaned solver goes
    # on has its solver refused, and ends.
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
    if [ -r "$deadline_file" ]; then
        deadline="$(cat "$deadline_file")"
        deadline_reason="the deadline (DEADLINE_HOURS)"
    fi
    [ -z "${BUDGET_HOURS:-}" ] || [ -n "${S3_DEST:-}" ] || die "BUDGET_HOURS counts in the ledger in S3: set S3_DEST"
    case "${BUDGET_HOURS:-0}:${RUN_UNTIL:-0}" in
        *[!0-9.:]* | *.*.* | *:*:*) die "BUDGET_HOURS must be a number of hours, RUN_UNTIL seconds since the epoch" ;;
    esac
    if ledger_active; then
        ledger_begin
    elif [ -n "${RUN_UNTIL:-}" ]; then
        ledger_deadline "$((RUN_UNTIL - LEDGER_MARGIN))" "the date $(epoch_date "$RUN_UNTIL")"
    fi
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
        ) 9>&- &
        saver=$!
        log "saving to $S3_DEST every ${SAVE_EVERY}s"
    fi
    if ledger_active; then
        (
            while sleep "$LEDGER_EVERY"; do
                ledger_write || log "could not write the ledger"
            done
        ) 9>&- &
        ledgerer=$!
        log "counting the hours in $S3_DEST/run/ledger every ${LEDGER_EVERY}s"
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
            log "the deadline has passed: $deadline_reason"
            status=124
            return 0
        fi
        wrap=(timeout --signal=TERM --kill-after=120 "$left")
    fi
    ${wrap[@]+"${wrap[@]}"} "$@" </dev/null >>"$out" 2>>"$err" 9>&- &
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
