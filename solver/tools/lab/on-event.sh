#!/usr/bin/env bash
# The program that `taxman solve --hook` runs when a game's search passes `--hook-after` and when it ends: it starts
# a study, an agent session in a sandbox that steers the search and studies it (solver/docs/runbook/lab.md, "Studies").
# It is code of the chain's checkout, which the sandbox cannot write; so are the settings it hands to the session
# (rendered fresh from settings.template.json at every start) and the procedure the session follows (study.md).  It
# never runs from the lab's own checkout, whose files a study may change.
#
#   on-event.sh EVENT_JSON   a study for the event {"event": "long"|"done", "n": N, ...} (the hook's one argument)
#   on-event.sh --proof      prove the sandbox on this machine, which every study needs first (a few short sessions);
#                            it takes the lock without waiting, and refuses while a study runs
#   on-event.sh --ask STUDY QUESTION   put QUESTION to a finished study (its folder's name in studies/): its session
#                            goes on with its whole context (claude --resume), and the answer is printed and added to
#                            the study's folder (asks.md)
#   on-event.sh --request [TEXT]       add a dated request for the studies to work/requests.md (without TEXT: print it)
#   on-event.sh --status     whether studies are on, the proof, the day's count, and the end of the log
#   on-event.sh --off        no study starts until --on (the file ctl/off)
#   on-event.sh --on
#
# A study, or an ask, starts only if all of these hold, and the log (ctl/on-event.log) says which did not:
#   - studies are on, and the tools are there (claude, jq, flock, timeout, nice, bwrap, socat)
#   - this program, study.md and the settings are outside every place the sandbox may write
#   - the settings still say what the sandbox needs (enabled, failIfUnavailable, no unsandboxed retry, ...)
#   - a proof of the sandbox exists that passed (--proof, run by a person for the first time and after a failure); if it
#     passed for other settings or other versions of claude or bwrap, as claude updates itself, the study or ask first
#     runs the proof again and goes on only if it passes (the log says "proof renewed")
#   - no other study runs (a long event, or an ask, is skipped or refused), and fewer than LAB_MAX_PER_DAY long events
#     and asks together have started today (UTC)
#   - for a done event: studies/ holds a study of that game's long event, which it compares the outcome with (so the
#     done studies are no more than the long ones, and the limit bounds both); it then waits for the study that runs
# The proof is renewed only once the lock is held and the day's limit has allowed the study, so it never runs beside a
# study, and never for an event that would be skipped.
# The session runs in LAB/work, at nice 19, with a clean environment, the turn limit and the time limit below, and
# without bypassing permission checks; its commands run in the sandbox or not at all.  Whoever runs the lab leaves
# standing questions in LAB/work/requests.md (lab.sh request), which a study reads: guidance, nothing that can change
# the settings or the sandbox, which come from this checkout.
#
# Environment:
#   LAB_HOME           the lab's directory, relative to the home                                  default taxman-lab
#   LAB_THREADS        threads the session may use (the study procedure says so)                  default 16
#   LAB_MAX_PER_DAY    studies for a long event, and asks, a day                                  default 4
#   LAB_MAX_TURNS      turns of the session (claude --max-turns)                                  default 80
#   LAB_ASK_TURNS      turns of an ask                                                            default 30
#   LAB_MODEL          model of the session                                                       default opus
#   LAB_MAX_HOURS      hours before the session is stopped                                        default 3
#   LAB_PROOF_MODEL    model of the sessions of --proof                                           default sonnet
#   CLAUDE_BIN         the claude program                                              default ~/.local/bin/claude

set -u
set -o pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)"
TOOLS="$(dirname "$HERE")"
STUDY="$HERE/study.md"
TEMPLATE="$HERE/settings.template.json"
LAB_HOME="${LAB_HOME:-taxman-lab}"
LAB="$HOME/$LAB_HOME"
CTL="$LAB/ctl"
WORK="$LAB/work"
STUDIES="$LAB/studies"
LAB_THREADS="${LAB_THREADS:-16}"
LAB_MAX_PER_DAY="${LAB_MAX_PER_DAY:-4}"
LAB_MAX_TURNS="${LAB_MAX_TURNS:-80}"
LAB_ASK_TURNS="${LAB_ASK_TURNS:-30}"
LAB_MODEL="${LAB_MODEL:-opus}"
LAB_MAX_HOURS="${LAB_MAX_HOURS:-3}"
LAB_PROOF_MODEL="${LAB_PROOF_MODEL:-sonnet}"
CLAUDE="${CLAUDE_BIN:-$HOME/.local/bin/claude}"

