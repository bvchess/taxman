#!/usr/bin/env bash
# Tests of the scripts of a run that need no cloud: chain.sh with the date, the ledger and the lock (run.sh), and
# status.sh, against a stand-in for the aws CLI (test/bin/aws: S3 as a directory) and for ssh (test/bin/ssh: the machine
# is this one).  The scenarios that run the chain (chain, ledger) need the release build of the solver
# (cd solver/frame && cargo build --release; or TAXMAN_BIN names another build of the same sources, say the debug build
# that `cargo build` makes) and rsync; lock and status need neither.
#
#   solver/tools/cloud/test/run.sh [SCENARIO...]   all of them, or the named (chain, ledger, lock, status)
#
# Prints `ok` or `FAIL` for each check and exits with the number of failures.  A few minutes: the chains are small.

set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
real_cloud="$(cd "$here/.." && pwd)"
crate="$(cd "$real_cloud/../../frame" && pwd)"
export TAXMAN_BIN="${TAXMAN_BIN:-$crate/target/release/taxman}"
selected=("$@")
[ "${#selected[@]}" -gt 0 ] || selected=(chain ledger lock status)
for s in "${selected[@]}"; do
    case "$s" in
        chain | ledger)
            [ -x "$TAXMAN_BIN" ] || {
                echo "build the solver first: cd $crate && cargo build --release" >&2
                exit 2
            }
            command -v rsync >/dev/null 2>&1 || {
                echo "rsync is needed (the stand-in for aws s3 sync)" >&2
                exit 2
            }
            ;;
    esac
