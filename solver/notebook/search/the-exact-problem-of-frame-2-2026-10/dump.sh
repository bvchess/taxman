#!/bin/zsh
# The nodes of the entry, one search at a time and at a low priority: `zsh dump.sh OUT` from solver/frame, after
# `cargo build --profile test --example wall --features studies`.  Appends the instance lines to OUT.
renice -n 10 -p $$ >/dev/null
OUT=$1
W=target/debug/examples/wall
SUITE=../notebook/search/the-hardest-searches-of-the-run-to-5000-2026-10/suite-hard.jsonl
KNOWN=../../results/strict.jsonl
for spec in "3940 3940-104" "4225 4225-1.15" "4416 4416-95"; do
  set -- ${=spec}
  $W local --known $KNOWN --suite $SUITE --ids $2 --per-unit 12 --checked none --out $OUT $1
done
for n in 4050 4176; do
  $W local --known $KNOWN --root --nodes 400000 --per-unit 10 --checked none --out $OUT $n
done
# 4548 has no known optimum: only its root is written, and nothing is searched.
$W local --known $KNOWN --root --out $OUT 4548