# --- the log, and the ways out -------------------------------------------------------------------------------------

log() {
    mkdir -p "$CTL" 2>/dev/null
    printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >>"$CTL/on-event.log"
}

# A study that does not start: skipped (nothing is wrong) or refused (something is).
skip() {
    log "skipped: $*"
    echo "on-event.sh: skipped: $*" >&2
    exit 0
}

refuse() {
    log "refused: $*"
    echo "on-event.sh: refused: $*" >&2
    exit 3
}

sha256() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$@"; else shasum -a 256 "$@"; fi
}

# The absolute path of $1 with its links resolved, whether or not it exists.
real() {
    readlink -f "$1" 2>/dev/null || printf '%s/%s\n' "$(cd "$(dirname "$1")" && pwd -P)" "$(basename "$1")"
}

# $1 with a leading ~/ the home.
expand_home() {
    case "$1" in
        "~/"*) printf '%s\n' "$HOME/${1#"~/"}" ;;
        *) printf '%s\n' "$1" ;;
    esac
}

# --- the settings, and what has to hold before any session ----------------------------------------------------------

# The settings of the sandbox, from the template with the lab's directory in place of @LAB@.
render_settings() {
    sed "s|@LAB@|~/$LAB_HOME|g" "$TEMPLATE"
}

# What the sandbox needs of the settings, whatever the template says.
INVARIANTS='.sandbox.enabled == true and .sandbox.failIfUnavailable == true
    and .sandbox.allowUnsandboxedCommands == false and .sandbox.autoAllowBashIfSandboxed == true
    and ((.sandbox.excludedCommands // []) | length) == 0 and .permissions.defaultMode == "dontAsk"
    and .disableAllHooks == true'

# The places the sandbox may write, one to a line: the working directory and the settings' allowWrite.
writable_roots() {
    local path
    real "$WORK"
    jq -r '.sandbox.filesystem.allowWrite[]?' "$CTL/settings.json" | while IFS= read -r path; do
        real "$(expand_home "$path")"
    done
}

# Refuses unless every file that governs a study, and the lab's controls, are outside the places the sandbox writes.
check_nothing_governing_is_writable() {
    local file path root
    for file in "$HERE/on-event.sh" "$STUDY" "$TEMPLATE" "$HERE/lab.sh" "$TOOLS/cloud/env.sh" "$CTL" "$STUDIES"; do
        path="$(real "$file")"
        while IFS= read -r root; do
            case "$path/" in
                "$root/"*) refuse "$path is where the sandbox may write ($root): run it from the chain's checkout" ;;
            esac
        done < <(writable_roots)
    done
}

# Makes the lab's directories and renders the settings fresh into ctl/settings.json, or refuses.
prepare() {
    [ -f "$STUDY" ] && [ -f "$TEMPLATE" ] || refuse "study.md and settings.template.json belong beside on-event.sh"
    command -v jq >/dev/null 2>&1 || refuse "jq is not installed"
    mkdir -p "$CTL" "$STUDIES" "$WORK" || refuse "cannot make the directories of $LAB"
    render_settings >"$CTL/settings.json.new" && jq -e . "$CTL/settings.json.new" >/dev/null ||
        refuse "the settings do not render to JSON"
    mv -f "$CTL/settings.json.new" "$CTL/settings.json"
    jq -e "$INVARIANTS" "$CTL/settings.json" >/dev/null || refuse "the settings lack what the sandbox needs"
}

# Refuses unless the tools are there, the controls are out of the sandbox's reach and bwrap works.
preflight() {
    local tool
    prepare
    for tool in flock timeout nice bwrap socat; do
        command -v "$tool" >/dev/null 2>&1 || refuse "$tool is not installed"
    done
    [ -x "$CLAUDE" ] || refuse "no claude program at $CLAUDE"
    check_nothing_governing_is_writable
    bwrap --ro-bind / / --dev /dev --proc /proc --unshare-pid --unshare-net true >/dev/null 2>&1 ||
        refuse "bwrap cannot start a sandbox here"
}

# What a proof is about: the settings, claude and bwrap.
proof_subject() {
    printf 'settings=%s\nclaude=%s\nbwrap=%s\n' "$(sha256 "$CTL/settings.json" | awk '{print $1}')" \
        "$("$CLAUDE" --version 2>&1 | head -n 1)" "$(bwrap --version 2>&1 | head -n 1)"
}

# How the proof in ctl/proof stands for the settings, claude and bwrap now: none (there is no proof), failed (the last
# proof failed), stale (it passed for other settings or other versions of claude or bwrap) or ok.
proof_state() {
    if [ ! -f "$CTL/proof" ]; then
        echo none
    elif ! grep -qx 'result=pass' "$CTL/proof"; then
        echo failed
    elif [ "$(proof_subject)" != "$(grep -E '^(settings|claude|bwrap)=' "$CTL/proof")" ]; then
        echo stale
    else
        echo ok
    fi
}

# Why the proof does not hold, or nothing if it does.
proof_problem() {
    case "$(proof_state)" in
        none) echo "there is no proof of the sandbox yet: run on-event.sh --proof (lab.sh proof)" ;;
        failed) echo "the proof of the sandbox failed: run on-event.sh --proof again when its findings are fixed" ;;
        stale) echo "the proof is for other settings or other versions of claude or bwrap; a study renews it" ;;
    esac
}

