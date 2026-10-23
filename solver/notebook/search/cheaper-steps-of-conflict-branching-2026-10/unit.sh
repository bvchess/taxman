#!/bin/bash
# unit.sh BIN LABEL UNITS ID [tune args...]: tune solve one unit (UNITS a units.jsonl; ID root for a whole game,
# then UNITS is the game number), one thread, nice 19, the environment passed through; the result line to
# runs/LABEL/ID.jsonl, stderr to runs/LABEL/ID.err
W=/home/brian/taxman-lab/work/research-stepcost
B=$W/bin/$1; label=$2; units=$3; id=$4; shift 4
O=$W/runs/$label; mkdir -p $O
K=/home/brian/taxman-work/results/strict.jsonl
if [ "$id" = root ]; then
  nice -n 19 $B solve $units --ids root --known $K "$@" > $O/$units-root.jsonl 2> $O/$units-root.err
else
  game=${id%%-*}
  nice -n 19 $B solve $game --units $units --ids $id --known $K "$@" > $O/$id.jsonl 2> $O/$id.err
fi
