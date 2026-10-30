#!/usr/bin/env bash
# Where a chain is: the games recorded and the games in flight, read from the machine that runs it, over ssh.
#
#   solver/tools/cloud/status.sh [--recorded N] --host USER@HOST
#
# One ssh call reads the state of the chain (chain.sh in the tmux session tx) in its data folder on that machine, whose
# place is in solver/tools/cloud/layout.sh.  It needs no AWS profile and makes no aws call.  It shows the machine's
# uptime, whether the chain runs, how it ended (chain.outcome), the window and the last game of the range (chain.log),
# when logs/chain.err last changed, the game the results file is recorded through, and a table: a line for each of the
# last games recorded and then for each game in flight (solver/docs/runbook/chain.md, "From another machine").
#
# options:
#   --recorded N          the games recorded that the table shows: the last N (0: none)               default 6
#   --host USER@HOST      the machine that runs the chain.  ssh runs with BatchMode: the key must need no password
# Other environment: SSH_CLI (ssh).

set -u
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"
# The folders of a machine that is not a cloud machine, which the host's chain keeps its state in.
# shellcheck source=layout.sh
. "$here/layout.sh"

SSH_CLI="${SSH_CLI:-ssh}"
SHOW_RECORDED=6
STATUS_HOST=""

# The comment at the top of this file.
usage() {
    sed -n '2,/^$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

is_whole_number() {
    case "$1" in '' | *[!0-9]*) return 1 ;; esac
    return 0
}

# A span of SECONDS in words: 45 s, 4 min 30 s, 2 h 5 min, 3 d 4 h.
span_text() {
    awk -v s="$1" 'BEGIN {
        if (s < 0) s = 0
        if (s < 60) printf "%d s\n", s
        else if (s < 3600) printf "%d min %d s\n", s / 60, s % 60
        else if (s < 86400) printf "%d h %d min\n", s / 3600, s / 60 % 60
        else printf "%d d %d h\n", s / 86400, s / 3600 % 24 }'
}

# What the line of a game that is not recorded has (an extended regular expression, for grep -E): the solver prints one
# for a game whose search a limit stopped, with `incomplete` where a recorded game has `optimal`, and the line ends
# there or, from a solver that prints the own search time, goes on to `own=`.
INCOMPLETE_LINE=' incomplete($|  own=)'

# The last line of each of the COUNT highest games up to THROUGH among the lines of recorded games on stdin
# (logs/chain.out), the lowest game first.  A replacement machine may print a game again after an interruption: the last
# line of a game is the one that counts.
recorded_games() {
    awk -v count="$1" -v through="$2" '
    { n = substr($1, 3) + 0; if (n <= through) line[n] = $0 }
    END {
        for (n = through; n >= 1 && shown < count; n--) if (n in line) game[++shown] = n
        for (i = shown; i >= 1; i--) print line[game[i]]
    }'
}

