#!/usr/bin/env bash
# Prepare a machine to build and run the Taxman solver and the prover: packages, the Rust toolchain, the checkout, the
# builds and quick checks.  Idempotent: run it again after a reboot, or to move to newer commits (it fast-forwards the
# checkout).
#
# Written for Amazon Linux 2023 (dnf) and Ubuntu 22.04/24.04 (apt), on arm64 and x86_64.  Other Linux with git, a C
# compiler and curl works with SKIP_PACKAGES=1.
#
#   solver/tools/cloud/setup.sh         from inside a checkout
#   ./setup.sh                          a copy of this script alone: it clones REPO_URL to $TAXMAN_DIR first
#
# The release binaries come from the build cache when it has them (solver/docs/runbook/build-cache.md): a clean checkout
# at a commit looks for that commit's binaries, built for this architecture and CPU, in S3 and downloads them instead of
# building.  Everything in the cache has passed the release suite.  Only RELEASE_TESTS=1, on one machine, puts binaries
# there.
#
# Environment (all optional):
#   REPO_URL        https://github.com/bvchess/taxman.git
#   BRANCH          master
#   TAXMAN_DIR      $HOME/taxman    where to clone, when the script is not inside a checkout
#   NO_PULL=1       leave the checkout as it is (otherwise: git pull --ff-only, if it is clean and on a branch)
#   SKIP_PACKAGES=1 SKIP_TESTS=1 SKIP_SMOKE=1
#   PROVER_ONLY=1 build and check only the prover (what prover.sh runs), not the solver: a few minutes less
#   BUILD           what to build in solver/frame/, as cargo arguments: --bins, --examples, --bin NAME and --example NAME, e.g.
#                   BUILD="--bin taxman --example ledger".  Default "--bins --examples".  A BUILD leaves the prover out
#                   unless BUILD_PROVER asks for it; the checks of a part that is not built are left out.
#   BUILD_PROVER    the same for solver/noframe/ (default "--bins --examples" when BUILD is not set, none when it is; the
#                   word `none` for none)
#   STUDIES=1       also build the study tools of solver/frame/ (tune, extended, structure, finder, wall: cargo's
#                   feature `studies`) and test them; the default build, with the ladder, the probe and the ledger,
#                   skips them.  A build with the studies is another entry of the cache than one without (and serves
#                   a request without them).
#   BUILD_CACHE     where the cache is: s3://taxman-013764204840-us-east-1/builds (default), file:///DIR (for tests) or
#                   off.  build_cache.py has the key and the layout.  A dirty checkout, no commit, RUSTFLAGS other than
#                   a single -C target-cpu, or a cache out of reach (no aws, no credentials) is a miss: the build
#                   goes on.
#   REQUIRE_CACHED=1  a miss is an error: do not build (a launch that must start at once and run only tested binaries)
#   RELEASE_TESTS=1   on a miss, once built (and the smoke checks passed): run the whole release suite of every crate
#                   built, on every CPU (run_tests.py: all the tests of all the test binaries at once, the longest
#                   first), and only if it passes put the binaries in the cache, with the result beside them.  If it
#                   fails nothing is put there: the failing tests go to $LOGS/release-tests.txt and, when S3_DEST is
#                   set, to $S3_DEST/RELEASE_TESTS_FAILED, and the script exits with status 1 (so does the job
#                   that was to start).  Replaces the tests SKIP_TESTS governs; a hit has nothing to run.
#   TEST_JOBS       tests at once in the release suite (default: every CPU); TEST_TIMEOUT a test's seconds (3600)
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
if [ -f "$here/../../frame/Cargo.toml" ] && [ -f "$here/../../../results/optimal.json" ]; then
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
if [ ! -f "$root/solver/frame/Cargo.toml" ]; then
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
. "$root/solver/tools/cloud/env.sh"

mkdir -p "$TAXMAN_WORK/results" "$CKPT" "$LOGS"
exec > >(tee -a "$LOGS/setup.log") 2>&1

