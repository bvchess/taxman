#!/usr/bin/env bash
# Tests of on-event.sh, the program that starts a study, with a stand-in for claude (and for bwrap, socat, curl and,
# where the machine has none, timeout and flock): no sandbox, no agent session.  They check what on-event.sh
# promises: it refuses without a proof, with settings that lack what the sandbox needs, from a place the sandbox may
# write, and when bwrap or claude fail; it runs one study at a time, and at most LAB_MAX_PER_DAY studies for long events
# and asks together; it hands claude the flags, the working directory, the niceness and the clean environment it should
# have; a proof fails when a probe's write is not refused; and an ask goes on with its study's session, and the requests
# are kept.  Needs jq; the checks of the lock need flock (Linux).
#
#   solver/tools/lab/test-studies.sh
#
# Prints `ok` or `FAIL` for each check and exits with the number of failures.  A few seconds.

set -u

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
command -v jq >/dev/null 2>&1 || {
    echo "jq is needed" >&2
    exit 2
}

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"; kill $(jobs -p) 2>/dev/null' EXIT
home="$tmp/home"
bin="$tmp/bin"
checkout="$tmp/checkout/solver/tools"
lab="$home/taxman-lab"
mkdir -p "$home/.local/bin" "$bin" "$home/taxman-work" "$home/taxman" "$home/taxman-archive" "$checkout"
cp -R "$here" "$checkout/lab"
cp -R "$here/../cloud" "$checkout/cloud"
program="$checkout/lab/on-event.sh"

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
logged() { grep -Fq -- "$1" "$lab/ctl/on-event.log" 2>/dev/null; }
claude_runs() { grep -c "^argv: " "$home/fake-claude.log" 2>/dev/null || echo 0; }

# Stand-ins.  claude records how it was run, and can be made to sleep, to fail, or to carry out the script of a probe
# session (without any sandbox, which a proof must notice); bwrap can be made to fail.
cat >"$home/.local/bin/claude" <<'EOF'
#!/bin/sh
d="$HOME"
if [ "$1" = --version ]; then
    echo "$(cat "$d/fake-claude.version" 2>/dev/null || echo 9.9.9) (stand-in)"
    exit 0
fi
{
    echo "argv: $*"
    echo "cwd: $(pwd)"
    echo "nice: $(ps -o ni= -p $$ | tr -d ' ')"
    echo "env: $(env | sort | tr '\n' ' ')"
} >>"$d/fake-claude.log"
prompt="$(cat)"
if [ -f "$d/fake-claude.fail" ]; then
    cat "$d/fake-claude.fail" >&2
    exit 1
fi
if [ -f "$d/fake-claude.sleep" ]; then sleep "$(cat "$d/fake-claude.sleep")"; fi
case "$prompt" in
    *"self-test of your sandbox"*)
        script="$(printf '%s\n' "$prompt" | sed -n 's/^bash //p' | head -n 1)"
        bash "$script" >/dev/null 2>&1
        ;;
esac
echo '{"type":"system","subtype":"init"}'
printf '%s' '{"type":"result","subtype":"success","is_error":false,"num_turns":3,"duration_ms":4000,'
echo '"total_cost_usd":0.12,"session_id":"sess-test","result":"the answer"}'
if [ -f "$d/fake-claude.linger" ]; then exec sleep 60; fi
EOF
cat >"$bin/bwrap" <<'EOF'
#!/bin/sh
if [ "$1" = --version ]; then echo "bubblewrap 0.6.1"; exit 0; fi
[ ! -f "$HOME/fake-bwrap.fail" ]
EOF
printf '#!/bin/sh\nexit 0\n' >"$bin/socat"
printf '#!/bin/sh\nexit 7\n' >"$home/.local/bin/curl"
real_flock=yes
if ! command -v flock >/dev/null 2>&1; then
    real_flock=no
    printf '#!/bin/sh\nexit 0\n' >"$bin/flock"
fi
if ! command -v timeout >/dev/null 2>&1; then
    printf '#!/bin/sh\nshift\nexec "$@"\n' >"$bin/timeout"
