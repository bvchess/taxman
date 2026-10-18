#!/usr/bin/env python3
"""Tests of setup.sh's build logic, with a stand-in for cargo (it records how it was called and makes the files a build
would) and a directory for the build cache: what is built for BUILD and BUILD_PROVER, the cache's hit and miss, the
release suite and what it does on a pass and on a failure, the dirty checkout, REQUIRE_CACHED.  No packages are
installed, nothing is built and nothing leaves the machine.

usage: python3 -I test_setup.py      (a few seconds each; runs setup.sh with the bash on the PATH, or $TEST_BASH:
       TEST_BASH=/bin/bash is macOS's 3.2)
"""
import json
import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
CLOUD = HERE.parent
FAKE_AWS = HERE / "bin"
BASH = os.environ.get("TEST_BASH", "bash")  # the bash that runs setup.sh

CARGO = r"""#!/usr/bin/env bash
# a stand-in for cargo: the commands setup.sh and run_tests.py give it
echo "cargo $(basename "$PWD") $*" >>"$STUB_LOG"
[ "$(basename "$PWD")" != "${STUB_FAIL_IN:-}" ] || exit 1
case "$(basename "$PWD")" in noframe) crate=prover ;; *) crate=solver ;; esac
tool() { mkdir -p "$(dirname "$1")"; printf '#!/bin/sh\nexit 0\n' >"$1"; chmod +x "$1"; }
release=target/release
case "$1" in
    build)
        shift
        echo "   Compiling the stand-in of the $crate"
        all_examples=0
        studies=0
        while [ $# -gt 0 ]; do
            case "$1" in
                --features) shift; [ "$1" != studies ] || studies=1 ;;
                --bins)
                    if [ "$crate" = solver ]; then
                        tool $release/taxman
                    else
                        tool $release/exact
                        tool $release/moves
                    fi
                    ;;
                --examples)
                    all_examples=1
                    if [ "$crate" = solver ]; then
                        tool $release/examples/ladder
                        tool $release/examples/ledger
                    else
                        tool $release/examples/inspect
                    fi
                    ;;
                --bin) shift; tool $release/$1 ;;
                --example) shift; tool $release/examples/$1 ;;
            esac
            shift
        done
        # the feature adds the study examples to all the examples
        [ "$crate$all_examples$studies" != solver11 ] || tool $release/examples/wall
        ;;
    test)
        case " $* " in
            *" --no-run "*)
                printf '{"reason":"compiler-artifact","manifest_path":"%s/Cargo.toml","target":{"kind":["lib"],' "$PWD"
                printf '"name":"stub"},"profile":{"test":true},"executable":"%s","fresh":true}\n' "$STUB_TESTBIN"
                ;;
            *" --doc "*) printf 'running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured\n' ;;
        esac
        ;;
esac
"""

TESTBIN = r"""#!/usr/bin/env bash
# a stand-in for a test executable with three tests, one of them ignored
case "$*" in
    "--list") printf 'alpha: test\nbeta: test\nignored_one: test\n\n3 tests, 0 benchmarks\n' ;;
    "--list --ignored") printf 'ignored_one: test\n\n1 test, 0 benchmarks\n' ;;
    --exact*)
        if [ "${FAIL_TEST:-}" = "$2" ]; then printf -- '---- %s stdout ----\nthe stand-in fails\n' "$2"; exit 101; fi
        printf 'running 1 test\ntest %s ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured\n' "$2"
        ;;
esac
"""


def git(root: Path, *arguments: str) -> str:
    environment = dict(os.environ, GIT_AUTHOR_NAME="t", GIT_AUTHOR_EMAIL="t@t", GIT_COMMITTER_NAME="t",
                       GIT_COMMITTER_EMAIL="t@t")
    return subprocess.run(["git", "-C", str(root), *arguments], capture_output=True, text=True, check=True,
                          env=environment).stdout


