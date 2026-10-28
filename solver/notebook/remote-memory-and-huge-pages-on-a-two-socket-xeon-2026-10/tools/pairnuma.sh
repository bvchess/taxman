#!/bin/bash
# Solves units of 4984 with one build twice at once on the two hyperthreads of one physical core (CORES, siblings 44
# above), one run with its memory bound to the core's node (local) and one to the other node (remote), under the
# chain's tactics (conflict branching on at gate 1200, price reuse, the warm start) with the carried store of 144 units
# of 4983; the hyperthreads swapped on even runs.  One file a solve, OUT/TAG-ID-REP.json (stderr in .err); resumable.
# Usage: pairnuma.sh IDS_FILE OUT REPS BINARY
ids=$1; out=$2; reps=$3; bin=$4
here=$(dirname "$0")
mkdir -p "$out"
store=/home/brian/taxman_work/lab/work/carry/s1/store-4983-144.txt
known=/home/brian/taxman_work/lab/work/step5-2026-10-28/known.jsonl
run() {
  cpu=$1; node=$2; tag=$3; id=$4; rep=$5
  f="$out/$tag-$id-$rep"
  [ -s "$f.json" ] && return
  nice -n 19 taskset -c "$cpu" python3 "$here/membind.py" "$node" "$bin" solve 4984 --known "$known" \
    --units /home/brian/taxman_work/lab/work/patch/units-4984.jsonl --ids "$id" \
    --tactic conflict_branching=on --gate 1200 --tactic price_reuse=on --tactic warm_start=on \
    --tactic carried_nogoods=on --carried "$store" > "$f.tmp" 2> "$f.err" && mv "$f.tmp" "$f.json"
}
lane() {
  core=$1; shift
  near=0; [ "$core" -ge 22 ] && near=1
  far=$((1 - near))
  for job in "$@"; do
    id=${job%:*}; rep=${job#*:}
    a=$core; b=$((core + 44))
    if [ $((rep % 2)) = 0 ]; then a=$((core + 44)); b=$core; fi
    run "$a" "$near" local "$id" "$rep" &
    run "$b" "$far" remote "$id" "$rep" &
    wait
  done
}
mapfile -t units < "$ids"
jobs=()
for rep in $(seq 1 "$reps"); do
  for u in "${units[@]}"; do jobs+=("${u%% *}:$rep"); done
done
read -r -a cores <<< "${CORES:-2 3 24 25}"
lanes=${#cores[@]}
for l in $(seq 0 $((lanes - 1))); do
  mine=()
  for k in "${!jobs[@]}"; do
    if [ $((k % lanes)) = "$l" ]; then mine+=("${jobs[$k]}"); fi
  done
  lane "${cores[$l]}" "${mine[@]}" &
done
wait
