#!/usr/bin/env bash
# The lab: studies of the largest subtrees of a live search, on spare threads of the machine that runs it
# (solver/docs/runbook/lab.md).  Run it from your machine, and each command is one ssh call that runs on the host and
# prints the result back; run it on the host itself, with --local or with LAB_HOST naming that machine, and the same
# commands run in place, with no ssh.  The lab's files are in its own folder on the host, beside the chain's (layout.sh
# has them all; solver/docs/runbook/lab.md, "The machine's folders", has the table).  The lab only reads and copies from
# the chain's folders there (its checkout and its data folder), writes nothing in them, and starts, stops and changes
# nothing of the chain: its processes are its own.
#
#   lab.sh [--local] setup [COMMIT]     clone the repository to src/ at COMMIT (default origin/master) and build taxman
#                                       and the example tune there, at nice 19
#   lab.sh [--local] snapshot [N] [--from DIR]
#                                       copy the checkpoint and the clock of game N in flight (default: the game whose
#                                       own search is the longest) to snap/N-STAMP/; DIR, an absolute path or from the
#                                       home, is where to copy from, instead of LAB_CKPT_DIR
#   lab.sh [--local] units SNAPDIR [TUNE ARGS]
#                                       tune units on the snapshot, into SNAPDIR/units.jsonl: the summary, then the 20
#                                       units with the largest gaps (the bound less what the incumbent needs)
#   lab.sh [--local] run SNAPDIR IDS [TUNE ARGS]
#                                       tune solve each unit of IDS (ids of units.jsonl, separated by commas) on one
#                                       thread, at most LAB_THREADS at a time, at nice 19; each result line is added to
#                                       SNAPDIR/runs.jsonl as the unit ends, and a row of a table is printed
#   lab.sh [--local] reports [N]        the reports of the studies (work/reports), the newest of game N or of all, and
#                                       the end of the journal (work/journal.md)
#   lab.sh [--local] proof              prove the sandbox of the studies on the host (on-event.sh --proof: a few short
#                                       agent sessions); every study needs it first
#   lab.sh [--local] studies [on|off]   turn the studies on or off; without an argument, say how they stand
#   lab.sh [--local] ask STUDY "QUESTION"
#                                       put a question to a finished study (the name of its folder in studies/): its
#                                       session goes on with its whole context; the answer is printed and kept
#   lab.sh [--local] request ["TEXT"]   add a dated request to work/requests.md, which every study reads after the
#                                       journal; without TEXT, print the file
#
# The studies themselves are started by `taxman solve --hook`, with the program on-event.sh of the chain's checkout on
# the host (solver/docs/runbook/lab.md, "Studies"); proof and studies run that program.
#
# SNAPDIR is a name under snap/ (the last line of snapshot says it) or a path inside the lab.  TUNE ARGS go to tune as
# they are, after the lab's own: for units --known, --sample and --seed, and for run --tactic NAME=VALUE, --gate GAP and
# --node-limit L (the top of solver/frame/examples/tune.rs says what they mean).  In them, a leading ~/ is the host's
# home.  A unit that the node limit stops has the state limit in the table.  If the connection to the host drops during
# a run, or ssh is stopped, the units already started go on to their end and add their lines to runs.jsonl; with
# --local, Ctrl-C stops them.
#
# Environment:
#   LAB_HOST       USER@HOST of the machine that runs the lab, reached by ssh with no password to type; not needed
#                  with --local                                                                              no default
#   LAB_THREADS    units that run solves at once, and jobs of the build in setup                              default 16
#   TAXMAN_HOME    the root of the chain's and the lab's folders on the host, relative to its home         see layout.sh
#   LAB_HOME       the lab's directory on the host, relative to its home                                   see layout.sh
#   LAB_CKPT_DIR   where snapshot copies from, relative to the host's home; only read                      see layout.sh
#   LAB_TOOLS      the tools of the chain's checkout on the host, relative to its home: proof and studies
#                  run LAB_TOOLS/lab/on-event.sh                                                           see layout.sh
#   SSH_CLI        ssh
# The variables of the studies (LAB_MAX_PER_DAY, LAB_MAX_TURNS, LAB_ASK_TURNS, LAB_MODEL, LAB_MAX_HOURS,
# LAB_PROOF_MODEL, CLAUDE_BIN) go to on-event.sh as they are; its top says what they mean.  The host needs git, a Rust
# toolchain (rustup's, in ~/.cargo) and jq, and for the studies what on-event.sh lists.

set -u
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../cloud/env.sh
. "$here/../cloud/env.sh"

