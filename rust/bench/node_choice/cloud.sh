#!/usr/bin/env bash
# The node-choice measurement on one machine: at which search nodes is the strong bound (the Lagrangian bound over
# nogoods, refitted at the node) worth computing besides today's GF(2) bound?  `wall choice-prepare` makes a file for
# each game (its nogood pool, its root multipliers, its sampled nodes), `wall choice` measures every node of them on
# WORKERS processes that share the nodes, and `analyze.py` reads the lines.  README.md here has the question and the
# fields; the nodes come from the tuning suite's units, the probe's fixtures and the proofs of a few games made with
# `perf make-nodes`.
#
#   bench/node_choice/cloud.sh          from rust/, once tools/cloud/setup.sh has built the release examples
#
# Environment (all optional):
#   WORKERS      processes at once, one thread each (default: every CPU)
#   GAMES_SUITE  the tuning suite's games (default "3441 3440 3300 3425": the end of the chain)
#   GAMES_MADE   the games whose proofs make nodes (default "3213 3260 3129 2976 3267 2700"), each a proof of 110,000
#                to 440,000 steps on one thread
#   GAMES_PROBE  the probe's fixtures in testdata/perf (default "936 1215")
#   KEEP         nodes kept of each source of a game, spread over the gaps of those the bound leaves open (default 60)
#   PATHS        nodes a game on the way to its optimum, for the bound's validity (default 3)
#   CPU          CPU seconds the finder and the sound search each get for a game's pool of nogoods (default 10)
#   ROOT_STEPS   subgradient steps of the fit at the root (default 200)
#   NODE_STEPS   subgradient steps of the fit at a node (default 200: the analysis can read fewer, --steps)
#   LIMIT        CPU seconds after which a node's solve is stopped, the node then counted as capped (default 900)
#   SUB          1 (the default) to repeat each fit with the nogoods of the node's own solve too, 0 not to
#   EVERY        the steps of a run of `perf make-nodes` (default 2000, as the entry that made fixtures used)
#   KNOWN        the results file that gives the games' optima (default results/strict.jsonl)
#   SUITE        the tuning suite (default bench/tuning/suite.jsonl)
#   BIN          the directory of the examples (default target/release/examples; a laptop's test is target/debug/examples,
#                built by `cargo build --profile test --example wall --example perf`)
#   OUT          where everything goes (default ~/node_choice): fixtures/, prep/ (a file a game), measure/ (a file of
#                lines a worker), claims/, logs/, faults (what went wrong, if anything), log (the times of the stages)
#                and analysis*.txt
#   S3_DEST      if set, OUT is synced to S3_DEST/out every five minutes and at the end
#   POWEROFF=1   shut the machine down at the end
#
# A run again, with the same OUT, goes on where it was: a fixture or a prepared file that is there is kept, and a node
# that a measure/*.jsonl file has is not measured again (copy S3_DEST/out back to OUT first on a new machine).  The
# script exits with status 1 if `faults` is not empty: a bound below what a node's best completion reaches, or a
# harvested nogood that can be promoted, which is a bug to report; the lines it comes with are in logs/.

set -u
cd "$(dirname "${BASH_SOURCE[0]}")/../.." || exit 1
workers="${WORKERS:-$(getconf _NPROCESSORS_ONLN)}"
out="${OUT:-$HOME/node_choice}"
bin="${BIN:-target/release/examples}"
known="${KNOWN:-results/strict.jsonl}"
suite="${SUITE:-bench/tuning/suite.jsonl}"
games_suite="${GAMES_SUITE-3441 3440 3300 3425}"
games_made="${GAMES_MADE-3213 3260 3129 2976 3267 2700}"
games_probe="${GAMES_PROBE-936 1215}"
sub_flag=""
if [ "${SUB:-1}" = 1 ]; then sub_flag="--sub-nogoods"; fi

if [ ! -x "$bin/wall" ] || [ ! -x "$bin/perf" ]; then
    echo "no $bin/wall or $bin/perf: run tools/cloud/setup.sh first" >&2
    exit 2
