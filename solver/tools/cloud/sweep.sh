#!/usr/bin/env bash
# The sweep: as many proven games as the time allows from a stretch of games, holes allowed, for little time and
# money.  Not a chain: the games are made in blocks of consecutive games, each block on one thread, and each game
# gets a limited search.
#
#   solver/tools/cloud/sweep.sh FIRST LAST
#
# Pass 1.  FIRST..LAST is split into BLOCKS blocks of consecutive games (default one a CPU), sized so that each block's
# expected work is about equal (solver/tools/sweep_plan.py: a game costs its setup, which grows steeply with n, and at most the
# limit of its search).  Each block is one process on one thread,
#
#   taxman solve A-B --threads 1 --time-limit $GAME_SECONDS --keep-going --pass 1 --out $BLOCK_DIR/block-NNN.jsonl
#
# in increasing n.  A game whose search finishes is a proof.  One that reaches the limit leaves an unfinished line with
# its best game, and the next game goes on from that game as a seed and nothing more: it gets the ceiling n + opt(n - 1),
# the prime formula and a repeat of an earlier search only from a game that is proven.  A block's first game has no game
# before it (it is solved standalone, no seed and no ceiling), except the first block's, which chains from KNOWN (a
# results file, default the committed chain) if that holds a proven game of FIRST - 1: the block's file starts as a
# copy of KNOWN's games up to it, as one extends a chain.
#
# Passes 2, 3, ...  A block's first game began without the game before it, which another block made.  Once the blocks
# are done they are merged, and then each pass searches again, with the same limit, every game that has only unfinished
# searches and whose game before it, as the merged file holds it, has more to give than it had when the game was last
# tried: it is proven (which gives the ceiling and perhaps the prime formula or a repeat), or it has a better best game
# than the seed was, or the game had none.  A pass runs one process a block that has such a game, each on one thread,
# from an excerpt of the merged file (solver/tools/sweep_plan.py), and goes through the block in increasing n, so that a game
# it has just proven helps the next (`taxman solve --keep-going --retry-improved`).  Passes repeat until one changes
# nothing (the games are then as an unbroken chain would have left them, whatever it could prove) or MAX_PASSES.
#
# Output.  The merged file $RESULTS (default $TAXMAN_WORK/results/explore.jsonl) holds every line of every block and
# pass, deduplicated; it is rebuilt from the block files, which only grow, every MERGE_EVERY seconds and when the script
# ends, and the merge is idempotent.  It is NOT a chain and never goes into results/optimal.jsonl.gz: it has holes
# (games that only an unfinished search is recorded for, or that no block reached), and the proofs in it rest only on
# the games it holds.  Both referees (verify.sh) accept it: each line is checked against what it rests on in the same
# file.  The line of a game says what it had from the game before it in its `prev` (null: nothing; no proof: the best
# game of an unfinished search, as a seed; a proof: a proven game) and the pass that made it in `options.pass`
# (solver/docs/REFERENCE.md, "What a game had from n - 1").
#
# Safe to run again at any time, after a crash, a reboot, a spot interruption or a SIGTERM: each block resumes its file
# (the solver skips the games it has a completed or unfinished line for), the games a stopped pass left are merged
# first, and the next pass plans from the merged file.  The games in progress are searched again from the start (there
# are no checkpoints: a game costs at most GAME_SECONDS), and a rerun with the same FIRST and LAST finds the plan it made
# ($BLOCK_DIR/plan; delete the directory to plan again).  A second sweep.sh while one runs is refused.
#
# What it does with the solver's exit status, per process:
#   0        the range is done
#   124      the deadline (DEADLINE_HOURS, BUDGET_HOURS or RUN_UNTIL) stopped it
#   130/143  stopped by SIGINT/SIGTERM (forwarded once): the game in progress has no line and the script ends
#   1, 2     an error (a game that failed a check, ...): everything is stopped and the script ends, nothing is retried
#   other    a crash: that process is run again after 10 s.  MAX_FAILS runs in a row that add no line and last under five
#            minutes end the script
# The script exits with 0 when every pass is done and both referees accept the merged file, 143 when stopped, 3 for the
# deadline, 1 otherwise.  The one line of $TAXMAN_WORK/sweep.outcome says how it ended.
#
# Environment (all optional):
#   BLOCKS         blocks, one thread each, default every CPU
#   GAME_SECONDS   the limit of one game's search, default 180
#   KNOWN          the results file the first block chains from, default the committed results/optimal.jsonl.gz
#   RESULTS        the merged file, default $TAXMAN_WORK/results/explore.jsonl; the block files are in
#                  RESULTS-minus-.jsonl-blocks/ beside it, which save.sh copies with it
#   MAX_PASSES     the most passes after the first in one run, default 20
#   PROGRESS       seconds between progress lines in each process's log (0: none), default 600
#   MERGE_EVERY    seconds between merges into $RESULTS, default 600       STATUS_EVERY  between status lines, default 600
#   EXTRA_ARGS     more flags for `taxman solve`, e.g. "--tactic gf2=off"
#   MAX_FAILS      default 5
#   DEADLINE_HOURS, BUDGET_HOURS, RUN_UNTIL, LEDGER_*, POWEROFF, S3_DEST, SAVE_EVERY: as in run.sh (chain.sh's)

