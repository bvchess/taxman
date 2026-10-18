#!/usr/bin/env bash
# Prepare a machine to build and run the Taxman solver and the prover: packages, the Rust toolchain, the checkout, the
# builds and quick checks.  Idempotent: run it again after a reboot, or to move to newer commits (it fast-forwards the
# checkout).
#
# Written for Amazon Linux 2023 (dnf) and Ubuntu 22.04/24.04 (apt), on arm64 and x86_64.  Other Linux with git, a C
# compiler and curl works with SKIP_PACKAGES=1.
#
#   rust/tools/cloud/setup.sh           from inside a checkout
#   ./setup.sh                          a copy of this script alone: it clones REPO_URL to $TAXMAN_DIR first
#
# Environment (all optional):
#   REPO_URL        https://github.com/bvchess/taxman.git
#   BRANCH          master
#   TAXMAN_DIR      $HOME/taxman    where to clone, when the script is not inside a checkout
#   NO_PULL=1       leave the checkout as it is (otherwise: git pull --ff-only, if it is clean and on a branch)
#   SKIP_PACKAGES=1 SKIP_TESTS=1 SKIP_SMOKE=1
#   PROVER_ONLY=1 build and check only the prover (what prover.sh runs), not the solver: a few minutes less
#   STUDIES=1     also build the study tools (tune, extended, structure, wall: cargo's feature `studies`); the default
#                 build, with the ladder, the probe and the ledger, skips them
#   TAXMAN_WORK     where results, checkpoints and logs live (see env.sh)
#   RUSTFLAGS       default "-C target-cpu=native" (see env.sh)

set -eu
set -o pipefail

