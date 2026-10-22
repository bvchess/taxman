#!/usr/bin/env bash
# Tests of lab.sh that need no remote machine and no build of the solver: a fake home with a chain's checkpoints in
# taxman-work/ckpt, a stand-in for tune that writes the lines tune writes, and a stand-in for ssh that runs the command
# here.  They check what the lab promises: it copies and never changes the chain's files, it picks the game with the
# longest search, it ranks units by gap alone, it runs at most LAB_THREADS units at once at nice 19 and passes the
# extra arguments on, and it runs in place with --local or when LAB_HOST names the machine.  Needs jq.
#
#   solver/tools/lab/test.sh
#
# Prints `ok` or `FAIL` for each check and exits with the number of failures.  A few seconds.

set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
lab="$here/lab.sh"
command -v jq >/dev/null 2>&1 || {
    echo "jq is needed" >&2
    exit 2
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
home="$tmp/home"
bin="$tmp/bin"
fakes="$home/taxman-lab/src/solver/frame/target/release/examples"
ckpt="$home/taxman-work/ckpt"
mkdir -p "$bin" "$fakes" "$ckpt"

failures=0
check() {
    local what="$1"
    shift
    if "$@" >/dev/null 2>&1; then
        echo "ok    $what"
    else
        echo "FAIL  $what"
        failures=$((failures + 1))
    fi
}
contains() { grep -Fq -- "$2" <<<"$1"; }
lacks() { ! grep -Fq -- "$2" <<<"$1"; }

# A stand-in for ssh: ssh [-o OPTION]... HOST COMMAND runs COMMAND here, and says so.
cat >"$bin/ssh" <<'EOF'
#!/bin/sh
echo "ssh $*" >> "$SSH_LOG"
while [ "$1" = -o ]; do shift 2; done
shift
exec sh -c "$*"
EOF
# A stand-in for tune: units writes a units file of four units (one without a gap), solve writes a result line, and
# each solve notes when it starts and ends and at which niceness, for the checks of the concurrency.
cat >"$fakes/tune" <<'EOF'
#!/bin/sh
cmd="$1"
game="$2"
shift 2
case "$cmd" in
units)
    ckpt="$1"
    shift
    while [ $# -gt 0 ]; do
        if [ "$1" = --out ]; then out="$2"; fi
        shift
    done
    [ -f "$ckpt" ] || exit 2
    {
        echo '{"kind":"game","game":'"$game"',"candidates":"ff"}'
        for u in "0 10" "1 900" "2 null" "3 300" "9 50"; do
            set -- $u
            printf '{"kind":"unit","game":%s,"id":"%s-%s","index":%s,"depth":%s,' "$game" "$game" "$1" "$1" "$1"
            printf '"n_cands":7,"sum":3,'
            printf '"bound":2000,"need":1100,"gap":%s,"promoted":"00","cands":"ff"}\n' "$2"
        done
    } >"$out"
    echo "5 units in $ckpt: 1000 nodes searched, incumbent 538080" >&2
    ;;
solve)
    echo "args $*" >>"$CONC_LOG"
    while [ $# -gt 0 ]; do
        case "$1" in
            --ids) id="$2" ;;
            --tactic) tactic="$2" ;;
            --node-limit) limit="$2" ;;
        esac
        shift
    done
    echo "start $$ $(ps -o ni= -p $$ | tr -d ' ') $PPID" >>"$CONC_LOG"
    sleep "${FAKE_SLEEP:-0}"
    echo "end $$" >>"$CONC_LOG"
    case "$id" in
        *-9)
            echo "error: no such luck" >&2
            exit 2
            ;;
    esac
    printf '{"game":%s,"id":"%s","depth":5,"cands":7,"nodes":%s,"cpu_s":1.25,"wall_s":2.5,"completed":%s,' \
        "$game" "$id" "${limit:-123456789}" "$([ -n "${limit:-}" ] && echo false || echo true)"
    printf '"best":538080,"tactics":"%s","node_limit":%s}\n' "${tactic:-default}" "${limit:-null}"
    ;;
