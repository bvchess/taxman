#!/usr/bin/env bash
# The certificates campaign of stage 4: `taxman certify` over the list files of solver/tools/certify/lists/stage4, one
# list or several at once, resumable, with logging.
#
#   solver/tools/cloud/certify.sh LIST[:THREADS]...
#
# LIST is a name (pilot7, dear_rest, rest_1 to rest_4, deferred: the file $LISTS/NAME.txt) or the path of a list file
# (one board size a line, as `taxman certify --list` reads it; the files of stage4 hold only games that need a
# certificate).  Each list runs as a process of its own, all at once:
#
#   taxman certify --list FILE --results $CERT_RESULTS --upper U... --out $OUT_DIR/certificates_NAME.jsonl \
#       --threads T --checkpoint-dir $CKPT/NAME --checkpoint-interval 300 --progress 60
#
# so each list has its own output file, its own checkpoint directory and its own logs (certify_NAME.out and .err).  T is
# the number after the colon; without one, the threads of the machine (THREADS, default every CPU) are shared equally
# among the lists.  The runbook has the commands of the phases (RUNBOOK.md, section 11): the pilot pilot7 on all
# threads, then dear_rest on all threads, then rest_1 rest_2 rest_3 rest_4 together on a quarter each, then deferred.
#
# Safe to run again at any time, after a crash, a reboot, a spot interruption or a SIGTERM: `taxman certify` skips the
# games its output file has a verdict for and resumes the game with a checkpoint, losing at most one checkpoint
# interval.  A second certify.sh while one runs is refused (the machine's threads are spoken for).  The threads, the
# lists' order and S3_DEST may differ between runs; the build must not (a checkpoint of another build is refused).
#
# What it does with the exit status of each `taxman certify`:
#   0        the list is done if every game has a verdict in its output; otherwise (a limit in EXTRA_ARGS) it goes on
#   2        a COUNTEREXAMPLE: a game that beats its recorded score (the result itself).  Everything stops at once, with
#            a final checkpoint, the game is in OUT.counterexample.jsonl, and the script ends with status 2.  Keep
#            the work directory as it is
#   130/143  stopped by SIGINT/SIGTERM (this script forwards them, once, to every process, which writes a final
#            checkpoint): the script ends
#   124      the deadline (DEADLINE_HOURS) stopped the process, with a final checkpoint
#   1        an error (a game that failed a check, a checkpoint of another build, ...): nothing is retried, the
#            other lists are stopped, and the script ends; read certify_NAME.err
#   other    a crash (the OOM killer): that list is run again after 10 s.  MAX_FAILS passes in a row of one list that
#            add no verdict and last under five minutes end the script
#
# When every list is done the script checks the output files with `taxman verify-certificates` (the barred bounds,
# the recorded games, the problem set up again, the games that rest on one another) and exits 0 only if that passes.
# The script exits 0 when complete, 2 for a counterexample, 143 when stopped, 3 for the deadline, 1 otherwise.
#
# Output (all in $OUT_DIR and $LOGS): certificates_NAME.jsonl per list; certify_NAME.out (the result line of each game)
# and certify_NAME.err (the progress lines) per list; certify_PHASE.log (this script: a status line every STATUS_EVERY
# seconds).  PHASE is the first list's name, with x and the number of lists if there are several (rest_1x4).
#
# Environment (all optional):
#   THREADS              threads to share among the lists without :THREADS, default every CPU
#   LISTS                where the lists are, default solver/tools/certify/lists/stage4
#   CERT_RESULTS         the recorded games, default results/strict.jsonl
#   CERT_UPPER           the prover's barred bounds, a list of files, default every solver/noframe/data/upper_*.jsonl
#   OUT_DIR              the output files, default $TAXMAN_WORK/results (the directory that save.sh copies)
#   CHECKPOINT_INTERVAL  seconds, default 300       PROGRESS  seconds between progress lines, default 60
#   STATUS_EVERY         seconds between status lines in certify_PHASE.log, default 600
#   EXTRA_ARGS           more flags for `taxman certify`, e.g. "--mem-cap 100000"
#   MAX_FAILS            default 5
#   DEADLINE_HOURS, POWEROFF, S3_DEST, SAVE_EVERY: as in run.sh, which this script and chain.sh share