REPO_URL="${REPO_URL:-https://github.com/bvchess/taxman.git}"
BRANCH="${BRANCH:-master}"
MIN_RUST="1.89"   # File::try_lock, used by the engine, is stable from 1.89

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
say() { printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$*" >&2; }
fail() { say "error: $*"; exit 1; }

# --- where the checkout is ---------------------------------------------------------------------------------------
if [ -f "$here/../../Cargo.toml" ] && [ -f "$here/../../../src/main/resources/optimal.json" ]; then
    root="$(cd "$here/../../.." && pwd)"
else
    root="${TAXMAN_DIR:-$HOME/taxman}"
fi

# --- packages ----------------------------------------------------------------------------------------------------
sudo_cmd=""
if [ "$(id -u)" -ne 0 ]; then
    command -v sudo >/dev/null 2>&1 && sudo_cmd="sudo"
fi

# Optional tools (GNU time reports the peak memory of a run; htop shows the CPUs): one at a time, so that a package
# a distribution does not have cannot fail the rest.
optional_packages() {
    local manager="$1" package
    for package in time htop; do
        $sudo_cmd env DEBIAN_FRONTEND=noninteractive "$manager" install -y "$package" ||
            say "note: could not install $package (optional)"
    done
}

install_packages() {
    if [ "$(id -u)" -ne 0 ] && [ -z "$sudo_cmd" ]; then
        fail "installing packages needs root or sudo (or SKIP_PACKAGES=1 if git, cc, python3 and curl are there)"
    fi
    if command -v dnf >/dev/null 2>&1; then
        # Amazon Linux 2023 ships curl-minimal, which conflicts with the curl package: add a curl only if there is none.
        $sudo_cmd dnf install -y git gcc python3 procps-ng tmux util-linux
        command -v curl >/dev/null 2>&1 || $sudo_cmd dnf install -y curl-minimal
        optional_packages dnf
    elif command -v yum >/dev/null 2>&1; then
        $sudo_cmd yum install -y git gcc python3 procps-ng tmux util-linux
        command -v curl >/dev/null 2>&1 || $sudo_cmd yum install -y curl
        optional_packages yum
    elif command -v apt-get >/dev/null 2>&1; then
        $sudo_cmd apt-get update -y
        $sudo_cmd env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
            git build-essential python3 procps tmux util-linux curl ca-certificates
        optional_packages apt-get
    else
        fail "no dnf, yum or apt-get: install git, a C compiler, python3 and curl yourself and set SKIP_PACKAGES=1"
    fi
}

if [ "${SKIP_PACKAGES:-0}" = 1 ]; then
    say "packages: skipped"
else
    say "packages"
    install_packages
fi
for tool in git cc python3 curl; do
    command -v "$tool" >/dev/null 2>&1 || fail "$tool is not installed"
done

# --- the Rust toolchain ------------------------------------------------------------------------------------------
if ! command -v cargo >/dev/null 2>&1 && [ -r "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
if ! command -v cargo >/dev/null 2>&1; then
    say "installing the Rust toolchain with rustup (stable, minimal profile)"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
fi
rust_version="$(rustc --version | awk '{print $2}')"
if [ "$(printf '%s\n%s\n' "$MIN_RUST" "$rust_version" | sort -V | head -n 1)" != "$MIN_RUST" ]; then
    if command -v rustup >/dev/null 2>&1; then
        say "rustc $rust_version is older than $MIN_RUST: updating stable"
        rustup toolchain install stable --profile minimal
        rustup default stable
        rust_version="$(rustc --version | awk '{print $2}')"
    else
        fail "rustc $rust_version is older than $MIN_RUST and there is no rustup to update it"
    fi
fi
say "rustc $rust_version"

# --- the checkout ------------------------------------------------------------------------------------------------
if [ ! -f "$root/rust/Cargo.toml" ]; then
    [ -e "$root" ] && fail "$root exists but is not a Taxman checkout"
    say "cloning $REPO_URL ($BRANCH) to $root"
    git clone --branch "$BRANCH" "$REPO_URL" "$root"
elif [ "${NO_PULL:-0}" = 1 ]; then
    say "checkout $root: left as it is (NO_PULL=1)"
elif ! git -C "$root" rev-parse HEAD >/dev/null 2>&1; then
    say "checkout $root: not a git repository, not updating"
elif [ -n "$(git -C "$root" status --porcelain --untracked-files=no)" ]; then
    say "checkout $root: uncommitted changes, not updating"
elif ! git -C "$root" symbolic-ref -q HEAD >/dev/null 2>&1 || ! git -C "$root" rev-parse '@{u}' >/dev/null 2>&1; then
    say "checkout $root: detached HEAD or no upstream branch, not updating"
else
    say "checkout $root: git pull --ff-only"
    git -C "$root" pull --ff-only
fi

export TAXMAN_ROOT="$root"
# shellcheck source=env.sh
. "$root/rust/tools/cloud/env.sh"

mkdir -p "$TAXMAN_WORK/results" "$CKPT" "$LOGS"
exec > >(tee -a "$LOGS/setup.log") 2>&1

# --- the build ---------------------------------------------------------------------------------------------------
if [ "${PROVER_ONLY:-0}" != 1 ]; then
    say "building (RUSTFLAGS='${RUSTFLAGS:-}'): the solver and the examples (the ladder among them)"
    studies=""
    if [ "${STUDIES:-0}" = 1 ]; then studies="--features studies"; fi
    # shellcheck disable=SC2086 # an option, or nothing
    (cd "$CRATE" && cargo build --release --bins --examples $studies)
    [ -x "$TAXMAN_BIN" ] || fail "the build did not produce $TAXMAN_BIN"
    [ -x "$LADDER_BIN" ] || fail "the build did not produce $LADDER_BIN"
fi
say "building the prover (its own workspace): exact, the other tools and their examples"
(cd "$PROVER" && cargo build --release --bins --examples)
[ -x "$EXACT_BIN" ] || fail "the build did not produce $EXACT_BIN"

# --- checks ------------------------------------------------------------------------------------------------------
if [ "${SKIP_SMOKE:-0}" != 1 ]; then
    if [ "${PROVER_ONLY:-0}" != 1 ]; then
        say "smoke: replay optimal.json"
        "$TAXMAN_BIN" verify-optimal
        say "smoke: the committed results files, with the solver's verifier and the independent Python one"
        "$TAXMAN_BIN" verify-results "${COMMITTED_RESULTS[@]}"
        python3 "$CRATE/tools/check_results.py" "${COMMITTED_RESULTS[@]}"
        say "smoke: games 1..400 against optimal.json"
        "$TAXMAN_BIN" regress --to 400 --threads "$(( $(ncpu) < 8 ? $(ncpu) : 8 ))"
    fi
    say "smoke: the prover on games 2..400, and the check of its proofs"
    smoke="$TAXMAN_WORK/smoke-exact"
    rm -rf "$smoke"
    "$EXACT_BIN" --from 2 --to 400 --threads "$(ncpu)" --checkpoints "$smoke/ckpt" --out "$smoke/exact.jsonl"
    python3 "$PROVER/tools/summarize.py" check 2 400 "$smoke/exact.jsonl"
    rm -rf "$smoke"
fi
if [ "${SKIP_TESTS:-0}" != 1 ]; then
    # A handful of test threads (DESIGN.md times the suite with two): many tests start a thread per CPU themselves,
    # and a test thread per CPU on top of that only oversubscribes a large machine.  The ignored large-machine tests
    # are run by hand (RUNBOOK.md).
    if [ "${PROVER_ONLY:-0}" != 1 ]; then
        say "cargo test -- --test-threads=4"
        (cd "$CRATE" && cargo test -- --test-threads=4)
    fi
    say "cargo test --release -- --test-threads=4, in the prover (about a minute)"
    (cd "$PROVER" && cargo test --release -- --test-threads=4)
fi

# --- report ------------------------------------------------------------------------------------------------------
say "ready"
env_report
echo
echo "work directory: $TAXMAN_WORK (results, ckpt, logs)"
echo "solver:         $TAXMAN_BIN"
if [ -x "$CRATE/tools/pgo.sh" ]; then
    echo "note: $CRATE/tools/pgo.sh exists in this checkout; read its header for an optional profile-guided build"
fi
echo "prover:         $EXACT_BIN"
echo "next:           $CLOUD_DIR/scaling.sh (the solver) or $CLOUD_DIR/prover.sh A B (the prover's proofs);" \
    "see RUNBOOK.md"
