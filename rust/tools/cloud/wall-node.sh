#!/usr/bin/env bash
# The instance's side of the unattended run (wall.sh): what a machine of the Auto Scaling group does from its first
# minute to its end.  The user data of the launch template starts it, as ec2-user in a tmux session `tx`, in a checkout
# of the pinned commit; it is not run by hand (`tmux attach -t tx` shows it).
#
#   S3_DEST=s3://bucket/prefix ASG=taxman-wall rust/tools/cloud/wall-node.sh
#
# Everything the run needs besides those two names is the config object $S3_DEST/run/config, which wall.sh launch wrote
# (the pinned commit, the budget, the date, the wall, the window, ...).  In order:
#
# 1. A run that has ended ($S3_DEST/run/ended exists) only sets the group's desired capacity to 0 again and powers off.
#    So does a run whose machines have been MAX_INSTANCES (default 20) already: a boot loop is a bug, not a run.
# 2. On a fresh machine (no results file), the logs come back from S3 first, then setup.sh builds (no tests; the smoke
#    checks run), then `save.sh --restore` brings the results and the checkpoints: the logs first, so that the first
#    save does not replace the old machine's logs with this one's, which share their names (RUNBOOK.md, section 5).
# 3. spot-watch.sh watches for an interruption notice, a loop watches for $S3_DEST/run/stop (wall.sh stop), and
#    chain.sh runs the chain with the settings of the config, GAME_HOURS (the wall), BUDGET_HOURS and RUN_UNTIL (the
#    ledger in S3, run.sh) and POWEROFF=never: this script decides what the end means.
# 4. What chain.sh's exit status means:
#      143  stopped: by wall.sh stop (the stop object exists): the run is over.  Otherwise an interruption
#           (spot-watch.sh sent the SIGTERM): nothing to do, the group replaces the machine and its replacement
#           resumes from S3.  The machine waits 5 minutes (INTERRUPTION_WAIT) to be taken away; one still alive then
#           was stopped by hand, and that ends the run too.
#      0    the range is complete                  3  the budget, the date or DEADLINE_HOURS
#      4    the wall: a game's own search passed GAME_HOURS (the checkpoint is kept)
#      else an error
#    Every ending but an interruption is the run's end: this script writes $S3_DEST/run/ended (the reason), sets the
#    group's desired capacity to 0 (the permission of wall-policy.json), saves, and powers off, so that nothing is
#    launched in its place.  A node script that dies on its own does the same: the run ends with the error.  A signal
#    to the script (the shutdown that follows an interruption sends SIGTERM to every process, in the setup as well)
#    is not an ending: it leaves the run as it is.
#
# Environment: S3_DEST, ASG (the group's name) and AWS_DEFAULT_REGION come from the user data.  SKIP_SETUP=1 leaves out
# setup.sh (a machine that has been set up).  RETRY_SLEEP (default 10 s) is the wait between tries to set the capacity,
# STOP_POLL (30 s) the wait between looks for the stop object, INTERRUPTION_WAIT (300 s) as above.

set -u
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"

[ -n "${S3_DEST:-}" ] || die "S3_DEST is not set (the user data of the launch template sets it)"
[ -n "${ASG:-}" ] || die "ASG is not set (the name of the Auto Scaling group)"
S3_DEST="${S3_DEST%/}"
export S3_DEST
AWS_CLI="${AWS_CLI:-aws}"
RETRY_SLEEP="${RETRY_SLEEP:-10}"
INTERRUPTION_WAIT="${INTERRUPTION_WAIT:-300}"
STOP_POLL="${STOP_POLL:-30}"

ended=""
interrupted=""

# Whether the object NAME is in $S3_DEST/run.
s3_has() {
    local listing
    listing="$("$AWS_CLI" s3 ls "$S3_DEST/run/$1" 2>/dev/null)"
    printf '%s\n' "$listing" | grep -q " $1\$"
}

# The settings of the run, from the config object.  Only these names are taken, each as the rest of its line.
config_load() {
    local text key value
    text="$("$AWS_CLI" s3 cp "$S3_DEST/run/config" - --only-show-errors)" || return 1
    while IFS='=' read -r key value; do
        case "$key" in
            COMMIT | INSTANCE_TYPE | CHAIN_TO | WINDOW | GAME_HOURS | BUDGET_HOURS | RUN_UNTIL | THREADS | \
                EXTRA_ARGS | CHECKPOINT_INTERVAL | SAVE_EVERY | LEDGER_MARGIN | LEDGER_EVERY | MAX_INSTANCES)
                export "$key=$value"
                ;;
        esac
    done <<<"$text"
}

# The run is over for the reason given: records it, takes the group to capacity 0 so that nothing replaces this
# machine, saves, and powers off.
end_run() {
    local reason="$*" try now
    ended=1
    log "the run is over: $reason"
    if ! s3_has ended; then
        printf 'time=%s\nreason=%s\n' "$(date +%s)" "$reason" |
            "$AWS_CLI" s3 cp - "$S3_DEST/run/ended" --only-show-errors || log "could not write $S3_DEST/run/ended"
    fi
    for try in 1 2 3 4 5; do
        if "$AWS_CLI" autoscaling set-desired-capacity --auto-scaling-group-name "$ASG" --desired-capacity 0; then
            now="$("$AWS_CLI" autoscaling describe-auto-scaling-groups --auto-scaling-group-names "$ASG" \
                --query 'AutoScalingGroups[0].DesiredCapacity' --output text 2>&1)"
            log "the group $ASG was set to desired capacity 0 (it says $now): nothing will replace this machine"
            break
        fi
        log "could not set the desired capacity of $ASG to 0 (try $try of 5)"
        [ "$try" = 5 ] || sleep "$RETRY_SLEEP"
    done
    "$here/save.sh" >>"$LOGS/save.log" 2>&1 || log "the last save failed (see save.log)"
    log "powering off"
    sudo -n shutdown -h now || log "could not shut down: $?"
}