SSH_CLI="${SSH_CLI:-ssh}"
LAB_HOST="${LAB_HOST:-}"
LOCAL=0

# --- the host's side ---------------------------------------------------------------------------------------------

# What the host runs (bash -s), after its settings: the commands as functions, then the call as the last line it reads,
# so that no program the call starts can read the rest of the script off stdin.  Everything the host writes is below
# $LAB; the chain's data folder and its checkout are only read.  The layout of the host's folders (layout.sh) comes
# before this, in the same stream.
host_lib() {
    cat <<'EOF'
REPO_URL=https://github.com/bvchess/taxman.git
: "${LAB_THREADS:=16}"
export TAXMAN_HOME LAB_HOME LAB_THREADS
LAB="$HOME/$LAB_HOME"
SRC="$LAB/src"
CRATE="$SRC/solver/frame"
TUNE="$CRATE/target/release/examples/tune"

die() {
    printf 'lab: %s\n' "$*" >&2
    exit 1
}

need() {
    command -v "$1" >/dev/null 2>&1 || die "$1 is not installed on $(hostname)"
}

# The size of a file, and the time of its last change in seconds since the epoch (GNU stat, else BSD's).
file_size() { stat -c %s "$1" 2>/dev/null || stat -f %z "$1"; }
file_time() { stat -c %Y "$1" 2>/dev/null || stat -f %m "$1"; }

# ARGS: the arguments of the call, a leading ~/ in each being the host's home.
home_args() {
    local arg
    ARGS=()
    for arg in "$@"; do
        case "$arg" in
            "~/"*) arg="$HOME/${arg#"~/"}" ;;
        esac
        ARGS+=("$arg")
    done
}

# The absolute path of the snapshot directory $1 (a name under snap/, or a path), which must be inside the lab: a run
# writes in it.
snapdir() {
    local dir="$1"
    case "$dir" in
        "~/"*) dir="$HOME/${dir#"~/"}" ;;
        /*) ;;
        *) dir="$LAB/snap/$dir" ;;
    esac
    [ -d "$dir" ] || die "no snapshot directory $dir"
    dir="$(cd "$dir" && pwd -P)"
    case "$dir" in
        "$(cd "$LAB" && pwd -P)"/*) ;;
        *) die "$dir is outside $LAB: the lab writes only there" ;;
    esac
    printf '%s\n' "$dir"
}

# Sets CKPT, the checkpoint of the snapshot directory $1, and GAME, its game.
snapshot_game() {
    local files
    files=("$1"/n*.ckpt)
    [ "${#files[@]}" -eq 1 ] && [ -f "${files[0]}" ] || die "$1 should hold one checkpoint nNNNN.ckpt"
    CKPT="${files[0]}"
    GAME="${CKPT##*/}"
    GAME="${GAME#n}"
    GAME="$((10#${GAME%.ckpt}))"
}

# Fails unless setup has built tune.
need_tune() {
    [ -x "$TUNE" ] || die "no $TUNE: run lab.sh setup first"
}

# The game in flight with the longest own search: the largest clock (the seconds its search has run, nNNNN.clock beside
# its checkpoint) among the checkpoints of the directory $1, 0 for a checkpoint with no clock.
longest_search() {
    local ckpt name n seconds
    for ckpt in "$1"/n*.ckpt; do
        [ -f "$ckpt" ] || continue
        name="${ckpt##*/}"
        n="${name#n}"
        n="${n%.ckpt}"
        case "$n" in '' | *[!0-9]*) continue ;; esac
        seconds="$(awk 'NR == 1 { print $1; exit }' "${ckpt%.ckpt}.clock" 2>/dev/null)"
        printf '%s %s\n' "${seconds:-0}" "$((10#$n))"
    done | sort -k1,1gr -k2,2nr | head -n 1 | cut -d' ' -f2
}

lab_setup() {
    local commit="${1:-origin/master}"
    need git
    if [ -f "$HOME/.cargo/env" ]; then
        . "$HOME/.cargo/env"
    fi
    need cargo
    mkdir -p "$LAB"
    if [ -d "$SRC/.git" ]; then
        git -C "$SRC" fetch --quiet origin || die "cannot fetch $REPO_URL"
    else
        git clone --quiet "$REPO_URL" "$SRC" || die "cannot clone $REPO_URL"
    fi
    git -C "$SRC" checkout --quiet --detach "$commit" || die "no commit $commit in $REPO_URL"
    git -C "$SRC" log -1 --format='source: %h %s' | cut -c1-120
    (cd "$CRATE" && RUSTFLAGS='-C target-cpu=native' nice -n 19 cargo build --release --features studies \
        --bin taxman --example tune -j "$LAB_THREADS" </dev/null 2>&1) || die "the build failed"
    need_tune
    ls -l "$CRATE/target/release/taxman" "$TUNE" | sed 's/^/built: /'
}