esac
EOF
chmod +x "$bin/ssh" "$fakes/tune"
mkdir -p "$home/taxman-lab/src"
git init -q "$home/taxman-lab/src" && git -C "$home/taxman-lab/src" -c user.name=t -c user.email=t@t \
    commit -q --allow-empty -m "a commit"

# The chain's checkpoints: games 4874 (clock 100.5 s), 4875 (900.25 s) and 4876 (no clock).
for n in 4874 4875 4876; do
    head -c 3000 /dev/urandom >"$ckpt/n$n.ckpt"
done
echo 100.5 >"$ckpt/n4874.clock"
echo 900.25 >"$ckpt/n4875.clock"
chain_state() { (cd "$home/taxman-work" && find . -type f | sort | while read -r f; do cksum "$f"; ls -l "$f"; done); }
before="$(chain_state)"

export HOME="$home" SSH_LOG="$tmp/ssh.log" CONC_LOG="$tmp/conc.log" SSH_CLI="$bin/ssh"
: >"$SSH_LOG"
: >"$CONC_LOG"
unset LAB_HOST LAB_THREADS LAB_DIR LAB_CKPT_DIR

echo "--- snapshot"
out="$("$lab" --local snapshot 2>&1)"
check "the default snapshot is the game with the longest own search" contains "$out" "snapshot 4875-"
snap="$(ls "$home/taxman-lab/snap" | grep '^4875-' | head -n 1)"
check "it copies the checkpoint" cmp -s "$ckpt/n4875.ckpt" "$home/taxman-lab/snap/$snap/n4875.ckpt"
check "it copies the clock" cmp -s "$ckpt/n4875.clock" "$home/taxman-lab/snap/$snap/n4875.clock"
check "it says the own search" contains "$out" "own search: 900.25 s"
out="$("$lab" --local snapshot 4876 2>&1)"
check "a game by number, with no clock" contains "$out" "snapshot 4876-"
out="$("$lab" --local snapshot 12 2>&1)"
check "a game not in flight is refused" contains "$out" "game 12 is not in flight"
check "no ssh was used with --local" test ! -s "$SSH_LOG"
out="$(LAB_CKPT_DIR=nowhere "$lab" --local snapshot 2>&1)"
check "a directory without checkpoints is refused" contains "$out" "no checkpoint in"

echo "--- units"
out="$("$lab" --local units "$snap" 2>&1)"
check "the summary of tune comes first" contains "$out" "5 units in"
check "the units are counted" contains "$out" "5 units, 4 with a gap"
rows="$(grep -E '^4875-' <<<"$out" | awk '{print $1}' | tr '\n' ' ')"
check "ranked by gap alone, the unit without a gap left out" test "$rows" = "4875-1 4875-3 4875-9 4875-0 "
check "the gap of the first is shown" contains "$out" "$(printf '%-14s %6s %6s %9s %9s %9s' 4875-1 1 7 2,000 1,100 900)"
check "units.jsonl is in the snapshot" test -s "$home/taxman-lab/snap/$snap/units.jsonl"
out="$("$lab" --local units ../../taxman-work/ckpt 2>&1)"
check "a directory outside the lab is refused" contains "$out" "outside"

echo "--- run"
: >"$CONC_LOG"
out="$(FAKE_SLEEP=1 LAB_THREADS=2 "$lab" --local run "$snap" 4875-1,4875-3,4875-0,4875-root --node-limit 5000 \
    --tactic conflict_branching=off 2>&1)"