set -u
set -o pipefail

# The directory save.sh copies is the merged file's, so that file and its blocks are saved.
: "${TAXMAN_WORK:=$HOME/taxman-work}"
: "${RESULTS:=$TAXMAN_WORK/results/explore.jsonl}"
export TAXMAN_WORK RESULTS

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"
# shellcheck source=run.sh
. "$here/run.sh"

first="${1:-}"
last="${2:-}"
case "$first$last" in
    '' | *[!0-9]*) die "usage: sweep.sh FIRST LAST   (the games to go over, FIRST..LAST, up to 8191)" ;;
esac
[ -n "$first" ] && [ -n "$last" ] || die "usage: sweep.sh FIRST LAST"
[ "$first" -ge 1 ] && [ "$first" -le "$last" ] && [ "$last" -le 8191 ] || die "FIRST..LAST must be within 1..8191"
[ -x "$TAXMAN_BIN" ] || die "$TAXMAN_BIN is missing: run setup.sh"
command -v python3 >/dev/null 2>&1 || die "python3 is needed (solver/tools/sweep_plan.py)"

BLOCKS="${BLOCKS:-$(ncpu)}"
GAME_SECONDS="${GAME_SECONDS:-180}"
KNOWN="${KNOWN:-${COMMITTED_RESULTS[0]}}"
MAX_PASSES="${MAX_PASSES:-20}"
PROGRESS="${PROGRESS:-600}"
MERGE_EVERY="${MERGE_EVERY:-600}"
STATUS_EVERY="${STATUS_EVERY:-600}"
MAX_FAILS="${MAX_FAILS:-5}"
case "$BLOCKS$MAX_PASSES$PROGRESS$MERGE_EVERY$STATUS_EVERY$MAX_FAILS" in
    "" | *[!0-9]*) die "BLOCKS, MAX_PASSES, PROGRESS, MERGE_EVERY, STATUS_EVERY and MAX_FAILS are whole numbers" ;;
esac
[ "$BLOCKS" -ge 1 ] || die "BLOCKS must be at least 1"
case "$GAME_SECONDS" in
    "" | *[!0-9.]* | *.*.* | .) die "GAME_SECONDS is a number of seconds" ;;
esac

BLOCK_DIR="$(dirname "$RESULTS")/$(basename "$RESULTS" .jsonl)-blocks"
PASS_DIR="$TAXMAN_WORK/sweep-pass"
PLAN="$BLOCK_DIR/plan"
PLAN_TOOL="$TOOLS/sweep_plan.py"
[ -r "$PLAN_TOOL" ] || die "$PLAN_TOOL is missing"

