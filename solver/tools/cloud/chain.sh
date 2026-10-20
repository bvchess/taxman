#!/usr/bin/env bash
# The production run: the chain of optimal games from n = 1 to N, resumable, with logging.
#
#   solver/tools/cloud/chain.sh N
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
#   124      the deadline (DEADLINE_HOURS, the budget BUDGET_HOURS or the date RUN_UNTIL, whichever came first) stopped
#            the solver, with a final checkpoint: the script ends
#   4        THE WALL: a game's own search passed GAME_HOURS (`taxman solve --game-hours`).  Every search was stopped
#            with a final checkpoint and nothing was recorded for the game: the script logs "wall at N after H
#            hours" and ends
#   1, 2     the solver reported an error (a refused checkpoint, a game that failed a check, ...): the script ends,
#            nothing is retried, because the error would repeat; read chain.err
#   other    a crash (abort, SIGKILL by the OOM killer): run again after 10 s.  MAX_FAILS passes in a row that add
#            no line to the results file and last under five minutes end the script
#
# The script exits with 0 when the range is complete, 143 when stopped, 3 for the deadline, 4 for the wall, 1 otherwise.
# How it ended is also the one line of $TAXMAN_WORK/chain.outcome (complete, stopped, "wall at N after H hours", the
# deadline's reason, failed), for a script that supervises it (wall-node.sh).
#
# Output: $LOGS/chain.out (the solver's result lines), chain.err (its progress lines), chain.log (this script).
#
# Environment (all optional):
#   THREADS              worker threads, default all CPUs the solver may use
#   WINDOW               the games in flight at once, default 4 (as `taxman solve --window`'s); 1 solves them one
#                        after the other.  4 is the best measured on 64 threads past 2225, where 16 is 20 to 32%
#                        slower than 1 (RUNBOOK.md, section 4.2; the notebook entries round-b-the-chain-on-many-threads
#                        and scaling-to-192-threads, both 2026-10)
#   CHECKPOINT_INTERVAL  seconds, default 300       PROGRESS  seconds between progress lines, default 60
#                        (a checkpoint stops every worker for the slowest one's step, up to 0.4 s near n = 4000: 0.15%
#                        of the machine at 300 s, 0.7% at 60 s; SIGTERM still writes a final one)
#   EXTRA_ARGS           more flags for `taxman solve`, e.g. "--mem-cap 100000", or "--use-prev": the chain then
#                        draws on the known games of the checkout's results/ (the committed chain, the
#                        proven boards of the bounds store) instead of searching for them (RUNBOOK.md, section 5.4)
#   MAX_FAILS            default 5
#   CHAIN_TO             N, if not given as the argument (the solver goes up to 8191; RUNBOOK.md, "The production
#                        chain", the item "Past 4095")
#   GAME_HOURS           the wall: stop at the first game whose own search has taken more than this many hours in all,
#                        over every run (`taxman solve --game-hours`; its clock is kept in the checkpoint directory,
#                        so a run that resumes the game goes on from it).  Default: none
#   DEADLINE_HOURS, BUDGET_HOURS, RUN_UNTIL, LEDGER_*, POWEROFF, S3_DEST, SAVE_EVERY: as in run.sh, which this script
#                        and prover.sh share

set -u
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"
# shellcheck source=run.sh
. "$here/run.sh"

to="${1:-${CHAIN_TO:-}}"
case "$to" in
    '' | *[!0-9]*) die "usage: chain.sh N   (the last game; the chain starts at 1)" ;;
esac
[ "$to" -ge 1 ] || die "N must be at least 1"
[ -x "$TAXMAN_BIN" ] || die "$TAXMAN_BIN is missing: run setup.sh"

