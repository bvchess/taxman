#!/usr/bin/env bash
# Settings and helpers shared by the scripts in this directory.  Source it; do not run it.
#
# Every value can be overridden from the environment before the script starts:
#
#   TAXMAN_ROOT   the checkout (default: the one this file is in)
#   TAXMAN_WORK   where results, checkpoints and logs live (default $HOME/taxman-work).  It must be on a disk that
#                 survives a stop or a reboot: an EBS volume, not tmpfs (/tmp is tmpfs on Amazon Linux 2023) and
#                 not instance-store NVMe.
#   RESULTS       the production results file     (default $TAXMAN_WORK/results/strict.jsonl)
#   CKPT          the checkpoint directory        (default $TAXMAN_WORK/ckpt)
#   LOGS          the log directory               (default $TAXMAN_WORK/logs)
#   TAXMAN_BIN    the solver binary               (default rust/target/release/taxman)
#   LADDER_BIN    the ladder example binary       (default rust/target/release/examples/ladder)
#   EXACT_BIN     the prover's `exact`            (default rust/prover/target/release/exact; prover.sh)
#   RUSTFLAGS     default "-C target-cpu=native" in these scripts (a plain `cargo build` builds generic code): build
#                 for the CPU of the machine that builds.  Build on the machine type that will run the binary.  Set it
#                 to the empty string to build generic code.  The build cache (setup.sh, build_cache.py) keys its
#                 binaries by the CPU this names, and does not take any other flags.

_cloud_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TAXMAN_ROOT="${TAXMAN_ROOT:-$(cd "$_cloud_dir/../../.." && pwd)}"
CRATE="$TAXMAN_ROOT/rust"
TAXMAN_WORK="${TAXMAN_WORK:-$HOME/taxman-work}"
RESULTS="${RESULTS:-$TAXMAN_WORK/results/strict.jsonl}"
CKPT="${CKPT:-$TAXMAN_WORK/ckpt}"
LOGS="${LOGS:-$TAXMAN_WORK/logs}"
TAXMAN_BIN="${TAXMAN_BIN:-$CRATE/target/release/taxman}"
LADDER_BIN="${LADDER_BIN:-$CRATE/target/release/examples/ladder}"
PROVER="$CRATE/prover"
EXACT_BIN="${EXACT_BIN:-$PROVER/target/release/exact}"
CLOUD_DIR="$_cloud_dir"

# The results file committed with the code, which a production run must agree with where they overlap.
COMMITTED_RESULTS=("$CRATE/results/strict.jsonl")

: "${RUSTFLAGS=-C target-cpu=native}"
export RUSTFLAGS

# Cargo and rustc, if rustup put them in the usual place and the shell has not been told.
if ! command -v cargo >/dev/null 2>&1 && [ -r "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi

# A message goes to the terminal (stderr) and, when LOG_FILE is set, to that file, written directly (not through a tee,
# which would die with a Ctrl-C aimed at the whole foreground group and take the log with it).  run_begin (run.sh) sets
# LOG_FILE to $LOGS/NAME.log; spot-watch.sh sets it to $LOGS/spot-watch.log.  Not an environment setting: it starts
# empty here and is not exported, so a script run by another (save.sh) keeps no log of its own.
LOG_FILE=""
log() {
    local line
    line="$(printf '%s %s' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*")"
    [ -n "${LOG_FILE:-}" ] && printf '%s\n' "$line" >>"$LOG_FILE"
    printf '%s\n' "$line" >&2
}

die() {
    log "error: $*"
    exit 1
}

# A time in seconds since the epoch as a UTC date (GNU date takes -d @N, BSD date -r N).
epoch_date() {
    date -u -d "@$1" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -r "$1" +%Y-%m-%dT%H:%M:%SZ
}

# The number of CPUs the OS offers (what `taxman solve` uses when given no --threads).
ncpu() {
    nproc 2>/dev/null || getconf _NPROCESSORS_ONLN 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 1
}

# What a log of a measurement needs to say about the machine and the build.
env_report() {
    echo "date:       $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "host:       $(hostname) $(uname -sr) $(uname -m)"
    echo "cpus:       $(ncpu)"
    if command -v lscpu >/dev/null 2>&1; then
        lscpu | grep -E '^(Architecture|Model name|CPU\(s\)|Thread\(s\) per core|Core\(s\) per socket|Socket\(s\)|NUMA node\(s\))' |
            sed 's/^/            /'
    fi
    if [ -r /proc/meminfo ]; then
        echo "memory:     $(awk '/MemTotal/ {printf "%.0f GiB", $2 / 1048576}' /proc/meminfo)"
    fi
    if command -v rustc >/dev/null 2>&1; then
        echo "rustc:      $(rustc --version)"
        echo "native cpu: $(rustc --print target-cpus 2>/dev/null | sed -n '2p' | sed 's/^ *//')"
    fi
    echo "RUSTFLAGS:  ${RUSTFLAGS:-}"
    if command -v git >/dev/null 2>&1 && git -C "$TAXMAN_ROOT" rev-parse HEAD >/dev/null 2>&1; then
        local dirty=""
        [ -n "$(git -C "$TAXMAN_ROOT" status --porcelain -- rust/src rust/Cargo.toml rust/Cargo.lock rust/build.rs 2>/dev/null)" ] &&
            dirty=" (sources modified)"
        echo "commit:     $(git -C "$TAXMAN_ROOT" rev-parse HEAD)$dirty"
    else
        echo "commit:     unknown (not a git checkout)"
    fi
    echo "work dir:   $TAXMAN_WORK ($(df -h "$TAXMAN_WORK" 2>/dev/null | awk 'NR == 2 {print $4 " free on " $1}'))"
}
