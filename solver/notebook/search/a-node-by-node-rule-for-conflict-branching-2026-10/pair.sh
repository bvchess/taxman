#!/bin/bash
# Solves units under two settings at once on the two hyperthreads of one physical core (cores FIRST.., siblings 44
# above), so that both see the same core, caches and clock; LANES cores at once (four by default, from FIRST=36), the
# hyperthreads swapped on even runs.  One file a solve, OUT/NAME-ID-REP.json (stderr in .err); resumable.
# Usage: pair.sh SETFILE IDSFILE OUT REPS NODELIMIT, SETFILE holding two lines "NAME BINARY ENV ARGS..." as run.sh's.
set=$1; ids=$2; out=$3; reps=$4; nl=$5
mkdir -p "$out"
K=/home/brian/taxman_work/chain-data/results/strict.jsonl
mapfile -t lines < <(grep -v '^#' "$set" | grep -v '^$')
run() {
  cpu=$1; line=$2; id=$3; rep=$4
  read -r s bin envs args <<< "$line"
  f="$out/$s-$id-$rep"
  [ -s "$f.json" ] && return
  game=${id%%-*}
  case $game in
    4983) units=/home/brian/taxman_work/lab/work/hardgames/forecast/units/live4983.jsonl; store= ;;
    4984) units=/home/brian/taxman_work/lab/work/patch/units-4984.jsonl
          store=/home/brian/taxman_work/lab/work/carry/s1/store-4983-144.txt ;;
    4991) units=/home/brian/taxman_work/lab/work/patch4991/units-4991.jsonl
          store=/home/brian/taxman_work/chain-data/carried/nogoods-4984.txt ;;
    4992) units=/home/brian/taxman_work/lab/work/patch4992/units-4992.jsonl
          store=/home/brian/taxman_work/chain-data/carried/nogoods-4991.txt ;;
    4995) units=/home/brian/taxman_work/lab/work/patch4995/units-4995.jsonl
          store=/home/brian/taxman_work/chain-data/carried/nogoods-4992.txt ;;
  esac
  if [ -n "$store" ]; then STORE_ARGS="--tactic carried_nogoods=on --carried $store"; else STORE_ARGS=; fi
  [ "$envs" = "-" ] && envs=
  eval "env ${envs//,/ } nice -n 19 taskset -c $cpu $bin solve $game --known $K --units $units --ids $id $args --node-limit $nl" \
    > "$f.tmp" 2> "$f.err" && mv "$f.tmp" "$f.json"
}
lane() {
  core=$1; shift
  for job in "$@"; do
    id=${job%:*}; rep=${job#*:}
    a=$core; b=$((core + 44))
    if [ $((rep % 2)) = 0 ]; then a=$((core + 44)); b=$core; fi
    run "$a" "${lines[0]}" "$id" "$rep" &
    run "$b" "${lines[1]}" "$id" "$rep" &
    wait
  done
}
mapfile -t units < <(grep -v '^#' "$ids" | awk '{print $1}')
jobs=()
for rep in $(seq 1 "$reps"); do
  for u in "${units[@]}"; do jobs+=("$u:$rep"); done
done
lanes=${LANES:-4}; first=${FIRST:-36}
for l in $(seq 0 $((lanes - 1))); do
  mine=()
  for k in "${!jobs[@]}"; do
    if [ $((k % lanes)) = "$l" ]; then mine+=("${jobs[$k]}"); fi
  done
  lane $((first + l)) "${mine[@]}" &
done
wait
touch "$out/DONE"
