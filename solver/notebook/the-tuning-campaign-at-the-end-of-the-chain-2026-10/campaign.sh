#!/bin/bash
# The tuning campaign on one c8g.16xlarge (as ec2-user, from ~/taxman, after setup.sh and the tune example's build).
#   Stage A: every configuration of stageA.json through the driver's three rungs (64 one-thread jobs at once).
#   Stage B: waits (up to 3 hours) for $D/stageB.json, then runs it through the rungs.
#   Stage C: waits for $D/stageC.json (name -> tactics), then complete 64-thread proofs of six games at the end of the chain for each
#            configuration of it and for the defaults (ladder, seeded with the optimum, an hour's limit each game).
# Results go to ~/tuning and to $D/out as they come; the machine powers off at the end (or when a wait runs out).
set -u
D=s3://taxman-013764204840-us-east-1/tuning-20261016
OUT=~/tuning
R=~/taxman/rust
mkdir -p "$OUT"
exec > >(tee -a "$OUT/campaign.log") 2>&1
sync_out() { aws s3 sync --quiet "$OUT" "$D/out"; }
wait_for() {  # FILE: copy $D/FILE to $OUT/FILE, waiting up to 3 hours
    for _ in $(seq 180); do
        aws s3 cp --quiet "$D/$1" "$OUT/$1" 2>/dev/null && return 0
        sleep 60
    done
    return 1
}
echo "start $(date -u +%FT%TZ) commit $(git -C ~/taxman rev-parse HEAD)"
cd "$R"
aws s3 cp --quiet "$D/stageA.json" "$OUT/stageA.json"

stage() {  # NAME: the driver's three rungs on $OUT/NAME.json
    echo "== stage $1 $(date -u +%T)"
    python3 -I bench/tuning/tune.py --configs "$OUT/$1.json" --results "$OUT/$1.jsonl" --workers 64 --rungs 3 \
        --cutoff 10 > "$OUT/$1.out" 2>&1
    python3 -I bench/tuning/tune.py --configs "$OUT/$1.json" --results "$OUT/$1.jsonl" --report > "$OUT/$1_report.txt" 2>&1
    tail -50 "$OUT/$1_report.txt"
    sync_out
}
stage stageA
echo "waiting for stageB.json $(date -u +%T)"; sync_out
wait_for stageB.json && stage stageB

echo "waiting for stageC.json $(date -u +%T)"; sync_out
if wait_for stageC.json; then
    GAMES="3232 3360 3388 3179 3328 3395"
    names=$(python3 -I -c "import json,sys; print(' '.join(json.load(open(sys.argv[1]))))" "$OUT/stageC.json")
    for name in default $names; do
        args=()
        if [ "$name" != default ]; then
            for t in $(python3 -I -c "import json,sys; print(' '.join(json.load(open(sys.argv[1]))[sys.argv[2]]))" \
                "$OUT/stageC.json" "$name"); do args+=(--tactic "$t"); done
        fi
        echo "== stage C $name ${args[*]:-} $(date -u +%T)"
        target/release/examples/ladder --threads 64 --seconds 3600 --seed-opt --known results/strict.jsonl \
            "${args[@]}" $GAMES > "$OUT/stageC_$name.txt" 2>&1
        grep -E '^n=' "$OUT/stageC_$name.txt" | cut -c1-110
        sync_out
    done
fi
echo "end $(date -u +%FT%TZ)"
sync_out
sudo shutdown -h now