# --- what to build -----------------------------------------------------------------------------------------------
# cargo's arguments for solver/frame/ (solver_args) and for solver/noframe/ (prover_args); an empty one builds nothing there.
solver_args="--bins --examples"
[ -z "${BUILD+set}" ] || solver_args="$BUILD"
if [ -n "${BUILD_PROVER+set}" ]; then
    prover_args="$BUILD_PROVER"
elif [ -n "${BUILD+set}" ]; then
    prover_args=""
else
    prover_args="--bins --examples"
fi
if [ "${PROVER_ONLY:-0}" = 1 ]; then
    solver_args=""
    [ -n "${BUILD_PROVER+set}" ] || prover_args="--bins --examples"
fi
[ "$prover_args" != none ] || prover_args=""

# STUDIES=1 adds cargo's feature `studies` to the solver's build, and to its tests (run_tests.py and cargo test take the
# same option): the cache then holds the build apart from one without.
studies=""
if [ "${STUDIES:-0}" = 1 ] && [ -n "$solver_args" ]; then
    studies="--features studies"
    solver_args="$solver_args $studies"
fi

# Whether the cargo arguments $1 build the binary or example (the word $2) named $3.
selects() {
    case " $1 " in
        *" --${2}s "* | *" --$2 $3 "* | *" --$2=$3 "*) return 0 ;;
    esac
    return 1
}

# --- the build cache ---------------------------------------------------------------------------------------------
cache_tmp="$TAXMAN_WORK/build-cache"
mkdir -p "$cache_tmp"
cache() { python3 -I "$here/build_cache.py" "$1" --root "$root" "--solver=$solver_args" "--prover=$prover_args" \
    --tmp "$cache_tmp" "${@:2}"; }

# The key says whether the cache can be used at all (a dirty checkout, no commit, other RUSTFLAGS cannot), and checks
# BUILD and BUILD_PROVER.
cache_usable=0
if [ -z "$solver_args$prover_args" ]; then
    [ "${RELEASE_TESTS:-0}" != 1 ] || fail "RELEASE_TESTS=1 with nothing to build (BUILD and BUILD_PROVER are empty)"
    say "nothing to build (BUILD and BUILD_PROVER are empty)"
else
    cache_status=0
    cache_key="$(cache key)" || cache_status=$?
    [ "$cache_status" != 2 ] || fail "BUILD or BUILD_PROVER is not understood"
    if [ "$cache_status" = 0 ]; then
        cache_usable=1
        say "build cache key: $cache_key"
    fi
fi

built_from_cache=0
if [ "$cache_usable" = 1 ]; then
    if cache fetch; then
        built_from_cache=1
    elif [ "${REQUIRE_CACHED:-0}" = 1 ]; then
        fail "REQUIRE_CACHED=1 and the cache has no tested build of $cache_key for this set: run RELEASE_TESTS=1" \
            "setup.sh on a machine of this type first (solver/docs/runbook/build-cache.md)"
    fi
elif [ "${REQUIRE_CACHED:-0}" = 1 ]; then
    fail "REQUIRE_CACHED=1 and the build cache cannot be used here (see above)"
fi

# --- the build ---------------------------------------------------------------------------------------------------
# Builds a crate: cargo's own default is every CPU for its crates and binaries (a `-j` here would only limit it).
build_crate() {
    local label="$1" directory="$2" arguments="$3"
    say "building the $label (RUSTFLAGS='${RUSTFLAGS:-}'): cargo build --release $arguments"
    # shellcheck disable=SC2086 # the arguments are words
    (cd "$directory" && cargo build --release $arguments) 2>&1 | while IFS= read -r line; do
        printf '[%s] %s\n' "$label" "$line"
    done
}

if [ "$built_from_cache" = 1 ]; then
    say "the binaries come from the build cache: no build"
