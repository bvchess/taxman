#!/usr/bin/env bash
# The scaling sweep: how the proof of a hard game speeds up with the number of threads, on this machine.
#
#   solver/tools/cloud/scaling.sh               the sweep (the ladder, --scaling: 1, 2, 4, ... up to all CPUs)
#   solver/tools/cloud/scaling.sh --engine      also the engine's own benchmark (overhead per node, no work per node)
#   solver/tools/cloud/scaling.sh --pauses      also one real game with a checkpoint every 0.5 s (cost of a pause)
#   solver/tools/cloud/scaling.sh --quick       one small game only (seconds: a check that everything runs)
#
# Everything is logged to $LOGS/scaling-<time>.txt with a description of the machine at the top.  RUNBOOK.md,
# "Reading the sweep", says how to read the tables.  Run it on an idle machine.
#
# Environment (all optional):
#   GAMES          games to sweep, default "1215 1225" (proofs that take about 1 and 2.5 minutes on one M1 CPU)
#   THREADS        a comma list such as 1,8,64,192, default: 1, 2, 4, ... and the number of CPUs
#   SECONDS_LIMIT  per game and thread count, default 1800; a search that hits it is marked with * in the tables
#   KNOWN          the results file the ladder takes games from, default results/strict.jsonl
#   ENGINE_THREADS, ENGINE_ITEMS, ENGINE_DEPTH   the engine benchmark's sizes (see tests/engine.rs)
#   PAUSE_GAME     the game of --pauses, default 1230; PAUSE_THREADS its threads, default all CPUs

set -eu
set -o pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=env.sh
. "$here/env.sh"

run_engine=0
run_pauses=0
quick=0
for arg in "$@"; do
    case "$arg" in
        --engine) run_engine=1 ;;
        --pauses) run_pauses=1 ;;
        --quick) quick=1 ;;
        -h | --help) sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) die "unknown option $arg (see --help)" ;;
    esac
done

[ -x "$LADDER_BIN" ] || die "$LADDER_BIN is missing: run setup.sh (or: cd solver/frame && cargo build --release --examples)"
[ -x "$TAXMAN_BIN" ] || die "$TAXMAN_BIN is missing: run setup.sh"

GAMES="${GAMES:-1215 1225}"
[ "$quick" = 1 ] && GAMES="${GAMES_QUICK:-1184}"
SECONDS_LIMIT="${SECONDS_LIMIT:-1800}"
KNOWN="${KNOWN:-$TAXMAN_ROOT/results/strict.jsonl}"
[ -r "$KNOWN" ] || die "$KNOWN is not readable"

mkdir -p "$LOGS"
out="$LOGS/scaling-$(date -u +%Y%m%dT%H%M%SZ).txt"
exec > >(tee -a "$out") 2>&1

echo "== machine and build"
env_report
echo "load:       $(uptime | sed 's/.*load average/load average/')"
echo
echo "== the ladder: the proof of each game, seeded with its optimum (--seed-opt)"
thread_args=(--scaling)
[ -n "${THREADS:-}" ] && thread_args=(--threads "$THREADS")
# GNU time reports the peak memory of the whole sweep; BSD time and a missing one just run the command.
timer=()
if [ -x /usr/bin/time ] && /usr/bin/time -v true >/dev/null 2>&1; then
    timer=(/usr/bin/time -v)
fi
# $GAMES is a list of words on purpose.
# shellcheck disable=SC2086
${timer[@]+"${timer[@]}"} "$LADDER_BIN" "${thread_args[@]}" --seed-opt --known "$KNOWN" \
    --seconds "$SECONDS_LIMIT" $GAMES

if [ "$run_engine" = 1 ]; then
    echo
    echo "== the engine's own benchmark (no work per node): per-node overhead against the threads"
    # Thread counts 1, 2, 4, ... and all CPUs, as the ladder's --scaling does.
    ncpus="$(ncpu)"
    counts="1"
    t=1
    while [ "$t" -lt "$ncpus" ]; do
        t=$((t * 2))
        [ "$t" -gt "$ncpus" ] && t="$ncpus"
        counts="$counts,$t"
    done
    (
        cd "$CRATE"
        TXENGINE_BENCH_THREADS="${ENGINE_THREADS:-$counts}" \
            TXENGINE_BENCH_ITEMS="${ENGINE_ITEMS:-80}" \
            TXENGINE_BENCH_DEPTH="${ENGINE_DEPTH:-35}" \
            cargo test --release --test engine -- --ignored --nocapture bench
    )
fi

if [ "$run_pauses" = 1 ]; then
    game="${PAUSE_GAME:-1230}"
    echo
    pause_threads=()
    [ -n "${PAUSE_THREADS:-}" ] && pause_threads=(--threads "$PAUSE_THREADS")
    echo "== the cost of checkpoint pauses (threads: ${PAUSE_THREADS:-all available}): game $game, a checkpoint every 0.5 s"
    # On the work disk (EBS), not /tmp (tmpfs on Amazon Linux): the cost of a checkpoint is largely its fsync.
    scratch="$(mktemp -d "$TAXMAN_WORK/pauses.XXXXXX")"
    # The committed chain up to the game before, so that the game starts from its predecessor as in a chain.
    python3 - "$KNOWN" "$scratch/results.jsonl" "$game" <<'PY'
import json, sys
source, target, game = sys.argv[1], sys.argv[2], int(sys.argv[3])
with open(source) as src, open(target, "w") as dst:
    for line in src:
        if line.strip() and json.loads(line)["n"] < game:
            dst.write(line)
PY
    "$TAXMAN_BIN" solve "$game" ${pause_threads[@]+"${pause_threads[@]}"} --out "$scratch/results.jsonl" \
        --checkpoint-dir "$scratch/ckpt" --checkpoint-interval 0.5 --progress 5
    rm -r "${scratch:?}"
fi

echo
echo "log: $out"
