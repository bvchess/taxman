#!/bin/bash
#
# Profile-guided build of the solver: an instrumented build runs a training workload, and the profile it records
# steers the optimized build: the same answers and trees, and 0-2% fewer cycles than the plain release build on
# the frontier's searches since the hot loop is inlined by hand (12% before; DESIGN.md, "Performance").
#
#   tools/pgo.sh [--no-train]
#
# Builds target/pgo/release/taxman and its examples (perf, ladder); target/release stays the plain build, which
# is the one to profile (with PGO the search is inlined into a few functions and a sampler sees little).
# --no-train reuses the profile of the last run (target/pgo/merged.profdata).  Takes a few minutes.
#
# The training workload: the perf probe's fixtures (936 and 1215, every hot kernel and a search window), and a
# short search on one thread of a game of every set width the solver dispatches to (each width is a separate
# copy of the search's code, and code a profile never saw is optimized as cold).
#
# Needs an llvm-profdata of the same LLVM major version as rustc's (`rustc -vV`): $LLVM_PROFDATA if set, else
# rustup's llvm-tools component, else Homebrew's llvm, else the PATH.
#
# Linux (e.g. AWS Graviton): `rustup component add llvm-tools` provides a matching llvm-profdata.  aarch64 Linux
# targets generic armv8-a by default, unlike macOS (apple-m1, where target-cpu gains nothing), so build for the
# machine with extra flags, for instance
#
#   RUSTFLAGS="-C target-cpu=native" tools/pgo.sh
#
# (Graviton4 is a Neoverse V2; check /proc/cpuinfo.)  The probe measures cycles only on macOS; on Linux run it
# under `perf stat` to compare builds.

set -euo pipefail

TRAIN=1
if [ "${1:-}" = "--no-train" ]; then
    TRAIN=0
elif [ $# -gt 0 ]; then
    echo "usage: tools/pgo.sh [--no-train]" >&2
    exit 2
fi

CRATE="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." >/dev/null 2>&1 && pwd )"
OUT="$CRATE/target/pgo"
EXTRA="${RUSTFLAGS:-}"  # kept, and added to the profile flags
TARGETS=(--bin taxman --example perf --example ladder)

# An llvm-profdata whose LLVM major version is rustc's.
llvm_major() { sed -n 's/.*LLVM version:\{0,1\} \([0-9][0-9]*\).*/\1/p' | head -1; }
WANT="$(rustc -vV | llvm_major)"
SYSROOT_TOOL="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin/llvm-profdata"
PROFDATA=""
for candidate in "${LLVM_PROFDATA:-}" "$SYSROOT_TOOL" /opt/homebrew/opt/llvm/bin/llvm-profdata \
    /usr/local/opt/llvm/bin/llvm-profdata "$(command -v llvm-profdata || true)"; do
    if [ -n "$candidate" ] && [ -x "$candidate" ] && [ "$("$candidate" --version | llvm_major)" = "$WANT" ]; then
        PROFDATA="$candidate"
        break
    fi
done
if [ -z "$PROFDATA" ]; then
    echo "no llvm-profdata of LLVM $WANT (rustc's): set LLVM_PROFDATA, or rustup component add llvm-tools" >&2
    exit 1
fi
echo "llvm-profdata: $PROFDATA (LLVM $WANT)"

if [ "$TRAIN" = 1 ]; then
    rm -rf "$OUT/raw"
    mkdir -p "$OUT/raw"
    echo "== instrumented build"
    RUSTFLAGS="-Cprofile-generate=$OUT/raw $EXTRA" cargo build --release --manifest-path "$CRATE/Cargo.toml" \
        --target-dir "$OUT/instrumented" "${TARGETS[@]}"
    BIN="$OUT/instrumented/release"
    echo "== training"
    "$BIN/examples/perf" --reps 1
    # A game of every width, from W = 8 to W = 64; the frontier's ones searched longer.
    for game in 480:2 700:2 936:4 1184:8 1300:8 1600:3 1900:3 2500:2 3500:2; do
        "$BIN/examples/ladder" --threads 1 --seconds "${game#*:}" --known "$CRATE/results/strict.jsonl" \
            --tactic revise_freedoms=off "${game%:*}" | cut -c1-100
    done
    "$PROFDATA" merge -o "$OUT/merged.profdata" "$OUT/raw"
fi

echo "== optimized build"
RUSTFLAGS="-Cprofile-use=$OUT/merged.profdata $EXTRA" \
    cargo build --release --manifest-path "$CRATE/Cargo.toml" --target-dir "$OUT" "${TARGETS[@]}"
echo "built $OUT/release/taxman and $OUT/release/examples/{perf,ladder}"
