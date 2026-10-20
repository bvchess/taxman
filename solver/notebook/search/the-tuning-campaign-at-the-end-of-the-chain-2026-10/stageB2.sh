#!/bin/bash
# Stage B again without the settings stage A showed to be far worse (cheaper branching at depth): as ec2-user.
set -u
D=s3://taxman-013764204840-us-east-1/tuning-20261016
OUT=~/tuning
cd ~/taxman/rust
aws s3 cp --quiet "$D/stageB2.json" "$OUT/stageB2.json"
python3 -I bench/tuning/tune.py --configs "$OUT/stageB2.json" --results "$OUT/stageB2.jsonl" --workers 64 --rungs 3 \
    --cutoff 10 > "$OUT/stageB2.out" 2>&1
python3 -I bench/tuning/tune.py --configs "$OUT/stageB2.json" --results "$OUT/stageB2.jsonl" --report \
    > "$OUT/stageB2_report.txt" 2>&1
aws s3 sync --quiet "$OUT" "$D/out"