lab_snapshot() {
    local dir="$HOME/$LAB_CKPT_DIR" game="" name ckpt clock stamp dest
    # [N] [--from DIR]: DIR is an absolute path, or from the home, and is only read.
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --from)
                [ "$#" -ge 2 ] || die "usage: lab.sh snapshot [N] [--from DIR]"
                case "$2" in /*) dir="$2" ;; "~/"*) dir="$HOME/${2#"~/"}" ;; *) dir="$HOME/$2" ;; esac
                shift 2
                ;;
            *[!0-9]* | '') die "usage: lab.sh snapshot [N] [--from DIR]" ;;
            *)
                game="$1"
                shift
                ;;
        esac
    done
    if [ -z "$game" ]; then
        game="$(longest_search "$dir")"
        [ -n "$game" ] || die "no checkpoint in $dir: no game has run long enough to keep one"
    fi
    game="$((10#$game))"
    name="$(printf 'n%04d' "$game")"
    ckpt="$dir/$name.ckpt"
    clock="$dir/$name.clock"
    [ -f "$ckpt" ] || die "no checkpoint $ckpt: game $game is not in flight"
    stamp="$(date -u +%Y%m%dT%H%M%SZ)"
    dest="$LAB/snap/$game-$stamp"
    mkdir -p "$LAB/snap" && mkdir "$dest" 2>/dev/null || die "cannot make $dest (a snapshot of the game this second?)"
    # A plain cp is safe: the engine writes a checkpoint to nNNNN.ckpt.tmp, syncs it and renames it over the old one
    # (solver/frame/src/engine.rs, "Checkpoint"), and the clock the same way (src/solver/clock.rs), so a copy sees one
    # whole file, the old or the new.  The checkpoint carries a checksum, which tune checks when it reads it.
    cp -p "$ckpt" "$dest/" || die "cannot copy $ckpt"
    if [ -f "$clock" ]; then
        cp -p "$clock" "$dest/" || die "cannot copy $clock"
    fi
    printf 'snapshot %s\n' "${dest##*/}"
    printf '  %s.ckpt: %s bytes, written %s s before the copy\n' "$name" "$(commas "$(file_size "$dest/$name.ckpt")")" \
        "$(($(date +%s) - $(file_time "$dest/$name.ckpt")))"
    if [ -f "$dest/$name.clock" ]; then
        printf '  own search: %s s (%s.clock)\n' "$(awk 'NR == 1 { print $1; exit }' "$dest/$name.clock")" "$name"
    fi
}

# The number of units of the units file $1, and the 20 with the largest gaps, by gap alone.
top_units() {
    jq -r -s '[.[] | select(.kind == "unit")]
        | "\(length) units, \(map(select(.gap != null)) | length) with a gap"' "$1"
    printf '%-14s %6s %6s %9s %9s %9s\n' unit depth cands bound need gap
    jq -r -s '[.[] | select(.kind == "unit" and .gap != null)] | sort_by(-.gap)[:20][]
        | [.id, .depth, .n_cands, .bound, .need, .gap] | @tsv' "$1" |
        awk -F'\t' "$AWK_COMMAS"'{ printf "%-14s %6s %6s %9s %9s %9s\n", $1, $2, $3, commas($4), commas($5),
            commas($6) }'
}

lab_units() {
    [ "$#" -ge 1 ] || die "usage: lab.sh units SNAPDIR [TUNE ARGS]"
    need jq
    need_tune
    local snap
    snap="$(snapdir "$1")"
    shift
    home_args "$@"
    snapshot_game "$snap"
    printf 'snapshot %s: game %s\n' "${snap##*/}" "$GAME"
    # All the units unless TUNE ARGS say --sample K; the summary is on tune's stderr.
    nice -n 19 "$TUNE" units "$GAME" "$CKPT" --sample 1000000 --out "$snap/units.jsonl" "${ARGS[@]}" </dev/null 2>&1 ||
        die "tune units failed"
    top_units "$snap/units.jsonl"
}

# Rows of the table of run, from result lines on stdin: the unit, its steps, the CPU seconds of its thread and the wall
# seconds, the best promotion sum, and whether the unit was solved (done) or stopped by the node limit (limit).
table_rows() {
    jq -r '[.id, .nodes, .cpu_s, .wall_s, .best, .completed] | @tsv' |
        awk -F'\t' "$AWK_COMMAS"'{ printf "%-14s %16s %10.1f %10.1f %9s  %s\n", $1, commas($2), $3, $4, commas($5),
            ($6 == "true" ? "done" : "limit") }'
}