run_begin sweep "$(dirname "$RESULTS")" "$BLOCK_DIR"
rm -f "$TAXMAN_WORK/sweep.outcome"
set_outcome() {
    printf '%s\n' "$*" >"$TAXMAN_WORK/sweep.outcome"
}

# The number of complete lines of a file (0 if there is none yet).
line_count() {
    if [ -r "$1" ]; then wc -l <"$1" | tr -d ' '; else echo 0; fi
}

# --- the plan ----------------------------------------------------------------------------------------------------

mkdir -p "$BLOCK_DIR" "$PASS_DIR"
if [ -s "$PLAN" ]; then
    planned_first="$(head -n 1 "$PLAN" | cut -d ' ' -f 1)"
    planned_last="$(tail -n 1 "$PLAN" | cut -d ' ' -f 2)"
    [ "$planned_first" = "$first" ] && [ "$planned_last" = "$last" ] ||
        die "$PLAN is the plan for $planned_first..$planned_last, not $first..$last: use another RESULTS, or delete $BLOCK_DIR"
    log "the plan in $PLAN: $(wc -l <"$PLAN" | tr -d ' ') blocks (BLOCKS is not used)"
else
    python3 "$PLAN_TOOL" blocks "$first" "$last" "$BLOCKS" "$GAME_SECONDS" >"$PLAN.tmp" || die "cannot plan the blocks"
    mv "$PLAN.tmp" "$PLAN"
    log "planned $(wc -l <"$PLAN" | tr -d ' ') blocks of $first..$last (BLOCKS $BLOCKS, GAME_SECONDS $GAME_SECONDS)"
fi
n_blocks=0
while read -r lo hi; do
    B_lo[$n_blocks]="$lo"
    B_hi[$n_blocks]="$hi"
    B_file[$n_blocks]="$(printf '%s/block-%03d.jsonl' "$BLOCK_DIR" $((n_blocks + 1)))"
    n_blocks=$((n_blocks + 1))
done <"$PLAN"
[ "$n_blocks" -ge 1 ] || die "$PLAN is empty"

# The first block chains from KNOWN, if it has a proven game of FIRST - 1: its file starts as a copy of KNOWN's games up
# to it (once; the solver opens the file, checks it and goes on from the game of FIRST - 1).  Any other block, and the
# first without such a game, starts standalone.
seed_first_block() {
    local file="${B_file[0]}" before=$((first - 1))
    [ "$first" -ge 2 ] && [ ! -s "$file" ] || return 0
    # (KNOWN is a gzip file, as the committed one is, or (-f) a plain one.  grep -c, not -q: -q ends the pipe early,
    # and pipefail fails the pipe.)
    if [ ! -r "$KNOWN" ] ||
        [ "$(gzip -dcf "$KNOWN" | grep -E -c "^\{\"schema\":5,\"n\":$before,\"score\":[0-9]")" -eq 0 ]; then
        log "KNOWN ($KNOWN) has no proven game of $before: the first block starts standalone"
        return 0
    fi
    # (The n starts at the 17th character, after {"schema":5,"n": , and ends before the comma that RLENGTH counts.)
    gzip -dcf "$KNOWN" | awk -v last="$before" 'match($0, /^\{"schema":5,"n":[0-9]+,/) {
        if (substr($0, 17, RLENGTH - 17) + 0 <= last) print
    }' >"$file.tmp" && mv "$file.tmp" "$file"
    log "the first block chains from $KNOWN: its file starts as a copy of the games up to $before"
}
seed_first_block

# --- the jobs ----------------------------------------------------------------------------------------------------

# A job is one `taxman solve` process: J_lo..J_hi into J_out, with the flags of the pass (JOB_FLAGS, JOB_PASS).  J_skip
# is the number of lines J_out started with (an excerpt of the merged file): its lines after those are the job's own.
J_lo=()
J_hi=()
J_out=()
J_tag=()
J_skip=()
J_state=()
J_pid=()
J_fails=()
J_begun=()
J_before=()
J_retry=()
n_jobs=0
JOB_FLAGS=()
JOB_PASS=1

