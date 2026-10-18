#!/usr/bin/env bash
# The prover's proofs of opt(n) for a range of board sizes, on all the CPUs of a machine, resumable.
#
#   solver/tools/cloud/prover.sh A B             prove opt(n) = V(n) for n = A..B  (V: results/strict.jsonl)
#   solver/tools/cloud/prover.sh --verify A B    check the output: every n in 2..B proven, every score the recorded one
#   OUT=FILE solver/tools/cloud/prover.sh --list N1,N2,...   a measurement on exactly those games (list mode, below)
#
# It runs `exact` of the prover (solver/noframe, README section 3), which knows nothing of the solver's frame
# model, over the range.  `exact` runs one game per thread (a game's search is one thread), so the games are spread over
# the CPUs; it runs for PASS_SECONDS, then pauses every search still going, saving its open subtrees and learned
# conflicts to a checkpoint ($CKPT/exact_N_low.ckpt), and this script runs it again, which resumes them.  A game may
# take any number of passes.  The games go dearest first, by the nodes the solver's search of each took (strict.jsonl;
# `exact` takes one to three times as many), so that a long game starts early instead of last.  The searches start
# from the roles of Theorem B (`exact`'s default; EXTRA_ARGS='--fix-low 0' for the plain root).
#
# Safe to run again at any time, after a crash, a reboot, a spot interruption or a SIGTERM: finished games are in the
# output file (one line each, appended and synced), unfinished ones in their checkpoints.  `exact` has no stop handler,
# so a stop or a kill loses the work of the pass in progress (at most PASS_SECONDS of each thread) and nothing else.
# THREADS may differ between runs (a smaller machine for the last games).  A second prover.sh is refused.
#
# What it does with `exact`'s exit status:
#   0        a pass ended (the pause, or all games done): go on, or finish when every game of the range has its line
#   130/143  stopped by SIGINT/SIGTERM: the script ends
#   124      the deadline (DEADLINE_HOURS) stopped it: the script ends
#   1, 2, 101, 134
#            the prover reported an error: a recorded game that does not replay, or (the finding to look for) a playable
#            set that BEATS a recorded optimum.  Nothing is retried; read prover.err
#   other    a crash (the OOM killer): run again after 10 s; MAX_FAILS in a row with no new line end the script
#
# When the range is complete it runs the check (`--verify`) and exits 0 only if that passes.
#
# List mode (`--list N1,N2,...`) runs `exact` over exactly the listed games, in the order given, in the same resumable
# passes, with EXTRA_ARGS (the flags of the measurement, for instance `--fix-upper 1`) into OUT, which must be set.  It
# is for a measurement, not a proof of a range: there is no order file and no check at the end.  It ends `complete`
# (and exits 0) when every listed game has a line in OUT.
#
# Output: $OUT (default $TAXMAN_WORK/results/exact_parity_A_B.jsonl, one line per game, the format of
# solver/noframe/data/exact_parity_2_1300.jsonl), logs in $LOGS: prover.log (this script), prover.err (`exact`).
#
# Environment (all optional, but OUT in list mode):
#   THREADS         threads, default every CPU
#   PASS_SECONDS    the length of a pass, default 300: the work lost to a kill, against the small cost of a restart
#   OUT             the output file (required in list mode)
#   STRICT          the recorded optima, default results/strict.jsonl (committed)
#   BASELINE        the proofs of the smaller games the check builds on (default: the prover's
#                   data/exact_parity_2_1300.jsonl)
#   EXTRA_ARGS      more flags for `exact`
#   MAX_FAILS       default 5
#   DEADLINE_HOURS, POWEROFF, S3_DEST, SAVE_EVERY: as in run.sh (SAVE_EVERY defaults to PASS_SECONDS here)

set -u
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"

STRICT="${STRICT:-$TAXMAN_ROOT/results/strict.jsonl}"
BASELINE="${BASELINE:-$PROVER/data/exact_parity_2_1300.jsonl}"
SUMMARIZE=("python3" "$PROVER/tools/summarize.py")

usage() {
    die "usage: prover.sh [--verify] A B   (the first and last board size, 2 <= A <= B)" \
        "or OUT=FILE prover.sh --list N1,N2,...   (board sizes, each at least 2)"
}
verify_only=0
list_mode=0
if [ "${1:-}" = "--verify" ]; then
    verify_only=1
    shift
elif [ "${1:-}" = "--list" ]; then
    list_mode=1
    shift
fi
if [ "$list_mode" = 1 ]; then
    # The games to measure, each once, in the order given.
    [ "$#" -eq 1 ] || usage
    case "$1" in "" | *[!0-9,]* | ,* | *, | *,,*) usage ;; esac
    list="$(printf '%s\n' "$1" | tr ',' '\n' | awk '!seen[$0]++' | paste -sd, -)"
    for n in ${list//,/ }; do
        [ "$n" -ge 2 ] || usage
    done
    total="$(printf '%s\n' "$list" | tr ',' '\n' | wc -l | tr -d ' ')"
    [ -n "${OUT:-}" ] || die "--list needs OUT=FILE, the output file of the measurement (it is not named after a range)"
    label="list"
    all_of="listed games"
else
    [ "$#" -eq 2 ] || usage
    from="$1"
    to="$2"
    case "$from$to" in *[!0-9]*) usage ;; esac
    [ "$from" -ge 2 ] && [ "$to" -ge "$from" ] || usage
    total=$((to - from + 1))
    OUT="${OUT:-$TAXMAN_WORK/results/exact_parity_${from}_${to}.jsonl}"
    label="$from-$to"
    all_of="games of $from-$to"