# Solves the unit $1 of UNITS, adds its result line to RUNS and prints its row; run in the background.  tune is a direct
# child of this shell, so that stop_jobs reaches it.
solve_unit() {
    local id="$1" out err line
    out="$(mktemp "$SNAP/.out.XXXXXX")"
    err="$(mktemp "$SNAP/.err.XXXXXX")"
    if nice -n 19 "$TUNE" solve "$GAME" --units "$UNITS" --ids "$id" "${ARGS[@]}" </dev/null >"$out" 2>"$err" &&
        line="$(jq -c --arg commit "$COMMIT" '. + {commit: $commit}' "$out")" && [ -n "$line" ]; then
        # One write of a line to a file opened for append: units that end together do not mix their lines.
        printf '%s\n' "$line" >>"$RUNS"
        rm -f "$out" "$err"
        # Last, because the connection that reads the row may be gone: the unit's line is in RUNS all the same.
        printf '%s\n' "$line" | table_rows
    else
        line="$(printf '%-14s FAILED: %s' "$id" "$(tail -n 1 "$err")")"
        rm -f "$out" "$err"
        printf '%s\n' "$line"
        return 1
    fi
}

# Stops the units that still run, and removes what they left in the snapshot directory.
stop_jobs() {
    local pid
    for pid in $(jobs -p); do
        pkill -TERM -P "$pid" 2>/dev/null
        kill -TERM "$pid" 2>/dev/null
    done
    rm -f "$SNAP"/.out.* "$SNAP"/.err.*
}

lab_run() {
    [ "$#" -ge 2 ] || die "usage: lab.sh run SNAPDIR IDS [TUNE ARGS]"
    need jq
    need_tune
    case "$LAB_THREADS" in '' | *[!0-9]* | 0) die "LAB_THREADS is \"$LAB_THREADS\": it should be 1 or more" ;; esac
    local ids id pid pids=() failed=0
    SNAP="$(snapdir "$1")"
    IFS=, read -r -a ids <<<"$2"
    shift 2
    home_args "$@"
    snapshot_game "$SNAP"
    UNITS="$SNAP/units.jsonl"
    RUNS="$SNAP/runs.jsonl"
    [ -f "$UNITS" ] || die "no $UNITS: run lab.sh units first"
    COMMIT="$(git -C "$SRC" rev-parse --short HEAD)"
    for id in "${ids[@]}"; do
        case "$id" in
            root | *-root) ;;
            *) jq -r 'select(.kind == "unit") | .id' "$UNITS" | grep -Fxq -- "$id" || die "no unit $id in $UNITS" ;;
        esac
    done
    printf 'snapshot %s: game %s, units to solve: %s, at a time: %s, build %s\n' "${SNAP##*/}" "$GAME" "${#ids[@]}" \
        "$LAB_THREADS" "$COMMIT"
    printf '%-14s %16s %10s %10s %9s  %s\n' unit steps cpu_s wall_s best state
    trap 'stop_jobs; exit 130' HUP INT TERM
    for id in "${ids[@]}"; do
        while [ "$(jobs -rp | wc -l)" -ge "$LAB_THREADS" ]; do
            sleep 1
        done
        solve_unit "$id" &
        pids+=("$!")
    done
    for pid in "${pids[@]}"; do
        wait "$pid" || failed=$((failed + 1))
    done
    [ "$failed" -eq 0 ] || die "$failed of ${#ids[@]} units failed"
    printf 'results added to %s\n' "$RUNS"
}

# The reports of the studies, newest first, the newest (of game $1, if given) in full, and the end of the journal.
lab_reports() {
    local reports="$LAB/work/reports" game="${1:-}" newest
    [ -d "$reports" ] && [ -n "$(ls -A "$reports")" ] || die "no reports yet in $reports"
    printf 'reports (newest first):\n'
    ls -t "$reports" | sed 's/^/  /'
    # shellcheck disable=SC2012 # the names are ours: N-STAMP.md
    newest="$(ls -t "$reports"/${game:+$game-}*.md 2>/dev/null | head -n 1)"
    [ -n "$newest" ] || die "no report${game:+ of game $game}"
    printf '\n== %s\n' "${newest##*/}"
    cat "$newest"
    if [ -f "$LAB/work/journal.md" ]; then
        printf '\n== the end of work/journal.md\n'
        tail -n 40 "$LAB/work/journal.md"
    fi
}