# A proof's subject (on stdin) in a line for the log, the hash of the settings cut short.
subject_line() {
    tr '\n' ' ' | sed 's/settings=\([0-9a-f]\{12\}\)[0-9a-f]*/settings=\1/; s/ *$//'
}

# Renews a proof that passed for other settings or other versions, as claude updates itself: runs the proof, and
# refuses if it fails.  Called with the lock held, so that it never runs beside a study.
renew_stale_proof() {
    local old
    [ "$(proof_state)" = stale ] || return 0
    old="$(grep -E '^(settings|claude|bwrap)=' "$CTL/proof" | subject_line)"
    run_proof >"$CTL/proof.out" 2>&1 ||
        refuse "the proof, run again for other settings or versions, failed: see $CTL/proof.out"
    log "proof renewed: $old -> $(proof_subject | subject_line)"
}

# Whether studies/ holds a study of a long event of the game $1.
has_long_study() {
    local event
    for event in "$STUDIES/$1"-*/event.json; do
        if [ -f "$event" ] && jq -e --argjson n "$1" '.event == "long" and .n == $n' "$event" >/dev/null 2>&1; then
            return 0
        fi
    done
    return 1
}

# The number of studies for a long event, and of asks, that began on the UTC day $1 (YYYYMMDD).
studies_on_day() {
    if [ -f "$CTL/starts.log" ]; then
        awk -v day="$1" '$1 ~ "^" day && ($2 == "long" || $2 == "ask") { count++ } END { print count + 0 }' \
            "$CTL/starts.log"
    else
        echo 0
    fi
}

# --- the session ----------------------------------------------------------------------------------------------------

