#!/bin/bash
# game.sh BIN LABEL N CPU [tune args...]: whole game N proved from its root against the known optimum, one thread,
# nice 19, env passed through; result line to runs/LABEL/gN.jsonl, stderr to runs/LABEL/gN.err
B=/home/brian/taxman-lab/work/research-cb/bin/$1
K=/home/brian/taxman-work/results/strict.jsonl
label=$2; n=$3; cpu=$4; shift 4
O=/home/brian/taxman-lab/work/research-cb/runs/$label
mkdir -p $O
nice -n 19 $B solve $n --ids root --known $K --cpu-limit $cpu "$@" > $O/g$n.jsonl 2> $O/g$n.err