fi

# The check: a line for every n in A..B, each with the recorded score, every n in 2..B proven.
verify() {
    "${SUMMARIZE[@]}" check "$from" "$to" "$BASELINE" "$OUT"
}
if [ "$verify_only" = 1 ]; then
    [ -r "$OUT" ] || die "$OUT is not readable (OUT=... names the output file)"
    verify
    exit $?
fi

[ -x "$EXACT_BIN" ] || die "$EXACT_BIN is missing: run setup.sh"
[ -r "$STRICT" ] || die "$STRICT is not readable"
if [ "$list_mode" = 1 ]; then
    for n in ${list//,/ }; do
        grep -q "\"n\":$n," "$STRICT" || die "$STRICT has no game $n (it holds the recorded optima)"
    done
else
    grep -q "\"n\":$to," "$STRICT" || die "$STRICT has no game $to (it holds the recorded optima to prove)"
fi
THREADS="${THREADS:-$(ncpu)}"
PASS_SECONDS="${PASS_SECONDS:-300}"
MAX_FAILS="${MAX_FAILS:-5}"
SAVE_EVERY="${SAVE_EVERY:-$PASS_SECONDS}"
# shellcheck source=run.sh
. "$here/run.sh"
run_begin prover "$(dirname "$OUT")"

# The games of A..B (in list mode, the distinct listed ones) with a line in $OUT.
games_done() {
    [ -r "$OUT" ] || { echo 0; return; }
    if [ "$list_mode" = 1 ]; then
        grep -o '^{"n":[0-9]*' "$OUT" | cut -d: -f2 |
            awk -v list="$list" 'BEGIN { k = split(list, a, ","); for (i = 1; i <= k; i++) want[a[i]] = 1 }
                                 ($1 in want) && !seen[$1]++' | wc -l | tr -d ' '
    else
        grep -o '^{"n":[0-9]*' "$OUT" | cut -d: -f2 | awk -v a="$from" -v b="$to" '$1 >= a && $1 <= b && !seen[$1]++' |
            wc -l | tr -d ' '
    fi
}

if [ "$list_mode" = 0 ]; then
    # The order of the games, the dearest first, made once and kept (the order does not change a result).
    order_file="$TAXMAN_WORK/prover-order_${from}_${to}.txt"
    [ -s "$order_file" ] || "${SUMMARIZE[@]}" order "$from" "$to" >"$order_file" || die "could not order the games"
    list="$(cat "$order_file")"
fi

log "prover $label: $total games; threads $THREADS; passes of ${PASS_SECONDS}s; output $OUT; checkpoints $CKPT"
env_report | tee -a "$LOGS/prover.log" >&2

fails=0
pass=0
while :; do
    if [ -n "$stop_requested" ]; then
        outcome="stopped"
        break
    fi
    done_now="$(games_done)"
    if [ "$done_now" -ge "$total" ]; then
        log "all $total $all_of have a line"
        outcome="complete"
        break
    fi
    pass=$((pass + 1))
    paused="$(find "$CKPT" -maxdepth 1 -name 'exact_*.ckpt' 2>/dev/null | wc -l | tr -d ' ')"
    log "pass $pass: $done_now of $total games done, $paused searches paused in $CKPT"
    if [ "$((total - done_now))" -lt "$THREADS" ] && [ "$THREADS" -gt 1 ]; then
        log "note: $((total - done_now)) games are left for $THREADS threads, and a search is one thread: most of the" \
            "machine idles.  A smaller instance finishes them for less (stop, run this again there: THREADS may differ)"
    fi

    lines_before="$done_now"
    run_child "$LOGS/prover.out" "$LOGS/prover.err" "$EXACT_BIN" --list "$list" --threads "$THREADS" \
        --seconds "$PASS_SECONDS" --checkpoints "$CKPT" --results "$STRICT" --out "$OUT" ${EXTRA_ARGS:-}
    log "pass $pass: exact exited with status $status"

    case "$status" in
        0) fails=0 ;;
        124)
            outcome="deadline"
            break
            ;;
        130 | 143)
            log "stopped by a signal (the searches of this pass go back to their last checkpoints)"
            outcome="stopped"
            break
            ;;
        1 | 2 | 101 | 134)
            log "exact reported an error; not retrying.  The last lines of prover.err:"
            tail -n 8 "$LOGS/prover.err" >&2
            outcome="failed"
            break
            ;;
        *)
            log "exact crashed (status $status)"
            if [ "$(games_done)" -gt "$lines_before" ]; then fails=0; else fails=$((fails + 1)); fi
            if [ "$fails" -ge "$MAX_FAILS" ]; then
                log "giving up after $fails passes in a row without progress"
                outcome="failed"
                break
            fi
            run_nap 10
            ;;
    esac
done

log "ended: $outcome"
if [ "$outcome" = complete ] && [ "$list_mode" = 0 ]; then
    verify 2>&1 | tee -a "$LOGS/prover.log" || outcome="failed"
fi
case "$outcome" in
    complete) exit 0 ;;
    stopped) exit 143 ;;
    deadline) exit 3 ;;
    *) exit 1 ;;
esac
