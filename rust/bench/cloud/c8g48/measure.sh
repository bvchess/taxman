#!/usr/bin/env bash
# Scaling to 192 threads (2026-10-15): the measurements of rust/bench/wall/README.md, "For a cloud machine", items 7,
# 8a, 8b, 1, 3 and 4, on a c8g.48xlarge.  Run as ec2-user in tmux after STUDIES=1 tools/cloud/setup.sh (it runs `wall`,
# a study tool that a default build skips).  Every output goes under
# $OUT and is copied to S3 after each item; the copies of the results files (9 MB each) stay in $WORK, and only their
# new lines go to $OUT.  The 64-thread runs of item 8 are bound to 64 CPUs of NUMA node 0 (a stand-in for the
# one-node c8g.16xlarge); item 1 runs beside them on the last CPU, one thread.
set -u
set -o pipefail

ROOT=$HOME/taxman
CRATE=$ROOT/rust
OUT=$HOME/measure
WORK=$HOME/work
S3=s3://taxman-013764204840-us-east-1/measure-2026-10-15/out
TX=$CRATE/target/release/taxman
LADDER=$CRATE/target/release/examples/ladder
WALL=$CRATE/target/release/examples/wall
# shellcheck disable=SC1091
. "$HOME/.cargo/env" 2>/dev/null || true
mkdir -p "$OUT" "$WORK"
cd "$CRATE" || exit 1