set -u
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"
# shellcheck source=run.sh
. "$here/run.sh"

LISTS="${LISTS:-$TOOLS/certify/lists/stage4}"
CERT_RESULTS="${CERT_RESULTS:-$TAXMAN_ROOT/results/strict.jsonl}"
OUT_DIR="${OUT_DIR:-$TAXMAN_WORK/results}"
CHECKPOINT_INTERVAL="${CHECKPOINT_INTERVAL:-300}"
PROGRESS="${PROGRESS:-60}"
STATUS_EVERY="${STATUS_EVERY:-600}"
MAX_FAILS="${MAX_FAILS:-5}"
THREADS="${THREADS:-$(ncpu)}"

[ "$#" -ge 1 ] || die "usage: certify.sh LIST[:THREADS]...   (a list name: pilot7, dear_rest, rest_1, ..., deferred)"
[ -x "$TAXMAN_BIN" ] || die "$TAXMAN_BIN is missing: run setup.sh"
[ -r "$CERT_RESULTS" ] || die "$CERT_RESULTS is not readable"
if [ -n "${CERT_UPPER:-}" ]; then
    read -r -a upper_files <<<"$CERT_UPPER"
else
    upper_files=("$PROVER"/data/upper_*.jsonl)
fi
upper_args=()
for f in ${upper_files[@]+"${upper_files[@]}"}; do
    [ -r "$f" ] || die "$f is not readable (the barred bounds; CERT_UPPER names the files)"
    upper_args+=(--upper "$f")
done
case "$THREADS$CHECKPOINT_INTERVAL$PROGRESS$STATUS_EVERY$MAX_FAILS" in
    "" | *[!0-9]*) die "THREADS, CHECKPOINT_INTERVAL, PROGRESS, STATUS_EVERY and MAX_FAILS are whole numbers" ;;
esac

# --- the lists ---------------------------------------------------------------------------------------------------

# The boards of a list file, one a line, sorted; the boards with a final verdict (certified or beaten) in an output
# file; and how many boards of a list have one.  The fields of a line are in the order `taxman certify` writes them.
list_boards() {
    grep -E '^[[:space:]]*[0-9]+[[:space:]]*$' "$1" | tr -d ' \t' | LC_ALL=C sort -u
}
final_boards() {
    grep -E -o '^\{"n":[0-9]+,"score":[0-9]+,"verdict":"(certified|beaten)"' "$1" |
        sed -E 's/^\{"n":([0-9]+),.*/\1/' | LC_ALL=C sort -u
}
count_final() {
    if [ -r "$1" ]; then LC_ALL=C comm -12 <(list_boards "$2") <(final_boards "$1") | wc -l | tr -d ' '; else echo 0; fi
}

n_lists=$#
per_list=$((THREADS / n_lists))
[ "$per_list" -ge 1 ] || per_list=1
i=0
for arg in "$@"; do
    name="${arg%%:*}"
    threads="$per_list"
    case "$arg" in *:*) threads="${arg#*:}" ;; esac
    case "$threads" in "" | *[!0-9]* | 0) die "$arg: the threads after the colon are a whole number, at least 1" ;; esac
    if [ -r "$LISTS/$name.txt" ]; then
        file="$LISTS/$name.txt"
    elif [ -f "$name" ]; then
        file="$name"
    else
        die "$name is neither a list in $LISTS nor a file"
    fi
    short="$(basename "$file" .txt)"
    for ((j = 0; j < i; j++)); do
        [ "${L_name[$j]}" != "$short" ] || die "the list $short is given twice"
    done
    L_name[$i]="$short"
    L_file[$i]="$file"
    L_threads[$i]="$threads"
    L_out[$i]="$OUT_DIR/certificates_$short.jsonl"
    L_ckpt[$i]="$CKPT/$short"
    L_total[$i]="$(list_boards "$file" | wc -l | tr -d ' ')"
    [ "${L_total[$i]}" -ge 1 ] || die "$file lists no board"
    L_state[$i]=start # start | run | wait | done | end
    L_pid[$i]=""
    L_fails[$i]=0
    L_begun[$i]=0
    L_before[$i]=0
    L_retry[$i]=0
    i=$((i + 1))