# The games, a line each: the recorded games, then those in flight.  The text on stdin is the lines of the recorded
# games (recorded_games), then the progress lines of the solver (logs/chain.err).  A recorded game's line, `n=4864
# score=7437375   search  nodes=1133462769   4636.026 s  optimal unknown  proof=exhausted  own=3011.250 s  best=341541`,
# gives the columns of the row: `nodes`; `own search`, the line's own search time (`own=`, the game's clock over every
# run of it); and `best`, its best sum of promotions (`best=`, the score less `in_the_bag`, or `-`).  A line that an
# older solver printed has neither field, and its row shows `-` for both.  The seconds of the line are not shown: they
# count the time the game waited behind others.  A game in flight whose search has started prints one progress line
# every PROGRESS seconds, `n=3930 1h45m00s nodes 1086013550 (242.8k/s) best 341541 queue ...` (the time since the search
# began, which the table does not show; the nodes so far; the nodes a second in the last interval, or `done`,
# `optimal`, `stopped`, `failed` or `panicked` in the last line; the best sum of promotions so far), and nothing else
# says where a game is: a game with no recent line has no state to tell (a setup, a prime or repeated game, a search
# not started).  Its `own search` is its clock (ckpt/nNNNN.clock).  Arguments: the first game in flight, how many
# games are in flight, and the games' clocks as "n seconds n seconds ...".  A line is recent unless three lines of
# another game in flight follow it (a game that still searches prints once an interval).  The state of a game in flight
# is told from its last line:
#   running      the nodes grow (1000 a second or more in the last interval)
#   suspended    the search has started, but the threads are another game's, and the nodes do not grow
#   search done  the search has ended (`done` or `optimal`) and the game is not recorded yet: the oldest game is being
#                recorded, any other waits for the games before it
games_table() {
    awk -v first="$1" -v flight="$2" -v clocks="$3" "$AWK_COMMAS"'
    function duration(s) {
        if (s < 60) return sprintf("%.1fs", s)
        if (s < 3600) return sprintf("%dm%02ds", s / 60, s % 60)
        return sprintf("%dh%02dm%02ds", s / 3600, s / 60 % 60, s % 60)
    }
    # Nodes a second, from the rate of a line: 0, 1, 236.5k, 1.20M, ...
    function rate(text,    unit) {
        sub("/s$", "", text)
        unit = substr(text, length(text))
        text = text + 0
        if (unit == "k") text *= 1e3
        else if (unit == "M") text *= 1e6
        else if (unit == "G") text *= 1e9
        return text
    }
    function row(n, own, nodes, best, state) {
        printf "  %5s  %-10s  %14s  %9s  %s\n", n, own, commas(nodes), commas(best), state
    }
    BEGIN {
        first += 0
        last = first + flight - 1
        count = split(clocks, c, " ")
        for (i = 1; i < count; i += 2) held[c[i] + 0] = c[i + 1] + 0
        row("n", "own search", "nodes", "best", "state")
    }
    # A recorded game, a row at once, so that the recorded games come above those in flight.  Its state is its score,
    # its method, its proof (the text after `proof=` up to the next two spaces) and `optimal ok`, if the solver found
    # the game in its table of optimal games.  Its own search time and its best sum are the last two fields of the line
    # (`  own=3011.250 s  best=341541`), if the solver that printed it had them.
    $1 ~ /^n=[0-9]+$/ && $2 ~ /^score=[0-9]+$/ && $4 ~ /^nodes=[0-9]+$/ && $6 == "s" {
        detail = $3
        where = index($0, "  proof=")
        if (where) {
            proof = substr($0, where + 8)
            sub(/  .*/, "", proof)
            detail = detail ", " proof
        }
        if ($7 == "optimal" && $8 != "unknown") detail = detail ", optimal " $8
        own_time = "-"
        best_sum = "-"
        if (match($0, /  own=[0-9]+([.][0-9]+)? s  best=([0-9]+|-)$/)) {
            split(substr($0, RSTART + 2), ends, " ")
            own_time = duration(substr(ends[1], 5) + 0)
            best_sum = substr(ends[3], 6)
        }
        row(substr($1, 3), own_time, substr($4, 7), best_sum,
            "recorded, score " commas(substr($2, 7)) " (" detail ")")
    }
    $1 ~ /^n=[0-9]+$/ && $3 == "nodes" && $4 ~ /^[0-9]+$/ && $5 ~ /^\(.*\)$/ && $6 == "best" && $8 == "queue" {
        n = substr($1, 3) + 0
        how = substr($5, 2, length($5) - 2)
        lines++
        game[lines] = n
        finished[lines] = (how !~ /\/s$/)
        at[n] = lines
        nodes[n] = $4
        best[n] = $7
        last_how[n] = how
    }
    END {
        for (g = first; g <= last; g++) {
            own = (g in held) ? duration(held[g]) : "-"
            how = (g in at) ? last_how[g] : ""
            searching = (how != "done" && how != "optimal")
            stale = 0
            if (g in at && searching) {
                split("", printed)
                for (i = at[g] + 1; i <= lines; i++) {
                    h = game[i]
                    if (h != g && h >= first && h <= last && !finished[i] && ++printed[h] >= 3) stale = 1
                }
            }
            if (!(g in at) || stale) {
                row(g, own, "-", "-", "no recent progress line")
                continue
            }
            if (!searching) {
                state = (g == first) ? "search done, being recorded" : "search done, waiting for the game before"
            } else if (how == "stopped") {
                state = "search stopped, its checkpoint kept"
            } else if (how == "failed" || how == "panicked") {
                state = "search " how
            } else if (rate(how) < 1000) {
                state = "suspended"
            } else {
                state = how
                sub("/s$", " nodes/s", state)
                state = "running, " state
            }
            row(g, own, nodes[g], best[g], state)
        }
    }'
}