check "a row for each unit" test "$(grep -c '^4875-' <<<"$out")" -eq 4
check "a unit stopped by the node limit says limit" contains "$out" "limit"
check "steps have their commas" contains "$out" "5,000"
runs="$home/taxman-lab/snap/$snap/runs.jsonl"
check "a line each in runs.jsonl" test "$(wc -l <"$runs" | tr -d ' ')" -eq 4
check "the extra arguments reach tune" test "$(jq -r '.tactics' "$runs" | sort -u)" = conflict_branching=off
check "each line has the commit of the build" test "$(jq -r '.commit' "$runs" | sort -u | wc -l | tr -d ' ')" -eq 1
clone_commit="$(git -C "$home/taxman-lab/src" rev-parse --short HEAD)"
check "the commit is the clone's" test "$(jq -r '.commit' "$runs" | head -n 1)" = "$clone_commit"
check "every solve ran at nice 19" test "$(grep '^start' "$CONC_LOG" | awk '{print $3}' | sort -u)" = 19
most="$(awk '/^start/ {n++; if (n > m) m = n} /^end/ {n--} END {print m}' "$CONC_LOG")"
check "at most LAB_THREADS units at once (saw $most)" test "$most" -le 2
check "and more than one at a time (saw $most)" test "$most" -ge 2
out="$("$lab" --local run "$snap" 4875-9,4875-3 2>&1)"
status=$?
check "a unit that fails is reported, and the others still run" contains "$out" "4875-9         FAILED: error: no such"
check "its siblings' rows are there" contains "$out" "4875-3"
check "the failure is the exit status" test "$status" -ne 0
check "temporary files are removed" test -z "$(ls -A "$home/taxman-lab/snap/$snap" | grep '^\.')"
out="$("$lab" --local run "$snap" 4875-77 2>&1)"
check "an unknown unit is refused before anything runs" contains "$out" "no unit 4875-77"
out="$(LAB_THREADS=0 "$lab" --local run "$snap" 4875-1 2>&1)"
check "LAB_THREADS must be a number of 1 or more" contains "$out" "LAB_THREADS"
out="$("$lab" --local run "$snap" 4875-1 --known '~/results.jsonl' 2>&1)"
check "a leading ~/ in an argument is the host's home" contains "$(cat "$CONC_LOG")" "--known $home/results.jsonl"

echo "--- stopping"
: >"$CONC_LOG"
FAKE_SLEEP=5 "$lab" --local run "$snap" 4875-1,4875-3 >"$tmp/stop.out" 2>&1 &
lab_pid=$!
for _ in $(seq 1 50); do
    [ "$(grep -c '^start' "$CONC_LOG")" -ge 2 ] && break
    sleep 0.2
done
tune_pid="$(grep '^start' "$CONC_LOG" | head -n 1 | awk '{print $2}')"
unit_pid="$(grep '^start' "$CONC_LOG" | head -n 1 | awk '{print $4}')"
# The host's shell is the parent of the unit's shell; a Ctrl-C reaches it as SIGINT.
kill -INT "$(ps -o ppid= -p "$unit_pid" | tr -d ' ')"
wait "$lab_pid"
sleep 0.5
check "a run that is interrupted stops its units" test -z "$(ps -o pid= -p "$tune_pid")"
check "and removes what they left in the snapshot" test -z "$(ls -A "$home/taxman-lab/snap/$snap" | grep '^\.')"

echo "--- ssh"
host="$(hostname -s 2>/dev/null || hostname)"
sleep 1 # a snapshot of a game is named by the second
out="$(LAB_HOST="u@elsewhere.example.net" "$lab" snapshot 4874 2>&1)"
check "from another machine the lab goes over ssh" contains "$out" "snapshot 4874-"
check "with BatchMode and a time-out" contains "$(cat "$SSH_LOG")" "BatchMode=yes"
check "to the host" contains "$(cat "$SSH_LOG")" "u@elsewhere.example.net"
: >"$SSH_LOG"
sleep 1
out="$(LAB_HOST="u@$host.example.net" "$lab" snapshot 4874 2>&1)"
check "on the host named by LAB_HOST it runs in place" contains "$out" "snapshot 4874-"
check "with no ssh" test ! -s "$SSH_LOG"
out="$("$lab" snapshot 2>&1)"
check "without LAB_HOST or --local it says what to set" contains "$out" "LAB_HOST is not set"
out="$(LAB_HOST='-oProxyCommand=x' "$lab" snapshot 2>&1)"
check "a LAB_HOST that is not USER@HOST is refused" contains "$out" "is not USER@HOST"
out="$("$lab" --local snapshot 1x 2>&1)"
check "N must be a number" contains "$out" "is not a game"

echo "--- the chain"
check "the chain's directory is as it was" test "$(chain_state)" = "$before"

echo
if [ "$failures" -eq 0 ]; then
    echo "all ok"
else
    echo "$failures failed"
fi
exit "$failures"
