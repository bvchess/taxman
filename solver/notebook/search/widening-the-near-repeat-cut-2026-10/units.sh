#!/bin/bash
# units.sh M N SNAP: the check of every mode at each unit of the snapshot SNAP (a folder of ~/taxman-lab/snap) of
# game N against game M, one thread at nice 19, to out/units-N-from-M.jsonl; then the greedy completions to
# out/greedy-N-from-M.jsonl.
R=/home/brian/taxman-lab/work/research-widen
m=$1; n=$2; snap=/home/brian/taxman-lab/snap/$3
known=/home/brian/taxman-work/results/strict.jsonl
mkdir -p $R/out
nice -n 19 $R/target/release/examples/widen units $m $n --units $snap/units.jsonl --known $known \
    > $R/out/units-$n-from-$m.jsonl 2> $R/out/units-$n-from-$m.err
nice -n 19 $R/target/release/examples/widen greedy $m $n --units $snap/units.jsonl --known $known \
    > $R/out/greedy-$n-from-$m.jsonl 2> $R/out/greedy-$n-from-$m.err