# The run cannot go on, for the reason given: it ends.
fail_run() {
    end_run "$*"
    exit 0
}

# However this script ends without having ended the run or been signalled, the run ends with the error.
node_exit() {
    local code=$?
    trap - EXIT
    if [ -z "$ended" ] && [ -z "$interrupted" ]; then
        end_run "the node script exited unexpectedly (status $code)"
    fi
}

# A signal to this script is the machine being taken away (the shutdown of a spot interruption sends SIGTERM to every
# process) or someone at the keyboard: not the run's end.  The group replaces the machine, or the timer ends it.
on_signal() {
    interrupted=1
    log "signalled: leaving the run as it is"
    exit 143
}

# Ctrl-C at the keyboard reaches this script alone (a job started with & ignores SIGINT), so it passes one SIGTERM to
# chain.sh, which stops the solver cleanly.
on_interrupt() {
    [ -z "${chain:-}" ] || kill -TERM "$chain" 2>/dev/null
    on_signal
}

mkdir -p "$TAXMAN_WORK" "$LOGS"
if [ ! -e "$RESULTS" ]; then
    "$AWS_CLI" s3 sync "$S3_DEST/logs" "$LOGS" --only-show-errors
fi
LOG_FILE="$LOGS/wall-node.log"
trap node_exit EXIT
trap on_signal TERM HUP
trap on_interrupt INT
log "wall-node on $(hostname): $S3_DEST, group $ASG"

config_load || fail_run "cannot read the run's config $S3_DEST/run/config"
if s3_has ended; then
    end_run "$("$AWS_CLI" s3 cp "$S3_DEST/run/ended" - --only-show-errors | sed -n 's/^reason=//p')"
    exit 0
fi
boots="$("$AWS_CLI" s3 ls "$S3_DEST/run/boots/" 2>/dev/null | wc -l | tr -d ' ')"
if [ "$boots" -ge "${MAX_INSTANCES:-20}" ]; then
    end_run "this is machine $((boots + 1)) of the run, more than MAX_INSTANCES=${MAX_INSTANCES:-20}: a boot loop?"
    exit 0
fi
printf '%s %s\n' "$(date +%s)" "$(hostname)" |
    "$AWS_CLI" s3 cp - "$S3_DEST/run/boots/$(date +%s)-$(hostname)" --only-show-errors ||
    log "could not record this machine"

if [ "${SKIP_SETUP:-0}" != 1 ]; then
    log "setup.sh"
    NO_PULL=1 SKIP_TESTS=1 "$here/setup.sh" || fail_run "setup.sh failed (see setup.log)"
fi
if [ ! -e "$RESULTS" ]; then
    "$here/save.sh" --restore || fail_run "cannot restore the state from $S3_DEST"
fi

# The helpers: the interruption watcher (a tmux window, to be seen), and the loop that carries out wall.sh stop.
if [ -n "${TMUX:-}" ]; then
    tmux new-window -d -n sw "env S3_DEST=$S3_DEST $here/spot-watch.sh"
else
    "$here/spot-watch.sh" >/dev/null 2>&1 &
fi
(
    while sleep "$STOP_POLL"; do
        if s3_has stop; then
            log "stop requested ($S3_DEST/run/stop): asking chain.sh to stop"
            kill -TERM "$(cat "$TAXMAN_WORK/chain.pid" 2>/dev/null)" 2>/dev/null
            exit 0
        fi
    done
) &
stopper=$!

export POWEROFF=never WINDOW="${WINDOW:-4}" CHECKPOINT_INTERVAL="${CHECKPOINT_INTERVAL:-300}" \
    SAVE_EVERY="${SAVE_EVERY:-600}"
chain_to="${CHAIN_TO:-8191}"
until_text=none
[ -z "${RUN_UNTIL:-}" ] || until_text="$(epoch_date "$RUN_UNTIL")"
log "chain.sh $chain_to (commit ${COMMIT:-unknown}; wall ${GAME_HOURS:-none} h; budget ${BUDGET_HOURS:-none} h;" \
    "until $until_text)"
# A signal that reaches the whole machine (a shutdown) reaches chain.sh itself, which stops the solver cleanly; this
# script does not forward it (that would be two), it only stays out of the end.
chain=""
"$here/chain.sh" "$chain_to" &
chain=$!
wait "$chain"
status=$?
pkill -P "$stopper" 2>/dev/null
kill "$stopper" 2>/dev/null
what="$(cat "$TAXMAN_WORK/chain.outcome" 2>/dev/null || echo 'no outcome recorded')"
log "chain.sh exited with status $status: $what"

case "$status" in
    143)
        if s3_has stop; then
            end_run "stopped on request (wall.sh stop)"
        else
            interrupted=1
            log "an interruption, it seems: the group will replace this machine; waiting ${INTERRUPTION_WAIT}s" \
                "to be taken away"
            sleep "$INTERRUPTION_WAIT"
            interrupted=""
            end_run "stopped by hand: chain.sh ended on a signal and nothing took this machine away"
        fi
        ;;
    0) end_run "$what" ;;
    3 | 4) end_run "$what" ;;
    *) end_run "error (chain.sh exited with status $status): $what" ;;
esac
exit 0
