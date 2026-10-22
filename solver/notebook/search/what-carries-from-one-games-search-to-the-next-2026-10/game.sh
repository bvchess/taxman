#!/bin/bash
# game.sh N SETTING [CPU]: whole game N proved from its root against its known optimum, one thread, nice 19, CPU
# seconds CPU (default 1800).  SETTING: on, off, gGAP (gate), xK (conflict branching off at nodes that have left out
# at least K members of the optimum, TAXMAN_CARRY_EXCL with the reference out/ref/N.hex), cM (the study cut from the
# proven game M, TAXMAN_CARRY_PREV).  The result line goes to out/games/gN-SETTING.json, the progress to .err beside
# it; an existing result is kept.
R=/home/brian/taxman-lab/work/research-carry
n=$1; s=$2; cpu=${3:-1800}
out=$R/out/games/g$n-$s.json
[ -s "$out" ] && exit 0
mkdir -p $R/out/games $R/out/ref
args=()
case $s in
    on) ;;
    c*) export TAXMAN_CARRY_PREV=${s#c} ;;
    off) args=(--tactic conflict_branching=off) ;;
    g*) export TAXMAN_CB_GAP=${s#g} ;;
    x*)
        [ -s $R/out/ref/$n.hex ] || $R/target/release/examples/carry optimum $n $n \
            --known /home/brian/taxman-work/results/strict.jsonl > $R/out/ref/$n.hex
        export TAXMAN_CARRY_EXCL=${s#x} TAXMAN_CARRY_REF=$R/out/ref/$n.hex ;;
esac
nice -n 19 $R/target/release/examples/tune solve $n --ids root --known /home/brian/taxman-work/results/strict.jsonl \
    --cpu-limit $cpu "${args[@]}" > $out.tmp 2> $out.err && mv $out.tmp $out