# Runs claude -p in LAB/work with the prompt $3, writing OUTDIR/session.jsonl (the session as JSON lines, the last the
# result), OUTDIR/stderr.txt and OUTDIR/argv.txt; $1 is the model, $2 the turns, $4 OUTDIR, a fifth argument, if any, a
# file of instructions to append to the system prompt, and a sixth the id of a session to go on with.  The session is
# kept on disk (for ask to go on with) when PERSIST is set.  Returns claude's status.
run_claude() {
    local model="$1" turns="$2" prompt="$3" out="$4" args timeout_bin nice_bin
    timeout_bin="$(command -v timeout)"
    nice_bin="$(command -v nice)"
    args=(-p --model "$model" --max-turns "$turns" --output-format stream-json --verbose
        --settings "$CTL/settings.json" --setting-sources ""
        --permission-mode dontAsk --permission-prompts none
        --tools "Bash,Read,Edit,Write,Glob,Grep"
        --strict-mcp-config --mcp-config '{"mcpServers":{}}'
        --disable-slash-commands)
    [ -n "${PERSIST:-}" ] || args+=(--no-session-persistence)
    [ -z "${5:-}" ] || args+=(--append-system-prompt-file "$5")
    [ -z "${6:-}" ] || args+=(--resume "$6")
    mkdir -p "$WORK/reports" "$WORK/tmp" "$WORK/cargo-home" "$LAB/snap"
    { printf '%q ' "$CLAUDE" "${args[@]}"; printf '\n'; } >"$out/argv.txt"
    # Nothing of a session's .claude directory outlives it, and no session reads settings from it (--setting-sources).
    rm -rf "$WORK/.claude"
    (
        cd "$WORK" || exit 97
        # A clean environment, the lock's descriptor closed, nice 19 and a time limit; no credentials but claude's own
        # login, which it reads from its files.  BASH_DEFAULT_TIMEOUT_MS is how long one command may run, an hour: in a
        # headless run a background command is otherwise stopped after ten minutes.  LAB_MAX_HOURS bounds the session.
        exec env -i HOME="$HOME" USER="$(id -un)" LOGNAME="$(id -un)" LANG=C.UTF-8 TERM=dumb \
            PATH="$HOME/.local/bin:$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin" \
            LAB_HOME="$LAB_HOME" LAB_THREADS="$LAB_THREADS" CARGO_BUILD_JOBS="$LAB_THREADS" \
            CARGO_HOME="$WORK/cargo-home" RUSTUP_HOME="$HOME/.rustup" RUSTFLAGS="-C target-cpu=native" \
            CLAUDE_CODE_TMPDIR="$WORK/tmp" CLAUDE_CODE_DISABLE_CLAUDE_MDS=1 CLAUDE_CODE_DISABLE_AUTO_MEMORY=1 \
            DISABLE_AUTOUPDATER=1 BASH_DEFAULT_TIMEOUT_MS=3600000 \
            "$timeout_bin" --kill-after=120 "${LAB_MAX_HOURS}h" "$nice_bin" -n 19 "$CLAUDE" "${args[@]}" \
            <"$prompt" >"$out/session.jsonl" 2>"$out/stderr.txt" 9>&-
    ) &
    local pid=$! status
    # claude does not end while a process the session left in the background runs, and goes on to a new turn when one
    # ends.  The session's answer is its first result: shortly after it, end the session, and with it the sandbox and
    # whatever runs in it.
    while kill -0 "$pid" 2>/dev/null; do
        if grep -q '"type":"result"' "$out/session.jsonl" 2>/dev/null; then
            sleep "${SESSION_GRACE:-20}"
            kill -TERM "$pid" 2>/dev/null
            break
        fi
        sleep 5
    done
    wait "$pid"
    status=$?
    # A session that was ended after its result succeeded.
    if [ "$status" -eq 143 ] && [ -n "$(session_result "$out")" ]; then status=0; fi
    rm -rf "$WORK/.claude"
    return "$status"
}

# The result line of the session in the directory $1, as JSON, or nothing.
session_result() {
    jq -c 'select(.type == "result")' "$1/session.jsonl" 2>/dev/null | tail -n 1
}

# What the session in the directory $1 came to, in a line: how it ended, its turns, seconds and cost; or the first
# line claude printed to stderr, when it left no result (the sandbox could not start, say).
describe_session() {
    local result
    result="$(session_result "$1")"
    if [ -n "$result" ]; then
        printf '%s\n' "$result" | jq -r '["subtype=\(.subtype)", "turns=\(.num_turns)",
            "seconds=\((.duration_ms / 1000) | floor)", "cost_usd=\(.total_cost_usd)"] | join(" ")'
    else
        printf 'no result: %s' "$(head -n 1 "$1/stderr.txt" 2>/dev/null | cut -c1-200)"
    fi
}

# --- an event -------------------------------------------------------------------------------------------------------

# The prompt of a study named $2 for the event $1 (JSON).
study_prompt() {
    cat <<EOF
An event arrived from the chain.  Carry out the study procedure in your instructions for this event.

Your study is called $2.  Your limits: at most $LAB_THREADS threads, every process at nice 19; $LAB_MAX_TURNS turns
in all, so plan for them.  The lab is ~/$LAB_HOME; its tool is $TOOLS/lab/lab.sh (use it with --local).
The time is $(date -u +%Y-%m-%dT%H:%M:%SZ).

The event, as the solver sent it:
$1
EOF
}

