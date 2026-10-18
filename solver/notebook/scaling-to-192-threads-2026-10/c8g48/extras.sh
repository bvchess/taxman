#!/usr/bin/env bash
# Extras after measure.sh: wait for its end, then (1) the first ladder again, to see whether its slow 192-thread rows
# repeat (the repeat in a fresh process had a step 1.7 times cheaper), and (2) the window at 4 on 2226..2276 at 192 and
# 64 threads.  Same layout and S3 copies as measure.sh.  It leaves the shutdown that measure.sh scheduled (20 minutes
# after its end) as it is: these take about 6 minutes.
set -u
set -o pipefail
ROOT=$HOME/taxman
CRATE=$ROOT/rust
OUT=$HOME/measure
WORK=$HOME/work
S3=s3://taxman-013764204840-us-east-1/measure-2026-10-15/out
TX=$CRATE/target/release/taxman
LADDER=$CRATE/target/release/examples/ladder
cd "$CRATE" || exit 1

stamp() { printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" | tee -a "$OUT/steps.log"; }
save() { nice -n 19 aws s3 sync "$OUT" "$S3" --only-show-errors || stamp "warning: the S3 copy failed"; }
run() {
    local name=$1 t0 t1 st
    shift
    stamp "start $name: $*"
    t0=$(date +%s.%N)
    /usr/bin/time -v -o "$OUT/$name.time" "$@" > "$OUT/$name.out" 2> "$OUT/$name.err"
    st=$?
    t1=$(date +%s.%N)
    stamp "end $name: status $st, wall $(awk -v a="$t0" -v b="$t1" 'BEGIN { printf "%.2f", b - a }') s"
}
past() { awk -F'"n":' -v n="$2" '{ if ($2 + 0 > n) print }' "$1"; }
C64=$(python3 -c '
s = open("/sys/devices/system/node/node0/cpulist").read().strip()
cpus = []
for part in s.split(","):
    a, _, b = part.partition("-")
    cpus += range(int(a), int(b or a) + 1)
print(",".join(map(str, cpus[:64])))')

# measure.sh's last command schedules the shutdown; its tmux session ends with it
while tmux has-session -t m 2>/dev/null; do sleep 10; done
stamp "extras.sh starts (measure.sh's shutdown stays scheduled)"

run e_ladder_again "$LADDER" --threads 1,16,64,96,192 --seed-opt --known "$WORK/known.jsonl" --seconds 3600 2170 2172
save
for spec in "192 4" "64 4"; do
    set -- $spec
    name="e8b_T$1_W$2"
    cp "$WORK/prefix.jsonl" "$WORK/$name.jsonl"
    if [ "$1" = 64 ]; then
        run "$name" taskset -c "$C64" "$TX" solve 2226-2276 --threads 64 --window "$2" --out "$WORK/$name.jsonl" \
            --progress 0
    else
        run "$name" "$TX" solve 2226-2276 --threads "$1" --window "$2" --out "$WORK/$name.jsonl" --progress 0
    fi
    past "$WORK/$name.jsonl" 2225 > "$OUT/$name.new.jsonl"
    python3 "$WORK/compare.py" "$WORK/$name.jsonl" bench/cloud/c8g.chain-2176.jsonl >> "$OUT/compare.txt" 2>&1
    save
done
stamp "extras done"
save
echo done | aws s3 cp - "$S3/EXTRAS-DONE"
