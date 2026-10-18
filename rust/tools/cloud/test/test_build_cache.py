#!/usr/bin/env python3
"""Tests of build_cache.py with a directory standing in for the bucket (file://), and with the stand-in for the aws CLI
of test/run.sh (S3 as a directory too): the key, the dirty-checkout refusal, the checksum, which entry serves which
request, and that nothing is stored without a passing suite.  No cargo, no AWS.

usage: python3 -I test_build_cache.py
"""
import importlib.util
import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
SCRIPT = HERE.parent / "build_cache.py"
FAKE_AWS = HERE / "bin"

PASSED = {"crates": {"solver": {"passed": 10, "failed": 0, "ignored": 1},
                     "prover": {"passed": 4, "failed": 0, "ignored": 0}},
          "seconds": 12.0, "jobs": 8}


def load():
    spec = importlib.util.spec_from_file_location("build_cache", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def git(root: Path, *arguments: str) -> str:
    environment = dict(os.environ, GIT_AUTHOR_NAME="t", GIT_AUTHOR_EMAIL="t@t", GIT_COMMITTER_NAME="t",
                       GIT_COMMITTER_EMAIL="t@t")
    return subprocess.run(["git", "-C", str(root), *arguments], capture_output=True, text=True, check=True,
                          env=environment).stdout


def make_binary(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(f"#!/bin/sh\necho {text}\n")
    path.chmod(0o755)


class Cache(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        base = Path(self.tmp.name)
        self.root = base / "checkout"
        (self.root / "rust" / "prover").mkdir(parents=True)
        (self.root / ".gitignore").write_text("target\n")
        (self.root / "rust" / "Cargo.toml").write_text("[package]\n")
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        git(self.root, "add", ".")
        git(self.root, "commit", "-q", "-m", "one")
        self.commit = git(self.root, "rev-parse", "HEAD").strip()
        self.build(self.root)
        self.cache = base / "cache"
        self.env = dict(os.environ, RUSTFLAGS="-C target-cpu=cortex-a76", BUILD_CACHE=f"file://{self.cache}")

    def tearDown(self):
        self.tmp.cleanup()

    def build(self, root: Path) -> None:
        """What `cargo build --release --bins --examples` leaves in the two crates, and its by-products."""
        release = root / "rust" / "target" / "release"
        make_binary(release / "taxman", "taxman")
        make_binary(release / "examples" / "ladder", "ladder")
        make_binary(release / "examples" / "ledger", "ledger")
        make_binary(release / "examples" / "ledger-0123456789abcdef", "ledger copy")
        (release / "examples" / "ledger.d").write_text("deps\n")
        (release / "libtaxman.rlib").write_text("library\n")
        make_binary(root / "rust" / "prover" / "target" / "release" / "exact", "exact")
        make_binary(root / "rust" / "prover" / "target" / "release" / "examples" / "inspect", "inspect")

    def cli(self, command: str, *arguments: str, env=None, root=None):
        where = ["--root", str(root or self.root)]
        return subprocess.run([sys.executable, "-I", str(SCRIPT), command, *where, *arguments], capture_output=True,
                              text=True, env=env or self.env)

    def store(self, solver="--bins --examples", prover="--bins --examples", summary=PASSED):
        stage = Path(self.tmp.name) / "stage"
        done = self.cli("pack", "--stage", str(stage), f"--solver={solver}", f"--prover={prover}")
        self.assertEqual(done.returncode, 0, done.stderr)
        results = Path(self.tmp.name) / "summary.json"
        results.write_text(json.dumps(summary))
        return self.cli("put", "--stage", str(stage), "--summary", str(results), f"--solver={solver}",
                        f"--prover={prover}")

    def fresh_checkout(self) -> Path:
        """A clone at the same commit with no build: what a machine that wants the cache has."""
        clone = Path(tempfile.mkdtemp(dir=self.tmp.name, prefix="machine")) / "taxman"
        subprocess.run(["git", "clone", "-q", str(self.root), str(clone)], check=True)
        return clone

    def fetch(self, solver, prover="", root=None, env=None):
        return self.cli("fetch", f"--solver={solver}", f"--prover={prover}", root=root, env=env)

    # --- the key ---
    def test_the_key_is_commit_architecture_and_the_cpu_named_in_rustflags(self):
        done = self.cli("key")
        self.assertEqual(done.returncode, 0, done.stderr)
        arch = "aarch64" if os.uname().machine == "arm64" else os.uname().machine
        self.assertEqual(done.stdout.strip(), f"{self.commit}-{arch}-cortex-a76")

    def test_native_is_the_cpu_the_compiler_reports(self):
        listing = subprocess.run(["rustc", "--print", "target-cpus"], capture_output=True, text=True).stdout
        reported = re.search(r"\(currently ([^)]+)\)", listing).group(1)
        done = self.cli("key", env=dict(self.env, RUSTFLAGS="-C target-cpu=native"))
        self.assertTrue(done.stdout.strip().endswith(f"-{reported}"), done.stdout)

    def test_no_rustflags_is_generic_and_other_flags_have_no_key(self):
        self.assertTrue(self.cli("key", env=dict(self.env, RUSTFLAGS="")).stdout.strip().endswith("-generic"))
        done = self.cli("key", env=dict(self.env, RUSTFLAGS="-C target-cpu=native -C opt-level=2"))
        self.assertEqual(done.returncode, 3)
        self.assertIn("RUSTFLAGS", done.stderr)

    def test_a_dirty_checkout_has_no_key_and_neither_reads_nor_writes(self):
        self.assertEqual(self.store().returncode, 0)
        self.assertEqual(self.cli("key").returncode, 0)  # the build outputs are ignored files: clean
        for dirt in ("rust/Cargo.toml", "rust/new.rs"):  # a change to a tracked file, a file git does not know
            (self.root / dirt).write_text("changed\n")
            self.assertEqual(self.cli("key").returncode, 3, dirt)
            self.assertEqual(self.fetch("--bins").returncode, 3, dirt)
            stage = Path(self.tmp.name) / "stage2"
            self.assertEqual(self.cli("pack", "--stage", str(stage), "--solver=--bins").returncode, 3, dirt)
            if dirt.endswith("toml"):
                git(self.root, "checkout", "--", dirt)
            else:
                (self.root / dirt).unlink()
        self.assertEqual(self.cli("key").returncode, 0)

    def test_no_commit_and_no_repository_have_no_key(self):
        bare = Path(self.tmp.name) / "plain"
        (bare / "rust").mkdir(parents=True)
        done = self.cli("key", root=bare)
        self.assertEqual(done.returncode, 3)
        self.assertIn("git", done.stderr)

    def test_the_cache_off_is_not_used(self):
        for off in ("off", "", "none"):
            for command in (lambda env: self.cli("key", env=env), lambda env: self.fetch("--bins", env=env)):
                done = command(dict(self.env, BUILD_CACHE=off))
                self.assertEqual(done.returncode, 3, off)
                self.assertIn("off", done.stderr)

    # --- storing and fetching ---
    def test_a_stored_build_is_fetched_by_a_machine_that_has_none_and_runs(self):
        done = self.store()
        self.assertEqual(done.returncode, 0, done.stderr)
        machine = self.fresh_checkout()
        done = self.fetch("--bins --examples", "--bins --examples", root=machine)
        self.assertEqual(done.returncode, 0, done.stderr)
        for relative in ("rust/target/release/taxman", "rust/target/release/examples/ledger",
                         "rust/prover/target/release/exact", "rust/prover/target/release/examples/inspect"):
            path = machine / relative
            self.assertTrue(os.access(path, os.X_OK), relative)
            self.assertEqual(subprocess.run([str(path)], capture_output=True, text=True).stdout.strip(), path.name)
        # not what cargo keeps for itself:
        self.assertFalse((machine / "rust/target/release/libtaxman.rlib").exists())
        self.assertFalse((machine / "rust/target/release/examples/ledger-0123456789abcdef").exists())
        self.assertFalse((machine / "rust/target/release/examples/ledger.d").exists())

    def test_the_entry_says_what_it_holds_and_that_the_suite_passed(self):
        self.store()
        (entry,) = (self.cache / f"{self.commit}-{self._arch()}-cortex-a76").iterdir()
        self.assertEqual(entry.name, "prover-bins+prover-examples+solver-bins+solver-examples")
        manifest = (entry / "manifest").read_text()
        for line in ("tests=passed", "tests_solver=10 passed, 0 failed, 1 ignored", "set=solver bins",
                     f"commit={self.commit}", "rustflags=-C target-cpu=cortex-a76"):
            self.assertIn(line + "\n", manifest)
        self.assertIn("sha256=", manifest)

    def _arch(self) -> str:
        return "aarch64" if os.uname().machine == "arm64" else os.uname().machine

    def test_an_entry_serves_what_it_covers_and_nothing_else(self):
        self.assertEqual(self.store(solver="--bin taxman --example ledger", prover="").returncode, 0)
        machine = self.fresh_checkout()
        self.assertEqual(self.fetch("--bin taxman", root=machine).returncode, 0)
        self.assertEqual(self.fetch("--example ledger --bin taxman", root=machine).returncode, 0)
        self.assertEqual(self.fetch("--example ladder", root=machine).returncode, 1)  # not in it
        self.assertEqual(self.fetch("--bins", root=machine).returncode, 1)  # all of them is more than one
        self.assertEqual(self.fetch("--bin taxman", prover="--bin exact", root=machine).returncode, 1)  # no prover
        self.assertFalse((machine / "rust/target/release/examples/ladder").exists())
        self.assertEqual(self.fetch("--bin taxman=oops").returncode, 2)

    def test_a_plural_entry_serves_a_named_request(self):
        self.store()
        done = self.fetch("--bin taxman --example ladder", "--bin exact", root=self.fresh_checkout())
        self.assertEqual(done.returncode, 0, done.stderr)

    def test_nothing_is_fetched_for_another_cpu_or_commit(self):
        self.store()
        machine = self.fresh_checkout()
        other_cpu = dict(self.env, RUSTFLAGS="-C target-cpu=neoverse-v2")
        self.assertEqual(self.fetch("--bins", root=machine, env=other_cpu).returncode, 1)
        (machine / "rust" / "more.rs").write_text("a change\n")
        git(machine, "add", ".")
        git(machine, "commit", "-q", "-m", "two")
        self.assertEqual(self.fetch("--bins", root=machine).returncode, 1)

    def test_a_tarball_that_does_not_match_its_checksum_is_not_used(self):
        self.store()
        (tarball,) = (self.cache).glob("*/*/binaries.tar.gz")
        data = bytearray(tarball.read_bytes())
        data[len(data) // 2] ^= 0xFF
        tarball.write_bytes(bytes(data))
        machine = self.fresh_checkout()
        done = self.fetch("--bins --examples", "--bins --examples", root=machine)
        self.assertEqual(done.returncode, 1)
        self.assertIn("checksum", done.stderr)
        self.assertFalse((machine / "rust" / "target").exists())

    def test_an_entry_without_its_manifest_or_without_a_passed_suite_does_not_exist(self):
        self.store()
        (manifest,) = self.cache.glob("*/*/manifest")
        text = manifest.read_text()
        manifest.write_text(text.replace("tests=passed\n", ""))
        self.assertEqual(self.fetch("--bins --examples", root=self.fresh_checkout()).returncode, 1)
        manifest.unlink()
        self.assertEqual(self.fetch("--bins --examples", root=self.fresh_checkout()).returncode, 1)

    def test_a_tarball_holding_a_stranger_is_refused(self):
        module = load()
        import io
        import tarfile
        stage = Path(self.tmp.name) / "evil"
        stage.mkdir()
        with tarfile.open(stage / module.TARBALL, "w:gz") as archive:
            info = tarfile.TarInfo("../outside")
            info.size = 2
            archive.addfile(info, io.BytesIO(b"x\n"))
        with tarfile.open(stage / module.TARBALL) as archive:
            with self.assertRaises(module.Failed):
                module.checked_members(archive)

    def test_nothing_is_stored_unless_the_suite_passed_for_every_crate_stored(self):
        failing = {"crates": {"solver": {"passed": 9, "failed": 1, "ignored": 0}, "prover": {"passed": 4, "failed": 0,
                                                                                            "ignored": 0}},
                   "seconds": 1.0, "jobs": 8}
        self.assertEqual(self.store(summary=failing).returncode, 1)
        solver_only = {"crates": {"solver": {"passed": 9, "failed": 0, "ignored": 0}}, "seconds": 1.0, "jobs": 8}
        self.assertEqual(self.store(summary=solver_only).returncode, 1)  # the prover's suite did not run
        self.assertEqual(self.store(summary={"crates": {}, "seconds": 0, "jobs": 1}).returncode, 1)
        self.assertEqual(list(self.cache.glob("*/*/*")) if self.cache.exists() else [], [])
        self.assertEqual(self.store(prover="", summary=solver_only).returncode, 0)  # the solver's suite is enough

    def test_a_pack_of_a_build_that_lacks_a_file_fails(self):
        (self.root / "rust" / "target" / "release" / "taxman").unlink()
        done = self.cli("pack", "--stage", str(Path(self.tmp.name) / "stage"), "--solver=--bin taxman")
        self.assertEqual(done.returncode, 1)
        self.assertIn("did not produce", done.stderr)

    def test_what_the_tests_left_in_the_checkout_does_not_stop_the_put(self):
        # The stage was packed from a clean checkout; the tests ran after that and may have written files in it.
        stage = Path(self.tmp.name) / "stage"
        self.assertEqual(self.cli("pack", "--stage", str(stage), "--solver=--bins").returncode, 0)
        (self.root / "rust" / "left-by-a-test.txt").write_text("x\n")
        summary = Path(self.tmp.name) / "summary.json"
        summary.write_text(json.dumps(PASSED))
        done = self.cli("put", "--stage", str(stage), "--summary", str(summary), "--solver=--bins")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual([p.parent.name for p in self.cache.glob("*/*/manifest")], ["solver-bins"])

    def test_the_test_durations_travel_through_the_cache(self):
        times = Path(self.tmp.name) / "times.json"
        times.write_text('{"tests": {"a": 1.5}}')
        self.assertEqual(self.cli("times-get", "--file", str(times)).returncode, 1)  # none yet
        self.assertEqual(self.cli("times-put", "--file", str(times)).returncode, 0)
        other = Path(self.tmp.name) / "other.json"
        self.assertEqual(self.cli("times-get", "--file", str(other)).returncode, 0)
        self.assertEqual(json.loads(other.read_text()), {"tests": {"a": 1.5}})

    # --- through the stand-in for the aws CLI ---
    def test_the_same_through_s3_commands(self):
        fake = {"PATH": f"{FAKE_AWS}{os.pathsep}{os.environ['PATH']}", "FAKE_S3": str(Path(self.tmp.name) / "s3"),
                "FAKE_STATE": str(Path(self.tmp.name) / "state"), "FAKE_LOG": str(Path(self.tmp.name) / "aws.log"),
                "BUILD_CACHE": "s3://bucket/builds"}
        self.env.update(fake)
        self.assertEqual(self.store().returncode, 0)
        machine = self.fresh_checkout()
        done = self.fetch("--bin taxman --example ledger", "--bin exact", root=machine)
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertTrue(os.access(machine / "rust/target/release/taxman", os.X_OK))
        stored = Path(fake["FAKE_S3"]) / "bucket" / "builds" / f"{self.commit}-{self._arch()}-cortex-a76"
        self.assertTrue(stored.is_dir())

    def test_without_the_aws_cli_the_cache_is_not_used(self):
        self.env.update({"BUILD_CACHE": "s3://bucket/builds", "AWS_CLI": "no-such-aws-cli"})
        self.assertEqual(self.fetch("--bins").returncode, 3)


class Selectors(unittest.TestCase):
    def test_what_a_set_covers(self):
        module = load()
        parse = module.parse_selectors
        everything = parse("solver", "--bins --examples") | parse("prover", "--bins")
        self.assertTrue(module.covers(everything, parse("solver", "--bin taxman --example ledger")))
        self.assertTrue(module.covers(everything, parse("prover", "--bin exact")))
        self.assertFalse(module.covers(everything, parse("prover", "--example inspect")))
        self.assertTrue(module.covers(parse("solver", "--bin=taxman"), parse("solver", "--bin taxman")))
        self.assertFalse(module.covers(parse("solver", "--bin taxman"), parse("solver", "--bins")))
        self.assertTrue(module.covers(frozenset(), frozenset()))

    def test_only_binaries_and_examples_are_understood(self):
        module = load()
        for text in ("--lib", "--all-targets", "--features x", "--bin", "--bin ../x", "-p taxman"):
            with self.assertRaises(module.Usage, msg=text):
                module.parse_selectors("solver", text)


if __name__ == "__main__":
    unittest.main()
