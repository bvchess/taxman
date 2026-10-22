#!/bin/bash
# unit.sh SNAP ID SETTING [CPU]: solve unit ID of snapshot SNAP (a folder of ~/taxman-lab/snap) on one thread at nice 19,
# proved against the known optimum, CPU seconds CPU (default 150).  SETTING: on, off, gGAP (gate), pSTEPS_DEPTH
# (probe), xK (conflict branching off at nodes that have left out at least K members of the snapshot's incumbent), cM
# (the study cut from the proven game M, TAXMAN_CARRY_PREV).  The result line goes to out/units/ID-SETTING.json, the
# progress to .err beside it; an existing result is kept.
R=/home/brian/taxman-lab/work/research-carry
snap=/home/brian/taxman-lab/snap/$1; id=$2; s=$3; cpu=${4:-150}
game=${id%%-*}
out=$R/out/units/$id-$s.json
[ -s "$out" ] && exit 0
mkdir -p $R/out/units $R/out/ref
args=()
case $s in
    on) ;;
    c*) export TAXMAN_CARRY_PREV=${s#c} ;;
    off) args=(--tactic conflict_branching=off) ;;
    g*) export TAXMAN_CB_GAP=${s#g} ;;
    p*) t=${s#p}; export TAXMAN_PROBE=${t%_*},${t#*_} ;;
    x*)
        ref=$R/out/ref/$1.hex
        [ -s $ref ] || head -1 $snap/units.jsonl | python3 -c "import json,sys; print(json.load(sys.stdin)['incumbent'])" > $ref
        export TAXMAN_CARRY_EXCL=${s#x} TAXMAN_CARRY_REF=$ref ;;
esac
nice -n 19 $R/target/release/examples/tune solve $game --units $snap/units.jsonl --ids $id \
    --known /home/brian/taxman-work/results/strict.jsonl --cpu-limit $cpu "${args[@]}" > $out.tmp 2> $out.err \
    && mv $out.tmp $out
