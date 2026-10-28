#!/bin/bash
# Solves units of 4984 twice at once on the two hyperthreads of one physical core (CORES, siblings 44 above), with the
# binaries BIN_A and BIN_B and the environments ENV_A and ENV_B (e.g. GLIBC_TUNABLES=..., or X=1 for none), under the
# chain's tactics (conflict branching on at gate 1200, price reuse, the warm start) with the carried store of 144 units
# of 4983; the hyperthreads swapped on even runs.  One file a solve, OUT/TAG-ID-REP.json (stderr in .err); resumable.
# Usage: pairenv.sh IDS_FILE OUT REPS TAG_A BIN_A ENV_A TAG_B BIN_B ENV_B
ids=$1; out=$2; reps=$3; ta=$4; ba=$5; ea=$6; tb=$7; bb=$8; eb=$9
mkdir -p "$out"
store=/home/brian/taxman_work/lab/work/carry/s1/store-4983-144.txt
known=/home/brian/taxman_work/lab/work/step5-2026-10-28/known.jsonl
run() {
  cpu=$1; tag=$2; bin=$3; envs=$4; id=$5; rep=$6
  f="$out/$tag-$id-$rep"
  [ -s "$f.json" ] && return
  env $envs nice -n 19 taskset -c "$cpu" "$bin" solve 4984 --known "$known" \
    --units /home/brian/taxman_work/lab/work/patch/units-4984.jsonl --ids "$id" \
    --tactic conflict_branching=on --gate 1200 --tactic price_reuse=on --tactic warm_start=on \
    --tactic carried_nogoods=on --carried "$store" > "$f.tmp" 2> "$f.err" && mv "$f.tmp" "$f.json"
}
lane() {
  core=$1; shift
  for job in "$@"; do
    id=${job%:*}; rep=${job#*:}
    a=$core; b=$((core + 44))
    if [ $((rep % 2)) = 0 ]; then a=$((core + 44)); b=$core; fi
    run "$a" "$ta" "$ba" "$ea" "$id" "$rep" &
    run "$b" "$tb" "$bb" "$eb" "$id" "$rep" &
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
