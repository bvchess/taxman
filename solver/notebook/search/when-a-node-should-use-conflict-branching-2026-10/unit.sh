#!/bin/bash
# unit.sh BIN LABEL ID [tune args...]: one unit of a snapshot (SNAP, default the 4895 one), one thread, nice 19,
# env passed through; result line to runs/LABEL/ID.jsonl, stderr to runs/LABEL/ID.err
B=/home/brian/taxman-lab/work/research-cb/bin/$1
S=${SNAP:-/home/brian/taxman-lab/snap/4895-20261022T123603Z}
label=$1; label=$2; id=$3; shift 3
game=${id%%-*}
O=/home/brian/taxman-lab/work/research-cb/runs/$label
mkdir -p $O
nice -n 19 $B solve $game --units $S/units.jsonl --ids $id "$@" > $O/$id.jsonl 2> $O/$id.err
