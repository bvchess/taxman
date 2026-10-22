#!/usr/bin/env bash
# Solves units of a units file on one thread each, at nice 19, JOBS at a time: each unit once from the units file's
# incumbent (the checkpoint's) and once from the known optimum.  One jsonl line a run is appended to OUT as it ends.
#
#   runpairs.sh TUNE GAME UNITS KNOWN OUT JOBS CPU_LIMIT ID...
#
# TUNE is the tune example, KNOWN a results file holding the game's optimum, CPU_LIMIT the seconds a run may take.
set -u
tune="$1" game="$2" units="$3" known="$4" out="$5" jobs="$6" limit="$7"
shift 7
for id in "$@"; do
    printf '%s start\n%s opt\n' "$id" "$id"
done | xargs -P "$jobs" -L 1 bash -c '
    id="$0" mode="$1"
    if [ "$mode" = opt ]; then extra=(--known "'"$known"'"); else extra=(); fi
    line="$(nice -n 19 "'"$tune"'" solve "'"$game"'" --units "'"$units"'" --ids "$id" "${extra[@]}" \
        --cpu-limit "'"$limit"'" 2>/dev/null)"
    printf "%s\n" "$line" >> "'"$out"'"
'