fi
chmod +x "$home/.local/bin/claude" "$home/.local/bin/curl" "$bin"/*

export HOME="$home" PATH="$bin:$PATH" CLAUDE_BIN="$home/.local/bin/claude"
unset LAB_HOME LAB_THREADS LAB_MAX_PER_DAY LAB_MAX_TURNS LAB_MODEL LAB_MAX_HOURS LAB_PROOF_MODEL
export SECRET_TOKEN=secret-one SERVICE_API_KEY=secret-two LAB_DIR=/not/for/us

event() {
    printf '{"event":"%s","n":%s,"nodes":1000,"best":5,"own_seconds":3700,' "$1" "$2"
    printf '"checkpoint_dir":null,"lab_dir":null}'
}

sha256() { sha256sum "$@" 2>/dev/null || shasum -a 256 "$@"; }

# A valid proof for the settings, claude and bwrap as they are now.
make_proof() {
    "$program" --status >/dev/null 2>&1
    {
        echo "result=pass"
        printf 'settings=%s\n' "$(sha256 "$lab/ctl/settings.json" | awk '{print $1}')"
        printf 'claude=%s (stand-in)\n' "$(cat "$home/fake-claude.version" 2>/dev/null || echo 9.9.9)"
        echo "bwrap=bubblewrap 0.6.1"
        echo "at=test"
    } >"$lab/ctl/proof"
}

echo "--- the proof"
out="$("$program" --status 2>&1)"
check "status says there is no proof" contains "$out" "NOT VALID: there is no proof"
out="$("$program" "$(event long 4879)" 2>&1)"
check "no study without a proof" contains "$out" "refused: there is no proof"
check "and claude was not run" test "$(claude_runs)" = 0
out="$("$program" --proof 2>&1)"
check "a sandbox that lets the probes write fails the proof" contains "$out" "FAIL  outside"
check "and the proof file says fail" grep -qx 'result=fail' "$lab/ctl/proof"
out="$("$program" "$(event long 4879)" 2>&1)"
check "and so does a study still refuse" contains "$out" "the proof of the sandbox failed"
check "the probe in the home directory was removed" test ! -e "$home/lab-probe-outside"
check "stage B (the chain's directories) was not run after a failure" test ! -e "$home/taxman-work/lab-probe"

echo "--- a study"
make_proof
rm -f "$home/fake-claude.log"
LAB_MAX_TURNS=7 LAB_MODEL=opus "$program" "$(event long 4879)" >/dev/null 2>&1
check "claude ran once" test "$(claude_runs)" = 1
args="$(grep '^argv: ' "$home/fake-claude.log")"
study_md="$(cd "$checkout/lab" && pwd -P)/study.md"
for flag in "-p" "--max-turns 7" "--model opus" "--permission-mode dontAsk" "--permission-prompts none" \
    "--setting-sources" "--settings $lab/ctl/settings.json" "--append-system-prompt-file $study_md" \
    "--strict-mcp-config"; do
    check "claude is given $flag" contains "$args" "$flag"
done
check "a study keeps its session, for ask to go on with" lacks "$args" "--no-session-persistence"
check "a proof session does not" contains "$(cat "$lab"/studies/proof-*/a/argv.txt)" "--no-session-persistence"
check "and never bypasses permission checks" lacks "$args" "dangerously"
check "and never bypassPermissions" lacks "$args" "bypassPermissions"
cwd="$(grep '^cwd: ' "$home/fake-claude.log")"
check "it runs in the working directory of the sandbox" contains "$cwd" "/taxman-lab/work"
check "at nice 19" contains "$(grep '^nice: ' "$home/fake-claude.log")" "nice: 19"
envs="$(grep '^env: ' "$home/fake-claude.log")"
check "with none of the caller's secrets" lacks "$envs" "secret-"
check "and not the chain's LAB_DIR" lacks "$envs" "/not/for/us"
check "with its own temp, cargo and thread settings" contains "$envs" "CLAUDE_CODE_TMPDIR=$lab/work/tmp"
check "and the memory files off" contains "$envs" "CLAUDE_CODE_DISABLE_CLAUDE_MDS=1"
study="$(ls -d "$lab"/studies/4879-* | head -n 1)"
check "the study keeps its event" test "$(jq -r .n "$study/event.json")" = 4879
check "its prompt holds the event" grep -q '"own_seconds":3700' "$study/prompt.txt"
check "its session" test -s "$study/session.jsonl"
check "its result" test "$(jq -r .num_turns "$study/result.json")" = 3
check "the log has the start" logged "start event=long n=4879"
check "and the end with the turns" logged "turns=3"
check "the settings were rendered for this lab" grep -q '"~/taxman-lab/channel"' "$lab/ctl/settings.json"
check "work/.claude does not outlive the session" test ! -e "$lab/work/.claude"
check "the journal exists for the session to append to" test -f "$lab/work/journal.md"

