#!/bin/bash
# unit.sh BIN LABEL UNITS ID [tune args...]: one unit (ID root: a whole game, UNITS the game) on one thread, nice 19;
# the result line to runs/LABEL/ID.jsonl, stderr to runs/LABEL/ID.err
S=/tmp/claude-1001/-home-brian-taxman-work-research/ee866c42-a3bc-4d6b-b3f5-6d2038b6659f/scratchpad
B=$S/bin/$1; label=$2; units=$3; id=$4; shift 4
O=$S/runs/$label; mkdir -p $O
K=${KNOWN:-$S/units/chain-strict.jsonl}
if [ "$id" = root ]; then
  /usr/bin/time -f "%U %S %e" -o $O/$units-root.time nice -n 19 $B solve $units --ids root --known $K "$@" > $O/$units-root.jsonl 2> $O/$units-root.err
else
  game=${id%%-*}
  /usr/bin/time -f "%U %S %e" -o $O/$id.time nice -n 19 $B solve $game --units $units --ids $id --known $K "$@" > $O/$id.jsonl 2> $O/$id.err
fi
