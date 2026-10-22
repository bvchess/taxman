#!/usr/bin/env bash
# Runs the finder alone (examples/finder.rs of the local branch research-wall, with --store and the stop at --target)
# once for each random seed, JOBS at a time, one thread each at nice 19; each run's line is appended to OUT.
#
#   finderruns.sh FINDER JOBS OUT GAME TARGET SECONDS "SEEDS" [FINDER ARGS]...
#
# SEEDS is a list of random seeds separated by spaces; FINDER ARGS go to the finder as they are (--known FILE, --store,
# --no-seed).  Run it from the repository's directory, where the finder finds the bounds store.
set -u
finder="$1" jobs="$2" out="$3" game="$4" target="$5" seconds="$6" seeds="$7"
shift 7
printf '%s\n' $seeds | xargs -P "$jobs" -I{} bash -c '
    nice -n 19 "$0" "$1" --target "$2" --seconds "$3" --seeds {} "${@:5}" 2>/dev/null | grep "\"random\"" >> "$4"
' "$finder" "$game" "$target" "$seconds" "$out" "$@"