# Makes the files a study reads and appends to, if they are not there: the journal and the requests.
ensure_notes() {
    [ -e "$WORK/journal.md" ] || printf '# Journal of the studies\n\n' >"$WORK/journal.md"
    [ -e "$WORK/requests.md" ] || printf '%s\n' '# Requests' '' \
        'Standing questions and things to try, from whoever runs the lab (lab.sh request).  A study handles each' \
        'request that is not marked, and marks it in place with its name.  Guidance only.' '' >"$WORK/requests.md"
}

handle_event() {
    local json="$1" event n stamp dir result count
    event="$(printf '%s' "$json" | jq -er '.event' 2>/dev/null)" || refuse 'the argument is not JSON with an "event"'
    n="$(printf '%s' "$json" | jq -er '.n' 2>/dev/null)" || refuse 'the argument has no "n"'
    case "$event" in long | done) ;; *) refuse "an event \"$event\" is not one that on-event.sh knows" ;; esac
    case "$n" in '' | *[!0-9]*) refuse "n is \"$n\", not a game" ;; esac
    if [ -e "$CTL/off" ]; then skip "studies are off ($CTL/off exists); on-event.sh --on turns them on"; fi
    if [ "$event" = done ] && ! has_long_study "$n"; then
        skip "event=done n=$n: no study of a long event of game $n to compare with (it was skipped, or never sent)"
    fi
    preflight
    # No proof, or a failed one, is for a person to deal with; a proof for other versions is renewed below.
    case "$(proof_state)" in none | failed) refuse "$(proof_problem)" ;; esac
    exec 9>"$CTL/study.lock"
    if [ "$event" = long ]; then
        flock -n 9 || skip "event=$event n=$n: a study is running"
        count="$(studies_on_day "$(date -u +%Y%m%d)")"
        [ "$count" -lt "$LAB_MAX_PER_DAY" ] ||
            skip "event=$event n=$n: $count studies began today, the most is LAB_MAX_PER_DAY=$LAB_MAX_PER_DAY"
    else
        # A study of a game's end waits for the study that runs, which it follows up.
        flock -w $((LAB_MAX_HOURS * 3600 + 600)) 9 || skip "event=$event n=$n: a study is still running"
    fi
    renew_stale_proof
    stamp="$(date -u +%Y%m%dT%H%M%SZ)"
    dir="$STUDIES/$n-$stamp"
    mkdir -p "$dir" || refuse "cannot make $dir"
    printf '%s %s %s\n' "$stamp" "$event" "$n" >>"$CTL/starts.log"
    printf '%s\n' "$json" >"$dir/event.json"
    study_prompt "$json" "$n-$stamp" >"$dir/prompt.txt"
    ensure_notes
    log "start event=$event n=$n study=$dir model=$LAB_MODEL turns=$LAB_MAX_TURNS hours=$LAB_MAX_HOURS"
    PERSIST=yes run_claude "$LAB_MODEL" "$LAB_MAX_TURNS" "$dir/prompt.txt" "$dir" "$STUDY"
    local status=$?
    session_result "$dir" >"$dir/result.json"
    log "exit event=$event n=$n status=$status $(describe_session "$dir") study=$dir"
    return "$status"
}

# Puts the question $2 to the finished study $1 (the name of its folder in studies/): its session goes on, with its
# whole context, under the same sandbox, settings and rules as a study; no ask while a study runs, and it counts toward
# the day's limit.  Prints the answer, and adds the question and the answer to the study's folder.
do_ask() {
    local name="${1:-}" question="${2:-}" dir session count stamp out answer status
    [ -n "$name" ] && [ -n "$question" ] || refuse "usage: on-event.sh --ask STUDY \"QUESTION\""
    dir="$STUDIES/${name##*/}"
    [ -f "$dir/event.json" ] && [ -f "$dir/session.jsonl" ] || refuse "no study ${name##*/} in $STUDIES"
    session="$(jq -r 'select(.session_id) | .session_id' "$dir/session.jsonl" 2>/dev/null | head -n 1)"
    [ -n "$session" ] || refuse "the study ${name##*/} left no session to go on with (did it finish?)"
    if [ -e "$CTL/off" ]; then refuse "studies are off ($CTL/off exists); on-event.sh --on turns them on"; fi
    preflight
    case "$(proof_state)" in none | failed) refuse "$(proof_problem)" ;; esac
    exec 9>"$CTL/study.lock"
    flock -n 9 || refuse "a study is running: ask when it has ended"
    count="$(studies_on_day "$(date -u +%Y%m%d)")"
    [ "$count" -lt "$LAB_MAX_PER_DAY" ] ||
        refuse "$count studies and asks began today, the most is LAB_MAX_PER_DAY=$LAB_MAX_PER_DAY"
    renew_stale_proof
    stamp="$(date -u +%Y%m%dT%H%M%SZ)"
    out="$dir/ask-$stamp"
    mkdir -p "$out" || refuse "cannot make $out"
    printf '%s ask %s\n' "$stamp" "${name%%-*}" >>"$CTL/starts.log"
    ensure_notes
    cat >"$out/prompt.txt" <<EOF