done

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"; kill $(jobs -p) 2>/dev/null' EXIT
# The scripts under test are a copy of the directory's, but for verify.sh, which replays the committed games
# (half a minute) every time a chain completes.
cloud="$tmp/cloud"
mkdir -p "$cloud"
cp "$real_cloud"/*.sh "$cloud/"
printf '#!/usr/bin/env bash\necho "verify.sh (a stand-in): $*"\n' >"$cloud/verify.sh"
chmod +x "$cloud"/*.sh
export TAXMAN_ROOT="$(cd "$crate/../.." && pwd)"
export PATH="$here/bin:$PATH"
export FAKE_S3="$tmp/s3" FAKE_LOG="$tmp/aws.log"
export AWS_DEFAULT_REGION=us-east-1
mkdir -p "$FAKE_S3"
failures=0

check() {
    local what="$1"
    shift
    if "$@"; then
        printf 'ok   %s\n' "$what"
    else
        printf 'FAIL %s\n' "$what"
        failures=$((failures + 1))
    fi
}

# The object NAME of the fake bucket b.
obj() {
    printf '%s/b/%s\n' "$FAKE_S3" "$1"
}

# Whether the file $1 holds the line (fixed text) $2 / the pattern $2.
has_line() { grep -qxF -- "$2" "$1" 2>/dev/null; }
matches() { grep -q -- "$2" "$1" 2>/dev/null; }
lacks() { [ -e "$1" ] && ! grep -q -- "$2" "$1"; }
exists() { [ -e "$1" ]; }
absent() { [ ! -e "$1" ]; }
equals() { [ "$1" = "$2" ]; }

# The seconds the fake machine has been up and its boot, which run.sh reads from these.
machine() {
    export UPTIME_SECONDS="$1" BOOT_ID="$2"
}

# --- chain.sh: the date ------------------------------------------------------------------------------------------

scenario_chain() {
    echo "== chain.sh: the date"
    # A date already within the margin: the deadline has passed before the solver starts.
    (
        export TAXMAN_WORK="$tmp/chain-date" RUN_UNTIL=$(($(date +%s) + 100)) THREADS=2 PROGRESS=0
        "$cloud/chain.sh" 50 >"$tmp/chain3.out" 2>&1
        echo $? >"$tmp/chain3.status"
    )
    check "a date within the margin ends the chain with the deadline's status 3" has_line "$tmp/chain3.status" 3
    check "and the outcome names the date" matches "$tmp/chain-date/chain.outcome" '^deadline: the date 20'
}

# --- the ledger: the budget across machines ----------------------------------------------------------------------

scenario_ledger() {
    echo "== run.sh: the ledger in S3"
    local ledger
    ledger="$(obj t1/run/ledger)"
    : >"$FAKE_LOG"
    # The first machine, up half an hour: base 0.
    (
        machine 1800 boot-a
        export TAXMAN_WORK="$tmp/m1" S3_DEST=s3://b/t1 BUDGET_HOURS=10 THREADS=2 PROGRESS=0
        "$cloud/chain.sh" 60 >"$tmp/m1.out" 2>&1
        echo $? >"$tmp/m1.status"
    )
    check "the first machine completes the chain" has_line "$tmp/m1.status" 0
    check "the ledger has the run's start" matches "$ledger" '^start=[0-9]\{10\}$'
    check "and its boot" has_line "$ledger" boot=boot-a
    check "and base 0 and half an hour" has_line "$ledger" base=0
    check "and its uptime in hours" has_line "$ledger" hours=0.5000
    check "the results are in S3" matches "$(obj t1/results/strict.jsonl)" '"n":60,'
    local start
    start="$(sed -n 's/^start=//p' "$ledger")"

    # A replacement machine, up ten minutes: the first one's half hour is its base.
    (
        machine 600 boot-b
        export TAXMAN_WORK="$tmp/m2" S3_DEST=s3://b/t1 BUDGET_HOURS=10 THREADS=2 PROGRESS=0
        mkdir -p "$tmp/m2"
        "$cloud/save.sh" --restore >/dev/null 2>&1
        "$cloud/chain.sh" 60 >"$tmp/m2.out" 2>&1
        echo $? >"$tmp/m2.status"
    )
    check "the replacement finds the chain done" has_line "$tmp/m2.status" 0
    check "the run's start is kept" has_line "$ledger" "start=$start"
    check "the replacement's base is the first machine's hours" has_line "$ledger" base=0.5000
    check "and the hours are the base and its own" has_line "$ledger" hours=0.6667

    # The same machine again (the same boot): its hours are not counted twice.
    (
        machine 3600 boot-b
        export TAXMAN_WORK="$tmp/m2" S3_DEST=s3://b/t1 BUDGET_HOURS=10 THREADS=2 PROGRESS=0
        "$cloud/chain.sh" 60 >"$tmp/m2b.out" 2>&1
    )
    check "a second script of the same boot keeps the base" has_line "$ledger" base=0.5000
    check "and the hours are base plus the boot's uptime" has_line "$ledger" hours=1.5000

    # A budget the earlier machines used up: nothing is started.
    (
        machine 600 boot-c
        export TAXMAN_WORK="$tmp/m3" S3_DEST=s3://b/t1 BUDGET_HOURS=1.5 THREADS=2 PROGRESS=0
        mkdir -p "$tmp/m3"
        "$cloud/save.sh" --restore >/dev/null 2>&1
        "$cloud/chain.sh" 60 >"$tmp/m3.out" 2>&1
        echo $? >"$tmp/m3.status"
    )
    check "a spent budget ends the chain with status 3" has_line "$tmp/m3.status" 3
    check "the outcome names the budget" matches "$tmp/m3/chain.outcome" '^deadline: the budget of 1.5 instance-hours$'
    check "the log says the deadline had passed" matches "$tmp/m3/logs/chain.log" \
        'the deadline has passed: the budget of 1.5 instance-hours'
    check "and the solver did not run" absent "$tmp/m3/logs/chain.out"

    # A ledger that cannot be read stops the machine: it must not run uncounted.
    (
        machine 600 boot-d
        export TAXMAN_WORK="$tmp/m4" S3_DEST=s3://b/t1 BUDGET_HOURS=10 FAKE_FAIL=nothing THREADS=2
        printf 'start=soon\nhours=lots\n' >"$ledger"
        "$cloud/chain.sh" 60 >"$tmp/m4.out" 2>&1
        echo $? >"$tmp/m4.status"
    )
    check "a ledger with no numbers stops the machine" has_line "$tmp/m4.status" 1
    check "and says so" matches "$tmp/m4.out" 'has no number for'
}

# --- the lock of run.sh ------------------------------------------------------------------------------------------

# run_begin keeps its lock on file descriptor 9 for as long as the script runs, and the lock is the script's alone: a
# solver that held it, or a program the solver starts and that outlives it, would keep a new chain.sh out.  A stand-in
# solver records whether fd 9 is open in it.  Where there is no flock (macOS), a stand-in that takes no lock is put in
# its place: run_begin opens fd 9 all the same.
scenario_lock() {
    echo "== run.sh: the lock is the script's alone"
    local w="$tmp/lock" bin="$tmp/lock-bin"
    mkdir -p "$bin"
    printf '%s\n' '#!/usr/bin/env bash' \
        '[ "${1:-}" != verify-results ] || { echo "n = 1..=5 (0 missing)"; exit 0; }' \
        'if { true >&9; } 2>/dev/null; then echo open; else echo closed; fi >"$TAXMAN_WORK/solver.fd9"' >"$bin/solver"
    command -v flock >/dev/null 2>&1 || printf '#!/bin/sh\nexit 0\n' >"$bin/flock"
    chmod +x "$bin"/*
    (
        export TAXMAN_WORK="$w" TAXMAN_BIN="$bin/solver" PATH="$bin:$PATH" THREADS=1 PROGRESS=0
        "$cloud/chain.sh" 5 >"$tmp/lock.out" 2>&1
        echo $? >"$tmp/lock.status"
    )
    check "a chain with the stand-in solver completes" has_line "$tmp/lock.status" 0
    check "the solver that run_launch starts does not hold fd 9, the lock" has_line "$w/solver.fd9" closed
}

# --- status.sh ---------------------------------------------------------------------------------------------------

# The solver's line for a game it recorded (solver.rs, summary_line): recorded_line N SCORE METHOD NODES SECONDS OPTIMAL
# PROOF [MORE], where OPTIMAL is the word after `optimal` (ok or unknown) and MORE is any text after the proof: what the
# proof assumes, and the fields that end the line, `  own=3011.250 s  best=341541` (a line of an older solver has
# neither).
recorded_line() {
    printf 'n=%-5d score=%-9d %-7s nodes=%-11d %9.3f s  optimal %s  proof=%s%s\n' "$1" "$2" "$3" "$4" "$5" "$6" "$7" \
        "${8:-}"
}

# The games of the table in the output $1 (normalized), in the order printed, as one line: "35 36 ... ".
table_games() {
    sed -n 's/^ \([0-9][0-9]*\) .*/\1/p' "$1" | tr '\n' ' '
}