# The program of the studies, in the chain's checkout.
on_event() {
    local program="$HOME/$LAB_TOOLS/lab/on-event.sh"
    [ -x "$program" ] || die "no $program: the chain's checkout lacks the studies (LAB_TOOLS says where its tools are)"
    "$program" "$@"
}

lab_proof() {
    [ "$#" -eq 0 ] || die "usage: lab.sh proof"
    on_event --proof
}

lab_studies() {
    case "${1:-status}" in
        on) on_event --on ;;
        off) on_event --off ;;
        status) on_event --status ;;
        *) die "usage: lab.sh studies [on|off]" ;;
    esac
}

lab_ask() {
    [ "$#" -eq 2 ] || die 'usage: lab.sh ask STUDY "QUESTION"'
    on_event --ask "$1" "$2"
}

lab_request() {
    [ "$#" -le 1 ] || die 'usage: lab.sh request ["TEXT"]'
    on_event --request "${1:-}"
}

EOF
}

# --- your machine's side ------------------------------------------------------------------------------------------

usage() {
    sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//; /^$/d' >&2
    exit 2
}

# Whether the lab runs on this machine: it was asked to (--local), or LAB_HOST names this machine (the first label of
# its host name is this machine's).
runs_here() {
    local host="${LAB_HOST#*@}"
    [ "$LOCAL" = 1 ] && return 0
    [ -n "$LAB_HOST" ] && [ "${host%%.*}" = "$(hostname -s 2>/dev/null || hostname)" ]
}

# Runs lab_COMMAND ARGS on the host: the settings, the layout of the folders, the commas function that its tables use,
# its functions, and the call, to a bash here or, over one ssh call, to a bash there.
remote() {
    local command="$1" var
    shift
    if ! runs_here; then
        [ -n "$LAB_HOST" ] || die "LAB_HOST is not set: LAB_HOST=USER@HOST $0 $command ... (--local on the host)"
        # USER@HOST, so that ssh cannot take it for an option.
        case "$LAB_HOST" in
            -* | *[!A-Za-z0-9_.:@%+-]* | @* | *@ | *@*@*) die "LAB_HOST: \"$LAB_HOST\" is not USER@HOST" ;;
            *@*) ;;
            *) die "LAB_HOST: \"$LAB_HOST\" is not USER@HOST" ;;
        esac
    fi
    {
        for var in TAXMAN_HOME LAB_HOME LAB_THREADS LAB_CKPT_DIR LAB_TOOLS LAB_MAX_PER_DAY LAB_MAX_TURNS LAB_ASK_TURNS \
            LAB_MODEL LAB_MAX_HOURS LAB_PROOF_MODEL CLAUDE_BIN; do
            [ -z "${!var:-}" ] || printf 'export %s=%q\n' "$var" "${!var}"
        done
        cat "$here/layout.sh"
        declare -p AWK_COMMAS
        declare -f commas
        host_lib
        printf 'lab_%s' "$command"
        [ "$#" -eq 0 ] || printf ' %q' "$@"
        printf '\n'
    } | if runs_here; then
        bash -s
    else
        "$SSH_CLI" -o BatchMode=yes -o ConnectTimeout=60 -o ConnectionAttempts=3 -o ServerAliveInterval=30 \
            "$LAB_HOST" 'bash -s'
    fi
}

if [ "${1:-}" = --local ]; then
    LOCAL=1
    shift
fi
command="${1:-}"
[ "$#" -eq 0 ] || shift
case "$command" in
    setup)
        [ "$#" -le 1 ] || usage
        case "${1:-}" in -*) die "COMMIT: \"$1\" is not a commit" ;; esac
        remote setup "$@"
        ;;
    snapshot)
        [ "$#" -le 3 ] || usage
        remote snapshot "$@"
        ;;
    units)
        [ "$#" -ge 1 ] || usage
        remote units "$@"
        ;;
    run)
        [ "$#" -ge 2 ] || usage
        remote run "$@"
        ;;
    reports)
        [ "$#" -le 1 ] || usage
        case "${1:-0}" in *[!0-9]*) die "N: \"$1\" is not a game" ;; esac
        remote reports "$@"
        ;;
    proof)
        [ "$#" -eq 0 ] || usage
        remote proof
        ;;
    studies)
        [ "$#" -le 1 ] || usage
        case "${1:-status}" in on | off | status) ;; *) usage ;; esac
        remote studies "$@"
        ;;
    ask)
        [ "$#" -eq 2 ] || usage
        remote ask "$@"
        ;;
    request)
        [ "$#" -le 1 ] || usage
        remote request "$@"
        ;;
    *) usage ;;
esac