# The line above the table of games: the RECORDED games it shows, and the IN_FLIGHT games in flight (WINDOW is the most
# at a time).  No line for a table with no game.
table_heading() {
    local recorded="$1" in_flight="$2" window="$3" what=""
    case "$recorded" in
        0) ;;
        1) what="the last game recorded" ;;
        *) what="the last $recorded games recorded" ;;
    esac
    if [ "$in_flight" -gt 0 ]; then
        if [ -n "$what" ]; then what="$what and the $in_flight in flight"; else what="games in flight"; fi
        what="$what (window $window)"
    fi
    [ -z "$what" ] || printf '  %s:\n' "$what"
}

# The games of the chain.  Arguments: the game recorded through and its score (both empty for no game), the window, the
# last game of the chain, the lines of the recorded games (logs/chain.out) and whether there are progress lines
# (logs/chain.err) to read.  It prints the line of the game recorded through, the heading, and the table of the last
# SHOW_RECORDED games recorded and the games in flight after the game recorded through.
chain_games() {
    local recorded="$1" score="$2" window="$3" chain_to="$4" lines="$5" have_progress="$6"
    local first_n last_n in_flight shown shown_count name n seconds clocks=""
    if [ -n "$recorded" ]; then
        printf '  recorded through %s (score %s)\n' "$recorded" "$(commas "$score")"
    else
        printf '  recorded through: no game yet\n'
        recorded=0
    fi
    first_n=$((recorded + 1))
    last_n=$((recorded + window))
    [ "$last_n" -le "$chain_to" ] || last_n="$chain_to"
    in_flight=$((last_n - first_n + 1))
    [ "$in_flight" -gt 0 ] || in_flight=0
    while read -r name seconds; do
        [ -n "$name" ] || continue
        n="${name#n}"
        n=$((10#${n%.clock}))
        if [ "$n" -ge "$first_n" ] && [ "$n" -le "$last_n" ] && [ -n "$seconds" ]; then
            clocks="$clocks $n $seconds"
        fi
    done <<<"$HOST_CLOCKS"
    shown="$(printf '%s\n' "$lines" | recorded_games "$SHOW_RECORDED" "$recorded")"
    shown_count="$(printf '%s\n' "$shown" | grep -c .)"
    table_heading "$shown_count" "$in_flight" "$window"
    if [ "$in_flight" -gt 0 ] && [ "$have_progress" = 1 ]; then
        { printf '%s\n' "$shown"; host_section err; } | games_table "$first_n" "$in_flight" "$clocks"
    else
        [ "$shown_count" -eq 0 ] || printf '%s\n' "$shown" | games_table "$first_n" 0 ""
        if [ "$in_flight" -gt 0 ]; then
            printf '  no progress lines yet\n'
        else
            printf '  no game in flight: the range to %s is recorded\n' "$chain_to"
        fi
    fi
}

# The script that the host runs (bash -s), one ssh call for all that status shows: sections, each after a line @@NAME,
# of the state of a chain in its data folder.  The results file's last lines are enough to find the last game
# completed, and the last 3000 lines of chain.err to tell the games in flight apart.
host_script() {
    printf 'incomplete_line=%q\nwork="$HOME"/%q\n' "$INCOMPLETE_LINE" "$CHAIN_DATA"
    cat <<'EOF'
changed() { stat -c %Y "$1" 2>/dev/null || stat -f %m "$1" 2>/dev/null; }
echo @@now
date +%s
echo @@uptime
uptime
echo @@session
if tmux has-session -t tx 2>/dev/null; then echo yes; else echo no; fi
echo @@solver
if ps -eo args | grep -q '[t]axman solve'; then echo yes; else echo no; fi
echo @@outcome
cat "$work/chain.outcome" 2>/dev/null
echo @@start
grep -E ' chain [0-9]+-[0-9]+; threads: ' "$work/logs/chain.log" 2>/dev/null | tail -n 1
echo @@errtime
changed "$work/logs/chain.err"
echo @@err
tail -n 3000 "$work/logs/chain.err" 2>/dev/null
echo @@result
tail -n 100 "$work/results/strict.jsonl" 2>/dev/null | grep '"completed":true' | tail -n 1 |
    grep -o '"n":[0-9]*,"score":[0-9]*' | head -n 1
echo @@out
grep '^n=' "$work/logs/chain.out" 2>/dev/null | grep -Ev "$incomplete_line"
echo @@clocks
for f in "$work"/ckpt/n*.clock; do
    [ -f "$f" ] && echo "$(basename "$f") $(awk '{print $1; exit}' "$f")"
done
true
EOF
}

# The section NAME of the host's answer (HOST_DATA), without its marker.
host_section() {
    printf '%s\n' "$HOST_DATA" | awk -v marker="@@$1" '$0 == marker { on = 1; next } /^@@/ { on = 0 } on'
}

# What the chain on the host shows: its state is in its data folder there, read over ssh.
status_host() {
    local now session solver running outcome start window chain_to changed result recorded score have_progress=0
    HOST_DATA="$(host_script | "$SSH_CLI" -o BatchMode=yes -o ConnectTimeout=10 "$STATUS_HOST" 'bash -s')" ||
        die "cannot read the chain on $STATUS_HOST over ssh"
    now="$(host_section now)"
    [ -n "$now" ] || die "$STATUS_HOST did not answer as expected: $(printf '%s\n' "$HOST_DATA" | head -n 1)"
    printf 'host %s\n' "$STATUS_HOST"
    printf '  %s\n' "$(host_section uptime | sed 's/^ *//')"
    session="$(host_section session)"
    solver="$(host_section solver)"
    running="is NOT running"
    if [ "$session" = yes ] && [ "$solver" = yes ]; then running="is running"; fi
    if [ "$session" = yes ]; then session="the tmux session tx exists"; else session="no tmux session tx"; fi
    if [ "$solver" = yes ]; then solver="a taxman solve process is alive"; else solver="no taxman solve process"; fi
    printf '  the chain %s (%s, %s)\n' "$running" "$session" "$solver"
    outcome="$(host_section outcome)"
    if [ -n "$outcome" ]; then printf '  ENDED: %s\n' "$outcome"; fi

    start="$(host_section start)"
    window="$(printf '%s\n' "$start" | sed -n 's/.* window \([0-9][0-9]*\);.*/\1/p')"
    window="${window:-4}"
    chain_to="$(printf '%s\n' "$start" | sed -n 's/.* chain 1-\([0-9][0-9]*\);.*/\1/p')"
    chain_to="${chain_to:-8191}"
    if [ -n "$start" ]; then
        printf '  chain 1-%s, window %s\n' "$chain_to" "$window"
    else
        printf '  no line of chain.log starts the chain yet\n'
    fi
    changed="$(host_section errtime)"
    if [ -n "$changed" ]; then
        printf '  last progress line: %s, %s ago\n' "$(epoch_local "$changed")" "$(span_text $((now - changed)))"
        have_progress=1
    else
        printf '  no progress line yet: no logs/chain.err\n'
    fi
    # The results file holds the whole chain; chain.out only the games of this machine's run.
    result="$(host_section result)"
    recorded="$(printf '%s\n' "$result" | sed -n 's/^"n":\([0-9]*\),.*/\1/p')"
    score="$(printf '%s\n' "$result" | sed -n 's/.*"score":\([0-9]*\)$/\1/p')"
    HOST_CLOCKS="$(host_section clocks)"
    chain_games "$recorded" "$score" "$window" "$chain_to" "$(host_section out)" "$have_progress"
}

# --- main --------------------------------------------------------------------------------------------------------

while [ "$#" -gt 0 ]; do
    case "$1" in
        --recorded)
            [ "$#" -ge 2 ] || die "--recorded needs a value (see the top of $0)"
            is_whole_number "$2" || die "--recorded: \"$2\" is not a whole number"
            SHOW_RECORDED="$2"
            shift
            ;;
        --host)
            [ "$#" -ge 2 ] || die "--host needs a value (see the top of $0)"
            # USER@HOST, so that ssh cannot take it for an option.
            case "$2" in
                -* | *[!A-Za-z0-9_.:@%+-]* | @* | *@ | *@*@*) die "--host: \"$2\" is not USER@HOST" ;;
                *@*) ;;
                *) die "--host: \"$2\" is not USER@HOST" ;;
            esac
            STATUS_HOST="$2"
            shift
            ;;
        -h | --help | help)
            usage
            exit 0
            ;;
        *) die "usage: status.sh [--recorded N] --host USER@HOST" ;;
    esac
    shift
done
[ -n "$STATUS_HOST" ] || die "usage: status.sh [--recorded N] --host USER@HOST"
status_host