else
    if [ -n "$solver_args" ] && [ -n "$prover_args" ] && [ "$(ncpu)" -ge 32 ]; then
        # Each build has fewer binaries to link than this machine has CPUs, each link a single thread (fat LTO, one
        # codegen unit): the two builds at once keep the CPUs busy where one after the other leaves most idle.  A
        # smaller machine is filled by one build, and two at once would only need more memory.
        build_crate solver "$CRATE" "$solver_args" &
        solver_build=$!
        build_crate prover "$PROVER" "$prover_args" &
        prover_build=$!
        build_status=0
        wait "$solver_build" || build_status=1
        wait "$prover_build" || build_status=1
        [ "$build_status" = 0 ] || fail "a build failed (see above)"
    else
        [ -z "$solver_args" ] || build_crate solver "$CRATE" "$solver_args"
        [ -z "$prover_args" ] || build_crate prover "$PROVER" "$prover_args"
    fi
fi
if [ -n "$solver_args" ]; then
    # A named binary or example must be there; "--bins" and "--examples" are held to the ones the scripts use.
    ! selects "$solver_args" bin taxman || [ -x "$TAXMAN_BIN" ] || fail "the build did not produce $TAXMAN_BIN"
    ! selects "$solver_args" example ladder || [ -x "$LADDER_BIN" ] || fail "the build did not produce $LADDER_BIN"
fi
if [ -n "$prover_args" ]; then
    ! selects "$prover_args" bin exact || [ -x "$EXACT_BIN" ] || fail "the build did not produce $EXACT_BIN"
fi

# What the cache is given if the release suite passes, packed now, before the tests run (and so before anything they
# build can change a file).
release_stage="$cache_tmp/stage"
release_packed=0
if [ "${RELEASE_TESTS:-0}" = 1 ] && [ "$built_from_cache" != 1 ] && [ "$cache_usable" = 1 ]; then
    cache pack --stage "$release_stage" || fail "could not pack the binaries for the cache"
    release_packed=1
fi

# --- checks ------------------------------------------------------------------------------------------------------
# The smoke checks run on the binaries of the cache too: they show that this machine runs them.  A part that is not
# built has no check.
if [ "${SKIP_SMOKE:-0}" != 1 ]; then
    if [ -n "$solver_args" ] && [ -x "$TAXMAN_BIN" ]; then
        say "smoke: replay optimal.json"
        "$TAXMAN_BIN" verify-optimal
        say "smoke: the committed results files, with the solver's verifier and the independent Python one"
        "$TAXMAN_BIN" verify-results "${COMMITTED_RESULTS[@]}"
        python3 "$TOOLS/check_results.py" "${COMMITTED_RESULTS[@]}"
        say "smoke: games 1..400 against optimal.json"
        "$TAXMAN_BIN" regress --to 400 --threads "$(( $(ncpu) < 8 ? $(ncpu) : 8 ))"
    else
        say "smoke: none for the solver (not in this build)"
    fi
    if [ -n "$prover_args" ] && [ -x "$EXACT_BIN" ]; then
        say "smoke: the prover on games 2..400, and the check of its proofs"
        smoke="$TAXMAN_WORK/smoke-exact"
        rm -rf "$smoke"
        "$EXACT_BIN" --from 2 --to 400 --threads "$(ncpu)" --checkpoints "$smoke/ckpt" --out "$smoke/exact.jsonl"
        python3 "$PROVER/tools/summarize.py" check 2 400 "$smoke/exact.jsonl"
        rm -rf "$smoke"
    else
        say "smoke: none for the prover (not in this build)"
    fi
fi