The lab's operator asks you a question about this study.  It is a question, not a new event: answer it in plain words
as your reply, and do not write a new report.  You still have your tools and your rules: short measurements are fine,
within the limits (at most $LAB_THREADS threads, nice 19), and nothing through the command folder unless the question
asks for it.  You have $LAB_ASK_TURNS turns.  The time is $(date -u +%Y-%m-%dT%H:%M:%SZ).

The question:
$question
EOF
    log "start event=ask study=$dir session=$session model=$LAB_MODEL turns=$LAB_ASK_TURNS"
    PERSIST=yes run_claude "$LAB_MODEL" "$LAB_ASK_TURNS" "$out/prompt.txt" "$out" "$STUDY" "$session"
    status=$?
    session_result "$out" >"$out/result.json"
    log "exit event=ask status=$status $(describe_session "$out") study=$dir"
    answer="$(jq -r '.result // empty' "$out/result.json" 2>/dev/null)"
    [ -n "$answer" ] || refuse "no answer from the session ($(describe_session "$out")): see $out"
    printf '%s\n' "$answer"
    printf '## %s\n\nQuestion: %s\n\n%s\n\n' "$stamp" "$question" "$answer" >>"$dir/asks.md"
}

# --- the proof ------------------------------------------------------------------------------------------------------

# Evidence of a probe: its status and its output, written by the script to WORK/proof/NAME.rc and .out.
probe_rc() { cat "$WORK/proof/$1.rc" 2>/dev/null; }

PASS=0
FAIL=0
# check NAME DESCRIPTION CONDITION...: counts and prints the outcome of a condition.
check() {
    local name="$1" what="$2"
    shift 2
    if "$@"; then
        echo "  ok    $name: $what"
        PASS=$((PASS + 1))
    else
        echo "  FAIL  $name: $what"
        FAIL=$((FAIL + 1))
    fi
}

probe_failed() { local rc; rc="$(probe_rc "$1")"; [ -n "$rc" ] && [ "$rc" != 0 ]; }
probe_worked() { [ "$(probe_rc "$1")" = 0 ]; }
absent() { [ ! -e "$1" ]; }
present() { [ -e "$1" ]; }

# Writes the script a probe session runs: the settings of the probes ($1 the lines that set variables), then the body.
# Each probe `try NAME COMMAND...` leaves NAME.rc and NAME.out for the checks above.
write_probe() {
    local script="$1"
    shift
    {
        printf 'PROOF=%q\nLAB=%q\nHERE=%q\nCHAIN_HOME=%q\n' "$WORK/proof" "$LAB" "$HERE" "$HOME"
        cat <<'EOF'
try() {
    local name="$1"
    shift
    "$@" >"$PROOF/$name.out" 2>&1
    echo $? >"$PROOF/$name.rc"
}
EOF
        cat
    } >"$script"
}

probe_prompt() {
    cat <<EOF
This is a self-test of your sandbox, run by the lab's operator.  Use the Bash tool to run this one command, once:

bash $1

Show its output, and then reply with the single word DONE.  Do not run anything else.  If a command in it fails, that is
the point of the test: do not retry it, and do not look for another way to do what it tried.
EOF
}