echo "--- limits"
LAB_MAX_PER_DAY=2 "$program" "$(event long 4879)" >/dev/null 2>&1
out="$(LAB_MAX_PER_DAY=2 "$program" "$(event long 4879)" 2>&1)"
check "the third long event of a day is skipped at LAB_MAX_PER_DAY=2" contains "$out" "the most is LAB_MAX_PER_DAY=2"
before="$(claude_runs)"
LAB_MAX_PER_DAY=2 "$program" "$(event done 4879)" >/dev/null 2>&1
check "a done event is not counted by the limit" test "$(claude_runs)" = $((before + 1))
before="$(claude_runs)"
out="$("$program" "$(event done 4999)" 2>&1)"
check "a done event of a game that had no long event is skipped" contains "$out" "no study of a long event of game 4999"
check "and the log says why" logged "event=done n=4999: no study of a long event"
LAB_MAX_PER_DAY=2 "$program" "$(event long 4998)" >/dev/null 2>&1
out="$("$program" "$(event done 4998)" 2>&1)"
check "so is the done event of a game whose long event was skipped by the limit" \
    contains "$out" "no study of a long event of game 4998"
check "and neither started a study" test "$(claude_runs)" = "$before"
"$program" --off >/dev/null 2>&1
out="$("$program" "$(event long 4880)" 2>&1)"
check "no study while studies are off" contains "$out" "studies are off"
"$program" --on >/dev/null 2>&1
check "and studies start again when they are on" contains "$("$program" --status 2>&1)" "studies: on"
out="$("$program" 'not json' 2>&1)"
check "an argument that is not an event is refused" contains "$out" 'refused: the argument is not JSON'
out="$("$program" '{"event":"boom","n":1}' 2>&1)"
check "so is an event on-event.sh does not know" contains "$out" 'an event "boom"'
if [ "$real_flock" = yes ]; then
    echo 1 >"$home/fake-claude.sleep"
    rm -f "$lab/ctl/starts.log"
    "$program" "$(event long 4881)" >/dev/null 2>&1 &
    for _ in $(seq 1 50); do
        logged "start event=long n=4881" && break
        sleep 0.1
    done
    out="$("$program" "$(event long 4882)" 2>&1)"
    check "a long event is skipped while a study runs" contains "$out" "a study is running"
    before="$(claude_runs)"
    "$program" "$(event done 4881)" >/dev/null 2>&1
    check "and a done event waits for it, then runs" test "$(claude_runs)" = $((before + 1))
    rm -f "$home/fake-claude.sleep"
    wait
else
    echo "skip  the checks of the lock: no flock on this machine"
fi

echo "--- refusals"
out="$(
    touch "$home/fake-bwrap.fail"
    "$program" "$(event long 4879)" 2>&1
)"
check "bwrap that cannot start refuses" contains "$out" "bwrap cannot start a sandbox"
rm -f "$home/fake-bwrap.fail"
echo 8.8.8 >"$home/fake-claude.version"
out="$("$program" "$(event long 4879)" 2>&1)"
check "a study refuses when claude is not the version proved" contains "$out" "other versions"
make_proof
touch "$home/fake-claude.fail"
echo "sandbox dependencies are missing" >"$home/fake-claude.fail"
rm -rf "$lab/ctl/starts.log"
"$program" "$(event long 4879)" >/dev/null 2>&1
check "claude that cannot start is logged with its message" logged "no result: sandbox dependencies are missing"
rm -f "$home/fake-claude.fail"
# Settings that lack what the sandbox needs, in a copy of the checkout.
copy="$tmp/copy/solver/tools"
mkdir -p "$copy"
cp -R "$checkout/lab" "$checkout/cloud" "$copy/"
chmod -R u+w "$copy"
sed -i.bak 's/"failIfUnavailable": true/"failIfUnavailable": false/' "$copy/lab/settings.template.json"
out="$("$copy/lab/on-event.sh" "$(event long 4879)" 2>&1)"
check "settings without failIfUnavailable are refused" contains "$out" "the settings lack what the sandbox needs"
# The program inside a place the sandbox may write (the lab's own checkout).
mkdir -p "$lab/src/solver"
cp -R "$checkout" "$lab/src/solver/"
out="$("$lab/src/solver/tools/lab/on-event.sh" "$(event long 4879)" 2>&1)"
check "a copy in the lab's src is refused" contains "$out" "where the sandbox may write"
rm -rf "$lab/src"