done
PHASE="${L_name[0]}"
[ "$n_lists" -eq 1 ] || PHASE="${PHASE}x$n_lists"

# Two campaigns at once would share the threads of the machine twice over.
for pidfile in "$TAXMAN_WORK"/certify_*.pid; do
    [ -r "$pidfile" ] && kill -0 "$(cat "$pidfile")" 2>/dev/null && die "another certify.sh is running ($pidfile)"
done

run_begin "certify_$PHASE" "$OUT_DIR"
total_threads=0
summary=""
for ((i = 0; i < n_lists; i++)); do
    mkdir -p "${L_ckpt[$i]}"
    total_threads=$((total_threads + L_threads[i]))
    summary="$summary ${L_name[$i]} (${L_total[$i]} games, ${L_threads[$i]} threads);"
done
log "certify $PHASE:$summary results $CERT_RESULTS; output $OUT_DIR; checkpoints $CKPT"
env_report | tee -a "$LOGS/$RUN_NAME.log" >&2

# --- the supervisor ----------------------------------------------------------------------------------------------

found=""
failed=""
deadline_hit=""
halted="" # a counterexample or an error: the other processes have been asked to stop

start_list() {
    local i="$1" name="${L_name[$1]}"
    L_before[$i]="$(count_final "${L_out[$i]}" "${L_file[$i]}")"
    L_begun[$i]="$(date +%s)"
    if [ "${L_before[$i]}" -ge "${L_total[$i]}" ]; then
        log "$name: all ${L_total[$i]} games have a verdict already"
        L_state[$i]=done
        return 0
    fi
    log "$name: starting (${L_before[$i]} of ${L_total[$i]} games have a verdict)"
    # EXTRA_ARGS is a list of words on purpose.
    # shellcheck disable=SC2086
    run_start "$LOGS/certify_$name.out" "$LOGS/certify_$name.err" "$TAXMAN_BIN" certify --list "${L_file[$i]}" \
        --results "$CERT_RESULTS" ${upper_args[@]+"${upper_args[@]}"} --out "${L_out[$i]}" \
        --threads "${L_threads[$i]}" --checkpoint-dir "${L_ckpt[$i]}" --checkpoint-interval "$CHECKPOINT_INTERVAL" \
        --progress "$PROGRESS" ${EXTRA_ARGS:-}
    if [ -n "$launched" ]; then
        L_pid[$i]="$launched"
        L_state[$i]=run
    else
        handle_exit "$i" "$status" # the deadline has passed
    fi
}

# What a process's exit status means for its list.
handle_exit() {
    local i="$1" st="$2" name="${L_name[$1]}" done_now
    log "$name: certify exited with status $st after $(($(date +%s) - L_begun[i])) s"
    L_pid[$i]=""
    case "$st" in
        0)
            done_now="$(count_final "${L_out[$i]}" "${L_file[$i]}")"
            if [ "$done_now" -ge "${L_total[$i]}" ]; then
                log "$name: all ${L_total[$i]} games have a verdict"
                L_state[$i]=done
            else
                log "$name: certify ended with $done_now of ${L_total[$i]} games done (a limit in EXTRA_ARGS?);" \
                    "going on"
                L_state[$i]=start
            fi
            ;;
        2)
            log "COUNTEREXAMPLE: $name has a game that beats its recorded score.  Everything stops.  The game is in" \
                "${L_out[$i]%.jsonl}.counterexample.jsonl; the last lines of certify_$name.out:"
            tail -n 3 "$LOGS/certify_$name.out" >&2
            L_state[$i]=end
            found=1
            ;;
        124)
            log "$name: the deadline stopped it (a final checkpoint was written)"
            L_state[$i]=end
            deadline_hit=1
            ;;
        130 | 143)
            log "$name: stopped by a signal (a final checkpoint was written)"
            L_state[$i]=end
            ;;
        1)
            log "$name: certify reported an error; not retrying.  The last lines of certify_$name.err:"
            tail -n 5 "$LOGS/certify_$name.err" >&2
            L_state[$i]=end
            failed=1
            ;;
        *)
            log "$name: certify crashed (status $st)"
            # A pass that added a verdict, or ran for five minutes, made progress.
            if [ "$(count_final "${L_out[$i]}" "${L_file[$i]}")" -gt "${L_before[$i]}" ] ||
                [ "$(($(date +%s) - L_begun[i]))" -ge 300 ]; then
                L_fails[$i]=0
            else
                L_fails[$i]=$((L_fails[i] + 1))
            fi
            if [ "${L_fails[$i]}" -ge "$MAX_FAILS" ]; then
                log "$name: giving up after ${L_fails[$i]} passes in a row without progress"
                L_state[$i]=end
                failed=1
            else
                L_state[$i]=wait
                L_retry[$i]=$(($(date +%s) + 10))
            fi
            ;;
    esac
}

