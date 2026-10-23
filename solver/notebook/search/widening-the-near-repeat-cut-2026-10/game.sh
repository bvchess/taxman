#!/bin/bash
# game.sh N SETTING [CPU]: whole game N proved from its root against its known optimum, one thread, nice 19, CPU
# seconds CPU (default 1500).  SETTING: on (the default search), or rM_K (the study cut of `search::widen` against
# game M, M a list joined by +, in mode K).  The result line goes to out/games/gN-SETTING.json, the cut's counts to
# its .err; an existing result is kept.
R=/home/brian/taxman-lab/work/research-widen
n=$1; s=$2; cpu=${3:-1500}
out=$R/out/games/g$n-$s.json
[ -s "$out" ] && exit 0
mkdir -p $R/out/games
case $s in
    on) ;;
    r*) t=${s#r}; refs=${t%_*}; export TAXMAN_WIDEN_REF=${refs//+/,} TAXMAN_WIDEN_MODE=${t#*_} ;;
esac
nice -n 19 $R/target/release/examples/tune solve $n --ids root --known /home/brian/taxman-work/results/strict.jsonl \
    --cpu-limit $cpu > $out.tmp 2> $out.err && mv $out.tmp $out