def executable(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    path.chmod(0o755)


class Setup(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.base = Path(self.tmp.name)
        self.source = self.base / "source"
        cloud = self.source / "solver" / "tools" / "cloud"
        cloud.mkdir(parents=True)
        for name in ("setup.sh", "env.sh", "build_cache.py", "run_tests.py"):
            shutil.copy(CLOUD / name, cloud / name)
        (self.source / "solver" / "frame").mkdir()
        (self.source / "solver" / "frame" / "Cargo.toml").write_text("[package]\n")
        (self.source / "solver" / "noframe").mkdir()
        (self.source / "solver" / "noframe" / "Cargo.toml").write_text("[package]\n")
        (self.source / "results").mkdir()
        (self.source / "results" / "optimal.json").write_text("{}\n")
        (self.source / "results" / "strict.jsonl").write_text("")
        executable(self.source / "solver" / "tools" / "check_results.py", "")
        executable(self.source / "solver" / "noframe" / "tools" / "summarize.py", "")
        (self.source / ".gitignore").write_text("target\n")
        subprocess.run(["git", "init", "-q", str(self.source)], check=True)
        git(self.source, "add", ".")
        git(self.source, "commit", "-q", "-m", "one")
        self.commit = git(self.source, "rev-parse", "HEAD").strip()

        self.bin = self.base / "bin"
        executable(self.bin / "cargo", CARGO)
        executable(self.bin / "testbin", TESTBIN)
        self.cache = self.base / "cache"
        self.log = self.base / "cargo.log"
        self.log.write_text("")
        self.machines = 0

    def tearDown(self):
        self.tmp.cleanup()

    def machine(self, dirty=False) -> Path:
        """A fresh machine: a clone at the commit, nothing built."""
        self.machines += 1
        clone = self.base / f"machine{self.machines}" / "taxman"
        clone.parent.mkdir()
        subprocess.run(["git", "clone", "-q", str(self.source), str(clone)], check=True)
        if dirty:
            (clone / "solver" / "frame" / "stray.rs").write_text("not committed\n")
        return clone

    def setup(self, machine: Path, **variables):
        """Runs the machine's setup.sh; returns (status, the text of its log, the work directory)."""
        work = machine.parent / "work"
        environment = dict(os.environ, PATH=f"{self.bin}{os.pathsep}{FAKE_AWS}{os.pathsep}{os.environ['PATH']}",
                           HOME=str(machine.parent), TAXMAN_WORK=str(work), SKIP_PACKAGES="1", NO_PULL="1",
                           SKIP_SMOKE="1", SKIP_TESTS="1", RUSTFLAGS="-C target-cpu=cortex-a76",
                           BUILD_CACHE=f"file://{self.cache}", STUB_LOG=str(self.log),
                           STUB_TESTBIN=str(self.bin / "testbin"),
                           FAKE_S3=str(self.base / "s3"), FAKE_STATE=str(self.base / "state"),
                           FAKE_LOG=str(self.base / "aws.log"))
        for name in ("BUILD", "BUILD_PROVER", "STUDIES", "RELEASE_TESTS", "REQUIRE_CACHED", "S3_DEST", "PROVER_ONLY",
                     "FAIL_TEST"):
            environment.pop(name, None)
        environment.update({name: str(value) for name, value in variables.items()})
        done = subprocess.run([BASH, str(machine / "solver" / "tools" / "cloud" / "setup.sh")], capture_output=True,
                              text=True, env=environment)
        logged = done.stdout + done.stderr
        return done.returncode, logged, work

    def cargo_calls(self) -> list:
        lines = self.log.read_text().splitlines()
        self.log.write_text("")
        return lines

    def entries(self) -> list:
        return sorted(p.parent.name for p in self.cache.glob("*/*/manifest"))

    # --- what is built ---
    def test_with_no_variable_everything_is_built_and_nothing_is_stored(self):
        machine = self.machine()
        status, logged, _ = self.setup(machine)
        self.assertEqual(status, 0, logged)
        self.assertEqual(self.cargo_calls(), ["cargo frame build --release --bins --examples",
                                              "cargo noframe build --release --bins --examples"])
        self.assertTrue((machine / "solver/frame/target/release/examples/ladder").exists())
        self.assertTrue((machine / "solver/noframe/target/release/exact").exists())
        self.assertEqual(self.entries(), [], "an untested build never goes to the cache")

    def test_build_names_what_is_built_and_leaves_the_prover_out(self):
        machine = self.machine()
        status, logged, _ = self.setup(machine, BUILD="--bin taxman --example ledger", SKIP_SMOKE="")
        self.assertEqual(status, 0, logged)
        self.assertEqual(self.cargo_calls(), ["cargo frame build --release --bin taxman --example ledger"])
        self.assertTrue((machine / "solver/frame/target/release/examples/ledger").exists())
        self.assertFalse((machine / "solver/frame/target/release/examples/ladder").exists())
        self.assertFalse((machine / "solver/noframe/target").exists())
        self.assertIn("smoke: replay optimal.json", logged)  # the solver has its checks, the prover none
        self.assertIn("smoke: none for the prover", logged)

    def test_build_prover_asks_for_the_prover_and_none_leaves_it_out(self):
        status, logged, _ = self.setup(self.machine(), BUILD="--bin taxman", BUILD_PROVER="--bin exact")
        self.assertEqual(status, 0, logged)
        self.assertEqual(self.cargo_calls(), ["cargo frame build --release --bin taxman",
                                              "cargo noframe build --release --bin exact"])
        status, logged, _ = self.setup(self.machine(), BUILD_PROVER="none")
        self.assertEqual(self.cargo_calls(), ["cargo frame build --release --bins --examples"])

    def test_a_large_machine_builds_both_crates_at_once_and_a_failure_in_either_stops_the_setup(self):
        executable(self.bin / "nproc", "#!/bin/sh\necho 64\n")
        status, logged, _ = self.setup(self.machine())
        self.assertEqual(status, 0, logged)
        self.assertEqual(sorted(self.cargo_calls()), ["cargo frame build --release --bins --examples",
                                                      "cargo noframe build --release --bins --examples"])
        self.assertIn("[solver] ", logged)  # each build's lines say whose they are
        self.assertIn("building the prover", logged)
        self.cargo_calls()
        for crate in ("noframe", "frame"):
            status, logged, _ = self.setup(self.machine(), STUB_FAIL_IN=crate)
            self.assertEqual(status, 1, logged)
            self.assertIn("a build failed", logged)

    def test_prover_only_builds_only_the_prover(self):
        machine = self.machine()
        status, logged, _ = self.setup(machine, PROVER_ONLY="1", SKIP_SMOKE="")
        self.assertEqual(status, 0, logged)
        self.assertEqual(self.cargo_calls(), ["cargo noframe build --release --bins --examples"])
        self.assertIn("smoke: none for the solver", logged)

    def test_a_build_that_is_not_understood_stops_the_setup(self):
        for text in ("--features", "--bin"):
            status, logged, _ = self.setup(self.machine(), BUILD=text)
            self.assertEqual(status, 1, logged)
            self.assertIn("not understood", logged)
        self.assertEqual(self.cargo_calls(), [])

    def test_the_old_tests_run_for_the_parts_that_are_built(self):
        status, logged, _ = self.setup(self.machine(), BUILD="--bin taxman", SKIP_TESTS="0")
        self.assertEqual(status, 0, logged)
        self.assertEqual(self.cargo_calls(), ["cargo frame build --release --bin taxman",
                                              "cargo frame test -- --test-threads=4"])

    def test_studies_adds_the_feature_to_the_solvers_build_and_tests_and_to_nothing_else(self):
        machine = self.machine()
        status, logged, _ = self.setup(machine, STUDIES="1", BUILD_PROVER="none", SKIP_TESTS="0")
        self.assertEqual(status, 0, logged)
        self.assertEqual(self.cargo_calls(), ["cargo frame build --release --bins --examples --features studies",
                                              "cargo frame test --features studies -- --test-threads=4"])
        self.assertTrue((machine / "solver/frame/target/release/examples/wall").exists())
        status, logged, _ = self.setup(self.machine(), STUDIES="1", BUILD="--bin taxman --example wall")
        self.assertEqual(status, 0, logged)
        self.assertEqual(self.cargo_calls(),
                         ["cargo frame build --release --bin taxman --example wall --features studies"])
        status, logged, _ = self.setup(self.machine(), STUDIES="1", PROVER_ONLY="1")
        self.assertEqual(status, 0, logged)
        self.assertEqual(self.cargo_calls(), ["cargo noframe build --release --bins --examples"])
        plain = self.machine()
        self.assertEqual(self.setup(plain)[0], 0)
        self.assertFalse((plain / "solver/frame/target/release/examples/wall").exists())

    # --- the release suite and the cache ---
    def test_a_release_run_tests_stores_and_a_second_machine_takes_it(self):
        first = self.machine()
        status, logged, work = self.setup(first, RELEASE_TESTS="1")
        self.assertEqual(status, 0, logged)
        calls = self.cargo_calls()
        self.assertIn("cargo frame build --release --bins --examples", calls)
        self.assertIn("cargo frame test --release --no-run --message-format=json", calls)
        self.assertIn("cargo noframe test --release --no-run --message-format=json", calls)
        self.assertIn("release suite: passed", logged)
        self.assertEqual(self.entries(), ["prover-bins+prover-examples+solver-bins+solver-examples"])
        (manifest,) = self.cache.glob("*/*/manifest")
        text = manifest.read_text()
        self.assertIn("tests=passed\n", text)
        self.assertIn("tests_solver=2 passed, 0 failed, 1 ignored\n", text)  # alpha and beta; ignored_one is not run
        self.assertTrue((self.cache / "test-times.json").is_file(), "the durations are kept for the next ordering")

        second = self.machine()
        status, logged, _ = self.setup(second, BUILD="--bin taxman", RELEASE_TESTS="1", SKIP_SMOKE="")
        self.assertEqual(status, 0, logged)
        self.assertEqual(self.cargo_calls(), [], "a hit builds nothing and runs no suite")
        self.assertIn("the binaries come from the build cache", logged)
        self.assertIn("release suite: not run", logged)
        self.assertTrue(os.access(second / "solver/frame/target/release/taxman", os.X_OK))
        self.assertIn("smoke: replay optimal.json", logged)  # the smoke checks still show that the machine runs them

    def test_a_build_with_the_studies_is_cached_apart_from_one_without(self):
        default_entry = "prover-bins+prover-examples+solver-bins+solver-examples"
        studies_entry = default_entry + "+solver-features-studies"
        self.assertEqual(self.setup(self.machine(), RELEASE_TESTS="1")[0], 0)
        self.assertEqual(self.entries(), [default_entry])
        self.cargo_calls()
        # the default build holds no study tool: a launch that needs `wall` finds nothing and builds nothing
        status, logged, _ = self.setup(self.machine(), REQUIRE_CACHED="1", STUDIES="1", BUILD="--example wall")
        self.assertEqual(status, 1, logged)
        self.assertEqual(self.cargo_calls(), [])

        status, logged, _ = self.setup(self.machine(), RELEASE_TESTS="1", STUDIES="1")
        self.assertEqual(status, 0, logged)
        calls = self.cargo_calls()
        self.assertIn("cargo frame build --release --bins --examples --features studies", calls)
        self.assertIn("cargo frame test --release --features studies --no-run --message-format=json", calls)
        self.assertIn("cargo noframe test --release --no-run --message-format=json", calls)  # the prover has none
        self.assertEqual(self.entries(), [default_entry, studies_entry])

        with_wall = self.machine()
        status, logged, _ = self.setup(with_wall, REQUIRE_CACHED="1", STUDIES="1", BUILD="--example wall")
        self.assertEqual(status, 0, logged)
        self.assertTrue(os.access(with_wall / "solver/frame/target/release/examples/wall", os.X_OK))
        without = self.machine()  # a request without the studies takes the entry without them
        self.assertEqual(self.setup(without, REQUIRE_CACHED="1")[0], 0)
        self.assertFalse((without / "solver/frame/target/release/examples/wall").exists())
        self.assertEqual(self.cargo_calls(), [])

    def test_a_failing_suite_stores_nothing_and_says_which_test_failed(self):
        machine = self.machine()
        status, logged, work = self.setup(machine, RELEASE_TESTS="1", FAIL_TEST="beta", S3_DEST="s3://bucket/job")
        self.assertEqual(status, 1, logged)
        self.assertIn("RELEASE_TESTS FAILED", logged)
        self.assertEqual(self.entries(), [])
        report = (work / "logs" / "release-tests.txt").read_text()
        self.assertIn("solver/lib:stub/beta", report)
        self.assertIn("the stand-in fails", report)
        uploaded = self.base / "s3" / "bucket" / "job" / "RELEASE_TESTS_FAILED"
        self.assertEqual(uploaded.read_text(), report)

    def test_a_dirty_checkout_does_not_use_the_cache_but_the_suite_runs(self):
        status, logged, _ = self.setup(self.machine(dirty=True), RELEASE_TESTS="1")
        self.assertEqual(status, 0, logged)
        self.assertIn("uncommitted changes", logged)
        self.assertIn("release suite: passed", logged)
        self.assertIn("nothing will be stored", logged)
        self.assertFalse(self.cache.exists())

    def test_a_dirty_checkout_does_not_read_the_cache(self):
        self.assertEqual(self.setup(self.machine(), RELEASE_TESTS="1")[0], 0)
        self.cargo_calls()
        status, logged, _ = self.setup(self.machine(dirty=True))
        self.assertEqual(status, 0, logged)
        self.assertIn("cargo frame build --release --bins --examples", self.cargo_calls())

    def test_require_cached_does_not_build_on_a_miss(self):
        status, logged, _ = self.setup(self.machine(), REQUIRE_CACHED="1")
        self.assertEqual(status, 1, logged)
        self.assertIn("REQUIRE_CACHED=1", logged)
        self.assertEqual(self.cargo_calls(), [])
        status, logged, _ = self.setup(self.machine(dirty=True), REQUIRE_CACHED="1")
        self.assertEqual(status, 1, logged)
        self.assertEqual(self.cargo_calls(), [])

    def test_require_cached_takes_a_hit(self):
        self.assertEqual(self.setup(self.machine(), RELEASE_TESTS="1")[0], 0)
        self.cargo_calls()
        status, logged, _ = self.setup(self.machine(), REQUIRE_CACHED="1", BUILD="--example ledger")
        self.assertEqual(status, 0, logged)
        self.assertEqual(self.cargo_calls(), [])

    def test_another_cpu_is_another_key(self):
        self.assertEqual(self.setup(self.machine(), RELEASE_TESTS="1")[0], 0)
        self.cargo_calls()
        status, logged, _ = self.setup(self.machine(), RUSTFLAGS="-C target-cpu=neoverse-v2")
        self.assertEqual(status, 0, logged)
        self.assertIn("cargo frame build --release --bins --examples", self.cargo_calls())

    def test_the_cache_off_builds_and_runs_the_suite_without_storing(self):
        status, logged, _ = self.setup(self.machine(), RELEASE_TESTS="1", BUILD_CACHE="off")
        self.assertEqual(status, 0, logged)
        self.assertIn("release suite: passed", logged)
        self.assertFalse(self.cache.exists())


if __name__ == "__main__":
    unittest.main()