fi
mkdir -p "$out/fixtures" "$out/prep" "$out/measure" "$out/logs"
stamp() { echo "$1 $(date -u +%FT%TZ)" >> "$out/log"; }
sync_out() { if [ -n "${S3_DEST:-}" ]; then aws s3 sync --quiet --exclude 'claims/*' "$out" "$S3_DEST/out"; fi; }
fault() { echo "$1" >> "$out/faults"; }
( while sleep 300; do sync_out; done ) &
syncer=$!

# The probe's fixtures are kept in the repository; the others are made from the proofs, a game a process.
stamp "start workers=$workers commit=$(git rev-parse HEAD 2> /dev/null || echo unknown) suite=[$games_suite] made=[$games_made] probe=[$games_probe]"
cp -n testdata/perf/n0936.nodes testdata/perf/n1215.nodes "$out/fixtures/"
fixture() {
    local game="$1" file
    file="$out/fixtures/n$(printf %04d "$game").nodes"
    [ -s "$file" ] && return 0
    "$bin/perf" make-nodes "$game" --known "$known" --every "${EVERY:-2000}" --keep 300 --fixtures "$out/fixtures" \
        > "$out/logs/fixture-$game.out" 2>&1 || fault "fixture $game: status $?"
}
prepare() {
    local game="$1" file
    file="$out/prep/n$(printf %04d "$game").choice"
    [ -s "$file" ] && return 0
    "$bin/wall" choice-prepare --known "$known" --suite "$suite" --fixtures "$out/fixtures" --prep "$out/prep" \
        --cpu "${CPU:-10}" --steps "${ROOT_STEPS:-200}" --keep "${KEEP:-60}" --paths "${PATHS:-3}" "$game" \
        > "$out/logs/prepare-$game.out" 2> "$out/logs/prepare-$game.err" || fault "prepare $game: status $?"
}
export -f fixture prepare fault
export out bin known suite

stamp "fixtures"
# shellcheck disable=SC2086 # lists of games
for game in $games_made; do echo "$game"; done | xargs -P "$workers" -I{} bash -c 'fixture {}'
stamp "prepare"
# shellcheck disable=SC2086
for game in $games_suite $games_made $games_probe; do echo "$game"; done | xargs -P "$workers" -I{} bash -c 'prepare {}'

# The workers take the nodes of a game one by one, the heaviest first, by making a file of the node's name in claims/;
# when a game has none left they go on to the next game.  The suite's games come first: their nodes are the dear ones.
stamp "measure"
rm -rf "$out/claims"
mkdir -p "$out/claims"
worker() {
    local i="$1" game
    # shellcheck disable=SC2086
    for game in $games_suite $games_made $games_probe; do
        [ -s "$out/prep/n$(printf %04d "$game").choice" ] || { fault "measure $game: no prepared file"; continue; }
        # shellcheck disable=SC2086 # an option, or nothing
        "$bin/wall" choice --prep "$out/prep" --results "$out/measure" --worker "$i" --claims "$out/claims" \
            --node-steps "${NODE_STEPS:-200}" --cap "${LIMIT:-900}" $sub_flag "$game" \
            >> "$out/logs/measure-$i.out" 2>> "$out/logs/measure-$i.err" || fault "choice $game worker $i: status $?"
    done
}
pids=()
for ((i = 0; i < workers; i++)); do
    worker "$i" &
    pids+=($!)
done
wait "${pids[@]}"
stamp "measure done"

python3 -I bench/node_choice/analyze.py "$out/measure" > "$out/analysis.txt" 2>&1
python3 -I bench/node_choice/analyze.py "$out/measure" --model floor > "$out/analysis-floor.txt" 2>&1
python3 -I bench/node_choice/analyze.py "$out/measure" --steps 50 > "$out/analysis-steps50.txt" 2>&1
stamp "analysis done"

kill "$syncer" 2> /dev/null
sync_out
status=0
if [ -s "$out/faults" ]; then status=1; fi
if [ "${POWEROFF:-0}" = 1 ]; then sudo shutdown -h now; fi
exit "$status"