# One stage of the proof: runs the probes of $1 (a name) in a session, with the script on stdin.
run_probe_session() {
    local name="$1" dir="$2" script="$WORK/proof/probe-$1.sh"
    mkdir -p "$WORK/proof" "$dir"
    rm -f "$WORK/proof"/*.rc "$WORK/proof"/*.out
    write_probe "$script"
    probe_prompt "$script" >"$dir/prompt.txt"
    run_claude "$LAB_PROOF_MODEL" 12 "$dir/prompt.txt" "$dir"
    local status=$?
    echo "  session of stage $name: exit $status, $(describe_session "$dir")"
}

# The proof on request: it takes the lock without waiting, and refuses while a study runs.
do_proof() {
    preflight
    exec 9>"$CTL/study.lock"
    flock -n 9 || refuse "a study is running: prove the sandbox when it has ended"
    run_proof
}

# The proof itself, with the lock held and the settings rendered (preflight): two short sessions try the writes a study
# must not make, and ctl/proof says the outcome for the subject now.  Succeeds if every check passed.
run_proof() {
    local stamp dir files before after chain_dir visible tmp_ok
    stamp="$(date -u +%Y%m%dT%H%M%SZ)"
    dir="$STUDIES/proof-$stamp"
    mkdir -p "$dir" "$WORK/proof" "$LAB/src" "$LAB/snap"
    log "proof start dir=$dir model=$LAB_PROOF_MODEL"
    echo "proof of the sandbox: settings $CTL/settings.json, $("$CLAUDE" --version 2>&1 | head -n 1)"
    files=("$HERE/on-event.sh" "$STUDY" "$TEMPLATE" "$HERE/lab.sh" "$CTL/settings.json")
    before="$(sha256 "${files[@]}")"

    echo "stage A: writes outside the writable places, and the controls"
    run_probe_session a "$dir/a" <<EOF
try outside touch "\$CHAIN_HOME/lab-probe-outside"
try lab-top touch "\$LAB/lab-probe"
try ctl-new touch "\$LAB/ctl/lab-probe"
try studies-new touch "\$LAB/studies/lab-probe"
for f in on-event.sh study.md settings.template.json lab.sh; do
    try "append-\$f" bash -c ': >> "\$1"' _ "\$HERE/\$f"
done
try append-settings bash -c ': >> "\$1"' _ "\$LAB/ctl/settings.json"
for f in settings.json settings.local.json; do
    try "plant-\$f" bash -c 'mkdir -p "\$1/work/.claude" && echo "{}" > "\$1/work/.claude/\$2"' _ "\$LAB" "\$f"
done
try work touch "\$LAB/work/lab-probe"
try src touch "\$LAB/src/lab-probe"
try snap touch "\$LAB/snap/lab-probe"
try github curl -sS -o /dev/null -m 20 https://github.com
try other curl -sS -o /dev/null -m 20 https://example.com
try tmp touch /tmp/lab-probe-tmp
ps -eo args | grep -c '[t]axman solve' >"\$PROOF/chain-visible.out"
EOF
    check outside "a write to the home directory fails" probe_failed outside
    check outside-file "and creates nothing" absent "$HOME/lab-probe-outside"
    check lab-top "a write to the lab's top directory fails" probe_failed lab-top
    check ctl-new "a write to ctl/ fails" probe_failed ctl-new
    check studies-new "a write to studies/ fails" probe_failed studies-new
    local f
    for f in on-event.sh study.md settings.template.json lab.sh; do
        check "append-$f" "appending to $f of the chain's checkout fails" probe_failed "append-$f"
    done
    check append-settings "appending to its own settings fails" probe_failed append-settings
    for f in settings.json settings.local.json; do
        check "plant-$f" "writing work/.claude/$f, which the next session might read, fails" probe_failed "plant-$f"
    done
    check no-dot-claude "no .claude directory is left in work/ after the session" absent "$WORK/.claude"
    after="$(sha256 "${files[@]}")"
    check unchanged "the controls are byte for byte as they were" test "$before" = "$after"
    check work "a write to work/ works (the session ran its commands)" probe_worked work
    check src "a write to src/ works" probe_worked src
    check snap "a write to snap/ works" probe_worked snap
    check github "github.com is reachable" probe_worked github
    check other "another host is not" probe_failed other
    visible="$(cat "$WORK/proof/chain-visible.out" 2>/dev/null)"
    tmp_ok=no
    probe_worked tmp && tmp_ok=yes
    echo "  info  chain processes visible to a sandboxed command: ${visible:-unknown}; /tmp writable: $tmp_ok"
    rm -f "$LAB/work/lab-probe" "$LAB/src/lab-probe" "$LAB/snap/lab-probe" "$HOME/lab-probe-outside" /tmp/lab-probe-tmp
    rm -f "$LAB/lab-probe" "$LAB/ctl/lab-probe" "$LAB/studies/lab-probe"

    if [ "$FAIL" -eq 0 ]; then
        echo "stage B: writes to the chain's own directories"
        local probes=""
        for chain_dir in taxman-work taxman taxman-archive; do
            [ -d "$HOME/$chain_dir" ] || continue
            probes="$probes
try chain-$chain_dir touch \"\$CHAIN_HOME/$chain_dir/lab-probe\""
        done
        run_probe_session b "$dir/b" <<<"$probes"
        for chain_dir in taxman-work taxman taxman-archive; do
            [ -d "$HOME/$chain_dir" ] || continue
            check "chain-$chain_dir" "a write to ~/$chain_dir fails" probe_failed "chain-$chain_dir"
            check "chain-$chain_dir-file" "and creates nothing (a file there would be the proof failing: remove it)" \
                absent "$HOME/$chain_dir/lab-probe"
        done
    fi
    {
        [ "$FAIL" -eq 0 ] && echo "result=pass" || echo "result=fail"
        proof_subject
        printf 'at=%s\nmodel=%s\nchain_visible=%s\ntmp_writable=%s\n' "$stamp" "$LAB_PROOF_MODEL" \
            "${visible:-unknown}" "$tmp_ok"
    } >"$CTL/proof"
    cp "$CTL/proof" "$dir/proof"
    log "proof $(head -n 1 "$CTL/proof") passed=$PASS failed=$FAIL dir=$dir"
    echo "$PASS checks passed, $FAIL failed: $(head -n 1 "$CTL/proof")"
    [ "$FAIL" -eq 0 ]
}

# --- requests -------------------------------------------------------------------------------------------------------

# Adds the request $1, dated, to work/requests.md; without one, prints the file.
do_request() {
    mkdir -p "$WORK" || refuse "cannot make $WORK"
    ensure_notes
    if [ -z "$1" ]; then
        cat "$WORK/requests.md"
    else
        printf -- '- [ ] %s %s\n' "$(date -u +%Y-%m-%dT%H:%MZ)" "$(printf '%s' "$1" | tr '\n' ' ')" \
            >>"$WORK/requests.md"
        echo "request added to $WORK/requests.md"
    fi
}

# --- status ---------------------------------------------------------------------------------------------------------

do_status() {
    local problem
    prepare
    if [ -e "$CTL/off" ]; then echo "studies: OFF ($CTL/off exists)"; else echo "studies: on"; fi
    problem="$(proof_problem)"
    if [ -n "$problem" ]; then
        echo "proof of the sandbox: NOT VALID: $problem"
    else
        echo "proof of the sandbox: valid ($(sed -n 's/^at=//p' "$CTL/proof"))"
    fi
    echo "studies for a long event today: $(studies_on_day "$(date -u +%Y%m%d)") of $LAB_MAX_PER_DAY"
    if [ -e "$CTL/study.lock" ] && ! flock -n "$CTL/study.lock" true 2>/dev/null; then
        echo "a study is running"
    else
        echo "no study is running"
    fi
    echo "the latest studies (for lab.sh ask):"
    ls -t "$STUDIES" 2>/dev/null | grep -v '^proof-' | head -n 5 | sed 's/^/  /'
    echo "the end of the log:"
    if [ -f "$CTL/on-event.log" ]; then tail -n 5 "$CTL/on-event.log" | sed 's/^/  /'; else echo "  (empty)"; fi
}

# --- main -----------------------------------------------------------------------------------------------------------

case "${1:-}" in
    --proof) do_proof ;;
    --ask) do_ask "${2:-}" "${3:-}" ;;
    --request) do_request "${2:-}" ;;
    --status) do_status ;;
    --off)
        mkdir -p "$CTL" && : >"$CTL/off" && log "studies turned off" && echo "studies are off"
        ;;
    --on)
        rm -f "$CTL/off" && log "studies turned on" && echo "studies are on"
        ;;
    '' | -*) sed -n '2,/^$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' >&2; exit 2 ;;
    *) handle_event "$1" ;;
esac
