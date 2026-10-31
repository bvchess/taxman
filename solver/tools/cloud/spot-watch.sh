#!/usr/bin/env bash
# On an EC2 Spot interruption notice (two minutes of warning), stop the run cleanly and save.
#
#   solver/tools/cloud/spot-watch.sh   run it next to chain.sh, prover.sh or certify.sh, in a second tmux window
#
# It polls the instance metadata service (IMDSv2) every 5 seconds.  When the notice appears it sends SIGTERM to the
# chain.sh, prover.sh or certify.sh whose pid is in $TAXMAN_WORK/chain.pid, prover.pid or certify_PHASE.pid (the
# solver writes a final checkpoint and chain.sh ends; so does each `taxman certify` of certify.sh, which ends;
# `exact` of prover.sh pauses every search into its checkpoint and prover.sh ends), waits up to 90 seconds for it,
# then runs save.sh (which copies to S3 if S3_DEST is set).  It also logs a rebalance recommendation, which comes
# earlier and is only a hint.  Without this script a spot interruption costs at most one checkpoint interval (300 s) of
# the chain's or the certificates' work, or one pass (PASS_SECONDS) of prover.sh's; with it, nothing.
#
# Its messages go to the terminal and to $LOGS/spot-watch.log, so the notice and the save leave a trace after the window
# is gone (the output of save.sh goes to $LOGS/save.log, as when run.sh runs it).
#
# Not on EC2 (no metadata service) it says so and exits.

set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"

mkdir -p "$LOGS"
LOG_FILE="$LOGS/spot-watch.log"

imds="http://169.254.169.254/latest"

token() {
    curl -sS -m 2 -X PUT "$imds/api/token" -H "X-aws-ec2-metadata-token-ttl-seconds: 60" 2>/dev/null
}

# The HTTP status of a metadata path (000: no answer).
status_of() {
    curl -s -m 2 -o /dev/null -w '%{http_code}' -H "X-aws-ec2-metadata-token: $1" "$imds/meta-data/$2" 2>/dev/null
}

rebalance_logged=""
log "watching for a spot interruption notice"
while :; do
    t="$(token)"
    if [ -z "$t" ]; then
        log "no metadata service answered: this is not an EC2 instance (or IMDS is off); exiting"
        exit 1
    fi
    if [ "$(status_of "$t" spot/instance-action)" = 200 ]; then
        log "SPOT INTERRUPTION NOTICE: $(curl -s -m 2 -H "X-aws-ec2-metadata-token: $t" "$imds/meta-data/spot/instance-action")"
        stopped=""
        for pidfile in "$TAXMAN_WORK"/chain.pid "$TAXMAN_WORK"/prover.pid \
            "$TAXMAN_WORK"/certify_*.pid; do
            if [ -r "$pidfile" ] && kill -0 "$(cat "$pidfile")" 2>/dev/null; then
                pid="$(cat "$pidfile")"
                name="$(basename "$pidfile" .pid)"
                log "stopping $name (pid $pid) with SIGTERM"
                kill -TERM "$pid"
                for _ in $(seq 1 90); do
                    kill -0 "$pid" 2>/dev/null || break
                    sleep 1
                done
                kill -0 "$pid" 2>/dev/null && log "$name is still running after 90 s"
                stopped=1
            fi
        done
        [ -n "$stopped" ] || log "no chain.sh, prover.sh or certify.sh is running"
        "$here/save.sh" >>"$LOGS/save.log" 2>&1 || log "save.sh failed (see $LOGS/save.log)"
        log "done; the instance will be taken away"
        exit 0
    fi
    if [ -z "$rebalance_logged" ] && [ "$(status_of "$t" events/recommendations/rebalance)" = 200 ]; then
        log "rebalance recommendation received (the instance is at elevated risk of interruption)"
        rebalance_logged=1
    fi
    sleep 5
done