stamp() { printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" | tee -a "$OUT/steps.log"; }
save() { nice -n 19 aws s3 sync "$OUT" "$S3" --only-show-errors || stamp "warning: the S3 copy failed"; }
# run NAME CMD...: stdout to NAME.out, stderr to NAME.err, GNU time's report to NAME.time; status and wall in steps.log
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
# the lines of a results file past game N
past() { awk -F'"n":' -v n="$2" '{ if ($2 + 0 > n) print }' "$1"; }
uptime_min() { awk '{ print int($1 / 60) }' /proc/uptime; }

NCPU=$(nproc)
LAST=$((NCPU - 1))
NODE0=$(cat /sys/devices/system/node/node0/cpulist)
C64=$(python3 -c '
s = open("/sys/devices/system/node/node0/cpulist").read().strip()
cpus = []
for part in s.split(","):
    a, _, b = part.partition("-")
    cpus += range(int(a), int(b or a) + 1)
print(",".join(map(str, cpus[:64])))')

stamp "measure.sh starts; uptime $(uptime_min) min; cpus $NCPU; node 0 $NODE0; 64-thread set $C64; item 1 on $LAST"
{
    date -u +%Y-%m-%dT%H:%M:%SZ
    uname -a
    lscpu
    numactl --hardware
    grep MemTotal /proc/meminfo
    token=$(curl -s -X PUT http://169.254.169.254/latest/api/token -H 'X-aws-ec2-metadata-token-ttl-seconds: 300')
    for key in instance-type placement/availability-zone instance-life-cycle ami-id; do
        echo "$key: $(curl -s -H "X-aws-ec2-metadata-token: $token" "http://169.254.169.254/latest/meta-data/$key")"
    done
    rustc --version
    echo "RUSTFLAGS=-C target-cpu=native (setup.sh)"
    rustc --print target-cpus | sed -n '2p'
    echo "commit $(git -C "$ROOT" rev-parse HEAD)"
    echo "64-thread set (node 0): $C64"
    echo "item 1 CPU: $LAST"
} > "$OUT/machine.txt" 2>&1
cp "$HOME/taxman-work/logs/setup.log" "$OUT/setup.log" 2>/dev/null

# The inputs: the known games (the committed chain's first 2175 games, which the file has more than now, and the c8g
# chain), the prefix to 2225, the copy to 2183.
{ head -n 2175 results/strict.jsonl; cat bench/cloud/c8g.chain-2176.jsonl; } > "$WORK/known.jsonl"
{ head -n 2175 results/strict.jsonl; awk -F'"n":' '{ if ($2 + 0 <= 2225) print }' bench/cloud/c8g.chain-2176.jsonl; } \
    > "$WORK/prefix.jsonl"
{ head -n 2175 results/strict.jsonl; awk -F'"n":' '{ if ($2 + 0 <= 2183) print }' bench/cloud/c8g.chain-2176.jsonl; } \
    > "$WORK/c2183.jsonl"
wc -l "$WORK"/*.jsonl | tee -a "$OUT/steps.log"
save

# --- item 7: where a step's time goes at 1, 16, 64, 96 and 192 threads -------------------------------------------
run i7_ladder "$LADDER" --threads 1,16,64,96,192 --seed-opt --known "$WORK/known.jsonl" --seconds 3600 2170 2172
run i7_ladder_rep "$LADDER" --threads 64,192,64,192 --seed-opt --known "$WORK/known.jsonl" --seconds 3600 2170 2172
run i7_ladder_node0 taskset -c "$NODE0" "$LADDER" --threads 64,96 --seed-opt --known "$WORK/known.jsonl" \
    --seconds 3600 2170 2172
run i7_ladder_2184 "$LADDER" --threads 16,64,96,192 --seed-opt --known "$WORK/known.jsonl" --seconds 900 2184
save

# --- item 1 in the background (one thread on the last CPU), item 8 at 64 threads on node 0 -----------------------
stamp "start i1_setup (background, CPU $LAST)"
(
    t0=$(date +%s)
    # shellcheck disable=SC2046
    taskset -c "$LAST" /usr/bin/time -v -o "$OUT/i1_setup.time" "$WALL" setup --known results/strict.jsonl \
        $(seq 2200 25 4095) > "$OUT/setup_c8g.jsonl" 2> "$OUT/i1_setup.err"
    stamp "end i1_setup: status $?, wall $(( $(date +%s) - t0 )) s"
) &
item1=$!

for r in 1 2; do
    for w in 1 4 8 16 32; do
        run "i8a_T64_W${w}_r$r" taskset -c "$C64" "$TX" regress --to 1300 --threads 64 --window "$w"
    done
done
save
for w in 1 16; do
    cp "$WORK/prefix.jsonl" "$WORK/w64_$w.jsonl"
    run "i8b_T64_W$w" taskset -c "$C64" "$TX" solve 2226-2276 --threads 64 --window "$w" --out "$WORK/w64_$w.jsonl" \
        --progress 0
    past "$WORK/w64_$w.jsonl" 2225 > "$OUT/w64_$w.new.jsonl"
done
save
run v_w64_16 taskset -c "$C64" tools/cloud/verify.sh "$WORK/w64_16.jsonl"
save
stamp "waiting for item 1"
wait "$item1"
save

# --- item 8 at 192 threads ----------------------------------------------------------------------------------------
for r in 1 2; do
    for w in 1 4 8 16 32; do
        run "i8a_T192_W${w}_r$r" "$TX" regress --to 1300 --threads 192 --window "$w"
    done
done
save
for w in 1 16; do
    cp "$WORK/prefix.jsonl" "$WORK/w192_$w.jsonl"
    run "i8b_T192_W$w" "$TX" solve 2226-2276 --threads 192 --window "$w" --out "$WORK/w192_$w.jsonl" --progress 0
    past "$WORK/w192_$w.jsonl" 2225 > "$OUT/w192_$w.new.jsonl"
done
cp "$WORK/prefix.jsonl" "$WORK/w192_16t.jsonl"
run i8b_T192_W16_trace env TAXMAN_WINDOW_TRACE=1 "$TX" solve 2226-2276 --threads 192 --window 16 \
    --out "$WORK/w192_16t.jsonl" --progress 0
past "$WORK/w192_16t.jsonl" 2225 > "$OUT/w192_16t.new.jsonl"
awk '$3 == "workers" { split($5, w, "->")
                       count[w[2] == 0 ? "suspensions" : w[2] < w[1] ? "reclaims" : "grants"]++ }
     $3 == "defer" || $3 == "hint" { count[$3 "s"]++ }
     END { for (kind in count) print kind, count[kind] }' "$OUT/i8b_T192_W16_trace.err" > "$OUT/trace_counts.txt"
save
run v_w192_16 tools/cloud/verify.sh "$WORK/w192_16.jsonl"
cat > "$WORK/compare.py" <<'EOF'
import json, sys
load = lambda path: {r["n"]: r for r in map(json.loads, open(path))}
new, old = load(sys.argv[1]), load(sys.argv[2])
key = lambda r: (r["score"], r["proof"], r["same_search"])
games = [n for n in sorted(new) if n > 2225]
bad = [n for n in games if key(new[n]) != key(old[n])]
print(len(games), "games after 2225 in", sys.argv[1], "differ from", sys.argv[2], "at", bad)
EOF
for f in w64_1 w64_16 w192_1 w192_16 w192_16t; do
    python3 "$WORK/compare.py" "$WORK/$f.jsonl" bench/cloud/c8g.chain-2176.jsonl
done > "$OUT/compare.txt" 2>&1
save

# --- item 3: the checkpoint's pause at 192 threads, game 2184 --------------------------------------------------------
for every in 1 300; do
    cp "$WORK/c2183.jsonl" "$WORK/i3_$every.jsonl"
    rm -rf "$WORK/ck_i3_$every"
    run "i3_ck$every" "$TX" solve 2184 --threads 192 --out "$WORK/i3_$every.jsonl" \
        --checkpoint-dir "$WORK/ck_i3_$every" --checkpoint-interval "$every" --progress 10 --time-limit 600
    past "$WORK/i3_$every.jsonl" 2183 > "$OUT/i3_ck$every.new.jsonl"
    ls -l "$WORK/ck_i3_$every" > "$OUT/i3_ck$every.ls.txt" 2>&1
done
save

# --- item 4: memory, the queue and the checkpoint at 4000, standalone, 192 threads --------------------------------
rm -rf "$WORK/ck4000" "$WORK/s4000.jsonl"
mkdir -p "$WORK/ck4000"
( while sleep 60; do date -u +%Y-%m-%dT%H:%M:%SZ; ls -l "$WORK/ck4000"; done ) > "$OUT/i4_ls.txt" 2>&1 &
lister=$!
run i4_4000 "$TX" solve 4000 --threads 192 --out "$WORK/s4000.jsonl" --checkpoint-dir "$WORK/ck4000" \
    --checkpoint-interval 300 --progress 10 --time-limit 600
kill "$lister" 2>/dev/null
{ echo "after the run:"; ls -l "$WORK/ck4000"; } >> "$OUT/i4_ls.txt" 2>&1
cp "$WORK/s4000.jsonl" "$OUT/i4_4000.line.jsonl" 2>/dev/null
save

# --- extras, while the machine's first 150 minutes last: the window's games again at 192 threads, and window 32 ----
extra() {
    local name=$1 threads=$2 w=$3
    if [ "$(uptime_min)" -ge 150 ]; then
        stamp "skip $name: uptime $(uptime_min) min"
        return
    fi
    cp "$WORK/prefix.jsonl" "$WORK/$name.jsonl"
    if [ "$threads" = 64 ]; then
        run "$name" taskset -c "$C64" "$TX" solve 2226-2276 --threads 64 --window "$w" --out "$WORK/$name.jsonl" \
            --progress 0
    else
        run "$name" "$TX" solve 2226-2276 --threads "$threads" --window "$w" --out "$WORK/$name.jsonl" --progress 0
    fi
    past "$WORK/$name.jsonl" 2225 > "$OUT/$name.new.jsonl"
    python3 "$WORK/compare.py" "$WORK/$name.jsonl" bench/cloud/c8g.chain-2176.jsonl >> "$OUT/compare.txt" 2>&1
    save
}
extra x8b_T192_W1_r2 192 1
extra x8b_T192_W16_r2 192 16
extra x8b_T192_W32 192 32
extra x8b_T64_W16_r2 64 16
extra x8b_T64_W1_r2 64 1
extra x8b_T64_W32 64 32

stamp "done; uptime $(uptime_min) min; the machine shuts down (and terminates) in 20 minutes"
save
echo done | aws s3 cp - "$S3/DONE"
sudo shutdown -h +20