# Without THREADS the solver uses every CPU it is allowed (in a container, the cgroup quota).
thread_args=()
[ -n "${THREADS:-}" ] && thread_args=(--threads "$THREADS")
wall_args=()
[ -n "${GAME_HOURS:-}" ] && wall_args=(--game-hours "$GAME_HOURS")
WINDOW="${WINDOW:-4}"
CHECKPOINT_INTERVAL="${CHECKPOINT_INTERVAL:-300}"
PROGRESS="${PROGRESS:-60}"
MAX_FAILS="${MAX_FAILS:-5}"

run_begin chain "$(dirname "$RESULTS")"
rm -f "$TAXMAN_WORK/chain.outcome"

# How the script ended, for whatever supervises it.
set_outcome() {
    outcome_text="$*"
    printf '%s\n' "$outcome_text" >"$TAXMAN_WORK/chain.outcome"
}

# The lines of the results file (0 if there is none yet).
line_count() {
    if [ -r "$RESULTS" ]; then wc -l <"$RESULTS" | tr -d ' '; else echo 0; fi
}

log "chain 1-$to; threads: ${THREADS:-all available}; window $WINDOW; wall: ${GAME_HOURS:-none} hours;" \
    "results $RESULTS; checkpoints $CKPT"
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
        ${wall_args[@]+"${wall_args[@]}"} --window "$WINDOW" --out "$RESULTS" --checkpoint-dir "$CKPT" \
        --checkpoint-interval "$CHECKPOINT_INTERVAL" --progress "$PROGRESS" ${EXTRA_ARGS:-}
    elapsed=$(($(date +%s) - started))
    log "the solver exited with status $status after ${elapsed}s"

    crashed=0
    case "$status" in
        0)
            report="$("$TAXMAN_BIN" verify-results "$RESULTS" 2>&1)" || log "verify-results failed: $report"
            if printf '%s\n' "$report" | grep -q "n = 1\.\.=$to (0 missing)"; then
                log "the range 1-$to is complete"
                outcome="complete"
                set_outcome "complete: the range 1-$to"
                break
            fi
            log "the solver ended before $to (a limit in EXTRA_ARGS?); going on"
            ;;
        124)
            log "stopped by $deadline_reason (a final checkpoint was written)"
            outcome="deadline"
            set_outcome "deadline: $deadline_reason"
            break
            ;;
        4)
            # The solver's line, `wall at N after H hours: ...`, is the last of its kind in chain.out.
            wall="$(sed -n 's/^\(wall at [0-9]* after [0-9.]* hours\):.*/\1/p' "$LOGS/chain.out" | tail -n 1)"
            log "${wall:-wall} (all searches stopped with a final checkpoint; nothing recorded for the game)"
            outcome="wall"
            set_outcome "${wall:-wall}"
            break
            ;;
        130 | 143)
            log "the solver was stopped by a signal (a final checkpoint was written)"
            outcome="stopped"
            set_outcome "stopped by a signal"
            break
            ;;
        1 | 2)
            log "the solver reported an error; not retrying.  The last lines of chain.err:"
            tail -n 5 "$LOGS/chain.err" >&2
            outcome="failed"
            set_outcome "failed: the solver reported an error (status $status):" \
                "$(tail -n 1 "$LOGS/chain.err" | cut -c1-200)"
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
        set_outcome "failed: $fails passes in a row without progress (the solver last exited with status $status)"
        break
    fi
    [ "$crashed" = 1 ] && run_nap 10
done

log "ended: $outcome"
if [ "$outcome" = complete ]; then
    if ! "$here/verify.sh" "$RESULTS" 2>&1 | tee -a "$LOGS/verify.log"; then
        outcome="failed"
        set_outcome "failed: verify.sh found a problem (see verify.log)"
    fi
fi
# A stop by SIGTERM before the first pass ended, or any other way out of the loop, still says how.
[ -s "$TAXMAN_WORK/chain.outcome" ] || set_outcome "$outcome"
case "$outcome" in
    complete) exit 0 ;;
    stopped) exit 143 ;;
    deadline) exit 3 ;;
    wall) exit 4 ;;
    *) exit 1 ;;
esac