# A progress line of the solver (engine.rs, progress_line): the game, the time since its search began, the nodes, the
# rate or how the search ended, and the best sum of promotions.
progress() {
    printf 'n=%s %s nodes %s (%s) best %s queue 3 spilled 0 | eff 99.9%% idle 0.0%% lock 0.0%% splits 1 steals 0 %s\n' \
        "$1" "$2" "$3" "$4" "$5" 'full 0.3ms ramp 0.0% tail 0.0% rebuild 0.0% first 0.0%'
}

# Sets the modification time of the file $1 to the epoch time $2 (touch -t reads UTC here).
stamp() {
    TZ=UTC touch -t "$(date -u -d "@$2" +%Y%m%d%H%M.%S 2>/dev/null || date -u -r "$2" +%Y%m%d%H%M.%S)" "$1"
}

# A time in seconds since the epoch in the zone TZ, as status prints it.
local_text() {
    date -d "@$1" '+%Y-%m-%d %H:%M %Z' 2>/dev/null || date -r "$1" '+%Y-%m-%d %H:%M %Z'
}

# A chain on the fake host, whose home directory $FAKE_HOME is that of the stand-in for ssh, with its data folder
# $work: a new chain.log for the range 1-LAST (LAST) and a window of 5, and nothing else.
fake_chain() {
    rm -rf "$work"
    mkdir -p "$work/results" "$work/logs" "$work/ckpt"
    printf '%s chain 1-%s; threads: all available; window 5; results x; checkpoints y\n' 2026-10-21T00:00:00Z "$1" \
        >"$work/logs/chain.log"
}

