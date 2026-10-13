#!/bin/bash
# The adversarial campaign of README.md, part 4: every family at several sizes, one resumable JSONL file per
# configuration under data/adversary/.  Rerunning resumes; THREADS and RUN_SECONDS (per configuration) can be set.
set -e
cd "$(dirname "$0")/.."
THREADS=${THREADS:-4}
SECONDS_EACH=${RUN_SECONDS:-500}
run() {  # family size eps seeds
  mkdir -p data/adversary
  ./target/release/adversary --family "$1" --size "$2" --eps "$3" --from 0 --to $(($4 - 1)) --threads "$THREADS" \
    --seconds "$SECONDS_EACH" --budget 200000 --out "data/adversary/$1_$2_$3.jsonl" 2>&1 | tail -1
}
run shuffled 3000 0 200
run shuffled 5000 0 200
run shuffled 10000 0 200
run window 3000 0 300
run window 5000 0 300
run window 10000 0 200
run window 20000 0 100
run noisy 10000 0.01 100
run noisy 10000 0.05 100
run noisy 10000 0.2 100
run noisy 10000 1.0 100
run perturbed 10000 0.05 100
run perturbed 10000 0.2 100
run perturbed 10000 0.5 100
run perturbed 20000 0.2 50
run cramer 10000 0 100
run cramer 20000 0 50
sweep() {  # family from to step: one instance per size (deterministic orders)
  mkdir -p data/adversary
  ./target/release/adversary --family "$1" --size 0 --from "$2" --to "$3" --step "$4" --threads "$THREADS" \
    --seconds "$SECONDS_EACH" --budget 200000 --out "data/adversary/$1_sizes_$2_$3_$4.jsonl" 2>&1 | tail -1
}
sweep window-low 2000 20000 100
run window-hyperfirst 5000 0 200
run window-hyperfirst 10000 0 100
run window-hyperlast 5000 0 200
run window-hyperlast 10000 0 100
sweep real 2000 20000 100
run antiwindow 5000 0 200
run antiwindow 10000 0 100
run rankedwindow 5000 0 300
run rankedwindow 10000 0 200
run rankedwindow 10000 0.05 100
run rankedwindow 10000 0.2 100
run rankedwindow 10000 0.5 100
