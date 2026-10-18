#!/bin/bash
# Stage C: complete 64-thread proofs (ladder, seeded with the optimum, an hour's limit a game) of seven games at the end of the chain,
# for today's defaults and the stage B winners.  Replaces campaign.sh's own stage C (it would power off at the end of
# its wait); as ec2-user.  Syncs after each configuration and powers the machine off at the end.
set -u
D=s3://taxman-013764204840-us-east-1/tuning-20261016
OUT=~/tuning
R=~/taxman/rust
tmux kill-session -t tune 2>/dev/null
exec > >(tee -a "$OUT/stageC.log") 2>&1
cd "$R"
GAMES="3232 3360 3388 3179 3328 3395 3300"
run() {  # NAME TACTIC...
    local name=$1; shift
    local args=()
    for t in "$@"; do args+=(--tactic "$t"); done
    echo "== $name $* $(date -u +%T)"
    target/release/examples/ladder --threads 64 --seconds 3600 --seed-opt --known results/strict.jsonl \
        "${args[@]}" $GAMES > "$OUT/stageC_$name.txt" 2>&1
    grep -E '^n=' "$OUT/stageC_$name.txt" | cut -c1-120
    aws s3 sync --quiet "$OUT" "$D/out"
}
run default
run fw_all force_window=all
run all_sl4 force_window=all strong_limit=4
run all_sl4_mfo force_window=all strong_limit=4 more_forced=off
echo "end $(date -u +%FT%TZ)"
aws s3 sync --quiet "$OUT" "$D/out"
sudo shutdown -h now