new_job() {
    J_lo[$n_jobs]="$1"
    J_hi[$n_jobs]="$2"
    J_out[$n_jobs]="$3"
    J_tag[$n_jobs]="$4"
    J_skip[$n_jobs]="$5"
    J_state[$n_jobs]=start # start | run | wait | done | end
    J_pid[$n_jobs]=""
    J_fails[$n_jobs]=0
    J_begun[$n_jobs]=0
    J_before[$n_jobs]=0
    J_retry[$n_jobs]=0
    n_jobs=$((n_jobs + 1))
}

# Appends to $RESULTS the complete lines the jobs have added to their files, but for those already in it: the merged
# file is rebuilt in a temporary file and moved into place, so that it is whole at every moment, and a line already in
# it is not added again, so that the merge can be run any number of times.
merge_jobs() {
    local tmp="$RESULTS.tmp.$$" i
    {
        [ ! -s "$RESULTS" ] || cat "$RESULTS"
        for ((i = 0; i < n_jobs; i++)); do
            [ -r "${J_out[$i]}" ] || continue
            # head -n of the count of newlines leaves out a last line that is still being written.
            head -n "$(line_count "${J_out[$i]}")" "${J_out[$i]}" | tail -n +"$((J_skip[i] + 1))"
        done
    } | awk '!seen[$0]++' >"$tmp" && mv "$tmp" "$RESULTS" || {
        rm -f "$tmp"
        log "could not merge into $RESULTS"
        return 1
    }
}

# The lines the jobs have added to their files.
added_lines() {
    local i total=0 count
    for ((i = 0; i < n_jobs; i++)); do
        count=$(($(line_count "${J_out[$i]}") - J_skip[i]))
        [ "$count" -le 0 ] || total=$((total + count))
    done
    echo "$total"
}

# The games a pass left when it was stopped (a crash of the machine, a deadline): their lines are good, and are merged
# before anything is planned.
merge_leftovers() {
    local dir file
    for dir in "$PASS_DIR"/pass-*; do
        [ -d "$dir" ] || continue
        n_jobs=0
        for file in "$dir"/job-*.jsonl; do
            [ -r "$file" ] || continue
            J_out[$n_jobs]="$file"
            J_skip[$n_jobs]=0
            [ ! -r "${file%.jsonl}.count" ] || J_skip[$n_jobs]="$(cat "${file%.jsonl}.count")"
            n_jobs=$((n_jobs + 1))
        done
        if [ "$n_jobs" -gt 0 ]; then
            log "merging the games an interrupted pass left in $dir ($(added_lines) lines)"
            merge_jobs
        fi
        rm -rf "$dir"
    done
    n_jobs=0
}

found=""
failed=""
deadline_hit=""
halted="" # an error: the other processes have been asked to stop

start_job() {
    local i="$1" tag="${J_tag[$1]}"
    J_before[$i]="$(line_count "${J_out[$i]}")"
    J_begun[$i]="$(date +%s)"
    log "$tag: starting ${J_lo[$i]}-${J_hi[$i]} (pass $JOB_PASS)"
    # EXTRA_ARGS is a list of words on purpose.
    # shellcheck disable=SC2086
    run_start "$LOGS/sweep-$tag.out" "$LOGS/sweep-$tag.err" "$TAXMAN_BIN" solve "${J_lo[$i]}-${J_hi[$i]}" --threads 1 \
        --time-limit "$GAME_SECONDS" --keep-going ${JOB_FLAGS[@]+"${JOB_FLAGS[@]}"} --pass "$JOB_PASS" \
        --out "${J_out[$i]}" --progress "$PROGRESS" ${EXTRA_ARGS:-}
    if [ -n "$launched" ]; then
        J_pid[$i]="$launched"
        J_state[$i]=run
    else
        handle_exit "$i" "$status" # the deadline has passed
    fi
}

