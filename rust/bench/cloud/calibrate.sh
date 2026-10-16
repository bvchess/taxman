#!/usr/bin/env bash
# The calibration runs of one machine: calibrate.sh PHASE OUTDIR [ARGUMENT...]
#
# It runs from a checkout whose solver (rust/) and prover (rust/prover/) are built in release mode, on Linux or
# macOS, and writes the logs of every run to OUTDIR (timed.sh); parse.py turns a directory of them into one table.
# The phases are the measurements of rust/docs/experiments/graviton-calibration-2026-10.md:
#
#   probe                    the solver's probe (examples/perf.rs), default and GF(2), three times each
#   ladder [N...]            one-thread proof-only runs (ladder --seed-opt), default 1215 1232 1287 2170 2172
#   prover [N...]            one-thread runs of the prover (rust/prover, exact), default 1300 990
#   contention-solver K...   K copies at once of the proof of $CONTENTION_GAME (default 1326), for each K given
#   contention-prover K...   K copies at once of the prover on $CONTENTION_PROVER_GAME (default 990), for each K given
#   scaling T... [-- N]      the proof-only run of game N (default 2172) at each thread count T
#   chain FROM TO MINUTES THREADS
#                            taxman solve FROM-TO on a copy of results/strict.jsonl cut after game FROM-1, for at most
#                            MINUTES minutes.  FROM = 2126, TO = 2175 re-solves games the file already has (the same
#                            scores; its seconds to compare); FROM = 2176 extends the chain
#
# Environment: ROOT (the checkout; default: the one this file is in), CONTENTION_GAME, CONTENTION_PROVER_GAME.

set -eu
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="${ROOT:-$(cd "$here/../../.." && pwd)}"
CRATE="$ROOT/rust"
PERF="$CRATE/target/release/examples/perf"
LADDER="$CRATE/target/release/examples/ladder"
TAXMAN="$CRATE/target/release/taxman"
PROVER="$CRATE/prover/target/release/exact"
KNOWN="$CRATE/results/strict.jsonl"
CONTENTION_GAME="${CONTENTION_GAME:-1326}"
CONTENTION_PROVER_GAME="${CONTENTION_PROVER_GAME:-990}"

phase="$1"
out="$2"
shift 2
mkdir -p "$out"
cd "$CRATE"

say() { printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*"; }

# A one-thread proof of game $1 (the optimum given: the run only proves it), however long it takes.
proof() { echo "$LADDER" --threads "${2:-1}" --seed-opt --seconds 86400 --known "$KNOWN" "$1"; }

# The prover on game $1 with files of its own, named by the label $2 of the timed run (parse.py reads
# $out/prover/$2.jsonl).
prove() { echo "$PROVER" --from "$1" --to "$1" --threads 1 --checkpoints "$out/ckpt/$2" --out "$out/prover/$2.jsonl"; }

case "$phase" in
probe)
    for rep in 1 2 3; do
        say "probe default, rep $rep"
        "$here/timed.sh" "$out" "probe-default.r$rep" "$PERF"
        say "probe gf2, rep $rep"
        "$here/timed.sh" "$out" "probe-gf2.r$rep" "$PERF" --tactic gf2=on
    done
    ;;
ladder)
    games=("$@")
    [ ${#games[@]} -gt 0 ] || games=(1215 1232 1287 2170 2172)
    for n in "${games[@]}"; do
        say "ladder $n, one thread"
        # shellcheck disable=SC2046
        "$here/timed.sh" "$out" "ladder-$n.t1" $(proof "$n")
    done
    ;;
prover)
    games=("$@")
    [ ${#games[@]} -gt 0 ] || games=(1300 990)
    for n in "${games[@]}"; do
        say "prover $n, one thread"
        # shellcheck disable=SC2046
        "$here/timed.sh" "$out" "prover-$n.t1" $(prove "$n" "prover-$n.t1")
    done
    ;;
contention-solver)
    for k in "$@"; do
        say "contention: $k copies of the proof of $CONTENTION_GAME"
        # shellcheck disable=SC2046
        "$here/copies.sh" "$out" "contention-solver-$CONTENTION_GAME" "$k" $(proof "$CONTENTION_GAME")
    done
    ;;
contention-prover)
    for k in "$@"; do
        say "contention: $k copies of the prover on $CONTENTION_PROVER_GAME"
        label="contention-prover-$CONTENTION_PROVER_GAME"
        # shellcheck disable=SC2046
        "$here/copies.sh" "$out" "$label" "$k" $(prove "$CONTENTION_PROVER_GAME" "$label.k$k.c@")
    done
    ;;
scaling)
    game=2172
    threads=()
    while [ $# -gt 0 ]; do
        if [ "$1" = "--" ]; then game="$2"; break; fi
        threads+=("$1")
        shift
    done
    for t in "${threads[@]}"; do
        say "scaling: game $game, $t threads"
        # shellcheck disable=SC2046
        "$here/timed.sh" "$out" "ladder-$game.t$t" $(proof "$game" "$t")
    done
    ;;
chain)
    from="$1"
    to="$2"
    minutes="$3"
    threads="$4"
    results="$out/chain-$from.jsonl"
    # The committed file has game n on line n: keep the games before FROM, and extend the copy.
    head -n "$((from - 1))" "$KNOWN" > "$results"
    tail -n 1 "$results" | grep -q "\"n\":$((from - 1)),"
    echo "$from" > "$out/chain-$from.from"
    say "chain: games $from-$to on $threads threads, at most $minutes minutes," \
        "from a copy of strict.jsonl to $((from - 1))"
    # timeout stops the run with SIGTERM, which `taxman solve` answers by checkpointing and exiting; every finished game
    # is already in the file.
    "$here/timed.sh" "$out" "chain-$from.t$threads" timeout -s TERM "$((minutes * 60))" \
        "$TAXMAN" solve "$from-$to" --threads "$threads" --out "$results" --checkpoint-dir "$out/chain-$from-ckpt" \
        --progress 0
    say "chain: $(( $(wc -l < "$results") - from + 1 )) games added"
    ;;
*)
    echo "unknown phase: $phase" >&2
    exit 2
    ;;
esac
say "done: $phase"
