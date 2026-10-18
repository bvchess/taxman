#!/bin/bash
# Calibration for the run to 4000, on one c8g.16xlarge (run as ec2-user from ~/taxman, after setup.sh).
#   1. game 3441 resumed from the chain's checkpoint for 180 s of search at 1, 16, 32 and 64 threads, under perf stat -d
#   2. ladder proofs (seeded with the optimum) of 3232 3360 3388 3179 at 64 threads: default, forget_nogoods=off;
#      and the default at 32 threads (the tree's growth with threads)
# Everything goes to ~/calib and to S3 ($D), then the machine powers off.
set -u
D=s3://taxman-013764204840-us-east-1/calib-20261016
OUT=~/calib
BIN=~/taxman/rust/target/release/taxman
LADDER=~/taxman/rust/target/release/examples/ladder
STRICT=~/taxman/rust/results/strict.jsonl
mkdir -p "$OUT"
exec > >(tee -a "$OUT/calib.log") 2>&1
echo "start $(date -u +%FT%TZ) commit $(git -C ~/taxman rev-parse HEAD)"
nproc; lscpu | grep -E 'Model name|L2|L3|NUMA'
aws s3 cp "$D/n3441.ckpt" "$OUT/n3441.ckpt"
sync_out() { aws s3 sync --quiet "$OUT" "$D/out" --exclude '*/ckpt/*' --exclude 'n3441.ckpt'; }

for T in 1 16 32 64; do
    W="$OUT/t$T"; rm -rf "$W"; mkdir -p "$W/ckpt"
    cp "$OUT/n3441.ckpt" "$W/ckpt/"; cp "$STRICT" "$W/out.jsonl"
    echo "== 3441 resumed on $T threads $(date -u +%T)"
    perf stat -d -o "$W/perf.txt" -- /usr/bin/time -v -o "$W/time.txt" "$BIN" solve 3441 --threads "$T" \
        --out "$W/out.jsonl" --checkpoint-dir "$W/ckpt" --checkpoint-interval 300 --progress 10 --time-limit 180 \
        > "$W/stdout.txt" 2> "$W/progress.txt"
    tail -2 "$W/progress.txt"; sync_out
done

ladder() {  # TAG THREADS [TACTIC...]
    local tag=$1 t=$2; shift 2
    echo "== ladder $tag ($t threads) $(date -u +%T)"
    /usr/bin/time -v -o "$OUT/ladder_$tag.time" "$LADDER" --threads "$t" --seed-opt --known "$STRICT" "$@" \
        3232 3360 3388 3179 > "$OUT/ladder_$tag.txt" 2>&1
    tail -8 "$OUT/ladder_$tag.txt"; sync_out
}
ladder default64 64
ladder noforget64 64 --tactic forget_nogoods=off
ladder default32 32

echo "end $(date -u +%FT%TZ)"
sync_out
sudo shutdown -h now