# The results file of the fake host recorded through the game $1 (0: none), and a line of a game not completed after it.
fake_results() {
    local n
    {
        for ((n = 1; n <= $1; n++)); do
            printf '{"schema":5,"n":%d,"score":%d,"moves":[%d],"completed":true}\n' "$n" $((n * 10)) "$n"
        done
        printf '{"schema":5,"n":%d,"score":null,"moves":[],"completed":false}\n' $(($1 + 1))
    } >"$work/results/strict.jsonl"
}

# The solver's line for each of the games 1..LAST that it recorded, in logs/chain.out of the fake host.
fake_recorded() {
    local n
    for ((n = 1; n <= $1; n++)); do
        recorded_line "$n" $((n * 10)) search 0 1 ok exhausted
    done >"$work/logs/chain.out"
}

scenario_status() {
    echo "== status.sh: a chain on another machine, over ssh"
    local status="$cloud/status.sh" host=user@fake.example now saved calls value
    local work="$tmp/home/taxman_work/chain-data"
    export FAKE_HOME="$tmp/home"
    now="$(date +%s)"
    : >"$FAKE_LOG"

    # The chain: recorded through 40, and its window of 5 holds 41..45.  The games 35 to 40 are of every kind; a
    # replacement machine printed 37 and 38 again after an interruption (the first lines of 37 and 38 are the earlier
    # machine's); the search of 41 was stopped, and its line is incomplete.  The lines of 35 to 37, 39 and 40 end with
    # the own search time and the best sum, as a solver prints them now; those of 38 and the earlier line of 41 are an
    # older solver's, with neither.  The progress lines (the oldest game, 41, runs; 42 is suspended; 43's search is
    # done; 44 has no line; 45's only line is from before three of 41's) and the clock of 41, which is the oldest game
    # with its search running.
    fake_chain 100
    fake_results 40
    fake_recorded 34
    {
        recorded_line 35 350 search 1133462769 4636.026 unknown exhausted "  own=3011.250 s  best=341541"
        recorded_line 36 360 repeat 0 0.240 unknown "same_search of 34" "  own=0.000 s  best=34"
        recorded_line 37 370 search 7 123.456 unknown exhausted
        recorded_line 38 380 search 9 1.5 unknown exhausted
        recorded_line 37 370 prime 0 0.055 unknown prime_formula "  own=0.000 s  best=-"
        recorded_line 38 380 known 0 0.003 ok "known from the committed chain"
        recorded_line 39 7466640 search 0 0.247 unknown cap "  assumes statement U  own=0.247 s  best=566640"
        recorded_line 40 400 search 248002917 812.5 unknown exhausted "  own=697.250 s  best=40000"
        printf 'n=41    score=410       search  nodes=1000000000  6000.000 s  incomplete\n'
        printf 'n=41    score=410       search  nodes=1000000000  6000.000 s  incomplete  own=5000.000 s  best=410\n'
    } >>"$work/logs/chain.out"
    {
        progress 40 9m00s 100 done 1
        echo 'note: a line of the solver that is not a progress line'
        progress 45 2m00s 63 240.0k/s 500
        progress 41 1h38m00s 900000000 250.0k/s 341541
        progress 42 1h08m00s 63 0/s 341541
        progress 43 30m00s 4000 230.0k/s 777
        progress 41 1h39m00s 925000000 250.0k/s 341541
        progress 42 1h09m00s 63 0/s 341541
        progress 43 30m30s 5000 done 777
        progress 41 1h40m00s 1000000000 250.0k/s 341541
        progress 42 1h10m00s 63 0/s 341541
    } >"$work/logs/chain.err"
    printf '4500.036\n' >"$work/ckpt/n0041.clock"
    # The last progress line, 261 seconds ago.
    stamp "$work/logs/chain.err" $((now - 261))
    saved="$(local_text $((now - 261)))"

    TZ=America/Los_Angeles "$status" --host "$host" >"$tmp/s1.out" 2>&1
    calls="$?,$(wc -l <"$FAKE_LOG" | tr -d ' ')"
    sed 's/  */ /g' "$tmp/s1.out" >"$tmp/s1.norm"
    check "status succeeds with no AWS profile and no aws call" equals "$calls" 0,0
    check "it names the host" has_line "$tmp/s1.out" "host $host"
    check "it shows the uptime of the machine" matches "$tmp/s1.out" ' up '
    check "it has the window and the last game of the range from chain.log" has_line "$tmp/s1.norm" \
        ' chain 1-100, window 5'
    check "it is recorded through the last game completed of the results file, not of chain.out" has_line \
        "$tmp/s1.norm" ' recorded through 40 (score 400)'
    check "the last progress line is in Pacific time, with how long ago" matches "$tmp/s1.norm" \
        "^ last progress line: $saved, 4 min [0-9]* s ago$"
    check "the oldest game runs, with its clock" has_line "$tmp/s1.norm" \
        ' 41 1h15m00s 1,000,000,000 341,541 running, 250.0k nodes/s'
    check "a game whose threads are another's is suspended" has_line "$tmp/s1.norm" \
        ' 42 - 63 341,541 suspended'
    check "a game whose search is done waits for the game before" has_line "$tmp/s1.norm" \
        ' 43 - 5,000 777 search done, waiting for the game before'
    check "a game with no line has no state to tell" has_line "$tmp/s1.norm" ' 44 - - - no recent progress line'
    check "and a game whose last line three of another's follow" has_line "$tmp/s1.norm" \
        ' 45 - - - no recent progress line'
    check "the table has the last 6 games recorded, then the window after them, in order" equals \
        "$(table_games "$tmp/s1.norm")" '35 36 37 38 39 40 41 42 43 44 45 '
    check "its heading says what it holds" has_line "$tmp/s1.norm" \
        ' the last 6 games recorded and the 5 in flight (window 5):'
    check "one header line of columns stands over both, with no elapsed time" equals \
        "$(grep -c '^ n own search nodes best state$' "$tmp/s1.norm")" 1
    check "a searched game has its nodes, its own search time (50m11s, not the 1h17m16s its line counts) and its best" \
        has_line "$tmp/s1.norm" ' 35 50m11s 1,133,462,769 341,541 recorded, score 350 (search, exhausted)'
    check "a repeat has the game whose search it repeats, and a game that ran no search has no time" has_line \
        "$tmp/s1.norm" ' 36 0.0s 0 34 recorded, score 360 (repeat, same_search of 34)'
    check "a game printed again is the last of its lines: a prime game, with no best sum" has_line "$tmp/s1.norm" \
        ' 37 0.0s 0 - recorded, score 370 (prime, prime_formula)'
    check "a line of an older solver has no own search time or best sum, and a known game has the words of its proof" \
        has_line "$tmp/s1.norm" \
        ' 38 - 0 - recorded, score 380 (known, known from the committed chain, optimal ok)'
    check "the proof ends at what the solver adds to it, and the fields come after it" has_line "$tmp/s1.norm" \
        ' 39 0.2s 0 566,640 recorded, score 7,466,640 (search, cap)'
    check "a recorded game's own search time is formatted as an in-flight game's, its best with commas" has_line \
        "$tmp/s1.norm" ' 40 11m37s 248,002,917 40,000 recorded, score 400 (search, exhausted)'
    check "an incomplete line, with the fields or without, is not a recorded game" lacks "$tmp/s1.norm" 'score 410'
    check "no column has the time a game waited" lacks "$tmp/s1.norm" 'elapsed'
    check "none is in UTC" lacks "$tmp/s1.out" 'UTC\|[0-9]Z'

    # The same time in other zones: the progress file was last written on 2026-01-15 at 03:30:00 UTC, the evening before
    # at Pacific time (standard time).
    stamp "$work/logs/chain.err" 1768447800
    TZ=America/Los_Angeles "$status" --host "$host" >"$tmp/s2.out" 2>&1
    sed 's/  */ /g' "$tmp/s2.out" >"$tmp/s2.norm"
    check "in Pacific standard time, with how long ago" matches "$tmp/s2.norm" \
        '^ last progress line: 2026-01-14 19:30 PST, [0-9]* d [0-9]* h ago$'
    TZ=Asia/Kolkata "$status" --host "$host" >"$tmp/s3.out" 2>&1
    check "in India it is the 15th, in IST" matches "$tmp/s3.out" 'last progress line: 2026-01-15 09:00 IST, '
    TZ=UTC "$status" --host "$host" >"$tmp/s4.out" 2>&1
    check "in UTC it says UTC" matches "$tmp/s4.out" 'last progress line: 2026-01-15 03:30 UTC, '
    stamp "$work/logs/chain.err" $((now - 261))

    # The oldest game's search is done (it is being recorded), and a stopped search.
    {
        progress 41 1h41m00s 1001000000 done 341541
        progress 44 5m00s 70 stopped 9
    } >>"$work/logs/chain.err"
    TZ=America/Los_Angeles "$status" --host "$host" >"$tmp/s5.out" 2>&1
    sed 's/  */ /g' "$tmp/s5.out" >"$tmp/s5.norm"
    check "the oldest game, searched, is being recorded" has_line "$tmp/s5.norm" \
        ' 41 1h15m00s 1,001,000,000 341,541 search done, being recorded'
    check "a suspended game is still current after the oldest game's last line" has_line "$tmp/s5.norm" \
        ' 42 - 63 341,541 suspended'
    check "a stopped search says so" has_line "$tmp/s5.norm" \
        ' 44 - 70 9 search stopped, its checkpoint kept'

    # --recorded N: how many of the games recorded the table shows, before the host or after it.
    TZ=America/Los_Angeles "$status" --host "$host" --recorded 2 >"$tmp/r1.out" 2>&1
    check "--recorded 2 succeeds" equals $? 0
    sed 's/  */ /g' "$tmp/r1.out" >"$tmp/r1.norm"
    check "it shows the last 2 games recorded, then those in flight" equals "$(table_games "$tmp/r1.norm")" \
        '39 40 41 42 43 44 45 '
    check "the heading counts them" has_line "$tmp/r1.norm" \
        ' the last 2 games recorded and the 5 in flight (window 5):'
    check "the line of the last game recorded stays" has_line "$tmp/r1.norm" ' recorded through 40 (score 400)'
    "$status" --recorded 2 --host "$host" >"$tmp/r2.out" 2>&1
    sed 's/  */ /g' "$tmp/r2.out" >"$tmp/r2.norm"
    check "the option may come before the host" equals "$(table_games "$tmp/r2.norm")" '39 40 41 42 43 44 45 '
    "$status" --host "$host" --recorded 0 >"$tmp/r3.out" 2>&1
    check "--recorded 0 succeeds" equals $? 0
    sed 's/  */ /g' "$tmp/r3.out" >"$tmp/r3.norm"
    check "it leaves the recorded games out" equals "$(table_games "$tmp/r3.norm")" '41 42 43 44 45 '
    check "and the heading is the games in flight's" has_line "$tmp/r3.out" '  games in flight (window 5):'
    check "no row says recorded" lacks "$tmp/r3.out" 'recorded, score'
    "$status" --host "$host" --recorded 100 >"$tmp/r4.out" 2>&1
    sed 's/  */ /g' "$tmp/r4.out" >"$tmp/r4.norm"
    check "more than the games recorded shows the 40 there are" equals "$(table_games "$tmp/r4.norm")" \
        "$(seq 1 45 | tr '\n' ' ')"
    check "and the heading counts the 40" has_line "$tmp/r4.norm" \
        ' the last 40 games recorded and the 5 in flight (window 5):'
    for value in x 2.5 -1 ''; do
        "$status" --host "$host" --recorded "$value" >"$tmp/r5.out" 2>&1
        check "--recorded \"$value\" is refused" equals $? 1
        check "with the words for it" matches "$tmp/r5.out" "error: --recorded: \"$value\" is not a whole number"
    done
    check "and nothing was printed of the chain" lacks "$tmp/r5.out" '^host '
    "$status" --host "$host" --recorded >"$tmp/r6.out" 2>&1
    check "--recorded with no value is refused" equals $? 1
    check "which says it needs one" matches "$tmp/r6.out" 'error: --recorded needs a value'
    "$status" help >"$tmp/r7.out" 2>&1
    check "the usage text names the command with its options" matches "$tmp/r7.out" \
        'status.sh \[--recorded N\] --host USER@HOST'
    check "and gives the default of --recorded" matches "$tmp/r7.out" '^  --recorded N .*default 6$'

    # The host: it is required, and it is USER@HOST, so that ssh cannot take it for an option.
    "$status" >"$tmp/h1.out" 2>&1
    check "status with no host is refused" equals $? 1
    check "with the usage" matches "$tmp/h1.out" 'usage: status.sh \[--recorded N\] --host USER@HOST'
    for value in -oProxyCommand=x fake.example @fake.example user@; do
        "$status" --host "$value" >"$tmp/h2.out" 2>&1
        check "--host \"$value\" is refused" equals $? 1
        check "as no USER@HOST" matches "$tmp/h2.out" "--host: \"$value\" is not USER@HOST"
    done

    # A chain that ended with the range recorded: no game in flight, and how it ended.  The last progress line was long
    # ago.
    fake_chain 40
    fake_results 40
    fake_recorded 40
    printf 'complete: the range 1-40\n' >"$work/chain.outcome"
    progress 40 9m00s 100 done 1 >"$work/logs/chain.err"
    stamp "$work/logs/chain.err" 1768447800
    TZ=America/Los_Angeles "$status" --host "$host" >"$tmp/e1.out" 2>&1
    sed 's/  */ /g' "$tmp/e1.out" >"$tmp/e1.norm"
    check "an ended chain says how it ended" has_line "$tmp/e1.norm" ' ENDED: complete: the range 1-40'
    check "and no game in flight" matches "$tmp/e1.norm" 'no game in flight: the range to 40 is recorded'
    check "above it the last 6 games recorded" equals "$(table_games "$tmp/e1.norm")" '35 36 37 38 39 40 '
    check "under a heading that holds no game in flight" has_line "$tmp/e1.norm" ' the last 6 games recorded:'
    check "and the note comes after them" equals "$(tail -n 1 "$tmp/e1.norm")" \
        ' no game in flight: the range to 40 is recorded'
    TZ=America/Los_Angeles "$status" --host "$host" --recorded 0 >"$tmp/e2.out" 2>&1
    check "with --recorded 0 an ended chain's status succeeds" equals $? 0
    check "and has the note" matches "$tmp/e2.out" '^  no game in flight: the range to 40 is recorded$'
    check "without a table" lacks "$tmp/e2.out" 'own search'
    check "or a heading" lacks "$tmp/e2.out" '^  the last .* recorded'
    check "or a recorded game" lacks "$tmp/e2.out" 'recorded, score'

    # A chain before its first progress line: logs/chain.err is not there, and the status is whole.
    fake_chain 100
    "$status" --host "$host" >"$tmp/b1.out" 2>&1
    check "a chain with no logs/chain.err is no error" equals $? 0
    check "it says no progress line is there" has_line "$tmp/b1.out" '  no progress line yet: no logs/chain.err'
    check "it says no game is recorded" has_line "$tmp/b1.out" '  recorded through: no game yet'
    check "and the games in flight have the note for it" equals "$(tail -n 1 "$tmp/b1.out")" '  no progress lines yet'
    check "in place of their rows" lacks "$tmp/b1.out" 'no recent progress line'

    # No game recorded yet (only an incomplete line), and a progress line: the table has no recorded game, and its
    # heading claims none.
    fake_chain 100
    fake_results 0
    printf 'n=1     score=10        search  nodes=5          2.000 s  incomplete  own=1.500 s  best=10\n' \
        >"$work/logs/chain.out"
    progress 1 1m00s 5000 120.0k/s 0 >"$work/logs/chain.err"
    "$status" --host "$host" >"$tmp/n1.out" 2>&1
    check "a chain with no game recorded succeeds" equals $? 0
    sed 's/  */ /g' "$tmp/n1.out" >"$tmp/n1.norm"
    check "it says no game is recorded" has_line "$tmp/n1.out" '  recorded through: no game yet'
    check "the table is the games in flight" equals "$(table_games "$tmp/n1.norm")" '1 2 3 4 5 '
    check "under their heading" has_line "$tmp/n1.out" '  games in flight (window 5):'
    check "with a game that runs" has_line "$tmp/n1.norm" ' 1 - 5,000 0 running, 120.0k nodes/s'
    check "and no game recorded" lacks "$tmp/n1.out" 'recorded, score\|^  the last'

    # Fewer games recorded than the table shows: those there are.
    fake_chain 100
    fake_results 3
    fake_recorded 3
    progress 4 1m00s 5000 120.0k/s 0 >"$work/logs/chain.err"
    "$status" --host "$host" >"$tmp/f1.out" 2>&1
    sed 's/  */ /g' "$tmp/f1.out" >"$tmp/f1.norm"
    check "with 3 games recorded the table has 3" equals "$(table_games "$tmp/f1.norm")" '1 2 3 4 5 6 7 8 '
    check "and its heading says 3" has_line "$tmp/f1.norm" ' the last 3 games recorded and the 5 in flight (window 5):'
    "$status" --host "$host" --recorded 1 >"$tmp/f2.out" 2>&1
    sed 's/  */ /g' "$tmp/f2.out" >"$tmp/f2.norm"
    check "one game is not games" has_line "$tmp/f2.norm" ' the last game recorded and the 5 in flight (window 5):'
    check "and it is the last recorded" equals "$(table_games "$tmp/f2.norm")" '3 4 5 6 7 8 '

    # Games recorded and no progress lines: the recorded games, then the note for the games in flight.
    rm -f "$work/logs/chain.err"
    "$status" --host "$host" >"$tmp/q1.out" 2>&1
    check "recorded games and no chain.err succeeds" equals $? 0
    sed 's/  */ /g' "$tmp/q1.out" >"$tmp/q1.norm"
    check "the table has the recorded games only" equals "$(table_games "$tmp/q1.norm")" '1 2 3 '
    check "under the heading of both" has_line "$tmp/q1.norm" \
        ' the last 3 games recorded and the 5 in flight (window 5):'
    check "and the note for the games in flight follows" equals "$(tail -n 1 "$tmp/q1.out")" \
        '  no progress lines yet'
    check "in place of their rows" lacks "$tmp/q1.out" 'no recent progress line'

    # A node count of 14,000,000,000 has its commas and ends under the header's right-aligned nodes; the window is the
    # chain.log's.
    fake_chain 100
    printf '%s chain 1-100; threads: all available; window 3; results x; checkpoints y\n' 2026-10-21T00:00:00Z \
        >"$work/logs/chain.log"
    fake_results 3
    recorded_line 2 20 search 5 1.5 unknown exhausted "  own=1.500 s  best=18" >"$work/logs/chain.out"
    progress 4 1h40m00s 14000000000 250.0k/s 341541 >"$work/logs/chain.err"
    printf '7200.5\n' >"$work/ckpt/n0004.clock"
    TZ=America/Los_Angeles "$status" --host "$host" >"$tmp/w1.out" 2>&1
    sed 's/  */ /g' "$tmp/w1.out" >"$tmp/w1.norm"
    check "the table has the game of chain.out, the game in flight with its clock, and the window of chain.log" equals \
        "$(grep -c -e ' 2 1.5s 5 18 recorded, score 20 (search, exhausted)$' \
            -e ' 4 2h00m00s 14,000,000,000 341,541 running, 250.0k nodes/s$' \
            -e ' 6 - - - no recent progress line$' "$tmp/w1.norm")" 3
    check "a node count of 14,000,000,000 has its commas and ends under the header's right-aligned nodes" equals \
        "$(awk '/ own search / {head = index($0, "nodes") + 4}
            /14,000,000,000/ {row = index($0, "14,000,000,000") + 13}
            END {print (head > 0 && head == row) ? "ok" : "head " head " row " row}' "$tmp/w1.out")" ok
}

for s in "${selected[@]}"; do
    case "$s" in
        chain) scenario_chain ;;
        ledger) scenario_ledger ;;
        lock) scenario_lock ;;
        status) scenario_status ;;
        *) echo "unknown scenario $s" >&2; exit 2 ;;
    esac
done
echo
if [ "$failures" = 0 ]; then echo "all checks passed"; else echo "$failures checks FAILED"; fi
exit "$failures"