# What a process's exit status means for its job.
handle_exit() {
    local i="$1" st="$2" tag="${J_tag[$1]}"
    log "$tag: solve exited with status $st after $(($(date +%s) - J_begun[i])) s"
    J_pid[$i]=""
    case "$st" in
        0) J_state[$i]=done ;;
        124)
            log "$tag: the deadline stopped it"
            J_state[$i]=end
            deadline_hit=1
            ;;
        130 | 143)
            log "$tag: stopped by a signal"
            J_state[$i]=end
            ;;
        1 | 2)
            log "$tag: solve reported an error; not retrying.  The last lines of sweep-$tag.err:"
            tail -n 5 "$LOGS/sweep-$tag.err" >&2
            J_state[$i]=end
            failed=1
            ;;
        *)
            log "$tag: solve crashed (status $st)"
            # A run that added a line, or ran for five minutes, made progress.
            if [ "$(line_count "${J_out[$i]}")" -gt "${J_before[$i]}" ] || [ "$(($(date +%s) - J_begun[i]))" -ge 300 ]; then
                J_fails[$i]=0
            else
                J_fails[$i]=$((J_fails[i] + 1))
            fi
            if [ "${J_fails[$i]}" -ge "$MAX_FAILS" ]; then
                log "$tag: giving up after ${J_fails[$i]} runs in a row without progress"
                J_state[$i]=end
                failed=1
            else
                J_state[$i]=wait
                J_retry[$i]=$(($(date +%s) + 10))
            fi
            ;;
    esac
}

# An error: the other processes are asked to stop, once, and nothing is started again.  (A signal has been forwarded to
# them already, by run.sh.)
halt_if_needed() {
    local pid
    if [ -n "$stop_requested$failed" ] && [ -z "$halted" ]; then
        halted=1
        if [ -z "$stop_requested" ]; then
            for pid in $children; do
                kill -TERM "$pid" 2>/dev/null
            done
        fi
    fi
}

# A line for the log: how the jobs stand.
status_line() {
    local i running=0 finished=0
    for ((i = 0; i < n_jobs; i++)); do
        case "${J_state[$i]}" in
            done) finished=$((finished + 1)) ;;
            run | start | wait) running=$((running + 1)) ;;
        esac
    done
    log "status: pass $JOB_PASS, $finished of $n_jobs jobs done, $running running; $(added_lines) lines added;" \
        "the merged file has $(line_count "$RESULTS") lines"
}

# Runs the jobs set up (new_job) until each is done or ended, merging into $RESULTS now and then.  Returns 0 if every
# job is done.
run_jobs() {
    local i active last_merge last_status
    last_merge="$(date +%s)"
    last_status="$last_merge"
    while :; do
        halt_if_needed
        active=0
        for ((i = 0; i < n_jobs; i++)); do
            case "${J_state[$i]}" in
                run)
                    if run_reap "${J_pid[$i]}"; then
                        handle_exit "$i" "$status"
                        halt_if_needed
                    fi
                    ;;
                wait) [ "$(date +%s)" -lt "${J_retry[$i]}" ] || J_state[$i]=start ;;
            esac
            if [ "${J_state[$i]}" = start ]; then
                if [ -n "$halted" ] || [ -n "$deadline_hit" ]; then J_state[$i]=end; else start_job "$i"; fi
            fi
            case "${J_state[$i]}" in start | run | wait) active=1 ;; esac
        done
        [ "$active" = 1 ] || break
        if [ "$(($(date +%s) - last_merge))" -ge "$MERGE_EVERY" ]; then
            merge_jobs
            last_merge="$(date +%s)"
        fi
        if [ "$(($(date +%s) - last_status))" -ge "$STATUS_EVERY" ]; then
            status_line
            last_status="$(date +%s)"
        fi
        run_nap 2
    done
    merge_jobs
    for ((i = 0; i < n_jobs; i++)); do
        [ "${J_state[$i]}" = done ] || return 1
    done
    return 0
}

