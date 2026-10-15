#!/usr/bin/env bash
# Second extras, after extras.sh: the chain's next games, 2277 on, behind the committed chain and the c8g chain's lines
# to 2276, at 192 threads and at 64 threads on node 0, window 1, a few minutes each: the games an hour of each machine
# on the stretch the next run will solve.  It does not touch the shutdown measure.sh scheduled: each run gets half of
# what is left before it, less two minutes for the copies, at most five minutes; with under two minutes each it skips.
set -u
set -o pipefail
ROOT=$HOME/taxman
CRATE=$ROOT/rust
OUT=$HOME/measure
WORK=$HOME/work
S3=s3://taxman-013764204840-us-east-1/measure-2026-10-15/out
TX=$CRATE/target/release/taxman
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

while tmux has-session -t m 2>/dev/null || tmux has-session -t x 2>/dev/null; do sleep 10; done
usec=$(sudo cat /run/systemd/shutdown/scheduled 2>/dev/null | sed -n 's/^USEC=//p')
if [ -z "$usec" ]; then
    stamp "extras2: no scheduled shutdown found; skipped"
    exit 0
fi
left=$(( usec / 1000000 - $(date +%s) ))
each=$(( (left - 120) / 2 ))
[ "$each" -gt 300 ] && each=300
stamp "extras2.sh starts; $left s before the scheduled shutdown; $each s a run"
if [ "$each" -lt 120 ]; then
    stamp "extras2: not enough time; skipped"
    exit 0
fi
{ cat results/strict.jsonl; awk -F'"n":' '{ if ($2 + 0 <= 2276) print }' bench/cloud/c8g.chain-2176.jsonl; } \
    > "$WORK/p2276.jsonl"
for t in 192 64; do
    cp "$WORK/p2276.jsonl" "$WORK/n$t.jsonl"
    if [ "$t" = 64 ]; then
        run "n2277_T$t" timeout -s INT "$each" taskset -c "$C64" "$TX" solve 2277-2400 --threads 64 --window 1 \
            --out "$WORK/n$t.jsonl" --progress 30
    else
        run "n2277_T$t" timeout -s INT "$each" "$TX" solve 2277-2400 --threads 192 --window 1 \
            --out "$WORK/n$t.jsonl" --progress 30
    fi
    past "$WORK/n$t.jsonl" 2276 > "$OUT/n2277_T$t.new.jsonl"
    save
done
stamp "extras2 done"
save
echo done | aws s3 cp - "$S3/EXTRAS2-DONE"