echo "--- a session that does not end by itself"
# claude stays alive while a process of the session runs in the background: the study ends shortly after its result.
touch "$home/fake-claude.linger"
rm -f "$lab/ctl/starts.log"
began="$(date +%s)"
SESSION_GRACE=1 "$program" "$(event long 4895)" >/dev/null 2>&1
check "a session is ended soon after its result (took $(($(date +%s) - began)) s of 60)" \
    test $(($(date +%s) - began)) -lt 30
check "and counts as a success" logged "exit event=long n=4895 status=0"
rm -f "$home/fake-claude.linger"

echo "--- ask, and requests"
rm -f "$home/fake-claude.fail" "$home/fake-claude.sleep" "$lab/ctl/starts.log" "$home/fake-claude.log"
make_proof
name="$(basename "$study")"
check "a study's prompt names the study" grep -q "Your study is called $name" "$study/prompt.txt"
check "the requests file is there for the studies" grep -q '^# Requests' "$lab/work/requests.md"
"$program" --request "try the other branching" >/dev/null 2>&1
"$program" --request $'two\nlines' >/dev/null 2>&1
dated='^- \[ \] [0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}Z '
check "a request is a dated, unmarked line" grep -Eq "${dated}try the other branching\$" "$lab/work/requests.md"
check "and one line, whatever the text" grep -Eq "${dated}two lines\$" "$lab/work/requests.md"
check "--request alone prints the file" contains "$("$program" --request 2>&1)" "try the other branching"
out="$("$program" --ask "$name" "what would you try next?" 2>&1)"
check "ask prints the answer" test "$out" = "the answer"
args="$(grep '^argv: ' "$home/fake-claude.log")"
check "and goes on with the session of the study" contains "$args" "--resume sess-test"
check "under the flags of a study" contains "$args" "--permission-mode dontAsk"
check "its question and answer are kept in the study's folder" \
    grep -q "Question: what would you try next?" "$study/asks.md"
check "and the session of the ask" test -s "$(ls -d "$study"/ask-* | head -n 1)/session.jsonl"
check "the log has the ask" logged "start event=ask"
out="$(LAB_MAX_PER_DAY=1 "$program" --ask "$name" "once more?" 2>&1)"
check "an ask counts toward the day's limit" contains "$out" "the most is LAB_MAX_PER_DAY=1"
check "an unknown study is refused" contains "$("$program" --ask no-such-study "x" 2>&1)" "no study no-such-study"
proof_dir="$(basename "$(ls -d "$lab"/studies/proof-* | head -n 1)")"
check "so is a proof, which is no study" contains "$("$program" --ask "$proof_dir" "x" 2>&1)" "no study $proof_dir"
check "and a question that is empty" contains "$("$program" --ask "$name" "" 2>&1)" "usage"
"$program" --off >/dev/null 2>&1
check "no ask while studies are off" contains "$("$program" --ask "$name" "x" 2>&1)" "studies are off"
"$program" --on >/dev/null 2>&1
if [ "$real_flock" = yes ]; then
    echo 3 >"$home/fake-claude.sleep"
    "$program" "$(event long 4890)" >/dev/null 2>&1 &
    for _ in $(seq 1 50); do
        logged "start event=long n=4890" && break
        sleep 0.1
    done
    out="$(LAB_MAX_PER_DAY=9 "$program" --ask "$name" "x" 2>&1)"
    check "no ask while a study runs" contains "$out" "a study is running"
    wait
    rm -f "$home/fake-claude.sleep"
fi

echo
if [ "$failures" -eq 0 ]; then
    echo "all ok"
else
    echo "$failures failed"
fi
exit "$failures"