# --- the passes --------------------------------------------------------------------------------------------------

log "sweep $first..$last: $n_blocks blocks, $GAME_SECONDS s a game, one thread a block; merged file $RESULTS;" \
    "blocks in $BLOCK_DIR; known $KNOWN"
env_report | tee -a "$LOGS/sweep.log" >&2

merge_leftovers

# Pass 1: the blocks.  A block whose file is complete returns at once (the solver skips its games).
JOB_PASS=1
JOB_FLAGS=()
n_jobs=0
for ((b = 0; b < n_blocks; b++)); do
    new_job "${B_lo[$b]}" "${B_hi[$b]}" "${B_file[$b]}" "$(printf 'b%03d' $((b + 1)))" 0
done
pass_ok=1
run_jobs || pass_ok=""
status_line
log "pass 1 ended; the merged file has $(line_count "$RESULTS") lines"

# Passes 2 and on: the games that have more to give than they had.
passes_made=0
if [ -n "$pass_ok" ]; then
    last_pass="$(cat "$BLOCK_DIR/passes" 2>/dev/null || echo 1)"
    while [ "$passes_made" -lt "$MAX_PASSES" ]; do
        JOB_PASS=$((last_pass + 1))
        dir="$PASS_DIR/pass-$JOB_PASS"
        rm -rf "$dir"
        mkdir -p "$dir"
        n_jobs=0
        JOB_FLAGS=(--retry-improved)
        # One reading of the merged file cuts the excerpt of every block that has a game to search again.
        planned="$(python3 "$PLAN_TOOL" passes "$RESULTS" "$PLAN" "$dir")" || die "cannot plan pass $JOB_PASS"
        while read -r index lo hi lines todo; do
            [ -n "${index:-}" ] || continue
            new_job "$lo" "$hi" "$dir/job-$index.jsonl" "$(printf 'p%d-b%s' "$JOB_PASS" "$index")" "$lines"
        done <<<"$planned"
        if [ "$n_jobs" -eq 0 ]; then
            log "pass $JOB_PASS: no game has more to give than it had: done"
            rm -rf "$dir"
            break
        fi
        echo "$JOB_PASS" >"$BLOCK_DIR/passes"
        last_pass="$JOB_PASS"
        passes_made=$((passes_made + 1))
        log "pass $JOB_PASS: $n_jobs blocks have a game to search again"
        run_jobs || pass_ok=""
        status_line
        log "pass $JOB_PASS added $(added_lines) lines; the merged file has $(line_count "$RESULTS")"
        rm -rf "$dir"
        n_jobs=0
        [ -n "$pass_ok" ] || break
    done
    if [ -n "$pass_ok" ] && [ "$passes_made" -ge "$MAX_PASSES" ]; then
        log "MAX_PASSES ($MAX_PASSES) passes made: stopping here"
    fi
fi

# --- the end -----------------------------------------------------------------------------------------------------

if [ -n "$failed" ]; then
    outcome="failed"
    set_outcome "failed: a solver reported an error (see sweep-*.err)"
elif [ -n "$stop_requested" ]; then
    outcome="stopped"
    set_outcome "stopped by a signal"
elif [ -n "$deadline_hit" ]; then
    outcome="deadline"
    set_outcome "deadline: $deadline_reason"
elif [ -n "$pass_ok" ]; then
    outcome="complete"
    set_outcome "complete: $first..$last in $n_blocks blocks, $((passes_made + 1)) passes"
else
    outcome="failed"
    set_outcome "failed: a job did not finish"
fi
log "ended: $outcome"
if [ "$outcome" = complete ]; then
    if ! "$here/verify.sh" "$RESULTS" 2>&1 | tee -a "$LOGS/verify.log"; then
        outcome="failed"
        set_outcome "failed: verify.sh found a problem (see verify.log)"
    fi
fi
case "$outcome" in
    complete) exit 0 ;;
    stopped) exit 143 ;;
    deadline) exit 3 ;;
    *) exit 1 ;;
esac