# --- the release suite -------------------------------------------------------------------------------------------
# The whole suite of every crate that was built, as run_tests.py runs it, on every CPU.  If it passes, the binaries
# packed above go to the cache with its result; if not, the failing tests are written where the job's output is, and
# the setup fails, so that the job that comes after it does not start on a broken build.
release_failed() {
    local report="$LOGS/release-tests.txt"
    [ -s "$report" ] || printf 'the release suite did not finish: see %s/setup.log\n' "$LOGS" >"$report"
    say "RELEASE_TESTS FAILED ($1): the report is $report"
    if [ -n "${S3_DEST:-}" ]; then
        "${AWS_CLI:-aws}" s3 cp "$report" "${S3_DEST%/}/RELEASE_TESTS_FAILED" --only-show-errors &&
            say "the report is in ${S3_DEST%/}/RELEASE_TESTS_FAILED" ||
            say "could not write ${S3_DEST%/}/RELEASE_TESTS_FAILED"
    fi
    exit 1
}

if [ "${RELEASE_TESTS:-0}" = 1 ] && [ "$built_from_cache" = 1 ]; then
    say "release suite: not run, the binaries come from the cache, where only tested ones are"
elif [ "${RELEASE_TESTS:-0}" = 1 ]; then
    release_crates=()
    [ -z "$solver_args" ] || release_crates+=("solver=$CRATE")
    [ -z "$prover_args" ] || release_crates+=("prover=$PROVER")
    release_features=()
    [ -z "$studies" ] || release_features=(--features solver=studies)
    [ "$cache_usable" = 1 ] || say "release suite: the build cache is not used here (see above): nothing will be stored"
    release_summary="$cache_tmp/release-summary.json"
    release_times="$cache_tmp/test-times.json"
    rm -f "$release_summary" "$LOGS/release-tests.txt"
    [ "$cache_usable" != 1 ] || cache times-get --file "$release_times" || say "no test times in the cache yet"
    say "release suite: $(ncpu) CPUs, ${release_crates[*]}"
    release_status=0
    python3 -I "$here/run_tests.py" --release --jobs "${TEST_JOBS:-$(ncpu)}" --timeout "${TEST_TIMEOUT:-3600}" \
        --times "$release_times" --report "$LOGS/release-tests.txt" --summary "$release_summary" \
        ${release_features[@]+"${release_features[@]}"} "${release_crates[@]}" || release_status=$?
    [ "$cache_usable" != 1 ] || [ ! -s "$release_times" ] || cache times-put --file "$release_times" ||
        say "could not store the test times"
    [ "$release_status" = 0 ] || release_failed "status $release_status"
    say "release suite: passed"
    if [ "$release_packed" = 1 ]; then
        cache put --stage "$release_stage" --summary "$release_summary" ||
            fail "the suite passed but the cache did not take the binaries"
        rm -rf "$release_stage"
    fi
elif [ "${SKIP_TESTS:-0}" != 1 ]; then
    # A handful of test threads: many tests start a thread per CPU themselves, and a test thread per CPU on top of that
    # only oversubscribes a large machine.  The ignored large-machine tests are run by hand (RUNBOOK.md).
    if [ -n "$solver_args" ]; then
        say "cargo test${studies:+ $studies} -- --test-threads=4"
        # shellcheck disable=SC2086 # an option, or nothing
        (cd "$CRATE" && cargo test $studies -- --test-threads=4)
    fi
    if [ -n "$prover_args" ]; then
        say "cargo test --release -- --test-threads=4, in the prover (about a minute)"
        (cd "$PROVER" && cargo test --release -- --test-threads=4)
    fi
fi

# --- report ------------------------------------------------------------------------------------------------------
say "ready"
env_report
echo
echo "work directory: $TAXMAN_WORK (results, ckpt, logs)"
echo "solver:         $TAXMAN_BIN"
if [ -x "$TOOLS/pgo.sh" ]; then
    echo "note: $TOOLS/pgo.sh exists in this checkout; read its header for an optional profile-guided build"
fi
echo "prover:         $EXACT_BIN"
echo "next:           $CLOUD_DIR/scaling.sh (the solver) or $CLOUD_DIR/prover.sh A B (the prover's proofs);" \
    "see RUNBOOK.md"