# One line a list: how many games have a verdict.
status_line() {
    local i line=""
    for ((i = 0; i < n_lists; i++)); do
        line="$line ${L_name[$i]} $(count_final "${L_out[$i]}" "${L_file[$i]}")/${L_total[$i]} (${L_state[$i]});"
    done
    log "status:$line"
}

# A signal, a counterexample or an error: the other processes are asked to stop, once, and nothing is started again.
# (A signal has been forwarded to them already, by run.sh.)
halt_if_needed() {
    local pid
    if [ -n "$stop_requested$found$failed" ] && [ -z "$halted" ]; then
        halted=1
        if [ -z "$stop_requested" ]; then
            for pid in $children; do
                kill -TERM "$pid" 2>/dev/null
            done
        fi
    fi
}

last_status="$(date +%s)"
while :; do
    halt_if_needed
    active=0
    for ((i = 0; i < n_lists; i++)); do
        case "${L_state[$i]}" in
            run)
                if run_reap "${L_pid[$i]}"; then
                    handle_exit "$i" "$status"
                    halt_if_needed
                fi
                ;;
            wait) [ "$(date +%s)" -lt "${L_retry[$i]}" ] || L_state[$i]=start ;;
        esac
        if [ "${L_state[$i]}" = start ]; then
            if [ -n "$halted" ] || [ -n "$deadline_hit" ]; then L_state[$i]=end; else start_list "$i"; fi
        fi
        case "${L_state[$i]}" in start | run | wait) active=1 ;; esac
    done
    [ "$active" = 1 ] || break
    if [ "$(($(date +%s) - last_status))" -ge "$STATUS_EVERY" ]; then
        status_line
        last_status="$(date +%s)"
    fi
    run_nap 2
done

complete=1
for ((i = 0; i < n_lists; i++)); do
    [ "${L_state[$i]}" = done ] || complete=""
done
if [ -n "$found" ]; then
    outcome="found"
elif [ -n "$failed" ]; then
    outcome="failed"
elif [ -n "$complete" ]; then
    outcome="complete"
elif [ -n "$stop_requested" ]; then
    outcome="stopped"
elif [ -n "$deadline_hit" ]; then
    outcome="deadline"
fi
status_line
log "ended: $outcome"

if [ "$outcome" = complete ]; then
    log "taxman verify-certificates over the output files"
    outs=()
    for ((i = 0; i < n_lists; i++)); do
        outs+=("${L_out[$i]}")
    done
    "$TAXMAN_BIN" verify-certificates "${outs[@]}" --results "$CERT_RESULTS" ${upper_args[@]+"${upper_args[@]}"} \
        --threads "$total_threads" 2>&1 | tee -a "$LOGS/$RUN_NAME.log" || outcome="failed"
    [ "$outcome" != failed ] || log "verify-certificates FAILED (see above)"
fi
case "$outcome" in
    complete) exit 0 ;;
    found) exit 2 ;;
    stopped) exit 143 ;;
    deadline) exit 3 ;;
    *) exit 1 ;;
esac
